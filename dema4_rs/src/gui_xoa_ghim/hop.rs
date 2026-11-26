//! Hộp phụ màn Xóa ghim (như prototype): xác nhận **Xóa n vùng vẽ?** (liệt kê tối đa 8 vùng, nút đỏ) và bảng
//! **thao tác nhanh** (F1).

use eframe::egui::{self, Align, Color32, Rect, Sense};

use super::thiet_ke as tk;
use super::widget::{font, font_dam};
use super::{ten_pham_vi, ManXoaGhim};
use crate::gui_tach::khung_hop as kh;
use crate::xoa_ghim::vung::LoaiVung;

/// Bảng thao tác nhanh (phím / thao tác, việc).
pub const THAO_TAC: [(&str, &str); 13] = [
    ("Bấm góc / cạnh", "trên biểu tượng Góc, Cạnh: thêm vùng; bấm lại để bỏ"),
    ("− / +", "trên biểu tượng Tùy biến: vẽ vùng xoá / bảo vệ (kéo trên trang Gốc)"),
    ("Kéo → phải", "khi không vẽ: chọn vùng nằm trọn trong khung"),
    ("Kéo ← trái", "chọn mọi vùng chạm vào khung"),
    ("Shift + bấm", "chọn thêm / bỏ bớt vùng"),
    ("Ctrl+A", "chọn mọi vùng"),
    ("Delete", "xoá vùng vẽ đang chọn (cạnh, góc giữ nguyên)"),
    ("Ctrl+Z", "hoàn tác"),
    ("Chuột phải", "xoá vùng, đổi xoá ↔ bảo vệ, nhân bản, phạm vi trang"),
    ("Esc", "thôi vẽ / bỏ chọn / đóng hộp"),
    ("[  ]", "văn bản trước / sau"),
    ("PageUp / PageDown", "trang trước / sau"),
    ("+  −  =", "phóng to, thu nhỏ, vừa bề ngang · Ctrl + cuộn chuột: phóng"),
];

impl ManXoaGhim {
    pub(super) fn hop_phu(&mut self, ctx: &egui::Context) {
        self.hop_xac_nhan_xoa(ctx);
        self.hop_tro_giup(ctx);
    }

    fn hop_xac_nhan_xoa(&mut self, ctx: &egui::Context) {
        let Some(ds) = self.xac_nhan_xoa.clone() else { return };
        let ds: Vec<_> = ds.into_iter().filter(|c| self.bo.lay(*c).is_some()).collect();
        let n = ds.len();
        if n == 0 {
            self.xac_nhan_xoa = None;
            return;
        }
        let dong: Vec<(Color32, String, String)> = ds
            .iter()
            .take(8)
            .map(|c| {
                let z = self.bo.lay(*c).expect("đã lọc");
                let mau = match z.loai {
                    LoaiVung::BaoVe => tk::VUNG_BAO_VE_VIEN,
                    LoaiVung::VoDoi => tk::VUNG_VO_DOI_VIEN,
                    LoaiVung::Xoa => tk::VUNG_VE_VIEN,
                };
                (mau, self.ten_vung(*c), ten_pham_vi(z.ap_trang))
            })
            .collect();
        let (mut huy, mut xoa) = (false, false);
        let ph = kh::hop(ctx, "xg_xac_nhan_xoa", 420.0, egui::RichText::new(format!("Xóa {n} vùng vẽ?")).font(font_dam(13.0)), |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 18, right: 18, top: 14, bottom: 10 }).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let mut job = egui::text::LayoutJob::default();
                // Biểu tượng cảnh báo vẽ bằng SVG (phông chữ không có ⚠)
                let (o_cb, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                super::bieu_tuong::ve(ui, "canh_bao", o_cb);
                job.append("Sẽ xoá ", 0.0, egui::TextFormat::simple(font(tk::CHU_CO), tk::CHU));
                job.append("tất cả ", 0.0, egui::TextFormat::simple(font_dam(tk::CHU_CO), tk::CHU));
                job.append(&n.to_string(), 0.0, egui::TextFormat::simple(font_dam(tk::CHU_CO), tk::CHU));
                job.append(" vùng vẽ Tùy biến:", 0.0, egui::TextFormat::simple(font(tk::CHU_CO), tk::CHU));
                ui.label(job);
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (mau, ten, pham_vi) in &dong {
                        ui.horizontal(|ui| {
                            let (o, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
                            ui.painter().rect_filled(Rect::from_center_size(o.center(), egui::vec2(9.0, 9.0)), 2.0, *mau);
                            ui.label(egui::RichText::new(ten).font(font(tk::CHU_CO)).color(tk::CHU));
                            ui.label(egui::RichText::new(format!("· {pham_vi}")).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
                        });
                    }
                    if n > 8 {
                        ui.label(egui::RichText::new(format!("… và {} vùng khác", n - 8)).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
                    }
                });
                ui.label(egui::RichText::new("Vùng cạnh, góc giữ nguyên. Có thể hoàn tác bằng Ctrl+Z.").font(font(12.0)).color(tk::CHU_PHU));
            });
            kh::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    xoa = ui.add(kh::nut_chan_mau(&format!("Xóa {n} vùng"), tk::DO_NUT)).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter));
                    huy = ui.add(kh::nut_chan("Hủy")).clicked();
                });
            });
        });
        if xoa {
            let da = self.bo.xoa(&ds);
            self.bao(format!("Đã xoá {da} vùng vẽ · Ctrl+Z để hoàn tác"));
        }
        if xoa || huy || ph.should_close() {
            self.xac_nhan_xoa = None;
        }
    }

    fn hop_tro_giup(&mut self, ctx: &egui::Context) {
        if !self.tro_giup {
            return;
        }
        let mut dong = false;
        let ph = kh::hop(ctx, "xg_tro_giup", 560.0, egui::RichText::new("Xóa ghim — thao tác nhanh").font(font_dam(13.0)), |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 18, right: 18, top: 12, bottom: 10 }).show(ui, |ui| {
                egui::Grid::new("xg_bang_phim").num_columns(2).spacing(egui::vec2(14.0, 8.0)).show(ui, |ui| {
                    for (phim, viec) in THAO_TAC {
                        let g = ui.painter().layout_no_wrap(phim.to_string(), font(11.5), tk::CHU);
                        let (o, _) = ui.allocate_exact_size(g.size() + egui::vec2(12.0, 5.0), Sense::hover());
                        ui.painter().rect(o, 4.0, Color32::from_rgb(0xF3, 0xF4, 0xF6), egui::Stroke::new(1.0, Color32::from_rgb(0xD1, 0xD5, 0xDB)), egui::StrokeKind::Inside);
                        ui.painter().hline(o.x_range().shrink(2.0), o.bottom() - 1.0, egui::Stroke::new(1.0, Color32::from_rgb(0xD1, 0xD5, 0xDB)));
                        ui.painter().galley(o.min + egui::vec2(6.0, 2.0), g, tk::CHU);
                        ui.label(egui::RichText::new(viec).font(font(tk::CHU_CO)).color(tk::CHU));
                        ui.end_row();
                    }
                });
            });
            kh::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    dong = ui.add(kh::nut_chan("Đóng")).clicked();
                });
            });
        });
        if dong || ph.should_close() {
            self.tro_giup = false;
        }
    }
}
