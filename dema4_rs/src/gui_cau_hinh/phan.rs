//! Phần cấu hình mỗi chức năng sửa ở trang Cấu hình — để biết chức năng nào còn thay đổi chưa lưu (chấm cam).
//! Trường không thuộc chức năng nào là trạng thái giao diện các màn tự nhớ (độ rộng cột, cỡ ảnh, file gần nhất…).

use crate::cau_hinh::{CauHinh, CauHinhMucLuc, CauHinhXoaGhim};
use crate::gui_tach::hop_cau_hinh::TabCauHinh;

/// Bản cấu hình chỉ giữ các trường của `tab` (còn lại mặc định) — so 2 bản này để biết `tab` có đổi.
pub fn chi_phan(tab: TabCauHinh, c: &CauHinh) -> CauHinh {
    let md = CauHinh::default();
    match tab {
        TabCauHinh::Dem => CauHinh { he_so: c.he_so, ranh_gioi: c.ranh_gioi, a0_lon: c.a0_lon, a0_lon_he_so: c.a0_lon_he_so, a0_nguong_mm: c.a0_nguong_mm, ..md },
        TabCauHinh::Tach => CauHinh {
            quet_trang: c.quet_trang,
            quet_ma: c.quet_ma,
            quet_xoay: c.quet_xoay,
            nguong_trang: c.nguong_trang,
            nguong_nghi: c.nguong_nghi,
            loai_ma: c.loai_ma,
            ma_index: c.ma_index.clone(),
            tin_cay_xoay: c.tin_cay_xoay,
            thu_muc_ra: c.thu_muc_ra.clone(),
            mau_ten: c.mau_ten.clone(),
            luu_nhan: c.luu_nhan,
            ..md
        },
        TabCauHinh::MucLuc => {
            let m = &c.muc_luc;
            CauHinh { muc_luc: CauHinhMucLuc { thu_muc_ra: m.thu_muc_ra.clone(), mau_ten: m.mau_ten.clone(), ghi_de: m.ghi_de, ..Default::default() }, ..md }
        }
        TabCauHinh::BocTach => CauHinh { boc_tach: c.boc_tach.clone(), ..md },
        // Độ rộng danh sách do màn Xóa ghim tự nhớ
        TabCauHinh::XoaGhim => CauHinh { xoa_ghim: CauHinhXoaGhim { rong_ds: CauHinhXoaGhim::default().rong_ds, ..c.xoa_ghim.clone() }, ..md },
        TabCauHinh::DuLieu => md,
    }
}

/// `tab` có khác nhau giữa 2 bản cấu hình.
pub fn khac(tab: TabCauHinh, a: &CauHinh, b: &CauHinh) -> bool {
    chi_phan(tab, a) != chi_phan(tab, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOI_TAB: [TabCauHinh; 6] = [TabCauHinh::Dem, TabCauHinh::Tach, TabCauHinh::MucLuc, TabCauHinh::BocTach, TabCauHinh::XoaGhim, TabCauHinh::DuLieu];

    /// Đổi 1 trường của 1 chức năng → chỉ chức năng đó báo đổi; trường giao diện không làm chức năng nào đổi.
    #[test]
    fn doi_mot_truong_chi_bao_dung_chuc_nang() {
        let goc = CauHinh::default();
        let bao = |c: &CauHinh| MOI_TAB.into_iter().filter(|t| khac(*t, &goc, c)).collect::<Vec<_>>();
        assert_eq!(bao(&CauHinh { he_so: [1.0; 8], ..goc.clone() }), [TabCauHinh::Dem]);
        assert_eq!(bao(&CauHinh { mau_ten: "{n}".into(), ..goc.clone() }), [TabCauHinh::Tach]);
        assert_eq!(bao(&CauHinh { muc_luc: CauHinhMucLuc { ghi_de: !goc.muc_luc.ghi_de, ..goc.muc_luc.clone() }, ..goc.clone() }), [TabCauHinh::MucLuc]);
        let mut bt = goc.clone();
        bt.boc_tach.jpeg = bt.boc_tach.jpeg.wrapping_add(1);
        assert_eq!(bao(&bt), [TabCauHinh::BocTach]);
        assert_eq!(bao(&CauHinh { xoa_ghim: CauHinhXoaGhim { lo_10: false, ..Default::default() }, ..goc.clone() }), [TabCauHinh::XoaGhim]);
        // Trạng thái giao diện
        assert!(bao(&CauHinh { rong_cot_trai: 999.0, ch_thu_ds: true, xoa_ghim: CauHinhXoaGhim { rong_ds: 400.0, ..Default::default() }, ..goc.clone() }).is_empty());
    }

    /// Thêm trường vào `CauHinh` mà quên xếp vào chức năng / giao diện → không biên dịch (liệt kê đủ, không `..`).
    #[test]
    fn moi_truong_deu_da_xep_loai() {
        let CauHinh {
            // Đếm trang A4
            he_so: _,
            ranh_gioi: _,
            a0_lon: _,
            a0_lon_he_so: _,
            a0_nguong_mm: _,
            // Tách
            quet_trang: _,
            quet_ma: _,
            quet_xoay: _,
            nguong_trang: _,
            nguong_nghi: _,
            loai_ma: _,
            ma_index: _,
            tin_cay_xoay: _,
            thu_muc_ra: _,
            mau_ten: _,
            luu_nhan: _,
            // Mục lục (thư mục ra / mẫu tên / ghi đè) — còn lại là lựa chọn gần nhất của màn
            muc_luc: _,
            boc_tach: _,
            xoa_ghim: _,
            // Trạng thái giao diện
            rong_cot_trai: _,
            co_anh_hs: _,
            co_anh_vb: _,
            ty_le_vb: _,
            che_do_vb: _,
            an_dong_thu_muc: _,
            ch_cua_so_nho: _,
            ch_thu_ds: _,
        } = CauHinh::default();
    }
}
