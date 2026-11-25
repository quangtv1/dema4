//! Cache kết quả phân tích trong `<thư mục dữ liệu ẩn>/cache/` (xem `du_lieu_an`): mỗi PDF 1 file nhị phân nhỏ.
//!
//! Khoá = đường dẫn tương đối + kích thước + thời điểm sửa của PDF; PDF đổi → cache file đó bị bỏ.
//! Lưu kết quả thô (dung lượng, kích thước, chỉ số trắng, mã đọc được, phân bố hướng) và trạng thái duyệt
//! của người dùng (giữ nội dung, đã xoá khỏi lưới, đang chỉnh xoay) → mở lại thư mục giữ nguyên trạng thái cuối,
//! đổi Cấu hình gán lại nhãn không cần render. Ghi file tạm rồi đổi tên; file hỏng → coi như chưa có.

use super::chi_so_trang::ChiSoTrang;
use super::doc_ma::MaDoc;
use super::dung_luong::ThongTinTrang;
use super::huong_trang::PhanBoHuong;
use super::phan_tich::KetQuaTrang;
use std::path::{Path, PathBuf};

const DAU_FILE: &[u8; 4] = b"SMD4";
/// 2: thêm vân tay nội dung PDF (bản 1 không đọc nữa — từ 2.3 cache nằm ở chỗ lưu mới, không có bản 1).
const PHIEN_BAN: u16 = 2;

/// Bit trạng thái duyệt của người dùng (không ghi vào `index_and_blank.txt`).
pub const GIU_NOI_DUNG: u8 = 1;
pub const DA_XOA: u8 = 2;
pub const DANG_CHINH_XOAY: u8 = 4;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrangCache {
    pub thong_tin: ThongTinTrang,
    /// `None` = chưa phân tích trang này.
    pub ket_qua: Option<KetQuaTrang>,
    pub trang_thai: u8,
}

/// Khoá nhận diện PDF. Khớp cache khi cùng đường dẫn + kích thước **và** (cùng thời điểm sửa tính theo giây **hoặc**
/// cùng vân tay nội dung). Vân tay giúp dùng lại cache khi chép thư mục qua USB / sang máy khác (thời điểm sửa đổi).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhoaFile {
    pub duong_dan: String,
    pub kich_thuoc: u64,
    /// ns từ epoch; so ở độ mịn giây (mỗi hệ điều hành / ổ đĩa giữ độ mịn khác nhau).
    pub sua_luc: u128,
    /// `van_tay_file`; 0 = chưa tính (tính lúc ghi cache).
    pub van_tay: u64,
}

impl KhoaFile {
    /// Chỉ đọc thông tin file (không đọc nội dung) — vân tay để 0.
    pub fn cua(pdf: &Path, duong_dan_tuong_doi: &str) -> Option<Self> {
        let md = std::fs::metadata(pdf).ok()?;
        let sua_luc = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        Some(Self { duong_dan: duong_dan_tuong_doi.to_string(), kich_thuoc: md.len(), sua_luc, van_tay: 0 })
    }
}

const NUA_VAN_TAY: u64 = 64 * 1024;

/// Vân tay nội dung: FNV-1a 64 của (kích thước ‖ 64 KB đầu ‖ 64 KB cuối); file ≤ 128 KB → cả file.
/// Giới hạn chấp nhận: sửa vài byte ở giữa file mà giữ nguyên kích thước thì không phát hiện (PDF scan sửa nội dung
/// hầu như luôn đổi kích thước hoặc bảng xref cuối file).
pub fn van_tay_file(pdf: &Path, kich_thuoc: u64) -> std::io::Result<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(pdf)?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut tron = |b: &[u8]| {
        for &x in b {
            h = (h ^ x as u64).wrapping_mul(0x100_0000_01b3);
        }
    };
    tron(&kich_thuoc.to_le_bytes());
    let mut buf = vec![0u8; NUA_VAN_TAY as usize];
    if kich_thuoc <= 2 * NUA_VAN_TAY {
        let mut ca = Vec::with_capacity(kich_thuoc as usize);
        f.read_to_end(&mut ca)?;
        tron(&ca);
    } else {
        f.read_exact(&mut buf)?;
        tron(&buf);
        f.seek(SeekFrom::Start(kich_thuoc - NUA_VAN_TAY))?;
        f.read_exact(&mut buf)?;
        tron(&buf);
    }
    Ok(h.max(1))
}

/// File cache của 1 PDF: tên = băm đường dẫn tương đối (ổn định giữa các lần chạy).
pub fn file_cache(thu_muc_cache: &Path, duong_dan_tuong_doi: &str) -> PathBuf {
    // FNV-1a 64 bit: không phụ thuộc phiên bản Rust như DefaultHasher
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in duong_dan_tuong_doi.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    thu_muc_cache.join(format!("{h:016x}.bin"))
}

struct Ghi(Vec<u8>);
impl Ghi {
    fn u8(&mut self, v: u8) { self.0.push(v); }
    fn u16(&mut self, v: u16) { self.0.extend(v.to_le_bytes()); }
    fn u32(&mut self, v: u32) { self.0.extend(v.to_le_bytes()); }
    fn u64(&mut self, v: u64) { self.0.extend(v.to_le_bytes()); }
    fn u128(&mut self, v: u128) { self.0.extend(v.to_le_bytes()); }
    fn f32(&mut self, v: f32) { self.0.extend(v.to_le_bytes()); }
    fn chuoi(&mut self, s: &str) { self.u32(s.len() as u32); self.0.extend(s.as_bytes()); }
    fn xac_suat(&mut self, p: &[f32; 4]) { for v in p { self.u8((v.clamp(0.0, 1.0) * 255.0).round() as u8); } }
}

struct Doc<'a>(&'a [u8]);
impl Doc<'_> {
    fn lay(&mut self, n: usize) -> Option<&[u8]> {
        (self.0.len() >= n).then(|| { let (a, b) = self.0.split_at(n); self.0 = b; a })
    }
    fn u8(&mut self) -> Option<u8> { self.lay(1).map(|b| b[0]) }
    fn u16(&mut self) -> Option<u16> { self.lay(2).map(|b| u16::from_le_bytes(b.try_into().unwrap())) }
    fn u32(&mut self) -> Option<u32> { self.lay(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())) }
    fn u64(&mut self) -> Option<u64> { self.lay(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())) }
    fn u128(&mut self) -> Option<u128> { self.lay(16).map(|b| u128::from_le_bytes(b.try_into().unwrap())) }
    fn f32(&mut self) -> Option<f32> { self.lay(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())) }
    fn chuoi(&mut self) -> Option<String> { let n = self.u32()? as usize; String::from_utf8(self.lay(n)?.to_vec()).ok() }
    fn xac_suat(&mut self) -> Option<[f32; 4]> { let b = self.lay(4)?; Some([0, 1, 2, 3].map(|i| b[i] as f32 / 255.0)) }
}

// Bit cờ của từng trang trong file
const CO_KET_QUA: u8 = 1;
const CO_QUET_MA: u8 = 2;
const CO_CHI_SO: u8 = 4;
const CO_HUONG: u8 = 8;
const CO_TANG_2: u8 = 16;

pub fn ma_hoa(khoa: &KhoaFile, trang: &[TrangCache]) -> Vec<u8> {
    let mut g = Ghi(Vec::with_capacity(64 + trang.len() * 40));
    g.0.extend(DAU_FILE);
    g.u16(PHIEN_BAN);
    g.chuoi(&khoa.duong_dan);
    g.u64(khoa.kich_thuoc);
    g.u128(khoa.sua_luc);
    g.u64(khoa.van_tay);
    g.u32(trang.len() as u32);
    for t in trang {
        g.u32(t.thong_tin.dung_luong);
        g.f32(t.thong_tin.rong);
        g.f32(t.thong_tin.cao);
        g.u8(t.trang_thai);
        let Some(kq) = &t.ket_qua else { g.u8(0); continue };
        let co = CO_KET_QUA
            | if kq.da_quet_ma { CO_QUET_MA } else { 0 }
            | if kq.chi_so.is_some() { CO_CHI_SO } else { 0 }
            | if kq.huong.is_some() { CO_HUONG } else { 0 }
            | if kq.huong.is_some_and(|h| h.tang_2.is_some()) { CO_TANG_2 } else { 0 };
        g.u8(co);
        if let Some(cs) = kq.chi_so {
            g.f32(cs.dam);
            g.f32(cs.mau);
        }
        if let Some(h) = kq.huong {
            g.xac_suat(&h.tang_1);
            if let Some(t2) = h.tang_2 {
                g.xac_suat(&t2);
            }
        }
        g.u8(kq.ma.len().min(255) as u8);
        for m in kq.ma.iter().take(255) {
            g.u8(m.loi_checksum as u8);
            g.chuoi(&m.gia_tri);
        }
    }
    g.0
}

/// Giải mã; `None` khi file hỏng hoặc sai phiên bản.
pub fn giai_ma(du_lieu: &[u8]) -> Option<(KhoaFile, Vec<TrangCache>)> {
    let mut d = Doc(du_lieu);
    if d.lay(4)? != DAU_FILE || d.u16()? != PHIEN_BAN {
        return None;
    }
    let khoa = KhoaFile { duong_dan: d.chuoi()?, kich_thuoc: d.u64()?, sua_luc: d.u128()?, van_tay: d.u64()? };
    let n = d.u32()? as usize;
    let mut trang = Vec::with_capacity(n.min(100_000));
    for _ in 0..n {
        let thong_tin = ThongTinTrang { dung_luong: d.u32()?, rong: d.f32()?, cao: d.f32()? };
        let trang_thai = d.u8()?;
        let co = d.u8()?;
        let ket_qua = if co & CO_KET_QUA == 0 {
            None
        } else {
            let chi_so = if co & CO_CHI_SO != 0 { Some(ChiSoTrang { dam: d.f32()?, mau: d.f32()? }) } else { None };
            let huong = if co & CO_HUONG != 0 {
                let tang_1 = d.xac_suat()?;
                let tang_2 = if co & CO_TANG_2 != 0 { Some(d.xac_suat()?) } else { None };
                Some(PhanBoHuong { tang_1, tang_2 })
            } else {
                None
            };
            let so_ma = d.u8()? as usize;
            let mut ma = Vec::with_capacity(so_ma);
            for _ in 0..so_ma {
                let loi_checksum = d.u8()? != 0;
                ma.push(MaDoc { gia_tri: d.chuoi()?, loi_checksum });
            }
            Some(KetQuaTrang { da_quet_ma: co & CO_QUET_MA != 0, ma, chi_so, huong })
        };
        trang.push(TrangCache { thong_tin, ket_qua, trang_thai });
    }
    d.0.is_empty().then_some((khoa, trang))
}

/// Đọc cache của PDF `pdf` (khoá `khoa` lấy từ thông tin file hiện tại). Trả khoá đầy đủ (kèm vân tay đã lưu) + trang;
/// `None` khi chưa có, hỏng, hoặc PDF đã đổi. Thời điểm sửa lệch (chép sang máy khác) mà vân tay khớp → vẫn dùng và
/// ghi lại phần đầu file cache với thời điểm mới (lần sau khớp ngay, không đọc lại PDF).
pub fn doc(thu_muc_cache: &Path, pdf: &Path, khoa: &KhoaFile) -> Option<(KhoaFile, Vec<TrangCache>)> {
    let du_lieu = std::fs::read(file_cache(thu_muc_cache, &khoa.duong_dan)).ok()?;
    let (luu, trang) = giai_ma(&du_lieu)?;
    if luu.duong_dan != khoa.duong_dan || luu.kich_thuoc != khoa.kich_thuoc {
        return None;
    }
    if luu.sua_luc / 1_000_000_000 == khoa.sua_luc / 1_000_000_000 {
        return Some((KhoaFile { van_tay: luu.van_tay, ..khoa.clone() }, trang));
    }
    if luu.van_tay == 0 || van_tay_file(pdf, khoa.kich_thuoc).ok()? != luu.van_tay {
        return None;
    }
    let moi = KhoaFile { van_tay: luu.van_tay, ..khoa.clone() };
    let _ = ghi_file_cache(thu_muc_cache, &moi, &trang);
    Some((moi, trang))
}

/// Ghi cache của 1 PDF; khoá chưa có vân tay thì tính từ `pdf` (lỗi đọc → để 0, cache vẫn ghi). Trả khoá đã ghi.
pub fn ghi(thu_muc_cache: &Path, pdf: &Path, khoa: &KhoaFile, trang: &[TrangCache]) -> std::io::Result<KhoaFile> {
    let mut k = khoa.clone();
    if k.van_tay == 0 {
        k.van_tay = van_tay_file(pdf, k.kich_thuoc).unwrap_or(0);
    }
    ghi_file_cache(thu_muc_cache, &k, trang)?;
    Ok(k)
}

/// File tạm rồi đổi tên — tắt app giữa chừng không để lại file hỏng.
fn ghi_file_cache(thu_muc_cache: &Path, khoa: &KhoaFile, trang: &[TrangCache]) -> std::io::Result<()> {
    crate::ghi_file::ghi_an_toan(&file_cache(thu_muc_cache, &khoa.duong_dan), ma_hoa(khoa, trang))
}

/// Phân tích lại: xoá toàn bộ cache.
pub fn xoa_het(thu_muc_cache: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(thu_muc_cache) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mau_trang() -> Vec<TrangCache> {
        vec![
            TrangCache { thong_tin: ThongTinTrang { dung_luong: 12345, rong: 595.0, cao: 842.0 }, ket_qua: None, trang_thai: 0 },
            TrangCache {
                thong_tin: ThongTinTrang { dung_luong: 300, rong: 842.0, cao: 595.0 },
                ket_qua: Some(KetQuaTrang { da_quet_ma: true, ma: vec![MaDoc { gia_tri: "NAMSAO-TACH".into(), loi_checksum: false }, MaDoc { gia_tri: "NA_SAO-TACH".into(), loi_checksum: true }], chi_so: Some(ChiSoTrang { dam: 0.0123, mau: 0.0004 }), huong: None }),
                trang_thai: GIU_NOI_DUNG | DA_XOA,
            },
            TrangCache {
                thong_tin: ThongTinTrang::default(),
                ket_qua: Some(KetQuaTrang { da_quet_ma: false, ma: vec![], chi_so: None, huong: Some(PhanBoHuong { tang_1: [0.0, 1.0, 0.0, 0.0], tang_2: Some([0.2, 0.2, 0.6, 0.0]) }) }),
                trang_thai: DANG_CHINH_XOAY,
            },
        ]
    }

    #[test]
    fn khu_hoi_giu_nguyen_va_xac_suat_lam_tron_1_255() {
        let khoa = KhoaFile { duong_dan: "HOP 462/4766/4766.pdf".into(), kich_thuoc: 9, sua_luc: 123, van_tay: 0xabc };
        let (k, t) = giai_ma(&ma_hoa(&khoa, &mau_trang())).unwrap();
        assert_eq!(k, khoa);
        assert_eq!(t[..2], mau_trang()[..2]);
        let h = t[2].ket_qua.as_ref().unwrap().huong.unwrap();
        assert_eq!(h.tang_1, [0.0, 1.0, 0.0, 0.0]);
        assert!((h.tang_2.unwrap()[2] - 0.6).abs() < 0.003);
        assert_eq!(t[2].trang_thai, DANG_CHINH_XOAY);
    }

    fn tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_cache_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// PDF giả: nội dung bất kỳ (vân tay chỉ đọc byte), `n` byte.
    fn pdf_gia(d: &Path, n: usize) -> PathBuf {
        let f = d.join("a.pdf");
        std::fs::write(&f, (0..n).map(|i| (i * 7 % 251) as u8).collect::<Vec<_>>()).unwrap();
        f
    }

    #[test]
    fn file_hong_hoac_pdf_doi_thi_bo_cache() {
        let d = tam("hong");
        let (thu_muc, pdf) = (d.join("cache"), pdf_gia(&d, 9));
        let khoa = KhoaFile::cua(&pdf, "a.pdf").unwrap();
        ghi(&thu_muc, &pdf, &khoa, &mau_trang()).unwrap();
        assert_eq!(doc(&thu_muc, &pdf, &khoa).map(|(_, t)| t.len()), Some(3));
        assert!(doc(&thu_muc, &pdf, &KhoaFile { kich_thuoc: 10, ..khoa.clone() }).is_none(), "PDF đổi kích thước");
        let du_lieu = ma_hoa(&khoa, &mau_trang());
        for cat in [0, 5, du_lieu.len() - 1] {
            assert!(giai_ma(&du_lieu[..cat]).is_none(), "cắt ở {cat}");
        }
        std::fs::write(file_cache(&thu_muc, "a.pdf"), b"rac").unwrap();
        assert!(doc(&thu_muc, &pdf, &khoa).is_none());
        xoa_het(&thu_muc).unwrap();
        assert!(!thu_muc.exists());
        xoa_het(&thu_muc).unwrap();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn chep_sang_may_khac_mtime_doi_van_dung_cache_nho_van_tay() {
        let d = tam("van_tay");
        let (thu_muc, pdf) = (d.join("cache"), pdf_gia(&d, 200_000));
        let khoa = KhoaFile::cua(&pdf, "a.pdf").unwrap();
        ghi(&thu_muc, &pdf, &khoa, &mau_trang()).unwrap();
        let (luu, _) = giai_ma(&std::fs::read(file_cache(&thu_muc, "a.pdf")).unwrap()).unwrap();
        assert_ne!(luu.van_tay, 0, "vân tay tính và lưu ngay lúc ghi (máy nguồn)");
        // Máy khác / chép USB: cùng nội dung, mtime lệch nhiều giây
        let khac = KhoaFile { sua_luc: khoa.sua_luc + 3_600_000_000_000, van_tay: 0, ..khoa.clone() };
        let (k, t) = doc(&thu_muc, &pdf, &khac).expect("khớp nhờ vân tay");
        assert_eq!((t.len(), k.sua_luc, k.van_tay), (3, khac.sua_luc, luu.van_tay));
        let (luu2, _) = giai_ma(&std::fs::read(file_cache(&thu_muc, "a.pdf")).unwrap()).unwrap();
        assert_eq!(luu2.sua_luc, khac.sua_luc, "ghi lại header → lần sau đi đường tắt theo mtime");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn doi_noi_dung_cung_kich_thuoc_va_mtime_lech_thi_bo_cache() {
        let d = tam("doi_byte");
        let (thu_muc, pdf) = (d.join("cache"), pdf_gia(&d, 200_000));
        let khoa = KhoaFile::cua(&pdf, "a.pdf").unwrap();
        ghi(&thu_muc, &pdf, &khoa, &mau_trang()).unwrap();
        let mut b = std::fs::read(&pdf).unwrap();
        b[10] ^= 0xff;
        std::fs::write(&pdf, b).unwrap();
        let khac = KhoaFile { sua_luc: khoa.sua_luc + 5_000_000_000, van_tay: 0, ..khoa.clone() };
        assert!(doc(&thu_muc, &pdf, &khac).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mtime_lech_duoi_1_giay_khop_ngay_khong_can_van_tay() {
        let d = tam("giay");
        let (thu_muc, pdf) = (d.join("cache"), pdf_gia(&d, 50));
        let khoa = KhoaFile { duong_dan: "a.pdf".into(), kich_thuoc: 50, sua_luc: 7_000_000_000, van_tay: 0 };
        std::fs::create_dir_all(&thu_muc).unwrap();
        std::fs::write(file_cache(&thu_muc, "a.pdf"), ma_hoa(&khoa, &mau_trang())).unwrap(); // không có vân tay
        let gan = KhoaFile { sua_luc: 7_000_500_000, ..khoa.clone() };
        assert!(doc(&thu_muc, &pdf, &gan).is_some(), "cùng giây (độ mịn khác nhau giữa HĐH)");
        let xa = KhoaFile { sua_luc: 9_000_000_000, ..khoa.clone() };
        assert!(doc(&thu_muc, &pdf, &xa).is_none(), "khác giây và không có vân tay lưu → bỏ");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn van_tay_file_nho_ca_file_file_lon_dau_cuoi() {
        let d = tam("vt");
        let nho = pdf_gia(&d, 1000);
        assert_eq!(van_tay_file(&nho, 1000).unwrap(), van_tay_file(&nho, 1000).unwrap());
        let lon = pdf_gia(&d, 300_000);
        let goc = van_tay_file(&lon, 300_000).unwrap();
        let mut b = std::fs::read(&lon).unwrap();
        b[150_000] ^= 1; // giữa file: ngoài 64 KB đầu / cuối (giới hạn đã chấp nhận)
        std::fs::write(&lon, &b).unwrap();
        assert_eq!(van_tay_file(&lon, 300_000).unwrap(), goc);
        b[299_999] ^= 1; // cuối file (xref) → khác
        std::fs::write(&lon, &b).unwrap();
        assert_ne!(van_tay_file(&lon, 300_000).unwrap(), goc);
        let _ = std::fs::remove_dir_all(&d);
    }
}
