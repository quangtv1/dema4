//! "Làm tiếp gần đây" của Trang chủ: tối đa 5 thư mục / file vừa làm (chức năng + tình trạng lúc rời màn),
//! lưu ở `<gốc dữ liệu>/gan_day.json` — mỗi máy riêng, không vào gói Xuất phiên. File hỏng → danh sách rỗng.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const TOI_DA: usize = 5;
const TEN_FILE: &str = "gan_day.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChucNang {
    Dem,
    Tach,
    MucLuc,
    BocTach,
}

/// Tình trạng lúc rời màn: chữ + thanh tiến độ (khi `tong > 0`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TinhTrang {
    pub chu: String,
    #[serde(default)]
    pub xong: u32,
    #[serde(default)]
    pub tong: u32,
    /// Có lỗi (thanh tiến độ tô cam).
    #[serde(default)]
    pub loi: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MucGanDay {
    pub cn: ChucNang,
    /// Thư mục (Đếm, Tách, Bóc tách) hoặc file biên mục (Mục lục).
    pub duong_dan: String,
    /// Bóc tách: loại tài liệu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loai: Option<String>,
    /// Mục lục: mẫu (số mẫu có sẵn hoặc đường dẫn mẫu riêng), sheet, dòng tiêu đề.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mau: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dong_tieu_de: Option<u32>,
    pub tinh: TinhTrang,
    /// Lúc ghi (unix ms).
    pub luc: u64,
}

impl MucGanDay {
    pub fn moi(cn: ChucNang, duong_dan: impl Into<String>, tinh: TinhTrang) -> Self {
        MucGanDay { cn, duong_dan: duong_dan.into(), loai: None, mau: None, sheet: None, dong_tieu_de: None, tinh, luc: bay_gio_ms() }
    }

    /// Cùng chức năng + cùng thư mục / file → cùng 1 dòng.
    fn trung(&self, khac: &MucGanDay) -> bool {
        self.cn == khac.cn && Path::new(&self.duong_dan) == Path::new(&khac.duong_dan)
    }

    /// (thư mục cha, tên cuối) — tách theo cả `/` và `\` (đường dẫn Windows / ổ mạng như nhau trên mọi máy).
    fn tach_ten(&self) -> (&str, &str) {
        let p = self.duong_dan.trim().trim_end_matches(['/', '\\']);
        p.rfind(['/', '\\']).map_or(("", p), |i| (&p[..i], &p[i + 1..]))
    }

    /// Tên hiện đậm: tên thư mục / file cuối đường dẫn.
    pub fn ten(&self) -> String {
        self.tach_ten().1.to_string()
    }

    /// Thư mục cha (dòng mờ dưới tên).
    pub fn cha(&self) -> String {
        self.tach_ten().0.to_string()
    }
}

pub fn bay_gio_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub fn file_mac_dinh() -> PathBuf {
    crate::tach_vb::du_lieu_an::goc_du_lieu().join(TEN_FILE)
}

/// Đọc danh sách (mới nhất trước); thiếu / hỏng → rỗng.
pub fn doc(p: &Path) -> Vec<MucGanDay> {
    let mut ds: Vec<MucGanDay> = std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    ds.truncate(TOI_DA);
    ds
}

pub fn ghi(p: &Path, ds: &[MucGanDay]) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(ds).map_err(std::io::Error::other)?;
    crate::ghi_file::ghi_an_toan(p, json)
}

/// Đưa `muc` lên đầu (thay dòng cùng chức năng + đường dẫn), giữ tối đa 5.
pub fn them(ds: &mut Vec<MucGanDay>, muc: MucGanDay) {
    ds.retain(|m| !m.trung(&muc));
    ds.insert(0, muc);
    ds.truncate(TOI_DA);
}

/// "vừa xong", "5 phút trước", "2 giờ trước", "hôm qua", "3 ngày trước", rồi ngày.
pub fn thoi_gian_tuong_doi(luc: u64, bay_gio: u64) -> String {
    let phut = bay_gio.saturating_sub(luc) / 60_000;
    match phut {
        0 => "vừa xong".into(),
        1..=59 => format!("{phut} phút trước"),
        60..=1439 => format!("{} giờ trước", phut / 60),
        1440..=2879 => "hôm qua".into(),
        2880..=10079 => format!("{} ngày trước", phut / 1440),
        _ => {
            use chrono::TimeZone;
            chrono::Local.timestamp_millis_opt(luc as i64).single().map_or_else(String::new, |t| t.format("%d/%m/%Y").to_string())
        }
    }
}

/// Số có dấu chấm ngăn nghìn kiểu Việt Nam (1.204).
pub fn so(n: usize) -> String {
    let s = n.to_string();
    let mut kq = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            kq.push('.');
        }
        kq.push(c);
    }
    kq
}

/// Danh sách của App: ghi file sau mỗi thay đổi; `mat[i]` = đường dẫn dòng i không còn (kiểm ở luồng nền — ổ mạng /
/// Drive chưa gắn có thể chậm; chưa có kết quả thì vẽ dòng bình thường).
#[derive(Default)]
pub struct DanhSach {
    /// `None` (test) = chỉ giữ trong bộ nhớ, không đọc / ghi file.
    file: Option<PathBuf>,
    pub ds: Vec<MucGanDay>,
    pub mat: Vec<bool>,
    phien: u64,
    phien_mat: u64,
    kiem: Option<std::sync::mpsc::Receiver<(u64, Vec<bool>)>>,
}

impl DanhSach {
    pub fn mac_dinh() -> Self {
        if cfg!(test) {
            return Self::default();
        }
        let f = file_mac_dinh();
        DanhSach { ds: doc(&f), file: Some(f), phien: 1, ..Default::default() }
    }

    fn doi(&mut self) {
        self.phien += 1;
        if let Some(f) = &self.file {
            let _ = ghi(f, &self.ds);
        }
    }

    pub fn them(&mut self, muc: MucGanDay) {
        them(&mut self.ds, muc);
        self.doi();
    }

    pub fn xoa(&mut self, i: usize) {
        if i < self.ds.len() {
            self.ds.remove(i);
            self.doi();
        }
    }

    pub fn xoa_het(&mut self) {
        self.ds.clear();
        self.doi();
    }

    /// "Chọn lại…" thư mục / file của dòng i (đã đổi chỗ / đổi tên).
    pub fn doi_duong_dan(&mut self, i: usize, p: &Path) {
        if let Some(m) = self.ds.get(i).cloned() {
            self.ds.remove(i);
            them(&mut self.ds, MucGanDay { duong_dan: p.display().to_string(), luc: bay_gio_ms(), ..m });
            self.doi();
        }
    }

    pub fn mat(&self, i: usize) -> bool {
        self.mat.get(i).copied().unwrap_or(false)
    }

    /// Gọi mỗi khung hình: danh sách đổi → kiểm lại `exists()` ở luồng nền (chỉ ≤ 5 đường dẫn).
    pub fn cap_nhat_mat(&mut self, ctx: &eframe::egui::Context) {
        if let Some(n) = &self.kiem {
            match n.try_recv() {
                Ok((phien, mat)) => {
                    self.kiem = None;
                    if phien == self.phien {
                        self.mat = mat;
                        self.phien_mat = phien;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.kiem = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
            }
        }
        if self.phien_mat != self.phien && self.kiem.is_none() {
            let (gui, nhan) = std::sync::mpsc::channel();
            let (phien, ds): (u64, Vec<PathBuf>) = (self.phien, self.ds.iter().map(|m| PathBuf::from(m.duong_dan.trim())).collect());
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = gui.send((phien, ds.iter().map(|p| !p.exists()).collect()));
                ctx.request_repaint();
            });
            self.kiem = Some(nhan);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn muc(cn: ChucNang, p: &str) -> MucGanDay {
        MucGanDay::moi(cn, p, TinhTrang { chu: p.into(), ..Default::default() })
    }

    #[test]
    fn them_giu_5_moi_nhat_gop_trung_dua_len_dau() {
        let mut ds = vec![];
        for i in 0..6 {
            them(&mut ds, muc(ChucNang::Tach, &format!("/a/{i}")));
        }
        assert_eq!(ds.iter().map(|m| m.duong_dan.as_str()).collect::<Vec<_>>(), ["/a/5", "/a/4", "/a/3", "/a/2", "/a/1"]);
        them(&mut ds, muc(ChucNang::Tach, "/a/3/"));
        assert_eq!((ds.len(), ds[0].duong_dan.as_str(), ds[1].duong_dan.as_str()), (5, "/a/3/", "/a/5"), "cùng thư mục → 1 dòng, lên đầu");
        them(&mut ds, muc(ChucNang::BocTach, "/a/5"));
        assert_eq!((ds[0].cn, ds[1].duong_dan.as_str(), ds.len()), (ChucNang::BocTach, "/a/3/", 5), "khác chức năng → dòng riêng");
    }

    #[test]
    fn doc_ghi_file_hong_thi_rong() {
        let d = std::env::temp_dir().join(format!("dema4_gan_day_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join(TEN_FILE);
        assert!(doc(&p).is_empty(), "chưa có file");
        let mut m = muc(ChucNang::BocTach, "/x/HS");
        m.loai = Some("Văn bản".into());
        let ds = vec![m, muc(ChucNang::Dem, "/y")];
        ghi(&p, &ds).unwrap();
        assert_eq!(doc(&p), ds);
        std::fs::write(&p, "{hỏng").unwrap();
        assert!(doc(&p).is_empty(), "file hỏng → rỗng");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn danh_sach_xoa_doi_duong_dan_kiem_mat() {
        let d = std::env::temp_dir().join(format!("dema4_gan_day_ds_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join(TEN_FILE);
        let mut ds = DanhSach { file: Some(f.clone()), phien: 1, ..Default::default() };
        ds.them(muc(ChucNang::Tach, &d.display().to_string()));
        ds.them(muc(ChucNang::Dem, "/khong/co/thu_muc"));
        assert_eq!(doc(&f).len(), 2, "ghi file sau mỗi thay đổi");
        let ctx = eframe::egui::Context::default();
        let t = std::time::Instant::now();
        while ds.phien_mat != ds.phien {
            assert!(t.elapsed() < std::time::Duration::from_secs(5));
            ds.cap_nhat_mat(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(ds.mat, [true, false], "dòng đầu (Đếm) không còn thư mục");
        ds.doi_duong_dan(0, &d);
        assert_eq!((ds.ds[0].cn, ds.ds[0].duong_dan.as_str()), (ChucNang::Dem, d.to_str().unwrap()));
        assert!(!ds.mat(0) || ds.phien_mat != ds.phien, "đổi → kiểm lại");
        ds.xoa(1);
        assert_eq!(doc(&f).len(), 1);
        ds.xoa_het();
        assert!(doc(&f).is_empty() && ds.ds.is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ten_cha_thoi_gian_so() {
        let m = muc(ChucNang::Tach, "/a/b/HS_2024/");
        assert_eq!((m.ten().as_str(), m.cha().as_str()), ("HS_2024", "/a/b"));
        let w = muc(ChucNang::Tach, r"D:\SoHoa\HS_2024\");
        assert_eq!((w.ten().as_str(), w.cha().as_str()), ("HS_2024", r"D:\SoHoa"), "đường dẫn Windows tách đúng trên mọi máy");
        let t = 1_000_000_000_000;
        assert_eq!(thoi_gian_tuong_doi(t, t + 30_000), "vừa xong");
        assert_eq!(thoi_gian_tuong_doi(t, t + 5 * 60_000), "5 phút trước");
        assert_eq!(thoi_gian_tuong_doi(t, t + 2 * 3_600_000), "2 giờ trước");
        assert_eq!(thoi_gian_tuong_doi(t, t + 30 * 3_600_000), "hôm qua");
        assert_eq!(thoi_gian_tuong_doi(t, t + 3 * 86_400_000), "3 ngày trước");
        assert!(thoi_gian_tuong_doi(t, t + 30 * 86_400_000).contains('/'));
        assert_eq!((so(7).as_str(), so(1204).as_str(), so(1234567).as_str()), ("7", "1.204", "1.234.567"));
    }
}
