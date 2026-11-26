//! Khung trang Cấu hình (như prototype): toàn cửa sổ (`ve_trang`) hoặc cửa sổ thu nhỏ 1120 × 720 nổi trên màn chức
//! năng (`ve_cua_so`); sidebar chức năng bên trái (240 px, thu nhỏ 56 px); hàng tiêu đề có Khôi phục mặc định · nhãn
//! chưa lưu · Huỷ thay đổi · Lưu · nút cửa sổ; thân cuộn trong cột cố định `RONG_COT`.
//!
//! Phím: Esc = quay lại (còn thay đổi → hỏi), Ctrl+S = Lưu, Ctrl+M = đổi kiểu cửa sổ, Ctrl+B = thu / mở sidebar.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke};

use super::bieu_tuong;
use crate::gui::ChuDam;
use super::{CHU, CHU_PHU, NEN_TRANG, RONG_COT, XANH, XANH_CHU};
use crate::cau_hinh::CauHinh;
use crate::gui_tach::hop_cau_hinh::{HopCauHinh, TabCauHinh};
use crate::gui_tach::khung_hop as kh;

const RONG_SIDEBAR: f32 = 240.0;
const RONG_SIDEBAR_THU: f32 = 56.0;
const NEN_SIDEBAR: Color32 = Color32::from_rgb(0xF7, 0xF8, 0xFA);
const VIEN: Color32 = Color32::from_rgb(0xDC, 0xDF, 0xE3);
const KE: Color32 = Color32::from_rgb(0xE3, 0xE5, 0xE8);
const NEN_CHON: Color32 = Color32::from_rgb(0xDC, 0xEB, 0xFA);
const NEN_HOVER: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xF1);
const CAM: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
const CUA_SO: egui::Vec2 = egui::vec2(1120.0, 720.0);
const THOI_TOAST: Duration = Duration::from_millis(2400);

/// Kết quả 1 khung hình của trang.
pub enum KetQuaTrang {
    Mo,
    /// Rời trang (không lưu gì thêm).
    Dong,
    /// Lưu bản nháp, ở lại trang.
    Luu(CauHinh),
    /// Lưu rồi rời trang ("Lưu và quay lại").
    LuuDong(CauHinh),
}

impl HopCauHinh {
    /// Toàn cửa sổ: sidebar + nội dung chiếm vùng `ui` (màn chức năng không vẽ).
    pub fn ve_trang(&mut self, ui: &mut egui::Ui) -> KetQuaTrang {
        let mut kq = KetQuaTrang::Mo;
        let ctx = ui.ctx().clone();
        self.phim(&ctx, &mut kq);
        self.sidebar_panel(ui, &mut kq);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN_TRANG)).show(ui, |ui| self.noi_dung(ui, false, &mut kq));
        self.hop_roi(&ctx, &mut kq);
        self.ve_toast(&ctx);
        kq
    }

    /// Cửa sổ thu nhỏ nổi trên màn chức năng (gọi **trước** khi vẽ màn: phím đã dùng ở đây bị lấy khỏi hàng sự kiện để
    /// màn phía sau không nhận).
    pub fn ve_cua_so(&mut self, ctx: &egui::Context) -> KetQuaTrang {
        let mut kq = KetQuaTrang::Mo;
        self.phim(ctx, &mut kq);
        let man = ctx.content_rect();
        let co = egui::vec2(CUA_SO.x.min(man.width() - 40.0), CUA_SO.y.min(man.height() - 40.0));
        let ph = egui::Modal::new(egui::Id::unique("ch_cua_so"))
            .frame(egui::Frame::new().fill(Color32::WHITE).corner_radius(8).shadow(egui::Shadow { offset: [0, 12], blur: 40, spread: 0, color: Color32::from_black_alpha(64) }))
            .backdrop_color(Color32::from_rgba_unmultiplied(17, 24, 39, 82))
            .show(ctx, |ui| {
                ui.set_min_size(co);
                ui.set_max_size(co);
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                // Thanh tiêu đề: ⚙ Cấu hình … ✕
                let (o, _) = ui.allocate_exact_size(egui::vec2(co.x, 36.0), Sense::hover());
                ui.painter().rect_filled(o, egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, NEN_SIDEBAR);
                ui.painter().hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, VIEN));
                bieu_tuong::ve(ui, "banh_rang", Rect::from_center_size(egui::pos2(o.left() + 22.0, o.center().y), egui::vec2(14.0, 14.0)), false);
                ui.painter().text(egui::pos2(o.left() + 36.0, o.center().y), egui::Align2::LEFT_CENTER, "Cấu hình", egui::FontId::proportional(13.0), CHU);
                let o_dong = Rect::from_center_size(egui::pos2(o.right() - 22.0, o.center().y), egui::vec2(28.0, 26.0));
                if nut_bieu_tuong(ui, o_dong, "dong", "Đóng (Esc)").clicked() {
                    self.yeu_cau_roi(&mut kq);
                }
                let than = Rect::from_min_max(egui::pos2(o.left(), o.bottom()), egui::pos2(o.right(), o.top() + co.y));
                let mut con = ui.new_child(egui::UiBuilder::new().max_rect(than));
                con.set_clip_rect(than);
                self.sidebar_panel(&mut con, &mut kq);
                egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN_TRANG)).show(&mut con, |ui| self.noi_dung(ui, true, &mut kq));
                ui.allocate_rect(than, Sense::hover());
            });
        if ph.should_close() && !self.hoi_roi {
            self.yeu_cau_roi(&mut kq);
        }
        self.hop_roi(ctx, &mut kq);
        self.ve_toast(ctx);
        // Màn phía sau không nhận phím khi Cấu hình đang mở (vd phím B gán nhãn ở Tách)
        ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))));
        kq
    }

    fn phim(&mut self, ctx: &egui::Context, kq: &mut KetQuaTrang) {
        if self.hoi_roi {
            return;
        }
        let dang_go = ctx.egui_wants_keyboard_input();
        let (esc, luu, cua_so, thu) = ctx.input(|i| {
            let c = i.modifiers.command;
            (i.key_pressed(egui::Key::Escape), c && i.key_pressed(egui::Key::S), c && i.key_pressed(egui::Key::M), c && i.key_pressed(egui::Key::B))
        });
        if luu {
            self.luu(kq, false);
        }
        if dang_go {
            return;
        }
        if esc {
            self.yeu_cau_roi(kq);
        }
        if cua_so {
            self.cua_so_nho = !self.cua_so_nho;
            self.doi_giao_dien = true;
        }
        if thu {
            self.thu_ds = !self.thu_ds;
            self.doi_giao_dien = true;
        }
    }

    /// Quay lại / Esc / ✕: còn thay đổi → hỏi, không thì rời.
    fn yeu_cau_roi(&mut self, kq: &mut KetQuaTrang) {
        if self.ds_doi().is_empty() {
            *kq = KetQuaTrang::Dong;
        } else {
            self.hoi_roi = true;
        }
    }

    fn luu(&mut self, kq: &mut KetQuaTrang, dong: bool) {
        let n = self.ds_doi().len();
        if n == 0 && !dong {
            return;
        }
        if !self.loi().is_empty() {
            self.bao("Sửa ô đang báo đỏ trước khi lưu");
            return;
        }
        if n > 0 {
            self.bao(format!("Đã lưu {n} chức năng"));
        }
        *kq = if dong { KetQuaTrang::LuuDong(self.nhap.clone()) } else { KetQuaTrang::Luu(self.nhap.clone()) };
    }

    pub(crate) fn bao(&mut self, chu: impl Into<String>) {
        self.toast = Some((chu.into(), Instant::now()));
    }

    fn sidebar_panel(&mut self, ui: &mut egui::Ui, kq: &mut KetQuaTrang) {
        let rong = if self.thu_ds { RONG_SIDEBAR_THU } else { RONG_SIDEBAR };
        egui::Panel::left("ch_sidebar").resizable(false).exact_size(rong).frame(egui::Frame::new().fill(NEN_SIDEBAR).stroke(Stroke::new(1.0, VIEN))).show(ui, |ui| self.sidebar(ui, kq));
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, kq: &mut KetQuaTrang) {
        let thu = self.thu_ds;
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let w = ui.available_width();
        // Đầu: Quay lại + nút thu nhỏ (thu nhỏ: xếp dọc)
        let (o_lai, o_thu) = if thu {
            let (o, _) = ui.allocate_exact_size(egui::vec2(w, 76.0), Sense::hover());
            (Rect::from_min_size(o.min, egui::vec2(w, 40.0)), Rect::from_min_size(o.min + egui::vec2(0.0, 40.0), egui::vec2(w, 36.0)))
        } else {
            let (o, _) = ui.allocate_exact_size(egui::vec2(w, 44.0), Sense::hover());
            (Rect::from_min_max(o.min, egui::pos2(o.right() - 40.0, o.bottom())), Rect::from_min_max(egui::pos2(o.right() - 40.0, o.top()), o.max))
        };
        let p = ui.painter().clone();
        p.hline(ui.max_rect().x_range(), o_thu.bottom().max(o_lai.bottom()) - 0.5, Stroke::new(1.0, KE));
        if thu {
            p.hline(o_thu.x_range(), o_thu.top(), Stroke::new(1.0, KE));
        }
        let ten_lai = if self.tu.is_empty() { "Quay lại".to_string() } else { format!("Quay lại {}", self.tu) };
        let ph = ui.interact(o_lai, ui.scope_id().with("ch_quay_lai"), Sense::click());
        if ph.hovered() {
            p.rect_filled(o_lai, 0.0, Color32::from_rgb(0xEE, 0xF2, 0xF7));
        }
        if thu {
            bieu_tuong::ve(ui, "quay_lai", Rect::from_center_size(o_lai.center(), egui::vec2(16.0, 16.0)), true);
        } else {
            bieu_tuong::ve(ui, "quay_lai", Rect::from_center_size(egui::pos2(o_lai.left() + 22.0, o_lai.center().y), egui::vec2(16.0, 16.0)), true);
            let mut job = egui::text::LayoutJob::default();
            job.append("Quay lại ", 0.0, egui::TextFormat::simple(egui::FontId::proportional(12.5), XANH_CHU));
            job.append(self.tu, 0.0, egui::TextFormat::simple(egui::FontId::new(12.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), XANH_CHU));
            let g = p.layout_job(job);
            p.galley(egui::pos2(o_lai.left() + 38.0, o_lai.center().y - g.size().y / 2.0), g, XANH_CHU);
        }
        ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Quay lại"));
        if ph.on_hover_text(format!("{ten_lai} (Esc)")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.yeu_cau_roi(kq);
        }
        let (bt, tip) = if thu { ("sidebar_thu", "Mở rộng danh sách (Ctrl+B)") } else { ("sidebar", "Thu nhỏ danh sách (Ctrl+B)") };
        if nut_bieu_tuong(ui, Rect::from_center_size(o_thu.center(), egui::vec2(28.0, 26.0)), bt, tip).clicked() {
            self.thu_ds = !self.thu_ds;
            self.doi_giao_dien = true;
        }
        // Nhóm + mục
        let nhom = |ui: &mut egui::Ui, ten: &str| {
            if thu {
                ui.add_space(10.0);
                let (o, _) = ui.allocate_exact_size(egui::vec2(w, 1.0), Sense::hover());
                ui.painter().hline(o.x_range().shrink(12.0), o.top(), Stroke::new(1.0, KE));
                ui.add_space(10.0);
            } else {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new(ten).size(11.0).color(CHU_PHU).dam());
                });
                ui.add_space(8.0);
            }
        };
        nhom(ui, "CHỨC NĂNG");
        for t in TabCauHinh::CHUC_NANG {
            self.muc(ui, t);
        }
        nhom(ui, "ỨNG DỤNG");
        self.muc(ui, TabCauHinh::DuLieu);
        if !thu {
            let o = ui.available_rect_before_wrap();
            let chu = "Lưu ở file .dema4_saomai.cfg trong thư mục dữ liệu. Tuỳ chọn theo thư mục của Xóa ghim lưu cùng vùng của thư mục đang mở.";
            let g = p.layout(chu.to_string(), egui::FontId::proportional(11.5), CHU_PHU, w - 32.0);
            let y = o.bottom() - g.size().y - 12.0;
            if y > ui.cursor().top() + 20.0 {
                p.hline(o.x_range(), y - 12.0, Stroke::new(1.0, KE));
                p.galley(egui::pos2(o.left() + 16.0, y), g, CHU_PHU);
            }
        }
    }

    /// 1 mục sidebar: cao 40, lề 8, bo 5; chọn = nền xanh nhạt + vạch trái 3 px; chấm cam khi còn thay đổi.
    fn muc(&mut self, ui: &mut egui::Ui, t: TabCauHinh) {
        let thu = self.thu_ds;
        let w = ui.available_width();
        let (hang, _) = ui.allocate_exact_size(egui::vec2(w, 42.0), Sense::hover());
        let o = Rect::from_min_max(egui::pos2(hang.left() + 8.0, hang.top() + 1.0), egui::pos2(hang.right() - 8.0, hang.bottom() - 1.0));
        let ph = ui.interact(o, ui.scope_id().with(("ch_muc", t as u8)), Sense::click());
        let chon = self.tab == t;
        let p = ui.painter();
        if chon {
            p.rect_filled(o, 5.0, NEN_CHON);
            p.rect_filled(Rect::from_min_max(egui::pos2(hang.left(), o.top() + 8.0), egui::pos2(hang.left() + 3.0, o.bottom() - 8.0)), 2.0, XANH);
        } else if ph.hovered() {
            p.rect_filled(o, 5.0, NEN_HOVER);
        }
        let x_bt = if thu { o.center().x } else { o.left() + 18.0 };
        bieu_tuong::ve(ui, t.bieu_tuong(), Rect::from_center_size(egui::pos2(x_bt, o.center().y), egui::vec2(16.0, 16.0)), chon);
        if !thu {
            let f = if chon { egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(13.0) };
            p.text(egui::pos2(o.left() + 36.0, o.center().y), egui::Align2::LEFT_CENTER, t.ten(), f, if chon { XANH_CHU } else { CHU });
        }
        if t != TabCauHinh::DuLieu && self.doi(t) {
            let c = if thu { egui::pos2(o.right() - 8.0, o.top() + 8.0) } else { egui::pos2(o.right() - 12.0, o.center().y) };
            p.circle_filled(c, 3.5, CAM);
        }
        ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, chon, t.ten()));
        let ph = if thu { ph.on_hover_text(t.ten()) } else { ph };
        if ph.clicked() {
            self.tab = t;
        }
    }

    /// Tiêu đề + thân cuộn trong cột cố định (cửa sổ thu nhỏ: hết bề ngang trừ lề 20).
    fn noi_dung(&mut self, ui: &mut egui::Ui, cua_so: bool, kq: &mut KetQuaTrang) {
        let vung = ui.max_rect();
        let rong = if cua_so { vung.width() - 40.0 } else { RONG_COT.min(vung.width() - 40.0) };
        let x0 = if cua_so { vung.left() + 20.0 } else { vung.center().x - rong / 2.0 };
        let cot = Rect::from_min_max(egui::pos2(x0, vung.top()), egui::pos2(x0 + rong, vung.bottom()));
        // Tiêu đề
        let o_td = Rect::from_min_size(cot.min, egui::vec2(rong, 78.0));
        let mut td = ui.new_child(egui::UiBuilder::new().max_rect(o_td.shrink2(egui::vec2(4.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Max)));
        self.tieu_de(&mut td, kq);
        ui.painter().hline(cot.x_range(), o_td.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(0xE3, 0xE5, 0xE8)));
        // Thân
        let o_than = Rect::from_min_max(egui::pos2(cot.left(), o_td.bottom()), egui::pos2(cot.right() + if cua_so { 14.0 } else { 0.0 }, cot.bottom()));
        let mut than = ui.new_child(egui::UiBuilder::new().max_rect(o_than).layout(egui::Layout::top_down(egui::Align::Min)));
        egui::ScrollArea::vertical().id_salt(("ch_than", self.tab as u8)).auto_shrink([false, false]).show(&mut than, |ui| {
            ui.set_width(rong - 8.0);
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.set_width(rong - 16.0);
                    self.ve_than(ui);
                });
            });
            ui.add_space(24.0);
        });
        ui.allocate_rect(vung, Sense::hover());
    }

    fn tieu_de(&mut self, ui: &mut egui::Ui, kq: &mut KetQuaTrang) {
        let t = self.tab;
        ui.add_space(0.0);
        ui.vertical(|ui| {
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                let (o, _) = ui.allocate_exact_size(egui::vec2(18.0, 22.0), Sense::hover());
                bieu_tuong::ve(ui, t.bieu_tuong(), Rect::from_center_size(o.center(), egui::vec2(16.0, 16.0)), true);
                ui.add_space(8.0);
                ui.label(RichText::new(t.ten()).size(18.0).color(CHU).dam());
            });
            ui.add_space(3.0);
            ui.label(RichText::new(t.mo_ta()).size(12.0).color(CHU_PHU));
            ui.add_space(14.0);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(0.0);
            ui.spacing_mut().item_spacing.x = 8.0;
            let (bt, tip) = if self.cua_so_nho { ("toan_cua_so", "Toàn cửa sổ (Ctrl+M)") } else { ("cua_so_nho", "Cửa sổ thu nhỏ (Ctrl+M)") };
            let (o, _) = ui.allocate_exact_size(egui::vec2(28.0, 26.0), Sense::hover());
            if nut_bieu_tuong(ui, o, bt, tip).clicked() {
                self.cua_so_nho = !self.cua_so_nho;
                self.doi_giao_dien = true;
            }
            if t == TabCauHinh::DuLieu && self.ds_doi().is_empty() {
                return;
            }
            let ds = self.ds_doi();
            let co = !ds.is_empty();
            let nut_luu = egui::Button::new(RichText::new("Lưu").color(Color32::WHITE).dam()).fill(if co && self.loi().is_empty() { XANH } else { Color32::from_rgb(0x9D, 0xB7, 0xE6) }).min_size(egui::vec2(48.0, 26.0));
            let sai = !self.loi().is_empty();
            let r = ui.add_enabled(co && !sai, nut_luu).on_hover_text("Lưu thay đổi của mọi chức năng (Ctrl+S)").on_disabled_hover_text(if sai { "Sửa ô đang báo đỏ trước" } else { "Chưa có thay đổi" });
            if r.clicked() {
                self.luu(kq, false);
            }
            if ui.add_enabled(co, egui::Button::new("Huỷ thay đổi").min_size(egui::vec2(0.0, 26.0))).on_hover_text("Bỏ mọi thay đổi chưa lưu").clicked() {
                self.huy_thay_doi();
            }
            // Nhãn trạng thái
            let (chu, nen, mau, vien) = if co {
                (format!("{} chức năng chưa lưu", ds.len()), Color32::from_rgb(0xFF, 0xF7, 0xED), Color32::from_rgb(0xB4, 0x53, 0x09), Color32::from_rgb(0xFE, 0xD7, 0xAA))
            } else {
                ("Đã lưu".to_string(), Color32::from_rgb(0xF0, 0xFD, 0xF4), Color32::from_rgb(0x15, 0x80, 0x3D), Color32::from_rgb(0xBB, 0xF7, 0xD0))
            };
            // Dấu chấm (chưa lưu) / dấu tích (đã lưu) vẽ bằng nét — phông chữ app không có ● ✓
            let g = ui.painter().layout_no_wrap(chu.clone(), egui::FontId::proportional(11.5), mau);
            let (o, ph) = ui.allocate_exact_size(g.size() + egui::vec2(32.0, 6.0), Sense::hover());
            let p = ui.painter();
            p.rect(o, 10.0, nen, Stroke::new(1.0, vien), egui::StrokeKind::Inside);
            let c = egui::pos2(o.left() + 14.0, o.center().y);
            if co {
                p.circle_filled(c, 3.5, mau);
            } else {
                p.line_segment([c + egui::vec2(-4.0, 0.0), c + egui::vec2(-1.2, 2.8)], Stroke::new(1.5, mau));
                p.line_segment([c + egui::vec2(-1.2, 2.8), c + egui::vec2(4.0, -3.0)], Stroke::new(1.5, mau));
            }
            p.galley(o.min + egui::vec2(23.0, 3.0), g, mau);
            ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, chu.clone()));
            if co {
                ph.on_hover_text(ds.iter().map(|t| t.ten()).collect::<Vec<_>>().join(", "));
            }
            if t != TabCauHinh::DuLieu {
                let r = ui.add(egui::Button::new(RichText::new("Khôi phục mặc định").size(12.0).color(XANH_CHU)).frame(false));
                if r.on_hover_text(format!("Chỉ đưa {} về mặc định", t.ten())).clicked() {
                    self.mac_dinh_tab();
                    self.bao(format!("{}: đã về mặc định — bấm Lưu để giữ", t.ten()));
                }
            }
        });
    }

    /// Hộp "Lưu thay đổi trước khi quay lại?".
    fn hop_roi(&mut self, ctx: &egui::Context, kq: &mut KetQuaTrang) {
        if !self.hoi_roi {
            return;
        }
        let ds = self.ds_doi().iter().map(|t| t.ten()).collect::<Vec<_>>().join(", ");
        let co_loi = !self.loi().is_empty();
        let (mut khong, mut o_lai, mut luu) = (false, false, false);
        let ph = kh::hop(ctx, "ch_hoi_roi", 440.0, "Lưu thay đổi trước khi quay lại?", |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 18, right: 18, top: 12, bottom: 14 }).show(ui, |ui| {
                let mut job = egui::text::LayoutJob::default();
                job.append("Có thay đổi chưa lưu ở: ", 0.0, egui::TextFormat::simple(egui::FontId::proportional(12.5), CHU));
                job.append(&ds, 0.0, egui::TextFormat::simple(egui::FontId::new(12.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU));
                job.append(".", 0.0, egui::TextFormat::simple(egui::FontId::proportional(12.5), CHU));
                ui.label(job);
                if co_loi {
                    ui.label(RichText::new("Có ô đang báo đỏ — sửa trước khi lưu, hoặc Không lưu.").size(11.5).color(crate::gui::DO));
                }
            });
            kh::chan(ui, |ui| {
                khong = ui.add(kh::nut_chan("Không lưu")).clicked();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    luu = ui.add_enabled(!co_loi, kh::nut_chan_mau("Lưu và quay lại", XANH)).clicked();
                    o_lai = ui.add(kh::nut_chan("Ở lại")).clicked();
                });
            });
        });
        if khong {
            self.hoi_roi = false;
            self.huy_thay_doi();
            *kq = KetQuaTrang::Dong;
        } else if luu {
            self.hoi_roi = false;
            self.luu(kq, true);
        } else if o_lai || ph.should_close() {
            self.hoi_roi = false;
        }
    }

    fn ve_toast(&mut self, ctx: &egui::Context) {
        let Some((chu, t)) = &self.toast else { return };
        if t.elapsed() > THOI_TOAST {
            self.toast = None;
            return;
        }
        let man = ctx.content_rect();
        egui::Area::new(egui::Id::unique("ch_toast")).order(egui::Order::Tooltip).pivot(egui::Align2::CENTER_BOTTOM).fixed_pos(egui::pos2(man.center().x, man.bottom() - 40.0)).interactable(false).show(ctx, |ui| {
            egui::Frame::new().fill(Color32::from_rgb(0x1F, 0x29, 0x37)).corner_radius(6).inner_margin(egui::Margin::symmetric(14, 7)).show(ui, |ui| {
                ui.label(RichText::new(chu.as_str()).size(12.5).color(Color32::WHITE));
            });
        });
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

/// Nút biểu tượng không viền 28 × 26 (nền xám khi rê chuột), tên trợ năng = `tip` bỏ phần phím tắt.
fn nut_bieu_tuong(ui: &mut egui::Ui, o: Rect, bt: &str, tip: &str) -> egui::Response {
    let ph = ui.interact(o, ui.scope_id().with(("ch_nut", bt)), Sense::click());
    if ph.hovered() {
        ui.painter().rect_filled(o, 5.0, Color32::from_rgb(0xE8, 0xEA, 0xED));
    }
    bieu_tuong::ve(ui, bt, Rect::from_center_size(o.center(), egui::vec2(16.0, 16.0)), false);
    let ten = tip.split(" (").next().unwrap_or(tip).to_string();
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, ten.clone()));
    ph.on_hover_text(tip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Cửa sổ thu nhỏ vẽ trước màn chức năng và lấy hết phím: màn vẽ sau (vd Tách — phím B gán nhãn) không thấy phím.
    #[test]
    fn cua_so_nho_lay_het_phim_truoc_man_phia_sau() {
        let thay_b = Rc::new(Cell::new(false));
        let t = thay_b.clone();
        let mut hop = HopCauHinh::moi(&CauHinh::default(), TabCauHinh::Dem);
        hop.cua_so_nho = true;
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            let _ = hop.ve_cua_so(ui.ctx());
            if ui.ctx().input(|i| i.key_pressed(egui::Key::B)) {
                t.set(true);
            }
        });
        h.run();
        h.key_press(egui::Key::B);
        h.run();
        assert!(!thay_b.get());
    }
}
