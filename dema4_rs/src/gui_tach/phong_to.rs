//! Hộp phóng to 1 trang (bấm đúp / Enter ở tab Hồ sơ và tab Văn bản): ‹ › / ←→ theo thứ tự đang xem,
//! − / + 50–250 %, Esc / ✕ / bấm nền để đóng. Ảnh render nền theo cỡ hiển thị, có ảnh nhỏ tạm trong lúc chờ.
//! Trong bộ lọc Nghi trắng / Nghi xoay: thanh hỏi duyệt và phím y như lưới (B / C / I / S chọn nhãn, Enter áp), duyệt xong
//! tự sang trang nghi kế; Hoàn tác / Làm lại (Ctrl+Z / Ctrl+Y) ngay trong hộp.
//! Nghi trắng: cột ảnh nhỏ bên trái = mọi trang còn nghi (bấm để xem / duyệt trang bất kỳ); Enter → trang rời cột, hộp sang
//! trang kế phía sau (chỉ tiến về cuối, vừa duyệt trang cuối thì về đầu); góc dưới phải nổi "… được gán nhãn: …".

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Stroke, StrokeKind};

use super::anh_nho::{self, bac_cho, KhoaAnh, YeuCau};
use super::bang_trang::EnterXoay;
use super::bieu_tuong::{self, BieuTuong};
use super::{nhom_nut, ngan_nho, nut_nhom, ten_loc, ve_net_dut, Loc, ManTach, NoiDungNut, Tab, Viec, CAM, MAU_S};
use crate::cau_hinh::CauHinh;
use crate::gui::{ChuDam, CHU_PHU, VIEN_NHAT, XANH_COBALT};
use crate::tach_vb::nhan::Nhan;

pub const TY_LE: [u16; 9] = [50, 75, 100, 125, 150, 175, 200, 225, 250];
/// Cột ảnh nhỏ trang Nghi trắng trong hộp phóng to.
const RONG_COT_NGHI: f32 = 104.0;
/// Thông báo nổi: hiện nhanh (mờ → rõ trong `BAO_NOI_VAO`), giữ rõ tới `BAO_NOI_GIU`, rồi mờ chậm trong `BAO_NOI_MO` giây.
const BAO_NOI_VAO: f32 = 0.2;
const BAO_NOI_GIU: f32 = 2.0;
const BAO_NOI_MO: f32 = 1.4;

#[derive(Debug, Clone, PartialEq)]
pub struct PhongTo {
    /// Thứ tự trang đang xem lúc mở (lọc + sắp tab Hồ sơ, hoặc trang của văn bản).
    pub ds: Vec<u32>,
    pub i: usize,
    /// Chỉ số trong `TY_LE`.
    pub nac: usize,
    /// Cột ảnh Nghi trắng cần cuộn tới trang đang xem (mở hộp / đổi trang).
    pub cuon_cot: bool,
}

impl PhongTo {
    pub fn moi(ds: Vec<u32>, k: u32) -> Self {
        let i = ds.iter().position(|&x| x == k).unwrap_or(0);
        Self { ds, i, nac: 2, cuon_cot: true }
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
        self.cuon_cot = true;
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
    /// Thông báo nổi: 1 dòng nền vàng nhạt "L1_05_13.2 được gán nhãn: Trắng" ngay phía trên dòng "Tự động lưu
    /// index_and_blank.txt" (căn phải theo dòng đó), nổi trên cả hộp phóng to; hiện nhanh, giữ một lúc rồi mờ chậm.
    pub(super) fn ve_bao_noi(&mut self, ctx: &egui::Context) {
        let Some((chu, luc)) = &self.bao_noi else { return };
        let da = luc.elapsed().as_secs_f32();
        if da >= BAO_NOI_GIU + BAO_NOI_MO {
            self.bao_noi = None;
            return;
        }
        // Vào: ease-out (nhanh rồi chậm dần tới rõ hẳn); ra: ease-in-out chậm
        let vao = (da / BAO_NOI_VAO).min(1.0);
        let ra = ((da - BAO_NOI_GIU).max(0.0) / BAO_NOI_MO).min(1.0);
        let do_ro = (1.0 - (1.0 - vao).powi(2)) * (1.0 - ra * ra * (3.0 - 2.0 * ra));
        let man = ctx.content_rect();
        let goc = self.o_luu_nhan.map_or(man.right_bottom() - egui::vec2(16.0, 16.0), |r| egui::pos2(r.right(), r.top() - 6.0));
        egui::Area::new(egui::Id::unique("bao_noi_phong_to")).order(egui::Order::Tooltip).interactable(false).pivot(egui::Align2::RIGHT_BOTTOM).fixed_pos(goc).show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(0xFF, 0xF4, 0xCC).gamma_multiply(do_ro))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0xE6, 0xC8, 0x5C).gamma_multiply(do_ro)))
                .corner_radius(3.0)
                .inner_margin(Margin::symmetric(12, 5))
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(chu.as_str()).color(Color32::from_rgb(0x5A, 0x46, 0x00).gamma_multiply(do_ro)).size(12.5)).extend());
                });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    pub(super) fn hop_phong_to(&mut self, ctx: &egui::Context, cfg: &mut CauHinh) {
        let la_vb = self.tab == Tab::VanBan;
        let loc = if la_vb { Loc::TatCa } else { self.loc };
        let duyet = matches!(loc, Loc::NghiTrang | Loc::NghiXoay);
        let mut viec: Vec<Viec> = Vec::new();
        let nhom = self.nhom();
        let xoa_bs = self.phong_to.is_some() && self.backspace_xoa(ctx);
        let Some(pt) = &mut self.phong_to else { return };
        let (Some(b), Some(k)) = (&self.bang, pt.trang()) else {
            self.phong_to = None;
            return;
        };
        let t = b.trang[k as usize];
        let enter_xoay = b.viec_enter_xoay(&[k]);
        // Nhãn đang chọn cho trang đang xem: Nghi trắng mặc định Trắng, nhóm thường mặc định nhãn hiện tại
        let nhan_duyet = match self.chon_nghi {
            Some((kc, n)) if kc == k => n,
            _ if loc == Loc::NghiTrang => Nhan::Trang,
            _ => t.nhan(),
        };
        let thuong = !la_vb && !duyet;
        // Nhãn cho trang đang xem: Nghi trắng chỉ chọn (Enter mới áp), Nghi xoay C = về chiều gốc, còn lại gán như lưới
        let gan = |n: Nhan| match (loc, n) {
            (Loc::NghiXoay, Nhan::NoiDung) => Viec::VeGoc,
            (Loc::NghiXoay, n) => Viec::Gan(n),
            // Nghi trắng và nhóm thường: chỉ chọn nhãn, Enter / Xong mới áp
            (_, n) => Viec::ChonNghi(n),
        };
        if xoa_bs {
            viec.push(if la_vb { Viec::BoKhoiVb } else { Viec::Xoa });
        }
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
                for (phim, v) in [(egui::Key::S, gan(Nhan::Split)), (egui::Key::I, gan(Nhan::Index)), (egui::Key::B, gan(Nhan::Trang)), (egui::Key::C, gan(Nhan::NoiDung))] {
                    if i.key_pressed(phim) {
                        viec.push(v);
                    }
                }
                // Enter duyệt như lưới: Nghi trắng → áp nhãn đang chọn; Nghi xoay → 1. xoay thử, 2. đúng rồi tiếp
                if i.key_pressed(egui::Key::Enter) {
                    match (loc, enter_xoay) {
                        (Loc::NghiTrang, _) => viec.push(Viec::DuyetNghiTrang(nhan_duyet)),
                        _ if thuong => viec.push(Viec::ApNhanHop(nhan_duyet)),
                        (Loc::NghiXoay, EnterXoay::XoayDeXuat) => viec.push(Viec::XoayDeXuat),
                        (Loc::NghiXoay, EnterXoay::XacNhan) => viec.push(Viec::XacNhanXoay),
                        _ => {}
                    }
                }
            }
            if i.modifiers.command && !la_vb {
                if i.key_pressed(egui::Key::Z) {
                    viec.push(if i.modifiers.shift { Viec::LamLai } else { Viec::HoanTac });
                }
                if i.key_pressed(egui::Key::Y) {
                    viec.push(Viec::LamLai);
                }
            }
            if !i.modifiers.command {
                if i.key_pressed(egui::Key::R) {
                    viec.push(Viec::Xoay(if i.modifiers.shift { 180 } else { 90 }));
                }
                if i.key_pressed(egui::Key::Delete) {
                    viec.push(if la_vb { Viec::BoKhoiVb } else { Viec::Xoa });
                }
            }
        });
        let man = ctx.content_rect();
        let rong_hop = (man.width() * 0.9).max(400.0);
        let cao_hop = (man.height() * 0.9).max(300.0);
        let mut dong = false;
        let khung = egui::Frame::popup(&ctx.global_style()).inner_margin(Margin::same(8)).fill(Color32::from_gray(245));
        let mut bam_cot_ngoai: Option<usize> = None;
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
                // S I B chỉ ở tab Hồ sơ (tab Văn bản chỉ xoay / bỏ khỏi văn bản)
                if !la_vb {
                    nhom_nut(ui, |ui| {
                        if nut_nhom(ui, NoiDungNut::Chu("S", Some(MAU_S)), (nhan_t == Nhan::Split, false), matches!(nhan_t, Nhan::NoiDung | Nhan::Split), "Split — văn bản mới bắt đầu từ trang này (S, bấm lại để huỷ)").clicked() {
                            viec.push(gan(Nhan::Split));
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
                    // Hoàn tác / Làm lại của nhóm lọc đang xem, như thanh công cụ lưới
                    nhom_nut(ui, |ui| {
                        for (bt, lam_lai, phim) in [(BieuTuong::HoanTac, false, "Ctrl+Z"), (BieuTuong::LamLai, true, "Ctrl+Y")] {
                            let buoc = b.buoc_ke(nhom, lam_lai);
                            let goi_y = match &buoc {
                                Some(m) => format!("{} trong nhóm {}: {m} ({phim})", if lam_lai { "Làm lại" } else { "Hoàn tác" }, ten_loc(loc)),
                                None => format!("Không còn thao tác để {} ({phim})", if lam_lai { "làm lại" } else { "hoàn tác" }),
                            };
                            if nut_nhom(ui, NoiDungNut::Icon(bt), (false, false), buoc.is_some(), &goi_y).clicked() {
                                viec.push(if lam_lai { Viec::LamLai } else { Viec::HoanTac });
                            }
                            if !lam_lai {
                                ngan_nho(ui);
                            }
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
            // Bộ lọc Nghi: thanh hỏi duyệt y như dưới thanh công cụ lưới, áp cho trang đang xem
            let cao_hoi = if duyet || thuong {
                let tren = ui.cursor().top();
                ManTach::thanh_hoi_cua(ui, b, &self.ten_hien, loc, &[k], nhan_duyet, true, &mut viec);
                ui.cursor().top() - tren
            } else {
                0.0
            };
            // Nghi trắng: cột ảnh nhỏ bên trái (mọi trang còn nghi), vùng ảnh lớn bên phải
            let cot_nghi = !la_vb;
            let cao_vung = cao_hop - 50.0 - cao_hoi;
            ui.horizontal_top(|ui| {
            if cot_nghi {
                ui.allocate_ui_with_layout(egui::vec2(RONG_COT_NGHI, cao_vung), egui::Layout::top_down(Align::Min), |ui| {
                ui.painter().rect_filled(ui.max_rect(), 3.0, Color32::WHITE);
                let o_w = RONG_COT_NGHI - 16.0;
                let cao_o = o_w * 1.414 + 24.0;
                let ppp = ctx.pixels_per_point();
                let anh = &mut self.anh;
                let yeu_cau = &mut self.yeu_cau_anh;
                let ten_hien = &self.ten_hien;
                let ds = pt.ds.clone();
                let i_dang = pt.i;
                let mut cuon = egui::ScrollArea::vertical().id_salt("phong_to_cot_nghi").max_height(cao_vung).auto_shrink([false, false]);
                // Cuộn tới trang đang xem (tính thẳng vị trí: show_rows chỉ dựng các ô đang hiện)
                if std::mem::take(&mut pt.cuon_cot) {
                    let buoc = cao_o + ui.spacing().item_spacing.y;
                    cuon = cuon.vertical_scroll_offset(((i_dang as f32 + 0.5) * buoc - cao_vung / 2.0).max(0.0));
                }
                cuon.show_rows(ui, cao_o, ds.len(), |ui, hang| {
                    for i in hang {
                        let kk = ds[i];
                        let tk = &b.trang[kk as usize];
                        let (o, ph) = ui.allocate_exact_size(egui::vec2(RONG_COT_NGHI - 4.0, cao_o), egui::Sense::click());
                        if i == i_dang {
                            ui.painter().rect(o.shrink(1.0), 3.0, Color32::from_rgb(0xEA, 0xF2, 0xFC), Stroke::new(2.0, XANH_COBALT), StrokeKind::Inside);
                        }
                        let r_anh = Rect::from_min_size(o.min + egui::vec2(6.0, 4.0), egui::vec2(o_w, o_w * 1.414));
                        let bac = bac_cho(o_w * ppp);
                        let khoa = KhoaAnh { trang: kk, bac };
                        let tex = anh.as_mut().and_then(|a| {
                            if !a.co(&khoa) {
                                yeu_cau.push(YeuCau { khoa, file: tk.ho_so as usize, so: tk.so as usize });
                            }
                            a.lay(kk, bac)
                        });
                        let ve_r = match &tex {
                            Some(tex) => anh_nho::ve_xoay(ui.painter(), r_anh, tex, tk.sua.goc),
                            None => {
                                ui.painter().rect_filled(r_anh, 0.0, Color32::from_gray(232));
                                r_anh
                            }
                        };
                        // Viền theo nhãn như ô lưới
                        let p = ui.painter();
                        match tk.nhan() {
                            Nhan::Index => {
                                p.rect_stroke(ve_r, 0.0, Stroke::new(2.0, XANH_COBALT), StrokeKind::Outside);
                            }
                            Nhan::Trang => ve_net_dut(p, ve_r.expand(1.0), Stroke::new(1.2, XANH_COBALT), 4.0),
                            Nhan::Split => {
                                p.rect_stroke(ve_r, 0.0, Stroke::new(1.5, CAM), StrokeKind::Outside);
                            }
                            Nhan::NoiDung if tk.nghi_trang() => ve_net_dut(p, ve_r.expand(1.0), Stroke::new(1.2, Color32::from_rgb(0xD4, 0xA0, 0x00)), 4.0),
                            Nhan::NoiDung if tk.nghi_xoay().is_some() => ve_net_dut(p, ve_r.expand(1.0), Stroke::new(1.2, CAM), 4.0),
                            Nhan::NoiDung => {
                                p.rect_stroke(ve_r, 0.0, Stroke::new(1.0, VIEN_NHAT), StrokeKind::Outside);
                            }
                        }
                        ui.painter().text(egui::pos2(o.center().x, o.bottom() - 8.0), egui::Align2::CENTER_CENTER, format!("{}.{}", ten_hien[tk.ho_so as usize], tk.so), egui::FontId::proportional(10.5), CHU_PHU);
                        let ph = ph.on_hover_text("Xem trang này");
                        if ph.clicked() {
                            bam_cot_ngoai = Some(i);
                        }
                    }
                });
                });
                ui.add_space(8.0);
            }
            ui.vertical(|ui| {
            // 100 % = cả trang vừa chiều cao hộp
            let (w, h) = if t.sua.goc % 180 == 90 { (t.cao, t.rong) } else { (t.rong, t.cao) };
            let cao_100 = cao_vung;
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
            });
        });
        if let (Some(i), Some(pt)) = (bam_cot_ngoai, &mut self.phong_to) {
            pt.i = i;
            pt.cuon_cot = true;
        }
        if dong || ph.should_close() {
            self.phong_to = None;
        }
        // Thao tác áp cho đúng trang đang xem trong hộp (chọn riêng trang đó rồi làm như trên lưới)
        let k = self.phong_to.as_ref().and_then(|p| p.trang());
        for v in viec {
            let Some(k) = k else { break };
            if la_vb { self.chon_vb.chon_rieng(k) } else { self.chon.chon_rieng(k) }
            // Nhóm thường: áp nhãn đã chọn (khác nhãn hiện tại mới đổi — không bật / tắt nhãn), rồi sang trang kế phía sau;
            // trang không còn thuộc nhóm thì rời cột ảnh; hết cuối danh sách thì về đầu
            if let Viec::ApNhanHop(n) = v {
                if self.bang.as_ref().is_some_and(|b| b.trang[k as usize].nhan() != n) {
                    self.lam(Viec::Gan(n), cfg);
                }
                self.chon_nghi = None;
                if let (Some(pt), Some(b)) = (&mut self.phong_to, &self.bang) {
                    let i_cu = pt.i;
                    let con = b.thuoc_loc(k, loc);
                    pt.ds.retain(|&x| b.thuoc_loc(x, loc));
                    let i = if con { i_cu + 1 } else { i_cu };
                    pt.i = if i < pt.ds.len() { i } else { 0 };
                    pt.cuon_cot = true;
                    match pt.ds.get(pt.i) {
                        Some(&k2) => self.chon.chon_rieng(k2),
                        None => self.phong_to = None,
                    }
                }
                if self.phong_to.is_none() {
                    break;
                }
                continue;
            }
            self.lam(v, cfg);
            // Duyệt xong: trang hết cờ Nghi rời danh sách, hộp sang trang nghi kế (lưới vừa chọn); hết thì đóng
            if !la_vb && ((loc == Loc::NghiTrang && matches!(v, Viec::DuyetNghiTrang(_))) || (loc == Loc::NghiXoay && matches!(v, Viec::XacNhanXoay))) {
                if let (Some(pt), Some(b)) = (&mut self.phong_to, &self.bang) {
                    let i_cu = pt.i;
                    pt.ds.retain(|&x| b.thuoc_loc(x, loc));
                    pt.i = if i_cu < pt.ds.len() { i_cu } else { 0 };
                    pt.cuon_cot = true;
                    match pt.ds.get(pt.i) {
                        Some(&k2) => self.chon.chon_rieng(k2),
                        None => self.phong_to = None,
                    }
                }
                if self.phong_to.is_none() {
                    break;
                }
                continue;
            }
            if duyet && matches!(v, Viec::DuyetNghiTrang(_) | Viec::XacNhanXoay) {
                let ke = self.chon.tieu_diem;
                if let (Some(pt), Some(b)) = (&mut self.phong_to, &self.bang) {
                    pt.ds.retain(|&x| if loc == Loc::NghiTrang { b.trang[x as usize].nghi_trang() } else { b.trang[x as usize].nghi_xoay().is_some() });
                    pt.i = ke.and_then(|k2| pt.ds.iter().position(|&x| x == k2)).unwrap_or(pt.i.min(pt.ds.len().saturating_sub(1)));
                    if pt.ds.is_empty() {
                        self.phong_to = None;
                    }
                }
                if self.phong_to.is_none() {
                    break;
                }
            }
            // Hoàn tác / làm lại: trang về / rời bộ lọc → nạp lại thứ tự lưới, hộp đứng ở trang lưới vừa chọn
            if !la_vb && matches!(v, Viec::HoanTac | Viec::LamLai) {
                self.cap_nhat_thu_tu();
                let ke = self.chon.tieu_diem;
                if let Some(pt) = &mut self.phong_to {
                    pt.ds = self.thu_tu.clone();
                    pt.i = ke.and_then(|k2| pt.ds.iter().position(|&x| x == k2)).unwrap_or(pt.i.min(pt.ds.len().saturating_sub(1)));
                    if pt.ds.is_empty() {
                        self.phong_to = None;
                        break;
                    }
                }
            }
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

    /// Lọc Nghi trắng, phóng to 1 trang: Enter = Trắng như lưới, hộp sang trang nghi kế, hết thì đóng.
    #[test]
    fn phong_to_trong_loc_nghi_trang_enter_duyet_va_sang_trang_ke() {
        use super::super::bang_trang::{BangTrang, Nhom, TrangMeta, Sua};
        use crate::tach_vb::phan_tich::NhanTuDong;
        let trang: Vec<TrangMeta> = (1..=4).map(|so| TrangMeta { ho_so: 0, so, dung_luong: 1, rong: 595.0, cao: 842.0, tu_dong: Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: so == 2 || so == 4, nghi_xoay: None }), sua: Sua::default() }).collect();
        let b = BangTrang::moi(vec!["a.pdf".into()], trang);
        let mut man = ManTach { bang: Some(b), ten_hien: vec!["a".into()], tab: Tab::HoSo, loc: Loc::NghiTrang, loc_menu: Loc::NghiTrang, ..Default::default() };
        man.cap_nhat_thu_tu();
        assert_eq!(man.thu_tu, [1, 3]);
        man.phong_to = Some(PhongTo::moi(man.thu_tu.clone(), 1));
        let mut cfg = CauHinh::default();
        let ctx = egui::Context::default();
        let bam = |man: &mut ManTach, cfg: &mut CauHinh, key: egui::Key, command: bool| {
            let mut raw = egui::RawInput::default();
            let modifiers = egui::Modifiers { command, ..Default::default() };
            raw.events.push(egui::Event::ModifiersChanged(modifiers));
            raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
            let _ = ctx.run_ui(raw, |ui| man.hop_phong_to(&ui.ctx().clone(), cfg));
            man.cap_nhat_thu_tu();
        };
        // I chỉ chọn nhãn, chưa đổi gì; Enter mới áp Index và sang trang nghi kế
        bam(&mut man, &mut cfg, egui::Key::I, false);
        assert!(man.bang.as_ref().unwrap().trang[1].nghi_trang(), "I chỉ chọn, trang vẫn nghi");
        assert_eq!(man.nhan_duyet_cua(Some(1)), Nhan::Index);
        bam(&mut man, &mut cfg, egui::Key::Enter, false);
        assert_eq!(man.bang.as_ref().unwrap().trang[1].nhan(), Nhan::Index, "Enter áp nhãn đã chọn");
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(3), "sang trang nghi kế");
        // Ctrl+Z trong hộp: trang 2 nghi trở lại, hộp quay về trang 2
        bam(&mut man, &mut cfg, egui::Key::Z, true);
        assert!(man.bang.as_ref().unwrap().trang[1].nghi_trang());
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(1));
        // Enter không chọn gì = Trắng
        bam(&mut man, &mut cfg, egui::Key::Enter, false);
        assert_eq!(man.bang.as_ref().unwrap().trang[1].nhan(), Nhan::Trang, "mặc định Trắng");
        bam(&mut man, &mut cfg, egui::Key::Enter, false);
        assert_eq!(man.bang.as_ref().unwrap().trang[3].nhan(), Nhan::Trang);
        assert!(man.phong_to.is_none(), "hết trang nghi → đóng hộp");
        let _ = Nhom::Loc(Loc::TatCa);
    }

    /// Nghi trắng: duyệt trang bất kỳ → sang trang kế phía sau; duyệt trang cuối → về đầu; có thông báo nổi; hết → đóng.
    #[test]
    fn phong_to_nghi_trang_duyet_tien_ve_cuoi_roi_ve_dau() {
        use super::super::bang_trang::{BangTrang, TrangMeta, Sua};
        use crate::tach_vb::phan_tich::NhanTuDong;
        let trang: Vec<TrangMeta> = (1..=6).map(|so| TrangMeta { ho_so: 0, so, dung_luong: 1, rong: 595.0, cao: 842.0, tu_dong: Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: so % 2 == 0, nghi_xoay: None }), sua: Sua::default() }).collect();
        let mut man = ManTach { bang: Some(BangTrang::moi(vec!["a.pdf".into()], trang)), ten_hien: vec!["a".into()], tab: Tab::HoSo, loc: Loc::NghiTrang, loc_menu: Loc::NghiTrang, ..Default::default() };
        man.cap_nhat_thu_tu();
        assert_eq!(man.thu_tu, [1, 3, 5]);
        man.phong_to = Some(PhongTo::moi(man.thu_tu.clone(), 3)); // mở ở trang giữa
        let mut cfg = CauHinh::default();
        let ctx = egui::Context::default();
        let bam = |man: &mut ManTach, cfg: &mut CauHinh, key: egui::Key| {
            let mut raw = egui::RawInput::default();
            raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
            let _ = ctx.run_ui(raw, |ui| man.hop_phong_to(&ui.ctx().clone(), cfg));
            man.cap_nhat_thu_tu();
        };
        let dang = |man: &ManTach| man.phong_to.as_ref().and_then(|p| p.trang());
        bam(&mut man, &mut cfg, egui::Key::I);
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert_eq!(man.bang.as_ref().unwrap().trang[3].nhan(), Nhan::Index);
        assert_eq!(dang(&man), Some(5), "sang trang kế phía sau");
        assert_eq!(man.bao_noi.as_ref().map(|b| b.0.as_str()), Some("a.4 được gán nhãn: Index"));
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert_eq!(dang(&man), Some(1), "vừa duyệt trang cuối → về đầu danh sách");
        assert_eq!(man.phong_to.as_ref().unwrap().ds, [1]);
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert!(man.phong_to.is_none(), "hết trang nghi → đóng hộp");
        assert_eq!(man.bao_noi.as_ref().map(|b| b.0.as_str()), Some("a.2 được gán nhãn: Trắng"));
    }

    /// Nhóm thường trong hộp: phím chỉ chọn nhãn, Enter áp + sang trang kế; Tất cả: trang ở lại; Index: gán khác Index → rời cột.
    #[test]
    fn phong_to_nhom_thuong_chon_roi_enter() {
        use super::super::bang_trang::{BangTrang, TrangMeta, Sua};
        use crate::tach_vb::phan_tich::NhanTuDong;
        let mk = || -> BangTrang {
            let trang: Vec<TrangMeta> = (1..=6).map(|so| TrangMeta { ho_so: 0, so, dung_luong: 1, rong: 595.0, cao: 842.0, tu_dong: Some(NhanTuDong { nhan: if so % 2 == 1 { Nhan::Index } else { Nhan::NoiDung }, nghi_trang: false, nghi_xoay: None }), sua: Sua::default() }).collect();
            BangTrang::moi(vec!["a.pdf".into()], trang)
        };
        let ctx = egui::Context::default();
        let bam = |man: &mut ManTach, cfg: &mut CauHinh, key: egui::Key| {
            let mut raw = egui::RawInput::default();
            raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
            let _ = ctx.run_ui(raw, |ui| man.hop_phong_to(&ui.ctx().clone(), cfg));
            man.cap_nhat_thu_tu();
        };
        let mut cfg = CauHinh::default();
        // Tất cả: B chỉ chọn, Enter áp Trắng cho trang 1, sang trang 2; trang 1 vẫn trong danh sách
        let mut man = ManTach { bang: Some(mk()), ten_hien: vec!["a".into()], tab: Tab::HoSo, loc: Loc::TatCa, loc_menu: Loc::TatCa, ..Default::default() };
        man.cap_nhat_thu_tu();
        man.phong_to = Some(PhongTo::moi(man.thu_tu.clone(), 0));
        bam(&mut man, &mut cfg, egui::Key::B);
        assert_eq!(man.bang.as_ref().unwrap().trang[0].nhan(), Nhan::Index, "B chưa Enter: chưa đổi");
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert_eq!(man.bang.as_ref().unwrap().trang[0].nhan(), Nhan::Trang);
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(1), "sang trang kế");
        assert_eq!(man.phong_to.as_ref().unwrap().ds.len(), 6, "Tất cả: trang ở lại danh sách");
        assert_eq!(man.bao_noi.as_ref().map(|b| b.0.as_str()), Some("a.1 được gán nhãn: Trắng"));
        // Enter không chọn gì = giữ nhãn hiện tại, chỉ sang trang
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert_eq!(man.bang.as_ref().unwrap().trang[1].nhan(), Nhan::NoiDung);
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(2));
        // Index: trang 3 (Index) gán Nội dung → rời cột, hộp đứng ở trang Index kế
        let mut man = ManTach { bang: Some(mk()), ten_hien: vec!["a".into()], tab: Tab::HoSo, loc: Loc::Index, loc_menu: Loc::Index, ..Default::default() };
        man.cap_nhat_thu_tu();
        assert_eq!(man.thu_tu, [0, 2, 4]);
        man.phong_to = Some(PhongTo::moi(man.thu_tu.clone(), 4));
        bam(&mut man, &mut cfg, egui::Key::C);
        bam(&mut man, &mut cfg, egui::Key::Enter);
        assert_eq!(man.bang.as_ref().unwrap().trang[4].nhan(), Nhan::NoiDung);
        assert_eq!(man.phong_to.as_ref().unwrap().ds, [0, 2], "rời cột ảnh");
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(0), "vừa duyệt trang cuối → về đầu");
    }

    /// Lần đầu mở hộp ở lọc Nghi trắng, bấm S (chưa Enter): trang vẫn nghi, vẫn trong danh sách, hộp đứng yên.
    #[test]
    fn phong_to_nghi_trang_bam_s_chua_enter_khong_ap() {
        use super::super::bang_trang::{BangTrang, TrangMeta, Sua};
        use crate::tach_vb::phan_tich::NhanTuDong;
        let trang: Vec<TrangMeta> = (1..=4).map(|so| TrangMeta { ho_so: 0, so, dung_luong: 1, rong: 595.0, cao: 842.0, tu_dong: Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: so == 2 || so == 4, nghi_xoay: None }), sua: Sua::default() }).collect();
        let b = BangTrang::moi(vec!["a.pdf".into()], trang);
        let mut man = ManTach { bang: Some(b), ten_hien: vec!["a".into()], tab: Tab::HoSo, loc: Loc::NghiTrang, loc_menu: Loc::NghiTrang, ..Default::default() };
        man.cap_nhat_thu_tu();
        man.phong_to = Some(PhongTo::moi(man.thu_tu.clone(), 1));
        let mut cfg = CauHinh::default();
        let ctx = egui::Context::default();
        for (key, cho) in [(None, 0), (Some(egui::Key::S), 1), (None, 2)] {
            let mut raw = egui::RawInput::default();
            if let Some(key) = key {
                raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
            }
            let _ = ctx.run_ui(raw, |ui| man.hop_phong_to(&ui.ctx().clone(), &mut cfg));
            man.cap_nhat_thu_tu();
            let _ = cho;
        }
        assert!(man.bang.as_ref().unwrap().trang[1].nghi_trang(), "S chỉ chọn, trang vẫn nghi");
        assert_eq!(man.thu_tu, [1, 3], "vẫn trong danh sách Nghi trắng");
        assert_eq!(man.phong_to.as_ref().and_then(|p| p.trang()), Some(1), "hộp đứng ở trang đang xem");
        assert_eq!(man.nhan_duyet_cua(Some(1)), Nhan::Split);
    }

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
