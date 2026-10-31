//! Đọc 1 file PDF (mỗi trang cần đọc 1 yêu cầu, gộp theo trường) và hàng đợi nhiều file chạy song song trên luồng nền.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use super::anh::{jpeg_trang, CauHinhAnh};
use super::cau_lenh::cau_lenh;
use super::chuan_hoa::chuan_hoa;
use super::doc_json;
use super::loai_tai_lieu::{LayTrang, LoaiTaiLieu};
use super::mo_hinh::{LoiDoc, MoHinhDoc};

/// Kết quả 1 file.
#[derive(Debug, Clone, PartialEq)]
pub struct KetQua {
    pub so_trang: usize,
    /// Giá trị đã chuẩn hoá, theo thứ tự trường của loại tài liệu.
    pub gia_tri: Vec<String>,
    /// Câu trả lời thô từng trang đã đọc (trang, nội dung) — ghi log để soát khi mô hình trả sai.
    pub tho: Vec<(usize, String)>,
    pub thoi_gian: Duration,
}

impl KetQua {
    /// Số trường có giá trị (hiện "x/6" ở cột Trường).
    pub fn so_co(&self) -> usize {
        self.gia_tri.iter().filter(|g| !g.is_empty()).count()
    }
}

/// Số lần thử lại khi quá tải (429 / 503) và khi lỗi mạng.
const THU_LAI_QUA_TAI: u32 = 3;
const THU_LAI_MANG: u32 = 1;

/// Gọi mô hình, thử lại khi quá tải (chờ `Retry-After`, không có: 2, 4, 8 s) / lỗi mạng (1 lần). `dung` bật → thôi chờ.
fn goi(mo_hinh: &dyn MoHinhDoc, cl: &str, jpeg: &[u8], dung: &AtomicBool) -> Result<String, LoiDoc> {
    let (mut qua_tai, mut mang) = (0, 0);
    loop {
        match mo_hinh.doc(cl, jpeg) {
            Err(LoiDoc::QuaTai { cho }) if qua_tai < THU_LAI_QUA_TAI => {
                let cho = cho.unwrap_or(Duration::from_secs(2 << qua_tai));
                qua_tai += 1;
                let het = Instant::now() + cho;
                while Instant::now() < het {
                    if dung.load(Ordering::Relaxed) {
                        return Err(LoiDoc::QuaTai { cho: None });
                    }
                    std::thread::sleep(Duration::from_millis(100).min(het - Instant::now()));
                }
            }
            Err(LoiDoc::Mang(_)) if mang < THU_LAI_MANG => mang += 1,
            kq => return kq,
        }
    }
}

/// Đọc 1 file: render các trang theo loại tài liệu, gọi mô hình từng trang, gộp + chuẩn hoá.
/// Gộp: trường nhắc "trang cuối" lấy ở trang cuối đã đọc, còn lại ở trang đầu; trống thì lấy trang khác đầu tiên có giá trị.
pub fn doc_file(pdf: &Path, loai: &LoaiTaiLieu, anh: &CauHinhAnh, mo_hinh: &dyn MoHinhDoc, dung: &AtomicBool) -> Result<KetQua, LoiDoc> {
    let bat_dau = Instant::now();
    let doc = mupdf::Document::open(pdf).map_err(|e| LoiDoc::TraLoi(format!("Không mở được PDF: {e}")))?;
    let so_trang = doc.page_count().map_err(|e| LoiDoc::TraLoi(format!("Không đọc được số trang: {e}")))? as usize;
    let trang = loai.trang_doc.cac_trang(so_trang);
    if trang.is_empty() {
        return Err(LoiDoc::TraLoi("PDF không có trang nào".into()));
    }
    let mut tho = Vec::new();
    let mut theo_trang = Vec::new();
    for &t in &trang {
        let jpeg = jpeg_trang(&doc, t, anh).map_err(LoiDoc::TraLoi)?;
        let tl = goi(mo_hinh, &cau_lenh(loai, t, so_trang), &jpeg, dung)?;
        let m = doc_json::doc(&tl);
        tho.push((t, tl));
        theo_trang.push(m.ok_or_else(|| LoiDoc::TraLoi(format!("Mô hình không trả JSON ở trang {t}")))?);
    }
    let gia_tri = loai
        .truong
        .iter()
        .map(|tr| {
            let uu_tien = if tr.lay_trang() == LayTrang::Cuoi { theo_trang.len() - 1 } else { 0 };
            let lay = |i: usize| chuan_hoa(&doc_json::chuoi(&theo_trang[i], &tr.bien), tr);
            let g = lay(uu_tien);
            if !g.is_empty() {
                return g;
            }
            (0..theo_trang.len()).filter(|&i| i != uu_tien).map(lay).find(|g| !g.is_empty()).unwrap_or_default()
        })
        .collect();
    Ok(KetQua { so_trang, gia_tri, tho, thoi_gian: bat_dau.elapsed() })
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuKien {
    /// Bắt đầu đọc file thứ i (chỉ số trong danh sách gửi vào).
    BatDau(usize),
    Xong(usize, KetQua),
    Loi(usize, String),
    /// Hàng đợi dừng sớm (sai khoá, mất kết nối liên tiếp); các file chưa đọc giữ nguyên.
    DungSom(String),
    /// Luôn gửi cuối cùng.
    HetHang,
}

/// Mất kết nối liên tiếp bao nhiêu file thì dừng hàng đợi.
const LOI_MANG_LIEN_TIEP: usize = 3;

/// Chạy hàng đợi `files` (chỉ số, đường dẫn) với `song_song` luồng; chặn tới khi xong — gọi trong luồng nền.
/// `dung` bật → không lấy file mới (file đang đọc đọc nốt).
pub fn chay_hang(files: &[(usize, PathBuf)], loai: &LoaiTaiLieu, anh: &CauHinhAnh, mo_hinh: &dyn MoHinhDoc, song_song: usize, dung: &AtomicBool, gui: &Sender<SuKien>) {
    let tiep = AtomicUsize::new(0);
    let mang_lien_tiep = AtomicUsize::new(0);
    let dung_som = AtomicBool::new(false);
    std::thread::scope(|s| {
        for _ in 0..song_song.clamp(1, 16).min(files.len().max(1)) {
            s.spawn(|| loop {
                if dung.load(Ordering::Relaxed) || dung_som.load(Ordering::Relaxed) {
                    return;
                }
                let Some((i, pdf)) = files.get(tiep.fetch_add(1, Ordering::Relaxed)) else { return };
                let _ = gui.send(SuKien::BatDau(*i));
                match doc_file(pdf, loai, anh, mo_hinh, dung) {
                    Ok(kq) => {
                        mang_lien_tiep.store(0, Ordering::Relaxed);
                        let _ = gui.send(SuKien::Xong(*i, kq));
                    }
                    Err(e) => {
                        let _ = gui.send(SuKien::Loi(*i, e.to_string()));
                        let ly_do = match &e {
                            LoiDoc::Khoa(_) => Some(e.to_string()),
                            LoiDoc::Mang(_) if mang_lien_tiep.fetch_add(1, Ordering::Relaxed) + 1 >= LOI_MANG_LIEN_TIEP => {
                                Some(format!("Mất kết nối {LOI_MANG_LIEN_TIEP} file liên tiếp — kiểm tra máy chủ / mạng rồi Chạy lại"))
                            }
                            _ => None,
                        };
                        if let Some(l) = ly_do {
                            if !dung_som.swap(true, Ordering::Relaxed) {
                                let _ = gui.send(SuKien::DungSom(l));
                            }
                        }
                    }
                }
            });
        }
    });
    let _ = gui.send(SuKien::HetHang);
}

#[cfg(test)]
mod tests {
    use super::super::loai_tai_lieu::co_san;
    use super::*;
    use std::sync::mpsc;
    use std::sync::Mutex;

    /// Mô hình giả: trả lời theo câu lệnh (biết trang đầu / cuối / file) qua hàm do test đặt; đếm số lần gọi.
    struct Gia<F: Fn(&str) -> Result<String, LoiDoc> + Send + Sync> {
        f: F,
        so_lan: Mutex<usize>,
    }
    impl<F: Fn(&str) -> Result<String, LoiDoc> + Send + Sync> MoHinhDoc for Gia<F> {
        fn doc(&self, cl: &str, jpeg: &[u8]) -> Result<String, LoiDoc> {
            assert_eq!(&jpeg[..2], [0xFF, 0xD8]);
            *self.so_lan.lock().unwrap() += 1;
            (self.f)(cl)
        }
    }
    fn gia<F: Fn(&str) -> Result<String, LoiDoc> + Send + Sync>(f: F) -> Gia<F> {
        Gia { f, so_lan: Mutex::new(0) }
    }

    fn thu_muc(ten: &str, files: &[(&str, usize)]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_bt_{ten}_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for (f, n) in files {
            crate::tach_vb::tach::tests::tao_pdf(&d.join(f), *n);
        }
        d
    }

    const TRANG_DAU: &str = r#"{"loai_van_ban": "Loại văn bản: Báo cáo", "so_ky_hieu": "Số 36/BC-NTTS", "ngay_thang": "6 tháng 4 năm 2015", "tac_gia": "Chi cục A", "trich_yeu": "Kết quả thực hiện", "nguoi_ky": ""}"#;
    const TRANG_CUOI: &str = r#"{"loai_van_ban": "", "so_ky_hieu": "", "ngay_thang": "", "tac_gia": "", "trich_yeu": "", "nguoi_ky": "NGUYỄN VĂN A"}"#;

    #[test]
    fn doc_file_gop_dau_cuoi_va_chuan_hoa() {
        let d = thu_muc("gop", &[("a.pdf", 4)]);
        let m = gia(|cl| Ok(if cl.contains("TRANG ĐẦU") { TRANG_DAU.into() } else { format!("```json\n{TRANG_CUOI}\n```") }));
        let kq = doc_file(&d.join("a.pdf"), &co_san()[0], &CauHinhAnh { canh_toi_da: 400, ..Default::default() }, &m, &AtomicBool::new(false)).unwrap();
        assert_eq!(kq.so_trang, 4);
        assert_eq!(kq.gia_tri, ["Báo cáo", "36/BC-NTTS", "06/04/2015", "Chi cục A", "Kết quả thực hiện", "NGUYỄN VĂN A"]);
        assert_eq!(kq.so_co(), 6);
        assert_eq!(kq.tho.iter().map(|t| t.0).collect::<Vec<_>>(), [1, 4], "đầu + cuối, mỗi trang 1 yêu cầu");
        assert_eq!(*m.so_lan.lock().unwrap(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn doc_file_khong_json_la_loi() {
        let d = thu_muc("khongjson", &[("a.pdf", 1)]);
        let m = gia(|_| Ok("Tôi không đọc được".into()));
        let e = doc_file(&d.join("a.pdf"), &co_san()[0], &CauHinhAnh { canh_toi_da: 300, ..Default::default() }, &m, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(e, LoiDoc::TraLoi("Mô hình không trả JSON ở trang 1".into()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn qua_tai_cho_roi_thu_lai() {
        let d = thu_muc("quatai", &[("a.pdf", 1)]);
        let lan = Mutex::new(0);
        let m = gia(|_| {
            let mut l = lan.lock().unwrap();
            *l += 1;
            if *l <= 2 { Err(LoiDoc::QuaTai { cho: Some(Duration::ZERO) }) } else { Ok(TRANG_DAU.into()) }
        });
        let kq = doc_file(&d.join("a.pdf"), &co_san()[0], &CauHinhAnh { canh_toi_da: 300, ..Default::default() }, &m, &AtomicBool::new(false)).unwrap();
        assert_eq!(kq.gia_tri[1], "36/BC-NTTS");
        assert_eq!(*m.so_lan.lock().unwrap(), 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    fn chay(d: &Path, ten: &[&str], m: &dyn MoHinhDoc, song_song: usize) -> Vec<SuKien> {
        let files: Vec<(usize, PathBuf)> = ten.iter().enumerate().map(|(i, f)| (i, d.join(f))).collect();
        let (gui, nhan) = mpsc::channel();
        chay_hang(&files, &co_san()[0], &CauHinhAnh { canh_toi_da: 300, ..Default::default() }, m, song_song, &AtomicBool::new(false), &gui);
        drop(gui);
        nhan.iter().collect()
    }

    #[test]
    fn hang_doi_song_song_du_file_va_het_hang_cuoi() {
        let d = thu_muc("hang", &[("a.pdf", 1), ("b.pdf", 2), ("c.pdf", 1)]);
        let sk = chay(&d, &["a.pdf", "b.pdf", "khong_co.pdf", "c.pdf"], &gia(|_| Ok(TRANG_DAU.into())), 2);
        assert_eq!(sk.last(), Some(&SuKien::HetHang));
        assert_eq!(sk.iter().filter(|s| matches!(s, SuKien::BatDau(_))).count(), 4);
        assert_eq!(sk.iter().filter(|s| matches!(s, SuKien::Xong(..))).count(), 3);
        assert!(sk.iter().any(|s| matches!(s, SuKien::Loi(2, e) if e.contains("Không mở được PDF"))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sai_khoa_dung_hang_doi() {
        let d = thu_muc("khoa", &[("a.pdf", 1), ("b.pdf", 1), ("c.pdf", 1)]);
        let sk = chay(&d, &["a.pdf", "b.pdf", "c.pdf"], &gia(|_| Err(LoiDoc::Khoa("API key not valid".into()))), 1);
        assert_eq!(sk.iter().filter(|s| matches!(s, SuKien::BatDau(_))).count(), 1, "dừng ngay sau file đầu");
        assert!(sk.iter().any(|s| matches!(s, SuKien::DungSom(l) if l.contains("Khoá API"))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mat_ket_noi_3_file_lien_tiep_thi_dung() {
        let d = thu_muc("mang", &[("a.pdf", 1), ("b.pdf", 1), ("c.pdf", 1), ("e.pdf", 1)]);
        let m = gia(|_| Err(LoiDoc::Mang("connection refused".into())));
        let sk = chay(&d, &["a.pdf", "b.pdf", "c.pdf", "e.pdf"], &m, 1);
        assert_eq!(sk.iter().filter(|s| matches!(s, SuKien::BatDau(_))).count(), 3);
        assert!(sk.iter().any(|s| matches!(s, SuKien::DungSom(_))));
        assert_eq!(*m.so_lan.lock().unwrap(), 6, "mỗi file thử lại 1 lần khi lỗi mạng");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Gọi mô hình thật 1 file: `DEMA4_BT_NCC` (noi_bo|chat_gpt|gemini|claude|open_router), `DEMA4_BT_DIA_CHI`,
    /// `DEMA4_BT_KHOA`, `DEMA4_BT_MODEL`, `DEMA4_BT_PDF`. Địa chỉ / khoá chỉ qua biến môi trường.
    #[test]
    #[ignore]
    fn goi_that() {
        use super::super::mo_hinh::{KetNoi, NhaCungCap};
        let bien = |k: &str| std::env::var(k).unwrap_or_default();
        let ncc: NhaCungCap = serde_json::from_value(serde_json::json!(bien("DEMA4_BT_NCC"))).unwrap_or(NhaCungCap::NoiBo);
        let kn = KetNoi::moi(ncc, &bien("DEMA4_BT_DIA_CHI"), &bien("DEMA4_BT_KHOA"), &bien("DEMA4_BT_MODEL"));
        let loai = &co_san()[0];
        let kq = doc_file(Path::new(&bien("DEMA4_BT_PDF")), loai, &CauHinhAnh { dpi: 150.0, canh_toi_da: 1600, chat_luong: 85 }, &kn, &AtomicBool::new(false)).unwrap();
        for (t, tl) in &kq.tho {
            println!("--- trang {t} ---\n{tl}");
        }
        for (tr, g) in loai.truong.iter().zip(&kq.gia_tri) {
            println!("{:<14} {g}", tr.ten_cot);
        }
        println!("{}/{} trường · {} trang · {:.1} s", kq.so_co(), loai.truong.len(), kq.so_trang, kq.thoi_gian.as_secs_f32());
    }
}
