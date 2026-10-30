//! Sinh hàng loạt: mỗi hồ sơ 1 file DOCX vào thư mục ra; chạy ở luồng nền, báo tiến độ qua kênh, dừng được giữa chừng.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::doc_bang::BangDuLieu;
use super::gom_nhom::HoSo;
use super::mau::MauDaBien;
use super::{ghep, ten_file, RangBuoc};

pub struct YeuCau {
    pub mau: Arc<MauDaBien>,
    pub bang: Arc<BangDuLieu>,
    pub ho_so: Arc<Vec<HoSo>>,
    pub rb: RangBuoc,
    pub thu_muc_ra: PathBuf,
    /// Mẫu tên file, vd `MLHS_{so_ho_so}`.
    pub mau_ten: String,
    /// File đã có: `true` ghi đè, `false` bỏ qua (không tính là lỗi).
    pub ghi_de: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuKien {
    /// Bắt đầu hồ sơ thứ `i` (0-based) với tên file.
    DangTao { i: usize, ten: String },
    Xong(TongKet),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TongKet {
    pub docx: usize,
    pub bo_qua: usize,
    /// (tên file, lỗi)
    pub loi: Vec<(String, String)>,
    pub da_dung: bool,
    pub thoi_gian: Duration,
}

/// Thư mục ra mặc định: `<dữ liệu ẩn cạnh exe>/muc_luc/<tên file dữ liệu không đuôi>`.
pub fn thu_muc_mac_dinh(goc: &Path, file_du_lieu: &Path) -> PathBuf {
    let ten = file_du_lieu.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "du_lieu".into());
    goc.join("muc_luc").join(ten)
}

/// Chạy đến hết hoặc tới khi `dung` bật; luôn gửi `Xong` cuối cùng.
pub fn chay(yc: &YeuCau, dung: &AtomicBool, gui: &Sender<SuKien>) -> TongKet {
    let bat_dau = Instant::now();
    let mut tk = TongKet::default();
    let ten = ten_file::dat_ten(&yc.mau_ten, &yc.ho_so);
    if let Err(e) = std::fs::create_dir_all(&yc.thu_muc_ra) {
        tk.loi.push((yc.thu_muc_ra.display().to_string(), format!("không tạo được thư mục: {e}")));
    } else {
        for (i, (hs, ten)) in yc.ho_so.iter().zip(&ten).enumerate() {
            if dung.load(Ordering::Relaxed) {
                tk.da_dung = true;
                break;
            }
            let file = format!("{ten}.docx");
            let _ = gui.send(SuKien::DangTao { i, ten: file.clone() });
            let dich = yc.thu_muc_ra.join(&file);
            if dich.exists() && !yc.ghi_de {
                tk.bo_qua += 1;
                continue;
            }
            match ghep::ghep(&yc.mau, &yc.bang, hs, &yc.rb).and_then(|b| std::fs::write(&dich, b).map_err(|e| e.to_string())) {
                Ok(()) => tk.docx += 1,
                Err(e) => tk.loi.push((file, e)),
            }
        }
    }
    tk.thoi_gian = bat_dau.elapsed();
    let _ = gui.send(SuKien::Xong(tk.clone()));
    tk
}

#[cfg(test)]
mod tests {
    use super::super::{doc_bang, khop, kiem_tra, mau};
    use super::*;

    fn yeu_cau(ra: &Path) -> YeuCau {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests_data/muc_luc/bien_muc_tong_hop.xlsx");
        let bang = doc_bang::doc(&p, Some("Bien muc"), 4).unwrap();
        let m = mau::bien_dich(mau::MAU_CO_SAN[0].1).unwrap();
        let rb = khop::tu_khop(&m, &bang.tieu_de);
        let ho_so = kiem_tra::kiem_tra(&m, &bang, &rb).ho_so;
        YeuCau { mau: Arc::new(m), bang: Arc::new(bang), ho_so: Arc::new(ho_so), rb, thu_muc_ra: ra.into(), mau_ten: ten_file::MAU_MAC_DINH.into(), ghi_de: false }
    }

    fn thu_muc(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_sinh_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn sinh_moi_ho_so_mot_docx_va_bo_qua_file_co_san() {
        let d = thu_muc("bo_qua");
        let yc = yeu_cau(&d);
        let n = yc.ho_so.len();
        assert!(n > 0);
        let (tx, rx) = std::sync::mpsc::channel();
        let tk = chay(&yc, &AtomicBool::new(false), &tx);
        assert_eq!((tk.docx, tk.bo_qua, tk.loi.len(), tk.da_dung), (n, 0, 0, false));
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), n);
        let su_kien: Vec<_> = rx.try_iter().collect();
        assert_eq!(su_kien.len(), n + 1, "mỗi hồ sơ 1 DangTao + 1 Xong");
        assert!(matches!(su_kien.last(), Some(SuKien::Xong(_))));
        let tk2 = chay(&yc, &AtomicBool::new(false), &tx);
        assert_eq!((tk2.docx, tk2.bo_qua), (0, n), "chạy lại, không ghi đè → bỏ qua");
        let tk3 = chay(&YeuCau { ghi_de: true, ..yc }, &AtomicBool::new(false), &tx);
        assert_eq!(tk3.docx, n);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn dung_giua_chung() {
        let d = thu_muc("dung");
        let (tx, _rx) = std::sync::mpsc::channel();
        let tk = chay(&yeu_cau(&d), &AtomicBool::new(true), &tx);
        assert_eq!((tk.docx, tk.da_dung), (0, true));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn thu_muc_ra_mac_dinh() {
        assert_eq!(thu_muc_mac_dinh(Path::new("/g"), Path::new("/a/Bien muc.xlsx")), Path::new("/g/muc_luc/Bien muc"));
    }
}
