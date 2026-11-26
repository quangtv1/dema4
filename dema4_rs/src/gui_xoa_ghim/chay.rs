//! Chạy Xóa ghim cả thư mục: kiểm lượt license `xoa_ghim` → chạy `lo::chay` ở luồng nền → hộp "Đang xoá ghim"
//! (tiến độ theo file, ước lượng thời gian còn, Dừng) → hộp kết quả (Mở thư mục, Mở ở Tách) + thống kê trạm (chỉ số đếm).
//! Trạng thái từng file (đã xoá / chưa / đổi cấu hình / kết quả mất) đọc ở luồng nền khi mở thư mục và sau mỗi lần chạy.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align};

use super::ManXoaGhim;
use crate::gui::{mo_bang_he_dieu_hanh, so_nguyen, CHU, CHU_PHU, DO};
use crate::gui_tach::khung_hop;
use crate::license::{ChucNang, LanChay};
use crate::xoa_ghim::lo::{self, TinhTrangFile, DA_DUNG};
use crate::xoa_ghim::xuat::{KetQuaFile, TrangThai};

pub struct DangChay {
    nhan: Receiver<(usize, Result<KetQuaFile, String>)>,
    pub dung: Arc<AtomicBool>,
    lan: LanChay,
    /// Chỉ số trong danh sách file của các file đang chạy.
    viec: Vec<usize>,
    pub xong: usize,
    ok: usize,
    loi: Vec<(String, String)>,
    trang: u64,
    xu_ly: u64,
    dpi: u16,
    bat_dau: Instant,
    ra: PathBuf,
}

pub struct DaChay {
    tong: usize,
    ok: usize,
    loi: Vec<(String, String)>,
    da_dung: bool,
    trang: u64,
    xu_ly: u64,
    thoi_gian: Duration,
    ra: PathBuf,
    co_telegram: bool,
}

impl ManXoaGhim {
    /// File đã đánh dấu cần chạy: chưa xoá, cấu hình đã đổi, kết quả mất (hoặc chưa biết trạng thái).
    pub(super) fn can_chay(&self) -> Vec<usize> {
        (0..self.ds.len()).filter(|&i| self.chon_chay.get(i).copied().unwrap_or(false) && !matches!(self.trang_thai_file(i), Some(TrangThai::DaXoa { dung_cau_hinh: true }))).collect()
    }

    pub(super) fn trang_thai_file(&self, i: usize) -> Option<TrangThai> {
        let tt = self.tinh_trang.get(i)?;
        // File đang xem: vùng riêng lấy bản đang sửa
        let tt = if i == self.chon { &TinhTrangFile { vung_rieng: self.bo.vf.vung.clone(), ..tt.clone() } } else { tt };
        Some(lo::trang_thai(tt, &self.bo.tm.vung, &self.bo.tm.tuy_chon, &self.bo.tm.xuat))
    }

    /// Đọc lại trạng thái các file ở luồng nền.
    pub(super) fn doc_trang_thai(&mut self) {
        let viec = self.ds.clone();
        let (gui, nhan) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = gui.send(lo::doc_tinh_trang(crate::tach_vb::kho::chung(), crate::xoa_ghim::kho::chung(), &viec));
        });
        self.nhan_tt = Some(nhan);
    }

    pub(super) fn nhan_trang_thai(&mut self) {
        if let Some(Ok(tt)) = self.nhan_tt.as_ref().map(|n| n.try_recv()) {
            if tt.len() == self.ds.len() {
                self.tinh_trang = tt;
            }
            self.nhan_tt = None;
        }
    }

    pub fn dang_chay(&self) -> bool {
        self.chay.is_some()
    }

    /// Nút **▶ Xóa ghim (n)** góc dưới phải (n = file đã tick cần chạy) + ▾ "Chạy lại mọi file đã tick (k)".
    pub(super) fn nut_chay(&mut self, ui: &mut egui::Ui) {
        use super::thiet_ke as tk;
        use super::widget::font_dam;
        let n = if self.thu_muc.is_some() { self.can_chay().len() } else { 0 };
        let k = self.chon_chay.iter().filter(|&&c| c).count();
        let duoc = n > 0 && self.chay.is_none();
        let (o, ph) = ui.allocate_exact_size(egui::vec2(126.0, 28.0), egui::Sense::click());
        let (o_chinh, o_mui) = (egui::Rect::from_min_max(o.min, egui::pos2(o.right() - 24.0, o.bottom())), egui::Rect::from_min_max(egui::pos2(o.right() - 24.0, o.top()), o.max));
        let ph_chinh = ui.interact(o_chinh, ui.scope_id().with("xg_chay"), if duoc { egui::Sense::click() } else { egui::Sense::hover() });
        let _ = ph;
        let p = ui.painter();
        let nen = if !duoc { tk::XANH_NUT_MO } else if ph_chinh.hovered() { tk::XANH_DAM_CHU } else { tk::XANH };
        p.rect_filled(o, 4.0, nen);
        p.vline(o_mui.left(), o.y_range().shrink(5.0), egui::Stroke::new(1.0, egui::Color32::from_white_alpha(110)));
        super::bieu_tuong::ve(ui, "chay", egui::Rect::from_center_size(egui::pos2(o.left() + 15.0, o.center().y), egui::vec2(11.0, 11.0)));
        let chu = if self.thu_muc.is_some() { format!("Xóa ghim ({n})") } else { "Xóa ghim".to_string() };
        p.text(egui::pos2(o.left() + 26.0, o.center().y), egui::Align2::LEFT_CENTER, chu, font_dam(tk::CHU_CO), egui::Color32::WHITE);
        ph_chinh.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, duoc, format!("Xóa ghim ({n})")));
        let goi_y = if self.thu_muc.is_none() { "Mở file hoặc thư mục trước".to_string() } else if n == 0 { "Mọi file đã tick đều đã xoá đúng vùng hiện tại".to_string() } else { format!("Xoá ghim {n} file đã tick chưa xoá / đã đổi vùng / mất kết quả") };
        if ph_chinh.on_hover_text(goi_y).clicked() {
            self.bam_chay(false);
        }
        let mut con = ui.new_child(egui::UiBuilder::new().max_rect(o_mui));
        let nut = egui::Button::image(super::bieu_tuong::anh("caret_trang", 8.0, 8.0)).frame(false).min_size(o_mui.size());
        let (r, _) = egui::containers::menu::MenuButton::from_button(nut).ui(&mut con, |ui| {
            ui.set_min_width(230.0);
            let duoc_lai = k > 0 && self.chay.is_none();
            if ui.add_enabled(duoc_lai, egui::Button::new(egui::RichText::new(format!("Chạy lại mọi file đã tick ({k})")).font(super::widget::font(tk::CHU_CO))).frame(false)).clicked() {
                self.bam_chay(true);
                ui.close();
            }
        });
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Tuỳ chọn chạy"));
    }

    /// Bấm Xóa ghim: `tat_ca` = cả thư mục, không thì chỉ file cần chạy.
    pub(super) fn bam_chay(&mut self, tat_ca: bool) {
        let viec = if tat_ca { (0..self.ds.len()).filter(|&i| self.chon_chay.get(i).copied().unwrap_or(false)).collect() } else { self.can_chay() };
        if viec.is_empty() || self.chay.is_some() {
            return;
        }
        let Some(ra) = self.thu_muc_ket_qua_hien_tai() else { return };
        match self.license.bat_dau(ChucNang::XoaGhim) {
            Ok(lan) => self.bat_dau(lan, viec, ra),
            Err(qd) => {
                self.hop_license = Some(crate::hop_license::HopLicense::moi(qd));
                self.cho_chay = Some(tat_ca);
            }
        }
    }

    fn bat_dau(&mut self, lan: LanChay, viec: Vec<usize>, ra: PathBuf) {
        self.ghi_ngay();
        let cap: Vec<(PathBuf, PathBuf)> = viec.iter().map(|&i| self.ds[i].clone()).collect();
        let (chung, tc, xuat) = (self.bo.tm.vung.clone(), self.bo.tm.tuy_chon, self.bo.tm.xuat);
        let chay_ch = crate::xoa_ghim::xuat::TuyChonChay { lo_10: self.cfg.lo_10 };
        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let dung_nen = dung.clone();
        std::thread::spawn(move || {
            let gui = std::sync::Mutex::new(gui);
            lo::chay_voi(crate::tach_vb::kho::chung(), crate::xoa_ghim::kho::chung(), &cap, &chung, &tc, &xuat, chay_ch, &dung_nen, |i, kq| {
                let _ = gui.lock().unwrap().send((i, kq));
            });
        });
        self.da_chay = None;
        self.chay = Some(DangChay { nhan, dung, lan, viec, xong: 0, ok: 0, loi: Vec::new(), trang: 0, xu_ly: 0, dpi: 0, bat_dau: Instant::now(), ra });
    }

    pub(super) fn nhan_chay(&mut self) {
        let Some(c) = &mut self.chay else { return };
        let mut het = false;
        loop {
            match c.nhan.try_recv() {
                Ok((i, kq)) => {
                    c.xong += 1;
                    match kq {
                        Ok(k) => {
                            c.ok += 1;
                            c.trang += k.so_trang as u64;
                            c.xu_ly += k.trang_xu_ly.len() as u64;
                            c.dpi = c.dpi.max(k.dpi);
                        }
                        Err(e) if e == DA_DUNG => {}
                        Err(e) => {
                            let ten = self.ds.get(c.viec[i]).and_then(|(n, _)| n.file_name()).map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
                            c.loi.push((ten, e));
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    het = true;
                    break;
                }
            }
        }
        if !het {
            return;
        }
        let c = self.chay.take().expect("đang chạy");
        // 1 lượt = 1 lần chạy có ≥ 1 file ra; không ra file nào → hoàn lượt
        self.license.ket_thuc(&c.lan, c.ok > 0);
        let thoi_gian = c.bat_dau.elapsed();
        let co_telegram = c.ok > 0 && {
            let (tong, ok, loi, trang, xu_ly, so_vung, dpi) = (c.viec.len(), c.ok, c.loi.len(), c.trang, c.xu_ly, self.bo.tm.vung.len(), if self.bo.tm.xuat.dpi == 0 { 0 } else { c.dpi });
            self.license.thong_ke("thong_ke_xoa_ghim", move |_| crate::tram::so_lieu_xoa_ghim(tong, ok, loi, trang, xu_ly, so_vung, dpi, false, thoi_gian))
        };
        self.so_da_xoa = Some((c.ok, c.viec.len()));
        self.da_chay = Some(DaChay { tong: c.viec.len(), ok: c.ok, da_dung: c.dung.load(Ordering::Relaxed), loi: c.loi, trang: c.trang, xu_ly: c.xu_ly, thoi_gian, ra: c.ra, co_telegram });
        self.doc_trang_thai();
    }

    pub(super) fn cac_hop(&mut self, ctx: &egui::Context) {
        self.hop_dang_chay(ctx);
        self.hop_da_chay(ctx);
        if let Some(h) = &mut self.hop_license {
            use crate::hop_license::KetQuaHop;
            match h.ve(ctx, &self.license) {
                KetQuaHop::Mo => {}
                KetQuaHop::Dong => {
                    self.hop_license = None;
                    self.cho_chay = None;
                }
                KetQuaHop::DuocChay => {
                    self.hop_license = None;
                    if let Some(tat_ca) = self.cho_chay.take() {
                        self.bam_chay(tat_ca);
                    }
                }
            }
        }
    }

    fn hop_dang_chay(&mut self, ctx: &egui::Context) {
        let Some(c) = &self.chay else { return };
        ctx.request_repaint_after(Duration::from_millis(100));
        let tong = c.viec.len();
        let phan = c.xong as f32 / tong.max(1) as f32;
        let chu_thanh = format!("{}/{} file · {}%", c.xong, tong, (phan * 100.0) as u32);
        let tg = c.bat_dau.elapsed();
        let con = if c.xong > 0 && c.xong < tong { format!(" · còn ~{}", crate::gui_tach::gio_phut_giay(tg.mul_f64((tong - c.xong) as f64 / c.xong as f64))) } else { String::new() };
        let thoi_gian = format!("Đã chạy {}{con}", crate::gui_tach::gio_phut_giay(tg));
        let ra = c.ra.display().to_string();
        let trang = format!("{} trang · đã xoá ghim {} trang", so_nguyen(c.trang as usize), so_nguyen(c.xu_ly as usize));
        let ten = self.thu_muc.as_ref().and_then(|p| p.file_name()).map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
        let dung = c.dung.clone();
        let dang_dung = dung.load(Ordering::Relaxed);
        khung_hop::hop(ctx, "xg_dang_chay", 480.0, khung_hop::tieu_de_dam("Đang xoá ghim ", &ten), |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 18, bottom: 6 }).show(ui, |ui| {
                let thanh = ui.add(egui::ProgressBar::new(phan).desired_width(ui.available_width()).desired_height(26.0).fill(crate::gui::XAM_TIEN_DO).corner_radius(0));
                ui.painter().rect_stroke(thanh.rect, 0.0, egui::Stroke::new(1.0, crate::gui::VIEN_NHAT), egui::StrokeKind::Inside);
                ui.painter().text(thanh.rect.center(), egui::Align2::CENTER_CENTER, chu_thanh, egui::FontId::proportional(12.0), CHU);
            });
            khung_hop::bang_gon_rong(ui, 110.0, &[("Đã làm", &trang, true, 12.0), ("Ghi ra", &ra, false, 11.5), ("Thời gian", &thoi_gian, false, 12.0)]);
            khung_hop::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if dang_dung {
                        ui.label(egui::RichText::new("Đang dừng — đợi file đang làm…").color(CHU_PHU));
                    } else if ui.add(khung_hop::nut_chan_mau("Dừng", DO)).on_hover_text("Dừng sau trang đang làm; file chưa xong không ghi").clicked() {
                        dung.store(true, Ordering::Relaxed);
                    }
                });
            });
        });
    }

    fn hop_da_chay(&mut self, ctx: &egui::Context) {
        let Some(x) = &self.da_chay else { return };
        let (mut dong, mut mo, mut tach) = (false, false, false);
        let ten = self.thu_muc.as_ref().and_then(|p| p.file_name()).map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
        let file = format!("{} / {} file", so_nguyen(x.ok), so_nguyen(x.tong));
        let trang = format!("{} trang · đã xoá ghim {} trang", so_nguyen(x.trang as usize), so_nguyen(x.xu_ly as usize));
        let loi = x.loi.len().to_string();
        let ra = x.ra.display().to_string();
        let thoi_gian = crate::gui::thoi_gian_phut_giay(x.thoi_gian);
        let tieu_de = khung_hop::tieu_de_dam(if x.da_dung { "Đã dừng xoá ghim " } else { "Đã xoá ghim " }, &ten);
        let ph = khung_hop::hop(ctx, "xg_da_chay", 420.0, tieu_de, |ui| {
            khung_hop::bang_gon(ui, &[("File ra", &file, true, 12.0), ("Trang", &trang, true, 12.0), ("Lỗi", &loi, true, 12.0), ("Ghi ra", &ra, false, 11.5), ("Thời gian", &thoi_gian, true, 12.0)]);
            for (f, l) in x.loi.iter().take(5) {
                khung_hop::ghi_gon(ui, &format!("{f}: {l}"), DO);
            }
            if x.co_telegram {
                khung_hop::ghi_gon(ui, "Thông tin xoá ghim đã được ghi lại", CHU_PHU);
            }
            khung_hop::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    dong = ui.add(khung_hop::nut_chan("Đóng")).clicked();
                    tach = x.ok > 0 && ui.add(khung_hop::nut_chan("Mở ở Tách")).on_hover_text("Mở thư mục kết quả ở màn Tách văn bản").clicked();
                    mo = x.ok > 0 && ui.add(khung_hop::nut_chan("Mở thư mục")).clicked();
                });
            });
        });
        if mo {
            mo_bang_he_dieu_hanh(&x.ra);
        }
        if tach {
            self.mo_o_tach = Some(x.ra.clone());
        }
        if dong || tach || ph.should_close() {
            self.da_chay = None;
        }
    }
}
