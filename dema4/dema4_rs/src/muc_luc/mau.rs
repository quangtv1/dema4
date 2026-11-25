//! Biên dịch mẫu DOCX một lần: tìm bảng dữ liệu + HÀNG MẪU (hàng có nhiều biến nhất), phân loại biến
//! (trong bảng / cấp hồ sơ / tự động / ảnh), thứ tự xuất hiện để sắp bảng ghép biến.

use std::collections::BTreeSet;

use super::docx::{Docx, PART_CHINH};
use super::token;
use super::xml::{self, The};

/// Biến tự động: engine chèn field PAGE / NUMPAGES, Word tự tính khi mở / in.
pub const TU_DONG: [&str; 2] = ["trang_so", "tong_so_trang"];

/// 4 mẫu nhúng sẵn (Quảng Trị, Quảng Ninh, Đống Đa, Vĩnh Phúc — từ print_mltl).
pub const MAU_CO_SAN: [(&str, &[u8]); 4] = [
    ("Mẫu 01 – Quảng Trị", include_bytes!("../../assets/muc_luc/mau-01.docx")),
    ("Mẫu 02 – Quảng Ninh", include_bytes!("../../assets/muc_luc/mau-02.docx")),
    ("Mẫu 03 – Đống Đa", include_bytes!("../../assets/muc_luc/mau-03.docx")),
    ("Mẫu 04 – Vĩnh Phúc", include_bytes!("../../assets/muc_luc/mau-04.docx")),
];

#[derive(Debug, Clone)]
pub struct MauDaBien {
    pub bytes: Vec<u8>,
    /// Chỉ số bảng dữ liệu (theo thứ tự tài liệu, kể cả bảng lồng).
    pub bang: usize,
    /// Chỉ số hàng mẫu trong bảng (hàng trực tiếp).
    pub hang_mau: usize,
    pub bien_dong: BTreeSet<String>,
    pub bien_ho_so: BTreeSet<String>,
    pub bien_tu_dong: BTreeSet<String>,
    /// Token chữ `{image…}`: giá trị là đường dẫn ảnh, chèn ảnh tại chỗ.
    pub bien_anh_token: BTreeSet<String>,
    /// Ảnh có Alt Text / tên bắt đầu bằng `image` (đổi ruột ảnh).
    pub bien_anh: BTreeSet<String>,
    /// Thứ tự biến xuất hiện: thân → đầu trang → chân trang → ảnh.
    pub thu_tu: Vec<String>,
}

fn token_cua(t: &The) -> Vec<String> {
    token::tim(&t.chu_w_t()).into_iter().map(|(_, _, n)| n.to_string()).collect()
}

fn chon_anh(gia_tri: &[Option<&str>]) -> Option<String> {
    gia_tri.iter().flatten().map(|v| v.trim()).find(|v| !v.is_empty() && v.to_ascii_lowercase().starts_with("image")).map(String::from)
}

/// Tên biến ảnh của một `w:drawing` (alt text / title / tên của `wp:docPr`, rồi `pic:cNvPr`).
pub fn ten_anh(drawing: &The) -> Option<String> {
    let mut dp = Vec::new();
    drawing.hau_due("wp:docPr", &mut dp);
    for d in dp {
        if let Some(s) = chon_anh(&[d.thuoc_tinh("descr"), d.thuoc_tinh("title"), d.thuoc_tinh("name")]) {
            return Some(s);
        }
    }
    let mut cn = Vec::new();
    drawing.hau_due("pic:cNvPr", &mut cn);
    cn.into_iter().find_map(|c| chon_anh(&[c.thuoc_tinh("descr"), c.thuoc_tinh("name")]))
}

fn anh_trong(t: &The) -> Vec<String> {
    let mut d = Vec::new();
    t.hau_due("w:drawing", &mut d);
    d.into_iter().filter_map(ten_anh).collect()
}

pub fn bien_dich(bytes: &[u8]) -> Result<MauDaBien, String> {
    let docx = Docx::doc(bytes)?;
    let tai_lieu = docx.xml(PART_CHINH)?;
    let goc = xml::goc(&tai_lieu).ok_or("document.xml rỗng")?;
    let than = goc.con_dau("w:body").ok_or("document.xml thiếu w:body")?;

    let mut bang = Vec::new();
    than.hau_due("w:tbl", &mut bang);
    let (mut tot_bang, mut tot_hang, mut tot_so) = (0, 0, 0);
    let mut bien_dong = BTreeSet::new();
    for (bi, b) in bang.iter().enumerate() {
        for (hi, h) in b.the_con().filter(|t| t.ten == "w:tr").enumerate() {
            let tap: BTreeSet<String> = token_cua(h).into_iter().collect();
            if tap.len() > tot_so {
                (tot_bang, tot_hang, tot_so, bien_dong) = (bi, hi, tap.len(), tap);
            }
        }
    }
    if tot_so == 0 {
        return Err("mẫu không có bảng nào chứa biến {…} (cần một hàng mẫu trong bảng)".into());
    }

    let mut phan: Vec<Vec<xml::Nut>> = Vec::new();
    for ten in docx.dau_chan_trang() {
        phan.push(docx.xml(&ten)?);
    }
    fn them(thu_tu: &mut Vec<String>, ds: Vec<String>) {
        for n in ds {
            if !thu_tu.contains(&n) {
                thu_tu.push(n);
            }
        }
    }
    let mut thu_tu: Vec<String> = Vec::new();
    them(&mut thu_tu, token_cua(than));
    for g in phan.iter().filter_map(|p| xml::goc(p)) {
        them(&mut thu_tu, token_cua(g));
    }
    let tat_ca: BTreeSet<String> = thu_tu.iter().cloned().collect();
    let mut ds_anh = anh_trong(than);
    for g in phan.iter().filter_map(|p| xml::goc(p)) {
        ds_anh.extend(anh_trong(g));
    }
    let bien_anh: BTreeSet<String> = ds_anh.iter().cloned().collect();
    them(&mut thu_tu, ds_anh);

    Ok(MauDaBien {
        bytes: bytes.to_vec(),
        bang: tot_bang,
        hang_mau: tot_hang,
        bien_tu_dong: tat_ca.iter().filter(|b| TU_DONG.contains(&b.as_str())).cloned().collect(),
        bien_ho_so: tat_ca.iter().filter(|b| !bien_dong.contains(*b) && !TU_DONG.contains(&b.as_str())).cloned().collect(),
        bien_anh_token: tat_ca.iter().chain(&bien_dong).filter(|b| b.to_ascii_lowercase().starts_with("image")).cloned().collect(),
        bien_dong,
        bien_anh,
        thu_tu,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(t: &BTreeSet<String>) -> Vec<&str> {
        t.iter().map(String::as_str).collect()
    }

    /// Khớp kết quả TemplateCompiler gốc trên 4 mẫu (bảng, hàng mẫu, biến dòng / hồ sơ / tự động).
    #[test]
    fn bien_dich_4_mau() {
        let mong_doi: [(usize, &[&str], &[&str], &[&str]); 4] = [
            (1, &["ghi_chu", "ngay_thang", "so_ky_hieu", "stt", "tac_gia", "to_so", "trich_yeu"], &["chi_nhanh", "don_vi", "nguoi_lap", "so_ho_so"], &[]),
            (0, &["ghi_chu", "ngay_thang", "so_ky_hieu", "tac_gia", "trang_tu_den", "trich_yeu", "tt"], &["ho_so_so", "muc_luc_hs_so", "phong_so", "tieu_de_tap"], &[]),
            (0, &["ghi_chu", "loai_vb", "ngay_ky", "sl_trang", "so_ky_hieu", "stt", "tac_gia", "to_so", "trich_yeu"], &["ho_so_so"], &[]),
            (0, &["ghi_chu", "ngay_thang", "so_ky_hieu", "stt", "tac_gia", "to_so", "trich_yeu"], &["so_ho_so", "ten_ho_so"], &["tong_so_trang", "trang_so"]),
        ];
        for ((ten, b), (bang, dong, ho_so, tu_dong)) in MAU_CO_SAN.iter().zip(mong_doi) {
            let m = bien_dich(b).unwrap_or_else(|e| panic!("{ten}: {e}"));
            assert_eq!((m.bang, m.hang_mau), (bang, 1), "{ten}");
            assert_eq!(ds(&m.bien_dong), dong, "{ten}");
            assert_eq!(ds(&m.bien_ho_so), ho_so, "{ten}");
            assert_eq!(ds(&m.bien_tu_dong), tu_dong, "{ten}");
            assert!(m.bien_anh.is_empty() && m.bien_anh_token.is_empty());
        }
        let m = bien_dich(MAU_CO_SAN[0].1).unwrap();
        assert_eq!(&m.thu_tu[..4], ["don_vi", "chi_nhanh", "so_ho_so", "stt"], "thứ tự đọc trong tài liệu");
    }

    #[test]
    fn mau_khong_co_bang_bien_bao_loi() {
        assert!(bien_dich(b"khong phai zip").is_err());
    }
}
