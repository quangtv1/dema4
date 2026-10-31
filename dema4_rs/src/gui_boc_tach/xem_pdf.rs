//! Khung xem PDF của màn Bóc tách: các trang xếp dọc, rộng theo khung × thu phóng; trang render ở luồng nền
//! (1 luồng, giữ PDF đang mở), chỉ trang đang thấy mới render, ảnh theo bậc bề rộng để không render lại khi kéo nhẹ.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui::{self, Color32, Rect, Sense, Stroke, TextureHandle};

use crate::gui::{CHU_PHU, VIEN_NHAT};

/// Nấc thu phóng (%).
pub const NAC: [u16; 8] = [50, 67, 75, 100, 125, 150, 200, 300];
const KHE: f32 = 8.0;
const LE: f32 = 10.0;
const NEN_KHUNG: Color32 = Color32::from_rgb(0xE9, 0xE9, 0xE9);

enum Tin {
    Mo(u64, PathBuf),
    Ve(u64, usize, u32),
}

enum Ve {
    KichThuoc(u64, Vec<(f32, f32)>),
    Anh(u64, usize, u32, egui::ColorImage),
    Loi(u64, String),
}

pub struct XemPdf {
    pdf: Option<PathBuf>,
    /// Kích thước trang (pt); trống = đang mở.
    kich_thuoc: Vec<(f32, f32)>,
    anh: HashMap<(usize, u32), TextureHandle>,
    cho: HashSet<(usize, u32)>,
    loi: Option<String>,
    the_he: u64,
    gui: Sender<Tin>,
    nhan: Receiver<Ve>,
    pub nac: usize,
}

impl Default for XemPdf {
    fn default() -> Self {
        Self::moi()
    }
}

impl XemPdf {
    pub fn moi() -> Self {
        let (gui, nhan_tin) = mpsc::channel::<Tin>();
        let (gui_ve, nhan) = mpsc::channel::<Ve>();
        std::thread::spawn(move || luong_ve(nhan_tin, gui_ve));
        Self { pdf: None, kich_thuoc: vec![], anh: HashMap::new(), cho: HashSet::new(), loi: None, the_he: 0, gui, nhan, nac: 3 }
    }

    pub fn ty_le(&self) -> u16 {
        NAC[self.nac]
    }

    /// Mở file khác (cùng file → không làm gì).
    pub fn mo(&mut self, pdf: Option<&Path>) {
        if self.pdf.as_deref() == pdf {
            return;
        }
        self.the_he += 1;
        self.pdf = pdf.map(Path::to_path_buf);
        self.kich_thuoc.clear();
        self.anh.clear();
        self.cho.clear();
        self.loi = None;
        if let Some(p) = &self.pdf {
            let _ = self.gui.send(Tin::Mo(self.the_he, p.clone()));
        }
    }

    fn nhan_anh(&mut self, ctx: &egui::Context) {
        while let Ok(v) = self.nhan.try_recv() {
            match v {
                Ve::KichThuoc(t, k) if t == self.the_he => self.kich_thuoc = k,
                Ve::Anh(t, i, bac, anh) if t == self.the_he => {
                    self.cho.remove(&(i, bac));
                    let tex = ctx.load_texture(format!("bt_trang_{t}_{i}_{bac}"), anh, egui::TextureOptions::LINEAR);
                    self.anh.insert((i, bac), tex);
                }
                Ve::Loi(t, e) if t == self.the_he => self.loi = Some(e),
                _ => {}
            }
        }
    }

    pub fn ve(&mut self, ui: &mut egui::Ui) {
        self.nhan_anh(ui.ctx());
        let khung = ui.available_rect_before_wrap();
        ui.painter().rect_filled(khung, 0.0, NEN_KHUNG);
        if self.pdf.is_none() {
            return;
        }
        if let Some(e) = &self.loi {
            ui.painter().text(khung.center(), egui::Align2::CENTER_CENTER, e, egui::FontId::proportional(12.0), CHU_PHU);
            return;
        }
        if self.kich_thuoc.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let rong = ((khung.width() - 2.0 * LE) * self.ty_le() as f32 / 100.0).max(40.0);
        let tong_cao: f32 = self.kich_thuoc.iter().map(|(w, h)| rong * h / w + KHE).sum::<f32>() + LE;
        let so = self.kich_thuoc.len();
        egui::ScrollArea::both().id_salt(("bt_xem_pdf", self.the_he)).auto_shrink([false, false]).show(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2((rong + 2.0 * LE).max(khung.width()), tong_cao), Sense::hover());
            let x0 = r.left() + ((r.width() - rong) / 2.0).max(LE);
            let mut y = r.top() + LE;
            let mut can = Vec::new();
            for (i, (w, h)) in self.kich_thuoc.iter().enumerate() {
                let o = Rect::from_min_size(egui::pos2(x0, y), egui::vec2(rong, rong * h / w));
                y += o.height() + KHE;
                if !ui.is_rect_visible(o) {
                    continue;
                }
                let p = ui.painter();
                p.rect_filled(o.translate(egui::vec2(0.0, 1.0)), 0.0, Color32::from_black_alpha(20));
                p.rect_filled(o, 0.0, Color32::WHITE);
                // Bậc ảnh: bề rộng px làm tròn lên bội 256 → kéo khung nhẹ không render lại
                let bac = ((o.width() * ppp / 256.0).ceil() as u32 * 256).clamp(256, 4096);
                match self.anh.get(&(i, bac)).or_else(|| self.anh.iter().find(|((t, _), _)| *t == i).map(|(_, tex)| tex)) {
                    Some(tex) => {
                        p.image(tex.id(), o, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    None => {}
                }
                if !self.anh.contains_key(&(i, bac)) && !self.cho.contains(&(i, bac)) {
                    can.push((i, bac));
                }
                p.rect_stroke(o, 0.0, Stroke::new(1.0, VIEN_NHAT), egui::StrokeKind::Outside);
                p.text(o.right_top() + egui::vec2(-8.0, 6.0), egui::Align2::RIGHT_TOP, format!("{}/{so}", i + 1), egui::FontId::proportional(10.5), CHU_PHU);
            }
            for (i, bac) in can {
                self.cho.insert((i, bac));
                let _ = self.gui.send(Tin::Ve(self.the_he, i, bac));
            }
        });
        if !self.cho.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}

/// Luồng render: giữ PDF đang mở; bỏ yêu cầu của file cũ (thế hệ nhỏ hơn).
fn luong_ve(nhan: Receiver<Tin>, gui: Sender<Ve>) {
    let mut dang: Option<(u64, mupdf::Document)> = None;
    while let Ok(tin) = nhan.recv() {
        match tin {
            Tin::Mo(t, p) => match mupdf::Document::open(p.as_path()) {
                Ok(doc) => {
                    let n = doc.page_count().unwrap_or(0).max(0) as usize;
                    let k = (0..n)
                        .map(|i| doc.load_page(i as i32).and_then(|pg| pg.bounds()).map(|b| ((b.x1 - b.x0).max(1.0), (b.y1 - b.y0).max(1.0))).unwrap_or((595.0, 842.0)))
                        .collect();
                    let _ = gui.send(Ve::KichThuoc(t, k));
                    dang = Some((t, doc));
                }
                Err(e) => {
                    let _ = gui.send(Ve::Loi(t, format!("Không mở được PDF: {e}")));
                    dang = None;
                }
            },
            Tin::Ve(t, i, bac) => {
                let Some((td, doc)) = &dang else { continue };
                if *td != t {
                    continue;
                }
                let Ok(pg) = doc.load_page(i as i32) else { continue };
                let Ok(b) = pg.bounds() else { continue };
                let s = bac as f32 / (b.x1 - b.x0).max(1.0);
                let Ok(pix) = pg.to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_rgb(), false, false) else { continue };
                let (w, h, n) = (pix.width() as usize, pix.height() as usize, pix.n() as usize);
                let rgb: Vec<u8> = if n == 3 { pix.samples()[..w * h * 3].to_vec() } else { pix.samples().chunks_exact(n).flat_map(|c| [c[0], c[1], c[2]]).collect() };
                let _ = gui.send(Ve::Anh(t, i, bac, egui::ColorImage::from_rgb([w, h], &rgb)));
            }
        }
    }
}
