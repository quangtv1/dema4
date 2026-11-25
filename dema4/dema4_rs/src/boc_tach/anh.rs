//! Render 1 trang PDF → JPEG gửi mô hình đọc (DPI, cạnh dài tối đa, chất lượng theo Cấu hình › Nâng cao).

use base64::Engine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CauHinhAnh {
    pub dpi: f32,
    /// Cạnh dài tối đa (px): trang khổ lớn bị thu lại để không tốn token / vượt giới hạn ảnh của dịch vụ.
    pub canh_toi_da: u32,
    /// Chất lượng JPEG 1..100.
    pub chat_luong: u8,
}

impl Default for CauHinhAnh {
    fn default() -> Self {
        Self { dpi: 200.0, canh_toi_da: 2048, chat_luong: 90 }
    }
}

/// JPEG của trang `trang` (đánh số từ 1).
pub fn jpeg_trang(doc: &mupdf::Document, trang: usize, ch: &CauHinhAnh) -> Result<Vec<u8>, String> {
    let p = doc.load_page(trang as i32 - 1).map_err(|e| format!("không mở được trang {trang}: {e}"))?;
    let b = p.bounds().map_err(|e| e.to_string())?;
    let canh_pt = (b.x1 - b.x0).max(b.y1 - b.y0).max(1.0);
    let s = (ch.dpi / 72.0).min(ch.canh_toi_da as f32 / canh_pt);
    let pix = p.to_pixmap(&mupdf::Matrix::new_scale(s, s), &mupdf::Colorspace::device_rgb(), false, false).map_err(|e| e.to_string())?;
    let (rong, cao, n) = (pix.width() as usize, pix.height() as usize, pix.n() as usize);
    let rgb: Vec<u8> = if n == 3 { pix.samples().to_vec() } else { pix.samples().chunks_exact(n).flat_map(|c| [c[0], c[1], c[2]]).collect() };
    let mut ra = Vec::new();
    jpeg_encoder::Encoder::new(&mut ra, ch.chat_luong.clamp(1, 100))
        .encode(&rgb[..rong * cao * 3], rong as u16, cao as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| format!("không mã hoá được JPEG: {e}"))?;
    Ok(ra)
}

pub fn base64(jpeg: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_khong_vuot_canh_toi_da() {
        let d = std::env::temp_dir().join(format!("dema4_bt_anh_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.pdf");
        crate::tach_vb::tach::tests::tao_pdf(&f, 2);
        let doc = mupdf::Document::open(f.to_str().unwrap()).unwrap();
        let ch = CauHinhAnh { dpi: 200.0, canh_toi_da: 800, chat_luong: 80 };
        let j = jpeg_trang(&doc, 2, &ch).unwrap();
        assert_eq!(&j[..2], [0xFF, 0xD8], "đầu file JPEG");
        // Kích thước trong khung SOF0: FF C0 len(2) bits(1) cao(2) rong(2)
        let i = j.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        let (cao, rong) = (u16::from_be_bytes([j[i + 5], j[i + 6]]), u16::from_be_bytes([j[i + 7], j[i + 8]]));
        assert_eq!(cao.max(rong), 800);
        assert!((rong as f32 / cao as f32 - 595.0 / 842.0).abs() < 0.01);
        assert!(jpeg_trang(&doc, 3, &ch).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
