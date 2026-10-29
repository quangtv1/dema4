//! Lệnh tách: từ hồ sơ + nhãn + góc xoay → ghi các văn bản PDF và dòng log `ketqua_tach.txt`.
//!
//! Thư mục ra `<cha>/<tên nguồn>_vanban` giữ nguyên cấu trúc thư mục nguồn và thêm 1 tầng thư mục hồ sơ:
//! `mau2/HOP 462/4766/4766.pdf` → `mau2_vanban/HOP 462/4766/4766/4766_part1.pdf`.

use super::cat_pdf::cat_bang_ghep;
use super::nhan::{tinh_dai, Nhan, Xoay};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAU_TEN_MAC_DINH: &str = "{hoso}_part{n}";
pub const TIEU_DE_LOG: &str = "STT\tFile đầu vào\tDải trang\tFile đầu ra\tTrang trắng đã bỏ\tSố trang";

/// Thư mục ra: `<cha>/<tên nguồn>_vanban`; `cha` trống = cha của thư mục nguồn. Không được nằm trong nguồn.
pub fn thu_muc_ra(nguon: &Path, cha: Option<&Path>) -> Result<PathBuf, String> {
    let ten = nguon.file_name().ok_or("thư mục nguồn không có tên")?.to_string_lossy().into_owned();
    let cha = match cha {
        Some(c) => c.to_path_buf(),
        None => nguon.parent().ok_or("thư mục nguồn không có thư mục cha")?.to_path_buf(),
    };
    let ra = cha.join(format!("{ten}_vanban"));
    if ra.starts_with(nguon) {
        return Err(format!("thư mục lưu kết quả {} nằm trong thư mục nguồn — lần quét sau sẽ lẫn kết quả", ra.display()));
    }
    Ok(ra)
}

/// Mẫu tên văn bản phải có `{n}` hoặc `{n3}` để các văn bản không trùng tên.
pub fn kiem_mau_ten(mau: &str) -> Result<(), String> {
    if mau.contains("{n}") || mau.contains("{n3}") { Ok(()) } else { Err("mẫu tên văn bản phải có {n} hoặc {n3}".to_string()) }
}

/// Tên file văn bản (kèm `.pdf`) theo mẫu; ký tự không hợp lệ cho tên file thay bằng `_`.
pub fn ten_van_ban(mau: &str, ho_so: &str, n: usize) -> String {
    let ten = mau.replace("{hoso}", ho_so).replace("{n3}", &format!("{n:03}")).replace("{n}", &n.to_string());
    let sach: String = ten.chars().map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c }).collect();
    format!("{sach}.pdf")
}

/// Tên file có phải 1 văn bản cũ sinh theo mẫu này không (để dọn part thừa của lần tách trước).
fn khop_mau(mau: &str, ho_so: &str, ten_file: &str) -> bool {
    let Some(goc) = ten_file.strip_suffix(".pdf") else { return false };
    let mau = mau.replace("{hoso}", ho_so);
    let (cho, rong) = if mau.contains("{n3}") { ("{n3}", 3) } else { ("{n}", 0) };
    let Some((truoc, sau)) = mau.split_once(cho) else { return false };
    let truoc: String = ten_van_ban(truoc, "", 0).trim_end_matches(".pdf").to_string();
    let sau: String = ten_van_ban(sau, "", 0).trim_end_matches(".pdf").to_string();
    let Some(giua) = goc.strip_prefix(&truoc).and_then(|x| x.strip_suffix(&sau)) else { return false };
    !giua.is_empty() && giua.bytes().all(|b| b.is_ascii_digit()) && (rong == 0 || giua.len() >= rong)
}

/// 1 hồ sơ cần tách: file PDF + nhãn + góc xoay (trang 1-based).
pub struct HoSoTach {
    pub pdf: PathBuf,
    pub nhan: BTreeMap<usize, Nhan>,
    pub xoay: BTreeMap<usize, Xoay>,
}

/// 1 dòng log (chưa có STT): văn bản đã ghi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DongLog {
    pub file_vao: PathBuf,
    pub tu: usize,
    pub den: usize,
    pub file_ra: PathBuf,
    pub trang_bo: Vec<usize>,
    pub so_trang: usize,
}

#[derive(Debug, Default)]
pub struct KetQuaTach {
    pub dong: Vec<DongLog>,
    /// Hồ sơ lỗi: (file, lý do) — không chặn hồ sơ khác.
    pub loi: Vec<(PathBuf, String)>,
    /// Số hồ sơ chép nguyên file (không nhãn, không xoay).
    pub sao_nguyen: usize,
    pub canh_bao: Vec<String>,
}

fn so_trang_pdf(pdf: &Path) -> Result<usize, String> {
    let doc = mupdf::Document::open(pdf.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(|e| format!("không mở được: {e}"))?;
    Ok(doc.page_count().map_err(|e| e.to_string())? as usize)
}

/// Tách 1 hồ sơ vào `thu_muc_ho_so` (= `<ra>/<thư mục con>/<hồ sơ>/`). Trả các dòng log + có chép nguyên không.
fn tach_ho_so(ho_so: &HoSoTach, thu_muc_ho_so: &Path, mau: &str) -> Result<(Vec<DongLog>, bool, Vec<String>), String> {
    let ten_ho_so = ho_so.pdf.file_stem().ok_or("tên file không hợp lệ")?.to_string_lossy().into_owned();
    let so_trang = so_trang_pdf(&ho_so.pdf)?;
    if let Some(&t) = ho_so.nhan.keys().chain(ho_so.xoay.keys()).filter(|&&t| t > so_trang).max() {
        return Err(format!("nhãn / góc xoay ở trang {t} nhưng hồ sơ chỉ có {so_trang} trang"));
    }
    let dai = tinh_dai(so_trang, &ho_so.nhan, &ho_so.xoay);
    std::fs::create_dir_all(thu_muc_ho_so).map_err(|e| format!("không tạo được thư mục {}: {e}", thu_muc_ho_so.display()))?;
    let mut dong = Vec::new();
    let mut da_ghi = std::collections::HashSet::new();
    for vb in &dai.van_ban {
        let ten = ten_van_ban(mau, &ten_ho_so, vb.so);
        let ra = thu_muc_ho_so.join(&ten);
        if dai.nguyen_ven {
            // Không nhãn, không xoay: chép từng byte (giữ cả chữ ký số), không ghi lại bằng MuPDF
            std::fs::copy(&ho_so.pdf, &ra).map_err(|e| format!("không chép được {}: {e}", ra.display()))?;
        } else {
            cat_bang_ghep(&ho_so.pdf, &vb.trang_giu, &ra).map_err(|e| format!("không ghi được {}: {e}", ra.display()))?;
        }
        da_ghi.insert(ten);
        dong.push(DongLog { file_vao: ho_so.pdf.clone(), tu: vb.tu, den: vb.den, file_ra: ra, trang_bo: vb.bo.clone(), so_trang: vb.trang_giu.len() });
    }
    // Dọn văn bản cũ của lần tách trước không còn sinh ra (chỉ file khớp mẫu tên đang áp)
    if let Ok(ds) = std::fs::read_dir(thu_muc_ho_so) {
        for muc in ds.flatten() {
            let ten = muc.file_name().to_string_lossy().into_owned();
            if !da_ghi.contains(&ten) && khop_mau(mau, &ten_ho_so, &ten) {
                let _ = std::fs::remove_file(muc.path());
            }
        }
    }
    Ok((dong, dai.nguyen_ven, dai.canh_bao.iter().map(|c| format!("{}: {c}", ho_so.pdf.display())).collect()))
}

/// Tách mọi hồ sơ (song song theo hồ sơ). `nguon` = thư mục đầu vào (để giữ cấu trúc thư mục con), `ra` = thư mục ra.
/// `tien_do(đã xong, tổng)` gọi sau mỗi hồ sơ; `dung` bật → bỏ các hồ sơ chưa bắt đầu.
pub fn tach_thu_muc(nguon: &Path, ra: &Path, ho_so: &[HoSoTach], mau: &str, dung: &AtomicBool, tien_do: &(dyn Fn(usize, usize) + Sync)) -> Result<KetQuaTach, String> {
    kiem_mau_ten(mau)?;
    let xong = std::sync::atomic::AtomicUsize::new(0);
    let tung: Vec<(usize, Result<(Vec<DongLog>, bool, Vec<String>), String>)> = ho_so
        .par_iter()
        .enumerate()
        .filter(|_| !dung.load(Ordering::Relaxed))
        .map(|(i, hs)| {
            let con = hs.pdf.parent().and_then(|p| p.strip_prefix(nguon).ok()).unwrap_or(Path::new(""));
            let ten = hs.pdf.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let kq = tach_ho_so(hs, &ra.join(con).join(ten), mau);
            tien_do(xong.fetch_add(1, Ordering::Relaxed) + 1, ho_so.len());
            (i, kq)
        })
        .collect();
    let mut kq = KetQuaTach::default();
    let mut tung = tung;
    tung.sort_by_key(|x| x.0);
    for (i, r) in tung {
        match r {
            Ok((dong, nguyen, cb)) => {
                kq.dong.extend(dong);
                kq.sao_nguyen += nguyen as usize;
                kq.canh_bao.extend(cb);
            }
            Err(e) => kq.loi.push((ho_so[i].pdf.clone(), e)),
        }
    }
    Ok(kq)
}

/// Ghép hồ sơ tìm thấy trong `thu_muc` với nhãn + góc xoay đọc từ file.
/// `goc_nhan` = thư mục gốc để hiểu đường dẫn tương đối trong file nhãn; file xoay luôn tương đối với `thu_muc`.
/// Trả hồ sơ + cảnh báo (dòng nhãn trỏ tới file không có trong thư mục).
pub fn ghep_ho_so(pdf: &[PathBuf], nhan: super::nhan::BangNhan, goc_nhan: &Path, xoay: super::nhan::BangXoay, thu_muc: &Path) -> (Vec<HoSoTach>, Vec<String>) {
    use super::nhan::duong_dan_that;
    let chuan = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut theo_file: std::collections::HashMap<PathBuf, HoSoTach> = pdf.iter().map(|p| (chuan(p), HoSoTach { pdf: p.clone(), nhan: BTreeMap::new(), xoay: BTreeMap::new() })).collect();
    let mut canh_bao = Vec::new();
    for (p, n) in nhan {
        match theo_file.get_mut(&chuan(&duong_dan_that(goc_nhan, &p))) {
            Some(hs) => hs.nhan = n,
            None => canh_bao.push(format!("file nhãn có {p} nhưng không thấy trong thư mục — bỏ qua")),
        }
    }
    for (p, x) in xoay {
        match theo_file.get_mut(&chuan(&duong_dan_that(thu_muc, &p))) {
            Some(hs) => hs.xoay = x,
            None => canh_bao.push(format!("file xoay có {p} nhưng không thấy trong thư mục — bỏ qua")),
        }
    }
    let mut ho_so: Vec<HoSoTach> = theo_file.into_values().collect();
    ho_so.sort_by(|a, b| a.pdf.cmp(&b.pdf));
    (ho_so, canh_bao)
}

/// Nội dung `ketqua_tach.txt` (UTF-8, tab, có tiêu đề, STT liên tục; đường dẫn file ra tuyệt đối; trang 1-based).
pub fn noi_dung_log(dong: &[DongLog]) -> String {
    let mut s = format!("{TIEU_DE_LOG}\n");
    for (i, d) in dong.iter().enumerate() {
        let bo = d.trang_bo.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
        s.push_str(&format!("{}\t{}\t{}-{}\t{}\t[{bo}]\t{}\n", i + 1, d.file_vao.display(), d.tu, d.den, d.file_ra.display(), d.so_trang));
    }
    s
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mupdf::pdf::PdfDocument;

    /// PDF `so_trang` trang A4, mỗi trang chữ "Trang n" (nội dung khác nhau để so) — ghi tay đủ bảng xref.
    pub(crate) fn tao_pdf(duong_dan: &Path, so_trang: usize) {
        let mut obj: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            String::new(), // Pages, điền sau
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
        ];
        let mut kids = Vec::new();
        for n in 1..=so_trang {
            let noi_dung = format!("BT /F1 24 Tf 72 720 Td (Trang {n}) Tj ET");
            obj.push(format!("<< /Length {} >>\nstream\n{noi_dung}\nendstream", noi_dung.len()));
            let so_noi_dung = obj.len();
            obj.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {so_noi_dung} 0 R >>"));
            kids.push(format!("{} 0 R", obj.len()));
        }
        obj[1] = format!("<< /Type /Pages /Kids [{}] /Count {so_trang} >>", kids.join(" "));
        let mut ra = b"%PDF-1.4\n".to_vec();
        let mut vi_tri = Vec::new();
        for (i, o) in obj.iter().enumerate() {
            vi_tri.push(ra.len());
            ra.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
        }
        let xref = ra.len();
        ra.extend(format!("xref\n0 {}\n0000000000 65535 f \n", obj.len() + 1).bytes());
        for v in vi_tri {
            ra.extend(format!("{v:010} 00000 n \n").bytes());
        }
        ra.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", obj.len() + 1).bytes());
        std::fs::write(duong_dan, ra).unwrap();
    }

    /// Chữ trên từng trang của PDF (để biết trang nào nằm trong văn bản).
    fn chu_cac_trang(p: &Path) -> Vec<String> {
        let doc = mupdf::Document::open(p.to_str().unwrap()).unwrap();
        (0..doc.page_count().unwrap()).map(|i| doc.load_page(i).unwrap().text(Default::default()).unwrap().trim().to_string()).collect()
    }

    fn thu_muc_tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_tach_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn ten_va_thu_muc_ra() {
        assert_eq!(ten_van_ban(MAU_TEN_MAC_DINH, "L1_05_13", 3), "L1_05_13_part3.pdf");
        assert_eq!(ten_van_ban("{hoso}_{n3}", "L1_05_13", 1), "L1_05_13_001.pdf");
        assert_eq!(ten_van_ban("{hoso}:{n}", "Hồ sơ số 5", 2), "Hồ sơ số 5_2.pdf");
        assert!(kiem_mau_ten("{hoso}_part").is_err());
        let nguon = Path::new("/du_lieu/mau2");
        assert_eq!(thu_muc_ra(nguon, None).unwrap(), PathBuf::from("/du_lieu/mau2_vanban"));
        assert_eq!(thu_muc_ra(nguon, Some(Path::new("/ra"))).unwrap(), PathBuf::from("/ra/mau2_vanban"));
        assert!(thu_muc_ra(nguon, Some(Path::new("/du_lieu/mau2/con"))).is_err());
        assert!(khop_mau(MAU_TEN_MAC_DINH, "a_b", "a_b_part12.pdf"));
        assert!(!khop_mau(MAU_TEN_MAC_DINH, "a_b", "a_b_partx.pdf"));
        assert!(!khop_mau(MAU_TEN_MAC_DINH, "a_b", "ghi_chu.pdf"));
        assert!(khop_mau("{hoso}_{n3}", "a", "a_004.pdf"));
    }

    #[test]
    fn tach_dung_trang_giu_cau_truc_va_chay_lai_khong_lan() {
        let goc = thu_muc_tam("cau_truc");
        let nguon = goc.join("mau2");
        let con = nguon.join("HOP 462").join("4766");
        std::fs::create_dir_all(&con).unwrap();
        std::fs::create_dir_all(nguon.join("chi_co_anh")).unwrap();
        std::fs::write(nguon.join("chi_co_anh").join("a.jpg"), b"x").unwrap();
        tao_pdf(&con.join("4766.pdf"), 10);
        tao_pdf(&nguon.join("goc.pdf"), 3);
        let nhan: BTreeMap<usize, Nhan> = [(3, Nhan::Index), (5, Nhan::Trang), (7, Nhan::Split)].into_iter().collect();
        let xoay: BTreeMap<usize, Xoay> = [(9, Xoay { goc: 90, xac_nhan: false })].into_iter().collect();
        let ho_so = vec![
            HoSoTach { pdf: con.join("4766.pdf"), nhan: nhan.clone(), xoay: xoay.clone() },
            HoSoTach { pdf: nguon.join("goc.pdf"), nhan: BTreeMap::new(), xoay: BTreeMap::new() },
        ];
        let ra = thu_muc_ra(&nguon, None).unwrap();
        let kq = tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert!(kq.loi.is_empty(), "{:?}", kq.loi);
        let thu_muc_hs = ra.join("HOP 462").join("4766").join("4766");
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766_part1.pdf")), vec!["Trang 1", "Trang 2"]);
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766_part2.pdf")), vec!["Trang 4", "Trang 6"]);
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766_part3.pdf")), vec!["Trang 7", "Trang 8", "Trang 9", "Trang 10"]);
        // Trang 9 xoay 90 → trang thứ 3 của part3 mang /Rotate 90; file gốc không đổi
        let doc = PdfDocument::open(thu_muc_hs.join("4766_part3.pdf").to_str().unwrap()).unwrap();
        assert_eq!(doc.find_page(2).unwrap().get_dict("Rotate").unwrap().unwrap().as_int().unwrap(), 90);
        // Hồ sơ không nhãn → chép nguyên từng byte; thư mục chỉ có ảnh không được tạo
        assert_eq!(std::fs::read(ra.join("goc").join("goc_part1.pdf")).unwrap(), std::fs::read(nguon.join("goc.pdf")).unwrap());
        assert_eq!(kq.sao_nguyen, 1);
        assert!(!ra.join("chi_co_anh").exists());
        // Log 1-based, STT liên tục
        let log = noi_dung_log(&kq.dong);
        let dong: Vec<&str> = log.lines().collect();
        assert_eq!(dong[0], TIEU_DE_LOG);
        assert!(dong[2].starts_with("2\t") && dong[2].contains("\t4-6\t") && dong[2].ends_with("\t[5]\t2"), "{}", dong[2]);
        assert_eq!(dong.len(), 1 + 4);
        // Chạy lại với ít văn bản hơn → part thừa cũ bị xoá, file khác trong thư mục giữ nguyên
        std::fs::write(thu_muc_hs.join("ghi_chu.txt"), b"giu").unwrap();
        let ho_so_2 = vec![HoSoTach { pdf: con.join("4766.pdf"), nhan: [(3, Nhan::Index)].into_iter().collect(), xoay: BTreeMap::new() }];
        let kq2 = tach_thu_muc(&nguon, &ra, &ho_so_2, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert_eq!(kq2.dong.len(), 2);
        assert!(!thu_muc_hs.join("4766_part3.pdf").exists());
        assert!(thu_muc_hs.join("ghi_chu.txt").exists());
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn hai_ho_so_trung_ten_khac_thu_muc_khong_de_nhau() {
        let goc = thu_muc_tam("trung_ten");
        let nguon = goc.join("nguon");
        for d in ["a", "b"] {
            std::fs::create_dir_all(nguon.join(d)).unwrap();
        }
        tao_pdf(&nguon.join("a").join("x.pdf"), 2);
        tao_pdf(&nguon.join("b").join("x.pdf"), 3);
        let ho_so: Vec<HoSoTach> = ["a", "b"].iter().map(|d| HoSoTach { pdf: nguon.join(d).join("x.pdf"), nhan: [(1, Nhan::Trang)].into_iter().collect(), xoay: BTreeMap::new() }).collect();
        let ra = thu_muc_ra(&nguon, None).unwrap();
        tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert_eq!(chu_cac_trang(&ra.join("a/x/x_part1.pdf")), vec!["Trang 2"]);
        assert_eq!(chu_cac_trang(&ra.join("b/x/x_part1.pdf")), vec!["Trang 2", "Trang 3"]);
        let _ = std::fs::remove_dir_all(&goc);
    }

    /// Mẫu 1: cắt theo nhãn b3 → 211 part, mỗi part trùng số trang + ảnh từng trang với `pdf_vanban` của b3.
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn tach_mau_1_khop_211_part_b3() {
        use super::super::cat_pdf::hash_anh_trang;
        use super::super::nhan::{doc_file_nhan, duong_dan_that};
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let (bang, _) = doc_file_nhan(&std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap(), &|_| None).unwrap();
        let ho_so: Vec<HoSoTach> = bang.into_iter().map(|(p, nhan)| HoSoTach { pdf: duong_dan_that(&mau, &p), nhan, xoay: BTreeMap::new() }).collect();
        let nguon = mau.join("pdf_goc");
        let ra = std::env::temp_dir().join("dema4_tach_mau1");
        let _ = std::fs::remove_dir_all(&ra);
        let t0 = std::time::Instant::now();
        let kq = tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        println!("{} part, {:.1} s", kq.dong.len(), t0.elapsed().as_secs_f64());
        assert!(kq.loi.is_empty());
        assert_eq!(kq.dong.len(), 211);
        for d in &kq.dong {
            let b3 = mau.join("pdf_vanban").join(d.file_ra.file_name().unwrap());
            let n = so_trang_pdf(&b3).unwrap();
            assert_eq!(d.so_trang, n, "{}", b3.display());
            for t in 1..=n {
                assert_eq!(hash_anh_trang(&d.file_ra, t), hash_anh_trang(&b3, t), "{} trang {t}", b3.display());
            }
        }
        let _ = std::fs::remove_dir_all(&ra);
    }
}
