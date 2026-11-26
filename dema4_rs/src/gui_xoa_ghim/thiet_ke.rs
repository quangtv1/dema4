//! Hằng số thiết kế màn Xóa ghim — màu, cỡ đúng bảng thông số của prototype đã chốt
//! (`plans/261126-0727-prototype-man-xoa-ghim/prototype-xoa-ghim.html`). Màn chỉ dùng hằng ở đây, không rải số.

use eframe::egui::Color32;

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied_const(r, g, b, (a * 255.0) as u8)
}

// ---------- Màu ----------
pub const XANH: Color32 = rgb(0x25, 0x63, 0xEB);
pub const XANH_DAM_CHU: Color32 = rgb(0x1F, 0x5F, 0xBF);
pub const XANH_THANH: Color32 = rgb(0x3B, 0x82, 0xF6);
pub const XANH_NUT_MO: Color32 = rgb(0x9D, 0xB7, 0xE6);
pub const CHU: Color32 = rgb(0x1F, 0x23, 0x28);
pub const CHU_PHU: Color32 = rgb(0x6B, 0x72, 0x80);
pub const CHU_MO: Color32 = rgb(0x9A, 0xA0, 0xA6);
pub const VIEN: Color32 = rgb(0xC9, 0xCC, 0xD1);
pub const VIEN_KHUNG: Color32 = rgb(0xDC, 0xDF, 0xE3);
pub const VIEN_NHAT: Color32 = rgb(0xE2, 0xE4, 0xE8);
pub const VIEN_DONG: Color32 = rgb(0xE6, 0xE8, 0xEB);
pub const NEN_NUT: Color32 = rgb(0xF6, 0xF7, 0xF8);
pub const NEN_NUT_HOVER: Color32 = rgb(0xEC, 0xEE, 0xF1);
pub const NEN_TIEU_DE_KHUNG: Color32 = rgb(0xF2, 0xF3, 0xF5);
pub const NEN_XEM: Color32 = rgb(0xE4, 0xE6, 0xEA);
pub const NEN_DAY: Color32 = rgb(0xF7, 0xF8, 0xF9);
pub const NEN_DS_TIEU_DE: Color32 = rgb(0xF3, 0xF4, 0xF6);
pub const NEN_HANG_SAP: Color32 = rgb(0xFA, 0xFB, 0xFC);
pub const NEN_DONG_CHON: Color32 = rgb(0xDC, 0xEB, 0xFA);
pub const NEN_DONG_HOVER: Color32 = rgb(0xF1, 0xF5, 0xFB);
pub const NEN_RANH: Color32 = rgb(0xF5, 0xF6, 0xF8);
pub const NEN_TRANG_GOC: Color32 = rgb(0xFB, 0xFA, 0xF7);
pub const NEN_TOOLTIP: Color32 = rgb(0x1F, 0x29, 0x37);
pub const NEN_NUT_VUA_DANG: Color32 = rgb(0xDB, 0xE7, 0xFB);
pub const VIEN_NUT_VUA_DANG: Color32 = rgb(0x93, 0xB4, 0xEA);
pub const VIEN_NUT_LINK: Color32 = rgb(0xB6, 0xCC, 0xEA);
pub const XTRON: Color32 = rgb(0x9C, 0xA3, 0xAF);
pub const XTRON_HOVER: Color32 = rgb(0xDC, 0x26, 0x26);
pub const CAM_CHU: Color32 = rgb(0xB4, 0x53, 0x09);
pub const CAM_NEN: Color32 = rgb(0xFF, 0xFB, 0xEB);
pub const CAM_VIEN: Color32 = rgb(0xFD, 0xE6, 0x8A);
pub const DO_NUT: Color32 = rgb(0xDC, 0x26, 0x26);
pub const XANH_LA_OK: Color32 = rgb(0x16, 0xA3, 0x4A);

// Vùng
pub const VUNG_GOC_NEN: Color32 = rgba(76, 175, 80, 0.26);
pub const VUNG_GOC_NEN_CHON: Color32 = rgba(76, 175, 80, 0.34);
pub const VUNG_GOC_VIEN: Color32 = rgb(0x43, 0xA0, 0x47);
pub const VUNG_VE_NEN: Color32 = rgba(37, 99, 235, 0.18);
pub const VUNG_VE_NEN_CHON: Color32 = rgba(37, 99, 235, 0.26);
pub const VUNG_VE_VIEN: Color32 = rgb(0x25, 0x63, 0xEB);
pub const VUNG_BAO_VE_NEN: Color32 = rgba(217, 70, 239, 0.20);
pub const VUNG_BAO_VE_VIEN: Color32 = rgb(0xC0, 0x26, 0xD3);
pub const VUNG_VO_DOI_NEN: Color32 = rgba(249, 115, 22, 0.22);
pub const VUNG_VO_DOI_NEN_CHON: Color32 = rgba(249, 115, 22, 0.32);
pub const VUNG_VO_DOI_VIEN: Color32 = rgb(0xEA, 0x58, 0x0C);
pub const TAY_NAM_VIEN: Color32 = rgb(0x1F, 0x5F, 0xBF);
pub const QUET_WINDOW_VIEN: Color32 = rgb(0x25, 0x63, 0xEB);
pub const QUET_WINDOW_NEN: Color32 = rgba(37, 99, 235, 0.08);
pub const QUET_CROSSING_VIEN: Color32 = rgb(0x16, 0xA3, 0x4A);
pub const QUET_CROSSING_NEN: Color32 = rgba(22, 163, 74, 0.08);
pub const NHAN_DIEN_NEN: Color32 = rgba(232, 84, 96, 0.30);
pub const NHAN_DIEN_VIEN: Color32 = rgba(200, 40, 55, 0.75);
// Biểu tượng trang giấy
pub const ICON_VIEN: Color32 = rgb(0xB3, 0xB7, 0xBD);
pub const ICON_DONG: Color32 = rgb(0xD3, 0xD6, 0xDA);
pub const ICON_HOVER: Color32 = rgb(0xCF, 0xE0, 0xFB);
pub const ICON_VE_XOA_NEN: Color32 = rgb(0xDB, 0xEA, 0xFE);
pub const ICON_VE_BAO_NEN: Color32 = rgb(0xFC, 0xE7, 0xF3);
pub const ICON_VE_BAO_VIEN: Color32 = rgb(0xDB, 0x27, 0x77);

// ---------- Cỡ ----------
pub const CHU_CO: f32 = 12.5;
pub const CHU_NHO: f32 = 11.5;
pub const CAO_NUT: f32 = 26.0;
pub const RONG_BNUT: f32 = 88.0;
pub const RONG_NUT_VB: f32 = 44.0;
pub const CAO_NUT_NHO: f32 = 22.0;
pub const DS_RONG_MIN: f32 = 200.0;
pub const DS_RONG_MAX: f32 = 520.0;
pub const DS_DAI_THU_NHO: f32 = 34.0;
pub const DS_CAO_TIEU_DE: f32 = 32.0;
pub const DS_CAO_HANG_SAP: f32 = 28.0;
pub const DS_CAO_DONG: f32 = 24.0;
pub const RONG_RANH: f32 = 4.0;
pub const CAO_TIEU_DE_KHUNG: f32 = 30.0;
pub const RONG_COT_NHO: f32 = 150.0;
pub const ANH_NHO: (f32, f32) = (96.0, 136.0);
pub const CAO_DAY: f32 = 40.0;
pub const KHE_TRANG: f32 = 18.0;
pub const DEM_TREN_TRANG: f32 = 6.0;
pub const DEM_DUOI_TRANG: f32 = 30.0;
/// Lề 2 bên khi vừa bề ngang (12 mỗi bên).
pub const LE_VUA: f32 = 24.0;
pub const RONG_NHAN: f32 = 62.0;
pub const RONG_NHAN_VUNG: f32 = 54.0;
pub const TAY_NAM: f32 = 9.0;
