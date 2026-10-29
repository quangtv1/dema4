//! Hộp phóng to 1 trang (bấm đúp / Enter ở tab Hồ sơ và tab Văn bản): ‹ › / ←→ theo thứ tự đang xem,
//! − / + 50–250 %, Esc / ✕ / bấm nền để đóng. Ảnh render nền theo cỡ hiển thị, có ảnh nhỏ tạm trong lúc chờ.

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Stroke, StrokeKind};

use super::anh_nho::{self, bac_cho, KhoaAnh, YeuCau};
use super::bieu_tuong::{self, BieuTuong};
use super::{nhom_nut, ngan_nho, nut_nhom, ve_net_dut, Loc, ManTach, NoiDungNut, Tab, Viec, CAM, MAU_S};
use crate::cau_hinh::CauHinh;
use crate::gui::{ChuDam, CHU_PHU, VIEN_NHAT, XANH_COBALT};
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
    pub(super) fn hop_phong_to(&mut self, ctx: &egui::Context, cfg: &mut CauHinh) {
        let la_vb = self.tab == Tab::VanBan;
        let duyet_nghi = !la_vb && self.loc == Loc::NghiTrang;
        let mut viec: Vec<Viec> = Vec::new();
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
            // Gán nhãn / xoay / xoá ngay trong hộp, cùng phím với lưới
            if !i.modifiers.command && !la_vb {
                let gan = |n: Nhan| if duyet_nghi && n != Nhan::Split { Viec::DuyetNghiTrang(n) } else { Viec::Gan(n) };
                for (phim, v) in [(egui::Key::S, Viec::Gan(Nhan::Split)), (egui::Key::I, gan(Nhan::Index)), (egui::Key::B, gan(Nhan::Trang)), (egui::Key::C, gan(Nhan::NoiDung))] {
                    if i.key_pressed(phim) {
                        viec.push(v);
                    }
                }
            }
            if !i.modifiers.command {
                if i.key_pressed(egui::Key::R) {
                    viec.push(Viec::Xoay(if i.modifiers.shift { 180 } else { 90 }));
                }
                if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    viec.push(if la_vb { Viec::BoKhoiVb } else { Viec::Xoa });
                }
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
                let hang = ui.max_rect();
                ui.label(RichText::new(format!("{}.{}", self.ten_hien[t.ho_so as usize], t.so)).dam());
                if bieu_tuong::nut(ui, BieuTuong::PhaiChevron, "Trang sau (→)", pt.co_sau()).clicked() {
                    pt.di(true);
                }
                // Giữa hàng: trạng thái trang (đậm) + vị trí, vd "Nội dung (12/215)"
                let nhan = match t.nhan() {
                    Nhan::NoiDung if t.nghi_trang() => "Nghi trắng",
                    Nhan::NoiDung if t.nghi_xoay().is_some() => "Nghi xoay",
                    Nhan::NoiDung => "Nội dung",
                    Nhan::Index => "Index (barcode)",
                    Nhan::Trang => "Trang trắng",
                    Nhan::Split => "Điểm tách",
                };
                let mut job = egui::text::LayoutJob::default();
                job.append(nhan, 0.0, super::chu_dong(crate::gui::CHU, true));
                job.append(&format!(" ({}/{})", pt.i + 1, pt.ds.len()), 0.0, super::chu_dong(CHU_PHU, false));
                let g = ui.painter().layout_job(job);
                ui.painter().galley(egui::pos2(hang.center().x - g.size().x / 2.0, ui.min_rect().center().y - g.size().y / 2.0), g, crate::gui::CHU);
                // Nhóm gắn nhãn + nhóm xoay / xoá như thanh công cụ lưới, áp cho trang đang xem
                ui.add_space(10.0);
                let nhan_t = t.nhan();
                let gan = |n: Nhan| if duyet_nghi && n != Nhan::Split { Viec::DuyetNghiTrang(n) } else { Viec::Gan(n) };
                // S I B chỉ ở tab Hồ sơ (tab Văn bản chỉ xoay / bỏ khỏi văn bản)
                if !la_vb {
                    nhom_nut(ui, |ui| {
                        if nut_nhom(ui, NoiDungNut::Chu("S", Some(MAU_S)), (nhan_t == Nhan::Split, false), matches!(nhan_t, Nhan::NoiDung | Nhan::Split), "Split — văn bản mới bắt đầu từ trang này (S, bấm lại để huỷ)").clicked() {
                            viec.push(Viec::Gan(Nhan::Split));
                        }
                        ngan_nho(ui);
                        if nut_nhom(ui, NoiDungNut::Chu("I", None), (nhan_t == Nhan::Index, false), true, "Index — tờ mã vạch, bỏ khi tách (I, bấm lại để huỷ)").clicked() {
                            viec.push(gan(Nhan::Index));
                        }
                        ngan_nho(ui);
                        if nut_nhom(ui, NoiDungNut::Chu("B", None), (nhan_t == Nhan::Trang, false), true, "Trắng — bỏ khi tách (B, bấm lại để huỷ)").clicked() {
                            viec.push(gan(Nhan::Trang));
                        }
                    });
                    ui.add_space(8.0);
                }
                nhom_nut(ui, |ui| {
                    if nut_nhom(ui, NoiDungNut::Icon(BieuTuong::XoayPhai), (false, false), true, "Xoay phải 90° (R)").clicked() {
                        viec.push(Viec::Xoay(90));
                    }
                    ngan_nho(ui);
                    if nut_nhom(ui, NoiDungNut::Icon(BieuTuong::Xoay180), (false, false), true, "Xoay 180° (Shift+R)").clicked() {
                        viec.push(Viec::Xoay(180));
                    }
                    ngan_nho(ui);
                    let (bt, goi_y) = if la_vb { (BieuTuong::TrangGach, "Bỏ khỏi văn bản (Del)") } else { (BieuTuong::ThungRac, "Xoá khỏi lưới (Del)") };
                    if nut_nhom(ui, NoiDungNut::Icon(bt), (false, false), true, goi_y).clicked() {
                        viec.push(if la_vb { Viec::BoKhoiVb } else { Viec::Xoa });
                    }
                });
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
                let anh = match &tex {
                    Some(tex) => anh_nho::ve_xoay(ui.painter(), hop, tex, t.sua.goc),
                    None => {
                        ui.painter().rect_filled(hop, 0.0, Color32::from_gray(225));
                        ui.painter().text(hop.center(), egui::Align2::CENTER_CENTER, "Đang tải…", egui::FontId::proportional(13.0), CHU_PHU);
                        hop
                    }
                };
                // Viền theo nhãn như ô lưới (nét dày hơn cho ảnh lớn)
                let p = ui.painter();
                match t.nhan() {
                    Nhan::Index => {
                        p.rect_stroke(anh, 0.0, Stroke::new(3.0, XANH_COBALT), StrokeKind::Outside);
                    }
                    Nhan::Trang => {
                        p.rect_filled(anh, 0.0, Color32::from_rgba_unmultiplied(0xEA, 0xF3, 0xFC, 110));
                        ve_net_dut(p, anh.expand(1.5), Stroke::new(2.5, XANH_COBALT), 8.0);
                    }
                    Nhan::Split => {
                        p.rect_stroke(anh, 0.0, Stroke::new(2.5, CAM), StrokeKind::Outside);
                        p.rect_filled(Rect::from_min_size(anh.min - egui::vec2(9.0, 0.0), egui::vec2(5.0, anh.height())), 0.0, CAM);
                    }
                    Nhan::NoiDung if t.nghi_trang() => ve_net_dut(p, anh.expand(1.5), Stroke::new(2.5, Color32::from_rgb(0xD4, 0xA0, 0x00)), 8.0),
                    Nhan::NoiDung if t.nghi_xoay().is_some() => ve_net_dut(p, anh.expand(1.5), Stroke::new(2.5, CAM), 8.0),
                    Nhan::NoiDung => {
                        p.rect_stroke(anh, 0.0, Stroke::new(1.0, VIEN_NHAT), StrokeKind::Outside);
                    }
                }
            });
        });
        if dong || ph.should_close() {
            self.phong_to = None;
        }
        // Thao tác áp cho đúng trang đang xem trong hộp (chọn riêng trang đó rồi làm như trên lưới)
        let k = self.phong_to.as_ref().and_then(|p| p.trang());
        for v in viec {
            let Some(k) = k else { break };
            if la_vb { self.chon_vb.chon_rieng(k) } else { self.chon.chon_rieng(k) }
            self.lam(v, cfg);
            // Xoá / bỏ khỏi văn bản: trang ra khỏi danh sách → hộp sang trang kế
            if matches!(v, Viec::Xoa | Viec::BoKhoiVb) {
                if let Some(pt) = &mut self.phong_to {
                    pt.ds.retain(|&x| x != k);
                    pt.i = pt.i.min(pt.ds.len().saturating_sub(1));
                    if pt.ds.is_empty() {
                        self.phong_to = None;
                    }
                }
            }
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
