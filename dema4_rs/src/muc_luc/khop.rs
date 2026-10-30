//! Tự khớp biến ↔ cột như bản gốc (`TextUtil.Normalize`: bỏ dấu, chữ thường, chỉ giữ chữ + số rồi so bằng),
//! chọn cột "Nhóm theo" mặc định, đếm số giá trị khác nhau mỗi cột, thứ tự biến theo nhóm để hiển thị.

use std::collections::HashSet;

use super::doc_bang::BangDuLieu;
use super::mau::MauDaBien;
use super::{Nguon, RangBuoc};

/// Chữ cái tiếng Việt có dấu → chữ không dấu (chữ thường; chữ hoa đã hạ trước khi tra).
fn bo_dau(c: char) -> char {
    const BANG: [(&str, char); 12] = [
        ("àáạảãâầấậẩẫăằắặẳẵ", 'a'),
        ("èéẹẻẽêềếệểễ", 'e'),
        ("ìíịỉĩ", 'i'),
        ("òóọỏõôồốộổỗơờớợởỡ", 'o'),
        ("ùúụủũưừứựửữ", 'u'),
        ("ỳýỵỷỹ", 'y'),
        ("đ", 'd'),
        ("ç", 'c'),
        ("ñ", 'n'),
        ("äå", 'a'),
        ("ëï", 'e'),
        ("öü", 'o'),
    ];
    BANG.iter().find(|(nhom, _)| nhom.contains(c)).map_or(c, |(_, k)| *k)
}

/// Dạng so khớp: bỏ dấu tiếng Việt, chữ thường, bỏ mọi ký tự không phải chữ / số (kể cả `_` và khoảng trắng).
pub fn chuan_hoa(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        // Dấu tổ hợp rời (chữ gõ kiểu NFD) bị bỏ như NonSpacingMark của .NET
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .map(bo_dau)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Tên file không dấu từ tên hiển thị: chữ / số giữ nguyên (bỏ dấu, giữ hoa thường), còn lại gộp thành `_`.
/// Vd "Mẫu 01 – Quảng Trị" → "Mau_01_Quang_Tri".
pub fn ten_file_khong_dau(s: &str) -> String {
    let mut ra = String::new();
    for c in s.chars().filter(|c| !('\u{0300}'..='\u{036F}').contains(c)) {
        let thuong = c.to_lowercase().next().unwrap_or(c);
        let k = bo_dau(thuong);
        if k.is_ascii_alphanumeric() {
            ra.push(if c.is_uppercase() { k.to_ascii_uppercase() } else { k });
        } else if !ra.ends_with('_') && !ra.is_empty() {
            ra.push('_');
        }
    }
    ra.trim_end_matches('_').to_string()
}

/// Biến của mẫu theo 3 nhóm hiển thị (thông tin hồ sơ, bảng văn bản, tự động), mỗi nhóm theo thứ tự xuất hiện.
pub fn nhom_bien(mau: &MauDaBien) -> [Vec<String>; 3] {
    let theo = |tap: &std::collections::BTreeSet<String>| mau.thu_tu.iter().filter(|b| tap.contains(*b)).cloned().collect::<Vec<_>>();
    [theo(&mau.bien_ho_so), theo(&mau.bien_dong), theo(&mau.bien_tu_dong)]
}

/// Cột "Nhóm theo" mặc định: cột đầu tiên có chữ "hoso" (sau chuẩn hoá), không có → cột có tên đầu tiên.
pub fn cot_nhom_mac_dinh(tieu_de: &[String]) -> String {
    tieu_de
        .iter()
        .find(|c| chuan_hoa(c).contains("hoso"))
        .or_else(|| tieu_de.iter().find(|c| !c.trim().is_empty()))
        .cloned()
        .unwrap_or_default()
}

/// Ràng buộc tự khớp: biến (không phải tự động / ảnh) trùng tên cột sau chuẩn hoá → cột đó; tự động → `TuDong`;
/// không trùng → bỏ trống (người dùng chọn tay).
pub fn tu_khop(mau: &MauDaBien, tieu_de: &[String]) -> RangBuoc {
    let [ho_so, dong, tu_dong] = nhom_bien(mau);
    let mut bien = Vec::new();
    for b in ho_so.iter().chain(&dong) {
        let nb = chuan_hoa(b);
        if let Some(c) = tieu_de.iter().find(|c| !nb.is_empty() && chuan_hoa(c) == nb) {
            bien.push((b.clone(), Nguon::Cot(c.clone())));
        }
    }
    bien.extend(tu_dong.into_iter().map(|b| (b, Nguon::TuDong)));
    RangBuoc { cot_nhom: cot_nhom_mac_dinh(tieu_de), bien }
}

/// Số giá trị khác nhau của từng cột (không tính dòng tiêu đề, bỏ ô trống; so sau khi gộp khoảng trắng như gom nhóm).
pub fn dem_khac_nhau(bang: &BangDuLieu) -> Vec<usize> {
    (0..bang.tieu_de.len())
        .map(|c| {
            bang.dong
                .iter()
                .filter_map(|d| d.get(c))
                .map(|o| o.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|o| !o.is_empty())
                .collect::<HashSet<_>>()
                .len()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::mau;
    use super::*;

    #[test]
    fn chuan_hoa_nhu_ban_goc() {
        assert_eq!(chuan_hoa("Số, ký hiệu văn bản"), "sokyhieuvanban");
        assert_eq!(chuan_hoa("so_ky_hieu"), "sokyhieu");
        assert_eq!(chuan_hoa("ĐƠN VỊ"), "donvi");
        assert_eq!(chuan_hoa("Tiêu đề hồ sơ"), "tieudehoso");
        assert_eq!(chuan_hoa("Ngày tháng\u{0300}"), "ngaythang", "dấu tổ hợp rời bị bỏ");
        assert_eq!(chuan_hoa("  "), "");
    }

    #[test]
    fn ten_file_khong_dau_tu_ten_mau() {
        assert_eq!(ten_file_khong_dau("Mẫu 01 – Quảng Trị"), "Mau_01_Quang_Tri");
        assert_eq!(ten_file_khong_dau("Mẫu 03 – Đống Đa"), "Mau_03_Dong_Da");
        assert_eq!(ten_file_khong_dau("  "), "");
    }

    #[test]
    fn tu_khop_theo_ten_cot() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[0].1).unwrap();
        let tieu_de: Vec<String> = ["STT", "Số ký hiệu", "Tác giả", "Trích yếu", "Tờ số", "Ghi chú", "Số hồ sơ", "Đơn vị"].map(String::from).to_vec();
        let rb = tu_khop(&m, &tieu_de);
        assert_eq!(rb.cot("stt"), Some("STT"));
        assert_eq!(rb.cot("so_ky_hieu"), Some("Số ký hiệu"));
        assert_eq!(rb.cot("so_ho_so"), Some("Số hồ sơ"));
        assert_eq!(rb.cot("don_vi"), Some("Đơn vị"));
        assert_eq!(rb.nguon("ngay_thang"), None, "không có cột trùng → để trống");
        assert_eq!(rb.cot_nhom, "Số hồ sơ", "cột nhóm mặc định chứa 'hoso'");
        let m4 = mau::bien_dich(mau::MAU_CO_SAN[3].1).unwrap();
        assert_eq!(tu_khop(&m4, &tieu_de).nguon("trang_so"), Some(&Nguon::TuDong));
        assert_eq!(cot_nhom_mac_dinh(&["".into(), "A".into()]), "A", "không có 'hồ sơ' → cột có tên đầu tiên");
    }

    #[test]
    fn nhom_bien_theo_thu_tu() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[0].1).unwrap();
        let [ho_so, dong, tu_dong] = nhom_bien(&m);
        assert_eq!(ho_so, ["don_vi", "chi_nhanh", "so_ho_so", "nguoi_lap"]);
        assert_eq!(dong[0], "stt");
        assert!(tu_dong.is_empty());
    }

    #[test]
    fn dem_gia_tri_khac_nhau() {
        let bang = BangDuLieu {
            tieu_de: vec!["HS".into(), "TY".into()],
            dong: vec![vec!["A".into(), "x".into()], vec!["A ".into(), "".into()], vec!["B".into(), "x".into()]],
            so_dong: vec![2, 3, 4],
        };
        assert_eq!(dem_khac_nhau(&bang), [2, 1]);
    }
}
