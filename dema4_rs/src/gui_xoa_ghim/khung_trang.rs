//! Khung giữa màn Xóa ghim (như prototype): hàng tiêu đề **Gốc** (phủ cả cột trang thu nhỏ; ◂ ▸ đổi file, luôn hiện,
//! tooltip ngay) | **Đích** (› thu gọn), cân giữa; bên dưới cột **trang thu nhỏ** (chỉ khi đã mở file / thư mục) +
//! khung xem Gốc và Đích **rộng bằng nhau**, mỗi khung 1 vùng cuộn, cuộn đồng bộ. Thanh đáy: đếm vùng, chuyển trang,
//! kiểu xem, vừa ngang / vừa cao, tỉ lệ, − +, nút **Xóa ghim (n)**.
//!
//! Chỉ trang đang thấy mới xin ảnh; ảnh Đích làm lại khi vùng / tuỳ chọn đổi (mã cấu hình khác), trong lúc chờ vẫn
//! hiện ảnh cũ. Giữ texture vài trang quanh trang đang xem.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind};

use super::thiet_ke as tk;
use super::widget::{self, font};
use super::xem_truoc::YeuCau;
use super::{bieu_tuong, Keo, ManXoaGhim, TyLe};

const GIU_QUANH: u32 = 4;
const CAC_PHONG: [u16; 6] = [25, 50, 75, 100, 125, 150];
const PHONG_MIN: f32 = 0.2;
const PHONG_MAX: f32 = 1.5;

/// Bố cục trang trong 1 khung xem: (trang, đỉnh y, cỡ màn hình).
type BoCuc = Vec<(u32, f32, egui::Vec2)>;

/// Khoảng cách dọc từ `y` tới trang (0 khi nằm trong trang).
fn khoang(y: f32, &(_, y0, co): &(u32, f32, egui::Vec2)) -> f32 {
    if y < y0 {
        y0 - y
    } else {
        (y - y0 - co.y).max(0.0)
    }
}

impl ManXoaGhim {
    // ---------------- Thanh đáy ----------------

    pub(super) fn thanh_duoi(&mut self, ui: &mut egui::Ui) {
        // Như prototype: trái = đếm vùng · giữa vùng xem = nhóm chuyển trang / kiểu xem / tỉ lệ · phải = Xóa ghim (n)
        let o = ui.max_rect();
        let (chung, rieng) = self.bo.dem();
        ui.painter().text(egui::pos2(o.left(), o.center().y), egui::Align2::LEFT_CENTER, format!("Zone chung: {chung}; Zone riêng: {rieng}"), font(tk::CHU_CO), tk::CHU_PHU);
        let mut phai = ui.new_child(egui::UiBuilder::new().max_rect(o).layout(egui::Layout::right_to_left(egui::Align::Center)));
        self.nut_chay(&mut phai);
        // Nhóm giữa: canh giữa theo bề rộng đo được ở khung trước
        let id = ui.scope_id().with("xg_nhom_giua");
        let rong = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(560.0);
        let x0 = (o.center().x - rong / 2.0).max(o.left() + 170.0);
        let mut giua = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(egui::pos2(x0, o.top()), o.max)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        giua.spacing_mut().item_spacing.x = 6.0;
        let co = !self.kho_trang.is_empty();
        let n = self.so_trang_xem();
        if widget::nut_bt(&mut giua, "trai", (8.0, 8.0), 26.0, 24.0, self.trang > 0, false, "Trang trước").on_hover_text("Trang trước (PageUp)").clicked() {
            self.sang_trang(self.trang.saturating_sub(1));
        }
        if widget::nut_bt(&mut giua, "phai", (8.0, 8.0), 26.0, 24.0, self.trang + 1 < n, false, "Trang sau").on_hover_text("Trang sau (PageDown)").clicked() {
            self.sang_trang(self.trang + 1);
        }
        let mut so = (self.trang + 1) as f32;
        if widget::o_so_canh(&mut giua, &mut so, 1.0, n.max(1) as f32, "", co, "Trang", egui::Align2::CENTER_CENTER) {
            self.sang_trang(so as u32 - 1);
        }
        giua.label(egui::RichText::new(format!("/ {n}")).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
        giua.add_space(8.0);
        let tung = self.cfg.tung_trang;
        giua.add_enabled_ui(co, |ui| {
            egui::ComboBox::from_id_salt("xg_kieu_xem")
                .icon(widget::mui_ten)
                .width(110.0)
                .selected_text(egui::RichText::new(if tung { "Từng trang" } else { "Liên tiếp" }).font(font(tk::CHU_CO)))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(!tung, egui::RichText::new("Liên tiếp").font(font(tk::CHU_CO))).clicked() && tung {
                        self.doi_cfg(|c| c.tung_trang = false);
                        self.cuon_toi = Some(self.trang);
                    }
                    if ui.selectable_label(tung, egui::RichText::new("Từng trang").font(font(tk::CHU_CO))).clicked() && !tung {
                        self.doi_cfg(|c| c.tung_trang = true);
                    }
                })
                .response
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Kiểu xem"));
        });
        giua.add_space(8.0);
        if widget::nut_bt(&mut giua, "vua_ngang", (16.0, 12.0), 28.0, 24.0, co, self.ty_le == TyLe::VuaNgang, "Vừa bề ngang").on_hover_text("Vừa bề ngang khung (=)").clicked() {
            self.ty_le = TyLe::VuaNgang;
            self.cuon_toi = Some(self.trang);
        }
        if widget::nut_bt(&mut giua, "vua_cao", (12.0, 16.0), 28.0, 24.0, co, self.ty_le == TyLe::VuaCao, "Vừa chiều cao").on_hover_text("Vừa chiều cao khung").clicked() {
            self.ty_le = TyLe::VuaCao;
            self.cuon_toi = Some(self.trang);
        }
        giua.add_space(8.0);
        let hien = (self.ty_le_cuoi * 100.0).round() as u16;
        giua.add_enabled_ui(co, |ui| {
            egui::ComboBox::from_id_salt("xg_phong")
                .icon(widget::mui_ten)
                .width(84.0)
                .selected_text(egui::RichText::new(format!("{hien}%")).font(font(tk::CHU_CO)))
                .show_ui(ui, |ui| {
                    for p in CAC_PHONG {
                        if ui.selectable_label(p == hien && matches!(self.ty_le, TyLe::CoDinh(_)), egui::RichText::new(format!("{p}%")).font(font(tk::CHU_CO))).clicked() {
                            self.ty_le = TyLe::CoDinh(p as f32 / 100.0);
                            self.cuon_toi = Some(self.trang);
                        }
                    }
                })
                .response
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Tỉ lệ"));
        });
        if widget::nut_bt(&mut giua, "tru", (12.0, 12.0), 26.0, 24.0, co, false, "Thu nhỏ").on_hover_text("Thu nhỏ (−)").clicked() {
            self.phong(-10.0);
        }
        if widget::nut_bt(&mut giua, "cong", (12.0, 12.0), 26.0, 24.0, co, false, "Phóng to").on_hover_text("Phóng to (+) · Ctrl + cuộn chuột").clicked() {
            self.phong(10.0);
        }
        let do_duoc = giua.min_rect().width();
        if (do_duoc - rong).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(id, do_duoc));
            ui.ctx().request_repaint();
        }
    }

    // ---------------- Khung giữa ----------------

    pub(super) fn khung_trang(&mut self, ui: &mut egui::Ui) {
        let tong = ui.available_rect_before_wrap();
        let nho = self.rong_nho();
        let co_dich = self.cfg.khung_dich;
        let x = if co_dich { ((tong.width() - tk::RONG_RANH - nho) / 2.0).max(40.0) } else { (tong.width() - nho).max(40.0) };
        let cao_td = tk::CAO_TIEU_DE_KHUNG;
        // Hàng tiêu đề
        let o_td_goc = Rect::from_min_size(tong.min, egui::vec2(nho + x, cao_td));
        let o_td_dich = Rect::from_min_max(egui::pos2(o_td_goc.right() + tk::RONG_RANH, tong.top()), egui::pos2(tong.right(), tong.top() + cao_td));
        self.tieu_de_goc(ui, o_td_goc, !co_dich);
        if co_dich {
            self.tieu_de_dich(ui, o_td_dich);
        }
        // Thân
        let than = Rect::from_min_max(egui::pos2(tong.left(), tong.top() + cao_td), tong.max);
        let p = ui.painter().clone();
        p.rect_filled(than, 0.0, tk::NEN_XEM);
        let o_nho = Rect::from_min_size(than.min, egui::vec2(nho, than.height()));
        let o_goc = Rect::from_min_size(egui::pos2(than.left() + nho, than.top()), egui::vec2(x, than.height()));
        let o_dich = Rect::from_min_max(egui::pos2(o_goc.right() + tk::RONG_RANH, than.top()), than.max);
        if co_dich {
            let ranh = Rect::from_min_max(egui::pos2(o_goc.right(), tong.top()), egui::pos2(o_dich.left(), tong.bottom()));
            p.rect_filled(ranh, 0.0, tk::NEN_RANH);
            p.vline(ranh.left() + 0.5, ranh.y_range(), Stroke::new(1.0, tk::VIEN_NHAT));
            p.vline(ranh.right() - 0.5, ranh.y_range(), Stroke::new(1.0, tk::VIEN_NHAT));
        }
        ui.allocate_rect(tong, Sense::hover());
        if self.thu_muc.is_none() {
            self.man_trong(ui, o_goc, than);
            return;
        }
        if nho > 0.0 {
            self.cot_anh_nho(ui, o_nho);
        }
        if self.ds.is_empty() || self.kho_trang.is_empty() {
            let chu = self.loi.clone();
            let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o_goc));
            con.centered_and_justified(|ui| match chu {
                Some(e) => {
                    ui.label(egui::RichText::new(e).font(font(tk::CHU_CO)).color(tk::DO_NUT));
                }
                None => {
                    ui.spinner();
                }
            });
            return;
        }
        // Ctrl + cuộn chuột trên khung xem: phóng
        let vung_xem = Rect::from_min_max(o_goc.min, if co_dich { o_dich.max } else { o_goc.max });
        if ui.rect_contains_pointer(vung_xem) {
            let z = ui.ctx().input(|i| i.zoom_delta());
            if z != 1.0 {
                self.ty_le = TyLe::CoDinh((self.ty_le_cuoi * z).clamp(PHONG_MIN, PHONG_MAX));
            }
        }
        let rong_max = self.kho_trang.iter().map(|k| k.0).fold(1.0, f32::max);
        // Vừa ngang / vừa cao: **mỗi trang một tỉ lệ** cho vừa khung (file trộn dọc / ngang: trang nào cũng đúng bề
        // ngang khung) → cuộn qua trang khác khổ không đổi phóng, vừa ngang không còn cuộn ngang
        let cao_vua = o_goc.height() - tk::DEM_TREN_TRANG - tk::DEM_DUOI_TRANG;
        let ty_trang = |(w, h): (f32, f32)| {
            match self.ty_le {
                TyLe::VuaNgang => (x - tk::LE_VUA) / w.max(1.0),
                TyLe::VuaCao => cao_vua / h.max(1.0),
                TyLe::CoDinh(v) => v,
            }
            .max(0.05)
        };
        let ty = ty_trang(self.kho_trang.get(self.trang as usize).copied().unwrap_or((rong_max, 1.0)));
        // Bố cục chỉ đổi khi đổi chế độ / tỉ lệ cố định / cỡ khung — không phải khi cuộn sang trang khác khổ.
        // Đổi bố cục: trục giữa khung trùng trục giữa trang; chiều dọc giữ điểm đang ở giữa khung
        let khoa = match self.ty_le {
            TyLe::VuaNgang => (0, x),
            TyLe::VuaCao => (1, cao_vua),
            TyLe::CoDinh(v) => (2, v),
        };
        let doi_ty = self.khoa_bo_cuc.is_some_and(|k| k.0 != khoa.0 || (k.1 - khoa.1).abs() > 1e-4);
        self.khoa_bo_cuc = Some(khoa);
        self.ty_le_cuoi = ty;
        let n = self.kho_trang.len() as u32;
        let cac: Vec<u32> = if self.cfg.tung_trang { vec![self.trang.min(n - 1)] } else { (0..n).collect() };
        let mut bo_cuc: BoCuc = Vec::with_capacity(cac.len());
        let mut y = tk::DEM_TREN_TRANG;
        let mut rong_noi: f32 = 0.0;
        for &t in &cac {
            let (w, h) = self.kho_trang[t as usize];
            let k = ty_trang((w, h));
            bo_cuc.push((t, y, egui::vec2(w * k, h * k)));
            rong_noi = rong_noi.max(w * k);
            y += h * k + tk::DEM_DUOI_TRANG.min(tk::KHE_TRANG + 12.0);
        }
        let noi = egui::vec2((rong_noi + tk::LE_VUA).max(x), y + tk::DEM_DUOI_TRANG);
        // Neo ngang: trục đứng trang đang xem ở trục giữa khung — giữ mỗi khung hình (kể cả khi ScrollArea còn kẹp
        // theo nội dung cũ) tới khi người dùng tự cuộn ngang; đổi tỉ lệ / đổi bề rộng khung (ẩn hiện Danh sách, Đích) neo lại
        let rong_xem = o_goc.width();
        if doi_ty || (rong_xem - self.rong_xem_cu).abs() > 0.5 {
            self.neo_ngang = true;
        }
        self.rong_xem_cu = rong_xem;
        if ui.rect_contains_pointer(vung_xem) && ui.input(|i| i.smooth_scroll_delta.x != 0.0) {
            self.neo_ngang = false;
        }
        // Neo dọc: điểm giữa khung = (trang, tỉ lệ theo chiều cao trang) đo trên bố cục cũ, đặt lại đúng chỗ đó trên
        // bố cục mới (khoảng cách giữa trang không co giãn theo tỉ lệ nên không nhân thẳng offset). Đang neo (phóng liên
        // tục) thì giữ điểm neo cũ, không đo lại từ offset có thể đã bị ScrollArea kẹp → không trôi.
        let cao_xem = o_goc.height();
        if doi_ty && self.cuon_toi.is_none() {
            let tam = self.cuon_goc.y + cao_xem / 2.0;
            let diem = self.neo_doc.map(|(t, f, _)| (t, f)).or_else(|| {
                self.bo_cuc_cu
                    .iter()
                    .min_by(|a, b| khoang(tam, a).total_cmp(&khoang(tam, b)))
                    .map(|&(t, y0, co)| (t, ((tam - y0) / co.y.max(1.0)).clamp(0.0, 1.0)))
            });
            self.neo_doc = diem.map(|(t, f)| (t, f, 4));
        }
        self.bo_cuc_cu = bo_cuc.clone();
        let x_neo = ((noi.x - rong_xem) / 2.0).max(0.0);
        // Ép cuộn: tới trang (chuyển trang / file) hoặc theo khung kia (cuộn đồng bộ)
        let toi = self.cuon_toi.take().and_then(|t| bo_cuc.iter().find(|(x, _, _)| *x == t).map(|(_, y0, _)| (y0 - tk::DEM_TREN_TRANG).max(0.0)));
        let ep = self.ep_cuon.take();
        let mut ep_goc = toi.map(|y| egui::vec2(self.cuon_goc.x, y)).or(ep.filter(|e| e.0).map(|e| e.1));
        let mut ep_dich = toi.map(|y| egui::vec2(self.cuon_dich.x, y)).or(ep.filter(|e| !e.0).map(|e| e.1));
        let neo_doc = self.neo_doc.take();
        if let Some((t, f, n)) = neo_doc {
            if n > 1 {
                self.neo_doc = Some((t, f, n - 1));
                ui.ctx().request_repaint();
            }
        }
        let y_neo = neo_doc.and_then(|(t, f, _)| bo_cuc.iter().find(|b| b.0 == t).map(|&(_, y0, co)| (y0 + f * co.y - cao_xem / 2.0).max(0.0)));
        for (ep, cu, y_neo) in [(&mut ep_goc, self.cuon_goc, y_neo), (&mut ep_dich, self.cuon_dich, y_neo)] {
            let mut v = ep.unwrap_or(cu);
            if let Some(y) = y_neo.filter(|_| toi.is_none()) {
                v.y = y;
            }
            if self.neo_ngang {
                v.x = x_neo;
            }
            if ep.is_some() || (v - cu).length() > 0.5 {
                *ep = Some(v);
            }
        }
        let (og, trang_tren) = self.khung_xem(ui, o_goc, true, &bo_cuc, noi, ep_goc);
        let od = if co_dich { self.khung_xem(ui, o_dich, false, &bo_cuc, noi, ep_dich).0 } else { og };
        // Kéo thanh cuộn ngang → bỏ neo (người dùng muốn tự xem lệch)
        if self.neo_ngang && ui.input(|i| i.pointer.any_down()) && ((ep_goc.is_none() && (og.x - self.cuon_goc.x).abs() > 0.5) || (co_dich && ep_dich.is_none() && (od.x - self.cuon_dich.x).abs() > 0.5)) {
            self.neo_ngang = false;
        }
        if self.cfg.cuon_cung && co_dich {
            if og != self.cuon_goc && ep_goc.is_none() {
                self.ep_cuon = Some((false, og));
            } else if od != self.cuon_dich && ep_dich.is_none() {
                self.ep_cuon = Some((true, od));
            }
            if self.ep_cuon.is_some() {
                ui.ctx().request_repaint();
            }
        }
        self.cuon_goc = og;
        self.cuon_dich = od;
        if !self.cfg.tung_trang {
            if let Some(t) = trang_tren {
                self.trang = t;
            }
        }
        // Bỏ texture trang ở xa trang đang xem (giới hạn bộ nhớ đồ hoạ)
        let giu = self.trang.saturating_sub(GIU_QUANH)..=self.trang + GIU_QUANH;
        self.anh.retain(|t, _| giu.contains(t));
    }

    /// 1 khung xem (Gốc hoặc Đích) ở `o`: vùng cuộn 2 chiều chứa các trang theo `bo_cuc`. Trả offset cuộn + trang
    /// đang ở trên cùng (chỉ khung Gốc tính).
    fn khung_xem(&mut self, ui: &mut egui::Ui, o: Rect, la_goc: bool, bo_cuc: &BoCuc, noi: egui::Vec2, ep: Option<egui::Vec2>) -> (egui::Vec2, Option<u32>) {
        let (ch, ma) = self.cau_hinh_xem();
        let ctx = ui.ctx().clone();
        let mut trang_tren = None;
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o).layout(egui::Layout::top_down(egui::Align::Min)));
        con.set_clip_rect(o);
        let mut sa = egui::ScrollArea::both().id_salt(("xg_cuon", la_goc, self.the_he, self.cfg.tung_trang)).auto_shrink([false, false]);
        if let Some(e) = ep {
            sa = sa.scroll_offset(e);
        }
        let ra = sa.show(&mut con, |ui| {
            let (r, _) = ui.allocate_exact_size(noi, Sense::hover());
            let nhin = ui.clip_rect();
            let o_cua = |y0: f32, co: egui::Vec2| Rect::from_min_size(egui::pos2(r.left() + ((r.width() - co.x) / 2.0).max(tk::LE_VUA / 2.0), r.top() + y0), co);
            // Kéo bắt đầu ở nền xám ngoài trang (khung Gốc): vẽ vùng (− / +) hoặc quét chọn vẫn được — điểm đầu kéo về
            // điểm gần nhất trên mép trang gần nhất. Đăng ký trước các trang nên trên trang thì trang nhận chuột.
            if la_goc {
                let nen = ui.interact(nhin, ui.scope_id().with("xg_nen_keo"), Sense::drag());
                if nen.drag_started() {
                    if let Some(q) = ctx.input(|i| i.pointer.press_origin()) {
                        let gan = bo_cuc.iter().map(|&(t, y0, co)| (t, o_cua(y0, co))).min_by(|a, b| a.1.distance_sq_to_pos(q).total_cmp(&b.1.distance_sq_to_pos(q)));
                        if let Some((t, o)) = gan {
                            let dau = q.clamp(o.min, o.max);
                            let them = ctx.input(|i| i.modifiers.shift);
                            self.keo = Some(if self.ve.is_some() { Keo::Moi { trang: t, dau } } else { Keo::Quet { trang: t, dau, them } });
                        }
                    }
                }
                if self.ve.is_some() {
                    nen.on_hover_cursor(egui::CursorIcon::Crosshair);
                }
            }
            for &(t, y0, co) in bo_cuc {
                let o_trang = o_cua(y0, co);
                if la_goc && trang_tren.is_none() && o_trang.bottom() > nhin.top() + nhin.height() * 0.3 {
                    trang_tren = Some(t);
                }
                if !nhin.expand(co.y * 0.5).intersects(o_trang) {
                    continue;
                }
                let can = self.anh.get(&t).is_none_or(|a| a.ma != ma);
                if can && !self.cho_anh.contains(&(t, ma)) {
                    self.cho_anh.retain(|(x, _)| *x != t);
                    self.cho_anh.insert((t, ma));
                    self.xem.gui(YeuCau::Trang { the_he: self.the_he, trang: t, ma, ch: ch.clone() });
                }
                let p = ui.painter();
                p.rect_filled(o_trang.translate(egui::vec2(0.0, 2.0)), 0.0, Color32::from_black_alpha(28));
                p.rect_filled(o_trang, 0.0, if la_goc { tk::NEN_TRANG_GOC } else { Color32::WHITE });
                let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                match self.anh.get(&t) {
                    Some(a) => {
                        p.image(if la_goc { a.goc.id() } else { a.dich.id() }, o_trang, uv, Color32::WHITE);
                        if !la_goc && a.ma != ma {
                            p.text(o_trang.right_top() + egui::vec2(-8.0, 8.0), egui::Align2::RIGHT_TOP, "đang cập nhật…", font(tk::CHU_NHO), tk::CHU_PHU);
                        }
                    }
                    None => {
                        p.text(o_trang.center(), egui::Align2::CENTER_CENTER, "Đang vẽ trang…", font(tk::CHU_CO), tk::CHU_PHU);
                    }
                }
                p.rect_stroke(o_trang, 0.0, Stroke::new(1.0, tk::VIEN), StrokeKind::Outside);
                p.text(egui::pos2(o_trang.center().x, o_trang.bottom() + 10.0), egui::Align2::CENTER_CENTER, format!("{}", t + 1), font(tk::CHU_NHO), tk::CHU_PHU);
                if la_goc {
                    self.vung_tren_trang(ui, t, o_trang, &ctx);
                }
            }
        });
        (ra.state.clamped_offset(), trang_tren)
    }

    /// Tiêu đề khung Gốc: ◂ ▸ (i/n) cố định bên trái; "Gốc: <đường dẫn đầy đủ>" cân giữa cả khung (chừa 70 px 2 bên).
    /// `mo_dich` = khung Đích đang thu → nút ‹ bên phải mở lại.
    fn tieu_de_goc(&mut self, ui: &mut egui::Ui, o: Rect, mo_dich: bool) {
        let p = ui.painter();
        p.rect_filled(o, 0.0, tk::NEN_TIEU_DE_KHUNG);
        p.hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, tk::VIEN_KHUNG));
        let ten = self.ds.get(self.chon).map(|(n, _)| n.clone());
        let chu = format!("Gốc: {}", ten.as_ref().map_or(String::new(), |n| n.display().to_string()));
        let co_nut = self.thu_muc.is_some() && self.mot_file.is_none() && !self.ds.is_empty();
        // Chừa chỗ nhóm ◂ ▸ (i/n) bên trái (cân đối 2 bên để chữ vẫn ở giữa khung)
        let so = format!("({}/{})", self.chon + 1, self.ds.len());
        let rong_nhom = if co_nut { 6.0 + 2.0 * tk::RONG_NUT_VB + 4.0 + 8.0 + ui.painter().layout_no_wrap(so.clone(), font(tk::CHU_CO), tk::CHU_PHU).size().x + 10.0 } else { 0.0 };
        tieu_de_giua(ui, o, &chu, rong_nhom.max(if mo_dich { 40.0 } else { 0.0 }));
        if co_nut {
            let mut con = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(o.min + egui::vec2(6.0, 0.0), egui::pos2(o.left() + 6.0 + 2.0 * tk::RONG_NUT_VB + 80.0, o.bottom()))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            con.spacing_mut().item_spacing.x = 4.0;
            let r = widget::nut_bt(&mut con, "trai", (8.0, 8.0), tk::RONG_NUT_VB, tk::CAO_NUT_NHO, self.chon > 0, false, "Văn bản trước");
            widget::tooltip_ngay(&con, &r, "Văn bản trước  ( [ )");
            if r.clicked() && self.chon > 0 {
                self.chon_file(self.chon - 1);
            }
            let r = widget::nut_bt(&mut con, "phai", (8.0, 8.0), tk::RONG_NUT_VB, tk::CAO_NUT_NHO, self.chon + 1 < self.ds.len(), false, "Văn bản sau");
            widget::tooltip_ngay(&con, &r, "Văn bản sau  ( ] )");
            if r.clicked() {
                self.chon_file(self.chon + 1);
            }
            con.add_space(4.0);
            con.label(egui::RichText::new(so).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
        }
        if mo_dich {
            let o_nut = Rect::from_min_size(egui::pos2(o.right() - 6.0 - 26.0, o.top() + 4.0), egui::vec2(26.0, 22.0));
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(o_nut));
            if widget::nut_mui_ten(&mut c, widget::Huong::Trai, 26.0, 22.0, "Mở khung Đích").on_hover_text("Mở khung Đích").clicked() {
                self.doi_cfg(|c| c.khung_dich = true);
            }
        }
    }

    fn tieu_de_dich(&mut self, ui: &mut egui::Ui, o: Rect) {
        let p = ui.painter();
        p.rect_filled(o, 0.0, tk::NEN_TIEU_DE_KHUNG);
        p.hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, tk::VIEN_KHUNG));
        let ra = self.ds.get(self.chon).map(|(_, d)| d.clone());
        tieu_de_giua(ui, o, &format!("Đích: {}", ra.as_ref().map_or(String::new(), |d| d.display().to_string())), 40.0);
        let o_nut = Rect::from_min_size(egui::pos2(o.right() - 6.0 - 26.0, o.top() + 4.0), egui::vec2(26.0, 22.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(o_nut));
        if widget::nut_mui_ten(&mut c, widget::Huong::Phai, 26.0, 22.0, "Thu gọn khung Đích").on_hover_text("Thu gọn khung Đích").clicked() {
            self.doi_cfg(|c| c.khung_dich = false);
        }
    }

    /// Cột trang thu nhỏ 150 px: ảnh nhỏ + số trang; trang đang xem viền xanh, số nền xanh. Bấm → sang trang.
    fn cot_anh_nho(&mut self, ui: &mut egui::Ui, o: Rect) {
        let p = ui.painter();
        p.rect_filled(o, 0.0, tk::NEN_DAY);
        p.vline(o.right() - 0.5, o.y_range(), Stroke::new(1.0, tk::VIEN_KHUNG));
        let rong = tk::ANH_NHO.0;
        let mut bam = None;
        let o_td = Rect::from_min_size(o.min, egui::vec2(o.width() - 1.0, 28.0));
        p.hline(o_td.x_range(), o_td.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(0xEC, 0xEE, 0xF1)));
        p.text(egui::pos2(o_td.left() + 10.0, o_td.center().y), egui::Align2::LEFT_CENTER, "Trang thu nhỏ", font(tk::CHU_CO), tk::CHU);
        let o = Rect::from_min_max(egui::pos2(o.left(), o_td.bottom()), o.max);
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o.with_max_x(o.right() - 1.0)));
        con.set_clip_rect(o);
        egui::ScrollArea::vertical().id_salt(("xg_nho_cuon", self.the_he)).auto_shrink([false, false]).show(&mut con, |ui| {
            for (t, &(w, h)) in self.kho_trang.iter().enumerate() {
                let t = t as u32;
                ui.add_space(10.0);
                let cao = (rong * h / w.max(1.0)).min(tk::ANH_NHO.1 * 1.6);
                let (oo, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), cao + 22.0), Sense::click());
                let anh_o = Rect::from_min_size(egui::pos2(oo.center().x - rong / 2.0, oo.top()), egui::vec2(rong, cao));
                if ui.is_rect_visible(oo) {
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
                    let vien = if dang { Stroke::new(2.0, tk::XANH) } else if ph.hovered() { Stroke::new(1.0, tk::XANH_THANH) } else { Stroke::new(1.0, tk::VIEN) };
                    p.rect_stroke(anh_o, 0.0, vien, StrokeKind::Outside);
                    let so = egui::pos2(oo.center().x, anh_o.bottom() + 11.0);
                    if dang {
                        p.rect_filled(Rect::from_center_size(so, egui::vec2(24.0, 15.0)), 3.0, tk::XANH);
                    }
                    p.text(so, egui::Align2::CENTER_CENTER, format!("{}", t + 1), font(tk::CHU_NHO), if dang { Color32::WHITE } else { tk::CHU_PHU });
                }
                ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Trang thu nhỏ {}", t + 1)));
                if ph.clicked() {
                    bam = Some(t);
                }
            }
            ui.add_space(10.0);
        });
        if let Some(t) = bam {
            self.sang_trang(t);
        }
    }

    /// Chưa mở gì: 2 nút lớn "Mở file pdf" / "Mở thư mục" giữa khung Gốc (biểu tượng cao 50, cách 56, chữ xám);
    /// logo Sao Mai + "Trang chủ" góc dưới phải vùng xem → về Trang chủ.
    fn man_trong(&mut self, ui: &mut egui::Ui, o_goc: Rect, than: Rect) {
        let nut = [("lon_pdf", egui::vec2(40.0, 50.0), "Mở file pdf", "Mở 1 file PDF (Ctrl+O)"), ("lon_thu_muc", egui::vec2(60.0, 50.0), "Mở thư mục", "Mở cả thư mục PDF (Ctrl+Shift+O)")];
        let cot = egui::vec2(100.0, 50.0 + 10.0 + 18.0 + 28.0);
        let tong = 2.0 * cot.x + 56.0;
        let goc = egui::pos2(o_goc.center().x - tong / 2.0, o_goc.center().y - cot.y / 2.0);
        for (i, (bt, kich, ten, tip)) in nut.into_iter().enumerate() {
            let vung = Rect::from_min_size(goc + egui::vec2(i as f32 * (cot.x + 56.0), 0.0), cot);
            let ph = ui.interact(vung, ui.scope_id().with(("xg_mo", i)), Sense::click());
            let p = ui.painter();
            if ph.hovered() {
                p.rect_filled(vung, 6.0, Color32::from_black_alpha(13));
            }
            let o_bt = Rect::from_center_size(egui::pos2(vung.center().x, vung.top() + 14.0 + 25.0), kich);
            bieu_tuong::ve(ui, bt, o_bt);
            if i == 0 {
                // Chữ "PDF" của biểu tượng (SVG không vẽ chữ): giữa ngang, đáy chữ ở y 34 / 50
                p.text(o_bt.min + egui::vec2(20.0, 30.5), egui::Align2::CENTER_CENTER, "PDF", font(10.0), Color32::from_rgb(0x8A, 0x8F, 0x96));
            }
            let mau = if ph.hovered() { Color32::from_gray(0x55) } else { Color32::from_rgb(0x8A, 0x8F, 0x96) };
            p.text(egui::pos2(vung.center().x, o_bt.bottom() + 10.0 + 9.0), egui::Align2::CENTER_CENTER, ten, font(13.0), mau);
            ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, ten));
            if ph.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if i == 0 {
                    self.chon_file_pdf();
                } else {
                    self.chon_thu_muc();
                }
            }
        }
        // Logo góc dưới phải + "Trang chủ"
        // Cùng cỡ logo góc của các chức năng khác
        let kich = crate::logo_sao_mai::kich(64.0);
        let o_nut = Rect::from_min_size(egui::pos2(than.right() - 18.0 - kich.x - 12.0, than.bottom() - 14.0 - kich.y - 12.0 - 16.0), kich + egui::vec2(12.0, 12.0 + 16.0));
        if o_nut.top() > goc.y + cot.y + 20.0 {
            let ph = ui.interact(o_nut, ui.scope_id().with("xg_logo"), Sense::click());
            if ph.hovered() {
                ui.painter().rect_filled(o_nut, 8.0, Color32::from_white_alpha(178));
            }
            crate::logo_sao_mai::ve_tinh(ui.painter(), Rect::from_min_size(o_nut.min + egui::vec2(6.0, 6.0), kich));
            ui.painter().text(egui::pos2(o_nut.center().x, o_nut.bottom() - 6.0 - 7.0), egui::Align2::CENTER_CENTER, "Trang chủ", font(11.0), tk::CHU_PHU);
            ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Trang chủ"));
            if ph.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Về Trang chủ DEMA4").clicked() {
                self.ve_trang_chu = true;
            }
        }
    }
}

/// Chữ tiêu đề khung cân giữa `o`, chừa ít nhất 70 px (hoặc `chua` px — chỗ nút) mỗi bên, cắt "…" khi dài; rê chuột
/// hiện đủ.
fn tieu_de_giua(ui: &mut egui::Ui, o: Rect, chu: &str, chua: f32) {
    let o_chu = o.shrink2(egui::vec2(chua.max(70.0) + 6.0, 0.0));
    if o_chu.width() < 20.0 {
        return;
    }
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o_chu).layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight)));
    con.add(egui::Label::new(egui::RichText::new(chu).font(font(tk::CHU_CO)).color(tk::CHU)).truncate()).on_hover_text(chu);
}
