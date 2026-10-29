//! Lệnh tách: từ hồ sơ + nhãn + góc xoay → ghi các văn bản PDF và dòng log `ketqua_tach.txt`.
//!
//! Thư mục ra `<cha>/<tên nguồn>_vanban` giữ nguyên cấu trúc thư mục nguồn và thêm 1 tầng thư mục hồ sơ:
//! `mau2/HOP 462/4766/4766.pdf` → `mau2_vanban/HOP 462/4766/4766/4766.1.pdf`.

use super::cat_pdf::cat_bang_ghep;
use super::nhan::{tinh_dai, Nhan, Xoay};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAU_TEN_MAC_DINH: &str = "{hoso}.{n}";
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
    /// Part cũ của lần tách trước không xoá được (thường do đang mở trong trình xem PDF trên Windows).
    pub khong_xoa: Vec<PathBuf>,
}

/// Kết quả tách 1 hồ sơ: dòng log, có chép nguyên không, cảnh báo, part cũ không xoá được.
type KetQuaHoSo = (Vec<DongLog>, bool, Vec<String>, Vec<PathBuf>);

fn so_trang_pdf(pdf: &Path) -> Result<usize, String> {
    let doc = mupdf::Document::open(pdf.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(|e| format!("không mở được: {e}"))?;
    // Có mật khẩu người dùng: MuPDF vẫn mở và đếm trang được nhưng luồng dữ liệu đọc ra là rác → file tách hỏng
    if doc.needs_password().map_err(|e| e.to_string())? {
        return Err(LOI_MAT_KHAU.into());
    }
    Ok(doc.page_count().map_err(|e| e.to_string())? as usize)
}

pub const LOI_MAT_KHAU: &str = "PDF có mật khẩu — mở bằng mật khẩu, lưu lại bản không mật khẩu rồi làm lại";

/// Diễn giải vài lỗi hay gặp của MuPDF / hệ điều hành cho người dùng; lỗi khác giữ nguyên.
pub fn ro_loi(e: String) -> String {
    let t = e.to_lowercase();
    if t.contains("no objects found") || t.contains("cannot recognize") || t.contains("not a pdf") {
        "file hỏng hoặc không phải PDF".into()
    } else if t.contains("os error 206)") || t.contains("name too long") {
        format!("đường dẫn quá dài (Windows giới hạn 260 ký tự) — đặt thư mục lưu kết quả ngắn hơn · {e}")
    } else if t.contains("permission denied") || t.contains("access is denied") || t.contains("os error 5)") {
        format!("không có quyền ghi · {e}")
    } else if t.contains("no space left") || t.contains("os error 112)") || t.contains("os error 28)") {
        format!("ổ đĩa đầy · {e}")
    } else {
        e
    }
}

/// Chữ trong panic (để báo thành lỗi thường thay vì sập app).
pub fn chu_panic(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "không rõ".into())
}

/// Đuôi file tạm khi đang ghi 1 văn bản: ghi xong mới đổi tên thành `.pdf`, nên tắt app giữa chừng không để lại PDF cụt.
const DUOI_TAM: &str = ".tmp";

/// Tách 1 hồ sơ vào `thu_muc_ho_so` (= `<ra>/<thư mục con>/<hồ sơ>/`). Trả các dòng log + có chép nguyên không.
fn tach_ho_so(ho_so: &HoSoTach, thu_muc_ho_so: &Path, mau: &str) -> Result<KetQuaHoSo, String> {
    let ten_ho_so = ho_so.pdf.file_stem().ok_or("tên file không hợp lệ")?.to_string_lossy().into_owned();
    let so_trang = so_trang_pdf(&ho_so.pdf)?;
    if so_trang == 0 {
        return Ok((Vec::new(), false, vec![format!("{}: 0 trang, không có gì để tách", ho_so.pdf.display())], Vec::new()));
    }
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
        let tam = thu_muc_ho_so.join(format!("{ten}{DUOI_TAM}"));
        let ghi = if dai.nguyen_ven {
            // Không nhãn, không xoay: chép từng byte (giữ cả chữ ký số), không ghi lại bằng MuPDF
            std::fs::copy(&ho_so.pdf, &tam).map(|_| ()).map_err(|e| format!("không chép được {}: {e}", ra.display()))
        } else {
            cat_bang_ghep(&ho_so.pdf, &vb.trang_giu, &tam).map_err(|e| format!("không ghi được {}: {e}", ra.display()))
        };
        if let Err(e) = ghi.and_then(|_| std::fs::rename(&tam, &ra).map_err(|e| format!("không đổi tên file tạm thành {}: {e}", ra.display()))) {
            let _ = std::fs::remove_file(&tam);
            return Err(e);
        }
        da_ghi.insert(ten);
        dong.push(DongLog { file_vao: ho_so.pdf.clone(), tu: vb.tu, den: vb.den, file_ra: ra, trang_bo: vb.bo.clone(), so_trang: vb.trang_giu.len() });
    }
    // Dọn văn bản cũ của lần tách trước không còn sinh ra (chỉ file khớp mẫu tên đang áp) và file tạm bỏ dở
    let mut khong_xoa = Vec::new();
    if let Ok(ds) = std::fs::read_dir(thu_muc_ho_so) {
        for muc in ds.flatten() {
            let ten = muc.file_name().to_string_lossy().into_owned();
            let tam_cu = ten.strip_suffix(DUOI_TAM).is_some_and(|goc| khop_mau(mau, &ten_ho_so, goc));
            let thua = !da_ghi.contains(&ten) && khop_mau(mau, &ten_ho_so, &ten);
            if (tam_cu || thua) && std::fs::remove_file(muc.path()).is_err() {
                khong_xoa.push(muc.path());
            }
        }
    }
    khong_xoa.sort();
    Ok((dong, dai.nguyen_ven, dai.canh_bao.iter().map(|c| format!("{}: {c}", ho_so.pdf.display())).collect(), khong_xoa))
}

/// Tách mọi hồ sơ (song song theo hồ sơ). `nguon` = thư mục đầu vào (để giữ cấu trúc thư mục con), `ra` = thư mục ra.
/// `tien_do(đã xong, tổng)` gọi sau mỗi hồ sơ; `dung` bật → bỏ các hồ sơ chưa bắt đầu.
pub fn tach_thu_muc(nguon: &Path, ra: &Path, ho_so: &[HoSoTach], mau: &str, dung: &AtomicBool, tien_do: &(dyn Fn(usize, usize) + Sync)) -> Result<KetQuaTach, String> {
    kiem_mau_ten(mau)?;
    let xong = std::sync::atomic::AtomicUsize::new(0);
    let tung: Vec<(usize, Result<KetQuaHoSo, String>)> = ho_so
        .par_iter()
        .enumerate()
        .filter(|_| !dung.load(Ordering::Relaxed))
        .map(|(i, hs)| {
            let con = hs.pdf.parent().and_then(|p| p.strip_prefix(nguon).ok()).unwrap_or(Path::new(""));
            let ten = hs.pdf.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            // Một hồ sơ gây panic (lỗi trong thư viện PDF) chỉ thành lỗi của hồ sơ đó, không làm hỏng cả lô
            let thu_muc_hs = ra.join(con).join(ten);
            let kq = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tach_ho_so(hs, &thu_muc_hs, mau)))
                .unwrap_or_else(|p| Err(format!("lỗi bất ngờ khi tách: {}", chu_panic(&*p))))
                .map_err(ro_loi);
            tien_do(xong.fetch_add(1, Ordering::Relaxed) + 1, ho_so.len());
            (i, kq)
        })
        .collect();
    let mut kq = KetQuaTach::default();
    let mut tung = tung;
    tung.sort_by_key(|x| x.0);
    for (i, r) in tung {
        match r {
            Ok((dong, nguyen, cb, khong_xoa)) => {
                kq.dong.extend(dong);
                kq.sao_nguyen += nguyen as usize;
                kq.canh_bao.extend(cb);
                kq.khong_xoa.extend(khong_xoa);
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

/// Dòng ghi thêm cuối log (bắt đầu bằng `#` để script đọc bảng bỏ qua): part cũ không xoá được.
pub fn dong_log_khong_xoa(khong_xoa: &[PathBuf]) -> String {
    khong_xoa.iter().map(|p| format!("# Không xoá được file cũ (có thể đang mở): {}\n", p.display())).collect()
}

/// Dòng `#` cuối log: hồ sơ lỗi (không tách được) và cảnh báo (dải toàn trắng, hồ sơ 0 trang) — để xem lại đủ sau popup.
pub fn dong_log_loi(loi: &[(PathBuf, String)], canh_bao: &[String]) -> String {
    let mut s: String = loi.iter().map(|(p, e)| format!("# Lỗi {}: {e}\n", p.display())).collect();
    s.extend(canh_bao.iter().map(|c| format!("# Cảnh báo {c}\n")));
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

    /// PDF mật khẩu người dùng: báo lỗi, không ghi file (ghi ra sẽ là PDF hỏng); chỉ mật khẩu chủ sở hữu: tách bình thường.
    #[test]
    fn pdf_mat_khau_bao_loi_khong_ghi_file_hong() {
        use mupdf::pdf::{Encryption, PdfWriteOptions};
        let goc = std::env::temp_dir().join(format!("dema4_mat_khau_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        tao_pdf(&goc.join("goc.pdf"), 4);
        let doc = PdfDocument::open(goc.join("goc.pdf").to_str().unwrap()).unwrap();
        for (ten, nguoi_dung) in [("mk_user.pdf", "matkhau"), ("mk_owner.pdf", "")] {
            let mut tc = PdfWriteOptions::default();
            tc.set_encryption(Encryption::Aes128).set_user_password(nguoi_dung).set_owner_password("chu");
            doc.save_with_options(nguon.join(ten).to_str().unwrap(), tc).unwrap();
        }
        let nhan: BTreeMap<usize, Nhan> = [(2, Nhan::Index)].into();
        let ho_so = vec![
            HoSoTach { pdf: nguon.join("mk_user.pdf"), nhan: nhan.clone(), xoay: BTreeMap::new() },
            HoSoTach { pdf: nguon.join("mk_owner.pdf"), nhan, xoay: BTreeMap::new() },
        ];
        let ra = thu_muc_ra(&nguon, None).unwrap();
        let kq = tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert_eq!(kq.loi.len(), 1, "{:?}", kq.loi);
        assert!(kq.loi[0].0.ends_with("mk_user.pdf") && kq.loi[0].1.contains("mật khẩu"), "{:?}", kq.loi);
        assert!(!ra.join("mk_user").exists(), "không được ghi file nào từ PDF có mật khẩu");
        assert_eq!(chu_cac_trang(&ra.join("mk_owner").join("mk_owner.1.pdf")), ["Trang 1"]);
        assert_eq!(chu_cac_trang(&ra.join("mk_owner").join("mk_owner.2.pdf")), ["Trang 3", "Trang 4"]);
        // Lúc phân tích cũng chặn: hồ sơ báo lỗi, không hiện trang để gán nhãn
        assert!(super::super::dung_luong::doc_thong_tin_file(&nguon.join("mk_user.pdf")).unwrap_err().contains("mật khẩu"));
        assert_eq!(super::super::dung_luong::doc_thong_tin_file(&nguon.join("mk_owner.pdf")).unwrap().len(), 4);
        let _ = std::fs::remove_dir_all(&goc);
    }

    /// Hồ sơ hỏng chỉ lỗi riêng nó; hồ sơ 0 trang thành cảnh báo; văn bản ghi qua file tạm, không để sót `.tmp`
    /// (kể cả file tạm bỏ dở của lần trước); lỗi + cảnh báo có dòng `#` trong log.
    #[test]
    fn ho_so_loi_khong_chan_ho_so_khac_khong_sot_file_tam_va_ghi_log() {
        let goc = std::env::temp_dir().join(format!("dema4_hs_loi_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        tao_pdf(&nguon.join("ok.pdf"), 3);
        tao_pdf(&nguon.join("trong.pdf"), 0);
        std::fs::write(nguon.join("hong.pdf"), b"%PDF-1.4\nrac rac rac").unwrap();
        let ra = thu_muc_ra(&nguon, None).unwrap();
        std::fs::create_dir_all(ra.join("ok")).unwrap();
        std::fs::write(ra.join("ok").join("ok.9.pdf.tmp"), b"do dang").unwrap();
        let nhan: BTreeMap<usize, Nhan> = [(2, Nhan::Index)].into();
        let ho_so: Vec<HoSoTach> = ["ok.pdf", "trong.pdf", "hong.pdf"].iter().map(|t| HoSoTach { pdf: nguon.join(t), nhan: nhan.clone(), xoay: BTreeMap::new() }).collect();
        let kq = tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert_eq!(kq.loi.len(), 1, "{:?}", kq.loi);
        assert!(kq.loi[0].0.ends_with("hong.pdf") && kq.loi[0].1.contains("file hỏng hoặc không phải PDF"), "{:?}", kq.loi);
        assert!(kq.canh_bao.iter().any(|c| c.contains("trong.pdf") && c.contains("0 trang")), "{:?}", kq.canh_bao);
        let mut ten: Vec<String> = std::fs::read_dir(ra.join("ok")).unwrap().map(|m| m.unwrap().file_name().to_string_lossy().into_owned()).collect();
        ten.sort();
        assert_eq!(ten, ["ok.1.pdf", "ok.2.pdf"], "không còn file tạm");
        let log = dong_log_loi(&kq.loi, &kq.canh_bao);
        assert!(log.contains("# Lỗi ") && log.contains("hong.pdf") && log.contains("# Cảnh báo ") && log.contains("0 trang"), "{log}");
        let _ = std::fs::remove_dir_all(&goc);
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
        assert_eq!(ten_van_ban(MAU_TEN_MAC_DINH, "L1_05_13", 3), "L1_05_13.3.pdf");
        assert_eq!(ten_van_ban("{hoso}_{n3}", "L1_05_13", 1), "L1_05_13_001.pdf");
        assert_eq!(ten_van_ban("{hoso}:{n}", "Hồ sơ số 5", 2), "Hồ sơ số 5_2.pdf");
        assert!(kiem_mau_ten("{hoso}_part").is_err());
        let nguon = Path::new("/du_lieu/mau2");
        assert_eq!(thu_muc_ra(nguon, None).unwrap(), PathBuf::from("/du_lieu/mau2_vanban"));
        assert_eq!(thu_muc_ra(nguon, Some(Path::new("/ra"))).unwrap(), PathBuf::from("/ra/mau2_vanban"));
        assert!(thu_muc_ra(nguon, Some(Path::new("/du_lieu/mau2/con"))).is_err());
        assert!(khop_mau(MAU_TEN_MAC_DINH, "a_b", "a_b.12.pdf"));
        assert!(!khop_mau(MAU_TEN_MAC_DINH, "a_b", "a_b.x.pdf"));
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
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766.1.pdf")), vec!["Trang 1", "Trang 2"]);
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766.2.pdf")), vec!["Trang 4", "Trang 6"]);
        assert_eq!(chu_cac_trang(&thu_muc_hs.join("4766.3.pdf")), vec!["Trang 7", "Trang 8", "Trang 9", "Trang 10"]);
        // Trang 9 xoay 90 → trang thứ 3 của part3 mang /Rotate 90; file gốc không đổi
        let doc = PdfDocument::open(thu_muc_hs.join("4766.3.pdf").to_str().unwrap()).unwrap();
        assert_eq!(doc.find_page(2).unwrap().get_dict("Rotate").unwrap().unwrap().as_int().unwrap(), 90);
        drop(doc); // Windows không xoá được file đang mở (lần chạy lại bên dưới phải xoá part3)
        // Hồ sơ không nhãn → chép nguyên từng byte; thư mục chỉ có ảnh không được tạo
        assert_eq!(std::fs::read(ra.join("goc").join("goc.1.pdf")).unwrap(), std::fs::read(nguon.join("goc.pdf")).unwrap());
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
        assert!(!thu_muc_hs.join("4766.3.pdf").exists());
        assert!(thu_muc_hs.join("ghi_chu.txt").exists());
        assert!(kq2.khong_xoa.is_empty());
        // Part cũ không xoá được (Windows: file đang mở; giả lập bằng thư mục trùng tên part) → báo lại, không im lặng
        std::fs::create_dir(thu_muc_hs.join("4766.3.pdf")).unwrap();
        std::fs::write(thu_muc_hs.join("4766.3.pdf").join("x"), b"").unwrap();
        let kq3 = tach_thu_muc(&nguon, &ra, &ho_so_2, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert!(kq3.loi.is_empty(), "{:?}", kq3.loi);
        assert_eq!(kq3.khong_xoa, vec![thu_muc_hs.join("4766.3.pdf")]);
        assert!(dong_log_khong_xoa(&kq3.khong_xoa).starts_with("# Không xoá được file cũ"));
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
        assert_eq!(chu_cac_trang(&ra.join("a/x/x.1.pdf")), vec!["Trang 2"]);
        assert_eq!(chu_cac_trang(&ra.join("b/x/x.1.pdf")), vec!["Trang 2", "Trang 3"]);
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

    /// Mẫu 2 (234 hồ sơ, nhãn tay): mỗi hồ sơ 1 thư mục giữ cấu trúc nguồn; tổng part = tổng văn bản theo dải cắt;
    /// hồ sơ không nhãn (4613, 4727) chép nguyên từng byte; log đủ dòng.
    #[test]
    #[ignore = "cần DEMA4_MAU2 + DEMA4_MAU2_NHAN, chạy ~1 phút"]
    fn tach_mau_2_cau_truc_thu_muc() {
        use super::super::nhan::{doc_file_nhan, tinh_dai};
        let nguon = PathBuf::from(std::env::var("DEMA4_MAU2").expect("DEMA4_MAU2"));
        let (bang, _) = doc_file_nhan(&std::fs::read_to_string(std::env::var("DEMA4_MAU2_NHAN").expect("DEMA4_MAU2_NHAN")).unwrap(), &|_| None).unwrap();
        let (gui, _n) = std::sync::mpsc::channel();
        let pdf = crate::dem_pdf::tim_file_pdf(&nguon.to_string_lossy(), &AtomicBool::new(false), &gui);
        assert_eq!(pdf.len(), 234);
        // File nhãn mẫu 2 ghi đường dẫn tuyệt đối của máy khác → ghép theo tên file
        let ho_so: Vec<HoSoTach> = pdf
            .iter()
            .map(|p| {
                let ten = p.file_name().unwrap().to_string_lossy().to_string();
                let nhan = bang.iter().find(|(k, _)| k.rsplit(['/', '\\']).next() == Some(ten.as_str())).map(|(_, v)| v.clone()).unwrap_or_default();
                HoSoTach { pdf: p.clone(), nhan, xoay: BTreeMap::new() }
            })
            .collect();
        let tong_vb: usize = ho_so.iter().map(|h| tinh_dai(so_trang_pdf(&h.pdf).unwrap(), &h.nhan, &h.xoay).van_ban.len()).sum();
        let ra = std::env::temp_dir().join("dema4_tach_mau2");
        let _ = std::fs::remove_dir_all(&ra);
        let t0 = std::time::Instant::now();
        let kq = tach_thu_muc(&nguon, &ra, &ho_so, MAU_TEN_MAC_DINH, &AtomicBool::new(false), &|_, _| {}).unwrap();
        println!("{} hồ sơ → {} part ({} chép nguyên) trong {:.1} s", ho_so.len(), kq.dong.len(), kq.sao_nguyen, t0.elapsed().as_secs_f64());
        assert!(kq.loi.is_empty(), "{:?}", kq.loi);
        assert_eq!(kq.dong.len(), tong_vb);
        assert_eq!(kq.sao_nguyen, 2, "4613 và 4727 không có nhãn");
        for h in &ho_so {
            let con = h.pdf.parent().unwrap().strip_prefix(&nguon).unwrap();
            let thu_muc_hs = ra.join(con).join(h.pdf.file_stem().unwrap());
            assert!(thu_muc_hs.is_dir(), "{}", thu_muc_hs.display());
        }
        let so_part: usize = walk_pdf(&ra);
        assert_eq!(so_part, tong_vb, "không có file thừa");
        assert_eq!(noi_dung_log(&kq.dong).lines().count(), tong_vb + 1);
        let _ = std::fs::remove_dir_all(&ra);
    }

    fn walk_pdf(d: &Path) -> usize {
        std::fs::read_dir(d).unwrap().flatten().map(|m| if m.path().is_dir() { walk_pdf(&m.path()) } else { (m.path().extension().is_some_and(|e| e == "pdf")) as usize }).sum()
    }
}

