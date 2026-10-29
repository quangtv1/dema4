//! Xử lý ảnh trang dùng chung: thu nhỏ bằng trung bình vùng, đổi RGB → xám.
//! Mỗi trang chỉ render 1 lần (150 dpi RGB); đọc mã, dò trắng, hướng trang, ảnh nhỏ đều lấy từ ảnh đó.

/// Ảnh 8 bit, `kenh` byte mỗi điểm (1 = xám, 3 = RGB), không đệm cuối dòng.
#[derive(Clone)]
pub struct Anh {
    pub rong: usize,
    pub cao: usize,
    pub kenh: usize,
    pub du_lieu: Vec<u8>,
}

impl Anh {
    /// Xám theo trọng số 0,30 R + 0,59 G + 0,11 B (như MuPDF).
    pub fn sang_xam(&self) -> Anh {
        if self.kenh == 1 {
            return self.clone();
        }
        let du_lieu = self.du_lieu.chunks_exact(self.kenh).map(|p| ((p[0] as u32 * 77 + p[1] as u32 * 151 + p[2] as u32 * 28 + 128) >> 8) as u8).collect();
        Anh { rong: self.rong, cao: self.cao, kenh: 1, du_lieu }
    }

    /// Thu nhỏ theo `ty_le` (< 1) bằng trung bình vùng — khử răng cưa khi thu nhỏ.
    pub fn thu_nho(&self, ty_le: f32) -> Anh {
        let rong_moi = ((self.rong as f32) * ty_le).max(1.0) as usize;
        let cao_moi = ((self.cao as f32) * ty_le).max(1.0) as usize;
        self.thu_ve(rong_moi, cao_moi)
    }

    /// Thu về đúng kích thước `rong_moi` × `cao_moi` (không lớn hơn ảnh gốc) bằng trung bình vùng.
    pub fn thu_ve(&self, rong_moi: usize, cao_moi: usize) -> Anh {
        Anh { rong: rong_moi, cao: cao_moi, kenh: self.kenh, du_lieu: thu_ve_mang(&self.du_lieu, self.rong, self.cao, self.kenh, rong_moi, cao_moi) }
    }
}

/// Thu mảng điểm `rong` × `cao` × `kenh` về `rong_moi` × `cao_moi` bằng trung bình vùng.
pub fn thu_ve_mang(du_lieu: &[u8], rong: usize, cao: usize, kenh: usize, rong_moi: usize, cao_moi: usize) -> Vec<u8> {
    let bien = |n_moi: usize, n: usize| -> Vec<(usize, usize)> {
        (0..n_moi).map(|i| { let a = (i * n / n_moi).min(n - 1); (a, ((i + 1) * n / n_moi).max(a + 1).min(n)) }).collect()
    };
    let (cot, hang) = (bien(rong_moi, rong), bien(cao_moi, cao));
    let k = kenh;
    let mut ra = vec![0u8; rong_moi * cao_moi * k];
    let mut tong = vec![0u32; k];
    for (y, &(y0, y1)) in hang.iter().enumerate() {
        for (x, &(x0, x1)) in cot.iter().enumerate() {
            tong.iter_mut().for_each(|t| *t = 0);
            for yy in y0..y1 {
                for p in du_lieu[(yy * rong + x0) * k..(yy * rong + x1) * k].chunks_exact(k) {
                    for c in 0..k {
                        tong[c] += p[c] as u32;
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            for c in 0..k {
                ra[(y * rong_moi + x) * k + c] = ((tong[c] + n / 2) / n) as u8;
            }
        }
    }
    ra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thu_nho_giu_mau_va_kich_thuoc() {
        let a = Anh { rong: 100, cao: 50, kenh: 3, du_lieu: [10u8, 200, 30].repeat(100 * 50) };
        let b = a.thu_nho(2.0 / 3.0);
        assert_eq!((b.rong, b.cao), (66, 33));
        assert!(b.du_lieu.chunks_exact(3).all(|p| p == [10, 200, 30]));
        assert_eq!(a.sang_xam().du_lieu[0], ((10 * 77 + 200 * 151 + 30 * 28 + 128) >> 8) as u8);
    }

    #[test]
    fn trung_binh_vung_dung() {
        let a = Anh { rong: 4, cao: 2, kenh: 1, du_lieu: vec![0, 100, 200, 255, 0, 100, 200, 255] };
        let b = a.thu_ve(2, 1);
        assert_eq!(b.du_lieu, vec![50, 228]);
    }
}
