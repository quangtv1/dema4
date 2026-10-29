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
    /// Menu Chức năng / Trợ giúp
    TaiLieu,
    Keo,
    But,
    HoiDap,
    /// Tab Văn bản / hộp phóng to
    TraiChevron,
    PhaiChevron,
    Luoi,
    MotTrang,
    HaiTrang,
    BoKhoi,
    Dong,
    /// Thanh công cụ: mũi ▾ nét của nút thả xuống, kính lúp thu nhỏ / phóng to, sắp xếp ⇅
    ChevronXuong,
    KinhTru,
    KinhCong,
    SapXep,
    /// Icon nét mảnh theo prototype (toạ độ lưới 16 × 16 của SVG)
    BanhRang,
    ChongTrang,
    MaVach,
    TrangTrong,
    TrangHoi,
    DanhSachSo,
    TangDan,
    GiamDan,
    ThungRac,
    TrangGach,
    ThuGon,
    MoRong,
    KinhLup,
}

/// Điểm trong lưới 16 × 16 (như `viewBox="0 0 16 16"` của prototype) → toạ độ màn hình trong `r`.
fn p16(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x / 16.0 * r.width(), y / 16.0 * r.height())
}

/// Cung tròn trong lưới 16: tâm (cx, cy), bán kính rr, từ góc `tu` tới `den` (radian).
fn cung16(r: Rect, cx: f32, cy: f32, rr: f32, tu: f32, den: f32) -> Vec<Pos2> {
    (0..=12).map(|i| tu + (den - tu) * i as f32 / 12.0).map(|a| p16(r, cx + rr * a.cos(), cy + rr * a.sin())).collect()
}

/// Trang giấy góc gấp phải trên (khung chung của các icon trang).
fn trang_giay(painter: &egui::Painter, r: Rect, s: Stroke) {
    painter.add(Shape::line(vec![p16(r, 9.5, 1.5), p16(r, 3.5, 1.5), p16(r, 3.5, 14.5), p16(r, 12.5, 14.5), p16(r, 12.5, 4.5), p16(r, 9.5, 1.5)], s));
    painter.add(Shape::line(vec![p16(r, 9.5, 1.5), p16(r, 9.5, 4.5), p16(r, 12.5, 4.5)], s));
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
            let s = Stroke::new(1.2, mau);
            let lat = |x: f32| if bt == BieuTuong::HoanTac { x } else { 16.0 - x };
            painter.add(Shape::line(vec![p16(r, lat(6.0), 9.5), p16(r, lat(2.5), 6.0), p16(r, lat(6.0), 2.5)], s));
            let mut diem = vec![p16(r, lat(2.5), 6.0), p16(r, lat(10.0), 6.0)];
            let (tu, den) = if bt == BieuTuong::HoanTac { (-PI / 2.0, PI / 2.0) } else { (-PI / 2.0, -3.0 * PI / 2.0) };
            diem.extend(cung16(r, lat(10.0), 9.5, 3.5, tu, den));
            diem.push(p16(r, lat(8.0), 13.0));
            painter.add(Shape::line(diem, s));
        }
        BieuTuong::ThongTin => {
            painter.circle_stroke(c, 0.85 * k, Stroke::new(1.2, mau));
            painter.circle_filled(c + Vec2::new(0.0, -0.4) * k, 0.12 * k, mau);
            painter.line_segment([c + Vec2::new(0.0, -0.12) * k, c + Vec2::new(0.0, 0.5) * k], Stroke::new(1.4, mau));
        }
        BieuTuong::TaiLieu => {
            let (w, h) = (0.62 * k, 0.8 * k);
            let goc = 0.3 * k;
            let diem = vec![c + Vec2::new(-w, -h), c + Vec2::new(w - goc, -h), c + Vec2::new(w, -h + goc), c + Vec2::new(w, h), c + Vec2::new(-w, h), c + Vec2::new(-w, -h)];
            painter.add(Shape::line(diem, s));
            for y in [-0.05, 0.3] {
                painter.line_segment([c + Vec2::new(-0.35, y) * k, c + Vec2::new(0.35, y) * k], s);
            }
        }
        BieuTuong::Keo => {
            for y in [-0.45, 0.45] {
                painter.circle_stroke(c + Vec2::new(-0.5, y) * k, 0.25 * k, s);
            }
            painter.line_segment([c + Vec2::new(-0.3, -0.3) * k, c + Vec2::new(0.8, 0.45) * k], s);
            painter.line_segment([c + Vec2::new(-0.3, 0.3) * k, c + Vec2::new(0.8, -0.45) * k], s);
        }
        BieuTuong::But => {
            painter.line_segment([c + Vec2::new(-0.6, 0.55) * k, c + Vec2::new(0.45, -0.5) * k], Stroke::new(NET * 1.3, mau));
            painter.line_segment([c + Vec2::new(-0.75, 0.8) * k, c + Vec2::new(-0.6, 0.55) * k], s);
            painter.line_segment([c + Vec2::new(0.0, 0.85) * k, c + Vec2::new(0.85, 0.85) * k], s);
        }
        BieuTuong::HoiDap => {
            painter.circle_stroke(c, 0.85 * k, Stroke::new(1.2, mau));
            painter.text(c + Vec2::new(0.0, 0.05 * k), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(1.2 * k), mau);
        }
        BieuTuong::TraiChevron | BieuTuong::PhaiChevron => {
            let l = if bt == BieuTuong::TraiChevron { 1.0 } else { -1.0 };
            painter.add(Shape::line(vec![c + Vec2::new(0.25 * l, -0.6) * k, c + Vec2::new(-0.25 * l, 0.0) * k, c + Vec2::new(0.25 * l, 0.6) * k], s));
        }
        BieuTuong::Luoi => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                painter.rect_stroke(Rect::from_center_size(c + Vec2::new(x, y) * 0.42 * k, Vec2::splat(0.62 * k)), 1.0, s, egui::StrokeKind::Middle);
            }
        }
        BieuTuong::MotTrang => {
            painter.rect_stroke(Rect::from_center_size(c, Vec2::new(1.1, 1.6) * k), 1.0, s, egui::StrokeKind::Middle);
        }
        BieuTuong::HaiTrang => {
            for x in [-0.45, 0.45] {
                painter.rect_stroke(Rect::from_center_size(c + Vec2::new(x, 0.0) * k, Vec2::new(0.75, 1.5) * k), 1.0, s, egui::StrokeKind::Middle);
            }
        }
        BieuTuong::BoKhoi => {
            // Trang có dấu trừ: bỏ trang khỏi văn bản
            painter.rect_stroke(Rect::from_center_size(c + Vec2::new(-0.15, 0.0) * k, Vec2::new(1.0, 1.5) * k), 1.0, s, egui::StrokeKind::Middle);
            painter.circle_filled(c + Vec2::new(0.45, 0.45) * k, 0.42 * k, painter.ctx().global_style().visuals.window_fill);
            painter.line_segment([c + Vec2::new(0.22, 0.45) * k, c + Vec2::new(0.68, 0.45) * k], Stroke::new(NET * 1.2, mau));
        }
        BieuTuong::Dong => {
            painter.line_segment([c + Vec2::new(-0.55, -0.55) * k, c + Vec2::new(0.55, 0.55) * k], s);
            painter.line_segment([c + Vec2::new(0.55, -0.55) * k, c + Vec2::new(-0.55, 0.55) * k], s);
        }
        BieuTuong::ChevronXuong => {
            painter.add(Shape::line(vec![c + Vec2::new(-0.45, -0.22) * k, c + Vec2::new(0.0, 0.22) * k, c + Vec2::new(0.45, -0.22) * k], Stroke::new(1.3, mau)));
        }
        BieuTuong::KinhTru | BieuTuong::KinhCong => {
            let s = Stroke::new(1.3, mau);
            let tam = c + Vec2::new(-0.15, -0.15) * k;
            painter.circle_stroke(tam, 0.58 * k, s);
            painter.line_segment([c + Vec2::new(0.3, 0.3) * k, c + Vec2::new(0.85, 0.85) * k], s);
            painter.line_segment([tam - Vec2::new(0.3 * k, 0.0), tam + Vec2::new(0.3 * k, 0.0)], s);
            if bt == BieuTuong::KinhCong {
                painter.line_segment([tam - Vec2::new(0.0, 0.3 * k), tam + Vec2::new(0.0, 0.3 * k)], s);
            }
        }
        BieuTuong::SapXep => {
            let s = Stroke::new(1.3, mau);
            painter.line_segment([c + Vec2::new(-0.4, -0.75) * k, c + Vec2::new(-0.4, 0.75) * k], s);
            painter.add(Shape::line(dau_mui_ten(c + Vec2::new(-0.4, 0.75) * k, Vec2::new(0.0, 1.0), 0.32 * k), s));
            painter.line_segment([c + Vec2::new(0.4, 0.75) * k, c + Vec2::new(0.4, -0.75) * k], s);
            painter.add(Shape::line(dau_mui_ten(c + Vec2::new(0.4, -0.75) * k, Vec2::new(0.0, -1.0), 0.32 * k), s));
        }
        BieuTuong::BanhRang => {
            // Bánh răng: vòng ngoài + 8 răng + lỗ giữa
            let s = Stroke::new(1.3, mau);
            painter.circle_stroke(c, 0.55 * k, s);
            painter.circle_stroke(c, 0.22 * k, s);
            for i in 0..8 {
                let a = i as f32 * PI / 4.0;
                painter.line_segment([c + 0.55 * k * Vec2::angled(a), c + 0.85 * k * Vec2::angled(a)], Stroke::new(2.2, mau));
            }
        }
        BieuTuong::ChongTrang => {
            let s = Stroke::new(1.25, mau);
            painter.add(Shape::line(vec![p16(r, 10.0, 1.5), p16(r, 6.0, 1.5), p16(r, 6.0, 11.5), p16(r, 13.0, 11.5), p16(r, 13.0, 4.5), p16(r, 10.0, 1.5)], s));
            painter.add(Shape::line(vec![p16(r, 3.5, 4.5), p16(r, 3.5, 14.5), p16(r, 10.0, 14.5)], s));
        }
        BieuTuong::MaVach => {
            for (x, w) in [(2.5, 1.2), (4.8, 0.8), (6.5, 1.6), (9.0, 0.8), (10.8, 1.2), (13.0, 0.8)] {
                painter.line_segment([p16(r, x, 3.0), p16(r, x, 13.0)], Stroke::new(w * r.width() / 16.0, mau));
            }
        }
        BieuTuong::TrangTrong => trang_giay(painter, r, Stroke::new(1.25, mau)),
        BieuTuong::TrangHoi => {
            trang_giay(painter, r, Stroke::new(1.25, mau));
            painter.text(p16(r, 8.0, 9.5), egui::Align2::CENTER_CENTER, "?", egui::FontId::new(0.5 * r.height(), egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), mau);
        }
        BieuTuong::TrangGach => {
            let s = Stroke::new(1.25, mau);
            trang_giay(painter, r, s);
            painter.line_segment([p16(r, 2.0, 14.0), p16(r, 14.0, 2.0)], s);
        }
        BieuTuong::DanhSachSo => {
            let s = Stroke::new(1.25, mau);
            for (i, y) in [4.0, 8.0, 12.0].into_iter().enumerate() {
                painter.line_segment([p16(r, 7.0, y), p16(r, 14.0, y)], s);
                painter.text(p16(r, 3.5, y), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::proportional(0.32 * r.height()), mau);
            }
        }
        BieuTuong::TangDan | BieuTuong::GiamDan => {
            let s = Stroke::new(1.25, mau);
            let tang = bt == BieuTuong::TangDan;
            painter.line_segment([p16(r, 4.0, 2.5), p16(r, 4.0, 13.5)], s);
            let dinh = if tang { p16(r, 4.0, 2.5) } else { p16(r, 4.0, 13.5) };
            painter.add(Shape::line(dau_mui_ten(dinh, Vec2::new(0.0, if tang { -1.0 } else { 1.0 }), 0.3 * k), s));
            let dai = if tang { [3.0, 5.0, 7.0] } else { [7.0, 5.0, 3.0] };
            for (y, d) in [4.0, 8.0, 12.0].into_iter().zip(dai) {
                painter.line_segment([p16(r, 7.5, y), p16(r, 7.5 + d, y)], s);
            }
        }
        BieuTuong::ThungRac => {
            let s = Stroke::new(1.25, mau);
            painter.line_segment([p16(r, 2.5, 4.0), p16(r, 13.5, 4.0)], s);
            painter.add(Shape::line(vec![p16(r, 6.0, 4.0), p16(r, 6.0, 2.5), p16(r, 10.0, 2.5), p16(r, 10.0, 4.0)], s));
            painter.add(Shape::line(vec![p16(r, 3.8, 4.0), p16(r, 4.5, 13.5), p16(r, 11.5, 13.5), p16(r, 12.2, 4.0)], s));
            for x in [6.5, 9.5] {
                painter.line_segment([p16(r, x, 6.5), p16(r, x, 11.0)], s);
            }
        }
        BieuTuong::ThuGon | BieuTuong::MoRong => {
            let s = Stroke::new(1.4, mau);
            painter.line_segment([p16(r, 3.0, 2.5), p16(r, 3.0, 13.5)], s);
            let diem = if bt == BieuTuong::ThuGon { [(11.0, 5.0), (8.0, 8.0), (11.0, 11.0)] } else { [(7.0, 5.0), (10.0, 8.0), (7.0, 11.0)] };
            painter.add(Shape::line(diem.iter().map(|&(x, y)| p16(r, x, y)).collect(), s));
        }
        BieuTuong::KinhLup => {
            let s = Stroke::new(1.4, mau);
            painter.circle_stroke(p16(r, 7.0, 7.0), 4.5 / 16.0 * r.width(), s);
            painter.line_segment([p16(r, 10.5, 10.5), p16(r, 14.0, 14.0)], s);
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

/// Icon làm 1 phần tử của `Button` (`Button::new((atom(..), "chữ"))`): nhãn truy cập vẫn chỉ là chữ.
pub fn atom(bt: BieuTuong, co: f32) -> egui::Atom<'static> {
    egui::Atom::paint(Vec2::splat(co), move |ui, a| ve(ui.painter(), a.rect, bt, a.fallback_text_color))
}

/// Icon màu cố định làm phần tử của `Button` (vd icon xám đầu mục thanh menu).
pub fn atom_mau(bt: BieuTuong, co: f32, mau: Color32) -> egui::Atom<'static> {
    egui::Atom::paint(Vec2::splat(co), move |ui, a| ve(ui.painter(), a.rect, bt, if ui.is_enabled() { mau } else { mau.gamma_multiply(0.45) }))
}

/// Mục menu chọn 1 (menu lọc / sắp xếp): icon nét mảnh đầu dòng, chữ, ✓ xanh ở cuối dòng khi đang chọn.
pub fn muc_menu_icon(ui: &mut egui::Ui, icon: BieuTuong, chu: impl Into<String>, dang_chon: bool, rong: f32) -> Response {
    let r = ui.add(egui::Button::new(format!("      {}", chu.into())).min_size(egui::vec2(rong, 0.0)));
    ve(ui.painter(), Rect::from_center_size(egui::pos2(r.rect.left() + 14.0, r.rect.center().y), egui::vec2(14.0, 14.0)), icon, Color32::from_gray(0x55));
    if dang_chon {
        ve(ui.painter(), Rect::from_center_size(egui::pos2(r.rect.right() - 12.0, r.rect.center().y), egui::vec2(12.0, 12.0)), BieuTuong::Dau, crate::gui::VIEN_TAB_CHON);
    }
    r
}

/// Nút phẳng thanh công cụ (`.nutma` của prototype): chỉ có nền khi rê chuột; `sang` = đang chọn (nền + viền xanh).
pub fn nut_phang<'a>(atoms: impl egui::IntoAtoms<'a>, sang: bool) -> egui::Button<'a> {
    let nut = egui::Button::new(atoms).stroke(Stroke::NONE).min_size(egui::vec2(24.0, 24.0));
    if sang {
        nut.fill(crate::gui::NEN_TAB_CHON).stroke(Stroke::new(1.0, crate::gui::VIEN_TAB_CHON))
    } else {
        nut.frame_when_inactive(false)
    }
}

/// Mục menu: icon sát lề trái, chữ, cuối dòng là ✓ (đang dùng) hoặc phím tắt.
pub fn muc_menu_day(ui: &mut egui::Ui, icon: BieuTuong, chu: &str, dang_chon: bool, phim: Option<&str>, rong: f32) -> Response {
    let r = ui.add(egui::Button::new(format!("      {chu}")).min_size(egui::vec2(rong, 0.0)));
    let mau = ui.visuals().text_color();
    ve(ui.painter(), Rect::from_center_size(egui::pos2(r.rect.left() + 13.0, r.rect.center().y), egui::vec2(14.0, 14.0)), icon, mau);
    if dang_chon {
        ve(ui.painter(), Rect::from_center_size(egui::pos2(r.rect.right() - 12.0, r.rect.center().y), egui::vec2(12.0, 12.0)), BieuTuong::Dau, crate::gui::VIEN_TAB_CHON);
    } else if let Some(p) = phim {
        ui.painter().text(egui::pos2(r.rect.right() - 8.0, r.rect.center().y), egui::Align2::RIGHT_CENTER, p, egui::TextStyle::Small.resolve(ui.style()), ui.visuals().weak_text_color());
    }
    r
}

/// Nút trong menu có dấu ✓ ở cuối dòng khi `dang_chon`.
pub fn muc_menu(ui: &mut egui::Ui, chu: impl Into<egui::WidgetText>, dang_chon: bool, rong: f32) -> Response {
    let r = ui.add(egui::Button::new(chu).min_size(egui::vec2(rong, 0.0)));
    if dang_chon {
        let o = Rect::from_center_size(egui::pos2(r.rect.right() - 12.0, r.rect.center().y), egui::vec2(12.0, 12.0));
        ve(ui.painter(), o, BieuTuong::Dau, crate::gui::VIEN_TAB_CHON);
    }
    r
}

/// ⓘ 14 px, rê chuột hiện giải thích.
pub fn thong_tin(ui: &mut egui::Ui, giai_thich: &str) -> Response {
    // Vòng tròn 14 px viền xám, chữ "i" đậm (như ⓘ của prototype)
    let (r, phan_hoi) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
    let (vien, chu) = if phan_hoi.hovered() { (crate::gui::VIEN_TAB_CHON, crate::gui::VIEN_TAB_CHON) } else { (egui::Color32::from_gray(0x9A), egui::Color32::from_gray(0x77)) };
    let p = ui.painter();
    p.circle_stroke(r.center(), 6.5, Stroke::new(1.0, vien));
    p.text(r.center() + Vec2::new(0.0, 0.5), egui::Align2::CENTER_CENTER, "i", egui::FontId::new(9.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), chu);
    phan_hoi.on_hover_text(giai_thich)
}
