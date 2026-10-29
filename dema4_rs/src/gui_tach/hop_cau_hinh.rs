//! Hộp Cấu hình (menu Cấu hình): 2 tab, thân cao cố định để hộp không nhảy khi đổi tab.
//! Tab Tách PDF: nhóm 1 "Tự động nhận dạng khi Phân tích", nhóm 2 "Kết quả tách".

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Sense, Stroke, StrokeKind};

use super::bieu_tuong::thong_tin;
use super::thanh_nguong::{self as tn, Mep};
use crate::cau_hinh::{CauHinh, LoaiMa, LoiCauHinh, LuuNhan, TIN_CAY_KHOANG};
use crate::gui::{nut_mau, ChuDam, CHU, CHU_PHU, DO, VIEN, VIEN_NHAT, XANH_COBALT, XANH_LA};

/// Dòng `nhãn | ô`: cột nhãn cố định, ô kéo tới mép phải (macro để 2 khối cùng mượn `self`).
macro_rules! dong {
    ($ui:expr, |$u:ident| $nhan:block, |$v:ident| $o:block $(,)?) => {
        $ui.horizontal(|$v| {
            cot_nhan($v, |$u| $nhan);
            $o
        })
    };
}

const RONG_HOP: f32 = 560.0;
const CAO_THAN: f32 = 470.0;
const RONG_NHAN: f32 = 118.0;
const VANG_CHU: Color32 = Color32::from_rgb(0xA0, 0x70, 0x00);
const XAM_DAM: Color32 = Color32::from_rgb(0x70, 0x70, 0x70);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCauHinh {
    Dem,
    Tach,
}

pub enum KetQuaHop {
    Mo,
    Huy,
    Luu(CauHinh),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Keo {
    Mep(Mep),
    /// Kéo cả khung: giá trị tại chỗ bắt đầu + 2 ngưỡng lúc đó.
    Khung { v0: f32, t0: f32, n0: f32 },
}

pub struct HopCauHinh {
    pub tab: TabCauHinh,
    pub nhap: CauHinh,
    o_trang: String,
    o_nghi: String,
    o_tin_cay: String,
    keo: Option<Keo>,
    mep_chon: Option<Mep>,
}

fn so_o(v: f32) -> String {
    format!("{v:.2}").replace('.', ",")
}

fn doc_so(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Đưa các mục Tách PDF về mặc định (giữ phần Đếm và trạng thái giao diện).
pub fn mac_dinh_tach(c: &CauHinh) -> CauHinh {
    let md = CauHinh::default();
    CauHinh {
        quet_trang: md.quet_trang,
        quet_ma: md.quet_ma,
        quet_xoay: md.quet_xoay,
        nguong_trang: md.nguong_trang,
        nguong_nghi: md.nguong_nghi,
        loai_ma: md.loai_ma,
        ma_index: md.ma_index,
        tin_cay_xoay: md.tin_cay_xoay,
        thu_muc_ra: md.thu_muc_ra,
        mau_ten: md.mau_ten,
        luu_nhan: md.luu_nhan,
        ..c.clone()
    }
}

/// Dòng ước tốc độ theo tổ hợp loại quét đang bật.
pub fn uoc_toc_do(c: &CauHinh) -> (String, bool) {
    let ma = c.quet_ma && c.loai_ma == LoaiMa::Code128;
    let mut s = match (c.quet_trang, ma) {
        (true, true) => "Ước ~120 ms/trang/lõi".to_string(),
        (true, false) => "Chỉ dò trắng: nhanh ~3 lần".to_string(),
        (false, true) => "Chỉ đọc mã: gần như không nhanh hơn".to_string(),
        (false, false) => "Không quét: chỉ đọc dung lượng + ảnh nhỏ, vài giây".to_string(),
    };
    if c.quet_xoay {
        s += " · chiều trang +~15 ms/trang";
    }
    let thieu = !c.quet_trang || !ma;
    if thieu {
        s += " — loại nào tắt thì gán tay I / B";
    }
    (s, thieu)
}

impl HopCauHinh {
    pub fn moi(cfg: &CauHinh, tab: TabCauHinh) -> Self {
        Self { tab, nhap: cfg.clone(), o_trang: so_o(cfg.nguong_trang), o_nghi: so_o(cfg.nguong_nghi), o_tin_cay: so_o(cfg.tin_cay_xoay), keo: None, mep_chon: None }
    }

    fn dong_bo_o(&mut self) {
        self.o_trang = so_o(self.nhap.nguong_trang);
        self.o_nghi = so_o(self.nhap.nguong_nghi);
        self.o_tin_cay = so_o(self.nhap.tin_cay_xoay);
    }

    /// Lỗi hiện tại (gồm ô gõ không ra số) → khoá Lưu.
    pub fn loi(&self) -> Vec<LoiCauHinh> {
        let mut l = self.nhap.kiem_tra();
        if (doc_so(&self.o_trang).is_none() || doc_so(&self.o_nghi).is_none()) && !l.contains(&LoiCauHinh::Nguong) {
            l.push(LoiCauHinh::Nguong);
        }
        if doc_so(&self.o_tin_cay).is_none() && !l.contains(&LoiCauHinh::TinCay) {
            l.push(LoiCauHinh::TinCay);
        }
        l
    }

    pub fn ve(&mut self, ctx: &egui::Context) -> KetQuaHop {
        let mut kq = KetQuaHop::Mo;
        let khung = egui::Frame::popup(&ctx.global_style()).inner_margin(Margin::same(14)).fill(Color32::WHITE);
        let phan_hoi = egui::Modal::new(egui::Id::unique("hop_cau_hinh")).frame(khung).show(ctx, |ui| {
            ui.set_width(RONG_HOP);
            ui.label(RichText::new("Cấu hình").dam().size(15.0));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (tab, ten) in [(TabCauHinh::Dem, "Đếm trang A4"), (TabCauHinh::Tach, "Tách PDF")] {
                    if ui.selectable_label(self.tab == tab, ten).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("than_cau_hinh").max_height(CAO_THAN).min_scrolled_height(CAO_THAN).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_min_height(CAO_THAN);
                match self.tab {
                    TabCauHinh::Tach => self.tab_tach(ui),
                    TabCauHinh::Dem => {
                        ui.add_space(20.0);
                        ui.label(RichText::new("Hệ số quy A4 và ranh giới khổ — đang chuyển từ màn Đếm vào đây.").color(CHU_PHU));
                    }
                }
            });
            ui.separator();
            let loi = self.loi();
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("Mặc định").min_size(egui::vec2(70.0, 24.0))).on_hover_text("Đưa các mục của tab đang mở về mặc định").clicked() && self.tab == TabCauHinh::Tach {
                    self.nhap = mac_dinh_tach(&self.nhap);
                    self.dong_bo_o();
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let luu = ui.add_enabled(loi.is_empty(), nut_mau("Lưu", XANH_LA).min_size(egui::vec2(70.0, 24.0))).on_disabled_hover_text("Sửa ô đang báo đỏ trước");
                    if luu.clicked() {
                        kq = KetQuaHop::Luu(self.nhap.clone());
                    }
                    if ui.add(egui::Button::new("Huỷ").min_size(egui::vec2(70.0, 24.0))).clicked() {
                        kq = KetQuaHop::Huy;
                    }
                });
            });
        });
        if phan_hoi.should_close() && matches!(kq, KetQuaHop::Mo) {
            kq = KetQuaHop::Huy;
        }
        kq
    }

    fn tab_tach(&mut self, ui: &mut egui::Ui) {
        let loi = self.loi();
        nhom(ui, "1. Tự động nhận dạng khi Phân tích", |ui| {
            // Trang trắng: 2 ô số + chú thích 3 vùng, thanh khoảng bên dưới
            dong!(
                ui,
                |ui| {
                    ui.checkbox(&mut self.nhap.quet_trang, "Trang trắng");
                    thong_tin(ui, &tn::goi_y(self.nhap.nguong_trang, self.nhap.nguong_nghi));
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_trang, |ui| {
                        let r1 = ui.add(egui::TextEdit::singleline(&mut self.o_trang).desired_width(46.0)).on_hover_text("Ngưỡng trắng (%)");
                        ui.label("–");
                        let r2 = ui.add(egui::TextEdit::singleline(&mut self.o_nghi).desired_width(46.0)).on_hover_text("Ngưỡng nghi trắng (%)");
                        ui.label("%");
                        if r1.changed() || r2.changed() {
                            if let (Some(t), Some(n)) = (doc_so(&self.o_trang), doc_so(&self.o_nghi)) {
                                (self.nhap.nguong_trang, self.nhap.nguong_nghi) = (t, n);
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| chu_thich_vung(ui));
                        (r1.has_focus() || r2.has_focus()).then_some(())
                    });
                },
            );
            ui.add_enabled_ui(self.nhap.quet_trang, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(RONG_NHAN + 8.0);
                    let sai = loi.contains(&LoiCauHinh::Nguong);
                    self.thanh_khoang(ui, sai);
                });
                let sai = loi.contains(&LoiCauHinh::Nguong);
                goi_y_duoi(
                    ui,
                    if sai { Some(("Ngưỡng trắng phải ≤ ngưỡng nghi, trong 0–10 %".to_string(), DO)) } else { Some(("Dưới số đầu: Trắng · đến số sau: Nghi trắng · trang có dấu / mực màu luôn vào Nghi trắng".to_string(), CHU_PHU)) },
                );
            });
            ui.add_space(6.0);
            dong!(
                ui,
                |ui| {
                    ui.checkbox(&mut self.nhap.quet_ma, "Mã index");
                    thong_tin(ui, "Trang có mã này là tờ index (điểm cắt, bỏ khi tách). Chấp nhận lệch ≤ 3 ký tự; để trống = mọi mã đọc được. Đổi giá trị → index gán lại ngay, không phân tích lại.");
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_ma, |ui| {
                        egui::ComboBox::from_id_salt("loai_ma")
                            .width(110.0)
                            .selected_text(match self.nhap.loai_ma {
                                LoaiMa::Code128 => "Code 128",
                                LoaiMa::Qr => "QR code",
                                LoaiMa::Chu => "Cụm ký tự",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.nhap.loai_ma, LoaiMa::Code128, "Code 128");
                                ui.selectable_value(&mut self.nhap.loai_ma, LoaiMa::Qr, "QR code");
                                ui.selectable_value(&mut self.nhap.loai_ma, LoaiMa::Chu, "Cụm ký tự");
                            });
                        ui.add(egui::TextEdit::singleline(&mut self.nhap.ma_index).desired_width(ui.available_width()).hint_text("mọi mã")).on_hover_text("Giá trị mã index");
                    });
                },
            );
            if self.nhap.quet_ma && self.nhap.loai_ma != LoaiMa::Code128 {
                goi_y_duoi(ui, Some(("Chưa hỗ trợ — Phân tích sẽ không tự tìm trang index (vẫn gán tay được)".into(), VANG_CHU)));
            }
            ui.add_space(6.0);
            dong!(
                ui,
                |ui| {
                    ui.checkbox(&mut self.nhap.quet_xoay, "Chiều trang");
                    thong_tin(
                        ui,
                        "Gợi ý trang scan bị xoay vào bộ lọc Nghi xoay, không tự xoay. Độ tin cậy 0–1 do mô hình chấm: từ mức này trở lên kết luận ngay, thấp hơn chạy thêm 4 góc xoay (đạt 0,5 mới gắn cờ). Tăng = ít cờ, chắc hơn, chậm hơn chút; giảm = nhiều cờ, dễ gợi ý sai. Mẫu 2: 0,8 không sai trang nào.",
                    );
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_xoay, |ui| {
                        let nhan = ui.label("Độ tin cậy ≥");
                        let khac = (self.nhap.tin_cay_xoay - CauHinh::default().tin_cay_xoay).abs() > 1e-4;
                        let o = egui::TextEdit::singleline(&mut self.o_tin_cay).desired_width(46.0).text_color(if khac { XANH_COBALT } else { CHU });
                        if ui.add(o).labelled_by(nhan.id).changed() {
                            if let Some(v) = doc_so(&self.o_tin_cay) {
                                self.nhap.tin_cay_xoay = v;
                            }
                        }
                    });
                },
            );
            if loi.contains(&LoiCauHinh::TinCay) {
                goi_y_duoi(ui, Some((format!("Nhập từ {} đến {}", so_o(TIN_CAY_KHOANG.0), so_o(TIN_CAY_KHOANG.1)), DO)));
            }
            ui.add_space(4.0);
            let (toc_do, thieu) = uoc_toc_do(&self.nhap);
            ui.label(RichText::new(toc_do).small().color(if thieu { VANG_CHU } else { CHU_PHU }));
        });
        ui.add_space(10.0);
        nhom(ui, "2. Kết quả tách", |ui| {
            let mut focus_ra = false;
            dong!(
                ui,
                |ui| {
                    ui.label("Lưu kết quả vào");
                },
                |ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut self.nhap.thu_muc_ra).desired_width(ui.available_width() - 34.0).hint_text("cạnh thư mục nguồn")).on_hover_text("Thư mục cha chứa <nguồn>_vanban");
                    focus_ra = r.has_focus();
                    if ui.button("…").on_hover_text("Chọn thư mục").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.nhap.thu_muc_ra = p.display().to_string();
                        }
                    }
                },
            );
            if focus_ra {
                let cha = if self.nhap.thu_muc_ra.trim().is_empty() { "…".to_string() } else { self.nhap.thu_muc_ra.trim_end_matches(['/', '\\']).to_string() };
                goi_y_duoi(ui, Some((format!("Ví dụ: {cha}{}<nguồn>_vanban (không được nằm trong thư mục nguồn)", std::path::MAIN_SEPARATOR), CHU_PHU)));
            }
            ui.add_space(6.0);
            let mut focus_ten = false;
            dong!(
                ui,
                |ui| {
                    ui.label("Tên văn bản");
                    thong_tin(ui, "Mẫu tên file văn bản: {hoso} = tên hồ sơ, {n} = số thứ tự, {n3} = số thứ tự 3 chữ số (001).");
                },
                |ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut self.nhap.mau_ten).desired_width(ui.available_width())).on_hover_text("Mẫu tên văn bản");
                    focus_ten = r.has_focus();
                },
            );
            if loi.contains(&LoiCauHinh::MauTen) {
                goi_y_duoi(ui, Some(("Mẫu phải có {n} hoặc {n3}".into(), DO)));
            } else if focus_ten {
                let vd = crate::tach_vb::tach::ten_van_ban(&self.nhap.mau_ten, "HS001", 1);
                goi_y_duoi(ui, Some((format!("Ví dụ: {vd} · biến {{hoso}} {{n}} {{n3}}"), CHU_PHU)));
            }
            ui.add_space(6.0);
            dong!(
                ui,
                |ui| {
                    ui.label("Lưu nhãn");
                },
                |ui| {
                    ui.radio_value(&mut self.nhap.luu_nhan, LuuNhan::TuDong, "Tự động");
                    ui.radio_value(&mut self.nhap.luu_nhan, LuuNhan::BangTay, "Bằng tay (Ctrl+S)");
                },
            );
        });
    }

    /// Thanh khoảng 2 ngưỡng: kéo mép, kéo cả khung, bấm thước, ←/→ (Shift = 0,1).
    fn thanh_khoang(&mut self, ui: &mut egui::Ui, sai: bool) {
        let rong = ui.available_width();
        let (vung, _) = ui.allocate_exact_size(egui::vec2(rong, 40.0), Sense::hover());
        let thuoc = Rect::from_min_size(vung.min + egui::vec2(0.0, 13.0), egui::vec2(rong, 12.0));
        let x_cua = |v: f32| thuoc.left() + tn::vi_tri(v) * thuoc.width();
        let v_tai = |x: f32| tn::gia_tri((x - thuoc.left()) / thuoc.width());
        let (t, n) = (self.nhap.nguong_trang, self.nhap.nguong_nghi);
        let id = ui.make_persistent_id("thanh_nguong");
        let p_thuoc = ui.interact(thuoc.expand2(egui::vec2(0.0, 4.0)), id.with("thuoc"), Sense::click());
        let khung = Rect::from_x_y_ranges(x_cua(t)..=x_cua(n), (thuoc.top() - 3.0)..=(thuoc.bottom() + 3.0));
        let p_khung = ui.interact(khung.shrink2(egui::vec2(5.0, 0.0)), id.with("khung"), Sense::drag());
        let p_trai = ui.interact(Rect::from_center_size(egui::pos2(x_cua(t), thuoc.center().y), egui::vec2(10.0, 22.0)), id.with("trai"), Sense::click_and_drag());
        let p_phai = ui.interact(Rect::from_center_size(egui::pos2(x_cua(n), thuoc.center().y), egui::vec2(10.0, 22.0)), id.with("phai"), Sense::click_and_drag());
        let vi_tri = ui.input(|i| i.pointer.interact_pos());
        // Bắt đầu kéo
        if p_trai.drag_started() || p_trai.clicked() {
            self.keo = p_trai.drag_started().then_some(Keo::Mep(Mep::Trai));
            self.mep_chon = Some(Mep::Trai);
        } else if p_phai.drag_started() || p_phai.clicked() {
            self.keo = p_phai.drag_started().then_some(Keo::Mep(Mep::Phai));
            self.mep_chon = Some(Mep::Phai);
        } else if p_khung.drag_started() {
            if let Some(p) = vi_tri {
                self.keo = Some(Keo::Khung { v0: v_tai(p.x), t0: t, n0: n });
            }
        } else if p_thuoc.clicked() {
            if let Some(p) = vi_tri {
                (self.nhap.nguong_trang, self.nhap.nguong_nghi) = tn::bam_thuoc(t, n, (p.x - thuoc.left()) / thuoc.width());
                self.dong_bo_o();
            }
        }
        if let (Some(k), Some(p)) = (self.keo, vi_tri) {
            if ui.input(|i| i.pointer.primary_down()) {
                let moi = match k {
                    Keo::Mep(m) => tn::dat_mep(t, n, m, v_tai(p.x)),
                    Keo::Khung { v0, t0, n0 } => tn::doi_khung(t0, n0, v_tai(p.x) - v0),
                };
                (self.nhap.nguong_trang, self.nhap.nguong_nghi) = moi;
                self.dong_bo_o();
            } else {
                self.keo = None;
            }
        }
        // Phím ←/→ trên mép đang chọn (khi không gõ trong ô chữ)
        if let Some(m) = self.mep_chon {
            if !ui.ctx().egui_wants_keyboard_input() {
                let (trai, phai, lon) = ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.modifiers.shift));
                if trai || phai {
                    (self.nhap.nguong_trang, self.nhap.nguong_nghi) = tn::phim(self.nhap.nguong_trang, self.nhap.nguong_nghi, m, phai, lon);
                    self.dong_bo_o();
                }
            }
        }
        // Vẽ
        let (t, n) = (self.nhap.nguong_trang, self.nhap.nguong_nghi);
        let p = ui.painter();
        let mo = if sai { 0.35 } else { 1.0 };
        let font = egui::FontId::proportional(10.0);
        for m in tn::MOC {
            let x = x_cua(m);
            p.text(egui::pos2(x, vung.top()), egui::Align2::CENTER_TOP, tn::so_vn(m), font.clone(), CHU_PHU.gamma_multiply(mo));
            p.vline(x, (thuoc.top() - 2.0)..=thuoc.top(), Stroke::new(1.0, VIEN));
        }
        let (xt, xn) = (x_cua(t), x_cua(n));
        p.rect(Rect::from_x_y_ranges(thuoc.left()..=xt, thuoc.y_range()), 0.0, Color32::WHITE, Stroke::new(1.0, VIEN), StrokeKind::Inside);
        // Nghi trắng: caro trắng / xám
        let o = 4.0;
        let mut x = xt;
        let mut cot = 0;
        while x < xn {
            for hang in 0..3 {
                if (cot + hang) % 2 == 0 {
                    let r = Rect::from_min_size(egui::pos2(x, thuoc.top() + hang as f32 * o), egui::vec2(o, o)).intersect(Rect::from_x_y_ranges(xt..=xn, thuoc.y_range()));
                    p.rect_filled(r, 0.0, Color32::from_gray(200).gamma_multiply(mo));
                }
            }
            x += o;
            cot += 1;
        }
        p.rect_filled(Rect::from_x_y_ranges(xn..=thuoc.right(), thuoc.y_range()), 0.0, XAM_DAM.gamma_multiply(mo));
        p.rect_stroke(thuoc, 0.0, Stroke::new(1.0, VIEN_NHAT), StrokeKind::Inside);
        p.rect_stroke(Rect::from_x_y_ranges(xt..=xn, (thuoc.top() - 3.0)..=(thuoc.bottom() + 3.0)), 0.0, Stroke::new(1.0, XAM_DAM), StrokeKind::Outside);
        let md = CauHinh::default();
        for (x, pr, mep, v, v_md) in [(xt, &p_trai, Mep::Trai, t, md.nguong_trang), (xn, &p_phai, Mep::Phai, n, md.nguong_nghi)] {
            let sang = pr.hovered() || pr.dragged() || self.mep_chon == Some(mep);
            p.vline(x, (thuoc.top() - 4.0)..=(thuoc.bottom() + 4.0), Stroke::new(2.0, if sang { XANH_COBALT } else { CHU }));
            let mau = if (v - v_md).abs() > 1e-4 { XANH_COBALT } else { CHU };
            p.text(egui::pos2(x, thuoc.bottom() + 4.0), egui::Align2::CENTER_TOP, tn::so_vn(v), egui::FontId::proportional(11.0), mau.gamma_multiply(mo));
        }
        if pr_hover(&p_trai, &p_phai, &p_khung) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }
}

fn pr_hover(a: &egui::Response, b: &egui::Response, c: &egui::Response) -> bool {
    a.hovered() || b.hovered() || a.dragged() || b.dragged() || c.dragged() || c.hovered()
}

/// Nhóm có khung + tiêu đề đánh số.
fn nhom(ui: &mut egui::Ui, tieu_de: &str, noi_dung: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().stroke(Stroke::new(1.0, VIEN_NHAT)).corner_radius(4).inner_margin(Margin::same(10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tieu_de).dam());
        ui.add_space(6.0);
        noi_dung(ui);
    });
}

/// Cột nhãn cố định của dòng `nhãn | ô`.
fn cot_nhan(ui: &mut egui::Ui, nhan: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(egui::vec2(RONG_NHAN, 24.0), egui::Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_width(RONG_NHAN);
        nhan(ui);
    });
    ui.add_space(8.0 - ui.spacing().item_spacing.x);
}

fn goi_y_duoi(ui: &mut egui::Ui, chu: Option<(String, Color32)>) {
    if let Some((chu, mau)) = chu {
        ui.horizontal(|ui| {
            ui.add_space(RONG_NHAN + 8.0);
            ui.label(RichText::new(chu).small().color(mau));
        });
    }
}

/// 3 ô mẫu: Trắng · Nghi trắng · Có chữ.
fn chu_thich_vung(ui: &mut egui::Ui) {
    for (ten, kieu) in [("Có chữ", 2), ("Nghi trắng", 1), ("Trắng", 0)] {
        ui.label(RichText::new(ten).size(12.0));
        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 11.0), Sense::hover());
        let p = ui.painter();
        match kieu {
            0 => {
                p.rect_filled(r, 0.0, Color32::WHITE);
            }
            1 => {
                p.rect_filled(r, 0.0, Color32::WHITE);
                for i in 0..5 {
                    for j in 0..3 {
                        if (i + j) % 2 == 0 {
                            p.rect_filled(Rect::from_min_size(r.min + egui::vec2(i as f32 * 3.6, j as f32 * 3.67), egui::vec2(3.6, 3.67)), 0.0, Color32::from_gray(200));
                        }
                    }
                }
            }
            _ => {
                p.rect_filled(r, 0.0, XAM_DAM);
            }
        }
        p.rect_stroke(r, 0.0, Stroke::new(1.0, VIEN), StrokeKind::Inside);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_dinh_tach_giu_phan_dem_va_giao_dien() {
        let c = CauHinh { he_so: [1.0; 8], rong_cot_trai: 300.0, nguong_trang: 2.0, mau_ten: "{n}".into(), ..Default::default() };
        let m = mac_dinh_tach(&c);
        assert_eq!((m.he_so, m.rong_cot_trai), ([1.0; 8], 300.0));
        assert_eq!((m.nguong_trang, m.mau_ten.as_str()), (1.0, "{hoso}_part{n}"));
    }

    #[test]
    fn o_go_sai_khoa_luu() {
        let mut h = HopCauHinh::moi(&CauHinh::default(), TabCauHinh::Tach);
        assert!(h.loi().is_empty());
        h.o_trang = "abc".into();
        assert_eq!(h.loi(), [LoiCauHinh::Nguong]);
        h.o_trang = "1,00".into();
        h.nhap.tin_cay_xoay = 0.3;
        assert_eq!(h.loi(), [LoiCauHinh::TinCay]);
    }

    #[test]
    fn uoc_toc_do_theo_to_hop() {
        assert_eq!(uoc_toc_do(&CauHinh::default()), ("Ước ~120 ms/trang/lõi · chiều trang +~15 ms/trang".to_string(), false));
        let (s, thieu) = uoc_toc_do(&CauHinh { quet_ma: false, quet_xoay: false, ..Default::default() });
        assert!(s.starts_with("Chỉ dò trắng") && thieu);
        assert!(uoc_toc_do(&CauHinh { loai_ma: LoaiMa::Qr, ..Default::default() }).1, "QR chưa hỗ trợ = như tắt mã");
    }
}
