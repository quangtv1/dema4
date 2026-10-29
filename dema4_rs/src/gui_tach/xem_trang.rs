//! Bố cục chế độ xem 1 trang (vừa chiều ngang) / 2 trang liên tục của tab Văn bản: cuộn dọc liên tục, nội dung
//! luôn cân giữa khung; tỷ lệ > 100 % thì rộng hơn khung → cuộn ngang, vị trí cuộn đặt ở giữa.

/// Tỷ lệ so với "vừa chiều ngang" (%).
pub const TY_LE: [u16; 9] = [50, 75, 100, 125, 150, 175, 200, 225, 250];
pub const LE: f32 = 12.0;
pub const KHE: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OTrang {
    pub x: f32,
    pub y: f32,
    pub rong: f32,
    pub cao: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoCucTrang {
    pub rong_noi_dung: f32,
    pub cao_noi_dung: f32,
    /// Vị trí từng trang trong toạ độ nội dung.
    pub o: Vec<OTrang>,
}

/// `kich_thuoc` = (rộng, cao) hiển thị của từng trang (đã đổi chiều nếu xoay 90/270).
pub fn bo_cuc(khung_rong: f32, ty_le: u16, hai_trang: bool, kich_thuoc: &[(f32, f32)]) -> BoCucTrang {
    let co_so = (khung_rong - 2.0 * LE).max(50.0);
    let k = ty_le as f32 / 100.0;
    let (cot, rong_o) = if hai_trang { (2, ((co_so - KHE) / 2.0) * k) } else { (1, co_so * k) };
    let rong_hang = cot as f32 * rong_o + (cot - 1) as f32 * KHE;
    let rong_noi_dung = rong_hang + 2.0 * LE;
    // Hẹp hơn khung → cân giữa bằng lề trái
    let le_trai = LE + ((khung_rong - rong_noi_dung) / 2.0).max(0.0);
    let mut o = Vec::with_capacity(kich_thuoc.len());
    let mut y = LE;
    for hang in kich_thuoc.chunks(cot) {
        let mut cao_hang = 0f32;
        for (i, &(w, h)) in hang.iter().enumerate() {
            let cao = rong_o * h / w.max(1.0);
            o.push(OTrang { x: le_trai + i as f32 * (rong_o + KHE), y, rong: rong_o, cao });
            cao_hang = cao_hang.max(cao);
        }
        y += cao_hang + KHE;
    }
    BoCucTrang { rong_noi_dung: rong_noi_dung.max(khung_rong), cao_noi_dung: y - KHE + LE, o }
}

/// Vị trí cuộn ngang để nội dung rộng hơn khung nằm giữa.
pub fn cuon_ngang_giua(rong_noi_dung: f32, khung_rong: f32) -> f32 {
    ((rong_noi_dung - khung_rong) / 2.0).max(0.0)
}

/// Bước nấc (tỷ lệ hoặc cỡ lưới): kẹp trong danh sách.
pub fn buoc_nac(i: usize, tang: bool, so_nac: usize) -> usize {
    if tang { (i + 1).min(so_nac - 1) } else { i.saturating_sub(1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: (f32, f32) = (595.0, 842.0);

    #[test]
    fn mot_trang_vua_ngang_can_giua_moi_ty_le() {
        let khung = 800.0;
        let b = bo_cuc(khung, 100, false, &[A4, A4]);
        assert_eq!(b.o[0].rong, 776.0);
        assert!((b.o[0].x + b.o[0].rong / 2.0 - khung / 2.0).abs() < 0.01, "giữa khung");
        assert_eq!(b.o[1].y, b.o[0].y + b.o[0].cao + KHE, "cuộn dọc liên tục");
        for ty_le in TY_LE {
            let b = bo_cuc(khung, ty_le, false, &[A4]);
            let giua = b.o[0].x + b.o[0].rong / 2.0;
            assert!((giua - b.rong_noi_dung / 2.0).abs() < 0.01, "{ty_le}%: trang giữa nội dung");
            assert!(b.rong_noi_dung >= khung);
        }
        let b = bo_cuc(khung, 150, false, &[A4]);
        assert!(b.rong_noi_dung > khung);
        assert_eq!(cuon_ngang_giua(b.rong_noi_dung, khung), (b.rong_noi_dung - khung) / 2.0);
        assert_eq!(cuon_ngang_giua(700.0, khung), 0.0);
    }

    #[test]
    fn hai_trang_xep_cap_va_hang_cao_theo_trang_cao_nhat() {
        let ngang = (842.0, 595.0);
        let b = bo_cuc(800.0, 75, true, &[A4, ngang, A4]);
        assert_eq!(b.o[0].y, b.o[1].y);
        assert!(b.o[1].x > b.o[0].x);
        assert_eq!(b.o[2].x, b.o[0].x, "trang 3 xuống hàng, cột trái");
        assert_eq!(b.o[2].y, b.o[0].y + b.o[0].cao + KHE, "hàng cao theo trang dọc");
        let giua = (b.o[0].x + b.o[1].x + b.o[1].rong) / 2.0;
        assert!((giua - 400.0).abs() < 0.01);
    }

    #[test]
    fn buoc_nac_kep() {
        assert_eq!(buoc_nac(0, false, 7), 0);
        assert_eq!(buoc_nac(6, true, 7), 6);
        assert_eq!(buoc_nac(2, true, 7), 3);
    }
}
