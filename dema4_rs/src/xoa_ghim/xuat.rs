//! Xuất PDF đã xoá ghim: trang không có vùng áp dụng → chép nguyên trang; trang có vùng → vẽ ở DPI (theo ảnh scan
//! hoặc chọn), xoá ghim, ghi JPEG (hoặc ảnh đen trắng / xám khi "Nén đen trắng") thành trang mới cùng khổ — như XoaGhim.
//!
//! Kết quả vào `<tên nguồn>_xoaghim/`, giữ thư mục con và tên file; file gốc không bị sửa. Ghi file tạm → đẩy xuống
//! đĩa → đổi tên (mất điện không để lại file kết quả dở); nhật ký ghi sau khi đổi tên xong.

use super::kho::{self as kho_xg, CauHinhXuat, NhatKy};
use super::vung::Vung;
use super::xu_ly::{xoa_ghim_trang, TuyChon};
use crate::tach_vb::anh::Anh;
use crate::tach_vb::kho::Id;
use mupdf::pdf::{PdfDocument, PdfWriteOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Hậu tố thư mục kết quả.
pub const HAU_TO: &str = "_xoaghim";
/// DPI khi trang không có ảnh scan (trang chữ / vector) — như XoaGhim.
const DPI_KHONG_ANH: u16 = 150;
/// Chặn DPI vẽ trang (bộ nhớ: A4 600 dpi ~ 100 MB / trang).
pub const DPI_TOI_DA: u16 = 600;

fn loi(e: mupdf::Error) -> String {
    e.to_string()
}

/// DPI ảnh scan của trang: ảnh nhúng rộng nhất / bề rộng trang (điểm), làm tròn như Python; không có ảnh
/// (hoặc ảnh < 60 dpi) → 150; chặn 600.
pub fn dpi_anh_trang(doc: &PdfDocument, so: usize) -> u16 {
    let doc_dpi = || -> Option<u16> {
        let rong_trang = doc.load_page(so as i32).ok()?.bounds().ok()?;
        let rong_trang = (rong_trang.x1 - rong_trang.x0) as f64;
        let trang = doc.find_page(so as i32).ok()?;
        let xobject = trang.get_dict_inheritable("Resources").ok()??.get_dict("XObject").ok()??;
        let mut tot = 0u16;
        for cap in xobject.dict_iter().ok()? {
            let Ok((_, anh)) = cap else { continue };
            if anh.get_dict("Subtype").ok().flatten().and_then(|v| v.as_name().ok()).as_deref() != Some(b"Image".as_slice()) {
                continue;
            }
            let rong = anh.get_dict("Width").ok().flatten().and_then(|v| v.as_int().ok()).unwrap_or(0);
            if rong > 0 && rong_trang > 0.0 {
                tot = tot.max(((rong as f64) * 72.0 / rong_trang).round_ties_even().min(u16::MAX as f64) as u16);
            }
        }
        (tot >= 60).then_some(tot.min(DPI_TOI_DA))
    };
    doc_dpi().unwrap_or(DPI_KHONG_ANH)
}

/// Ảnh xám cùng công thức với bước xoá ghim (OpenCV).
fn xam_opencv(a: &Anh) -> Anh {
    let du_lieu = a.du_lieu.chunks_exact(3).map(|p| super::xu_ly::xam(p[0], p[1], p[2]) as u8).collect();
    Anh { rong: a.rong, cao: a.cao, kenh: 1, du_lieu }
}

/// Xám (|R−G|, |G−B| ≤ 5 mọi điểm) — như `_is_grayscale_image`.
fn la_anh_xam(a: &Anh) -> bool {
    a.du_lieu.chunks_exact(3).all(|p| p[0].abs_diff(p[1]) <= 5 && p[1].abs_diff(p[2]) <= 5)
}

/// Đen trắng: ≥ 85% điểm rất tối (< 50) hoặc rất sáng (> 205) — như `_is_bw_image`.
fn la_anh_den_trang(xam: &Anh) -> bool {
    let n = xam.du_lieu.iter().filter(|&&v| !(50..=205).contains(&v)).count();
    n as f64 / xam.du_lieu.len().max(1) as f64 >= 0.85
}

/// Ảnh trang đã xử lý → ảnh PDF theo cách ghi đã chọn.
fn anh_pdf(anh: &Anh, xuat: &CauHinhXuat) -> Result<mupdf::Image, String> {
    let jpeg = |du_lieu: &[u8], mau: jpeg_encoder::ColorType, chat_luong: u8| -> Result<mupdf::Image, String> {
        let mut ra = Vec::new();
        let mut bm = jpeg_encoder::Encoder::new(&mut ra, chat_luong.clamp(1, 100));
        // Màu lấy mẫu 4:2:0 ở mọi chất lượng như OpenCV (bộ mã hoá mặc định 4:4:4 khi ≥ 90 → file lớn hơn ~18%)
        bm.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
        bm.encode(du_lieu, anh.rong as u16, anh.cao as u16, mau).map_err(|e| e.to_string())?;
        mupdf::Image::from_bytes(&ra).map_err(loi)
    };
    if anh.rong > u16::MAX as usize || anh.cao > u16::MAX as usize {
        return Err("trang quá lớn".into());
    }
    if xuat.nen_den_trang {
        let xam = xam_opencv(anh);
        if la_anh_den_trang(&xam) {
            // Ngưỡng 127 → ảnh 2 mức, ghi nén không mất (PDF nén Flate — gọn với ảnh 2 mức)
            let mut pix = mupdf::Pixmap::new_with_w_h(&mupdf::Colorspace::device_gray(), xam.rong as i32, xam.cao as i32, false).map_err(loi)?;
            for (o, v) in pix.samples_mut().iter_mut().zip(&xam.du_lieu) {
                *o = if *v > 127 { 255 } else { 0 };
            }
            return mupdf::Image::from_pixmap(&pix).map_err(loi);
        }
        if la_anh_xam(anh) {
            return jpeg(&xam.du_lieu, jpeg_encoder::ColorType::Luma, xuat.chat_luong.min(90));
        }
    }
    jpeg(&anh.du_lieu, jpeg_encoder::ColorType::Rgb, xuat.chat_luong)
}

/// Thêm trang mới khổ `rong` × `cao` điểm, phủ kín bằng `anh`, ở vị trí `thu_tu`.
fn them_trang_anh(dst: &mut PdfDocument, thu_tu: usize, rong: f32, cao: f32, anh: &mupdf::Image) -> Result<(), String> {
    let anh_obj = dst.add_image(anh).map_err(loi)?;
    let mut xobject = dst.new_dict().map_err(loi)?;
    xobject.dict_put("Im0", anh_obj).map_err(loi)?;
    let mut tai_nguyen = dst.new_dict().map_err(loi)?;
    tai_nguyen.dict_put("XObject", xobject).map_err(loi)?;
    let lenh = format!("q {rong} 0 0 {cao} 0 0 cm /Im0 Do Q");
    let noi_dung = dst.add_stream(&mupdf::Buffer::from_bytes(lenh.as_bytes()).map_err(loi)?, None, false).map_err(loi)?;
    let mut khung = dst.new_array().map_err(loi)?;
    for v in [0.0, 0.0, rong, cao] {
        khung.array_push(dst.new_real(v).map_err(loi)?).map_err(loi)?;
    }
    let mut trang = dst.new_dict().map_err(loi)?;
    trang.dict_put("Type", dst.new_name("Page").map_err(loi)?).map_err(loi)?;
    trang.dict_put("MediaBox", khung).map_err(loi)?;
    trang.dict_put("Resources", tai_nguyen).map_err(loi)?;
    trang.dict_put("Contents", noi_dung).map_err(loi)?;
    let obj = dst.add_object(&trang).map_err(loi)?;
    dst.insert_page(thu_tu as i32, &obj).map_err(loi)
}

/// Kết quả xoá ghim 1 file.
#[derive(Debug, Clone, PartialEq)]
pub struct KetQuaFile {
    pub so_trang: u32,
    /// Trang đã vẽ lại + xoá ghim (từ 0).
    pub trang_xu_ly: Vec<u32>,
    /// DPI lớn nhất đã vẽ (0 = không trang nào).
    pub dpi: u16,
}

/// Xoá ghim `nguon` → nội dung PDF kết quả (chưa ghi file). `vung`: vùng chung rồi vùng riêng của file, đúng thứ tự.
pub fn xoa_ghim_pdf(nguon: &Path, vung: &[Vung], tc: &TuyChon, xuat: &CauHinhXuat, dung: &AtomicBool) -> Result<(Vec<u8>, KetQuaFile), String> {
    let src = PdfDocument::open(nguon.to_str().ok_or("đường dẫn không hợp lệ")?).map_err(loi)?;
    if src.needs_password().map_err(loi)? {
        return Err("PDF có mật khẩu".into());
    }
    let so_trang = src.page_count().map_err(loi)? as usize;
    let mut dst = PdfDocument::new();
    let mut ghep = dst.new_graft_map().map_err(loi)?;
    let mut kq = KetQuaFile { so_trang: so_trang as u32, trang_xu_ly: Vec::new(), dpi: 0 };
    for so in 0..so_trang {
        if dung.load(Ordering::Relaxed) {
            return Err("đã dừng".into());
        }
        if !vung.iter().any(|v| v.ap_trang.ap_dung(so as u32)) {
            crate::tach_vb::cat_pdf::ghep_trang(&src, so, &mut dst, &mut ghep, so, 0)?;
            continue;
        }
        let dpi = if xuat.dpi == 0 { dpi_anh_trang(&src, so) } else { xuat.dpi.min(DPI_TOI_DA) };
        let trang = src.load_page(so as i32).map_err(loi)?;
        let khung = trang.bounds().map_err(loi)?;
        let mut anh = crate::tach_vb::phan_tich::render_rgb(&trang, dpi as f32)?;
        xoa_ghim_trang(&mut anh, vung, so as u32, dpi as f64, tc, &[]);
        them_trang_anh(&mut dst, so, khung.x1 - khung.x0, khung.y1 - khung.y0, &anh_pdf(&anh, xuat)?)?;
        kq.trang_xu_ly.push(so as u32);
        kq.dpi = kq.dpi.max(dpi);
    }
    let mut tuy_chon = PdfWriteOptions::default();
    tuy_chon.set_compress(true);
    let mut ra = Vec::new();
    dst.write_to_with_options(&mut ra, tuy_chon).map_err(loi)?;
    Ok((ra, kq))
}

/// Ghi `noi_dung` vào `dich` chắc chắn: file tạm cạnh đích → đẩy xuống đĩa → đổi tên → đẩy thư mục (khi được).
pub fn ghi_chac(dich: &Path, noi_dung: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let cha = dich.parent().ok_or_else(|| std::io::Error::other("đường dẫn kết quả không có thư mục"))?;
    std::fs::create_dir_all(cha)?;
    let mut ten = dich.as_os_str().to_owned();
    ten.push(".xoaghim.tmp");
    let tam = PathBuf::from(ten);
    let kq = (|| {
        let mut f = std::fs::File::create(&tam)?;
        f.write_all(noi_dung)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tam, dich)
    })();
    if kq.is_err() {
        let _ = std::fs::remove_file(&tam);
    }
    kq?;
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(cha) {
        let _ = d.sync_all();
    }
    Ok(())
}

/// Các PDF trong thư mục nguồn (theo thứ tự cây), bỏ thư mục kết quả Xóa ghim (`*_xoaghim`) nằm bên trong,
/// kèm đường dẫn kết quả tương ứng trong `ra` (giữ thư mục con, giữ tên file).
pub fn ds_viec(nguon: &Path, ra: &Path, dung: &AtomicBool) -> Vec<(PathBuf, PathBuf)> {
    let mut ds = Vec::new();
    crate::tach_vb::phien::duyet_pdf_theo_cay(nguon, dung, |lo| ds.extend(lo));
    ds.into_iter()
        .filter_map(|p| {
            let tuong_doi = p.strip_prefix(nguon).ok()?.to_path_buf();
            let trong_ket_qua = tuong_doi.parent().is_some_and(|c| c.components().any(|x| x.as_os_str().to_string_lossy().to_lowercase().ends_with(HAU_TO)));
            (!trong_ket_qua).then(|| (p, ra.join(tuong_doi)))
        })
        .collect()
}

/// Trạng thái kết quả của 1 PDF nguồn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThai {
    ChuaXoa,
    /// Đã xoá; `dung_cau_hinh` = kết quả theo đúng vùng + tuỳ chọn hiện tại.
    DaXoa { dung_cau_hinh: bool },
    /// Nhật ký có nhưng file kết quả không còn / đã bị thay (xoá, di chuyển thư mục ra, sửa file).
    KetQuaMat,
}

/// Trạng thái: chỉ "đã xoá" khi file ở `dich` (tính lại từ nguồn hiện tại) có id đúng id trong nhật ký.
/// `id_dich`: id file đang có ở `dich` (`None` = không có / không đọc được).
pub fn trang_thai(nk: Option<&NhatKy>, id_dich: Option<Id>, ma_cau_hinh_hien_tai: u64) -> TrangThai {
    match nk {
        None => TrangThai::ChuaXoa,
        Some(nk) if id_dich == Some(nk.ket_qua) => TrangThai::DaXoa { dung_cau_hinh: nk.ma_cau_hinh == ma_cau_hinh_hien_tai },
        Some(_) => TrangThai::KetQuaMat,
    }
}

/// Đường dẫn chưa chắc tồn tại → id nội dung (đọc 64 KB đầu + cuối) mà không ghi nhớ vào bảng đường dẫn.
fn id_file(p: &Path) -> Option<Id> {
    let n = std::fs::metadata(p).ok()?.len();
    Some((crate::tach_vb::cache_trang::van_tay_file(p, n).ok()?, n))
}

/// Lỗi chặn ghi: file kết quả đã có là kết quả xoá ghim của **nguồn khác** (trùng tên ở nơi khác) — không ghi đè.
pub const LOI_NGUON_KHAC: &str = "file kết quả đã có là kết quả của một PDF khác (trùng tên) — không ghi đè";

/// Xoá ghim 1 file nguồn (`id_nguon` = id nội dung của nó) vào `dich`, rồi ghi nhật ký vào `kho`.
/// Không ghi đè file kết quả của nguồn khác (`nguon_theo_ket_qua`: dựng 1 lần / lượt); chạy lại cùng nguồn thì ghi đè.
#[allow(clippy::too_many_arguments)]
pub fn xoa_ghim_file(
    kho: &std::sync::Mutex<crate::tach_vb::kho::Kho>,
    nguon_theo_ket_qua: &std::collections::HashMap<Id, Id>,
    nguon: &Path,
    id_nguon: Id,
    dich: &Path,
    vung: &[Vung],
    tc: &TuyChon,
    xuat: &CauHinhXuat,
    dung: &AtomicBool,
) -> Result<KetQuaFile, String> {
    let khoa = || kho.lock().unwrap_or_else(|e| e.into_inner());
    if id_file(dich).and_then(|id_cu| nguon_theo_ket_qua.get(&id_cu)).is_some_and(|n| *n != id_nguon) {
        return Err(LOI_NGUON_KHAC.into());
    }
    let (noi_dung, kq) = xoa_ghim_pdf(nguon, vung, tc, xuat, dung)?;
    ghi_chac(dich, &noi_dung).map_err(|e| format!("không ghi được {}: {e}", dich.display()))?;
    let ket_qua = id_file(dich).ok_or_else(|| format!("không đọc lại được {}", dich.display()))?;
    let nk = NhatKy {
        luc: crate::tach_vb::moc_sua::bay_gio_ms(),
        ket_qua,
        ma_cau_hinh: kho_xg::ma_cau_hinh(vung, tc, xuat),
        so_trang: kq.so_trang,
        dpi: kq.dpi,
        ai: false,
        trang: kho_xg::thanh_dai(&kq.trang_xu_ly),
    };
    let mut k = khoa();
    kho_xg::ghi_nhat_ky(&mut k, id_nguon, &nk);
    k.day().map_err(|e| format!("đã ghi {} nhưng chưa lưu được nhật ký: {e}", dich.display()))?;
    Ok(kq)
}

/// Số luồng xoá ghim song song theo file: nửa số nhân (để chừa máy cho giao diện / việc khác), 1–8.
/// Mỗi luồng giữ 1 ảnh trang (A4 300 dpi ~ 26 MB, 600 dpi ~ 100 MB) và 1 PDF kết quả trong bộ nhớ.
pub fn so_luong() -> usize {
    (std::thread::available_parallelism().map_or(2, |n| n.get()) / 2).clamp(1, 8)
}

/// Pool luồng riêng của Xóa ghim (không tranh pool chung của Tách / Đếm).
pub fn pool() -> &'static rayon::ThreadPool {
    static POOL: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
    POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(so_luong()).thread_name(|i| format!("xoa-ghim-{i}")).build().expect("tạo luồng Xóa ghim"))
}

#[cfg(test)]
mod tests {
    use super::super::vung::{ApTrang, LoaiVung, ViTri};
    use super::*;
    use crate::tach_vb::kho::Kho;

    fn tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_xuat_xg_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// PDF `so_trang` trang, mỗi trang 1 ảnh JPEG xám nhạt 200×283 điểm ảnh phủ khổ A4 nhỏ (72 dpi giả lập
    /// 100 dpi), góc trên trái có vết ghim xám đậm.
    fn tao_pdf(p: &Path, so_trang: usize) {
        let mut doc = PdfDocument::new();
        let (w, h) = (200usize, 283usize);
        for i in 0..so_trang {
            let mut a = Anh { rong: w, cao: h, kenh: 3, du_lieu: [235u8, 233, 228].repeat(w * h) };
            for y in 8..20 {
                for x in 8..30 {
                    let k = (y * w + x) * 3;
                    a.du_lieu[k..k + 3].copy_from_slice(&[140, 140, 140]);
                }
            }
            let anh = anh_pdf(&a, &CauHinhXuat { chat_luong: 95, ..Default::default() }).unwrap();
            them_trang_anh(&mut doc, i, 144.0, 203.76, &anh).unwrap();
        }
        let mut b = Vec::new();
        doc.write_to(&mut b).unwrap();
        std::fs::write(p, b).unwrap();
    }

    fn goc_tren_trai() -> Vung {
        Vung { vi_tri: ViTri::GocTrenTrai, loai: LoaiVung::Xoa, ap_trang: ApTrang::Le, x: 0.0, y: 0.0, rong: 0.2, cao: 0.2, rong_px: 50, cao_px: 50, nguong: 5 }
    }

    #[test]
    fn xoa_trang_co_vung_chep_nguyen_trang_khong_vung_dpi_theo_anh_scan() {
        let d = tam("pdf");
        let nguon = d.join("a.pdf");
        tao_pdf(&nguon, 3);
        let src = PdfDocument::open(nguon.to_str().unwrap()).unwrap();
        assert_eq!(dpi_anh_trang(&src, 0), 100, "200 điểm ảnh / 144 điểm × 72");
        let (b, kq) = xoa_ghim_pdf(&nguon, &[goc_tren_trai()], &TuyChon::default(), &CauHinhXuat::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(kq, KetQuaFile { so_trang: 3, trang_xu_ly: vec![0, 2], dpi: 100 });
        let ra = d.join("ra.pdf");
        std::fs::write(&ra, &b).unwrap();
        // Trang 2 (không vùng) giữ nguyên luồng ảnh gốc; trang 1 đổi
        assert_eq!(crate::tach_vb::cat_pdf::hash_anh_trang(&ra, 2), crate::tach_vb::cat_pdf::hash_anh_trang(&nguon, 2));
        assert_ne!(crate::tach_vb::cat_pdf::hash_anh_trang(&ra, 1), crate::tach_vb::cat_pdf::hash_anh_trang(&nguon, 1));
        // Vết ghim trang 1 đã mất: vẽ lại ở 100 dpi, điểm (15, 12) gần màu nền
        let doc = PdfDocument::open(ra.to_str().unwrap()).unwrap();
        let khung = doc.load_page(0).unwrap().bounds().unwrap();
        assert!((khung.x1 - 144.0).abs() < 0.01 && (khung.y1 - 203.76).abs() < 0.01, "giữ khổ trang");
        let anh = crate::tach_vb::phan_tich::render_rgb(&doc.load_page(0).unwrap(), 100.0).unwrap();
        let k = (12 * anh.rong + 15) * 3;
        assert!(anh.du_lieu[k] > 200, "vết ghim còn: {:?}", &anh.du_lieu[k..k + 3]);
        let dung = AtomicBool::new(true);
        assert!(xoa_ghim_pdf(&nguon, &[goc_tren_trai()], &TuyChon::default(), &CauHinhXuat::default(), &dung).is_err(), "Dừng");
    }

    #[test]
    fn nen_den_trang_ghi_anh_2_muc_va_jpeg_xam() {
        let den_trang = Anh { rong: 4, cao: 2, kenh: 3, du_lieu: [[0u8; 3], [255; 3], [10; 3], [250; 3]].concat().repeat(2) };
        let xam = Anh { rong: 4, cao: 2, kenh: 3, du_lieu: [[100u8; 3], [120; 3], [130; 3], [140; 3]].concat().repeat(2) };
        let mau = Anh { rong: 4, cao: 2, kenh: 3, du_lieu: [[200u8, 30, 30]; 8].concat() };
        let nen = CauHinhXuat { nen_den_trang: true, ..Default::default() };
        assert!(la_anh_den_trang(&xam_opencv(&den_trang)) && !la_anh_den_trang(&xam_opencv(&xam)));
        assert!(la_anh_xam(&xam) && !la_anh_xam(&mau));
        for a in [&den_trang, &xam, &mau] {
            assert!(anh_pdf(a, &nen).is_ok());
        }
    }

    #[test]
    fn ds_viec_giu_thu_muc_con_bo_thu_muc_ket_qua_ben_trong() {
        let d = tam("ds");
        let nguon = d.join("Nguon");
        for f in ["a.pdf", "con/b.pdf", "cu_xoaghim/c.pdf", "con/X_XOAGHIM/d.pdf"] {
            std::fs::create_dir_all(nguon.join(f).parent().unwrap()).unwrap();
            std::fs::write(nguon.join(f), b"%PDF").unwrap();
        }
        let ra = d.join("Nguon_xoaghim");
        let ds = ds_viec(&nguon, &ra, &AtomicBool::new(false));
        assert_eq!(ds, vec![(nguon.join("a.pdf"), ra.join("a.pdf")), (nguon.join("con/b.pdf"), ra.join("con/b.pdf"))]);
    }

    fn bang(kho: &std::sync::Mutex<Kho>) -> std::collections::HashMap<Id, Id> {
        kho_xg::nguon_theo_ket_qua(&mut kho.lock().unwrap())
    }

    #[test]
    fn ghi_ket_qua_nhat_ky_trang_thai_va_khong_de_ket_qua_nguon_khac() {
        let d = tam("file");
        let kho = std::sync::Mutex::new(Kho::mo(&d.join("kho_xoa_ghim.bin")).unwrap());
        let (a, b) = (d.join("nguon/a.pdf"), d.join("nguon2/a.pdf"));
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(b.parent().unwrap()).unwrap();
        tao_pdf(&a, 1);
        tao_pdf(&b, 2);
        let (id_a, id_b) = (id_file(&a).unwrap(), id_file(&b).unwrap());
        let dich = d.join("ra/a.pdf");
        let (vung, tc, xuat, dung) = ([goc_tren_trai()], TuyChon::default(), CauHinhXuat::default(), AtomicBool::new(false));
        let kq = xoa_ghim_file(&kho, &bang(&kho), &a, id_a, &dich, &vung, &tc, &xuat, &dung).unwrap();
        assert_eq!(kq.trang_xu_ly, vec![0]);
        assert!(std::fs::read_dir(dich.parent().unwrap()).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")), "không sót file tạm");
        let nk = kho_xg::doc_nhat_ky(&mut kho.lock().unwrap(), id_a).unwrap();
        let ma = kho_xg::ma_cau_hinh(&vung, &tc, &xuat);
        assert_eq!(trang_thai(Some(&nk), id_file(&dich), ma), TrangThai::DaXoa { dung_cau_hinh: true });
        assert_eq!(trang_thai(Some(&nk), id_file(&dich), ma ^ 1), TrangThai::DaXoa { dung_cau_hinh: false });
        assert_eq!(trang_thai(None, id_file(&dich), ma), TrangThai::ChuaXoa);
        // Nguồn khác cùng tên, cùng thư mục ra → không ghi đè
        let truoc = std::fs::read(&dich).unwrap();
        assert_eq!(xoa_ghim_file(&kho, &bang(&kho), &b, id_b, &dich, &vung, &tc, &xuat, &dung).unwrap_err(), LOI_NGUON_KHAC);
        assert_eq!(std::fs::read(&dich).unwrap(), truoc);
        // Chạy lại cùng nguồn → ghi đè được
        assert!(xoa_ghim_file(&kho, &bang(&kho), &a, id_a, &dich, &vung, &tc, &xuat, &dung).is_ok());
        // Xoá kết quả → "kết quả mất"
        std::fs::remove_file(&dich).unwrap();
        assert_eq!(trang_thai(Some(&nk), id_file(&dich), ma), TrangThai::KetQuaMat);
        // File hỏng → lỗi của file đó
        let hong = d.join("nguon/hong.pdf");
        std::fs::write(&hong, b"khong phai pdf").unwrap();
        assert!(xoa_ghim_file(&kho, &bang(&kho), &hong, (1, 1), &d.join("ra/hong.pdf"), &vung, &tc, &xuat, &dung).is_err());
        assert!(!d.join("ra/hong.pdf").exists());
    }
}
