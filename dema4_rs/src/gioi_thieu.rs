//! Màn mở mặc định (chưa có dữ liệu) của từng chức năng (Đếm, Tách, Sinh mục lục; Bóc tách, Chuẩn hoá tên, Ký, Đóng gói AIP đang phát triển): tên chức năng, mô tả ngắn,
//! 3 bước trong 3 thẻ vuông (vạch xanh mảnh mép trên, số thứ tự ô xám); logo nhỏ góc dưới phải về Trang chủ.
//! Link trong lời giới thiệu ("Chọn", "Phân tích") trả về `Bam` để màn gọi đúng việc như bấm nút.

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::logo_chuc_nang::{self, Logo};
use crate::gui::{ChuDam, CHU, CHU_PHU, NEN, VIEN, XANH_COBALT};

/// Link người dùng bấm trong lời giới thiệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bam {
    Chon,
    PhanTich,
    /// Bóc tách: tới ô Mô hình đọc / Loại tài liệu / tab Trường dữ liệu.
    MoHinh,
    LoaiTaiLieu,
    TruongDuLieu,
}

/// Một đoạn chữ trong thân thẻ bước: chữ thường hoặc link.
pub enum Doan {
    Chu(&'static str),
    Dam(&'static str),
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
    /// Logo lớn trên tiêu đề — nhìn là biết đang ở chức năng nào.
    pub logo: Logo,
    /// Chức năng chưa dùng được: cả màn mờ đi, nhãn "ĐANG PHÁT TRIỂN" nổi dưới tiêu đề.
    pub dang_phat_trien: bool,
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
    logo: Logo::Tach,
    dang_phat_trien: false,
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
    logo: Logo::Dem,
    dang_phat_trien: false,
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
    logo: Logo::MucLuc,
    dang_phat_trien: false,
};

pub const KY: GioiThieu = GioiThieu {
    tieu_de: "Ký văn bản",
    mo_ta: "Ký số hàng loạt các văn bản PDF đã tách bằng chứng thư số của đơn vị; chèn hình chữ ký, kiểm tra file đã ký.",
    buoc: [
        Buoc { ten: "Chọn thư mục", than: &[Doan::Chu("Thư mục văn bản đã tách (kết quả của Tách văn bản).")] },
        Buoc { ten: "Chọn chứng thư", than: &[Doan::Chu("USB token hoặc chứng thư trong máy; vị trí, kích thước chữ ký.")] },
        Buoc { ten: "Ký", than: &[Doan::Chu("Ký lần lượt từng file, báo file lỗi, ghi log kết quả.")] },
    ],
    kho: false,
    logo: Logo::Ky,
    dang_phat_trien: true,
};

pub const BOC_TACH: GioiThieu = GioiThieu {
    tieu_de: "Bóc tách dữ liệu",
    mo_ta: "Đọc văn bản PDF bằng AI để lấy số ký hiệu, ngày tháng, trích yếu, cơ quan ban hành, người ký…; kết quả ghi ra Excel và Google Sheet, dùng làm file biên mục cho Sinh mục lục.",
    buoc: [
        Buoc { ten: "Chọn nguồn văn bản", than: &[Doan::Chu("Thư mục trên máy, Google Drive, Google Cloud Storage hoặc kho S3; bấm "), Doan::Link("Chọn", Bam::Chon), Doan::Chu(".")] },
        Buoc { ten: "Chọn mô hình đọc", than: &[Doan::Chu("Máy chủ nội bộ, ChatGPT, Gemini, Claude… ở ô "), Doan::Link("Mô hình đọc", Bam::MoHinh), Doan::Chu("; khai khoá ở Cấu hình.")] },
        Buoc { ten: "Chọn loại tài liệu", than: &[Doan::Chu("Chọn ở ô "), Doan::Link("Loại tài liệu", Bam::LoaiTaiLieu), Doan::Chu("; tab "), Doan::Link("Trường dữ liệu", Bam::TruongDuLieu), Doan::Chu(" xem, thử đọc, rồi bấm "), Doan::Dam("Bắt đầu"), Doan::Chu(".")] },
    ],
    kho: false,
    logo: Logo::BocTach,
    dang_phat_trien: false,
};

pub const CHUAN_TEN: GioiThieu = GioiThieu {
    tieu_de: "Chuẩn hoá tên",
    mo_ta: "Đổi tên hàng loạt file văn bản theo một quy tắc thống nhất, lấy thông tin từ kết quả Bóc tách (số ký hiệu, ngày, loại văn bản…); bỏ ký tự không hợp lệ, xem trước rồi mới đổi.",
    buoc: [
        Buoc { ten: "Chọn thư mục", than: &[Doan::Chu("Thư mục văn bản cần đổi tên và file kết quả Bóc tách tương ứng.")] },
        Buoc { ten: "Đặt quy tắc tên", than: &[Doan::Chu("Ghép biến, vd {so_ky_hieu}_{ngay_thang}; bỏ dấu, thay ký tự cấm, xử lý trùng tên.")] },
        Buoc { ten: "Xem trước, đổi tên", than: &[Doan::Chu("Bảng tên cũ → tên mới; đổi hàng loạt, ghi log đối chiếu để hoàn tác.")] },
    ],
    kho: false,
    logo: Logo::ChuanTen,
    dang_phat_trien: true,
};

pub const AIP: GioiThieu = GioiThieu {
    tieu_de: "Đóng gói AIP",
    mo_ta: "Đóng gói hồ sơ lưu trữ thông tin số (AIP) để nộp lưu: gom văn bản PDF, siêu dữ liệu (kết quả Bóc tách) và mục lục thành gói chuẩn, kèm mã kiểm tra toàn vẹn.",
    buoc: [
        Buoc { ten: "Chọn hồ sơ", than: &[Doan::Chu("Thư mục văn bản đã tách, file biên mục / kết quả bóc tách.")] },
        Buoc { ten: "Kiểm tra", than: &[Doan::Chu("Đủ văn bản, đủ siêu dữ liệu bắt buộc, định dạng PDF/A, chữ ký số.")] },
        Buoc { ten: "Đóng gói", than: &[Doan::Chu("Tạo gói AIP cho từng hồ sơ, ghi mã kiểm tra, báo hồ sơ lỗi.")] },
    ],
    kho: false,
    logo: Logo::Aip,
    dang_phat_trien: true,
};

const RONG_MO_TA: f32 = 560.0;
const KICH_LOGO: f32 = 68.0;
const RONG_THE: f32 = 160.0;
const KHE_THE: f32 = 14.0;
const CAO_THE: f32 = 96.0;
const RONG_LOGO: f32 = 64.0;

/// Vẽ vào vùng còn trống của `ui`; trả link vừa bấm (nếu có).
pub fn ve(ui: &mut egui::Ui, gt: &GioiThieu) -> Option<Bam> {
    ve_noi_bat(ui, gt, None)
}

/// Như `ve`, làm nổi thẻ bước `noi_bat` (1-based) — bước người dùng cần làm tiếp.
pub fn ve_noi_bat(ui: &mut egui::Ui, gt: &GioiThieu, noi_bat: Option<usize>) -> Option<Bam> {
    ve_trang_thai(ui, gt, noi_bat, [false; 3])
}

/// Như `ve_noi_bat`, bước đã xong (`xong[i]`) hiện dấu ✓ nền xanh ở ô số.
pub fn ve_trang_thai(ui: &mut egui::Ui, gt: &GioiThieu, noi_bat: Option<usize>, xong: [bool; 3]) -> Option<Bam> {
    let vung = ui.available_rect_before_wrap();
    let mut bam = None;
    let mut cho_nhan = None;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if gt.dang_phat_trien {
            ui.multiply_opacity(0.4);
        }
        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            let (o, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), KICH_LOGO + 4.0), Sense::hover());
            logo_chuc_nang::ve(ui.painter(), Rect::from_center_size(egui::pos2(o.center().x, o.top() + KICH_LOGO / 2.0), egui::vec2(KICH_LOGO, KICH_LOGO)), gt.logo);
            ui.add_space(14.0);
            // Tiêu đề vẽ thẳng (không phải widget) để không trùng nhãn menu / tab cùng tên khi tìm theo accessibility
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, gt.tieu_de, egui::FontId::new(17.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU);
            ui.add_space(4.0);
            if gt.dang_phat_trien {
                // Chừa chỗ cho nhãn; vẽ sau, ngoài phần bị làm mờ
                let (o, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::hover());
                ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, "Chức năng đang phát triển"));
                cho_nhan = Some(o);
                ui.add_space(4.0);
            }
            ui.allocate_ui_with_layout(egui::vec2(RONG_MO_TA.min(ui.available_width()), 0.0), egui::Layout::top_down(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(gt.mo_ta).size(12.0).color(CHU_PHU)).halign(Align::Center).wrap());
            });
            ui.add_space(16.0);
        });
        // 3 thẻ bước, căn giữa theo bề rộng
        let rong_hang = 3.0 * RONG_THE + 2.0 * KHE_THE;
        // 3 thẻ cao bằng nhau: lấy chiều cao thẻ cao nhất của khung hình trước
        let id_cao = egui::Id::unique("gt_cao_the").with(gt.tieu_de);
        let cao = ui.ctx().data(|d| d.get_temp::<f32>(id_cao)).unwrap_or(CAO_THE);
        let mut cao_moi = CAO_THE;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = KHE_THE;
            ui.add_space(((ui.available_width() - rong_hang) / 2.0).max(0.0));
            for (i, b) in gt.buoc.iter().enumerate() {
                let (x, h) = the_buoc(ui, i + 1, b, noi_bat == Some(i + 1), xong[i], cao);
                cao_moi = cao_moi.max(h);
                if x.is_some() {
                    bam = x;
                }
            }
        });
        if (cao_moi - cao).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(id_cao, cao_moi));
            ui.ctx().request_repaint();
        }
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
    if let Some(o) = cho_nhan {
        nhan_dang_phat_trien(&ui.ctx().layer_painter(ui.layer_id()), o.center());
    }
    logo_goc(ui, vung);
    bam
}

/// Nhãn "ĐANG PHÁT TRIỂN": viên thuốc nền cam, chữ trắng đậm — giữ nguyên độ đậm khi cả màn bị làm mờ.
fn nhan_dang_phat_trien(p: &egui::Painter, tam: egui::Pos2) {
    let f = egui::FontId::new(12.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into()));
    let g = p.layout_no_wrap("ĐANG PHÁT TRIỂN".into(), f, Color32::WHITE);
    let r = Rect::from_center_size(tam, g.size() + egui::vec2(28.0, 10.0));
    p.rect_filled(r, r.height() / 2.0, Color32::from_rgb(0xD9, 0x82, 0x2B));
    p.galley(r.center() - g.size() / 2.0, g, Color32::WHITE);
}

/// 1 thẻ bước: ô số xám, tên đậm, thân chữ (có thể chứa link). `noi` = bước cần làm tiếp: viền xanh, ô số xanh.
/// Trả (link vừa bấm, chiều cao nội dung thẻ) — `cao_min` = chiều cao chung của hàng thẻ.
fn the_buoc(ui: &mut egui::Ui, so: usize, b: &Buoc, noi: bool, xong: bool, cao_min: f32) -> (Option<Bam>, f32) {
    let mut bam = None;
    let mut cao_noi_dung = CAO_THE;
    let khung = egui::Frame::new().fill(Color32::WHITE).stroke(if noi { Stroke::new(1.5, XANH_COBALT) } else { Stroke::new(1.0, VIEN) }).inner_margin(Margin { left: 10, right: 10, top: 10, bottom: 12 });
    let r = khung.show(ui, |ui| {
        // Thẻ nằm trong hàng ngang: nội dung bên trong phải xếp dọc
        ui.vertical(|ui| {
        let tren = ui.cursor().top();
        ui.set_width(RONG_THE - 22.0);
        ui.set_min_height(cao_min);
        ui.spacing_mut().item_spacing.y = 0.0;
        let (o, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
        let (nen, vien, chu) = if xong { (crate::gui::XANH_LA, crate::gui::XANH_LA, Color32::WHITE) } else if noi { (XANH_COBALT, XANH_COBALT, Color32::WHITE) } else { (NEN, VIEN, CHU_PHU) };
        ui.painter().rect(o, 0.0, nen, Stroke::new(1.0, vien), StrokeKind::Inside);
        if xong {
            // Dấu ✓ vẽ nét (font giao diện không có ký tự ✓)
            let c = o.center();
            ui.painter().line(vec![c + egui::vec2(-5.0, 0.0), c + egui::vec2(-1.5, 3.5), c + egui::vec2(5.0, -4.0)], Stroke::new(1.8, Color32::WHITE));
        } else {
            ui.painter().text(o.center(), egui::Align2::CENTER_CENTER, so.to_string(), egui::FontId::proportional(12.0), chu);
        }
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
                    Doan::Dam(c) => {
                        ui.label(RichText::new(*c).size(12.0).color(CHU).dam());
                    }
                    Doan::Link(c, x) => {
                        if ui.add(egui::Link::new(RichText::new(*c).size(12.0).color(XANH_COBALT).underline())).clicked() {
                            bam = Some(*x);
                        }
                    }
                }
            }
        });
        cao_noi_dung = ui.cursor().top() - tren;
        });
    });
    // Vạch xanh mảnh mép trên thẻ
    let rect = r.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(rect.width(), if noi { 3.0 } else { 2.0 })), 0.0, XANH_COBALT);
    (bam, cao_noi_dung)
}

/// Cờ "về Trang chủ" (bấm logo góc dưới phải) — cửa sổ chính đọc và xoá sau mỗi khung hình.
pub fn id_ve_trang_chu() -> egui::Id {
    egui::Id::unique("gioi_thieu_ve_trang_chu")
}

/// Logo Sao Mai góc dưới phải vùng trống; bấm về Trang chủ.
fn logo_goc(ui: &mut egui::Ui, vung: Rect) {
    let tex = crate::tro_giup::logo(ui.ctx());
    let [w, h] = tex.size();
    let kich = egui::vec2(RONG_LOGO, RONG_LOGO * h as f32 / w.max(1) as f32);
    let r = Rect::from_min_size(egui::pos2(vung.right() - 20.0 - kich.x, vung.bottom() - 16.0 - kich.y), kich);
    if r.min.y < vung.top() + 300.0 {
        return; // vùng quá thấp (cửa sổ nhỏ): không đè logo lên nội dung
    }
    let anh = egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), kich)).sense(Sense::click());
    let ph = ui.put(r, anh);
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, "Trang chủ"));
    let ph = ph.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Trang chủ");
    if ph.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(id_ve_trang_chu(), true));
    }
}
