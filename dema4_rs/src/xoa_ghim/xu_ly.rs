//! Xoá vết ghim trên ảnh trang RGB theo danh sách vùng.
//!
//! Mỗi vùng xoá: điểm xám tối hơn màu nền quá ngưỡng là vết; bỏ nét đen / dấu đỏ-xanh (nếu bật) và
//! phần gần vùng bảo vệ; đóng 2 lần + giãn 3 lần cho liền vết; tô màu nền. Chỉ đọc / ghi trong khung
//! vùng (cộng dải giữa trang để lấy màu nền). Đổi màu → xám / HSV giống hệt OpenCV 4.12 mà XoaGhim dùng.

use super::hinh_thai::{bo_manh_nho, co_nhan, dong_roi_gian, nhan_elip};
use super::vung::{Hcn, LoaiVung, Vung};
use crate::tach_vb::anh::Anh;

/// Lề an toàn quanh vùng bảo vệ (điểm ảnh).
pub const LE_BAO_VE: i64 = 5;
/// Phần còn được xoá nhỏ hơn diện tích này (điểm ảnh) thì bỏ.
const DIEN_TICH_TOI_THIEU: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TuyChon {
    /// Không xoá điểm rất tối (< 50 ở góc / cạnh, < 80 ở vùng vẽ tay) — giữ nét chữ đen.
    pub bao_ve_net_den: bool,
    /// Không xoá điểm đỏ / xanh dương (dấu, chữ ký).
    pub bao_ve_do_xanh: bool,
    /// Tô trắng thay vì màu nền đo được.
    pub nen_trang: bool,
    /// Nhận diện layout (chữ / bảng / hình) để không xoá vào nội dung — đổi kết quả nên vào mã cấu hình.
    pub nhan_dien: bool,
}

impl Default for TuyChon {
    fn default() -> Self {
        TuyChon { bao_ve_net_den: true, bao_ve_do_xanh: false, nen_trang: false, nhan_dien: false }
    }
}

/// Xám 8 bit đúng `cv2.cvtColor(BGR2GRAY)` (OpenCV 4.12: hệ số 15 bit).
#[inline]
pub(super) fn xam(r: u8, g: u8, b: u8) -> i32 {
    ((r as i32 * 9798 + g as i32 * 19235 + b as i32 * 3735 + (1 << 14)) >> 15) as i32
}

/// Điểm đỏ (H < 10 hoặc > 170) hoặc xanh dương (100 < H < 130), S > 50, V > 50 — H/S/V tính như
/// `cv2.cvtColor(BGR2HSV)` 8 bit (H 0..180, bảng chia 12 bit).
fn la_do_xanh(r: u8, g: u8, b: u8) -> bool {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let v = r.max(g).max(b);
    let chenh = v - r.min(g).min(b);
    if v <= 50 || chenh == 0 {
        return false;
    }
    let s = (chenh * ((((255 << 12) as f64) / v as f64).round_ties_even() as i32) + (1 << 11)) >> 12;
    if s <= 50 {
        return false;
    }
    let tu = if v == r { g - b } else if v == g { b - r + 2 * chenh } else { r - g + 4 * chenh };
    let mut h = (tu * ((((180 << 12) as f64) / (6.0 * chenh as f64)).round_ties_even() as i32) + (1 << 11)) >> 12;
    if h < 0 {
        h += 180;
    }
    h < 10 || h > 170 || (h > 100 && h < 130)
}

/// Màu nền [R, G, B]: trung vị từng kênh của dải giữa-phải trang (dòng 1/3–2/3, cột 1/2–3/4),
/// trung vị chẵn lấy trung bình 2 giữa rồi cắt nguyên như `int(np.median(...))`.
fn mau_nen(anh: &Anh) -> [u8; 3] {
    let (w, h) = (anh.rong, anh.cao);
    let (y1, y2, x1, x2) = (h / 3, 2 * h / 3, w / 2, 3 * w / 4);
    let n = (y2 - y1) * (x2 - x1);
    if n == 0 {
        return [255; 3];
    }
    let mut dem = [[0u32; 256]; 3];
    for y in y1..y2 {
        for p in anh.du_lieu[(y * w + x1) * 3..(y * w + x2) * 3].chunks_exact(3) {
            for c in 0..3 {
                dem[c][p[c] as usize] += 1;
            }
        }
    }
    // Giá trị thứ `k` (từ 0) theo thứ tự tăng
    let thu = |d: &[u32; 256], k: usize| {
        let mut cong = 0usize;
        d.iter().position(|&c| {
            cong += c as usize;
            cong > k
        }).unwrap_or(255) as u32
    };
    let mut ra = [0u8; 3];
    for c in 0..3 {
        ra[c] = if n % 2 == 1 { thu(&dem[c], n / 2) } else { (thu(&dem[c], n / 2 - 1) + thu(&dem[c], n / 2)) / 2 } as u8;
    }
    ra
}

/// Mặt nạ (trong khung `k`) phần còn được xoá khi tránh `bao_ve`: bỏ điểm cách vùng bảo vệ chạm khung
/// ≤ `LE_BAO_VE` (góc bo tròn), rồi bỏ mảnh còn lại quá nhỏ. `None` khi không vùng bảo vệ nào chạm khung.
fn phan_duoc_xoa(k: Hcn, bao_ve: &[Hcn]) -> Option<Vec<u8>> {
    let cham: Vec<&Hcn> = bao_ve.iter().filter(|b| b.x1 <= k.x2 && b.x2 >= k.x1 && b.y1 <= k.y2 && b.y2 >= k.y1).collect();
    if cham.is_empty() {
        return None;
    }
    let (w, h) = (k.rong() as usize, k.cao() as usize);
    let mut m = vec![1u8; w * h];
    let le2 = LE_BAO_VE * LE_BAO_VE;
    for b in cham {
        for y in (b.y1 - LE_BAO_VE).max(k.y1)..(b.y2 + LE_BAO_VE + 1).min(k.y2) {
            let dy = (b.y1 - y).max(y - b.y2).max(0);
            for x in (b.x1 - LE_BAO_VE).max(k.x1)..(b.x2 + LE_BAO_VE + 1).min(k.x2) {
                let dx = (b.x1 - x).max(x - b.x2).max(0);
                if dx * dx + dy * dy <= le2 {
                    m[(y - k.y1) as usize * w + (x - k.x1) as usize] = 0;
                }
            }
        }
    }
    bo_manh_nho(&mut m, w, DIEN_TICH_TOI_THIEU);
    Some(m)
}

/// Xoá vết trong khung `k` của 1 vùng. `duoc_xoa`: mặt nạ phần được phép xoá (cùng cỡ khung).
///
/// Nét đen / dấu chỉ bị bỏ khỏi **điểm mầm** của vết; bước giãn sau đó vẫn phủ lên chúng nếu sát vết.
/// Cố ý như vậy: lõi vết ghim / lỗ bấm cũng tối như nét chữ, bỏ chúng cả sau giãn thì vết ghim còn
/// nguyên lõi đen (đã thử trên bộ mẫu 266 trang).
fn xoa_trong_khung(anh: &mut Anh, v: &Vung, k: Hcn, dpi: f64, tc: &TuyChon, duoc_xoa: Option<&[u8]>) -> bool {
    // "Tô nền trắng": tô trắng đặc cả vùng, chỉ chừa điểm được bảo vệ (nét đen, đỏ / xanh, vùng bảo vệ / nhận diện).
    // Không bật: nhận vết so với màu nền đo được rồi tô vết bằng màu nền đó.
    let nen = mau_nen(anh);
    let mau_to = if tc.nen_trang { [255; 3] } else { nen };
    // Python: int(0.114·B + 0.587·G + 0.299·R) trên số thực, cộng theo đúng thứ tự đó
    let nen_xam = (0.114 * nen[2] as f64 + 0.587 * nen[1] as f64 + 0.299 * nen[0] as f64) as i32;
    let nguong_den = if v.vi_tri.la_goc_canh() { 50 } else { 80 };
    let (w, h) = (k.rong() as usize, k.cao() as usize);
    let nguong = v.nguong_vet();
    let mut vet = vec![0u8; w * h];
    let mut giu_lai = vec![0u8; w * h];
    for y in 0..h {
        let dau = ((k.y1 as usize + y) * anh.rong + k.x1 as usize) * 3;
        for (x, p) in anh.du_lieu[dau..dau + w * 3].chunks_exact(3).enumerate() {
            let gx = xam(p[0], p[1], p[2]);
            let i = y * w + x;
            let bao_ve = (tc.bao_ve_net_den && gx < nguong_den) || (tc.bao_ve_do_xanh && la_do_xanh(p[0], p[1], p[2]));
            let giu = bao_ve || duoc_xoa.is_some_and(|m| m[i] == 0);
            let la_vet = tc.nen_trang || nen_xam - gx > nguong;
            vet[i] = (la_vet && !giu) as u8;
            giu_lai[i] = giu as u8;
        }
    }
    let phu = if tc.nen_trang { 0.0 } else { v.phu_nen() };
    if vet.iter().all(|&x| x == 0) && phu <= 0.0 {
        return false;
    }
    if tc.nen_trang {
        return to_mat_na(anh, k, &vet, None, 0.0, mau_to);
    }
    let vet = dong_roi_gian(vet, w, h, &nhan_elip(co_nhan(dpi)), 2, 3);
    // Mép chỗ xoá chuyển dần sang màu giấy (bán kính theo dpi) — không gợn khựng
    let mem = lam_mem(&vet, w, h, ((dpi / 100.0).round() as usize).max(1));
    let mem: Vec<f32> = mem.iter().zip(&giu_lai).map(|(&a, &g)| if g != 0 { 0.0 } else { a }).collect();
    let vet: Vec<u8> = vet.iter().zip(duoc_xoa.map_or(&vec![1u8; w * h][..], |m| m)).map(|(&a, &b)| a & (b != 0) as u8).collect();
    let phu_mat: Vec<u8> = giu_lai.iter().map(|&g| (g == 0) as u8).collect();
    to_mat_na(anh, k, &vet, Some((&mem, &phu_mat)), phu, mau_to)
}

/// Tô trong khung `k`: điểm `vet` = màu tô đặc; còn lại (khi có `mem`) trộn về màu tô theo max(độ mềm mép, `phu`)
/// — chỉ ở điểm `phu_mat` (không bảo vệ). Trả `true` khi có điểm đổi.
fn to_mat_na(anh: &mut Anh, k: Hcn, vet: &[u8], mem: Option<(&[f32], &[u8])>, phu: f32, mau_to: [u8; 3]) -> bool {
    let (w, h) = (k.rong() as usize, k.cao() as usize);
    let mut doi = false;
    for y in 0..h {
        let dau = ((k.y1 as usize + y) * anh.rong + k.x1 as usize) * 3;
        for (x, p) in anh.du_lieu[dau..dau + w * 3].chunks_exact_mut(3).enumerate() {
            let i = y * w + x;
            let a = if vet[i] != 0 {
                1.0
            } else if let Some((m, duoc)) = mem {
                if duoc[i] != 0 { m[i].max(phu) } else { 0.0 }
            } else {
                0.0
            };
            if a <= 0.0 {
                continue;
            }
            let moi = if a >= 0.999 { mau_to } else { std::array::from_fn(|c| (p[c] as f32 + (mau_to[c] as f32 - p[c] as f32) * a).round() as u8) };
            if moi != [p[0], p[1], p[2]] {
                p.copy_from_slice(&moi);
                doi = true;
            }
        }
    }
    doi
}

/// Mặt nạ mềm 0..1 quanh `vet` (ngoài vết): trung bình hộp bán kính `r`, giảm dần ra xa mép.
fn lam_mem(vet: &[u8], w: usize, h: usize, r: usize) -> Vec<f32> {
    // Cộng dồn theo hàng rồi theo cột (hộp (2r+1)²)
    let mut hang = vec![0u32; w * h];
    for y in 0..h {
        let mut tong = 0u32;
        let dong = &vet[y * w..(y + 1) * w];
        for x in 0..(r + 1).min(w) {
            tong += dong[x] as u32;
        }
        for x in 0..w {
            hang[y * w + x] = tong;
            if x + r + 1 < w {
                tong += dong[x + r + 1] as u32;
            }
            if x >= r {
                tong -= dong[x - r] as u32;
            }
        }
    }
    let mut ra = vec![0f32; w * h];
    let dt = ((2 * r + 1) * (2 * r + 1)) as f32;
    for x in 0..w {
        let mut tong = 0u32;
        for y in 0..(r + 1).min(h) {
            tong += hang[y * w + x];
        }
        for y in 0..h {
            ra[y * w + x] = (tong as f32 / dt * 2.0).min(1.0);
            if y + r + 1 < h {
                tong += hang[(y + r + 1) * w + x];
            }
            if y >= r {
                tong -= hang[(y - r) * w + x];
            }
        }
    }
    ra
}

/// Xoá ghim trên ảnh trang RGB (`trang` đếm từ 0, ảnh vẽ ở `dpi`) theo các vùng áp dụng cho trang đó.
/// Khung bảo vệ từ nhận diện layout (khối chữ, bảng…) cho trang `trang`: chỉ khi bật `nhan_dien`, có mô hình và trang có
/// vùng xoá áp dụng. Mô hình lỗi → không khung nào (xoá như khi tắt nhận diện).
pub fn khung_ai(anh: &Anh, vung: &[Vung], trang: u32, tc: &TuyChon) -> Vec<Hcn> {
    if !tc.nhan_dien || !vung.iter().any(|v| v.loai == LoaiVung::Xoa && v.ap_trang.ap_dung(trang)) {
        return Vec::new();
    }
    khung_ai_anh(anh)
}

/// Khung bảo vệ nhận diện layout của cả ảnh (chưa có mô hình / lỗi → trống).
pub fn khung_ai_anh(anh: &Anh) -> Vec<Hcn> {
    if !super::mo_hinh::co_san() {
        return Vec::new();
    }
    super::ai::nhan_dien(anh).map(|h| super::ai::hop_bao_ve(&h)).unwrap_or_default()
}

/// `bao_ve_them`: khung cần giữ ngoài vùng bảo vệ vẽ tay (vd vùng chữ AI), toạ độ điểm ảnh.
/// Thứ tự như XoaGhim: vô đối trước (không xét vùng bảo vệ), rồi từng vùng xoá theo thứ tự danh sách,
/// mỗi vùng thấy kết quả của vùng trước. Trả về `true` nếu có điểm bị đổi.
pub fn xoa_ghim_trang(anh: &mut Anh, vung: &[Vung], trang: u32, dpi: f64, tc: &TuyChon, bao_ve_them: &[Hcn]) -> bool {
    assert_eq!(anh.kenh, 3, "ảnh trang phải là RGB");
    let (w, h) = (anh.rong as i64, anh.cao as i64);
    let ap: Vec<&Vung> = vung.iter().filter(|v| v.ap_trang.ap_dung(trang)).collect();
    let mut doi = false;
    for v in ap.iter().filter(|v| v.loai == LoaiVung::VoDoi) {
        if let Some(k) = v.khung_xoa(w, h, dpi) {
            doi |= xoa_trong_khung(anh, v, k, dpi, tc, None);
        }
    }
    let mut bao_ve: Vec<Hcn> = bao_ve_them.to_vec();
    bao_ve.extend(ap.iter().filter(|v| v.loai == LoaiVung::BaoVe).map(|v| v.khung(w, h, dpi)));
    for v in ap.iter().filter(|v| v.loai == LoaiVung::Xoa) {
        if let Some(k) = v.khung_xoa(w, h, dpi) {
            let duoc_xoa = phan_duoc_xoa(k, &bao_ve);
            doi |= xoa_trong_khung(anh, v, k, dpi, tc, duoc_xoa.as_deref());
        }
    }
    doi
}

#[cfg(test)]
mod tests {
    use super::super::vung::{ApTrang, ViTri};
    use super::*;

    const NEN: [u8; 3] = [240, 238, 230];

    fn trang_trang(w: usize, h: usize) -> Anh {
        Anh { rong: w, cao: h, kenh: 3, du_lieu: NEN.repeat(w * h) }
    }

    fn to(anh: &mut Anh, x1: usize, y1: usize, x2: usize, y2: usize, mau: [u8; 3]) {
        for y in y1..y2 {
            for x in x1..x2 {
                let i = (y * anh.rong + x) * 3;
                anh.du_lieu[i..i + 3].copy_from_slice(&mau);
            }
        }
    }

    fn diem(anh: &Anh, x: usize, y: usize) -> [u8; 3] {
        let i = (y * anh.rong + x) * 3;
        [anh.du_lieu[i], anh.du_lieu[i + 1], anh.du_lieu[i + 2]]
    }

    fn tay(x: f64, y: f64, rong: f64, cao: f64, loai: LoaiVung) -> Vung {
        Vung { vi_tri: ViTri::Tay, loai, ap_trang: ApTrang::TatCa, x, y, rong, cao, rong_px: 0, cao_px: 0, muc: 52 }
    }

    #[test]
    fn xam_va_hsv_khop_opencv() {
        // Giá trị lấy từ cv2.cvtColor (OpenCV 4.12) với ảnh BGR
        assert_eq!(xam(255, 0, 0), 76);
        assert_eq!(xam(0, 255, 0), 150);
        assert_eq!(xam(0, 0, 255), 29);
        assert_eq!(xam(200, 120, 90), 141);
        assert!(la_do_xanh(200, 30, 30)); // đỏ
        assert!(la_do_xanh(30, 60, 200)); // xanh dương
        assert!(!la_do_xanh(30, 200, 30)); // xanh lá
        assert!(!la_do_xanh(120, 110, 108)); // xám ngả đỏ, S thấp
        assert!(!la_do_xanh(40, 5, 5)); // đỏ quá tối (V ≤ 50)
    }

    #[test]
    fn trung_vi_nen_lay_dai_giua_phai_bo_qua_vet_o_goc() {
        let mut a = trang_trang(40, 30);
        to(&mut a, 0, 0, 10, 10, [0, 0, 0]);
        assert_eq!(mau_nen(&a), NEN);
    }

    /// Tô nền trắng: cả vùng trắng đặc (giấy ngả màu cũng trắng), nét đen được bảo vệ giữ nguyên, ngoài vùng giữ nguyên.
    #[test]
    fn to_nen_trang_to_trang_ca_vung_chua_net_bao_ve() {
        let mut a = trang_trang(200, 200);
        to(&mut a, 10, 10, 20, 20, [150, 150, 150]);
        to(&mut a, 40, 40, 44, 44, [10, 10, 10]);
        let v = tay(0.0, 0.0, 0.3, 0.3, LoaiVung::Xoa);
        assert!(xoa_ghim_trang(&mut a, &[v], 0, 120.0, &TuyChon { nen_trang: true, ..Default::default() }, &[]));
        assert_eq!(diem(&a, 15, 15), [255; 3], "vết tô trắng");
        assert_eq!(diem(&a, 50, 50), [255; 3], "giấy trong vùng: trắng đặc");
        assert_eq!(diem(&a, 42, 42), [10; 3], "nét đen được bảo vệ");
        assert_eq!(diem(&a, 150, 150), NEN, "ngoài vùng: giữ màu giấy");
    }

    #[test]
    fn xoa_vet_xam_trong_vung_giu_ngoai_vung() {
        let mut a = trang_trang(200, 200);
        to(&mut a, 10, 10, 20, 20, [150, 150, 150]); // vết ghim xám trong vùng
        to(&mut a, 150, 150, 160, 160, [150, 150, 150]); // ngoài vùng
        let v = tay(0.0, 0.0, 0.3, 0.3, LoaiVung::Xoa);
        assert!(xoa_ghim_trang(&mut a, &[v], 0, 120.0, &TuyChon::default(), &[]));
        assert_eq!(diem(&a, 15, 15), NEN);
        assert_eq!(diem(&a, 155, 155), [150, 150, 150]);
    }

    #[test]
    fn net_den_xa_vet_duoc_giu_sat_vet_thi_bi_gian_phu_nhu_xoaghim() {
        let lam = |bat: bool| {
            let mut a = trang_trang(200, 200);
            to(&mut a, 10, 10, 30, 30, [150, 150, 150]); // vết xám
            to(&mut a, 32, 10, 34, 30, [20, 20, 20]); // nét đen sát vết (cách 2 điểm)
            to(&mut a, 60, 60, 63, 70, [20, 20, 20]); // nét đen xa vết
            let v = tay(0.0, 0.0, 0.4, 0.4, LoaiVung::Xoa);
            xoa_ghim_trang(&mut a, &[v], 0, 120.0, &TuyChon { bao_ve_net_den: bat, ..Default::default() }, &[]);
            a
        };
        let bat = lam(true);
        assert_eq!(diem(&bat, 61, 65), [20, 20, 20]);
        assert_eq!(diem(&bat, 33, 20), NEN, "giãn từ vết phủ cả nét đen sát bên");
        assert_eq!(diem(&bat, 20, 20), NEN);
        assert_eq!(diem(&lam(false), 61, 65), NEN, "tắt bảo vệ: nét đen cũng là vết");
    }

    #[test]
    fn dau_do_chi_giu_khi_bat_bao_ve_do_xanh() {
        let lam = |bat: bool| {
            let mut a = trang_trang(200, 200);
            to(&mut a, 10, 10, 30, 30, [200, 40, 40]);
            xoa_ghim_trang(&mut a, &[tay(0.0, 0.0, 0.3, 0.3, LoaiVung::Xoa)], 0, 120.0, &TuyChon { bao_ve_do_xanh: bat, ..Default::default() }, &[]);
            diem(&a, 20, 20)
        };
        assert_eq!(lam(true), [200, 40, 40]);
        assert_eq!(lam(false), NEN);
    }

    #[test]
    fn vung_bao_ve_khong_bi_dung_vo_doi_thi_xoa_ca_trong_do() {
        let lam = |loai_xoa: LoaiVung| {
            let mut a = trang_trang(200, 200);
            to(&mut a, 10, 10, 40, 40, [150, 150, 150]);
            let vung = [tay(0.0, 0.0, 0.5, 0.5, loai_xoa), tay(0.1, 0.1, 0.1, 0.1, LoaiVung::BaoVe)];
            xoa_ghim_trang(&mut a, &vung, 0, 120.0, &TuyChon::default(), &[]);
            a
        };
        let a = lam(LoaiVung::Xoa);
        assert_eq!(diem(&a, 25, 25), [150, 150, 150], "trong vùng bảo vệ");
        assert_eq!(diem(&a, 16, 25), [150, 150, 150], "trong lề 5 điểm quanh vùng bảo vệ");
        assert_eq!(diem(&a, 12, 12), NEN);
        let b = lam(LoaiVung::VoDoi);
        assert_eq!(diem(&b, 25, 25), NEN);
    }

    #[test]
    fn khung_bao_ve_them_tu_ngoai_cung_duoc_tranh() {
        let mut a = trang_trang(200, 200);
        to(&mut a, 10, 10, 40, 40, [150, 150, 150]);
        let ai = Hcn { x1: 20, y1: 20, x2: 30, y2: 30 };
        xoa_ghim_trang(&mut a, &[tay(0.0, 0.0, 0.5, 0.5, LoaiVung::Xoa)], 0, 120.0, &TuyChon::default(), &[ai]);
        assert_eq!(diem(&a, 25, 25), [150, 150, 150]);
        assert_eq!(diem(&a, 11, 11), NEN);
    }

    #[test]
    fn chi_ap_vung_cua_trang_dang_xu_ly() {
        let mut a = trang_trang(200, 200);
        to(&mut a, 10, 10, 20, 20, [150, 150, 150]);
        let v = Vung { ap_trang: ApTrang::Chan, ..tay(0.0, 0.0, 0.3, 0.3, LoaiVung::Xoa) };
        assert!(!xoa_ghim_trang(&mut a, std::slice::from_ref(&v), 0, 120.0, &TuyChon::default(), &[]));
        assert!(xoa_ghim_trang(&mut a, &[v], 1, 120.0, &TuyChon::default(), &[]));
    }
}

#[cfg(test)]
mod tests_muc_xoa {
    use super::*;
    use crate::xoa_ghim::vung::{muc_tu_nguong, ApTrang, LoaiVung, ViTri};

    const GIAY: [u8; 3] = [236, 230, 214];

    fn giay(w: usize, h: usize) -> Anh {
        Anh { rong: w, cao: h, kenh: 3, du_lieu: GIAY.repeat(w * h) }
    }

    fn vung(muc: u8) -> Vung {
        Vung { vi_tri: ViTri::Tay, loai: LoaiVung::Xoa, ap_trang: ApTrang::TatCa, x: 0.0, y: 0.0, rong: 0.5, cao: 0.5, rong_px: 0, cao_px: 0, muc }
    }

    fn diem(a: &Anh, x: usize, y: usize) -> [u8; 3] {
        let i = (y * a.rong + x) * 3;
        [a.du_lieu[i], a.du_lieu[i + 1], a.du_lieu[i + 2]]
    }

    /// Ngưỡng cũ 1–30 đổi sang mức xoá rồi đổi lại đúng ngưỡng đó.
    #[test]
    fn muc_xoa_doi_qua_lai_dung_nguong_cu() {
        for n in 1..=30u8 {
            let v = Vung { muc: muc_tu_nguong(n), ..vung(0) };
            assert_eq!(v.nguong_vet(), n as i32, "ngưỡng {n} → mức {}", v.muc);
            assert_eq!(v.phu_nen(), 0.0);
        }
    }

    /// Mức 100: cả vùng một màu giấy đặc (giấy lốm đốm cũng phẳng); ngoài vùng giữ nguyên.
    #[test]
    fn muc_100_phu_dac_mau_giay() {
        let mut a = giay(200, 200);
        // Đốm giấy hơi sáng (không phải vết) trong vùng
        for x in 30..60 {
            let i = (40 * 200 + x) * 3;
            a.du_lieu[i..i + 3].copy_from_slice(&[250, 248, 240]);
        }
        let mut b = a.clone();
        assert!(xoa_ghim_trang(&mut a, &[vung(100)], 0, 120.0, &TuyChon::default(), &[]));
        assert_eq!(diem(&a, 45, 40), GIAY, "mức 100: đốm sáng thành màu giấy");
        assert_eq!(diem(&a, 150, 150), GIAY, "ngoài vùng giữ nguyên");
        // Mức 52 (ngưỡng 5 cũ): đốm sáng không phải vết → giữ nguyên
        xoa_ghim_trang(&mut b, &[vung(52)], 0, 120.0, &TuyChon::default(), &[]);
        assert_eq!(diem(&b, 45, 40), [250, 248, 240]);
    }

    /// Mép chỗ xoá chuyển dần: điểm ngay ngoài vết đã giãn được trộn một phần về màu giấy, không đổi đột ngột.
    #[test]
    fn mep_cho_xoa_chuyen_dan() {
        let mut a = giay(200, 200);
        let toi = [150, 150, 150];
        let nhat = [228, 222, 206]; // tối hơn giấy ~8: không phải vết ở mức 20 (≈ ngưỡng 20)
        for y in 40..60 {
            for x in 40..60 {
                let i = (y * 200 + x) * 3;
                a.du_lieu[i..i + 3].copy_from_slice(&toi);
            }
        }
        // Dải nền hơi tối quanh vết (không đủ là vết ở mức 20 ≈ ngưỡng 20)
        for y in 20..80 {
            for x in 20..80 {
                let i = (y * 200 + x) * 3;
                if a.du_lieu[i] != 150 {
                    a.du_lieu[i..i + 3].copy_from_slice(&nhat);
                }
            }
        }
        assert!(xoa_ghim_trang(&mut a, &[vung(20)], 0, 120.0, &TuyChon::default(), &[]));
        assert_eq!(diem(&a, 50, 50), GIAY, "lõi vết thành màu giấy");
        // Từ trong ra ngoài: màu tiến dần từ màu giấy về màu nền tối hơn, không nhảy một bước
        let mut truoc = diem(&a, 50, 50)[0] as i32;
        let mut buoc_lon = 0;
        for x in 51..80 {
            let c = diem(&a, x, 50)[0] as i32;
            buoc_lon = buoc_lon.max((c - truoc).abs());
            truoc = c;
        }
        assert!(buoc_lon < (GIAY[0] as i32 - nhat[0] as i32), "có chuyển dần ở mép (bước lớn nhất {buoc_lon})");
    }
}
