//! Luồng xem trước Xóa ghim: vẽ trang ở **đúng DPI sẽ xuất** (`xuat::ve_trang`) rồi xoá ghim bằng đúng hàm của
//! lúc xuất → ảnh Kết quả = ảnh ghi vào PDF (trước nén JPEG). Giữ PDF đang mở + ảnh gốc trang vừa vẽ nên đổi vùng /
//! ngưỡng chỉ chạy lại bước xoá (vài chục ms). Yêu cầu dồn (kéo thanh trượt) → chỉ làm yêu cầu mới nhất.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;

use crate::tach_vb::anh::Anh;
use crate::xoa_ghim::vung::Vung;
use crate::xoa_ghim::xu_ly::{xoa_ghim_trang, TuyChon};

/// Cạnh dài tối đa của ảnh hiển thị (giới hạn texture); ảnh lớn hơn thu nhỏ **sau** khi xoá ghim.
const CANH_HIEN_TOI_DA: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct YeuCau {
    pub so: u64,
    pub pdf: PathBuf,
    pub trang: u32,
    /// Vùng áp cho file (chung rồi riêng), đúng thứ tự xuất.
    pub vung: Vec<Vung>,
    pub tc: TuyChon,
    /// 0 = theo ảnh scan.
    pub dpi_chon: u16,
}

pub struct KetQua {
    pub so: u64,
    pub so_trang: u32,
    pub dpi: u16,
    /// Cỡ ảnh trang ở DPI xuất (điểm ảnh) — để vẽ khung vùng đúng chỗ.
    pub co_anh: (usize, usize),
    pub goc: egui::ColorImage,
    pub ket_qua: egui::ColorImage,
}

pub struct XemTruoc {
    gui: Sender<YeuCau>,
    nhan: Receiver<Result<KetQua, (u64, String)>>,
}

impl Default for XemTruoc {
    fn default() -> Self {
        let (gui, nhan_yc) = mpsc::channel::<YeuCau>();
        let (gui_kq, nhan) = mpsc::channel();
        std::thread::Builder::new().name("xem-truoc-xoa-ghim".into()).spawn(move || luong(nhan_yc, gui_kq)).expect("tạo luồng xem trước");
        XemTruoc { gui, nhan }
    }
}

impl XemTruoc {
    pub fn gui(&self, yc: YeuCau) {
        let _ = self.gui.send(yc);
    }

    pub fn nhan(&self) -> Option<Result<KetQua, (u64, String)>> {
        let mut cuoi = None;
        while let Ok(k) = self.nhan.try_recv() {
            cuoi = Some(k);
        }
        cuoi
    }
}

fn anh_hien(a: &Anh) -> egui::ColorImage {
    let dai = a.rong.max(a.cao);
    let a = if dai > CANH_HIEN_TOI_DA { a.thu_ve(a.rong * CANH_HIEN_TOI_DA / dai, a.cao * CANH_HIEN_TOI_DA / dai) } else { a.clone() };
    egui::ColorImage::from_rgb([a.rong, a.cao], &a.du_lieu)
}

/// Ảnh gốc + ảnh kết quả của 1 yêu cầu — tách riêng để test so với ảnh lúc xuất.
pub fn xu_ly(doc: &mupdf::pdf::PdfDocument, yc: &YeuCau, nho: &mut Option<(PathBuf, u32, u16, Anh, u16)>) -> Result<(Anh, Anh, u16), String> {
    let (goc, dpi) = match nho {
        Some((p, t, chon, a, dpi)) if *p == yc.pdf && *t == yc.trang && *chon == yc.dpi_chon => (a.clone(), *dpi),
        _ => {
            let (a, dpi, _) = crate::xoa_ghim::xuat::ve_trang(doc, yc.trang as usize, yc.dpi_chon)?;
            *nho = Some((yc.pdf.clone(), yc.trang, yc.dpi_chon, a.clone(), dpi));
            (a, dpi)
        }
    };
    let mut kq = goc.clone();
    xoa_ghim_trang(&mut kq, &yc.vung, yc.trang, dpi as f64, &yc.tc, &[]);
    Ok((goc, kq, dpi))
}

fn luong(nhan: Receiver<YeuCau>, gui: Sender<Result<KetQua, (u64, String)>>) {
    let mut doc: Option<(PathBuf, mupdf::pdf::PdfDocument)> = None;
    let mut nho = None;
    while let Ok(mut yc) = nhan.recv() {
        while let Ok(moi) = nhan.try_recv() {
            yc = moi;
        }
        if doc.as_ref().is_none_or(|(p, _)| *p != yc.pdf) {
            nho = None;
            doc = match mupdf::pdf::PdfDocument::open(yc.pdf.to_string_lossy().as_ref()) {
                Ok(d) => Some((yc.pdf.clone(), d)),
                Err(e) => {
                    let _ = gui.send(Err((yc.so, format!("Không mở được PDF: {e}"))));
                    continue;
                }
            };
        }
        let (_, d) = doc.as_ref().unwrap();
        let so_trang = d.page_count().unwrap_or(0).max(0) as u32;
        if yc.trang >= so_trang {
            let _ = gui.send(Err((yc.so, "PDF không có trang này".into())));
            continue;
        }
        let kq = xu_ly(d, &yc, &mut nho).map(|(goc, kq, dpi)| KetQua { so: yc.so, so_trang, dpi, co_anh: (goc.rong, goc.cao), goc: anh_hien(&goc), ket_qua: anh_hien(&kq) });
        let _ = gui.send(kq.map_err(|e| (yc.so, e)));
    }
}
