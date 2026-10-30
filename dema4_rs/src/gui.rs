//! Giao diện đếm trang A4, tông xám như bản Python (dema4.py):
//! hàng chọn thư mục, hàng hệ số, dòng tóm tắt, bộ lọc theo khổ, bảng từng trang và thanh tiến độ.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke, TextStyle};
use egui_extras::{Column, TableBuilder};

use crate::dem_pdf::{self, KetQuaDem, SuKien, KHO};

const TIEU_DE: &str = "Đếm trang A4 (Giải pháp số Sao Mai)";
/// Số sự kiện xử lý tối đa mỗi khung hình để giao diện không bị đơ khi đếm nhanh
const SU_KIEN_MOI_KHUNG: usize = 20_000;
pub(crate) const RONG_NUT: f32 = 80.0;
/// Dòng báo công khai ở góc dưới khi có cấu hình Telegram; chỉ đổi sang chữ đỏ khi gửi lỗi
const THONG_BAO_THONG_KE: &str = "Thông tin đếm thư mục đã được ghi lại";
/// Số cột của bảng chi tiết
const SO_COT: usize = 10;

// Bảng màu xám kiểu Windows cổ điển; chỉ nút Đếm/Dừng và chữ cảnh báo có màu
pub(crate) const NEN: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub(crate) const VIEN: Color32 = Color32::from_rgb(0xAD, 0xAD, 0xAD);
pub(crate) const VIEN_NHAT: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
const NUT: Color32 = Color32::from_rgb(0xE1, 0xE1, 0xE1);
pub(crate) const NUT_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
pub(crate) const CHU: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
pub(crate) const CHU_PHU: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);
pub(crate) const XANH_LA: Color32 = Color32::from_rgb(0x00, 0x80, 0x00);
pub(crate) const DO: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);
const DO_SAM: Color32 = Color32::from_rgb(0xA0, 0x10, 0x10);
pub(crate) const CHU_MO: Color32 = Color32::from_rgb(0xA8, 0xA8, 0xA8);
/// Màu phần đã chạy của thanh tiến độ (xanh nhạt cho dễ nhìn)
pub(crate) const XAM_TIEN_DO: Color32 = Color32::from_rgb(0xA8, 0xD1, 0xF5);
pub(crate) const XANH_COBALT: Color32 = Color32::from_rgb(0x00, 0x47, 0xAB);

pub fn chay() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]).with_min_inner_size([800.0, 420.0]).with_title(TIEU_DE);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../icon.png")) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native(
        TIEU_DE,
        options,
        Box::new(|cc| {
            cai_giao_dien(&cc.egui_ctx);
            let app = UngDungChinh::default();
            app.dem.license.mo_app();
            Ok(Box::new(app))
        }),
    )
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Man {
    Dem,
    Tach,
    MucLuc,
    Ky,
}

impl Man {
    fn tieu_de(self) -> &'static str {
        match self {
            Man::Dem => TIEU_DE,
            Man::Tach => "Tách văn bản PDF (Giải pháp số Sao Mai)",
            Man::MucLuc => crate::gui_muc_luc::TIEU_DE,
            Man::Ky => "Ký văn bản (Giải pháp số Sao Mai)",
        }
    }
}

/// Cửa sổ chính: menu Chức năng + các màn (Đếm, Tách, Ký giữ chỗ).
struct UngDungChinh {
    man: Man,
    tieu_de_da_dat: Man,
    dem: UngDung,
    tach: crate::gui_tach::ManTach,
    muc_luc: crate::gui_muc_luc::ManMucLuc,
    cfg: crate::cau_hinh::CauHinh,
    hop_cau_hinh: Option<crate::gui_tach::hop_cau_hinh::HopCauHinh>,
    tro_giup: Option<crate::tro_giup::HopTroGiup>,
    /// Đóng app khi nhãn chưa lưu (lưu bằng tay): đang hỏi Lưu / Bỏ / Huỷ; `cho_dong` = đã trả lời, cho đóng.
    hoi_dong: bool,
    /// Đóng app khi đang phân tích / tách: hỏi Thoát luôn / Huỷ (thoát giữa chừng mất kết quả đang ghi dở).
    hoi_dong_chay: bool,
    cho_dong: bool,
}

impl Default for UngDungChinh {
    fn default() -> Self {
        let cfg = crate::cau_hinh::doc();
        let dem = UngDung { quy_tac: cfg.quy_tac(), ..Default::default() };
        let mut tach = crate::gui_tach::ManTach::default();
        tach.license = dem.license.clone();
        let mut muc_luc = crate::gui_muc_luc::ManMucLuc::tu_cau_hinh(&cfg.muc_luc);
        muc_luc.license = dem.license.clone();
        Self { man: Man::Dem, tieu_de_da_dat: Man::Dem, dem, tach, muc_luc, cfg, hop_cau_hinh: None, tro_giup: None, hoi_dong: false, hoi_dong_chay: false, cho_dong: false }
    }
}

impl UngDungChinh {
    fn tab_huong_dan(&self) -> crate::tro_giup::TabHuongDan {
        match self.man {
            Man::Tach => crate::tro_giup::TabHuongDan::Tach,
            Man::MucLuc => crate::tro_giup::TabHuongDan::MucLuc,
            _ => crate::tro_giup::TabHuongDan::Dem,
        }
    }

    /// Hỏi Lưu / Bỏ / Huỷ khi đóng app còn nhãn chưa lưu.
    fn hop_hoi_dong_chay(&mut self, ctx: &egui::Context) {
        if !self.hoi_dong_chay {
            return;
        }
        let mut chon = None;
        use crate::gui_tach::khung_hop as kh;
        let viec = if self.muc_luc.dang_chay() {
            "sinh mục lục"
        } else if self.tach.dang_tach() {
            "tách văn bản"
        } else {
            "phân tích"
        };
        kh::hop(ctx, "hoi_dong_chay", 360.0, "Đang xử lý", |ui| {
            kh::than_chu(ui, &format!("Đang {viec}. Thoát ngay thì phần đang làm dở sẽ mất; nên bấm *Dừng*, đợi xong rồi mới thoát."));
            kh::chan(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(kh::nut_chan_mau("Thoát luôn", DO)).clicked() {
                        chon = Some(true);
                    }
                    if ui.add(kh::nut_chan("Huỷ")).clicked() {
                        chon = Some(false);
                    }
                });
            });
        });
        if let Some(thoat) = chon {
            self.hoi_dong_chay = false;
            if thoat {
                self.cho_dong = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn hop_hoi_dong(&mut self, ctx: &egui::Context) {
        if !self.hoi_dong {
            return;
        }
        let mut chon = None;
        use crate::gui_tach::khung_hop as kh;
        kh::hop(ctx, "hoi_dong", 340.0, "Nhãn chưa lưu", |ui| {
            kh::than_chu(ui, "Có thay đổi nhãn chưa ghi vào index_and_blank.txt. Lưu trước khi đóng?");
            kh::chan(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(kh::nut_chan_mau("Lưu", XANH_LA)).clicked() {
                        chon = Some(1);
                    }
                    if ui.add(kh::nut_chan("Bỏ")).clicked() {
                        chon = Some(2);
                    }
                    if ui.add(kh::nut_chan("Huỷ")).clicked() {
                        chon = Some(0);
                    }
                });
            });
        });
        if let Some(c) = chon {
            self.hoi_dong = false;
            if c > 0 {
                if c == 1 {
                    self.tach.luu_nhan();
                }
                self.cho_dong = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Áp Cấu hình mới cho cả 2 màn (`ghi` = ghi `.dema4_saomai.cfg`).
    fn ap_cau_hinh(&mut self, moi: crate::cau_hinh::CauHinh, ghi: bool) {
        self.tach.ap_cau_hinh(&self.cfg, &moi);
        self.dem.quy_tac = moi.quy_tac();
        self.cfg = moi;
        if ghi {
            let _ = crate::cau_hinh::ghi(&self.cfg);
        }
    }

    fn dang_chay(&self) -> bool {
        self.dem.dang_dem.is_some() || self.tach.dang_chay() || self.muc_luc.dang_chay()
    }

    fn thanh_menu(&mut self, ui: &mut egui::Ui) {
        let khoa = self.dang_chay();
        use crate::gui_tach::bieu_tuong::{atom_mau, BieuTuong};
        // Icon xám nét đơn ở đầu mỗi mục (lưới · bánh răng · dấu hỏi)
        let xam = Color32::from_gray(0x55);
        // Thanh menu như prototype: nền #F7F7F7, viền dưới, nút có mũi ▾ khi mở menu con
        let r = egui::Panel::top("menu").frame(egui::Frame::new().fill(NEN_TAB).inner_margin(Margin::symmetric(6, 1))).show(ui, |ui| {
            // Như prototype: chữ 13 px, mỗi mục đệm 12 px hai bên, icon cách chữ 6 px, cao 28 px
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.spacing_mut().icon_spacing = 6.0;
            ui.spacing_mut().button_padding = egui::vec2(12.0, 5.0);
            ui.spacing_mut().interact_size.y = 26.0;
            for kieu in [TextStyle::Body, TextStyle::Button] {
                ui.style_mut().text_styles.insert(kieu, egui::FontId::proportional(13.0));
            }
            egui::MenuBar::new().ui(ui, |ui| {
                // MenuBar đặt lại khoảng cách: đệm 12 px mỗi bên mục = 24 px giữa 2 mục, lề trái 12 px
                ui.spacing_mut().item_spacing.x = 24.0;
                ui.add_space(12.0);
                ui.add_enabled_ui(!khoa, |ui| {
                    egui::containers::menu::MenuButton::from_button(egui::Button::new((atom_mau(BieuTuong::MenuLuoi, 14.0, xam), "Chức năng", atom_mau(BieuTuong::MuiTenXuong, 13.0, CHU)))).ui(ui, |ui| {
                        use crate::gui_tach::bieu_tuong::{muc_menu_day, BieuTuong};
                        for (man, ten, icon) in [(Man::Dem, "Đếm trang A4", BieuTuong::TaiLieu), (Man::Tach, "Tách văn bản", BieuTuong::Keo), (Man::MucLuc, "Sinh mục lục", BieuTuong::DanhSachSo), (Man::Ky, "Ký văn bản", BieuTuong::But)] {
                            if muc_menu_day(ui, icon, ten, self.man == man, None, 190.0).clicked() {
                                self.man = man;
                                ui.close();
                            }
                        }
                    });
                })
                .response
                .on_disabled_hover_text("Đang chạy — dừng hoặc chờ xong để đổi chức năng");
                let cau_hinh = ui.add_enabled(!khoa, egui::Button::new((atom_mau(BieuTuong::BanhRang, 14.0, xam), "Cấu hình")).frame(false)).on_disabled_hover_text("Đang chạy — dừng hoặc chờ xong để sửa Cấu hình");
                if cau_hinh.clicked() {
                    use crate::gui_tach::hop_cau_hinh::{HopCauHinh, TabCauHinh};
                    let tab = match self.man {
                        Man::Dem => TabCauHinh::Dem,
                        Man::MucLuc => TabCauHinh::MucLuc,
                        _ => TabCauHinh::Tach,
                    };
                    let mut hop = HopCauHinh::moi(&self.cfg, tab);
                    hop.co_cache = self.tach.co_cache();
                    self.hop_cau_hinh = Some(hop);
                }
                egui::containers::menu::MenuButton::from_button(egui::Button::new((atom_mau(BieuTuong::HoiDap, 14.0, xam), "Trợ giúp", atom_mau(BieuTuong::MuiTenXuong, 13.0, CHU)))).ui(ui, |ui| {
                    use crate::gui_tach::bieu_tuong::{muc_menu_day, BieuTuong};
                    use crate::tro_giup::HopTroGiup;
                    if muc_menu_day(ui, BieuTuong::CapNhat, "Kiểm tra cập nhật…", false, None, 210.0).clicked() {
                        self.tro_giup = Some(HopTroGiup::cap_nhat());
                        ui.close();
                    }
                    if muc_menu_day(ui, BieuTuong::HoiDap, "Hướng dẫn sử dụng", false, Some("F1"), 210.0).clicked() {
                        self.tro_giup = Some(HopTroGiup::HuongDan(self.tab_huong_dan()));
                        ui.close();
                    }
                    if muc_menu_day(ui, BieuTuong::ThongTin, "Thông tin", false, None, 210.0).clicked() {
                        self.tro_giup = Some(HopTroGiup::ThongTin);
                        ui.close();
                    }
                });
            });
        });
        ui.painter().hline(r.response.rect.x_range(), r.response.rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    }
}

impl eframe::App for UngDungChinh {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.tieu_de_da_dat != self.man {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.man.tieu_de().into()));
            self.tieu_de_da_dat = self.man;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.cho_dong {
                self.tach.ghi_cache();
            } else if self.tach.dang_chay() || self.muc_luc.dang_chay() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hoi_dong_chay = true;
            } else if self.tach.chua_luu() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hoi_dong = true;
            } else {
                self.tach.ghi_cache();
            }
        }
        self.hop_hoi_dong(&ctx);
        self.hop_hoi_dong_chay(&ctx);
        if self.tro_giup.is_none() && self.hop_cau_hinh.is_none() && ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.tro_giup = Some(crate::tro_giup::HopTroGiup::HuongDan(self.tab_huong_dan()));
        }
        self.thanh_menu(ui);
        match self.man {
            Man::Dem => self.dem.ve(ui),
            Man::Tach => {
                egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 0 })).show(ui, |ui| self.tach.ve(ui, &mut self.cfg));
                if std::mem::take(&mut self.tach.cfg_doi) {
                    let _ = crate::cau_hinh::ghi(&self.cfg);
                }
            }
            Man::MucLuc => {
                let cfg_ml = self.cfg.muc_luc.clone();
                egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 0 })).show(ui, |ui| self.muc_luc.ve(ui, &cfg_ml));
                if std::mem::take(&mut self.muc_luc.cfg_doi) {
                    self.muc_luc.ghi_vao(&mut self.cfg.muc_luc);
                    let _ = crate::cau_hinh::ghi(&self.cfg);
                }
            }
            Man::Ky => {
                egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN)).show(ui, |ui| {
                    crate::gioi_thieu::ve(ui, &crate::gioi_thieu::KY);
                });
            }
        }
        if self.tro_giup.as_mut().is_some_and(|h| h.ve(&ctx)) {
            self.tro_giup = None;
        }
        if let Some(hop) = &mut self.hop_cau_hinh {
            use crate::gui_tach::hop_cau_hinh::KetQuaHop;
            let kq = hop.ve(&ctx);
            if std::mem::take(&mut hop.xoa_cache) {
                self.tach.xoa_cache();
                hop.co_cache = self.tach.co_cache();
            }
            match kq {
                KetQuaHop::Mo => {}
                KetQuaHop::Huy => self.hop_cau_hinh = None,
                KetQuaHop::Luu(moi) => {
                    self.ap_cau_hinh(moi, true);
                    self.hop_cau_hinh = None;
                }
            }
        }
    }
}

/// Tên họ font đậm (egui không có chữ đậm sẵn, strong() chỉ đổi màu)
pub(crate) const HO_FONT_DAM: &str = "dam";

/// In đậm thật bằng họ font đậm của hệ thống
pub(crate) trait ChuDam {
    fn dam(self) -> Self;
}

impl ChuDam for RichText {
    fn dam(self) -> Self {
        self.family(egui::FontFamily::Name(HO_FONT_DAM.into()))
    }
}

/// Tông xám, font hệ thống có dấu tiếng Việt; áp cho cả chế độ sáng và tối của hệ điều hành
fn cai_giao_dien(ctx: &egui::Context) {
    // Ưu tiên Arial trên mọi hệ điều hành
    const FONT_THUONG: [&str; 5] = [
        r"C:\Windows\Fonts\arial.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    const FONT_DAM: [&str; 5] = [
        r"C:\Windows\Fonts\arialbd.ttf",
        r"C:\Windows\Fonts\segoeuib.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ];
    let doc = |ds: &[&str]| ds.iter().find_map(|p| std::fs::read(p).ok());
    let mut fonts = egui::FontDefinitions::default();
    let mac_dinh = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(du_lieu) = doc(&FONT_THUONG) {
        fonts.font_data.insert("he_thong".into(), Arc::new(egui::FontData::from_owned(du_lieu)));
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "he_thong".into());
        fonts.families.entry(egui::FontFamily::Monospace).or_default().push("he_thong".into());
    }
    // Họ font đậm: font đậm của hệ thống, rồi tới font thường để không thiếu ký tự
    let mut dam = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or(mac_dinh);
    if let Some(du_lieu) = doc(&FONT_DAM) {
        fonts.font_data.insert("he_thong_dam".into(), Arc::new(egui::FontData::from_owned(du_lieu)));
        dam.insert(0, "he_thong_dam".into());
    }
    fonts.families.insert(egui::FontFamily::Name(HO_FONT_DAM.into()), dam);
    ctx.set_fonts(fonts);

    crate::gui_tach::bieu_tuong::dang_ky_svg(ctx);
    ctx.set_theme(egui::ThemePreference::Light);
    ctx.all_styles_mut(|style| {
        use egui::{FontFamily::Proportional, FontId};
        style.visuals = egui::Visuals::light();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(15.0, Proportional)),
            (TextStyle::Body, FontId::new(12.5, Proportional)),
            (TextStyle::Button, FontId::new(12.5, Proportional)),
            (TextStyle::Small, FontId::new(11.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 26.0;

        let v = &mut style.visuals;
        v.override_text_color = Some(CHU);
        v.panel_fill = NEN;
        v.window_fill = NEN;
        v.extreme_bg_color = Color32::WHITE;
        v.text_edit_bg_color = Some(Color32::WHITE);
        v.faint_bg_color = NEN_XEN_KE; // màu dòng xen kẽ của bảng (striped)
        v.selection.bg_fill = Color32::from_rgb(0xCC, 0xE4, 0xF7);
        v.selection.stroke = Stroke::new(1.0, CHU);
        v.hyperlink_color = CHU;
        v.window_corner_radius = CornerRadius::same(4);
        v.window_stroke = Stroke::new(1.0, VIEN);
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive, &mut v.widgets.open] {
            w.corner_radius = CornerRadius::same(3);
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, VIEN_NHAT);
        v.widgets.inactive.weak_bg_fill = NUT;
        v.widgets.inactive.bg_fill = NUT;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, VIEN);
        v.widgets.hovered.weak_bg_fill = NUT_HOVER;
        v.widgets.hovered.bg_fill = NUT_HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x00, 0x78, 0xD7));
        v.widgets.active.weak_bg_fill = Color32::from_rgb(0xCC, 0xE4, 0xF7);
        v.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x00, 0x54, 0x99));
    });
}

/// Một dòng trong bảng kết quả
enum DongBang {
    Trang {
        o: Vec<String>,
        canh_bao: bool,
        /// Thứ tự file (0-based) và khổ của trang, dùng cho bộ lọc theo khổ
        file: usize,
        kho: Option<usize>,
    },
    Loi(String),
}

/// Số liệu cộng dồn trong lúc đếm, tính từ các dòng nhận được
#[derive(Default, Clone)]
struct ThongKe {
    trang: usize,
    theo_kho: [usize; 8],
    canh_bao: usize,
    loi: usize,
    /// Số file đã ký số (đếm theo file, dựa trên trang đầu của mỗi file)
    da_ky: usize,
    /// Kích thước (rộng, cao) các trang khổ A0: quy A4 theo diện tích thật nên cần giữ để tính lại khi đổi hệ số
    kich_thuoc_a0: Vec<(f64, f64)>,
}

struct DangDem {
    nhan: Receiver<SuKien>,
    dung: Arc<AtomicBool>,
    thu_muc: String,
    bat_dau: Instant,
    lan: crate::license::LanChay,
}

/// Thông tin khi luồng đếm kết thúc
struct DaXong {
    thu_muc: String,
    thoi_gian: Duration,
    da_dung: bool,
}

/// Nội dung popup "Đã đếm xong"
struct ThongBaoXong {
    ten_thu_muc: String,
    so_file_dem: usize,
    so_file_loi: usize,
    so_trang: usize,
    quy_a4: f64,
    thoi_gian: Duration,
    da_dung: bool,
    /// Đường dẫn file tổng hợp đã lưu, hoặc thông báo lỗi khi ghi file
    ket_qua: Result<PathBuf, String>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum LocKySo {
    TatCa,
    Co,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum BoLoc {
    TatCa,
    /// Trang có cảnh báo
    CanhBao,
    /// File không mở được
    Loi,
    /// Mọi trang của các file có chứa ít nhất một trang khổ này
    Kho(usize),
}

struct UngDung {
    thu_muc: String,
    dong: Vec<DongBang>,
    /// Với mỗi file: tập các khổ có trong file (bit k = có trang khổ KHO[k])
    kho_cua_file: Vec<u8>,
    bo_loc: BoLoc,
    /// Kết quả lọc đã tính: (bộ lọc, số dòng lúc tính, chỉ số các dòng hiện)
    /// Lọc theo cột Ký số (dropdown ở header)
    loc_ky_so: LocKySo,
    da_loc: Option<((BoLoc, LocKySo, usize), Vec<usize>)>,
    thong_ke: ThongKe,
    tien_do: Option<(usize, usize)>,
    dang_dem: Option<DangDem>,
    trang_thai: String,
    /// Quy tắc xếp khổ / quy A4 từ Cấu hình › Đếm trang A4
    quy_tac: dem_pdf::QuyTac,
    /// Quy tắc đã dùng cho lần đếm gần nhất (để nhắc khi người dùng đổi Cấu hình sau đó)
    quy_tac_da_dung: Option<dem_pdf::QuyTac>,
    thong_bao: Option<ThongBaoXong>,
    /// Thời gian của lần đếm xong gần nhất (hiện ở dòng chân khi bảng có kết quả)
    thoi_gian_dem: Option<Duration>,
    /// Đường dẫn và chỉ số file của dòng trang gần nhất, để gom các trang cùng file
    file_cuoi: Option<(String, usize)>,
    /// Số dòng của bảng lúc cuộn tới cuối lần gần nhất; khác dong.len() là có dòng mới cần cuộn theo
    so_dong_da_cuon: usize,
    /// Khối "KẾT QUẢ ĐẾM" của lần đếm xong gần nhất, dùng cho nút copy
    ket_qua_chep: Option<String>,
    da_chep_luc: Option<Instant>,
    /// Ô "Check DPI": mặc định tắt cho nhanh
    kiem_tra_dpi: bool,
    /// Ô "Check chữ ký": mặc định tắt
    kiem_tra_ky: bool,
    /// Đã gửi số liệu lên trạm (có trạm); quyết định có hiện dòng báo trong popup không
    co_telegram: bool,
    /// License dùng chung với màn Tách; popup "Liên hệ Sao Mai" khi hết lượt Đếm.
    license: Arc<crate::license::KiemTra>,
    hop_license: Option<crate::hop_license::HopLicense>,
}

impl Default for UngDung {
    fn default() -> Self {
        Self {
            thu_muc: String::new(),
            dong: Vec::new(),
            kho_cua_file: Vec::new(),
            bo_loc: BoLoc::TatCa,
            loc_ky_so: LocKySo::TatCa,
            da_loc: None,
            thong_ke: ThongKe::default(),
            tien_do: None,
            dang_dem: None,
            trang_thai: String::new(),
            quy_tac: dem_pdf::QuyTac::default(),
            quy_tac_da_dung: None,
            thong_bao: None,
            thoi_gian_dem: None,
            file_cuoi: None,
            so_dong_da_cuon: 0,
            ket_qua_chep: None,
            da_chep_luc: None,
            kiem_tra_dpi: false,
            kiem_tra_ky: true,
            co_telegram: false,
            license: crate::license::KiemTra::mac_dinh(),
            hop_license: None,
        }
    }
}

/// 5063 -> "5.063"
pub(crate) fn so_nguyen(n: usize) -> String {
    let s = n.to_string();
    let mut kq = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            kq.push('.');
        }
        kq.push(c);
    }
    kq
}

/// 885.5 -> "885,5"; 885.0 -> "885"
pub(crate) fn so_thuc(x: f64) -> String {
    let s = format!("{x:.2}");
    let (nguyen, thap_phan) = s.split_once('.').unwrap_or((&s, ""));
    let nguyen = nguyen.parse::<usize>().map(so_nguyen).unwrap_or_else(|_| nguyen.to_string());
    match thap_phan.trim_end_matches('0') {
        "" => nguyen,
        tp => format!("{nguyen},{tp}"),
    }
}

/// 65,4 giây -> "1 phút, 5,4 giây"
pub(crate) fn thoi_gian_phut_giay(t: Duration) -> String {
    let giay = t.as_secs_f64();
    let phut = (giay / 60.0).floor();
    let con_lai = format!("{:.1}", giay - phut * 60.0).replace('.', ",");
    format!("{} phút, {con_lai} giây", phut as u64)
}

/// Mở file bằng app mặc định (PDF: trình duyệt / Foxit…), hoặc mở thư mục bằng trình quản lý file
pub(crate) fn mo_bang_he_dieu_hanh(p: &Path) {
    let lenh = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(lenh).arg(p).spawn();
}

#[derive(Clone, Copy, PartialEq)]
enum CanLe {
    Trai,
    Giua,
    Phai,
}

pub(crate) const NEN_TIEU_DE: Color32 = Color32::from_rgb(0xEB, 0xEB, 0xEB);
pub(crate) const NEN_TAB: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xF7);
pub(crate) const NEN_TAB_CHON: Color32 = Color32::from_rgb(0xDC, 0xEB, 0xFA);
pub(crate) const VIEN_TAB_CHON: Color32 = Color32::from_rgb(0x2F, 0x7B, 0xD9);
pub(crate) const NEN_XEN_KE: Color32 = Color32::from_rgb(0xF6, 0xF6, 0xF6);
/// Lề trái/phải trong mỗi ô để chữ không sát mép
const LE_O: f32 = 8.0;
const CAO_TIEU_DE: f32 = 26.0;
const CAO_DONG: f32 = 22.0;

/// Làm tròn một kích thước (điểm) về số pixel nguyên theo tỷ lệ màn hình.
/// Windows phóng 125%/175% làm 22pt = 27,5px → các hàng bị vẽ luân phiên 27/28px, nhìn to bé không đều.
fn tron_pixel(ui: &egui::Ui, diem: f32) -> f32 {
    let ppp = ui.ctx().pixels_per_point();
    (diem * ppp).round() / ppp
}
/// Các cột bảng chi tiết: tên, căn lề, độ rộng (None = cột File giãn theo cửa sổ)
const COT_BANG: [(&str, CanLe, Option<f32>); SO_COT] = [
    ("STT", CanLe::Giua, Some(76.0)),
    ("File (bấm để mở)", CanLe::Trai, None),
    ("Trang", CanLe::Phai, Some(48.0)),
    ("Chiều rộng", CanLe::Phai, Some(78.0)),
    ("Chiều cao", CanLe::Phai, Some(78.0)),
    ("Hướng", CanLe::Giua, Some(68.0)),
    ("Khổ", CanLe::Giua, Some(52.0)),
    ("Ký số", CanLe::Giua, Some(84.0)),
    ("DPI", CanLe::Giua, Some(58.0)),
    ("Cảnh báo", CanLe::Trai, Some(176.0)),
];

/// Một ô bảng có lề hai bên và căn trái / giữa / phải
fn o_bang(ui: &mut egui::Ui, chu: RichText, can: CanLe) {
    let nhan = egui::Label::new(chu).truncate();
    match can {
        CanLe::Trai => {
            ui.add_space(LE_O);
            ui.add(nhan);
        }
        CanLe::Giua => {
            ui.with_layout(Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| ui.add(nhan));
        }
        CanLe::Phai => {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(LE_O);
                ui.add(nhan);
            });
        }
    }
}

/// Liên kết chữ kiểu web: rê chuột hiện gạch chân và con trỏ bàn tay; đang chọn thì in đậm, gạch chân;
/// không có dữ liệu thì chữ mờ, không bấm được. Trả về true khi được bấm.
fn lien_ket(ui: &mut egui::Ui, chu: RichText, dang_chon: bool, bat: bool) -> bool {
    let chu = if dang_chon { chu.dam() } else if bat { chu } else { chu.color(CHU_MO) };
    let phan_hoi = ui.add(egui::Label::new(chu).selectable(false).sense(if bat { egui::Sense::click() } else { egui::Sense::hover() }));
    if bat && (phan_hoi.hovered() || dang_chon) {
        let r = phan_hoi.rect;
        let mau = ui.style().visuals.override_text_color.unwrap_or(CHU);
        ui.painter().hline(r.x_range(), r.bottom(), Stroke::new(1.0, mau));
    }
    bat && phan_hoi.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Nút copy vẽ bằng hình (hai tờ giấy chồng nhau), không phụ thuộc font có ký tự biểu tượng
fn nut_copy(ui: &mut egui::Ui, bat: bool) -> egui::Response {
    let (rect, phan_hoi) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), if bat { egui::Sense::click() } else { egui::Sense::hover() });
    let painter = ui.painter();
    if bat && phan_hoi.hovered() {
        painter.rect_filled(rect, 4.0, NUT_HOVER);
    }
    let mau = if bat { CHU } else { CHU_MO };
    let net = Stroke::new(1.3, mau);
    let sau = egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 5.0), egui::vec2(9.0, 11.0));
    let truoc = sau.translate(egui::vec2(3.5, 3.5));
    painter.rect_stroke(sau, 1.5, net, egui::StrokeKind::Middle);
    painter.rect(truoc, 1.5, if bat && phan_hoi.hovered() { NUT_HOVER } else { NEN }, net, egui::StrokeKind::Middle);
    if bat {
        phan_hoi.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        phan_hoi
    }
}

pub(crate) fn nut(chu: &str) -> egui::Button<'static> {
    egui::Button::new(chu.to_owned()).min_size(egui::vec2(RONG_NUT, 26.0))
}

pub(crate) fn nut_mau(chu: &str, nen: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(chu.to_owned()).color(Color32::WHITE)).fill(nen).min_size(egui::vec2(RONG_NUT, 26.0))
}

impl UngDung {
    /// Xoá kết quả bảng về như lúc mới mở ứng dụng (giữ thư mục, hệ số, tuỳ chọn)
    fn xoa_ket_qua(&mut self) {
        self.dong.clear();
        self.kho_cua_file.clear();
        self.da_loc = None;
        self.bo_loc = BoLoc::TatCa;
        self.loc_ky_so = LocKySo::TatCa;
        self.thong_ke = ThongKe::default();
        self.tien_do = None;
        self.ket_qua_chep = None;
        self.da_chep_luc = None;
        self.thong_bao = None;
        self.thoi_gian_dem = None;
        self.file_cuoi = None;
        self.so_dong_da_cuon = 0;
        self.quy_tac_da_dung = None;
        self.trang_thai = String::new();
    }

    fn bat_dau_dem(&mut self, ctx: &egui::Context) {
        let thu_muc = self.thu_muc.trim().to_string();
        if thu_muc.is_empty() {
            self.trang_thai = "Lỗi: hãy chọn thư mục chứa file PDF".into();
            return;
        }
        if !Path::new(&thu_muc).is_dir() {
            self.trang_thai = format!("Lỗi: không tìm thấy thư mục {thu_muc}");
            return;
        }
        let lan = match self.license.bat_dau(crate::license::ChucNang::Dem) {
            Ok(l) => l,
            Err(qd) => {
                self.hop_license = Some(crate::hop_license::HopLicense::moi(qd));
                return;
            }
        };
        let quy_tac = self.quy_tac;

        self.xoa_ket_qua();
        self.trang_thai = "Đang tìm file PDF…".into();

        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let (tm, dung_luong, ctx, kiem_tra_dpi, kiem_tra_ky) = (thu_muc.clone(), Arc::clone(&dung), ctx.clone(), self.kiem_tra_dpi, self.kiem_tra_ky);
        std::thread::spawn(move || {
            dem_pdf::dem_thu_muc(&tm, quy_tac, kiem_tra_dpi, kiem_tra_ky, &dung_luong, &gui);
            ctx.request_repaint();
        });
        self.quy_tac_da_dung = Some(quy_tac);
        self.dang_dem = Some(DangDem { nhan, dung, thu_muc, bat_dau: Instant::now(), lan });
    }

    fn them_dong(&mut self, s: String) {
        if let Some(duong_dan) = s.strip_prefix("Lỗi mở tệp: ") {
            self.thong_ke.loi += 1;
            self.dong.push(DongBang::Loi(duong_dan.to_string()));
            return;
        }
        let o: Vec<String> = s.split('\t').map(str::to_owned).collect();
        if o.len() != 9 || o[0] == "STT" {
            return; // dòng tiêu đề
        }
        // Các trang của một file đến liền nhau: đổi đường dẫn (phần trước " => trang: ") là sang file mới.
        // Đánh chỉ số file theo cách này nên không phụ thuộc cột STT và không thể cấp phát vô hạn
        let duong_dan = o[1].rsplit_once(" => trang: ").map_or(o[1].as_str(), |(d, _)| d);
        let file = match &self.file_cuoi {
            Some((d, i)) if d == duong_dan => *i,
            Some((_, i)) => i + 1,
            None => 0,
        };
        self.file_cuoi = Some((duong_dan.to_string(), file));
        if self.kho_cua_file.len() <= file {
            self.kho_cua_file.resize(file + 1, 0);
        }
        let kho = KHO.iter().position(|k| *k == o[5]);
        if let Some(k) = kho {
            self.kho_cua_file[file] |= 1 << k;
            self.thong_ke.theo_kho[k] += 1;
            if k == 0 {
                let so = |x: &str| x.trim().parse::<f64>().unwrap_or(0.0);
                self.thong_ke.kich_thuoc_a0.push((so(&o[2]), so(&o[3])));
            }
        }
        let canh_bao = !o[7].is_empty();
        self.thong_ke.trang += 1;
        if canh_bao {
            self.thong_ke.canh_bao += 1;
        }
        // Ký số là thuộc tính của file: đếm một lần ở trang đầu mỗi file
        if o[8] == "Có" && o[1].ends_with(" => trang: 1") {
            self.thong_ke.da_ky += 1;
        }
        self.dong.push(DongBang::Trang { o, canh_bao, file, kho });
    }

    /// Quy ra A4 = Σ (số trang khổ Ax × hệ số Ax) theo Cấu hình hiện tại
    fn quy_a4_hien_tai(&self) -> Option<f64> {
        let q = &self.quy_tac;
        // A0 lớn quy theo Cấu hình từng trang (cùng công thức với lõi đếm), các khổ khác theo hệ số
        let quy_a0: f64 = self.thong_ke.kich_thuoc_a0.iter().map(|&(r, c)| q.quy_a4(0, r, c)).sum();
        Some(dem_pdf::lam_tron_2((1..8).fold(quy_a0, |tong, k| tong + self.thong_ke.theo_kho[k] as f64 * q.he_so[k])))
    }

    /// Chỉ số các dòng hiện theo bộ lọc; tính lại khi đổi bộ lọc hoặc có thêm dòng
    fn dong_hien(&mut self) -> Option<&[usize]> {
        // Không lọc gì cả thì hiện toàn bộ (None)
        if self.bo_loc == BoLoc::TatCa && self.loc_ky_so == LocKySo::TatCa {
            return None;
        }
        let khoa = (self.bo_loc, self.loc_ky_so, self.dong.len());
        let can_tinh = !matches!(&self.da_loc, Some((k, _)) if *k == khoa);
        if can_tinh {
            let (bo_loc, loc_ky) = (self.bo_loc, self.loc_ky_so);
            let chon: Vec<usize> = self
                .dong
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    let hop_bo_loc = match (bo_loc, d) {
                        (BoLoc::TatCa, _) => true,
                        (BoLoc::CanhBao, DongBang::Trang { canh_bao, .. }) => *canh_bao,
                        (BoLoc::Loi, DongBang::Loi(_)) => true,
                        (BoLoc::Kho(k), DongBang::Trang { file, .. }) => self.kho_cua_file[*file] & (1 << k) != 0,
                        _ => false,
                    };
                    let hop_ky = match loc_ky {
                        LocKySo::TatCa => true,
                        LocKySo::Co => matches!(d, DongBang::Trang { o, .. } if o[8] == "Có"),
                    };
                    hop_bo_loc && hop_ky
                })
                .map(|(i, _)| i)
                .collect();
            self.da_loc = Some((khoa, chon));
        }
        self.da_loc.as_ref().map(|(_, v)| v.as_slice())
    }

    /// Nhận kết quả từ luồng đếm; trả về kết quả cuối cùng khi đếm xong
    fn nhan_su_kien(&mut self) -> Option<(KetQuaDem, DaXong)> {
        let dang_dem = self.dang_dem.as_ref()?;
        let mut moi = Vec::new();
        let mut xong = None;
        let mut mat_ket_noi = false;
        for _ in 0..SU_KIEN_MOI_KHUNG {
            match dang_dem.nhan.try_recv() {
                Ok(SuKien::Dong(s)) => moi.push(s),
                Ok(SuKien::DangTim(n)) => self.trang_thai = format!("Đang tìm file PDF… đã thấy {}", so_nguyen(n)),
                Ok(SuKien::TienDo(d, t)) => self.tien_do = Some((d, t)),
                Ok(SuKien::Xong(kq)) => {
                    xong = Some(kq);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    mat_ket_noi = true;
                    break;
                }
            }
        }
        for s in moi {
            self.them_dong(s);
        }
        if self.tien_do.is_some() {
            self.trang_thai.clear();
        }
        if mat_ket_noi {
            if let Some(dd) = self.dang_dem.take() {
                self.license.ket_thuc(&dd.lan, false);
            }
            self.trang_thai = "Lỗi: luồng đếm dừng bất thường".into();
            return None;
        }
        let kq = xong?;
        let dd = self.dang_dem.take()?;
        // Có kết quả là tính lượt (kể cả Dừng giữa chừng); không đếm được file nào → hoàn lượt
        self.license.ket_thuc(&dd.lan, !kq.dong_tong_hop.is_empty());
        let (thoi_gian, da_dung) = (dd.bat_dau.elapsed(), dd.dung.load(Ordering::Relaxed));
        Some((kq, DaXong { thu_muc: dd.thu_muc, thoi_gian, da_dung }))
    }

    /// Tự lưu vào <thư mục>/<tên thư mục>_dem.txt (ghi đè nếu đã có) rồi mở popup tóm tắt
    fn luu_va_thong_bao(&mut self, kq: &KetQuaDem, xong: DaXong) {
        self.ket_qua_chep = Some(kq.tong_hop.trim_start_matches('\n').to_string());
        self.thoi_gian_dem = Some(xong.thoi_gian);
        let duong_dan = dem_pdf::duong_dan_luu_mac_dinh(&xong.thu_muc);
        let ket_qua = match dem_pdf::luu_ket_qua(kq, &duong_dan) {
            Ok(_) => Ok(duong_dan),
            Err(e) => {
                self.trang_thai = format!("Lỗi khi ghi file {}: {e}", duong_dan.display());
                Err(format!("Không ghi được file kết quả: {e}"))
            }
        };
        // Gửi số liệu lên trạm ở luồng riêng (trạm ghi Sheet + xếp tin Telegram); chạy ngầm, im lặng khi lỗi.
        // Popup có dòng báo app gửi thống kê. Test (kể cả chụp ảnh màn hình) không gọi trạm thật
        let so_lieu = crate::tram::so_lieu_dem(&xong.thu_muc, kq, xong.thoi_gian);
        self.co_telegram = self.license.thong_ke("thong_ke_dem", move |_| so_lieu);
        let ten_thu_muc = Path::new(xong.thu_muc.trim_end_matches(['/', '\\'])).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or(xong.thu_muc);
        self.thong_bao = Some(ThongBaoXong {
            ten_thu_muc,
            so_file_dem: kq.dong_tong_hop.len(),
            so_file_loi: kq.so_file_loi,
            so_trang: kq.tong_so_trang,
            quy_a4: kq.quy_a4.unwrap_or(0.0),
            thoi_gian: xong.thoi_gian,
            da_dung: xong.da_dung,
            ket_qua,
        });
    }

    fn hang_thu_muc(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            let ranh = self.dang_dem.is_none();
            let nhan = ui.label("Chọn thư mục pdf:");
            let rong_o = ui.available_width() - 2.0 * (RONG_NUT + ui.spacing().item_spacing.x);
            let o = egui::TextEdit::singleline(&mut self.thu_muc).desired_width(rong_o).margin(Margin::symmetric(5, 4));
            let phan_hoi = ui.add_enabled(ranh, o).labelled_by(nhan.id);
            if ranh && phan_hoi.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.bat_dau_dem(&ctx);
            }
            if ui.add_enabled(ranh, nut("Chọn")).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục chứa PDF").pick_folder() {
                    self.thu_muc = p.display().to_string();
                    // Chọn thư mục mới: xoá bảng kết quả cũ như lúc mới mở ứng dụng
                    self.xoa_ket_qua();
                }
            }
            match &self.dang_dem {
                None => {
                    if ui.add(nut_mau("Đếm", XANH_LA)).clicked() {
                        self.bat_dau_dem(&ctx);
                    }
                }
                Some(d) if d.dung.load(Ordering::Relaxed) => {
                    ui.add_enabled(false, nut("Đang dừng"));
                }
                Some(d) => {
                    if ui.add(nut_mau("Dừng", DO)).clicked() {
                        d.dung.store(true, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    /// Hàng hệ số A0..A7, chỉ hiện khi bấm nút "Hệ số"; giá trị màu xám, căn giữa ô
    /// Bấm vào một liên kết lọc: chọn bộ lọc đó, bấm lại bộ lọc đang chọn thì bỏ lọc
    fn bam_loc(&mut self, loc: BoLoc) {
        self.bo_loc = if self.bo_loc == loc { BoLoc::TatCa } else { loc };
    }

    fn dong_tom_tat(&mut self, ui: &mut egui::Ui) {
        let tk = self.thong_ke.clone();
        let so_file = self.tien_do.map(|(d, _)| d).unwrap_or(0);
        ui.horizontal(|ui| {
            let muc = |ui: &mut egui::Ui, nhan: &str, gia_tri: String| {
                ui.label(RichText::new(nhan).color(CHU_PHU));
                ui.label(RichText::new(gia_tri).dam());
                ui.add_space(14.0);
            };
            muc(ui, "File PDF:", so_nguyen(so_file));
            muc(ui, "Số trang:", so_nguyen(tk.trang));
            match self.quy_a4_hien_tai() {
                Some(q) => muc(ui, "Quy ra A4:", so_thuc(q)),
                None => {
                    ui.label(RichText::new("Quy ra A4:").color(CHU_PHU));
                    ui.label(RichText::new("?").dam().color(DO)).on_hover_text("Có ô hệ số không phải là số");
                    ui.add_space(14.0);
                }
            }
            // Cảnh báo và File lỗi là liên kết lọc như hàng Chi tiết
            for (nhan, so, loc) in [("Cảnh báo:", tk.canh_bao, BoLoc::CanhBao), ("File lỗi:", tk.loi, BoLoc::Loi)] {
                ui.label(RichText::new(nhan).color(CHU_PHU));
                let mau = if so > 0 { DO_SAM } else { CHU };
                if lien_ket(ui, RichText::new(so_nguyen(so)).color(mau), self.bo_loc == loc, so > 0) {
                    self.bam_loc(loc);
                }
                ui.add_space(14.0);
            }
            // Ký số: liên kết lọc nhanh các file đã ký số
            ui.label(RichText::new("Ký số:").color(CHU_PHU));
            let dang_loc_ky = self.loc_ky_so == LocKySo::Co;
            let mau_ky = if tk.da_ky > 0 { XANH_COBALT } else { CHU };
            if lien_ket(ui, RichText::new(so_nguyen(tk.da_ky)).color(mau_ky), dang_loc_ky, tk.da_ky > 0) {
                self.loc_ky_so = if dang_loc_ky { LocKySo::TatCa } else { LocKySo::Co };
            }
            ui.add_space(14.0);

            // Nút copy ngay sau "File lỗi": chép khối "KẾT QUẢ ĐẾM", chỉ bấm được sau khi đếm xong
            let co_ket_qua = self.dang_dem.is_none() && self.ket_qua_chep.is_some();
            let nut = nut_copy(ui, co_ket_qua);
            let nut = if co_ket_qua { nut.on_hover_text("Sao chép kết quả đếm") } else { nut.on_hover_text("Chỉ sao chép được sau khi đếm xong") };
            if co_ket_qua && nut.clicked() {
                if let Some(chu) = &self.ket_qua_chep {
                    ui.ctx().copy_text(chu.clone());
                    self.da_chep_luc = Some(Instant::now());
                }
            }
            if let Some(luc) = self.da_chep_luc {
                if luc.elapsed() < Duration::from_secs(2) {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Đã sao chép").color(CHU_PHU));
                    ui.ctx().request_repaint_after(Duration::from_millis(200));
                } else {
                    self.da_chep_luc = None;
                }
            }

            let quy_tac_doi = self.quy_tac_da_dung.is_some_and(|cu| cu != self.quy_tac);
            if quy_tac_doi && self.dang_dem.is_none() {
                ui.add_space(14.0);
                ui.label(RichText::new("Cấu hình quy A4 đã đổi – bấm Đếm để đếm và ghi lại file kết quả").italics().color(CHU_PHU));
            }

            // Bên phải: Check ký số, Check DPI, liên kết Hệ số
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled(self.dang_dem.is_none(), egui::Checkbox::new(&mut self.kiem_tra_dpi, "Check DPI")).on_hover_text(
                    "Tính DPI ảnh scan và cảnh báo trang có DPI khác 300.\nChạy nội dung từng trang nên chậm với tài liệu văn bản nhiều trang; tắt để đếm nhanh.",
                );
                ui.add_space(6.0);
                ui.add_enabled(self.dang_dem.is_none(), egui::Checkbox::new(&mut self.kiem_tra_ky, "Check ký số")).on_hover_text(
                    "Kiểm tra từng file PDF đã ký số hay chưa (đọc AcroForm, một lần mỗi file).\nNhẹ hơn Check DPI nhiều.",
                );
            });
        });
    }

    /// Thanh tab lọc liền khối ở đầu bảng: Tất cả, A0..A7 chia đều, không có khe.
    /// Tab đang chọn nền xanh nhạt + gạch xanh ở đáy; bấm lại tab đang chọn thì về Tất cả.
    fn thanh_tab_loc(&mut self, ui: &mut egui::Ui) {
        const CAO_TAB: f32 = 30.0;
        // Trải hết bề ngang khung bảng (kể cả lề trong 2px mỗi bên)
        let (vung, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_TAB), egui::Sense::hover());
        let vung = vung.expand2(egui::vec2(2.0, 0.0));
        let rong = vung.width() / 9.0;
        let painter = ui.painter().clone();
        let mut bam: Option<BoLoc> = None;

        let muc: Vec<(BoLoc, String, bool)> = std::iter::once((BoLoc::TatCa, format!("Tất cả ({})", so_nguyen(self.dong.len())), true))
            .chain((0..8).map(|k| {
                let n = self.thong_ke.theo_kho[k];
                (BoLoc::Kho(k), format!("{} ({})", KHO[k], so_nguyen(n)), n > 0)
            }))
            .collect();

        for (i, (loc, chu, bat)) in muc.into_iter().enumerate() {
            let o = egui::Rect::from_min_size(vung.min + egui::vec2(i as f32 * rong, 0.0), egui::vec2(rong, CAO_TAB));
            let phan_hoi = ui.interact(o, ui.scope_id().with(("tab_loc", i)), if bat { egui::Sense::click() } else { egui::Sense::hover() });
            let dang_chon = self.bo_loc == loc;
            let nen = if dang_chon {
                NEN_TAB_CHON
            } else if bat && phan_hoi.hovered() {
                NUT_HOVER
            } else {
                NEN_TAB
            };
            painter.rect_filled(o, 0.0, nen);
            if dang_chon {
                let day = egui::Rect::from_min_max(egui::pos2(o.left(), o.bottom() - 2.0), o.max);
                painter.rect_filled(day, 0.0, VIEN_TAB_CHON);
            }
            let mut chu = RichText::new(chu);
            chu = if dang_chon { chu.dam() } else if bat { chu } else { chu.color(CHU_MO) };
            let galley = egui::WidgetText::from(chu).into_galley(ui, Some(egui::TextWrapMode::Truncate), o.width() - 8.0, TextStyle::Button);
            painter.galley(o.center() - galley.size() / 2.0, galley, CHU);
            if bat && phan_hoi.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                bam = Some(loc);
            }
        }
        painter.hline(vung.x_range(), vung.bottom(), Stroke::new(1.0, VIEN));

        match bam {
            Some(BoLoc::TatCa) => self.bo_loc = BoLoc::TatCa,
            Some(loc) => self.bam_loc(loc),
            None => {}
        }
    }

    fn bang_ket_qua(&mut self, ui: &mut egui::Ui) {
        let khung = egui::Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(2, 0));
        let khung_bang = khung.show(ui, |ui| {
            // Thanh tab lọc là phần trên cùng của bảng, ngay sau là tiêu đề cột
            self.thanh_tab_loc(ui);
            ui.add_space(-ui.spacing().item_spacing.y);
            if self.dong.is_empty() {
                if self.dang_dem.is_some() {
                    ui.allocate_ui_with_layout(ui.available_size(), Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.label(RichText::new("Đang đọc file…").color(CHU_PHU));
                    });
                } else if crate::gioi_thieu::ve(ui, &crate::gioi_thieu::DEM) == Some(crate::gioi_thieu::Bam::Chon) {
                    // Link "Chọn" trong giới thiệu = nút Chọn
                    if let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục chứa PDF").pick_folder() {
                        self.thu_muc = p.display().to_string();
                        self.xoa_ket_qua();
                    }
                }
                return;
            }
            let bo_loc = self.bo_loc;
            let loc: Option<Vec<usize>> = self.dong_hien().map(<[usize]>::to_vec);
            let so_dong = loc.as_ref().map_or(self.dong.len(), Vec::len);
            // Số file pdf trong phần đang hiển thị: không lọc thì lấy tổng (O(1)),
            // có lọc thì đếm file khác nhau trong danh sách đã lọc (nhỏ)
            let so_file = match &loc {
                None => self.tien_do.map_or(0, |(d, _)| d),
                Some(v) => {
                    let mut files = std::collections::HashSet::new();
                    let mut loi = 0usize;
                    for &i in v {
                        match &self.dong[i] {
                            DongBang::Trang { file, .. } => {
                                files.insert(*file);
                            }
                            DongBang::Loi(_) => loi += 1,
                        }
                    }
                    files.len() + loi
                }
            };
            // Chừa chỗ cho dòng chân (số dòng / số file) ở đáy bảng: bảng chiếm phần còn lại
            const CAO_CHAN: f32 = 28.0;
            // Khe giữa các dòng và chiều cao dòng/tiêu đề/chân làm tròn về pixel nguyên để mọi hàng cao bằng nhau
            let khe = tron_pixel(ui, ui.spacing().item_spacing.y);
            ui.spacing_mut().item_spacing.y = khe;
            let cao_tieu_de = tron_pixel(ui, CAO_TIEU_DE);
            let cao_dong = tron_pixel(ui, CAO_DONG + khe) - khe;
            let cao_chan = tron_pixel(ui, CAO_CHAN);
            let cao_bang = (ui.available_height() - cao_chan).max(60.0);
            let rong_bang = ui.available_width();
            let file_can_mo = ui.allocate_ui_with_layout(egui::vec2(rong_bang, cao_bang), Layout::top_down(Align::Min), |ui| {
                let mut file_can_mo: Option<String> = None;

                // Nền xám nhạt và đường kẻ dưới cho hàng tiêu đề
                let khe = ui.spacing().item_spacing.y;
                let vung_dau = egui::Rect::from_min_size(ui.cursor().min - egui::vec2(2.0, 0.0), egui::vec2(ui.available_width() + 4.0, cao_tieu_de + khe / 2.0));
                ui.painter().rect_filled(vung_dau, 0.0, NEN_TIEU_DE);
                ui.painter().hline(vung_dau.x_range(), vung_dau.bottom(), Stroke::new(1.0, VIEN_NHAT));

                // resizable: kéo mép cột để đổi độ rộng; dùng Column::initial để cột co giãn được
                // Có dòng mới (đang đếm, hoặc đợt cuối tới cùng khung hình với "Xong"): cuộn theo dòng mới nhất.
                // Vẽ thêm một khung để bảng biết kích thước nội dung mới rồi mới cuộn chắc chắn
                let cuon_cuoi = so_dong > 0 && (self.dang_dem.is_some() || self.dong.len() != self.so_dong_da_cuon);
                if cuon_cuoi {
                    ui.ctx().request_repaint();
                }
                // striped: bảng tự tô xen kẽ theo đúng bước dòng (tự vẽ theo ô sẽ lệch trên Windows)
                let mut bang = TableBuilder::new(ui).striped(true).resizable(true).stick_to_bottom(true).auto_shrink(false).cell_layout(Layout::left_to_right(Align::Center));
                if cuon_cuoi {
                    bang = bang.scroll_to_row(so_dong - 1, Some(Align::BOTTOM));
                }
            for (_, _, rong) in COT_BANG {
                bang = bang.column(match rong {
                    Some(w) => Column::initial(w).at_least(36.0).clip(true),
                    None => Column::remainder().at_least(120.0).clip(true),
                });
            }
            bang.header(cao_tieu_de, |mut h| {
                for (ten, can, _) in COT_BANG.iter() {
                    h.col(|ui| o_bang(ui, RichText::new(*ten), *can));
                }
            })
            .body(|body| {
                body.rows(cao_dong, so_dong, |mut row| {
                    let i = loc.as_ref().map_or(row.index(), |v| v[row.index()]);
                    match &self.dong[i] {
                        DongBang::Loi(duong_dan) => {
                            let ten = Path::new(duong_dan).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| duong_dan.clone());
                            row.col(|ui| {
                                o_bang(ui,RichText::new("—").color(CHU_PHU), CanLe::Giua);
                            });
                            row.col(|ui| {
                                ui.add_space(LE_O);
                                ui.add(egui::Label::new(RichText::new(&ten).color(DO_SAM)).truncate()).on_hover_text(duong_dan);
                            });
                            for _ in 2..SO_COT - 1 {
                                row.col(|_| {});
                            }
                            row.col(|ui| o_bang(ui, RichText::new("Không mở được file").color(DO_SAM), CanLe::Trai));
                        }
                        DongBang::Trang { o, kho, .. } => {
                            let (duong_dan, so_trang) = o[1].rsplit_once(" => trang: ").unwrap_or((&o[1], ""));
                            let ten = Path::new(duong_dan).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                            // Khi lọc theo khổ: in đậm các trang đúng khổ đó
                            let dam = matches!(bo_loc, BoLoc::Kho(k) if *kho == Some(k));
                            let chu = |s: &str| if dam { RichText::new(s).dam() } else { RichText::new(s) };
                            row.col(|ui| {
                                o_bang(ui,chu(&o[0]).color(CHU_PHU), CanLe::Giua);
                            });
                            row.col(|ui| {
                                ui.add_space(LE_O);
                                let lien_ket = ui.add(egui::Label::new(chu(&ten)).truncate().sense(egui::Sense::click())).on_hover_text(format!("{duong_dan}\nBấm để mở file PDF"));
                                let lien_ket = lien_ket.on_hover_cursor(egui::CursorIcon::PointingHand);
                                if lien_ket.hovered() {
                                    let r = lien_ket.rect;
                                    ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(1.0, CHU));
                                }
                                if lien_ket.clicked() {
                                    file_can_mo = Some(duong_dan.to_string());
                                }
                            });
                            // Cột 2..6: Trang, Chiều rộng, Chiều cao, Hướng, Khổ
                            let gia_tri = [so_trang, &o[2], &o[3], &o[4], &o[5]];
                            for (c, gt) in gia_tri.into_iter().enumerate() {
                                row.col(|ui| o_bang(ui, chu(gt), COT_BANG[c + 2].1));
                            }
                            // Ký số (o[8]) đứng trước DPI: "Có" tô xanh cobalt; còn lại ("Không"/"-") để đen
                            let mau_ky = if o[8] == "Có" { XANH_COBALT } else { CHU };
                            row.col(|ui| o_bang(ui, RichText::new(&o[8]).color(mau_ky), CanLe::Giua));
                            row.col(|ui| o_bang(ui, chu(&o[6]), CanLe::Giua)); // DPI
                            row.col(|ui| o_bang(ui, RichText::new(&o[7]).color(DO_SAM), CanLe::Trai)); // Cảnh báo
                        }
                    }
                });
                });

                file_can_mo
            })
            .inner;

            self.so_dong_da_cuon = self.dong.len();
            if let Some(p) = file_can_mo {
                mo_bang_he_dieu_hanh(Path::new(&p));
            }

            // Dòng chân: nền xám mờ, canh giữa theo chiều dọc.
            // Trái: thời gian đếm (chạy khi đang đếm, giữ lại khi xong) · Phải: số dòng / số file đang hiển thị
            egui::Frame::new().fill(NEN_XEN_KE).inner_margin(Margin { left: 10, right: 12, top: 0, bottom: 0 }).show(ui, |ui| {
                let rong = ui.available_width();
                ui.allocate_ui_with_layout(egui::vec2(rong, cao_chan), Layout::left_to_right(Align::Center), |ui| {
                    // Đồng hồ: khi đang đếm thì chạy; đếm xong vẫn hiện nếu bảng có kết quả
                    let giay = match &self.dang_dem {
                        Some(d) => Some(d.bat_dau.elapsed().as_secs()),
                        None if !self.dong.is_empty() => self.thoi_gian_dem.map(|t| t.as_secs()),
                        None => None,
                    };
                    if let Some(s) = giay {
                        // Không xuống dòng: dòng chân chỉ cao một dòng, chữ bị gói sẽ mất phần sau
                        ui.add(egui::Label::new(RichText::new(format!("Thời gian đếm: {}:{:02}", s / 60, s % 60)).small().color(CHU_PHU)).extend());
                        if self.dang_dem.is_some() {
                            // Ép vẽ lại đều đặn để đồng hồ chạy ngay cả khi chưa có dòng mới
                            ui.ctx().request_repaint_after(Duration::from_millis(250));
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let chan = format!("Hiển thị: {} dòng ; {} file pdf", so_nguyen(so_dong), so_nguyen(so_file));
                        ui.add(egui::Label::new(RichText::new(chan).small().color(CHU_PHU)).extend());
                    });
                });
            });
        });
        // Vẽ viền sau cùng, nằm trên nền tiêu đề và nền dòng xen kẽ, để viền liền ở mọi dòng
        ui.painter().rect_stroke(khung_bang.response.rect, 0.0, Stroke::new(1.0, VIEN), egui::StrokeKind::Inside);
    }

    fn thanh_trang_thai(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| self.thanh_tien_do(ui));
    }

    /// Thanh tiến độ xám kiểu bản Python: "100% (48/48 files)"; khi có lỗi thì hiện lỗi thay thế
    fn thanh_tien_do(&self, ui: &mut egui::Ui) {
        if self.trang_thai.starts_with("Lỗi") {
            ui.label(RichText::new(&self.trang_thai).color(DO_SAM));
            return;
        }
        let (ty_le, chu) = match self.tien_do {
            Some((d, t)) if t > 0 => (d as f32 / t as f32, format!("{}% ({d}/{t} files)", d * 100 / t)),
            Some(_) => (1.0, "Không có file PDF nào".to_string()),
            None => (0.0, self.trang_thai.clone()),
        };
        // Chữ "% (x/y files)" vẽ vào chính giữa thanh (mặc định egui đặt chữ lệch trái)
        let thanh = ui.add(egui::ProgressBar::new(ty_le).desired_width(ui.available_width()).desired_height(20.0).fill(XAM_TIEN_DO).corner_radius(3));
        ui.painter().text(thanh.rect.center(), egui::Align2::CENTER_CENTER, chu, TextStyle::Body.resolve(ui.style()), CHU);
    }

    /// Popup kiểu hộp thoại: phần đầu (tiêu đề, thư mục), phần thông tin 2 cột, phần nút căn phải
    fn popup_xong(&mut self, ctx: &egui::Context) {
        let co_telegram = self.co_telegram;
        let chep = self.ket_qua_chep.clone();
        let Some(tb) = &self.thong_bao else {
            return;
        };
        const RONG: f32 = 420.0;
        const BO_GOC: u8 = 6;
        let (mut mo, mut dong) = (false, false);
        let khung = egui::Frame::popup(&ctx.global_style()).fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN)).inner_margin(Margin::ZERO).corner_radius(BO_GOC);
        let phan_hoi = egui::Modal::new(egui::Id::unique("popup_xong")).frame(khung).show(ctx, |ui| {
            ui.set_width(RONG);
            ui.spacing_mut().item_spacing.y = 0.0;
            // Chữ trong popup nhỏ hơn cửa sổ chính một chút cho cân đối với khung 420px
            ui.style_mut().text_styles.insert(TextStyle::Body, egui::FontId::proportional(12.0));

            // Phần đầu: nền xám nhạt, tiêu đề và tên thư mục
            let dau = CornerRadius { nw: BO_GOC, ne: BO_GOC, sw: 0, se: 0 };
            egui::Frame::new().fill(NEN_TIEU_DE).corner_radius(dau).inner_margin(Margin { left: 20, right: 20, top: 14, bottom: 12 }).show(ui, |ui| {
                ui.set_width(RONG - 40.0);
                ui.spacing_mut().item_spacing.y = 4.0;
                // Một dòng: "Đã đếm xong" + tên thư mục in đậm
                let tieu_de = if tb.da_dung { "Đã dừng đếm " } else { "Đã đếm xong " };
                let mut dong_tieu_de = egui::text::LayoutJob::default();
                let co_chu = egui::FontId::proportional(14.0);
                dong_tieu_de.append(tieu_de, 0.0, egui::TextFormat { font_id: co_chu, color: CHU, ..Default::default() });
                dong_tieu_de.append(&tb.ten_thu_muc, 0.0, egui::TextFormat { font_id: egui::FontId::new(14.0, egui::FontFamily::Name(HO_FONT_DAM.into())), color: CHU, ..Default::default() });
                ui.add(egui::Label::new(dong_tieu_de).truncate());
            });
            ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, VIEN_NHAT));

            // Phần thông tin: nhãn xám bên trái, giá trị đậm bên phải, thẳng hàng
            egui::Frame::new().inner_margin(Margin { left: 20, right: 20, top: 16, bottom: 16 }).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::Grid::new("thong_tin_xong").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                    ui.label(RichText::new("Số file pdf:").color(CHU_PHU));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(RichText::new(format!("{} (đếm),", so_nguyen(tb.so_file_dem))).dam());
                        ui.label(RichText::new(format!("{} (lỗi)", so_nguyen(tb.so_file_loi))).dam().color(if tb.so_file_loi > 0 { DO_SAM } else { CHU }));
                    });
                    ui.end_row();
                    ui.label(RichText::new("Số trang:").color(CHU_PHU));
                    ui.label(RichText::new(format!("{} (không quy) / {} (quy A4)", so_nguyen(tb.so_trang), so_thuc(tb.quy_a4))).dam());
                    ui.end_row();
                    ui.label(RichText::new("Thời gian đếm:").color(CHU_PHU));
                    ui.label(RichText::new(thoi_gian_phut_giay(tb.thoi_gian)).dam());
                    ui.end_row();
                });
                if let Err(e) = &tb.ket_qua {
                    ui.add_space(10.0);
                    ui.colored_label(DO_SAM, e.as_str());
                }
                if co_telegram {
                    // Báo công khai cho người dùng: app gửi thống kê sử dụng về quản trị viên
                    ui.add_space(10.0);
                    ui.add(egui::Label::new(RichText::new(THONG_BAO_THONG_KE).small().color(CHU_PHU)).truncate())
                        .on_hover_text("Sau mỗi lần đếm xong, app gửi tên người dùng, tên máy, địa chỉ MAC, thư mục và số trang đã đếm về nhóm Telegram của quản trị viên");
                }
            });
            ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, VIEN_NHAT));

            // Phần nút: nền xám như cửa sổ chính.
            // Trái: Mở kết quả (nền như nút thường) · Giữa: Copy dữ liệu · Phải: Đóng
            let cuoi = CornerRadius { nw: 0, ne: 0, sw: BO_GOC, se: BO_GOC };
            egui::Frame::new().fill(NEN).corner_radius(cuoi).inner_margin(Margin { left: 20, right: 20, top: 12, bottom: 12 }).show(ui, |ui| {
                ui.set_width(RONG - 40.0);
                // Ba nút cùng độ rộng, căn giữa mỗi cột đều nhau → khoảng cách giữa các nút bằng nhau
                let rong_nut = egui::vec2(104.0, 28.0);
                ui.columns(3, |c| {
                    c[0].with_layout(Layout::top_down(Align::Center), |ui| {
                        if tb.ket_qua.is_ok() {
                            mo = ui.add(nut("Mở kết quả").min_size(rong_nut)).clicked();
                        }
                    });
                    c[1].with_layout(Layout::top_down(Align::Center), |ui| {
                        if ui.add_enabled(chep.is_some(), nut("Copy dữ liệu").min_size(rong_nut)).clicked() {
                            if let Some(s) = &chep {
                                ui.ctx().copy_text(s.clone());
                            }
                        }
                    });
                    c[2].with_layout(Layout::top_down(Align::Center), |ui| {
                        dong = ui.add(nut("Đóng").min_size(rong_nut)).clicked();
                    });
                });
            });
        });
        if mo {
            if let Ok(p) = &tb.ket_qua {
                mo_bang_he_dieu_hanh(p);
            }
        }
        if mo || dong || phan_hoi.should_close() {
            self.thong_bao = None;
        }
    }
}

impl eframe::App for UngDung {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ve(ui);
    }
}

impl UngDung {
    /// Màn Đếm trang A4.
    fn ve(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if let Some((kq, xong)) = self.nhan_su_kien() {
            self.luu_va_thong_bao(&kq, xong);
        }
        if let Some(h) = &mut self.hop_license {
            use crate::hop_license::KetQuaHop;
            match h.ve(&ctx, &self.license) {
                KetQuaHop::Mo => {}
                KetQuaHop::Dong => self.hop_license = None,
                KetQuaHop::DuocChay => {
                    self.hop_license = None;
                    self.bat_dau_dem(&ctx);
                }
            }
        }
        if self.dang_dem.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::bottom("trang_thai").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 4, bottom: 8 })).show(ui, |ui| self.thanh_trang_thai(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 10, bottom: 4 })).show(ui, |ui| {
            self.hang_thu_muc(ui);
            ui.add_space(2.0);
            self.dong_tom_tat(ui);
            ui.add_space(4.0);
            self.bang_ket_qua(ui);
        });

        self.popup_xong(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinh_dang_so_kieu_viet_nam() {
        assert_eq!(so_nguyen(5063), "5.063");
        assert_eq!(so_nguyen(999), "999");
        assert_eq!(so_nguyen(1234567), "1.234.567");
        assert_eq!(so_thuc(885.0), "885");
        assert_eq!(so_thuc(885.5), "885,5");
        assert_eq!(so_thuc(12345.126), "12.345,13");
        assert_eq!(thoi_gian_phut_giay(Duration::from_millis(65_400)), "1 phút, 5,4 giây");
        assert_eq!(thoi_gian_phut_giay(Duration::from_millis(1_900)), "0 phút, 1,9 giây");
    }

    fn app_mau(noi_dung: &str) -> UngDung {
        let mut app = UngDung::default();
        for dong in noi_dung.lines() {
            app.them_dong(dong.to_string());
        }
        app
    }

    #[test]
    fn loc_theo_kho_hien_moi_trang_cua_file_co_kho_do() {
        let noi_dung = "1/2\t/a/x.pdf => trang: 1\t595.0\t842.0\tDoc\tA4\t300\t\t-\n\
                        1/2\t/a/x.pdf => trang: 2\t842.0\t1191.0\tDoc\tA3\t300\t\t-\n\
                        2/2\t/a/y.pdf => trang: 1\t595.0\t842.0\tDoc\tA4\t300\tSat nguong A5 (90%)\tCó";
        let mut app = app_mau(noi_dung);
        app.bo_loc = BoLoc::Kho(3); // A3: chỉ file x.pdf, cả 2 trang
        assert_eq!(app.dong_hien().unwrap(), &[0, 1]);
        app.bo_loc = BoLoc::Kho(4); // A4: cả 2 file
        assert_eq!(app.dong_hien().unwrap(), &[0, 1, 2]);
        app.bo_loc = BoLoc::CanhBao;
        assert_eq!(app.dong_hien().unwrap(), &[2]);
        app.them_dong("Lỗi mở tệp: /a/z.pdf".into());
        app.bo_loc = BoLoc::Loi;
        assert_eq!(app.dong_hien().unwrap(), &[3]);
        app.bo_loc = BoLoc::CanhBao; // lỗi không lẫn vào cảnh báo
        assert_eq!(app.dong_hien().unwrap(), &[2]);
        app.bo_loc = BoLoc::TatCa;
        assert!(app.dong_hien().is_none());

        // Lọc theo cột Ký số: x.pdf (2 trang) = "-", y.pdf (1 trang) = "Có"
        app.loc_ky_so = LocKySo::Co;
        assert_eq!(app.dong_hien().unwrap(), &[2]);
        app.loc_ky_so = LocKySo::TatCa;
        // Kết hợp tab A4 + ký số Có: chỉ trang A4 của y.pdf
        app.bo_loc = BoLoc::Kho(4);
        app.loc_ky_so = LocKySo::Co;
        assert_eq!(app.dong_hien().unwrap(), &[2]);
    }

    #[test]
    fn quy_a4_tinh_lai_khi_doi_he_so() {
        let mut app = app_mau("1/1\t/a/x.pdf => trang: 1\t595.0\t842.0\tDoc\tA4\t300\t\t-\n1/1\t/a/x.pdf => trang: 2\t842.0\t1191.0\tDoc\tA3\t300\t\t-");
        assert_eq!(app.quy_a4_hien_tai(), Some(3.0)); // mặc định: 1 trang A4 × 1 + 1 trang A3 × 2
        app.quy_tac.he_so[3] = 2.5;
        assert_eq!(app.quy_a4_hien_tai(), Some(3.5));
    }

    #[test]
    fn trang_lon_hon_a0_quy_a4_theo_dien_tich_that() {
        // Trang A0 chuẩn tính 16; trang 2x A0 (4767,88 x 3370,39) vẫn là khổ A0 nhưng quy ~32,06 A4
        let mut app = app_mau(
            "1/1\t/a/x.pdf => trang: 1\t2383.94\t3370.39\tDọc\tA0\t-\t\t-\n\
             1/1\t/a/x.pdf => trang: 2\t4767.88\t3370.39\tNgang\tA0\t-\t\t-",
        );
        let q = app.quy_a4_hien_tai().unwrap();
        assert_eq!(q, 48.06); // 16 + 32,06 (2 x 16,03), làm tròn 2 chữ số
        // Hệ số A0 là mức sàn: nâng lên 40 thì trang chuẩn tính 40, trang lớn (~32) cũng lên 40
        app.quy_tac.he_so[0] = 40.0;
        assert_eq!(app.quy_a4_hien_tai(), Some(80.0));
    }

    /// Lưu Cấu hình › Đếm → màn Đếm tính lại Quy A4 theo hệ số mới
    #[test]
    fn luu_cau_hinh_cap_nhat_quy_a4_man_dem() {
        let mut app = UngDungChinh { dem: app_mau("1/1\t/a/x.pdf => trang: 1\t842.0\t1191.0\tDoc\tA3\t300\t\t-"), ..Default::default() };
        let mut cfg = crate::cau_hinh::CauHinh::default();
        app.ap_cau_hinh(cfg.clone(), false);
        assert_eq!(app.dem.quy_a4_hien_tai(), Some(2.0));
        cfg.he_so[3] = 5.0;
        app.ap_cau_hinh(cfg, false);
        assert_eq!(app.dem.quy_a4_hien_tai(), Some(5.0));
    }

    fn harness_chinh(man: Man) -> egui_kittest::Harness<'static, UngDungChinh> {
        egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 700.0)).build_eframe(move |cc| {
            cai_giao_dien(&cc.egui_ctx);
            UngDungChinh { man, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() }
        })
    }

    /// Hết lượt Đếm: popup Liên hệ Sao Mai (lý do, mã máy), không đếm; Sao Mai nâng hạn → Kiểm tra lại thì đếm luôn.
    #[test]
    fn het_luot_dem_hien_popup_kiem_tra_lai_thi_dem() {
        use egui_kittest::kittest::Queryable;
        let goc = std::env::temp_dir().join(format!("dema4_het_luot_dem_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(&goc).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("a.pdf"), 1);
        let kho = Arc::new(crate::tram::KhoGia::moi(0, 5));
        let kt = crate::license::KiemTra::gia(kho.clone());
        kt.mo_app();
        kt.moc().recv().unwrap();
        let ma = kt.ma_may().to_string();
        let tm = goc.display().to_string();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 700.0)).build_eframe(move |cc| {
            cai_giao_dien(&cc.egui_ctx);
            let mut app = UngDungChinh { man: Man::Dem, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() };
            app.dem.license = kt.clone();
            app.dem.thu_muc = tm.clone();
            app
        });
        h.run();
        // Nút Đếm ở thanh chọn thư mục (nút đầu tiên có nhãn "Đếm")
        h.get_all_by_label("Đếm").next().unwrap().click();
        h.run();
        h.get_by_label("Lý do: Số lần Đếm còn lại (");
        h.get_by_label(&ma);
        h.get_by_label(crate::hop_license::DIEN_THOAI);
        assert!(h.state().dem.dang_dem.is_none(), "hết lượt: không chạy đếm");
        *kho.han.lock().unwrap() = crate::license::HanMuc { dem: 5, tach: 5, muc_luc: 5 };
        h.get_by_label("Kiểm tra lại").click();
        let bat_dau = Instant::now();
        while h.state().dem.hop_license.is_some() {
            assert!(bat_dau.elapsed() < Duration::from_secs(5), "Kiểm tra lại không xong");
            std::thread::sleep(Duration::from_millis(20));
            h.run();
        }
        assert!(h.state().dem.dang_dem.is_some() || h.state().dem.thong_bao.is_some(), "được cấp lượt → đếm chạy luôn");
        let _ = std::fs::remove_dir_all(&goc);
    }

    /// Màn Tách: chưa chọn thư mục → giới thiệu; chọn thư mục → bỏ giới thiệu, hiện số hồ sơ + nút Phân tích.
    #[test]
    fn chon_thu_muc_tach_bo_gioi_thieu() {
        use egui_kittest::kittest::Queryable;
        let goc = std::env::temp_dir().join(format!("dema4_bo_gioi_thieu_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join("con")).unwrap();
        for f in ["a.pdf", "b.pdf", "con/c.pdf"] {
            std::fs::write(goc.join(f), b"%PDF-1.4").unwrap();
        }
        let mut h = harness_chinh(Man::Tach);
        h.run();
        h.get_by_label("Duyệt nhãn");
        h.state_mut().tach.thu_muc = goc.display().to_string();
        let bat_dau = Instant::now();
        while h.query_by_label("3 hồ sơ PDF — bấm Phân tích để xem trang và gắn nhãn").is_none() {
            assert!(bat_dau.elapsed() < Duration::from_secs(5), "không hiện số hồ sơ");
            std::thread::sleep(Duration::from_millis(20));
            h.run_steps(2); // đang đọc danh sách: app tự vẽ lại liên tục
        }
        assert!(h.query_by_label("Duyệt nhãn").is_none(), "đã chọn thư mục → bỏ màn giới thiệu");
        assert_eq!(h.get_all_by_label("Phân tích").count(), 2, "nút Phân tích ở thanh thư mục + ở khung gợi ý");
        let _ = std::fs::remove_dir_all(&goc);
    }

    /// Menu Chức năng 3 mục đổi màn; F1 mở Hướng dẫn đúng tab của màn đang dùng; Đóng tắt hộp.
    #[test]
    fn menu_chuc_nang_doi_man_va_f1_mo_huong_dan() {
        use egui_kittest::kittest::Queryable;
        let mut h = harness_chinh(Man::Dem);
        h.run();
        h.get_by_label("Chức năng").click();
        h.run();
        for ten in ["Đếm trang A4", "Tách văn bản", "Ký văn bản"] {
            h.get_by_label_contains(ten);
        }
        h.get_by_label_contains("Tách văn bản").click();
        h.run();
        assert_eq!(h.state().man, Man::Tach);
        h.get_by_label("Thư mục hồ sơ PDF:");
        h.key_press(egui::Key::F1);
        h.run();
        assert!(matches!(h.state().tro_giup, Some(crate::tro_giup::HopTroGiup::HuongDan(crate::tro_giup::TabHuongDan::Tach))));
        h.get_by_label("Đóng").click();
        h.run();
        assert!(h.state().tro_giup.is_none());
        h.get_by_label("Chức năng").click();
        h.run();
        h.get_by_label_contains("Sinh mục lục").click();
        h.run();
        assert_eq!(h.state().man, Man::MucLuc);
        h.get_by_label("File biên mục:");
        h.key_press(egui::Key::F1);
        h.run();
        assert!(matches!(h.state().tro_giup, Some(crate::tro_giup::HopTroGiup::HuongDan(crate::tro_giup::TabHuongDan::MucLuc))));
        h.get_by_label("Đóng").click();
        h.run();
        h.get_by_label("Chức năng").click();
        h.run();
        h.get_by_label_contains("Ký văn bản").click();
        h.run();
        assert_eq!(h.state().man, Man::Ky);
        h.get_by_label_contains("Chức năng đang phát triển");
        h.key_press(egui::Key::F1);
        h.run();
        assert!(matches!(h.state().tro_giup, Some(crate::tro_giup::HopTroGiup::HuongDan(crate::tro_giup::TabHuongDan::Dem))));
    }

    /// Đang phân tích: khoá menu Chức năng + Cấu hình; nút Dừng vẫn bấm được.
    #[test]
    fn menu_khoa_khi_dang_phan_tich_nut_dung_van_bam_duoc() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let mut h = harness_chinh(Man::Tach);
        h.state_mut().tach.thu_muc = "/x".into();
        h.run();
        assert!(!h.get_by_label("Chức năng").accesskit_node().is_disabled());
        h.state_mut().tach.gia_dang_chay();
        h.run_steps(4); // vạch đang đọc chạy liên tục, không "lặng"
        assert!(h.get_by_label("Chức năng").accesskit_node().is_disabled());
        assert!(h.get_by_label("Cấu hình").accesskit_node().is_disabled());
        assert!(!h.get_by_label("Dừng").accesskit_node().is_disabled());
        assert!(h.query_by_label("Phân tích").is_none());
    }

    /// Hộp Cấu hình: Huỷ đóng không đổi gì; ô sai khoá Lưu.
    #[test]
    fn cau_hinh_huy_va_o_sai_khoa_luu() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let mut h = harness_chinh(Man::Tach);
        h.run();
        h.get_by_label("Cấu hình").click();
        h.run();
        assert!(h.state().hop_cau_hinh.is_some());
        assert!(!h.get_by_label("Lưu").accesskit_node().is_disabled());
        h.state_mut().hop_cau_hinh.as_mut().unwrap().nhap.mau_ten = "{hoso}".into();
        h.run();
        assert!(h.get_by_label("Lưu").accesskit_node().is_disabled(), "mẫu tên thiếu {{n}} → khoá Lưu");
        h.get_by_label_contains("Mẫu phải có");
        h.get_by_label("Huỷ").click();
        h.run();
        assert!(h.state().hop_cau_hinh.is_none());
        assert_eq!(h.state().cfg.mau_ten, "{hoso}.{n}");
    }

    /// Cửa sổ thấp (980×600) hay mặc định: nút Lưu/Huỷ của hộp Cấu hình vẫn nằm trong cửa sổ, cả 2 tab.
    #[test]
    fn cau_hinh_khong_tran_cua_so() {
        use egui_kittest::kittest::Queryable;
        for (rong, cao) in [(980.0, 600.0), (1100.0, 720.0), (800.0, 420.0)] {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(rong, cao)).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                UngDungChinh { man: Man::Dem, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() }
            });
            h.run();
            h.get_by_label("Cấu hình").click();
            h.run();
            for tab in ["Đếm trang A4", "Tách PDF"] {
                h.get_by_label(tab).click();
                h.run();
                for nut in ["Lưu", "Huỷ"] {
                    let r = h.get_by_label(nut).rect();
                    assert!(r.min.y >= 0.0 && r.max.y <= cao, "{rong}×{cao} tab {tab}: nút {nut} tràn ({r:?})");
                }
            }
        }
    }

    #[test]
    fn luong_dem_chet_khong_lam_sap_giao_dien() {
        let mut app = UngDung::default();
        let (gui, nhan) = mpsc::channel::<SuKien>();
        app.dang_dem = Some(DangDem { nhan, dung: Arc::new(AtomicBool::new(false)), thu_muc: "x".into(), bat_dau: Instant::now(), lan: app.license.bat_dau(crate::license::ChucNang::Dem).unwrap() });
        drop(gui); // luồng đếm panic/chết: đầu gửi bị huỷ mà chưa có Xong
        assert!(app.nhan_su_kien().is_none());
        assert!(app.dang_dem.is_none());
        assert!(app.trang_thai.starts_with("Lỗi"), "{}", app.trang_thai);
    }

    #[test]
    fn them_dong_du_lieu_di_dang_khong_panic() {
        let mut app = UngDung::default();
        for dong in [
            "",
            "\t",
            "STT\tTrang",
            "1/2",
            "a\tb\tc\td\te\tf\tg\th",       // thiếu cột
            "a\tb\tc\td\te\tf\tg\th\ti\tj", // thừa cột
            "abc/5\t/a/x.pdf => trang: 1\t595\t842\tDọc\tA4\t300\t\tCó",
            "0/5\t/a/x.pdf => trang: 2\t595\t842\tDọc\tA4\t300\t\tCó",
            "18446744073709551615/18446744073709551615\t/a/y.pdf => trang: 1\t\t\t\tZZ\t\t\t",
            "5/0\t/a/z.pdf\t-\t-\t-\t-\t-\t-\t-",
            "Lỗi mở tệp: ",
        ] {
            app.them_dong(dong.to_string());
        }
        // 4 dòng 9 cột + 1 dòng lỗi được nhận, các dòng khác bị bỏ qua; mọi bộ lọc chạy không panic
        assert_eq!(app.dong.len(), 5);
        assert_eq!(app.kho_cua_file.len(), 3, "x.pdf, y.pdf, z.pdf là 3 file");
        for loc in [BoLoc::CanhBao, BoLoc::Loi, BoLoc::Kho(0), BoLoc::Kho(4), BoLoc::TatCa] {
            app.bo_loc = loc;
            let _ = app.dong_hien();
        }
        app.loc_ky_so = LocKySo::Co;
        assert_eq!(app.dong_hien().unwrap(), &[0, 1]);
    }

    #[test]
    fn nut_copy_chi_co_ket_qua_sau_khi_dem_xong() {
        let mut app = UngDung::default();
        assert!(app.ket_qua_chep.is_none());
        let kq = KetQuaDem { dong_tong_hop: vec![], dong_chi_tiet: vec![], tong_hop: "\nKẾT QUẢ ĐẾM\n===\n".into(), so_file_loi: 0, tong_so_trang: 0, quy_a4: None, so_trang_canh_bao: 0 };
        app.thu_muc = std::env::temp_dir().display().to_string();
        let xong = DaXong { thu_muc: std::env::temp_dir().join("dema4_kiem_thu_copy").display().to_string(), thoi_gian: Duration::from_secs(1), da_dung: false };
        std::fs::create_dir_all(&xong.thu_muc).unwrap();
        app.luu_va_thong_bao(&kq, xong);
        assert_eq!(app.ket_qua_chep.as_deref(), Some("KẾT QUẢ ĐẾM\n===\n"));
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("dema4_kiem_thu_copy"));
    }

    /// Chụp màn Đếm sau khi đếm xong thư mục mẫu 1 (cùng biến môi trường như `chup_man_tach`).
    #[test]
    #[ignore = "cần GPU và thư mục mẫu"]
    fn chup_man_dem() {
        let mau = crate::tach_vb::thu_muc_mau().expect("DEMA4_MAU_TACH");
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 760.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
            cai_giao_dien(&cc.egui_ctx);
            let mut app = UngDungChinh { man: Man::Dem, ..Default::default() };
            app.cfg = crate::cau_hinh::CauHinh::default();
            app.dem.thu_muc = mau.join("pdf_goc").display().to_string();
            app
        });
        harness.run_steps(2);
        harness.render().unwrap().save(thu_muc_anh.join("dem_mo_dau.png")).unwrap();
        let ctx = harness.ctx.clone();
        harness.state_mut().dem.bat_dau_dem(&ctx);
        let bat_dau = Instant::now();
        while harness.state().dem.dang_dem.is_some() && bat_dau.elapsed() < Duration::from_secs(120) {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("dem_ket_qua.png")).unwrap();
    }

    /// Chụp màn Tách sau khi phân tích thư mục mẫu 1.
    /// Chạy: DEMA4_MAU_TACH=<thư mục mẫu> DEMA4_ANH=<thư mục ảnh> cargo test --release -- --ignored chup_man_tach
    #[test]
    #[ignore = "cần GPU và thư mục mẫu"]
    fn chup_man_tach() {
        let mau = crate::tach_vb::thu_muc_mau().expect("DEMA4_MAU_TACH");
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 760.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
            cai_giao_dien(&cc.egui_ctx);
            let mut app = UngDungChinh { man: Man::Tach, ..Default::default() };
            app.cfg = crate::cau_hinh::CauHinh::default();
            app.tach.thu_muc = mau.join("pdf_goc").display().to_string();
            app
        });
        // Vừa chọn thư mục, chưa Phân tích: cột trái hiện ngay tên các PDF
        for _ in 0..30 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.render().unwrap().save(thu_muc_anh.join("tach_0_xem_truoc.png")).unwrap();
        let ch = harness.state().cfg.cau_hinh_phan_tich();
        harness.state_mut().tach.bam_phan_tich(&ch);
        let bat_dau = Instant::now();
        while harness.state().tach.dang_chay() && bat_dau.elapsed() < Duration::from_secs(120) {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        // Chờ luồng ảnh nhỏ render các ô đang hiện
        for _ in 0..40 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.render().unwrap().save(thu_muc_anh.join("tach_1_tat_ca.png")).unwrap();
        // Chọn 1 trang ở cuối danh sách → cột trái đánh dấu + tự cuộn tới hồ sơ; rồi thu gọn cột, rồi tìm
        let cuoi = harness.state().tach.thu_tu_hien().last().copied();
        if let Some(k) = cuoi {
            harness.state_mut().tach.chon.chon_rieng(k);
        }
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_1b_dang_xem.png")).unwrap();
        harness.state_mut().tach.trai_gon = true;
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_1c_gon.png")).unwrap();
        harness.state_mut().tach.trai_gon = false;
        harness.state_mut().tach.tim_hs = "2".into();
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_1d_tim.png")).unwrap();
        harness.state_mut().tach.tim_hs.clear();
        harness.state_mut().tach.chon.bo_het();
        harness.state_mut().tach.loc = crate::gui_tach::bang_trang::Loc::NghiTrang;
        harness.run_steps(2);
        let dau = harness.state().tach.thu_tu_hien().first().copied();
        if let Some(k) = dau {
            harness.state_mut().tach.chon.chon_rieng(k);
        }
        for _ in 0..20 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.render().unwrap().save(thu_muc_anh.join("tach_2_nghi_trang.png")).unwrap();
        let cfg = harness.state().cfg.clone();
        harness.state_mut().hop_cau_hinh = Some(crate::gui_tach::hop_cau_hinh::HopCauHinh::moi(&cfg, crate::gui_tach::hop_cau_hinh::TabCauHinh::Tach));
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_3_cau_hinh.png")).unwrap();
        let mut cfg_dem = cfg.clone();
        cfg_dem.ranh_gioi[3] = 380.0;
        let mut hop = crate::gui_tach::hop_cau_hinh::HopCauHinh::moi(&cfg_dem, crate::gui_tach::hop_cau_hinh::TabCauHinh::Dem);
        hop.dat_o_thu("370");
        harness.state_mut().hop_cau_hinh = Some(hop);
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_4_cau_hinh_dem.png")).unwrap();
        harness.state_mut().hop_cau_hinh = None;
        harness.state_mut().tro_giup = Some(crate::tro_giup::HopTroGiup::HuongDan(crate::tro_giup::TabHuongDan::Tach));
        harness.run_steps(3);
        harness.render().unwrap().save(thu_muc_anh.join("tach_5_huong_dan.png")).unwrap();
        // Tab Văn bản: Lưới, 1 trang 100 %, 2 trang 75 %, hộp phóng to
        harness.state_mut().tro_giup = None;
        harness.state_mut().tach.tab = crate::gui_tach::Tab::VanBan;
        harness.state_mut().tach.vb = 1;
        for (ten, che_do, ty_le) in [("tach_6_vb_luoi", crate::cau_hinh::CheDoVb::Luoi, 100), ("tach_7_vb_1trang", crate::cau_hinh::CheDoVb::MotTrang, 100), ("tach_8_vb_2trang", crate::cau_hinh::CheDoVb::HaiTrang, 75)] {
            harness.state_mut().cfg.che_do_vb = che_do;
            harness.state_mut().cfg.ty_le_vb = ty_le;
            for _ in 0..30 {
                harness.run_steps(1);
                std::thread::sleep(Duration::from_millis(50));
            }
            harness.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
        let ds = harness.state().tach.thu_tu_hien().to_vec();
        harness.state_mut().tach.phong_to = Some(crate::gui_tach::phong_to::PhongTo::moi(ds.clone(), ds[0]));
        for _ in 0..30 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.render().unwrap().save(thu_muc_anh.join("tach_9_phong_to.png")).unwrap();
        // Hộp phóng to ở tab Hồ sơ (có S I B)
        harness.state_mut().tach.phong_to = None;
        harness.state_mut().tach.tab = crate::gui_tach::Tab::HoSo;
        harness.state_mut().tach.loc = crate::gui_tach::bang_trang::Loc::TatCa;
        harness.run_steps(2);
        let ds = harness.state().tach.thu_tu_hien().to_vec();
        harness.state_mut().tach.phong_to = Some(crate::gui_tach::phong_to::PhongTo::moi(ds.clone(), ds[ds.len().min(10) - 1]));
        for _ in 0..20 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        harness.render().unwrap().save(thu_muc_anh.join("tach_9b_phong_to_hs.png")).unwrap();
        // Hộp phóng to trong bộ lọc Nghi trắng: có thanh hỏi duyệt như lưới
        harness.state_mut().tach.phong_to = None;
        harness.state_mut().tach.loc = crate::gui_tach::bang_trang::Loc::NghiTrang;
        harness.run_steps(2);
        let ds = harness.state().tach.thu_tu_hien().to_vec();
        if let Some(&k) = ds.first() {
            harness.state_mut().tach.phong_to = Some(crate::gui_tach::phong_to::PhongTo::moi(ds.clone(), k));
            for _ in 0..30 {
                harness.run_steps(1);
                std::thread::sleep(Duration::from_millis(50));
            }
            harness.render().unwrap().save(thu_muc_anh.join("tach_9c_phong_to_nghi.png")).unwrap();
        }
    }

    /// Chụp lúc đang Phân tích (bản sao thư mục mẫu không có cache): vạch tiến độ hồ sơ ở cột trái.
    /// Chạy: DEMA4_MAU_TACH=<thư mục mẫu> DEMA4_ANH=<thư mục ảnh> cargo test -- --ignored chup_dang_phan_tich
    #[test]
    #[ignore = "cần GPU và thư mục mẫu"]
    fn chup_dang_phan_tich() {
        let mau = crate::tach_vb::thu_muc_mau().expect("DEMA4_MAU_TACH");
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let ban_sao = std::env::temp_dir().join(format!("dema4_dang_pt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ban_sao);
        std::fs::create_dir_all(&ban_sao).unwrap();
        for f in std::fs::read_dir(mau.join("pdf_goc")).unwrap().flatten() {
            if f.path().extension().is_some_and(|e| e == "pdf") {
                std::fs::copy(f.path(), ban_sao.join(f.file_name())).unwrap();
            }
        }
        let tm = ban_sao.display().to_string();
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 760.0)).with_pixels_per_point(1.5).build_eframe(move |cc| {
            cai_giao_dien(&cc.egui_ctx);
            let mut app = UngDungChinh { man: Man::Tach, ..Default::default() };
            app.cfg = crate::cau_hinh::CauHinh::default();
            app.tach.thu_muc = tm.clone();
            let ch = app.cfg.cau_hinh_phan_tich();
            app.tach.bam_phan_tich(&ch);
            app
        });
        let bat_dau = Instant::now();
        let mut lan = 0;
        while harness.state().tach.dang_chay() && bat_dau.elapsed() < Duration::from_secs(120) {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
            lan += 1;
            if lan % 40 == 0 && lan <= 160 {
                harness.render().unwrap().save(thu_muc_anh.join(format!("dang_pt_{lan}.png"))).unwrap();
            }
        }
        let _ = std::fs::remove_dir_all(&ban_sao);
    }

    /// Vẽ giao diện ra ảnh PNG (không mở cửa sổ) từ một file chi tiết thật để xem bố cục.
    /// Chạy: DEMA4_MAU=<..._dem_chitiet.txt> DEMA4_ANH=<thư mục ảnh> cargo test -- --ignored chup_giao_dien
    /// Chụp hộp Cấu hình (2 tab) và Hướng dẫn để so với prototype. Chạy: DEMA4_ANH=<thư mục ảnh> cargo test -- --ignored chup_hop
    #[test]
    #[ignore = "cần GPU, chỉ chạy tay khi chỉnh giao diện"]
    fn chup_hop() {
        use egui_kittest::kittest::Queryable;
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        for (ten, tab) in [("ch_dem", "Đếm trang A4"), ("ch_tach", "Tách PDF")] {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                UngDungChinh { man: Man::Dem, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() }
            });
            h.run();
            h.get_by_label("Cấu hình").click();
            h.run();
            h.get_by_label(tab).click();
            h.run_steps(4);
            h.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
        use crate::tro_giup::{HopTroGiup, TabHuongDan};
        let hop_mau = |ten: &str| match ten {
            "hd_dem" => HopTroGiup::HuongDan(TabHuongDan::Dem),
            "hd_tach" => HopTroGiup::HuongDan(TabHuongDan::Tach),
            "thong_tin" => HopTroGiup::ThongTin,
            _ => {
                let (gui, nhan) = mpsc::channel();
                gui.send(Ok("v9.9.9".to_string())).unwrap();
                HopTroGiup::CapNhat { nhan, kq: None }
            }
        };
        for ten in ["hd_dem", "hd_tach", "thong_tin", "cap_nhat"] {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                UngDungChinh { man: Man::Dem, cfg: crate::cau_hinh::CauHinh::default(), tro_giup: Some(hop_mau(ten)), ..Default::default() }
            });
            h.run_steps(4);
            h.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
        {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                UngDungChinh { man: Man::Ky, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() }
            });
            h.run_steps(4);
            h.render().unwrap().save(thu_muc_anh.join("ky_mo_dau.png")).unwrap();
        }
        for ten in ["chua_tach", "phan_tich_lai", "dang_tach", "tach_xong", "het_luot", "het_luot_offline"] {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_pixels_per_point(1.5).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                let mut app = UngDungChinh { man: Man::Tach, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() };
                app.tach.mo_popup_mau(ten);
                app
            });
            h.run_steps(4);
            h.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
    }

    /// Chụp màn Sinh mục lục (fixture trong repo): mở đầu, tab Mẫu, đang sửa nguồn, tab Kết quả, popup đã sinh.
    /// Chạy: DEMA4_ANH=<thư mục ảnh> cargo test --release -- --ignored chup_man_muc_luc
    #[test]
    #[ignore = "cần GPU"]
    fn chup_man_muc_luc() {
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let tao = |nap: bool| {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1240.0, 820.0)).with_pixels_per_point(1.5).build_eframe(move |cc| {
                cai_giao_dien(&cc.egui_ctx);
                let mut app = UngDungChinh { man: Man::MucLuc, cfg: crate::cau_hinh::CauHinh::default(), ..Default::default() };
                app.muc_luc = if nap { crate::gui_muc_luc::tests::man_thu(5) } else { crate::gui_muc_luc::ManMucLuc::tu_cau_hinh(&Default::default()) };
                if nap {
                    // Ảnh dùng cho README (repo công khai): đường dẫn hiển thị trung tính, không lộ thư mục máy dev
                    app.muc_luc.file = r"D:\SoHoa\Bien_muc\Bien_muc_tong_hop.xlsx".into();
                }
                app
            });
            h.run_steps(3);
            h
        };
        let mut h = tao(false);
        h.render().unwrap().save(thu_muc_anh.join("ml_1_mo_dau.png")).unwrap();
        let mut h = tao(true);
        h.render().unwrap().save(thu_muc_anh.join("ml_2_mau.png")).unwrap();
        h.state_mut().muc_luc.mo = Some("nguoi_lap".into());
        h.run_steps(3);
        h.render().unwrap().save(thu_muc_anh.join("ml_3_sua_nguon.png")).unwrap();
        h.state_mut().muc_luc.mo = None;
        h.state_mut().muc_luc.tab = crate::gui_muc_luc::Tab::KetQua;
        h.run_steps(3);
        h.render().unwrap().save(thu_muc_anh.join("ml_4_ket_qua.png")).unwrap();
        h.state_mut().muc_luc.mo_hop_ghep_thu();
        h.run_steps(3);
        h.render().unwrap().save(thu_muc_anh.join("ml_5_hop_ghep.png")).unwrap();
        let mut h = tao(true);
        let cfg = h.state().cfg.clone();
        h.state_mut().hop_cau_hinh = Some(crate::gui_tach::hop_cau_hinh::HopCauHinh::moi(&cfg, crate::gui_tach::hop_cau_hinh::TabCauHinh::MucLuc));
        h.run_steps(3);
        h.render().unwrap().save(thu_muc_anh.join("ml_6_cau_hinh.png")).unwrap();
    }

    #[test]
    #[ignore = "cần GPU và file mẫu, chỉ chạy tay khi chỉnh giao diện"]
    fn chup_giao_dien() {
        let mau = std::env::var("DEMA4_MAU").expect("DEMA4_MAU");
        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let noi_dung = std::fs::read_to_string(&mau).unwrap();

        let tao_app = |dang_dem: bool, bo_loc: BoLoc, popup: bool| {
            let mut app = app_mau(&noi_dung);
            app.thu_muc = Path::new(&mau).parent().unwrap().display().to_string();
            app.them_dong(format!("Lỗi mở tệp: {}/loi/file_hong.pdf", app.thu_muc));
            app.bo_loc = bo_loc;
            if dang_dem {
                let (gui, nhan) = mpsc::channel();
                std::mem::forget(gui); // giữ kênh mở để app coi như đang đếm
                app.dang_dem = Some(DangDem { nhan, dung: Arc::new(AtomicBool::new(false)), thu_muc: app.thu_muc.clone(), bat_dau: Instant::now(), lan: app.license.bat_dau(crate::license::ChucNang::Dem).unwrap() });
                app.tien_do = Some((48, 121));
            } else {
                app.tien_do = Some((121, 121));
                app.ket_qua_chep = Some("KẾT QUẢ ĐẾM".into());
            }
            if popup {
                app.co_telegram = true;
                app.thong_bao = Some(ThongBaoXong {
                    ten_thu_muc: "appt".into(),
                    so_file_dem: 115,
                    so_file_loi: 6,
                    so_trang: 849,
                    quy_a4: 885.0,
                    thoi_gian: Duration::from_millis(65_400),
                    da_dung: false,
                    ket_qua: Ok(PathBuf::from("appt_dem.txt")),
                });
            }
            app
        };

        for (ten, dang_dem, bo_loc, popup) in [
            ("1_xong", false, BoLoc::TatCa, false),
            ("2_loc_a0", false, BoLoc::Kho(0), false),
            ("3_loc_canh_bao", false, BoLoc::CanhBao, false),
            ("4_dang_dem", true, BoLoc::TatCa, false),
            ("5_popup_xong", false, BoLoc::TatCa, true),
        ] {
            let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(980.0, 600.0)).with_pixels_per_point(2.0).build_eframe(|cc| {
                cai_giao_dien(&cc.egui_ctx);
                tao_app(dang_dem, bo_loc, popup)
            });
            harness.run_steps(4);
            harness.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
    }
}
