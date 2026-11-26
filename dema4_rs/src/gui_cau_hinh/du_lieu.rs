//! Trang **Dữ liệu ứng dụng** của Cấu hình: thư mục dữ liệu (Mở thư mục), Xoá cache phân tích của thư mục Tách đang
//! mở, Dọn dữ liệu cũ — có hỏi lại.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;

use eframe::egui::{self, Color32, Margin, RichText, Sense, Stroke};

use super::{dong, goi_y, the, CHU, CHU_PHU, KE_DONG};
use crate::gui::{ChuDam, DO};
use crate::gui_tach::bieu_tuong::{self as bt, BieuTuong};
use crate::gui_tach::khung_hop as kh;
use crate::tach_vb::cache_trang::{self, KhoaFile};
use crate::tach_vb::kho::{Id, Kho};

/// Việc app phải làm sau khung hình (trang chỉ báo, app làm vì cần màn Tách / kho).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViecDuLieu {
    Khong,
    XoaCache,
    DonCu,
}

/// Hộp hỏi lại đang mở.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hoi {
    XoaCache,
    DonCu,
}

/// Dung lượng cache phân tích của thư mục Tách đang mở.
enum DungLuong {
    /// Màn Tách chưa mở thư mục.
    KhongMo,
    /// Luồng nền đang tính.
    DangTinh(Receiver<u64>),
    /// Số byte cache còn trong kho (0 = chưa có cache).
    Xong(u64),
}

/// Trạng thái trang (tạo khi mở Cấu hình).
pub struct TrangDuLieu {
    thu_muc_du_lieu: PathBuf,
    /// Thư mục đang mở ở màn Tách (để hiện tên + dung lượng cache).
    thu_muc_tach: Option<PathBuf>,
    dung_luong: DungLuong,
    hoi: Option<Hoi>,
}

impl TrangDuLieu {
    /// Tạo trang khi mở Cấu hình.
    /// - `thu_muc_du_lieu`: gốc dữ liệu của app (hiện ở ô Vị trí, nút Mở thư mục mở nó).
    /// - `tach`: thư mục đang mở ở màn Tách kèm id nội dung các PDF của nó trong kho (`ManTach::cache_dang_mo()`),
    ///   `None` nếu chưa mở thư mục. Dung lượng cache của các id đó được tính **một lần** ở luồng nền; xong thì
    ///   `ctx` được báo vẽ lại.
    pub fn moi(ctx: &egui::Context, thu_muc_du_lieu: PathBuf, tach: Option<(PathBuf, Vec<Id>)>) -> Self {
        let (thu_muc_tach, dung_luong) = match tach {
            None => (None, DungLuong::KhongMo),
            Some((thu_muc, ids)) => {
                let (gui, nhan) = mpsc::channel();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let _ = gui.send(dung_luong_cache(crate::tach_vb::kho::chung(), &ids));
                    ctx.request_repaint();
                });
                (Some(thu_muc), DungLuong::DangTinh(nhan))
            }
        };
        Self { thu_muc_du_lieu, thu_muc_tach, dung_luong, hoi: None }
    }

    /// Báo app đã xoá cache của thư mục Tách (sau `ViecDuLieu::XoaCache`): nút Xoá cache mờ lại.
    pub fn da_xoa_cache(&mut self) {
        if self.thu_muc_tach.is_some() {
            self.dung_luong = DungLuong::Xong(0);
        }
    }

    fn nhan_dung_luong(&mut self) {
        let DungLuong::DangTinh(nhan) = &self.dung_luong else { return };
        match nhan.try_recv() {
            Ok(b) => self.dung_luong = DungLuong::Xong(b),
            Err(TryRecvError::Empty) => {}
            // Luồng tính chết giữa chừng: coi như không có số liệu
            Err(TryRecvError::Disconnected) => self.dung_luong = DungLuong::Xong(0),
        }
    }

    /// Vẽ thân trang trong cột nội dung; trả việc người dùng đã xác nhận.
    pub fn ve(&mut self, ui: &mut egui::Ui) -> ViecDuLieu {
        self.nhan_dung_luong();
        if matches!(self.dung_luong, DungLuong::DangTinh(_)) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        the(ui, "Thư mục dữ liệu", None, None, |ui| {
            dong(
                ui,
                false,
                |ui| {
                    ui.label(RichText::new("Vị trí").color(CHU));
                },
                |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let rong_nut = 112.0;
                    let vi_tri = self.thu_muc_du_lieu.display().to_string();
                    ui.add(
                        egui::TextEdit::singleline(&mut vi_tri.as_str())
                            .font(egui::FontId::proportional(12.0))
                            .margin(Margin::symmetric(6, 4))
                            .min_size(egui::vec2(0.0, 24.0))
                            .background_color(Color32::from_rgb(0xF8, 0xF9, 0xFA))
                            .desired_width(ui.available_width() - rong_nut - 6.0),
                    )
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Vị trí thư mục dữ liệu"));
                    if ui.add(egui::Button::new("Mở thư mục").min_size(egui::vec2(rong_nut, 24.0))).on_hover_text("Mở thư mục dữ liệu trong trình quản lý file").clicked() {
                        crate::gui::mo_bang_he_dieu_hanh(&self.thu_muc_du_lieu);
                    }
                },
            );
            goi_y(ui, "Cấu hình, khoá API mã hoá, license, nhãn / cache Tách, kết quả Bóc tách, vùng Xóa ghim, lịch sử gần đây. Cố định theo người dùng — dời file chạy không mất.", None);
        });
        let mut bam = None;
        the(ui, "Dọn dẹp", None, None, |ui| {
            let (co_cache, mo_ta) = self.mo_ta_cache();
            let ly_do_mo = if self.thu_muc_tach.is_some() { "Thư mục đang mở chưa có cache" } else { "Chưa mở thư mục ở màn Tách" };
            dong_don_dep(ui, BieuTuong::ThungRac, DO, "Xoá cache phân tích (Tách)", mo_ta, "Xoá cache…", co_cache.then_some("Xoá kết quả phân tích đã lưu của thư mục đang mở").ok_or(ly_do_mo), || bam = Some(Hoi::XoaCache));
            dong_don_dep(ui, BieuTuong::LamLai, CHU_PHU, "Dọn dữ liệu cũ", vec![("Bỏ nhãn, kết quả phân tích, kết quả bóc tách, vùng Xóa ghim của PDF không mở lại quá 12 tháng, rồi thu gọn kho. Không đụng file PDF và file kết quả.".into(), false)], "Dọn dữ liệu cũ…", Ok("Bỏ phần đã làm của các PDF không mở lại quá 12 tháng"), || bam = Some(Hoi::DonCu));
        });
        if bam.is_some() {
            self.hoi = bam;
        }
        self.hop_hoi(ui.ctx())
    }

    /// Có cache để xoá không + đoạn mô tả dưới tên dòng "Xoá cache phân tích" (đoạn, đậm?).
    fn mo_ta_cache(&self) -> (bool, Vec<(String, bool)>) {
        let Some(thu_muc) = &self.thu_muc_tach else {
            return (false, vec![("Chưa mở thư mục nào ở màn Tách.".into(), false)]);
        };
        let dau = ("Thư mục đang mở ở Tách: ".to_string(), false);
        let ten = (thu_muc.display().to_string(), true);
        match &self.dung_luong {
            DungLuong::KhongMo => (false, vec![("Chưa mở thư mục nào ở màn Tách.".into(), false)]),
            DungLuong::DangTinh(_) => (false, vec![dau, ten, (" · đang tính…".into(), false)]),
            DungLuong::Xong(0) => (false, vec![dau, ten, (" · chưa có cache phân tích.".into(), false)]),
            DungLuong::Xong(b) => (true, vec![dau, ten, (format!(" · {}. Nhãn đã gán không mất; lần Phân tích sau phân tích lại mọi trang.", dinh_dang_dung_luong(*b)), false)]),
        }
    }

    /// Hộp hỏi lại; trả việc khi xác nhận.
    fn hop_hoi(&mut self, ctx: &egui::Context) -> ViecDuLieu {
        let Some(hoi) = self.hoi else { return ViecDuLieu::Khong };
        let mut viec = ViecDuLieu::Khong;
        let mut dong_hop = false;
        let (tieu_de, nut_ok, mau_ok) = match hoi {
            Hoi::XoaCache => ("Xoá cache phân tích?", "Xoá cache", DO),
            Hoi::DonCu => ("Dọn dữ liệu cũ?", "Dọn", crate::gui::XANH_COBALT),
        };
        let phan_hoi = kh::hop(ctx, "hop_du_lieu", 440.0, RichText::new(tieu_de).strong().size(14.0), |ui| {
            egui::Frame::new().inner_margin(Margin { left: 18, right: 18, top: 10, bottom: 16 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                match hoi {
                    Hoi::XoaCache => {
                        let thu_muc = self.thu_muc_tach.as_deref().map_or(String::new(), |p| p.display().to_string());
                        let dung_luong = match self.dung_luong {
                            DungLuong::Xong(b) if b > 0 => format!("{} ", dinh_dang_dung_luong(b)),
                            _ => String::new(),
                        };
                        doan_van(ui, &[(&format!("Xoá {dung_luong}kết quả phân tích của "), false), (&thu_muc, true), (". Nhãn đã gán không mất.", false)], 13.0, CHU);
                    }
                    Hoi::DonCu => {
                        doan_van(ui, &[("Bỏ phần đã làm của các PDF không mở lại quá 12 tháng (chạy ngầm, vài giây với kho lớn).", false)], 13.0, CHU);
                    }
                }
            });
            kh::chan(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if ui.add(kh::nut_chan_mau(nut_ok, mau_ok)).clicked() {
                        viec = match hoi {
                            Hoi::XoaCache => ViecDuLieu::XoaCache,
                            Hoi::DonCu => ViecDuLieu::DonCu,
                        };
                        dong_hop = true;
                    }
                    if ui.add(kh::nut_chan("Huỷ")).clicked() {
                        dong_hop = true;
                    }
                });
            });
        });
        if dong_hop || phan_hoi.should_close() {
            self.hoi = None;
        }
        viec
    }
}

/// Dòng của thẻ Dọn dẹp: biểu tượng · tên + mô tả (các đoạn, đoạn đậm nếu `true`) · nút bên phải.
/// `nut`: `Ok(gợi ý)` = bấm được, `Err(lý do)` = mờ. `khi_bam` chạy khi bấm.
fn dong_don_dep(ui: &mut egui::Ui, icon: BieuTuong, mau_icon: Color32, ten: &str, mo_ta: Vec<(String, bool)>, nut: &str, bam_duoc: Result<&str, &str>, khi_bam: impl FnOnce()) {
    let y = ui.cursor().top();
    ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, KE_DONG));
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (o, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
        bt::ve(ui.painter(), o.translate(egui::vec2(0.0, 1.0)), icon, mau_icon);
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 160.0).max(120.0));
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(ten).size(12.5).color(CHU));
            let doan: Vec<(&str, bool)> = mo_ta.iter().map(|(t, d)| (t.as_str(), *d)).collect();
            doan_van(ui, &doan, 11.5, CHU_PHU);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let r = ui.add_enabled(bam_duoc.is_ok(), egui::Button::new(nut).min_size(egui::vec2(0.0, 24.0)));
            let r = match bam_duoc {
                Ok(goi_y) => r.on_hover_text(goi_y),
                Err(ly_do) => r.on_disabled_hover_text(ly_do),
            };
            if r.clicked() {
                khi_bam();
            }
        });
    });
    ui.add_space(8.0);
}

/// Đoạn văn tự xuống dòng gồm các đoạn chữ, đoạn `true` in đậm.
fn doan_van(ui: &mut egui::Ui, doan: &[(&str, bool)], co: f32, mau: Color32) {
    let mut job = egui::text::LayoutJob::default();
    for (chu, dam) in doan {
        let r = RichText::new(*chu).size(co).color(mau);
        let r = if *dam { r.dam() } else { r };
        r.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
    }
    ui.add(egui::Label::new(job).wrap());
}

/// "182 MB", "1,5 GB" (dấu phẩy thập phân).
fn dinh_dang_dung_luong(byte: u64) -> String {
    const DON_VI: [&str; 4] = ["B", "KB", "MB", "GB"];
    let (mut v, mut i) = (byte as f64, 0);
    while v >= 1024.0 && i < DON_VI.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 || v >= 100.0 { format!("{:.0} {}", v, DON_VI[i]) } else { format!("{:.1} {}", v, DON_VI[i]).replace('.', ",") }
}

/// Tổng dung lượng (byte, dạng lưu trong kho) kết quả phân tích của `ids`. Mỗi PDF khoá kho riêng một lần, không giữ
/// khoá suốt lượt (màn Tách vẫn dùng kho được trong lúc tính).
fn dung_luong_cache(kho: &Mutex<Kho>, ids: &[Id]) -> u64 {
    ids.iter()
        .map(|&id| {
            let mut k = kho.lock().unwrap_or_else(|e| e.into_inner());
            if !k.co_phan_tich(id) {
                return 0;
            }
            k.phan_tich(id).map_or(0, |trang| cache_trang::ma_hoa(&KhoaFile { duong_dan: String::new(), kich_thuoc: id.1, sua_luc: 0, van_tay: id.0 }, &trang).len() as u64)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tach_vb::cache_trang::TrangCache;
    use egui_kittest::kittest::Queryable;

    fn trang_mau(co_cache: Option<u64>) -> TrangDuLieu {
        let dung_luong = match co_cache {
            Some(b) => DungLuong::Xong(b),
            None => DungLuong::KhongMo,
        };
        TrangDuLieu { thu_muc_du_lieu: PathBuf::from("/du/lieu/sm-dema4"), thu_muc_tach: co_cache.map(|_| PathBuf::from("/ho_so/lo_01")), dung_luong, hoi: None }
    }

    fn harness(trang: TrangDuLieu) -> egui_kittest::Harness<'static, (TrangDuLieu, Vec<ViecDuLieu>)> {
        egui_kittest::Harness::builder().with_size(egui::vec2(900.0, 600.0)).build_ui_state(
            |ui, (trang, viec): &mut (TrangDuLieu, Vec<ViecDuLieu>)| {
                crate::gui::cai_giao_dien(ui.ctx());
                let v = trang.ve(ui);
                if v != ViecDuLieu::Khong {
                    viec.push(v);
                }
            },
            (trang, Vec::new()),
        )
    }

    #[test]
    fn xoa_cache_hoi_lai_roi_moi_bao_viec() {
        let mut h = harness(trang_mau(Some(190_000_000)));
        h.run();
        h.get_by_label("Xoá cache…").click();
        h.run();
        assert!(h.state().1.is_empty(), "mới hỏi, chưa làm");
        h.get_by_label("Xoá cache").click();
        h.run();
        assert_eq!(h.state().1, vec![ViecDuLieu::XoaCache]);
        assert!(h.query_by_label("Huỷ").is_none(), "xác nhận xong hộp đóng");
    }

    #[test]
    fn huy_hop_khong_bao_viec() {
        let mut h = harness(trang_mau(Some(190_000_000)));
        h.run();
        h.get_by_label("Dọn dữ liệu cũ…").click();
        h.run();
        h.get_by_label("Huỷ").click();
        h.run();
        h.get_by_label("Xoá cache…").click();
        h.run();
        h.get_by_label("Huỷ").click();
        h.run();
        assert!(h.state().1.is_empty());
        assert!(h.query_by_label("Huỷ").is_none());
    }

    #[test]
    fn don_cu_hoi_lai_roi_moi_bao_viec_ke_ca_khi_chua_mo_thu_muc() {
        let mut h = harness(trang_mau(None));
        h.run();
        h.get_by_label("Dọn dữ liệu cũ…").click();
        h.run();
        assert!(h.state().1.is_empty());
        h.get_by_label("Dọn").click();
        h.run();
        assert_eq!(h.state().1, vec![ViecDuLieu::DonCu]);
    }

    #[test]
    fn nut_xoa_cache_mo_khi_chua_co_cache_hoac_chua_mo_thu_muc() {
        for co_cache in [None, Some(0)] {
            let mut h = harness(trang_mau(co_cache));
            h.run();
            h.get_by_label("Xoá cache…").click();
            h.run();
            assert!(h.query_by_label("Huỷ").is_none(), "nút mờ không mở hộp hỏi ({co_cache:?})");
            assert!(h.state().1.is_empty());
        }
    }

    #[test]
    fn da_xoa_cache_lam_mo_nut() {
        let mut t = trang_mau(Some(5_000));
        t.da_xoa_cache();
        assert!(!t.mo_ta_cache().0);
    }

    #[test]
    fn dinh_dang_dung_luong_don_vi_va_dau_phay() {
        assert_eq!(dinh_dang_dung_luong(512), "512 B");
        assert_eq!(dinh_dang_dung_luong(1536), "1,5 KB");
        assert_eq!(dinh_dang_dung_luong(190_000_000), "181 MB");
        assert_eq!(dinh_dang_dung_luong(3 * 1024 * 1024 * 1024), "3,0 GB");
    }

    #[test]
    fn dung_luong_cache_chi_tinh_pdf_con_cache() {
        let kho = Mutex::new(Kho::bo_nho());
        {
            let mut k = kho.lock().unwrap();
            k.ghi_phan_tich((1, 10), &vec![TrangCache::default(); 3]);
            k.ghi_phan_tich((2, 20), &vec![TrangCache::default(); 30]);
            k.bo_phan_tich((2, 20));
        }
        let mot = dung_luong_cache(&kho, &[(1, 10)]);
        assert!(mot > 0);
        assert_eq!(dung_luong_cache(&kho, &[(1, 10), (2, 20), (3, 30)]), mot, "PDF đã bỏ cache / chưa từng có không tính");
        assert_eq!(dung_luong_cache(&kho, &[]), 0);
    }

    /// Chụp thân 3 trang (Bóc tách ×3 trạng thái, Xóa ghim, Dữ liệu) trong cột 880 px trên nền xám của trang Cấu hình.
    /// Chạy: DEMA4_ANH=<thư mục ảnh> cargo test --release -- --ignored chup_cau_hinh_bt_xg_du_lieu
    #[test]
    #[ignore = "cần GPU"]
    fn chup_cau_hinh_bt_xg_du_lieu() {
        use crate::cau_hinh::{CauHinh, CauHinhXoaGhim};
        use crate::gui_boc_tach::cau_hinh::{Muc, TabBocTach};
        use crate::gui_xoa_ghim::TabXoaGhim;
        use crate::boc_tach::mo_hinh::NhaCungCap;
        use std::collections::BTreeMap;

        let thu_muc_anh = PathBuf::from(std::env::var("DEMA4_ANH").expect("DEMA4_ANH"));
        let mut cfg = CauHinh::default();
        let b = &mut cfg.boc_tach;
        (b.mo_hinh_mut(NhaCungCap::NoiBo).dia_chi, b.mo_hinh_mut(NhaCungCap::NoiBo).model) = ("http://192.168.1.10:8000/v1".into(), "5CD-AI/Vintern-1B-v3_5".into());
        b.mo_hinh_mut(NhaCungCap::Gemini).model = "gemini-2.5-flash".into();
        b.mac_dinh = Some(NhaCungCap::NoiBo);
        b.sheet_link = "https://docs.google.com/spreadsheets/d/1AbC/edit".into();
        let khoa: BTreeMap<String, String> = [("gemini".to_string(), "khoa-minh-hoa".to_string())].into();

        struct Trang {
            bt: TabBocTach,
            ch: crate::cau_hinh::CauHinhBocTach,
            khoa: BTreeMap<String, String>,
            xg: TabXoaGhim,
            cfg_xg: CauHinhXoaGhim,
            dl: TrangDuLieu,
            hien: u8,
        }
        let ch = cfg.boc_tach.clone();
        let xg = TabXoaGhim::dang_mo_thu_muc();
        let trang = Trang { bt: TabBocTach::moi(&ch), ch, khoa, xg, cfg_xg: CauHinhXoaGhim::default(), dl: trang_mau(Some(190_000_000)), hien: 0 };
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1000.0, 900.0)).with_pixels_per_point(1.5).build_ui_state(
            |ui, t: &mut Trang| {
                crate::gui::cai_giao_dien(ui.ctx());
                ui.painter().rect_filled(ui.max_rect(), 0.0, crate::gui_cau_hinh::NEN_TRANG);
                ui.spacing_mut().item_spacing.y = 8.0;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        ui.allocate_ui_with_layout(egui::vec2(crate::gui_cau_hinh::RONG_COT, 800.0), egui::Layout::top_down(egui::Align::Min), |ui| match t.hien {
                            0..=3 => t.bt.ve(ui, &mut t.ch, &mut t.khoa),
                            4 => t.xg.ve(ui, &mut t.cfg_xg),
                            _ => {
                                t.dl.ve(ui);
                            }
                        });
                    });
                });
            },
            trang,
        );
        let ds = [("cau_hinh_moi_bt_doc", Muc::MoHinh, NhaCungCap::NoiBo, 0u8), ("cau_hinh_moi_bt_doc_gemini", Muc::MoHinh, NhaCungCap::Gemini, 1), ("cau_hinh_moi_bt_nguon", Muc::Nguon, NhaCungCap::NoiBo, 2), ("cau_hinh_moi_bt_ghi", Muc::Ghi, NhaCungCap::NoiBo, 3), ("cau_hinh_moi_xg", Muc::MoHinh, NhaCungCap::NoiBo, 4), ("cau_hinh_moi_du_lieu", Muc::MoHinh, NhaCungCap::NoiBo, 5)];
        for (ten, muc, ncc, hien) in ds {
            let t = h.state_mut();
            (t.bt.muc, t.bt.ncc, t.hien) = (muc, ncc, hien);
            h.run_steps(3);
            h.render().unwrap().save(thu_muc_anh.join(format!("{ten}.png"))).unwrap();
        }
        // Chưa mở thư mục ở Xóa ghim
        let t = h.state_mut();
        (t.xg.thu_muc, t.hien) = (None, 4);
        h.run_steps(3);
        h.render().unwrap().save(thu_muc_anh.join("cau_hinh_moi_xg_chua_mo.png")).unwrap();
    }
}
