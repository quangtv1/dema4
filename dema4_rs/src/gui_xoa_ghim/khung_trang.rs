//! Khung giữa màn Xóa ghim (như XoaGhim): cột **Trang thu nhỏ**, 2 khung **Gốc | Đích** có thanh tiêu đề, các trang
//! xếp dọc cuộn liên tục (hoặc từng trang) trong cùng 1 vùng cuộn nên cuộn / phóng luôn đồng bộ; khung vùng vẽ trên
//! ảnh Gốc, kéo chuột để vẽ / chọn / di chuyển / đổi cỡ vùng. Thanh đáy: đếm vùng, chuyển trang, kiểu xem, phóng.
//!
//! Chỉ trang đang thấy (± 1) mới xin ảnh; ảnh Đích làm lại khi vùng / tuỳ chọn đổi (mã cấu hình khác), trong lúc chờ
//! vẫn hiện ảnh cũ. Giữ texture ~ 8 trang quanh trang đang xem.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};

use super::sua_vung::{canh_keo_duoc, keo_vung, Canh};
use super::xem_truoc::{YeuCau, RONG_ANH_NHO};
use super::{mau_loai, ManXoaGhim};
use crate::gui::{CHU, CHU_PHU, NEN_TIEU_DE, VIEN, VIEN_NHAT};
use crate::gui_tach::bieu_tuong::{atom_mau, BieuTuong};
use crate::xoa_ghim::vung::{LoaiVung, Vung};

const LE: f32 = 12.0;
const KHE: f32 = 12.0;
const KHE_TRANG: f32 = 14.0;
/// Khoảng bắt cạnh vùng khi kéo đổi cỡ (điểm màn hình).
const BAT_CANH: f32 = 6.0;
const GIU_QUANH: u32 = 4;
const CAC_PHONG: [u16; 9] = [25, 50, 75, 100, 125, 150, 200, 300, 400];
const NEN_KHUNG: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);

impl ManXoaGhim {
    pub(super) fn thanh_duoi(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("xg_duoi").frame(egui::Frame::new().fill(NEN_TIEU_DE).inner_margin(egui::Margin::symmetric(10, 4)).stroke(Stroke::new(1.0, VIEN_NHAT))).show(ui, |ui| {
            ui.horizontal(|ui| {
                let (chung, rieng) = self.bo.dem();
                ui.label(RichText::new(format!("Zone chung: {chung}; Zone riêng: {rieng}")).color(CHU_PHU));
                let n = self.kho_trang.len() as u32;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let r = ui.add(egui::Button::new(atom_mau(BieuTuong::KinhCong, 14.0, CHU)));
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Phóng to"));
                    if r.on_hover_text("Phóng to (+) · Ctrl + cuộn chuột").clicked() {
                        self.phong(1.25);
                    }
                    let r = ui.add(egui::Button::new(atom_mau(BieuTuong::KinhTru, 14.0, CHU)));
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Thu nhỏ"));
                    if r.on_hover_text("Thu nhỏ (−)").clicked() {
                        self.phong(0.8);
                    }
                    let hien = (self.ty_le_cuoi * 100.0).round() as u16;
                    egui::ComboBox::from_id_salt("xg_phong").width(70.0).selected_text(format!("{hien}%")).show_ui(ui, |ui| {
                        for p in CAC_PHONG {
                            if ui.selectable_label(p == hien, format!("{p}%")).clicked() {
                                self.ty_le = Some(p as f32 / 100.0);
                            }
                        }
                    })
                    .response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Phóng"));
                    if ui.selectable_label(self.ty_le.is_none() && self.vua_cao, "Vừa cao").on_hover_text("Cả trang vừa chiều cao khung").clicked() {
                        self.ty_le = None;
                        self.vua_cao = true;
                    }
                    if ui.selectable_label(self.ty_le.is_none() && !self.vua_cao, "Vừa ngang").on_hover_text("Trang vừa bề ngang khung (=)").clicked() {
                        self.ty_le = None;
                        self.vua_cao = false;
                    }
                    ui.separator();
                    egui::ComboBox::from_id_salt("xg_kieu_xem").width(90.0).selected_text(if self.lien_tiep { "Liên tiếp" } else { "Từng trang" }).show_ui(ui, |ui| {
                        if ui.selectable_label(self.lien_tiep, "Liên tiếp").clicked() {
                            self.lien_tiep = true;
                            self.cuon_toi = Some(self.trang);
                        }
                        if ui.selectable_label(!self.lien_tiep, "Từng trang").clicked() {
                            self.lien_tiep = false;
                        }
                    })
                    .response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Kiểu xem"));
                    ui.separator();
                    if ui.add_enabled(self.trang + 1 < n, egui::Button::new(atom_mau(BieuTuong::PhaiChevron, 14.0, CHU))).on_hover_text("Trang sau (PageDown)").clicked() {
                        self.sang_trang(self.trang + 1);
                    }
                    ui.label(format!("/ {n}"));
                    let mut so = self.trang + 1;
                    let r = ui.add(egui::DragValue::new(&mut so).range(1..=n.max(1)));
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, true, "Trang"));
                    if r.changed() {
                        self.sang_trang(so - 1);
                    }
                    if ui.add_enabled(self.trang > 0, egui::Button::new(atom_mau(BieuTuong::TraiChevron, 14.0, CHU))).on_hover_text("Trang trước (PageUp)").clicked() {
                        self.sang_trang(self.trang - 1);
                    }
                });
            });
        });
    }

    fn cot_anh_nho(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("xg_nho").resizable(false).exact_size(RONG_ANH_NHO as f32 + 34.0).frame(egui::Frame::new().fill(Color32::from_gray(0xF7)).stroke(Stroke::new(1.0, VIEN_NHAT))).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Trang thu nhỏ").size(12.0));
            });
            let rong = RONG_ANH_NHO as f32;
            let mut bam = None;
            egui::ScrollArea::vertical().id_salt(("xg_nho_cuon", self.the_he)).auto_shrink([false, false]).show(ui, |ui| {
                for (t, &(w, h)) in self.kho_trang.iter().enumerate() {
                    let t = t as u32;
                    ui.add_space(8.0);
                    let cao = rong * h / w;
                    let (o, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), cao + 18.0), Sense::click());
                    let anh_o = Rect::from_min_size(egui::pos2(o.center().x - rong / 2.0, o.top()), egui::vec2(rong, cao));
                    if ui.is_rect_visible(o) {
                        let p = ui.painter();
                        p.rect_filled(anh_o, 0.0, Color32::WHITE);
                        match self.anh_nho.get(&t) {
                            Some(tex) => {
                                p.image(tex.id(), anh_o, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            None if self.cho_nho.insert(t) => self.xem.gui(YeuCau::AnhNho { the_he: self.the_he, trang: t }),
                            None => {}
                        }
                        let dang = t == self.trang;
                        p.rect_stroke(anh_o, 0.0, Stroke::new(if dang { 2.0 } else { 1.0 }, if dang { crate::gui::VIEN_TAB_CHON } else { VIEN }), StrokeKind::Outside);
                        let so = egui::pos2(o.center().x, anh_o.bottom() + 9.0);
                        if dang {
                            p.rect_filled(Rect::from_center_size(so, egui::vec2(20.0, 14.0)), 2.0, crate::gui::VIEN_TAB_CHON);
                        }
                        p.text(so, egui::Align2::CENTER_CENTER, format!("{}", t + 1), egui::FontId::proportional(10.5), if dang { Color32::WHITE } else { CHU_PHU });
                    }
                    if ph.clicked() {
                        bam = Some(t);
                    }
                }
            });
            if let Some(t) = bam {
                self.sang_trang(t);
            }
        });
    }

    pub(super) fn khung_trang(&mut self, ui: &mut egui::Ui) {
        if self.ds.is_empty() {
            ui.centered_and_justified(|ui| ui.label(RichText::new("Chọn hoặc kéo thả thư mục chứa PDF scan để xoá ghim.").size(14.0).color(CHU_PHU)));
            return;
        }
        self.cot_anh_nho(ui);
        // Thanh tiêu đề 2 khung
        let khung = ui.available_rect_before_wrap();
        let nua = ((khung.width() - KHE) / 2.0).max(50.0);
        let (pdf, ra) = self.ds.get(self.chon).cloned().unwrap_or_default();
        let (dong, _) = ui.allocate_exact_size(egui::vec2(khung.width(), 28.0), Sense::hover());
        ui.painter().rect_filled(dong, 0.0, NEN_TIEU_DE);
        let o_trai = Rect::from_min_size(dong.min, egui::vec2(nua, dong.height()));
        let o_phai = Rect::from_min_size(dong.min + egui::vec2(nua + KHE, 0.0), egui::vec2(nua, dong.height()));
        {
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(o_trai.shrink2(egui::vec2(4.0, 1.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let ui = &mut ui;
            ui.spacing_mut().item_spacing.x = 4.0;
            {
                if ui.add_enabled(self.chon > 0, egui::Button::new(atom_mau(BieuTuong::TraiChevron, 13.0, CHU))).on_hover_text("File trước ([)").clicked() {
                    self.chon_file(self.chon - 1);
                }
                if ui.add_enabled(self.chon + 1 < self.ds.len(), egui::Button::new(atom_mau(BieuTuong::PhaiChevron, 13.0, CHU))).on_hover_text("File sau (])").clicked() {
                    self.chon_file(self.chon + 1);
                }
                ui.label(RichText::new(format!("({}/{})", self.chon + 1, self.ds.len())).color(CHU_PHU));
                ui.add(egui::Label::new(format!("Gốc: {}", pdf.display())).truncate()).on_hover_text(pdf.display().to_string());
            }
        }
        {
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(o_phai.shrink2(egui::vec2(4.0, 1.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            ui.add(egui::Label::new(format!("Đích: {}", ra.display())).truncate()).on_hover_text(ra.display().to_string());
        }
        if self.kho_trang.is_empty() {
            ui.centered_and_justified(|ui| {
                if let Some(e) = &self.loi {
                    ui.label(RichText::new(e).color(crate::gui::DO));
                } else {
                    ui.spinner();
                }
            });
            return;
        }
        let khung = ui.available_rect_before_wrap();
        ui.painter().rect_filled(khung, 0.0, NEN_KHUNG);
        // Ctrl + cuộn chuột trên khung: phóng
        if ui.rect_contains_pointer(khung) {
            let z = ui.ctx().input(|i| i.zoom_delta());
            if z != 1.0 {
                self.ty_le = Some((self.ty_le_cuoi * z).clamp(0.1, 8.0));
            }
        }
        let rong_max = self.kho_trang.iter().map(|k| k.0).fold(1.0, f32::max);
        let cao_max = self.kho_trang.iter().map(|k| k.1).fold(1.0, f32::max);
        let vua = if self.vua_cao { (khung.height() - 2.0 * LE) / cao_max } else { (nua - 2.0 * LE) / rong_max };
        let ty = self.ty_le.unwrap_or(vua).max(0.05);
        self.ty_le_cuoi = ty;
        let cac: Vec<u32> = if self.lien_tiep { (0..self.kho_trang.len() as u32).collect() } else { vec![self.trang.min(self.kho_trang.len() as u32 - 1)] };
        // Vị trí từng trang (tương đối nội dung cuộn): đỉnh + cỡ
        let mut dinh = Vec::with_capacity(cac.len());
        let mut y = LE;
        for &t in &cac {
            let (w, h) = self.kho_trang[t as usize];
            dinh.push((t, y, egui::vec2(w * ty, h * ty)));
            y += h * ty + KHE_TRANG;
        }
        let tong = egui::vec2((2.0 * rong_max * ty + KHE + 2.0 * LE).max(khung.width()), y + LE);
        let (ch, ma) = self.cau_hinh_xem();
        let mut cuon = egui::ScrollArea::both().id_salt(("xg_cuon", self.the_he, self.lien_tiep)).auto_shrink([false, false]);
        if let Some(t) = self.cuon_toi.take() {
            if let Some((_, y0, _)) = dinh.iter().find(|(x, _, _)| *x == t) {
                cuon = cuon.vertical_scroll_offset((y0 - LE).max(0.0));
            }
        }
        let ctx = ui.ctx().clone();
        let mut trang_tren = None;
        let ra_cuon = cuon.show(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(tong, Sense::hover());
            let cot = r.left() + ((r.width() - 2.0 * rong_max * ty - KHE) / 2.0).max(LE);
            let nhin = ui.clip_rect();
            for &(t, y0, co) in &dinh {
                let o_goc = Rect::from_min_size(egui::pos2(cot + (rong_max * ty - co.x) / 2.0, r.top() + y0), co);
                let o_dich = o_goc.translate(egui::vec2(rong_max * ty + KHE, 0.0));
                if trang_tren.is_none() && o_goc.bottom() > nhin.top() + nhin.height() * 0.3 {
                    trang_tren = Some(t);
                }
                if !nhin.expand(co.y).intersects(o_goc) {
                    continue;
                }
                // Xin ảnh nếu chưa có / đã cũ
                let can = self.anh.get(&t).is_none_or(|a| a.ma != ma);
                if can && !self.cho_anh.contains(&(t, ma)) {
                    self.cho_anh.retain(|(x, _)| *x != t);
                    self.cho_anh.insert((t, ma));
                    self.xem.gui(YeuCau::Trang { the_he: self.the_he, trang: t, ma, ch: ch.clone() });
                }
                let p = ui.painter();
                for o in [o_goc, o_dich] {
                    p.rect_filled(o.translate(egui::vec2(0.0, 2.0)), 0.0, Color32::from_black_alpha(25));
                    p.rect_filled(o, 0.0, Color32::WHITE);
                }
                let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                match self.anh.get(&t) {
                    Some(a) => {
                        p.image(a.goc.id(), o_goc, uv, Color32::WHITE);
                        p.image(a.dich.id(), o_dich, uv, Color32::WHITE);
                        if a.ma != ma {
                            p.text(o_dich.right_top() + egui::vec2(-8.0, 8.0), egui::Align2::RIGHT_TOP, "đang cập nhật…", egui::FontId::proportional(11.0), CHU_PHU);
                        }
                    }
                    None => {
                        p.text(o_goc.center(), egui::Align2::CENTER_CENTER, "Đang vẽ trang…", egui::FontId::proportional(12.0), CHU_PHU);
                    }
                }
                for o in [o_goc, o_dich] {
                    p.rect_stroke(o, 0.0, Stroke::new(1.0, VIEN), StrokeKind::Outside);
                }
                self.vung_tren_trang(ui, t, o_goc, &ctx);
            }
        });
        if self.lien_tiep {
            if let Some(t) = trang_tren {
                self.trang = t;
            }
        }
        let _ = ra_cuon;
        // Bỏ texture trang ở xa trang đang xem (giới hạn bộ nhớ đồ hoạ)
        let giu = self.trang.saturating_sub(GIU_QUANH)..=self.trang + GIU_QUANH;
        self.anh.retain(|t, _| giu.contains(t));
    }

    /// Khung các vùng trên ảnh Gốc của trang `t` + thao tác chuột (vẽ / chọn / kéo).
    fn vung_tren_trang(&mut self, ui: &mut egui::Ui, t: u32, o_goc: Rect, ctx: &egui::Context) {
        let Some((co_anh, dpi)) = self.anh.get(&t).map(|a| (a.co_anh, a.dpi)) else { return };
        let (w, h) = (co_anh.0 as f32, co_anh.1 as f32);
        let k = o_goc.width() / w;
        let khung_man = |z: &Vung| -> Option<Rect> {
            let kk = z.khung_xoa(co_anh.0 as i64, co_anh.1 as i64, dpi as f64)?;
            Some(Rect::from_min_max(o_goc.min + egui::vec2(kk.x1 as f32 * k, kk.y1 as f32 * k), o_goc.min + egui::vec2(kk.x2 as f32 * k, kk.y2 as f32 * k)))
        };
        let chung = self.bo.tm.vung.len();
        let p = ui.painter().clone();
        for (i, z) in self.bo.vung().iter().enumerate() {
            if !z.ap_trang.ap_dung(t) {
                continue;
            }
            let Some(r) = khung_man(z) else { continue };
            let dang = self.bo.chon == Some((i >= chung, if i >= chung { i - chung } else { i }));
            let mau = mau_loai(z.loai);
            p.rect_filled(r, 0.0, mau.gamma_multiply(if dang { 0.25 } else { 0.13 }));
            p.rect_stroke(r, 0.0, Stroke::new(if dang { 2.5 } else { 1.2 }, mau), StrokeKind::Inside);
        }
        let ph = ui.interact(o_goc, ui.scope_id().with(("xg_trang", t)), Sense::click_and_drag());
        let anh = |q: Pos2| (((q.x - o_goc.left()) / k) as f64, ((q.y - o_goc.top()) / k) as f64);
        if let Some(loai) = self.ve {
            // Vẽ vùng tay: kéo trên ảnh Gốc của trang này
            if ph.drag_started() {
                self.keo = ph.interact_pointer_pos().map(|q| (t, q));
            }
            if let (Some((tk, a0)), Some(b0)) = (self.keo, ctx.pointer_latest_pos()) {
                if tk == t {
                    let r = Rect::from_two_pos(a0, b0.clamp(o_goc.min, o_goc.max));
                    let mau = mau_loai(if loai == LoaiVung::Xoa && self.loc_trang == super::LocTrang::VoDoi { LoaiVung::VoDoi } else { loai });
                    p.rect_filled(r, 0.0, mau.gamma_multiply(0.15));
                    p.rect_stroke(r, 0.0, Stroke::new(1.5, mau), StrokeKind::Inside);
                    if ph.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
                        self.keo = None;
                        let ((x1, y1), (x2, y2)) = (anh(r.min), anh(r.max));
                        self.them_ve((x1 / w as f64, y1 / h as f64, (x2 - x1) / w as f64, (y2 - y1) / h as f64), t);
                    }
                }
            }
            ph.on_hover_cursor(egui::CursorIcon::Crosshair);
            return;
        }
        // Chọn: bấm chọn vùng; kéo thân vùng vẽ để di chuyển, kéo cạnh để đổi cỡ (có sẵn: chỉ cạnh phía trong)
        let canh_tai = |z: &Vung, q: Pos2| -> Option<Canh> {
            let r = khung_man(z)?;
            if !r.expand(BAT_CANH).contains(q) {
                return None;
            }
            let duoc = canh_keo_duoc(z.vi_tri);
            let c = Canh {
                trai: duoc.trai && (q.x - r.left()).abs() <= BAT_CANH,
                phai: duoc.phai && (q.x - r.right()).abs() <= BAT_CANH,
                tren: duoc.tren && (q.y - r.top()).abs() <= BAT_CANH,
                duoi: duoc.duoi && (q.y - r.bottom()).abs() <= BAT_CANH,
            };
            (!c.than() || (z.vi_tri == crate::xoa_ghim::vung::ViTri::Tay && r.contains(q))).then_some(c)
        };
        if let Some(q) = ph.hover_pos() {
            if let Some(c) = self.bo.vung_chon().filter(|z| z.ap_trang.ap_dung(t)).and_then(|z| canh_tai(z, q)) {
                ctx.set_cursor_icon(match (c.trai || c.phai, c.tren || c.duoi) {
                    (true, true) if c.trai == c.tren => egui::CursorIcon::ResizeNwSe,
                    (true, true) => egui::CursorIcon::ResizeNeSw,
                    (true, false) => egui::CursorIcon::ResizeHorizontal,
                    (false, true) => egui::CursorIcon::ResizeVertical,
                    _ => egui::CursorIcon::Move,
                });
            }
        }
        let pos_anh = |q: Pos2| {
            let (x, y) = anh(q);
            (x as i64, y as i64)
        };
        if ph.drag_started() {
            if let Some(q) = ph.interact_pointer_pos() {
                if self.bo.vung_chon().is_none_or(|z| !z.ap_trang.ap_dung(t) || canh_tai(z, q).is_none()) {
                    self.bo.chon = self.bo.tim(t, pos_anh(q), (co_anh.0 as i64, co_anh.1 as i64), dpi as f64);
                }
                if let (Some(chon), Some(z)) = (self.bo.chon, self.bo.vung_chon().cloned()) {
                    if let Some(c) = canh_tai(&z, q) {
                        self.bo.ghi_buoc();
                        self.keo_vung = Some((chon, c, z, q));
                    }
                }
            }
        }
        if let (Some((chon, c, goc, q0)), Some(q)) = (self.keo_vung.clone(), ctx.pointer_latest_pos()) {
            if self.bo.chon == Some(chon) && ph.dragged() {
                let moi = keo_vung(&goc, c, (((q.x - q0.x) / k) as f64, ((q.y - q0.y) / k) as f64), (w as f64, h as f64), dpi as f64);
                if self.bo.vung_chon() != Some(&moi) {
                    if let Some(v) = self.bo.vung_chon_mut() {
                        *v = moi;
                    }
                    self.bo.sua_chon();
                }
            }
            if ph.drag_stopped() {
                self.keo_vung = None;
            }
        }
        if ph.clicked() {
            if let Some(q) = ph.interact_pointer_pos() {
                self.bo.chon = self.bo.tim(t, pos_anh(q), (co_anh.0 as i64, co_anh.1 as i64), dpi as f64);
            }
        }
    }
}
