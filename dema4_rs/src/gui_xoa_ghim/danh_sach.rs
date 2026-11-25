//! Cột trái màn Xóa ghim (như XoaGhim): "Danh sách (chọn / tổng)", ô chọn tất cả, cột **Tên file** (bấm đổi chiều
//! sắp xếp) và **Trang** (đọc dần ở luồng nền), ô lọc tên; mỗi dòng: ô chọn chạy · chấm trạng thái · tên · số trang.

use eframe::egui::{self, Color32, RichText, Sense};

use super::ManXoaGhim;
use crate::gui::{CHU, CHU_PHU, DO, VIEN_NHAT, XANH_LA};
use crate::xoa_ghim::xuat::TrangThai;

const CAO_DONG: f32 = 22.0;
const NEN_CHON: Color32 = Color32::from_rgb(0xDC, 0xEB, 0xFA);

impl ManXoaGhim {
    /// Thứ tự hiện: tên tự nhiên (tăng / giảm), lọc theo tên (NFC, không phân biệt hoa thường).
    fn thu_tu_hien(&self) -> Vec<usize> {
        let goc = self.thu_muc.clone().unwrap_or_default();
        let ten = |i: usize| self.ds[i].0.strip_prefix(&goc).unwrap_or(&self.ds[i].0).to_string_lossy().into_owned();
        let loc = crate::tach_vb::nhan::nfc(&self.loc.trim().to_lowercase());
        let mut ds: Vec<usize> = (0..self.ds.len()).filter(|&i| loc.is_empty() || crate::tach_vb::nhan::nfc(&ten(i).to_lowercase()).contains(&loc)).collect();
        ds.sort_by(|&a, &b| crate::tach_vb::phien::so_ten_tu_nhien(&ten(a), &ten(b)));
        if self.sap_giam {
            ds.reverse();
        }
        ds
    }

    pub(super) fn danh_sach(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("xg_ds").resizable(true).default_size(300.0).size_range(220.0..=520.0).frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(6, 6))).show(ui, |ui| {
            let so_chon = self.chon_chay.iter().filter(|&&c| c).count();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Danh sách ({so_chon}/{})", self.ds.len())));
            });
            ui.separator();
            // Hàng tiêu đề: chọn tất cả · Tên file ↑↓ · Trang
            ui.horizontal(|ui| {
                let mut tat_ca = !self.chon_chay.is_empty() && self.chon_chay.iter().all(|&c| c);
                let r = ui.checkbox(&mut tat_ca, "");
                r.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, tat_ca, "Chọn mọi file"));
                if r.on_hover_text("Chọn / bỏ chọn mọi file").changed() {
                    self.chon_chay.iter_mut().for_each(|c| *c = tat_ca);
                }
                let chieu = if self.sap_giam { "Tên file ↓" } else { "Tên file ↑" };
                if ui.add(egui::Label::new(RichText::new(chieu).strong()).sense(Sense::click())).on_hover_text("Đổi chiều sắp xếp").clicked() {
                    self.sap_giam = !self.sap_giam;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Trang").strong());
                });
            });
            ui.add(egui::TextEdit::singleline(&mut self.loc).hint_text("Lọc tên file…").desired_width(f32::INFINITY));
            ui.add_space(2.0);
            let goc = self.thu_muc.clone().unwrap_or_default();
            let hien = self.thu_tu_hien();
            let mut chon = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, CAO_DONG, hien.len(), |ui, hang| {
                for i in hang.map(|k| hien[k]) {
                    let (o, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_DONG), Sense::click());
                    if i == self.chon {
                        ui.painter().rect_filled(o, 0.0, NEN_CHON);
                    } else if ph.hovered() {
                        ui.painter().rect_filled(o, 0.0, Color32::from_gray(0xF3));
                    }
                    ui.painter().hline(o.x_range(), o.bottom() - 0.5, egui::Stroke::new(1.0, VIEN_NHAT.gamma_multiply(0.5)));
                    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o.shrink2(egui::vec2(2.0, 1.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    let ten_file = self.ds[i].0.file_name().map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
                    if let Some(c) = self.chon_chay.get_mut(i) {
                        con.checkbox(c, "").widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, *c, format!("Chọn {ten_file}")));
                    }
                    let (mau, goi_y) = match self.trang_thai_file(i) {
                        None => (Color32::TRANSPARENT, "Đang đọc trạng thái…"),
                        Some(TrangThai::ChuaXoa) => (Color32::from_gray(0xC8), "Chưa xoá ghim"),
                        Some(TrangThai::DaXoa { dung_cau_hinh: true }) => (XANH_LA, "Đã xoá ghim (đúng vùng / tuỳ chọn hiện tại)"),
                        Some(TrangThai::DaXoa { dung_cau_hinh: false }) => (Color32::from_rgb(0xD9, 0x82, 0x2B), "Đã xoá nhưng vùng / tuỳ chọn đã đổi — sẽ chạy lại"),
                        Some(TrangThai::KetQuaMat) => (DO, "File kết quả không còn / đã bị thay — sẽ chạy lại"),
                    };
                    let (cham, _) = con.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
                    con.painter().circle_filled(cham.center(), 3.5, mau);
                    let ten = self.ds[i].0.strip_prefix(&goc).unwrap_or(&self.ds[i].0).display().to_string();
                    let so = self.so_trang_file.get(i).copied().flatten().map_or(String::new(), |n| n.to_string());
                    let rong_ten = (con.available_width() - 40.0).max(20.0);
                    con.allocate_ui_with_layout(egui::vec2(rong_ten, CAO_DONG - 2.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(&ten).color(CHU)).truncate().selectable(false));
                    });
                    con.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        ui.label(RichText::new(so).color(CHU_PHU));
                    });
                    if ph.on_hover_text(goi_y).clicked() {
                        chon = Some(i);
                    }
                }
            });
            if let Some(i) = chon {
                self.chon_file(i);
            }
        });
    }
}
