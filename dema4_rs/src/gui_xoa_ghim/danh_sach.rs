//! Cột trái **Danh sách** (như prototype): mặc định mở rộng; thu nhỏ thành dải dọc 34 px (chữ "Danh sách" xoay dọc +
//! số file); kéo mép phải đổi rộng 200–520 (bấm đúp: 282). Tiêu đề "Danh sách (tick/tổng)" + "đang hiện m"; hàng sắp
//! xếp Tên file / Trang (1 cột mỗi lần); ô lọc tên + ô lọc gộp (trạng thái hoặc gõ số trang); dòng file: ô tick ·
//! tên · biểu tượng trạng thái · số trang; **Lọc nâng cao** (khổ / chiều / dải trang) mở mặc định.

use eframe::egui::{self, Color32, Rect, Sense, Stroke};

use super::thiet_ke as tk;
use super::widget::{self, font, font_dam};
use super::{bieu_tuong, khop_nang_cao, khop_so, LocNangCao, LocTrangThai, ManXoaGhim, SapTheo};
use crate::xoa_ghim::xuat::TrangThai;

const KHO: [&str; 8] = ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7"];

impl ManXoaGhim {
    /// Đang mở rộng: chưa mở file / thư mục → thu nhỏ; đã mở → mở rộng. Người dùng bấm thu / mở thì giữ tới khi mở
    /// file / thư mục khác hoặc đóng.
    fn ds_mo_rong(&self) -> bool {
        self.ds_mo.unwrap_or(self.thu_muc.is_some())
    }

    /// Thứ tự hiện: lọc tên (NFC, không phân biệt hoa thường) + lọc gộp + Lọc nâng cao; sắp tên tự nhiên / số trang.
    pub(super) fn thu_tu_hien(&self) -> Vec<usize> {
        let goc = self.thu_muc.clone().unwrap_or_default();
        let ten = |i: usize| self.ds[i].0.strip_prefix(&goc).unwrap_or(&self.ds[i].0).to_string_lossy().into_owned();
        let loc = crate::tach_vb::nhan::nfc(&self.loc_ten.trim().to_lowercase());
        let so = khop_so(&self.loc_so);
        let nc = self.loc_nc_ap.as_ref();
        let mut ds: Vec<usize> = (0..self.ds.len())
            .filter(|&i| loc.is_empty() || crate::tach_vb::nhan::nfc(&ten(i).to_lowercase()).contains(&loc))
            .filter(|&i| match &so {
                Some(f) => self.thong_tin.get(i).and_then(Option::as_ref).is_some_and(|t| f(t.so_trang)),
                None => self.loc_so.trim().is_empty() && self.khop_trang_thai(i),
            })
            .filter(|&i| nc.is_none_or(|l| self.thong_tin.get(i).and_then(Option::as_ref).is_some_and(|t| khop_nang_cao(t, l, &self.quy_tac))))
            .collect();
        match self.sap_theo {
            SapTheo::Ten => ds.sort_by(|&a, &b| crate::tach_vb::phien::so_ten_tu_nhien(&ten(a), &ten(b))),
            SapTheo::Trang => ds.sort_by_key(|&i| (self.thong_tin.get(i).and_then(Option::as_ref).map_or(0, |t| t.so_trang), i)),
        }
        if self.sap_giam {
            ds.reverse();
        }
        ds
    }

    fn khop_trang_thai(&self, i: usize) -> bool {
        let tt = self.trang_thai_file(i);
        match self.loc_tt {
            LocTrangThai::TatCa => true,
            LocTrangThai::CanChay => !matches!(tt, Some(TrangThai::DaXoa { dung_cau_hinh: true })),
            LocTrangThai::ChuaXoa => matches!(tt, Some(TrangThai::ChuaXoa) | None),
            LocTrangThai::DaXoa => matches!(tt, Some(TrangThai::DaXoa { dung_cau_hinh: true })),
            LocTrangThai::DaDoi => matches!(tt, Some(TrangThai::DaXoa { dung_cau_hinh: false })),
            LocTrangThai::MatKetQua => matches!(tt, Some(TrangThai::KetQuaMat)),
        }
    }

    pub(super) fn danh_sach(&mut self, ui: &mut egui::Ui) {
        if !self.ds_mo_rong() {
            self.dai_thu_nho(ui);
            return;
        }
        let rong = self.cfg.rong_ds.clamp(tk::DS_RONG_MIN, tk::DS_RONG_MAX);
        // Kéo mép phải đổi bề rộng: dùng tay nắm có sẵn của Panel (nằm trên màn trang, không bị vùng trang che mất)
        let kq = egui::Panel::left("xg_ds").resizable(true).default_size(rong).size_range(tk::DS_RONG_MIN..=tk::DS_RONG_MAX).frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, tk::VIEN_KHUNG))).show(ui, |ui_panel| {
            // Panel lấy bề rộng theo nội dung → nội dung đã trải hết bề rộng thì kéo nhỏ không được. Vẽ trong ô con
            // giới hạn đúng bề rộng panel (cắt phần thừa) để kéo nhỏ được tới mức tối thiểu.
            let o_ds = ui_panel.max_rect();
            let mut con = ui_panel.new_child(egui::UiBuilder::new().max_rect(o_ds).layout(egui::Layout::top_down(egui::Align::Min)));
            con.set_clip_rect(o_ds.intersect(ui_panel.clip_rect()));
            ui_panel.expand_to_include_rect(o_ds);
            let ui = &mut con;
            // Các hàng (tiêu đề 32, sắp xếp 28, lọc, danh sách) nối liền như prototype
            ui.spacing_mut().item_spacing.y = 0.0;
            self.tieu_de_ds(ui);
            if self.thu_muc.is_none() {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| ui.label(egui::RichText::new("Chưa mở file hoặc thư mục.").font(font(tk::CHU_CO)).color(tk::CHU_PHU)));
            } else {
                self.hang_sap_xep(ui);
                self.hang_loc(ui);
                // Khối Lọc nâng cao đo ở khung trước → danh sách chừa đúng chỗ, nút Áp dụng không bị cắt
                let cao_ds = (ui.available_height() - self.cao_nc).max(40.0);
                ui.allocate_ui(egui::vec2(ui.available_width(), cao_ds), |ui| self.cac_dong(ui));
                // Đo đúng phần khối chiếm (scope riêng) — min_rect của cả cột luôn chạm đáy nên không co lại được
                let cao = ui.scope(|ui| self.loc_nang_cao(ui)).response.rect.height();
                if (cao - self.cao_nc).abs() > 0.5 {
                    self.cao_nc = cao;
                    ui.ctx().request_repaint();
                }
            }
        });
        // Ghi bề rộng vào cấu hình khi thả chuột (đang kéo chỉ nhớ tạm)
        let w = kq.response.rect.width();
        if ui.input(|i| i.pointer.any_down()) {
            if (w - self.cfg.rong_ds).abs() > 0.5 {
                self.rong_keo = Some(w);
            }
        } else if let Some(w) = self.rong_keo.take() {
            self.doi_cfg(|c| c.rong_ds = w);
        }
    }

    /// Dải thu nhỏ: ▸| · "Danh sách" dọc · số file; bấm bất kỳ đâu → mở rộng.
    fn dai_thu_nho(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("xg_ds_nho").resizable(false).exact_size(tk::DS_DAI_THU_NHO).frame(egui::Frame::new().fill(tk::NEN_DS_TIEU_DE).stroke(Stroke::new(1.0, tk::VIEN_KHUNG))).show(ui, |ui| {
            let o = ui.max_rect();
            let ph = ui.interact(o, ui.scope_id().with("dai_thu_nho"), Sense::click());
            let p = ui.painter();
            if ph.hovered() {
                p.rect_filled(o, 0.0, tk::NEN_NUT_HOVER);
            }
            let tren = egui::pos2(o.center().x, o.top() + 6.0 + 11.0);
            widget::ve_mui_ten(p, tren, widget::Huong::Phai, if ph.hovered() { tk::XANH } else { Color32::from_rgb(0x37, 0x41, 0x51) });
            let g = p.layout_no_wrap("Danh sách".to_owned(), font(tk::CHU_CO), Color32::from_rgb(0x37, 0x41, 0x51));
            let (gw, gh) = (g.size().x, g.size().y);
            // Xoay −90°: đọc từ dưới lên; đỉnh chữ nằm trên đáy khung chữ
            let goc = egui::pos2(o.center().x - gh / 2.0, tren.y + 21.0 + gw);
            p.add(egui::epaint::TextShape::new(goc, g, Color32::from_rgb(0x37, 0x41, 0x51)).with_angle(-std::f32::consts::FRAC_PI_2));
            if self.thu_muc.is_some() {
                let so = self.ds.len().to_string();
                let gs = p.layout_no_wrap(so, font(10.5), Color32::WHITE);
                let r = Rect::from_center_size(egui::pos2(o.center().x, tren.y + 21.0 + gw + 10.0 + 8.0), egui::vec2(gs.size().x + 10.0, 15.0));
                p.rect_filled(r, 8.0, tk::XANH);
                p.galley(r.center() - gs.size() / 2.0, gs, Color32::WHITE);
            }
            ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Mở rộng danh sách"));
            if ph.on_hover_text("Mở rộng danh sách").clicked() {
                self.ds_mo = Some(true);
            }
        });
    }

    fn tieu_de_ds(&mut self, ui: &mut egui::Ui) {
        let (o, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), tk::DS_CAO_TIEU_DE), Sense::hover());
        ui.painter().rect_filled(o, 0.0, tk::NEN_DS_TIEU_DE);
        ui.painter().hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(0xE3, 0xE5, 0xE8)));
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o.shrink2(egui::vec2(10.0, 0.0)).with_max_x(o.right() - 6.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let tieu_de = if self.thu_muc.is_none() {
            "Danh sách".to_string()
        } else if self.mot_file.is_some() {
            "Danh sách (1/1)".to_string()
        } else {
            format!("Danh sách ({}/{})", self.chon_chay.iter().filter(|&&c| c).count(), self.ds.len())
        };
        con.label(egui::RichText::new(tieu_de).font(font(tk::CHU_CO)).color(tk::CHU));
        con.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let r = widget::nut_mui_ten(ui, widget::Huong::Trai, 22.0, 22.0, "Thu nhỏ danh sách");
            if r.on_hover_text("Thu nhỏ danh sách").clicked() {
                self.ds_mo = Some(false);
            }
            if self.thu_muc.is_some() && self.mot_file.is_none() {
                let hien = self.thu_tu_hien().len();
                if hien < self.ds.len() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(format!("đang hiện {hien}")).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
                }
            }
        });
    }

    fn hang_sap_xep(&mut self, ui: &mut egui::Ui) {
        let (o, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), tk::DS_CAO_HANG_SAP), Sense::hover());
        ui.painter().rect_filled(o, 0.0, tk::NEN_HANG_SAP);
        ui.painter().hline(o.x_range(), o.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(0xEC, 0xEE, 0xF1)));
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o.shrink2(egui::vec2(10.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let mut tat_ca = !self.chon_chay.is_empty() && self.chon_chay.iter().all(|&c| c);
        let r = widget::tick(&mut con, &mut tat_ca, "");
        r.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, tat_ca, "Chọn mọi file"));
        if r.on_hover_text("Chọn / bỏ chọn mọi file").changed() {
            self.chon_chay.iter_mut().for_each(|c| *c = tat_ca);
        }
        let mut nut_sap = |ui: &mut egui::Ui, theo: SapTheo, ten: &str| {
            let dang = self.sap_theo == theo;
            let chu = if dang { format!("{ten} {}", if self.sap_giam { "↓" } else { "↑" }) } else { ten.to_string() };
            let r = ui.add(egui::Label::new(egui::RichText::new(chu).font(font_dam(tk::CHU_CO)).color(if dang { tk::XANH_DAM_CHU } else { tk::CHU })).sense(Sense::click()));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Sắp theo {ten}")));
            if r.on_hover_text(format!("Sắp theo {} — bấm lại để đảo chiều", ten.to_lowercase())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if dang {
                    self.sap_giam = !self.sap_giam;
                } else {
                    self.sap_theo = theo;
                    self.sap_giam = false;
                }
            }
        };
        nut_sap(&mut con, SapTheo::Ten, "Tên file");
        con.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| nut_sap(ui, SapTheo::Trang, "Trang"));
    }

    /// Ô lọc tên + ô lọc gộp (chọn trạng thái ▾ hoặc gõ số trang).
    fn hang_loc(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            let rong_gop = 118.0;
            let rong_ten = (ui.available_width() - rong_gop - 14.0).max(60.0);
            let te = ui.add(
                egui::TextEdit::singleline(&mut self.loc_ten)
                    .hint_text("Lọc tên file…")
                    .desired_width(rong_ten)
                    .min_size(egui::vec2(rong_ten, tk::CAO_NUT))
                    .align(egui::Align2::LEFT_CENTER)
                    .margin(egui::Margin { left: 24, right: 6, top: 0, bottom: 0 }),
            );
            bieu_tuong::ve(ui, "kinh_lup", Rect::from_center_size(egui::pos2(te.rect.left() + 13.0, te.rect.center().y), egui::vec2(12.0, 12.0)));
            te.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Lọc tên file"));
            // Ô lọc gộp: chữ gõ = số trang; trống = hiện tên trạng thái làm chữ mờ đậm
            let o = ui.allocate_ui(egui::vec2(rong_gop, tk::CAO_NUT), |ui| {
                let te = ui.add(
                    egui::TextEdit::singleline(&mut self.loc_so)
                        .hint_text(egui::RichText::new(self.loc_tt.ten()).color(tk::CHU))
                        .desired_width(rong_gop - 22.0)
                        .min_size(egui::vec2(rong_gop, tk::CAO_NUT))
                        .align(egui::Align2::LEFT_CENTER)
                        .margin(egui::Margin { left: 8, right: 22, top: 0, bottom: 0 }),
                );
                te.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Lọc trạng thái / số trang"));
                te.on_hover_text("Chọn trạng thái, hoặc gõ số trang: 1-3, 7, >10, <5");
            });
            let r_o = o.response.rect;
            let nut = Rect::from_min_size(egui::pos2(r_o.right() - 21.0, r_o.top() + 1.0), egui::vec2(20.0, r_o.height() - 2.0));
            let mut con = ui.new_child(egui::UiBuilder::new().max_rect(nut));
            let (menu, _) = egui::containers::menu::MenuButton::from_button(egui::Button::image(bieu_tuong::anh("caret", 8.0, 8.0)).frame(false)).ui(&mut con, |ui| {
                ui.set_min_width(200.0);
                for l in LocTrangThai::DS {
                    let dang = self.loc_tt == l && self.loc_so.trim().is_empty();
                    // Mục đang lọc: nền chọn + dấu tích vẽ (phông màn Xóa ghim không có ✓)
                    let anh = bieu_tuong::anh("m_chon", 12.0, 12.0).tint(if dang { Color32::WHITE } else { Color32::TRANSPARENT });
                    if ui.add(egui::Button::image_and_text(anh, egui::RichText::new(l.ten()).font(font(tk::CHU_CO)).color(if dang { tk::XANH_DAM_CHU } else { tk::CHU })).selected(dang).frame(false)).clicked() {
                        self.loc_tt = l;
                        self.loc_so.clear();
                        ui.close();
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("Hoặc gõ số trang vào ô:\n1-3, 7, >10, <5").font(font(tk::CHU_NHO)).color(tk::CHU_PHU));
            });
            menu.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Mở danh sách lọc"));
        });
        ui.add_space(6.0);
        let y = ui.cursor().top();
        ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, Color32::from_rgb(0xEC, 0xEE, 0xF1)));
    }

    fn cac_dong(&mut self, ui: &mut egui::Ui) {
        let goc = self.thu_muc.clone().unwrap_or_default();
        let hien = self.thu_tu_hien();
        let mut chon = None;
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical().id_salt("xg_ds_cuon").auto_shrink([false, false]).show_rows(ui, tk::DS_CAO_DONG, hien.len().max(1), |ui, hang| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if hien.is_empty() {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Không có file khớp bộ lọc.").font(font(tk::CHU_CO)).color(tk::CHU_PHU));
                });
                return;
            }
            for i in hang.filter_map(|k| hien.get(k).copied()) {
                let (o, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), tk::DS_CAO_DONG), Sense::click());
                let p = ui.painter().clone();
                if i == self.chon {
                    p.rect_filled(o, 0.0, tk::NEN_DONG_CHON);
                } else if ph.hovered() {
                    p.rect_filled(o, 0.0, tk::NEN_DONG_HOVER);
                }
                // Kẻ đứt dưới dòng
                p.add(egui::Shape::dashed_line(&[o.left_bottom() + egui::vec2(0.0, -0.5), o.right_bottom() + egui::vec2(0.0, -0.5)], Stroke::new(1.0, tk::VIEN_DONG), 3.0, 2.0));
                let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o.shrink2(egui::vec2(10.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                con.spacing_mut().item_spacing.x = 8.0;
                let ten_file = self.ds[i].0.file_name().map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
                if let Some(c) = self.chon_chay.get_mut(i) {
                    widget::tick(&mut con, c, "").widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, *c, format!("Chọn {ten_file}")));
                }
                let ten = self.ds[i].0.strip_prefix(&goc).unwrap_or(&self.ds[i].0).display().to_string();
                let so = self.thong_tin.get(i).and_then(Option::as_ref).map_or(String::new(), |t| t.so_trang.to_string());
                let (bt, goi_y) = match self.trang_thai_file(i) {
                    None | Some(TrangThai::ChuaXoa) => ("tt_chua", "Chưa xoá ghim"),
                    Some(TrangThai::DaXoa { dung_cau_hinh: true }) => ("tt_xong", "Đã xoá ghim — đúng vùng / tuỳ chọn hiện tại"),
                    Some(TrangThai::DaXoa { dung_cau_hinh: false }) => ("tt_doi", "Đã xoá nhưng vùng / tuỳ chọn đã đổi — sẽ chạy lại"),
                    Some(TrangThai::KetQuaMat) => ("tt_mat", "File kết quả không còn / bị thay — sẽ chạy lại"),
                };
                // Cột phải cố định: số trang canh phải (26) · trạng thái (14) — như prototype
                let x_phai = o.right() - 10.0;
                let o_tt = Rect::from_center_size(egui::pos2(x_phai - 26.0 - 12.0, o.center().y), egui::vec2(14.0, 14.0));
                let mau_ten = if i == self.chon { tk::XANH_DAM_CHU } else { tk::CHU };
                let x_ten = con.cursor().min.x;
                let o_ten = Rect::from_min_max(egui::pos2(x_ten, o.top()), egui::pos2(o_tt.left() - 8.0, o.bottom()));
                let mut ui_ten = con.new_child(egui::UiBuilder::new().max_rect(o_ten).layout(egui::Layout::left_to_right(egui::Align::Center)));
                ui_ten.add(egui::Label::new(egui::RichText::new(&ten).font(font(tk::CHU_CO)).color(mau_ten)).truncate().selectable(false));
                bieu_tuong::ve(&con, bt, Rect::from_center_size(o_tt.center(), egui::vec2(11.0, 11.0)));
                con.interact(o_tt, con.scope_id().with(("xg_tt", i)), Sense::hover()).on_hover_text(goi_y);
                p.text(egui::pos2(x_phai, o.center().y), egui::Align2::RIGHT_CENTER, so, font(tk::CHU_CO), tk::CHU_PHU);
                if ph.clicked() {
                    chon = Some(i);
                }
            }
        });
        if self.bo.tm.ghi.kieu == crate::xoa_ghim::kho::KieuGhi::CanhFile && self.mot_file.is_none() {
            ui.label(egui::RichText::new("   Đã ẩn các file kết quả *_xoaghim.pdf nằm cạnh file gốc.").font(font(tk::CHU_NHO)).color(tk::CHU_PHU));
        }
        if let Some(i) = chon {
            self.chon_file(i);
        }
    }

    /// Khối Lọc nâng cao ở đáy cột (mở mặc định): chip khổ / chiều kèm số file, ô dải trang, dòng số file đang hiện.
    fn loc_nang_cao(&mut self, ui: &mut egui::Ui) {
        let o = ui.available_rect_before_wrap();
        ui.painter().rect_filled(o, 0.0, tk::NEN_HANG_SAP);
        ui.painter().hline(o.x_range(), o.top(), Stroke::new(1.0, tk::VIEN_KHUNG));
        ui.horizontal(|ui| {
            ui.set_height(30.0);
            ui.add_space(12.0);
            // Áp tự động: tick đổi → áp ngay; gõ dải trang → áp sau 2 s
            if self.han_loc_nc.is_some_and(|t| ui.input(|i| i.time) >= t) {
                self.ap_loc_nc();
            }
            // Tiêu đề "Lọc nâng cao (x)": x = số file đang hiện sau mọi bộ lọc (xanh khi đang lọc nâng cao)
            let dang_loc = self.loc_nc_ap.is_some();
            let con = self.thu_tu_hien().len();
            let mut job = egui::text::LayoutJob::default();
            job.append("Lọc nâng cao", 0.0, egui::TextFormat::simple(font_dam(tk::CHU_CO), tk::CHU));
            job.append(&format!(" ({con})"), 0.0, egui::TextFormat::simple(font(tk::CHU_CO), if dang_loc { tk::XANH_DAM_CHU } else { tk::CHU_PHU }));
            let r = ui.add(egui::Label::new(job).sense(Sense::click()));
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.loc_nc_mo = !self.loc_nc_mo;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                let (o, ph) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
                let mau = if ph.hovered() { tk::XANH } else { Color32::from_rgb(0x37, 0x41, 0x51) };
                widget::ve_mui_ten(ui.painter(), o.center(), if self.loc_nc_mo { widget::Huong::Xuong } else { widget::Huong::Len }, mau);
                ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Thu gọn / mở lọc nâng cao"));
                if ph.on_hover_text(if self.loc_nc_mo { "Thu gọn" } else { "Mở rộng" }).clicked() {
                    self.loc_nc_mo = !self.loc_nc_mo;
                }
                // Reset: về chọn hết, bỏ lọc
                if self.loc_nc_mo {
                    ui.add_space(4.0);
                    let r = ui.add(egui::Label::new(egui::RichText::new("Reset").font(font(tk::CHU_CO)).color(tk::XANH).underline()).sense(Sense::click()));
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Reset"));
                    if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Chọn lại tất cả, xoá dải trang — bỏ lọc").clicked() {
                        self.loc_nc = LocNangCao::default();
                        self.ap_loc_nc();
                    }
                }
            });
        });
        if !self.loc_nc_mo {
            return;
        }
        // Khổ / chiều không có trang nào trong các file đang mở → ô mờ, không bấm được (khổ xếp theo quy tắc Đếm trang A4)
        let (co_kho, co_ngang, co_doc) = super::co_mat(&self.thong_tin, &self.quy_tac);
        let truoc = self.loc_nc.clone();
        egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 12, top: 2, bottom: 10 }).show(ui, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let phu = |ui: &mut egui::Ui, t: &str| ui.label(egui::RichText::new(t).font(font(tk::CHU_CO)).color(tk::CHU_PHU));
                // Các ô xếp trái → phải, hết chỗ thì xuống dòng
                phu(ui, "Khổ giấy:");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(12.0, 4.0);
                    for (i, k) in KHO.iter().enumerate() {
                        let mut b = self.loc_nc.kho[i] && co_kho[i];
                        let mau = if b { tk::CHU } else { tk::CHU_MO };
                        let r = o_tick(ui, co_kho[i], k, |ui| widget::tick(ui, &mut b, egui::RichText::new(*k).font(font(tk::CHU_CO)).color(mau)));
                        if !co_kho[i] {
                            r.on_disabled_hover_text(format!("Không có trang {k} trong các file đang mở"));
                        } else if r.changed() {
                            self.loc_nc.kho[i] = b;
                        }
                    }
                });
                phu(ui, "Hướng trang:");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(12.0, 4.0);
                    for (co, gt, ten) in [(co_doc, &mut self.loc_nc.doc, "Dọc"), (co_ngang, &mut self.loc_nc.ngang, "Ngang")] {
                        let mut b = *gt && co;
                        let r = o_tick(ui, co, ten, |ui| widget::tick(ui, &mut b, egui::RichText::new(ten).font(font(tk::CHU_CO))));
                        if !co {
                            r.on_disabled_hover_text(format!("Không có trang {} trong các file đang mở", ten.to_lowercase()));
                        } else if r.changed() {
                            *gt = b;
                        }
                    }
                });
                phu(ui, "Trong dải trang:");
                let o = ui.add(egui::TextEdit::singleline(&mut self.loc_nc.dai).hint_text("tất cả (vd: 1-5, 7, 10-15)").desired_width(f32::INFINITY).min_size(egui::vec2(0.0, tk::CAO_NUT)).align(egui::Align2::LEFT_CENTER));
                o.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Trong dải trang"));
                // Enter / rời ô: áp ngay, không chờ 2 s
                if o.lost_focus() && self.han_loc_nc.is_some() {
                    self.han_loc_nc = Some(0.0);
                }
            });
        });
        if self.loc_nc.dai != truoc.dai {
            let t = ui.input(|i| i.time) + 2.0;
            self.han_loc_nc = Some(t);
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f32(2.05));
        } else if (self.loc_nc.kho, self.loc_nc.ngang, self.loc_nc.doc) != (truoc.kho, truoc.ngang, truoc.doc) {
            self.ap_loc_nc();
        }
    }

    /// Áp điều kiện đang nhập; trùng mặc định (chọn hết, không dải trang) = không lọc.
    fn ap_loc_nc(&mut self) {
        self.han_loc_nc = None;
        self.loc_nc_ap = (self.loc_nc != LocNangCao::default()).then(|| self.loc_nc.clone());
    }

}

/// 1 ô tick trong hàng tự xuống dòng: xin trước đúng bề rộng ô (hộp 14 + 6 + chữ) để hết chỗ thì xuống dòng
/// (ô con không biết cỡ trước thì `horizontal_wrapped` không xuống dòng được); `duoc` = false → mờ.
fn o_tick(ui: &mut egui::Ui, duoc: bool, chu: &str, ve: impl FnOnce(&mut egui::Ui) -> egui::Response) -> egui::Response {
    let rong = 20.0 + ui.painter().layout_no_wrap(chu.to_owned(), font(tk::CHU_CO), Color32::BLACK).size().x;
    ui.allocate_ui(egui::vec2(rong, 20.0), |ui| {
        if !duoc {
            ui.disable();
        }
        ve(ui)
    })
    .inner
}
