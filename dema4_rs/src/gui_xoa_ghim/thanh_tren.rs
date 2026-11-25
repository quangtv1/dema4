//! Thanh trên màn Xóa ghim (như XoaGhim): 3 biểu tượng trang giấy — **Góc** (bấm từng góc), **Cạnh** (bấm từng cạnh),
//! **Tùy biến** (nửa trên "−" vẽ vùng xoá, nửa dưới "+" vẽ vùng bảo vệ) · cột lọc trang · vùng đang chọn + Rộng / Cao
//! (% trang) / Ngưỡng · DPI, Nén, thư mục kết quả, tuỳ chọn bảo vệ · nút **Xóa ghim**.

use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind};

use super::{LocTrang, ManXoaGhim, TabCongCu};
use crate::gui::{CHU, CHU_PHU, DO, NEN_TAB, VIEN, VIEN_NHAT, XANH_COBALT};
use crate::gui_tach::bieu_tuong::{atom_mau, BieuTuong};
use crate::xoa_ghim::vung::{ApTrang, LoaiVung, ViTri, Vung, DPI_GOC};

const CAC_DPI: [u16; 6] = [0, 300, 250, 200, 100, 72];
const CAC_CHAT_LUONG: [u8; 4] = [100, 90, 80, 70];
/// Bề rộng đủ để mọi khối nằm 1 hàng; hẹp hơn thì tuỳ chọn xuất xuống hàng 2.
const RONG_MOT_HANG: f32 = 1280.0;
const CO_ICON: egui::Vec2 = egui::vec2(66.0, 82.0);
const XANH: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);
const HONG: Color32 = Color32::from_rgb(0xF4, 0x72, 0xB6);
const XAM_NET: Color32 = Color32::from_rgb(0x9C, 0xA3, 0xAF);

#[derive(Clone, Copy, PartialEq, Eq)]
enum KieuIcon {
    Goc,
    Canh,
    TuyBien,
}

/// Vạch ngăn dọc cao cố định (vạch ngăn của egui trong hàng ngang lấy hết chiều cao còn lại của thanh).
fn ke_doc(ui: &mut egui::Ui, cao: f32) {
    let (o, _) = ui.allocate_exact_size(egui::vec2(9.0, cao), Sense::hover());
    ui.painter().vline(o.center().x, o.y_range(), Stroke::new(1.0, VIEN_NHAT));
}

/// Khung tờ giấy trong ô biểu tượng (chừa chỗ cho tờ xếp chồng phía sau).
fn to_giay(o: Rect) -> Rect {
    Rect::from_min_max(o.min + egui::vec2(10.0, 4.0), o.max - egui::vec2(4.0, 10.0))
}

/// Ô bấm được của 1 vùng có sẵn trên tờ giấy biểu tượng.
fn o_vung(giay: Rect, v: ViTri) -> Rect {
    let (g, d) = (15.0, 8.0);
    let (l, t, r, b) = (giay.left(), giay.top(), giay.right(), giay.bottom());
    match v {
        ViTri::GocTrenTrai => Rect::from_min_size(egui::pos2(l, t), egui::vec2(g, g)),
        ViTri::GocTrenPhai => Rect::from_min_size(egui::pos2(r - g, t), egui::vec2(g, g)),
        ViTri::GocDuoiTrai => Rect::from_min_size(egui::pos2(l, b - g), egui::vec2(g, g)),
        ViTri::GocDuoiPhai => Rect::from_min_size(egui::pos2(r - g, b - g), egui::vec2(g, g)),
        ViTri::CanhTrai => Rect::from_min_max(egui::pos2(l, t + g), egui::pos2(l + d, b - g)),
        ViTri::CanhPhai => Rect::from_min_max(egui::pos2(r - d, t + g), egui::pos2(r, b - g)),
        ViTri::CanhTren => Rect::from_min_max(egui::pos2(l + d, t), egui::pos2(r - d, t + d)),
        ViTri::CanhDuoi => Rect::from_min_max(egui::pos2(l + d, b - d), egui::pos2(r - d, b)),
        ViTri::Tay => giay,
    }
}

impl ManXoaGhim {
    pub(super) fn thanh_tren(&mut self, ui: &mut egui::Ui) {
        // Hàng tab (như XoaGhim): Tệp tin · Xem · Chỉnh sửa · Cài đặt — góc phải: thu gọn / mở thanh công cụ + nút chạy
        egui::Panel::top("xg_tab").frame(egui::Frame::new().fill(NEN_TAB).inner_margin(egui::Margin { left: 8, right: 6, top: 3, bottom: 0 })).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (t, ten) in [(TabCongCu::TepTin, "Tệp tin"), (TabCongCu::Xem, "Xem"), (TabCongCu::ChinhSua, "Chỉnh sửa"), (TabCongCu::CaiDat, "Cài đặt")] {
                    let dang = self.tab == t && !self.an_cong_cu;
                    let g = ui.painter().layout_no_wrap(ten.into(), egui::FontId::proportional(13.0), CHU);
                    let (o, ph) = ui.allocate_exact_size(egui::vec2(g.size().x + 30.0, 30.0), Sense::click());
                    if dang {
                        ui.painter().rect_filled(o, CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, Color32::WHITE);
                    } else if ph.hovered() {
                        ui.painter().rect_filled(o.shrink(2.0), 3.0, Color32::from_gray(0xE8));
                    }
                    ui.painter().galley(o.center() - g.size() / 2.0, g, CHU);
                    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, dang, ten));
                    if ph.clicked() {
                        self.an_cong_cu = self.tab == t && !self.an_cong_cu;
                        self.tab = t;
                    }
                }
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 30.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    self.nut_chay(ui);
                    ui.add_space(8.0);
                    // ^ thu gọn / v mở thanh công cụ
                    let (o, ph) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                    if ph.hovered() {
                        ui.painter().rect_filled(o, 3.0, Color32::from_gray(0xE5));
                    }
                    let c = o.center();
                    let d = if self.an_cong_cu { 1.0 } else { -1.0 };
                    let net = Stroke::new(1.5, CHU_PHU);
                    ui.painter().line_segment([c + egui::vec2(-5.0, -2.5 * d), c + egui::vec2(0.0, 2.5 * d)], net);
                    ui.painter().line_segment([c + egui::vec2(0.0, 2.5 * d), c + egui::vec2(5.0, -2.5 * d)], net);
                    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Thu gọn thanh công cụ"));
                    if ph.on_hover_text(if self.an_cong_cu { "Mở thanh công cụ" } else { "Thu gọn thanh công cụ" }).clicked() {
                        self.an_cong_cu = !self.an_cong_cu;
                    }
                });
            });
        });
        if self.an_cong_cu {
            return;
        }
        egui::Panel::top("xg_tren").frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(egui::Margin { left: 14, right: 12, top: 10, bottom: 8 }).stroke(Stroke::new(1.0, VIEN_NHAT))).show(ui, |ui| match self.tab {
            TabCongCu::ChinhSua => {
                // Rộng: 1 hàng như XoaGhim. Hẹp: tuỳ chọn xuất xuống hàng 2 (1 dòng gọn)
                let rong = ui.available_width() >= RONG_MOT_HANG;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for k in [KieuIcon::Goc, KieuIcon::Canh, KieuIcon::TuyBien] {
                        self.bieu_tuong(ui, k);
                    }
                    ui.add_space(4.0);
                    self.cot_loc_trang(ui);
                    ke_doc(ui, 118.0);
                    let rong_vung = if rong { (ui.available_width() - 560.0).clamp(360.0, 760.0) } else { (ui.available_width() - 20.0).clamp(360.0, 760.0) };
                    self.khoi_vung(ui, rong_vung);
                    if rong {
                        ke_doc(ui, 118.0);
                        ui.vertical(|ui| self.khoi_xuat(ui, false));
                    }
                });
                if !rong {
                    ui.separator();
                    self.khoi_xuat(ui, true);
                }
            }
            TabCongCu::TepTin => {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Mở file PDF…").min_size(egui::vec2(120.0, 30.0))).clicked() {
                        self.chon_file_pdf();
                    }
                    if ui.add(egui::Button::new("Mở thư mục…").min_size(egui::vec2(120.0, 30.0))).on_hover_text("Hoặc kéo thả thư mục / PDF vào cửa sổ").clicked() {
                        self.chon_thu_muc();
                    }
                    ke_doc(ui, 30.0);
                    let ra = self.thu_muc.as_deref().map(super::thu_muc_ket_qua);
                    let co = matches!(&ra, Some(Ok(p)) if p.is_dir());
                    if ui.add_enabled(co, egui::Button::new("Mở thư mục kết quả").min_size(egui::vec2(140.0, 30.0))).clicked() {
                        if let Some(Ok(p)) = &ra {
                            crate::gui::mo_bang_he_dieu_hanh(p);
                        }
                    }
                    ui.add_space(10.0);
                    match &self.thu_muc {
                        Some(p) => ui.add(egui::Label::new(RichText::new(p.display().to_string()).color(CHU_PHU)).truncate()),
                        None => ui.label(RichText::new("Chưa mở thư mục").color(CHU_PHU)),
                    };
                });
            }
            TabCongCu::Xem => {
                ui.horizontal(|ui| {
                    ui.label("Kiểu xem:");
                    ui.selectable_value(&mut self.lien_tiep, true, "Liên tiếp");
                    ui.selectable_value(&mut self.lien_tiep, false, "Từng trang");
                    ke_doc(ui, 24.0);
                    if ui.selectable_label(self.ty_le.is_none() && !self.vua_cao, "Vừa ngang").clicked() {
                        self.ty_le = None;
                        self.vua_cao = false;
                    }
                    if ui.selectable_label(self.ty_le.is_none() && self.vua_cao, "Vừa cao").clicked() {
                        self.ty_le = None;
                        self.vua_cao = true;
                    }
                    ke_doc(ui, 24.0);
                    ui.checkbox(&mut self.hien_anh_nho, "Trang thu nhỏ");
                    let mut hien_dich = !self.an_dich;
                    if ui.checkbox(&mut hien_dich, "Khung Đích").changed() {
                        self.an_dich = !hien_dich;
                    }
                });
            }
            TabCongCu::CaiDat => {
                ui.horizontal(|ui| {
                    let tc = &mut self.bo.tm.tuy_chon;
                    let mut doi = ui.checkbox(&mut tc.bao_ve_net_den, "Bảo vệ nét đen").on_hover_text("Không lấy điểm rất tối làm vết (giữ nét chữ đen)").changed();
                    doi |= ui.checkbox(&mut tc.bao_ve_do_xanh, "Bảo vệ dấu đỏ / chữ ký xanh").changed();
                    doi |= ui.checkbox(&mut tc.nen_trang, "Nền trắng").on_hover_text("Tô vết bằng trắng thay cho màu nền đo được").changed();
                    if doi {
                        self.bo.doi_chung();
                    }
                    ke_doc(ui, 24.0);
                    ui.label(RichText::new("Vùng chung lưu theo thư mục, vùng vẽ (Tùy biến) lưu riêng từng file — tự lưu.").color(CHU_PHU));
                });
            }
        });
    }

    /// 1 biểu tượng trang giấy xếp chồng + nhãn dưới.
    fn bieu_tuong(&mut self, ui: &mut egui::Ui, k: KieuIcon) {
        ui.vertical(|ui| {
            let (o, ph) = ui.allocate_exact_size(CO_ICON, Sense::click());
            let p = ui.painter();
            let giay = to_giay(o);
            // Tờ phía sau + tờ trước
            p.rect(giay.translate(egui::vec2(-5.0, 5.0)), 1.0, Color32::from_gray(0xF3), Stroke::new(1.0, XAM_NET), StrokeKind::Inside);
            p.rect(giay, 1.0, Color32::WHITE, Stroke::new(1.0, XAM_NET), StrokeKind::Inside);
            let tro = ph.hover_pos();
            let mut bam = None;
            match k {
                KieuIcon::Goc | KieuIcon::Canh => {
                    let ds: &[ViTri] = if k == KieuIcon::Goc { &ViTri::CO_SAN[..4] } else { &ViTri::CO_SAN[4..] };
                    for &v in ds {
                        let r = o_vung(giay, v);
                        let bat = self.bo.tm.vung.iter().any(|z| z.vi_tri == v);
                        let tren = tro.is_some_and(|q| r.contains(q));
                        let nen = if bat { XANH } else if tren { XANH.gamma_multiply(0.3) } else { Color32::TRANSPARENT };
                        p.rect(r.shrink(1.0), 0.0, nen, Stroke::new(1.0, if bat || tren { XANH } else { Color32::from_gray(0xC8) }), StrokeKind::Inside);
                        if tren && ph.clicked() {
                            bam = Some(v);
                        }
                    }
                    // Nét chữ giả trên tờ
                    for i in 0..4 {
                        let y = giay.top() + 22.0 + i as f32 * 9.0;
                        p.hline(giay.left() + 14.0..=giay.right() - 14.0, y, Stroke::new(1.0, Color32::from_gray(0xD0)));
                    }
                    ph.on_hover_text(if k == KieuIcon::Goc { "Bấm vào từng góc để bật / tắt vùng góc" } else { "Bấm vào từng cạnh để bật / tắt vùng cạnh" });
                }
                KieuIcon::TuyBien => {
                    let nua = giay.height() / 2.0;
                    let tren_r = Rect::from_min_max(giay.min, egui::pos2(giay.right(), giay.top() + nua)).shrink(2.0);
                    let duoi_r = Rect::from_min_max(egui::pos2(giay.left(), giay.top() + nua), giay.max).shrink(2.0);
                    for (r, loai, mau, cong) in [(tren_r, LoaiVung::Xoa, XANH, false), (duoi_r, LoaiVung::BaoVe, HONG, true)] {
                        let dang = self.ve == Some(loai);
                        let tren = tro.is_some_and(|q| r.contains(q));
                        if dang || tren {
                            p.rect_filled(r, 0.0, mau.gamma_multiply(if dang { 0.3 } else { 0.15 }));
                        }
                        let net = Stroke::new(2.0, if dang || tren { mau } else { Color32::from_gray(0x6B) });
                        let c = r.center();
                        p.line_segment([c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)], net);
                        if cong {
                            p.line_segment([c - egui::vec2(0.0, 8.0), c + egui::vec2(0.0, 8.0)], net);
                        }
                        if tren && ph.clicked() {
                            self.ve = if dang { None } else { Some(loai) };
                            if self.ve.is_some() && !matches!(self.loc_trang, LocTrang::TungTrang | LocTrang::VoDoi) {
                                self.loc_trang = LocTrang::TungTrang; // vào chế độ vẽ: mặc định Từng trang (như XoaGhim)
                            }
                            if self.ve != Some(LoaiVung::Xoa) && self.loc_trang == LocTrang::VoDoi {
                                self.loc_trang = LocTrang::TungTrang;
                            }
                        }
                    }
                    p.hline(giay.left() + 4.0..=giay.right() - 4.0, giay.top() + nua, Stroke::new(1.0, Color32::from_gray(0xC8)));
                    if let Some(l) = self.ve {
                        p.rect_stroke(giay.shrink(1.0), 0.0, Stroke::new(2.0, if l == LoaiVung::BaoVe { HONG } else { XANH }), StrokeKind::Inside);
                    }
                    ph.on_hover_text("Nửa trên (−): vẽ vùng xoá · nửa dưới (+): vẽ vùng bảo vệ · kéo chuột trên ảnh Gốc · Esc thôi vẽ");
                }
            }
            if let Some(v) = bam {
                self.bat_tat_co_san(v);
            }
            let ten = match k {
                KieuIcon::Goc => "Góc",
                KieuIcon::Canh => "Cạnh",
                KieuIcon::TuyBien => "Tùy biến",
            };
            ui.add_sized(egui::vec2(CO_ICON.x, 16.0), egui::Label::new(RichText::new(ten).size(12.0).color(CHU_PHU)));
        });
    }

    /// Cột lọc trang. Đang chọn 1 vùng thì Tất cả / Lẻ / Chẵn đổi luôn trang áp dụng của vùng đó.
    fn cot_loc_trang(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let vo_doi = self.ve == Some(LoaiVung::Xoa);
            for (l, ten) in [(LocTrang::TatCa, "Tất cả"), (LocTrang::Le, "Trang lẻ"), (LocTrang::Chan, "Trang chẵn"), (LocTrang::TungTrang, "Từng trang"), (LocTrang::VoDoi, "Vô đối")] {
                if l == LocTrang::VoDoi && !vo_doi {
                    continue;
                }
                let r = ui.radio(self.loc_trang == l, ten).on_hover_text(match l {
                    LocTrang::TatCa => "Áp mọi trang",
                    LocTrang::Le => "Áp trang 1, 3, 5…",
                    LocTrang::Chan => "Áp trang 2, 4, 6…",
                    LocTrang::TungTrang => "Vùng vẽ chỉ áp trang đang vẽ",
                    LocTrang::VoDoi => "Vùng vẽ xoá cả trong vùng bảo vệ (trang đang vẽ)",
                });
                if r.clicked() {
                    self.loc_trang = l;
                    let ap = match l {
                        LocTrang::TatCa => Some(ApTrang::TatCa),
                        LocTrang::Le => Some(ApTrang::Le),
                        LocTrang::Chan => Some(ApTrang::Chan),
                        _ => None,
                    };
                    if let (Some(ap), Some(z)) = (ap, self.bo.vung_chon().cloned()) {
                        if z.ap_trang != ap {
                            self.bo.ghi_buoc();
                            if let Some(v) = self.bo.vung_chon_mut() {
                                v.ap_trang = ap;
                            }
                            self.bo.sua_chon();
                        }
                    }
                }
            }
        });
    }

    /// Tên vùng trong ô chọn: góc / cạnh theo vị trí; vùng vẽ theo loại + số thứ tự.
    fn ten_vung(&self, rieng: bool, i: usize, z: &Vung) -> String {
        if z.vi_tri != ViTri::Tay {
            return z.vi_tri.ten().to_string();
        }
        let loai = match z.loai {
            LoaiVung::Xoa => "Xoá",
            LoaiVung::BaoVe => "Bảo vệ",
            LoaiVung::VoDoi => "Vô đối",
        };
        let trang = match z.ap_trang {
            ApTrang::MotTrang(t) => format!(" · trang {}", t + 1),
            ApTrang::Le => " · trang lẻ".into(),
            ApTrang::Chan => " · trang chẵn".into(),
            ApTrang::TatCa => String::new(),
        };
        format!("{loai} {}{}{trang}", i + 1, if rieng { "" } else { " (chung)" })
    }

    /// Vùng đang chọn + Rộng / Cao (% trang đang xem) + Ngưỡng; Xóa vùng chọn, Hoàn tác, tuỳ chọn bảo vệ.
    fn khoi_vung(&mut self, ui: &mut egui::Ui, rong: f32) {
        ui.vertical(|ui| {
            ui.set_width(rong);
            ui.spacing_mut().item_spacing.y = 3.0;

            let ds: Vec<((bool, usize), String)> = self
                .bo
                .tm
                .vung
                .iter()
                .enumerate()
                .map(|(i, z)| ((false, i), self.ten_vung(false, i, z)))
                .chain(self.bo.vf.vung.iter().enumerate().map(|(i, z)| ((true, i), self.ten_vung(true, i, z))))
                .collect();
            ui.horizontal(|ui| {
                let nhan = ui.add_sized(egui::vec2(52.0, 18.0), egui::Label::new("Vùng:"));
                let chu = self.bo.chon.and_then(|c| ds.iter().find(|(k, _)| *k == c)).map_or("— chọn vùng —".to_string(), |(_, t)| t.clone());
                egui::ComboBox::from_id_salt("xg_vung").width(rong - 60.0).selected_text(chu).show_ui(ui, |ui| {
                    for (k, t) in &ds {
                        if ui.selectable_label(self.bo.chon == Some(*k), t).clicked() {
                            self.bo.chon = Some(*k);
                        }
                    }
                })
                .response
                .labelled_by(nhan.id);
            });
            let (w_pt, h_pt) = self.kho_trang.get(self.trang as usize).copied().unwrap_or((595.0, 842.0));
            let z = self.bo.vung_chon().cloned();
            let mut moi = z.clone();
            let mut bat_dau_keo = false;
            {
                // Trả `true` khi người dùng đổi giá trị (chỉ khi đó mới ghi lại vùng — tránh sai số làm tròn mỗi khung)
                let mut thanh = |ui: &mut egui::Ui, ten: &str, gia_tri: &mut f32, khoang: std::ops::RangeInclusive<f32>, duoc: bool, hau_to: &str| -> bool {
                    ui.horizontal(|ui| {
                        let nhan = ui.add_sized(egui::vec2(52.0, 16.0), egui::Label::new(ten));
                        ui.spacing_mut().slider_width = rong - 120.0;
                        // Thanh trượt kiểu XoaGhim (chỉ trong hàng này): rãnh mảnh, phần đã kéo xanh, nút tròn
                        ui.visuals_mut().selection.bg_fill = XANH;
                        ui.visuals_mut().widgets.inactive.bg_fill = XANH;
                        ui.spacing_mut().slider_rail_height = 4.0;
                        ui.spacing_mut().interact_size.y = 16.0;
                        let r = ui.add_enabled(duoc, egui::Slider::new(gia_tri, khoang).show_value(false).trailing_fill(true).handle_shape(egui::style::HandleShape::Circle)).labelled_by(nhan.id);
                        if duoc {
                            ui.label(if hau_to == "%" { format!("{:.1}%", *gia_tri) } else { format!("{:.0}", *gia_tri) });
                        }
                        bat_dau_keo |= r.drag_started();
                        r.changed()
                    })
                    .inner
                };
                // % trang ↔ kích thước: vùng vẽ theo tỉ lệ; góc / cạnh theo điểm ảnh @120 dpi (1 điểm PDF = 120/72 điểm ảnh)
                let px_pt = DPI_GOC as f32 / 72.0;
                let (mut rong, mut cao, mut nguong) = match &moi {
                    Some(v) if v.vi_tri == ViTri::Tay => (v.rong as f32 * 100.0, v.cao as f32 * 100.0, v.nguong as f32),
                    Some(v) => {
                        let r = if v.rong_px > 0 { v.rong_px as f32 / (w_pt * px_pt) * 100.0 } else { v.rong as f32 * 100.0 };
                        let c = if v.cao_px > 0 { v.cao_px as f32 / (h_pt * px_pt) * 100.0 } else { v.cao as f32 * 100.0 };
                        (r, c, v.nguong as f32)
                    }
                    None => (0.0, 0.0, 5.0),
                };
                let (duoc_rong, duoc_cao) = match moi.as_ref().map(|v| v.vi_tri) {
                    None => (false, false),
                    Some(ViTri::CanhTren | ViTri::CanhDuoi) => (false, true),
                    Some(ViTri::CanhTrai | ViTri::CanhPhai) => (true, false),
                    Some(_) => (true, true),
                };
                let doi_rong = thanh(ui, "Rộng:", &mut rong, 0.5..=100.0, duoc_rong, "%");
                let doi_cao = thanh(ui, "Cao:", &mut cao, 0.5..=100.0, duoc_cao, "%");
                let co_nguong = moi.as_ref().is_some_and(|v| v.loai != LoaiVung::BaoVe);
                let doi_nguong = thanh(ui, "Ngưỡng:", &mut nguong, 1.0..=30.0, co_nguong, "");
                if let Some(v) = moi.as_mut() {
                    if v.vi_tri == ViTri::Tay {
                        if doi_rong {
                            v.rong = (rong as f64 / 100.0).min(1.0 - v.x);
                        }
                        if doi_cao {
                            v.cao = (cao as f64 / 100.0).min(1.0 - v.y);
                        }
                    } else {
                        if doi_rong && v.rong_px > 0 {
                            v.rong_px = ((rong / 100.0) * w_pt * px_pt).round().max(5.0) as u32;
                        }
                        if doi_cao && v.cao_px > 0 {
                            v.cao_px = ((cao / 100.0) * h_pt * px_pt).round().max(5.0) as u32;
                        }
                    }
                    if doi_nguong {
                        v.nguong = nguong.round() as u8;
                    }
                }
            }
            if bat_dau_keo {
                self.bo.ghi_buoc();
            }
            if moi != z {
                if let (Some(v), Some(moi)) = (self.bo.vung_chon_mut(), moi) {
                    *v = moi;
                }
                self.bo.sua_chon();
            }
            ui.horizontal(|ui| {
                let nut_xoa = egui::Button::new(RichText::new("Xóa vùng chọn").size(12.0).color(XANH_COBALT)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(0xB6, 0xCC, 0xEA)));
                if ui.add_enabled(self.bo.chon.is_some(), nut_xoa).on_hover_text("Delete").clicked() {
                    self.bo.xoa_chon();
                }
                ui.menu_button("Xoá hết…", |ui| {
                    if ui.button("Vùng vẽ chung của thư mục").clicked() {
                        self.bo.xoa_het_ve(false);
                        ui.close();
                    }
                    if ui.add_enabled(!self.bo.vf.vung.is_empty(), egui::Button::new("Vùng riêng của file này")).clicked() {
                        self.bo.xoa_het_ve(true);
                        ui.close();
                    }
                });
                let ht = ui.add_enabled(self.bo.co_hoan_tac(), egui::Button::new(atom_mau(BieuTuong::HoanTac, 13.0, CHU)));
                ht.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Hoàn tác"));
                if ht.on_hover_text("Hoàn tác (Ctrl+Z)").clicked() {
                    self.bo.hoan_tac();
                }
                let tc = &mut self.bo.tm.tuy_chon;
                let mut doi = ui.checkbox(&mut tc.bao_ve_net_den, "Bảo vệ nét đen").on_hover_text("Không lấy điểm rất tối làm vết (giữ nét chữ đen)").changed();
                doi |= ui.checkbox(&mut tc.bao_ve_do_xanh, "Bảo vệ dấu đỏ / chữ ký xanh").changed();
                if doi {
                    self.bo.doi_chung();
                }
            });
        });
    }

    /// DPI, Nén · Thư mục kết quả · File đích · Nền trắng, Nén đen trắng. `gon` = 1 dòng (cửa sổ hẹp), không thì lưới.
    fn khoi_xuat(&mut self, ui: &mut egui::Ui, gon: bool) {
        let mut doi = false;
        let ra = self.thu_muc.as_deref().map(super::thu_muc_ket_qua);
        let chu_ra = match &ra {
            Some(Ok(p)) => p.display().to_string(),
            Some(Err(e)) => e.clone(),
            None => "<thư mục nguồn>_xoaghim".into(),
        };
        let co_ra = matches!(&ra, Some(Ok(p)) if p.is_dir());
        let dpi_nen = |ui: &mut egui::Ui, x: &mut crate::xoa_ghim::kho::CauHinhXuat, doi: &mut bool, nhan_dpi: egui::Response| {
            let ten_dpi = |d: u16| if d == 0 { "Gốc".to_string() } else { format!("{d}") };
            egui::ComboBox::from_id_salt("xg_dpi").width(70.0).selected_text(ten_dpi(x.dpi)).show_ui(ui, |ui| {
                for d in CAC_DPI {
                    if ui.selectable_label(x.dpi == d, ten_dpi(d)).on_hover_text(if d == 0 { "Theo ảnh scan của từng trang" } else { "" }).clicked() && x.dpi != d {
                        x.dpi = d;
                        *doi = true;
                    }
                }
            })
            .response
            .labelled_by(nhan_dpi.id);
            let nhan_nen = ui.label("Nén:");
            egui::ComboBox::from_id_salt("xg_nen").width(64.0).selected_text(format!("{}%", x.chat_luong)).show_ui(ui, |ui| {
                for c in CAC_CHAT_LUONG {
                    if ui.selectable_label(x.chat_luong == c, format!("{c}%")).clicked() && x.chat_luong != c {
                        x.chat_luong = c;
                        *doi = true;
                    }
                }
            })
            .response
            .labelled_by(nhan_nen.id);
            *doi |= ui.checkbox(&mut x.nen_den_trang, "Nén đen trắng").on_hover_text("Trang đen trắng / xám ghi gọn hơn").changed();
        };
        let thu_muc = |ui: &mut egui::Ui, rong: f32| {
            egui::Frame::new().stroke(Stroke::new(1.0, VIEN)).corner_radius(CornerRadius::same(2)).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
                ui.set_width(rong);
                ui.add(egui::Label::new(RichText::new(&chu_ra).size(12.0)).truncate()).on_hover_text(&chu_ra);
            });
            if ui.add_enabled(co_ra, egui::Button::new("Mở")).on_hover_text("Mở thư mục kết quả").on_disabled_hover_text("Chưa có kết quả").clicked() {
                if let Some(Ok(p)) = &ra {
                    crate::gui::mo_bang_he_dieu_hanh(p);
                }
            }
        };
        let tuy_chon = |ui: &mut egui::Ui, tm: &mut crate::xoa_ghim::kho::ThuMuc, doi: &mut bool| {
            *doi |= ui.checkbox(&mut tm.tuy_chon.nen_trang, "Nền trắng").on_hover_text("Tô vết bằng trắng thay cho màu nền đo được").changed();
        };
        if gon {
            ui.horizontal(|ui| {
                let nhan = ui.label("DPI:");
                dpi_nen(ui, &mut self.bo.tm.xuat, &mut doi, nhan);
                ke_doc(ui, 20.0);
                ui.label("Thư mục:");
                thu_muc(ui, 260.0);
                ke_doc(ui, 20.0);
                tuy_chon(ui, &mut self.bo.tm, &mut doi);
            });
        } else {
            egui::Grid::new("xg_xuat").num_columns(2).spacing(egui::vec2(10.0, 7.0)).show(ui, |ui| {
                let nhan = ui.label("DPI:");
                ui.horizontal(|ui| dpi_nen(ui, &mut self.bo.tm.xuat, &mut doi, nhan));
                ui.end_row();
                ui.label("Thư mục:");
                ui.horizontal(|ui| thu_muc(ui, 300.0));
                ui.end_row();
                ui.label("File đích:");
                egui::Frame::new().stroke(Stroke::new(1.0, VIEN)).corner_radius(CornerRadius::same(2)).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.label(RichText::new("{gốc}.pdf — giữ tên file gốc, giữ thư mục con").size(12.0).color(CHU_PHU));
                });
                ui.end_row();
                ui.label("Tùy chọn:");
                ui.horizontal(|ui| tuy_chon(ui, &mut self.bo.tm, &mut doi));
                ui.end_row();
            });
        }
        if doi {
            self.bo.doi_chung();
        }
        if let Some(e) = &self.loi {
            ui.label(RichText::new(e).color(DO).size(11.5));
        }
    }

    /// Nút xanh "Xóa ghim" (file đã chọn chưa xoá / đổi cấu hình / mất kết quả) + ⌄ chạy lại mọi file đã chọn.
    fn nut_chay(&mut self, ui: &mut egui::Ui) {
        let can = self.can_chay().len();
        let dang_doc = self.tinh_trang.is_empty() && !self.ds.is_empty();
        let (menu, _) = egui::containers::menu::MenuButton::from_button(egui::Button::new(atom_mau(BieuTuong::ChevronXuong, 13.0, Color32::WHITE)).fill(crate::gui::XANH_COBALT).min_size(egui::vec2(24.0, 26.0))).ui(ui, |ui| {
            let chon = self.chon_chay.iter().filter(|&&c| c).count();
            if ui.add_enabled(chon > 0, egui::Button::new(format!("Chạy lại mọi file đã chọn ({chon})"))).clicked() {
                ui.close();
                self.bam_chay(true);
            }
        });
        menu.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Tuỳ chọn chạy"));
        let chu = if dang_doc { "Xóa ghim".to_string() } else { format!("Xóa ghim ({can})") };
        let nut = egui::Button::new(RichText::new(chu).strong().size(13.0).color(Color32::WHITE)).fill(crate::gui::XANH_COBALT).min_size(egui::vec2(110.0, 26.0));
        let r = ui
            .add_enabled(!dang_doc && can > 0, nut)
            .on_hover_text("Xoá ghim các file đã đánh dấu chưa xoá, đã đổi vùng / tuỳ chọn hoặc mất kết quả; ghi vào thư mục <tên nguồn>_xoaghim")
            .on_disabled_hover_text(if self.ds.is_empty() { "Chọn thư mục PDF trước" } else if dang_doc { "Đang đọc trạng thái các file…" } else { "Mọi file đã chọn đã xoá ghim đúng vùng / tuỳ chọn hiện tại" });
        if r.clicked() {
            self.bam_chay(false);
        }
    }
}
