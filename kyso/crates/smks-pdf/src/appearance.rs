//! Khối hiển thị của chữ ký và ảnh dấu (kế hoạch 7.3; port `PdfAppearanceBuilder.cs`).
//!
//! Khác bản cũ: không "cấy" nội dung từ một PDF tạm do PDFsharp vẽ, mà tự dựng Form XObject:
//! - **Chữ:** font TrueType của Windows nhúng subset dạng Type0 / CIDFontType2 / Identity-H, có `/W` và
//!   `/ToUnicode` (copy chữ ra được); chuỗi chuẩn hoá NFC. Font thiếu glyph tiếng Việt (Georgia) thì dùng
//!   Times New Roman cho cả khối (TD-33), để không in ra ô vuông.
//! - **Ảnh:** PNG/JPG giải mã rồi áp độ đậm và màu mực lên điểm ảnh (cùng công thức với `/Decode` của bản cũ),
//!   kênh alpha thành `/SMask`, nén Flate, thu nhỏ theo khung (không quá 300 dpi). Độ dày nét: vẽ ảnh 9 lần
//!   lệch theo 8 hướng quanh lõi, như bản cũ.
//! - Trang xoay: `/Matrix` của Form XObject.

use smks_core::config::{ImageLayout, ImageSplit};
use std::collections::BTreeMap;

use image::GenericImageView;
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Content, Filter, Name, Rect as PdfRect, Ref, Str};
use smks_core::consts::{appearance as ap, stamp_image as si};
use smks_core::{ErrorCode, Result, SmksError};
use subsetter::GlyphRemapper;
use ttf_parser::{Face, GlyphId, name_id};
use unicode_normalization::UnicodeNormalization;

use crate::geometry::rotation_matrix;

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignFailed, message)
}

/// Cấp số object mới.
#[derive(Debug, Clone, Copy)]
pub struct Alloc(pub u32);

impl Alloc {
    pub fn bump(&mut self) -> u32 {
        let n = self.0;
        self.0 += 1;
        n
    }
}

/// Object mới đã ghi sẵn (`N 0 obj … endobj`).
#[derive(Debug, Clone)]
pub struct NewObject {
    pub number: u32,
    pub bytes: Vec<u8>,
}

fn single(number: u32, write: impl FnOnce(&mut Chunk, Ref)) -> NewObject {
    let mut chunk = Chunk::new();
    write(&mut chunk, Ref::new(number as i32));
    let mut bytes = chunk.as_bytes().to_vec();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    NewObject { number, bytes }
}

/// Form XObject đã dựng cùng mọi object nó dùng.
#[derive(Debug, Clone)]
pub struct Built {
    pub form: u32,
    pub objects: Vec<NewObject>,
}

// ----------------------------------------------------------------------------------------------
// Font
// ----------------------------------------------------------------------------------------------

/// File font Windows của 5 font bản cũ cho chọn (Phụ lục A.5).
fn font_file(family: &str) -> Option<&'static str> {
    Some(match family {
        "Times New Roman" => "times.ttf",
        "Arial" => "arial.ttf",
        "Verdana" => "verdana.ttf",
        "Georgia" => "georgia.ttf",
        "Courier New" => "cour.ttf",
        _ => return None,
    })
}

fn fonts_dir() -> std::path::PathBuf {
    let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
    std::path::PathBuf::from(windir).join("Fonts")
}

/// Font đã nạp.
#[derive(Debug, Clone)]
pub struct FontData {
    pub family: String,
    data: Vec<u8>,
}

impl FontData {
    pub fn load(family: &str) -> Result<Self> {
        let file = font_file(family)
            .ok_or_else(|| fail(format!("Font {family} không có trong danh sách hỗ trợ.")))?;
        let path = fonts_dir().join(file);
        let data = std::fs::read(&path)
            .map_err(|e| fail(format!("Không đọc được font {}: {e}", path.display())))?;
        Face::parse(&data, 0).map_err(|e| fail(format!("Font {family} không hợp lệ: {e}")))?;
        Ok(Self {
            family: family.to_owned(),
            data,
        })
    }

    fn face(&self) -> Result<Face<'_>> {
        Face::parse(&self.data, 0)
            .map_err(|e| fail(format!("Font {} không hợp lệ: {e}", self.family)))
    }

    /// Font có đủ glyph cho mọi ký tự (trừ khoảng trắng điều khiển) của `lines`.
    pub fn covers(&self, lines: &[String]) -> bool {
        let Ok(face) = self.face() else { return false };
        lines
            .iter()
            .flat_map(|l| l.chars())
            .all(|c| c.is_control() || face.glyph_index(c).is_some())
    }

    /// Chọn font theo tên; thiếu glyph thì đổi sang Times New Roman (TD-33). Trả kèm cờ đã đổi font.
    pub fn for_lines(family: &str, lines: &[String]) -> Result<(Self, bool)> {
        let font = Self::load(family)?;
        if font.covers(lines) || family == ap::DEFAULT_FONT_FAMILY {
            return Ok((font, false));
        }
        Ok((Self::load(ap::DEFAULT_FONT_FAMILY)?, true))
    }

    /// Đo khối chữ: bề rộng dòng dài nhất, chiều cao một dòng (`ascender − descender + lineGap` của `hhea`).
    pub fn measure(&self, lines: &[String], size: f64) -> Result<(f64, f64)> {
        let face = self.face()?;
        let units = f64::from(face.units_per_em());
        let mut width = 0.0f64;
        for line in lines {
            let advance: f64 = line
                .nfc()
                .map(|c| {
                    f64::from(
                        face.glyph_index(c)
                            .and_then(|g| face.glyph_hor_advance(g))
                            .unwrap_or(0),
                    )
                })
                .sum();
            width = width.max(advance * size / units);
        }
        let line_height = f64::from(
            i32::from(face.ascender()) - i32::from(face.descender()) + i32::from(face.line_gap()),
        ) * size
            / units;
        Ok((width, line_height))
    }
}

/// Nhúng subset font cho `lines` và trả nội dung đã mã hoá theo glyph mới.
struct EmbeddedFont {
    type0: u32,
    objects: Vec<NewObject>,
    /// Mỗi dòng: chuỗi 2 byte / glyph.
    encoded: Vec<Vec<u8>>,
    ascent: f64,
    line_height: f64,
    widths: Vec<f64>,
}

fn embed_font(
    font: &FontData,
    lines: &[String],
    size: f64,
    alloc: &mut Alloc,
) -> Result<EmbeddedFont> {
    let face = font.face()?;
    let units = f64::from(face.units_per_em());
    let mut remapper = GlyphRemapper::new();
    let mut to_unicode: BTreeMap<u16, char> = BTreeMap::new();
    let mut encoded = Vec::new();
    let mut widths = Vec::new();
    for line in lines {
        let mut bytes = Vec::new();
        let mut advance = 0.0f64;
        for ch in line.nfc() {
            let old = face.glyph_index(ch).unwrap_or(GlyphId(0));
            let new = remapper.remap(old.0);
            to_unicode.entry(new).or_insert(ch);
            bytes.extend_from_slice(&new.to_be_bytes());
            advance += f64::from(face.glyph_hor_advance(old).unwrap_or(0));
        }
        widths.push(advance * size / units);
        encoded.push(bytes);
    }
    let subset = subsetter::subset(&font.data, 0, &remapper)
        .map_err(|e| fail(format!("Không tạo được subset font: {e:?}")))?;
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&subset, 6);

    let type0 = alloc.bump();
    let cid = alloc.bump();
    let descriptor = alloc.bump();
    let file = alloc.bump();
    let cmap_n = alloc.bump();

    let post_script = face
        .names()
        .into_iter()
        .find(|n| n.name_id == name_id::POST_SCRIPT_NAME && n.is_unicode())
        .and_then(|n| n.to_string())
        .unwrap_or_else(|| font.family.replace(' ', ""));
    // Tiền tố subset 6 chữ hoa, suy từ số object để khác nhau giữa các khối.
    let tag: String = (0..6)
        .map(|i| char::from(b'A' + ((type0 / 26u32.pow(i)) % 26) as u8))
        .collect();
    let base_font = format!("{tag}+{post_script}");
    let system_info = SystemInfo {
        registry: Str(b"Adobe"),
        ordering: Str(b"Identity"),
        supplement: 0,
    };
    let scale = 1000.0 / units as f32;
    let glyph_widths: Vec<f32> = remapper
        .remapped_gids()
        .map(|old| f32::from(face.glyph_hor_advance(GlyphId(old)).unwrap_or(0)) * scale)
        .collect();
    let mut flags = FontFlags::SYMBOLIC;
    if face.is_monospaced() {
        flags |= FontFlags::FIXED_PITCH;
    }
    if face.is_italic() {
        flags |= FontFlags::ITALIC;
    }
    if matches!(
        font.family.as_str(),
        "Times New Roman" | "Georgia" | "Courier New"
    ) {
        flags |= FontFlags::SERIF;
    }
    let bbox = face.global_bounding_box();
    let ascent = f32::from(face.ascender());
    let descent = f32::from(face.descender());

    let mut objects = vec![
        single(type0, |c, r| {
            c.type0_font(r)
                .base_font(Name(base_font.as_bytes()))
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(Ref::new(cid as i32))
                .to_unicode(Ref::new(cmap_n as i32));
        }),
        single(cid, |c, r| {
            let mut f = c.cid_font(r);
            f.subtype(CidFontType::Type2);
            f.base_font(Name(base_font.as_bytes()));
            f.system_info(system_info);
            f.font_descriptor(Ref::new(descriptor as i32));
            f.default_width(0.0);
            f.widths().consecutive(0, glyph_widths.iter().copied());
            f.cid_to_gid_map_predefined(Name(b"Identity"));
        }),
        single(descriptor, |c, r| {
            c.font_descriptor(r)
                .name(Name(base_font.as_bytes()))
                .flags(flags)
                .bbox(PdfRect::new(
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
                .font_file2(Ref::new(file as i32));
        }),
        single(file, |c, r| {
            c.stream(r, &compressed)
                .filter(Filter::FlateDecode)
                .pair(Name(b"Length1"), subset.len() as i32);
        }),
    ];
    let mut cmap = UnicodeCmap::new(Name(b"Custom"), system_info);
    for (glyph, ch) in &to_unicode {
        cmap.pair(*glyph, *ch);
    }
    let cmap_bytes = cmap.finish();
    objects.push(single(cmap_n, |c, r| {
        c.cmap(r, cmap_bytes.as_slice());
    }));
    Ok(EmbeddedFont {
        type0,
        objects,
        encoded,
        ascent: f64::from(ascent) * size / units,
        line_height: f64::from(ascent - descent + f32::from(face.line_gap())) * size / units,
        widths,
    })
}

// ----------------------------------------------------------------------------------------------
// Ảnh
// ----------------------------------------------------------------------------------------------

/// Ảnh dấu / chữ ký tươi đã xử lý điểm ảnh.
#[derive(Debug, Clone)]
pub struct StampImage {
    pub width: u32,
    pub height: u32,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

/// Màu `#RRGGBB` → (r, g, b) trong [0, 1].
pub fn parse_color(text: &str) -> Option<[f64; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some([
        f64::from((v >> 16) & 0xFF) / 255.0,
        f64::from((v >> 8) & 0xFF) / 255.0,
        f64::from(v & 0xFF) / 255.0,
    ])
}

impl StampImage {
    /// Giải mã PNG/JPG, thu nhỏ để cạnh dài không quá `max_px`, áp độ đậm (40–250, 100 là giữ nguyên) và màu
    /// mực (`None` là giữ màu ảnh). Công thức như `ApplyDarknessAndColor` của bản cũ:
    /// `ra = mực + (1 − mực) · (thấp + giá trị · (cao − thấp))`.
    pub fn prepare(
        bytes: &[u8],
        darkness: i32,
        ink: Option<[f64; 3]>,
        max_px: u32,
    ) -> Result<Self> {
        if bytes.len() as u64 > si::MAX_BYTES {
            return Err(SmksError::new(
                ErrorCode::StampImageInvalid,
                "Ảnh dấu hoặc chữ ký tươi lớn hơn 5MB.",
            ));
        }
        let img = image::load_from_memory(bytes).map_err(|e| {
            SmksError::new(
                ErrorCode::StampImageInvalid,
                format!("Không đọc được ảnh (chỉ nhận PNG, JPG): {e}"),
            )
        })?;
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err(SmksError::new(ErrorCode::StampImageInvalid, "Ảnh rỗng."));
        }
        let img = if w.max(h) > max_px.max(16) {
            img.resize(
                max_px.max(16),
                max_px.max(16),
                image::imageops::FilterType::Triangle,
            )
        } else {
            img
        };
        let has_alpha = img.color().has_alpha();
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let darkness = darkness.clamp(si::DARKNESS_MIN, si::DARKNESS_MAX);
        let factor = f64::from(darkness) / 100.0;
        let adjust = (factor - 1.0).abs() > 0.001 || ink.is_some();
        let brightness = (1.0 - (factor - 1.0) * si::DARKNESS_BRIGHTNESS_FACTOR)
            .max(si::DARKNESS_MIN_BRIGHTNESS);
        let low = brightness * (1.0 - factor) / 2.0;
        let high = low + brightness * factor;
        let ink = ink.unwrap_or([0.0; 3]);
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        let mut alpha = Vec::with_capacity((width * height) as usize);
        for px in rgba.pixels() {
            for (i, channel) in px.0[..3].iter().enumerate() {
                let value = if adjust {
                    let v = f64::from(*channel) / 255.0;
                    let level = low + v * (high - low);
                    ((ink[i] + (1.0 - ink[i]) * level).clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    *channel
                };
                rgb.push(value);
            }
            alpha.push(px.0[3]);
        }
        Ok(Self {
            width,
            height,
            rgb,
            alpha: has_alpha.then_some(alpha),
        })
    }

    fn write(&self, alloc: &mut Alloc) -> (u32, Vec<NewObject>) {
        let image = alloc.bump();
        let mut objects = Vec::new();
        let mask = self.alpha.as_ref().map(|alpha| {
            let n = alloc.bump();
            let data = miniz_oxide::deflate::compress_to_vec_zlib(alpha, 6);
            let (w, h) = (self.width as i32, self.height as i32);
            objects.push(single(n, |c, r| {
                let mut x = c.image_xobject(r, &data);
                x.filter(Filter::FlateDecode);
                x.width(w).height(h).bits_per_component(8);
                x.color_space().device_gray();
            }));
            n
        });
        let data = miniz_oxide::deflate::compress_to_vec_zlib(&self.rgb, 6);
        let (w, h) = (self.width as i32, self.height as i32);
        objects.insert(
            0,
            single(image, |c, r| {
                let mut x = c.image_xobject(r, &data);
                x.filter(Filter::FlateDecode);
                x.width(w).height(h).bits_per_component(8);
                x.color_space().device_rgb();
                if let Some(m) = mask {
                    x.s_mask(Ref::new(m as i32));
                }
            }),
        );
        (image, objects)
    }
}

/// Số điểm ảnh tối đa theo khung: 300 dpi, không quá 1600.
pub fn max_pixels_for(width_pt: f64, height_pt: f64) -> u32 {
    ((width_pt.max(height_pt) * 300.0 / 72.0).ceil() as u32).clamp(64, 1600)
}

/// Vẽ ảnh vừa khung `(x, y, w, h)`, giữ tỉ lệ, căn giữa; `stroke` (0–200) làm đậm nét như bản cũ.
fn draw_image(
    content: &mut Content,
    name: &[u8],
    image: &StampImage,
    (x, y, w, h): (f64, f64, f64, f64),
    stroke: i32,
) {
    let iw = f64::from(image.width);
    let ih = f64::from(image.height);
    let scale = (w / iw).min(h / ih);
    let (dw, dh) = (iw * scale, ih * scale);
    let (bx, by) = (x + (w - dw) / 2.0, y + (h - dh) / 2.0);
    let radius =
        dw * si::STROKE_RADIUS_RATIO * f64::from(stroke.clamp(si::STROKE_MIN, si::STROKE_MAX))
            / 100.0;
    let mut layers = Vec::new();
    if radius > 0.0 && dw > radius * 2.0 && dh > radius * 2.0 {
        let (cx, cy, cw, ch) = (
            bx + radius,
            by + radius,
            dw - 2.0 * radius,
            dh - 2.0 * radius,
        );
        for i in 0..si::STROKE_DIRECTIONS {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / si::STROKE_DIRECTIONS as f64;
            layers.push((cx + radius * angle.cos(), cy + radius * angle.sin(), cw, ch));
        }
        layers.push((cx, cy, cw, ch));
    } else {
        layers.push((bx, by, dw, dh));
    }
    for (lx, ly, lw, lh) in layers {
        content.save_state();
        content.transform([lw as f32, 0.0, 0.0, lh as f32, lx as f32, ly as f32]);
        content.x_object(Name(name));
        content.restore_state();
    }
}

// ----------------------------------------------------------------------------------------------
// Khối hoàn chỉnh
// ----------------------------------------------------------------------------------------------

/// Phần chữ của khối.
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font: FontData,
    pub size: f64,
    pub color: [f64; 3],
}

fn write_form(
    number: u32,
    width: f64,
    height: f64,
    rotation: i32,
    content: &[u8],
    fonts: &[(Vec<u8>, u32)],
    images: &[(Vec<u8>, u32)],
) -> NewObject {
    single(number, |c, r| {
        let mut form = c.form_xobject(r, content);
        form.bbox(PdfRect::new(0.0, 0.0, width as f32, height as f32));
        if let Some(m) = rotation_matrix(rotation) {
            form.matrix(m.map(|v| v as f32));
        }
        let mut res = form.resources();
        if !fonts.is_empty() {
            let mut f = res.fonts();
            for (name, n) in fonts {
                f.pair(Name(name), Ref::new(*n as i32));
            }
        }
        if !images.is_empty() {
            let mut x = res.x_objects();
            for (name, n) in images {
                x.pair(Name(name), Ref::new(*n as i32));
            }
        }
    })
}

fn text_lines(
    content: &mut Content,
    font: &EmbeddedFont,
    size: f64,
    color: [f64; 3],
    x: f64,
    top: f64,
) {
    content.save_state();
    content.set_fill_rgb(color[0] as f32, color[1] as f32, color[2] as f32);
    content.begin_text();
    content.set_font(Name(b"F1"), size as f32);
    let mut baseline = top - font.ascent;
    for bytes in &font.encoded {
        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, x as f32, baseline as f32]);
        content.show(Str(bytes));
        baseline -= font.line_height;
    }
    content.end_text();
    content.restore_state();
}

/// Khối chỉ có chữ, kích thước `width × height`. Chữ tràn bề rộng thì tự giảm cỡ.
pub fn build_text(
    alloc: &mut Alloc,
    width: f64,
    height: f64,
    rotation: i32,
    lines: &[String],
    style: &TextStyle,
) -> Result<Built> {
    let (measured, line_height) = style.font.measure(lines, style.size)?;
    let fit_w = if measured > width && measured > 0.0 {
        width / measured
    } else {
        1.0
    };
    let fit_h = if line_height * lines.len() as f64 > height && line_height > 0.0 {
        height / (line_height * lines.len() as f64)
    } else {
        1.0
    };
    let size = (style.size * fit_w.min(fit_h)).max(1.0);
    let form = alloc.bump();
    let font = embed_font(&style.font, lines, size, alloc)?;
    let mut content = Content::new();
    text_lines(&mut content, &font, size, style.color, 0.0, height);
    let content = content.finish();
    let mut objects = vec![write_form(
        form,
        width,
        height,
        rotation,
        content.as_slice(),
        &[(b"F1".to_vec(), font.type0)],
        &[],
    )];
    objects.extend(font.objects);
    Ok(Built { form, objects })
}

/// Khối chỉ có ảnh (ảnh vừa khung, giữ tỉ lệ).
pub fn build_image(
    alloc: &mut Alloc,
    width: f64,
    height: f64,
    rotation: i32,
    image: &StampImage,
    stroke: i32,
) -> Built {
    let form = alloc.bump();
    let (image_n, image_objects) = image.write(alloc);
    let mut content = Content::new();
    draw_image(
        &mut content,
        b"Im1",
        image,
        (0.0, 0.0, width, height),
        stroke,
    );
    let content = content.finish();
    let mut objects = vec![write_form(
        form,
        width,
        height,
        rotation,
        content.as_slice(),
        &[],
        &[(b"Im1".to_vec(), image_n)],
    )];
    objects.extend(image_objects);
    Built { form, objects }
}

/// Phần chữ được chiếm tối đa bao nhiêu chiều cao khối khi ảnh nằm trên / dưới chữ.
const STACKED_TEXT_SHARE: f64 = 0.5;

/// Chỗ của ảnh và chữ trong khối `width × height` (gốc dưới trái, đơn vị pt).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageTextBoxes {
    /// Ô đặt ảnh `(x, y, w, h)`; ảnh vừa ô, giữ tỉ lệ, căn giữa.
    pub image: (f64, f64, f64, f64),
    /// Cỡ chữ sau khi thu cho vừa.
    pub size: f64,
    /// Lề trái của khối chữ và đường trên của dòng đầu.
    pub text_x: f64,
    pub text_top: f64,
}

/// Chia khối "Ảnh + thông tin" theo `layout`. Tự động (`split` để trống): trái / phải thì ảnh rộng tối đa 45% khối,
/// chữ căn giữa theo chiều dọc (như bản trước); trên / dưới thì chữ chiếm tối đa một nửa chiều cao, khối chữ căn
/// giữa theo chiều ngang, ảnh lấy phần còn lại. `split.share` đặt phần dành cho ảnh, `split.gap` đặt khoảng cách ảnh
/// – chữ (% khối theo chiều xếp, TD-51); ảnh vừa ô của nó, giữ tỉ lệ. Chữ không vừa thì thu nhỏ.
pub fn image_text_boxes(
    (width, height): (f64, f64),
    aspect: f64,
    (measured, line_height, lines): (f64, f64, usize),
    base_size: f64,
    layout: ImageLayout,
    split: ImageSplit,
) -> ImageTextBoxes {
    let shrink = |avail_w: f64, avail_h: f64| {
        let total_h = line_height * lines as f64;
        let fw = if measured > avail_w && measured > 0.0 {
            avail_w / measured
        } else {
            1.0
        };
        let fh = if total_h > avail_h && total_h > 0.0 {
            avail_h / total_h
        } else {
            1.0
        };
        fw.min(fh)
    };
    if let Some((image, text)) = split.free {
        // Tự do: ảnh vừa ô ảnh; chữ căn trái, giữa theo chiều dọc trong ô chữ, thu nhỏ cho vừa ô.
        let to_pt = |b: smks_core::config::PartBox| {
            let b = b.clean().unwrap_or_default();
            (
                b.x * width,
                height - (b.y + b.h) * height,
                b.w * width,
                b.h * height,
            )
        };
        let image = to_pt(image);
        let (tx, ty, tw, th) = to_pt(text);
        let fit = shrink(tw, th);
        let text_h = line_height * fit * lines as f64;
        return ImageTextBoxes {
            image,
            size: (base_size * fit).max(1.0),
            text_x: tx,
            text_top: ty + th - (th - text_h).max(0.0) / 2.0,
        };
    }
    let share = split.share_ratio();
    if layout.is_stacked() {
        let gap = split
            .gap_ratio()
            .map_or((height * 0.04).min(3.0), |g| height * g);
        let (image_h, text_area) = match share {
            Some(r) => {
                let image_h = (height * r).max(1.0);
                (image_h, (height - image_h - gap).max(1.0))
            }
            None => (0.0, height * STACKED_TEXT_SHARE),
        };
        let fit = shrink(width, text_area);
        let text_h = line_height * fit * lines as f64;
        let text_w = measured * fit;
        let image_h = if share.is_some() {
            image_h
        } else {
            (height - text_h - gap).max(1.0)
        };
        let text_x = ((width - text_w) / 2.0).max(0.0);
        let (image_y, text_top) = if layout == ImageLayout::Top {
            (height - image_h, (height - image_h - gap).max(text_h))
        } else {
            (0.0, height)
        };
        return ImageTextBoxes {
            image: (0.0, image_y, width, image_h),
            size: (base_size * fit).max(1.0),
            text_x,
            text_top,
        };
    }
    let image_w = share.map_or((height * aspect).min(width * 0.45), |r| width * r);
    let gap = split
        .gap_ratio()
        .map_or((width * 0.03).min(6.0), |g| width * g);
    let text_w = (width - image_w - gap).max(1.0);
    let fit = shrink(text_w, height);
    let text_h = line_height * fit * lines as f64;
    let top = height - (height - text_h).max(0.0) / 2.0;
    let (image_x, text_x) = if layout == ImageLayout::Right {
        (width - image_w, 0.0)
    } else {
        (0.0, image_w + gap)
    };
    ImageTextBoxes {
        image: (image_x, 0.0, image_w, height),
        size: (base_size * fit).max(1.0),
        text_x,
        text_top: top,
    }
}

/// Khối "Ảnh + thông tin": ảnh trái / phải / trên / dưới chữ theo `layout` (xem [`image_text_boxes`]).
pub fn build_image_and_text(
    alloc: &mut Alloc,
    (width, height): (f64, f64),
    rotation: i32,
    (image, stroke): (&StampImage, i32),
    lines: &[String],
    style: &TextStyle,
    (layout, split): (ImageLayout, ImageSplit),
) -> Result<Built> {
    let aspect = f64::from(image.width) / f64::from(image.height);
    let (measured, line_height) = style.font.measure(lines, style.size)?;
    let boxes = image_text_boxes(
        (width, height),
        aspect,
        (measured, line_height, lines.len()),
        style.size,
        layout,
        split,
    );
    let size = boxes.size;
    let form = alloc.bump();
    let (image_n, image_objects) = image.write(alloc);
    let font = embed_font(&style.font, lines, size, alloc)?;
    let mut content = Content::new();
    draw_image(&mut content, b"Im1", image, boxes.image, stroke);
    text_lines(
        &mut content,
        &font,
        size,
        style.color,
        boxes.text_x,
        boxes.text_top,
    );
    let content = content.finish();
    let mut objects = vec![write_form(
        form,
        width,
        height,
        rotation,
        content.as_slice(),
        &[(b"F1".to_vec(), font.type0)],
        &[(b"Im1".to_vec(), image_n)],
    )];
    objects.extend(image_objects);
    objects.extend(font.objects);
    let _ = font.widths;
    Ok(Built { form, objects })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chia khối 170 × 30 pt (trái / phải) và 130 × 66 pt (trên / dưới) với ảnh tỉ lệ 2:1, chữ rộng 80 pt, 2 dòng
    /// cao 11,5 pt, cỡ 10.
    #[test]
    fn image_text_layouts() {
        let text = (80.0, 11.5, 2);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
        let left = image_text_boxes(
            (170.0, 30.0),
            2.0,
            text,
            10.0,
            ImageLayout::Left,
            ImageSplit::default(),
        );
        assert_eq!(
            left.image,
            (0.0, 0.0, 60.0, 30.0),
            "ảnh trái, rộng min(30 × 2, 45%)"
        );
        assert!(
            close(left.text_x, 65.1) && close(left.size, 10.0),
            "{left:?}"
        );
        assert!(close(left.text_top, 30.0 - (30.0 - 23.0) / 2.0));
        let right = image_text_boxes(
            (170.0, 30.0),
            2.0,
            text,
            10.0,
            ImageLayout::Right,
            ImageSplit::default(),
        );
        assert_eq!(right.image, (110.0, 0.0, 60.0, 30.0), "ảnh sát mép phải");
        assert!(close(right.text_x, 0.0) && close(right.text_top, left.text_top));
        let top = image_text_boxes(
            (130.0, 66.0),
            2.0,
            text,
            10.0,
            ImageLayout::Top,
            ImageSplit::default(),
        );
        let gap = (66.0_f64 * 0.04).min(3.0);
        assert!(close(top.text_top, 23.0), "chữ ở đáy khối: {top:?}");
        assert!(
            close(top.image.1, 23.0 + gap) && close(top.image.3, 66.0 - 23.0 - gap),
            "{top:?}"
        );
        assert!(close(top.text_x, 25.0), "khối chữ căn giữa ngang");
        let bottom = image_text_boxes(
            (130.0, 66.0),
            2.0,
            text,
            10.0,
            ImageLayout::Bottom,
            ImageSplit::default(),
        );
        assert!(
            close(bottom.text_top, 66.0)
                && close(bottom.image.1, 0.0)
                && close(bottom.image.3, top.image.3)
        );
        // Khối thấp: chữ không chiếm quá nửa chiều cao, thu nhỏ cho vừa.
        let tight = image_text_boxes(
            (130.0, 30.0),
            2.0,
            text,
            10.0,
            ImageLayout::Top,
            ImageSplit::default(),
        );
        assert!(close(tight.size, 10.0 * 15.0 / 23.0), "{tight:?}");
        // Chữ quá rộng: thu theo bề rộng.
        let wide = image_text_boxes(
            (60.0, 66.0),
            2.0,
            (120.0, 11.5, 2),
            10.0,
            ImageLayout::Bottom,
            ImageSplit::default(),
        );
        assert!(close(wide.size, 5.0) && close(wide.text_x, 0.0), "{wide:?}");

        // Tự chỉnh (TD-51): ảnh chiếm 30% bề ngang, cách chữ 10%.
        let split = ImageSplit {
            share: Some(30),
            gap: Some(10),
            free: None,
        };
        let left = image_text_boxes((170.0, 30.0), 2.0, text, 10.0, ImageLayout::Left, split);
        assert!(
            close(left.image.2, 51.0) && close(left.text_x, 68.0),
            "{left:?}"
        );
        let right = image_text_boxes((170.0, 30.0), 2.0, text, 10.0, ImageLayout::Right, split);
        assert!(
            close(right.image.0, 119.0) && close(right.text_x, 0.0),
            "{right:?}"
        );
        // Ảnh chiếm 60% chiều cao, cách 5%: ảnh trên, chữ ngay dưới khoảng cách.
        let split = ImageSplit {
            share: Some(60),
            gap: Some(5),
            free: None,
        };
        let top = image_text_boxes((130.0, 66.0), 2.0, text, 10.0, ImageLayout::Top, split);
        assert!(
            close(top.image.3, 39.6) && close(top.image.1, 26.4) && close(top.text_top, 23.1),
            "{top:?}"
        );
        // Phần chữ còn 23,1 pt ≥ 23 pt nên không thu nhỏ; ảnh 80% thì chữ còn 9,9 pt, thu cho vừa.
        assert!(close(top.size, 10.0));
        let big = ImageSplit {
            share: Some(80),
            gap: Some(5),
            free: None,
        };
        let bottom = image_text_boxes((130.0, 66.0), 2.0, text, 10.0, ImageLayout::Bottom, big);
        assert!(
            close(bottom.image.1, 0.0)
                && close(bottom.text_top, 66.0)
                && close(bottom.size, 10.0 * 9.9 / 23.0),
            "{bottom:?}"
        );
        // Ngoài khoảng cho phép thì giới hạn lại.
        let wild = ImageSplit {
            share: Some(250),
            gap: Some(99),
            free: None,
        };
        assert_eq!(
            (wild.share_ratio(), wild.gap_ratio()),
            (Some(0.9), Some(0.3))
        );

        // Tự do (TD-52): ô theo tỉ lệ khối, gốc trên trái; ảnh vừa ô, chữ giữa theo chiều dọc ô chữ, thu cho vừa.
        use smks_core::config::PartBox;
        let free = ImageSplit {
            free: Some((
                PartBox {
                    x: 0.6,
                    y: 0.0,
                    w: 0.4,
                    h: 0.5,
                },
                PartBox {
                    x: 0.0,
                    y: 0.5,
                    w: 0.5,
                    h: 0.5,
                },
            )),
            ..ImageSplit::default()
        };
        let f = image_text_boxes((170.0, 30.0), 2.0, text, 10.0, ImageLayout::Left, free);
        assert!(
            close(f.image.0, 102.0)
                && close(f.image.1, 15.0)
                && close(f.image.2, 68.0)
                && close(f.image.3, 15.0),
            "ảnh góc trên phải: {f:?}"
        );
        // Ô chữ 85 × 15 pt: chữ 80 × 23 pt thu theo chiều cao 15/23, nằm sát đáy ô (giữa theo chiều dọc).
        assert!(
            close(f.size, 10.0 * 15.0 / 23.0) && close(f.text_x, 0.0) && close(f.text_top, 15.0),
            "{f:?}"
        );
        // Ô vượt ra ngoài khối thì kéo vào trong.
        let out = PartBox {
            x: 0.9,
            y: -0.2,
            w: 0.5,
            h: 0.01,
        }
        .clean()
        .unwrap();
        assert!(
            close(out.x, 0.5) && close(out.y, 0.0) && close(out.h, 0.03),
            "{out:?}"
        );
        assert!(
            PartBox {
                x: f64::NAN,
                y: 0.0,
                w: 1.0,
                h: 1.0
            }
            .clean()
            .is_none()
        );
    }

    #[test]
    fn darkness_formula_matches_legacy_decode_mapping() {
        // Ảnh 2×1: một điểm đen, một điểm trắng.
        let mut png = Vec::new();
        let img = image::RgbImage::from_raw(2, 1, vec![0, 0, 0, 255, 255, 255]).unwrap();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let same = StampImage::prepare(&png, 100, None, 100).unwrap();
        assert_eq!(same.rgb, vec![0, 0, 0, 255, 255, 255]);
        assert!(same.alpha.is_none());
        // Đậm 140: thấp = 0,8·(1−1,4)/2 = −0,16 → đen giữ đen; cao = 0,96 → trắng thành 245.
        let dark = StampImage::prepare(&png, 140, None, 100).unwrap();
        assert_eq!(dark.rgb, vec![0, 0, 0, 245, 245, 245]);
        // Mực đỏ: đen thành đỏ, trắng gần trắng.
        let red = StampImage::prepare(&png, 100, parse_color("#FF0000"), 100).unwrap();
        assert_eq!(&red.rgb[..3], &[255, 0, 0]);
        assert_eq!(&red.rgb[3..], &[255, 255, 255]);
        assert!(StampImage::prepare(b"khong phai anh", 100, None, 100).is_err());
    }

    #[test]
    fn vietnamese_text_block_embeds_font() {
        let lines = vec![
            "Ký bởi: Nguyễn Thị Ánh Tuyết".to_owned(),
            "Ngày ký: 19/11/2026 10:00".to_owned(),
        ];
        let (font, switched) = FontData::for_lines("Times New Roman", &lines).unwrap();
        assert!(!switched);
        let (w, lh) = font.measure(&lines, 10.0).unwrap();
        assert!(w > 100.0 && (lh - 11.5).abs() < 0.1, "{w} {lh}");
        let style = TextStyle {
            font,
            size: 10.0,
            color: [0.0; 3],
        };
        let mut alloc = Alloc(100);
        let built = build_text(&mut alloc, w, lh * 2.0, 0, &lines, &style).unwrap();
        assert_eq!(built.form, 100);
        assert_eq!(built.objects.len(), 6);
        assert!(String::from_utf8_lossy(&built.objects[1].bytes).contains("/Identity-H"));
        // Georgia thiếu glyph tiếng Việt: đổi sang Times New Roman.
        let (fallback, switched) = FontData::for_lines("Georgia", &lines).unwrap();
        assert!(switched);
        assert_eq!(fallback.family, "Times New Roman");
    }
}
