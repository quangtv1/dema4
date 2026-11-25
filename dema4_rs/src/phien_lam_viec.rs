//! Gói phiên làm việc: mang phần đang làm dở (nhãn, góc xoay, cache phân tích, mốc sửa, kết quả bóc tách) của 1 thư mục
//! đầu vào sang máy khác để làm tiếp.
//!
//! - **Xuất**: đóng gói đúng danh sách trắng các file trong thư mục dữ liệu làm dở thành 1 file zip + `manifest.json`.
//!   Không bao giờ có khoá API, license, cấu hình, mã máy (chúng nằm ngoài `viec/`).
//! - **Nhập**: vào thư mục đang mở trên máy nhận, gộp theo hồ sơ (xem `gop`). Không tin gì trong zip: chỉ đọc đúng tên
//!   cần, có giới hạn dung lượng, mọi khoá hồ sơ phải trùng 1 PDF có trên máy nhận, tên file cache dựng lại từ khoá.
//! - Mỗi file đích ghi an toàn (file tạm + đổi tên); nhập lại cùng gói cho cùng kết quả.

mod gop;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::boc_tach::loai_tai_lieu::{self, LoaiTaiLieu};
use crate::boc_tach::luu::{self, BanGhi};
use crate::ghi_file::ghi_an_toan;
use crate::tach_vb::cache_trang;
use crate::tach_vb::du_lieu_an::thu_muc_an;
use crate::tach_vb::moc_sua::{self, Moc};
use crate::tach_vb::nhan::{self, BangNhan, BangXoay};

/// Gói từ bản app mới hơn (phiên bản gói lớn hơn) → từ chối, không đoán.
pub const PHIEN_BAN_GOI: u32 = 1;
/// Đuôi tên gói: `<tên thư mục>.sm-dema4.zip`.
pub const DUOI_GOI: &str = ".sm-dema4.zip";

const F_MANIFEST: &str = "manifest.json";
const F_NHAN: &str = "index_and_blank.txt";
const F_XOAY: &str = "xoay_trang.txt";
const F_JSONL: &str = "boc_tach.jsonl";
const THU_MUC_CACHE: &str = "cache";

const TOI_DA_MANIFEST: u64 = 64 << 20;
const TOI_DA_FILE: u64 = 200 << 20;
const TOI_DA_BIN: u64 = 1 << 20;
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
    /// PDF trong gói có trên máy này (cùng khoá + kích thước).
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
    /// Khoá trong gói không có PDF tương ứng trên máy này / không hợp lệ.
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

/// Khoá (chuẩn) các PDF của gói có bản **giống hệt** trên máy này: cùng đường dẫn tương đối, cùng kích thước và — khi gói
/// có mã kiểm tra — cùng mã nội dung. PDF khác bản thì nhãn / trang / kết quả không còn đúng → nơi gọi bỏ qua.
fn pdf_khop(goi: &[PdfGoi], may: &BTreeMap<String, (PathBuf, u64)>) -> HashSet<String> {
    use rayon::prelude::*;
    goi.par_iter()
        .filter_map(|p| {
            let k = nhan::chuan_duong_dan(&p.khoa);
            let (duong, n) = may.get(&k)?;
            let cung_ma = p.ma.is_none_or(|ma| cache_trang::van_tay_file(duong, *n).ok() == Some(ma));
            (*n == p.kich_thuoc && cung_ma).then_some(k)
        })
        .collect()
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

/// Đóng gói phần việc của thư mục `goc` vào `file_zip`. Nơi gọi đã ghi hết trạng thái trong bộ nhớ xuống đĩa.
pub fn xuat(goc: &Path, file_zip: &Path) -> Result<TomTatXuat, String> {
    xuat_voi(goc, file_zip, &loai_tai_lieu::file_mac_dinh())
}

fn xuat_voi(goc: &Path, file_zip: &Path, file_loai: &Path) -> Result<TomTatXuat, String> {
    let an = thu_muc_an(goc);
    let _khoa = KhoaPhien::lay(&an)?;
    let jsonl = std::fs::read_to_string(an.join(F_JSONL)).unwrap_or_default();
    let ban_ghi = luu::doc_tat_ca(&jsonl);
    let ten_loai: BTreeSet<&str> = ban_ghi.keys().map(|(_, l)| l.as_str()).collect();
    let manifest = Manifest {
        phien_ban_goi: PHIEN_BAN_GOI,
        phien_ban_app: env!("CARGO_PKG_VERSION").into(),
        ten_thu_muc: ten_thu_muc(goc),
        tao_luc: moc_sua::bay_gio_ms(),
        pdf: {
            use rayon::prelude::*;
            let ds: Vec<_> = pdf_tren_may(goc).into_iter().collect();
            ds.into_par_iter().map(|(khoa, (p, kich_thuoc))| PdfGoi { ma: cache_trang::van_tay_file(&p, kich_thuoc).ok(), khoa, kich_thuoc }).collect()
        },
        loai_tai_lieu: loai_tai_lieu::doc(file_loai).into_iter().filter(|l| ten_loai.contains(l.ten.as_str())).collect(),
    };
    let so_ho_so = moc_sua::doc(&an.join(moc_sua::TEN_FILE)).values().filter(|m| m.chu_ky != moc_sua::CHU_KY_RONG).count();

    let mut ten_tam = file_zip.as_os_str().to_owned();
    ten_tam.push(".tmp");
    let tam = PathBuf::from(ten_tam);
    let ghi = || -> Result<(), String> {
        let loi = |e: &dyn std::fmt::Display| format!("không ghi được gói: {e}");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&tam).map_err(|e| loi(&e))?);
        let tuy_chon = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut them = |ten: &str, du_lieu: &[u8]| -> Result<(), String> {
            z.start_file(ten, tuy_chon).map_err(|e| loi(&e))?;
            z.write_all(du_lieu).map_err(|e| loi(&e))
        };
        them(F_MANIFEST, &serde_json::to_vec(&manifest).map_err(|e| loi(&e))?)?;
        // Thiếu file = chưa có phần đó; đọc lỗi thì dừng (gói thiếu mốc sẽ làm máy nhận gộp sai)
        let doc = |f: &Path| match std::fs::read(f) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("không đọc được {}: {e}", f.display())),
        };
        for ten in [F_NHAN, F_XOAY, moc_sua::TEN_FILE, F_JSONL] {
            if let Some(b) = doc(&an.join(ten))? {
                them(ten, &b)?;
            }
        }
        if let Ok(ds) = std::fs::read_dir(an.join(THU_MUC_CACHE)) {
            for muc in ds.flatten() {
                let ten = muc.file_name().to_string_lossy().into_owned();
                if ten.ends_with(".bin") {
                    if let Some(b) = doc(&muc.path())? {
                        them(&format!("{THU_MUC_CACHE}/{ten}"), &b)?;
                    }
                }
            }
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

// ---------- Xem trước ----------

pub fn xem_truoc(goc: &Path, file_zip: &Path) -> Result<XemTruoc, String> {
    xem_truoc_voi(goc, file_zip, &loai_tai_lieu::file_mac_dinh())
}

fn xem_truoc_voi(goc: &Path, file_zip: &Path, file_loai: &Path) -> Result<XemTruoc, String> {
    let mut goi = Goi::mo(file_zip)?;
    let may = pdf_tren_may(goc);
    let so_pdf_khop = pdf_khop(&goi.manifest.pdf, &may).len();
    let so_ho_so = moc_sua::doc_chuoi(&goi.doc_chu(moc_sua::TEN_FILE)?).values().filter(|m| m.chu_ky != moc_sua::CHU_KY_RONG).count();
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
        so_pdf_goi: goi.manifest.pdf.len(),
        so_pdf_khop,
        so_ho_so,
        loai_moi,
        lech_dong_ho_ms: (goi.manifest.tao_luc as i128 - moc_sua::bay_gio_ms() as i128).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
    })
}

// ---------- Nhập ----------

/// Nhập gói vào thư mục `goc` đang mở, gộp theo hồ sơ. Nơi gọi đã ghi hết trạng thái trong bộ nhớ xuống đĩa và sẽ nạp
/// lại từ đĩa sau khi nhập (không ghi bản cũ trong bộ nhớ đè lên).
pub fn nhap(goc: &Path, file_zip: &Path) -> Result<BaoCaoGop, String> {
    nhap_voi(goc, file_zip, &loai_tai_lieu::file_mac_dinh())
}

fn nhap_voi(goc: &Path, file_zip: &Path, file_loai: &Path) -> Result<BaoCaoGop, String> {
    let an = thu_muc_an(goc);
    let _khoa = KhoaPhien::lay(&an)?;
    let mut goi = Goi::mo(file_zip)?;
    let may_pdf = pdf_tren_may(goc);
    let bay_gio = moc_sua::bay_gio_ms();
    let mut bc = BaoCaoGop::default();
    // Khoá từ gói → khoá chuẩn nếu trùng 1 PDF **giống hệt** trên máy này (xem `pdf_khop`)
    let khop = pdf_khop(&goi.manifest.pdf, &may_pdf);
    let kiem_khoa = |k: &str| -> Option<String> {
        let c = nhan::chuan_duong_dan(k);
        (khoa_hop_le(&c) && khop.contains(&c)).then_some(c)
    };
    let mut bo_qua: BTreeSet<String> = BTreeSet::new();

    // --- Đọc gói (gói hỏng → dừng, máy này không đổi) ---
    let goi_moc = moc_sua::doc_chuoi(&goi.doc_chu(moc_sua::TEN_FILE)?);
    let goi_nhan = nhan::doc_file_nhan(&goi.doc_chu(F_NHAN)?, &|_| None).map_err(|_| "file nhãn trong gói hỏng".to_string())?.0;
    let goi_xoay = nhan::doc_file_xoay(&goi.doc_chu(F_XOAY)?, &|_| None).map_err(|_| "file góc xoay trong gói hỏng".to_string())?;
    let goi_jsonl = goi.doc_chu(F_JSONL)?;

    // --- Đọc máy này **chặt**: đọc / hiểu không được thì dừng, không coi là rỗng rồi ghi đè mất nhãn ---
    let f_moc = an.join(moc_sua::TEN_FILE);
    let mut may_moc = moc_sua::doc_kiem(&f_moc).map_err(|e| format!("không đọc được mốc sửa: {e}"))?;
    const LOI_NHAN_MAY: &str = "File nhãn / góc xoay của thư mục này đang lỗi — mở thư mục, Lưu nhãn lại rồi nhập";
    let mut may_nhan: BangNhan = nhan::doc_file_nhan(&doc_file_may(&an.join(F_NHAN))?, &|_| None).map_err(|_| LOI_NHAN_MAY.to_string())?.0;
    let mut may_xoay: BangXoay = nhan::doc_file_xoay(&doc_file_may(&an.join(F_XOAY))?, &|_| None).map_err(|_| LOI_NHAN_MAY.to_string())?;

    // --- Tách: quyết định theo hồ sơ ---
    /// Máy này có nhãn / xoay mà chưa có mốc (nhãn từ trước khi có mốc): coi như mốc 0, khác mọi chữ ký.
    const CHU_KY_CHUA_MOC: u64 = u64::MAX;
    let khoa_goi: BTreeSet<&String> = goi_moc.keys().chain(goi_nhan.keys()).chain(goi_xoay.keys()).collect();
    let mut lay: Vec<(String, Moc)> = Vec::new();
    for raw in khoa_goi {
        let Some(k) = kiem_khoa(raw) else {
            bo_qua.insert(raw.clone());
            continue;
        };
        let g = goi_moc.get(raw).map(|m| {
            let (luc, lech) = gop::kep(m.luc, bay_gio);
            bc.lech_dong_ho += lech as usize;
            Moc { luc, ..*m }
        });
        let m = may_moc.get(&k).copied().or_else(|| (may_nhan.contains_key(&k) || may_xoay.contains_key(&k)).then_some(Moc { luc: 0, chu_ky: CHU_KY_CHUA_MOC }));
        let ket_qua = gop::ho_so(g, m);
        if let (Some(g), Some(m), false) = (g, m, ket_qua == gop::KetQua::Giong) {
            bc.trung.push(Trung { khoa: k.clone(), moc_goi: g.luc, moc_may: m.luc, lay_goi: ket_qua == gop::KetQua::LayGoi });
        }
        match (ket_qua, g) {
            (gop::KetQua::LayGoi, Some(g)) => {
                bc.nhan += 1;
                lay.push((k, g));
            }
            (gop::KetQua::Giong, _) => {}
            _ => bc.giu += 1,
        }
    }

    // --- Chuẩn bị đủ dữ liệu trước khi ghi: cache hợp lệ của gói + số trang thật để lọc nhãn ---
    let mut cache_goi: HashMap<String, Vec<u8>> = HashMap::new();
    let mut so_trang: HashMap<String, usize> = HashMap::new();
    for (k, _) in &lay {
        // Cache hỏng / quá lớn / sai khoá → chỉ bỏ cache đó (hồ sơ phân tích lại), không dừng cả lần nhập
        let bin = goi.doc(&ten_muc_cache(k), TOI_DA_BIN).ok().flatten();
        let hop = bin.as_deref().and_then(cache_trang::giai_ma).filter(|(kf, _)| kf.duong_dan == *k && Some(kf.kich_thuoc) == may_pdf.get(k).map(|(_, n)| *n));
        let n = match &hop {
            Some((_, trang)) => trang.len(),
            None => crate::tach_vb::dung_luong::doc_thong_tin_file(&may_pdf[k].0).map(|t| t.len()).unwrap_or(0),
        };
        so_trang.insert(k.clone(), n);
        if let (Some(b), Some(_)) = (bin, hop) {
            cache_goi.insert(k.clone(), b);
        }
    }

    // --- Ghi: cache → nhãn → xoay → mốc (mốc sau cùng: hỏng giữa chừng thì lần nhập sau làm lại được) ---
    let thu_muc_cache = an.join(THU_MUC_CACHE);
    for (k, g) in &lay {
        let n = so_trang[k];
        // Chỉ nhận trang có thật trong PDF máy này (gói lạ / hỏng không làm file nhãn mở không được)
        match goi_nhan.get(k).map(|t| t.iter().filter(|(so, _)| **so <= n).map(|(so, v)| (*so, *v)).collect::<BTreeMap<_, _>>()) {
            Some(t) if !t.is_empty() => may_nhan.insert(k.clone(), t),
            _ => may_nhan.remove(k),
        };
        match goi_xoay.get(k).map(|t| t.iter().filter(|(so, _)| **so <= n).map(|(so, v)| (*so, *v)).collect::<BTreeMap<_, _>>()) {
            Some(t) if !t.is_empty() => may_xoay.insert(k.clone(), t),
            _ => may_xoay.remove(k),
        };
        may_moc.insert(k.clone(), *g);
        let f = cache_trang::file_cache(&thu_muc_cache, k);
        match cache_goi.get(k) {
            Some(b) => ghi_an_toan(&f, b).map_err(|e| format!("không ghi được cache: {e}"))?,
            // Gói thắng mà không có cache dùng được: bỏ cache cũ (cờ duyệt cũ không được trộn với nhãn mới)
            None => {
                let _ = std::fs::remove_file(&f);
            }
        }
    }
    if !lay.is_empty() {
        ghi_an_toan(&an.join(F_NHAN), nhan::ghi_file_nhan(&may_nhan)).map_err(|e| format!("không ghi được nhãn: {e}"))?;
        ghi_an_toan(&an.join(F_XOAY), nhan::ghi_file_xoay(&may_xoay)).map_err(|e| format!("không ghi được góc xoay: {e}"))?;
        moc_sua::ghi(&f_moc, &may_moc).map_err(|e| format!("không ghi được mốc sửa: {e}"))?;
    }

    // --- Bóc tách: theo (file, loại); chỉ ghi nối các bản ghi thắng (luật đọc sẵn có tự lấy bản sau) ---
    let f_jsonl = an.join(F_JSONL);
    let may_bg = luu::doc_tat_ca(&doc_file_may(&f_jsonl)?);
    let mut goi_bg: Vec<BanGhi> = luu::doc_tat_ca(&goi_jsonl).into_values().collect();
    goi_bg.sort_by(|a, b| (&a.file, &a.loai).cmp(&(&b.file, &b.loai)));
    for mut b in goi_bg {
        let Some(k) = kiem_khoa(&b.file) else {
            bo_qua.insert(b.file.clone());
            continue;
        };
        b.file = k;
        let (luc, lech) = gop::kep(b.luc, bay_gio);
        bc.lech_dong_ho += lech as usize;
        b.luc = luc;
        match gop::ban_ghi(&b, may_bg.get(&(b.file.clone(), b.loai.clone()))) {
            gop::KetQua::LayGoi => {
                luu::ghi_them(&f_jsonl, &b).map_err(|e| format!("không ghi được kết quả bóc tách: {e}"))?;
                bc.boc_nhan += 1;
            }
            gop::KetQua::GiuMay => bc.boc_giu += 1,
            gop::KetQua::Giong => {}
        }
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
