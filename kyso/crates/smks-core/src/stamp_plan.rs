//! Lập kế hoạch đặt dấu đỏ, chữ ký tươi theo trang.
//!
//! Port từ `ktl.be.applications/Document/Implements/StampPlanner.cs` của bản .NET 1.4.0,
//! giữ nguyên quy tắc và câu thông báo. Khác bản cũ ở hai điểm:
//!
//! - Module này không đọc file ảnh. Bên gọi kiểm tra ảnh (tồn tại, đuôi, dung lượng) rồi báo
//!   loại ảnh nào đã có qua [`AvailableImages`].
//! - Kế hoạch trả về chỉ ghi loại ảnh và chỉ số quy tắc, không mang theo dữ liệu ảnh.
//!
//! Toạ độ là tỉ lệ 0–1 so với khổ trang hiển thị, gốc ở góc trên bên trái, trục Y hướng xuống.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::consts::merge::CUSTOM_PAGE_SEPARATOR;
use crate::error::{ErrorCode, Result, SmksError};

/// Loại ảnh đóng lên trang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StampKind {
    /// Dấu đỏ.
    Seal,
    /// Chữ ký tươi.
    WetSignature,
}

impl StampKind {
    /// Tên gọi tiếng Việt dùng trong thông báo.
    pub const fn label(self) -> &'static str {
        match self {
            StampKind::Seal => "dấu đỏ",
            StampKind::WetSignature => "chữ ký tươi",
        }
    }
}

/// Phạm vi áp dụng của một khối ảnh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StampScope {
    /// Chỉ một trang cụ thể.
    CurrentPage,
    /// Lặp trên nhiều trang theo [`RepeatMode`].
    Repeat,
}

/// Kiểu lặp theo trang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatMode {
    All,
    Even,
    Odd,
    /// Danh sách tuỳ chọn, ví dụ `1,3,5-7`.
    Custom,
}

/// Vùng chữ nhật theo tỉ lệ 0–1 của khổ trang hiển thị (gốc trên trái, Y hướng xuống).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatioRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Vị trí riêng của một khối ảnh trên một trang cụ thể.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StampPagePosition {
    pub page_number: i32,
    #[serde(flatten)]
    pub area: RatioRect,
}

/// Cấu hình một khối ảnh do giao diện gửi xuống, chưa kiểm tra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StampSpec {
    pub kind: StampKind,
    pub scope: StampScope,
    pub repeat_mode: RepeatMode,
    #[serde(default)]
    pub custom_pages: Option<String>,
    pub page_number: i32,
    #[serde(flatten)]
    pub area: RatioRect,
    #[serde(default)]
    pub page_positions: Vec<StampPagePosition>,
}

/// Khoảng trang liên tục, tính cả hai đầu, đánh số từ 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRange {
    pub from: u32,
    pub to: u32,
}

/// Quy tắc đã kiểm tra, sẵn sàng áp dụng cho từng file.
#[derive(Debug, Clone, PartialEq)]
pub struct StampRule {
    pub kind: StampKind,
    pub scope: StampScope,
    pub repeat_mode: RepeatMode,
    pub custom_pages: Vec<PageRange>,
    /// Trang đặt ảnh khi phạm vi là [`StampScope::CurrentPage`].
    pub page_number: u32,
    pub area: RatioRect,
    /// Vị trí riêng theo số trang; trang không có trong đây dùng `area`.
    pub page_areas: BTreeMap<u32, RatioRect>,
}

/// Loại ảnh nào đã được chọn và hợp lệ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AvailableImages {
    pub seal: bool,
    pub wet_signature: bool,
}

impl AvailableImages {
    const fn has(self, kind: StampKind) -> bool {
        match kind {
            StampKind::Seal => self.seal,
            StampKind::WetSignature => self.wet_signature,
        }
    }
}

/// Một lần đóng ảnh lên một trang.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannedStamp {
    /// Chỉ số (từ 0) của quy tắc sinh ra lần đóng này.
    pub rule_index: usize,
    pub kind: StampKind,
    pub page_number: u32,
    pub area: RatioRect,
}

/// Kế hoạch đóng ảnh cho một file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StampPlan {
    pub stamps: Vec<PlannedStamp>,
    pub warnings: Vec<String>,
}

fn invalid(message: String) -> SmksError {
    SmksError::new(ErrorCode::MergeStampInvalid, message)
}

/// Kiểm tra cấu hình các khối ảnh và dựng quy tắc.
///
/// `index` trong thông báo đánh số từ 1 theo thứ tự khối ảnh, như bản cũ.
pub fn build_rules(specs: &[StampSpec], images: AvailableImages) -> Result<Vec<StampRule>> {
    let mut rules = Vec::with_capacity(specs.len());
    for (i, spec) in specs.iter().enumerate() {
        let index = i + 1;

        if !images.has(spec.kind) {
            return Err(invalid(format!(
                "Khối ảnh {index} cần ảnh {} nhưng chưa chọn ảnh.",
                spec.kind.label()
            )));
        }
        if spec.scope == StampScope::CurrentPage && spec.page_number < 1 {
            return Err(invalid(format!(
                "Khối ảnh {index} đặt ở trang {} không hợp lệ.",
                spec.page_number
            )));
        }

        let custom_pages =
            if spec.scope == StampScope::Repeat && spec.repeat_mode == RepeatMode::Custom {
                parse_custom_pages(spec.custom_pages.as_deref(), index)?
            } else {
                Vec::new()
            };

        let mut page_areas = BTreeMap::new();
        for position in &spec.page_positions {
            let page = u32::try_from(position.page_number)
                .ok()
                .filter(|p| *p >= 1)
                .ok_or_else(|| {
                    invalid(format!(
                        "Khối ảnh {index} có vị trí riêng ở trang {} không hợp lệ.",
                        position.page_number
                    ))
                })?;
            let area = normalize_area(position.area, index)?;
            if page_areas.insert(page, area).is_some() {
                return Err(invalid(format!(
                    "Khối ảnh {index} có nhiều vị trí riêng cho trang {page}."
                )));
            }
        }

        rules.push(StampRule {
            kind: spec.kind,
            scope: spec.scope,
            repeat_mode: spec.repeat_mode,
            custom_pages,
            // Với phạm vi lặp, số trang không được dùng; chuẩn hoá về 1 nếu giao diện gửi số lạ.
            page_number: u32::try_from(spec.page_number).unwrap_or(1).max(1),
            area: normalize_area(spec.area, index)?,
            page_areas,
        });
    }
    Ok(rules)
}

/// Áp các quy tắc lên một file có `page_count` trang.
pub fn plan(rules: &[StampRule], page_count: u32) -> StampPlan {
    let mut result = StampPlan::default();
    for (i, rule) in rules.iter().enumerate() {
        let (pages, warning) = resolve_pages(rule, page_count);
        if let Some(warning) = warning {
            result.warnings.push(format!(
                "Khối ảnh {} ({}) {warning}",
                i + 1,
                rule.kind.label()
            ));
        }
        result
            .stamps
            .extend(pages.into_iter().map(|page| PlannedStamp {
                rule_index: i,
                kind: rule.kind,
                page_number: page,
                area: rule.page_areas.get(&page).copied().unwrap_or(rule.area),
            }));
    }
    result
}

fn resolve_pages(rule: &StampRule, page_count: u32) -> (Vec<u32>, Option<String>) {
    if rule.scope == StampScope::CurrentPage {
        if rule.page_number <= page_count {
            return (vec![rule.page_number], None);
        }
        return (
            Vec::new(),
            Some(format!(
                "đặt ở trang {} nhưng file chỉ có {page_count} trang nên bỏ qua.",
                rule.page_number
            )),
        );
    }

    let pages: Vec<u32> = (1..=page_count)
        .filter(|page| match rule.repeat_mode {
            RepeatMode::All => true,
            RepeatMode::Even => page % 2 == 0,
            RepeatMode::Odd => page % 2 == 1,
            RepeatMode::Custom => rule
                .custom_pages
                .iter()
                .any(|r| *page >= r.from && *page <= r.to),
        })
        .collect();

    let warning = if rule.repeat_mode == RepeatMode::Custom
        && rule.custom_pages.iter().any(|r| r.to > page_count)
    {
        Some(format!(
            "có trang vượt quá {page_count} trang của file nên bỏ qua các trang đó."
        ))
    } else if pages.is_empty() {
        Some("không có trang nào khớp kiểu lặp đã chọn nên bỏ qua.".to_owned())
    } else {
        None
    };

    (pages, warning)
}

fn normalize_area(area: RatioRect, index: usize) -> Result<RatioRect> {
    // Bản cũ chỉ kiểm tra `<= 0`; NaN lọt qua và sinh toạ độ hỏng. Ở đây chặn luôn.
    if !(area.width > 0.0 && area.height > 0.0)
        || !area.width.is_finite()
        || !area.height.is_finite()
    {
        return Err(invalid(format!(
            "Khối ảnh {index} có kích thước không hợp lệ (bề rộng hoặc chiều cao bằng 0)."
        )));
    }
    if !area.x.is_finite() || !area.y.is_finite() {
        return Err(invalid(format!("Khối ảnh {index} có toạ độ không hợp lệ.")));
    }

    let width = area.width.min(1.0);
    let height = area.height.min(1.0);
    Ok(RatioRect {
        x: area.x.clamp(0.0, 1.0 - width),
        y: area.y.clamp(0.0, 1.0 - height),
        width,
        height,
    })
}

/// Đọc danh sách trang tuỳ chọn dạng `1,3,5-7`.
///
/// Mỗi phần phải khớp `^(\d+)(?:\s*-\s*(\d+))?$` (`DocumentMergeConstants.CustomPagePattern`).
pub fn parse_custom_pages(custom_pages: Option<&str>, index: usize) -> Result<Vec<PageRange>> {
    let text = custom_pages.unwrap_or_default();
    let bad_format = || {
        invalid(format!(
            "Khối ảnh {index} có danh sách trang \"{text}\" sai định dạng, ví dụ đúng: 1,3,5-7."
        ))
    };

    let mut ranges = Vec::new();
    for part in text
        .split(CUSTOM_PAGE_SEPARATOR)
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let bad_range = || {
            invalid(format!(
                "Khối ảnh {index} có khoảng trang \"{part}\" không hợp lệ."
            ))
        };

        let (from_text, to_text) = match part.split_once('-') {
            Some((from, to)) => (from.trim_end(), Some(to.trim_start())),
            None => (part, None),
        };
        if !is_digits(from_text) || to_text.is_some_and(|t| !is_digits(t)) {
            return Err(bad_format());
        }

        // Bản cũ đọc bằng int.TryParse: số đầu tràn int thì báo sai định dạng,
        // số cuối tràn int thì báo khoảng trang không hợp lệ.
        let from: i32 = from_text.parse().map_err(|_| bad_format())?;
        let to: i32 = match to_text {
            Some(t) => t.parse().map_err(|_| bad_range())?,
            None => from,
        };
        if from < 1 || to < from {
            return Err(bad_range());
        }

        ranges.push(PageRange {
            from: from.unsigned_abs(),
            to: to.unsigned_abs(),
        });
    }

    if ranges.is_empty() {
        return Err(invalid(format!(
            "Khối ảnh {index} chọn lặp tùy chọn nhưng chưa nhập danh sách trang."
        )));
    }
    Ok(ranges)
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: AvailableImages = AvailableImages {
        seal: true,
        wet_signature: true,
    };

    fn area(x: f64, y: f64, width: f64, height: f64) -> RatioRect {
        RatioRect {
            x,
            y,
            width,
            height,
        }
    }

    fn spec(kind: StampKind, scope: StampScope, repeat_mode: RepeatMode) -> StampSpec {
        StampSpec {
            kind,
            scope,
            repeat_mode,
            custom_pages: None,
            page_number: 1,
            area: area(0.6, 0.7, 0.2, 0.1),
            page_positions: Vec::new(),
        }
    }

    fn repeat(mode: RepeatMode, custom: Option<&str>) -> StampSpec {
        StampSpec {
            custom_pages: custom.map(str::to_owned),
            ..spec(StampKind::Seal, StampScope::Repeat, mode)
        }
    }

    fn pages_of(plan: &StampPlan) -> Vec<u32> {
        plan.stamps.iter().map(|s| s.page_number).collect()
    }

    fn message(result: Result<Vec<StampRule>>) -> String {
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::MergeStampInvalid);
        err.message
    }

    #[test]
    fn empty_specs_give_empty_rules_even_without_images() {
        assert_eq!(
            build_rules(&[], AvailableImages::default()).unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn missing_image_is_reported_with_kind_label() {
        let specs = [spec(
            StampKind::WetSignature,
            StampScope::CurrentPage,
            RepeatMode::All,
        )];
        let images = AvailableImages {
            seal: true,
            wet_signature: false,
        };
        assert_eq!(
            message(build_rules(&specs, images)),
            "Khối ảnh 1 cần ảnh chữ ký tươi nhưng chưa chọn ảnh."
        );
    }

    #[test]
    fn current_page_below_one_is_rejected() {
        let mut s = spec(StampKind::Seal, StampScope::CurrentPage, RepeatMode::All);
        s.page_number = 0;
        assert_eq!(
            message(build_rules(&[s], BOTH)),
            "Khối ảnh 1 đặt ở trang 0 không hợp lệ."
        );
    }

    #[test]
    fn repeat_scope_ignores_page_number() {
        let mut s = repeat(RepeatMode::All, None);
        s.page_number = -5;
        let rules = build_rules(&[s], BOTH).unwrap();
        assert_eq!(pages_of(&plan(&rules, 3)), vec![1, 2, 3]);
    }

    #[test]
    fn area_is_clamped_into_page() {
        let mut s = spec(StampKind::Seal, StampScope::CurrentPage, RepeatMode::All);
        s.area = area(0.95, -0.2, 0.2, 1.5);
        let rules = build_rules(&[s], BOTH).unwrap();
        assert_eq!(rules[0].area, area(0.8, 0.0, 0.2, 1.0));
    }

    #[test]
    fn zero_or_nan_size_is_rejected() {
        for bad in [
            area(0.1, 0.1, 0.0, 0.1),
            area(0.1, 0.1, 0.1, -1.0),
            area(0.1, 0.1, f64::NAN, 0.1),
        ] {
            let mut s = spec(StampKind::Seal, StampScope::CurrentPage, RepeatMode::All);
            s.area = bad;
            assert_eq!(
                message(build_rules(&[s], BOTH)),
                "Khối ảnh 1 có kích thước không hợp lệ (bề rộng hoặc chiều cao bằng 0)."
            );
        }
    }

    #[test]
    fn page_positions_are_validated_and_must_be_unique() {
        let mut s = repeat(RepeatMode::All, None);
        s.page_positions = vec![StampPagePosition {
            page_number: 0,
            area: area(0.1, 0.1, 0.2, 0.2),
        }];
        assert_eq!(
            message(build_rules(&[s.clone()], BOTH)),
            "Khối ảnh 1 có vị trí riêng ở trang 0 không hợp lệ."
        );

        s.page_positions = vec![
            StampPagePosition {
                page_number: 2,
                area: area(0.1, 0.1, 0.2, 0.2),
            },
            StampPagePosition {
                page_number: 2,
                area: area(0.3, 0.3, 0.2, 0.2),
            },
        ];
        assert_eq!(
            message(build_rules(&[s], BOTH)),
            "Khối ảnh 1 có nhiều vị trí riêng cho trang 2."
        );
    }

    #[test]
    fn page_position_overrides_shared_area_on_that_page_only() {
        let mut s = repeat(RepeatMode::All, None);
        let own = area(0.1, 0.1, 0.3, 0.3);
        s.page_positions = vec![StampPagePosition {
            page_number: 2,
            area: own,
        }];
        let rules = build_rules(&[s], BOTH).unwrap();
        let result = plan(&rules, 3);
        assert_eq!(result.stamps[0].area, area(0.6, 0.7, 0.2, 0.1));
        assert_eq!(result.stamps[1].area, own);
        assert_eq!(result.stamps[2].area, area(0.6, 0.7, 0.2, 0.1));
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn repeat_modes_select_expected_pages() {
        let cases = [
            (repeat(RepeatMode::All, None), vec![1, 2, 3, 4, 5]),
            (repeat(RepeatMode::Even, None), vec![2, 4]),
            (repeat(RepeatMode::Odd, None), vec![1, 3, 5]),
            (repeat(RepeatMode::Custom, Some("1, 3 - 4")), vec![1, 3, 4]),
        ];
        for (s, expected) in cases {
            let rules = build_rules(&[s], BOTH).unwrap();
            let result = plan(&rules, 5);
            assert_eq!(pages_of(&result), expected);
            assert!(result.warnings.is_empty());
        }
    }

    #[test]
    fn current_page_beyond_document_is_skipped_with_warning() {
        let mut s = spec(
            StampKind::WetSignature,
            StampScope::CurrentPage,
            RepeatMode::All,
        );
        s.page_number = 4;
        let rules = build_rules(&[s], BOTH).unwrap();
        let result = plan(&rules, 2);
        assert!(result.stamps.is_empty());
        assert_eq!(
            result.warnings,
            vec!["Khối ảnh 1 (chữ ký tươi) đặt ở trang 4 nhưng file chỉ có 2 trang nên bỏ qua."]
        );
    }

    #[test]
    fn custom_pages_beyond_document_warn_but_keep_valid_pages() {
        let rules = build_rules(&[repeat(RepeatMode::Custom, Some("1,3,5-7"))], BOTH).unwrap();
        let result = plan(&rules, 5);
        assert_eq!(pages_of(&result), vec![1, 3, 5]);
        assert_eq!(
            result.warnings,
            vec!["Khối ảnh 1 (dấu đỏ) có trang vượt quá 5 trang của file nên bỏ qua các trang đó."]
        );
    }

    #[test]
    fn repeat_without_matching_page_warns() {
        let rules = build_rules(&[repeat(RepeatMode::Even, None)], BOTH).unwrap();
        let result = plan(&rules, 1);
        assert!(result.stamps.is_empty());
        assert_eq!(
            result.warnings,
            vec!["Khối ảnh 1 (dấu đỏ) không có trang nào khớp kiểu lặp đã chọn nên bỏ qua."]
        );
    }

    #[test]
    fn warnings_are_numbered_by_rule_position() {
        let mut second = spec(
            StampKind::WetSignature,
            StampScope::CurrentPage,
            RepeatMode::All,
        );
        second.page_number = 9;
        let rules = build_rules(&[repeat(RepeatMode::All, None), second], BOTH).unwrap();
        let result = plan(&rules, 2);
        assert_eq!(pages_of(&result), vec![1, 2]);
        assert_eq!(result.stamps[0].rule_index, 0);
        assert!(result.warnings[0].starts_with("Khối ảnh 2 (chữ ký tươi) "));
    }

    #[test]
    fn custom_pages_accept_single_pages_ranges_and_spaces() {
        assert_eq!(
            parse_custom_pages(Some(" 1 ,3, 5 - 7 ,, "), 1).unwrap(),
            vec![
                PageRange { from: 1, to: 1 },
                PageRange { from: 3, to: 3 },
                PageRange { from: 5, to: 7 }
            ]
        );
    }

    #[test]
    fn custom_pages_report_format_errors_like_legacy() {
        let format_error = |text: &str| {
            format!("Khối ảnh 2 có danh sách trang \"{text}\" sai định dạng, ví dụ đúng: 1,3,5-7.")
        };
        for text in ["a", "1-", "-3", "1-2-3", "1.5", "99999999999"] {
            assert_eq!(
                parse_custom_pages(Some(text), 2).unwrap_err().message,
                format_error(text)
            );
        }

        let range_error =
            |part: &str| format!("Khối ảnh 2 có khoảng trang \"{part}\" không hợp lệ.");
        for part in ["0", "5-3", "1-99999999999"] {
            assert_eq!(
                parse_custom_pages(Some(part), 2).unwrap_err().message,
                range_error(part)
            );
        }

        for empty in [None, Some(""), Some(" , ,")] {
            assert_eq!(
                parse_custom_pages(empty, 2).unwrap_err().message,
                "Khối ảnh 2 chọn lặp tùy chọn nhưng chưa nhập danh sách trang."
            );
        }
    }

    #[test]
    fn spec_deserializes_from_camel_case_json() {
        let json = r#"{
            "kind": "wetSignature", "scope": "repeat", "repeatMode": "custom",
            "customPages": "2-3", "pageNumber": 1,
            "x": 0.5, "y": 0.5, "width": 0.2, "height": 0.1,
            "pagePositions": [{ "pageNumber": 3, "x": 0.1, "y": 0.1, "width": 0.2, "height": 0.1 }]
        }"#;
        let parsed: StampSpec = serde_json::from_str(json).unwrap();
        let rules = build_rules(&[parsed], BOTH).unwrap();
        let result = plan(&rules, 4);
        assert_eq!(pages_of(&result), vec![2, 3]);
        assert_eq!(result.stamps[1].area, area(0.1, 0.1, 0.2, 0.1));
        assert_eq!(result.stamps[0].kind, StampKind::WetSignature);
    }
}
