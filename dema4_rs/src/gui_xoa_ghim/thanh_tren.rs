//! Thanh công cụ màn Xóa ghim (như prototype), chia 2 nửa bằng ranh giới 4 px **thẳng** ranh giới Gốc | Đích bên dưới:
//! - nửa trái: biểu tượng Góc / Cạnh (bấm từng góc / cạnh) / Tùy biến (− vẽ vùng xoá, + vẽ vùng bảo vệ) · cột lọc trang
//!   (theo vùng đang chọn hoặc phạm vi vùng sắp vẽ) · khối Vùng / Rộng / Cao / Mức xoá;
//! - nửa phải: Đầu vào (⊗ · File · Thư mục), File đích (kiểu ghi · đường dẫn mẫu · Thư mục), Nhận diện layout,
//!   Tô nền trắng, Xóa vùng vẽ, Hoàn tác.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind};

use super::thiet_ke as tk;
use super::widget::{self, font, DuoiO};
use super::{ten_pham_vi, LocTrang, ManXoaGhim};
use crate::xoa_ghim::kho::KieuGhi;
use crate::xoa_ghim::vung::{ApTrang, LoaiVung, ViTri, DPI_GOC};

/// Bề rộng nửa trái (= Gốc: cột thu nhỏ + khung xem) khi tổng `w`, cột thu nhỏ `nho` (0 = không có).
pub(super) fn chia_doi(w: f32, nho: f32) -> f32 {
    ((w - tk::RONG_RANH + nho) / 2.0).max(0.0)
}

/// Đường dẫn dài: giữ 3 cấp cuối, đầu thay "…".
fn rut_gon(p: &str) -> String {
    let a: Vec<&str> = p.split(['/', '\\']).collect();
    if p.chars().count() > 42 && a.len() > 4 {
        format!("…/{}", a[a.len() - 3..].join("/"))
    } else {
        p.to_string()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Icon {
    Goc,
    Canh,
    TuyBien,
}

impl ManXoaGhim {
    /// Bề rộng cột trang thu nhỏ (+ viền) đang chiếm trong khung Gốc.
    pub(super) fn rong_nho(&self) -> f32 {
        if self.cfg.thu_nho && self.thu_muc.is_some() && !self.ds.is_empty() {
            tk::RONG_COT_NHO + 1.0
        } else {
            0.0
        }
    }

    pub(super) fn thanh_tren(&mut self, ui: &mut egui::Ui) {
        let tong = ui.available_rect_before_wrap();
        let trai = chia_doi(tong.width(), self.rong_nho());
        let dau = tong.left_top();
        let r_trai = ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(dau, egui::vec2(trai, 400.0))), |ui| {
            // Nửa trái không bao giờ vẽ đè sang nửa phải (cửa sổ hẹp)
            ui.set_clip_rect(ui.clip_rect().intersect(Rect::from_min_size(dau, egui::vec2(trai, 400.0))));
            let x_phai = dau.x + trai - 12.0;
            egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 12, top: 12, bottom: 10 }).show(ui, |ui| {
                ui.set_width((trai - 26.0).max(10.0));
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    self.ba_bieu_tuong(ui);
                    self.cot_loc_trang(ui);
                    widget::vach_doc(ui, 104.0);
                    self.khoi_vung(ui, x_phai);
                });
            });
        });
        let x_phai = dau.x + trai + tk::RONG_RANH;
        let r_phai = ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(egui::pos2(x_phai, dau.y), egui::pos2(tong.right(), dau.y + 400.0))), |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 14, top: 12, bottom: 10 }).show(ui, |ui| {
                ui.set_width((tong.right() - x_phai - 26.0).max(10.0));
                self.nua_phai(ui);
            });
        });
        let cao = r_trai.response.rect.height().max(r_phai.response.rect.height());
        let o = Rect::from_min_size(dau, egui::vec2(tong.width(), cao));
        let p = ui.painter();
        let ranh = Rect::from_min_size(egui::pos2(dau.x + trai, dau.y), egui::vec2(tk::RONG_RANH, cao));
        p.rect_filled(ranh, 0.0, tk::NEN_RANH);
        p.vline(ranh.left() + 0.5, ranh.y_range(), Stroke::new(1.0, tk::VIEN_NHAT));
        p.vline(ranh.right() - 0.5, ranh.y_range(), Stroke::new(1.0, tk::VIEN_NHAT));
        p.hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, tk::VIEN_KHUNG));
        ui.allocate_rect(o, Sense::hover());
    }

    // ---------------- Nửa trái ----------------

    fn ba_bieu_tuong(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 10.0;
        for k in [Icon::Goc, Icon::Canh, Icon::TuyBien] {
            self.bieu_tuong_trang(ui, k);
        }
        ui.spacing_mut().item_spacing.x = 12.0;
    }

    /// 1 biểu tượng trang giấy 58 × 78 + nhãn; vẽ đúng toạ độ SVG prototype, bấm từng góc / cạnh / nửa − +.
    fn bieu_tuong_trang(&mut self, ui: &mut egui::Ui, k: Icon) {
        let (o_tong, _) = ui.allocate_exact_size(egui::vec2(66.0, 104.0), Sense::hover());
        let goc = o_tong.min + egui::vec2(4.0, 2.0);
        let o = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(goc + egui::vec2(x, y), egui::vec2(w, h));
        let p = ui.painter().clone();
        let mut bat_bat_ky = false;
        let duong = |p: &egui::Painter, x0: f32, dx: f32, cuoi: f32| {
            for (i, y) in [24.0, 31.0, 38.0, 45.0, 52.0].into_iter().enumerate() {
                let dai = if i == 4 { cuoi } else { dx };
                p.line_segment([goc + egui::vec2(x0, y), goc + egui::vec2(x0 + dai, y)], Stroke::new(1.4, tk::ICON_DONG));
            }
        };
        match k {
            Icon::Goc | Icon::Canh => {
                p.rect(o(2.0, 2.0, 50.0, 70.0), 0.0, Color32::WHITE, Stroke::new(1.0, tk::ICON_VIEN), StrokeKind::Middle);
                let (ds, ten): (&[(ViTri, f32, f32, f32, f32)], &str) = if k == Icon::Goc {
                    duong(&p, 16.0, 22.0, 14.0);
                    (&[(ViTri::GocTrenTrai, 3.0, 3.0, 12.0, 12.0), (ViTri::GocTrenPhai, 39.0, 3.0, 12.0, 12.0), (ViTri::GocDuoiTrai, 3.0, 59.0, 12.0, 12.0), (ViTri::GocDuoiPhai, 39.0, 59.0, 12.0, 12.0)], "Góc")
                } else {
                    duong(&p, 14.0, 26.0, 18.0);
                    (&[(ViTri::CanhTren, 3.0, 3.0, 48.0, 7.0), (ViTri::CanhDuoi, 3.0, 64.0, 48.0, 7.0), (ViTri::CanhTrai, 3.0, 11.0, 7.0, 52.0), (ViTri::CanhPhai, 44.0, 11.0, 7.0, 52.0)], "Cạnh")
                };
                for &(v, x, y, w, h) in ds {
                    let r = o(x, y, w, h);
                    let bat = self.bo.tm.vung.iter().any(|z| z.vi_tri == v);
                    bat_bat_ky |= bat;
                    let ph = ui.interact(r, ui.scope_id().with(("xg_ic", v as u8)), Sense::click());
                    let nen = if bat { tk::XANH } else if ph.hovered() { tk::ICON_HOVER } else { Color32::WHITE };
                    p.rect(r, 0.0, nen, Stroke::new(1.0, if bat { tk::XANH_DAM_CHU } else { tk::ICON_VIEN }), StrokeKind::Middle);
                    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, bat, v.ten()));
                    if ph.on_hover_text(format!("{} — bấm để {}", v.ten(), if bat { "bỏ" } else { "thêm" })).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.bat_tat_co_san(v);
                    }
                }
                self.nhan_bieu_tuong(ui, o_tong, ten, bat_bat_ky);
            }
            Icon::TuyBien => {
                for (loai, y, nen_bat, vien_bat, tip) in [
                    (LoaiVung::Xoa, 2.0, tk::ICON_VE_XOA_NEN, tk::XANH, "Vẽ vùng xoá — kéo trên trang Gốc · Esc để thôi · F1: mọi thao tác"),
                    (LoaiVung::BaoVe, 37.0, tk::ICON_VE_BAO_NEN, tk::ICON_VE_BAO_VIEN, "Vẽ vùng bảo vệ — kéo trên trang Gốc · Esc để thôi · F1: mọi thao tác"),
                ] {
                    let r = o(2.0, y, 50.0, 35.0);
                    let dang = self.ve == Some(loai);
                    bat_bat_ky |= dang;
                    let ph = ui.interact(r, ui.scope_id().with(("xg_ve", y as u8)), Sense::click());
                    let nen = if dang { nen_bat } else if ph.hovered() { tk::ICON_HOVER } else { Color32::WHITE };
                    p.rect(r, 0.0, nen, Stroke::new(1.0, if dang { vien_bat } else { tk::ICON_VIEN }), StrokeKind::Middle);
                    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, dang, if loai == LoaiVung::Xoa { "Vẽ vùng xoá" } else { "Vẽ vùng bảo vệ" }));
                    if ph.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.bat_ve(loai);
                    }
                }
                let mau_tru = if self.ve == Some(LoaiVung::Xoa) { tk::XANH } else { Color32::from_gray(0x66) };
                let mau_cong = if self.ve == Some(LoaiVung::BaoVe) { tk::ICON_VE_BAO_VIEN } else { Color32::from_gray(0x66) };
                p.line_segment([goc + egui::vec2(19.0, 20.0), goc + egui::vec2(35.0, 20.0)], Stroke::new(1.8, mau_tru));
                p.line_segment([goc + egui::vec2(19.0, 55.0), goc + egui::vec2(35.0, 55.0)], Stroke::new(1.8, mau_cong));
                p.line_segment([goc + egui::vec2(27.0, 47.0), goc + egui::vec2(27.0, 63.0)], Stroke::new(1.8, mau_cong));
                self.nhan_bieu_tuong(ui, o_tong, "Tùy biến", bat_bat_ky);
            }
        }
    }

    fn nhan_bieu_tuong(&self, ui: &egui::Ui, o: Rect, ten: &str, dang: bool) {
        ui.painter().text(egui::pos2(o.center().x, o.top() + 2.0 + 78.0 + 6.0 + 8.0), egui::Align2::CENTER_CENTER, ten, font(tk::CHU_CO), if dang { tk::CHU } else { tk::CHU_PHU });
    }

    /// Bật / tắt chế độ vẽ: bỏ chọn vùng; phạm vi vùng sắp vẽ mặc định Từng trang (giữ Vô đối khi vẽ xoá).
    pub(super) fn bat_ve(&mut self, loai: LoaiVung) {
        if self.ve == Some(loai) {
            self.ve = None;
            return;
        }
        self.ve = Some(loai);
        self.bo.chon.clear();
        if !(loai == LoaiVung::Xoa && self.loc_trang == LocTrang::VoDoi) {
            self.loc_trang = LocTrang::TungTrang;
        }
    }

    /// Lựa chọn đang hiện ở cột lọc trang: đang vẽ → phạm vi sắp vẽ; chọn đúng 1 vùng → phạm vi vùng đó.
    pub(super) fn loc_hien(&self) -> LocTrang {
        if self.ve.is_some() {
            return self.loc_trang;
        }
        match (self.bo.chon.len(), self.bo.vung_chon()) {
            (1, Some(z)) => LocTrang::cua_vung(z),
            _ => self.loc_trang,
        }
    }

    /// Có lựa chọn "Vô đối": đang vẽ vùng xoá, hoặc chọn đúng 1 vùng vẽ xoá / vô đối.
    pub(super) fn co_vo_doi(&self) -> bool {
        let chon1 = (self.bo.chon.len() == 1).then(|| self.bo.chinh()).flatten();
        let vung_ve_xoa = chon1.and_then(|c| self.bo.lay(c)).is_some_and(|z| z.vi_tri == ViTri::Tay && z.loai != LoaiVung::BaoVe);
        self.ve == Some(LoaiVung::Xoa) || (self.ve.is_none() && vung_ve_xoa)
    }

    fn cot_loc_trang(&mut self, ui: &mut egui::Ui) {
        let co_vo_doi = self.co_vo_doi();
        let hien = self.loc_hien();
        ui.vertical(|ui| {
            // Dòng 21 px liền nhau như prototype
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut ds = vec![(LocTrang::TatCa, "Tất cả"), (LocTrang::Le, "Trang lẻ"), (LocTrang::Chan, "Trang chẵn"), (LocTrang::TungTrang, "Từng trang")];
            if co_vo_doi {
                ds.push((LocTrang::VoDoi, "Vô đối"));
            }
            for (l, ten) in ds {
                if widget::tron(ui, hien == l, egui::RichText::new(ten).font(font(tk::CHU_CO))).clicked() {
                    self.chon_loc_trang(l);
                }
            }
        });
    }

    /// Bấm 1 lựa chọn lọc trang: đang chọn đúng 1 vùng (không vẽ) → đổi phạm vi / loại vùng đó (hoàn tác được).
    pub(super) fn chon_loc_trang(&mut self, l: LocTrang) {
        self.loc_trang = l;
        if self.ve.is_some() {
            return;
        }
        let Some(c) = (self.bo.chon.len() == 1).then(|| self.bo.chinh()).flatten() else { return };
        let Some(z) = self.bo.lay(c) else { return };
        let ap = match l {
            LocTrang::TatCa => ApTrang::TatCa,
            LocTrang::Le => ApTrang::Le,
            LocTrang::Chan => ApTrang::Chan,
            LocTrang::TungTrang | LocTrang::VoDoi => match z.ap_trang {
                ApTrang::MotTrang(t) => ApTrang::MotTrang(t),
                _ => ApTrang::MotTrang(self.trang),
            },
        };
        self.bo.doi_ap_trang(c, ap, l == LocTrang::VoDoi);
    }

    /// Combo Vùng + Rộng / Cao / Mức xoá (thanh trượt + ô số) trong phần còn lại tới `x_phai` (mép phải nửa trái).
    /// Hẹp hơn `RONG_CO_THANH`: ẩn thanh trượt, chỉ còn ô số.
    fn khoi_vung(&mut self, ui: &mut egui::Ui, x_phai: f32) {
        const RONG_CO_THANH: f32 = 250.0;
        let rong = (x_phai - ui.cursor().min.x).max(120.0);
        let co_thanh = rong >= RONG_CO_THANH;
        ui.vertical(|ui| {
            ui.set_width(rong);
            // Dòng cao 26, cách nhau 2 → 28 px như prototype
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                let nhan = ui.add_sized(egui::vec2(tk::RONG_NHAN_VUNG, 20.0), egui::Label::new(egui::RichText::new("Vùng:").font(font(tk::CHU_CO)).color(Color32::from_gray(0x33))));
                let chu = match self.bo.chon.len() {
                    0 => "— chọn vùng —".to_string(),
                    1 => self.bo.chinh().map(|c| format!("{} · {}", self.ten_vung(c), self.bo.lay(c).map_or(String::new(), |z| ten_pham_vi(z.ap_trang)))).unwrap_or_default(),
                    n => format!("{n} vùng đang chọn"),
                };
                let ds: Vec<_> = self.bo.tat_ca().into_iter().map(|c| (c, format!("{} · {}", self.ten_vung(c), self.bo.lay(c).map_or(String::new(), |z| ten_pham_vi(z.ap_trang))))).collect();
                egui::ComboBox::from_id_salt("xg_vung")
                    .icon(widget::mui_ten)
                    .width((rong - tk::RONG_NHAN_VUNG - ui.spacing().item_spacing.x).max(60.0))
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .height(320.0)
                    .selected_text(egui::RichText::new(chu).font(font(tk::CHU_CO)))
                    .show_ui(ui, |ui| {
                        for (c, t) in &ds {
                            if ui.selectable_label(self.bo.chon == [*c], egui::RichText::new(t).font(font(tk::CHU_CO))).clicked() {
                                self.bo.chon = vec![*c];
                            }
                        }
                    })
                    .response
                    .labelled_by(nhan.id);
            });
            let (w_pt, h_pt) = self.kho_trang.get(self.trang as usize).copied().unwrap_or((595.0, 842.0));
            let px_pt = DPI_GOC as f32 / 72.0;
            let z = (self.bo.chon.len() == 1).then(|| self.bo.vung_chon().cloned()).flatten();
            // Giá trị % trang + khoá theo loại vùng (cạnh ngang khoá Rộng, cạnh dọc khoá Cao)
            let (mut rong_pt, mut cao_pt, mut nguong, khoa_rong, khoa_cao) = match &z {
                None => (1.0, 1.0, 1.0, true, true),
                Some(v) if v.vi_tri == ViTri::Tay => ((v.rong as f32 * 100.0).round(), (v.cao as f32 * 100.0).round(), v.muc as f32, false, false),
                Some(v) => {
                    let ngang = matches!(v.vi_tri, ViTri::CanhTren | ViTri::CanhDuoi);
                    let doc = matches!(v.vi_tri, ViTri::CanhTrai | ViTri::CanhPhai);
                    let r = if ngang { 100.0 } else { v.rong_px as f32 / (w_pt * px_pt) * 100.0 };
                    let c = if doc { 100.0 } else { v.cao_px as f32 / (h_pt * px_pt) * 100.0 };
                    (r.round(), c.round(), v.muc as f32, ngang, doc)
                }
            };
            let co_nguong = z.as_ref().is_some_and(|v| v.loai != LoaiVung::BaoVe);
            let co = z.is_some();
            let hang = |ui: &mut egui::Ui, ten: &str, gt: &mut f32, max: f32, duoc: bool, don_vi: &str| -> bool {
                let mut doi = false;
                ui.horizontal(|ui| {
                    ui.set_min_height(26.0);
                    ui.add_sized(egui::vec2(tk::RONG_NHAN_VUNG, 20.0), egui::Label::new(egui::RichText::new(ten).font(font(tk::CHU_CO)).color(Color32::from_gray(0x33))));
                    let w = (rong - tk::RONG_NHAN_VUNG - 58.0 - 2.0 * ui.spacing().item_spacing.x).max(40.0);
                    let max_hien = if !duoc && co { 100.0 } else { max };
                    let ten_gon = ten.trim_end_matches(':');
                    if co_thanh {
                        doi |= widget::thanh_truot(ui, gt, 1.0, max_hien, w, duoc, ten_gon).changed();
                    }
                    doi |= widget::o_so(ui, gt, 1.0, max_hien, don_vi, duoc, &format!("{ten_gon} (số)"));
                });
                doi
            };
            let doi_rong = hang(ui, "Rộng:", &mut rong_pt, 60.0, co && !khoa_rong, " %");
            let doi_cao = hang(ui, "Cao:", &mut cao_pt, 60.0, co && !khoa_cao, " %");
            // Mức xoá: cao = mạnh — tới 60 nhận vết nhạy dần, trên 60 phủ dần giấy trong vùng về màu giấy, 100 = đặc
            let doi_nguong = hang(ui, "Mức xoá:", &mut nguong, 100.0, co_nguong, "");
            if (doi_rong || doi_cao || doi_nguong) && co {
                self.bo.ghi_buoc();
                if let Some(v) = self.bo.vung_chon_mut() {
                    if v.vi_tri == ViTri::Tay {
                        if doi_rong {
                            let phai = v.x + v.rong >= 0.999 && v.x > 0.0;
                            v.rong = (rong_pt / 100.0) as f64;
                            v.x = if phai { 1.0 - v.rong } else { v.x.clamp(0.0, 1.0 - v.rong) };
                        }
                        if doi_cao {
                            let duoi = v.y + v.cao >= 0.999 && v.y > 0.0;
                            v.cao = (cao_pt / 100.0) as f64;
                            v.y = if duoi { 1.0 - v.cao } else { v.y.clamp(0.0, 1.0 - v.cao) };
                        }
                    } else {
                        if doi_rong {
                            v.rong_px = (rong_pt / 100.0 * w_pt * px_pt).round().max(5.0) as u32;
                        }
                        if doi_cao {
                            v.cao_px = (cao_pt / 100.0 * h_pt * px_pt).round().max(5.0) as u32;
                        }
                    }
                    if doi_nguong {
                        v.muc = nguong.round().clamp(1.0, 100.0) as u8;
                    }
                }
                self.bo.sua_chon();
            }
        });
    }

    // ---------------- Nửa phải ----------------

    fn nua_phai(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 8.0);
        let nhan = |ui: &mut egui::Ui, t: &str| {
            ui.add_sized(egui::vec2(tk::RONG_NHAN, tk::CAO_NUT), egui::Label::new(egui::RichText::new(t).font(font(tk::CHU_CO))).halign(egui::Align::Min));
        };
        // Đầu vào
        ui.horizontal(|ui| {
            nhan(ui, "Đầu vào:");
            let rong = (ui.available_width() - 2.0 * (tk::RONG_BNUT + 6.0)).max(60.0);
            let chu = match (&self.mot_file, &self.thu_muc) {
                (Some(f), _) => f.display().to_string(),
                (None, Some(t)) => t.display().to_string(),
                _ => String::new(),
            };
            let duoi = if self.thu_muc.is_some() { DuoiO::XoaTron("Đóng — về màn ban đầu") } else { DuoiO::Khong };
            if widget::o_duong_dan(ui, rong, &chu, "Chọn 1 file PDF hoặc cả thư mục…", &chu, tk::CHU, duoi, "Đầu vào") {
                self.dong();
            }
            if widget::nut_bt_chu(ui, "file_pdf", (13.0, 14.0), "File", true).on_hover_text("Mở 1 file PDF (Ctrl+O)").clicked() {
                self.chon_file_pdf();
            }
            if widget::nut_bt_chu(ui, "thu_muc", (15.0, 13.0), "Thư mục", true).on_hover_text("Mở cả thư mục PDF, gồm thư mục con (Ctrl+Shift+O)").clicked() {
                self.chon_thu_muc();
            }
        });
        // File đích
        let ghi = self.bo.tm.ghi.clone();
        ui.horizontal(|ui| {
            nhan(ui, "File đích:");
            ui.add_enabled_ui(!ghi.ghi_de, |ui| {
                let mut kieu = ghi.kieu;
                egui::ComboBox::from_id_salt("xg_kieu_ghi")
                    .icon(widget::mui_ten)
                    .width(110.0)
                    .selected_text(egui::RichText::new(if kieu == KieuGhi::CanhFile { "Cạnh gốc" } else { "Thư mục mới" }).font(font(tk::CHU_CO)))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut kieu, KieuGhi::NhanBan, egui::RichText::new("Thư mục mới").font(font(tk::CHU_CO))).on_hover_text("Nhân bản cây thư mục sang thư mục đích (mặc định <nguồn>_xoaghim)");
                        ui.selectable_value(&mut kieu, KieuGhi::CanhFile, egui::RichText::new("Cạnh gốc").font(font(tk::CHU_CO))).on_hover_text("Ghi <tên>_xoaghim.pdf cùng thư mục với file gốc");
                    })
                    .response
                    .on_hover_text(if ghi.ghi_de { "Đang bật Ghi đè file gốc (Cấu hình)" } else { "Cách ghi file kết quả" })
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Kiểu ghi"));
                if kieu != ghi.kieu {
                    self.bo.tm.ghi.kieu = kieu;
                    self.bo.doi_chung();
                    self.lap_ds_giu_chon();
                }
            });
            let rong = (ui.available_width() - tk::RONG_BNUT - 6.0).max(60.0);
            let mau = self.mau_dich();
            let (chu, mau_chu, duoi, tip) = if ghi.ghi_de {
                ("Ghi đè lên file gốc".to_string(), tk::CAM_CHU, DuoiO::Nhan("ghi đè", tk::CAM_NEN, tk::CAM_CHU), "Bật trong Cấu hình → Xuất file".to_string())
            } else if ghi.kieu == KieuGhi::CanhFile {
                (rut_gon(&mau), tk::CHU, DuoiO::Khong, mau.clone())
            } else if ghi.thu_muc.is_some() {
                (rut_gon(&mau), tk::CHU, DuoiO::XoaTron("Về thư mục mặc định (<nguồn>_xoaghim)"), mau.clone())
            } else {
                (rut_gon(&mau), tk::CHU, DuoiO::Khong, mau.clone())
            };
            if widget::o_duong_dan(ui, rong, &chu, "", &tip, mau_chu, duoi, "File đích") {
                self.bo.tm.ghi.thu_muc = None;
                self.bo.doi_chung();
                self.lap_ds_giu_chon();
            }
            let duoc = !ghi.ghi_de && ghi.kieu != KieuGhi::CanhFile;
            let r = widget::nut_bt_chu(ui, "", (0.0, 0.0), "Chọn", duoc);
            let r = r.on_hover_text(if duoc { "Chọn thư mục đích khác (mặc định: _xoaghim cạnh nguồn)" } else { "Ghi cạnh file gốc / ghi đè — không cần chọn thư mục" });
            if duoc && r.clicked() {
                self.chon_thu_muc_dich();
            }
        });
        // Tuỳ chọn + nút
        ui.horizontal_wrapped(|ui| {
            ui.add_space(tk::RONG_NHAN + 6.0);
            ui.spacing_mut().item_spacing.x = 10.0;
            let co_mo_hinh = crate::xoa_ghim::mo_hinh::co_san();
            let mut nd = self.bo.tm.tuy_chon.nhan_dien && co_mo_hinh;
            let tip = if co_mo_hinh { "Bảo vệ khối chữ, bảng — không xoá vào nội dung (cùng tuỳ chọn trong Cấu hình)" } else { "Cần tải mô hình AI trong Cấu hình" };
            if widget::o_tick(ui, &mut nd, "Nhận diện layout", co_mo_hinh, tip) {
                self.bo.tm.tuy_chon.nhan_dien = nd;
                self.bo.doi_chung();
            }
            if !co_mo_hinh {
                let r = ui.add(egui::Label::new(egui::RichText::new("tải mô hình").font(font(tk::CHU_NHO)).color(tk::XANH_DAM_CHU).underline()).sense(Sense::click()));
                if r.on_hover_text("Mở Cấu hình để tải mô hình (37,5 MB)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.mo_cau_hinh = true;
                }
            }
            let mut nt = self.bo.tm.tuy_chon.nen_trang;
            if widget::o_tick(ui, &mut nt, "Tô nền trắng", true, "Tô trắng đặc cả vùng xoá, chừa nét được bảo vệ (cùng tuỳ chọn trong Cấu hình)") {
                self.bo.tm.tuy_chon.nen_trang = nt;
                self.bo.doi_chung();
            }
            let co_ve = !self.bo.moi_vung_ve().is_empty();
            if widget::nut_link(ui, "Xóa vùng vẽ", co_ve, true).on_hover_text("Xoá hết các vùng vẽ Tùy biến (có hỏi lại). Cạnh, góc giữ nguyên. Xoá riêng vùng đang chọn: phím Delete / chuột phải.").clicked() {
                self.xac_nhan_xoa = Some(self.bo.moi_vung_ve());
            }
            if widget::nut_link(ui, "Hoàn tác", self.bo.co_hoan_tac(), false).on_hover_text("Ctrl+Z").clicked() {
                self.bo.hoan_tac();
            }
        });
    }

    /// Mẫu đường dẫn file kết quả theo kiểu ghi (ô File đích).
    pub(super) fn mau_dich(&self) -> String {
        let ghi = &self.bo.tm.ghi;
        if ghi.kieu == KieuGhi::CanhFile {
            let goc = self.thu_muc.as_ref().map_or("<thư mục file gốc>".to_string(), |p| p.display().to_string());
            return format!("{goc}/{{gốc}}_xoaghim.pdf");
        }
        let ra = match (&ghi.thu_muc, &self.thu_muc) {
            (Some(d), _) => d.display().to_string(),
            (None, Some(_)) => self.thu_muc_ket_qua_hien_tai().map_or(String::new(), |p| p.display().to_string()),
            (None, None) => "<thư mục nguồn>_xoaghim".to_string(),
        };
        let con = if self.mot_file.is_some() { "" } else { "/{thư mục con}" };
        format!("{ra}{con}/{{gốc}}.pdf")
    }

    fn chon_thu_muc_dich(&mut self) {
        let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục ghi kết quả").pick_folder() else { return };
        if let Some(goc) = &self.thu_muc {
            if crate::tach_vb::tach::nam_trong(&p, goc) {
                self.bao("Thư mục đích nằm trong thư mục nguồn — lần quét sau sẽ lẫn kết quả. Chọn thư mục khác.");
                return;
            }
        }
        self.bo.tm.ghi.thu_muc = Some(p);
        self.bo.doi_chung();
        self.lap_ds_giu_chon();
    }

    /// Dựng lại danh sách (đích đổi theo kiểu ghi) mà vẫn giữ file đang xem.
    pub(super) fn lap_ds_giu_chon(&mut self) {
        let dang = self.ds.get(self.chon).map(|(n, _)| n.clone());
        self.ghi_ngay();
        self.lap_ds();
        if let Some(i) = dang.and_then(|n| self.ds.iter().position(|(x, _)| *x == n)) {
            self.chon = i;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chia_doi_thang_ranh_gioi_khung_ben_duoi() {
        // Gốc = cột thu nhỏ 151 + X, Đích = X, ranh 4 → nửa trái = 151 + X
        let w = 1200.0;
        let trai = chia_doi(w, 151.0);
        let x = (w - 4.0 - 151.0) / 2.0;
        assert!((trai - (151.0 + x)).abs() < 1e-3);
        assert!((chia_doi(w, 0.0) - (w - 4.0) / 2.0).abs() < 1e-3);
    }

    #[test]
    fn rut_gon_giu_3_cap_cuoi() {
        assert_eq!(rut_gon("D:/Ket_qua/Xoa_ghim/Phong_luu_tru/Dot_2026/Lo_so_hoa_01/{thư mục con}/{gốc}.pdf"), "…/Lo_so_hoa_01/{thư mục con}/{gốc}.pdf");
        assert_eq!(rut_gon("D:/a/b.pdf"), "D:/a/b.pdf");
    }
}
