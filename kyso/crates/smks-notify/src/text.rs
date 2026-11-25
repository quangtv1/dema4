//! Làm sạch chuỗi trước khi đưa vào sự kiện, tin nhắn và log.

/// Bỏ ký tự điều khiển, gộp khoảng trắng, cắt còn tối đa `max` ký tự (thêm `…` nếu cắt).
pub fn clean(text: &str, max: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max * 4));
    let mut space = false;
    for ch in text.chars() {
        if ch.is_control() || ch.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(ch);
    }
    if out.chars().count() > max {
        let mut cut: String = out.chars().take(max.saturating_sub(1)).collect();
        cut.push('…');
        return cut;
    }
    out
}

/// Ký tự kết thúc một đường dẫn Windows trong câu thông báo.
fn ends_path(ch: char) -> bool {
    matches!(
        ch,
        '"' | '\'' | '<' | '>' | '|' | '*' | '?' | '\n' | '\r' | '\t'
    )
}

/// Thay mọi đường dẫn tuyệt đối (`C:\…`, `\\máy\…`) trong câu bằng riêng tên file: thư mục của người dùng không
/// rời khỏi máy.
pub fn strip_paths(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let drive = chars[i].is_ascii_alphabetic()
            && chars.get(i + 1) == Some(&':')
            && matches!(chars.get(i + 2), Some('\\' | '/'))
            && (i == 0 || !chars[i - 1].is_alphanumeric());
        let unc = chars[i] == '\\' && chars.get(i + 1) == Some(&'\\');
        if !drive && !unc {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut end = i + 2;
        while end < chars.len() && !ends_path(chars[end]) {
            // "…\tên file.pdf: lý do" — dấu hai chấm theo sau là khoảng trắng (hoặc hết câu) kết thúc đường dẫn.
            if chars[end] == ':' && chars.get(end + 1).is_none_or(|c| c.is_whitespace()) {
                break;
            }
            end += 1;
        }
        let path: String = chars[i..end].iter().collect();
        let name = path
            .trim_end_matches(['\\', '/'])
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or("");
        out.push_str(if name.is_empty() || name.ends_with(':') {
            "…"
        } else {
            name
        });
        i = end;
    }
    out
}

/// Thông báo lỗi an toàn để gửi đi: bỏ đường dẫn thư mục, ký tự điều khiển, cắt 300 ký tự.
pub fn safe_error_message(text: &str) -> String {
    clean(&strip_paths(text), 300)
}

/// Thoát ký tự cho `parse_mode=HTML` của Telegram.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Che bí mật khi hiển thị: chỉ giữ 4 ký tự cuối (`••••AbCd`). Chuỗi ngắn thì che hết.
pub fn mask(secret: &str) -> String {
    let n = secret.chars().count();
    if n == 0 {
        return String::new();
    }
    if n <= 8 {
        return "••••".into();
    }
    let tail: String = secret.chars().skip(n - 4).collect();
    format!("••••{tail}")
}

/// Xoá bí mật khỏi một chuỗi sắp ghi log hoặc hiển thị (thông báo lỗi của thư viện HTTP có thể chứa URL).
pub fn redact(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_owned();
    for secret in secrets {
        if secret.len() >= 6 {
            out = out.replace(secret, "***");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_collapses_and_truncates() {
        assert_eq!(clean("  a\r\n\tb   c ", 50), "a b c");
        assert_eq!(clean("abcdefgh", 5), "abcd…");
        assert_eq!(clean("\u{0}\u{7}", 5), "");
    }

    #[test]
    fn paths_are_reduced_to_file_names() {
        assert_eq!(
            safe_error_message(
                r"Không đọc được D:\Hồ sơ mật\Phòng TC\quyet dinh 01.pdf: Access is denied."
            ),
            "Không đọc được quyet dinh 01.pdf: Access is denied."
        );
        assert_eq!(
            safe_error_message(r#"Không tạo được thư mục lưu "C:\Users\an.nv\DaKySo": hết chỗ"#),
            r#"Không tạo được thư mục lưu "DaKySo": hết chỗ"#
        );
        assert_eq!(
            safe_error_message(r"Lỗi ở \\may-chu\chia-se\a.pdf"),
            "Lỗi ở a.pdf"
        );
        assert_eq!(
            safe_error_message(r"Thư mục C:\Users\an.nv\"),
            "Thư mục an.nv"
        );
        // Không phải đường dẫn: giữ nguyên.
        assert_eq!(
            safe_error_message("Tỉ lệ 1:2, mã lỗi 0x80090002"),
            "Tỉ lệ 1:2, mã lỗi 0x80090002"
        );
    }

    #[test]
    fn html_and_masking() {
        assert_eq!(html_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        assert_eq!(mask("1234567890:AAExampleTokenXyZ9"), "••••XyZ9");
        assert_eq!(mask("12345"), "••••");
        assert_eq!(mask(""), "");
        assert_eq!(
            redact("GET https://x/botABCDEF123/getMe lỗi", &["ABCDEF123", ""]),
            "GET https://x/bot***/getMe lỗi"
        );
    }
}
