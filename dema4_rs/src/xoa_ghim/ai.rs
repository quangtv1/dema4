//! Nhận diện layout trang (khối chữ, bảng, tiêu đề…) bằng YOLOv12s-DocLayNet (ONNX) để bảo vệ chữ khi xoá ghim.
//! Cùng cách làm `YOLODocLayNetONNXDetector` của XoaGhim (Python):
//! letterbox 1024 (song tuyến như `cv2.resize`, đệm 114, căn giữa, RGB / 255, CHW) → mô hình →
//! đầu ra `[1, 15, 21504]` (4 hộp + 11 lớp) → ngưỡng 0,1 → đổi về toạ độ trang, kẹp → NMS IoU 0,5 theo lớp.
//!
//! Chạy chậm (~0,3 s / trang trên Mac): **không gọi trên luồng giao diện**. Phiên mô hình dùng chung
//! (khoá `Mutex`, luồng ONNX = 80% số nhân), nạp lần đầu từ `mo_hinh::duong_dan()`.
//! Không lưu kết quả xuống đĩa; nhớ tạm hộp theo (PDF, trang) là việc của nơi gọi.

use std::path::Path;
use std::sync::Mutex;

use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;

use super::vung::Hcn;
use crate::tach_vb::anh::Anh;

/// Cạnh ảnh vào mô hình.
pub const CANH: usize = 1024;
/// Màu đệm letterbox (như Ultralytics).
const DEM: u8 = 114;
/// Số lớp DocLayNet.
const SO_LOP: usize = 11;
/// Ngưỡng tin cậy (giữ hộp có tin cậy lớn hơn hẳn ngưỡng).
pub const NGUONG_TIN_CAY: f32 = 0.1;
/// Hai hộp cùng lớp chồng nhau từ ngưỡng này trở lên thì bỏ hộp tin cậy thấp hơn.
const NGUONG_IOU: f32 = 0.5;

/// 11 lớp DocLayNet, đúng thứ tự đầu ra mô hình.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LopAi {
    ChuThichHinh,
    ChuThichChan,
    CongThuc,
    MucDanhSach,
    ChanTrang,
    DauTrang,
    HinhAnh,
    TieuDeMuc,
    Bang,
    Chu,
    TieuDe,
}

impl LopAi {
    const THEO_SO: [LopAi; SO_LOP] = [
        LopAi::ChuThichHinh,
        LopAi::ChuThichChan,
        LopAi::CongThuc,
        LopAi::MucDanhSach,
        LopAi::ChanTrang,
        LopAi::DauTrang,
        LopAi::HinhAnh,
        LopAi::TieuDeMuc,
        LopAi::Bang,
        LopAi::Chu,
        LopAi::TieuDe,
    ];

    pub fn tu_so(i: usize) -> Option<LopAi> {
        Self::THEO_SO.get(i).copied()
    }

    pub fn ten(self) -> &'static str {
        match self {
            LopAi::ChuThichHinh => "Chú thích",
            LopAi::ChuThichChan => "Chú thích chân",
            LopAi::CongThuc => "Công thức",
            LopAi::MucDanhSach => "Mục danh sách",
            LopAi::ChanTrang => "Chân trang",
            LopAi::DauTrang => "Đầu trang",
            LopAi::HinhAnh => "Hình ảnh",
            LopAi::TieuDeMuc => "Tiêu đề mục",
            LopAi::Bang => "Bảng",
            LopAi::Chu => "Chữ",
            LopAi::TieuDe => "Tiêu đề",
        }
    }

    /// Mặc định của XoaGhim (`TextProtectionOptions`): tiêu đề, chữ, bảng, chú thích, chú thích chân, công thức.
    /// Không bảo vệ hình ảnh, đầu / chân trang.
    pub fn bao_ve(self) -> bool {
        !matches!(self, LopAi::HinhAnh | LopAi::DauTrang | LopAi::ChanTrang)
    }
}

/// Một khối nhận diện được, toạ độ điểm ảnh của ảnh trang đưa vào (cắt phần lẻ như `int()` của Python).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HopAi {
    pub hcn: Hcn,
    pub lop: LopAi,
    pub tin_cay: f32,
}

/// Nhận diện mọi khối trên ảnh trang (RGB hoặc xám). Lỗi khi chưa có / không nạp được mô hình.
/// Chặn tới khi xong (~0,3 s); các luồng gọi cùng lúc dùng chung 1 phiên nên chạy lần lượt.
pub fn nhan_dien(anh: &Anh) -> Result<Vec<HopAi>, String> {
    static PHIEN: Mutex<Option<Session>> = Mutex::new(None);
    let dv = letterbox(anh);
    let y = {
        let mut khoa = PHIEN.lock().unwrap_or_else(|e| e.into_inner());
        if khoa.is_none() {
            if !super::mo_hinh::co_san() {
                return Err("Chưa có mô hình nhận diện layout".into());
            }
            *khoa = Some(tao_phien(&super::mo_hinh::duong_dan())?);
        }
        chay(khoa.as_mut().unwrap(), dv.tensor)?
    };
    Ok(hau_xu_ly(&y, dv.ty_le, dv.le_x, dv.le_y, anh.rong, anh.cao))
}

/// Khung cần giữ của các khối thuộc nhóm bảo vệ — đưa thẳng vào `xoa_ghim_trang(.., bao_ve_them)`.
/// Không nới ở đây: `xoa_ghim_trang` đã nới mọi khung bảo vệ thêm `LE_BAO_VE` (5 điểm ảnh) như XoaGhim.
pub fn hop_bao_ve(hop: &[HopAi]) -> Vec<Hcn> {
    hop.iter().filter(|h| h.lop.bao_ve()).map(|h| h.hcn).collect()
}

/// Nạp mô hình từ file; luồng ONNX = 80% số nhân (như XoaGhim — chừa nhân cho giao diện).
pub(crate) fn tao_phien(p: &Path) -> Result<Session, String> {
    let nhan = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    Session::builder()
        .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level3))
        .and_then(|b| b.with_intra_threads((nhan * 4 / 5).max(1)))
        .and_then(|b| b.with_inter_threads(1))
        .and_then(|b| b.commit_from_file(p))
        .map_err(|e| format!("Không nạp được mô hình nhận diện layout: {e}"))
}

/// Chạy mô hình trên tensor 1×3×1024×1024; trả đầu ra thô 15 × số neo.
pub(crate) fn chay(phien: &mut Session, x: Vec<f32>) -> Result<Vec<f32>, String> {
    let vao = Tensor::from_array(([1usize, 3, CANH, CANH], x)).map_err(|e| e.to_string())?;
    let ra = phien.run(ort::inputs![vao]).map_err(|e| e.to_string())?;
    let (_, y) = ra[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
    Ok(y.to_vec())
}

/// Ảnh vào mô hình + phép biến đổi để đổi hộp về toạ độ trang.
pub(crate) struct DauVao {
    pub tensor: Vec<f32>,
    pub ty_le: f64,
    pub le_x: usize,
    pub le_y: usize,
}

/// Thu ảnh vào khung 1024 giữ tỉ lệ, căn giữa, đệm 114; RGB / 255, CHW (như `_preprocess` của XoaGhim).
pub(crate) fn letterbox(anh: &Anh) -> DauVao {
    let (w, h) = (anh.rong, anh.cao);
    let ty_le = (CANH as f64 / h as f64).min(CANH as f64 / w as f64);
    let rm = ((w as f64 * ty_le) as usize).clamp(1, CANH);
    let cm = ((h as f64 * ty_le) as usize).clamp(1, CANH);
    let nho = song_tuyen(anh, rm, cm);
    let (le_x, le_y) = ((CANH - rm) / 2, (CANH - cm) / 2);
    let mat = CANH * CANH;
    let mut tensor = vec![DEM as f32 / 255.0; 3 * mat];
    for y in 0..cm {
        for x in 0..rm {
            let o = (y + le_y) * CANH + x + le_x;
            for c in 0..3 {
                tensor[c * mat + o] = nho[(y * rm + x) * 3 + c] as f32 / 255.0;
            }
        }
    }
    DauVao { tensor, ty_le, le_x, le_y }
}

/// Hệ số nội suy 1 chiều như `cv2.resize(INTER_LINEAR)` 8 bit: tâm điểm lệch ½, kẹp mép, trọng số 11 bit.
fn he_so(nguon: usize, dich: usize) -> Vec<(usize, usize, i32, i32)> {
    let ty = 1.0 / (dich as f64 / nguon as f64);
    (0..dich)
        .map(|d| {
            let mut f = ((d as f64 + 0.5) * ty - 0.5) as f32;
            let mut s = f.floor() as i64;
            f -= s as f32;
            if s < 0 {
                (f, s) = (0.0, 0);
            }
            if s >= nguon as i64 - 1 {
                (f, s) = (0.0, nguon as i64 - 1);
            }
            let s = s as usize;
            (s, (s + 1).min(nguon - 1), ((1.0 - f) * 2048.0).round_ties_even() as i32, (f * 2048.0).round_ties_even() as i32)
        })
        .collect()
}

/// Đổi cỡ ảnh về `rm` × `cm` RGB bằng nội suy song tuyến điểm cố định, khớp `cv2.resize` (nhánh SIMD) tới ±1.
fn song_tuyen(anh: &Anh, rm: usize, cm: usize) -> Vec<u8> {
    let k = anh.kenh;
    let kenh = |c: usize| if k >= 3 { c } else { 0 };
    let (cot, hang) = (he_so(anh.rong, rm), he_so(anh.cao, cm));
    let ngang = |sy: usize| -> Vec<i32> {
        let dong = &anh.du_lieu[sy * anh.rong * k..(sy + 1) * anh.rong * k];
        let mut d = Vec::with_capacity(rm * 3);
        for &(s0, s1, a0, a1) in &cot {
            for c in 0..3 {
                d.push(dong[s0 * k + kenh(c)] as i32 * a0 + dong[s1 * k + kenh(c)] as i32 * a1);
            }
        }
        d
    };
    let mut ra = vec![0u8; rm * cm * 3];
    let mut nho: Option<(usize, Vec<i32>, Vec<i32>)> = None;
    for (y, &(s0, s1, b0, b1)) in hang.iter().enumerate() {
        let (r0, r1) = match nho.take() {
            Some((s, r0, r1)) if s == s0 => (r0, r1),
            Some((s, _, r1)) if s + 1 == s0 && s1 == s0 + 1 => (r1, ngang(s1)),
            _ => (ngang(s0), ngang(s1)),
        };
        for (o, (&p0, &p1)) in ra[y * rm * 3..(y + 1) * rm * 3].iter_mut().zip(r0.iter().zip(&r1)) {
            let v = (((p0 >> 4) * b0) >> 16) + (((p1 >> 4) * b1) >> 16);
            *o = ((v + 2) >> 2).clamp(0, 255) as u8;
        }
        nho = Some((s0, r0, r1));
    }
    ra
}

/// Đầu ra thô `[15, số neo]` → hộp toạ độ trang `rong` × `cao` (như `_postprocess` + `_nms` của XoaGhim).
pub(crate) fn hau_xu_ly(y: &[f32], ty_le: f64, le_x: usize, le_y: usize, rong: usize, cao: usize) -> Vec<HopAi> {
    let n = y.len() / (4 + SO_LOP);
    let (ty, lx, ly, w, h) = (ty_le as f32, le_x as f32, le_y as f32, rong as f32, cao as f32);
    let mut ung_vien: Vec<([f32; 4], f32, usize)> = Vec::new();
    for a in 0..n {
        let mut lop = 0;
        for l in 1..SO_LOP {
            if y[(4 + l) * n + a] > y[(4 + lop) * n + a] {
                lop = l;
            }
        }
        let tc = y[(4 + lop) * n + a];
        if tc <= NGUONG_TIN_CAY {
            continue;
        }
        let (cx, cy, bw, bh) = (y[a], y[n + a], y[2 * n + a], y[3 * n + a]);
        let x1 = ((cx - bw / 2.0 - lx) / ty).clamp(0.0, w);
        let y1 = ((cy - bh / 2.0 - ly) / ty).clamp(0.0, h);
        let x2 = ((cx + bw / 2.0 - lx) / ty).clamp(0.0, w);
        let y2 = ((cy + bh / 2.0 - ly) / ty).clamp(0.0, h);
        ung_vien.push(([x1, y1, x2, y2], tc, lop));
    }
    nms(ung_vien)
        .into_iter()
        .map(|(b, tin_cay, lop)| HopAi {
            hcn: Hcn { x1: b[0] as i64, y1: b[1] as i64, x2: b[2] as i64, y2: b[3] as i64 },
            lop: LopAi::THEO_SO[lop],
            tin_cay,
        })
        .collect()
}

/// NMS tham lam theo tin cậy giảm dần (sắp ổn định), chỉ loại hộp cùng lớp có IoU ≥ 0,5.
fn nms(mut ds: Vec<([f32; 4], f32, usize)>) -> Vec<([f32; 4], f32, usize)> {
    ds.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut giu: Vec<([f32; 4], f32, usize)> = Vec::new();
    for d in ds {
        if giu.iter().all(|g| g.2 != d.2 || iou(&g.0, &d.0) < NGUONG_IOU) {
            giu.push(d);
        }
    }
    giu
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let giao = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0) * (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let hop = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - giao;
    if hop > 0.0 {
        giao / hop
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anh_mau(w: usize, h: usize, mau: [u8; 3]) -> Anh {
        Anh { rong: w, cao: h, kenh: 3, du_lieu: mau.repeat(w * h) }
    }

    fn diem(t: &[f32], c: usize, x: usize, y: usize) -> f32 {
        t[c * CANH * CANH + y * CANH + x]
    }

    #[test]
    fn letterbox_trang_doc_can_giua_dem_114() {
        let dv = letterbox(&anh_mau(2480, 3508, [10, 200, 30]));
        let rm = (2480.0 * (1024.0 / 3508.0)) as usize;
        assert_eq!((dv.le_y, dv.le_x), (0, (CANH - rm) / 2));
        assert!((dv.ty_le - 1024.0 / 3508.0).abs() < 1e-12);
        // Đệm hai bên, ảnh ở giữa đúng màu (ảnh 1 màu → song tuyến giữ nguyên)
        assert_eq!(diem(&dv.tensor, 0, 0, 500), 114.0 / 255.0);
        assert_eq!(diem(&dv.tensor, 2, CANH - 1, 500), 114.0 / 255.0);
        assert_eq!(diem(&dv.tensor, 0, dv.le_x, 0), 10.0 / 255.0);
        assert_eq!(diem(&dv.tensor, 1, dv.le_x + rm - 1, CANH - 1), 200.0 / 255.0);
        assert_eq!(diem(&dv.tensor, 2, CANH / 2, CANH / 2), 30.0 / 255.0);
        assert_eq!(diem(&dv.tensor, 0, dv.le_x - 1, 500), 114.0 / 255.0);
    }

    #[test]
    fn letterbox_trang_ngang_va_anh_xam() {
        let a = Anh { rong: 3000, cao: 2000, kenh: 1, du_lieu: vec![77; 3000 * 2000] };
        let dv = letterbox(&a);
        let cm = (2000.0 * (1024.0 / 3000.0)) as usize;
        assert_eq!((dv.le_x, dv.le_y), (0, (CANH - cm) / 2));
        for c in 0..3 {
            assert_eq!(diem(&dv.tensor, c, 100, CANH / 2), 77.0 / 255.0, "xám nhân 3 kênh");
            assert_eq!(diem(&dv.tensor, c, 100, 0), 114.0 / 255.0);
        }
    }

    #[test]
    fn song_tuyen_giong_cv2_tren_doc_tuyen_tinh() {
        // Dải ngang 0..255 thu 4 lần: điểm đích d lấy tâm nguồn 4d + 1,5 → giá trị ≈ trung bình 2 điểm giữa
        let w = 256;
        let du_lieu: Vec<u8> = (0..4).flat_map(|_| (0..w).flat_map(|x| [x as u8; 3])).collect();
        let a = Anh { rong: w, cao: 4, kenh: 3, du_lieu };
        let r = song_tuyen(&a, 64, 1);
        for d in 0..64 {
            let mong = 4.0 * d as f32 + 1.5;
            assert!((r[d * 3] as f32 - mong).abs() <= 1.0, "điểm {d}: {} ≠ {mong}", r[d * 3]);
        }
    }

    /// Đầu ra giả: mỗi hộp (cx, cy, w, h trong khung 1024, lớp, tin cậy).
    fn dau_ra_gia(hop: &[(f32, f32, f32, f32, usize, f32)], n: usize) -> Vec<f32> {
        let mut y = vec![0f32; (4 + SO_LOP) * n];
        for (a, &(cx, cy, w, h, lop, tc)) in hop.iter().enumerate() {
            y[a] = cx;
            y[n + a] = cy;
            y[2 * n + a] = w;
            y[3 * n + a] = h;
            y[(4 + lop) * n + a] = tc;
            // lớp khác có điểm thấp hơn → argmax đúng lớp
            y[(4 + (lop + 1) % SO_LOP) * n + a] = tc / 2.0;
        }
        y
    }

    #[test]
    fn nms_theo_lop_va_nguong() {
        let y = dau_ra_gia(
            &[
                (500.0, 500.0, 200.0, 100.0, 9, 0.9),  // chữ, giữ
                (505.0, 500.0, 200.0, 100.0, 9, 0.8),  // chữ chồng nhiều → bỏ
                (505.0, 500.0, 200.0, 100.0, 8, 0.7),  // bảng chồng chữ → giữ (khác lớp)
                (500.0, 700.0, 200.0, 100.0, 10, 0.1), // đúng ngưỡng → bỏ (cần lớn hơn hẳn)
                (500.0, 900.0, 100.0, 50.0, 10, 0.11), // giữ
            ],
            8,
        );
        let hop = hau_xu_ly(&y, 1.0, 0, 0, 1024, 1024);
        let tom: Vec<(LopAi, f32)> = hop.iter().map(|h| (h.lop, h.tin_cay)).collect();
        assert_eq!(tom, vec![(LopAi::Chu, 0.9), (LopAi::Bang, 0.7), (LopAi::TieuDe, 0.11)]);
        assert_eq!(hop[0].hcn, Hcn { x1: 400, y1: 450, x2: 600, y2: 550 });
    }

    #[test]
    fn doi_toa_do_ve_trang_doc_va_ngang() {
        // Trang dọc 2480 × 3508: tỉ lệ 1024/3508, lề trái
        let dv = letterbox(&anh_mau(2480, 3508, [255; 3]));
        let ty = dv.ty_le as f32;
        let lx = dv.le_x as f32;
        // Hộp trang (248, 351)–(1240, 1754) đưa sang khung 1024 rồi đổi ngược
        let (x1, y1, x2, y2) = (248.0f32, 351.0f32, 1240.0f32, 1754.0f32);
        let khung = |v: f32, le: f32| v * ty + le;
        let (cx, cy) = ((khung(x1, lx) + khung(x2, lx)) / 2.0, (khung(y1, 0.0) + khung(y2, 0.0)) / 2.0);
        let (bw, bh) = (khung(x2, lx) - khung(x1, lx), khung(y2, 0.0) - khung(y1, 0.0));
        let y = dau_ra_gia(&[(cx, cy, bw, bh, 9, 0.5), (lx - 20.0, 30.0, 100.0, 100.0, 8, 0.5)], 4);
        let hop = hau_xu_ly(&y, dv.ty_le, dv.le_x, dv.le_y, 2480, 3508);
        let h = hop[0].hcn;
        for (a, b) in [(h.x1, 248), (h.y1, 351), (h.x2, 1240), (h.y2, 1754)] {
            assert!((a - b).abs() <= 1, "{h:?}");
        }
        // Hộp tràn sang phần đệm trái → kẹp về mép trang
        assert_eq!((hop[1].hcn.x1, hop[1].hcn.y1), (0, 0));

        // Trang ngang 3508 × 2480: lề trên
        let dv = letterbox(&anh_mau(3508, 2480, [255; 3]));
        assert_eq!(dv.le_x, 0);
        let (ty, ly) = (dv.ty_le as f32, dv.le_y as f32);
        let y = dau_ra_gia(&[(1754.0 * ty, 1240.0 * ty + ly, 702.0 * ty, 496.0 * ty, 10, 0.6), (512.0, CANH as f32 - 5.0, 50.0, 60.0, 9, 0.4)], 4);
        let hop = hau_xu_ly(&y, dv.ty_le, dv.le_x, dv.le_y, 3508, 2480);
        let h = hop[0].hcn;
        for (a, b) in [(h.x1, 1403), (h.y1, 992), (h.x2, 2105), (h.y2, 1488)] {
            assert!((a - b).abs() <= 1, "{h:?}");
        }
        assert_eq!(hop[1].hcn.y2, 2480, "tràn xuống phần đệm dưới → kẹp về đáy trang");
    }

    #[test]
    fn nhom_bao_ve_nhu_xoaghim() {
        let bv: Vec<LopAi> = LopAi::THEO_SO.iter().copied().filter(|l| l.bao_ve()).collect();
        assert_eq!(bv.len(), 8);
        let hop = [
            HopAi { hcn: Hcn { x1: 0, y1: 0, x2: 5, y2: 5 }, lop: LopAi::Chu, tin_cay: 0.5 },
            HopAi { hcn: Hcn { x1: 1, y1: 1, x2: 5, y2: 5 }, lop: LopAi::HinhAnh, tin_cay: 0.5 },
            HopAi { hcn: Hcn { x1: 2, y1: 2, x2: 5, y2: 5 }, lop: LopAi::DauTrang, tin_cay: 0.5 },
            HopAi { hcn: Hcn { x1: 3, y1: 3, x2: 5, y2: 5 }, lop: LopAi::TieuDeMuc, tin_cay: 0.5 },
        ];
        assert_eq!(hop_bao_ve(&hop), vec![hop[0].hcn, hop[3].hcn]);
    }

    // ---- Đối chiếu với XoaGhim (Python) — chạy tay ----
    // DEMA4_MO_HINH_XG=<yolov12s-doclaynet.onnx> DEMA4_DOI_CHIEU_AI=<thư mục do sinh_doi_chieu_ai.py tạo> \
    //   cargo test --release doi_chieu_ai -- --ignored --nocapture

    fn doc_npy(p: &Path) -> Vec<f32> {
        let b = std::fs::read(p).unwrap();
        assert_eq!(&b[..6], b"\x93NUMPY");
        let dau = 10 + u16::from_le_bytes([b[8], b[9]]) as usize;
        assert!(std::str::from_utf8(&b[10..dau]).unwrap().contains("'<f4'"), "{}", p.display());
        b[dau..].chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
    }

    fn doc_png(p: &Path) -> Anh {
        let mut gm = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(p).unwrap())).read_info().unwrap();
        let mut buf = vec![0; gm.output_buffer_size().unwrap()];
        let tt = gm.next_frame(&mut buf).unwrap();
        assert_eq!(tt.color_type, png::ColorType::Rgb);
        buf.truncate(tt.buffer_size());
        Anh { rong: tt.width as usize, cao: tt.height as usize, kenh: 3, du_lieu: buf }
    }

    fn iou_hcn(a: &Hcn, b: &Hcn) -> f32 {
        let f = |h: &Hcn| [h.x1 as f32, h.y1 as f32, h.x2 as f32, h.y2 as f32];
        iou(&f(a), &f(b))
    }

    #[test]
    #[ignore]
    fn doi_chieu_ai_voi_python() {
        let mo_hinh = std::env::var("DEMA4_MO_HINH_XG").expect("DEMA4_MO_HINH_XG");
        let goc = std::path::PathBuf::from(std::env::var("DEMA4_DOI_CHIEU_AI").expect("DEMA4_DOI_CHIEU_AI"));
        let t = std::time::Instant::now();
        let mut phien = tao_phien(Path::new(&mo_hinh)).unwrap();
        println!("nạp mô hình: {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        let mau: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(goc.join("mau.json")).unwrap()).unwrap();
        let (mut khop_tong, mut rust_tong, mut py_tong) = (0, 0, 0);
        for m in mau.as_array().unwrap() {
            let d = goc.join(m["ten"].as_str().unwrap());
            let anh = doc_png(&d.join("vao.png"));
            let x_py = doc_npy(&d.join("x.npy"));
            let y_py = doc_npy(&d.join("y.npy"));

            // (a) cùng tensor vào → đầu ra thô
            let y_rs = chay(&mut phien, x_py.clone()).unwrap();
            let lech_ra = y_rs.iter().zip(&y_py).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
            // (b) letterbox Rust ↔ Python
            let dv = letterbox(&anh);
            let lech: Vec<f32> = dv.tensor.iter().zip(&x_py).map(|(a, b)| (a - b).abs()).collect();
            let tb = lech.iter().sum::<f32>() / lech.len() as f32;
            let max = lech.iter().copied().fold(0f32, f32::max);
            let khac = lech.iter().filter(|&&v| v > 0.0).count();
            // (c) cả chuỗi Rust ↔ hộp Python, tin cậy ≥ 0,25
            let t = std::time::Instant::now();
            let (ty_le, le_x, le_y) = (dv.ty_le, dv.le_x, dv.le_y);
            let y = chay(&mut phien, dv.tensor).unwrap();
            let hop = hau_xu_ly(&y, ty_le, le_x, le_y, anh.rong, anh.cao);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let py: Vec<(Hcn, usize, f32)> = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(d.join("hop.json")).unwrap()).unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|h| {
                    let f = |k: &str| h[k].as_f64().unwrap();
                    (Hcn { x1: f("x1") as i64, y1: f("y1") as i64, x2: f("x2") as i64, y2: f("y2") as i64 }, h["lop"].as_u64().unwrap() as usize, f("tin_cay") as f32)
                })
                .collect();
            let rs: Vec<&HopAi> = hop.iter().filter(|h| h.tin_cay >= 0.25).collect();
            let py25: Vec<&(Hcn, usize, f32)> = py.iter().filter(|h| h.2 >= 0.25).collect();
            let mut da_dung = vec![false; rs.len()];
            let mut khop = 0;
            let mut lech_toa_do = 0i64;
            for p in &py25 {
                if let Some(i) = (0..rs.len()).find(|&i| !da_dung[i] && LopAi::THEO_SO[p.1] == rs[i].lop && iou_hcn(&p.0, &rs[i].hcn) >= 0.9) {
                    da_dung[i] = true;
                    khop += 1;
                    let (a, b) = (p.0, rs[i].hcn);
                    lech_toa_do = lech_toa_do.max((a.x1 - b.x1).abs().max((a.y1 - b.y1).abs()).max((a.x2 - b.x2).abs()).max((a.y2 - b.y2).abs()));
                }
            }
            println!(
                "{}: {}×{} | (a) lệch đầu ra thô max {:.2e} | (b) letterbox lệch TB {:.2e}, max {:.4} ({} / {} giá trị khác) | (c) hộp ≥0,25 Python {} Rust {} khớp {} (lệch toạ độ max {} px); mọi hộp Python {} Rust {} | Rust {:.0} ms",
                m["ten"], anh.rong, anh.cao, lech_ra, tb, max, khac, lech.len(), py25.len(), rs.len(), khop, lech_toa_do, py.len(), hop.len(), ms
            );
            assert!(lech_ra < 1e-2, "đầu ra thô lệch {lech_ra}");
            assert!(tb < 1e-3, "letterbox lệch trung bình {tb}");
            khop_tong += khop;
            rust_tong += rs.len();
            py_tong += py25.len();
        }
        let (recall, precision) = (khop_tong as f32 / py_tong.max(1) as f32, khop_tong as f32 / rust_tong.max(1) as f32);
        println!("tổng: recall {recall:.3}, precision {precision:.3} ({khop_tong} / Python {py_tong} / Rust {rust_tong})");
        assert!(recall >= 0.95 && precision >= 0.95);
    }

    /// Đo thời gian 1 trang A4 300 dpi (ảnh vào của mẫu `hs1_t1_300`): letterbox + mô hình + hậu xử lý.
    #[test]
    #[ignore]
    fn do_thoi_gian_ai_300dpi() {
        let mo_hinh = std::env::var("DEMA4_MO_HINH_XG").expect("DEMA4_MO_HINH_XG");
        let goc = std::path::PathBuf::from(std::env::var("DEMA4_DOI_CHIEU_AI").expect("DEMA4_DOI_CHIEU_AI"));
        let anh = doc_png(&goc.join("hs1_t1_300/vao.png"));
        let mut phien = tao_phien(Path::new(&mo_hinh)).unwrap();
        let mut moc = Vec::new();
        for _ in 0..6 {
            let t = std::time::Instant::now();
            let dv = letterbox(&anh);
            let t_lb = t.elapsed();
            let (ty_le, le_x, le_y) = (dv.ty_le, dv.le_x, dv.le_y);
            let y = chay(&mut phien, dv.tensor).unwrap();
            let _ = hau_xu_ly(&y, ty_le, le_x, le_y, anh.rong, anh.cao);
            moc.push((t_lb.as_secs_f64() * 1000.0, t.elapsed().as_secs_f64() * 1000.0));
        }
        println!("{}×{}: (letterbox ms, tổng ms) = {:?}", anh.rong, anh.cao, moc.iter().map(|(a, b)| (a.round(), b.round())).collect::<Vec<_>>());
    }
}
