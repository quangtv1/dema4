//! Ghi nối bản một chữ ký (kế hoạch 7.2; port `PdfIncrementalSigner.cs` và spike S2).
//!
//! Sửa các giới hạn của bản cũ:
//! - `/AcroForm`, `/Fields`, `/Annots` là tham chiếu gián tiếp thì ghi lại chính object đó (bản cũ báo lỗi).
//! - Giữ đúng số generation khi ghi đè object (bản cũ luôn ghi 0).
//! - `/SigFlags` luôn bật đủ bit 1 và 2.
//!
//! Giữ như bản cũ: một field chữ ký; một khối thì field gộp widget, nhiều khối (ký nháy nhiều trang) thì field
//! có `/Kids`; chữ ký ẩn là widget `/Rect [0 0 0 0]` ở trang 1; ảnh dấu tách riêng là chú thích `/Stamp`;
//! xref mới cùng kiểu với xref cuối của file (bảng, hoặc stream `/W [1 4 2]`), có `/Prev`; `/ByteRange` 4 số
//! rộng 10 ký tự; `/Contents` chừa 32.768 byte.

use std::collections::BTreeMap;

use lopdf::{Dictionary, Object, ObjectId};
use sha2::{Digest, Sha256};
use smks_core::config::{ImageLayout, ImageSplit};
use smks_core::consts::signing as k;
use smks_core::{ErrorCode, Result, SmksError};

use crate::appearance::{self, Alloc, Built, NewObject, StampImage, TextStyle};
use crate::geometry::to_user_space;
use crate::object::{self, pdf_date_vietnam, text};
use crate::reader::{PageInfo, PdfFile, Rect};

fn unsupported(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignPdfStructureUnsupported, message)
}

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignFailed, message)
}

/// Nội dung một khối chữ ký hiển thị.
#[derive(Debug, Clone)]
pub enum WidgetContent {
    Text {
        lines: Vec<String>,
        style: TextStyle,
    },
    Image {
        image: StampImage,
        stroke: i32,
    },
    ImageAndText {
        image: StampImage,
        stroke: i32,
        lines: Vec<String>,
        style: TextStyle,
        /// Ảnh nằm ở đâu so với chữ.
        layout: ImageLayout,
        /// Phần ảnh và khoảng cách ảnh – chữ (TD-51).
        split: ImageSplit,
    },
}

/// Một khối chữ ký hiển thị trên trang `page` (từ 1), khung `rect` theo toạ độ hiển thị.
#[derive(Debug, Clone)]
pub struct Widget {
    pub page: u32,
    pub rect: Rect,
    pub content: WidgetContent,
}

/// Ảnh dấu đặt riêng (chú thích `/Stamp`), cùng lần ghi nối với chữ ký nên được chữ ký phủ.
#[derive(Debug, Clone)]
pub struct Stamp {
    pub page: u32,
    pub rect: Rect,
    pub image: StampImage,
    pub stroke: i32,
}

/// Thông tin của lần ký.
#[derive(Debug, Clone)]
pub struct SignRequest {
    pub signer_name: String,
    pub reason: String,
    pub location: String,
    /// Giây Unix, ghi vào `/M` theo giờ Việt Nam.
    pub signed_at: i64,
    /// Rỗng: chữ ký ẩn.
    pub widgets: Vec<Widget>,
    pub stamps: Vec<Stamp>,
}

/// File đã dựng phần nối, chờ điền CMS vào `/Contents`.
#[derive(Debug, Clone)]
pub struct Prepared {
    bytes: Vec<u8>,
    /// `[0, độ dài phần đầu, vị trí phần sau, độ dài phần sau]`.
    pub byte_range: [usize; 4],
    /// Vị trí chữ số hex đầu tiên của `/Contents`.
    contents_hex_start: usize,
}

impl Prepared {
    /// SHA-256 của các byte theo `/ByteRange`.
    pub fn digest(&self) -> [u8; 32] {
        let [_, len1, start2, len2] = self.byte_range;
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes[..len1]);
        hasher.update(&self.bytes[start2..start2 + len2]);
        hasher.finalize().into()
    }

    /// Điền CMS (DER) vào `/Contents`.
    pub fn finish(mut self, cms: &[u8]) -> Result<Vec<u8>> {
        if cms.len() > k::SIGNATURE_CONTENTS_BYTES {
            return Err(fail(format!(
                "Khối chữ ký {} byte vượt chỗ trống {} byte đã chừa.",
                cms.len(),
                k::SIGNATURE_CONTENTS_BYTES
            )));
        }
        let hex = object::hex_upper(cms);
        let start = self.contents_hex_start;
        self.bytes[start..start + hex.len()].copy_from_slice(hex.as_bytes());
        Ok(self.bytes)
    }
}

struct Writer {
    out: Vec<u8>,
    offsets: BTreeMap<u32, (usize, u16)>,
}

impl Writer {
    fn object(&mut self, id: ObjectId, value: &Object) {
        self.offsets.insert(id.0, (self.out.len(), id.1));
        self.out
            .extend_from_slice(format!("{} {} obj\n", id.0, id.1).as_bytes());
        object::write(&mut self.out, value);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    fn new_object(&mut self, obj: &NewObject) {
        self.offsets.insert(obj.number, (self.out.len(), 0));
        self.out.extend_from_slice(&obj.bytes);
    }
}

fn name(n: &str) -> Object {
    Object::Name(n.as_bytes().to_vec())
}

fn rect_array(r: &Rect) -> Object {
    Object::Array(
        [r.x, r.y, r.right(), r.top()]
            .iter()
            .map(|v| Object::Real(((v * 10_000.0).round() / 10_000.0) as f32))
            .collect(),
    )
}

fn build_widget(
    content: &WidgetContent,
    rect: &Rect,
    rotation: i32,
    alloc: &mut Alloc,
) -> Result<Built> {
    match content {
        WidgetContent::Text { lines, style } => {
            appearance::build_text(alloc, rect.width, rect.height, rotation, lines, style)
        }
        WidgetContent::Image { image, stroke } => Ok(appearance::build_image(
            alloc,
            rect.width,
            rect.height,
            rotation,
            image,
            *stroke,
        )),
        WidgetContent::ImageAndText {
            image,
            stroke,
            lines,
            style,
            layout,
            split,
        } => appearance::build_image_and_text(
            alloc,
            (rect.width, rect.height),
            rotation,
            (image, *stroke),
            lines,
            style,
            (*layout, *split),
        ),
    }
}

/// Dựng phần nối thêm cho một chữ ký.
pub fn prepare(original: &[u8], request: &SignRequest) -> Result<Prepared> {
    let file = PdfFile::open(original)?;
    file.check_signable()?;
    let pages = file.pages()?;
    let page_of = |n: u32| -> Result<&PageInfo> {
        pages.iter().find(|p| p.number == n).ok_or_else(|| {
            SmksError::new(
                ErrorCode::SignPositionInvalid,
                format!("File PDF không có trang {n}."),
            )
        })
    };
    let root_id = file.root_id()?;

    let mut alloc = Alloc(file.next_object_number());
    let field_id = alloc.bump();
    let sig_id = alloc.bump();

    // Khối hiển thị.
    struct Placed {
        page: ObjectId,
        rect: Rect,
        form: Option<u32>,
    }
    let mut widgets: Vec<Placed> = Vec::new();
    let mut new_objects: Vec<NewObject> = Vec::new();
    for w in &request.widgets {
        let page = page_of(w.page)?;
        let built = build_widget(&w.content, &w.rect, page.rotation, &mut alloc)?;
        widgets.push(Placed {
            page: page.id,
            rect: to_user_space(page, &w.rect),
            form: Some(built.form),
        });
        new_objects.extend(built.objects);
    }
    if widgets.is_empty() {
        widgets.push(Placed {
            page: page_of(1)?.id,
            rect: Rect::default(),
            form: None,
        });
    }
    let mut stamps: Vec<Placed> = Vec::new();
    for s in &request.stamps {
        let page = page_of(s.page)?;
        let built = appearance::build_image(
            &mut alloc,
            s.rect.width,
            s.rect.height,
            page.rotation,
            &s.image,
            s.stroke,
        );
        stamps.push(Placed {
            page: page.id,
            rect: to_user_space(page, &s.rect),
            form: Some(built.form),
        });
        new_objects.extend(built.objects);
    }
    let widget_ids: Vec<u32> = if widgets.len() == 1 {
        vec![field_id]
    } else {
        widgets.iter().map(|_| alloc.bump()).collect()
    };
    let stamp_ids: Vec<u32> = stamps.iter().map(|_| alloc.bump()).collect();

    // Ghi đè Catalog / AcroForm / Fields và /Annots của các trang.
    let mut overrides: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut catalog = file.catalog()?.clone();
    match catalog.get(b"AcroForm").ok().cloned() {
        None => {
            let mut form = Dictionary::new();
            form.set("Fields", vec![Object::Reference((field_id, 0))]);
            form.set("SigFlags", 3);
            form.set(
                "DA",
                Object::String(b"/Helv 0 Tf 0 g ".to_vec(), lopdf::StringFormat::Literal),
            );
            catalog.set("AcroForm", form);
            overrides.insert(root_id, Object::Dictionary(catalog));
        }
        Some(Object::Reference(form_id)) => {
            let mut form = file
                .doc
                .get_dictionary(form_id)
                .map_err(|_| unsupported("/AcroForm của file PDF không đọc được."))?
                .clone();
            add_field(&file, &mut form, field_id, &mut overrides)?;
            overrides.insert(form_id, Object::Dictionary(form));
        }
        Some(Object::Dictionary(mut form)) => {
            add_field(&file, &mut form, field_id, &mut overrides)?;
            catalog.set("AcroForm", form);
            overrides.insert(root_id, Object::Dictionary(catalog));
        }
        Some(_) => return Err(unsupported("/AcroForm của Catalog không hợp lệ.")),
    }
    let mut annots_by_page: BTreeMap<ObjectId, Vec<u32>> = BTreeMap::new();
    for (p, id) in widgets.iter().zip(&widget_ids) {
        annots_by_page.entry(p.page).or_default().push(*id);
    }
    for (p, id) in stamps.iter().zip(&stamp_ids) {
        annots_by_page.entry(p.page).or_default().push(*id);
    }
    for (page_id, ids) in &annots_by_page {
        let refs: Vec<Object> = ids.iter().map(|n| Object::Reference((*n, 0))).collect();
        // Trang có thể đã nằm trong overrides (hiếm: Catalog cũng là trang thì không xảy ra).
        let mut page = file
            .doc
            .get_dictionary(*page_id)
            .map_err(|_| unsupported("Không đọc được trang của file PDF."))?
            .clone();
        match page.get(b"Annots").ok().cloned() {
            None => {
                page.set("Annots", refs);
                overrides.insert(*page_id, Object::Dictionary(page));
            }
            Some(Object::Reference(annots_id)) => {
                let mut annots = match overrides.get(&annots_id) {
                    Some(Object::Array(a)) => a.clone(),
                    _ => file
                        .doc
                        .get_object(annots_id)
                        .and_then(Object::as_array)
                        .map_err(|_| unsupported("/Annots của trang không đọc được."))?
                        .clone(),
                };
                annots.extend(refs);
                overrides.insert(annots_id, Object::Array(annots));
            }
            Some(Object::Array(mut annots)) => {
                annots.extend(refs);
                page.set("Annots", annots);
                overrides.insert(*page_id, Object::Dictionary(page));
            }
            Some(_) => return Err(unsupported("/Annots của trang không hợp lệ.")),
        }
    }

    // Ghi phần nối.
    let mut w = Writer {
        out: original.to_vec(),
        offsets: BTreeMap::new(),
    };
    if !matches!(w.out.last(), Some(b'\n' | b'\r')) {
        w.out.push(b'\n');
    }
    for (id, value) in &overrides {
        w.object(*id, value);
    }

    let field_name = text(&format!("{}{}", k::SIGNATURE_FIELD_NAME_PREFIX, field_id));
    let widget_dict = |placed: &Placed| {
        let mut d = Dictionary::new();
        d.set("Type", name("Annot"));
        d.set("Subtype", name("Widget"));
        d.set("P", Object::Reference(placed.page));
        d.set("F", 4);
        d.set("Rect", rect_array(&placed.rect));
        if let Some(form) = placed.form {
            let mut ap = Dictionary::new();
            ap.set("N", Object::Reference((form, 0)));
            d.set("AP", ap);
        }
        d
    };
    if widgets.len() == 1 {
        let mut d = widget_dict(&widgets[0]);
        d.set("FT", name("Sig"));
        d.set("Ff", 0);
        d.set("T", field_name);
        d.set("V", Object::Reference((sig_id, 0)));
        w.object((field_id, 0), &Object::Dictionary(d));
    } else {
        let mut field = Dictionary::new();
        field.set("FT", name("Sig"));
        field.set("Ff", 0);
        field.set("T", field_name);
        field.set("V", Object::Reference((sig_id, 0)));
        field.set(
            "Kids",
            widget_ids
                .iter()
                .map(|n| Object::Reference((*n, 0)))
                .collect::<Vec<_>>(),
        );
        w.object((field_id, 0), &Object::Dictionary(field));
        for (placed, id) in widgets.iter().zip(&widget_ids) {
            let mut d = widget_dict(placed);
            d.set("Parent", Object::Reference((field_id, 0)));
            w.object((*id, 0), &Object::Dictionary(d));
        }
    }
    for (placed, id) in stamps.iter().zip(&stamp_ids) {
        let mut d = widget_dict(placed);
        d.set("Subtype", name("Stamp"));
        w.object((*id, 0), &Object::Dictionary(d));
    }
    for obj in &new_objects {
        w.new_object(obj);
    }

    // Từ điển chữ ký: /ByteRange và /Contents là chỗ trống độ dài cố định, điền sau.
    w.offsets.insert(sig_id, (w.out.len(), 0));
    let mut head = Vec::new();
    head.extend_from_slice(
        format!(
            "{sig_id} 0 obj\n<</Type /Sig/Filter /{}/SubFilter /{}/Name ",
            k::PDF_FILTER,
            k::PDF_SUB_FILTER
        )
        .as_bytes(),
    );
    object::write(&mut head, &text(&request.signer_name));
    head.extend_from_slice(b"/Reason ");
    object::write(&mut head, &text(&request.reason));
    head.extend_from_slice(b"/Location ");
    object::write(&mut head, &text(&request.location));
    head.extend_from_slice(format!("/M ({})", pdf_date_vietnam(request.signed_at)).as_bytes());
    head.extend_from_slice(b"/ByteRange [");
    w.out.extend_from_slice(&head);
    let byte_range_at = w.out.len();
    let placeholder_len = k::BYTE_RANGE_FIELD_WIDTH * 4 + 3;
    w.out.extend(std::iter::repeat_n(b' ', placeholder_len));
    w.out.extend_from_slice(b"]/Contents <");
    let contents_hex_start = w.out.len();
    w.out
        .extend(std::iter::repeat_n(b'0', k::SIGNATURE_CONTENTS_BYTES * 2));
    w.out.extend_from_slice(b">>>\nendobj\n");

    write_xref(&mut w, &file, root_id, &mut alloc)?;

    // /ByteRange thật: mỗi số rộng 10 ký tự, đệm khoảng trắng bên phải như bản cũ.
    let gap_start = contents_hex_start - 1;
    let gap_end = contents_hex_start + k::SIGNATURE_CONTENTS_BYTES * 2 + 1;
    let byte_range = [0, gap_start, gap_end, w.out.len() - gap_end];
    let text_range = byte_range
        .iter()
        .map(|v| format!("{v:<width$}", width = k::BYTE_RANGE_FIELD_WIDTH))
        .collect::<Vec<_>>()
        .join(" ");
    if text_range.len() != placeholder_len {
        return Err(fail(
            "/ByteRange thật dài khác chỗ trống đã chừa (file quá lớn).",
        ));
    }
    w.out[byte_range_at..byte_range_at + placeholder_len].copy_from_slice(text_range.as_bytes());

    Ok(Prepared {
        bytes: w.out,
        byte_range,
        contents_hex_start,
    })
}

/// Ghi xref (cùng kiểu với xref cuối của file, có `/Prev`) và `startxref` / `%%EOF` cho phần nối.
fn write_xref(w: &mut Writer, file: &PdfFile, root_id: ObjectId, alloc: &mut Alloc) -> Result<()> {
    let xref_offset = w.out.len();
    let mut trailer = Dictionary::new();
    trailer.set("Root", Object::Reference(root_id));
    for key in [b"Info".as_slice(), b"ID".as_slice()] {
        if let Ok(value) = file.doc.trailer.get(key) {
            trailer.set(key, value.clone());
        }
    }
    trailer.set("Prev", file.last_xref as i64);
    if file.xref_stream {
        let xref_id = alloc.bump();
        w.offsets.insert(xref_id, (xref_offset, 0));
        let mut index = Vec::new();
        let mut rows = Vec::new();
        for (start, run) in runs(&w.offsets) {
            index.push(Object::Integer(i64::from(start)));
            index.push(Object::Integer(run.len() as i64));
            for (offset, generation) in run {
                let offset =
                    u32::try_from(offset).map_err(|_| unsupported("File PDF lớn hơn 4GB."))?;
                rows.push(1u8);
                rows.extend_from_slice(&offset.to_be_bytes());
                rows.extend_from_slice(&generation.to_be_bytes());
            }
        }
        trailer.set("Type", name("XRef"));
        trailer.set("Size", i64::from(alloc.0));
        trailer.set("Index", index);
        trailer.set("W", vec![1.into(), 4.into(), 2.into()]);
        let mut stream = lopdf::Stream::new(trailer, rows);
        stream.allows_compression = false;
        w.out
            .extend_from_slice(format!("{xref_id} 0 obj\n").as_bytes());
        object::write(&mut w.out, &Object::Stream(stream));
        w.out.extend_from_slice(b"\nendobj\n");
    } else {
        trailer.set("Size", i64::from(alloc.0));
        w.out.extend_from_slice(b"xref\n");
        for (start, run) in runs(&w.offsets) {
            w.out
                .extend_from_slice(format!("{start} {}\n", run.len()).as_bytes());
            for (offset, generation) in run {
                w.out
                    .extend_from_slice(format!("{offset:010} {generation:05} n\r\n").as_bytes());
            }
        }
        w.out.extend_from_slice(b"trailer\n");
        object::write(&mut w.out, &Object::Dictionary(trailer));
        w.out.push(b'\n');
    }
    w.out
        .extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    Ok(())
}

/// Dữ liệu kiểm tra dài hạn nhúng vào `/DSS` (PAdES B-LT, TD-58): chứng thư, phản hồi OCSP, CRL (DER).
#[derive(Debug, Clone, Default)]
pub struct DssData {
    pub certs: Vec<Vec<u8>>,
    pub ocsps: Vec<Vec<u8>>,
    pub crls: Vec<Vec<u8>>,
}

impl DssData {
    pub fn is_empty(&self) -> bool {
        self.certs.is_empty() && self.ocsps.is_empty() && self.crls.is_empty()
    }
}

/// Nội dung các stream của một mảng trong `/DSS` (bỏ qua phần tử không đọc được).
fn dss_items(file: &PdfFile, dss: &Dictionary, key: &[u8]) -> (Vec<Object>, Vec<Vec<u8>>) {
    let Some(Object::Array(items)) = dss.get(key).ok().map(|o| file.resolve(o)) else {
        return (Vec::new(), Vec::new());
    };
    let contents = items
        .iter()
        .filter_map(|o| match file.resolve(o) {
            Object::Stream(s) => s
                .decompressed_content()
                .ok()
                .or_else(|| Some(s.content.clone())),
            _ => None,
        })
        .collect();
    (items.clone(), contents)
}

/// Đọc `/DSS` của file (nếu có).
pub fn read_dss(bytes: &[u8]) -> Result<Option<DssData>> {
    let file = PdfFile::open(bytes)?;
    let catalog = file.catalog()?;
    let Some(Object::Dictionary(dss)) = catalog.get(b"DSS").ok().map(|o| file.resolve(o)) else {
        return Ok(None);
    };
    Ok(Some(DssData {
        certs: dss_items(&file, dss, b"Certs").1,
        ocsps: dss_items(&file, dss, b"OCSPs").1,
        crls: dss_items(&file, dss, b"CRLs").1,
    }))
}

/// Nối một bản cập nhật thêm `data` vào `/DSS` của Catalog (giữ phần đã có, không thêm trùng). Không ký lại:
/// chữ ký trước vẫn khớp vì chỉ nối thêm (PAdES B-LT).
pub fn append_dss(original: &[u8], data: &DssData) -> Result<Vec<u8>> {
    let file = PdfFile::open(original)?;
    let root_id = file.root_id()?;
    let mut catalog = file.catalog()?.clone();
    let existing = match catalog.get(b"DSS").ok() {
        Some(Object::Reference(id)) => Some((
            Some(*id),
            file.doc
                .get_dictionary(*id)
                .map_err(|_| unsupported("/DSS của file PDF không đọc được."))?
                .clone(),
        )),
        Some(Object::Dictionary(d)) => Some((None, d.clone())),
        _ => None,
    };
    let mut alloc = Alloc(file.next_object_number());
    let mut w = Writer {
        out: original.to_vec(),
        offsets: BTreeMap::new(),
    };
    if !matches!(w.out.last(), Some(b'\n' | b'\r')) {
        w.out.push(b'\n');
    }
    let mut dss = existing
        .as_ref()
        .map(|(_, d)| d.clone())
        .unwrap_or_default();
    for (key, items) in [
        (b"Certs".as_slice(), &data.certs),
        (b"OCSPs".as_slice(), &data.ocsps),
        (b"CRLs".as_slice(), &data.crls),
    ] {
        let (mut refs, have) = existing
            .as_ref()
            .map(|(_, d)| dss_items(&file, d, key))
            .unwrap_or_default();
        let mut added = false;
        for item in items {
            if have.contains(item) {
                continue;
            }
            let id = alloc.bump();
            let mut stream = lopdf::Stream::new(Dictionary::new(), item.clone());
            stream.allows_compression = false;
            w.object((id, 0), &Object::Stream(stream));
            refs.push(Object::Reference((id, 0)));
            added = true;
        }
        if added && !refs.is_empty() {
            dss.set(key, refs);
        }
    }
    dss.set("Type", name("DSS"));
    match existing.and_then(|(id, _)| id) {
        Some(id) => w.object(id, &Object::Dictionary(dss)),
        None => {
            let id = alloc.bump();
            w.object((id, 0), &Object::Dictionary(dss));
            catalog.set("DSS", Object::Reference((id, 0)));
            w.object(root_id, &Object::Dictionary(catalog));
        }
    }
    write_xref(&mut w, &file, root_id, &mut alloc)?;
    Ok(w.out)
}

/// Nhóm các số object liên tiếp thành từng đoạn của xref.
fn runs(offsets: &BTreeMap<u32, (usize, u16)>) -> Vec<(u32, Vec<(usize, u16)>)> {
    let mut out: Vec<(u32, Vec<(usize, u16)>)> = Vec::new();
    for (number, entry) in offsets {
        match out.last_mut() {
            Some((start, run)) if *start + run.len() as u32 == *number => run.push(*entry),
            _ => out.push((*number, vec![*entry])),
        }
    }
    out
}

fn add_field(
    file: &PdfFile,
    form: &mut Dictionary,
    field_id: u32,
    overrides: &mut BTreeMap<ObjectId, Object>,
) -> Result<()> {
    match form.get(b"Fields").ok().cloned() {
        Some(Object::Reference(fields_id)) => {
            let mut fields = file
                .doc
                .get_object(fields_id)
                .and_then(Object::as_array)
                .map_err(|_| unsupported("/Fields của AcroForm không đọc được."))?
                .clone();
            fields.push(Object::Reference((field_id, 0)));
            overrides.insert(fields_id, Object::Array(fields));
        }
        Some(Object::Array(mut fields)) => {
            fields.push(Object::Reference((field_id, 0)));
            form.set("Fields", fields);
        }
        _ => form.set("Fields", vec![Object::Reference((field_id, 0))]),
    }
    let flags = form.get(b"SigFlags").and_then(Object::as_i64).unwrap_or(0);
    form.set("SigFlags", flags | 3);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::{FontData, StampImage, TextStyle};
    use crate::sigscan;

    #[test]
    fn dss_is_appended_and_merged_without_duplicates() {
        for name in ["table-xref.pdf", "xref-stream.pdf", "word-hybrid-xref.pdf"] {
            let original = fixture(name);
            let data = DssData {
                certs: vec![b"chung thu 1".to_vec(), b"chung thu 2".to_vec()],
                ocsps: vec![b"ocsp 1".to_vec()],
                crls: vec![],
            };
            let once = append_dss(&original, &data).unwrap();
            assert!(once.starts_with(&original), "{name}: chỉ nối thêm");
            let read = read_dss(&once).unwrap().unwrap();
            assert_eq!(read.certs, data.certs, "{name}");
            assert_eq!(read.ocsps, data.ocsps, "{name}");
            assert!(read.crls.is_empty());
            // Lần hai: chứng thư trùng không thêm, OCSP mới và CRL được thêm vào cùng /DSS.
            let more = DssData {
                certs: vec![b"chung thu 2".to_vec()],
                ocsps: vec![b"ocsp 2".to_vec()],
                crls: vec![b"crl 1".to_vec()],
            };
            let twice = append_dss(&once, &more).unwrap();
            assert!(twice.starts_with(&once));
            let read = read_dss(&twice).unwrap().unwrap();
            assert_eq!(read.certs.len(), 2, "{name}");
            assert_eq!(
                read.ocsps,
                vec![b"ocsp 1".to_vec(), b"ocsp 2".to_vec()],
                "{name}"
            );
            assert_eq!(read.crls, vec![b"crl 1".to_vec()], "{name}");
            // File vẫn đọc được trang như cũ.
            assert_eq!(
                PdfFile::open(&twice).unwrap().pages().unwrap().len(),
                PdfFile::open(&original).unwrap().pages().unwrap().len()
            );
            assert!(read_dss(&original).unwrap().is_none());
        }
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/pdf")
                .join(name),
        )
        .unwrap()
    }

    fn request(widgets: Vec<Widget>, stamps: Vec<Stamp>) -> SignRequest {
        SignRequest {
            signer_name: "Nguyễn Văn Thử".into(),
            reason: k::DEFAULT_REASON.into(),
            location: k::DEFAULT_LOCATION.into(),
            signed_at: 1_795_057_200,
            widgets,
            stamps,
        }
    }

    /// "CMS" giả: băm của phần được ký, đủ để kiểm cấu trúc và vị trí.
    fn sign_fake(input: &[u8], req: &SignRequest) -> Vec<u8> {
        let prepared = prepare(input, req).unwrap();
        let digest = prepared.digest();
        let mut cms = vec![0x30, 0x22, 0x04, 0x20];
        cms.extend_from_slice(&digest);
        prepared.finish(&cms).unwrap()
    }

    fn red_png() -> Vec<u8> {
        let img = image::RgbaImage::from_fn(40, 20, |x, _| {
            if x % 2 == 0 {
                image::Rgba([200, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        });
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    fn text_widget(page: u32) -> Widget {
        let lines = vec![
            "Ký bởi: Nguyễn Văn Thử".to_owned(),
            "Ngày ký: 19/11/2026 10:00".to_owned(),
        ];
        let (font, _) = FontData::for_lines("Times New Roman", &lines).unwrap();
        Widget {
            page,
            rect: Rect::new(300.0, 700.0, 170.0, 30.0),
            content: WidgetContent::Text {
                lines,
                style: TextStyle {
                    font,
                    size: 10.0,
                    color: [0.0; 3],
                },
            },
        }
    }

    #[test]
    fn hidden_signature_on_every_xref_kind_and_countersign() {
        for name in ["table-xref.pdf", "xref-stream.pdf", "word-hybrid-xref.pdf"] {
            let original = fixture(name);
            let once = sign_fake(&original, &request(vec![], vec![]));
            assert!(
                once.starts_with(&original),
                "{name}: phải giữ nguyên file gốc"
            );
            let sigs = sigscan::scan(&once).unwrap();
            assert_eq!(sigs.len(), 1, "{name}");
            let s = &sigs[0];
            assert!(
                s.range_ok && s.covers_whole_file,
                "{name}: {:?}",
                s.byte_range
            );
            assert_eq!(s.sub_filter, "ETSI.CAdES.detached");
            assert_eq!(s.name.as_deref(), Some("Nguyễn Văn Thử"));
            assert_eq!(s.signing_time.as_deref(), Some("D:20261119100000+07'00'"));
            assert_eq!(s.page, Some(1));
            assert!(!s.visible);
            // Chữ ký thứ hai không làm hỏng chữ ký thứ nhất: phần đã ký giữ nguyên byte.
            let twice = sign_fake(&once, &request(vec![text_widget(1)], vec![]));
            assert!(twice.starts_with(&once));
            let sigs = sigscan::scan(&twice).unwrap();
            assert_eq!(sigs.len(), 2, "{name}");
            assert!(!sigs[0].covers_whole_file && sigs[1].covers_whole_file);
            assert!(sigs[1].visible);
            let parts = sigs[0].signed_parts(&twice).unwrap();
            let mut h = Sha256::new();
            h.update(parts[0]);
            h.update(parts[1]);
            let digest: [u8; 32] = h.finalize().into();
            assert_eq!(
                &sigs[0].contents[4..36],
                &digest,
                "{name}: chữ ký 1 vẫn khớp nội dung"
            );
            // lopdf đọc lại được toàn bộ.
            assert!(!PdfFile::open(&twice).unwrap().pages().unwrap().is_empty());
        }
    }

    #[test]
    fn several_widgets_and_stamps() {
        let original = fixture("table-xref.pdf");
        let image = StampImage::prepare(&red_png(), 140, None, 200).unwrap();
        let req = request(
            vec![text_widget(1), text_widget(1)],
            vec![Stamp {
                page: 1,
                rect: Rect::new(50.0, 50.0, 80.0, 40.0),
                image,
                stroke: 100,
            }],
        );
        let signed = sign_fake(&original, &req);
        let file = PdfFile::open(&signed).unwrap();
        let sigs = sigscan::scan(&signed).unwrap();
        assert_eq!(sigs.len(), 1);
        assert!(sigs[0].visible);
        let page = file
            .doc
            .get_dictionary(file.pages().unwrap()[0].id)
            .unwrap();
        let annots = file
            .resolve(page.get(b"Annots").unwrap())
            .as_array()
            .unwrap();
        let subtypes: Vec<String> = annots
            .iter()
            .filter_map(|a| file.resolve(a).as_dict().ok())
            .filter_map(|d| d.get(b"Subtype").and_then(Object::as_name).ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .collect();
        assert_eq!(subtypes.iter().filter(|s| *s == "Widget").count(), 2);
        assert_eq!(subtypes.iter().filter(|s| *s == "Stamp").count(), 1);
        // Ảnh có SMask (PNG có alpha).
        assert!(String::from_utf8_lossy(&signed).contains("/SMask"));
    }

    #[test]
    fn rejects_missing_page_and_oversized_cms() {
        let original = fixture("table-xref.pdf");
        let err = prepare(&original, &request(vec![text_widget(9)], vec![])).unwrap_err();
        assert_eq!(err.code, ErrorCode::SignPositionInvalid);
        let prepared = prepare(&original, &request(vec![], vec![])).unwrap();
        let err = prepared
            .finish(&vec![0u8; k::SIGNATURE_CONTENTS_BYTES + 1])
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SignFailed);
        let err = prepare(b"%PDF-1.7\nkhong phai pdf", &request(vec![], vec![])).unwrap_err();
        assert_eq!(err.code, ErrorCode::SignPdfStructureUnsupported);
    }
}
