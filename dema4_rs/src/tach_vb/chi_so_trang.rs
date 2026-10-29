//! Chỉ số trắng v3 của trang: phần diện tích có nét mực đậm (+ một phần mực màu).
//!
//! Trên ảnh 100 dpi RGB, bỏ 8 % mỗi mép; nền cục bộ = trung vị khối 16 px;
//! - `dam` = tỷ lệ khối 8×8 có ≥ 6 điểm tối hơn nền ≥ 60 và không phải mực màu
//!   (chữ thấm mặt sau, vân giấy, bìa màu đều nhạt → không tính);
//! - `mau` = tỷ lệ khối 8×8 có ≥ 6 điểm mực đỏ / xanh (dấu, chữ ký, viết tay màu);
//! - chỉ số = `dam + 0,3 × mau`.
//!
//! Chốt theo nhãn tay mẫu 2 (21.113 trang): ngưỡng 1,0 % / 3,0 % → Trắng đúng 99,85 %, trắng lọt Content 0,95 %.

use super::anh::Anh;

const MEP: f32 = 0.08;
const KHOI_NEN: usize = 16;
const KHOI_CUM: usize = 8;
const DIEM_TOI_THIEU: u32 = 6;
const LECH_NEN: f32 = 60.0;
pub const HE_SO_MAU: f32 = 0.3;
/// Trang có mực màu từ mức này (tỷ lệ khối) không bao giờ tự gọi Trắng — vào Nghi trắng để người dùng xem.
pub const MAU_TOI_DA_CHO_TRANG: f32 = 0.0005;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ChiSoTrang {
    pub dam: f32,
    pub mau: f32,
}

impl ChiSoTrang {
    pub fn gia_tri(&self) -> f32 {
        self.dam + HE_SO_MAU * self.mau
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaiTrang {
    Trang,
    NghiTrang,
    NoiDung,
}

/// Phân loại theo 2 ngưỡng (tỷ lệ, vd 0.01 = 1 %).
pub fn phan_loai(cs: ChiSoTrang, nguong_trang: f32, nguong_nghi: f32) -> LoaiTrang {
    let v = cs.gia_tri();
    if v < nguong_trang && cs.mau < MAU_TOI_DA_CHO_TRANG {
        LoaiTrang::Trang
    } else if v < nguong_nghi {
        LoaiTrang::NghiTrang
    } else {
        LoaiTrang::NoiDung
    }
}

/// Chỉ số của ảnh RGB 100 dpi (ảnh khác độ phân giải vẫn chạy được nhưng ngưỡng đã chốt cho 100 dpi).
pub fn tinh_chi_so(rgb: &Anh) -> ChiSoTrang {
    assert_eq!(rgb.kenh, 3);
    let (x0, y0) = ((rgb.rong as f32 * MEP) as usize, (rgb.cao as f32 * MEP) as usize);
    let (x1, y1) = ((rgb.rong as f32 * (1.0 - MEP)) as usize, (rgb.cao as f32 * (1.0 - MEP)) as usize);
    let rong = (x1 - x0) / KHOI_NEN * KHOI_NEN;
    let cao = (y1 - y0) / KHOI_NEN * KHOI_NEN;
    if rong == 0 || cao == 0 {
        return ChiSoTrang::default();
    }
    let diem = |x: usize, y: usize| -> (f32, f32, f32) {
        let i = ((y0 + y) * rgb.rong + x0 + x) * 3;
        (rgb.du_lieu[i] as f32, rgb.du_lieu[i + 1] as f32, rgb.du_lieu[i + 2] as f32)
    };
    // Nền cục bộ: trung vị độ sáng trong từng khối 16×16
    let (nx, ny) = (rong / KHOI_NEN, cao / KHOI_NEN);
    let mut nen = vec![0f32; nx * ny];
    let mut khoi = Vec::with_capacity(KHOI_NEN * KHOI_NEN);
    for by in 0..ny {
        for bx in 0..nx {
            khoi.clear();
            for y in by * KHOI_NEN..(by + 1) * KHOI_NEN {
                for x in bx * KHOI_NEN..(bx + 1) * KHOI_NEN {
                    let (r, g, b) = diem(x, y);
                    khoi.push((r + g + b) / 3.0);
                }
            }
            khoi.sort_unstable_by(f32::total_cmp);
            let m = khoi.len() / 2;
            nen[by * nx + bx] = (khoi[m - 1] + khoi[m]) / 2.0;
        }
    }
    // Đếm cụm 8×8 có đủ điểm mực đậm / điểm mực màu
    let (cx, cy) = (rong / KHOI_CUM, cao / KHOI_CUM);
    let (mut cum_dam, mut cum_mau) = (0usize, 0usize);
    for ky in 0..cy {
        for kx in 0..cx {
            let (mut n_dam, mut n_mau) = (0u32, 0u32);
            for y in ky * KHOI_CUM..(ky + 1) * KHOI_CUM {
                for x in kx * KHOI_CUM..(kx + 1) * KHOI_CUM {
                    let (r, g, b) = diem(x, y);
                    let la_mau = (r - g > 40.0 && r - b > 40.0) || (b - r > 40.0 && b - g > 25.0);
                    if la_mau {
                        n_mau += 1;
                    } else if nen[(y / KHOI_NEN) * nx + x / KHOI_NEN] - (r + g + b) / 3.0 >= LECH_NEN {
                        n_dam += 1;
                    }
                }
            }
            cum_dam += (n_dam >= DIEM_TOI_THIEU) as usize;
            cum_mau += (n_mau >= DIEM_TOI_THIEU) as usize;
        }
    }
    let tong = (cx * cy) as f32;
    ChiSoTrang { dam: cum_dam as f32 / tong, mau: cum_mau as f32 / tong }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trang A4 100 dpi nền `nen`, vẽ thêm các hình chữ nhật (x, y, rộng, cao, màu).
    fn trang(nen: [u8; 3], hinh: &[(usize, usize, usize, usize, [u8; 3])]) -> Anh {
        let (w, h) = (827, 1169);
        let mut du_lieu = nen.repeat(w * h);
        for &(x, y, rw, rh, c) in hinh {
            for yy in y..(y + rh).min(h) {
                for xx in x..(x + rw).min(w) {
                    du_lieu[(yy * w + xx) * 3..(yy * w + xx) * 3 + 3].copy_from_slice(&c);
                }
            }
        }
        Anh { rong: w, cao: h, kenh: 3, du_lieu }
    }

    /// Các dòng "chữ": vạch ngang mảnh, cao 3 px, cách nhau 20 px.
    fn dong_chu(so_dong: usize, mau: [u8; 3]) -> Vec<(usize, usize, usize, usize, [u8; 3])> {
        (0..so_dong).flat_map(|i| (0..30).map(move |j| (120 + j * 20, 150 + i * 20, 12, 3, mau))).collect()
    }

    #[test]
    fn trang_thuan_bia_mau_vet_mep_la_trang() {
        let (t1, t2) = (0.01, 0.03);
        assert_eq!(phan_loai(tinh_chi_so(&trang([255; 3], &[])), t1, t2), LoaiTrang::Trang);
        // bìa màu nhạt + vệt đen sát mép + lỗ bấm ở lề (trong 8 % mép)
        let bia = trang([235, 225, 200], &[(0, 0, 30, 1169, [20, 20, 20]), (20, 300, 25, 25, [0, 0, 0])]);
        assert_eq!(phan_loai(tinh_chi_so(&bia), t1, t2), LoaiTrang::Trang);
        // chữ xám rất nhạt (giả chữ thấm từ mặt sau)
        let tham = trang([250; 3], &dong_chu(40, [215, 215, 215]));
        assert_eq!(phan_loai(tinh_chi_so(&tham), t1, t2), LoaiTrang::Trang);
    }

    #[test]
    fn dau_do_vao_nghi_trang_chu_den_la_noi_dung() {
        let (t1, t2) = (0.01, 0.03);
        let dau = trang([255; 3], &[(500, 900, 60, 60, [200, 40, 40])]);
        let cs = tinh_chi_so(&dau);
        assert!(cs.dam < 0.001 && cs.mau > 0.0005, "{cs:?}");
        assert_eq!(phan_loai(cs, t1, t2), LoaiTrang::NghiTrang);
        let chu = trang([255; 3], &dong_chu(40, [30, 30, 30]));
        assert_eq!(phan_loai(tinh_chi_so(&chu), t1, t2), LoaiTrang::NoiDung);
    }

    #[test]
    fn nguong_doi_thi_phan_loai_doi() {
        let cs = ChiSoTrang { dam: 0.015, mau: 0.0 };
        assert_eq!(phan_loai(cs, 0.01, 0.03), LoaiTrang::NghiTrang);
        assert_eq!(phan_loai(cs, 0.02, 0.03), LoaiTrang::Trang);
        assert_eq!(phan_loai(cs, 0.005, 0.01), LoaiTrang::NoiDung);
    }
}
