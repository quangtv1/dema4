//! Màn mở mặc định (chưa có dữ liệu) của Đếm trang A4 / Tách văn bản / Sinh mục lục / Ký văn bản: tên chức năng, mô tả ngắn,
//! 3 bước trong 3 thẻ vuông (vạch xanh mảnh mép trên, số thứ tự ô xám); logo nhỏ góc dưới phải mở saomaiso.com.
//! Link trong lời giới thiệu ("Chọn", "Phân tích") trả về `Bam` để màn gọi đúng việc như bấm nút.

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::gui::{ChuDam, CHU, CHU_PHU, NEN, VIEN, XANH_COBALT};

/// Link người dùng bấm trong lời giới thiệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bam {
    Chon,
    PhanTich,
}

/// Một đoạn chữ trong thân thẻ bước: chữ thường hoặc link.
pub enum Doan {
    Chu(&'static str),
    Link(&'static str, Bam),
}

pub struct Buoc {
    pub ten: &'static str,
    pub than: &'static [Doan],
}

pub struct GioiThieu {
    pub tieu_de: &'static str,
    pub mo_ta: &'static str,
    pub buoc: [Buoc; 3],
    /// Dãy nhãn khổ A0–A7 dưới các bước (màn Đếm).
    pub kho: bool,
}

pub const TACH: GioiThieu = GioiThieu {
    tieu_de: "Tách văn bản",
    mo_ta: "Tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã vạch), tự bỏ trang trắng và gợi ý trang bị xoay. Nhãn do máy đoán, người duyệt lại trước khi tách — file PDF gốc không bị sửa.",
    buoc: [
        Buoc { ten: "Chọn thư mục", than: &[Doan::Chu("Bấm "), Doan::Link("Chọn", Bam::Chon), Doan::Chu(" rồi "), Doan::Link("Phân tích", Bam::PhanTich), Doan::Chu("; tên hồ sơ hiện ngay ở cột trái.")] },
        Buoc { ten: "Duyệt nhãn", than: &[Doan::Chu("Kiểm tra Index · Trắng · Split; duyệt hết Nghi trắng, Nghi xoay.")] },
        Buoc { ten: "Tách", than: &[Doan::Chu("Mỗi hồ sơ một thư mục, văn bản đặt tên theo mẫu, có log kết quả.")] },
    ],
    kho: false,
};

pub const DEM: GioiThieu = GioiThieu {
    tieu_de: "Đếm trang A4",
    mo_ta: "Đếm số trang của mọi PDF trong thư mục (cả thư mục con), phân loại khổ giấy và quy đổi ra số trang A4 theo hệ số; cảnh báo trang sát ngưỡng khổ, DPI bất thường, file đã ký số.",
    buoc: [
        Buoc { ten: "Chọn thư mục", than: &[Doan::Chu("Bấm "), Doan::Link("Chọn", Bam::Chon), Doan::Chu("; tuỳ chọn Check ký số, Check DPI.")] },
        Buoc { ten: "Đếm", than: &[Doan::Chu("Bảng chi tiết từng trang, lọc theo khổ, bấm tên file để mở.")] },
        Buoc { ten: "Lấy kết quả", than: &[Doan::Chu("Tự lưu _dem.txt · Mở kết quả · Copy dán vào Excel.")] },
    ],
    kho: true,
};

pub const MUC_LUC: GioiThieu = GioiThieu {
    tieu_de: "Sinh mục lục hồ sơ",
    mo_ta: "Sinh mục lục văn bản cho từng hồ sơ từ file biên mục Excel/CSV theo mẫu DOCX của đơn vị; xem mẫu, xem trước từng hồ sơ, xuất DOCX hàng loạt.",
    buoc: [
        Buoc { ten: "Chọn file biên mục", than: &[Doan::Chu("Bấm "), Doan::Link("Chọn", Bam::Chon), Doan::Chu("; mỗi văn bản một dòng, các dòng cùng hồ sơ nằm liền nhau.")] },
        Buoc { ten: "Chọn mẫu, ghép biến", than: &[Doan::Chu("Tab Mẫu xem biến của mẫu; biến tự khớp tên cột, bấm ô Lấy từ để sửa.")] },
        Buoc { ten: "Sinh mục lục", than: &[Doan::Chu("Tab Kết quả xem từng hồ sơ; mỗi hồ sơ một file DOCX.")] },
    ],
    kho: false,
};

pub const KY: GioiThieu = GioiThieu {
    tieu_de: "Ký văn bản",
    mo_ta: "Ký số hàng loạt các văn bản PDF đã tách bằng chứng thư số của đơn vị; chèn hình chữ ký, kiểm tra file đã ký. Chức năng đang phát triển.",
    buoc: [
        Buoc { ten: "Chọn thư mục", than: &[Doan::Chu("Thư mục văn bản đã tách (kết quả của Tách văn bản).")] },
        Buoc { ten: "Chọn chứng thư", than: &[Doan::Chu("USB token hoặc chứng thư trong máy; vị trí, kích thước chữ ký.")] },
        Buoc { ten: "Ký", than: &[Doan::Chu("Ký lần lượt từng file, báo file lỗi, ghi log kết quả.")] },
    ],
    kho: false,
};

const RONG_MO_TA: f32 = 560.0;
const RONG_THE: f32 = 160.0;
const KHE_THE: f32 = 14.0;
const CAO_THE: f32 = 96.0;
const RONG_LOGO: f32 = 64.0;
pub const WEB: &str = "https://saomaiso.com";

/// Vẽ vào vùng còn trống của `ui`; trả link vừa bấm (nếu có).
pub fn ve(ui: &mut egui::Ui, gt: &GioiThieu) -> Option<Bam> {
    ve_noi_bat(ui, gt, None)
}

/// Như `ve`, làm nổi thẻ bước `noi_bat` (1-based) — bước người dùng cần làm tiếp.
pub fn ve_noi_bat(ui: &mut egui::Ui, gt: &GioiThieu, noi_bat: Option<usize>) -> Option<Bam> {
    let vung = ui.available_rect_before_wrap();
    let mut bam = None;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(56.0);
        ui.vertical_centered(|ui| {
            // Tiêu đề vẽ thẳng (không phải widget) để không trùng nhãn menu / tab cùng tên khi tìm theo accessibility
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, gt.tieu_de, egui::FontId::new(17.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU);
            ui.add_space(4.0);
            ui.allocate_ui_with_layout(egui::vec2(RONG_MO_TA.min(ui.available_width()), 0.0), egui::Layout::top_down(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(gt.mo_ta).size(12.0).color(CHU_PHU)).halign(Align::Center).wrap());
            });
            ui.add_space(16.0);
        });
        // 3 thẻ bước, căn giữa theo bề rộng
        let rong_hang = 3.0 * RONG_THE + 2.0 * KHE_THE;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = KHE_THE;
            ui.add_space(((ui.available_width() - rong_hang) / 2.0).max(0.0));
            for (i, b) in gt.buoc.iter().enumerate() {
                if let Some(x) = the_buoc(ui, i + 1, b, noi_bat == Some(i + 1)) {
                    bam = Some(x);
                }
            }
        });
        if gt.kho {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let rong = 8.0 * 30.0 + 7.0 * 4.0;
                ui.add_space(((ui.available_width() - rong) / 2.0).max(0.0));
                for k in 0..8 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 18.0), Sense::hover());
                    ui.painter().rect(r, 0.0, Color32::WHITE, Stroke::new(1.0, VIEN), StrokeKind::Inside);
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format!("A{k}"), egui::FontId::proportional(11.0), CHU_PHU);
                }
            });
        }
    });
    logo_goc(ui, vung);
    bam
}

/// 1 thẻ bước: ô số xám, tên đậm, thân chữ (có thể chứa link). `noi` = bước cần làm tiếp: viền xanh, ô số xanh.
fn the_buoc(ui: &mut egui::Ui, so: usize, b: &Buoc, noi: bool) -> Option<Bam> {
    let mut bam = None;
    let khung = egui::Frame::new().fill(Color32::WHITE).stroke(if noi { Stroke::new(1.5, XANH_COBALT) } else { Stroke::new(1.0, VIEN) }).inner_margin(Margin { left: 10, right: 10, top: 10, bottom: 12 });
    let r = khung.show(ui, |ui| {
        // Thẻ nằm trong hàng ngang: nội dung bên trong phải xếp dọc
        ui.vertical(|ui| {
        ui.set_width(RONG_THE - 22.0);
        ui.set_min_height(CAO_THE);
        ui.spacing_mut().item_spacing.y = 0.0;
        let (o, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
        let (nen, vien, chu) = if noi { (XANH_COBALT, XANH_COBALT, Color32::WHITE) } else { (NEN, VIEN, CHU_PHU) };
        ui.painter().rect(o, 0.0, nen, Stroke::new(1.0, vien), StrokeKind::Inside);
        ui.painter().text(o.center(), egui::Align2::CENTER_CENTER, so.to_string(), egui::FontId::proportional(12.0), chu);
        ui.add_space(6.0);
        ui.label(RichText::new(b.ten).size(12.0).color(CHU).dam());
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for d in b.than {
                match d {
                    Doan::Chu(c) => {
                        ui.label(RichText::new(*c).size(12.0).color(CHU));
                    }
                    Doan::Link(c, x) => {
                        if ui.add(egui::Link::new(RichText::new(*c).size(12.0).color(XANH_COBALT).underline())).clicked() {
                            bam = Some(*x);
                        }
                    }
                }
            }
        });
        });
    });
    // Vạch xanh mảnh mép trên thẻ
    let rect = r.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(rect.width(), if noi { 3.0 } else { 2.0 })), 0.0, XANH_COBALT);
    bam
}

/// Logo Sao Mai góc dưới phải vùng trống; bấm mở website.
fn logo_goc(ui: &mut egui::Ui, vung: Rect) {
    let tex = crate::tro_giup::logo(ui.ctx());
    let [w, h] = tex.size();
    let kich = egui::vec2(RONG_LOGO, RONG_LOGO * h as f32 / w.max(1) as f32);
    let r = Rect::from_min_size(egui::pos2(vung.right() - 20.0 - kich.x, vung.bottom() - 16.0 - kich.y), kich);
    if r.min.y < vung.top() + 300.0 {
        return; // vùng quá thấp (cửa sổ nhỏ): không đè logo lên nội dung
    }
    let anh = egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), kich)).sense(Sense::click());
    let ph = ui.put(r, anh).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("saomaiso.com");
    if ph.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(WEB));
    }
}
