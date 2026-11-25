//! Dữ liệu Xóa ghim trong kho riêng `<gốc dữ liệu>/kho_xoa_ghim.bin` (cùng code kho Tách, loại bản ghi riêng):
//! - `VUNG_FILE`: vùng riêng của 1 PDF (khoá id nội dung PDF — đổi tên / di chuyển vẫn theo);
//! - `VUNG_THU_MUC`: vùng chung + tuỳ chọn của 1 thư mục nguồn (khoá đường dẫn thư mục — đổi tên thư mục thì chọn lại);
//! - `NHAT_KY`: lần xoá ghim sau cùng của 1 PDF nguồn (khoá id nguồn): id file kết quả, mã cấu hình, trang đã xử lý.
//!
//! Mọi nội dung mở đầu bằng phiên bản định dạng (1 byte) + mốc sửa (ms) — gộp giữa máy: mốc sau thắng.
//! Đọc nội dung lạ / hỏng / ngoài giới hạn → coi như không có (không làm hỏng phần khác).
//! Thứ tự khoá khi cần cả 2 kho: kho Tách rồi kho Xóa ghim, không giữ 2 khoá qua đọc / ghi file.

use super::vung::{ApTrang, LoaiVung, ViTri, Vung};
use super::xu_ly::TuyChon;
use crate::tach_vb::kho::{Id, Kho, KHAC_TU};

pub const VUNG_FILE: u8 = KHAC_TU;
pub const VUNG_THU_MUC: u8 = KHAC_TU + 1;
pub const NHAT_KY: u8 = KHAC_TU + 2;

const PHIEN_BAN: u8 = 1;
/// Giới hạn số vùng của 1 bản ghi (chặn dữ liệu hỏng / gói lạ làm phình bộ nhớ).
pub const TOI_DA_VUNG: usize = 500;

/// Cách ghi trang đã xử lý.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CauHinhXuat {
    /// DPI vẽ trang đã xử lý; 0 = theo ảnh scan của trang (tối đa 600).
    pub dpi: u16,
    /// Chất lượng JPEG (1–100).
    pub chat_luong: u8,
    /// Trang đen trắng / xám ghi gọn hơn (đen trắng: ảnh 2 mức nén không mất; xám: JPEG xám).
    pub nen_den_trang: bool,
}

impl Default for CauHinhXuat {
    fn default() -> Self {
        CauHinhXuat { dpi: 0, chat_luong: 90, nen_den_trang: false }
    }
}

/// Phần chung của 1 thư mục nguồn.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ThuMuc {
    pub luc: u64,
    pub vung: Vec<Vung>,
    pub tuy_chon: TuyChon,
    pub xuat: CauHinhXuat,
}

/// Vùng riêng của 1 PDF.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VungFile {
    pub luc: u64,
    pub vung: Vec<Vung>,
}

/// Lần xoá ghim sau cùng của 1 PDF nguồn.
#[derive(Clone, Debug, PartialEq)]
pub struct NhatKy {
    pub luc: u64,
    /// Id nội dung file kết quả lúc ghi — so với file đang có ở thư mục ra để biết kết quả còn nguyên.
    pub ket_qua: Id,
    /// Băm vùng + tuỳ chọn đã dùng (đổi cấu hình → biết kết quả cũ không còn đúng cấu hình hiện tại).
    pub ma_cau_hinh: u64,
    pub so_trang: u32,
    /// DPI lớn nhất đã vẽ (0 = không trang nào phải vẽ lại).
    pub dpi: u16,
    pub ai: bool,
    /// Trang đã xử lý (từ 0), dạng dải `[đầu, cuối]`.
    pub trang: Vec<(u32, u32)>,
}

/// Gộp danh sách trang tăng dần thành dải liền.
pub fn thanh_dai(trang: &[u32]) -> Vec<(u32, u32)> {
    let mut ra: Vec<(u32, u32)> = Vec::new();
    for &t in trang {
        match ra.last_mut() {
            Some((_, cuoi)) if *cuoi + 1 == t => *cuoi = t,
            _ => ra.push((t, t)),
        }
    }
    ra
}

struct Ghi(Vec<u8>);

impl Ghi {
    fn moi(luc: u64) -> Self {
        let mut b = vec![PHIEN_BAN];
        b.extend(luc.to_le_bytes());
        Ghi(b)
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend(v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend(v.to_le_bytes());
    }
    fn vung(&mut self, ds: &[Vung]) {
        self.u32(ds.len() as u32);
        for v in ds {
            self.u8(ma_vi_tri(v.vi_tri));
            self.u8(match v.loai {
                LoaiVung::Xoa => 0,
                LoaiVung::BaoVe => 1,
                LoaiVung::VoDoi => 2,
            });
            let (kieu, trang) = match v.ap_trang {
                ApTrang::TatCa => (0, 0),
                ApTrang::Le => (1, 0),
                ApTrang::Chan => (2, 0),
                ApTrang::MotTrang(t) => (3, t),
            };
            self.u8(kieu);
            self.u32(trang);
            for x in [v.x, v.y, v.rong, v.cao] {
                self.f64(x);
            }
            self.u32(v.rong_px);
            self.u32(v.cao_px);
            self.u8(v.nguong);
        }
    }
}

struct Doc<'a>(&'a [u8]);

impl Doc<'_> {
    fn lay<const N: usize>(&mut self) -> Option<[u8; N]> {
        let (a, b) = self.0.split_first_chunk::<N>()?;
        self.0 = b;
        Some(*a)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.lay::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.lay()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.lay()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.lay()?))
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.lay()?))
    }
    /// Phiên bản + mốc sửa.
    fn dau(&mut self) -> Option<u64> {
        (self.u8()? == PHIEN_BAN).then_some(())?;
        self.u64()
    }
    fn vung(&mut self) -> Option<Vec<Vung>> {
        let n = self.u32()? as usize;
        if n > TOI_DA_VUNG {
            return None;
        }
        let mut ds = Vec::with_capacity(n);
        for _ in 0..n {
            let vi_tri = vi_tri_tu_ma(self.u8()?)?;
            let loai = match self.u8()? {
                0 => LoaiVung::Xoa,
                1 => LoaiVung::BaoVe,
                2 => LoaiVung::VoDoi,
                _ => return None,
            };
            let (kieu, trang) = (self.u8()?, self.u32()?);
            let ap_trang = match kieu {
                0 => ApTrang::TatCa,
                1 => ApTrang::Le,
                2 => ApTrang::Chan,
                3 => ApTrang::MotTrang(trang),
                _ => return None,
            };
            let v = Vung { vi_tri, loai, ap_trang, x: self.f64()?, y: self.f64()?, rong: self.f64()?, cao: self.f64()?, rong_px: self.u32()?, cao_px: self.u32()?, nguong: self.u8()? };
            hop_le(&v).then_some(())?;
            ds.push(v);
        }
        Some(ds)
    }
    fn het(&self) -> Option<()> {
        self.0.is_empty().then_some(())
    }
}

fn ma_vi_tri(v: ViTri) -> u8 {
    match v {
        ViTri::GocTrenTrai => 0,
        ViTri::GocTrenPhai => 1,
        ViTri::GocDuoiTrai => 2,
        ViTri::GocDuoiPhai => 3,
        ViTri::CanhTrai => 4,
        ViTri::CanhPhai => 5,
        ViTri::CanhTren => 6,
        ViTri::CanhDuoi => 7,
        ViTri::Tay => 8,
    }
}

fn vi_tri_tu_ma(m: u8) -> Option<ViTri> {
    Some(match m {
        0 => ViTri::GocTrenTrai,
        1 => ViTri::GocTrenPhai,
        2 => ViTri::GocDuoiTrai,
        3 => ViTri::GocDuoiPhai,
        4 => ViTri::CanhTrai,
        5 => ViTri::CanhPhai,
        6 => ViTri::CanhTren,
        7 => ViTri::CanhDuoi,
        8 => ViTri::Tay,
        _ => return None,
    })
}

/// Vùng hợp lệ: toạ độ tỉ lệ trong trang, kích thước dương; kích thước điểm ảnh ≤ 10.000 (ở 120 dpi).
pub fn hop_le(v: &Vung) -> bool {
    let ti_le = |x: f64| x.is_finite() && (0.0..=1.0).contains(&x);
    let tran = 1.0 + 1e-6;
    ti_le(v.x) && ti_le(v.y) && ti_le(v.rong) && ti_le(v.cao) && v.x + v.rong <= tran && v.y + v.cao <= tran && (v.rong > 0.0 || v.rong_px > 0) && (v.cao > 0.0 || v.cao_px > 0) && v.rong_px <= 10_000 && v.cao_px <= 10_000
}

impl VungFile {
    pub fn ma_hoa(&self) -> Vec<u8> {
        let mut g = Ghi::moi(self.luc);
        g.vung(&self.vung);
        g.0
    }
    pub fn giai_ma(b: &[u8]) -> Option<Self> {
        let mut d = Doc(b);
        let luc = d.dau()?;
        let vung = d.vung()?;
        d.het()?;
        Some(VungFile { luc, vung })
    }
}

impl ThuMuc {
    pub fn ma_hoa(&self) -> Vec<u8> {
        let mut g = Ghi::moi(self.luc);
        g.vung(&self.vung);
        g.u8(self.tuy_chon.bao_ve_net_den as u8 | (self.tuy_chon.bao_ve_do_xanh as u8) << 1 | (self.tuy_chon.nen_trang as u8) << 2 | (self.xuat.nen_den_trang as u8) << 3);
        g.u16(self.xuat.dpi);
        g.u8(self.xuat.chat_luong);
        g.0
    }
    pub fn giai_ma(b: &[u8]) -> Option<Self> {
        let mut d = Doc(b);
        let luc = d.dau()?;
        let vung = d.vung()?;
        let co = d.u8()?;
        let dpi = d.u16()?;
        let chat_luong = d.u8()?;
        d.het()?;
        (dpi <= 600 && (1..=100).contains(&chat_luong)).then_some(())?;
        Some(ThuMuc {
            luc,
            vung,
            tuy_chon: TuyChon { bao_ve_net_den: co & 1 != 0, bao_ve_do_xanh: co & 2 != 0, nen_trang: co & 4 != 0 },
            xuat: CauHinhXuat { dpi, chat_luong, nen_den_trang: co & 8 != 0 },
        })
    }
}

impl NhatKy {
    pub fn ma_hoa(&self) -> Vec<u8> {
        let mut g = Ghi::moi(self.luc);
        g.u64(self.ket_qua.0);
        g.u64(self.ket_qua.1);
        g.u64(self.ma_cau_hinh);
        g.u32(self.so_trang);
        g.u16(self.dpi);
        g.u8(self.ai as u8);
        g.u32(self.trang.len() as u32);
        for &(a, b) in &self.trang {
            g.u32(a);
            g.u32(b);
        }
        g.0
    }
    pub fn giai_ma(b: &[u8]) -> Option<Self> {
        let mut d = Doc(b);
        let luc = d.dau()?;
        let ket_qua = (d.u64()?, d.u64()?);
        let (ma_cau_hinh, so_trang, dpi, ai) = (d.u64()?, d.u32()?, d.u16()?, d.u8()? != 0);
        let n = d.u32()? as usize;
        // Số dải phải khớp số byte còn lại (không cấp bộ nhớ theo số khai trong dữ liệu lạ)
        (n <= so_trang as usize && n * 8 == d.0.len()).then_some(())?;
        let mut trang = Vec::with_capacity(n);
        for _ in 0..n {
            let (a, b) = (d.u32()?, d.u32()?);
            (a <= b && b < so_trang).then_some(())?;
            trang.push((a, b));
        }
        d.het()?;
        Some(NhatKy { luc, ket_qua, ma_cau_hinh, so_trang, dpi, ai, trang })
    }
}

/// Mốc sửa của 1 nội dung bản ghi Xóa ghim bất kỳ (gộp giữa máy, dọn dữ liệu cũ).
pub fn luc_cua(nd: &[u8]) -> Option<u64> {
    Doc(nd).dau()
}

/// Mã cấu hình: băm vùng áp dụng cho file + tuỳ chọn xử lý + cách ghi.
pub fn ma_cau_hinh(vung: &[Vung], tc: &TuyChon, xuat: &CauHinhXuat) -> u64 {
    let mut g = Ghi::moi(0);
    g.vung(vung);
    g.u8(tc.bao_ve_net_den as u8 | (tc.bao_ve_do_xanh as u8) << 1 | (tc.nen_trang as u8) << 2 | (xuat.nen_den_trang as u8) << 3);
    g.u16(xuat.dpi);
    g.u8(xuat.chat_luong);
    g.0.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// Khoá bản ghi vùng chung của thư mục nguồn.
pub fn khoa_thu_muc(thu_muc: &std::path::Path) -> Id {
    (crate::tach_vb::kho::khoa_duong_dan(thu_muc), 0)
}

pub fn doc_thu_muc(k: &mut Kho, thu_muc: &std::path::Path) -> Option<ThuMuc> {
    ThuMuc::giai_ma(&k.khac(VUNG_THU_MUC, khoa_thu_muc(thu_muc))?)
}

pub fn ghi_thu_muc(k: &mut Kho, thu_muc: &std::path::Path, tm: &ThuMuc) {
    k.ghi_khac(VUNG_THU_MUC, khoa_thu_muc(thu_muc), &tm.ma_hoa());
}

pub fn doc_vung_file(k: &mut Kho, id: Id) -> Option<VungFile> {
    VungFile::giai_ma(&k.khac(VUNG_FILE, id)?)
}

/// Ghi vùng riêng của PDF; không còn vùng nào → bỏ bản ghi.
pub fn ghi_vung_file(k: &mut Kho, id: Id, vf: &VungFile) {
    k.ghi_khac(VUNG_FILE, id, &if vf.vung.is_empty() { Vec::new() } else { vf.ma_hoa() });
}

pub fn doc_nhat_ky(k: &mut Kho, id_nguon: Id) -> Option<NhatKy> {
    NhatKy::giai_ma(&k.khac(NHAT_KY, id_nguon)?)
}

pub fn ghi_nhat_ky(k: &mut Kho, id_nguon: Id, nk: &NhatKy) {
    k.ghi_khac(NHAT_KY, id_nguon, &nk.ma_hoa());
}

/// Bảng id file kết quả → id nguồn theo nhật ký (dựng 1 lần cho cả lượt chạy — để không ghi đè kết quả của nguồn
/// khác cùng tên). Trong 1 lượt các file đích khác nhau nên không cần cập nhật giữa chừng.
pub fn nguon_theo_ket_qua(k: &mut Kho) -> std::collections::HashMap<Id, Id> {
    k.khoa_khac(NHAT_KY).into_iter().filter_map(|id| Some((doc_nhat_ky(k, id)?.ket_qua, id))).collect()
}

/// Kho Xóa ghim của app, mở 1 lần. Mở chỉ đọc khi biết chắc nơi khác đang giữ khoá "app đang chạy"; ổ không khoá
/// được (khoá không xác định) thì vẫn ghi như kho Tách. Không mở được → kho chỉ trong bộ nhớ.
pub fn chung() -> &'static std::sync::Mutex<Kho> {
    static KHO: std::sync::OnceLock<std::sync::Mutex<Kho>> = std::sync::OnceLock::new();
    KHO.get_or_init(|| {
        let f = crate::tach_vb::du_lieu_an::goc_du_lieu().join("kho_xoa_ghim.bin");
        let kho = if crate::tach_vb::kho::giu_khoa_that() == Some(false) { Kho::mo_chi_doc(&f) } else { Kho::mo(&f) };
        std::sync::Mutex::new(kho.unwrap_or_else(|_| Kho::bo_nho()))
    })
}

pub fn kho() -> std::sync::MutexGuard<'static, Kho> {
    chung().lock().unwrap_or_else(|e| e.into_inner())
}

/// Dọn dữ liệu Xóa ghim cũ: giữ vùng / nhật ký của PDF còn sống (`con_song` — lấy từ kho Tách **trước** khi dọn kho Tách)
/// và vùng thư mục sửa từ `truoc` (ms) trở đi.
pub fn don(k: &mut Kho, con_song: &std::collections::HashSet<Id>, truoc: u64) -> std::io::Result<usize> {
    k.don_khac(|loai, id, nd| match loai {
        VUNG_THU_MUC => luc_cua(nd).is_some_and(|l| l >= truoc),
        VUNG_FILE | NHAT_KY => con_song.contains(&id),
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vung_mau() -> Vec<Vung> {
        vec![
            Vung { vi_tri: ViTri::GocTrenTrai, loai: LoaiVung::Xoa, ap_trang: ApTrang::TatCa, x: 0.0, y: 0.0, rong: 0.12, cao: 0.12, rong_px: 130, cao_px: 130, nguong: 3 },
            Vung { vi_tri: ViTri::Tay, loai: LoaiVung::VoDoi, ap_trang: ApTrang::MotTrang(4), x: 0.1, y: 0.2, rong: 0.3, cao: 0.4, rong_px: 0, cao_px: 0, nguong: 7 },
            Vung { vi_tri: ViTri::Tay, loai: LoaiVung::BaoVe, ap_trang: ApTrang::Chan, x: 0.5, y: 0.5, rong: 0.5, cao: 0.5, rong_px: 0, cao_px: 0, nguong: 5 },
        ]
    }

    #[test]
    fn ma_hoa_giai_ma_ca_ba_loai() {
        let tm = ThuMuc { luc: 9, vung: vung_mau(), tuy_chon: TuyChon { bao_ve_net_den: false, bao_ve_do_xanh: true, nen_trang: true }, xuat: CauHinhXuat { dpi: 300, chat_luong: 80, nen_den_trang: true } };
        assert_eq!(ThuMuc::giai_ma(&tm.ma_hoa()), Some(tm.clone()));
        let vf = VungFile { luc: 5, vung: vung_mau() };
        assert_eq!(VungFile::giai_ma(&vf.ma_hoa()), Some(vf));
        let nk = NhatKy { luc: 7, ket_qua: (1, 2), ma_cau_hinh: 3, so_trang: 10, dpi: 300, ai: false, trang: thanh_dai(&[0, 1, 2, 5, 7, 8]) };
        assert_eq!(nk.trang, vec![(0, 2), (5, 5), (7, 8)]);
        assert_eq!(NhatKy::giai_ma(&nk.ma_hoa()), Some(nk.clone()));
        assert_eq!(luc_cua(&nk.ma_hoa()), Some(7));
    }

    #[test]
    fn du_lieu_sai_hoac_ngoai_gioi_han_bi_bo() {
        let vf = VungFile { luc: 1, vung: vung_mau() };
        let b = vf.ma_hoa();
        assert!(VungFile::giai_ma(&b[..b.len() - 1]).is_none(), "cụt");
        let mut thua = b.clone();
        thua.push(0);
        assert!(VungFile::giai_ma(&thua).is_none(), "thừa");
        let mut phien_ban_la = b.clone();
        phien_ban_la[0] = 9;
        assert!(VungFile::giai_ma(&phien_ban_la).is_none());
        // Toạ độ ra ngoài trang / NaN / x1 ≥ x2
        for sai in [Vung { x: 0.9, rong: 0.2, ..vung_mau()[1].clone() }, Vung { y: f64::NAN, ..vung_mau()[1].clone() }, Vung { rong: 0.0, ..vung_mau()[1].clone() }] {
            assert!(VungFile::giai_ma(&VungFile { luc: 1, vung: vec![sai] }.ma_hoa()).is_none());
        }
        let nhieu = VungFile { luc: 1, vung: vec![vung_mau()[0].clone(); TOI_DA_VUNG + 1] };
        assert!(VungFile::giai_ma(&nhieu.ma_hoa()).is_none());
        let nk = NhatKy { luc: 1, ket_qua: (1, 1), ma_cau_hinh: 0, so_trang: 3, dpi: 0, ai: false, trang: vec![(2, 3)] };
        assert!(NhatKy::giai_ma(&nk.ma_hoa()).is_none(), "trang vượt số trang");
        // Số dải khai rất lớn (gói lạ) → bỏ, không cấp bộ nhớ theo số khai
        let mut b = NhatKy { luc: 1, ket_qua: (1, 1), ma_cau_hinh: 0, so_trang: u32::MAX, dpi: 0, ai: false, trang: vec![] }.ma_hoa();
        let n = b.len();
        b[n - 4..].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(NhatKy::giai_ma(&b).is_none());
        let tm = ThuMuc { xuat: CauHinhXuat { chat_luong: 0, ..Default::default() }, ..Default::default() };
        assert!(ThuMuc::giai_ma(&tm.ma_hoa()).is_none());
    }

    #[test]
    fn kho_ghi_doc_tim_nguon_cua_ket_qua_va_don() {
        let d = std::env::temp_dir().join(format!("dema4_kho_xg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut k = Kho::mo(&d.join("kho_xoa_ghim.bin")).unwrap();
        let thu_muc = d.join("nguon");
        ghi_thu_muc(&mut k, &thu_muc, &ThuMuc { luc: 100, vung: vung_mau(), ..Default::default() });
        ghi_vung_file(&mut k, (1, 1), &VungFile { luc: 1, vung: vung_mau() });
        ghi_vung_file(&mut k, (2, 2), &VungFile { luc: 1, vung: vung_mau() });
        ghi_vung_file(&mut k, (2, 2), &VungFile { luc: 2, vung: vec![] });
        let nk = NhatKy { luc: 3, ket_qua: (9, 9), ma_cau_hinh: 0, so_trang: 1, dpi: 0, ai: false, trang: vec![] };
        ghi_nhat_ky(&mut k, (1, 1), &nk);
        k.day().unwrap();
        assert_eq!(doc_thu_muc(&mut k, &thu_muc).unwrap().vung.len(), 3);
        assert!(doc_vung_file(&mut k, (2, 2)).is_none(), "hết vùng = bỏ bản ghi");
        assert_eq!(nguon_theo_ket_qua(&mut k), [((9, 9), (1, 1))].into_iter().collect());
        // (1, 1) không còn sống; thư mục sửa trước mốc dọn
        assert_eq!(don(&mut k, &Default::default(), 50).unwrap(), 2, "bỏ vùng + nhật ký của PDF không còn");
        assert!(doc_thu_muc(&mut k, &thu_muc).is_some());
        assert_eq!(don(&mut k, &Default::default(), 200).unwrap(), 1, "thư mục sửa trước mốc");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Đo: 100.000 PDF, mỗi PDF 1 nhật ký + 1 bộ vùng riêng 3 vùng — dung lượng kho và thời gian mở.
    #[test]
    #[ignore]
    fn do_kho_100000_pdf() {
        let d = std::env::temp_dir().join(format!("dema4_do_kho_xg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let f = d.join("kho_xoa_ghim.bin");
        let mut k = Kho::mo(&f).unwrap();
        for i in 0..100_000u64 {
            ghi_nhat_ky(&mut k, (i, i), &NhatKy { luc: i, ket_qua: (i, 1), ma_cau_hinh: i, so_trang: 20, dpi: 300, ai: false, trang: vec![(0, 19)] });
            ghi_vung_file(&mut k, (i, i), &VungFile { luc: i, vung: vung_mau() });
        }
        k.day().unwrap();
        drop(k);
        let t = std::time::Instant::now();
        let k = Kho::mo(&f).unwrap();
        println!("DO|kho_xg_100000|{} KB, mở {:.2} s, {} bản ghi", std::fs::metadata(&f).unwrap().len() / 1024, t.elapsed().as_secs_f64(), k.so_ban_ghi);
        let _ = std::fs::remove_dir_all(&d);
    }
}
