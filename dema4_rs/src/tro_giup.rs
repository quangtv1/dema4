//! Menu Trợ giúp: Kiểm tra cập nhật (GitHub Releases), Hướng dẫn sử dụng (F1, 2 tab), Thông tin.

use std::cmp::Ordering;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui::{self, Align, Color32, Margin, Rect, RichText, Stroke};

use crate::gui::{ChuDam, CHU, CHU_PHU, XANH_LA};
use crate::gui_tach::khung_hop as kh;

const API_PHAT_HANH: &str = "https://api.github.com/repos/quangtv1/dema4/releases/latest";
pub const TRANG_TAI: &str = "https://github.com/quangtv1/dema4/releases/latest";
pub const PHIEN_BAN: &str = env!("CARGO_PKG_VERSION");
pub const WEBSITE: &str = "https://saomaiso.com";

/// Logo Sao Mai (PNG nhúng trong exe), nạp texture 1 lần rồi giữ trong bộ nhớ egui.
fn logo(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::unique("logo_sao_mai");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let anh = eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo-sao-mai.png")).expect("logo PNG hợp lệ");
    let t = ctx.load_texture("logo_sao_mai", egui::ColorImage::from_rgba_unmultiplied([anh.width as usize, anh.height as usize], &anh.rgba), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}
const RONG: f32 = 560.0;
/// Popup gọn (Cập nhật, Thông tin).
const RONG_GON: f32 = 340.0;
const CAO_THAN: f32 = 430.0;

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
        let ph = match self {
            HopTroGiup::CapNhat { nhan, kq } => {
                if kq.is_none() {
                    *kq = nhan.try_recv().ok();
                    ctx.request_repaint_after(Duration::from_millis(200));
                }
                let (moi_nhat, co_moi) = match kq {
                    None => ("đang kiểm tra…".to_string(), false),
                    Some(Err(_)) => ("không kiểm tra được (mất mạng hoặc GitHub không trả lời)".to_string(), false),
                    Some(Ok(tag)) => {
                        let moi = so_phien_ban(tag, PHIEN_BAN) == Ordering::Greater;
                        (format!("{}{}", tag.trim_start_matches('v'), if moi { " — có bản mới" } else { " — đang dùng bản mới nhất" }), moi)
                    }
                };
                kh::hop(ctx, "hop_tro_giup", RONG_GON, "Kiểm tra cập nhật", |ui| {
                    kh::bang_gon(ui, &[("Đang dùng", PHIEN_BAN, true, 12.0), ("Mới nhất", &moi_nhat, true, 12.0)]);
                    kh::ghi_gon(ui, "Tải từ trang phát hành của sm-dema4 (GitHub); không gửi dữ liệu nào khác.", CHU_PHU);
                    kh::chan(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kh::nut_chan("Đóng")).clicked() {
                                dong = true;
                            }
                            if ui.add_enabled(co_moi, kh::nut_chan_mau("Tải bản mới", XANH_LA)).clicked() {
                                ctx.open_url(egui::OpenUrl::new_tab(TRANG_TAI));
                            }
                        });
                    });
                })
            }
            HopTroGiup::HuongDan(tab) => kh::hop(ctx, "hop_tro_giup", RONG, "Hướng dẫn sử dụng", |ui| {
                kh::hang_tab(ui, &[(TabHuongDan::Dem, "Đếm trang A4"), (TabHuongDan::Tach, "Tách văn bản")], tab);
                kh::than(ui, "than_huong_dan", CAO_THAN, Margin { left: 22, right: 22, top: 4, bottom: 16 }, |ui| {
                    ve_khoi(ui, if *tab == TabHuongDan::Dem { HD_DEM } else { HD_TACH });
                });
                kh::chan(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(kh::nut_chan("Đóng")).clicked() {
                            dong = true;
                        }
                    });
                });
            }),
            HopTroGiup::ThongTin => kh::hop(ctx, "hop_tro_giup", RONG_GON, "Thông tin", |ui| {
                // Logo đơn vị (bỏ slogan) ở đầu hộp
                egui::Frame::new().inner_margin(Margin { left: 16, right: 16, top: 16, bottom: 0 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical_centered(|ui| {
                        let tex = logo(ctx);
                        ui.add(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(150.0, 150.0 * tex.size()[1] as f32 / tex.size()[0] as f32)));
                    });
                });
                kh::bang_gon(
                    ui,
                    &[
                        ("Ứng dụng", "sm-dema4", true, 12.0),
                        ("Phiên bản", PHIEN_BAN, true, 12.0),
                        ("Chức năng", "Đếm trang A4 · Tách văn bản PDF", false, 12.0),
                        ("Đơn vị", "Giải pháp số Sao Mai", false, 12.0),
                        ("Website", WEBSITE, false, 12.0),
                        ("Thư viện", "MuPDF (AGPL) · zxing-cpp (Apache 2.0) · ONNX Runtime (MIT) · mô hình PP-LCNet_x1_0_doc_ori (Apache 2.0) · egui", false, 11.5),
                    ],
                );
                kh::ghi_gon(ui, "Ứng dụng gửi thống kê đếm/tách (tên máy, thư mục, số liệu) về nhóm quản lý.", CHU_PHU);
                kh::chan(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(kh::nut_chan("Đóng")).clicked() {
                            dong = true;
                        }
                    });
                });
            }),
        };
        dong || ph.should_close()
    }
}

/// Khối nội dung hướng dẫn. Trong chuỗi: `*đậm*`, `~nghiêng~`, `[phím]`, `` `mã` ``.
/// Cột tên của `Bang`: bắt đầu `#` = dòng tiêu đề nhóm; `{s}` `{i}` `{b}` `{.}` `{nghi}` `{xoay}` = huy hiệu nhãn.
/// `Muc("Tiêu đề|chú thích")`: phần sau `|` là chú thích nhỏ.
pub enum Khoi {
    Muc(&'static str),
    Buoc(&'static [&'static str]),
    Bang(&'static [(&'static str, &'static str)]),
    Ghi(&'static str),
}

const HD_DEM: &[Khoi] = &[
    Khoi::Muc("Các bước"),
    Khoi::Buoc(&[
        "*Chọn thư mục* chứa PDF (quét cả thư mục con). Tuỳ chọn: *Check DPI*, *Check ký số* (chậm hơn).",
        "Bấm *Đếm*. Bảng hiện từng trang: khổ, hướng, DPI, ký số, cảnh báo. *Dừng* để ngừng giữa chừng.",
        "*Lọc* bằng hàng tab trên bảng (Tất cả, A0–A7) hoặc bấm số *Cảnh báo* · *File lỗi* · *Ký số*. Bấm tên file để mở PDF.",
        "Xong: tự lưu `<thư mục>_dem.txt` và `_dem_chitiet.txt`; popup có *Mở kết quả* · *Copy dữ liệu*.",
    ]),
    Khoi::Muc("Cách quy ra A4"),
    Khoi::Bang(&[
        ("Xếp khổ", "Theo *cạnh dài* của trang, không phụ thuộc dọc/ngang."),
        ("Quy A4", "Σ số trang mỗi khổ × hệ số. Vd 10 A3 + 5 A4 = 10×2 + 5×1 = *25*."),
        ("Sát ngưỡng", "Cạnh dài đã đi ≥ 80 % quãng tới khổ kề bên → nên mở file xem lại."),
    ]),
    Khoi::Muc("Cấu hình › Đếm trang A4"),
    Khoi::Bang(&[
        ("Hệ số quy A4", "A0–A7, mặc định 16 · 8 · 4 · 2 · 1 · 0,5 · 0,25 · 0,125."),
        ("Lớn hơn A0", "Cạnh dài > 1399 mm: ~theo diện tích ÷ A4~ (vd 1500×900 mm ≈ 21,64) hoặc ~hệ số cố định~ ≥ hệ số A0."),
        ("Ranh giới khổ", "Kéo vạch trên thước (mặc định ở giữa 2 khổ, vd A3|A4 = 358,5 mm). Gõ *Thử cạnh dài* để xem trang thuộc khổ nào."),
        ("Lưu · Mặc định", "Áp cho lần Đếm sau (`sm-dema4.cfg` cạnh file chạy). Ranh giới khác mặc định được ghi trong `_dem.txt`."),
    ]),
];

const HD_TACH: &[Khoi] = &[
    Khoi::Muc("Các bước"),
    Khoi::Buoc(&[
        "*Chọn thư mục* hồ sơ PDF → *Phân tích*. Lưới hiện ngay, nhãn tự động đến dần trong lúc chạy.",
        "Tab *Hồ sơ*: kiểm tra nhãn, sửa bằng [S] [I] [B]. Chọn trang như Explorer; sắp ~Dung lượng~ để dồn trang index/trắng.",
        "Duyệt hết *Nghi trắng* và *Nghi xoay* (menu lọc) — bắt buộc trước khi tách.",
        "Tab *Văn bản*: xem trước từng văn bản (Lưới · 1 trang · 2 trang); xoay hoặc *Bỏ khỏi văn bản* nếu cần.",
        "*Tách* → `<gốc>_vanban`, giữ nguyên cấu trúc thư mục; mỗi hồ sơ một thư mục chứa `<hồ sơ>.1.pdf`…",
    ]),
    Khoi::Muc("Nhãn trang"),
    Khoi::Bang(&[
        ("{s}Split", "Văn bản mới bắt đầu từ trang này, trang được giữ. Dùng khi thiếu tờ index."),
        ("{i}Index", "Tờ mã vạch phân cách: điểm cắt, bị bỏ khi tách."),
        ("{b}Trắng", "Trang trắng, bìa, chữ thấm mặt sau: bị bỏ khi tách."),
        ("{.}Nội dung", "Trang chưa gắn nhãn."),
    ]),
    Khoi::Muc("Duyệt trang Nghi|chỉ có tác dụng trong bộ lọc tương ứng"),
    Khoi::Bang(&[
        ("{nghi}Nghi trắng", "Chọn nhãn thật: [B] Trắng (mặc định) · [C] Nội dung · [I] Index · [S] Split, rồi [Enter] áp → tự sang trang nghi tiếp. Phóng to 1 trang vẫn duyệt được như vậy."),
        ("{xoay}Nghi xoay", "*1.* [Enter] xoay thử · *2.* sai thì [R] / [Shift+R] xoay tay, [C] về như cũ · *3.* [Enter] đúng rồi, tiếp."),
    ]),
    Khoi::Muc("Phím tắt"),
    Khoi::Bang(&[
        ("#Chọn trang · cả 2 tab", ""),
        ("Chuột", "bấm · [Ctrl] + bấm thêm/bớt · [Shift] + bấm chọn dải · kéo khung"),
        ("Bàn phím", "[←] [→] [↑] [↓] [Home] [End] · [PgUp] [PgDn] 1 vùng nhìn (tab Hồ sơ) · [Ctrl+A] chọn hết · [Esc] bỏ chọn"),
        ("Xem", "[Enter] hoặc bấm đúp: xem lớn · chuột phải: menu thao tác"),
        ("#Sửa", ""),
        ("Nhãn", "[S] [I] [B] gán / bấm lại để huỷ · [C] bỏ nhãn"),
        ("Xoay · xoá", "[R] 90° · [Shift+R] 180° · [Del] xoá khỏi lưới / bỏ khỏi văn bản"),
        ("#Chung", ""),
        ("Hoàn tác", "[Ctrl+Z] hoàn tác · [Ctrl+Y] làm lại — riêng từng nhóm lọc"),
        ("Khác", "[Ctrl+S] lưu nhãn · [Ctrl] + cuộn hoặc nút − / + cỡ ảnh · [PgUp] [PgDn] văn bản trước/sau · [F1] hướng dẫn"),
    ]),
    Khoi::Ghi("Nhãn, góc xoay, log và cache lưu trong thư mục ẩn `.sm-dema4` cạnh file ứng dụng (mỗi thư mục đầu vào 1 thư mục con riêng); thư mục đầu vào và file PDF gốc không bị sửa. *Phân tích lại* = làm lại từ đầu."),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KieuDoan {
    Thuong,
    Dam,
    Nghieng,
    Phim,
    Ma,
}

/// Tách chuỗi thành đoạn (chữ, kiểu). Trong `[phím]` và `` `mã` `` không hiểu dấu định dạng khác.
pub fn tach_doan(s: &str) -> Vec<(String, KieuDoan)> {
    let mut kq = Vec::new();
    let mut hien = String::new();
    let mut kieu = KieuDoan::Thuong;
    let mut chu = s.chars();
    let day = |hien: &mut String, kieu: KieuDoan, kq: &mut Vec<(String, KieuDoan)>| {
        if !hien.is_empty() {
            kq.push((std::mem::take(hien), kieu));
        }
    };
    while let Some(c) = chu.next() {
        match c {
            '*' | '~' => {
                day(&mut hien, kieu, &mut kq);
                let k = if c == '*' { KieuDoan::Dam } else { KieuDoan::Nghieng };
                kieu = if kieu == k { KieuDoan::Thuong } else { k };
            }
            '[' | '`' => {
                day(&mut hien, kieu, &mut kq);
                let dong = if c == '[' { ']' } else { '`' };
                let trong: String = chu.by_ref().take_while(|&x| x != dong).collect();
                kq.push((trong, if c == '[' { KieuDoan::Phim } else { KieuDoan::Ma }));
            }
            c => hien.push(c),
        }
    }
    day(&mut hien, kieu, &mut kq);
    kq
}

const CO_CHU: f32 = 12.5;
/// line-height 1.6 của prototype.
const CAO_DONG: f32 = CO_CHU * 1.6;
const XANH_BUOC: Color32 = Color32::from_rgb(0x2F, 0x7B, 0xD9);

/// 1 đoạn tự xuống dòng; phím vẽ khung viền (đáy dày 2 px), mã nền xám — tính vị trí theo glyph sau khi dàn trang.
pub(crate) fn ve_doan(ui: &mut egui::Ui, s: &str, co: f32, mau: Color32) {
    use egui::text::{LayoutJob, TextFormat};
    let cao_dong = co * 1.6;
    let dam = egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into());
    let mut job = LayoutJob { wrap: egui::text::TextWrapping { max_width: ui.available_width(), ..Default::default() }, ..Default::default() };
    // (chỉ số ký tự đầu, số ký tự, kiểu) của các đoạn cần vẽ nền
    let mut nen: Vec<(usize, usize, KieuDoan)> = Vec::new();
    let (mut so_ky_tu, mut cach_truoc) = (0usize, 0.0f32);
    for (chu, kieu) in tach_doan(s) {
        let dinh_dang = match kieu {
            KieuDoan::Phim => TextFormat { font_id: egui::FontId::proportional(10.5), color: CHU, line_height: Some(cao_dong), valign: Align::Center, ..Default::default() },
            KieuDoan::Ma => TextFormat { font_id: egui::FontId::monospace(11.5), color: CHU, line_height: Some(cao_dong), valign: Align::Center, ..Default::default() },
            KieuDoan::Dam => TextFormat { font_id: egui::FontId::new(co, dam.clone()), color: CHU, line_height: Some(cao_dong), ..Default::default() },
            KieuDoan::Nghieng => TextFormat { font_id: egui::FontId::proportional(co), color: mau, italics: true, line_height: Some(cao_dong), ..Default::default() },
            KieuDoan::Thuong => TextFormat { font_id: egui::FontId::proportional(co), color: mau, line_height: Some(cao_dong), ..Default::default() },
        };
        let n = chu.chars().count();
        let le = match kieu {
            KieuDoan::Phim => 10.0,
            KieuDoan::Ma => 4.0,
            _ => 0.0,
        };
        if le > 0.0 {
            nen.push((so_ky_tu, n, kieu));
        }
        job.append(&chu, cach_truoc + le, dinh_dang);
        cach_truoc = le;
        so_ky_tu += n;
    }
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let (r, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    // Vị trí từng ký tự: đếm glyph tuần tự qua các dòng (chỉ dùng khi khớp số ký tự)
    let mut vi_tri: Vec<(f32, f32, f32)> = Vec::new(); // (x trái, x phải, tâm y)
    for dong in &galley.rows {
        let cy = r.top() + dong.pos.y + dong.row.size.y / 2.0;
        for g in &dong.row.glyphs {
            let x = r.left() + dong.pos.x + g.pos.x;
            vi_tri.push((x, x + g.advance_width, cy));
        }
    }
    let p = ui.painter();
    if vi_tri.len() == so_ky_tu {
        for (dau, n, kieu) in nen {
            // Đoạn có thể bị ngắt dòng: vẽ từng mảnh theo dòng
            let mut i = dau;
            while i < dau + n {
                let cy = vi_tri[i].2;
                let mut j = i;
                while j + 1 < dau + n && vi_tri[j + 1].2 == cy {
                    j += 1;
                }
                let (x0, x1) = (vi_tri[i].0, vi_tri[j].1);
                match kieu {
                    KieuDoan::Phim => {
                        // min-width 12 + padding 5 mỗi bên + viền; đáy viền dày 2 px
                        let (giua, rong) = ((x0 + x1) / 2.0, (x1 - x0).max(12.0) + 12.0);
                        let o = Rect::from_x_y_ranges((giua - rong / 2.0)..=(giua + rong / 2.0), (cy - 8.0)..=(cy + 9.0));
                        p.rect(o, 3.0, Color32::from_gray(0xFA), Stroke::new(1.0, Color32::from_gray(0xCF)), egui::StrokeKind::Inside);
                        p.hline(o.x_range().shrink(2.0), o.bottom() - 1.5, Stroke::new(1.0, Color32::from_gray(0xCF)));
                    }
                    _ => {
                        p.rect_filled(Rect::from_x_y_ranges((x0 - 4.0)..=(x1 + 4.0), (cy - 8.0)..=(cy + 8.0)), 2.0, Color32::from_gray(0xF2));
                    }
                }
                i = j + 1;
            }
        }
    }
    p.galley(r.min, galley, CHU);
}

/// Huy hiệu nhãn 18×18 trước tên dòng bảng (S / I / B / · / Nghi trắng / Nghi xoay).
fn huy_hieu(ui: &mut egui::Ui, ma: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    let p = ui.painter();
    let o = r.shrink(0.5);
    let duong = |p: &egui::Painter, mau: Color32, nen: Color32| {
        p.rect_filled(o, 3.0, nen);
        let canh = [o.left_top(), o.right_top(), o.right_bottom(), o.left_bottom(), o.left_top()];
        p.extend(egui::Shape::dashed_line(&canh, Stroke::new(1.0, mau), 3.0, 2.0));
    };
    let chu = |p: &egui::Painter, s: &str, mau: Color32| {
        p.text(r.center(), egui::Align2::CENTER_CENTER, s, egui::FontId::proportional(11.0), mau);
    };
    match ma {
        "s" => {
            p.rect(o, 3.0, Color32::WHITE, Stroke::new(1.0, Color32::from_rgb(0xE0, 0x7A, 0x00)), egui::StrokeKind::Inside);
            chu(p, "S", Color32::from_rgb(0xB8, 0x5F, 0x00));
        }
        "i" => {
            p.rect(r, 3.0, Color32::WHITE, Stroke::new(2.0, crate::gui::XANH_COBALT), egui::StrokeKind::Inside);
            chu(p, "I", crate::gui::XANH_COBALT);
        }
        "b" => {
            duong(p, Color32::from_gray(0x8A), Color32::from_rgb(0xE6, 0xF0, 0xF8));
            chu(p, "B", CHU);
        }
        "nghi" => {
            duong(p, Color32::from_rgb(0xD9, 0xA4, 0x00), Color32::from_rgb(0xFF, 0xFC, 0xEB));
            chu(p, "?", Color32::from_rgb(0x9A, 0x74, 0x00));
        }
        "xoay" => {
            duong(p, Color32::from_rgb(0xC2, 0x5E, 0x00), Color32::from_rgb(0xFF, 0xF6, 0xEC));
            crate::gui_tach::bieu_tuong::ve(p, Rect::from_center_size(r.center(), egui::vec2(11.0, 11.0)), crate::gui_tach::bieu_tuong::BieuTuong::XoayPhai, Color32::from_rgb(0xB8, 0x5F, 0x00));
        }
        _ => {
            p.rect(o, 3.0, Color32::WHITE, Stroke::new(1.0, Color32::from_gray(0xBD)), egui::StrokeKind::Inside);
            chu(p, "·", CHU);
        }
    }
}

pub fn ve_khoi(ui: &mut egui::Ui, khoi: &[Khoi]) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let ke = |ui: &mut egui::Ui, mau: u8| crate::gui_tach::khung_hop::ke_ngang(ui, Color32::from_gray(mau));
    for k in khoi {
        match k {
            Khoi::Muc(t) => {
                ui.add_space(16.0);
                let (ten, phu) = t.split_once('|').unwrap_or((t, ""));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ten).dam().size(CO_CHU));
                    if !phu.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(phu).size(11.5).color(CHU_PHU));
                    }
                });
                ui.add_space(4.0);
                ke(ui, 0xE6);
                ui.add_space(8.0);
            }
            Khoi::Buoc(ds) => {
                for (i, b) in ds.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 10.0, XANH_BUOC);
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::new(11.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), Color32::WHITE);
                        ui.add_space(10.0);
                        ui.vertical(|ui| ve_doan(ui, b, CO_CHU, CHU));
                    });
                    ui.add_space(7.0);
                }
            }
            Khoi::Bang(dong) => {
                for (i, (ten, mo_ta)) in dong.iter().enumerate() {
                    if let Some(nhom) = ten.strip_prefix('#') {
                        ui.add_space(if i == 0 { 2.0 } else { 10.0 });
                        ui.label(RichText::new(nhom).dam().size(11.5).color(CHU_PHU));
                        ui.add_space(3.0);
                        ke(ui, 0xE6);
                        continue;
                    }
                    ui.add_space(5.0);
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(egui::vec2(132.0, CAO_DONG), egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.set_min_width(132.0);
                            let ten = match ten.strip_prefix('{').and_then(|s| s.split_once('}')) {
                                Some((ma, con)) => {
                                    huy_hieu(ui, ma);
                                    ui.add_space(6.0);
                                    con
                                }
                                None => ten,
                            };
                            ui.label(RichText::new(ten).dam().size(CO_CHU));
                        });
                        ui.vertical(|ui| ve_doan(ui, mo_ta, CO_CHU, Color32::from_gray(0x44)));
                    });
                    ui.add_space(5.0);
                    ke(ui, 0xF2);
                }
            }
            Khoi::Ghi(t) => {
                ui.add_space(14.0);
                let khung = egui::Frame::new().fill(Color32::from_rgb(0xF6, 0xF8, 0xFB)).inner_margin(Margin { left: 13, right: 10, top: 8, bottom: 8 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ve_doan(ui, t, 11.5, CHU_PHU);
                });
                let r = khung.response.rect;
                ui.painter().rect_filled(Rect::from_min_size(r.min, egui::vec2(3.0, r.height())), 0.0, XANH_BUOC);
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
        use KieuDoan::*;
        assert_eq!(
            tach_doan("bấm [Enter] để *xem* ~lớn~ `<gốc>_vanban`"),
            [("bấm ".into(), Thuong), ("Enter".into(), Phim), (" để ".into(), Thuong), ("xem".into(), Dam), (" ".into(), Thuong), ("lớn".into(), Nghieng), (" ".into(), Thuong), ("<gốc>_vanban".into(), Ma)]
        );
    }
}
