//! Token `{ten}` trong mẫu: ký tự tên = chữ Latin không dấu, số, `_` (như regex `\{([a-zA-Z0-9_]+)\}` của bản gốc).

fn la_ky_tu_ten(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Các token theo thứ tự, không chồng lấn: (vị trí byte bắt đầu, độ dài byte, tên).
pub fn tim(s: &str) -> Vec<(usize, usize, &str)> {
    let mut ra = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let mut j = i + 1;
            while j < b.len() && la_ky_tu_ten(b[j] as char) {
                j += 1;
            }
            if j > i + 1 && j < b.len() && b[j] == b'}' {
                ra.push((i, j + 1 - i, &s[i + 1..j]));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    ra
}

/// Đuôi chuỗi là phần mở của một token chưa đóng: `{` + ký tự tên tới hết chuỗi (`\{[a-zA-Z0-9_]*$`).
pub fn duoi_mo(s: &str) -> bool {
    s.rfind('{').is_some_and(|i| s[i + 1..].chars().all(la_ky_tu_ten))
}

/// Đầu chuỗi là một token trọn vẹn (`^\{[a-zA-Z0-9_]+\}`).
pub fn dau_dong(s: &str) -> bool {
    tim(s).first().is_some_and(|(i, _, _)| *i == 0)
}

/// Thay mọi token: `gia_tri(ten)` = Some → thay, None → giữ nguyên token.
pub fn thay(s: &str, gia_tri: &dyn Fn(&str) -> Option<String>) -> String {
    let mut ra = String::with_capacity(s.len());
    let mut cuoi = 0;
    for (i, n, ten) in tim(s) {
        ra.push_str(&s[cuoi..i]);
        match gia_tri(ten) {
            Some(v) => ra.push_str(&v),
            None => ra.push_str(&s[i..i + n]),
        }
        cuoi = i + n;
    }
    ra.push_str(&s[cuoi..]);
    ra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tim_token_nhu_regex_goc() {
        assert_eq!(tim("a {x} {y_1}{z}").iter().map(|t| t.2).collect::<Vec<_>>(), ["x", "y_1", "z"]);
        assert_eq!(tim("{{a}").iter().map(|t| (t.0, t.2)).collect::<Vec<_>>(), [(1, "a")]);
        assert!(tim("{} {a b} {đ} {a").is_empty());
        assert!(duoi_mo("x {so_") && duoi_mo("{") && !duoi_mo("{a b") && !duoi_mo("abc"));
        assert!(dau_dong("{a}b") && !dau_dong(" {a}") && !dau_dong("{a"));
        assert_eq!(thay("{a}-{b}", &|t| (t == "a").then(|| "1".into())), "1-{b}");
    }
}
