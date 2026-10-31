//! Màn Bóc tách dữ liệu (theo prototype đã duyệt `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`,
//! màn `#manBocTach`): khối đầu vào 2 dòng (Nguồn văn bản · Loại tài liệu / Mô hình đọc · Ghi ra), giới thiệu 3 bước
//! tới khi chọn đủ, rồi 2 tab Trường dữ liệu / Kết quả bóc; dòng đáy tóm tắt + nút Bắt đầu.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Align, Color32, Layout, Margin, Rect, RichText, Sense, Stroke};

use crate::boc_tach::loai_tai_lieu::{self, LoaiTaiLieu};
use crate::boc_tach::mo_hinh::NhaCungCap;
use crate::cau_hinh::CauHinhBocTach;
use crate::gui::{nut, nut_mau, so_nguyen, CHU, CHU_MO, CHU_PHU, DO, NEN, NEN_TIEU_DE, VIEN, VIEN_NHAT, VIEN_TAB_CHON, XANH_LA};
use crate::gui_muc_luc::{kieu_o_chon, mui_ten_chon};
use crate::gui_tach::bieu_tuong::{self as bt, BieuTuong};

pub mod cau_hinh;
mod hop_loai;
mod truong;
mod xem_pdf;

pub const TIEU_DE: &str = "Bóc tách dữ liệu (Giải pháp số Sao Mai)";
const RONG_NHAN: f32 = 104.0;
const RONG_NGUON: f32 = 150.0;
/// Khối phải 2 dòng đầu vào (Loại tài liệu / Ghi ra) — như khối phải màn Sinh mục lục.
const RONG_KHOI_PHAI: f32 = 354.0;
const CAO_TAB: f32 = 30.0;
const RONG_TAB: f32 = 170.0;
const NUT_TAT: Color32 = Color32::from_rgb(0xB8, 0xB8, 0xB8);
/// Giây đọc 1 trang khi chưa Thử đọc (ước tính thời gian ở dòng đáy).
const GIAY_MOI_TRANG: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Truong,
    KetQua,
}

/// File PDF trong thư mục nguồn (gồm thư mục con), số trang đếm ở luồng nền.
#[derive(Debug, Clone, PartialEq)]
pub struct FilePdf {
    pub duong_dan: PathBuf,
    /// Tên hiển thị: đường dẫn tương đối so với thư mục nguồn.
    pub ten: String,
    pub so_trang: usize,
}

/// Kết quả kiểm tra kết nối mô hình đang chọn.
enum KetNoi {
    Dang(NhaCungCap, Receiver<Result<(), String>>),
    Xong(NhaCungCap, Result<(), String>),
}

pub(crate) enum Hop {
    CauLenh(bool),
    Sua(hop_loai::BanSua),
    XacNhanXoa(hop_loai::BanSua),
    LuuMoi(hop_loai::BanSua, String),
}

pub struct ManBocTach {
    pub thu_muc: String,
    pub files: Vec<FilePdf>,
    quet: Option<Receiver<Result<Vec<FilePdf>, String>>>,
    loi_quet: Option<String>,
    chon_tm: Option<Receiver<Option<PathBuf>>>,
    pub ds_loai: Vec<LoaiTaiLieu>,
    pub loai: Option<usize>,
    pub ncc: Option<NhaCungCap>,
    ket_noi: Option<KetNoi>,
    pub tab: Tab,
    /// Thu gọn khối đầu vào (nút góc phải hàng tab).
    pub gon: bool,
    pub(crate) truong: truong::TabTruong,
    pub(crate) hop: Option<Hop>,
    /// Bấm mục "Cấu hình mô hình…" của ô Mô hình đọc → app mở Cấu hình › Bóc tách.
    pub mo_cau_hinh: bool,
    /// Bấm link Mô hình đọc / Loại tài liệu ở màn giới thiệu → mở ô chọn tương ứng.
    mo_o: Option<&'static str>,
}

impl Default for ManBocTach {
    fn default() -> Self {
        Self {
            thu_muc: String::new(),
            files: vec![],
            quet: None,
            loi_quet: None,
            chon_tm: None,
            ds_loai: loai_tai_lieu::doc(&loai_tai_lieu::file_mac_dinh()),
            loai: None,
            ncc: None,
            ket_noi: None,
            tab: Tab::Truong,
            gon: false,
            truong: Default::default(),
            hop: None,
            mo_cau_hinh: false,
            mo_o: None,
        }
    }
}

impl ManBocTach {
    /// Mở lại như lần đầu (chuyển từ chức năng khác sang).
    pub fn dat_lai(&mut self) {
        *self = Self::default();
    }

    pub fn dang_chay(&self) -> bool {
        false
    }

    pub fn loai_chon(&self) -> Option<&LoaiTaiLieu> {
        self.loai.and_then(|i| self.ds_loai.get(i))
    }

    /// Đủ 3 bước: có file PDF, đã chọn mô hình đọc và loại tài liệu.
    pub fn du_buoc(&self) -> [bool; 3] {
        [!self.files.is_empty(), self.ncc.is_some(), self.loai.is_some()]
    }

    fn san_sang(&self) -> bool {
        self.du_buoc().iter().all(|x| *x)
    }

    /// Số trang sẽ gửi đọc theo trang đọc của loại tài liệu.
    pub fn so_trang_doc(&self) -> usize {
        self.loai_chon().map_or(0, |l| self.files.iter().map(|f| l.trang_doc.cac_trang(f.so_trang).len()).sum())
    }

    pub fn chon_thu_muc(&mut self, thu_muc: String, ctx: &egui::Context) {
        self.thu_muc = thu_muc;
        self.files.clear();
        self.loi_quet = None;
        self.truong.dat_lai_file();
        if self.thu_muc.trim().is_empty() {
            self.quet = None;
            return;
        }
        let (gui, nhan) = mpsc::channel();
        let goc = PathBuf::from(self.thu_muc.trim());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = gui.send(quet_pdf(&goc));
            ctx.request_repaint();
        });
        self.quet = Some(nhan);
    }

    fn nhan_nen(&mut self, ctx: &egui::Context) {
        if let Some(n) = &self.quet {
            match n.try_recv() {
                Ok(kq) => {
                    self.quet = None;
                    match kq {
                        Ok(ds) => self.files = ds,
                        Err(e) => self.loi_quet = Some(e),
                    }
                    self.truong.dat_lai_file();
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(TryRecvError::Disconnected) => self.quet = None,
            }
        }
        if let Some(n) = &self.chon_tm {
            match n.try_recv() {
                Ok(kq) => {
                    self.chon_tm = None;
                    if let Some(p) = kq {
                        self.chon_thu_muc(p.display().to_string(), ctx);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.chon_tm = None,
            }
        }
        if let Some(KetNoi::Dang(n, nhan)) = &self.ket_noi {
            match nhan.try_recv() {
                Ok(kq) => self.ket_noi = Some(KetNoi::Xong(*n, kq)),
                Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(TryRecvError::Disconnected) => self.ket_noi = None,
            }
        }
    }

    fn kiem_tra_ket_noi(&mut self, ctx: &egui::Context, cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>) {
        let Some(n) = self.ncc else { return };
        let kn = cfg.ket_noi(n, khoa.get(n.ma()).map_or("", String::as_str));
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = gui.send(kn.liet_ke_model().map(|_| ()).map_err(|e| e.to_string()));
            ctx.request_repaint();
        });
        self.ket_noi = Some(KetNoi::Dang(n, nhan));
    }

    pub fn ve(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinhBocTach, khoa: &BTreeMap<String, String>) -> bool {
        let ctx = ui.ctx().clone();
        self.nhan_nen(&ctx);
        let mut cfg_doi = false;
        if !self.gon {
            cfg_doi |= self.hai_dong_dau_vao(ui, cfg, khoa);
            ui.add_space(8.0);
        }
        egui::Panel::bottom("bt_day").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 0, right: 0, top: 8, bottom: 8 })).show(ui, |ui| self.dong_day(ui, cfg));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN)).inner_margin(1)).show(ui, |ui| {
            self.thanh_tab(ui);
            ui.add_space(-ui.spacing().item_spacing.y);
            if !self.san_sang() {
                self.gioi_thieu(ui);
                return;
            }
            match self.tab {
                Tab::Truong => truong::ve(self, ui, cfg, khoa),
                Tab::KetQua => {
                    let r = ui.available_rect_before_wrap();
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "Kết quả bóc: bấm Bắt đầu để đọc các file", egui::FontId::proportional(12.0), CHU_PHU);
                }
            }
        });
        self.cac_hop(&ctx);
        cfg_doi
    }

    /// Màn giới thiệu khi chưa chọn đủ: cột trái trống như prototype, 3 bước (bước xong có dấu ✓).
    fn gioi_thieu(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("bt_trai_gt").exact_size(300.0).resizable(false).frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |_| {});
        let r = ui.available_rect_before_wrap();
        ui.painter().vline(r.left() - 0.5, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
            let xong = self.du_buoc();
            let noi = xong.iter().position(|x| !x).map(|i| i + 1);
            match crate::gioi_thieu::ve_trang_thai(ui, &crate::gioi_thieu::BOC_TACH, noi, xong) {
                Some(crate::gioi_thieu::Bam::Chon) if self.chon_tm.is_none() => self.chon_tm = Some(crate::gui_tach::chon_thu_muc_nen(&self.thu_muc, ui.ctx())),
                Some(crate::gioi_thieu::Bam::MoHinh) => self.mo_o = Some("bt_mo_hinh"),
                Some(crate::gioi_thieu::Bam::LoaiTaiLieu) | Some(crate::gioi_thieu::Bam::TruongDuLieu) => self.mo_o = Some("bt_loai"),
                _ => {}
            }
        });
    }

    fn nhan_co_dinh(ui: &mut egui::Ui, chu: &str) -> egui::Response {
        ui.allocate_ui_with_layout(egui::vec2(RONG_NHAN, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_width(RONG_NHAN);
            ui.label(chu)
        })
        .inner
    }

    /// 2 dòng đầu vào; trả `true` khi đổi tuỳ chọn cần ghi Cấu hình (ô Google Sheet).
    fn hai_dong_dau_vao(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinhBocTach, khoa: &BTreeMap<String, String>) -> bool {
        let khe = ui.spacing().item_spacing.x;
        let mut cfg_doi = false;
        let rong_giua = |ui: &egui::Ui| ui.available_width() - crate::gui::RONG_NUT - RONG_KHOI_PHAI - 3.0 * khe - 13.0;
        ui.horizontal(|ui| {
            let nhan = Self::nhan_co_dinh(ui, "Nguồn văn bản:");
            ui.scope(|ui| {
                kieu_o_chon(ui, Color32::WHITE, VIEN);
                egui::ComboBox::from_id_salt("bt_nguon")
                    .icon(mui_ten_chon)
                    .width(RONG_NGUON)
                    .selected_text("Máy tính")
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "Máy tính");
                        for ten in ["Google Drive", "Google Cloud Storage", "Kho S3"] {
                            ui.add_enabled(false, egui::Button::selectable(false, ten)).on_disabled_hover_text("Có ở bản sau");
                        }
                    })
                    .response
                    .labelled_by(nhan.id);
            });
            let w = rong_giua(ui) - RONG_NGUON - khe;
            let r = ui.add(egui::TextEdit::singleline(&mut self.thu_muc).hint_text("Chọn thư mục PDF (gồm cả thư mục con)").desired_width(w).margin(Margin { left: 8, ..crate::gui::LE_O_CO_NUT_XOA }));
            if !self.thu_muc.is_empty() && crate::gui::nut_xoa_trong_o(ui, r.rect, "Xoá đường dẫn") {
                self.chon_thu_muc(String::new(), ui.ctx());
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let tm = self.thu_muc.clone();
                self.chon_thu_muc(tm, ui.ctx());
            }
            if ui.add_enabled(self.chon_tm.is_none(), nut("Chọn")).clicked() {
                self.chon_tm = Some(crate::gui_tach::chon_thu_muc_nen(&self.thu_muc, ui.ctx()));
            }
            ui.add_space(10.0);
            ui.allocate_ui_with_layout(egui::vec2(RONG_KHOI_PHAI, 26.0), Layout::right_to_left(Align::Center), |ui| {
                let mut chon = None;
                ui.scope(|ui| {
                    kieu_o_chon(ui, Color32::WHITE, VIEN);
                    let hien = self.loai_chon().map_or_else(|| RichText::new("— Chọn loại tài liệu —"), |l| RichText::new(&l.ten));
                    let ph = egui::ComboBox::from_id_salt("bt_loai").icon(mui_ten_chon).width(RONG_KHOI_PHAI - 92.0).selected_text(hien).show_ui(ui, |ui| {
                        for (i, l) in self.ds_loai.iter().enumerate() {
                            if ui.selectable_label(self.loai == Some(i), &l.ten).clicked() {
                                chon = Some(i);
                            }
                        }
                    });
                    ph.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Loại tài liệu"));
                    if self.mo_o == Some("bt_loai") {
                        self.mo_o = None;
                        egui::Popup::open_id(ui.ctx(), ph.response.id.with("popup"));
                    }
                });
                ui.label(RichText::new("Loại tài liệu").color(CHU_PHU)).on_hover_text("Mỗi loại tài liệu có bộ trường, mô tả, chuẩn hoá và trang đọc riêng");
                if let Some(i) = chon {
                    self.loai = Some(i);
                    self.truong.chon = 0;
                    self.truong.bo_ket_qua_thu();
                }
            });
        });
        ui.add_space(8.0 - ui.spacing().item_spacing.y);
        ui.horizontal(|ui| {
            Self::nhan_co_dinh(ui, "Mô hình đọc:");
            let w = rong_giua(ui);
            // Mô hình đã khai đủ ở Cấu hình; ★ Mặc định đứng đầu
            let mut ds: Vec<NhaCungCap> = NhaCungCap::TAT_CA.into_iter().filter(|n| cfg.da_cai(*n, khoa.get(n.ma()).map_or("", String::as_str))).collect();
            ds.sort_by_key(|n| Some(*n) != cfg.mac_dinh);
            let mut chon = None;
            let mut mo_ch = false;
            let o = ui.scope(|ui| {
                kieu_o_chon(ui, Color32::WHITE, VIEN);
                let hien = self.ncc.map_or_else(|| RichText::new("— Chọn mô hình đọc —"), |n| RichText::new(cau_hinh::ten_mo_hinh(cfg, n)));
                let ph = egui::ComboBox::from_id_salt("bt_mo_hinh").icon(mui_ten_chon).width(w - 1.0).selected_text(hien).show_ui(ui, |ui| {
                    for n in &ds {
                        if ui.selectable_label(self.ncc == Some(*n), cau_hinh::ten_mo_hinh(cfg, *n)).clicked() {
                            chon = Some(*n);
                        }
                    }
                    if ds.is_empty() {
                        ui.label(RichText::new("Chưa khai mô hình nào").color(CHU_MO));
                    }
                    ui.separator();
                    if ui.selectable_label(false, "Cấu hình mô hình…").clicked() {
                        mo_ch = true;
                    }
                });
                ph.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Mô hình đọc"));
                if self.mo_o == Some("bt_mo_hinh") {
                    self.mo_o = None;
                    egui::Popup::open_id(ui.ctx(), ph.response.id.with("popup"));
                }
                ph.response.rect
            });
            let o = o.inner;
            // Trạng thái kết nối trong ô, trước mũi tên
            if let Some(KetNoi::Xong(n, kq)) = &self.ket_noi {
                if Some(*n) == self.ncc {
                    let (chu, mau) = match kq {
                        Ok(()) => ("Đã kết nối".to_string(), XANH_LA),
                        Err(_) => ("Lỗi kết nối".to_string(), DO),
                    };
                    let f = egui::FontId::proportional(12.0);
                    let g = ui.painter().layout_no_wrap(chu, f, mau);
                    let x = o.right() - 30.0 - g.size().x;
                    ui.painter().circle_filled(egui::pos2(x - 8.0, o.center().y), 3.5, mau);
                    let r = Rect::from_min_size(egui::pos2(x, o.center().y - g.size().y / 2.0), g.size());
                    ui.painter().galley(r.min, g, mau);
                    if let Err(e) = kq {
                        ui.interact(r, ui.scope_id().with("bt_loi_kn"), Sense::hover()).on_hover_text(e);
                    }
                }
            }
            if let Some(n) = chon {
                if self.ncc != Some(n) {
                    self.ncc = Some(n);
                    self.ket_noi = None;
                    self.truong.bo_ket_qua_thu();
                }
            }
            if mo_ch {
                self.mo_cau_hinh = true;
            }
            let dang = matches!(self.ket_noi, Some(KetNoi::Dang(..)));
            if ui.add_enabled(self.ncc.is_some() && !dang, nut(if dang { "Đang thử…" } else { "Kiểm tra" })).on_hover_text("Kiểm tra kết nối tới mô hình đọc").on_disabled_hover_text("Chọn mô hình đọc trước").clicked() {
                self.kiem_tra_ket_noi(ui.ctx(), cfg, khoa);
            }
            ui.add_space(10.0);
            ui.allocate_ui_with_layout(egui::vec2(RONG_KHOI_PHAI, 26.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("Ghi ra").color(CHU_PHU));
                ui.add_space(6.0);
                let mut excel = true;
                ui.add_enabled_ui(false, |ui| crate::gui_tach::khung_hop::o_tick(ui, &mut excel, "Excel")).response.on_disabled_hover_text("File Excel luôn được tạo");
                ui.add_space(8.0);
                if crate::gui_tach::khung_hop::o_tick(ui, &mut cfg.ghi_sheet, "Google Sheet").changed() {
                    cfg_doi = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let co_link = !cfg.sheet_link.trim().is_empty();
                    let l = ui.add_enabled(co_link, egui::Label::new(RichText::new("Mở Sheet").color(CHU)).sense(Sense::click()));
                    if co_link && l.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if l.on_disabled_hover_text("Nhập link Google Sheet ở Cấu hình › Bóc tách › Ghi kết quả").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(cfg.sheet_link.trim()));
                    }
                });
            });
        });
        cfg_doi
    }

    /// Hàng tab: 2 tab rộng cố định 170 px, gạch chọn 1 px; nút thu gọn khối đầu vào ở góc phải.
    fn thanh_tab(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_TAB), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, NEN_TIEU_DE);
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        let san = self.san_sang();
        let so_truong = self.loai_chon().map_or(0, |l| l.truong.len());
        let ds = [(Tab::Truong, if san { format!("Trường dữ liệu ({so_truong})") } else { "Trường dữ liệu".into() }), (Tab::KetQua, if san { format!("Kết quả bóc ({})", so_nguyen(self.files.len())) } else { "Kết quả bóc".into() })];
        for (i, (tab, chu)) in ds.iter().enumerate() {
            let chon = san && self.tab == *tab;
            let r = Rect::from_min_size(egui::pos2(rect.left() + i as f32 * RONG_TAB, rect.top()), egui::vec2(RONG_TAB, CAO_TAB));
            let ph = ui.interact(r, ui.scope_id().with(("bt_tab", i)), if san { Sense::click() } else { Sense::hover() });
            ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, san, chon, chu));
            if chon {
                ui.painter().rect_filled(r, 0.0, crate::gui::NEN_TAB_CHON);
                ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(1.0, VIEN_TAB_CHON));
            } else if san && ph.hovered() {
                ui.painter().rect_filled(r, 0.0, crate::gui::NUT_HOVER);
            }
            let font = if chon { egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(13.0) };
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, chu, font, CHU);
            if ph.clicked() {
                self.tab = *tab;
            }
        }
        // Thu gọn / mở khối đầu vào
        let o = Rect::from_center_size(egui::pos2(rect.right() - 18.0, rect.center().y), egui::vec2(24.0, 24.0));
        let ph = ui.interact(o, ui.scope_id().with("bt_gon"), Sense::click());
        let ten = if self.gon { "Hiện khối đầu vào" } else { "Thu gọn khối đầu vào" };
        ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, ten));
        if ph.hovered() {
            ui.painter().rect_filled(o, 3.0, crate::gui::NUT_HOVER);
        }
        bt::ve(ui.painter(), o.shrink(5.0), if self.gon { BieuTuong::HienDong } else { BieuTuong::AnDong }, CHU);
        if ph.on_hover_text(ten).clicked() {
            self.gon = !self.gon;
        }
    }

    /// Dòng đáy: File PDF · Trang sẽ đọc · Ước tính (song song) + nút Bắt đầu.
    fn dong_day(&mut self, ui: &mut egui::Ui, cfg: &CauHinhBocTach) {
        ui.horizontal(|ui| {
            let (thuong, dam) = (crate::gui_tach::chu_dong(CHU_PHU, false), crate::gui_tach::chu_dong(CHU, true));
            let mut job = egui::text::LayoutJob::default();
            if self.quet.is_some() {
                job.append("Đang tìm file PDF…", 0.0, thuong.clone());
            } else if let Some(e) = &self.loi_quet {
                job.append(e, 0.0, crate::gui_tach::chu_dong(DO, false));
            } else if !self.files.is_empty() {
                job.append("File PDF: ", 0.0, thuong.clone());
                job.append(&so_nguyen(self.files.len()), 0.0, dam.clone());
                if self.loai.is_some() {
                    let trang = self.so_trang_doc();
                    job.append(" · Trang sẽ đọc: ", 0.0, thuong.clone());
                    job.append(&so_nguyen(trang), 0.0, dam.clone());
                    if let Some(n) = self.ncc {
                        let ss = cfg.mo_hinh(n).song_song.max(1);
                        let giay = trang as f32 * self.truong.giay_moi_trang().unwrap_or(GIAY_MOI_TRANG) / ss as f32;
                        job.append(&format!(" · Ước tính ~{}", crate::gui_tach::gio_phut_giay(Duration::from_secs_f32(giay))), 0.0, thuong.clone());
                        job.append(&format!(" ({ss} yêu cầu song song)"), 0.0, crate::gui_tach::chu_dong(CHU_MO, false));
                    }
                }
            } else if !self.thu_muc.trim().is_empty() {
                job.append("Không có file PDF trong thư mục này", 0.0, thuong.clone());
            }
            if !job.text.is_empty() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                bt::ve(ui.painter(), r, BieuTuong::ThongTin, CHU_PHU);
                ui.label(job);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let bat = self.san_sang();
                let goi_y = if bat { "Đọc mọi file PDF trong thư mục" } else { "Chọn nguồn văn bản, mô hình đọc và loại tài liệu trước" };
                ui.add_enabled(bat, nut_mau("Bắt đầu", if bat { XANH_LA } else { NUT_TAT }).min_size(egui::vec2(130.0, 28.0))).on_hover_text(goi_y).on_disabled_hover_text(goi_y);
            });
        });
    }

    fn cac_hop(&mut self, ctx: &egui::Context) {
        let Some(hop) = &mut self.hop else { return };
        let mut dong = false;
        let mut sau: Option<Hop> = None;
        match hop {
            Hop::CauLenh(da_chep) => {
                if let Some(l) = self.loai.and_then(|i| self.ds_loai.get(i)) {
                    let so = self.truong.file_xem(&self.files).map_or(1, |f| f.so_trang);
                    dong = hop_loai::ve_cau_lenh(ctx, l, so, da_chep);
                } else {
                    dong = true;
                }
            }
            Hop::Sua(b) => match hop_loai::ve_sua(ctx, b) {
                hop_loai::HanhDong::Mo => {}
                hop_loai::HanhDong::Huy => dong = true,
                hop_loai::HanhDong::Luu => {
                    self.ds_loai[b.chi_so] = b.ket_qua();
                    let _ = loai_tai_lieu::ghi(&loai_tai_lieu::file_mac_dinh(), &self.ds_loai);
                    self.truong.bo_ket_qua_thu();
                    dong = true;
                }
                hop_loai::HanhDong::Xoa => sau = Some(Hop::XacNhanXoa(b.clone())),
                hop_loai::HanhDong::LuuMoi => sau = Some(Hop::LuuMoi(b.clone(), format!("{} (bản sao)", b.loai.ten.trim()))),
            },
            Hop::XacNhanXoa(b) => match hop_loai::ve_xn_xoa(ctx, &self.ds_loai[b.chi_so].ten, self.ds_loai[b.chi_so].truong.len()) {
                Some(true) => {
                    self.ds_loai.remove(b.chi_so);
                    if self.ds_loai.is_empty() {
                        self.ds_loai = loai_tai_lieu::co_san();
                    }
                    let _ = loai_tai_lieu::ghi(&loai_tai_lieu::file_mac_dinh(), &self.ds_loai);
                    self.loai = None;
                    self.truong.bo_ket_qua_thu();
                    dong = true;
                }
                Some(false) => sau = Some(Hop::Sua(b.clone())),
                None => {}
            },
            Hop::LuuMoi(b, ten) => {
                let da_co: Vec<String> = self.ds_loai.iter().map(|l| l.ten.clone()).collect();
                match hop_loai::ve_luu_moi(ctx, ten, &self.ds_loai[b.chi_so].ten, &da_co) {
                    Some(true) => {
                        let mut moi = b.ket_qua();
                        moi.ten = ten.trim().to_string();
                        self.ds_loai.push(moi);
                        let _ = loai_tai_lieu::ghi(&loai_tai_lieu::file_mac_dinh(), &self.ds_loai);
                        self.loai = Some(self.ds_loai.len() - 1);
                        self.truong.bo_ket_qua_thu();
                        dong = true;
                    }
                    Some(false) => sau = Some(Hop::Sua(b.clone())),
                    None => {}
                }
            }
        }
        if let Some(h) = sau {
            self.hop = Some(h);
        } else if dong {
            self.hop = None;
        }
    }
}

/// Mọi file .pdf trong `goc` (gồm thư mục con), xếp theo đường dẫn; đếm số trang từng file.
fn quet_pdf(goc: &Path) -> Result<Vec<FilePdf>, String> {
    if !goc.is_dir() {
        return Err(format!("Không mở được thư mục {}", goc.display()));
    }
    let mut ds: Vec<PathBuf> = jwalk::WalkDir::new(goc)
        .skip_hidden(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")))
        .map(|e| e.path())
        .collect();
    ds.sort();
    Ok(ds
        .into_iter()
        .map(|p| {
            let so_trang = mupdf::Document::open(p.as_path()).and_then(|d| d.page_count()).map_or(0, |n| n.max(0) as usize);
            let ten = p.strip_prefix(goc).unwrap_or(&p).to_string_lossy().into_owned();
            FilePdf { duong_dan: p, ten, so_trang }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quet_pdf_gom_thu_muc_con_va_dem_trang() {
        let d = std::env::temp_dir().join(format!("dema4_bt_quet_{}", std::process::id()));
        std::fs::create_dir_all(d.join("HS02")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&d.join("b.pdf"), 2);
        crate::tach_vb::tach::tests::tao_pdf(&d.join("HS02/a.PDF"), 3);
        std::fs::write(d.join("ghi_chu.txt"), "x").unwrap();
        let ds = quet_pdf(&d).unwrap();
        let ten: Vec<_> = ds.iter().map(|f| (f.ten.replace('\\', "/"), f.so_trang)).collect();
        assert_eq!(ten, [("HS02/a.PDF".to_string(), 3), ("b.pdf".to_string(), 2)]);
        assert!(quet_pdf(&d.join("khong_co")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn trang_se_doc_theo_loai() {
        let mut m = ManBocTach { ds_loai: loai_tai_lieu::co_san(), ..Default::default() };
        m.files = [1, 4, 2].iter().map(|&n| FilePdf { duong_dan: PathBuf::new(), ten: String::new(), so_trang: n }).collect();
        assert_eq!(m.so_trang_doc(), 0, "chưa chọn loại");
        m.loai = Some(0);
        assert_eq!(m.so_trang_doc(), 1 + 2 + 2, "đầu + cuối");
        m.loai = Some(2);
        assert_eq!(m.so_trang_doc(), 7, "hợp đồng đọc mọi trang");
        assert_eq!(m.du_buoc(), [true, false, true]);
    }
}
