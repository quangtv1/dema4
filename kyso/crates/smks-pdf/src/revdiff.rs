//! So một bản (revision) đã ký với file hiện tại để biết **sau chữ ký có gì bị thay đổi** (như Adobe báo
//! "tài liệu đã bị sửa sau khi ký"). Mỗi bản cập nhật tăng dần chỉ được phép thêm chữ ký / dấu thời gian,
//! dữ liệu kiểm tra dài hạn (DSS), điền biểu mẫu, chú thích; còn lại (đổi nội dung trang, tài nguyên, cây
//! trang, xoá đối tượng…) là "sửa nội dung".
//!
//! Cách làm theo kiểu "mặc định từ chối": đối tượng **mới** không tự có tác dụng nếu không được một đối tượng
//! cũ trỏ tới, nên chỉ xét các đối tượng **cũ bị thay** hoặc bị xoá. Mỗi đối tượng cũ được nhận vai trò
//! (Catalog, AcroForm, trang, mảng `/Annots`, field chữ ký, …); đối tượng không có vai trò được phép thay
//! mà bị thay thì là "sửa nội dung".

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;
use smks_core::{ErrorCode, Result, SmksError};

/// Mức thay đổi sau một chữ ký, từ nhẹ đến nặng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeLevel {
    /// Không có bản cập nhật nào sau chữ ký.
    None,
    /// Chỉ thêm chữ ký, dấu thời gian, dữ liệu kiểm tra dài hạn (DSS) hoặc thông tin tài liệu.
    Signatures,
    /// Có điền biểu mẫu.
    FormFill,
    /// Có thêm chú thích (annotation) hoặc trường biểu mẫu mới.
    Annotations,
    /// Nội dung hoặc cấu trúc văn bản bị sửa.
    Other,
}

/// Một loại thay đổi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeNote {
    pub level: ChangeLevel,
    /// Mô tả tiếng Việt.
    pub text: String,
}

/// Kết quả so sánh.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionChanges {
    pub level: ChangeLevel,
    /// Từng loại thay đổi (không trùng).
    pub notes: Vec<ChangeNote>,
}

impl RevisionChanges {
    fn none() -> Self {
        Self {
            level: ChangeLevel::None,
            notes: Vec::new(),
        }
    }

    fn add(&mut self, level: ChangeLevel, note: impl Into<String>) {
        self.level = self.level.max(level);
        let note = ChangeNote {
            level,
            text: note.into(),
        };
        if !self.notes.contains(&note) {
            self.notes.push(note);
        }
    }

    /// Mô tả các thay đổi đúng mức `level`, nối bằng dấu cách.
    pub fn text_at(&self, level: ChangeLevel) -> String {
        self.notes
            .iter()
            .filter(|n| n.level == level)
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Vai trò của một đối tượng trong bản cũ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Catalog,
    Info,
    Metadata,
    AcroForm,
    /// `/DR` của AcroForm hoặc từ điển con của nó (font dùng chung của biểu mẫu).
    FormResources,
    FieldsArray,
    Page(u32),
    AnnotsArray(u32),
    /// Field chữ ký (hoặc widget của nó); `true` khi đã có `/V`.
    SigField(bool),
    /// Appearance của field chữ ký đã ký.
    SignedAppearance,
    OtherField,
    Dss,
}

fn load(bytes: &[u8]) -> Result<Document> {
    Document::load_mem(bytes).map_err(|e| {
        SmksError::new(
            ErrorCode::SignPdfStructureUnsupported,
            format!("Không đọc được cấu trúc file PDF: {e}"),
        )
    })
}

fn deref<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    doc.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

fn dict_of<'a>(doc: &'a Document, o: &'a Object) -> Option<&'a Dictionary> {
    deref(doc, o).as_dict().ok()
}

fn name_of<'a>(dict: &'a Dictionary, key: &[u8]) -> Option<&'a [u8]> {
    dict.get(key).and_then(Object::as_name).ok()
}

/// Gắn vai trò cho các đối tượng gián tiếp của `doc`.
fn roles(doc: &Document) -> HashMap<ObjectId, Role> {
    let mut map = HashMap::new();
    let put = |o: &Object, role: Role, map: &mut HashMap<ObjectId, Role>| {
        if let Ok(id) = o.as_reference() {
            map.entry(id).or_insert(role);
        }
    };
    if let Ok(info) = doc.trailer.get(b"Info") {
        put(info, Role::Info, &mut map);
    }
    let Ok(root) = doc.trailer.get(b"Root") else {
        return map;
    };
    put(root, Role::Catalog, &mut map);
    for (number, id) in doc.get_pages() {
        map.insert(id, Role::Page(number));
        if let Ok(page) = doc.get_dictionary(id)
            && let Ok(annots) = page.get(b"Annots")
        {
            put(annots, Role::AnnotsArray(number), &mut map);
        }
    }
    let Some(catalog) = dict_of(doc, root) else {
        return map;
    };
    if let Ok(m) = catalog.get(b"Metadata") {
        put(m, Role::Metadata, &mut map);
    }
    if let Ok(dss) = catalog.get(b"DSS") {
        put(dss, Role::Dss, &mut map);
        if let Some(d) = dict_of(doc, dss) {
            for (_, v) in d.iter() {
                put(v, Role::Dss, &mut map);
            }
        }
    }
    let Ok(form) = catalog.get(b"AcroForm") else {
        return map;
    };
    put(form, Role::AcroForm, &mut map);
    let Some(form) = dict_of(doc, form) else {
        return map;
    };
    if let Ok(dr) = form.get(b"DR") {
        put(dr, Role::FormResources, &mut map);
        if let Some(d) = dict_of(doc, dr) {
            for (_, v) in d.iter() {
                put(v, Role::FormResources, &mut map);
            }
        }
    }
    let Ok(fields) = form.get(b"Fields") else {
        return map;
    };
    put(fields, Role::FieldsArray, &mut map);
    let Ok(fields) = deref(doc, fields).as_array() else {
        return map;
    };
    // Duyệt cây field, kế thừa /FT và trạng thái đã ký từ cha.
    let mut stack: Vec<(&Object, Option<bool>, usize)> =
        fields.iter().map(|f| (f, None, 0)).collect();
    while let Some((node, parent_sig, depth)) = stack.pop() {
        if depth > 32 {
            continue;
        }
        let Some(d) = dict_of(doc, node) else {
            continue;
        };
        let sig = match name_of(d, b"FT") {
            Some(ft) => (ft == b"Sig").then(|| d.has(b"V")),
            None => parent_sig.map(|signed| signed || d.has(b"V")),
        };
        let role = match sig {
            Some(signed) => Role::SigField(signed),
            None => Role::OtherField,
        };
        put(node, role, &mut map);
        if sig == Some(true)
            && let Some(ap) = d.get(b"AP").ok().and_then(|a| dict_of(doc, a))
        {
            put(
                d.get(b"AP").unwrap_or(&Object::Null),
                Role::SignedAppearance,
                &mut map,
            );
            for (_, v) in ap.iter() {
                put(v, Role::SignedAppearance, &mut map);
            }
        }
        if let Ok(Object::Array(kids)) = d.get(b"Kids").map(|k| deref(doc, k)) {
            for k in kids {
                stack.push((k, sig, depth + 1));
            }
        }
    }
    map
}

/// Khoá có giá trị khác nhau giữa hai từ điển (kể cả thêm / bớt).
fn changed_keys(old: &Dictionary, new: &Dictionary) -> Vec<Vec<u8>> {
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for (k, v) in old.iter() {
        if new.get(k).ok() != Some(v) {
            keys.push(k.clone());
        }
    }
    for (k, _) in new.iter() {
        if !old.has(k) {
            keys.push(k.clone());
        }
    }
    keys
}

/// Mọi phần tử của `old` vẫn còn trong `new`; trả về phần tử thêm vào.
fn added_items<'a>(old: &[Object], new: &'a [Object]) -> Option<Vec<&'a Object>> {
    let mut rest: Vec<&Object> = new.iter().collect();
    for o in old {
        let at = rest.iter().position(|n| *n == o)?;
        rest.remove(at);
    }
    Some(rest)
}

fn key_text(k: &[u8]) -> String {
    format!("/{}", String::from_utf8_lossy(k))
}

/// Phân loại một chú thích được thêm vào trang `page` (đọc trong bản mới).
fn classify_annot(new: &Document, annot: &Object, page: u32, out: &mut RevisionChanges) {
    let Some(d) = dict_of(new, annot) else {
        out.add(
            ChangeLevel::Other,
            format!("Trang {page}: thêm chú thích không đọc được."),
        );
        return;
    };
    let subtype = name_of(d, b"Subtype").unwrap_or_default();
    if subtype == b"Widget" {
        let ft = name_of(d, b"FT").or_else(|| {
            d.get(b"Parent")
                .ok()
                .and_then(|p| dict_of(new, p))
                .and_then(|p| name_of(p, b"FT"))
        });
        if ft == Some(b"Sig".as_slice()) {
            out.add(ChangeLevel::Signatures, "Thêm chữ ký số sau chữ ký này.");
        } else {
            out.add(
                ChangeLevel::Annotations,
                format!("Trang {page}: thêm trường biểu mẫu."),
            );
        }
    } else {
        out.add(
            ChangeLevel::Annotations,
            format!(
                "Trang {page}: thêm chú thích ({}).",
                String::from_utf8_lossy(subtype)
            ),
        );
    }
}

fn is_structural(o: &Object) -> bool {
    match o {
        Object::Stream(s) => s.dict.has_type(b"XRef") || s.dict.has_type(b"ObjStm"),
        _ => false,
    }
}

fn label(id: ObjectId) -> String {
    format!("đối tượng {} {}", id.0, id.1)
}

/// Xét một đối tượng cũ bị thay.
fn review(
    role: Option<Role>,
    id: ObjectId,
    old_doc: &Document,
    old: &Object,
    new_doc: &Document,
    new: &Object,
    out: &mut RevisionChanges,
) {
    let other = |out: &mut RevisionChanges, what: String| out.add(ChangeLevel::Other, what);
    match role {
        Some(Role::Info) | Some(Role::Metadata) => {
            out.add(
                ChangeLevel::Signatures,
                "Cập nhật thông tin tài liệu (Info / Metadata).",
            );
        }
        Some(Role::Dss) => out.add(
            ChangeLevel::Signatures,
            "Bổ sung dữ liệu kiểm tra dài hạn (DSS).",
        ),
        Some(Role::Catalog) => {
            let (Ok(o), Ok(n)) = (old.as_dict(), new.as_dict()) else {
                return other(out, "Catalog của văn bản bị thay.".into());
            };
            for k in changed_keys(o, n) {
                match k.as_slice() {
                    b"DSS" => out.add(
                        ChangeLevel::Signatures,
                        "Bổ sung dữ liệu kiểm tra dài hạn (DSS).",
                    ),
                    b"Metadata" | b"Extensions" | b"Version" => out.add(
                        ChangeLevel::Signatures,
                        "Cập nhật thông tin tài liệu (Info / Metadata).",
                    ),
                    b"AcroForm" => match (o.get(b"AcroForm"), n.get(b"AcroForm")) {
                        (Ok(Object::Dictionary(a)), Ok(Object::Dictionary(b))) => {
                            review_acroform(old_doc, a, new_doc, b, out);
                        }
                        // Thêm AcroForm mới (bản cũ chưa có biểu mẫu) hoặc đổi tham chiếu.
                        (Err(_), Ok(_)) => {
                            out.add(ChangeLevel::Signatures, "Thêm biểu mẫu chữ ký.")
                        }
                        _ => other(out, "Biểu mẫu (AcroForm) của văn bản bị thay.".into()),
                    },
                    _ => other(
                        out,
                        format!("Catalog của văn bản bị sửa ({}).", key_text(&k)),
                    ),
                }
            }
        }
        Some(Role::AcroForm) => match (old.as_dict(), new.as_dict()) {
            (Ok(a), Ok(b)) => review_acroform(old_doc, a, new_doc, b, out),
            _ => other(out, "Biểu mẫu (AcroForm) của văn bản bị thay.".into()),
        },
        Some(Role::FormResources) => match (old.as_dict(), new.as_dict()) {
            // Chỉ được thêm tài nguyên (font của khối chữ ký), không đổi cái đã có.
            (Ok(a), Ok(b)) if a.iter().all(|(k, v)| b.get(k).ok() == Some(v)) => {
                out.add(ChangeLevel::Signatures, "Thêm font dùng cho biểu mẫu.");
            }
            _ => other(out, "Tài nguyên dùng chung của biểu mẫu bị sửa.".into()),
        },
        Some(Role::FieldsArray) => match (old.as_array(), new.as_array()) {
            (Ok(a), Ok(b)) if added_items(a, b).is_some() => {}
            _ => other(
                out,
                "Danh sách trường biểu mẫu bị sửa (có trường bị bỏ).".into(),
            ),
        },
        Some(Role::Page(n)) => {
            let (Ok(o), Ok(p)) = (old.as_dict(), new.as_dict()) else {
                return other(out, format!("Trang {n} bị thay."));
            };
            for k in changed_keys(o, p) {
                if k != b"Annots" {
                    other(
                        out,
                        format!(
                            "Trang {n} bị sửa ({}): nội dung hiển thị có thể khác bản đã ký.",
                            key_text(&k)
                        ),
                    );
                    continue;
                }
                let empty = Vec::new();
                let a = o.get(b"Annots").ok().map(|x| deref(old_doc, x));
                let b = p.get(b"Annots").ok().map(|x| deref(new_doc, x));
                let a = a.and_then(|x| x.as_array().ok()).unwrap_or(&empty);
                match b.and_then(|x| x.as_array().ok()) {
                    Some(b) => match added_items(a, b) {
                        Some(added) => {
                            for x in added {
                                classify_annot(new_doc, x, n, out);
                            }
                        }
                        None => other(
                            out,
                            format!("Trang {n}: có chú thích / khối chữ ký bị xoá."),
                        ),
                    },
                    None if a.is_empty() => {}
                    None => other(
                        out,
                        format!("Trang {n}: có chú thích / khối chữ ký bị xoá."),
                    ),
                }
            }
        }
        Some(Role::AnnotsArray(n)) => match (old.as_array(), new.as_array()) {
            (Ok(a), Ok(b)) => match added_items(a, b) {
                Some(added) => {
                    for x in added {
                        classify_annot(new_doc, x, n, out);
                    }
                }
                None => other(
                    out,
                    format!("Trang {n}: có chú thích / khối chữ ký bị xoá."),
                ),
            },
            _ => other(out, format!("Trang {n}: danh sách chú thích bị thay.")),
        },
        Some(Role::SigField(true)) | Some(Role::SignedAppearance) => other(
            out,
            "Một chữ ký đã có (hoặc hình hiển thị của nó) bị thay sau chữ ký này.".into(),
        ),
        Some(Role::SigField(false)) => {
            let (Ok(o), Ok(p)) = (old.as_dict(), new.as_dict()) else {
                return other(out, "Trường chữ ký bị thay.".into());
            };
            const ALLOWED: [&[u8]; 9] = [
                b"V", b"AP", b"AS", b"F", b"MK", b"P", b"Rect", b"DA", b"Lock",
            ];
            if changed_keys(o, p)
                .iter()
                .all(|k| ALLOWED.contains(&k.as_slice()))
            {
                out.add(ChangeLevel::Signatures, "Thêm chữ ký số sau chữ ký này.");
            } else {
                other(out, "Trường chữ ký trống bị sửa ngoài phần ký.".into());
            }
        }
        Some(Role::OtherField) => {
            let (Ok(o), Ok(p)) = (old.as_dict(), new.as_dict()) else {
                return other(out, "Trường biểu mẫu bị thay.".into());
            };
            const ALLOWED: [&[u8]; 4] = [b"V", b"AP", b"AS", b"MK"];
            if changed_keys(o, p)
                .iter()
                .all(|k| ALLOWED.contains(&k.as_slice()))
            {
                out.add(ChangeLevel::FormFill, "Điền / sửa giá trị biểu mẫu.");
            } else {
                other(out, "Trường biểu mẫu bị sửa.".into());
            }
        }
        None => other(
            out,
            format!(
                "Nội dung văn bản bị sửa ({}): có thể là chữ, hình, font hoặc tài nguyên của trang.",
                label(id)
            ),
        ),
    }
}

fn review_acroform(
    old_doc: &Document,
    old: &Dictionary,
    new_doc: &Document,
    new: &Dictionary,
    out: &mut RevisionChanges,
) {
    for k in changed_keys(old, new) {
        match k.as_slice() {
            b"Fields" => {
                let empty = Vec::new();
                let a = old.get(b"Fields").ok().map(|x| deref(old_doc, x));
                let b = new.get(b"Fields").ok().map(|x| deref(new_doc, x));
                let a = a.and_then(|x| x.as_array().ok()).unwrap_or(&empty);
                let ok = b
                    .and_then(|x| x.as_array().ok())
                    .is_some_and(|b| added_items(a, b).is_some());
                if !ok {
                    out.add(
                        ChangeLevel::Other,
                        "Danh sách trường biểu mẫu bị sửa (có trường bị bỏ).",
                    );
                }
            }
            b"SigFlags" | b"DR" | b"DA" | b"NeedAppearances" => {}
            _ => out.add(
                ChangeLevel::Other,
                format!("Biểu mẫu (AcroForm) bị sửa ({}).", key_text(&k)),
            ),
        }
    }
}

/// Những gì thay đổi trong `bytes` so với bản kết thúc ở byte `revision_end` (bản chứa chữ ký).
pub fn changes_after(bytes: &[u8], revision_end: usize) -> Result<RevisionChanges> {
    if revision_end >= bytes.len() {
        return Ok(RevisionChanges::none());
    }
    let old_doc = load(&bytes[..revision_end])?;
    let new_doc = load(bytes)?;
    let role_of = roles(&old_doc);
    let mut out = RevisionChanges::none();
    let ids: HashSet<ObjectId> = old_doc
        .objects
        .keys()
        .chain(new_doc.objects.keys())
        .copied()
        .collect();
    let mut ids: Vec<ObjectId> = ids.into_iter().collect();
    ids.sort_unstable();
    for id in ids {
        match (old_doc.objects.get(&id), new_doc.objects.get(&id)) {
            (Some(o), Some(n)) if o == n => {}
            (Some(o), Some(n)) => {
                if is_structural(o) && is_structural(n) {
                    continue;
                }
                review(
                    role_of.get(&id).copied(),
                    id,
                    &old_doc,
                    o,
                    &new_doc,
                    n,
                    &mut out,
                );
            }
            (Some(o), None) => {
                if !is_structural(o) {
                    out.add(
                        ChangeLevel::Other,
                        format!("Có đối tượng của bản đã ký bị xoá ({}).", label(id)),
                    );
                }
            }
            // Đối tượng mới: chỉ có tác dụng qua đối tượng cũ trỏ tới (đã xét ở trên).
            (None, _) => {}
        }
    }
    // Bản mới phải còn đúng Catalog cũ (đổi /Root sang Catalog khác là thay cả văn bản).
    let old_root = old_doc
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .ok();
    let new_root = new_doc
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .ok();
    if old_root != new_root {
        out.add(
            ChangeLevel::Other,
            "Văn bản được trỏ sang Catalog khác sau khi ký.",
        );
    }
    if out.level == ChangeLevel::None {
        // Có bản cập nhật nhưng không đổi đối tượng nào.
        out.add(
            ChangeLevel::Signatures,
            "Có bản cập nhật không làm thay đổi nội dung.",
        );
    }
    Ok(out)
}
