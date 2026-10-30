//! Popup "Liên hệ Sao Mai" khi hết lượt Đếm / Tách (theo prototype `#het-luot`): logo, ổ khoá đỏ + lý do,
//! mã máy / điện thoại / email (mỗi dòng có Copy), website; nút Đóng, Kiểm tra lại (hỏi trạm, chờ tối đa 8 s).

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Layout, Margin, RichText, Sense, Stroke};

use crate::gui::{ChuDam, CHU, CHU_PHU, DO, XANH_COBALT, XANH_LA};
use crate::gui_tach::khung_hop as kh;
use crate::license::{ChucNang, KiemTra, QuyetDinh};

pub const DIEN_THOAI: &str = "0979824615";
pub const EMAIL: &str = "hotro@saomaiso.com";
const RONG: f32 = 380.0;
/// Lề trái + cột nhãn (prototype: 18 + 70 + 8) — dòng lý do canh theo cột giá trị.
const LE: f32 = 18.0;
const COT_NHAN: f32 = 70.0;
const KHOANG: f32 = 8.0;
const CHO_TOI_DA: Duration = Duration::from_secs(8);

pub enum KetQuaHop {
    Mo,
    Dong,
    /// Kiểm tra lại xong và đã có lượt: nơi gọi chạy luôn.
    DuocChay,
}

pub struct HopLicense {
    pub qd: QuyetDinh,
    dang_kiem: Option<(Receiver<()>, Instant)>,
    da_chep: Option<(&'static str, Instant)>,
}

impl HopLicense {
    pub fn moi(qd: QuyetDinh) -> Self {
        HopLicense { qd, dang_kiem: None, da_chep: None }
    }

    fn chuc_nang(&self) -> ChucNang {
        match self.qd {
            QuyetDinh::HetLuot(cn) | QuyetDinh::HetLuotOffline(cn) => cn,
            QuyetDinh::Cho => ChucNang::Dem,
        }
    }

    pub fn ve(&mut self, ctx: &egui::Context, kt: &KiemTra) -> KetQuaHop {
        if let Some((nhan, tu)) = &self.dang_kiem {
            let xong = !matches!(nhan.try_recv(), Err(TryRecvError::Empty)) || tu.elapsed() > CHO_TOI_DA;
            if !xong {
                ctx.request_repaint_after(Duration::from_millis(100));
            } else {
                self.dang_kiem = None;
                match kt.quyet_dinh(self.chuc_nang()) {
                    QuyetDinh::Cho => return KetQuaHop::DuocChay,
                    qd => self.qd = qd,
                }
            }
        }
        if self.da_chep.is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(1)) {
            self.da_chep = None;
        }
        let ma = kt.ma_may().to_string();
        let ten_cn = if self.chuc_nang() == ChucNang::Dem { "Đếm" } else { "Tách" };
        let mut kq = KetQuaHop::Mo;
        let phan_hoi = kh::hop(ctx, "hop_license", RONG, "Liên hệ Sao Mai", |ui| {
            egui::Frame::new().inner_margin(Margin { left: 16, right: 16, top: 14, bottom: 0 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    let tex = crate::tro_giup::logo(ctx);
                    let anh = egui::Image::new(&tex).fit_to_exact_size(egui::vec2(64.0, 64.0 * tex.size()[1] as f32 / tex.size()[0] as f32)).sense(Sense::click());
                    if ui.add(anh).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("saomaiso.com").clicked() {
                        ctx.open_url(egui::OpenUrl::new_tab(crate::tro_giup::WEBSITE));
                    }
                });
            });
            egui::Frame::new().inner_margin(Margin { left: LE as i8, right: LE as i8, top: 16, bottom: 6 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (o, _) = ui.allocate_exact_size(egui::vec2(COT_NHAN + KHOANG, 40.0), Sense::hover());
                    o_khoa(ui, egui::Rect::from_min_size(o.left_center() - egui::vec2(0.0, 15.0), egui::vec2(30.0, 30.0)), DO);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(RichText::new(format!("Lý do: Số lần {ten_cn} còn lại (")).size(13.0).color(CHU));
                            ui.label(RichText::new("0").size(13.0).color(DO).dam());
                            ui.label(RichText::new(")").size(13.0).color(CHU));
                        });
                        ui.label(RichText::new("Liên hệ với Sao Mai để được hỗ trợ.").size(13.0).color(CHU));
                    });
                });
            });
            if matches!(self.qd, QuyetDinh::HetLuotOffline(_)) {
                egui::Frame::new().inner_margin(Margin { left: (LE + COT_NHAN + KHOANG) as i8, right: LE as i8, top: 0, bottom: 6 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(egui::Label::new(RichText::new("Chưa kết nối được máy chủ — kết nối mạng rồi bấm Kiểm tra lại.").size(11.5).color(CHU_PHU)).wrap());
                });
            }
            egui::Frame::new().inner_margin(Margin { left: LE as i8, right: LE as i8, top: 8, bottom: 14 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 7.0;
                let mailto = format!("mailto:{EMAIL}?subject={}", ma_url(&format!("Cấp lượt dùng dema4 - {ma}")));
                let dong: [(&'static str, &str, Option<&str>, bool); 4] = [
                    ("Mã máy", &ma, None, true),
                    ("Điện thoại", DIEN_THOAI, None, true),
                    ("Email", EMAIL, Some(&mailto), true),
                    ("Website", "saomaiso.com", Some(crate::tro_giup::WEBSITE), false),
                ];
                for (nhan, gia_tri, lien_ket, co_chep) in dong {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = KHOANG;
                        ui.allocate_ui_with_layout(egui::vec2(COT_NHAN, 20.0), Layout::left_to_right(Align::Center), |ui| {
                            ui.set_min_width(COT_NHAN);
                            ui.label(RichText::new(nhan).size(12.0).color(CHU_PHU));
                        });
                        match lien_ket {
                            Some(url) => {
                                ui.hyperlink_to(RichText::new(gia_tri).size(12.0).color(XANH_COBALT), url);
                            }
                            None => {
                                ui.label(RichText::new(gia_tri).size(12.0).color(CHU).dam());
                            }
                        }
                        if co_chep {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let chu = if self.da_chep.is_some_and(|(n, _)| n == nhan) { "Đã chép" } else { "Copy" };
                                if ui.add(egui::Button::new(RichText::new(chu).size(11.0)).min_size(egui::vec2(0.0, 20.0))).clicked() {
                                    ctx.copy_text(gia_tri.to_string());
                                    self.da_chep = Some((nhan, Instant::now()));
                                }
                            });
                        }
                    });
                }
            });
            kh::chan(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let dang = self.dang_kiem.is_some();
                    let nut = kh::nut_chan_mau(if dang { "Đang kiểm tra…" } else { "Kiểm tra lại" }, XANH_LA);
                    if ui.add_enabled(!dang, nut).clicked() {
                        self.dang_kiem = Some((kt.kiem_tra_lai(), Instant::now()));
                    }
                    if ui.add(kh::nut_chan("Đóng")).clicked() {
                        kq = KetQuaHop::Dong;
                    }
                });
            });
        });
        if phan_hoi.should_close() {
            kq = KetQuaHop::Dong;
        }
        kq
    }
}

/// Ổ khoá nét mảnh (như icon prototype): thân bo góc, quai, khe khoá.
fn o_khoa(ui: &egui::Ui, r: egui::Rect, mau: Color32) {
    let p = ui.painter();
    let s = r.width() / 24.0;
    let net = Stroke::new(1.75 * s, mau);
    let diem = |x: f32, y: f32| r.min + egui::vec2(x * s, y * s);
    p.rect_stroke(egui::Rect::from_min_max(diem(4.5, 10.5), diem(19.5, 20.5)), 1.5 * s, net, egui::StrokeKind::Middle);
    let quai: Vec<egui::Pos2> = (0..=16).map(|i| {
        let g = std::f32::consts::PI * i as f32 / 16.0;
        diem(12.0 - 4.0 * g.cos(), 7.5 - 4.0 * g.sin())
    }).collect();
    p.line_segment([diem(8.0, 10.5), diem(8.0, 7.5)], net);
    p.line_segment([diem(16.0, 10.5), diem(16.0, 7.5)], net);
    p.add(egui::Shape::line(quai, net));
    p.line_segment([diem(12.0, 14.5), diem(12.0, 17.0)], net);
}

/// Mã hoá phần trăm cho tiêu đề mailto (UTF-8, giữ chữ / số / `-._~`).
fn ma_url(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailto_ma_hoa_tieng_viet() {
        assert_eq!(ma_url("Cấp lượt - SM-1A"), "C%E1%BA%A5p%20l%C6%B0%E1%BB%A3t%20-%20SM-1A");
    }
}
