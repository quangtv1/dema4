//! Tên file mục lục từ mẫu (vd `MLHS_{so_ho_so}`): thay biến của hồ sơ, bỏ ký tự cấm, trùng tên thêm `_2`, `_3`…

use std::collections::HashSet;

use super::gom_nhom::HoSo;
use super::token;

pub const MAU_MAC_DINH: &str = "MLHS_{so_ho_so}";

fn sach(s: &str) -> String {
    let s: String = s.chars().map(|c| if c.is_control() || r#"\/:*?"<>|"#.contains(c) { '_' } else { c }).collect();
    s.trim().trim_end_matches('.').trim().to_string()
}

/// Tên (không đuôi) cho từng hồ sơ, theo thứ tự; trùng (không phân biệt hoa thường) → `_2`, `_3`.
pub fn dat_ten(mau: &str, ho_so: &[HoSo]) -> Vec<String> {
    let mut da_dung: HashSet<String> = HashSet::new();
    ho_so
        .iter()
        .map(|hs| {
            // Biến trong mẫu tên đều trống (vd khoá hồ sơ rỗng) → phần chữ cố định + số thứ tự, như bản gốc
            let co_gia_tri = token::tim(mau).iter().any(|(_, _, t)| hs.gia_tri.get(*t).is_some_and(|v| !v.trim().is_empty()));
            let goc = if co_gia_tri || token::tim(mau).is_empty() {
                sach(&token::thay(mau, &|t| hs.gia_tri.get(t).cloned()))
            } else {
                sach(&format!("{}{}", token::thay(mau, &|_| Some(String::new())), hs.chi_so + 1))
            };
            let goc = if goc.is_empty() { format!("MLHS_{}", hs.chi_so + 1) } else { goc };
            let mut ten = goc.clone();
            let mut n = 1;
            while !da_dung.insert(ten.to_lowercase()) {
                n += 1;
                ten = format!("{goc}_{n}");
            }
            ten
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn hs(i: usize, so: &str) -> HoSo {
        HoSo { chi_so: i, khoa: so.into(), dong: vec![], gia_tri: HashMap::from([("so_ho_so".to_string(), so.to_string())]) }
    }

    #[test]
    fn ten_file_trung_ky_tu_cam() {
        let ds = [hs(0, "HS/01"), hs(1, "hs_01"), hs(2, "HS_01"), hs(3, "  "), hs(4, "A:B*?. ")];
        assert_eq!(dat_ten(MAU_MAC_DINH, &ds), ["MLHS_HS_01", "MLHS_hs_01_2", "MLHS_HS_01_3", "MLHS_4", "MLHS_A_B__"]);
        assert_eq!(dat_ten("{khong_co}", &ds[..1]), ["1"], "biến không có giá trị → số thứ tự");
        assert_eq!(dat_ten("", &ds[..1]), ["MLHS_1"]);
    }
}
