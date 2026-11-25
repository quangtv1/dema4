//! Lõi dùng chung của Sao Mai Ký số.
//!
//! Crate này chỉ chứa dữ liệu và logic thuần (không I/O, không FFI, không Tauri):
//!
//! - [`config`]: kiểu của `config.json`, mặc định, làm sạch giá trị, mẫu chữ ký theo khổ giấy.
//! - [`consts`]: hằng số mang từ bản .NET 1.4.0 (Phụ lục A của kế hoạch).
//! - [`error`]: mã lỗi nghiệp vụ và thông báo tiếng Việt (Phụ lục B).
//! - [`stamp_plan`]: quy tắc trang cho dấu đỏ, chữ ký tươi (port `StampPlanner.cs`).

pub mod config;
pub mod consts;
pub mod error;
pub mod stamp_plan;

pub use error::{ErrorCode, Result, SmksError};
