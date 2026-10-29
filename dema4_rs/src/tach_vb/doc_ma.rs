//! Đọc mã Code 128 trên ảnh xám của trang (tờ index phân cách văn bản).
//!
//! Đọc nhiều lượt trên cùng một ảnh 150 dpi (không render lại), dừng ở lượt đầu tiên ra mã hợp lệ:
//! 1. nhị phân hoá LocalAverage, giữ cả mã đọc ra nhưng sai checksum;
//! 2. nhị phân hoá GlobalHistogram;
//! 3. ảnh thu 0,5 — tờ index scan ảnh rất nhỏ (mẫu 1: ~27 dpi) bị nhoè khi phóng lên 150 dpi;
//! 4. ảnh thu 0,8 rồi 0,9 — tránh răng cưa lấy mẫu ở đúng 150 dpi (mẫu 2: trang 4762 tr.122 chỉ ra ở 0,8, dạng mã lỗi
//!    checksum lệch 1 ký tự — được nhận vì khớp mã index trong cấu hình).
//!
//! Đo: mẫu 1 đủ 209/209, mẫu 2 đủ 1.780/1.780 tờ index, không gán thừa.

use zxingcpp::{BarcodeError, BarcodeFormat, Binarizer, ImageFormat, ImageView};

/// Một mã đọc được trên trang.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaDoc {
    pub gia_tri: String,
    /// Đọc ra ký tự nhưng sai checksum: chỉ dùng khi giá trị khớp mã index trong cấu hình.
    pub loi_checksum: bool,
}

/// Đọc mọi mã Code 128 trên ảnh xám 8 bit (`rong` × `cao`, mỗi điểm 1 byte).
pub fn doc_ma_code128(xam: &[u8], rong: u32, cao: u32) -> Vec<MaDoc> {
    let mut loi = doc_mot_luot(xam, rong, cao, Binarizer::LocalAverage, true);
    if loi.iter().any(|m| !m.loi_checksum) {
        return loi;
    }
    let luot_2 = doc_mot_luot(xam, rong, cao, Binarizer::GlobalHistogram, false);
    if !luot_2.is_empty() {
        return luot_2;
    }
    for ty_le in [0.5, 0.8, 0.9] {
        let (nho, rong_nho, cao_nho) = thu_nho(xam, rong, cao, ty_le);
        let luot = doc_mot_luot(&nho, rong_nho, cao_nho, Binarizer::LocalAverage, true);
        if luot.iter().any(|m| !m.loi_checksum) {
            return luot;
        }
        loi.extend(luot);
    }
    loi
}

fn doc_mot_luot(xam: &[u8], rong: u32, cao: u32, nhi_phan: Binarizer, giu_loi: bool) -> Vec<MaDoc> {
    let Ok(anh) = ImageView::from_slice(xam, rong, cao, ImageFormat::Lum) else {
        return Vec::new();
    };
    let doc = zxingcpp::read().formats([BarcodeFormat::Code128]).binarizer(nhi_phan).return_errors(giu_loi);
    let Ok(ds) = doc.from(anh) else {
        return Vec::new();
    };
    ds.into_iter()
        .filter_map(|ma| match ma.error() {
            BarcodeError::None() => Some(MaDoc { gia_tri: ma.text(), loi_checksum: false }),
            BarcodeError::Checksum(_) => Some(MaDoc { gia_tri: ma.text(), loi_checksum: true }),
            _ => None,
        })
        .collect()
}

/// Thu nhỏ ảnh xám theo tỷ lệ `ty_le` (< 1) bằng trung bình vùng (khử răng cưa khi thu nhỏ, như MuPDF).
fn thu_nho(xam: &[u8], rong: u32, cao: u32, ty_le: f32) -> (Vec<u8>, u32, u32) {
    let rong_moi = ((rong as f32) * ty_le).max(1.0) as usize;
    let cao_moi = ((cao as f32) * ty_le).max(1.0) as usize;
    (super::anh::thu_ve_mang(xam, rong as usize, cao as usize, 1, rong_moi, cao_moi), rong_moi as u32, cao_moi as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mupdf::{Colorspace, Document, Matrix};
    use std::time::Instant;

    /// Render trang (1-based) ở `dpi` thành ảnh xám.
    fn render_xam(doc: &Document, trang: i32, dpi: f32) -> (Vec<u8>, u32, u32) {
        let page = doc.load_page(trang - 1).unwrap();
        let s = dpi / 72.0;
        let pix = page.to_pixmap(&Matrix::new_scale(s, s), &Colorspace::device_gray(), false, false).unwrap();
        (pix.samples().to_vec(), pix.width(), pix.height())
    }

    #[test]
    fn thu_nho_giu_kich_thuoc_dung_ty_le() {
        let anh = vec![200u8; 100 * 50];
        let (ra, w, h) = thu_nho(&anh, 100, 50, 0.9);
        assert_eq!((w, h), (90, 45));
        assert!(ra.iter().all(|&v| v == 200));
    }

    #[test]
    fn anh_trang_khong_co_ma() {
        let anh = vec![255u8; 600 * 800];
        assert!(doc_ma_code128(&anh, 600, 800).is_empty());
    }

    /// Mẫu 1: 896 trang, 209 tờ index theo `index_and_blank.txt` (chuẩn của b3).
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn doc_du_ma_index_tren_mau_1() {
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let nhan = std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap();
        let chuan: std::collections::BTreeSet<(String, i32)> = nhan
            .lines()
            .filter_map(|d| {
                let c: Vec<&str> = d.trim().split(';').collect();
                (c.len() == 3 && c[2].trim() == "index").then(|| (c[0].rsplit('/').next().unwrap().to_string(), c[1].trim().parse().unwrap()))
            })
            .collect();
        let (mut tim_thay, mut thua) = (std::collections::BTreeSet::new(), Vec::new());
        let (mut t_render, mut t_doc, mut so_trang) = (0f64, 0f64, 0usize);
        for ten in ["L1_05_13.pdf", "L1_06_14.pdf"] {
            let doc = Document::open(mau.join("pdf_goc").join(ten).to_str().unwrap()).unwrap();
            for trang in 1..=doc.page_count().unwrap() {
                let t0 = Instant::now();
                let (xam, w, h) = render_xam(&doc, trang, 150.0);
                let t1 = Instant::now();
                let ma = doc_ma_code128(&xam, w, h);
                t_render += (t1 - t0).as_secs_f64();
                t_doc += t1.elapsed().as_secs_f64();
                so_trang += 1;
                if ma.iter().any(|m| m.gia_tri.contains("NAMSAO")) {
                    if chuan.contains(&(ten.to_string(), trang)) {
                        tim_thay.insert((ten.to_string(), trang));
                    } else {
                        thua.push((ten, trang, ma));
                    }
                }
            }
        }
        let sot: Vec<_> = chuan.difference(&tim_thay).collect();
        println!(
            "{so_trang} trang: render {:.1} ms/trang, đọc mã {:.1} ms/trang; index {}/{}, thừa {}",
            t_render * 1e3 / so_trang as f64,
            t_doc * 1e3 / so_trang as f64,
            tim_thay.len(),
            chuan.len(),
            thua.len()
        );
        assert!(sot.is_empty(), "sót index: {sot:?}");
        assert!(thua.is_empty(), "gán thừa: {thua:?}");
    }
}

#[cfg(test)]
mod chan_doan {
    use super::*;
    /// Đo trên toàn mẫu 1: mỗi tỷ lệ thu nhỏ (từ ảnh 150 dpi) đọc được bao nhiêu trang index, có gán thừa không.
    #[test]
    #[ignore = "chẩn đoán tay"]
    fn do_ty_le_thu_nho_mau_1() {
        let mau = crate::tach_vb::thu_muc_mau().unwrap();
        let nhan = std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap();
        let chuan: std::collections::HashSet<(String, i32)> = nhan.lines().filter_map(|d| { let c: Vec<&str> = d.trim().split(';').collect(); (c.len() == 3 && c[2].trim() == "index").then(|| (c[0].rsplit('/').next().unwrap().to_string(), c[1].trim().parse().unwrap())) }).collect();
        let ty_le = [1.0f32, 0.9, 0.8, 0.75, 0.67, 0.5];
        let mut dung = vec![0; ty_le.len()]; let mut thua = vec![0; ty_le.len()];
        let mut chua_doc: Vec<(String, i32)> = Vec::new(); let mut doc_duoc: Vec<Vec<bool>> = Vec::new();
        for ten in ["L1_05_13.pdf", "L1_06_14.pdf"] {
            let doc = mupdf::Document::open(mau.join("pdf_goc").join(ten).to_str().unwrap()).unwrap();
            for t in 1..=doc.page_count().unwrap() {
                let s = 150.0 / 72.0;
                let pix = doc.load_page(t - 1).unwrap().to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_gray(), false, false).unwrap();
                let (w, h) = (pix.width(), pix.height()); let xam = pix.samples().to_vec();
                let la_index = chuan.contains(&(ten.to_string(), t));
                let mut co = false; let mut hang = vec![false; ty_le.len()];
                for (i, &k) in ty_le.iter().enumerate() {
                    let (a, w2, h2) = if k < 1.0 { thu_nho(&xam, w, h, k) } else { (xam.clone(), w, h) };
                    let ok = doc_mot_luot(&a, w2, h2, Binarizer::LocalAverage, false).iter().any(|m| m.gia_tri == "NAMSAO-TACH");
                    if ok { if la_index { dung[i] += 1; co = true; hang[i] = true; } else { thua[i] += 1; } }
                }
                if la_index && !co { chua_doc.push((ten.to_string(), t)); }
                if la_index { doc_duoc.push(hang); }
            }
        }
        for (i, k) in ty_le.iter().enumerate() { println!("tỷ lệ {k}: đúng {}/{} thừa {}", dung[i], chuan.len(), thua[i]); }
        for bo in [vec![0, 5], vec![0, 5, 1], vec![0, 2, 5], vec![0, 3, 5], vec![0, 1, 3, 5]] {
            let n = doc_duoc.iter().filter(|h| bo.iter().any(|&i| h[i])).count();
            println!("gộp {:?}: {n}/{}", bo.iter().map(|&i| ty_le[i]).collect::<Vec<_>>(), chuan.len());
        }
        println!("không tỷ lệ nào đọc được: {chua_doc:?}");
    }
}

#[cfg(test)]
mod mau_2 {
    use super::*;
    use rayon::prelude::*;

    /// Mẫu 2 (`DEMA4_MAU2` = thư mục, `DEMA4_MAU2_NHAN` = file nhãn tay): đủ 1.780/1.780 index, không thừa.
    #[test]
    #[ignore = "cần DEMA4_MAU2 + DEMA4_MAU2_NHAN, chạy vài phút"]
    fn doc_du_ma_index_tren_mau_2() {
        let thu_muc = std::path::PathBuf::from(std::env::var_os("DEMA4_MAU2").expect("đặt DEMA4_MAU2"));
        let nhan = std::fs::read_to_string(std::env::var_os("DEMA4_MAU2_NHAN").expect("đặt DEMA4_MAU2_NHAN")).unwrap();
        let ten = |p: &str| std::path::Path::new(p).file_stem().unwrap().to_string_lossy().to_string();
        let chuan: std::collections::HashSet<(String, i32)> = nhan.lines().filter_map(|d| { let c: Vec<&str> = d.trim().split(';').collect(); (c.len() == 3 && c[2] == "index").then(|| (ten(c[0]), c[1].parse().unwrap())) }).collect();
        let mut pdf = Vec::new();
        for muc in jwalk::WalkDir::new(&thu_muc) { let p = muc.unwrap().path(); if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) { pdf.push(p); } }
        let t0 = std::time::Instant::now();
        let tim: Vec<(String, i32)> = pdf.par_iter().flat_map_iter(|p| {
            let doc = mupdf::Document::open(p.to_str().unwrap()).unwrap();
            let s = 150.0 / 72.0;
            (1..=doc.page_count().unwrap()).filter_map(|t| {
                let pix = doc.load_page(t - 1).unwrap().to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_gray(), false, false).unwrap();
                doc_ma_code128(pix.samples(), pix.width(), pix.height()).iter().any(|m| crate::tach_vb::phan_tich::lech_ky_tu(&m.gia_tri, "NAMSAO-TACH") <= 3).then(|| (ten(p.to_str().unwrap()), t))
            }).collect::<Vec<_>>().into_iter()
        }).collect();
        let tim: std::collections::HashSet<_> = tim.into_iter().collect();
        let sot: Vec<_> = chuan.difference(&tim).collect();
        let thua: Vec<_> = tim.difference(&chuan).collect();
        println!("{} PDF, {:.0} s: index {}/{}, sót {:?}, thừa {:?}", pdf.len(), t0.elapsed().as_secs_f64(), chuan.len() - sot.len(), chuan.len(), sot, thua);
        // Chỉ được sót 2 trang mã "Van-ban-khac": nhãn tay ghi là index nhưng mã khác mã index trong cấu hình
        let van_ban_khac: std::collections::HashSet<(String, i32)> = [("4632".to_string(), 29), ("4705".to_string(), 90)].into_iter().collect();
        assert!(sot.iter().all(|k| van_ban_khac.contains(*k)), "sót {sot:?}");
        assert!(thua.is_empty(), "thừa {thua:?}");
    }
}

#[cfg(test)]
mod chan_doan_trang_kho {
    use super::*;
    /// Trang 4762 tr.122 mẫu 2: chỉ đọc được ở vài độ phân giải — dò tỷ lệ thu nhỏ nào cứu được.
    #[test]
    #[ignore = "chẩn đoán tay, cần DEMA4_MAU2"]
    fn do_trang_4762_122() {
        let p = std::path::PathBuf::from(std::env::var_os("DEMA4_MAU2").unwrap()).join("HOP 461/4762/4762.pdf");
        let doc = mupdf::Document::open(p.to_str().unwrap()).unwrap();
        let s = 150.0 / 72.0;
        let pix = doc.load_page(121).unwrap().to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_gray(), false, false).unwrap();
        let (w, h, xam) = (pix.width(), pix.height(), pix.samples().to_vec());
        for k in [1.0f32, 0.95, 0.9, 0.85, 0.8, 0.75, 0.7, 0.67, 0.6, 0.5] {
            let (a, w2, h2) = if k < 1.0 { thu_nho(&xam, w, h, k) } else { (xam.clone(), w, h) };
            println!("{k}: {:?}", doc_mot_luot(&a, w2, h2, Binarizer::LocalAverage, true));
        }
    }
}
