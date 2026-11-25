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
const PHIEN_BAN: u16 = 1;

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

/// Khoá nhận diện PDF: đổi kích thước / thời điểm sửa → cache không còn đúng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhoaFile {
    pub duong_dan: String,
    pub kich_thuoc: u64,
    pub sua_luc: u128,
}

impl KhoaFile {
    pub fn cua(pdf: &Path, duong_dan_tuong_doi: &str) -> Option<Self> {
        let md = std::fs::metadata(pdf).ok()?;
        let sua_luc = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        Some(Self { duong_dan: duong_dan_tuong_doi.to_string(), kich_thuoc: md.len(), sua_luc })
    }
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
    let khoa = KhoaFile { duong_dan: d.chuoi()?, kich_thuoc: d.u64()?, sua_luc: d.u128()? };
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

/// Đọc cache của 1 PDF; `None` khi chưa có, hỏng, hoặc PDF đã đổi (khoá khác).
pub fn doc(thu_muc_cache: &Path, khoa: &KhoaFile) -> Option<Vec<TrangCache>> {
    let du_lieu = std::fs::read(file_cache(thu_muc_cache, &khoa.duong_dan)).ok()?;
    let (k, trang) = giai_ma(&du_lieu)?;
    (k == *khoa).then_some(trang)
}

/// Ghi cache của 1 PDF (file tạm rồi đổi tên — tắt app giữa chừng không để lại file hỏng).
pub fn ghi(thu_muc_cache: &Path, khoa: &KhoaFile, trang: &[TrangCache]) -> std::io::Result<()> {
    std::fs::create_dir_all(thu_muc_cache)?;
    let dich = file_cache(thu_muc_cache, &khoa.duong_dan);
    let tam = dich.with_extension("tmp");
    std::fs::write(&tam, ma_hoa(khoa, trang))?;
    std::fs::rename(&tam, &dich)
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
        let khoa = KhoaFile { duong_dan: "HOP 462/4766/4766.pdf".into(), kich_thuoc: 9, sua_luc: 123 };
        let (k, t) = giai_ma(&ma_hoa(&khoa, &mau_trang())).unwrap();
        assert_eq!(k, khoa);
        assert_eq!(t[..2], mau_trang()[..2]);
        let h = t[2].ket_qua.as_ref().unwrap().huong.unwrap();
        assert_eq!(h.tang_1, [0.0, 1.0, 0.0, 0.0]);
        assert!((h.tang_2.unwrap()[2] - 0.6).abs() < 0.003);
        assert_eq!(t[2].trang_thai, DANG_CHINH_XOAY);
    }

    #[test]
    fn file_hong_hoac_pdf_doi_thi_bo_cache() {
        let thu_muc = std::env::temp_dir().join(format!("dema4_cache_{}", std::process::id()));
        let khoa = KhoaFile { duong_dan: "a.pdf".into(), kich_thuoc: 9, sua_luc: 1 };
        ghi(&thu_muc, &khoa, &mau_trang()).unwrap();
        assert_eq!(doc(&thu_muc, &khoa).map(|t| t.len()), Some(3));
        assert!(doc(&thu_muc, &KhoaFile { sua_luc: 2, ..khoa.clone() }).is_none(), "PDF đã sửa");
        let du_lieu = ma_hoa(&khoa, &mau_trang());
        for cat in [0, 5, du_lieu.len() - 1] {
            assert!(giai_ma(&du_lieu[..cat]).is_none(), "cắt ở {cat}");
        }
        std::fs::write(file_cache(&thu_muc, "a.pdf"), b"rac").unwrap();
        assert!(doc(&thu_muc, &khoa).is_none());
        xoa_het(&thu_muc).unwrap();
        assert!(!thu_muc.exists());
        xoa_het(&thu_muc).unwrap();
    }
}
