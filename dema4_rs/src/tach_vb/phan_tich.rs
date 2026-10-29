//! Phân tích 1 trang: render 1 lần → mã vạch, chỉ số trắng, phân bố hướng, ảnh nhỏ (kết quả thô, lưu cache);
//! và gán nhãn tự động từ kết quả thô + Cấu hình (hàm thuần — đổi ngưỡng / mã index / độ tin cậy không render lại).

use super::anh::Anh;
use super::chi_so_trang::{phan_loai, tinh_chi_so, ChiSoTrang, LoaiTrang};
use super::doc_ma::{doc_ma_code128, MaDoc};
use super::huong_trang::{ket_luan, MoHinhHuong, PhanBoHuong};
use super::nhan::Nhan;

/// Phần Cấu hình ảnh hưởng tới phân tích tự động.
#[derive(Debug, Clone, PartialEq)]
pub struct CauHinhPhanTich {
    pub quet_trang: bool,
    pub quet_ma: bool,
    pub quet_xoay: bool,
    /// Mã index (Code 128); trống = mọi mã Code 128 hợp lệ là index.
    pub ma_index: String,
    /// Ngưỡng Trắng / Nghi trắng (tỷ lệ, 0.01 = 1 %).
    pub nguong_trang: f32,
    pub nguong_nghi: f32,
    pub tin_cay_xoay: f32,
}

impl Default for CauHinhPhanTich {
    fn default() -> Self {
        Self { quet_trang: true, quet_ma: true, quet_xoay: true, ma_index: "NAMSAO-TACH".into(), nguong_trang: 0.01, nguong_nghi: 0.03, tin_cay_xoay: 0.8 }
    }
}

/// Mã đọc được lệch tối đa bấy nhiêu ký tự so với mã index trong Cấu hình vẫn tính là index.
pub const LECH_MA_TOI_DA: usize = 3;
/// Cạnh dài ảnh nhỏ cho lưới.
pub const CANH_ANH_NHO: usize = 165;

/// Kết quả thô của 1 trang (lưu cache; nhãn tự động tính lại từ đây).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KetQuaTrang {
    /// Đã quét mã (để biết khi bật thêm loại quét thì chỉ chạy phần còn thiếu).
    pub da_quet_ma: bool,
    pub ma: Vec<MaDoc>,
    pub chi_so: Option<ChiSoTrang>,
    pub huong: Option<PhanBoHuong>,
}

/// Nhãn tự động của trang.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NhanTuDong {
    pub nhan: Nhan,
    pub nghi_trang: bool,
    /// Góc đề xuất (90/180/270) khi mô hình cho là trang scan bị xoay.
    pub nghi_xoay: Option<u16>,
}

/// Khoảng cách sửa chuỗi (Levenshtein) theo ký tự.
pub fn lech_ky_tu(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut dong: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut moi = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            moi.push((dong[j + 1] + 1).min(moi[j] + 1).min(dong[j] + (ca != cb) as usize));
        }
        dong = moi;
    }
    dong[b.len()]
}

/// Trang có mã index theo Cấu hình: mã trống → mọi mã hợp lệ; có mã → mã hợp lệ hoặc lỗi checksum lệch ≤ 3 ký tự.
pub fn la_index(ma: &[MaDoc], ma_index: &str) -> bool {
    let ma_index = ma_index.trim();
    if ma_index.is_empty() {
        return ma.iter().any(|m| !m.loi_checksum);
    }
    ma.iter().any(|m| lech_ky_tu(&m.gia_tri, ma_index) <= LECH_MA_TOI_DA)
}

/// Gán nhãn tự động: index trước, rồi trắng / nghi trắng; chỉ trang nội dung mới xét Nghi xoay.
pub fn gan_nhan(kq: &KetQuaTrang, ch: &CauHinhPhanTich) -> NhanTuDong {
    if ch.quet_ma && la_index(&kq.ma, &ch.ma_index) {
        return NhanTuDong { nhan: Nhan::Index, nghi_trang: false, nghi_xoay: None };
    }
    let loai = match (ch.quet_trang, kq.chi_so) {
        (true, Some(cs)) => phan_loai(cs, ch.nguong_trang, ch.nguong_nghi),
        _ => LoaiTrang::NoiDung,
    };
    match loai {
        LoaiTrang::Trang => NhanTuDong { nhan: Nhan::Trang, nghi_trang: false, nghi_xoay: None },
        LoaiTrang::NghiTrang => NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: true, nghi_xoay: None },
        LoaiTrang::NoiDung => {
            let nghi_xoay = if ch.quet_xoay { kq.huong.as_ref().and_then(|pb| ket_luan(pb, ch.tin_cay_xoay)).map(|h| h.goc).filter(|&g| g != 0) } else { None };
            NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: false, nghi_xoay }
        }
    }
}

/// Render trang thành ảnh RGB ở `dpi`.
pub fn render_rgb(trang: &mupdf::Page, dpi: f32) -> Result<Anh, String> {
    let s = dpi / 72.0;
    let pix = trang.to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_rgb(), false, false).map_err(|e| e.to_string())?;
    let (rong, cao, n) = (pix.width() as usize, pix.height() as usize, pix.n() as usize);
    let du_lieu = if n == 3 { pix.samples().to_vec() } else { pix.samples().chunks_exact(n).flat_map(|p| [p[0], p[1], p[2]]).collect() };
    Ok(Anh { rong, cao, kenh: 3, du_lieu })
}

/// Ảnh nhỏ xám cạnh dài `CANH_ANH_NHO`.
pub fn anh_nho(rgb: &Anh) -> Anh {
    let ty_le = CANH_ANH_NHO as f32 / rgb.rong.max(rgb.cao) as f32;
    let xam = rgb.sang_xam();
    if ty_le >= 1.0 { xam } else { xam.thu_nho(ty_le) }
}

/// Phân tích 1 trang. Đọc mã + hướng cần 150 dpi; chỉ dò trắng thì render 100 dpi (nhanh ~3 lần).
/// Hướng chỉ chạy cho trang nội dung (theo Cấu hình lúc phân tích); `mo_hinh = None` → bỏ hướng.
pub fn phan_tich_trang(trang: &mupdf::Page, ch: &CauHinhPhanTich, mo_hinh: Option<&mut MoHinhHuong>) -> Result<(KetQuaTrang, Option<Anh>), String> {
    let mut kq = KetQuaTrang::default();
    if !ch.quet_trang && !ch.quet_ma && !ch.quet_xoay {
        return Ok((kq, None));
    }
    let dpi = if ch.quet_ma || ch.quet_xoay { 150.0 } else { 100.0 };
    let rgb = render_rgb(trang, dpi)?;
    if ch.quet_ma {
        let xam = rgb.sang_xam();
        kq.ma = doc_ma_code128(&xam.du_lieu, xam.rong as u32, xam.cao as u32);
        kq.da_quet_ma = true;
    }
    if ch.quet_trang {
        let r100 = if dpi > 100.0 { rgb.thu_nho(100.0 / dpi) } else { rgb.clone() };
        kq.chi_so = Some(tinh_chi_so(&r100));
    }
    if let (true, Some(mh)) = (ch.quet_xoay, mo_hinh) {
        let tam = gan_nhan(&kq, &CauHinhPhanTich { quet_xoay: false, ..ch.clone() });
        if tam.nhan == Nhan::NoiDung && !tam.nghi_trang {
            kq.huong = Some(mh.phan_bo(&rgb.du_lieu, rgb.rong, rgb.cao, ch.tin_cay_xoay)?);
        }
    }
    Ok((kq, Some(anh_nho(&rgb))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ma(g: &str, loi: bool) -> MaDoc {
        MaDoc { gia_tri: g.into(), loi_checksum: loi }
    }

    #[test]
    fn lech_ky_tu_dung() {
        assert_eq!(lech_ky_tu("NAMSAO-TACH", "NAMSAO-TACH"), 0);
        assert_eq!(lech_ky_tu("NAM5AO-T,CH", "NAMSAO-TACH"), 2);
        assert!(lech_ky_tu("Van-ban-khac", "NAMSAO-TACH") > LECH_MA_TOI_DA);
        assert_eq!(lech_ky_tu("abc", ""), 3);
    }

    #[test]
    fn index_theo_ma_cau_hinh() {
        assert!(la_index(&[ma("NAMSAO-TACH", false)], "NAMSAO-TACH"));
        assert!(la_index(&[ma("NAM5AO-T,CH", false)], "NAMSAO-TACH"));
        assert!(la_index(&[ma("NA_SAO-TACH", true)], "NAMSAO-TACH"), "mã lỗi checksum khớp gần vẫn là index");
        assert!(!la_index(&[ma("Van-ban-khac", false)], "NAMSAO-TACH"));
        assert!(la_index(&[ma("Van-ban-khac", false)], ""), "ô mã trống: mọi mã hợp lệ");
        assert!(!la_index(&[ma("fAMgAO-TACH", true)], ""), "ô mã trống: mã lỗi checksum không tính");
        assert!(!la_index(&[], "NAMSAO-TACH"));
    }

    #[test]
    fn gan_nhan_theo_thu_tu_uu_tien() {
        let ch = CauHinhPhanTich::default();
        let trang_co_ma = KetQuaTrang { da_quet_ma: true, ma: vec![ma("NAMSAO-TACH", false)], chi_so: Some(ChiSoTrang { dam: 0.0, mau: 0.0 }), huong: None };
        assert_eq!(gan_nhan(&trang_co_ma, &ch).nhan, Nhan::Index, "trang vừa có mã vừa trắng → index");
        let trang = KetQuaTrang { chi_so: Some(ChiSoTrang { dam: 0.002, mau: 0.0 }), ..Default::default() };
        assert_eq!(gan_nhan(&trang, &ch).nhan, Nhan::Trang);
        let nghi = KetQuaTrang { chi_so: Some(ChiSoTrang { dam: 0.02, mau: 0.0 }), ..Default::default() };
        assert!(gan_nhan(&nghi, &ch).nghi_trang);
        let xoay = KetQuaTrang { chi_so: Some(ChiSoTrang { dam: 0.2, mau: 0.0 }), huong: Some(PhanBoHuong { tang_1: [0.05, 0.05, 0.85, 0.05], tang_2: None }), ..Default::default() };
        assert_eq!(gan_nhan(&xoay, &ch).nghi_xoay, Some(180));
        assert_eq!(gan_nhan(&xoay, &CauHinhPhanTich { tin_cay_xoay: 0.9, ..ch.clone() }).nghi_xoay, None, "tăng độ tin cậy → bỏ cờ");
        assert_eq!(gan_nhan(&xoay, &CauHinhPhanTich { quet_xoay: false, ..ch.clone() }).nghi_xoay, None);
        // Tắt quét mã → không có index dù có mã trong kết quả cũ
        assert_eq!(gan_nhan(&trang_co_ma, &CauHinhPhanTich { quet_ma: false, ..ch.clone() }).nhan, Nhan::Trang);
        // Đổi ngưỡng → nhãn đổi, không cần render
        assert_eq!(gan_nhan(&nghi, &CauHinhPhanTich { nguong_trang: 0.03, ..ch }).nhan, Nhan::Trang);
    }
}

#[cfg(test)]
mod do_thoi_gian {
    use super::*;
    use std::time::Instant;

    /// Đo từng công đoạn trên 40 trang đầu 1 hồ sơ mẫu 2 (1 luồng).
    #[test]
    #[ignore = "đo tay, cần DEMA4_MAU2"]
    fn do_tung_cong_doan() {
        let p = std::path::PathBuf::from(std::env::var_os("DEMA4_MAU2").unwrap()).join("HOP 461/4762/4762.pdf");
        let doc = mupdf::Document::open(p.to_str().unwrap()).unwrap();
        let mut mh = MoHinhHuong::nap().unwrap();
        let mut t = [0f64; 8];
        let ten = ["render 150 RGB", "sang xám", "đọc mã", "thu 100 dpi", "chỉ số trắng", "chuẩn bị tensor", "mô hình (2 tầng)", "ảnh nhỏ"];
        let n = 40;
        for i in 0..n {
            let trang = doc.load_page(i).unwrap();
            let mut moc = Instant::now();
            let mut bam = |k: usize, moc: &mut Instant| { t[k] += moc.elapsed().as_secs_f64(); *moc = Instant::now(); };
            let rgb = render_rgb(&trang, 150.0).unwrap(); bam(0, &mut moc);
            let xam = rgb.sang_xam(); bam(1, &mut moc);
            let _ = doc_ma_code128(&xam.du_lieu, xam.rong as u32, xam.cao as u32); bam(2, &mut moc);
            let r100 = rgb.thu_nho(100.0 / 150.0); bam(3, &mut moc);
            let _ = tinh_chi_so(&r100); bam(4, &mut moc);
            let _ = crate::tach_vb::huong_trang::chuan_bi_dau_vao(&rgb.du_lieu, rgb.rong, rgb.cao); bam(5, &mut moc);
            let _ = mh.phan_bo(&rgb.du_lieu, rgb.rong, rgb.cao, 0.8).unwrap(); bam(6, &mut moc);
            let _ = anh_nho(&rgb); bam(7, &mut moc);
        }
        for (k, ten) in ten.iter().enumerate() {
            println!("{ten:18} {:6.1} ms/trang", t[k] * 1e3 / n as f64);
        }
        println!("{:18} {:6.1} ms/trang", "tổng", t.iter().sum::<f64>() * 1e3 / n as f64);
    }
}
