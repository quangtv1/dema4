//! License giới hạn số lần Đếm / Tách — phần lõi không gọi mạng: mã máy, quyết định cho chạy / chặn,
//! sổ lượt trên đĩa (đặt chỗ trước khi chạy, lần chạy chờ trạm xác nhận, câu trả lời có chữ ký gần nhất).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChucNang {
    Dem,
    Tach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThai {
    DungThu,
    DaCap,
    Khoa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HanMuc {
    pub dem: u32,
    pub tach: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrangThaiMay {
    pub han: HanMuc,
    pub da_dung: HanMuc,
    pub trang_thai: TrangThai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuyetDinh {
    Cho,
    HetLuot(ChucNang),
    HetLuotOffline(ChucNang),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrangThaiLan {
    DatCho,
    DaChay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanChay {
    pub ma: String,
    pub cn: ChucNang,
    pub luc: String,
    pub trang_thai: TrangThaiLan,
}

/// Câu trả lời của trạm đúng như nhận (chuỗi JSON + chữ ký base64) — kiểm chữ ký ở tầng kho.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraLoiKy {
    pub du_lieu: String,
    pub chu_ky: String,
}

/// Mã trạng thái ASCII từ trạm; giá trị lạ → Khoá (an toàn: không vô tình mở khoá).
pub fn trang_thai_tu_ma(ma: &str) -> TrangThai {
    match ma {
        "dung_thu" => TrangThai::DungThu,
        "da_cap" => TrangThai::DaCap,
        _ => TrangThai::Khoa,
    }
}

impl ChucNang {
    pub fn ma(self) -> &'static str {
        match self {
            ChucNang::Dem => "dem",
            ChucNang::Tach => "tach",
        }
    }
    fn tu_ma(s: &str) -> Option<Self> {
        match s {
            "dem" => Some(ChucNang::Dem),
            "tach" => Some(ChucNang::Tach),
            _ => None,
        }
    }
    fn cua(self, h: &HanMuc) -> u32 {
        match self {
            ChucNang::Dem => h.dem,
            ChucNang::Tach => h.tach,
        }
    }
}

/// Số lần tối đa chạy khi chưa được trạm xác nhận (mất mạng / trạm lỗi), mỗi chức năng.
pub const TOI_DA_OFFLINE: u32 = 2;

/// `SM-XXXX-XXXX-XXXX` = 12 ký tự hex đầu của SHA-256(ID hệ điều hành).
fn ma_may_tu(ma_hdh: &str) -> String {
    use rsa::sha2::{Digest, Sha256};
    let h: String = Sha256::digest(ma_hdh.trim().as_bytes()).iter().take(6).map(|b| format!("{b:02X}")).collect();
    format!("SM-{}-{}-{}", &h[0..4], &h[4..8], &h[8..12])
}

/// Mã máy của máy đang chạy: chỉ từ ID hệ điều hành (không đổi khi đổi tên máy / card mạng / VPN);
/// không đọc được → ID ngẫu nhiên tạo 1 lần, lưu cạnh dữ liệu ẩn của app.
pub fn ma_may() -> String {
    static MA: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    MA.get_or_init(|| {
        let id = id_he_dieu_hanh().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| id_du_phong(crate::tach_vb::du_lieu_an::goc_du_lieu()));
        ma_may_tu(&id)
    })
    .clone()
}

#[cfg(windows)]
fn id_he_dieu_hanh() -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY};
    let rong = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (khoa, ten) = (rong(r"SOFTWARE\Microsoft\Cryptography"), rong("MachineGuid"));
    let mut buf = [0u16; 128];
    let mut co = (buf.len() * 2) as u32;
    // SAFETY: chuỗi UTF-16 kết thúc bằng 0; buf đủ chỗ và `co` báo đúng kích thước byte.
    let kq = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, khoa.as_ptr(), ten.as_ptr(), RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut co) };
    (kq == 0).then(|| String::from_utf16_lossy(&buf[..(co as usize / 2).saturating_sub(1)]))
}

#[cfg(target_os = "macos")]
fn id_he_dieu_hanh() -> Option<String> {
    let ra = std::process::Command::new("/usr/sbin/ioreg").args(["-rd1", "-c", "IOPlatformExpertDevice"]).output().ok()?;
    let chu = String::from_utf8_lossy(&ra.stdout);
    let dong = chu.lines().find(|d| d.contains("\"IOPlatformUUID\""))?;
    Some(dong.rsplit('"').nth(1)?.to_string())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn id_he_dieu_hanh() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id").ok()
}

/// 16 byte ngẫu nhiên dạng hex (mã lần chạy, nonce, ID dự phòng).
pub fn ngau_nhien_hex() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("nguồn ngẫu nhiên của hệ điều hành");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Cho / chặn **không gọi mạng**. `da_ky`: câu trả lời trạm gần nhất (đã kiểm chữ ký); `cho_ghi`: lần chạy
/// trạm chưa xác nhận (kể cả đặt chỗ đang chạy). Quá `TOI_DA_OFFLINE` lần chưa xác nhận → chặn (mất mạng).
pub fn quyet_dinh(da_ky: Option<&TrangThaiMay>, cho_ghi: &[LanChay], cn: ChucNang) -> QuyetDinh {
    let chua_xac_nhan = cho_ghi.iter().filter(|l| l.cn == cn).count() as u32;
    if let Some(m) = da_ky {
        if m.trang_thai == TrangThai::Khoa || cn.cua(&m.da_dung) + chua_xac_nhan >= cn.cua(&m.han) {
            return QuyetDinh::HetLuot(cn);
        }
    }
    if chua_xac_nhan >= TOI_DA_OFFLINE {
        return QuyetDinh::HetLuotOffline(cn);
    }
    QuyetDinh::Cho
}

const TEN_FILE: &str = "license_so_luot.json";
const TEN_ID: &str = "license_id_may.txt";

/// Sổ lượt trên đĩa, lưu ở nhiều nơi (đọc hợp tất cả, ghi tất cả). Ghi nguyên tử (file tạm + đổi tên),
/// khoá bằng file `.lock` để nhiều cửa sổ / tiến trình không ghi đè nhau.
pub struct SoLuot {
    noi: Vec<PathBuf>,
}

#[derive(Default)]
struct NoiDung {
    lan: Vec<LanChay>,
    tra_loi: Option<TraLoiKy>,
}

impl NoiDung {
    fn tu_json(v: &serde_json::Value) -> Self {
        let lan = v["lan"].as_array().map(|ds| {
            ds.iter()
                .filter_map(|l| {
                    Some(LanChay {
                        ma: l["ma"].as_str()?.to_string(),
                        cn: ChucNang::tu_ma(l["cn"].as_str()?)?,
                        luc: l["luc"].as_str().unwrap_or_default().to_string(),
                        trang_thai: if l["trang_thai"].as_str() == Some("da_chay") { TrangThaiLan::DaChay } else { TrangThaiLan::DatCho },
                    })
                })
                .collect()
        });
        let tra_loi = v["tra_loi"]["du_lieu"].as_str().zip(v["tra_loi"]["chu_ky"].as_str()).map(|(d, c)| TraLoiKy { du_lieu: d.into(), chu_ky: c.into() });
        Self { lan: lan.unwrap_or_default(), tra_loi }
    }
    fn json(&self) -> serde_json::Value {
        let lan: Vec<serde_json::Value> = self
            .lan
            .iter()
            .map(|l| serde_json::json!({ "ma": l.ma, "cn": l.cn.ma(), "luc": l.luc, "trang_thai": if l.trang_thai == TrangThaiLan::DaChay { "da_chay" } else { "dat_cho" } }))
            .collect();
        serde_json::json!({ "lan": lan, "tra_loi": self.tra_loi.as_ref().map(|t| serde_json::json!({ "du_lieu": t.du_lieu, "chu_ky": t.chu_ky })) })
    }
}

impl SoLuot {
    pub fn moi(noi: Vec<PathBuf>) -> Self {
        Self { noi }
    }

    /// 2 nơi có thư mục cha khác nhau: dữ liệu ẩn cạnh file chạy + thư mục người dùng.
    pub fn mac_dinh() -> Self {
        let mut noi = vec![crate::tach_vb::du_lieu_an::goc_du_lieu().to_path_buf()];
        if let Some(d) = crate::cau_hinh::thu_muc_nguoi_dung() {
            if !noi.contains(&d) {
                noi.push(d);
            }
        }
        Self { noi }
    }

    /// Đọc hợp các nơi: lần chạy gộp theo mã (DaChay thắng DatCho); câu trả lời lấy nơi đầu tiên có.
    fn doc(&self) -> NoiDung {
        let mut kq = NoiDung::default();
        for d in &self.noi {
            let Some(v) = std::fs::read_to_string(d.join(TEN_FILE)).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) else { continue };
            let n = NoiDung::tu_json(&v);
            for l in n.lan {
                match kq.lan.iter_mut().find(|x| x.ma == l.ma) {
                    Some(x) if l.trang_thai == TrangThaiLan::DaChay => x.trang_thai = TrangThaiLan::DaChay,
                    Some(_) => {}
                    None => kq.lan.push(l),
                }
            }
            if kq.tra_loi.is_none() {
                kq.tra_loi = n.tra_loi;
            }
        }
        kq
    }

    fn ghi(&self, nd: &NoiDung) {
        let chu = nd.json().to_string();
        for d in &self.noi {
            let _ = std::fs::create_dir_all(d);
            let tam = d.join(format!("{TEN_FILE}.tmp"));
            if std::fs::write(&tam, &chu).is_ok() {
                let _ = std::fs::rename(&tam, d.join(TEN_FILE));
            }
        }
    }

    /// Đọc – sửa – ghi dưới khoá file (nơi đầu tiên); khoá cũ quá 10 s coi như bỏ dở.
    fn sua<R>(&self, f: impl FnOnce(&mut NoiDung) -> R) -> R {
        let khoa = self.noi.first().map(|d| d.join(format!("{TEN_FILE}.lock")));
        let mut giu = false;
        if let Some(k) = &khoa {
            let _ = std::fs::create_dir_all(k.parent().unwrap_or(Path::new(".")));
            for _ in 0..50 {
                if std::fs::OpenOptions::new().write(true).create_new(true).open(k).is_ok() {
                    giu = true;
                    break;
                }
                let cu = std::fs::metadata(k).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|e| e.as_secs() > 10);
                if cu {
                    let _ = std::fs::remove_file(k);
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        let mut nd = self.doc();
        let kq = f(&mut nd);
        self.ghi(&nd);
        if let (true, Some(k)) = (giu, &khoa) {
            let _ = std::fs::remove_file(k);
        }
        kq
    }

    /// Ghi đặt chỗ **trước khi** chạy (app chết giữa chừng → lần sau vẫn tính).
    pub fn dat_cho(&self, cn: ChucNang) -> LanChay {
        let l = LanChay { ma: ngau_nhien_hex(), cn, luc: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), trang_thai: TrangThaiLan::DatCho };
        self.sua(|nd| nd.lan.push(l.clone()));
        l
    }

    /// Chạy xong: có kết quả → chờ ghi lên trạm; không có kết quả → hoàn lượt.
    pub fn xong(&self, ma: &str, co_ket_qua: bool) {
        self.sua(|nd| {
            if co_ket_qua {
                if let Some(l) = nd.lan.iter_mut().find(|l| l.ma == ma) {
                    l.trang_thai = TrangThaiLan::DaChay;
                }
            } else {
                nd.lan.retain(|l| l.ma != ma);
            }
        });
    }

    /// Trạm đã ghi các mã này → xoá đúng chúng (lần chạy khác thêm sau vẫn giữ).
    pub fn da_xac_nhan(&self, ma: &[String]) {
        self.sua(|nd| nd.lan.retain(|l| !ma.contains(&l.ma)));
    }

    /// Lần chạy trạm chưa xác nhận (đặt chỗ đang chạy hoặc sót lại cũng tính).
    pub fn cho_ghi(&self) -> Vec<LanChay> {
        self.doc().lan
    }

    pub fn luu_tra_loi(&self, tl: &TraLoiKy) {
        self.sua(|nd| nd.tra_loi = Some(tl.clone()));
    }

    pub fn tra_loi(&self) -> Option<TraLoiKy> {
        self.doc().tra_loi
    }
}

/// ID ngẫu nhiên dự phòng khi không đọc được ID hệ điều hành: tạo 1 lần, lưu trong `thu_muc`.
fn id_du_phong(thu_muc: &Path) -> String {
    let p = thu_muc.join(TEN_ID);
    if let Some(s) = std::fs::read_to_string(&p).ok().map(|s| s.trim().to_string()).filter(|s| s.len() == 32) {
        return s;
    }
    let id = ngau_nhien_hex();
    let _ = std::fs::create_dir_all(thu_muc);
    let _ = std::fs::write(&p, &id);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thu_muc_tam(ten: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_license_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn so_luot(ten: &str) -> (SoLuot, PathBuf) {
        let d = thu_muc_tam(ten);
        (SoLuot::moi(vec![d.join("a"), d.join("b")]), d)
    }

    fn may(han: (u32, u32), dung: (u32, u32), tt: TrangThai) -> TrangThaiMay {
        TrangThaiMay { han: HanMuc { dem: han.0, tach: han.1 }, da_dung: HanMuc { dem: dung.0, tach: dung.1 }, trang_thai: tt }
    }

    fn lan(cn: ChucNang) -> LanChay {
        LanChay { ma: "x".into(), cn, luc: String::new(), trang_thai: TrangThaiLan::DaChay }
    }

    #[test]
    fn ma_may_chi_tu_id_he_dieu_hanh() {
        let a = ma_may_tu("4C4C4544-0035-3010-8052-B4C04F4E4B32");
        assert_eq!(a, ma_may_tu("4C4C4544-0035-3010-8052-B4C04F4E4B32"), "cùng ID → cùng mã");
        assert_ne!(a, ma_may_tu("khac"));
        let dung_mau = a.len() == 17 && a.starts_with("SM-") && a[3..].split('-').all(|n| n.len() == 4 && n.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
        assert!(dung_mau, "{a}");
        let d = thu_muc_tam("du_phong");
        let x = id_du_phong(&d);
        assert_eq!(x.len(), 32);
        assert_eq!(id_du_phong(&d), x, "lần 2 đọc lại đúng ID đã lưu");
    }

    #[test]
    fn quyet_dinh_theo_tra_loi_ky() {
        use ChucNang::*;
        assert_eq!(quyet_dinh(Some(&may((2, 2), (1, 0), TrangThai::DungThu)), &[], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(Some(&may((2, 2), (2, 0), TrangThai::DungThu)), &[], Dem), QuyetDinh::HetLuot(Dem));
        assert_eq!(quyet_dinh(Some(&may((2, 2), (2, 0), TrangThai::DungThu)), &[], Tach), QuyetDinh::Cho, "Đếm, Tách tính riêng");
        assert_eq!(quyet_dinh(Some(&may((100, 100), (0, 0), TrangThai::Khoa)), &[], Dem), QuyetDinh::HetLuot(Dem), "Khoá còn lượt vẫn chặn");
        assert_eq!(quyet_dinh(Some(&may((2, 2), (1, 0), TrangThai::DungThu)), &[lan(Dem)], Dem), QuyetDinh::HetLuot(Dem), "lần chờ ghi cộng vào đã dùng");
        assert_eq!(quyet_dinh(Some(&may((100, 2), (5, 0), TrangThai::DaCap)), &[lan(Dem)], Dem), QuyetDinh::Cho);
    }

    #[test]
    fn quyet_dinh_offline() {
        use ChucNang::*;
        assert_eq!(quyet_dinh(None, &[], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(None, &[lan(Dem)], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(None, &[lan(Dem), lan(Dem)], Dem), QuyetDinh::HetLuotOffline(Dem));
        assert_eq!(quyet_dinh(None, &[lan(Dem), lan(Dem)], Tach), QuyetDinh::Cho);
        // Có trả lời ký nhưng 2 lần chưa được trạm xác nhận (mất mạng) → chặn dù hạn mức lớn
        assert_eq!(quyet_dinh(Some(&may((100, 100), (0, 0), TrangThai::DaCap)), &[lan(Dem), lan(Dem)], Dem), QuyetDinh::HetLuotOffline(Dem));
    }

    #[test]
    fn dat_cho_va_hoan() {
        let (s, _) = so_luot("dat_cho");
        let a = s.dat_cho(ChucNang::Tach);
        assert_eq!(a.ma.len(), 32);
        assert_eq!(s.cho_ghi().len(), 1, "đặt chỗ đã tính ngay");
        s.xong(&a.ma, false);
        assert!(s.cho_ghi().is_empty(), "không có kết quả → hoàn lượt");
        let b = s.dat_cho(ChucNang::Dem);
        s.xong(&b.ma, true);
        assert_eq!(s.cho_ghi().iter().map(|l| (&l.ma, l.trang_thai.clone())).collect::<Vec<_>>(), [(&b.ma, TrangThaiLan::DaChay)]);
        let c = s.dat_cho(ChucNang::Dem); // giả app chết, không gọi xong()
        let doc_lai = SoLuot::moi(s.noi.clone());
        assert!(doc_lai.cho_ghi().iter().any(|l| l.ma == c.ma), "đặt chỗ sót lại vẫn tính");
    }

    #[test]
    fn so_luot_hai_noi_va_xac_nhan_dung_ma() {
        let (s, d) = so_luot("hai_noi");
        let a = s.dat_cho(ChucNang::Dem);
        s.xong(&a.ma, true);
        let b = s.dat_cho(ChucNang::Tach);
        s.xong(&b.ma, true);
        std::fs::remove_file(d.join("a").join("license_so_luot.json")).unwrap();
        assert_eq!(SoLuot::moi(s.noi.clone()).cho_ghi().len(), 2, "xoá 1 nơi vẫn đủ");
        std::fs::write(d.join("b").join("license_so_luot.json"), b"hong").unwrap();
        let s2 = SoLuot::moi(s.noi.clone());
        assert!(s2.cho_ghi().is_empty() || s2.cho_ghi().len() == 2, "file hỏng bị bỏ qua, không panic");
        let (s, _) = so_luot("xac_nhan");
        let a = s.dat_cho(ChucNang::Dem);
        s.xong(&a.ma, true);
        let b = s.dat_cho(ChucNang::Dem);
        s.xong(&b.ma, true);
        s.da_xac_nhan(std::slice::from_ref(&a.ma));
        assert_eq!(s.cho_ghi().iter().map(|l| l.ma.clone()).collect::<Vec<_>>(), [b.ma.clone()], "chỉ xoá đúng mã đã xác nhận");
        let tl = TraLoiKy { du_lieu: "{\"x\":1}".into(), chu_ky: "abc".into() };
        s.luu_tra_loi(&tl);
        assert_eq!(SoLuot::moi(s.noi.clone()).tra_loi(), Some(tl));
    }

    /// Mã máy thật của máy đang chạy: đọc được ID hệ điều hành, 2 lần gọi ra cùng mã.
    #[test]
    fn ma_may_that_on_dinh() {
        assert!(id_he_dieu_hanh().is_some_and(|s| !s.trim().is_empty()), "đọc được ID hệ điều hành");
        assert_eq!(ma_may(), ma_may());
        println!("Mã máy: {}", ma_may());
    }

    #[test]
    fn trang_thai_la_coi_la_khoa() {
        assert_eq!(trang_thai_tu_ma("dung_thu"), TrangThai::DungThu);
        assert_eq!(trang_thai_tu_ma("da_cap"), TrangThai::DaCap);
        assert_eq!(trang_thai_tu_ma("khoa"), TrangThai::Khoa);
        assert_eq!(trang_thai_tu_ma("abc"), TrangThai::Khoa);
        assert_eq!(trang_thai_tu_ma(""), TrangThai::Khoa);
    }
}
