use super::sua_vung::PhamVi;
use super::*;
use crate::xoa_ghim::kho::KieuGhi;
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
    assert!(man.bo.tm.vung.is_empty(), "thư mục mới: chưa có vùng nào (góc / cạnh / Tùy biến đều tắt)");
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
    assert_eq!(man.bo.chon.iter().map(|c| c.0).collect::<Vec<_>>(), vec![false], "góc vừa bật được chọn");
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
/// → `xoa-ghim-trong.png` (chưa mở), `xoa-ghim.png` (màn chính, đang chọn vùng cạnh trái), `xoa-ghim-cau-hinh.png`
/// (Cấu hình › Xóa ghim), `xoa-ghim-xong.png` (hộp kết quả sau khi chạy).
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
            trong.ve(ui, &mut CauHinhXoaGhim::default());
        });
        h.run_steps(3);
        h.render().unwrap().save(anh.join("xoa-ghim-trong.png")).unwrap();
    }
    man.mo_thu_muc(PathBuf::from(mau));
    if !man.bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::CanhTrai) {
        man.bo.bat_tat(ViTri::CanhTrai);
    }
    man.bo.chon = man.bo.tm.vung.iter().position(|z| z.vi_tri == ViTri::CanhTrai).map(|i| (false, i)).into_iter().collect();
    man.bo.them_ve((0.62, 0.035, 0.3, 0.05), LoaiVung::BaoVe, PhamVi::File, ApTrang::TatCa, 0);
    let man = std::rc::Rc::new(std::cell::RefCell::new(man));
    let m2 = man.clone();
    let hop: std::rc::Rc<std::cell::RefCell<Option<crate::gui_tach::hop_cau_hinh::HopCauHinh>>> = Default::default();
    let hop2 = hop.clone();
    let mut da_cai = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(rong, cao)).with_pixels_per_point(1.25).build_ui(move |ui| {
        if !std::mem::replace(&mut da_cai, true) {
            crate::gui::cai_giao_dien(ui.ctx());
        }
        m2.borrow_mut().ve(ui, &mut CauHinhXoaGhim::default());
        if let Some(h) = hop2.borrow_mut().as_mut() {
            h.cua_so_nho = true;
            let _ = h.ve_cua_so(ui.ctx());
        }
    });
    let cho = |h: &mut egui_kittest::Harness, n: usize| {
        for _ in 0..n {
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(40));
        }
    };
    cho(&mut h, 80);
    h.render().unwrap().save(anh.join("xoa-ghim.png")).unwrap();
    // Cấu hình › Xóa ghim trên nền màn đang mở
    {
        let mut x = crate::gui_tach::hop_cau_hinh::HopCauHinh::moi(&crate::cau_hinh::CauHinh::default(), crate::gui_tach::hop_cau_hinh::TabCauHinh::XoaGhim);
        x.xg.thu_muc = man.borrow().cau_hinh_thu_muc();
        *hop.borrow_mut() = Some(x);
        cho(&mut h, 20);
        h.render().unwrap().save(anh.join("xoa-ghim-cau-hinh.png")).unwrap();
        *hop.borrow_mut() = None;
        cho(&mut h, 5);
    }
    man.borrow_mut().bam_chay(true);
    cho(&mut h, 80);
    h.render().unwrap().save(anh.join("xoa-ghim-xong.png")).unwrap();
}

/// Thư mục tạm có `ten` PDF (số trang theo danh sách).
fn thu_muc_mau(ten: &str, ds: &[(&str, usize)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dema4_man_xg_{ten}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (f, n) in ds {
        let p = d.join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&p, *n);
    }
    d
}

/// Chờ luồng nền đọc xong số trang.
fn cho_thong_tin(man: &mut ManXoaGhim) {
    for _ in 0..200 {
        if let Some(n) = &man.nhan_thong_tin {
            for (i, tt) in n.try_iter() {
                man.thong_tin[i] = Some(tt);
            }
        }
        if man.thong_tin.iter().all(Option::is_some) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("không đọc xong số trang");
}

#[test]
fn loc_so_trang_va_nang_cao() {
    let f = khop_so("1-3, 7, >10, <=0").unwrap();
    assert!(f(2) && f(7) && f(11) && f(0) && !f(5) && !f(10));
    assert!(khop_so("  ").is_none() && khop_so("abc").is_none());
    // Cỡ trang theo điểm PDF như màn Đếm: A4 dọc, A3 ngang, A4 dọc
    let tt = ThongTinFile { so_trang: 3, trang: vec![(595.276, 841.89), (1190.551, 841.89), (595.276, 841.89)] };
    let qt = crate::dem_pdf::QuyTac::default();
    let khop = |tt: &ThongTinFile, l: &LocNangCao| khop_nang_cao(tt, l, &qt);
    let mut l = LocNangCao::default();
    assert!(khop(&tt, &l), "mặc định A3 + A4, 2 chiều");
    l.kho = [false; 8];
    l.kho[3] = true;
    l.ngang = false;
    assert!(!khop(&tt, &l), "A3 chỉ có trang ngang");
    l.ngang = true;
    l.dai = "1, 3".into();
    assert!(!khop(&tt, &l), "A3 ngang ở trang 2 — ngoài dải");
    l.dai = "2".into();
    assert!(khop(&tt, &l));
    // Ô có mặt: chỉ A3, A4; có cả ngang lẫn dọc. File chỉ A4 dọc → không có ngang
    let (kho, ngang, doc) = co_mat(&[Some(tt.clone()), None], &qt);
    assert_eq!((kho, ngang, doc), ([false, false, false, true, true, false, false, false], true, true));
    let chi_doc = ThongTinFile { so_trang: 1, trang: vec![(595.276, 841.89)] };
    assert_eq!(co_mat(&[Some(chi_doc)], &qt).1, false);
    // Ranh giới khổ tuỳ chỉnh của Đếm: A4 dài 297 mm, đặt ranh A3|A4 = 290 mm → xếp A3
    let mut rg = [0.0; 7];
    for (i, r) in rg.iter_mut().enumerate() {
        *r = [1000.0, 700.0, 500.0, 290.0, 250.0, 180.0, 120.0][i];
    }
    let qt2 = crate::dem_pdf::QuyTac { ranh_gioi_mm: Some(rg), ..Default::default() };
    assert!(co_mat(&[Some(tt)], &qt2).0[3] && !co_mat(&[Some(ThongTinFile { so_trang: 1, trang: vec![(595.276, 841.89)] })], &qt2).0[4]);
}

#[test]
fn danh_sach_sap_xep_loc_ten_va_so_trang() {
    let d = thu_muc_mau("ds", &[("b10.pdf", 1), ("a2.pdf", 3), ("con/a10.pdf", 2)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    cho_thong_tin(&mut man);
    let ten = |m: &ManXoaGhim| m.thu_tu_hien().iter().map(|&i| m.ds[i].0.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
    // Tên tự nhiên (đường dẫn tương đối): a2 < b10 < con/a10
    assert_eq!(ten(&man), ["a2.pdf", "b10.pdf", "a10.pdf"]);
    man.sap_giam = true;
    assert_eq!(ten(&man), ["a10.pdf", "b10.pdf", "a2.pdf"]);
    man.sap_theo = SapTheo::Trang;
    man.sap_giam = false;
    assert_eq!(ten(&man), ["b10.pdf", "a10.pdf", "a2.pdf"], "1 < 2 < 3 trang");
    man.loc_so = ">1".into();
    assert_eq!(ten(&man), ["a10.pdf", "a2.pdf"]);
    man.loc_so.clear();
    man.loc_ten = "A1".into();
    assert_eq!(ten(&man), ["a10.pdf"], "không phân biệt hoa thường, theo đường dẫn trong thư mục con");
    man.loc_ten = "10".into();
    assert_eq!(ten(&man), ["b10.pdf", "a10.pdf"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chon_nhieu_xoa_vung_ve_can_xac_nhan_va_hoan_tac() {
    let d = thu_muc_mau("xoa", &[("a.pdf", 2)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    man.loc_trang = LocTrang::TatCa;
    man.bat_tat_co_san(ViTri::GocTrenTrai);
    man.ve = Some(LoaiVung::Xoa);
    man.them_ve((0.1, 0.1, 0.2, 0.2), 0);
    man.them_ve((0.5, 0.5, 0.2, 0.2), 0);
    man.ve = None;
    assert_eq!(man.bo.moi_vung_ve().len(), 2);
    // Ctrl+A chọn cả góc lẫn vùng vẽ; xoá chỉ lấy vùng vẽ
    man.bo.chon = man.bo.tat_ca();
    let ve = man.bo.chi_vung_ve(&man.bo.chon.clone());
    assert_eq!(ve.len(), 2);
    // Nút "Xóa vùng vẽ" mở hộp xác nhận, chưa xoá
    man.xac_nhan_xoa = Some(man.bo.moi_vung_ve());
    assert_eq!(man.bo.moi_vung_ve().len(), 2);
    assert_eq!(man.bo.xoa(&man.xac_nhan_xoa.take().unwrap()), 2);
    assert!(man.bo.moi_vung_ve().is_empty());
    assert_eq!(man.bo.tm.vung.len(), 1, "góc giữ nguyên");
    assert!(man.bo.hoan_tac());
    assert_eq!(man.bo.moi_vung_ve().len(), 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn file_dich_theo_kieu_ghi_va_mot_file() {
    let d = thu_muc_mau("dich", &[("a.pdf", 1), ("con/b.pdf", 1)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    assert_eq!(man.ds.len(), 2);
    let ra = man.thu_muc_ket_qua_hien_tai().unwrap();
    assert!(ra.to_string_lossy().ends_with("_xoaghim"), "mặc định: nhân bản cạnh nguồn");
    assert!(man.ds.iter().all(|(_, dich)| dich.starts_with(&ra)));
    // Cạnh file gốc: đích = <tên>_xoaghim.pdf cùng thư mục; danh sách không lẫn file kết quả
    man.bo.tm.ghi.kieu = KieuGhi::CanhFile;
    man.lap_ds_giu_chon();
    assert!(man.ds.iter().all(|(n, dich)| dich.parent() == n.parent() && dich.file_name().unwrap().to_string_lossy().ends_with("_xoaghim.pdf")));
    std::fs::copy(d.join("a.pdf"), d.join("a_xoaghim.pdf")).unwrap();
    man.lap_ds_giu_chon();
    assert_eq!(man.ds.len(), 2, "bỏ *_xoaghim.pdf");
    // Ghi đè: đích = nguồn
    man.bo.tm.ghi.ghi_de = true;
    man.lap_ds_giu_chon();
    assert!(man.ds.iter().all(|(n, dich)| n == dich));
    // Mở đúng 1 file
    let mut mot = ManXoaGhim::default();
    mot.mo_pdf(d.join("con/b.pdf"));
    assert_eq!(mot.ds.len(), 1);
    assert_eq!(mot.thu_muc.as_deref(), Some(d.join("con").as_path()));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn chia_doi_thanh_cong_cu_va_cau_hinh_chung_dong_bo() {
    let d = thu_muc_mau("cfg", &[("a.pdf", 1)]);
    let mut man = ManXoaGhim::default();
    assert_eq!(man.rong_nho(), 0.0, "chưa mở: không có cột trang thu nhỏ");
    man.mo_thu_muc(d.clone());
    assert!(man.rong_nho() > 0.0);
    man.cfg.thu_nho = false;
    assert_eq!(man.rong_nho(), 0.0);
    // Cấu hình theo thư mục qua hộp Cấu hình
    let mut c = man.cau_hinh_thu_muc().unwrap();
    c.xuat.dpi = 200;
    c.tc.nen_trang = true;
    man.ap_cau_hinh_thu_muc(c);
    assert_eq!((man.bo.tm.xuat.dpi, man.bo.tm.tuy_chon.nen_trang), (200, true));
    let mut lai = ManXoaGhim::default();
    lai.mo_thu_muc(d.clone());
    assert_eq!(lai.bo.tm.xuat.dpi, 200, "lưu kho theo thư mục");
    // Chưa có mô hình: tick Nhận diện layout không bật được ở thanh công cụ (ô mờ) — cờ lưu vẫn tắt mặc định
    assert!(!lai.bo.tm.tuy_chon.nhan_dien);
    let _ = std::fs::remove_dir_all(&d);
}

/// Ảnh đối chiếu prototype (dữ liệu mẫu trung tính):
/// `DEMA4_MAU_XG=<thư mục> DEMA4_ANH=<thư mục ảnh> cargo test --release chup_doi_chieu_prototype -- --ignored`
/// → `doi-chieu-trong.png`, `doi-chieu-mo.png`, `doi-chieu-cau-hinh.png` (1600 × 900, như ảnh prototype).
#[test]
#[ignore = "cần GPU và thư mục mẫu"]
fn chup_doi_chieu_prototype() {
    let (Some(mau), Some(anh)) = (std::env::var_os("DEMA4_MAU_XG"), std::env::var_os("DEMA4_ANH")) else { return };
    let anh = PathBuf::from(anh);
    let chup = |ten: &str, man: ManXoaGhim, cau_hinh: bool| {
        let man = std::rc::Rc::new(std::cell::RefCell::new(man));
        let m2 = man.clone();
        let mut da_cai = false;
        let mut hop = cau_hinh.then(|| {
            let mut h = crate::gui_tach::hop_cau_hinh::HopCauHinh::moi(&crate::cau_hinh::CauHinh::default(), crate::gui_tach::hop_cau_hinh::TabCauHinh::XoaGhim);
            h.xg.thu_muc = man.borrow().cau_hinh_thu_muc();
            h
        });
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1600.0, 900.0)).with_pixels_per_point(1.0).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            m2.borrow_mut().ve(ui, &mut CauHinhXoaGhim::default());
            if let Some(hop) = &mut hop {
                hop.cua_so_nho = true;
                let _ = hop.ve_cua_so(ui.ctx());
            }
        });
        for _ in 0..90 {
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(40));
        }
        h.render().unwrap().save(anh.join(ten)).unwrap();
    };
    chup("doi-chieu-trong.png", ManXoaGhim::default(), false);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(PathBuf::from(&mau));
    man.bo.chon = vec![(false, 0)];
    chup("doi-chieu-mo.png", man, false);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(PathBuf::from(&mau));
    chup("doi-chieu-cau-hinh.png", man, true);
}

/// Cửa sổ mặc định / hẹp: khối Vùng · Rộng · Cao · Ngưỡng không tràn sang nửa phải (Đầu vào); hẹp thì chỉ còn ô số.
#[test]
fn khoi_vung_khong_tran_sang_nua_phai() {
    use egui_kittest::kittest::Queryable;
    for (rong, co_thanh) in [(1120.0, false), (1600.0, true), (980.0, false)] {
        let mut man = ManXoaGhim::default();
        man.ds_mo = Some(false);
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(rong, 760.0)).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            man.ve(ui, &mut CauHinhXoaGhim::default());
        });
        h.run_steps(3);
        let dau_vao = h.get_by_label("Đầu vào").rect();
        let vung = h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Vùng:").rect();
        assert!(vung.max.x <= dau_vao.min.x - 20.0, "{rong}: ô Vùng ({vung:?}) tràn");
        for o in ["Rộng (số)", "Cao (số)", "Ngưỡng (số)"] {
            let r = h.get_by_label(o).rect();
            assert!(r.max.x <= dau_vao.min.x - 60.0, "{rong}: ô {o} ({r:?}) tràn sang Đầu vào ({dau_vao:?})");
        }
        assert_eq!(h.query_by_role_and_label(egui::accesskit::Role::Slider, "Rộng").is_some(), co_thanh, "{rong}: thanh trượt");
        // Có DEMA4_ANH (máy có GPU): lưu ảnh để xem bằng mắt
        if let Some(d) = std::env::var_os("DEMA4_ANH") {
            if let Ok(anh) = h.render() {
                let _ = anh.save(PathBuf::from(d).join(format!("xoa-ghim-rong-{rong}.png")));
            }
        }
    }
}

/// Bấm ô "Tô nền trắng" ở thanh công cụ → đổi tuỳ chọn thư mục (bật / tắt), ảnh Đích làm lại (mã cấu hình đổi).
#[test]
fn o_to_nen_trang_bat_tat_duoc() {
    use egui_kittest::kittest::Queryable;
    let d = thu_muc_mau("nen", &[("a.pdf", 1)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    let man = std::rc::Rc::new(std::cell::RefCell::new(man));
    let m2 = man.clone();
    let mut da_cai = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1600.0, 900.0)).build_ui(move |ui| {
        if !std::mem::replace(&mut da_cai, true) {
            crate::gui::cai_giao_dien(ui.ctx());
        }
        m2.borrow_mut().ve(ui, &mut CauHinhXoaGhim::default());
    });
    h.run_steps(3);
    let truoc = man.borrow().bo.tm.tuy_chon.nen_trang;
    let ma_truoc = man.borrow().cau_hinh_xem().1;
    h.get_by_label("Tô nền trắng").click();
    h.run_steps(2);
    assert_eq!(man.borrow().bo.tm.tuy_chon.nen_trang, !truoc, "bấm 1 lần → đổi");
    assert_ne!(man.borrow().cau_hinh_xem().1, ma_truoc, "ảnh Đích phải làm lại");
    h.get_by_label("Tô nền trắng").click();
    h.run_steps(2);
    assert_eq!(man.borrow().bo.tm.tuy_chon.nen_trang, truoc, "bấm lần 2 → về như cũ");
    let _ = std::fs::remove_dir_all(&d);
}

/// Thu gọn (^) → hàng mini: bấm góc / cạnh bật-tắt vùng, − / + vẽ vùng, phạm vi trang đổi vùng đang chọn,
/// Xóa vùng vẽ hỏi lại, AI mờ khi chưa có mô hình; bấm ^ lần nữa về thanh đầy đủ.
#[test]
fn thanh_mini_du_chuc_nang() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let d = thu_muc_mau("mini", &[("a.pdf", 2)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    let man = std::rc::Rc::new(std::cell::RefCell::new(man));
    let m2 = man.clone();
    let mut da_cai = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 800.0)).build_ui(move |ui| {
        if !std::mem::replace(&mut da_cai, true) {
            crate::gui::cai_giao_dien(ui.ctx());
        }
        let mut m = m2.borrow_mut();
        egui::Panel::top("menu").show(ui, |ui| m.nut_thu_gon_cong_cu(ui));
        m.ve(ui, &mut CauHinhXoaGhim::default());
    });
    h.run_steps(3);
    h.get_by_label("Thu gọn / mở thanh công cụ").click();
    h.run_steps(2);
    assert!(man.borrow().an_cong_cu);
    assert!(h.query_by_label("Đầu vào").is_none(), "thanh đầy đủ đã ẩn");
    // Góc dưới phải: bật rồi tắt
    h.get_by_label("Góc dưới phải").click();
    h.run_steps(2);
    assert!(man.borrow().bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::GocDuoiPhai));
    // Cạnh trái bật; phạm vi "Trang lẻ" áp cho vùng vừa bật (đang chọn)
    h.get_by_label("Cạnh trái").click();
    h.run_steps(2);
    h.get_by_label("Trang lẻ").click();
    h.run_steps(2);
    let z = man.borrow().bo.tm.vung.iter().find(|z| z.vi_tri == ViTri::CanhTrai).cloned().unwrap();
    assert_eq!(z.ap_trang, ApTrang::Le);
    h.get_by_label("Góc dưới phải").click();
    h.run_steps(2);
    assert!(!man.borrow().bo.tm.vung.iter().any(|z| z.vi_tri == ViTri::GocDuoiPhai));
    // − vẽ vùng xoá → có thêm "Vô đối"; bấm lại thôi vẽ
    h.get_by_label("Vẽ vùng xoá").click();
    h.run_steps(2);
    assert_eq!(man.borrow().ve, Some(LoaiVung::Xoa));
    h.get_by_label("Vô đối");
    h.get_by_label("Vẽ vùng bảo vệ").click();
    h.run_steps(2);
    assert_eq!(man.borrow().ve, Some(LoaiVung::BaoVe));
    // Xóa vùng vẽ mờ khi chưa có vùng vẽ; có vùng vẽ → mở hộp hỏi
    assert!(h.get_by_label("Xóa vùng vẽ").accesskit_node().is_disabled());
    man.borrow_mut().loc_trang = LocTrang::TatCa;
    man.borrow_mut().them_ve((0.4, 0.4, 0.1, 0.1), 0);
    man.borrow_mut().ve = None;
    h.run_steps(2);
    h.get_by_label("Xóa vùng vẽ").click();
    h.run_steps(2);
    assert!(man.borrow().xac_nhan_xoa.is_some());
    man.borrow_mut().xac_nhan_xoa = None;
    h.run_steps(1);
    // AI: chưa có mô hình → mờ
    if !crate::xoa_ghim::mo_hinh::co_san() {
        assert!(h.get_by_label("Nhận diện layout").accesskit_node().is_disabled());
    }
    if let Some(dd) = std::env::var_os("DEMA4_ANH") {
        if let Ok(anh) = h.render() {
            let _ = anh.save(PathBuf::from(dd).join("xoa-ghim-mini.png"));
        }
    }
    // ^ lần nữa → thanh đầy đủ
    h.get_by_label("Thu gọn / mở thanh công cụ").click();
    h.run_steps(2);
    assert!(!man.borrow().an_cong_cu);
    h.get_by_label("Đầu vào");
    let _ = std::fs::remove_dir_all(&d);
}

/// Vừa bề ngang theo trang đang xem: file trộn A4 dọc + A3 ngang → sang trang A3 thì tỉ lệ nhỏ lại theo trang đó.
#[test]
fn vua_ngang_theo_trang_dang_xem() {
    let d = thu_muc_mau("vua", &[("a.pdf", 1)]);
    let mut man = ManXoaGhim::default();
    man.mo_thu_muc(d.clone());
    let man = std::rc::Rc::new(std::cell::RefCell::new(man));
    let m2 = man.clone();
    let mut da_cai = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 800.0)).build_ui(move |ui| {
        if !std::mem::replace(&mut da_cai, true) {
            crate::gui::cai_giao_dien(ui.ctx());
        }
        m2.borrow_mut().ve(ui, &mut CauHinhXoaGhim::default());
    });
    // Khổ giả: trang 1 A4 dọc, trang 2 A3 ngang (rộng gấp ~2)
    man.borrow_mut().kho_trang = vec![(595.0, 842.0), (1191.0, 842.0)];
    man.borrow_mut().cfg.tung_trang = true;
    h.run_steps(2);
    let ty_a4 = man.borrow().ty_le_cuoi;
    man.borrow_mut().sang_trang(1);
    h.run_steps(2);
    let ty_a3 = man.borrow().ty_le_cuoi;
    assert!((ty_a4 / ty_a3 - 1191.0 / 595.0).abs() < 0.01, "A4 {ty_a4} vs A3 {ty_a3}: tỉ lệ theo bề rộng trang đang xem");
    let _ = std::fs::remove_dir_all(&d);
}
