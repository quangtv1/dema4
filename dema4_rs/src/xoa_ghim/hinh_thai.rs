//! Phép hình thái trên mặt nạ nhị phân (1 byte / điểm, 0 hoặc 1) với nhân elip, cho kết quả giống
//! OpenCV (`MORPH_ELLIPSE`, biên mặc định: ngoài ảnh không làm giãn vào, không làm co vào).
//!
//! Giãn theo từng hàng nhân: mỗi hàng nhân là 1 đoạn ngang nửa rộng `dx`, nên chỉ cần khoảng cách
//! ngang tới điểm bật gần nhất của từng dòng rồi so với `dx` — mỗi điểm tốn `k` phép so, không phụ
//! thuộc diện tích nhân.

/// Nửa bề rộng từng hàng của nhân elip `k`×`k` (hàng `dy = -r..=r`, `r = k / 2`), đúng công thức OpenCV.
pub fn nhan_elip(k: usize) -> Vec<usize> {
    let r = (k / 2) as i64;
    if r == 0 {
        return vec![0];
    }
    let nghich_r2 = 1.0 / (r * r) as f64;
    (-r..=r).map(|dy| ((r as f64) * (((r * r - dy * dy) as f64) * nghich_r2).sqrt()).round_ties_even() as usize).collect()
}

/// Cỡ nhân giãn theo dpi như XoaGhim: `max(5, int(5·dpi/120))`, làm lẻ.
pub fn co_nhan(dpi: f64) -> usize {
    let k = ((5.0 * (dpi / 120.0)) as usize).max(5);
    k | 1
}

/// Khoảng cách ngang tới điểm bật gần nhất trên cùng dòng (`u32::MAX` khi dòng không có điểm nào).
fn khoang_cach_ngang(m: &[u8], w: usize) -> Vec<u32> {
    let mut d = vec![u32::MAX; m.len()];
    for (dong, kc) in m.chunks_exact(w).zip(d.chunks_exact_mut(w)) {
        let mut gan = u32::MAX;
        for x in 0..w {
            gan = if dong[x] != 0 { 0 } else { gan.saturating_add(1) };
            kc[x] = gan;
        }
        gan = u32::MAX;
        for x in (0..w).rev() {
            gan = if dong[x] != 0 { 0 } else { gan.saturating_add(1) };
            kc[x] = kc[x].min(gan);
        }
    }
    d
}

/// Giãn 1 lần: điểm bật nếu nhân đặt tại đó chạm điểm bật nào của `m` (ngoài ảnh coi như tắt).
pub fn gian(m: &[u8], w: usize, h: usize, nhan: &[usize]) -> Vec<u8> {
    let r = (nhan.len() / 2) as i64;
    let kc = khoang_cach_ngang(m, w);
    let mut ra = vec![0u8; m.len()];
    for y in 0..h as i64 {
        let dong_ra = &mut ra[y as usize * w..(y as usize + 1) * w];
        for (i, &dx) in nhan.iter().enumerate() {
            let yy = y + i as i64 - r;
            if yy < 0 || yy >= h as i64 {
                continue;
            }
            let dong_kc = &kc[yy as usize * w..(yy as usize + 1) * w];
            for (o, &d) in dong_ra.iter_mut().zip(dong_kc) {
                *o |= (d as usize <= dx) as u8;
            }
        }
    }
    ra
}

/// Co 1 lần: điểm còn bật nếu nhân đặt tại đó nằm trọn trong điểm bật (ngoài ảnh coi như bật).
pub fn co(m: &[u8], w: usize, h: usize, nhan: &[usize]) -> Vec<u8> {
    let dao: Vec<u8> = m.iter().map(|&v| (v == 0) as u8).collect();
    gian(&dao, w, h, nhan).into_iter().map(|v| (v == 0) as u8).collect()
}

/// Đóng `lan_dong` lần (giãn hết rồi co hết, như `morphologyEx(CLOSE, iterations)`) rồi giãn thêm `lan_gian` lần.
pub fn dong_roi_gian(m: Vec<u8>, w: usize, h: usize, nhan: &[usize], lan_dong: usize, lan_gian: usize) -> Vec<u8> {
    let mut m = m;
    for _ in 0..lan_dong {
        m = gian(&m, w, h, nhan);
    }
    for _ in 0..lan_dong {
        m = co(&m, w, h, nhan);
    }
    for _ in 0..lan_gian {
        m = gian(&m, w, h, nhan);
    }
    m
}

/// Tắt các mảng liền (4 hướng) có ít hơn `toi_thieu` điểm.
pub fn bo_manh_nho(m: &mut [u8], w: usize, toi_thieu: usize) {
    let mut da_xet = vec![false; m.len()];
    let mut ngan_xep = Vec::new();
    let mut manh = Vec::new();
    for bat_dau in 0..m.len() {
        if m[bat_dau] == 0 || da_xet[bat_dau] {
            continue;
        }
        manh.clear();
        da_xet[bat_dau] = true;
        ngan_xep.push(bat_dau);
        while let Some(i) = ngan_xep.pop() {
            manh.push(i);
            let x = i % w;
            let ke = [(x > 0).then(|| i - 1), (x + 1 < w).then(|| i + 1), i.checked_sub(w), (i + w < m.len()).then(|| i + w)];
            for j in ke.into_iter().flatten() {
                if m[j] != 0 && !da_xet[j] {
                    da_xet[j] = true;
                    ngan_xep.push(j);
                }
            }
        }
        if manh.len() < toi_thieu {
            for &i in &manh {
                m[i] = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Giãn / co theo định nghĩa (duyệt cả nhân) để đối chiếu cách tính theo hàng.
    fn tham_chieu(m: &[u8], w: usize, h: usize, nhan: &[usize], la_gian: bool) -> Vec<u8> {
        let r = (nhan.len() / 2) as i64;
        let mut ra = vec![0u8; m.len()];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let mut cham = false;
                let mut tron = true;
                for (i, &dx) in nhan.iter().enumerate() {
                    let yy = y + i as i64 - r;
                    for xx in x - dx as i64..=x + dx as i64 {
                        let trong = yy >= 0 && yy < h as i64 && xx >= 0 && xx < w as i64;
                        let v = if trong { m[(yy * w as i64 + xx) as usize] != 0 } else { !la_gian };
                        cham |= v;
                        tron &= v;
                    }
                }
                ra[(y * w as i64 + x) as usize] = if la_gian { cham } else { tron } as u8;
            }
        }
        ra
    }

    #[test]
    fn nhan_elip_khop_opencv() {
        // Tổng từng hàng của cv2.getStructuringElement(MORPH_ELLIPSE, (k, k)) — OpenCV 4.12
        let rong = |k| nhan_elip(k).into_iter().map(|d| 2 * d + 1).collect::<Vec<_>>();
        assert_eq!(rong(5), vec![1, 5, 5, 5, 1]);
        assert_eq!(rong(13), vec![1, 7, 9, 11, 13, 13, 13, 13, 13, 11, 9, 7, 1]);
        assert_eq!(rong(25), vec![1, 11, 15, 17, 19, 21, 21, 23, 23, 25, 25, 25, 25, 25, 25, 25, 23, 23, 21, 21, 19, 17, 15, 11, 1]);
        assert_eq!(co_nhan(120.0), 5);
        assert_eq!(co_nhan(200.0), 9); // 8,33 → 8 → 9
        assert_eq!(co_nhan(300.0), 13); // 12,5 → 12 → 13
    }

    #[test]
    fn gian_co_theo_hang_bang_duyet_ca_nhan_ke_ca_sat_bien() {
        let (w, h) = (23, 17);
        let mut hat = 12345u32;
        let m: Vec<u8> = (0..w * h).map(|_| {
            hat = hat.wrapping_mul(1103515245).wrapping_add(12345);
            ((hat >> 16) % 7 == 0) as u8
        }).collect();
        for k in [5, 9, 13] {
            let nhan = nhan_elip(k);
            assert_eq!(gian(&m, w, h, &nhan), tham_chieu(&m, w, h, &nhan, true), "giãn k={k}");
            assert_eq!(co(&m, w, h, &nhan), tham_chieu(&m, w, h, &nhan, false), "co k={k}");
        }
    }

    #[test]
    fn bo_manh_duoi_dien_tich_toi_thieu() {
        let w = 10;
        let mut m = vec![0u8; 100];
        for i in 0..3 {
            m[i] = 1; // mảnh 3 điểm
        }
        for y in 5..10 {
            for x in 5..10 {
                m[y * w + x] = 1; // mảnh 25 điểm
            }
        }
        bo_manh_nho(&mut m, w, 10);
        assert_eq!(m.iter().filter(|&&v| v == 1).count(), 25);
        assert_eq!(m[0], 0);
    }
}
