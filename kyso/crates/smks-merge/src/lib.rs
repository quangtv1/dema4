//! Lõi Tạo văn bản của Sao Mai Ký số.
//!
//! - [`table`]: đọc dữ liệu Excel (`.xlsx`, `.xls`) và CSV, định dạng giá trị như Excel hiển thị.
//! - [`numfmt`]: áp mã định dạng số của Excel.
//! - [`docx`]: tìm và điền biến `{ten_bien}` trong mẫu Word (port `WordTemplateEngine.cs`).
//! - [`plan`]: ghép biến với nguồn, đặt tên file, kiểm tra trước khi tạo (port `DocumentMergeService.cs`).
//! - [`preview`]: xem trước nhanh (phiên Word giữ sẵn, bộ đệm theo nội dung, tạo trước dòng lân cận).
//! - [`text`]: chuẩn hoá chữ (bỏ dấu) dùng cho tự ghép và tên file.
//! - [`word`]: xuất DOCX ra PDF bằng Microsoft Word qua COM (chỉ Windows).

pub mod batch;
pub mod docx;
pub mod numfmt;
pub mod plan;
pub mod preview;
pub mod table;
pub mod text;
#[cfg(windows)]
pub mod word;
