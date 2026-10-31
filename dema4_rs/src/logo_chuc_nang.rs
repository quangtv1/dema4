//! Logo lớn của từng chức năng (màn giới thiệu): nét mảnh một màu kiểu cổ điển, không nền —
//! Đếm trang A4 (chồng trang + "A4"), Tách văn bản (một văn bản rẽ nhánh thành hai),
//! Sinh mục lục (trang danh sách đánh số), Bóc tách dữ liệu (văn bản → bảng), Chuẩn hoá tên (văn bản + ô tên có con trỏ),
//! Ký văn bản (trang có chữ ký + bút), Đóng gói AIP (hộp gói).
//! Các nét không chồng lên nhau nên không cần tô nền che (đặt được trên nền trắng hay xám).

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Dem,
    Tach,
    MucLuc,
    BocTach,
    ChuanTen,
    Ky,
    Aip,
}

/// Màu nét chung của mọi logo.
const MAU: Color32 = Color32::from_rgb(0x3C, 0x48, 0x58);

/// Viền trang giấy góc gấp phải trên (`gap` = cạnh góc gấp).
fn trang(p: &egui::Painter, r: Rect, gap: f32, net: Stroke) {
    let vien = vec![r.left_top(), egui::pos2(r.right() - gap, r.top()), egui::pos2(r.right(), r.top() + gap), r.right_bottom(), r.left_bottom()];
    p.add(Shape::closed_line(vien, net));
    p.add(Shape::line(vec![egui::pos2(r.right() - gap, r.top()), egui::pos2(r.right() - gap, r.top() + gap), egui::pos2(r.right(), r.top() + gap)], net));
}

/// Vẽ logo vào ô vuông `o` (màu nét chung).
pub fn ve(p: &egui::Painter, o: Rect, logo: Logo) {
    ve_mau(p, o, logo, MAU);
}

/// Như `ve`, màu nét tuỳ chọn (Trang chủ: rê chuột → xanh, chức năng chưa dùng được → mờ).
pub fn ve_mau(p: &egui::Painter, o: Rect, logo: Logo, mau: Color32) {
    let s = o.width();
    let q = |x: f32, y: f32| -> Pos2 { egui::pos2(o.left() + x * s, o.top() + y * s) };
    let hcn = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(q(x0, y0), q(x1, y1));
    let net = Stroke::new((s * 0.03).max(1.5), mau);
    let net_manh = Stroke::new((s * 0.022).max(1.2), mau);
    let dam = |co: f32| egui::FontId::new(s * co, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into()));
    match logo {
        Logo::Dem => {
            // Hai trang phía sau chỉ lộ mép trên + mép phải (hình chữ L), trang trước có "A4"
            let (l, t, r, b) = (0.20, 0.30, 0.62, 0.88);
            for k in 1..=2 {
                let d = 0.07 * k as f32;
                p.add(Shape::line(vec![q(l + d, t - 0.07 * (k - 1) as f32), q(l + d, t - d), q(r + d, t - d), q(r + d, b - d), q(r + d - 0.07, b - d)], net_manh));
            }
            trang(p, hcn(l, t, r, b), s * 0.11, net);
            p.text(q((l + r) / 2.0, 0.62), egui::Align2::CENTER_CENTER, "A4", dam(0.19), mau);
        }
        Logo::Tach => {
            // Một văn bản bên trái → mũi tên rẽ nhánh chữ Y → hai văn bản nhỏ bên phải
            trang(p, hcn(0.06, 0.26, 0.40, 0.74), s * 0.09, net);
            for (y, x1) in [(0.42, 0.30), (0.51, 0.33), (0.60, 0.26)] {
                p.line_segment([q(0.13, y), q(x1, y)], net_manh);
            }
            for t in [0.10, 0.56] {
                trang(p, hcn(0.68, t, 0.94, t + 0.34), s * 0.07, net);
                for (dy, x1) in [(0.13, 0.86), (0.21, 0.82)] {
                    p.line_segment([q(0.74, t + dy), q(x1, t + dy)], net_manh);
                }
            }
            // Thân mũi tên rồi tách hai nhánh, đầu mũi tên chĩa vào hai văn bản con
            let (goc, re) = (q(0.44, 0.50), q(0.51, 0.50));
            p.line_segment([goc, re], net);
            for y in [0.27, 0.73] {
                let ngon = q(0.64, y);
                p.add(Shape::line(vec![re, q(0.56, y), ngon], net));
                let (lui, cao) = (s * 0.065, s * 0.06);
                p.add(Shape::line(vec![ngon + egui::vec2(-lui, -cao), ngon, ngon + egui::vec2(-lui, cao)], net));
            }
        }
        Logo::MucLuc => {
            // Trang danh sách: tiêu đề, 3 dòng có ô số 1-2-3, vạch chữ, chấm dẫn tới số trang
            trang(p, hcn(0.20, 0.12, 0.80, 0.88), s * 0.12, net);
            p.line_segment([q(0.33, 0.26), q(0.57, 0.26)], net);
            for (i, y) in [0.42, 0.57, 0.72].into_iter().enumerate() {
                let o_so = Rect::from_center_size(q(0.34, y), egui::vec2(s * 0.11, s * 0.11));
                p.rect_stroke(o_so, 1.5, net_manh, egui::StrokeKind::Middle);
                p.text(o_so.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), dam(0.075), mau);
                p.line_segment([q(0.44, y), q(0.57, y)], net_manh);
                for k in 0..3 {
                    p.circle_filled(q(0.61 + k as f32 * 0.035, y), s * 0.009, mau);
                }
            }
        }
        Logo::BocTach => {
            // Văn bản có dòng chữ → mũi tên → bảng 3 hàng 2 cột
            trang(p, hcn(0.12, 0.14, 0.50, 0.86), s * 0.10, net);
            for (y, x1) in [(0.36, 0.40), (0.46, 0.43), (0.56, 0.36), (0.66, 0.42)] {
                p.line_segment([q(0.19, y), q(x1, y)], net_manh);
            }
            p.line_segment([q(0.54, 0.50), q(0.62, 0.50)], net);
            p.add(Shape::line(vec![q(0.58, 0.45), q(0.63, 0.50), q(0.58, 0.55)], net));
            p.rect_stroke(hcn(0.67, 0.30, 0.92, 0.70), 1.5, net, egui::StrokeKind::Middle);
            for y in [0.433, 0.566] {
                p.line_segment([q(0.67, y), q(0.92, y)], net_manh);
            }
            p.line_segment([q(0.77, 0.30), q(0.77, 0.70)], net_manh);
        }
        Logo::ChuanTen => {
            // Văn bản có dòng chữ + ô tên có con trỏ gõ chữ (chữ I)
            trang(p, hcn(0.12, 0.12, 0.50, 0.86), s * 0.10, net);
            for (y, x1) in [(0.32, 0.38), (0.41, 0.42), (0.50, 0.34)] {
                p.line_segment([q(0.19, y), q(x1, y)], net_manh);
            }
            p.rect_stroke(hcn(0.38, 0.58, 0.88, 0.78), 2.0, net, egui::StrokeKind::Middle);
            p.line_segment([q(0.45, 0.68), q(0.66, 0.68)], net_manh);
            p.line_segment([q(0.76, 0.61), q(0.76, 0.75)], net_manh);
            for y in [0.61, 0.75] {
                p.line_segment([q(0.73, y), q(0.79, y)], net_manh);
            }
        }
        Logo::Aip => {
            // Hộp gói nhìn nghiêng: nắp, cạnh giữa, dây buộc, nhãn 2 vạch
            p.add(Shape::closed_line(vec![q(0.18, 0.34), q(0.50, 0.20), q(0.82, 0.34), q(0.82, 0.70), q(0.50, 0.84), q(0.18, 0.70)], net));
            p.add(Shape::line(vec![q(0.18, 0.34), q(0.50, 0.48), q(0.82, 0.34)], net));
            p.line_segment([q(0.50, 0.48), q(0.50, 0.84)], net);
            p.add(Shape::line(vec![q(0.34, 0.27), q(0.66, 0.41), q(0.66, 0.52)], net_manh));
            p.line_segment([q(0.26, 0.58), q(0.40, 0.64)], net_manh);
            p.line_segment([q(0.26, 0.66), q(0.36, 0.70)], net_manh);
        }
        Logo::Ky => {
            // Trang có dòng chữ + chữ ký lượn sóng trên dòng ký; bút đặt ngoài mép phải, ngòi chạm cuối chữ ký
            trang(p, hcn(0.14, 0.12, 0.60, 0.88), s * 0.11, net);
            for (y, x1) in [(0.27, 0.44), (0.36, 0.48), (0.45, 0.38)] {
                p.line_segment([q(0.23, y), q(x1, y)], net_manh);
            }
            let ky: Vec<Pos2> = (0..=28)
                .map(|i| {
                    let t = i as f32 / 28.0;
                    q(0.22 + t * 0.31, 0.65 - (t * 9.5).sin() * 0.045 * (1.0 - t * 0.4))
                })
                .collect();
            p.add(Shape::line(ky, net));
            p.line_segment([q(0.22, 0.75), q(0.52, 0.75)], net_manh);
            // Bút: thân hình chữ nhật chéo + ngòi tam giác
            let (dau, ngoi) = (q(0.90, 0.30), q(0.62, 0.68));
            let huong = (ngoi - dau).normalized();
            let ngang = egui::vec2(-huong.y, huong.x) * s * 0.05;
            let co = ngoi - huong * s * 0.11;
            p.add(Shape::closed_line(vec![dau + ngang, co + ngang, ngoi, co - ngang, dau - ngang], net));
            p.line_segment([co + ngang, co - ngang], net_manh);
            p.line_segment([dau + ngang + huong * s * 0.07, dau - ngang + huong * s * 0.07], net_manh);
        }
    }
}
