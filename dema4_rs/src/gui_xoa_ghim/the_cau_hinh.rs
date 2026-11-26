//! Thân **Xóa ghim** trong trang Cấu hình (như prototype): dải "Thư mục đang mở" rồi 4 thẻ 2 cột — Nhận diện & bảo vệ,
//! Hiệu năng & lưu | Xuất file, Hiển thị. Thẻ gắn nhãn **theo thư mục** (lưu cùng vùng của thư mục đang mở, kho Xóa
//! ghim) hoặc **chung** (cấu hình app). Dòng Nhận diện layout có nút tải / nhập mô hình AI và tiến độ tải.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Rect, Sense, Stroke};

use super::thiet_ke as tk;
use super::widget::{font, font_dam};
use super::{bieu_tuong, ManXoaGhim};
use crate::cau_hinh::CauHinhXoaGhim;
use crate::gui_cau_hinh::{self as gc, Pham};
use crate::xoa_ghim::kho::CauHinhXuat;
use crate::xoa_ghim::mo_hinh::{self, TaiMoHinh, TrangThaiMoHinh};
use crate::xoa_ghim::xu_ly::TuyChon;

const DPI: [(u16, &str); 4] = [(0, "Gốc"), (300, "300"), (200, "200"), (150, "150")];
const NEN: [u8; 4] = [95, 90, 80, 70];

/// Tuỳ chọn theo thư mục đang mở (bản nháp trong hộp).
#[derive(Clone, Debug, PartialEq)]
pub struct CauHinhThuMuc {
    pub tc: TuyChon,
    pub xuat: CauHinhXuat,
    pub ghi_de: bool,
}

/// Trạng thái tab trong hộp Cấu hình.
#[derive(Default)]
pub struct TabXoaGhim {
    /// `None` = chưa mở thư mục (thẻ "theo thư mục" mờ).
    pub thu_muc: Option<CauHinhThuMuc>,
    /// Đường dẫn thư mục (hoặc file) đang mở ở màn Xóa ghim, hiện ở dải đầu tab; `None` khi chưa mở.
    pub duong_dan: Option<PathBuf>,
    /// Việc tải / nhập mô hình (mượn của màn Xóa ghim khi mở hộp, trả lại khi đóng — tải tiếp sau khi đóng hộp).
    pub mo_hinh: Option<TaiMoHinh>,
}

impl ManXoaGhim {
    /// Tuỳ chọn theo thư mục đang mở (để hộp Cấu hình sửa).
    pub fn cau_hinh_thu_muc(&self) -> Option<CauHinhThuMuc> {
        self.thu_muc.as_ref()?;
        Some(CauHinhThuMuc { tc: self.bo.tm.tuy_chon, xuat: self.bo.tm.xuat, ghi_de: self.bo.tm.ghi.ghi_de })
    }

    /// Lưu từ hộp Cấu hình: ghi vào thư mục đang mở (đổi ghi đè → dựng lại danh sách đích).
    pub fn ap_cau_hinh_thu_muc(&mut self, c: CauHinhThuMuc) {
        if self.thu_muc.is_none() || self.cau_hinh_thu_muc().as_ref() == Some(&c) {
            return;
        }
        let doi_dich = c.ghi_de != self.bo.tm.ghi.ghi_de;
        self.bo.tm.tuy_chon = c.tc;
        self.bo.tm.xuat = c.xuat;
        self.bo.tm.ghi.ghi_de = c.ghi_de;
        self.bo.doi_chung();
        if doi_dich {
            self.lap_ds_giu_chon();
        } else {
            self.ghi_ngay();
        }
    }
}

/// Trạng thái mô hình hiện tại: theo việc đang chạy, không có việc thì theo file trên đĩa.
fn trang_thai(m: &Option<TaiMoHinh>) -> TrangThaiMoHinh {
    match m {
        Some(t) => t.trang_thai(),
        None => TrangThaiMoHinh::hien_tai(),
    }
}

/// Thẻ: biểu tượng + tên + nhãn phạm vi; `duoc` = false → thân mờ, không sửa được (chưa mở thư mục).
fn the(ui: &mut egui::Ui, bt: &str, ten: &str, pham: Pham, duoc: bool, noi: impl FnOnce(&mut egui::Ui)) {
    gc::the_tuy_bien(
        ui,
        |ui| {
            let (o, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
            bieu_tuong::ve(ui, bt, o);
            ui.label(egui::RichText::new(ten).font(font_dam(13.5)).color(tk::CHU));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| gc::nhan_pham(ui, pham));
        },
        |ui| {
            ui.add_enabled_ui(duoc, noi);
        },
    );
}

/// 1 dòng: biểu tượng · tên + mô tả · điều khiển bên phải (`dk`), + dòng phụ `duoi` dưới tên.
fn dong(ui: &mut egui::Ui, bt: &str, ten: &str, mo_ta: &str, rong_dk: f32, dk: impl FnOnce(&mut egui::Ui), duoi: impl FnOnce(&mut egui::Ui)) {
    let y = ui.cursor().top();
    ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, Color32::from_rgb(0xF0, 0xF1, 0xF3)));
    ui.add_space(6.0);
    ui.horizontal_top(|ui| {
        let (o, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
        bieu_tuong::ve(ui, bt, Rect::from_center_size(o.center() + egui::vec2(0.0, 1.0), egui::vec2(16.0, 16.0)));
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - rong_dk).max(80.0));
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(egui::RichText::new(ten).font(font(tk::CHU_CO)).color(tk::CHU));
            if !mo_ta.is_empty() {
                ui.label(egui::RichText::new(mo_ta).font(font(tk::CHU_NHO)).color(tk::CHU_PHU));
            }
            duoi(ui);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), dk);
    });
    ui.add_space(4.0);
}

fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, gt: &mut T, ds: &[(T, String)], ten: &str) -> bool {
    let mut doi = false;
    let hien = ds.iter().find(|(v, _)| v == gt).map_or(String::new(), |(_, t)| t.clone());
    egui::ComboBox::from_id_salt(id)
        .icon(super::widget::mui_ten)
        .width(92.0)
        .selected_text(egui::RichText::new(hien).font(font(tk::CHU_CO)))
        .show_ui(ui, |ui| {
            for (v, t) in ds {
                if ui.selectable_label(gt == v, egui::RichText::new(t).font(font(tk::CHU_CO))).clicked() && gt != v {
                    *gt = *v;
                    doi = true;
                }
            }
        })
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, ten));
    doi
}

fn tick(ui: &mut egui::Ui, b: &mut bool, ten: &str) {
    let r = super::widget::tick(ui, b, "");
    r.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, *b, ten));
}

impl TabXoaGhim {
    /// Thân tab; `cfg` = bản nháp cấu hình chung.
    pub fn ve(&mut self, ui: &mut egui::Ui, cfg: &mut CauHinhXoaGhim) {
        self.dai_thu_muc(ui);
        ui.add_space(gc::KHE_THE);
        ui.spacing_mut().item_spacing = egui::vec2(gc::KHE_THE, 0.0);
        ui.columns(2, |cot| {
            self.the_nhan_dien(&mut cot[0]);
            the_hieu_nang(&mut cot[0], cfg);
            self.the_xuat(&mut cot[1]);
            the_hien_thi(&mut cot[1], cfg);
        });
    }

    /// Dải đầu tab: xanh nhạt "Thư mục đang mở: <đường dẫn>"; chưa mở → dải cam hướng dẫn mở ở màn Xóa ghim.
    fn dai_thu_muc(&self, ui: &mut egui::Ui) {
        let (nen, vien) = if self.thu_muc.is_some() { (Color32::from_rgb(0xEF, 0xF6, 0xFF), Color32::from_rgb(0xBF, 0xDB, 0xFE)) } else { (gc::CAM_NEN, gc::CAM_VIEN) };
        egui::Frame::new().fill(nen).stroke(Stroke::new(1.0, vien)).corner_radius(5).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (o, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                bieu_tuong::ve(ui, "thu_muc", o);
                match (&self.thu_muc, &self.duong_dan) {
                    (Some(_), duong_dan) => {
                        ui.label(egui::RichText::new("Thư mục đang mở:").font(font(12.0)).color(tk::CHU));
                        if let Some(p) = duong_dan {
                            ui.label(egui::RichText::new(p.display().to_string()).font(font_dam(12.0)).color(tk::CHU));
                        }
                        ui.label(egui::RichText::new("— thẻ").font(font(12.0)).color(tk::CHU));
                        gc::nhan_pham(ui, Pham::ThuMuc);
                        ui.label(egui::RichText::new("áp cho thư mục này.").font(font(12.0)).color(tk::CHU));
                    }
                    (None, _) => {
                        ui.label(egui::RichText::new("Mở 1 file hoặc thư mục ở màn Xóa ghim để chỉnh phần theo thư mục").font(font(12.0)).color(gc::CAM_CHU));
                    }
                }
            });
        });
    }

    /// Test / chụp ảnh: tab như khi màn Xóa ghim đang mở thư mục `duong_dan` với tuỳ chọn mặc định.
    #[cfg(test)]
    pub fn dang_mo_thu_muc(duong_dan: PathBuf) -> Self {
        let thu_muc = CauHinhThuMuc { tc: TuyChon::default(), xuat: CauHinhXuat::default(), ghi_de: false };
        Self { thu_muc: Some(thu_muc), duong_dan: Some(duong_dan), mo_hinh: None }
    }

    /// Mặc định: tuỳ chọn thư mục + cấu hình chung về giá trị đầu.
    pub fn mac_dinh(&mut self, cfg: &mut CauHinhXoaGhim) {
        *cfg = CauHinhXoaGhim::default();
        if let Some(t) = &mut self.thu_muc {
            *t = CauHinhThuMuc { tc: TuyChon::default(), xuat: CauHinhXuat::default(), ghi_de: false };
        }
    }

    fn the_nhan_dien(&mut self, ui: &mut egui::Ui) {
        let duoc = self.thu_muc.is_some();
        let tt = trang_thai(&self.mo_hinh);
        let mut tai = false;
        let mut nhap = false;
        let mut huy = false;
        let mut tm = self.thu_muc.clone().unwrap_or(CauHinhThuMuc { tc: TuyChon::default(), xuat: CauHinhXuat::default(), ghi_de: false });
        the(ui, "khien", "Nhận diện & bảo vệ", Pham::ThuMuc, true, |ui| {
            let san = matches!(tt, TrangThaiMoHinh::SanSang);
            let mo_ta = if san { "Tìm khối chữ, bảng; không xoá vào nội dung" } else { "Cần tải mô hình AI (37,5 MB)" };
            dong(
                ui,
                "layout",
                "Nhận diện layout",
                mo_ta,
                172.0,
                |ui| {
                    ui.add_enabled_ui(duoc && san, |ui| {
                        let mut b = tm.tc.nhan_dien && san;
                        let r = super::widget::tick(ui, &mut b, "");
                        r.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, duoc && san, b, "Nhận diện layout"));
                        if r.on_disabled_hover_text(if san { "Mở thư mục trước" } else { "Tải mô hình trước" }).changed() {
                            tm.tc.nhan_dien = b;
                        }
                    });
                    match &tt {
                        TrangThaiMoHinh::ChuaCo | TrangThaiMoHinh::Loi(_) => {
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(egui::RichText::new("…").font(font(tk::CHU_CO))).min_size(egui::vec2(24.0, 24.0))).on_hover_text("Máy không có mạng: chọn file mô hình đã chép sẵn").clicked() {
                                nhap = true;
                            }
                            let r = ui.add(egui::Button::image_and_text(bieu_tuong::anh("tai", 12.0, 12.0), egui::RichText::new("Tải mô hình").font(font(tk::CHU_CO)).color(tk::XANH_DAM_CHU)).fill(Color32::WHITE).stroke(Stroke::new(1.0, tk::VIEN_NUT_LINK)).min_size(egui::vec2(0.0, 24.0)));
                            if r.on_hover_text("Tải 37,5 MB từ kho XoaGhim trên GitHub, kiểm SHA-256 rồi lưu vào thư mục dữ liệu; dùng ngoại tuyến sau đó").clicked() {
                                tai = true;
                            }
                        }
                        TrangThaiMoHinh::SanSang => {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("sẵn sàng").font(font(tk::CHU_NHO)).color(tk::XANH_LA_OK));
                            let (o, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                            bieu_tuong::ve(ui, "dau_xanh", o);
                        }
                        TrangThaiMoHinh::DangTai { .. } | TrangThaiMoHinh::KiemTra => {
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(egui::RichText::new("Huỷ").font(font(tk::CHU_CO))).min_size(egui::vec2(0.0, 24.0))).clicked() {
                                huy = true;
                            }
                        }
                    }
                },
                |ui| match &tt {
                    TrangThaiMoHinh::DangTai { da, tong } => {
                        let tong = (*tong).max(mo_hinh::CO_FILE);
                        let phan = *da as f32 / tong as f32;
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let (o, _) = ui.allocate_exact_size(egui::vec2((ui.available_width() - 100.0).max(40.0), 5.0), Sense::hover());
                            ui.painter().rect_filled(o, 3.0, Color32::from_rgb(0xE5, 0xE7, 0xEB));
                            ui.painter().rect_filled(Rect::from_min_size(o.min, egui::vec2(o.width() * phan.clamp(0.0, 1.0), o.height())), 3.0, tk::XANH);
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} MB", *da as f64 / 1e6, tong as f64 / 1e6).replace('.', ",")).font(font(11.0)).color(tk::CHU_PHU));
                        });
                    }
                    TrangThaiMoHinh::KiemTra => {
                        ui.label(egui::RichText::new("Đang kiểm tra tệp (SHA-256)…").font(font(11.0)).color(tk::CHU_PHU));
                    }
                    TrangThaiMoHinh::Loi(e) => {
                        ui.label(egui::RichText::new(e).font(font(11.0)).color(tk::DO_NUT));
                    }
                    _ => {}
                },
            );
            ui.add_enabled_ui(duoc, |ui| {
                dong(ui, "net_den", "Bảo vệ nét đen", "Không coi điểm rất tối là vết bẩn — giữ nét chữ", 30.0, |ui| tick(ui, &mut tm.tc.bao_ve_net_den, "Bảo vệ nét đen"), |_| {});
                dong(ui, "dau", "Bảo vệ dấu đỏ / chữ ký xanh", "Giữ con dấu, chữ ký mực màu", 30.0, |ui| tick(ui, &mut tm.tc.bao_ve_do_xanh, "Bảo vệ dấu đỏ / chữ ký xanh"), |_| {});
            });
        });
        if duoc {
            self.thu_muc = Some(tm);
        }
        let ctx = ui.ctx().clone();
        if tai {
            self.mo_hinh = Some(TaiMoHinh::tai(move || ctx.request_repaint()));
        } else if nhap {
            if let Some(p) = rfd::FileDialog::new().set_title("Chọn file mô hình yolov12s-doclaynet.onnx").add_filter("ONNX", &["onnx"]).pick_file() {
                self.mo_hinh = Some(TaiMoHinh::nhap(p, move || ctx.request_repaint()));
            }
        } else if huy {
            if let Some(m) = &self.mo_hinh {
                m.huy();
            }
        }
        if self.mo_hinh.as_ref().is_some_and(|m| !m.xong()) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    fn the_xuat(&mut self, ui: &mut egui::Ui) {
        let duoc = self.thu_muc.is_some();
        let mut tm = self.thu_muc.clone().unwrap_or(CauHinhThuMuc { tc: TuyChon::default(), xuat: CauHinhXuat::default(), ghi_de: false });
        the(ui, "xuat", "Xuất file", Pham::ThuMuc, duoc, |ui| {
            let ds_dpi: Vec<(u16, String)> = DPI.iter().map(|(v, t)| (*v, t.to_string())).collect();
            dong(ui, "dpi", "Độ phân giải (DPI)", "Gốc = giữ như bản quét", 118.0, |ui| {
                combo(ui, "xg_cfg_dpi", &mut tm.xuat.dpi, &ds_dpi, "Độ phân giải (DPI)");
            }, |_| {});
            let mut ds_nen: Vec<(u8, String)> = NEN.iter().map(|v| (*v, format!("{v}%"))).collect();
            if !NEN.contains(&tm.xuat.chat_luong) {
                ds_nen.push((tm.xuat.chat_luong, format!("{}%", tm.xuat.chat_luong)));
            }
            dong(ui, "nen", "Chất lượng nén JPEG", "Thấp hơn = file nhỏ hơn", 118.0, |ui| {
                combo(ui, "xg_cfg_nen", &mut tm.xuat.chat_luong, &ds_nen, "Chất lượng nén JPEG");
            }, |_| {});
            dong(ui, "den_trang", "Nén đen trắng", "Trang đen trắng / xám ghi gọn hơn", 30.0, |ui| tick(ui, &mut tm.xuat.nen_den_trang, "Nén đen trắng"), |_| {});
            dong(ui, "nen_trang", "Tô nền trắng", "Tô vết bằng màu trắng thay màu nền đo được", 30.0, |ui| tick(ui, &mut tm.tc.nen_trang, "Tô nền trắng"), |_| {});
            let ghi_de = tm.ghi_de;
            dong(ui, "ghi_de", "Ghi đè file gốc", "Không tạo thư mục _xoaghim, thay thẳng file nguồn", 30.0, |ui| tick(ui, &mut tm.ghi_de, "Ghi đè file gốc"), |ui| {
                if ghi_de {
                    ui.add_space(3.0);
                    egui::Frame::new().fill(tk::CAM_NEN).stroke(Stroke::new(1.0, tk::CAM_VIEN)).corner_radius(3).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                        ui.label(egui::RichText::new("Không hoàn tác được — nên sao lưu thư mục nguồn trước.").font(font(tk::CHU_NHO)).color(tk::CAM_CHU));
                    });
                }
            });
        });
        if duoc {
            self.thu_muc = Some(tm);
        }
    }
}

fn the_hieu_nang(ui: &mut egui::Ui, cfg: &mut CauHinhXoaGhim) {
    the(ui, "hieu_nang", "Hiệu năng & lưu", Pham::Chung, true, |ui| {
        dong(ui, "lop", "Xử lý theo lô 10 trang", "Nhanh hơn, tốn RAM hơn", 30.0, |ui| tick(ui, &mut cfg.lo_10, "Xử lý theo lô 10 trang"), |_| {});
        dong(ui, "dong_ho", "Tự lưu vùng", "0 = lưu ngay khi sửa", 96.0, |ui| {
            let r = ui.add(egui::DragValue::new(&mut cfg.tu_luu_phut).range(0..=60).suffix(" phút"));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SpinButton, true, "Tự lưu vùng (phút)"));
        }, |_| {});
    });
}

fn the_hien_thi(ui: &mut egui::Ui, cfg: &mut CauHinhXoaGhim) {
    the(ui, "mat", "Hiển thị", Pham::Chung, true, |ui| {
        dong(ui, "kieu_xem", "Kiểu xem", "", 118.0, |ui| {
            let ds = [(false, "Liên tiếp".to_string()), (true, "Từng trang".to_string())];
            combo(ui, "xg_cfg_kieu", &mut cfg.tung_trang, &ds, "Kiểu xem");
        }, |_| {});
        dong(ui, "luoi", "Cột trang thu nhỏ", "", 30.0, |ui| tick(ui, &mut cfg.thu_nho, "Cột trang thu nhỏ"), |_| {});
        dong(ui, "hai_khung", "Khung Đích", "Xem kết quả bên phải", 30.0, |ui| tick(ui, &mut cfg.khung_dich, "Khung Đích"), |_| {});
        dong(ui, "noi", "Cuộn Gốc | Đích cùng nhau", "", 30.0, |ui| tick(ui, &mut cfg.cuon_cung, "Cuộn Gốc | Đích cùng nhau"), |_| {});
    });
}
