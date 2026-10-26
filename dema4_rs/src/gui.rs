//! Giao diện đếm trang A4, tông xám như bản Python (dema4.py):
//! hàng chọn thư mục, hàng hệ số, dòng tóm tắt, bộ lọc theo khổ, bảng từng trang và thanh tiến độ.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke, TextStyle};
use egui_extras::{Column, TableBuilder};

use crate::dem_pdf::{self, KetQuaDem, SuKien, HE_SO_MAC_DINH, KHO};
use crate::telegram;

const TIEU_DE: &str = "Đếm số trang A4 - SaoMai";
/// Số sự kiện xử lý tối đa mỗi khung hình để giao diện không bị đơ khi đếm nhanh
const SU_KIEN_MOI_KHUNG: usize = 20_000;
const RONG_NUT: f32 = 80.0;
/// Dòng báo công khai ở góc dưới khi có cấu hình Telegram; chỉ đổi sang chữ đỏ khi gửi lỗi
const THONG_BAO_THONG_KE: &str = "Thông tin đếm thư mục đã được ghi lại";
/// Số cột của bảng chi tiết
const SO_COT: usize = 10;

// Bảng màu xám kiểu Windows cổ điển; chỉ nút Đếm/Dừng và chữ cảnh báo có màu
const NEN: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
const VIEN: Color32 = Color32::from_rgb(0xAD, 0xAD, 0xAD);
const VIEN_NHAT: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
const NUT: Color32 = Color32::from_rgb(0xE1, 0xE1, 0xE1);
const NUT_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
const CHU: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
const CHU_PHU: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);
const XANH_LA: Color32 = Color32::from_rgb(0x00, 0x80, 0x00);
const DO: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);
const DO_SAM: Color32 = Color32::from_rgb(0xA0, 0x10, 0x10);
const CHU_MO: Color32 = Color32::from_rgb(0xA8, 0xA8, 0xA8);
/// Màu phần đã chạy của thanh tiến độ (xanh nhạt cho dễ nhìn)
const XAM_TIEN_DO: Color32 = Color32::from_rgb(0xA8, 0xD1, 0xF5);
const XANH_COBALT: Color32 = Color32::from_rgb(0x00, 0x47, 0xAB);

pub fn chay() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([980.0, 600.0]).with_min_inner_size([800.0, 420.0]).with_title(TIEU_DE);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../icon.png")) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native(
        TIEU_DE,
        options,
        Box::new(|cc| {
            cai_giao_dien(&cc.egui_ctx);
            Ok(Box::new(UngDung::default()))
        }),
    )
}

/// Tên họ font đậm (egui không có chữ đậm sẵn, strong() chỉ đổi màu)
const HO_FONT_DAM: &str = "dam";

/// In đậm thật bằng họ font đậm của hệ thống
trait ChuDam {
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
}

struct DangDem {
    nhan: Receiver<SuKien>,
    dung: Arc<AtomicBool>,
    thu_muc: String,
    bat_dau: Instant,
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
    /// Hàng hệ số: đang sổ ra hay ẩn, và nội dung 8 ô A0..A7
    hien_he_so: bool,
    he_so: [String; 8],
    /// Hệ số đã dùng cho lần đếm gần nhất (để nhắc khi người dùng đổi hệ số sau đó)
    he_so_da_dung: Option<[f64; 8]>,
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
    /// Có file telegram.txt cạnh file chạy (đọc lúc mở app); quyết định có hiện dòng báo trong popup không
    co_telegram: bool,
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
            hien_he_so: false,
            he_so: HE_SO_MAC_DINH.map(dem_pdf::so_kieu_python),
            he_so_da_dung: None,
            thong_bao: None,
            thoi_gian_dem: None,
            file_cuoi: None,
            so_dong_da_cuon: 0,
            ket_qua_chep: None,
            da_chep_luc: None,
            kiem_tra_dpi: false,
            kiem_tra_ky: true,
            co_telegram: telegram::doc_cau_hinh().is_some(),
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
fn mo_bang_he_dieu_hanh(p: &Path) {
    let lenh = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(lenh).arg(p).spawn();
}

#[derive(Clone, Copy, PartialEq)]
enum CanLe {
    Trai,
    Giua,
    Phai,
}

const NEN_TIEU_DE: Color32 = Color32::from_rgb(0xEB, 0xEB, 0xEB);
const NEN_TAB: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xF7);
const NEN_TAB_CHON: Color32 = Color32::from_rgb(0xDC, 0xEB, 0xFA);
const VIEN_TAB_CHON: Color32 = Color32::from_rgb(0x2F, 0x7B, 0xD9);
const NEN_XEN_KE: Color32 = Color32::from_rgb(0xF6, 0xF6, 0xF6);
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

fn nut(chu: &str) -> egui::Button<'static> {
    egui::Button::new(chu.to_owned()).min_size(egui::vec2(RONG_NUT, 26.0))
}

fn nut_mau(chu: &str, nen: Color32) -> egui::Button<'static> {
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
        self.he_so_da_dung = None;
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
        let he_so = match self.he_so_hien_tai() {
            Ok(h) => h,
            Err(k) => {
                self.trang_thai = format!("Lỗi: hệ số {} không phải là số (dùng dấu chấm, ví dụ 0.5)", KHO[k]);
                self.hien_he_so = true;
                return;
            }
        };

        self.xoa_ket_qua();
        self.trang_thai = "Đang tìm file PDF…".into();

        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let (tm, dung_luong, ctx, kiem_tra_dpi, kiem_tra_ky) = (thu_muc.clone(), Arc::clone(&dung), ctx.clone(), self.kiem_tra_dpi, self.kiem_tra_ky);
        std::thread::spawn(move || {
            dem_pdf::dem_thu_muc(&tm, he_so, kiem_tra_dpi, kiem_tra_ky, &dung_luong, &gui);
            ctx.request_repaint();
        });
        self.he_so_da_dung = Some(he_so);
        self.dang_dem = Some(DangDem { nhan, dung, thu_muc, bat_dau: Instant::now() });
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

    /// Hệ số đang nhập trong 8 ô A0..A7; Err(k) nếu ô KHO[k] không phải là số
    fn he_so_hien_tai(&self) -> Result<[f64; 8], usize> {
        let mut he_so = [0.0; 8];
        for (k, s) in self.he_so.iter().enumerate() {
            he_so[k] = s.trim().parse::<f64>().map_err(|_| k)?;
        }
        Ok(he_so)
    }

    /// Quy ra A4 = Σ (số trang khổ Ax × hệ số Ax) theo hệ số đang nhập; None nếu có ô nhập sai
    fn quy_a4_hien_tai(&self) -> Option<f64> {
        let he_so = self.he_so_hien_tai().ok()?;
        Some((0..8).map(|k| self.thong_ke.theo_kho[k] as f64 * he_so[k]).sum())
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
            self.dang_dem = None;
            self.trang_thai = "Lỗi: luồng đếm dừng bất thường".into();
            return None;
        }
        let kq = xong?;
        let dd = self.dang_dem.take()?;
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
        // Gửi thống kê về Telegram ở luồng riêng (đọc lại cấu hình mỗi lần để sửa file không cần mở lại app).
        // Chạy ngầm, không hiện trạng thái gửi trên giao diện; popup có dòng báo app gửi thống kê.
        if let Some(cau_hinh) = telegram::doc_cau_hinh() {
            let tin = telegram::noi_dung_thong_ke(&xong.thu_muc, kq, xong.thoi_gian);
            std::thread::spawn(move || {
                let _ = telegram::gui_tin(&cau_hinh, &tin);
            });
            self.co_telegram = true;
        }
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
    fn hang_he_so(&mut self, ui: &mut egui::Ui) {
        if !self.hien_he_so {
            return;
        }
        ui.horizontal(|ui| {
            let cho_sua = self.dang_dem.is_none();
            let khe = ui.spacing().item_spacing.x;
            let rong_o = ((ui.available_width() - 8.0 * 26.0 - 15.0 * khe) / 8.0).clamp(44.0, 90.0);
            for k in 0..8 {
                let nhan = ui.label(format!("{}:", KHO[k]));
                let hop_le = self.he_so[k].trim().parse::<f64>().is_ok();
                let mau = if hop_le { CHU } else { DO };
                let o = egui::TextEdit::singleline(&mut self.he_so[k]).desired_width(rong_o).align(egui::Align2::CENTER_CENTER).text_color(mau).margin(Margin::symmetric(4, 3));
                ui.add_enabled(cho_sua, o).labelled_by(nhan.id);
            }
        });
    }

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

            let he_so_doi = matches!((self.he_so_da_dung, self.he_so_hien_tai()), (Some(cu), Ok(moi)) if cu != moi);
            if he_so_doi && self.dang_dem.is_none() {
                ui.add_space(14.0);
                ui.label(RichText::new("Hệ số đã đổi – bấm Đếm để ghi lại file kết quả").italics().color(CHU_PHU));
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
                // Liên kết "Hệ số" bên trái ô Check DPI: bấm để mở / thu hàng hệ số ngay bên dưới
                ui.add_space(10.0);
                let ui_lien_ket = ui.scope(|ui| lien_ket(ui, RichText::new("Hệ số"), self.hien_he_so, true));
                if ui_lien_ket.inner {
                    self.hien_he_so = !self.hien_he_so;
                }
                ui_lien_ket.response.on_hover_text("Hệ số nhân với số trang mỗi khổ để ra số trang quy A4");
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
                ui.allocate_ui_with_layout(ui.available_size(), Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    let goi_y = if self.dang_dem.is_some() {
                        "Đang đọc file…"
                    } else if !self.thu_muc.trim().is_empty() {
                        "Click nút Đếm để thực hiện"
                    } else {
                        "Chọn thư mục chứa PDF rồi bấm Đếm"
                    };
                    ui.label(RichText::new(goi_y).color(CHU_PHU));
                });
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
        let ctx = ui.ctx().clone();
        if let Some((kq, xong)) = self.nhan_su_kien() {
            self.luu_va_thong_bao(&kq, xong);
        }
        if self.dang_dem.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::bottom("trang_thai").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 4, bottom: 8 })).show(ui, |ui| self.thanh_trang_thai(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 12, right: 12, top: 10, bottom: 4 })).show(ui, |ui| {
            self.hang_thu_muc(ui);
            ui.add_space(2.0);
            self.dong_tom_tat(ui);
            self.hang_he_so(ui);
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
        app.he_so[3] = "2.5".into();
        assert_eq!(app.quy_a4_hien_tai(), Some(3.5));
        app.he_so[4] = "abc".into();
        assert_eq!(app.quy_a4_hien_tai(), None);
        assert_eq!(app.he_so_hien_tai(), Err(4));
    }

    /// Gõ vào ô hệ số qua giao diện thật (không mở cửa sổ): ô sửa được và Quy ra A4 đổi theo
    #[test]
    fn go_he_so_qua_giao_dien() {
        use egui_kittest::kittest::Queryable;
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(980.0, 600.0)).build_eframe(|cc| {
            cai_giao_dien(&cc.egui_ctx);
            let mut app = app_mau("1/1\t/a/x.pdf => trang: 1\t842.0\t1191.0\tDoc\tA3\t300\t\t-");
            app.hien_he_so = true;
            app
        });
        harness.run();
        harness.get_by_label("A3:").focus();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A); // chọn hết "2.0" để gõ đè
        harness.run();
        harness.get_by_label("A3:").type_text("5");
        harness.run();
        assert_eq!(harness.state().he_so[3], "5");
        assert_eq!(harness.state().quy_a4_hien_tai(), Some(5.0));
    }

    #[test]
    fn luong_dem_chet_khong_lam_sap_giao_dien() {
        let mut app = UngDung::default();
        let (gui, nhan) = mpsc::channel::<SuKien>();
        app.dang_dem = Some(DangDem { nhan, dung: Arc::new(AtomicBool::new(false)), thu_muc: "x".into(), bat_dau: Instant::now() });
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

    /// Vẽ giao diện ra ảnh PNG (không mở cửa sổ) từ một file chi tiết thật để xem bố cục.
    /// Chạy: DEMA4_MAU=<..._dem_chitiet.txt> DEMA4_ANH=<thư mục ảnh> cargo test -- --ignored chup_giao_dien
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
                app.dang_dem = Some(DangDem { nhan, dung: Arc::new(AtomicBool::new(false)), thu_muc: app.thu_muc.clone(), bat_dau: Instant::now() });
                app.tien_do = Some((48, 121));
            } else {
                app.tien_do = Some((121, 121));
                app.hien_he_so = bo_loc == BoLoc::TatCa && !popup;
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
