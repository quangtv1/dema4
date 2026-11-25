//! Tìm các chữ ký trong một PDF (kế hoạch mục 9, phần PDF): field `/FT /Sig` trong `/AcroForm` (kể cả qua
//! `/Kids`), giá trị `/V` có `/ByteRange` và `/Contents`. Phần mật mã kiểm ở `smks-crypto::verify`.

use lopdf::{Dictionary, Object, ObjectId};
use serde::Serialize;
use smks_core::{ErrorCode, Result, SmksError};

use crate::geometry::{Placement, from_user_space, to_placement};
use crate::reader::{PdfFile, Rect};

/// Một chữ ký đọc được từ file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureField {
    pub field_name: String,
    pub byte_range: [usize; 4],
    /// DER của CMS (đã bỏ phần đệm 0 nếu có, xem `smks-crypto::verify::trim_der`).
    #[serde(skip)]
    pub contents: Vec<u8>,
    pub sub_filter: String,
    pub name: Option<String>,
    pub reason: Option<String>,
    pub location: Option<String>,
    /// Chuỗi `/M` gốc.
    pub signing_time: Option<String>,
    /// Trang có widget (từ 1), nếu tìm được.
    pub page: Option<u32>,
    /// Khối chữ ký có kích thước khác 0 (chữ ký hiển thị).
    pub visible: bool,
    /// `/ByteRange` hợp lệ: bắt đầu từ 0, chỗ trống đúng là `/Contents`, nằm trong file.
    pub range_ok: bool,
    /// `/ByteRange` phủ tới cuối file (không có gì được thêm sau chữ ký này).
    pub covers_whole_file: bool,
    /// Dấu thời gian tài liệu (`/Type /DocTimeStamp`, `/SubFilter /ETSI.RFC3161`): token RFC 3161 phủ cả tài
    /// liệu, không có người ký.
    pub doc_timestamp: bool,
    /// Chữ ký chứng nhận (`/Reference` … `/TransformMethod /DocMDP`): quyền `/P` 1, 2 hoặc 3.
    pub docmdp: Option<u8>,
    /// Khung chữ ký trên trang (tỉ lệ theo khung nhìn, gốc trên trái), nếu là chữ ký hiển thị.
    pub placement: Option<Placement>,
}

impl SignatureField {
    /// Hai đoạn byte được ký.
    pub fn signed_parts<'a>(&self, bytes: &'a [u8]) -> Option<[&'a [u8]; 2]> {
        let [a, b, c, d] = self.byte_range;
        if a != 0 || c < b || c.checked_add(d)? > bytes.len() {
            return None;
        }
        Some([&bytes[..b], &bytes[c..c + d]])
    }
}

/// Giải mã chuỗi văn bản PDF (UTF-16BE có BOM, UTF-8 có BOM, hoặc PDFDocEncoding ~ Latin-1).
pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    bytes.iter().map(|b| char::from(*b)).collect()
}

fn text_of(file: &PdfFile, dict: &Dictionary, key: &[u8]) -> Option<String> {
    match file.resolve(dict.get(key).ok()?) {
        Object::String(bytes, _) => Some(decode_text(bytes)),
        _ => None,
    }
}

/// Mọi chữ ký trong file, theo thứ tự `/ByteRange` (chữ ký cũ trước).
pub fn scan(bytes: &[u8]) -> Result<Vec<SignatureField>> {
    let file = PdfFile::open(bytes)?;
    let pages = file.pages().unwrap_or_default();
    let page_number = |id: ObjectId| pages.iter().find(|p| p.id == id).map(|p| p.number);
    let mut out = Vec::new();
    let Ok(catalog) = file.catalog() else {
        return Ok(out);
    };
    let Some(Object::Dictionary(form)) = catalog.get(b"AcroForm").ok().map(|o| file.resolve(o))
    else {
        return Ok(out);
    };
    let Some(Object::Array(fields)) = form.get(b"Fields").ok().map(|o| file.resolve(o)) else {
        return Ok(out);
    };
    let mut stack: Vec<(Object, String, bool)> = fields
        .iter()
        .rev()
        .map(|f| (f.clone(), String::new(), false))
        .collect();
    let mut guard = 0;
    while let Some((node, parent_name, parent_is_sig)) = stack.pop() {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        let Object::Dictionary(dict) = file.resolve(&node) else {
            continue;
        };
        let own = text_of(&file, dict, b"T").unwrap_or_default();
        let full_name = match (parent_name.is_empty(), own.is_empty()) {
            (true, _) => own.clone(),
            (false, true) => parent_name.clone(),
            (false, false) => format!("{parent_name}.{own}"),
        };
        let is_sig = dict
            .get(b"FT")
            .and_then(Object::as_name)
            .map_or(parent_is_sig, |n| n == b"Sig");
        if is_sig
            && let Some(Object::Dictionary(value)) = dict.get(b"V").ok().map(|o| file.resolve(o))
            && let Some(sig) = read_signature(&file, bytes.len(), value, &full_name)
        {
            let own_id = node.as_reference().ok();
            let (page, rect) = widget_info(&file, dict, own_id, &page_number, &pages);
            let placement = page.zip(rect).and_then(|(n, r)| {
                let info = pages.iter().find(|p| p.number == n)?;
                let display = info.display_box();
                Some(to_placement(&display, n, &from_user_space(info, &r)))
            });
            out.push(SignatureField {
                page,
                visible: rect.is_some(),
                placement,
                ..sig
            });
        }
        if let Some(Object::Array(kids)) = dict.get(b"Kids").ok().map(|o| file.resolve(o)) {
            // Widget con của field chữ ký không có /T: chỉ duyệt khi có thể là field con.
            if !(is_sig && dict.has(b"V")) {
                for kid in kids.iter().rev() {
                    stack.push((kid.clone(), full_name.clone(), is_sig));
                }
            }
        }
    }
    out.sort_by_key(|s| s.byte_range[1]);
    Ok(out)
}

/// Trang (từ 1) và khung (toạ độ trang) của widget đầu tiên có kích thước khác 0. Trang lấy theo `/P`; không
/// có `/P` thì tìm trang có widget trong `/Annots`.
fn widget_info(
    file: &PdfFile,
    field: &Dictionary,
    own_id: Option<ObjectId>,
    page_number: &dyn Fn(ObjectId) -> Option<u32>,
    pages: &[crate::reader::PageInfo],
) -> (Option<u32>, Option<Rect>) {
    let widgets: Vec<(&Dictionary, Option<ObjectId>)> =
        match field.get(b"Kids").ok().map(|o| file.resolve(o)) {
            Some(Object::Array(kids)) => kids
                .iter()
                .filter_map(|k| Some((file.resolve(k).as_dict().ok()?, k.as_reference().ok())))
                .collect(),
            _ => vec![(field, own_id)],
        };
    let on_page = |id: ObjectId| -> Option<u32> {
        pages.iter().find_map(|p| {
            let page = file.doc.get_dictionary(p.id).ok()?;
            let Object::Array(annots) = file.resolve(page.get(b"Annots").ok()?) else {
                return None;
            };
            annots
                .iter()
                .any(|a| a.as_reference().ok() == Some(id))
                .then_some(p.number)
        })
    };
    let mut first_page = None;
    for (w, id) in widgets {
        let page = w
            .get(b"P")
            .ok()
            .and_then(|p| p.as_reference().ok())
            .and_then(page_number)
            .or_else(|| id.and_then(on_page));
        first_page = first_page.or(page);
        if let Some(Object::Array(rect)) = w.get(b"Rect").ok().map(|o| file.resolve(o)) {
            let n: Vec<f64> = rect
                .iter()
                .filter_map(|v| match file.resolve(v) {
                    Object::Integer(i) => Some(*i as f64),
                    Object::Real(r) => Some(f64::from(*r)),
                    _ => None,
                })
                .collect();
            if let [a, b, c, d] = n[..]
                && (c - a).abs() > 0.5
                && (d - b).abs() > 0.5
            {
                let r = Rect::new(a.min(c), b.min(d), (c - a).abs(), (d - b).abs());
                return (page.or(first_page), Some(r));
            }
        }
    }
    (first_page, None)
}

/// Quyền `/P` của chữ ký chứng nhận (`/Reference` có `/TransformMethod /DocMDP`); mặc định 2.
fn docmdp_of(file: &PdfFile, value: &Dictionary) -> Option<u8> {
    let Object::Array(refs) = file.resolve(value.get(b"Reference").ok()?) else {
        return None;
    };
    refs.iter().find_map(|r| {
        let Object::Dictionary(entry) = file.resolve(r) else {
            return None;
        };
        if entry
            .get(b"TransformMethod")
            .and_then(Object::as_name)
            .ok()?
            != b"DocMDP"
        {
            return None;
        }
        let p = entry
            .get(b"TransformParams")
            .ok()
            .map(|o| file.resolve(o))
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"P").ok())
            .map(|o| file.resolve(o))
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(2);
        Some(u8::try_from(p.clamp(1, 3)).unwrap_or(2))
    })
}

fn read_signature(
    file: &PdfFile,
    len: usize,
    value: &Dictionary,
    field_name: &str,
) -> Option<SignatureField> {
    let Object::Array(range) = file.resolve(value.get(b"ByteRange").ok()?) else {
        return None;
    };
    let numbers: Vec<usize> = range
        .iter()
        .filter_map(|o| match file.resolve(o) {
            Object::Integer(i) => usize::try_from(*i).ok(),
            _ => None,
        })
        .collect();
    let [a, b, c, d] = numbers[..] else {
        return None;
    };
    let Object::String(contents, _) = file.resolve(value.get(b"Contents").ok()?) else {
        return None;
    };
    let sub_filter = value
        .get(b"SubFilter")
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_default();
    let end = c.saturating_add(d);
    // Chỗ trống giữa hai đoạn phải đúng bằng `<hex>` của /Contents.
    let range_ok = a == 0 && c > b && end <= len && c - b == contents.len() * 2 + 2;
    let doc_timestamp = sub_filter == "ETSI.RFC3161"
        || value
            .get(b"Type")
            .and_then(Object::as_name)
            .is_ok_and(|n| n == b"DocTimeStamp");
    Some(SignatureField {
        field_name: field_name.to_owned(),
        byte_range: [a, b, c, d],
        contents: contents.clone(),
        sub_filter,
        name: text_of(file, value, b"Name"),
        reason: text_of(file, value, b"Reason"),
        location: text_of(file, value, b"Location"),
        signing_time: text_of(file, value, b"M"),
        page: None,
        visible: false,
        range_ok,
        covers_whole_file: end == len,
        doc_timestamp,
        docmdp: docmdp_of(file, value),
        placement: None,
    })
}

/// Số chữ ký trong file (0 nếu file không đọc được), để hiện nhãn "Đã có N chữ ký".
pub fn count(bytes: &[u8]) -> usize {
    scan(bytes).map(|s| s.len()).unwrap_or(0)
}

/// Lỗi khi file không có chữ ký nào.
pub fn none_found() -> SmksError {
    SmksError::new(
        ErrorCode::SignatureInvalid,
        "File PDF không có chữ ký số nào.",
    )
}
