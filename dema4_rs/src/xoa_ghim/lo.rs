//! Chạy Xóa ghim cả thư mục: id nội dung các PDF (bảng đường dẫn kho Tách), vùng chung + vùng riêng từng PDF,
//! song song theo file trên pool riêng; mỗi file xong báo 1 lần. Đọc trạng thái từng file (nhật ký + id file kết
//! quả đang có) để màn biết file nào đã xoá / chưa / kết quả mất.
//!
//! Cặp (nguồn, đích) có đích trùng nguồn = ghi đè: nhật ký / vùng riêng theo id nguồn gốc (`xuat::id_goc`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use rayon::prelude::*;

use super::kho::{self as kho_xg, CauHinhXuat, NhatKy};
use super::vung::Vung;
use super::xu_ly::TuyChon;
use super::xuat::{self, KetQuaFile, TuyChonChay};
use crate::tach_vb::kho::{ids_cua_trong, Id, Kho};

/// Lỗi của file bị bỏ vì người dùng bấm Dừng (không tính là lỗi file).
pub const DA_DUNG: &str = "đã dừng";

/// Xoá ghim các cặp (nguồn, đích) `viec`; `bao(i, kết quả)` gọi từ luồng xử lý khi xong file thứ `i`.
/// Bấm Dừng: file đang làm dừng ở trang kế, file chưa làm báo `DA_DUNG`.
#[allow(clippy::too_many_arguments)]
pub fn chay(
    kho_tach: &Mutex<Kho>,
    kho: &Mutex<Kho>,
    viec: &[(PathBuf, PathBuf)],
    chung: &[Vung],
    tc: &TuyChon,
    xuat_ch: &CauHinhXuat,
    dung: &AtomicBool,
    bao: impl Fn(usize, Result<KetQuaFile, String>) + Sync,
) {
    chay_voi(kho_tach, kho, viec, chung, tc, xuat_ch, TuyChonChay::default(), dung, bao)
}

/// Như `chay`, theo `chay_ch` (Lô 10 trang: các trang của 1 file chạy song song trên cùng pool).
#[allow(clippy::too_many_arguments)]
pub fn chay_voi(
    kho_tach: &Mutex<Kho>,
    kho: &Mutex<Kho>,
    viec: &[(PathBuf, PathBuf)],
    chung: &[Vung],
    tc: &TuyChon,
    xuat_ch: &CauHinhXuat,
    chay_ch: TuyChonChay,
    dung: &AtomicBool,
    bao: impl Fn(usize, Result<KetQuaFile, String>) + Sync,
) {
    let ids = ids_cua_trong(kho_tach, &viec.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>());
    let bang = kho_xg::nguon_theo_ket_qua(&mut kho.lock().unwrap_or_else(|e| e.into_inner()));
    xuat::pool().install(|| {
        viec.par_iter().enumerate().for_each(|(i, (nguon, dich))| {
            if dung.load(Ordering::Relaxed) {
                bao(i, Err(DA_DUNG.into()));
                return;
            }
            let Some(id) = ids[i] else {
                bao(i, Err("không đọc được file".into()));
                return;
            };
            let id_vung = if nguon == dich { xuat::id_goc(&bang, id) } else { id };
            let mut vung = chung.to_vec();
            if let Some(vf) = kho_xg::doc_vung_file(&mut kho.lock().unwrap_or_else(|e| e.into_inner()), id_vung) {
                vung.extend(vf.vung);
            }
            bao(i, xuat::xoa_ghim_file_voi(kho, &bang, nguon, id, dich, &vung, tc, xuat_ch, chay_ch, dung));
        });
    });
}

/// Phần đã biết của 1 file để tính trạng thái (mã cấu hình tính lại theo vùng / tuỳ chọn hiện tại).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TinhTrangFile {
    pub nhat_ky: Option<NhatKy>,
    /// Id file đang có ở đường dẫn kết quả (`None` = không có).
    pub id_dich: Option<Id>,
    pub vung_rieng: Vec<Vung>,
}

/// Đọc trạng thái mọi file (đọc 64 KB đầu + cuối từng file kết quả — chạy ở luồng nền).
/// Cặp ghi đè (đích = nguồn): file hiện tại là kết quả đã biết → nhật ký của nguồn gốc (→ "đã xoá"), không thì không
/// có nhật ký (→ "chưa xoá"); `id_dich` = id file hiện tại; vùng riêng theo id nguồn gốc.
pub fn doc_tinh_trang(kho_tach: &Mutex<Kho>, kho: &Mutex<Kho>, viec: &[(PathBuf, PathBuf)]) -> Vec<TinhTrangFile> {
    let ids = ids_cua_trong(kho_tach, &viec.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>());
    let id_dich: Vec<Option<Id>> = viec.par_iter().zip(ids.par_iter()).map(|((n, d), id)| if n == d { *id } else { xuat::id_file(d) }).collect();
    let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
    let bang = if viec.iter().any(|(n, d)| n == d) { kho_xg::nguon_theo_ket_qua(&mut k) } else { Default::default() };
    viec.iter()
        .zip(ids.iter().zip(id_dich))
        .map(|((n, d), (id, id_dich))| match id {
            Some(id) => {
                let (id_nk, id_vung) = if n == d { (bang.get(id).copied(), xuat::id_goc(&bang, *id)) } else { (Some(*id), *id) };
                TinhTrangFile {
                    nhat_ky: id_nk.and_then(|i| kho_xg::doc_nhat_ky(&mut k, i)),
                    id_dich,
                    vung_rieng: kho_xg::doc_vung_file(&mut k, id_vung).map(|v| v.vung).unwrap_or_default(),
                }
            }
            None => TinhTrangFile { id_dich, ..Default::default() },
        })
        .collect()
}

/// Trạng thái 1 file theo vùng chung + tuỳ chọn hiện tại.
pub fn trang_thai(tt: &TinhTrangFile, chung: &[Vung], tc: &TuyChon, xuat_ch: &CauHinhXuat) -> xuat::TrangThai {
    let vung: Vec<Vung> = chung.iter().chain(&tt.vung_rieng).cloned().collect();
    xuat::trang_thai(tt.nhat_ky.as_ref(), tt.id_dich, kho_xg::ma_cau_hinh(&vung, tc, xuat_ch))
}

#[cfg(test)]
mod tests {
    use super::super::vung::{co_san, ViTri};
    use super::*;
    use xuat::TrangThai;

    #[test]
    fn chay_thu_muc_vung_rieng_trang_thai_va_dung() {
        let d = std::env::temp_dir().join(format!("dema4_lo_xg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let nguon = d.join("Nguon");
        std::fs::create_dir_all(nguon.join("con")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 2);
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("con/b.pdf"), 1);
        let (kt, kx) = (Mutex::new(Kho::mo(&d.join("tach.bin")).unwrap()), Mutex::new(Kho::mo(&d.join("xg.bin")).unwrap()));
        let ra = d.join("Nguon_xoaghim");
        let viec = xuat::ds_viec(&nguon, &ra, &AtomicBool::new(false));
        let chung = vec![co_san(ViTri::GocTrenTrai)];
        let (tc, xc) = (TuyChon::default(), CauHinhXuat::default());
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert!(tt.iter().all(|t| trang_thai(t, &chung, &tc, &xc) == TrangThai::ChuaXoa));
        let kq = Mutex::new(Vec::new());
        chay(&kt, &kx, &viec, &chung, &tc, &xc, &AtomicBool::new(false), |i, r| kq.lock().unwrap().push((i, r.map(|k| k.so_trang))));
        let mut kq = kq.into_inner().unwrap();
        kq.sort_by_key(|x| x.0);
        assert_eq!(kq, vec![(0, Ok(2)), (1, Ok(1))]);
        assert!(ra.join("a.pdf").exists() && ra.join("con/b.pdf").exists(), "giữ thư mục con, giữ tên");
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert!(tt.iter().all(|t| trang_thai(t, &chung, &tc, &xc) == TrangThai::DaXoa { dung_cau_hinh: true }));
        let khac = vec![co_san(ViTri::GocTrenPhai)];
        assert_eq!(trang_thai(&tt[0], &khac, &tc, &xc), TrangThai::DaXoa { dung_cau_hinh: false }, "đổi vùng → cấu hình đã đổi");
        std::fs::remove_dir_all(ra.join("con")).unwrap();
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert_eq!(trang_thai(&tt[1], &chung, &tc, &xc), TrangThai::KetQuaMat);
        // Dừng trước khi chạy: không file nào ghi, không sót file tạm
        let n = Mutex::new(0);
        chay(&kt, &kx, &viec, &chung, &tc, &xc, &AtomicBool::new(true), |_, r| {
            assert_eq!(r.unwrap_err(), DA_DUNG);
            *n.lock().unwrap() += 1;
        });
        assert_eq!(*n.lock().unwrap(), 2);
        assert!(!ra.join("con").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ghi_de_trang_thai_chi_da_xoa_hoac_chua_xoa_vung_rieng_theo_nguon_goc() {
        let d = std::env::temp_dir().join(format!("dema4_lo_xg_de_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let nguon = d.join("Nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 2);
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("b.pdf"), 1);
        let (kt, kx) = (Mutex::new(Kho::mo(&d.join("tach.bin")).unwrap()), Mutex::new(Kho::mo(&d.join("xg.bin")).unwrap()));
        let ghi = kho_xg::CauHinhGhi { ghi_de: true, ..Default::default() };
        let viec = xuat::ds_viec_ghi(&nguon, &ghi, &AtomicBool::new(false)).unwrap();
        assert!(viec.iter().all(|(n, d)| n == d));
        let chung = vec![co_san(ViTri::GocTrenTrai)];
        let (tc, xc) = (TuyChon::default(), CauHinhXuat::default());
        // Vùng riêng của a theo id trước khi ghi đè
        let id_a = ids_cua_trong(&kt, &[viec[0].0.clone()])[0].unwrap();
        let rieng = vec![co_san(ViTri::GocDuoiPhai)];
        kho_xg::ghi_vung_file(&mut kx.lock().unwrap(), id_a, &kho_xg::VungFile { luc: 1, vung: rieng.clone() });
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert!(tt.iter().all(|t| trang_thai(t, &chung, &tc, &xc) == TrangThai::ChuaXoa));
        let kq = Mutex::new(Vec::new());
        chay_voi(&kt, &kx, &viec[..1], &chung, &tc, &xc, TuyChonChay { lo_10: true }, &AtomicBool::new(false), |_, r| kq.lock().unwrap().push(r.map(|k| k.trang_xu_ly)));
        assert_eq!(kq.into_inner().unwrap(), vec![Ok(vec![0, 1])], "vùng riêng góc dưới phải áp mọi trang");
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert_eq!(tt[0].vung_rieng, rieng, "vùng riêng theo nguồn gốc");
        assert_eq!(trang_thai(&tt[0], &chung, &tc, &xc), TrangThai::DaXoa { dung_cau_hinh: true });
        assert_eq!(trang_thai(&tt[0], &[co_san(ViTri::GocTrenPhai)], &tc, &xc), TrangThai::DaXoa { dung_cau_hinh: false });
        assert_eq!(trang_thai(&tt[1], &chung, &tc, &xc), TrangThai::ChuaXoa);
        // File bị thay bằng nội dung khác (không phải kết quả đã biết) → chưa xoá, không "kết quả mất"
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 3);
        let tt = doc_tinh_trang(&kt, &kx, &viec);
        assert_eq!(trang_thai(&tt[0], &chung, &tc, &xc), TrangThai::ChuaXoa);
        let _ = std::fs::remove_dir_all(&d);
    }
}
