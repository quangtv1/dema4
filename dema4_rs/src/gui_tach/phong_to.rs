//! Hộp phóng to 1 trang (bấm đúp / Enter ở tab Hồ sơ và tab Văn bản): ‹ › / ←→ theo thứ tự đang xem,
//! − / + 50–250 %, Esc / ✕ / bấm nền để đóng. Ảnh render nền theo cỡ hiển thị, có ảnh nhỏ tạm trong lúc chờ.

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText};

use super::anh_nho::{self, bac_cho, KhoaAnh, YeuCau};
use super::bang_trang::ten_nhan;
use super::bieu_tuong::{self, BieuTuong};
use super::ManTach;
use crate::gui::{ChuDam, CHU_PHU};
use crate::tach_vb::nhan::Nhan;

pub const TY_LE: [u16; 9] = [50, 75, 100, 125, 150, 175, 200, 225, 250];

#[derive(Debug, Clone, PartialEq)]
pub struct PhongTo {
    /// Thứ tự trang đang xem lúc mở (lọc + sắp tab Hồ sơ, hoặc trang của văn bản).
    pub ds: Vec<u32>,
    pub i: usize,
    /// Chỉ số trong `TY_LE`.
    pub nac: usize,
}

impl PhongTo {
    pub fn moi(ds: Vec<u32>, k: u32) -> Self {
        let i = ds.iter().position(|&x| x == k).unwrap_or(0);
        Self { ds, i, nac: 2 }
    }

    pub fn trang(&self) -> Option<u32> {
        self.ds.get(self.i).copied()
    }

    /// ‹ / ›: vô hiệu ở đầu / cuối.
    pub fn di(&mut self, toi: bool) {
        if toi {
            self.i = (self.i + 1).min(self.ds.len().saturating_sub(1));
        } else {
            self.i = self.i.saturating_sub(1);
        }
    }

    pub fn co_truoc(&self) -> bool {
        self.i > 0
    }

    pub fn co_sau(&self) -> bool {
        self.i + 1 < self.ds.len()
    }
}

impl ManTach {
    /// Vẽ hộp phóng to nếu đang mở.
    pub(super) fn hop_phong_to(&mut self, ctx: &egui::Context) {
        let Some(pt) = &mut self.phong_to else { return };
        let (Some(b), Some(k)) = (&self.bang, pt.trang()) else {
            self.phong_to = None;
            return;
        };
        let t = b.trang[k as usize];
        // Phím trong hộp
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) {
                pt.di(false);
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                pt.di(true);
            }
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                pt.nac = (pt.nac + 1).min(TY_LE.len() - 1);
            }
            if i.key_pressed(egui::Key::Minus) {
                pt.nac = pt.nac.saturating_sub(1);
            }
        });
        let man = ctx.content_rect();
        let rong_hop = (man.width() * 0.9).max(400.0);
        let cao_hop = (man.height() * 0.9).max(300.0);
        let mut dong = false;
        let khung = egui::Frame::popup(&ctx.global_style()).inner_margin(Margin::same(8)).fill(Color32::from_gray(245));
        let ph = egui::Modal::new(egui::Id::unique("phong_to")).frame(khung).show(ctx, |ui| {
            ui.set_width(rong_hop);
            ui.horizontal(|ui| {
                if bieu_tuong::nut(ui, BieuTuong::TraiChevron, "Trang trước (←)", pt.co_truoc()).clicked() {
                    pt.di(false);
                }
                let nhan = match t.nhan() {
                    Nhan::NoiDung if t.nghi_trang() => "Nghi trắng".to_string(),
                    Nhan::NoiDung => "Nội dung".to_string(),
                    n => ten_nhan(n).to_string(),
                };
                ui.label(RichText::new(format!("{}.{}", self.ten_hien[t.ho_so as usize], t.so)).dam());
                ui.label(RichText::new(format!("· {nhan} ({}/{})", pt.i + 1, pt.ds.len())).color(CHU_PHU));
                if bieu_tuong::nut(ui, BieuTuong::PhaiChevron, "Trang sau (→)", pt.co_sau()).clicked() {
                    pt.di(true);
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if bieu_tuong::nut(ui, BieuTuong::Dong, "Đóng (Esc)", true).clicked() {
                        dong = true;
                    }
                    ui.add_space(12.0);
                    if ui.add_enabled(pt.nac + 1 < TY_LE.len(), egui::Button::new("+")).on_hover_text("Phóng to (+)").clicked() {
                        pt.nac += 1;
                    }
                    ui.label(format!("{}%", TY_LE[pt.nac]));
                    if ui.add_enabled(pt.nac > 0, egui::Button::new("−")).on_hover_text("Thu nhỏ (−)").clicked() {
                        pt.nac -= 1;
                    }
                });
            });
            // 100 % = cả trang vừa chiều cao hộp
            let (w, h) = if t.sua.goc % 180 == 90 { (t.cao, t.rong) } else { (t.rong, t.cao) };
            let cao_100 = cao_hop - 50.0;
            let k_ty = TY_LE[pt.nac] as f32 / 100.0;
            let kich = egui::vec2(cao_100 * w / h.max(1.0) * k_ty, cao_100 * k_ty);
            let px = (if t.sua.goc % 180 == 90 { kich.y } else { kich.x }) * ctx.pixels_per_point();
            let bac = bac_cho(px);
            let khoa = KhoaAnh { trang: k, bac };
            let tex = self.anh.as_mut().and_then(|a| {
                if !a.co(&khoa) {
                    self.yeu_cau_anh.push(YeuCau { khoa, file: t.ho_so as usize, so: t.so as usize });
                }
                a.lay(k, bac)
            });
            egui::ScrollArea::both().id_salt("phong_to_cuon").max_height(cao_100).min_scrolled_height(cao_100).auto_shrink([false, false]).show(ui, |ui| {
                let rong_vung = ui.available_width().max(kich.x);
                let (vung, _) = ui.allocate_exact_size(egui::vec2(rong_vung, kich.y.max(cao_100)), egui::Sense::hover());
                let hop = Rect::from_center_size(vung.center(), kich);
                match &tex {
                    Some(tex) => {
                        anh_nho::ve_xoay(ui.painter(), hop, tex, t.sua.goc);
                    }
                    None => {
                        ui.painter().rect_filled(hop, 0.0, Color32::from_gray(225));
                        ui.painter().text(hop.center(), egui::Align2::CENTER_CENTER, "Đang tải…", egui::FontId::proportional(13.0), CHU_PHU);
                    }
                }
            });
        });
        if dong || ph.should_close() {
            self.phong_to = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn di_theo_thu_tu_dang_xem_vo_hieu_o_dau_cuoi() {
        let mut p = PhongTo::moi(vec![7, 3, 9], 3);
        assert_eq!((p.i, p.trang()), (1, Some(3)));
        p.di(true);
        assert_eq!(p.trang(), Some(9));
        assert!(!p.co_sau());
        p.di(true);
        assert_eq!(p.trang(), Some(9), "cuối: đứng yên");
        p.di(false);
        p.di(false);
        p.di(false);
        assert_eq!(p.trang(), Some(7));
        assert!(!p.co_truoc());
        assert_eq!(PhongTo::moi(vec![1, 2], 99).i, 0, "trang không còn trong danh sách → đầu");
    }
}
