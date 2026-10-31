//! Tab Kết quả bóc (prototype `#boc-tach-ket-qua`, `-dang-chay`, `-loi`, `-loc`, `-gon`, `-xong`): sidebar file (lọc, thu
//! gọn), chi tiết PDF | các trường (sửa tay → Lưu), bảng "Kết quả bóc" ở đáy; chạy hàng loạt ở luồng nền, kết quả ghi
//! `boc_tach.jsonl` sau mỗi file; popup "Đã bóc tách xong".

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Margin, Rect, RichText, Sense, Stroke};

use super::xem_pdf::{XemPdf, NAC};
use super::ManBocTach;
use crate::boc_tach::chay::{chay_hang, SuKien};
use crate::boc_tach::excel::{self, DongExcel};
use crate::boc_tach::sheet::{self, CachGhi, DichSheet, HangDoi};
use crate::boc_tach::loai_tai_lieu::LoaiTaiLieu;
use crate::boc_tach::luu::{self, BanGhi};
use crate::cau_hinh::{CachGhiSheet, CauHinhBocTach};
use crate::gui::{so_nguyen, CHU, CHU_MO, CHU_PHU, DO, NEN_TAB_CHON, NEN_TIEU_DE, VIEN, VIEN_NHAT};
use crate::gui_tach::bieu_tuong::{self as bt, BieuTuong};
use crate::gui_tach::khung_hop as kh;
use crate::muc_luc::khop::chuan_hoa as dang_so;

const CAO_DAU: f32 = 30.0;
const CAO_DONG: f32 = 29.0;
const CAO_DONG_BANG: f32 = 27.0;
const RONG_SO: f32 = 40.0;
const RONG_TRANG: f32 = 50.0;
const RONG_TRUONG: f32 = 58.0;
const RONG_GON: f32 = 44.0;
const RONG_NHAN_FORM: f32 = 104.0;
const XANH_DOC: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xD0);
const NAU: Color32 = Color32::from_rgb(0x8A, 0x4B, 0x08);
const NEN_DAU_BANG: Color32 = Color32::from_rgb(0xF3, 0xF3, 0xF3);

/// Trạng thái 1 file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TrangThai {
    Cho,
    Dang,
    Xong(BanGhi),
    Loi(String),
}

impl TrangThai {
    /// Chữ cột Trường của sidebar: "x/6", "Đọc…", "-", "Lỗi".
    fn nhan(&self, n_truong: usize) -> (String, Color32) {
        match self {
            TrangThai::Cho => ("-".into(), CHU_MO),
            TrangThai::Dang => ("Đọc…".into(), XANH_DOC),
            TrangThai::Xong(b) => (format!("{}/{n_truong}", b.gia_tri.values().filter(|g| !g.is_empty()).count()), CHU),
            TrangThai::Loi(_) => ("Lỗi".into(), DO),
        }
    }
}

pub(crate) struct DangChay {
    nhan: Receiver<SuKien>,
    pub dung: Arc<AtomicBool>,
    bat_dau: Instant,
    /// Danh sách file của lần chạy — giữ cố định (lọc đổi trong lúc chạy không làm mất dòng).
    pub ds: Vec<usize>,
    xong: usize,
    truong_boc: usize,
    bat_dau_file: HashMap<usize, Instant>,
    dung_som: Option<String>,
    mo_hinh: String,
}

/// Số liệu popup "Đã bóc tách xong".
pub(crate) struct TongKet {
    ten_nguon: String,
    tong: usize,
    ok: usize,
    loi: usize,
    trang: usize,
    loai: String,
    truong_co: usize,
    truong_tong: usize,
    thieu: usize,
    mo_hinh: String,
    thoi_gian: Duration,
    so_file_chay: usize,
    dung_som: Option<String>,
    excel: Option<Result<PathBuf, String>>,
    tab_sheet: Option<String>,
}

pub(crate) struct TabKetQua {
    pub i_xem: usize,
    pub loc: String,
    pub gon_trai: bool,
    pub gon_bang: bool,
    pub xem: XemPdf,
    /// (file, giá trị lúc mở, bản đang sửa) — khác nhau → bật Lưu; chuyển file khi chưa Lưu = bỏ.
    sua: Option<(usize, Vec<String>, Vec<String>)>,
    cuon_toi: bool,
    rong_cot: BTreeMap<String, f32>,
    o_so: String,
}

impl Default for TabKetQua {
    fn default() -> Self {
        Self { i_xem: 0, loc: String::new(), gon_trai: false, gon_bang: false, xem: XemPdf::moi(), sua: None, cuon_toi: false, rong_cot: BTreeMap::new(), o_so: String::new() }
    }
}

/// Bộ lọc của ô tìm: tên file (không dấu, không hoa) hoặc trạng thái cột Trường ("lỗi", "đọc", "-", "5/6").
pub(crate) fn khop_loc(ten: &str, tt: &TrangThai, loc: &str, n_truong: usize) -> bool {
    let q = loc.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let theo_tt = match q.as_str() {
        "lỗi" | "loi" => matches!(tt, TrangThai::Loi(_)),
        "đọc" | "doc" | "đọc…" => matches!(tt, TrangThai::Dang),
        "-" => matches!(tt, TrangThai::Cho),
        _ => match q.split_once('/') {
            Some((a, b)) if a.trim().parse::<usize>().is_ok() => tt.nhan(n_truong).0 == format!("{}/{}", a.trim(), b.trim()),
            _ => false,
        },
    };
    theo_tt || (!dang_so(&q).is_empty() && dang_so(ten).contains(&dang_so(&q)))
}

/// Đích Google Sheet theo Cấu hình (khoá service account đọc từ file, URL Apps Script từ kho khoá).
pub(crate) fn tao_dich_sheet(cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>) -> Result<DichSheet, String> {
    let cach = match cfg.sheet_cach {
        CachGhiSheet::ServiceAccount => {
            if cfg.sheet_file_khoa.trim().is_empty() {
                return Err("Chưa chọn file khoá service account (Cấu hình › Bóc tách › Ghi kết quả)".into());
            }
            let (email, khoa_pem) = sheet::doc_khoa_sa(&cfg.sheet_file_khoa)?;
            CachGhi::ServiceAccount { email, khoa_pem }
        }
        CachGhiSheet::AppsScript => CachGhi::AppsScript { url: khoa.get(super::cau_hinh::KHOA_APPS_SCRIPT).cloned().unwrap_or_default() },
    };
    DichSheet::moi(cach, &cfg.sheet_link, &cfg.sheet_tab)
}

fn giay(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f32()).replace('.', ",")
}

impl ManBocTach {
    /// Nạp kết quả đã đọc trước đó (jsonl) cho danh sách file + loại đang chọn.
    pub(crate) fn nap_ket_qua(&mut self) {
        self.tt = vec![TrangThai::Cho; self.files.len()];
        self.kq.sua = None;
        let Some(loai) = self.loai_chon() else { return };
        if self.thu_muc.trim().is_empty() {
            return;
        }
        let ds = luu::doc(&luu::file_jsonl(Path::new(self.thu_muc.trim())), &loai.ten);
        self.excel = None;
        for (i, f) in self.files.iter().enumerate() {
            if let Some(b) = ds.get(&f.ten) {
                self.tt[i] = match &b.loi {
                    Some(e) => TrangThai::Loi(e.clone()),
                    None => TrangThai::Xong(b.clone()),
                };
            }
        }
    }

    /// File đang hiện theo ô lọc (trong lúc chạy: danh sách cố định của lần chạy, vẫn áp lọc tên).
    pub(crate) fn hien(&self) -> Vec<usize> {
        if let Some(c) = self.chay.as_ref().filter(|_| !self.kq.loc.trim().is_empty()) {
            return c.ds.clone();
        }
        let n = self.loai_chon().map_or(0, |l| l.truong.len());
        (0..self.files.len()).filter(|&i| khop_loc(&self.files[i].ten, self.tt.get(i).unwrap_or(&TrangThai::Cho), &self.kq.loc, n)).collect()
    }

    /// Bắt đầu / Chạy lại các file `ds` trên luồng nền.
    pub(crate) fn bat_dau(&mut self, ds: Vec<usize>, cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>, ctx: &egui::Context) {
        let (Some(loai), Some(n)) = (self.loai_chon().cloned(), self.ncc) else { return };
        if ds.is_empty() {
            return;
        }
        let kn = cfg.ket_noi(n, khoa.get(n.ma()).map_or("", String::as_str));
        let song_song = cfg.mo_hinh(n).song_song as usize;
        let anh = cfg.anh();
        let files: Vec<(usize, PathBuf)> = ds.iter().map(|&i| (i, self.files[i].duong_dan.clone())).collect();
        self.tao_sheet(cfg, khoa);
        let dung = Arc::new(AtomicBool::new(false));
        let (gui, nhan) = mpsc::channel();
        let (d2, ctx2) = (dung.clone(), ctx.clone());
        std::thread::spawn(move || {
            chay_hang(&files, &loai, &anh, &kn, song_song, &d2, &gui);
            ctx2.request_repaint();
        });
        self.kq.sua = None;
        self.tong_ket = None;
        self.tab = super::Tab::KetQua;
        self.chay = Some(DangChay { nhan, dung, bat_dau: Instant::now(), ds, xong: 0, truong_boc: 0, bat_dau_file: HashMap::new(), dung_som: None, mo_hinh: super::cau_hinh::ten_mo_hinh(cfg, n) });
    }

    pub(crate) fn nhan_chay(&mut self, ctx: &egui::Context, cfg: &CauHinhBocTach) {
        let Some(loai) = self.loai_chon().cloned() else { return };
        let goc = PathBuf::from(self.thu_muc.trim());
        let f_jsonl = luu::file_jsonl(&goc);
        loop {
            let Some(c) = &mut self.chay else { return };
            let sk = match c.nhan.try_recv() {
                Ok(sk) => sk,
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                    return;
                }
                Err(TryRecvError::Disconnected) => SuKien::HetHang,
            };
            match sk {
                SuKien::BatDau(i) => {
                    c.bat_dau_file.insert(i, Instant::now());
                    self.tt[i] = TrangThai::Dang;
                    // Chi tiết + 2 bảng đi theo file đang bóc
                    self.kq.i_xem = i;
                    self.kq.cuon_toi = true;
                }
                SuKien::Xong(i, kq) => {
                    let b = BanGhi::tu_ket_qua(&self.files[i].ten, &loai, &kq);
                    let _ = luu::ghi_them(&f_jsonl, &b);
                    c.xong += 1;
                    c.truong_boc += kq.so_co();
                    self.tt[i] = TrangThai::Xong(b);
                    self.day_sheet(i, &loai);
                }
                SuKien::Loi(i, e) => {
                    let _ = luu::ghi_them(&f_jsonl, &BanGhi::loi(&self.files[i].ten, &loai, self.files[i].so_trang, &e));
                    c.xong += 1;
                    self.tt[i] = TrangThai::Loi(e);
                    self.day_sheet(i, &loai);
                }
                SuKien::DungSom(l) => c.dung_som = Some(l),
                SuKien::HetHang => {
                    let c = self.chay.take().unwrap();
                    self.thoi_gian_chay = Some(c.bat_dau.elapsed());
                    self.ghi_excel(cfg);
                    self.tong_ket = Some(self.tinh_tong_ket(&c, &loai));
                    return;
                }
            }
        }
    }

    fn day_sheet(&self, i: usize, loai: &LoaiTaiLieu) {
        if let (Some(h), Some(d)) = (&self.sheet, self.dong_excel(i, loai)) {
            h.them(excel::dong_chu(&d, true));
        }
    }

    fn tinh_tong_ket(&self, c: &DangChay, loai: &LoaiTaiLieu) -> TongKet {
        let (mut ok, mut loi, mut co, mut thieu) = (0, 0, 0, 0);
        for t in &self.tt {
            match t {
                TrangThai::Xong(b) => {
                    ok += 1;
                    let n = b.gia_tri.values().filter(|g| !g.is_empty()).count();
                    co += n;
                    thieu += usize::from(n < loai.truong.len());
                }
                TrangThai::Loi(_) => loi += 1,
                _ => {}
            }
        }
        let trang = c.ds.iter().filter(|&&i| matches!(self.tt[i], TrangThai::Xong(_))).map(|&i| loai.trang_doc.cac_trang(self.files[i].so_trang).len()).sum();
        let ten_nguon = Path::new(self.thu_muc.trim()).file_name().map_or_else(|| self.thu_muc.clone(), |s| s.to_string_lossy().into_owned());
        TongKet {
            ten_nguon,
            tong: self.files.len(),
            ok,
            loi,
            trang,
            loai: loai.ten.clone(),
            truong_co: co,
            truong_tong: ok * loai.truong.len(),
            thieu,
            mo_hinh: c.mo_hinh.clone(),
            thoi_gian: c.bat_dau.elapsed(),
            so_file_chay: c.xong,
            dung_som: c.dung_som.clone(),
            excel: self.excel.clone(),
            tab_sheet: self.sheet.as_ref().map(|_| self.tab_sheet.clone()),
        }
    }

    /// Dòng Excel / Sheet của file `i` (đã đọc xong hoặc lỗi).
    fn dong_excel(&self, i: usize, loai: &LoaiTaiLieu) -> Option<DongExcel> {
        let f = &self.files[i];
        let goc = Path::new(self.thu_muc.trim());
        // Hồ sơ = thư mục chứa file (file ở ngay thư mục nguồn → tên thư mục nguồn)
        let ho_so = Path::new(&f.ten).parent().and_then(|p| p.file_name()).or_else(|| goc.file_name()).map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let (gia_tri, loi) = match self.tt.get(i)? {
            TrangThai::Xong(b) => (b.theo_truong(loai), None),
            TrangThai::Loi(e) => (vec![String::new(); loai.truong.len()], Some(e.clone())),
            _ => return None,
        };
        Some(DongExcel { ten_file: f.ten.clone(), ho_so, so_trang: f.so_trang, gia_tri, loi })
    }

    /// Ghi lại toàn bộ Excel từ kết quả hiện có (chạy xong / Lưu sửa tay).
    pub(crate) fn ghi_excel(&mut self, cfg: &CauHinhBocTach) {
        let Some(loai) = self.loai_chon().cloned() else { return };
        let dong: Vec<DongExcel> = (0..self.files.len()).filter_map(|i| self.dong_excel(i, &loai)).collect();
        if dong.is_empty() {
            return;
        }
        let f = excel::duong_dan(Path::new(self.thu_muc.trim()), &cfg.thu_muc_excel);
        self.excel = Some(excel::ghi(&f, &loai, &dong));
    }

    /// Hàng đợi Google Sheet theo Cấu hình (ô Google Sheet đang bật); lỗi cấu hình → ghi vào `loi_sheet`.
    fn tao_sheet(&mut self, cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>) {
        self.sheet = None;
        self.loi_sheet = None;
        let Some(loai) = self.loai_chon().cloned() else { return };
        if !cfg.ghi_sheet {
            return;
        }
        match tao_dich_sheet(cfg, khoa) {
            Ok(d) => {
                self.tab_sheet = if cfg.sheet_tab.trim().is_empty() { excel::TAB.into() } else { cfg.sheet_tab.trim().into() };
                self.sheet = Some(HangDoi::moi(d, excel::tieu_de(&loai, true)));
            }
            Err(e) => self.loi_sheet = Some(e),
        }
    }

    /// Dòng đáy khi đang chạy / sau khi chạy (None = chưa chạy lần nào trong danh sách này).
    pub(crate) fn dong_trang_thai(&self, job: &mut egui::text::LayoutJob) -> bool {
        let (thuong, dam) = (crate::gui_tach::chu_dong(CHU_PHU, false), crate::gui_tach::chu_dong(CHU, true));
        let n = self.loai_chon().map_or(0, |l| l.truong.len());
        if let Some(c) = &self.chay {
            let dang = c.ds.iter().filter(|&&i| self.tt[i] == TrangThai::Dang).count();
            job.append("Đang đọc ", 0.0, thuong.clone());
            job.append(&format!("{}/{}", (c.xong + dang).min(c.ds.len()), c.ds.len()), 0.0, dam.clone());
            job.append(" file · đã bóc ", 0.0, thuong.clone());
            job.append(&so_nguyen(c.truong_boc), 0.0, dam.clone());
            job.append(" trường · thời gian đã bóc ", 0.0, thuong.clone());
            job.append(&crate::gui_tach::gio_phut_giay(c.bat_dau.elapsed()), 0.0, dam.clone());
            if c.xong > 0 {
                let con = c.bat_dau.elapsed().mul_f64((c.ds.len() - c.xong) as f64 / c.xong as f64);
                job.append(&format!(" · dự kiến còn ~{}", crate::gui_tach::gio_phut_giay(con)), 0.0, thuong);
            }
            return true;
        }
        let (ok, loi, co) = self.tt.iter().fold((0, 0, 0), |(o, l, c), t| match t {
            TrangThai::Xong(b) => (o + 1, l, c + b.gia_tri.values().filter(|g| !g.is_empty()).count()),
            TrangThai::Loi(_) => (o, l + 1, c),
            _ => (o, l, c),
        });
        if ok + loi == 0 {
            return false;
        }
        job.append("Đã bóc tách ", 0.0, thuong.clone());
        job.append(&format!("{}/{}", so_nguyen(ok), so_nguyen(self.files.len())), 0.0, dam.clone());
        job.append(&format!(" file{} · ", if loi > 0 { format!(" ({loi} lỗi)") } else { String::new() }), 0.0, thuong.clone());
        job.append(&format!("{}/{}", so_nguyen(co), so_nguyen(ok * n)), 0.0, dam.clone());
        job.append(" trường", 0.0, thuong.clone());
        if let Some(t) = self.thoi_gian_chay {
            job.append(" · thời gian đã bóc ", 0.0, thuong);
            job.append(&crate::gui_tach::gio_phut_giay(t), 0.0, dam);
        }
        true
    }

    /// Cảnh báo đầu khung (tab bar): file lỗi · thiếu trường bắt buộc.
    pub(crate) fn canh_bao(&self) -> Option<String> {
        let loai = self.loai_chon()?;
        let loi: Vec<&str> = self.tt.iter().enumerate().filter(|(_, t)| matches!(t, TrangThai::Loi(_))).map(|(i, _)| self.files[i].ten.as_str()).collect();
        let thieu = self.tt.iter().filter(|t| matches!(t, TrangThai::Xong(b) if loai.truong.iter().any(|tr| tr.bat_buoc && b.gia_tri.get(&tr.bien).is_none_or(|g| g.is_empty())))).count();
        let mut phan = vec![];
        if !loi.is_empty() {
            let ten: Vec<&str> = loi.iter().take(2).map(|t| Path::new(t).file_name().and_then(|s| s.to_str()).unwrap_or(t)).collect();
            phan.push(format!("{} file lỗi ({}{})", loi.len(), ten.join(", "), if loi.len() > 2 { ", …" } else { "" }));
        }
        if thieu > 0 {
            phan.push(format!("{thieu} file thiếu trường bắt buộc"));
        }
        (!phan.is_empty()).then(|| format!("Kiểm tra: {}", phan.join(" · ")))
    }
}

pub(crate) fn ve(m: &mut ManBocTach, ui: &mut egui::Ui, cfg: &CauHinhBocTach) {
    let Some(loai) = m.loai_chon().cloned() else { return };
    let hien = m.hien();
    if m.kq.i_xem >= m.files.len() {
        m.kq.i_xem = 0;
    }
    let cao = if m.kq.gon_bang { CAO_DONG_BANG } else { 150.0 };
    let mut p = egui::Panel::bottom("bt_kq_bang").frame(egui::Frame::new().fill(Color32::WHITE)).resizable(!m.kq.gon_bang).size_range(60.0..=400.0).default_size(cao);
    if m.kq.gon_bang {
        p = p.exact_size(CAO_DONG_BANG + 1.0);
    }
    p.show(ui, |ui| bang_day(m, ui, &loai, &hien, cfg));
    let mut trai = egui::Panel::left("bt_kq_trai").frame(egui::Frame::new().fill(Color32::WHITE));
    trai = if m.kq.gon_trai { trai.exact_size(RONG_GON).resizable(false) } else { trai.resizable(true).default_size(300.0).size_range(200.0..=520.0) };
    trai.show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().vline(r.right() - 0.5, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
        sidebar(m, ui, &loai, &hien);
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| chi_tiet(m, ui, &loai, cfg));
    m.kq.cuon_toi = false;
}

/// Sidebar: # | Tên file | Trang | Trường; vạch tiến độ dưới tên file đang bóc; ô tìm + nút thu gọn ở đáy.
fn sidebar(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, hien: &[usize]) {
    let khung = ui.max_rect();
    let gon = m.kq.gon_trai;
    let p = ui.painter().clone();
    let dau = Rect::from_min_size(khung.min, egui::vec2(khung.width() - 1.0, CAO_DAU));
    p.hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let f = egui::FontId::proportional(12.0);
    let x_trang = dau.right() - RONG_TRUONG - RONG_TRANG;
    p.text(egui::pos2(dau.left() + 12.0, dau.center().y), egui::Align2::LEFT_CENTER, "#", f.clone(), CHU_PHU);
    if !gon {
        p.text(egui::pos2(dau.left() + RONG_SO, dau.center().y), egui::Align2::LEFT_CENTER, "Tên file", f.clone(), CHU_PHU);
        p.text(egui::pos2(x_trang + RONG_TRANG - 10.0, dau.center().y), egui::Align2::RIGHT_CENTER, "Trang", f.clone(), CHU_PHU);
        p.text(egui::pos2(dau.right() - 8.0, dau.center().y), egui::Align2::RIGHT_CENTER, "Trường", f.clone(), CHU_PHU);
    }
    let cao_tim = 34.0;
    let than = Rect::from_min_max(egui::pos2(khung.left(), dau.bottom()), egui::pos2(khung.right() - 1.0, khung.bottom() - cao_tim));
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(than));
    let mut sa = egui::ScrollArea::vertical().id_salt("bt_kq_ds").auto_shrink([false, false]);
    if m.kq.cuon_toi {
        if let Some(k) = hien.iter().position(|&i| i == m.kq.i_xem) {
            sa = sa.vertical_scroll_offset((k as f32 * CAO_DONG - than.height() / 2.0 + CAO_DONG / 2.0).max(0.0));
        }
    }
    let n = loai.truong.len();
    let mut bam = None;
    sa.show_rows(&mut con, CAO_DONG, hien.len(), |ui, pham_vi| {
        for k in pham_vi {
            let i = hien[k];
            let (r, ph) = ui.allocate_exact_size(egui::vec2(than.width(), CAO_DONG), Sense::click());
            let p = ui.painter();
            if i == m.kq.i_xem {
                p.rect_filled(r, 0.0, NEN_TAB_CHON);
            } else if ph.hovered() {
                p.rect_filled(r, 0.0, Color32::from_gray(0xF7));
            }
            p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(0xEC)));
            let f13 = egui::FontId::proportional(12.5);
            p.text(egui::pos2(r.left() + 12.0, r.center().y), egui::Align2::LEFT_CENTER, (i + 1).to_string(), f13.clone(), CHU_PHU);
            let fp = &m.files[i];
            let tt = m.tt.get(i).cloned().unwrap_or(TrangThai::Cho);
            if !gon {
                let rong_ten = x_trang - r.left() - RONG_SO - 6.0;
                let mut job = egui::text::LayoutJob::single_section(fp.ten.clone(), egui::TextFormat { font_id: f13.clone(), color: CHU, ..Default::default() });
                job.wrap = egui::text::TextWrapping::truncate_at_width(rong_ten);
                let g = ui.fonts_mut(|fo| fo.layout_job(job));
                let gw = g.size().x;
                p.galley(egui::pos2(r.left() + RONG_SO, r.center().y - g.size().y / 2.0), g, CHU);
                p.text(egui::pos2(x_trang + RONG_TRANG - 10.0, r.center().y), egui::Align2::RIGHT_CENTER, fp.so_trang.to_string(), f13.clone(), CHU);
                let (chu, mau) = tt.nhan(n);
                p.text(egui::pos2(r.right() - 8.0, r.center().y), egui::Align2::RIGHT_CENTER, chu, f13.clone(), mau);
                // Vạch tiến độ mảnh dưới tên file đang bóc (ước theo thời gian đọc trung bình)
                if tt == TrangThai::Dang {
                    if let Some(t0) = m.chay.as_ref().and_then(|c| c.bat_dau_file.get(&i)) {
                        let uoc = m.truong.giay_moi_trang().unwrap_or(3.0) * loai.trang_doc.cac_trang(fp.so_trang).len().max(1) as f32;
                        let tl = (t0.elapsed().as_secs_f32() / uoc).min(0.95);
                        let y = r.bottom() - 4.0;
                        let x0 = r.left() + RONG_SO;
                        p.hline(x0..=x0 + rong_ten.min(gw.max(60.0)) * tl, y, Stroke::new(1.5, XANH_DOC));
                    }
                }
            }
            if let TrangThai::Loi(e) = &tt {
                ph.clone().on_hover_text(e);
            }
            ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, i == m.kq.i_xem, &fp.ten));
            if ph.clicked() {
                bam = Some(i);
            }
        }
    });
    if let Some(i) = bam {
        m.kq.i_xem = i;
    }
    // Ô tìm + nút thu gọn sidebar
    let o = Rect::from_min_max(egui::pos2(khung.left() + 6.0, khung.bottom() - cao_tim + 5.0), egui::pos2(khung.right() - 7.0, khung.bottom() - 5.0));
    ui.painter().hline(khung.x_range(), khung.bottom() - cao_tim + 0.5, Stroke::new(1.0, VIEN_NHAT));
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o).layout(egui::Layout::right_to_left(egui::Align::Center)));
    con.spacing_mut().item_spacing.x = 4.0;
    let (bt_ico, ten) = if gon { (BieuTuong::MoRong, "Mở rộng danh sách file") } else { (BieuTuong::ThuGon, "Thu gọn danh sách file") };
    if bt::nut(&mut con, bt_ico, ten, true).clicked() {
        m.kq.gon_trai = !gon;
    }
    if !gon {
        con.add(kh::o_nhap(&mut m.kq.loc).desired_width(con.available_width()).hint_text("Tìm file, lỗi, 5/6…")).on_hover_text("Lọc theo tên file hoặc cột Trường: lỗi, đọc, -, 5/6");
    }
    // Giữ bề rộng panel (kéo được), không co theo nội dung
    ui.allocate_rect(khung, Sense::hover());
}

/// Chi tiết: thanh "‹ n / N › tên file · Đã đọc: 2,4 s" · thu phóng · Lưu; PDF | các trường.
fn chi_tiet(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, cfg: &CauHinhBocTach) {
    let i = m.kq.i_xem;
    let n_file = m.files.len();
    let Some(fp) = m.files.get(i).cloned() else { return };
    let tt = m.tt.get(i).cloned().unwrap_or(TrangThai::Cho);
    // Giá trị hiện tại + bản sửa
    let goc: Vec<String> = match &tt {
        TrangThai::Xong(b) => b.theo_truong(loai),
        _ => vec![String::new(); loai.truong.len()],
    };
    let can_moi = match &m.kq.sua {
        Some((j, g, b)) => *j != i || (*g != goc && b == g),
        None => true,
    };
    if can_moi {
        m.kq.sua = Some((i, goc.clone(), goc.clone()));
        m.kq.o_so = (i + 1).to_string();
    }
    let doi = m.kq.sua.as_ref().is_some_and(|(_, g, b)| g != b);
    let thanh = Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.max_rect().width(), CAO_DAU));
    ui.painter().hline(thanh.x_range(), thanh.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(thanh.shrink2(egui::vec2(8.0, 3.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    con.spacing_mut().item_spacing.x = 4.0;
    let mut toi = None;
    if bt::nut(&mut con, BieuTuong::TraiChevron, "File trước", i > 0).clicked() {
        toi = Some(i - 1);
    }
    let o = con.add(kh::o_so(&mut m.kq.o_so).desired_width(44.0));
    if o.lost_focus() {
        match m.kq.o_so.trim().parse::<usize>() {
            Ok(k) if (1..=n_file).contains(&k) => toi = Some(k - 1),
            _ => m.kq.o_so = (i + 1).to_string(),
        }
    }
    o.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Số thứ tự file"));
    con.label(RichText::new(format!("/ {}", so_nguyen(n_file))).color(CHU_PHU));
    if bt::nut(&mut con, BieuTuong::PhaiChevron, "File sau", i + 1 < n_file).clicked() {
        toi = Some(i + 1);
    }
    con.add_space(8.0);
    let (tt_chu, tt_mau) = match &tt {
        TrangThai::Cho => ("Chưa đọc".to_string(), CHU_PHU),
        TrangThai::Dang => ("Đang đọc…".to_string(), CHU_PHU),
        TrangThai::Xong(b) => (format!("Đã đọc: {} s", giay(Duration::from_secs_f32(b.giay))), CHU_PHU),
        TrangThai::Loi(e) => (format!("Lỗi: {e}"), DO),
    };
    let rong_chu = (con.available_width() - 210.0).max(60.0);
    let mut job = egui::text::LayoutJob::default();
    job.append(&fp.ten, 0.0, crate::gui_tach::chu_dong(CHU, false));
    job.append(" · ", 0.0, crate::gui_tach::chu_dong(CHU_PHU, false));
    job.append(&tt_chu, 0.0, crate::gui_tach::chu_dong(tt_mau, false));
    job.wrap = egui::text::TextWrapping::truncate_at_width(rong_chu);
    con.add(egui::Label::new(job).sense(Sense::hover())).on_hover_text(format!("{} · {tt_chu}", fp.ten));
    let mut luu = false;
    con.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        luu = ui.add_enabled(doi, egui::Button::new(RichText::new("Lưu").size(12.0)).min_size(egui::vec2(76.0, 22.0))).on_hover_text("Lưu giá trị sửa tay (ghi lại kết quả của file này)").on_disabled_hover_text("Sửa ô bên dưới để bật Lưu").clicked();
        ui.add_space(8.0);
        let x = &mut m.kq.xem;
        if bt::nut(ui, BieuTuong::KinhCong, "Phóng to", x.nac + 1 < NAC.len()).clicked() {
            x.nac += 1;
        }
        ui.add_sized(egui::vec2(44.0, 22.0), egui::Label::new(RichText::new(format!("{}%", x.ty_le())).size(12.0)));
        if bt::nut(ui, BieuTuong::KinhTru, "Thu nhỏ", x.nac > 0).clicked() {
            x.nac -= 1;
        }
    });
    ui.allocate_rect(thanh, Sense::hover());
    if luu {
        luu_sua(m, loai, i, &fp.ten, fp.so_trang);
        // Lưu ghi lại Excel + dòng của file đó trên Sheet
        m.ghi_excel(cfg);
        if let (Some(h), Some(d)) = (&m.sheet, m.dong_excel(i, loai)) {
            h.ghi_de(d.ten_file.clone(), excel::dong_chu(&d, true));
        }
    }
    if let Some(k) = toi {
        m.kq.i_xem = k;
        m.kq.cuon_toi = true;
    }
    let vung = Rect::from_min_max(egui::pos2(thanh.left(), thanh.bottom()), ui.max_rect().max);
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(vung));
    let rong_form = (vung.width() * 0.45).round();
    egui::Panel::right("bt_kq_form").resizable(true).default_size(rong_form).size_range(220.0..=vung.width() * 0.75).frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(Margin { left: 14, right: 14, top: 12, bottom: 8 })).show(&mut con, |ui| {
        let r = ui.max_rect().expand2(egui::vec2(14.0, 12.0));
        ui.painter().vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
        form(m, ui, loai, &tt);
    });
    egui::CentralPanel::default().frame(egui::Frame::new()).show(&mut con, |ui| {
        m.kq.xem.mo(Some(fp.duong_dan.as_path()));
        m.kq.xem.ve(ui);
    });
}

/// Các trường: nhãn | ô (Trích yếu / Nội dung nhiều dòng); đang bóc → vòng xoay + "Đang bóc".
fn form(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, tt: &TrangThai) {
    let Some((_, _, ban)) = &mut m.kq.sua else { return };
    egui::ScrollArea::vertical().id_salt("bt_kq_form_cuon").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        for (k, tr) in loai.truong.iter().enumerate() {
            let nhieu_dong = matches!(tr.bien.as_str(), "trich_yeu" | "noi_dung");
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(RONG_NHAN_FORM, 26.0), Sense::hover());
                ui.painter().text(egui::pos2(r.left(), r.center().y), egui::Align2::LEFT_CENTER, &tr.ten_cot, egui::FontId::proportional(12.5), CHU);
                let w = ui.available_width();
                if *tt == TrangThai::Dang {
                    let cao = if nhieu_dong { 56.0 } else { 26.0 };
                    let (o, _) = ui.allocate_exact_size(egui::vec2(w, cao), Sense::hover());
                    ui.painter().rect(o, 3.0, Color32::WHITE, Stroke::new(1.0, VIEN), egui::StrokeKind::Inside);
                    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(o.min + egui::vec2(8.0, 5.0), egui::vec2(w - 16.0, 16.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    c.spacing_mut().item_spacing.x = 6.0;
                    c.add(egui::Spinner::new().size(12.0).color(CHU_MO));
                    c.label(RichText::new("Đang bóc").size(12.0).color(CHU_MO));
                } else if nhieu_dong {
                    ui.add(egui::TextEdit::multiline(&mut ban[k]).desired_rows(3).desired_width(w).font(egui::FontId::proportional(12.5)).hint_text(&tr.ten_cot));
                } else {
                    ui.add(kh::o_nhap(&mut ban[k]).desired_width(w).hint_text(&tr.ten_cot));
                }
            });
        }
    });
}

/// Lưu bản sửa tay của file `i`: ghi jsonl (biến đã sửa cộng dồn), cập nhật bảng.
fn luu_sua(m: &mut ManBocTach, loai: &LoaiTaiLieu, i: usize, ten: &str, so_trang: usize) {
    let Some((_, goc, ban)) = m.kq.sua.clone() else { return };
    let mut b = match m.tt.get(i) {
        Some(TrangThai::Xong(b)) => b.clone(),
        _ => BanGhi { file: ten.into(), loai: loai.ten.clone(), so_trang, gia_tri: BTreeMap::new(), loi: None, sua: vec![], giay: 0.0, tho: vec![] },
    };
    for (k, tr) in loai.truong.iter().enumerate() {
        let moi = ban[k].trim().to_string();
        if moi != goc[k] && !b.sua.contains(&tr.bien) {
            b.sua.push(tr.bien.clone());
        }
        b.gia_tri.insert(tr.bien.clone(), moi);
    }
    b.loi = None;
    let _ = luu::ghi_them(&luu::file_jsonl(Path::new(m.thu_muc.trim())), &b);
    m.tt[i] = TrangThai::Xong(b);
    m.kq.sua = None;
}

/// Bảng "Kết quả bóc" ở đáy: # | Tên file | Số trang | các trường; tiêu đề "Mở Excel · Mở GSheet"; thu / mở.
fn bang_day(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, hien: &[usize], cfg: &CauHinhBocTach) {
    let khung = ui.max_rect();
    let dau = Rect::from_min_size(khung.min, egui::vec2(khung.width(), CAO_DONG_BANG));
    let ph_dau = ui.interact(Rect::from_min_max(dau.min, egui::pos2(dau.right() - 260.0, dau.bottom())), ui.scope_id().with("bt_kq_dau"), Sense::click());
    let p = ui.painter().clone();
    p.rect_filled(dau, 0.0, NEN_TIEU_DE);
    p.hline(dau.x_range(), dau.top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
    p.hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let gd = p.layout_no_wrap("Kết quả bóc".into(), egui::FontId::new(12.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU);
    let mut x = dau.left() + 12.0;
    p.galley(egui::pos2(x, dau.center().y - gd.size().y / 2.0), gd.clone(), CHU);
    x += gd.size().x + 10.0;
    let (ok, loi) = m.tt.iter().fold((0, 0), |(o, l), t| match t {
        TrangThai::Xong(_) => (o + 1, l),
        TrangThai::Loi(_) => (o, l + 1),
        _ => (o, l),
    });
    let tt = match &m.chay {
        Some(c) => format!("Đang bóc {}/{}", c.xong.min(c.ds.len()), c.ds.len()),
        None if ok + loi > 0 => format!("{}/{} file{}", ok, m.files.len(), if loi > 0 { format!(" · {loi} lỗi") } else { String::new() }),
        None => "Chưa bóc — bấm Bắt đầu".into(),
    };
    let g = p.layout_no_wrap(tt, egui::FontId::proportional(12.0), CHU_PHU);
    p.galley(egui::pos2(x, dau.center().y - g.size().y / 2.0), g.clone(), CHU_PHU);
    x += g.size().x + 8.0;
    if !m.kq.loc.trim().is_empty() {
        p.text(egui::pos2(x, dau.center().y), egui::Align2::LEFT_CENTER, format!("Lọc \"{}\": {}/{} file", m.kq.loc.trim(), hien.len(), m.files.len()), egui::FontId::proportional(12.0), XANH_DOC);
    }
    // Phải: Mở Excel (xám khi chưa có) · Mở GSheet · thu / mở
    let o_dong = Rect::from_center_size(egui::pos2(dau.right() - 18.0, dau.center().y), egui::vec2(22.0, 22.0));
    let ph_dong = ui.interact(o_dong, ui.scope_id().with("bt_kq_gon"), Sense::click());
    bt::ve(&p, o_dong.shrink(4.0), if m.kq.gon_bang { BieuTuong::HienDong } else { BieuTuong::AnDong }, CHU);
    ph_dong.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Thu / mở bảng Kết quả bóc"));
    if ph_dong.clicked() || ph_dau.clicked() {
        m.kq.gon_bang = !m.kq.gon_bang;
    }
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(egui::pos2(dau.right() - 300.0, dau.top()), egui::pos2(o_dong.left() - 8.0, dau.bottom()))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    con.spacing_mut().item_spacing.x = 4.0;
    let co_sheet = !cfg.sheet_link.trim().is_empty();
    let tt_sheet = m.sheet.as_ref().map(HangDoi::trang_thai);
    let goi_sheet = match (&tt_sheet, &m.loi_sheet) {
        (_, Some(e)) => format!("Không ghi được Google Sheet: {e}"),
        (Some(t), _) if t.loi.is_some() => format!("Đã ghi {} dòng, chờ gửi lại {} dòng: {}", t.da_gui, t.cho, t.loi.as_deref().unwrap_or("")),
        (Some(t), _) => format!("Đã ghi {} dòng lên Google Sheet{}", t.da_gui, if t.cho > 0 { format!(", đang gửi {}", t.cho) } else { String::new() }),
        _ => "Mở Google Sheet trên trình duyệt".into(),
    };
    let mau_sheet = if m.loi_sheet.is_some() || tt_sheet.as_ref().is_some_and(|t| t.loi.is_some()) { DO } else { CHU };
    if lien_ket(&mut con, "Mở GSheet", co_sheet, mau_sheet).on_hover_text(&goi_sheet).on_disabled_hover_text("Chưa khai link Google Sheet ở Cấu hình › Bóc tách").clicked() {
        con.ctx().open_url(egui::OpenUrl::new_tab(cfg.sheet_link.trim()));
    }
    con.label(RichText::new("·").size(12.0).color(CHU_PHU));
    match &m.excel {
        Some(Ok(p)) => {
            let ten = p.file_name().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            if lien_ket(&mut con, &format!("Mở Excel: {ten}"), p.exists(), CHU).on_hover_text(p.display().to_string()).clicked() {
                crate::gui::mo_bang_he_dieu_hanh(p);
            }
        }
        Some(Err(e)) => {
            lien_ket(&mut con, "Mở Excel", false, DO).on_disabled_hover_text(format!("Không ghi được Excel: {e}"));
        }
        None => {
            lien_ket(&mut con, "Mở Excel", false, CHU).on_disabled_hover_text("File Excel tạo khi bóc xong");
        }
    }
    if m.kq.gon_bang {
        ui.allocate_rect(khung, Sense::hover());
        return;
    }
    // Bảng: tiêu đề cố định, cuộn ngang chung, cuộn dọc riêng phần dòng
    let cot: Vec<(String, String)> = [("stt".to_string(), "#".to_string()), ("ten_file".into(), "Tên file".into()), ("so_trang".into(), "Số trang".into())].into_iter().chain(loai.truong.iter().map(|t| (t.bien.clone(), t.ten_cot.clone()))).collect();
    let rong: Vec<f32> = cot.iter().map(|(b, _)| *m.kq.rong_cot.get(b).unwrap_or(&super::truong::rong_mac_dinh_kq(b))).collect();
    let tong: f32 = rong.iter().sum();
    let vung = Rect::from_min_max(egui::pos2(khung.left(), dau.bottom()), khung.max);
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(vung));
    let mut bam = None;
    egui::ScrollArea::horizontal().id_salt("bt_kq_ngang").auto_shrink([false, false]).show(&mut con, |ui| {
        let rong_bang = tong.max(vung.width());
        let (r_dau, _) = ui.allocate_exact_size(egui::vec2(rong_bang, CAO_DONG_BANG), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(r_dau, 0.0, NEN_DAU_BANG);
        p.hline(r_dau.x_range(), r_dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        let mut xx = r_dau.left();
        for (k, ((b, ten), w)) in cot.iter().zip(&rong).enumerate() {
            p.text(egui::pos2(xx + 12.0, r_dau.center().y), egui::Align2::LEFT_CENTER, ten, egui::FontId::proportional(12.0), CHU_PHU);
            let mep = Rect::from_center_size(egui::pos2(xx + w, r_dau.center().y), egui::vec2(8.0, CAO_DONG_BANG));
            let keo = ui.interact(mep, ui.scope_id().with(("bt_kq_keo", k)), Sense::drag());
            if keo.hovered() || keo.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }
            if keo.drag_delta().x != 0.0 {
                m.kq.rong_cot.insert(b.clone(), (w + keo.drag_delta().x).clamp(36.0, 600.0));
            }
            xx += w;
        }
        let cao_than = (vung.bottom() - r_dau.bottom()).max(0.0);
        let mut sa = egui::ScrollArea::vertical().id_salt("bt_kq_doc").max_height(cao_than).auto_shrink([false, false]);
        if m.kq.cuon_toi {
            if let Some(k) = hien.iter().position(|&i| i == m.kq.i_xem) {
                sa = sa.vertical_scroll_offset((k as f32 * CAO_DONG_BANG - cao_than / 2.0 + CAO_DONG_BANG / 2.0).max(0.0));
            }
        }
        sa.show_rows(ui, CAO_DONG_BANG, hien.len(), |ui, pham_vi| {
            for k in pham_vi {
                let i = hien[k];
                let (r, ph) = ui.allocate_exact_size(egui::vec2(rong_bang, CAO_DONG_BANG), Sense::click());
                let p = ui.painter();
                if i == m.kq.i_xem {
                    p.rect_filled(r, 0.0, NEN_TAB_CHON);
                }
                p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(0xEC)));
                let fp = &m.files[i];
                let tt = m.tt.get(i).cloned().unwrap_or(TrangThai::Cho);
                let mut xx = r.left();
                for (c, ((bien, _), w)) in cot.iter().zip(&rong).enumerate() {
                    let (chu, mau, nghieng) = match c {
                        0 => ((i + 1).to_string(), CHU_PHU, false),
                        1 => (fp.ten.clone(), CHU, false),
                        2 => (fp.so_trang.to_string(), CHU, false),
                        _ => match &tt {
                            TrangThai::Cho => ("-".into(), CHU_MO, false),
                            TrangThai::Dang if c == 3 => ("Đang bóc…".into(), CHU_MO, true),
                            TrangThai::Dang => (String::new(), CHU_MO, false),
                            TrangThai::Loi(e) if c == 3 => (format!("Lỗi: {e}"), DO, false),
                            TrangThai::Loi(_) => (String::new(), DO, false),
                            TrangThai::Xong(b) => match b.gia_tri.get(bien).filter(|g| !g.is_empty()) {
                                Some(g) => (g.clone(), if b.sua.contains(bien) { NAU } else { CHU }, false),
                                None => ("Trống".into(), CHU_MO, true),
                            },
                        },
                    };
                    // Lỗi tràn sang các cột trường sau
                    let rong_o = if c == 3 && matches!(tt, TrangThai::Loi(_)) { rong.iter().skip(3).sum::<f32>() } else { *w };
                    let mut f = egui::TextFormat { font_id: egui::FontId::proportional(12.5), color: mau, ..Default::default() };
                    f.italics = nghieng;
                    let mut job = egui::text::LayoutJob::single_section(chu, f);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(rong_o - 20.0);
                    let g = ui.fonts_mut(|fo| fo.layout_job(job));
                    ui.painter().galley(egui::pos2(xx + 12.0, r.center().y - g.size().y / 2.0), g, mau);
                    xx += w;
                }
                ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, i == m.kq.i_xem, format!("Dòng {}", fp.ten)));
                if ph.clicked() {
                    bam = Some(i);
                }
            }
        });
    });
    if let Some(i) = bam {
        m.kq.i_xem = i;
    }
    // Giữ chiều cao panel (kéo mép trên đổi được), không co theo nội dung
    ui.allocate_rect(khung, Sense::hover());
}

/// Link chữ ở tiêu đề bảng: rê → gạch chân, con trỏ tay; tắt → chữ mờ.
fn lien_ket(ui: &mut egui::Ui, chu: &str, bat: bool, mau: Color32) -> egui::Response {
    let ph = ui.add_enabled(bat, egui::Label::new(RichText::new(chu).size(12.0).color(if bat { mau } else { CHU_MO })).sense(Sense::click()));
    if bat && ph.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().hline(ph.rect.x_range(), ph.rect.bottom() - 1.0, Stroke::new(1.0, mau));
    }
    ph
}

/// Popup "Đã bóc tách xong": trả `Some(true)` = Xem file lỗi, `Some(false)` = Đóng.
pub(crate) fn ve_tong_ket(ctx: &egui::Context, t: &TongKet, link_sheet: &str) -> Option<bool> {
    let mut kq = None;
    let tieu_de = {
        let mut job = egui::text::LayoutJob::default();
        job.append("Đã bóc tách xong ", 0.0, crate::gui_tach::chu_dong(CHU, false));
        job.append(&t.ten_nguon, 0.0, crate::gui_tach::chu_dong(CHU, true));
        job
    };
    let ph = kh::hop(ctx, "bt_tong_ket", 440.0, tieu_de, |ui| {
        kh::than(ui, "bt_than_tong_ket", 230.0, Margin::same(14), |ui| {
            let dong = |ui: &mut egui::Ui, nhan: &str, f: &dyn Fn(&mut egui::text::LayoutJob)| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(100.0, 18.0), Sense::hover());
                    ui.painter().text(r.left_center(), egui::Align2::LEFT_CENTER, nhan, egui::FontId::proportional(12.0), CHU_PHU);
                    let mut job = egui::text::LayoutJob::default();
                    f(&mut job);
                    // Đường dẫn dài xuống dòng, không kéo rộng popup
                    job.wrap.max_width = 440.0 - 28.0 - 100.0 - 12.0;
                    job.wrap.break_anywhere = true;
                    // Dựng sẵn galley: Label nhận LayoutJob sẽ ghi đè chế độ ngắt dòng của hàng ngang
                    let g = ui.fonts_mut(|f| f.layout_job(job));
                    ui.label(g);
                });
            };
            let dam = crate::gui_tach::chu_dong(CHU, true);
            dong(ui, "Văn bản", &|j| {
                j.append(&format!("{} / {} file", so_nguyen(t.ok), so_nguyen(t.tong)), 0.0, dam.clone());
                if t.loi > 0 {
                    j.append(&format!(" · {} lỗi", t.loi), 0.0, crate::gui_tach::chu_dong(DO, true));
                }
            });
            dong(ui, "Trang đã đọc", &|j| j.append(&format!("{} trang ({})", so_nguyen(t.trang), t.loai), 0.0, dam.clone()));
            dong(ui, "Trường", &|j| j.append(&format!("{} / {}{}", so_nguyen(t.truong_co), so_nguyen(t.truong_tong), if t.thieu > 0 { format!(" · {} file thiếu trường", t.thieu) } else { String::new() }), 0.0, dam.clone()));
            dong(ui, "Mô hình đọc", &|j| j.append(&t.mo_hinh, 0.0, dam.clone()));
            let tb = if t.so_file_chay > 0 { format!(" · TB {} s / file", giay(t.thoi_gian / t.so_file_chay as u32)) } else { String::new() };
            dong(ui, "Thời gian", &|j| j.append(&format!("{}{tb}", crate::gui_tach::gio_phut_giay(t.thoi_gian)), 0.0, dam.clone()));
            let excel = match &t.excel {
                Some(Ok(p)) => p.display().to_string(),
                Some(Err(e)) => format!("Excel lỗi: {e}"),
                None => "—".into(),
            };
            let sheet = t.tab_sheet.as_ref().map(|tab| format!("\nGoogle Sheet › tab {tab}"));
            dong(ui, "Ghi ra", &|j| j.append(&format!("{excel}{}", sheet.clone().unwrap_or_default()), 0.0, crate::gui_tach::chu_dong(CHU, false)));
            if let Some(l) = &t.dung_som {
                dong(ui, "Dừng sớm", &|j| j.append(l, 0.0, crate::gui_tach::chu_dong(DO, false)));
            }
            ui.add_space(6.0);
            ui.label(RichText::new("Thông tin bóc tách đã được ghi lại").size(11.0).color(CHU_PHU));
        });
        kh::chan(ui, |ui| {
            if t.loi > 0 && ui.add(kh::nut_chan("Xem file lỗi")).on_hover_text("Lọc danh sách: chỉ file lỗi").clicked() {
                kq = Some(true);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(kh::nut_chan("Đóng")).clicked() {
                    kq = Some(false);
                }
                let co_sheet = !link_sheet.trim().is_empty();
                if ui.add_enabled(co_sheet, kh::nut_chan("Mở GSheet")).on_disabled_hover_text("Chưa khai link Google Sheet").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(link_sheet.trim()));
                }
                let p_excel = t.excel.as_ref().and_then(|k| k.as_ref().ok()).filter(|p| p.exists());
                if ui.add_enabled(p_excel.is_some(), kh::nut_chan("Mở Excel")).on_disabled_hover_text("Không có file Excel").clicked() {
                    if let Some(p) = p_excel {
                        crate::gui::mo_bang_he_dieu_hanh(p);
                    }
                }
            });
        });
    });
    kq.or(ph.should_close().then_some(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xong(n: usize) -> TrangThai {
        let gia_tri = (0..6).map(|k| (format!("b{k}"), if k < n { "x".to_string() } else { String::new() })).collect();
        TrangThai::Xong(BanGhi { file: String::new(), loai: String::new(), so_trang: 1, gia_tri, loi: None, sua: vec![], giay: 1.0, tho: vec![] })
    }

    #[test]
    fn loc_theo_ten_va_trang_thai() {
        assert!(khop_loc("HS01_VB05.pdf", &xong(5), "5/6", 6));
        assert!(!khop_loc("HS01_VB06.pdf", &xong(6), "5/6", 6));
        assert!(khop_loc("HS02_VB04.pdf", &TrangThai::Loi("x".into()), "lỗi", 6));
        assert!(khop_loc("a.pdf", &TrangThai::Dang, "Đọc", 6));
        assert!(khop_loc("a.pdf", &TrangThai::Cho, "-", 6));
        assert!(khop_loc("HS-01/Tờ trình.pdf", &TrangThai::Cho, "to trinh", 6), "tên không dấu");
        assert!(!khop_loc("HS-01/a.pdf", &TrangThai::Cho, "vb09", 6));
        assert!(khop_loc("x", &TrangThai::Cho, "  ", 6));
    }
}
