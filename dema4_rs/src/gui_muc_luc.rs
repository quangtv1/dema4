//! Màn Sinh mục lục hồ sơ (bố cục đã duyệt trong prototype): 2 dòng đầu vào (file biên mục + sheet + dòng tiêu đề;
//! mẫu + Ghép biến), khung tab Mẫu / Kết quả với sidebar kéo được, dòng đáy + nút Sinh mục lục, popup tiến độ / kết quả.
//! Đọc Excel ở luồng nền; kiểm tra dữ liệu chạy lại sau mỗi thay đổi; sinh DOCX ở luồng nền, tính lượt license `muc_luc`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Layout, Margin, Rect, RichText, Sense, Stroke};

use crate::cau_hinh::{CauHinhMucLuc, RONG_TRAI_ML};
use crate::gui::{nut, nut_mau, so_nguyen, CHU, CHU_MO, CHU_PHU, DO, NEN, NEN_TIEU_DE, VIEN, VIEN_NHAT, VIEN_TAB_CHON, XANH_LA};
use crate::muc_luc::doc_bang::{self, BangDuLieu};
use crate::muc_luc::kiem_tra::{self, KetQuaKiemTra};
use crate::muc_luc::mau::{self, MauDaBien};
use crate::muc_luc::mo_phong::{self, Khoi};
use crate::muc_luc::{khop, sinh, Nguon, RangBuoc};

mod bang_bien;
mod chay;
mod hop_ghep;
mod trang;

use bang_bien::{Loai, NhomBien};

pub const TIEU_DE: &str = "Sinh mục lục hồ sơ (Giải pháp số Sao Mai)";
/// Khối bên phải 2 dòng đầu vào (Sheet + Dòng tiêu đề / Ghép biến) — 2 dòng thẳng cột nhau.
const RONG_KHOI_PHAI: f32 = 320.0;
const RONG_NHAN: f32 = 104.0;
const CAO_TAB: f32 = 32.0;
const CAM: Color32 = Color32::from_rgb(0xB8, 0x5F, 0x00);
const NUT_TAT: Color32 = Color32::from_rgb(0xB8, 0xB8, 0xB8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Mau,
    KetQua,
}

/// Ô chọn như prototype: nền `nen`, viền `vien`, bo 3 px; rê chuột viền đậm hơn.
pub(super) fn kieu_o_chon(ui: &mut egui::Ui, nen: Color32, vien: Color32) {
    let w = &mut ui.visuals_mut().widgets;
    for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        v.weak_bg_fill = nen;
        v.bg_fill = nen;
        v.corner_radius = egui::CornerRadius::same(3);
        v.bg_stroke = Stroke::new(1.0, vien);
        v.expansion = 0.0;
    }
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_gray(0x8A));
}

/// Mũi tên ⌄ nét mảnh của ô chọn (thay tam giác đặc mặc định của egui).
pub(super) fn mui_ten_chon(ui: &egui::Ui, r: Rect, _: &egui::style::WidgetVisuals, _: bool) {
    let c = r.center();
    let mau = if ui.is_enabled() { Color32::from_gray(0x55) } else { CHU_MO };
    ui.painter().add(egui::Shape::line(vec![c + egui::vec2(-4.5, -2.0), c + egui::vec2(0.0, 2.5), c + egui::vec2(4.5, -2.0)], Stroke::new(1.3, mau)));
}

/// Mẫu đang dùng: "1".."4" = mẫu nhúng sẵn, còn lại = đường dẫn DOCX riêng.
fn ten_mau(ma: &str) -> String {
    match ma.parse::<usize>() {
        Ok(i) if (1..=mau::MAU_CO_SAN.len()).contains(&i) => mau::MAU_CO_SAN[i - 1].0.to_string(),
        _ => format!("Mẫu riêng: {}", Path::new(ma).file_name().map_or_else(|| ma.to_string(), |s| s.to_string_lossy().into_owned())),
    }
}

fn byte_mau(ma: &str) -> Result<Vec<u8>, String> {
    match ma.parse::<usize>() {
        Ok(i) if (1..=mau::MAU_CO_SAN.len()).contains(&i) => Ok(mau::MAU_CO_SAN[i - 1].1.to_vec()),
        _ => std::fs::read(ma).map_err(|e| format!("không đọc được mẫu {ma}: {e}")),
    }
}

pub struct DuLieu {
    /// File đã đọc (ô đường dẫn có thể đang gõ dở file khác).
    pub duong_dan: PathBuf,
    pub bang: Arc<BangDuLieu>,
    pub khac_nhau: Vec<usize>,
    pub ds_sheet: Vec<String>,
    pub sheet: String,
}

pub struct MauDung {
    pub db: Arc<MauDaBien>,
    pub nhom: NhomBien,
    pub trang: Vec<Khoi>,
}

struct DangSinh {
    nhan: Receiver<sinh::SuKien>,
    dung: Arc<AtomicBool>,
    lan: crate::license::LanChay,
    i: usize,
    ten: String,
    bat_dau: Instant,
    tong: usize,
    van_ban: usize,
    thu_muc: PathBuf,
}

struct DaSinh {
    tk: sinh::TongKet,
    tong: usize,
    van_ban: usize,
    thu_muc: PathBuf,
    co_telegram: bool,
}

pub struct ManMucLuc {
    pub license: Arc<crate::license::KiemTra>,
    pub file: String,
    o_dong: String,
    pub mau_chon: String,
    pub du_lieu: Option<DuLieu>,
    loi_doc: Option<String>,
    doc_nen: Option<Receiver<Result<DuLieu, String>>>,
    chon_file: Option<Receiver<Option<PathBuf>>>,
    chon_mau: Option<Receiver<Option<PathBuf>>>,
    /// Hộp lưu mẫu đang mở (link "Lưu mẫu" ở đuôi ô chọn mẫu) và kết quả lần lưu gần nhất (hiện ở dòng đáy).
    luu_mau: Option<Receiver<Option<PathBuf>>>,
    da_luu_mau: Option<(Result<PathBuf, String>, Instant)>,
    pub mau: Option<MauDung>,
    loi_mau: Option<String>,
    /// Ràng buộc đã lưu (mẫu, JSON) — áp lại khi đọc xong dữ liệu.
    bien_luu: (String, String),
    pub rb: RangBuoc,
    pub kq: KetQuaKiemTra,
    canh_bao_hs: Vec<Option<String>>,
    pub tab: Tab,
    pub hs: usize,
    tim_hs: String,
    pub mo: Option<String>,
    o_go: String,
    sang: Option<String>,
    zoom: f32,
    trang_kq: Option<(usize, u64, Vec<Khoi>)>,
    phien_rb: u64,
    rong_trai: f32,
    sinh: Option<DangSinh>,
    da_sinh: Option<DaSinh>,
    hop_license: Option<crate::hop_license::HopLicense>,
    yeu_cau_sinh: bool,
    /// Hộp Ghép biến đang mở: ràng buộc lúc mở (Huỷ trả lại); `mo_hop` = biến đang sửa trong hộp.
    hop_ghep: Option<RangBuoc>,
    mo_hop: Option<String>,
    /// Lựa chọn thay đổi → ghi `.dema4_saomai.cfg` (gui.rs ghi).
    pub cfg_doi: bool,
}

impl Default for ManMucLuc {
    fn default() -> Self {
        Self::tu_cau_hinh(&CauHinhMucLuc::default())
    }
}

impl ManMucLuc {
    pub fn tu_cau_hinh(c: &CauHinhMucLuc) -> Self {
        let mut m = ManMucLuc {
            license: crate::license::KiemTra::mac_dinh(),
            file: c.file.clone(),
            o_dong: c.dong_tieu_de.to_string(),
            mau_chon: c.mau.clone(),
            du_lieu: None,
            loi_doc: None,
            doc_nen: None,
            chon_file: None,
            chon_mau: None,
            luu_mau: None,
            da_luu_mau: None,
            mau: None,
            loi_mau: None,
            bien_luu: (c.mau.clone(), c.bien.clone()),
            rb: RangBuoc::default(),
            kq: KetQuaKiemTra::default(),
            canh_bao_hs: Vec::new(),
            tab: Tab::Mau,
            hs: 0,
            tim_hs: String::new(),
            mo: None,
            o_go: String::new(),
            sang: None,
            zoom: 1.0,
            trang_kq: None,
            phien_rb: 0,
            rong_trai: c.rong_trai,
            sinh: None,
            da_sinh: None,
            hop_license: None,
            yeu_cau_sinh: false,
            hop_ghep: None,
            mo_hop: None,
            cfg_doi: false,
        };
        m.nap_mau();
        if !m.file.trim().is_empty() && Path::new(m.file.trim()).is_file() {
            m.doc_du_lieu(None, Some(c.sheet.clone()));
        }
        m
    }

    pub fn dang_chay(&self) -> bool {
        self.sinh.is_some()
    }

    /// Ghi lựa chọn hiện tại vào cấu hình (gọi khi `cfg_doi`).
    pub fn ghi_vao(&self, c: &mut CauHinhMucLuc) {
        c.file = self.file.trim().to_string();
        if let Some(d) = &self.du_lieu {
            c.sheet = d.sheet.clone();
        }
        c.dong_tieu_de = self.dong_tieu_de() as u32;
        c.mau = self.mau_chon.clone();
        c.rong_trai = self.rong_trai;
        // Ràng buộc chỉ lưu cho đúng mẫu nó thuộc về (đổi mẫu khi chưa nạp dữ liệu → không mang sang mẫu mới)
        c.bien = if self.bien_luu.0 == self.mau_chon { self.bien_luu.1.clone() } else { String::new() };
    }

    fn dong_tieu_de(&self) -> usize {
        self.o_dong.trim().parse::<usize>().ok().filter(|n| *n >= 1).unwrap_or(1)
    }

    // ---- Nạp dữ liệu / mẫu, ràng buộc, kiểm tra ----

    fn nap_mau(&mut self) {
        let kq = byte_mau(&self.mau_chon).and_then(|b| {
            let db = mau::bien_dich(&b)?;
            let trang = mo_phong::mo_phong(&b)?;
            Ok(MauDung { nhom: NhomBien { nhom: khop::nhom_bien(&db) }, db: Arc::new(db), trang })
        });
        match kq {
            Ok(m) => {
                self.mau = Some(m);
                self.loi_mau = None;
            }
            Err(e) => {
                self.mau = None;
                self.loi_mau = Some(e);
            }
        }
        self.ap_rang_buoc();
    }

    /// Đọc file ở luồng nền: danh sách sheet, bảng (sheet chọn / sheet đã nhớ / sheet đầu), số giá trị khác nhau.
    pub fn doc_du_lieu(&mut self, ctx: Option<&egui::Context>, sheet: Option<String>) {
        let p = PathBuf::from(self.file.trim());
        let dong = self.dong_tieu_de();
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.cloned();
        std::thread::spawn(move || {
            let kq = (|| {
                let ds_sheet = doc_bang::danh_sach_sheet(&p)?;
                let sheet = sheet.filter(|s| ds_sheet.contains(s)).or_else(|| ds_sheet.first().cloned()).unwrap_or_default();
                let bang = doc_bang::doc(&p, (!sheet.is_empty()).then_some(sheet.as_str()), dong)?;
                Ok(DuLieu { khac_nhau: khop::dem_khac_nhau(&bang), bang: Arc::new(bang), ds_sheet, sheet, duong_dan: p.clone() })
            })();
            let _ = gui.send(kq);
            if let Some(c) = ctx {
                c.request_repaint();
            }
        });
        self.doc_nen = Some(nhan);
    }

    /// Ràng buộc khi đổi dữ liệu / mẫu: dùng lại lựa chọn đã lưu cho đúng mẫu này (bỏ cột không còn), không có → tự khớp.
    fn ap_rang_buoc(&mut self) {
        let (Some(m), Some(d)) = (&self.mau, &self.du_lieu) else {
            self.kiem_tra_lai();
            return;
        };
        let tu_khop = khop::tu_khop(&m.db, &d.bang.tieu_de);
        let luu = (self.bien_luu.0 == self.mau_chon).then(|| RangBuoc::doc_json(&self.bien_luu.1)).flatten();
        self.rb = match luu {
            Some(mut rb) => {
                let co_cot = |c: &str| d.bang.cot(c).is_some();
                let tat_ca: Vec<&String> = m.nhom.nhom.iter().flatten().collect();
                rb.bien.retain(|(b, n)| tat_ca.contains(&b) && !matches!(n, Nguon::Cot(c) if !co_cot(c)));
                if !co_cot(&rb.cot_nhom) {
                    rb.cot_nhom = tu_khop.cot_nhom.clone();
                }
                for (b, n) in tu_khop.bien.iter().filter(|(_, n)| *n == Nguon::TuDong) {
                    if rb.nguon(b).is_none() {
                        rb.bien.push((b.clone(), n.clone()));
                    }
                }
                rb
            }
            None => tu_khop,
        };
        self.kiem_tra_lai();
    }

    fn kiem_tra_lai(&mut self) {
        self.phien_rb += 1;
        self.trang_kq = None;
        let (Some(m), Some(d)) = (&self.mau, &self.du_lieu) else {
            self.kq = KetQuaKiemTra::default();
            self.canh_bao_hs.clear();
            return;
        };
        self.kq = kiem_tra::kiem_tra(&m.db, &d.bang, &self.rb);
        // Cảnh báo theo hồ sơ: ô trống ở cột đã ghép (mục lục để trống ô đó)
        self.canh_bao_hs = self
            .kq
            .ho_so
            .iter()
            .map(|h| {
                let mut dau = None;
                let mut so = 0;
                for &r in &h.dong {
                    for (b, n) in &self.rb.bien {
                        let Nguon::Cot(c) = n else { continue };
                        let trong_bang = m.db.bien_dong.contains(b);
                        if (trong_bang || r == h.dong[0]) && d.bang.o(r, c).trim().is_empty() {
                            so += 1;
                            dau.get_or_insert_with(|| format!("dòng {} trống cột {c} — ô để trống trong mục lục", d.bang.so_dong.get(r).copied().unwrap_or(r + 1)));
                        }
                    }
                }
                dau.map(|s| if so > 1 { format!("{s} (+{} ô trống khác)", so - 1) } else { s })
            })
            .collect();
        self.hs = self.hs.min(self.kq.ho_so.len().saturating_sub(1));
    }

    pub fn dat_nguon(&mut self, bien: &str, nguon: Option<Nguon>) {
        self.rb.bien.retain(|(b, _)| b != bien);
        if let Some(n) = nguon {
            self.rb.bien.push((bien.to_string(), n));
        }
        self.mo = None;
        self.da_doi();
    }

    pub fn dat_nhom(&mut self, cot: &str) {
        self.rb.cot_nhom = cot.to_string();
        self.da_doi();
    }

    fn da_doi(&mut self) {
        self.bien_luu = (self.mau_chon.clone(), self.rb.json());
        self.cfg_doi = true;
        self.kiem_tra_lai();
    }

    fn co_du_lieu(&self) -> bool {
        self.du_lieu.is_some() && self.mau.is_some()
    }

    fn so_van_ban(&self) -> usize {
        self.kq.ho_so.iter().map(|h| h.dong.len()).sum()
    }

    fn co_the_sinh(&self) -> bool {
        self.co_du_lieu() && self.kq.loi.is_empty() && !self.kq.ho_so.is_empty() && self.sinh.is_none()
    }

    // ---- Nhận kết quả luồng nền ----

    fn nhan_nen(&mut self, ctx: &egui::Context) {
        if let Some(n) = &self.chon_file {
            match n.try_recv() {
                Ok(p) => {
                    self.chon_file = None;
                    if let Some(p) = p {
                        self.file = p.display().to_string();
                        self.cfg_doi = true;
                        self.doc_du_lieu(Some(ctx), None);
                    }
                }
                Err(TryRecvError::Disconnected) => self.chon_file = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(n) = &self.chon_mau {
            match n.try_recv() {
                Ok(p) => {
                    self.chon_mau = None;
                    if let Some(p) = p {
                        self.mau_chon = p.display().to_string();
                        self.cfg_doi = true;
                        self.nap_mau();
                    }
                }
                Err(TryRecvError::Disconnected) => self.chon_mau = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(n) = &self.doc_nen {
            match n.try_recv() {
                Ok(kq) => {
                    self.doc_nen = None;
                    match kq {
                        Ok(d) => {
                            self.du_lieu = Some(d);
                            self.loi_doc = None;
                            self.hs = 0;
                        }
                        Err(e) => {
                            self.du_lieu = None;
                            self.loi_doc = Some(e);
                        }
                    }
                    self.ap_rang_buoc();
                    self.cfg_doi = true;
                }
                Err(TryRecvError::Disconnected) => self.doc_nen = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(n) = &self.luu_mau {
            match n.try_recv() {
                Ok(p) => {
                    self.luu_mau = None;
                    if let Some(p) = p {
                        let kq = byte_mau(&self.mau_chon).and_then(|b| std::fs::write(&p, b).map_err(|e| format!("không ghi được {}: {e}", p.display()))).map(|_| p);
                        self.da_luu_mau = Some((kq, Instant::now()));
                    }
                }
                Err(TryRecvError::Disconnected) => self.luu_mau = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        // Báo "Đã lưu mẫu" ở dòng đáy 15 giây
        if self.da_luu_mau.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(15)) {
            self.da_luu_mau = None;
        }
        if self.da_luu_mau.is_some() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        self.nhan_sinh();
        if self.chon_file.is_some() || self.chon_mau.is_some() || self.luu_mau.is_some() || self.doc_nen.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self.sinh.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    // ---- Vẽ ----

    pub fn ve(&mut self, ui: &mut egui::Ui, cfg: &CauHinhMucLuc) {
        let ctx = ui.ctx().clone();
        self.nhan_nen(&ctx);
        self.hai_dong_dau_vao(ui);
        ui.add_space(8.0);
        egui::Panel::bottom("ml_day").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 0, right: 0, top: 8, bottom: 8 })).show(ui, |ui| self.dong_day(ui, cfg));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN)).inner_margin(1)).show(ui, |ui| {
            self.thanh_tab(ui);
            ui.add_space(-ui.spacing().item_spacing.y);
            let trai = egui::Panel::left("ml_trai")
                .resizable(true)
                .default_size(self.rong_trai)
                .size_range(RONG_TRAI_ML.0..=RONG_TRAI_ML.1)
                .frame(egui::Frame::new().fill(Color32::WHITE))
                .show(ui, |ui| self.sidebar(ui));
            let rong = trai.response.rect.width().round();
            if (rong - self.rong_trai).abs() >= 1.0 && !ctx.input(|i| i.pointer.any_down()) {
                self.rong_trai = rong;
                self.cfg_doi = true;
            }
            egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| self.khung_phai(ui));
        });
        if std::mem::take(&mut self.yeu_cau_sinh) {
            self.bam_sinh(cfg);
        }
        self.cac_hop(&ctx, cfg);
    }

    fn nhan_co_dinh(ui: &mut egui::Ui, chu: &str) -> egui::Response {
        ui.allocate_ui_with_layout(egui::vec2(RONG_NHAN, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_width(RONG_NHAN);
            ui.label(chu)
        })
        .inner
    }

    fn hai_dong_dau_vao(&mut self, ui: &mut egui::Ui) {
        let khoa = self.sinh.is_some();
        let khe = ui.spacing().item_spacing.x;
        let rong_o = |ui: &egui::Ui| ui.available_width() - crate::gui::RONG_NUT - RONG_KHOI_PHAI - 3.0 * khe - 10.0;
        ui.horizontal(|ui| {
            let nhan = Self::nhan_co_dinh(ui, "File biên mục:");
            let w = rong_o(ui);
            let r = ui.add_enabled(!khoa, egui::TextEdit::singleline(&mut self.file).hint_text("Chọn file Excel (.xlsx .xlsb .xls) hoặc CSV").desired_width(w)).labelled_by(nhan.id);
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.file.trim().is_empty() {
                self.doc_du_lieu(Some(ui.ctx()), None);
            }
            if ui.add_enabled(!khoa && self.chon_file.is_none(), nut("Chọn")).clicked() {
                self.chon_file = Some(chon_file_nen(&self.file, ui.ctx(), "Chọn file biên mục", &["xlsx", "xlsb", "xls", "xlsm", "csv"]));
            }
            ui.add_space(10.0);
            ui.allocate_ui_with_layout(egui::vec2(RONG_KHOI_PHAI, 26.0), Layout::left_to_right(Align::Center), |ui| {
                let nhan_sheet = ui.label(RichText::new("Sheet").color(CHU_PHU));
                let ds: Vec<String> = self.du_lieu.as_ref().map(|d| d.ds_sheet.clone()).unwrap_or_default();
                let hien = self.du_lieu.as_ref().map(|d| d.sheet.clone()).unwrap_or_default();
                let mut chon = None;
                ui.add_enabled_ui(!khoa && ds.len() > 1, |ui| {
                    kieu_o_chon(ui, Color32::WHITE, VIEN);
                    egui::ComboBox::from_id_salt("ml_sheet")
                        .icon(mui_ten_chon)
                        .width(ui.available_width() - 120.0)
                        .selected_text(if hien.is_empty() { "—".to_string() } else { hien.clone() })
                        .show_ui(ui, |ui| {
                            for s in &ds {
                                if ui.selectable_label(*s == hien, s).clicked() {
                                    chon = Some(s.clone());
                                }
                            }
                        })
                        .response
                        .labelled_by(nhan_sheet.id);
                });
                if let Some(s) = chon {
                    self.doc_du_lieu(Some(ui.ctx()), Some(s));
                }
                let nhan_dong = ui.label(RichText::new("Dòng tiêu đề").color(CHU_PHU));
                let r = ui.add_enabled(!khoa, egui::TextEdit::singleline(&mut self.o_dong).desired_width(44.0).align(egui::Align2::RIGHT_CENTER)).labelled_by(nhan_dong.id);
                if r.lost_focus() && !self.file.trim().is_empty() {
                    let sheet = self.du_lieu.as_ref().map(|d| d.sheet.clone());
                    self.doc_du_lieu(Some(ui.ctx()), sheet);
                }
            });
        });
        ui.add_space(8.0 - ui.spacing().item_spacing.y);
        ui.horizontal(|ui| {
            let nhan_mau = Self::nhan_co_dinh(ui, "Mẫu mục lục:");
            let w = rong_o(ui);
            let mut chon = None;
            ui.add_enabled_ui(!khoa, |ui| {
                kieu_o_chon(ui, Color32::WHITE, VIEN);
                let ph = egui::ComboBox::from_id_salt("ml_mau").icon(mui_ten_chon).width(w - 8.0).selected_text(ten_mau(&self.mau_chon)).show_ui(ui, |ui| {
                    for (i, (ten, _)) in mau::MAU_CO_SAN.iter().enumerate() {
                        let ma = (i + 1).to_string();
                        if ui.selectable_label(self.mau_chon == ma, *ten).clicked() {
                            chon = Some(ma);
                        }
                    }
                    if self.mau_chon.parse::<usize>().is_err() {
                        let _ = ui.selectable_label(true, ten_mau(&self.mau_chon));
                    }
                });
                let o = ph.response.rect;
                ph.response.labelled_by(nhan_mau.id);
                // Link "Lưu mẫu" ở đuôi ô chọn mẫu, trước mũi tên: chữ xám, rê chuột → đen + gạch chân
                let f = egui::FontId::proportional(12.0);
                let rong_chu = ui.fonts_mut(|x| x.layout_no_wrap("Lưu mẫu".into(), f.clone(), CHU).size().x);
                let r = Rect::from_center_size(egui::pos2(o.right() - 30.0 - rong_chu / 2.0, o.center().y), egui::vec2(rong_chu + 8.0, 18.0));
                let link = ui.interact(r, ui.scope_id().with("ml_luu_mau"), if khoa { Sense::hover() } else { Sense::click() });
                link.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, !khoa, "Lưu mẫu"));
                let re = link.hovered() && !khoa;
                let g = ui.painter().layout_no_wrap("Lưu mẫu".into(), f, if re { CHU } else { CHU_PHU });
                let goc = egui::pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0);
                if re {
                    ui.painter().hline(goc.x..=goc.x + g.size().x, goc.y + g.size().y - 1.0, Stroke::new(1.0, CHU));
                }
                ui.painter().galley(goc, g, CHU);
                if link.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Lưu mẫu đang chọn ra file DOCX — sửa trong Word rồi bấm Chọn để dùng làm mẫu riêng").clicked() && self.luu_mau.is_none() {
                    let ten = match self.mau_chon.parse::<usize>() {
                        Ok(_) => format!("{}.docx", khop::ten_file_khong_dau(&ten_mau(&self.mau_chon))),
                        Err(_) => Path::new(&self.mau_chon).file_name().map_or_else(|| "mau_muc_luc.docx".into(), |s| s.to_string_lossy().into_owned()),
                    };
                    self.luu_mau = Some(luu_file_nen(&ten, ui.ctx()));
                }
            });
            if let Some(ma) = chon {
                self.mau_chon = ma;
                self.cfg_doi = true;
                self.nap_mau();
            }
            if ui.add_enabled(!khoa && self.chon_mau.is_none(), nut("Chọn")).on_hover_text("Chọn mẫu DOCX riêng của đơn vị").clicked() {
                self.chon_mau = Some(chon_file_nen(&self.mau_chon, ui.ctx(), "Chọn mẫu mục lục (DOCX)", &["docx"]));
            }
            ui.add_space(10.0);
            let bat = !khoa && self.co_du_lieu();
            if ui.add_enabled(bat, egui::Button::new("Ghép biến…").min_size(egui::vec2(RONG_KHOI_PHAI, 26.0))).on_hover_text("Chọn nguồn dữ liệu cho từng biến của mẫu").on_disabled_hover_text("Chọn file biên mục trước").clicked() {
                self.mo_hop_ghep();
            }
        });
    }

    fn thanh_tab(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_TAB), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, NEN_TIEU_DE);
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        let co = self.co_du_lieu();
        let (tong_bien, chua) = self.mau.as_ref().map_or((0, 0), |m| (m.nhom.tong(), m.nhom.nhom.iter().flatten().filter(|b| m.nhom.loai(&self.rb, b) == Loai::ChuaGhep).count()));
        let ds = [(Tab::Mau, if co { format!("Mẫu ({}/{} biến)", tong_bien - chua, tong_bien) } else { "Mẫu".into() }), (Tab::KetQua, if co { format!("Kết quả ({})", so_nguyen(self.kq.ho_so.len())) } else { "Kết quả".into() })];
        // 2 tab chia đôi bề rộng sidebar (co giãn theo sidebar)
        let rong = (self.rong_trai / 2.0).round();
        for (i, (tab, chu)) in ds.iter().enumerate() {
            let chon = co && self.tab == *tab;
            let r = Rect::from_min_size(egui::pos2(rect.left() + i as f32 * rong, rect.top()), egui::vec2(rong, CAO_TAB));
            let ph = ui.interact(r, ui.scope_id().with(("ml_tab", i)), if co { Sense::click() } else { Sense::hover() });
            ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, co, chon, chu));
            if chon {
                ui.painter().rect_filled(r, 0.0, crate::gui::NEN_TAB_CHON);
                ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(2.0, VIEN_TAB_CHON));
            } else if co && ph.hovered() {
                ui.painter().rect_filled(r, 0.0, crate::gui::NUT_HOVER);
            }
            let font = if chon { egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(13.0) };
            let mut job = egui::text::LayoutJob::single_section(chu.clone(), egui::TextFormat { font_id: font, color: if co { CHU } else { CHU_MO }, ..Default::default() });
            job.wrap = egui::text::TextWrapping::truncate_at_width(rong - 12.0);
            let g = ui.fonts_mut(|f| f.layout_job(job));
            ui.painter().galley(r.center() - g.size() / 2.0, g, CHU);
            if ph.clicked() {
                self.tab = *tab;
            }
        }
        // Cảnh báo đầu khung: biến chưa ghép · hồ sơ cần xem / lỗi chặn — bấm tới hồ sơ đầu tiên
        let mut phan = Vec::new();
        if co && chua > 0 {
            phan.push(format!("{chua} biến chưa ghép"));
        }
        let loi_hs: Vec<usize> = self.canh_bao_hs.iter().enumerate().filter(|(_, c)| c.is_some()).map(|(i, _)| i).collect();
        if !loi_hs.is_empty() {
            let ten: Vec<String> = loi_hs.iter().take(3).map(|&i| self.kq.ho_so[i].khoa.clone()).collect();
            phan.push(format!("{} hồ sơ cần xem ({}{})", loi_hs.len(), ten.join(", "), if loi_hs.len() > 3 { ", …" } else { "" }));
        }
        let loi_chan = self.loi_doc.clone().or_else(|| self.loi_mau.clone()).or_else(|| self.kq.loi.first().cloned());
        let chu = match &loi_chan {
            Some(l) => l.clone(),
            None => phan.join(" · "),
        };
        if !chu.is_empty() {
            let mau = if loi_chan.is_some() { DO } else { CAM };
            let x_trai = rect.left() + 2.0 * rong + 30.0;
            let mut job = egui::text::LayoutJob::single_section(chu.clone(), egui::TextFormat { font_id: egui::FontId::proportional(12.0), color: mau, ..Default::default() });
            job.wrap = egui::text::TextWrapping::truncate_at_width((rect.right() - 12.0 - x_trai).max(20.0));
            let g = ui.fonts_mut(|f| f.layout_job(job));
            let r = Rect::from_min_size(egui::pos2(rect.right() - 12.0 - g.size().x, rect.center().y - g.size().y / 2.0), g.size());
            ui.painter().galley(r.min, g, mau);
            trang::tam_giac_canh_bao(ui.painter(), Rect::from_center_size(egui::pos2(r.left() - 10.0, r.center().y), egui::vec2(13.0, 13.0)), mau);
            let ph = ui.interact(r, ui.scope_id().with("ml_canh_bao"), Sense::click()).on_hover_text(&chu);
            if let Some(&i) = loi_hs.first() {
                if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.tab = Tab::KetQua;
                    self.hs = i;
                }
            }
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let co = self.co_du_lieu();
        let mau_tab = co && self.tab == Tab::Mau;
        egui::Panel::bottom("ml_day_trai").exact_size(crate::gui_tach::cot_trai::CAO_DAY).frame(egui::Frame::new().fill(Color32::from_gray(0xFA))).show(ui, |ui| {
            if mau_tab {
                let d = self.du_lieu.as_ref().expect("có dữ liệu");
                if let Some(c) = bang_bien::hang_nhom_theo(ui, &d.bang.tieu_de, &d.khac_nhau, &self.rb.cot_nhom) {
                    self.dat_nhom(&c);
                }
            } else {
                ui.painter().hline(ui.max_rect().x_range(), ui.max_rect().top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
                ui.horizontal_centered(|ui| {
                    ui.add_space(6.0);
                    ui.add_enabled(co, egui::TextEdit::singleline(&mut self.tim_hs).hint_text("Tìm hồ sơ").desired_width(ui.available_width() - 8.0).frame(egui::Frame::NONE));
                });
            }
        });
        if !co {
            return;
        }
        match self.tab {
            Tab::Mau => {
                let (Some(m), Some(d)) = (&self.mau, &self.du_lieu) else { return };
                let mut re = None;
                let v = bang_bien::VaoBang { nhom: &m.nhom, rb: &self.rb, cot: &d.bang.tieu_de, khac_nhau: &d.khac_nhau, mo: self.mo.as_deref(), sang: self.sang.as_deref() };
                let viec = bang_bien::ve(ui, &v, &mut self.o_go, &mut re);
                if ui.ui_contains_pointer() {
                    self.sang = re;
                }
                for v in viec {
                    match v {
                        bang_bien::Viec::MoSua(b) => {
                            self.o_go = match self.rb.nguon(&b) {
                                Some(Nguon::Hang(h)) => h.clone(),
                                _ => String::new(),
                            };
                            self.mo = Some(b);
                        }
                        bang_bien::Viec::DongSua => self.mo = None,
                        bang_bien::Viec::DatNguon(b, n) => self.dat_nguon(&b, n),
                    }
                }
            }
            Tab::KetQua => self.bang_ho_so(ui),
        }
    }

    fn bang_ho_so(&mut self, ui: &mut egui::Ui) {
        use crate::gui_tach::{bat_dau_bang, ke_duoi, o_so, CAO_DAU_BANG, CAO_DONG_BANG, VACH_DONG};
        use egui_extras::{Column, TableBuilder};
        let hien: Vec<usize> = (0..self.kq.ho_so.len()).filter(|&i| crate::gui_tach::cot_trai::khop_tim(&self.kq.ho_so[i].khoa, &self.tim_hs)).collect();
        bat_dau_bang(ui);
        let mut chon = None;
        TableBuilder::new(ui)
            .id_salt("ml_ho_so")
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(28.0))
            .column(Column::remainder().at_least(60.0).clip(true))
            .column(Column::exact(64.0))
            .column(Column::exact(28.0))
            .header(CAO_DAU_BANG, |mut h| {
                for (i, t) in ["#", "Hồ sơ", "Số dòng", ""].into_iter().enumerate() {
                    h.col(|ui| o_so(ui, RichText::new(t).color(CHU_PHU), i != 1));
                }
            })
            .body(|body| {
                body.rows(CAO_DONG_BANG, hien.len(), |mut row| {
                    let i = hien[row.index()];
                    let h = &self.kq.ho_so[i];
                    row.set_selected(i == self.hs);
                    row.col(|ui| {
                        ke_duoi(ui, VACH_DONG);
                        o_so(ui, RichText::new((i + 1).to_string()).color(CHU_PHU), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, VACH_DONG);
                        o_so(ui, RichText::new(if h.khoa.is_empty() { "(trống)" } else { h.khoa.as_str() }), false);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, VACH_DONG);
                        o_so(ui, RichText::new(so_nguyen(h.dong.len())), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, VACH_DONG);
                        if let Some(c) = &self.canh_bao_hs[i] {
                            let r = Rect::from_center_size(ui.max_rect().center(), egui::vec2(13.0, 13.0));
                            trang::tam_giac_canh_bao(ui.painter(), r, CAM);
                            ui.interact(ui.max_rect(), ui.scope_id().with(("ml_canh_hs", i)), Sense::hover()).on_hover_text(c);
                        }
                    });
                    if row.response().clicked() {
                        chon = Some(i);
                    }
                });
            });
        if let Some(i) = chon {
            self.hs = i;
        }
    }

    fn thanh_cong_cu(&mut self, ui: &mut egui::Ui, trai: impl FnOnce(&mut Self, &mut egui::Ui)) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), bang_bien::CAO_DAU), Sense::hover());
        ui.painter().rect_filled(r, 0.0, Color32::from_gray(0xFA));
        ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(egui::vec2(12.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
        con.spacing_mut().item_spacing.x = 6.0;
        trai(self, &mut con);
        con.with_layout(Layout::right_to_left(Align::Center), |ui| {
            use crate::gui_tach::bieu_tuong::{nut as nut_icon, BieuTuong};
            if nut_icon(ui, BieuTuong::KinhCong, "Phóng to", self.zoom < 2.0).clicked() {
                self.zoom = (self.zoom + 0.25).min(2.0);
            }
            ui.label(RichText::new(format!("{} %", (self.zoom * 100.0).round())).size(12.0).color(CHU_PHU));
            if nut_icon(ui, BieuTuong::KinhTru, "Thu nhỏ", self.zoom > 0.5).clicked() {
                self.zoom = (self.zoom - 0.25).max(0.5);
            }
        });
    }

    fn khung_phai(&mut self, ui: &mut egui::Ui) {
        if !self.co_du_lieu() {
            egui::Frame::new().show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                let dang_doc = self.doc_nen.is_some();
                if let Some(crate::gioi_thieu::Bam::Chon) = crate::gioi_thieu::ve(ui, &crate::gioi_thieu::MUC_LUC) {
                    if !dang_doc && self.chon_file.is_none() {
                        self.chon_file = Some(chon_file_nen(&self.file, ui.ctx(), "Chọn file biên mục", &["xlsx", "xlsb", "xls", "xlsm", "csv"]));
                    }
                }
            });
            return;
        }
        match self.tab {
            Tab::Mau => {
                let b = self.mau.as_ref().map(|m| (m.db.bang + 1, m.db.hang_mau + 1)).unwrap_or((1, 1));
                self.thanh_cong_cu(ui, |_, ui| {
                    ui.label(RichText::new(format!("Hàng mẫu: bảng {}, dòng {}", b.0, b.1)).size(12.0).color(CHU_PHU));
                    ui.add_space(8.0);
                    for (loai, ten) in [(Loai::ThongTin, "thông tin hồ sơ"), (Loai::Bang, "bảng văn bản"), (Loai::TuDong, "tự động"), (Loai::ChuaGhep, "chưa ghép")] {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
                        ui.painter().rect_filled(r, 2.0, loai.mau().0);
                        ui.label(RichText::new(ten).size(11.5).color(CHU_PHU));
                    }
                });
                let Some(m) = &self.mau else { return };
                let nhom = &m.nhom;
                let rb = &self.rb;
                let mau_chip = |t: &str| nhom.loai(rb, t).mau();
                let kieu = trang::KieuVe { zoom: self.zoom, mau_chip: Some(&mau_chip), sang: self.sang.as_deref() };
                let ph = trang::ve(ui, "ml_trang_mau", &m.trang, &kieu, None);
                if ui.ui_contains_pointer() {
                    self.sang = ph.re;
                }
                if let Some(b) = ph.bam {
                    if !m.nhom.nhom[2].contains(&b) {
                        self.o_go = match self.rb.nguon(&b) {
                            Some(Nguon::Hang(h)) => h.clone(),
                            _ => String::new(),
                        };
                        self.mo = Some(b);
                    }
                }
            }
            Tab::KetQua => {
                let n = self.kq.ho_so.len();
                if n == 0 {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| ui.label(RichText::new("Chưa có hồ sơ — kiểm tra cột Nhóm theo / dòng tiêu đề").color(CHU_PHU)));
                    return;
                }
                self.hs = self.hs.min(n - 1);
                let ten = self.kq.ho_so[self.hs].khoa.clone();
                let so_dong = self.kq.ho_so[self.hs].dong.len();
                self.thanh_cong_cu(ui, |s, ui| {
                    use crate::gui_tach::bieu_tuong::{nut as nut_icon, BieuTuong};
                    if nut_icon(ui, BieuTuong::TraiChevron, "Hồ sơ trước", s.hs > 0).clicked() {
                        s.hs -= 1;
                    }
                    ui.label(RichText::new(format!("{} / {}", s.hs + 1, n)).size(12.0));
                    if nut_icon(ui, BieuTuong::PhaiChevron, "Hồ sơ sau", s.hs + 1 < n).clicked() {
                        s.hs += 1;
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Mục lục hồ sơ {ten} · {so_dong} dòng")).size(12.0).color(CHU_PHU));
                });
                if self.trang_kq.as_ref().is_none_or(|(h, p, _)| *h != self.hs || *p != self.phien_rb) {
                    let khoi = match (&self.mau, &self.du_lieu) {
                        (Some(m), Some(d)) => crate::muc_luc::ghep::ghep(&m.db, &d.bang, &self.kq.ho_so[self.hs], &self.rb).and_then(|b| mo_phong::mo_phong(&b)).unwrap_or_else(|e| {
                            // Không dựng được trang xem trước: hiện lý do trên trang thay vì trang trắng
                            vec![Khoi::Doan(mo_phong::Doan { chu: format!("Không xem trước được hồ sơ này: {e}"), can: mo_phong::Can::Trai, dam: false, co: None })]
                        }),
                        _ => Vec::new(),
                    };
                    self.trang_kq = Some((self.hs, self.phien_rb, khoi));
                }
                let kieu = trang::KieuVe { zoom: self.zoom, mau_chip: None, sang: None };
                let canh_bao = self.canh_bao_hs.get(self.hs).cloned().flatten();
                if let Some((_, _, khoi)) = &self.trang_kq {
                    trang::ve(ui, "ml_trang_kq", khoi, &kieu, canh_bao.as_deref());
                }
            }
        }
    }

    fn dong_day(&mut self, ui: &mut egui::Ui, cfg: &CauHinhMucLuc) {
        ui.horizontal(|ui| {
            if self.sinh.is_none() {
                if let Some((kq, _)) = &self.da_luu_mau {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                    crate::gui_tach::bieu_tuong::ve(ui.painter(), r, crate::gui_tach::bieu_tuong::BieuTuong::ThongTin, CHU_PHU);
                    match kq {
                        Ok(p) => {
                            ui.label(RichText::new(format!("Đã lưu mẫu {}: {}", ten_mau(&self.mau_chon), p.display())).color(CHU_PHU));
                            if ui.link("Mở thư mục").clicked() {
                                if let Some(cha) = p.parent() {
                                    crate::gui::mo_bang_he_dieu_hanh(cha);
                                }
                            }
                        }
                        Err(e) => {
                            ui.label(RichText::new(format!("Không lưu được mẫu: {e}")).color(DO));
                        }
                    }
                    self.nut_day(ui, cfg);
                    return;
                }
            }
            // Như prototype: nhãn xám, số đậm ("Hồ sơ: **22**; Văn bản: **326**")
            let (thuong, dam) = (crate::gui_tach::chu_dong(CHU_PHU, false), crate::gui_tach::chu_dong(CHU, true));
            let mut job = egui::text::LayoutJob::default();
            match &self.sinh {
                Some(s) => {
                    job.append("Đang sinh ", 0.0, thuong.clone());
                    job.append(&format!("{}/{}", (s.i + 1).min(s.tong), s.tong), 0.0, dam.clone());
                    job.append(&format!(" hồ sơ · {} · đã chạy {}", s.ten, crate::gui_tach::gio_phut_giay(s.bat_dau.elapsed())), 0.0, thuong.clone());
                }
                None if self.co_du_lieu() => {
                    for (i, (ten, so)) in [("Hồ sơ", self.kq.ho_so.len()), ("Văn bản", self.so_van_ban())].into_iter().enumerate() {
                        job.append(&format!("{}{ten}: ", if i == 0 { "" } else { "; " }), 0.0, thuong.clone());
                        job.append(&so_nguyen(so), 0.0, dam.clone());
                    }
                }
                None if self.doc_nen.is_some() => job.append("Đang đọc file biên mục…", 0.0, thuong.clone()),
                None => {}
            }
            if !job.text.is_empty() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                crate::gui_tach::bieu_tuong::ve(ui.painter(), r, crate::gui_tach::bieu_tuong::BieuTuong::ThongTin, CHU_PHU);
                ui.label(job);
            }
            self.nut_day(ui, cfg);
        });
    }

    /// Bên phải dòng đáy: ☐ Kèm PDF + nút Sinh mục lục / Dừng.
    fn nut_day(&mut self, ui: &mut egui::Ui, cfg: &CauHinhMucLuc) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(s) = &self.sinh {
                    if ui.add(nut_mau("Dừng", DO).min_size(egui::vec2(130.0, 28.0))).on_hover_text("Dừng sau hồ sơ đang làm").clicked() {
                        s.dung.store(true, Ordering::Relaxed);
                    }
                } else {
                    let bat = self.co_the_sinh();
                    let goi_y = if !self.co_du_lieu() { "Chọn file biên mục và mẫu trước".to_string() } else if let Some(l) = self.kq.loi.first() { l.clone() } else { format!("Ghi ra: {}", self.thu_muc_ra(cfg).display()) };
                    if ui.add_enabled(bat, nut_mau("Sinh mục lục", if bat { XANH_LA } else { NUT_TAT }).min_size(egui::vec2(130.0, 28.0))).on_disabled_hover_text(&goi_y).clicked() {
                        self.yeu_cau_sinh = true;
                    }
                }
                let mut pdf = false;
                ui.add_enabled(false, egui::Checkbox::new(&mut pdf, "Kèm PDF")).on_disabled_hover_text("Xuất PDF cần Word / LibreOffice — có ở bản sau");
        });
    }
}

/// Hộp chọn file: Windows chạy ở luồng riêng (cửa sổ app không đứng hình) và đánh thức giao diện khi chọn xong;
/// macOS bắt buộc mở trên luồng chính (rfd).
/// Hộp lưu file mẫu (DOCX), cùng cách chạy như `chon_file_nen`.
fn luu_file_nen(ten: &str, ctx: &egui::Context) -> Receiver<Option<PathBuf>> {
    let (gui, nhan) = mpsc::channel();
    let ten = ten.to_string();
    let ctx = ctx.clone();
    let mo = move || {
        let _ = gui.send(rfd::FileDialog::new().set_title("Lưu mẫu mục lục").add_filter("Mẫu Word", &["docx"]).set_file_name(&ten).save_file());
        ctx.request_repaint();
    };
    if cfg!(target_os = "windows") {
        std::thread::spawn(mo);
    } else {
        mo();
    }
    nhan
}

fn chon_file_nen(hien_tai: &str, ctx: &egui::Context, tieu_de: &'static str, duoi: &'static [&'static str]) -> Receiver<Option<PathBuf>> {
    let (gui, nhan) = mpsc::channel();
    let dau = PathBuf::from(hien_tai.trim());
    let ctx = ctx.clone();
    let mo = move || {
        let mut hop = rfd::FileDialog::new().set_title(tieu_de).add_filter("Biên mục / mẫu", duoi);
        if let Some(cha) = dau.parent().filter(|p| p.is_dir()) {
            hop = hop.set_directory(cha);
        }
        let _ = gui.send(hop.pick_file());
        ctx.request_repaint();
    };
    if cfg!(target_os = "windows") {
        std::thread::spawn(mo);
    } else {
        mo();
    }
    nhan
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests_data/muc_luc/bien_muc_tong_hop.xlsx")
    }

    impl ManMucLuc {
        /// Nạp file biên mục đồng bộ (test không chờ luồng nền).
        pub fn nap_thu(&mut self, p: &Path, sheet: &str, dong: usize) {
            self.file = p.display().to_string();
            self.o_dong = dong.to_string();
            let bang = doc_bang::doc(p, Some(sheet), dong).unwrap();
            self.du_lieu = Some(DuLieu { khac_nhau: khop::dem_khac_nhau(&bang), ds_sheet: doc_bang::danh_sach_sheet(p).unwrap(), sheet: sheet.into(), bang: Arc::new(bang), duong_dan: p.into() });
            self.ap_rang_buoc();
        }

        pub fn mo_hop_ghep_thu(&mut self) {
            self.mo_hop_ghep();
            self.mo_hop = Some("nguoi_lap".into());
        }

        pub fn cho_sinh_xong(&mut self) {
            let bat_dau = Instant::now();
            while self.sinh.is_some() {
                assert!(bat_dau.elapsed() < Duration::from_secs(30), "sinh không xong");
                self.nhan_sinh();
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    /// Màn đã nạp fixture + mẫu 01, license giả (`han_ml` lượt Sinh mục lục).
    pub fn man_thu(han_ml: u32) -> ManMucLuc {
        let kt = crate::license::KiemTra::gia(Arc::new(crate::tram::KhoGia::moi(5, han_ml)));
        kt.mo_app();
        kt.moc().recv().unwrap();
        let mut m = ManMucLuc { license: kt, ..ManMucLuc::tu_cau_hinh(&CauHinhMucLuc::default()) };
        m.nap_thu(&fixture(), "Bien muc", 4);
        m
    }

    fn thu_muc(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_gui_ml_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn tu_khop_kiem_tra_va_sinh_docx() {
        let mut m = man_thu(5);
        assert_eq!(m.rb.cot_nhom, "Số hồ sơ");
        assert_eq!(m.rb.cot("stt"), Some("STT"));
        assert!(m.kq.loi.is_empty(), "{:?}", m.kq.loi);
        let n = m.kq.ho_so.len();
        assert!(n > 0 && m.co_the_sinh());
        let ra = thu_muc("sinh");
        let cfg = CauHinhMucLuc { thu_muc_ra: ra.display().to_string(), ..Default::default() };
        m.bam_sinh(&cfg);
        assert!(m.dang_chay());
        m.cho_sinh_xong();
        let x = m.da_sinh.as_ref().expect("popup Đã sinh");
        assert_eq!((x.tk.docx, x.tong), (n, n));
        assert_eq!(std::fs::read_dir(&ra).unwrap().count(), n);
        let _ = std::fs::remove_dir_all(&ra);
    }

    #[test]
    fn het_luot_hien_popup_khong_ghi_file() {
        let mut m = man_thu(0);
        let ra = thu_muc("het_luot");
        m.bam_sinh(&CauHinhMucLuc { thu_muc_ra: ra.display().to_string(), ..Default::default() });
        use crate::license::{ChucNang, QuyetDinh};
        assert!(!m.dang_chay());
        assert_eq!(m.hop_license.as_ref().map(|h| h.qd), Some(QuyetDinh::HetLuot(ChucNang::MucLuc)));
        assert!(!ra.exists());
    }

    #[test]
    fn sua_nguon_va_nhom_theo_cap_nhat_kiem_tra() {
        let mut m = man_thu(5);
        let chua: Vec<String> = m.kq.bien_chua_ghep.clone();
        assert!(chua.contains(&"nguoi_lap".to_string()));
        m.dat_nguon("nguoi_lap", Some(Nguon::Hang("Nguyễn Văn A".into())));
        assert!(!m.kq.bien_chua_ghep.contains(&"nguoi_lap".to_string()));
        assert!(m.cfg_doi, "đổi ràng buộc → ghi cấu hình");
        let mut c = CauHinhMucLuc::default();
        m.ghi_vao(&mut c);
        assert_eq!(RangBuoc::doc_json(&c.bien).unwrap().nguon("nguoi_lap"), Some(&Nguon::Hang("Nguyễn Văn A".into())));
        let truoc = m.kq.ho_so.len();
        m.dat_nhom("STT");
        assert_ne!(m.kq.ho_so.len(), truoc, "đổi Nhóm theo → số hồ sơ đổi");
        // Đổi sang mẫu khác: ràng buộc của mẫu cũ không bị lưu thành của mẫu mới
        let mut m3 = man_thu(5);
        m3.dat_nguon("nguoi_lap", Some(Nguon::Hang("A".into())));
        m3.mau_chon = "2".into();
        let mut c3 = CauHinhMucLuc::default();
        m3.ghi_vao(&mut c3);
        assert_eq!((c3.mau.as_str(), c3.bien.as_str()), ("2", ""));
        // Mở lại app: ràng buộc đã lưu được áp lại cho đúng mẫu
        let mut m2 = ManMucLuc { bien_luu: (c.mau.clone(), c.bien.clone()), ..man_thu(5) };
        m2.ap_rang_buoc();
        assert_eq!(m2.rb.nguon("nguoi_lap"), Some(&Nguon::Hang("Nguyễn Văn A".into())));
    }
}
