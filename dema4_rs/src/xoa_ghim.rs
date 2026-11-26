//! Xóa ghim: xoá vết ghim / bấm lỗ / bóng mép trên trang scan theo vùng (góc, cạnh, vẽ tay),
//! giữ nét chữ đen, dấu đỏ / chữ ký xanh (tuỳ chọn) và vùng bảo vệ. Cùng thuật toán XoaGhim (Python).

pub mod ai;
mod hinh_thai;
pub mod kho;
pub mod lo;
pub mod mo_hinh;
pub mod vung;
pub mod xu_ly;
pub mod xuat;

#[cfg(test)]
mod doi_chieu;
