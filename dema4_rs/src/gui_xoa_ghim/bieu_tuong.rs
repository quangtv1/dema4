//! Biểu tượng SVG của màn Xóa ghim — chép nguyên văn từ prototype đã chốt (cùng hình, cùng màu), đăng ký 1 lần
//! (`dang_ky`) dưới `bytes://xg/<tên>.svg`, vẽ bằng `egui::Image`.

use eframe::egui;

/// (tên, khung viewBox, nội dung). Màu cố định như prototype; chữ trong biểu tượng ("PDF") vẽ bằng painter vì bộ đọc SVG
/// không nạp phông.
const DS: &[(&str, &str, &str)] = &[
    // Thanh menu chung
    ("tep_tin", "0 0 14 14", r##"<path d="M1 3h4.5l1.2 1.4H13v7.6H1z" fill="none" stroke="#555" stroke-width="1.2"/>"##),
    ("chuc_nang", "0 0 14 14", r##"<path d="M1.5 1.5h4v4h-4zM8.5 1.5h4v4h-4zM1.5 8.5h4v4h-4zM8.5 8.5h4v4h-4z" fill="none" stroke="#555" stroke-width="1.2"/>"##),
    ("cau_hinh", "0 0 14 14", r##"<circle cx="7" cy="7" r="2.2" fill="none" stroke="#555" stroke-width="1.2"/><path d="M7 1v2M7 11v2M1 7h2M11 7h2M2.8 2.8l1.4 1.4M9.8 9.8l1.4 1.4M2.8 11.2l1.4-1.4M9.8 4.2l1.4-1.4" stroke="#555" stroke-width="1.2"/>"##),
    ("tro_giup", "0 0 14 14", r##"<circle cx="7" cy="7" r="5.8" fill="none" stroke="#555" stroke-width="1.2"/><path d="M5.3 5.3a1.8 1.8 0 1 1 2.4 1.7c-.5.2-.7.6-.7 1.1V9" fill="none" stroke="#555" stroke-width="1.2"/><circle cx="7" cy="10.6" r=".7" fill="#555"/>"##),
    ("caret", "0 0 8 8", r##"<path d="M1 2.5l3 3 3-3" fill="none" stroke="#444" stroke-width="1.3"/>"##),
    ("caret_trang", "0 0 8 8", r##"<path d="M1 2.5l3 3 3-3" fill="none" stroke="#fff" stroke-width="1.3"/>"##),
    ("thu_gon", "0 0 14 14", r##"<path d="M3 9l4-4 4 4" fill="none" stroke="#555" stroke-width="1.6"/>"##),
    ("mo_cong_cu", "0 0 14 14", r##"<path d="M3 5l4 4 4-4" fill="none" stroke="#555" stroke-width="1.6"/>"##),
    // Nút File / Thư mục
    ("file_pdf", "0 0 13 14", r##"<path d="M2 1h6l3 3v9H2z" fill="#fff" stroke="#6b7280"/><path d="M8 1v3h3" fill="none" stroke="#6b7280"/><rect x="3.3" y="8" width="6.4" height="3.2" rx=".5" fill="#dc2626"/>"##),
    ("thu_muc", "0 0 16 13", r##"<path d="M1 2.5h5l1.5 1.5H15v8H1z" fill="#f4c542" stroke="#b8901c"/>"##),
    // Điều hướng
    ("trai", "0 0 8 8", r##"<path d="M6 0L1 4l5 4z" fill="#555"/>"##),
    ("phai", "0 0 8 8", r##"<path d="M2 0l5 4-5 4z" fill="#555"/>"##),
    ("thu_ds", "0 0 12 12", r##"<path d="M7.5 2.5L4 6l3.5 3.5" fill="none" stroke="#444" stroke-width="1.5"/><path d="M2 1.5v9" stroke="#444" stroke-width="1.5"/>"##),
    ("mo_ds", "0 0 12 12", r##"<path d="M4.5 2.5L8 6 4.5 9.5" fill="none" stroke="#444" stroke-width="1.5"/><path d="M10 1.5v9" stroke="#444" stroke-width="1.5"/>"##),
    ("an_dich", "0 0 8 10", r##"<path d="M2 1l4 4-4 4" fill="none" stroke="#444" stroke-width="1.5"/>"##),
    ("mo_dich", "0 0 8 10", r##"<path d="M6 1L2 5l4 4" fill="none" stroke="#444" stroke-width="1.5"/>"##),
    ("vua_ngang", "0 0 16 12", r##"<path d="M1 1v10M15 1v10M2 6h12M4 4L2 6l2 2M12 4l2 2-2 2" fill="none" stroke="#555" stroke-width="1.3"/>"##),
    // Menu chuột phải trên trang Gốc (nét mảnh 14 px)
    ("m_ve_xoa", "0 0 16 16", r##"<rect x="2" y="2" width="12" height="12" rx="1.5" fill="none" stroke="#2563eb" stroke-width="1.3"/><path d="M5 8h6" stroke="#2563eb" stroke-width="1.5"/>"##),
    ("m_ve_bao", "0 0 16 16", r##"<rect x="2" y="2" width="12" height="12" rx="1.5" fill="none" stroke="#c026d3" stroke-width="1.3" stroke-dasharray="2.5 1.5"/><path d="M5 8h6M8 5v6" stroke="#c026d3" stroke-width="1.5"/>"##),
    ("m_chon_het", "0 0 16 16", r##"<rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-dasharray="2 1.6"/><path d="M5.2 8.2l1.9 1.9 3.8-4" fill="none" stroke="#4b5563" stroke-width="1.4"/>"##),
    ("m_xoa", "0 0 16 16", r##"<path d="M2.5 4.5h11M6 4.5V2.8h4v1.7M4 4.5l.7 9h6.6l.7-9M6.7 7v4.5M9.3 7v4.5" fill="none" stroke="#dc2626" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"##),
    ("m_doi", "0 0 16 16", r##"<path d="M3 5.5h9.5M10 3l2.5 2.5L10 8M13 10.5H3.5M6 8l-2.5 2.5L6 13" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"##),
    ("m_nhan_ban", "0 0 16 16", r##"<rect x="5" y="5" width="9" height="9" rx="1.3" fill="none" stroke="#4b5563" stroke-width="1.3"/><path d="M11 5V3.3A1.3 1.3 0 0 0 9.7 2H3.3A1.3 1.3 0 0 0 2 3.3v6.4A1.3 1.3 0 0 0 3.3 11H5" fill="none" stroke="#4b5563" stroke-width="1.3"/>"##),
    ("m_trang", "0 0 16 16", r##"<path d="M4 1.8h5.5L13 5.3v8.9H4z" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linejoin="round"/><path d="M9.5 1.8v3.5H13" fill="none" stroke="#4b5563" stroke-width="1.3"/>"##),
    ("m_chon", "0 0 16 16", r##"<path d="M3.5 8.5l3 3 6-7" fill="none" stroke="#1f5fbf" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"##),
    ("m_hoan_tac", "0 0 16 16", r##"<path d="M5.5 3.5L2.5 6.5l3 3" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M2.5 6.5h7a4 4 0 0 1 0 8H6" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round"/>"##),
    ("cong", "0 0 12 12", r##"<path d="M6 1v10M1 6h10" fill="none" stroke="#444" stroke-width="1.6"/>"##),
    ("tru", "0 0 12 12", r##"<path d="M1 6h10" fill="none" stroke="#444" stroke-width="1.6"/>"##),
    ("vua_cao", "0 0 12 16", r##"<path d="M1 1h10M1 15h10M6 2v12M4 4l2-2 2 2M4 12l2 2 2-2" fill="none" stroke="#555" stroke-width="1.3"/>"##),
    ("chay", "0 0 9 10", r##"<path d="M1 1l7 4-7 4z" fill="#fff"/>"##),
    ("kinh_lup", "0 0 12 12", r##"<circle cx="5" cy="5" r="3.6" fill="none" stroke="#777" stroke-width="1.3"/><path d="M7.7 7.7L11 11" stroke="#777" stroke-width="1.3"/>"##),
    // Màn trống
    ("lon_pdf", "0 0 40 50", r##"<path d="M2 2h26l10 10v36H2z" fill="#f4f5f7" stroke="#a7abb1" stroke-width="1.5"/><path d="M28 2v10h10" fill="none" stroke="#a7abb1" stroke-width="1.5"/>"##),
    ("lon_thu_muc", "0 0 48 40", r##"<path d="M2 6h16l4 5h24v27H2z" fill="#f4f5f7" stroke="#a7abb1" stroke-width="1.5"/><path d="M2 14h44" stroke="#a7abb1" stroke-width="1.5"/>"##),
    ("canh_bao", "0 0 18 18", r##"<path d="M9 1.5L17 16H1z" fill="#fef3c7" stroke="#d97706" stroke-width="1.2"/><path d="M9 6.5v4.2" stroke="#b45309" stroke-width="1.6"/><circle cx="9" cy="13" r=".9" fill="#b45309"/>"##),
    ("tai", "0 0 12 12", r##"<path d="M6 1.5v6M3.5 5.2L6 7.7l2.5-2.5M2 10.5h8" fill="none" stroke="#1f5fbf" stroke-width="1.4"/>"##),
    ("dau_xanh", "0 0 12 12", r##"<path d="M2 6.5l2.6 2.5L10 3.5" fill="none" stroke="#16a34a" stroke-width="1.6"/>"##),
    // Trạng thái file
    ("tt_chua", "0 0 11 11", r##"<circle cx="5.5" cy="5.5" r="4" fill="none" stroke="#b6bbc2" stroke-width="1.4"/>"##),
    ("tt_xong", "0 0 11 11", r##"<path d="M1.5 6l2.6 2.5L9.5 2.5" fill="none" stroke="#16a34a" stroke-width="1.8"/>"##),
    ("tt_doi", "0 0 11 11", r##"<path d="M9 4.2A3.8 3.8 0 1 0 9.3 7" fill="none" stroke="#d97706" stroke-width="1.5"/><path d="M9.5 1.5v3h-3" fill="none" stroke="#d97706" stroke-width="1.5"/>"##),
    ("tt_mat", "0 0 11 11", r##"<circle cx="5.5" cy="5.5" r="4.6" fill="#dc2626"/><path d="M5.5 2.8v3.3" stroke="#fff" stroke-width="1.4"/><circle cx="5.5" cy="8" r=".8" fill="#fff"/>"##),
    // Hộp Cấu hình
    ("khien", "0 0 16 16", r##"<g fill="none" stroke="#2563eb" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M8 1.5l5.5 2v4c0 3.4-2.4 5.8-5.5 7-3.1-1.2-5.5-3.6-5.5-7v-4z"/><path d="M5.6 8l1.7 1.7L10.6 6.4"/></g>"##),
    ("layout", "0 0 16 16", r##"<g fill="none" stroke="#dc2626" stroke-width="1.3" stroke-linecap="round"><rect x="2" y="2" width="12" height="12" rx="1.5"/><path d="M4.5 5h7M4.5 8h7M4.5 11h4"/></g>"##),
    ("net_den", "0 0 16 16", r##"<g fill="none" stroke="#111827" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M3 13l2.2-.6L13 4.6 11.4 3 3.6 10.8z"/><path d="M3 13h10"/></g>"##),
    ("dau", "0 0 16 16", r##"<g fill="none" stroke="#dc2626" stroke-width="1.3"><circle cx="8" cy="8" r="5.5"/><circle cx="8" cy="8" r="2.6"/></g>"##),
    ("xuat", "0 0 16 16", r##"<g fill="none" stroke="#2563eb" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M9 1.8H4a1 1 0 0 0-1 1v10.4a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V5.8z"/><path d="M9 1.8v4h4"/><path d="M8 8v4M6.2 10.2L8 12l1.8-1.8"/></g>"##),
    ("dpi", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><rect x="2" y="2" width="12" height="12" rx="1"/><path d="M6 2v12M10 2v12M2 6h12M2 10h12"/></g>"##),
    ("nen", "0 0 16 16", r##"<path d="M2.5 5.5h4v-4M13.5 10.5h-4v4M6.5 5.5L2 1M9.5 10.5L14 15" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"##),
    ("den_trang", "0 0 16 16", r##"<circle cx="8" cy="8" r="5.8" fill="none" stroke="#4b5563" stroke-width="1.3"/><path d="M8 2.2v11.6a5.8 5.8 0 0 0 0-11.6z" fill="#4b5563"/>"##),
    ("nen_trang", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9.5L8 4.5l4.5 4.5-5 5z"/><path d="M12.5 9.5c.8 1 1.5 1.9 1.5 2.6a1.3 1.3 0 0 1-2.6 0c0-.7.4-1.4 1.1-2.6z"/></g>"##),
    ("ghi_de", "0 0 16 16", r##"<path d="M8 1.8L15 14H1z" fill="none" stroke="#d97706" stroke-width="1.3" stroke-linejoin="round"/><path d="M8 6.2v3.6" stroke="#d97706" stroke-width="1.3" stroke-linecap="round"/><circle cx="8" cy="11.8" r=".5" fill="#d97706"/>"##),
    ("hieu_nang", "0 0 16 16", r##"<g fill="none" stroke="#2563eb" stroke-width="1.3" stroke-linecap="round"><path d="M2.5 11.5a5.5 5.5 0 1 1 11 0"/><path d="M8 11.5l3-4"/></g><circle cx="8" cy="11.5" r=".8" fill="#2563eb"/>"##),
    ("lop", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linejoin="round"><path d="M8 2l6 3-6 3-6-3z"/><path d="M2 8l6 3 6-3M2 11l6 3 6-3"/></g>"##),
    ("dong_ho", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round"><circle cx="8" cy="8" r="6"/><path d="M8 4.5V8l2.4 1.6"/></g>"##),
    ("mat", "0 0 16 16", r##"<g fill="none" stroke="#2563eb" stroke-width="1.3"><path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/></g>"##),
    ("kieu_xem", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><rect x="4" y="1.5" width="8" height="5.5" rx=".8"/><rect x="4" y="9" width="8" height="5.5" rx=".8"/></g>"##),
    ("luoi", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><rect x="2" y="2" width="5" height="5" rx=".8"/><rect x="9" y="2" width="5" height="5" rx=".8"/><rect x="2" y="9" width="5" height="5" rx=".8"/><rect x="9" y="9" width="5" height="5" rx=".8"/></g>"##),
    ("hai_khung", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><rect x="1.5" y="2.5" width="13" height="11" rx="1"/><path d="M8 2.5v11"/></g>"##),
    ("noi", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round"><path d="M6.5 9.5l3-3"/><path d="M7.5 4.5l1-1a2.8 2.8 0 0 1 4 4l-1 1M8.5 11.5l-1 1a2.8 2.8 0 0 1-4-4l1-1"/></g>"##),
    ("tab_dem", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><path d="M4 2h6l3 3v9H4z"/><path d="M6.5 8h4M6.5 11h4"/></g>"##),
    ("tab_tach", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round"><circle cx="4.5" cy="4.5" r="2"/><circle cx="4.5" cy="11.5" r="2"/><path d="M6 5.8l8 6.2M6 10.2l8-6.2"/></g>"##),
    ("tab_muc_luc", "0 0 16 16", r##"<path d="M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.5M2.5 8h.5M2.5 12h.5" fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round"/>"##),
    ("tab_boc", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3"><rect x="2" y="2" width="12" height="12" rx="1"/><path d="M5 6h6M5 9h3"/></g>"##),
    ("tab_xoa_ghim", "0 0 16 16", r##"<g fill="none" stroke="#4b5563" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 2.5L11 8.5M5.5 9.5l-2 2 1 1 2-2"/><path d="M9 2l5 5-2 2-5-5z"/></g>"##),
];

fn uri(ten: &str) -> String {
    format!("bytes://xg/{ten}.svg")
}

/// Đăng ký mọi SVG (gọi 1 lần sau khi nạp bộ đọc ảnh).
pub fn dang_ky(ctx: &egui::Context) {
    for (ten, khung, than) in DS {
        let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{khung}">{than}</svg>"##);
        ctx.include_bytes(uri(ten), svg.into_bytes());
    }
}

/// Ảnh biểu tượng `ten`, cỡ `rong × cao` điểm.
pub fn anh(ten: &str, rong: f32, cao: f32) -> egui::Image<'static> {
    debug_assert!(DS.iter().any(|(t, _, _)| *t == ten), "thiếu biểu tượng {ten}");
    egui::Image::new(uri(ten)).fit_to_exact_size(egui::vec2(rong, cao))
}

/// Vẽ biểu tượng vào khung `o` (không tạo widget).
pub fn ve(ui: &egui::Ui, ten: &str, o: egui::Rect) {
    anh(ten, o.width(), o.height()).paint_at(ui, o);
}
