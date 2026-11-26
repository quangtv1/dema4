//! Màn Xóa ghim (bố cục như XoaGhim): thanh trên (Góc / Cạnh / Tùy biến, lọc trang, vùng đang chọn + Rộng / Cao /
//! Ngưỡng, DPI / Nén / thư mục ra, nút Xóa ghim), cột trái danh sách file (chọn / lọc / số trang), giữa 2 khung
//! **Gốc | Đích** cuộn liên tục cùng nhau + cột trang thu nhỏ, thanh đáy (đếm vùng, chuyển trang, phóng).
//!
//! Góc / Cạnh = vùng chung thư mục; Tùy biến (vẽ tay) = vùng riêng của file. Ảnh Đích vẽ ở đúng DPI sẽ xuất bằng
//! đúng bước xoá. Vùng / tuỳ chọn lưu kho Xóa ghim (chung theo thư mục, riêng theo nội dung PDF), ghi trễ ngắn.

mod chay;
mod danh_sach;
mod khung_trang;
mod sua_vung;
mod thanh_tren;
mod xem_truoc;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, TextureHandle};

use crate::xoa_ghim::kho::{self as kho_xg, CauHinhXuat, ThuMuc};
use crate::xoa_ghim::vung::{co_san, ApTrang, LoaiVung, ViTri};
use crate::xoa_ghim::xuat::{self, HAU_TO};
use sua_vung::{BoVung, Canh, ChonVung, PhamVi};
use xem_truoc::{CauHinhXem, KetQua, XemTruoc, YeuCau};

pub const TIEU_DE: &str = "Xóa ghim (Giải pháp số Sao Mai)";
/// Ghi vùng / tuỳ chọn vào kho sau khi ngừng chỉnh chừng này (kéo thanh trượt không ghi liên tục).
const TRE_GHI: Duration = Duration::from_millis(600);

/// Vùng mặc định của thư mục chưa chỉnh lần nào: góc trên trái + góc trên phải (như XoaGhim).
pub fn thu_muc_mac_dinh() -> ThuMuc {
    ThuMuc { luc: 0, vung: vec![co_san(ViTri::GocTrenTrai), co_san(ViTri::GocTrenPhai)], tuy_chon: Default::default(), xuat: CauHinhXuat::default(), ghi: Default::default() }
}

/// Đường dẫn thư mục kết quả mặc định của nguồn (cạnh nguồn).
pub fn thu_muc_ket_qua(nguon: &Path) -> Result<PathBuf, String> {
    crate::tach_vb::tach::thu_muc_ra_hau_to(nguon, None, HAU_TO)
}

/// Lọc trang (cột nút tròn như XoaGhim): Góc / Cạnh dùng Tất cả / Lẻ / Chẵn; vùng vẽ thêm Từng trang, Vô đối.
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
}

/// Tab thanh công cụ (như XoaGhim).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabCongCu {
    TepTin,
    Xem,
    ChinhSua,
    CaiDat,
}

/// Ảnh 1 trang đang hiện: gốc + đích (đích theo cấu hình mã `ma`).
struct AnhTrang {
    goc: TextureHandle,
    dich: TextureHandle,
    ma: u64,
    dpi: u16,
    co_anh: (usize, usize),
}

pub struct ManXoaGhim {
    pub thu_muc: Option<PathBuf>,
    /// (PDF nguồn, PDF kết quả) theo thứ tự cây thư mục.
    ds: Vec<(PathBuf, PathBuf)>,
    /// File được chọn để chạy (ô đánh dấu cột trái).
    chon_chay: Vec<bool>,
    so_trang_file: Vec<Option<u32>>,
    nhan_so_trang: Option<Receiver<(usize, u32)>>,
    sap_giam: bool,
    loc: String,
    chon: usize,
    bo: BoVung,
    /// Đang vẽ vùng tay: `Some(Xoa)` = "−", `Some(BaoVe)` = "+"; `None` = chọn / kéo vùng.
    ve: Option<LoaiVung>,
    loc_trang: LocTrang,
    tab: TabCongCu,
    /// Thu gọn thanh công cụ (chỉ còn hàng tab).
    an_cong_cu: bool,
    /// Ẩn khung Đích (khung Gốc rộng hết).
    an_dich: bool,
    /// Hiện cột Trang thu nhỏ.
    hien_anh_nho: bool,
    /// Kéo vẽ vùng mới: (trang, điểm bắt đầu).
    keo: Option<(u32, Pos2)>,
    /// Đang kéo vùng có sẵn: (vùng, cạnh, vùng lúc bắt đầu, điểm bắt đầu).
    keo_vung: Option<(ChonVung, Canh, crate::xoa_ghim::vung::Vung, Pos2)>,
    // --- Xem ---
    xem: XemTruoc,
    the_he: u64,
    /// Khổ từng trang (điểm) của file đang xem; trống = đang mở.
    kho_trang: Vec<(f32, f32)>,
    anh: HashMap<u32, AnhTrang>,
    cho_anh: HashSet<(u32, u64)>,
    anh_nho: HashMap<u32, TextureHandle>,
    cho_nho: HashSet<u32>,
    /// Điểm màn hình / 1 điểm PDF; `None` = vừa khung (ngang, hoặc cao khi `vua_cao`).
    ty_le: Option<f32>,
    vua_cao: bool,
    lien_tiep: bool,
    /// Trang đang xem (trang trên cùng đang thấy khi cuộn liên tục).
    trang: u32,
    cuon_toi: Option<u32>,
    /// Tỉ lệ đang hiển thị ở khung vừa vẽ (cho nút phóng).
    ty_le_cuoi: f32,
    loi: Option<String>,
    can_ghi: Option<Instant>,
    pub license: std::sync::Arc<crate::license::KiemTra>,
    /// Trạng thái từng file (theo `ds`); trống = đang đọc.
    tinh_trang: Vec<crate::xoa_ghim::lo::TinhTrangFile>,
    nhan_tt: Option<Receiver<Vec<crate::xoa_ghim::lo::TinhTrangFile>>>,
    chay: Option<chay::DangChay>,
    da_chay: Option<chay::DaChay>,
    hop_license: Option<crate::hop_license::HopLicense>,
    /// Hết lượt rồi được cấp thêm → chạy tiếp đúng kiểu đã bấm (`true` = mọi file đã chọn).
    cho_chay: Option<bool>,
    /// Lần chạy gần nhất: (file ra, tổng) — cho Làm tiếp gần đây.
    so_da_xoa: Option<(usize, usize)>,
    /// Bấm "Mở ở Tách" ở hộp kết quả → màn chính chuyển sang Tách với thư mục này.
    pub mo_o_tach: Option<PathBuf>,
}

impl Default for ManXoaGhim {
    fn default() -> Self {
        ManXoaGhim {
            thu_muc: None,
            ds: Vec::new(),
            chon_chay: Vec::new(),
            so_trang_file: Vec::new(),
            nhan_so_trang: None,
            sap_giam: false,
            loc: String::new(),
            chon: 0,
            bo: BoVung::moi(thu_muc_mac_dinh()),
            ve: None,
            loc_trang: LocTrang::TatCa,
            tab: TabCongCu::ChinhSua,
            an_cong_cu: false,
            an_dich: false,
            hien_anh_nho: true,
            keo: None,
            keo_vung: None,
            xem: XemTruoc::default(),
            the_he: 0,
            kho_trang: Vec::new(),
            anh: HashMap::new(),
            cho_anh: HashSet::new(),
            anh_nho: HashMap::new(),
            cho_nho: HashSet::new(),
            ty_le: None,
            vua_cao: false,
            lien_tiep: true,
            trang: 0,
            cuon_toi: None,
            ty_le_cuoi: 1.0,
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
        }
    }
}

impl ManXoaGhim {
    /// Mở thư mục nguồn: liệt kê PDF (bỏ thư mục kết quả `*_xoaghim`), nạp vùng chung của thư mục từ kho.
    pub fn mo_thu_muc(&mut self, p: PathBuf) {
        self.ghi_ngay();
        let ra = thu_muc_ket_qua(&p);
        self.loi = ra.as_ref().err().cloned();
        self.ds = xuat::ds_viec(&p, ra.as_deref().unwrap_or(&p), &std::sync::atomic::AtomicBool::new(false));
        self.chon_chay = vec![true; self.ds.len()];
        self.doc_so_trang();
        let tm = kho_xg::doc_thu_muc(&mut kho_xg::kho(), &p).unwrap_or_else(thu_muc_mac_dinh);
        self.bo = BoVung::moi(tm);
        self.thu_muc = Some(p);
        self.chon = usize::MAX;
        self.chon_file(0);
        self.tinh_trang.clear();
        self.so_da_xoa = None;
        self.doc_trang_thai();
        if self.ds.is_empty() && self.loi.is_none() {
            self.loi = Some("Thư mục không có PDF".into());
        }
    }

    /// Số trang từng file ở luồng nền (danh sách hiện dần).
    fn doc_so_trang(&mut self) {
        self.so_trang_file = vec![None; self.ds.len()];
        let ds: Vec<PathBuf> = self.ds.iter().map(|(n, _)| n.clone()).collect();
        let (gui, nhan) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for (i, p) in ds.iter().enumerate() {
                let n = mupdf::Document::open(p.to_string_lossy().as_ref()).and_then(|d| d.page_count()).unwrap_or(0).max(0) as u32;
                if gui.send((i, n)).is_err() {
                    return;
                }
            }
        });
        self.nhan_so_trang = Some(nhan);
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
        self.keo_vung = None;
        self.xem.gui(YeuCau::Mo { the_he: self.the_he, pdf: self.ds[i].0.clone() });
        // Id qua bảng đường dẫn của kho Tách (Dọn dữ liệu cũ dựa vào bảng này để biết PDF còn dùng)
        let id = crate::tach_vb::kho::ids_cua(std::slice::from_ref(&self.ds[i].0))[0];
        let vf = id.and_then(|id| kho_xg::doc_vung_file(&mut kho_xg::kho(), id)).unwrap_or_default();
        self.bo.doi_file(id, vf);
    }

    /// Cấu hình xoá hiện tại + mã của nó (đổi mã → ảnh Đích phải làm lại).
    fn cau_hinh_xem(&self) -> (CauHinhXem, u64) {
        let ch = CauHinhXem { vung: self.bo.vung(), tc: self.bo.tm.tuy_chon, dpi_chon: self.bo.tm.xuat.dpi };
        let ma = kho_xg::ma_cau_hinh(&ch.vung, &ch.tc, &CauHinhXuat { dpi: ch.dpi_chon, ..Default::default() });
        (ch, ma)
    }

    fn nhan_nen(&mut self, ctx: &egui::Context) {
        if let Some(n) = &self.nhan_so_trang {
            for (i, so) in n.try_iter() {
                if let Some(x) = self.so_trang_file.get_mut(i) {
                    *x = Some(so);
                }
            }
            if self.so_trang_file.iter().all(Option::is_some) {
                self.nhan_so_trang = None;
            }
        }
        for kq in self.xem.nhan() {
            match kq {
                KetQua::KhoTrang { the_he, kho } if the_he == self.the_he => self.kho_trang = kho,
                KetQua::AnhNho { the_he, trang, anh } if the_he == self.the_he => {
                    self.cho_nho.remove(&trang);
                    self.anh_nho.insert(trang, ctx.load_texture(format!("xg_nho_{trang}"), anh, egui::TextureOptions::LINEAR));
                }
                KetQua::Trang { the_he, trang, ma, dpi, co_anh, goc, dich } if the_he == self.the_he => {
                    self.cho_anh.retain(|(t, _)| *t != trang);
                    let opt = egui::TextureOptions::LINEAR;
                    self.anh.insert(trang, AnhTrang { goc: ctx.load_texture(format!("xg_goc_{trang}"), goc, opt), dich: ctx.load_texture(format!("xg_dich_{trang}"), dich, opt), ma, dpi, co_anh });
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

    pub fn ve(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.nhan_nen(&ctx);
        self.nhan_trang_thai();
        self.nhan_chay();
        self.cac_hop(&ctx);
        if (self.bo.doi_chung || self.bo.doi_rieng) && self.can_ghi.is_none() {
            self.can_ghi = Some(Instant::now());
        }
        if self.can_ghi.is_some_and(|t| t.elapsed() >= TRE_GHI) {
            self.ghi_ngay();
        } else if self.can_ghi.is_some() {
            ctx.request_repaint_after(TRE_GHI);
        }
        if self.chay.is_none() && self.hop_license.is_none() && self.da_chay.is_none() {
            self.phim(&ctx);
            self.tha_vao(&ctx);
        }
        self.thanh_tren(ui);
        self.thanh_duoi(ui);
        if !self.ds.is_empty() {
            self.danh_sach(ui);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(crate::gui::NEN)).show(ui, |ui| self.khung_trang(ui));
        let cho_mo = self.kho_trang.is_empty() && !self.ds.is_empty() && self.loi.is_none();
        if !self.cho_anh.is_empty() || !self.cho_nho.is_empty() || cho_mo || self.nhan_so_trang.is_some() || self.nhan_tt.is_some() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
    }

    /// Hộp chọn thư mục PDF.
    fn chon_thu_muc(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục PDF cần xoá ghim").pick_folder() {
            self.mo_thu_muc(p);
        }
    }

    /// Hộp chọn 1 file PDF: mở thư mục chứa nó, chọn đúng file.
    fn chon_file_pdf(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Chọn file PDF cần xoá ghim").add_filter("PDF", &["pdf", "PDF"]).pick_file() {
            self.mo_pdf(p);
        }
    }

    fn mo_pdf(&mut self, p: PathBuf) {
        if let Some(cha) = p.parent() {
            self.mo_thu_muc(cha.to_path_buf());
            if let Some(i) = self.ds.iter().position(|(n, _)| *n == p) {
                self.chon_file(i);
            }
        }
    }

    /// Kéo thả thư mục (mở) hoặc 1 PDF (mở thư mục chứa nó, chọn đúng file) vào cửa sổ.
    fn tha_vao(&mut self, ctx: &egui::Context) {
        let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())) else { return };
        if p.is_dir() {
            self.mo_thu_muc(p);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
            self.mo_pdf(p);
        }
    }

    fn sang_trang(&mut self, t: u32) {
        let n = self.kho_trang.len() as u32;
        if n > 0 {
            self.trang = t.min(n - 1);
            self.cuon_toi = Some(self.trang);
        }
    }

    fn phong(&mut self, k: f32) {
        self.ty_le = Some((self.ty_le_cuoi * k).clamp(0.1, 8.0));
    }

    fn phim(&mut self, ctx: &egui::Context) {
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        use egui::Key;
        let (truoc, sau, len, xuong, xoa, hoan_tac, thoat, phong_to, thu_nho, vua) = ctx.input(|i| {
            (
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::CloseBracket),
                i.key_pressed(Key::PageUp),
                i.key_pressed(Key::PageDown),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.modifiers.command && i.key_pressed(Key::Z),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Plus) || (i.key_pressed(Key::Equals) && i.modifiers.shift),
                i.key_pressed(Key::Minus),
                i.key_pressed(Key::Equals) && !i.modifiers.shift,
            )
        });
        if truoc && self.chon > 0 {
            self.chon_file(self.chon - 1);
        }
        if sau {
            self.chon_file(self.chon + 1);
        }
        if len && self.trang > 0 {
            self.sang_trang(self.trang - 1);
        }
        if xuong {
            self.sang_trang(self.trang + 1);
        }
        if xoa {
            self.bo.xoa_chon();
        }
        if hoan_tac {
            self.bo.hoan_tac();
        }
        if thoat {
            self.ve = None;
        }
        if phong_to {
            self.phong(1.25);
        }
        if thu_nho {
            self.phong(0.8);
        }
        if vua {
            self.ty_le = None;
            self.vua_cao = false;
        }
    }

    /// Bấm vùng Góc / Cạnh trên biểu tượng: bật (theo lọc trang đang chọn) / tắt.
    fn bat_tat_co_san(&mut self, v: ViTri) {
        if self.bo.bat_tat(v) {
            let ap = match self.loc_trang {
                LocTrang::Le => ApTrang::Le,
                LocTrang::Chan => ApTrang::Chan,
                _ => ApTrang::TatCa,
            };
            if let Some(i) = self.bo.tm.vung.iter().position(|z| z.vi_tri == v) {
                self.bo.tm.vung[i].ap_trang = ap;
                self.bo.chon = Some((false, i));
            }
        }
    }

    /// Thêm vùng vẽ tay (vùng riêng của file) theo chế độ vẽ + lọc trang.
    fn them_ve(&mut self, khung: (f64, f64, f64, f64), trang: u32) {
        let Some(loai) = self.ve else { return };
        let loai = if loai == LoaiVung::Xoa && self.loc_trang == LocTrang::VoDoi { LoaiVung::VoDoi } else { loai };
        let pham_vi = if matches!(self.loc_trang, LocTrang::TungTrang | LocTrang::VoDoi) { PhamVi::Trang } else { PhamVi::File };
        if !self.bo.them_ve(khung, loai, pham_vi, self.loc_trang.ap(trang), trang) && self.bo.id_file.is_none() {
            self.loi = Some("Không đọc được PDF này — không vẽ được vùng riêng".into());
        }
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

/// Màu khung vùng theo loại (như XoaGhim: xoá xanh dương, bảo vệ hồng, vô đối đỏ).
fn mau_loai(l: LoaiVung) -> Color32 {
    match l {
        LoaiVung::Xoa => Color32::from_rgb(0x3B, 0x82, 0xF6),
        LoaiVung::BaoVe => Color32::from_rgb(0xF4, 0x72, 0xB6),
        LoaiVung::VoDoi => Color32::from_rgb(0xDC, 0x26, 0x26),
    }
}

#[cfg(test)]
mod tests;
