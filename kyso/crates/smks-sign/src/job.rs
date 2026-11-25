//! Lô ký (kế hoạch 6.6; port `SignService.SignAsync` / `RunSignAsync` / `ListPdfFiles`).
//!
//! Giữ như bản cũ: sắp xếp tự nhiên (số đệm 12 chữ số), gộp file trùng, bỏ qua file `_daky_yyyyMMdd_HHmmss`
//! do bản cũ tạo (Q-05, tạm giữ như bản cũ); chạy song song 2–8 luồng theo số CPU; lệnh ký trên token tuần tự
//! (khoá trong `StoreKey`), gọi TSA song song; cứ 2 giây kiểm token còn cắm, rút ra thì dừng; ba loại sự kiện
//! `manifest`, `file`, `summary`; file chưa xử lý khi dừng được báo lỗi kèm lý do; kiểm lại từng file vừa ký.
//!
//! Khác bản cũ (ADR 0004): lưu vào `<thư mục nguồn>\DaKySo\<tên>_signed.pdf` (hoặc thư mục cố định). Ký cả thư
//! mục thì duyệt cả thư mục con và lưu vào thư mục cùng cấp `<tên thư mục>_DaKySo`, giữ nguyên cấu trúc thư mục
//! con (TD-50). Không bao giờ ghi đè file gốc, trùng tên thì thêm ` (1)`, ` (2)`… trừ khi người dùng chọn ghi đè bản đã ký.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use smks_core::config::{
    Anchor, ImageLayout, ImageSplit, PagePick, PartBox, SignKind, SignatureProfile,
};
use smks_core::consts::{appearance as ap, output, signing as k, stamp_image as si};
use smks_core::stamp_plan::{self, AvailableImages, StampKind, StampSpec};
use smks_core::{ErrorCode, Result, SmksError};
use smks_crypto::cert::Cert;
use smks_crypto::key::SigningKey;
use smks_crypto::tsa::TsaClient;
use smks_pdf::appearance::{FontData, StampImage, TextStyle, parse_color};
use smks_pdf::geometry::{self, Placement, page_scale};
use smks_pdf::incremental::{SignRequest, Stamp, Widget, WidgetContent};
use smks_pdf::reader::{PdfFile, Rect};

use crate::sign::sign_pdf;
use crate::verify::Verdict;

/// Một file trong danh sách cần ký.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// Số chữ ký đã có (nhãn "Đã có N chữ ký").
    pub signature_count: usize,
    /// File không đọc được như PDF.
    pub unreadable: bool,
    /// Thư mục người dùng chọn mà file này nằm trong (kể cả trong thư mục con); `None` khi chọn từng file.
    pub root: Option<String>,
}

fn natural_key(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 16);
    let mut digits = String::new();
    for ch in name.chars().chain(std::iter::once('\0')) {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        if !digits.is_empty() {
            out.push_str(&format!("{digits:0>12}"));
            digits.clear();
        }
        if ch != '\0' {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// Tên có hậu tố `_daky_yyyyMMdd_HHmmss` do bản 1.4.0 tạo.
pub fn is_legacy_signed_name(stem: &str) -> bool {
    let Some(at) = stem.rfind(k::LEGACY_SIGNED_FILE_SUFFIX) else {
        return false;
    };
    let tail = &stem[at + k::LEGACY_SIGNED_FILE_SUFFIX.len()..];
    let bytes = tail.as_bytes();
    bytes.len() == 15
        && bytes[8] == b'_'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 8 || b.is_ascii_digit())
}

fn name_key(p: &Path) -> String {
    natural_key(
        &p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    )
}

/// Thư mục kết quả của lần ký trước, không liệt kê lại: `DaKySo` (bản đã ký cạnh file gốc) và `…_DaKySo` (thư mục
/// song song khi ký cả thư mục, phòng trường hợp nó nằm trong thư mục nguồn, ví dụ thư mục cố định đặt bên trong).
fn is_output_dir_name(name: &str) -> bool {
    let name = name.to_lowercase();
    let sub = output::DEFAULT_SUBFOLDER_NAME.to_lowercase();
    name == sub || name.ends_with(&format!("_{sub}"))
}

/// Duyệt thư mục và mọi thư mục con: file PDF của thư mục trước (sắp xếp tự nhiên), rồi lần lượt từng thư mục con.
/// Không theo lối tắt (symlink, junction) để tránh vòng lặp; thư mục con không đọc được thì bỏ qua.
fn walk_pdfs(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if !is_output_dir_name(&entry.file_name().to_string_lossy()) {
                dirs.push(path);
            }
        } else if kind.is_file()
            && is_pdf(&path)
            && !path
                .file_stem()
                .is_some_and(|s| is_legacy_signed_name(&s.to_string_lossy()))
        {
            files.push(path);
        }
    }
    files.sort_by_cached_key(|p| name_key(p));
    dirs.sort_by_cached_key(|p| name_key(p));
    out.append(&mut files);
    for sub in dirs {
        let _ = walk_pdfs(&sub, out);
    }
    Ok(())
}

/// Liệt kê file PDF từ các nguồn (file, hoặc thư mục kể cả thư mục con). File lấy từ thư mục có `root` là thư mục
/// đó và `name` là đường dẫn tương đối (`PhongA\b.pdf`), để lưu bản đã ký giữ nguyên cấu trúc thư mục.
pub fn list_pdf_files(sources: &[PathBuf]) -> Result<Vec<FileItem>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for source in sources {
        let mut root = None;
        let paths: Vec<PathBuf> = if source.is_dir() {
            let dir = std::path::absolute(source).unwrap_or_else(|_| source.clone());
            let mut files = Vec::new();
            walk_pdfs(&dir, &mut files).map_err(|e| {
                SmksError::new(
                    ErrorCode::SignSourceNotFound,
                    format!("Không đọc được thư mục {}: {e}", source.display()),
                )
            })?;
            root = Some(dir);
            files
        } else if source.is_file() {
            if is_pdf(source) {
                vec![source.clone()]
            } else {
                Vec::new()
            }
        } else {
            return Err(SmksError::new(
                ErrorCode::SignSourceNotFound,
                format!(
                    "Không tìm thấy file hoặc thư mục cần ký: {}",
                    source.display()
                ),
            ));
        };
        for path in paths {
            let full = std::path::absolute(&path).unwrap_or(path);
            if !seen.insert(full.to_string_lossy().to_lowercase()) {
                continue;
            }
            let size = std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
            let (signature_count, unreadable) = match std::fs::read(&full) {
                Ok(bytes) => match smks_pdf::sigscan::scan(&bytes) {
                    Ok(sigs) => (sigs.len(), false),
                    Err(_) => (0, true),
                },
                Err(_) => (0, true),
            };
            let name = root
                .as_deref()
                .and_then(|r| full.strip_prefix(r).ok())
                .map(|rel| rel.display().to_string())
                .filter(|rel| !rel.is_empty())
                .unwrap_or_else(|| {
                    full.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
            out.push(FileItem {
                name,
                path: full.display().to_string(),
                size,
                signature_count,
                unreadable,
                root: root.as_ref().map(|r| r.display().to_string()),
            });
        }
    }
    Ok(out)
}

pub use smks_core::config::SignatureMode;

/// Cấu hình hiển thị dùng chung cho cả lô.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub mode: SignatureMode,
    /// Lượt ký chính hay ký nháy (chỗ mặc định của khung khác nhau). Lượt nào cũng ra đúng một chữ ký.
    pub kind: SignKind,
    /// Các khung tự đặt (kéo, thêm, xoá) áp cho cả lô; rỗng thì theo `profiles` (mẫu theo khổ giấy), không có mẫu
    /// thì góc trên phải trang 1 như bản cũ. Mỗi khung là một chỗ hiện cùng một chữ ký.
    pub placements: Vec<Placement>,
    /// Mẫu chữ ký theo khổ giấy (CH-01…CH-03): mỗi file dùng mẫu khớp khổ trang đầu.
    pub profiles: Vec<SignatureProfile>,
    pub font_family: String,
    pub font_size: f64,
    pub text_color: String,
    pub reason: String,
    pub location: String,
    pub seal_image: Option<PathBuf>,
    pub wet_image: Option<PathBuf>,
    /// Ảnh dùng trong khối chữ ký khi `mode` là ImageAndText / Image: `seal` hoặc `wetSignature`.
    pub signature_image: Option<StampKind>,
    /// Ảnh nằm ở đâu so với chữ (kiểu "Ảnh + thông tin").
    pub image_layout: ImageLayout,
    /// Phần khối dành cho ảnh (%), `None` = tự động (TD-51).
    pub image_share: Option<u8>,
    /// Khoảng cách ảnh – chữ (% khối), `None` = tự động (TD-51).
    pub image_gap: Option<u8>,
    /// Kiểu "Tự do" (TD-52): ô ảnh và ô chữ trong khối; chỉ dùng khi có cả hai.
    pub image_box: Option<PartBox>,
    pub text_box: Option<PartBox>,
    pub seal_darkness: i32,
    pub wet_darkness: i32,
    pub wet_stroke: i32,
    pub wet_color: Option<String>,
    /// Ảnh dấu đóng riêng (chú thích `/Stamp`) theo quy tắc trang.
    pub stamps: Vec<StampSpec>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: SignatureMode::Text,
            kind: SignKind::Main,
            placements: Vec::new(),
            profiles: Vec::new(),
            font_family: ap::DEFAULT_FONT_FAMILY.into(),
            font_size: ap::DEFAULT_FONT_SIZE_PT,
            text_color: ap::DEFAULT_TEXT_COLOR.into(),
            reason: k::DEFAULT_REASON.into(),
            location: k::DEFAULT_LOCATION.into(),
            seal_image: None,
            wet_image: None,
            signature_image: None,
            image_layout: ImageLayout::Left,
            image_share: None,
            image_gap: None,
            image_box: None,
            text_box: None,
            seal_darkness: si::DARKNESS_DEFAULT,
            wet_darkness: si::DARKNESS_DEFAULT,
            wet_stroke: si::STROKE_DEFAULT,
            wet_color: None,
            stamps: Vec::new(),
        }
    }
}

/// Nơi lưu kết quả.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputOptions {
    /// Thư mục con trong thư mục nguồn (mặc định `DaKySo`).
    pub subfolder: String,
    /// Hậu tố tên file (mặc định `_signed`).
    pub suffix: String,
    /// Lưu tất cả vào một thư mục cố định (bỏ qua `subfolder`).
    pub fixed_folder: Option<PathBuf>,
    /// Trùng tên thì ghi đè bản đã ký trước (không bao giờ là file gốc); mặc định thêm ` (1)`.
    pub overwrite: bool,
    /// "Xóa file gốc" (ADR 0003): bản đã ký thay vào chỗ file gốc, file gốc vào Thùng rác. Chỉ làm khi chữ ký
    /// vừa ký kiểm lại hợp lệ; không đưa được vào Thùng rác thì giữ file gốc và lưu như thường.
    pub replace_original: bool,
    /// Thư mục người dùng chọn để ký cả thư mục (`FileItem::root`). File nằm trong một thư mục này được lưu vào thư
    /// mục song song `<tên thư mục>_<subfolder>` (cạnh thư mục đó, hoặc trong `fixed_folder`), giữ nguyên thư mục con.
    pub source_roots: Vec<PathBuf>,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            subfolder: output::DEFAULT_SUBFOLDER_NAME.into(),
            suffix: output::DEFAULT_SIGNED_SUFFIX.into(),
            fixed_folder: None,
            overwrite: false,
            replace_original: false,
            source_roots: Vec::new(),
        }
    }
}

/// Yêu cầu một lô ký.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct JobRequest {
    /// File cần ký (đã liệt kê bằng [`list_pdf_files`]).
    pub files: Vec<PathBuf>,
    pub appearance: Appearance,
    pub output: OutputOptions,
    /// Khung riêng cho từng file (đường dẫn → khung) của lượt ký này, ghi đè `appearance.placements`. Giao diện gửi
    /// đúng danh sách theo loại lượt ký (khung chính hay khung nháy).
    pub placements: HashMap<String, Vec<Placement>>,
    /// Kiểu hiển thị riêng cho từng file (đường dẫn → kiểu), ghi đè `appearance.mode`.
    pub modes: HashMap<String, SignatureMode>,
    /// Chỉ chấp nhận kết quả "Hợp lệ" khi kiểm lại (sản phẩm); tắt khi thử bằng CA thử.
    pub require_trusted: bool,
}

/// Sự kiện tiến độ (giữ ba loại như bản cũ).
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JobEvent {
    Manifest {
        job_id: String,
        total: usize,
        files: Vec<String>,
        /// Thư mục kết quả nếu mọi file chung một thư mục, rỗng nếu khác nhau.
        output_folder: String,
        /// Nguồn giờ ký của lô (TD-54).
        clock: Clock,
        /// Có khi ký theo giờ máy tính vì không lấy được giờ máy chủ dấu thời gian: lý do, nói rõ file không có
        /// dấu thời gian.
        clock_note: Option<String>,
    },
    File {
        index: usize,
        name: String,
        success: bool,
        output_path: Option<String>,
        reason: Option<String>,
        verdict: Option<Verdict>,
        trusted_time: Option<String>,
        /// File chưa ký vì lô tạm dừng (người dùng bấm Tạm dừng, hoặc rút token): không phải lỗi của file, "Ký
        /// tiếp" sẽ ký lại. `reason` nói lý do dừng.
        paused: bool,
        done: usize,
        total: usize,
    },
    Summary {
        job_id: String,
        total: usize,
        success: usize,
        /// File lỗi thật (không tính file tạm dừng).
        failed: usize,
        /// File chưa ký vì lô tạm dừng.
        paused: usize,
        stopped: bool,
        token_removed: bool,
        elapsed_ms: u64,
        output_folder: String,
        /// Như `Manifest::clock_note` (để hộp kết quả nhắc lại).
        clock_note: Option<String>,
    },
}

/// Nguồn giờ ký của một lô (TD-54).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Clock {
    /// Giờ của máy chủ dấu thời gian Ban Cơ yếu (hỏi ở đầu lô), mỗi file có dấu thời gian.
    Tsa,
    /// Giờ máy tính, không có dấu thời gian (mất mạng / máy chủ không trả lời, hoặc chạy không có TSA).
    Computer,
}

/// Giờ ký của một lô: độ lệch so với đồng hồ máy và có gắn dấu thời gian hay không.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchClock {
    pub clock: Clock,
    /// Cộng vào đồng hồ máy để ra giờ ký (giây).
    pub offset: i64,
    pub note: Option<String>,
}

impl BatchClock {
    /// Đầu lô: có TSA thì hỏi giờ máy chủ (`probe`); lấy được thì giờ ký = đồng hồ máy + độ lệch tới giờ máy chủ, mỗi
    /// file gắn dấu thời gian như cũ. Không lấy được (mất mạng, máy chủ không trả lời / trả sai) thì ký theo giờ máy
    /// tính, không gắn dấu thời gian, kèm lý do. `local_now`: đồng hồ máy lúc nhận giờ máy chủ.
    pub fn pick(
        tsa: Option<&TsaClient>,
        probe: &dyn Fn(&TsaClient) -> Result<i64>,
        local_now: &dyn Fn() -> i64,
    ) -> Self {
        let Some(tsa) = tsa else {
            return Self {
                clock: Clock::Computer,
                offset: 0,
                note: None,
            };
        };
        match probe(tsa) {
            Ok(server) => Self {
                clock: Clock::Tsa,
                offset: server - local_now(),
                note: None,
            },
            Err(e) => Self {
                clock: Clock::Computer,
                offset: 0,
                note: Some(format!(
                    "Không lấy được giờ của máy chủ dấu thời gian Ban Cơ yếu ({}). Đã ký theo giờ máy tính ({}), \
                     văn bản không có dấu thời gian.",
                    e.message.trim_end_matches('.'),
                    crate::verify::vietnam_text(local_now())
                )),
            },
        }
    }
}

/// Thư mục đã chọn (trong `source_roots`) chứa `source`, lấy thư mục sâu nhất, kèm đường dẫn tương đối của thư mục
/// chứa file so với nó (rỗng nếu file nằm ngay trong thư mục đã chọn).
fn source_root<'a>(source: &Path, out: &'a OutputOptions) -> Option<(&'a Path, PathBuf)> {
    let parent = source.parent()?;
    out.source_roots
        .iter()
        .filter_map(|r| {
            parent
                .strip_prefix(r)
                .ok()
                .map(|rel| (r.as_path(), rel.to_path_buf()))
        })
        .max_by_key(|(r, _)| r.components().count())
}

/// Thư mục song song khi ký cả thư mục: `<thư mục cha>\<tên>_DaKySo`, hoặc `<thư mục cố định>\<tên>_DaKySo`. Thư
/// mục đã chọn là gốc ổ đĩa (`D:\`, không có tên) thì dùng `D:\DaKySo` (khi liệt kê lại sẽ bỏ qua thư mục này).
fn mirror_root(root: &Path, out: &OutputOptions) -> PathBuf {
    let sub = match out.subfolder.trim() {
        "" => output::DEFAULT_SUBFOLDER_NAME,
        s => s,
    };
    let base = out.fixed_folder.as_deref().or(root.parent());
    match (root.file_name(), base) {
        (Some(name), Some(base)) => base.join(format!("{}_{sub}", name.to_string_lossy())),
        _ => out.fixed_folder.as_deref().unwrap_or(root).join(sub),
    }
}

/// Thư mục kết quả "trên cùng" của một file, để báo cho giao diện mở: thư mục song song khi ký cả thư mục, còn lại
/// đúng thư mục chứa file kết quả.
fn output_top(source: &Path, out: &OutputOptions) -> PathBuf {
    if !out.replace_original
        && let Some((root, _)) = source_root(source, out)
    {
        return mirror_root(root, out);
    }
    output_dir(source, out)
}

fn output_dir(source: &Path, out: &OutputOptions) -> PathBuf {
    if out.replace_original {
        return source.parent().unwrap_or(Path::new(".")).to_path_buf();
    }
    if let Some((root, rel)) = source_root(source, out) {
        return mirror_root(root, out).join(rel);
    }
    match &out.fixed_folder {
        Some(dir) => dir.clone(),
        None => source
            .parent()
            .unwrap_or(Path::new("."))
            .join(&out.subfolder),
    }
}

/// Chọn đường dẫn kết quả không trùng file đã có và không trùng file khác trong lô.
fn reserve_output(
    source: &Path,
    out: &OutputOptions,
    taken: &Mutex<HashSet<String>>,
) -> Result<PathBuf> {
    let dir = output_dir(source, out);
    std::fs::create_dir_all(&dir).map_err(|e| {
        SmksError::new(
            ErrorCode::OutputFolderInvalid,
            format!("Không tạo được thư mục lưu {}: {e}", dir.display()),
        )
    })?;
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "van-ban".into());
    let mut taken = taken.lock().unwrap_or_else(PoisonError::into_inner);
    for n in 0..10_000 {
        let name = if n == 0 {
            format!("{stem}{}.pdf", out.suffix)
        } else {
            format!("{stem}{} ({n}).pdf", out.suffix)
        };
        let path = dir.join(name);
        let key = path.to_string_lossy().to_lowercase();
        if taken.contains(&key) {
            continue;
        }
        if path.exists() && !out.overwrite {
            continue;
        }
        if same_path(&path, source) {
            continue;
        }
        taken.insert(key);
        return Ok(path);
    }
    Err(SmksError::new(
        ErrorCode::OutputExists,
        "Không chọn được tên file kết quả chưa có.",
    ))
}

/// Tên file kết quả dự kiến của một file nguồn (chưa thêm ` (1)`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedOutput {
    pub source: String,
    pub output: String,
    /// Đã có file trùng tên ở nơi lưu (hộp "File đã ký / trùng tên", KS-16).
    pub exists: bool,
}

/// Nơi lưu dự kiến của từng file, để hỏi người dùng trước khi ký khi đã có file trùng tên. "Xóa file gốc"
/// thì kết quả nằm đúng chỗ file gốc nên không tính là trùng.
pub fn plan_outputs(files: &[PathBuf], out: &OutputOptions) -> Vec<PlannedOutput> {
    files
        .iter()
        .map(|source| {
            if out.replace_original {
                return PlannedOutput {
                    source: source.display().to_string(),
                    output: source.display().to_string(),
                    exists: false,
                };
            }
            let stem = source
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "van-ban".into());
            let path = output_dir(source, out).join(format!("{stem}{}.pdf", out.suffix));
            PlannedOutput {
                source: source.display().to_string(),
                exists: path.is_file() && !same_path(&path, source),
                output: path.display().to_string(),
            }
        })
        .collect()
}

/// "Xóa file gốc": ghi bản đã ký cạnh file gốc (`.tmp`), đưa file gốc vào Thùng rác (`trash`), rồi đổi tên bản
/// đã ký về đúng tên file gốc. `trash` lỗi thì giữ file gốc, lưu bản đã ký vào nơi lưu thường và trả lý do.
fn replace_source(
    source: &Path,
    bytes: &[u8],
    request: &JobRequest,
    taken: &Mutex<HashSet<String>>,
    trash: &dyn Fn(&Path) -> std::result::Result<(), String>,
) -> std::result::Result<(PathBuf, Option<String>), String> {
    let mut tmp = source.as_os_str().to_owned();
    tmp.push(output::TEMP_EXTENSION);
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| format!("Không ghi được {}: {e}", tmp.display()))?;
    match trash(source) {
        Ok(()) => match std::fs::rename(&tmp, source) {
            Ok(()) => Ok((source.to_path_buf(), None)),
            Err(e) => Err(format!(
                "File gốc đã vào Thùng rác nhưng không đổi tên được bản đã ký ({e}). Bản đã ký: {}; lấy lại file gốc trong Thùng rác nếu cần.",
                tmp.display()
            )),
        },
        Err(why) => {
            let _ = std::fs::remove_file(&tmp);
            let normal = OutputOptions {
                replace_original: false,
                ..request.output.clone()
            };
            let path = reserve_output(source, &normal, taken).map_err(|e| e.message)?;
            write_atomic(&path, bytes).map_err(|e| e.message)?;
            Ok((
                path.clone(),
                Some(format!(
                    "Không thay file gốc: {why}. File gốc giữ nguyên, bản đã ký lưu ở {}.",
                    path.display()
                )),
            ))
        }
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(output::TEMP_EXTENSION);
    let tmp = PathBuf::from(tmp);
    let fail = |e: std::io::Error| {
        SmksError::new(
            ErrorCode::OutputFolderInvalid,
            format!("Không ghi được {}: {e}", path.display()),
        )
    };
    std::fs::write(&tmp, bytes).map_err(fail)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(fail)?;
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })
}

/// Ảnh đã đọc một lần cho cả lô.
struct Images {
    seal: Option<StampImage>,
    wet: Option<StampImage>,
    wet_stroke: i32,
}

impl Images {
    fn load(a: &Appearance) -> Result<Self> {
        let read = |path: &Option<PathBuf>,
                    darkness: i32,
                    ink: Option<[f64; 3]>,
                    label: &str|
         -> Result<Option<StampImage>> {
            let Some(path) = path else { return Ok(None) };
            let ext = path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                .unwrap_or_default();
            if !si::ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
                return Err(SmksError::new(
                    ErrorCode::StampImageInvalid,
                    format!("Ảnh {label} phải là PNG hoặc JPG: {}", path.display()),
                ));
            }
            let bytes = std::fs::read(path).map_err(|e| {
                SmksError::new(
                    ErrorCode::StampImageInvalid,
                    format!(
                        "Ảnh {label} không hợp lệ hoặc không tồn tại: {} ({e})",
                        path.display()
                    ),
                )
            })?;
            Ok(Some(StampImage::prepare(&bytes, darkness, ink, 1200)?))
        };
        let ink = match &a.wet_color {
            Some(c) if !c.trim().is_empty() => Some(parse_color(c).ok_or_else(|| {
                SmksError::new(
                    ErrorCode::BadRequest,
                    "Màu mực chữ ký tươi không hợp lệ, cần dạng #RRGGBB.",
                )
            })?),
            _ => None,
        };
        Ok(Self {
            seal: read(&a.seal_image, a.seal_darkness, None, "dấu đỏ")?,
            wet: read(&a.wet_image, a.wet_darkness, ink, "chữ ký tươi")?,
            wet_stroke: a.wet_stroke.clamp(si::STROKE_MIN, si::STROKE_MAX),
        })
    }

    fn get(&self, kind: StampKind) -> Option<(&StampImage, i32)> {
        match kind {
            StampKind::Seal => self.seal.as_ref().map(|i| (i, si::STROKE_MIN)),
            StampKind::WetSignature => self.wet.as_ref().map(|i| (i, self.wet_stroke)),
        }
    }
}

/// Khung mặc định của khối chữ ký theo tỉ lệ (bản cũ: 170 × 30 pt trên A4).
/// Cỡ khối có ảnh theo pt trên trang A4: ảnh trên / dưới chữ thì khối hẹp và cao hơn.
fn image_box_pt(a: &Appearance) -> (f64, f64) {
    if a.mode == SignatureMode::ImageAndText && a.image_layout.is_stacked() {
        (ap::STACKED_WIDTH_PT, ap::STACKED_HEIGHT_PT)
    } else {
        (ap::WIDTH_PT, ap::HEIGHT_PT)
    }
}

fn default_ratio_size(a: &Appearance) -> (f64, f64) {
    let (w, h) = image_box_pt(a);
    (
        w / ap::REFERENCE_PAGE_WIDTH_PT,
        h / ap::REFERENCE_PAGE_HEIGHT_PT,
    )
}

/// Một khối chữ ký đã tính chỗ: trang, khung (toạ độ hiển thị), kiểu chữ theo cỡ trang.
struct Spot {
    page: u32,
    rect: Rect,
    display: Rect,
    style: TextStyle,
}

/// Góc của khung ký nháy theo mẫu: hàng dưới, cột đối diện chữ ký chính (chính ở phải hoặc giữa thì nháy ở trái,
/// chính ở trái thì nháy ở phải), để hai lượt ký không chồng lên nhau.
pub fn initial_anchor(main: Anchor) -> Anchor {
    if main.column() == 0 {
        Anchor::Br
    } else {
        Anchor::Bl
    }
}

/// Khung theo mẫu của lượt ký: chữ ký chính theo mẫu khổ giấy; ký nháy mọi trang, góc theo [`initial_anchor`].
/// Không có mẫu: chữ ký chính về chỗ của bản cũ (`None`), ký nháy dùng mẫu A4 mặc định.
fn template_for(a: &Appearance, first: &Rect) -> Option<SignatureProfile> {
    let profile = if a.profiles.is_empty() {
        match a.kind {
            SignKind::Main => return None,
            SignKind::Initial => SignatureProfile::default(),
        }
    } else {
        smks_core::config::pick_profile(
            &a.profiles,
            geometry::paper_of(first),
            first.width > first.height,
        )
    };
    Some(match a.kind {
        SignKind::Main => profile,
        SignKind::Initial => SignatureProfile {
            page: PagePick::All,
            anchor: initial_anchor(profile.anchor),
            ..profile
        },
    })
}

/// Tính chỗ các khối chữ ký của một file: các khung tự đặt (`manual`), hoặc khung theo mẫu của lượt ký
/// ([`template_for`]: trang đầu / cuối / mọi trang), hoặc góc trên phải trang 1 khi không có mẫu (như bản cũ).
/// Khung tự đặt ở trang vượt số trang của file thì về trang cuối; hai khung trùng chỗ chỉ đặt một.
fn plan_spots(
    pages: &[smks_pdf::reader::PageInfo],
    a: &Appearance,
    manual: &[Placement],
    lines: &[String],
) -> Result<Vec<Spot>> {
    let page_count = pages.len() as u32;
    if page_count == 0 {
        return Err(SmksError::new(
            ErrorCode::SignPdfStructureUnsupported,
            "File PDF không có trang nào.",
        ));
    }
    let (font, _) = FontData::for_lines(&a.font_family, lines)?;
    let color = parse_color(&a.text_color).unwrap_or([0.0; 3]);
    let base_size = a
        .font_size
        .clamp(ap::MIN_FONT_SIZE_PT, ap::MAX_FONT_SIZE_PT);
    let style_for = |display: &Rect| TextStyle {
        font: font.clone(),
        size: base_size * page_scale(display),
        color,
    };
    let has_image = matches!(a.mode, SignatureMode::ImageAndText | SignatureMode::Image);
    let display_of = |n: u32| pages[(n.clamp(1, page_count) - 1) as usize].display_box();

    // Một khung tự đặt (hoặc chỗ của bản cũ khi `None`).
    let manual_spot = |p: Option<&Placement>| -> Result<Spot> {
        let page_no = p.map_or(1, |p| p.page_in(page_count));
        let display = display_of(page_no);
        let style = style_for(&display);
        let rect = if has_image {
            let (w, h) = default_ratio_size(a);
            let p = p.copied().unwrap_or(Placement {
                page: page_no,
                x_ratio: 1.0 - w - 0.02,
                y_ratio: 0.02,
                width_ratio: w,
                height_ratio: h,
            });
            let p = Placement {
                width_ratio: if p.width_ratio > 0.0 {
                    p.width_ratio.min(1.0)
                } else {
                    w
                },
                height_ratio: if p.height_ratio > 0.0 {
                    p.height_ratio.min(1.0)
                } else {
                    h
                },
                ..p
            };
            geometry::frame(&display, &p)?
        } else {
            let (w, line_h) = style.font.measure(lines, style.size)?;
            geometry::resolve(&display, p, w, line_h * lines.len() as f64)?
        };
        Ok(Spot {
            page: page_no,
            rect,
            display,
            style,
        })
    };

    let mut spots: Vec<Spot> = Vec::new();
    if !manual.is_empty() {
        for p in manual {
            let spot = manual_spot(Some(p))?;
            let same = |x: &Spot| {
                x.page == spot.page
                    && [
                        x.rect.x - spot.rect.x,
                        x.rect.y - spot.rect.y,
                        x.rect.width - spot.rect.width,
                        x.rect.height - spot.rect.height,
                    ]
                    .iter()
                    .all(|d| d.abs() < 0.01)
            };
            if !spots.iter().any(same) {
                spots.push(spot);
            }
        }
        return Ok(spots);
    }
    let Some(profile) = template_for(a, &display_of(1)) else {
        spots.push(manual_spot(None)?);
        return Ok(spots);
    };
    let numbers: Vec<u32> = match profile.page {
        PagePick::First => vec![1],
        PagePick::Last => vec![page_count],
        PagePick::All => (1..=page_count).collect(),
    };
    let margin = (
        profile.margin_mm.x * smks_core::config::PT_PER_MM,
        profile.margin_mm.y * smks_core::config::PT_PER_MM,
    );
    let factor = f64::from(profile.scale) / 100.0;
    for n in numbers {
        let display = display_of(n);
        let style = style_for(&display);
        let (w, h) = if has_image {
            let s = page_scale(&display) * factor;
            let (bw, bh) = image_box_pt(a);
            (bw * s, bh * s)
        } else {
            let (w, line_h) = style.font.measure(lines, style.size)?;
            (w, line_h * lines.len() as f64)
        };
        let rect = geometry::anchor_rect(&display, profile.anchor, margin, w, h)?;
        spots.push(Spot {
            page: n,
            rect,
            display,
            style,
        });
    }
    Ok(spots)
}

fn preview_lines(signer: &str) -> Vec<String> {
    vec![
        format!("Ký bởi: {signer}"),
        format!("Ngày ký: {}", crate::verify::now_vietnam().1),
    ]
}

fn to_placements(spots: &[Spot]) -> Vec<Placement> {
    spots
        .iter()
        .map(|s| geometry::to_placement(&s.display, s.page, &s.rect))
        .collect()
}

/// Chỗ các khối chữ ký sẽ đặt trong file (tỉ lệ trang, gốc trên trái) để giao diện vẽ khung xem trước đúng
/// như khi ký. `signer` chỉ để đo bề rộng khối chữ.
pub fn preview_positions(
    bytes: &[u8],
    a: &Appearance,
    manual: &[Placement],
    signer: &str,
) -> Result<Vec<Placement>> {
    if a.mode == SignatureMode::Hidden {
        return Ok(Vec::new());
    }
    let file = PdfFile::open(bytes)?;
    let pages = file.pages()?;
    Ok(to_placements(&plan_spots(
        &pages,
        a,
        manual,
        &preview_lines(signer),
    )?))
}

/// Bố cục chữ ký của một file cho màn Vị trí chữ ký (ảnh thu nhỏ, thanh Cỡ, nút Đặt lại).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLayout {
    pub page_count: u32,
    /// Khổ hiển thị (pt) của trang có khối chữ ký đầu tiên, để vẽ ảnh thu nhỏ đúng tỉ lệ trang.
    pub page_width: f64,
    pub page_height: f64,
    /// Chỗ các khối chữ ký với các khung đang đặt (`manual`; rỗng thì theo mẫu).
    pub boxes: Vec<Placement>,
    /// Chỗ các khối theo mẫu khổ giấy (bỏ qua vị trí kéo tay): cỡ 100% và chỗ của nút Đặt lại.
    pub template: Vec<Placement>,
}

/// Bố cục chữ ký của một file: như [`preview_positions`], thêm số trang, khổ trang và chỗ theo mẫu. Kiểu "Ẩn"
/// vẫn tính chỗ như "Chỉ thông tin" (màn Vị trí chữ ký vẽ khung để tham khảo).
pub fn preview_layout(
    bytes: &[u8],
    a: &Appearance,
    manual: &[Placement],
    signer: &str,
) -> Result<FileLayout> {
    let file = PdfFile::open(bytes)?;
    let pages = file.pages()?;
    let shown;
    let a = if a.mode == SignatureMode::Hidden {
        shown = Appearance {
            mode: SignatureMode::Text,
            ..a.clone()
        };
        &shown
    } else {
        a
    };
    let lines = preview_lines(signer);
    let template = plan_spots(&pages, a, &[], &lines)?;
    let spots = if manual.is_empty() {
        Vec::new()
    } else {
        plan_spots(&pages, a, manual, &lines)?
    };
    let first = spots.first().or(template.first());
    let size = first.map_or_else(
        || pages.first().map(|p| p.display_box()),
        |s| Some(s.display),
    );
    let (page_width, page_height) = size.map_or((0.0, 0.0), |r| (r.width, r.height));
    let template = to_placements(&template);
    Ok(FileLayout {
        page_count: pages.len() as u32,
        page_width,
        page_height,
        boxes: if !manual.is_empty() {
            to_placements(&spots)
        } else {
            template.clone()
        },
        template,
    })
}

/// Dựng yêu cầu ký cho một file (khối chữ ký + ảnh dấu theo quy tắc trang).
fn build_request(
    bytes: &[u8],
    signer: &str,
    a: &Appearance,
    manual: &[Placement],
    images: &Images,
    signed_at: i64,
    signed_text: &str,
) -> Result<SignRequest> {
    let file = PdfFile::open(bytes)?;
    file.check_signable()?;
    let pages = file.pages()?;
    let page_count = pages.len() as u32;
    let mut widgets = Vec::new();
    if a.mode != SignatureMode::Hidden {
        let lines = vec![
            format!("Ký bởi: {signer}"),
            format!("Ngày ký: {signed_text}"),
        ];
        let image = a.signature_image.and_then(|kind| images.get(kind));
        if matches!(a.mode, SignatureMode::ImageAndText | SignatureMode::Image) && image.is_none() {
            return Err(SmksError::new(
                ErrorCode::StampImageInvalid,
                "Kiểu hiển thị có ảnh nhưng chưa chọn ảnh dấu hoặc chữ ký tươi.",
            ));
        }
        for spot in plan_spots(&pages, a, manual, &lines)? {
            let content = match (a.mode, image) {
                (SignatureMode::ImageAndText, Some((img, stroke))) => WidgetContent::ImageAndText {
                    image: img.clone(),
                    stroke,
                    lines: lines.clone(),
                    style: spot.style,
                    layout: a.image_layout,
                    split: ImageSplit {
                        share: a.image_share,
                        gap: a.image_gap,
                        free: a.image_box.zip(a.text_box),
                    },
                },
                (SignatureMode::Image, Some((img, stroke))) => WidgetContent::Image {
                    image: img.clone(),
                    stroke,
                },
                _ => WidgetContent::Text {
                    lines: lines.clone(),
                    style: spot.style,
                },
            };
            widgets.push(Widget {
                page: spot.page,
                rect: spot.rect,
                content,
            });
        }
    }
    let mut stamps = Vec::new();
    if !a.stamps.is_empty() {
        let available = AvailableImages {
            seal: images.seal.is_some(),
            wet_signature: images.wet.is_some(),
        };
        let rules = stamp_plan::build_rules(&a.stamps, available)?;
        for planned in stamp_plan::plan(&rules, page_count).stamps {
            let Some((image, stroke)) = images.get(planned.kind) else {
                continue;
            };
            let page = &pages[(planned.page_number - 1) as usize];
            let p = Placement {
                page: planned.page_number,
                x_ratio: planned.area.x,
                y_ratio: planned.area.y,
                width_ratio: planned.area.width,
                height_ratio: planned.area.height,
            };
            let rect = geometry::frame(&page.display_box(), &p)?;
            stamps.push(Stamp {
                page: planned.page_number,
                rect,
                image: image.clone(),
                stroke,
            });
        }
    }
    Ok(SignRequest {
        signer_name: signer.to_owned(),
        reason: if a.reason.trim().is_empty() {
            k::DEFAULT_REASON.into()
        } else {
            a.reason.clone()
        },
        location: if a.location.trim().is_empty() {
            k::DEFAULT_LOCATION.into()
        } else {
            a.location.clone()
        },
        signed_at,
        widgets,
        stamps,
    })
}

/// Kiểm tra cấu hình trước khi mở lô (lỗi trả ngay cho giao diện, như bản cũ).
pub fn validate(request: &JobRequest) -> Result<()> {
    if request.files.is_empty() {
        return Err(SmksError::new(
            ErrorCode::SignNoPdfFound,
            "Không có file PDF nào để ký.",
        ));
    }
    Images::load(&request.appearance)?;
    if !ap::FONT_FAMILIES.contains(&request.appearance.font_family.as_str()) {
        return Err(SmksError::new(
            ErrorCode::BadRequest,
            format!("Font {} không được hỗ trợ.", request.appearance.font_family),
        ));
    }
    Ok(())
}

/// Số luồng song song: theo số CPU, trong khoảng 2–8 (như bản cũ).
pub fn parallelism() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(k::MIN_PARALLELISM)
        .clamp(k::MIN_PARALLELISM, k::MAX_PARALLELISM)
}

/// Chạy lô ký. `token_present`: kiểm token còn cắm (gọi mỗi 2 giây); `cancel`: người dùng bấm Dừng.
#[allow(clippy::too_many_arguments)]
pub fn run(
    job_id: &str,
    request: &JobRequest,
    key: &dyn SigningKey,
    chain: &[Cert],
    tsa: Option<&TsaClient>,
    cancel: &AtomicBool,
    token_present: Option<&(dyn Fn() -> bool + Sync)>,
    emit: &mut dyn FnMut(JobEvent),
) {
    let started = Instant::now();
    let total = request.files.len();
    let folders: HashSet<PathBuf> = request
        .files
        .iter()
        .map(|f| output_top(f, &request.output))
        .collect();
    let output_folder = if folders.len() == 1 {
        folders
            .into_iter()
            .next()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let clock = BatchClock::pick(
        tsa,
        &|t| t.server_time(Duration::from_secs(k::TSA_PROBE_TIMEOUT_SECONDS)),
        &crate::verify::clock_now,
    );
    emit(JobEvent::Manifest {
        job_id: job_id.to_owned(),
        total,
        files: request
            .files
            .iter()
            .map(|f| {
                f.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect(),
        output_folder: output_folder.clone(),
        clock: clock.clock,
        clock_note: clock.note.clone(),
    });
    // Ký theo giờ máy tính thì không gắn dấu thời gian.
    let tsa = tsa.filter(|_| clock.clock == Clock::Tsa);
    // Ký dài hạn (TD-58): chỉ khi có mạng tới máy chủ dấu thời gian; dữ liệu thu hồi dùng chung cả lô.
    let ltv_cache = smks_crypto::ltv::LtvCache::new();
    let ltv = tsa.map(|_| &ltv_cache);

    let signer = key.certificate().common_name();
    let images = Images::load(&request.appearance);
    let taken = Mutex::new(HashSet::new());
    let next = AtomicUsize::new(0);
    let token_removed = AtomicBool::new(false);
    let finished = AtomicBool::new(false);
    let stop = || cancel.load(Ordering::SeqCst) || token_removed.load(Ordering::SeqCst);
    let (tx, rx) = mpsc::channel::<(
        usize,
        std::result::Result<(PathBuf, crate::verify::SignatureReport), String>,
    )>();
    let mut done = vec![false; total];
    let (mut success, mut failed, mut done_count) = (0usize, 0usize, 0usize);

    std::thread::scope(|scope| {
        if let Some(present) = token_present {
            let (finished, token_removed) = (&finished, &token_removed);
            scope.spawn(move || {
                let interval = Duration::from_secs(k::TOKEN_MONITOR_INTERVAL_SECONDS);
                let mut last = Instant::now();
                while !finished.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(100));
                    if last.elapsed() >= interval {
                        last = Instant::now();
                        if !present() {
                            token_removed.store(true, Ordering::SeqCst);
                            return;
                        }
                    }
                }
            });
        }
        for _ in 0..parallelism().min(total.max(1)) {
            let tx = tx.clone();
            let (next, taken, images, signer, clock) = (&next, &taken, &images, &signer, &clock);
            let stop = &stop;
            scope.spawn(move || {
                loop {
                    if stop() {
                        return;
                    }
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(source) = request.files.get(index) else {
                        return;
                    };
                    let result = sign_one(
                        source, request, key, chain, tsa, ltv, clock, images, signer, taken,
                    );
                    if tx.send((index, result)).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);
        for (index, result) in rx {
            done[index] = true;
            done_count += 1;
            let name = request.files[index]
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let event = match result {
                Ok((path, check)) => {
                    success += 1;
                    JobEvent::File {
                        index,
                        name,
                        success: true,
                        output_path: Some(path.display().to_string()),
                        reason: check.reason.clone(),
                        verdict: Some(check.verdict),
                        trusted_time: check.trusted_time.clone(),
                        paused: false,
                        done: done_count,
                        total,
                    }
                }
                Err(reason) => {
                    failed += 1;
                    JobEvent::File {
                        index,
                        name,
                        success: false,
                        output_path: None,
                        reason: Some(reason),
                        verdict: None,
                        trusted_time: None,
                        paused: false,
                        done: done_count,
                        total,
                    }
                }
            };
            emit(event);
        }
        finished.store(true, Ordering::SeqCst);
    });

    let removed = token_removed.load(Ordering::SeqCst);
    let pending_reason = if removed {
        "Chưa ký: đã rút USB token khi đang ký nên tạm dừng."
    } else {
        "Chưa ký: đã tạm dừng."
    };
    let mut paused = 0;
    for (index, was_done) in done.iter().enumerate() {
        if *was_done {
            continue;
        }
        paused += 1;
        emit(JobEvent::File {
            index,
            name: request.files[index]
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            success: false,
            output_path: None,
            reason: Some(pending_reason.to_owned()),
            verdict: None,
            trusted_time: None,
            paused: true,
            done: done_count,
            total,
        });
    }
    emit(JobEvent::Summary {
        job_id: job_id.to_owned(),
        total,
        success,
        failed,
        paused,
        stopped: cancel.load(Ordering::SeqCst) || removed,
        token_removed: removed,
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        output_folder,
        clock_note: clock.note.clone(),
    });
}

/// Cấu hình hiển thị của một file: kiểu riêng của file (nếu có) thay cho `a.mode`.
pub fn with_mode(a: &Appearance, mode: Option<SignatureMode>) -> std::borrow::Cow<'_, Appearance> {
    match mode {
        Some(m) if m != a.mode => std::borrow::Cow::Owned(Appearance {
            mode: m,
            ..a.clone()
        }),
        _ => std::borrow::Cow::Borrowed(a),
    }
}

#[allow(clippy::too_many_arguments)]
fn sign_one(
    source: &Path,
    request: &JobRequest,
    key: &dyn SigningKey,
    chain: &[Cert],
    tsa: Option<&TsaClient>,
    ltv: Option<&smks_crypto::ltv::LtvCache>,
    clock: &BatchClock,
    images: &Result<Images>,
    signer: &str,
    taken: &Mutex<HashSet<String>>,
) -> std::result::Result<(PathBuf, crate::verify::SignatureReport), String> {
    let images = images.as_ref().map_err(|e| e.message.clone())?;
    let bytes = std::fs::read(source).map_err(|e| format!("Không đọc được file: {e}"))?;
    // Giờ ký (dòng "Ngày ký" và `/M`): giờ máy chủ dấu thời gian, hoặc giờ máy tính khi không lấy được (TD-54).
    let now = crate::verify::clock_now() + clock.offset;
    let now_text = crate::verify::vietnam_text(now);
    let key_path = source.display().to_string();
    let manual: &[Placement] = request
        .placements
        .get(&key_path)
        .filter(|l| !l.is_empty())
        .unwrap_or(&request.appearance.placements);
    let appearance = with_mode(&request.appearance, request.modes.get(&key_path).copied());
    let sign_request = build_request(
        &bytes,
        signer,
        &appearance,
        manual,
        images,
        now,
        now_text.get(..16).unwrap_or(&now_text),
    )
    .map_err(|e| e.message)?;
    let signed = sign_pdf(&bytes, &sign_request, key, chain, tsa).map_err(|e| e.message)?;
    // Ký theo giờ máy tính (không lấy được giờ máy chủ): chữ ký hợp lệ nhưng "Không có dấu thời gian" là đúng ý.
    let accepted = signed.check.verdict == Verdict::ValidTrusted
        || (clock.clock == Clock::Computer && signed.check.verdict == Verdict::NoTimestamp);
    if request.require_trusted && !accepted {
        return Err(signed
            .check
            .reason
            .clone()
            .unwrap_or_else(|| signed.check.verdict.label().to_owned()));
    }
    // Ký dài hạn: thêm DSS (chứng thư, OCSP / CRL). Không lấy được thì giữ bản B-T, kèm lưu ý.
    let mut signed = signed;
    if let Some(cache) = ltv {
        let outcome = crate::sign::add_ltv(&signed.bytes, cache);
        signed.bytes = outcome.bytes;
        signed.check.long_term = outcome.long_term;
        if let Some(note) = outcome.note {
            signed.check.reason = Some(match signed.check.reason.take() {
                Some(r) => format!("{r} {note}"),
                None => note,
            });
        }
    }
    if request.output.replace_original {
        // ADR 0003: chỉ thay file gốc khi chữ ký vừa ký kiểm lại hợp lệ (bản sản phẩm đã đòi "Hợp lệ" ở trên).
        if matches!(
            signed.check.verdict,
            Verdict::InvalidSignature | Verdict::NotApplicable
        ) {
            return Err(format!(
                "Chữ ký vừa ký không hợp lệ ({}) nên không thay file gốc.",
                signed.check.verdict.label()
            ));
        }
        let (path, note) = replace_source(
            source,
            &signed.bytes,
            request,
            taken,
            &crate::ffi::move_to_recycle_bin,
        )?;
        let mut check = signed.check;
        if let Some(note) = note {
            check.reason = Some(match check.reason {
                Some(r) => format!("{r} {note}"),
                None => note,
            });
        }
        return Ok((path, check));
    }
    let path = reserve_output(source, &request.output, taken).map_err(|e| e.message)?;
    write_atomic(&path, &signed.bytes).map_err(|e| e.message)?;
    Ok((path, signed.check))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smks_crypto::testpki::TestPki;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/pdf")
            .join(name)
    }

    fn workdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smks-job-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (i, f) in ["table-xref.pdf", "xref-stream.pdf", "word-hybrid-xref.pdf"]
            .iter()
            .enumerate()
        {
            std::fs::copy(
                fixture(f),
                dir.join(format!("van ban {}.pdf", [10, 2, 1][i])),
            )
            .unwrap();
        }
        std::fs::write(dir.join("ghi chu.txt"), "x").unwrap();
        std::fs::copy(
            fixture("table-xref.pdf"),
            dir.join("cu_daky_20260101_101010.pdf"),
        )
        .unwrap();
        dir
    }

    #[test]
    fn positions_follow_profile_drag_or_legacy_default() {
        use smks_core::config::{Anchor, MarginMm, PagePick, SignatureProfile, default_profiles};
        let bytes = std::fs::read(fixture("word-hybrid-xref.pdf")).unwrap();
        let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
        // Không có mẫu, không kéo: góc trên phải trang 1, cách mép 8 pt (bản cũ).
        let legacy = Appearance::default();
        let p = preview_positions(&bytes, &legacy, &[], "Nguyễn Văn A").unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].page, 1);
        assert!(
            close(p[0].x_ratio + p[0].width_ratio, 1.0 - 8.0 / 595.32),
            "{p:?}"
        );
        // Mẫu A4: dưới phải, trang cuối, lề 15 / 20 mm, khung ảnh 100%.
        let mut profiles = default_profiles();
        profiles[0] = SignatureProfile {
            anchor: Anchor::Br,
            page: PagePick::Last,
            margin_mm: MarginMm { x: 15.0, y: 20.0 },
            ..profiles[0].clone()
        };
        let a = Appearance {
            mode: SignatureMode::Image,
            profiles: profiles.clone(),
            ..Appearance::default()
        };
        let p = preview_positions(&bytes, &a, &[], "Nguyễn Văn A").unwrap();
        let mm = smks_core::config::PT_PER_MM;
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].page, 3, "trang cuối của file 3 trang");
        assert!(
            close(p[0].x_ratio + p[0].width_ratio, 1.0 - 15.0 * mm / 595.32),
            "{p:?}"
        );
        assert!(
            close(p[0].y_ratio + p[0].height_ratio, 1.0 - 20.0 * mm / 841.92),
            "{p:?}"
        );
        assert!(close(
            p[0].width_ratio * 595.32,
            170.0 * (595.32_f64 / 595.0).min(841.92 / 842.0)
        ));
        // Mọi trang (ký nháy): mỗi trang một khối.
        profiles[0].page = PagePick::All;
        let a = Appearance {
            profiles: profiles.clone(),
            ..a
        };
        let p = preview_positions(&bytes, &a, &[], "Nguyễn Văn A").unwrap();
        assert_eq!(p.iter().map(|x| x.page).collect::<Vec<_>>(), [1, 2, 3]);
        // Vị trí kéo tay thắng mẫu; trang 0 là trang cuối.
        let drag = Placement {
            page: 0,
            x_ratio: 0.1,
            y_ratio: 0.5,
            width_ratio: 0.3,
            height_ratio: 0.05,
        };
        let p = preview_positions(&bytes, &a, &[drag], "Nguyễn Văn A").unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].page, 3);
        assert!(
            close(p[0].x_ratio, 0.1) && close(p[0].y_ratio, 0.5) && close(p[0].width_ratio, 0.3)
        );
        // Ẩn: không có khối nào.
        let hidden = Appearance {
            mode: SignatureMode::Hidden,
            ..a.clone()
        };
        assert!(
            preview_positions(&bytes, &hidden, &[], "A")
                .unwrap()
                .is_empty()
        );
        // Bố cục cho màn Vị trí chữ ký: số trang, khổ trang, chỗ theo mẫu và chỗ đang đặt.
        let l = preview_layout(&bytes, &a, &[], "A").unwrap();
        assert_eq!(l.page_count, 3);
        assert!(
            close(l.page_width, 595.32) && close(l.page_height, 841.92),
            "{l:?}"
        );
        assert_eq!(l.boxes, l.template, "chưa kéo tay: khung theo mẫu");
        assert_eq!(l.template.len(), 3);
        let l = preview_layout(&bytes, &a, &[drag], "A").unwrap();
        assert_eq!(l.boxes.len(), 1);
        assert_eq!(l.boxes[0].page, 3);
        assert_eq!(l.template.len(), 3, "chỗ theo mẫu không đổi khi đã kéo tay");
        // Ẩn: vẫn tính khung (tham khảo).
        let l = preview_layout(&bytes, &hidden, &[], "A").unwrap();
        assert_eq!(l.boxes.len(), 3);
    }

    #[test]
    fn natural_order_skips_legacy_and_non_pdf() {
        let dir = workdir("list");
        let files = list_pdf_files(&[dir.clone(), dir.join("van ban 1.pdf")]).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["van ban 1.pdf", "van ban 2.pdf", "van ban 10.pdf"]);
        assert!(
            files
                .iter()
                .all(|f| f.signature_count == 0 && !f.unreadable)
        );
        assert!(is_legacy_signed_name("abc_daky_20260101_101010"));
        assert!(!is_legacy_signed_name("abc_daky_2026"));
        assert!(list_pdf_files(&[dir.join("khong-co")]).is_err());
    }

    #[test]
    fn batch_signs_into_dakyso_and_keeps_both_on_conflict() {
        let dir = workdir("run");
        let pki = TestPki::new();
        let key = pki.software_key();
        let files: Vec<PathBuf> = list_pdf_files(std::slice::from_ref(&dir))
            .unwrap()
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .collect();
        let request = JobRequest {
            files: files.clone(),
            ..JobRequest::default()
        };
        validate(&request).unwrap();
        let mut events = Vec::new();
        run(
            "j1",
            &request,
            &key,
            &[],
            None,
            &AtomicBool::new(false),
            None,
            &mut |e| events.push(e),
        );
        let Some(JobEvent::Summary {
            success,
            failed,
            stopped,
            output_folder,
            ..
        }) = events.last()
        else {
            panic!()
        };
        assert_eq!((*success, *failed, *stopped), (3, 0, false), "{events:?}");
        assert!(output_folder.ends_with("DaKySo"));
        let out = dir.join("DaKySo");
        assert!(out.join("van ban 1_signed.pdf").is_file());
        // Gốc không đổi.
        assert_eq!(
            std::fs::read(dir.join("van ban 1.pdf")).unwrap(),
            std::fs::read(fixture("word-hybrid-xref.pdf")).unwrap()
        );
        // Ký lại: giữ cả hai.
        let mut again = Vec::new();
        run(
            "j2",
            &request,
            &key,
            &[],
            None,
            &AtomicBool::new(false),
            None,
            &mut |e| again.push(e),
        );
        assert!(out.join("van ban 1_signed (1).pdf").is_file());
        // Không còn file .tmp.
        assert!(
            std::fs::read_dir(&out)
                .unwrap()
                .flatten()
                .all(|e| !e.path().to_string_lossy().ends_with(".tmp"))
        );
    }

    #[test]
    fn batch_clock_uses_server_time_or_falls_back_to_computer() {
        let tsa = TsaClient::new("http://127.0.0.1:9/");
        // Lấy được giờ máy chủ: giờ ký = đồng hồ máy + độ lệch (máy chậm 1 giờ thì cộng 1 giờ).
        let c = BatchClock::pick(Some(&tsa), &|_| Ok(1_800_003_600), &|| 1_800_000_000);
        assert_eq!((c.clock, c.offset, c.note), (Clock::Tsa, 3600, None));
        // Không kết nối được: giờ máy tính, có lý do.
        let c = BatchClock::pick(
            Some(&tsa),
            &|t| t.server_time(Duration::from_secs(3)),
            &|| 1_800_000_000,
        );
        assert_eq!((c.clock, c.offset), (Clock::Computer, 0));
        let note = c.note.unwrap();
        assert!(
            note.contains("Ban Cơ yếu")
                && note.contains("giờ máy tính (15/01/2027 15:00:00)")
                && note.contains("không có dấu thời gian"),
            "{note}"
        );
        // Không có TSA (chạy thử): giờ máy tính, không nhắc.
        let c = BatchClock::pick(None, &|_| unreachable!(), &|| 0);
        assert_eq!((c.clock, c.offset, c.note), (Clock::Computer, 0, None));
    }

    #[test]
    fn offline_batch_signs_with_computer_time_without_timestamp() {
        let dir = workdir("offline");
        let pki = TestPki::new();
        let key = pki.software_key();
        let files: Vec<PathBuf> = list_pdf_files(std::slice::from_ref(&dir))
            .unwrap()
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .collect();
        let request = JobRequest {
            files,
            ..JobRequest::default()
        };
        let tsa = TsaClient::new("http://127.0.0.1:9/");
        let before = crate::verify::clock_now();
        let mut events = Vec::new();
        run(
            "j-offline",
            &request,
            &key,
            &[],
            Some(&tsa),
            &AtomicBool::new(false),
            None,
            &mut |e| events.push(e),
        );
        let Some(JobEvent::Manifest {
            clock, clock_note, ..
        }) = events.first()
        else {
            panic!()
        };
        assert_eq!(*clock, Clock::Computer);
        assert!(clock_note.as_deref().unwrap_or("").contains("giờ máy tính"));
        let Some(JobEvent::Summary {
            success,
            failed,
            clock_note,
            ..
        }) = events.last()
        else {
            panic!()
        };
        assert_eq!((*success, *failed), (3, 0), "{events:?}");
        assert!(clock_note.is_some());
        // File đã ký: không có dấu thời gian, giờ ký tự khai là giờ máy.
        let bytes = std::fs::read(dir.join("DaKySo").join("van ban 1_signed.pdf")).unwrap();
        let report =
            crate::verify::verify_pdf(&bytes, crate::verify::VerifyOptions::default()).unwrap();
        let sig = report.signatures.last().unwrap();
        assert!(sig.trusted_time.is_none(), "{sig:?}");
        let m = sig.pdf_time.clone().unwrap();
        let v = crate::verify::vietnam_text(before);
        let hour = format!("D:{}{}{}{}", &v[6..10], &v[3..5], &v[..2], &v[11..13]);
        assert!(m.starts_with(&hour), "{m} / {v}");
    }

    #[test]
    fn per_file_modes_override_appearance_mode() {
        let dir = workdir("modes");
        let pki = TestPki::new();
        let key = pki.software_key();
        let files: Vec<PathBuf> = list_pdf_files(std::slice::from_ref(&dir))
            .unwrap()
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .collect();
        let key_of = |i: usize| files[i].display().to_string();
        // Kiểu chung "Chỉ thông tin"; file 0 ẩn, file 1 "Ảnh + thông tin" khi chưa có ảnh → file 1 phải lỗi.
        let request = JobRequest {
            files: files.clone(),
            modes: HashMap::from([
                (key_of(0), SignatureMode::Hidden),
                (key_of(1), SignatureMode::ImageAndText),
            ]),
            ..JobRequest::default()
        };
        let mut events = Vec::new();
        run(
            "jm",
            &request,
            &key,
            &[],
            None,
            &AtomicBool::new(false),
            None,
            &mut |e| events.push(e),
        );
        let result = |i: usize| {
            let name = files[i].file_name().unwrap().to_string_lossy().to_string();
            events
                .iter()
                .find_map(|e| match e {
                    JobEvent::File {
                        name: n,
                        success,
                        reason,
                        ..
                    } if *n == name => Some((*success, reason.clone())),
                    _ => None,
                })
                .unwrap()
        };
        assert!(result(0).0, "{events:?}");
        let (ok1, reason1) = result(1);
        assert!(
            !ok1 && reason1.unwrap().contains("chưa chọn ảnh"),
            "{events:?}"
        );
        assert!(result(2).0, "{events:?}");
        let a = Appearance::default();
        assert!(matches!(with_mode(&a, None), std::borrow::Cow::Borrowed(_)));
        assert_eq!(
            with_mode(&a, Some(SignatureMode::Image)).mode,
            SignatureMode::Image
        );
    }

    /// Lượt ký nháy: mặc định mọi trang ở góc dưới đối diện chữ ký chính. Nhiều khung tự đặt: mỗi khung một khối,
    /// khung trang vượt số trang về trang cuối, trùng chỗ thì bỏ.
    #[test]
    fn initial_kind_and_box_lists() {
        use smks_core::config::{Anchor, default_profiles};
        let bytes = std::fs::read(fixture("word-hybrid-xref.pdf")).unwrap();
        let mm = smks_core::config::PT_PER_MM;
        let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
        let main = Appearance {
            profiles: default_profiles(),
            ..Appearance::default()
        };
        let initial = Appearance {
            kind: SignKind::Initial,
            ..main.clone()
        };
        let p = preview_positions(&bytes, &main, &[], "A").unwrap();
        assert_eq!(p.iter().map(|x| x.page).collect::<Vec<_>>(), [3]);
        assert!(close(
            p[0].x_ratio + p[0].width_ratio,
            1.0 - 15.0 * mm / 595.32
        ));
        let p = preview_positions(&bytes, &initial, &[], "A").unwrap();
        assert_eq!(p.iter().map(|x| x.page).collect::<Vec<_>>(), [1, 2, 3]);
        for x in &p {
            assert!(close(x.x_ratio, 15.0 * mm / 595.32), "góc dưới trái: {x:?}");
            assert!(close(x.y_ratio + x.height_ratio, 1.0 - 20.0 * mm / 841.92));
        }
        // Không có mẫu: chữ ký chính về chỗ bản cũ (1 khối trang 1), ký nháy vẫn mọi trang.
        let bare = Appearance {
            kind: SignKind::Initial,
            ..Appearance::default()
        };
        assert_eq!(preview_positions(&bytes, &bare, &[], "A").unwrap().len(), 3);
        assert_eq!(initial_anchor(Anchor::Br), Anchor::Bl);
        assert_eq!(initial_anchor(Anchor::Tc), Anchor::Bl);
        assert_eq!(initial_anchor(Anchor::Bl), Anchor::Br);
        // Danh sách khung tự đặt.
        let at = |page: u32, x: f64| Placement {
            page,
            x_ratio: x,
            y_ratio: 0.8,
            width_ratio: 0.2,
            height_ratio: 0.05,
        };
        let list = [at(1, 0.1), at(2, 0.5), at(9, 0.3), at(3, 0.3), at(0, 0.6)];
        let p = preview_positions(&bytes, &initial, &list, "A").unwrap();
        assert_eq!(
            p.iter()
                .map(|x| (x.page, (x.x_ratio * 10.0).round() as i32))
                .collect::<Vec<_>>(),
            [(1, 1), (2, 5), (3, 3), (3, 6)],
            "trang 9 về trang cuối và trùng khung trang 3 thì bỏ; trang 0 là trang cuối"
        );
        let l = preview_layout(&bytes, &initial, &list, "A").unwrap();
        assert_eq!(l.boxes.len(), 4);
        assert_eq!(
            l.template.len(),
            3,
            "chỗ theo mẫu của lượt ký nháy: mọi trang"
        );
    }

    /// Ảnh trên / dưới chữ: khối mặc định 130 × 66 pt (hẹp, cao); ảnh trái / phải giữ 170 × 30 pt. Ký được với
    /// cả 4 bố cục.
    #[test]
    fn image_layout_changes_box_size_and_signs() {
        use smks_core::config::default_profiles;
        let bytes = std::fs::read(fixture("word-hybrid-xref.pdf")).unwrap();
        let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
        let base = Appearance {
            mode: SignatureMode::ImageAndText,
            profiles: default_profiles(),
            ..Appearance::default()
        };
        let p = preview_positions(&bytes, &base, &[], "A").unwrap();
        assert!(close(
            p[0].width_ratio * 595.32,
            170.0 * (595.32_f64 / 595.0).min(841.92 / 842.0)
        ));
        let stacked = Appearance {
            image_layout: ImageLayout::Top,
            ..base.clone()
        };
        let q = preview_positions(&bytes, &stacked, &[], "A").unwrap();
        let k = (595.32_f64 / 595.0).min(841.92 / 842.0);
        assert!(close(q[0].width_ratio * 595.32, 130.0 * k), "{q:?}");
        assert!(close(q[0].height_ratio * 841.92, 66.0 * k), "{q:?}");
        // Ký thật với ảnh dấu ở cả 4 bố cục.
        let png = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Hai_Chu_Ky.png"),
        )
        .unwrap();
        let image = StampImage::prepare(&png, 140, None, 400_000).unwrap();
        let images = Images {
            seal: Some(image),
            wet: None,
            wet_stroke: 0,
        };
        for layout in [
            ImageLayout::Left,
            ImageLayout::Right,
            ImageLayout::Top,
            ImageLayout::Bottom,
        ] {
            let a = Appearance {
                image_layout: layout,
                signature_image: Some(StampKind::Seal),
                ..base.clone()
            };
            let req = build_request(
                &bytes,
                "Nguyễn Văn A",
                &a,
                &[],
                &images,
                0,
                "20/11/2026 10:00",
            )
            .unwrap();
            assert_eq!(req.widgets.len(), 1);
            assert!(matches!(
                req.widgets[0].content,
                WidgetContent::ImageAndText { layout: l, .. } if l == layout
            ));
            smks_pdf::incremental::prepare(&bytes, &req).unwrap();
            // Tự chỉnh phần ảnh và khoảng cách (TD-51): truyền đúng xuống khối, vẫn dựng được.
            let tuned = Appearance {
                image_share: Some(30),
                image_gap: Some(8),
                ..a.clone()
            };
            let req = build_request(
                &bytes,
                "Nguyễn Văn A",
                &tuned,
                &[],
                &images,
                0,
                "20/11/2026 10:00",
            )
            .unwrap();
            assert!(matches!(
                req.widgets[0].content,
                WidgetContent::ImageAndText { split, .. }
                    if split == ImageSplit { share: Some(30), gap: Some(8), free: None }
            ));
            smks_pdf::incremental::prepare(&bytes, &req).unwrap();
            // Tự do (TD-52): có cả hai ô thì truyền xuống khối, vẫn dựng được.
            let free = Appearance {
                image_box: Some(PartBox {
                    x: 0.5,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0,
                }),
                text_box: Some(PartBox {
                    x: 0.0,
                    y: 0.2,
                    w: 0.6,
                    h: 0.6,
                }),
                ..a.clone()
            };
            let req = build_request(
                &bytes,
                "Nguyễn Văn A",
                &free,
                &[],
                &images,
                0,
                "20/11/2026 10:00",
            )
            .unwrap();
            assert!(matches!(
                req.widgets[0].content,
                WidgetContent::ImageAndText {
                    split: ImageSplit { free: Some(_), .. },
                    ..
                }
            ));
            smks_pdf::incremental::prepare(&bytes, &req).unwrap();
        }
    }

    /// Hai lượt: ký nháy rồi ký chính lên bản đã ký nháy. Mỗi lượt đúng một chữ ký; file cuối có hai chữ ký,
    /// chữ ký nháy hiện ở 3 trang, chữ ký chính ở trang cuối; chữ ký trước vẫn nguyên vẹn.
    #[test]
    fn initial_then_main_gives_two_signatures() {
        use smks_core::config::default_profiles;
        let dir = std::env::temp_dir().join("smks-job-nhay-chinh");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("hop dong.pdf");
        std::fs::copy(fixture("word-hybrid-xref.pdf"), &source).unwrap();
        let pki = TestPki::new();
        let key = pki.software_key();
        let pass = |file: PathBuf, kind: SignKind| {
            let request = JobRequest {
                files: vec![file],
                appearance: Appearance {
                    kind,
                    profiles: default_profiles(),
                    ..Appearance::default()
                },
                ..JobRequest::default()
            };
            let mut out = None;
            run(
                "nc",
                &request,
                &key,
                &[],
                None,
                &AtomicBool::new(false),
                None,
                &mut |e| {
                    if let JobEvent::File {
                        success: true,
                        output_path,
                        ..
                    } = e
                    {
                        out = output_path;
                    }
                },
            );
            PathBuf::from(out.expect("lượt ký phải thành công"))
        };
        let once = pass(source.clone(), SignKind::Initial);
        let first = std::fs::read(&once).unwrap();
        assert_eq!(smks_pdf::sigscan::count(&first), 1, "một lượt = một chữ ký");
        let twice = pass(once, SignKind::Main);
        let last = std::fs::read(&twice).unwrap();
        assert!(last.starts_with(&first), "ký chồng: bản trước là tiền tố");
        let fields = smks_pdf::sigscan::scan(&last).unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().all(|f| f.range_ok && f.visible));
        // Widget của từng chữ ký: đọc các /Annots của từng trang theo tên field cha.
        let text = String::from_utf8_lossy(&last);
        assert!(!fields[0].covers_whole_file && fields[1].covers_whole_file);
        assert_eq!(fields[1].page, Some(3), "chữ ký chính ở trang cuối");
        assert!(text.contains("/Kids"), "chữ ký nháy nhiều khối dùng /Kids");
    }

    /// Ký cả thư mục (TD-50): liệt kê cả thư mục con, bỏ qua thư mục kết quả cũ, lưu vào `<tên>_DaKySo` cùng cấp
    /// và giữ nguyên cấu trúc thư mục con.
    #[test]
    fn folder_source_mirrors_tree_into_sibling_folder() {
        let base = std::env::temp_dir().join("smks-job-mirror");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("HoSo");
        std::fs::create_dir_all(root.join("PhongA").join("To 1")).unwrap();
        std::fs::create_dir_all(root.join("DaKySo")).unwrap();
        std::fs::copy(fixture("table-xref.pdf"), root.join("a.pdf")).unwrap();
        std::fs::copy(
            fixture("xref-stream.pdf"),
            root.join("PhongA").join("b.pdf"),
        )
        .unwrap();
        std::fs::copy(
            fixture("word-hybrid-xref.pdf"),
            root.join("PhongA").join("To 1").join("c.pdf"),
        )
        .unwrap();
        // Bản đã ký của lần trước (theo cách cũ) không được liệt kê lại.
        std::fs::copy(
            fixture("table-xref.pdf"),
            root.join("DaKySo").join("a_signed.pdf"),
        )
        .unwrap();

        let items = list_pdf_files(std::slice::from_ref(&root)).unwrap();
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            items.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
            [
                "a.pdf".to_owned(),
                format!("PhongA{sep}b.pdf"),
                format!("PhongA{sep}To 1{sep}c.pdf"),
            ]
        );
        let root_str = std::path::absolute(&root).unwrap().display().to_string();
        assert!(
            items
                .iter()
                .all(|f| f.root.as_deref() == Some(root_str.as_str()))
        );

        let files: Vec<PathBuf> = items.iter().map(|f| PathBuf::from(&f.path)).collect();
        let out = OutputOptions {
            source_roots: vec![PathBuf::from(&root_str)],
            ..OutputOptions::default()
        };
        let mirror = std::path::absolute(base.join("HoSo_DaKySo")).unwrap();
        let p = plan_outputs(&files, &out);
        assert_eq!(
            p.iter()
                .map(|x| PathBuf::from(&x.output))
                .collect::<Vec<_>>(),
            [
                mirror.join("a_signed.pdf"),
                mirror.join("PhongA").join("b_signed.pdf"),
                mirror.join("PhongA").join("To 1").join("c_signed.pdf"),
            ]
        );

        // Thư mục cố định: thư mục song song nằm trong thư mục cố định.
        let fixed = OutputOptions {
            fixed_folder: Some(base.join("Ra")),
            ..out.clone()
        };
        assert_eq!(
            PathBuf::from(&plan_outputs(&files[1..2], &fixed)[0].output),
            base.join("Ra")
                .join("HoSo_DaKySo")
                .join("PhongA")
                .join("b_signed.pdf")
        );

        // Ký thật: file kết quả đúng chỗ, báo thư mục song song, thư mục gốc không thêm file.
        let pki = TestPki::new();
        let key = pki.software_key();
        let request = JobRequest {
            files: files.clone(),
            output: out,
            ..JobRequest::default()
        };
        let mut events = Vec::new();
        run(
            "mirror",
            &request,
            &key,
            &[],
            None,
            &AtomicBool::new(false),
            None,
            &mut |e| events.push(e),
        );
        let Some(JobEvent::Summary {
            success,
            failed,
            output_folder,
            ..
        }) = events.last()
        else {
            panic!()
        };
        assert_eq!((*success, *failed), (3, 0), "{events:?}");
        assert_eq!(PathBuf::from(output_folder), mirror);
        assert!(
            mirror
                .join("PhongA")
                .join("To 1")
                .join("c_signed.pdf")
                .is_file()
        );
        assert!(!root.join("PhongA").join("DaKySo").exists());
        // Liệt kê lại thư mục gốc vẫn chỉ ra 3 file gốc.
        assert_eq!(
            list_pdf_files(std::slice::from_ref(&root)).unwrap().len(),
            3
        );
    }

    #[test]
    fn plan_outputs_reports_existing_names() {
        let dir = workdir("plan");
        let files = vec![dir.join("van ban 1.pdf"), dir.join("van ban 2.pdf")];
        let out = OutputOptions::default();
        let p = plan_outputs(&files, &out);
        assert!(p.iter().all(|x| !x.exists));
        assert_eq!(
            PathBuf::from(&p[0].output),
            dir.join("DaKySo").join("van ban 1_signed.pdf")
        );
        std::fs::create_dir_all(dir.join("DaKySo")).unwrap();
        std::fs::write(dir.join("DaKySo").join("van ban 2_signed.pdf"), b"%PDF").unwrap();
        let p = plan_outputs(&files, &out);
        assert_eq!(
            p.iter().map(|x| x.exists).collect::<Vec<_>>(),
            [false, true]
        );
        // Xóa file gốc: kết quả đúng chỗ file gốc, không tính là trùng.
        let replace = OutputOptions {
            replace_original: true,
            ..out
        };
        let p = plan_outputs(&files, &replace);
        assert!(p.iter().all(|x| !x.exists && x.output == x.source));
    }

    #[test]
    fn replace_source_moves_original_to_trash_or_keeps_it() {
        let dir = workdir("replace");
        let source = dir.join("van ban 2.pdf");
        let original = std::fs::read(&source).unwrap();
        let request = JobRequest {
            output: OutputOptions {
                replace_original: true,
                ..OutputOptions::default()
            },
            ..JobRequest::default()
        };
        let taken = Mutex::new(HashSet::new());
        // Thùng rác giả: chuyển file sang thư mục riêng.
        let bin = dir.join("thung-rac");
        std::fs::create_dir_all(&bin).unwrap();
        let fake = |p: &Path| {
            std::fs::rename(p, bin.join(p.file_name().unwrap())).map_err(|e| e.to_string())
        };
        let (path, note) = replace_source(&source, b"%PDF-da-ky", &request, &taken, &fake).unwrap();
        assert_eq!(path, source);
        assert!(note.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), b"%PDF-da-ky");
        assert_eq!(
            std::fs::read(bin.join("van ban 2.pdf")).unwrap(),
            original,
            "file gốc nằm trong Thùng rác"
        );
        // Không đưa được vào Thùng rác: giữ file gốc, lưu bản đã ký vào DaKySo, có lý do.
        let other = dir.join("van ban 1.pdf");
        let before = std::fs::read(&other).unwrap();
        let fail = |_: &Path| Err("ổ mạng".to_owned());
        let (path, note) = replace_source(&other, b"%PDF-da-ky", &request, &taken, &fail).unwrap();
        assert_eq!(std::fs::read(&other).unwrap(), before);
        assert!(path.ends_with("DaKySo/van ban 1_signed.pdf"), "{path:?}");
        assert!(note.unwrap().contains("ổ mạng"));
        assert!(
            std::fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .all(|e| !e.path().to_string_lossy().ends_with(".tmp"))
        );
    }

    #[test]
    fn cancel_and_token_removal_report_pending_files() {
        let dir = workdir("stop");
        let pki = TestPki::new();
        let key = pki.software_key();
        let files: Vec<PathBuf> = list_pdf_files(std::slice::from_ref(&dir))
            .unwrap()
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .collect();
        let request = JobRequest {
            files,
            ..JobRequest::default()
        };
        let mut events = Vec::new();
        run(
            "j3",
            &request,
            &key,
            &[],
            None,
            &AtomicBool::new(true),
            None,
            &mut |e| events.push(e),
        );
        let Some(JobEvent::Summary {
            success,
            failed,
            paused,
            stopped,
            ..
        }) = events.last()
        else {
            panic!()
        };
        // Tạm dừng: file chưa ký là "tạm dừng", không tính là lỗi.
        assert_eq!((*success, *failed, *paused, *stopped), (0, 0, 3, true));
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, JobEvent::File { paused: true, success: false, reason: Some(r), .. } if r == "Chưa ký: đã tạm dừng."))
                .count(),
            3
        );
        // Token biến mất: theo dõi phát hiện, các file còn lại báo đúng lý do.
        let gone = || false;
        let slow_key = SlowKey(pki.software_key());
        let mut events = Vec::new();
        run(
            "j4",
            &request,
            &slow_key,
            &[],
            None,
            &AtomicBool::new(false),
            Some(&gone),
            &mut |e| events.push(e),
        );
        let Some(JobEvent::Summary {
            token_removed,
            paused,
            ..
        }) = events.last()
        else {
            panic!()
        };
        assert!(*token_removed);
        // File chưa kịp ký khi rút token (nếu có) là "tạm dừng" với lý do rút token, không tính là lỗi.
        let paused_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                JobEvent::File {
                    paused: true,
                    reason,
                    ..
                } => Some(reason.clone().unwrap_or_default()),
                _ => None,
            })
            .collect();
        assert_eq!(paused_events.len(), *paused);
        assert!(paused_events.iter().all(|r| r.contains("rút USB token")));
    }

    /// Khoá ký chậm 2,5 giây để theo dõi token kịp phát hiện.
    struct SlowKey(smks_crypto::key::software::SoftwareKey);

    impl SigningKey for SlowKey {
        fn certificate(&self) -> &Cert {
            self.0.certificate()
        }
        fn algorithm(&self) -> smks_crypto::key::SigAlg {
            self.0.algorithm()
        }
        fn sign_digest(&self, d: &[u8; 32]) -> Result<Vec<u8>> {
            std::thread::sleep(Duration::from_millis(2500));
            self.0.sign_digest(d)
        }
        fn provider(&self) -> String {
            self.0.provider()
        }
    }
}
