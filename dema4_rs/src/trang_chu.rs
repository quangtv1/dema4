//! Trang chủ (màn mở mặc định): dải đầu nền xanh nhạt có lưới chấm (logo Sao Mai, tên, lời mời chọn chức năng),
//! 7 ô chức năng, mỗi hàng 4 (logo nét mảnh + tên; rê chuột → viền xanh + bong bóng tóm tắt; chức năng đang phát triển mờ, nhãn "Đang phát triển"),
//! lưới 6 thông tin ngắn, thanh đáy: website · email | tên PC · MAC · IP · phiên bản.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind};

use crate::gui::{CHU, CHU_PHU, NEN, VIEN, VIEN_NHAT, XANH_COBALT};
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
}

const TIEU_DE: &str = "Hỗ trợ nghiệp vụ số hóa tài liệu";
const CAO_DAU: f32 = 160.0;
const CAO_DAY: f32 = 30.0;
const RONG_O: f32 = 132.0;
const CAO_O: f32 = 128.0;
const KHE_O: f32 = 22.0;
const KHE_HANG: f32 = 18.0;
const O_MOI_HANG: usize = 4;
const CO_LOGO: f32 = 46.0;
const MAU_NET: Color32 = Color32::from_rgb(0x3C, 0x48, 0x58);
const NEN_DAU: Color32 = Color32::from_rgb(0xF3, 0xF6, 0xFB);
const CHAM_DAU: Color32 = Color32::from_rgb(0xD5, 0xDE, 0xEB);
const NEN_TOM: Color32 = Color32::from_rgb(0x2B, 0x34, 0x40);
const CAM: Color32 = Color32::from_rgb(0xD9, 0x82, 0x2B);

struct O {
    cn: ChonChucNang,
    logo: Logo,
    ten: &'static str,
    tom: &'static str,
    dang_phat_trien: bool,
}

const CAC_O: [O; 7] = [
    O { cn: ChonChucNang::Dem, logo: Logo::Dem, ten: "Đếm trang A4", tom: "Đếm số trang mọi PDF trong thư mục (cả thư mục con), phân loại khổ A0–A7 và quy đổi ra số trang A4; cảnh báo trang sát ngưỡng, DPI bất thường, file đã ký số.", dang_phat_trien: false },
    O { cn: ChonChucNang::Tach, logo: Logo::Tach, ten: "Tách văn bản", tom: "Tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã vạch), tự bỏ trang trắng, gợi ý trang bị xoay; người duyệt nhãn trước khi tách.", dang_phat_trien: false },
    O { cn: ChonChucNang::MucLuc, logo: Logo::MucLuc, ten: "Sinh mục lục", tom: "Sinh mục lục văn bản cho từng hồ sơ từ file biên mục Excel/CSV theo mẫu DOCX của đơn vị; xem trước từng hồ sơ, xuất DOCX hàng loạt.", dang_phat_trien: false },
    O { cn: ChonChucNang::BocTach, logo: Logo::BocTach, ten: "Bóc tách dữ liệu", tom: "Đọc văn bản PDF bằng AI (máy chủ nội bộ, ChatGPT, Gemini, Claude) để lấy số ký hiệu, ngày, trích yếu, cơ quan ban hành, người ký; ghi ra Excel và Google Sheet. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: ChonChucNang::ChuanTen, logo: Logo::ChuanTen, ten: "Chuẩn hoá tên", tom: "Đổi tên hàng loạt file văn bản theo quy tắc thống nhất (số ký hiệu, ngày, loại văn bản… lấy từ kết quả Bóc tách); xem trước trước khi đổi. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: ChonChucNang::Ky, logo: Logo::Ky, ten: "Ký văn bản", tom: "Ký số hàng loạt văn bản PDF đã tách bằng chứng thư số của đơn vị, chèn hình chữ ký. Chức năng đang phát triển.", dang_phat_trien: true },
    O { cn: ChonChucNang::Aip, logo: Logo::Aip, ten: "Đóng gói AIP", tom: "Đóng gói hồ sơ lưu trữ thông tin số (AIP): gom văn bản PDF, siêu dữ liệu và mục lục thành gói chuẩn, kiểm tra toàn vẹn. Chức năng đang phát triển.", dang_phat_trien: true },
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

    /// Vẽ Trang chủ vào `ui` (đã có thanh menu phía trên); trả chức năng vừa bấm.
    pub fn ve(&self, ui: &mut egui::Ui, license: &crate::license::KiemTra) -> Option<ChonChucNang> {
        egui::Panel::bottom("tc_day").exact_size(CAO_DAY).frame(egui::Frame::new().fill(NEN)).show(ui, |ui| self.thanh_day(ui, license));
        let mut chon = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
            let vung = ui.max_rect();
            let dau = Rect::from_min_size(vung.min, egui::vec2(vung.width(), CAO_DAU));
            dai_dau(ui, dau);
            let y_o = dau.bottom() + 46.0;
            // Mỗi hàng 4 ô, hàng cuối căn trái thẳng cột hàng trên
            let rong_hang = O_MOI_HANG as f32 * RONG_O + (O_MOI_HANG - 1) as f32 * KHE_O;
            let x0 = vung.center().x - rong_hang / 2.0;
            let so_hang = CAC_O.len().div_ceil(O_MOI_HANG);
            let mut tom = None;
            for (i, o) in CAC_O.iter().enumerate() {
                let (hang, cot) = (i / O_MOI_HANG, i % O_MOI_HANG);
                let r = Rect::from_min_size(egui::pos2(x0 + cot as f32 * (RONG_O + KHE_O), y_o + hang as f32 * (CAO_O + KHE_HANG)), egui::vec2(RONG_O, CAO_O));
                let ph = ui.interact(r, ui.scope_id().with(("tc_o", i)), Sense::click());
                ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, o.ten));
                let re = ph.hovered() || ph.has_focus();
                o_chuc_nang(ui, r, o, re);
                if re {
                    tom = Some((r, o.tom));
                }
                if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    chon = Some(o.cn);
                }
            }
            luoi_thong_tin(ui, vung.center().x, y_o + so_hang as f32 * CAO_O + (so_hang - 1) as f32 * KHE_HANG + 44.0, license);
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
    let tex = crate::tro_giup::logo(ui.ctx());
    let kich = egui::vec2(60.0, 60.0 * tex.size()[1] as f32 / tex.size()[0].max(1) as f32);
    let ten = p.layout_no_wrap(TIEU_DE.into(), font(19.0, true), CHU);
    let moi = p.layout_no_wrap("Chọn chức năng để bắt đầu".into(), font(12.5, false), CHU_PHU);
    let cao = kich.y + 8.0 + ten.size().y + 4.0 + moi.size().y;
    let mut y = r.center().y - cao / 2.0;
    let o_logo = Rect::from_center_size(egui::pos2(r.center().x, y + kich.y / 2.0), kich);
    p.image(tex.id(), o_logo, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    let ph = ui.interact(o_logo, ui.scope_id().with("tc_logo"), Sense::click()).on_hover_text("saomaiso.com");
    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(crate::tro_giup::WEBSITE));
    }
    y += kich.y + 8.0;
    p.galley(egui::pos2(r.center().x - ten.size().x / 2.0, y), ten.clone(), CHU);
    y += ten.size().y + 4.0;
    p.galley(egui::pos2(r.center().x - moi.size().x / 2.0, y), moi, CHU_PHU);
}

/// 1 ô chức năng: viền bo góc, logo nét mảnh + tên; rê chuột → nổi lên 2px, viền + logo xanh, bóng mờ.
fn o_chuc_nang(ui: &egui::Ui, r: Rect, o: &O, re: bool) {
    let p = ui.painter();
    let r = if re { r.translate(egui::vec2(0.0, -2.0)) } else { r };
    if re {
        p.rect_filled(r.translate(egui::vec2(0.0, 5.0)).expand(1.0), 7.0, Color32::from_rgba_unmultiplied(0, 71, 171, 18));
    }
    p.rect(r, 6.0, Color32::WHITE, Stroke::new(1.0, if re { XANH_COBALT } else { VIEN }), StrokeKind::Inside);
    let mo = |c: Color32| if o.dang_phat_trien { c.gamma_multiply(0.4) } else { c };
    let ten = p.layout_no_wrap(o.ten.into(), font(13.5, true), mo(CHU));
    let cao = CO_LOGO + 12.0 + ten.size().y;
    let y = r.center().y - cao / 2.0;
    logo_chuc_nang::ve_mau(p, Rect::from_min_size(egui::pos2(r.center().x - CO_LOGO / 2.0, y), egui::vec2(CO_LOGO, CO_LOGO)), o.logo, mo(if re { XANH_COBALT } else { MAU_NET }));
    p.galley(egui::pos2(r.center().x - ten.size().x / 2.0, y + CO_LOGO + 12.0), ten, mo(CHU));
    if o.dang_phat_trien {
        let g = p.layout_no_wrap("Đang phát triển".into(), font(10.0, true), Color32::WHITE);
        let n = Rect::from_min_size(egui::pos2(r.right() - 10.0 - g.size().x - 16.0, r.top() + 10.0), g.size() + egui::vec2(16.0, 4.0));
        p.rect_filled(n, n.height() / 2.0, CAM);
        p.galley(n.center() - g.size() / 2.0, g, Color32::WHITE);
    }
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
fn luoi_thong_tin(ui: &egui::Ui, giua: f32, y: f32, license: &crate::license::KiemTra) {
    let p = ui.painter();
    let rong_ke = 780.0_f32.min(ui.max_rect().width() - 40.0);
    p.hline(egui::Rangef::new(giua - rong_ke / 2.0, giua + rong_ke / 2.0), y, Stroke::new(1.0, VIEN_NHAT));
    let luot = match license.luot() {
        Some((dung, han)) => format!("Lượt chạy {dung} / {han}"),
        None => "Lượt chạy".into(),
    };
    let muc: [(BieuTuong, String, &str); 6] = [
        (BieuTuong::ManHinh, "Xử lý ngay trên máy".into(), "File PDF / Excel không tải lên mạng"),
        (BieuTuong::Khien, "Giữ nguyên file gốc".into(), "Kết quả ghi ra thư mục riêng"),
        (BieuTuong::DongHo, luot, "Dùng chung mọi chức năng"),
        (BieuTuong::TaiLieu, "Định dạng hỗ trợ".into(), "PDF scan · Excel · CSV · DOCX"),
        (BieuTuong::HoiDap, "Hướng dẫn sử dụng".into(), "Nhấn F1 hoặc Trợ giúp › Hướng dẫn"),
        (BieuTuong::DongLenh, "Chạy hàng loạt".into(), "Đếm / Tách bằng dòng lệnh"),
    ];
    let gal: Vec<_> = muc.iter().map(|(_, a, b)| (p.layout_no_wrap(a.clone(), font(12.5, true), CHU), p.layout_no_wrap((*b).into(), font(12.0, false), CHU_PHU))).collect();
    const ICON: f32 = 22.0;
    const KHE_ICON: f32 = 10.0;
    const KHE_COT: f32 = 60.0;
    let rong_cot: Vec<f32> = (0..3).map(|c| [c, c + 3].iter().map(|&i| ICON + KHE_ICON + gal[i].0.size().x.max(gal[i].1.size().x)).fold(0.0, f32::max)).collect();
    let tong = rong_cot.iter().sum::<f32>() + 2.0 * KHE_COT;
    for (i, ((a, b), (icon, _, _))) in gal.into_iter().zip(muc.iter()).enumerate() {
        let (cot, hang) = (i % 3, i / 3);
        let x = giua - tong / 2.0 + rong_cot[..cot].iter().sum::<f32>() + cot as f32 * KHE_COT;
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

    #[test]
    fn bam_o_tra_dung_chuc_nang() {
        use egui_kittest::kittest::Queryable;
        let kt = crate::license::KiemTra::gia(std::sync::Arc::new(crate::tram::KhoGia::moi(10)));
        let tc = TrangChu { ip: "192.168.1.5".into() };
        let chon = std::rc::Rc::new(std::cell::Cell::new(None));
        let c2 = chon.clone();
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1240.0, 800.0)).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            if let Some(cn) = tc.ve(ui, &kt) {
                c2.set(Some(cn));
            }
        });
        h.run();
        for (ten, cn) in [("Đếm trang A4", ChonChucNang::Dem), ("Tách văn bản", ChonChucNang::Tach), ("Sinh mục lục", ChonChucNang::MucLuc), ("Bóc tách dữ liệu", ChonChucNang::BocTach), ("Chuẩn hoá tên", ChonChucNang::ChuanTen), ("Ký văn bản", ChonChucNang::Ky), ("Đóng gói AIP", ChonChucNang::Aip)] {
            h.get_by_label(ten).click();
            h.run();
            assert_eq!(chon.take(), Some(cn), "{ten}");
        }
        assert!(h.get_all_by_label("saomaiso.com").next().is_some(), "link website ở thanh đáy");
        h.get_by_label(crate::hop_license::EMAIL);
    }
}
