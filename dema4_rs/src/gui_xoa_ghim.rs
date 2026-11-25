//! Màn Xóa ghim: chọn thư mục PDF scan, bật vùng góc / cạnh hoặc vẽ vùng tay (xoá / bảo vệ / xoá vô đối; chung thư
//! mục, riêng file, riêng trang; tất cả / lẻ / chẵn), xem trước **Gốc | Kết quả** ở đúng độ phân giải sẽ xuất.
//! Vùng chung + tuỳ chọn lưu kho Xóa ghim theo thư mục, vùng riêng theo nội dung PDF; ghi trễ ngắn sau mỗi thay đổi.

mod sua_vung;
mod xem_truoc;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle};

use crate::gui::{CHU, CHU_PHU, DO, VIEN_NHAT};
use crate::gui_tach::bieu_tuong::{atom_mau, BieuTuong};
use crate::xoa_ghim::kho::{self as kho_xg, CauHinhXuat, ThuMuc};
use crate::xoa_ghim::vung::{co_san, ApTrang, LoaiVung, ViTri};
use crate::xoa_ghim::xuat::{self, HAU_TO};
use sua_vung::{BoVung, PhamVi};
use xem_truoc::{XemTruoc, YeuCau};

pub const TIEU_DE: &str = "Xóa ghim (Giải pháp số Sao Mai)";
/// Ghi vùng / tuỳ chọn vào kho sau khi ngừng chỉnh chừng này (kéo thanh trượt không ghi liên tục).
const TRE_GHI: Duration = Duration::from_millis(600);
/// Lựa chọn DPI xuất như XoaGhim (0 = Gốc: theo ảnh scan).
const CAC_DPI: [u16; 6] = [0, 300, 250, 200, 100, 72];
const CAC_CHAT_LUONG: [u8; 4] = [100, 90, 80, 70];
const NEN_XEM: Color32 = Color32::from_rgb(0xE9, 0xE9, 0xE9);

/// Vùng mặc định của thư mục chưa chỉnh lần nào: góc trên trái + góc trên phải (như XoaGhim).
pub fn thu_muc_mac_dinh() -> ThuMuc {
    ThuMuc { luc: 0, vung: vec![co_san(ViTri::GocTrenTrai), co_san(ViTri::GocTrenPhai)], tuy_chon: Default::default(), xuat: CauHinhXuat::default() }
}

/// Đường dẫn thư mục kết quả mặc định của nguồn (cạnh nguồn).
pub fn thu_muc_ket_qua(nguon: &Path) -> Result<PathBuf, String> {
    crate::tach_vb::tach::thu_muc_ra_hau_to(nguon, None, HAU_TO)
}

fn mau_loai(l: LoaiVung) -> Color32 {
    match l {
        LoaiVung::Xoa => Color32::from_rgb(0x2F, 0x7B, 0xD9),
        LoaiVung::BaoVe => Color32::from_rgb(0x00, 0x99, 0x44),
        LoaiVung::VoDoi => Color32::from_rgb(0xC4, 0x2B, 0x1C),
    }
}

fn ten_loai(l: LoaiVung) -> &'static str {
    match l {
        LoaiVung::Xoa => "Xoá",
        LoaiVung::BaoVe => "Bảo vệ",
        LoaiVung::VoDoi => "Xoá vô đối",
    }
}

fn ten_ap(a: ApTrang) -> String {
    match a {
        ApTrang::TatCa => "Mọi trang".into(),
        ApTrang::Le => "Trang lẻ".into(),
        ApTrang::Chan => "Trang chẵn".into(),
        ApTrang::MotTrang(t) => format!("Trang {}", t + 1),
    }
}

struct AnhXem {
    goc: TextureHandle,
    ket_qua: TextureHandle,
    co_anh: (usize, usize),
    dpi: u16,
}

pub struct ManXoaGhim {
    pub thu_muc: Option<PathBuf>,
    /// (PDF nguồn, PDF kết quả) theo thứ tự cây thư mục.
    ds: Vec<(PathBuf, PathBuf)>,
    chon: usize,
    trang: u32,
    so_trang: u32,
    bo: BoVung,
    /// Chế độ vẽ (kéo chuột trên ảnh Gốc tạo vùng); tắt = chọn vùng.
    ve: bool,
    loai_ve: LoaiVung,
    pham_vi: PhamVi,
    ap_ve: ApTrang,
    keo: Option<Pos2>,
    xem: XemTruoc,
    so_yc: u64,
    yc_cuoi: Option<YeuCau>,
    anh: Option<AnhXem>,
    loi: Option<String>,
    can_ghi: Option<Instant>,
}

impl Default for ManXoaGhim {
    fn default() -> Self {
        ManXoaGhim {
            thu_muc: None,
            ds: Vec::new(),
            chon: 0,
            trang: 0,
            so_trang: 0,
            bo: BoVung::moi(thu_muc_mac_dinh()),
            ve: false,
            loai_ve: LoaiVung::Xoa,
            pham_vi: PhamVi::ThuMuc,
            ap_ve: ApTrang::TatCa,
            keo: None,
            xem: XemTruoc::default(),
            so_yc: 0,
            yc_cuoi: None,
            anh: None,
            loi: None,
            can_ghi: None,
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
        let tm = kho_xg::doc_thu_muc(&mut kho_xg::kho(), &p).unwrap_or_else(thu_muc_mac_dinh);
        self.bo = BoVung::moi(tm);
        self.thu_muc = Some(p);
        self.chon = usize::MAX;
        self.chon_file(0);
        if self.ds.is_empty() && self.loi.is_none() {
            self.loi = Some("Thư mục không có PDF".into());
        }
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
        self.so_trang = 0;
        self.anh = None;
        // Id qua bảng đường dẫn của kho Tách (Dọn dữ liệu cũ dựa vào bảng này để biết PDF còn dùng)
        let id = crate::tach_vb::kho::ids_cua(std::slice::from_ref(&self.ds[i].0))[0];
        let vf = id.and_then(|id| kho_xg::doc_vung_file(&mut kho_xg::kho(), id)).unwrap_or_default();
        self.bo.doi_file(id, vf);
    }

    fn gui_xem_truoc(&mut self) {
        let Some((pdf, _)) = self.ds.get(self.chon) else { return };
        let yc = YeuCau { so: 0, pdf: pdf.clone(), trang: self.trang, vung: self.bo.vung(), tc: self.bo.tm.tuy_chon, dpi_chon: self.bo.tm.xuat.dpi };
        if self.yc_cuoi.as_ref().is_some_and(|c| YeuCau { so: 0, ..c.clone() } == yc) {
            return;
        }
        self.so_yc += 1;
        let yc = YeuCau { so: self.so_yc, ..yc };
        self.yc_cuoi = Some(yc.clone());
        self.xem.gui(yc);
    }

    fn nhan_xem_truoc(&mut self, ctx: &egui::Context) {
        match self.xem.nhan() {
            Some(Ok(kq)) if kq.so == self.so_yc => {
                self.so_trang = kq.so_trang;
                self.anh = Some(AnhXem {
                    goc: ctx.load_texture("xg_goc", kq.goc, egui::TextureOptions::LINEAR),
                    ket_qua: ctx.load_texture("xg_kq", kq.ket_qua, egui::TextureOptions::LINEAR),
                    co_anh: kq.co_anh,
                    dpi: kq.dpi,
                });
                if self.loi.as_deref().is_some_and(|l| l.starts_with("Không mở được PDF")) {
                    self.loi = None;
                }
            }
            Some(Err((so, e))) if so == self.so_yc => {
                self.anh = None;
                self.loi = Some(e);
            }
            _ => {}
        }
    }

    pub fn ve(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.nhan_xem_truoc(&ctx);
        if (self.bo.doi_chung || self.bo.doi_rieng) && self.can_ghi.is_none() {
            self.can_ghi = Some(Instant::now());
        }
        if self.can_ghi.is_some_and(|t| t.elapsed() >= TRE_GHI) {
            self.ghi_ngay();
        } else if self.can_ghi.is_some() {
            ctx.request_repaint_after(TRE_GHI);
        }
        self.phim(&ctx);
        self.thanh_cong_cu(ui);
        if self.thu_muc.is_some() {
            self.cot_file(ui);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(NEN_XEM)).show(ui, |ui| self.khung_xem(ui));
        self.gui_xem_truoc();
        if self.yc_cuoi.as_ref().is_some_and(|y| self.anh.is_none() && y.so == self.so_yc) && self.loi.is_none() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn phim(&mut self, ctx: &egui::Context) {
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        use egui::Key;
        let (truoc, sau, len, xuong, xoa, hoan_tac, ve, chon) = ctx.input(|i| {
            (
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::CloseBracket),
                i.key_pressed(Key::PageUp),
                i.key_pressed(Key::PageDown),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.modifiers.command && i.key_pressed(Key::Z),
                !i.modifiers.command && i.key_pressed(Key::V),
                !i.modifiers.command && i.key_pressed(Key::C),
            )
        });
        if truoc && self.chon > 0 {
            self.chon_file(self.chon - 1);
        }
        if sau {
            self.chon_file(self.chon + 1);
        }
        if len && self.trang > 0 {
            self.trang -= 1;
        }
        if xuong && self.trang + 1 < self.so_trang {
            self.trang += 1;
        }
        if xoa {
            self.bo.xoa_chon();
        }
        if hoan_tac {
            self.bo.hoan_tac();
        }
        if ve {
            self.ve = true;
        }
        if chon {
            self.ve = false;
        }
    }

    fn thanh_cong_cu(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("xg_cong_cu").frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(10, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Chọn thư mục…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().set_title("Chọn thư mục PDF cần xoá ghim").pick_folder() {
                        self.mo_thu_muc(p);
                    }
                }
                match &self.thu_muc {
                    Some(p) => ui.label(p.display().to_string()),
                    None => ui.label(RichText::new("Chưa chọn thư mục").color(CHU_PHU)),
                };
                if let Some(e) = &self.loi {
                    ui.label(RichText::new(e).color(DO));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Có sẵn:");
                for v in ViTri::CO_SAN {
                    let bat = self.bo.tm.vung.iter().any(|z| z.vi_tri == v);
                    if ui.selectable_label(bat, v.ten()).on_hover_text("Bấm để bật / tắt vùng").clicked() && self.bo.bat_tat(v) {
                        self.bo.chon = self.bo.tm.vung.iter().position(|z| z.vi_tri == v).map(|i| (false, i));
                    }
                }
            });
            self.thanh_ve(ui);
            self.chinh_vung(ui);
            self.tuy_chon(ui);
        });
    }

    /// Chế độ chọn / vẽ, loại + phạm vi + trang của vùng vẽ mới, hoàn tác, xoá, bộ đếm.
    fn thanh_ve(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(!self.ve, "Chọn (C)").on_hover_text("Bấm vào khung vùng trên ảnh Gốc để chỉnh / xoá (Delete)").clicked() {
                self.ve = false;
            }
            if ui.selectable_label(self.ve, "Vẽ vùng (V)").on_hover_text("Kéo chuột trên ảnh Gốc để vẽ vùng").clicked() {
                self.ve = true;
            }
            ui.separator();
            for l in [LoaiVung::Xoa, LoaiVung::BaoVe, LoaiVung::VoDoi] {
                let r = ui.selectable_label(self.loai_ve == l, RichText::new(ten_loai(l)).color(mau_loai(l)));
                if r.on_hover_text(match l {
                    LoaiVung::Xoa => "Xoá vết trong vùng, tránh vùng bảo vệ",
                    LoaiVung::BaoVe => "Không xoá gì trong vùng này",
                    LoaiVung::VoDoi => "Xoá cả trong vùng bảo vệ",
                }).clicked() {
                    self.loai_ve = l;
                    self.ve = true;
                }
            }
            ui.separator();
            for (p, ten) in [(PhamVi::ThuMuc, "Chung thư mục"), (PhamVi::File, "Riêng file"), (PhamVi::Trang, "Riêng trang")] {
                if ui.selectable_label(self.pham_vi == p, ten).clicked() {
                    self.pham_vi = p;
                }
            }
            if self.pham_vi != PhamVi::Trang {
                egui::ComboBox::from_id_salt("xg_ap_ve").selected_text(ten_ap(self.ap_ve)).show_ui(ui, |ui| {
                    for a in [ApTrang::TatCa, ApTrang::Le, ApTrang::Chan] {
                        if ui.selectable_label(self.ap_ve == a, ten_ap(a)).clicked() {
                            self.ap_ve = a;
                        }
                    }
                });
            }
            ui.separator();
            if ui.add_enabled(self.bo.co_hoan_tac(), egui::Button::new((atom_mau(BieuTuong::HoanTac, 14.0, CHU), "Hoàn tác"))).on_hover_text("Ctrl+Z").clicked() {
                self.bo.hoan_tac();
            }
            if ui.add_enabled(self.bo.chon.is_some(), egui::Button::new("Xoá vùng chọn")).on_hover_text("Delete").clicked() {
                self.bo.xoa_chon();
            }
            ui.menu_button("Xoá hết…", |ui| {
                if ui.button("Vùng vẽ chung của thư mục").clicked() {
                    self.bo.xoa_het_ve(false);
                    ui.close();
                }
                if ui.add_enabled(!self.bo.vf.vung.is_empty(), egui::Button::new("Vùng riêng của file này")).clicked() {
                    self.bo.xoa_het_ve(true);
                    ui.close();
                }
            });
            let (chung, rieng) = self.bo.dem();
            ui.label(RichText::new(format!("Vùng chung: {chung} · riêng file: {rieng}")).color(CHU_PHU));
        });
    }

    /// Chỉnh vùng đang chọn: có sẵn → kích thước; mọi vùng → ngưỡng (vùng xoá), loại + trang (vùng vẽ).
    fn chinh_vung(&mut self, ui: &mut egui::Ui) {
        let Some(z) = self.bo.vung_chon().cloned() else {
            ui.label(RichText::new("Chọn 1 vùng (bấm vào khung trên ảnh Gốc) để chỉnh kích thước, ngưỡng.").size(11.5).color(CHU_PHU));
            return;
        };
        let mut moi = z.clone();
        let mut keo_bat_dau = false;
        ui.horizontal_wrapped(|ui| {
            let rieng = self.bo.chon.is_some_and(|(r, _)| r);
            ui.label(RichText::new(format!("{}{}:", z.vi_tri.ten(), if rieng { " (riêng file)" } else { "" })).strong());
            let mut thanh = |ui: &mut egui::Ui, gia_tri: &mut u32, khoang: std::ops::RangeInclusive<u32>, chu: &str| {
                let r = ui.add(egui::Slider::new(gia_tri, khoang).text(chu));
                keo_bat_dau |= r.drag_started();
            };
            if z.vi_tri.la_goc() {
                thanh(ui, &mut moi.rong_px, 20..=600, "rộng");
                thanh(ui, &mut moi.cao_px, 20..=600, "cao");
                ui.label(RichText::new("điểm ảnh @120 dpi").size(11.0).color(CHU_PHU));
            } else if matches!(z.vi_tri, ViTri::CanhTrai | ViTri::CanhPhai) {
                thanh(ui, &mut moi.rong_px, 5..=400, "độ sâu");
            } else if matches!(z.vi_tri, ViTri::CanhTren | ViTri::CanhDuoi) {
                thanh(ui, &mut moi.cao_px, 5..=400, "độ sâu");
            }
            if z.loai != LoaiVung::BaoVe {
                let mut n = moi.nguong as u32;
                thanh(ui, &mut n, 1..=30, "ngưỡng");
                moi.nguong = n as u8;
            }
            if z.vi_tri == ViTri::Tay {
                egui::ComboBox::from_id_salt("xg_loai_chon").selected_text(ten_loai(moi.loai)).show_ui(ui, |ui| {
                    for l in [LoaiVung::Xoa, LoaiVung::BaoVe, LoaiVung::VoDoi] {
                        ui.selectable_value(&mut moi.loai, l, ten_loai(l));
                    }
                });
                if !matches!(moi.ap_trang, ApTrang::MotTrang(_)) {
                    egui::ComboBox::from_id_salt("xg_ap_chon").selected_text(ten_ap(moi.ap_trang)).show_ui(ui, |ui| {
                        for a in [ApTrang::TatCa, ApTrang::Le, ApTrang::Chan] {
                            ui.selectable_value(&mut moi.ap_trang, a, ten_ap(a));
                        }
                    });
                } else {
                    ui.label(ten_ap(moi.ap_trang));
                }
            }
        });
        if keo_bat_dau || (moi != z && (moi.loai != z.loai || moi.ap_trang != z.ap_trang)) {
            self.bo.ghi_buoc();
        }
        if moi != z {
            if let Some(v) = self.bo.vung_chon_mut() {
                *v = moi;
            }
            self.bo.sua_chon();
        }
    }

    fn tuy_chon(&mut self, ui: &mut egui::Ui) {
        let mut doi = false;
        ui.horizontal_wrapped(|ui| {
            let tc = &mut self.bo.tm.tuy_chon;
            doi |= ui.checkbox(&mut tc.bao_ve_net_den, "Bảo vệ nét đen").on_hover_text("Không lấy điểm rất tối làm vết (giữ nét chữ đen)").changed();
            doi |= ui.checkbox(&mut tc.bao_ve_do_xanh, "Bảo vệ dấu đỏ / chữ ký xanh").changed();
            doi |= ui.checkbox(&mut tc.nen_trang, "Tô nền trắng").changed();
            ui.separator();
            let x = &mut self.bo.tm.xuat;
            let ten_dpi = |d: u16| if d == 0 { "Gốc".to_string() } else { format!("{d}") };
            egui::ComboBox::from_id_salt("xg_dpi").selected_text(format!("DPI: {}", ten_dpi(x.dpi))).show_ui(ui, |ui| {
                for d in CAC_DPI {
                    if ui.selectable_label(x.dpi == d, ten_dpi(d)).clicked() && x.dpi != d {
                        x.dpi = d;
                        doi = true;
                    }
                }
            });
            egui::ComboBox::from_id_salt("xg_jpeg").selected_text(format!("Nén: {}%", x.chat_luong)).show_ui(ui, |ui| {
                for c in CAC_CHAT_LUONG {
                    if ui.selectable_label(x.chat_luong == c, format!("{c}%")).clicked() && x.chat_luong != c {
                        x.chat_luong = c;
                        doi = true;
                    }
                }
            });
            doi |= ui.checkbox(&mut x.nen_den_trang, "Nén đen trắng").on_hover_text("Trang đen trắng / xám ghi gọn hơn").changed();
        });
        if doi {
            self.bo.doi_chung();
        }
    }

    fn cot_file(&mut self, ui: &mut egui::Ui) {
        let goc = self.thu_muc.clone().unwrap_or_default();
        egui::Panel::left("xg_file").resizable(true).default_size(240.0).frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(6)).show(ui, |ui| {
            ui.label(RichText::new(format!("{} file", self.ds.len())).color(CHU_PHU));
            let mut chon = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 20.0, self.ds.len(), |ui, hang| {
                for i in hang {
                    let ten = self.ds[i].0.strip_prefix(&goc).unwrap_or(&self.ds[i].0).display().to_string();
                    if ui.selectable_label(i == self.chon, ten).clicked() {
                        chon = Some(i);
                    }
                }
            });
            if let Some(i) = chon {
                self.chon_file(i);
            }
        });
    }

    fn khung_xem(&mut self, ui: &mut egui::Ui) {
        if self.ds.is_empty() {
            ui.centered_and_justified(|ui| ui.label(RichText::new("Chọn thư mục chứa PDF scan để xoá ghim.").color(CHU_PHU)));
            return;
        }
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if ui.add_enabled(self.trang > 0, egui::Button::new(atom_mau(BieuTuong::TraiChevron, 14.0, CHU))).on_hover_text("PageUp").clicked() {
                self.trang -= 1;
            }
            ui.label(if self.so_trang > 0 { format!("Trang {}/{}", self.trang + 1, self.so_trang) } else { "Trang …".into() });
            if ui.add_enabled(self.trang + 1 < self.so_trang, egui::Button::new(atom_mau(BieuTuong::PhaiChevron, 14.0, CHU))).on_hover_text("PageDown").clicked() {
                self.trang += 1;
            }
            if let Some(a) = &self.anh {
                ui.label(RichText::new(format!("Xem ở {} dpi — như file xuất · [ ] đổi file", a.dpi)).color(CHU_PHU));
            }
        });
        let Some(a) = &self.anh else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };
        let (co_anh, dpi) = (a.co_anh, a.dpi);
        let khung = ui.available_rect_before_wrap().shrink(8.0);
        let nua = (khung.width() - 8.0) / 2.0;
        let (w, h) = (co_anh.0 as f32, co_anh.1 as f32);
        let ty = (nua / w).min((khung.height() - 14.0) / h);
        let co = egui::vec2(w * ty, h * ty);
        let o_goc = Rect::from_min_size(egui::pos2(khung.left() + (nua - co.x) / 2.0, khung.top() + 14.0), co);
        let o_kq = o_goc.translate(egui::vec2(nua + 8.0, 0.0));
        ui.allocate_rect(khung, Sense::hover());
        let ph = ui.interact(o_goc, ui.scope_id().with("xg_ve"), Sense::click_and_drag());
        {
            let p = ui.painter();
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            for (o, tex) in [(o_goc, &a.goc), (o_kq, &a.ket_qua)] {
                p.image(tex.id(), o, uv, Color32::WHITE);
                p.rect_stroke(o, 0.0, Stroke::new(1.0, VIEN_NHAT), egui::StrokeKind::Outside);
            }
            p.text(o_goc.left_top() + egui::vec2(0.0, -2.0), egui::Align2::LEFT_BOTTOM, "Gốc", egui::FontId::proportional(11.0), CHU_PHU);
            p.text(o_kq.left_top() + egui::vec2(0.0, -2.0), egui::Align2::LEFT_BOTTOM, "Kết quả", egui::FontId::proportional(11.0), CHU_PHU);
        }
        // Khung vùng trên ảnh gốc (đúng khung xử lý, kể cả phần nới ra mép); vùng đang chọn viền đậm
        let chung = self.bo.tm.vung.len();
        for (i, z) in self.bo.vung().iter().enumerate() {
            if !z.ap_trang.ap_dung(self.trang) {
                continue;
            }
            let Some(k) = z.khung_xoa(co_anh.0 as i64, co_anh.1 as i64, dpi as f64) else { continue };
            let r = Rect::from_min_max(o_goc.min + egui::vec2(k.x1 as f32 * ty, k.y1 as f32 * ty), o_goc.min + egui::vec2(k.x2 as f32 * ty, k.y2 as f32 * ty));
            let dang_chon = self.bo.chon == Some((i >= chung, if i >= chung { i - chung } else { i }));
            let mau = mau_loai(z.loai);
            ui.painter().rect_filled(r, 0.0, mau.gamma_multiply(if dang_chon { 0.22 } else { 0.12 }));
            ui.painter().rect_stroke(r, 0.0, Stroke::new(if dang_chon { 2.5 } else { 1.2 }, mau), egui::StrokeKind::Inside);
        }
        // Vẽ: kéo trên ảnh Gốc; Chọn: bấm vào khung vùng
        let ve_anh = |p: Pos2| ((p.x - o_goc.left()) / co.x, (p.y - o_goc.top()) / co.y);
        if self.ve {
            if ph.drag_started() {
                self.keo = ph.interact_pointer_pos();
            }
            if let (Some(a0), Some(b0)) = (self.keo, ui.ctx().pointer_latest_pos()) {
                let r = Rect::from_two_pos(a0, b0.clamp(o_goc.min, o_goc.max));
                ui.painter().rect_stroke(r, 0.0, Stroke::new(1.5, mau_loai(self.loai_ve)), egui::StrokeKind::Inside);
                if ph.drag_stopped() || !ui.ctx().input(|i| i.pointer.primary_down()) {
                    self.keo = None;
                    let (x1, y1) = ve_anh(r.min);
                    let (x2, y2) = ve_anh(r.max);
                    self.bo.them_ve((x1 as f64, y1 as f64, (x2 - x1) as f64, (y2 - y1) as f64), self.loai_ve, self.pham_vi, self.ap_ve, self.trang);
                    if self.pham_vi != PhamVi::ThuMuc && self.bo.id_file.is_none() {
                        self.loi = Some("Không đọc được PDF này — chỉ vẽ được vùng chung".into());
                    }
                }
            }
            ph.on_hover_cursor(egui::CursorIcon::Crosshair);
        } else if ph.clicked() {
            if let Some(p) = ph.interact_pointer_pos() {
                let (x, y) = ve_anh(p);
                self.bo.chon = self.bo.tim(self.trang, ((x * w) as i64, (y * h) as i64), (co_anh.0 as i64, co_anh.1 as i64), dpi as f64);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chụp màn Xóa ghim với 1 thư mục mẫu để xem bằng mắt:
    /// `DEMA4_MAU_XG=<thư mục> DEMA4_ANH_XG=<file.png> cargo test chup_man_xoa_ghim -- --ignored`.
    #[test]
    #[ignore]
    fn chup_man_xoa_ghim() {
        let (Some(mau), Some(f)) = (std::env::var_os("DEMA4_MAU_XG"), std::env::var_os("DEMA4_ANH_XG")) else { return };
        let mut man = ManXoaGhim::default();
        man.mo_thu_muc(PathBuf::from(mau));
        man.bo.them_ve((0.6, 0.05, 0.3, 0.06), LoaiVung::BaoVe, PhamVi::File, ApTrang::TatCa, 0);
        let mut da_cai = false;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui(move |ui| {
            if !std::mem::replace(&mut da_cai, true) {
                crate::gui::cai_giao_dien(ui.ctx());
            }
            man.ve(ui);
        });
        for _ in 0..100 {
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(50));
        }
        h.render().unwrap().save(f).unwrap();
    }
}
