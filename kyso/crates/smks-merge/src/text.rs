//! Chuẩn hoá chữ tiếng Việt.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Bỏ dấu tiếng Việt, đổi `đ`/`Đ` thành `d`/`D`; giữ nguyên chữ hoa thường.
pub fn strip_diacritics(value: &str) -> String {
    value
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| match c {
            'đ' => 'd',
            'Đ' => 'D',
            other => other,
        })
        .collect()
}

/// Khoá so khớp tên biến với tên cột (port `NormalizeWords` của bản cũ):
/// `_` thành khoảng trắng, bỏ dấu, chữ thường, chỉ giữ các cụm chữ và số, nối bằng một khoảng trắng.
pub fn match_key(value: &str) -> String {
    let stripped = strip_diacritics(&value.replace('_', " ")).to_lowercase();
    stripped
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Khoá tìm kiếm không dấu, không phân biệt hoa thường (cho ô tìm ở giao diện và tên file).
pub fn search_key(value: &str) -> String {
    strip_diacritics(value).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_vietnamese_diacritics() {
        assert_eq!(strip_diacritics("Nguyễn Văn Đức"), "Nguyen Van Duc");
        assert_eq!(strip_diacritics("ăắằẳẵặ ƯỨỪỬỮỰ đ"), "aaaaaa UUUUUU d");
    }

    #[test]
    fn match_key_matches_legacy_normalize_words() {
        assert_eq!(match_key("ho_ten"), "ho ten");
        assert_eq!(match_key("  Họ   và Tên "), "ho va ten");
        assert_eq!(match_key("Ngày sinh (dd/mm)"), "ngay sinh dd mm");
        assert_eq!(match_key("Đơn vị"), match_key("don_vi"));
        assert_eq!(match_key("___"), "");
    }
}
