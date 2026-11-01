//! Logo Sao Mai động (Trang chủ, giống prototype đã duyệt): chấm sáng vuốt dọc vệt cong quanh chữ S từ chân lên tâm
//! ngôi sao cam (ngôi sao mờ đi trong lúc vuốt, chấm sáng lớn dần ở quãng cuối), tới nơi ngôi sao sáng lại và phóng
//! 1,5 lần rồi về 1, sau đó đứng yên tới chu kỳ sau. Ảnh nền là logo đã bỏ ngôi sao (`logo-sao-mai-nen.png`), ngôi sao
//! cam là ảnh riêng (`logo-sao-mai-sao.png`) — tách từ `logo-sao-mai.png`.

use eframe::egui::{self, Color32, Painter, Rect, TextureHandle};

/// Kích thước ảnh logo gốc (toạ độ dưới đây theo pixel ảnh này).
pub const KICH: [f32; 2] = [300.0, 246.0];
/// Ô ngôi sao cam trong ảnh gốc (góc trái trên, cạnh) và tâm sao.
const O_SAO: (f32, f32, f32) = (189.0, 1.0, 46.0);
const TAM_SAO: (f32, f32) = (212.0, 23.0);
/// Đường vuốt (Bézier bậc 3) theo vệt cong quanh chữ S: đuôi trái dưới → tâm ngôi sao cam.
const DUONG: [(f32, f32); 4] = [(100.0, 110.0), (170.0, 100.0), (236.0, 62.0), (212.0, 23.0)];

/// Mốc thời gian 1 chu kỳ (giây): vuốt → ngôi sao phóng 1,5 lần → nghỉ.
const T_VUOT: f64 = 0.7;
const T_NHAY: f64 = 0.3;
pub const CHU_KY: f64 = 5.0;

const VANG: Color32 = Color32::from_rgb(0xFF, 0xD0, 0x6A);

fn tex(ctx: &egui::Context, ten: &'static str, png: &[u8]) -> TextureHandle {
    let id = egui::Id::unique(ten);
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return t;
    }
    let anh = eframe::icon_data::from_png_bytes(png).expect("logo PNG hợp lệ");
    let t = ctx.load_texture(ten, egui::ColorImage::from_rgba_unmultiplied([anh.width as usize, anh.height as usize], &anh.rgba), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// Kích thước hiển thị theo bề rộng `rong` (giữ tỉ lệ ảnh gốc).
pub fn kich(rong: f32) -> egui::Vec2 {
    egui::vec2(rong, rong * KICH[1] / KICH[0])
}

fn bezier(t: f32) -> (f32, f32) {
    let [a, b, c, d] = DUONG;
    let u = 1.0 - t;
    let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (k[0] * a.0 + k[1] * b.0 + k[2] * c.0 + k[3] * d.0, k[0] * a.1 + k[1] * b.1 + k[2] * c.1 + k[3] * d.1)
}

fn em(t: f32) -> f32 {
    // ease-in-out: chậm ở đuôi, nhanh giữa, chậm dần khi tới đỉnh
    t * t * (3.0 - 2.0 * t)
}

fn mo(m: Color32, a: f32) -> Color32 {
    m.gamma_multiply(a.clamp(0.0, 1.0))
}

/// Logo tĩnh (ngôi sao đứng yên) — popup, góc màn Giới thiệu.
pub fn ve_tinh(p: &Painter, r: Rect) {
    let (nen, sao) = anh(p.ctx());
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    p.image(nen.id(), r, uv, Color32::WHITE);
    let f = r.width() / KICH[0];
    p.image(sao.id(), Rect::from_min_size(egui::pos2(r.left() + O_SAO.0 * f, r.top() + O_SAO.1 * f), egui::vec2(O_SAO.2, O_SAO.2) * f), uv, Color32::WHITE);
}

fn anh(ctx: &egui::Context) -> (TextureHandle, TextureHandle) {
    (tex(ctx, "logo_sao_mai_nen", include_bytes!("../assets/logo-sao-mai-nen.png")), tex(ctx, "logo_sao_mai_sao", include_bytes!("../assets/logo-sao-mai-sao.png")))
}

/// Logo động (Trang chủ), chu kỳ đầu tính từ lần vẽ đầu tiên trong phiên. Tự xin vẽ lại khi đang chuyển động.
pub fn ve(p: &Painter, r: Rect) {
    let ctx = p.ctx();
    let (nen, sao) = anh(ctx);
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    p.image(nen.id(), r, uv, Color32::WHITE);

    let bay_gio = ctx.input(|i| i.time);
    let id_t0 = egui::Id::unique("logo_sao_mai_t0");
    let t0 = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(id_t0, || bay_gio));
    let t = (bay_gio - t0).rem_euclid(CHU_KY);
    let f = r.width() / KICH[0];
    let diem = |(x, y): (f32, f32)| egui::pos2(r.left() + x * f, r.top() + y * f);

    // Ngôi sao cam: mờ còn 20% khi chấm sáng bắt đầu vuốt; tới nơi sáng lại và phóng 1,5 lần rồi về 1
    let (do_ro, ty_le) = if t < T_VUOT {
        ((1.0 - t / 0.15 * 0.8).max(0.2) as f32, 1.0)
    } else if t < T_VUOT + T_NHAY {
        let s = ((t - T_VUOT) / T_NHAY) as f32;
        ((0.2 + 0.8 * s / 0.4).min(1.0), 1.0 + 0.5 * (std::f32::consts::PI * s).sin())
    } else {
        (1.0, 1.0)
    };
    let tam = diem(TAM_SAO);
    let o = Rect::from_min_size(diem((O_SAO.0, O_SAO.1)), egui::vec2(O_SAO.2, O_SAO.2) * f);
    let o = Rect::from_center_size(tam + (o.center() - tam) * ty_le, o.size() * ty_le);
    p.image(sao.id(), o, uv, mo(Color32::WHITE, do_ro));

    if t < T_VUOT {
        // Chấm sáng + vệt; lớn dần ở 25% quãng cuối (tới sát ngôi sao mới to)
        let u = em((t / T_VUOT) as f32);
        for k in (1..=6).rev() {
            let v = u - k as f32 * 0.04;
            if v > 0.0 {
                p.circle_filled(diem(bezier(v)), (2.0 + 5.0 * (1.0 - k as f32 / 7.0)) * f, mo(VANG, 0.55 * (1.0 - k as f32 / 7.0)));
            }
        }
        let to = 1.0 + 1.4 * ((u - 0.75) / 0.25).max(0.0);
        let dau = diem(bezier(u));
        p.circle_filled(dau, 9.0 * f * to, mo(VANG, 0.45));
        p.circle_filled(dau, 4.5 * f * to, Color32::WHITE);
        ctx.request_repaint();
    } else if t < T_VUOT + T_NHAY {
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(CHU_KY - t));
    }
}
