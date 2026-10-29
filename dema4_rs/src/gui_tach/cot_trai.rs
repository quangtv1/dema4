//! Cột trái chung của tab Hồ sơ và Văn bản: dải đáy (ô tìm theo tên + nút thu gọn) và chế độ thu gọn chỉ còn cột #
//! (không cuộn tay, tự cuộn tới hồ sơ / văn bản đang xem). Mở app cột trái luôn mở rộng (không lưu cfg).

use eframe::egui::{self, Align, Color32, Rect, RichText, Sense, Stroke, StrokeKind};

use super::bieu_tuong::{self, BieuTuong};
use crate::gui::{CHU, CHU_PHU, VIEN, VIEN_NHAT, VIEN_TAB_CHON};

/// Bề ngang cột trái khi thu gọn.
pub const RONG_GON: f32 = 36.0;
pub const CAO_DAY: f32 = 30.0;
const NEN_DAY: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA);
/// Hàng hồ sơ chứa trang đang xem.
pub const NEN_DANG: Color32 = Color32::from_rgb(0xEE, 0xF4, 0xFB);

/// Tên khớp chuỗi tìm (không phân biệt hoa thường); chuỗi trống = khớp hết.
pub fn khop_tim(ten: &str, tim: &str) -> bool {
    let q = tim.trim().to_lowercase();
    q.is_empty() || ten.to_lowercase().contains(&q)
}

/// Dải đáy cột trái: ô tìm (chìm khi không dùng, nổi khi bấm vào / đang có chữ) + nút thu gọn / mở rộng.
pub fn day(ui: &mut egui::Ui, id: &str, tim: &mut String, goi_y: &str, gon: &mut bool) {
    egui::Panel::bottom(egui::Id::unique(id)).exact_size(CAO_DAY).frame(egui::Frame::new().fill(NEN_DAY).inner_margin(egui::Margin::symmetric(6, 0))).show(ui, |ui| {
        let vung = ui.max_rect();
        ui.painter().hline(vung.expand2(egui::vec2(6.0, 0.0)).x_range(), vung.top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if !*gon {
                o_tim(ui, id, tim, goi_y, ui.available_width() - 30.0);
            }
            let (bt, chu) = if *gon { (BieuTuong::MoRong, "Mở rộng cột trái") } else { (BieuTuong::ThuGon, "Thu gọn cột trái — mở rộng vùng xem") };
            if *gon {
                ui.add_space(((ui.available_width() - 24.0) / 2.0).max(0.0));
            }
            if bieu_tuong::nut(ui, bt, chu, true).clicked() {
                *gon = !*gon;
            }
        });
    });
}

fn o_tim(ui: &mut egui::Ui, id: &str, tim: &mut String, goi_y: &str, rong: f32) {
    let (vung, ph) = ui.allocate_exact_size(egui::vec2(rong.max(40.0), 22.0), Sense::click());
    let id_o = egui::Id::unique(id).with("o_tim");
    let dang_go = ui.memory(|m| m.has_focus(id_o));
    let co_chu = !tim.is_empty();
    let p = ui.painter();
    if dang_go {
        p.rect(vung, 3.0, Color32::WHITE, Stroke::new(1.0, VIEN_TAB_CHON), StrokeKind::Inside);
        p.rect_stroke(vung.expand(2.0), 4.0, Stroke::new(2.0, VIEN_TAB_CHON.gamma_multiply(0.18)), StrokeKind::Inside);
    } else if co_chu {
        p.rect(vung, 3.0, Color32::WHITE, Stroke::new(1.0, VIEN), StrokeKind::Inside);
    } else if ui.rect_contains_pointer(vung) {
        p.rect_stroke(vung, 3.0, Stroke::new(1.0, Color32::from_gray(0xE2)), StrokeKind::Inside);
    }
    let mau_icon = if dang_go || co_chu { Color32::from_gray(0x66) } else { Color32::from_gray(0xB0) };
    bieu_tuong::ve(p, Rect::from_center_size(egui::pos2(vung.left() + 12.0, vung.center().y), egui::vec2(13.0, 13.0)), BieuTuong::KinhLup, mau_icon);
    let o = egui::TextEdit::singleline(tim)
        .id(id_o)
        .frame(egui::Frame::NONE)
        .font(egui::FontId::proportional(12.0))
        .hint_text(RichText::new(goi_y).color(Color32::from_gray(0xB5)))
        .desired_width(vung.width() - 28.0)
        .align(egui::Align2::LEFT_CENTER);
    let r_o = ui.put(Rect::from_min_max(egui::pos2(vung.left() + 22.0, vung.top()), vung.max - egui::vec2(4.0, 0.0)), o);
    if ph.clicked() {
        r_o.request_focus();
    }
    // Esc xoá chuỗi tìm
    if r_o.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        tim.clear();
    }
    r_o.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SearchInput, true, goi_y));
}

/// Cột # khi thu gọn: số thứ tự canh giữa, không cuộn tay (chỉ tự cuộn tới `cuon_toi`). Trả hàng vừa bấm.
/// `chu(i)` = số hiển thị, `chon(i)` = hàng đang chọn (nền xanh), `dang(i)` = hồ sơ chứa trang đang xem (vạch xanh trái).
pub fn cot_so_gon(ui: &mut egui::Ui, id: &str, so_dong: usize, cao: f32, cao_dau: f32, chu: impl Fn(usize) -> String, goi_y: impl Fn(usize) -> String, chon: impl Fn(usize) -> bool, dang: impl Fn(usize) -> bool, cuon_toi: Option<usize>) -> Option<usize> {
    let mut bam = None;
    // Tiêu đề "#"
    let (dau, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), cao_dau), Sense::hover());
    ui.painter().rect_filled(dau, 0.0, crate::gui::NEN_TIEU_DE);
    ui.painter().hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    ui.painter().text(dau.center(), egui::Align2::CENTER_CENTER, "#", egui::FontId::proportional(12.5), CHU_PHU);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .scroll_source(egui::scroll_area::ScrollSource::NONE)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink(false)
        .show_viewport(ui, |ui, vp| {
            let rong = ui.available_width();
            ui.set_min_size(egui::vec2(rong, so_dong as f32 * cao));
            let goc = ui.max_rect().min;
            if let Some(i) = cuon_toi {
                ui.scroll_to_rect(Rect::from_min_size(goc + egui::vec2(0.0, i as f32 * cao), egui::vec2(rong, cao)), Some(Align::Center));
            }
            let dau_hang = (vp.min.y / cao).floor().max(0.0) as usize;
            let cuoi_hang = ((vp.max.y / cao).ceil() as usize).min(so_dong);
            for i in dau_hang..cuoi_hang {
                let r = Rect::from_min_size(goc + egui::vec2(0.0, i as f32 * cao), egui::vec2(rong, cao));
                let ph = ui.interact(r, egui::Id::unique(id).with(i), Sense::click()).on_hover_text(goi_y(i));
                let p = ui.painter();
                if chon(i) {
                    p.rect_filled(r, 0.0, crate::gui::NEN_TAB_CHON);
                } else if dang(i) {
                    p.rect_filled(r, 0.0, NEN_DANG);
                } else if ph.hovered() {
                    p.rect_filled(r, 0.0, Color32::from_rgb(0xF2, 0xF7, 0xFC));
                }
                if dang(i) {
                    p.rect_filled(Rect::from_min_size(r.min, egui::vec2(3.0, r.height())), 0.0, VIEN_TAB_CHON);
                }
                p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, super::VACH_DONG));
                let dam = chon(i) || dang(i);
                let font = if dam { egui::FontId::new(12.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(12.0) };
                p.text(r.center(), egui::Align2::CENTER_CENTER, chu(i), font, if dam { CHU } else { CHU_PHU });
                if ph.clicked() {
                    bam = Some(i);
                }
            }
        });
    bam
}

/// Dòng báo "không có kết quả" vẽ tràn qua các cột của hàng (ô bảng bị cắt theo bề ngang cột).
pub fn bao_khong_thay(ui: &egui::Ui, chu: &str) {
    let r = ui.max_rect();
    ui.ctx().layer_painter(ui.layer_id()).text(egui::pos2(r.left() + 8.0, r.center().y), egui::Align2::LEFT_CENTER, chu, egui::FontId::proportional(12.0), CHU_PHU);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tim_khong_phan_biet_hoa_thuong() {
        assert!(khop_tim("L3_10_18", ""));
        assert!(khop_tim("L3_10_18", " l3_1 "));
        assert!(!khop_tim("L2_10_18", "L3_1"));
        assert!(khop_tim("Hồ Sơ Đất", "sơ đ"));
    }
}
