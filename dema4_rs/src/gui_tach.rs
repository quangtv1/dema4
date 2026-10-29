//! Màn "Tách văn bản": hàng thư mục + Phân tích nền, tab Hồ sơ (bảng hồ sơ, lưới gộp mọi trang để duyệt và
//! gán nhãn kiểu Explorer), dòng đáy. Đặc tả = `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`.

pub mod bang_trang;
pub mod bieu_tuong;
pub mod chon;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use egui_extras::{Column, TableBuilder};

use crate::cau_hinh::{CauHinh, LuuNhan};
use crate::gui::{nut, nut_mau, so_nguyen, ChuDam, CHU, CHU_MO, CHU_PHU, DO, NEN, VIEN_NHAT, VIEN_TAB_CHON, XAM_TIEN_DO, XANH_COBALT, XANH_LA};
use crate::tach_vb::hang_doi::{self, HangDoi, SuKienPhanTich, ViecTrang};
use crate::tach_vb::nhan::{self, Nhan};
use crate::tach_vb::phan_tich::{gan_nhan, CauHinhPhanTich};
use crate::tach_vb::phien::{thu_muc_an, PhienPhanTich};
use crate::tach_vb::cache_trang;
use bang_trang::{ke_tiep, ten_nhan, BangTrang, EnterXoay, Loc, Nhom, Sap};
use bieu_tuong::{muc_menu, BieuTuong};
use chon::{BoChon, BoCucLuoi, PhimDiChuyen, PhimPhu};

/// 7 nấc cỡ ảnh (bề ngang ô, px).
pub const NAC_ANH: [f32; 7] = [56.0, 76.0, 100.0, 128.0, 160.0, 196.0, 230.0];
const KHE_X: f32 = 28.0;
const KHE_Y: f32 = 22.0;
/// 2 dòng chữ dưới ảnh.
const CAO_CHU: f32 = 30.0;
const LE: f32 = 12.0;
const SU_KIEN_MOI_KHUNG: usize = 5_000;

const NEN_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF3, 0xFF);
const NEN_CHON: Color32 = Color32::from_rgb(0xCC, 0xE8, 0xFF);
const VIEN_CHON: Color32 = Color32::from_rgb(0x99, 0xD1, 0xFF);
const XANH_NUT: Color32 = Color32::from_rgb(0x00, 0x67, 0xC0);
const CAM: Color32 = Color32::from_rgb(0xE0, 0x7A, 0x10);
const VANG: Color32 = Color32::from_rgb(0xD4, 0xA0, 0x00);
const NEN_TRANG_NHAN: Color32 = Color32::from_rgb(0xEA, 0xF3, 0xFC);
const NEN_ANH_CHO: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    HoSo,
    VanBan,
}

enum Chay {
    /// Đang tìm PDF + đọc dung lượng / cache.
    Mo { nhan: Receiver<PhienPhanTich>, dung: Arc<AtomicBool> },
    PhanTich { nhan: Receiver<SuKienPhanTich>, hang: Arc<HangDoi>, tong: usize, xong: usize, loi: usize, bat_dau: Instant },
}

/// Việc người dùng bấm trong lúc vẽ (thực hiện sau khi vẽ xong để khỏi vướng mượn).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Viec {
    Gan(Nhan),
    Xoa,
    KhoiPhuc,
    Xoay(u16),
    XoayDeXuat,
    VeGoc,
    XacNhanXoay,
    DuyetNghiTrang(Nhan),
    HoanTac,
    LamLai,
    ChonTat,
    BoChon,
    DatLoc(Loc),
    DatSap(Sap),
    CoAnh(i32),
    LuuNhan,
    /// Bấm nhãn góc "Trắng?" / góc đề xuất ngoài bộ lọc → mở bộ lọc, chọn sẵn trang.
    MoLoc(Loc, u32),
}

pub struct ManTach {
    pub thu_muc: String,
    phien: Option<PhienPhanTich>,
    pub bang: Option<BangTrang>,
    chay: Option<Chay>,
    pub tab: Tab,
    pub chon: BoChon,
    pub loc: Loc,
    /// Mục đang chọn trong menu lọc (nhớ khi đang xem Điểm cắt).
    loc_menu: Loc,
    pub ho_so_loc: Option<u32>,
    pub sap: Sap,
    thu_tu: Vec<u32>,
    khoa_thu_tu: Option<(Loc, Option<u32>, Sap, u64)>,
    ten_hien: Vec<String>,
    bao: Option<(String, Instant)>,
    trang_thai: String,
    so_cot: usize,
    /// Kéo khung: điểm bắt đầu (toạ độ nội dung lưới).
    keo_tu: Option<Pos2>,
    cuon_toi: Option<u32>,
    dang_xem: Vec<u32>,
    vb_tong: Option<(u64, usize)>,
    hoi_phan_tich_lai: bool,
    /// Cấu hình đổi trong màn (độ rộng cột, cỡ ảnh…) → app ghi `sm-dema4.cfg`.
    pub cfg_doi: bool,
}

impl Default for ManTach {
    fn default() -> Self {
        Self {
            thu_muc: String::new(),
            phien: None,
            bang: None,
            chay: None,
            tab: Tab::HoSo,
            chon: BoChon::default(),
            loc: Loc::TatCa,
            loc_menu: Loc::TatCa,
            ho_so_loc: None,
            sap: Sap::ThuTu,
            thu_tu: Vec::new(),
            khoa_thu_tu: None,
            ten_hien: Vec::new(),
            bao: None,
            trang_thai: String::new(),
            so_cot: 1,
            keo_tu: None,
            cuon_toi: None,
            dang_xem: Vec::new(),
            vb_tong: None,
            hoi_phan_tich_lai: false,
            cfg_doi: false,
        }
    }
}

fn ten_loc(l: Loc) -> &'static str {
    match l {
        Loc::TatCa => "Tất cả",
        Loc::Index => "Index",
        Loc::Trang => "Trắng",
        Loc::NghiTrang => "Nghi trắng",
        Loc::NghiXoay => "Nghi xoay",
        Loc::DiemCat => "Điểm cắt",
    }
}

/// Tên hiển thị hồ sơ: tên file bỏ `.pdf`; trùng tên ở thư mục khác → kèm thư mục cha.
pub fn ten_hien_ho_so(duong_dan: &[String]) -> Vec<String> {
    let ten = |p: &str| -> String {
        let f = p.rsplit('/').next().unwrap_or(p);
        f.strip_suffix(".pdf").or_else(|| f.strip_suffix(".PDF")).unwrap_or(f).to_string()
    };
    let mut dem = std::collections::HashMap::new();
    for p in duong_dan {
        *dem.entry(ten(p)).or_insert(0) += 1;
    }
    duong_dan
        .iter()
        .map(|p| {
            let t = ten(p);
            if dem[&t] > 1 {
                let mut phan = p.rsplit('/');
                phan.next();
                phan.next().map_or(t.clone(), |cha| format!("{cha}/{t}"))
            } else {
                t
            }
        })
        .collect()
}

/// Ghi qua file tạm rồi đổi tên: tắt app giữa chừng không làm hỏng file nhãn.
fn ghi_an_toan(p: &Path, noi_dung: &str) -> std::io::Result<()> {
    if let Some(cha) = p.parent() {
        std::fs::create_dir_all(cha)?;
    }
    let tam = p.with_extension("tmp");
    std::fs::write(&tam, noi_dung)?;
    std::fs::rename(&tam, p)
}

impl ManTach {
    pub fn thu_tu_hien(&self) -> &[u32] {
        &self.thu_tu
    }

    pub fn dang_chay(&self) -> bool {
        self.chay.is_some()
    }

    pub fn da_phan_tich(&self) -> bool {
        self.bang.is_some()
    }

    fn bao(&mut self, s: impl Into<String>) {
        self.bao = Some((s.into(), Instant::now()));
    }

    fn file_nhan(&self) -> PathBuf {
        thu_muc_an(Path::new(&self.thu_muc)).join("index_and_blank.txt")
    }

    fn file_xoay(&self) -> PathBuf {
        thu_muc_an(Path::new(&self.thu_muc)).join("xoay_trang.txt")
    }

    /// Có sửa tay (nhãn, góc, cờ duyệt) → Phân tích lại phải hỏi.
    fn co_sua_tay(&self) -> bool {
        self.bang.as_ref().is_some_and(|b| b.trang.iter().any(|t| t.sua != Default::default()))
    }

    // ---- Phân tích ----

    pub(crate) fn bam_phan_tich(&mut self, ch: &CauHinhPhanTich) {
        if let Some(chay) = &self.chay {
            match chay {
                Chay::Mo { dung, .. } => dung.store(true, std::sync::atomic::Ordering::Relaxed),
                Chay::PhanTich { hang, .. } => hang.dung(),
            }
            return;
        }
        if self.da_phan_tich() && self.co_sua_tay() {
            self.hoi_phan_tich_lai = true;
            return;
        }
        let lam_lai = self.da_phan_tich();
        self.bat_dau(ch, lam_lai);
    }

    /// Mở thư mục (dùng cache + nhãn đã lưu) hoặc làm lại từ đầu (bỏ cache, nhãn, góc xoay).
    fn bat_dau(&mut self, ch: &CauHinhPhanTich, lam_lai: bool) {
        let goc = PathBuf::from(self.thu_muc.trim());
        if !goc.is_dir() {
            self.trang_thai = "Thư mục không tồn tại".into();
            return;
        }
        if lam_lai {
            let _ = cache_trang::xoa_het(&thu_muc_an(&goc).join("cache"));
            for f in [self.file_nhan(), self.file_xoay()] {
                if f.exists() {
                    let _ = ghi_an_toan(&f, "");
                }
            }
        }
        self.bang = None;
        self.phien = None;
        self.chon = BoChon::default();
        self.loc = Loc::TatCa;
        self.loc_menu = Loc::TatCa;
        self.ho_so_loc = None;
        self.khoa_thu_tu = None;
        self.vb_tong = None;
        self.trang_thai = "Đang đọc thư mục…".into();
        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let dung2 = dung.clone();
        let _ = ch;
        std::thread::spawn(move || {
            let _ = gui.send(PhienPhanTich::mo(&goc, &dung2));
        });
        self.chay = Some(Chay::Mo { nhan, dung });
    }

    fn da_mo(&mut self, phien: PhienPhanTich, ch: &CauHinhPhanTich) {
        let so_trang_cua = |p: &str| phien.ho_so.iter().find(|h| h.tuong_doi == p).map(|h| h.trang.len());
        let nf = std::fs::read_to_string(self.file_nhan()).ok().and_then(|s| nhan::doc_file_nhan(&s, &so_trang_cua).ok()).map(|(b, _)| b);
        let xf = std::fs::read_to_string(self.file_xoay()).ok().and_then(|s| nhan::doc_file_xoay(&s, &so_trang_cua).ok());
        let bang = BangTrang::tu_phien(&phien, ch, nf.as_ref(), xf.as_ref());
        self.ten_hien = ten_hien_ho_so(&bang.ho_so);
        let viec = phien.viec_con_thieu(ch);
        let tong = viec.len();
        self.bang = Some(bang);
        if tong == 0 {
            self.phien = Some(phien);
            self.chay = None;
            self.trang_thai = "Đã phân tích".into();
            return;
        }
        let so_luong = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).max(1);
        let (gui, nhan) = mpsc::channel();
        let hang = HangDoi::moi(viec);
        hang_doi::chay(hang.clone(), Arc::new(phien.duong_dan_pdf()), ch.clone(), so_luong, gui);
        self.phien = Some(phien);
        self.chay = Some(Chay::PhanTich { nhan, hang, tong, xong: 0, loi: 0, bat_dau: Instant::now() });
    }

    fn nhan_su_kien(&mut self, ch: &CauHinhPhanTich) {
        match &mut self.chay {
            None => {}
            Some(Chay::Mo { nhan, .. }) => match nhan.try_recv() {
                Ok(phien) => self.da_mo(phien, ch),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.chay = None;
                    self.trang_thai = "Không đọc được thư mục".into();
                }
            },
            Some(Chay::PhanTich { nhan, hang, tong, xong, loi, bat_dau }) => {
                let (Some(phien), Some(bang)) = (self.phien.as_mut(), self.bang.as_mut()) else { return };
                let mut ket_thuc = false;
                for _ in 0..SU_KIEN_MOI_KHUNG {
                    match nhan.try_recv() {
                        Ok(SuKienPhanTich::Trang { viec, kq, .. }) => {
                            let k = bang.chi_so(viec.file, viec.trang);
                            bang.dat_tu_dong(k, gan_nhan(&kq, ch));
                            phien.nhan_ket_qua(viec, kq);
                            *xong += 1;
                        }
                        Ok(SuKienPhanTich::Loi { .. }) => {
                            *xong += 1;
                            *loi += 1;
                        }
                        Ok(SuKienPhanTich::Xong) | Err(TryRecvError::Disconnected) => {
                            ket_thuc = true;
                            break;
                        }
                        Err(TryRecvError::Empty) => break,
                    }
                }
                if ket_thuc {
                    let (da_dung, phan_tram, loi, giay) = (hang.da_dung(), *xong * 100 / (*tong).max(1), *loi, bat_dau.elapsed().as_secs());
                    bang.chep_trang_thai(phien);
                    phien.ghi_cache();
                    self.chay = None;
                    self.trang_thai = if da_dung { format!("Đã dừng ở {phan_tram}%") } else { "Đã phân tích".into() };
                    if loi > 0 {
                        self.trang_thai += &format!(" · {loi} trang lỗi");
                    }
                    let _ = giay;
                }
            }
        }
    }

    /// Đưa trang đang hiện lên đầu hàng đợi phân tích.
    fn uu_tien_dang_xem(&self, xem: &[u32]) {
        let (Some(Chay::PhanTich { hang, .. }), Some(b)) = (&self.chay, &self.bang) else { return };
        let ds: Vec<ViecTrang> = xem.iter().map(|&k| &b.trang[k as usize]).filter(|t| t.tu_dong.is_none()).map(|t| ViecTrang { file: t.ho_so as usize, trang: t.so as usize }).collect();
        if !ds.is_empty() {
            hang.uu_tien(&ds);
        }
    }

    // ---- Lưu ----

    pub fn luu_nhan(&mut self) {
        let Some(b) = &mut self.bang else { return };
        let (n, x) = (nhan::ghi_file_nhan(&b.bang_nhan()), nhan::ghi_file_xoay(&b.bang_xoay()));
        let fx = thu_muc_an(Path::new(&self.thu_muc)).join("xoay_trang.txt");
        let kq = ghi_an_toan(&thu_muc_an(Path::new(&self.thu_muc)).join("index_and_blank.txt"), &n).and_then(|_| if x.is_empty() && !fx.exists() { Ok(()) } else { ghi_an_toan(&fx, &x) });
        match kq {
            Ok(()) => b.chua_luu = false,
            Err(e) => self.trang_thai = format!("Không ghi được nhãn: {e}"),
        }
    }

    /// Ghi cache (cờ duyệt) — gọi khi đóng app / đổi thư mục.
    pub fn ghi_cache(&mut self) {
        if let (Some(b), Some(p)) = (&self.bang, &mut self.phien) {
            b.chep_trang_thai(p);
            p.ghi_cache();
        }
    }

    // ---- Thứ tự hiện ----

    fn cap_nhat_thu_tu(&mut self) {
        let Some(b) = &self.bang else {
            self.thu_tu.clear();
            return;
        };
        let khoa = (self.loc, self.ho_so_loc, self.sap, b.phien_ban);
        if self.khoa_thu_tu != Some(khoa) {
            self.thu_tu = b.thu_tu(self.loc, self.ho_so_loc, self.sap);
            self.khoa_thu_tu = Some(khoa);
            self.chon.giu_trong(&self.thu_tu);
        }
    }

    fn nhom(&self) -> Nhom {
        match self.tab {
            Tab::HoSo => Nhom::Loc(self.loc),
            Tab::VanBan => Nhom::VanBan,
        }
    }

    // ---- Thao tác ----

    fn lam(&mut self, viec: Viec, cfg: &mut CauHinh) {
        let nhom = self.nhom();
        let thu_tu = self.thu_tu.clone();
        let ds = self.chon.theo_thu_tu(&thu_tu);
        let Some(b) = self.bang.as_mut() else { return };
        let mut doi = 0;
        // Nghi trắng / Nghi xoay: sau khi duyệt tự chọn trang còn cờ kế tiếp theo thứ tự đang hiện
        let mut sang_ke: Option<(Loc, u32)> = None;
        match viec {
            Viec::Gan(n) => doi = b.gan(nhom, &ds, &ds, n),
            Viec::Xoa => {
                doi = b.xoa(nhom, &ds, &ds);
                if let Some(Chay::PhanTich { hang, .. }) = &self.chay {
                    for &k in &ds {
                        let t = &b.trang[k as usize];
                        hang.bo(ViecTrang { file: t.ho_so as usize, trang: t.so as usize });
                    }
                }
                if doi > 0 {
                    self.bao = Some((format!("Đã bỏ nhãn {doi} trang (thành nội dung) · Ctrl+Z để hoàn tác"), Instant::now()));
                }
            }
            Viec::KhoiPhuc => doi = b.khoi_phuc(nhom, self.ho_so_loc, &ds),
            Viec::Xoay(g) => doi = b.xoay(nhom, &ds, &ds, g, nhom == Nhom::VanBan),
            Viec::XoayDeXuat => doi = b.xoay_de_xuat(nhom, &ds, &ds),
            Viec::VeGoc => doi = b.ve_chieu_goc(nhom, &ds, &ds),
            Viec::XacNhanXoay => {
                doi = b.xac_nhan_xoay(nhom, &ds, &ds);
                sang_ke = ds.last().map(|&k| (Loc::NghiXoay, k));
            }
            Viec::DuyetNghiTrang(n) => {
                doi = b.duyet_nghi_trang(&ds, &ds, n);
                sang_ke = ds.last().map(|&k| (Loc::NghiTrang, k));
            }
            Viec::HoanTac | Viec::LamLai => {
                let kq = if viec == Viec::HoanTac { b.hoan_tac(nhom) } else { b.lam_lai(nhom) };
                let ten = ten_loc(self.loc);
                let chu = match kq {
                    None => format!("Nhóm {ten}: không còn thao tác để {}", if viec == Viec::HoanTac { "hoàn tác" } else { "làm lại" }),
                    Some(kq) => {
                        self.chon.bo_het();
                        for &k in &kq.chon {
                            self.chon.dao_tieu_diem_k(k);
                        }
                        let dau = if viec == Viec::HoanTac { "Đã hoàn tác" } else { "Đã làm lại" };
                        let bo_qua = if kq.bo_qua > 0 { format!(" · bỏ qua {} trang đã sửa ở nhóm khác", kq.bo_qua) } else { String::new() };
                        doi = 1;
                        format!("{dau} ({ten}): {}{bo_qua}", kq.mo_ta)
                    }
                };
                self.bao = Some((chu, Instant::now()));
            }
            Viec::ChonTat => self.chon.chon_tat(&thu_tu),
            Viec::BoChon => self.chon.bo_het(),
            Viec::DatLoc(l) => {
                self.loc = l;
                if l != Loc::DiemCat {
                    self.loc_menu = l;
                }
                self.chon.bo_het();
            }
            Viec::MoLoc(l, k) => {
                self.loc = l;
                self.loc_menu = l;
                self.ho_so_loc = None;
                self.chon.chon_rieng(k);
                self.cuon_toi = Some(k);
            }
            Viec::DatSap(s) => self.sap = s,
            Viec::CoAnh(d) => {
                let moi = (cfg.co_anh_hs as i32 + d).clamp(0, NAC_ANH.len() as i32 - 1) as u8;
                if moi != cfg.co_anh_hs {
                    cfg.co_anh_hs = moi;
                    self.cfg_doi = true;
                }
            }
            Viec::LuuNhan => {
                self.luu_nhan();
                return;
            }
        }
        if let (Some((l, cuoi)), Some(b)) = (sang_ke, self.bang.as_ref()) {
            let con = |k: u32| if l == Loc::NghiTrang { b.trang[k as usize].nghi_trang() } else { b.trang[k as usize].nghi_xoay().is_some() };
            match ke_tiep(&thu_tu, cuoi, con) {
                Some(k) => {
                    self.chon.chon_rieng(k);
                    self.cuon_toi = Some(k);
                }
                None if doi > 0 => {
                    self.chon.bo_het();
                    self.bao(if l == Loc::NghiTrang { "Đã duyệt hết trang Nghi trắng" } else { "Đã xác nhận hết trang Nghi xoay" });
                }
                None => {}
            }
        }
        if doi > 0 && cfg.luu_nhan == LuuNhan::TuDong {
            self.luu_nhan();
        }
    }

    /// Phím tắt tab Hồ sơ (khi không gõ chữ).
    fn phim_tat(&mut self, ctx: &egui::Context) -> Vec<Viec> {
        if ctx.egui_wants_keyboard_input() || self.bang.is_none() || self.tab != Tab::HoSo {
            return Vec::new();
        }
        let mut viec = Vec::new();
        let (loc, so_cot) = (self.loc, self.so_cot);
        let thu_tu = &self.thu_tu;
        let enter_xoay = self.bang.as_ref().map(|b| b.viec_enter_xoay(&self.chon.theo_thu_tu(thu_tu)));
        ctx.input(|i| {
            let m = i.modifiers;
            let phu = PhimPhu { ctrl: m.command, shift: m.shift };
            for (k, p) in [
                (egui::Key::ArrowLeft, PhimDiChuyen::Trai),
                (egui::Key::ArrowRight, PhimDiChuyen::Phai),
                (egui::Key::ArrowUp, PhimDiChuyen::Len),
                (egui::Key::ArrowDown, PhimDiChuyen::Xuong),
                (egui::Key::Home, PhimDiChuyen::DauDs),
                (egui::Key::End, PhimDiChuyen::CuoiDs),
            ] {
                if i.key_pressed(k) {
                    self.cuon_toi = self.chon.phim(p, phu, thu_tu, so_cot);
                }
            }
            let bam = |k: egui::Key| i.key_pressed(k);
            if m.command {
                if bam(egui::Key::A) {
                    viec.push(Viec::ChonTat);
                }
                if bam(egui::Key::Space) {
                    self.chon.dao_tieu_diem();
                }
                if bam(egui::Key::Z) {
                    viec.push(if m.shift { Viec::LamLai } else { Viec::HoanTac });
                }
                if bam(egui::Key::Y) {
                    viec.push(Viec::LamLai);
                }
                if bam(egui::Key::S) {
                    viec.push(Viec::LuuNhan);
                }
                return;
            }
            if bam(egui::Key::Escape) {
                viec.push(Viec::BoChon);
            }
            if bam(egui::Key::Delete) || bam(egui::Key::Backspace) {
                viec.push(Viec::Xoa);
            }
            if bam(egui::Key::R) {
                viec.push(Viec::Xoay(if m.shift { 180 } else { 90 }));
            }
            if bam(egui::Key::S) {
                viec.push(Viec::Gan(Nhan::Split));
            }
            match loc {
                Loc::NghiTrang => {
                    if bam(egui::Key::Enter) || bam(egui::Key::B) {
                        viec.push(Viec::DuyetNghiTrang(Nhan::Trang));
                    }
                    if bam(egui::Key::C) {
                        viec.push(Viec::DuyetNghiTrang(Nhan::NoiDung));
                    }
                    if bam(egui::Key::I) {
                        viec.push(Viec::DuyetNghiTrang(Nhan::Index));
                    }
                }
                _ => {
                    if bam(egui::Key::I) {
                        viec.push(Viec::Gan(Nhan::Index));
                    }
                    if bam(egui::Key::B) {
                        viec.push(Viec::Gan(Nhan::Trang));
                    }
                    if bam(egui::Key::C) {
                        viec.push(if loc == Loc::NghiXoay { Viec::VeGoc } else { Viec::Gan(Nhan::NoiDung) });
                    }
                    if loc == Loc::NghiXoay && bam(egui::Key::Enter) {
                        match enter_xoay {
                            Some(EnterXoay::XoayDeXuat) => viec.push(Viec::XoayDeXuat),
                            Some(EnterXoay::XacNhan) => viec.push(Viec::XacNhanXoay),
                            _ => {}
                        }
                    }
                }
            }
        });
        viec
    }

    // ---- Vẽ ----

    pub fn ve(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinh) {
        let ctx = ui.ctx().clone();
        let ch = cfg.cau_hinh_phan_tich();
        self.nhan_su_kien(&ch);
        if self.chay.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.bao.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(3)) {
            self.bao = None;
        }
        self.cap_nhat_thu_tu();
        let mut viec = self.phim_tat(&ctx);

        if !cfg.an_dong_thu_muc {
            self.hang_thu_muc(ui, &ch);
            ui.add_space(4.0);
        }
        self.hang_tab(ui, cfg);
        egui::Panel::bottom("tach_day").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 0, right: 0, top: 4, bottom: 6 })).show(ui, |ui| self.dong_day(ui, cfg, &mut viec));
        if self.tab == Tab::VanBan {
            ui.centered_and_justified(|ui| ui.label(RichText::new("Tab Văn bản — đang phát triển").color(CHU_PHU)));
        } else {
            let rong_cu = cfg.rong_cot_trai;
            let trai = egui::Panel::left("tach_ho_so")
                .resizable(true)
                .default_size(cfg.rong_cot_trai)
                .size_range(160.0..=520.0)
                .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN_NHAT)))
                .show(ui, |ui| self.bang_ho_so(ui));
            let rong = trai.response.rect.width().round();
            if (rong - rong_cu).abs() >= 1.0 && !ctx.input(|i| i.pointer.any_down()) {
                cfg.rong_cot_trai = rong;
                self.cfg_doi = true;
            }
            egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 8, right: 0, top: 0, bottom: 0 })).show(ui, |ui| {
                self.thanh_cong_cu(ui, &mut viec);
                self.luoi(ui, cfg, &mut viec);
            });
        }
        for v in viec {
            self.lam(v, cfg);
        }
        self.hop_phan_tich_lai(&ctx, &ch);
    }

    fn hang_thu_muc(&mut self, ui: &mut egui::Ui, ch: &CauHinhPhanTich) {
        let dang_chay = self.chay.is_some();
        ui.horizontal(|ui| {
            let nhan = ui.label("Thư mục hồ sơ PDF:");
            let o = egui::TextEdit::singleline(&mut self.thu_muc).desired_width(ui.available_width() - 2.0 * 86.0 - 12.0);
            ui.add_enabled(!dang_chay, o).labelled_by(nhan.id);
            if ui.add_enabled(!dang_chay, nut("Chọn")).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.thu_muc = p.display().to_string();
                }
            }
            let (chu, mau) = if dang_chay {
                ("Dừng", DO)
            } else if self.da_phan_tich() {
                ("Phân tích lại", XANH_LA)
            } else {
                ("Phân tích", XANH_LA)
            };
            let bat = dang_chay || !self.thu_muc.trim().is_empty();
            if ui.add_enabled(bat, nut_mau(chu, mau)).on_disabled_hover_text("Chọn thư mục hồ sơ PDF trước").clicked() {
                self.bam_phan_tich(ch);
            }
        });
    }

    fn hang_tab(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinh) {
        const CAO: f32 = 28.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO), Sense::hover());
        let mut x = rect.left();
        let so_hs = self.bang.as_ref().map_or(0, |b| b.ho_so.len());
        let so_vb = self.tong_van_ban();
        for (tab, chu, bat) in [(Tab::HoSo, format!("Hồ sơ ({})", so_nguyen(so_hs)), true), (Tab::VanBan, format!("Văn bản ({})", so_nguyen(so_vb)), self.da_phan_tich())] {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let rong = ui.fonts_mut(|f| f.layout_no_wrap(chu.clone(), font.clone(), CHU).size().x) + 24.0;
            let r = Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(rong, CAO));
            let phan_hoi = ui.interact(r, ui.scope_id().with(("tab", chu.as_str())), if bat { Sense::click() } else { Sense::hover() });
            let chon = self.tab == tab;
            let mau = if !bat { CHU_MO } else { CHU };
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &chu, font, mau);
            if chon {
                ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(1.0, VIEN_TAB_CHON));
            }
            if phan_hoi.clicked() {
                self.tab = tab;
            }
            if !bat {
                phan_hoi.on_hover_text("Phân tích thư mục trước");
            }
            x += rong;
        }
        // Bên phải: chữ tiến độ + nút ẩn/hiện dòng thư mục
        let nut_r = Rect::from_min_size(egui::pos2(rect.right() - 24.0, rect.top() + 2.0), egui::vec2(24.0, 24.0));
        let nut_an = ui.interact(nut_r, ui.scope_id().with("an_dong"), Sense::click());
        let an = cfg.an_dong_thu_muc;
        let mau_nut = if nut_an.hovered() { XANH_NUT } else { CHU_PHU };
        let c = nut_r.center();
        ui.painter().hline((c.x - 7.0)..=(c.x + 7.0), c.y - 5.0, Stroke::new(1.5, mau_nut));
        let (y0, y1) = if an { (c.y - 1.0, c.y + 5.0) } else { (c.y + 5.0, c.y - 1.0) };
        ui.painter().line_segment([egui::pos2(c.x - 4.0, y0), egui::pos2(c.x, y1)], Stroke::new(1.5, mau_nut));
        ui.painter().line_segment([egui::pos2(c.x + 4.0, y0), egui::pos2(c.x, y1)], Stroke::new(1.5, mau_nut));
        if nut_an.on_hover_text(if an { "Hiện dòng thư mục (Chọn · Phân tích)" } else { "Ẩn dòng thư mục — mở rộng vùng duyệt" }).clicked() {
            cfg.an_dong_thu_muc = !an;
            self.cfg_doi = true;
        }
        let (chu, ty_le) = match &self.chay {
            Some(Chay::Mo { .. }) => ("Đang đọc thư mục…".to_string(), Some(0.0)),
            Some(Chay::PhanTich { tong, xong, .. }) => {
                let p = *xong as f32 / (*tong).max(1) as f32;
                (format!("Đang phân tích {}%", (p * 100.0) as u32), Some(p))
            }
            None => (self.trang_thai.clone(), None),
        };
        ui.painter().text(egui::pos2(nut_r.left() - 8.0, rect.center().y), egui::Align2::RIGHT_CENTER, chu, egui::TextStyle::Small.resolve(ui.style()), CHU_PHU);
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        if let Some(p) = ty_le {
            let day = Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 2.0), egui::pos2(rect.right(), rect.bottom()));
            ui.painter().rect_filled(day, 0.0, XAM_TIEN_DO.gamma_multiply(0.35));
            ui.painter().rect_filled(Rect::from_min_size(day.min, egui::vec2(day.width() * p, 2.0)), 0.0, XANH_NUT);
        }
    }

    fn tong_van_ban(&mut self) -> usize {
        let Some(b) = &self.bang else { return 0 };
        match self.vb_tong {
            Some((pb, n)) if pb == b.phien_ban => n,
            _ => {
                let n = (0..b.ho_so.len()).map(|h| b.van_ban(h).van_ban.len()).sum();
                self.vb_tong = Some((b.phien_ban, n));
                n
            }
        }
    }

    fn bang_ho_so(&mut self, ui: &mut egui::Ui) {
        let Some(b) = &self.bang else {
            ui.add_space(12.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Chọn thư mục rồi bấm Phân tích").color(CHU_PHU)));
            return;
        };
        let so_hs = b.ho_so.len();
        let tong_vb = self.vb_tong.map_or(0, |(_, n)| n);
        let mut chon_hs: Option<Option<u32>> = None;
        let ho_so_loc = self.ho_so_loc;
        TableBuilder::new(ui)
            .striped(true)
            .sense(Sense::click())
            .cell_layout(egui::Layout::left_to_right(Align::Center))
            .column(Column::exact(34.0))
            .column(Column::remainder().at_least(60.0).clip(true))
            .columns(Column::auto().at_least(34.0), 4)
            .header(24.0, |mut h| {
                for t in ["#", "Hồ sơ", "Trang", "Index", "Trắng", "VB"] {
                    h.col(|ui| {
                        ui.label(RichText::new(t).dam());
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, so_hs + 1, |mut row| {
                    let i = row.index();
                    let h = i.checked_sub(1).map(|h| h as u32);
                    row.set_selected(h == ho_so_loc);
                    let (dem, so_trang, vb, ten) = match h {
                        None => (b.tong, b.trang.len(), tong_vb, "Tất cả".to_string()),
                        Some(h) => (b.dem_ho_so(h), b.so_trang(h as usize), b.van_ban(h as usize).van_ban.len(), self.ten_hien[h as usize].clone()),
                    };
                    let dam = |s: String| if h.is_none() { RichText::new(s).dam() } else { RichText::new(s) };
                    row.col(|ui| {
                        ui.label(RichText::new(if i == 0 { String::new() } else { i.to_string() }).color(CHU_PHU));
                    });
                    row.col(|ui| {
                        let r = ui.label(dam(ten));
                        if let Some(h) = h {
                            r.on_hover_text(&b.ho_so[h as usize]);
                        }
                    });
                    row.col(|ui| {
                        ui.label(dam(so_nguyen(so_trang)));
                    });
                    row.col(|ui| {
                        ui.label(dam(so_nguyen(dem.index as usize)));
                        if dem.split > 0 {
                            ui.label(RichText::new(format!("+{}", dem.split)).color(CAM));
                        }
                    });
                    row.col(|ui| {
                        ui.label(dam(so_nguyen(dem.trang as usize)));
                    });
                    row.col(|ui| {
                        ui.label(dam(so_nguyen(vb)));
                    });
                    if row.response().clicked() {
                        chon_hs = Some(h);
                    }
                });
            });
        if let Some(h) = chon_hs {
            self.ho_so_loc = h;
            self.chon.bo_het();
        }
    }

    fn thanh_cong_cu(&mut self, ui: &mut egui::Ui, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        let ds = self.chon.theo_thu_tu(&self.thu_tu);
        let dem = match self.ho_so_loc {
            Some(h) => b.dem_ho_so(h),
            None => b.tong,
        };
        ui.horizontal(|ui| {
            ui.set_height(30.0);
            let muc = self.loc_menu;
            let ten_menu = format!("{} ({})     ", ten_loc(muc), so_nguyen(dem.theo_loc(muc) as usize));
            let nut_menu = egui::Button::new(ten_menu).selected(self.loc != Loc::DiemCat && self.loc != Loc::TatCa);
            let mut mo_lai_muc = false;
            let (r, _) = egui::containers::menu::MenuButton::from_button(nut_menu).ui(ui, |ui| {
                for (i, l) in [Loc::TatCa, Loc::Index, Loc::Trang, Loc::NghiTrang, Loc::NghiXoay].into_iter().enumerate() {
                    if i == 3 {
                        ui.separator();
                    }
                    if muc_menu(ui, format!("{} ({})", ten_loc(l), so_nguyen(dem.theo_loc(l) as usize)), muc == l, 170.0).clicked() {
                        viec.push(Viec::DatLoc(l));
                        ui.close();
                    }
                }
            });
            ve_tam_giac(ui, &r);
            if r.clicked() && self.loc == Loc::DiemCat {
                mo_lai_muc = true;
            }
            if mo_lai_muc {
                viec.push(Viec::DatLoc(muc));
            }
            let cat = ui.add(egui::Button::new(format!("Điểm cắt ({})", so_nguyen(dem.split as usize))).selected(self.loc == Loc::DiemCat)).on_hover_text("Trang có nhãn Split (S)");
            if cat.clicked() {
                viec.push(Viec::DatLoc(if self.loc == Loc::DiemCat { muc } else { Loc::DiemCat }));
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let (r, _) = egui::containers::menu::MenuButton::from_button(egui::Button::new("Sắp xếp     ")).ui(ui, |ui| {
                    for (s, ten) in [(Sap::ThuTu, "Thứ tự trang"), (Sap::DungLuongTang, "Dung lượng nhỏ → lớn"), (Sap::DungLuongGiam, "Dung lượng lớn → nhỏ")] {
                        if muc_menu(ui, ten, self.sap == s, 190.0).clicked() {
                            viec.push(Viec::DatSap(s));
                            ui.close();
                        }
                    }
                });
                ve_tam_giac(ui, &r);
                if ui.button("+").on_hover_text("Ảnh to hơn (Ctrl + cuộn chuột)").clicked() {
                    viec.push(Viec::CoAnh(1));
                }
                if ui.button("−").on_hover_text("Ảnh nhỏ hơn (Ctrl + cuộn chuột)").clicked() {
                    viec.push(Viec::CoAnh(-1));
                }
                let co_chon = !ds.is_empty();
                if bieu_tuong::nut(ui, BieuTuong::Xoay180, "Xoay 180° (Shift+R)", co_chon).clicked() {
                    viec.push(Viec::Xoay(180));
                }
                if bieu_tuong::nut(ui, BieuTuong::XoayPhai, "Xoay phải 90° (R)", co_chon).clicked() {
                    viec.push(Viec::Xoay(90));
                }
                ui.add_space(8.0);
                if ui.add_enabled(co_chon, egui::Button::new("Xoá trang")).on_hover_text("Bỏ nhãn và ẩn khỏi lưới, vẫn có trong file tách (Del)").on_disabled_hover_text("Chọn trang trước (Del)").clicked() {
                    viec.push(Viec::Xoa);
                }
                // Nút nhãn sáng đậm khi mọi trang chọn cùng nhãn, viền khi một phần
                let nut_nhan = |ui: &mut egui::Ui, n: Nhan, chu: &str, goi_y: &str, bat: bool| -> bool {
                    let co = ds.iter().filter(|&&k| b.trang[k as usize].nhan() == n).count();
                    let het = co_chon && co == ds.len();
                    let mut nut = egui::Button::new(if het { RichText::new(chu).color(Color32::WHITE) } else { RichText::new(chu) });
                    if het {
                        nut = nut.fill(XANH_NUT);
                    } else if co > 0 {
                        nut = nut.stroke(Stroke::new(1.5, XANH_NUT));
                    }
                    ui.add_enabled(bat, nut).on_hover_text(goi_y).on_disabled_hover_text(goi_y).clicked()
                };
                let co_noi_dung = ds.iter().any(|&k| matches!(b.trang[k as usize].nhan(), Nhan::NoiDung | Nhan::Split));
                if nut_nhan(ui, Nhan::Trang, "B", "Trắng — bỏ khi tách (B) · C bỏ mọi nhãn", co_chon) {
                    viec.push(if self.loc == Loc::NghiTrang { Viec::DuyetNghiTrang(Nhan::Trang) } else { Viec::Gan(Nhan::Trang) });
                }
                if nut_nhan(ui, Nhan::Index, "I", "Index — tờ phân cách, bỏ khi tách (I) · C bỏ mọi nhãn", co_chon) {
                    viec.push(if self.loc == Loc::NghiTrang { Viec::DuyetNghiTrang(Nhan::Index) } else { Viec::Gan(Nhan::Index) });
                }
                ui.add_space(6.0);
                if nut_nhan(ui, Nhan::Split, "S", "Split — văn bản mới bắt đầu từ trang này (S)", co_noi_dung) {
                    viec.push(Viec::Gan(Nhan::Split));
                }
                ui.label(RichText::new(format!("{} chọn", so_nguyen(ds.len()))).color(CHU_PHU));
            });
        });
        self.thanh_hoi(ui, &ds, viec);
    }

    /// Thanh hỏi duyệt trong bộ lọc Nghi trắng / Nghi xoay.
    fn thanh_hoi(&self, ui: &mut egui::Ui, ds: &[u32], viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        let ten = |k: u32| {
            let t = &b.trang[k as usize];
            format!("{}.{}", self.ten_hien[t.ho_so as usize], t.so)
        };
        match self.loc {
            Loc::NghiTrang => {
                let co: Vec<u32> = ds.iter().copied().filter(|&k| b.trang[k as usize].nghi_trang()).collect();
                if co.is_empty() {
                    return;
                }
                egui::Frame::new().fill(Color32::from_rgb(0xFF, 0xF4, 0xCC)).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let chu = if co.len() == 1 { format!("Trang {} nghi trắng — bỏ khi tách?", ten(co[0])) } else { format!("{} trang nghi trắng — bỏ khi tách?", co.len()) };
                        ui.label(chu);
                        if ui.add(nut_mau("Trắng (Enter · B)", XANH_LA)).clicked() {
                            viec.push(Viec::DuyetNghiTrang(Nhan::Trang));
                        }
                        if ui.add(nut("Giữ nội dung (C)")).clicked() {
                            viec.push(Viec::DuyetNghiTrang(Nhan::NoiDung));
                        }
                        if ui.add(nut("Index (I)")).clicked() {
                            viec.push(Viec::DuyetNghiTrang(Nhan::Index));
                        }
                        ui.label(RichText::new(format!("còn {} trang Nghi trắng", so_nguyen(b.tong.nghi_trang as usize))).color(CHU_PHU));
                    });
                });
            }
            Loc::NghiXoay => {
                let co: Vec<u32> = ds.iter().copied().filter(|&k| b.trang[k as usize].nghi_xoay().is_some()).collect();
                if co.is_empty() {
                    return;
                }
                let da_chinh = co.iter().all(|&k| b.trang[k as usize].da_chinh());
                egui::Frame::new().fill(Color32::from_rgb(0xFD, 0xEB, 0xD8)).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let chu = if co.len() == 1 {
                            format!("{} (đề xuất xoay {}°) →", ten(co[0]), b.trang[co[0] as usize].nghi_xoay().unwrap_or(0))
                        } else {
                            format!("{} trang Nghi xoay (đề xuất riêng từng trang) →", co.len())
                        };
                        ui.label(chu);
                        let (n1, n2) = if da_chinh { (nut("1. Xoay đề xuất"), nut_mau("2. Xác nhận và tiếp (Enter)", XANH_LA)) } else { (nut_mau("1. Xoay đề xuất (Enter)", XANH_LA), nut("2. Xác nhận và tiếp")) };
                        if ui.add(n1).clicked() {
                            viec.push(Viec::XoayDeXuat);
                        }
                        ui.label("→");
                        if ui.add(n2).clicked() {
                            viec.push(Viec::XacNhanXoay);
                        }
                        ui.label(RichText::new(format!("R chỉnh · C về gốc · còn {}", so_nguyen(b.tong.nghi_xoay as usize))).color(CHU_PHU));
                    });
                });
            }
            _ => {}
        }
    }

    fn luoi(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else {
            return;
        };
        let co = NAC_ANH[cfg.co_anh_hs as usize];
        let (cao_anh, cao_o) = (co * 1.414, co * 1.414 + CAO_CHU);
        let n = self.thu_tu.len();
        // Ctrl + cuộn (Mac: chụm 2 ngón) đổi cỡ ảnh khi con trỏ trên lưới
        let vung = ui.available_rect_before_wrap();
        if ui.rect_contains_pointer(vung) {
            let zoom = ui.input(|i| i.zoom_delta());
            if zoom > 1.05 {
                viec.push(Viec::CoAnh(1));
            } else if zoom < 0.95 {
                viec.push(Viec::CoAnh(-1));
            }
        }
        let thu_tu = &self.thu_tu;
        let chon = &mut self.chon;
        let ten_hien = &self.ten_hien;
        let mut keo_tu = self.keo_tu;
        let mut cuon_toi = self.cuon_toi.take();
        let mut dang_xem = Vec::new();
        let mut so_cot = 1;
        egui::ScrollArea::vertical().id_salt("luoi_ho_so").auto_shrink(false).show_viewport(ui, |ui, vp| {
            let rong = ui.available_width();
            let mut bc = BoCucLuoi::moi(rong - 2.0 * LE, co, cao_o, KHE_X, KHE_Y);
            bc.le_trai = LE;
            so_cot = bc.so_cot;
            let cao_noi_dung = bc.so_hang(n) as f32 * bc.cao_hang() + LE;
            ui.set_min_size(egui::vec2(rong, cao_noi_dung));
            let goc = ui.max_rect().min + egui::vec2(0.0, LE);
            let vung_nhin = Rect::from_min_size(ui.max_rect().min + vp.min.to_vec2(), vp.size());
            let phan_hoi = ui.interact(vung_nhin, ui.scope_id().with("luoi"), Sense::click_and_drag());
            let toa_do = |p: Pos2| p - goc.to_vec2();
            let o_tai = |p: Pos2| {
                let q = toa_do(p);
                bc.o_tai(q.x, q.y, n).map(|i| thu_tu[i])
            };
            let (nhan, giu, tha, vi_tri, phu) = ui.input(|i| {
                (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.primary_released(), i.pointer.interact_pos(), PhimPhu { ctrl: i.modifiers.command, shift: i.modifiers.shift })
            });
            // Nhấn trong lưới: chọn ngay (Explorer), bắt đầu theo dõi kéo khung
            if nhan && phan_hoi.hovered() {
                if let Some(p) = vi_tri {
                    chon.nhan(o_tai(p), phu, thu_tu);
                    keo_tu = Some(toa_do(p));
                }
            }
            if let (Some(dau), true, Some(p)) = (keo_tu, giu, vi_tri) {
                let cuoi = toa_do(p);
                if (cuoi - dau).length() > 5.0 {
                    let ds: Vec<u32> = bc.o_giao_khung((dau.x, dau.y), (cuoi.x, cuoi.y), n).into_iter().map(|i| thu_tu[i]).collect();
                    chon.keo(&ds);
                    let khung = Rect::from_two_pos(dau + goc.to_vec2(), p);
                    ui.painter().rect(khung, 0.0, NEN_CHON.gamma_multiply(0.4), Stroke::new(1.0, XANH_NUT), StrokeKind::Inside);
                    // Sát mép trên/dưới → tự cuộn
                    if p.y > vung_nhin.bottom() - 28.0 {
                        ui.scroll_with_delta(egui::vec2(0.0, -16.0));
                    } else if p.y < vung_nhin.top() + 28.0 {
                        ui.scroll_with_delta(egui::vec2(0.0, 16.0));
                    }
                    ui.ctx().request_repaint();
                }
            }
            if tha && keo_tu.is_some() {
                chon.tha();
                keo_tu = None;
            }
            if phan_hoi.secondary_clicked() {
                if let Some(k) = phan_hoi.interact_pointer_pos().and_then(o_tai) {
                    chon.bam_phai(k);
                }
            }
            let hover = phan_hoi.hover_pos().and_then(o_tai);
            // Cuộn tới ô tiêu điểm
            if let Some(k) = cuon_toi.take() {
                if let Some(i) = thu_tu.iter().position(|&x| x == k) {
                    let r = Rect::from_min_size(goc + egui::vec2(LE + (i % bc.so_cot) as f32 * (co + KHE_X), (i / bc.so_cot) as f32 * bc.cao_hang()), egui::vec2(co, cao_o));
                    ui.scroll_to_rect(r, None);
                }
            }
            // Chỉ vẽ hàng đang hiện
            let hang_dau = ((vp.min.y - LE) / bc.cao_hang()).floor().max(0.0) as usize;
            let hang_cuoi = ((vp.max.y / bc.cao_hang()).ceil() as usize).min(bc.so_hang(n));
            let painter = ui.painter_at(vung_nhin);
            let font = egui::TextStyle::Small.resolve(ui.style());
            for hang in hang_dau..hang_cuoi {
                for cot in 0..bc.so_cot {
                    let i = hang * bc.so_cot + cot;
                    if i >= n {
                        break;
                    }
                    let k = thu_tu[i];
                    dang_xem.push(k);
                    let t = &b.trang[k as usize];
                    let o = Rect::from_min_size(goc + egui::vec2(LE + cot as f32 * (co + KHE_X), hang as f32 * bc.cao_hang()), egui::vec2(co, cao_o));
                    let nen_o = o.expand(4.0);
                    if chon.co(k) {
                        painter.rect(nen_o, 2.0, NEN_CHON, Stroke::new(1.0, VIEN_CHON), StrokeKind::Inside);
                    } else if hover == Some(k) {
                        painter.rect_filled(nen_o, 2.0, NEN_HOVER);
                    }
                    if chon.tieu_diem == Some(k) {
                        ve_net_dut(&painter, nen_o.shrink(1.0), Stroke::new(1.0, CHU_PHU), 2.0);
                    }
                    // Ảnh (chờ texture: khung xám); xoay 90/270 thì đổi chiều cho vừa ô
                    let (rw, rh) = if t.sua.goc % 180 == 90 { (co, co / 1.414) } else { (co, cao_anh) };
                    let anh = Rect::from_center_size(egui::pos2(o.center().x, o.top() + cao_anh / 2.0), egui::vec2(rw, rh));
                    let nhan_t = t.nhan();
                    let nghi_t = t.nghi_trang();
                    let nghi_x = t.nghi_xoay();
                    painter.rect_filled(anh, 0.0, if nhan_t == Nhan::Trang { NEN_TRANG_NHAN } else { NEN_ANH_CHO });
                    match nhan_t {
                        Nhan::Index => {
                            painter.rect_stroke(anh, 0.0, Stroke::new(2.0, XANH_COBALT), StrokeKind::Outside);
                        }
                        Nhan::Trang => ve_net_dut(&painter, anh, Stroke::new(1.5, XANH_COBALT), 4.0),
                        Nhan::Split => {
                            painter.rect_stroke(anh, 0.0, Stroke::new(1.5, CAM), StrokeKind::Outside);
                            painter.rect_filled(Rect::from_min_size(anh.min - egui::vec2(5.0, 0.0), egui::vec2(3.0, anh.height())), 0.0, CAM);
                        }
                        Nhan::NoiDung if nghi_t => ve_net_dut(&painter, anh, Stroke::new(1.5, VANG), 4.0),
                        Nhan::NoiDung if nghi_x.is_some() => ve_net_dut(&painter, anh, Stroke::new(1.5, CAM), 4.0),
                        Nhan::NoiDung => {
                            painter.rect_stroke(anh, 0.0, Stroke::new(1.0, VIEN_NHAT), StrokeKind::Outside);
                        }
                    }
                    if t.tu_dong.is_none() {
                        painter.text(anh.center(), egui::Align2::CENTER_CENTER, "…", font.clone(), CHU_MO);
                    }
                    // Nhãn góc trên phải: góc đang xoay / đề xuất Nghi xoay / "Trắng?"
                    let goc_tren = egui::pos2(anh.right() - 3.0, anh.top() + 3.0);
                    let nhan_goc = if nghi_t {
                        Some(("Trắng?".to_string(), VANG, Loc::NghiTrang))
                    } else if let Some(g) = nghi_x {
                        Some((format!("{g}°?"), CAM, Loc::NghiXoay))
                    } else {
                        None
                    };
                    if let Some((chu, mau, l)) = nhan_goc {
                        let r = painter.text(goc_tren, egui::Align2::RIGHT_TOP, &chu, font.clone(), Color32::WHITE);
                        painter.rect_filled(r.expand(2.0), 2.0, mau);
                        painter.text(goc_tren, egui::Align2::RIGHT_TOP, &chu, font.clone(), Color32::WHITE);
                        if nhan && vi_tri.is_some_and(|p| r.expand(2.0).contains(p)) {
                            viec.push(if self.loc == l {
                                if l == Loc::NghiTrang { Viec::DuyetNghiTrang(Nhan::Trang) } else { Viec::XoayDeXuat }
                            } else {
                                Viec::MoLoc(l, k)
                            });
                        }
                    } else if t.sua.goc != 0 {
                        let r = painter.text(goc_tren, egui::Align2::RIGHT_TOP, format!("{}°", t.sua.goc), font.clone(), CHU);
                        painter.rect_filled(r.expand(2.0), 2.0, Color32::from_white_alpha(220));
                        painter.text(goc_tren, egui::Align2::RIGHT_TOP, format!("{}°", t.sua.goc), font.clone(), CHU);
                    }
                    // 2 dòng chữ dưới ảnh
                    let y = o.top() + cao_anh + 3.0;
                    painter.text(egui::pos2(o.center().x, y), egui::Align2::CENTER_TOP, format!("{}.{}", ten_hien[t.ho_so as usize], t.so), font.clone(), CHU);
                    let dong_2 = match nhan_t {
                        Nhan::NoiDung if nghi_t => Some(("Nghi trắng".to_string(), VANG)),
                        Nhan::NoiDung | Nhan::Split if nghi_x.is_some() && t.da_chinh() => Some((format!("Đang {}° · chờ xác nhận", t.sua.goc), CAM)),
                        Nhan::NoiDung if nghi_x.is_some() => Some(("Nghi xoay".to_string(), CAM)),
                        Nhan::NoiDung => None,
                        n => Some((ten_nhan(n).to_string(), if n == Nhan::Split { CAM } else { XANH_COBALT })),
                    };
                    if let Some((chu, mau)) = dong_2 {
                        painter.text(egui::pos2(o.center().x, y + 14.0), egui::Align2::CENTER_TOP, chu, font.clone(), mau);
                    }
                }
            }
            if n == 0 {
                painter.text(vung_nhin.center(), egui::Align2::CENTER_CENTER, "Không có trang nào", egui::TextStyle::Body.resolve(ui.style()), CHU_PHU);
            }
            phan_hoi.context_menu(|ui| menu_phai(ui, b, chon, thu_tu, self.loc, viec));
        });
        self.keo_tu = keo_tu;
        self.so_cot = so_cot;
        if dang_xem != self.dang_xem {
            self.uu_tien_dang_xem(&dang_xem);
            self.dang_xem = dang_xem;
        }
    }

    fn dong_day(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else {
            ui.label(RichText::new(&self.trang_thai).color(CHU_PHU));
            return;
        };
        let nhom = self.nhom();
        ui.horizontal(|ui| {
            ui.label(format!("Còn trong view: {} trang · đã xoá {} · Đã chọn: {} trang", so_nguyen(self.thu_tu.len()), so_nguyen(b.tong.da_xoa as usize), so_nguyen(self.chon.so_luong())));
            let ten = ten_loc(self.loc);
            for (bt, lam_lai, phim) in [(BieuTuong::HoanTac, false, "Ctrl+Z"), (BieuTuong::LamLai, true, "Ctrl+Y")] {
                let buoc = b.buoc_ke(nhom, lam_lai);
                let goi_y = match buoc {
                    Some(m) => format!("{} trong nhóm {ten}: {m} ({phim})", if lam_lai { "Làm lại" } else { "Hoàn tác" }),
                    None => format!("Nhóm {ten}: không còn thao tác để {}", if lam_lai { "làm lại" } else { "hoàn tác" }),
                };
                if bieu_tuong::nut(ui, bt, &goi_y, buoc.is_some()).clicked() {
                    viec.push(if lam_lai { Viec::LamLai } else { Viec::HoanTac });
                }
            }
            if let Some((chu, _)) = &self.bao {
                ui.label(RichText::new(chu).color(XANH_NUT));
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if cfg.luu_nhan == LuuNhan::BangTay && ui.add(egui::Button::new("Lưu nhãn")).on_hover_text("Ctrl+S").clicked() {
                    viec.push(Viec::LuuNhan);
                }
                let r = ui.label(RichText::new(if cfg.luu_nhan == LuuNhan::TuDong { "Tự động lưu index_and_blank.txt" } else { "Lưu bằng tay index_and_blank.txt" }).color(CHU_PHU));
                if b.chua_luu {
                    ui.painter().circle_filled(r.rect.right_top() + egui::vec2(3.0, 1.0), 3.0, DO);
                }
            });
        });
    }

    fn hop_phan_tich_lai(&mut self, ctx: &egui::Context, ch: &CauHinhPhanTich) {
        if !self.hoi_phan_tich_lai {
            return;
        }
        let mut chon = None;
        egui::Modal::new(egui::Id::unique("hoi_phan_tich_lai")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new("Phân tích lại từ đầu?").dam());
            ui.add_space(4.0);
            ui.label("Nhãn sửa tay, Split, góc xoay, trang đã duyệt và trang đã xoá sẽ mất; nhãn tính lại theo Cấu hình.");
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.add(nut_mau("Phân tích lại", XANH_LA)).clicked() {
                    chon = Some(true);
                }
                if ui.add(nut("Huỷ")).clicked() {
                    chon = Some(false);
                }
            });
        });
        if let Some(c) = chon {
            self.hoi_phan_tich_lai = false;
            if c {
                self.bat_dau(ch, true);
            }
        }
    }
}

impl BoChon {
    /// Thêm 1 trang vào vùng chọn (khôi phục vùng chọn khi hoàn tác).
    fn dao_tieu_diem_k(&mut self, k: u32) {
        self.tieu_diem = Some(k);
        self.dao_tieu_diem();
    }
}

/// Menu chuột phải của lưới: áp cho cả vùng chọn.
fn menu_phai(ui: &mut egui::Ui, b: &BangTrang, chon: &mut BoChon, thu_tu: &[u32], loc: Loc, viec: &mut Vec<Viec>) {
    let ds = chon.theo_thu_tu(thu_tu);
    let mut muc = |ui: &mut egui::Ui, chu: &str, phim: &str, v: Viec, bat: bool, dau: bool| {
        let nhan = format!("{chu}{}", if phim.is_empty() { String::new() } else { format!("   {phim}") });
        if ui.add_enabled_ui(bat, |ui| muc_menu(ui, nhan, dau, 250.0)).inner.clicked() {
            viec.push(v);
            ui.close();
        }
    };
    if ds.is_empty() {
        muc(ui, "Chọn tất cả", "Ctrl+A", Viec::ChonTat, true, false);
        muc(ui, "Khôi phục trang đã xoá", "", Viec::KhoiPhuc, b.tong.da_xoa > 0, false);
        return;
    }
    ui.label(RichText::new(format!("{} trang đang chọn", so_nguyen(ds.len()))).color(CHU_PHU));
    let dem = |n: Nhan| ds.iter().filter(|&&k| b.trang[k as usize].nhan() == n).count();
    let tat = |n: Nhan| dem(n) == ds.len();
    let co_xoay = loc == Loc::NghiXoay && ds.iter().any(|&k| b.trang[k as usize].nghi_xoay().is_some());
    let co_nghi = loc == Loc::NghiTrang && ds.iter().any(|&k| b.trang[k as usize].nghi_trang());
    if co_xoay {
        muc(ui, "Xoay đề xuất", "", Viec::XoayDeXuat, true, false);
        muc(ui, "Về chiều gốc", "C", Viec::VeGoc, true, false);
        muc(ui, "Xác nhận và tiếp", "Enter", Viec::XacNhanXoay, true, false);
        ui.separator();
    }
    let co_noi_dung = dem(Nhan::NoiDung) + dem(Nhan::Split) > 0;
    muc(ui, "Split — văn bản mới từ trang này", "S", Viec::Gan(Nhan::Split), co_noi_dung, tat(Nhan::Split));
    muc(ui, "Index", "I", if co_nghi { Viec::DuyetNghiTrang(Nhan::Index) } else { Viec::Gan(Nhan::Index) }, true, tat(Nhan::Index));
    muc(ui, "Trắng", if co_nghi { "B · Enter" } else { "B" }, if co_nghi { Viec::DuyetNghiTrang(Nhan::Trang) } else { Viec::Gan(Nhan::Trang) }, true, tat(Nhan::Trang));
    if co_nghi {
        muc(ui, "Giữ nội dung (bỏ cờ nghi trắng)", "C", Viec::DuyetNghiTrang(Nhan::NoiDung), true, false);
    } else {
        muc(ui, "Bỏ nhãn", "C", Viec::Gan(Nhan::NoiDung), !tat(Nhan::NoiDung), false);
    }
    ui.separator();
    muc(ui, "Xoay phải 90°", "R", Viec::Xoay(90), true, false);
    muc(ui, "Xoay 180°", "Shift+R", Viec::Xoay(180), true, false);
    muc(ui, "Xoá khỏi lưới", "Del", Viec::Xoa, true, false);
    ui.separator();
    muc(ui, "Chọn tất cả", "Ctrl+A", Viec::ChonTat, true, false);
    muc(ui, "Bỏ chọn", "Esc", Viec::BoChon, true, false);
}

/// Tam giác ▾ ở mép phải nút menu.
fn ve_tam_giac(ui: &egui::Ui, r: &egui::Response) {
    let o = Rect::from_center_size(egui::pos2(r.rect.right() - 12.0, r.rect.center().y), egui::vec2(10.0, 10.0));
    bieu_tuong::ve(ui.painter(), o, BieuTuong::MuiTenXuong, CHU_PHU);
}

/// Viền nét đứt quanh hình chữ nhật.
fn ve_net_dut(painter: &egui::Painter, r: Rect, net: Stroke, dai: f32) {
    let diem = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&diem, net, dai, dai * 0.75));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_hien_kem_thu_muc_cha_khi_trung() {
        let ds: Vec<String> = ["HOP 462/4766/4766.pdf", "HOP 463/4766.pdf", "x/L1_05_13.pdf", "a.PDF"].map(String::from).into();
        assert_eq!(ten_hien_ho_so(&ds), ["4766/4766", "HOP 463/4766", "L1_05_13", "a"]);
    }

    /// Phân tích thư mục nhỏ qua màn Tách (không mở cửa sổ): lưới đủ trang, gán nhãn tự lưu file.
    #[test]
    fn phan_tich_gan_nhan_va_tu_luu() {
        let goc = std::env::temp_dir().join(format!("dema4_man_tach_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(&goc).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("a.pdf"), 3);
        let mut cfg = CauHinh { quet_ma: false, quet_xoay: false, ..Default::default() };
        let mut man = ManTach { thu_muc: goc.display().to_string(), ..Default::default() };
        let ch = cfg.cau_hinh_phan_tich();
        man.bam_phan_tich(&ch);
        let bat_dau = Instant::now();
        while man.dang_chay() {
            assert!(bat_dau.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(20));
            man.nhan_su_kien(&ch);
        }
        let b = man.bang.as_ref().unwrap();
        assert_eq!(b.trang.len(), 3);
        assert!(b.trang.iter().all(|t| t.tu_dong.is_some()));
        man.cap_nhat_thu_tu();
        man.chon.chon_rieng(man.thu_tu[1]);
        man.lam(Viec::Gan(Nhan::Index), &mut cfg);
        let file = std::fs::read_to_string(goc.join(".sm-dema4/index_and_blank.txt")).unwrap();
        assert!(file.contains("a.pdf;2;index"), "{file}");
        // Mở lại: nhãn sửa tay còn nguyên
        let mut man2 = ManTach { thu_muc: goc.display().to_string(), ..Default::default() };
        man2.bam_phan_tich(&ch);
        while man2.dang_chay() {
            std::thread::sleep(Duration::from_millis(20));
            man2.nhan_su_kien(&ch);
        }
        assert_eq!(man2.bang.as_ref().unwrap().trang[1].nhan(), Nhan::Index);
        let _ = std::fs::remove_dir_all(&goc);
    }
}
