//! Spike S4: đọc Excel bằng calamine theo đúng quy tắc của `ExcelReader.cs` bản .NET 1.4.0
//! (sheet đầu, dòng đầu của vùng có dữ liệu là tiêu đề, bỏ cột tiêu đề trống, bỏ dòng trống),
//! nhưng CHƯA áp định dạng số: giá trị số in theo cách mặc định của Rust.
//! Mục đích là xem calamine trả gì để biết phải tự định dạng những gì.
//!
//! Cách dùng: s4-calamine-read <file JSON kết quả> <file.xlsx>...

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("Cách dùng: s4-calamine-read <out.json> <file.xlsx>...".into());
    }

    let mut files = Vec::new();
    for path in &args[1..] {
        files.push(read_file(Path::new(path))?);
    }
    let output = json!({ "reader": "calamine 0.38.0", "culture": "-", "files": files });
    std::fs::write(&args[0], serde_json::to_string_pretty(&output)?)?;
    println!("Đã ghi {} ({} file).", args[0], files.len());
    Ok(())
}

fn read_file(path: &Path) -> Result<Value, Box<dyn Error>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let sheet = workbook.sheet_names().first().cloned().ok_or("File Excel không có sheet nào.")?;
    let range = workbook.worksheet_range(&sheet)?;
    let styles = workbook.worksheet_style(&sheet)?;

    let (start_row, start_col) = range.start().ok_or("File Excel không có dữ liệu.")?;
    let (height, width) = range.get_size();

    // Tiêu đề: dòng đầu của vùng có dữ liệu; cột có tiêu đề trống bị bỏ.
    let mut headers: Vec<(u32, String)> = Vec::new();
    for offset in 0..width {
        let name = render(range.get((0, offset)).unwrap_or(&Data::Empty));
        let name = name.trim();
        if !name.is_empty() {
            headers.push((start_col + offset as u32, name.to_owned()));
        }
    }

    let mut rows = Vec::new();
    for row_offset in 1..height {
        let row = start_row + row_offset as u32;
        let mut values = BTreeMap::new();
        let mut meta = BTreeMap::new();
        for (col, _) in &headers {
            let cell = range.get_value((row, *col)).unwrap_or(&Data::Empty);
            // Số cột xuất ra đánh từ 1 như ClosedXML.
            values.insert(col + 1, render(cell).trim().to_owned());
            let format = styles.get((row, *col)).and_then(|s| s.number_format.as_ref());
            meta.insert(
                col + 1,
                json!({
                    "type": kind(cell),
                    "numFmtId": format.and_then(|f| f.format_id),
                    "format": format.map(|f| f.format_code.clone()),
                }),
            );
        }
        if values.values().all(|v| v.trim().is_empty()) {
            continue;
        }
        rows.push(json!({ "rowNumber": row + 1, "values": values, "meta": meta }));
    }

    Ok(json!({
        "file": path.file_name().map(|n| n.to_string_lossy().into_owned()),
        "sheet": sheet,
        "headers": headers
            .iter()
            .map(|(col, name)| json!({ "columnNumber": col + 1, "name": name }))
            .collect::<Vec<_>>(),
        "rows": rows,
    }))
}

/// In giá trị ô theo cách thô nhất: chỉ riêng ngày được định dạng `dd/MM/yyyy` như bản cũ.
fn render(cell: &Data) -> String {
    match cell {
        Data::DateTime(dt) if dt.is_datetime() => {
            let (year, month, day, ..) = dt.to_ymd_hms_milli();
            format!("{day:02}/{month:02}/{year:04}")
        }
        other => other.to_string(),
    }
}

fn kind(cell: &Data) -> &'static str {
    match cell {
        Data::Int(_) => "Int",
        Data::Float(_) => "Float",
        Data::String(_) => "String",
        Data::Bool(_) => "Bool",
        Data::DateTime(dt) if dt.is_datetime() => "DateTime",
        Data::DateTime(_) => "Duration",
        Data::DateTimeIso(_) => "DateTimeIso",
        Data::DurationIso(_) => "DurationIso",
        Data::Error(_) => "Error",
        Data::Empty => "Empty",
    }
}
