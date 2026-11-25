//! Tìm và điền biến `{ten_bien}` trong mẫu Word (port `WordTemplateEngine.cs` của bản .NET 1.4.0).
//!
//! Xử lý `word/document.xml`, mọi `header*.xml`, `footer*.xml`, và thêm `footnotes.xml`, `endnotes.xml` (bản cũ
//! chưa xử lý hai phần này). Trong mỗi đoạn `w:p` (kể cả đoạn nằm trong text box), các `w:t` thuộc trực tiếp
//! đoạn đó được nối lại để tìm biến, nên biến bị Word tách qua nhiều run vẫn tìm được. Giá trị được đặt vào
//! `w:t` đầu tiên mà biến chạm tới, phần còn lại của biến bị xoá, `w:t` bị sửa được thêm
//! `xml:space="preserve"`. Biến không có trong bảng giá trị thì giữ nguyên.
//!
//! XML được sửa tại chỗ: chỉ nội dung các `w:t` bị đổi, mọi byte khác của mẫu giữ nguyên.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use regex::Regex;
use smks_core::consts::merge::FIELD_PATTERN;
use smks_core::{ErrorCode, Result, SmksError};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn invalid(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::MergeTemplateInvalid, message)
}

fn field_regex() -> Result<Regex> {
    Regex::new(FIELD_PATTERN).map_err(|e| SmksError::new(ErrorCode::InternalError, e.to_string()))
}

/// Phần XML có thể chứa biến.
fn is_text_part(name: &str) -> bool {
    let Some(file) = name.strip_prefix("word/") else {
        return false;
    };
    if file.contains('/') || !file.ends_with(".xml") {
        return false;
    }
    file == "document.xml"
        || file == "footnotes.xml"
        || file == "endnotes.xml"
        || file.starts_with("header")
        || file.starts_with("footer")
}

/// Tên các biến trong mẫu, theo thứ tự xuất hiện, không trùng.
pub fn read_fields(template: &[u8]) -> Result<Vec<String>> {
    let mut archive = open(template)?;
    let regex = field_regex()?;
    let mut fields = Vec::new();
    for name in part_names(&mut archive) {
        let xml = read_part(&mut archive, &name)?;
        for paragraph in scan(&xml).paragraphs {
            let joined: String = paragraph.iter().map(|n| n.text.as_str()).collect();
            for capture in regex.captures_iter(&joined) {
                let name = capture[1].trim().to_owned();
                if !name.is_empty() && !fields.contains(&name) {
                    fields.push(name);
                }
            }
        }
    }
    Ok(fields)
}

/// Điền giá trị vào mẫu, trả về file DOCX mới.
pub fn fill(template: &[u8], values: &HashMap<String, String>) -> Result<Vec<u8>> {
    transform(template, values, Mark::None)
}

/// Như [`fill`] nhưng tô nền giá trị đã điền (vàng) và biến không có giá trị (đỏ). Chỉ dùng cho bản xem
/// trước: run chứa biến được tách thành nhiều run cùng định dạng, run giữa có thêm `w:shd`.
pub fn fill_marked(template: &[u8], values: &HashMap<String, String>) -> Result<Vec<u8>> {
    transform(template, values, Mark::Values)
}

/// Tô nền mọi biến `{ten_bien}` của mẫu (vàng), không điền gì. Chỉ dùng cho bản xem trước mẫu.
pub fn mark_fields(template: &[u8]) -> Result<Vec<u8>> {
    transform(template, &HashMap::new(), Mark::Template)
}

fn transform(template: &[u8], values: &HashMap<String, String>, mark: Mark) -> Result<Vec<u8>> {
    let mut archive = open(template)?;
    let regex = field_regex()?;
    let targets = part_names(&mut archive);

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let name = archive
            .by_index_raw(index)
            .and_then(|entry| entry.name().map(|n| n.into_owned()))
            .map_err(|e| invalid(format!("File mẫu Word bị hỏng: {e}")))?;
        if targets.contains(&name) {
            let xml = read_part(&mut archive, &name)?;
            let filled = fill_xml(&xml, values, &regex, mark);
            writer
                .start_file(name.as_str(), options)
                .map_err(write_error)?;
            writer.write_all(filled.as_bytes()).map_err(write_error)?;
        } else {
            let entry = archive
                .by_index_raw(index)
                .map_err(|e| invalid(format!("File mẫu Word bị hỏng: {e}")))?;
            writer.raw_copy_file(entry).map_err(write_error)?;
        }
    }
    Ok(writer.finish().map_err(write_error)?.into_inner())
}

fn write_error(error: impl std::fmt::Display) -> SmksError {
    SmksError::new(
        ErrorCode::MergeFailed,
        format!("Không ghi được file văn bản: {error}"),
    )
}

fn open(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("File mẫu không phải văn bản Word (.docx) hợp lệ."))?;
    if archive.by_name("word/document.xml").is_err() {
        return Err(invalid(
            "File mẫu Word không có nội dung chính (word/document.xml).",
        ));
    }
    Ok(archive)
}

fn part_names(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Vec<String> {
    let mut names: Vec<String> = archive
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.into_owned()))
        .filter(|n| is_text_part(n))
        .collect();
    // document.xml trước, rồi header, footer, footnotes, endnotes để thứ tự biến ổn định.
    names.sort_by_key(|n| {
        let rank = if n.ends_with("document.xml") {
            0
        } else if n.contains("header") {
            1
        } else if n.contains("footer") {
            2
        } else {
            3
        };
        (rank, n.clone())
    });
    names
}

fn read_part(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| invalid(format!("Không đọc được {name}: {e}")))?;
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|e| invalid(format!("Không đọc được {name}: {e}")))?;
    Ok(xml)
}

// ----------------------------------------------------------------------------------------------
// Quét XML
// ----------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TextNode {
    /// Vị trí `<` của thẻ mở `w:t`.
    tag_start: usize,
    /// Vị trí ngay sau `>` của thẻ mở.
    content_start: usize,
    /// Vị trí `<` của `</w:t>`.
    content_end: usize,
    /// Nội dung đã giải mã thực thể XML.
    text: String,
    /// Run `w:r` chứa `w:t` này (để tách run khi tô màu).
    run: Option<RunSpan>,
}

/// Vị trí thẻ mở của một run và `w:rPr` trực tiếp của nó (nếu có) trong XML.
#[derive(Debug, Clone, Copy)]
struct RunSpan {
    open: (usize, usize),
    rpr: Option<(usize, usize)>,
}

/// Vị trí ngay sau phần tử bắt đầu ở `start` (thẻ mở tới thẻ đóng tương ứng, kể cả phần tử cùng tên lồng nhau).
fn element_end(xml: &str, start: usize) -> usize {
    let open_end = tag_end(xml, start);
    if xml[..open_end].ends_with("/>") {
        return open_end;
    }
    let name = tag_name(&xml[start + 1..]);
    let mut depth = 1usize;
    let mut pos = open_end;
    while let Some(found) = xml[pos..].find('<') {
        let at = pos + found;
        let end = tag_end(xml, at);
        if let Some(closing) = xml[at..].strip_prefix("</") {
            if tag_name(closing) == name {
                depth -= 1;
                if depth == 0 {
                    return end;
                }
            }
        } else if tag_name(&xml[at + 1..]) == name && !xml[..end].ends_with("/>") {
            depth += 1;
        }
        pos = end;
    }
    xml.len()
}

/// `w:rPr` là con đầu tiên của run có thẻ mở kết thúc ở `open_end`.
fn run_properties(xml: &str, open_end: usize) -> Option<(usize, usize)> {
    let skipped = xml[open_end..].len() - xml[open_end..].trim_start().len();
    let start = open_end + skipped;
    let rest = xml.get(start..)?;
    (rest.starts_with('<') && tag_name(&rest[1..]) == "w:rPr")
        .then(|| (start, element_end(xml, start)))
}

#[derive(Debug, Default)]
struct Scan {
    /// Mỗi đoạn là danh sách `w:t` thuộc trực tiếp đoạn đó, theo thứ tự.
    paragraphs: Vec<Vec<TextNode>>,
}

/// Tên thẻ ngay sau `<` hoặc `</`.
fn tag_name(rest: &str) -> &str {
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Vị trí ngay sau `>` kết thúc thẻ bắt đầu ở `start`, bỏ qua `>` nằm trong giá trị thuộc tính.
fn tag_end(xml: &str, start: usize) -> usize {
    let mut quote: Option<u8> = None;
    for (offset, byte) in xml.as_bytes()[start..].iter().enumerate() {
        match (quote, byte) {
            (Some(q), b) if *b == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(*byte),
            (None, b'>') => return start + offset + 1,
            _ => {}
        }
    }
    xml.len()
}

fn scan(xml: &str) -> Scan {
    let mut result = Scan::default();
    // Ngăn xếp chỉ số đoạn đang mở, và run đang mở.
    let mut stack: Vec<usize> = Vec::new();
    let mut runs: Vec<RunSpan> = Vec::new();
    let mut pos = 0;
    while let Some(found) = xml[pos..].find('<') {
        let start = pos + found;
        let rest = &xml[start..];
        if rest.starts_with("<?") {
            pos = xml[start..].find("?>").map_or(xml.len(), |e| start + e + 2);
            continue;
        }
        if rest.starts_with("<!--") {
            pos = xml[start..]
                .find("-->")
                .map_or(xml.len(), |e| start + e + 3);
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            pos = xml[start..]
                .find("]]>")
                .map_or(xml.len(), |e| start + e + 3);
            continue;
        }
        let end = tag_end(xml, start);
        if let Some(closing) = rest.strip_prefix("</") {
            match tag_name(closing) {
                "w:p" => {
                    stack.pop();
                }
                "w:r" => {
                    runs.pop();
                }
                _ => {}
            }
            pos = end;
            continue;
        }
        let name = tag_name(&rest[1..]);
        let self_closing = xml[..end].ends_with("/>");
        match name {
            "w:p" if !self_closing => {
                result.paragraphs.push(Vec::new());
                stack.push(result.paragraphs.len() - 1);
                pos = end;
            }
            "w:r" if !self_closing => {
                runs.push(RunSpan {
                    open: (start, end),
                    rpr: run_properties(xml, end),
                });
                pos = end;
            }
            "w:t" if !self_closing => {
                let content_end = xml[end..].find("</w:t>").map_or(xml.len(), |e| end + e);
                if let Some(paragraph) = stack.last() {
                    result.paragraphs[*paragraph].push(TextNode {
                        tag_start: start,
                        content_start: end,
                        content_end,
                        text: unescape(&xml[end..content_end]),
                        run: runs.last().copied(),
                    });
                }
                pos = content_end;
            }
            _ => pos = end,
        }
    }
    result
}

/// Cách tô màu khi điền.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// Điền bình thường (văn bản thật).
    None,
    /// Tô mọi biến của mẫu, không điền.
    Template,
    /// Điền và tô giá trị; biến không có giá trị được giữ và tô đỏ.
    Values,
}

// Ký tự vùng dùng riêng đánh dấu đoạn cần tô trong chuỗi trung gian (bị bỏ khỏi chữ của mẫu trước khi dùng).
const MARK_ON: char = '\u{E000}';
const MARK_OFF: char = '\u{E001}';
const MISS_ON: char = '\u{E002}';
const MISS_OFF: char = '\u{E003}';
/// Màu nền: biến/giá trị (vàng nhạt), biến chưa có giá trị (đỏ nhạt).
const FILL_FIELD: &str = "FFE58F";
const FILL_MISSING: &str = "FFC9C9";

fn is_marker(c: char) -> bool {
    matches!(c, MARK_ON | MARK_OFF | MISS_ON | MISS_OFF)
}

fn fill_xml(xml: &str, values: &HashMap<String, String>, regex: &Regex, mark: Mark) -> String {
    let scanned = scan(xml);
    // Nội dung mới theo vị trí `tag_start` của các `w:t` bị sửa.
    let mut replaced: Vec<(TextNode, String)> = Vec::new();
    for paragraph in scanned.paragraphs {
        let mut texts: Vec<String> = paragraph
            .iter()
            .map(|n| {
                if mark == Mark::None {
                    n.text.clone()
                } else {
                    n.text.replace(is_marker, "")
                }
            })
            .collect();
        let mut touched = vec![false; texts.len()];
        let joined: String = texts.concat();
        let matches: Vec<(usize, usize, String)> = regex
            .captures_iter(&joined)
            .filter_map(|c| {
                let whole = c.get(0)?;
                Some((whole.start(), whole.end(), c[1].trim().to_owned()))
            })
            .collect();
        for (start, end, name) in matches.into_iter().rev() {
            let original = &joined[start..end];
            let value = match (mark, values.get(&name)) {
                (Mark::None, Some(value)) => value.clone(),
                (Mark::None, None) => continue,
                (Mark::Template, _) => format!("{MARK_ON}{original}{MARK_OFF}"),
                (Mark::Values, Some(value)) if value.is_empty() => String::new(),
                (Mark::Values, Some(value)) => format!("{MARK_ON}{value}{MARK_OFF}"),
                (Mark::Values, None) => format!("{MISS_ON}{original}{MISS_OFF}"),
            };
            let mut offset = 0;
            let mut first = true;
            for (index, text) in texts.iter_mut().enumerate() {
                let length = text.len();
                let from = start.saturating_sub(offset);
                let to = end.saturating_sub(offset).min(length);
                if from < to {
                    let replacement = if first { value.as_str() } else { "" };
                    *text = format!("{}{replacement}{}", &text[..from], &text[to..]);
                    touched[index] = true;
                    first = false;
                }
                offset += length;
            }
        }
        for ((node, text), changed) in paragraph.into_iter().zip(texts).zip(touched) {
            if changed {
                replaced.push((node, text));
            }
        }
    }
    if replaced.is_empty() {
        return xml.to_owned();
    }
    replaced.sort_by_key(|(node, _)| node.tag_start);

    let mut out = String::with_capacity(xml.len());
    let mut last = 0;
    for (node, text) in replaced {
        out.push_str(&xml[last..node.tag_start]);
        out.push_str(&preserve_space(&xml[node.tag_start..node.content_start]));
        match node.run {
            Some(run) if text.contains(is_marker) => push_marked(&mut out, xml, run, &text),
            _ => out.push_str(&escape(&text.replace(is_marker, ""))),
        }
        last = node.content_end;
    }
    out.push_str(&xml[last..]);
    out
}

/// Ghi nội dung `w:t` có đoạn đánh dấu: mỗi đoạn tô màu nằm trong một run riêng cùng định dạng với run gốc.
/// Thẻ mở `w:t` gốc đã được ghi trước; thẻ đóng `</w:t>` gốc được ghi sau bởi bên gọi.
fn push_marked(out: &mut String, xml: &str, run: RunSpan, text: &str) {
    let open = &xml[run.open.0..run.open.1];
    let rpr = run.rpr.map_or("", |(a, b)| &xml[a..b]);
    let mut segment = String::new();
    for c in text.chars() {
        let fill = match c {
            MARK_ON => Some(Some(FILL_FIELD)),
            MISS_ON => Some(Some(FILL_MISSING)),
            MARK_OFF | MISS_OFF => Some(None),
            _ => None,
        };
        let Some(fill) = fill else {
            segment.push(c);
            continue;
        };
        out.push_str(&escape(&segment));
        segment.clear();
        out.push_str("</w:t></w:r>");
        out.push_str(open);
        match fill {
            Some(color) => out.push_str(&shaded_rpr(rpr, color)),
            None => out.push_str(rpr),
        }
        out.push_str("<w:t xml:space=\"preserve\">");
    }
    out.push_str(&escape(&segment));
}

/// Phần tử con của `w:rPr` đứng sau `w:shd` theo thứ tự của lược đồ WordprocessingML.
const AFTER_SHD: &[&str] = &[
    "w:fitText",
    "w:vertAlign",
    "w:rtl",
    "w:cs",
    "w:em",
    "w:lang",
    "w:eastAsianLayout",
    "w:specVanish",
    "w:oMath",
    "w:rPrChange",
];

/// `w:rPr` của run gốc thêm nền `color`: bỏ `w:highlight`, `w:shd` cũ, chèn `w:shd` đúng vị trí theo lược đồ
/// (Word từ chối mở file có phần tử sai thứ tự).
fn shaded_rpr(rpr: &str, color: &str) -> String {
    let shd = format!(r#"<w:shd w:val="clear" w:color="auto" w:fill="{color}"/>"#);
    if rpr.is_empty() || rpr.ends_with("/>") {
        return format!("<w:rPr>{shd}</w:rPr>");
    }
    let open_end = tag_end(rpr, 0);
    let close_start = rpr.rfind("</w:rPr>").unwrap_or(rpr.len());
    let mut out = String::with_capacity(rpr.len() + shd.len());
    out.push_str(&rpr[..open_end]);
    let mut inserted = false;
    let mut pos = open_end;
    while pos < close_start {
        let Some(found) = rpr[pos..close_start].find('<') else {
            out.push_str(&rpr[pos..close_start]);
            break;
        };
        let at = pos + found;
        out.push_str(&rpr[pos..at]);
        let end = element_end(rpr, at).min(close_start);
        let name = tag_name(&rpr[at + 1..]);
        if !inserted && AFTER_SHD.contains(&name) {
            out.push_str(&shd);
            inserted = true;
        }
        if name != "w:highlight" && name != "w:shd" {
            out.push_str(&rpr[at..end]);
        }
        pos = end;
    }
    if !inserted {
        out.push_str(&shd);
    }
    out.push_str("</w:rPr>");
    out
}

/// Thêm `xml:space="preserve"` vào thẻ mở `w:t` nếu chưa có.
fn preserve_space(open_tag: &str) -> String {
    if open_tag.contains("xml:space") {
        return open_tag.to_owned();
    }
    let body = open_tag.trim_end_matches('>');
    format!("{body} xml:space=\"preserve\">")
}

fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        let Some(semi) = after.find(';') else {
            out.push_str(after);
            return out;
        };
        let entity = &after[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(&after[..=semi]),
        }
        rest = &after[semi + 1..];
    }
    out.push_str(rest);
    out
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // Ký tự điều khiển không hợp lệ trong XML 1.0 (trừ tab, xuống dòng) bị bỏ.
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Nội dung chữ của tất cả các đoạn trong phần chính, mỗi đoạn một dòng (dùng cho kiểm thử và tìm kiếm).
pub fn document_text(docx: &[u8]) -> Result<String> {
    let mut archive = open(docx)?;
    let xml = read_part(&mut archive, "word/document.xml")?;
    Ok(scan(&xml)
        .paragraphs
        .iter()
        .map(|p| p.iter().map(|n| n.text.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn fill_str(xml: &str, pairs: &[(&str, &str)]) -> String {
        fill_xml(xml, &values(pairs), &field_regex().unwrap(), Mark::None)
    }

    fn mark_str(xml: &str, pairs: &[(&str, &str)], mark: Mark) -> String {
        fill_xml(xml, &values(pairs), &field_regex().unwrap(), mark)
    }

    const SHD_Y: &str = r#"<w:shd w:val="clear" w:color="auto" w:fill="FFE58F"/>"#;
    const SHD_R: &str = r#"<w:shd w:val="clear" w:color="auto" w:fill="FFC9C9"/>"#;

    #[test]
    fn template_mark_splits_run_and_keeps_formatting() {
        let xml = r#"<w:p><w:r w:rsidR="1"><w:rPr><w:b/><w:highlight w:val="red"/><w:lang w:val="vi"/></w:rPr><w:t>Kính gửi {ho_ten},</w:t></w:r></w:p>"#;
        let rpr = r#"<w:rPr><w:b/><w:highlight w:val="red"/><w:lang w:val="vi"/></w:rPr>"#;
        let shaded = format!(r#"<w:rPr><w:b/>{SHD_Y}<w:lang w:val="vi"/></w:rPr>"#);
        let expected = format!(
            concat!(
                r#"<w:p><w:r w:rsidR="1">{rpr}<w:t xml:space="preserve">Kính gửi </w:t></w:r>"#,
                r#"<w:r w:rsidR="1">{shaded}<w:t xml:space="preserve">{{ho_ten}}</w:t></w:r>"#,
                r#"<w:r w:rsidR="1">{rpr}<w:t xml:space="preserve">,</w:t></w:r></w:p>"#
            ),
            rpr = rpr,
            shaded = shaded
        );
        assert_eq!(mark_str(xml, &[], Mark::Template), expected);
    }

    #[test]
    fn value_mark_colors_values_and_missing_fields() {
        let xml = r#"<w:p><w:r><w:t>{a} và {b}</w:t></w:r></w:p>"#;
        let out = mark_str(xml, &[("a", "A&B")], Mark::Values);
        assert_eq!(
            out,
            format!(
                concat!(
                    r#"<w:p><w:r><w:t xml:space="preserve"></w:t></w:r>"#,
                    r#"<w:r><w:rPr>{y}</w:rPr><w:t xml:space="preserve">A&amp;B</w:t></w:r>"#,
                    r#"<w:r><w:t xml:space="preserve"> và </w:t></w:r>"#,
                    r#"<w:r><w:rPr>{r}</w:rPr><w:t xml:space="preserve">{{b}}</w:t></w:r>"#,
                    r#"<w:r><w:t xml:space="preserve"></w:t></w:r></w:p>"#
                ),
                y = SHD_Y,
                r = SHD_R
            )
        );
        // Giá trị rỗng: không có gì để tô.
        assert_eq!(
            mark_str(xml, &[("a", ""), ("b", "x")], Mark::Values),
            format!(
                concat!(
                    r#"<w:p><w:r><w:t xml:space="preserve"> và </w:t></w:r>"#,
                    r#"<w:r><w:rPr>{y}</w:rPr><w:t xml:space="preserve">x</w:t></w:r>"#,
                    r#"<w:r><w:t xml:space="preserve"></w:t></w:r></w:p>"#
                ),
                y = SHD_Y
            )
        );
    }

    #[test]
    fn shading_is_placed_in_schema_order() {
        assert_eq!(
            shaded_rpr(
                r#"<w:rPr><w:rFonts w:ascii="X"/><w:sz w:val="26"/><w:u w:val="single"/><w:shd w:fill="000000"/><w:vertAlign w:val="superscript"/></w:rPr>"#,
                "FFE58F"
            ),
            format!(
                r#"<w:rPr><w:rFonts w:ascii="X"/><w:sz w:val="26"/><w:u w:val="single"/>{SHD_Y}<w:vertAlign w:val="superscript"/></w:rPr>"#
            )
        );
        assert_eq!(
            shaded_rpr("<w:rPr/>", "FFE58F"),
            format!("<w:rPr>{SHD_Y}</w:rPr>")
        );
        // rPrChange chứa rPr lồng nhau.
        assert_eq!(
            shaded_rpr(
                r#"<w:rPr><w:b/><w:rPrChange w:id="1"><w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr>"#,
                "FFE58F"
            ),
            format!(
                r#"<w:rPr><w:b/>{SHD_Y}<w:rPrChange w:id="1"><w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr>"#
            )
        );
    }

    #[test]
    fn marking_sample_templates_keeps_every_field_and_value() {
        for name in [
            "1_giay_moi_hop.docx",
            "2_quyet_dinh_khen_thuong.docx",
            "3_hop_dong_lao_dong.docx",
        ] {
            let template = fixture(name);
            let fields = read_fields(&template).unwrap();
            // Tô mẫu: vẫn đọc ra đủ biến (mỗi biến nằm trọn trong một run).
            assert_eq!(
                read_fields(&mark_fields(&template).unwrap()).unwrap(),
                fields,
                "{name}"
            );
            // Tô bản điền: biến có giá trị được điền, một biến bỏ trống thì còn lại đúng biến đó.
            let map: HashMap<String, String> = fields
                .iter()
                .skip(1)
                .map(|f| (f.clone(), format!("<{f}>")))
                .collect();
            let marked = fill_marked(&template, &map).unwrap();
            assert_eq!(
                read_fields(&marked).unwrap(),
                vec![fields[0].clone()],
                "{name}"
            );
            let plain = fill(&template, &map).unwrap();
            // Chữ của văn bản giống hệt bản điền thường; chỉ khác định dạng.
            assert_eq!(
                document_text(&marked).unwrap(),
                document_text(&plain).unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn replaces_field_inside_single_run_and_adds_preserve() {
        let xml = r#"<w:body><w:p><w:r><w:t>Kính gửi {ho_ten},</w:t></w:r></w:p></w:body>"#;
        assert_eq!(
            fill_str(xml, &[("ho_ten", "Nguyễn Văn A")]),
            r#"<w:body><w:p><w:r><w:t xml:space="preserve">Kính gửi Nguyễn Văn A,</w:t></w:r></w:p></w:body>"#
        );
    }

    #[test]
    fn replaces_field_split_across_runs() {
        let xml = concat!(
            r#"<w:p><w:r><w:t>Ông {</w:t></w:r><w:proofErr w:type="spellStart"/>"#,
            r#"<w:r><w:rPr><w:b/></w:rPr><w:t>ho_</w:t></w:r><w:r><w:t xml:space="preserve">ten} và {x}</w:t></w:r></w:p>"#
        );
        let out = fill_str(xml, &[("ho_ten", "Trần B"), ("x", "<1&2>")]);
        assert_eq!(
            out,
            concat!(
                r#"<w:p><w:r><w:t xml:space="preserve">Ông Trần B</w:t></w:r><w:proofErr w:type="spellStart"/>"#,
                r#"<w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve"></w:t></w:r><w:r><w:t xml:space="preserve"> và &lt;1&amp;2&gt;</w:t></w:r></w:p>"#
            )
        );
    }

    #[test]
    fn unknown_fields_are_kept_and_names_are_trimmed() {
        let xml = r#"<w:p><w:r><w:t>{ a } {b}</w:t></w:r></w:p>"#;
        assert_eq!(
            fill_str(xml, &[("a", "1")]),
            r#"<w:p><w:r><w:t xml:space="preserve">1 {b}</w:t></w:r></w:p>"#
        );
    }

    #[test]
    fn text_box_paragraph_is_separate_from_outer_paragraph() {
        // Đoạn trong text box nằm lồng trong một run của đoạn ngoài.
        let xml = concat!(
            r#"<w:p><w:r><w:t>{ngoai</w:t></w:r><w:r><w:pict><w:txbxContent>"#,
            r#"<w:p><w:r><w:t>{trong}</w:t></w:r></w:p></w:txbxContent></w:pict></w:r><w:r><w:t>}</w:t></w:r></w:p>"#
        );
        let out = fill_str(xml, &[("ngoai", "N"), ("trong", "T")]);
        assert!(
            out.contains(r#"<w:t xml:space="preserve">N</w:t>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<w:t xml:space="preserve">T</w:t>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<w:t xml:space="preserve"></w:t></w:r></w:p>"#),
            "{out}"
        );
    }

    #[test]
    fn entities_are_decoded_for_matching_and_other_tags_untouched() {
        let xml = r#"<?xml version="1.0"?><w:p><w:r><w:tab/><w:t>&#123;so&#125; &amp; </w:t><w:tbl/></w:r></w:p>"#;
        assert_eq!(
            fill_str(xml, &[("so", "01")]),
            r#"<?xml version="1.0"?><w:p><w:r><w:tab/><w:t xml:space="preserve">01 &amp; </w:t><w:tbl/></w:r></w:p>"#
        );
        assert_eq!(unescape("&lt;&#x41;&bogus;"), "<A&bogus;");
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/merge")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn reads_fields_of_sample_templates_including_footer() {
        let fields = read_fields(&fixture("3_hop_dong_lao_dong.docx")).unwrap();
        assert_eq!(fields.len(), 18);
        assert_eq!(fields[0], "so_hop_dong");
        assert!(fields.contains(&"ho_ten".to_owned()));
        assert_eq!(
            read_fields(&fixture("1_giay_moi_hop.docx")).unwrap().len(),
            11
        );
        assert_eq!(
            read_fields(&fixture("2_quyet_dinh_khen_thuong.docx"))
                .unwrap()
                .len(),
            10
        );
    }

    #[test]
    fn fills_sample_template_and_leaves_no_known_field() {
        let template = fixture("3_hop_dong_lao_dong.docx");
        let fields = read_fields(&template).unwrap();
        let map: HashMap<String, String> = fields
            .iter()
            .map(|f| (f.clone(), format!("<{f}>")))
            .collect();
        let filled = fill(&template, &map).unwrap();
        assert!(read_fields(&filled).unwrap().is_empty());
        let text = document_text(&filled).unwrap();
        assert!(text.contains("<ho_ten>"), "{text}");
        // Footer cũng được điền.
        let mut archive = ZipArchive::new(Cursor::new(filled.as_slice())).unwrap();
        let mut footer = String::new();
        archive
            .by_name("word/footer1.xml")
            .unwrap()
            .read_to_string(&mut footer)
            .unwrap();
        assert!(footer.contains("&lt;so_hop_dong&gt;"), "{footer}");
        // Các phần khác giữ nguyên số lượng.
        assert_eq!(
            archive.len(),
            ZipArchive::new(Cursor::new(template.as_slice()))
                .unwrap()
                .len()
        );
    }

    #[test]
    fn rejects_non_docx() {
        assert_eq!(
            read_fields(b"not a zip").unwrap_err().code,
            ErrorCode::MergeTemplateInvalid
        );
    }
}
