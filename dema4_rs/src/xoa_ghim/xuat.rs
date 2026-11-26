//! Xuất PDF đã xoá ghim: trang không có vùng áp dụng → chép nguyên trang; trang có vùng → vẽ ở DPI (theo ảnh scan
//! hoặc chọn), xoá ghim, ghi JPEG (hoặc ảnh đen trắng / xám khi "Nén đen trắng") thành trang mới cùng khổ — như XoaGhim.
//!
//! Nơi ghi theo `CauHinhGhi` (`dich_cua`): mặc định `<tên nguồn>_xoaghim/` giữ thư mục con và tên file; hoặc thư mục
//! chọn; hoặc cạnh file (`<tên>_xoaghim.pdf`); hoặc ghi đè chính file nguồn. Ghi file tạm → đẩy xuống đĩa → đổi tên
//! (mất điện không để lại file kết quả dở, ghi đè không làm hỏng nguồn); nhật ký ghi sau khi đổi tên xong.

use super::kho::{self as kho_xg, CauHinhGhi, CauHinhXuat, KieuGhi, NhatKy};
use super::vung::Vung;
use super::xu_ly::{xoa_ghim_trang, TuyChon};
use crate::tach_vb::anh::Anh;
use crate::tach_vb::kho::Id;
use mupdf::pdf::{PdfDocument, PdfWriteOptions};
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Hậu tố thư mục kết quả (và tên file kết quả khi ghi cạnh file).
pub const HAU_TO: &str = "_xoaghim";
/// Số trang tối đa của 1 file vẽ + xoá song song khi bật "Lô 10 trang".
pub const LO_TRANG: usize = 10;
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

/// Ảnh trang đã mã hoá xong, chưa thành đối tượng MuPDF (dữ liệu thuần — chuyển được giữa các luồng).
enum AnhMaHoa {
    Jpeg(Vec<u8>),
    /// Ảnh xám 1 kênh đã ngưỡng về 0 / 255.
    HaiMuc(Anh),
}

/// Ảnh trang đã xử lý → dữ liệu ảnh theo cách ghi đã chọn (phần nặng, chạy song song được).
fn ma_hoa_anh(anh: &Anh, xuat: &CauHinhXuat) -> Result<AnhMaHoa, String> {
    let jpeg = |du_lieu: &[u8], mau: jpeg_encoder::ColorType, chat_luong: u8| -> Result<AnhMaHoa, String> {
        let mut ra = Vec::new();
        let mut bm = jpeg_encoder::Encoder::new(&mut ra, chat_luong.clamp(1, 100));
        // Màu lấy mẫu 4:2:0 ở mọi chất lượng như OpenCV (bộ mã hoá mặc định 4:4:4 khi ≥ 90 → file lớn hơn ~18%)
        bm.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
        bm.encode(du_lieu, anh.rong as u16, anh.cao as u16, mau).map_err(|e| e.to_string())?;
        Ok(AnhMaHoa::Jpeg(ra))
    };
    if anh.rong > u16::MAX as usize || anh.cao > u16::MAX as usize {
        return Err("trang quá lớn".into());
    }
    if xuat.nen_den_trang {
        let mut xam = xam_opencv(anh);
        if la_anh_den_trang(&xam) {
            // Ngưỡng 127 → ảnh 2 mức, ghi nén không mất (PDF nén Flate — gọn với ảnh 2 mức)
            for v in xam.du_lieu.iter_mut() {
                *v = if *v > 127 { 255 } else { 0 };
            }
            return Ok(AnhMaHoa::HaiMuc(xam));
        }
        if la_anh_xam(anh) {
            return jpeg(&xam.du_lieu, jpeg_encoder::ColorType::Luma, xuat.chat_luong.min(90));
        }
    }
    jpeg(&anh.du_lieu, jpeg_encoder::ColorType::Rgb, xuat.chat_luong)
}

/// Dữ liệu ảnh đã mã hoá → ảnh MuPDF (ở luồng đang ghi PDF đích).
fn thanh_anh_pdf(a: &AnhMaHoa) -> Result<mupdf::Image, String> {
    match a {
        AnhMaHoa::Jpeg(b) => mupdf::Image::from_bytes(b).map_err(loi),
        AnhMaHoa::HaiMuc(xam) => {
            let mut pix = mupdf::Pixmap::new_with_w_h(&mupdf::Colorspace::device_gray(), xam.rong as i32, xam.cao as i32, false).map_err(loi)?;
            pix.samples_mut().copy_from_slice(&xam.du_lieu);
            mupdf::Image::from_pixmap(&pix).map_err(loi)
        }
    }
}

/// Ảnh trang đã xử lý → ảnh PDF theo cách ghi đã chọn.
#[cfg(test)]
fn anh_pdf(anh: &Anh, xuat: &CauHinhXuat) -> Result<mupdf::Image, String> {
    thanh_anh_pdf(&ma_hoa_anh(anh, xuat)?)
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

/// Vẽ trang `so` (từ 0) như lúc xuất: ở DPI chọn (`dpi_chon`, 0 = theo ảnh scan, chặn 600). Trả (ảnh RGB, DPI đã vẽ,
/// khổ trang điểm). Xem trước dùng đúng hàm này + `xoa_ghim_trang` nên ảnh xem trước = ảnh ghi vào PDF (trước nén JPEG).
pub fn ve_trang(doc: &PdfDocument, so: usize, dpi_chon: u16) -> Result<(Anh, u16, (f32, f32)), String> {
    let dpi = if dpi_chon == 0 { dpi_anh_trang(doc, so) } else { dpi_chon.min(DPI_TOI_DA) };
    let trang = doc.load_page(so as i32).map_err(loi)?;
    let khung = trang.bounds().map_err(loi)?;
    let anh = crate::tach_vb::phan_tich::render_rgb(&trang, dpi as f32)?;
    Ok((anh, dpi, (khung.x1 - khung.x0, khung.y1 - khung.y0)))
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

/// Tuỳ chọn cách chạy (không đổi nội dung kết quả).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TuyChonChay {
    /// Vẽ + xoá tối đa `LO_TRANG` trang của 1 file song song (nhanh hơn, tốn RAM hơn); tắt = từng trang.
    pub lo_10: bool,
}

/// 1 trang đã vẽ + xoá ghim + mã hoá ảnh.
struct TrangXong {
    anh: AnhMaHoa,
    dpi: u16,
    kho: (f32, f32),
}

fn xu_ly_trang(doc: &PdfDocument, so: usize, vung: &[Vung], tc: &TuyChon, xuat: &CauHinhXuat) -> Result<TrangXong, String> {
    let (mut anh, dpi, kho) = ve_trang(doc, so, xuat.dpi)?;
    xoa_ghim_trang(&mut anh, vung, so as u32, dpi as f64, tc, &[]);
    Ok(TrangXong { anh: ma_hoa_anh(&anh, xuat)?, dpi, kho })
}

/// Xoá ghim `nguon` → nội dung PDF kết quả (chưa ghi file). `vung`: vùng chung rồi vùng riêng của file, đúng thứ tự.
pub fn xoa_ghim_pdf(nguon: &Path, vung: &[Vung], tc: &TuyChon, xuat: &CauHinhXuat, dung: &AtomicBool) -> Result<(Vec<u8>, KetQuaFile), String> {
    xoa_ghim_pdf_voi(nguon, vung, tc, xuat, TuyChonChay::default(), dung)
}

/// Như `xoa_ghim_pdf`, theo `chay`. Lô 10 trang: mỗi lô vẽ + xoá + mã hoá song song trên `pool()` (mỗi luồng mở
/// PDF nguồn riêng — tài liệu MuPDF không dùng chung giữa luồng), rồi ghép vào PDF đích đúng thứ tự trang →
/// nội dung kết quả giống hệt chạy từng trang.
pub fn xoa_ghim_pdf_voi(nguon: &Path, vung: &[Vung], tc: &TuyChon, xuat: &CauHinhXuat, chay: TuyChonChay, dung: &AtomicBool) -> Result<(Vec<u8>, KetQuaFile), String> {
    let ten = nguon.to_str().ok_or("đường dẫn không hợp lệ")?;
    let src = PdfDocument::open(ten).map_err(loi)?;
    if src.needs_password().map_err(loi)? {
        return Err("PDF có mật khẩu".into());
    }
    let so_trang = src.page_count().map_err(loi)? as usize;
    let mut dst = PdfDocument::new();
    let mut ghep = dst.new_graft_map().map_err(loi)?;
    let mut kq = KetQuaFile { so_trang: so_trang as u32, trang_xu_ly: Vec::new(), dpi: 0 };
    let co_vung = |so: usize| vung.iter().any(|v| v.ap_trang.ap_dung(so as u32));
    let buoc = if chay.lo_10 { LO_TRANG } else { 1 };
    for dau in (0..so_trang).step_by(buoc) {
        if dung.load(Ordering::Relaxed) {
            return Err(super::lo::DA_DUNG.into());
        }
        let cuoi = (dau + buoc).min(so_trang);
        let can: Vec<usize> = (dau..cuoi).filter(|&so| co_vung(so)).collect();
        let xong: Vec<Result<TrangXong, String>> = if can.len() > 1 {
            pool().install(|| {
                can.par_iter()
                    .map_init(
                        || PdfDocument::open(ten).map_err(loi),
                        |doc, &so| {
                            if dung.load(Ordering::Relaxed) {
                                return Err(super::lo::DA_DUNG.to_string());
                            }
                            xu_ly_trang(doc.as_ref().map_err(|e| e.clone())?, so, vung, tc, xuat)
                        },
                    )
                    .collect()
            })
        } else {
            can.iter().map(|&so| xu_ly_trang(&src, so, vung, tc, xuat)).collect()
        };
        let mut xong = xong.into_iter();
        for so in dau..cuoi {
            if !co_vung(so) {
                crate::tach_vb::cat_pdf::ghep_trang(&src, so, &mut dst, &mut ghep, so, 0)?;
                continue;
            }
            let t = xong.next().ok_or("thiếu trang đã xử lý")??;
            them_trang_anh(&mut dst, so, t.kho.0, t.kho.1, &thanh_anh_pdf(&t.anh)?)?;
            kq.trang_xu_ly.push(so as u32);
            kq.dpi = kq.dpi.max(t.dpi);
        }
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

/// Đường dẫn của `file` so với nguồn `goc` (thư mục, hoặc chính file khi mở 1 file → tên file).
fn tuong_doi(goc: &Path, file: &Path) -> Option<PathBuf> {
    if goc == file {
        return file.file_name().map(PathBuf::from);
    }
    file.strip_prefix(goc).ok().map(Path::to_path_buf)
}

/// Tên file là kết quả Xóa ghim ghi cạnh file (`*_xoaghim.pdf`, không phân biệt hoa thường).
pub fn la_file_ket_qua(p: &Path) -> bool {
    p.file_name().is_some_and(|t| t.to_string_lossy().to_lowercase().ends_with(&format!("{HAU_TO}.pdf")))
}

/// Đích của `file` (trong nguồn `goc` — thư mục, hoặc chính file khi mở 1 file) theo nơi ghi `ghi`:
/// - ghi đè → chính `file`;
/// - nhân bản, chưa chọn thư mục → `<goc>_xoaghim/` (mở 1 file: `<thư mục cha>_xoaghim/`) + đường dẫn tương đối;
/// - nhân bản vào thư mục `d` → `d` + đường dẫn tương đối; `d` nằm trong thư mục nguồn → lỗi (lần quét sau lẫn kết quả);
/// - cạnh file → `<tên>_xoaghim.pdf` cùng thư mục.
///
/// Ngoài ghi đè, đích trùng chính file nguồn → lỗi.
pub fn dich_cua(goc: &Path, file: &Path, ghi: &CauHinhGhi) -> Result<PathBuf, String> {
    if ghi.ghi_de {
        return Ok(file.to_path_buf());
    }
    let dich = match ghi.kieu {
        KieuGhi::CanhFile => {
            let ten = file.file_stem().ok_or("file không có tên")?.to_string_lossy();
            file.with_file_name(format!("{ten}{HAU_TO}.pdf"))
        }
        KieuGhi::NhanBan => {
            let mot_file = goc == file || goc.is_file();
            let thu_muc_goc = if mot_file { goc.parent().ok_or("file không có thư mục cha")? } else { goc };
            let tuong_doi = tuong_doi(goc, file).ok_or_else(|| format!("{} không nằm trong {}", file.display(), goc.display()))?;
            let ra = match &ghi.thu_muc {
                None => crate::tach_vb::tach::thu_muc_ra_hau_to(thu_muc_goc, None, HAU_TO)?,
                Some(d) if !mot_file && crate::tach_vb::tach::nam_trong(d, thu_muc_goc) => {
                    return Err(format!("thư mục lưu kết quả {} nằm trong thư mục nguồn — lần quét sau sẽ lẫn kết quả", d.display()));
                }
                Some(d) => d.clone(),
            };
            ra.join(tuong_doi)
        }
    };
    if crate::tach_vb::tach::nam_trong(&dich, file) {
        return Err(format!("file kết quả {} trùng file nguồn — chọn nơi lưu khác hoặc bật Ghi đè", dich.display()));
    }
    Ok(dich)
}

/// Các PDF của nguồn (thư mục: theo thứ tự cây; 1 file: chính nó), luôn bỏ kết quả Xóa ghim: thư mục `*_xoaghim`
/// nằm bên trong và file `*_xoaghim.pdf`.
pub fn ds_pdf(nguon: &Path, dung: &AtomicBool) -> Vec<PathBuf> {
    let mut ds = Vec::new();
    if nguon.is_file() {
        ds.push(nguon.to_path_buf());
    } else {
        crate::tach_vb::phien::duyet_pdf_theo_cay(nguon, dung, |lo| ds.extend(lo));
    }
    let trong_ket_qua = |p: &Path| tuong_doi(nguon, p).is_some_and(|t| t.parent().is_some_and(|c| c.components().any(|x| x.as_os_str().to_string_lossy().to_lowercase().ends_with(HAU_TO))));
    ds.retain(|p| !trong_ket_qua(p) && !la_file_ket_qua(p));
    ds
}

/// Các PDF của nguồn (`ds_pdf`) kèm đường dẫn kết quả tương ứng trong `ra` (giữ thư mục con, giữ tên file).
pub fn ds_viec(nguon: &Path, ra: &Path, dung: &AtomicBool) -> Vec<(PathBuf, PathBuf)> {
    ds_pdf(nguon, dung)
        .into_iter()
        .filter_map(|p| {
            let dich = ra.join(tuong_doi(nguon, &p)?);
            Some((p, dich))
        })
        .collect()
}

/// Các PDF của nguồn (`ds_pdf`) kèm đích theo `dich_cua`. Nơi ghi không hợp lệ → lỗi (như nhau cho mọi file).
pub fn ds_viec_ghi(nguon: &Path, ghi: &CauHinhGhi, dung: &AtomicBool) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    ds_pdf(nguon, dung)
        .into_iter()
        .map(|p| {
            let dich = dich_cua(nguon, &p, ghi)?;
            Ok((p, dich))
        })
        .collect()
}

/// Id nguồn gốc của 1 file: file đã bị ghi đè bằng kết quả (id nằm trong bảng `nguon_theo_ket_qua`) → id nguồn trước
/// khi ghi đè (vùng riêng + nhật ký vẫn theo id này); không thì chính nó.
pub fn id_goc(nguon_theo_ket_qua: &std::collections::HashMap<Id, Id>, id: Id) -> Id {
    nguon_theo_ket_qua.get(&id).copied().unwrap_or(id)
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
/// Ghi đè: nhật ký tra theo id file hiện tại qua `nguon_theo_ket_qua` (`None` nếu không phải kết quả) và
/// `id_dich` = id file hiện tại → chỉ ra "đã xoá" / "chưa xoá" (`lo::doc_tinh_trang` làm sẵn).
pub fn trang_thai(nk: Option<&NhatKy>, id_dich: Option<Id>, ma_cau_hinh_hien_tai: u64) -> TrangThai {
    match nk {
        None => TrangThai::ChuaXoa,
        Some(nk) if id_dich == Some(nk.ket_qua) => TrangThai::DaXoa { dung_cau_hinh: nk.ma_cau_hinh == ma_cau_hinh_hien_tai },
        Some(_) => TrangThai::KetQuaMat,
    }
}

/// Đường dẫn chưa chắc tồn tại → id nội dung (đọc 64 KB đầu + cuối) mà không ghi nhớ vào bảng đường dẫn.
pub(crate) fn id_file(p: &Path) -> Option<Id> {
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
    xoa_ghim_file_voi(kho, nguon_theo_ket_qua, nguon, id_nguon, dich, vung, tc, xuat, TuyChonChay::default(), dung)
}

/// Như `xoa_ghim_file`, theo `chay`. `dich == nguon` = ghi đè nguồn: ghi nguyên tử (file tạm cùng thư mục rồi đổi
/// tên), nhật ký khoá theo id nguồn gốc (`id_goc`) để file đã ghi đè vẫn tra ra nhật ký / vùng riêng.
#[allow(clippy::too_many_arguments)]
pub fn xoa_ghim_file_voi(
    kho: &std::sync::Mutex<crate::tach_vb::kho::Kho>,
    nguon_theo_ket_qua: &std::collections::HashMap<Id, Id>,
    nguon: &Path,
    id_nguon: Id,
    dich: &Path,
    vung: &[Vung],
    tc: &TuyChon,
    xuat: &CauHinhXuat,
    chay: TuyChonChay,
    dung: &AtomicBool,
) -> Result<KetQuaFile, String> {
    let khoa = || kho.lock().unwrap_or_else(|e| e.into_inner());
    let id_nguon = if dich == nguon {
        id_goc(nguon_theo_ket_qua, id_nguon)
    } else if id_file(dich).and_then(|id_cu| nguon_theo_ket_qua.get(&id_cu)).is_some_and(|n| *n != id_nguon) {
        return Err(LOI_NGUON_KHAC.into());
    } else {
        id_nguon
    };
    let (noi_dung, kq) = xoa_ghim_pdf_voi(nguon, vung, tc, xuat, chay, dung)?;
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
        for f in ["a.pdf", "con/b.pdf", "cu_xoaghim/c.pdf", "con/X_XOAGHIM/d.pdf", "a_xoaghim.pdf", "con/B_XoaGhim.PDF"] {
            std::fs::create_dir_all(nguon.join(f).parent().unwrap()).unwrap();
            std::fs::write(nguon.join(f), b"%PDF").unwrap();
        }
        let ra = d.join("Nguon_xoaghim");
        let dung = AtomicBool::new(false);
        let ds = ds_viec(&nguon, &ra, &dung);
        assert_eq!(ds, vec![(nguon.join("a.pdf"), ra.join("a.pdf")), (nguon.join("con/b.pdf"), ra.join("con/b.pdf"))]);
        // Theo nơi ghi: mặc định như trên; cạnh file; ghi đè
        assert_eq!(ds_viec_ghi(&nguon, &CauHinhGhi::default(), &dung).unwrap(), ds);
        let canh = CauHinhGhi { kieu: KieuGhi::CanhFile, ..Default::default() };
        assert_eq!(ds_viec_ghi(&nguon, &canh, &dung).unwrap()[1], (nguon.join("con/b.pdf"), nguon.join("con/b_xoaghim.pdf")));
        let de = CauHinhGhi { ghi_de: true, ..canh };
        assert!(ds_viec_ghi(&nguon, &de, &dung).unwrap().iter().all(|(n, d)| n == d));
        // Thư mục chọn nằm trong nguồn → lỗi
        assert!(ds_viec_ghi(&nguon, &CauHinhGhi { thu_muc: Some(nguon.join("con")), ..Default::default() }, &dung).is_err());
        // Mở 1 file: chính nó; file kết quả `*_xoaghim.pdf` bỏ
        assert_eq!(ds_pdf(&nguon.join("a.pdf"), &dung), vec![nguon.join("a.pdf")]);
        assert!(ds_pdf(&nguon.join("a_xoaghim.pdf"), &dung).is_empty());
    }

    #[test]
    fn dich_cua_theo_noi_ghi_ca_thu_muc_va_mot_file() {
        let d = tam("dich");
        let goc = d.join("Nguon");
        let f = goc.join("con/a.pdf");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, b"%PDF").unwrap();
        let mac_dinh = CauHinhGhi::default();
        assert_eq!(dich_cua(&goc, &f, &mac_dinh).unwrap(), d.join("Nguon_xoaghim/con/a.pdf"));
        let chon = CauHinhGhi { thu_muc: Some(d.join("Ra")), ..Default::default() };
        assert_eq!(dich_cua(&goc, &f, &chon).unwrap(), d.join("Ra/con/a.pdf"));
        assert!(dich_cua(&goc, &f, &CauHinhGhi { thu_muc: Some(goc.join("CON")), ..Default::default() }).is_err(), "trong nguồn (không phân biệt hoa thường)");
        let canh = CauHinhGhi { kieu: KieuGhi::CanhFile, thu_muc: Some(d.join("Ra")), ghi_de: false };
        assert_eq!(dich_cua(&goc, &f, &canh).unwrap(), goc.join("con/a_xoaghim.pdf"), "cạnh file bỏ qua thư mục chọn");
        assert_eq!(dich_cua(&goc, &f, &CauHinhGhi { ghi_de: true, ..canh.clone() }).unwrap(), f, "ghi đè thắng kiểu");
        // Mở 1 file: tương đối = tên file, mặc định `<cha>_xoaghim`
        assert_eq!(dich_cua(&f, &f, &mac_dinh).unwrap(), goc.join("con_xoaghim/a.pdf"));
        assert_eq!(dich_cua(&f, &f, &chon).unwrap(), d.join("Ra/a.pdf"));
        assert_eq!(dich_cua(&f, &f, &canh).unwrap(), goc.join("con/a_xoaghim.pdf"));
        assert!(dich_cua(&f, &f, &CauHinhGhi { thu_muc: Some(goc.join("con")), ..Default::default() }).is_err(), "đích trùng nguồn");
        assert_eq!(dich_cua(&f, &f, &CauHinhGhi { thu_muc: Some(goc.clone()), ..Default::default() }).unwrap(), goc.join("a.pdf"), "1 file: thư mục chọn ở trên vẫn được");
    }

    #[test]
    fn lo_10_trang_cho_ket_qua_giong_het_tung_trang() {
        let d = tam("lo10");
        let nguon = d.join("a.pdf");
        tao_pdf(&nguon, 23);
        let vung = [goc_tren_trai(), Vung { ap_trang: ApTrang::MotTrang(12), ..goc_tren_trai() }];
        let dung = AtomicBool::new(false);
        for xuat in [CauHinhXuat::default(), CauHinhXuat { nen_den_trang: true, dpi: 120, ..Default::default() }] {
            let tung = xoa_ghim_pdf_voi(&nguon, &vung, &TuyChon::default(), &xuat, TuyChonChay { lo_10: false }, &dung).unwrap();
            let lo = xoa_ghim_pdf_voi(&nguon, &vung, &TuyChon::default(), &xuat, TuyChonChay { lo_10: true }, &dung).unwrap();
            assert_eq!(tung.1, lo.1);
            assert_eq!(lo.1.trang_xu_ly, (0..23).step_by(2).chain([12]).collect::<std::collections::BTreeSet<u32>>().into_iter().collect::<Vec<_>>());
            assert!(tung.0 == lo.0, "nội dung PDF khác nhau");
        }
        assert_eq!(xoa_ghim_pdf_voi(&nguon, &vung, &TuyChon::default(), &CauHinhXuat::default(), TuyChonChay { lo_10: true }, &AtomicBool::new(true)).unwrap_err(), super::super::lo::DA_DUNG);
    }

    #[test]
    fn ghi_de_nguon_nguyen_tu_nhat_ky_theo_nguon_goc() {
        let d = tam("ghi_de");
        let kho = std::sync::Mutex::new(Kho::mo(&d.join("kho_xoa_ghim.bin")).unwrap());
        let a = d.join("a.pdf");
        tao_pdf(&a, 2);
        let id_s = id_file(&a).unwrap();
        let (vung, tc, xuat, dung) = ([goc_tren_trai()], TuyChon::default(), CauHinhXuat::default(), AtomicBool::new(false));
        let ma = kho_xg::ma_cau_hinh(&vung, &tc, &xuat);
        xoa_ghim_file(&kho, &bang(&kho), &a, id_s, &a, &vung, &tc, &xuat, &dung).unwrap();
        assert!(std::fs::read_dir(&d).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")), "không sót file tạm");
        let id_r1 = id_file(&a).unwrap();
        assert_ne!(id_r1, id_s);
        let b = bang(&kho);
        assert_eq!(b.get(&id_r1), Some(&id_s), "kết quả → nguồn gốc");
        let nk = kho_xg::doc_nhat_ky(&mut kho.lock().unwrap(), id_goc(&b, id_r1));
        assert_eq!(trang_thai(nk.as_ref(), Some(id_r1), ma), TrangThai::DaXoa { dung_cau_hinh: true });
        // Chạy lại trên file đã ghi đè (id hiện tại là kết quả): không báo "nguồn khác", nhật ký vẫn khoá nguồn gốc
        xoa_ghim_file(&kho, &b, &a, id_r1, &a, &vung, &tc, &xuat, &dung).unwrap();
        let id_r2 = id_file(&a).unwrap();
        let b = bang(&kho);
        assert_eq!(b.get(&id_r2), Some(&id_s));
        assert_eq!(b.len(), 1, "chỉ 1 nhật ký — khoá nguồn gốc, không thêm nhật ký theo id kết quả");
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
