//! Ghi object PDF dạng văn bản cho phần nối thêm (port `PdfObjectWriter.cs`, viết lại trên kiểu của `lopdf`).

use lopdf::{Object, StringFormat};

/// Số thực gọn: tối đa 4 chữ số thập phân, bỏ số 0 thừa, không dùng dạng mũ.
pub fn real(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" || text.is_empty() {
        "0".into()
    } else {
        text.to_owned()
    }
}

/// Chuỗi văn bản PDF: ASCII in được ghi nguyên, còn lại UTF-16BE có BOM (như bản cũ).
pub fn text(value: &str) -> Object {
    if value.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        Object::String(value.as_bytes().to_vec(), StringFormat::Literal)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in value.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, StringFormat::Hexadecimal)
    }
}

pub fn hex_upper(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0xF)]));
    }
    out
}

fn name(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'/');
    for byte in bytes {
        if (0x21..0x7F).contains(byte) && !b"()<>[]{}/%#".contains(byte) {
            out.push(*byte);
        } else {
            out.extend_from_slice(format!("#{byte:02X}").as_bytes());
        }
    }
}

/// Ghi `object` theo cú pháp PDF. Stream ghi cả từ điển lẫn dữ liệu (độ dài lấy từ dữ liệu thật).
pub fn write(out: &mut Vec<u8>, object: &Object) {
    match object {
        Object::Null => out.extend_from_slice(b"null"),
        Object::Boolean(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
        Object::Integer(value) => out.extend_from_slice(value.to_string().as_bytes()),
        Object::Real(value) => out.extend_from_slice(real(f64::from(*value)).as_bytes()),
        Object::Name(bytes) => name(out, bytes),
        Object::String(bytes, _) => {
            out.push(b'<');
            out.extend_from_slice(hex_upper(bytes).as_bytes());
            out.push(b'>');
        }
        Object::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                write(out, item);
            }
            out.push(b']');
        }
        Object::Dictionary(dict) => {
            out.extend_from_slice(b"<<");
            for (key, value) in dict.iter() {
                name(out, key);
                out.push(b' ');
                write(out, value);
            }
            out.extend_from_slice(b">>");
        }
        Object::Reference(id) => out.extend_from_slice(format!("{} {} R", id.0, id.1).as_bytes()),
        Object::Stream(stream) => {
            let mut dict = stream.dict.clone();
            dict.set("Length", stream.content.len() as i64);
            write(out, &Object::Dictionary(dict));
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(&stream.content);
            out.extend_from_slice(b"\nendstream");
        }
    }
}

/// `D:yyyyMMddHHmmss+07'00'` theo giờ Việt Nam từ giây Unix.
pub fn pdf_date_vietnam(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = vietnam_parts(unix);
    format!("D:{y:04}{mo:02}{d:02}{h:02}{mi:02}{s:02}+07'00'")
}

/// Năm, tháng, ngày, giờ, phút, giây theo giờ Việt Nam.
pub fn vietnam_parts(unix: i64) -> (i64, i64, i64, i64, i64, i64) {
    let secs = unix + i64::from(smks_core::consts::app::VIETNAM_UTC_OFFSET_SECONDS);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Dictionary;

    #[test]
    fn writes_objects() {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name(b"Sig".to_vec()));
        dict.set("Name", text("Nguyễn A"));
        dict.set("Rect", vec![Object::Real(1.5), Object::Integer(2)]);
        dict.set("N a", Object::Reference((5, 1)));
        let mut out = Vec::new();
        write(&mut out, &Object::Dictionary(dict));
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "<</Type /Sig/Name <FEFF004E0067007500791EC5006E00200041>/Rect [1.5 2]/N#20a 5 1 R>>"
        );
        assert_eq!(real(12.0), "12");
        assert_eq!(real(-0.00001), "0");
        assert_eq!(real(595.276), "595.276");
    }

    #[test]
    fn vietnam_date() {
        // 2026-11-19 03:00:00 UTC.
        assert_eq!(pdf_date_vietnam(1_795_057_200), "D:20261119100000+07'00'");
    }
}
