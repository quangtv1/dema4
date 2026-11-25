//! Trang chủ (màn mở mặc định): dải đầu nền xanh nhạt có lưới chấm (logo Sao Mai, tên, lời mời chọn chức năng),
//! 10 ô chức năng nhỏ cân trái, đầy hàng mới xuống hàng (logo nét mảnh + tên; rê chuột → viền xanh + bong bóng tóm tắt;
//! chức năng đang phát triển mờ, nhãn "Đang phát triển" trên mép ô), dưới là khối "Làm tiếp gần đây" (≤ 5 dòng) hoặc —
//! khi chưa có lịch sử — lưới 6 thông tin ngắn; thanh đáy: website · email | tên PC · MAC · IP · phiên bản.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind};

use crate::gan_day::{self, MucGanDay};
use crate::gui::{CHU, CHU_PHU, DO, NEN, VIEN, VIEN_NHAT, XANH_COBALT, XANH_LA};
use crate::gui_tach::bieu_tuong::{self, BieuTuong};
use crate::logo_chuc_nang::{self, Logo};

/// Chức năng người dùng bấm trên Trang chủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChonChucNang {
    Dem,
    Tach,
    MucLuc,
    BocTach,
    ChuanTen,
    Ky,
    Aip,
    XoaGhim,
}

/// Việc người dùng bấm trên Trang chủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChonTrangChu {
    ChucNang(ChonChucNang),
    /// Bấm dòng "Làm tiếp gần đây" thứ i.
    GanDay(usize),
    /// ✕ của dòng thứ i.
    XoaGanDay(usize),
    XoaHet,
    /// "Chọn lại…" của dòng có thư mục / file không còn.
    ChonLai(usize),
}

const TIEU_DE: &str = "Hỗ trợ nghiệp vụ số hóa tài liệu";
const CAO_DAU: f32 = 160.0;
const CAO_DAY: f32 = 30.0;
/// Số ô hàng đầu khi đủ chỗ; ô chia đều bề ngang khối (tên chức năng 1 dòng), không hẹp hơn `RONG_O_MIN`.
const O_MOI_HANG: usize = 7;
const RONG_O_MIN: f32 = 79.0;
const CAO_O: f32 = 84.0;
const KHE_O: f32 = 15.0;
const KHE_HANG: f32 = 15.0;
const CO_LOGO: f32 = 32.0;
/// Bề rộng chung của hàng ô chức năng, khối gần đây và lưới thông tin (mép trái thẳng nhau).
const RONG_KHOI: f32 = 780.0;
const CAO_DONG: f32 = 42.0;
const MAU_NET: Color32 = Color32::from_rgb(0x3C, 0x48, 0x58);
const NEN_DAU: Color32 = Color32::from_rgb(0xF3, 0xF6, 0xFB);
const CHAM_DAU: Color32 = Color32::from_rgb(0xD5, 0xDE, 0xEB);
const NEN_TOM: Color32 = Color32::from_rgb(0x2B, 0x34, 0x40);
const CAM: Color32 = Color32::from_rgb(0xD9, 0x82, 0x2B);

struct O {
    /// `None` = chức năng đang phát triển chưa có màn giới thiệu (bấm không mở).
    cn: Option<ChonChucNang>,
    logo: Logo,
    ten: &'static str,
    tom: &'static str,
    dang_phat_trien: bool,
}

const CAC_O: [O; 10] = [
    O { cn: Some(ChonChucNang::Dem), logo: Logo::Dem, ten: "Đếm trang A4", tom: "Đếm số trang mọi PDF trong thư mục (cả thư mục con), phân loại khổ A0–A7 và quy đổi ra số trang A4; cảnh báo trang sát ngưỡng, DPI bất thường, file đã ký số.", dang_phat_trien: false },
    O { cn: Some(ChonChucNang::Tach), logo: Logo::Tach, ten: "Tách văn bản", tom: "Tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã vạch), tự bỏ trang trắng, gợi ý trang bị xoay; người duyệt nhãn trước khi tách.", dang_phat_trien: false },
    O { cn: Some(ChonChucNang::MucLuc), logo: Logo::MucLuc, ten: "Sinh mục lục", tom: "Sinh mục lục văn bản cho từng hồ sơ từ file biên mục Excel/CSV theo mẫu DOCX của đơn vị; xem trước từng hồ sơ, xuất DOCX hàng loạt.", dang_phat_trien: false },
    O { cn: Some(ChonChucNang::BocTach), logo: Logo::BocTach, ten: "Bóc tách dữ liệu", tom: "Đọc văn bản PDF bằng AI (máy chủ nội bộ, ChatGPT, Gemini, Claude) để lấy số ký hiệu, ngày, trích yếu, cơ quan ban hành, người ký; ghi ra Excel và Google Sheet.", dang_phat_trien: false },
    O { cn: Some(ChonChucNang::XoaGhim), logo: Logo::XoaGhim, ten: "Xóa ghim", tom: "Xoá vết ghim / kẹp giấy / bóng mép trên ảnh scan theo vùng góc, cạnh hoặc vẽ tay; giữ nét chữ, xem trước trước khi xuất. Nên làm trước khi tách.", dang_phat_trien: false },
    O { cn: Some(ChonChucNang::ChuanTen), logo: Logo::ChuanTen, ten: "Chuẩn hoá tên", tom: "Đổi tên hàng loạt file văn bản theo quy tắc thống nhất (số ký hiệu, ngày, loại văn bản… lấy từ kết quả Bóc tách); xem trước trước khi đổi. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: Some(ChonChucNang::Ky), logo: Logo::Ky, ten: "Ký văn bản", tom: "Ký số hàng loạt văn bản PDF đã tách bằng chứng thư số của đơn vị, chèn hình chữ ký. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: Some(ChonChucNang::Aip), logo: Logo::Aip, ten: "Đóng gói AIP", tom: "Đóng gói hồ sơ lưu trữ thông tin số (AIP): gom văn bản PDF, siêu dữ liệu và mục lục thành gói chuẩn, kiểm tra toàn vẹn. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: None, logo: Logo::Scan, ten: "Scan tài liệu", tom: "Quét tài liệu giấy thành PDF ngay trong app. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: None, logo: Logo::NghiemThu, ten: "Nghiệm thu DA", tom: "Kiểm tra, nghiệm thu kết quả số hoá của dự án (số trang, chất lượng, đủ hồ sơ). Chức năng đang phát triển.", dang_phat_trien: true },
];

/// Thông tin máy cho thanh đáy — đọc 1 lần khi dựng Trang chủ (IP nội bộ: hỏi hệ điều hành, không gửi gói tin).
pub struct TrangChu {
    ip: String,
}

impl Default for TrangChu {
    fn default() -> Self {
        TrangChu { ip: crate::telegram::ip_noi_bo() }
    }
}

fn font(co: f32, dam: bool) -> egui::FontId {
    if dam { egui::FontId::new(co, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(co) }
}

impl TrangChu {
    /// IP cố định cho test / ảnh chụp tài liệu (không lộ IP máy chạy test).
    #[cfg(test)]
    pub fn thu(ip: &str) -> Self {
        TrangChu { ip: ip.into() }
    }

    /// Vẽ Trang chủ vào `ui` (đã có thanh menu phía trên); `gan` = lịch sử gần đây, `mat[i]` = đường dẫn dòng i không còn.
    pub fn ve(&self, ui: &mut egui::Ui, license: &crate::license::KiemTra, gan: &[MucGanDay], mat: &[bool]) -> Option<ChonTrangChu> {
        egui::Panel::bottom("tc_day").exact_size(CAO_DAY).frame(egui::Frame::new().fill(NEN)).show(ui, |ui| self.thanh_day(ui, license));
        let mut chon = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
            let vung = ui.max_rect();
            let dau = Rect::from_min_size(vung.min, egui::vec2(vung.width(), CAO_DAU));
            dai_dau(ui, dau);
            let y_o = dau.bottom() + 30.0;
            let rong = RONG_KHOI.min(vung.width() - 40.0);
            let x0 = vung.center().x - rong / 2.0;
            let mut tom = None;
            let mut day = y_o;
            for (i, (o, r)) in CAC_O.iter().zip(vi_tri_o(x0, y_o, rong)).enumerate() {
                day = r.bottom();
                let ph = ui.interact(r, ui.scope_id().with(("tc_o", i)), if o.cn.is_some() { Sense::click() } else { Sense::hover() });
                ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, o.cn.is_some(), o.ten));
                let re = ph.hovered() || ph.has_focus();
                o_chuc_nang(ui, r, o, re);
                if re {
                    tom = Some((r, o.tom));
                }
                if let Some(cn) = o.cn {
                    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        chon = Some(ChonTrangChu::ChucNang(cn));
                    }
                }
            }
            if gan.is_empty() {
                luoi_thong_tin(ui, x0, rong, day + 30.0, license);
            } else if let Some(c) = khoi_gan_day(ui, x0, rong, day + 30.0, gan, mat) {
                chon = Some(c);
            }
            if let Some((r, chu)) = tom {
                bong_bong(ui, r, chu);
            }
        });
        chon
    }

    fn thanh_day(&self, ui: &mut egui::Ui, license: &crate::license::KiemTra) {
        let r = ui.max_rect();
        ui.painter().hline(r.x_range(), r.top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
        let f = font(12.0, false);
        // Các mục ngăn bằng vạch dọc mảnh: trái = liên hệ (bấm mở web / thư), phải = thông tin máy
        let ve_cum = |ui: &egui::Ui, muc: &[(&str, String, Option<String>)], tu_phai: bool| {
            let p = ui.painter();
            let mut gal: Vec<_> = muc.iter().map(|(nhan, gt, _)| (p.layout_no_wrap(format!("{nhan} "), f.clone(), CHU_PHU), p.layout_no_wrap(gt.clone(), f.clone(), CHU))).collect();
            let rong: Vec<f32> = gal.iter().map(|(a, b)| a.size().x + b.size().x).collect();
            let tong = rong.iter().sum::<f32>() + 25.0 * (muc.len() as f32 - 1.0);
            let mut x = if tu_phai { r.right() - 28.0 - tong } else { r.left() + 28.0 };
            for (i, ((a, b), w)) in gal.drain(..).zip(&rong).enumerate() {
                if i > 0 {
                    p.vline(x + 12.0, egui::Rangef::new(r.center().y - 6.0, r.center().y + 6.0), Stroke::new(1.0, Color32::from_gray(0xD5)));
                    x += 25.0;
                }
                let y = r.center().y - a.size().y / 2.0;
                let wa = a.size().x;
                let o_gt = Rect::from_min_size(egui::pos2(x + wa, y), b.size());
                p.galley(egui::pos2(x, y), a, CHU_PHU);
                p.galley(o_gt.min, b, CHU);
                if let Some(url) = &muc[i].2 {
                    let ph = ui.interact(o_gt, ui.scope_id().with(("tc_lk", i)), Sense::click());
                    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &muc[i].1));
                    if ph.hovered() {
                        p.hline(o_gt.x_range(), o_gt.bottom() - 1.0, Stroke::new(1.0, XANH_COBALT));
                    }
                    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                }
                x += w;
            }
        };
        let web = crate::tro_giup::WEBSITE;
        let email = crate::hop_license::EMAIL;
        ve_cum(ui, &[("Website:", web.trim_start_matches("https://").into(), Some(web.into())), ("Email:", email.into(), Some(format!("mailto:{email}")))], false);
        ve_cum(
            ui,
            &[("Tên PC:", license.ten_pc().into(), None), ("MAC:", license.mac().to_lowercase(), None), ("IP:", self.ip.clone(), None), ("Phiên bản:", crate::tro_giup::PHIEN_BAN.into(), None)],
            true,
        );
    }
}

/// Dải đầu: nền xanh rất nhạt + lưới chấm mờ, viền dưới; logo Sao Mai giữa, tên, lời mời chọn chức năng.
fn dai_dau(ui: &egui::Ui, r: Rect) {
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, NEN_DAU);
    let mut y = r.top() + 9.0;
    while y < r.bottom() {
        let mut x = r.left() + 9.0;
        while x < r.right() {
            p.circle_filled(egui::pos2(x, y), 0.9, CHAM_DAU);
            x += 18.0;
        }
        y += 18.0;
    }
    p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(0xE1, 0xE7, 0xF0)));
    let kich = crate::logo_sao_mai::kich(60.0);
    let ten = p.layout_no_wrap(TIEU_DE.into(), font(19.0, true), CHU);
    let moi = p.layout_no_wrap("Chọn chức năng để bắt đầu".into(), font(12.5, false), CHU_PHU);
    let cao = kich.y + 8.0 + ten.size().y + 4.0 + moi.size().y;
    let mut y = r.center().y - cao / 2.0;
    let o_logo = Rect::from_center_size(egui::pos2(r.center().x, y + kich.y / 2.0), kich);
    crate::logo_sao_mai::ve(&p, o_logo);
    let ph = ui.interact(o_logo, ui.scope_id().with("tc_logo"), Sense::click()).on_hover_text("saomaiso.com");
    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(crate::tro_giup::WEBSITE));
    }
    y += kich.y + 8.0;
    p.galley(egui::pos2(r.center().x - ten.size().x / 2.0, y), ten.clone(), CHU);
    y += ten.size().y + 4.0;
    p.galley(egui::pos2(r.center().x - moi.size().x / 2.0, y), moi, CHU_PHU);
}

/// Vị trí 10 ô: cân trái từ `x0`; đủ chỗ thì hàng đầu 7 ô chia đều bề rộng `rong` (ô rộng → tên 1 dòng), hẹp thì ô
/// `RONG_O_MIN`, đầy hàng mới xuống hàng.
fn vi_tri_o(x0: f32, y0: f32, rong: f32) -> Vec<Rect> {
    let rong_o = ((rong - (O_MOI_HANG - 1) as f32 * KHE_O) / O_MOI_HANG as f32).max(RONG_O_MIN);
    let moi_hang = (((rong + KHE_O) / (rong_o + KHE_O)).floor() as usize).max(1);
    (0..CAC_O.len()).map(|i| Rect::from_min_size(egui::pos2(x0 + (i % moi_hang) as f32 * (rong_o + KHE_O), y0 + (i / moi_hang) as f32 * (CAO_O + KHE_HANG)), egui::vec2(rong_o, CAO_O))).collect()
}

/// Chia tên ô thành dòng vừa `rong` (tên dài xuống 2 dòng).
fn dong_ten(p: &egui::Painter, ten: &str, f: &egui::FontId, rong: f32) -> Vec<String> {
    let mut dong: Vec<String> = Vec::new();
    for tu in ten.split(' ') {
        match dong.last_mut() {
            Some(d) if p.layout_no_wrap(format!("{d} {tu}"), f.clone(), CHU).size().x <= rong => *d = format!("{d} {tu}"),
            _ => dong.push(tu.into()),
        }
    }
    dong
}

/// 1 ô chức năng: viền bo góc, logo nét mảnh + tên; rê chuột → nổi lên 2px, viền + logo xanh, bóng mờ.
fn o_chuc_nang(ui: &egui::Ui, r: Rect, o: &O, re: bool) {
    let p = ui.painter();
    let r = if re { r.translate(egui::vec2(0.0, -2.0)) } else { r };
    if re {
        p.rect_filled(r.translate(egui::vec2(0.0, 4.0)).expand(1.0), 6.0, Color32::from_rgba_unmultiplied(0, 71, 171, 18));
    }
    p.rect(r, 5.0, Color32::WHITE, Stroke::new(1.0, if re { XANH_COBALT } else { VIEN }), StrokeKind::Inside);
    let mo = |c: Color32| if o.dang_phat_trien { c.gamma_multiply(0.4) } else { c };
    let f = font(11.0, true);
    let dong = dong_ten(p, o.ten, &f, r.width() - 6.0);
    let gal: Vec<_> = dong.into_iter().map(|d| p.layout_no_wrap(d, f.clone(), mo(CHU))).collect();
    let cao_chu: f32 = gal.iter().map(|g| g.size().y).sum();
    let mut y = r.center().y - (CO_LOGO + 5.0 + cao_chu) / 2.0;
    logo_chuc_nang::ve_mau(p, Rect::from_min_size(egui::pos2(r.center().x - CO_LOGO / 2.0, y), egui::vec2(CO_LOGO, CO_LOGO)), o.logo, mo(if re { XANH_COBALT } else { MAU_NET }));
    y += CO_LOGO + 5.0;
    for g in gal {
        let h = g.size().y;
        p.galley(egui::pos2(r.center().x - g.size().x / 2.0, y), g, mo(CHU));
        y += h;
    }
    if o.dang_phat_trien {
        // Nhãn nằm trên mép trên ô
        let g = p.layout_no_wrap("Đang phát triển".into(), font(8.5, true), Color32::WHITE);
        let n = Rect::from_center_size(egui::pos2(r.center().x, r.top()), g.size() + egui::vec2(12.0, 2.0));
        p.rect_filled(n, n.height() / 2.0, CAM);
        p.galley(n.center() - g.size() / 2.0, g, Color32::WHITE);
    }
}

/// Chữ một dòng cắt cho vừa `rong`: `dau` = cắt đầu (giữ đuôi đường dẫn), không thì cắt cuối; thêm "…".
fn cat(p: &egui::Painter, chu: &str, f: egui::FontId, mau: Color32, rong: f32, dau: bool) -> std::sync::Arc<egui::Galley> {
    let g = p.layout_no_wrap(chu.into(), f.clone(), mau);
    if g.size().x <= rong {
        return g;
    }
    let ky: Vec<char> = chu.chars().collect();
    for bo in 1..ky.len() {
        let s: String = if dau { std::iter::once('…').chain(ky[bo..].iter().copied()).collect() } else { ky[..ky.len() - bo].iter().copied().chain(std::iter::once('…')).collect() };
        let g = p.layout_no_wrap(s, f.clone(), mau);
        if g.size().x <= rong {
            return g;
        }
    }
    p.layout_no_wrap("…".into(), f, mau)
}

fn logo_cn(cn: gan_day::ChucNang) -> Logo {
    match cn {
        gan_day::ChucNang::Dem => Logo::Dem,
        gan_day::ChucNang::Tach => Logo::Tach,
        gan_day::ChucNang::MucLuc => Logo::MucLuc,
        gan_day::ChucNang::BocTach => Logo::BocTach,
    }
}

/// Khối "Làm tiếp gần đây": kẻ trên, tiêu đề + số dòng + "Xoá lịch sử"; mỗi dòng: biểu tượng chức năng · tên (đậm)
/// + loại / mẫu · thư mục cha (mờ, cắt đầu) · tình trạng (chữ + thanh tiến độ) · thời gian · ✕ khi rê chuột.
fn khoi_gan_day(ui: &egui::Ui, x0: f32, rong: f32, y: f32, gan: &[MucGanDay], mat: &[bool]) -> Option<ChonTrangChu> {
    let p = ui.painter();
    let mut chon = None;
    p.hline(egui::Rangef::new(x0, x0 + rong), y, Stroke::new(1.0, VIEN_NHAT));
    let tieu_de = p.layout_no_wrap("Làm tiếp gần đây".into(), font(13.5, true), CHU);
    let y_td = y + 12.0;
    let h_td = tieu_de.size().y;
    let x_dem = x0 + 2.0 + tieu_de.size().x + 10.0;
    p.galley(egui::pos2(x0 + 2.0, y_td), tieu_de, CHU);
    let dem = p.layout_no_wrap(format!("{} mục", gan.len()), font(12.0, false), CHU_PHU);
    p.galley(egui::pos2(x_dem, y_td + (h_td - dem.size().y) / 2.0), dem, CHU_PHU);
    let xoa = p.layout_no_wrap("Xoá lịch sử".into(), font(12.0, false), XANH_COBALT);
    let o_xoa = Rect::from_min_size(egui::pos2(x0 + rong - 2.0 - xoa.size().x, y_td + (h_td - xoa.size().y) / 2.0), xoa.size());
    let ph = ui.interact(o_xoa, ui.scope_id().with("tc_xoa_het"), Sense::click());
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Xoá lịch sử"));
    if ph.hovered() {
        p.hline(o_xoa.x_range(), o_xoa.bottom(), Stroke::new(1.0, XANH_COBALT));
    }
    p.galley(o_xoa.min, xoa, XANH_COBALT);
    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        chon = Some(ChonTrangChu::XoaHet);
    }
    // Cột: biểu tượng 30 · tên (co giãn) · tình trạng 270 · thời gian 84 · ✕ 22, khe 12, đệm 8
    const KHE: f32 = 12.0;
    let rong_ten = (rong - 16.0 - 30.0 - 270.0 - 84.0 - 22.0 - 4.0 * KHE).max(80.0);
    let bay_gio = gan_day::bay_gio_ms();
    let mut y_dong = y_td + h_td + 8.0;
    for (i, m) in gan.iter().enumerate() {
        let mat_tm = mat.get(i).copied().unwrap_or(false);
        let r = Rect::from_min_size(egui::pos2(x0, y_dong), egui::vec2(rong, CAO_DONG));
        y_dong += CAO_DONG;
        let ph = ui.interact(r, ui.scope_id().with(("tc_gd", i)), Sense::click());
        ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, !mat_tm, m.ten()));
        let re = ph.hovered();
        if re {
            p.rect_filled(r, 5.0, Color32::from_rgb(0xEE, 0xF4, 0xFC));
        } else if i > 0 {
            p.hline(egui::Rangef::new(r.left() + 8.0, r.right() - 8.0), r.top(), Stroke::new(1.0, Color32::from_rgb(0xEE, 0xF0, 0xF2)));
        }
        let mo = |c: Color32| if mat_tm { c.gamma_multiply(0.45) } else { c };
        let mut x = r.left() + 8.0;
        let chip = Rect::from_min_size(egui::pos2(x, r.center().y - 15.0), egui::vec2(30.0, 30.0));
        p.rect(chip, 6.0, mo(Color32::from_rgb(0xF3, 0xF6, 0xFB)), Stroke::new(1.0, mo(Color32::from_rgb(0xE1, 0xE7, 0xF0))), StrokeKind::Inside);
        logo_chuc_nang::ve_mau(p, Rect::from_center_size(chip.center(), egui::vec2(19.0, 19.0)), logo_cn(m.cn), mo(MAU_NET));
        x += 30.0 + KHE;
        // Tên đậm + loại / mẫu nhỏ, dưới là thư mục cha
        let mau_ten = mo(if re && !mat_tm { XANH_COBALT } else { CHU });
        let ten = cat(p, &m.ten(), font(13.0, true), mau_ten, rong_ten, false);
        let (wt, ht) = (ten.size().x, ten.size().y);
        let duong = cat(p, &m.cha(), font(11.5, false), mo(CHU_PHU), rong_ten, true);
        let y_ten = r.center().y - (ht + 2.0 + duong.size().y) / 2.0;
        p.galley(egui::pos2(x, y_ten), ten, mau_ten);
        if let Some(phu) = m.loai.clone().or_else(|| m.mau.as_deref().map(crate::gui_muc_luc::ten_mau)) {
            let con = rong_ten - wt - 6.0;
            if con > 20.0 {
                let g = cat(p, &phu, font(11.0, false), mo(CHU_PHU), con, false);
                p.galley(egui::pos2(x + wt + 6.0, y_ten + ht - g.size().y - 1.0), g, mo(CHU_PHU));
            }
        }
        p.galley(egui::pos2(x, y_ten + ht + 2.0), duong, mo(CHU_PHU));
        x += rong_ten + KHE;
        // Tình trạng: thư mục không còn → chữ đỏ + "Chọn lại…"; không thì chữ + thanh tiến độ
        if mat_tm {
            let a = p.layout_no_wrap(format!("Không tìm thấy {} · ", if m.cn == gan_day::ChucNang::MucLuc { "file" } else { "thư mục" }), font(12.0, false), DO);
            let b = p.layout_no_wrap("Chọn lại…".into(), font(12.0, false), XANH_COBALT);
            let y_c = r.center().y - a.size().y / 2.0;
            let o_lai = Rect::from_min_size(egui::pos2(x + a.size().x, y_c), b.size());
            p.galley(egui::pos2(x, y_c), a, DO);
            p.hline(o_lai.x_range(), o_lai.bottom(), Stroke::new(1.0, XANH_COBALT));
            p.galley(o_lai.min, b, XANH_COBALT);
            let ph = ui.interact(o_lai, ui.scope_id().with(("tc_gd_lai", i)), Sense::click());
            ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Chọn lại…"));
            if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                chon = Some(ChonTrangChu::ChonLai(i));
            }
        } else {
            let t = &m.tinh;
            let g = cat(p, &t.chu, font(12.0, false), Color32::from_gray(0x33), 270.0, false);
            let co_thanh = t.tong > 0;
            let cao = g.size().y + if co_thanh { 9.0 } else { 0.0 };
            let y_c = r.center().y - cao / 2.0;
            let hg = g.size().y;
            p.galley(egui::pos2(x, y_c), g, Color32::from_gray(0x33));
            if co_thanh {
                let thanh = Rect::from_min_size(egui::pos2(x, y_c + hg + 4.0), egui::vec2(270.0, 5.0));
                p.rect_filled(thanh, 2.5, Color32::from_rgb(0xE6, 0xE9, 0xED));
                let ty_le = (t.xong as f32 / t.tong as f32).clamp(0.0, 1.0);
                if ty_le > 0.0 {
                    p.rect_filled(Rect::from_min_size(thanh.min, egui::vec2(270.0 * ty_le, 5.0)), 2.5, if t.loi { CAM } else { XANH_LA });
                }
            }
        }
        x += 270.0 + KHE;
        let luc = p.layout_no_wrap(gan_day::thoi_gian_tuong_doi(m.luc, bay_gio), font(12.0, false), CHU_PHU);
        p.galley(egui::pos2(x + 84.0 - luc.size().x, r.center().y - luc.size().y / 2.0), luc, CHU_PHU);
        x += 84.0 + KHE;
        // ✕ chỉ hiện khi rê dòng (vẽ sau dòng → nhận bấm trước dòng)
        let o_x = Rect::from_center_size(egui::pos2(x + 11.0, r.center().y), egui::vec2(22.0, 22.0));
        let px = ui.interact(o_x, ui.scope_id().with(("tc_gd_xoa", i)), Sense::click());
        px.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Bỏ {} khỏi danh sách", m.ten())));
        if re || px.hovered() {
            if px.hovered() {
                p.rect_filled(o_x, 3.0, Color32::from_rgb(0xDD, 0xE3, 0xEA));
            }
            let net = Stroke::new(1.4, if px.hovered() { CHU } else { Color32::from_gray(0x88) });
            let (c, k) = (o_x.center(), 4.5);
            p.line_segment([c + egui::vec2(-k, -k), c + egui::vec2(k, k)], net);
            p.line_segment([c + egui::vec2(k, -k), c + egui::vec2(-k, k)], net);
        }
        if px.clicked() {
            chon = Some(ChonTrangChu::XoaGanDay(i));
        } else if !mat_tm && ph.clone().on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&m.duong_dan).clicked() {
            chon = chon.or(Some(ChonTrangChu::GanDay(i)));
        } else if mat_tm {
            ph.on_hover_text(&m.duong_dan);
        }
    }
    chon
}

/// Bong bóng tóm tắt dưới ô đang rê chuột (lớp nổi, có mũi nhọn chỉ lên ô).
fn bong_bong(ui: &egui::Ui, o: Rect, chu: &str) {
    let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("tc_bong_bong")));
    let mut job = egui::text::LayoutJob::single_section(chu.into(), egui::TextFormat { font_id: font(12.0, false), color: Color32::WHITE, line_height: Some(18.0), ..Default::default() });
    job.wrap.max_width = 212.0;
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let r = Rect::from_min_size(egui::pos2(o.center().x - 118.0, o.bottom() + 12.0), egui::vec2(236.0, g.size().y + 18.0));
    p.rect_filled(r, 5.0, NEN_TOM);
    let c = egui::pos2(o.center().x, r.top());
    p.add(egui::Shape::convex_polygon(vec![c + egui::vec2(-6.0, 0.0), c + egui::vec2(0.0, -6.0), c + egui::vec2(6.0, 0.0)], NEN_TOM, Stroke::NONE));
    p.galley(r.min + egui::vec2(12.0, 9.0), g, Color32::WHITE);
}

/// Lưới 2 × 3 thông tin ngắn dưới các ô (mỗi cột rộng theo chữ, cả cụm căn giữa), kẻ mảnh phía trên.
fn luoi_thong_tin(ui: &egui::Ui, x0: f32, rong_ke: f32, y: f32, license: &crate::license::KiemTra) {
    let p = ui.painter();
    let giua = x0 + rong_ke / 2.0;
    p.hline(egui::Rangef::new(x0, x0 + rong_ke), y, Stroke::new(1.0, VIEN_NHAT));
    let luot = match license.luot() {
        Some((dung, han)) => format!("Lượt chạy {dung} / {han}"),
        None => "Lượt chạy".into(),
    };
    let muc: [(BieuTuong, String, &str); 6] = [
        (BieuTuong::ManHinh, "Xử lý ngay trên máy".into(), "Riêng Bóc tách gửi ảnh tới mô hình đọc đã chọn"),
        (BieuTuong::Khien, "Giữ nguyên file gốc".into(), "Kết quả ghi ra thư mục riêng"),
        (BieuTuong::DongHo, luot, "Dùng chung mọi chức năng"),
        (BieuTuong::TaiLieu, "Định dạng hỗ trợ".into(), "PDF scan · Excel · CSV · DOCX"),
        (BieuTuong::HoiDap, "Hướng dẫn sử dụng".into(), "Nhấn F1 hoặc Trợ giúp › Hướng dẫn"),
        (BieuTuong::DongLenh, "Chạy hàng loạt".into(), "Đếm / Tách bằng dòng lệnh"),
    ];
    let gal: Vec<_> = muc.iter().map(|(_, a, b)| (p.layout_no_wrap(a.clone(), font(12.5, true), CHU), p.layout_no_wrap((*b).into(), font(12.0, false), CHU_PHU))).collect();
    const ICON: f32 = 22.0;
    const KHE_ICON: f32 = 10.0;
    let rong_cot: Vec<f32> = (0..3).map(|c| [c, c + 3].iter().map(|&i| ICON + KHE_ICON + gal[i].0.size().x.max(gal[i].1.size().x)).fold(0.0, f32::max)).collect();
    // Cột trái thẳng mép trái đường kẻ, cột phải thẳng mép phải: khe giữa các cột chia đều phần còn lại
    let khe_cot = ((rong_ke - rong_cot.iter().sum::<f32>()) / 2.0).max(16.0);
    let tong = rong_cot.iter().sum::<f32>() + 2.0 * khe_cot;
    for (i, ((a, b), (icon, _, _))) in gal.into_iter().zip(muc.iter()).enumerate() {
        let (cot, hang) = (i % 3, i / 3);
        let x = giua - tong / 2.0 + rong_cot[..cot].iter().sum::<f32>() + cot as f32 * khe_cot;
        let cao = a.size().y + 2.0 + b.size().y;
        let y_giua = y + 22.0 + hang as f32 * 54.0 + 17.0;
        bieu_tuong::ve(p, Rect::from_center_size(egui::pos2(x + ICON / 2.0, y_giua), egui::vec2(ICON, ICON)), *icon, MAU_NET);
        let y_chu = y_giua - cao / 2.0;
        let x_chu = x + ICON + KHE_ICON;
        p.galley(egui::pos2(x_chu, y_chu + a.size().y + 2.0), b, CHU_PHU);
        p.galley(egui::pos2(x_chu, y_chu), a, CHU);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::gan_day::{ChucNang, MucGanDay, TinhTrang};

    #[test]
    fn o_can_trai_day_hang_moi_xuong_hang() {
        let o = vi_tri_o(100.0, 50.0, RONG_KHOI);
        assert_eq!(o.len(), 10);
        assert!(o[..7].iter().all(|r| r.top() == 50.0) && o[7..].iter().all(|r| r.top() > 50.0), "780 px: 7 ô hàng 1, 3 ô hàng 2");
        assert_eq!((o[0].left(), o[7].left()), (100.0, 100.0), "mép trái ô đầu mỗi hàng = mép trái khối dưới");
        assert!((o[6].right() - (100.0 + RONG_KHOI)).abs() < 0.01, "hàng đầu vừa khít bề ngang khối");
        assert!(o[0].width() > 95.0);
        let hep = vi_tri_o(0.0, 0.0, 500.0);
        assert_eq!(hep.iter().filter(|r| r.top() == 0.0).count(), 5, "hẹp hơn → ô cỡ tối thiểu, ít ô mỗi hàng, vẫn theo thứ tự");
    }

    fn harness(gan: Vec<MucGanDay>, mat: Vec<bool>) -> (egui_kittest::Harness<'static>, std::rc::Rc<std::cell::Cell<Option<ChonTrangChu>>>) {
        let kt = crate::license::KiemTra::gia(std::sync::Arc::new(crate::tram::KhoGia::moi(10)));
        let tc = TrangChu { ip: "192.168.1.5".into() };
        let chon = std::rc::Rc::new(std::cell::Cell::new(None));
        let c2 = chon.clone();
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 742.0)).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            if let Some(c) = tc.ve(ui, &kt, &gan, &mat) {
                c2.set(Some(c));
            }
        });
        // Logo Sao Mai động luôn xin vẽ lại → chạy số bước cố định thay vì chờ giao diện đứng yên
        h.run_steps(3);
        (h, chon)
    }

    /// Chụp Trang chủ ra PNG để xem bằng mắt: `DEMA4_ANH=<file.png> cargo test chup_trang_chu -- --ignored`.
    #[test]
    #[ignore]
    fn chup_trang_chu() {
        let Some(f) = std::env::var_os("DEMA4_ANH") else { return };
        let (mut h, _) = harness(vec![], vec![]);
        h.render().unwrap().save(f).unwrap();
    }

    #[test]
    fn bam_o_tra_dung_chuc_nang() {
        use egui_kittest::kittest::Queryable;
        let (mut h, chon) = harness(vec![], vec![]);
        for (ten, cn) in [("Đếm trang A4", ChonChucNang::Dem), ("Tách văn bản", ChonChucNang::Tach), ("Sinh mục lục", ChonChucNang::MucLuc), ("Bóc tách dữ liệu", ChonChucNang::BocTach), ("Chuẩn hoá tên", ChonChucNang::ChuanTen), ("Ký văn bản", ChonChucNang::Ky), ("Đóng gói AIP", ChonChucNang::Aip), ("Xóa ghim", ChonChucNang::XoaGhim)] {
            h.get_by_label(ten).click();
            h.run_steps(3);
            assert_eq!(chon.take(), Some(ChonTrangChu::ChucNang(cn)), "{ten}");
        }
        for ten in ["Scan tài liệu", "Nghiệm thu DA"] {
            h.get_by_label(ten).click();
            h.run_steps(3);
            assert_eq!(chon.take(), None, "{ten}: đang phát triển, bấm không mở");
        }
        assert!(h.query_by_label("Xoá lịch sử").is_none(), "chưa có lịch sử → lưới 6 ô thông tin, không có khối gần đây");
        assert!(h.get_all_by_label("saomaiso.com").next().is_some(), "link website ở thanh đáy");
        h.get_by_label(crate::hop_license::EMAIL);
    }

    #[test]
    fn khoi_gan_day_bam_dong_xoa_chon_lai() {
        use egui_kittest::kittest::Queryable;
        let mut bt = MucGanDay::moi(ChucNang::BocTach, "/a/HS_Boc", TinhTrang { chu: "60/100 file · 2 lỗi".into(), xong: 62, tong: 100, loi: true });
        bt.loai = Some("Văn bản".into());
        let gan = vec![bt, MucGanDay::moi(ChucNang::Tach, "/a/HS_Tach", TinhTrang { chu: "42/120 hồ sơ đã duyệt".into(), xong: 42, tong: 120, loi: false }), MucGanDay::moi(ChucNang::Dem, "/mat/HS_Dem", TinhTrang { chu: "Chưa đếm".into(), ..Default::default() })];
        let (mut h, chon) = harness(gan, vec![false, false, true]);
        for (ten, i) in [("HS_Boc", 0), ("HS_Tach", 1)] {
            h.get_by_label(ten).click();
            h.run_steps(3);
            assert_eq!(chon.take(), Some(ChonTrangChu::GanDay(i)), "{ten}");
        }
        h.get_by_label("HS_Dem").click();
        h.run_steps(3);
        assert_eq!(chon.take(), None, "thư mục không còn → bấm dòng không mở");
        h.get_by_label("Chọn lại…").click();
        h.run_steps(3);
        assert_eq!(chon.take(), Some(ChonTrangChu::ChonLai(2)));
        h.get_by_label("Bỏ HS_Tach khỏi danh sách").click();
        h.run_steps(3);
        assert_eq!(chon.take(), Some(ChonTrangChu::XoaGanDay(1)));
        h.get_by_label("Xoá lịch sử").click();
        h.run_steps(3);
        assert_eq!(chon.take(), Some(ChonTrangChu::XoaHet));
        // 5 dòng vừa cửa sổ 1200×800 (vùng giữa 742 px) không cần cuộn
        let r = h.get_by_label("Xoá lịch sử").rect();
        assert!(r.top() + 30.0 + 5.0 * CAO_DONG < 742.0 - CAO_DAY, "khối gần đây 5 dòng nằm trong màn: đầu khối y = {}", r.top());
    }
}
