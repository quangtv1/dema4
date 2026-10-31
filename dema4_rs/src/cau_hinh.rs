//! Cấu hình người dùng `.dema4_saomai.cfg` (file ẩn; dạng `khoá=giá trị`, mỗi dòng 1 khoá): hệ số quy A4, ranh giới khổ,
//! tuỳ chọn Tách PDF và trạng thái giao diện.
//!
//! Đặt cạnh file chạy; thư mục đó không ghi được → thư mục người dùng (`%APPDATA%\sm-dema4\`,
//! macOS `~/Library/Application Support/sm-dema4/`). Khoá thiếu / sai → giá trị mặc định, không báo lỗi.

use crate::boc_tach::mo_hinh::NhaCungCap;
use crate::dem_pdf::HE_SO_MAC_DINH;
use crate::tach_vb::tach::MAU_TEN_MAC_DINH;
use std::path::{Path, PathBuf};

/// Cùng tên ứng dụng, ẩn: dấu chấm đầu (macOS / Linux) + thuộc tính Hidden khi ghi (Windows).
pub const TEN_FILE: &str = ".dema4_saomai.cfg";
/// Tên cũ (≤ v1.0.0 bản đầu): vẫn đọc nếu chưa có file mới; lần ghi sau chuyển sang tên mới và xoá file cũ.
const TEN_FILE_CU: &str = "sm-dema4.cfg";
/// Ranh giới khổ mặc định (mm, cạnh dài): điểm giữa 2 khổ liền kề A0|A1 … A6|A7.
pub const RANH_GIOI_MAC_DINH: [f64; 7] = [1015.0, 717.5, 507.0, 358.5, 253.5, 179.0, 126.5];
/// Cạnh dài chuẩn (mm) A0 … A7.
pub const CANH_CHUAN_MM: [f64; 8] = [1189.0, 841.0, 594.0, 420.0, 297.0, 210.0, 148.0, 105.0];
/// Trang A0 dài hơn mức này (mm) quy A4 theo cách chọn ở `a0_lon`.
pub const A0_NGUONG_MAC_DINH: f64 = 1399.0;
pub const A0_NGUONG_KHOANG: (f64, f64) = (1189.0, 2500.0);
pub const TIN_CAY_KHOANG: (f32, f32) = (0.50, 0.95);
/// Số nấc cỡ ảnh lưới (xem `gui_tach::NAC_ANH`).
pub const SO_NAC_ANH: u8 = 12;
/// Nấc mặc định: 110 px = 26 %.
pub const NAC_ANH_MAC_DINH: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A0Lon {
    /// Diện tích thật ÷ A4, không thấp hơn hệ số A0 (cách tính từ trước tới nay).
    DienTich,
    /// Hệ số cố định `a0_lon_he_so`.
    CoDinh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaiMa {
    Code128,
    /// Giữ chỗ: chưa hỗ trợ đọc.
    Qr,
    /// Giữ chỗ: chưa hỗ trợ đọc.
    Chu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuuNhan {
    TuDong,
    BangTay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheDoVb {
    Luoi,
    MotTrang,
    HaiTrang,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CauHinh {
    pub he_so: [f64; 8],
    pub ranh_gioi: [f64; 7],
    pub a0_lon: A0Lon,
    pub a0_lon_he_so: f64,
    pub a0_nguong_mm: f64,
    pub quet_trang: bool,
    pub quet_ma: bool,
    pub quet_xoay: bool,
    /// Phần trăm (1.0 = 1 %).
    pub nguong_trang: f32,
    pub nguong_nghi: f32,
    pub loai_ma: LoaiMa,
    pub ma_index: String,
    pub tin_cay_xoay: f32,
    /// Thư mục cha chứa `<nguồn>_vanban`; trống = cạnh thư mục nguồn.
    pub thu_muc_ra: String,
    pub mau_ten: String,
    pub luu_nhan: LuuNhan,
    pub rong_cot_trai: f32,
    /// Nấc cỡ ảnh 0..SO_NAC_ANH.
    pub co_anh_hs: u8,
    pub co_anh_vb: u8,
    /// Tỷ lệ % chế độ 1/2 trang tab Văn bản.
    pub ty_le_vb: u16,
    pub che_do_vb: CheDoVb,
    pub an_dong_thu_muc: bool,
    pub muc_luc: CauHinhMucLuc,
    pub boc_tach: CauHinhBocTach,
}

/// Cấu hình + lựa chọn gần nhất của màn Sinh mục lục (khoá `ml_*` trong file).
#[derive(Debug, Clone, PartialEq)]
pub struct CauHinhMucLuc {
    /// Thư mục ra; trống = `.sm-dema4/muc_luc/<tên file dữ liệu>` cạnh file chạy.
    pub thu_muc_ra: String,
    pub mau_ten: String,
    /// File đã có: ghi đè (`true`) hay bỏ qua.
    pub ghi_de: bool,
    pub rong_trai: f32,
    pub file: String,
    pub sheet: String,
    /// Số dòng vật lý (1-based) của hàng tiêu đề.
    pub dong_tieu_de: u32,
    /// "1".."4" = mẫu nhúng sẵn; còn lại = đường dẫn DOCX riêng.
    pub mau: String,
    /// Ràng buộc biến gần nhất (JSON một dòng, xem `RangBuoc::json`) của mẫu `mau`.
    pub bien: String,
}

pub const RONG_TRAI_ML: (f32, f32) = (220.0, 600.0);

/// Thông số 1 nhà cung cấp mô hình đọc (khoá `bt_<mã>_*`). Khoá API không ở đây: lưu mã hoá trong `boc_tach::khoa_cache`.
#[derive(Debug, Clone, PartialEq)]
pub struct MoHinhBocTach {
    /// Trống = địa chỉ mặc định của dịch vụ đám mây (Nội bộ bắt buộc nhập).
    pub dia_chi: String,
    pub model: String,
    pub song_song: u32,
    pub cho_s: u32,
}

impl MoHinhBocTach {
    pub fn mac_dinh(n: NhaCungCap) -> Self {
        Self { dia_chi: String::new(), model: String::new(), song_song: n.song_song_mac_dinh() as u32, cho_s: n.cho_mac_dinh_s() as u32 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachGhiSheet {
    ServiceAccount,
    AppsScript,
}

pub const SONG_SONG_KHOANG: (u32, u32) = (1, 16);
pub const CHO_KHOANG: (u32, u32) = (10, 600);
pub const DPI_KHOANG: (u32, u32) = (72, 400);
pub const CANH_KHOANG: (u32, u32) = (512, 4096);
pub const JPEG_KHOANG: (u32, u32) = (30, 100);
pub const TOKEN_KHOANG: (u32, u32) = (256, 16384);

/// Cấu hình + lựa chọn gần nhất của màn Bóc tách (khoá `bt_*` trong file).
#[derive(Debug, Clone, PartialEq)]
pub struct CauHinhBocTach {
    /// Theo thứ tự `NhaCungCap::TAT_CA`.
    pub mo_hinh: [MoHinhBocTach; 5],
    /// ★ Mặc định: chọn sẵn và đứng đầu danh sách Mô hình đọc ở màn Bóc tách.
    pub mac_dinh: Option<NhaCungCap>,
    pub dpi: u32,
    pub canh_toi_da: u32,
    pub jpeg: u32,
    pub max_tokens: u32,
    /// Thư mục ghi Excel; trống = cạnh thư mục nguồn.
    pub thu_muc_excel: String,
    pub sheet_link: String,
    pub sheet_tab: String,
    pub sheet_cach: CachGhiSheet,
    /// File khoá JSON service account (đường dẫn; nội dung không chép vào cấu hình).
    pub sheet_file_khoa: String,
    /// Ô "Google Sheet" ở dòng Ghi ra (Excel luôn tạo).
    pub ghi_sheet: bool,
}

impl Default for CauHinhBocTach {
    fn default() -> Self {
        Self {
            mo_hinh: NhaCungCap::TAT_CA.map(MoHinhBocTach::mac_dinh),
            mac_dinh: None,
            dpi: 200,
            canh_toi_da: 2048,
            jpeg: 90,
            max_tokens: 2048,
            thu_muc_excel: String::new(),
            sheet_link: String::new(),
            sheet_tab: "Boc_tach".into(),
            sheet_cach: CachGhiSheet::ServiceAccount,
            sheet_file_khoa: String::new(),
            ghi_sheet: false,
        }
    }
}

impl CauHinhBocTach {
    pub fn mo_hinh(&self, n: NhaCungCap) -> &MoHinhBocTach {
        &self.mo_hinh[n.chi_so()]
    }

    pub fn mo_hinh_mut(&mut self, n: NhaCungCap) -> &mut MoHinhBocTach {
        &mut self.mo_hinh[n.chi_so()]
    }

    /// Đủ thông số để gọi: có model; Nội bộ có địa chỉ; dịch vụ đám mây có khoá.
    pub fn da_cai(&self, n: NhaCungCap, khoa: &str) -> bool {
        let m = self.mo_hinh(n);
        !m.model.trim().is_empty() && if n.la_dam_may() { !khoa.trim().is_empty() } else { !m.dia_chi.trim().is_empty() }
    }

    pub fn anh(&self) -> crate::boc_tach::anh::CauHinhAnh {
        crate::boc_tach::anh::CauHinhAnh { dpi: self.dpi as f32, canh_toi_da: self.canh_toi_da, chat_luong: self.jpeg as u8 }
    }

    pub fn ket_noi(&self, n: NhaCungCap, khoa: &str) -> crate::boc_tach::mo_hinh::KetNoi {
        let m = self.mo_hinh(n);
        let mut k = crate::boc_tach::mo_hinh::KetNoi::moi(n, &m.dia_chi, khoa, &m.model);
        k.cho_toi_da = std::time::Duration::from_secs(m.cho_s as u64);
        k.max_tokens = self.max_tokens;
        k
    }
}

impl Default for CauHinhMucLuc {
    fn default() -> Self {
        Self {
            thu_muc_ra: String::new(),
            mau_ten: crate::muc_luc::ten_file::MAU_MAC_DINH.into(),
            ghi_de: true,
            rong_trai: 290.0,
            file: String::new(),
            sheet: String::new(),
            dong_tieu_de: 1,
            // Trống = chưa chọn mẫu (người dùng tự chọn)
            mau: String::new(),
            bien: String::new(),
        }
    }
}

impl Default for CauHinh {
    fn default() -> Self {
        Self {
            he_so: HE_SO_MAC_DINH,
            ranh_gioi: RANH_GIOI_MAC_DINH,
            a0_lon: A0Lon::DienTich,
            a0_lon_he_so: HE_SO_MAC_DINH[0],
            a0_nguong_mm: A0_NGUONG_MAC_DINH,
            quet_trang: true,
            quet_ma: true,
            quet_xoay: true,
            nguong_trang: 1.0,
            nguong_nghi: 3.2,
            loai_ma: LoaiMa::Code128,
            ma_index: "NAMSAO-TACH".into(),
            tin_cay_xoay: 0.8,
            thu_muc_ra: String::new(),
            mau_ten: MAU_TEN_MAC_DINH.into(),
            luu_nhan: LuuNhan::TuDong,
            rong_cot_trai: 290.0,
            co_anh_hs: NAC_ANH_MAC_DINH,
            co_anh_vb: NAC_ANH_MAC_DINH,
            ty_le_vb: 100,
            che_do_vb: CheDoVb::Luoi,
            an_dong_thu_muc: false,
            muc_luc: CauHinhMucLuc::default(),
            boc_tach: CauHinhBocTach::default(),
        }
    }
}

fn doc_boc_tach(b: &mut CauHinhBocTach, k: &str, v: &str) {
    let so = |(tu, den): (u32, u32)| v.parse::<u32>().ok().filter(|x| (tu..=den).contains(x));
    let bool_ = || match v {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    };
    match k {
        "mac_dinh" => b.mac_dinh = NhaCungCap::tu_ma(v),
        "dpi" => b.dpi = so(DPI_KHOANG).unwrap_or(b.dpi),
        "canh_toi_da" => b.canh_toi_da = so(CANH_KHOANG).unwrap_or(b.canh_toi_da),
        "jpeg" => b.jpeg = so(JPEG_KHOANG).unwrap_or(b.jpeg),
        "max_tokens" => b.max_tokens = so(TOKEN_KHOANG).unwrap_or(b.max_tokens),
        "thu_muc_excel" => b.thu_muc_excel = v.into(),
        "sheet_link" => b.sheet_link = v.into(),
        "sheet_tab" => b.sheet_tab = v.into(),
        "sheet_cach" => b.sheet_cach = if v == "apps_script" { CachGhiSheet::AppsScript } else { CachGhiSheet::ServiceAccount },
        "sheet_file_khoa" => b.sheet_file_khoa = v.into(),
        "ghi_sheet" => b.ghi_sheet = bool_().unwrap_or(b.ghi_sheet),
        _ => {
            // bt_<mã nhà cung cấp>_<thông số>
            let Some(n) = NhaCungCap::TAT_CA.into_iter().find(|n| k.starts_with(n.ma()) && k[n.ma().len()..].starts_with('_')) else { return };
            let m = b.mo_hinh_mut(n);
            match &k[n.ma().len() + 1..] {
                "dia_chi" => m.dia_chi = v.into(),
                "model" => m.model = v.into(),
                "song_song" => m.song_song = so(SONG_SONG_KHOANG).unwrap_or(m.song_song),
                "cho" => m.cho_s = so(CHO_KHOANG).unwrap_or(m.cho_s),
                _ => {}
            }
        }
    }
}

fn ghi_boc_tach(b: &CauHinhBocTach) -> String {
    let bool_ = |x: bool| if x { "1" } else { "0" };
    let ma = |n: Option<NhaCungCap>| n.map_or("", NhaCungCap::ma);
    let mut s = format!("bt_mac_dinh={}\nbt_dpi={}\nbt_canh_toi_da={}\nbt_jpeg={}\nbt_max_tokens={}\n", ma(b.mac_dinh), b.dpi, b.canh_toi_da, b.jpeg, b.max_tokens);
    for n in NhaCungCap::TAT_CA {
        let m = b.mo_hinh(n);
        s += &format!("bt_{0}_dia_chi={1}\nbt_{0}_model={2}\nbt_{0}_song_song={3}\nbt_{0}_cho={4}\n", n.ma(), m.dia_chi, m.model, m.song_song, m.cho_s);
    }
    let cach = if b.sheet_cach == CachGhiSheet::AppsScript { "apps_script" } else { "service_account" };
    s += &format!("bt_thu_muc_excel={}\nbt_sheet_link={}\nbt_sheet_tab={}\nbt_sheet_cach={cach}\nbt_sheet_file_khoa={}\n", b.thu_muc_excel, b.sheet_link, b.sheet_tab, b.sheet_file_khoa);
    s += &format!("bt_ghi_sheet={}\n", bool_(b.ghi_sheet));
    s
}

/// Giá trị không hợp lệ: hộp Cấu hình hiện đỏ và khoá Lưu; lúc đọc file thì nhóm đó về mặc định.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoiCauHinh {
    HeSo(usize),
    RanhGioi,
    A0LonHeSo,
    A0Nguong,
    Nguong,
    TinCay,
    MauTen,
    MauTenMucLuc,
}

impl CauHinh {
    pub fn kiem_tra(&self) -> Vec<LoiCauHinh> {
        let mut loi = Vec::new();
        for (i, h) in self.he_so.iter().enumerate() {
            if !(h.is_finite() && *h > 0.0) {
                loi.push(LoiCauHinh::HeSo(i));
            }
        }
        // Mỗi ranh giới nằm giữa 2 cạnh chuẩn kề bên (nên tự tăng dần)
        if self.ranh_gioi.iter().enumerate().any(|(i, r)| !(*r < CANH_CHUAN_MM[i] && *r > CANH_CHUAN_MM[i + 1])) {
            loi.push(LoiCauHinh::RanhGioi);
        }
        if !(self.a0_lon_he_so.is_finite() && self.a0_lon_he_so >= self.he_so[0]) {
            loi.push(LoiCauHinh::A0LonHeSo);
        }
        if !(A0_NGUONG_KHOANG.0..=A0_NGUONG_KHOANG.1).contains(&self.a0_nguong_mm) {
            loi.push(LoiCauHinh::A0Nguong);
        }
        if !(0.0 <= self.nguong_trang && self.nguong_trang <= self.nguong_nghi && self.nguong_nghi <= 10.0) {
            loi.push(LoiCauHinh::Nguong);
        }
        // Sai số dấu phẩy động khi gõ 0,95
        if !(TIN_CAY_KHOANG.0 - 1e-6..=TIN_CAY_KHOANG.1 + 1e-6).contains(&self.tin_cay_xoay) {
            loi.push(LoiCauHinh::TinCay);
        }
        if crate::tach_vb::tach::kiem_mau_ten(&self.mau_ten).is_err() {
            loi.push(LoiCauHinh::MauTen);
        }
        if self.muc_luc.mau_ten.trim().is_empty() {
            loi.push(LoiCauHinh::MauTenMucLuc);
        }
        loi
    }

    /// Đọc từ nội dung file; khoá lạ bỏ qua, giá trị sai hoặc nhóm không hợp lệ → mặc định.
    pub fn phan_tich(noi_dung: &str) -> Self {
        let md = Self::default();
        let mut c = md.clone();
        for dong in noi_dung.lines().map(str::trim).filter(|d| !d.is_empty() && !d.starts_with('#')) {
            let Some((k, v)) = dong.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let so = || v.replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite());
            let bool_ = || match v {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            };
            match k {
                _ if k.starts_with("he_so_a") => {
                    if let (Some(i), Some(x)) = (k["he_so_a".len()..].parse::<usize>().ok().filter(|i| *i < 8), so()) {
                        c.he_so[i] = x;
                    }
                }
                "ranh_gioi" => {
                    let ds: Vec<f64> = v.split(';').filter_map(|s| s.trim().replace(',', ".").parse().ok()).collect();
                    if let Ok(mang) = <[f64; 7]>::try_from(ds) {
                        c.ranh_gioi = mang;
                    }
                }
                "a0_lon" => match v {
                    "dien_tich" => c.a0_lon = A0Lon::DienTich,
                    "co_dinh" => c.a0_lon = A0Lon::CoDinh,
                    _ => {}
                },
                "a0_lon_he_so" => c.a0_lon_he_so = so().unwrap_or(c.a0_lon_he_so),
                "a0_nguong_mm" => c.a0_nguong_mm = so().unwrap_or(c.a0_nguong_mm),
                "quet_trang" => c.quet_trang = bool_().unwrap_or(c.quet_trang),
                "quet_ma" => c.quet_ma = bool_().unwrap_or(c.quet_ma),
                "quet_xoay" => c.quet_xoay = bool_().unwrap_or(c.quet_xoay),
                "nguong_trang" => c.nguong_trang = so().map_or(c.nguong_trang, |x| x as f32),
                "nguong_nghi" => c.nguong_nghi = so().map_or(c.nguong_nghi, |x| x as f32),
                "loai_ma" => match v {
                    "code128" => c.loai_ma = LoaiMa::Code128,
                    "qr" => c.loai_ma = LoaiMa::Qr,
                    "chu" => c.loai_ma = LoaiMa::Chu,
                    _ => {}
                },
                "ma_index" => c.ma_index = v.to_string(),
                "tin_cay_xoay" => c.tin_cay_xoay = so().map_or(c.tin_cay_xoay, |x| x as f32),
                "thu_muc_ra" => c.thu_muc_ra = v.to_string(),
                "mau_ten" => c.mau_ten = if v == "{hoso}_part{n}" { MAU_TEN_MAC_DINH.to_string() } else { v.to_string() },
                "luu_nhan" => match v {
                    "tu_dong" => c.luu_nhan = LuuNhan::TuDong,
                    "bang_tay" => c.luu_nhan = LuuNhan::BangTay,
                    _ => {}
                },
                "rong_cot_trai" => c.rong_cot_trai = so().map_or(c.rong_cot_trai, |x| (x as f32).clamp(160.0, 520.0)),
                "nac_anh_hs" => c.co_anh_hs = so().map_or(c.co_anh_hs, |x| (x as u8).min(SO_NAC_ANH - 1)),
                "nac_anh_vb" => c.co_anh_vb = so().map_or(c.co_anh_vb, |x| (x as u8).min(SO_NAC_ANH - 1)),
                "ty_le_vb" => c.ty_le_vb = so().filter(|x| (10.0..=400.0).contains(x)).map_or(c.ty_le_vb, |x| x as u16),
                "che_do_vb" => match v {
                    "luoi" => c.che_do_vb = CheDoVb::Luoi,
                    "mot_trang" => c.che_do_vb = CheDoVb::MotTrang,
                    "hai_trang" => c.che_do_vb = CheDoVb::HaiTrang,
                    _ => {}
                },
                "an_dong_thu_muc" => c.an_dong_thu_muc = bool_().unwrap_or(c.an_dong_thu_muc),
                "ml_thu_muc_ra" => c.muc_luc.thu_muc_ra = v.to_string(),
                "ml_mau_ten" => c.muc_luc.mau_ten = v.to_string(),
                "ml_ghi_de" => c.muc_luc.ghi_de = bool_().unwrap_or(c.muc_luc.ghi_de),
                "ml_rong_trai" => c.muc_luc.rong_trai = so().map_or(c.muc_luc.rong_trai, |x| (x as f32).clamp(RONG_TRAI_ML.0, RONG_TRAI_ML.1)),
                "ml_file" => c.muc_luc.file = v.to_string(),
                "ml_sheet" => c.muc_luc.sheet = v.to_string(),
                "ml_dong_tieu_de" => c.muc_luc.dong_tieu_de = so().filter(|x| (1.0..=100_000.0).contains(x)).map_or(c.muc_luc.dong_tieu_de, |x| x as u32),
                "ml_mau" => c.muc_luc.mau = v.to_string(),
                "ml_bien" => c.muc_luc.bien = v.to_string(),
                _ if k.starts_with("bt_") => doc_boc_tach(&mut c.boc_tach, &k[3..], v),
                _ => {}
            }
        }
        for loi in c.kiem_tra() {
            match loi {
                LoiCauHinh::HeSo(i) => c.he_so[i] = md.he_so[i],
                LoiCauHinh::RanhGioi => c.ranh_gioi = md.ranh_gioi,
                LoiCauHinh::A0Nguong => c.a0_nguong_mm = md.a0_nguong_mm,
                LoiCauHinh::Nguong => (c.nguong_trang, c.nguong_nghi) = (md.nguong_trang, md.nguong_nghi),
                LoiCauHinh::TinCay => c.tin_cay_xoay = md.tin_cay_xoay,
                LoiCauHinh::MauTen => c.mau_ten = md.mau_ten.clone(),
                LoiCauHinh::MauTenMucLuc => c.muc_luc.mau_ten = md.muc_luc.mau_ten.clone(),
                LoiCauHinh::A0LonHeSo => {}
            }
        }
        // Sửa hệ số A0 có thể làm hệ số cố định thấp hơn → kiểm lại sau cùng
        if c.kiem_tra().contains(&LoiCauHinh::A0LonHeSo) {
            c.a0_lon_he_so = c.he_so[0];
        }
        c
    }

    pub fn noi_dung(&self) -> String {
        let bool_ = |b: bool| if b { "1" } else { "0" };
        let mut s = String::from("# Cấu hình dema4-saomai (sửa trong app: menu Cấu hình)\n");
        for (i, h) in self.he_so.iter().enumerate() {
            s += &format!("he_so_a{i}={h}\n");
        }
        s += &format!("ranh_gioi={}\n", self.ranh_gioi.iter().map(f64::to_string).collect::<Vec<_>>().join(";"));
        s += &format!("a0_lon={}\n", if self.a0_lon == A0Lon::DienTich { "dien_tich" } else { "co_dinh" });
        s += &format!("a0_lon_he_so={}\na0_nguong_mm={}\n", self.a0_lon_he_so, self.a0_nguong_mm);
        s += &format!("quet_trang={}\nquet_ma={}\nquet_xoay={}\n", bool_(self.quet_trang), bool_(self.quet_ma), bool_(self.quet_xoay));
        s += &format!("nguong_trang={:.2}\nnguong_nghi={:.2}\n", self.nguong_trang, self.nguong_nghi);
        s += &format!(
            "loai_ma={}\n",
            match self.loai_ma {
                LoaiMa::Code128 => "code128",
                LoaiMa::Qr => "qr",
                LoaiMa::Chu => "chu",
            }
        );
        s += &format!("ma_index={}\ntin_cay_xoay={:.2}\n", self.ma_index, self.tin_cay_xoay);
        s += &format!("thu_muc_ra={}\nmau_ten={}\n", self.thu_muc_ra, self.mau_ten);
        s += &format!("luu_nhan={}\n", if self.luu_nhan == LuuNhan::TuDong { "tu_dong" } else { "bang_tay" });
        s += &format!("rong_cot_trai={}\nnac_anh_hs={}\nnac_anh_vb={}\nty_le_vb={}\n", self.rong_cot_trai, self.co_anh_hs, self.co_anh_vb, self.ty_le_vb);
        s += &format!(
            "che_do_vb={}\n",
            match self.che_do_vb {
                CheDoVb::Luoi => "luoi",
                CheDoVb::MotTrang => "mot_trang",
                CheDoVb::HaiTrang => "hai_trang",
            }
        );
        s += &format!("an_dong_thu_muc={}\n", bool_(self.an_dong_thu_muc));
        let m = &self.muc_luc;
        s += &format!("ml_thu_muc_ra={}\nml_mau_ten={}\nml_ghi_de={}\nml_rong_trai={}\n", m.thu_muc_ra, m.mau_ten, bool_(m.ghi_de), m.rong_trai);
        s += &format!("ml_file={}\nml_sheet={}\nml_dong_tieu_de={}\nml_mau={}\nml_bien={}\n", m.file, m.sheet, m.dong_tieu_de, m.mau, m.bien);
        s += &ghi_boc_tach(&self.boc_tach);
        s
    }

    /// Quy tắc xếp khổ / quy A4 cho màn Đếm và CLI đếm. Giá trị mặc định → đúng công thức cũ (kết quả giống hệt).
    pub fn quy_tac(&self) -> crate::dem_pdf::QuyTac {
        use crate::dem_pdf::{A0Lon as A0, QuyTac, NGUONG_A0_THEO_DIEN_TICH, PT_MOI_MM};
        QuyTac {
            he_so: self.he_so,
            ranh_gioi_mm: (self.ranh_gioi != RANH_GIOI_MAC_DINH).then_some(self.ranh_gioi),
            a0_nguong_pt: if self.a0_nguong_mm == A0_NGUONG_MAC_DINH { NGUONG_A0_THEO_DIEN_TICH } else { self.a0_nguong_mm * PT_MOI_MM },
            a0_lon: match self.a0_lon {
                A0Lon::DienTich => A0::DienTich,
                A0Lon::CoDinh => A0::CoDinh(self.a0_lon_he_so),
            },
        }
    }

    /// Cấu hình phân tích trang (ngưỡng % → tỷ lệ). Loại mã chưa hỗ trợ → không đọc mã.
    pub fn cau_hinh_phan_tich(&self) -> crate::tach_vb::phan_tich::CauHinhPhanTich {
        crate::tach_vb::phan_tich::CauHinhPhanTich {
            quet_trang: self.quet_trang,
            quet_ma: self.quet_ma && self.loai_ma == LoaiMa::Code128,
            quet_xoay: self.quet_xoay,
            ma_index: self.ma_index.clone(),
            nguong_trang: self.nguong_trang / 100.0,
            nguong_nghi: self.nguong_nghi / 100.0,
            tin_cay_xoay: self.tin_cay_xoay,
        }
    }
}

/// Thư mục đặt file theo thứ tự ưu tiên: cạnh file chạy, rồi thư mục người dùng.
fn thu_muc_dat_file() -> Vec<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)).into_iter().chain(thu_muc_nguoi_dung()).collect()
}

/// Chỗ đặt file (tên mới) theo thứ tự ưu tiên.
pub fn cho_dat_file() -> Vec<PathBuf> {
    thu_muc_dat_file().into_iter().map(|d| d.join(TEN_FILE)).collect()
}

/// Chỗ đọc: tên mới trước, rồi tên cũ.
fn cho_doc(thu_muc: &[PathBuf]) -> Vec<PathBuf> {
    thu_muc.iter().map(|d| d.join(TEN_FILE)).chain(thu_muc.iter().map(|d| d.join(TEN_FILE_CU))).collect()
}

pub(crate) fn thu_muc_nguoi_dung() -> Option<PathBuf> {
    let goc = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".config")))?
    };
    Some(goc.join("sm-dema4"))
}

/// Đọc file đầu tiên có trong danh sách; không có file nào → mặc định.
pub fn doc_tu(cho: &[PathBuf]) -> CauHinh {
    cho.iter().find_map(|p| std::fs::read_to_string(p).ok()).map(|s| CauHinh::phan_tich(&s)).unwrap_or_default()
}

pub fn doc() -> CauHinh {
    doc_tu(&cho_doc(&thu_muc_dat_file()))
}

fn ghi_file(p: &Path, noi_dung: &str) -> std::io::Result<()> {
    if let Some(cha) = p.parent() {
        std::fs::create_dir_all(cha)?;
    }
    let tam = p.with_extension("cfg.tmp");
    std::fs::write(&tam, noi_dung)?;
    an_hien(p, false); // Windows không cho thay file đang ẩn → bỏ ẩn bản cũ trước khi đổi tên đè
    std::fs::rename(&tam, p).inspect_err(|_| {
        let _ = std::fs::remove_file(&tam);
    })?;
    an_hien(p, true);
    Ok(())
}

/// Bật / tắt thuộc tính Hidden (chỉ Windows; macOS / Linux ẩn nhờ dấu chấm đầu tên). Lỗi bỏ qua.
#[cfg(windows)]
fn an_hien(p: &Path, an: bool) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, INVALID_FILE_ATTRIBUTES};
    let ten: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        let tt = GetFileAttributesW(ten.as_ptr());
        if tt != INVALID_FILE_ATTRIBUTES {
            SetFileAttributesW(ten.as_ptr(), if an { tt | FILE_ATTRIBUTE_HIDDEN } else { tt & !FILE_ATTRIBUTE_HIDDEN });
        }
    }
}

#[cfg(not(windows))]
fn an_hien(_p: &Path, _an: bool) {}

/// Ghi vào chỗ đầu tiên ghi được; trả đường dẫn đã ghi. Ghi được chỗ sau thì xoá bản cũ ở chỗ trước
/// (nếu xoá được) để lần đọc sau không lấy nhầm bản cũ.
pub fn ghi_vao(c: &CauHinh, cho: &[PathBuf]) -> std::io::Result<PathBuf> {
    let noi_dung = c.noi_dung();
    let mut loi = std::io::Error::other("không có chỗ ghi cấu hình");
    for (i, p) in cho.iter().enumerate() {
        match ghi_file(p, &noi_dung) {
            Ok(()) => {
                for cu in &cho[..i] {
                    let _ = std::fs::remove_file(cu);
                }
                return Ok(p.clone());
            }
            Err(e) => loi = e,
        }
    }
    Err(loi)
}

pub fn ghi(c: &CauHinh) -> std::io::Result<PathBuf> {
    ghi_trong(c, &thu_muc_dat_file())
}

/// Ghi tên mới; ghi được thì xoá file tên cũ ở mọi chỗ (lần đọc sau không lấy nhầm bản cũ).
fn ghi_trong(c: &CauHinh, thu_muc: &[PathBuf]) -> std::io::Result<PathBuf> {
    let p = ghi_vao(c, &thu_muc.iter().map(|d| d.join(TEN_FILE)).collect::<Vec<_>>())?;
    for d in thu_muc {
        let _ = std::fs::remove_file(d.join(TEN_FILE_CU));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_cfg_{}_{ten}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn mau_ten_mac_dinh_cu_doi_sang_moi() {
        assert_eq!(CauHinh::phan_tich("mau_ten={hoso}_part{n}\n").mau_ten, MAU_TEN_MAC_DINH);
        assert_eq!(CauHinh::phan_tich("mau_ten={hoso}-{n3}\n").mau_ten, "{hoso}-{n3}", "mẫu người dùng tự đặt giữ nguyên");
    }

    #[test]
    fn mac_dinh_hop_le_va_khu_hoi_moi_khoa() {
        assert!(CauHinh::default().kiem_tra().is_empty());
        assert_eq!(CauHinh::phan_tich(&CauHinh::default().noi_dung()), CauHinh::default());
        let c = CauHinh {
            he_so: [20.0, 9.0, 4.5, 2.0, 1.0, 0.5, 0.25, 0.1],
            ranh_gioi: [1100.0, 700.0, 500.0, 380.0, 250.0, 180.0, 120.0],
            a0_lon: A0Lon::CoDinh,
            a0_lon_he_so: 24.0,
            a0_nguong_mm: 1500.0,
            quet_trang: false,
            quet_ma: true,
            quet_xoay: false,
            nguong_trang: 0.75,
            nguong_nghi: 4.0,
            loai_ma: LoaiMa::Qr,
            ma_index: String::new(),
            tin_cay_xoay: 0.95,
            thu_muc_ra: r"D:\Kết quả".into(),
            mau_ten: "{hoso}-{n3}".into(),
            luu_nhan: LuuNhan::BangTay,
            rong_cot_trai: 333.0,
            co_anh_hs: 6,
            co_anh_vb: 0,
            ty_le_vb: 150,
            che_do_vb: CheDoVb::HaiTrang,
            an_dong_thu_muc: true,
            muc_luc: CauHinhMucLuc {
                thu_muc_ra: r"E:\Mục lục".into(),
                mau_ten: "ML-{so_ho_so}".into(),
                ghi_de: false,
                rong_trai: 412.0,
                file: r"G:\Biên mục=1.xlsx".into(),
                sheet: "Sheet 2".into(),
                dong_tieu_de: 4,
                mau: r"D:\Mẫu riêng.docx".into(),
                bien: crate::muc_luc::RangBuoc { cot_nhom: "Số HS".into(), bien: vec![("a".into(), crate::muc_luc::Nguon::Hang("x=y".into()))] }.json(),
            },
            boc_tach: {
                let mut b = CauHinhBocTach { mac_dinh: Some(NhaCungCap::Gemini), dpi: 150, canh_toi_da: 1600, jpeg: 85, max_tokens: 4096, ..Default::default() };
                *b.mo_hinh_mut(NhaCungCap::NoiBo) = MoHinhBocTach { dia_chi: "http://192.168.1.5:8001/v1".into(), model: "5CD-AI/Vintern-1B-v3_5".into(), song_song: 1, cho_s: 300 };
                b.mo_hinh_mut(NhaCungCap::Gemini).model = "gemini-2.5-flash".into();
                b.sheet_cach = CachGhiSheet::AppsScript;
                (b.ghi_sheet, b.sheet_file_khoa) = (true, r"D:\khoa=1.json".into());
                b
            },
        };
        assert!(c.kiem_tra().is_empty(), "{:?}", c.kiem_tra());
        assert_eq!(CauHinh::phan_tich(&c.noi_dung()), c);
        for loai in [LoaiMa::Code128, LoaiMa::Chu] {
            let c = CauHinh { loai_ma: loai, ..Default::default() };
            assert_eq!(CauHinh::phan_tich(&c.noi_dung()).loai_ma, loai);
        }
    }

    #[test]
    fn boc_tach_gia_tri_ngoai_khoang_ve_mac_dinh() {
        let c = CauHinh::phan_tich("bt_dpi=9999\nbt_noi_bo_song_song=0\nbt_gemini_cho=5\nbt_mac_dinh=khong_co\nbt_claude_model=claude-haiku-4-5\nbt_open_router_x=1");
        assert_eq!(c.boc_tach.dpi, 200);
        assert_eq!(c.boc_tach.mo_hinh(NhaCungCap::NoiBo).song_song, 1);
        assert_eq!(c.boc_tach.mo_hinh(NhaCungCap::Gemini).cho_s, 60);
        assert_eq!(c.boc_tach.mac_dinh, None);
        assert_eq!(c.boc_tach.mo_hinh(NhaCungCap::Claude).model, "claude-haiku-4-5");
        assert!(!c.noi_dung().to_lowercase().contains("khoa_api"), "khoá API không bao giờ nằm trong file cấu hình");
    }

    #[test]
    fn file_hong_hoac_thieu_khoa_dung_mac_dinh() {
        assert_eq!(CauHinh::phan_tich(""), CauHinh::default());
        assert_eq!(CauHinh::phan_tich("rác\n\0\u{FFFD}=\n==\nhe_so_a9=3\nhe_so_a=1"), CauHinh::default());
        let c = CauHinh::phan_tich("he_so_a2=abc\nquet_ma=có\nnguong_trang=x\nloai_ma=pdf417\nranh_gioi=1;2\nnguong_nghi=3,5");
        assert_eq!(c, CauHinh { nguong_nghi: 3.5, ..Default::default() }, "dấu phẩy thập phân vẫn đọc được");
    }

    #[test]
    fn gia_tri_sai_bi_tu_choi_luc_kiem_va_ve_mac_dinh_luc_doc() {
        let kiem = |c: CauHinh| c.kiem_tra();
        let md = CauHinh::default;
        assert_eq!(kiem(CauHinh { mau_ten: "{hoso}".into(), ..md() }), [LoiCauHinh::MauTen]);
        assert_eq!(kiem(CauHinh { muc_luc: CauHinhMucLuc { mau_ten: " ".into(), ..Default::default() }, ..md() }), [LoiCauHinh::MauTenMucLuc]);
        let ml = CauHinh::phan_tich("ml_mau_ten=\nml_rong_trai=9999\nml_dong_tieu_de=0\nml_ghi_de=x").muc_luc;
        assert_eq!(ml, CauHinhMucLuc { rong_trai: RONG_TRAI_ML.1, ..Default::default() }, "mẫu tên trống / dòng 0 / bool sai → mặc định; độ rộng kẹp");
        assert_eq!(kiem(CauHinh { nguong_trang: 3.5, ..md() }), [LoiCauHinh::Nguong]);
        assert_eq!(kiem(CauHinh { nguong_nghi: 10.5, ..md() }), [LoiCauHinh::Nguong]);
        assert_eq!(kiem(CauHinh { nguong_trang: -0.1, ..md() }), [LoiCauHinh::Nguong]);
        assert_eq!(kiem(CauHinh { tin_cay_xoay: 0.45, ..md() }), [LoiCauHinh::TinCay]);
        assert_eq!(kiem(CauHinh { tin_cay_xoay: 0.96, ..md() }), [LoiCauHinh::TinCay]);
        assert_eq!(kiem(CauHinh { a0_lon_he_so: 15.0, ..md() }), [LoiCauHinh::A0LonHeSo]);
        assert_eq!(kiem(CauHinh { a0_nguong_mm: 1100.0, ..md() }), [LoiCauHinh::A0Nguong]);
        assert_eq!(kiem(CauHinh { he_so: [16.0, 8.0, 0.0, 2.0, 1.0, 0.5, 0.25, 0.125], ..md() }), [LoiCauHinh::HeSo(2)]);
        assert_eq!(kiem(CauHinh { ranh_gioi: [1015.0, 717.5, 507.0, 430.0, 253.5, 179.0, 126.5], ..md() }), [LoiCauHinh::RanhGioi]);

        let doc = CauHinh::phan_tich("mau_ten={hoso}\nnguong_trang=5\nnguong_nghi=4\ntin_cay_xoay=0.3\nranh_gioi=1015;717.5;507;430;253.5;179;126.5\nhe_so_a0=30\na0_lon_he_so=20\nquet_xoay=0");
        assert_eq!(doc.mau_ten, MAU_TEN_MAC_DINH);
        assert_eq!((doc.nguong_trang, doc.nguong_nghi), (1.0, 3.2));
        assert_eq!(doc.tin_cay_xoay, 0.8);
        assert_eq!(doc.ranh_gioi, RANH_GIOI_MAC_DINH);
        assert_eq!(doc.he_so[0], 30.0);
        assert_eq!(doc.a0_lon_he_so, 30.0, "hệ số cố định < hệ số A0 → lấy hệ số A0");
        assert!(!doc.quet_xoay, "khoá hợp lệ khác vẫn giữ");
    }

    #[test]
    fn ghi_vao_cho_dau_tien_ghi_duoc_va_doc_lai() {
        let d = tam("ghi");
        // Chỗ 1 là file thường nên không tạo được thư mục con → rơi xuống chỗ 2
        std::fs::write(d.join("khong_phai_thu_muc"), "").unwrap();
        let cho = vec![d.join("khong_phai_thu_muc/sm-dema4.cfg"), d.join("nguoi_dung/sm-dema4.cfg")];
        let c = CauHinh { an_dong_thu_muc: true, rong_cot_trai: 300.0, ..Default::default() };
        assert_eq!(ghi_vao(&c, &cho).unwrap(), cho[1]);
        assert_eq!(doc_tu(&cho), c);
        assert_eq!(doc_tu(&[d.join("khong_co.cfg")]), CauHinh::default());
        // Ghi được chỗ 1 → chỗ 1 được ưu tiên khi đọc
        let cho2 = vec![d.join("canh_exe.cfg"), cho[1].clone()];
        ghi_vao(&CauHinh::default(), &cho2).unwrap();
        assert_eq!(doc_tu(&cho2), CauHinh::default());
        assert!(!d.join("canh_exe.cfg.tmp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn file_an_ten_ung_dung_va_chuyen_tu_ten_cu() {
        assert!(TEN_FILE.starts_with('.') && TEN_FILE.contains("dema4_saomai"), "ẩn trên macOS / Linux, cùng tên ứng dụng");
        let d = tam("ten_moi");
        let (exe, nd) = (d.join("canh_exe"), d.join("nguoi_dung"));
        std::fs::create_dir_all(&exe).unwrap();
        let cu = CauHinh { rong_cot_trai: 321.0, ..Default::default() };
        std::fs::write(exe.join(TEN_FILE_CU), cu.noi_dung()).unwrap();
        let thu_muc = vec![exe.clone(), nd];
        assert_eq!(doc_tu(&cho_doc(&thu_muc)), cu, "chưa có file mới → đọc file tên cũ");
        let moi = CauHinh { rong_cot_trai: 222.0, ..cu.clone() };
        assert_eq!(ghi_trong(&moi, &thu_muc).unwrap(), exe.join(TEN_FILE));
        assert!(!exe.join(TEN_FILE_CU).exists(), "ghi xong xoá file tên cũ");
        assert_eq!(doc_tu(&cho_doc(&thu_muc)), moi);
        ghi_trong(&cu, &thu_muc).unwrap(); // ghi đè file đã có (Windows: file đang ẩn)
        assert_eq!(doc_tu(&cho_doc(&thu_muc)), cu);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn quy_tac_dem_theo_cau_hinh() {
        use crate::dem_pdf::{A0Lon as A0, QuyTac, PT_MOI_MM};
        assert_eq!(CauHinh::default().quy_tac(), QuyTac::default(), "mặc định → đúng công thức cũ");
        // Dời ranh giới A3|A4 lên 380 mm → trang cạnh dài 370 mm thành A4
        let mut c = CauHinh::default();
        c.ranh_gioi[3] = 380.0;
        let q = c.quy_tac();
        assert_eq!(QuyTac::default().kho(250.0 * PT_MOI_MM, 370.0 * PT_MOI_MM), 3);
        assert_eq!(q.kho(250.0 * PT_MOI_MM, 370.0 * PT_MOI_MM), 4);
        assert_eq!(q.kho(250.0 * PT_MOI_MM, 381.0 * PT_MOI_MM), 3);
        // Sát ngưỡng tính theo ranh giới mới: A4 297 → ranh 380, trang 363,4 mm = 80 %
        let (ty_le, ke) = q.sat_nguong(200.0 * PT_MOI_MM, 363.4 * PT_MOI_MM, 4);
        assert_eq!(ke, Some(3));
        assert!((ty_le - 0.8).abs() < 1e-3, "{ty_le}");
        // A0 lớn: 1500 × 900 mm → diện tích 21,64 (A4 tính theo pt như lõi đếm cũ) / cố định 20
        let (w, h) = (900.0 * PT_MOI_MM, 1500.0 * PT_MOI_MM);
        assert_eq!(CauHinh::default().quy_tac().quy_a4(0, w, h), 21.64);
        let c = CauHinh { a0_lon: A0Lon::CoDinh, a0_lon_he_so: 20.0, ..Default::default() };
        assert_eq!(c.quy_tac().a0_lon, A0::CoDinh(20.0));
        assert_eq!(c.quy_tac().quy_a4(0, w, h), 20.0);
        assert_eq!(c.quy_tac().quy_a4(0, 841.0 * PT_MOI_MM, 1189.0 * PT_MOI_MM), 16.0, "A0 chuẩn vẫn theo hệ số");
        // Nâng ngưỡng A0 lớn lên 1600 mm → trang 1500 mm tính theo hệ số A0
        let c = CauHinh { a0_nguong_mm: 1600.0, ..Default::default() };
        assert_eq!(c.quy_tac().quy_a4(0, w, h), 16.0);
        assert!(c.quy_tac().dong_tuy_chinh().contains("1600 mm"));
        assert_eq!(CauHinh::default().quy_tac().dong_tuy_chinh(), "");
        let mut c = CauHinh::default();
        c.ranh_gioi[3] = 380.0;
        assert_eq!(c.quy_tac().dong_tuy_chinh(), "Ranh giới khổ (mm): 1015 · 717,5 · 507 · 380 · 253,5 · 179 · 126,5\n");
    }

    #[test]
    fn loai_ma_chua_ho_tro_thi_khong_doc_ma() {
        let c = CauHinh { loai_ma: LoaiMa::Qr, nguong_trang: 0.5, ..Default::default() };
        let ch = c.cau_hinh_phan_tich();
        assert!(!ch.quet_ma && ch.quet_trang);
        assert!((ch.nguong_trang - 0.005).abs() < 1e-7 && (ch.nguong_nghi - 0.032).abs() < 1e-7);
    }
}
