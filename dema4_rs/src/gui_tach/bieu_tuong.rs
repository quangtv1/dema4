//! Icon nét 1 màu (kiểu Lucide) vẽ bằng `painter`: font Arial / font sẵn của egui không có ✓ ▾ ↻ ↶ ↷.

use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Vec2};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BieuTuong {
    /// Dấu ✓ (mục đang dùng trong menu).
    Dau,
    /// Tam giác ▾ của nút menu.
    MuiTenXuong,
    HoanTac,
    LamLai,
    XoayPhai,
    Xoay180,
    /// ⓘ (tooltip giải thích).
    ThongTin,
}

const NET: f32 = 1.75;

fn cung(c: Pos2, r: f32, tu: f32, den: f32) -> Vec<Pos2> {
    (0..=16).map(|i| tu + (den - tu) * i as f32 / 16.0).map(|a| c + r * Vec2::angled(a)).collect()
}

/// Đầu mũi tên tại `p` hướng theo `huong` (đã chuẩn hoá).
fn dau_mui_ten(p: Pos2, huong: Vec2, co: f32) -> Vec<Pos2> {
    let n = huong.rot90();
    vec![p - huong * co + n * co * 0.8, p, p - huong * co - n * co * 0.8]
}

/// Vẽ icon vừa khít `r` (vùng vuông, ~15 px).
pub fn ve(painter: &egui::Painter, r: Rect, bt: BieuTuong, mau: Color32) {
    let s = Stroke::new(NET, mau);
    let c = r.center();
    let k = r.width().min(r.height()) / 2.0;
    match bt {
        BieuTuong::Dau => {
            painter.add(Shape::line(vec![c + Vec2::new(-0.6, 0.0) * k, c + Vec2::new(-0.2, 0.45) * k, c + Vec2::new(0.65, -0.5) * k], s));
        }
        BieuTuong::MuiTenXuong => {
            painter.add(Shape::convex_polygon(vec![c + Vec2::new(-0.45, -0.2) * k, c + Vec2::new(0.45, -0.2) * k, c + Vec2::new(0.0, 0.3) * k], mau, Stroke::NONE));
        }
        BieuTuong::HoanTac | BieuTuong::LamLai => {
            // Mũi tên quay đầu: vạch ngang trên + nửa vòng bên phải (hoàn tác) / trái (làm lại)
            let lat = if bt == BieuTuong::HoanTac { 1.0 } else { -1.0 };
            let tam = c + Vec2::new(0.15 * lat, 0.1) * k;
            let r_cung = 0.45 * k;
            let (tu, den) = if lat > 0.0 { (-PI / 2.0, PI / 2.0) } else { (-PI / 2.0, -3.0 * PI / 2.0) };
            let mut diem = vec![tam + Vec2::new(-0.75 * lat * k, -r_cung)];
            diem.extend(cung(tam, r_cung, tu, den));
            diem.push(tam + Vec2::new(-0.3 * lat * k, r_cung));
            painter.add(Shape::line(diem, s));
            painter.add(Shape::line(dau_mui_ten(tam + Vec2::new(-0.75 * lat * k, -r_cung), Vec2::new(-lat, 0.0), 0.3 * k), s));
        }
        BieuTuong::ThongTin => {
            painter.circle_stroke(c, 0.85 * k, Stroke::new(1.2, mau));
            painter.circle_filled(c + Vec2::new(0.0, -0.4) * k, 0.12 * k, mau);
            painter.line_segment([c + Vec2::new(0.0, -0.12) * k, c + Vec2::new(0.0, 0.5) * k], Stroke::new(1.4, mau));
        }
        BieuTuong::XoayPhai | BieuTuong::Xoay180 => {
            // Vòng 3/4 theo chiều kim đồng hồ, mũi tên ở cuối
            let r_cung = 0.62 * k;
            let (tu, den) = (0.75 * PI, 2.2 * PI);
            painter.add(Shape::line(cung(c, r_cung, tu, den), s));
            let cuoi = c + r_cung * Vec2::angled(den);
            painter.add(Shape::line(dau_mui_ten(cuoi, Vec2::angled(den + PI / 2.0), 0.3 * k), s));
            if bt == BieuTuong::Xoay180 {
                painter.circle_filled(c, 0.16 * k, mau);
            }
        }
    }
}

/// Nút chỉ có icon (24 px), rê chuột đổi nền; tooltip hiện cả khi vô hiệu.
pub fn nut(ui: &mut egui::Ui, bt: BieuTuong, goi_y: &str, bat: bool) -> Response {
    let (r, phan_hoi) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), if bat { Sense::click() } else { Sense::hover() });
    let v = ui.visuals();
    if bat && phan_hoi.hovered() {
        ui.painter().rect_filled(r, 3.0, v.widgets.hovered.weak_bg_fill);
    }
    let mau = if bat { v.text_color() } else { v.weak_text_color() };
    ve(ui.painter(), r.shrink(4.5), bt, mau);
    phan_hoi.on_hover_text(goi_y)
}

/// Nút trong menu có dấu ✓ ở cuối dòng khi `dang_chon`.
pub fn muc_menu(ui: &mut egui::Ui, chu: impl Into<egui::WidgetText>, dang_chon: bool, rong: f32) -> Response {
    let r = ui.add(egui::Button::new(chu).min_size(egui::vec2(rong, 0.0)));
    if dang_chon {
        let o = Rect::from_center_size(egui::pos2(r.rect.right() - 12.0, r.rect.center().y), egui::vec2(12.0, 12.0));
        ve(ui.painter(), o, BieuTuong::Dau, crate::gui::XANH_LA);
    }
    r
}

/// ⓘ 14 px, rê chuột hiện giải thích.
pub fn thong_tin(ui: &mut egui::Ui, giai_thich: &str) -> Response {
    let (r, phan_hoi) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
    let mau = if phan_hoi.hovered() { crate::gui::XANH_COBALT } else { ui.visuals().weak_text_color() };
    ve(ui.painter(), r, BieuTuong::ThongTin, mau);
    phan_hoi.on_hover_text(giai_thich)
}
