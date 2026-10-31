//! Cấu hình › Bóc tách (prototype `#cau-hinh-boc-tach`, `-gemini`, `-nguon`): sơ đồ luồng Nguồn văn bản → Mô hình đọc
//! → Ghi kết quả là thanh chuyển mục; Mô hình đọc = 3 bước (chọn nhà cung cấp · địa chỉ / khoá · model + kiểm tra).

use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::boc_tach::mo_hinh::NhaCungCap;
use crate::cau_hinh::{CachGhiSheet, CauHinh, CauHinhBocTach, MoHinhBocTach, CANH_KHOANG, CHO_KHOANG, DPI_KHOANG, JPEG_KHOANG, SONG_SONG_KHOANG, TOKEN_KHOANG};
use crate::gui::{ChuDam, CHU, CHU_MO, CHU_PHU, DO, VIEN, VIEN_NHAT, VIEN_TAB_CHON, XANH_LA};
use crate::gui_tach::bieu_tuong::{self as bt, BieuTuong};
use crate::gui_tach::khung_hop as kh;

const RONG_NHAN: f32 = 110.0;
const NEN_CHON: Color32 = Color32::from_rgb(0xEE, 0xF5, 0xFE);
const NAU: Color32 = Color32::from_rgb(0x8A, 0x5A, 0x00);
const NEN_CANH_BAO: Color32 = Color32::from_rgb(0xFF, 0xF8, 0xE5);
const VIEN_CANH_BAO: Color32 = Color32::from_rgb(0xF0, 0xD8, 0x9A);
/// Khoá Apps Script nằm cùng kho khoá API (URL Web app ghi được vào Sheet nên coi như bí mật).
pub const KHOA_APPS_SCRIPT: &str = "apps_script";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Muc {
    Nguon,
    MoHinh,
    Ghi,
}

type KetQuaKiemTra = Result<(Vec<String>, Duration), String>;

pub struct TabBocTach {
    pub muc: Muc,
    /// Nhà cung cấp đang sửa ở bước 1.
    pub ncc: NhaCungCap,
    nang_cao: bool,
    dang_kiem_tra: Option<(NhaCungCap, mpsc::Receiver<KetQuaKiemTra>)>,
    kq_kiem_tra: HashMap<NhaCungCap, Result<(usize, Duration), String>>,
    ds_model: HashMap<NhaCungCap, Vec<String>>,
    /// (đường dẫn file khoá service account, email đọc được trong file).
    email_sa: (String, Option<String>),
}

/// Mặc định tab Bóc tách: chỉ phần Nâng cao (ảnh gửi đi, token, song song, chờ); giữ nhà cung cấp, model, Sheet, lựa chọn gần nhất.
pub fn mac_dinh(c: &CauHinh) -> CauHinh {
    let md = CauHinhBocTach::default();
    let mut b = CauHinhBocTach { dpi: md.dpi, canh_toi_da: md.canh_toi_da, jpeg: md.jpeg, max_tokens: md.max_tokens, ..c.boc_tach.clone() };
    for n in NhaCungCap::TAT_CA {
        let m = b.mo_hinh_mut(n);
        let d = MoHinhBocTach::mac_dinh(n);
        (m.song_song, m.cho_s) = (d.song_song, d.cho_s);
    }
    CauHinh { boc_tach: b, ..c.clone() }
}

/// "Máy chủ nội bộ · qwen2.5-vl-7b" cho ô Mô hình đọc của sơ đồ luồng / ô chọn ở màn chính.
pub fn ten_mo_hinh(ch: &CauHinhBocTach, n: NhaCungCap) -> String {
    let ten = if n == NhaCungCap::NoiBo { "Máy chủ nội bộ" } else { n.ten() };
    let m = &ch.mo_hinh(n).model;
    if m.is_empty() { ten.into() } else { format!("{ten} · {m}") }
}

fn icon_ncc(n: NhaCungCap) -> BieuTuong {
    match n {
        NhaCungCap::NoiBo => BieuTuong::MayChu,
        NhaCungCap::ChatGpt => BieuTuong::NccOpenAi,
        NhaCungCap::Gemini => BieuTuong::NccGemini,
        NhaCungCap::Claude => BieuTuong::NccClaude,
        NhaCungCap::OpenRouter => BieuTuong::NccOpenRouter,
    }
}

fn khoa_cua<'a>(khoa: &'a BTreeMap<String, String>, n: NhaCungCap) -> &'a str {
    khoa.get(n.ma()).map_or("", String::as_str)
}

impl TabBocTach {
    pub fn moi(ch: &CauHinhBocTach) -> Self {
        Self {
            muc: Muc::MoHinh,
            ncc: ch.mac_dinh.unwrap_or(NhaCungCap::NoiBo),
            nang_cao: false,
            dang_kiem_tra: None,
            kq_kiem_tra: HashMap::new(),
            ds_model: HashMap::new(),
            email_sa: (String::new(), None),
        }
    }

    #[cfg(test)]
    pub fn mo_nang_cao(&mut self) {
        self.nang_cao = true;
    }

    pub fn ve(&mut self, ui: &mut egui::Ui, ch: &mut CauHinhBocTach, khoa: &mut BTreeMap<String, String>) {
        self.nhan_kiem_tra(ui.ctx(), ch);
        self.so_do(ui, ch, khoa);
        ui.add_space(10.0);
        the(ui, |ui| match self.muc {
            Muc::MoHinh => self.muc_mo_hinh(ui, ch, khoa),
            Muc::Nguon => muc_nguon(ui),
            Muc::Ghi => self.muc_ghi(ui, ch, khoa),
        });
    }

    /// 3 ô sơ đồ luồng nối mũi tên; ô là nút chuyển mục, dòng phụ tóm tắt lựa chọn.
    fn so_do(&mut self, ui: &mut egui::Ui, ch: &CauHinhBocTach, khoa: &BTreeMap<String, String>) {
        let mo_hinh = match ch.mac_dinh.filter(|n| ch.da_cai(*n, khoa_cua(khoa, *n))) {
            Some(n) => ten_mo_hinh(ch, n),
            None => "Chưa chọn mô hình mặc định".into(),
        };
        let ghi = if ch.ghi_sheet || !ch.sheet_link.trim().is_empty() { "Excel + Google Sheet" } else { "Excel" };
        let o = [(Muc::Nguon, BieuTuong::ThuMuc, "Nguồn văn bản", "Máy tính".to_string()), (Muc::MoHinh, BieuTuong::MoHinhDoc, "Mô hình đọc", mo_hinh), (Muc::Ghi, BieuTuong::BangKetQua, "Ghi kết quả", ghi.into())];
        let mui = 26.0;
        let rong = (ui.available_width() - 2.0 * mui) / 3.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (muc, icon, ten, phu)) in o.into_iter().enumerate() {
                if i > 0 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(mui, 44.0), Sense::hover());
                    let (a, b) = (egui::pos2(r.left() + 4.0, r.center().y), egui::pos2(r.right() - 4.0, r.center().y));
                    ui.painter().line_segment([a, b], Stroke::new(1.0, VIEN));
                    ui.painter().add(egui::Shape::convex_polygon(vec![b, b + egui::vec2(-5.0, -3.5), b + egui::vec2(-5.0, 3.5)], VIEN, Stroke::NONE));
                }
                let (r, p) = ui.allocate_exact_size(egui::vec2(rong, 44.0), Sense::click());
                let chon = self.muc == muc;
                o_chon(ui, r, &p, chon, true);
                bt::ve(ui.painter(), Rect::from_center_size(egui::pos2(r.left() + 24.0, r.center().y), egui::vec2(20.0, 20.0)), icon, CHU);
                hai_dong(ui, egui::pos2(r.left() + 46.0, r.top() + 7.0), r.width() - 54.0, ten, &phu, false, 13.0);
                p.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chon, ten));
                if p.clicked() {
                    self.muc = muc;
                }
            }
        });
    }

    fn muc_mo_hinh(&mut self, ui: &mut egui::Ui, ch: &mut CauHinhBocTach, khoa: &mut BTreeMap<String, String>) {
        let n = self.ncc;
        // Bước 1: 5 ô nhà cung cấp 1 hàng + ★ Mặc định / Đặt làm mặc định · Xoá
        ui.horizontal(|ui| {
            buoc(ui, 1, "Chọn nhà cung cấp");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                if lien_ket(ui, "Xoá", DO).on_hover_text("Xoá địa chỉ, khoá, model đã khai của nhà cung cấp này").clicked() {
                    *ch.mo_hinh_mut(n) = MoHinhBocTach::mac_dinh(n);
                    khoa.remove(n.ma());
                    self.kq_kiem_tra.remove(&n);
                    if ch.mac_dinh == Some(n) {
                        ch.mac_dinh = None;
                    }
                }
                if ch.mac_dinh == Some(n) {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new("Mặc định").size(12.5).color(NAU));
                    let (r, _) = ui.allocate_exact_size(egui::vec2(13.0, 13.0), Sense::hover());
                    sao(ui.painter(), r, NAU);
                } else if lien_ket(ui, "Đặt làm mặc định", VIEN_TAB_CHON).on_hover_text("Chọn sẵn mô hình này ở màn Bóc tách").clicked() {
                    ch.mac_dinh = Some(n);
                }
            });
        });
        let rong = (ui.available_width() - 4.0 * 8.0) / 5.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for x in NhaCungCap::TAT_CA {
                let cai = ch.da_cai(x, khoa_cua(khoa, x));
                let phu = if cai { ch.mo_hinh(x).model.clone() } else { "Chưa cài đặt".into() };
                let (r, p) = ui.allocate_exact_size(egui::vec2(rong, 40.0), Sense::click());
                o_chon(ui, r, &p, x == n, true);
                bt::ve(ui.painter(), Rect::from_center_size(egui::pos2(r.left() + 17.0, r.center().y), egui::vec2(15.0, 15.0)), icon_ncc(x), CHU);
                hai_dong(ui, egui::pos2(r.left() + 32.0, r.top() + 5.0), r.width() - 38.0, x.ten(), &phu, !cai, 12.5);
                p.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, x == n, x.ten()));
                if p.clicked() {
                    self.ncc = x;
                }
            }
        });
        ui.add_space(6.0);
        // Bước 2: địa chỉ (dịch vụ đám mây tự điền, khoá ô) + khoá API
        buoc(ui, 2, if n.la_dam_may() { "Dán khoá API" } else { "Địa chỉ máy chủ" });
        dong(ui, "Địa chỉ", |ui| {
            if n.la_dam_may() {
                let mut dc = n.dia_chi_mac_dinh().to_string();
                ui.add_enabled(false, kh::o_nhap(&mut dc).desired_width(ui.available_width()).hint_text("Địa chỉ")).on_disabled_hover_text("Tự điền theo nhà cung cấp");
            } else {
                ui.add(kh::o_nhap(&mut ch.mo_hinh_mut(n).dia_chi).desired_width(ui.available_width()).hint_text("http://máy-chủ:cổng/v1"));
            }
        });
        if !n.la_dam_may() {
            goi_y(ui, "vLLM: cổng 8000 · Ollama: 11434 · LM Studio: 1234 — địa chỉ kết thúc bằng /v1");
        }
        dong(ui, "Khoá API", |ui| {
            let k = khoa.entry(n.ma().to_string()).or_default();
            let goi = if n.la_dam_may() { "Dán khoá API" } else { "Không bắt buộc" };
            ui.add(kh::o_nhap(k).password(true).desired_width(ui.available_width()).hint_text(goi));
        });
        if n.la_dam_may() {
            goi_y(ui, &format!("Lấy khoá: {}", n.noi_lay_khoa()));
        }
        if !crate::boc_tach::khoa_cache::co_luu() {
            goi_y_mau(ui, "Bản chạy này không lưu khoá ra đĩa — khoá chỉ dùng trong phiên làm việc", NAU);
        }
        ui.add_space(6.0);
        // Bước 3: model + kiểm tra kết nối
        buoc(ui, 3, "Chọn model, kiểm tra");
        let dang = self.dang_kiem_tra.as_ref().is_some_and(|(x, _)| *x == n);
        dong(ui, "Model", |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let ds = self.ds_model.get(&n).cloned().unwrap_or_default();
            // Phải → trái: nút Kiểm tra, nút ▾ danh sách model, ô model lấp phần còn lại
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let co_dia_chi = n.la_dam_may() || !ch.mo_hinh(n).dia_chi.trim().is_empty();
                let nut = ui.add_enabled(!dang && co_dia_chi, egui::Button::new("Kiểm tra kết nối").min_size(egui::vec2(120.0, 24.0))).on_hover_text("Gửi 1 yêu cầu lấy danh sách model").on_disabled_hover_text("Nhập địa chỉ máy chủ trước");
                if nut.clicked() {
                    self.kiem_tra(ui.ctx(), ch, n, khoa_cua(khoa, n));
                }
                ui.add_enabled_ui(!ds.is_empty(), |ui| {
                    let (nut, _) = egui::containers::menu::MenuButton::from_button(egui::Button::new(bt::atom_mau(BieuTuong::ChevronXuong, 13.0, CHU)).min_size(egui::vec2(24.0, 24.0))).ui(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for m in &ds {
                                if ui.selectable_label(ch.mo_hinh(n).model == *m, m).clicked() {
                                    ch.mo_hinh_mut(n).model = m.clone();
                                    ui.close();
                                }
                            }
                        });
                    });
                    nut.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Chọn model"));
                })
                .response
                .on_disabled_hover_text("Bấm Kiểm tra kết nối để lấy danh sách model");
                ui.add(kh::o_nhap(&mut ch.mo_hinh_mut(n).model).desired_width(ui.available_width()).hint_text("Tên model, hoặc Kiểm tra kết nối để lấy danh sách"));
            });
        });
        let (chu, mau, dau) = match (dang, self.kq_kiem_tra.get(&n)) {
            (true, _) => ("Đang kiểm tra…".to_string(), CHU_PHU, None),
            (_, Some(Ok((so, t)))) => (format!("Kết nối được · {so} model · {:.1} s", t.as_secs_f32()).replace('.', ","), XANH_LA, Some(true)),
            (_, Some(Err(e))) => (e.clone(), DO, Some(false)),
            _ => (if n.la_dam_may() { "Chưa kiểm tra — dán khoá API rồi bấm Kiểm tra kết nối" } else { "Chưa kiểm tra — nhập địa chỉ rồi bấm Kiểm tra kết nối" }.to_string(), CHU_PHU, None),
        };
        ui.horizontal(|ui| {
            ui.add_space(RONG_NHAN + 8.0);
            ui.spacing_mut().item_spacing.x = 5.0;
            if let Some(ok) = dau {
                let (r, _) = ui.allocate_exact_size(egui::vec2(11.0, 14.0), Sense::hover());
                dau_ket_qua(ui.painter(), r, ok, mau);
            }
            ui.add(egui::Label::new(RichText::new(chu).size(11.5).color(mau)).wrap());
        });
        if n.la_dam_may() {
            ui.add_space(4.0);
            egui::Frame::new().fill(NEN_CANH_BAO).stroke(Stroke::new(1.0, VIEN_CANH_BAO)).corner_radius(4).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    canh_bao(ui.painter(), r, NAU);
                    ui.label(RichText::new(format!("Ảnh trang PDF gửi tới máy chủ của {}. Không dùng cho tài liệu mật.", n.cong_ty())).size(12.0).color(NAU));
                });
            });
        }
        ui.add_space(6.0);
        let mui = if self.nang_cao { BieuTuong::ChevronXuong } else { BieuTuong::PhaiChevron };
        let nut = ui.add(egui::Button::new((bt::atom_mau(mui, 12.0, CHU), RichText::new("Nâng cao: song song, thời gian chờ, ảnh gửi đi").size(12.5))).frame(false));
        if nut.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if nut.clicked() {
            self.nang_cao = !self.nang_cao;
        }
        if self.nang_cao {
            let m = ch.mo_hinh_mut(n);
            dong(ui, "Chạy song song", |ui| {
                so(ui, &mut m.song_song, SONG_SONG_KHOANG);
                ui.label("yêu cầu");
                ui.add_space(12.0);
                ui.label(RichText::new("Chờ tối đa").color(CHU_PHU));
                so(ui, &mut m.cho_s, CHO_KHOANG);
                ui.label("giây, thử lại 1 lần");
            });
            dong(ui, "Ảnh gửi đi", |ui| {
                so(ui, &mut ch.dpi, DPI_KHOANG);
                ui.label("DPI");
                ui.add_space(12.0);
                ui.label(RichText::new("cạnh tối đa").color(CHU_PHU));
                so(ui, &mut ch.canh_toi_da, CANH_KHOANG);
                ui.label("px");
                ui.add_space(12.0);
                ui.label(RichText::new("JPEG").color(CHU_PHU));
                so(ui, &mut ch.jpeg, JPEG_KHOANG);
            });
            dong(ui, "Số token tối đa", |ui| {
                so(ui, &mut ch.max_tokens, TOKEN_KHOANG);
                ui.label(RichText::new("mỗi câu trả lời; tăng khi mô hình trả lời bị cụt").color(CHU_PHU));
            });
            goi_y(ui, "Song song và thời gian chờ riêng cho từng nhà cung cấp; ảnh gửi đi dùng chung. Mỗi yêu cầu gửi 1 trang.");
        }
    }

    fn kiem_tra(&mut self, ctx: &egui::Context, ch: &CauHinhBocTach, n: NhaCungCap, khoa: &str) {
        let kn = ch.ket_noi(n, khoa);
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let t = Instant::now();
            let _ = gui.send(kn.liet_ke_model().map(|ds| (ds, t.elapsed())).map_err(|e| e.to_string()));
            ctx.request_repaint();
        });
        self.kq_kiem_tra.remove(&n);
        self.dang_kiem_tra = Some((n, nhan));
    }

    fn nhan_kiem_tra(&mut self, ctx: &egui::Context, ch: &mut CauHinhBocTach) {
        let Some((n, nhan)) = &self.dang_kiem_tra else { return };
        let n = *n;
        match nhan.try_recv() {
            Ok(kq) => {
                self.dang_kiem_tra = None;
                match kq {
                    Ok((ds, t)) => {
                        // Chưa có model → điền model đầu tiên của máy chủ (máy chủ nội bộ thường chỉ chạy 1 model)
                        let m = ch.mo_hinh_mut(n);
                        if m.model.trim().is_empty() {
                            m.model = ds.first().cloned().unwrap_or_default();
                        }
                        self.kq_kiem_tra.insert(n, Ok((ds.len(), t)));
                        self.ds_model.insert(n, ds);
                    }
                    Err(e) => {
                        self.kq_kiem_tra.insert(n, Err(e));
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            Err(mpsc::TryRecvError::Disconnected) => self.dang_kiem_tra = None,
        }
    }

    fn muc_ghi(&mut self, ui: &mut egui::Ui, ch: &mut CauHinhBocTach, khoa: &mut BTreeMap<String, String>) {
        tieu_de_the(ui, "File Excel", "— luôn tạo");
        dong(ui, "Thư mục ghi", |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add(kh::o_nhap(&mut ch.thu_muc_excel).desired_width(ui.available_width() - 34.0).hint_text("Để trống = cạnh thư mục nguồn"));
            if ui.add(egui::Button::new("…").min_size(egui::vec2(30.0, 24.0))).on_hover_text("Chọn thư mục").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    ch.thu_muc_excel = p.display().to_string();
                }
            }
        });
        ui.add_space(10.0);
        kh::ke_ngang(ui, VIEN_NHAT);
        ui.add_space(8.0);
        tieu_de_the(ui, "Google Sheet", "— ghi thêm 1 dòng sau mỗi file");
        dong(ui, "Link Sheet", |ui| {
            ui.add(kh::o_nhap(&mut ch.sheet_link).desired_width(ui.available_width()).hint_text("https://docs.google.com/spreadsheets/d/…/edit"));
        });
        dong(ui, "Tab", |ui| {
            ui.add(kh::o_nhap(&mut ch.sheet_tab).desired_width(180.0).hint_text("Tên tab"));
        });
        dong(ui, "Cách ghi", |ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            kh::o_radio(ui, &mut ch.sheet_cach, CachGhiSheet::ServiceAccount, "Khoá service account");
            kh::o_radio(ui, &mut ch.sheet_cach, CachGhiSheet::AppsScript, "Apps Script");
        });
        match ch.sheet_cach {
            CachGhiSheet::ServiceAccount => {
                dong(ui, "File khoá JSON", |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.add(kh::o_nhap(&mut ch.sheet_file_khoa).desired_width(ui.available_width() - 64.0).hint_text("File khoá service account (.json)"));
                    if ui.add(egui::Button::new("Chọn").min_size(egui::vec2(60.0, 24.0))).clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("Khoá JSON", &["json"]).pick_file() {
                            ch.sheet_file_khoa = p.display().to_string();
                        }
                    }
                });
                if self.email_sa.0 != ch.sheet_file_khoa {
                    self.email_sa = (ch.sheet_file_khoa.clone(), email_service_account(&ch.sheet_file_khoa));
                }
                match &self.email_sa.1 {
                    Some(email) => {
                        ui.horizontal(|ui| {
                            ui.add_space(RONG_NHAN + 8.0);
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(RichText::new("Chia sẻ Sheet cho").size(11.5).color(CHU_PHU));
                            ui.label(RichText::new(email).size(11.5).dam());
                            ui.label(RichText::new("quyền Biên tập ·").size(11.5).color(CHU_PHU));
                            if lien_ket(ui, "Chép email", VIEN_TAB_CHON).clicked() {
                                ui.ctx().copy_text(email.clone());
                            }
                        });
                    }
                    None if !ch.sheet_file_khoa.trim().is_empty() => goi_y_mau(ui, "Không đọc được email trong file khoá (cần file JSON service account)", DO),
                    None => goi_y(ui, "Tạo service account trong Google Cloud, tải khoá JSON, rồi chia sẻ Sheet cho email của nó"),
                }
            }
            CachGhiSheet::AppsScript => {
                dong(ui, "URL Web app", |ui| {
                    let url = khoa.entry(KHOA_APPS_SCRIPT.to_string()).or_default();
                    ui.add(kh::o_nhap(url).desired_width(ui.available_width()).hint_text("https://script.google.com/macros/s/…/exec"));
                });
                goi_y(ui, "Dán script mẫu vào Sheet (Tiện ích › Apps Script), triển khai Web app rồi dán URL");
            }
        }
        ui.horizontal(|ui| {
            ui.add_space(RONG_NHAN + 8.0);
            ui.add_enabled(false, egui::Button::new("Ghi thử 1 dòng").min_size(egui::vec2(0.0, 24.0))).on_disabled_hover_text("Có khi hoàn thành phần ghi Google Sheet");
        });
    }
}

/// Mục Nguồn văn bản: bản đầu chỉ Máy tính; Drive / Cloud Storage / S3 có ở bản sau.
fn muc_nguon(ui: &mut egui::Ui) {
    buoc(ui, 1, "Chọn nguồn");
    let o = [(BieuTuong::ThuMuc, "Máy tính", "Luôn dùng được", true), (BieuTuong::GgDrive, "Google Drive", "Sắp có", false), (BieuTuong::DamMay, "Google Cloud Storage", "Sắp có", false), (BieuTuong::KhoS3, "Kho S3", "Sắp có", false)];
    let rong = (ui.available_width() - 3.0 * 8.0) / 4.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (icon, ten, phu, bat) in o {
            let (r, p) = ui.allocate_exact_size(egui::vec2(rong, 40.0), if bat { Sense::click() } else { Sense::hover() });
            o_chon(ui, r, &p, bat, bat);
            bt::ve(ui.painter(), Rect::from_center_size(egui::pos2(r.left() + 17.0, r.center().y), egui::vec2(15.0, 15.0)), icon, if bat { CHU } else { CHU_MO });
            hai_dong(ui, egui::pos2(r.left() + 32.0, r.top() + 5.0), r.width() - 38.0, ten, phu, !bat, 12.5);
            if !bat {
                p.on_hover_text("Có ở bản sau");
            }
        }
    });
    ui.add_space(6.0);
    buoc(ui, 2, "Thư mục");
    dong(ui, "Thư mục", |ui| {
        ui.label(RichText::new("Chọn ở màn chính (nút Chọn); gồm cả thư mục con.").color(CHU_PHU));
    });
    goi_y(ui, "Thư mục Google Drive for Desktop, ổ mạng cũng chọn như thư mục trên máy.");
}

/// Email `client_email` trong file khoá service account.
fn email_service_account(duong_dan: &str) -> Option<String> {
    if duong_dan.trim().is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(duong_dan.trim()).ok()?).ok()?;
    v["client_email"].as_str().filter(|e| e.contains('@')).map(str::to_string)
}

/// Khung thẻ trắng viền xám bo góc chứa nội dung mục.
fn the(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().stroke(Stroke::new(1.0, VIEN_NHAT)).corner_radius(6).inner_margin(Margin { left: 16, right: 16, top: 12, bottom: 14 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        f(ui);
    });
}

/// Ô chọn (sơ đồ luồng, nhà cung cấp, nguồn): chọn = viền xanh nền xanh nhạt; rê = viền đậm hơn.
fn o_chon(ui: &egui::Ui, r: Rect, p: &egui::Response, chon: bool, bat: bool) {
    if bat && p.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let (nen, vien) = if chon { (NEN_CHON, VIEN_TAB_CHON) } else if bat && p.hovered() { (Color32::WHITE, VIEN) } else { (Color32::WHITE, VIEN_NHAT) };
    ui.painter().rect(r, CornerRadius::same(5), nen, Stroke::new(1.0, vien), StrokeKind::Inside);
}

/// Tên đậm + dòng phụ nhỏ (xám nghiêng khi `mo`), cắt bằng "…" khi dài.
fn hai_dong(ui: &egui::Ui, goc: egui::Pos2, rong: f32, ten: &str, phu: &str, mo: bool, co: f32) {
    let ve = |chu: RichText, y: f32| {
        let mut job = egui::text::LayoutJob::default();
        chu.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
        job.wrap = egui::text::TextWrapping::truncate_at_width(rong);
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(egui::pos2(goc.x, goc.y + y), g, CHU);
    };
    ve(RichText::new(ten).size(co).color(CHU).dam(), 0.0);
    let phu = RichText::new(phu).size(11.0).color(if mo { CHU_MO } else { CHU_PHU });
    ve(if mo { phu.italics() } else { phu }, co + 4.0);
}

/// Ngôi sao 5 cánh (font giao diện không có ký tự ★).
fn sao(p: &egui::Painter, r: Rect, mau: Color32) {
    let (c, ngoai) = (r.center(), r.width() / 2.0);
    let dinh: Vec<egui::Pos2> = (0..10)
        .map(|i| {
            let goc = std::f32::consts::PI * (i as f32 / 5.0) - std::f32::consts::FRAC_PI_2;
            let k = if i % 2 == 0 { ngoai } else { ngoai * 0.45 };
            c + egui::vec2(goc.cos(), goc.sin()) * k
        })
        .collect();
    // Hình sao lõm: vẽ 5 tam giác cánh + ngũ giác giữa
    for i in 0..5 {
        p.add(egui::Shape::convex_polygon(vec![dinh[2 * i], dinh[(2 * i + 1) % 10], c, dinh[(2 * i + 9) % 10]], mau, Stroke::NONE));
    }
}

/// Dấu ✓ (đúng) / ✗ (lỗi) vẽ nét.
fn dau_ket_qua(p: &egui::Painter, r: Rect, ok: bool, mau: Color32) {
    let s = Stroke::new(1.6, mau);
    let c = r.center();
    if ok {
        p.line(vec![c + egui::vec2(-4.5, 0.0), c + egui::vec2(-1.5, 3.0), c + egui::vec2(4.5, -3.5)], s);
    } else {
        p.line_segment([c + egui::vec2(-3.5, -3.5), c + egui::vec2(3.5, 3.5)], s);
        p.line_segment([c + egui::vec2(3.5, -3.5), c + egui::vec2(-3.5, 3.5)], s);
    }
}

/// Tam giác cảnh báo có dấu "!".
fn canh_bao(p: &egui::Painter, r: Rect, mau: Color32) {
    let (t, l, rr) = (egui::pos2(r.center().x, r.top() + 1.0), egui::pos2(r.left(), r.bottom() - 1.0), egui::pos2(r.right(), r.bottom() - 1.0));
    p.add(egui::Shape::closed_line(vec![t, rr, l], Stroke::new(1.3, mau)));
    p.line_segment([r.center() + egui::vec2(0.0, -2.0), r.center() + egui::vec2(0.0, 2.0)], Stroke::new(1.4, mau));
    p.circle_filled(r.center() + egui::vec2(0.0, 4.2), 0.9, mau);
}

/// Đầu bước: vòng tròn xanh số trắng + chữ đậm.
fn buoc(ui: &mut egui::Ui, so: u8, chu: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 9.0, VIEN_TAB_CHON);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, so.to_string(), egui::FontId::new(11.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), Color32::WHITE);
        ui.label(RichText::new(chu).size(12.5).dam());
    });
}

fn tieu_de_the(ui: &mut egui::Ui, chu: &str, phu: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(RichText::new(chu).size(12.5).dam());
        ui.label(RichText::new(phu).size(12.0).color(CHU_PHU));
    });
}

/// Dòng `nhãn | nội dung` với cột nhãn cố định.
fn dong(ui: &mut egui::Ui, nhan: &str, f: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(RONG_NHAN, 24.0), Sense::hover());
        ui.painter().text(egui::pos2(r.left(), r.center().y), egui::Align2::LEFT_CENTER, nhan, egui::FontId::proportional(12.0), CHU);
        ui.add_space(8.0);
        f(ui);
    });
}

fn goi_y(ui: &mut egui::Ui, chu: &str) {
    goi_y_mau(ui, chu, CHU_PHU);
}

fn goi_y_mau(ui: &mut egui::Ui, chu: &str, mau: Color32) {
    ui.horizontal(|ui| {
        ui.add_space(RONG_NHAN + 8.0);
        ui.add(egui::Label::new(RichText::new(chu).size(11.5).color(mau)).wrap());
    });
}

fn lien_ket(ui: &mut egui::Ui, chu: &str, mau: Color32) -> egui::Response {
    let p = ui.add(egui::Label::new(RichText::new(chu).size(12.5).color(mau)).sense(Sense::click()));
    if p.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    p
}

fn so(ui: &mut egui::Ui, v: &mut u32, (tu, den): (u32, u32)) {
    ui.add(egui::DragValue::new(v).range(tu..=den).speed(1.0)).on_hover_text(format!("{tu} – {den}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_dinh_giu_nha_cung_cap_va_sheet() {
        let mut c = CauHinh::default();
        c.boc_tach.mo_hinh_mut(NhaCungCap::NoiBo).model = "m".into();
        c.boc_tach.mo_hinh_mut(NhaCungCap::NoiBo).song_song = 8;
        c.boc_tach.dpi = 120;
        c.boc_tach.sheet_link = "https://docs.google.com/x".into();
        let m = mac_dinh(&c).boc_tach;
        assert_eq!((m.dpi, m.mo_hinh(NhaCungCap::NoiBo).song_song), (200, 1));
        assert_eq!(m.mo_hinh(NhaCungCap::NoiBo).model, "m");
        assert_eq!(m.sheet_link, "https://docs.google.com/x");
    }

    #[test]
    fn doc_email_service_account() {
        let d = std::env::temp_dir().join(format!("dema4_bt_sa_{}.json", std::process::id()));
        std::fs::write(&d, r#"{"type":"service_account","client_email":"dema4@du-an.iam.gserviceaccount.com"}"#).unwrap();
        assert_eq!(email_service_account(&d.display().to_string()).as_deref(), Some("dema4@du-an.iam.gserviceaccount.com"));
        std::fs::write(&d, "không phải json").unwrap();
        assert_eq!(email_service_account(&d.display().to_string()), None);
        let _ = std::fs::remove_file(&d);
    }
}
