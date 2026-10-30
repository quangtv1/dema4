//! Hộp Cấu hình (menu Cấu hình): 2 tab, thân cao cố định để hộp không nhảy khi đổi tab.
//! Tab Tách PDF: nhóm 1 "Tự động nhận dạng khi Phân tích", nhóm 2 "Kết quả tách".

use eframe::egui::{self, Align, Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, StrokeKind};

use super::bieu_tuong::thong_tin;
use super::khung_hop as kh;
use super::thanh_nguong::{self as tn, Mep};
use super::thuoc_kho as tk;
use crate::cau_hinh::{A0Lon, CauHinh, LoaiMa, LoiCauHinh, LuuNhan, CANH_CHUAN_MM, RANH_GIOI_MAC_DINH, TIN_CAY_KHOANG};
use crate::dem_pdf::{KHO, PT_MOI_MM};
use crate::gui::{ChuDam, CHU, CHU_PHU, DO, HO_FONT_DAM, VIEN_NHAT, VIEN_TAB_CHON, XANH_LA};

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
/// Cột nhãn tab Tách PDF (vừa "☑ Trang trắng ⓘ"); ô bắt đầu ở `LE_O`.
const RONG_NHAN: f32 = 118.0;
const LE_O: f32 = 126.0;
const RONG_NUT_CHON: f32 = 30.0;
const NEN_NHOM: Color32 = Color32::from_rgb(0xFC, 0xFC, 0xFC);
const VIEN_THUOC: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);
const VANG_CHU: Color32 = Color32::from_rgb(0x9A, 0x6A, 0x00);
const NAU: Color32 = Color32::from_rgb(0x7A, 0x4A, 0x00);

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
    o_he_so: [String; 8],
    o_a0_he_so: String,
    o_thu: String,
    /// Tay đang kéo / đang chọn trên thước khổ (0..7 ranh giới, 7 = ngưỡng A0).
    keo_tay: Option<usize>,
    tay_chon: Option<usize>,
    /// Thư mục đang mở còn cache phân tích (bật nút Xoá cache); `xoa_cache` = vừa bấm, app xoá rồi cập nhật `co_cache`.
    pub co_cache: bool,
    pub xoa_cache: bool,
}

fn so_o(v: f32) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// Hệ số giữ đủ f64 (0,1 không thành 0,10000000149).
fn doc_so64(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
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

/// Đưa các mục Đếm trang A4 về mặc định.
pub fn mac_dinh_dem(c: &CauHinh) -> CauHinh {
    let md = CauHinh::default();
    CauHinh { he_so: md.he_so, ranh_gioi: md.ranh_gioi, a0_lon: md.a0_lon, a0_lon_he_so: md.a0_lon_he_so, a0_nguong_mm: md.a0_nguong_mm, ..c.clone() }
}

/// Dòng ước tốc độ theo tổ hợp loại quét đang bật (QR / cụm ký tự chưa hỗ trợ = như tắt mã).
pub fn uoc_toc_do(c: &CauHinh) -> (String, bool) {
    let ma = c.quet_ma && c.loai_ma == LoaiMa::Code128;
    let mut s = match (c.quet_trang, ma) {
        (true, true) => "Quét cả hai: ~120 ms/trang/lõi (100.000 trang ≈ 25 phút trên 8 lõi)",
        (true, false) => "Chỉ trang trắng: nhanh hơn ~3 lần (100.000 trang ≈ 8 phút) · trang index gán tay (I)",
        (false, true) => "Chỉ mã index: gần như không nhanh hơn (đọc mã cần ảnh 150 dpi) · trang trắng gán tay (B)",
        (false, false) => "Không quét: chỉ đọc dung lượng + ảnh nhỏ, vài giây · mọi nhãn gán tay",
    }
    .to_string();
    if c.quet_xoay {
        s += " · chiều trang +~15 ms/trang";
    }
    (s, !c.quet_trang || !ma)
}

impl HopCauHinh {
    pub fn moi(cfg: &CauHinh, tab: TabCauHinh) -> Self {
        let mut h = Self {
            tab,
            nhap: cfg.clone(),
            o_trang: String::new(),
            o_nghi: String::new(),
            o_tin_cay: String::new(),
            keo: None,
            mep_chon: None,
            o_he_so: Default::default(),
            o_a0_he_so: String::new(),
            o_thu: String::new(),
            keo_tay: None,
            tay_chon: None,
            co_cache: false,
            xoa_cache: false,
        };
        h.dong_bo_o();
        h
    }

    #[cfg(test)]
    pub fn dat_o_thu(&mut self, s: &str) {
        self.o_thu = s.into();
    }

    fn dong_bo_o(&mut self) {
        self.o_trang = so_o(self.nhap.nguong_trang);
        self.o_nghi = so_o(self.nhap.nguong_nghi);
        self.o_tin_cay = so_o(self.nhap.tin_cay_xoay);
        self.o_he_so = self.nhap.he_so.map(|h| h.to_string().replace('.', ","));
        self.o_a0_he_so = self.nhap.a0_lon_he_so.to_string().replace('.', ",");
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
        for (i, o) in self.o_he_so.iter().enumerate() {
            if doc_so64(o).is_none_or(|v| v <= 0.0) && !l.contains(&LoiCauHinh::HeSo(i)) {
                l.push(LoiCauHinh::HeSo(i));
            }
        }
        // Hệ số cố định chỉ chặn Lưu khi đang chọn
        l.retain(|e| *e != LoiCauHinh::A0LonHeSo || self.nhap.a0_lon == A0Lon::CoDinh);
        if self.nhap.a0_lon == A0Lon::CoDinh && doc_so64(&self.o_a0_he_so).is_none() && !l.contains(&LoiCauHinh::A0LonHeSo) {
            l.push(LoiCauHinh::A0LonHeSo);
        }
        l
    }

    pub fn ve(&mut self, ctx: &egui::Context) -> KetQuaHop {
        let mut kq = KetQuaHop::Mo;
        let phan_hoi = kh::hop(ctx, "hop_cau_hinh", RONG_HOP, "Cấu hình", |ui| {
            let co_cache = self.co_cache;
            let mut bam_xoa = false;
            kh::hang_tab_phai(ui, &[(TabCauHinh::Dem, "Đếm trang A4"), (TabCauHinh::Tach, "Tách PDF")], &mut self.tab, |ui| {
                // Canh phải, thẳng hàng 2 tab: chỉ bấm được khi thư mục đang mở còn cache
                let nut = egui::Button::new(RichText::new("Xoá cache").size(12.0)).min_size(egui::vec2(0.0, 22.0));
                bam_xoa = ui
                    .add_enabled(co_cache, nut)
                    .on_hover_text("Xoá kết quả phân tích đã lưu (cache, cạnh file ứng dụng) của thư mục đang mở. Nhãn đã gán không mất; lần Phân tích sau sẽ phân tích lại mọi trang.")
                    .on_disabled_hover_text("Thư mục đang mở chưa có cache phân tích")
                    .clicked();
            });
            if bam_xoa {
                self.xoa_cache = true;
            }
            kh::than(ui, "than_cau_hinh", CAO_THAN, Margin { left: 20, right: 20, top: 16, bottom: 8 }, |ui| match self.tab {
                TabCauHinh::Tach => self.tab_tach(ui),
                TabCauHinh::Dem => self.tab_dem(ui),
            });
            let loi = self.loi();
            kh::chan(ui, |ui| {
                if ui.add(kh::nut_chan("Mặc định")).on_hover_text("Đưa tab đang mở về giá trị mặc định").clicked() {
                    self.nhap = match self.tab {
                        TabCauHinh::Tach => mac_dinh_tach(&self.nhap),
                        TabCauHinh::Dem => mac_dinh_dem(&self.nhap),
                    };
                    self.dong_bo_o();
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let luu = ui.add_enabled(loi.is_empty(), kh::nut_chan_mau("Lưu", XANH_LA)).on_disabled_hover_text("Sửa ô đang báo đỏ trước");
                    if luu.clicked() {
                        kq = KetQuaHop::Luu(self.nhap.clone());
                    }
                    if ui.add(kh::nut_chan("Huỷ")).clicked() {
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
        nhom(ui, "1. Tự động nhận dạng khi Phân tích", Some("Tắt bớt để Phân tích nhanh hơn khi máy chậm hoặc hồ sơ quá nhiều trang; trang không quét thì gán nhãn bằng tay (I / B)."), |ui| {
            // Trang trắng: 2 ô số + chú thích 3 vùng, thanh khoảng ngay dưới
            let mut focus_nguong = false;
            dong!(
                ui,
                |ui| {
                    kh::o_tick(ui, &mut self.nhap.quet_trang, "Trang trắng");
                    thong_tin(ui, &tn::goi_y(self.nhap.nguong_trang, self.nhap.nguong_nghi));
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_trang, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let r1 = ui.add(kh::o_so(&mut self.o_trang).desired_width(58.0)).on_hover_text("Ngưỡng trắng (%)");
                        ui.label(RichText::new("–").color(CHU_PHU));
                        let r2 = ui.add(kh::o_so(&mut self.o_nghi).desired_width(58.0)).on_hover_text("Ngưỡng nghi trắng (%)");
                        ui.label(RichText::new("%").color(CHU_PHU));
                        if r1.changed() || r2.changed() {
                            if let (Some(t), Some(n)) = (doc_so(&self.o_trang), doc_so(&self.o_nghi)) {
                                (self.nhap.nguong_trang, self.nhap.nguong_nghi) = (t, n);
                            }
                        }
                        focus_nguong = r1.has_focus() || r2.has_focus();
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| chu_thich_vung(ui));
                    });
                },
            );
            let sai = loi.contains(&LoiCauHinh::Nguong);
            ui.add_space(4.0 - ui.spacing().item_spacing.y);
            ui.add_enabled_ui(self.nhap.quet_trang, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(LE_O);
                    self.thanh_khoang(ui, sai);
                });
            });
            ui.add_space(4.0);
            if sai {
                goi_y_duoi(ui, "Số thứ hai phải ≥ số thứ nhất, trong 0–10 %", DO);
            } else if focus_nguong {
                let (t, n) = (tn::so_vn(self.nhap.nguong_trang), tn::so_vn(self.nhap.nguong_nghi));
                goi_y_duoi(ui, &format!("Dưới {t} % là Trắng · {t}–{n} % hoặc có dấu/mực màu là Nghi trắng"), CHU_PHU);
            }
            let mut focus_ma = false;
            dong!(
                ui,
                |ui| {
                    kh::o_tick(ui, &mut self.nhap.quet_ma, "Mã index");
                    thong_tin(ui, "Trang có mã này là tờ index (điểm cắt, bỏ khi tách). Code 128: chấp nhận đọc lệch ≤ 3 ký tự, để trống = mọi mã Code 128 là index. Đổi giá trị → index gán lại ngay, không phân tích lại. QR code, Cụm ký tự: chưa hỗ trợ.");
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_ma, |ui| {
                        o_chon_loai_ma(ui, &mut self.nhap.loai_ma);
                        let goi_y = match self.nhap.loai_ma {
                            LoaiMa::Code128 => "mọi mã Code 128",
                            LoaiMa::Qr => "nội dung QR",
                            LoaiMa::Chu => "cụm ký tự trên trang index",
                        };
                        let r = ui.add(kh::o_nhap(&mut self.nhap.ma_index).desired_width(ui.available_width()).hint_text(goi_y)).on_hover_text("Giá trị mã index");
                        focus_ma = r.has_focus();
                    });
                },
            );
            if self.nhap.quet_ma && self.nhap.loai_ma != LoaiMa::Code128 {
                goi_y_duoi(ui, "Chưa hỗ trợ — Phân tích sẽ không tự tìm trang index (vẫn gán tay được)", VANG_CHU);
            } else if focus_ma {
                goi_y_duoi(ui, "Lệch ≤ 3 ký tự vẫn nhận · trống = mọi mã", CHU_PHU);
            }
            dong!(
                ui,
                |ui| {
                    kh::o_tick(ui, &mut self.nhap.quet_xoay, "Chiều trang");
                    thong_tin(
                        ui,
                        "Mô hình nhận hướng trang gợi ý trang scan bị xoay 90/180/270° vào bộ lọc Nghi xoay; không tự xoay (Enter để xoay theo gợi ý). Độ tin cậy 0–1 mô hình chấm cho mỗi trang: từ mức này trở lên thì kết luận ngay; thấp hơn thì chạy thêm 4 góc xoay, đạt 0,5 mới gắn cờ, còn lại bỏ qua. Tăng: ít cờ hơn, chắc hơn, Phân tích chậm hơn chút. Giảm: nhiều cờ hơn, dễ gợi ý sai. Mẫu 2: 0,8 không sai trang nào.",
                    );
                },
                |ui| {
                    ui.add_enabled_ui(self.nhap.quet_xoay, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let nhan = ui.label(RichText::new("Độ tin cậy ≥").color(CHU_PHU));
                        let khac = (self.nhap.tin_cay_xoay - CauHinh::default().tin_cay_xoay).abs() > 1e-4;
                        let sai = loi.contains(&LoiCauHinh::TinCay);
                        let o = kh::o_so(&mut self.o_tin_cay).desired_width(58.0).text_color(if sai { DO } else if khac { VIEN_TAB_CHON } else { CHU });
                        if ui.add(o).labelled_by(nhan.id).changed() {
                            if let Some(v) = doc_so(&self.o_tin_cay) {
                                self.nhap.tin_cay_xoay = v;
                            }
                        }
                    });
                },
            );
            if loi.contains(&LoiCauHinh::TinCay) {
                goi_y_duoi(ui, &format!("Nhập từ {} đến {}", so_o(TIN_CAY_KHOANG.0), so_o(TIN_CAY_KHOANG.1)), DO);
            }
            let (toc_do, thieu) = uoc_toc_do(&self.nhap);
            ui.add_space(-4.0);
            goi_y_duoi(ui, &toc_do, if thieu { VANG_CHU } else { CHU_PHU });
        });
        nhom(ui, "2. Kết quả tách", None, |ui| {
            let mut focus_ra = false;
            dong!(
                ui,
                |ui| {
                    ui.label("Lưu kết quả");
                },
                |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let r = ui.add(kh::o_nhap(&mut self.nhap.thu_muc_ra).desired_width(ui.available_width() - RONG_NUT_CHON - 4.0).hint_text("cạnh thư mục gốc")).on_hover_text("Thư mục cha chứa <gốc>_vanban");
                    focus_ra = r.has_focus();
                    if ui.add(egui::Button::new("…").min_size(egui::vec2(RONG_NUT_CHON, 24.0))).on_hover_text("Chọn thư mục").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.nhap.thu_muc_ra = p.display().to_string();
                        }
                    }
                },
            );
            if focus_ra {
                let cha = if self.nhap.thu_muc_ra.trim().is_empty() { "…".to_string() } else { self.nhap.thu_muc_ra.trim_end_matches(['/', '\\']).to_string() };
                goi_y_duoi(ui, &format!("Vd: {cha}{}<gốc>_vanban (không được nằm trong thư mục gốc)", std::path::MAIN_SEPARATOR), CHU_PHU);
            }
            let mut focus_ten = false;
            dong!(
                ui,
                |ui| {
                    ui.label("Tên văn bản");
                    thong_tin(ui, "{hoso} tên hồ sơ · {n} số thứ tự · {n3} đệm 0 cho đủ 3 chữ số (001), {n7} đủ 7 chữ số (0000001)… Phải có {n} hoặc {nK}.");
                },
                |ui| {
                    let r = ui.add(kh::o_nhap(&mut self.nhap.mau_ten).desired_width(ui.available_width())).on_hover_text("Mẫu tên văn bản");
                    focus_ten = r.has_focus();
                },
            );
            if loi.contains(&LoiCauHinh::MauTen) {
                goi_y_duoi(ui, "Mẫu phải có {n} hoặc {n3} / {n7}…", DO);
            } else if focus_ten {
                let vd = |n| crate::tach_vb::tach::ten_van_ban(&self.nhap.mau_ten, "HS001", n);
                goi_y_duoi(ui, &format!("Vd: {}, {}", vd(1), vd(2)), CHU_PHU);
            }
            dong!(
                ui,
                |ui| {
                    ui.label("Lưu nhãn");
                    thong_tin(ui, "Ghi index_and_blank.txt (thư mục ẩn .sm-dema4 cạnh file ứng dụng) sau mỗi thay đổi, hoặc khi bấm Lưu nhãn / Ctrl+S.");
                },
                |ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;
                    kh::o_radio(ui, &mut self.nhap.luu_nhan, LuuNhan::TuDong, "Tự động");
                    kh::o_radio(ui, &mut self.nhap.luu_nhan, LuuNhan::BangTay, "Bằng tay (Ctrl+S)");
                },
            );
        });
    }

    fn tab_dem(&mut self, ui: &mut egui::Ui) {
        let loi = self.loi();
        nhom(ui, "1. Hệ số quy A4", Some("Số trang A4 tương đương của 1 trang mỗi khổ (A0 lớn hơn ngưỡng tính theo diện tích thật)"), |ui| {
            // 4 cột đều nhau: nhãn 18 px + ô kéo hết cột
            let cot = (ui.available_width() - 3.0 * 8.0) / 4.0;
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.spacing_mut().interact_size.y = 24.0;
            for hang in [0..4, 4..8] {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for k in hang {
                        let nhan = ui.allocate_ui_with_layout(egui::vec2(21.0, 24.0), egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.set_min_width(21.0);
                            ui.label(RichText::new(KHO[k]).size(11.5).color(CHU_PHU))
                        });
                        let sai = loi.contains(&LoiCauHinh::HeSo(k));
                        let khac = (self.nhap.he_so[k] - crate::dem_pdf::HE_SO_MAC_DINH[k]).abs() > 1e-9;
                        let o = kh::o_so(&mut self.o_he_so[k]).desired_width(cot - 21.0).text_color(if sai { DO } else if khac { VIEN_TAB_CHON } else { CHU });
                        if ui.add(o).labelled_by(nhan.inner.id).changed() {
                            if let Some(v) = doc_so64(&self.o_he_so[k]).filter(|v| *v > 0.0) {
                                self.nhap.he_so[k] = v;
                            }
                        }
                        if k % 4 != 3 {
                            ui.add_space(8.0);
                        }
                    }
                });
            }
            // Lớn hơn A0: vạch đứt ngăn cách, nhãn | 2 lựa chọn, gợi ý dưới cột lựa chọn
            ui.add_space(12.0);
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().extend(egui::Shape::dashed_line(&[r.left_center(), r.right_center()], Stroke::new(1.0, Color32::from_gray(0xE2)), 3.0, 3.0));
            ui.add_space(2.0);
            let mut le_cot = 0.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let trai = ui.min_rect().left();
                ui.label("Lớn hơn A0");
                thong_tin(ui, "Trang A0 có cạnh dài vượt ngưỡng ở thước mục 2 (mặc định 1399 mm). Chọn hệ số cố định (≥ hệ số A0) hoặc quy theo diện tích thật ÷ diện tích A4 (không thấp hơn hệ số A0).");
                ui.add_space(10.0);
                le_cot = ui.cursor().left() - trai;
                kh::o_radio(ui, &mut self.nhap.a0_lon, A0Lon::DienTich, "Theo diện tích ÷ A4");
                ui.add_space(14.0);
                kh::o_radio(ui, &mut self.nhap.a0_lon, A0Lon::CoDinh, "Hệ số cố định");
                ui.add_space(2.0);
                ui.add_enabled_ui(self.nhap.a0_lon == A0Lon::CoDinh, |ui| {
                    let sai = loi.contains(&LoiCauHinh::A0LonHeSo);
                    let o = kh::o_so(&mut self.o_a0_he_so).desired_width(64.0).text_color(if sai { DO } else { CHU });
                    if ui.add(o).on_hover_text("Hệ số cố định cho trang A0 lớn").changed() {
                        if let Some(v) = doc_so64(&self.o_a0_he_so) {
                            self.nhap.a0_lon_he_so = v;
                        }
                    }
                });
            });
            let (goi_y, mau) = if loi.contains(&LoiCauHinh::A0LonHeSo) {
                (format!("Hệ số phải ≥ hệ số A0 ({})", self.o_he_so[0]), DO)
            } else if self.nhap.a0_lon == A0Lon::CoDinh {
                (format!("Mọi trang A0 lớn tính {} trang A4", self.o_a0_he_so), CHU_PHU)
            } else {
                let q = self.nhap.quy_tac().quy_a4(0, 900.0 * PT_MOI_MM, 1500.0 * PT_MOI_MM);
                (format!("Vd trang 1500 × 900 mm = {} trang A4", tn::so_vn(q as f32)), CHU_PHU)
            };
            ui.add_space(-4.0);
            dong_le(ui, le_cot, &goi_y, mau);
        });
        nhom(
            ui,
            "2. Ranh giới khổ theo cạnh dài (mm)",
            Some("Kéo vạch đen giữa 2 khổ: trang có cạnh dài nằm trong đoạn nào thì tính khổ đó. Vạch chấm = cạnh dài chuẩn. Vạch nâu cuối = ngưỡng A0 lớn quy A4 theo diện tích thật (mặc định 1399 = 1189 + 210 mm)."),
            |ui| {
                self.thuoc_kho(ui);
                chu_thich_thuoc(ui);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let nhan = ui.label(RichText::new("Thử cạnh dài").color(CHU_PHU));
                    ui.add(kh::o_so(&mut self.o_thu).desired_width(80.0).hint_text("mm")).labelled_by(nhan.id).on_hover_text("Cạnh dài trang (mm)");
                    if let Some(mm) = doc_so(&self.o_thu).filter(|v| *v > 0.0) {
                        let co_dinh = (self.nhap.a0_lon == A0Lon::CoDinh).then_some(self.o_a0_he_so.as_str());
                        ui.label(RichText::new("→").color(CHU_PHU));
                        ui.label(RichText::new(tk::thu(mm as f64, &self.nhap.ranh_gioi, self.nhap.a0_nguong_mm, co_dinh)).dam());
                    }
                });
            },
        );
    }

    /// Thước ranh giới khổ: 7 tay ranh giới + tay nâu ngưỡng A0 lớn; ô Thử → vạch đỏ đứt.
    fn thuoc_kho(&mut self, ui: &mut egui::Ui) {
        let rong = ui.available_width() - 20.0;
        let (o, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 8.0 + 88.0), Sense::hover());
        let goc = o.min + egui::vec2(10.0, 8.0);
        let dai = Rect::from_min_size(goc + egui::vec2(0.0, 20.0), egui::vec2(rong, 22.0));
        let x_cua = |mm: f32| dai.left() + tk::vi_tri(mm) * dai.width();
        let id = ui.make_persistent_id("thuoc_kho");
        let gia_tri = |h: &Self, i: usize| if i == 7 { h.nhap.a0_nguong_mm as f32 } else { h.nhap.ranh_gioi[i] as f32 };
        let vi_tri = ui.input(|i| i.pointer.interact_pos());
        let mut hover = None;
        for i in 0..8 {
            let r = Rect::from_min_max(egui::pos2(x_cua(gia_tri(self, i)) - 6.0, dai.top() - 4.0), egui::pos2(x_cua(gia_tri(self, i)) + 7.0, dai.top() + 34.0));
            let p = ui.interact(r, id.with(i), Sense::click_and_drag());
            if p.hovered() {
                hover = Some(i);
            }
            if p.drag_started() {
                self.keo_tay = Some(i);
            }
            if p.drag_started() || p.clicked() {
                self.tay_chon = Some(i);
            }
            p.on_hover_text(if i == 7 {
                "Trên ngưỡng này trang A0 quy A4 theo diện tích thật (mặc định 1399 = 1189 + 210 mm) · kéo hoặc ←/→".to_string()
            } else {
                format!("Ranh giới {} | {} (mặc định {} mm) · kéo hoặc ←/→", KHO[i], KHO[i + 1], tn::so_vn(RANH_GIOI_MAC_DINH[i] as f32))
            });
        }
        let dat = |h: &mut Self, i: usize, mm: f32| {
            let mm = tk::dat(i, mm) as f64;
            if i == 7 {
                h.nhap.a0_nguong_mm = mm;
            } else {
                h.nhap.ranh_gioi[i] = mm;
            }
        };
        if let (Some(i), Some(p)) = (self.keo_tay, vi_tri) {
            if ui.input(|i| i.pointer.primary_down()) {
                dat(self, i, tk::mm_tai((p.x - dai.left()) / dai.width()));
            } else {
                self.keo_tay = None;
            }
        }
        if let Some(i) = self.tay_chon {
            if !ui.ctx().egui_wants_keyboard_input() {
                let (trai, phai, lon) = ui.input(|k| (k.key_pressed(egui::Key::ArrowLeft), k.key_pressed(egui::Key::ArrowRight), k.modifiers.shift));
                if trai || phai {
                    let v = tk::phim(i, gia_tri(self, i), phai, lon);
                    dat(self, i, v);
                }
            }
        }
        // Dải trắng viền xám (mỗi đoạn 1 khổ), đoạn A0 lớn gạch chéo, cạnh chuẩn chấm, tay kéo + tam giác + số mm
        let p = ui.painter();
        let xa0 = x_cua(self.nhap.a0_nguong_mm as f32);
        p.rect_filled(dai, 0.0, Color32::WHITE);
        ve_gach_cheo(p, Rect::from_x_y_ranges(xa0..=dai.right(), dai.y_range()), 7.0, 2.0, Color32::from_gray(0xED));
        p.text(egui::pos2((xa0 + dai.right()) / 2.0, dai.center().y), egui::Align2::CENTER_CENTER, "A0 lớn", egui::FontId::proportional(10.5), Color32::from_gray(0x55));
        let vien = Stroke::new(1.0, VIEN_THUOC);
        p.hline(dai.x_range(), dai.top(), vien);
        p.hline(dai.x_range(), dai.bottom(), vien);
        for x in [dai.left(), dai.right()].into_iter().chain((0..7).map(|i| x_cua(self.nhap.ranh_gioi[i] as f32))) {
            p.vline(x, dai.y_range(), vien);
        }
        let chu_chuan = egui::FontId::proportional(10.0);
        for (k, &c) in CANH_CHUAN_MM.iter().enumerate() {
            let xc = x_cua(c as f32);
            p.extend(egui::Shape::dotted_line(&[egui::pos2(xc, dai.top() - 8.0), egui::pos2(xc, dai.bottom())], Color32::from_gray(0x8A), 2.0, 0.5));
            p.text(egui::pos2(xc, dai.top() - 10.0), egui::Align2::CENTER_BOTTOM, format!("{} {}", KHO[k], c as u32), chu_chuan.clone(), Color32::from_gray(0x66));
        }
        if let Some(mm) = doc_so(&self.o_thu).filter(|v| *v > 0.0) {
            let xt = x_cua(mm).clamp(dai.left(), dai.right());
            p.extend(egui::Shape::dashed_line(&[egui::pos2(xt, dai.top() - 8.0), egui::pos2(xt, dai.bottom() + 8.0)], Stroke::new(2.0, Color32::from_rgb(0xC0, 0x39, 0x2B)), 4.0, 3.0));
        }
        for i in 0..8 {
            let v = gia_tri(self, i);
            let xt = x_cua(v);
            let sang = hover == Some(i) || self.keo_tay == Some(i) || self.tay_chon == Some(i);
            let mau = if sang { VIEN_TAB_CHON } else if i == 7 { NAU } else { Color32::from_gray(0x1F) };
            p.vline(xt, (dai.top() - 4.0)..=(dai.top() + 26.0), Stroke::new(2.0, mau));
            let (dinh, day) = (dai.top() + 29.0, dai.top() + 34.0);
            p.add(egui::Shape::convex_polygon(vec![egui::pos2(xt, dinh), egui::pos2(xt + 5.0, day), egui::pos2(xt - 5.0, day)], mau, Stroke::NONE));
            let md = if i == 7 { crate::cau_hinh::A0_NGUONG_MAC_DINH as f32 } else { RANH_GIOI_MAC_DINH[i] as f32 };
            let doi = (v - md).abs() > 1e-3;
            let chu = if doi { egui::FontId::new(10.5, egui::FontFamily::Name(HO_FONT_DAM.into())) } else { egui::FontId::proportional(10.5) };
            p.text(egui::pos2(xt, dai.top() + 36.0), egui::Align2::CENTER_TOP, tn::so_vn(v), chu, if doi { VIEN_TAB_CHON } else { CHU });
        }
        if hover.is_some() || self.keo_tay.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }

    /// Thanh khoảng 2 ngưỡng: kéo mép, kéo cả khung, bấm thước, ←/→ (Shift = 0,1).
    fn thanh_khoang(&mut self, ui: &mut egui::Ui, sai: bool) {
        let rong = ui.available_width();
        let (vung, _) = ui.allocate_exact_size(egui::vec2(rong, 44.0), Sense::hover());
        let thuoc = Rect::from_min_size(vung.min + egui::vec2(0.0, 16.0), egui::vec2(rong, 12.0));
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
        // Vẽ: mốc + vạch nhỏ, 3 vùng (trắng · caro · xám), khung kính, 2 mép, 2 số
        let (t, n) = (self.nhap.nguong_trang, self.nhap.nguong_nghi);
        let mo = if sai || !ui.is_enabled() { 0.4 } else { 1.0 };
        let p = ui.painter();
        for m in tn::MOC {
            let x = x_cua(m);
            p.text(egui::pos2(x, vung.top()), egui::Align2::CENTER_TOP, tn::so_vn(m), egui::FontId::proportional(9.0), Color32::from_gray(0x99).gamma_multiply(mo));
            p.vline(x, (vung.top() + 11.0)..=(vung.top() + 15.0), Stroke::new(1.0, Color32::from_gray(0xC8).gamma_multiply(mo)));
        }
        let (xt, xn) = (x_cua(t), x_cua(n));
        let vien = Stroke::new(1.0, Color32::from_gray(0xBD).gamma_multiply(mo));
        p.rect_filled(Rect::from_x_y_ranges(thuoc.left()..=xt, thuoc.y_range()), CornerRadius { nw: 2, sw: 2, ne: 0, se: 0 }, Color32::WHITE);
        ve_caro(p, Rect::from_x_y_ranges(xt..=xn, thuoc.y_range()), 4.0, Color32::from_gray(0xA8).gamma_multiply(mo));
        p.rect_filled(Rect::from_x_y_ranges(xn..=thuoc.right(), thuoc.y_range()), CornerRadius { nw: 0, sw: 0, ne: 2, se: 2 }, Color32::from_gray(0x8C).gamma_multiply(mo));
        p.hline(thuoc.left()..=xn, thuoc.top(), vien);
        p.hline(thuoc.left()..=xn, thuoc.bottom(), vien);
        p.vline(thuoc.left(), thuoc.y_range(), vien);
        p.rect_stroke(Rect::from_x_y_ranges(xt..=xn, (vung.top() + 13.0)..=(vung.top() + 31.0)), 2.0, Stroke::new(1.0, Color32::from_gray(0x5A).gamma_multiply(mo)), StrokeKind::Inside);
        let md = CauHinh::default();
        for (x, pr, mep) in [(xt, &p_trai, Mep::Trai), (xn, &p_phai, Mep::Phai)] {
            let sang = pr.hovered() || pr.dragged() || self.mep_chon == Some(mep);
            let mau = if sang { VIEN_TAB_CHON } else { Color32::from_rgba_unmultiplied(70, 70, 70, 140) };
            p.vline(x, (vung.top() + 13.0)..=(vung.top() + 31.0), Stroke::new(2.0, mau.gamma_multiply(mo)));
        }
        let so = |v: f32, v_md: f32, x: f32, duoi: &str| {
            let doi = (v - v_md).abs() > 1e-4;
            let chu = if doi { egui::FontId::new(10.0, egui::FontFamily::Name(HO_FONT_DAM.into())) } else { egui::FontId::proportional(10.0) };
            p.text(egui::pos2(x, vung.top() + 33.0), egui::Align2::CENTER_TOP, format!("{}{duoi}", so_o(v)), chu, (if doi { VIEN_TAB_CHON } else { CHU }).gamma_multiply(mo));
        };
        so(t, md.nguong_trang, xt, "");
        // Số thứ hai cách số đầu ít nhất 15 % bề ngang để không đè nhau
        so(n, md.nguong_nghi, xn.max(xt + 0.15 * thuoc.width()), " %");
        if pr_hover(&p_trai, &p_phai, &p_khung) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }
}

fn pr_hover(a: &egui::Response, b: &egui::Response, c: &egui::Response) -> bool {
    a.hovered() || b.hovered() || a.dragged() || b.dragged() || c.dragged() || c.hovered()
}

/// Ô chọn loại mã: nền trắng, mũi tên ⌄ như ô select của prototype.
fn o_chon_loai_ma(ui: &mut egui::Ui, loai: &mut LoaiMa) {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            v.weak_bg_fill = Color32::WHITE;
            v.bg_fill = Color32::WHITE;
        }
        egui::ComboBox::from_id_salt("loai_ma")
            .width(111.0)
            .selected_text(match loai {
                LoaiMa::Code128 => "Code 128",
                LoaiMa::Qr => "QR code",
                LoaiMa::Chu => "Cụm ký tự",
            })
            .icon(|ui, r, _, _| {
                let c = r.center();
                let mau = if ui.is_enabled() { CHU } else { CHU_PHU };
                ui.painter().add(egui::Shape::line(vec![c + egui::vec2(-4.0, -2.0), c + egui::vec2(0.0, 2.0), c + egui::vec2(4.0, -2.0)], Stroke::new(1.5, mau)));
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(loai, LoaiMa::Code128, "Code 128");
                ui.selectable_value(loai, LoaiMa::Qr, "QR code");
                ui.selectable_value(loai, LoaiMa::Chu, "Cụm ký tự");
            })
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Loại mã index"));
    });
}

/// Nhóm khung viền nhạt nền #FCFCFC: tiêu đề đậm (+ ⓘ) có gạch dưới, các dòng cách nhau 8 px.
fn nhom(ui: &mut egui::Ui, tieu_de: &str, giai_thich: Option<&str>, noi_dung: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(NEN_NHOM).stroke(Stroke::new(1.0, VIEN_NHAT)).corner_radius(5).inner_margin(Margin { left: 12, right: 12, top: 10, bottom: 4 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.scope(|ui| {
            // Dòng tiêu đề chỉ cao theo chữ, không theo chiều cao ô nhập
            ui.spacing_mut().interact_size.y = 0.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(RichText::new(tieu_de).dam().size(12.5));
                if let Some(g) = giai_thich {
                    thong_tin(ui, g);
                }
            });
        });
        ui.add_space(6.0);
        kh::ke_ngang(ui, Color32::from_gray(0xED));
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        // Dòng `nhãn | ô` cao 26 như prototype
        ui.spacing_mut().interact_size.y = 26.0;
        noi_dung(ui);
    });
    ui.add_space(12.0 - ui.spacing().item_spacing.y);
}

/// Cột nhãn cố định của dòng `nhãn | ô`.
fn cot_nhan(ui: &mut egui::Ui, nhan: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(egui::vec2(RONG_NHAN, 24.0), egui::Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_width(RONG_NHAN);
        ui.spacing_mut().item_spacing.x = 4.0;
        nhan(ui);
    });
    ui.add_space(LE_O - RONG_NHAN - ui.spacing().item_spacing.x);
}

/// Dòng chữ nhỏ (11 px) thụt theo cột ô, tự xuống dòng.
fn goi_y_duoi(ui: &mut egui::Ui, chu: &str, mau: Color32) {
    ui.add_space(4.0 - ui.spacing().item_spacing.y);
    dong_le(ui, LE_O, chu, mau);
}

fn dong_le(ui: &mut egui::Ui, le: f32, chu: &str, mau: Color32) {
    ui.horizontal(|ui| {
        ui.add_space(le);
        ui.add(egui::Label::new(RichText::new(chu).size(11.0).color(mau)).wrap());
    });
}

/// Gạch chéo 135° trên nền trắng: `buoc` px một chu kỳ, nét `day` px.
fn ve_gach_cheo(p: &egui::Painter, r: Rect, buoc: f32, day: f32, mau: Color32) {
    if r.width() <= 0.0 {
        return;
    }
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    let mut x = r.left() - r.height();
    while x < r.right() + r.height() {
        p.line_segment([egui::pos2(x, r.bottom()), egui::pos2(x + r.height(), r.top())], Stroke::new(day, mau));
        x += buoc;
    }
}

/// Caro ô vuông `o` px: ô chẵn màu `mau`, ô lẻ trắng.
fn ve_caro(p: &egui::Painter, r: Rect, o: f32, mau: Color32) {
    p.rect_filled(r, 0.0, Color32::WHITE);
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (mut cot, mut x) = (0, r.left());
    while x < r.right() {
        let mut hang = 0;
        let mut y = r.top();
        while y < r.bottom() {
            if (cot + hang) % 2 == 1 {
                p.rect_filled(Rect::from_min_size(egui::pos2(x, y), egui::vec2(o, o)), 0.0, mau);
            }
            y += o;
            hang += 1;
        }
        x += o;
        cot += 1;
    }
}

/// Chú thích thước khổ: cạnh dài chuẩn · ranh giới · A0 lớn.
fn chu_thich_thuoc(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.x = 4.0;
        let chu = |ui: &mut egui::Ui, s: &str| ui.label(RichText::new(s).size(11.0).color(CHU_PHU));
        let (r, _) = ui.allocate_exact_size(egui::vec2(1.0, 12.0), Sense::hover());
        ui.painter().extend(egui::Shape::dotted_line(&[r.center_top(), r.center_bottom()], Color32::from_gray(0x66), 2.0, 0.5));
        chu(ui, "cạnh dài chuẩn");
        ui.add_space(12.0);
        let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 12.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, Color32::from_gray(0x1F));
        chu(ui, "ranh giới (kéo được)");
        ui.add_space(12.0);
        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, Color32::WHITE);
        ve_gach_cheo(ui.painter(), r, 5.0, 2.0, Color32::from_gray(0xDD));
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, VIEN_THUOC), StrokeKind::Inside);
        chu(ui, "A0 lớn: cách quy A4 chọn ở mục 1");
    });
}

/// 3 ô mẫu: Trắng · Nghi trắng · Có chữ (vẽ phải → trái).
fn chu_thich_vung(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 5.0;
    for (ten, kieu) in [("Có chữ", 2), ("Nghi trắng", 1), ("Trắng", 0)] {
        ui.label(RichText::new(ten).size(11.5));
        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 11.0), Sense::hover());
        let p = ui.painter();
        match kieu {
            0 => {
                p.rect_filled(r, 1.0, Color32::WHITE);
            }
            1 => ve_caro(p, r, 4.0, Color32::from_gray(0xA8)),
            _ => {
                p.rect_filled(r, 1.0, Color32::from_gray(0x8C));
            }
        }
        p.rect_stroke(r, 1.0, Stroke::new(1.0, if kieu == 2 { Color32::from_gray(0x8C) } else { VIEN_THUOC }), StrokeKind::Inside);
        ui.add_space(4.0);
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
        assert_eq!((m.nguong_trang, m.mau_ten.as_str()), (1.0, "{hoso}.{n}"));
    }

    #[test]
    fn mac_dinh_dem_va_o_he_so_sai() {
        let c = CauHinh { he_so: [2.0; 8], nguong_trang: 2.0, ..Default::default() };
        let m = mac_dinh_dem(&c);
        assert_eq!((m.he_so, m.nguong_trang), (crate::dem_pdf::HE_SO_MAC_DINH, 2.0));
        let mut h = HopCauHinh::moi(&CauHinh::default(), TabCauHinh::Dem);
        assert_eq!(h.o_he_so[5], "0,5");
        h.o_he_so[5] = "x".into();
        assert_eq!(h.loi(), [LoiCauHinh::HeSo(5)]);
        h.o_he_so[5] = "0,5".into();
        // Hệ số cố định < A0 chỉ lỗi khi đang chọn "Hệ số cố định"
        h.nhap.a0_lon_he_so = 10.0;
        assert!(h.loi().is_empty());
        h.nhap.a0_lon = A0Lon::CoDinh;
        assert_eq!(h.loi(), [LoiCauHinh::A0LonHeSo]);
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
        assert_eq!(uoc_toc_do(&CauHinh::default()), ("Quét cả hai: ~120 ms/trang/lõi (100.000 trang ≈ 25 phút trên 8 lõi) · chiều trang +~15 ms/trang".to_string(), false));
        let (s, thieu) = uoc_toc_do(&CauHinh { quet_ma: false, quet_xoay: false, ..Default::default() });
        assert!(s.starts_with("Chỉ trang trắng") && thieu);
        assert!(uoc_toc_do(&CauHinh { loai_ma: LoaiMa::Qr, ..Default::default() }).1, "QR chưa hỗ trợ = như tắt mã");
    }
}
