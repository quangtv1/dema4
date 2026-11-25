use super::*;
use crate::tach_vb::nhan::Nhan;
use crate::tach_vb::tach::tests::tao_pdf;

/// Thư mục test riêng: `<tạm>/dema4_phien_<ten>_<pid>/`, trả (gốc test, file loại tài liệu riêng).
fn tam(ten: &str) -> (PathBuf, PathBuf) {
    let d = std::env::temp_dir().join(format!("dema4_phien_{ten}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f_loai = d.join("loai_tai_lieu.json");
    (d, f_loai)
}

/// Thư mục đầu vào có `a.pdf` (3 trang), `b.pdf` (2 trang), `c.pdf` (2 trang); xoá dữ liệu làm dở cũ.
fn thu_muc(cha: &Path, ten: &str) -> PathBuf {
    let g = cha.join(ten);
    std::fs::create_dir_all(&g).unwrap();
    tao_pdf(&g.join("a.pdf"), 3);
    tao_pdf(&g.join("b.pdf"), 2);
    tao_pdf(&g.join("c.pdf"), 2);
    let _ = std::fs::remove_dir_all(thu_muc_an(&g));
    g
}

fn ghi_nhan(goc: &Path, ds: &[(&str, usize, Nhan)]) {
    let mut b = BangNhan::new();
    for (k, t, n) in ds {
        b.entry(k.to_string()).or_default().insert(*t, *n);
    }
    ghi_an_toan(&thu_muc_an(goc).join(F_NHAN), nhan::ghi_file_nhan(&b)).unwrap();
}

fn ghi_moc(goc: &Path, ds: &[(&str, u64, u64)]) {
    let m = ds.iter().map(|(k, luc, ck)| (k.to_string(), Moc { luc: *luc, chu_ky: *ck })).collect();
    moc_sua::ghi(&thu_muc_an(goc).join(moc_sua::TEN_FILE), &m).unwrap();
}

fn doc_nhan(goc: &Path) -> BangNhan {
    nhan::doc_file_nhan(&std::fs::read_to_string(thu_muc_an(goc).join(F_NHAN)).unwrap_or_default(), &|_| None).unwrap().0
}

fn ban_ghi(file: &str, loai: &str, so: &str, luc: u64, loi: bool) -> BanGhi {
    let mut gia_tri = BTreeMap::new();
    gia_tri.insert("so_ky_hieu".to_string(), so.to_string());
    BanGhi { file: file.into(), loai: loai.into(), so_trang: 1, gia_tri, loi: loi.then(|| "500".into()), sua: vec![], canh_bao: None, giay: 0.0, tho: vec![], luc }
}

fn ten_trong_goi(f: &Path) -> Vec<String> {
    let z = zip::ZipArchive::new(std::fs::File::open(f).unwrap()).unwrap();
    z.file_names().map(|n| n.unwrap().into_owned()).collect()
}

/// Gói tự dựng (test gói độc / phiên bản lạ).
fn tao_goi(f: &Path, muc: &[(&str, &[u8])]) {
    let mut z = zip::ZipWriter::new(std::fs::File::create(f).unwrap());
    for (ten, b) in muc {
        z.start_file(*ten, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b).unwrap();
    }
    z.finish().unwrap();
}

fn manifest_mau(phien_ban: u32) -> Vec<u8> {
    serde_json::to_vec(&Manifest { phien_ban_goi: phien_ban, phien_ban_app: "x".into(), ten_thu_muc: "B".into(), tao_luc: 1, pdf: vec![], loai_tai_lieu: vec![] }).unwrap()
}

#[test]
fn xuat_roi_nhap_sang_may_khac_ten_nfd_nfc_giu_nhan_cache_ket_qua_khong_mang_khoa_license() {
    let (d, f_loai) = tam("khu_hoi");
    // Máy A: tên thư mục dạng NFD (Drive trên Mac); máy B: cùng nội dung, đường dẫn khác, tên NFC, mtime khác (chép)
    let a = thu_muc(&d, "Ha\u{0309}i_A");
    let an = thu_muc_an(&a);
    ghi_nhan(&a, &[("a.pdf", 2, Nhan::Index)]);
    ghi_moc(&a, &[("a.pdf", 100, 7)]);
    let khoa = cache_trang::KhoaFile::cua(&a.join("a.pdf"), "a.pdf").unwrap();
    cache_trang::ghi(&an.join("cache"), &a.join("a.pdf"), &khoa, &[Default::default(), Default::default(), Default::default()]).unwrap();
    luu::ghi_them(&an.join(F_JSONL), &ban_ghi("a.pdf", "VB", "15/TTr", 50, false)).unwrap();
    // Những thứ không bao giờ được vào gói, dù nằm trong thư mục việc
    for f in ["khoa.bin", "license_so_luot.json", ".dema4_saomai.cfg", "ketqua_tach.txt"] {
        std::fs::write(an.join(f), "bi mat").unwrap();
    }
    let goi = d.join("A.sm-dema4.zip");
    let tt = xuat_voi(&a, &goi, &f_loai).unwrap();
    assert_eq!((tt.so_ho_so, tt.so_ban_ghi_boc), (1, 1));
    let ten = ten_trong_goi(&goi);
    for f in ["khoa.bin", "license_so_luot.json", ".dema4_saomai.cfg", "ketqua_tach.txt"] {
        assert!(!ten.iter().any(|t| t.ends_with(f)), "{f} không được vào gói: {ten:?}");
    }

    let b = d.join("H\u{1ea3}i_B");
    std::fs::create_dir_all(&b).unwrap();
    for f in ["a.pdf", "b.pdf", "c.pdf"] {
        std::fs::write(b.join(f), std::fs::read(a.join(f)).unwrap()).unwrap(); // chép: mtime mới
    }
    let _ = std::fs::remove_dir_all(thu_muc_an(&b));
    let xt = xem_truoc_voi(&b, &goi, &f_loai).unwrap();
    assert_eq!((xt.so_pdf_goi, xt.so_pdf_khop, xt.so_ho_so), (3, 3, 1));
    let bc = nhap_voi(&b, &goi, &f_loai).unwrap();
    assert_eq!((bc.nhan, bc.boc_nhan, bc.bo_qua.len()), (1, 1, 0), "{bc:?}");
    assert_eq!(doc_nhan(&b)["a.pdf"][&2], Nhan::Index);
    let kb = cache_trang::KhoaFile::cua(&b.join("a.pdf"), "a.pdf").unwrap();
    assert!(cache_trang::doc(&thu_muc_an(&b).join("cache"), &b.join("a.pdf"), &kb).is_some(), "cache dùng được trên máy nhận (khớp nhờ vân tay)");
    assert_eq!(luu::doc(&luu::file_jsonl(&b), "VB")["a.pdf"].gia_tri["so_ky_hieu"], "15/TTr");
    assert_eq!(moc_sua::doc(&thu_muc_an(&b).join(moc_sua::TEN_FILE))["a.pdf"].luc, 100);
    // Nhập lại cùng gói: không đổi gì
    let bc2 = nhap_voi(&b, &goi, &f_loai).unwrap();
    assert_eq!((bc2.nhan, bc2.boc_nhan), (0, 0));
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn hai_may_chia_ho_so_gop_sau_cung_thang_bia_mo_khong_hoi_nhan_cu() {
    let (d, f_loai) = tam("a_b");
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    // Máy A: a.pdf sửa lúc 100; b.pdf sửa lúc 300; c.pdf đã xoá hết nhãn lúc 500 (bia mộ)
    ghi_nhan(&a, &[("a.pdf", 2, Nhan::Index), ("b.pdf", 1, Nhan::Index)]);
    ghi_moc(&a, &[("a.pdf", 100, 1), ("b.pdf", 300, 2), ("c.pdf", 500, moc_sua::CHU_KY_RONG)]);
    // Máy B: a.pdf sửa sau (200, khác), b.pdf sửa trước (250, khác), c.pdf còn nhãn cũ (400)
    ghi_nhan(&b, &[("a.pdf", 3, Nhan::Trang), ("b.pdf", 2, Nhan::Trang), ("c.pdf", 1, Nhan::Index)]);
    ghi_moc(&b, &[("a.pdf", 200, 11), ("b.pdf", 250, 12), ("c.pdf", 400, 13)]);
    let goi = d.join("B.zip");
    xuat_voi(&b, &goi, &f_loai).unwrap();
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    let n = doc_nhan(&a);
    assert_eq!(n["a.pdf"].get(&3), Some(&Nhan::Trang), "a.pdf: B sửa sau → lấy của B");
    assert_eq!(n["b.pdf"].get(&1), Some(&Nhan::Index), "b.pdf: A sửa sau → giữ của A");
    assert!(!n.contains_key("c.pdf"), "c.pdf: bia mộ A mới hơn → không hồi nhãn cũ của B");
    assert_eq!((bc.nhan, bc.giu), (1, 2));
    assert_eq!(bc.trung.len(), 3);
    assert!(bc.trung.iter().any(|t| t.khoa == "a.pdf" && t.lay_goi && t.moc_goi == 200 && t.moc_may == 100));
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn boc_tach_loi_moi_hon_khong_de_ket_qua_tot_hai_loai_cung_giu() {
    let (d, f_loai) = tam("boc");
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    let (fa, fb) = (luu::file_jsonl(&a), luu::file_jsonl(&b));
    luu::ghi_them(&fa, &ban_ghi("a.pdf", "VB", "15/TTr", 100, false)).unwrap(); // A: kết quả tốt
    luu::ghi_them(&fb, &ban_ghi("a.pdf", "VB", "", 900, true)).unwrap(); // B: đọc lại bị lỗi (mới hơn)
    luu::ghi_them(&fb, &ban_ghi("a.pdf", "HD", "HĐ-01", 50, false)).unwrap(); // B: loại khác
    luu::ghi_them(&fb, &ban_ghi("b.pdf", "VB", "16/QĐ", u64::MAX, false)).unwrap(); // đồng hồ B sai
    let goi = d.join("B.zip");
    xuat_voi(&b, &goi, &f_loai).unwrap();
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    assert_eq!(luu::doc(&fa, "VB")["a.pdf"].gia_tri["so_ky_hieu"], "15/TTr", "bản lỗi không đè kết quả tốt");
    assert_eq!(luu::doc(&fa, "HD")["a.pdf"].gia_tri["so_ky_hieu"], "HĐ-01", "loại khác của cùng file vẫn nhận");
    let b_pdf = &luu::doc(&fa, "VB")["b.pdf"];
    assert!(b_pdf.luc < u64::MAX, "mốc từ đồng hồ sai bị kẹp");
    assert_eq!((bc.boc_nhan, bc.boc_giu, bc.lech_dong_ho), (2, 1, 1));
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goi_doc_khoa_la_bi_bo_qua_phien_ban_moi_hon_va_zip_cut_cut_bi_tu_choi() {
    let (d, f_loai) = tam("doc");
    let a = thu_muc(&d, "A");
    ghi_nhan(&a, &[("a.pdf", 2, Nhan::Index)]);
    let truoc = std::fs::read_to_string(thu_muc_an(&a).join(F_NHAN)).unwrap();
    // Khoá lạ trong file nhãn / mốc / jsonl: ../, tuyệt đối, ổ đĩa, file không có trên máy
    let nhan_doc = "../x.pdf;1;index\n/etc/x.pdf;1;index\nC:/x.pdf;1;index\nkhong_co.pdf;1;index\n";
    let moc_doc = "../x.pdf;9;1\n/etc/x.pdf;9;1\nC:/x.pdf;9;1\nkhong_co.pdf;9;1\n";
    let jsonl = serde_json::to_string(&ban_ghi("../a.pdf", "VB", "x", 1, false)).unwrap();
    let goi = d.join("doc.zip");
    tao_goi(&goi, &[(F_MANIFEST, &manifest_mau(1)), (F_NHAN, nhan_doc.as_bytes()), (moc_sua::TEN_FILE, moc_doc.as_bytes()), (F_JSONL, jsonl.as_bytes())]);
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    assert_eq!((bc.nhan, bc.boc_nhan), (0, 0), "{bc:?}");
    assert_eq!(bc.bo_qua.len(), 5, "{:?}", bc.bo_qua);
    assert_eq!(std::fs::read_to_string(thu_muc_an(&a).join(F_NHAN)).unwrap(), truoc, "không ghi gì");
    assert!(!d.join("x.pdf").exists() && !a.parent().unwrap().join("x.pdf").exists());
    // Gói từ bản mới hơn
    let moi = d.join("moi.zip");
    tao_goi(&moi, &[(F_MANIFEST, &manifest_mau(2))]);
    assert!(nhap_voi(&a, &moi, &f_loai).unwrap_err().contains("mới hơn"));
    // Zip cắt cụt
    let b = std::fs::read(&goi).unwrap();
    let cut = d.join("cut.zip");
    std::fs::write(&cut, &b[..b.len() / 2]).unwrap();
    assert!(nhap_voi(&a, &cut, &f_loai).is_err());
    assert_eq!(std::fs::read_to_string(thu_muc_an(&a).join(F_NHAN)).unwrap(), truoc, "gói hỏng → máy nhận không đổi");
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn loai_tai_lieu_chi_them_loai_moi_hop_le_trung_ten_giu_may_nay() {
    let (d, f_loai) = tam("loai");
    let a = thu_muc(&d, "A");
    let co_san = loai_tai_lieu::co_san();
    loai_tai_lieu::ghi(&f_loai, &co_san).unwrap();
    let mut moi = co_san[0].clone();
    moi.ten = "Loại riêng B".into();
    let mut trung = co_san[0].clone();
    trung.huong_dan = "khác nội dung".into();
    let mut sai = co_san[0].clone();
    sai.ten = "Loại sai".into();
    sai.truong[0].bien = "../x".into();
    let m = Manifest { loai_tai_lieu: vec![moi.clone(), trung.clone(), sai], ..serde_json::from_slice(&manifest_mau(1)).unwrap() };
    let goi = d.join("loai.zip");
    tao_goi(&goi, &[(F_MANIFEST, &serde_json::to_vec(&m).unwrap())]);
    let xt = xem_truoc_voi(&a, &goi, &f_loai).unwrap();
    assert_eq!(xt.loai_moi.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["Loại riêng B"]);
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    assert_eq!((bc.loai_moi, bc.loai_trung, bc.loai_loi), (vec!["Loại riêng B".to_string()], vec![trung.ten.clone()], vec!["Loại sai".to_string()]));
    let sau = loai_tai_lieu::doc(&f_loai);
    assert!(sau.contains(&moi));
    assert_eq!(sau.iter().find(|l| l.ten == trung.ten).unwrap(), &co_san[0], "trùng tên → giữ bản máy này");
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn xem_truoc_ty_le_khop_khi_mo_nham_thu_muc() {
    let (d, f_loai) = tam("ty_le");
    let a = thu_muc(&d, "A");
    let b = d.join("B");
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(b.join("a.pdf"), std::fs::read(a.join("a.pdf")).unwrap()).unwrap();
    std::fs::write(b.join("x.pdf"), b"khac").unwrap();
    let goi = d.join("A.zip");
    xuat_voi(&a, &goi, &f_loai).unwrap();
    let xt = xem_truoc_voi(&b, &goi, &f_loai).unwrap();
    assert_eq!((xt.so_pdf_goi, xt.so_pdf_khop), (3, 1));
    assert!((xt.ty_le_khop() - 1.0 / 3.0).abs() < 1e-6);
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn khoa_phien_chan_xuat_nhap_cung_luc() {
    let (d, f_loai) = tam("khoa");
    let a = thu_muc(&d, "A");
    let _k = KhoaPhien::lay(&thu_muc_an(&a)).unwrap();
    assert!(xuat_voi(&a, &d.join("x.zip"), &f_loai).unwrap_err().contains("cửa sổ khác"));
    drop(_k);
    assert!(xuat_voi(&a, &d.join("x.zip"), &f_loai).is_ok(), "nhả khoá → làm được");
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn goi_co_muc_trung_ten_bi_tu_choi() {
    let (d, f_loai) = tam("trung_ten");
    let a = thu_muc(&d, "A");
    let goi = d.join("trung.zip");
    // ZipWriter không cho 2 mục cùng tên: tạo "manifest.json" + "manifest.jsoX" rồi vá byte cho trùng tên thật
    tao_goi(&goi, &[(F_MANIFEST, &manifest_mau(1)), ("manifest.jsoX", &manifest_mau(1))]);
    let mut b = std::fs::read(&goi).unwrap();
    let mau = b"manifest.jsoX";
    let mut i = 0;
    while i + mau.len() <= b.len() {
        if &b[i..i + mau.len()] == mau {
            b[i + mau.len() - 1] = b'n';
        }
        i += 1;
    }
    std::fs::write(&goi, b).unwrap();
    assert!(nhap_voi(&a, &goi, &f_loai).unwrap_err().contains("trùng tên"));
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn file_nhan_may_nay_loi_thi_dung_khong_ghi_de() {
    let (d, f_loai) = tam("nhan_loi");
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    ghi_nhan(&b, &[("a.pdf", 2, Nhan::Index)]);
    ghi_moc(&b, &[("a.pdf", 100, 1)]);
    let goi = d.join("B.zip");
    xuat_voi(&b, &goi, &f_loai).unwrap();
    std::fs::create_dir_all(thu_muc_an(&a)).unwrap();
    let hong = "b.pdf;1;index\ndòng hỏng không đủ cột\n";
    std::fs::write(thu_muc_an(&a).join(F_NHAN), hong).unwrap();
    assert!(nhap_voi(&a, &goi, &f_loai).unwrap_err().contains("Lưu nhãn lại"));
    assert_eq!(std::fs::read_to_string(thu_muc_an(&a).join(F_NHAN)).unwrap(), hong, "không ghi đè mất nhãn của máy này");
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn trang_ngoai_so_trang_that_bi_bo_pdf_khac_kich_thuoc_bi_bo_qua() {
    let (d, f_loai) = tam("trang");
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    // Gói: a.pdf (3 trang) có nhãn trang 99; b.pdf ở máy nhận là bản khác (kích thước khác)
    ghi_an_toan(&thu_muc_an(&b).join(F_NHAN), "a.pdf;2;index\na.pdf;99;index\nb.pdf;1;index\n").unwrap();
    ghi_moc(&b, &[("a.pdf", 100, 1), ("b.pdf", 100, 2)]);
    let goi = d.join("B.zip");
    xuat_voi(&b, &goi, &f_loai).unwrap();
    tao_pdf(&a.join("b.pdf"), 5);
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    let n = doc_nhan(&a);
    assert_eq!(n["a.pdf"].keys().copied().collect::<Vec<_>>(), [2], "trang 99 không có trong PDF → bỏ");
    assert!(!n.contains_key("b.pdf") && bc.bo_qua.contains(&"b.pdf".to_string()), "PDF khác bản → bỏ qua: {bc:?}");
    // File nhãn sau gộp vẫn đọc được với số trang thật (giao diện mở được)
    let so = |k: &str| if k == "a.pdf" { Some(3) } else { Some(5) };
    assert!(nhan::doc_file_nhan(&std::fs::read_to_string(thu_muc_an(&a).join(F_NHAN)).unwrap(), &so).is_ok());
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn nhan_chua_co_moc_khong_bi_xoa_bia_mo_moi_hon_trong_goi_xoa_nhan_va_cache() {
    let (d, f_loai) = tam("chua_moc");
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    // Máy A: a.pdf có nhãn từ trước khi có mốc; b.pdf có nhãn + cache, mốc 100
    ghi_nhan(&a, &[("a.pdf", 2, Nhan::Index), ("b.pdf", 1, Nhan::Index)]);
    ghi_moc(&a, &[("b.pdf", 100, 5)]);
    let kb = cache_trang::KhoaFile::cua(&a.join("b.pdf"), "b.pdf").unwrap();
    cache_trang::ghi(&thu_muc_an(&a).join("cache"), &a.join("b.pdf"), &kb, &[Default::default(), Default::default()]).unwrap();
    // Gói B: a.pdf chỉ có dòng xoay, không mốc; b.pdf bia mộ mốc 200 (đã Phân tích lại); mốc a.pdf từ đồng hồ sai
    ghi_an_toan(&thu_muc_an(&b).join(F_XOAY), "a.pdf;1;90\n").unwrap();
    ghi_moc(&b, &[("b.pdf", 200, moc_sua::CHU_KY_RONG), ("c.pdf", u64::MAX, 9)]);
    let goi = d.join("B.zip");
    xuat_voi(&b, &goi, &f_loai).unwrap();
    let bc = nhap_voi(&a, &goi, &f_loai).unwrap();
    let n = doc_nhan(&a);
    assert_eq!(n["a.pdf"][&2], Nhan::Index, "gói không có mốc → không xoá nhãn máy này");
    assert!(!n.contains_key("b.pdf"), "bia mộ trong gói mới hơn → xoá nhãn");
    assert!(!cache_trang::file_cache(&thu_muc_an(&a).join("cache"), "b.pdf").exists(), "và bỏ cache cũ (cờ duyệt không trộn với nhãn mới)");
    assert_eq!(bc.lech_dong_ho, 1);
    assert!(moc_sua::doc(&thu_muc_an(&a).join(moc_sua::TEN_FILE))["c.pdf"].luc < u64::MAX, "mốc đồng hồ sai bị kẹp");
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}
