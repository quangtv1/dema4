//! Vùng trên ảnh Gốc (như prototype): màu theo loại — góc / cạnh xanh lá, vẽ xoá xanh lam, bảo vệ hồng tím nét đứt,
//! vô đối cam; vùng chọn đậm hơn + nhãn tên; vùng chính có tay nắm ở các cạnh kéo được.
//!
//! Chuột: bấm chọn (Shift: thêm / bớt); kéo thân vùng vẽ = di chuyển (nhiều vùng đang chọn → cả nhóm); kéo tay nắm /
//! cạnh = đổi cỡ (góc / cạnh có sẵn: chỉ cạnh phía trong); kéo trên chỗ trống = khung quét kiểu AutoCAD (sang phải
//! **Window** — vùng nằm trọn; sang trái **Crossing** — vùng chạm khung); chuột phải = menu vùng. Chế độ vẽ: kéo để
//! vẽ vùng mới theo phạm vi ở cột lọc trang.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, StrokeKind};

use super::sua_vung::{canh_keo_duoc, keo_vung, Canh, ChonVung};
use super::thiet_ke as tk;
use super::widget::font;
use super::{ten_pham_vi, Keo, LocTrang, ManXoaGhim};
use crate::xoa_ghim::vung::{ApTrang, Hcn, LoaiVung, ViTri, Vung};

/// Khoảng bắt cạnh khi kéo đổi cỡ (điểm màn hình).
const BAT_CANH: f32 = 6.0;
/// Khung quét nhỏ hơn chừng này (điểm màn hình) coi như bấm.
const QUET_TOI_THIEU: f32 = 4.0;
/// Khoảng (điểm màn) hút điểm vẽ vào mép trang.
const HUT_MEP: f32 = 4.0;

/// (nền, viền, nét đứt) của vùng.
fn mau_vung(z: &Vung, chon: bool) -> (Color32, Color32, bool) {
    match z.loai {
        LoaiVung::BaoVe => (tk::VUNG_BAO_VE_NEN, tk::VUNG_BAO_VE_VIEN, true),
        LoaiVung::VoDoi => (if chon { tk::VUNG_VO_DOI_NEN_CHON } else { tk::VUNG_VO_DOI_NEN }, tk::VUNG_VO_DOI_VIEN, false),
        LoaiVung::Xoa if z.vi_tri != ViTri::Tay => (if chon { tk::VUNG_GOC_NEN_CHON } else { tk::VUNG_GOC_NEN }, tk::VUNG_GOC_VIEN, false),
        LoaiVung::Xoa => (if chon { tk::VUNG_VE_NEN_CHON } else { tk::VUNG_VE_NEN }, tk::VUNG_VE_VIEN, false),
    }
}

/// Màu khung xem trước khi đang vẽ vùng mới.
fn mau_ve_moi(loai: LoaiVung) -> (Color32, Color32, bool) {
    let z = Vung { loai, ..crate::xoa_ghim::vung::co_san(ViTri::GocTrenTrai) };
    mau_vung(&Vung { vi_tri: ViTri::Tay, ..z }, true)
}

/// Điểm kéo về trong trang `o`; cách mép ≤ `HUT_MEP` điểm màn thì hút đúng vào mép (vùng sát mép, không hở).
fn trong_trang(q: Pos2, o: Rect) -> Pos2 {
    let hut = |v: f32, a: f32, b: f32| {
        let v = v.clamp(a, b);
        if v - a <= HUT_MEP {
            a
        } else if b - v <= HUT_MEP {
            b
        } else {
            v
        }
    };
    egui::pos2(hut(q.x, o.left(), o.right()), hut(q.y, o.top(), o.bottom()))
}

fn vien_dut(p: &egui::Painter, r: Rect, s: Stroke) {
    let diem = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&diem, s, 5.0, 3.0));
}

impl ManXoaGhim {
    /// Khung các vùng trên ảnh Gốc của trang `t` (`o_goc` = ô ảnh trên màn) + thao tác chuột.
    pub(super) fn vung_tren_trang(&mut self, ui: &mut egui::Ui, t: u32, o_goc: Rect, ctx: &egui::Context) {
        let Some((co_anh, dpi)) = self.anh.get(&t).map(|a| (a.co_anh, a.dpi)) else { return };
        let (w, h) = (co_anh.0 as f32, co_anh.1 as f32);
        let kich = (co_anh.0 as i64, co_anh.1 as i64);
        let k = o_goc.width() / w;
        let khung_man = |z: &Vung| -> Option<Rect> {
            let kk = z.khung_xoa(kich.0, kich.1, dpi as f64)?;
            Some(Rect::from_min_max(o_goc.min + egui::vec2(kk.x1 as f32 * k, kk.y1 as f32 * k), o_goc.min + egui::vec2(kk.x2 as f32 * k, kk.y2 as f32 * k)))
        };
        let anh = |q: Pos2| (((q.x - o_goc.left()) / k) as f64, ((q.y - o_goc.top()) / k) as f64);
        let pos_anh = |q: Pos2| {
            let (x, y) = anh(q);
            (x as i64, y as i64)
        };
        let p = ui.painter().clone();
        // Khung nhận diện layout (vùng được bảo vệ khỏi xoá)
        if self.bo.tm.tuy_chon.nhan_dien {
            for hc in self.anh.get(&t).map(|a| a.ai.clone()).unwrap_or_default() {
                let r = Rect::from_min_max(o_goc.min + egui::vec2(hc.x1 as f32 * k, hc.y1 as f32 * k), o_goc.min + egui::vec2(hc.x2 as f32 * k, hc.y2 as f32 * k));
                p.rect_filled(r, 0.0, tk::NHAN_DIEN_NEN);
                p.rect_stroke(r, 0.0, Stroke::new(1.0, tk::NHAN_DIEN_VIEN), StrokeKind::Inside);
            }
        }
        // Vẽ vùng
        let chinh = self.bo.chinh();
        let mot = self.bo.chon.len() == 1;
        for c in self.bo.tat_ca() {
            let Some(z) = self.bo.lay(c) else { continue };
            if !z.ap_trang.ap_dung(t) {
                continue;
            }
            let Some(r) = khung_man(z) else { continue };
            let dang = self.bo.dang_chon(c);
            let (nen, vien, dut) = mau_vung(z, dang);
            p.rect_filled(r, 0.0, nen);
            let s = Stroke::new(if dang { 1.2 } else { 0.8 }, vien);
            if dut {
                vien_dut(&p, r, s);
            } else {
                p.rect_stroke(r, 0.0, s, StrokeKind::Inside);
            }
            if dang {
                let ten = self.ten_vung(c);
                let g = p.layout_no_wrap(ten, font(10.5), Color32::WHITE);
                let o_nhan = Rect::from_min_size(egui::pos2(r.left(), (r.top() - g.size().y - 4.0).max(o_goc.top())), g.size() + egui::vec2(8.0, 4.0));
                p.rect_filled(o_nhan, 2.0, vien);
                p.galley(o_nhan.min + egui::vec2(4.0, 2.0), g, Color32::WHITE);
            }
            if dang && mot && Some(c) == chinh {
                let duoc = canh_keo_duoc(z.vi_tri);
                let mut nam = Vec::new();
                let (x1, x2, y1, y2, cx, cy) = (r.left(), r.right(), r.top(), r.bottom(), r.center().x, r.center().y);
                if duoc.trai {
                    nam.push(egui::pos2(x1, cy));
                }
                if duoc.phai {
                    nam.push(egui::pos2(x2, cy));
                }
                if duoc.tren {
                    nam.push(egui::pos2(cx, y1));
                }
                if duoc.duoi {
                    nam.push(egui::pos2(cx, y2));
                }
                for (a, b, q) in [(duoc.trai, duoc.tren, egui::pos2(x1, y1)), (duoc.phai, duoc.tren, egui::pos2(x2, y1)), (duoc.trai, duoc.duoi, egui::pos2(x1, y2)), (duoc.phai, duoc.duoi, egui::pos2(x2, y2))] {
                    if a && b {
                        nam.push(q);
                    }
                }
                for q in nam {
                    let o = Rect::from_center_size(q, egui::vec2(tk::TAY_NAM, tk::TAY_NAM));
                    p.rect(o, 1.0, Color32::WHITE, Stroke::new(1.2, tk::TAY_NAM_VIEN), StrokeKind::Inside);
                }
            }
        }
        let ph = ui.interact(o_goc, ui.scope_id().with(("xg_trang", t)), Sense::click_and_drag());
        let shift = ctx.input(|i| i.modifiers.shift);
        let diem_dau = || ctx.input(|i| i.pointer.press_origin());
        // ----- Chế độ vẽ -----
        if let Some(loai) = self.ve {
            if ph.drag_started() {
                if let Some(q) = diem_dau().or(ph.interact_pointer_pos()) {
                    self.keo = Some(Keo::Moi { trang: t, dau: trong_trang(q, o_goc) });
                }
            }
            if let (Some(Keo::Moi { trang, dau }), Some(q)) = (self.keo.clone(), ctx.pointer_latest_pos()) {
                if trang == t {
                    let r = Rect::from_two_pos(trong_trang(dau, o_goc), trong_trang(q, o_goc));
                    let loai_hien = if loai == LoaiVung::Xoa && self.loc_trang == LocTrang::VoDoi { LoaiVung::VoDoi } else { loai };
                    let (nen, vien, dut) = mau_ve_moi(loai_hien);
                    p.rect_filled(r, 0.0, nen);
                    if dut {
                        vien_dut(&p, r, Stroke::new(1.0, vien));
                    } else {
                        p.rect_stroke(r, 0.0, Stroke::new(1.0, vien), StrokeKind::Inside);
                    }
                    self.goi_y_co(ctx, q, r.width() / o_goc.width(), r.height() / o_goc.height());
                    if ph.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
                        self.keo = None;
                        // Tỉ lệ theo ô trang trên màn (mép trang = đúng 0 / 1; đổi qua điểm ảnh rồi chia lại lệch chút ở mép dưới)
                        let ti = |q: Pos2| (((q.x - o_goc.left()) / o_goc.width()) as f64, ((q.y - o_goc.top()) / o_goc.height()) as f64);
                        let ((x1, y1), (x2, y2)) = (ti(r.min), ti(r.max));
                        self.them_ve((x1, y1, x2 - x1, y2 - y1), t);
                    }
                }
            }
            ph.on_hover_cursor(egui::CursorIcon::Crosshair);
            return;
        }
        // ----- Chọn / kéo -----
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
            (!c.than() || (z.vi_tri == ViTri::Tay && r.contains(q))).then_some(c)
        };
        let vung_chinh_trang = |s: &Self| s.bo.chinh().filter(|c| s.bo.lay(*c).is_some_and(|z| z.ap_trang.ap_dung(t)));
        if let Some(q) = ph.hover_pos() {
            if self.keo.is_none() {
                let c = vung_chinh_trang(self).and_then(|c| self.bo.lay(c)).and_then(|z| canh_tai(z, q));
                if let Some(c) = c {
                    ctx.set_cursor_icon(match (c.trai || c.phai, c.tren || c.duoi) {
                        (true, true) if c.trai == c.tren => egui::CursorIcon::ResizeNwSe,
                        (true, true) => egui::CursorIcon::ResizeNeSw,
                        (true, false) => egui::CursorIcon::ResizeHorizontal,
                        (false, true) => egui::CursorIcon::ResizeVertical,
                        _ => egui::CursorIcon::Move,
                    });
                }
            }
        }
        if ph.drag_started() {
            if let Some(q) = diem_dau().or(ph.interact_pointer_pos()) {
                self.bat_dau_keo(t, q, shift, &canh_tai);
            }
        }
        match self.keo.clone() {
            Some(Keo::Vung { trang, chon, canh, goc, dau }) if trang == t => {
                if let Some(q) = ctx.pointer_latest_pos() {
                    if ph.dragged() {
                        let moi = keo_vung(&goc, canh, (((q.x - dau.x) / k) as f64, ((q.y - dau.y) / k) as f64), (w as f64, h as f64), dpi as f64);
                        if self.bo.lay(chon) != Some(&moi) {
                            if let Some(v) = self.bo.lay_mut(chon) {
                                *v = moi.clone();
                            }
                            self.bo.sua_chon();
                        }
                        if let Some(r) = khung_man(&moi) {
                            self.goi_y_co(ctx, q, r.width() / o_goc.width(), r.height() / o_goc.height());
                        }
                    }
                }
                if ph.drag_stopped() {
                    self.keo = None;
                }
            }
            Some(Keo::Nhom { trang, goc, dau }) if trang == t => {
                if let (Some(q), true) = (ctx.pointer_latest_pos(), ph.dragged()) {
                    let d = (((q.x - dau.x) / k) as f64, ((q.y - dau.y) / k) as f64);
                    for (c, g) in &goc {
                        let moi = keo_vung(g, Canh::default(), d, (w as f64, h as f64), dpi as f64);
                        if let Some(v) = self.bo.lay_mut(*c) {
                            *v = moi;
                        }
                    }
                    self.bo.sua_chon();
                }
                if ph.drag_stopped() {
                    self.keo = None;
                }
            }
            Some(Keo::Quet { trang, dau, them }) if trang == t => {
                if let Some(q) = ctx.pointer_latest_pos() {
                    let r = Rect::from_two_pos(dau.clamp(o_goc.min, o_goc.max), q.clamp(o_goc.min, o_goc.max));
                    let cat = q.x < dau.x;
                    if cat {
                        p.rect_filled(r, 0.0, tk::QUET_CROSSING_NEN);
                        vien_dut(&p, r, Stroke::new(1.0, tk::QUET_CROSSING_VIEN));
                    } else {
                        p.rect_filled(r, 0.0, tk::QUET_WINDOW_NEN);
                        p.rect_stroke(r, 0.0, Stroke::new(1.0, tk::QUET_WINDOW_VIEN), StrokeKind::Inside);
                    }
                    if ph.drag_stopped() || !ctx.input(|i| i.pointer.primary_down()) {
                        self.keo = None;
                        if r.width().max(r.height()) < QUET_TOI_THIEU {
                            if !them {
                                self.bo.chon.clear();
                            }
                        } else {
                            let (a, b) = (pos_anh(r.min), pos_anh(r.max));
                            let ds = self.bo.quet(t, Hcn { x1: a.0, y1: a.1, x2: b.0, y2: b.1 }, cat, kich, dpi as f64);
                            if them {
                                self.bo.chon_them(ds);
                            } else {
                                self.bo.chon = ds;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if ph.clicked() {
            if let Some(q) = ph.interact_pointer_pos() {
                match self.bo.tim(t, pos_anh(q), kich, dpi as f64) {
                    Some(c) if shift => self.bo.chon_dao(c),
                    Some(c) => self.bo.chon_mot(Some(c)),
                    None if !shift => self.bo.chon.clear(),
                    None => {}
                }
            }
        }
        if ph.secondary_clicked() {
            if let Some(q) = ph.interact_pointer_pos() {
                if let Some(c) = self.bo.tim(t, pos_anh(q), kich, dpi as f64) {
                    if !self.bo.dang_chon(c) {
                        self.bo.chon_mot(Some(c));
                    }
                }
            }
        }
        ph.context_menu(|ui| self.menu_vung(ui, t, kich, dpi as f64));
    }

    /// Bắt đầu kéo ở điểm `q`: tay nắm / thân vùng chính → đổi cỡ / di chuyển (nhiều vùng vẽ đang chọn → cả nhóm);
    /// trên 1 vùng khác → chọn rồi kéo; chỗ trống → khung quét.
    fn bat_dau_keo(&mut self, t: u32, q: Pos2, shift: bool, canh_tai: &dyn Fn(&Vung, Pos2) -> Option<Canh>) {
        let ap = |s: &Self, c: ChonVung| s.bo.lay(c).is_some_and(|z| z.ap_trang.ap_dung(t));
        let chinh = self.bo.chinh().filter(|c| ap(self, *c));
        let tren_chinh = chinh.and_then(|c| self.bo.lay(c).and_then(|z| canh_tai(z, q)).map(|canh| (c, canh)));
        let (c, canh) = match tren_chinh {
            Some(x) => x,
            None => {
                // Vùng trên cùng tại điểm bấm (vùng riêng / vẽ sau ưu tiên)
                let tim = self.bo.tat_ca().into_iter().rev().find(|c| ap(self, *c) && self.bo.lay(*c).is_some_and(|z| canh_tai(z, q).is_some_and(|x| x.than()) || (z.vi_tri != ViTri::Tay && canh_tai(z, q).is_some())));
                match tim {
                    Some(c) => {
                        if shift {
                            self.bo.chon_them([c]);
                        } else if !self.bo.dang_chon(c) {
                            self.bo.chon_mot(Some(c));
                        }
                        let canh = self.bo.lay(c).and_then(|z| canh_tai(z, q)).unwrap_or_default();
                        (c, canh)
                    }
                    None => {
                        self.keo = Some(Keo::Quet { trang: t, dau: q, them: shift });
                        return;
                    }
                }
            }
        };
        let z = self.bo.lay(c).cloned();
        let Some(z) = z else { return };
        // Góc / cạnh có sẵn chỉ đổi cỡ, không di chuyển thân
        if canh.than() && z.vi_tri != ViTri::Tay {
            return;
        }
        self.bo.ghi_buoc();
        let nhom: Vec<(ChonVung, Vung)> = self.bo.chon.iter().filter(|x| ap(self, **x)).filter_map(|x| self.bo.lay(*x).filter(|v| v.vi_tri == ViTri::Tay).map(|v| (*x, v.clone()))).collect();
        self.keo = if canh.than() && nhom.len() > 1 { Some(Keo::Nhom { trang: t, goc: nhom, dau: q }) } else { Some(Keo::Vung { trang: t, chon: c, canh, goc: z, dau: q }) };
    }

    /// Nhãn cỡ "Rộng x% · Cao y%" cạnh con trỏ khi vẽ / kéo.
    fn goi_y_co(&self, ctx: &egui::Context, q: Pos2, rong: f32, cao: f32) {
        egui::Area::new(egui::Id::unique("xg_goi_y_co")).order(egui::Order::Tooltip).fixed_pos(q + egui::vec2(14.0, 14.0)).interactable(false).show(ctx, |ui| {
            egui::Frame::new().fill(tk::NEN_TOOLTIP).corner_radius(3).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                ui.label(egui::RichText::new(format!("{:.0}% × {:.0}%", rong * 100.0, cao * 100.0)).font(font(tk::CHU_NHO)).color(Color32::WHITE));
            });
        });
    }

    /// Menu chuột phải trên trang Gốc: mỗi dòng có biểu tượng đầu dòng.
    fn menu_vung(&mut self, ui: &mut egui::Ui, t: u32, kich: (i64, i64), dpi: f64) {
        ui.set_min_width(230.0);
        let muc = |ui: &mut egui::Ui, bt: &str, chu: &str, duoc: bool| {
            let anh = super::bieu_tuong::anh(bt, 14.0, 14.0);
            let anh = if duoc { anh } else { anh.tint(Color32::from_white_alpha(110)) };
            ui.add_enabled(duoc, egui::Button::image_and_text(anh, egui::RichText::new(chu).font(font(tk::CHU_CO))).frame(false).min_size(egui::vec2(0.0, 24.0))).clicked()
        };
        if self.bo.chon.is_empty() {
            if muc(ui, "m_ve_xoa", "Vẽ vùng xoá", true) {
                self.ve = Some(LoaiVung::Xoa);
                ui.close();
            }
            if muc(ui, "m_ve_bao", "Vẽ vùng bảo vệ", true) {
                self.ve = Some(LoaiVung::BaoVe);
                ui.close();
            }
            if muc(ui, "m_chon_het", "Chọn tất cả (Ctrl+A)", !self.bo.tat_ca().is_empty()) {
                self.bo.chon = self.bo.tat_ca();
                ui.close();
            }
        } else {
            let tieu_de = if self.bo.chon.len() == 1 { self.bo.chinh().map(|c| self.ten_vung(c)).unwrap_or_default() } else { format!("{} vùng đang chọn", self.bo.chon.len()) };
            ui.label(egui::RichText::new(tieu_de).font(super::widget::font_dam(tk::CHU_CO)).color(tk::CHU_PHU));
            ui.separator();
            let ve = self.bo.chi_vung_ve(&self.bo.chon.clone());
            if muc(ui, "m_xoa", &format!("Xóa vùng{} (Delete)", if ve.len() > 1 { format!(" ({})", ve.len()) } else { String::new() }), !ve.is_empty()) {
                let n = self.bo.xoa(&ve);
                self.bao(format!("Đã xoá {n} vùng · Ctrl+Z để hoàn tác"));
                ui.close();
            }
            if self.bo.chon.len() == 1 {
                let c = self.bo.chinh().expect("có vùng chọn");
                let z = self.bo.lay(c).cloned();
                if let Some(z) = z {
                    let doi = if z.loai == LoaiVung::BaoVe { "Đổi thành vùng xoá" } else { "Đổi thành vùng bảo vệ" };
                    if muc(ui, "m_doi", doi, z.vi_tri == ViTri::Tay) {
                        self.bo.doi_loai(c);
                        ui.close();
                    }
                    if muc(ui, "m_nhan_ban", "Nhân bản", true) {
                        if !self.bo.nhan_ban(c, kich, dpi) {
                            self.bao("Không nhân bản được (PDF chưa đọc được hoặc đã đủ số vùng)");
                        }
                        ui.close();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(format!("Áp cho: {}", ten_pham_vi(z.ap_trang))).font(font(tk::CHU_NHO)).color(tk::CHU_PHU));
                    let vo_doi = z.loai == LoaiVung::VoDoi;
                    for (ap, ten) in [(ApTrang::TatCa, "Mọi trang"), (ApTrang::Le, "Trang lẻ"), (ApTrang::Chan, "Trang chẵn"), (ApTrang::MotTrang(t), "Chỉ trang này")] {
                        let dang = z.ap_trang == ap;
                        if muc(ui, if dang { "m_chon" } else { "m_trang" }, ten, !dang) {
                            self.bo.doi_ap_trang(c, ap, vo_doi);
                            ui.close();
                        }
                    }
                }
            }
        }
        ui.separator();
        if muc(ui, "m_hoan_tac", "Hoàn tác (Ctrl+Z)", self.bo.co_hoan_tac()) {
            self.bo.hoan_tac();
            ui.close();
        }
    }
}
