//! Giao diện Xuất phiên / Nhập phiên (menu Chức năng): hộp chọn file (cùng cách mở như các hộp chọn khác của app),
//! việc nặng ở luồng nền, hộp chờ modal khi đang chạy (chặn chuột; App khoá phím tắt màn Tách khi hộp đang mở),
//! hộp xác nhận nhập, hộp kết quả.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui::{self, Align, Layout, RichText};

use crate::gui::{CHU, CHU_PHU, DO, XANH_LA};
use crate::gui_tach::khung_hop as kh;
use crate::phien_lam_viec::{self as plv, BaoCaoGop, TomTatXuat, XemTruoc};

#[derive(Clone, Copy, PartialEq)]
enum Viec {
    Xuat,
    XemTruoc,
    Nhap,
}

impl Viec {
    fn chu(self) -> &'static str {
        match self {
            Viec::Xuat => "Đang xuất phiên làm việc…",
            Viec::XemTruoc => "Đang đọc gói phiên…",
            Viec::Nhap => "Đang nhập phiên làm việc…",
        }
    }
}

/// Ngưỡng cảnh báo trong hộp xác nhận nhập.
const KHOP_TOI_THIEU: f32 = 0.9;
const LECH_DONG_HO_CANH_BAO_MS: i64 = 10 * 60 * 1000;

enum SuKien {
    XemTruoc { zip: PathBuf, kq: Result<XemTruoc, String> },
    Xuat { zip: PathBuf, kq: Result<TomTatXuat, String> },
    Nhap(Result<BaoCaoGop, String>),
}

enum TrangThai {
    /// Hộp chọn / lưu file đang mở.
    ChonFile { nhan: Receiver<Option<PathBuf>>, viec: Viec, goc: PathBuf },
    Cho { nhan: Receiver<SuKien>, viec: Viec, goc: PathBuf },
    XacNhan { goc: PathBuf, zip: PathBuf, xt: XemTruoc },
    DaXuat { zip: PathBuf, tt: TomTatXuat },
    DaNhap(BaoCaoGop),
    Loi(String),
}

/// Việc App phải làm sau 1 khung hình của hộp phiên.
pub enum HanhDong {
    /// Người dùng bấm Nhập: App ghi trạng thái trong bộ nhớ xuống đĩa, bỏ phiên đang mở (không lưu lại) rồi gọi `nhap`.
    XacNhanNhap { goc: PathBuf, zip: PathBuf },
    /// Nhập xong: App nạp lại thư mục từ đĩa (không bước lưu).
    DaNhap { goc: PathBuf },
}

#[derive(Default)]
pub struct HopPhien {
    tt: Option<TrangThai>,
}

/// Thư mục mặc định của hộp lưu gói: thư mục Tải về của người dùng; không có → thư mục cha của thư mục hồ sơ.
pub fn thu_muc_tai_ve(goc: &Path) -> PathBuf {
    let nha = if cfg!(windows) { std::env::var_os("USERPROFILE") } else { std::env::var_os("HOME") };
    nha.map(|h| PathBuf::from(h).join("Downloads")).filter(|d| d.is_dir()).unwrap_or_else(|| goc.parent().unwrap_or(goc).to_path_buf())
}

/// Hộp chọn file: Windows ở luồng riêng (cửa sổ app không đứng hình), macOS trên luồng chính (rfd) — như `chon_thu_muc_nen`.
fn hop_file_nen(hop: rfd::FileDialog, luu: bool, ctx: &egui::Context) -> Receiver<Option<PathBuf>> {
    let (gui, nhan) = mpsc::channel();
    let ctx = ctx.clone();
    let mo = move || {
        let _ = gui.send(if luu { hop.save_file() } else { hop.pick_file() });
        ctx.request_repaint();
    };
    if cfg!(target_os = "windows") {
        std::thread::spawn(mo);
    } else {
        mo();
    }
    nhan
}

/// Việc nặng ở luồng nền; kết quả gửi qua kênh (luồng chết giữa chừng → kênh đứt → hộp báo lỗi, không kẹt hộp chờ).
fn chay_nen(f: impl FnOnce() -> SuKien + Send + 'static, ctx: &egui::Context) -> Receiver<SuKien> {
    let (gui, nhan) = mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = gui.send(f());
        ctx.request_repaint();
    });
    nhan
}

fn ten_goi(goc: &Path) -> String {
    format!("{}{}", goc.file_name().map_or_else(|| "thu_muc".into(), |t| t.to_string_lossy().into_owned()), plv::DUOI_GOI)
}

fn gio(ms: u64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ms as i64).single().map_or_else(|| "?".into(), |t| t.format("%d/%m %H:%M").to_string())
}

impl HopPhien {
    /// Đang chọn file / xuất / nhập (tính vào "đang chạy" của App: khoá menu, đóng app thì hỏi).
    pub fn dang_chay(&self) -> bool {
        matches!(self.tt, Some(TrangThai::ChonFile { .. } | TrangThai::Cho { .. }))
    }

    /// Đang mở bất kỳ hộp nào của Xuất / Nhập (App khoá phím tắt màn Tách).
    pub fn dang_mo(&self) -> bool {
        self.tt.is_some()
    }

    /// Xuất: hộp lưu file rồi đóng gói ở luồng nền. Nơi gọi đã ghi hết trạng thái trong bộ nhớ của thư mục xuống đĩa.
    pub fn xuat(&mut self, goc: PathBuf, ctx: &egui::Context) {
        let hop = rfd::FileDialog::new().set_title("Xuất phiên làm việc").add_filter("Gói phiên", &["zip"]).set_directory(thu_muc_tai_ve(&goc)).set_file_name(ten_goi(&goc));
        self.tt = Some(TrangThai::ChonFile { nhan: hop_file_nen(hop, true, ctx), viec: Viec::Xuat, goc });
    }

    /// Nhập bước 1: chọn gói rồi đọc xem trước ở luồng nền.
    pub fn chon_goi(&mut self, goc: PathBuf, ctx: &egui::Context) {
        let hop = rfd::FileDialog::new().set_title("Nhập phiên làm việc").add_filter("Gói phiên", &["zip"]).set_directory(thu_muc_tai_ve(&goc));
        self.tt = Some(TrangThai::ChonFile { nhan: hop_file_nen(hop, false, ctx), viec: Viec::XemTruoc, goc });
    }

    /// Nhập bước 2 (sau khi App đã ghi xuống đĩa và bỏ phiên đang mở): gộp ở luồng nền.
    pub fn nhap(&mut self, goc: PathBuf, zip: PathBuf, ctx: &egui::Context) {
        let g = goc.clone();
        self.tt = Some(TrangThai::Cho { nhan: chay_nen(move || SuKien::Nhap(plv::nhap(&g, &zip)), ctx), viec: Viec::Nhap, goc });
    }

    /// Không nhập được trước khi gộp (vd không ghi được nhãn chưa lưu) — báo lỗi, không đụng thư mục.
    pub fn bao_loi(&mut self, loi: String) {
        self.tt = Some(TrangThai::Loi(loi));
    }

    /// Vẽ hộp đang mở; trả việc App cần làm.
    pub fn ve(&mut self, ctx: &egui::Context) -> Option<HanhDong> {
        let mut hanh_dong = None;
        if let Some(TrangThai::ChonFile { nhan, viec, goc }) = &self.tt {
            match nhan.try_recv() {
                Ok(Some(zip)) => {
                    let (viec, goc) = (*viec, goc.clone());
                    let g = goc.clone();
                    let nhan = match viec {
                        Viec::Xuat => chay_nen(move || SuKien::Xuat { kq: plv::xuat(&g, &zip), zip }, ctx),
                        _ => chay_nen(move || SuKien::XemTruoc { kq: plv::xem_truoc(&g, &zip), zip }, ctx),
                    };
                    self.tt = Some(TrangThai::Cho { nhan, viec, goc });
                }
                Ok(None) | Err(TryRecvError::Disconnected) => self.tt = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(TrangThai::Cho { nhan, viec, goc }) = &self.tt {
            let sk = match nhan.try_recv() {
                Ok(sk) => Some(Ok(sk)),
                Err(TryRecvError::Disconnected) => Some(Err(*viec)),
                Err(TryRecvError::Empty) => None,
            };
            if let Some(sk) = sk {
                let goc = goc.clone();
                self.tt = Some(match sk {
                    Ok(SuKien::Xuat { zip, kq: Ok(tt) }) => TrangThai::DaXuat { zip, tt },
                    Ok(SuKien::XemTruoc { zip, kq: Ok(xt) }) => TrangThai::XacNhan { goc, zip, xt },
                    Ok(SuKien::Nhap(Ok(bc))) => {
                        hanh_dong = Some(HanhDong::DaNhap { goc });
                        TrangThai::DaNhap(bc)
                    }
                    Ok(SuKien::Xuat { kq: Err(e), .. } | SuKien::XemTruoc { kq: Err(e), .. }) => TrangThai::Loi(e),
                    Ok(SuKien::Nhap(Err(e))) => {
                        hanh_dong = Some(HanhDong::DaNhap { goc }); // nạp lại dù lỗi: phiên đã bỏ trước khi nhập
                        TrangThai::Loi(e)
                    }
                    Err(viec) => {
                        if viec == Viec::Nhap {
                            hanh_dong = Some(HanhDong::DaNhap { goc });
                        }
                        TrangThai::Loi("Lỗi bất ngờ khi xử lý gói phiên — thử lại; còn lỗi thì gửi gói cho Sao Mai kiểm tra.".into())
                    }
                });
            }
        }
        let mut dong = false;
        match &self.tt {
            None => {}
            Some(TrangThai::ChonFile { .. }) => cho(ctx, "Đang chờ chọn file…"),
            Some(TrangThai::Cho { viec, .. }) => cho(ctx, viec.chu()),
            Some(TrangThai::XacNhan { goc, zip, xt }) => {
                let mut chon = None;
                let ph = kh::hop(ctx, "phien_xac_nhan", 460.0, "Nhập phiên làm việc?", |ui| {
                    egui::Frame::new().inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(RichText::new(format!("Gói: {} · {} hồ sơ đã làm", xt.ten_thu_muc, xt.so_ho_so)).color(CHU));
                        ui.label(RichText::new(format!("Nhập vào: {}", goc.display())).color(CHU_PHU).size(12.0));
                        let ty_le = xt.ty_le_khop();
                        let chu = format!("PDF khớp với thư mục đang mở: {}/{} ({:.0}%)", xt.so_pdf_khop, xt.so_pdf_goi, ty_le * 100.0);
                        if ty_le < KHOP_TOI_THIEU {
                            ui.label(RichText::new(format!("{chu} — có thể đang mở nhầm thư mục; PDF không khớp sẽ bị bỏ qua")).color(DO));
                        } else {
                            ui.label(RichText::new(chu).color(CHU));
                        }
                        // Gói "tạo ở tương lai" = đồng hồ máy xuất chạy nhanh (gói cũ hơn chỉ là gói mang sang muộn, không cảnh báo)
                        if xt.lech_dong_ho_ms > LECH_DONG_HO_CANH_BAO_MS {
                            ui.label(RichText::new(format!("Đồng hồ máy xuất nhanh hơn máy này khoảng {} phút — chọn bản sửa sau cùng có thể sai", xt.lech_dong_ho_ms / 60_000)).color(DO));
                        }
                        if !xt.loai_moi.is_empty() {
                            ui.label(RichText::new("Thêm loại tài liệu mới:").color(CHU));
                            for (ten, hd) in &xt.loai_moi {
                                ui.label(RichText::new(format!("• {ten} — {hd}")).color(CHU_PHU).size(12.0));
                            }
                        }
                        ui.label(RichText::new("Mỗi hồ sơ giữ bản sửa sau cùng (máy này hoặc gói); danh sách trùng hiện sau khi nhập.").color(CHU_PHU).size(12.0));
                    });
                    kh::chan(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kh::nut_chan_mau("Nhập", XANH_LA)).clicked() {
                                chon = Some(true);
                            }
                            if ui.add(kh::nut_chan("Huỷ")).clicked() {
                                chon = Some(false);
                            }
                        });
                    });
                });
                match chon.or(ph.should_close().then_some(false)) {
                    Some(true) => hanh_dong = Some(HanhDong::XacNhanNhap { goc: goc.clone(), zip: zip.clone() }),
                    Some(false) => dong = true,
                    None => {}
                }
            }
            Some(TrangThai::DaXuat { zip, tt }) => {
                let mut mo = false;
                let ph = kh::hop(ctx, "phien_da_xuat", 420.0, "Đã xuất phiên làm việc", |ui| {
                    kh::than_chu(ui, &format!("Đã xuất {} hồ sơ, {} kết quả bóc tách ({:.1} MB) vào {}. Mang file này sang máy khác, mở cùng thư mục rồi chọn Chức năng › Nhập phiên làm việc….", tt.so_ho_so, tt.so_ban_ghi_boc, tt.dung_luong as f64 / 1_048_576.0, zip.display()));
                    kh::chan(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kh::nut_chan("Đóng")).clicked() {
                                dong = true;
                            }
                            if ui.add(kh::nut_chan("Mở thư mục")).clicked() {
                                mo = true;
                            }
                        });
                    });
                });
                if mo {
                    crate::gui::mo_bang_he_dieu_hanh(zip.parent().unwrap_or(zip));
                }
                dong |= ph.should_close();
            }
            Some(TrangThai::DaNhap(bc)) => {
                let ph = kh::hop(ctx, "phien_da_nhap", 520.0, "Đã nhập phiên làm việc", |ui| {
                    egui::Frame::new().inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 5.0;
                        ui.label(RichText::new(tom_tat(bc)).color(CHU));
                        if !bc.trung.is_empty() {
                            ui.label(RichText::new("Hồ sơ hai máy cùng sửa (giữ bản sửa sau cùng):").color(CHU));
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                for t in &bc.trung {
                                    let ben = if t.lay_goi { "lấy bản gói" } else { "giữ bản máy này" };
                                    ui.label(RichText::new(format!("• {} — {ben} (gói {} · máy này {})", t.khoa, gio(t.moc_goi), gio(t.moc_may))).size(12.0).color(CHU_PHU));
                                }
                            });
                        }
                        if !bc.loai_trung.is_empty() {
                            ui.label(RichText::new(format!("Loại tài liệu trùng tên, giữ bản máy này: {}", bc.loai_trung.join(", "))).size(12.0).color(CHU_PHU));
                        }
                        if bc.lech_dong_ho > 0 {
                            ui.label(RichText::new(format!("{} mốc sửa của máy gửi vượt giờ máy này hơn 1 ngày (đồng hồ sai) — đã tính theo giờ máy này", bc.lech_dong_ho)).size(12.0).color(DO));
                        }
                    });
                    kh::chan(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kh::nut_chan("Đóng")).clicked() {
                                dong = true;
                            }
                        });
                    });
                });
                dong |= ph.should_close();
            }
            Some(TrangThai::Loi(e)) => {
                let ph = kh::hop(ctx, "phien_loi", 420.0, "Không xuất / nhập được", |ui| {
                    kh::than_chu(ui, e);
                    kh::chan(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(kh::nut_chan("Đóng")).clicked() {
                                dong = true;
                            }
                        });
                    });
                });
                dong |= ph.should_close();
            }
        }
        if dong {
            self.tt = None;
        }
        hanh_dong
    }
}

/// Hộp chờ (modal, không đóng được) khi đang chọn file / chạy.
fn cho(ctx: &egui::Context, viec: &str) {
    kh::hop(ctx, "phien_cho", 340.0, "Phiên làm việc", |ui| {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.spinner();
            ui.label(RichText::new(viec).color(CHU));
        });
        ui.add_space(16.0);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

/// Dòng tóm tắt kết quả nhập.
fn tom_tat(bc: &BaoCaoGop) -> String {
    let mut s = format!("Tách: nhận {} hồ sơ · giữ {} của máy này · {} trùng", bc.nhan, bc.giu, bc.trung.len());
    if bc.boc_nhan + bc.boc_giu > 0 {
        s += &format!("\nBóc tách: nhận {} kết quả · giữ {} của máy này", bc.boc_nhan, bc.boc_giu);
    }
    if !bc.loai_moi.is_empty() {
        s += &format!("\nThêm loại tài liệu: {}", bc.loai_moi.join(", "));
    }
    if !bc.loai_loi.is_empty() {
        s += &format!("\nBỏ qua loại tài liệu không hợp lệ: {}", bc.loai_loi.join(", "));
    }
    if !bc.bo_qua.is_empty() {
        s += &format!("\nBỏ qua {} mục không có PDF tương ứng trong thư mục này, hoặc PDF khác bản (nội dung file không giống máy gửi)", bc.bo_qua.len());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_goi_va_tom_tat() {
        assert_eq!(ten_goi(Path::new("/a/HS_2024")), "HS_2024.sm-dema4.zip");
        let bc = BaoCaoGop { nhan: 3, giu: 2, boc_nhan: 1, bo_qua: vec!["x.pdf".into()], loai_moi: vec!["HĐ".into()], ..Default::default() };
        let s = tom_tat(&bc);
        assert!(s.contains("nhận 3 hồ sơ · giữ 2 của máy này · 0 trùng") && s.contains("Bóc tách: nhận 1") && s.contains("Bỏ qua 1") && s.contains("HĐ"), "{s}");
    }

    fn mot_khung(hp: &mut HopPhien) -> Option<HanhDong> {
        let ctx = egui::Context::default();
        let mut kq = None;
        let mut ra = ctx.run_ui(Default::default(), |ui| kq = hp.ve(ui.ctx()));
        ra.textures_delta.clear();
        kq
    }

    #[test]
    fn luong_chet_giua_chung_bao_loi_khong_ket_hop_cho_nhap_thi_van_nap_lai() {
        for (viec, nap_lai) in [(Viec::Xuat, false), (Viec::XemTruoc, false), (Viec::Nhap, true)] {
            let (gui, nhan) = mpsc::channel::<SuKien>();
            drop(gui);
            let mut hp = HopPhien { tt: Some(TrangThai::Cho { nhan, viec, goc: "/a/HS".into() }) };
            assert!(hp.dang_chay());
            let hd = mot_khung(&mut hp);
            assert!(matches!(hp.tt, Some(TrangThai::Loi(_))) && !hp.dang_chay(), "hộp lỗi, hết khoá");
            assert_eq!(matches!(hd, Some(HanhDong::DaNhap { .. })), nap_lai, "nhập dở → phiên đã bỏ → phải nạp lại");
        }
        // Đóng hộp chọn file (không chọn) → về như cũ
        let (gui, nhan) = mpsc::channel();
        gui.send(None).unwrap();
        let mut hp = HopPhien { tt: Some(TrangThai::ChonFile { nhan, viec: Viec::Xuat, goc: "/a/HS".into() }) };
        assert!(mot_khung(&mut hp).is_none() && hp.tt.is_none());
    }
}
