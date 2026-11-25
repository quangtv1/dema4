//! Spike S3: khối chữ tiếng Việt trong Form XObject, nhúng subset font TrueType dạng
//! Type0 / CIDFontType2 / Identity-H, có /W và /ToUnicode.
//!
//! Tạo một trang A4 có một khối chữ cho mỗi font ở Phụ lục A.5, rồi in báo cáo:
//! kích thước subset, số glyph, ký tự không có glyph, kích thước khối đo được.
//!
//! Cách dùng: s3-font <file PDF kết quả> <file văn bản mong đợi>

use std::collections::BTreeMap;
use std::error::Error;

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Content, Filter, Name, Pdf, Rect, Ref, Str};
use subsetter::GlyphRemapper;
use ttf_parser::{Face, GlyphId, name_id};
use unicode_normalization::UnicodeNormalization;

const FONTS: [(&str, &str); 5] = [
    ("Times New Roman", "times.ttf"),
    ("Arial", "arial.ttf"),
    ("Verdana", "verdana.ttf"),
    ("Georgia", "georgia.ttf"),
    ("Courier New", "cour.ttf"),
];

const FONT_SIZE: f32 = 10.0;
const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;

/// Các dòng thử: tên có dấu, đủ bộ nguyên âm có dấu của tiếng Việt, và một dòng nhập ở dạng NFD.
fn sample_lines(font_label: &str) -> Vec<String> {
    let nfd: String = "Trần Quốc Toản, Đỗ Thị Hường".nfd().collect();
    vec![
        format!("Ký bởi: Nguyễn Thị Ánh Tuyết ({font_label})"),
        "Ngày ký: 18/11/2026 14:05".to_owned(),
        "ĂẮẰẲẴẶ ÂẤẦẨẪẬ ÊẾỀỂỄỆ ÔỐỒỔỖỘ ƠỚỜỞỠỢ ƯỨỪỬỮỰ ĐÁÀẢÃẠ ÍÌỈĨỊ ÝỲỶỸỴ".to_owned(),
        "ăắằẳẵặ âấầẩẫậ êếềểễệ ôốồổỗộ ơớờởỡợ ưứừửữự đáàảãạ íìỉĩị ýỳỷỹỵ".to_owned(),
        format!("Nhập dạng NFD: {nfd}"),
    ]
}

struct Block {
    form_ref: Ref,
    width: f32,
    height: f32,
}

struct Report {
    font: String,
    original_bytes: usize,
    subset_bytes: usize,
    compressed_bytes: usize,
    glyphs: u16,
    missing: Vec<char>,
    width: f32,
    height: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Cách dùng: s3-font <out.pdf> <expected.txt>".into());
    }

    let mut alloc = Ref::new(1);
    let catalog_ref = alloc.bump();
    let pages_ref = alloc.bump();
    let page_ref = alloc.bump();
    let page_content_ref = alloc.bump();

    let mut pdf = Pdf::new();
    let mut blocks = Vec::new();
    let mut reports = Vec::new();
    let mut expected = String::new();

    for (index, (label, file)) in FONTS.iter().enumerate() {
        let path = format!(r"C:\Windows\Fonts\{file}");
        let data = std::fs::read(&path).map_err(|e| format!("Không đọc được {path}: {e}"))?;
        let lines: Vec<String> =
            sample_lines(label).iter().map(|line| line.nfc().collect()).collect();
        for line in &lines {
            expected.push_str(line);
            expected.push('\n');
        }
        let (block, report) = write_block(&mut pdf, &mut alloc, index, label, &data, &lines)?;
        blocks.push(block);
        reports.push(report);
    }

    // Trang: đặt các khối từ trên xuống, kẻ khung theo kích thước đo được để soi độ khớp.
    let mut content = Content::new();
    let mut top = PAGE_HEIGHT - 40.0;
    for (index, block) in blocks.iter().enumerate() {
        let x = 40.0;
        let y = top - block.height;
        let name = format!("Fm{index}");
        content.save_state();
        content.transform([1.0, 0.0, 0.0, 1.0, x, y]);
        content.x_object(Name(name.as_bytes()));
        content.restore_state();
        content.save_state();
        content.set_line_width(0.3);
        content.set_stroke_rgb(0.8, 0.1, 0.1);
        content.rect(x, y, block.width, block.height);
        content.stroke();
        content.restore_state();
        top = y - 24.0;
    }
    let content = content.finish();

    pdf.catalog(catalog_ref).pages(pages_ref);
    pdf.pages(pages_ref).kids([page_ref]).count(1);
    {
        let mut page = pdf.page(page_ref);
        page.parent(pages_ref);
        page.media_box(Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT));
        page.contents(page_content_ref);
        let mut resources = page.resources();
        let mut x_objects = resources.x_objects();
        for (index, block) in blocks.iter().enumerate() {
            let name = format!("Fm{index}");
            x_objects.pair(Name(name.as_bytes()), block.form_ref);
        }
    }
    pdf.stream(page_content_ref, content.as_slice());

    std::fs::write(&args[0], pdf.finish())?;
    std::fs::write(&args[1], expected)?;

    println!(
        "{:<16} {:>9} {:>8} {:>8} {:>6} {:>8} {:>7}  thiếu glyph",
        "font", "gốc", "subset", "nén", "glyph", "rộng", "cao"
    );
    for r in &reports {
        println!(
            "{:<16} {:>9} {:>8} {:>8} {:>6} {:>8.2} {:>7.2}  {}",
            r.font,
            r.original_bytes,
            r.subset_bytes,
            r.compressed_bytes,
            r.glyphs,
            r.width,
            r.height,
            if r.missing.is_empty() {
                "không".to_owned()
            } else {
                r.missing.iter().collect::<String>()
            }
        );
    }
    println!("Đã ghi {} và {}", args[0], args[1]);
    Ok(())
}

fn write_block(
    pdf: &mut Pdf,
    alloc: &mut Ref,
    index: usize,
    label: &str,
    data: &[u8],
    lines: &[String],
) -> Result<(Block, Report), Box<dyn Error>> {
    let face = Face::parse(data, 0)?;
    let units = f32::from(face.units_per_em());
    let to_pt = |value: f32| value * FONT_SIZE / units;

    // Ký tự -> glyph gốc -> glyph mới (đánh số lại liền nhau trong subset).
    let mut remapper = GlyphRemapper::new();
    let mut to_unicode: BTreeMap<u16, char> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut encoded_lines = Vec::new();
    let mut max_width = 0.0f32;
    for line in lines {
        let mut bytes = Vec::with_capacity(line.len() * 2);
        let mut advance = 0.0f32;
        for ch in line.chars() {
            let old = face.glyph_index(ch).unwrap_or(GlyphId(0));
            if old.0 == 0 && !missing.contains(&ch) {
                missing.push(ch);
            }
            let new = remapper.remap(old.0);
            to_unicode.entry(new).or_insert(ch);
            bytes.extend_from_slice(&new.to_be_bytes());
            advance += f32::from(face.glyph_hor_advance(old).unwrap_or(0));
        }
        max_width = max_width.max(to_pt(advance));
        encoded_lines.push(bytes);
    }

    let subset = subsetter::subset(data, 0, &remapper)?;
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&subset, 6);

    // Kích thước khối: dòng dài nhất x (số dòng x chiều cao dòng), như cách bản cũ đo.
    let ascent = f32::from(face.ascender());
    let descent = f32::from(face.descender());
    let line_height = to_pt(ascent - descent + f32::from(face.line_gap()));
    let width = max_width;
    let height = line_height * lines.len() as f32;

    let type0_ref = alloc.bump();
    let cid_ref = alloc.bump();
    let descriptor_ref = alloc.bump();
    let font_file_ref = alloc.bump();
    let cmap_ref = alloc.bump();
    let form_ref = alloc.bump();

    let post_script = face
        .names()
        .into_iter()
        .find(|n| n.name_id == name_id::POST_SCRIPT_NAME && n.is_unicode())
        .and_then(|n| n.to_string())
        .unwrap_or_else(|| label.replace(' ', ""));
    // Tiền tố subset: 6 chữ hoa. Ở spike đặt cố định theo thứ tự font.
    let tag = format!("SMKSA{}", char::from(b'A' + index as u8));
    let base_font = format!("{tag}+{post_script}");
    let base_font = Name(base_font.as_bytes());
    let system_info =
        SystemInfo { registry: Str(b"Adobe"), ordering: Str(b"Identity"), supplement: 0 };

    pdf.type0_font(type0_ref)
        .base_font(base_font)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid_ref)
        .to_unicode(cmap_ref);

    // Bề rộng từng glyph theo thứ tự glyph mới, đơn vị 1/1000 em.
    let widths: Vec<f32> = remapper
        .remapped_gids()
        .map(|old| f32::from(face.glyph_hor_advance(GlyphId(old)).unwrap_or(0)) * 1000.0 / units)
        .collect();
    {
        let mut cid = pdf.cid_font(cid_ref);
        cid.subtype(CidFontType::Type2);
        cid.base_font(base_font);
        cid.system_info(system_info);
        cid.font_descriptor(descriptor_ref);
        cid.default_width(0.0);
        cid.widths().consecutive(0, widths);
        cid.cid_to_gid_map_predefined(Name(b"Identity"));
    }

    let mut flags = FontFlags::SYMBOLIC;
    if face.is_monospaced() {
        flags |= FontFlags::FIXED_PITCH;
    }
    if face.is_italic() {
        flags |= FontFlags::ITALIC;
    }
    if matches!(label, "Times New Roman" | "Georgia" | "Courier New") {
        flags |= FontFlags::SERIF;
    }
    let bbox = face.global_bounding_box();
    let scale = 1000.0 / units;
    pdf.font_descriptor(descriptor_ref)
        .name(base_font)
        .flags(flags)
        .bbox(Rect::new(
            f32::from(bbox.x_min) * scale,
            f32::from(bbox.y_min) * scale,
            f32::from(bbox.x_max) * scale,
            f32::from(bbox.y_max) * scale,
        ))
        .italic_angle(face.italic_angle())
        .ascent(ascent * scale)
        .descent(descent * scale)
        .cap_height(f32::from(face.capital_height().unwrap_or(face.ascender())) * scale)
        .stem_v(10.0 + 0.244 * (f32::from(face.weight().to_number()) - 50.0))
        .font_file2(font_file_ref);

    pdf.stream(font_file_ref, &compressed)
        .filter(Filter::FlateDecode)
        .pair(Name(b"Length1"), subset.len() as i32);

    let mut cmap = UnicodeCmap::new(Name(b"Custom"), system_info);
    for (glyph, ch) in &to_unicode {
        cmap.pair(*glyph, *ch);
    }
    pdf.cmap(cmap_ref, cmap.finish().as_slice());

    // Nội dung khối: mỗi dòng một lệnh Tj, đường gốc tính từ mép trên của khối.
    let mut content = Content::new();
    content.save_state();
    content.set_fill_rgb(0.0, 0.0, 0.0);
    content.begin_text();
    content.set_font(Name(b"F1"), FONT_SIZE);
    let mut baseline = height - to_pt(ascent);
    for bytes in &encoded_lines {
        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, 0.0, baseline]);
        content.show(Str(bytes));
        baseline -= line_height;
    }
    content.end_text();
    content.restore_state();
    let content = content.finish();
    {
        let mut form = pdf.form_xobject(form_ref, content.as_slice());
        form.bbox(Rect::new(0.0, 0.0, width, height));
        form.resources().fonts().pair(Name(b"F1"), type0_ref);
    }

    Ok((
        Block { form_ref, width, height },
        Report {
            font: label.to_owned(),
            original_bytes: data.len(),
            subset_bytes: subset.len(),
            compressed_bytes: compressed.len(),
            glyphs: remapper.num_gids(),
            missing,
            width,
            height,
        },
    ))
}
