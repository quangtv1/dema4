//! Phiên phân tích 1 thư mục đầu vào: tìm PDF, nạp cache hoặc đọc nhanh dung lượng, liệt kê trang còn thiếu
//! kết quả, nhận kết quả từ hàng đợi, ghi cache, xuất nhãn tự động. Dùng cho CLI `--tach` và giao diện.

use super::cache_trang::{self, KhoaFile, TrangCache};
use super::dung_luong::doc_thong_tin_file;
use super::hang_doi::ViecTrang;
use super::huong_trang::can_tang_2;
use super::nhan::{duong_dan_ghi, BangNhan, Nhan};
use super::phan_tich::{gan_nhan, CauHinhPhanTich, KetQuaTrang};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Thư mục ẩn cạnh dữ liệu: nhãn, góc xoay, log tách, cache.
pub fn thu_muc_an(goc: &Path) -> PathBuf {
    goc.join(".sm-dema4")
}

pub struct HoSoPhanTich {
    pub pdf: PathBuf,
    /// Đường dẫn tương đối từ thư mục gốc, dùng `/` (ghi vào file nhãn / cache).
    pub tuong_doi: String,
    pub khoa: Option<KhoaFile>,
    pub trang: Vec<TrangCache>,
    /// Lỗi mở / đọc file (hồ sơ vẫn nằm trong danh sách, không có trang).
    pub loi: Option<String>,
}

pub struct PhienPhanTich {
    pub goc: PathBuf,
    pub ho_so: Vec<HoSoPhanTich>,
}

/// Sự kiện danh sách xem trước (đọc nền khi chọn thư mục).
pub enum XemTruocSuKien {
    /// PDF của 1 thư mục vừa đọc xong (đường dẫn đầy đủ, theo thứ tự hiển thị).
    Them(Vec<PathBuf>),
    /// Đã duyệt hết cây thư mục.
    DaDuyet,
    /// Số trang (theo cache) của mọi file, cùng thứ tự.
    SoTrang(Vec<Option<usize>>),
}

/// So tên kiểu tự nhiên, không phân biệt hoa thường: cụm số so theo giá trị ("HS 2" trước "HS 10").
pub fn so_ten_tu_nhien(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let lay = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().filter(|c| c.is_ascii_digit()).copied() {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (lay(&mut x), lay(&mut y));
                let (m2, n2) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = m2.len().cmp(&n2.len()).then(m2.cmp(n2)).then(m.len().cmp(&n.len()));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Duyệt cây thư mục theo thứ tự hiển thị: trong mỗi thư mục, các PDF (tên tự nhiên) trước, rồi lần lượt từng
/// thư mục con (tên tự nhiên, đệ quy). Mỗi thư mục có PDF gọi `moi_thu_muc` 1 lần để giao diện hiện dần ngay.
/// Bỏ thư mục ẩn / hệ thống (như màn Đếm), không đi theo symlink; thư mục gốc người dùng chọn luôn được duyệt.
pub fn duyet_pdf_theo_cay(goc: &Path, dung: &AtomicBool, mut moi_thu_muc: impl FnMut(Vec<PathBuf>)) {
    let mut ngan_xep = vec![goc.to_path_buf()];
    while let Some(tm) = ngan_xep.pop() {
        if dung.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let Ok(doc) = std::fs::read_dir(&tm) else { continue };
        let (mut pdf, mut con) = (Vec::new(), Vec::new());
        for m in doc.flatten() {
            let Ok(loai) = m.file_type() else { continue };
            let ten = m.file_name();
            if loai.is_dir() {
                if !crate::dem_pdf::bo_qua_thu_muc(&ten) {
                    con.push(m.path());
                }
            } else if loai.is_file() && ten.to_string_lossy().to_lowercase().ends_with(".pdf") {
                pdf.push(m.path());
            }
        }
        let ten = |p: &PathBuf| p.file_name().map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
        pdf.sort_by(|a, b| so_ten_tu_nhien(&ten(a), &ten(b)));
        con.sort_by(|a, b| so_ten_tu_nhien(&ten(b), &ten(a))); // ngăn xếp: đảo để lấy ra theo thứ tự tên
        ngan_xep.extend(con);
        if !pdf.is_empty() {
            moi_thu_muc(pdf);
        }
    }
}

impl PhienPhanTich {
    /// Mở thư mục: file có cache khớp → dùng ngay; chưa có → đọc dung lượng + kích thước (không render).
    pub fn mo(goc: &Path, dung: &AtomicBool) -> Self {
        let mut pdf = Vec::new();
        duyet_pdf_theo_cay(goc, dung, |ds| pdf.extend(ds));
        Self::mo_tu_ds(goc, pdf)
    }

    /// Như `mo` nhưng dùng danh sách PDF đã duyệt sẵn (bước xem trước) — khỏi duyệt lại cây thư mục.
    pub fn mo_tu_ds(goc: &Path, pdf: Vec<PathBuf>) -> Self {
        let cache = thu_muc_an(goc).join("cache");
        let ho_so = pdf
            .into_par_iter()
            .map(|pdf| {
                let tuong_doi = duong_dan_ghi(goc, &pdf);
                let khoa = KhoaFile::cua(&pdf, &tuong_doi);
                if let Some(trang) = khoa.as_ref().and_then(|k| cache_trang::doc(&cache, k)) {
                    return HoSoPhanTich { pdf, tuong_doi, khoa, trang, loi: None };
                }
                match doc_thong_tin_file(&pdf) {
                    Ok(tt) => HoSoPhanTich { pdf, tuong_doi, khoa, trang: tt.into_iter().map(|thong_tin| TrangCache { thong_tin, ..Default::default() }).collect(), loi: None },
                    Err(e) => HoSoPhanTich { pdf, tuong_doi, khoa, trang: Vec::new(), loi: Some(e) },
                }
            })
            .collect();
        Self { goc: goc.to_path_buf(), ho_so }
    }

    /// Xem trước khi Phân tích: gửi tên PDF từng thư mục ngay khi đọc xong thư mục đó (hiện dần, đúng thứ tự cây),
    /// rồi số trang của các file đã có cache. Không mở PDF nào.
    pub fn xem_truoc(goc: &Path, dung: &AtomicBool, gui: &std::sync::mpsc::Sender<XemTruocSuKien>) {
        let mut tat_ca = Vec::new();
        duyet_pdf_theo_cay(goc, dung, |ds| {
            tat_ca.extend(ds.iter().cloned());
            let _ = gui.send(XemTruocSuKien::Them(ds));
        });
        let _ = gui.send(XemTruocSuKien::DaDuyet);
        let cache = thu_muc_an(goc).join("cache");
        let so: Vec<Option<usize>> = tat_ca
            .par_iter()
            .map(|pdf| KhoaFile::cua(pdf, &duong_dan_ghi(goc, pdf)).and_then(|k| cache_trang::doc(&cache, &k)).map(|t| t.len()))
            .collect();
        let _ = gui.send(XemTruocSuKien::SoTrang(so));
    }

    pub fn duong_dan_pdf(&self) -> Vec<PathBuf> {
        self.ho_so.iter().map(|h| h.pdf.clone()).collect()
    }

    /// Trang cần (chạy lại) phân tích với Cấu hình `ch`: chưa có kết quả, thiếu loại quét vừa bật,
    /// hoặc trang nội dung cần tầng 2 hướng sau khi tăng độ tin cậy. Bỏ trang đã xoá khỏi lưới.
    pub fn viec_con_thieu(&self, ch: &CauHinhPhanTich) -> Vec<ViecTrang> {
        let mut ds = Vec::new();
        for (file, hs) in self.ho_so.iter().enumerate() {
            for (i, t) in hs.trang.iter().enumerate() {
                if t.trang_thai & cache_trang::DA_XOA != 0 {
                    continue;
                }
                let thieu = match &t.ket_qua {
                    None => ch.quet_trang || ch.quet_ma || ch.quet_xoay,
                    Some(kq) => {
                        let noi_dung = { let n = gan_nhan(kq, &CauHinhPhanTich { quet_xoay: false, ..ch.clone() }); n.nhan == Nhan::NoiDung && !n.nghi_trang };
                        (ch.quet_ma && !kq.da_quet_ma)
                            || (ch.quet_trang && kq.chi_so.is_none())
                            || (ch.quet_xoay && noi_dung && kq.huong.map_or(true, |h| can_tang_2(&h, ch.tin_cay_xoay)))
                    }
                };
                if thieu {
                    ds.push(ViecTrang { file, trang: i + 1 });
                }
            }
        }
        ds
    }

    /// Ghi kết quả 1 trang từ hàng đợi (giữ trạng thái duyệt của người dùng).
    pub fn nhan_ket_qua(&mut self, viec: ViecTrang, kq: KetQuaTrang) {
        if let Some(t) = self.ho_so.get_mut(viec.file).and_then(|h| h.trang.get_mut(viec.trang - 1)) {
            t.ket_qua = Some(kq);
        }
    }

    /// Ghi cache mọi hồ sơ (song song). Trả số file ghi lỗi (thư mục chỉ đọc… — app vẫn chạy không cache).
    pub fn ghi_cache(&self) -> usize {
        let cache = thu_muc_an(&self.goc).join("cache");
        self.ho_so.par_iter().filter(|h| h.khoa.as_ref().is_some_and(|k| cache_trang::ghi(&cache, k, &h.trang).is_err())).count()
    }

    /// Nhãn tự động theo Cấu hình (chỉ index / trắng; trang nội dung không ghi).
    pub fn bang_nhan_tu_dong(&self, ch: &CauHinhPhanTich) -> BangNhan {
        let mut bang = BangNhan::new();
        for hs in &self.ho_so {
            for (i, t) in hs.trang.iter().enumerate() {
                let Some(kq) = &t.ket_qua else { continue };
                let n = gan_nhan(kq, ch).nhan;
                if n != Nhan::NoiDung {
                    bang.entry(hs.tuong_doi.clone()).or_default().insert(i + 1, n);
                }
            }
        }
        bang
    }
}

/// Phân tích cả thư mục không cần giao diện (CLI): chạy hàng đợi cho trang còn thiếu, ghi cache, trả phiên.
pub fn phan_tich_thu_muc(goc: &Path, ch: &CauHinhPhanTich, tien_do: &dyn Fn(usize, usize)) -> PhienPhanTich {
    use super::hang_doi::{chay, HangDoi, SuKienPhanTich};
    let dung = AtomicBool::new(false);
    let mut phien = PhienPhanTich::mo(goc, &dung);
    let viec = phien.viec_con_thieu(ch);
    let tong = viec.len();
    if tong > 0 {
        let so_luong = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let (gui, nhan) = std::sync::mpsc::channel();
        chay(HangDoi::moi(viec), std::sync::Arc::new(phien.duong_dan_pdf()), ch.clone(), so_luong, gui);
        let mut xong = 0;
        for sk in nhan {
            match sk {
                SuKienPhanTich::Trang { viec, kq, .. } => phien.nhan_ket_qua(viec, kq),
                SuKienPhanTich::Loi { .. } => {}
                SuKienPhanTich::Xong => break,
            }
            xong += 1;
            tien_do(xong, tong);
        }
    }
    phien.ghi_cache();
    phien
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duyet_cay_pdf_truoc_roi_thu_muc_con_ten_tu_nhien() {
        let goc = std::env::temp_dir().join(format!("dema4_duyet_cay_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        for d in ["HS 10", "HS 2", "HS 2/con", ".an"] {
            std::fs::create_dir_all(goc.join(d)).unwrap();
        }
        for f in ["b.pdf", "a10.PDF", "a2.pdf", "HS 10/x.pdf", "HS 2/y.pdf", "HS 2/con/z.pdf", ".an/an.pdf", "HS 2/ghi.txt"] {
            std::fs::write(goc.join(f), b"").unwrap();
        }
        let mut lo = Vec::new();
        duyet_pdf_theo_cay(&goc, &AtomicBool::new(false), |ds| lo.push(ds.iter().map(|p| p.strip_prefix(&goc).unwrap().to_string_lossy().replace('\\', "/")).collect::<Vec<_>>()));
        assert_eq!(lo, vec![vec!["a2.pdf", "a10.PDF", "b.pdf"], vec!["HS 2/y.pdf"], vec!["HS 2/con/z.pdf"], vec!["HS 10/x.pdf"]]);
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn mo_lan_2_dung_cache_khong_phan_tich_lai_va_bat_them_loai_quet() {
        let goc = std::env::temp_dir().join(format!("dema4_phien_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join("con")).unwrap();
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("con/a.pdf"), 3);
        let chi_trang = CauHinhPhanTich { quet_ma: false, quet_xoay: false, ..Default::default() };
        let p1 = phan_tich_thu_muc(&goc, &chi_trang, &|_, _| {});
        assert_eq!(p1.ho_so[0].tuong_doi, "con/a.pdf");
        assert!(p1.ho_so[0].trang.iter().all(|t| t.ket_qua.as_ref().is_some_and(|k| k.chi_so.is_some() && !k.da_quet_ma)));
        // Mở lại: đủ kết quả từ cache, không còn việc
        let p2 = PhienPhanTich::mo(&goc, &AtomicBool::new(false));
        assert!(p2.viec_con_thieu(&chi_trang).is_empty());
        assert_eq!(p2.bang_nhan_tu_dong(&chi_trang)["con/a.pdf"].len(), 3, "3 trang chữ nhỏ → Trắng");
        // Bật thêm quét mã → mọi trang cần chạy (thiếu mã)
        assert_eq!(p2.viec_con_thieu(&CauHinhPhanTich { quet_xoay: false, ..Default::default() }).len(), 3);
        // PDF đổi → cache bị bỏ
        crate::tach_vb::tach::tests::tao_pdf(&goc.join("con/a.pdf"), 4);
        let p3 = PhienPhanTich::mo(&goc, &AtomicBool::new(false));
        assert_eq!(p3.viec_con_thieu(&chi_trang).len(), 4);
        let _ = std::fs::remove_dir_all(&goc);
    }
}
