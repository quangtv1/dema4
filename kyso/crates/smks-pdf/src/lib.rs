//! Lõi PDF của Sao Mai Ký số (kế hoạch mục 7).
//!
//! - [`reader`]: đọc cấu trúc, cây trang, kiểm điều kiện ký nối bản.
//! - [`geometry`]: vị trí khối chữ ký, trang xoay, khổ giấy.
//! - [`appearance`]: khối chữ (subset font tiếng Việt) và ảnh dấu.
//! - [`incremental`]: ghi nối bản một chữ ký (field, widget, ảnh dấu, xref).
//! - [`sigscan`]: tìm chữ ký có sẵn trong file.
//! - [`revdiff`]: so bản đã ký với file hiện tại (sửa sau khi ký).
//! - [`object`]: ghi object PDF cho phần nối thêm.

pub mod appearance;
pub mod geometry;
pub mod incremental;
pub mod object;
pub mod reader;
pub mod revdiff;
pub mod sigscan;
