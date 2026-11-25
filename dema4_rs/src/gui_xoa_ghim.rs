//! Màn Xóa ghim: chọn thư mục PDF scan, bật vùng góc / cạnh, chỉnh kích thước + ngưỡng, xem trước **Gốc | Kết quả**
//! ở đúng độ phân giải sẽ xuất. Vùng chung + tuỳ chọn của thư mục lưu trong kho Xóa ghim (theo thư mục), ghi trễ
//! ngắn sau mỗi thay đổi; mở lại thư mục thấy lại.

mod xem_truoc;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, TextureHandle};

use crate::gui::{CHU, CHU_PHU, DO, VIEN_NHAT};
use crate::gui_tach::bieu_tuong::{atom_mau, BieuTuong};
use crate::xoa_ghim::kho::{self as kho_xg, CauHinhXuat, ThuMuc};
use crate::xoa_ghim::vung::{co_san, LoaiVung, ViTri, Vung};
use crate::xoa_ghim::xuat::{self, HAU_TO};
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
    tm: ThuMuc,
    dang_chinh: Option<ViTri>,
    xem: XemTruoc,
    so_yc: u64,
    yc_cuoi: Option<YeuCau>,
    anh: Option<AnhXem>,
    loi: Option<String>,
    can_ghi: Option<Instant>,
}

impl Default for ManXoaGhim {
    fn default() -> Self {
        ManXoaGhim { thu_muc: None, ds: Vec::new(), chon: 0, trang: 0, so_trang: 0, tm: thu_muc_mac_dinh(), dang_chinh: None, xem: XemTruoc::default(), so_yc: 0, yc_cuoi: None, anh: None, loi: None, can_ghi: None }
    }
}

impl ManXoaGhim {
    /// Mở thư mục nguồn: liệt kê PDF (bỏ thư mục kết quả `*_xoaghim`), nạp vùng chung của thư mục từ kho.
    pub fn mo_thu_muc(&mut self, p: PathBuf) {
        self.ghi_ngay();
        let ra = thu_muc_ket_qua(&p);
        self.loi = ra.as_ref().err().cloned();
        self.ds = xuat::ds_viec(&p, ra.as_deref().unwrap_or(&p), &std::sync::atomic::AtomicBool::new(false));
        self.tm = kho_xg::doc_thu_muc(&mut kho_xg::kho(), &p).unwrap_or_else(thu_muc_mac_dinh);
        self.thu_muc = Some(p);
        self.chon = 0;
        self.trang = 0;
        self.so_trang = 0;
        self.anh = None;
        self.yc_cuoi = None;
        if self.ds.is_empty() && self.loi.is_none() {
            self.loi = Some("Thư mục không có PDF".into());
        }
    }

    /// Vùng áp cho file đang xem, đúng thứ tự xuất.
    fn vung(&self) -> Vec<Vung> {
        self.tm.vung.clone()
    }

    fn doi(&mut self) {
        self.tm.luc = crate::tach_vb::moc_sua::bay_gio_ms();
        self.can_ghi = Some(Instant::now());
    }

    /// Ghi phần chung của thư mục xuống kho ngay (đổi thư mục / rời màn).
    pub fn ghi_ngay(&mut self) {
        if self.can_ghi.take().is_none() {
            return;
        }
        let Some(tm) = &self.thu_muc else { return };
        let mut k = kho_xg::kho();
        kho_xg::ghi_thu_muc(&mut k, tm, &self.tm);
        if let Err(e) = k.day() {
            self.loi = Some(format!("Chưa lưu được vùng: {e}"));
        }
    }

    fn gui_xem_truoc(&mut self) {
        let Some((pdf, _)) = self.ds.get(self.chon) else { return };
        let yc = YeuCau { so: 0, pdf: pdf.clone(), trang: self.trang, vung: self.vung(), tc: self.tm.tuy_chon, dpi_chon: self.tm.xuat.dpi };
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

    fn chon_file(&mut self, i: usize) {
        if i < self.ds.len() && i != self.chon {
            self.chon = i;
            self.trang = 0;
            self.so_trang = 0;
            self.anh = None;
        }
    }

    pub fn ve(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.nhan_xem_truoc(&ctx);
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
        let (truoc, sau, len, xuong) = ctx.input(|i| (i.key_pressed(egui::Key::OpenBracket), i.key_pressed(egui::Key::CloseBracket), i.key_pressed(egui::Key::PageUp), i.key_pressed(egui::Key::PageDown)));
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
                ui.label("Vùng:");
                for v in ViTri::CO_SAN {
                    let bat = self.tm.vung.iter().any(|z| z.vi_tri == v);
                    let r = ui.selectable_label(bat, v.ten()).on_hover_text("Bấm để bật / tắt vùng");
                    if r.clicked() {
                        if bat {
                            self.tm.vung.retain(|z| z.vi_tri != v);
                            if self.dang_chinh == Some(v) {
                                self.dang_chinh = None;
                            }
                        } else {
                            // Giữ thứ tự góc → cạnh như XoaGhim
                            self.tm.vung.push(co_san(v));
                            self.tm.vung.sort_by_key(|z| ViTri::CO_SAN.iter().position(|x| *x == z.vi_tri).unwrap_or(usize::MAX));
                            self.dang_chinh = Some(v);
                        }
                        self.doi();
                    }
                }
            });
            self.chinh_vung(ui);
            self.tuy_chon(ui);
        });
    }

    /// Thanh trượt kích thước + ngưỡng của vùng có sẵn đang chỉnh.
    fn chinh_vung(&mut self, ui: &mut egui::Ui) {
        let bat: Vec<ViTri> = self.tm.vung.iter().map(|z| z.vi_tri).filter(|v| v.la_goc_canh()).collect();
        if bat.is_empty() {
            return;
        }
        if self.dang_chinh.is_none_or(|v| !bat.contains(&v)) {
            self.dang_chinh = bat.first().copied();
        }
        let Some(dang) = self.dang_chinh else { return };
        let mut doi = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Chỉnh:");
            egui::ComboBox::from_id_salt("xg_chinh").selected_text(dang.ten()).show_ui(ui, |ui| {
                for v in &bat {
                    if ui.selectable_label(*v == dang, v.ten()).clicked() {
                        self.dang_chinh = Some(*v);
                    }
                }
            });
            let Some(z) = self.tm.vung.iter_mut().find(|z| z.vi_tri == dang) else { return };
            if dang.la_goc() {
                doi |= ui.add(egui::Slider::new(&mut z.rong_px, 20..=600).text("rộng")).changed();
                doi |= ui.add(egui::Slider::new(&mut z.cao_px, 20..=600).text("cao")).changed();
            } else if matches!(dang, ViTri::CanhTrai | ViTri::CanhPhai) {
                doi |= ui.add(egui::Slider::new(&mut z.rong_px, 5..=400).text("độ sâu")).changed();
            } else {
                doi |= ui.add(egui::Slider::new(&mut z.cao_px, 5..=400).text("độ sâu")).changed();
            }
            ui.label(RichText::new("điểm ảnh @120 dpi").size(11.0).color(CHU_PHU));
            doi |= ui.add(egui::Slider::new(&mut z.nguong, 1..=30).text("ngưỡng")).on_hover_text("Điểm tối hơn nền quá ngưỡng này là vết; nhỏ = xoá nhiều hơn").changed();
        });
        if doi {
            self.doi();
        }
    }

    fn tuy_chon(&mut self, ui: &mut egui::Ui) {
        let mut doi = false;
        ui.horizontal_wrapped(|ui| {
            let tc = &mut self.tm.tuy_chon;
            doi |= ui.checkbox(&mut tc.bao_ve_net_den, "Bảo vệ nét đen").on_hover_text("Không lấy điểm rất tối làm vết (giữ nét chữ đen)").changed();
            doi |= ui.checkbox(&mut tc.bao_ve_do_xanh, "Bảo vệ dấu đỏ / chữ ký xanh").changed();
            doi |= ui.checkbox(&mut tc.nen_trang, "Tô nền trắng").changed();
            ui.separator();
            let x = &mut self.tm.xuat;
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
            self.doi();
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
            if ui.add_enabled(self.trang > 0, egui::Button::new(atom_mau(BieuTuong::TraiChevron, 14.0, CHU))).clicked() {
                self.trang -= 1;
            }
            ui.label(if self.so_trang > 0 { format!("Trang {}/{}", self.trang + 1, self.so_trang) } else { "Trang …".into() });
            if ui.add_enabled(self.trang + 1 < self.so_trang, egui::Button::new(atom_mau(BieuTuong::PhaiChevron, 14.0, CHU))).clicked() {
                self.trang += 1;
            }
            if let Some(a) = &self.anh {
                ui.label(RichText::new(format!("Xem ở {} dpi — như file xuất", a.dpi)).color(CHU_PHU));
            }
        });
        let Some(a) = &self.anh else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };
        let khung = ui.available_rect_before_wrap().shrink(8.0);
        let nua = (khung.width() - 8.0) / 2.0;
        let (w, h) = (a.co_anh.0 as f32, a.co_anh.1 as f32);
        let ty = (nua / w).min(khung.height() / h);
        let co = egui::vec2(w * ty, h * ty);
        let o_goc = Rect::from_min_size(egui::pos2(khung.left() + (nua - co.x) / 2.0, khung.top()), co);
        let o_kq = o_goc.translate(egui::vec2(nua + 8.0, 0.0));
        ui.allocate_rect(khung, Sense::hover());
        let p = ui.painter();
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        for (o, tex) in [(o_goc, &a.goc), (o_kq, &a.ket_qua)] {
            p.image(tex.id(), o, uv, Color32::WHITE);
            p.rect_stroke(o, 0.0, Stroke::new(1.0, VIEN_NHAT), egui::StrokeKind::Outside);
        }
        p.text(o_goc.left_top() + egui::vec2(0.0, -2.0), egui::Align2::LEFT_BOTTOM, "Gốc", egui::FontId::proportional(11.0), CHU_PHU);
        p.text(o_kq.left_top() + egui::vec2(0.0, -2.0), egui::Align2::LEFT_BOTTOM, "Kết quả", egui::FontId::proportional(11.0), CHU_PHU);
        // Khung vùng trên ảnh gốc (đúng khung xử lý, kể cả phần nới ra mép)
        for z in self.vung().iter().filter(|z| z.ap_trang.ap_dung(self.trang)) {
            let Some(k) = z.khung_xoa(a.co_anh.0 as i64, a.co_anh.1 as i64, a.dpi as f64) else { continue };
            let r = Rect::from_min_max(o_goc.min + egui::vec2(k.x1 as f32 * ty, k.y1 as f32 * ty), o_goc.min + egui::vec2(k.x2 as f32 * ty, k.y2 as f32 * ty));
            let mau = match z.loai {
                LoaiVung::Xoa => Color32::from_rgb(0x2F, 0x7B, 0xD9),
                LoaiVung::BaoVe => Color32::from_rgb(0x00, 0x99, 0x44),
                LoaiVung::VoDoi => Color32::from_rgb(0xC4, 0x2B, 0x1C),
            };
            p.rect_filled(r, 0.0, mau.gamma_multiply(0.12));
            p.rect_stroke(r, 0.0, Stroke::new(1.5, mau), egui::StrokeKind::Inside);
        }
    }
}

/// Đường dẫn thư mục kết quả mặc định của nguồn (cạnh nguồn).
pub fn thu_muc_ket_qua(nguon: &Path) -> Result<PathBuf, String> {
    crate::tach_vb::tach::thu_muc_ra_hau_to(nguon, None, HAU_TO)
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
