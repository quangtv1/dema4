//! Mục lục hồ sơ: đọc biên mục Excel/CSV, gom nhóm theo hồ sơ, ghép mẫu DOCX → mỗi hồ sơ 1 mục lục DOCX.
//! Port lõi print_mltl (github.com/quangtv1/print_mltl, C# OpenXML) — giữ đúng thuật toán để ra DOCX tương đương.

pub mod doc_bang;
pub mod docx;
pub mod ghep;
pub mod gom_nhom;
pub mod khop;
pub mod kiem_tra;
pub mod mau;
pub mod sinh;
pub mod ten_file;
pub mod token;
pub mod xml;

/// Nguồn giá trị của một biến trong mẫu.
#[derive(Debug, Clone, PartialEq)]
pub enum Nguon {
    /// Cột dữ liệu (biến trong bảng: theo từng dòng; biến hồ sơ: dòng đầu nhóm).
    Cot(String),
    /// Giá trị cố định cho mọi hồ sơ.
    Hang(String),
    /// `{trang_so}` / `{tong_so_trang}`: field Word tự tính.
    TuDong,
}

/// Ràng buộc biến + cột gom nhóm cho một lần tạo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RangBuoc {
    pub cot_nhom: String,
    pub bien: Vec<(String, Nguon)>,
}

impl RangBuoc {
    pub fn nguon(&self, bien: &str) -> Option<&Nguon> {
        self.bien.iter().find(|(b, _)| b == bien).map(|(_, n)| n)
    }

    /// Cột của biến (nếu nguồn là cột).
    pub fn cot(&self, bien: &str) -> Option<&str> {
        match self.nguon(bien) {
            Some(Nguon::Cot(c)) => Some(c),
            _ => None,
        }
    }

    /// Đọc dạng JSON `{"nhom": "...", "bien": {"x": {"cot": ".."} | {"hang": ".."} | {"tu_dong": true}}}`
    /// (cùng định dạng chương trình đối chứng C# dùng; app lưu lựa chọn gần nhất trong cấu hình). Sai → `None`.
    pub fn doc_json(s: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        let bien = v["bien"]
            .as_object()?
            .iter()
            .map(|(k, n)| {
                let nguon = if let Some(c) = n["cot"].as_str() {
                    Nguon::Cot(c.into())
                } else if let Some(h) = n["hang"].as_str() {
                    Nguon::Hang(h.into())
                } else {
                    Nguon::TuDong
                };
                (k.clone(), nguon)
            })
            .collect();
        Some(RangBuoc { cot_nhom: v["nhom"].as_str()?.into(), bien })
    }

    /// JSON một dòng (ngược với `doc_json`).
    pub fn json(&self) -> String {
        let bien: serde_json::Map<String, serde_json::Value> = self
            .bien
            .iter()
            .map(|(b, n)| {
                let v = match n {
                    Nguon::Cot(c) => serde_json::json!({ "cot": c }),
                    Nguon::Hang(h) => serde_json::json!({ "hang": h }),
                    Nguon::TuDong => serde_json::json!({ "tu_dong": true }),
                };
                (b.clone(), v)
            })
            .collect();
        serde_json::json!({ "nhom": self.cot_nhom, "bien": bien }).to_string()
    }

    #[cfg(test)]
    pub fn tu_json(s: &str) -> Self {
        Self::doc_json(s).expect("JSON ràng buộc hợp lệ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rang_buoc_json_khu_hoi() {
        let rb = RangBuoc { cot_nhom: "Số HS".into(), bien: vec![("a".into(), Nguon::Cot("C=1|x".into())), ("b".into(), Nguon::Hang("Đơn vị \"A\"\n2".into())), ("trang_so".into(), Nguon::TuDong)] };
        let mut doc = RangBuoc::doc_json(&rb.json()).unwrap();
        doc.bien.sort_by(|x, y| x.0.cmp(&y.0));
        assert_eq!(doc, rb);
        assert!(!rb.json().contains('\n'), "một dòng (lưu được trong file cấu hình)");
        assert_eq!(RangBuoc::doc_json("rác"), None);
        assert_eq!(RangBuoc::doc_json(r#"{"bien":{}}"#), None, "thiếu nhom");
    }
}
