//! Sinh mục lục: kiểm lượt license `muc_luc` → chạy `sinh::chay` ở luồng nền → popup "Đang sinh" (tiến độ, Dừng)
//! → popup "Đã sinh" (Mở thư mục) + gửi thống kê lên trạm (tin 📑 SINH MỤC LỤC).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Align};

use super::{ten_mau, DaSinh, DangSinh, ManMucLuc};
use crate::cau_hinh::CauHinhMucLuc;
use crate::gui::{mo_bang_he_dieu_hanh, so_nguyen, CHU, CHU_PHU, DO};
use crate::gui_tach::khung_hop;
use crate::muc_luc::sinh;

impl ManMucLuc {
    /// Thư mục ra: Cấu hình (gõ đường dẫn) hoặc mặc định `.sm-dema4/muc_luc/<tên file dữ liệu>` cạnh file chạy.
    pub fn thu_muc_ra(&self, cfg: &CauHinhMucLuc) -> PathBuf {
        let rieng = cfg.thu_muc_ra.trim();
        if rieng.is_empty() {
            sinh::thu_muc_mac_dinh(crate::tach_vb::du_lieu_an::goc_du_lieu(), std::path::Path::new(self.file.trim()))
        } else {
            PathBuf::from(rieng)
        }
    }

    pub(super) fn bam_sinh(&mut self, cfg: &CauHinhMucLuc) {
        if !self.co_the_sinh() {
            return;
        }
        match self.license.bat_dau(crate::license::ChucNang::MucLuc) {
            Ok(lan) => self.bat_dau_sinh(lan, cfg),
            Err(qd) => self.hop_license = Some(crate::hop_license::HopLicense::moi(qd)),
        }
    }

    fn bat_dau_sinh(&mut self, lan: crate::license::LanChay, cfg: &CauHinhMucLuc) {
        let (Some(m), Some(d)) = (&self.mau, &self.du_lieu) else {
            self.license.ket_thuc(&lan, false);
            return;
        };
        let thu_muc = self.thu_muc_ra(cfg);
        let yc = sinh::YeuCau {
            mau: m.db.clone(),
            bang: d.bang.clone(),
            ho_so: Arc::new(self.kq.ho_so.clone()),
            rb: self.rb.clone(),
            thu_muc_ra: thu_muc.clone(),
            mau_ten: cfg.mau_ten.clone(),
            ghi_de: cfg.ghi_de,
        };
        let (gui, nhan) = mpsc::channel();
        let dung = Arc::new(AtomicBool::new(false));
        let dung_nen = dung.clone();
        std::thread::spawn(move || {
            sinh::chay(&yc, &dung_nen, &gui);
        });
        self.da_sinh = None;
        self.sinh = Some(DangSinh { nhan, dung, lan, i: 0, ten: "…".into(), bat_dau: Instant::now(), tong: self.kq.ho_so.len(), van_ban: self.so_van_ban(), thu_muc });
    }

    pub(super) fn nhan_sinh(&mut self) {
        let Some(s) = &mut self.sinh else { return };
        let mut xong = None;
        loop {
            match s.nhan.try_recv() {
                Ok(sinh::SuKien::DangTao { i, ten }) => {
                    s.i = i;
                    s.ten = ten;
                }
                Ok(sinh::SuKien::Xong(tk)) => {
                    xong = Some(tk);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    xong = Some(sinh::TongKet { loi: vec![(String::new(), "luồng sinh dừng bất thường".into())], ..Default::default() });
                    break;
                }
            }
        }
        let Some(tk) = xong else { return };
        let s = self.sinh.take().expect("đang sinh");
        // 1 lượt = 1 lần Sinh có ≥ 1 DOCX ra; không ra file nào → hoàn lượt
        self.license.ket_thuc(&s.lan, tk.docx > 0);
        let co_telegram = tk.docx > 0 && {
            let (du_lieu, mau, ho_so, van_ban, tk2) = (self.file.trim().to_string(), ten_mau(&self.mau_chon), s.tong, s.van_ban, tk.clone());
            self.license.thong_ke("thong_ke_muc_luc", move |_| crate::tram::so_lieu_muc_luc(&du_lieu, &mau, ho_so, van_ban, &tk2, 0))
        };
        self.da_sinh = Some(DaSinh { tk, tong: s.tong, van_ban: s.van_ban, thu_muc: s.thu_muc, co_telegram });
    }

    pub(super) fn cac_hop(&mut self, ctx: &egui::Context, cfg: &CauHinhMucLuc) {
        self.hop_ghep(ctx);
        self.hop_dang_sinh(ctx);
        self.hop_da_sinh(ctx);
        if let Some(h) = &mut self.hop_license {
            use crate::hop_license::KetQuaHop;
            match h.ve(ctx, &self.license) {
                KetQuaHop::Mo => {}
                KetQuaHop::Dong => self.hop_license = None,
                KetQuaHop::DuocChay => {
                    self.hop_license = None;
                    self.bam_sinh(cfg);
                }
            }
        }
    }

    fn hop_dang_sinh(&mut self, ctx: &egui::Context) {
        let Some(s) = &self.sinh else { return };
        let xong = (s.i).min(s.tong);
        let phan = xong as f32 / s.tong.max(1) as f32;
        let chu_thanh = format!("{}/{} hồ sơ · {}%", xong, s.tong, (phan * 100.0) as u32);
        let tg = s.bat_dau.elapsed();
        let con = if xong > 0 { format!(" · còn ~{}", crate::gui_tach::gio_phut_giay(tg.mul_f64((s.tong - xong) as f64 / xong as f64))) } else { String::new() };
        let thoi_gian = format!("Đã chạy {}{con}", crate::gui_tach::gio_phut_giay(tg));
        let mau = format!("{} · nhóm theo {}", ten_mau(&self.mau_chon), self.rb.cot_nhom);
        let ra = s.thu_muc.display().to_string();
        let ten_file = std::path::Path::new(self.file.trim()).file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let dung = s.dung.clone();
        khung_hop::hop(ctx, "ml_dang_sinh", 480.0, khung_hop::tieu_de_dam("Đang sinh mục lục ", &ten_file), |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 18, bottom: 6 }).show(ui, |ui| {
                let thanh = ui.add(egui::ProgressBar::new(phan).desired_width(ui.available_width()).desired_height(26.0).fill(crate::gui::XAM_TIEN_DO).corner_radius(0));
                ui.painter().rect_stroke(thanh.rect, 0.0, egui::Stroke::new(1.0, crate::gui::VIEN_NHAT), egui::StrokeKind::Inside);
                ui.painter().text(thanh.rect.center(), egui::Align2::CENTER_CENTER, chu_thanh, egui::FontId::proportional(12.0), CHU);
            });
            khung_hop::bang_gon_rong(ui, 110.0, &[("Đang tạo", &s.ten, true, 12.0), ("Mẫu", &mau, false, 12.0), ("Ghi ra", &ra, false, 11.5), ("Thời gian", &thoi_gian, false, 12.0)]);
            khung_hop::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(khung_hop::nut_chan_mau("Dừng", DO)).on_hover_text("Dừng sau hồ sơ đang làm").clicked() {
                        dung.store(true, Ordering::Relaxed);
                    }
                });
            });
        });
    }

    fn hop_da_sinh(&mut self, ctx: &egui::Context) {
        let Some(x) = &self.da_sinh else { return };
        let (mut dong, mut mo) = (false, false);
        let tk = &x.tk;
        let ten_file = std::path::Path::new(self.file.trim()).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ho_so = so_nguyen(x.tong);
        let van_ban = so_nguyen(x.van_ban);
        let file_ra = format!("{} DOCX{}", so_nguyen(tk.docx), if tk.bo_qua > 0 { format!(" · bỏ qua {} file đã có", so_nguyen(tk.bo_qua)) } else { String::new() });
        let loi = tk.loi.len().to_string();
        let ra = x.thu_muc.display().to_string();
        let thoi_gian = crate::gui::thoi_gian_phut_giay(tk.thoi_gian);
        let tieu_de = khung_hop::tieu_de_dam(if tk.da_dung { "Đã dừng sinh mục lục " } else { "Đã sinh mục lục " }, &ten_file);
        let ph = khung_hop::hop(ctx, "ml_da_sinh", 400.0, tieu_de, |ui| {
            khung_hop::bang_gon(ui, &[("Hồ sơ", &ho_so, true, 12.0), ("Văn bản", &van_ban, true, 12.0), ("File ra", &file_ra, true, 12.0), ("Lỗi", &loi, true, 12.0), ("Ghi ra", &ra, false, 11.5), ("Thời gian", &thoi_gian, true, 12.0)]);
            for (f, l) in tk.loi.iter().take(5) {
                khung_hop::ghi_gon(ui, &format!("{f}: {l}"), DO);
            }
            if x.co_telegram {
                khung_hop::ghi_gon(ui, "Thông tin sinh mục lục đã được ghi lại", CHU_PHU);
            }
            khung_hop::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    dong = ui.add(khung_hop::nut_chan("Đóng")).clicked();
                    mo = ui.add(khung_hop::nut_chan("Mở thư mục")).clicked();
                });
            });
        });
        if mo {
            mo_bang_he_dieu_hanh(&x.thu_muc);
        }
        if dong || ph.should_close() {
            self.da_sinh = None;
        }
    }
}
