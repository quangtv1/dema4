//! Trang **Dữ liệu ứng dụng** của Cấu hình: thư mục dữ liệu (Mở thư mục), Xoá cache phân tích của thư mục Tách đang
//! mở, Dọn dữ liệu cũ — có hỏi lại.

use std::path::PathBuf;

/// Việc app phải làm sau khung hình (trang chỉ báo, app làm vì cần màn Tách / kho).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViecDuLieu {
    Khong,
    XoaCache,
    DonCu,
}

/// Trạng thái trang (tạo khi mở Cấu hình).
#[derive(Default)]
pub struct TrangDuLieu {
    /// Thư mục đang mở ở màn Tách (để hiện tên + tính dung lượng cache).
    pub thu_muc_tach: Option<PathBuf>,
    /// Thư mục đó còn cache phân tích.
    pub co_cache: bool,
}

impl TrangDuLieu {
    /// Vẽ thân trang trong cột nội dung; trả việc người dùng đã xác nhận.
    pub fn ve(&mut self, ui: &mut eframe::egui::Ui) -> ViecDuLieu {
        let _ = ui;
        ViecDuLieu::Khong
    }
}
