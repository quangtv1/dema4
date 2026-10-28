//! Cắt PDF hồ sơ thành file văn bản: chép nguyên luồng ảnh scan (không nén lại), áp góc xoay bằng `/Rotate`.
//!
//! Có 2 cách, giữ cả hai để so ở bước thử (test `#[ignore]` đo trên mẫu 1):
//! - `cat_bang_ghep`: tạo PDF mới, ghép từng trang nguồn qua 1 bảng ghép (graft map) chung
//!   → tài nguyên dùng chung giữa các trang chỉ chép 1 lần;
//! - `cat_bang_xoa`: mở lại PDF nguồn, xoá các trang ngoài dải rồi lưu có dọn đối tượng thừa.

use mupdf::pdf::{PdfDocument, PdfObject, PdfWriteOptions};
use std::path::Path;

/// Thuộc tính trang có thể kế thừa từ nút cha trong cây trang (PDF 1.7, mục 7.7.3.4).
const KHOA_KE_THUA: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

fn loi(e: mupdf::Error) -> String {
    e.to_string()
}

/// Góc `/Rotate` sau khi cộng thêm `goc` (theo chiều kim đồng hồ, bội của 90).
fn goc_moi(goc_cu: i32, goc: u16) -> i32 {
    (goc_cu + goc as i32).rem_euclid(360)
}

/// Tạo `dich` gồm các trang `trang` (1-based, đúng thứ tự, kèm góc xoay thêm) của `nguon`.
pub fn cat_bang_ghep(nguon: &Path, trang: &[(usize, u16)], dich: &Path) -> Result<(), String> {
    let src = PdfDocument::open(nguon.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(loi)?;
    let mut dst = PdfDocument::new();
    let mut ghep = dst.new_graft_map().map_err(loi)?;
    for (thu_tu, &(so, goc)) in trang.iter().enumerate() {
        let trang_nguon = src.find_page(so as i32 - 1).map_err(loi)?;
        let mut moi = dst.new_dict().map_err(loi)?;
        for cap in trang_nguon.dict_iter().map_err(loi)? {
            let (khoa, gia_tri) = cap.map_err(loi)?;
            if khoa.as_name().map_err(loi)? == b"Parent" {
                continue;
            }
            moi.dict_put(khoa, ghep.graft_object(&gia_tri).map_err(loi)?).map_err(loi)?;
        }
        for khoa in KHOA_KE_THUA {
            if moi.get_dict(khoa).map_err(loi)?.is_none() {
                if let Some(v) = trang_nguon.get_dict_inheritable(khoa).map_err(loi)? {
                    moi.dict_put(khoa, ghep.graft_object(&v).map_err(loi)?).map_err(loi)?;
                }
            }
        }
        dat_goc(&dst, &mut moi, goc)?;
        let obj = dst.add_object(&moi).map_err(loi)?;
        dst.insert_page(thu_tu as i32, &obj).map_err(loi)?;
    }
    dst.save_with_options(dich.to_str().ok_or("đường dẫn không hợp lệ")?, PdfWriteOptions::default()).map_err(loi)
}

/// Như `cat_bang_ghep` nhưng mở lại nguồn, xoá trang ngoài danh sách (giữ thứ tự gốc) rồi dọn đối tượng thừa.
pub fn cat_bang_xoa(nguon: &Path, trang: &[(usize, u16)], dich: &Path) -> Result<(), String> {
    let mut doc = PdfDocument::open(nguon.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(loi)?;
    let tong = doc.page_count().map_err(loi)? as usize;
    let giu: std::collections::HashMap<usize, u16> = trang.iter().copied().collect();
    for so in (1..=tong).rev() {
        if !giu.contains_key(&so) {
            doc.delete_page(so as i32 - 1).map_err(loi)?;
        }
    }
    let mut con_lai: Vec<usize> = giu.keys().copied().collect();
    con_lai.sort_unstable();
    for (i, so) in con_lai.iter().enumerate() {
        let mut obj = doc.find_page(i as i32).map_err(loi)?;
        let goc = giu[so];
        if goc != 0 {
            let cu = obj.get_dict_inheritable("Rotate").map_err(loi)?.map(|v| v.as_int().unwrap_or(0)).unwrap_or(0);
            let moi = doc.new_int(goc_moi(cu, goc)).map_err(loi)?;
            obj.dict_put("Rotate", moi).map_err(loi)?;
        }
    }
    let mut tuy_chon = PdfWriteOptions::default();
    tuy_chon.set_garbage_level(2);
    doc.save_with_options(dich.to_str().ok_or("đường dẫn không hợp lệ")?, tuy_chon).map_err(loi)
}

fn dat_goc(doc: &PdfDocument, trang: &mut PdfObject, goc: u16) -> Result<(), String> {
    if goc == 0 {
        return Ok(());
    }
    let cu = trang.get_dict("Rotate").map_err(loi)?.map(|v| v.as_int().unwrap_or(0)).unwrap_or(0);
    let moi = doc.new_int(goc_moi(cu, goc)).map_err(loi)?;
    trang.dict_put("Rotate", moi).map_err(loi)
}

/// Băm luồng ảnh nhúng (dữ liệu nén thô) của 1 trang (1-based) — so nội dung trang mà không phụ thuộc cách ghi PDF.
#[cfg(test)]
pub(crate) fn hash_anh_trang(pdf: &Path, so: usize) -> Vec<u64> {
    use std::hash::{Hash, Hasher};
    let doc = PdfDocument::open(pdf.to_str().unwrap()).unwrap();
    let trang = doc.find_page(so as i32 - 1).unwrap();
    let Some(tai_nguyen) = trang.get_dict_inheritable("Resources").unwrap() else { return Vec::new() };
    let Some(xobject) = tai_nguyen.get_dict("XObject").unwrap() else { return Vec::new() };
    let mut ds = Vec::new();
    for cap in xobject.dict_iter().unwrap() {
        let (_, anh) = cap.unwrap();
        if anh.get_dict("Subtype").unwrap().and_then(|v| v.as_name().ok()).as_deref() == Some(b"Image".as_slice()) {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            anh.read_raw_stream().unwrap().hash(&mut h);
            ds.push(h.finish());
        }
    }
    ds.sort_unstable();
    ds
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn goc_moi_cong_don_theo_vong() {
        assert_eq!(goc_moi(0, 90), 90);
        assert_eq!(goc_moi(270, 180), 90);
        assert_eq!(goc_moi(90, 270), 0);
    }

    /// Mẫu 1: L1_05_13 trang 1–9 phải trùng `pdf_vanban/L1_05_13_part1.pdf` của b3 (so băm ảnh từng trang);
    /// đo cả 2 cách cắt cho toàn bộ 211 dải của `ketqua_tach.txt`.
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn cat_dai_trang_giu_nguyen_anh() {
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let nguon = mau.join("pdf_goc/L1_05_13.pdf");
        let b3 = mau.join("pdf_vanban/L1_05_13_part1.pdf");
        let tam = std::env::temp_dir().join("dema4_cat_thu");
        std::fs::create_dir_all(&tam).unwrap();
        let dai: Vec<(usize, u16)> = (1..=9).map(|t| (t, 0)).collect();
        for (ten, ham) in [("ghep", cat_bang_ghep as fn(&Path, &[(usize, u16)], &Path) -> Result<(), String>), ("xoa", cat_bang_xoa)] {
            let ra = tam.join(format!("part1_{ten}.pdf"));
            ham(&nguon, &dai, &ra).unwrap();
            for t in 1..=9 {
                assert_eq!(hash_anh_trang(&ra, t), hash_anh_trang(&b3, t), "{ten}: trang {t}");
                assert_eq!(hash_anh_trang(&ra, t), hash_anh_trang(&nguon, t), "{ten}: trang {t} khác nguồn");
            }
        }
        // Xoay: trang ra mang đúng /Rotate
        let ra = tam.join("xoay.pdf");
        cat_bang_ghep(&nguon, &[(1, 90), (2, 270)], &ra).unwrap();
        let doc = PdfDocument::open(ra.to_str().unwrap()).unwrap();
        let goc = |i| doc.find_page(i).unwrap().get_dict("Rotate").unwrap().unwrap().as_int().unwrap();
        assert_eq!((goc(0), goc(1)), (90, 270));

        // Đo 211 dải của b3 bằng 2 cách: thời gian + tổng dung lượng file ra
        let ketqua = std::fs::read_to_string(mau.join("ketqua_tach.txt")).unwrap();
        let dong: Vec<(String, usize, usize)> = ketqua
            .lines()
            .skip(1)
            .filter_map(|d| {
                let c: Vec<&str> = d.split('\t').collect();
                let (a, b) = c.get(2)?.split_once('-')?;
                Some((c[1].to_string(), a.parse().ok()?, b.parse().ok()?))
            })
            .collect();
        for (ten, ham) in [("ghep", cat_bang_ghep as fn(&Path, &[(usize, u16)], &Path) -> Result<(), String>), ("xoa", cat_bang_xoa)] {
            let t0 = Instant::now();
            let mut tong = 0u64;
            for (i, (file, a, b)) in dong.iter().enumerate() {
                let ra = tam.join(format!("{ten}_{i}.pdf"));
                let dai: Vec<(usize, u16)> = (*a..=*b).map(|t| (t, 0)).collect();
                ham(&mau.join("pdf_goc").join(file), &dai, &ra).unwrap();
                tong += std::fs::metadata(&ra).unwrap().len();
                std::fs::remove_file(&ra).ok();
            }
            println!("{ten}: {} part, {:.1} s, tổng {:.1} MB", dong.len(), t0.elapsed().as_secs_f64(), tong as f64 / 1e6);
        }
    }
}
