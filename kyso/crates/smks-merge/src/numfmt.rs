//! Áp mã định dạng số của Excel để ra chuỗi giống `GetFormattedString()` của ClosedXML (bản .NET 1.4.0).
//!
//! Hỗ trợ: `General` (15 chữ số có nghĩa như `G15` của .NET), chữ số `0 # ?`, nhóm nghìn `,`, chia nghìn
//! bằng `,` cuối, phần trăm `%`, dạng khoa học `E+00`, chuỗi cố định (`"..."`, `\x`), `_x` (khoảng trắng),
//! `*x` (bỏ qua), `[...]` (màu, điều kiện, mã vùng: bỏ qua; `[$ký hiệu-...]`: giữ ký hiệu), tối đa 3 phần
//! `dương;âm;không`, và định dạng giờ (`h m s AM/PM`, `[h]`).
//!
//! Dấu thập phân và dấu nhóm nghìn lấy từ [`NumberLocale`] (bản cũ dùng vùng văn hoá của Windows).
//! Mã định dạng không hiểu được thì trả về dạng `General`.

/// Dấu thập phân và dấu nhóm nghìn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberLocale {
    pub decimal: char,
    pub group: char,
}

impl NumberLocale {
    /// Kiểu Mỹ: `1,234,567.50`.
    pub const EN_US: Self = Self {
        decimal: '.',
        group: ',',
    };
    /// Kiểu Việt Nam: `1.234.567,50`.
    pub const VI_VN: Self = Self {
        decimal: ',',
        group: '.',
    };
}

impl Default for NumberLocale {
    fn default() -> Self {
        Self::EN_US
    }
}

/// Định dạng `General` của ClosedXML/.NET (`G15`): tối đa 15 chữ số có nghĩa; dạng khoa học khi số mũ
/// ≥ 15 hoặc ≤ -5, ví dụ `1E-07`, `1.23456789012346E+16`.
pub fn format_general(value: f64, locale: NumberLocale) -> String {
    if value == 0.0 || !value.is_finite() {
        return if value.is_nan() {
            "NaN".into()
        } else if value == 0.0 {
            "0".into()
        } else {
            value.to_string()
        };
    }
    let negative = value < 0.0;
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };

    let body = if exponent >= 15 || exponent <= -5 {
        let (first, rest) = digits.split_at(1);
        let mut out = first.to_string();
        if !rest.is_empty() {
            out.push(locale.decimal);
            out.push_str(rest);
        }
        out.push_str(&format!(
            "E{}{:02}",
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        ));
        out
    } else if exponent < 0 {
        let mut out = String::from("0");
        out.push(locale.decimal);
        out.push_str(&"0".repeat((-exponent - 1) as usize));
        out.push_str(digits);
        out
    } else {
        let int_len = exponent as usize + 1;
        if digits.len() <= int_len {
            format!("{digits}{}", "0".repeat(int_len - digits.len()))
        } else {
            let (int_part, frac_part) = digits.split_at(int_len);
            format!("{int_part}{}{frac_part}", locale.decimal)
        }
    };
    if negative { format!("-{body}") } else { body }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// Chữ số: `'0'`, `'#'` hoặc `'?'`.
    Digit(char),
    Point,
    Comma,
    Percent,
    /// `E+` hoặc `E-`; `true` nếu luôn in dấu.
    Exponent(bool),
    Literal(String),
    Text,
    /// Thành phần ngày giờ: `y m d h s`, chuỗi gốc như `hh`, `mm`, `AM/PM`, `[h]`.
    Time(String),
}

fn split_sections(code: &str) -> Vec<String> {
    let mut sections = vec![String::new()];
    let mut chars = code.chars().peekable();
    let mut in_quote = false;
    let mut in_bracket = false;
    while let Some(c) = chars.next() {
        let last = sections.len() - 1;
        match c {
            '"' if !in_bracket => {
                in_quote = !in_quote;
                sections[last].push(c);
            }
            '\\' if !in_quote && !in_bracket => {
                sections[last].push(c);
                if let Some(next) = chars.next() {
                    sections[last].push(next);
                }
            }
            '[' if !in_quote => {
                in_bracket = true;
                sections[last].push(c);
            }
            ']' if !in_quote => {
                in_bracket = false;
                sections[last].push(c);
            }
            ';' if !in_quote && !in_bracket => sections.push(String::new()),
            _ => sections[last].push(c),
        }
    }
    sections
}

fn tokenize(section: &str) -> Vec<Token> {
    let chars: Vec<char> = section.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let push_literal = |tokens: &mut Vec<Token>, text: &str| {
        if let Some(Token::Literal(previous)) = tokens.last_mut() {
            previous.push_str(text);
        } else {
            tokens.push(Token::Literal(text.to_owned()));
        }
    };
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' => {
                let end = chars[i + 1..]
                    .iter()
                    .position(|x| *x == '"')
                    .map_or(chars.len(), |p| i + 1 + p);
                let text: String = chars[i + 1..end].iter().collect();
                push_literal(&mut tokens, &text);
                i = end + 1;
            }
            '\\' => {
                if let Some(next) = chars.get(i + 1) {
                    push_literal(&mut tokens, &next.to_string());
                }
                i += 2;
            }
            '_' => {
                push_literal(&mut tokens, " ");
                i += 2;
            }
            '*' => i += 2,
            '[' => {
                let end = chars[i..]
                    .iter()
                    .position(|x| *x == ']')
                    .map_or(chars.len(), |p| i + p);
                let inner: String = chars[i + 1..end].iter().collect();
                let lower = inner.to_ascii_lowercase();
                if let Some(rest) = inner.strip_prefix('$') {
                    let symbol = rest.split('-').next().unwrap_or_default();
                    if !symbol.is_empty() {
                        push_literal(&mut tokens, symbol);
                    }
                } else if lower == "h"
                    || lower == "hh"
                    || lower == "m"
                    || lower == "mm"
                    || lower == "s"
                    || lower == "ss"
                {
                    tokens.push(Token::Time(format!("[{lower}]")));
                }
                i = end + 1;
            }
            '0' | '#' | '?' => {
                tokens.push(Token::Digit(c));
                i += 1;
            }
            '.' => {
                tokens.push(Token::Point);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '@' => {
                tokens.push(Token::Text);
                i += 1;
            }
            'E' | 'e' if matches!(chars.get(i + 1), Some('+' | '-')) => {
                tokens.push(Token::Exponent(chars[i + 1] == '+'));
                i += 2;
            }
            'A' | 'a'
                if chars[i..]
                    .iter()
                    .take(5)
                    .collect::<String>()
                    .eq_ignore_ascii_case("AM/PM") =>
            {
                tokens.push(Token::Time("AM/PM".into()));
                i += 5;
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' => {
                let lower = c.to_ascii_lowercase();
                let mut j = i;
                while j < chars.len() && chars[j].to_ascii_lowercase() == lower {
                    j += 1;
                }
                tokens.push(Token::Time(
                    chars[i..j].iter().collect::<String>().to_ascii_lowercase(),
                ));
                i = j;
            }
            _ => {
                push_literal(&mut tokens, &c.to_string());
                i += 1;
            }
        }
    }
    tokens
}

fn is_general(section: &str) -> bool {
    section.trim().eq_ignore_ascii_case("general") || section.trim().is_empty()
}

/// Mã định dạng có phần ngày (năm, ngày, hoặc tháng) hay không. Không có thì là định dạng chỉ giờ.
pub fn has_date_part(code: &str) -> bool {
    let tokens = split_sections(code)
        .first()
        .map(|s| tokenize(s))
        .unwrap_or_default();
    let has_hour_or_second = tokens
        .iter()
        .any(|t| matches!(t, Token::Time(s) if s.contains('h') || s.contains('s')));
    tokens.iter().any(|t| match t {
        Token::Time(s) => {
            s.starts_with('y') || s.starts_with('d') || (s.starts_with('m') && !has_hour_or_second)
        }
        _ => false,
    })
}

/// Định dạng phần giờ của một số sê-ri ngày giờ của Excel theo mã định dạng chỉ có giờ.
pub fn format_time(serial: f64, code: &str) -> String {
    let section = split_sections(code).into_iter().next().unwrap_or_default();
    let tokens = tokenize(&section);
    let total_seconds = (serial * 86_400.0).round() as i64;
    let day_seconds = total_seconds.rem_euclid(86_400);
    let (hours, minutes, seconds) = (
        day_seconds / 3600,
        (day_seconds % 3600) / 60,
        day_seconds % 60,
    );
    let twelve_hour = tokens
        .iter()
        .any(|t| matches!(t, Token::Time(s) if s == "AM/PM"));
    let mut out = String::new();
    for token in &tokens {
        match token {
            Token::Time(s) if s == "AM/PM" => out.push_str(if hours < 12 { "AM" } else { "PM" }),
            Token::Time(s) if s.starts_with("[h") => {
                out.push_str(&(total_seconds / 3600).to_string())
            }
            Token::Time(s) if s.starts_with("[m") => {
                out.push_str(&(total_seconds / 60).to_string())
            }
            Token::Time(s) if s.starts_with("[s") => out.push_str(&total_seconds.to_string()),
            Token::Time(s) if s.starts_with('h') => {
                let h = if twelve_hour {
                    (hours + 11) % 12 + 1
                } else {
                    hours
                };
                out.push_str(&if s.len() >= 2 {
                    format!("{h:02}")
                } else {
                    h.to_string()
                });
            }
            Token::Time(s) if s.starts_with('m') => {
                out.push_str(&if s.len() >= 2 {
                    format!("{minutes:02}")
                } else {
                    minutes.to_string()
                });
            }
            Token::Time(s) if s.starts_with('s') => {
                out.push_str(&if s.len() >= 2 {
                    format!("{seconds:02}")
                } else {
                    seconds.to_string()
                });
            }
            Token::Literal(text) => out.push_str(text),
            Token::Point => out.push('.'),
            Token::Comma => out.push(','),
            _ => {}
        }
    }
    out
}

/// Áp mã định dạng số cho một giá trị số.
pub fn format_number(value: f64, code: &str, locale: NumberLocale) -> String {
    if !value.is_finite() {
        return format_general(value, locale);
    }
    let sections = split_sections(code);
    let (section, absolute) = match (sections.len(), value) {
        (n, v) if v < 0.0 && n >= 2 => (sections[1].as_str(), true),
        (n, v) if v == 0.0 && n >= 3 => (sections[2].as_str(), true),
        _ => (sections[0].as_str(), false),
    };
    if is_general(section) {
        let shown = if absolute { value.abs() } else { value };
        return format_general(shown, locale);
    }
    let tokens = tokenize(section);
    if !tokens.iter().any(|t| matches!(t, Token::Digit(_))) {
        // Phần chỉ có chữ cố định (ví dụ "-" cho số 0) hoặc mã không hiểu được.
        if tokens.iter().all(|t| matches!(t, Token::Literal(_))) && !tokens.is_empty() {
            return tokens
                .iter()
                .map(|t| {
                    if let Token::Literal(s) = t {
                        s.as_str()
                    } else {
                        ""
                    }
                })
                .collect();
        }
        return format_general(value, locale);
    }
    let shown = if absolute { value.abs() } else { value };
    let body = render_digits(shown.abs(), &tokens, locale);
    let negative_sign =
        !absolute && shown < 0.0 && body.chars().any(|c| c.is_ascii_digit() && c != '0');
    if negative_sign {
        format!("-{body}")
    } else {
        body
    }
}

fn render_digits(value: f64, tokens: &[Token], locale: NumberLocale) -> String {
    let first_digit = tokens
        .iter()
        .position(|t| matches!(t, Token::Digit(_)))
        .unwrap_or(0);
    let exponent_at = tokens.iter().position(|t| matches!(t, Token::Exponent(_)));
    let mantissa_end = exponent_at.unwrap_or(tokens.len());
    let last_digit = tokens[..mantissa_end]
        .iter()
        .rposition(|t| matches!(t, Token::Digit(_)))
        .unwrap_or(first_digit);
    let point_at = tokens[first_digit..=last_digit]
        .iter()
        .position(|t| *t == Token::Point)
        .map(|p| p + first_digit);

    let int_tokens = &tokens[first_digit..point_at.unwrap_or(last_digit + 1)];
    let frac_tokens: &[Token] = match point_at {
        Some(p) => &tokens[p + 1..=last_digit],
        None => &[],
    };
    let int_digits: Vec<char> = int_tokens
        .iter()
        .filter_map(|t| {
            if let Token::Digit(c) = t {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    let frac_digits: Vec<char> = frac_tokens
        .iter()
        .filter_map(|t| {
            if let Token::Digit(c) = t {
                Some(*c)
            } else {
                None
            }
        })
        .collect();

    // Dấu phẩy sát sau chữ số cuối của phần nguyên (không có dấu chấm theo sau) là chia nghìn.
    let mut scale_commas = 0;
    let mut k = point_at.unwrap_or(last_digit + 1);
    while k < tokens.len() && tokens[k] == Token::Comma {
        scale_commas += 1;
        k += 1;
    }
    let grouping = int_tokens.contains(&Token::Comma);
    let percents = tokens.iter().filter(|t| **t == Token::Percent).count();

    let mut number = value * 100f64.powi(percents as i32) / 1000f64.powi(scale_commas);
    let mut exponent_value = 0i32;
    if exponent_at.is_some() && number != 0.0 {
        let int_places = int_digits.len().max(1) as i32;
        exponent_value = number.log10().floor() as i32 - (int_places - 1);
        number /= 10f64.powi(exponent_value);
    }

    let decimals = frac_digits.len();
    let number = round_half_away(number, decimals);
    let rounded = format!("{number:.decimals$}");
    let (int_text, frac_text) = rounded.split_once('.').unwrap_or((&rounded, ""));
    // Sau khi làm tròn, dạng khoa học có thể tràn sang 10.0: chỉnh lại.
    let (int_text, frac_text, exponent_value) =
        if exponent_at.is_some() && int_text.len() > int_digits.len().max(1) {
            let renumber = number / 10.0;
            let again = format!("{renumber:.decimals$}");
            let (i, f) = again
                .split_once('.')
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .unwrap_or((again.clone(), String::new()));
            (i, f, exponent_value + 1)
        } else {
            (int_text.to_string(), frac_text.to_string(), exponent_value)
        };

    // Phần nguyên.
    let min_int = int_digits.iter().filter(|c| **c == '0').count();
    let mut int_text = if int_text == "0" && min_int == 0 {
        String::new()
    } else {
        int_text
    };
    while int_text.len() < min_int {
        int_text.insert(0, '0');
    }
    if grouping && int_text.len() > 3 {
        let bytes: Vec<char> = int_text.chars().collect();
        let mut grouped = String::new();
        for (index, ch) in bytes.iter().enumerate() {
            if index > 0 && (bytes.len() - index).is_multiple_of(3) {
                grouped.push(locale.group);
            }
            grouped.push(*ch);
        }
        int_text = grouped;
    }

    // Phần thập phân: bỏ số 0 cuối ở các vị trí `#`.
    let mut frac: Vec<char> = frac_text.chars().collect();
    while let (Some(last), Some(kind)) = (frac.last(), frac_digits.get(frac.len().wrapping_sub(1)))
    {
        if *last == '0' && *kind != '0' {
            frac.pop();
        } else {
            break;
        }
    }

    let mut out = String::new();
    for token in &tokens[..first_digit] {
        render_literal(token, &mut out);
    }
    out.push_str(&int_text);
    if point_at.is_some() && (!frac.is_empty() || frac_digits.contains(&'0')) {
        out.push(locale.decimal);
        out.extend(frac.iter());
    }
    if let Some(e) = exponent_at {
        let show_plus = matches!(tokens[e], Token::Exponent(true));
        let exp_digits = tokens[e + 1..]
            .iter()
            .take_while(|t| matches!(t, Token::Digit(_)))
            .count()
            .max(1);
        out.push('E');
        if exponent_value < 0 {
            out.push('-');
        } else if show_plus {
            out.push('+');
        }
        out.push_str(&format!(
            "{:0width$}",
            exponent_value.abs(),
            width = exp_digits
        ));
        for token in tokens[e + 1 + exp_digits.min(tokens.len() - e - 1)..].iter() {
            render_literal(token, &mut out);
        }
    } else {
        for token in &tokens[last_digit + 1..] {
            render_literal(token, &mut out);
        }
    }
    out
}

/// Làm tròn kiểu Excel: số .5 làm tròn ra xa số 0 (Rust `format!` làm tròn về số chẵn).
/// Bước nhân 1e9 rồi làm tròn trước để khử sai số nhị phân, ví dụ 1.005 × 100 = 100.49999…
fn round_half_away(value: f64, decimals: usize) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    let scaled = value * factor;
    if !scaled.is_finite() || scaled.abs() > 1e15 {
        return value;
    }
    let denoised = (scaled * 1e6).round() / 1e6;
    denoised.round() / factor
}

fn render_literal(token: &Token, out: &mut String) {
    match token {
        Token::Literal(text) => out.push_str(text),
        Token::Percent => out.push('%'),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: NumberLocale = NumberLocale::EN_US;
    const VI: NumberLocale = NumberLocale::VI_VN;

    /// Giá trị mong đợi lấy từ ClosedXML 0.105.1 trên `fixtures/table/tricky.xlsx` (spike S4,
    /// `docs/spikes/evidence/S4/closedxml.*.json`).
    #[test]
    fn matches_closedxml_on_tricky_fixture() {
        let cases: &[(f64, &str, &str, &str)] = &[
            (1.0, "0000", "0001", "0001"),
            (123.0, "0000", "0123", "0123"),
            (1_234_567.0, "#,##0", "1,234,567", "1.234.567"),
            (999.0, "#,##0", "999", "999"),
            (1_234_567.5, "#,##0.00", "1,234,567.50", "1.234.567,50"),
            (0.5, "#,##0.00", "0.50", "0,50"),
            (1000.0, "#,##0.00", "1,000.00", "1.000,00"),
            (0.125, "0.00%", "12.50%", "12,50%"),
            (1.0, "0.00%", "100.00%", "100,00%"),
            (0.0333, "0.00%", "3.33%", "3,33%"),
            (
                2_500_000.0,
                "#,##0\\ \"VND\"",
                "2,500,000 VND",
                "2.500.000 VND",
            ),
            (0.0, "#,##0\\ \"VND\"", "0 VND", "0 VND"),
            (-1500.0, "#,##0;\\(#,##0\\)", "(1,500)", "(1.500)"),
            (1500.0, "#,##0;\\(#,##0\\)", "1,500", "1.500"),
            (0.0, "#,##0;\\(#,##0\\)", "0", "0"),
            (15_000_000.0, "General", "15000000", "15000000"),
            (3.5, "General", "3.5", "3,5"),
            (-7.0, "General", "-7", "-7"),
            (0.1, "General", "0.1", "0,1"),
            (1234.5678, "General", "1234.5678", "1234,5678"),
            (1e-7, "General", "1E-07", "1E-07"),
            (123_456_789_012.0, "General", "123456789012", "123456789012"),
            (
                12_345_678_901_234_568.0,
                "General",
                "1.23456789012346E+16",
                "1,23456789012346E+16",
            ),
            (0.000_001_234, "General", "1.234E-06", "1,234E-06"),
        ];
        for (value, code, en, vi) in cases {
            assert_eq!(format_number(*value, code, EN), *en, "{value} {code} en-US");
            assert_eq!(format_number(*value, code, VI), *vi, "{value} {code} vi-VN");
        }
    }

    #[test]
    fn general_switches_to_scientific_like_dotnet_g15() {
        assert_eq!(format_general(0.0001, EN), "0.0001");
        assert_eq!(format_general(0.00001, EN), "1E-05");
        assert_eq!(format_general(1e15, EN), "1E+15");
        assert_eq!(format_general(999_999_999_999_999.0, EN), "999999999999999");
        assert_eq!(format_general(-0.25, EN), "-0.25");
    }

    #[test]
    fn other_common_formats() {
        assert_eq!(format_number(1234.0, "#,##0,\"K\"", EN), "1K");
        assert_eq!(format_number(0.5, "#.00", EN), ".50");
        assert_eq!(format_number(12345.0, "0.00E+00", EN), "1.23E+04");
        assert_eq!(format_number(-3.0, "0.0", EN), "-3.0");
        assert_eq!(format_number(1234.5, "[$₫-42A] #,##0", EN), "₫ 1,235");
        assert_eq!(format_number(7.0, "[Red]0", EN), "7");
        assert_eq!(format_number(0.0, "#,##0;-#,##0;\"-\"", EN), "-");
        assert_eq!(format_number(1.005, "0.00", EN), "1.01");
        assert_eq!(format_number(2.5, "0", EN), "3");
        assert_eq!(format_number(-2.5, "0", EN), "-3");
    }

    #[test]
    fn time_only_formats() {
        assert!(!has_date_part("hh:mm"));
        assert!(!has_date_part("h:mm AM/PM"));
        assert!(has_date_part("dd/mm/yyyy"));
        assert!(has_date_part("m/d/yyyy"));
        assert!(has_date_part("mmm-yy"));
        assert!(has_date_part("dd/mm/yyyy\\ hh:mm"));
        assert_eq!(format_time(0.604_166_666_666_667, "hh:mm"), "14:30");
        assert_eq!(format_time(0.25, "hh:mm"), "06:00");
        assert_eq!(format_time(0.999, "hh:mm"), "23:58");
        assert_eq!(format_time(0.604_166_666_666_667, "h:mm AM/PM"), "2:30 PM");
        assert_eq!(format_time(1.5, "[h]:mm"), "36:00");
    }
}
