//! Lõi đếm trang: phân loại khổ giấy, cảnh báo sát ngưỡng / DPI, và định dạng kết quả
//! giống hệt chức năng "1. Đếm trang A4" của dema4_gom.py (cùng cột, cùng cách ghi số).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use mupdf::pdf::{PdfDocument, PdfObject};
use mupdf::{ColorParams, Colorspace, Device, Document, Image, Matrix, NativeDevice};
use rayon::prelude::*;

fn co_truong_da_ky(pdf: &PdfDocument) -> Result<bool, mupdf::Error> {
    let Some(acro) = pdf.catalog()?.get_dict("AcroForm")? else {
        return Ok(false);
    };
    let Some(fields) = acro.get_dict("Fields")? else {
        return Ok(false);
    };
    duyet_truong_ky(&fields, 0)
}

/// Duyệt mảng Fields (và Kids) tìm trường chữ ký đã ký; giới hạn độ sâu để tránh vòng lặp
fn duyet_truong_ky(mang: &PdfObject, do_sau: u32) -> Result<bool, mupdf::Error> {
    if do_sau > 8 {
        return Ok(false);
    }
    for i in 0..mang.len().unwrap_or(0) as i32 {
        let Some(f) = mang.get_array(i)? else { continue };
        let f = f.resolve()?.unwrap_or(f);
        let la_sig = f.get_dict("FT")?.and_then(|ft| ft.as_name().ok()).map(|n| n == b"Sig").unwrap_or(false);
        if la_sig {
            if let Some(v) = f.get_dict("V")? {
                if !v.is_null().unwrap_or(true) {
                    return Ok(true); // có /V = đã ký
                }
            }
        }
        if let Some(kids) = f.get_dict("Kids")? {
            if duyet_truong_ky(&kids, do_sau + 1)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub const KHO: [&str; 8] = ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7"];
/// Cạnh dài chuẩn (pt) của từng khổ, từ lớn đến nhỏ
pub const A_SIZES: [f64; 8] = [3370.39, 2383.94, 1683.78, 1190.55, 841.89, 595.28, 419.53, 297.64];
/// Diện tích trang A4 chuẩn (pt²): cạnh ngắn A4 = cạnh dài A5
pub const DIEN_TICH_A4: f64 = A_SIZES[4] * A_SIZES[5];
/// Cạnh dài tối đa (pt) để trang khổ A0 vẫn tính theo hệ số A0:
/// dài A0 + cạnh ngắn A4 = 1189 + 210 = 1399 mm
pub const NGUONG_A0_THEO_DIEN_TICH: f64 = A_SIZES[0] + A_SIZES[5];

pub const HE_SO_MAC_DINH: [f64; 8] = [16.0, 8.0, 4.0, 2.0, 1.0, 0.5, 0.25, 0.125];
/// 1 mm = 72 / 25,4 pt
pub const PT_MOI_MM: f64 = 72.0 / 25.4;

/// Cách quy A4 trang khổ A0 có cạnh dài vượt ngưỡng.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum A0Lon {
    /// Diện tích thật ÷ A4, không thấp hơn hệ số A0.
    DienTich,
    /// Hệ số cố định (≥ hệ số A0).
    CoDinh(f64),
}

/// Quy tắc xếp khổ + quy A4 (Cấu hình › Đếm trang A4). Mặc định = cách đếm từ trước tới nay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuyTac {
    pub he_so: [f64; 8],
    /// Ranh giới khổ theo cạnh dài (mm) A0|A1 … A6|A7; None = điểm giữa 2 cạnh chuẩn (mặc định).
    pub ranh_gioi_mm: Option<[f64; 7]>,
    /// Trang A0 dài hơn mức này (pt) quy theo `a0_lon`.
    pub a0_nguong_pt: f64,
    pub a0_lon: A0Lon,
}

impl Default for QuyTac {
    fn default() -> Self {
        Self { he_so: HE_SO_MAC_DINH, ranh_gioi_mm: None, a0_nguong_pt: NGUONG_A0_THEO_DIEN_TICH, a0_lon: A0Lon::DienTich }
    }
}

impl QuyTac {
    /// Khổ theo cạnh dài: ≥ ranh giới i → khổ i, nhỏ hơn ranh giới cuối → A7.
    pub fn kho(&self, chieu_rong: f64, chieu_cao: f64) -> usize {
        let Some(rg) = self.ranh_gioi_mm else { return xac_dinh_kho_a(chieu_rong, chieu_cao) };
        let canh_dai = chieu_rong.max(chieu_cao);
        if canh_dai.is_nan() {
            return 4;
        }
        rg.iter().position(|&r| canh_dai >= r * PT_MOI_MM).unwrap_or(7)
    }

    /// Tỷ lệ đã dùng của khoảng từ cạnh chuẩn tới ranh giới với khổ kề bên (cảnh báo "Sát ngưỡng").
    pub fn sat_nguong(&self, chieu_rong: f64, chieu_cao: f64, kho: usize) -> (f64, Option<usize>) {
        let Some(rg) = self.ranh_gioi_mm else { return muc_sat_nguong(chieu_rong, chieu_cao, kho) };
        let canh_dai = chieu_rong.max(chieu_cao);
        let chuan = A_SIZES[kho];
        let (ke_ben, ranh) = if canh_dai > chuan {
            if kho == 0 {
                return (0.0, None);
            }
            (kho - 1, rg[kho - 1])
        } else {
            if kho == 7 {
                return (0.0, None);
            }
            (kho + 1, rg[kho])
        };
        ((canh_dai - chuan).abs() / (ranh * PT_MOI_MM - chuan).abs(), Some(ke_ben))
    }

    /// Số trang A4 quy đổi của 1 trang.
    pub fn quy_a4(&self, kho: usize, chieu_rong: f64, chieu_cao: f64) -> f64 {
        if kho == 0 && chieu_rong.max(chieu_cao) > self.a0_nguong_pt {
            match self.a0_lon {
                A0Lon::DienTich => self.he_so[0].max(lam_tron_2(chieu_rong * chieu_cao / DIEN_TICH_A4)),
                A0Lon::CoDinh(h) => h,
            }
        } else {
            self.he_so[kho]
        }
    }

    /// Dòng ghi thêm vào khối KẾT QUẢ ĐẾM khi khác mặc định (để biết số liệu đếm theo tuỳ chỉnh).
    pub fn dong_tuy_chinh(&self) -> String {
        let mut s = String::new();
        if let Some(rg) = self.ranh_gioi_mm {
            s += &format!("Ranh giới khổ (mm): {}\n", rg.iter().map(|r| r.to_string().replace('.', ",")).collect::<Vec<_>>().join(" · "));
        }
        let nguong_mm = (self.a0_nguong_pt / PT_MOI_MM).round();
        match self.a0_lon {
            A0Lon::CoDinh(h) => s += &format!("Trang A0 dài hơn {nguong_mm} mm: hệ số cố định {}\n", h.to_string().replace('.', ",")),
            A0Lon::DienTich if self.a0_nguong_pt != NGUONG_A0_THEO_DIEN_TICH => s += &format!("Trang A0 dài hơn {nguong_mm} mm: quy A4 theo diện tích\n"),
            A0Lon::DienTich => {}
        }
        s
    }
}
/// Cảnh báo khi cạnh dài đã lệch >= 80% nửa khoảng tới khổ kề bên
pub const NGUONG_CANH_BAO: f64 = 0.8;
pub const DPI_CHUAN: f64 = 300.0;
/// Cảnh báo khi DPI lệch quá 3% so với DPI chuẩn
pub const DPI_SAI_SO: f64 = 0.03;

pub const TIEU_DE_TONG_HOP: &str = "STT\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4\tVi tri page";
pub const TIEU_DE_CHI_TIET: &str = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tKho giay\tDPI\tCanh bao\tKy so";
const GACH: &str = "------------------------------------------------------";

/// Xếp vào khổ có cạnh dài chuẩn gần nhất (ngưỡng là điểm giữa hai khổ liền kề).
/// Lớn hơn A0 vẫn tính A0, nhỏ hơn A7 vẫn tính A7. Trả về chỉ số trong KHO.
pub fn xac_dinh_kho_a(chieu_rong: f64, chieu_cao: f64) -> usize {
    let canh_dai = if chieu_rong > chieu_cao { chieu_rong } else { chieu_cao };
    if canh_dai > A_SIZES[0] {
        return 0;
    }
    if canh_dai < A_SIZES[7] {
        return 7;
    }
    for i in 0..7 {
        if A_SIZES[i + 1] <= canh_dai && canh_dai <= A_SIZES[i] {
            let diem_giua = (A_SIZES[i] + A_SIZES[i + 1]) / 2.0;
            return if canh_dai >= diem_giua { i } else { i + 1 };
        }
    }
    // Chỉ xảy ra khi kích thước là NaN (PDF hỏng); coi như A4 để không làm dừng việc đếm
    4
}

/// Trả về (tỷ lệ đã dùng của nửa khoảng tới khổ kề bên, khổ kề bên).
/// Tỷ lệ 0 = đúng kích thước chuẩn, gần 1 = sắp bị xếp sang khổ kề bên.
pub fn muc_sat_nguong(chieu_rong: f64, chieu_cao: f64, kho: usize) -> (f64, Option<usize>) {
    let canh_dai = chieu_rong.max(chieu_cao);
    let chuan = A_SIZES[kho];
    let ke_ben = if canh_dai > chuan {
        if kho == 0 {
            return (0.0, None); // Lớn hơn A0 vẫn tính A0
        }
        kho - 1
    } else {
        if kho == 7 {
            return (0.0, None); // Nhỏ hơn A7 vẫn tính A7
        }
        kho + 1
    };
    ((canh_dai - chuan).abs() / ((A_SIZES[ke_ben] - chuan).abs() / 2.0), Some(ke_ben))
}

/// round(x, 3) của Python (làm tròn thập phân chính xác)
fn lam_tron_3(x: f64) -> f64 {
    format!("{x:.3}").parse().unwrap_or(x)
}

/// Làm tròn 2 chữ số thập phân cho số trang quy A4 (tránh in 446.75984611351856)
pub fn lam_tron_2(x: f64) -> f64 {
    format!("{x:.2}").parse().unwrap_or(x)
}

/// Ghi số thực giống str(float) của Python cho các giá trị gặp ở đây: 606.24, 2526.0, 885.0
pub fn so_kieu_python(x: f64) -> String {
    let s = format!("{x}");
    if x.is_finite() && !s.contains('.') {
        format!("{s}.0")
    } else {
        s
    }
}

/// Đường dẫn tuyệt đối như os.path.abspath (không đổi ký tự, chỉ ghép thư mục hiện tại nếu cần)
fn duong_dan_tuyet_doi(p: &Path) -> String {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()).to_string_lossy().into_owned()
}

/// Ghi lại ảnh có nhiều điểm ảnh nhất trên trang cùng ma trận đặt ảnh (giống get_image_info)
#[derive(Default)]
struct AnhLonNhat {
    anh: Option<(u32, u32, Matrix)>,
}

impl AnhLonNhat {
    fn xet(&mut self, img: &Image, ctm: Matrix) {
        let (w, h) = (img.width(), img.height());
        if w == 0 || h == 0 {
            return;
        }
        // Lấy ảnh đầu tiên có diện tích lớn nhất, như max() của Python
        let lon_hon = match &self.anh {
            None => true,
            Some((bw, bh, _)) => (w as u64) * (h as u64) > (*bw as u64) * (*bh as u64),
        };
        if lon_hon {
            self.anh = Some((w, h, ctm));
        }
    }
}

impl NativeDevice for AnhLonNhat {
    fn fill_image(&mut self, img: &Image, ctm: Matrix, _alpha: f32, _cp: ColorParams) {
        self.xet(img, ctm);
    }

    fn fill_image_mask(&mut self, img: &Image, ctm: Matrix, _cs: &Colorspace, _color: &[f32], _alpha: f32, _cp: ColorParams) {
        self.xet(img, ctm);
    }
}

/// DPI thực tế của ảnh scan lớn nhất trên trang = số pixel / số inch ảnh chiếm trên trang.
/// Tính theo ma trận đặt ảnh nên không bị ảnh hưởng khi trang hoặc ảnh bị xoay.
fn tinh_dpi(trang: &mupdf::Page) -> Option<(f64, f64)> {
    let ghi_nhan = Rc::new(RefCell::new(AnhLonNhat::default()));
    let thiet_bi = Device::from_native(Rc::clone(&ghi_nhan)).ok()?;
    let _ = trang.run(&thiet_bi, &Matrix::IDENTITY);
    drop(thiet_bi);
    let (w, h, m) = ghi_nhan.borrow_mut().anh.take()?;
    let (a, b, c, d) = (m.a as f64, m.b as f64, m.c as f64, m.d as f64);
    let rong_inch = (a * a + b * b).sqrt() / 72.0;
    let cao_inch = (c * c + d * d).sqrt() / 72.0;
    if rong_inch == 0.0 || cao_inch == 0.0 {
        return None;
    }
    Some((w as f64 / rong_inch, h as f64 / cao_inch))
}

/// Kết quả của một file PDF
pub struct KetQuaFile {
    pub dong_chi_tiet: Vec<String>,
    pub dong_tong_hop: String,
    pub dem: [usize; 8],
    pub quy_a4: f64,
    pub sat_nguong: usize,
    pub lech_dpi: usize,
    /// Số trang có ít nhất một cảnh báo
    pub trang_canh_bao: usize,
    /// Chữ ký số của file: Some(true)=đã ký, Some(false)=chưa, None=không kiểm tra/không tra được
    pub ky_so: Option<bool>,
}

/// Đọc một file PDF; trang nào cũng có dòng chi tiết. Lỗi mở file trả về Err.
/// `kiem_tra_dpi = false` bỏ qua bước tính DPI (phải chạy nội dung trang, chậm với tài liệu văn bản nhiều trang)
/// Tài liệu mở một lần, dùng chung cho đọc trang và (khi cần) kiểm tra chữ ký.
/// PdfDocument kế thừa Document nên đọc trang qua deref, không phải mở file hai lần.
enum TaiLieu {
    Pdf(PdfDocument),
    Doc(Document),
}

impl TaiLieu {
    fn doc(&self) -> &Document {
        match self {
            TaiLieu::Pdf(p) => p, // deref PdfDocument -> Document
            TaiLieu::Doc(d) => d,
        }
    }
}

fn phan_tich_file(duong_dan: &Path, stt: usize, tong: usize, quy_tac: &QuyTac, kiem_tra_dpi: bool, kiem_tra_ky: bool, dung: &AtomicBool) -> Result<KetQuaFile, String> {
    let ten = duong_dan.to_string_lossy();
    // Khi cần kiểm tra chữ ký thì mở dạng PdfDocument (vẫn đọc trang được); nếu không thì mở Document như cũ
    let (tai_lieu, ky_so) = if kiem_tra_ky {
        let pdf = PdfDocument::open(ten.as_ref()).map_err(|e| e.to_string())?;
        let ky = Some(co_truong_da_ky(&pdf).unwrap_or(false));
        (TaiLieu::Pdf(pdf), ky)
    } else {
        (TaiLieu::Doc(Document::open(ten.as_ref()).map_err(|e| e.to_string())?), None)
    };
    let tai_lieu = tai_lieu.doc();
    let so_trang = tai_lieu.page_count().map_err(|e| e.to_string())? as usize;
    let duong_dan_tuyet_doi = duong_dan_tuyet_doi(duong_dan);

    // Cột chữ ký: Có / Khong / "-" (không kiểm tra hoặc không tra được)
    let ky_so_str = match (kiem_tra_ky, ky_so) {
        (false, _) => "-",
        (true, Some(true)) => "Có",
        (true, Some(false)) => "Không",
        (true, None) => "-",
    };

    let mut kq = KetQuaFile { dong_chi_tiet: Vec::with_capacity(so_trang), dong_tong_hop: String::new(), dem: [0; 8], quy_a4: 0.0, sat_nguong: 0, lech_dpi: 0, trang_canh_bao: 0, ky_so };
    let mut chuoi_kho: Vec<(usize, usize)> = Vec::new();
    let mut quy_a0 = 0.0; // trang A0 quy theo diện tích thật (>= hệ số A0)

    for i in 0..so_trang {
        if dung.load(Ordering::Relaxed) {
            break;
        }
        let trang = tai_lieu.load_page(i as i32).map_err(|e| e.to_string())?;
        let r = trang.bounds().map_err(|e| e.to_string())?;
        let chieu_rong = lam_tron_3(r.x1 as f64 - r.x0 as f64);
        let chieu_cao = lam_tron_3(r.y1 as f64 - r.y0 as f64);
        let huong_trang = if chieu_rong > chieu_cao { "Ngang" } else { "Dọc" };
        let kho = quy_tac.kho(chieu_rong, chieu_cao);
        kq.dem[kho] += 1;
        if kho == 0 {
            quy_a0 += quy_tac.quy_a4(0, chieu_rong, chieu_cao);
        }

        let mut canh_bao = Vec::new();
        let (ty_le, kho_ke_ben) = quy_tac.sat_nguong(chieu_rong, chieu_cao, kho);
        if ty_le >= NGUONG_CANH_BAO {
            // Làm tròn xuống để trang chưa vượt ngưỡng không bị hiển thị 100%
            canh_bao.push(format!("Sát ngưỡng {} ({}%)", KHO[kho_ke_ben.unwrap()], (ty_le * 100.0) as i64));
            kq.sat_nguong += 1;
        }
        let dpi = if kiem_tra_dpi { tinh_dpi(&trang) } else { None };
        let dpi_str = match dpi {
            None => "-".to_string(),
            Some((dpi_ngang, dpi_doc)) => {
                // round() của Python làm tròn nửa về số chẵn
                let (n, d) = (dpi_ngang.round_ties_even() as i64, dpi_doc.round_ties_even() as i64);
                // Hai trục chênh ≤ 2 DPI coi như bằng nhau (sai số làm tròn ảnh scan): gộp thành 1 số
                let s = if (n - d).abs() <= 2 {
                    ((dpi_ngang + dpi_doc) / 2.0).round_ties_even().to_string()
                } else {
                    format!("{n}x{d}")
                };
                if [dpi_ngang, dpi_doc].iter().any(|x| (x - DPI_CHUAN).abs() > DPI_CHUAN * DPI_SAI_SO) {
                    canh_bao.push(format!("DPI {s} khác {DPI_CHUAN}"));
                    kq.lech_dpi += 1;
                }
                s
            }
        };

        if !canh_bao.is_empty() {
            kq.trang_canh_bao += 1;
        }
        kq.dong_chi_tiet.push(format!(
            "{stt}/{tong}\t{duong_dan_tuyet_doi} => trang: {}\t{}\t{}\t{huong_trang}\t{}\t{dpi_str}\t{}\t{ky_so_str}",
            i + 1,
            so_kieu_python(chieu_rong),
            so_kieu_python(chieu_cao),
            KHO[kho],
            canh_bao.join("; ")
        ));
        match chuoi_kho.last_mut() {
            Some((n, k)) if *k == kho => *n += 1,
            _ => chuoi_kho.push((1, kho)),
        }
    }

    // Cộng theo đúng thứ tự A0..A7 như Python để số thực ra giống hệt
    kq.quy_a4 = lam_tron_2((1..8).fold(quy_a0, |tong, i| tong + kq.dem[i] as f64 * quy_tac.he_so[i]));
    let dem: Vec<String> = kq.dem.iter().map(|d| d.to_string()).collect();
    let chuoi: Vec<String> = chuoi_kho.iter().map(|(n, k)| format!("{n}{}", KHO[*k])).collect();
    kq.dong_tong_hop = format!("{stt}/{tong}\t{duong_dan_tuyet_doi}\t{so_trang}\t{}\t'=>\t{}\t{}", dem.join("\t"), so_kieu_python(kq.quy_a4), chuoi.join(", "));
    Ok(kq)
}

/// Sự kiện gửi về giao diện trong lúc đếm
pub enum SuKien {
    /// Một dòng hiển thị (tiêu đề, dòng chi tiết trang, hoặc thông báo lỗi mở file)
    Dong(String),
    /// Đang quét tìm file PDF: số file .pdf đã thấy tới thời điểm này
    DangTim(usize),
    TienDo(usize, usize),
    Xong(KetQuaDem),
}

/// Toàn bộ kết quả của một lần đếm, dùng để hiển thị và ghi file
pub struct KetQuaDem {
    pub dong_tong_hop: Vec<String>,
    pub dong_chi_tiet: Vec<String>,
    /// Khối "KẾT QUẢ ĐẾM" ở cuối file tổng hợp
    pub tong_hop: String,
    pub so_file_loi: usize,
    pub tong_so_trang: usize,
    /// Tổng quy ra A4; None khi chưa đếm được file nào
    pub quy_a4: Option<f64>,
    /// Số trang có ít nhất một cảnh báo (sát ngưỡng hoặc DPI)
    pub so_trang_canh_bao: usize,
}

/// Tên thư mục giống os.path.basename (đường dẫn kết thúc bằng dấu phân cách thì rỗng)
fn ten_thu_muc(thu_muc: &str) -> String {
    if thu_muc.ends_with('/') || thu_muc.ends_with(std::path::MAIN_SEPARATOR) {
        return String::new();
    }
    Path::new(thu_muc).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Danh sách file .pdf trong thư mục và thư mục con, sắp theo đường dẫn như sorted() của Python
/// Thư mục con bị bỏ qua khi quét (rác/hệ thống/cache) để không chui vào vùng chậm.
/// Chỉ áp cho THƯ MỤC CON; thư mục gốc người dùng chọn luôn được quét.
/// Lưu ý KHÔNG bỏ qua CloudStorage (Google Drive/iCloud) vì tài liệu có thể nằm ở đó.
fn bo_qua_thu_muc(ten: &std::ffi::OsStr) -> bool {
    let ten = ten.to_string_lossy();
    if ten.starts_with('.') {
        return true; // thư mục ẩn: .Trash, .git, .cache…
    }
    const DANH_SACH: [&str; 7] = [
        "node_modules",
        "Library",                   // macOS: cache/app data của người dùng
        "AppData",                   // Windows: dữ liệu ứng dụng
        "Windows",                   // Windows: thư mục hệ thống
        "Program Files",
        "Program Files (x86)",
        "$Recycle.Bin",
    ];
    DANH_SACH.iter().any(|x| ten.eq_ignore_ascii_case(x))
}

/// Quét tìm file .pdf bằng bộ duyệt song song jwalk (nhanh hơn nhiều trên cây lớn và thư mục mạng).
/// Vừa duyệt vừa báo số file đã thấy và kiểm tra cờ dừng để app không "đơ" và bấm Dừng là dừng ngay.
/// Bỏ qua thư mục hệ thống/cache (bo_qua_thu_muc), thư mục không đọc được, và không đi theo symlink.
pub(crate) fn tim_file_pdf(thu_muc: &str, dung: &AtomicBool, gui: &Sender<SuKien>) -> Vec<PathBuf> {
    let duyet = jwalk::WalkDir::new(thu_muc)
        .skip_hidden(false) // tự lọc ẩn trong bo_qua_thu_muc để vẫn quét được thư mục gốc là ẩn
        .follow_links(false)
        .process_read_dir(|depth, _path, _state, children| {
            // depth None là lần đọc ảo sinh ra entry thư mục gốc: gốc người dùng chọn luôn được quét,
            // kể cả khi tên là ".", bắt đầu bằng "." hay trùng danh sách bỏ qua
            if depth.is_none() {
                return;
            }
            // Cắt nhánh: không đi vào thư mục con rác/hệ thống; bỏ entry không đọc được
            children.retain(|r| match r {
                Ok(e) => !(e.file_type.is_dir() && bo_qua_thu_muc(&e.file_name)),
                Err(_) => false,
            });
        });

    let mut ds: Vec<(String, PathBuf)> = Vec::new();
    for muc in duyet {
        if dung.load(Ordering::Relaxed) {
            break;
        }
        let Ok(muc) = muc else {
            continue;
        };
        // Đuôi .pdf không phân biệt hoa/thường (.PDF, .Pdf…); so trên byte để không cắt giữa ký tự UTF-8
        let ten = muc.file_name().to_string_lossy();
        let la_pdf = ten.len() >= 4 && ten.as_bytes()[ten.len() - 4..].eq_ignore_ascii_case(b".pdf");
        if !muc.file_type().is_dir() && la_pdf {
            ds.push((muc.path().to_string_lossy().into_owned(), muc.path()));
            if ds.len() % 100 == 0 {
                let _ = gui.send(SuKien::DangTim(ds.len()));
            }
        }
    }
    ds.sort_by(|a, b| a.0.cmp(&b.0));
    ds.into_iter().map(|(_, p)| p).collect()
}

/// Đếm cả thư mục. Các file được đọc song song nhưng kết quả gửi về đúng thứ tự file.
pub fn dem_thu_muc(thu_muc: &str, quy_tac: QuyTac, kiem_tra_dpi: bool, kiem_tra_ky: bool, dung: &AtomicBool, gui: &Sender<SuKien>) {
    let bat_dau = Instant::now();
    let cac_file = tim_file_pdf(thu_muc, dung, gui);
    let tong = cac_file.len();
    let _ = gui.send(SuKien::Dong(TIEU_DE_CHI_TIET.to_string()));

    let mut dong_tong_hop = Vec::new();
    let mut dong_chi_tiet = Vec::new();
    let mut dem_tong = [0usize; 8];
    let mut quy_a4_tong: Option<f64> = None; // None: chưa đếm được file nào (Python in ra "0")
    let (mut sat_nguong, mut lech_dpi, mut trang_canh_bao, mut so_file_ky_so) = (0usize, 0usize, 0usize, 0usize);
    let mut file_loi = Vec::new();
    let mut da_xong = 0usize;

    let (tx, rx) = mpsc::channel::<(usize, Result<KetQuaFile, String>)>();
    std::thread::scope(|s| {
        let cac_file = &cac_file;
        s.spawn(move || {
            cac_file.par_iter().enumerate().for_each_with(tx, |tx, (i, p)| {
                if dung.load(Ordering::Relaxed) {
                    return;
                }
                let _ = tx.send((i, phan_tich_file(p, i + 1, tong, &quy_tac, kiem_tra_dpi, kiem_tra_ky, dung)));
            });
        });

        // Gom kết quả về đúng thứ tự file; khi bấm Dừng chỉ giữ các file liền mạch từ đầu
        let mut cho: BTreeMap<usize, Result<KetQuaFile, String>> = BTreeMap::new();
        for (i, kq) in rx {
            cho.insert(i, kq);
            while let Some(kq) = cho.remove(&da_xong) {
                match kq {
                    Ok(kq) => {
                        for dong in &kq.dong_chi_tiet {
                            let _ = gui.send(SuKien::Dong(dong.clone()));
                        }
                        for k in 0..8 {
                            dem_tong[k] += kq.dem[k];
                        }
                        quy_a4_tong = Some(lam_tron_2(quy_a4_tong.unwrap_or(0.0) + kq.quy_a4));
                        sat_nguong += kq.sat_nguong;
                        if kq.ky_so == Some(true) {
                            so_file_ky_so += 1;
                        }
                        trang_canh_bao += kq.trang_canh_bao;
                        lech_dpi += kq.lech_dpi;
                        dong_chi_tiet.extend(kq.dong_chi_tiet);
                        dong_tong_hop.push(kq.dong_tong_hop);
                    }
                    Err(_) => {
                        let p = &cac_file[da_xong];
                        let _ = gui.send(SuKien::Dong(format!("Lỗi mở tệp: {}", p.to_string_lossy())));
                        file_loi.push(duong_dan_tuyet_doi(p));
                    }
                }
                da_xong += 1;
                let _ = gui.send(SuKien::TienDo(da_xong, tong));
            }
        }
    });

    let tong_so_trang: usize = dem_tong.iter().sum();
    let (quy_str, tang_them_str) = match quy_a4_tong {
        Some(q) => (so_kieu_python(q), so_kieu_python(q - tong_so_trang as f64)),
        None => ("0".to_string(), "0".to_string()),
    };
    // Thời gian dạng giờ:phút:giây.mmm như bản Python
    let giay = lam_tron_3(bat_dau.elapsed().as_secs_f64());
    let (gio, con_lai) = ((giay / 3600.0).floor(), giay % 3600.0);
    let (phut, giay) = ((con_lai / 60.0).floor(), con_lai % 60.0);

    let mut tong_hop = format!(
        "\nKẾT QUẢ ĐẾM\n===========================\n\
         Tổng số file pdf trong thư mục {}: {tong} \n\
         Tổng số trang {tong_so_trang} các loại => quy ra A4: {quy_str} (+{tang_them_str})\n\
         Thời gian đếm: {}:{}:{giay:.3}\n{}{GACH}\nTrong đó, bao gồm:\n",
        ten_thu_muc(thu_muc),
        gio as i64,
        phut as i64,
        quy_tac.dong_tuy_chinh()
    );
    for k in 0..8 {
        tong_hop += &format!(" - Tổng số trang {}: {}\n", KHO[k], dem_tong[k]);
    }
    tong_hop += &format!(
        "{GACH}\nCảnh báo cần kiểm tra (xem cột Canh bao ở danh sách trang):\n\
         \x20- Số trang sát ngưỡng chuyển khổ (>= {}% nửa khoảng): {sat_nguong}\n\
         \x20- Số trang có DPI khác {DPI_CHUAN}: {}\n\
         \x20- Số file đã ký số: {}\n{GACH}\n",
        (NGUONG_CANH_BAO * 100.0).round() as i64,
        if kiem_tra_dpi { lech_dpi.to_string() } else { "không kiểm tra".to_string() },
        if kiem_tra_ky { so_file_ky_so.to_string() } else { "không kiểm tra".to_string() }
    );
    if !file_loi.is_empty() {
        tong_hop += &format!("Danh sách file PDF bị lỗi ({} file, không được đếm):\n{}\n{GACH}\n", file_loi.len(), file_loi.join("\n"));
    }

    let so_file_loi = file_loi.len();
    let _ = gui.send(SuKien::Xong(KetQuaDem { dong_tong_hop, dong_chi_tiet, tong_hop, so_file_loi, tong_so_trang, quy_a4: quy_a4_tong, so_trang_canh_bao: trang_canh_bao }));
}

/// Tên file gợi ý: <thư mục>/<tên thư mục>_dem.txt
pub fn duong_dan_luu_mac_dinh(thu_muc: &str) -> PathBuf {
    let ten = match ten_thu_muc(thu_muc.trim_end_matches(['/', std::path::MAIN_SEPARATOR])) {
        t if t.is_empty() => "ket_qua".to_string(),
        t => t,
    };
    Path::new(thu_muc).join(format!("{ten}_dem.txt"))
}

/// Ghi file tổng hợp vào `duong_dan` (ví dụ thumuc_dem.txt) và file chi tiết vào <tên file>_chitiet.txt
/// cùng thư mục (ví dụ thumuc_dem_chitiet.txt). Các cột cách nhau bởi tab; file đã có thì ghi đè.
/// Trả về đường dẫn file chi tiết.
pub fn luu_ket_qua(kq: &KetQuaDem, duong_dan: &Path) -> io::Result<PathBuf> {
    let mut f = BufWriter::new(File::create(duong_dan)?);
    writeln!(f, "{TIEU_DE_TONG_HOP}")?;
    for dong in &kq.dong_tong_hop {
        writeln!(f, "{dong}")?;
    }
    f.write_all(kq.tong_hop.as_bytes())?;
    f.flush()?;

    let goc = duong_dan.with_extension("");
    let duoi = duong_dan.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "txt".to_string());
    let duong_dan_chi_tiet = PathBuf::from(format!("{}_chitiet.{duoi}", goc.to_string_lossy()));
    let mut f = BufWriter::new(File::create(&duong_dan_chi_tiet)?);
    writeln!(f, "{TIEU_DE_CHI_TIET}")?;
    for dong in &kq.dong_chi_tiet {
        writeln!(f, "{dong}")?;
    }
    f.flush()?;
    Ok(duong_dan_chi_tiet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bo_qua_thu_muc_he_thong_va_an_nhung_giu_cloudstorage() {
        use std::ffi::OsStr;
        for ten in ["node_modules", "Library", ".git", ".Trash", "AppData", "Windows", "$Recycle.Bin"] {
            assert!(bo_qua_thu_muc(OsStr::new(ten)), "phải bỏ qua {ten}");
        }
        for ten in ["02", "CloudStorage", "GoogleDrive", "Documents", "ho_so"] {
            assert!(!bo_qua_thu_muc(OsStr::new(ten)), "KHÔNG được bỏ qua {ten}");
        }
    }

    const PT: f64 = 72.0 / 25.4;

    #[test]
    fn kho_chuan_doc_va_ngang() {
        let chuan = [(841.0, 1189.0), (594.0, 841.0), (420.0, 594.0), (297.0, 420.0), (210.0, 297.0), (148.0, 210.0), (105.0, 148.0), (74.0, 105.0)];
        for (k, (w, h)) in chuan.iter().enumerate() {
            assert_eq!(xac_dinh_kho_a(w * PT, h * PT), k);
            assert_eq!(xac_dinh_kho_a(h * PT, w * PT), k);
        }
    }

    #[test]
    fn lon_hon_a0_van_la_a0_nho_hon_a7_van_la_a7() {
        assert_eq!(xac_dinh_kho_a(5000.0, 9000.0), 0);
        assert_eq!(xac_dinh_kho_a(10.0, 20.0), 7);
    }

    #[test]
    fn nguong_la_diem_giua_hai_kho_va_bang_nguong_thi_lay_kho_lon() {
        let giua = (A_SIZES[3] + A_SIZES[4]) / 2.0; // A3/A4
        assert_eq!(xac_dinh_kho_a(500.0, giua), 3);
        assert_eq!(xac_dinh_kho_a(500.0, giua - 0.001), 4);
    }

    #[test]
    fn a4_ngang_gan_vuong_khong_con_doi_thanh_a5() {
        // 754,56 x 539,28 pt từng bị quy tắc cũ đổi thành A5
        assert_eq!(xac_dinh_kho_a(754.56, 539.28), 4);
    }

    #[test]
    fn muc_sat_nguong_tinh_theo_nua_khoang() {
        let (ty_le, ke) = muc_sat_nguong(500.0, A_SIZES[4], 4);
        assert_eq!((ty_le, ke), (0.0, Some(5)));
        let (ty_le, ke) = muc_sat_nguong(507.6, 373.68, 5); // trang A5 sát A6 trong dữ liệu thật
        assert_eq!(ke, Some(6));
        assert!(ty_le > 0.99 && ty_le < 1.0);
        assert_eq!(muc_sat_nguong(9000.0, 5000.0, 0), (0.0, None));
    }

    #[test]
    fn a0_quy_a4_theo_dien_tich_khong_thap_hon_he_so_a0() {
        let hs = HE_SO_MAC_DINH;
        // Đúng A0 hoặc nhỏ hơn: theo hệ số A0 (16)
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, A_SIZES[1], A_SIZES[0]), 16.0);
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, 2000.0, 3000.0), 16.0);
        // Cạnh dài 1398 mm (chưa vượt 1189 + 210): vẫn 16 dù diện tích lớn hơn A0
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, 841.0 * PT, 1398.0 * PT), 16.0);
        // Cạnh dài 1400 mm: theo diện tích 841 x 1400 / (210 x 297) = 18,88
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, 841.0 * PT, 1400.0 * PT), 18.88);
        // Dải dài nhưng hẹp: vượt ngưỡng chiều dài nhưng diện tích nhỏ → không thấp hơn 16
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, 1.0, 14400.0), 16.0);
        // Khổ 1189 x 1682 mm (2 x A0): theo diện tích thật ~ 32 A4
        let q = QuyTac { he_so: hs, ..Default::default() }.quy_a4(0, 1189.0 * PT, 1682.0 * PT);
        assert!((q - 32.0).abs() < 0.1, "{q}");
        // Khổ khác vẫn theo hệ số
        assert_eq!(QuyTac { he_so: hs, ..Default::default() }.quy_a4(4, 9999.0, 9999.0), 1.0);
    }

    #[test]
    fn so_ghi_giong_python() {
        assert_eq!(so_kieu_python(606.24), "606.24");
        assert_eq!(so_kieu_python(2526.0), "2526.0");
        assert_eq!(so_kieu_python(0.125), "0.125");
        assert_eq!(lam_tron_3(841.8897637795275), 841.89);
    }

    /// PDF tối thiểu không có xref (MuPDF tự sửa được): chèn thêm vào Page, object rời, và Catalog
    fn pdf_toi_thieu(trang_them: &[u8], obj_them: &[u8], catalog_them: &[u8]) -> Vec<u8> {
        let mut v = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R".to_vec();
        v.extend_from_slice(catalog_them);
        v.extend_from_slice(b">>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 595 842]");
        v.extend_from_slice(trang_them);
        v.extend_from_slice(b">>endobj\n");
        v.extend_from_slice(obj_them);
        v.extend_from_slice(b"trailer<</Root 1 0 R>>\n%%EOF\n");
        v
    }

    fn thu_muc_tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_test_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn quet_ca_thu_muc_goc_an_hoac_trung_danh_sach_bo_qua_nhung_van_bo_qua_thu_muc_con() {
        let goc = thu_muc_tam("goc_an");
        let pdf = pdf_toi_thieu(b"", b"", b"");
        for ten_goc in [".an", "node_modules"] {
            let d = goc.join(ten_goc);
            let con = d.join("node_modules");
            std::fs::create_dir_all(&con).unwrap();
            for ten in ["a.pdf", "B.PDF", "c.Pdf", "d.pdfx", "e.txt"] {
                std::fs::write(d.join(ten), &pdf).unwrap();
            }
            std::fs::write(con.join("b.pdf"), &pdf).unwrap();
            let (gui, _nhan) = mpsc::channel();
            let ds = tim_file_pdf(d.to_str().unwrap(), &AtomicBool::new(false), &gui);
            // Đuôi .pdf không phân biệt hoa/thường; .pdfx và .txt bị bỏ; con node_modules bị bỏ
            assert_eq!(ds, vec![d.join("B.PDF"), d.join("a.pdf"), d.join("c.Pdf")], "gốc {ten_goc} phải được quét đúng");
        }
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn pdf_co_widget_chu_ky_tren_trang_mo_duoc_va_bao_da_ky() {
        let d = thu_muc_tam("widget_ky");
        let pdf = pdf_toi_thieu(
            b"/Annots[4 0 R]",
            b"4 0 obj<</Type/Annot/Subtype/Widget/FT/Sig/T(chuky)/Rect[50 50 250 120]/P 3 0 R/F 4/V 5 0 R>>endobj\n\
              5 0 obj<</Type/Sig/Filter/Adobe.PPKLite/SubFilter/adbe.pkcs7.detached/ByteRange[0 0 0 0]/Contents<00>>>endobj\n",
            b"/AcroForm<</Fields[4 0 R]/SigFlags 3>>",
        );
        let p = d.join("ky.pdf");
        std::fs::write(&p, &pdf).unwrap();
        let kq = phan_tich_file(&p, 1, 1, &QuyTac::default(), true, true, &AtomicBool::new(false)).expect("phải mở được PDF có widget chữ ký");
        assert_eq!(kq.ky_so, Some(true));
        assert_eq!(kq.dong_chi_tiet.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn file_hong_khong_lam_dung_dem_va_bam_dung_van_nhan_duoc_xong() {
        let d = thu_muc_tam("hong_va_dung");
        let pdf = pdf_toi_thieu(b"", b"", b"");
        std::fs::write(d.join("00_rong.pdf"), b"").unwrap();
        std::fs::write(d.join("01_rac.pdf"), b"khong phai pdf").unwrap();
        const SO_FILE_OK: usize = 1500;
        for i in 0..SO_FILE_OK {
            std::fs::write(d.join(format!("{:04}_ok.pdf", i + 10)), &pdf).unwrap();
        }
        let thu_muc = d.to_str().unwrap();

        // Đếm trọn: 2 file hỏng được báo lỗi, các file còn lại vẫn đếm đủ
        let (gui, nhan) = mpsc::channel();
        dem_thu_muc(thu_muc, QuyTac::default(), false, true, &AtomicBool::new(false), &gui);
        drop(gui);
        let xong = nhan.into_iter().find_map(|s| if let SuKien::Xong(kq) = s { Some(kq) } else { None }).expect("phải có Xong");
        assert_eq!(xong.so_file_loi, 2);
        assert_eq!(xong.dong_tong_hop.len(), SO_FILE_OK);
        assert_eq!(xong.tong_so_trang, SO_FILE_OK);

        // Bấm Dừng ngay khi có tiến độ đầu tiên: vẫn phải nhận được Xong, không panic
        let dung = AtomicBool::new(false);
        let (gui, nhan) = mpsc::channel();
        let xong = std::thread::scope(|s| {
            s.spawn(|| dem_thu_muc(thu_muc, QuyTac::default(), false, false, &dung, &gui));
            let mut xong = None;
            for su_kien in &nhan {
                match su_kien {
                    SuKien::TienDo(..) => dung.store(true, Ordering::Relaxed),
                    SuKien::Xong(kq) => {
                        xong = Some(kq);
                        break;
                    }
                    _ => {}
                }
            }
            xong
        })
        .expect("bấm Dừng vẫn phải có Xong");
        assert!(xong.dong_tong_hop.len() <= SO_FILE_OK);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Chẩn đoán tay: in lý do một file không mở được.
    /// Chạy: DEMA4_FILE=<đường dẫn> cargo test -- --ignored --nocapture in_ly_do_loi_mo_file
    #[test]
    #[ignore = "chẩn đoán tay, cần DEMA4_FILE"]
    fn in_ly_do_loi_mo_file() {
        let p = PathBuf::from(std::env::var("DEMA4_FILE").expect("DEMA4_FILE"));
        match phan_tich_file(&p, 1, 1, &QuyTac::default(), true, true, &AtomicBool::new(false)) {
            Ok(kq) => println!("Mở OK: {} trang, ký số {:?}", kq.dong_chi_tiet.len(), kq.ky_so),
            Err(e) => println!("LỖI: {e}"),
        }
    }

    #[test]
    fn ten_file_mac_dinh() {
        assert_eq!(duong_dan_luu_mac_dinh("/a/thumuc"), PathBuf::from("/a/thumuc/thumuc_dem.txt"));
        assert_eq!(duong_dan_luu_mac_dinh("/a/thumuc/"), PathBuf::from("/a/thumuc/thumuc_dem.txt"));
    }
}
