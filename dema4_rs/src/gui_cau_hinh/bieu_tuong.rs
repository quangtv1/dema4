//! Biểu tượng nét mảnh 16 px của trang Cấu hình — chép nguyên văn từ prototype đã chốt. Mỗi hình đăng ký 2 màu
//! (`<tên>` xám `#4b5563`, `<tên>_xanh` `#1f5fbf`) dưới `bytes://ch/<tên>.svg`.

use eframe::egui;

const XAM: &str = "#4b5563";
const XANH: &str = "#1f5fbf";

/// (tên, nội dung trong khung 16 × 16; `{m}` = màu nét).
const DS: &[(&str, &str)] = &[
    ("dem", r#"<path d="M4 1.8h5.5L13 5.3v8.9H4z"/><path d="M9.5 1.8v3.5H13"/><path d="M6.3 8.5h4.4M6.3 11h4.4"/>"#),
    ("tach", r#"<circle cx="4.5" cy="4.5" r="2"/><circle cx="4.5" cy="11.5" r="2"/><path d="M6.1 5.8l7.9 6.4M6.1 10.2l7.9-6.4"/>"#),
    ("muc_luc", r#"<path d="M5.5 4h8M5.5 8h8M5.5 12h8"/><circle cx="2.8" cy="4" r=".6" fill="{m}"/><circle cx="2.8" cy="8" r=".6" fill="{m}"/><circle cx="2.8" cy="12" r=".6" fill="{m}"/>"#),
    ("boc_tach", r#"<rect x="2" y="2" width="12" height="12" rx="1.2"/><path d="M5 5.5h6M5 8h6M5 10.5h3.5"/>"#),
    ("xoa_ghim", r#"<path d="M3.5 12.5l3-3"/><path d="M6 2.5l7.5 7.5M9.5 2l4.5 4.5-2.5 2.5L7 4.5z"/>"#),
    ("du_lieu", r#"<ellipse cx="8" cy="3.8" rx="5.5" ry="2"/><path d="M2.5 3.8v8.4c0 1.1 2.5 2 5.5 2s5.5-.9 5.5-2V3.8M2.5 8c0 1.1 2.5 2 5.5 2s5.5-.9 5.5-2"/>"#),
    ("quay_lai", r#"<path d="M10 3L5 8l5 5"/>"#),
    // Sidebar: khung có cột trái; thu nhỏ → cột tô đặc
    ("sidebar", r#"<rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M6 3v10"/>"#),
    ("sidebar_thu", r#"<rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M6 3v10"/><rect x="2.6" y="3.6" width="2.9" height="8.8" rx=".6" fill="{m}" stroke="none"/>"#),
    // Cửa sổ: ⧉ = thu nhỏ thành cửa sổ, □ = toàn cửa sổ
    ("cua_so_nho", r#"<rect x="2.5" y="5.5" width="8" height="8" rx="1.3"/><path d="M5.5 5.5V3.8a1.3 1.3 0 0 1 1.3-1.3h5.4a1.3 1.3 0 0 1 1.3 1.3v5.4a1.3 1.3 0 0 1-1.3 1.3h-1.7"/>"#),
    ("toan_cua_so", r#"<rect x="3" y="3" width="10" height="10" rx="1.5"/>"#),
    ("dong", r#"<path d="M4 4l8 8M12 4l-8 8"/>"#),
    ("banh_rang", r#"<circle cx="8" cy="8" r="2.4"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/>"#),
];

fn uri(ten: &str, xanh: bool) -> String {
    format!("bytes://ch/{ten}{}.svg", if xanh { "_xanh" } else { "" })
}

/// Đăng ký mọi biểu tượng (gọi 1 lần khi dựng giao diện).
pub fn dang_ky(ctx: &egui::Context) {
    for (ten, than) in DS {
        for xanh in [false, true] {
            let m = if xanh { XANH } else { XAM };
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="{m}" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
                than.replace("{m}", m)
            );
            ctx.include_bytes(uri(ten, xanh), svg.into_bytes());
        }
    }
}

/// Ảnh biểu tượng `ten` cỡ `kich` (vuông), xanh nếu `xanh`.
pub fn anh(ten: &str, kich: f32, xanh: bool) -> egui::Image<'static> {
    debug_assert!(DS.iter().any(|(t, _)| *t == ten), "thiếu biểu tượng {ten}");
    egui::Image::new(uri(ten, xanh)).fit_to_exact_size(egui::vec2(kich, kich))
}

/// Vẽ biểu tượng vào khung `o` (không tạo widget).
pub fn ve(ui: &egui::Ui, ten: &str, o: egui::Rect, xanh: bool) {
    anh(ten, o.width(), xanh).paint_at(ui, o);
}
