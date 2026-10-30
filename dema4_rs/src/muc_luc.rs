//! Mục lục hồ sơ: đọc biên mục Excel/CSV, gom nhóm theo hồ sơ, ghép mẫu DOCX → mỗi hồ sơ 1 mục lục DOCX.
//! Port lõi print_mltl (github.com/quangtv1/print_mltl, C# OpenXML) — giữ đúng thuật toán để ra DOCX tương đương.

pub mod doc_bang;
pub mod docx;
pub mod ghep;
pub mod gom_nhom;
pub mod kiem_tra;
pub mod mau;
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
    /// (cùng định dạng chương trình đối chứng C# dùng).
    #[cfg(test)]
    pub fn tu_json(s: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        let bien = v["bien"]
            .as_object()
            .unwrap()
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
        RangBuoc { cot_nhom: v["nhom"].as_str().unwrap().into(), bien }
    }
}
