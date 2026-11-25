//! Gói phiên làm việc: mang phần đang làm dở (nhãn, góc xoay, cờ duyệt, mốc sửa, kết quả phân tích — kho theo id nội
//! dung PDF — và kết quả bóc tách) của các PDF trong 1 thư mục đầu vào sang máy khác để làm tiếp.
//!
//! - **Xuất**: 1 file zip: `manifest.json` (PDF: đường dẫn tương đối, kích thước, mã nội dung), `tach.bin` + `phan_tich.bin`
//!   (bản ghi kho theo id), `boc_tach.jsonl` (kèm id), `xoa_ghim.bin` (vùng + nhật ký Xóa ghim — mục tuỳ chọn, bản cũ bỏ
//!   qua). Không bao giờ có khoá API, license, cấu hình, mã máy.
//! - **Nhập**: khớp **theo nội dung PDF** (id) với PDF trong thư mục đang mở — máy nhận mở thư mục ở cấp nào, đổi tên
//!   thế nào cũng khớp; gộp theo luật `gop` (mốc sau cùng thắng; kết quả lỗi không đè kết quả tốt). Gói bản 2.3
//!   (theo đường dẫn) vẫn nhập được. Không tin gì trong zip: chỉ đọc đúng tên cần, có giới hạn dung lượng.

mod gop;
mod xoa_ghim;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::boc_tach::loai_tai_lieu::{self, LoaiTaiLieu};
use crate::boc_tach::luu::{self, BanGhi};
use crate::tach_vb::cache_trang;
use crate::tach_vb::du_lieu_an::thu_muc_an;
use crate::tach_vb::kho::{self, BanTach, Id, Kho};
use crate::tach_vb::moc_sua::{self, Moc};
use crate::tach_vb::nhan;

/// Gói từ bản app mới hơn (phiên bản gói lớn hơn) → từ chối, không đoán.
pub const PHIEN_BAN_GOI: u32 = 2;
/// Đuôi tên gói: `<tên thư mục>.sm-dema4.zip`.
pub const DUOI_GOI: &str = ".sm-dema4.zip";

const F_MANIFEST: &str = "manifest.json";
const F_NHAN: &str = "index_and_blank.txt";
const F_XOAY: &str = "xoay_trang.txt";
const F_JSONL: &str = "boc_tach.jsonl";
const THU_MUC_CACHE: &str = "cache";
const F_TACH: &str = "tach.bin";
const F_PHAN_TICH: &str = "phan_tich.bin";

const TOI_DA_MANIFEST: u64 = 64 << 20;
const TOI_DA_FILE: u64 = 200 << 20;
const TOI_DA_BIN: u64 = 1 << 20;
const TOI_DA_KHO: u64 = 2 << 30;
const TOI_DA_MUC: usize = 500_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    phien_ban_goi: u32,
    phien_ban_app: String,
    ten_thu_muc: String,
    /// Giờ máy xuất (unix ms) — để báo lệch đồng hồ giữa 2 máy.
    tao_luc: u64,
    pdf: Vec<PdfGoi>,
    /// Định nghĩa các loại tài liệu có kết quả trong gói (loại riêng của máy xuất).
    #[serde(default)]
    loai_tai_lieu: Vec<LoaiTaiLieu>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PdfGoi {
    khoa: String,
    kich_thuoc: u64,
    /// Mã kiểm tra nội dung PDF (64 KB đầu + 64 KB cuối + kích thước, như cache phân tích): máy nhận chỉ nhận dữ liệu
    /// cho PDF cùng mã — trùng tên + trùng kích thước mà khác bản scan vẫn bị bỏ qua. Gói từ v2.3.0 không có → chỉ so kích thước.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ma: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TomTatXuat {
    pub so_ho_so: usize,
    pub so_ban_ghi_boc: usize,
    pub dung_luong: u64,
}

/// Nội dung gói xem trước khi nhập.
#[derive(Debug, Clone, PartialEq)]
pub struct XemTruoc {
    pub ten_thu_muc: String,
    pub so_pdf_goi: usize,
    /// PDF trong gói có trên máy này (cùng nội dung — ở đường dẫn nào cũng được).
    pub so_pdf_khop: usize,
    pub so_ho_so: usize,
    /// Loại tài liệu trong gói mà máy này chưa có: (tên, đoạn đầu hướng dẫn).
    pub loai_moi: Vec<(String, String)>,
    /// Giờ máy xuất − giờ máy này lúc đóng gói / xem (ms, có dấu).
    pub lech_dong_ho_ms: i64,
}

impl XemTruoc {
    pub fn ty_le_khop(&self) -> f32 {
        if self.so_pdf_goi == 0 { 1.0 } else { self.so_pdf_khop as f32 / self.so_pdf_goi as f32 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trung {
    pub khoa: String,
    pub moc_goi: u64,
    pub moc_may: u64,
    pub lay_goi: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaoCaoGop {
    /// Hồ sơ Tách lấy từ gói.
    pub nhan: usize,
    /// Hồ sơ Tách giữ bản máy này (máy này mới hơn hoặc bằng mốc).
    pub giu: usize,
    /// Hồ sơ hai bên sửa khác nhau (đã chọn theo mốc).
    pub trung: Vec<Trung>,
    /// PDF trong gói không có trên máy này (theo nội dung) / khoá không hợp lệ.
    pub bo_qua: Vec<String>,
    /// Bản ghi Bóc tách lấy từ gói.
    pub boc_nhan: usize,
    pub boc_giu: usize,
    pub loai_moi: Vec<String>,
    /// Loại trong gói trùng tên loại máy này nhưng khác nội dung → giữ bản máy này.
    pub loai_trung: Vec<String>,
    /// Loại trong gói không hợp lệ → bỏ.
    pub loai_loi: Vec<String>,
    /// Số mốc trong gói vượt giờ máy này + 1 ngày (đồng hồ máy gửi sai) → đã kẹp.
    pub lech_dong_ho: usize,
    /// Bản ghi Xóa ghim (vùng, nhật ký) lấy từ gói / giữ bản máy này.
    pub xoa_ghim_nhan: usize,
    pub xoa_ghim_giu: usize,
}

// ---------- Khoá liên tiến trình ----------

/// Khoá 1 thư mục việc khi Xuất / Nhập (2 cửa sổ / tiến trình không cùng ghi). Khoá quá 10 phút coi như bỏ dở.
struct KhoaPhien(PathBuf);

impl KhoaPhien {
    fn lay(an: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(an).map_err(|e| format!("không tạo được thư mục dữ liệu: {e}"))?;
        let f = an.join(".phien.lock");
        for _ in 0..2 {
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&f) {
                Ok(_) => return Ok(KhoaPhien(f)),
                Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => return Err(format!("không tạo được khoá thư mục dữ liệu: {e}")),
                Err(_) => {
                    let cu = std::fs::metadata(&f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
                    if cu.is_some_and(|d| d.as_secs() > 600) {
                        let _ = std::fs::remove_file(&f);
                        continue;
                    }
                    return Err("Thư mục này đang được xuất / nhập ở cửa sổ khác — đợi xong rồi thử lại".into());
                }
            }
        }
        Err("không lấy được khoá thư mục dữ liệu".into())
    }
}

impl Drop for KhoaPhien {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ---------- Chung ----------

/// PDF có trên máy: khoá (tương đối, `/`, NFC) → (đường dẫn thật, kích thước).
fn pdf_tren_may(goc: &Path) -> BTreeMap<String, (PathBuf, u64)> {
    let mut ds = BTreeMap::new();
    crate::tach_vb::phien::duyet_pdf_theo_cay(goc, &std::sync::atomic::AtomicBool::new(false), |thu_muc| {
        for p in thu_muc {
            if let Ok(md) = std::fs::metadata(&p) {
                ds.insert(nhan::duong_dan_ghi(goc, &p), (p, md.len()));
            }
        }
    });
    ds
}


/// Đọc file của máy này: không có → rỗng; byte hỏng chỉ mất ký tự đó; lỗi đọc khác → `Err` (không coi là rỗng rồi ghi đè).
fn doc_file_may(f: &Path) -> Result<String, String> {
    match std::fs::read(f) {
        Ok(b) => Ok(String::from_utf8_lossy(&b).into_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("không đọc được {}: {e}", f.display())),
    }
}

fn ten_thu_muc(goc: &Path) -> String {
    goc.file_name().map_or_else(|| "thu_muc".into(), |t| t.to_string_lossy().into_owned())
}

/// Tên mục cache trong gói cho khoá `k` (luôn `/`, kể cả trên Windows).
fn ten_muc_cache(k: &str) -> String {
    let f = cache_trang::file_cache(Path::new(""), k);
    format!("{THU_MUC_CACHE}/{}", f.file_name().unwrap_or_default().to_string_lossy())
}

/// Khoá hồ sơ từ gói hợp lệ: tương đối, `/`, không đoạn rỗng / `.` / `..`, không ổ đĩa (`C:`), không ký tự 0.
/// (Khoá còn phải trùng 1 PDF có trên máy nhận mới được dùng.)
fn khoa_hop_le(k: &str) -> bool {
    let o_dia = k.as_bytes().get(1) == Some(&b':');
    !k.is_empty() && !k.starts_with('/') && !o_dia && !k.contains('\0') && k.split('/').all(|d| !d.is_empty() && d != "." && d != "..")
}

// ---------- Xuất ----------

/// Đóng gói phần việc của các PDF trong thư mục `goc` vào `file_zip`. Nơi gọi đã ghi hết trạng thái trong bộ nhớ.
pub fn xuat(goc: &Path, file_zip: &Path) -> Result<TomTatXuat, String> {
    xuat_trong(goc, file_zip, &loai_tai_lieu::file_mac_dinh(), kho::chung(), crate::xoa_ghim::kho::chung())
}

fn ghi_ban(b: &mut Vec<u8>, id: Id, nd: &[u8]) {
    b.extend(id.0.to_le_bytes());
    b.extend(id.1.to_le_bytes());
    b.extend((nd.len() as u32).to_le_bytes());
    b.extend(nd);
}

/// Đọc `[mã 8][kích thước 8][độ dài 4][nội dung]…`; hỏng → dừng ở chỗ hỏng.
fn doc_ban(b: &[u8]) -> Vec<(Id, &[u8])> {
    let mut ds = Vec::new();
    let mut i = 0;
    while i + 20 <= b.len() {
        let id = (u64::from_le_bytes(b[i..i + 8].try_into().unwrap()), u64::from_le_bytes(b[i + 8..i + 16].try_into().unwrap()));
        let n = u32::from_le_bytes(b[i + 16..i + 20].try_into().unwrap()) as usize;
        if i + 20 + n > b.len() {
            break;
        }
        ds.push((id, &b[i + 20..i + 20 + n]));
        i += 20 + n;
    }
    ds
}

fn xuat_trong(goc: &Path, file_zip: &Path, file_loai: &Path, kho: &std::sync::Mutex<Kho>, kho_xg: &std::sync::Mutex<Kho>) -> Result<TomTatXuat, String> {
    let an = thu_muc_an(goc);
    let _khoa = KhoaPhien::lay(&an)?;
    // Id **thật** (đọc nội dung) của mọi PDF — file bị thay mà giữ thời điểm sửa không mang nhãn cũ đi
    let pdf: Vec<(String, (PathBuf, u64))> = pdf_tren_may(goc).into_iter().collect();
    let ma: Vec<Option<u64>> = {
        use rayon::prelude::*;
        pdf.par_iter().map(|(_, (p, n))| cache_trang::van_tay_file(p, *n).ok()).collect()
    };
    let (mut tach, mut phan_tich, mut so_ho_so) = (Vec::new(), Vec::new(), 0);
    let mut theo_khoa: HashMap<String, Id> = HashMap::new();
    {
        let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
        let mut da: HashSet<Id> = HashSet::new();
        for ((khoa, (_, n)), m) in pdf.iter().zip(&ma) {
            let Some(m) = m else { continue };
            let id = (*m, *n);
            theo_khoa.insert(khoa.clone(), id);
            if !da.insert(id) {
                continue;
            }
            if let Some(bt) = k.tach(id) {
                so_ho_so += (!bt.bia_mo && bt.chu_ky != moc_sua::CHU_KY_RONG) as usize;
                ghi_ban(&mut tach, id, &bt.ma_hoa());
            }
            if let Some(trang) = k.phan_tich(id) {
                let kf = cache_trang::KhoaFile { duong_dan: String::new(), kich_thuoc: id.1, sua_luc: 0, van_tay: id.0 };
                ghi_ban(&mut phan_tich, id, &cache_trang::ma_hoa(&kf, &trang));
            }
        }
    }
    let xoa_ghim = {
        let ids: HashSet<Id> = theo_khoa.values().copied().collect();
        xoa_ghim::dong_goi(&mut kho_xg.lock().unwrap_or_else(|e| e.into_inner()), goc, &ids)
    };
    // Kết quả bóc tách: bản ghi chưa có id (bản cũ) gắn id theo PDF hiện tại
    let jsonl = std::fs::read_to_string(an.join(F_JSONL)).unwrap_or_default();
    let mut ban_ghi: Vec<BanGhi> = luu::doc_tat_ca(&jsonl).into_values().collect();
    ban_ghi.sort_by(|a, b| (&a.file, &a.loai).cmp(&(&b.file, &b.loai)));
    let mut jsonl_goi = String::new();
    for b in &mut ban_ghi {
        b.id = b.id.or_else(|| theo_khoa.get(&b.file).copied());
        jsonl_goi += &serde_json::to_string(b).map_err(|e| e.to_string())?;
        jsonl_goi.push('\n');
    }
    let ten_loai: BTreeSet<&str> = ban_ghi.iter().map(|b| b.loai.as_str()).collect();
    let manifest = Manifest {
        phien_ban_goi: PHIEN_BAN_GOI,
        phien_ban_app: env!("CARGO_PKG_VERSION").into(),
        ten_thu_muc: ten_thu_muc(goc),
        tao_luc: moc_sua::bay_gio_ms(),
        pdf: pdf.iter().zip(&ma).map(|((khoa, (_, n)), m)| PdfGoi { khoa: khoa.clone(), kich_thuoc: *n, ma: *m }).collect(),
        loai_tai_lieu: loai_tai_lieu::doc(file_loai).into_iter().filter(|l| ten_loai.contains(l.ten.as_str())).collect(),
    };

    let mut ten_tam = file_zip.as_os_str().to_owned();
    ten_tam.push(".tmp");
    let tam = PathBuf::from(ten_tam);
    let ghi = || -> Result<(), String> {
        let loi = |e: &dyn std::fmt::Display| format!("không ghi được gói: {e}");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&tam).map_err(|e| loi(&e))?);
        let tuy_chon = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
        let mut them = |ten: &str, du_lieu: &[u8]| -> Result<(), String> {
            z.start_file(ten, tuy_chon).map_err(|e| loi(&e))?;
            z.write_all(du_lieu).map_err(|e| loi(&e))
        };
        them(F_MANIFEST, &serde_json::to_vec(&manifest).map_err(|e| loi(&e))?)?;
        them(F_TACH, &tach)?;
        them(F_PHAN_TICH, &phan_tich)?;
        them(F_JSONL, jsonl_goi.as_bytes())?;
        if !xoa_ghim.is_empty() {
            them(xoa_ghim::F_XOA_GHIM, &xoa_ghim)?;
        }
        z.finish().map_err(|e| loi(&e))?;
        Ok(())
    };
    if let Err(e) = ghi() {
        let _ = std::fs::remove_file(&tam);
        return Err(e);
    }
    std::fs::rename(&tam, file_zip).map_err(|e| {
        let _ = std::fs::remove_file(&tam);
        format!("không ghi được gói: {e}")
    })?;
    let dung_luong = std::fs::metadata(file_zip).map_or(0, |m| m.len());
    Ok(TomTatXuat { so_ho_so, so_ban_ghi_boc: ban_ghi.len(), dung_luong })
}

// ---------- Đọc gói ----------

struct Goi {
    manifest: Manifest,
    z: zip::ZipArchive<std::fs::File>,
}

impl Goi {
    fn mo(file_zip: &Path) -> Result<Goi, String> {
        let f = std::fs::File::open(file_zip).map_err(|e| format!("không mở được gói: {e}"))?;
        let mut z = zip::ZipArchive::new(f).map_err(|_| "file không phải gói phiên hợp lệ (zip hỏng)".to_string())?;
        if z.len() > TOI_DA_MUC {
            return Err("gói có quá nhiều mục".into());
        }
        // Thư viện zip giữ 1 mục / tên (mục sau đè mục trước) → phải hỏi riêng có tên trùng hay không
        if z.has_duplicate_names() {
            return Err("gói có mục trùng tên — không nhập".into());
        }
        if z.file_names().any(|n| n.is_err()) {
            return Err("gói có tên mục không đọc được — không nhập".into());
        }
        let manifest = doc_muc(&mut z, F_MANIFEST, TOI_DA_MANIFEST)?.ok_or("gói thiếu manifest.json")?;
        let manifest: Manifest = serde_json::from_slice(&manifest).map_err(|_| "manifest.json của gói hỏng".to_string())?;
        if manifest.phien_ban_goi > PHIEN_BAN_GOI {
            return Err("Gói được tạo từ bản ứng dụng mới hơn — cập nhật ứng dụng rồi nhập lại".into());
        }
        Ok(Goi { manifest, z })
    }

    fn doc(&mut self, ten: &str, toi_da: u64) -> Result<Option<Vec<u8>>, String> {
        doc_muc(&mut self.z, ten, toi_da)
    }

    fn doc_chu(&mut self, ten: &str) -> Result<String, String> {
        Ok(self.doc(ten, TOI_DA_FILE)?.map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default())
    }
}

/// Đọc đúng 1 mục theo tên; đếm byte thật (không tin kích thước khai trong zip). Không có mục → `None`.
fn doc_muc(z: &mut zip::ZipArchive<std::fs::File>, ten: &str, toi_da: u64) -> Result<Option<Vec<u8>>, String> {
    let muc = match z.by_name(ten) {
        Ok(m) => m,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(format!("không đọc được {ten} trong gói: {e}")),
    };
    let mut b = Vec::new();
    muc.take(toi_da + 1).read_to_end(&mut b).map_err(|e| format!("không đọc được {ten} trong gói: {e}"))?;
    if b.len() as u64 > toi_da {
        return Err(format!("{ten} trong gói quá lớn"));
    }
    Ok(Some(b))
}

// ---------- Nội dung gói theo id ----------

/// Nội dung gói đã đổi sang id nội dung PDF.
struct NoiDungGoi {
    /// PDF của gói: id → đường dẫn tương đối bên máy xuất (để báo cáo).
    pdf: HashMap<Id, String>,
    /// PDF của gói không xác định được id (gói 2.3.0 không có mã, không có PDF cùng đường dẫn + kích thước ở đây).
    khong_id: Vec<String>,
    tach: Vec<(Id, BanTach)>,
    phan_tich: HashMap<Id, Vec<u8>>,
    ban_ghi: Vec<BanGhi>,
}

/// Đọc gói (bản 2 theo id; bản 1 / 2.3 theo đường dẫn → đổi sang id qua mã nội dung hoặc PDF cùng đường dẫn + kích
/// thước ở máy này). `may`: đường dẫn tương đối ở máy này → (đường dẫn, kích thước, id).
fn doc_noi_dung(goi: &mut Goi, may: &HashMap<String, (PathBuf, u64, Option<Id>)>) -> Result<NoiDungGoi, String> {
    let mut theo_khoa: HashMap<String, Id> = HashMap::new();
    let mut nd = NoiDungGoi { pdf: HashMap::new(), khong_id: Vec::new(), tach: Vec::new(), phan_tich: HashMap::new(), ban_ghi: Vec::new() };
    for p in &goi.manifest.pdf {
        let k = nhan::chuan_duong_dan(&p.khoa);
        let id = p.ma.map(|m| (m, p.kich_thuoc)).or_else(|| may.get(&k).filter(|(_, n, _)| *n == p.kich_thuoc).and_then(|(_, _, id)| *id));
        match id {
            Some(id) => {
                theo_khoa.insert(k.clone(), id);
                nd.pdf.entry(id).or_insert(k);
            }
            None => nd.khong_id.push(k),
        }
    }
    let id_cua = |k: &str| theo_khoa.get(&nhan::chuan_duong_dan(k)).copied();
    if goi.manifest.phien_ban_goi >= 2 {
        let b = goi.doc(F_TACH, TOI_DA_KHO)?.unwrap_or_default();
        nd.tach = doc_ban(&b).into_iter().filter_map(|(id, x)| Some((id, BanTach::giai_ma(x)?))).collect();
        let b = goi.doc(F_PHAN_TICH, TOI_DA_KHO)?.unwrap_or_default();
        nd.phan_tich = doc_ban(&b).into_iter().filter(|(_, x)| x.len() as u64 <= TOI_DA_BIN).map(|(id, x)| (id, x.to_vec())).collect();
    } else {
        // Gói 2.3: file nhãn / xoay / mốc / cache theo đường dẫn → bản ghi theo id
        let moc = moc_sua::doc_chuoi(&goi.doc_chu(moc_sua::TEN_FILE)?);
        let nhan_goi = nhan::doc_file_nhan(&goi.doc_chu(F_NHAN)?, &|_| None).map_err(|_| "file nhãn trong gói hỏng".to_string())?.0;
        let xoay_goi = nhan::doc_file_xoay(&goi.doc_chu(F_XOAY)?, &|_| None).map_err(|_| "file góc xoay trong gói hỏng".to_string())?;
        let khoa: BTreeSet<&String> = moc.keys().chain(nhan_goi.keys()).chain(xoay_goi.keys()).collect();
        for raw in khoa {
            let Some(id) = id_cua(raw).filter(|_| khoa_hop_le(&nhan::chuan_duong_dan(raw))) else { continue };
            let trang = goi.doc(&ten_muc_cache(&nhan::chuan_duong_dan(raw)), TOI_DA_BIN).ok().flatten().and_then(|b| cache_trang::giai_ma(&b)).map(|(_, t)| t);
            let duyet: BTreeMap<usize, u8> = trang.iter().flatten().enumerate().filter(|(_, t)| t.trang_thai != 0).map(|(i, t)| (i + 1, t.trang_thai)).collect();
            if let Some(t) = &trang {
                let kf = cache_trang::KhoaFile { duong_dan: String::new(), kich_thuoc: id.1, sua_luc: 0, van_tay: id.0 };
                nd.phan_tich.insert(id, cache_trang::ma_hoa(&kf, t));
            }
            // Gói không có mốc cho hồ sơ (nhãn từ trước khi có mốc) → không ghi đè máy này
            let Some(m) = moc.get(raw) else { continue };
            let n = nhan_goi.get(raw).cloned().unwrap_or_default();
            let x = xoay_goi.get(raw).cloned().unwrap_or_default();
            let chu_ky = if m.chu_ky == moc_sua::CHU_KY_RONG { moc_sua::CHU_KY_RONG } else { kho::chu_ky_tach(&n, &x, &duyet) };
            let so_trang = trang.as_ref().map_or(0, |t| t.len() as u32);
            nd.tach.push((id, BanTach { so_trang, nhan: n, xoay: x, duyet, luc: m.luc, chu_ky, bia_mo: false }));
        }
    }
    nd.ban_ghi = luu::doc_tat_ca(&goi.doc_chu(F_JSONL)?)
        .into_values()
        .filter_map(|mut b| {
            b.id = b.id.or_else(|| id_cua(&b.file));
            b.id.map(|_| b)
        })
        .collect();
    Ok(nd)
}

/// PDF ở máy này: đường dẫn tương đối → (đường dẫn, kích thước, id).
fn pdf_may_id(goc: &Path, kho: &std::sync::Mutex<Kho>) -> HashMap<String, (PathBuf, u64, Option<Id>)> {
    let ds: Vec<(String, (PathBuf, u64))> = pdf_tren_may(goc).into_iter().collect();
    let ids = kho::ids_cua_trong(kho, &ds.iter().map(|(_, (p, _))| p.clone()).collect::<Vec<_>>());
    ds.into_iter().zip(ids).map(|((k, (p, n)), id)| (k, (p, n, id))).collect()
}

// ---------- Xem trước ----------

pub fn xem_truoc(goc: &Path, file_zip: &Path) -> Result<XemTruoc, String> {
    xem_truoc_trong(goc, file_zip, &loai_tai_lieu::file_mac_dinh(), kho::chung())
}

fn xem_truoc_trong(goc: &Path, file_zip: &Path, file_loai: &Path, kho: &std::sync::Mutex<Kho>) -> Result<XemTruoc, String> {
    let mut goi = Goi::mo(file_zip)?;
    let may = pdf_may_id(goc, kho);
    let nd = doc_noi_dung(&mut goi, &may)?;
    let co: HashSet<Id> = may.values().filter_map(|(_, _, id)| *id).collect();
    let so_pdf_khop = nd.pdf.keys().filter(|id| co.contains(id)).count();
    let so_ho_so = nd.tach.iter().filter(|(id, b)| co.contains(id) && !b.bia_mo && b.chu_ky != moc_sua::CHU_KY_RONG).count();
    let co_san: BTreeSet<String> = loai_tai_lieu::doc(file_loai).into_iter().map(|l| l.ten).collect();
    let loai_moi = goi
        .manifest
        .loai_tai_lieu
        .iter()
        .filter(|l| !co_san.contains(&l.ten) && loai_tai_lieu::kiem(l).is_none())
        .map(|l| (l.ten.clone(), l.huong_dan.chars().take(160).collect()))
        .collect();
    Ok(XemTruoc {
        ten_thu_muc: goi.manifest.ten_thu_muc.clone(),
        so_pdf_goi: nd.pdf.len() + nd.khong_id.len(),
        so_pdf_khop,
        so_ho_so,
        loai_moi,
        lech_dong_ho_ms: (goi.manifest.tao_luc as i128 - moc_sua::bay_gio_ms() as i128).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
    })
}

// ---------- Nhập ----------

/// Nhập gói vào thư mục `goc` đang mở, gộp theo nội dung PDF. Nơi gọi đã ghi hết trạng thái trong bộ nhớ xuống đĩa và
/// sẽ nạp lại sau khi nhập (không ghi bản cũ trong bộ nhớ đè lên).
pub fn nhap(goc: &Path, file_zip: &Path) -> Result<BaoCaoGop, String> {
    nhap_trong(goc, file_zip, &loai_tai_lieu::file_mac_dinh(), kho::chung(), crate::xoa_ghim::kho::chung())
}

fn nhap_trong(goc: &Path, file_zip: &Path, file_loai: &Path, kho: &std::sync::Mutex<Kho>, kho_xg: &std::sync::Mutex<Kho>) -> Result<BaoCaoGop, String> {
    let an = thu_muc_an(goc);
    let _khoa = KhoaPhien::lay(&an)?;
    let mut goi = Goi::mo(file_zip)?;
    let may = pdf_may_id(goc, kho);
    // id → PDF ở máy này (đường dẫn tương đối, đường dẫn, kích thước)
    let mut may_id: HashMap<Id, (String, PathBuf)> = HashMap::new();
    for (k, (p, _, id)) in &may {
        if let Some(id) = id {
            may_id.entry(*id).or_insert((k.clone(), p.clone()));
        }
    }
    let nd = doc_noi_dung(&mut goi, &may)?;
    let bay_gio = moc_sua::bay_gio_ms();
    let mut bc = BaoCaoGop::default();
    let mut bo_qua: BTreeSet<String> = nd.khong_id.iter().cloned().collect();
    bo_qua.extend(nd.pdf.iter().filter(|(id, _)| !may_id.contains_key(id)).map(|(_, k)| k.clone()));

    // --- Tách: theo id, mốc sau cùng thắng ---
    {
        let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
        // Kết quả phân tích (theo nội dung PDF, 2 máy như nhau): máy này chưa có thì lấy của gói — trước phần Tách để
        // số trang thật lấy từ đây (khỏi mở từng PDF)
        for (id, b) in &nd.phan_tich {
            if may_id.contains_key(id) && !k.co_phan_tich(*id) {
                if let Some((kf, trang)) = cache_trang::giai_ma(b) {
                    if (kf.van_tay, kf.kich_thuoc) == *id {
                        k.ghi_phan_tich(*id, &trang);
                    }
                }
            }
        }
        let mut da: HashSet<Id> = HashSet::new();
        for (id, mut bt) in nd.tach {
            let Some((khoa, pdf)) = may_id.get(&id) else { continue };
            if !da.insert(id) {
                continue;
            }
            let (luc, lech) = gop::kep(bt.luc, bay_gio);
            bc.lech_dong_ho += lech as usize;
            bt.luc = luc;
            let cu = k.tach(id);
            let m = cu.as_ref().map(|c| Moc { luc: c.luc, chu_ky: c.chu_ky });
            let g = Moc { luc: bt.luc, chu_ky: bt.chu_ky };
            let ket_qua = gop::ho_so(Some(g), m);
            if let (Some(m), false) = (m, ket_qua == gop::KetQua::Giong) {
                bc.trung.push(Trung { khoa: khoa.clone(), moc_goi: g.luc, moc_may: m.luc, lay_goi: ket_qua == gop::KetQua::LayGoi });
            }
            match ket_qua {
                gop::KetQua::LayGoi => {
                    // Chỉ nhận trang có thật trong PDF máy này (gói lạ / hỏng không làm bảng nhãn mở không được)
                    let n = k.phan_tich(id).map(|t| t.len()).or_else(|| crate::tach_vb::dung_luong::doc_thong_tin_file(pdf).ok().map(|t| t.len())).unwrap_or(0);
                    bt.nhan.retain(|so, _| *so <= n);
                    bt.xoay.retain(|so, _| *so <= n);
                    bt.duyet.retain(|so, _| *so <= n);
                    if bt.so_trang as usize != n {
                        bt.so_trang = n as u32;
                    }
                    k.them_tach(id, &bt);
                    bc.nhan += 1;
                }
                gop::KetQua::GiuMay => bc.giu += 1,
                gop::KetQua::Giong => {}
            }
        }
        k.day().map_err(|e| format!("không ghi được vào kho: {e}"))?;
    }

    // --- Xóa ghim: mục tuỳ chọn; khoá kho Xóa ghim sau khi đã nhả kho Tách ---
    if let Some(b) = goi.doc(xoa_ghim::F_XOA_GHIM, TOI_DA_KHO)? {
        let co: HashSet<Id> = may_id.keys().copied().collect();
        let mut k = kho_xg.lock().unwrap_or_else(|e| e.into_inner());
        let kq = xoa_ghim::nhap(&mut k, goc, &b, &co, bay_gio);
        bc.xoa_ghim_nhan = kq.nhan;
        bc.xoa_ghim_giu = kq.giu;
        bc.lech_dong_ho += kq.lech_dong_ho;
        if kq.nhan > 0 {
            k.day().map_err(|e| format!("không ghi được vào kho Xóa ghim: {e}"))?;
        }
    }

    // --- Bóc tách: theo (file, loại) của máy này; bản lỗi không bao giờ thắng bản đọc được ---
    let f_jsonl = an.join(F_JSONL);
    let mut may_bg = luu::doc_tat_ca(&doc_file_may(&f_jsonl)?);
    let mut goi_bg = nd.ban_ghi;
    goi_bg.sort_by(|a, b| (&a.file, &a.loai).cmp(&(&b.file, &b.loai)));
    let mut moi: Vec<BanGhi> = Vec::new();
    for mut b in goi_bg {
        let Some((khoa, _)) = b.id.and_then(|id| may_id.get(&id)) else {
            bo_qua.insert(b.file.clone());
            continue;
        };
        b.file = khoa.clone();
        let (luc, lech) = gop::kep(b.luc, bay_gio);
        bc.lech_dong_ho += lech as usize;
        b.luc = luc;
        let k = (b.file.clone(), b.loai.clone());
        match gop::ban_ghi(&b, may_bg.get(&k)) {
            gop::KetQua::LayGoi => {
                bc.boc_nhan += 1;
                may_bg.insert(k, b.clone());
                moi.push(b);
            }
            gop::KetQua::GiuMay => bc.boc_giu += 1,
            gop::KetQua::Giong => {}
        }
    }
    luu::ghi_them_nhieu(&f_jsonl, &moi).map_err(|e| format!("không ghi được kết quả bóc tách: {e}"))?;
    if !moi.is_empty() {
        crate::tach_vb::du_lieu_an::ghi_goc(goc);
        let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
        for b in &moi {
            if let Some(id) = b.id {
                k.ghi_boc(id, &f_jsonl);
            }
        }
        let _ = k.day();
    }

    // --- Loại tài liệu: chỉ thêm loại chưa có tên, đã kiểm hợp lệ; trùng tên khác nội dung → giữ máy này ---
    let mut ds_loai = loai_tai_lieu::doc(file_loai);
    let mut them_loai = false;
    let theo_ten: HashMap<String, LoaiTaiLieu> = ds_loai.iter().map(|l| (l.ten.clone(), l.clone())).collect();
    for l in &goi.manifest.loai_tai_lieu {
        if loai_tai_lieu::kiem(l).is_some() {
            bc.loai_loi.push(l.ten.clone());
        } else if let Some(cu) = theo_ten.get(&l.ten) {
            if cu != l {
                bc.loai_trung.push(l.ten.clone());
            }
        } else {
            ds_loai.push(l.clone());
            bc.loai_moi.push(l.ten.clone());
            them_loai = true;
        }
    }
    if them_loai {
        loai_tai_lieu::ghi(file_loai, &ds_loai).map_err(|e| format!("không ghi được loại tài liệu: {e}"))?;
    }
    bc.bo_qua = bo_qua.into_iter().collect();
    Ok(bc)
}

#[cfg(test)]
mod tests;
