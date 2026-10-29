//! Thước ranh giới khổ (Cấu hình › Đếm trang A4): 1 thước liên tục A7 → A0 thang log (mỗi khổ rộng gần bằng
//! nhau), 7 tay ranh giới + 1 tay ngưỡng A0 quy theo diện tích. Hàm thuần ở đây, phần vẽ ở `hop_cau_hinh`.

use crate::cau_hinh::{A0_NGUONG_KHOANG, CANH_CHUAN_MM};
use crate::dem_pdf::KHO;

pub const MM_DAU: f32 = 90.0;
pub const MM_CUOI: f32 = 2600.0;
pub const BUOC: f32 = 0.5;
pub const BUOC_LON: f32 = 5.0;

/// mm → vị trí 0..1 (trái = nhỏ).
pub fn vi_tri(mm: f32) -> f32 {
    ((mm.clamp(MM_DAU, MM_CUOI) / MM_DAU).ln() / (MM_CUOI / MM_DAU).ln()).clamp(0.0, 1.0)
}

/// Vị trí 0..1 → mm.
pub fn mm_tai(x: f32) -> f32 {
    MM_DAU * (MM_CUOI / MM_DAU).powf(x.clamp(0.0, 1.0))
}

fn lam_tron(mm: f32) -> f32 {
    (mm / BUOC).round() * BUOC
}

/// Khoảng kéo được của tay `i` (0..7 = ranh giới A0|A1 … A6|A7; 7 = ngưỡng A0 lớn).
pub fn khoang(i: usize) -> (f32, f32) {
    if i == 7 {
        (A0_NGUONG_KHOANG.0 as f32, A0_NGUONG_KHOANG.1 as f32)
    } else {
        (CANH_CHUAN_MM[i + 1] as f32 + BUOC, CANH_CHUAN_MM[i] as f32 - BUOC)
    }
}

/// Đặt tay `i` tại `mm`: làm tròn 0,5 mm, kẹp trong khoảng 2 cạnh chuẩn kề bên.
pub fn dat(i: usize, mm: f32) -> f32 {
    let (a, b) = khoang(i);
    lam_tron(mm).clamp(a, b)
}

/// ←/→ trên tay đang chọn: ±0,5 mm, Shift ±5 mm.
pub fn phim(i: usize, mm: f32, tang: bool, lon: bool) -> f32 {
    dat(i, mm + if lon { BUOC_LON } else { BUOC } * if tang { 1.0 } else { -1.0 })
}

/// Kết quả ô "Thử cạnh dài": khổ theo ranh giới đang kéo, kèm A0 quy theo diện tích nếu vượt ngưỡng.
pub fn thu(mm: f64, ranh: &[f64; 7], a0_nguong: f64) -> String {
    let kho = ranh.iter().position(|&r| mm >= r).unwrap_or(7);
    if kho == 0 && mm > a0_nguong {
        "→ A0 · quy A4 theo diện tích".to_string()
    } else {
        format!("→ {}", KHO[kho])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cau_hinh::RANH_GIOI_MAC_DINH;

    #[test]
    fn thang_log_khu_hoi_va_moi_kho_rong_gan_bang_nhau() {
        for mm in [90.0, 105.0, 297.0, 1189.0, 2600.0] {
            assert!((mm_tai(vi_tri(mm)) - mm).abs() < 0.01 * mm, "{mm}");
        }
        let rong: Vec<f32> = CANH_CHUAN_MM.windows(2).map(|w| vi_tri(w[0] as f32) - vi_tri(w[1] as f32)).collect();
        let (nho, lon) = rong.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(lon / nho < 1.15, "{rong:?}");
    }

    #[test]
    fn tay_keo_kep_trong_khoang_buoc_nua_mm() {
        assert_eq!(dat(3, 380.26), 380.5);
        assert_eq!(dat(3, 500.0), 419.5, "A3|A4 không vượt cạnh A3");
        assert_eq!(dat(3, 100.0), 297.5);
        assert_eq!(dat(7, 3000.0), 2500.0);
        assert_eq!(dat(7, 1000.0), 1189.0);
        assert_eq!(phim(4, 253.5, true, false), 254.0);
        assert_eq!(phim(4, 253.5, false, true), 248.5);
    }

    #[test]
    fn thu_canh_dai_theo_ranh_gioi() {
        let mut rg = RANH_GIOI_MAC_DINH;
        assert_eq!(thu(297.0, &rg, 1399.0), "→ A4");
        assert_eq!(thu(370.0, &rg, 1399.0), "→ A3");
        rg[3] = 380.0;
        assert_eq!(thu(370.0, &rg, 1399.0), "→ A4");
        assert_eq!(thu(1500.0, &rg, 1399.0), "→ A0 · quy A4 theo diện tích");
        assert_eq!(thu(1300.0, &rg, 1399.0), "→ A0");
        assert_eq!(thu(50.0, &rg, 1399.0), "→ A7");
    }
}
