//! Logo lớn của từng chức năng (màn giới thiệu): ô vuông bo góc màu riêng, hình trắng vẽ vector —
//! Đếm trang A4 (chồng trang + "A4" + huy hiệu số), Tách văn bản (trang tách đôi theo đường cắt),
//! Sinh mục lục (trang danh sách đánh số), Ký văn bản (trang có chữ ký + ngòi bút).

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Dem,
    Tach,
    MucLuc,
    Ky,
}

impl Logo {
    /// Màu nền ô logo: mỗi chức năng một màu trầm, cùng độ đậm.
    pub fn mau(self) -> Color32 {
        match self {
            Logo::Dem => Color32::from_rgb(0x1F, 0x5F, 0xBF),
            Logo::Tach => Color32::from_rgb(0x12, 0x80, 0x74),
            Logo::MucLuc => Color32::from_rgb(0xC0, 0x74, 0x12),
            Logo::Ky => Color32::from_rgb(0x6E, 0x48, 0xB8),
        }
    }
}

/// Trang giấy góc gấp phải trên trong ô `r` (toạ độ thật); `gap` = cạnh góc gấp.
fn trang(p: &egui::Painter, r: Rect, gap: f32, nen: Color32, mau_gap: Color32) {
    let diem = vec![r.left_top(), egui::pos2(r.right() - gap, r.top()), egui::pos2(r.right(), r.top() + gap), r.right_bottom(), r.left_bottom()];
    p.add(Shape::convex_polygon(diem, nen, Stroke::NONE));
    p.add(Shape::convex_polygon(vec![egui::pos2(r.right() - gap, r.top()), egui::pos2(r.right() - gap, r.top() + gap), egui::pos2(r.right(), r.top() + gap)], mau_gap, Stroke::NONE));
}

/// Vẽ logo vào ô vuông `o`.
pub fn ve(p: &egui::Painter, o: Rect, logo: Logo) {
    let mau = logo.mau();
    let s = o.width();
    // Bóng mờ + ô bo góc
    p.rect_filled(o.translate(egui::vec2(0.0, 3.0)), s * 0.24, Color32::from_black_alpha(28));
    p.rect_filled(o, s * 0.24, mau);
    // Toạ độ trong ô theo tỉ lệ 0..1
    let q = |x: f32, y: f32| -> Pos2 { egui::pos2(o.left() + x * s, o.top() + y * s) };
    let hcn = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(q(x0, y0), q(x1, y1));
    let trang_mo = Color32::from_white_alpha(120);
    let gap_mau = mau.gamma_multiply(0.55);
    let net = |w: f32| Stroke::new(w * s / 76.0, mau);
    match logo {
        Logo::Dem => {
            // Hai trang phía sau lệch dần + trang trước có chữ "A4", huy hiệu số góc dưới phải
            p.rect_filled(hcn(0.36, 0.16, 0.76, 0.70), 2.0, Color32::from_white_alpha(70));
            p.rect_filled(hcn(0.30, 0.20, 0.70, 0.74), 2.0, trang_mo);
            trang(p, hcn(0.22, 0.25, 0.64, 0.84), s * 0.11, Color32::WHITE, gap_mau);
            p.text(q(0.43, 0.56), egui::Align2::CENTER_CENTER, "A4", egui::FontId::new(s * 0.20, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), mau);
            let c = q(0.70, 0.74);
            p.circle_filled(c, s * 0.135, Color32::WHITE);
            p.circle_stroke(c, s * 0.135, Stroke::new(s * 0.03, mau));
            p.text(c, egui::Align2::CENTER_CENTER, "12", egui::FontId::new(s * 0.12, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), mau);
        }
        Logo::Tach => {
            // Trang bị cắt chéo: nửa trên lệch lên trái, nửa dưới lệch xuống phải, đường cắt nét đứt ở giữa
            let (l, r, t, b) = (0.26, 0.70, 0.16, 0.86);
            let cat = |x: f32| 0.56 - (x - l) * 0.28; // đường cắt chéo xuống trái → lên phải
            let tren = vec![q(l - 0.05, t - 0.03), q(r - 0.05, t - 0.03), q(r - 0.05, cat(r) - 0.03), q(l - 0.05, cat(l) - 0.03)];
            p.add(Shape::convex_polygon(tren, Color32::WHITE, Stroke::NONE));
            let duoi = vec![q(l + 0.05, cat(l) + 0.05), q(r + 0.05, cat(r) + 0.05), q(r + 0.05, b + 0.03), q(l + 0.05, b + 0.03)];
            p.add(Shape::convex_polygon(duoi, Color32::WHITE, Stroke::NONE));
            // Dòng chữ trên hai nửa
            for (y, x1) in [(0.27, 0.56), (0.36, 0.50)] {
                p.line_segment([q(l + 0.03, y), q(x1, y)], net(3.0));
            }
            for (y, x1) in [(0.70, 0.66), (0.79, 0.60)] {
                p.line_segment([q(l + 0.13, y), q(x1, y)], net(3.0));
            }
            // Đường cắt nét đứt trắng kéo dài ra ngoài trang
            let (a, b2) = (q(0.12, cat(0.12) + 0.01), q(0.88, cat(0.88) + 0.01));
            p.extend(Shape::dashed_line(&[a, b2], Stroke::new(s * 0.025, Color32::WHITE), s * 0.05, s * 0.035));
        }
        Logo::MucLuc => {
            // Trang danh sách: 3 dòng có ô số 1-2-3, vạch chữ và chấm dẫn tới số trang
            trang(p, hcn(0.20, 0.13, 0.80, 0.87), s * 0.12, Color32::WHITE, gap_mau);
            p.line_segment([q(0.32, 0.27), q(0.58, 0.27)], net(4.0));
            for (i, y) in [0.43, 0.58, 0.73].into_iter().enumerate() {
                let o_so = Rect::from_center_size(q(0.33, y), egui::vec2(s * 0.11, s * 0.11));
                p.rect_filled(o_so, 2.0, mau);
                p.text(o_so.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::new(s * 0.09, egui::FontFamily::Name(crate::gui::HO_FONT_DAM.into())), Color32::WHITE);
                p.line_segment([q(0.43, y), q(0.56, y)], net(3.0));
                for k in 0..3 {
                    p.circle_filled(q(0.60 + k as f32 * 0.04, y + 0.01), s * 0.010, mau.gamma_multiply(0.7));
                }
                p.line_segment([q(0.72, y), q(0.74, y)], net(3.0));
            }
        }
        Logo::Ky => {
            // Trang có dòng chữ + chữ ký lượn sóng, ngòi bút chéo góc dưới phải
            trang(p, hcn(0.18, 0.13, 0.66, 0.85), s * 0.11, Color32::WHITE, gap_mau);
            for (y, x1) in [(0.28, 0.50), (0.37, 0.54), (0.46, 0.44)] {
                p.line_segment([q(0.27, y), q(x1, y)], net(3.0));
            }
            let ky: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let t = i as f32 / 24.0;
                    q(0.26 + t * 0.30, 0.66 - (t * 9.0).sin() * 0.05 * (1.0 - t * 0.5))
                })
                .collect();
            p.add(Shape::line(ky, net(3.2)));
            p.line_segment([q(0.25, 0.75), q(0.58, 0.75)], Stroke::new(s * 0.015, mau.gamma_multiply(0.5)));
            // Bút: thân trắng chéo, ngòi tam giác chạm cuối chữ ký
            let (dau, cuoi) = (q(0.86, 0.36), q(0.62, 0.66));
            let huong = (cuoi - dau).normalized();
            let ngang = egui::vec2(-huong.y, huong.x) * s * 0.055;
            let co = cuoi - huong * s * 0.11;
            p.add(Shape::convex_polygon(vec![dau + ngang, co + ngang, co - ngang, dau - ngang], Color32::WHITE, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![co + ngang, cuoi, co - ngang], Color32::from_white_alpha(200), Stroke::NONE));
            p.line_segment([co + ngang * 0.9, co - ngang * 0.9], Stroke::new(s * 0.02, mau));
        }
    }
}
