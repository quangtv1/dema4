//! Icon nét 1 màu: dùng đúng SVG của prototype (`plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`,
//! cùng toạ độ path và độ dày nét), vẽ bằng bộ đọc SVG của egui_extras; nét trắng rồi tô màu bằng `tint`.

use eframe::egui::{self, Color32, Rect, Response, Sense, Shape, Stroke, Vec2};

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
    /// Menu Trợ giúp: Thông tin (info), Kiểm tra cập nhật (refresh), Hướng dẫn (circle-help)
    ThongTin,
    CapNhat,
    HoiDap,
    /// Menu Chức năng: Đếm (file-text), Tách (scissors), Ký (pen)
    TaiLieu,
    Keo,
    But,
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
    /// Thanh menu: lưới 4 ô (Chức năng), bánh răng (Cấu hình)
    MenuLuoi,
    BanhRang,
    /// Menu lọc / sắp xếp
    ChongTrang,
    MaVach,
    TrangTrong,
    TrangHoi,
    XoayMenu,
    DanhSachSo,
    TangDan,
    GiamDan,
    /// Nhóm sửa / tab Văn bản
    ThungRac,
    TrangGach,
    /// Cột trái: thu gọn / mở rộng, kính lúp ô tìm; hàng tab: ẩn / hiện dòng thư mục
    ThuGon,
    MoRong,
    KinhLup,
    AnDong,
    HienDong,
}

/// (path SVG, cạnh viewBox, độ dày nét) — chép nguyên từ prototype.
fn svg_cua(bt: BieuTuong) -> Option<(&'static str, u8, f32)> {
    use BieuTuong::*;
    Some(match bt {
        Dau => (r#"<path d="M3.5 8.5 6.5 11.5 12.5 4.5"/>"#, 16, 1.8),
        MuiTenXuong | BoKhoi => return None,
        HoanTac => (r#"<path d="M6 9.5 2.5 6 6 2.5"/><path d="M2.5 6H10a3.5 3.5 0 0 1 0 7H8"/>"#, 16, 1.15),
        LamLai => (r#"<path d="M10 9.5 13.5 6 10 2.5"/><path d="M13.5 6H6a3.5 3.5 0 0 0 0 7h2"/>"#, 16, 1.15),
        XoayPhai => (r#"<path d="M12.5 7.5A4.5 4.5 0 1 1 8 3h3.5"/><path d="M9.5 1 11.5 3 9.5 5"/>"#, 16, 1.3),
        Xoay180 => (r#"<path d="M3.5 8a4.5 4.5 0 0 1 9 0v3"/><path d="M10.5 9 12.5 11 14.5 9"/>"#, 16, 1.3),
        ThongTin => (r#"<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>"#, 24, 1.75),
        CapNhat => (r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#, 24, 1.75),
        HoiDap => (r#"<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>"#, 24, 1.8),
        TaiLieu => (r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M16 13H8"/><path d="M16 17H8"/><path d="M10 9H8"/>"#, 24, 1.75),
        Keo => (r#"<circle cx="6" cy="6" r="3"/><path d="M8.12 8.12 12 12"/><path d="M20 4 8.12 15.88"/><circle cx="6" cy="18" r="3"/><path d="M14.8 14.8 20 20"/>"#, 24, 1.75),
        But => (r#"<path d="M12 20h9"/><path d="M16.4 3.6a2.1 2.1 0 0 1 3 3L7.4 18.6a2 2 0 0 1-.9.5l-2.9.9.9-2.9a2 2 0 0 1 .5-.9z"/>"#, 24, 1.75),
        TraiChevron => (r#"<path d="M10 3.5 5.5 8l4.5 4.5"/>"#, 16, 1.3),
        PhaiChevron => (r#"<path d="M6 3.5 10.5 8 6 12.5"/>"#, 16, 1.3),
        Luoi => (r#"<rect x="2" y="2" width="5" height="5"/><rect x="9" y="2" width="5" height="5"/><rect x="2" y="9" width="5" height="5"/><rect x="9" y="9" width="5" height="5"/>"#, 16, 1.3),
        MotTrang => (r#"<rect x="4" y="1.5" width="8" height="13"/><path d="M1.5 8h1.5M13 8h1.5"/>"#, 16, 1.3),
        HaiTrang => (r#"<rect x="1.5" y="2.5" width="6" height="11"/><rect x="8.5" y="2.5" width="6" height="11"/>"#, 16, 1.3),
        Dong => (r#"<path d="M4 4l8 8M12 4l-8 8"/>"#, 16, 1.3),
        ChevronXuong => (r#"<path d="M4.5 6.5 8 10l3.5-3.5"/>"#, 16, 1.3),
        KinhTru => (r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14M5 7h4"/>"#, 16, 1.3),
        KinhCong => (r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14M5 7h4M7 5v4"/>"#, 16, 1.3),
        SapXep => (r#"<path d="M5 3v10M2.5 10.5 5 13l2.5-2.5M11 13V3M8.5 5.5 11 3l2.5 2.5"/>"#, 16, 1.3),
        MenuLuoi => (r#"<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>"#, 24, 1.8),
        BanhRang => (r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>"#, 24, 1.8),
        ChongTrang => (r#"<path d="M15 2H9a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2V7z"/><path d="M4 7v13a2 2 0 0 0 2 2h9"/>"#, 24, 1.8),
        MaVach => (r#"<path d="M3 5v14M6.5 5v14M9 5v14M13 5v14M15.5 5v14M18 5v14M21 5v14"/>"#, 24, 1.8),
        TrangTrong => (r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>"#, 24, 1.8),
        TrangHoi => (r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M10 13a2 2 0 1 1 3 1.7c-.6.3-1 .8-1 1.3"/><path d="M12 19h.01"/>"#, 24, 1.8),
        XoayMenu => (r#"<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>"#, 24, 1.8),
        DanhSachSo => (r#"<path d="M10 6h11M10 12h11M10 18h11"/><path d="M4 6h1v4"/><path d="M4 10h2"/><path d="M6 18H4c0-1 2-2 2-3s-1-1.5-2-1"/>"#, 24, 1.8),
        TangDan => (r#"<path d="m3 8 4-4 4 4"/><path d="M7 4v16"/><path d="M11 12h4"/><path d="M11 16h7"/><path d="M11 20h10"/>"#, 24, 1.8),
        GiamDan => (r#"<path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="M11 4h10"/><path d="M11 8h7"/><path d="M11 12h4"/>"#, 24, 1.8),
        ThungRac => (r#"<path d="M2.5 4h11"/><path d="M6 4V2.5h4V4"/><path d="M3.8 4l.7 9.5h7l.7-9.5"/><path d="M6.5 6.5v4.5M9.5 6.5v4.5"/>"#, 16, 1.15),
        TrangGach => (r#"<path d="M9.5 1.5H4.5a1 1 0 0 0-1 1v11a1 1 0 0 0 1 1h7a1 1 0 0 0 1-1v-8.5z"/><path d="M9.5 1.5v3h3"/><path d="M2 14 14 2"/>"#, 16, 1.3),
        ThuGon => (r#"<path d="M3 2.5v11"/><path d="M11 5 8 8l3 3"/>"#, 16, 1.4),
        MoRong => (r#"<path d="M3 2.5v11"/><path d="M5 5l3 3-3 3"/>"#, 16, 1.4),
        KinhLup => (r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14"/>"#, 16, 1.4),
        AnDong => (r#"<path d="M3 3.5h10"/><path d="M5 10l3-3 3 3"/>"#, 16, 1.4),
        HienDong => (r#"<path d="M3 3.5h10"/><path d="M5 7l3 3 3-3"/>"#, 16, 1.4),
    })
}

const TAT_CA: [BieuTuong; 40] = {
    use BieuTuong::*;
    [
        Dau, MuiTenXuong, HoanTac, LamLai, XoayPhai, Xoay180, ThongTin, CapNhat, HoiDap, TaiLieu, Keo, But, TraiChevron, PhaiChevron, Luoi, MotTrang, HaiTrang, BoKhoi, Dong, ChevronXuong, KinhTru, KinhCong, SapXep, MenuLuoi,
        BanhRang, ChongTrang, MaVach, TrangTrong, TrangHoi, XoayMenu, DanhSachSo, TangDan, GiamDan, ThungRac, TrangGach, ThuGon, MoRong, KinhLup, AnDong, HienDong,
    ]
};

fn uri(bt: BieuTuong) -> String {
    format!("bytes://bieu_tuong/{bt:?}.svg")
}

/// Nạp bộ đọc ảnh (SVG) và đăng ký SVG của mọi icon — gọi 1 lần khi dựng giao diện.
pub fn dang_ky_svg(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    for bt in TAT_CA {
        if let Some((than, canh, net)) = svg_cua(bt) {
            let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canh} {canh}" fill="none" stroke="#fff" stroke-width="{net}" stroke-linecap="round" stroke-linejoin="round">{than}</svg>"##);
            ctx.include_bytes(uri(bt), svg.into_bytes());
        }
    }
}

/// Vẽ icon vừa khít `r` (vùng vuông) màu `mau`.
pub fn ve(painter: &egui::Painter, r: Rect, bt: BieuTuong, mau: Color32) {
    let c = r.center();
    let k = r.width().min(r.height()) / 2.0;
    match bt {
        BieuTuong::MuiTenXuong => {
            painter.add(Shape::convex_polygon(vec![c + Vec2::new(-0.45, -0.2) * k, c + Vec2::new(0.45, -0.2) * k, c + Vec2::new(0.0, 0.3) * k], mau, Stroke::NONE));
        }
        BieuTuong::BoKhoi => {}
        _ => {
            let ctx = painter.ctx();
            let px = (r.width().max(r.height()) * ctx.pixels_per_point()).round().max(1.0) as u32;
            let goi_y = egui::SizeHint::Size { width: px, height: px, maintain_aspect_ratio: true };
            match ctx.try_load_texture(&uri(bt), egui::TextureOptions::LINEAR, goi_y) {
                Ok(egui::load::TexturePoll::Ready { texture }) => {
                    painter.image(texture.id, r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), mau);
                }
                Ok(egui::load::TexturePoll::Pending { .. }) => ctx.request_repaint(),
                Err(_) => {}
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
    atom_mau(bt, co, Color32::from_gray(0x55))
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
