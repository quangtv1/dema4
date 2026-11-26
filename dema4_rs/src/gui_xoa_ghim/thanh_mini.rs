//! Thanh công cụ **mini** (như XoaGhim khi bấm ^ thu gọn): 1 hàng biểu tượng — 4 góc · 4 cạnh · − / + vẽ vùng ·
//! phạm vi trang (tất cả / lẻ / chẵn / từng trang, + vô đối khi vẽ / chọn vùng xoá) · Xóa vùng vẽ · AI (nhận diện
//! layout). Cùng hành vi với thanh đầy đủ; nút đang bật nền xanh nhạt.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind};

use super::thiet_ke as tk;
use super::widget::{self, font, font_dam};
use super::{LocTrang, ManXoaGhim};
use crate::xoa_ghim::vung::{LoaiVung, ViTri};

/// Cao hàng mini.
pub(super) const CAO_MINI: f32 = 44.0;
const NUT: egui::Vec2 = egui::vec2(32.0, 30.0);
const NET: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);

/// Hình vẽ trong 1 nút.
#[derive(Clone, Copy)]
enum Hinh {
    Goc(ViTri),
    Canh(ViTri),
    Tru,
    Cong,
    NhieuTrang,
    Chu(&'static str),
    Choi,
    Ai,
}

/// Trang giấy 11 × 14 giữa nút.
fn trang(c: Pos2) -> Rect {
    Rect::from_center_size(c, egui::vec2(11.0, 14.0))
}

fn ve_hinh(p: &egui::Painter, o: Rect, h: Hinh, mau: Color32) {
    let c = o.center();
    let net = Stroke::new(1.2, mau);
    match h {
        Hinh::Goc(v) | Hinh::Canh(v) => {
            let t = trang(c);
            p.rect_stroke(t, 1.0, net, StrokeKind::Inside);
            let dac = match (h, v) {
                (Hinh::Goc(_), ViTri::GocTrenTrai) => Rect::from_min_size(t.min, egui::vec2(5.0, 5.0)),
                (Hinh::Goc(_), ViTri::GocTrenPhai) => Rect::from_min_size(egui::pos2(t.right() - 5.0, t.top()), egui::vec2(5.0, 5.0)),
                (Hinh::Goc(_), ViTri::GocDuoiTrai) => Rect::from_min_size(egui::pos2(t.left(), t.bottom() - 5.0), egui::vec2(5.0, 5.0)),
                (Hinh::Goc(_), _) => Rect::from_min_size(t.max - egui::vec2(5.0, 5.0), egui::vec2(5.0, 5.0)),
                (_, ViTri::CanhTren) => Rect::from_min_size(t.min, egui::vec2(t.width(), 4.0)),
                (_, ViTri::CanhDuoi) => Rect::from_min_size(egui::pos2(t.left(), t.bottom() - 4.0), egui::vec2(t.width(), 4.0)),
                (_, ViTri::CanhTrai) => Rect::from_min_size(t.min, egui::vec2(4.0, t.height())),
                _ => Rect::from_min_size(egui::pos2(t.right() - 4.0, t.top()), egui::vec2(4.0, t.height())),
            };
            p.rect_filled(dac, 0.5, mau);
        }
        Hinh::Tru => {
            p.line_segment([c - egui::vec2(7.0, 0.0), c + egui::vec2(7.0, 0.0)], Stroke::new(1.6, mau));
        }
        Hinh::Cong => {
            p.line_segment([c - egui::vec2(7.0, 0.0), c + egui::vec2(7.0, 0.0)], Stroke::new(1.6, mau));
            p.line_segment([c - egui::vec2(0.0, 7.0), c + egui::vec2(0.0, 7.0)], Stroke::new(1.6, mau));
        }
        Hinh::NhieuTrang => {
            p.rect_stroke(trang(c + egui::vec2(2.0, -2.0)), 1.0, net, StrokeKind::Inside);
            let t = trang(c + egui::vec2(-2.0, 2.0));
            p.rect(t, 1.0, Color32::WHITE, net, StrokeKind::Inside);
        }
        Hinh::Chu(k) => {
            let t = trang(c);
            p.rect_stroke(t, 1.0, net, StrokeKind::Inside);
            p.text(t.center() + egui::vec2(0.0, 0.5), egui::Align2::CENTER_CENTER, k, font(9.5), mau);
        }
        Hinh::Choi => {
            // Cán chéo + bó lông chổi
            p.line_segment([c + egui::vec2(4.0, -8.0), c + egui::vec2(-1.0, 2.0)], Stroke::new(1.4, mau));
            let lo = vec![c + egui::vec2(-4.0, 1.0), c + egui::vec2(2.0, 3.0), c + egui::vec2(1.0, 8.0), c + egui::vec2(-7.0, 8.0)];
            p.add(egui::Shape::convex_polygon(lo, Color32::TRANSPARENT, Stroke::new(1.2, mau)));
            p.line_segment([c + egui::vec2(-4.0, 8.0), c + egui::vec2(-2.0, 3.0)], Stroke::new(1.0, mau));
        }
        Hinh::Ai => {
            p.text(c, egui::Align2::CENTER_CENTER, "AI", font_dam(12.0), mau);
        }
    }
}

/// 1 nút mini: nền xanh nhạt khi `dang`, xám khi rê chuột; mờ khi `!duoc`. Tên trợ năng = `ten`.
fn nut(ui: &mut egui::Ui, h: Hinh, dang: bool, duoc: bool, ten: &str, tip: &str) -> egui::Response {
    let (o, ph) = ui.allocate_exact_size(NUT, if duoc { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if dang {
        p.rect(o, 4.0, tk::NEN_NUT_VUA_DANG, Stroke::new(1.0, tk::VIEN_NUT_VUA_DANG), StrokeKind::Inside);
    } else if duoc && ph.hovered() {
        p.rect_filled(o, 4.0, tk::NEN_NUT_HOVER);
    }
    let mau = if !duoc { tk::CHU_MO } else if dang { tk::XANH_DAM_CHU } else { NET };
    ve_hinh(p, o, h, mau);
    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, duoc, dang, ten));
    // Mô tả hiện ngay khi rê chuột (kể cả nút mờ) — nút chỉ có hình nên cần biết chức năng
    widget::tooltip_ngay(ui, &ph, tip);
    ph
}

fn vach(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    widget::vach_doc(ui, 22.0);
    ui.add_space(4.0);
}

impl ManXoaGhim {
    pub(super) fn thanh_mini(&mut self, ui: &mut egui::Ui) {
        let (o, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_MINI), Sense::hover());
        ui.painter().hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, tk::VIEN_KHUNG));
        let mut h = ui.new_child(egui::UiBuilder::new().max_rect(o.shrink2(egui::vec2(10.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        h.spacing_mut().item_spacing.x = 4.0;
        let ui = &mut h;
        // 4 góc · 4 cạnh
        for (v, ten) in [(ViTri::GocTrenTrai, "Góc trên trái"), (ViTri::GocTrenPhai, "Góc trên phải"), (ViTri::GocDuoiTrai, "Góc dưới trái"), (ViTri::GocDuoiPhai, "Góc dưới phải")] {
            let bat = self.bo.tm.vung.iter().any(|z| z.vi_tri == v);
            if nut(ui, Hinh::Goc(v), bat, true, ten, &format!("{ten} — bấm để {}", if bat { "bỏ" } else { "thêm" })).clicked() {
                self.bat_tat_co_san(v);
            }
        }
        vach(ui);
        for (v, ten) in [(ViTri::CanhTren, "Cạnh trên"), (ViTri::CanhDuoi, "Cạnh dưới"), (ViTri::CanhTrai, "Cạnh trái"), (ViTri::CanhPhai, "Cạnh phải")] {
            let bat = self.bo.tm.vung.iter().any(|z| z.vi_tri == v);
            if nut(ui, Hinh::Canh(v), bat, true, ten, &format!("{ten} — bấm để {}", if bat { "bỏ" } else { "thêm" })).clicked() {
                self.bat_tat_co_san(v);
            }
        }
        vach(ui);
        // Vẽ vùng
        if nut(ui, Hinh::Tru, self.ve == Some(LoaiVung::Xoa), true, "Vẽ vùng xoá", "Vẽ vùng xoá — kéo trên trang Gốc · Esc để thôi").clicked() {
            self.bat_ve(LoaiVung::Xoa);
        }
        if nut(ui, Hinh::Cong, self.ve == Some(LoaiVung::BaoVe), true, "Vẽ vùng bảo vệ", "Vẽ vùng bảo vệ — kéo trên trang Gốc · Esc để thôi").clicked() {
            self.bat_ve(LoaiVung::BaoVe);
        }
        vach(ui);
        // Phạm vi trang
        let hien = self.loc_hien();
        let mut ds = vec![(LocTrang::TatCa, Hinh::NhieuTrang, "Tất cả"), (LocTrang::Le, Hinh::Chu("1"), "Trang lẻ"), (LocTrang::Chan, Hinh::Chu("2"), "Trang chẵn"), (LocTrang::TungTrang, Hinh::Chu("*"), "Từng trang")];
        if self.co_vo_doi() {
            ds.push((LocTrang::VoDoi, Hinh::Chu("!"), "Vô đối"));
        }
        for (l, hinh, ten) in ds {
            if nut(ui, hinh, hien == l, true, ten, &format!("Phạm vi: {ten}")).clicked() {
                self.chon_loc_trang(l);
            }
        }
        vach(ui);
        let co_ve = !self.bo.moi_vung_ve().is_empty();
        if nut(ui, Hinh::Choi, false, co_ve, "Xóa vùng vẽ", "Xoá hết các vùng vẽ Tùy biến (có hỏi lại). Cạnh, góc giữ nguyên.").clicked() {
            self.xac_nhan_xoa = Some(self.bo.moi_vung_ve());
        }
        vach(ui);
        let co_mo_hinh = crate::xoa_ghim::mo_hinh::co_san();
        let nd = self.bo.tm.tuy_chon.nhan_dien && co_mo_hinh;
        let tip = if co_mo_hinh { "Nhận diện layout — bảo vệ khối chữ, bảng" } else { "Nhận diện layout — cần tải mô hình AI trong Cấu hình" };
        if nut(ui, Hinh::Ai, nd, co_mo_hinh, "Nhận diện layout", tip).clicked() {
            self.bo.tm.tuy_chon.nhan_dien = !nd;
            self.bo.doi_chung();
        }
    }
}
