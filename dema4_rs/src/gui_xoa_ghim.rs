//! Màn Xóa ghim — bố cục theo prototype đã chốt (`plans/261126-0727-prototype-man-xoa-ghim/`):
//! - trái: **Danh sách** file (kéo rộng 200–520, thu nhỏ thành dải dọc), sắp xếp, trạng thái, lọc gộp, Lọc nâng cao;
//! - trên: thanh công cụ 2 nửa thẳng ranh giới Gốc | Đích (Góc / Cạnh / Tùy biến · lọc trang · Vùng / Rộng / Cao /
//!   Ngưỡng ‖ Đầu vào · File đích · tuỳ chọn);
//! - giữa: khung **Gốc** (cột trang thu nhỏ + trang) và **Đích** rộng bằng nhau, cuộn đồng bộ;
//! - đáy: đếm vùng, chuyển trang, kiểu xem, tỉ lệ, nút **Xóa ghim (n)**.
//!
//! Góc / Cạnh = vùng chung thư mục; Tùy biến (vẽ tay) = vùng riêng của file. Ảnh Đích vẽ ở đúng DPI xuất bằng đúng
//! bước xoá. Vùng / tuỳ chọn theo thư mục lưu kho Xóa ghim; tuỳ chọn chung (hiển thị, hiệu năng) lưu cấu hình app.

mod bieu_tuong;
mod chay;
mod danh_sach;
mod hop;
mod khung_trang;
mod sua_vung;
mod thanh_mini;
mod thanh_tren;
mod the_cau_hinh;
mod thiet_ke;
mod ve_vung;
mod widget;
mod xem_truoc;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, TextureHandle};

use crate::cau_hinh::CauHinhXoaGhim;
use crate::xoa_ghim::kho::{self as kho_xg, CauHinhXuat, ThuMuc};
use crate::xoa_ghim::vung::{ApTrang, LoaiVung, ViTri, Vung};
use crate::xoa_ghim::xuat;
use sua_vung::{BoVung, Canh, ChonVung};
use xem_truoc::{CauHinhXem, KetQua, XemTruoc, YeuCau};

pub use the_cau_hinh::{CauHinhThuMuc, TabXoaGhim};
pub use widget::{HO_FONT_XG, HO_FONT_XG_DAM};

pub const TIEU_DE: &str = "Xóa ghim (Giải pháp số Sao Mai)";
/// Ghi vùng / tuỳ chọn vào kho sau khi ngừng chỉnh chừng này (kéo thanh trượt không ghi liên tục).
const TRE_GHI: Duration = Duration::from_millis(600);
/// Thông báo nhanh hiện trong chừng này.
const THOI_TOAST: Duration = Duration::from_millis(2400);

/// Đăng ký biểu tượng SVG của màn (gọi 1 lần khi dựng giao diện).
pub fn dang_ky_bieu_tuong(ctx: &egui::Context) {
    bieu_tuong::dang_ky(ctx);
}

/// Thư mục chưa chỉnh lần nào: chưa có vùng nào (góc / cạnh / Tùy biến đều tắt) — người dùng tự chọn.
pub fn thu_muc_mac_dinh() -> ThuMuc {
    ThuMuc::default()
}

/// Lọc trang (cột nút tròn): Tất cả / Lẻ / Chẵn / Từng trang (+ Vô đối khi vẽ / chọn vùng xoá vẽ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocTrang {
    TatCa,
    Le,
    Chan,
    TungTrang,
    VoDoi,
}

impl LocTrang {
    fn ap(self, trang: u32) -> ApTrang {
        match self {
            LocTrang::TatCa => ApTrang::TatCa,
            LocTrang::Le => ApTrang::Le,
            LocTrang::Chan => ApTrang::Chan,
            LocTrang::TungTrang | LocTrang::VoDoi => ApTrang::MotTrang(trang),
        }
    }

    /// Lựa chọn tương ứng phạm vi / loại của 1 vùng (hiện ở cột lọc trang khi chọn đúng 1 vùng).
    fn cua_vung(z: &Vung) -> Self {
        if z.loai == LoaiVung::VoDoi {
            return LocTrang::VoDoi;
        }
        match z.ap_trang {
            ApTrang::TatCa => LocTrang::TatCa,
            ApTrang::Le => LocTrang::Le,
            ApTrang::Chan => LocTrang::Chan,
            ApTrang::MotTrang(_) => LocTrang::TungTrang,
        }
    }
}

/// Sắp xếp danh sách: theo tên (tự nhiên) hoặc số trang — 1 cột mỗi lần.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SapTheo {
    Ten,
    Trang,
}

/// Lọc trạng thái (ô lọc gộp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocTrangThai {
    TatCa,
    CanChay,
    ChuaXoa,
    DaXoa,
    DaDoi,
    MatKetQua,
}

impl LocTrangThai {
    pub const DS: [LocTrangThai; 6] = [LocTrangThai::TatCa, LocTrangThai::CanChay, LocTrangThai::ChuaXoa, LocTrangThai::DaXoa, LocTrangThai::DaDoi, LocTrangThai::MatKetQua];
    pub fn ten(self) -> &'static str {
        match self {
            LocTrangThai::TatCa => "Tất cả",
            LocTrangThai::CanChay => "Cần chạy",
            LocTrangThai::ChuaXoa => "Chưa xóa",
            LocTrangThai::DaXoa => "Đã xóa",
            LocTrangThai::DaDoi => "Đã đổi vùng",
            LocTrangThai::MatKetQua => "Mất kết quả",
        }
    }
}

/// Điều kiện Lọc nâng cao: có trang khổ A0–A7 (đúng chiều) trong dải trang.
#[derive(Clone, Debug, PartialEq)]
pub struct LocNangCao {
    pub kho: [bool; 8],
    pub ngang: bool,
    pub doc: bool,
    pub dai: String,
}

impl Default for LocNangCao {
    fn default() -> Self {
        LocNangCao { kho: [false, false, false, true, true, false, false, false], ngang: true, doc: true, dai: String::new() }
    }
}

/// Số trang + cỡ (rộng, cao — điểm PDF, như màn Đếm) từng trang — đọc ở luồng nền. Khổ tính lúc lọc theo quy tắc
/// Cấu hình › Đếm trang A4 đang dùng (đổi ranh giới không phải đọc lại file).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThongTinFile {
    pub so_trang: u32,
    pub trang: Vec<(f64, f64)>,
}

impl ThongTinFile {
    /// (khổ A0..A7, ngang?) từng trang theo quy tắc `qt` — như màn Đếm: khổ theo cạnh dài, ngang khi rộng > cao.
    pub fn kho_chieu<'a>(&'a self, qt: &'a crate::dem_pdf::QuyTac) -> impl Iterator<Item = (usize, bool)> + 'a {
        self.trang.iter().map(move |&(w, h)| (qt.kho(w, h), w > h))
    }
}

/// Tỉ lệ hiển thị: vừa bề ngang / vừa chiều cao khung (tính lại theo khung) hoặc cố định (điểm màn / 1 điểm PDF).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TyLe {
    VuaNgang,
    VuaCao,
    CoDinh(f32),
}

/// Đang kéo chuột trên trang Gốc.
#[derive(Clone, Debug)]
enum Keo {
    /// Vẽ vùng mới: trang, điểm bắt đầu (màn hình).
    Moi { trang: u32, dau: Pos2 },
    /// Di chuyển / đổi cỡ 1 vùng.
    Vung { trang: u32, chon: ChonVung, canh: Canh, goc: Vung, dau: Pos2 },
    /// Kéo cả nhóm đang chọn.
    Nhom { trang: u32, goc: Vec<(ChonVung, Vung)>, dau: Pos2 },
    /// Quét khung chọn (sang phải = Window, sang trái = Crossing); `them` = giữ Shift.
    Quet { trang: u32, dau: Pos2, them: bool },
}

/// Ảnh 1 trang đang hiện: gốc + đích (đích theo cấu hình mã `ma`).
struct AnhTrang {
    goc: TextureHandle,
    dich: TextureHandle,
    ma: u64,
    dpi: u16,
    co_anh: (usize, usize),
    /// Khung nhận diện layout (điểm ảnh của `co_anh`) — tô đỏ nhạt trên Gốc.
    ai: Vec<crate::xoa_ghim::vung::Hcn>,
}

pub struct ManXoaGhim {
    /// Thư mục nguồn (mở 1 file: thư mục chứa file — vùng chung theo thư mục này).
    pub thu_muc: Option<PathBuf>,
    /// Đang mở đúng 1 file PDF.
    mot_file: Option<PathBuf>,
    /// (PDF nguồn, PDF kết quả) theo thứ tự cây thư mục.
    ds: Vec<(PathBuf, PathBuf)>,
    /// File được tick để chạy.
    chon_chay: Vec<bool>,
    thong_tin: Vec<Option<ThongTinFile>>,
    nhan_thong_tin: Option<Receiver<(usize, ThongTinFile)>>,
    // --- Danh sách ---
    sap_theo: SapTheo,
    sap_giam: bool,
    loc_ten: String,
    loc_tt: LocTrangThai,
    /// Chữ gõ vào ô lọc gộp (lọc theo số trang); trống = lọc theo `loc_tt`.
    loc_so: String,
    loc_nc: LocNangCao,
    loc_nc_ap: Option<LocNangCao>,
    loc_nc_bat: bool,
    loc_nc_mo: bool,
    /// `None` = mặc định (mở rộng); `Some(false)` = người dùng đã thu nhỏ.
    ds_mo: Option<bool>,
    chon: usize,
    bo: BoVung,
    /// Đang vẽ vùng tay: `Some(Xoa)` = "−", `Some(BaoVe)` = "+"; `None` = chọn / kéo vùng.
    ve: Option<LoaiVung>,
    loc_trang: LocTrang,
    an_cong_cu: bool,
    keo: Option<Keo>,
    // --- Xem ---
    xem: XemTruoc,
    the_he: u64,
    /// Khổ từng trang (điểm) của file đang xem; trống = đang mở.
    kho_trang: Vec<(f32, f32)>,
    anh: HashMap<u32, AnhTrang>,
    cho_anh: HashSet<(u32, u64)>,
    anh_nho: HashMap<u32, TextureHandle>,
    cho_nho: HashSet<u32>,
    ty_le: TyLe,
    /// Tỉ lệ đang hiển thị ở khung vừa vẽ (cho − / +, ô tỉ lệ).
    ty_le_cuoi: f32,
    trang: u32,
    /// Cuộn tới trang này ở khung tới (chuyển trang / file).
    cuon_toi: Option<u32>,
    /// Offset cuộn 2 khung ở khung trước + khung cần ép theo khung kia.
    cuon_goc: egui::Vec2,
    cuon_dich: egui::Vec2,
    ep_cuon: Option<(bool, egui::Vec2)>,
    loi: Option<String>,
    can_ghi: Option<Instant>,
    pub license: std::sync::Arc<crate::license::KiemTra>,
    /// Trạng thái từng file (theo `ds`); trống = đang đọc.
    tinh_trang: Vec<crate::xoa_ghim::lo::TinhTrangFile>,
    nhan_tt: Option<Receiver<Vec<crate::xoa_ghim::lo::TinhTrangFile>>>,
    chay: Option<chay::DangChay>,
    da_chay: Option<chay::DaChay>,
    hop_license: Option<crate::hop_license::HopLicense>,
    cho_chay: Option<bool>,
    so_da_xoa: Option<(usize, usize)>,
    /// Bấm "Mở ở Tách" ở hộp kết quả → màn chính chuyển sang Tách với thư mục này.
    pub mo_o_tach: Option<PathBuf>,
    /// Bấm logo Sao Mai ở màn trống → về Trang chủ.
    pub ve_trang_chu: bool,
    // --- Hộp / thông báo ---
    /// Hộp xác nhận "Xóa n vùng vẽ?".
    xac_nhan_xoa: Option<Vec<ChonVung>>,
    tro_giup: bool,
    toast: Option<(String, Instant)>,
    /// Tuỳ chọn chung (cấu hình app) — đồng bộ ra ngoài khi đổi.
    pub cfg: CauHinhXoaGhim,
    /// Quy tắc xếp khổ của Cấu hình › Đếm trang A4 (Lọc nâng cao xếp khổ giống màn Đếm); app cập nhật khi lưu cấu hình.
    pub quy_tac: crate::dem_pdf::QuyTac,
    /// Bề rộng Danh sách đang kéo (chưa ghi cấu hình tới khi thả chuột).
    rong_keo: Option<f32>,
    /// Neo trục đứng trang đang xem vào trục giữa khung Gốc / Đích (bỏ khi người dùng tự cuộn ngang).
    neo_ngang: bool,
    /// Bề rộng khung xem ở khung hình trước (đổi → neo lại).
    rong_xem_cu: f32,
    /// Cuộn dọc cần giữ sau khi đổi tỉ lệ: (Gốc, Đích, số khung hình còn áp).
    neo_doc: Option<(f32, f32, u8)>,
    /// Cao khối Lọc nâng cao đo ở khung trước (danh sách file chừa chỗ đúng bằng nó).
    cao_nc: f32,
    cfg_doi: bool,
    /// Việc tải / nhập mô hình AI đang chạy (nhận diện layout); `None` = không có việc.
    pub mo_hinh: Option<crate::xoa_ghim::mo_hinh::TaiMoHinh>,
    /// Bấm "tải mô hình" → màn chính mở Cấu hình ở tab Xóa ghim.
    pub mo_cau_hinh: bool,
}

impl Default for ManXoaGhim {
    fn default() -> Self {
        ManXoaGhim {
            thu_muc: None,
            mot_file: None,
            ds: Vec::new(),
            chon_chay: Vec::new(),
            thong_tin: Vec::new(),
            nhan_thong_tin: None,
            sap_theo: SapTheo::Ten,
            sap_giam: false,
            loc_ten: String::new(),
            loc_tt: LocTrangThai::TatCa,
            loc_so: String::new(),
            loc_nc: LocNangCao::default(),
            loc_nc_ap: None,
            loc_nc_bat: true,
            loc_nc_mo: false,
            ds_mo: None,
            chon: 0,
            bo: BoVung::moi(thu_muc_mac_dinh()),
            ve: None,
            loc_trang: LocTrang::TatCa,
            an_cong_cu: false,
            keo: None,
            xem: XemTruoc::default(),
            the_he: 0,
            kho_trang: Vec::new(),
            anh: HashMap::new(),
            cho_anh: HashSet::new(),
            anh_nho: HashMap::new(),
            cho_nho: HashSet::new(),
            ty_le: TyLe::VuaNgang,
            ty_le_cuoi: 1.0,
            trang: 0,
            cuon_toi: None,
            cuon_goc: egui::Vec2::ZERO,
            cuon_dich: egui::Vec2::ZERO,
            ep_cuon: None,
            loi: None,
            can_ghi: None,
            license: crate::license::KiemTra::mac_dinh(),
            tinh_trang: Vec::new(),
            nhan_tt: None,
            chay: None,
            da_chay: None,
            hop_license: None,
            cho_chay: None,
            so_da_xoa: None,
            mo_o_tach: None,
            ve_trang_chu: false,
            xac_nhan_xoa: None,
            tro_giup: false,
            toast: None,
            cfg: CauHinhXoaGhim::default(),
            quy_tac: crate::dem_pdf::QuyTac::default(),
            rong_keo: None,
            cao_nc: 31.0,
            neo_ngang: true,
            rong_xem_cu: 0.0,
            neo_doc: None,
            cfg_doi: false,
            mo_hinh: None,
            mo_cau_hinh: false,
        }
    }
}

impl ManXoaGhim {
    /// Đang mở thư mục / file.
    pub fn dang_mo(&self) -> bool {
        self.thu_muc.is_some()
    }

    /// Mở thư mục nguồn: liệt kê PDF theo kiểu ghi của thư mục (bỏ kết quả `*_xoaghim`), nạp vùng chung từ kho.
    pub fn mo_thu_muc(&mut self, p: PathBuf) {
        self.mo(p, None);
    }

    /// Mở `goc` (thư mục), hoặc đúng 1 file `file` trong thư mục `goc`.
    fn mo(&mut self, goc: PathBuf, file: Option<PathBuf>) {
        self.ghi_ngay();
        let tm = kho_xg::doc_thu_muc(&mut kho_xg::kho(), &goc).unwrap_or_else(thu_muc_mac_dinh);
        self.bo = BoVung::moi(tm);
        self.thu_muc = Some(goc);
        self.mot_file = file;
        self.lap_ds();
        self.chon = usize::MAX;
        self.chon_file(0);
        // Như prototype: vùng đầu tiên đang chọn để thanh Rộng / Cao / Ngưỡng sẵn sàng
        self.bo.chon = self.bo.tat_ca().into_iter().take(1).collect();
        self.so_da_xoa = None;
        self.ds_mo = None;
        self.loc_ten.clear();
        self.loc_so.clear();
        self.loc_tt = LocTrangThai::TatCa;
        self.loc_nc_ap = None;
        self.ty_le = TyLe::VuaNgang;
    }

    /// Dựng lại danh sách việc (nguồn → đích) theo kiểu ghi hiện tại; đọc lại số trang + trạng thái.
    fn lap_ds(&mut self) {
        let Some(goc) = self.thu_muc.clone() else { return };
        let ghi = self.bo.tm.ghi.clone();
        self.loi = None;
        self.ds = match &self.mot_file {
            Some(f) => match xuat::dich_cua(&goc, f, &ghi) {
                Ok(d) => vec![(f.clone(), d)],
                Err(e) => {
                    self.loi = Some(e);
                    Vec::new()
                }
            },
            None => match xuat::ds_viec_ghi(&goc, &ghi, &std::sync::atomic::AtomicBool::new(false)) {
                Ok(ds) => ds,
                Err(e) => {
                    self.loi = Some(e);
                    Vec::new()
                }
            },
        };
        self.chon_chay = vec![true; self.ds.len()];
        self.doc_thong_tin();
        self.tinh_trang.clear();
        self.doc_trang_thai();
        if self.ds.is_empty() && self.loi.is_none() {
            self.loi = Some("Thư mục không có PDF".into());
        }
    }

    /// Hộp chọn thư mục PDF.
    pub fn chon_thu_muc(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục PDF cần xoá ghim").pick_folder() {
            self.mo_thu_muc(p);
        }
    }

    /// Hộp chọn 1 file PDF: mở đúng file đó (vùng chung theo thư mục chứa nó).
    pub fn chon_file_pdf(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Chọn file PDF cần xoá ghim").add_filter("PDF", &["pdf", "PDF"]).pick_file() {
            self.mo_pdf(p);
        }
    }

    fn mo_pdf(&mut self, p: PathBuf) {
        if let Some(cha) = p.parent() {
            self.mo(cha.to_path_buf(), Some(p));
        }
    }

    /// ⊗ / Đóng thư mục: về màn ban đầu.
    pub fn dong(&mut self) {
        self.ghi_ngay();
        if let Some(c) = &self.chay {
            c.dung.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let (license, cfg, mo_hinh, quy_tac) = (self.license.clone(), self.cfg.clone(), std::mem::take(&mut self.mo_hinh), self.quy_tac);
        *self = ManXoaGhim { license, cfg, mo_hinh, quy_tac, ..Default::default() };
    }

    /// Thư mục kết quả hiện tại (theo kiểu ghi; ghi cạnh file / ghi đè = thư mục nguồn).
    pub fn thu_muc_ket_qua_hien_tai(&self) -> Option<PathBuf> {
        let goc = self.thu_muc.as_ref()?;
        let ghi = &self.bo.tm.ghi;
        if ghi.ghi_de || ghi.kieu == kho_xg::KieuGhi::CanhFile {
            return Some(goc.clone());
        }
        match &ghi.thu_muc {
            Some(d) => Some(d.clone()),
            None => crate::tach_vb::tach::thu_muc_ra_hau_to(goc, None, xuat::HAU_TO).ok(),
        }
    }

    /// Số trang + khổ / chiều từng trang ở luồng nền (danh sách hiện dần; Lọc nâng cao dùng khổ).
    fn doc_thong_tin(&mut self) {
        self.thong_tin = vec![None; self.ds.len()];
        let ds: Vec<PathBuf> = self.ds.iter().map(|(n, _)| n.clone()).collect();
        let (gui, nhan) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for (i, p) in ds.iter().enumerate() {
                if gui.send((i, doc_thong_tin_file(p))).is_err() {
                    return;
                }
            }
        });
        self.nhan_thong_tin = Some(nhan);
    }

    /// Ghi phần đã đổi xuống kho ngay (đổi file / thư mục / rời màn).
    pub fn ghi_ngay(&mut self) {
        self.can_ghi = None;
        let (chung, rieng) = (std::mem::take(&mut self.bo.doi_chung), std::mem::take(&mut self.bo.doi_rieng));
        if !chung && !rieng {
            return;
        }
        let mut k = kho_xg::kho();
        if let (true, Some(tm)) = (chung, &self.thu_muc) {
            kho_xg::ghi_thu_muc(&mut k, tm, &self.bo.tm);
        }
        if let (true, Some(id)) = (rieng, self.bo.id_file) {
            kho_xg::ghi_vung_file(&mut k, id, &self.bo.vf);
        }
        if let Err(e) = k.day() {
            self.loi = Some(format!("Chưa lưu được vùng: {e}"));
        }
    }

    fn chon_file(&mut self, i: usize) {
        if i >= self.ds.len() || i == self.chon {
            return;
        }
        self.ghi_ngay();
        self.chon = i;
        self.trang = 0;
        self.cuon_toi = Some(0);
        self.the_he += 1;
        self.kho_trang.clear();
        self.anh.clear();
        self.cho_anh.clear();
        self.anh_nho.clear();
        self.cho_nho.clear();
        self.keo = None;
        self.xem.gui(YeuCau::Mo { the_he: self.the_he, pdf: self.ds[i].0.clone() });
        // Id qua bảng đường dẫn của kho Tách (Dọn dữ liệu cũ dựa vào bảng này để biết PDF còn dùng)
        let id = crate::tach_vb::kho::ids_cua(std::slice::from_ref(&self.ds[i].0))[0];
        let mut k = kho_xg::kho();
        // Ghi đè file gốc: nội dung (id) đổi sau mỗi lần chạy — vùng riêng vẫn theo id trước lần ghi đè đầu
        let id = id.map(|id| if self.bo.tm.ghi.ghi_de { xuat::id_goc(&kho_xg::nguon_theo_ket_qua(&mut k), id) } else { id });
        let vf = id.and_then(|id| kho_xg::doc_vung_file(&mut k, id)).unwrap_or_default();
        drop(k);
        self.bo.doi_file(id, vf);
    }

    /// Cấu hình xoá hiện tại + mã của nó (đổi mã → ảnh Đích phải làm lại).
    fn cau_hinh_xem(&self) -> (CauHinhXem, u64) {
        let ch = CauHinhXem { vung: self.bo.vung(), tc: self.bo.tm.tuy_chon, dpi_chon: self.bo.tm.xuat.dpi };
        let ma = kho_xg::ma_cau_hinh(&ch.vung, &ch.tc, &CauHinhXuat { dpi: ch.dpi_chon, ..Default::default() });
        (ch, ma)
    }

    fn nhan_nen(&mut self, ctx: &egui::Context) {
        if let Some(n) = &self.nhan_thong_tin {
            for (i, tt) in n.try_iter() {
                if let Some(x) = self.thong_tin.get_mut(i) {
                    *x = Some(tt);
                }
            }
            if self.thong_tin.iter().all(Option::is_some) {
                self.nhan_thong_tin = None;
            }
        }
        for kq in self.xem.nhan() {
            match kq {
                KetQua::KhoTrang { the_he, kho } if the_he == self.the_he => self.kho_trang = kho,
                KetQua::AnhNho { the_he, trang, anh } if the_he == self.the_he => {
                    self.cho_nho.remove(&trang);
                    self.anh_nho.insert(trang, ctx.load_texture(format!("xg_nho_{trang}"), anh, egui::TextureOptions::LINEAR));
                }
                KetQua::Trang { the_he, trang, ma, dpi, co_anh, goc, dich, ai } if the_he == self.the_he => {
                    self.cho_anh.retain(|(t, _)| *t != trang);
                    let opt = egui::TextureOptions::LINEAR;
                    self.anh.insert(trang, AnhTrang { goc: ctx.load_texture(format!("xg_goc_{trang}"), goc, opt), dich: ctx.load_texture(format!("xg_dich_{trang}"), dich, opt), ma, dpi, co_anh, ai });
                    if self.loi.as_deref().is_some_and(|l| l.starts_with("Không mở được PDF")) {
                        self.loi = None;
                    }
                }
                KetQua::Loi { the_he, loi } if the_he == self.the_he => {
                    self.cho_anh.clear();
                    self.loi = Some(loi);
                }
                _ => {}
            }
        }
    }

    /// Vẽ cả màn. `cfg` = tuỳ chọn chung trong cấu hình app; trả `true` khi đã đổi (để app ghi file cấu hình).
    pub fn ve(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinhXoaGhim) -> bool {
        if self.cfg != *cfg && !self.cfg_doi {
            self.cfg = cfg.clone();
        }
        let ctx = ui.ctx().clone();
        widget::ap_kieu(ui);
        self.nhan_nen(&ctx);
        self.nhan_trang_thai();
        self.nhan_chay();
        self.cac_hop(&ctx);
        self.hop_phu(&ctx);
        if (self.bo.doi_chung || self.bo.doi_rieng) && self.can_ghi.is_none() {
            self.can_ghi = Some(Instant::now());
        }
        let tre = if self.cfg.tu_luu_phut == 0 { TRE_GHI } else { Duration::from_secs(self.cfg.tu_luu_phut as u64 * 60) };
        if self.can_ghi.is_some_and(|t| t.elapsed() >= tre) {
            self.ghi_ngay();
        } else if self.can_ghi.is_some() {
            ctx.request_repaint_after(tre.min(Duration::from_secs(30)));
        }
        if self.chay.is_none() && self.hop_license.is_none() && self.da_chay.is_none() && self.xac_nhan_xoa.is_none() && !self.tro_giup {
            self.phim(&ctx);
            self.tha_vao(&ctx);
        }
        self.danh_sach(ui);
        egui::Panel::bottom("xg_duoi").exact_size(thiet_ke::CAO_DAY).frame(egui::Frame::new().fill(thiet_ke::NEN_DAY).stroke(egui::Stroke::new(1.0, thiet_ke::VIEN_KHUNG)).inner_margin(egui::Margin::symmetric(12, 0))).show(ui, |ui| self.thanh_duoi(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::WHITE)).show(ui, |ui| {
            // Thu gọn (^): hàng biểu tượng mini thay thanh công cụ đầy đủ
            if self.an_cong_cu {
                self.thanh_mini(ui);
            } else {
                self.thanh_tren(ui);
            }
            self.khung_trang(ui);
        });
        if let Some((chu, t)) = &self.toast {
            if t.elapsed() < THOI_TOAST {
                widget::toast(&ctx, chu);
                ctx.request_repaint_after(Duration::from_millis(200));
            } else {
                self.toast = None;
            }
        }
        let cho_mo = self.kho_trang.is_empty() && !self.ds.is_empty() && self.loi.is_none();
        if !self.cho_anh.is_empty() || !self.cho_nho.is_empty() || cho_mo || self.nhan_thong_tin.is_some() || self.nhan_tt.is_some() || self.mo_hinh.as_ref().is_some_and(|m| !m.xong()) {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
        if std::mem::take(&mut self.cfg_doi) {
            *cfg = self.cfg.clone();
            return true;
        }
        false
    }

    /// Nút ^ / v trên thanh menu chung: thu gọn / mở thanh công cụ.
    pub fn nut_thu_gon_cong_cu(&mut self, ui: &mut egui::Ui) {
        let huong = if self.an_cong_cu { widget::Huong::Xuong } else { widget::Huong::Len };
        let r = widget::nut_mui_ten(ui, huong, 28.0, 24.0, "Thu gọn / mở thanh công cụ");
        if r.on_hover_text(if self.an_cong_cu { "Mở thanh công cụ đầy đủ" } else { "Thu gọn thành hàng biểu tượng (mini)" }).clicked() {
            self.an_cong_cu = !self.an_cong_cu;
        }
    }

    pub(crate) fn bao(&mut self, chu: impl Into<String>) {
        self.toast = Some((chu.into(), Instant::now()));
    }

    fn doi_cfg(&mut self, f: impl FnOnce(&mut CauHinhXoaGhim)) {
        f(&mut self.cfg);
        self.cfg_doi = true;
    }

    /// Kéo thả thư mục (mở) hoặc 1 PDF (mở đúng file) vào cửa sổ.
    fn tha_vao(&mut self, ctx: &egui::Context) {
        let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())) else { return };
        if p.is_dir() {
            self.mo_thu_muc(p);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
            self.mo_pdf(p);
        }
    }

    fn so_trang_xem(&self) -> u32 {
        self.kho_trang.len() as u32
    }

    fn sang_trang(&mut self, t: u32) {
        let n = self.so_trang_xem();
        if n > 0 {
            self.trang = t.min(n - 1);
            self.cuon_toi = Some(self.trang);
        }
    }

    /// − / +: ±10% (20–150%).
    fn phong(&mut self, buoc: f32) {
        let pt = ((self.ty_le_cuoi * 100.0).round() + buoc).clamp(20.0, 150.0);
        self.ty_le = TyLe::CoDinh(pt / 100.0);
        self.cuon_toi = Some(self.trang);
    }

    fn phim(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::Key;
        let (truoc, sau, len, xuong, xoa, hoan_tac, thoat, phong_to, thu_nho, vua, chon_het, f1, mo_file, mo_tm) = ctx.input(|i| {
            (
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::CloseBracket),
                i.key_pressed(Key::PageUp),
                i.key_pressed(Key::PageDown),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.modifiers.command && i.key_pressed(Key::Z),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Plus) || (i.key_pressed(Key::Equals) && i.modifiers.shift),
                i.key_pressed(Key::Minus) && !i.modifiers.command,
                i.key_pressed(Key::Equals) && !i.modifiers.shift && !i.modifiers.command,
                i.modifiers.command && i.key_pressed(Key::A),
                i.key_pressed(Key::F1),
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::O),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::O),
            )
        });
        if truoc && self.chon > 0 && self.mot_file.is_none() {
            self.chon_file(self.chon - 1);
        }
        if sau && self.mot_file.is_none() {
            self.chon_file(self.chon + 1);
        }
        if len && self.trang > 0 {
            self.sang_trang(self.trang - 1);
        }
        if xuong {
            self.sang_trang(self.trang + 1);
        }
        if xoa && !self.bo.chon.is_empty() {
            let ve = self.bo.chi_vung_ve(&self.bo.chon.clone());
            if ve.is_empty() {
                self.bao("Vùng cạnh / góc: bỏ bằng cách bấm lại trên biểu tượng");
            } else {
                let n = self.bo.xoa(&ve);
                self.bao(format!("Đã xoá {n} vùng · Ctrl+Z để hoàn tác"));
            }
        }
        if hoan_tac {
            self.bo.hoan_tac();
        }
        if thoat {
            if self.ve.is_some() {
                self.ve = None;
            } else {
                self.bo.chon.clear();
            }
        }
        if phong_to {
            self.phong(10.0);
        }
        if thu_nho {
            self.phong(-10.0);
        }
        if vua {
            self.ty_le = TyLe::VuaNgang;
            self.cuon_toi = Some(self.trang);
        }
        if chon_het && self.dang_mo() {
            self.bo.chon = self.bo.tat_ca();
        }
        if f1 {
            self.tro_giup = true;
        }
        if mo_file {
            self.chon_file_pdf();
        }
        if mo_tm {
            self.chon_thu_muc();
        }
    }

    /// Bấm 1 góc / cạnh trên biểu tượng: bật (phạm vi theo cột lọc trang) / tắt.
    fn bat_tat_co_san(&mut self, v: ViTri) {
        if self.bo.bat_tat(v) {
            let ap = match self.loc_trang {
                LocTrang::Le => ApTrang::Le,
                LocTrang::Chan => ApTrang::Chan,
                LocTrang::TungTrang | LocTrang::VoDoi => ApTrang::MotTrang(self.trang),
                LocTrang::TatCa => ApTrang::TatCa,
            };
            if let Some(i) = self.bo.tm.vung.iter().position(|z| z.vi_tri == v) {
                self.bo.tm.vung[i].ap_trang = ap;
                self.bo.chon = vec![(false, i)];
            }
        }
    }

    /// Thêm vùng vẽ tay (vùng riêng của file) theo chế độ vẽ + lọc trang.
    fn them_ve(&mut self, khung: (f64, f64, f64, f64), trang: u32) {
        let Some(loai) = self.ve else { return };
        let loai = if loai == LoaiVung::Xoa && self.loc_trang == LocTrang::VoDoi { LoaiVung::VoDoi } else { loai };
        let pham_vi = if matches!(self.loc_trang, LocTrang::TungTrang | LocTrang::VoDoi) { sua_vung::PhamVi::Trang } else { sua_vung::PhamVi::File };
        if !self.bo.them_ve(khung, loai, pham_vi, self.loc_trang.ap(trang), trang) && self.bo.id_file.is_none() {
            self.loi = Some("Không đọc được PDF này — không vẽ được vùng riêng".into());
        }
    }

    /// Tên vùng như prototype: "Góc trên trái", "Xóa 2", "Bảo vệ 1", "Vô đối 1".
    fn ten_vung(&self, c: ChonVung) -> String {
        let Some(z) = self.bo.lay(c) else { return String::new() };
        if z.vi_tri != ViTri::Tay {
            return z.vi_tri.ten().to_string();
        }
        let cung: Vec<ChonVung> = self.bo.tat_ca().into_iter().filter(|x| self.bo.lay(*x).is_some_and(|v| v.vi_tri == ViTri::Tay && v.loai == z.loai)).collect();
        let so = cung.iter().position(|x| *x == c).map_or(0, |i| i + 1);
        let ten = match z.loai {
            LoaiVung::Xoa => "Xóa",
            LoaiVung::BaoVe => "Bảo vệ",
            LoaiVung::VoDoi => "Vô đối",
        };
        format!("{ten} {so}")
    }

    /// Dòng Làm tiếp gần đây khi rời màn.
    pub fn muc_gan_day(&self) -> Option<crate::gan_day::MucGanDay> {
        use crate::gan_day::{ChucNang, MucGanDay, TinhTrang};
        let tm = self.thu_muc.as_ref()?;
        let tinh = match self.so_da_xoa {
            Some((ok, tong)) => TinhTrang { chu: format!("Đã xoá ghim {ok}/{tong} file"), xong: ok as u32, tong: tong as u32, loi: ok < tong },
            None => TinhTrang { chu: format!("{} file", self.ds.len()), ..Default::default() },
        };
        Some(MucGanDay::moi(ChucNang::XoaGhim, tm.display().to_string(), tinh))
    }
}

/// Tên phạm vi trang của vùng (combo Vùng, menu chuột phải, hộp xác nhận).
fn ten_pham_vi(ap: ApTrang) -> String {
    match ap {
        ApTrang::TatCa => "mọi trang".into(),
        ApTrang::Le => "trang lẻ".into(),
        ApTrang::Chan => "trang chẵn".into(),
        ApTrang::MotTrang(t) => format!("trang {}", t + 1),
    }
}

/// Số trang + cỡ từng trang của 1 PDF (lỗi → 0 trang).
fn doc_thong_tin_file(p: &Path) -> ThongTinFile {
    let Ok(d) = mupdf::Document::open(p.to_string_lossy().as_ref()) else { return ThongTinFile::default() };
    let n = d.page_count().unwrap_or(0).max(0);
    let trang = (0..n)
        .map(|i| {
            d.load_page(i)
                .and_then(|pg| pg.bounds())
                // Cùng cách đo với màn Đếm: điểm PDF, làm tròn 3 số lẻ
                .map(|b| (((b.x1 - b.x0) as f64 * 1000.0).round() / 1000.0, ((b.y1 - b.y0) as f64 * 1000.0).round() / 1000.0))
                .unwrap_or((595.276, 841.89))
        })
        .collect();
    ThongTinFile { so_trang: n as u32, trang }
}

/// "1-3, 7, >10, <=5" → bộ kiểm số (`None` = chuỗi trống / không hiểu).
pub fn khop_so(txt: &str) -> Option<Box<dyn Fn(u32) -> bool>> {
    let mut ds: Vec<Box<dyn Fn(u32) -> bool>> = Vec::new();
    for p in txt.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let so = |s: &str| s.trim().parse::<u32>().ok();
        if let Some((a, b)) = p.split_once('-') {
            if let (Some(a), Some(b)) = (so(a), so(b)) {
                ds.push(Box::new(move |n| n >= a && n <= b));
            }
        } else if let Some(r) = p.strip_prefix(">=") {
            if let Some(a) = so(r) {
                ds.push(Box::new(move |n| n >= a));
            }
        } else if let Some(r) = p.strip_prefix("<=") {
            if let Some(a) = so(r) {
                ds.push(Box::new(move |n| n <= a));
            }
        } else if let Some(r) = p.strip_prefix('>') {
            if let Some(a) = so(r) {
                ds.push(Box::new(move |n| n > a));
            }
        } else if let Some(r) = p.strip_prefix('<') {
            if let Some(a) = so(r) {
                ds.push(Box::new(move |n| n < a));
            }
        } else if let Some(a) = so(p) {
            ds.push(Box::new(move |n| n == a));
        }
    }
    if ds.is_empty() {
        None
    } else {
        Some(Box::new(move |n| ds.iter().any(|f| f(n))))
    }
}

/// File có ≥ 1 trang (trong dải) khớp khổ + chiều đã chọn; khổ theo quy tắc Đếm trang A4 `qt`.
pub fn khop_nang_cao(tt: &ThongTinFile, l: &LocNangCao, qt: &crate::dem_pdf::QuyTac) -> bool {
    let trong = khop_so(&l.dai);
    tt.kho_chieu(qt).enumerate().any(|(i, (kho, ngang))| trong.as_ref().is_none_or(|f| f(i as u32 + 1)) && l.kho.get(kho).copied().unwrap_or(false) && if ngang { l.ngang } else { l.doc })
}

/// Khổ (A0..A7) và chiều (ngang, dọc) **có mặt** trong các file đã đọc — ô lọc không có trang nào thì mờ.
pub fn co_mat(ds: &[Option<ThongTinFile>], qt: &crate::dem_pdf::QuyTac) -> ([bool; 8], bool, bool) {
    let (mut kho, mut ngang, mut doc) = ([false; 8], false, false);
    for (k, n) in ds.iter().flatten().flat_map(|t| t.kho_chieu(qt)) {
        kho[k] = true;
        if n {
            ngang = true;
        } else {
            doc = true;
        }
    }
    (kho, ngang, doc)
}

#[cfg(test)]
mod tests;
