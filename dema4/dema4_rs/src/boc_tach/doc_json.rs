//! Đọc JSON trong câu trả lời của mô hình: chấp nhận khối ```json, chữ thừa trước / sau, chỉ lấy object đầu tiên.

use serde_json::{Map, Value};

/// Object JSON đầu tiên trong câu trả lời; không có → `None`.
pub fn doc(tra_loi: &str) -> Option<Map<String, Value>> {
    for (i, _) in tra_loi.match_indices('{') {
        let mut it = serde_json::Deserializer::from_str(&tra_loi[i..]).into_iter::<Value>();
        if let Some(Ok(Value::Object(m))) = it.next() {
            return Some(m);
        }
    }
    None
}

/// Như `doc`, ưu tiên object có ít nhất 1 khoá trong `bien` (câu trả lời có thể chứa JSON khác trước kết quả).
pub fn doc_khop(tra_loi: &str, bien: &[&str]) -> Option<Map<String, Value>> {
    let mut dau = None;
    for (i, _) in tra_loi.match_indices('{') {
        let mut it = serde_json::Deserializer::from_str(&tra_loi[i..]).into_iter::<Value>();
        if let Some(Ok(Value::Object(m))) = it.next() {
            if bien.iter().any(|b| m.contains_key(*b)) {
                return Some(m);
            }
            dau.get_or_insert(m);
        }
    }
    dau
}

/// Giá trị của `bien` dạng chuỗi: thiếu / null → rỗng; số → chữ số; mảng → nối bằng ", ".
pub fn chuoi(m: &Map<String, Value>, bien: &str) -> String {
    fn ra_chuoi(v: &Value) -> String {
        match v {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            Value::Array(a) => a.iter().map(ra_chuoi).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
            khac => khac.to_string(),
        }
    }
    m.get(bien).map(ra_chuoi).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiu_khoi_ma_va_chu_thua() {
        let m = doc("Kết quả:\n```json\n{\"so_ky_hieu\": \"36/BC-NTTS\", \"so_trang\": 4, \"ds\": [\"a\", \"b\"], \"x\": null}\n```\nHết.").unwrap();
        assert_eq!(chuoi(&m, "so_ky_hieu"), "36/BC-NTTS");
        assert_eq!(chuoi(&m, "so_trang"), "4");
        assert_eq!(chuoi(&m, "ds"), "a, b");
        assert_eq!(chuoi(&m, "x"), "");
        assert_eq!(chuoi(&m, "khong_co"), "");
    }

    #[test]
    fn uu_tien_object_co_khoa_can_doc() {
        let m = doc_khop(r#"Ví dụ {"x": 1} và kết quả {"so_ky_hieu": "1/QĐ"}"#, &["so_ky_hieu"]).unwrap();
        assert_eq!(chuoi(&m, "so_ky_hieu"), "1/QĐ");
        assert!(doc_khop(r#"{"x": 1}"#, &["a"]).is_some(), "không object nào khớp → object đầu");
    }

    #[test]
    fn bo_qua_ngoac_khong_phai_json() {
        let m = doc("Mẫu {sai} rồi {\"a\": \"1\"}").unwrap();
        assert_eq!(chuoi(&m, "a"), "1");
        assert!(doc("không có JSON").is_none());
        assert!(doc("{\"a\": ").is_none());
    }
}
