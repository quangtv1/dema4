//! Ghép biến với nguồn dữ liệu, đặt tên file, kiểm tra trước khi tạo.
//!
//! Port phần `BuildContext`, `Validate`, `BuildRowWarnings`, `AutoMap` của `DocumentMergeService.cs`.
//! Khác bản cũ (theo kế hoạch mục 8.5 và prototype, xem Q-07):
//! - Mỗi biến lấy từ một cột, hoặc một giá trị cố định, hoặc để trống.
//! - Tên file theo mẫu như `{so}_{hoten}` (tên trong ngoặc là biến của mẫu văn bản), bỏ dấu, ký tự cấm thành
//!   `-`, khoảng trắng thành `_`, tối đa 100 ký tự; trùng thì thêm ` (1)`, ` (2)`; rỗng thì `vanban_<số dòng>`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use smks_core::consts::merge::{FIELD_PATTERN, FILE_NAME_MAX_LENGTH};
use smks_core::{ErrorCode, Result, SmksError};

use crate::numfmt::NumberLocale;
use crate::table::{Table, TableColumn, read_table};
use crate::text::{match_key, strip_diacritics};

/// Nguồn giá trị của một biến.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FieldSource {
    Column { column: u32 },
    Fixed { value: String },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DataSource {
    Table {
        path: String,
        sheet: Option<String>,
        header_row: u32,
    },
    Manual {
        values: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Pdf,
    Docx,
}

impl OutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            OutputFormat::Pdf => "pdf",
            OutputFormat::Docx => "docx",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequest {
    pub template_path: String,
    pub data: DataSource,
    #[serde(default)]
    pub mappings: HashMap<String, FieldSource>,
    pub file_name_pattern: String,
    pub format: OutputFormat,
    #[serde(default)]
    pub output_folder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub row_number: u32,
    pub file_name: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePlan {
    pub files: Vec<PlannedFile>,
    pub unmapped_fields: Vec<String>,
    pub warnings: Vec<String>,
    pub output_folder: String,
    pub word_available: bool,
}

/// Một văn bản sẽ tạo: giá trị đã ghép cho từng biến.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub row_number: u32,
    pub values: HashMap<String, String>,
    pub file_name: String,
    pub warnings: Vec<String>,
}

/// Mọi thứ cần để chạy một lô, đã kiểm tra.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub template: Vec<u8>,
    pub template_name: String,
    pub fields: Vec<String>,
    pub records: Vec<Record>,
    pub unmapped_fields: Vec<String>,
    pub warnings: Vec<String>,
    pub format: OutputFormat,
    pub output_folder: PathBuf,
}

impl Prepared {
    pub fn plan(&self, word_available: bool) -> MergePlan {
        MergePlan {
            files: self
                .records
                .iter()
                .map(|r| PlannedFile {
                    row_number: r.row_number,
                    file_name: r.file_name.clone(),
                    warnings: r.warnings.clone(),
                })
                .collect(),
            unmapped_fields: self.unmapped_fields.clone(),
            warnings: self.warnings.clone(),
            output_folder: self.output_folder.display().to_string(),
            word_available,
        }
    }
}

fn mapping_error(message: String) -> SmksError {
    SmksError::new(ErrorCode::MergeMappingInvalid, message)
}

/// Đọc mẫu và dữ liệu, ghép biến, đặt tên file, lập cảnh báo.
///
/// `stamp` là mốc thời gian `yyyyMMdd_HHmmss` dùng cho tên thư mục kết quả.
pub fn prepare(request: &MergeRequest, locale: NumberLocale, stamp: &str) -> Result<Prepared> {
    let template_path = Path::new(&request.template_path);
    let is_docx = template_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("docx"));
    if !is_docx {
        return Err(SmksError::new(
            ErrorCode::MergeTemplateInvalid,
            "Mẫu văn bản phải là file Word .docx.",
        ));
    }
    let template = std::fs::read(template_path).map_err(|e| {
        SmksError::new(
            ErrorCode::MergeTemplateInvalid,
            format!("Không đọc được file mẫu {}: {e}", template_path.display()),
        )
    })?;
    let fields = crate::docx::read_fields(&template)?;
    let template_name = template_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut warnings = Vec::new();
    let (records_values, unmapped, base_folder) = match &request.data {
        DataSource::Table {
            path,
            sheet,
            header_row,
        } => {
            let table = read_table(Path::new(path), sheet.as_deref(), Some(*header_row), locale)?;
            if table.rows.is_empty() {
                return Err(SmksError::new(
                    ErrorCode::MergeDataInvalid,
                    "File dữ liệu không có dòng dữ liệu nào.",
                ));
            }
            let (rows, unmapped) = resolve_table(&table, &fields, &request.mappings)?;
            (
                rows,
                unmapped,
                Path::new(path).parent().map(Path::to_path_buf),
            )
        }
        DataSource::Manual { values } => {
            let row: HashMap<String, String> = fields
                .iter()
                .map(|f| (f.clone(), values.get(f).cloned().unwrap_or_default()))
                .collect();
            let empty: Vec<String> = fields
                .iter()
                .filter(|f| row[*f].trim().is_empty())
                .cloned()
                .collect();
            let row_warnings = empty
                .iter()
                .map(|f| format!("Trường \"{f}\" để trống."))
                .collect();
            (
                vec![(1, row, row_warnings)],
                Vec::new(),
                template_path.parent().map(Path::to_path_buf),
            )
        }
    };
    if !unmapped.is_empty() {
        warnings.push(format!(
            "Có {} biến chưa ghép nguồn, sẽ để trống khi tạo: {}.",
            unmapped.len(),
            unmapped.join(", ")
        ));
    }

    let pattern_fields = pattern_fields(&request.file_name_pattern)?;
    for name in &pattern_fields {
        if !fields.contains(name) {
            warnings.push(format!(
                "Mẫu tên file có biến {{{name}}} không có trong mẫu văn bản, phần đó sẽ để trống."
            ));
        }
    }

    let extension = request.format.extension();
    let mut used: HashSet<String> = HashSet::new();
    let mut records = Vec::with_capacity(records_values.len());
    for (row_number, values, mut row_warnings) in records_values {
        let raw = apply_pattern(&request.file_name_pattern, &values);
        let mut stem = sanitize_file_stem(&raw);
        if stem.is_empty() {
            stem = format!("vanban_{row_number}");
            row_warnings.push(format!(
                "Tên file theo mẫu bị rỗng, dùng \"{stem}.{extension}\"."
            ));
        }
        let base = stem.clone();
        let mut counter = 1;
        while !used.insert(stem.to_lowercase()) {
            stem = format!("{base} ({counter})");
            counter += 1;
        }
        if stem != base {
            row_warnings.push(format!("Tên file \"{base}.{extension}\" trùng với dòng khác, đổi thành \"{stem}.{extension}\"."));
        }
        records.push(Record {
            row_number,
            values,
            file_name: format!("{stem}.{extension}"),
            warnings: row_warnings,
        });
    }

    let base_folder = match &request.output_folder {
        Some(folder) if !folder.trim().is_empty() => PathBuf::from(folder.trim()),
        _ => base_folder.unwrap_or_else(|| PathBuf::from(".")),
    };
    let output_folder = base_folder.join(format!("{}_{stamp}", sanitize_file_stem(&template_name)));

    Ok(Prepared {
        template,
        template_name,
        fields,
        records,
        unmapped_fields: unmapped,
        warnings,
        format: request.format,
        output_folder,
    })
}

type RowValues = (u32, HashMap<String, String>, Vec<String>);

fn resolve_table(
    table: &Table,
    fields: &[String],
    mappings: &HashMap<String, FieldSource>,
) -> Result<(Vec<RowValues>, Vec<String>)> {
    for (field, source) in mappings {
        if !fields.contains(field) {
            // Giao diện có thể giữ ghép cũ khi đổi mẫu: bỏ qua biến không còn trong mẫu thay vì báo lỗi,
            // trừ khi nó trỏ tới cột không tồn tại (dấu hiệu dữ liệu đã đổi).
            continue;
        }
        if let FieldSource::Column { column } = source
            && !table.columns.iter().any(|c| c.column == *column)
        {
            return Err(mapping_error(format!(
                "Cột số {column} được ghép cho biến \"{field}\" không có trong file dữ liệu. Ghép lại biến này."
            )));
        }
    }
    let unmapped: Vec<String> = fields
        .iter()
        .filter(|f| matches!(mappings.get(*f), None | Some(FieldSource::None)))
        .cloned()
        .collect();

    let header_of = |column: u32| {
        table
            .columns
            .iter()
            .find(|c| c.column == column)
            .map_or("", |c| c.name.as_str())
    };
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let mut values = HashMap::new();
            let mut warnings = Vec::new();
            for field in fields {
                let value = match mappings.get(field) {
                    Some(FieldSource::Column { column }) => {
                        let value = table.value(row, *column).unwrap_or_default().to_owned();
                        if value.trim().is_empty() {
                            warnings.push(format!(
                                "Trường \"{field}\" để trống (cột \"{}\").",
                                header_of(*column)
                            ));
                        }
                        value
                    }
                    Some(FieldSource::Fixed { value }) => value.clone(),
                    Some(FieldSource::None) | None => String::new(),
                };
                values.insert(field.clone(), value);
            }
            (row.row_number, values, warnings)
        })
        .collect();
    Ok((rows, unmapped))
}

/// Tự ghép biến với cột: so khớp tên đã bỏ dấu, chữ thường, `_` như khoảng trắng (như bản cũ).
pub fn auto_map(fields: &[String], columns: &[TableColumn]) -> HashMap<String, Option<u32>> {
    let keys: Vec<(String, u32)> = columns
        .iter()
        .map(|c| (match_key(&c.name), c.column))
        .filter(|(k, _)| !k.is_empty())
        .collect();
    fields
        .iter()
        .map(|field| {
            let key = match_key(field);
            (
                field.clone(),
                keys.iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, column)| *column),
            )
        })
        .collect()
}

fn pattern_fields(pattern: &str) -> Result<Vec<String>> {
    let regex = Regex::new(FIELD_PATTERN)
        .map_err(|e| SmksError::new(ErrorCode::InternalError, e.to_string()))?;
    Ok(regex
        .captures_iter(pattern)
        .map(|c| c[1].trim().to_owned())
        .collect())
}

/// Thay `{bien}` trong mẫu tên file bằng giá trị.
pub fn apply_pattern(pattern: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find(['}', '{']) {
            Some(close) if after.as_bytes()[close] == b'}' => {
                let name = after[..close].trim();
                out.push_str(values.get(name).map_or("", String::as_str));
                rest = &after[close + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Phần tên file (không đuôi) an toàn trên Windows.
pub fn sanitize_file_stem(raw: &str) -> String {
    let stripped = strip_diacritics(raw);
    let mut out = String::with_capacity(stripped.len());
    let mut last_was_space = false;
    for c in stripped.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                out.push('_');
            }
            last_was_space = true;
            continue;
        }
        last_was_space = false;
        if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            out.push('-');
        } else {
            out.push(c);
        }
    }
    let trimmed = out.trim_matches(|c| c == '_' || c == '.' || c == ' ');
    let mut result: String = trimmed.chars().take(FILE_NAME_MAX_LENGTH).collect();
    result = result.trim_end_matches(['.', ' ']).to_owned();
    let upper = result.to_ascii_uppercase();
    if RESERVED_NAMES
        .iter()
        .any(|r| upper == *r || upper.starts_with(&format!("{r}.")))
    {
        result.push('_');
    }
    result
}

/// Mốc `yyyyMMdd_HHmmss` theo giờ Việt Nam (UTC+7, không đổi giờ mùa hè).
pub fn vietnam_stamp(unix_seconds: i64) -> String {
    let seconds = unix_seconds + i64::from(smks_core::consts::app::VIETNAM_UTC_OFFSET_SECONDS);
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        day_seconds / 3600,
        (day_seconds % 3600) / 60,
        day_seconds % 60,
    );
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}_{hour:02}{minute:02}{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(relative: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(relative)
            .display()
            .to_string()
    }

    fn request(pattern: &str, mappings: HashMap<String, FieldSource>) -> MergeRequest {
        MergeRequest {
            template_path: fixture("fixtures/merge/1_giay_moi_hop.docx"),
            data: DataSource::Table {
                path: fixture("fixtures/merge/1_giay_moi_hop.xlsx"),
                sheet: None,
                header_row: 1,
            },
            mappings,
            file_name_pattern: pattern.into(),
            format: OutputFormat::Pdf,
            output_folder: None,
        }
    }

    fn auto_mappings() -> HashMap<String, FieldSource> {
        let template = std::fs::read(fixture("fixtures/merge/1_giay_moi_hop.docx")).unwrap();
        let fields = crate::docx::read_fields(&template).unwrap();
        let table = read_table(
            Path::new(&fixture("fixtures/merge/1_giay_moi_hop.xlsx")),
            None,
            None,
            NumberLocale::EN_US,
        )
        .unwrap();
        auto_map(&fields, &table.columns)
            .into_iter()
            .map(|(f, c)| {
                (
                    f,
                    c.map_or(FieldSource::None, |column| FieldSource::Column { column }),
                )
            })
            .collect()
    }

    #[test]
    fn auto_map_matches_every_field_of_sample_1() {
        let mappings = auto_mappings();
        assert_eq!(mappings.len(), 11);
        assert!(
            mappings
                .values()
                .all(|s| matches!(s, FieldSource::Column { .. })),
            "{mappings:?}"
        );
    }

    #[test]
    fn prepares_sample_1_with_names_from_pattern() {
        let prepared = prepare(
            &request("{so_van_ban}_{ho_ten}", auto_mappings()),
            NumberLocale::EN_US,
            "20261118_120000",
        )
        .unwrap();
        assert_eq!(prepared.records.len(), 6);
        assert!(prepared.unmapped_fields.is_empty());
        assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);
        let first = &prepared.records[0];
        assert_eq!(first.row_number, 2);
        assert!(first.file_name.ends_with(".pdf"));
        assert!(first.file_name.is_ascii(), "{}", first.file_name);
        assert!(!first.values["ho_ten"].is_empty());
        assert!(
            prepared
                .output_folder
                .ends_with("1_giay_moi_hop_20261118_120000")
        );
    }

    #[test]
    fn unmapped_fixed_and_missing_columns() {
        let mut mappings = auto_mappings();
        mappings.insert("ho_ten".into(), FieldSource::None);
        mappings.insert(
            "don_vi".into(),
            FieldSource::Fixed {
                value: "Sở Nội vụ".into(),
            },
        );
        let prepared = prepare(
            &request("{ho_ten}", mappings.clone()),
            NumberLocale::EN_US,
            "x",
        )
        .unwrap();
        assert_eq!(prepared.unmapped_fields, ["ho_ten"]);
        assert!(prepared.warnings[0].starts_with("Có 1 biến chưa ghép nguồn"));
        assert!(
            prepared
                .records
                .iter()
                .all(|r| r.values["don_vi"] == "Sở Nội vụ" && r.values["ho_ten"].is_empty())
        );
        // Tên rỗng thì dùng vanban_<số dòng>.
        assert_eq!(prepared.records[0].file_name, "vanban_2.pdf");

        mappings.insert("ho_ten".into(), FieldSource::Column { column: 99 });
        assert_eq!(
            prepare(&request("x", mappings), NumberLocale::EN_US, "x")
                .unwrap_err()
                .code,
            ErrorCode::MergeMappingInvalid
        );
    }

    #[test]
    fn duplicate_names_get_counter_and_warning() {
        let prepared = prepare(
            &request("cung_ten", auto_mappings()),
            NumberLocale::EN_US,
            "x",
        )
        .unwrap();
        let names: Vec<&str> = prepared
            .records
            .iter()
            .map(|r| r.file_name.as_str())
            .collect();
        assert_eq!(names[0], "cung_ten.pdf");
        assert_eq!(names[1], "cung_ten (1).pdf");
        assert_eq!(names[5], "cung_ten (5).pdf");
        assert!(
            prepared.records[1]
                .warnings
                .iter()
                .any(|w| w.contains("trùng"))
        );
    }

    #[test]
    fn manual_entry_uses_template_folder() {
        let mut values = HashMap::new();
        values.insert("ho_ten".to_owned(), "Lê Văn C".to_owned());
        let mut req = request("{ho_ten}", HashMap::new());
        req.data = DataSource::Manual { values };
        req.format = OutputFormat::Docx;
        let prepared = prepare(&req, NumberLocale::EN_US, "s").unwrap();
        assert_eq!(prepared.records.len(), 1);
        assert_eq!(prepared.records[0].file_name, "Le_Van_C.docx");
        assert!(
            prepared.records[0]
                .warnings
                .iter()
                .any(|w| w.contains("so_van_ban"))
        );
    }

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_stem("  Nguyễn Văn Đức  "), "Nguyen_Van_Duc");
        assert_eq!(sanitize_file_stem("QĐ 01/2026: a*b?"), "QD_01-2026-_a-b-");
        assert_eq!(sanitize_file_stem("tên. "), "ten");
        assert_eq!(sanitize_file_stem("con"), "con_");
        assert_eq!(sanitize_file_stem(&"a".repeat(150)).len(), 100);
        assert_eq!(
            apply_pattern(
                "{so}_{ ho }-{x}{",
                &[("so".into(), "1".into()), ("ho".into(), "A".into())].into()
            ),
            "1_A-{"
        );
    }

    #[test]
    fn vietnam_stamp_is_utc_plus_7() {
        // 2026-11-18T07:17:57Z
        assert_eq!(vietnam_stamp(1_794_986_277), "20261118_141757");
        // 2026-11-18T18:30:00Z: đã qua nửa đêm theo giờ Việt Nam.
        assert_eq!(vietnam_stamp(1_795_026_600), "20261119_013000");
        // 2024-02-29T16:59:59Z: ngày 29/02 của năm nhuận, 23:59:59 giờ Việt Nam.
        assert_eq!(vietnam_stamp(1_709_225_999), "20240229_235959");
    }

    #[test]
    fn request_deserializes_from_ui_json() {
        let json = r#"{
            "templatePath": "a.docx",
            "data": { "kind": "table", "path": "b.xlsx", "sheet": null, "headerRow": 2 },
            "mappings": { "ho_ten": { "kind": "column", "column": 3 }, "x": { "kind": "fixed", "value": "1" }, "y": { "kind": "none" } },
            "fileNamePattern": "{ho_ten}", "format": "pdf", "outputFolder": null
        }"#;
        let parsed: MergeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.data,
            DataSource::Table {
                path: "b.xlsx".into(),
                sheet: None,
                header_row: 2
            }
        );
        assert_eq!(parsed.mappings["ho_ten"], FieldSource::Column { column: 3 });
        let manual: DataSource =
            serde_json::from_str(r#"{ "kind": "manual", "values": { "a": "1" } }"#).unwrap();
        assert!(matches!(manual, DataSource::Manual { .. }));
    }
}
