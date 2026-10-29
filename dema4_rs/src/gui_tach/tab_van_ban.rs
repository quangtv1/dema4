//! Tab Văn bản: xem trước từng văn bản sẽ tách (đúng trang của file part), điều hướng liên tục qua mọi hồ sơ,
//! 3 chế độ xem (Lưới · 1 trang vừa chiều ngang · 2 trang liên tục), chọn trang kiểu Explorer, xoay / bỏ khỏi văn bản.

use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use egui_extras::{Column, TableBuilder};

use super::anh_nho::{self, bac_cho, KhoaAnh, YeuCau};
use super::bieu_tuong::{self, atom, nut_phang, BieuTuong};
use super::chon::{BoCucLuoi, PhimDiChuyen, PhimPhu};
use super::van_ban::{self, MucVanBan};
use super::xem_trang::{self, TY_LE};
use super::{bat_dau_bang, chuot_luoi, ke_duoi, ngan, o_so, ChuotLuoi, ManTach, Tab, Viec, CAM, NEN_ANH_CHO, NEN_CHON, NEN_HOVER, VACH_DONG, VIEN_CHON};
use crate::cau_hinh::{CauHinh, CheDoVb};
use crate::gui::{so_nguyen, ChuDam, CHU, CHU_PHU, VIEN_NHAT};

const CAO_CHU_VB: f32 = 18.0;
/// Nền vùng xem 1 trang / 2 trang (trang giấy trắng đổ bóng nổi trên nền xám).
const NEN_XEM_TRANG: Color32 = Color32::from_rgb(0xE9, 0xE9, 0xE9);
const NEN_DIEU_HUONG: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA);

/// Chỉ số tỷ lệ gần nhất với giá trị % đã lưu.
pub fn nac_ty_le(ty_le: u16) -> usize {
    (0..TY_LE.len()).min_by_key(|&i| (TY_LE[i] as i32 - ty_le as i32).abs()).unwrap_or(2)
}

impl ManTach {
    /// Tính lại danh sách văn bản khi nhãn đổi (đang phân tích: tối đa 2 lần/giây để không ì giao diện).
    pub(super) fn cap_nhat_ds_vb(&mut self) {
        let Some(b) = &self.bang else {
            self.ds_vb.clear();
            return;
        };
        if self.khoa_ds_vb == Some(b.phien_ban) {
            return;
        }
        if self.chay.is_some() && self.luc_tinh_vb.is_some_and(|t| t.elapsed() < Duration::from_millis(500)) {
            return;
        }
        let moi = van_ban::tinh(b);
        let cu = self.ds_vb.get(self.vb).cloned();
        let vi_tri = van_ban::giu_vi_tri(cu.as_ref(), self.vb, &moi);
        if cu.as_ref().map(|c| (c.ho_so, c.tu)) != moi.get(vi_tri).map(|m| (m.ho_so, m.tu)) {
            self.chon_vb.bo_het();
        }
        self.vb = vi_tri;
        self.ds_vb = moi;
        self.khoa_ds_vb = Some(b.phien_ban);
        self.luc_tinh_vb = Some(Instant::now());
        self.o_so_vb = (self.vb + 1).to_string();
    }

    pub(super) fn trang_vb_hien(&self) -> Vec<u32> {
        self.ds_vb.get(self.vb).map(|v| v.trang.clone()).unwrap_or_default()
    }

    pub(super) fn doi_vb(&mut self, i: usize) {
        if self.ds_vb.is_empty() {
            return;
        }
        let i = i.min(self.ds_vb.len() - 1);
        if i != self.vb {
            self.chon_vb.bo_het();
        }
        self.vb = i;
        self.o_so_vb = (i + 1).to_string();
        self.cuon_ds_vb = true;
        self.cuon_dau_vb = true;
    }

    /// Phím tắt tab Văn bản: PgUp/PgDn luôn đổi văn bản; ←/→ đổi văn bản khi chưa chọn trang, có chọn thì đi
    /// trong trang kiểu Explorer; Enter xem lớn; Del bỏ khỏi văn bản; R / Shift+R xoay; Ctrl+Z / Ctrl+Y.
    pub(super) fn phim_tat_vb(&mut self, ctx: &egui::Context) -> Vec<Viec> {
        if ctx.egui_wants_keyboard_input() || self.bang.is_none() || self.tab != Tab::VanBan || self.phong_to.is_some() {
            return Vec::new();
        }
        let mut viec = Vec::new();
        let trang = self.trang_vb_hien();
        let so_cot = self.so_cot_vb;
        let co_chon = self.chon_vb.so_luong() > 0;
        ctx.input(|i| {
            let m = i.modifiers;
            let bam = |k: egui::Key| i.key_pressed(k);
            if bam(egui::Key::PageUp) {
                viec.push(Viec::DoiVb(-1));
            }
            if bam(egui::Key::PageDown) {
                viec.push(Viec::DoiVb(1));
            }
            if m.command {
                if bam(egui::Key::A) {
                    viec.push(Viec::ChonTat);
                }
                if bam(egui::Key::Z) {
                    viec.push(if m.shift { Viec::LamLai } else { Viec::HoanTac });
                }
                if bam(egui::Key::Y) {
                    viec.push(Viec::LamLai);
                }
                if bam(egui::Key::S) {
                    viec.push(Viec::LuuNhan);
                }
            }
            if !co_chon && !m.command {
                if bam(egui::Key::ArrowLeft) {
                    viec.push(Viec::DoiVb(-1));
                }
                if bam(egui::Key::ArrowRight) {
                    viec.push(Viec::DoiVb(1));
                }
            } else {
                let phu = PhimPhu { ctrl: m.command, shift: m.shift };
                for (k, p) in [
                    (egui::Key::ArrowLeft, PhimDiChuyen::Trai),
                    (egui::Key::ArrowRight, PhimDiChuyen::Phai),
                    (egui::Key::ArrowUp, PhimDiChuyen::Len),
                    (egui::Key::ArrowDown, PhimDiChuyen::Xuong),
                    (egui::Key::Home, PhimDiChuyen::DauDs),
                    (egui::Key::End, PhimDiChuyen::CuoiDs),
                ] {
                    if bam(k) {
                        self.chon_vb.phim(p, phu, &trang, so_cot);
                    }
                }
            }
            if m.command {
                return;
            }
            if bam(egui::Key::Escape) {
                viec.push(Viec::BoChon);
            }
            if bam(egui::Key::Delete) || bam(egui::Key::Backspace) {
                viec.push(Viec::BoKhoiVb);
            }
            if bam(egui::Key::R) {
                viec.push(Viec::Xoay(if m.shift { 180 } else { 90 }));
            }
            if bam(egui::Key::Enter) {
                if let Some(k) = self.chon_vb.tieu_diem.or(trang.first().copied()) {
                    viec.push(Viec::PhongTo(k));
                }
            }
        });
        viec
    }

    pub(super) fn tab_van_ban(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinh, viec: &mut Vec<Viec>) {
        super::cot_trai_panel("tach_van_ban_trai", self.trai_gon, cfg.rong_cot_trai).show(ui, |ui| self.bang_van_ban(ui, cfg, viec));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
            if self.ds_vb.is_empty() {
                ui.centered_and_justified(|ui| ui.label(RichText::new("Chưa có văn bản nào").color(CHU_PHU)));
                return;
            }
            self.thanh_dieu_huong(ui, cfg, viec);
            let vb = self.ds_vb[self.vb].clone();
            // Đáy vùng xem: trang trắng bị bỏ trong dải · số trang đang chọn (như dòng trạng thái tab Hồ sơ)
            let so_chon = self.chon_vb.so_luong();
            egui::Panel::bottom("vb_bo_trang").frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(12, 3))).show(ui, |ui| {
                ui.painter().hline(ui.max_rect().expand2(egui::vec2(12.0, 0.0)).x_range(), ui.max_rect().top() - 3.5, Stroke::new(1.0, VIEN_NHAT));
                ui.horizontal(|ui| {
                    ui.set_min_height(22.0);
                    let mut job = egui::text::LayoutJob::default();
                    let thuong = super::chu_dong(CHU_PHU, false);
                    if !vb.bo.is_empty() {
                        job.append(&format!("Bỏ trắng: tr.{}", vb.bo.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")), 0.0, thuong.clone());
                    }
                    if so_chon > 0 {
                        job.append(if vb.bo.is_empty() { "Đã chọn: " } else { " · Đã chọn: " }, 0.0, thuong.clone());
                        job.append(&so_nguyen(so_chon), 0.0, super::chu_dong(crate::gui::VIEN_TAB_CHON, true));
                        job.append(" trang", 0.0, thuong);
                    }
                    ui.label(job);
                    self.bao_tam(ui);
                });
            });
            match cfg.che_do_vb {
                CheDoVb::Luoi => self.luoi_van_ban(ui, cfg, &vb, viec),
                che_do => self.xem_trang_van_ban(ui, cfg, &vb, che_do == CheDoVb::HaiTrang, viec),
            }
        });
    }

    fn bang_van_ban(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let gon_truoc = self.trai_gon;
        super::cot_trai::day(ui, "day_trai_vb", &mut self.tim_vb, "Tìm văn bản", &mut self.trai_gon);
        if self.trai_gon != gon_truoc {
            self.cuon_ds_vb = true;
        }
        let (vb, cuon) = (self.vb, std::mem::take(&mut self.cuon_ds_vb));
        let Some(b) = &self.bang else { return };
        // Cột trái: tên file văn bản sẽ ghi ra (luôn đuôi `.pdf` thường)
        let ten_vb = |m: &MucVanBan| van_ban::ten(&cfg.mau_ten, &b.ho_so[m.ho_so as usize], m.so) + ".pdf";
        // Văn bản hiện trong cột: khớp ô tìm theo tên
        let hien: Vec<usize> = (0..self.ds_vb.len()).filter(|&i| super::cot_trai::khop_tim(&ten_vb(&self.ds_vb[i]), &self.tim_vb)).collect();
        let khong_thay = hien.is_empty() && !self.tim_vb.trim().is_empty();
        let vi_tri = if cuon { hien.iter().position(|&i| i == vb) } else { None };
        if self.trai_gon {
            let ds = &self.ds_vb;
            if let Some(j) = super::cot_trai::cot_so_gon(ui, "cot_so_vb", hien.len(), 26.0, super::CAO_DAU_BANG, |j| (hien[j] + 1).to_string(), |j| ten_vb(&ds[hien[j]]), |j| hien[j] == vb, |_| None, vi_tri) {
                viec.push(Viec::DenVb(hien[j]));
            }
            return;
        }
        bat_dau_bang(ui);
        let mut bang = TableBuilder::new(ui)
            .sense(Sense::click())
            .cell_layout(egui::Layout::left_to_right(Align::Center))
            .column(Column::exact(36.0))
            .column(Column::remainder().at_least(80.0).clip(true))
            .column(Column::exact(52.0));
        if let Some(j) = vi_tri {
            bang = bang.scroll_to_row(j, Some(Align::Center));
        }
        bang.header(super::CAO_DAU_BANG, |mut h| {
            for (i, t) in ["#", "Tên văn bản", "Trang"].into_iter().enumerate() {
                h.col(|ui| o_so(ui, RichText::new(t).color(CHU_PHU), i != 1));
            }
        })
        .body(|body| {
            body.rows(26.0, hien.len().max(khong_thay as usize), |mut row| {
                if khong_thay {
                    row.col(|ui| super::cot_trai::bao_khong_thay(ui, &format!("Không có văn bản nào chứa “{}”", self.tim_vb.trim())));
                    return;
                }
                let i = hien[row.index()];
                let m = &self.ds_vb[i];
                let chon = i == vb;
                row.set_selected(chon);
                // Vạch đậm hơn ở chỗ đổi hồ sơ
                let vach = if self.ds_vb.get(i + 1).is_some_and(|sau| sau.ho_so != m.ho_so) { VIEN_NHAT } else { VACH_DONG };
                let dam = |s: String| if chon { RichText::new(s).dam() } else { RichText::new(s) };
                row.col(|ui| {
                    ke_duoi(ui, vach);
                    o_so(ui, RichText::new((i + 1).to_string()).color(CHU_PHU), true);
                });
                row.col(|ui| {
                    ke_duoi(ui, vach);
                    ui.add_space(super::LE_O_BANG);
                    if m.mo_dau_split {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(13.0, 13.0), Sense::hover());
                        bieu_tuong::ve(ui.painter(), r, BieuTuong::Keo, CAM);
                        ui.add_space(3.0);
                    }
                    ui.label(dam(ten_vb(m))).on_hover_text(&b.ho_so[m.ho_so as usize]);
                });
                row.col(|ui| {
                    ke_duoi(ui, vach);
                    o_so(ui, dam(so_nguyen(m.trang.len())), true);
                });
                if row.response().clicked() {
                    viec.push(Viec::DenVb(i));
                }
            });
        });
    }

    fn thanh_dieu_huong(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let n = self.ds_vb.len();
        let m = self.ds_vb[self.vb].clone();
        let Some(b) = &self.bang else { return };
        let ten = van_ban::ten(&cfg.mau_ten, &b.ho_so[m.ho_so as usize], m.so);
        let so_chon = self.chon_vb.so_luong();
        let thanh = egui::Frame::new().fill(NEN_DIEU_HUONG).inner_margin(Margin::symmetric(12, 4)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().button_padding.x = 6.0;
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                ui.set_height(30.0);
                if ui.add_enabled(self.vb > 0, nut_phang(atom(BieuTuong::TraiChevron, 15.0), false)).on_hover_text("Văn bản trước (PgUp)").clicked() {
                    viec.push(Viec::DoiVb(-1));
                }
                let nhan = ui.label("");
                let id_o = ui.make_persistent_id("o_so_vb");
                if !ui.memory(|m| m.has_focus(id_o)) {
                    self.o_so_vb = (self.vb + 1).to_string();
                }
                let o = ui.add(egui::TextEdit::singleline(&mut self.o_so_vb).id(id_o).desired_width(44.0).align(egui::Align2::CENTER_CENTER)).labelled_by(nhan.id).on_hover_text("Gõ số văn bản rồi Enter");
                if o.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    match van_ban::tu_o_so(&self.o_so_vb, n) {
                        Some(i) => viec.push(Viec::DenVb(i)),
                        None => self.o_so_vb = (self.vb + 1).to_string(),
                    }
                }
                ui.label(RichText::new(format!("/ {}", so_nguyen(n))).color(CHU_PHU));
                if ui.add_enabled(self.vb + 1 < n, nut_phang(atom(BieuTuong::PhaiChevron, 15.0), false)).on_hover_text("Văn bản sau (PgDn)").clicked() {
                    viec.push(Viec::DoiVb(1));
                }
                ui.add_space(12.0);
                ui.label(RichText::new(ten).dam());
                ui.label(RichText::new(format!("· {} trang", so_nguyen(m.trang.len()))).color(CHU_PHU));
                if m.mo_dau_split {
                    ui.label(RichText::new("· mở đầu bằng Split").color(CAM));
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    for (che_do, bt, goi_y) in [(CheDoVb::HaiTrang, BieuTuong::HaiTrang, "2 trang cạnh nhau, cuộn liên tục"), (CheDoVb::MotTrang, BieuTuong::MotTrang, "1 trang vừa chiều ngang, cuộn liên tục"), (CheDoVb::Luoi, BieuTuong::Luoi, "Lưới ảnh nhỏ")] {
                        let chon = cfg.che_do_vb == che_do;
                        if ui.add(nut_phang(atom(bt, 15.0), chon)).on_hover_text(goi_y).clicked() {
                            viec.push(Viec::CheDoVb(che_do));
                        }
                    }
                    ngan(ui);
                    let (pt, nho, lon) = if cfg.che_do_vb == CheDoVb::Luoi {
                        (super::phan_tram_nac(cfg.co_anh_vb), cfg.co_anh_vb > 0, (cfg.co_anh_vb as usize) < super::NAC_ANH.len() - 1)
                    } else {
                        let i = nac_ty_le(cfg.ty_le_vb);
                        (TY_LE[i] as u32, i > 0, i < TY_LE.len() - 1)
                    };
                    super::nut_thu_phong(ui, pt, nho, lon, "", viec, Viec::NacVb(-1), Viec::NacVb(1));
                    ngan(ui);
                    let co = so_chon > 0;
                    super::nut_xoay(ui, co, viec);
                    if ui.add_enabled(co, nut_phang(atom(BieuTuong::TrangGach, 16.0), false)).on_hover_text("Bỏ khỏi văn bản — trang đang chọn thành Trắng, không có trong file tách (Del) · Ctrl+Z hoàn tác").on_disabled_hover_text("Chọn trang trước (Del)").clicked() {
                        viec.push(Viec::BoKhoiVb);
                    }
                });
            });
        });
        ui.painter().hline(thanh.response.rect.x_range(), thanh.response.rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        ui.add_space(-ui.spacing().item_spacing.y);
    }

    /// Nhãn ảnh: `i/n · gốc tr.X`.
    fn nhan_trang_vb(i: usize, n: usize, so: u32) -> String {
        format!("{}/{} · gốc tr.{}", i + 1, n, so)
    }

    fn luoi_van_ban(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, vb: &MucVanBan, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        let co = super::co_vua_khung(cfg.co_anh_vb, ui.available_width());
        let (cao_anh, cao_o) = (co * 1.414, co * 1.414 + CAO_CHU_VB);
        if ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            let z = ui.input(|i| i.zoom_delta());
            if z > 1.05 || z < 0.95 {
                viec.push(Viec::NacVb(if z > 1.0 { 1 } else { -1 }));
            }
        }
        let trang = &vb.trang;
        let n = trang.len();
        let chon = &mut self.chon_vb;
        let anh = &mut self.anh;
        let yeu_cau = &mut self.yeu_cau_anh;
        let mut keo_tu = self.keo_tu_vb;
        let mut so_cot = 1;
        let bac = bac_cho(co * ui.ctx().pixels_per_point()).min(if co > 230.0 { 3 } else { 1 });
        let cuon_dau = std::mem::take(&mut self.cuon_dau_vb);
        let mut vung_cuon = egui::ScrollArea::vertical().id_salt(("luoi_vb", vb.ho_so, vb.tu)).auto_shrink(false);
        if cuon_dau {
            vung_cuon = vung_cuon.vertical_scroll_offset(0.0);
        }
        vung_cuon.show_viewport(ui, |ui, vp| {
            let rong = ui.available_width();
            let mut bc = BoCucLuoi::deu(rong - 2.0 * super::LE, co, cao_o, super::DEM_O, super::KHE_X, super::KHE_Y);
            bc.le_trai += super::LE;
            so_cot = bc.so_cot;
            ui.set_min_size(egui::vec2(rong, bc.so_hang(n) as f32 * bc.cao_hang() + super::LE));
            let goc = ui.max_rect().min + egui::vec2(0.0, super::LE);
            let vung_nhin = Rect::from_min_size(ui.max_rect().min + vp.min.to_vec2(), vp.size());
            let ph = ui.interact(vung_nhin, ui.scope_id().with("luoi_vb"), Sense::click_and_drag());
            let ChuotLuoi { hover, bam_dup, .. } = chuot_luoi(ui, &ph, &bc, goc, vung_nhin, trang, chon, &mut keo_tu);
            if let Some(k) = bam_dup {
                viec.push(Viec::PhongTo(k));
            }
            let painter = ui.painter_at(vung_nhin);
            let font = egui::TextStyle::Small.resolve(ui.style());
            let hang_dau = ((vp.min.y - super::LE) / bc.cao_hang()).floor().max(0.0) as usize;
            let hang_cuoi = ((vp.max.y / bc.cao_hang()).ceil() as usize).min(bc.so_hang(n));
            for hang in hang_dau..hang_cuoi {
                for cot in 0..bc.so_cot {
                    let i = hang * bc.so_cot + cot;
                    if i >= n {
                        break;
                    }
                    let k = trang[i];
                    let t = &b.trang[k as usize];
                    let o = Rect::from_min_size(goc + egui::Vec2::from(bc.goc_o(i)), egui::vec2(co, cao_o));
                    ve_nen_o(&painter, o, chon.co(k), hover == Some(k), chon.tieu_diem == Some(k));
                    let hop = Rect::from_min_size(o.min, egui::vec2(co, cao_anh));
                    let r = ve_anh(&painter, anh, yeu_cau, hop, k, t.ho_so, t.so, t.sua.goc, bac);
                    let viền = if i == 0 && vb.mo_dau_split { Stroke::new(2.0, CAM) } else { Stroke::new(1.0, VIEN_NHAT) };
                    painter.rect_stroke(r, 0.0, viền, StrokeKind::Outside);
                    ve_goc_xoay(&painter, r, t.sua.goc, &font);
                    painter.text(egui::pos2(o.center().x, o.top() + cao_anh + 3.0), egui::Align2::CENTER_TOP, Self::nhan_trang_vb(i, n, t.so), font.clone(), CHU);
                }
            }
            ph.context_menu(|ui| menu_phai_vb(ui, chon.so_luong(), viec, chon.tieu_diem));
        });
        self.keo_tu_vb = keo_tu;
        self.so_cot_vb = so_cot;
    }

    fn xem_trang_van_ban(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, vb: &MucVanBan, hai_trang: bool, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        if ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            let z = ui.input(|i| i.zoom_delta());
            if z > 1.05 || z < 0.95 {
                viec.push(Viec::NacVb(if z > 1.0 { 1 } else { -1 }));
            }
        }
        let ty_le = TY_LE[nac_ty_le(cfg.ty_le_vb)];
        let kich: Vec<(f32, f32)> = vb.trang.iter().map(|&k| b.trang[k as usize]).map(|t| if t.sua.goc % 180 == 90 { (t.cao, t.rong) } else { (t.rong, t.cao) }).collect();
        let khung_rong = ui.available_width();
        let bo_cuc = xem_trang::bo_cuc(khung_rong - 14.0, ty_le, hai_trang, &kich);
        let n = vb.trang.len();
        let chon = &mut self.chon_vb;
        let anh = &mut self.anh;
        let yeu_cau = &mut self.yeu_cau_anh;
        let mut vung_cuon = egui::ScrollArea::both().id_salt(("trang_vb", hai_trang)).auto_shrink(false);
        if std::mem::take(&mut self.dat_giua_ngang) {
            vung_cuon = vung_cuon.horizontal_scroll_offset(xem_trang::cuon_ngang_giua(bo_cuc.rong_noi_dung, khung_rong - 14.0));
        }
        if std::mem::take(&mut self.cuon_dau_vb) {
            vung_cuon = vung_cuon.vertical_scroll_offset(0.0);
        }
        let ppp = ui.ctx().pixels_per_point();
        let mut so_cot = 1;
        vung_cuon.show_viewport(ui, |ui, vp| {
            so_cot = if hai_trang { 2 } else { 1 };
            ui.set_min_size(egui::vec2(bo_cuc.rong_noi_dung, bo_cuc.cao_noi_dung));
            let goc = ui.max_rect().min;
            let vung_nhin = Rect::from_min_size(goc + vp.min.to_vec2(), vp.size());
            let ph = ui.interact(vung_nhin, ui.scope_id().with("trang_vb"), Sense::click());
            let o_man = |o: &xem_trang::OTrang| Rect::from_min_size(goc + egui::vec2(o.x, o.y), egui::vec2(o.rong, o.cao));
            let o_tai = |p: Pos2| bo_cuc.o.iter().position(|o| o_man(o).contains(p)).map(|i| vb.trang[i]);
            let (nhan, tha, vi_tri, phu) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.interact_pos(), PhimPhu { ctrl: i.modifiers.command, shift: i.modifiers.shift }));
            if nhan && ph.hovered() {
                chon.nhan(vi_tri.and_then(o_tai), phu, &vb.trang);
            }
            if tha && ph.hovered() {
                chon.tha();
            }
            if ph.secondary_clicked() {
                if let Some(k) = ph.interact_pointer_pos().and_then(o_tai) {
                    chon.bam_phai(k);
                }
            }
            if ph.double_clicked() {
                if let Some(k) = ph.interact_pointer_pos().and_then(o_tai) {
                    viec.push(Viec::PhongTo(k));
                }
            }
            let hover = ph.hover_pos().and_then(o_tai);
            let painter = ui.painter_at(vung_nhin);
            painter.rect_filled(vung_nhin, 0.0, NEN_XEM_TRANG);
            let font = egui::TextStyle::Small.resolve(ui.style());
            for (i, o) in bo_cuc.o.iter().enumerate() {
                let r = o_man(o);
                if !r.expand(20.0).intersects(vung_nhin) {
                    continue;
                }
                let k = vb.trang[i];
                let t = &b.trang[k as usize];
                ve_nen_o(&painter, r.expand2(egui::vec2(0.0, 9.0)).translate(egui::vec2(0.0, 9.0)), chon.co(k), hover == Some(k), chon.tieu_diem == Some(k));
                let bac = bac_cho(o.rong * ppp);
                painter.add(egui::epaint::Shadow { offset: [0, 1], blur: 4, spread: 0, color: Color32::from_black_alpha(46) }.as_shape(r, 0.0));
                let ra = ve_anh(&painter, anh, yeu_cau, r, k, t.ho_so, t.so, t.sua.goc, bac);
                if i == 0 && vb.mo_dau_split {
                    painter.rect_stroke(ra, 0.0, Stroke::new(2.0, CAM), StrokeKind::Outside);
                }
                ve_goc_xoay(&painter, ra, t.sua.goc, &font);
                painter.text(egui::pos2(r.center().x, r.bottom() + 2.0), egui::Align2::CENTER_TOP, Self::nhan_trang_vb(i, n, t.so), font.clone(), CHU_PHU);
            }
            ph.context_menu(|ui| menu_phai_vb(ui, chon.so_luong(), viec, chon.tieu_diem));
        });
        self.so_cot_vb = so_cot;
    }
}

/// Nền ô: đang chọn / rê chuột / tiêu điểm (giống lưới tab Hồ sơ).
fn ve_nen_o(painter: &egui::Painter, o: Rect, chon: bool, hover: bool, tieu_diem: bool) {
    let nen = o.expand(4.0);
    if chon {
        painter.rect(nen, 2.0, NEN_CHON, Stroke::new(1.0, VIEN_CHON), StrokeKind::Inside);
    } else if hover {
        painter.rect_filled(nen, 2.0, NEN_HOVER);
    }
    if tieu_diem {
        super::ve_net_dut(painter, nen.shrink(1.0), Stroke::new(1.0, CHU_PHU), 2.0);
    }
}

/// Vẽ ảnh trang vừa `hop` (xoay theo góc); chưa có ảnh → khung xám + gửi yêu cầu render. Trả hình chữ nhật ảnh.
#[allow(clippy::too_many_arguments)]
fn ve_anh(painter: &egui::Painter, anh: &mut Option<anh_nho::AnhNho>, yeu_cau: &mut Vec<YeuCau>, hop: Rect, k: u32, ho_so: u32, so: u32, goc: u16, bac: u8) -> Rect {
    let khoa = KhoaAnh { trang: k, bac };
    let tex = anh.as_mut().and_then(|a| {
        if !a.co(&khoa) {
            yeu_cau.push(YeuCau { khoa, file: ho_so as usize, so: so as usize });
        }
        a.lay(k, bac)
    });
    match tex {
        Some(tex) => anh_nho::ve_xoay(painter, hop, &tex, goc),
        None => {
            let (w, h) = if goc % 180 == 90 { (1.414, 1.0) } else { (1.0, 1.414) };
            let k = (hop.width() / w).min(hop.height() / h);
            let r = Rect::from_center_size(hop.center(), egui::vec2(w * k, h * k));
            painter.rect_filled(r, 0.0, NEN_ANH_CHO);
            r
        }
    }
}

fn ve_goc_xoay(painter: &egui::Painter, r: Rect, goc: u16, font: &egui::FontId) {
    if goc != 0 {
        let p = egui::pos2(r.right() - 3.0, r.top() + 3.0);
        let khung = painter.text(p, egui::Align2::RIGHT_TOP, format!("{goc}°"), font.clone(), CHU);
        painter.rect_filled(khung.expand(2.0), 2.0, Color32::from_white_alpha(220));
        painter.text(p, egui::Align2::RIGHT_TOP, format!("{goc}°"), font.clone(), CHU);
    }
}

/// Menu chuột phải trên trang của văn bản.
fn menu_phai_vb(ui: &mut egui::Ui, so_chon: usize, viec: &mut Vec<Viec>, tieu_diem: Option<u32>) {
    bieu_tuong::menu_gon(ui);
    let mut muc = |ui: &mut egui::Ui, icon: BieuTuong, chu: &str, phim: &str, v: Viec, bat: bool| {
        if bieu_tuong::muc_menu_phai(ui, icon, chu, phim, false, bat).clicked() {
            viec.push(v);
            ui.close();
        }
    };
    if so_chon > 0 {
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(format!("{} trang đang chọn", so_nguyen(so_chon))).size(11.5).color(CHU_PHU));
        });
        ui.add_space(3.0);
    }
    if let Some(k) = tieu_diem {
        muc(ui, BieuTuong::KinhCong, "Xem lớn", "Enter", Viec::PhongTo(k), true);
        ui.separator();
    }
    muc(ui, BieuTuong::XoayPhai, "Xoay phải 90°", "R", Viec::Xoay(90), so_chon > 0);
    muc(ui, BieuTuong::Xoay180, "Xoay 180°", "Shift+R", Viec::Xoay(180), so_chon > 0);
    ui.separator();
    muc(ui, BieuTuong::TrangGach, "Bỏ khỏi văn bản", "Del", Viec::BoKhoiVb, so_chon > 0);
    ui.separator();
    muc(ui, BieuTuong::ChonTat, "Chọn tất cả", "Ctrl+A", Viec::ChonTat, true);
    muc(ui, BieuTuong::BoChon, "Bỏ chọn", "Esc", Viec::BoChon, so_chon > 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cau_hinh::CauHinh;

    use crate::gui_tach::bang_trang::{BangTrang, Sua, TrangMeta};
    use crate::tach_vb::nhan::Nhan;
    use crate::tach_vb::phan_tich::NhanTuDong;

    /// a: 5 trang — tr.2 Index, tr.4 Nghi trắng, tr.5 Nghi xoay 180 → văn bản [1] và [3,4,5].
    fn man() -> ManTach {
        let mut trang: Vec<TrangMeta> = (1..=5)
            .map(|so| TrangMeta { ho_so: 0, so, dung_luong: 0, rong: 595.0, cao: 842.0, tu_dong: Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: false, nghi_xoay: None }), sua: Sua::default() })
            .collect();
        trang[1].tu_dong = Some(NhanTuDong { nhan: Nhan::Index, nghi_trang: false, nghi_xoay: None });
        trang[3].tu_dong = Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: true, nghi_xoay: None });
        trang[4].tu_dong = Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: false, nghi_xoay: Some(180) });
        let mut m = ManTach { tab: Tab::VanBan, ..Default::default() };
        m.bang = Some(BangTrang::moi(vec!["a.pdf".into()], trang));
        m.cap_nhat_ds_vb();
        m
    }

    #[test]
    fn xoay_chot_nghi_xoay_bo_khoi_van_ban_va_hoan_tac_rieng_nhom() {
        let mut m = man();
        let mut cfg = CauHinh::default();
        assert_eq!(m.ds_vb.len(), 2);
        m.lam(Viec::DoiVb(1), &mut cfg);
        assert_eq!((m.vb, m.trang_vb_hien()), (1, vec![2, 3, 4]));
        // Xoay trang Nghi xoay ở tab Văn bản = chốt luôn
        m.chon_vb.chon_rieng(4);
        m.lam(Viec::Xoay(180), &mut cfg);
        let b = m.bang.as_ref().unwrap();
        assert_eq!((b.trang[4].sua.goc, b.trang[4].nghi_xoay()), (180, None));
        // Bỏ khỏi văn bản trang Nghi trắng → Trắng, hết cờ
        m.chon_vb.chon_rieng(3);
        m.lam(Viec::BoKhoiVb, &mut cfg);
        let b = m.bang.as_ref().unwrap();
        assert_eq!(b.trang[3].nhan(), Nhan::Trang);
        assert!(!b.trang[3].nghi_trang());
        m.cap_nhat_ds_vb();
        assert_eq!(m.trang_vb_hien(), vec![2, 4], "văn bản còn tr.3 và tr.5");
        // Bỏ hết trang của văn bản → văn bản biến mất, vị trí về văn bản kề
        m.chon_vb.chon_tat(&[2, 4]);
        m.lam(Viec::BoKhoiVb, &mut cfg);
        m.cap_nhat_ds_vb();
        assert_eq!((m.ds_vb.len(), m.vb), (1, 0));
        // Ctrl+Z ở tab Văn bản chỉ đảo nhóm Văn bản; tab Hồ sơ (nhóm Tất cả) không có gì để hoàn tác
        m.lam(Viec::HoanTac, &mut cfg);
        m.cap_nhat_ds_vb();
        assert_eq!(m.ds_vb.len(), 2);
        m.tab = Tab::HoSo;
        m.lam(Viec::HoanTac, &mut cfg);
        assert!(m.bao.as_ref().is_some_and(|(c, _)| c.contains("không còn thao tác")));
    }

    #[test]
    fn nac_ty_le_gan_nhat() {
        assert_eq!(TY_LE[nac_ty_le(100)], 100);
        assert_eq!(TY_LE[nac_ty_le(110)], 100);
        assert_eq!(TY_LE[nac_ty_le(400)], 250);
        assert_eq!(TY_LE[nac_ty_le(10)], 50);
    }
}
