// Trên Windows chạy như ứng dụng cửa sổ (không bật cửa sổ đen console)
#![cfg_attr(windows, windows_subsystem = "windows")]

//! Đếm số trang A4 trong thư mục PDF.
//! - Không tham số: mở giao diện.
//! - `dema4_rs <thư mục>`: đếm không cần giao diện, ghi <thư mục>/<tên>_dem.txt và <tên>_dem_chitiet.txt.
//! - Thêm `--dpi` để kiểm tra DPI ảnh scan, `--kyso` để kiểm tra chữ ký số (mặc định tắt).
//! - `dema4_rs <thư mục> --khong-ghi`: chỉ đếm và in kết quả (không ghi file, không gửi thống kê), dùng để đo tốc độ.

mod dem_pdf;
mod gui;
mod telegram;

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
    dem_pdf::dem_thu_muc(thu_muc, dem_pdf::HE_SO_MAC_DINH, kiem_tra_dpi, kiem_tra_ky, &AtomicBool::new(false), &gui);
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
