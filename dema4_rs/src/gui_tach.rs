//! Màn "Tách văn bản": hàng thư mục + Phân tích nền, tab Hồ sơ (bảng hồ sơ, lưới gộp mọi trang để duyệt và
//! gán nhãn kiểu Explorer), dòng đáy. Đặc tả = `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`.

pub mod anh_nho;
pub mod bang_trang;
pub mod bieu_tuong;
pub mod chon;
pub mod cot_trai;
pub mod hop_cau_hinh;
pub mod khung_hop;
pub mod nut_tach;
pub mod phong_to;
pub mod tab_van_ban;
pub mod thanh_nguong;
pub mod thuoc_kho;
pub mod van_ban;
pub mod xem_trang;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use egui_extras::{Column, TableBuilder};

use crate::cau_hinh::{CauHinh, LuuNhan};
use crate::gui::{nut, nut_mau, so_nguyen, ChuDam, CHU, CHU_MO, CHU_PHU, DO, NEN, NEN_TAB, NEN_TAB_CHON, NEN_TIEU_DE, VIEN, VIEN_NHAT, VIEN_TAB_CHON, XAM_TIEN_DO, XANH_COBALT, XANH_LA};
use crate::tach_vb::hang_doi::{self, HangDoi, SuKienPhanTich, ViecTrang};
use crate::tach_vb::nhan::{self, Nhan};
use crate::tach_vb::phan_tich::{gan_nhan, CauHinhPhanTich};
use crate::tach_vb::phien::{thu_muc_an, PhienPhanTich};
use crate::tach_vb::cache_trang;
use bang_trang::{ke_tiep, ten_nhan, BangTrang, EnterXoay, Loc, Nhom, Sap};
use anh_nho::{bac_cho, AnhNho, KhoaAnh, YeuCau};
use bieu_tuong::{atom, muc_menu_icon, nut_phang, BieuTuong};
use chon::{BoChon, BoCucLuoi, PhimDiChuyen, PhimPhu};

/// Nấc cỡ ảnh lưới (bề ngang ô, px) dùng chung tab Hồ sơ và Văn bản. 100 % = `PX_100` (cỡ trang ở hộp phóng to
/// của prototype); nấc cuối 1075 px = 250 % như khi bấm vào từng trang. Ô lớn hơn khung thì co vừa bề ngang.
pub const NAC_ANH: [f32; 12] = [56.0, 72.0, 88.0, 110.0, 140.0, 180.0, 230.0, 300.0, 430.0, 645.0, 860.0, 1075.0];
pub const PX_100: f32 = 430.0;

/// Phần trăm hiển thị của 1 nấc lưới.
pub fn phan_tram_nac(nac: u8) -> u32 {
    (NAC_ANH[(nac as usize).min(NAC_ANH.len() - 1)] / PX_100 * 100.0).round() as u32
}

/// Cỡ ô thật: nấc đã chọn nhưng không rộng hơn bề ngang khung (chừa lề 2 bên + khe).
pub(super) fn co_vua_khung(nac: u8, rong_khung: f32) -> f32 {
    NAC_ANH[(nac as usize).min(NAC_ANH.len() - 1)].min((rong_khung - 2.0 * LE - DEM_O).max(NAC_ANH[0]))
}
/// Lưới như prototype: cột tối thiểu = ảnh + 22 px, khe 12 px, hàng cách nhau đủ cho 2 dòng chữ + khe.
pub(super) const DEM_O: f32 = 22.0;
pub(super) const KHE_X: f32 = 12.0;
pub(super) const KHE_Y: f32 = 36.0;
/// 2 dòng chữ dưới ảnh.
const CAO_CHU: f32 = 30.0;
pub(super) const LE: f32 = 12.0;
const SU_KIEN_MOI_KHUNG: usize = 5_000;

pub(super) const NEN_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF3, 0xFF);
pub(super) const NEN_CHON: Color32 = Color32::from_rgb(0xCC, 0xE8, 0xFF);
pub(super) const VIEN_CHON: Color32 = Color32::from_rgb(0x99, 0xD1, 0xFF);
pub(super) const XANH_NUT: Color32 = Color32::from_rgb(0x00, 0x67, 0xC0);
pub(super) const CAM: Color32 = Color32::from_rgb(0xE0, 0x7A, 0x10);
const VANG: Color32 = Color32::from_rgb(0xD4, 0xA0, 0x00);
const NEN_TRANG_NHAN: Color32 = Color32::from_rgb(0xEA, 0xF3, 0xFC);
pub(super) const NEN_ANH_CHO: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);

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
    /// Hộp phóng to tại trang (thứ tự = danh sách đang xem).
    PhongTo(u32),
    /// Tab Văn bản: văn bản trước / sau, tới văn bản thứ i, bỏ trang khỏi văn bản, chế độ xem, nấc cỡ/tỷ lệ.
    DoiVb(i32),
    DenVb(usize),
    BoKhoiVb,
    CheDoVb(crate::cau_hinh::CheDoVb),
    NacVb(i32),
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
    /// Số hàng lưới thấy được (PageUp / PageDown dời đúng 1 vùng nhìn).
    so_hang_nhin: usize,
    /// Kéo khung: điểm bắt đầu (toạ độ nội dung lưới).
    keo_tu: Option<Pos2>,
    cuon_toi: Option<u32>,
    dang_xem: Vec<u32>,
    hoi_phan_tich_lai: bool,
    anh: Option<AnhNho>,
    // Tab Văn bản
    ds_vb: Vec<van_ban::MucVanBan>,
    khoa_ds_vb: Option<u64>,
    luc_tinh_vb: Option<Instant>,
    pub vb: usize,
    o_so_vb: String,
    pub chon_vb: BoChon,
    keo_tu_vb: Option<Pos2>,
    so_cot_vb: usize,
    cuon_ds_vb: bool,
    cuon_dau_vb: bool,
    dat_giua_ngang: bool,
    pub phong_to: Option<phong_to::PhongTo>,
    // Tách
    tach: Option<nut_tach::DangTach>,
    xong_tach: Option<nut_tach::XongTach>,
    chua_tach: Option<(u32, u32)>,
    hoi_luu_tach: bool,
    tong_tt: Option<(u64, crate::dem_pdf::QuyTac, van_ban::TongVanBan)>,
    /// Ảnh cần render trong khung này (gom từ lưới, xem trang, hộp phóng to rồi gửi 1 lần).
    yeu_cau_anh: Vec<YeuCau>,
    /// Cấu hình đổi trong màn (độ rộng cột, cỡ ảnh…) → app ghi `sm-dema4.cfg`.
    pub cfg_doi: bool,
    /// Lúc bấm Phân tích (tính tổng thời gian cả bước đọc thư mục).
    bat_dau_pt: Option<Instant>,
    /// Cột trái: chuỗi tìm của từng tab, thu gọn (không lưu cfg: mở app luôn mở rộng), hồ sơ đã tự cuộn tới.
    pub(crate) tim_hs: String,
    tim_vb: String,
    pub trai_gon: bool,
    hs_dang_cuon: Option<u32>,
    /// Chọn thư mục (chưa Phân tích): danh sách PDF đọc nền để hiện ngay tên ở cột trái.
    xem_truoc: XemTruoc,
    /// Đang Phân tích: tiến độ từng hồ sơ (vạch dưới dòng cột trái), hồ sơ cột trái đã tự cuộn tới,
    /// lần cuối người dùng chạm cột trái / lưới (tạm dừng tự cuộn 5 s).
    tien_do_hs: cot_trai::TienDoHoSo,
    hs_pt_cuon: Option<u32>,
    trai_cham: Option<Instant>,
    luoi_cham: Option<Instant>,
    /// Cuộn tới `cuon_toi` đặt giữa vùng nhìn (nhảy tới trang cuối hồ sơ vừa xong).
    cuon_giua: bool,
    /// Đang gõ ô thư mục (chưa quét cho tới khi gõ xong) / hộp chọn thư mục chạy luồng riêng (Windows).
    dang_go_tm: bool,
    chon_tm: Option<Receiver<Option<PathBuf>>>,
}

/// Danh sách hồ sơ xem trước: thư mục đã quét, kết quả đang chờ, tên hiển thị + số trang (nếu có cache).
#[derive(Default)]
struct XemTruoc {
    thu_muc: String,
    nhan: Option<Receiver<crate::tach_vb::phien::XemTruocSuKien>>,
    /// Đường dẫn đầy đủ + tương đối (theo thứ tự cây), đã duyệt hết chưa (dùng lại khi bấm Phân tích).
    pdf: Vec<PathBuf>,
    tuong_doi: Vec<String>,
    da_duyet: bool,
    ten: Vec<String>,
    so_trang: Vec<Option<usize>>,
    /// Tên hiển thị tính lại tối đa 5 lần/giây khi danh sách còn đang về.
    luc_tinh_ten: Option<Instant>,
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
            so_hang_nhin: 1,
            keo_tu: None,
            cuon_toi: None,
            dang_xem: Vec::new(),
            hoi_phan_tich_lai: false,
            anh: None,
            ds_vb: Vec::new(),
            khoa_ds_vb: None,
            luc_tinh_vb: None,
            vb: 0,
            o_so_vb: "1".into(),
            chon_vb: BoChon::default(),
            keo_tu_vb: None,
            so_cot_vb: 1,
            cuon_ds_vb: false,
            cuon_dau_vb: false,
            dat_giua_ngang: true,
            phong_to: None,
            tach: None,
            xong_tach: None,
            chua_tach: None,
            hoi_luu_tach: false,
            tong_tt: None,
            yeu_cau_anh: Vec::new(),
            cfg_doi: false,
            bat_dau_pt: None,
            tim_hs: String::new(),
            tim_vb: String::new(),
            trai_gon: false,
            hs_dang_cuon: None,
            xem_truoc: XemTruoc::default(),
            tien_do_hs: Default::default(),
            hs_pt_cuon: None,
            trai_cham: None,
            luoi_cham: None,
            cuon_giua: false,
            dang_go_tm: false,
            chon_tm: None,
        }
    }
}

/// Hộp chọn thư mục: Windows chạy ở luồng riêng để cửa sổ app không đứng hình lúc hộp thoại mở / duyệt ổ mạng;
/// macOS bắt buộc mở trên luồng chính (rfd) nên mở trực tiếp.
fn chon_thu_muc_nen(hien_tai: &str) -> Receiver<Option<PathBuf>> {
    let (gui, nhan) = mpsc::channel();
    let dau = PathBuf::from(hien_tai.trim());
    let mo = move || {
        let mut hop = rfd::FileDialog::new();
        if dau.is_dir() {
            hop = hop.set_directory(&dau);
        }
        let _ = gui.send(hop.pick_folder());
    };
    if cfg!(target_os = "windows") {
        std::thread::spawn(mo);
    } else {
        mo();
    }
    nhan
}

/// Thời lượng dạng hh:mm:ss (10 s → 00:00:10, 10 phút 10 s → 00:10:10).
pub fn gio_phut_giay(d: Duration) -> String {
    let g = d.as_secs_f64().round() as u64;
    format!("{:02}:{:02}:{:02}", g / 3600, g % 3600 / 60, g % 60)
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
    /// Giả lập đang đọc thư mục (kênh không bao giờ trả) — cho test khoá menu.
    #[cfg(test)]
    pub fn gia_dang_chay(&mut self) {
        let (gui, nhan) = mpsc::channel();
        std::mem::forget(gui);
        self.chay = Some(Chay::Mo { nhan, dung: Arc::new(AtomicBool::new(false)) });
    }

    pub fn thu_tu_hien(&self) -> &[u32] {
        &self.thu_tu
    }

    /// Đang phân tích hoặc đang tách (khoá menu, Cấu hình, nút Phân tích).
    pub fn dang_chay(&self) -> bool {
        self.chay.is_some() || self.tach.is_some()
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
        self.anh = None;
        self.chon = BoChon::default();
        self.loc = Loc::TatCa;
        self.loc_menu = Loc::TatCa;
        self.ho_so_loc = None;
        self.khoa_thu_tu = None;
        self.trang_thai = "Đang đọc thư mục…".into();
        self.bat_dau_pt = Some(Instant::now());
        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let dung2 = dung.clone();
        let _ = ch;
        let x = &self.xem_truoc;
        let co_san = (x.da_duyet && Path::new(&x.thu_muc) == goc).then(|| x.pdf.clone());
        std::thread::spawn(move || {
            let _ = gui.send(match co_san {
                Some(pdf) => PhienPhanTich::mo_tu_ds(&goc, pdf),
                None => PhienPhanTich::mo(&goc, &dung2),
            });
        });
        self.chay = Some(Chay::Mo { nhan, dung });
    }

    fn da_mo(&mut self, phien: PhienPhanTich, ch: &CauHinhPhanTich) {
        let so_trang_cua = |p: &str| phien.ho_so.iter().find(|h| h.tuong_doi == p).map(|h| h.trang.len());
        let nf = std::fs::read_to_string(self.file_nhan()).ok().and_then(|s| nhan::doc_file_nhan(&s, &so_trang_cua).ok()).map(|(b, _)| b);
        let xf = std::fs::read_to_string(self.file_xoay()).ok().and_then(|s| nhan::doc_file_xoay(&s, &so_trang_cua).ok());
        let bang = BangTrang::tu_phien(&phien, ch, nf.as_ref(), xf.as_ref());
        self.ten_hien = ten_hien_ho_so(&bang.ho_so);
        self.bang = Some(bang);
        self.phien = Some(phien);
        self.chay = None;
        self.chay_phan_con_thieu(ch);
    }

    /// Phân tích nền các trang còn thiếu kết quả theo Cấu hình `ch` (mở thư mục, hoặc vừa bật thêm loại quét /
    /// tăng độ tin cậy hướng trang).
    fn chay_phan_con_thieu(&mut self, ch: &CauHinhPhanTich) {
        let Some(phien) = &self.phien else { return };
        let viec = phien.viec_con_thieu(ch);
        let tong = viec.len();
        let bat_dau = self.bat_dau_pt.take().unwrap_or_else(Instant::now);
        if tong == 0 {
            self.trang_thai = format!("Phân tích xong ({})", gio_phut_giay(bat_dau.elapsed()));
            return;
        }
        let so_luong = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).max(1);
        let (gui, nhan) = mpsc::channel();
        self.tien_do_hs = cot_trai::TienDoHoSo::moi(phien.ho_so.len(), viec.iter().map(|v| v.file));
        self.hs_pt_cuon = None;
        let hang = HangDoi::moi(viec);
        hang_doi::chay(hang.clone(), Arc::new(phien.duong_dan_pdf()), ch.clone(), so_luong, gui);
        self.chay = Some(Chay::PhanTich { nhan, hang, tong, xong: 0, loi: 0, bat_dau });
    }

    fn nhan_su_kien(&mut self, ch: &CauHinhPhanTich, ctx: Option<&egui::Context>) {
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
                        Ok(SuKienPhanTich::Trang { viec, kq, anh_nho, .. }) => {
                            let k = bang.chi_so(viec.file, viec.trang);
                            // Ảnh xám của phân tích: dùng luôn cho ô đang hiện (đỡ render lại)
                            if let (Some(a), Some(anh), Some(ctx)) = (anh_nho, self.anh.as_mut(), ctx) {
                                if a.kenh == 1 && self.dang_xem.contains(&k) {
                                    anh.them_anh_xam(ctx, KhoaAnh { trang: k, bac: 0 }, a.rong, a.cao, &a.du_lieu);
                                }
                            }
                            bang.dat_tu_dong(k, gan_nhan(&kq, ch));
                            self.tien_do_hs.nhan(viec.file);
                            phien.nhan_ket_qua(viec, kq);
                            *xong += 1;
                        }
                        Ok(SuKienPhanTich::Loi { viec, .. }) => {
                            self.tien_do_hs.nhan(viec.file);
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
                    let (da_dung, phan_tram, loi, tg) = (hang.da_dung(), *xong * 100 / (*tong).max(1), *loi, gio_phut_giay(bat_dau.elapsed()));
                    bang.chep_trang_thai(phien);
                    phien.ghi_cache();
                    self.chay = None;
                    self.tien_do_hs.dang = None;
                    self.trang_thai = if da_dung { format!("Đã dừng ở {phan_tram}% ({tg})") } else { format!("Phân tích xong ({tg})") };
                    if loi > 0 {
                        self.trang_thai += &format!(" · {loi} trang lỗi");
                    }
                }
            }
        }
    }

    /// Chọn / gõ thư mục khi chưa Phân tích: quét nền tên các PDF (+ số trang nếu đã có cache) để hiện ngay ở cột trái.
    fn cap_nhat_xem_truoc(&mut self) {
        if let Some(nhan) = &self.chon_tm {
            match nhan.try_recv() {
                Ok(p) => {
                    self.chon_tm = None;
                    if let Some(p) = p {
                        self.thu_muc = p.display().to_string();
                    }
                }
                Err(TryRecvError::Disconnected) => self.chon_tm = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        // Đổi sang thư mục khác (gõ xong / chọn xong) khi đang mở thư mục cũ: lưu nhãn + cache rồi bỏ phiên cũ
        if !self.dang_go_tm && self.chay.is_none() && self.tach.is_none() {
            if let Some(goc) = self.phien.as_ref().map(|p| p.goc.clone()) {
                if Path::new(self.thu_muc.trim()) != goc {
                    if self.chua_luu() {
                        self.luu_nhan();
                    }
                    self.ghi_cache();
                    self.bang = None;
                    self.phien = None;
                    self.anh = None;
                    self.chon = BoChon::default();
                    self.chon_vb = BoChon::default();
                    self.loc = Loc::TatCa;
                    self.loc_menu = Loc::TatCa;
                    self.ho_so_loc = None;
                    self.khoa_thu_tu = None;
                    self.khoa_ds_vb = None;
                    self.ds_vb.clear();
                    self.trang_thai.clear();
                    self.xem_truoc.thu_muc.clear();
                }
            }
        }
        if self.dang_go_tm {
            return;
        }
        let x = &mut self.xem_truoc;
        if let Some(nhan) = &x.nhan {
            use crate::tach_vb::phien::XemTruocSuKien;
            let goc = PathBuf::from(&x.thu_muc);
            let mut moi = false;
            loop {
                match nhan.try_recv() {
                    Ok(XemTruocSuKien::Them(ds)) => {
                        x.tuong_doi.extend(ds.iter().map(|p| nhan::duong_dan_ghi(&goc, p)));
                        x.so_trang.extend(ds.iter().map(|_| None));
                        x.pdf.extend(ds);
                        moi = true;
                    }
                    Ok(XemTruocSuKien::DaDuyet) => {
                        x.da_duyet = true;
                        moi = true;
                        x.luc_tinh_ten = None;
                    }
                    Ok(XemTruocSuKien::SoTrang(so)) => {
                        if so.len() == x.so_trang.len() {
                            x.so_trang = so;
                        }
                        x.nhan = None;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        x.nhan = None;
                        break;
                    }
                }
            }
            if moi && (x.da_duyet || x.luc_tinh_ten.is_none_or(|t| t.elapsed() > Duration::from_millis(200))) {
                x.ten = ten_hien_ho_so(&x.tuong_doi);
                x.luc_tinh_ten = Some(Instant::now());
            }
        }
        let tm = self.thu_muc.trim();
        if self.bang.is_some() || self.chay.is_some() || tm == x.thu_muc {
            return;
        }
        *x = XemTruoc { thu_muc: tm.to_string(), ..Default::default() };
        let goc = PathBuf::from(tm);
        if tm.is_empty() || !goc.is_dir() {
            return;
        }
        let (gui, nhan) = mpsc::channel();
        std::thread::spawn(move || PhienPhanTich::xem_truoc(&goc, &AtomicBool::new(false), &gui));
        x.nhan = Some(nhan);
    }

    /// Hồ sơ vừa phân tích xong → lưới cuộn tới ảnh cuối của hồ sơ đó (giữa vùng nhìn), trừ khi người dùng vừa
    /// chạm lưới trong 5 s hoặc đang lọc hồ sơ khác. Mỗi hồ sơ 1 lần nên nhẹ (lưới chỉ vẽ ô đang hiện).
    fn nhay_trang_cuoi(&mut self) {
        let xong = self.tien_do_hs.lay_vua_xong();
        let (Some(&h), Some(b)) = (xong.last(), &self.bang) else { return };
        if self.tab != Tab::HoSo || self.luoi_cham.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) || self.ho_so_loc.is_some_and(|x| x != h) {
            return;
        }
        if let Some(&k) = self.thu_tu.iter().rev().find(|&&k| b.trang[k as usize].ho_so == h) {
            self.cuon_toi = Some(k);
            self.cuon_giua = true;
        }
    }

    // ---- Lưu ----

    pub fn luu_nhan(&mut self) {
        let Some(b) = &mut self.bang else { return };
        let goc = self.phien.as_ref().map(|p| p.goc.clone()).unwrap_or_else(|| PathBuf::from(self.thu_muc.trim()));
        let (n, x) = (nhan::ghi_file_nhan(&b.bang_nhan()), nhan::ghi_file_xoay(&b.bang_xoay()));
        let fx = thu_muc_an(&goc).join("xoay_trang.txt");
        let kq = ghi_an_toan(&thu_muc_an(&goc).join("index_and_blank.txt"), &n).and_then(|_| if x.is_empty() && !fx.exists() { Ok(()) } else { ghi_an_toan(&fx, &x) });
        match kq {
            Ok(()) => b.chua_luu = false,
            Err(e) => self.trang_thai = format!("Không ghi được nhãn: {e}"),
        }
    }

    /// Cấu hình vừa lưu: đổi ngưỡng / mã index / độ tin cậy → gán lại nhãn tự động ngay (không render lại,
    /// nhãn người đã sửa giữ nguyên).
    pub fn ap_cau_hinh(&mut self, cu: &CauHinh, moi: &CauHinh) {
        let ch = moi.cau_hinh_phan_tich();
        if cu.cau_hinh_phan_tich() == ch {
            return;
        }
        if let (Some(b), Some(p)) = (&mut self.bang, &self.phien) {
            b.gan_lai_tu_dong(p, &ch);
        }
        if self.chay.is_none() {
            self.chay_phan_con_thieu(&ch);
        }
    }

    /// Còn nhãn chưa ghi ra file (chế độ lưu bằng tay).
    pub fn chua_luu(&self) -> bool {
        self.bang.as_ref().is_some_and(|b| b.chua_luu)
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
        let la_vb = self.tab == Tab::VanBan;
        let thu_tu = if la_vb { self.trang_vb_hien() } else { self.thu_tu.clone() };
        let ds = if la_vb { self.chon_vb.theo_thu_tu(&thu_tu) } else { self.chon.theo_thu_tu(&thu_tu) };
        // Việc chỉ đổi cách xem (không sửa nhãn)
        match viec {
            Viec::DoiVb(d) => return self.doi_vb((self.vb as i64 + d as i64).max(0) as usize),
            Viec::DenVb(i) => return self.doi_vb(i),
            Viec::PhongTo(k) => {
                self.phong_to = Some(phong_to::PhongTo::moi(thu_tu, k));
                return;
            }
            Viec::CheDoVb(c) => {
                cfg.che_do_vb = c;
                self.cfg_doi = true;
                self.dat_giua_ngang = true;
                return;
            }
            Viec::NacVb(d) => {
                if cfg.che_do_vb == crate::cau_hinh::CheDoVb::Luoi {
                    cfg.co_anh_vb = xem_trang::buoc_nac(cfg.co_anh_vb as usize, d > 0, NAC_ANH.len()) as u8;
                } else {
                    cfg.ty_le_vb = xem_trang::TY_LE[xem_trang::buoc_nac(tab_van_ban::nac_ty_le(cfg.ty_le_vb), d > 0, xem_trang::TY_LE.len())];
                    self.dat_giua_ngang = true;
                }
                self.cfg_doi = true;
                return;
            }
            _ => {}
        }
        let chon_hien = if la_vb { &mut self.chon_vb } else { &mut self.chon };
        match viec {
            Viec::ChonTat => return chon_hien.chon_tat(&thu_tu),
            Viec::BoChon => return chon_hien.bo_het(),
            _ => {}
        }
        let Some(b) = self.bang.as_mut() else { return };
        let mut doi = 0;
        // Nghi trắng / Nghi xoay: sau khi duyệt tự chọn trang còn cờ kế tiếp theo thứ tự đang hiện
        let mut sang_ke: Option<(Loc, u32)> = None;
        match viec {
            Viec::Gan(n) => doi = b.gan(nhom, &ds, &ds, n),
            // Bỏ khỏi văn bản = gán Trắng (không có trong file tách; bỏ luôn cờ Nghi trắng)
            Viec::BoKhoiVb => doi = b.gan(nhom, &ds, &ds, Nhan::Trang),
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
                let ten = if la_vb { "Văn bản" } else { ten_loc(self.loc) };
                let chu = match kq {
                    None => format!("Nhóm {ten}: không còn thao tác để {}", if viec == Viec::HoanTac { "hoàn tác" } else { "làm lại" }),
                    Some(kq) => {
                        let chon = if la_vb { &mut self.chon_vb } else { &mut self.chon };
                        chon.bo_het();
                        for &k in &kq.chon {
                            chon.dao_tieu_diem_k(k);
                        }
                        let dau = if viec == Viec::HoanTac { "Đã hoàn tác" } else { "Đã làm lại" };
                        let bo_qua = if kq.bo_qua > 0 { format!(" · bỏ qua {} trang đã sửa ở nhóm khác", kq.bo_qua) } else { String::new() };
                        doi = 1;
                        format!("{dau} ({ten}): {}{bo_qua}", kq.mo_ta)
                    }
                };
                self.bao = Some((chu, Instant::now()));
            }
            Viec::ChonTat | Viec::BoChon | Viec::DoiVb(_) | Viec::DenVb(_) | Viec::PhongTo(_) | Viec::CheDoVb(_) | Viec::NacVb(_) => {}
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
        if ctx.egui_wants_keyboard_input() || self.bang.is_none() || self.tab != Tab::HoSo || self.phong_to.is_some() {
            return Vec::new();
        }
        let mut viec = Vec::new();
        let (loc, so_cot, so_hang) = (self.loc, self.so_cot, self.so_hang_nhin);
        let thu_tu = &self.thu_tu;
        let enter_xoay = self.bang.as_ref().map(|b| b.viec_enter_xoay(&self.chon.theo_thu_tu(thu_tu)));
        let tieu_diem = self.chon.tieu_diem;
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
                (egui::Key::PageUp, PhimDiChuyen::TrangTruoc(so_hang)),
                (egui::Key::PageDown, PhimDiChuyen::TrangSau(so_hang)),
            ] {
                if i.key_pressed(k) {
                    self.cuon_toi = self.chon.phim(p, phu, thu_tu, so_cot);
                    self.luoi_cham = Some(Instant::now());
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
                    if loc != Loc::NghiXoay && bam(egui::Key::Enter) {
                        if let Some(k) = tieu_diem {
                            viec.push(Viec::PhongTo(k));
                        }
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
        self.nhan_su_kien(&ch, Some(&ctx));
        self.nhan_ket_qua_tach();
        if let (None, Some(p)) = (&self.anh, &self.phien) {
            self.anh = Some(AnhNho::moi(p.duong_dan_pdf(), 2, ctx.clone()));
        }
        if let Some(a) = &mut self.anh {
            a.nhan_anh(&ctx);
        }
        if self.dang_chay() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.bao.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(3)) {
            self.bao = None;
        }
        self.cap_nhat_xem_truoc();
        self.cap_nhat_thu_tu();
        self.nhay_trang_cuoi();
        self.cap_nhat_ds_vb();
        let mut viec = self.phim_tat(&ctx);
        viec.extend(self.phim_tat_vb(&ctx));

        if !cfg.an_dong_thu_muc {
            self.hang_thu_muc(ui, &ch);
            ui.add_space(8.0);
        }
        // Dòng Thông tin + nút Tách: ngoài khung, sát đáy cửa sổ
        egui::Panel::bottom("tach_day").frame(egui::Frame::new().fill(NEN).inner_margin(Margin { left: 0, right: 0, top: 8, bottom: 8 })).show(ui, |ui| self.dong_thong_tin_tach(ui, cfg));
        // Khung trắng viền xám: hàng tab | cột trái + vùng duyệt (thanh công cụ, lưới, dòng trạng thái)
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, VIEN)).inner_margin(1)).show(ui, |ui| {
            self.hang_tab(ui, cfg);
            ui.add_space(-ui.spacing().item_spacing.y);
            if self.tab == Tab::VanBan {
                self.tab_van_ban(ui, cfg, &mut viec);
                return;
            }
            let rong_cu = cfg.rong_cot_trai;
            let gon = self.trai_gon;
            let trai = cot_trai_panel("tach_ho_so", gon, cfg.rong_cot_trai).show(ui, |ui| self.bang_ho_so(ui));
            let rong = trai.response.rect.width().round();
            if !gon && !self.trai_gon && (rong - rong_cu).abs() >= 1.0 && !ctx.input(|i| i.pointer.any_down()) {
                cfg.rong_cot_trai = rong;
                self.cfg_doi = true;
            }
            egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ui, |ui| {
                self.thanh_cong_cu(ui, cfg, &mut viec);
                if self.bang.is_some() {
                    egui::Panel::bottom("tach_trang_thai").frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(12, 3))).show(ui, |ui| {
                        ui.painter().hline(ui.max_rect().expand2(egui::vec2(12.0, 0.0)).x_range(), ui.max_rect().top() - 3.5, Stroke::new(1.0, VIEN_NHAT));
                        self.dong_trang_thai(ui, cfg, &mut viec);
                    });
                }
                self.luoi(ui, cfg, &mut viec);
            });
        });
        if let Some(a) = &mut self.anh {
            a.yeu_cau(std::mem::take(&mut self.yeu_cau_anh));
        }
        for v in viec {
            self.lam(v, cfg);
        }
        self.hop_phan_tich_lai(&ctx, &ch);
        self.hop_phong_to(&ctx, cfg);
        self.hop_tach(&ctx, cfg);
    }

    fn hang_thu_muc(&mut self, ui: &mut egui::Ui, ch: &CauHinhPhanTich) {
        let dang_chay = self.chay.is_some();
        ui.horizontal(|ui| {
            let nhan = ui.label("Thư mục hồ sơ PDF:");
            let khoa = dang_chay || self.tach.is_some();
            let o = egui::TextEdit::singleline(&mut self.thu_muc).desired_width(ui.available_width() - 2.0 * 86.0 - 12.0);
            let r = ui.add_enabled(!khoa, o).labelled_by(nhan.id);
            // Gõ xong (rời ô / Enter) mới quét, tránh quét cả ổ khi đang gõ dở "G:\"
            self.dang_go_tm = r.has_focus();
            if ui.add_enabled(!khoa && self.chon_tm.is_none(), nut("Chọn")).clicked() {
                self.chon_tm = Some(chon_thu_muc_nen(&self.thu_muc));
            }
            let (chu, mau) = if dang_chay {
                ("Dừng", DO)
            } else if self.da_phan_tich() {
                ("Phân tích lại", XANH_LA)
            } else {
                ("Phân tích", XANH_LA)
            };
            let bat = (dang_chay || !self.thu_muc.trim().is_empty()) && self.tach.is_none();
            if ui.add_enabled(bat, nut_mau(chu, mau)).on_disabled_hover_text("Chọn thư mục hồ sơ PDF trước").clicked() {
                self.bam_phan_tich(ch);
            }
        });
    }

    fn hang_tab(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinh) {
        const CAO: f32 = 32.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), CAO), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, NEN_TIEU_DE);
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
        let mut x = rect.left();
        let so_hs = self.bang.as_ref().map_or(self.xem_truoc.ten.len(), |b| b.ho_so.len());
        let so_vb = self.tong_van_ban();
        for (tab, chu, bat) in [(Tab::HoSo, format!("Hồ sơ ({})", so_nguyen(so_hs)), true), (Tab::VanBan, format!("Văn bản ({})", so_nguyen(so_vb)), self.da_phan_tich())] {
            let chon = self.tab == tab;
            let font = if chon { egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(13.0) };
            let rong = ui.fonts_mut(|f| f.layout_no_wrap(chu.clone(), font.clone(), CHU).size().x) + 36.0;
            let r = Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(rong, CAO));
            let phan_hoi = ui.interact(r, ui.scope_id().with(("tab", tab == Tab::HoSo)), if bat { Sense::click() } else { Sense::hover() });
            phan_hoi.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, bat, chon, &chu));
            if chon {
                ui.painter().rect_filled(r, 0.0, Color32::WHITE);
                ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(1.0, VIEN_TAB_CHON));
            } else if bat && phan_hoi.hovered() {
                ui.painter().rect_filled(r, 0.0, crate::gui::NUT_HOVER);
            }
            ui.painter().vline(r.right() - 0.5, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
            let mau = if !bat { CHU_MO } else { CHU };
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &chu, font, mau);
            if phan_hoi.clicked() {
                self.tab = tab;
            }
            if !bat {
                phan_hoi.on_hover_text("Phân tích thư mục trước");
            }
            x += rong;
        }
        // Bên phải: chữ tiến độ + nút ẩn/hiện dòng thư mục
        let nut_r = Rect::from_min_size(egui::pos2(rect.right() - 30.0, rect.top() + 4.0), egui::vec2(24.0, 24.0));
        let nut_an = ui.interact(nut_r, ui.scope_id().with("an_dong"), Sense::click());
        let an = cfg.an_dong_thu_muc;
        // Nút icon nét (như .nutgon prototype): nền xanh nhạt khi rê chuột
        if nut_an.hovered() {
            ui.painter().rect(nut_r, 3.0, Color32::from_rgb(0xE5, 0xF3, 0xFF), Stroke::new(1.0, Color32::from_rgb(0xCC, 0xE8, 0xFF)), StrokeKind::Inside);
        }
        bieu_tuong::ve(ui.painter(), Rect::from_center_size(nut_r.center(), egui::vec2(15.0, 15.0)), if an { BieuTuong::HienDong } else { BieuTuong::AnDong }, Color32::from_gray(0x44));
        if nut_an.on_hover_text(if an { "Hiện dòng thư mục (Chọn · Phân tích)" } else { "Ẩn dòng thư mục — mở rộng vùng duyệt" }).clicked() {
            cfg.an_dong_thu_muc = !an;
            self.cfg_doi = true;
        }
        let (chu, ty_le) = match &self.chay {
            // Bộ đếm thời gian chạy (từ lúc bấm Phân tích, gồm bước đọc thư mục)
            Some(Chay::Mo { .. }) => (format!("Đang đọc thư mục… ({})", gio_phut_giay(self.bat_dau_pt.map_or(Duration::ZERO, |t| t.elapsed()))), Some(0.0)),
            Some(Chay::PhanTich { tong, xong, bat_dau, .. }) => {
                let p = *xong as f32 / (*tong).max(1) as f32;
                (format!("Đang phân tích {}% ({})", (p * 100.0) as u32, gio_phut_giay(bat_dau.elapsed())), Some(p))
            }
            None => match &self.tach {
                Some(t) => {
                    let p = t.xong.load(std::sync::atomic::Ordering::Relaxed) as f32 / t.tong.max(1) as f32;
                    (format!("Đang tách {}%", (p * 100.0) as u32), Some(p))
                }
                None => (self.trang_thai.clone(), None),
            },
        };
        // Dừng giữa chừng: chữ cam đậm trên nền cam nhạt cho nổi; xong: chữ xanh lá
        let (mau_chu, nen) = if ty_le.is_some() {
            (CHU_PHU, None)
        } else if chu.starts_with("Đã dừng") {
            (Color32::from_rgb(0xB8, 0x5F, 0x00), Some(Color32::from_rgb(0xFF, 0xF1, 0xDC)))
        } else if chu.starts_with("Phân tích xong") {
            (XANH_LA, None)
        } else {
            (CHU_PHU, None)
        };
        let font = if nen.is_some() || mau_chu == XANH_LA { egui::FontId::new(11.5, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::TextStyle::Small.resolve(ui.style()) };
        let g = ui.painter().layout_no_wrap(chu, font, mau_chu);
        let o = Rect::from_min_size(egui::pos2(nut_r.left() - 12.0 - g.size().x, rect.center().y - g.size().y / 2.0), g.size());
        if let Some(nen) = nen {
            ui.painter().rect(o.expand2(egui::vec2(8.0, 3.0)), 3.0, nen, Stroke::new(1.0, Color32::from_rgb(0xF0, 0xB9, 0x7A)), StrokeKind::Inside);
        }
        ui.painter().galley(o.min, g, mau_chu);

        if let Some(p) = ty_le {
            let day = Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 2.0), egui::pos2(rect.right(), rect.bottom()));
            ui.painter().rect_filled(day, 0.0, XAM_TIEN_DO.gamma_multiply(0.35));
            ui.painter().rect_filled(Rect::from_min_size(day.min, egui::vec2(day.width() * p, 2.0)), 0.0, XANH_NUT);
        }
    }

    fn tong_van_ban(&mut self) -> usize {
        self.ds_vb.len()
    }

    fn bang_ho_so(&mut self, ui: &mut egui::Ui) {
        let gon_truoc = self.trai_gon;
        cot_trai::day(ui, "day_trai_hs", &mut self.tim_hs, "Tìm hồ sơ", &mut self.trai_gon);
        if self.trai_gon != gon_truoc {
            self.hs_dang_cuon = None;
        }
        let Some(b) = &self.bang else {
            self.bang_xem_truoc(ui);
            return;
        };
        let tong_vb = self.ds_vb.len();
        let ho_so_loc = self.ho_so_loc;
        // Hồ sơ hiện trong cột: khớp ô tìm (dòng "Tất cả" luôn ở đầu)
        let hien: Vec<u32> = (0..b.ho_so.len() as u32).filter(|&h| cot_trai::khop_tim(&self.ten_hien[h as usize], &self.tim_hs)).collect();
        let khong_thay = hien.is_empty() && !self.tim_hs.trim().is_empty();
        // Hồ sơ chứa trang đang xem (khi xem Tất cả): tự cuộn tới 1 lần mỗi khi đổi hồ sơ (không tô nền)
        let dang = if ho_so_loc.is_none() && self.chon.so_luong() > 0 { self.chon.tieu_diem.map(|k| b.trang[k as usize].ho_so) } else { None };
        let cuon = match dang {
            Some(h) if self.hs_dang_cuon != Some(h) => hien.iter().position(|&x| x == h).map(|i| i + 1),
            _ => None,
        };
        self.hs_dang_cuon = dang;
        if cot_trai::vua_cham(ui, ui.max_rect()) {
            self.trai_cham = Some(Instant::now());
        }
        let tam_dung = self.trai_cham.is_some_and(|t| t.elapsed() < Duration::from_secs(5));
        let cuon = match self.tien_do_hs.dang {
            Some(h) if self.hs_pt_cuon != Some(h) && !tam_dung => {
                self.hs_pt_cuon = Some(h);
                hien.iter().position(|&x| x == h).map(|i| i + 1).or(cuon)
            }
            _ => cuon,
        };
        let td = &self.tien_do_hs;
        let mut chon_hs: Option<Option<u32>> = None;
        if self.trai_gon {
            let ten_hien = &self.ten_hien;
            let bam = cot_trai::cot_so_gon(
                ui,
                "cot_so_hs",
                1 + hien.len(),
                CAO_DONG_BANG,
                CAO_DAU_BANG,
                |i| if i == 0 { "∑".into() } else { (hien[i - 1] + 1).to_string() },
                |i| if i == 0 { "Tất cả hồ sơ".into() } else { ten_hien[hien[i - 1] as usize].clone() },
                |i| (if i == 0 { None } else { Some(hien[i - 1]) }) == ho_so_loc,
                |i| if i == 0 { None } else { td.phan(hien[i - 1]) },
                cuon,
            );
            if let Some(i) = bam {
                chon_hs = Some(if i == 0 { None } else { Some(hien[i - 1]) });
            }
        } else {
            bat_dau_bang(ui);
            let (x_trai, rong_bang) = (ui.max_rect().left(), ui.max_rect().width());
            let mut bang = TableBuilder::new(ui)
                .sense(Sense::click())
                .cell_layout(egui::Layout::left_to_right(Align::Center))
                .column(Column::exact(28.0))
                .column(Column::remainder().at_least(60.0).clip(true))
                .columns(Column::exact(44.0), 3)
                .column(Column::exact(40.0));
            if let Some(i) = cuon {
                bang = bang.scroll_to_row(i, Some(Align::Center));
            }
            bang.header(CAO_DAU_BANG, |mut h| {
                for (i, t) in ["#", "Hồ sơ", "Trang", "Index", "Trắng", "VB"].into_iter().enumerate() {
                    h.col(|ui| o_so(ui, RichText::new(t).color(CHU_PHU), i != 1));
                }
            })
            .body(|body| {
                body.rows(CAO_DONG_BANG, 1 + hien.len().max(khong_thay as usize), |mut row| {
                    let i = row.index();
                    if khong_thay && i == 1 {
                        row.col(|ui| cot_trai::bao_khong_thay(ui, &format!("Không có hồ sơ nào chứa “{}”", self.tim_hs.trim())));
                        return;
                    }
                    let h = i.checked_sub(1).map(|i| hien[i]);
                    row.set_selected(h == ho_so_loc);
                    let f_td = h.and_then(|h| td.phan(h));
                    let ke_duoi = |ui: &egui::Ui, mau: Color32| {
                        ke_duoi(ui, mau);
                        if let Some(f) = f_td {
                            cot_trai::vach_tien_do(ui, f, x_trai, rong_bang);
                        }
                    };
                    let (dem, so_trang, vb, ten) = match h {
                        None => (b.tong, b.trang.len(), tong_vb, "Tất cả".to_string()),
                        Some(h) => (b.dem_ho_so(h), b.so_trang(h as usize), b.van_ban(h as usize).van_ban.len(), self.ten_hien[h as usize].clone()),
                    };
                    let dam = |s: String| if h.is_none() { RichText::new(s).dam() } else { RichText::new(s) };
                    // Dòng "Tất cả" ngăn với các hồ sơ bằng vạch đậm hơn
                    let vach = if h.is_none() { VIEN_NHAT } else { VACH_DONG };
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        o_so(ui, RichText::new(h.map(|h| (h + 1).to_string()).unwrap_or_default()).color(CHU_PHU), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        ui.add_space(LE_O_BANG);
                        let r = ui.add(egui::Label::new(dam(ten)).truncate());
                        if let Some(h) = h {
                            r.on_hover_text(&b.ho_so[h as usize]);
                        }
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        o_so(ui, dam(so_nguyen(so_trang)), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        let mut job = egui::text::LayoutJob::default();
                        job.append(&so_nguyen(dem.index as usize), 0.0, chu_dong(CHU, h.is_none()));
                        if dem.split > 0 {
                            job.append(&format!(" +{}", dem.split), 0.0, chu_dong(CAM, false));
                        }
                        ui.centered_and_justified(|ui| ui.add(egui::Label::new(job).truncate()));
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        o_so(ui, dam(so_nguyen(dem.trang as usize)), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        o_so(ui, dam(so_nguyen(vb)), true);
                    });
                    if row.response().clicked() {
                        chon_hs = Some(h);
                    }
                });
            });
        }
        if let Some(h) = chon_hs {
            self.ho_so_loc = h;
            self.chon.bo_het();
        }
    }

    /// Chưa Phân tích: chỉ tên các PDF (và số trang nếu có cache); các cột còn lại điền dần khi Phân tích.
    fn bang_xem_truoc(&mut self, ui: &mut egui::Ui) {
        let x = &self.xem_truoc;
        if x.ten.is_empty() {
            if !self.trai_gon {
                ui.add_space(12.0);
                let chu = if x.nhan.is_some() { "Đang đọc danh sách PDF…" } else if x.thu_muc.is_empty() { "Chọn thư mục rồi bấm Phân tích" } else { "Không có file PDF nào" };
                ui.vertical_centered(|ui| ui.label(RichText::new(chu).color(CHU_PHU)));
            }
            return;
        }
        let hien: Vec<usize> = (0..x.ten.len()).filter(|&i| cot_trai::khop_tim(&x.ten[i], &self.tim_hs)).collect();
        if self.trai_gon {
            cot_trai::cot_so_gon(ui, "cot_so_hs", 1 + hien.len(), CAO_DONG_BANG, CAO_DAU_BANG, |i| if i == 0 { "∑".into() } else { (hien[i - 1] + 1).to_string() }, |i| if i == 0 { "Tất cả hồ sơ".into() } else { x.ten[hien[i - 1]].clone() }, |i| i == 0, |_| None, None);
            return;
        }
        let tong_trang: usize = x.so_trang.iter().flatten().sum();
        bat_dau_bang(ui);
        TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(Align::Center))
            .column(Column::exact(28.0))
            .column(Column::remainder().at_least(60.0).clip(true))
            .columns(Column::exact(44.0), 3)
            .column(Column::exact(40.0))
            .header(CAO_DAU_BANG, |mut h| {
                for (i, t) in ["#", "Hồ sơ", "Trang", "Index", "Trắng", "VB"].into_iter().enumerate() {
                    h.col(|ui| o_so(ui, RichText::new(t).color(CHU_PHU), i != 1));
                }
            })
            .body(|body| {
                body.rows(CAO_DONG_BANG, 1 + hien.len(), |mut row| {
                    let i = row.index();
                    let (so, ten, trang) = if i == 0 {
                        (String::new(), RichText::new("Tất cả").dam(), (tong_trang > 0).then(|| RichText::new(so_nguyen(tong_trang)).dam()))
                    } else {
                        let f = hien[i - 1];
                        ((f + 1).to_string(), RichText::new(&x.ten[f]), x.so_trang[f].map(|n| RichText::new(so_nguyen(n))))
                    };
                    let vach = if i == 0 { VIEN_NHAT } else { VACH_DONG };
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        o_so(ui, RichText::new(so).color(CHU_PHU), true);
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        ui.add_space(LE_O_BANG);
                        ui.add(egui::Label::new(ten).truncate());
                    });
                    row.col(|ui| {
                        ke_duoi(ui, vach);
                        if let Some(t) = trang {
                            o_so(ui, t, true);
                        }
                    });
                    for _ in 0..3 {
                        row.col(|ui| ke_duoi(ui, vach));
                    }
                });
            });
    }

    fn thanh_cong_cu(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        let ds = self.chon.theo_thu_tu(&self.thu_tu);
        let dem = match self.ho_so_loc {
            Some(h) => b.dem_ho_so(h),
            None => b.tong,
        };
        let thanh = egui::Frame::new().fill(NEN_TAB).inner_margin(Margin { left: 0, right: 8, top: 0, bottom: 0 }).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().button_padding.x = 6.0;
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                ui.set_height(30.0);
                ui.spacing_mut().item_spacing.x = 0.0;
                // Bộ lọc: 5 mục gom vào 1 tab thả xuống (mang tên mục đang chọn); Điểm cắt là tab riêng
                let muc = self.loc_menu;
                let r = tab_loc(ui, &format!("{} ({})", ten_loc(muc), so_nguyen(dem.theo_loc(muc) as usize)), self.loc != Loc::DiemCat, true);
                if r.clicked() && self.loc == Loc::DiemCat {
                    viec.push(Viec::DatLoc(muc));
                }
                egui::Popup::menu(&r).show(|ui| {
                    for (i, (l, icon)) in [(Loc::TatCa, BieuTuong::ChongTrang), (Loc::Index, BieuTuong::MaVach), (Loc::Trang, BieuTuong::TrangTrong), (Loc::NghiTrang, BieuTuong::TrangHoi), (Loc::NghiXoay, BieuTuong::XoayMenu)].into_iter().enumerate() {
                        if i == 3 {
                            ui.separator();
                        }
                        if muc_menu_icon(ui, icon, format!("{} ({})", ten_loc(l), so_nguyen(dem.theo_loc(l) as usize)), muc == l, RONG_MENU_LOC).clicked() {
                            viec.push(Viec::DatLoc(l));
                            ui.close();
                        }
                    }
                });
                let cat = tab_loc(ui, &format!("Điểm cắt ({})", so_nguyen(dem.split as usize)), self.loc == Loc::DiemCat, false).on_hover_text("Các trang Split (S) · bấm lại để về bộ lọc trước");
                if cat.clicked() {
                    viec.push(Viec::DatLoc(if self.loc == Loc::DiemCat { muc } else { Loc::DiemCat }));
                }
                // Ngay sau tab Điểm cắt: nhóm gắn nhãn (S│I│B) và nhóm sửa (Hoàn tác│Làm lại│Xoá trang)
                ui.add_space(4.0);
                ngan(ui);
                ui.add_space(4.0);
                let co_chon = !ds.is_empty();
                let co_noi_dung = ds.iter().any(|&k| matches!(b.trang[k as usize].nhan(), Nhan::NoiDung | Nhan::Split));
                let trang_thai = |n: Nhan| {
                    let co = ds.iter().filter(|&&k| b.trang[k as usize].nhan() == n).count();
                    (co_chon && co == ds.len(), co > 0 && co < ds.len())
                };
                nhom_nut(ui, |ui| {
                    if nut_nhom(ui, NoiDungNut::Chu("S", Some(MAU_S)), trang_thai(Nhan::Split), co_noi_dung, "Split — văn bản mới bắt đầu từ trang này, trang được giữ (S, bấm lại để huỷ)").clicked() {
                        viec.push(Viec::Gan(Nhan::Split));
                    }
                    ngan_nho(ui);
                    if nut_nhom(ui, NoiDungNut::Chu("I", None), trang_thai(Nhan::Index), co_chon, "Index — tờ mã vạch, điểm cắt và bị bỏ khi tách (I, bấm lại để huỷ) · C bỏ mọi nhãn").clicked() {
                        viec.push(if self.loc == Loc::NghiTrang { Viec::DuyetNghiTrang(Nhan::Index) } else { Viec::Gan(Nhan::Index) });
                    }
                    ngan_nho(ui);
                    if nut_nhom(ui, NoiDungNut::Chu("B", None), trang_thai(Nhan::Trang), co_chon, "Trắng — trang trắng/bìa, bỏ khi tách (B, bấm lại để huỷ) · C bỏ mọi nhãn").clicked() {
                        viec.push(if self.loc == Loc::NghiTrang { Viec::DuyetNghiTrang(Nhan::Trang) } else { Viec::Gan(Nhan::Trang) });
                    }
                });
                ui.add_space(8.0);
                nhom_nut(ui, |ui| {
                    let nhom = self.nhom();
                    let ten = ten_loc(self.loc);
                    for (bt, lam_lai, phim) in [(BieuTuong::HoanTac, false, "Ctrl+Z"), (BieuTuong::LamLai, true, "Ctrl+Y")] {
                        let buoc = b.buoc_ke(nhom, lam_lai);
                        let goi_y = match buoc {
                            Some(m) => format!("{} trong nhóm {ten}: {m} ({phim})", if lam_lai { "Làm lại" } else { "Hoàn tác" }),
                            None => format!("Nhóm {ten}: không còn thao tác để {} ({phim})", if lam_lai { "làm lại" } else { "hoàn tác" }),
                        };
                        if nut_nhom(ui, NoiDungNut::Icon(bt), (false, false), buoc.is_some(), &goi_y).clicked() {
                            viec.push(if lam_lai { Viec::LamLai } else { Viec::HoanTac });
                        }
                        ngan_nho(ui);
                    }
                    if nut_nhom(ui, NoiDungNut::Icon(BieuTuong::ThungRac), (false, false), co_chon, "Xoá khỏi lưới: trang đang chọn thành nội dung và ẩn đi; vẫn có trong file tách (Del) · Ctrl+Z hoàn tác").clicked() {
                        viec.push(Viec::Xoa);
                    }
                });
                // Dồn phải: xoay · cỡ ảnh · sắp xếp
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let ten_sap = match self.sap {
                        Sap::ThuTu => "Sắp xếp",
                        Sap::DungLuongTang => "Nhỏ → lớn",
                        Sap::DungLuongGiam => "Lớn → nhỏ",
                    };
                    let r = ui.add(nut_phang((atom(BieuTuong::SapXep, 15.0), ten_sap, atom(BieuTuong::ChevronXuong, 11.0)), false)).on_hover_text("Sắp xếp");
                    egui::Popup::menu(&r).show(|ui| {
                        for (s, icon, ten) in [(Sap::ThuTu, BieuTuong::DanhSachSo, "Thứ tự trang"), (Sap::DungLuongTang, BieuTuong::TangDan, "Dung lượng: nhỏ → lớn"), (Sap::DungLuongGiam, BieuTuong::GiamDan, "Dung lượng: lớn → nhỏ")] {
                            if muc_menu_icon(ui, icon, ten, self.sap == s, 200.0).clicked() {
                                viec.push(Viec::DatSap(s));
                                ui.close();
                            }
                        }
                    });
                    ngan(ui);
                    nut_thu_phong(ui, phan_tram_nac(cfg.co_anh_hs), cfg.co_anh_hs > 0, (cfg.co_anh_hs as usize) < NAC_ANH.len() - 1, "ảnh", viec, Viec::CoAnh(-1), Viec::CoAnh(1));
                    ngan(ui);
                    nut_xoay(ui, co_chon, viec);
                });
            });
        });
        ui.painter().hline(thanh.response.rect.x_range(), thanh.response.rect.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
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
        let co = co_vua_khung(cfg.co_anh_hs, ui.available_width());
        let (cao_anh, cao_o) = (co * 1.414, co * 1.414 + CAO_CHU);
        let n = self.thu_tu.len();
        // Ctrl + cuộn (Mac: chụm 2 ngón) đổi cỡ ảnh khi con trỏ trên lưới
        let vung = ui.available_rect_before_wrap();
        if cot_trai::vua_cham(ui, vung) {
            self.luoi_cham = Some(Instant::now());
        }
        let cuon_giua = std::mem::take(&mut self.cuon_giua);
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
        let mut anh_nho = self.anh.as_mut();
        // Lưới hiện nhiều ô cùng lúc: tối đa bậc 240 px (ảnh lớn có LRU nhỏ, không đủ cho cả màn)
        let bac = bac_cho(co * ui.ctx().pixels_per_point()).min(if co > 230.0 { 3 } else { 1 });
        let yeu_cau = &mut self.yeu_cau_anh;
        let ten_hien = &self.ten_hien;
        let mut keo_tu = self.keo_tu;
        let mut cuon_toi = self.cuon_toi.take();
        let mut dang_xem = Vec::new();
        let mut so_cot = 1;
        let mut so_hang_nhin = 1;
        egui::ScrollArea::vertical().id_salt("luoi_ho_so").auto_shrink(false).show_viewport(ui, |ui, vp| {
            let rong = ui.available_width();
            let mut bc = BoCucLuoi::deu(rong - 2.0 * LE, co, cao_o, DEM_O, KHE_X, KHE_Y);
            bc.le_trai += LE;
            so_cot = bc.so_cot;
            let cao_noi_dung = bc.so_hang(n) as f32 * bc.cao_hang() + LE;
            ui.set_min_size(egui::vec2(rong, cao_noi_dung));
            let goc = ui.max_rect().min + egui::vec2(0.0, LE);
            let vung_nhin = Rect::from_min_size(ui.max_rect().min + vp.min.to_vec2(), vp.size());
            so_hang_nhin = ((vp.height() / bc.cao_hang()).floor() as usize).max(1);
            let phan_hoi = ui.interact(vung_nhin, ui.scope_id().with("luoi"), Sense::click_and_drag());
            let ChuotLuoi { hover, nhan, vi_tri, bam_dup } = chuot_luoi(ui, &phan_hoi, &bc, goc, vung_nhin, thu_tu, chon, &mut keo_tu);
            if let Some(k) = bam_dup {
                viec.push(Viec::PhongTo(k));
            }
            // Cuộn tới ô tiêu điểm
            if let Some(k) = cuon_toi.take() {
                if let Some(i) = thu_tu.iter().position(|&x| x == k) {
                    let r = Rect::from_min_size(goc + egui::Vec2::from(bc.goc_o(i)), egui::vec2(co, cao_o));
                    ui.scroll_to_rect(r, if cuon_giua { Some(Align::Center) } else { None });
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
                    let o = Rect::from_min_size(goc + egui::Vec2::from(bc.goc_o(i)), egui::vec2(co, cao_o));
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
                    let hop = Rect::from_min_size(o.min, egui::vec2(co, cao_anh));
                    let nhan_t = t.nhan();
                    let nghi_t = t.nghi_trang();
                    let nghi_x = t.nghi_xoay();
                    let tex = anh_nho.as_mut().and_then(|a| {
                        let khoa = KhoaAnh { trang: k, bac };
                        if !a.co(&khoa) {
                            yeu_cau.push(YeuCau { khoa, file: t.ho_so as usize, so: t.so as usize });
                        }
                        a.lay(k, bac)
                    });
                    let anh = match &tex {
                        Some(tex) => {
                            let r = anh_nho::ve_xoay(&painter, hop, tex, t.sua.goc);
                            if nhan_t == Nhan::Trang {
                                painter.rect_filled(r, 0.0, NEN_TRANG_NHAN.gamma_multiply(0.45));
                            }
                            r
                        }
                        None => {
                            let (rw, rh) = if t.sua.goc % 180 == 90 { (co, co / 1.414) } else { (co, cao_anh) };
                            let r = Rect::from_center_size(hop.center(), egui::vec2(rw, rh));
                            painter.rect_filled(r, 0.0, if nhan_t == Nhan::Trang { NEN_TRANG_NHAN } else { NEN_ANH_CHO });
                            r
                        }
                    };
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
        self.so_hang_nhin = so_hang_nhin;
        // Phân tích đi lần lượt từ hồ sơ đầu danh sách tới cuối (không ưu tiên trang đang hiện nữa)
        self.dang_xem = dang_xem;
    }

    /// Dòng trạng thái dưới lưới (trong vùng duyệt): còn trong view, hoàn tác / làm lại, trạng thái lưu nhãn.
    fn dong_trang_thai(&mut self, ui: &mut egui::Ui, cfg: &CauHinh, viec: &mut Vec<Viec>) {
        let Some(b) = &self.bang else { return };
        ui.horizontal(|ui| {
            let mut job = egui::text::LayoutJob::default();
            let thuong = chu_dong(CHU_PHU, false);
            job.append("Còn trong view: ", 0.0, thuong.clone());
            job.append(&so_nguyen(self.thu_tu.len()), 0.0, chu_dong(CHU, true));
            job.append(&format!(" trang · đã xoá {}", so_nguyen(b.tong.da_xoa as usize)), 0.0, thuong.clone());
            let sc = self.chon.so_luong();
            if sc > 0 {
                job.append(" · Đã chọn: ", 0.0, thuong.clone());
                job.append(&so_nguyen(sc), 0.0, chu_dong(VIEN_TAB_CHON, true));
                job.append(" trang", 0.0, thuong);
            }
            ui.label(job);
            self.bao_tam(ui);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if cfg.luu_nhan == LuuNhan::BangTay && ui.add_enabled(b.chua_luu, nut_nho("Lưu nhãn")).on_hover_text("Ctrl+S").clicked() {
                    viec.push(Viec::LuuNhan);
                }
                let r = ui.label(RichText::new(if cfg.luu_nhan == LuuNhan::TuDong { "Tự động lưu index_and_blank.txt" } else { "Lưu bằng tay index_and_blank.txt" }).size(12.0).color(CHU_PHU));
                let r = if b.chua_luu { r.on_hover_text("Có thay đổi chưa lưu") } else { r.on_hover_text("Đã lưu") };
                if b.chua_luu {
                    ui.painter().circle_filled(r.rect.right_top() + egui::vec2(5.0, 1.0), 3.5, Color32::from_rgb(0xE8, 0x11, 0x23));
                }
            });
        });
    }

    /// Báo tạm (vd "Đã hoàn tác …") ở dòng trạng thái.
    fn bao_tam(&self, ui: &mut egui::Ui) {
        if let Some((chu, _)) = &self.bao {
            ui.label(RichText::new(chu).color(VIEN_TAB_CHON).dam());
        }
    }

    fn hop_phan_tich_lai(&mut self, ctx: &egui::Context, ch: &CauHinhPhanTich) {
        if !self.hoi_phan_tich_lai {
            return;
        }
        let mut chon = None;
        khung_hop::hop(ctx, "hoi_phan_tich_lai", 340.0, "Phân tích lại từ đầu?", |ui| {
            khung_hop::than_chu(ui, "Nhãn sửa tay, Split, góc xoay, trang đã duyệt và trang đã xoá sẽ mất; nhãn tính lại theo Cấu hình.");
            khung_hop::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(khung_hop::nut_chan_mau("Phân tích lại", XANH_LA)).clicked() {
                        chon = Some(true);
                    }
                    if ui.add(khung_hop::nut_chan("Huỷ")).clicked() {
                        chon = Some(false);
                    }
                });
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

#[cfg(test)]
impl ManTach {
    /// Dựng sẵn 1 popup để chụp ảnh so prototype.
    pub(crate) fn mo_popup_mau(&mut self, ten: &str) {
        match ten {
            "chua_tach" => self.chua_tach = Some((3, 2)),
            "phan_tich_lai" => self.hoi_phan_tich_lai = true,
            _ => self.xong_tach = Some(nut_tach::XongTach::mau()),
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

/// Kết quả xử lý chuột 1 khung trên lưới ô đều.
pub(crate) struct ChuotLuoi {
    pub hover: Option<u32>,
    /// Vừa nhấn chuột trái (để các nút nhỏ trên ô xử lý tiếp).
    pub nhan: bool,
    pub vi_tri: Option<Pos2>,
    pub bam_dup: Option<u32>,
}

/// Chọn kiểu Explorer trên lưới ô đều (dùng chung tab Hồ sơ và tab Văn bản): nhấn chọn ngay, kéo khung
/// (sát mép tự cuộn), thả, bấm phải chọn riêng ô chưa chọn, bấm đúp. `goc` = gốc toạ độ nội dung trên màn.
pub(crate) fn chuot_luoi(ui: &egui::Ui, phan_hoi: &egui::Response, bc: &BoCucLuoi, goc: Pos2, vung_nhin: Rect, thu_tu: &[u32], chon: &mut BoChon, keo_tu: &mut Option<Pos2>) -> ChuotLuoi {
    let n = thu_tu.len();
    let toa_do = |p: Pos2| p - goc.to_vec2();
    let o_tai = |p: Pos2| {
        let q = toa_do(p);
        bc.o_tai(q.x, q.y, n).map(|i| thu_tu[i])
    };
    let (nhan, giu, tha, vi_tri, phu) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.primary_released(), i.pointer.interact_pos(), PhimPhu { ctrl: i.modifiers.command, shift: i.modifiers.shift }));
    if nhan && phan_hoi.hovered() {
        if let Some(p) = vi_tri {
            chon.nhan(o_tai(p), phu, thu_tu);
            *keo_tu = Some(toa_do(p));
        }
    }
    if let (Some(dau), true, Some(p)) = (*keo_tu, giu, vi_tri) {
        let cuoi = toa_do(p);
        if (cuoi - dau).length() > 5.0 {
            let ds: Vec<u32> = bc.o_giao_khung((dau.x, dau.y), (cuoi.x, cuoi.y), n).into_iter().map(|i| thu_tu[i]).collect();
            chon.keo(&ds);
            let khung = Rect::from_two_pos(dau + goc.to_vec2(), p);
            ui.painter().rect(khung, 0.0, NEN_CHON.gamma_multiply(0.4), Stroke::new(1.0, XANH_NUT), StrokeKind::Inside);
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
        *keo_tu = None;
    }
    if phan_hoi.secondary_clicked() {
        if let Some(k) = phan_hoi.interact_pointer_pos().and_then(o_tai) {
            chon.bam_phai(k);
        }
    }
    let bam_dup = if phan_hoi.double_clicked() { phan_hoi.interact_pointer_pos().and_then(o_tai) } else { None };
    ChuotLuoi { hover: phan_hoi.hover_pos().and_then(o_tai), nhan, vi_tri, bam_dup }
}

/// Menu chuột phải của lưới: áp cho cả vùng chọn.
fn menu_phai(ui: &mut egui::Ui, b: &BangTrang, chon: &mut BoChon, thu_tu: &[u32], loc: Loc, viec: &mut Vec<Viec>) {
    bieu_tuong::menu_gon(ui);
    let ds = chon.theo_thu_tu(thu_tu);
    let mut muc = |ui: &mut egui::Ui, icon: BieuTuong, chu: &str, phim: &str, v: Viec, bat: bool, dau: bool| {
        if bieu_tuong::muc_menu_phai(ui, icon, chu, phim, dau, bat).clicked() {
            viec.push(v);
            ui.close();
        }
    };
    if ds.is_empty() {
        muc(ui, BieuTuong::ChonTat, "Chọn tất cả", "Ctrl+A", Viec::ChonTat, true, false);
        muc(ui, BieuTuong::KhoiPhuc, "Khôi phục trang đã xoá", "", Viec::KhoiPhuc, b.tong.da_xoa > 0, false);
        return;
    }
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(RichText::new(format!("{} trang đang chọn", so_nguyen(ds.len()))).size(11.5).color(CHU_PHU));
    });
    ui.add_space(3.0);
    let dem = |n: Nhan| ds.iter().filter(|&&k| b.trang[k as usize].nhan() == n).count();
    let tat = |n: Nhan| dem(n) == ds.len();
    let co_xoay = loc == Loc::NghiXoay && ds.iter().any(|&k| b.trang[k as usize].nghi_xoay().is_some());
    let co_nghi = loc == Loc::NghiTrang && ds.iter().any(|&k| b.trang[k as usize].nghi_trang());
    if co_xoay {
        muc(ui, BieuTuong::XoayPhai, "Xoay đề xuất", "", Viec::XoayDeXuat, true, false);
        muc(ui, BieuTuong::HoanTac, "Về chiều gốc", "C", Viec::VeGoc, true, false);
        muc(ui, BieuTuong::Dau, "Xác nhận và tiếp", "Enter", Viec::XacNhanXoay, true, false);
    }
    if let Some(k) = chon.tieu_diem.or(ds.first().copied()) {
        muc(ui, BieuTuong::KinhCong, "Xem lớn", if co_xoay || co_nghi { "bấm đúp" } else { "Enter" }, Viec::PhongTo(k), true, false);
    }
    ui.separator();
    let co_noi_dung = dem(Nhan::NoiDung) + dem(Nhan::Split) > 0;
    muc(ui, BieuTuong::Keo, "Split — văn bản mới từ trang này", "S", Viec::Gan(Nhan::Split), co_noi_dung, tat(Nhan::Split));
    muc(ui, BieuTuong::MaVach, "Index", "I", if co_nghi { Viec::DuyetNghiTrang(Nhan::Index) } else { Viec::Gan(Nhan::Index) }, true, tat(Nhan::Index));
    muc(ui, BieuTuong::TrangTrong, "Trắng", if co_nghi { "B · Enter" } else { "B" }, if co_nghi { Viec::DuyetNghiTrang(Nhan::Trang) } else { Viec::Gan(Nhan::Trang) }, true, tat(Nhan::Trang));
    if co_nghi {
        muc(ui, BieuTuong::BoNhan, "Giữ nội dung (bỏ cờ nghi trắng)", "C", Viec::DuyetNghiTrang(Nhan::NoiDung), true, false);
    } else {
        muc(ui, BieuTuong::BoNhan, "Bỏ nhãn (nội dung)", "C", Viec::Gan(Nhan::NoiDung), !tat(Nhan::NoiDung), false);
    }
    ui.separator();
    muc(ui, BieuTuong::XoayPhai, "Xoay phải 90°", "R", Viec::Xoay(90), true, false);
    muc(ui, BieuTuong::Xoay180, "Xoay 180°", "Shift+R", Viec::Xoay(180), true, false);
    ui.separator();
    muc(ui, BieuTuong::ThungRac, "Xoá khỏi lưới", "Del", Viec::Xoa, true, false);
    ui.separator();
    muc(ui, BieuTuong::ChonTat, "Chọn tất cả", "Ctrl+A", Viec::ChonTat, true, false);
    muc(ui, BieuTuong::BoChon, "Bỏ chọn", "Esc", Viec::BoChon, true, false);
}

pub(super) const CAO_DAU_BANG: f32 = 28.0;
pub(super) const CAO_DONG_BANG: f32 = 28.0;
/// Vạch mảnh giữa các dòng bảng (#F0F0F0).
pub(super) const VACH_DONG: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);

/// Trước `TableBuilder` của cột trái: nền xám dải tiêu đề + viền dưới, dòng chọn nền xanh nhạt, bỏ khoảng cách dọc.
pub(super) fn bat_dau_bang(ui: &mut egui::Ui) {
    let dau = Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), CAO_DAU_BANG));
    ui.painter().rect_filled(dau, 0.0, NEN_TIEU_DE);
    ui.painter().hline(dau.x_range(), dau.bottom() - 0.5, Stroke::new(1.0, VIEN_NHAT));
    ui.visuals_mut().selection.bg_fill = NEN_TAB_CHON;
    // Cột sát nhau (lề chữ nằm trong ô) để vạch dòng và nền dòng chọn liền một dải
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    // Chữ trong ô không bắt chuột (mặc định egui cho bôi chọn chữ) → bấm chỗ nào trên dòng cũng chọn dòng
    ui.style_mut().interaction.selectable_labels = false;
}

/// Panel cột trái: mở rộng = kéo đổi độ rộng 160–520 px; thu gọn = cố định 36 px (id riêng để không đè độ rộng đã nhớ).
pub(super) fn cot_trai_panel(id: &'static str, gon: bool, rong: f32) -> egui::Panel {
    let khung = egui::Frame::new().fill(Color32::WHITE);
    if gon {
        egui::Panel::left(egui::Id::unique(id).with("gon")).resizable(false).exact_size(cot_trai::RONG_GON).frame(khung)
    } else {
        egui::Panel::left(id).resizable(true).default_size(rong).size_range(160.0..=520.0).frame(khung)
    }
}

/// Lề chữ hai bên trong ô bảng.
pub(super) const LE_O_BANG: f32 = 5.0;

/// Ô bảng cột trái: số canh giữa (`giua`), chữ canh trái.
pub(super) fn o_so(ui: &mut egui::Ui, chu: RichText, giua: bool) {
    if giua {
        ui.centered_and_justified(|ui| {
            ui.add(egui::Label::new(chu).truncate());
        });
    } else {
        ui.add_space(LE_O_BANG);
        ui.add(egui::Label::new(chu).truncate());
    }
}

/// Vạch dưới ô.
pub(super) fn ke_duoi(ui: &egui::Ui, mau: Color32) {
    let r = ui.max_rect();
    ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, mau));
}

/// Vạch đứng ngăn nhóm nút trên thanh công cụ.
pub(super) fn ngan(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 18.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), Stroke::new(1.0, VIEN_NHAT));
}

/// Nút nhỏ có viền (`.nut.nho`): S / I / B / Xoá trang.
pub(super) fn nut_nho(chu: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(chu).min_size(egui::vec2(24.0, 22.0))
}

/// Tab lọc phẳng trên thanh công cụ: đang chọn = nền xanh nhạt + gạch dưới 2 px + chữ đậm; `mui` = có ▾ (mở menu).
fn tab_loc(ui: &mut egui::Ui, chu: &str, chon: bool, mui: bool) -> egui::Response {
    let font = if chon { egui::FontId::new(13.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(13.0) };
    let rong_chu = ui.fonts_mut(|f| f.layout_no_wrap(chu.to_owned(), font.clone(), CHU).size().x);
    let rong = if mui { (rong_chu + 40.0).max(128.0) } else { rong_chu + 20.0 };
    let (r, ph) = ui.allocate_exact_size(egui::vec2(rong, ui.available_height()), Sense::click());
    ph.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chon, chu));
    if chon {
        ui.painter().rect_filled(r, 0.0, NEN_TAB_CHON);
        ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(2.0, VIEN_TAB_CHON));
    } else if ph.hovered() {
        ui.painter().rect_filled(r, 0.0, crate::gui::NUT_HOVER);
    }
    let x_chu = if mui { r.left() + 14.0 } else { r.left() + 10.0 };
    ui.painter().text(egui::pos2(x_chu, r.center().y), egui::Align2::LEFT_CENTER, chu, font, CHU);
    if mui {
        bieu_tuong::ve(ui.painter(), Rect::from_center_size(egui::pos2(r.right() - 16.0, r.center().y), egui::vec2(11.0, 11.0)), BieuTuong::ChevronXuong, CHU_PHU);
    }
    ph
}

/// Rộng menu lọc "Tất cả ▾".
const RONG_MENU_LOC: f32 = 156.0;
/// Chữ nút S (cam nâu như prototype).
pub(super) const MAU_S: Color32 = Color32::from_rgb(0xB8, 0x5F, 0x00);
/// Nền nhóm nút cùng loại trên thanh công cụ (xanh xám nhạt) và nền khi rê chuột.
const NEN_NHOM_NUT: Color32 = Color32::from_rgb(0xED, 0xF1, 0xF6);
const NEN_NHOM_HOVER: Color32 = Color32::from_rgb(0xD9, 0xE4, 0xF1);

/// Nội dung 1 nút trong nhóm: chữ đậm (màu riêng nếu có) hoặc icon nét mảnh.
pub(super) enum NoiDungNut {
    Chu(&'static str, Option<Color32>),
    Icon(BieuTuong),
}

/// Nhóm nút cùng loại: nền #EDF1F6 bo 4, nút không viền sát nhau.
pub(super) fn nhom_nut(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(NEN_NHOM_NUT).corner_radius(4).inner_margin(Margin::symmetric(2, 1)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        ui.horizontal(f);
    });
}

/// Vạch ngăn mảnh giữa các nút trong nhóm.
pub(super) fn ngan_nho(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(5.0, 14.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), Stroke::new(1.0, Color32::from_rgb(0xC9, 0xD2, 0xDD)));
}

/// Nút 26 × 20 trong nhóm. `(het, mot_phan)`: mọi trang chọn mang nhãn này = nền xanh đậm chữ trắng; chỉ một phần = viền
/// xanh trong. Vô hiệu thì mờ; tooltip hiện cả khi vô hiệu.
pub(super) fn nut_nhom(ui: &mut egui::Ui, nd: NoiDungNut, (het, mot_phan): (bool, bool), bat: bool, goi_y: &str) -> egui::Response {
    let (r, ph) = ui.allocate_exact_size(egui::vec2(26.0, 20.0), if bat { Sense::click() } else { Sense::hover() });
    let ten = match &nd {
        NoiDungNut::Chu(c, _) => c.to_string(),
        NoiDungNut::Icon(_) => goi_y.split([' ', '(']).next().unwrap_or_default().to_string(),
    };
    ph.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, bat, &ten));
    let mo = if bat { 1.0 } else { 0.4 };
    let p = ui.painter();
    if bat && het {
        p.rect_filled(r, 3.0, XANH_NUT);
    } else if bat && mot_phan {
        p.rect(r, 3.0, Color32::WHITE, Stroke::new(2.0, XANH_NUT), StrokeKind::Inside);
    } else if bat && ph.hovered() {
        p.rect_filled(r, 3.0, NEN_NHOM_HOVER);
    }
    match nd {
        NoiDungNut::Chu(c, mau) => {
            let mau = if bat && het { Color32::WHITE } else if bat && mot_phan { XANH_NUT } else { mau.unwrap_or(CHU) };
            p.text(r.center(), egui::Align2::CENTER_CENTER, c, egui::FontId::new(12.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), mau.gamma_multiply(mo));
        }
        NoiDungNut::Icon(bt) => bieu_tuong::ve(p, Rect::from_center_size(r.center(), egui::vec2(16.0, 16.0)), bt, Color32::from_gray(0x2B).gamma_multiply(mo)),
    }
    if bat && ph.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ph.on_hover_text(goi_y)
}

/// Thu nhỏ · % · phóng to (trong layout phải → trái: + nằm trước). `vat` = "ảnh" hoặc "" cho tooltip.
fn nut_thu_phong(ui: &mut egui::Ui, phan_tram: u32, nho_duoc: bool, lon_duoc: bool, vat: &str, viec: &mut Vec<Viec>, nho: Viec, lon: Viec) {
    let vat = if vat.is_empty() { String::new() } else { format!(" {vat}") };
    if ui.add_enabled(lon_duoc, nut_phang(atom(BieuTuong::KinhCong, 15.0), false)).on_hover_text(format!("Phóng to{vat} (Ctrl + cuộn lên) · tối đa 250 % như khi bấm vào từng trang")).clicked() {
        viec.push(lon);
    }
    ui.add_sized(egui::vec2(38.0, 22.0), egui::Label::new(RichText::new(format!("{phan_tram}%")).size(12.0).color(CHU_PHU)));
    if ui.add_enabled(nho_duoc, nut_phang(atom(BieuTuong::KinhTru, 15.0), false)).on_hover_text(format!("Thu nhỏ{vat} (Ctrl + cuộn xuống)")).clicked() {
        viec.push(nho);
    }
}

/// Xoay phải 90° / 180° các trang đang chọn (trong layout phải → trái: 180 nằm sau 90).
fn nut_xoay(ui: &mut egui::Ui, co_chon: bool, viec: &mut Vec<Viec>) {
    if ui.add_enabled(co_chon, nut_phang((atom(BieuTuong::Xoay180, 14.0), RichText::new("180").size(10.0)), false)).on_hover_text("Xoay 180° các trang đang chọn (Shift+R)").on_disabled_hover_text("Chọn trang trước (Shift+R)").clicked() {
        viec.push(Viec::Xoay(180));
    }
    if ui.add_enabled(co_chon, nut_phang(atom(BieuTuong::XoayPhai, 15.0), false)).on_hover_text("Xoay phải 90° các trang đang chọn (R) — chỉ áp khi tách, không sửa file gốc").on_disabled_hover_text("Chọn trang trước (R)").clicked() {
        viec.push(Viec::Xoay(90));
    }
}

/// Định dạng chữ 12 px của các dòng trạng thái / thông tin.
pub(super) fn chu_dong(mau: Color32, dam: bool) -> egui::TextFormat {
    let ho = if dam { egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into()) } else { egui::FontFamily::Proportional };
    egui::TextFormat { color: mau, font_id: egui::FontId::new(12.0, ho), ..Default::default() }
}

/// Viền nét đứt quanh hình chữ nhật.
pub(super) fn ve_net_dut(painter: &egui::Painter, r: Rect, net: Stroke, dai: f32) {
    let diem = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&diem, net, dai, dai * 0.75));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chọn thư mục (chưa Phân tích): danh sách PDF đọc nền, hiện tên ngay ở cột trái.
    #[test]
    fn chon_thu_muc_hien_ngay_ten_pdf() {
        let goc = std::env::temp_dir().join(format!("dema4_xem_truoc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join("con")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("b.pdf"), 2);
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("con").join("a.pdf"), 3);
        let mut man = ManTach { thu_muc: goc.display().to_string(), ..Default::default() };
        let bat_dau = Instant::now();
        while man.xem_truoc.ten.is_empty() && bat_dau.elapsed() < Duration::from_secs(10) {
            man.cap_nhat_xem_truoc();
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut ten = man.xem_truoc.ten.clone();
        ten.sort();
        assert_eq!(ten, ["a", "b"]);
        assert!(man.xem_truoc.so_trang.iter().all(|s| s.is_none()), "chưa có cache → chưa có số trang");
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn gio_phut_giay_theo_mau() {
        assert_eq!(gio_phut_giay(Duration::from_secs(10)), "00:00:10");
        assert_eq!(gio_phut_giay(Duration::from_secs(610)), "00:10:10");
        assert_eq!(gio_phut_giay(Duration::from_millis(3_725_400)), "01:02:05");
        assert_eq!(phan_tram_nac(3), 26);
        assert_eq!(phan_tram_nac(11), 250);
        assert_eq!(co_vua_khung(11, 600.0), 600.0 - 2.0 * LE - DEM_O);
        assert_eq!(co_vua_khung(3, 1600.0), 110.0);
    }

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
            man.nhan_su_kien(&ch, None);
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
            man2.nhan_su_kien(&ch, None);
        }
        assert_eq!(man2.bang.as_ref().unwrap().trang[1].nhan(), Nhan::Index);
        // Bật thêm quét mã trong Cấu hình → tự chạy bổ sung phần còn thiếu, nhãn sửa tay giữ nguyên
        let cfg_moi = CauHinh { quet_ma: true, ..cfg.clone() };
        man2.ap_cau_hinh(&cfg, &cfg_moi);
        assert!(man2.dang_chay(), "phải chạy bổ sung đọc mã");
        let ch_moi = cfg_moi.cau_hinh_phan_tich();
        while man2.dang_chay() {
            std::thread::sleep(Duration::from_millis(20));
            man2.nhan_su_kien(&ch_moi, None);
        }
        assert!(man2.phien.as_ref().unwrap().ho_so[0].trang.iter().all(|t| t.ket_qua.as_ref().is_some_and(|k| k.da_quet_ma)));
        assert_eq!(man2.bang.as_ref().unwrap().trang[1].nhan(), Nhan::Index);
        man2.ap_cau_hinh(&cfg_moi, &cfg_moi);
        assert!(!man2.dang_chay(), "không đổi gì thì không chạy");
        let _ = std::fs::remove_dir_all(&goc);
    }

    /// Đo bộ 100k trang sinh giả (200 file × 500 trang): mở thư mục (đọc dung lượng), dựng bảng, lọc / sắp.
    /// Chạy: cargo test --release -- --ignored --nocapture do_100k_trang
    #[test]
    #[ignore = "đo hiệu năng, tạo ~200 PDF tạm"]
    fn do_100k_trang() {
        let goc = std::env::temp_dir().join("dema4_100k");
        if !goc.join("199/hs.pdf").exists() {
            for f in 0..200 {
                std::fs::create_dir_all(goc.join(f.to_string())).unwrap();
                crate::tach_vb::tach::tests::tao_pdf(&goc.join(format!("{f}/hs.pdf")), 500);
            }
        }
        let t = Instant::now();
        let phien = PhienPhanTich::mo(&goc, &AtomicBool::new(false));
        let t_mo = t.elapsed();
        let ch = CauHinhPhanTich::default();
        let t = Instant::now();
        let b = BangTrang::tu_phien(&phien, &ch, None, None);
        let t_bang = t.elapsed();
        let t = Instant::now();
        let tt = b.thu_tu(Loc::TatCa, None, Sap::DungLuongTang);
        let t_sap = t.elapsed();
        let t = Instant::now();
        let n_vb: usize = (0..b.ho_so.len()).map(|h| b.van_ban(h).van_ban.len()).sum();
        let t_vb = t.elapsed();
        println!(
            "{} trang: mở {:.2} s · bảng {:.0} ms · sắp dung lượng {:.0} ms · tính {} văn bản {:.0} ms · bảng ~{} MB",
            tt.len(),
            t_mo.as_secs_f64(),
            t_bang.as_secs_f64() * 1e3,
            t_sap.as_secs_f64() * 1e3,
            n_vb,
            t_vb.as_secs_f64() * 1e3,
            b.trang.len() * std::mem::size_of::<bang_trang::TrangMeta>() / 1_000_000
        );
        assert_eq!(tt.len(), 100_000);
    }

    /// Đo render ảnh nhỏ 120 px (đủ 1 màn ~60 ô) trên trang scan thật.
    #[test]
    #[ignore = "cần DEMA4_MAU2"]
    fn do_render_anh_nho() {
        let thu_muc = std::env::var("DEMA4_MAU2").expect("DEMA4_MAU2");
        let (gui, _n) = mpsc::channel();
        let pdf = crate::dem_pdf::tim_file_pdf(&thu_muc, &AtomicBool::new(false), &gui);
        let doc = mupdf::Document::open(pdf[0].to_str().unwrap()).unwrap();
        let n = doc.page_count().unwrap().min(60) as usize;
        let t = Instant::now();
        for so in 1..=n {
            let trang = doc.load_page(so as i32 - 1).unwrap();
            let hop = trang.bounds().unwrap();
            let k = 120.0 / (hop.x1 - hop.x0);
            trang.to_pixmap(&mupdf::Matrix::new_scale(k, k), &mupdf::Colorspace::device_rgb(), false, false).unwrap();
        }
        let ms = t.elapsed().as_secs_f64() * 1e3 / n as f64;
        println!("{n} ảnh nhỏ: {ms:.1} ms/ảnh → 60 ô / 2 luồng ≈ {:.2} s", ms * 30.0 / 1e3);
    }
}
