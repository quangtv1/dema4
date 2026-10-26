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

use mupdf::{ColorParams, Colorspace, Device, Document, Image, Matrix, NativeDevice};
use rayon::prelude::*;
use walkdir::WalkDir;

pub const KHO: [&str; 8] = ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7"];
/// Cạnh dài chuẩn (pt) của từng khổ, từ lớn đến nhỏ
pub const A_SIZES: [f64; 8] = [3370.39, 2383.94, 1683.78, 1190.55, 841.89, 595.28, 419.53, 297.64];
pub const HE_SO_MAC_DINH: [f64; 8] = [16.0, 8.0, 4.0, 2.0, 1.0, 0.5, 0.25, 0.125];
/// Cảnh báo khi cạnh dài đã lệch >= 80% nửa khoảng tới khổ kề bên
pub const NGUONG_CANH_BAO: f64 = 0.8;
pub const DPI_CHUAN: f64 = 300.0;
/// Cảnh báo khi DPI lệch quá 3% so với DPI chuẩn
pub const DPI_SAI_SO: f64 = 0.03;

pub const TIEU_DE_TONG_HOP: &str = "STT\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4\tVi tri page";
pub const TIEU_DE_CHI_TIET: &str = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tKho giay\tDPI\tCanh bao";
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
}

/// Đọc một file PDF; trang nào cũng có dòng chi tiết. Lỗi mở file trả về Err.
/// `kiem_tra_dpi = false` bỏ qua bước tính DPI (phải chạy nội dung trang, chậm với tài liệu văn bản nhiều trang)
fn phan_tich_file(duong_dan: &Path, stt: usize, tong: usize, he_so: &[f64; 8], kiem_tra_dpi: bool, dung: &AtomicBool) -> Result<KetQuaFile, String> {
    let ten = duong_dan.to_string_lossy();
    let tai_lieu = Document::open(ten.as_ref()).map_err(|e| e.to_string())?;
    let so_trang = tai_lieu.page_count().map_err(|e| e.to_string())? as usize;
    let duong_dan_tuyet_doi = duong_dan_tuyet_doi(duong_dan);

    let mut kq = KetQuaFile { dong_chi_tiet: Vec::with_capacity(so_trang), dong_tong_hop: String::new(), dem: [0; 8], quy_a4: 0.0, sat_nguong: 0, lech_dpi: 0, trang_canh_bao: 0 };
    let mut chuoi_kho: Vec<(usize, usize)> = Vec::new();

    for i in 0..so_trang {
        if dung.load(Ordering::Relaxed) {
            break;
        }
        let trang = tai_lieu.load_page(i as i32).map_err(|e| e.to_string())?;
        let r = trang.bounds().map_err(|e| e.to_string())?;
        let chieu_rong = lam_tron_3(r.x1 as f64 - r.x0 as f64);
        let chieu_cao = lam_tron_3(r.y1 as f64 - r.y0 as f64);
        let huong_trang = if chieu_rong > chieu_cao { "Ngang" } else { "Doc" };
        let kho = xac_dinh_kho_a(chieu_rong, chieu_cao);
        kq.dem[kho] += 1;

        let mut canh_bao = Vec::new();
        let (ty_le, kho_ke_ben) = muc_sat_nguong(chieu_rong, chieu_cao, kho);
        if ty_le >= NGUONG_CANH_BAO {
            // Làm tròn xuống để trang chưa vượt ngưỡng không bị hiển thị 100%
            canh_bao.push(format!("Sat nguong {} ({}%)", KHO[kho_ke_ben.unwrap()], (ty_le * 100.0) as i64));
            kq.sat_nguong += 1;
        }
        let dpi = if kiem_tra_dpi { tinh_dpi(&trang) } else { None };
        let dpi_str = match dpi {
            None => "-".to_string(),
            Some((dpi_ngang, dpi_doc)) => {
                // round() của Python làm tròn nửa về số chẵn
                let (n, d) = (dpi_ngang.round_ties_even() as i64, dpi_doc.round_ties_even() as i64);
                let s = if n == d { n.to_string() } else { format!("{n}x{d}") };
                if [dpi_ngang, dpi_doc].iter().any(|x| (x - DPI_CHUAN).abs() > DPI_CHUAN * DPI_SAI_SO) {
                    canh_bao.push(format!("DPI {s} khac {DPI_CHUAN}"));
                    kq.lech_dpi += 1;
                }
                s
            }
        };

        if !canh_bao.is_empty() {
            kq.trang_canh_bao += 1;
        }
        kq.dong_chi_tiet.push(format!(
            "{stt}/{tong}\t{duong_dan_tuyet_doi} => trang: {}\t{}\t{}\t{huong_trang}\t{}\t{dpi_str}\t{}",
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
    kq.quy_a4 = (0..8).fold(0.0, |tong, i| tong + kq.dem[i] as f64 * he_so[i]);
    let dem: Vec<String> = kq.dem.iter().map(|d| d.to_string()).collect();
    let chuoi: Vec<String> = chuoi_kho.iter().map(|(n, k)| format!("{n}{}", KHO[*k])).collect();
    kq.dong_tong_hop = format!("{stt}/{tong}\t{duong_dan_tuyet_doi}\t{so_trang}\t{}\t'=>\t{}\t{}", dem.join("\t"), so_kieu_python(kq.quy_a4), chuoi.join(", "));
    Ok(kq)
}

/// Sự kiện gửi về giao diện trong lúc đếm
pub enum SuKien {
    /// Một dòng hiển thị (tiêu đề, dòng chi tiết trang, hoặc thông báo lỗi mở file)
    Dong(String),
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
fn tim_file_pdf(thu_muc: &str) -> Vec<PathBuf> {
    let mut ds: Vec<(String, PathBuf)> = WalkDir::new(thu_muc)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir() && e.file_name().to_string_lossy().ends_with(".pdf"))
        .map(|e| (e.path().to_string_lossy().into_owned(), e.into_path()))
        .collect();
    ds.sort_by(|a, b| a.0.cmp(&b.0));
    ds.into_iter().map(|(_, p)| p).collect()
}

/// Đếm cả thư mục. Các file được đọc song song nhưng kết quả gửi về đúng thứ tự file.
pub fn dem_thu_muc(thu_muc: &str, he_so: [f64; 8], kiem_tra_dpi: bool, dung: &AtomicBool, gui: &Sender<SuKien>) {
    let bat_dau = Instant::now();
    let cac_file = tim_file_pdf(thu_muc);
    let tong = cac_file.len();
    let _ = gui.send(SuKien::Dong(TIEU_DE_CHI_TIET.to_string()));

    let mut dong_tong_hop = Vec::new();
    let mut dong_chi_tiet = Vec::new();
    let mut dem_tong = [0usize; 8];
    let mut quy_a4_tong: Option<f64> = None; // None: chưa đếm được file nào (Python in ra "0")
    let (mut sat_nguong, mut lech_dpi, mut trang_canh_bao) = (0usize, 0usize, 0usize);
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
                let _ = tx.send((i, phan_tich_file(p, i + 1, tong, &he_so, kiem_tra_dpi, dung)));
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
                        quy_a4_tong = Some(quy_a4_tong.unwrap_or(0.0) + kq.quy_a4);
                        sat_nguong += kq.sat_nguong;
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
         Thời gian đếm: {}:{}:{giay:.3}\n{GACH}\nTrong đó, bao gồm:\n",
        ten_thu_muc(thu_muc),
        gio as i64,
        phut as i64
    );
    for k in 0..8 {
        tong_hop += &format!(" - Tổng số trang {}: {}\n", KHO[k], dem_tong[k]);
    }
    tong_hop += &format!(
        "{GACH}\nCảnh báo cần kiểm tra (xem cột Canh bao ở danh sách trang):\n\
         \x20- Số trang sát ngưỡng chuyển khổ (>= {}% nửa khoảng): {sat_nguong}\n\
         \x20- Số trang có DPI khác {DPI_CHUAN}: {}\n{GACH}\n",
        (NGUONG_CANH_BAO * 100.0).round() as i64,
        if kiem_tra_dpi { lech_dpi.to_string() } else { "không kiểm tra".to_string() }
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
    fn so_ghi_giong_python() {
        assert_eq!(so_kieu_python(606.24), "606.24");
        assert_eq!(so_kieu_python(2526.0), "2526.0");
        assert_eq!(so_kieu_python(0.125), "0.125");
        assert_eq!(lam_tron_3(841.8897637795275), 841.89);
    }

    #[test]
    fn ten_file_mac_dinh() {
        assert_eq!(duong_dan_luu_mac_dinh("/a/thumuc"), PathBuf::from("/a/thumuc/thumuc_dem.txt"));
        assert_eq!(duong_dan_luu_mac_dinh("/a/thumuc/"), PathBuf::from("/a/thumuc/thumuc_dem.txt"));
    }
}
