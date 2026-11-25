//! Tách văn bản: đọc mã vạch tờ index, đoán hướng trang scan, cắt PDF thành các văn bản.

pub mod anh;
pub mod cache_trang;
pub mod cat_pdf;
pub mod chi_so_trang;
pub mod doc_ma;
pub mod du_lieu_an;
pub mod dung_luong;
pub mod hang_doi;
pub mod huong_trang;
pub mod nhan;
pub mod phan_tich;
pub mod phien;
pub mod tach;
pub mod thong_ke_nguong;

/// Thư mục mẫu 1 (`temp/function2/mau`) cho các test vàng chạy tay (`cargo test -- --ignored`).
#[cfg(test)]
pub(crate) fn thu_muc_mau() -> Option<std::path::PathBuf> {
    std::env::var_os("DEMA4_MAU_TACH").map(std::path::PathBuf::from)
}
