//! Vẽ trang mô phỏng (đoạn văn + bảng có viền) trên nền xám; biến `{x}` thành chip tô màu theo loại.
//! Rê chuột / bấm chip trả về tên biến để định vị hai chiều với sidebar.

use std::sync::Arc;

use eframe::egui::{self, Align, Color32, Rect, Sense, Stroke};

use crate::muc_luc::mo_phong::{Can, Doan, Khoi};
use crate::muc_luc::token;

/// Bề rộng trang ở 100 % (tỷ lệ A4 dọc).
const RONG_TRANG: f32 = 560.0;
const CAO_TRANG_MIN: f32 = RONG_TRANG * 1.414;
const LE_TRANG: f32 = 40.0;
const CO_CHU: f32 = 11.0;
const LE_O: f32 = 4.0;
/// Chip đang định vị (rê chuột ở sidebar hoặc trên trang): vàng.
pub const NEN_SANG: Color32 = Color32::from_rgb(0xFF, 0xE0, 0x66);
pub const CHU_SANG: Color32 = Color32::from_rgb(0x3A, 0x2E, 0x00);

/// Tam giác cảnh báo vẽ tay (font giao diện không có ký tự ⚠), căn giữa ô `r`.
pub fn tam_giac_canh_bao(p: &egui::Painter, r: Rect, mau: Color32) {
    let c = r.center();
    let s = r.width().min(r.height()) * 0.5;
    let dinh = [egui::pos2(c.x, c.y - s), egui::pos2(c.x + s * 1.05, c.y + s * 0.8), egui::pos2(c.x - s * 1.05, c.y + s * 0.8)];
    p.add(egui::Shape::closed_line(dinh.to_vec(), Stroke::new(1.3, mau)));
    p.line_segment([egui::pos2(c.x, c.y - s * 0.35), egui::pos2(c.x, c.y + s * 0.25)], Stroke::new(1.3, mau));
    p.circle_filled(egui::pos2(c.x, c.y + s * 0.52), 0.9, mau);
}

#[derive(Default)]
pub struct PhanHoiTrang {
    pub re: Option<String>,
    pub bam: Option<String>,
}

/// `mau_chip(ten)` = (nền, chữ) của chip biến; `None` → hiển thị chữ thường (trang kết quả đã ghép).
pub struct KieuVe<'a> {
    pub zoom: f32,
    pub mau_chip: Option<&'a dyn Fn(&str) -> (Color32, Color32)>,
    pub sang: Option<&'a str>,
}

/// Một khối chữ đã dàn + vị trí (theo ký tự) của từng chip để dò chuột.
struct ChuDaDan {
    galley: Arc<egui::Galley>,
    /// Lệch ngang của điểm neo so với mép trái vùng chữ (căn giữa / phải neo ở giữa / mép phải).
    lech: f32,
    chip: Vec<(usize, usize, String)>,
}

fn dan(ui: &egui::Ui, d: &Doan, rong: f32, kieu: &KieuVe) -> ChuDaDan {
    let co = CO_CHU * kieu.zoom;
    let font = if d.dam { egui::FontId::new(co, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())) } else { egui::FontId::proportional(co) };
    let thuong = egui::TextFormat { font_id: font.clone(), color: Color32::from_gray(0x11), ..Default::default() };
    let mut job = egui::text::LayoutJob { halign: match d.can { Can::Trai => Align::LEFT, Can::Giua => Align::Center, Can::Phai => Align::RIGHT }, ..Default::default() };
    job.wrap.max_width = rong;
    let mut chip = Vec::new();
    let mut dau = 0;
    let mut so_ky_tu = 0;
    let them = |job: &mut egui::text::LayoutJob, s: &str, f: egui::TextFormat, so_ky_tu: &mut usize| {
        job.append(s, 0.0, f);
        *so_ky_tu += s.chars().count();
    };
    for (vt, dai, ten) in token::tim(&d.chu) {
        match kieu.mau_chip {
            Some(mau) => {
                them(&mut job, &d.chu[dau..vt], thuong.clone(), &mut so_ky_tu);
                let (nen, chu) = if kieu.sang == Some(ten) { (NEN_SANG, CHU_SANG) } else { mau(ten) };
                let f = egui::TextFormat { font_id: egui::FontId::monospace(co * 0.85), color: chu, background: nen, ..Default::default() };
                let bat_dau = so_ky_tu;
                them(&mut job, &d.chu[vt..vt + dai], f, &mut so_ky_tu);
                chip.push((bat_dau, so_ky_tu, ten.to_string()));
            }
            None => them(&mut job, &d.chu[dau..vt + dai], thuong.clone(), &mut so_ky_tu),
        }
        dau = vt + dai;
    }
    them(&mut job, &d.chu[dau..], thuong, &mut so_ky_tu);
    let lech = match d.can {
        Can::Trai => 0.0,
        Can::Giua => rong / 2.0,
        Can::Phai => rong,
    };
    ChuDaDan { galley: ui.fonts_mut(|f| f.layout_job(job)), lech, chip }
}

/// Chip dưới con trỏ (nếu có) trong khối chữ vẽ tại `goc`.
fn chip_duoi_chuot(ui: &egui::Ui, c: &ChuDaDan, goc: egui::Pos2) -> Option<String> {
    let p = ui.input(|i| i.pointer.hover_pos())?;
    if c.chip.is_empty() || !c.galley.rect.translate(goc.to_vec2()).expand(2.0).contains(p) {
        return None;
    }
    let k = c.galley.cursor_from_pos(p - goc).index.0;
    c.chip.iter().find(|(a, b, _)| (*a..*b).contains(&k)).map(|(_, _, t)| t.clone())
}

/// Vẽ trang vào vùng cuộn (tự cuộn); `canh_bao` = dải vàng phía trên trang.
pub fn ve(ui: &mut egui::Ui, id: &str, khoi: &[Khoi], kieu: &KieuVe, canh_bao: Option<&str>) -> PhanHoiTrang {
    let mut ph = PhanHoiTrang::default();
    let z = kieu.zoom;
    ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, Color32::from_gray(0xE6));
    egui::ScrollArea::both().id_salt(id).auto_shrink([false; 2]).show(ui, |ui| {
        ui.add_space(18.0);
        let rong_trang = RONG_TRANG * z;
        let le_ngang = ((ui.available_width() - rong_trang) / 2.0).max(12.0);
        if let Some(cb) = canh_bao {
            ui.horizontal(|ui| {
                ui.add_space(le_ngang);
                egui::Frame::new().fill(Color32::from_rgb(0xFF, 0xF6, 0xE0)).stroke(Stroke::new(1.0, Color32::from_rgb(0xE8, 0xC7, 0x7A))).inner_margin(egui::Margin::symmetric(10, 7)).show(ui, |ui| {
                    ui.set_width(rong_trang - 20.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                        tam_giac_canh_bao(ui.painter(), r, Color32::from_rgb(0xB8, 0x5F, 0x00));
                        ui.label(egui::RichText::new(cb).size(12.0).color(Color32::from_rgb(0x6B, 0x4A, 0x00)));
                    });
                });
            });
            ui.add_space(10.0);
        }
        let le = LE_TRANG * z;
        let rong = rong_trang - 2.0 * le;
        // Dàn trước để biết chiều cao trang
        enum DaDan {
            Doan(ChuDaDan),
            Bang(Vec<(Vec<ChuDaDan>, f32)>, bool),
        }
        let mut ds = Vec::new();
        let mut cao = 0.0;
        for k in khoi {
            match k {
                Khoi::Doan(d) => {
                    let c = dan(ui, d, rong, kieu);
                    cao += c.galley.size().y.max(CO_CHU * z * 1.3) + 2.0 * z;
                    ds.push(DaDan::Doan(c));
                }
                Khoi::Bang { hang, vien } => {
                    let mut ds_hang = Vec::new();
                    for h in hang {
                        let n = h.len().max(1) as f32;
                        let rong_o = rong / n - 2.0 * LE_O * z;
                        let o: Vec<ChuDaDan> = h
                            .iter()
                            .map(|o| {
                                let gop = Doan { chu: o.iter().map(|d| d.chu.as_str()).collect::<Vec<_>>().join("\n"), can: o.first().map_or(Can::Trai, |d| d.can), dam: !o.is_empty() && o.iter().all(|d| d.dam || d.chu.trim().is_empty()) };
                                dan(ui, &gop, rong_o.max(10.0), kieu)
                            })
                            .collect();
                        let cao_h = o.iter().map(|c| c.galley.size().y).fold(CO_CHU * z * 1.3, f32::max) + 2.0 * LE_O * z;
                        cao += cao_h;
                        ds_hang.push((o, cao_h));
                    }
                    cao += 6.0 * z;
                    ds.push(DaDan::Bang(ds_hang, *vien));
                }
            }
        }
        let cao_trang = (cao + 2.0 * le).max(CAO_TRANG_MIN * z);
        let (_, vung) = ui.horizontal(|ui| {
            ui.add_space(le_ngang);
            ui.allocate_exact_size(egui::vec2(rong_trang, cao_trang), Sense::click())
        })
        .inner;
        let p = ui.painter_at(vung.rect.expand(4.0));
        p.rect_filled(vung.rect.translate(egui::vec2(1.0, 2.0)), 0.0, Color32::from_black_alpha(40));
        p.rect_filled(vung.rect, 0.0, Color32::WHITE);
        let mut y = vung.rect.top() + le;
        let x0 = vung.rect.left() + le;
        let vien = Stroke::new(0.8, Color32::from_gray(0x33));
        for d in &ds {
            match d {
                DaDan::Doan(c) => {
                    let goc = egui::pos2(x0 + c.lech, y);
                    p.galley(goc, c.galley.clone(), Color32::BLACK);
                    if let Some(t) = chip_duoi_chuot(ui, c, goc) {
                        ph.re = Some(t);
                    }
                    y += c.galley.size().y.max(CO_CHU * z * 1.3) + 2.0 * z;
                }
                DaDan::Bang(ds_hang, co_vien) => {
                    for (o, cao_h) in ds_hang {
                        let rong_o = rong / o.len().max(1) as f32;
                        for (i, c) in o.iter().enumerate() {
                            let r = Rect::from_min_size(egui::pos2(x0 + i as f32 * rong_o, y), egui::vec2(rong_o, *cao_h));
                            if *co_vien {
                                p.rect_stroke(r, 0.0, vien, egui::StrokeKind::Middle);
                            }
                            let goc = r.min + egui::vec2(LE_O * z + c.lech, LE_O * z);
                            p.galley(goc, c.galley.clone(), Color32::BLACK);
                            if let Some(t) = chip_duoi_chuot(ui, c, goc) {
                                ph.re = Some(t);
                            }
                        }
                        y += cao_h;
                    }
                    y += 6.0 * z;
                }
            }
        }
        if ph.re.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if vung.clicked() {
                ph.bam = ph.re.clone();
            }
        }
        ui.add_space(18.0);
    });
    ph
}
