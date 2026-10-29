//! Bước 1 của Phân tích: đọc nhanh dung lượng + kích thước mọi trang (không render, không giải nén ảnh)
//! để lưới có đủ ô và sắp theo dung lượng được ngay.
//!
//! Dung lượng trang = tổng `/Length` các ảnh nhúng của trang; trang không có ảnh → độ dài luồng nội dung.

use mupdf::pdf::{PdfDocument, PdfObject};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ThongTinTrang {
    /// Byte dữ liệu nén của trang.
    pub dung_luong: u32,
    /// Kích thước trang (điểm PDF, 1/72 inch) theo CropBox, không có thì MediaBox.
    pub rong: f32,
    pub cao: f32,
}

fn loi(e: mupdf::Error) -> String {
    e.to_string()
}

/// Giá trị số nguyên của khoá (kể cả khi là tham chiếu gián tiếp).
fn so_nguyen(obj: &PdfObject, khoa: &str) -> u32 {
    obj.get_dict(khoa).ok().flatten().and_then(|v| v.resolve().ok().flatten().or(Some(v))).and_then(|v| v.as_int().ok()).unwrap_or(0).max(0) as u32
}

fn do_dai_noi_dung(trang: &PdfObject) -> u32 {
    let Some(c) = trang.get_dict("Contents").ok().flatten() else { return 0 };
    let c = c.resolve().ok().flatten().unwrap_or(c);
    if c.is_array().unwrap_or(false) {
        (0..c.len().unwrap_or(0) as i32).filter_map(|i| c.get_array(i).ok().flatten()).map(|s| so_nguyen(&s.resolve().ok().flatten().unwrap_or(s), "Length")).sum()
    } else {
        so_nguyen(&c, "Length")
    }
}

fn dung_luong_anh(trang: &PdfObject) -> u32 {
    let Some(tn) = trang.get_dict_inheritable("Resources").ok().flatten() else { return 0 };
    let Some(xo) = tn.get_dict("XObject").ok().flatten() else { return 0 };
    let Ok(ds) = xo.dict_iter() else { return 0 };
    ds.flatten()
        .filter(|(_, v)| v.get_dict("Subtype").ok().flatten().and_then(|s| s.as_name().ok()).as_deref() == Some(b"Image".as_slice()))
        .map(|(_, v)| so_nguyen(&v, "Length"))
        .sum()
}

fn kich_thuoc(trang: &PdfObject) -> (f32, f32) {
    let hop = trang.get_dict_inheritable("CropBox").ok().flatten().or_else(|| trang.get_dict_inheritable("MediaBox").ok().flatten());
    let Some(hop) = hop else { return (595.0, 842.0) };
    let so = |i: i32| hop.get_array(i).ok().flatten().and_then(|v| v.as_float().ok()).unwrap_or(0.0);
    let (rong, cao) = ((so(2) - so(0)).abs(), (so(3) - so(1)).abs());
    let xoay = trang.get_dict_inheritable("Rotate").ok().flatten().and_then(|v| v.as_int().ok()).unwrap_or(0).rem_euclid(360);
    if xoay == 90 || xoay == 270 { (cao, rong) } else { (rong, cao) }
}

/// Đọc thông tin mọi trang của 1 file PDF.
pub fn doc_thong_tin_file(pdf: &Path) -> Result<Vec<ThongTinTrang>, String> {
    let doc = PdfDocument::open(pdf.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(loi)?;
    let n = doc.page_count().map_err(loi)?;
    (0..n)
        .map(|i| {
            let trang = doc.find_page(i).map_err(loi)?;
            let anh = dung_luong_anh(&trang);
            let (rong, cao) = kich_thuoc(&trang);
            Ok(ThongTinTrang { dung_luong: if anh > 0 { anh } else { do_dai_noi_dung(&trang) }, rong, cao })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[test]
    fn trang_khong_anh_lay_do_dai_noi_dung() {
        let tep = std::env::temp_dir().join(format!("dema4_dung_luong_{}.pdf", std::process::id()));
        crate::tach_vb::tach::tests::tao_pdf(&tep, 3);
        let tt = doc_thong_tin_file(&tep).unwrap();
        assert_eq!(tt.len(), 3);
        assert_eq!(tt[0].dung_luong as usize, "BT /F1 24 Tf 72 720 Td (Trang 1) Tj ET".len());
        assert_eq!((tt[0].rong, tt[0].cao), (595.0, 842.0));
        let _ = std::fs::remove_file(&tep);
    }

    /// Mẫu 1: dung lượng tờ index (ảnh rất nhỏ) nhỏ hơn hẳn trang chữ.
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn dung_luong_mau_1() {
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let tt = doc_thong_tin_file(&mau.join("pdf_goc/L1_05_13.pdf")).unwrap();
        assert_eq!(tt.len(), 491);
        assert!(tt[9].dung_luong < tt[0].dung_luong / 3, "trang 10 là index: {:?} vs {:?}", tt[9], tt[0]);
    }

    /// Mẫu 2: 21.113 trang — đo thời gian bước 1 (mục tiêu 100k trang ≤ 10 s).
    #[test]
    #[ignore = "cần DEMA4_MAU2"]
    fn toc_do_mau_2() {
        let thu_muc = std::env::var("DEMA4_MAU2").expect("đặt DEMA4_MAU2");
        let (gui, _n) = std::sync::mpsc::channel();
        let t0 = std::time::Instant::now();
        let pdf = crate::dem_pdf::tim_file_pdf(&thu_muc, &std::sync::atomic::AtomicBool::new(false), &gui);
        let so_trang: usize = pdf.par_iter().map(|p| doc_thong_tin_file(p).unwrap().len()).sum();
        println!("{} PDF, {so_trang} trang: {:.2} s", pdf.len(), t0.elapsed().as_secs_f64());
    }
}
