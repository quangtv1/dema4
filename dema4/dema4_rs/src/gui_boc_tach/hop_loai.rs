//! Popup của tab Trường dữ liệu (prototype `#boc-tach-cau-lenh`, `-sua-loai`, `-xoa-loai`, `-luu-loai-moi`):
//! Câu lệnh gửi mô hình đọc (nút Chép), Chỉnh sửa loại tài liệu, xác nhận Xoá loại / Lưu thành loại mới.

use eframe::egui::{self, Color32, Margin, RichText, Sense, Stroke};

use crate::boc_tach::cau_lenh::cau_lenh;
use crate::boc_tach::loai_tai_lieu::{bien_tu_ten, ChuanHoa, LoaiTaiLieu, TrangDoc, Truong};
use crate::gui::{CHU, CHU_MO, CHU_PHU, DO, VIEN, VIEN_NHAT, XANH_LA};
use crate::gui_tach::khung_hop as kh;

/// Bản đang sửa của 1 loại tài liệu: danh sách giá trị giữ dạng chữ "a, b, c" để gõ thoải mái.
#[derive(Debug, Clone, PartialEq)]
pub struct BanSua {
    pub loai: LoaiTaiLieu,
    pub chi_so: usize,
    ds_chu: Vec<String>,
    keo: Option<usize>,
}

impl BanSua {
    pub fn moi(loai: &LoaiTaiLieu, chi_so: usize) -> Self {
        let ds_chu = loai.truong.iter().map(|t| t.danh_sach.join(", ")).collect();
        Self { loai: loai.clone(), chi_so, ds_chu, keo: None }
    }

    /// Loại đã sửa (danh sách giá trị tách theo dấu phẩy / xuống dòng).
    pub fn ket_qua(&self) -> LoaiTaiLieu {
        let mut l = self.loai.clone();
        for (t, ds) in l.truong.iter_mut().zip(&self.ds_chu) {
            t.danh_sach = ds.split([',', '\n', ';']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect();
        }
        l.ten = l.ten.trim().to_string();
        l
    }

    /// Lỗi chặn Lưu: tên trống, chưa có trường, biến trống / sai ký tự / trùng, Danh sách chưa có giá trị.
    pub fn loi(&self) -> Option<String> {
        let l = self.ket_qua();
        if l.ten.is_empty() {
            return Some("Nhập tên loại".into());
        }
        if l.truong.is_empty() {
            return Some("Cần ít nhất 1 trường".into());
        }
        for (i, t) in l.truong.iter().enumerate() {
            if t.ten_cot.trim().is_empty() {
                return Some(format!("Trường {}: nhập tên cột", i + 1));
            }
            if t.bien.is_empty() || !t.bien.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                return Some(format!("Biến \"{}\": chỉ dùng chữ thường không dấu, số và _", t.bien));
            }
            if l.truong[..i].iter().any(|x| x.bien == t.bien) {
                return Some(format!("Biến \"{}\" bị trùng", t.bien));
            }
            if t.chuan_hoa == ChuanHoa::DanhSach && t.danh_sach.is_empty() {
                return Some(format!("Trường \"{}\": nhập danh sách giá trị", t.ten_cot));
            }
        }
        None
    }

    fn them(&mut self) {
        let n = self.loai.truong.len() + 1;
        let mut t = Truong::moi(&format!("Trường {n}"), "", ChuanHoa::GiuNguyen, false);
        t.bien = format!("truong_{n}");
        self.loai.truong.push(t);
        self.ds_chu.push(String::new());
    }

    fn xoa(&mut self, i: usize) {
        self.loai.truong.remove(i);
        self.ds_chu.remove(i);
    }

    fn doi_cho(&mut self, tu: usize, den: usize) {
        if tu != den && tu < self.loai.truong.len() && den < self.loai.truong.len() {
            let t = self.loai.truong.remove(tu);
            self.loai.truong.insert(den, t);
            let d = self.ds_chu.remove(tu);
            self.ds_chu.insert(den, d);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HanhDong {
    Mo,
    Huy,
    Luu,
    Xoa,
    LuuMoi,
}

fn ten_trang_doc(t: TrangDoc) -> &'static str {
    match t {
        TrangDoc::Dau => "Trang đầu",
        TrangDoc::DauCuoi => "Trang đầu + trang cuối",
        TrangDoc::TatCa => "Mọi trang",
    }
}

/// Popup Câu lệnh: toàn văn câu lệnh trang đầu; `true` = đóng.
pub fn ve_cau_lenh(ctx: &egui::Context, loai: &LoaiTaiLieu, so_trang: usize, da_chep: &mut bool) -> bool {
    let mut dong = false;
    let noi_dung = cau_lenh(loai, 1, so_trang.max(1));
    let rong = 640.0_f32.min(ctx.content_rect().width() - 40.0);
    let ph = kh::hop(ctx, "bt_cau_lenh", rong, format!("Câu lệnh gửi mô hình đọc · {} · trang đầu", loai.ten), |ui| {
        kh::than(ui, "bt_than_cau_lenh", 360.0, Margin::same(14), |ui| {
            egui::Frame::new().fill(Color32::from_gray(0xF7)).stroke(Stroke::new(1.0, VIEN_NHAT)).corner_radius(4).inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new(&noi_dung).monospace().size(11.5).color(CHU)).wrap());
            });
        });
        kh::chan(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(kh::nut_chan("Đóng")).clicked() {
                    dong = true;
                }
                if ui.add(kh::nut_chan(if *da_chep { "Đã chép" } else { "Chép" })).on_hover_text("Chép câu lệnh vào clipboard").clicked() {
                    ui.ctx().copy_text(noi_dung.clone());
                    *da_chep = true;
                }
            });
        });
    });
    dong || ph.should_close()
}

/// Popup Chỉnh sửa loại tài liệu.
pub fn ve_sua(ctx: &egui::Context, b: &mut BanSua) -> HanhDong {
    let mut hd = HanhDong::Mo;
    let rong = 1000.0_f32.min(ctx.content_rect().width() - 40.0);
    let ph = kh::hop(ctx, "bt_sua_loai", rong, "Chỉnh sửa loại tài liệu", |ui| {
        kh::than(ui, "bt_than_sua_loai", 520.0, Margin::same(14), |ui| {
            ui.horizontal(|ui| {
                let w = ui.available_width() - 230.0;
                ui.vertical(|ui| {
                    ui.label(RichText::new("Tên loại").color(CHU_PHU));
                    ui.add(kh::o_nhap(&mut b.loai.ten).desired_width(w).hint_text("Tên loại tài liệu"));
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Trang đọc mặc định").color(CHU_PHU));
                    crate::gui_muc_luc::kieu_o_chon(ui, Color32::WHITE, VIEN);
                    egui::ComboBox::from_id_salt("bt_trang_doc").icon(crate::gui_muc_luc::mui_ten_chon).width(ui.available_width()).selected_text(ten_trang_doc(b.loai.trang_doc)).show_ui(ui, |ui| {
                        for t in [TrangDoc::Dau, TrangDoc::DauCuoi, TrangDoc::TatCa] {
                            ui.selectable_value(&mut b.loai.trang_doc, t, ten_trang_doc(t));
                        }
                    });
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hướng dẫn chung").color(CHU_PHU));
                crate::gui_tach::bieu_tuong::thong_tin(ui, "Đoạn mở đầu câu lệnh: bố cục chung của loại tài liệu, giúp mô hình đọc tìm đúng chỗ");
            });
            ui.add(egui::TextEdit::multiline(&mut b.loai.huong_dan).desired_rows(2).desired_width(f32::INFINITY).font(egui::FontId::proportional(12.0)).hint_text("Hướng dẫn chung"));
            ui.add_space(6.0);
            bang_truong(ui, b);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new("Kéo").size(11.0).color(CHU_PHU));
                let (o, _) = ui.allocate_exact_size(egui::vec2(10.0, 14.0), Sense::hover());
                tay_nam(ui.painter(), o.center(), CHU_PHU);
                ui.label(RichText::new("để đổi thứ tự cột trong Excel / Google Sheet").size(11.0).color(CHU_PHU));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("+ Thêm trường").min_size(egui::vec2(0.0, 24.0))).clicked() {
                        b.them();
                    }
                });
            });
        });
        let loi = b.loi();
        kh::chan(ui, |ui| {
            if ui.add(kh::nut_chan("Xoá loại")).on_hover_text("Xoá loại tài liệu này").clicked() {
                hd = HanhDong::Xoa;
            }
            if ui.add_enabled(loi.is_none(), kh::nut_chan("Lưu thành loại mới…")).on_disabled_hover_text(loi.clone().unwrap_or_default()).clicked() {
                hd = HanhDong::LuuMoi;
            }
            if let Some(l) = &loi {
                ui.add_space(8.0);
                ui.label(RichText::new(l).size(12.0).color(DO));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(loi.is_none(), kh::nut_chan_mau("Lưu", XANH_LA)).on_disabled_hover_text(loi.clone().unwrap_or_default()).clicked() {
                    hd = HanhDong::Luu;
                }
                if ui.add(kh::nut_chan("Huỷ")).clicked() {
                    hd = HanhDong::Huy;
                }
            });
        });
    });
    if hd == HanhDong::Mo && ph.should_close() {
        HanhDong::Huy
    } else {
        hd
    }
}

/// Bảng trường trong popup: ⋮⋮ | Tên cột | Biến | Mô tả | Chuẩn hoá | Danh sách giá trị | Bắt buộc | ×.
fn bang_truong(ui: &mut egui::Ui, b: &mut BanSua) {
    const COT: [f32; 8] = [16.0, 0.17, 0.14, 0.24, 0.17, 0.2, 52.0, 24.0];
    let tong_co_dinh = COT[0] + COT[6] + COT[7] + 7.0 * 8.0 + 16.0;
    egui::Frame::new().stroke(Stroke::new(1.0, VIEN_NHAT)).corner_radius(4).inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let co_gian = ui.available_width() - tong_co_dinh;
        let rong: Vec<f32> = COT.iter().enumerate().map(|(i, &c)| if matches!(i, 0 | 6 | 7) { c } else { c * co_gian / 0.92 }).collect();
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.horizontal(|ui| {
            for (i, ten) in ["", "Tên cột", "Biến", "Mô tả gửi mô hình đọc", "Chuẩn hoá", "Danh sách giá trị", "Bắt buộc", ""].iter().enumerate() {
                let (r, p) = ui.allocate_exact_size(egui::vec2(rong[i], 18.0), Sense::hover());
                ui.painter().text(r.left_center(), egui::Align2::LEFT_CENTER, *ten, egui::FontId::proportional(12.0), CHU_PHU);
                if i == 5 {
                    p.on_hover_text("Chỉ dùng khi Chuẩn hoá = Danh sách: mô hình đọc phải trả về đúng một giá trị trong danh sách");
                } else if i == 6 {
                    p.on_hover_text("Để trống thì báo Kiểm tra ở tab Kết quả");
                }
            }
        });
        kh::ke_ngang(ui, VIEN_NHAT);
        let mut xoa = None;
        let mut tha = None;
        let n = b.loai.truong.len();
        let mut tam_dong = Vec::with_capacity(n);
        for i in 0..n {
            let r = ui
                .horizontal(|ui| {
                    // Tay kéo đổi thứ tự
                    let (o, keo) = ui.allocate_exact_size(egui::vec2(rong[0], 26.0), Sense::drag());
                    tay_nam(ui.painter(), o.center(), CHU_MO);
                    if keo.hovered() || keo.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                    keo.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Kéo để đổi thứ tự trường {}", i + 1)));
                    if keo.drag_started() {
                        b.keo = Some(i);
                    }
                    if keo.drag_stopped() {
                        tha = ui.ctx().pointer_interact_pos();
                    }
                    let t = &mut b.loai.truong[i];
                    let cu = t.ten_cot.clone();
                    ui.add(kh::o_nhap(&mut t.ten_cot).desired_width(rong[1]).hint_text("Tên cột"));
                    // Biến đang tự sinh theo tên cột → đổi tên cột thì đổi biến theo
                    if t.ten_cot != cu && t.bien == bien_tu_ten(&cu) {
                        t.bien = bien_tu_ten(&t.ten_cot);
                    }
                    ui.add(kh::o_nhap(&mut t.bien).desired_width(rong[2]).font(egui::FontId::monospace(11.5)).hint_text("bien"));
                    ui.add(egui::TextEdit::multiline(&mut t.mo_ta).desired_rows(2).desired_width(rong[3]).font(egui::FontId::proportional(12.0)).hint_text("Mô tả: ở trang nào, trông như thế nào, ví dụ"));
                    ui.scope(|ui| {
                        crate::gui_muc_luc::kieu_o_chon(ui, Color32::WHITE, VIEN);
                        egui::ComboBox::from_id_salt(("bt_chuan_hoa", i)).icon(crate::gui_muc_luc::mui_ten_chon).width(rong[4]).selected_text(t.chuan_hoa.ten()).show_ui(ui, |ui| {
                            for c in ChuanHoa::TAT_CA {
                                ui.selectable_value(&mut t.chuan_hoa, c, c.ten());
                            }
                        });
                    });
                    if t.chuan_hoa == ChuanHoa::DanhSach {
                        ui.add(egui::TextEdit::multiline(&mut b.ds_chu[i]).desired_rows(2).desired_width(rong[5]).font(egui::FontId::proportional(12.0)).hint_text("Giá trị cách nhau dấu phẩy"));
                    } else {
                        let (o, _) = ui.allocate_exact_size(egui::vec2(rong[5], 26.0), Sense::hover());
                        ui.painter().text(o.left_center(), egui::Align2::LEFT_CENTER, "—", egui::FontId::proportional(12.0), CHU_MO);
                    }
                    let t = &mut b.loai.truong[i];
                    ui.allocate_ui_with_layout(egui::vec2(rong[6], 26.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        kh::o_tick(ui, &mut t.bat_buoc, "").widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, t.bat_buoc, format!("Bắt buộc: {}", t.ten_cot)));
                    });
                    let x = ui.add(egui::Button::new(RichText::new("×").size(14.0).color(CHU_PHU)).frame(false).min_size(egui::vec2(rong[7], 24.0))).on_hover_text("Xoá trường");
                    if x.clicked() {
                        xoa = Some(i);
                    }
                })
                .response
                .rect;
            tam_dong.push(r);
            kh::ke_ngang(ui, VIEN_NHAT);
        }
        // Đang kéo: vạch xanh ở chỗ sẽ thả
        if let (Some(tu), Some(p)) = (b.keo, ui.ctx().pointer_interact_pos()) {
            let den = tam_dong.iter().position(|r| p.y < r.center().y).unwrap_or(n.saturating_sub(1));
            if let Some(r) = tam_dong.get(den) {
                let y = if den > tu { r.bottom() } else { r.top() };
                ui.painter().hline(r.x_range(), y, Stroke::new(2.0, crate::gui::VIEN_TAB_CHON));
            }
        }
        if let (Some(tu), Some(p)) = (b.keo, tha) {
            let den = tam_dong.iter().position(|r| p.y < r.center().y).unwrap_or(n.saturating_sub(1));
            b.doi_cho(tu, den);
            b.keo = None;
        }
        if let Some(i) = xoa {
            b.xoa(i);
        }
    });
}

/// Xác nhận Xoá loại: `Some(true)` = xoá, `Some(false)` = huỷ.
pub fn ve_xn_xoa(ctx: &egui::Context, ten: &str, so_truong: usize) -> Option<bool> {
    let mut kq = None;
    let ph = kh::hop(ctx, "bt_xn_xoa", 420.0, "Xoá loại tài liệu?", |ui| {
        kh::than(ui, "bt_than_xn_xoa", 70.0, Margin::same(14), |ui| {
            let mut job = egui::text::LayoutJob::default();
            let (t, d) = (crate::gui_tach::chu_dong(CHU, false), crate::gui_tach::chu_dong(CHU, true));
            job.append("Xoá loại ", 0.0, t.clone());
            job.append(ten, 0.0, d.clone());
            job.append(" cùng ", 0.0, t.clone());
            job.append(&format!("{so_truong} trường"), 0.0, d);
            job.append(", mô tả và chuẩn hoá của nó. Không khôi phục được. Kết quả đã bóc tách trước đó không bị ảnh hưởng.", 0.0, t);
            job.wrap.max_width = ui.available_width();
            ui.label(job);
        });
        kh::chan(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(kh::nut_chan_mau("Xoá", DO)).clicked() {
                    kq = Some(true);
                }
                if ui.add(kh::nut_chan("Huỷ")).clicked() {
                    kq = Some(false);
                }
            });
        });
    });
    kq.or(ph.should_close().then_some(false))
}

/// Lưu thành loại mới: nhập tên; `Some(true)` = lưu (tên hợp lệ, không trùng), `Some(false)` = huỷ.
pub fn ve_luu_moi(ctx: &egui::Context, ten_moi: &mut String, ten_cu: &str, ten_da_co: &[String]) -> Option<bool> {
    let mut kq = None;
    let trung = ten_da_co.iter().any(|t| t.trim().eq_ignore_ascii_case(ten_moi.trim()));
    let hop_le = !ten_moi.trim().is_empty() && !trung;
    let ph = kh::hop(ctx, "bt_luu_moi", 420.0, "Lưu thành loại tài liệu mới?", |ui| {
        kh::than(ui, "bt_than_luu_moi", 96.0, Margin::same(14), |ui| {
            ui.label(RichText::new("Tên loại mới").color(CHU_PHU));
            let o = ui.add(kh::o_nhap(ten_moi).desired_width(f32::INFINITY).hint_text("Tên loại mới"));
            if !o.has_focus() && ui.memory(|m| m.focused().is_none()) {
                o.request_focus();
            }
            if trung {
                ui.label(RichText::new("Đã có loại tên này").size(11.5).color(DO));
            } else {
                let mut job = egui::text::LayoutJob::default();
                let (t, d) = (crate::gui_tach::chu_dong(CHU_PHU, false), crate::gui_tach::chu_dong(CHU, true));
                job.append("Tạo loại mới từ các trường đang sửa. Loại ", 0.0, t.clone());
                job.append(ten_cu, 0.0, d);
                job.append(" giữ nguyên như trước khi sửa.", 0.0, t);
                job.wrap.max_width = ui.available_width();
                ui.label(job);
            }
        });
        kh::chan(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(hop_le, kh::nut_chan_mau("Lưu loại mới", XANH_LA)).clicked() {
                    kq = Some(true);
                }
                if ui.add(kh::nut_chan("Huỷ")).clicked() {
                    kq = Some(false);
                }
            });
        });
    });
    kq.or(ph.should_close().then_some(false))
}

/// Tay nắm kéo: 2 cột × 3 chấm (font giao diện không có ký tự ⋮).
fn tay_nam(p: &egui::Painter, c: egui::Pos2, mau: Color32) {
    for dx in [-2.5, 2.5] {
        for dy in [-4.0, 0.0, 4.0] {
            p.circle_filled(c + egui::vec2(dx, dy), 1.1, mau);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boc_tach::loai_tai_lieu::co_san;

    #[test]
    fn ban_sua_kiem_loi_va_doi_cho() {
        let mut b = BanSua::moi(&co_san()[0], 0);
        assert_eq!(b.loi(), None);
        assert_eq!(b.ket_qua(), co_san()[0], "mở rồi lưu không đổi gì");
        b.loai.truong[1].bien = "loai_van_ban".into();
        assert!(b.loi().unwrap().contains("trùng"));
        b.loai.truong[1].bien = "Số".into();
        assert!(b.loi().unwrap().contains("chữ thường"));
        b.loai.truong[1].bien = "so_ky_hieu".into();
        b.ds_chu[0] = "Quyết định,\nCông văn; ".into();
        assert_eq!(b.ket_qua().truong[0].danh_sach, ["Quyết định", "Công văn"]);
        b.doi_cho(5, 0);
        assert_eq!(b.ket_qua().truong[0].bien, "nguoi_ky");
        assert_eq!(b.ket_qua().truong[1].danh_sach, ["Quyết định", "Công văn"], "danh sách đi theo trường khi đổi chỗ");
        b.them();
        assert_eq!(b.loai.truong.len(), 7);
        b.xoa(6);
        b.loai.ten = "  ".into();
        assert_eq!(b.loi().as_deref(), Some("Nhập tên loại"));
    }
}
