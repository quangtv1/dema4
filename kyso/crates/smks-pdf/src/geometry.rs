//! Hình học trang (kế hoạch 7.4): vị trí khối chữ ký theo tỉ lệ (port `SignaturePlacementResolver.cs`), đổi
//! toạ độ hiển thị sang toạ độ trang khi trang xoay (port `ToUserSpace`), nhận diện khổ A1–A5.

use serde::{Deserialize, Serialize};
use smks_core::consts::appearance as ap;
use smks_core::{ErrorCode, Result, SmksError};

use crate::reader::{PageInfo, Rect};

pub use smks_core::config::Placement;

fn invalid(message: &str) -> SmksError {
    SmksError::new(ErrorCode::SignPositionInvalid, message)
}

/// Tỉ lệ cỡ chữ / lề theo khổ trang so với A4 (bản cũ: `min(rộng/595, cao/842)`).
pub fn page_scale(display: &Rect) -> f64 {
    (display.width / ap::REFERENCE_PAGE_WIDTH_PT).min(display.height / ap::REFERENCE_PAGE_HEIGHT_PT)
}

/// Khung (toạ độ hiển thị, gốc dưới trái) cho khối kích thước `width × height`.
///
/// - Không có `placement`: góc trên phải, cách mép `8pt × tỉ lệ khổ`.
/// - Có `placement`: neo cạnh trái của khung người dùng vẽ, hoặc cạnh phải nếu tâm khung nằm nửa phải trang
///   (khối chữ đo theo nội dung nên có thể hẹp hơn khung). Cạnh trên luôn theo khung.
/// - Luôn ép vào trong trang.
pub fn resolve(
    display: &Rect,
    placement: Option<&Placement>,
    width: f64,
    height: f64,
) -> Result<Rect> {
    if display.width <= 0.0 || display.height <= 0.0 {
        return Err(invalid(
            "Không đọc được khổ trang PDF nên không tính được vị trí đặt chữ ký.",
        ));
    }
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return Err(invalid(
            "Kích thước khối chữ ký không hợp lệ (bề rộng hoặc chiều cao bằng 0).",
        ));
    }
    let width = width.min(display.width);
    let height = height.min(display.height);
    let (left, bottom) = match placement {
        None => {
            let margin = ap::PAGE_MARGIN_PT * page_scale(display);
            (
                display.right() - width - margin,
                display.top() - height - margin,
            )
        }
        Some(p) => {
            let anchor_right = p.x_ratio + p.width_ratio / 2.0 > 0.5;
            let left = if anchor_right {
                display.x + (p.x_ratio + p.width_ratio) * display.width - width
            } else {
                display.x + p.x_ratio * display.width
            };
            (left, display.top() - p.y_ratio * display.height - height)
        }
    };
    Ok(Rect::new(
        left.clamp(display.x, display.right() - width),
        bottom.clamp(display.y, display.top() - height),
        width,
        height,
    ))
}

/// Khung theo đúng tỉ lệ người dùng vẽ (ảnh dấu, chữ ký tươi), đã ép vào trang.
pub fn frame(display: &Rect, placement: &Placement) -> Result<Rect> {
    resolve(
        display,
        Some(placement),
        placement.width_ratio * display.width,
        placement.height_ratio * display.height,
    )
}

/// Khung `width × height` neo vào một trong 6 điểm của trang (mẫu chữ ký theo khổ giấy), cách mép `margin`
/// (pt, ngang / dọc); điểm neo giữa thì căn giữa theo chiều ngang. Luôn ép vào trong trang.
pub fn anchor_rect(
    display: &Rect,
    anchor: smks_core::config::Anchor,
    margin: (f64, f64),
    width: f64,
    height: f64,
) -> Result<Rect> {
    if display.width <= 0.0 || display.height <= 0.0 {
        return Err(invalid(
            "Không đọc được khổ trang PDF nên không tính được vị trí đặt chữ ký.",
        ));
    }
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return Err(invalid(
            "Kích thước khối chữ ký không hợp lệ (bề rộng hoặc chiều cao bằng 0).",
        ));
    }
    let width = width.min(display.width);
    let height = height.min(display.height);
    let (mx, my) = (margin.0.max(0.0), margin.1.max(0.0));
    let left = match anchor.column() {
        0 => display.x + mx,
        1 => display.x + (display.width - width) / 2.0,
        _ => display.right() - width - mx,
    };
    let bottom = if anchor.is_bottom() {
        display.y + my
    } else {
        display.top() - my - height
    };
    Ok(Rect::new(
        left.clamp(display.x, display.right() - width),
        bottom.clamp(display.y, display.top() - height),
        width,
        height,
    ))
}

/// Đổi khung (toạ độ hiển thị, gốc dưới trái) sang tỉ lệ trang gốc trên trái, như giao diện dùng.
pub fn to_placement(display: &Rect, page: u32, rect: &Rect) -> Placement {
    Placement {
        page,
        x_ratio: (rect.x - display.x) / display.width,
        y_ratio: (display.top() - rect.top()) / display.height,
        width_ratio: rect.width / display.width,
        height_ratio: rect.height / display.height,
    }
}

/// Đổi khổ nhận diện được sang khổ của mẫu cấu hình.
pub fn paper_of(display: &Rect) -> Option<smks_core::config::Paper> {
    use smks_core::config::Paper;
    PaperSize::detect(display).map(|s| match s {
        PaperSize::A1 => Paper::A1,
        PaperSize::A2 => Paper::A2,
        PaperSize::A3 => Paper::A3,
        PaperSize::A4 => Paper::A4,
        PaperSize::A5 => Paper::A5,
    })
}

/// Đổi khung từ toạ độ hiển thị sang toạ độ của trang (tính `/Rotate` và gốc của CropBox).
pub fn to_user_space(page: &PageInfo, rect: &Rect) -> Rect {
    let crop = page.crop_box;
    let (x, y, w, h) = match page.rotation {
        90 => (crop.width - rect.top(), rect.x, rect.height, rect.width),
        180 => (
            crop.width - rect.right(),
            crop.height - rect.top(),
            rect.width,
            rect.height,
        ),
        270 => (rect.y, crop.height - rect.right(), rect.height, rect.width),
        _ => (rect.x, rect.y, rect.width, rect.height),
    };
    Rect::new(crop.x + x, crop.y + y, w, h)
}

/// Ngược của [`to_user_space`]: khung theo toạ độ trang → toạ độ hiển thị (gốc dưới trái của khung nhìn).
pub fn from_user_space(page: &PageInfo, rect: &Rect) -> Rect {
    let crop = page.crop_box;
    let (x, y, w, h) = (rect.x - crop.x, rect.y - crop.y, rect.width, rect.height);
    match page.rotation {
        90 => Rect::new(y, crop.width - x - w, h, w),
        180 => Rect::new(crop.width - x - w, crop.height - y - h, w, h),
        270 => Rect::new(crop.height - y - h, x, h, w),
        _ => Rect::new(x, y, w, h),
    }
}

/// Ma trận `/Matrix` cho Form XObject để nội dung đứng thẳng trên trang xoay.
pub fn rotation_matrix(rotation: i32) -> Option<[f64; 6]> {
    match rotation {
        90 => Some([0.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
        180 => Some([-1.0, 0.0, 0.0, -1.0, 0.0, 0.0]),
        270 => Some([0.0, -1.0, 1.0, 0.0, 0.0, 0.0]),
        _ => None,
    }
}

/// Khổ giấy nhận diện được.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSize {
    A1,
    A2,
    A3,
    A4,
    A5,
}

impl PaperSize {
    pub const ALL: [PaperSize; 5] = [Self::A5, Self::A4, Self::A3, Self::A2, Self::A1];

    /// Kích thước khổ đứng (pt).
    pub fn points(self) -> (f64, f64) {
        match self {
            Self::A5 => (420.0, 595.0),
            Self::A4 => (595.0, 842.0),
            Self::A3 => (842.0, 1191.0),
            Self::A2 => (1191.0, 1684.0),
            Self::A1 => (1684.0, 2384.0),
        }
    }

    /// Khổ của trang (xét cả dọc và ngang), sai số ±3%.
    pub fn detect(display: &Rect) -> Option<Self> {
        let (short, long) = if display.width <= display.height {
            (display.width, display.height)
        } else {
            (display.height, display.width)
        };
        Self::ALL.into_iter().find(|size| {
            let (w, h) = size.points();
            (short - w).abs() <= w * 0.03 && (long - h).abs() <= h * 0.03
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a4() -> Rect {
        Rect::new(0.0, 0.0, 595.0, 842.0)
    }

    #[test]
    fn anchor_rect_six_points() {
        use smks_core::config::Anchor;
        let d = a4();
        let r = anchor_rect(&d, Anchor::Tr, (10.0, 20.0), 170.0, 30.0).unwrap();
        assert_eq!((r.right(), r.top()), (585.0, 822.0));
        let r = anchor_rect(&d, Anchor::Bl, (10.0, 20.0), 170.0, 30.0).unwrap();
        assert_eq!((r.x, r.y), (10.0, 20.0));
        let r = anchor_rect(&d, Anchor::Bc, (10.0, 20.0), 170.0, 30.0).unwrap();
        assert!((r.x - (595.0 - 170.0) / 2.0).abs() < 1e-9);
        let r = anchor_rect(&d, Anchor::Br, (900.0, 900.0), 170.0, 30.0).unwrap();
        assert!(r.x >= 0.0 && r.y >= 0.0 && r.right() <= 595.0 && r.top() <= 842.0);
        let p = to_placement(&d, 1, &Rect::new(59.5, 842.0 - 84.2 - 20.0, 119.0, 20.0));
        assert!(
            (p.x_ratio - 0.1).abs() < 1e-9
                && (p.y_ratio - 0.1).abs() < 1e-9
                && (p.width_ratio - 0.2).abs() < 1e-9
        );
    }

    #[test]
    fn default_placement_is_top_right_with_margin() {
        let r = resolve(&a4(), None, 170.0, 30.0).unwrap();
        assert_eq!((r.x, r.y), (595.0 - 170.0 - 8.0, 842.0 - 30.0 - 8.0));
    }

    #[test]
    fn anchors_left_or_right_and_clamps() {
        let left = Placement {
            page: 1,
            x_ratio: 0.1,
            y_ratio: 0.1,
            width_ratio: 0.2,
            height_ratio: 0.05,
        };
        let r = resolve(&a4(), Some(&left), 100.0, 20.0).unwrap();
        assert!((r.x - 59.5).abs() < 1e-9);
        assert!((r.top() - (842.0 - 84.2)).abs() < 1e-9);
        let right = Placement {
            x_ratio: 0.6,
            ..left
        };
        let r = resolve(&a4(), Some(&right), 100.0, 20.0).unwrap();
        assert!((r.right() - 0.8 * 595.0).abs() < 1e-9);
        let outside = Placement {
            x_ratio: 0.95,
            y_ratio: 0.99,
            ..left
        };
        let r = resolve(&a4(), Some(&outside), 100.0, 20.0).unwrap();
        assert!(r.right() <= 595.0 && r.y >= 0.0);
        assert!(resolve(&a4(), None, 0.0, 10.0).is_err());
    }

    #[test]
    fn user_space_for_rotated_pages() {
        let mut page = PageInfo {
            number: 1,
            id: (1, 0),
            media_box: Rect::new(0.0, 0.0, 595.0, 842.0),
            crop_box: Rect::new(10.0, 20.0, 595.0, 842.0),
            rotation: 0,
        };
        let r = Rect::new(100.0, 200.0, 50.0, 30.0);
        assert_eq!(
            to_user_space(&page, &r),
            Rect::new(110.0, 220.0, 50.0, 30.0)
        );
        page.rotation = 90;
        // Khung hiển thị 842 × 595; khung (100,200,50,30) nằm ở x' = 595 - 230 trong toạ độ trang.
        assert_eq!(
            to_user_space(&page, &r),
            Rect::new(10.0 + 365.0, 20.0 + 100.0, 30.0, 50.0)
        );
        page.rotation = 180;
        assert_eq!(
            to_user_space(&page, &r),
            Rect::new(10.0 + 445.0, 20.0 + 612.0, 50.0, 30.0)
        );
        page.rotation = 270;
        assert_eq!(
            to_user_space(&page, &r),
            Rect::new(10.0 + 200.0, 20.0 + 692.0, 30.0, 50.0)
        );
        // Đổi ngược (màn Kiểm tra vẽ khung chữ ký đã có): mọi góc xoay về đúng khung hiển thị.
        for rotation in [0, 90, 180, 270] {
            page.rotation = rotation;
            assert_eq!(
                from_user_space(&page, &to_user_space(&page, &r)),
                r,
                "xoay {rotation}"
            );
        }
    }

    #[test]
    fn paper_sizes() {
        assert_eq!(PaperSize::detect(&a4()), Some(PaperSize::A4));
        assert_eq!(
            PaperSize::detect(&Rect::new(0.0, 0.0, 842.0, 595.0)),
            Some(PaperSize::A4)
        );
        assert_eq!(
            PaperSize::detect(&Rect::new(0.0, 0.0, 841.89, 1190.55)),
            Some(PaperSize::A3)
        );
        assert_eq!(
            PaperSize::detect(&Rect::new(0.0, 0.0, 612.0, 792.0)),
            None,
            "Letter"
        );
    }
}
