//! Kho dữ liệu Tách **1 file cho cả máy** (`<gốc dữ liệu>/kho_tach.bin`), khoá theo **id nội dung PDF**
//! (mã kiểm tra 64 KB đầu + cuối, kích thước) — đổi tên / di chuyển / mở thư mục cha-con vẫn thấy phần đã làm.
//!
//! - Ghi = **nối thêm** 1 bản ghi vào cuối file (không bao giờ sửa bản ghi cũ); bản ghi sau cùng của 1 khoá thắng.
//! - Bản ghi: dấu mở đầu (4 byte) · loại · độ dài · nội dung · CRC32. Mở: bản ghi hỏng (CRC sai, ghi dở khi mất điện,
//!   hỏng giữa file) → bỏ đúng bản ghi đó, tìm dấu mở đầu kế tiếp, phần sau vẫn đọc.
//! - Bộ nhớ chỉ giữ **chỉ mục** (khoá → vị trí trong file) + bảng đường dẫn → id; nội dung đọc theo vị trí khi cần.
//! - `ghi_ngay` (nhãn người sửa): đẩy xuống đĩa ngay. `ghi_dem` (phân tích): gom, đẩy khi `day` (~1 s / khi đóng).
//!   Ghi lỗi (ổ đầy…) → giữ trong bộ nhớ, thử lại lần sau; không cắt mất dữ liệu đã có.
//! - `nen`: viết lại chỉ bản ghi còn dùng (file tạm → đổi tên), bản trước giữ `.bak`.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::cache_trang::{self, KhoaFile, TrangCache};
use super::nhan::{Nhan, Xoay};

const DAU_FILE: &[u8; 8] = b"SMKHO\x00\x01\n";
const DAU_BAN_GHI: [u8; 4] = [0xD5, 0x4B, 0x48, 0x4F];
/// Dấu (4) + loại (1) + độ dài (4).
const DAU_DAI: u64 = 9;
const TOI_DA_NOI_DUNG: u32 = 64 << 20;

const PHAN_TICH: u8 = 1;
const TACH: u8 = 2;
const DUONG_DAN: u8 = 3;
/// Con trỏ kết quả Bóc tách: id → file `boc_tach.jsonl` đang giữ kết quả của PDF đó.
const BOC: u8 = 4;

/// id nội dung PDF: (mã kiểm tra, kích thước).
pub type Id = (u64, u64);

/// Phần Tách của 1 PDF: nhãn, góc xoay, cờ duyệt từng trang + mốc sửa (luật "sau cùng thắng" khi gộp giữa máy).
/// Không còn nhãn nào = bia mộ (Phân tích lại / xoá hết nhãn) — vẫn mang mốc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BanTach {
    pub so_trang: u32,
    pub nhan: BTreeMap<usize, Nhan>,
    pub xoay: BTreeMap<usize, Xoay>,
    pub duyet: BTreeMap<usize, u8>,
    pub luc: u64,
    pub chu_ky: u64,
    /// Bia mộ của Phân tích lại: lần mở sau dùng nhãn tự động (khác "người bỏ hết nhãn" — giữ đúng như vậy).
    pub bia_mo: bool,
}

/// Chữ ký phần Tách của 1 hồ sơ (cùng cách viết với bản trước: từng trang tăng dần `n<trang>:<nhãn>;` ·
/// `x<trang>:<góc>:<xác nhận>;` · `d<trang>:<cờ>;`). Không còn gì → `CHU_KY_RONG`.
pub fn chu_ky_tach(nhan: &BTreeMap<usize, Nhan>, xoay: &BTreeMap<usize, Xoay>, duyet: &BTreeMap<usize, u8>) -> u64 {
    use std::fmt::Write;
    let trang: std::collections::BTreeSet<usize> = nhan.keys().chain(xoay.keys()).chain(duyet.keys()).copied().collect();
    let mut s = String::new();
    for t in trang {
        if let Some(c) = nhan.get(&t).and_then(|n| n.chu()) {
            let _ = write!(s, "n{t}:{c};");
        }
        if let Some(x) = xoay.get(&t).filter(|x| x.goc != 0 || x.xac_nhan) {
            let _ = write!(s, "x{t}:{}:{};", x.goc, x.xac_nhan as u8);
        }
        if let Some(d) = duyet.get(&t).filter(|d| **d != 0) {
            let _ = write!(s, "d{t}:{d};");
        }
    }
    super::moc_sua::chu_ky(&s)
}

fn ma_nhan(n: Nhan) -> u8 {
    match n {
        Nhan::NoiDung => 0,
        Nhan::Index => 1,
        Nhan::Trang => 2,
        Nhan::Split => 3,
    }
}

fn nhan_tu_ma(m: u8) -> Option<Nhan> {
    Some(match m {
        0 => Nhan::NoiDung,
        1 => Nhan::Index,
        2 => Nhan::Trang,
        3 => Nhan::Split,
        _ => return None,
    })
}

struct Doc<'a>(&'a [u8]);

impl Doc<'_> {
    fn lay(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.lay(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.lay(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.lay(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.lay(8)?.try_into().ok()?))
    }
}

impl BanTach {
    pub(crate) fn ma_hoa(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(32 + 8 * (self.nhan.len() + self.xoay.len() + self.duyet.len()));
        b.extend(self.so_trang.to_le_bytes());
        b.extend(self.luc.to_le_bytes());
        b.extend(self.chu_ky.to_le_bytes());
        b.push(self.bia_mo as u8);
        b.extend((self.nhan.len() as u32).to_le_bytes());
        for (t, n) in &self.nhan {
            b.extend((*t as u32).to_le_bytes());
            b.push(ma_nhan(*n));
        }
        b.extend((self.xoay.len() as u32).to_le_bytes());
        for (t, x) in &self.xoay {
            b.extend((*t as u32).to_le_bytes());
            b.extend(x.goc.to_le_bytes());
            b.push(x.xac_nhan as u8);
        }
        b.extend((self.duyet.len() as u32).to_le_bytes());
        for (t, d) in &self.duyet {
            b.extend((*t as u32).to_le_bytes());
            b.push(*d);
        }
        b
    }

    pub(crate) fn giai_ma(b: &[u8]) -> Option<Self> {
        let mut d = Doc(b);
        let mut bt = BanTach { so_trang: d.u32()?, luc: d.u64()?, chu_ky: d.u64()?, bia_mo: d.u8()? != 0, ..Default::default() };
        for _ in 0..d.u32()? {
            let t = d.u32()? as usize;
            bt.nhan.insert(t, nhan_tu_ma(d.u8()?)?);
        }
        for _ in 0..d.u32()? {
            let t = d.u32()? as usize;
            bt.xoay.insert(t, Xoay { goc: d.u16()?, xac_nhan: d.u8()? != 0 });
        }
        for _ in 0..d.u32()? {
            let t = d.u32()? as usize;
            bt.duyet.insert(t, d.u8()?);
        }
        d.0.is_empty().then_some(bt)
    }
}

/// Dòng bảng đường dẫn → id: PDF ở đường dẫn này có kích thước + thời điểm sửa (giây) như trên thì id như trên.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DauFile {
    pub kich_thuoc: u64,
    pub sua_giay: u64,
    pub id: Id,
    /// Lần cuối thấy file (ms) — dọn dữ liệu PDF lâu không thấy.
    pub lan_cuoi: u64,
}

/// Khoá trong bảng đường dẫn: băm FNV-1a 64 của đường dẫn tuyệt đối chuẩn (NFC; Windows chữ thường).
pub fn khoa_duong_dan(pdf: &Path) -> u64 {
    let p = std::path::absolute(pdf).unwrap_or_else(|_| pdf.to_path_buf());
    let s = super::nhan::nfc(&p.to_string_lossy());
    let s = if cfg!(windows) { s.to_lowercase() } else { s };
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

fn bay_gio_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Vị trí nội dung 2 bản ghi còn dùng của 1 PDF trong file (độ dài 0 = không có).
#[derive(Debug, Clone, Copy, Default)]
struct ViTri {
    phan_tich: u64,
    tach: u64,
    phan_tich_dai: u32,
    tach_dai: u32,
}

impl ViTri {
    fn lay(&self, loai: u8) -> Option<(u64, u32)> {
        let (v, n) = if loai == PHAN_TICH { (self.phan_tich, self.phan_tich_dai) } else { (self.tach, self.tach_dai) };
        (n > 0).then_some((v, n))
    }

    fn dat(&mut self, loai: u8, v: u64, n: u32) {
        if loai == PHAN_TICH {
            (self.phan_tich, self.phan_tich_dai) = (v, n);
        } else {
            (self.tach, self.tach_dai) = (v, n);
        }
    }

    fn so_ban_ghi(&self) -> usize {
        (self.phan_tich_dai > 0) as usize + (self.tach_dai > 0) as usize
    }
}

pub struct Kho {
    /// `None` = chỉ trong bộ nhớ (không mở được file — gốc dữ liệu không ghi được).
    file: Option<PathBuf>,
    doc: Option<std::fs::File>,
    /// id → vị trí bản ghi phân tích + Tách trong file (1 dòng / PDF — 1.000.000 PDF ~50 MB).
    chi_muc: HashMap<Id, ViTri>,
    duong_dan: HashMap<u64, DauFile>,
    boc: HashMap<Id, String>,
    /// Bản ghi chờ ghi xuống đĩa + nội dung của chúng (đọc được ngay trước khi ghi).
    dem: Vec<u8>,
    cho: HashMap<(u8, Id), Vec<u8>>,
    /// Độ dài phần file đã ghi chắc chắn (bản ghi kế tiếp bắt đầu ở đây).
    cuoi: u64,
    pub so_ban_ghi: usize,
    /// Số bản ghi hỏng đã bỏ qua lúc mở.
    pub so_hong: usize,
}

impl Kho {
    /// Kho chỉ trong bộ nhớ (không ghi file).
    pub fn bo_nho() -> Self {
        Kho { file: None, doc: None, chi_muc: HashMap::new(), duong_dan: HashMap::new(), boc: HashMap::new(), dem: Vec::new(), cho: HashMap::new(), cuoi: DAU_FILE.len() as u64, so_ban_ghi: 0, so_hong: 0 }
    }

    pub fn mo(file: &Path) -> std::io::Result<Self> {
        let mut kho = Kho { file: Some(file.to_path_buf()), ..Kho::bo_nho() };
        if let Some(cha) = file.parent() {
            std::fs::create_dir_all(cha)?;
        }
        // Kho mất (sự cố giữa lúc dọn của bản trước) mà còn bản sao → dùng bản sao
        let bak = file.with_extension("bak");
        if !file.exists() && bak.exists() {
            std::fs::copy(&bak, file)?;
        }
        let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(file)?;
        let dai = f.metadata()?.len();
        let mut r = BufReader::with_capacity(1 << 20, &f);
        let mut dau = [0u8; 8];
        let co_dau = dai >= 8 && r.read_exact(&mut dau).is_ok() && &dau == DAU_FILE;
        let mut tot = DAU_FILE.len() as u64;
        if co_dau {
            let mut pos = tot;
            let mut nd = Vec::new();
            while pos + DAU_DAI + 4 <= dai {
                match doc_ban_ghi(&mut r, pos, dai, &mut nd) {
                    Some((loai, n)) => {
                        kho.nap(loai, &nd, pos + DAU_DAI, n);
                        pos += DAU_DAI + n as u64 + 4;
                        tot = pos;
                    }
                    None => {
                        // Bản ghi hỏng: tìm dấu mở đầu kế tiếp rồi đọc tiếp từ đó
                        kho.so_hong += 1;
                        match tim_dau(&mut r, pos + 1, dai) {
                            Some(p) if r.seek(SeekFrom::Start(p)).is_ok() => pos = p,
                            _ => break,
                        }
                    }
                }
            }
        }
        drop(r);
        if !co_dau {
            // File mới / không phải kho → bắt đầu lại (file lạ được giữ ở .hong để xem)
            if dai > 0 {
                let _ = std::fs::copy(file, file.with_extension("hong"));
            }
            f.set_len(0)?;
            (&f).write_all(DAU_FILE)?;
            f.sync_data()?;
        } else if tot < dai {
            // Đuôi ghi dở (mất điện) → cắt; phần hỏng ở giữa (có bản ghi tốt phía sau) không cắt
            f.set_len(tot)?;
        }
        kho.cuoi = tot;
        kho.doc = Some(f);
        Ok(kho)
    }

    fn nap(&mut self, loai: u8, nd: &[u8], vi_tri: u64, n: u32) {
        if nd.len() < 16 {
            return;
        }
        let id = (u64::from_le_bytes(nd[..8].try_into().unwrap()), u64::from_le_bytes(nd[8..16].try_into().unwrap()));
        self.so_ban_ghi += 1;
        match loai {
            PHAN_TICH | TACH => self.dat_vi_tri(loai, id, vi_tri, if nd.len() == 16 { 0 } else { n }),
            BOC => {
                if nd.len() == 16 {
                    self.boc.remove(&id);
                } else {
                    self.boc.insert(id, String::from_utf8_lossy(&nd[16..]).into_owned());
                }
            }
            DUONG_DAN => {
                let mut d = Doc(&nd[16..]);
                match (d.u64(), d.u64(), d.u64(), d.u64(), d.u64()) {
                    (Some(kich_thuoc), Some(sua_giay), Some(a), Some(b), Some(lan_cuoi)) => {
                        self.duong_dan.insert(id.0, DauFile { kich_thuoc, sua_giay, id: (a, b), lan_cuoi });
                    }
                    _ => {
                        self.duong_dan.remove(&id.0);
                    }
                }
            }
            _ => {}
        }
    }

    /// Độ dài 0 = bỏ bản ghi loại đó của PDF.
    fn dat_vi_tri(&mut self, loai: u8, id: Id, vi_tri: u64, n: u32) {
        let v = self.chi_muc.entry(id).or_default();
        v.dat(loai, vi_tri, n);
        if v.so_ban_ghi() == 0 {
            self.chi_muc.remove(&id);
        }
    }

    fn them(&mut self, loai: u8, id: Id, du_lieu: &[u8]) {
        let mut nd = Vec::with_capacity(16 + du_lieu.len());
        nd.extend(id.0.to_le_bytes());
        nd.extend(id.1.to_le_bytes());
        nd.extend(du_lieu);
        let vi_tri = self.cuoi + self.dem.len() as u64 + DAU_DAI;
        self.dem.extend(DAU_BAN_GHI);
        self.dem.push(loai);
        self.dem.extend((nd.len() as u32).to_le_bytes());
        self.dem.extend(&nd);
        let mut crc = crc32fast::Hasher::new();
        crc.update(&[loai]);
        crc.update(&nd);
        self.dem.extend(crc.finalize().to_le_bytes());
        self.so_ban_ghi += 1;
        if loai == DUONG_DAN || loai == BOC {
            return;
        }
        if du_lieu.is_empty() {
            self.dat_vi_tri(loai, id, 0, 0);
            self.cho.remove(&(loai, id));
        } else {
            self.dat_vi_tri(loai, id, vi_tri, nd.len() as u32);
            self.cho.insert((loai, id), nd);
        }
    }

    /// Ghi phần đang chờ xuống đĩa. Lỗi (ổ đầy, file bị khoá) → giữ phần chờ, trả lỗi; phần đã ghi dở bị cắt bỏ.
    pub fn day(&mut self) -> std::io::Result<()> {
        if self.dem.is_empty() {
            return Ok(());
        }
        if self.file.is_none() {
            return Err(std::io::Error::other("không mở được kho dữ liệu (thư mục dữ liệu không ghi được) — phần vừa làm chưa được lưu"));
        }
        let f = self.doc.as_mut().ok_or_else(|| std::io::Error::other("kho chưa mở"))?;
        let kq = f.seek(SeekFrom::Start(self.cuoi)).and_then(|_| f.write_all(&self.dem)).and_then(|_| f.sync_data());
        match kq {
            Ok(()) => {
                self.cuoi += self.dem.len() as u64;
                self.dem.clear();
                self.cho.clear();
                Ok(())
            }
            Err(e) => {
                let _ = f.set_len(self.cuoi);
                Err(e)
            }
        }
    }

    fn doc_nd(&mut self, loai: u8, id: Id) -> Option<Vec<u8>> {
        if let Some(nd) = self.cho.get(&(loai, id)) {
            return Some(nd[16..].to_vec());
        }
        let (vi_tri, n) = self.chi_muc.get(&id)?.lay(loai)?;
        let f = self.doc.as_mut()?;
        let mut b = vec![0u8; n as usize];
        f.seek(SeekFrom::Start(vi_tri)).ok()?;
        f.read_exact(&mut b).ok()?;
        Some(b.split_off(16))
    }

    pub fn phan_tich(&mut self, id: Id) -> Option<Vec<TrangCache>> {
        cache_trang::giai_ma(&self.doc_nd(PHAN_TICH, id)?).map(|(_, t)| t)
    }

    pub fn tach(&mut self, id: Id) -> Option<BanTach> {
        BanTach::giai_ma(&self.doc_nd(TACH, id)?)
    }

    pub fn co_phan_tich(&self, id: Id) -> bool {
        self.chi_muc.get(&id).is_some_and(|v| v.phan_tich_dai > 0)
    }

    /// Kết quả phân tích (gom, ghi khi `day`).
    pub fn ghi_phan_tich(&mut self, id: Id, trang: &[TrangCache]) {
        let k = KhoaFile { duong_dan: String::new(), kich_thuoc: id.1, sua_luc: 0, van_tay: id.0 };
        self.them(PHAN_TICH, id, &cache_trang::ma_hoa(&k, trang));
    }

    /// Bỏ kết quả phân tích (Xoá cache / Phân tích lại).
    pub fn bo_phan_tich(&mut self, id: Id) {
        if self.co_phan_tich(id) {
            self.them(PHAN_TICH, id, &[]);
        }
    }

    /// Thêm phần Tách (chưa đẩy xuống đĩa — nơi gọi `day` sau 1 lượt).
    pub fn them_tach(&mut self, id: Id, bt: &BanTach) {
        self.them(TACH, id, &bt.ma_hoa());
    }

    /// Phần Tách do người sửa: ghi xuống đĩa **ngay**.
    pub fn ghi_tach(&mut self, id: Id, bt: &BanTach) -> std::io::Result<()> {
        self.them(TACH, id, &bt.ma_hoa());
        self.day()
    }

    pub fn dau_file(&self, pdf: &Path) -> Option<DauFile> {
        self.duong_dan.get(&khoa_duong_dan(pdf)).copied()
    }

    fn ghi_duong_dan(&mut self, khoa: u64, d: DauFile) {
        let mut b = Vec::with_capacity(40);
        for x in [d.kich_thuoc, d.sua_giay, d.id.0, d.id.1, d.lan_cuoi] {
            b.extend(x.to_le_bytes());
        }
        self.them(DUONG_DAN, (khoa, 0), &b);
        self.duong_dan.insert(khoa, d);
    }

    /// Id của PDF: theo bảng đường dẫn khi kích thước + thời điểm sửa (giây) không đổi — không đọc PDF; không thì tính
    /// mã nội dung (đọc 64 KB đầu + cuối) và ghi nhớ. Ghi nhớ gom, ghi khi `day`.
    pub fn id_cua(&mut self, pdf: &Path) -> Option<Id> {
        let md = std::fs::metadata(pdf).ok()?;
        let sua_giay = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        let khoa = khoa_duong_dan(pdf);
        let bay_gio = bay_gio_ms();
        if let Some(d) = self.duong_dan.get(&khoa).copied() {
            if d.kich_thuoc == md.len() && d.sua_giay == sua_giay {
                // Lần cuối thấy: cập nhật thưa (mỗi ngày 1 lần) để kho không phình
                if bay_gio.saturating_sub(d.lan_cuoi) > 86_400_000 {
                    self.ghi_duong_dan(khoa, DauFile { lan_cuoi: bay_gio, ..d });
                }
                return Some(d.id);
            }
        }
        let id = (cache_trang::van_tay_file(pdf, md.len()).ok()?, md.len());
        self.ghi_duong_dan(khoa, DauFile { kich_thuoc: md.len(), sua_giay, id, lan_cuoi: bay_gio });
        Some(id)
    }

    /// Id thật (luôn đọc nội dung): trước Tách / Xuất phiên — file bị thay mà giữ nguyên thời điểm sửa vẫn phát hiện.
    pub fn id_that(&mut self, pdf: &Path) -> Option<Id> {
        let md = std::fs::metadata(pdf).ok()?;
        let sua_giay = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        let id = (cache_trang::van_tay_file(pdf, md.len()).ok()?, md.len());
        let khoa = khoa_duong_dan(pdf);
        if self.duong_dan.get(&khoa).map(|d| d.id) != Some(id) {
            self.ghi_duong_dan(khoa, DauFile { kich_thuoc: md.len(), sua_giay, id, lan_cuoi: bay_gio_ms() });
        }
        Some(id)
    }

    /// Nên dọn: bản ghi trong file nhiều hơn 2 lần khoá còn dùng.
    pub fn nen_don(&self) -> bool {
        self.so_ban_ghi > 1000 && self.so_ban_ghi > 2 * self.so_ban_ghi_song()
    }

    fn so_ban_ghi_song(&self) -> usize {
        self.chi_muc.values().map(ViTri::so_ban_ghi).sum::<usize>() + self.duong_dan.len() + self.boc.len()
    }

    /// Viết lại kho chỉ còn bản ghi mới nhất của mỗi khoá: file tạm → kho cũ thành `.bak` → file tạm thành kho.
    /// Lỗi bất kỳ bước nào → kho cũ vẫn dùng được.
    pub fn nen(&mut self) -> std::io::Result<()> {
        self.day()?;
        let Some(file) = self.file.clone() else { return Ok(()) };
        let tam = file.with_extension("tmp");
        let mut chi_muc_moi: HashMap<Id, ViTri> = HashMap::with_capacity(self.chi_muc.len());
        {
            let mut w = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&tam)?);
            w.write_all(DAU_FILE)?;
            let mut pos = DAU_FILE.len() as u64;
            let mut viet = |w: &mut std::io::BufWriter<std::fs::File>, loai: u8, nd: &[u8]| -> std::io::Result<u64> {
                w.write_all(&DAU_BAN_GHI)?;
                w.write_all(&[loai])?;
                w.write_all(&(nd.len() as u32).to_le_bytes())?;
                w.write_all(nd)?;
                let mut crc = crc32fast::Hasher::new();
                crc.update(&[loai]);
                crc.update(nd);
                w.write_all(&crc.finalize().to_le_bytes())?;
                let vi_tri = pos + DAU_DAI;
                pos += DAU_DAI + nd.len() as u64 + 4;
                Ok(vi_tri)
            };
            let mut ds: Vec<(Id, ViTri)> = self.chi_muc.iter().map(|(k, v)| (*k, *v)).collect();
            ds.sort_unstable_by_key(|(_, v)| v.phan_tich.min(v.tach));
            for (id, v) in ds {
                for loai in [PHAN_TICH, TACH] {
                    let Some((vi_tri, n)) = v.lay(loai) else { continue };
                    let mut nd = vec![0u8; n as usize];
                    let f = self.doc.as_mut().ok_or_else(|| std::io::Error::other("kho chưa mở"))?;
                    f.seek(SeekFrom::Start(vi_tri))?;
                    f.read_exact(&mut nd)?;
                    let moi = viet(&mut w, loai, &nd)?;
                    chi_muc_moi.entry(id).or_default().dat(loai, moi, n);
                }
            }
            for (khoa, d) in &self.duong_dan {
                let mut nd = Vec::with_capacity(56);
                for x in [*khoa, 0, d.kich_thuoc, d.sua_giay, d.id.0, d.id.1, d.lan_cuoi] {
                    nd.extend(x.to_le_bytes());
                }
                viet(&mut w, DUONG_DAN, &nd)?;
            }
            for (id, p) in &self.boc {
                let mut nd = Vec::with_capacity(16 + p.len());
                nd.extend(id.0.to_le_bytes());
                nd.extend(id.1.to_le_bytes());
                nd.extend(p.as_bytes());
                viet(&mut w, BOC, &nd)?;
            }
            w.flush()?;
            w.get_ref().sync_all()?;
        }
        // Bản sao kho cũ (.bak) rồi đổi tên file tạm đè lên kho (thay thế nguyên tử): mất điện lúc nào kho cũng còn
        let mo_lai = |p: &Path| std::fs::OpenOptions::new().read(true).write(true).open(p);
        if let Err(e) = std::fs::copy(&file, file.with_extension("bak")) {
            let _ = std::fs::remove_file(&tam);
            return Err(e);
        }
        self.doc = None;
        if let Err(e) = std::fs::rename(&tam, &file) {
            let _ = std::fs::remove_file(&tam);
            self.doc = Some(mo_lai(&file)?);
            return Err(e);
        }
        self.doc = Some(mo_lai(&file)?);
        self.cuoi = std::fs::metadata(&file)?.len();
        self.chi_muc = chi_muc_moi;
        self.so_ban_ghi = self.so_ban_ghi_song();
        Ok(())
    }

    /// File kết quả Bóc tách đang giữ kết quả của PDF `id`.
    pub fn boc(&self, id: Id) -> Option<PathBuf> {
        self.boc.get(&id).map(PathBuf::from)
    }

    /// Ghi nhớ file kết quả Bóc tách của PDF `id` (gom, ghi khi `day`).
    pub fn ghi_boc(&mut self, id: Id, jsonl: &Path) {
        let p = jsonl.to_string_lossy().into_owned();
        if self.boc.get(&id) != Some(&p) {
            self.them(BOC, id, p.as_bytes());
            self.boc.insert(id, p);
        }
    }

    /// Dọn dữ liệu của PDF không thấy lại từ trước `truoc` (ms): bỏ dòng đường dẫn cũ, rồi bỏ phân tích / Tách / con trỏ
    /// Bóc tách của id không còn đường dẫn nào trỏ tới; viết lại kho. Trả số PDF đã bỏ.
    pub fn don(&mut self, truoc: u64) -> std::io::Result<usize> {
        let cu: Vec<u64> = self.duong_dan.iter().filter(|(_, d)| d.lan_cuoi < truoc).map(|(k, _)| *k).collect();
        for k in cu {
            self.them(DUONG_DAN, (k, 0), &[]);
            self.duong_dan.remove(&k);
        }
        let con: std::collections::HashSet<Id> = self.duong_dan.values().map(|d| d.id).collect();
        let bo: Vec<Id> = self.chi_muc.keys().chain(self.boc.keys()).copied().filter(|id| !con.contains(id)).collect::<std::collections::HashSet<_>>().into_iter().collect();
        for &id in &bo {
            self.them(PHAN_TICH, id, &[]);
            self.them(TACH, id, &[]);
            if self.boc.remove(&id).is_some() {
                self.them(BOC, id, &[]);
            }
        }
        self.nen()?;
        Ok(bo.len())
    }

    /// Dung lượng file kho (byte).
    pub fn dung_luong(&self) -> u64 {
        self.cuoi + self.dem.len() as u64
    }

    /// Số PDF có dữ liệu (phân tích hoặc Tách).
    pub fn so_khoa(&self) -> usize {
        self.chi_muc.len()
    }
}

/// Đọc 1 bản ghi ở `pos` (đã kiểm dấu, độ dài, CRC). Trả (loại, độ dài nội dung); `nd` = nội dung.
/// Đọc tuần tự từ vị trí hiện tại của `r` (= `pos`) — không nhảy vị trí (nhảy làm mất bộ đệm đọc, mở kho rất chậm).
fn doc_ban_ghi(r: &mut BufReader<&std::fs::File>, pos: u64, dai: u64, nd: &mut Vec<u8>) -> Option<(u8, u32)> {
    let mut dau = [0u8; 9];
    r.read_exact(&mut dau).ok()?;
    if dau[..4] != DAU_BAN_GHI {
        return None;
    }
    let loai = dau[4];
    let n = u32::from_le_bytes(dau[5..9].try_into().unwrap());
    if n > TOI_DA_NOI_DUNG || pos + DAU_DAI + n as u64 + 4 > dai {
        return None;
    }
    nd.resize(n as usize, 0);
    r.read_exact(nd).ok()?;
    let mut crc = [0u8; 4];
    r.read_exact(&mut crc).ok()?;
    let mut h = crc32fast::Hasher::new();
    h.update(&[loai]);
    h.update(nd);
    (h.finalize() == u32::from_le_bytes(crc)).then_some((loai, n))
}

/// Vị trí dấu mở đầu bản ghi kế tiếp từ `tu`.
fn tim_dau(r: &mut BufReader<&std::fs::File>, tu: u64, dai: u64) -> Option<u64> {
    r.seek(SeekFrom::Start(tu)).ok()?;
    let mut khop = 0usize;
    let mut pos = tu;
    let mut b = [0u8; 1];
    while pos < dai {
        r.read_exact(&mut b).ok()?;
        pos += 1;
        khop = if b[0] == DAU_BAN_GHI[khop] { khop + 1 } else if b[0] == DAU_BAN_GHI[0] { 1 } else { 0 };
        if khop == 4 {
            return Some(pos - 4);
        }
    }
    None
}

impl Kho {
    /// Id đã biết của PDF (kích thước + thời điểm sửa khớp bảng đường dẫn) — không đọc PDF.
    pub fn id_da_biet(&self, pdf: &Path) -> Option<Id> {
        let md = std::fs::metadata(pdf).ok()?;
        let sua_giay = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        self.duong_dan.get(&khoa_duong_dan(pdf)).filter(|d| d.kich_thuoc == md.len() && d.sua_giay == sua_giay).map(|d| d.id)
    }
}

/// Id của nhiều PDF: tra bảng đường dẫn (giữ khoá kho ngắn), PDF chưa biết thì tính mã nội dung **song song ngoài khoá**
/// rồi ghi nhớ 1 lượt. File không đọc được → `None`.
pub fn ids_cua(pdf: &[PathBuf]) -> Vec<Option<Id>> {
    ids_cua_trong(chung(), pdf)
}

/// Như `ids_cua` với kho `kho` (test: giả lập 2 máy, mỗi máy 1 kho).
pub fn ids_cua_trong(kho: &std::sync::Mutex<Kho>, pdf: &[PathBuf]) -> Vec<Option<Id>> {
    let kho = || kho.lock().unwrap_or_else(|e| e.into_inner());
    use rayon::prelude::*;
    let md: Vec<Option<(u64, u64)>> = pdf
        .par_iter()
        .map(|p| {
            let m = std::fs::metadata(p).ok()?;
            Some((m.len(), m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs()))
        })
        .collect();
    let khoa: Vec<u64> = pdf.par_iter().map(|p| khoa_duong_dan(p)).collect();
    let bay_gio = bay_gio_ms();
    let mut ids: Vec<Option<Id>> = {
        let mut k = kho();
        let ids: Vec<Option<Id>> = (0..pdf.len()).map(|i| md[i].and_then(|(n, s)| k.duong_dan.get(&khoa[i]).filter(|d| d.kich_thuoc == n && d.sua_giay == s).map(|d| d.id))).collect();
        // Lần cuối thấy file (Dọn dữ liệu cũ dựa vào đây): cập nhật thưa — mỗi ngày 1 lần / file
        let cu: Vec<usize> = (0..pdf.len()).filter(|&i| ids[i].is_some() && k.duong_dan.get(&khoa[i]).is_some_and(|d| bay_gio.saturating_sub(d.lan_cuoi) > 86_400_000)).collect();
        for i in cu {
            let d = k.duong_dan[&khoa[i]];
            k.ghi_duong_dan(khoa[i], DauFile { lan_cuoi: bay_gio, ..d });
        }
        let _ = k.day();
        ids
    };
    let moi: Vec<(usize, Id)> = (0..pdf.len())
        .into_par_iter()
        .filter(|&i| ids[i].is_none())
        .filter_map(|i| {
            let (n, _) = md[i]?;
            Some((i, (cache_trang::van_tay_file(&pdf[i], n).ok()?, n)))
        })
        .collect();
    if !moi.is_empty() {
        let mut k = kho();
        for (i, id) in moi {
            let (n, s) = md[i].unwrap();
            k.ghi_duong_dan(khoa[i], DauFile { kich_thuoc: n, sua_giay: s, id, lan_cuoi: bay_gio });
            ids[i] = Some(id);
        }
        let _ = k.day();
    }
    ids
}

/// Phân tích lại: bỏ kết quả phân tích + bia mộ phần Tách (mốc `luc`) cho các PDF — cùng PDF ở chỗ khác cũng mất
/// (dùng chung theo nội dung).
pub fn lam_lai(ids: &[Id], luc: u64) -> std::io::Result<()> {
    let mut k = kho();
    for &id in ids {
        k.bo_phan_tich(id);
        let cu = k.tach(id);
        let so_trang = cu.as_ref().map_or(0, |b| b.so_trang);
        if cu.is_some_and(|b| b.bia_mo) {
            continue;
        }
        let luc = cu_luc(&mut k, id).map_or(luc, |c| luc.max(c + 1));
        k.them(TACH, id, &BanTach { so_trang, luc, chu_ky: super::moc_sua::CHU_KY_RONG, bia_mo: true, ..Default::default() }.ma_hoa());
    }
    k.day()
}

fn cu_luc(k: &mut Kho, id: Id) -> Option<u64> {
    k.tach(id).map(|b| b.luc)
}

/// Giữ khoá "app đang chạy" (`<gốc dữ liệu>/app.lock`, khoá của hệ điều hành — app tắt đột ngột thì tự nhả) suốt
/// đời tiến trình. `false` = đã có cửa sổ app / lệnh `--tach` khác đang chạy (chỉ 1 tiến trình ghi kho).
pub fn giu_khoa_ung_dung() -> bool {
    static GIU: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    if GIU.get().is_some() {
        return true;
    }
    match thu_khoa(&super::du_lieu_an::goc_du_lieu().join("app.lock")) {
        Some(Some(tep)) => {
            let _ = GIU.set(tep);
            true
        }
        Some(None) => false,
        None => true, // không tạo / khoá được file (ổ chỉ đọc…) → không chặn mở app
    }
}

/// Khoá file `f`: `Some(Some(tệp))` = giữ được, `Some(None)` = nơi khác đang giữ, `None` = không khoá được.
fn thu_khoa(f: &Path) -> Option<Option<std::fs::File>> {
    let tep = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(f).ok()?;
    match tep.try_lock() {
        Ok(()) => Some(Some(tep)),
        Err(std::fs::TryLockError::WouldBlock) => Some(None),
        Err(_) => None,
    }
}

/// Kho của app (`<gốc dữ liệu>/kho_tach.bin`), mở 1 lần. Không mở được → kho chỉ trong bộ nhớ.
pub fn kho() -> std::sync::MutexGuard<'static, Kho> {
    chung().lock().unwrap_or_else(|e| e.into_inner())
}

/// Kho chung của app.
pub fn chung() -> &'static std::sync::Mutex<Kho> {
    static KHO: std::sync::OnceLock<std::sync::Mutex<Kho>> = std::sync::OnceLock::new();
    KHO.get_or_init(|| std::sync::Mutex::new(Kho::mo(&super::du_lieu_an::goc_du_lieu().join("kho_tach.bin")).unwrap_or_else(|_| Kho::bo_nho())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_kho2_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn bt(so: u32, luc: u64) -> BanTach {
        BanTach {
            so_trang: so,
            nhan: [(1, Nhan::Index), (3, Nhan::Trang)].into_iter().collect(),
            xoay: [(2, Xoay { goc: 90, xac_nhan: true })].into_iter().collect(),
            duyet: [(3, 1)].into_iter().collect(),
            luc,
            chu_ky: 7,
            bia_mo: false,
        }
    }

    fn trang(n: usize) -> Vec<TrangCache> {
        vec![TrangCache::default(); n]
    }

    #[test]
    fn ghi_doc_ban_sau_thang_bo_phan_tich_mo_lai() {
        let d = tam("dung");
        let f = d.join("kho.bin");
        let mut k = Kho::mo(&f).unwrap();
        k.ghi_tach((1, 10), &bt(3, 1)).unwrap();
        k.ghi_tach((1, 10), &bt(3, 2)).unwrap();
        k.ghi_phan_tich((1, 10), &trang(3));
        k.ghi_phan_tich((2, 20), &trang(5));
        assert_eq!(k.phan_tich((2, 20)).unwrap().len(), 5, "đọc được trước khi ghi xuống đĩa");
        k.day().unwrap();
        k.bo_phan_tich((2, 20));
        k.day().unwrap();
        drop(k);
        let mut k = Kho::mo(&f).unwrap();
        assert_eq!(k.tach((1, 10)), Some(bt(3, 2)), "bản sau thắng");
        assert_eq!(k.phan_tich((1, 10)).unwrap().len(), 3);
        assert!(k.phan_tich((2, 20)).is_none(), "đã bỏ");
        assert_eq!((k.so_hong, k.so_khoa()), (0, 1), "chỉ còn PDF (1,10)");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mat_dien_ghi_do_va_hong_giua_file_khong_mat_phan_con_lai() {
        let d = tam("hong");
        let f = d.join("kho.bin");
        let mut k = Kho::mo(&f).unwrap();
        for i in 0..5u64 {
            k.ghi_tach((i, i), &bt(3, i)).unwrap();
        }
        drop(k);
        // Hỏng 1 byte trong bản ghi thứ 2
        let mut b = std::fs::read(&f).unwrap();
        let mot = (b.len() - 8) / 5;
        b[8 + mot + 20] ^= 0xFF;
        // Ghi dở ở cuối (mất điện)
        b.extend(&DAU_BAN_GHI);
        b.extend([TACH, 200, 0]);
        std::fs::write(&f, &b).unwrap();
        let mut k = Kho::mo(&f).unwrap();
        assert_eq!(k.so_hong, 1, "1 bản ghi hỏng giữa (đuôi ghi dở quá ngắn để là bản ghi → chỉ cắt)");
        for i in [0u64, 2, 3, 4] {
            assert_eq!(k.tach((i, i)), Some(bt(3, i)), "bản ghi {i} sau chỗ hỏng vẫn đọc");
        }
        assert!(k.tach((1, 1)).is_none());
        assert_eq!(std::fs::metadata(&f).unwrap().len() as usize, b.len() - 7, "cắt đuôi ghi dở");
        k.ghi_tach((9, 9), &bt(2, 9)).unwrap();
        drop(k);
        assert_eq!(Kho::mo(&f).unwrap().tach((9, 9)), Some(bt(2, 9)), "ghi tiếp sau khi cắt");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn ghi_loi_giu_trong_bo_nho_thu_lai_duoc() {
        use std::os::unix::fs::PermissionsExt;
        let d = tam("day");
        let f = d.join("kho.bin");
        let mut k = Kho::mo(&f).unwrap();
        k.ghi_tach((1, 1), &bt(3, 1)).unwrap();
        // Ổ không ghi được: mở lại file chỉ đọc để giả lập lỗi ghi
        k.doc = Some(std::fs::File::open(&f).unwrap());
        assert!(k.ghi_tach((2, 2), &bt(3, 2)).is_err());
        assert_eq!(k.tach((2, 2)), Some(bt(3, 2)), "vẫn còn trong bộ nhớ");
        k.doc = Some(std::fs::OpenOptions::new().read(true).write(true).open(&f).unwrap());
        k.day().unwrap();
        drop(k);
        let k2 = Kho::mo(&f);
        assert_eq!(k2.unwrap().tach((2, 2)), Some(bt(3, 2)), "thử lại thành công");
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn nen_giu_ban_moi_nhat_co_bak() {
        let d = tam("nen");
        let f = d.join("kho.bin");
        let mut k = Kho::mo(&f).unwrap();
        for luc in 0..50u64 {
            k.ghi_tach((1, 1), &bt(3, luc)).unwrap();
        }
        k.ghi_phan_tich((2, 2), &trang(4));
        k.nen().unwrap();
        assert!(f.with_extension("bak").exists());
        assert_eq!((k.so_ban_ghi, k.tach((1, 1)).unwrap().luc), (2, 49), "1 Tách + 1 phân tích");
        k.ghi_tach((3, 3), &bt(1, 1)).unwrap();
        drop(k);
        let mut k = Kho::mo(&f).unwrap();
        assert_eq!((k.so_ban_ghi, k.tach((1, 1)).unwrap().luc, k.phan_tich((2, 2)).unwrap().len(), k.tach((3, 3)).is_some()), (3, 49, 4, true));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn id_theo_bang_duong_dan_id_that_phat_hien_file_thay_giu_thoi_diem() {
        let d = tam("id");
        let pdf = d.join("a.pdf");
        crate::tach_vb::tach::tests::tao_pdf(&pdf, 2);
        let mut k = Kho::mo(&d.join("kho.bin")).unwrap();
        let id = k.id_cua(&pdf).unwrap();
        let so = k.so_ban_ghi;
        assert_eq!((k.id_cua(&pdf), k.so_ban_ghi), (Some(id), so), "lần 2 tra bảng, không đọc PDF, không ghi");
        // Thay nội dung, giữ kích thước + thời điểm sửa
        let md = std::fs::metadata(&pdf).unwrap();
        let mut b = std::fs::read(&pdf).unwrap();
        let n = b.len();
        b[n - 3] ^= 0x01;
        std::fs::write(&pdf, &b).unwrap();
        std::fs::File::options().write(true).open(&pdf).unwrap().set_modified(md.modified().unwrap()).unwrap();
        assert_eq!(k.id_cua(&pdf), Some(id), "bảng đường dẫn không biết file đã bị thay");
        let that = k.id_that(&pdf).unwrap();
        assert_ne!(that, id, "id thật phát hiện");
        assert_eq!(k.id_cua(&pdf), Some(that), "bảng đường dẫn cập nhật theo id thật");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn khoa_chi_1_noi_giu_nha_khi_dong() {
        let d = tam("khoa");
        let f = d.join("app.lock");
        let giu = thu_khoa(&f).unwrap().expect("giữ được");
        assert!(thu_khoa(&f).unwrap().is_none(), "nơi thứ 2 không giữ được");
        drop(giu);
        assert!(thu_khoa(&f).unwrap().is_some(), "nhả khi đóng (kể cả tắt đột ngột)");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mo_lai_file_cap_nhat_lan_cuoi_thay_don_khong_xoa() {
        let d = tam("lan_cuoi");
        let pdf = d.join("a.pdf");
        crate::tach_vb::tach::tests::tao_pdf(&pdf, 2);
        let k = std::sync::Mutex::new(Kho::mo(&d.join("kho.bin")).unwrap());
        let id = ids_cua_trong(&k, std::slice::from_ref(&pdf))[0].unwrap();
        k.lock().unwrap().ghi_tach(id, &bt(2, 1)).unwrap();
        // Giả lập: lần thấy đầu cách đây 2 năm
        {
            let mut g = k.lock().unwrap();
            let kh = khoa_duong_dan(&pdf);
            let cu = DauFile { lan_cuoi: 1, ..g.duong_dan[&kh] };
            g.ghi_duong_dan(kh, cu);
        }
        assert_eq!(ids_cua_trong(&k, std::slice::from_ref(&pdf))[0], Some(id), "mở lại");
        let mot_nam = 365 * 86_400_000;
        assert_eq!(k.lock().unwrap().don(bay_gio_ms() - mot_nam).unwrap(), 0, "vừa thấy lại → không dọn");
        assert!(k.lock().unwrap().tach(id).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mat_kho_giua_luc_don_dung_ban_sao() {
        let d = tam("mat_kho");
        let f = d.join("kho.bin");
        let mut k = Kho::mo(&f).unwrap();
        k.ghi_tach((1, 1), &bt(3, 5)).unwrap();
        k.nen().unwrap();
        drop(k);
        std::fs::remove_file(&f).unwrap();
        assert_eq!(Kho::mo(&f).unwrap().tach((1, 1)).map(|b| b.luc), Some(5), "kho mất → khôi phục từ .bak");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kho_bo_nho_bao_loi_khi_luu() {
        let mut k = Kho::bo_nho();
        assert!(k.ghi_tach((1, 1), &bt(1, 1)).is_err(), "không có file → báo lỗi, không giả vờ đã lưu");
    }

    #[test]
    fn don_bo_pdf_lau_khong_thay_giu_pdf_moi_thay() {
        let d = tam("don");
        let mut k = Kho::mo(&d.join("kho.bin")).unwrap();
        k.ghi_duong_dan(1, DauFile { kich_thuoc: 10, sua_giay: 1, id: (1, 10), lan_cuoi: 100 });
        k.ghi_duong_dan(2, DauFile { kich_thuoc: 20, sua_giay: 1, id: (2, 20), lan_cuoi: 5_000 });
        for id in [(1, 10), (2, 20)] {
            k.ghi_tach(id, &bt(3, 1)).unwrap();
            k.ghi_phan_tich(id, &trang(3));
            k.ghi_boc(id, Path::new("/x/boc_tach.jsonl"));
        }
        assert_eq!(k.don(1_000).unwrap(), 1);
        assert!(k.tach((1, 10)).is_none() && k.phan_tich((1, 10)).is_none() && k.boc((1, 10)).is_none());
        assert!(k.tach((2, 20)).is_some() && k.boc((2, 20)).is_some());
        drop(k);
        let mut k = Kho::mo(&d.join("kho.bin")).unwrap();
        assert_eq!((k.so_khoa(), k.duong_dan.len(), k.tach((2, 20)).is_some()), (1, 1, true), "mở lại vẫn đúng");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Mở kho 100.000 / 1.000.000 PDF (mỗi PDF: phân tích 20 trang + Tách + đường dẫn).
    /// Chạy: DEMA4_DO=<thư mục tạm> [DEMA4_SO_FILE=1000000] cargo test --release -- --ignored do_kho_lon --nocapture
    #[test]
    #[ignore = "đo hiệu năng"]
    fn do_kho_lon() {
        use std::time::Instant;
        let goc = PathBuf::from(std::env::var("DEMA4_DO").expect("DEMA4_DO"));
        let n: u64 = std::env::var("DEMA4_SO_FILE").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000_000);
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(&goc).unwrap();
        let mau = goc.join("mau");
        std::fs::create_dir_all(&mau).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&mau.join("mau.pdf"), 20);
        let trang = crate::tach_vb::phien::phan_tich_thu_muc(&mau, &Default::default(), &|_, _| {}).ho_so.remove(0).trang;
        let f = goc.join("kho.bin");
        let t = Instant::now();
        let mut k = Kho::mo(&f).unwrap();
        for i in 0..n {
            k.ghi_phan_tich((i, i), &trang);
            k.them(TACH, (i, i), &bt(20, i).ma_hoa());
            k.ghi_duong_dan(i.wrapping_mul(0x9E37_79B9_7F4A_7C15), DauFile { kich_thuoc: i, sua_giay: 1, id: (i, i), lan_cuoi: 1 });
            if i % 10_000 == 0 {
                k.day().unwrap();
            }
        }
        k.day().unwrap();
        println!("DO|ghi|{:.2} s", t.elapsed().as_secs_f64());
        drop(k);
        println!("DO|dung_luong|{:.1} MB", std::fs::metadata(&f).unwrap().len() as f64 / 1e6);
        let rss = || {
            let o = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
            String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().unwrap_or(0) / 1024
        };
        let truoc = rss();
        let t = Instant::now();
        let mut k = Kho::mo(&f).unwrap();
        let _ = (rss, truoc);
        let bo_nho = k.chi_muc.capacity() * (std::mem::size_of::<(Id, ViTri)>() + 1) + k.duong_dan.capacity() * (std::mem::size_of::<(u64, DauFile)>() + 1);
        println!("DO|mo|{:.2} s ({} khoá, {} đường dẫn, chỉ mục ~{} MB)", t.elapsed().as_secs_f64(), k.so_khoa(), k.duong_dan.len(), bo_nho / 1_000_000);
        let t = Instant::now();
        let mut thay = 0;
        for i in (0..n).step_by((n / 100_000).max(1) as usize) {
            thay += k.phan_tich((i, i)).is_some() as usize + k.tach((i, i)).is_some() as usize;
        }
        println!("DO|tra_100000|{:.2} s ({thay})", t.elapsed().as_secs_f64());
        let _ = std::fs::remove_dir_all(&goc);
    }
}
