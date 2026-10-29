//! Dòng Thông tin + nút Tách ở chân màn Tách: kiểm trang Nghi còn chờ duyệt → nhãn chưa lưu → tách nền
//! (phase 4), ghi `ketqua_tach.txt`, popup "Đã tách xong", gửi thống kê Telegram.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Margin, RichText};

use super::bang_trang::{BangTrang, Loc};
use super::van_ban::{self, TongVanBan};
use super::{ManTach, Tab};
use crate::cau_hinh::{CauHinh, LuuNhan};
use crate::dem_pdf::QuyTac;
use crate::gui::{mo_bang_he_dieu_hanh, nut, nut_mau, so_nguyen, so_thuc, thoi_gian_phut_giay, ChuDam, CHU, CHU_PHU, DO, XANH_LA};
use crate::tach_vb::nhan::{Nhan, Xoay};
use crate::tach_vb::phien::{thu_muc_an, PhienPhanTich};
use crate::tach_vb::chi_so_trang::phan_loai;
use crate::tach_vb::tach::{self, HoSoTach, KetQuaTach};
use crate::tach_vb::thong_ke_nguong::{self, MauTrang, MauXoay, ThongKeNguong};
use crate::telegram;

pub(super) struct DangTach {
    nhan: Receiver<Result<KetQuaTach, String>>,
    pub(super) xong: Arc<AtomicUsize>,
    pub(super) tong: usize,
    dung: Arc<AtomicBool>,
    bat_dau: Instant,
    ra: PathBuf,
    so_lieu: telegram::SoLieuTach,
    /// Thống kê ngưỡng trắng chụp lúc bấm Tách (None khi tắt quét trắng).
    thong_ke: Option<ThongKeNguong>,
    nguong: (f32, f32),
}

pub(super) struct XongTach {
    ten_thu_muc: String,
    so_lieu: telegram::SoLieuTach,
    ra: PathBuf,
    log: PathBuf,
    thoi_gian: Duration,
    da_dung: bool,
    khong_xoa: Vec<PathBuf>,
    loi: Vec<String>,
    co_telegram: bool,
}

/// Hồ sơ cần tách từ bảng trang hiện tại: nhãn dùng (khác nội dung) + góc xoay ≠ 0.
pub fn ho_so_tach(phien: &PhienPhanTich, b: &BangTrang) -> Vec<HoSoTach> {
    let mut ds: Vec<HoSoTach> = phien.ho_so.iter().map(|h| HoSoTach { pdf: h.pdf.clone(), nhan: Default::default(), xoay: Default::default() }).collect();
    for t in &b.trang {
        let hs = &mut ds[t.ho_so as usize];
        let n = t.nhan();
        if n != Nhan::NoiDung {
            hs.nhan.insert(t.so as usize, n);
        }
        if t.sua.goc != 0 {
            hs.xoay.insert(t.so as usize, Xoay { goc: t.sua.goc, xac_nhan: false });
        }
    }
    ds
}

/// Thống kê ngưỡng của lô: nhãn cuối (sau duyệt) của mọi trang có chỉ số trắng + trang đã được đề xuất xoay.
pub fn thong_ke_lo(phien: &PhienPhanTich, b: &BangTrang, cfg: &CauHinh) -> Option<ThongKeNguong> {
    if !cfg.quet_trang {
        return None;
    }
    let ch = cfg.cau_hinh_phan_tich();
    let (mut trang, mut xoay) = (Vec::new(), Vec::new());
    for t in &b.trang {
        let Some(kq) = phien.ho_so[t.ho_so as usize].trang[t.so as usize - 1].ket_qua.as_ref() else { continue };
        if let Some(cs) = kq.chi_so {
            trang.push(MauTrang { v: cs.gia_tri(), tu_dong: phan_loai(cs, ch.nguong_trang, ch.nguong_nghi), nhan_cuoi: t.nhan() });
        }
        if let Some(g) = t.tu_dong.and_then(|td| td.nghi_xoay) {
            xoay.push(MauXoay { de_xuat: g, goc: t.sua.goc, xac_nhan: t.sua.co & super::bang_trang::CO_XAC_NHAN != 0 });
        }
    }
    Some(thong_ke_nguong::thong_ke(&trang, &xoay))
}

/// Gửi thống kê tách: tin chữ (+ 2 dòng ngưỡng), file CSV histogram, 1 hàng tab "nguong" Google Sheet. Luồng nền, im lặng khi lỗi.
fn gui_thong_ke(thu_muc: String, so_lieu: telegram::SoLieuTach, thoi_gian: Duration, thong_ke: Option<ThongKeNguong>, nguong: (f32, f32)) -> bool {
    let ch = telegram::doc_cau_hinh().filter(|_| !cfg!(test));
    let sheet = crate::google_sheet::co_khoa() && !cfg!(test);
    if ch.is_none() && !sheet {
        return false;
    }
    std::thread::spawn(move || {
        let bay_gio = chrono::Local::now();
        let ten = Path::new(thu_muc.trim_end_matches(['/', '\\'])).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(ch) = &ch {
            let mut tin = telegram::noi_dung_tach(&thu_muc, &so_lieu, thoi_gian);
            if let Some(tk) = &thong_ke {
                tin += &telegram::dong_nguong(tk, nguong.0, nguong.1);
            }
            let _ = telegram::gui_tin(ch, &tin);
        }
        let Some(tk) = thong_ke else { return };
        let dau = [bay_gio.format("%Y-%m-%d %H:%M:%S").to_string(), telegram::ten_pc(), ten.clone(), env!("CARGO_PKG_VERSION").to_string(), format!("{:.2}", nguong.0), format!("{:.2}", nguong.1)];
        if let Some(ch) = &ch {
            let csv = thong_ke_nguong::csv(&tk, &thong_ke_nguong::COT_SHEET[..6].iter().copied().zip(dau.iter().cloned()).collect::<Vec<_>>());
            let _ = telegram::gui_file(ch, &format!("nguong_{ten}_{}.csv", bay_gio.format("%Y%m%d-%H%M%S")), csv.as_bytes());
        }
        if sheet {
            let _ = crate::google_sheet::ghi_dong("nguong", &thong_ke_nguong::COT_SHEET, thong_ke_nguong::dong_sheet(&tk, dau));
        }
    });
    true
}

impl ManTach {
    /// Tổng cho dòng Thông tin (tính lại khi nhãn hoặc quy tắc quy A4 đổi).
    pub(super) fn tong_thong_tin(&mut self, quy_tac: &QuyTac) -> Option<TongVanBan> {
        let b = self.bang.as_ref()?;
        match &self.tong_tt {
            Some((pb, q, t)) if *pb == self.khoa_ds_vb.unwrap_or(u64::MAX) && q == quy_tac => Some(*t),
            _ => {
                let t = van_ban::tong(b, &self.ds_vb, quy_tac);
                self.tong_tt = Some((self.khoa_ds_vb.unwrap_or(u64::MAX), *quy_tac, t));
                Some(t)
            }
        }
    }

    pub(super) fn dang_tach(&self) -> bool {
        self.tach.is_some()
    }

    /// `Thông tin: Hồ sơ · Văn bản · Số trang · Quy A4` + nút Tách (đang tách → Dừng).
    pub(super) fn dong_thong_tin_tach(&mut self, ui: &mut egui::Ui, cfg: &CauHinh) {
        let tong = self.tong_thong_tin(&cfg.quy_tac());
        let so_hs = self.bang.as_ref().map_or(0, |b| b.ho_so.len());
        let (nghi_trang, nghi_xoay) = self.bang.as_ref().map_or((0, 0), |b| b.con_nghi());
        let ra = tach::thu_muc_ra(Path::new(self.thu_muc.trim()), (!cfg.thu_muc_ra.trim().is_empty()).then(|| Path::new(cfg.thu_muc_ra.trim())));
        ui.horizontal(|ui| {
            if let Some(t) = tong {
                let mut job = egui::text::LayoutJob::default();
                let (thuong, dam) = (super::chu_dong(CHU_PHU, false), super::chu_dong(CHU, true));
                for (i, (ten, gia_tri)) in [("Hồ sơ", so_nguyen(so_hs)), ("Văn bản", so_nguyen(t.so_van_ban)), ("Số trang", so_nguyen(t.so_trang)), ("Quy A4", so_thuc(t.quy_a4))].into_iter().enumerate() {
                    job.append(&format!("{}{ten}: ", if i == 0 { "Thông tin: " } else { "; " }), 0.0, thuong.clone());
                    job.append(&gia_tri, 0.0, dam.clone());
                }
                ui.label(job);
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if self.tach.is_some() {
                    if ui.add(nut_mau("Dừng", DO)).on_hover_text("Dừng tách (hồ sơ đang làm vẫn xong)").clicked() {
                        if let Some(t) = &self.tach {
                            t.dung.store(true, Ordering::Relaxed);
                        }
                    }
                    return;
                }
                let mut goi_y = match &ra {
                    Ok(p) => format!("Ghi ra: {} (giữ cấu trúc thư mục gốc)", p.display()),
                    Err(e) => e.clone(),
                };
                if nghi_trang + nghi_xoay > 0 {
                    goi_y += &format!(" · Phải duyệt hết Nghi trắng ({nghi_trang}) và Nghi xoay ({nghi_xoay}) trước khi tách");
                }
                let bat = self.da_phan_tich() && self.chay.is_none() && ra.is_ok();
                let nut = nut_mau("Tách", if bat { XANH_LA } else { Color32::from_rgb(0xB8, 0xB8, 0xB8) }).min_size(egui::vec2(110.0, 28.0));
                if ui.add_enabled(bat, nut).on_hover_text(&goi_y).on_disabled_hover_text(if self.da_phan_tich() { goi_y.as_str() } else { "Phân tích thư mục trước" }).clicked() {
                    self.bam_tach(cfg);
                }
            });
        });
    }

    /// Thứ tự kiểm: còn trang Nghi → hộp "Chưa tách được"; nhãn chưa lưu → hỏi Lưu / Bỏ / Huỷ; rồi tách.
    pub(super) fn bam_tach(&mut self, cfg: &CauHinh) {
        let Some(b) = &self.bang else { return };
        let (nt, nx) = b.con_nghi();
        if nt + nx > 0 {
            self.chua_tach = Some((nt, nx));
            return;
        }
        if b.chua_luu && cfg.luu_nhan == LuuNhan::BangTay {
            self.hoi_luu_tach = true;
            return;
        }
        self.bat_dau_tach(cfg);
    }

    fn bat_dau_tach(&mut self, cfg: &CauHinh) {
        let (Some(phien), Some(b)) = (&self.phien, &self.bang) else { return };
        let nguon = PathBuf::from(self.thu_muc.trim());
        let cha = cfg.thu_muc_ra.trim();
        let ra = match tach::thu_muc_ra(&nguon, (!cha.is_empty()).then(|| Path::new(cha))) {
            Ok(r) => r,
            Err(e) => {
                self.trang_thai = e;
                return;
            }
        };
        let ho_so = ho_so_tach(phien, b);
        let so_lieu = telegram::SoLieuTach { ho_so: ho_so.len(), bo_index: b.tong.index as usize, bo_trang: b.tong.trang as usize, split: b.tong.split as usize, ..Default::default() };
        let thong_ke = thong_ke_lo(phien, b, cfg);
        let nguong = (cfg.nguong_trang, cfg.nguong_nghi);
        let (gui, nhan) = mpsc::channel();
        let (xong, dung) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(false)));
        let (xong2, dung2, ra2, mau) = (xong.clone(), dung.clone(), ra.clone(), cfg.mau_ten.clone());
        let tong = ho_so.len();
        std::thread::spawn(move || {
            let kq = tach::tach_thu_muc(&nguon, &ra2, &ho_so, &mau, &dung2, &|n, _| xong2.store(n, Ordering::Relaxed));
            let _ = gui.send(kq);
        });
        self.tach = Some(DangTach { nhan, xong, tong, dung, bat_dau: Instant::now(), ra, so_lieu, thong_ke, nguong });
    }

    /// Nhận kết quả tách: ghi log, gửi thống kê, mở popup.
    pub(super) fn nhan_ket_qua_tach(&mut self) {
        let Some(t) = &self.tach else { return };
        let kq = match t.nhan.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(kq) => kq,
            Err(TryRecvError::Disconnected) => Err("luồng tách dừng bất thường".into()),
        };
        let t = self.tach.take().unwrap();
        let kq = match kq {
            Ok(k) => k,
            Err(e) => {
                self.trang_thai = format!("Không tách được: {e}");
                return;
            }
        };
        let log = thu_muc_an(Path::new(self.thu_muc.trim())).join("ketqua_tach.txt");
        let mut loi: Vec<String> = kq.loi.iter().map(|(p, e)| format!("{}: {e}", p.display())).collect();
        let noi_dung = tach::noi_dung_log(&kq.dong) + &tach::dong_log_khong_xoa(&kq.khong_xoa);
        if let Err(e) = std::fs::create_dir_all(log.parent().unwrap()).and_then(|_| std::fs::write(&log, noi_dung)) {
            loi.push(format!("Không ghi được log {}: {e}", log.display()));
        }
        let mut so_lieu = t.so_lieu;
        so_lieu.van_ban = kq.dong.len();
        so_lieu.so_trang = kq.dong.iter().map(|d| d.so_trang).sum();
        so_lieu.sao_nguyen = kq.sao_nguyen;
        so_lieu.ho_so_loi = kq.loi.len();
        let thoi_gian = t.bat_dau.elapsed();
        // Test không gửi tin thật (cấu hình Telegram được nhúng sẵn trong file chạy)
        let co_telegram = gui_thong_ke(self.thu_muc.trim().to_string(), so_lieu.clone(), thoi_gian, t.thong_ke, t.nguong);
        let ten_thu_muc = Path::new(self.thu_muc.trim().trim_end_matches(['/', '\\'])).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        self.xong_tach = Some(XongTach { ten_thu_muc, so_lieu, ra: t.ra, log, thoi_gian, da_dung: t.dung.load(Ordering::Relaxed), khong_xoa: kq.khong_xoa, loi, co_telegram });
    }

    /// Hộp "Chưa tách được" (còn trang Nghi) + hỏi lưu trước khi tách + popup "Đã tách xong".
    pub(super) fn hop_tach(&mut self, ctx: &egui::Context, cfg: &CauHinh) {
        if let Some((nt, nx)) = self.chua_tach {
            let mut chon: Option<Option<Loc>> = None;
            egui::Modal::new(egui::Id::unique("chua_tach")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(RichText::new("Chưa tách được").dam());
                ui.add_space(4.0);
                ui.label(format!(
                    "Còn {nt} trang Nghi trắng và {nx} trang Nghi xoay chưa duyệt. Duyệt hết (Nghi trắng: Enter / B / C / I · Nghi xoay: Xác nhận và tiếp) rồi bấm Tách lại."
                ));
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if nt > 0 && ui.add(nut_mau("Duyệt Nghi trắng", XANH_LA)).clicked() {
                        chon = Some(Some(Loc::NghiTrang));
                    }
                    if nx > 0 && ui.add(nut("Duyệt Nghi xoay")).clicked() {
                        chon = Some(Some(Loc::NghiXoay));
                    }
                    if ui.add(nut("Đóng")).clicked() {
                        chon = Some(None);
                    }
                });
            });
            if let Some(c) = chon {
                self.chua_tach = None;
                if let Some(l) = c {
                    self.mo_duyet(l);
                }
            }
        }
        if self.hoi_luu_tach {
            let mut chon = None;
            egui::Modal::new(egui::Id::unique("hoi_luu_tach")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(RichText::new("Nhãn chưa lưu").dam());
                ui.add_space(4.0);
                ui.label("Có thay đổi nhãn chưa ghi vào index_and_blank.txt. Lưu trước khi tách?");
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(nut_mau("Lưu", XANH_LA)).clicked() {
                        chon = Some(1);
                    }
                    if ui.add(nut("Bỏ")).clicked() {
                        chon = Some(2);
                    }
                    if ui.add(nut("Huỷ")).clicked() {
                        chon = Some(0);
                    }
                });
            });
            if let Some(c) = chon {
                self.hoi_luu_tach = false;
                if c == 1 {
                    self.luu_nhan();
                }
                if c > 0 {
                    self.bat_dau_tach(cfg);
                }
            }
        }
        self.popup_tach_xong(ctx);
    }

    /// Về tab Hồ sơ, bỏ lọc hồ sơ, bật bộ lọc Nghi tương ứng, chọn sẵn trang đầu.
    pub(super) fn mo_duyet(&mut self, l: Loc) {
        self.tab = Tab::HoSo;
        self.ho_so_loc = None;
        self.loc = l;
        self.loc_menu = l;
        self.chon.bo_het();
        self.khoa_thu_tu = None;
        self.cap_nhat_thu_tu();
        if let Some(&k) = self.thu_tu.first() {
            self.chon.chon_rieng(k);
            self.cuon_toi = Some(k);
        }
    }

    fn popup_tach_xong(&mut self, ctx: &egui::Context) {
        let Some(x) = &self.xong_tach else { return };
        let (mut dong, mut mo_ra, mut mo_log) = (false, false, false);
        let khung = egui::Frame::popup(&ctx.global_style()).fill(Color32::WHITE).inner_margin(Margin::same(18));
        let ph = egui::Modal::new(egui::Id::unique("tach_xong")).frame(khung).show(ctx, |ui| {
            ui.set_width(440.0);
            let mut tieu_de = egui::text::LayoutJob::default();
            tieu_de.append(if x.da_dung { "Đã dừng tách " } else { "Đã tách xong " }, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(14.0), color: CHU, ..Default::default() });
            tieu_de.append(&x.ten_thu_muc, 0.0, egui::TextFormat { font_id: egui::FontId::new(14.0, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), color: CHU, ..Default::default() });
            ui.add(egui::Label::new(tieu_de).truncate());
            ui.add_space(10.0);
            let s = &x.so_lieu;
            egui::Grid::new("tach_xong_tt").num_columns(2).spacing([18.0, 8.0]).show(ui, |ui| {
                let mut dong_tt = |ten: &str, gia_tri: String| {
                    ui.label(RichText::new(ten).color(CHU_PHU));
                    ui.label(RichText::new(gia_tri).dam());
                    ui.end_row();
                };
                dong_tt("Hồ sơ:", format!("{}{}", so_nguyen(s.ho_so), if s.ho_so_loi > 0 { format!(" ({} lỗi)", s.ho_so_loi) } else { String::new() }));
                dong_tt("Văn bản:", format!("{} ({} trang)", so_nguyen(s.van_ban), so_nguyen(s.so_trang)));
                dong_tt("Trang bỏ:", format!("{} index, {} trắng", so_nguyen(s.bo_index), so_nguyen(s.bo_trang)));
                dong_tt("Sao nguyên:", format!("{} hồ sơ", so_nguyen(s.sao_nguyen)));
                dong_tt("Ghi ra:", x.ra.display().to_string());
                dong_tt("Thời gian:", thoi_gian_phut_giay(x.thoi_gian));
            });
            if !x.khong_xoa.is_empty() {
                ui.add_space(8.0);
                let ds: Vec<String> = x.khong_xoa.iter().take(3).map(|p| p.display().to_string()).collect();
                let them = if x.khong_xoa.len() > 3 { format!(" … (+{})", x.khong_xoa.len() - 3) } else { String::new() };
                ui.colored_label(DO, format!("Không xoá được {} file cũ (có thể đang mở): {}{them} — đóng file rồi xoá tay hoặc Tách lại.", x.khong_xoa.len(), ds.join("; ")));
            }
            for l in x.loi.iter().take(5) {
                ui.colored_label(DO, l);
            }
            if x.co_telegram {
                ui.add_space(8.0);
                ui.label(RichText::new("Thông tin tách thư mục đã được ghi lại").small().color(CHU_PHU));
            }
            ui.add_space(10.0);
            let rong_nut = egui::vec2(104.0, 28.0);
            ui.columns(3, |c| {
                c[0].vertical_centered(|ui| mo_ra = ui.add(nut("Mở thư mục").min_size(rong_nut)).clicked());
                c[1].vertical_centered(|ui| mo_log = ui.add(nut("Mở log").min_size(rong_nut)).clicked());
                c[2].vertical_centered(|ui| dong = ui.add(nut("Đóng").min_size(rong_nut)).clicked());
            });
        });
        if mo_ra {
            mo_bang_he_dieu_hanh(&x.ra);
        }
        if mo_log {
            mo_bang_he_dieu_hanh(&x.log);
        }
        if dong || ph.should_close() {
            self.xong_tach = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_tach::bang_trang::Nhom;

    fn cho_xong(man: &mut ManTach, ch: &crate::tach_vb::phan_tich::CauHinhPhanTich) {
        let bat_dau = Instant::now();
        while man.dang_chay() {
            assert!(bat_dau.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(20));
            man.nhan_su_kien(ch, None);
            man.nhan_ket_qua_tach();
        }
    }

    /// Còn trang Nghi → không ghi file nào, hộp đúng số; Duyệt → đúng bộ lọc, chọn sẵn trang đầu;
    /// Xoá trang Nghi → hết chặn; tách xong: file part + log + số liệu popup đúng.
    #[test]
    fn chan_khi_con_nghi_roi_tach_ghi_dung_file() {
        let goc = std::env::temp_dir().join(format!("dema4_nut_tach_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        let nguon = goc.join("nguon");
        std::fs::create_dir_all(&nguon).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&nguon.join("a.pdf"), 5);
        let cfg = CauHinh { quet_ma: false, quet_xoay: false, ..Default::default() };
        let ch = cfg.cau_hinh_phan_tich();
        let mut man = ManTach { thu_muc: nguon.display().to_string(), ..Default::default() };
        man.bam_phan_tich(&ch);
        cho_xong(&mut man, &ch);
        // Trang chữ nhỏ → Trắng; đặt tay: 1,3,4,5 nội dung, 2 Index; trang 4 giả làm Nghi trắng
        let tc = Nhom::Loc(Loc::TatCa);
        let b = man.bang.as_mut().unwrap();
        b.gan(tc, &[0, 2, 4], &[], Nhan::NoiDung);
        b.gan(tc, &[1], &[], Nhan::Index);
        let mut td = b.trang[3].tu_dong.unwrap();
        td.nhan = Nhan::NoiDung;
        td.nghi_trang = true;
        b.dat_tu_dong(3, td);
        man.bam_tach(&cfg);
        assert_eq!(man.chua_tach, Some((1, 0)));
        assert!(man.tach.is_none());
        let ra = goc.join("nguon_vanban");
        assert!(!ra.exists(), "không ghi file nào khi còn trang Nghi");
        man.chua_tach = None;
        man.mo_duyet(Loc::NghiTrang);
        assert_eq!((man.tab, man.loc, man.chon.theo_thu_tu(&man.thu_tu.clone())), (Tab::HoSo, Loc::NghiTrang, vec![3]));
        // Xoá trang Nghi khỏi lưới → không còn chặn (trang vẫn nằm trong file tách)
        man.bang.as_mut().unwrap().xoa(tc, &[3], &[]);
        man.bam_tach(&cfg);
        assert!(man.chua_tach.is_none());
        cho_xong(&mut man, &ch);
        let x = man.xong_tach.as_ref().expect("có popup Đã tách xong");
        assert_eq!((x.so_lieu.ho_so, x.so_lieu.van_ban, x.so_lieu.so_trang, x.so_lieu.bo_index), (1, 2, 4, 1));
        assert!(x.khong_xoa.is_empty() && x.loi.is_empty());
        assert!(ra.join("a").join("a_part1.pdf").exists() && ra.join("a").join("a_part2.pdf").exists());
        // Thống kê ngưỡng của lô: 5 trang có chỉ số; máy cho Trắng 4 trang (trang 4 bị giả làm Nghi), người giữ nội dung / Index hết
        let tk = thong_ke_lo(man.phien.as_ref().unwrap(), man.bang.as_ref().unwrap(), &cfg).unwrap();
        assert_eq!(tk.o.iter().map(|x| x.0 + x.1).sum::<u32>(), 5);
        assert_eq!((tk.trang_tu_dong, tk.trang_tu_dong_sua), (5, 5), "phân loại lại theo chỉ số đã lưu");
        let log = std::fs::read_to_string(nguon.join(".sm-dema4/ketqua_tach.txt")).unwrap();
        assert_eq!(log.lines().count(), 3, "{log}");
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn hoi_luu_truoc_khi_tach_khi_luu_bang_tay() {
        let goc = std::env::temp_dir().join(format!("dema4_nut_tach_luu_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join("n")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("n/a.pdf"), 2);
        let cfg = CauHinh { quet_ma: false, quet_xoay: false, luu_nhan: LuuNhan::BangTay, ..Default::default() };
        let ch = cfg.cau_hinh_phan_tich();
        let mut man = ManTach { thu_muc: goc.join("n").display().to_string(), ..Default::default() };
        man.bam_phan_tich(&ch);
        cho_xong(&mut man, &ch);
        man.bang.as_mut().unwrap().gan(Nhom::Loc(Loc::TatCa), &[0, 1], &[], Nhan::NoiDung);
        man.bam_tach(&cfg);
        assert!(man.hoi_luu_tach && man.tach.is_none());
        let _ = std::fs::remove_dir_all(&goc);
    }
}
