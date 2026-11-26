// Trên Windows chạy như ứng dụng cửa sổ (không bật cửa sổ đen console)
#![cfg_attr(windows, windows_subsystem = "windows")]

//! Đếm số trang A4 trong thư mục PDF.
//! - Không tham số: mở giao diện.
//! - `dema4_rs <thư mục>`: đếm không cần giao diện, ghi <thư mục>/<tên>_dem.txt và <tên>_dem_chitiet.txt.
//! - Thêm `--dpi` để kiểm tra DPI ảnh scan, `--kyso` để kiểm tra chữ ký số (mặc định tắt).
//! - `dema4_rs <thư mục> --khong-ghi`: chỉ đếm và in kết quả (không ghi file, không gửi thống kê), dùng để đo tốc độ.
//! - `dema4_rs --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục cha>]`: tách văn bản theo nhãn,
//!   ghi `<cha>/<tên>_vanban/…` và `ketqua_tach.txt` trong thư mục dữ liệu làm dở của thư mục pdf (thư mục người dùng của app); chưa có nhãn thì phân tích tự động trước.

#[allow(dead_code)] // nối dần vào màn Bóc tách
mod boc_tach;
#[allow(dead_code)] // nối dần vào màn Đếm / Tách
mod cau_hinh;
mod dem_pdf;
mod ghi_file;
mod gioi_thieu;
mod license;
mod logo_chuc_nang;
mod logo_sao_mai;
mod muc_luc;
mod phien_lam_viec;
mod tram;
mod hop_license;
mod gui;
mod gan_day;
mod gui_muc_luc;
mod gui_phien;
mod gui_boc_tach;
mod gui_cau_hinh;
#[allow(dead_code)] // nối dần vào giao diện
mod gui_tach;
mod gui_xoa_ghim;
#[allow(dead_code)] // dùng dần từ các phần phân tích / giao diện tách văn bản
mod tach_vb;
mod telegram;
mod tro_giup;
mod trang_chu;
#[allow(dead_code)] // nối dần vào màn Xóa ghim
mod xoa_ghim;

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
            let kt = license::KiemTra::mac_dinh();
            kt.mo_app();
            let ma = if thu_muc == "--tach" && !tach_vb::kho::giu_khoa_ung_dung() {
                eprintln!("Ứng dụng đang mở (hoặc lệnh --tach khác đang chạy) — đóng ứng dụng rồi chạy lại lệnh.");
                4
            } else if thu_muc == "--tach" {
                chay_tach(&kt, &std::env::args().skip(2).collect::<Vec<_>>())
            } else {
                let khong_ghi = std::env::args().any(|a| a == "--khong-ghi");
                let kiem_tra_dpi = std::env::args().any(|a| a == "--dpi");
                let kiem_tra_ky = std::env::args().any(|a| a == "--kyso");
                chay_dong_lenh(&kt, &thu_muc, kiem_tra_dpi, kiem_tra_ky, khong_ghi)
            };
            // Chờ luồng gửi ghi lượt / thống kê lên trạm trước khi thoát (Ctrl+C: đặt chỗ còn trên đĩa, lần sau tính)
            let _ = kt.moc().recv_timeout(std::time::Duration::from_secs(20));
            std::process::exit(ma);
        }
        None => {
            // Chỉ 1 cửa sổ app (1 tiến trình ghi kho dữ liệu)
            if !tach_vb::kho::giu_khoa_ung_dung() {
                rfd::MessageDialog::new().set_title("Sao Mai").set_description("Ứng dụng đang mở ở cửa sổ khác.").set_level(rfd::MessageLevel::Info).show();
                return;
            }
            // Dọn kho (bỏ bản ghi cũ đã bị thay) khi cần — ở luồng nền, không chậm lúc mở
            std::thread::spawn(|| {
                for kho in [tach_vb::kho::chung(), xoa_ghim::kho::chung()] {
                    let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
                    if k.nen_don() {
                        let _ = k.nen();
                    }
                }
            });
            if let Err(e) = gui::chay() {
                let chu = format!("Không mở được giao diện: {e}\n\nThử cập nhật driver card đồ hoạ, hoặc chạy với biến môi trường WGPU_BACKEND=gl.");
                eprintln!("{chu}");
                hop_loi_windows(&chu);
                std::process::exit(1);
            }
        }
    }
}

/// Lỗi mở giao diện: app cửa sổ trên Windows không có console nên `eprintln!` không ai thấy → hiện hộp thông báo.
#[cfg(windows)]
fn hop_loi_windows(chu: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (noi_dung, tieu_de) = (w(chu), w("dema4 – Sao Mai"));
    // SAFETY: 2 chuỗi UTF-16 kết thúc bằng 0, sống tới hết lời gọi; không có cửa sổ cha
    unsafe {
        MessageBoxW(std::ptr::null_mut(), noi_dung.as_ptr(), tieu_de.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn hop_loi_windows(_chu: &str) {}

/// Hết lượt: in lý do + cách liên hệ + mã máy, mã thoát 3.
fn bat_dau_luot(kt: &license::KiemTra, cn: license::ChucNang) -> Result<license::LanChay, i32> {
    kt.bat_dau(cn).map_err(|qd| {
        // Hạn mức dùng chung mọi chức năng
        if matches!(qd, license::QuyetDinh::CanMangLanDau(_)) {
            eprintln!("{} Mã máy: {}", hop_license::LY_DO_CAN_MANG_DONG_LENH, kt.ma_may());
            return 3;
        }
        eprintln!("Số lần dùng còn lại (0). Liên hệ với Sao Mai để được hỗ trợ: {} · {} · Mã máy: {}", hop_license::DIEN_THOAI, hop_license::EMAIL, kt.ma_may());
        if matches!(qd, license::QuyetDinh::HetLuotOffline(_)) {
            eprintln!("Chưa kết nối được máy chủ — kết nối mạng rồi chạy lại.");
        }
        3
    })
}

fn chay_dong_lenh(kt: &license::KiemTra, thu_muc: &str, kiem_tra_dpi: bool, kiem_tra_ky: bool, khong_ghi: bool) -> i32 {
    let lan = match bat_dau_luot(kt, license::ChucNang::Dem) {
        Ok(l) => l,
        Err(ma) => return ma,
    };
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
    // Có kết quả là tính lượt (cả khi chỉ in, --khong-ghi); không đếm được file nào → hoàn lượt
    kt.ket_thuc(&lan, ket_qua.as_ref().is_some_and(|k| !k.dong_tong_hop.is_empty()));
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
            let so_lieu = tram::so_lieu_dem(thu_muc, &kq, bat_dau.elapsed());
            kt.thong_ke("thong_ke_dem", move |_| so_lieu);
            0
        }
        Err(e) => {
            eprintln!("Lỗi khi ghi file {}: {e}", duong_dan.display());
            1
        }
    }
}

/// `--tach <thư mục pdf> [--nhan <file>] [--xuat-nhan <file>] [--ra <thư mục cha>]`: cắt theo nhãn có sẵn.
/// Nhãn + góc xoay lấy từ kho theo nội dung từng PDF (chưa có gì → phân tích tự động, ghi vào kho); `--nhan` = đọc
/// file nhãn dạng chữ ngoài; `--xuat-nhan` = ghi nhãn trong kho ra file chữ rồi dừng (không tách).
fn chay_tach(kt: &license::KiemTra, tham_so: &[String]) -> i32 {
    use std::path::{Path, PathBuf};
    use tach_vb::{kho, nhan, tach};
    let mut thu_muc: Option<PathBuf> = None;
    let (mut file_nhan, mut xuat_nhan, mut cha_ra): (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) = (None, None, None);
    let mut i = 0;
    while i < tham_so.len() {
        match tham_so[i].as_str() {
            "--nhan" => { file_nhan = tham_so.get(i + 1).map(PathBuf::from); i += 1; }
            "--xuat-nhan" => { xuat_nhan = tham_so.get(i + 1).map(PathBuf::from); i += 1; }
            "--ra" => { cha_ra = tham_so.get(i + 1).map(PathBuf::from); i += 1; }
            khac => thu_muc = Some(PathBuf::from(khac)),
        }
        i += 1;
    }
    let Some(thu_muc) = thu_muc.filter(|p| p.is_dir()) else {
        eprintln!("Cách dùng: dema4_saomai --tach <thư mục pdf> [--nhan <file nhãn>] [--xuat-nhan <file>] [--ra <thư mục cha>]\nThiếu thư mục PDF hoặc thư mục không tồn tại.");
        return 2;
    };
    let thu_muc = std::fs::canonicalize(&thu_muc).unwrap_or(thu_muc);
    tach_vb::du_lieu_an::chuyen_du_lieu_cu(&thu_muc);
    let an = tach_vb::phien::thu_muc_an(&thu_muc);
    let (gui, _nhan_su_kien) = mpsc::channel();
    let pdf = dem_pdf::tim_file_pdf(&thu_muc.to_string_lossy(), &AtomicBool::new(false), &gui);
    let (ho_so, canh_bao) = if let Some(file_nhan) = &file_nhan {
        // File nhãn dạng chữ ngoài: đường dẫn trong file tương đối với thư mục chứa file
        let Ok(noi_dung) = std::fs::read_to_string(file_nhan) else {
            eprintln!("Không đọc được file nhãn {}", file_nhan.display());
            return 2;
        };
        let bang_nhan = match nhan::doc_file_nhan(&noi_dung, &|_| None) {
            Ok((b, cb)) => { cb.iter().for_each(|c| eprintln!("Cảnh báo: {c}")); b }
            Err(loi) => { loi.iter().for_each(|l| eprintln!("{}: dòng {}: {}", file_nhan.display(), l.dong, l.ly_do)); return 1; }
        };
        let goc_nhan = file_nhan.parent().map(Path::to_path_buf).unwrap_or_else(|| thu_muc.clone());
        tach::ghep_ho_so(&pdf, bang_nhan, &goc_nhan, Default::default(), &thu_muc)
    } else {
        let ids = kho::ids_cua(&pdf);
        tach_vb::chuyen_kho::chuyen(&thu_muc, &pdf, &ids);
        let co_nhan = { let mut k = kho::kho(); ids.iter().flatten().any(|id| k.tach(*id).is_some_and(|b| !b.bia_mo)) };
        if !co_nhan && xuat_nhan.is_none() {
            // Chưa có nhãn: phân tích tự động (Cấu hình mặc định), ghi nhãn tự động vào kho
            println!("Chưa có nhãn — phân tích tự động…");
            let ch = tach_vb::phan_tich::CauHinhPhanTich::default();
            let phien = tach_vb::phien::phan_tich_thu_muc(&thu_muc, &ch, &|xong, tong| {
                if xong % 500 == 0 || xong == tong { eprintln!("  {xong}/{tong} trang"); }
            });
            let bang = phien.bang_nhan_tu_dong(&ch);
            let bay_gio = tach_vb::moc_sua::bay_gio_ms();
            let mut k = kho::kho();
            for h in &phien.ho_so {
                let (Some(id), None) = (h.id, &h.loi) else { continue };
                let n = bang.get(&h.tuong_doi).cloned().unwrap_or_default();
                let chu_ky = kho::chu_ky_tach(&n, &Default::default(), &Default::default());
                k.them_tach(id, &kho::BanTach { so_trang: h.trang.len() as u32, nhan: n, luc: bay_gio, chu_ky, ..Default::default() });
            }
            if let Err(e) = k.day() {
                eprintln!("Không ghi được nhãn vào kho: {e}");
                return 1;
            }
        }
        let mut k = kho::kho();
        let ho_so: Vec<tach::HoSoTach> = pdf
            .iter()
            .zip(&ids)
            .map(|(p, id)| {
                let bt = id.and_then(|id| k.tach(id)).filter(|b| !b.bia_mo).unwrap_or_default();
                tach::HoSoTach { pdf: p.clone(), nhan: bt.nhan, xoay: bt.xoay.into_iter().filter(|(_, x)| x.goc != 0).collect(), id: *id }
            })
            .collect();
        (ho_so, Vec::new())
    };
    if let Some(f) = &xuat_nhan {
        let bang: nhan::BangNhan = ho_so.iter().filter(|h| !h.nhan.is_empty()).map(|h| (nhan::duong_dan_ghi(&thu_muc, &h.pdf), h.nhan.clone())).collect();
        return match std::fs::write(f, nhan::ghi_file_nhan(&bang)) {
            Ok(()) => { println!("Đã ghi nhãn {} hồ sơ ra {}", bang.len(), f.display()); 0 }
            Err(e) => { eprintln!("Không ghi được {}: {e}", f.display()); 1 }
        };
    }
    canh_bao.iter().for_each(|c| eprintln!("Cảnh báo: {c}"));
    let ra = match tach::thu_muc_ra(&thu_muc, cha_ra.as_deref()) {
        Ok(r) => r,
        Err(e) => { eprintln!("{e}"); return 2; }
    };
    let lan = match bat_dau_luot(kt, license::ChucNang::Tach) {
        Ok(l) => l,
        Err(ma) => return ma,
    };
    let bat_dau = std::time::Instant::now();
    let kq = tach::tach_thu_muc(&thu_muc, &ra, &ho_so, tach::MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_| {});
    kt.ket_thuc(&lan, kq.as_ref().is_ok_and(|k| !k.dong.is_empty()));
    let kq = match kq {
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
    use super::{chay_dong_lenh, chay_tach};
    use crate::license::KiemTra;
    use crate::tram::KhoGia;
    use std::sync::Arc;

    fn kt() -> Arc<KiemTra> {
        KiemTra::mac_dinh()
    }

    /// Hạn mức đã biết từ trạm (giả): mở app → đọc trạng thái ký.
    fn kt_han(han: u32) -> Arc<KiemTra> {
        let kt = KiemTra::gia(Arc::new(KhoGia::moi(han)));
        kt.mo_app();
        kt.moc().recv().unwrap();
        kt
    }

    #[test]
    fn dem_khong_ghi_van_tinh_luot() {
        let goc = std::env::temp_dir().join(format!("dema4_cli_luot_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(&goc).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("a.pdf"), 1);
        let kt = kt_han(2);
        let tm = goc.to_string_lossy().into_owned();
        assert_eq!(chay_dong_lenh(&kt, &tm, false, false, true), 0);
        assert_eq!(chay_dong_lenh(&kt, &tm, false, false, true), 0);
        assert_eq!(chay_dong_lenh(&kt, &tm, false, false, true), 3, "hạn 2 → lần 3 bị chặn");
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn tach_het_luot_khong_ghi_ket_qua() {
        let goc = std::env::temp_dir().join(format!("dema4_cli_het_luot_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("c.pdf"), 2);
        assert_eq!(chay_tach(&kt_han(0), &[nguon.to_string_lossy().into_owned()]), 3);
        assert!(!goc.join("nguon_vanban").exists(), "hết lượt: không tạo thư mục kết quả");
        let _ = std::fs::remove_dir_all(crate::tach_vb::phien::thu_muc_an(&nguon));
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn tach_thieu_thu_muc_bao_loi() {
        assert_eq!(chay_tach(&kt(), &[]), 2);
        assert_eq!(chay_tach(&kt(), &["/khong/co/thu/muc/nay".to_string()]), 2);
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
        assert_eq!(chay_tach(&kt(), &[nguon.to_string_lossy().into_owned(), "--nhan".into(), "/khong/co.txt".into()]), 2);
        assert_eq!(chay_tach(&kt(), &[nguon.to_string_lossy().into_owned()]), 0);
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
        assert_eq!(chay_tach(&kt(), &[nguon.to_string_lossy().into_owned()]), 0);
        let id = crate::tach_vb::kho::ids_cua(&[nguon.join("b.pdf")])[0].unwrap();
        let mut k = crate::tach_vb::kho::kho();
        let bt = k.tach(id).expect("nhãn tự động ghi vào kho");
        assert_eq!(bt.nhan.len(), 3, "3 trang chữ nhỏ → blank: {bt:?}");
        assert!(bt.luc > 0 && k.phan_tich(id).is_some(), "có mốc + kết quả phân tích");
        drop(k);
        // Xuất nhãn trong kho ra file chữ
        let f = goc.join("nhan.txt");
        assert_eq!(chay_tach(&kt(), &[nguon.to_string_lossy().into_owned(), "--xuat-nhan".into(), f.to_string_lossy().into_owned()]), 0);
        assert_eq!(std::fs::read_to_string(&f).unwrap().lines().count(), 3);
        assert!(!nguon.join(".sm-dema4").exists(), "không ghi gì vào thư mục đầu vào");
        let _ = std::fs::remove_dir_all(&goc);
    }
}
