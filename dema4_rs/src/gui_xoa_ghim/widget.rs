//! Widget dùng chung của màn Xóa ghim — vẽ đúng kiểu prototype: nút thường / nút link / nút biểu tượng + chữ,
//! ô đường dẫn chỉ đọc có ⊗ hoặc nhãn, thanh trượt rãnh mảnh + nút tròn, ô số kiểu DragValue, tooltip hiện ngay,
//! thông báo nhanh (toast).

use eframe::egui::{self, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui};

use super::bieu_tuong;
use super::thiet_ke as tk;

/// Họ phông riêng của màn Xóa ghim (Segoe UI trên Windows; nơi khác = phông giao diện chung).
pub const HO_FONT_XG: &str = "xoa_ghim";
pub const HO_FONT_XG_DAM: &str = "xoa_ghim_dam";

pub fn font(co: f32) -> FontId {
    FontId::new(co, egui::FontFamily::Name(HO_FONT_XG.into()))
}
pub fn font_dam(co: f32) -> FontId {
    FontId::new(co, egui::FontFamily::Name(HO_FONT_XG_DAM.into()))
}

/// Áp kiểu chữ + màu nút của màn cho `ui` (con của nó thừa hưởng).
pub fn ap_kieu(ui: &mut Ui) {
    let s = ui.style_mut();
    use egui::TextStyle;
    s.text_styles.insert(TextStyle::Body, font(tk::CHU_CO));
    s.text_styles.insert(TextStyle::Button, font(tk::CHU_CO));
    s.text_styles.insert(TextStyle::Small, font(tk::CHU_NHO));
    let v = &mut s.visuals;
    v.selection.bg_fill = tk::XANH;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, tk::VIEN);
    v.widgets.inactive.weak_bg_fill = tk::NEN_NUT;
    v.widgets.hovered.weak_bg_fill = tk::NEN_NUT_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, tk::VIEN);
    v.widgets.active.bg_stroke = Stroke::new(1.0, tk::VIEN);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
        w.corner_radius = CornerRadius::same(3);
    }
    v.widgets.inactive.fg_stroke.color = tk::CHU;
    v.widgets.noninteractive.fg_stroke.color = tk::CHU;
}

/// Nút thường cao 26 (`.btn`).
pub fn nut(ui: &mut Ui, chu: &str, duoc: bool) -> Response {
    ui.add_enabled(duoc, egui::Button::new(egui::RichText::new(chu).font(font(tk::CHU_CO))).min_size(egui::vec2(0.0, tk::CAO_NUT)))
}

/// Nút chính màu xanh (Áp dụng, Xong).
pub fn nut_xanh(ui: &mut Ui, chu: &str, duoc: bool) -> Response {
    ui.add_enabled(duoc, egui::Button::new(egui::RichText::new(chu).font(font_dam(tk::CHU_CO)).color(Color32::WHITE)).fill(tk::XANH).stroke(Stroke::new(1.0, tk::XANH)).min_size(egui::vec2(0.0, tk::CAO_NUT)))
}

/// Nút kiểu link (Xóa vùng vẽ, Hoàn tác): nền trắng, chữ xanh, viền xanh nhạt, cao 24.
pub fn nut_link(ui: &mut Ui, chu: &str, duoc: bool, xanh: bool) -> Response {
    let mau = if xanh { tk::XANH_DAM_CHU } else { tk::CHU };
    let vien = if xanh { tk::VIEN_NUT_LINK } else { tk::VIEN };
    let nen = if xanh { Color32::WHITE } else { tk::NEN_NUT };
    ui.add_enabled(duoc, egui::Button::new(egui::RichText::new(chu).font(font(tk::CHU_CO)).color(mau)).fill(nen).stroke(Stroke::new(1.0, vien)).min_size(egui::vec2(0.0, 24.0)))
}

/// Nút biểu tượng + chữ rộng cố định 88 × 26 (`.bnut`: File, Thư mục).
pub fn nut_bt_chu(ui: &mut Ui, bt: &str, kich: (f32, f32), chu: &str, duoc: bool) -> Response {
    let (o, ph) = ui.allocate_exact_size(egui::vec2(tk::RONG_BNUT, tk::CAO_NUT), if duoc { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(o) {
        let p = ui.painter();
        let nen = if duoc && ph.hovered() { tk::NEN_NUT_HOVER } else { tk::NEN_NUT };
        p.rect(o, 3.0, nen, Stroke::new(1.0, tk::VIEN), StrokeKind::Inside);
        let g = p.layout_no_wrap(chu.to_owned(), font(tk::CHU_CO), tk::CHU);
        let rong = kich.0 + 5.0 + g.size().x;
        let x0 = o.center().x - rong / 2.0;
        let mo = if duoc { 1.0 } else { 0.45 };
        let o_bt = Rect::from_center_size(egui::pos2(x0 + kich.0 / 2.0, o.center().y), egui::vec2(kich.0, kich.1));
        bieu_tuong::anh(bt, kich.0, kich.1).tint(Color32::WHITE.gamma_multiply(mo)).paint_at(ui, o_bt);
        p.galley(egui::pos2(x0 + kich.0 + 5.0, o.center().y - g.size().y / 2.0), g, tk::CHU.gamma_multiply(mo));
    }
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, duoc, chu));
    ph
}

/// Nút chỉ có biểu tượng (`.ibtn`), cỡ `rong × cao`; `dang` = đang bật (nền xanh nhạt).
pub fn nut_bt(ui: &mut Ui, bt: &str, kich: (f32, f32), rong: f32, cao: f32, duoc: bool, dang: bool, ten: &str) -> Response {
    let (o, ph) = ui.allocate_exact_size(egui::vec2(rong, cao), if duoc { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(o) {
        let p = ui.painter();
        let (nen, vien) = if dang { (tk::NEN_NUT_VUA_DANG, tk::VIEN_NUT_VUA_DANG) } else if duoc && ph.hovered() { (tk::NEN_NUT_HOVER, tk::VIEN) } else { (tk::NEN_NUT, tk::VIEN) };
        p.rect(o, 3.0, nen, Stroke::new(1.0, vien), StrokeKind::Inside);
        let mo = if duoc { 1.0 } else { 0.45 };
        bieu_tuong::anh(bt, kich.0, kich.1).tint(Color32::WHITE.gamma_multiply(mo)).paint_at(ui, Rect::from_center_size(o.center(), egui::vec2(kich.0, kich.1)));
    }
    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, duoc, dang, ten));
    ph
}

/// Phần cuối ô đường dẫn.
pub enum DuoiO<'a> {
    Khong,
    /// ⊗ tròn xám (rê: đỏ) — bấm để xoá / đóng.
    XoaTron(&'a str),
    /// Nhãn nhỏ (mặc định / ghi đè): (chữ, nền, màu chữ).
    Nhan(&'a str, Color32, Color32),
}

/// Ô đường dẫn chỉ đọc cao 26: chữ cắt "…", rê hiện đủ; trả `true` khi bấm ⊗.
pub fn o_duong_dan(ui: &mut Ui, rong: f32, chu: &str, goi_y: &str, day_du: &str, mau_chu: Color32, duoi: DuoiO, ten: &str) -> bool {
    let (o, ph) = ui.allocate_exact_size(egui::vec2(rong, tk::CAO_NUT), Sense::hover());
    let p = ui.painter().clone();
    p.rect(o, 3.0, Color32::WHITE, Stroke::new(1.0, tk::VIEN), StrokeKind::Inside);
    let mut phai = o.right() - 8.0;
    let mut bam_x = false;
    match duoi {
        DuoiO::Khong => {}
        DuoiO::XoaTron(tip) => {
            let c = egui::pos2(o.right() - 5.0 - 8.0, o.center().y);
            let r = Rect::from_center_size(c, egui::vec2(16.0, 16.0));
            let x = ui.interact(r, ui.scope_id().with((ten, "x")), Sense::click());
            p.circle_filled(c, 8.0, if x.hovered() { tk::XTRON_HOVER } else { tk::XTRON });
            let n = Stroke::new(1.5, Color32::WHITE);
            p.line_segment([c + egui::vec2(-2.5, -2.5), c + egui::vec2(2.5, 2.5)], n);
            p.line_segment([c + egui::vec2(2.5, -2.5), c + egui::vec2(-2.5, 2.5)], n);
            x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
            bam_x = x.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked();
            phai = r.left() - 4.0;
        }
        DuoiO::Nhan(t, nen, mau) => {
            let g = p.layout_no_wrap(t.to_owned(), font(10.5), mau);
            let r = Rect::from_min_size(egui::pos2(o.right() - 7.0 - g.size().x - 10.0, o.center().y - 8.0), egui::vec2(g.size().x + 10.0, 16.0));
            p.rect_filled(r, 3.0, nen);
            p.galley(r.center() - g.size() / 2.0, g, mau);
            phai = r.left() - 6.0;
        }
    }
    let (t, mau) = if chu.is_empty() { (goi_y, tk::CHU_MO) } else { (chu, mau_chu) };
    let mut job = egui::text::LayoutJob::simple_singleline(t.to_owned(), font(tk::CHU_CO), mau);
    job.wrap = egui::text::TextWrapping::truncate_at_width((phai - o.left() - 8.0).max(10.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(egui::pos2(o.left() + 8.0, o.center().y - g.size().y / 2.0), g, mau);
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, false, ten));
    if !day_du.is_empty() {
        ph.on_hover_text(day_du);
    }
    bam_x
}

/// Thanh trượt kiểu prototype: rãnh 4 px (phần đã kéo xanh), nút tròn Ø14 viền 2 xanh, rê phóng 1,15.
/// Trả response; `changed()` khi kéo / bấm đổi giá trị.
pub fn thanh_truot(ui: &mut Ui, gia_tri: &mut f32, min: f32, max: f32, rong: f32, duoc: bool, ten: &str) -> Response {
    let (o, mut ph) = ui.allocate_exact_size(egui::vec2(rong, 20.0), if duoc { Sense::click_and_drag() } else { Sense::hover() });
    let x0 = o.left() + 7.0;
    let x1 = o.right() - 7.0;
    if duoc && (ph.dragged() || ph.clicked()) {
        if let Some(q) = ph.interact_pointer_pos() {
            let t = ((q.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
            let moi = (min + t * (max - min)).round();
            if moi != *gia_tri {
                *gia_tri = moi;
                ph.mark_changed();
            }
        }
    }
    if ui.is_rect_visible(o) {
        let p = ui.painter();
        let t = ((*gia_tri - min) / (max - min).max(1e-6)).clamp(0.0, 1.0);
        let cx = x0 + t * (x1 - x0);
        let cy = o.center().y;
        let ray = Rect::from_min_max(egui::pos2(x0 - 2.0, cy - 2.0), egui::pos2(x1 + 2.0, cy + 2.0));
        if duoc {
            p.rect_filled(ray, 2.0, Color32::from_rgb(0xDF, 0xE3, 0xE8));
            p.rect_filled(Rect::from_min_max(ray.min, egui::pos2(cx, ray.max.y)), 2.0, tk::XANH_THANH);
        } else {
            p.rect_filled(ray, 2.0, Color32::from_rgb(0xEC, 0xEE, 0xF1));
        }
        let to = ui.ctx().animate_bool_with_time(ph.id.with("to"), duoc && (ph.hovered() || ph.dragged()), 0.08);
        let r = 7.0 * (1.0 + 0.15 * to);
        let c = egui::pos2(cx, cy);
        if duoc {
            p.circle_filled(c + egui::vec2(0.0, 1.0), r + 0.5, Color32::from_black_alpha(50));
        }
        p.circle(c, r - 1.0, Color32::WHITE, Stroke::new(2.0, if duoc { tk::XANH_THANH } else { Color32::from_rgb(0xC4, 0xC8, 0xCE) }));
    }
    ph.widget_info(|| egui::WidgetInfo::slider(duoc, *gia_tri as f64, ten));
    ph
}

/// Ô số 58 × 22 có đơn vị (`.num`) — gõ / kéo; trả `true` khi đổi.
pub fn o_so(ui: &mut Ui, gia_tri: &mut f32, min: f32, max: f32, don_vi: &str, duoc: bool, ten: &str) -> bool {
    let mut doi = false;
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
            w.bg_fill = Color32::from_rgb(0xF8, 0xF9, 0xFA);
            w.weak_bg_fill = Color32::from_rgb(0xF8, 0xF9, 0xFA);
            w.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xD0, 0xD4, 0xD9));
            w.corner_radius = CornerRadius::same(4);
        }
        v.widgets.active.bg_stroke = Stroke::new(1.0, tk::XANH_THANH);
        ui.spacing_mut().interact_size = egui::vec2(58.0, 22.0);
        let mut x = *gia_tri as f64;
        let r = ui.add_enabled(duoc, egui::DragValue::new(&mut x).range(min as f64..=max as f64).speed(0.25).max_decimals(0).suffix(don_vi));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, duoc, ten));
        if r.changed() && (x as f32 - *gia_tri).abs() >= 1.0 {
            *gia_tri = x.round() as f32;
            doi = true;
        }
    });
    doi
}

/// Tooltip hiện **ngay** dưới widget khi rê chuột (kể cả widget mờ) — nền tối, chữ trắng 11,5.
pub fn tooltip_ngay(ui: &Ui, ph: &Response, chu: &str) {
    if !ph.contains_pointer() {
        return;
    }
    egui::Area::new(ph.id.with("tooltip_ngay")).order(egui::Order::Tooltip).fixed_pos(ph.rect.left_bottom() + egui::vec2(0.0, 6.0)).interactable(false).show(ui.ctx(), |ui| {
        egui::Frame::new().fill(tk::NEN_TOOLTIP).corner_radius(4).inner_margin(egui::Margin::symmetric(8, 3)).show(ui, |ui| {
            ui.label(egui::RichText::new(chu).font(font(tk::CHU_NHO)).color(Color32::WHITE));
        });
    });
}

/// Thông báo nhanh đáy giữa màn (2,4 s).
pub fn toast(ctx: &egui::Context, chu: &str) {
    let man = ctx.content_rect();
    egui::Area::new(egui::Id::unique("xg_toast")).order(egui::Order::Foreground).pivot(egui::Align2::CENTER_BOTTOM).fixed_pos(egui::pos2(man.center().x, man.bottom() - 56.0)).interactable(false).show(ctx, |ui| {
        egui::Frame::new().fill(tk::NEN_TOOLTIP).corner_radius(6).inner_margin(egui::Margin::symmetric(14, 7)).show(ui, |ui| {
            ui.label(egui::RichText::new(chu).font(font(tk::CHU_CO)).color(Color32::WHITE));
        });
    });
}

/// Vạch dọc cao `cao` (giữa các khối thanh công cụ).
pub fn vach_doc(ui: &mut Ui, cao: f32) {
    let (o, _) = ui.allocate_exact_size(egui::vec2(1.0, cao), Sense::hover());
    ui.painter().vline(o.center().x, o.y_range(), Stroke::new(1.0, tk::VIEN_NHAT));
}

/// Ô tick + chữ (bấm cả chữ); `ghi_chu` = tooltip. Trả `true` khi đổi.
pub fn o_tick(ui: &mut Ui, gia_tri: &mut bool, chu: &str, duoc: bool, ghi_chu: &str) -> bool {
    let r = ui.add_enabled(duoc, egui::Checkbox::new(gia_tri, egui::RichText::new(chu).font(font(tk::CHU_CO)).color(if duoc { tk::CHU } else { tk::CHU_MO })));
    let doi = r.changed();
    if !ghi_chu.is_empty() {
        r.on_hover_text(ghi_chu);
    }
    doi
}
