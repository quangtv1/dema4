//! Giao diện Xuất phiên / Nhập phiên (menu Chức năng): chọn file ở luồng nền, hộp chờ khi đang chạy (modal — chặn mọi
//! thao tác khác, không việc nào ghi chen vào thư mục dữ liệu đang xuất / nhập), hộp xác nhận nhập, hộp kết quả.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, Align, Layout, RichText};

use crate::gui::{CHU, CHU_PHU, DO, XANH_LA};
use crate::gui_tach::khung_hop as kh;
use crate::phien_lam_viec::{self as plv, BaoCaoGop, TomTatXuat, XemTruoc};

/// Ngưỡng cảnh báo trong hộp xác nhận nhập.
const KHOP_TOI_THIEU: f32 = 0.9;
const LECH_DONG_HO_CANH_BAO_MS: i64 = 10 * 60 * 1000;

enum SuKien {
    /// Người dùng đóng hộp chọn / lưu file.
    Huy,
    XemTruoc { goc: PathBuf, zip: PathBuf, kq: Result<XemTruoc, String> },
    Xuat { zip: PathBuf, kq: Result<TomTatXuat, String> },
    Nhap { goc: PathBuf, kq: Result<BaoCaoGop, String> },
}

enum TrangThai {
    Cho { nhan: Receiver<SuKien>, viec: &'static str },
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
        matches!(self.tt, Some(TrangThai::Cho { .. }))
    }

    /// Xuất: hộp lưu file rồi đóng gói ở luồng nền. Nơi gọi đã ghi hết trạng thái trong bộ nhớ của thư mục xuống đĩa.
    pub fn xuat(&mut self, goc: PathBuf, ctx: &egui::Context) {
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let chon = rfd::FileDialog::new().set_title("Xuất phiên làm việc").add_filter("Gói phiên", &["zip"]).set_directory(thu_muc_tai_ve(&goc)).set_file_name(ten_goi(&goc)).save_file();
            let _ = gui.send(match chon {
                Some(zip) => SuKien::Xuat { kq: plv::xuat(&goc, &zip), zip },
                None => SuKien::Huy,
            });
            ctx.request_repaint();
        });
        self.tt = Some(TrangThai::Cho { nhan, viec: "Đang xuất phiên làm việc…" });
    }

    /// Nhập bước 1: chọn gói rồi đọc xem trước ở luồng nền.
    pub fn chon_goi(&mut self, goc: PathBuf, ctx: &egui::Context) {
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let chon = rfd::FileDialog::new().set_title("Nhập phiên làm việc").add_filter("Gói phiên", &["zip"]).set_directory(thu_muc_tai_ve(&goc)).pick_file();
            let _ = gui.send(match chon {
                Some(zip) => SuKien::XemTruoc { kq: plv::xem_truoc(&goc, &zip), goc, zip },
                None => SuKien::Huy,
            });
            ctx.request_repaint();
        });
        self.tt = Some(TrangThai::Cho { nhan, viec: "Đang đọc gói phiên…" });
    }

    /// Nhập bước 2 (sau khi App đã ghi xuống đĩa và bỏ phiên đang mở): gộp ở luồng nền.
    pub fn nhap(&mut self, goc: PathBuf, zip: PathBuf, ctx: &egui::Context) {
        let (gui, nhan) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = gui.send(SuKien::Nhap { kq: plv::nhap(&goc, &zip), goc });
            ctx.request_repaint();
        });
        self.tt = Some(TrangThai::Cho { nhan, viec: "Đang nhập phiên làm việc…" });
    }

    /// Vẽ hộp đang mở; trả việc App cần làm.
    pub fn ve(&mut self, ctx: &egui::Context) -> Option<HanhDong> {
        let mut hanh_dong = None;
        if let Some(TrangThai::Cho { nhan, .. }) = &self.tt {
            if let Ok(sk) = nhan.try_recv() {
                self.tt = match sk {
                    SuKien::Huy => None,
                    SuKien::Xuat { zip, kq: Ok(tt) } => Some(TrangThai::DaXuat { zip, tt }),
                    SuKien::XemTruoc { goc, zip, kq: Ok(xt) } => Some(TrangThai::XacNhan { goc, zip, xt }),
                    SuKien::Nhap { goc, kq: Ok(bc) } => {
                        hanh_dong = Some(HanhDong::DaNhap { goc });
                        Some(TrangThai::DaNhap(bc))
                    }
                    SuKien::Xuat { kq: Err(e), .. } | SuKien::XemTruoc { kq: Err(e), .. } => Some(TrangThai::Loi(e)),
                    SuKien::Nhap { goc, kq: Err(e) } => {
                        hanh_dong = Some(HanhDong::DaNhap { goc }); // nạp lại dù lỗi: phiên đã bỏ trước khi nhập
                        Some(TrangThai::Loi(e))
                    }
                };
            }
        }
        let mut dong = false;
        match &self.tt {
            None => {}
            Some(TrangThai::Cho { viec, .. }) => {
                kh::hop(ctx, "phien_cho", 340.0, "Phiên làm việc", |ui| {
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.spinner();
                        ui.label(RichText::new(*viec).color(CHU));
                    });
                    ui.add_space(16.0);
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
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
                        if xt.lech_dong_ho_ms.abs() > LECH_DONG_HO_CANH_BAO_MS {
                            ui.label(RichText::new(format!("Đồng hồ máy xuất lệch {} phút so với máy này — chọn bản sửa sau cùng có thể sai", xt.lech_dong_ho_ms / 60_000)).color(DO));
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
                    kh::than_chu(ui, &format!("Đã xuất {} hồ sơ, {} kết quả bóc tách ({:.1} MB) vào {}. Mang file này sang máy khác, mở cùng thư mục rồi chọn Chức năng › Nhập phiên.", tt.so_ho_so, tt.so_ban_ghi_boc, tt.dung_luong as f64 / 1_048_576.0, zip.display()));
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

/// Dòng tóm tắt kết quả nhập.
fn tom_tat(bc: &BaoCaoGop) -> String {
    let mut s = format!("Tách: nhận {} hồ sơ · giữ {} của máy này · {} trùng", bc.nhan, bc.giu, bc.trung.len());
    if bc.boc_nhan + bc.boc_giu > 0 {
        s += &format!("\nBóc tách: nhận {} kết quả · giữ {} của máy này", bc.boc_nhan, bc.boc_giu);
    }
    if !bc.loai_moi.is_empty() {
        s += &format!("\nThêm loại tài liệu: {}", bc.loai_moi.join(", "));
    }
    if !bc.bo_qua.is_empty() {
        s += &format!("\nBỏ qua {} mục không có PDF tương ứng (hoặc PDF khác bản) trong thư mục này", bc.bo_qua.len());
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

    #[test]
    fn thu_muc_tai_ve_khong_co_thi_dung_thu_muc_cha() {
        let d = thu_muc_tai_ve(Path::new("/khong/co/HS"));
        assert!(d.ends_with("Downloads") || d == Path::new("/khong/co"), "{}", d.display());
    }
}
