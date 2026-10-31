//! Tab Trường dữ liệu (prototype `#boc-tach-truong-b`, `-b-thu`): bảng trường chỉ xem (2/3) | PDF (1/3) có thanh
//! ‹ file › · Thử đọc · thu phóng; dải "Kết quả đọc thử" ngang toàn bề rộng ở đáy. Thử đọc không trừ lượt chạy.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke};

use super::xem_pdf::{XemPdf, NAC};
use super::{hop_loai, FilePdf, Hop, ManBocTach};
use crate::boc_tach::chay::{doc_file, KetQua};
use crate::boc_tach::loai_tai_lieu::{ChuanHoa, LoaiTaiLieu};
use crate::cau_hinh::CauHinhBocTach;
use crate::gui::{CHU, CHU_MO, CHU_PHU, DO, NEN_TAB_CHON, NEN_TIEU_DE, NUT_HOVER, VIEN_NHAT, VIEN_TAB_CHON, XANH_LA};
use crate::gui_tach::bieu_tuong::{self as bt, BieuTuong};

const CAO_DAU: f32 = 30.0;
const RONG_SO: f32 = 34.0;
const CAO_DONG_THU: f32 = 28.0;
const NEN_CHIP: Color32 = Color32::from_rgb(0xEF, 0xEF, 0xEF);
const NEN_DAU_BANG: Color32 = Color32::from_rgb(0xF3, 0xF3, 0xF3);

/// Bề rộng mặc định cột của dải Kết quả đọc thử (như prototype; Trích yếu rộng nhất).
fn rong_mac_dinh(bien: &str) -> f32 {
    match bien {
        "ten_file" => 140.0,
        "so_trang" => 82.0,
        "loai_van_ban" => 110.0,
        "so_ky_hieu" => 130.0,
        "ngay_thang" | "ngay_ky" => 100.0,
        "tac_gia" => 190.0,
        "trich_yeu" | "noi_dung" => 340.0,
        _ => 140.0,
    }
}

/// Bề rộng mặc định cột bảng Kết quả bóc (thêm cột #).
pub(crate) fn rong_mac_dinh_kq(bien: &str) -> f32 {
    if bien == "stt" { 44.0 } else { rong_mac_dinh(bien) }
}

struct DangThu {
    i_file: usize,
    nhan: Receiver<Result<KetQua, String>>,
    bat_dau: Instant,
}

pub(crate) struct TabTruong {
    /// Trường đang chọn (dòng bảng / cột dải đọc thử).
    pub chon: usize,
    pub i_file: usize,
    rong_ten: f32,
    rong_chuan: f32,
    pub xem: XemPdf,
    thu: Option<DangThu>,
    kq_thu: Option<(usize, Result<KetQua, String>)>,
    pub gon_thu: bool,
    rong_cot_thu: BTreeMap<String, f32>,
}

impl Default for TabTruong {
    fn default() -> Self {
        Self { chon: 0, i_file: 0, rong_ten: 190.0, rong_chuan: 170.0, xem: XemPdf::moi(), thu: None, kq_thu: None, gon_thu: false, rong_cot_thu: BTreeMap::new() }
    }
}

impl TabTruong {
    /// Đổi thư mục / danh sách file: về file đầu, bỏ kết quả thử.
    pub fn dat_lai_file(&mut self) {
        self.i_file = 0;
        self.bo_ket_qua_thu();
    }

    /// Đổi loại tài liệu / mô hình: kết quả thử cũ không còn đúng.
    pub fn bo_ket_qua_thu(&mut self) {
        self.thu = None;
        self.kq_thu = None;
    }

    pub fn file_xem<'a>(&self, files: &'a [FilePdf]) -> Option<&'a FilePdf> {
        files.get(self.i_file.min(files.len().saturating_sub(1)))
    }

    /// Giây đọc 1 trang theo lần Thử đọc gần nhất (ước tính ở dòng đáy).
    pub fn giay_moi_trang(&self) -> Option<f32> {
        match &self.kq_thu {
            Some((_, Ok(kq))) if !kq.tho.is_empty() => Some(kq.thoi_gian.as_secs_f32() / kq.tho.len() as f32),
            _ => None,
        }
    }

    fn nhan_thu(&mut self, ctx: &egui::Context) {
        let Some(t) = &self.thu else { return };
        match t.nhan.try_recv() {
            Ok(kq) => {
                self.kq_thu = Some((t.i_file, kq));
                self.thu = None;
            }
            Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            Err(TryRecvError::Disconnected) => self.thu = None,
        }
    }

    fn thu_doc(&mut self, ctx: &egui::Context, f: &FilePdf, loai: &LoaiTaiLieu, cfg: &CauHinhBocTach, kn: crate::boc_tach::mo_hinh::KetNoi) {
        let (gui, nhan) = mpsc::channel();
        let (pdf, loai, anh, ctx2) = (f.duong_dan.clone(), loai.clone(), cfg.anh(), ctx.clone());
        std::thread::spawn(move || {
            let kq = doc_file(&pdf, &loai, &anh, &kn, &AtomicBool::new(false)).map_err(|e| e.to_string());
            let _ = gui.send(kq);
            ctx2.request_repaint();
        });
        self.kq_thu = None;
        self.thu = Some(DangThu { i_file: self.i_file, nhan, bat_dau: Instant::now() });
    }
}

pub(crate) fn ve(m: &mut ManBocTach, ui: &mut egui::Ui, cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>) {
    m.truong.nhan_thu(ui.ctx());
    let Some(loai) = m.loai_chon().cloned() else { return };
    m.truong.chon = m.truong.chon.min(loai.truong.len().saturating_sub(1));
    m.truong.i_file = m.truong.i_file.min(m.files.len().saturating_sub(1));
    let file = m.truong.file_xem(&m.files).cloned();
    let cao_thu = if m.truong.gon_thu { CAO_DONG_THU } else { 150.0 };
    let mut p_thu = egui::Panel::bottom("bt_thu").frame(egui::Frame::new().fill(Color32::WHITE)).resizable(!m.truong.gon_thu).size_range(60.0..=360.0).default_size(cao_thu);
    if m.truong.gon_thu {
        p_thu = p_thu.exact_size(CAO_DONG_THU);
    }
    p_thu.show(ui, |ui| dai_thu(m, ui, &loai, file.as_ref()));
    let rong_pdf = (ui.available_width() / 3.0).round();
    egui::Panel::right("bt_pdf").resizable(true).default_size(rong_pdf).size_range(240.0..=ui.available_width() * 0.7).frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
        khung_pdf(m, ui, &loai, file.as_ref(), cfg, khoa);
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| bang(m, ui, &loai));
}

/// Bảng trường chỉ xem: # | Tên cột + chip biến | Mô tả (Câu lệnh) | Chuẩn hoá ✎.
fn bang(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu) {
    let t = &mut m.truong;
    let khung = ui.available_rect_before_wrap();
    let rong_mo_ta = (khung.width() - RONG_SO - t.rong_ten - t.rong_chuan).max(120.0);
    let x = [khung.left(), khung.left() + RONG_SO, khung.left() + RONG_SO + t.rong_ten, khung.left() + RONG_SO + t.rong_ten + rong_mo_ta];
    // Tiêu đề
    let dau = Rect::from_min_size(khung.min, egui::vec2(khung.width(), CAO_DAU));
    ui.allocate_rect(dau, Sense::hover());
    let p = ui.painter().clone();
    p.hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let f = egui::FontId::proportional(12.0);
    for (i, ten) in ["#", "Tên cột"].iter().enumerate() {
        p.text(egui::pos2(x[i] + 12.0, dau.center().y), egui::Align2::LEFT_CENTER, *ten, f.clone(), CHU_PHU);
    }
    // "Mô tả (Câu lệnh)": link mở popup Câu lệnh
    let g1 = p.layout_no_wrap("Mô tả (".into(), f.clone(), CHU_PHU);
    let g2 = p.layout_no_wrap("Câu lệnh".into(), f.clone(), VIEN_TAB_CHON);
    let x_link = x[2] + 12.0 + g1.size().x;
    let r_link = Rect::from_min_size(egui::pos2(x_link, dau.center().y - g2.size().y / 2.0), g2.size());
    p.galley(egui::pos2(x[2] + 12.0, dau.center().y - g1.size().y / 2.0), g1, CHU_PHU);
    p.text(egui::pos2(r_link.right(), dau.center().y), egui::Align2::LEFT_CENTER, ")", f.clone(), CHU_PHU);
    let link = ui.interact(r_link, ui.scope_id().with("bt_link_cau_lenh"), Sense::click());
    link.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Câu lệnh"));
    if link.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        p.hline(r_link.x_range(), r_link.bottom() - 1.0, Stroke::new(1.0, VIEN_TAB_CHON));
    }
    p.galley(r_link.min, g2, VIEN_TAB_CHON);
    let mo_cau_lenh = link.on_hover_text("Xem toàn văn câu lệnh gửi mô hình đọc").clicked();
    p.text(egui::pos2(x[3] + 12.0, dau.center().y), egui::Align2::LEFT_CENTER, "Chuẩn hoá", f.clone(), CHU_PHU);
    // ✎ mở popup Chỉnh sửa loại tài liệu
    let o_but = Rect::from_center_size(egui::pos2(dau.right() - 18.0, dau.center().y), egui::vec2(22.0, 22.0));
    let but = ui.interact(o_but, ui.scope_id().with("bt_sua_loai"), Sense::click());
    but.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Chỉnh sửa loại tài liệu"));
    if but.hovered() {
        p.rect_filled(o_but, 3.0, NUT_HOVER);
    }
    bt::ve(&p, o_but.shrink(4.0), BieuTuong::But, CHU_PHU);
    let mut mo_sua = but.on_hover_text("Chỉnh sửa loại tài liệu: tên cột, mô tả, chuẩn hoá, thứ tự").clicked();
    // Kéo ranh giới cột ở mép tiêu đề
    for (k, xx) in [(0usize, x[2]), (1, x[3])] {
        let o = Rect::from_center_size(egui::pos2(xx, dau.center().y), egui::vec2(8.0, CAO_DAU));
        let keo = ui.interact(o, ui.scope_id().with(("bt_keo_cot", k)), Sense::drag());
        if keo.hovered() || keo.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            p.vline(xx, dau.y_range(), Stroke::new(1.0, VIEN_TAB_CHON));
        }
        let dx = keo.drag_delta().x;
        if dx != 0.0 {
            if k == 0 {
                t.rong_ten = (t.rong_ten + dx).clamp(120.0, 360.0);
            } else {
                t.rong_chuan = (t.rong_chuan - dx).clamp(130.0, 320.0);
            }
        }
    }
    // Các dòng
    let than = Rect::from_min_max(egui::pos2(khung.left(), dau.bottom()), khung.max);
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(than));
    egui::ScrollArea::vertical().id_salt("bt_bang_truong").auto_shrink([false, false]).show(&mut con, |ui| {
        let f13 = egui::FontId::proportional(13.0);
        for (i, tr) in loai.truong.iter().enumerate() {
            let mut job = egui::text::LayoutJob::simple(tr.mo_ta.clone(), f13.clone(), CHU, rong_mo_ta - 24.0);
            job.wrap.max_width = rong_mo_ta - 24.0;
            let g_mo_ta = ui.fonts_mut(|fo| fo.layout_job(job));
            let cao = (g_mo_ta.size().y + 22.0).max(56.0);
            let (r, ph) = ui.allocate_exact_size(egui::vec2(than.width(), cao), Sense::click());
            let p = ui.painter();
            if t.chon == i {
                p.rect_filled(r, 0.0, NEN_TAB_CHON);
            } else if ph.hovered() {
                p.rect_filled(r, 0.0, Color32::from_gray(0xF7));
            }
            p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
            p.text(egui::pos2(x[0] + 12.0, r.top() + 11.0), egui::Align2::LEFT_TOP, (i + 1).to_string(), f13.clone(), CHU_PHU);
            p.text(egui::pos2(x[1] + 12.0, r.top() + 10.0), egui::Align2::LEFT_TOP, &tr.ten_cot, f13.clone(), CHU);
            // Chip biến {bien}
            let g = p.layout_no_wrap(format!("{{{}}}", tr.bien), egui::FontId::monospace(11.0), CHU_PHU);
            let chip = Rect::from_min_size(egui::pos2(x[1] + 10.0, r.top() + 30.0), g.size() + egui::vec2(10.0, 6.0));
            p.rect_filled(chip, CornerRadius::same(3), NEN_CHIP);
            p.galley(chip.min + egui::vec2(5.0, 3.0), g, CHU_PHU);
            p.galley(egui::pos2(x[2] + 12.0, r.top() + 10.0), g_mo_ta, CHU);
            let ten_ch = if tr.chuan_hoa == ChuanHoa::DanhSach { format!("Danh sách ({})", tr.danh_sach.len()) } else { tr.chuan_hoa.ten().to_string() };
            p.text(egui::pos2(x[3] + 12.0, r.top() + 10.0), egui::Align2::LEFT_TOP, ten_ch, f13.clone(), CHU);
            if tr.chuan_hoa == ChuanHoa::DanhSach {
                let mut job = egui::text::LayoutJob::single_section(tr.danh_sach.join(", "), egui::TextFormat { font_id: egui::FontId::proportional(11.0), color: CHU_PHU, ..Default::default() });
                job.wrap = egui::text::TextWrapping::truncate_at_width(t.rong_chuan - 24.0);
                let g = ui.fonts_mut(|fo| fo.layout_job(job));
                ui.painter().galley(egui::pos2(x[3] + 12.0, r.top() + 30.0), g, CHU_PHU);
            }
            ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, t.chon == i, &tr.ten_cot));
            if ph.clicked() {
                t.chon = i;
            }
            if ph.double_clicked() {
                mo_sua = true;
            }
        }
    });
    if mo_cau_lenh {
        m.hop = Some(Hop::CauLenh(false));
    }
    if mo_sua {
        if let Some(i) = m.loai {
            m.hop = Some(Hop::Sua(hop_loai::BanSua::moi(&m.ds_loai[i], i)));
        }
    }
}

/// Khung PDF: thanh 30 px "‹ tên file ›" · Thử đọc · − 100% +, dưới là các trang.
fn khung_pdf(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, file: Option<&FilePdf>, cfg: &CauHinhBocTach, khoa: &BTreeMap<String, String>) {
    let n_file = m.files.len();
    let dang_thu = m.truong.thu.is_some();
    let thanh = Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.max_rect().width(), CAO_DAU));
    ui.painter().hline(thanh.x_range(), thanh.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(thanh.shrink2(egui::vec2(8.0, 3.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    con.spacing_mut().item_spacing.x = 2.0;
    if bt::nut(&mut con, BieuTuong::TraiChevron, "File trước", m.truong.i_file > 0).clicked() {
        m.truong.i_file -= 1;
    }
    let ten = file.map_or(String::new(), |f| f.ten.clone());
    let rong_ten = (con.available_width() - 24.0 - 6.0 - 76.0 - 3.0 * 24.0 - 44.0 - 16.0).max(40.0);
    let mut job = egui::text::LayoutJob::single_section(ten.clone(), egui::TextFormat { font_id: egui::FontId::proportional(12.5), color: CHU, ..Default::default() });
    job.wrap = egui::text::TextWrapping::truncate_at_width(rong_ten);
    let g = con.fonts_mut(|fo| fo.layout_job(job));
    con.add(egui::Label::new(g).sense(Sense::hover())).on_hover_text(&ten);
    if bt::nut(&mut con, BieuTuong::PhaiChevron, "File sau", m.truong.i_file + 1 < n_file).clicked() {
        m.truong.i_file += 1;
    }
    con.add_space(6.0);
    let ncc = m.ncc;
    let co_mo_hinh = ncc.is_some_and(|n| cfg.da_cai(n, khoa.get(n.ma()).map_or("", String::as_str)));
    let nut = egui::Button::new(egui::RichText::new(if dang_thu { "Đang đọc…" } else { "Thử đọc" }).color(Color32::WHITE).size(12.0)).fill(if co_mo_hinh && !dang_thu { XANH_LA } else { Color32::from_gray(0xB8) }).min_size(egui::vec2(76.0, 22.0));
    let bam = con
        .add_enabled(co_mo_hinh && !dang_thu && file.is_some(), nut)
        .on_hover_text("Đọc thử file đang xem bằng mô hình đã chọn — không trừ lượt chạy")
        .on_disabled_hover_text(if dang_thu { "Đang đọc thử" } else { "Mô hình đọc chưa khai đủ ở Cấu hình" })
        .clicked();
    if bam {
        if let (Some(f), Some(n)) = (file, ncc) {
            let kn = cfg.ket_noi(n, khoa.get(n.ma()).map_or("", String::as_str));
            m.truong.thu_doc(ui.ctx(), f, loai, cfg, kn);
        }
    }
    con.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let x = &mut m.truong.xem;
        if bt::nut(ui, BieuTuong::KinhCong, "Phóng to", x.nac + 1 < NAC.len()).clicked() {
            x.nac += 1;
        }
        ui.add_sized(egui::vec2(44.0, 22.0), egui::Label::new(egui::RichText::new(format!("{}%", x.ty_le())).size(12.0)));
        if bt::nut(ui, BieuTuong::KinhTru, "Thu nhỏ", x.nac > 0).clicked() {
            x.nac -= 1;
        }
    });
    ui.allocate_rect(thanh, Sense::hover());
    m.truong.xem.mo(file.map(|f| f.duong_dan.as_path()));
    let vung = Rect::from_min_max(egui::pos2(thanh.left() + 1.0, thanh.bottom()), ui.max_rect().max);
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(vung));
    m.truong.xem.ve(&mut con);
}

/// Dải "Kết quả đọc thử": tiêu đề + trạng thái, bảng 2 dòng (tên cột | giá trị); bấm cột → chọn trường.
fn dai_thu(m: &mut ManBocTach, ui: &mut egui::Ui, loai: &LoaiTaiLieu, file: Option<&FilePdf>) {
    let t = &mut m.truong;
    let khung = ui.max_rect();
    let dau = Rect::from_min_size(khung.min, egui::vec2(khung.width(), CAO_DONG_THU));
    let ph_dau = ui.interact(dau, ui.scope_id().with("bt_dau_thu"), Sense::click());
    let p = ui.painter().clone();
    p.rect_filled(dau, 0.0, NEN_TIEU_DE);
    p.hline(dau.x_range(), dau.top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
    p.hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let gd = p.layout_no_wrap("Kết quả đọc thử".into(), egui::FontId::new(12.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU);
    let x_tt = dau.left() + 12.0 + gd.size().x + 10.0;
    p.galley(egui::pos2(dau.left() + 12.0, dau.center().y - gd.size().y / 2.0), gd, CHU);
    let (tt, mau) = match (&t.thu, &t.kq_thu) {
        (Some(d), _) => (format!("Đang đọc {} · {} s", file.map_or("", |f| f.ten.as_str()), giay(d.bat_dau.elapsed())), CHU_PHU),
        (_, Some((i, Ok(kq)))) => {
            let ten = m.files.get(*i).map_or("", |f| f.ten.as_str());
            (format!("{ten} · {}/{} trường · đọc {} trang · {} s", kq.so_co(), loai.truong.len(), kq.tho.len(), giay(kq.thoi_gian)), CHU_PHU)
        }
        (_, Some((_, Err(e)))) => (format!("Lỗi: {e}"), DO),
        _ => ("Chưa đọc — bấm Thử đọc".into(), CHU_PHU),
    };
    let mut job = egui::text::LayoutJob::single_section(tt.clone(), egui::TextFormat { font_id: egui::FontId::proportional(12.0), color: mau, ..Default::default() });
    job.wrap = egui::text::TextWrapping::truncate_at_width((dau.right() - 40.0 - x_tt).max(40.0));
    let g = ui.fonts_mut(|fo| fo.layout_job(job));
    p.galley(egui::pos2(x_tt, dau.center().y - g.size().y / 2.0), g, mau);
    if t.thu.is_some() {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    // Thu / mở: icon gạch + mũi tên góc phải; bấm cả thanh cũng được
    bt::ve(&p, Rect::from_center_size(egui::pos2(dau.right() - 18.0, dau.center().y), egui::vec2(14.0, 14.0)), if t.gon_thu { BieuTuong::HienDong } else { BieuTuong::AnDong }, CHU);
    ph_dau.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, if t.gon_thu { "Mở Kết quả đọc thử" } else { "Thu gọn Kết quả đọc thử" }));
    if ph_dau.on_hover_text(&tt).clicked() {
        t.gon_thu = !t.gon_thu;
    }
    if t.gon_thu {
        return;
    }
    // Bảng: Tên file | Số trang | các trường; giá trị "-" khi chưa thử
    let cot: Vec<(String, String)> = [("ten_file".to_string(), "Tên file".to_string()), ("so_trang".into(), "Số trang".into())].into_iter().chain(loai.truong.iter().map(|tr| (tr.bien.clone(), tr.ten_cot.clone()))).collect();
    let kq_dung = t.kq_thu.as_ref().filter(|(i, _)| *i == t.i_file);
    let gia_tri: Vec<String> = cot
        .iter()
        .enumerate()
        .map(|(k, _)| match k {
            0 => file.map_or(String::new(), |f| f.ten.clone()),
            1 => file.map_or(String::new(), |f| f.so_trang.to_string()),
            _ => match kq_dung {
                Some((_, Ok(kq))) => kq.gia_tri.get(k - 2).filter(|g| !g.is_empty()).cloned().unwrap_or_else(|| "-".into()),
                _ => "-".into(),
            },
        })
        .collect();
    let vung = Rect::from_min_max(egui::pos2(khung.left(), dau.bottom()), khung.max);
    let mut con = ui.new_child(egui::UiBuilder::new().max_rect(vung));
    egui::ScrollArea::horizontal().id_salt("bt_bang_thu").auto_shrink([false, false]).show(&mut con, |ui| {
        let rong: Vec<f32> = cot.iter().map(|(b, _)| *t.rong_cot_thu.get(b).unwrap_or(&rong_mac_dinh(b))).collect();
        let tong: f32 = rong.iter().sum();
        let (r, _) = ui.allocate_exact_size(egui::vec2(tong.max(vung.width()), 2.0 * CAO_DONG_THU), Sense::hover());
        let p = ui.painter_at(Rect::from_min_max(r.min, egui::pos2(r.right(), vung.bottom())));
        p.rect_filled(Rect::from_min_size(r.min, egui::vec2(r.width(), CAO_DONG_THU)), 0.0, NEN_DAU_BANG);
        let mut xx = r.left();
        for (k, ((bien, ten), w)) in cot.iter().zip(&rong).enumerate() {
            let o = Rect::from_min_size(egui::pos2(xx, r.top()), egui::vec2(*w, 2.0 * CAO_DONG_THU));
            let la_chon = k >= 2 && k - 2 == t.chon;
            if la_chon {
                p.rect_filled(o, 0.0, NEN_TAB_CHON);
            }
            let ph = ui.interact(o, ui.scope_id().with(("bt_cot_thu", k)), if k >= 2 { Sense::click() } else { Sense::hover() });
            if k >= 2 {
                ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, la_chon, format!("Cột {ten}")));
                if ph.clicked() {
                    t.chon = k - 2;
                }
            }
            for (dong, chu, mau) in [(0, ten.as_str(), CHU_PHU), (1, gia_tri[k].as_str(), if gia_tri[k] == "-" { CHU_MO } else { CHU })] {
                let mut job = egui::text::LayoutJob::single_section(chu.to_string(), egui::TextFormat { font_id: egui::FontId::proportional(12.5), color: mau, ..Default::default() });
                job.wrap = egui::text::TextWrapping::truncate_at_width(w - 20.0);
                let g = ui.fonts_mut(|fo| fo.layout_job(job));
                p.galley(egui::pos2(o.left() + 12.0, o.top() + dong as f32 * CAO_DONG_THU + (CAO_DONG_THU - g.size().y) / 2.0), g, mau);
            }
            if gia_tri[k].len() > 20 {
                ph.on_hover_text(&gia_tri[k]);
            }
            // Kéo mép phải tiêu đề đổi độ rộng cột
            let mep = Rect::from_center_size(egui::pos2(o.right(), r.top() + CAO_DONG_THU / 2.0), egui::vec2(8.0, CAO_DONG_THU));
            let keo = ui.interact(mep, ui.scope_id().with(("bt_keo_thu", k)), Sense::drag());
            if keo.hovered() || keo.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }
            if keo.drag_delta().x != 0.0 {
                t.rong_cot_thu.insert(bien.clone(), (w + keo.drag_delta().x).clamp(60.0, 600.0));
            }
            xx += w;
        }
        p.hline(r.x_range(), r.top() + CAO_DONG_THU - 0.5, Stroke::new(1.0, VIEN_NHAT));
        p.hline(r.x_range(), r.top() + 2.0 * CAO_DONG_THU - 0.5, Stroke::new(1.0, VIEN_NHAT));
    });
    // Giữ chiều cao panel (kéo mép trên đổi được), không co theo nội dung
    ui.allocate_rect(khung, Sense::hover());
}

/// Giây 1 chữ số thập phân, dấu phẩy: 2,1.
fn giay(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f32()).replace('.', ",")
}
