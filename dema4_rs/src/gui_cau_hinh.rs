//! Trang **Cấu hình** toàn cửa sổ (như prototype `plans/261126-1129-prototype-cau-hinh-toan-cua-so/`): sidebar chức
//! năng bên trái, cột nội dung cố định `RONG_COT`, nút Lưu trên tiêu đề, chế độ cửa sổ thu nhỏ.
//!
//! Module này giữ phần dùng chung cho thân các chức năng: thẻ (`the`), dòng `nhãn | ô` (`dong`), chữ gợi ý (`goi_y`).

#[allow(dead_code)]
pub mod du_lieu;

use eframe::egui::{self, Color32, Margin, RichText, Stroke};

use crate::gui::ChuDam;

/// Bề rộng cột nội dung — cố định cho mọi chức năng (cửa sổ thu nhỏ: hết bề ngang trừ lề).
pub const RONG_COT: f32 = 880.0;
/// Cột nhãn của dòng `nhãn | ô`.
pub const RONG_NHAN: f32 = 200.0;
/// Khoảng cách giữa các thẻ.
pub const KHE_THE: f32 = 14.0;

pub const XANH: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const XANH_CHU: Color32 = Color32::from_rgb(0x1F, 0x5F, 0xBF);
pub const CHU: Color32 = Color32::from_rgb(0x1F, 0x23, 0x28);
pub const CHU_PHU: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);
pub const NEN_TRANG: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);
pub const VIEN_THE: Color32 = Color32::from_rgb(0xE5, 0xE7, 0xEB);
pub const KE_DONG: Color32 = Color32::from_rgb(0xF0, 0xF1, 0xF3);
pub const CAM_CHU: Color32 = Color32::from_rgb(0xB4, 0x53, 0x09);
pub const CAM_NEN: Color32 = Color32::from_rgb(0xFF, 0xFB, 0xEB);
pub const CAM_VIEN: Color32 = Color32::from_rgb(0xFD, 0xE6, 0x8A);

/// Nhãn phạm vi lưu của thẻ (góc phải tiêu đề thẻ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pham {
    /// Lưu cùng vùng của thư mục đang mở (Xóa ghim).
    ThuMuc,
    /// Áp dụng cho mọi thư mục.
    Chung,
}

/// Thẻ nền trắng viền `#e5e7eb` bo 6: tiêu đề 13,5 đậm (+ ⓘ `giai_thich`, + nhãn `pham`), rồi các dòng.
pub fn the(ui: &mut egui::Ui, tieu_de: &str, giai_thich: Option<&str>, pham: Option<Pham>, noi_dung: impl FnOnce(&mut egui::Ui)) {
    the_tuy_bien(
        ui,
        |ui| {
            ui.label(RichText::new(tieu_de).size(13.5).color(CHU).dam());
            if let Some(g) = giai_thich {
                crate::gui_tach::bieu_tuong::thong_tin(ui, g);
            }
            if let Some(p) = pham {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| nhan_pham(ui, p));
            }
        },
        noi_dung,
    );
}

/// Như `the` nhưng dòng tiêu đề do nơi gọi vẽ (biểu tượng, vòng số bước, liên kết bên phải…); khung, lề, khoảng cách giống nhau.
pub fn the_tuy_bien(ui: &mut egui::Ui, tieu_de: impl FnOnce(&mut egui::Ui), noi_dung: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN_THE)).corner_radius(6).inner_margin(Margin { left: 16, right: 16, top: 12, bottom: 8 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            tieu_de(ui);
        });
        ui.add_space(6.0);
        ui.spacing_mut().interact_size.y = 26.0;
        noi_dung(ui);
    });
    ui.add_space(KHE_THE);
}

/// Nhãn "theo thư mục" / "chung" bo tròn.
pub fn nhan_pham(ui: &mut egui::Ui, p: Pham) {
    let (chu, nen, mau, tip) = match p {
        Pham::ThuMuc => ("theo thư mục", Color32::from_rgb(0xEF, 0xF6, 0xFF), Color32::from_rgb(0x1D, 0x4E, 0xD8), "Lưu cùng vùng của thư mục đang mở"),
        Pham::Chung => ("chung", Color32::from_rgb(0xF1, 0xF5, 0xF9), Color32::from_rgb(0x47, 0x55, 0x69), "Áp dụng cho mọi thư mục"),
    };
    let g = ui.painter().layout_no_wrap(chu.to_string(), egui::FontId::proportional(11.0), mau);
    let (o, ph) = ui.allocate_exact_size(g.size() + egui::vec2(16.0, 4.0), egui::Sense::hover());
    ui.painter().rect_filled(o, 9.0, nen);
    ui.painter().galley(o.min + egui::vec2(8.0, 2.0), g, mau);
    ph.on_hover_text(tip);
}

/// Dòng `nhãn | ô` (lưới 200 px + phần còn lại), kẻ mảnh phía trên (trừ dòng đầu thẻ: `dau = true`).
pub fn dong(ui: &mut egui::Ui, dau: bool, nhan: impl FnOnce(&mut egui::Ui), o: impl FnOnce(&mut egui::Ui)) {
    if !dau {
        let y = ui.cursor().top();
        ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, KE_DONG));
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        // Ô bắt đầu đúng cột `RONG_NHAN + 12` (thẳng với chữ gợi ý bên dưới)
        let khe = std::mem::replace(&mut ui.spacing_mut().item_spacing.x, 0.0);
        ui.allocate_ui_with_layout(egui::vec2(RONG_NHAN, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(RONG_NHAN);
            ui.spacing_mut().item_spacing.x = 6.0;
            nhan(ui);
        });
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.x = khe;
        o(ui);
    });
    ui.add_space(8.0);
}

/// Chữ gợi ý 11,5 xám dưới dòng, thẳng cột ô (sau cột nhãn).
pub fn goi_y(ui: &mut egui::Ui, chu: &str, mau: Option<Color32>) {
    ui.add_space(-4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(RONG_NHAN + 12.0);
        ui.add(egui::Label::new(RichText::new(chu).size(11.5).color(mau.unwrap_or(CHU_PHU))).wrap());
    });
    ui.add_space(8.0);
}
