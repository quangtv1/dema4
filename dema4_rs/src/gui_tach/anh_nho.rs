//! Ảnh nhỏ cho lưới: luồng render MuPDF riêng chỉ làm các ô đang hiện (mỗi khung giao diện thay cả danh sách
//! yêu cầu → ô đã cuộn qua bị bỏ), texture giữ trong LRU có giới hạn để RAM không tăng theo số trang đã cuộn.

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};

/// Bậc độ phân giải theo bề ngang ảnh (px thật): 0–1 ảnh nhỏ lưới, 2–4 xem trang / phóng to.
pub const BE_NGANG: [u32; 5] = [120, 240, 480, 960, 1600];
pub const SO_TEXTURE_TOI_DA: usize = 600;
/// Ảnh lớn (bậc ≥ 2) tốn RAM (A4 960 px ≈ 5 MB) → LRU riêng, giới hạn nhỏ.
pub const SO_ANH_LON_TOI_DA: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KhoaAnh {
    /// Chỉ số trang trong bảng.
    pub trang: u32,
    pub bac: u8,
}

#[derive(Debug, Clone)]
pub struct YeuCau {
    pub khoa: KhoaAnh,
    pub file: usize,
    /// 1-based
    pub so: usize,
}

#[derive(Default)]
struct HangCho {
    ds: Vec<YeuCau>,
    dang_lam: HashSet<KhoaAnh>,
    dung: bool,
}

/// Bậc nhỏ nhất đủ nét cho ô rộng `rong_px` px thật (cho phép phóng ~10 %).
pub fn bac_cho(rong_px: f32) -> u8 {
    BE_NGANG.iter().position(|&b| rong_px <= b as f32 * 1.1).unwrap_or(BE_NGANG.len() - 1) as u8
}

/// Bộ nhớ texture LRU (thuần, để test): bỏ mục dùng lâu nhất khi vượt giới hạn.
pub struct Lru<T> {
    muc: HashMap<KhoaAnh, (T, u64)>,
    nhip: u64,
    gioi_han: usize,
}

impl<T> Lru<T> {
    pub fn moi(gioi_han: usize) -> Self {
        Self { muc: HashMap::new(), nhip: 0, gioi_han }
    }

    pub fn lay(&mut self, k: &KhoaAnh) -> Option<&T> {
        self.nhip += 1;
        let nhip = self.nhip;
        self.muc.get_mut(k).map(|(t, dung)| {
            *dung = nhip;
            &*t
        })
    }

    pub fn co(&self, k: &KhoaAnh) -> bool {
        self.muc.contains_key(k)
    }

    pub fn them(&mut self, k: KhoaAnh, t: T) {
        self.nhip += 1;
        self.muc.insert(k, (t, self.nhip));
        while self.muc.len() > self.gioi_han {
            let cu = *self.muc.iter().min_by_key(|(_, (_, d))| *d).map(|(k, _)| k).unwrap();
            self.muc.remove(&cu);
        }
    }

    pub fn len(&self) -> usize {
        self.muc.len()
    }

    pub fn xoa_het(&mut self) {
        self.muc.clear();
    }
}

pub struct AnhNho {
    hang: Arc<(Mutex<HangCho>, Condvar)>,
    nhan: Receiver<(KhoaAnh, ColorImage)>,
    lru: Lru<TextureHandle>,
    lru_lon: Lru<TextureHandle>,
    /// Đã yêu cầu ở khung trước (tránh gửi lại y nguyên).
    da_gui: Vec<KhoaAnh>,
}

impl AnhNho {
    pub fn moi(pdf: Vec<PathBuf>, so_luong: usize, ctx: egui::Context) -> Self {
        let hang = Arc::new((Mutex::new(HangCho::default()), Condvar::new()));
        let (gui, nhan) = std::sync::mpsc::channel();
        let pdf = Arc::new(pdf);
        for _ in 0..so_luong.max(1) {
            let (hang, gui, pdf, ctx) = (hang.clone(), gui.clone(), pdf.clone(), ctx.clone());
            std::thread::spawn(move || lam_viec(&hang, &pdf, &gui, &ctx));
        }
        Self { hang, nhan, lru: Lru::moi(SO_TEXTURE_TOI_DA), lru_lon: Lru::moi(SO_ANH_LON_TOI_DA), da_gui: Vec::new() }
    }

    fn bo(&mut self, k: &KhoaAnh) -> &mut Lru<TextureHandle> {
        if k.bac >= 2 { &mut self.lru_lon } else { &mut self.lru }
    }

    pub fn co(&self, k: &KhoaAnh) -> bool {
        if k.bac >= 2 { self.lru_lon.co(k) } else { self.lru.co(k) }
    }

    /// Ảnh đúng bậc nếu có, không thì ảnh bậc thấp hơn gần nhất (hiện tạm trong lúc chờ render).
    pub fn lay(&mut self, trang: u32, bac: u8) -> Option<TextureHandle> {
        (0..=bac).rev().find_map(|b| {
            let k = KhoaAnh { trang, bac: b };
            self.bo(&k).lay(&k).cloned()
        })
    }

    /// Thay danh sách cần render bằng các ô đang hiện còn thiếu ảnh (thứ tự hiện = thứ tự render).
    pub fn yeu_cau(&mut self, ds: Vec<YeuCau>) {
        let ds: Vec<YeuCau> = ds.into_iter().filter(|y| !self.co(&y.khoa)).collect();
        let khoa: Vec<KhoaAnh> = ds.iter().map(|y| y.khoa).collect();
        if khoa == self.da_gui {
            return;
        }
        self.da_gui = khoa;
        let (m, cv) = &*self.hang;
        let mut h = m.lock().unwrap();
        h.ds = ds.into_iter().filter(|y| !h.dang_lam.contains(&y.khoa)).collect();
        h.ds.reverse(); // lấy từ cuối
        cv.notify_all();
    }

    /// Nhận ảnh đã render → texture.
    pub fn nhan_anh(&mut self, ctx: &egui::Context) {
        while let Ok((k, anh)) = self.nhan.try_recv() {
            let tex = ctx.load_texture(format!("anh_nho_{}_{}", k.trang, k.bac), anh, TextureOptions::LINEAR);
            self.bo(&k).them(k, tex);
        }
    }

    /// Ảnh xám có sẵn từ phân tích (đỡ render lại khi đang hiện).
    pub fn them_anh_xam(&mut self, ctx: &egui::Context, k: KhoaAnh, rong: usize, cao: usize, xam: &[u8]) {
        if self.co(&k) || xam.len() != rong * cao {
            return;
        }
        let anh = ColorImage::from_gray([rong, cao], xam);
        let tex = ctx.load_texture(format!("anh_nho_{}_{}", k.trang, k.bac), anh, TextureOptions::LINEAR);
        self.lru.them(k, tex);
    }
}

impl Drop for AnhNho {
    fn drop(&mut self) {
        let (m, cv) = &*self.hang;
        m.lock().unwrap().dung = true;
        cv.notify_all();
    }
}

fn lam_viec(hang: &(Mutex<HangCho>, Condvar), pdf: &[PathBuf], gui: &Sender<(KhoaAnh, ColorImage)>, ctx: &egui::Context) {
    let mut dang_mo: Option<(usize, mupdf::Document)> = None;
    loop {
        let y = {
            let (m, cv) = hang;
            let mut h = m.lock().unwrap();
            loop {
                if h.dung {
                    return;
                }
                if let Some(y) = h.ds.pop() {
                    h.dang_lam.insert(y.khoa);
                    break y;
                }
                h = cv.wait(h).unwrap();
            }
        };
        if dang_mo.as_ref().is_none_or(|(f, _)| *f != y.file) {
            dang_mo = mupdf::Document::open(pdf[y.file].to_string_lossy().as_ref()).ok().map(|d| (y.file, d));
        }
        let anh = dang_mo.as_ref().and_then(|(_, d)| render(d, y.so, BE_NGANG[y.khoa.bac as usize]));
        hang.0.lock().unwrap().dang_lam.remove(&y.khoa);
        if let Some(anh) = anh {
            if gui.send((y.khoa, anh)).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    }
}

fn render(doc: &mupdf::Document, so: usize, rong: u32) -> Option<ColorImage> {
    let trang = doc.load_page(so as i32 - 1).ok()?;
    let hop = trang.bounds().ok()?;
    let ty_le = rong as f32 / (hop.x1 - hop.x0).max(1.0);
    let pix = trang.to_pixmap(&mupdf::Matrix::new_scale(ty_le, ty_le), &mupdf::Colorspace::device_rgb(), false, false).ok()?;
    let (w, h, n) = (pix.width() as usize, pix.height() as usize, pix.n() as usize);
    let rgb: Vec<u8> = if n == 3 { pix.samples().to_vec() } else { pix.samples().chunks_exact(n).flat_map(|p| [p[0], p[1], p[2]]).collect() };
    Some(ColorImage::from_rgb([w, h], &rgb))
}

/// Vẽ texture vừa khít `hop`, xoay `goc` (0/90/180/270, theo kim) bằng toạ độ UV — không render lại.
pub fn ve_xoay(painter: &egui::Painter, hop: Rect, tex: &TextureHandle, goc: u16) -> Rect {
    let [w, h] = tex.size().map(|x| x as f32);
    let (w, h) = if goc % 180 == 90 { (h, w) } else { (w, h) };
    let ty_le = (hop.width() / w).min(hop.height() / h);
    let r = Rect::from_center_size(hop.center(), egui::vec2(w * ty_le, h * ty_le));
    // UV của 4 góc ảnh gốc theo thứ tự TL, TR, BR, BL; xoay theo kim = góc hiển thị lấy UV lùi 1 bước
    let uv = [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)];
    let buoc = (goc / 90) as usize % 4;
    let goc_hien = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
    let mut mesh = egui::Mesh::with_texture(tex.id());
    for (i, p) in goc_hien.iter().enumerate() {
        mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv: uv[(i + 4 - buoc) % 4], color: Color32::WHITE });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(t: u32) -> KhoaAnh {
        KhoaAnh { trang: t, bac: 0 }
    }

    #[test]
    fn lru_bo_muc_dung_lau_nhat() {
        let mut l = Lru::moi(3);
        for t in 0..3 {
            l.them(k(t), t);
        }
        assert_eq!(l.lay(&k(0)), Some(&0)); // 0 vừa dùng → 1 là cũ nhất
        l.them(k(3), 3);
        assert_eq!(l.len(), 3);
        assert!(!l.co(&k(1)) && l.co(&k(0)) && l.co(&k(3)));
        // Cuộn lại ô còn trong cache: không cần render lại
        assert_eq!(l.lay(&k(2)), Some(&2));
    }

    #[test]
    fn bac_anh_theo_be_ngang_o() {
        assert_eq!(bac_cho(56.0), 0);
        assert_eq!(bac_cho(130.0), 0);
        assert_eq!(bac_cho(200.0), 1);
        assert_eq!(bac_cho(900.0), 3);
        assert_eq!(bac_cho(5000.0), 4);
    }

    #[test]
    fn render_trang_pdf_dung_be_ngang() {
        let tep = std::env::temp_dir().join(format!("dema4_anh_nho_{}.pdf", std::process::id()));
        crate::tach_vb::tach::tests::tao_pdf(&tep, 2);
        let doc = mupdf::Document::open(tep.to_str().unwrap()).unwrap();
        let anh = render(&doc, 2, 120).unwrap();
        assert_eq!(anh.size[0], 120);
        assert!(anh.size[1].abs_diff((120.0f32 * 842.0 / 595.0).round() as usize) <= 1, "{:?}", anh.size);
        assert!(render(&doc, 3, 120).is_none(), "trang không có");
        let _ = std::fs::remove_file(&tep);
    }
}
