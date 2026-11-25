use std::sync::Mutex;

use super::*;
use crate::tach_vb::nhan::Nhan;
use crate::tach_vb::tach::tests::tao_pdf;

fn tam(ten: &str) -> (PathBuf, PathBuf) {
    let d = std::env::temp_dir().join(format!("dema4_phien_{ten}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f_loai = d.join("loai_tai_lieu.json");
    (d, f_loai)
}

/// Bộ PDF mẫu `a.pdf` (3 trang), `b.pdf`, `c.pdf` (2 trang) của `d`; `thu_muc` chép bộ này (giống hệt nhau — như
/// cùng hồ sơ trên 2 máy) vào `d/<duong>`.
fn thu_muc(d: &Path, duong: &str) -> PathBuf {
    let mau = d.join("_mau_pdf");
    if !mau.join("c.pdf").exists() {
        std::fs::create_dir_all(&mau).unwrap();
        tao_pdf(&mau.join("a.pdf"), 3);
        tao_pdf(&mau.join("b.pdf"), 2);
        tao_pdf(&mau.join("c.pdf"), 2);
    }
    let g = d.join(duong);
    std::fs::create_dir_all(&g).unwrap();
    for f in ["a.pdf", "b.pdf", "c.pdf"] {
        std::fs::copy(mau.join(f), g.join(f)).unwrap();
    }
    let _ = std::fs::remove_dir_all(thu_muc_an(&g));
    g
}

/// Kho riêng của 1 "máy".
fn may(d: &Path, ten: &str) -> Mutex<Kho> {
    Mutex::new(Kho::mo(&d.join(format!("{ten}.kho"))).unwrap())
}

fn id(k: &Mutex<Kho>, pdf: &Path) -> Id {
    kho::ids_cua_trong(k, &[pdf.to_path_buf()])[0].unwrap()
}

fn gan(k: &Mutex<Kho>, pdf: &Path, ds: &[(usize, Nhan)], luc: u64) {
    let i = id(k, pdf);
    let nhan: BTreeMap<usize, Nhan> = ds.iter().copied().collect();
    let chu_ky = kho::chu_ky_tach(&nhan, &Default::default(), &Default::default());
    k.lock().unwrap().ghi_tach(i, &BanTach { nhan, luc, chu_ky, ..Default::default() }).unwrap();
}

fn tach(k: &Mutex<Kho>, pdf: &Path) -> Option<BanTach> {
    let i = id(k, pdf);
    k.lock().unwrap().tach(i)
}

fn ban_ghi(file: &str, loai: &str, so: &str, luc: u64, loi: bool) -> BanGhi {
    let mut gia_tri = BTreeMap::new();
    gia_tri.insert("so".to_string(), so.to_string());
    BanGhi { file: file.into(), loai: loai.into(), so_trang: 1, gia_tri, loi: loi.then(|| "500".into()), sua: vec![], canh_bao: None, giay: 0.0, tho: vec![], luc, id: None }
}

fn ten_trong_goi(f: &Path) -> Vec<String> {
    let z = zip::ZipArchive::new(std::fs::File::open(f).unwrap()).unwrap();
    z.file_names().map(|n| n.unwrap().into_owned()).collect()
}

/// Gói tự dựng (test gói độc / phiên bản lạ / gói bản 2.3).
fn tao_goi(f: &Path, muc: &[(&str, &[u8])]) {
    let mut z = zip::ZipWriter::new(std::fs::File::create(f).unwrap());
    for (ten, b) in muc {
        z.start_file(*ten, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b).unwrap();
    }
    z.finish().unwrap();
}

fn manifest(phien_ban: u32, pdf: Vec<PdfGoi>) -> Vec<u8> {
    serde_json::to_vec(&Manifest { phien_ban_goi: phien_ban, phien_ban_app: "x".into(), ten_thu_muc: "B".into(), tao_luc: 1, pdf, loai_tai_lieu: vec![] }).unwrap()
}

/// Máy A làm ở `P` (nhãn, phân tích, kết quả bóc bản cũ không id) → xuất; máy B có cùng hồ sơ ở chỗ khác, tên khác,
/// mở ở cấp thư mục con → nhập: khớp theo nội dung; gói chỉ có đúng danh sách trắng.
#[test]
fn xuat_nhap_khac_may_khop_theo_noi_dung_o_cap_thu_muc_khac() {
    let (d, f_loai) = tam("tron_vong");
    let (ka, kb) = (may(&d, "A"), may(&d, "B"));
    let p = d.join("A/P");
    let con = thu_muc(&d, "A/P/Con");
    gan(&ka, &con.join("a.pdf"), &[(2, Nhan::Index)], 1_000);
    let ia = id(&ka, &con.join("a.pdf"));
    ka.lock().unwrap().ghi_phan_tich(ia, &vec![cache_trang::TrangCache::default(); 3]);
    ka.lock().unwrap().day().unwrap();
    luu::ghi_them(&luu::file_jsonl(&p), &ban_ghi("Con/b.pdf", "VB", "15/TTr", 1_000, false)).unwrap();
    let goi = d.join("P.zip");
    let tt = xuat_trong(&p, &goi, &f_loai, &ka).unwrap();
    assert_eq!((tt.so_ho_so, tt.so_ban_ghi_boc), (1, 1));
    let mut ten = ten_trong_goi(&goi);
    ten.sort();
    assert_eq!(ten, [F_JSONL, F_MANIFEST, F_PHAN_TICH, F_TACH], "chỉ đúng danh sách trắng");

    let con_b = thu_muc(&d, "B/Khac/Con_doi_ten");
    let xt = xem_truoc_trong(&con_b, &goi, &f_loai, &kb).unwrap();
    assert_eq!((xt.so_pdf_khop, xt.so_pdf_goi, xt.so_ho_so), (3, 3, 1), "{xt:?}");
    let bc = nhap_trong(&con_b, &goi, &f_loai, &kb).unwrap();
    assert_eq!((bc.nhan, bc.boc_nhan, bc.bo_qua.len()), (1, 1, 0), "{bc:?}");
    assert_eq!(tach(&kb, &con_b.join("a.pdf")).unwrap().nhan.get(&2), Some(&Nhan::Index));
    assert_eq!(kb.lock().unwrap().phan_tich(ia).map(|t| t.len()), Some(3), "kết quả phân tích sang theo");
    let bg = luu::doc_tat_ca(&std::fs::read_to_string(luu::file_jsonl(&con_b)).unwrap());
    assert!(bg.contains_key(&("b.pdf".to_string(), "VB".to_string())), "kết quả bóc theo đường dẫn mới: {bg:?}");
    let _ = std::fs::remove_dir_all(thu_muc_an(&p));
    let _ = std::fs::remove_dir_all(thu_muc_an(&con_b));
    let _ = std::fs::remove_dir_all(&d);
}

/// 2 máy cùng sửa: mốc sau cùng thắng cả 2 chiều; bia mộ (Phân tích lại) mới hơn → máy nhận bỏ nhãn; hồ sơ gói
/// không có → máy nhận giữ nguyên.
#[test]
fn hai_may_sau_cung_thang_bia_mo_khong_dong_ho_so_khac() {
    let (d, f_loai) = tam("hai_may");
    let (ka, kb) = (may(&d, "A"), may(&d, "B"));
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    gan(&ka, &a.join("a.pdf"), &[(2, Nhan::Index)], 2_000);
    gan(&kb, &b.join("a.pdf"), &[(3, Nhan::Index)], 1_000);
    gan(&ka, &a.join("b.pdf"), &[(1, Nhan::Index)], 1_000);
    gan(&kb, &b.join("b.pdf"), &[(2, Nhan::Index)], 3_000);
    gan(&kb, &b.join("c.pdf"), &[(1, Nhan::Trang)], 1_000);
    let goi = d.join("A.zip");
    xuat_trong(&a, &goi, &f_loai, &ka).unwrap();
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    assert_eq!((bc.nhan, bc.giu, bc.trung.len()), (1, 1, 2), "{bc:?}");
    assert!(tach(&kb, &b.join("a.pdf")).unwrap().nhan.contains_key(&2), "A mới hơn → lấy");
    assert!(tach(&kb, &b.join("b.pdf")).unwrap().nhan.contains_key(&2), "B mới hơn → giữ");
    assert!(tach(&kb, &b.join("c.pdf")).unwrap().nhan.contains_key(&1), "gói không có → giữ");
    // A Phân tích lại (bia mộ mới hơn) → nhập: B bỏ nhãn a.pdf
    let ia = id(&ka, &a.join("a.pdf"));
    ka.lock().unwrap().ghi_tach(ia, &BanTach { luc: 9_000, chu_ky: moc_sua::CHU_KY_RONG, bia_mo: true, ..Default::default() }).unwrap();
    xuat_trong(&a, &goi, &f_loai, &ka).unwrap();
    nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    let t = tach(&kb, &b.join("a.pdf")).unwrap();
    assert!(t.bia_mo && t.nhan.is_empty(), "{t:?}");
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn boc_tach_loi_moi_hon_khong_de_ket_qua_tot_hai_loai_cung_giu() {
    let (d, f_loai) = tam("boc");
    let (ka, kb) = (may(&d, "A"), may(&d, "B"));
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    for (f, l, so, luc, loi) in [("a.pdf", "VB", "", 9, true), ("b.pdf", "VB", "16", 9, false), ("c.pdf", "HD", "1", 1, false)] {
        luu::ghi_them(&luu::file_jsonl(&a), &ban_ghi(f, l, so, luc, loi)).unwrap();
    }
    for (f, l, so, luc, loi) in [("a.pdf", "VB", "15/TTr", 1, false), ("b.pdf", "VB", "15", 1, false), ("c.pdf", "VB", "x", 1, false)] {
        luu::ghi_them(&luu::file_jsonl(&b), &ban_ghi(f, l, so, luc, loi)).unwrap();
    }
    let goi = d.join("A.zip");
    xuat_trong(&a, &goi, &f_loai, &ka).unwrap();
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    let bg = luu::doc_tat_ca(&std::fs::read_to_string(luu::file_jsonl(&b)).unwrap());
    let gt = |f: &str, l: &str| bg[&(f.to_string(), l.to_string())].gia_tri["so"].clone();
    assert_eq!((gt("a.pdf", "VB"), gt("b.pdf", "VB"), gt("c.pdf", "VB"), gt("c.pdf", "HD")), ("15/TTr".into(), "16".into(), "x".into(), "1".into()));
    assert_eq!((bc.boc_nhan, bc.boc_giu), (2, 1), "{bc:?}");
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Gói độc: khoá lạ, PDF không có trên máy → không ghi gì; gói từ bản mới hơn / zip cắt cụt / mục trùng tên → từ chối.
#[test]
fn goi_doc_phien_ban_moi_hon_cat_cut_trung_ten_bi_tu_choi() {
    let (d, f_loai) = tam("doc");
    let kb = may(&d, "B");
    let b = thu_muc(&d, "B");
    let nhan_doc = "../x.pdf;1;index\n/etc/x.pdf;1;index\nC:/x.pdf;1;index\nkhong_co.pdf;1;index\n";
    let moc_doc = "../x.pdf;9;1\n/etc/x.pdf;9;1\nC:/x.pdf;9;1\nkhong_co.pdf;9;1\n";
    let pdf = vec![PdfGoi { khoa: "khong_co.pdf".into(), kich_thuoc: 1, ma: Some(1) }];
    let goi = d.join("doc.zip");
    tao_goi(&goi, &[(F_MANIFEST, &manifest(1, pdf)), (F_NHAN, nhan_doc.as_bytes()), (moc_sua::TEN_FILE, moc_doc.as_bytes())]);
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    assert_eq!((bc.nhan, bc.boc_nhan, bc.bo_qua.len()), (0, 0, 1), "{bc:?}");
    assert_eq!(kb.lock().unwrap().so_khoa(), 0, "không ghi gì vào kho");
    let moi = d.join("moi.zip");
    tao_goi(&moi, &[(F_MANIFEST, &manifest(PHIEN_BAN_GOI + 1, vec![]))]);
    assert!(nhap_trong(&b, &moi, &f_loai, &kb).unwrap_err().contains("mới hơn"));
    let byte = std::fs::read(&goi).unwrap();
    let cut = d.join("cut.zip");
    std::fs::write(&cut, &byte[..byte.len() / 2]).unwrap();
    assert!(nhap_trong(&b, &cut, &f_loai, &kb).is_err());
    // ZipWriter không cho 2 mục cùng tên: tạo "manifest.json" + "manifest.jsoX" rồi vá byte cho trùng tên thật
    let trung = d.join("trung.zip");
    tao_goi(&trung, &[(F_MANIFEST, &manifest(2, vec![])), ("manifest.jsoX", &manifest(2, vec![]))]);
    let mut byte = std::fs::read(&trung).unwrap();
    let mau = b"manifest.jsoX";
    for i in 0..byte.len().saturating_sub(mau.len()) {
        if &byte[i..i + mau.len()] == mau {
            byte[i + mau.len() - 1] = b'n';
        }
    }
    std::fs::write(&trung, byte).unwrap();
    assert!(nhap_trong(&b, &trung, &f_loai, &kb).unwrap_err().contains("trùng tên"));
    let _ = std::fs::remove_dir_all(thu_muc_an(&b));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn loai_tai_lieu_chi_them_loai_moi_hop_le_trung_ten_giu_may_nay() {
    let (d, f_loai) = tam("loai");
    let kb = may(&d, "B");
    let b = thu_muc(&d, "B");
    let mut co_san = loai_tai_lieu::doc(&f_loai);
    let mut rieng = co_san[0].clone();
    rieng.ten = "Loại riêng A".into();
    let mut sua = co_san[0].clone();
    sua.huong_dan = "sửa khác".into();
    let mut hong = co_san[0].clone();
    hong.ten = "Hỏng".into();
    hong.truong.clear();
    let m = serde_json::to_vec(&Manifest { phien_ban_goi: 2, phien_ban_app: "x".into(), ten_thu_muc: "B".into(), tao_luc: 1, pdf: vec![], loai_tai_lieu: vec![rieng.clone(), sua.clone(), hong] }).unwrap();
    let goi = d.join("loai.zip");
    tao_goi(&goi, &[(F_MANIFEST, &m)]);
    let xt = xem_truoc_trong(&b, &goi, &f_loai, &kb).unwrap();
    assert_eq!(xt.loai_moi.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["Loại riêng A"]);
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    assert_eq!((bc.loai_moi.len(), bc.loai_trung.len(), bc.loai_loi.len()), (1, 1, 1), "{bc:?}");
    co_san.push(rieng);
    assert_eq!(loai_tai_lieu::doc(&f_loai), co_san, "thêm loại mới, giữ loại cùng tên của máy này");
    let _ = std::fs::remove_dir_all(thu_muc_an(&b));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn mo_nham_thu_muc_ty_le_khop_thap_pdf_khac_noi_dung_khong_nhan() {
    let (d, f_loai) = tam("nham");
    let (ka, kb) = (may(&d, "A"), may(&d, "B"));
    let a = thu_muc(&d, "A");
    gan(&ka, &a.join("a.pdf"), &[(2, Nhan::Index)], 1_000);
    let goi = d.join("A.zip");
    xuat_trong(&a, &goi, &f_loai, &ka).unwrap();
    // Thư mục khác: cùng tên file, khác nội dung
    let khac = d.join("Khac");
    std::fs::create_dir_all(&khac).unwrap();
    for (f, n) in [("a.pdf", 3), ("b.pdf", 2), ("c.pdf", 2)] {
        tao_pdf(&khac.join(f), n);
    }
    let xt = xem_truoc_trong(&khac, &goi, &f_loai, &kb).unwrap();
    assert_eq!((xt.so_pdf_khop, xt.ty_le_khop()), (0, 0.0));
    let bc = nhap_trong(&khac, &goi, &f_loai, &kb).unwrap();
    assert_eq!((bc.nhan, bc.bo_qua.len()), (0, 3), "{bc:?}");
    assert!(tach(&kb, &khac.join("a.pdf")).is_none(), "trùng tên khác nội dung → không nhận");
    for g in [&a, &khac] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn khoa_phien_chan_xuat_nhap_cung_luc() {
    let (d, f_loai) = tam("khoa");
    let ka = may(&d, "A");
    let a = thu_muc(&d, "A");
    let _k = KhoaPhien::lay(&thu_muc_an(&a)).unwrap();
    assert!(xuat_trong(&a, &d.join("x.zip"), &f_loai, &ka).unwrap_err().contains("cửa sổ khác"));
    drop(_k);
    assert!(xuat_trong(&a, &d.join("x.zip"), &f_loai, &ka).is_ok(), "nhả khoá → làm được");
    let _ = std::fs::remove_dir_all(thu_muc_an(&a));
    let _ = std::fs::remove_dir_all(&d);
}

/// Trang ngoài số trang thật bị bỏ; mốc từ đồng hồ máy gửi sai (vượt xa giờ máy nhận) bị kẹp.
#[test]
fn trang_ngoai_so_trang_bi_bo_dong_ho_sai_bi_kep() {
    let (d, f_loai) = tam("trang");
    let (ka, kb) = (may(&d, "A"), may(&d, "B"));
    let (a, b) = (thu_muc(&d, "A"), thu_muc(&d, "B"));
    gan(&ka, &a.join("a.pdf"), &[(2, Nhan::Index), (99, Nhan::Index)], u64::MAX);
    let goi = d.join("A.zip");
    xuat_trong(&a, &goi, &f_loai, &ka).unwrap();
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    let t = tach(&kb, &b.join("a.pdf")).unwrap();
    assert_eq!((t.nhan.keys().copied().collect::<Vec<_>>(), t.so_trang), (vec![2], 3), "trang 99 không có trong PDF → bỏ");
    assert!(bc.lech_dong_ho == 1 && t.luc < u64::MAX, "mốc bị kẹp về giờ máy nhận");
    for g in [&a, &b] {
        let _ = std::fs::remove_dir_all(thu_muc_an(g));
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Gói bản 2.3 (theo đường dẫn: file nhãn + mốc + cache) vẫn nhập được; hồ sơ không có mốc trong gói không ghi đè.
#[test]
fn goi_ban_cu_theo_duong_dan_van_nhap_duoc() {
    let (d, f_loai) = tam("ban_cu");
    let kb = may(&d, "B");
    let b = thu_muc(&d, "B");
    let (na, nb) = (std::fs::metadata(b.join("a.pdf")).unwrap().len(), std::fs::metadata(b.join("b.pdf")).unwrap().len());
    gan(&kb, &b.join("b.pdf"), &[(1, Nhan::Trang)], 1_000);
    let kf = cache_trang::KhoaFile { duong_dan: "a.pdf".into(), kich_thuoc: na, sua_luc: 1, van_tay: 1 };
    let cache = cache_trang::ma_hoa(&kf, &vec![cache_trang::TrangCache::default(); 3]);
    // 2.3.0: không có mã nội dung → khớp theo đường dẫn + kích thước
    let pdf = vec![PdfGoi { khoa: "a.pdf".into(), kich_thuoc: na, ma: None }, PdfGoi { khoa: "b.pdf".into(), kich_thuoc: nb, ma: None }];
    let goi = d.join("cu.zip");
    tao_goi(&goi, &[(F_MANIFEST, &manifest(1, pdf)), (F_NHAN, b"a.pdf;2;index\nb.pdf;2;index\n"), (moc_sua::TEN_FILE, b"a.pdf;5000;1\n"), (&ten_muc_cache("a.pdf"), &cache)]);
    let bc = nhap_trong(&b, &goi, &f_loai, &kb).unwrap();
    assert_eq!(bc.nhan, 1, "{bc:?}");
    assert_eq!(tach(&kb, &b.join("a.pdf")).unwrap().nhan.get(&2), Some(&Nhan::Index));
    assert!(tach(&kb, &b.join("b.pdf")).unwrap().nhan.contains_key(&1), "gói không có mốc cho b.pdf → giữ máy này");
    let _ = std::fs::remove_dir_all(thu_muc_an(&b));
    let _ = std::fs::remove_dir_all(&d);
}

/// Đo cả luồng với kho theo id: N PDF (mặc định 100.000 × 20 trang, 100 thư mục con × 1.000, mỗi file khác nội dung).
/// Chạy: DEMA4_DO=<thư mục tạm> [DEMA4_SO_FILE=100000] cargo test --release -- --ignored do_hieu_nang_kho --nocapture
#[test]
#[ignore = "đo hiệu năng — tạo vài trăm MB dữ liệu thử"]
fn do_hieu_nang_kho() {
    use crate::tach_vb::phien::PhienPhanTich;
    use std::time::Instant;
    let goc_do = PathBuf::from(std::env::var("DEMA4_DO").expect("DEMA4_DO"));
    let n: usize = std::env::var("DEMA4_SO_FILE").ok().and_then(|v| v.parse().ok()).unwrap_or(100_000);
    let _ = std::fs::remove_dir_all(&goc_do);
    std::fs::create_dir_all(&goc_do).unwrap();
    let in_ra = |ten: &str, gt: String| println!("DO|{ten}|{gt}");
    let giay = |t: Instant| format!("{:.2} s", t.elapsed().as_secs_f64());
    let dung = std::sync::atomic::AtomicBool::new(false);
    // Mẫu 20 trang + kết quả phân tích thật
    let mau_dir = goc_do.join("mau");
    std::fs::create_dir_all(&mau_dir).unwrap();
    tao_pdf(&mau_dir.join("mau.pdf"), 20);
    let ch = crate::tach_vb::phan_tich::CauHinhPhanTich::default();
    let trang_mau = crate::tach_vb::phien::phan_tich_thu_muc(&mau_dir, &ch, &|_, _| {}).ho_so.remove(0).trang;
    let mau = std::fs::read(mau_dir.join("mau.pdf")).unwrap();
    // N PDF khác nội dung (dòng chú thích cuối khác nhau)
    let p = goc_do.join("P");
    let t = Instant::now();
    let mut ds = Vec::with_capacity(n);
    for i in 0..n {
        let f = p.join(format!("Con_{:03}", i / 1000)).join(format!("f_{:05}.pdf", i % 1000));
        if i % 1000 == 0 {
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        }
        let mut b = mau.clone();
        b.extend(format!("% {i:012}\n").bytes());
        std::fs::write(&f, b).unwrap();
        ds.push(f);
    }
    in_ra("tao_pdf", giay(t));
    // Lần đầu: tính id (đọc nội dung) + ghi phân tích + nhãn vào kho (như đã làm xong ở các thư mục con)
    let t = Instant::now();
    let ids = kho::ids_cua(&ds);
    in_ra("tinh_id_lan_dau", giay(t));
    let t = Instant::now();
    {
        let mut k = kho::kho();
        for id in ids.iter().flatten() {
            k.ghi_phan_tich(*id, &trang_mau);
            let nhan: BTreeMap<usize, Nhan> = [(1, Nhan::Index), (12, Nhan::Index)].into_iter().collect();
            let chu_ky = kho::chu_ky_tach(&nhan, &Default::default(), &Default::default());
            k.them_tach(*id, &BanTach { so_trang: 20, nhan, luc: 1, chu_ky, ..Default::default() });
        }
        k.day().unwrap();
        in_ra("kho_sau_ghi", format!("{:.1} MB", k.dung_luong() as f64 / 1e6));
    }
    in_ra("ghi_phan_tich_va_nhan_vao_kho", giay(t));
    // Mở thư mục cha: id theo bảng đường dẫn + đọc phân tích từ kho
    let t = Instant::now();
    let ph = PhienPhanTich::mo(&p, &dung);
    let co = ph.ho_so.iter().filter(|h| h.trang.iter().all(|t| t.ket_qua.is_some())).count();
    in_ra("mo_thu_muc_cha", format!("{} ({} hồ sơ, {co} có kết quả phân tích)", giay(t), ph.ho_so.len()));
    let t = Instant::now();
    let _ = PhienPhanTich::mo(&p, &dung);
    in_ra("mo_lai_thu_muc_cha", giay(t));
    // Đổi tên 1 thư mục con (1.000 PDF) rồi mở lại: id của 1.000 file tính lại theo nội dung
    std::fs::rename(p.join("Con_000"), p.join("Con_000_doi_ten")).unwrap();
    let t = Instant::now();
    let ph = PhienPhanTich::mo(&p, &dung);
    let co = ph.ho_so.iter().filter(|h| h.tuong_doi.starts_with("Con_000_doi_ten/") && h.trang.iter().all(|t| t.ket_qua.is_some())).count();
    in_ra("mo_sau_doi_ten_1_thu_muc_con", format!("{} ({co}/1000 hồ sơ đổi tên vẫn có kết quả)", giay(t)));
    // Sửa nhãn 1 hồ sơ: 1 bản ghi ghi ngay
    let id0 = ph.ho_so[0].id.unwrap();
    let truoc = kho::kho().dung_luong();
    let t = Instant::now();
    kho::kho().ghi_tach(id0, &BanTach { so_trang: 20, nhan: [(2, Nhan::Index)].into_iter().collect(), luc: 2, chu_ky: 5, ..Default::default() }).unwrap();
    in_ra("sua_1_nhan", format!("{:.1} ms, ghi {} byte", t.elapsed().as_secs_f64() * 1000.0, kho::kho().dung_luong() - truoc));
    // Xuất / nhập sang máy khác (kho riêng)
    let t = Instant::now();
    let tt = xuat_trong(&p, &goc_do.join("P.zip"), &goc_do.join("loai.json"), kho::chung()).unwrap();
    in_ra("xuat_phien", format!("{} ({:.1} MB, {} hồ sơ)", giay(t), tt.dung_luong as f64 / 1e6, tt.so_ho_so));
    let kb = Mutex::new(Kho::mo(&goc_do.join("B.kho")).unwrap());
    let t = Instant::now();
    let bc = nhap_trong(&p, &goc_do.join("P.zip"), &goc_do.join("loai.json"), &kb).unwrap();
    in_ra("nhap_phien_may_khac", format!("{} ({} hồ sơ)", giay(t), bc.nhan));
    let t = Instant::now();
    let mut k = kho::kho();
    let truoc = k.dung_luong();
    k.nen().unwrap();
    in_ra("don_kho", format!("{} ({:.1} → {:.1} MB)", giay(t), truoc as f64 / 1e6, k.dung_luong() as f64 / 1e6));
    drop(k);
    let _ = std::fs::remove_dir_all(&goc_do);
}
