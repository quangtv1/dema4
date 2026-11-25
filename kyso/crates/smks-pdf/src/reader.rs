//! Đọc PDF để ký nối bản (kế hoạch 7.1, thay `PdfRevisionInfo.cs`): object qua `lopdf` (kể cả object stream,
//! file hybrid-reference), phần đuôi file (`startxref`, kiểu xref cuối) tự đọc, cây trang có kế thừa
//! `MediaBox` / `CropBox` / `Rotate`.

use lopdf::{Dictionary, Document, Object, ObjectId, xref::XrefType};
use serde::Serialize;
use smks_core::{ErrorCode, Result, SmksError};

/// Hình chữ nhật theo điểm (pt), gốc ở góc dưới trái.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn top(&self) -> f64 {
        self.y + self.height
    }

    fn from_array(values: &[f64]) -> Option<Self> {
        let [a, b, c, d] = values else { return None };
        let (x0, x1) = if a <= c { (*a, *c) } else { (*c, *a) };
        let (y0, y1) = if b <= d { (*b, *d) } else { (*d, *b) };
        (x1 > x0 && y1 > y0).then(|| Self::new(x0, y0, x1 - x0, y1 - y0))
    }

    fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.top().min(other.top());
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Một trang.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    /// Số trang, từ 1.
    pub number: u32,
    #[serde(skip)]
    pub id: ObjectId,
    pub media_box: Rect,
    /// `CropBox ∩ MediaBox` (mặc định bằng MediaBox).
    pub crop_box: Rect,
    /// 0, 90, 180, 270.
    pub rotation: i32,
}

impl PageInfo {
    /// Khung người xem thấy: CropBox, đổi chiều rộng/cao khi trang xoay 90/270 (gốc vẫn ở góc dưới trái).
    pub fn display_box(&self) -> Rect {
        let c = self.crop_box;
        if self.rotation == 90 || self.rotation == 270 {
            Rect::new(0.0, 0.0, c.height, c.width)
        } else {
            Rect::new(0.0, 0.0, c.width, c.height)
        }
    }
}

fn unsupported(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignPdfStructureUnsupported, message)
}

/// File PDF đã mở.
#[derive(Debug)]
pub struct PdfFile {
    pub doc: Document,
    /// Offset của bảng / stream xref cuối (giá trị `startxref` cuối file).
    pub last_xref: usize,
    /// Xref cuối là stream (`/W [1 4 2]` khi ghi nối) hay bảng.
    pub xref_stream: bool,
    pub len: usize,
}

impl PdfFile {
    pub fn open(bytes: &[u8]) -> Result<Self> {
        let last_xref = last_startxref(bytes)?;
        let doc = Document::load_mem(bytes).map_err(|e| {
            if e.to_string().to_lowercase().contains("encrypt") {
                SmksError::new(
                    ErrorCode::SignPdfEncrypted,
                    "File PDF được mã hoá nên không ký số được.",
                )
            } else {
                unsupported(format!("Không đọc được cấu trúc file PDF: {e}"))
            }
        })?;
        let xref_stream = matches!(
            doc.reference_table.cross_reference_type,
            XrefType::CrossReferenceStream
        );
        Ok(Self {
            doc,
            last_xref,
            xref_stream,
            len: bytes.len(),
        })
    }

    pub fn root_id(&self) -> Result<ObjectId> {
        self.doc
            .trailer
            .get(b"Root")
            .and_then(Object::as_reference)
            .map_err(|_| unsupported("File PDF không có Catalog (/Root)."))
    }

    pub fn catalog(&self) -> Result<&Dictionary> {
        self.doc
            .get_dictionary(self.root_id()?)
            .map_err(|_| unsupported("Không đọc được Catalog của file PDF."))
    }

    /// Số object kế tiếp còn trống.
    pub fn next_object_number(&self) -> u32 {
        let size = self
            .doc
            .trailer
            .get(b"Size")
            .and_then(Object::as_i64)
            .unwrap_or(0);
        u32::try_from(size).unwrap_or(0).max(self.doc.max_id + 1)
    }

    /// Lấy object, đi qua tham chiếu gián tiếp.
    pub fn resolve<'a>(&'a self, object: &'a Object) -> &'a Object {
        self.doc
            .dereference(object)
            .map(|(_, o)| o)
            .unwrap_or(object)
    }

    fn number(&self, object: &Object) -> Option<f64> {
        match self.resolve(object) {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(r) => Some(f64::from(*r)),
            _ => None,
        }
    }

    fn rect_of(&self, object: &Object) -> Option<Rect> {
        let Object::Array(items) = self.resolve(object) else {
            return None;
        };
        let values: Vec<f64> = items.iter().filter_map(|o| self.number(o)).collect();
        Rect::from_array(&values)
    }

    /// Thuộc tính có kế thừa qua `/Parent` (MediaBox, CropBox, Rotate).
    fn inherited<'a>(&'a self, mut dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
        for _ in 0..64 {
            if let Ok(value) = dict.get(key) {
                return Some(value);
            }
            let parent = dict.get(b"Parent").ok()?.as_reference().ok()?;
            dict = self.doc.get_dictionary(parent).ok()?;
        }
        None
    }

    pub fn pages(&self) -> Result<Vec<PageInfo>> {
        let pages = self.doc.get_pages();
        if pages.is_empty() {
            return Err(unsupported("File PDF không có trang nào."));
        }
        let mut out = Vec::with_capacity(pages.len());
        for (number, id) in pages {
            let dict = self
                .doc
                .get_dictionary(id)
                .map_err(|_| unsupported(format!("Không đọc được trang {number} của file PDF.")))?;
            // Không có MediaBox thì mặc định A4 (giống trình xem thông dụng).
            let media_box = self
                .inherited(dict, b"MediaBox")
                .and_then(|o| self.rect_of(o))
                .unwrap_or(Rect::new(0.0, 0.0, 595.0, 842.0));
            let crop_box = self
                .inherited(dict, b"CropBox")
                .and_then(|o| self.rect_of(o))
                .and_then(|c| c.intersect(&media_box))
                .unwrap_or(media_box);
            let rotate = self
                .inherited(dict, b"Rotate")
                .and_then(|o| self.number(o))
                .unwrap_or(0.0);
            let rotation = ((rotate / 90.0).round() as i32 * 90).rem_euclid(360);
            out.push(PageInfo {
                number,
                id,
                media_box,
                crop_box,
                rotation,
            });
        }
        Ok(out)
    }

    /// Có chữ ký chứng thực `/DocMDP` với `/P 1` (không cho phép thay đổi gì, kể cả ký thêm).
    pub fn is_locked_by_docmdp(&self) -> bool {
        let Ok(catalog) = self.catalog() else {
            return false;
        };
        let Some(Object::Dictionary(perms)) = catalog.get(b"Perms").ok().map(|o| self.resolve(o))
        else {
            return false;
        };
        let Some(Object::Dictionary(sig)) = perms.get(b"DocMDP").ok().map(|o| self.resolve(o))
        else {
            return false;
        };
        let Some(Object::Array(refs)) = sig.get(b"Reference").ok().map(|o| self.resolve(o)) else {
            return false;
        };
        refs.iter().any(|r| {
            let Object::Dictionary(entry) = self.resolve(r) else {
                return false;
            };
            let is_docmdp = entry
                .get(b"TransformMethod")
                .and_then(Object::as_name)
                .is_ok_and(|n| n == b"DocMDP");
            let p = entry
                .get(b"TransformParams")
                .ok()
                .map(|o| self.resolve(o))
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"P").ok())
                .and_then(|o| self.number(o))
                .unwrap_or(2.0);
            is_docmdp && p == 1.0
        })
    }

    /// Kiểm điều kiện ký nối bản: không mã hoá, không bị khoá bởi `/DocMDP P=1`.
    pub fn check_signable(&self) -> Result<()> {
        if self.doc.trailer.has(b"Encrypt") {
            return Err(SmksError::new(
                ErrorCode::SignPdfEncrypted,
                "File PDF được mã hoá nên không ký số được.",
            ));
        }
        if self.is_locked_by_docmdp() {
            return Err(SmksError::new(
                ErrorCode::SignPdfLocked,
                "File PDF đã có chữ ký chứng thực không cho phép thay đổi (/DocMDP P=1) nên không ký thêm được.",
            ));
        }
        Ok(())
    }
}

/// Giá trị `startxref` cuối file.
pub fn last_startxref(bytes: &[u8]) -> Result<usize> {
    let tail = &bytes[bytes.len().saturating_sub(2048)..];
    let at = tail
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| unsupported("Không tìm thấy startxref ở cuối file PDF."))?;
    let digits: String = tail[at + 9..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|b| char::from(*b))
        .collect();
    digits
        .parse()
        .map_err(|_| unsupported("Giá trị startxref ở cuối file PDF không hợp lệ."))
}
