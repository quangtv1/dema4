//! Bộ vùng đang sửa của màn Xóa ghim: vùng chung thư mục (+ tuỳ chọn) và vùng riêng của PDF đang xem; thêm vùng vẽ,
//! bật / tắt vùng có sẵn, chọn / xoá, hoàn tác (chỉ trong phiên làm việc). Không đụng giao diện — test được.

use crate::tach_vb::kho::Id;
use crate::xoa_ghim::kho::{ThuMuc, VungFile};
use crate::xoa_ghim::vung::{co_san, ApTrang, Hcn, LoaiVung, ViTri, Vung};

/// Số bước hoàn tác giữ lại (XoaGhim: 79).
const BUOC_HOAN_TAC: usize = 100;
/// Vùng vẽ nhỏ hơn tỉ lệ này (mỗi chiều) coi như bấm nhầm.
const VE_TOI_THIEU: f64 = 0.005;
/// Ngưỡng mặc định của vùng vẽ tay.
pub const NGUONG_VE: u8 = 5;

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
    pub chon: Option<ChonVung>,
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
        self.chon = None;
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
        self.chon = None;
        true
    }

    /// Bật / tắt vùng có sẵn (giữ thứ tự góc → cạnh như XoaGhim). Trả `true` nếu vừa bật.
    pub fn bat_tat(&mut self, v: ViTri) -> bool {
        self.ghi_buoc();
        self.chon = None;
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
        let v = Vung { vi_tri: ViTri::Tay, loai, ap_trang, x, y, rong, cao, rong_px: 0, cao_px: 0, nguong: NGUONG_VE };
        self.ghi_buoc();
        let rieng = pham_vi != PhamVi::ThuMuc;
        let ds = if rieng { &mut self.vf.vung } else { &mut self.tm.vung };
        if ds.len() >= crate::xoa_ghim::kho::TOI_DA_VUNG {
            return false;
        }
        ds.push(v);
        self.chon = Some((rieng, ds.len() - 1));
        self.da_doi(rieng);
        true
    }

    pub fn vung_chon(&self) -> Option<&Vung> {
        let (rieng, i) = self.chon?;
        if rieng { self.vf.vung.get(i) } else { self.tm.vung.get(i) }
    }

    pub fn vung_chon_mut(&mut self) -> Option<&mut Vung> {
        let (rieng, i) = self.chon?;
        if rieng { self.vf.vung.get_mut(i) } else { self.tm.vung.get_mut(i) }
    }

    /// Vùng đang chọn đã sửa (ngưỡng, loại…).
    pub fn sua_chon(&mut self) {
        if let Some((rieng, _)) = self.chon {
            self.da_doi(rieng);
        }
    }

    /// Xoá vùng đang chọn.
    pub fn xoa_chon(&mut self) -> bool {
        let Some((rieng, i)) = self.chon.take() else { return false };
        self.ghi_buoc();
        let ds = if rieng { &mut self.vf.vung } else { &mut self.tm.vung };
        if i >= ds.len() {
            return false;
        }
        ds.remove(i);
        self.da_doi(rieng);
        true
    }

    /// Xoá mọi vùng vẽ tay của phần chung (`rieng = false`) hoặc mọi vùng riêng của PDF đang xem.
    pub fn xoa_het_ve(&mut self, rieng: bool) -> bool {
        let co = if rieng { !self.vf.vung.is_empty() } else { self.tm.vung.iter().any(|z| !z.vi_tri.la_goc_canh()) };
        if !co {
            return false;
        }
        self.ghi_buoc();
        if rieng {
            self.vf.vung.clear();
        } else {
            self.tm.vung.retain(|z| z.vi_tri.la_goc_canh());
        }
        self.chon = None;
        self.da_doi(rieng);
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

    /// Số vùng chung / riêng (bộ đếm).
    pub fn dem(&self) -> (usize, usize) {
        (self.tm.vung.len(), self.vf.vung.len())
    }
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
        assert_eq!(b.chon, Some((true, 0)));
        assert!(b.them_ve((0.0, 0.9, 0.3, 0.1), LoaiVung::VoDoi, PhamVi::File, ApTrang::Chan, 3));
        assert_eq!(b.vf.vung[1].ap_trang, ApTrang::Chan);
        assert!(!b.them_ve((0.1, 0.1, 0.001, 0.2), LoaiVung::Xoa, PhamVi::File, ApTrang::TatCa, 0), "quá nhỏ");
        assert_eq!(b.vung().len(), 4, "chung rồi riêng");
        // Tìm theo điểm: trang 3 ở giữa vùng bảo vệ; trang 2 không có vùng riêng trang đó
        assert_eq!(b.tim(3, (600, 550), (1000, 1000), 120.0), Some((true, 0)));
        assert_eq!(b.tim(2, (600, 550), (1000, 1000), 120.0), None);
        assert_eq!(b.tim(0, (5, 5), (1000, 1000), 120.0), Some((false, 0)), "góc có sẵn");
        b.chon = Some((true, 0));
        assert!(b.xoa_chon());
        assert_eq!(b.vf.vung.len(), 1);
        assert!(b.doi_rieng);
        assert!(b.hoan_tac());
        assert_eq!(b.vf.vung.len(), 2);
        assert!(b.xoa_het_ve(true));
        assert!(b.vf.vung.is_empty());
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
