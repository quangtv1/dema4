//! Luồng xem trước Xóa ghim (1 luồng, giữ PDF đang mở):
//! - mở file → khổ từng trang (điểm) để dựng bố cục cuộn liên tục ngay;
//! - ảnh nhỏ từng trang (cạnh ngang `RONG_ANH_NHO`);
//! - trang xem: vẽ ở **đúng DPI sẽ xuất** (`xuat::ve_trang`) rồi xoá ghim bằng đúng hàm lúc xuất → ảnh Đích = ảnh
//!   ghi vào PDF (trước nén JPEG); ảnh hiển thị thu về cạnh dài `CANH_HIEN_TOI_DA` sau khi xoá.
//!
//! Nhớ ảnh gốc vài trang gần nhất (đổi vùng / ngưỡng chỉ chạy lại bước xoá). Yêu cầu dồn: trang trùng lấy yêu cầu
//! mới nhất, làm trang mới yêu cầu trước (trang đang thấy).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;

use crate::tach_vb::anh::Anh;
use crate::xoa_ghim::vung::{Hcn, Vung};
use crate::xoa_ghim::xu_ly::{khung_ai_anh, xoa_ghim_trang, TuyChon};

/// Cạnh dài tối đa của ảnh trang hiển thị (texture); ảnh lớn hơn thu nhỏ **sau** khi xoá ghim.
const CANH_HIEN_TOI_DA: usize = 1800;
/// Bề ngang ảnh nhỏ trang (điểm ảnh).
pub const RONG_ANH_NHO: usize = 120;
/// Số ảnh gốc trang (độ phân giải xuất) giữ trong bộ nhớ.
const NHO_TRANG: usize = 6;

/// Cấu hình xoá của 1 lần xem — đổi cái này thì ảnh Đích phải làm lại.
#[derive(Clone, Debug, PartialEq)]
pub struct CauHinhXem {
    pub vung: Vec<Vung>,
    pub tc: TuyChon,
    /// 0 = theo ảnh scan.
    pub dpi_chon: u16,
}

pub enum YeuCau {
    Mo { the_he: u64, pdf: PathBuf },
    AnhNho { the_he: u64, trang: u32 },
    Trang { the_he: u64, trang: u32, ma: u64, ch: CauHinhXem },
}

pub enum KetQua {
    KhoTrang { the_he: u64, kho: Vec<(f32, f32)> },
    AnhNho { the_he: u64, trang: u32, anh: egui::ColorImage },
    /// `ai`: khung bảo vệ nhận diện layout (điểm ảnh của ảnh `co_anh`), trống khi tắt nhận diện.
    Trang { the_he: u64, trang: u32, ma: u64, dpi: u16, co_anh: (usize, usize), goc: egui::ColorImage, dich: egui::ColorImage, ai: Vec<Hcn> },
    Loi { the_he: u64, loi: String },
}

pub struct XemTruoc {
    gui: Sender<YeuCau>,
    nhan: Receiver<KetQua>,
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

    pub fn nhan(&self) -> Vec<KetQua> {
        self.nhan.try_iter().collect()
    }
}

fn anh_hien(a: &Anh, canh: usize) -> egui::ColorImage {
    let dai = a.rong.max(a.cao);
    let a = if dai > canh { a.thu_ve((a.rong * canh / dai).max(1), (a.cao * canh / dai).max(1)) } else { a.clone() };
    egui::ColorImage::from_rgb([a.rong, a.cao], &a.du_lieu)
}

fn luong(nhan: Receiver<YeuCau>, gui: Sender<KetQua>) {
    let mut doc: Option<(u64, mupdf::pdf::PdfDocument)> = None;
    // (trang, DPI chọn) → (ảnh gốc, DPI đã vẽ); mới dùng ở cuối
    let mut nho: VecDeque<((u32, u16), Anh, u16)> = VecDeque::new();
    // Khung AI theo (trang, DPI chọn) — nhận diện ~0,25 s/trang nên chỉ chạy 1 lần mỗi ảnh gốc
    let mut nho_ai: Vec<((u32, u16), Vec<Hcn>)> = Vec::new();
    let mut cho: Vec<YeuCau> = Vec::new();
    loop {
        if cho.is_empty() {
            match nhan.recv() {
                Ok(y) => cho.push(y),
                Err(_) => return,
            }
        }
        cho.extend(nhan.try_iter());
        // Mở file mới: bỏ mọi yêu cầu trước nó
        if let Some(i) = cho.iter().rposition(|y| matches!(y, YeuCau::Mo { .. })) {
            cho.drain(..i);
        }
        // Yêu cầu mới nhất trước; trang trùng chỉ giữ bản mới nhất
        let yc = cho.pop().unwrap();
        cho.retain(|y| match (&yc, y) {
            (YeuCau::Trang { trang: a, .. }, YeuCau::Trang { trang: b, .. }) => a != b,
            (YeuCau::AnhNho { trang: a, .. }, YeuCau::AnhNho { trang: b, .. }) => a != b,
            _ => true,
        });
        match yc {
            YeuCau::Mo { the_he, pdf } => {
                nho.clear();
                nho_ai.clear();
                doc = None;
                match mupdf::pdf::PdfDocument::open(pdf.to_string_lossy().as_ref()) {
                    Ok(d) => {
                        let n = d.page_count().unwrap_or(0).max(0);
                        let kho = (0..n).map(|i| d.load_page(i).and_then(|p| p.bounds()).map(|b| ((b.x1 - b.x0).max(1.0), (b.y1 - b.y0).max(1.0))).unwrap_or((595.0, 842.0))).collect();
                        let _ = gui.send(KetQua::KhoTrang { the_he, kho });
                        doc = Some((the_he, d));
                    }
                    Err(e) => {
                        let _ = gui.send(KetQua::Loi { the_he, loi: format!("Không mở được PDF: {e}") });
                    }
                }
            }
            YeuCau::AnhNho { the_he, trang } => {
                let Some((t, d)) = &doc else { continue };
                if *t != the_he {
                    continue;
                }
                let Ok(p) = d.load_page(trang as i32) else { continue };
                let Ok(b) = p.bounds() else { continue };
                let dpi = RONG_ANH_NHO as f32 * 72.0 / (b.x1 - b.x0).max(1.0);
                if let Ok(a) = crate::tach_vb::phan_tich::render_rgb(&p, dpi) {
                    let _ = gui.send(KetQua::AnhNho { the_he, trang, anh: anh_hien(&a, 4 * RONG_ANH_NHO) });
                }
            }
            YeuCau::Trang { the_he, trang, ma, ch } => {
                let Some((t, d)) = &doc else { continue };
                if *t != the_he {
                    continue;
                }
                let khoa = (trang, ch.dpi_chon);
                let goc = match nho.iter().position(|(k, _, _)| *k == khoa) {
                    Some(i) => {
                        let x = nho.remove(i).unwrap();
                        nho.push_back(x);
                        let (_, a, dpi) = nho.back().unwrap();
                        Ok((a.clone(), *dpi))
                    }
                    None => crate::xoa_ghim::xuat::ve_trang(d, trang as usize, ch.dpi_chon).map(|(a, dpi, _)| {
                        nho.push_back((khoa, a.clone(), dpi));
                        if nho.len() > NHO_TRANG {
                            nho.pop_front();
                        }
                        (a, dpi)
                    }),
                };
                match goc {
                    Ok((goc, dpi)) => {
                        let mut dich = goc.clone();
                        let ai = if ch.tc.nhan_dien {
                            match nho_ai.iter().find(|(k, _)| *k == khoa) {
                                Some((_, h)) => h.clone(),
                                None => {
                                    // Mọi trang (không chỉ trang có vùng xoá) để hiện khung trên Gốc
                                    let h = khung_ai_anh(&goc);
                                    nho_ai.push((khoa, h.clone()));
                                    if nho_ai.len() > 4 * NHO_TRANG {
                                        nho_ai.remove(0);
                                    }
                                    h
                                }
                            }
                        } else {
                            Vec::new()
                        };
                        let co_xoa = ch.vung.iter().any(|v| v.loai == crate::xoa_ghim::vung::LoaiVung::Xoa && v.ap_trang.ap_dung(trang));
                        xoa_ghim_trang(&mut dich, &ch.vung, trang, dpi as f64, &ch.tc, if co_xoa { &ai } else { &[] });
                        let _ = gui.send(KetQua::Trang { the_he, trang, ma, dpi, co_anh: (goc.rong, goc.cao), goc: anh_hien(&goc, CANH_HIEN_TOI_DA), dich: anh_hien(&dich, CANH_HIEN_TOI_DA), ai });
                    }
                    Err(e) => {
                        let _ = gui.send(KetQua::Loi { the_he, loi: e });
                    }
                }
            }
        }
    }
}
