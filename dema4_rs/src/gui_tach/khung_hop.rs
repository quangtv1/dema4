//! Khung hộp thoại kiểu popup của prototype: dải tiêu đề xám, hàng tab gạch xanh, thân trắng cuộn, chân xám chứa nút.
//! Kèm các ô nhập / ô tick / nút radio vẽ tay theo kiểu trình duyệt (cỡ 12 px, cao 24 px).

use eframe::egui::{self, Color32, CornerRadius, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextWrapMode, Vec2};

use crate::gui::{ChuDam, CHU, CHU_MO, NEN, VIEN, VIEN_NHAT, VIEN_TAB_CHON};

/// Nền dải tiêu đề và hàng tab của hộp.
pub const NEN_DAU_HOP: Color32 = Color32::from_rgb(0xF3, 0xF3, 0xF3);
/// Chiều cao hộp ngoài phần thân cuộn (tiêu đề, tab, chân) + khoảng hở trên/dưới cửa sổ.
pub const PHAN_NGOAI_THAN: f32 = 150.0;
const VIEN_O_CHON: Color32 = Color32::from_rgb(0x76, 0x76, 0x76);
const BO_GOC: u8 = 6;

/// Hộp modal rộng `rong`: dải tiêu đề rồi tới `noi_dung` (hàng tab, thân, chân do nơi gọi vẽ).
pub fn hop<R>(ctx: &egui::Context, id: &'static str, rong: f32, tieu_de: impl Into<egui::WidgetText>, noi_dung: impl FnOnce(&mut egui::Ui) -> R) -> egui::ModalResponse<R> {
    let khung = egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN)).corner_radius(BO_GOC).shadow(egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(56) });
    egui::Modal::new(egui::Id::unique(id)).frame(khung).backdrop_color(Color32::from_black_alpha(64)).show(ctx, |ui| {
        ui.set_width(rong);
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        // Ô đang gõ: viền xanh như prototype
        ui.visuals_mut().selection.stroke = Stroke::new(1.0, VIEN_TAB_CHON);
        dai(ui, NEN_DAU_HOP, Margin { left: 16, right: 16, top: 11, bottom: 10 }, CornerRadius { nw: BO_GOC, ne: BO_GOC, sw: 0, se: 0 }, |ui| {
            ui.add(egui::Label::new(tieu_de).truncate());
        });
        ke_ngang(ui, VIEN_NHAT);
        noi_dung(ui)
    })
}

fn dai(ui: &mut egui::Ui, nen: Color32, le: Margin, goc: CornerRadius, f: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(nen).inner_margin(le).corner_radius(goc).show(ui, |ui| {
        ui.set_width(ui.available_width());
        f(ui);
    });
}

pub fn ke_ngang(ui: &mut egui::Ui, mau: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, mau);
}

/// Hàng tab nền xám: tab chọn chữ đậm + gạch xanh 2 px ở đáy.
pub fn hang_tab<T: PartialEq + Copy>(ui: &mut egui::Ui, ds: &[(T, &str)], chon: &mut T) {
    dai(ui, NEN_DAU_HOP, Margin::symmetric(16, 0), CornerRadius::ZERO, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for &(t, ten) in ds {
                let dang = *chon == t;
                let chu = RichText::new(ten).size(12.5);
                let g = egui::WidgetText::from(if dang { chu.dam() } else { chu }).into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body);
                let (r, p) = ui.allocate_exact_size(egui::vec2(g.size().x + 28.0, 30.0), Sense::click());
                if p.clicked() {
                    *chon = t;
                }
                if p.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                p.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, dang, ten));
                let pt = ui.painter();
                pt.galley(r.center() - g.size() / 2.0, g, CHU);
                if dang {
                    pt.rect_filled(Rect::from_min_max(egui::pos2(r.left(), r.bottom() - 2.0), r.max), 0.0, VIEN_TAB_CHON);
                }
            }
        });
    });
    ke_ngang(ui, VIEN_NHAT);
}

/// Thân hộp cao cố định `cao` (co lại khi cửa sổ thấp), lề trong `le`, nội dung dài thì cuộn.
pub fn than(ui: &mut egui::Ui, id: &str, cao: f32, le: Margin, f: impl FnOnce(&mut egui::Ui)) {
    let cao = cao.min(ui.ctx().content_rect().height() - PHAN_NGOAI_THAN).max(160.0);
    egui::ScrollArea::vertical().id_salt(id).max_height(cao).min_scrolled_height(cao).auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(le).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(cao - le.sum().y);
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().interact_size.y = 24.0;
            for (kieu, co) in [(egui::TextStyle::Body, 12.0), (egui::TextStyle::Button, 12.0)] {
                ui.style_mut().text_styles.insert(kieu, egui::FontId::proportional(co));
            }
            f(ui);
        });
    });
}

/// Chân hộp nền xám, viền trên; nút canh theo `f` (thường: trái → phải).
pub fn chan(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
    ke_ngang(ui, VIEN_NHAT);
    dai(ui, NEN, Margin::symmetric(16, 9), CornerRadius { nw: 0, ne: 0, sw: BO_GOC, se: BO_GOC }, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            f(ui);
        });
    });
}

/// Nút ở chân hộp: cao 24, rộng tối thiểu 64, chữ 12.
pub fn nut_chan(chu: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(chu.to_owned()).size(12.0)).min_size(egui::vec2(64.0, 24.0))
}

/// Nút chính ở chân hộp (nền màu, chữ trắng).
pub fn nut_chan_mau(chu: &str, nen: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(chu.to_owned()).size(12.0).color(Color32::WHITE)).fill(nen).stroke(Stroke::new(1.0, nen.gamma_multiply(0.8))).min_size(egui::vec2(64.0, 24.0))
}

/// Ô nhập 1 dòng kiểu prototype: chữ 12, cao 24, chữ giữa theo chiều dọc.
pub fn o_nhap(s: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(s).font(egui::FontId::proportional(12.0)).margin(Margin::symmetric(6, 4)).min_size(egui::vec2(0.0, 24.0)).align(egui::Align2::LEFT_CENTER)
}

/// Ô số: như `o_nhap`, chữ canh phải.
pub fn o_so(s: &mut String) -> egui::TextEdit<'_> {
    o_nhap(s).align(egui::Align2::RIGHT_CENTER)
}

/// Chữ cạnh ô tick / radio; mờ khi ô bị khoá.
fn chu_canh(ui: &egui::Ui, chu: &str) -> std::sync::Arc<egui::Galley> {
    let mau = if ui.is_enabled() { CHU } else { CHU_MO };
    egui::WidgetText::from(RichText::new(chu).size(12.0).color(mau)).into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body)
}

/// Vùng ô 13 px + chữ; trả (ô, phản hồi).
fn cap_cho(ui: &mut egui::Ui, g: &egui::Galley) -> (Rect, Response) {
    let rong = if g.size().x > 0.0 { 13.0 + 5.0 + g.size().x } else { 13.0 };
    let (r, p) = ui.allocate_exact_size(egui::vec2(rong, g.size().y.max(18.0)), Sense::click());
    (Rect::from_center_size(egui::pos2(r.left() + 6.5, r.center().y), Vec2::splat(13.0)), p)
}

/// Ô tick kiểu trình duyệt: bật = nền xanh, dấu ✓ trắng.
pub fn o_tick(ui: &mut egui::Ui, bat: &mut bool, chu: &str) -> Response {
    let g = chu_canh(ui, chu);
    let (o, mut p) = cap_cho(ui, &g);
    if p.clicked() {
        *bat = !*bat;
        p.mark_changed();
    }
    let gia_tri = *bat;
    p.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), gia_tri, chu));
    let mo = if ui.is_enabled() { 1.0 } else { 0.45 };
    let pt = ui.painter();
    if gia_tri {
        pt.rect_filled(o, 2.0, VIEN_TAB_CHON.gamma_multiply(mo));
        let d = |x: f32, y: f32| o.min + egui::vec2(x, y);
        pt.add(egui::Shape::line(vec![d(3.0, 6.8), d(5.6, 9.4), d(10.2, 3.8)], Stroke::new(1.8, Color32::WHITE)));
    } else {
        let vien = if p.hovered() { CHU } else { VIEN_O_CHON };
        pt.rect(o, 2.0, Color32::WHITE, Stroke::new(1.0, vien.gamma_multiply(mo)), StrokeKind::Inside);
    }
    pt.galley(egui::pos2(o.right() + 5.0, p.rect.center().y - g.size().y / 2.0), g, CHU);
    if p.hovered() && ui.is_enabled() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    p
}

/// Nút radio kiểu trình duyệt (màu nhấn xanh): chọn = vòng xanh + chấm xanh.
pub fn o_radio<T: PartialEq + Copy>(ui: &mut egui::Ui, hien_tai: &mut T, gia_tri: T, chu: &str) -> Response {
    let g = chu_canh(ui, chu);
    let (o, mut p) = cap_cho(ui, &g);
    if p.clicked() && *hien_tai != gia_tri {
        *hien_tai = gia_tri;
        p.mark_changed();
    }
    let chon = *hien_tai == gia_tri;
    p.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, ui.is_enabled(), chon, chu));
    let mo = if ui.is_enabled() { 1.0 } else { 0.45 };
    let (c, pt) = (o.center(), ui.painter());
    pt.circle_filled(c, 6.5, Color32::WHITE);
    if chon {
        pt.circle_stroke(c, 6.0, Stroke::new(1.5, VIEN_TAB_CHON.gamma_multiply(mo)));
        pt.circle_filled(c, 3.5, VIEN_TAB_CHON.gamma_multiply(mo));
    } else {
        let vien = if p.hovered() { CHU } else { VIEN_O_CHON };
        pt.circle_stroke(c, 6.0, Stroke::new(1.0, vien.gamma_multiply(mo)));
    }
    pt.galley(egui::pos2(o.right() + 5.0, p.rect.center().y - g.size().y / 2.0), g, CHU);
    if p.hovered() && ui.is_enabled() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    p
}

/// Tiêu đề hộp "chữ thường + **phần đậm**" cỡ 13.
pub fn tieu_de_dam(truoc: &str, dam: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(truoc, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(13.0), color: CHU, ..Default::default() });
    job.append(dam, 0.0, egui::TextFormat { font_id: egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), color: CHU, ..Default::default() });
    job
}

/// Thân popup gọn chỉ có 1 đoạn chữ 12 px, giãn dòng 1.6 (`*đậm*` như Hướng dẫn).
pub fn than_chu(ui: &mut egui::Ui, chu: &str) {
    egui::Frame::new().inner_margin(Margin::symmetric(16, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        crate::tro_giup::ve_doan(ui, chu, 12.0, CHU);
    });
}

/// Bảng 2 cột của popup gọn: nhãn xám 96 px | giá trị (đậm nếu `dam`; cỡ `co`), dòng cách 9 px.
pub fn bang_gon(ui: &mut egui::Ui, dong: &[(&str, &str, bool, f32)]) {
    egui::Frame::new().inner_margin(Margin::symmetric(16, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 9.0;
        // Dòng chỉ cao theo chữ (không theo chiều cao nút)
        ui.spacing_mut().interact_size.y = 0.0;
        for &(nhan, gia_tri, dam, co) in dong {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.allocate_ui_with_layout(egui::vec2(96.0, 14.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_width(96.0);
                    ui.label(RichText::new(nhan).size(12.0).color(crate::gui::CHU_PHU));
                });
                let chu = RichText::new(gia_tri).size(co);
                ui.add(egui::Label::new(if dam { chu.dam() } else { chu }).wrap());
            });
        }
    });
}

/// Dòng ghi chú nhỏ (11 px) dưới bảng popup gọn.
pub fn ghi_gon(ui: &mut egui::Ui, chu: &str, mau: Color32) {
    egui::Frame::new().inner_margin(Margin { left: 16, right: 16, top: 0, bottom: 10 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add(egui::Label::new(RichText::new(chu).size(11.0).color(mau)).wrap());
    });
}
