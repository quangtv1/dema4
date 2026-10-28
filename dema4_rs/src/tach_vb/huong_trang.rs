//! Đoán hướng trang scan (0/90/180/270°) bằng mô hình PP-LCNet_x1_0_doc_ori (ONNX, nhúng trong exe).
//!
//! Hai tầng: chạy 1 lần, tin cậy ≥ `tin_cay` thì kết luận; thấp hơn thì chạy thêm 4 góc xoay của cùng ảnh
//! (xoay ảnh rồi dịch kết quả về), cộng lại, tin cậy ≥ `TIN_CAY_TANG_2` mới kết luận.
//! Mẫu 2 (200 trang × 4 chiều xoay giả): quyết 780/800 lần, sai 0.

use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;

static MO_HINH: &[u8] = include_bytes!("../../assets/doc_ori_b1.onnx");

const CANH: usize = 224;
const CANH_NGAN: usize = 256;
const TRUNG_BINH: [f32; 3] = [0.485, 0.456, 0.406];
const DO_LECH: [f32; 3] = [0.229, 0.224, 0.225];
/// Lớp đầu ra [0, 90, 180, 270] = góc ảnh đang bị xoay ngược kim → góc cần xoay theo kim để trang đứng.
const GOC_THEO_LOP: [u16; 4] = [0, 270, 180, 90];
pub const TIN_CAY_TANG_2: f32 = 0.5;

/// Phiên chạy mô hình; mỗi luồng giữ 1 phiên riêng.
pub struct MoHinhHuong {
    phien: Session,
}

/// Kết luận hướng: góc cần xoay theo kim (0/90/180/270) + độ tin cậy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Huong {
    pub goc: u16,
    pub tin_cay: f32,
    /// Đã phải chạy thêm tầng 2 (4 góc xoay).
    pub tang_2: bool,
}

impl MoHinhHuong {
    pub fn nap() -> Result<Self, String> {
        let phien = Session::builder()
            .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level3))
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_memory(MO_HINH))
            .map_err(|e| e.to_string())?;
        Ok(Self { phien })
    }

    /// Chạy mô hình trên 1 tensor CHW 3×224×224 đã chuẩn hoá; trả xác suất 4 lớp.
    pub fn du_doan_huong(&mut self, x: &[f32]) -> Result<[f32; 4], String> {
        let dau_vao = Tensor::from_array(([1usize, 3, CANH, CANH], x.to_vec())).map_err(|e| e.to_string())?;
        let ra = self.phien.run(ort::inputs![dau_vao]).map_err(|e| e.to_string())?;
        let (_, p) = ra[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
        Ok([p[0], p[1], p[2], p[3]])
    }

    /// Đoán hướng trang từ ảnh RGB (mỗi điểm 3 byte); `None` khi không đủ tin cậy ở cả 2 tầng.
    pub fn doan_huong(&mut self, rgb: &[u8], rong: usize, cao: usize, tin_cay: f32) -> Result<Option<Huong>, String> {
        let x = chuan_bi_dau_vao(rgb, rong, cao);
        let (goc, tc) = ket_luan(&theo_goc(self.du_doan_huong(&x)?));
        if tc >= tin_cay {
            return Ok(Some(Huong { goc, tin_cay: tc, tang_2: false }));
        }
        let mut cong = [0f32; 4];
        for r in 0..4 {
            let q = theo_goc(self.du_doan_huong(&xoay_nguoc_kim(&x, r))?);
            // Ảnh xoay thêm r×90° ngược kim thì góc cần xoay theo kim tăng r×90° → dịch về
            for (k, v) in cong.iter_mut().enumerate() {
                *v += q[(k + r) % 4];
            }
        }
        let tong: f32 = cong.iter().sum();
        let (goc, tc) = ket_luan(&cong.map(|v| v / tong));
        Ok((tc >= TIN_CAY_TANG_2).then_some(Huong { goc, tin_cay: tc, tang_2: true }))
    }
}

/// Xác suất theo "góc cần xoay theo kim" (chỉ số k = góc / 90).
fn theo_goc(p: [f32; 4]) -> [f32; 4] {
    let mut q = [0f32; 4];
    for (lop, &v) in p.iter().enumerate() {
        q[(GOC_THEO_LOP[lop] / 90) as usize] = v;
    }
    q
}

fn ket_luan(q: &[f32; 4]) -> (u16, f32) {
    let (k, &v) = q.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
    (k as u16 * 90, v)
}

/// Thu cạnh ngắn về 256 (trung bình vùng — khử răng cưa khi thu nhỏ), cắt giữa 224×224, chuẩn hoá ImageNet, CHW.
pub fn chuan_bi_dau_vao(rgb: &[u8], rong: usize, cao: usize) -> Vec<f32> {
    let ty_le = CANH_NGAN as f32 / rong.min(cao) as f32;
    let (rong_moi, cao_moi) = (((rong as f32) * ty_le).round() as usize, ((cao as f32) * ty_le).round() as usize);
    let (x0, y0) = ((rong_moi - CANH) / 2, (cao_moi - CANH) / 2);
    let mut ra = vec![0f32; 3 * CANH * CANH];
    for y in 0..CANH {
        let (sy0, sy1) = vung(y + y0, cao_moi, cao);
        for x in 0..CANH {
            let (sx0, sx1) = vung(x + x0, rong_moi, rong);
            let mut tong = [0u32; 3];
            for sy in sy0..sy1 {
                let dong = &rgb[(sy * rong + sx0) * 3..(sy * rong + sx1) * 3];
                for px in dong.chunks_exact(3) {
                    for c in 0..3 {
                        tong[c] += px[c] as u32;
                    }
                }
            }
            let n = ((sy1 - sy0) * (sx1 - sx0)) as f32;
            for c in 0..3 {
                ra[c * CANH * CANH + y * CANH + x] = (tong[c] as f32 / n / 255.0 - TRUNG_BINH[c]) / DO_LECH[c];
            }
        }
    }
    ra
}

/// Vùng điểm nguồn [a, b) ứng với điểm `i` của ảnh đích `n_dich` (nguồn `n_nguon`).
fn vung(i: usize, n_dich: usize, n_nguon: usize) -> (usize, usize) {
    let a = i * n_nguon / n_dich;
    let b = ((i + 1) * n_nguon / n_dich).max(a + 1).min(n_nguon);
    (a.min(n_nguon - 1), b)
}

/// Xoay tensor CHW vuông `r` lần 90° ngược chiều kim đồng hồ.
fn xoay_nguoc_kim(x: &[f32], r: usize) -> Vec<f32> {
    if r % 4 == 0 {
        return x.to_vec();
    }
    let mut ra = vec![0f32; x.len()];
    for c in 0..3 {
        let kenh = &x[c * CANH * CANH..(c + 1) * CANH * CANH];
        for y in 0..CANH {
            for xx in 0..CANH {
                // điểm (y, xx) của ảnh mới lấy từ ảnh cũ theo r
                let (sy, sx) = match r % 4 {
                    1 => (xx, CANH - 1 - y),
                    2 => (CANH - 1 - y, CANH - 1 - xx),
                    _ => (CANH - 1 - xx, y),
                };
                ra[c * CANH * CANH + y * CANH + xx] = kenh[sy * CANH + sx];
            }
        }
    }
    ra
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tensor mẫu xuất từ Python (PyMuPDF + PIL) cho trang 4547 tr.5 mẫu 2; ONNX Runtime Python cho kết quả này.
    #[test]
    fn mo_hinh_khop_onnx_runtime_python() {
        let du_lieu = include_bytes!("mau_dau_vao_huong.f32");
        let x: Vec<f32> = du_lieu.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let p = MoHinhHuong::nap().unwrap().du_doan_huong(&x).unwrap();
        let chuan = [0.024_885_82, 0.024_827_93, 0.925_689_9, 0.024_596_4];
        for (a, b) in p.iter().zip(chuan) {
            assert!((a - b).abs() < 1e-4, "{p:?}");
        }
    }

    #[test]
    fn xoay_4_lan_ve_nhu_cu() {
        let x: Vec<f32> = (0..3 * CANH * CANH).map(|i| i as f32).collect();
        let mut y = x.clone();
        for _ in 0..4 {
            y = xoay_nguoc_kim(&y, 1);
        }
        assert_eq!(x, y);
        assert_eq!(xoay_nguoc_kim(&xoay_nguoc_kim(&x, 1), 1), xoay_nguoc_kim(&x, 2));
    }

    #[test]
    fn lop_doi_ra_goc_can_xoay() {
        assert_eq!(ket_luan(&theo_goc([0.1, 0.8, 0.05, 0.05])), (270, 0.8));
        assert_eq!(ket_luan(&theo_goc([0.1, 0.05, 0.05, 0.8])), (90, 0.8));
    }

    /// Xoay ảnh RGB r×90° ngược kim (trang cần xoay theo kim r×90° để đứng).
    fn xoay_rgb(rgb: &[u8], w: usize, h: usize, r: usize) -> (Vec<u8>, usize, usize) {
        let mut a = rgb.to_vec();
        let (mut w, mut h) = (w, h);
        for _ in 0..r {
            let mut b = vec![0u8; a.len()];
            for y in 0..w {
                for x in 0..h {
                    let (sy, sx) = (x, w - 1 - y);
                    b[(y * h + x) * 3..(y * h + x) * 3 + 3].copy_from_slice(&a[(sy * w + sx) * 3..(sy * w + sx) * 3 + 3]);
                }
            }
            a = b;
            std::mem::swap(&mut w, &mut h);
        }
        (a, w, h)
    }

    /// Mẫu 1: 40 trang nội dung đầu L1_05_13, xoay giả 4 chiều ⇒ mọi kết luận đều đúng góc; đo thời gian.
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn doan_chieu_trang_mau_1() {
        use mupdf::{Colorspace, Document, Matrix};
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let nhan = std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap();
        let bo: std::collections::HashSet<i32> =
            nhan.lines().filter(|d| d.contains("L1_05_13.pdf;")).filter_map(|d| d.split(';').nth(1)?.trim().parse().ok()).collect();
        let doc = Document::open(mau.join("pdf_goc/L1_05_13.pdf").to_str().unwrap()).unwrap();
        let mut mh = MoHinhHuong::nap().unwrap();
        let (mut quyet, mut sai, mut lan, mut thoi_gian, mut tang2) = (0, 0, 0, 0f64, 0);
        for trang in (1..=doc.page_count().unwrap()).filter(|t| !bo.contains(t)).take(40) {
            let s = 150.0 / 72.0;
            let pix = doc.load_page(trang - 1).unwrap().to_pixmap(&Matrix::new_scale(s, s), &Colorspace::device_rgb(), false, false).unwrap();
            let (w, h) = (pix.width() as usize, pix.height() as usize);
            let rgb: Vec<u8> = if pix.n() == 3 { pix.samples().to_vec() } else { pix.samples().chunks_exact(pix.n() as usize).flat_map(|p| [p[0], p[1], p[2]]).collect() };
            for r in 0..4 {
                let (a, w2, h2) = xoay_rgb(&rgb, w, h, r);
                let t0 = std::time::Instant::now();
                let kq = mh.doan_huong(&a, w2, h2, 0.8).unwrap();
                thoi_gian += t0.elapsed().as_secs_f64();
                lan += 1;
                if let Some(k) = kq {
                    quyet += 1;
                    tang2 += k.tang_2 as usize;
                    if k.goc != (r as u16) * 90 {
                        sai += 1;
                        println!("sai: trang {trang} xoay {} → đoán {} ({:.2})", r * 90, k.goc, k.tin_cay);
                    }
                }
            }
        }
        println!("{lan} lần: quyết {quyet} (tầng 2: {tang2}), sai {sai}, {:.1} ms/lần", thoi_gian * 1e3 / lan as f64);
        assert_eq!(sai, 0);
        assert!(quyet * 100 >= lan * 85, "quyết {quyet}/{lan}");
    }
}
