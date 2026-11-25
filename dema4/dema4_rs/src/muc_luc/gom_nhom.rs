//! Gom các dòng LIÊN TIẾP cùng giá trị cột nhóm thành hồ sơ; khoá đã đóng mà gặp lại → lỗi.
//! Biến cấp hồ sơ lấy từ DÒNG ĐẦU nhóm (hằng giữ nguyên), biến trong bảng để bước ghép lặp theo dòng.

use std::collections::{HashMap, HashSet};

use super::doc_bang::BangDuLieu;
use super::{Nguon, RangBuoc};

#[derive(Debug, Clone, PartialEq)]
pub struct HoSo {
    pub chi_so: usize,
    pub khoa: String,
    /// Chỉ số dòng (trong `BangDuLieu::dong`) của các văn bản thuộc hồ sơ.
    pub dong: Vec<usize>,
    /// Giá trị cấp hồ sơ: hằng + cột (dòng đầu nhóm) của mọi biến đã ghép.
    pub gia_tri: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoiKhongLienTiep {
    pub khoa: String,
    /// Dòng vật lý trong file (như Excel hiển thị) nơi khoá xuất hiện lại.
    pub dong: usize,
}

fn chuan_khoa(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn gom(bang: &BangDuLieu, rb: &RangBuoc) -> Result<Vec<HoSo>, LoiKhongLienTiep> {
    let mut ra: Vec<HoSo> = Vec::new();
    let mut da_dong: HashSet<String> = HashSet::new();
    for i in 0..bang.dong.len() {
        let khoa = chuan_khoa(bang.o(i, &rb.cot_nhom));
        if let Some(cuoi) = ra.last_mut() {
            if cuoi.khoa == khoa {
                cuoi.dong.push(i);
                continue;
            }
            da_dong.insert(cuoi.khoa.clone());
            if da_dong.contains(&khoa) {
                return Err(LoiKhongLienTiep { khoa, dong: bang.so_dong.get(i).copied().unwrap_or(i + 1) });
            }
        }
        let gia_tri = rb
            .bien
            .iter()
            .filter_map(|(b, n)| match n {
                Nguon::Hang(h) => Some((b.clone(), h.clone())),
                Nguon::Cot(c) => Some((b.clone(), bang.o(i, c).to_string())),
                Nguon::TuDong => None,
            })
            .collect();
        ra.push(HoSo { chi_so: ra.len(), khoa, dong: vec![i], gia_tri });
    }
    Ok(ra)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bang(khoa: &[&str]) -> BangDuLieu {
        BangDuLieu {
            tieu_de: vec!["HS".into(), "TY".into()],
            dong: khoa.iter().enumerate().map(|(i, k)| vec![k.to_string(), format!("vb{i}")]).collect(),
            so_dong: (0..khoa.len()).map(|i| i + 5).collect(),
        }
    }

    fn rb() -> RangBuoc {
        RangBuoc { cot_nhom: "HS".into(), bien: vec![("so".into(), Nguon::Cot("HS".into())), ("ty".into(), Nguon::Cot("TY".into())), ("dv".into(), Nguon::Hang("UB".into())), ("trang_so".into(), Nguon::TuDong)] }
    }

    #[test]
    fn gom_nhom_lien_tiep() {
        let hs = gom(&bang(&["A", "A ", " A", "B", "B  x", "B  x"]), &rb()).unwrap();
        assert_eq!(hs.iter().map(|h| (h.khoa.as_str(), h.dong.len())).collect::<Vec<_>>(), [("A", 3), ("B", 1), ("B x", 2)]);
        assert_eq!((hs[0].gia_tri["ty"].as_str(), hs[0].gia_tri["dv"].as_str()), ("vb0", "UB"), "biến hồ sơ lấy dòng đầu nhóm");
        assert!(!hs[0].gia_tri.contains_key("trang_so"));
        assert_eq!(hs[2].chi_so, 2);
    }

    #[test]
    fn gom_nhom_khong_lien_tiep_bao_loi() {
        assert_eq!(gom(&bang(&["A", "B", "A"]), &rb()), Err(LoiKhongLienTiep { khoa: "A".into(), dong: 7 }), "số dòng vật lý trong file");
    }
}
