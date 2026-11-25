use super::*;
use crate::xoa_ghim::vung::{ApTrang, LoaiVung, ViTri};
use std::time::Duration;

#[test]
fn vung_chung_va_rieng_luu_kho_mo_lai_thu_muc_thay_lai() {
    let d = std::env::temp_dir().join(format!("dema4_man_xg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let nguon = d.join("Nguon");
    std::fs::create_dir_all(nguon.join("con")).unwrap();
    crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 2);
    crate::tach_vb::tach::tests::tao_pdf(&nguon.join("con/b.pdf"), 1);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(nguon.clone());
    assert_eq!(man.ds.len(), 2);
    assert_eq!(man.bo.tm.vung.len(), 2, "thư mục mới: 2 góc trên mặc định");
    man.bo.bat_tat(ViTri::CanhTrai);
    assert!(man.bo.them_ve((0.2, 0.2, 0.3, 0.1), LoaiVung::BaoVe, PhamVi::Trang, ApTrang::TatCa, 1));
    man.chon_file(1); // đổi file → ghi ngay phần riêng của file trước
    assert!(man.bo.vf.vung.is_empty());
    man.ghi_ngay();
    let mut lai = ManXoaGhim::default();
    lai.mo_thu_muc(nguon.clone());
    assert!(lai.bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::CanhTrai));
    assert_eq!(lai.bo.vf.vung.len(), 1, "vùng riêng trang của a.pdf");
    assert_eq!(lai.bo.vf.vung[0].ap_trang, ApTrang::MotTrang(1));
    // Đổi tên file: vùng riêng theo nội dung vẫn còn
    std::fs::rename(nguon.join("a.pdf"), nguon.join("a_doi_ten.pdf")).unwrap();
    let mut doi_ten = ManXoaGhim::default();
    doi_ten.mo_thu_muc(nguon.clone());
    assert_eq!(doi_ten.bo.vf.vung.len(), 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goc_canh_theo_loc_trang_tuy_bien_la_vung_rieng_vo_doi_chi_khi_ve_xoa() {
    let d = std::env::temp_dir().join(format!("dema4_man_xg_tt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    crate::tach_vb::tach::tests::tao_pdf(&d.join("a.pdf"), 3);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    // Góc dưới phải bật khi đang chọn Trang lẻ → vùng chung áp trang lẻ
    man.loc_trang = LocTrang::Le;
    man.bat_tat_co_san(ViTri::GocDuoiPhai);
    let z = man.bo.tm.vung.iter().find(|z| z.vi_tri == ViTri::GocDuoiPhai).unwrap();
    assert_eq!(z.ap_trang, ApTrang::Le);
    assert_eq!(man.bo.chon.map(|c| c.0), Some(false));
    // Bấm lại → tắt
    man.bat_tat_co_san(ViTri::GocDuoiPhai);
    assert!(!man.bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::GocDuoiPhai));
    // Tùy biến "−" + Từng trang → vùng riêng file, chỉ trang đang vẽ
    man.ve = Some(LoaiVung::Xoa);
    man.loc_trang = LocTrang::TungTrang;
    man.them_ve((0.1, 0.1, 0.2, 0.2), 2);
    assert_eq!((man.bo.vf.vung[0].loai, man.bo.vf.vung[0].ap_trang), (LoaiVung::Xoa, ApTrang::MotTrang(2)));
    // "−" + Vô đối → vùng vô đối riêng trang
    man.loc_trang = LocTrang::VoDoi;
    man.them_ve((0.5, 0.5, 0.2, 0.2), 1);
    assert_eq!((man.bo.vf.vung[1].loai, man.bo.vf.vung[1].ap_trang), (LoaiVung::VoDoi, ApTrang::MotTrang(1)));
    // "+" + Tất cả → vùng bảo vệ riêng file mọi trang
    man.ve = Some(LoaiVung::BaoVe);
    man.loc_trang = LocTrang::TatCa;
    man.them_ve((0.3, 0.3, 0.1, 0.1), 0);
    assert_eq!((man.bo.vf.vung[2].loai, man.bo.vf.vung[2].ap_trang), (LoaiVung::BaoVe, ApTrang::TatCa));
    assert!(man.bo.tm.vung.iter().all(|z| z.vi_tri != ViTri::Tay), "Tùy biến không vào vùng chung");
    // Bỏ chọn file → không chạy file đó
    man.chon_chay[0] = false;
    assert!(man.can_chay().is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

/// Chụp ảnh tài liệu màn Xóa ghim (thư mục PDF mẫu trung tính, không dữ liệu thật):
/// `DEMA4_MAU_XG=<thư mục> DEMA4_ANH=<thư mục ảnh> cargo test --release chup_man_xoa_ghim -- --ignored`
/// → `xoa-ghim.png` (màn chính, đang chọn vùng cạnh trái) + `xoa-ghim-xong.png` (hộp kết quả sau khi chạy).
#[test]
#[ignore = "cần GPU và thư mục mẫu"]
fn chup_man_xoa_ghim() {
    let (Some(mau), Some(anh)) = (std::env::var_os("DEMA4_MAU_XG"), std::env::var_os("DEMA4_ANH")) else { return };
    let anh = PathBuf::from(anh);
    let rong: f32 = std::env::var("DEMA4_RONG").ok().and_then(|v| v.parse().ok()).unwrap_or(1440.0);
    let cao: f32 = std::env::var("DEMA4_CAO").ok().and_then(|v| v.parse().ok()).unwrap_or(860.0);
    let mut man = ManXoaGhim::default();
    man.license = crate::license::KiemTra::gia(std::sync::Arc::new(crate::tram::KhoGia::moi(10)));
    // Màn trống (chưa mở thư mục)
    {
        let mut trong = ManXoaGhim::default();
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(rong, cao)).with_pixels_per_point(1.25).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            trong.ve(ui);
        });
        h.run_steps(3);
        h.render().unwrap().save(anh.join("xoa-ghim-trong.png")).unwrap();
    }
    man.mo_thu_muc(PathBuf::from(mau));
    if !man.bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::CanhTrai) {
        man.bo.bat_tat(ViTri::CanhTrai);
    }
    man.bo.chon = man.bo.tm.vung.iter().position(|z| z.vi_tri == ViTri::CanhTrai).map(|i| (false, i));
    man.bo.them_ve((0.62, 0.035, 0.3, 0.05), LoaiVung::BaoVe, PhamVi::File, ApTrang::TatCa, 0);
    let man = std::rc::Rc::new(std::cell::RefCell::new(man));
    let m2 = man.clone();
    let mut da_cai = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(rong, cao)).with_pixels_per_point(1.25).build_ui(move |ui| {
        if !std::mem::replace(&mut da_cai, true) {
            crate::gui::cai_giao_dien(ui.ctx());
        }
        m2.borrow_mut().ve(ui);
    });
    let cho = |h: &mut egui_kittest::Harness, n: usize| {
        for _ in 0..n {
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(40));
        }
    };
    cho(&mut h, 80);
    h.render().unwrap().save(anh.join("xoa-ghim.png")).unwrap();
    man.borrow_mut().bam_chay(true);
    cho(&mut h, 80);
    h.render().unwrap().save(anh.join("xoa-ghim-xong.png")).unwrap();
}
