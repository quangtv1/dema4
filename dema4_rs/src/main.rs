// Trên Windows chạy như ứng dụng cửa sổ (không bật cửa sổ đen console)
#![cfg_attr(windows, windows_subsystem = "windows")]

//! Đếm số trang A4 trong thư mục PDF.
//! - Không tham số: mở giao diện.
//! - `dema4_rs <thư mục>`: đếm không cần giao diện, ghi <thư mục>/<tên>_dem.txt và <tên>_dem_chitiet.txt.
//! - Thêm `--dpi` để kiểm tra DPI ảnh scan, `--kyso` để kiểm tra chữ ký số (mặc định tắt).
//! - `dema4_rs <thư mục> --khong-ghi`: chỉ đếm và in kết quả (không ghi file, không gửi thống kê), dùng để đo tốc độ.
//! - `dema4_rs --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục cha>]`: tách văn bản theo nhãn,
//!   ghi `<cha>/<tên>_vanban/…` và `ketqua_tach.txt` trong thư mục dữ liệu ẩn của thư mục pdf (cạnh file chạy); chưa có nhãn thì phân tích tự động trước.

#[allow(dead_code)] // nối dần vào màn Đếm / Tách
mod cau_hinh;
mod dem_pdf;
mod gioi_thieu;
mod license;
mod google_sheet;
mod gui;
#[allow(dead_code)] // nối dần vào giao diện
mod gui_tach;
#[allow(dead_code)] // dùng dần từ các phần phân tích / giao diện tách văn bản
mod tach_vb;
mod telegram;
mod tro_giup;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

fn main() {
    match std::env::args().nth(1) {
        Some(thu_muc) => {
            #[cfg(windows)]
            // Ứng dụng cửa sổ không có console riêng: gắn vào console đã gọi lệnh để in được kết quả
            unsafe {
                use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
            if thu_muc == "--tach" {
                std::process::exit(chay_tach(&std::env::args().skip(2).collect::<Vec<_>>()));
            }
            let khong_ghi = std::env::args().any(|a| a == "--khong-ghi");
            let kiem_tra_dpi = std::env::args().any(|a| a == "--dpi");
            let kiem_tra_ky = std::env::args().any(|a| a == "--kyso");
            std::process::exit(chay_dong_lenh(&thu_muc, kiem_tra_dpi, kiem_tra_ky, khong_ghi));
        }
        None => {
            if let Err(e) = gui::chay() {
                eprintln!("Không mở được giao diện: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn chay_dong_lenh(thu_muc: &str, kiem_tra_dpi: bool, kiem_tra_ky: bool, khong_ghi: bool) -> i32 {
    let bat_dau = std::time::Instant::now();
    let (gui, nhan) = mpsc::channel();
    dem_pdf::dem_thu_muc(thu_muc, cau_hinh::doc().quy_tac(), kiem_tra_dpi, kiem_tra_ky, &AtomicBool::new(false), &gui);
    drop(gui);

    let mut ket_qua = None;
    for su_kien in nhan {
        match su_kien {
            dem_pdf::SuKien::Dong(s) if s.starts_with("Lỗi mở tệp") => eprintln!("{s}"),
            dem_pdf::SuKien::Xong(kq) => ket_qua = Some(kq),
            _ => {}
        }
    }
    let Some(kq) = ket_qua else {
        eprintln!("Không có kết quả đếm");
        return 1;
    };
    print!("{}", kq.tong_hop);
    if khong_ghi {
        return 0;
    }
    let duong_dan = dem_pdf::duong_dan_luu_mac_dinh(thu_muc);
    match dem_pdf::luu_ket_qua(&kq, &duong_dan) {
        Ok(chi_tiet) => {
            println!("Kết quả đếm ghi tại file:\n {}\n {}", duong_dan.display(), chi_tiet.display());
            if let Some(cau_hinh) = telegram::doc_cau_hinh() {
                let tin = telegram::noi_dung_thong_ke(thu_muc, &kq, bat_dau.elapsed());
                match telegram::gui_tin(&cau_hinh, &tin) {
                    Ok(()) => println!("Đã gửi thống kê về Telegram"),
                    Err(e) => eprintln!("Không gửi được thống kê: {e}"),
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Lỗi khi ghi file {}: {e}", duong_dan.display());
            1
        }
    }
}

/// `--tach <thư mục pdf> [--nhan <file>] [--ra <thư mục cha>]`: cắt theo nhãn có sẵn (không phân tích).
/// Nhãn mặc định `index_and_blank.txt`, góc xoay `xoay_trang.txt` (nếu có) trong thư mục dữ liệu ẩn của `<thư mục>`.
fn chay_tach(tham_so: &[String]) -> i32 {
    use std::path::{Path, PathBuf};
    use tach_vb::{nhan, tach};
    let mut thu_muc: Option<PathBuf> = None;
    let (mut file_nhan, mut cha_ra): (Option<PathBuf>, Option<PathBuf>) = (None, None);
    let mut i = 0;
    while i < tham_so.len() {
        match tham_so[i].as_str() {
            "--nhan" => { file_nhan = tham_so.get(i + 1).map(PathBuf::from); i += 1; }
            "--ra" => { cha_ra = tham_so.get(i + 1).map(PathBuf::from); i += 1; }
            khac => thu_muc = Some(PathBuf::from(khac)),
        }
        i += 1;
    }
    let Some(thu_muc) = thu_muc.filter(|p| p.is_dir()) else {
        eprintln!("Cách dùng: sm-dema4 --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục cha>]\nThiếu thư mục PDF hoặc thư mục không tồn tại.");
        return 2;
    };
    let thu_muc = std::fs::canonicalize(&thu_muc).unwrap_or(thu_muc);
    tach_vb::du_lieu_an::chuyen_du_lieu_cu(&thu_muc);
    let an = tach_vb::phien::thu_muc_an(&thu_muc);
    let co_nhan_rieng = file_nhan.is_some();
    let file_nhan = file_nhan.unwrap_or_else(|| an.join("index_and_blank.txt"));
    if !co_nhan_rieng && !file_nhan.exists() {
        // Chưa có nhãn: phân tích tự động (Cấu hình mặc định) rồi ghi index_and_blank.txt để kiểm tra
        println!("Chưa có {} — phân tích tự động…", file_nhan.display());
        let ch = tach_vb::phan_tich::CauHinhPhanTich::default();
        let phien = tach_vb::phien::phan_tich_thu_muc(&thu_muc, &ch, &|xong, tong| {
            if xong % 500 == 0 || xong == tong { eprintln!("  {xong}/{tong} trang"); }
        });
        if let Err(e) = std::fs::create_dir_all(&an).and_then(|_| std::fs::write(&file_nhan, nhan::ghi_file_nhan(&phien.bang_nhan_tu_dong(&ch)))) {
            eprintln!("Không ghi được {}: {e}", file_nhan.display());
            return 1;
        }
    }
    let Ok(noi_dung) = std::fs::read_to_string(&file_nhan) else {
        eprintln!("Không đọc được file nhãn {}", file_nhan.display());
        return 2;
    };
    let goc_nhan = if file_nhan.starts_with(&an) { thu_muc.clone() } else { file_nhan.parent().map(Path::to_path_buf).unwrap_or_else(|| thu_muc.clone()) };
    let bang_nhan = match nhan::doc_file_nhan(&noi_dung, &|_| None) {
        Ok((b, cb)) => { cb.iter().for_each(|c| eprintln!("Cảnh báo: {c}")); b }
        Err(loi) => { loi.iter().for_each(|l| eprintln!("{}: dòng {}: {}", file_nhan.display(), l.dong, l.ly_do)); return 1; }
    };
    let file_xoay = an.join("xoay_trang.txt");
    let bang_xoay = match std::fs::read_to_string(&file_xoay) {
        Err(_) => Default::default(),
        Ok(nd) => match nhan::doc_file_xoay(&nd, &|_| None) {
            Ok(b) => b,
            Err(loi) => { loi.iter().for_each(|l| eprintln!("{}: dòng {}: {}", file_xoay.display(), l.dong, l.ly_do)); return 1; }
        },
    };
    let (gui, _nhan_su_kien) = mpsc::channel();
    let pdf = dem_pdf::tim_file_pdf(&thu_muc.to_string_lossy(), &AtomicBool::new(false), &gui);
    let (ho_so, canh_bao) = tach::ghep_ho_so(&pdf, bang_nhan, &goc_nhan, bang_xoay, &thu_muc);
    canh_bao.iter().for_each(|c| eprintln!("Cảnh báo: {c}"));
    let ra = match tach::thu_muc_ra(&thu_muc, cha_ra.as_deref()) {
        Ok(r) => r,
        Err(e) => { eprintln!("{e}"); return 2; }
    };
    let bat_dau = std::time::Instant::now();
    let kq = match tach::tach_thu_muc(&thu_muc, &ra, &ho_so, tach::MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_| {}) {
        Ok(k) => k,
        Err(e) => { eprintln!("{e}"); return 2; }
    };
    kq.canh_bao.iter().for_each(|c| eprintln!("Cảnh báo: {c}"));
    kq.loi.iter().for_each(|(p, e)| eprintln!("Lỗi {}: {e}", p.display()));
    kq.khong_xoa.iter().for_each(|p| eprintln!("Không xoá được file cũ (có thể đang mở): {}", p.display()));
    let log = an.join("ketqua_tach.txt");
    let noi_dung = tach::noi_dung_log(&kq.dong) + &tach::dong_log_khong_xoa(&kq.khong_xoa);
    if let Err(e) = std::fs::create_dir_all(&an).and_then(|_| std::fs::write(&log, noi_dung)) {
        eprintln!("Không ghi được {}: {e}", log.display());
        return 1;
    }
    println!("Tách {} hồ sơ → {} văn bản ({} hồ sơ chép nguyên) trong {:.1} s\nKết quả: {}\nLog: {}", ho_so.len(), kq.dong.len(), kq.sao_nguyen, bat_dau.elapsed().as_secs_f64(), ra.display(), log.display());
    if kq.loi.is_empty() { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::chay_tach;

    #[test]
    fn tach_thieu_thu_muc_bao_loi() {
        assert_eq!(chay_tach(&[]), 2);
        assert_eq!(chay_tach(&["/khong/co/thu/muc/nay".to_string()]), 2);
    }

    #[test]
    fn tach_ap_goc_xoay_va_ghi_log() {
        let goc = std::env::temp_dir().join(format!("dema4_cli_tach_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("ho so");
        std::fs::create_dir_all(nguon.join(".sm-dema4")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 4);
        std::fs::write(nguon.join(".sm-dema4/index_and_blank.txt"), "a.pdf;2;index\n").unwrap();
        std::fs::write(nguon.join(".sm-dema4/xoay_trang.txt"), "a.pdf;4;180\n").unwrap();
        // Chỉ file nhãn không tồn tại → mã thoát 2
        assert_eq!(chay_tach(&[nguon.to_string_lossy().into_owned(), "--nhan".into(), "/khong/co.txt".into()]), 2);
        assert_eq!(chay_tach(&[nguon.to_string_lossy().into_owned()]), 0);
        let ra = goc.join("ho so_vanban").join("a");
        let doc = mupdf::pdf::PdfDocument::open(ra.join("a.2.pdf").to_str().unwrap()).unwrap();
        assert_eq!(doc.page_count().unwrap(), 2);
        assert_eq!(doc.find_page(1).unwrap().get_dict("Rotate").unwrap().unwrap().as_int().unwrap(), 180);
        // Nhãn cũ trong thư mục đầu vào đã được chuyển sang thư mục dữ liệu cạnh app
        assert!(!nguon.join(".sm-dema4").exists());
        let an = crate::tach_vb::phien::thu_muc_an(&nguon);
        let log = std::fs::read_to_string(an.join("ketqua_tach.txt")).unwrap();
        let _ = std::fs::remove_dir_all(&an);
        assert_eq!(log.lines().count(), 3);
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn tach_chua_co_nhan_thi_phan_tich_tu_dong() {
        let goc = std::env::temp_dir().join(format!("dema4_cli_tu_dong_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("b.pdf"), 3);
        assert_eq!(chay_tach(&[nguon.to_string_lossy().into_owned()]), 0);
        let an = crate::tach_vb::phien::thu_muc_an(&nguon);
        let nhan = std::fs::read_to_string(an.join("index_and_blank.txt")).unwrap();
        assert_eq!(nhan.lines().count(), 3, "3 trang chữ nhỏ → blank: {nhan}");
        assert!(an.join("cache").is_dir());
        assert!(!nguon.join(".sm-dema4").exists(), "không ghi gì vào thư mục đầu vào");
        let _ = std::fs::remove_dir_all(&goc);
    }
}
