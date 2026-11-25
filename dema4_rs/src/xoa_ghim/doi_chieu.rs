//! Đối chiếu với XoaGhim (Python) trên bộ mẫu thật — chạy tay:
//! `DEMA4_DOI_CHIEU_XG=<thư mục> cargo test doi_chieu -- --ignored --nocapture`.
//!
//! Thư mục do script Python sinh: `doi_chieu.json` (mỗi trang: file, trang, dpi, vùng như hàng loạt
//! của XoaGhim) + `<file>/<trang>_vao.png` (ảnh trang đã vẽ) + `<trang>_ra.png` (kết quả Python).
//! Rust xử lý đúng ảnh vào đó (không tự vẽ lại) nên so được tới từng điểm.

use super::vung::{ApTrang, LoaiVung, ViTri, Vung};
use super::xu_ly::{xoa_ghim_trang, TuyChon};
use crate::tach_vb::anh::Anh;
use rayon::prelude::*;
use std::path::Path;

fn doc_png(p: &Path) -> Anh {
    let mut gm = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(p).unwrap())).read_info().unwrap();
    let mut buf = vec![0; gm.output_buffer_size().unwrap()];
    let tt = gm.next_frame(&mut buf).unwrap();
    assert_eq!(tt.color_type, png::ColorType::Rgb, "{}", p.display());
    buf.truncate(tt.buffer_size());
    Anh { rong: tt.width as usize, cao: tt.height as usize, kenh: 3, du_lieu: buf }
}

fn vung_tu_json(z: &serde_json::Value) -> Vung {
    let s = |k: &str| z[k].as_str().unwrap();
    let f = |k: &str| z[k].as_f64().unwrap();
    let vi_tri = match s("id") {
        "corner_tl" => ViTri::GocTrenTrai,
        "corner_tr" => ViTri::GocTrenPhai,
        "corner_bl" => ViTri::GocDuoiTrai,
        "corner_br" => ViTri::GocDuoiPhai,
        "margin_left" => ViTri::CanhTrai,
        "margin_right" => ViTri::CanhPhai,
        "margin_top" => ViTri::CanhTren,
        "margin_bottom" => ViTri::CanhDuoi,
        _ => ViTri::Tay,
    };
    let loai = match s("zone_type") {
        "protect" => LoaiVung::BaoVe,
        "remove_override" => LoaiVung::VoDoi,
        _ => LoaiVung::Xoa,
    };
    let trang = z["target_page"].as_i64().unwrap();
    let ap_trang = if trang >= 0 {
        ApTrang::MotTrang(trang as u32)
    } else {
        match s("page_filter") {
            "odd" => ApTrang::Le,
            "even" => ApTrang::Chan,
            _ => ApTrang::TatCa,
        }
    };
    Vung {
        vi_tri,
        loai,
        ap_trang,
        x: f("x"),
        y: f("y"),
        rong: f("width"),
        cao: f("height"),
        rong_px: z["width_px"].as_u64().unwrap() as u32,
        cao_px: z["height_px"].as_u64().unwrap() as u32,
        nguong: z["threshold"].as_u64().unwrap() as u8,
    }
}

/// Mặt nạ xoá (điểm đổi so với ảnh vào) của Python và Rust: (số điểm giao, số điểm hợp).
fn so_mat_na(vao: &Anh, py: &Anh, rs: &Anh) -> (usize, usize) {
    let (mut giao, mut hop) = (0, 0);
    for ((a, b), c) in vao.du_lieu.chunks_exact(3).zip(py.du_lieu.chunks_exact(3)).zip(rs.du_lieu.chunks_exact(3)) {
        let (p, r) = (a != b, a != c);
        giao += (p && r) as usize;
        hop += (p || r) as usize;
    }
    (giao, hop)
}

#[test]
#[ignore]
fn doi_chieu_xoaghim_python_tren_bo_mau() {
    let Some(goc) = std::env::var_os("DEMA4_DOI_CHIEU_XG").map(std::path::PathBuf::from) else {
        eprintln!("bỏ qua: chưa đặt DEMA4_DOI_CHIEU_XG");
        return;
    };
    let ds: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(goc.join("doi_chieu.json")).unwrap()).unwrap();
    let bat_dau = std::time::Instant::now();
    // (file, trang, khớp từng điểm, IoU, số điểm khác)
    let kq: Vec<(String, u32, bool, f64, usize)> = ds
        .par_iter()
        .map(|m| {
            let file = m["file"].as_str().unwrap().to_string();
            let trang = m["trang"].as_u64().unwrap() as u32;
            let dpi = m["dpi"].as_f64().unwrap();
            let vung: Vec<Vung> = m["vung"].as_array().unwrap().iter().map(vung_tu_json).collect();
            let vao = doc_png(&goc.join(&file).join(format!("{trang}_vao.png")));
            let py = doc_png(&goc.join(&file).join(format!("{trang}_ra.png")));
            let mut rs = vao.clone();
            xoa_ghim_trang(&mut rs, &vung, trang, dpi, &TuyChon::default(), &[]);
            let khac = rs.du_lieu.chunks_exact(3).zip(py.du_lieu.chunks_exact(3)).filter(|(a, b)| a != b).count();
            let (giao, hop) = so_mat_na(&vao, &py, &rs);
            let iou = if hop == 0 { 1.0 } else { giao as f64 / hop as f64 };
            (file, trang, khac == 0, iou, khac)
        })
        .collect();
    let khop = kq.iter().filter(|k| k.2).count();
    let iou_min = kq.iter().map(|k| k.3).fold(1.0, f64::min);
    for k in kq.iter().filter(|k| !k.2) {
        eprintln!("khác: {} trang {} — {} điểm, IoU {:.5}", k.0, k.1, k.4, k.3);
    }
    eprintln!("{} trang: khớp từng điểm {khop}, IoU nhỏ nhất {iou_min:.5}; {:.1}s", kq.len(), bat_dau.elapsed().as_secs_f64());
    assert!(iou_min >= 0.99, "IoU mặt nạ xoá < 0,99");
}
