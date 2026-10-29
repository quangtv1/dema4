//! Hộp phóng to 1 trang (bấm đúp / Enter ở tab Hồ sơ và tab Văn bản): ‹ › / ←→ theo thứ tự đang xem,
//! − / + 50–250 %, Esc / ✕ / bấm nền để đóng. Ảnh render nền theo cỡ hiển thị, có ảnh nhỏ tạm trong lúc chờ.
//! Trong bộ lọc Nghi trắng / Nghi xoay: thanh hỏi duyệt và phím y như lưới (B / C / I / S chọn nhãn, Enter áp), duyệt xong
//! tự sang trang nghi kế; Hoàn tác / Làm lại (Ctrl+Z / Ctrl+Y) ngay trong hộp.

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Stroke, StrokeKind};

use super::anh_nho::{self, bac_cho, KhoaAnh, YeuCau};
use super::bang_trang::EnterXoay;
use super::bieu_tuong::{self, BieuTuong};
use super::{nhom_nut, ngan_nho, nut_nhom, ten_loc, ve_net_dut, Loc, ManTach, NoiDungNut, Tab, Viec, CAM, MAU_S};
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
        let loc = if la_vb { Loc::TatCa } else { self.loc };
        let duyet = matches!(loc, Loc::NghiTrang | Loc::NghiXoay);
        let mut viec: Vec<Viec> = Vec::new();
        let nhan_duyet = self.nhan_duyet_cua(self.phong_to.as_ref().and_then(|p| p.trang()));
        let nhom = self.nhom();
        let Some(pt) = &mut self.phong_to else { return };
        let (Some(b), Some(k)) = (&self.bang, pt.trang()) else {
            self.phong_to = None;
            return;
        };
        let t = b.trang[k as usize];
        let enter_xoay = b.viec_enter_xoay(&[k]);
        // Nhãn cho trang đang xem: Nghi trắng chỉ chọn (Enter mới áp), Nghi xoay C = về chiều gốc, còn lại gán như lưới
        let gan = |n: Nhan| match (loc, n) {
            (Loc::NghiTrang, n) => Viec::ChonNghi(n),
            (Loc::NghiXoay, Nhan::NoiDung) => Viec::VeGoc,
            _ => Viec::Gan(n),
        };
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
            let cao_hoi = if duyet {
                let tren = ui.cursor().top();
                ManTach::thanh_hoi_cua(ui, b, &self.ten_hien, loc, &[k], nhan_duyet, true, &mut viec);
                ui.cursor().top() - tren
            } else {
                0.0
            };
            // 100 % = cả trang vừa chiều cao hộp
            let (w, h) = if t.sua.goc % 180 == 90 { (t.cao, t.rong) } else { (t.rong, t.cao) };
            let cao_100 = cao_hop - 50.0 - cao_hoi;
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
            // Duyệt xong: trang hết cờ Nghi rời danh sách, hộp sang trang nghi kế (lưới vừa chọn); hết thì đóng
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
