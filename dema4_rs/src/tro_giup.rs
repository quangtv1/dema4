//! Menu Trợ giúp: Kiểm tra cập nhật (GitHub Releases), Hướng dẫn sử dụng (F1, 2 tab), Thông tin.

use std::cmp::Ordering;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui::{self, Align, Color32, Margin, RichText, Stroke};

use crate::gui::{nut_mau, ChuDam, CHU, CHU_PHU, VIEN_NHAT, XANH_LA};

const API_PHAT_HANH: &str = "https://api.github.com/repos/quangtv1/dema4/releases/latest";
pub const TRANG_TAI: &str = "https://github.com/quangtv1/dema4/releases/latest";
pub const PHIEN_BAN: &str = env!("CARGO_PKG_VERSION");
const RONG: f32 = 560.0;
/// Chiều cao hộp ngoài phần thân cuộn (lề, tiêu đề, tab, nút) + khoảng hở trên/dưới.
const PHAN_NGOAI_THAN: f32 = 190.0;
const CAO_THAN: f32 = 440.0;

/// So 2 phiên bản dạng `v1.2.3` / `1.2.3` theo từng số.
pub fn so_phien_ban(a: &str, b: &str) -> Ordering {
    let so = |s: &str| -> Vec<u64> { s.trim().trim_start_matches(['v', 'V']).split(['.', '-']).map_while(|p| p.parse().ok()).collect() };
    let (a, b) = (so(a), so(b));
    for i in 0..a.len().max(b.len()) {
        match a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0)) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    Ordering::Equal
}

/// Lấy `tag_name` trong JSON phát hành (không cần thư viện JSON).
pub fn doc_tag(json: &str) -> Option<String> {
    let sau = &json[json.find("\"tag_name\"")? + "\"tag_name\"".len()..];
    let sau = &sau[sau.find('"')? + 1..];
    Some(sau[..sau.find('"')?].to_string())
}

/// Hỏi GitHub bản mới nhất (luồng nền, timeout 10 s). Chỉ gửi yêu cầu tới GitHub, không gửi dữ liệu người dùng.
fn hoi_ban_moi() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
    let mut ph = agent.get(API_PHAT_HANH).header("User-Agent", "sm-dema4").header("Accept", "application/vnd.github+json").call().map_err(|e| e.to_string())?;
    let than = ph.body_mut().read_to_string().map_err(|e| e.to_string())?;
    doc_tag(&than).ok_or_else(|| "không đọc được phiên bản".into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabHuongDan {
    Dem,
    Tach,
}

pub enum HopTroGiup {
    CapNhat { nhan: Receiver<Result<String, String>>, kq: Option<Result<String, String>> },
    HuongDan(TabHuongDan),
    ThongTin,
}

impl HopTroGiup {
    pub fn cap_nhat() -> Self {
        let (gui, nhan) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = gui.send(hoi_ban_moi());
        });
        HopTroGiup::CapNhat { nhan, kq: None }
    }

    /// Vẽ hộp; trả true khi đóng.
    pub fn ve(&mut self, ctx: &egui::Context) -> bool {
        let mut dong = false;
        let khung = egui::Frame::popup(&ctx.global_style()).inner_margin(Margin::same(14)).fill(Color32::WHITE);
        let ph = egui::Modal::new(egui::Id::unique("hop_tro_giup")).frame(khung).show(ctx, |ui| match self {
            HopTroGiup::CapNhat { nhan, kq } => {
                if kq.is_none() {
                    *kq = nhan.try_recv().ok();
                    ctx.request_repaint_after(Duration::from_millis(200));
                }
                ui.set_width(340.0);
                ui.label(RichText::new("Kiểm tra cập nhật").dam());
                ui.add_space(6.0);
                ui.label(format!("Đang dùng {PHIEN_BAN}"));
                let co_moi = match kq {
                    None => {
                        ui.label("Mới nhất: đang kiểm tra…");
                        false
                    }
                    Some(Err(_)) => {
                        ui.label("Không kiểm tra được (mất mạng hoặc GitHub không trả lời)");
                        false
                    }
                    Some(Ok(tag)) => {
                        let moi = so_phien_ban(tag, PHIEN_BAN) == Ordering::Greater;
                        ui.label(if moi { format!("Mới nhất {} — có bản mới", tag.trim_start_matches('v')) } else { format!("Mới nhất {} — đang dùng bản mới nhất", tag.trim_start_matches('v')) });
                        moi
                    }
                };
                ui.label(RichText::new("Tải từ trang phát hành của sm-dema4 (GitHub); không gửi dữ liệu nào khác.").small().color(CHU_PHU));
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Đóng").min_size(egui::vec2(70.0, 24.0))).clicked() {
                        dong = true;
                    }
                    if co_moi && ui.add(nut_mau("Tải bản mới", XANH_LA).min_size(egui::vec2(90.0, 24.0))).clicked() {
                        ctx.open_url(egui::OpenUrl::new_tab(TRANG_TAI));
                    }
                });
            }
            HopTroGiup::HuongDan(tab) => {
                ui.set_width(RONG);
                ui.label(RichText::new("Hướng dẫn sử dụng").dam().size(15.0));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for (t, ten) in [(TabHuongDan::Dem, "Đếm trang A4"), (TabHuongDan::Tach, "Tách văn bản")] {
                        if ui.selectable_label(*tab == t, ten).clicked() {
                            *tab = t;
                        }
                    }
                });
                ui.separator();
                // Cửa sổ thấp thì thân hộp co lại (cuộn) để nút Lưu/Đóng không tràn ra ngoài
                let cao_than = CAO_THAN.min(ctx.content_rect().height() - PHAN_NGOAI_THAN).max(160.0);
                egui::ScrollArea::vertical().id_salt("than_huong_dan").max_height(cao_than).min_scrolled_height(cao_than).auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_min_height(cao_than);
                    ve_khoi(ui, if *tab == TabHuongDan::Dem { HD_DEM } else { HD_TACH });
                });
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Đóng").min_size(egui::vec2(70.0, 24.0))).clicked() {
                        dong = true;
                    }
                });
            }
            HopTroGiup::ThongTin => {
                ui.set_width(400.0);
                ui.label(RichText::new("Thông tin").dam());
                ui.add_space(6.0);
                ve_khoi(
                    ui,
                    &[Khoi::Bang(&[
                        ("Ứng dụng", "sm-dema4"),
                        ("Phiên bản", PHIEN_BAN),
                        ("Chức năng", "Đếm trang A4 · Tách văn bản PDF"),
                        ("Đơn vị", "Giải pháp số Sao Mai"),
                        ("Thư viện", "MuPDF (AGPL) · zxing-cpp (Apache 2.0) · ONNX Runtime (MIT) · mô hình PP-LCNet_x1_0_doc_ori (Apache 2.0) · egui"),
                    ])],
                );
                ui.add_space(4.0);
                ui.label(RichText::new("Ứng dụng gửi thống kê đếm / tách (tên máy, thư mục, số liệu) về nhóm quản lý.").small().color(CHU_PHU));
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Đóng").min_size(egui::vec2(70.0, 24.0))).clicked() {
                        dong = true;
                    }
                });
            }
        });
        dong || ph.should_close()
    }
}

/// Khối nội dung hướng dẫn. Trong chuỗi: `[phím]` vẽ dạng nút phím, `*đậm*` in đậm.
pub enum Khoi {
    Muc(&'static str),
    Buoc(&'static [&'static str]),
    Bang(&'static [(&'static str, &'static str)]),
    Ghi(&'static str),
}

const HD_DEM: &[Khoi] = &[
    Khoi::Muc("Các bước"),
    Khoi::Buoc(&[
        "Chọn thư mục chứa PDF (quét cả thư mục con). Tuỳ chọn: *Check DPI*, *Check ký số* (chậm hơn).",
        "Bấm *Đếm*. Bảng hiện từng trang: khổ, hướng, DPI, ký số, cảnh báo. *Dừng* để ngừng giữa chừng.",
        "Lọc bằng hàng tab trên bảng (Tất cả, A0–A7) hoặc bấm số *Cảnh báo · File lỗi · Ký số*. Bấm tên file để mở PDF.",
        "Xong: tự lưu *<thư mục>_dem.txt* và *_dem_chitiet.txt*; popup có *Mở kết quả · Copy dữ liệu*.",
    ]),
    Khoi::Muc("Cách quy ra A4"),
    Khoi::Bang(&[
        ("Xếp khổ", "Theo cạnh dài của trang, không phụ thuộc dọc / ngang."),
        ("Quy A4", "Σ số trang mỗi khổ × hệ số. Vd 10 A3 + 5 A4 = 10×2 + 5×1 = *25*."),
        ("Sát ngưỡng", "Cạnh dài đã đi ≥ 80 % quãng tới ranh giới khổ kề bên → nên mở file xem lại."),
    ]),
    Khoi::Muc("Cấu hình › Đếm trang A4"),
    Khoi::Bang(&[
        ("Hệ số", "A0–A7, mặc định 16 · 8 · 4 · 2 · 1 · 0,5 · 0,25 · 0,125."),
        ("Lớn hơn A0", "Cạnh dài > 1399 mm: theo diện tích ÷ A4 (vd 1500×900 mm ≈ 21,64) hoặc hệ số cố định ≥ hệ số A0."),
        ("Ranh giới khổ", "Kéo vạch trên thước (mặc định ở giữa 2 khổ, vd A3|A4 = 358,5 mm). Gõ *Thử cạnh dài* để xem trang thuộc khổ nào."),
        ("Lưu · Mặc định", "Áp cho lần Đếm sau (*sm-dema4.cfg* cạnh file chạy). Ranh giới khác mặc định được ghi trong *_dem.txt*."),
    ]),
];

const HD_TACH: &[Khoi] = &[
    Khoi::Muc("Các bước"),
    Khoi::Buoc(&[
        "Chọn thư mục hồ sơ PDF → *Phân tích*. Lưới hiện ngay, nhãn tự động đến dần trong lúc chạy.",
        "Tab *Hồ sơ*: kiểm tra nhãn, sửa bằng [S] [I] [B]. Chọn trang như Explorer; sắp *Dung lượng* để dồn trang index / trắng.",
        "Duyệt hết *Nghi trắng* và *Nghi xoay* (menu lọc) — bắt buộc trước khi tách.",
        "Tab *Văn bản*: xem trước từng văn bản (Lưới · 1 trang · 2 trang); xoay hoặc *Bỏ khỏi văn bản* nếu cần.",
        "*Tách* → *<gốc>_vanban*, giữ nguyên cấu trúc thư mục; mỗi hồ sơ một thư mục chứa *<hồ sơ>_part1.pdf* …",
    ]),
    Khoi::Muc("Nhãn trang"),
    Khoi::Bang(&[
        ("[S] Split", "Văn bản mới bắt đầu từ trang này, trang được giữ. Dùng khi thiếu tờ index."),
        ("[I] Index", "Tờ mã vạch phân cách: điểm cắt, bị bỏ khi tách."),
        ("[B] Trắng", "Trang trắng, bìa, chữ thấm mặt sau: bị bỏ khi tách."),
        ("Nội dung", "Trang chưa gắn nhãn."),
    ]),
    Khoi::Muc("Duyệt trang Nghi (chỉ trong bộ lọc tương ứng)"),
    Khoi::Bang(&[
        ("Nghi trắng", "[Enter] / [B] Trắng · [C] giữ nội dung · [I] Index → tự sang trang tiếp."),
        ("Nghi xoay", "1. [Enter] xoay đề xuất · 2. sai thì [R] / [Shift+R] chỉnh, [C] về gốc · 3. [Enter] xác nhận và tiếp."),
    ]),
    Khoi::Muc("Phím tắt"),
    Khoi::Bang(&[
        ("Chọn trang", "bấm · [Ctrl] + bấm thêm / bớt · [Shift] + bấm chọn dải · kéo khung"),
        ("Bàn phím", "[←] [→] [↑] [↓] [Home] [End] · [Ctrl+A] chọn hết · [Esc] bỏ chọn"),
        ("Xem", "[Enter] hoặc bấm đúp: xem lớn · chuột phải: menu thao tác"),
        ("Nhãn", "[S] [I] [B] gán / bấm lại để huỷ · [C] bỏ nhãn"),
        ("Xoay · xoá", "[R] 90° · [Shift+R] 180° · [Del] xoá khỏi lưới / bỏ khỏi văn bản"),
        ("Hoàn tác", "[Ctrl+Z] hoàn tác · [Ctrl+Y] làm lại — riêng từng nhóm lọc"),
        ("Khác", "[Ctrl+S] lưu nhãn · [Ctrl] + cuộn hoặc − / + cỡ ảnh · [PgUp] [PgDn] văn bản trước / sau · [F1] hướng dẫn"),
    ]),
    Khoi::Ghi("Nhãn, góc xoay và log lưu trong thư mục ẩn *.sm-dema4* của thư mục gốc; file PDF gốc không bị sửa. *Phân tích lại* = làm lại từ đầu."),
];

/// Tách chuỗi thành đoạn: (chữ, đậm, phím).
pub fn tach_doan(s: &str) -> Vec<(String, bool, bool)> {
    let mut kq = Vec::new();
    let mut hien = String::new();
    let mut dam = false;
    let mut chu = s.chars().peekable();
    while let Some(c) = chu.next() {
        match c {
            '*' => {
                if !hien.is_empty() {
                    kq.push((std::mem::take(&mut hien), dam, false));
                }
                dam = !dam;
            }
            '[' => {
                if !hien.is_empty() {
                    kq.push((std::mem::take(&mut hien), dam, false));
                }
                let phim: String = chu.by_ref().take_while(|&c| c != ']').collect();
                kq.push((phim, false, true));
            }
            c => hien.push(c),
        }
    }
    if !hien.is_empty() {
        kq.push((hien, dam, false));
    }
    kq
}

/// 1 đoạn = 1 LayoutJob (tự xuống dòng): đậm dùng họ font đậm, phím có nền xám.
fn ve_doan(ui: &mut egui::Ui, s: &str) {
    use egui::text::{LayoutJob, TextFormat};
    let co = 12.5;
    let thuong = egui::FontId::proportional(co);
    let dam = egui::FontId::new(co, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into()));
    let mau = ui.visuals().override_text_color.unwrap_or(CHU);
    let mut job = LayoutJob { wrap: egui::text::TextWrapping { max_width: ui.available_width(), ..Default::default() }, ..Default::default() };
    for (chu, la_dam, phim) in tach_doan(s) {
        let dinh_dang = if phim {
            TextFormat { font_id: egui::FontId::proportional(11.5), color: CHU, background: Color32::from_gray(228), ..Default::default() }
        } else {
            TextFormat { font_id: if la_dam { dam.clone() } else { thuong.clone() }, color: mau, ..Default::default() }
        };
        let chu = if phim { format!("\u{2009}{chu}\u{2009}") } else { chu };
        job.append(&chu, 0.0, dinh_dang);
    }
    ui.label(job);
}

pub fn ve_khoi(ui: &mut egui::Ui, khoi: &[Khoi]) {
    for k in khoi {
        match k {
            Khoi::Muc(t) => {
                ui.add_space(8.0);
                let r = ui.label(RichText::new(*t).dam());
                ui.painter().hline(r.rect.left()..=ui.max_rect().right(), r.rect.bottom() + 2.0, Stroke::new(1.0, VIEN_NHAT));
                ui.add_space(4.0);
            }
            Khoi::Buoc(ds) => {
                for (i, b) in ds.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 8.5, crate::gui::XANH_COBALT);
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::proportional(11.0), Color32::WHITE);
                        ui.vertical(|ui| ve_doan(ui, b));
                    });
                }
            }
            Khoi::Bang(dong) => {
                for (ten, mo_ta) in dong.iter() {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(egui::vec2(120.0, 18.0), egui::Layout::left_to_right(Align::Min), |ui| {
                            ui.set_min_width(120.0);
                            ve_doan(ui, &format!("*{}*", ten.replace('*', "")));
                        });
                        ui.vertical(|ui| ve_doan(ui, mo_ta));
                    });
                }
            }
            Khoi::Ghi(t) => {
                ui.add_space(8.0);
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = Some(CHU_PHU);
                    ve_doan(ui, t);
                });
                ui.visuals_mut().override_text_color = Some(CHU);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_sanh_phien_ban() {
        assert_eq!(so_phien_ban("v0.3.0", "0.2.3"), Ordering::Greater);
        assert_eq!(so_phien_ban("0.2.3", "v0.2.3"), Ordering::Equal);
        assert_eq!(so_phien_ban("v0.2.10", "0.2.9"), Ordering::Greater);
        assert_eq!(so_phien_ban("v1.0", "1.0.1"), Ordering::Less);
    }

    #[test]
    fn doc_tag_tu_json_phat_hanh() {
        assert_eq!(doc_tag(r#"{"url":"x","tag_name": "v0.3.0","name":"y"}"#).as_deref(), Some("v0.3.0"));
        assert_eq!(doc_tag(r#"{"message":"Not Found"}"#), None);
    }

    #[test]
    fn tach_doan_dam_va_phim() {
        assert_eq!(
            tach_doan("bấm [Enter] để *xem* lớn"),
            [("bấm ".into(), false, false), ("Enter".into(), false, true), (" để ".into(), false, false), ("xem".into(), true, false), (" lớn".into(), false, false)]
        );
    }
}
