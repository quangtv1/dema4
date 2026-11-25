//! Đọc dữ liệu cho Tạo văn bản từ Excel (`.xlsx`, `.xls`) hoặc CSV.
//!
//! Quy tắc giữ như `ExcelReader.cs` của bản .NET 1.4.0: dòng tiêu đề là dòng đầu của vùng có dữ liệu (nay
//! chọn được dòng khác), cột có tiêu đề trống bị bỏ, dòng trống hoàn toàn bị bỏ, ô ngày ra `dd/MM/yyyy`, ô
//! khác ra đúng chữ Excel hiển thị (đã cắt khoảng trắng hai đầu). Khác bản cũ: chọn được sheet (bản cũ luôn
//! đọc sheet đầu) và đọc được `.xls`, CSV.

use std::collections::HashSet;
use std::path::Path;

use calamine::{Data, Range, Reader, open_workbook_auto};
use serde::{Deserialize, Serialize};
use smks_core::{ErrorCode, Result, SmksError};
use unicode_normalization::UnicodeNormalization;

use crate::numfmt::{NumberLocale, format_general, format_number, format_time, has_date_part};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TableKind {
    Xlsx,
    Xls,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    /// Số cột thật trên sheet, đánh từ 1.
    pub column: u32,
    pub name: String,
    /// Số giá trị khác nhau (không tính ô trống).
    pub distinct: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    /// Số dòng thật trên sheet (hoặc số bản ghi trong CSV), đánh từ 1.
    pub row_number: u32,
    /// Cùng thứ tự với `Table::columns`.
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub path: String,
    pub file_name: String,
    pub kind: TableKind,
    pub sheets: Vec<String>,
    pub sheet: Option<String>,
    pub header_row: u32,
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

impl Table {
    /// Giá trị của một cột (theo số cột thật) ở một dòng.
    pub fn value<'a>(&self, row: &'a TableRow, column: u32) -> Option<&'a str> {
        let index = self.columns.iter().position(|c| c.column == column)?;
        row.values.get(index).map(String::as_str)
    }
}

fn invalid(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::MergeDataInvalid, message)
}

/// Loại file theo đuôi.
pub fn kind_of(path: &Path) -> Option<TableKind> {
    match path
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "xlsx" | "xlsm" => Some(TableKind::Xlsx),
        "xls" => Some(TableKind::Xls),
        "csv" => Some(TableKind::Csv),
        _ => None,
    }
}

/// Đọc file dữ liệu. `sheet` = None: sheet đầu tiên. `header_row` = None: dòng đầu tiên có dữ liệu.
pub fn read_table(
    path: &Path,
    sheet: Option<&str>,
    header_row: Option<u32>,
    locale: NumberLocale,
) -> Result<Table> {
    let kind = kind_of(path)
        .ok_or_else(|| invalid("File dữ liệu phải là Excel (.xlsx, .xls) hoặc CSV (.csv)."))?;
    if !path.is_file() {
        return Err(invalid(format!(
            "Không tìm thấy file dữ liệu: {}",
            path.display()
        )));
    }
    let (sheets, sheet, cells) = match kind {
        TableKind::Csv => (Vec::new(), None, read_csv_cells(path)?),
        TableKind::Xlsx | TableKind::Xls => read_excel_cells(path, sheet, locale)?,
    };
    let mut table = build_table(cells, header_row)?;
    table.path = path.display().to_string();
    table.file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    table.kind = kind;
    table.sheets = sheets;
    table.sheet = sheet;
    Ok(table)
}

/// Lưới ô đã định dạng: (số dòng, các ô (số cột, chữ)).
type Cells = Vec<(u32, Vec<(u32, String)>)>;

fn build_table(cells: Cells, header_row: Option<u32>) -> Result<Table> {
    let non_empty = |row: &(u32, Vec<(u32, String)>)| row.1.iter().any(|(_, v)| !v.is_empty());
    let header = match header_row {
        Some(n) => cells
            .iter()
            .find(|r| r.0 == n)
            .filter(|r| non_empty(r))
            .ok_or_else(|| {
                invalid(format!(
                    "Dòng tiêu đề {n} không có tên cột nào. Chọn lại dòng tiêu đề."
                ))
            })?,
        None => cells
            .iter()
            .find(|r| non_empty(r))
            .ok_or_else(|| invalid("File dữ liệu không có dữ liệu."))?,
    };
    let header_number = header.0;
    let mut columns: Vec<TableColumn> = header
        .1
        .iter()
        .filter(|(_, name)| !name.is_empty())
        .map(|(column, name)| TableColumn {
            column: *column,
            name: name.clone(),
            distinct: 0,
        })
        .collect();

    let mut rows = Vec::new();
    for (row_number, row_cells) in cells.iter().filter(|r| r.0 > header_number) {
        let values: Vec<String> = columns
            .iter()
            .map(|c| {
                row_cells
                    .iter()
                    .find(|(col, _)| *col == c.column)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            })
            .collect();
        if values.iter().all(String::is_empty) {
            continue;
        }
        rows.push(TableRow {
            row_number: *row_number,
            values,
        });
    }
    for (index, column) in columns.iter_mut().enumerate() {
        let distinct: HashSet<&str> = rows
            .iter()
            .map(|r| r.values[index].as_str())
            .filter(|v| !v.is_empty())
            .collect();
        column.distinct = distinct.len();
    }
    Ok(Table {
        path: String::new(),
        file_name: String::new(),
        kind: TableKind::Csv,
        sheets: Vec::new(),
        sheet: None,
        header_row: header_number,
        columns,
        rows,
    })
}

// ----------------------------------------------------------------------------------------------
// Excel
// ----------------------------------------------------------------------------------------------

fn read_excel_cells(
    path: &Path,
    sheet: Option<&str>,
    locale: NumberLocale,
) -> Result<(Vec<String>, Option<String>, Cells)> {
    let mut workbook =
        open_workbook_auto(path).map_err(|e| invalid(format!("Không đọc được file Excel: {e}")))?;
    let sheets = workbook.sheet_names();
    let name = match sheet {
        Some(wanted) => sheets
            .iter()
            .find(|s| s.as_str() == wanted)
            .cloned()
            .ok_or_else(|| {
                SmksError::new(
                    ErrorCode::TableSheetNotFound,
                    format!("File Excel không có sheet \"{wanted}\"."),
                )
            })?,
        None => sheets
            .first()
            .cloned()
            .ok_or_else(|| invalid("File Excel không có sheet nào."))?,
    };
    let range: Range<Data> = workbook
        .worksheet_range(&name)
        .map_err(|e| invalid(format!("Không đọc được sheet \"{name}\": {e}")))?;
    // Định dạng ô chỉ đọc được với .xlsx; .xls trả về rỗng và rơi về General.
    let styles = workbook.worksheet_style(&name).ok();

    let mut cells: Cells = Vec::new();
    if let (Some((start_row, start_col)), Some((end_row, end_col))) = (range.start(), range.end()) {
        for row in start_row..=end_row {
            let mut row_cells = Vec::new();
            for col in start_col..=end_col {
                let Some(data) = range.get_value((row, col)) else {
                    continue;
                };
                if matches!(data, Data::Empty) {
                    continue;
                }
                let format = styles
                    .as_ref()
                    .and_then(|s| s.get((row, col)))
                    .and_then(|s| s.number_format.as_ref())
                    .map(|f| f.format_code.as_str())
                    .unwrap_or("General");
                let text = render_cell(data, format, locale);
                row_cells.push((col + 1, text.trim().to_owned()));
            }
            cells.push((row + 1, row_cells));
        }
    }
    Ok((sheets, Some(name), cells))
}

/// Chữ hiển thị của một ô Excel.
pub fn render_cell(data: &Data, format: &str, locale: NumberLocale) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(text) => text.clone(),
        Data::Bool(value) => if *value { "TRUE" } else { "FALSE" }.to_owned(),
        Data::Error(error) => error.to_string(),
        Data::Int(value) => format_number(*value as f64, format, locale),
        Data::Float(value) => format_number(*value, format, locale),
        Data::DateTime(value) => {
            if value.is_datetime()
                && (format.eq_ignore_ascii_case("general") || has_date_part(format))
            {
                let (year, month, day, ..) = value.to_ymd_hms_milli();
                format!("{day:02}/{month:02}/{year:04}")
            } else {
                format_time(value.as_f64(), format)
            }
        }
        Data::DateTimeIso(text) => {
            // yyyy-mm-dd[Thh:mm:ss]
            let date = text.get(..10).unwrap_or(text);
            let parts: Vec<&str> = date.split('-').collect();
            if let [year, month, day] = parts.as_slice() {
                format!("{day}/{month}/{year}")
            } else {
                text.clone()
            }
        }
        Data::DurationIso(text) => text.clone(),
    }
}

/// Định dạng một số không có mã định dạng (dùng cho CSV và kiểm thử).
pub fn render_number(value: f64, locale: NumberLocale) -> String {
    format_general(value, locale)
}

// ----------------------------------------------------------------------------------------------
// CSV
// ----------------------------------------------------------------------------------------------

fn decode_csv(bytes: &[u8]) -> String {
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        encoding_rs::UTF_16LE.decode(bytes).0.into_owned()
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        encoding_rs::UTF_16BE.decode(bytes).0.into_owned()
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        text.to_owned()
    } else {
        // Excel tiếng Việt lưu CSV theo bảng mã ANSI Windows-1258 (dấu tổ hợp).
        encoding_rs::WINDOWS_1258.decode(bytes).0.into_owned()
    };
    text.nfc().collect()
}

fn detect_delimiter(text: &str) -> (u8, &str) {
    // Excel có thể ghi dòng đầu "sep=;".
    if let Some(first) = text.lines().next() {
        let trimmed = first.trim();
        if let Some(sep) = trimmed.strip_prefix("sep=")
            && let Some(byte) = sep.bytes().next()
        {
            let rest = text.split_once('\n').map_or("", |(_, r)| r);
            return (byte, rest);
        }
    }
    let first_line = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    let mut counts = [(b',', 0usize), (b';', 0), (b'\t', 0)];
    let mut in_quote = false;
    for byte in first_line.bytes() {
        if byte == b'"' {
            in_quote = !in_quote;
        } else if !in_quote {
            for (delimiter, count) in counts.iter_mut() {
                if byte == *delimiter {
                    *count += 1;
                }
            }
        }
    }
    let best = counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map_or(b',', |(d, c)| if *c > 0 { *d } else { b',' });
    (best, text)
}

fn read_csv_cells(path: &Path) -> Result<Cells> {
    let bytes =
        std::fs::read(path).map_err(|e| invalid(format!("Không đọc được file CSV: {e}")))?;
    let text = decode_csv(&bytes);
    let (delimiter, body) = detect_delimiter(&text);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(body.as_bytes());
    let mut cells: Cells = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| invalid(format!("File CSV sai định dạng: {e}")))?;
        let row: Vec<(u32, String)> = record
            .iter()
            .enumerate()
            .map(|(col, value)| (col as u32 + 1, value.trim().to_owned()))
            .collect();
        cells.push((index as u32 + 1, row));
    }
    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(relative)
    }

    /// So với kết quả ClosedXML 0.105.1 lưu ở spike S4 (`docs/spikes/evidence/S4/`).
    fn assert_matches_closedxml(evidence: &str, locale: NumberLocale) {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fixture(evidence)).unwrap()).unwrap();
        for file in json["files"].as_array().unwrap() {
            let name = file["file"].as_str().unwrap();
            let path = if name == "tricky.xlsx" {
                fixture("fixtures/table/tricky.xlsx")
            } else {
                fixture(&format!("fixtures/merge/{name}"))
            };
            let table = read_table(&path, None, None, locale).unwrap();
            assert_eq!(table.sheet.as_deref(), file["sheet"].as_str(), "{name}");
            let expected_columns: Vec<(u32, String)> = file["headers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| {
                    (
                        h["columnNumber"].as_u64().unwrap() as u32,
                        h["name"].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            let columns: Vec<(u32, String)> = table
                .columns
                .iter()
                .map(|c| (c.column, c.name.clone()))
                .collect();
            assert_eq!(columns, expected_columns, "{name}: tiêu đề");
            let expected_rows = file["rows"].as_array().unwrap();
            assert_eq!(table.rows.len(), expected_rows.len(), "{name}: số dòng");
            for (row, expected) in table.rows.iter().zip(expected_rows) {
                assert_eq!(
                    u64::from(row.row_number),
                    expected["rowNumber"].as_u64().unwrap(),
                    "{name}"
                );
                for (column, value) in expected["values"].as_object().unwrap() {
                    let column: u32 = column.parse().unwrap();
                    assert_eq!(
                        table.value(row, column).unwrap_or_default(),
                        value.as_str().unwrap(),
                        "{name}: dòng {} cột {column}",
                        row.row_number
                    );
                }
            }
        }
    }

    #[test]
    fn excel_matches_closedxml_en_us() {
        assert_matches_closedxml(
            "docs/spikes/evidence/S4/closedxml.en-US.json",
            NumberLocale::EN_US,
        );
    }

    #[test]
    fn excel_matches_closedxml_vi_vn() {
        assert_matches_closedxml(
            "docs/spikes/evidence/S4/closedxml.vi-VN.json",
            NumberLocale::VI_VN,
        );
    }

    #[test]
    fn header_row_and_sheet_can_be_chosen() {
        let path = fixture("fixtures/table/tricky.xlsx");
        let table = read_table(&path, Some("DuLieu"), Some(3), NumberLocale::EN_US).unwrap();
        assert_eq!(table.header_row, 3);
        assert_eq!(table.columns[0].name, "0001");
        assert_eq!(table.sheets, vec!["DuLieu".to_owned(), "Khac".to_owned()]);

        let other = read_table(&path, Some("Khac"), None, NumberLocale::EN_US).unwrap();
        assert_eq!(other.columns[0].name, "khong doc sheet nay");
        assert!(other.rows.is_empty());

        let missing = read_table(&path, Some("KhongCo"), None, NumberLocale::EN_US).unwrap_err();
        assert_eq!(missing.code, ErrorCode::TableSheetNotFound);
        let bad_header = read_table(&path, None, Some(5), NumberLocale::EN_US).unwrap_err();
        assert_eq!(bad_header.code, ErrorCode::MergeDataInvalid);
    }

    #[test]
    fn distinct_counts_ignore_empty_cells() {
        let table = read_table(
            &fixture("fixtures/table/tricky.xlsx"),
            None,
            None,
            NumberLocale::EN_US,
        )
        .unwrap();
        let merged = table.columns.iter().find(|c| c.name == "o_gop").unwrap();
        assert_eq!(merged.distinct, 2);
    }

    fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("smks-merge-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn csv_utf8_bom_semicolon() {
        let path = write_temp(
            "bom.csv",
            "\u{FEFF}Họ tên;Số\r\n Nguyễn Văn A ;0001\r\n;\r\nTrần B;\"1;2\"\r\n".as_bytes(),
        );
        let table = read_table(&path, None, None, NumberLocale::EN_US).unwrap();
        assert_eq!(table.kind, TableKind::Csv);
        assert_eq!(
            table
                .columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["Họ tên", "Số"]
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].values, ["Nguyễn Văn A", "0001"]);
        assert_eq!(table.rows[1].values, ["Trần B", "1;2"]);
        assert_eq!(table.rows[1].row_number, 4);
    }

    #[test]
    fn csv_windows_1258_and_sep_line() {
        // Windows-1258 có sẵn chữ ă â ê ô ơ ư đ nhưng dấu thanh là ký tự tổ hợp đứng sau, như Excel ghi ra.
        let source = "sep=,\nHo\u{323} tên,Đơn vi\u{323}\nĐô\u{303} Thi\u{323} Hươ\u{300}ng,Pho\u{300}ng Tô\u{309}ng hơ\u{323}p\n";
        let (bytes, _, unmappable) = encoding_rs::WINDOWS_1258.encode(source);
        assert!(!unmappable);
        let path = write_temp("cp1258.csv", &bytes);
        let table = read_table(&path, None, None, NumberLocale::EN_US).unwrap();
        assert_eq!(table.columns[1].name, "Đơn vị");
        assert_eq!(table.rows[0].values, ["Đỗ Thị Hường", "Phòng Tổng hợp"]);
    }

    #[test]
    fn rejects_unknown_extension() {
        let path = write_temp("x.txt", b"a");
        assert_eq!(
            read_table(&path, None, None, NumberLocale::EN_US)
                .unwrap_err()
                .code,
            ErrorCode::MergeDataInvalid
        );
    }
}
