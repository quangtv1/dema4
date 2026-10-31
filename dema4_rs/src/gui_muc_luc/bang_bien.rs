//! Sidebar tab Mẫu: bảng `Biến | Lấy từ | !` chia 3 nhóm; ô Lấy từ sửa tại chỗ (khung xổ ngay dưới dòng: gõ giá trị
//! cố định rồi Enter, hoặc chọn cột; Esc / bấm ra ngoài = đóng); hàng "Nhóm theo" ở chân sidebar.

use eframe::egui::{self, Color32, Key, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::gui::{CHU, CHU_PHU, VIEN, VIEN_NHAT};
use crate::muc_luc::{Nguon, RangBuoc};

pub const RONG_COT_BIEN: f32 = 128.0;
const RONG_COT_CANH: f32 = 22.0;
const CAO_DONG: f32 = 33.0;
const CAO_NHOM: f32 = 24.0;
pub const CAO_DAU: f32 = 27.0;
const CAM: Color32 = Color32::from_rgb(0xB8, 0x5F, 0x00);

/// Loại biến → (nền, chữ) của chip; dùng chung cho sidebar và trang mẫu để màu khớp nhau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loai {
    ThongTin,
    Bang,
    TuDong,
    ChuaGhep,
}

impl Loai {
    pub fn mau(self) -> (Color32, Color32) {
        match self {
            Loai::ThongTin => (Color32::from_rgb(0xDA, 0xE4, 0xF0), Color32::from_rgb(0x2C, 0x3E, 0x55)),
            Loai::Bang => (Color32::from_rgb(0xD8, 0xEC, 0xDE), Color32::from_rgb(0x2F, 0x54, 0x40)),
            Loai::TuDong => (Color32::from_rgb(0xEE, 0xF0, 0xF2), Color32::from_rgb(0x7A, 0x85, 0x94)),
            Loai::ChuaGhep => (Color32::from_rgb(0xFF, 0xE9, 0xC2), Color32::from_rgb(0x8A, 0x5A, 0x00)),
        }
    }
}

/// Biến của mẫu theo nhóm: thông tin hồ sơ, bảng văn bản, tự động.
pub struct NhomBien {
    pub nhom: [Vec<String>; 3],
}

impl NhomBien {
    pub fn loai(&self, rb: &RangBuoc, b: &str) -> Loai {
        if self.nhom[2].iter().any(|x| x == b) {
            Loai::TuDong
        } else if rb.nguon(b).is_none() {
            Loai::ChuaGhep
        } else if self.nhom[1].iter().any(|x| x == b) {
            Loai::Bang
        } else {
            Loai::ThongTin
        }
    }

    /// Loại theo vùng của biến (không xét đã ghép hay chưa) — màu chip ở sidebar.
    pub fn loai_vung(&self, b: &str) -> Loai {
        if self.nhom[2].iter().any(|x| x == b) {
            Loai::TuDong
        } else if self.nhom[1].iter().any(|x| x == b) {
            Loai::Bang
        } else {
            Loai::ThongTin
        }
    }

    pub fn tong(&self) -> usize {
        self.nhom.iter().map(Vec::len).sum()
    }
}

pub enum Viec {
    MoSua(String),
    DongSua,
    DatNguon(String, Option<Nguon>),
}

pub struct VaoBang<'a> {
    pub nhom: &'a NhomBien,
    pub rb: &'a RangBuoc,
    pub cot: &'a [String],
    pub khac_nhau: &'a [usize],
    pub mo: Option<&'a str>,
    pub sang: Option<&'a str>,
}

/// Chip `{bien}` vẽ tay (nền bo góc) trong ô `o`, căn trái giữa dọc.
pub(super) fn chip(ui: &egui::Ui, o: Rect, ten: &str, (nen, chu): (Color32, Color32)) {
    let g = ui.painter().layout(format!("{{{ten}}}"), egui::FontId::monospace(11.5), chu, o.width() - 12.0);
    let r = Rect::from_min_size(egui::pos2(o.left(), o.center().y - g.size().y / 2.0 - 3.0), g.size() + egui::vec2(12.0, 6.0));
    ui.painter().rect_filled(r, 3.0, nen);
    ui.painter().galley(r.min + egui::vec2(6.0, 3.0), g, chu);
}

/// Nền dải đầu (tiêu đề bảng biến, thanh công cụ khung xem bên phải) — cùng một màu.
pub const NEN_DAI_DAU: Color32 = Color32::from_gray(0xFA);
/// Kẻ mờ ngăn giữa các vùng biến (thông tin hồ sơ · bảng văn bản · tự động).
const KE_VUNG: Color32 = Color32::from_gray(0xE4);

fn hang_tieu_de(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_DAU), Sense::hover());
    ui.painter().rect_filled(r, 0.0, NEN_DAI_DAU);
    ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    let f = egui::FontId::proportional(12.0);
    ui.painter().text(egui::pos2(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, "Biến", f.clone(), CHU_PHU);
    ui.painter().text(egui::pos2(r.left() + RONG_COT_BIEN + 6.0, r.center().y), egui::Align2::LEFT_CENTER, "Lấy từ", f, CHU_PHU);
}

pub(super) fn hang_nhom(ui: &mut egui::Ui, ten: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_NHOM), Sense::hover());
    ui.painter().rect_filled(r, 0.0, Color32::from_gray(0xFA));
    ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(0xEF)));
    ui.painter().text(egui::pos2(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, ten, egui::FontId::new(11.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CHU_PHU);
}

/// Ô Lấy từ: cột = nền slate nhạt; giá trị cố định = như ô nhập (trắng, viền); tự động = xám; chưa ghép = vàng.
pub(super) fn o_lay_tu(ui: &egui::Ui, r: Rect, nguon: Option<&Nguon>, tu_dong: Option<&str>, mo: bool, re: bool) {
    let (nen, vien, chu, nghieng, chu_hien) = match (tu_dong, nguon) {
        (Some(t), _) => (Color32::from_rgb(0xF4, 0xF5, 0xF7), Color32::TRANSPARENT, Color32::from_rgb(0x7A, 0x85, 0x94), false, t.to_string()),
        (None, Some(Nguon::Cot(c))) => (Color32::from_rgb(0xED, 0xF1, 0xF6), Color32::TRANSPARENT, Color32::from_rgb(0x2C, 0x3E, 0x55), false, c.clone()),
        (None, Some(Nguon::Hang(h))) => (Color32::WHITE, Color32::from_rgb(0xC9, 0xD3, 0xDF), CHU, false, h.clone()),
        (None, Some(Nguon::TuDong)) => (Color32::from_rgb(0xF4, 0xF5, 0xF7), Color32::TRANSPARENT, CHU_PHU, false, "tự động".into()),
        (None, None) => (Color32::from_rgb(0xFF, 0xFB, 0xEF), Color32::from_rgb(0xE8, 0xC7, 0x7A), CAM, true, "Chọn cột / nhập giá trị".into()),
    };
    let vien = if mo { crate::gui::VIEN_TAB_CHON } else if re && tu_dong.is_none() { Color32::from_rgb(0x9F, 0xB0, 0xC4) } else { vien };
    ui.painter().rect(r, 3.0, nen, Stroke::new(1.0, vien), StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::single_section(chu_hien, egui::TextFormat { font_id: egui::FontId::proportional(12.0), color: chu, italics: nghieng, ..Default::default() });
    job.wrap = egui::text::TextWrapping::truncate_at_width(r.width() - 22.0);
    let g = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(egui::pos2(r.left() + 6.0, r.center().y - g.size().y / 2.0), g, chu);
    if (mo || re) && tu_dong.is_none() {
        crate::gui_tach::bieu_tuong::ve(ui.painter(), Rect::from_center_size(egui::pos2(r.right() - 10.0, r.center().y), egui::vec2(10.0, 10.0)), crate::gui_tach::bieu_tuong::BieuTuong::ChevronXuong, Color32::from_gray(0x66));
    }
}

/// Khung xổ dưới dòng đang sửa: ô gõ + danh sách cột (lọc theo chữ gõ) + Bỏ ghép.
pub(super) fn khung_sua(ui: &mut egui::Ui, v: &VaoBang, b: &str, o_go: &mut String, viec: &mut Vec<Viec>) -> Rect {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, VIEN))
        .corner_radius(4.0)
        .shadow(egui::Shadow { offset: [0, 3], blur: 10, spread: 0, color: Color32::from_black_alpha(40) })
        .inner_margin(6)
        .outer_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 8 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let id = ui.make_persistent_id(("ml_go", b));
            let r = ui.add(egui::TextEdit::singleline(o_go).id(id).hint_text("Gõ giá trị cố định rồi Enter, hoặc chọn…").desired_width(f32::INFINITY));
            // Mở khung → con trỏ vào ô gõ ngay (1 lần cho mỗi biến)
            let da = ui.data_mut(|d| std::mem::replace(d.get_temp_mut_or_default::<String>(egui::Id::unique("ml_go_focus")), b.to_string()));
            if da != b {
                r.request_focus();
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                viec.push(Viec::DongSua);
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && !o_go.trim().is_empty() {
                viec.push(Viec::DatNguon(b.into(), Some(Nguon::Hang(o_go.trim().to_string()))));
            }
            ui.add_space(2.0);
            ui.label(RichText::new("Cột trong file biên mục").size(11.0).color(CHU_PHU));
            let loc = crate::muc_luc::khop::chuan_hoa(o_go);
            egui::ScrollArea::vertical().id_salt(("ml_ds_cot", b)).max_height(220.0).min_scrolled_height(160.0).show(ui, |ui| {
                for (i, c) in v.cot.iter().enumerate().filter(|(_, c)| !c.trim().is_empty() && (loc.is_empty() || crate::muc_luc::khop::chuan_hoa(c).contains(&loc))) {
                    let chon = matches!(v.rb.nguon(b), Some(Nguon::Cot(x)) if x == c);
                    let (r, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), Sense::click());
                    if ph.hovered() {
                        ui.painter().rect_filled(r, 3.0, crate::gui::NEN_TAB_CHON);
                    }
                    let f = if chon { egui::FontId::new(12.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(12.0) };
                    ui.painter().text(egui::pos2(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, c, f, CHU);
                    if let Some(n) = v.khac_nhau.get(i) {
                        ui.painter().text(egui::pos2(r.right() - 6.0, r.center().y), egui::Align2::RIGHT_CENTER, n.to_string(), egui::FontId::proportional(11.0), CHU_PHU);
                    }
                    if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        viec.push(Viec::DatNguon(b.into(), Some(Nguon::Cot(c.clone()))));
                    }
                }
            });
            if v.rb.nguon(b).is_some() {
                ui.add_space(2.0);
                ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.cursor().top(), Stroke::new(1.0, Color32::from_gray(0xEF)));
                let (r, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), Sense::click());
                if ph.hovered() {
                    ui.painter().rect_filled(r, 3.0, crate::gui::NEN_TAB_CHON);
                }
                ui.painter().text(egui::pos2(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, "Bỏ ghép", egui::FontId::proportional(12.0), CAM);
                if ph.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    viec.push(Viec::DatNguon(b.into(), None));
                }
            }
        })
        .response
        .rect
}

/// Bảng biến; `re` nhận tên biến dòng đang rê chuột (định vị trên trang mẫu).
pub fn ve(ui: &mut egui::Ui, v: &VaoBang, o_go: &mut String, re: &mut Option<String>) -> Vec<Viec> {
    let mut viec = Vec::new();
    hang_tieu_de(ui);
    let mut vung_mo: Vec<Rect> = Vec::new();
    egui::ScrollArea::vertical().id_salt("ml_bang_bien").auto_shrink([false; 2]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        // Các vùng không có tiêu đề: phân biệt bằng màu chip, giữa hai vùng một kẻ mờ
        let mut vung_truoc = false;
        ui.add_space(4.0);
        for (g, ds) in v.nhom.nhom.iter().enumerate() {
            if ds.is_empty() {
                continue;
            }
            if vung_truoc {
                ui.add_space(5.0);
                let x = ui.available_rect_before_wrap().x_range();
                ui.painter().hline(egui::Rangef::new(x.min + 6.0, x.max - 6.0), ui.cursor().top() + 0.5, Stroke::new(1.0, KE_VUNG));
                ui.add_space(6.0);
            }
            vung_truoc = true;
            for b in ds {
                let (r, ph) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO_DONG), Sense::hover());
                let dang_re = ph.hovered() || v.sang == Some(b.as_str());
                if ph.hovered() {
                    *re = Some(b.clone());
                }
                if dang_re {
                    ui.painter().rect_filled(r, 0.0, Color32::from_rgb(0xE6, 0xF0, 0xFB));
                }
                let loai = v.nhom.loai(v.rb, b);
                // Chip biến giữ màu của vùng; chưa ghép chỉ tô ô Lấy từ + dấu ! cuối dòng
                chip(ui, Rect::from_min_max(egui::pos2(r.left() + 6.0, r.top()), egui::pos2(r.left() + RONG_COT_BIEN - 4.0, r.bottom())), b, v.nhom.loai_vung(b).mau());
                let o = Rect::from_min_max(egui::pos2(r.left() + RONG_COT_BIEN, r.top() + 3.0), egui::pos2(r.right() - RONG_COT_CANH, r.bottom() - 3.0));
                let tu_dong = (g == 2).then(|| if b == "tong_so_trang" { "tổng số trang" } else if b == "trang_so" { "số trang" } else { "tự động" });
                let mo = v.mo == Some(b.as_str());
                let ph_o = ui.interact(o, ui.scope_id().with(("ml_o", b)), if tu_dong.is_some() { Sense::hover() } else { Sense::click() });
                o_lay_tu(ui, o, v.rb.nguon(b), tu_dong, mo, ph_o.hovered());
                if tu_dong.is_some() {
                    ph_o.on_hover_text("Word tự điền khi in");
                } else if ph_o.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    viec.push(if mo { Viec::DongSua } else { Viec::MoSua(b.clone()) });
                }
                if loai == Loai::ChuaGhep {
                    ui.painter().text(egui::pos2(r.right() - RONG_COT_CANH / 2.0, r.center().y), egui::Align2::CENTER_CENTER, "!", egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), CAM);
                }
                if mo {
                    vung_mo.push(o);
                    vung_mo.push(khung_sua(ui, v, b, o_go, &mut viec));
                }
            }
        }
    });
    // Bấm ra ngoài ô / khung đang sửa → đóng
    if v.mo.is_some() && viec.is_empty() {
        let bam = ui.input(|i| if i.pointer.primary_clicked() { i.pointer.interact_pos() } else { None });
        if bam.is_some_and(|p| !vung_mo.iter().any(|r| r.contains(p))) {
            viec.push(Viec::DongSua);
        }
    }
    viec
}

/// Hàng "Nhóm theo" (chân sidebar): thẳng cột với bảng, nền riêng; trả cột vừa chọn.
pub fn hang_nhom_theo(ui: &mut egui::Ui, cot: &[String], khac_nhau: &[usize], hien_tai: &str) -> Option<String> {
    let r = ui.max_rect();
    ui.painter().rect_filled(r, 0.0, Color32::from_rgb(0xF4, 0xF6, 0xF9));
    ui.painter().hline(r.x_range(), r.top() + 0.5, Stroke::new(1.0, VIEN_NHAT));
    let mut chon = None;
    let goi_y = "Nhóm các dòng liền nhau có cùng giá trị ở cột này thành 1 hồ sơ; (số) = số giá trị khác nhau trong cột";
    let ten = |i: usize, c: &str| format!("{c} ({})", khac_nhau.get(i).copied().unwrap_or(0));
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let nhan = ui
            .allocate_ui_with_layout(egui::vec2(RONG_COT_BIEN, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Nhóm theo").size(12.0).color(Color32::from_rgb(0x2C, 0x3E, 0x55)).dam()).on_hover_text(goi_y)
            })
            .inner;
        let hien = cot.iter().position(|c| c == hien_tai).map_or_else(|| hien_tai.to_string(), |i| ten(i, hien_tai));
        ui.scope(|ui| {
            super::kieu_o_chon(ui, Color32::from_rgb(0xE1, 0xE8, 0xF0), Color32::TRANSPARENT);
            let rong = (ui.available_width() - 6.0).max(40.0);
            egui::ComboBox::from_id_salt("ml_nhom_theo")
                .icon(super::mui_ten_chon)
                .width(rong)
                .selected_text(RichText::new(hien).size(12.0).color(Color32::from_rgb(0x2C, 0x3E, 0x55)))
                .show_ui(ui, |ui| {
                    for (i, c) in cot.iter().enumerate().filter(|(_, c)| !c.trim().is_empty()) {
                        if ui.selectable_label(c == hien_tai, ten(i, c)).clicked() {
                            chon = Some(c.clone());
                        }
                    }
                })
                .response
                .labelled_by(nhan.id)
                .on_hover_text(goi_y);
        });
    });
    chon
}

use crate::gui::ChuDam;
