//! Thanh khoảng 2 ngưỡng trắng (kiểu cắt clip) trong Cấu hình › Tách PDF: thước 0–10 % thang căn bậc hai
//! (giãn vùng 0,5–3 % hay dùng). Hàm thuần ở đây, phần vẽ / chuột ở `hop_cau_hinh`.

pub const TOI_DA: f32 = 10.0;
pub const BUOC: f32 = 0.01;
pub const BUOC_LON: f32 = 0.1;
pub const MOC: [f32; 7] = [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0];

/// Giá trị (%) → vị trí 0..1 trên thước.
pub fn vi_tri(v: f32) -> f32 {
    (v.clamp(0.0, TOI_DA) / TOI_DA).sqrt()
}

/// Vị trí 0..1 → giá trị (%), làm tròn 0,01.
pub fn gia_tri(x: f32) -> f32 {
    lam_tron(x.clamp(0.0, 1.0).powi(2) * TOI_DA)
}

pub fn lam_tron(v: f32) -> f32 {
    (v / BUOC).round() * BUOC
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mep {
    Trai,
    Phai,
}

/// Đặt 1 mép, kẹp 0 ≤ trắng ≤ nghi ≤ 10. Trả (trắng, nghi).
pub fn dat_mep(trang: f32, nghi: f32, mep: Mep, v: f32) -> (f32, f32) {
    let v = lam_tron(v);
    match mep {
        Mep::Trai => (v.clamp(0.0, nghi), nghi),
        Mep::Phai => (trang, v.clamp(trang, TOI_DA)),
    }
}

/// Kéo cả khung thêm `delta` (%): giữ `nghi − trang`, kẹp ở 2 đầu thước.
pub fn doi_khung(trang: f32, nghi: f32, delta: f32) -> (f32, f32) {
    let rong = nghi - trang;
    let t = lam_tron((trang + delta).clamp(0.0, TOI_DA - rong));
    (t, lam_tron(t + rong))
}

/// Bấm vào thước tại vị trí `x`: mép gần nhất (theo vị trí) nhảy tới.
pub fn bam_thuoc(trang: f32, nghi: f32, x: f32) -> (f32, f32) {
    let mep = if (x - vi_tri(trang)).abs() <= (x - vi_tri(nghi)).abs() && x <= vi_tri(nghi) { Mep::Trai } else { Mep::Phai };
    dat_mep(trang, nghi, mep, gia_tri(x))
}

/// ←/→ trên mép đang chọn: ±0,01, Shift ±0,1.
pub fn phim(trang: f32, nghi: f32, mep: Mep, tang: bool, lon: bool) -> (f32, f32) {
    let b = if lon { BUOC_LON } else { BUOC } * if tang { 1.0 } else { -1.0 };
    let v = match mep {
        Mep::Trai => trang,
        Mep::Phai => nghi,
    };
    dat_mep(trang, nghi, mep, v + b)
}

/// Câu tooltip ⓘ, số theo giá trị đang đặt.
pub fn goi_y(trang: f32, nghi: f32) -> String {
    format!("% = phần diện tích trang có nét mực đậm. Dưới số đầu ({} %): Trắng · đến số sau ({} %): Nghi trắng · trên: có chữ.", so_vn(trang), so_vn(nghi))
}

/// 3.2 → "3,2"; 1.0 → "1"; 0.75 → "0,75".
pub fn so_vn(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gan(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
    }

    #[test]
    fn anh_xa_can_bac_hai_khu_hoi() {
        for v in [0.0, 0.01, 0.5, 1.0, 3.2, 7.77, 10.0] {
            assert!((gia_tri(vi_tri(v)) - v).abs() < 0.006, "{v}");
        }
        assert_eq!(vi_tri(0.0), 0.0);
        assert_eq!(vi_tri(10.0), 1.0);
        assert!((vi_tri(2.5) - 0.5).abs() < 1e-6, "giữa thước = 2,5 %");
        assert_eq!(vi_tri(20.0), 1.0);
        assert!(MOC.windows(2).all(|w| vi_tri(w[0]) < vi_tri(w[1])));
    }

    #[test]
    fn keo_mep_khong_vuot_nhau_va_keo_khung_giu_khoang() {
        assert!(gan(dat_mep(1.0, 3.2, Mep::Trai, 5.0), (3.2, 3.2)));
        assert!(gan(dat_mep(1.0, 3.2, Mep::Phai, 0.5), (1.0, 1.0)));
        assert!(gan(dat_mep(1.0, 3.2, Mep::Phai, 12.0), (1.0, 10.0)));
        assert!(gan(dat_mep(1.0, 3.2, Mep::Trai, -1.0), (0.0, 3.2)));
        assert!(gan(doi_khung(1.0, 3.2, 0.5), (1.5, 3.7)));
        assert!(gan(doi_khung(1.0, 3.2, -5.0), (0.0, 2.2)), "kẹp đầu 0, giữ khoảng");
        assert!(gan(doi_khung(1.0, 3.2, 20.0), (7.8, 10.0)), "kẹp đầu 10");
    }

    #[test]
    fn bam_thuoc_mep_gan_nhat_va_phim() {
        assert!(gan(bam_thuoc(1.0, 3.2, vi_tri(0.5)), (0.5, 3.2)));
        assert!(gan(bam_thuoc(1.0, 3.2, vi_tri(6.0)), (1.0, 6.0)));
        assert!(gan(bam_thuoc(1.0, 3.2, vi_tri(1.5)), (1.5, 3.2)), "1,5 gần mép trái hơn (theo vị trí)");
        assert!(gan(phim(1.0, 3.2, Mep::Trai, true, false), (1.01, 3.2)));
        assert!(gan(phim(1.0, 3.2, Mep::Phai, false, true), (1.0, 3.1)));
        assert!(gan(phim(3.2, 3.2, Mep::Trai, true, true), (3.2, 3.2)));
    }

    #[test]
    fn goi_y_theo_gia_tri_dang_dat() {
        assert_eq!(so_vn(3.2), "3,2");
        assert_eq!(so_vn(1.0), "1");
        assert_eq!(so_vn(0.75), "0,75");
        assert!(goi_y(0.8, 4.0).contains("(0,8 %): Trắng") && goi_y(0.8, 4.0).contains("(4 %): Nghi trắng"));
    }
}
