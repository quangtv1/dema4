//! Bóc tách dữ liệu: đọc ảnh trang PDF bằng mô hình đọc ảnh (máy chủ nội bộ hoặc dịch vụ đám mây) → các trường
//! của loại tài liệu (số ký hiệu, ngày, trích yếu…) → ghi Excel / Google Sheet, dùng làm biên mục cho Sinh mục lục.

pub mod anh;
pub mod cau_lenh;
pub mod chay;
pub mod chuan_hoa;
pub mod doc_json;
pub mod khoa_cache;
pub mod loai_tai_lieu;
pub mod mo_hinh;
