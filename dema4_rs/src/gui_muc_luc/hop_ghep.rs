//! Hộp "Ghép biến…": Nhóm theo + bảng `! | Biến | Lấy từ | Giá trị — hồ sơ đang xem` (cùng ô Lấy từ như sidebar).
//! Sửa áp ngay; Huỷ trả lại ràng buộc lúc mở hộp, Lưu giữ, Tự khớp lại khớp lại từ đầu theo tên cột.

use eframe::egui::{self, Align, Color32, Rect, RichText, Sense, Stroke};

use super::bang_bien::{self, Loai, VaoBang, Viec};
use super::ManMucLuc;
use crate::gui::{CHU, CHU_PHU, NEN_TIEU_DE, VIEN_NHAT};
use crate::gui_tach::khung_hop;
use crate::muc_luc::{khop, Nguon};

const CAO_DONG: f32 = 34.0;
const RONG: [f32; 3] = [34.0, 190.0, 380.0];

impl ManMucLuc {
    pub(super) fn mo_hop_ghep(&mut self) {
        self.mo = None;
        self.mo_hop = None;
        self.hop_ghep = Some(self.rb.clone());
    }

    fn gia_tri(&self, b: &str) -> Option<String> {
        let d = self.du_lieu.as_ref()?;
        match self.rb.nguon(b)? {
            Nguon::Cot(c) => Some(self.kq.ho_so.get(self.hs).and_then(|h| h.dong.first()).map_or(String::new(), |&r| d.bang.o(r, c).to_string())),
            Nguon::Hang(h) => Some(h.clone()),
            Nguon::TuDong => Some("Word tự điền khi in".into()),
        }
    }

    pub(super) fn hop_ghep(&mut self, ctx: &egui::Context) {
        let (Some(goc), Some(m), Some(d)) = (&self.hop_ghep, &self.mau, &self.du_lieu) else { return };
        let goc = goc.clone();
        // Esc lúc đang mở khung xổ = đóng khung đó, không phải Huỷ cả hộp
        let esc_dong_khung = self.mo_hop.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let (mut huy, mut luu, mut tu_khop) = (false, false, false);
        let mut viec = Vec::new();
        let mut nhom_moi = None;
        let tong = m.nhom.tong();
        let chua = m.nhom.nhom.iter().flatten().filter(|b| m.nhom.loai(&self.rb, b) == Loai::ChuaGhep).count();
        let ten_hs = self.kq.ho_so.get(self.hs).map(|h| h.khoa.clone()).unwrap_or_default();
        let gia_tri: Vec<(String, Option<String>)> = m.nhom.nhom.iter().flatten().map(|b| (b.clone(), self.gia_tri(b))).collect();
        let tieu_de = khung_hop::tieu_de_dam("Ghép biến — ", &super::ten_mau(&self.mau_chon));
        let (so_hs, so_vb) = (self.kq.ho_so.len(), self.so_van_ban());
        let ph = khung_hop::hop(ctx, "ml_hop_ghep", 920.0, tieu_de, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 12)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let nhan = ui.label("Nhóm theo");
                    let ten = |c: &str| format!("{c} ({})", d.bang.cot(c).and_then(|i| d.khac_nhau.get(i)).copied().unwrap_or(0));
                    super::kieu_o_chon(ui, Color32::WHITE, crate::gui::VIEN);
                    egui::ComboBox::from_id_salt("ml_hop_nhom")
                        .icon(super::mui_ten_chon)
                        .width(240.0)
                        .selected_text(ten(&self.rb.cot_nhom))
                        .show_ui(ui, |ui| {
                            for c in d.bang.tieu_de.iter().filter(|c| !c.trim().is_empty()) {
                                if ui.selectable_label(*c == self.rb.cot_nhom, ten(c)).clicked() {
                                    nhom_moi = Some(c.clone());
                                }
                            }
                        })
                        .response
                        .labelled_by(nhan.id);
                    ui.label(RichText::new("(dòng liền nhau cùng giá trị = 1 hồ sơ)").color(CHU_PHU));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{so_hs} hồ sơ · {so_vb} văn bản · đã ghép {}/{} biến", tong - chua, tong)).color(CHU_PHU));
                    });
                });
            });
            khung_hop::ke_ngang(ui, VIEN_NHAT);
            // Tiêu đề bảng
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::hover());
            ui.painter().rect_filled(r, 0.0, NEN_TIEU_DE);
            ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
            let mut x = r.left();
            for (i, t) in ["", "Biến trong mẫu", "Lấy từ", &format!("Giá trị — hồ sơ {ten_hs}")].into_iter().enumerate() {
                ui.painter().text(egui::pos2(x + 12.0, r.center().y), egui::Align2::LEFT_CENTER, t, egui::FontId::proportional(12.5), CHU_PHU);
                x += RONG.get(i).copied().unwrap_or(0.0);
            }
            egui::ScrollArea::vertical().id_salt("ml_hop_bang").max_height(470.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let v = VaoBang { nhom: &m.nhom, rb: &self.rb, cot: &d.bang.tieu_de, khac_nhau: &d.khac_nhau, mo: self.mo_hop.as_deref(), sang: None };
                let ten_nhom = ["Thông tin hồ sơ (in một lần)", "Bảng văn bản (mỗi văn bản một dòng)", "Tự động"];
                for (g, ds) in m.nhom.nhom.iter().enumerate() {
                    if ds.is_empty() {
                        continue;
                    }
                    bang_bien::hang_nhom(ui, ten_nhom[g]);
                    for b in ds {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_DONG), Sense::hover());
                        ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(0xEF)));
                        let loai = m.nhom.loai(&self.rb, b);
                        if loai == Loai::ChuaGhep {
                            ui.painter().text(egui::pos2(r.left() + RONG[0] / 2.0, r.center().y), egui::Align2::CENTER_CENTER, "!", egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), Color32::from_rgb(0xB8, 0x5F, 0x00));
                        }
                        bang_bien::chip(ui, Rect::from_min_max(egui::pos2(r.left() + RONG[0] + 6.0, r.top()), egui::pos2(r.left() + RONG[0] + RONG[1], r.bottom())), b, loai.mau());
                        let o = Rect::from_min_size(egui::pos2(r.left() + RONG[0] + RONG[1], r.top() + 3.0), egui::vec2(RONG[2] - 12.0, CAO_DONG - 6.0));
                        let tu_dong = (g == 2).then_some("tự động");
                        let mo = self.mo_hop.as_deref() == Some(b.as_str());
                        let ph_o = ui.interact(o, ui.scope_id().with(("ml_hop_o", b)), if tu_dong.is_some() { Sense::hover() } else { Sense::click() });
                        bang_bien::o_lay_tu(ui, o, self.rb.nguon(b), tu_dong, mo, ph_o.hovered());
                        if tu_dong.is_none() && ph_o.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            viec.push(if mo { Viec::DongSua } else { Viec::MoSua(b.clone()) });
                        }
                        let x_gt = r.left() + RONG[0] + RONG[1] + RONG[2];
                        let (chu, mau, nghieng) = match gia_tri.iter().find(|(x, _)| x == b).and_then(|(_, g)| g.clone()) {
                            Some(g) => (g, CHU, false),
                            None => (format!("chưa ghép — để nguyên {{{b}}}"), Color32::from_rgb(0xB8, 0x5F, 0x00), true),
                        };
                        let mut job = egui::text::LayoutJob::single_section(chu, egui::TextFormat { font_id: egui::FontId::proportional(12.5), color: mau, italics: nghieng, ..Default::default() });
                        job.wrap = egui::text::TextWrapping::truncate_at_width(r.right() - x_gt - 12.0);
                        let gal = ui.fonts_mut(|f| f.layout_job(job));
                        ui.painter().galley(egui::pos2(x_gt, r.center().y - gal.size().y / 2.0), gal, mau);
                        if mo {
                            ui.horizontal(|ui| {
                                ui.add_space(RONG[0] + RONG[1] - 6.0);
                                ui.allocate_ui_with_layout(egui::vec2(RONG[2] + 6.0, 0.0), egui::Layout::top_down(Align::Min), |ui| {
                                    bang_bien::khung_sua(ui, &v, b, &mut self.o_go, &mut viec);
                                });
                            });
                        }
                    }
                }
            });
            khung_hop::ke_ngang(ui, VIEN_NHAT);
            egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 8)).show(ui, |ui| {
                ui.label(RichText::new("Tự động, không cần ghép: {trang_so} {tong_so_trang} — Word tự tính khi in. Biến chưa ghép giữ nguyên {…} trong file kết quả.").size(12.0).color(CHU_PHU));
            });
            khung_hop::chan(ui, |ui| {
                tu_khop = ui.add(khung_hop::nut_chan("Tự khớp lại")).clicked();
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    luu = ui.add(khung_hop::nut_chan_mau("Lưu", crate::gui::XANH_LA)).clicked();
                    huy = ui.add(khung_hop::nut_chan("Huỷ")).clicked();
                });
            });
        });
        for v in viec {
            match v {
                Viec::MoSua(b) => {
                    self.o_go = match self.rb.nguon(&b) {
                        Some(Nguon::Hang(h)) => h.clone(),
                        _ => String::new(),
                    };
                    self.mo_hop = Some(b);
                }
                Viec::DongSua => self.mo_hop = None,
                Viec::DatNguon(b, n) => {
                    self.dat_nguon(&b, n);
                    self.mo_hop = None;
                }
            }
        }
        if let Some(c) = nhom_moi {
            self.dat_nhom(&c);
        }
        if tu_khop {
            if let (Some(m), Some(d)) = (&self.mau, &self.du_lieu) {
                self.rb = khop::tu_khop(&m.db, &d.bang.tieu_de);
                self.mo_hop = None;
                self.da_doi();
            }
        }
        if huy || (ph.should_close() && !luu && !esc_dong_khung) {
            self.rb = goc;
            self.da_doi();
            self.hop_ghep = None;
            self.mo_hop = None;
        } else if luu {
            self.hop_ghep = None;
            self.mo_hop = None;
        }
    }
}
