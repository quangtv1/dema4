//! Bộ vùng đang sửa của màn Xóa ghim: vùng chung thư mục (+ tuỳ chọn) và vùng riêng của PDF đang xem; thêm vùng vẽ,
//! bật / tắt vùng có sẵn, chọn / xoá, hoàn tác (chỉ trong phiên làm việc). Không đụng giao diện — test được.

use crate::tach_vb::kho::Id;
use crate::xoa_ghim::kho::{ThuMuc, VungFile};
use crate::xoa_ghim::vung::{co_san, ApTrang, Hcn, LoaiVung, ViTri, Vung};

/// Số bước hoàn tác giữ lại (XoaGhim: 79).
const BUOC_HOAN_TAC: usize = 100;
/// Vùng vẽ nhỏ hơn tỉ lệ này (mỗi chiều) coi như bấm nhầm.
const VE_TOI_THIEU: f64 = 0.005;
/// Mức xoá mặc định của vùng vẽ (= ngưỡng 5 cũ).
pub const MUC_VE: u8 = 52;

/// Vùng vẽ mới thuộc về đâu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhamVi {
    /// Chung cả thư mục (mọi PDF).
    ThuMuc,
    /// Riêng PDF đang xem (mọi trang theo lọc trang).
    File,
    /// Riêng trang đang xem của PDF đang xem.
    Trang,
}

/// Vùng đang chọn: (`true` = vùng riêng của file, chỉ số trong danh sách tương ứng).
pub type ChonVung = (bool, usize);

#[derive(Clone, Debug, PartialEq)]
struct Buoc {
    chung: Vec<Vung>,
    rieng: Vec<Vung>,
    id_file: Option<Id>,
}

#[derive(Debug, Default)]
pub struct BoVung {
    pub tm: ThuMuc,
    pub vf: VungFile,
    /// Id nội dung PDF đang xem (`None` = chưa có / không đọc được → không có vùng riêng).
    pub id_file: Option<Id>,
    /// Vùng đang chọn theo thứ tự chọn; phần tử cuối = vùng chính (thanh trượt / combo Vùng sửa khi chọn đúng 1).
    pub chon: Vec<ChonVung>,
    lich_su: Vec<Buoc>,
    /// Phần chung / riêng đã đổi, chưa ghi kho.
    pub doi_chung: bool,
    pub doi_rieng: bool,
}

impl BoVung {
    pub fn moi(tm: ThuMuc) -> Self {
        BoVung { tm, ..Default::default() }
    }

    /// Đổi sang PDF khác (vùng riêng của nó đã đọc từ kho). Lịch sử hoàn tác giữ (bước của file khác chỉ hoàn tác
    /// phần chung).
    pub fn doi_file(&mut self, id: Option<Id>, vf: VungFile) {
        self.id_file = id;
        self.vf = vf;
        self.chon.clear();
        self.doi_rieng = false;
    }

    /// Vùng áp cho PDF đang xem, đúng thứ tự xuất: chung rồi riêng.
    pub fn vung(&self) -> Vec<Vung> {
        self.tm.vung.iter().chain(&self.vf.vung).cloned().collect()
    }

    /// Ghi lại trạng thái trước khi đổi (để hoàn tác).
    pub fn ghi_buoc(&mut self) {
        let b = Buoc { chung: self.tm.vung.clone(), rieng: self.vf.vung.clone(), id_file: self.id_file };
        if self.lich_su.last() != Some(&b) {
            self.lich_su.push(b);
            if self.lich_su.len() > BUOC_HOAN_TAC {
                self.lich_su.remove(0);
            }
        }
    }

    pub fn co_hoan_tac(&self) -> bool {
        !self.lich_su.is_empty()
    }

    fn da_doi(&mut self, rieng: bool) {
        let bay_gio = crate::tach_vb::moc_sua::bay_gio_ms();
        if rieng {
            self.vf.luc = bay_gio;
            self.doi_rieng = true;
        } else {
            self.tm.luc = bay_gio;
            self.doi_chung = true;
        }
    }

    /// Đánh dấu phần chung (vùng có sẵn / tuỳ chọn) đã đổi.
    pub fn doi_chung(&mut self) {
        self.da_doi(false);
    }

    /// Hoàn tác 1 bước. Bước ghi ở PDF khác: chỉ lấy lại phần chung.
    pub fn hoan_tac(&mut self) -> bool {
        let Some(b) = self.lich_su.pop() else { return false };
        if self.tm.vung != b.chung {
            self.tm.vung = b.chung;
            self.da_doi(false);
        }
        if b.id_file == self.id_file && self.vf.vung != b.rieng {
            self.vf.vung = b.rieng;
            self.da_doi(true);
        }
        self.chon.clear();
        true
    }

    /// Bật / tắt vùng có sẵn (giữ thứ tự góc → cạnh như XoaGhim). Trả `true` nếu vừa bật.
    pub fn bat_tat(&mut self, v: ViTri) -> bool {
        self.ghi_buoc();
        self.chon.clear();
        let bat = if self.tm.vung.iter().any(|z| z.vi_tri == v) {
            self.tm.vung.retain(|z| z.vi_tri != v);
            false
        } else {
            // Sau vùng có sẵn cuối cùng đứng trước nó; không có thì trước vùng có sẵn đầu tiên
            let vi = match self.tm.vung.iter().rposition(|z| z.vi_tri.la_goc_canh() && thu_tu(z.vi_tri) < thu_tu(v)) {
                Some(i) => i + 1,
                None => self.tm.vung.iter().position(|z| z.vi_tri.la_goc_canh()).unwrap_or(0),
            };
            self.tm.vung.insert(vi, co_san(v));
            self.chon = vec![(false, vi)];
            true
        };
        self.da_doi(false);
        bat
    }

    /// Thêm vùng vẽ (tỉ lệ trang `x, y, rộng, cao`). Vùng quá nhỏ hoặc chưa có PDF (phạm vi riêng) → bỏ.
    pub fn them_ve(&mut self, (x, y, rong, cao): (f64, f64, f64, f64), loai: LoaiVung, pham_vi: PhamVi, ap: ApTrang, trang: u32) -> bool {
        let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        let (rong, cao) = (rong.min(1.0 - x), cao.min(1.0 - y));
        if rong < VE_TOI_THIEU || cao < VE_TOI_THIEU || (pham_vi != PhamVi::ThuMuc && self.id_file.is_none()) {
            return false;
        }
        let ap_trang = if pham_vi == PhamVi::Trang { ApTrang::MotTrang(trang) } else { ap };
        let v = Vung { vi_tri: ViTri::Tay, loai, ap_trang, x, y, rong, cao, rong_px: 0, cao_px: 0, muc: MUC_VE };
        self.ghi_buoc();
        let rieng = pham_vi != PhamVi::ThuMuc;
        let ds = if rieng { &mut self.vf.vung } else { &mut self.tm.vung };
        if ds.len() >= crate::xoa_ghim::kho::TOI_DA_VUNG {
            return false;
        }
        ds.push(v);
        self.chon = vec![(rieng, ds.len() - 1)];
        self.da_doi(rieng);
        true
    }

    /// Vùng chính đang chọn (phần tử chọn sau cùng).
    pub fn chinh(&self) -> Option<ChonVung> {
        self.chon.last().copied()
    }

    pub fn lay(&self, (rieng, i): ChonVung) -> Option<&Vung> {
        if rieng { self.vf.vung.get(i) } else { self.tm.vung.get(i) }
    }

    pub fn lay_mut(&mut self, (rieng, i): ChonVung) -> Option<&mut Vung> {
        if rieng { self.vf.vung.get_mut(i) } else { self.tm.vung.get_mut(i) }
    }

    pub fn vung_chon(&self) -> Option<&Vung> {
        self.lay(self.chinh()?)
    }

    pub fn vung_chon_mut(&mut self) -> Option<&mut Vung> {
        let c = self.chinh()?;
        self.lay_mut(c)
    }

    pub fn dang_chon(&self, c: ChonVung) -> bool {
        self.chon.contains(&c)
    }

    pub fn chon_mot(&mut self, c: Option<ChonVung>) {
        self.chon = c.into_iter().collect();
    }

    /// Shift + bấm: thêm / bỏ khỏi nhóm chọn.
    pub fn chon_dao(&mut self, c: ChonVung) {
        if let Some(i) = self.chon.iter().position(|x| *x == c) {
            self.chon.remove(i);
        } else {
            self.chon.push(c);
        }
    }

    pub fn chon_them(&mut self, ds: impl IntoIterator<Item = ChonVung>) {
        for c in ds {
            if !self.chon.contains(&c) {
                self.chon.push(c);
            }
        }
    }

    /// Mọi vùng (chung + riêng).
    pub fn tat_ca(&self) -> Vec<ChonVung> {
        (0..self.tm.vung.len()).map(|i| (false, i)).chain((0..self.vf.vung.len()).map(|i| (true, i))).collect()
    }

    /// Vùng đang chọn đã sửa (ngưỡng, loại…).
    pub fn sua_chon(&mut self) {
        for (rieng, _) in self.chon.clone() {
            self.da_doi(rieng);
        }
    }

    /// Xoá các vùng `ds` (chỉ số giảm dần từng danh sách để không lệch). Trả số vùng đã xoá.
    pub fn xoa(&mut self, ds: &[ChonVung]) -> usize {
        if ds.is_empty() {
            return 0;
        }
        self.ghi_buoc();
        let mut n = 0;
        for rieng in [false, true] {
            let mut cs: Vec<usize> = ds.iter().filter(|c| c.0 == rieng).map(|c| c.1).collect();
            cs.sort_unstable_by(|a, b| b.cmp(a));
            cs.dedup();
            let list = if rieng { &mut self.vf.vung } else { &mut self.tm.vung };
            let truoc = list.len();
            for i in cs {
                if i < list.len() {
                    list.remove(i);
                }
            }
            if list.len() != truoc {
                n += truoc - list.len();
                self.da_doi(rieng);
            }
        }
        self.chon.retain(|c| !ds.contains(c));
        self.chon.clear();
        n
    }


    /// Vùng vẽ Tùy biến trong `ds` (bỏ góc / cạnh có sẵn).
    pub fn chi_vung_ve(&self, ds: &[ChonVung]) -> Vec<ChonVung> {
        ds.iter().copied().filter(|c| self.lay(*c).is_some_and(|z| z.vi_tri == ViTri::Tay)).collect()
    }

    /// Mọi vùng vẽ Tùy biến (chung + riêng).
    pub fn moi_vung_ve(&self) -> Vec<ChonVung> {
        self.chi_vung_ve(&self.tat_ca())
    }


    /// Đổi Xoá ↔ Bảo vệ (Vô đối → Bảo vệ).
    pub fn doi_loai(&mut self, c: ChonVung) {
        self.ghi_buoc();
        if let Some(z) = self.lay_mut(c) {
            z.loai = if z.loai == LoaiVung::BaoVe { LoaiVung::Xoa } else { LoaiVung::BaoVe };
        }
        self.da_doi(c.0);
    }

    /// Nhân bản vùng (bản sao là vùng vẽ, lệch 3% trang, cùng danh sách); chọn bản sao.
    pub fn nhan_ban(&mut self, c: ChonVung, (w, h): (i64, i64), dpi: f64) -> bool {
        let Some(z) = self.lay(c).cloned() else { return false };
        // Vùng có sẵn: đổi sang toạ độ tỉ lệ theo khung đang thấy
        let (x, y, rong, cao) = match z.khung_xoa(w, h, dpi) {
            Some(k) if z.vi_tri != ViTri::Tay => (k.x1 as f64 / w as f64, k.y1 as f64 / h as f64, k.rong() as f64 / w as f64, k.cao() as f64 / h as f64),
            _ => (z.x, z.y, z.rong, z.cao),
        };
        let (x, y) = ((x + 0.03).min(1.0 - rong).max(0.0), (y + 0.03).min(1.0 - cao).max(0.0));
        let moi = Vung { vi_tri: ViTri::Tay, x, y, rong, cao, rong_px: 0, cao_px: 0, ..z };
        let rieng = c.0 || self.id_file.is_some();
        if rieng && self.id_file.is_none() {
            return false;
        }
        self.ghi_buoc();
        let ds = if rieng { &mut self.vf.vung } else { &mut self.tm.vung };
        if ds.len() >= crate::xoa_ghim::kho::TOI_DA_VUNG {
            return false;
        }
        ds.push(moi);
        self.chon = vec![(rieng, ds.len() - 1)];
        self.da_doi(rieng);
        true
    }

    /// Đổi phạm vi trang của vùng; `vo_doi` = vùng xoá thành vô đối (chỉ vùng vẽ xoá).
    pub fn doi_ap_trang(&mut self, c: ChonVung, ap: ApTrang, vo_doi: bool) -> bool {
        let Some(z) = self.lay(c) else { return false };
        let la_ve = z.vi_tri == ViTri::Tay;
        let loai_moi = match z.loai {
            LoaiVung::Xoa | LoaiVung::VoDoi if la_ve => if vo_doi { LoaiVung::VoDoi } else { LoaiVung::Xoa },
            l => l,
        };
        if z.ap_trang == ap && z.loai == loai_moi {
            return false;
        }
        self.ghi_buoc();
        if let Some(z) = self.lay_mut(c) {
            z.ap_trang = ap;
            z.loai = loai_moi;
        }
        self.da_doi(c.0);
        true
    }

    /// Vùng ở điểm (toạ độ điểm ảnh trên ảnh trang `w × h` vẽ ở `dpi`) của trang `trang`; vùng riêng / vẽ sau ưu tiên.
    pub fn tim(&self, trang: u32, (px, py): (i64, i64), (w, h): (i64, i64), dpi: f64) -> Option<ChonVung> {
        let trong = |z: &Vung| z.ap_trang.ap_dung(trang) && z.khung_xoa(w, h, dpi).is_some_and(|k: Hcn| px >= k.x1 && px < k.x2 && py >= k.y1 && py < k.y2);
        if let Some(i) = self.vf.vung.iter().rposition(trong) {
            return Some((true, i));
        }
        self.tm.vung.iter().rposition(trong).map(|i| (false, i))
    }

    /// Vùng áp cho trang `trang` có khung (điểm ảnh) cắt `hcn` (`cat = true`, Crossing) hoặc nằm trọn trong (`false`,
    /// Window).
    pub fn quet(&self, trang: u32, hcn: Hcn, cat: bool, (w, h): (i64, i64), dpi: f64) -> Vec<ChonVung> {
        self.tat_ca()
            .into_iter()
            .filter(|c| {
                let Some(z) = self.lay(*c) else { return false };
                if !z.ap_trang.ap_dung(trang) {
                    return false;
                }
                let Some(k) = z.khung_xoa(w, h, dpi) else { return false };
                if cat {
                    k.x1 < hcn.x2 && k.x2 > hcn.x1 && k.y1 < hcn.y2 && k.y2 > hcn.y1
                } else {
                    k.x1 >= hcn.x1 && k.y1 >= hcn.y1 && k.x2 <= hcn.x2 && k.y2 <= hcn.y2
                }
            })
            .collect()
    }

    /// Số vùng chung / riêng (bộ đếm).
    pub fn dem(&self) -> (usize, usize) {
        (self.tm.vung.len(), self.vf.vung.len())
    }
}

/// Phần của vùng đang kéo: thân (di chuyển) hoặc các cạnh (đổi cỡ).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Canh {
    pub trai: bool,
    pub tren: bool,
    pub phai: bool,
    pub duoi: bool,
}

impl Canh {
    pub fn than(&self) -> bool {
        !(self.trai || self.tren || self.phai || self.duoi)
    }
}

/// Cạnh kéo được của vùng: vùng vẽ — mọi cạnh + thân; góc / cạnh có sẵn — chỉ cạnh phía trong trang (đổi kích thước,
/// vị trí bám mép).
pub fn canh_keo_duoc(v: ViTri) -> Canh {
    let c = |trai, tren, phai, duoi| Canh { trai, tren, phai, duoi };
    match v {
        ViTri::Tay => c(true, true, true, true),
        ViTri::GocTrenTrai => c(false, false, true, true),
        ViTri::GocTrenPhai => c(true, false, false, true),
        ViTri::GocDuoiTrai => c(false, true, true, false),
        ViTri::GocDuoiPhai => c(true, true, false, false),
        ViTri::CanhTrai => c(false, false, true, false),
        ViTri::CanhPhai => c(true, false, false, false),
        ViTri::CanhTren => c(false, false, false, true),
        ViTri::CanhDuoi => c(false, true, false, false),
    }
}

/// Vùng sau khi kéo `canh` của `goc` một đoạn `(dx, dy)` điểm ảnh trên ảnh trang `w × h` vẽ ở `dpi`.
/// Vùng vẽ: đổi tỉ lệ, giữ trong trang, cỡ tối thiểu; có sẵn: đổi kích thước điểm ảnh @120 dpi (tối thiểu 5).
pub fn keo_vung(goc: &Vung, canh: Canh, (dx, dy): (f64, f64), (w, h): (f64, f64), dpi: f64) -> Vung {
    let mut v = goc.clone();
    if goc.vi_tri == ViTri::Tay {
        let (fx, fy) = (dx / w, dy / h);
        let min = VE_TOI_THIEU;
        if canh.than() {
            v.x = (goc.x + fx).clamp(0.0, 1.0 - goc.rong);
            v.y = (goc.y + fy).clamp(0.0, 1.0 - goc.cao);
            return v;
        }
        let (mut x1, mut y1, mut x2, mut y2) = (goc.x, goc.y, goc.x + goc.rong, goc.y + goc.cao);
        if canh.trai {
            x1 = (x1 + fx).clamp(0.0, x2 - min);
        }
        if canh.phai {
            x2 = (x2 + fx).clamp(x1 + min, 1.0);
        }
        if canh.tren {
            y1 = (y1 + fy).clamp(0.0, y2 - min);
        }
        if canh.duoi {
            y2 = (y2 + fy).clamp(y1 + min, 1.0);
        }
        (v.x, v.y, v.rong, v.cao) = (x1, y1, x2 - x1, y2 - y1);
        return v;
    }
    let doi = |px: u32, d: f64| ((px as f64 + d * crate::xoa_ghim::vung::DPI_GOC / dpi).round().clamp(5.0, 10_000.0)) as u32;
    let duoc = canh_keo_duoc(goc.vi_tri);
    if canh.phai && duoc.phai {
        v.rong_px = doi(goc.rong_px, dx);
    }
    if canh.trai && duoc.trai {
        v.rong_px = doi(goc.rong_px, -dx);
    }
    if canh.duoi && duoc.duoi {
        v.cao_px = doi(goc.cao_px, dy);
    }
    if canh.tren && duoc.tren {
        v.cao_px = doi(goc.cao_px, -dy);
    }
    v
}

fn thu_tu(v: ViTri) -> usize {
    ViTri::CO_SAN.iter().position(|x| *x == v).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bo() -> BoVung {
        let mut b = BoVung::moi(ThuMuc { vung: vec![co_san(ViTri::GocTrenTrai), co_san(ViTri::GocTrenPhai)], ..Default::default() });
        b.doi_file(Some((1, 1)), VungFile::default());
        b
    }

    #[test]
    fn bat_tat_co_san_giu_thu_tu_va_hoan_tac() {
        let mut b = bo();
        assert!(b.bat_tat(ViTri::CanhTrai));
        assert!(b.them_ve((0.1, 0.1, 0.2, 0.2), LoaiVung::Xoa, PhamVi::ThuMuc, ApTrang::TatCa, 0));
        assert!(b.bat_tat(ViTri::GocDuoiTrai));
        let thu: Vec<ViTri> = b.tm.vung.iter().map(|z| z.vi_tri).collect();
        assert_eq!(thu, vec![ViTri::GocTrenTrai, ViTri::GocTrenPhai, ViTri::GocDuoiTrai, ViTri::CanhTrai, ViTri::Tay], "có sẵn theo thứ tự góc → cạnh, vùng vẽ sau");
        assert!(!b.bat_tat(ViTri::GocTrenTrai));
        assert!(b.doi_chung);
        for _ in 0..4 {
            assert!(b.hoan_tac());
        }
        assert_eq!(b.tm.vung.len(), 2);
        assert!(!b.hoan_tac());
    }

    #[test]
    fn ve_theo_pham_vi_chon_xoa_va_tim() {
        let mut b = bo();
        assert!(b.them_ve((0.5, 0.5, 0.2, 0.1), LoaiVung::BaoVe, PhamVi::Trang, ApTrang::TatCa, 3));
        assert_eq!(b.vf.vung[0].ap_trang, ApTrang::MotTrang(3), "riêng trang");
        assert_eq!(b.chon, vec![(true, 0)]);
        assert!(b.them_ve((0.0, 0.9, 0.3, 0.1), LoaiVung::VoDoi, PhamVi::File, ApTrang::Chan, 3));
        assert_eq!(b.vf.vung[1].ap_trang, ApTrang::Chan);
        assert!(!b.them_ve((0.1, 0.1, 0.001, 0.2), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0), "quá nhỏ");
        assert_eq!(b.vung().len(), 4, "chung rồi riêng");
        // Tìm theo điểm: trang 3 ở giữa vùng bảo vệ; trang 2 không có vùng riêng trang đó
        assert_eq!(b.tim(3, (600, 550), (1000, 1000), 120.0), Some((true, 0)));
        assert_eq!(b.tim(2, (600, 550), (1000, 1000), 120.0), None);
        assert_eq!(b.tim(0, (5, 5), (1000, 1000), 120.0), Some((false, 0)), "góc có sẵn");
        b.chon = vec![(true, 0)];
        assert_eq!(b.xoa(&b.chon.clone()), 1);
        assert_eq!(b.vf.vung.len(), 1);
        assert!(b.doi_rieng);
        assert!(b.hoan_tac());
        assert_eq!(b.vf.vung.len(), 2);
        assert_eq!(b.xoa(&b.moi_vung_ve()), 2);
        assert!(b.vf.vung.is_empty());
    }

    #[test]
    fn chon_nhieu_quet_xoa_vung_ve_giu_goc_canh() {
        let mut b = bo();
        assert!(b.them_ve((0.4, 0.4, 0.1, 0.1), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0));
        assert!(b.them_ve((0.6, 0.6, 0.1, 0.1), LoaiVung::BaoVe, PhamVi::File, ApTrang::TatCa, 0));
        // Window bao trọn 2 vùng vẽ; Crossing chạm 1
        let w = b.quet(0, Hcn { x1: 350, y1: 350, x2: 800, y2: 800 }, false, (1000, 1000), 120.0);
        assert_eq!(w, vec![(true, 0), (true, 1)]);
        let c = b.quet(0, Hcn { x1: 450, y1: 450, x2: 460, y2: 460 }, true, (1000, 1000), 120.0);
        assert_eq!(c, vec![(true, 0)]);
        b.chon = b.tat_ca();
        let ve = b.chi_vung_ve(&b.chon.clone());
        assert_eq!(ve.len(), 2, "góc có sẵn không tính");
        assert_eq!(b.xoa(&ve), 2);
        assert_eq!(b.tm.vung.len(), 2, "góc giữ nguyên");
        assert!(b.hoan_tac());
        assert_eq!(b.vf.vung.len(), 2);
        b.chon_dao((true, 1));
        b.chon_dao((true, 0));
        b.chon_dao((true, 1));
        assert_eq!(b.chon, vec![(true, 0)]);
    }

    #[test]
    fn doi_loai_pham_vi_vo_doi_va_nhan_ban() {
        let mut b = bo();
        assert!(b.them_ve((0.4, 0.4, 0.1, 0.1), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0));
        assert!(b.doi_ap_trang((true, 0), ApTrang::MotTrang(2), true));
        assert_eq!((b.vf.vung[0].loai, b.vf.vung[0].ap_trang), (LoaiVung::VoDoi, ApTrang::MotTrang(2)));
        assert!(b.doi_ap_trang((true, 0), ApTrang::Le, false));
        assert_eq!(b.vf.vung[0].loai, LoaiVung::Xoa);
        assert!(!b.doi_ap_trang((false, 0), ApTrang::TatCa, true), "góc: không đổi gì");
        b.doi_loai((true, 0));
        assert_eq!(b.vf.vung[0].loai, LoaiVung::BaoVe);
        assert!(b.nhan_ban((false, 0), (1000, 1000), 120.0), "nhân bản góc → vùng vẽ");
        assert_eq!(b.vf.vung.last().unwrap().vi_tri, ViTri::Tay);
        assert_eq!(b.chon, vec![(true, 1)]);
    }

    #[test]
    fn keo_than_canh_vung_ve_va_canh_trong_cua_vung_co_san() {
        let tay = Vung { vi_tri: ViTri::Tay, loai: LoaiVung::Xoa, ap_trang: ApTrang::TatCa, x: 0.1, y: 0.1, rong: 0.2, cao: 0.2, rong_px: 0, cao_px: 0, muc: 52 };
        let co = (1000.0, 2000.0);
        let than = keo_vung(&tay, Canh::default(), (100.0, 200.0), co, 300.0);
        assert!((than.x - 0.2).abs() < 1e-9 && (than.y - 0.2).abs() < 1e-9 && than.rong == tay.rong);
        let ra_ngoai = keo_vung(&tay, Canh::default(), (5000.0, 0.0), co, 300.0);
        assert!((ra_ngoai.x + ra_ngoai.rong - 1.0).abs() < 1e-9, "không ra khỏi trang");
        let phai_duoi = keo_vung(&tay, Canh { phai: true, duoi: true, ..Default::default() }, (100.0, -1000.0), co, 300.0);
        assert!((phai_duoi.rong - 0.3).abs() < 1e-9 && phai_duoi.cao >= VE_TOI_THIEU - 1e-12 && phai_duoi.y == tay.y);
        // Góc trên phải: kéo cạnh trái sang trái 250 điểm ảnh @300 dpi = +100 @120
        let g = co_san(ViTri::GocTrenPhai);
        let g2 = keo_vung(&g, Canh { trai: true, ..Default::default() }, (-250.0, 0.0), co, 300.0);
        assert_eq!((g2.rong_px, g2.cao_px), (230, 130));
        let g3 = keo_vung(&g, Canh { phai: true, tren: true, ..Default::default() }, (500.0, 500.0), co, 300.0);
        assert_eq!(g3, g, "cạnh bám mép không kéo được");
        let c = keo_vung(&co_san(ViTri::CanhDuoi), Canh { tren: true, ..Default::default() }, (0.0, 10_000.0), co, 300.0);
        assert_eq!(c.cao_px, 5, "tối thiểu 5");
    }

    #[test]
    fn khong_co_pdf_thi_khong_ve_rieng_hoan_tac_file_khac_chi_lay_phan_chung() {
        let mut b = bo();
        assert!(b.them_ve((0.1, 0.1, 0.2, 0.2), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0));
        b.bat_tat(ViTri::CanhDuoi);
        b.doi_file(None, VungFile::default());
        assert!(!b.them_ve((0.1, 0.1, 0.2, 0.2), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0));
        b.doi_file(Some((2, 2)), VungFile::default());
        assert!(b.hoan_tac(), "bước bật cạnh dưới");
        assert!(!b.tm.vung.iter().any(|z| z.vi_tri == ViTri::CanhDuoi));
        assert!(b.vf.vung.is_empty(), "không đem vùng riêng của PDF trước sang PDF này");
    }
}
