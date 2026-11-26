//! File mô hình nhận diện layout (YOLOv12s-DocLayNet, AGPL-3.0) — không nhúng trong exe:
//! tải 1 lần từ kho XoaGhim công khai (hoặc chọn file có sẵn), kiểm kích thước + SHA-256, lưu vào
//! `<thư mục dữ liệu>/mo_hinh/`; sau đó dùng ngoại tuyến.
//!
//! Tải ghi vào `<file>.part` rồi mới đổi tên → không bao giờ để lại file dở; huỷ / lỗi thì xoá `.part`.
//! Giao diện dùng `TaiMoHinh` (luồng nền) và đọc `trang_thai()` mỗi khung hình.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};

pub const TEN_FILE: &str = "yolov12s-doclaynet.onnx";
pub const CO_FILE: u64 = 37_544_909;
pub const SHA256: &str = "b5c0defaf2748d3a02438311be854a12a4b47698633ba7f91a08567197fe927a";
/// Bản công khai trong kho XoaGhim (cùng file, cùng SHA-256) — tải thẳng, không cần bản phát hành riêng.
pub const DIA_CHI: &str = "https://raw.githubusercontent.com/quangtv1/xoaghim/main/resources/models/yolov12s-doclaynet.onnx";

const HUY: &str = "Đã huỷ";

/// `<thư mục dữ liệu>/mo_hinh/yolov12s-doclaynet.onnx`.
pub fn duong_dan() -> PathBuf {
    crate::tach_vb::du_lieu_an::goc_du_lieu().join("mo_hinh").join(TEN_FILE)
}

/// Đã có file mô hình đúng kích thước (SHA chỉ kiểm lúc tải / chọn file cho nhanh).
pub fn co_san() -> bool {
    std::fs::metadata(duong_dan()).is_ok_and(|m| m.is_file() && m.len() == CO_FILE)
}

/// Tải `url` về `dich` (đúng mô hình của app). `tien_do(đã tải, tổng)` gọi sau mỗi đoạn; `dung` = huỷ.
pub fn tai(url: &str, dich: &Path, tien_do: impl Fn(u64, u64), dung: &AtomicBool) -> Result<(), String> {
    tai_kiem(url, dich, CO_FILE, SHA256, &tien_do, dung)
}

/// Chép file mô hình người dùng chọn vào chỗ (kiểm kích thước + SHA-256 trước).
pub fn nhap_file(p: &Path) -> Result<(), String> {
    nhap_kiem(p, &duong_dan(), CO_FILE, SHA256, &AtomicBool::new(false))
}

fn duoi_part(dich: &Path) -> PathBuf {
    let mut s = dich.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Ghi vào `.part` bằng `ghi`, rồi đổi tên thành `dich`; lỗi / huỷ → xoá `.part`.
fn ghi_an_toan(dich: &Path, ghi: impl FnOnce(&mut File) -> Result<(), String>) -> Result<(), String> {
    if let Some(cha) = dich.parent() {
        std::fs::create_dir_all(cha).map_err(|e| format!("Không tạo được thư mục mô hình: {e}"))?;
    }
    let part = duoi_part(dich);
    let kq = File::create(&part)
        .map_err(|e| format!("Không ghi được tệp: {e}"))
        .and_then(|mut f| ghi(&mut f).and_then(|_| f.sync_all().map_err(|e| format!("Không ghi được tệp: {e}"))))
        .and_then(|_| std::fs::rename(&part, dich).map_err(|e| format!("Không đổi tên được tệp: {e}")));
    if kq.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    kq
}

/// Đọc hết `vao` sang `ra`, tính SHA-256; kiểm đúng `co` byte và mã `sha`.
fn chep_kiem(vao: &mut dyn Read, ra: &mut dyn Write, co: u64, sha: &str, tien_do: &dyn Fn(u64, u64), dung: &AtomicBool, loi_doc: &str) -> Result<(), String> {
    let mut bam = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut da = 0u64;
    loop {
        if dung.load(Ordering::Relaxed) {
            return Err(HUY.into());
        }
        let n = vao.read(&mut buf).map_err(|_| loi_doc.to_string())?;
        if n == 0 {
            break;
        }
        da += n as u64;
        if da > co {
            return Err("Tệp sai kích thước".into());
        }
        bam.update(&buf[..n]);
        ra.write_all(&buf[..n]).map_err(|e| format!("Không ghi được tệp: {e}"))?;
        tien_do(da, co);
    }
    if da != co {
        return Err(format!("Tệp sai kích thước ({da} / {co} byte)"));
    }
    if hex(&bam.finalize()) != sha {
        return Err("Tệp sai mã kiểm SHA-256".into());
    }
    Ok(())
}

fn tai_kiem(url: &str, dich: &Path, co: u64, sha: &str, tien_do: &dyn Fn(u64, u64), dung: &AtomicBool) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let tl = agent.get(url).call().map_err(|e| match e {
        ureq::Error::Timeout(_) => "Mạng chậm, hết thời gian chờ".to_string(),
        _ => "Không kết nối được máy chủ tải mô hình (kiểm tra mạng)".to_string(),
    })?;
    match tl.status().as_u16() {
        200 => {}
        404 => return Err("Chưa có tệp mô hình trên máy chủ (mã 404)".into()),
        m => return Err(format!("Máy chủ trả mã {m}")),
    }
    let mut than = tl.into_body().into_reader();
    ghi_an_toan(dich, |f| chep_kiem(&mut than, f, co, sha, tien_do, dung, "Mất kết nối khi đang tải"))
}

fn nhap_kiem(nguon: &Path, dich: &Path, co: u64, sha: &str, dung: &AtomicBool) -> Result<(), String> {
    let mut f = File::open(nguon).map_err(|e| format!("Không mở được tệp: {e}"))?;
    if f.metadata().map(|m| m.len()).unwrap_or(0) != co {
        return Err("Không phải tệp mô hình của app (sai kích thước)".into());
    }
    if std::fs::canonicalize(nguon).ok() == std::fs::canonicalize(dich).ok() {
        // Chọn đúng file đang dùng → chỉ kiểm
        return chep_kiem(&mut f, &mut std::io::sink(), co, sha, &|_, _| {}, dung, "Không đọc được tệp");
    }
    ghi_an_toan(dich, |ra| chep_kiem(&mut f, ra, co, sha, &|_, _| {}, dung, "Không đọc được tệp"))
}

/// Trạng thái file mô hình cho nút cạnh "Nhận diện layout".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrangThaiMoHinh {
    ChuaCo,
    DangTai { da: u64, tong: u64 },
    /// Đang kiểm SHA-256 file người dùng chọn.
    KiemTra,
    SanSang,
    Loi(String),
}

impl TrangThaiMoHinh {
    /// Trạng thái lúc mở app / chưa có việc nền: có file đúng cỡ = sẵn sàng.
    pub fn hien_tai() -> Self {
        if co_san() {
            TrangThaiMoHinh::SanSang
        } else {
            TrangThaiMoHinh::ChuaCo
        }
    }
}

/// Việc tải / nhập mô hình chạy nền. Huỷ → về `ChuaCo`; lỗi → `Loi`; xong → `SanSang`.
/// `bao_doi` gọi mỗi khi trạng thái đổi (vd `ctx.request_repaint()`).
pub struct TaiMoHinh {
    tt: Arc<Mutex<TrangThaiMoHinh>>,
    dung: Arc<AtomicBool>,
}

impl TaiMoHinh {
    /// Tải mô hình từ kho XoaGhim công khai về `duong_dan()`.
    pub fn tai(bao_doi: impl Fn() + Send + 'static) -> Self {
        Self::tai_tu(DIA_CHI.to_string(), duong_dan(), CO_FILE, SHA256.to_string(), bao_doi)
    }

    /// Kiểm rồi chép file người dùng chọn vào `duong_dan()`.
    pub fn nhap(nguon: PathBuf, bao_doi: impl Fn() + Send + 'static) -> Self {
        Self::chay_nen(TrangThaiMoHinh::KiemTra, bao_doi, move |_, dung| nhap_kiem(&nguon, &duong_dan(), CO_FILE, SHA256, dung))
    }

    fn tai_tu(url: String, dich: PathBuf, co: u64, sha: String, bao_doi: impl Fn() + Send + 'static) -> Self {
        Self::chay_nen(TrangThaiMoHinh::DangTai { da: 0, tong: co }, bao_doi, move |tien_do, dung| tai_kiem(&url, &dich, co, &sha, tien_do, dung))
    }

    fn chay_nen(
        dau: TrangThaiMoHinh,
        bao_doi: impl Fn() + Send + 'static,
        viec: impl FnOnce(&dyn Fn(u64, u64), &AtomicBool) -> Result<(), String> + Send + 'static,
    ) -> Self {
        let tt = Arc::new(Mutex::new(dau));
        let dung = Arc::new(AtomicBool::new(false));
        let (tt2, dung2) = (tt.clone(), dung.clone());
        std::thread::spawn(move || {
            let dat = |v: TrangThaiMoHinh| {
                *tt2.lock().unwrap_or_else(|e| e.into_inner()) = v;
                bao_doi();
            };
            let kq = viec(&|da, tong| dat(TrangThaiMoHinh::DangTai { da, tong }), &dung2);
            dat(match kq {
                Ok(()) => TrangThaiMoHinh::SanSang,
                Err(_) if dung2.load(Ordering::Relaxed) => TrangThaiMoHinh::ChuaCo,
                Err(e) => TrangThaiMoHinh::Loi(e),
            });
        });
        TaiMoHinh { tt, dung }
    }

    pub fn trang_thai(&self) -> TrangThaiMoHinh {
        self.tt.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Huỷ; luồng nền dừng ở đoạn kế tiếp và xoá `.part`.
    pub fn huy(&self) {
        self.dung.store(true, Ordering::Relaxed);
    }

    /// Việc nền đã xong (sẵn sàng / lỗi / đã huỷ).
    pub fn xong(&self) -> bool {
        !matches!(self.trang_thai(), TrangThaiMoHinh::DangTai { .. } | TrangThaiMoHinh::KiemTra)
    }
}

#[cfg(test)]
mod tests {
    /// Tải thật từ `DIA_CHI` (cần mạng): đúng cỡ + SHA-256, ghi xong mới đổi tên. Chạy:
    /// `cargo test --release -- --ignored tai_that_tu_dia_chi`
    #[test]
    #[ignore = "cần mạng"]
    fn tai_that_tu_dia_chi() {
        let d = std::env::temp_dir().join(format!("dema4_tai_mo_hinh_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let dich = d.join(TEN_FILE);
        tai(DIA_CHI, &dich, |_, _| {}, &AtomicBool::new(false)).expect("tải mô hình");
        assert_eq!(std::fs::metadata(&dich).unwrap().len(), CO_FILE);
        let _ = std::fs::remove_dir_all(&d);
    }

    use super::*;
    use std::net::TcpListener;

    fn sha(b: &[u8]) -> String {
        hex(&Sha256::digest(b))
    }

    /// Máy chủ HTTP 1 lần trả lời: mã `ma`, thân `than` (gửi từng đoạn 4 KB, `cham` = nghỉ giữa các đoạn).
    fn may_chu(ma: u16, than: Vec<u8>, cham: bool) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let cong = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut yc = Vec::new();
            let mut b = [0u8; 1024];
            while !yc.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = s.read(&mut b).unwrap();
                if n == 0 {
                    return;
                }
                yc.extend_from_slice(&b[..n]);
            }
            let dau = format!("HTTP/1.1 {ma} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", than.len());
            let _ = s.write_all(dau.as_bytes());
            for doan in than.chunks(4096) {
                if s.write_all(doan).is_err() {
                    return;
                }
                if cham {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        });
        format!("http://127.0.0.1:{cong}/m.onnx")
    }

    fn thu_muc(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_mo_hinh_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn du_lieu() -> Vec<u8> {
        (0..100_000u32).map(|i| (i * 7 % 251) as u8).collect()
    }

    fn sach(dich: &Path) -> bool {
        !duoi_part(dich).exists()
    }

    #[test]
    fn tai_dung_ma_kiem_thi_luu_va_bao_tien_do() {
        let d = du_lieu();
        let dich = thu_muc("dung").join("mo_hinh").join(TEN_FILE);
        let cuoi = Mutex::new((0, 0));
        tai_kiem(&may_chu(200, d.clone(), false), &dich, d.len() as u64, &sha(&d), &|a, b| *cuoi.lock().unwrap() = (a, b), &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read(&dich).unwrap(), d);
        assert_eq!(*cuoi.lock().unwrap(), (d.len() as u64, d.len() as u64));
        assert!(sach(&dich));
        let _ = std::fs::remove_dir_all(dich.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn tai_sai_sha_hoac_sai_co_thi_tu_choi_khong_de_lai_file() {
        let d = du_lieu();
        let dich = thu_muc("sai").join(TEN_FILE);
        let e = tai_kiem(&may_chu(200, d.clone(), false), &dich, d.len() as u64, &sha(b"khac"), &|_, _| {}, &AtomicBool::new(false)).unwrap_err();
        assert!(e.contains("SHA-256"), "{e}");
        assert!(!dich.exists() && sach(&dich));
        let e = tai_kiem(&may_chu(200, d.clone(), false), &dich, d.len() as u64 + 1, &sha(&d), &|_, _| {}, &AtomicBool::new(false)).unwrap_err();
        assert!(e.contains("kích thước"), "{e}");
        assert!(!dich.exists() && sach(&dich));
        let e = tai_kiem(&may_chu(404, b"khong co".to_vec(), false), &dich, 8, &sha(b"khong co"), &|_, _| {}, &AtomicBool::new(false)).unwrap_err();
        assert!(e.contains("404"), "{e}");
        let _ = std::fs::remove_dir_all(dich.parent().unwrap());
    }

    #[test]
    fn huy_giua_chung_xoa_part() {
        let d = du_lieu();
        let dich = thu_muc("huy").join(TEN_FILE);
        let dung = AtomicBool::new(false);
        let e = tai_kiem(&may_chu(200, d.clone(), true), &dich, d.len() as u64, &sha(&d), &|da, _| if da > 10_000 { dung.store(true, Ordering::Relaxed) }, &dung).unwrap_err();
        assert_eq!(e, HUY);
        assert!(!dich.exists() && sach(&dich));
        let _ = std::fs::remove_dir_all(dich.parent().unwrap());
    }

    #[test]
    fn khong_mang_bao_loi_gon() {
        let cong = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // đóng ngay → từ chối kết nối
        let dich = thu_muc("mat_mang").join(TEN_FILE);
        let e = tai_kiem(&format!("http://127.0.0.1:{cong}/m.onnx"), &dich, 10, SHA256, &|_, _| {}, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(e, "Không kết nối được máy chủ tải mô hình (kiểm tra mạng)");
        assert!(!dich.exists() && sach(&dich));
    }

    #[test]
    fn nhap_file_kiem_sha_roi_chep() {
        let d = du_lieu();
        let tm = thu_muc("nhap");
        std::fs::create_dir_all(&tm).unwrap();
        let nguon = tm.join("chon.onnx");
        std::fs::write(&nguon, &d).unwrap();
        let dich = tm.join("mo_hinh").join(TEN_FILE);
        let khong = AtomicBool::new(false);
        let e = nhap_kiem(&nguon, &dich, d.len() as u64, &sha(b"x"), &khong).unwrap_err();
        assert!(e.contains("SHA-256") && !dich.exists() && sach(&dich), "{e}");
        assert!(nhap_kiem(&nguon, &dich, 5, &sha(&d), &khong).unwrap_err().contains("kích thước"));
        nhap_kiem(&nguon, &dich, d.len() as u64, &sha(&d), &khong).unwrap();
        assert_eq!(std::fs::read(&dich).unwrap(), d);
        // Chọn lại đúng file đang dùng → chỉ kiểm, không hỏng file
        nhap_kiem(&dich, &dich, d.len() as u64, &sha(&d), &khong).unwrap();
        assert_eq!(std::fs::read(&dich).unwrap(), d);
        let _ = std::fs::remove_dir_all(&tm);
    }

    fn cho_xong(t: &TaiMoHinh) -> TrangThaiMoHinh {
        for _ in 0..500 {
            if t.xong() {
                return t.trang_thai();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("quá lâu: {:?}", t.trang_thai());
    }

    #[test]
    fn viec_nen_bao_trang_thai_va_huy() {
        let d = du_lieu();
        let tm = thu_muc("nen");
        let dich = tm.join(TEN_FILE);
        let lan = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let l2 = lan.clone();
        let t = TaiMoHinh::tai_tu(may_chu(200, d.clone(), false), dich.clone(), d.len() as u64, sha(&d), move || {
            l2.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(cho_xong(&t), TrangThaiMoHinh::SanSang);
        assert!(lan.load(Ordering::Relaxed) >= 2, "báo đổi khi tải + khi xong");

        let _ = std::fs::remove_file(&dich);
        let t = TaiMoHinh::tai_tu(may_chu(200, d.clone(), true), dich.clone(), d.len() as u64, sha(&d), || {});
        while !matches!(t.trang_thai(), TrangThaiMoHinh::DangTai { da, .. } if da > 0) {
            std::thread::sleep(Duration::from_millis(5));
        }
        t.huy();
        assert_eq!(cho_xong(&t), TrangThaiMoHinh::ChuaCo);
        assert!(!dich.exists() && sach(&dich));

        let t = TaiMoHinh::tai_tu(may_chu(200, d.clone(), false), dich.clone(), d.len() as u64, SHA256.into(), || {});
        assert_eq!(cho_xong(&t), TrangThaiMoHinh::Loi("Tệp sai mã kiểm SHA-256".into()));
        let _ = std::fs::remove_dir_all(&tm);
    }
}
