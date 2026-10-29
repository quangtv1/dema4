//! Bảng trang của màn Tách: nhãn 2 lớp (tự động / sửa tay), cờ duyệt, góc xoay, bộ lọc, hoàn tác riêng từng nhóm.
//!
//! Nhãn dùng = `tay` nếu có, không thì nhãn tự động. Kết quả phân tích tới muộn, đổi ngưỡng, nạp cache chỉ ghi
//! nhãn tự động → không bao giờ đè sửa tay. Mọi sửa của người dùng đi qua [`BangTrang::sua`]: ghi bước hoàn tác
//! (chỉ trang bị đổi + giá trị trước/sau), đặt cờ chưa lưu, cập nhật số đếm cộng dồn — không quét lại cả bảng.

use crate::tach_vb::cache_trang::{DANG_CHINH_XOAY, DA_XOA, GIU_NOI_DUNG};
use crate::tach_vb::nhan::{BangNhan, BangXoay, Nhan, Xoay};
use crate::tach_vb::phan_tich::{gan_nhan, CauHinhPhanTich, NhanTuDong};
use crate::tach_vb::phien::PhienPhanTich;

/// Cờ của trang (3 bit đầu trùng `trang_thai` trong cache).
pub const CO_GIU_NOI_DUNG: u8 = GIU_NOI_DUNG;
pub const CO_DA_XOA: u8 = DA_XOA;
/// Nghi xoay đã chỉnh góc, chờ xác nhận.
pub const CO_DA_CHINH: u8 = DANG_CHINH_XOAY;
/// Nghi xoay đã xác nhận (kể cả 0°): không gắn cờ lại. Lưu ở `xoay_trang.txt` (`;xac_nhan`).
pub const CO_XAC_NHAN: u8 = 8;
const BIT_CACHE: u8 = GIU_NOI_DUNG | DA_XOA | DANG_CHINH_XOAY;

pub const BUOC_TOI_DA: usize = 100;

/// Phần người dùng sửa được của 1 trang (đơn vị lưu của bước hoàn tác).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sua {
    pub tay: Option<Nhan>,
    pub co: u8,
    /// Góc xoay thêm khi tách (0/90/180/270, theo kim đồng hồ).
    pub goc: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrangMeta {
    pub ho_so: u32,
    /// Số trang trong hồ sơ (1-based).
    pub so: u32,
    pub dung_luong: u32,
    /// `None` = chưa có kết quả phân tích.
    pub tu_dong: Option<NhanTuDong>,
    pub sua: Sua,
}

impl TrangMeta {
    pub fn nhan(&self) -> Nhan {
        self.sua.tay.or(self.tu_dong.map(|t| t.nhan)).unwrap_or(Nhan::NoiDung)
    }

    pub fn da_xoa(&self) -> bool {
        self.sua.co & CO_DA_XOA != 0
    }

    /// Cờ Nghi trắng còn hiệu lực: máy nghi, người chưa gán nhãn, chưa duyệt giữ nội dung.
    pub fn nghi_trang(&self) -> bool {
        self.tu_dong.is_some_and(|t| t.nghi_trang) && self.sua.tay.is_none() && self.sua.co & CO_GIU_NOI_DUNG == 0
    }

    /// Góc đề xuất khi cờ Nghi xoay còn hiệu lực (gán I/B chỉ ẩn cờ; gỡ nhãn → hiện lại).
    pub fn nghi_xoay(&self) -> Option<u16> {
        let goc = self.tu_dong.and_then(|t| t.nghi_xoay)?;
        (matches!(self.nhan(), Nhan::NoiDung | Nhan::Split) && self.sua.co & CO_XAC_NHAN == 0).then_some(goc)
    }

    pub fn da_chinh(&self) -> bool {
        self.sua.co & CO_DA_CHINH != 0
    }

    /// Bit trạng thái ghi vào cache.
    pub fn trang_thai_cache(&self) -> u8 {
        self.sua.co & BIT_CACHE
    }
}

/// Bộ lọc lưới tab Hồ sơ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loc {
    TatCa,
    Index,
    Trang,
    NghiTrang,
    NghiXoay,
    DiemCat,
}

/// Nhóm hoàn tác: mỗi bộ lọc + tab Văn bản có ngăn xếp riêng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nhom {
    Loc(Loc),
    VanBan,
}

impl Nhom {
    fn chi_so(self) -> usize {
        match self {
            Nhom::Loc(l) => l as usize,
            Nhom::VanBan => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sap {
    ThuTu,
    DungLuongTang,
    DungLuongGiam,
}

/// Số đếm (trang chưa xoá) theo nhãn / cờ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dem {
    pub hien: u32,
    pub da_xoa: u32,
    pub index: u32,
    pub trang: u32,
    pub split: u32,
    pub nghi_trang: u32,
    pub nghi_xoay: u32,
}

impl Dem {
    fn cua(t: &TrangMeta) -> Self {
        if t.da_xoa() {
            return Dem { da_xoa: 1, ..Default::default() };
        }
        let n = t.nhan();
        Dem {
            hien: 1,
            da_xoa: 0,
            index: (n == Nhan::Index) as u32,
            trang: (n == Nhan::Trang) as u32,
            split: (n == Nhan::Split) as u32,
            nghi_trang: t.nghi_trang() as u32,
            nghi_xoay: t.nghi_xoay().is_some() as u32,
        }
    }

    fn cong(&mut self, d: Dem, dau: i64) {
        for (a, b) in [
            (&mut self.hien, d.hien),
            (&mut self.da_xoa, d.da_xoa),
            (&mut self.index, d.index),
            (&mut self.trang, d.trang),
            (&mut self.split, d.split),
            (&mut self.nghi_trang, d.nghi_trang),
            (&mut self.nghi_xoay, d.nghi_xoay),
        ] {
            *a = (*a as i64 + dau * b as i64) as u32;
        }
    }

    pub fn theo_loc(&self, l: Loc) -> u32 {
        match l {
            Loc::TatCa => self.hien,
            Loc::Index => self.index,
            Loc::Trang => self.trang,
            Loc::NghiTrang => self.nghi_trang,
            Loc::NghiXoay => self.nghi_xoay,
            Loc::DiemCat => self.split,
        }
    }
}

#[derive(Debug, Clone)]
struct Buoc {
    mo_ta: String,
    /// (trang, trước, sau)
    doi: Vec<(u32, Sua, Sua)>,
    /// Vùng chọn lúc làm (trả lại khi hoàn tác / làm lại).
    chon: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
struct LichSu {
    hoan_tac: Vec<Buoc>,
    lam_lai: Vec<Buoc>,
}

/// Kết quả hoàn tác / làm lại 1 bước.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KetQuaDao {
    pub mo_ta: String,
    pub so_trang: usize,
    /// Trang đã bị sửa tiếp ở nhóm khác → bỏ qua, không ghi đè.
    pub bo_qua: usize,
    pub chon: Vec<u32>,
}

/// Việc Enter làm trong bộ lọc Nghi xoay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterXoay {
    /// Chưa chỉnh → đặt góc đề xuất.
    XoayDeXuat,
    /// Mọi trang cờ đang chọn đã chỉnh → xác nhận.
    XacNhan,
    KhongCo,
}

pub struct BangTrang {
    /// Đường dẫn tương đối của hồ sơ (chỉ số = `ho_so`), đã sắp theo đường dẫn.
    pub ho_so: Vec<String>,
    pub trang: Vec<TrangMeta>,
    dem_ho_so: Vec<Dem>,
    pub tong: Dem,
    lich_su: [LichSu; 7],
    pub chua_luu: bool,
}

impl BangTrang {
    /// `trang` theo thứ tự hồ sơ rồi số trang.
    pub fn moi(ho_so: Vec<String>, trang: Vec<TrangMeta>) -> Self {
        let mut b = BangTrang { dem_ho_so: vec![Dem::default(); ho_so.len()], ho_so, trang, tong: Dem::default(), lich_su: Default::default(), chua_luu: false };
        for t in &b.trang {
            let d = Dem::cua(t);
            b.dem_ho_so[t.ho_so as usize].cong(d, 1);
            b.tong.cong(d, 1);
        }
        b
    }

    pub fn dem_ho_so(&self, h: u32) -> Dem {
        self.dem_ho_so[h as usize]
    }

    /// Đổi 1 trang và cập nhật số đếm.
    fn dat(&mut self, k: u32, f: impl FnOnce(&mut TrangMeta)) {
        let t = &mut self.trang[k as usize];
        let truoc = Dem::cua(t);
        f(t);
        let sau = Dem::cua(t);
        if truoc != sau {
            let h = t.ho_so as usize;
            self.dem_ho_so[h].cong(truoc, -1);
            self.dem_ho_so[h].cong(sau, 1);
            self.tong.cong(truoc, -1);
            self.tong.cong(sau, 1);
        }
    }

    /// Nhãn tự động mới (kết quả phân tích, đổi ngưỡng / mã / độ tin cậy). Không ghi lịch sử, không đụng sửa tay.
    pub fn dat_tu_dong(&mut self, k: u32, td: NhanTuDong) {
        self.dat(k, |t| t.tu_dong = Some(td));
    }

    /// Điểm duy nhất người dùng sửa trang: áp `f` lên từng trang trong `ds`, ghi 1 bước vào nhóm `nhom`.
    /// Trả số trang thực sự đổi (0 → không ghi bước).
    pub fn sua(&mut self, nhom: Nhom, mo_ta: &str, ds: &[u32], chon: &[u32], f: impl Fn(&TrangMeta) -> Sua) -> usize {
        let mut doi = Vec::new();
        for &k in ds {
            let t = self.trang[k as usize];
            let moi = f(&t);
            if moi != t.sua {
                self.dat(k, |t| t.sua = moi);
                doi.push((k, t.sua, moi));
            }
        }
        let n = doi.len();
        if n > 0 {
            let ls = &mut self.lich_su[nhom.chi_so()];
            ls.lam_lai.clear();
            ls.hoan_tac.push(Buoc { mo_ta: format!("{mo_ta} {n} trang"), doi, chon: chon.to_vec() });
            if ls.hoan_tac.len() > BUOC_TOI_DA {
                ls.hoan_tac.remove(0);
            }
            self.chua_luu = true;
        }
        n
    }

    /// Mô tả bước sẽ hoàn tác / làm lại của nhóm (cho tooltip); None = hết bước.
    pub fn buoc_ke(&self, nhom: Nhom, lam_lai: bool) -> Option<&str> {
        let ls = &self.lich_su[nhom.chi_so()];
        if lam_lai { ls.lam_lai.last() } else { ls.hoan_tac.last() }.map(|b| b.mo_ta.as_str())
    }

    fn dao(&mut self, nhom: Nhom, lam_lai: bool) -> Option<KetQuaDao> {
        let i = nhom.chi_so();
        let buoc = if lam_lai { self.lich_su[i].lam_lai.pop()? } else { self.lich_su[i].hoan_tac.pop()? };
        let mut bo_qua = 0;
        for &(k, truoc, sau) in &buoc.doi {
            let (hien_tai, dich) = if lam_lai { (truoc, sau) } else { (sau, truoc) };
            if self.trang[k as usize].sua == hien_tai {
                self.dat(k, |t| t.sua = dich);
            } else {
                bo_qua += 1;
            }
        }
        let kq = KetQuaDao { mo_ta: buoc.mo_ta.clone(), so_trang: buoc.doi.len() - bo_qua, bo_qua, chon: buoc.chon.clone() };
        if lam_lai { &mut self.lich_su[i].hoan_tac } else { &mut self.lich_su[i].lam_lai }.push(buoc);
        self.chua_luu = true;
        Some(kq)
    }

    pub fn hoan_tac(&mut self, nhom: Nhom) -> Option<KetQuaDao> {
        self.dao(nhom, false)
    }

    pub fn lam_lai(&mut self, nhom: Nhom) -> Option<KetQuaDao> {
        self.dao(nhom, true)
    }

    // ---- Thao tác (mỗi hàm = 1 bước hoàn tác) ----

    /// Gán I / B / S kiểu bật/tắt: mọi trang áp dụng đã mang nhãn đó → về nội dung; còn lại → gán.
    /// S chỉ áp trang nội dung / Split. `Nhan::NoiDung` = phím C: bỏ mọi nhãn.
    pub fn gan(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32], nhan: Nhan) -> usize {
        let ap: Vec<u32> = if nhan == Nhan::Split { ds.iter().copied().filter(|&k| matches!(self.trang[k as usize].nhan(), Nhan::NoiDung | Nhan::Split)).collect() } else { ds.to_vec() };
        let tat = nhan != Nhan::NoiDung && !ap.is_empty() && ap.iter().all(|&k| self.trang[k as usize].nhan() == nhan);
        let (dich, mo_ta) = match (tat, nhan) {
            (_, Nhan::NoiDung) => (Nhan::NoiDung, "bỏ nhãn".to_string()),
            (true, n) => (Nhan::NoiDung, format!("bỏ {}", ten_nhan(n))),
            (false, n) => (n, format!("gán {}", ten_nhan(n))),
        };
        // Trang đã mang đúng nhãn đó giữ nguyên (trừ C trên trang Nghi trắng: bỏ cờ)
        self.sua(nhom, &mo_ta, &ap, chon, |t| if t.nhan() == dich && !t.nghi_trang() { t.sua } else { Sua { tay: Some(dich), ..t.sua } })
    }

    /// Xoá khỏi lưới: trang thành nội dung (bỏ nhãn, bỏ cờ Nghi trắng) + ẩn; vẫn có trong file tách.
    pub fn xoa(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32]) -> usize {
        self.sua(nhom, "xoá", ds, chon, |t| Sua { tay: Some(Nhan::NoiDung), co: t.sua.co | CO_DA_XOA, ..t.sua })
    }

    /// Hiện lại trang đã xoá của hồ sơ `ho_so` (None = mọi hồ sơ), giữ nhãn hiện tại.
    pub fn khoi_phuc(&mut self, nhom: Nhom, ho_so: Option<u32>, chon: &[u32]) -> usize {
        let ds: Vec<u32> = (0..self.trang.len() as u32).filter(|&k| self.trang[k as usize].da_xoa() && ho_so.is_none_or(|h| self.trang[k as usize].ho_so == h)).collect();
        self.sua(nhom, "khôi phục", &ds, chon, |t| Sua { co: t.sua.co & !CO_DA_XOA, ..t.sua })
    }

    /// R / Shift+R: cộng dồn góc cho mọi trang chọn. Trang Nghi xoay: `chot` (tab Văn bản) = xác nhận luôn,
    /// không thì đánh dấu đã chỉnh.
    pub fn xoay(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32], them: u16, chot: bool) -> usize {
        self.sua(nhom, &format!("xoay {them}°"), ds, chon, |t| {
            let mut s = Sua { goc: (t.sua.goc + them) % 360, ..t.sua };
            if t.nghi_xoay().is_some() {
                s.co = if chot { (s.co | CO_XAC_NHAN) & !CO_DA_CHINH } else { s.co | CO_DA_CHINH };
            }
            s
        })
    }

    /// Đặt (không cộng dồn) góc đề xuất cho trang Nghi xoay; trang vẫn trong danh sách.
    pub fn xoay_de_xuat(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32]) -> usize {
        self.sua(nhom, "xoay đề xuất", ds, chon, |t| match t.nghi_xoay() {
            Some(g) => Sua { goc: g, co: t.sua.co | CO_DA_CHINH, ..t.sua },
            None => t.sua,
        })
    }

    /// C trong bộ lọc Nghi xoay: về chiều gốc 0°, giữ cờ.
    pub fn ve_chieu_goc(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32]) -> usize {
        self.sua(nhom, "về chiều gốc", ds, chon, |t| match t.nghi_xoay() {
            Some(_) => Sua { goc: 0, co: t.sua.co | CO_DA_CHINH, ..t.sua },
            None => t.sua,
        })
    }

    /// Chốt góc hiện tại (0° = đề xuất sai), bỏ cờ Nghi xoay.
    pub fn xac_nhan_xoay(&mut self, nhom: Nhom, ds: &[u32], chon: &[u32]) -> usize {
        self.sua(nhom, "xác nhận xoay", ds, chon, |t| match t.nghi_xoay() {
            Some(_) => Sua { co: (t.sua.co | CO_XAC_NHAN) & !CO_DA_CHINH, ..t.sua },
            None => t.sua,
        })
    }

    /// Enter trong bộ lọc Nghi xoay: lần đầu xoay đề xuất, khi mọi trang cờ đang chọn đã chỉnh thì xác nhận.
    pub fn viec_enter_xoay(&self, ds: &[u32]) -> EnterXoay {
        let co: Vec<&TrangMeta> = ds.iter().map(|&k| &self.trang[k as usize]).filter(|t| t.nghi_xoay().is_some()).collect();
        match (co.is_empty(), co.iter().all(|t| t.da_chinh())) {
            (true, _) => EnterXoay::KhongCo,
            (false, true) => EnterXoay::XacNhan,
            (false, false) => EnterXoay::XoayDeXuat,
        }
    }

    /// Duyệt Nghi trắng trong bộ lọc: Trắng / Index gán nhãn, NoiDung = giữ nội dung. Chỉ áp trang còn cờ.
    pub fn duyet_nghi_trang(&mut self, ds: &[u32], chon: &[u32], nhan: Nhan) -> usize {
        let mo_ta = match nhan {
            Nhan::NoiDung => "giữ nội dung".to_string(),
            n => format!("gán {}", ten_nhan(n)),
        };
        self.sua(Nhom::Loc(Loc::NghiTrang), &mo_ta, ds, chon, |t| match (t.nghi_trang(), nhan) {
            (false, _) => t.sua,
            (true, Nhan::NoiDung) => Sua { co: t.sua.co | CO_GIU_NOI_DUNG, ..t.sua },
            (true, n) => Sua { tay: Some(n), ..t.sua },
        })
    }

    // ---- Hiển thị ----

    pub fn thuoc_loc(&self, k: u32, l: Loc) -> bool {
        let t = &self.trang[k as usize];
        !t.da_xoa()
            && match l {
                Loc::TatCa => true,
                Loc::Index => t.nhan() == Nhan::Index,
                Loc::Trang => t.nhan() == Nhan::Trang,
                Loc::DiemCat => t.nhan() == Nhan::Split,
                Loc::NghiTrang => t.nghi_trang(),
                Loc::NghiXoay => t.nghi_xoay().is_some(),
            }
    }

    /// Thứ tự hiện của lưới (bỏ trang đã xoá).
    pub fn thu_tu(&self, l: Loc, ho_so: Option<u32>, sap: Sap) -> Vec<u32> {
        let mut ds: Vec<u32> = (0..self.trang.len() as u32).filter(|&k| ho_so.is_none_or(|h| self.trang[k as usize].ho_so == h) && self.thuoc_loc(k, l)).collect();
        match sap {
            Sap::ThuTu => {}
            Sap::DungLuongTang => ds.sort_by_key(|&k| self.trang[k as usize].dung_luong),
            Sap::DungLuongGiam => ds.sort_by_key(|&k| std::cmp::Reverse(self.trang[k as usize].dung_luong)),
        }
        ds
    }

    /// Còn trang Nghi trắng / Nghi xoay (chưa xoá) → chưa cho Tách.
    pub fn con_nghi(&self) -> (u32, u32) {
        (self.tong.nghi_trang, self.tong.nghi_xoay)
    }

    // ---- Lưu ----

    /// Nhãn dùng của mọi trang (ghi `index_and_blank.txt`).
    pub fn bang_nhan(&self) -> BangNhan {
        let mut b = BangNhan::new();
        for t in &self.trang {
            let n = t.nhan();
            if n != Nhan::NoiDung {
                b.entry(self.ho_so[t.ho_so as usize].clone()).or_default().insert(t.so as usize, n);
            }
        }
        b
    }

    /// Góc xoay + xác nhận (ghi `xoay_trang.txt`).
    pub fn bang_xoay(&self) -> BangXoay {
        let mut b = BangXoay::new();
        for t in &self.trang {
            let xac_nhan = t.sua.co & CO_XAC_NHAN != 0;
            if t.sua.goc != 0 || xac_nhan {
                b.entry(self.ho_so[t.ho_so as usize].clone()).or_default().insert(t.so as usize, Xoay { goc: t.sua.goc, xac_nhan });
            }
        }
        b
    }
}

impl BangTrang {
    /// Dựng bảng từ phiên phân tích. Mở lại thư mục: nhãn trong file khác nhãn tự động → là sửa tay
    /// (trang có kết quả mà file không ghi nhãn → người đã bỏ nhãn); góc + xác nhận từ file xoay;
    /// cờ duyệt (giữ nội dung, đã xoá, đang chỉnh) từ cache.
    pub fn tu_phien(phien: &PhienPhanTich, ch: &CauHinhPhanTich, nhan_file: Option<&BangNhan>, xoay_file: Option<&BangXoay>) -> Self {
        let mut trang = Vec::new();
        for (h, hs) in phien.ho_so.iter().enumerate() {
            let nf = nhan_file.and_then(|b| b.get(&hs.tuong_doi));
            let xf = xoay_file.and_then(|b| b.get(&hs.tuong_doi));
            for (i, tc) in hs.trang.iter().enumerate() {
                let so = i + 1;
                let tu_dong = tc.ket_qua.as_ref().map(|kq| gan_nhan(kq, ch));
                let tay = match (nhan_file, tu_dong) {
                    (Some(_), Some(td)) => {
                        let trong_file = nf.and_then(|m| m.get(&so)).copied().unwrap_or(Nhan::NoiDung);
                        (trong_file != td.nhan).then_some(trong_file)
                    }
                    (Some(_), None) => nf.and_then(|m| m.get(&so)).copied(),
                    _ => None,
                };
                let x = xf.and_then(|m| m.get(&so));
                let co = (tc.trang_thai & BIT_CACHE) | if x.is_some_and(|x| x.xac_nhan) { CO_XAC_NHAN } else { 0 };
                let sua = Sua { tay, co, goc: x.map_or(0, |x| x.goc) };
                trang.push(TrangMeta { ho_so: h as u32, so: so as u32, dung_luong: tc.thong_tin.dung_luong, tu_dong, sua });
            }
        }
        Self::moi(phien.ho_so.iter().map(|h| h.tuong_doi.clone()).collect(), trang)
    }

    /// Đổi Cấu hình (ngưỡng, mã index, độ tin cậy): gán lại nhãn tự động từ kết quả đã lưu, không render lại.
    pub fn gan_lai_tu_dong(&mut self, phien: &PhienPhanTich, ch: &CauHinhPhanTich) {
        for k in 0..self.trang.len() as u32 {
            let t = self.trang[k as usize];
            if let Some(kq) = phien.ho_so[t.ho_so as usize].trang[t.so as usize - 1].ket_qua.as_ref() {
                self.dat_tu_dong(k, gan_nhan(kq, ch));
            }
        }
    }

    /// Chép cờ duyệt về phiên trước khi ghi cache.
    pub fn chep_trang_thai(&self, phien: &mut PhienPhanTich) {
        for t in &self.trang {
            if let Some(tc) = phien.ho_so[t.ho_so as usize].trang.get_mut(t.so as usize - 1) {
                tc.trang_thai = t.trang_thai_cache();
            }
        }
    }
}

pub fn ten_nhan(n: Nhan) -> &'static str {
    match n {
        Nhan::NoiDung => "Nội dung",
        Nhan::Index => "Index",
        Nhan::Trang => "Trắng",
        Nhan::Split => "Split",
    }
}

/// Trang kế tiếp thoả `con` sau `sau` theo thứ tự `thu_tu` (vòng lại đầu); dùng để tự sang trang Nghi kế.
pub fn ke_tiep(thu_tu: &[u32], sau: u32, con: impl Fn(u32) -> bool) -> Option<u32> {
    let i = thu_tu.iter().position(|&k| k == sau).map_or(0, |i| i + 1);
    thu_tu[i..].iter().chain(&thu_tu[..i]).copied().find(|&k| con(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TC: Nhom = Nhom::Loc(Loc::TatCa);

    fn td(nhan: Nhan, nghi_trang: bool, nghi_xoay: Option<u16>) -> Option<NhanTuDong> {
        Some(NhanTuDong { nhan, nghi_trang, nghi_xoay })
    }

    /// 2 hồ sơ: a (6 trang), b (4 trang). a.2 Index, a.4 Trắng, a.5 Nghi trắng, a.6 Nghi xoay 180, b.2 Nghi xoay 90.
    fn bang() -> BangTrang {
        let mut trang = Vec::new();
        for (h, n) in [(0u32, 6u32), (1, 4)] {
            for so in 1..=n {
                trang.push(TrangMeta { ho_so: h, so, dung_luong: 1000 - so * 10 + h, tu_dong: td(Nhan::NoiDung, false, None), sua: Sua::default() });
            }
        }
        trang[1].tu_dong = td(Nhan::Index, false, None);
        trang[3].tu_dong = td(Nhan::Trang, false, None);
        trang[4].tu_dong = td(Nhan::NoiDung, true, None);
        trang[5].tu_dong = td(Nhan::NoiDung, false, Some(180));
        trang[7].tu_dong = td(Nhan::NoiDung, false, Some(90));
        trang[9].tu_dong = None;
        BangTrang::moi(vec!["a.pdf".into(), "b/b.pdf".into()], trang)
    }

    fn dem_lai(b: &BangTrang) -> Dem {
        let mut d = Dem::default();
        b.trang.iter().for_each(|t| d.cong(Dem::cua(t), 1));
        d
    }

    #[test]
    fn nhan_hai_lop_ket_qua_toi_muon_khong_de_sua_tay() {
        let mut b = bang();
        b.gan(TC, &[9], &[], Nhan::Index);
        assert_eq!(b.trang[9].nhan(), Nhan::Index);
        // Kết quả phân tích trang 9 tới sau: nhãn dùng vẫn là sửa tay
        b.dat_tu_dong(9, NhanTuDong { nhan: Nhan::Trang, nghi_trang: false, nghi_xoay: None });
        assert_eq!(b.trang[9].nhan(), Nhan::Index);
        // Đổi ngưỡng chỉ đổi trang chưa sửa tay
        b.dat_tu_dong(4, NhanTuDong { nhan: Nhan::Trang, nghi_trang: false, nghi_xoay: None });
        assert_eq!(b.trang[4].nhan(), Nhan::Trang);
        assert_eq!(b.tong, dem_lai(&b));
        assert_eq!(b.tong.trang, 2);
    }

    #[test]
    fn gan_bat_tat_va_split_chi_ap_trang_noi_dung() {
        let mut b = bang();
        assert_eq!(b.gan(TC, &[0, 1], &[], Nhan::Index), 1, "a.2 đã Index → chỉ a.1 đổi");
        assert_eq!(b.gan(TC, &[0, 1], &[], Nhan::Index), 2, "cả 2 đã Index → bỏ");
        assert_eq!((b.trang[0].nhan(), b.trang[1].nhan()), (Nhan::NoiDung, Nhan::NoiDung));
        assert_eq!(b.gan(TC, &[2, 3], &[], Nhan::Split), 1, "a.4 Trắng không nhận Split");
        assert_eq!(b.trang[3].nhan(), Nhan::Trang);
        assert_eq!(b.gan(TC, &[2, 3], &[], Nhan::Split), 1, "bấm lại → bỏ Split");
        assert_eq!(b.trang[2].nhan(), Nhan::NoiDung);
        assert_eq!(b.gan(TC, &[3], &[], Nhan::NoiDung), 1);
        assert_eq!(b.trang[3].nhan(), Nhan::NoiDung);
        assert_eq!(b.buoc_ke(TC, false), Some("bỏ nhãn 1 trang"));
        assert!(b.chua_luu);
        assert_eq!(b.tong, dem_lai(&b));
    }

    #[test]
    fn loc_sap_xoa_khoi_phuc() {
        let mut b = bang();
        assert_eq!(b.thu_tu(Loc::Index, None, Sap::ThuTu), [1]);
        assert_eq!(b.thu_tu(Loc::NghiTrang, None, Sap::ThuTu), [4]);
        assert_eq!(b.thu_tu(Loc::NghiXoay, None, Sap::ThuTu), [5, 7]);
        assert_eq!(b.thu_tu(Loc::TatCa, Some(1), Sap::ThuTu), [6, 7, 8, 9]);
        // Dung lượng: a.n = 1000-10n, b.n = 1001-10n → hoà thứ tự ổn định
        assert_eq!(b.thu_tu(Loc::TatCa, None, Sap::DungLuongTang)[..4], [5, 4, 3, 9]);
        assert_eq!(b.thu_tu(Loc::TatCa, None, Sap::DungLuongGiam)[..2], [6, 0]);
        // Xoá Index + Nghi trắng + Nghi xoay: bỏ nhãn, ẩn, không đếm; cờ Nghi xoay giữ để Khôi phục hiện lại
        assert_eq!(b.xoa(TC, &[1, 4, 7], &[]), 3);
        assert_eq!(b.tong.index, 0);
        assert_eq!(b.tong.nghi_trang, 0);
        assert_eq!(b.tong.nghi_xoay, 1);
        assert_eq!(b.tong.da_xoa, 3);
        assert_eq!(b.tong.hien, 7);
        assert!(!b.thu_tu(Loc::TatCa, None, Sap::ThuTu).contains(&1));
        // Kết quả phân tích tới sau không làm hiện lại trang đã xoá
        b.dat_tu_dong(1, NhanTuDong { nhan: Nhan::Index, nghi_trang: false, nghi_xoay: None });
        assert!(b.trang[1].da_xoa() && b.trang[1].nhan() == Nhan::NoiDung);
        // Khôi phục chỉ trong hồ sơ đang lọc
        assert_eq!(b.khoi_phuc(TC, Some(1), &[]), 1);
        assert_eq!(b.tong.nghi_xoay, 2, "b.2 hiện lại với cờ");
        assert_eq!(b.khoi_phuc(TC, None, &[]), 2);
        assert_eq!(b.trang[1].nhan(), Nhan::NoiDung, "khôi phục giữ nhãn hiện tại");
        assert_eq!(b.tong, dem_lai(&b));
    }

    #[test]
    fn duyet_nghi_trang_va_tu_sang_trang_ke() {
        let mut b = bang();
        b.dat_tu_dong(0, NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: true, nghi_xoay: None });
        b.dat_tu_dong(8, NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: true, nghi_xoay: None });
        let tt = b.thu_tu(Loc::NghiTrang, None, Sap::ThuTu);
        assert_eq!(tt, [0, 4, 8]);
        assert_eq!(b.duyet_nghi_trang(&[4], &[4], Nhan::Trang), 1);
        assert_eq!(ke_tiep(&tt, 4, |k| b.trang[k as usize].nghi_trang()), Some(8));
        assert_eq!(b.duyet_nghi_trang(&[8], &[8], Nhan::NoiDung), 1);
        assert!(b.trang[8].sua.co & CO_GIU_NOI_DUNG != 0 && b.trang[8].nhan() == Nhan::NoiDung);
        assert_eq!(ke_tiep(&tt, 8, |k| b.trang[k as usize].nghi_trang()), Some(0), "vòng lại đầu");
        assert_eq!(b.duyet_nghi_trang(&[0, 4], &[], Nhan::Index), 1, "a.5 đã duyệt → bỏ qua");
        assert_eq!(ke_tiep(&tt, 0, |k| b.trang[k as usize].nghi_trang()), None);
        // Đổi ngưỡng sau khi duyệt: trang đã giữ nội dung không gắn cờ lại
        b.dat_tu_dong(8, NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: true, nghi_xoay: None });
        assert!(!b.trang[8].nghi_trang());
        // Ctrl+Z trong nhóm Nghi trắng trả lại cờ + nhãn
        b.hoan_tac(Nhom::Loc(Loc::NghiTrang)).unwrap();
        b.hoan_tac(Nhom::Loc(Loc::NghiTrang)).unwrap();
        assert!(b.trang[0].nghi_trang() && b.trang[8].nghi_trang());
        assert_eq!(b.tong, dem_lai(&b));
    }

    #[test]
    fn nghi_xoay_dat_de_xuat_chinh_xac_nhan() {
        let mut b = bang();
        let nx = Nhom::Loc(Loc::NghiXoay);
        assert_eq!(b.viec_enter_xoay(&[5]), EnterXoay::XoayDeXuat);
        b.xoay_de_xuat(nx, &[5], &[5]);
        b.xoay_de_xuat(nx, &[5], &[5]);
        assert_eq!(b.trang[5].sua.goc, 180, "đặt, không cộng dồn");
        assert_eq!(b.trang[5].nghi_xoay(), Some(180), "vẫn trong danh sách");
        assert_eq!(b.viec_enter_xoay(&[5]), EnterXoay::XacNhan);
        b.xoay(nx, &[5], &[5], 90, false);
        assert_eq!(b.trang[5].sua.goc, 270);
        b.ve_chieu_goc(nx, &[5], &[5]);
        assert_eq!(b.trang[5].sua.goc, 0);
        b.xac_nhan_xoay(nx, &[5], &[5]);
        assert_eq!(b.trang[5].nghi_xoay(), None);
        // Xác nhận ở 0°: đổi độ tin cậy không gắn cờ lại; lưu dòng `;0;xac_nhan`
        b.dat_tu_dong(5, NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: false, nghi_xoay: Some(90) });
        assert_eq!(b.trang[5].nghi_xoay(), None);
        assert_eq!(b.bang_xoay()["a.pdf"][&6], Xoay { goc: 0, xac_nhan: true });
        // Lẫn trang có / không cờ: Enter chỉ xét trang có cờ, R áp mọi trang
        assert_eq!(b.viec_enter_xoay(&[0, 7]), EnterXoay::XoayDeXuat);
        assert_eq!(b.xoay_de_xuat(nx, &[0, 7], &[]), 1);
        assert_eq!(b.xoay(nx, &[0, 7], &[], 90, false), 2);
        assert_eq!((b.trang[0].sua.goc, b.trang[7].sua.goc), (90, 180));
        // Gán I ẩn cờ, gỡ → hiện lại
        b.gan(TC, &[7], &[], Nhan::Index);
        assert_eq!(b.trang[7].nghi_xoay(), None);
        b.gan(TC, &[7], &[], Nhan::Index);
        assert_eq!(b.trang[7].nghi_xoay(), Some(90));
        // Xoay ở tab Văn bản = chốt luôn
        b.xoay(Nhom::VanBan, &[7], &[], 90, true);
        assert_eq!(b.trang[7].nghi_xoay(), None);
        assert_eq!(b.con_nghi(), (1, 0));
        assert_eq!(b.tong, dem_lai(&b));
    }

    #[test]
    fn xoay_cong_don_va_ve_0_khong_ghi_file() {
        let mut b = bang();
        b.xoay(TC, &[0, 2], &[], 90, false);
        b.xoay(TC, &[0], &[], 180, false);
        assert_eq!((b.trang[0].sua.goc, b.trang[2].sua.goc), (270, 90));
        b.xoay(TC, &[0], &[], 90, false);
        assert_eq!(b.trang[0].sua.goc, 0);
        let x = b.bang_xoay();
        assert_eq!(x["a.pdf"].len(), 1, "góc 0 chưa xác nhận không ghi");
        b.hoan_tac(TC).unwrap();
        assert_eq!(b.trang[0].sua.goc, 270);
    }

    #[test]
    fn hoan_tac_rieng_tung_nhom() {
        let mut b = bang();
        let (nt, tr) = (Nhom::Loc(Loc::NghiTrang), Nhom::Loc(Loc::Trang));
        b.gan(TC, &[0], &[0], Nhan::Trang);
        b.duyet_nghi_trang(&[4], &[4], Nhan::Trang);
        // Ctrl+Z ở Nghi trắng chỉ trả a.5
        let kq = b.hoan_tac(nt).unwrap();
        assert_eq!((kq.mo_ta.as_str(), kq.so_trang, kq.bo_qua, kq.chon), ("gán Trắng 1 trang", 1, 0, vec![4]));
        assert!(b.trang[4].nghi_trang());
        assert_eq!(b.trang[0].nhan(), Nhan::Trang);
        assert!(b.hoan_tac(nt).is_none(), "nhóm rỗng");
        // Làm lại đúng nhóm
        b.lam_lai(nt).unwrap();
        assert_eq!(b.trang[4].nhan(), Nhan::Trang);
        // a.1 gán Trắng ở Tất cả rồi gán Index ở nhóm Trắng → Ctrl+Z ở Tất cả bỏ qua a.1
        b.gan(tr, &[0], &[], Nhan::Index);
        let kq = b.hoan_tac(TC).unwrap();
        assert_eq!((kq.so_trang, kq.bo_qua), (0, 1));
        assert_eq!(b.trang[0].nhan(), Nhan::Index);
        // Thao tác mới xoá ngăn làm lại của nhóm đó
        assert!(b.buoc_ke(TC, true).is_some());
        b.gan(TC, &[2], &[], Nhan::Split);
        assert!(b.buoc_ke(TC, true).is_none());
        // Tối đa 100 bước
        for _ in 0..120 {
            b.xoay(TC, &[3], &[], 90, false);
        }
        assert_eq!(b.lich_su[TC.chi_so()].hoan_tac.len(), BUOC_TOI_DA);
        assert_eq!(b.tong, dem_lai(&b));
    }

    #[test]
    fn chuoi_thao_tac_ngau_nhien_dem_khop_va_hoan_tac_het_ve_ban_dau() {
        let mut hat = 0x9E3779B97F4A7C15u64;
        let mut ngau = move |n: u64| {
            hat ^= hat << 13;
            hat ^= hat >> 7;
            hat ^= hat << 17;
            hat % n
        };
        for _ in 0..200 {
            let mut b = bang();
            let ban_dau: Vec<Sua> = b.trang.iter().map(|t| t.sua).collect();
            let mut so_buoc = 0;
            for _ in 0..30 {
                let ds: Vec<u32> = (0..10).filter(|_| ngau(3) == 0).collect();
                let n = match ngau(9) {
                    0 => b.gan(TC, &ds, &[], [Nhan::NoiDung, Nhan::Index, Nhan::Trang, Nhan::Split][ngau(4) as usize]),
                    1 => b.xoa(TC, &ds, &[]),
                    2 => b.khoi_phuc(TC, None, &[]),
                    3 => b.xoay(TC, &ds, &[], [90, 180][ngau(2) as usize], ngau(2) == 0),
                    4 => b.xoay_de_xuat(TC, &ds, &[]),
                    5 => b.ve_chieu_goc(TC, &ds, &[]),
                    6 => b.xac_nhan_xoay(TC, &ds, &[]),
                    7 => {
                        // nhóm khác ghi vào lịch sử riêng → không tính vào bước của Tất cả, nhưng đổi trạng thái
                        let k = ngau(10) as u32;
                        b.dat_tu_dong(k, NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: ngau(2) == 0, nghi_xoay: None });
                        0
                    }
                    _ => b.gan(TC, &ds, &[], Nhan::NoiDung),
                };
                so_buoc += (n > 0) as usize;
                assert_eq!(b.tong, dem_lai(&b));
            }
            for _ in 0..so_buoc {
                let kq = b.hoan_tac(TC).unwrap();
                assert_eq!(kq.bo_qua, 0);
            }
            assert!(b.hoan_tac(TC).is_none());
            assert_eq!(b.trang.iter().map(|t| t.sua).collect::<Vec<_>>(), ban_dau);
            assert_eq!(b.tong, dem_lai(&b));
        }
    }

    #[test]
    fn mo_lai_thu_muc_khoi_phuc_sua_tay_xoay_va_co_duyet() {
        use crate::tach_vb::cache_trang::TrangCache;
        use crate::tach_vb::chi_so_trang::ChiSoTrang;
        use crate::tach_vb::phan_tich::KetQuaTrang;
        use crate::tach_vb::phien::HoSoPhanTich;
        let kq = |v: f32| Some(KetQuaTrang { chi_so: Some(ChiSoTrang { dam: v, mau: 0.0 }), ..Default::default() });
        // 5 trang: Trắng, Nghi trắng, nội dung, nội dung, chưa phân tích
        let trang: Vec<TrangCache> = [kq(0.001), kq(0.02), kq(0.2), kq(0.2), None].into_iter().map(|ket_qua| TrangCache { ket_qua, ..Default::default() }).collect();
        let mut phien = PhienPhanTich { goc: "/x".into(), ho_so: vec![HoSoPhanTich { pdf: "/x/a.pdf".into(), tuong_doi: "a.pdf".into(), khoa: None, trang, loi: None }] };
        phien.ho_so[0].trang[3].trang_thai = CO_DA_XOA;
        let ch = CauHinhPhanTich { quet_ma: false, quet_xoay: false, ..Default::default() };
        // Không có file nhãn: toàn nhãn tự động
        let b = BangTrang::tu_phien(&phien, &ch, None, None);
        assert_eq!(b.trang.iter().map(|t| t.nhan()).collect::<Vec<_>>(), [Nhan::Trang, Nhan::NoiDung, Nhan::NoiDung, Nhan::NoiDung, Nhan::NoiDung]);
        assert!(b.trang[1].nghi_trang() && b.trang[3].da_xoa());
        // File nhãn: trang 1 người bỏ Trắng, trang 3 gán Index, trang 5 (chưa phân tích) Split; trang 2 đã duyệt → Trắng
        let mut nf = BangNhan::new();
        nf.entry("a.pdf".into()).or_default().extend([(2, Nhan::Trang), (3, Nhan::Index), (5, Nhan::Split)]);
        let mut xf = BangXoay::new();
        xf.entry("a.pdf".into()).or_default().extend([(3, Xoay { goc: 90, xac_nhan: false }), (4, Xoay { goc: 0, xac_nhan: true })]);
        let mut b = BangTrang::tu_phien(&phien, &ch, Some(&nf), Some(&xf));
        assert_eq!(b.trang.iter().map(|t| t.sua.tay).collect::<Vec<_>>(), [Some(Nhan::NoiDung), Some(Nhan::Trang), Some(Nhan::Index), None, Some(Nhan::Split)]);
        assert_eq!(b.bang_nhan(), nf, "khứ hồi file nhãn");
        assert_eq!(b.bang_xoay(), xf, "khứ hồi file xoay");
        // Đổi ngưỡng: trang 1 (đã sửa tay) giữ nguyên
        b.gan_lai_tu_dong(&phien, &CauHinhPhanTich { nguong_trang: 0.0, nguong_nghi: 0.0, ..ch.clone() });
        assert_eq!(b.trang[0].nhan(), Nhan::NoiDung);
        assert_eq!(b.tong, dem_lai(&b));
        // Ghi cờ về phiên
        b.duyet_nghi_trang(&[], &[], Nhan::NoiDung);
        b.xoa(TC, &[0], &[]);
        b.chep_trang_thai(&mut phien);
        assert_eq!(phien.ho_so[0].trang.iter().map(|t| t.trang_thai).collect::<Vec<_>>(), [CO_DA_XOA, 0, 0, CO_DA_XOA, 0]);
    }

    #[test]
    fn bang_nhan_ghi_nhan_dung() {
        let mut b = bang();
        b.gan(TC, &[6], &[], Nhan::Split);
        let n = b.bang_nhan();
        assert_eq!(n["a.pdf"].iter().map(|(&p, &n)| (p, n)).collect::<Vec<_>>(), [(2, Nhan::Index), (4, Nhan::Trang)]);
        assert_eq!(n["b/b.pdf"][&1], Nhan::Split);
        assert_eq!(b.trang[3].trang_thai_cache(), 0);
    }
}
