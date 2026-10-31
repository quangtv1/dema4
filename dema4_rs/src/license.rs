//! License giới hạn tổng số lần dùng (Đếm / Tách / Sinh mục lục chung một hạn mức) — phần lõi không gọi mạng: mã máy, quyết định cho chạy / chặn,
//! sổ lượt trên đĩa (đặt chỗ trước khi chạy, lần chạy chờ trạm xác nhận, câu trả lời có chữ ký gần nhất).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::tram::{KetQuaTram, KhoLicense, ThongTinMay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChucNang {
    Dem,
    Tach,
    MucLuc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThai {
    DungThu,
    DaCap,
    Khoa,
}

/// Trạng thái máy theo trạm: một hạn mức + một số đã dùng chung cho mọi chức năng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrangThaiMay {
    pub han: u32,
    pub da_dung: u32,
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
            ChucNang::MucLuc => "muc_luc",
        }
    }
    fn tu_ma(s: &str) -> Option<Self> {
        match s {
            "dem" => Some(ChucNang::Dem),
            "tach" => Some(ChucNang::Tach),
            "muc_luc" => Some(ChucNang::MucLuc),
            _ => None,
        }
    }
}

/// Số lần tối đa chạy khi chưa được trạm xác nhận (mất mạng / trạm lỗi), tính chung mọi chức năng.
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
/// trạm chưa xác nhận (kể cả đặt chỗ đang chạy), mọi chức năng. Quá `TOI_DA_OFFLINE` lần chưa xác nhận → chặn
/// (mất mạng). `cn` chỉ để báo chức năng bị chặn — hạn mức dùng chung.
pub fn quyet_dinh(da_ky: Option<&TrangThaiMay>, cho_ghi: &[LanChay], cn: ChucNang) -> QuyetDinh {
    let chua_xac_nhan = cho_ghi.len() as u32;
    if let Some(m) = da_ky {
        if m.trang_thai == TrangThai::Khoa || m.da_dung + chua_xac_nhan >= m.han {
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
    #[cfg(test)]
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

/// Việc của luồng gửi (1 luồng / app: không có 2 lời gọi trạm cùng lúc).
enum Viec {
    /// Ghi các lần chờ lên trạm (không có thì `kiem_tra`) rồi cập nhật câu trả lời ký.
    DongBo,
    ThongKe(&'static str, Box<dyn FnOnce(&ThongTinMay) -> serde_json::Value + Send>),
    /// Mốc: báo khi mọi việc trước đó đã xong.
    Moc(std::sync::mpsc::Sender<()>),
}

type LayIp = Box<dyn Fn() -> (String, String) + Send + Sync>;

struct Chung {
    kho: Option<Arc<dyn KhoLicense>>,
    so: SoLuot,
    ma: String,
    ten: String,
    mac: String,
    lay_ip: LayIp,
    da_ky: Mutex<Option<TrangThaiMay>>,
    /// Đặt chỗ của chính app này đang chạy (không gửi trạm khi chưa xong).
    dang_chay: Mutex<HashSet<String>>,
    /// Lần chạy xong trong phiên này (ghi trạm với nguồn "online"; còn lại là bù offline).
    xong_phien: Mutex<HashSet<String>>,
    /// Phiên này đã hỏi trạm ít nhất 1 lần (thành công hay lỗi).
    da_thu_tram: std::sync::atomic::AtomicBool,
}

impl Chung {
    /// Tên máy / MAC đọc 1 lần; IP đọc lại mỗi lời gọi (máy có thể đã chuyển mạng).
    fn may(&self) -> ThongTinMay {
        let (ip_noi_bo, ip_public) = (self.lay_ip)();
        ThongTinMay { ma: self.ma.clone(), ten: self.ten.clone(), mac: self.mac.clone(), ip_noi_bo, ip_public, phien_ban: env!("CARGO_PKG_VERSION").into() }
    }

    fn nhan(&self, kq: &KetQuaTram) {
        self.so.luu_tra_loi(&kq.ky);
        *self.da_ky.lock().unwrap() = Some(kq.may);
        self.so.da_xac_nhan(&kq.da_ghi);
    }

    /// Lỗi → giữ nguyên sổ, lần sau gửi lại (trạm chống trùng theo mã lần).
    fn dong_bo(&self) {
        self.dong_bo_trong();
        self.da_thu_tram.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn dong_bo_trong(&self) {
        let Some(kho) = &self.kho else { return };
        let may = self.may();
        // Đọc sổ dưới khoá `dang_chay`: đặt chỗ vừa ghi đĩa luôn đã nằm trong `dang_chay` (xem `bat_dau`)
        let cho: Vec<LanChay> = {
            let dang = self.dang_chay.lock().unwrap();
            self.so.cho_ghi().into_iter().filter(|l| l.trang_thai == TrangThaiLan::DaChay || !dang.contains(&l.ma)).collect()
        };
        if cho.is_empty() {
            if let Ok(kq) = kho.kiem_tra(&may) {
                self.nhan(&kq);
            }
            return;
        }
        for phan in cho.chunks(TOI_DA_LAN_MOI_LOI_GOI) {
            let online = self.xong_phien.lock().unwrap().clone();
            match kho.ghi_lan(&may, phan, &online) {
                Ok(kq) => self.nhan(&kq),
                Err(_) => return,
            }
        }
    }
}

/// Trạm nhận tối đa 3 lần mỗi lời gọi (1 vừa chạy + bù offline).
const TOI_DA_LAN_MOI_LOI_GOI: usize = 3;

/// License của app: quyết định tức thì (không chờ mạng) + 1 luồng gửi lên trạm.
pub struct KiemTra {
    chung: Arc<Chung>,
    viec: Mutex<std::sync::mpsc::Sender<Viec>>,
}

impl KiemTra {
    pub fn moi(kho: Option<Arc<dyn KhoLicense>>, so: SoLuot, ma: String, ten: String, mac: String, lay_ip: LayIp) -> Self {
        let da_ky = so.tra_loi().zip(kho.as_ref()).and_then(|(tl, k)| k.doc_da_luu(&tl, &ma));
        let chung = Arc::new(Chung { kho, so, ma, ten, mac, lay_ip, da_ky: Mutex::new(da_ky), dang_chay: Mutex::default(), xong_phien: Mutex::default(), da_thu_tram: Default::default() });
        let (gui, nhan) = std::sync::mpsc::channel::<Viec>();
        let c = Arc::clone(&chung);
        std::thread::spawn(move || {
            for v in nhan {
                match v {
                    Viec::DongBo => c.dong_bo(),
                    Viec::ThongKe(hd, soan) => {
                        if let Some(kho) = &c.kho {
                            let may = c.may();
                            let _ = kho.thong_ke(&may, hd, soan(&may));
                        }
                    }
                    Viec::Moc(xong) => {
                        let _ = xong.send(());
                    }
                }
            }
        });
        KiemTra { chung, viec: Mutex::new(gui) }
    }

    /// Trạm thật + sổ lượt thật, dùng chung cả app.
    fn that() -> Arc<KiemTra> {
        static KT: OnceLock<Arc<KiemTra>> = OnceLock::new();
        KT.get_or_init(|| {
            let kho = crate::tram::KhoTram::moi().map(|k| Arc::new(k) as Arc<dyn KhoLicense>);
            use crate::telegram::{dia_chi_mac, ip_noi_bo, ip_public, ten_pc};
            Arc::new(KiemTra::moi(kho, SoLuot::mac_dinh(), ma_may(), ten_pc(), dia_chi_mac(), Box::new(|| (ip_noi_bo(), ip_public()))))
        })
        .clone()
    }

    /// Kho giả hạn mức lớn + sổ lượt trong thư mục tạm riêng: test không gọi trạm thật, không đụng sổ thật.
    #[cfg(test)]
    pub fn gia(kho: Arc<crate::tram::KhoGia>) -> Arc<KiemTra> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!("dema4_kiem_tra_{}_{}", std::process::id(), DEM.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        Arc::new(KiemTra::moi(Some(kho), SoLuot::moi(vec![d]), "SM-0000-0000-0001".into(), "PC-TEST".into(), "AA:BB".into(), Box::new(|| ("?".into(), "?".into()))))
    }

    /// Mặc định của giao diện: app thật dùng trạm thật; test dùng kho giả hạn mức lớn.
    pub fn mac_dinh() -> Arc<KiemTra> {
        #[cfg(test)]
        return KiemTra::gia(Arc::new(crate::tram::KhoGia::moi(1_000_000)));
        #[cfg(not(test))]
        KiemTra::that()
    }

    pub fn ma_may(&self) -> &str {
        &self.chung.ma
    }

    fn gui(&self, v: Viec) {
        let _ = self.viec.lock().unwrap().send(v);
    }

    /// Quyết định tức thì trên câu trả lời ký gần nhất + sổ lượt (không gọi mạng).
    pub fn quyet_dinh(&self, cn: ChucNang) -> QuyetDinh {
        quyet_dinh(self.chung.da_ky.lock().unwrap().as_ref(), &self.chung.so.cho_ghi(), cn)
    }

    /// Được chạy → ghi đặt chỗ lên đĩa **trước khi** chạy; hết lượt → lý do để hiện popup.
    pub fn bat_dau(&self, cn: ChucNang) -> Result<LanChay, QuyetDinh> {
        // Chưa có trả lời ký hợp lệ (máy mới, hoặc file trên đĩa bị xoá / sửa) mà phiên này chưa hỏi trạm:
        // chờ trạm rồi mới quyết định, để xoá file không mở thêm lượt. Mất mạng thật → luật offline.
        // Thường không phải chờ: mở app đã hỏi trạm (`mo_app`).
        let c = &self.chung;
        if c.kho.is_some() && !c.da_thu_tram.load(std::sync::atomic::Ordering::SeqCst) && c.da_ky.lock().unwrap().is_none() {
            self.gui(Viec::DongBo);
            let _ = self.moc().recv_timeout(std::time::Duration::from_secs(20));
        }
        match self.quyet_dinh(cn) {
            QuyetDinh::Cho => {
                // Ghi đĩa + đánh dấu đang chạy cùng 1 khoá: luồng gửi không thấy đặt chỗ này như lần bỏ dở
                let mut dang = self.chung.dang_chay.lock().unwrap();
                let lan = self.chung.so.dat_cho(cn);
                dang.insert(lan.ma.clone());
                Ok(lan)
            }
            qd => Err(qd),
        }
    }

    /// Chạy xong: có kết quả → tính lượt + ghi lên trạm ở luồng nền; không có → hoàn lượt.
    pub fn ket_thuc(&self, lan: &LanChay, co_ket_qua: bool) {
        self.chung.so.xong(&lan.ma, co_ket_qua);
        self.chung.dang_chay.lock().unwrap().remove(&lan.ma);
        if co_ket_qua {
            self.chung.xong_phien.lock().unwrap().insert(lan.ma.clone());
            self.gui(Viec::DongBo);
        }
    }

    /// Mở app: hỏi trạm + ghi bù các lần chạy offline.
    pub fn mo_app(&self) {
        self.gui(Viec::DongBo);
    }

    /// Nút Kiểm tra lại: hỏi trạm; nhận tín hiệu khi xong (nơi gọi chờ tối đa 8 s rồi `quyet_dinh` lại).
    pub fn kiem_tra_lai(&self) -> std::sync::mpsc::Receiver<()> {
        self.gui(Viec::DongBo);
        self.moc()
    }

    /// Báo khi mọi việc đã xếp trước đó xong (dòng lệnh chờ ghi trạm trước khi thoát).
    pub fn moc(&self) -> std::sync::mpsc::Receiver<()> {
        let (gui, nhan) = std::sync::mpsc::channel();
        self.gui(Viec::Moc(gui));
        nhan
    }

    /// Gửi số liệu thống kê ở luồng gửi; false khi chưa có trạm (popup không hiện dòng "đã ghi lại").
    pub fn thong_ke(&self, hanh_dong: &'static str, soan: impl FnOnce(&ThongTinMay) -> serde_json::Value + Send + 'static) -> bool {
        if self.chung.kho.is_none() {
            return false;
        }
        self.gui(Viec::ThongKe(hanh_dong, Box::new(soan)));
        true
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

    fn may(han: u32, da_dung: u32, trang_thai: TrangThai) -> TrangThaiMay {
        TrangThaiMay { han, da_dung, trang_thai }
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
        assert_eq!(quyet_dinh(Some(&may(2, 1, TrangThai::DungThu)), &[], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(Some(&may(2, 2, TrangThai::DungThu)), &[], Dem), QuyetDinh::HetLuot(Dem));
        for cn in [Tach, MucLuc] {
            assert_eq!(quyet_dinh(Some(&may(2, 2, TrangThai::DungThu)), &[], cn), QuyetDinh::HetLuot(cn), "hạn mức chung: hết là hết mọi chức năng");
        }
        assert_eq!(quyet_dinh(None, &[lan(MucLuc), lan(Dem)], Tach), QuyetDinh::HetLuotOffline(Tach), "lần chưa xác nhận tính chung");
        assert_eq!(quyet_dinh(Some(&may(100, 0, TrangThai::Khoa)), &[], Dem), QuyetDinh::HetLuot(Dem), "Khoá còn lượt vẫn chặn");
        assert_eq!(quyet_dinh(Some(&may(2, 1, TrangThai::DungThu)), &[lan(Tach)], Dem), QuyetDinh::HetLuot(Dem), "lần chờ ghi của chức năng khác cũng cộng vào");
        assert_eq!(quyet_dinh(Some(&may(100, 5, TrangThai::DaCap)), &[lan(Dem)], Dem), QuyetDinh::Cho);
    }

    #[test]
    fn quyet_dinh_offline() {
        use ChucNang::*;
        assert_eq!(quyet_dinh(None, &[], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(None, &[lan(Dem)], Dem), QuyetDinh::Cho);
        assert_eq!(quyet_dinh(None, &[lan(Dem), lan(Dem)], Dem), QuyetDinh::HetLuotOffline(Dem));
        assert_eq!(quyet_dinh(None, &[lan(Dem), lan(Dem)], Tach), QuyetDinh::HetLuotOffline(Tach), "luật offline tính chung");
        // Có trả lời ký nhưng 2 lần chưa được trạm xác nhận (mất mạng) → chặn dù hạn mức lớn
        assert_eq!(quyet_dinh(Some(&may(100, 0, TrangThai::DaCap)), &[lan(Dem), lan(Dem)], Dem), QuyetDinh::HetLuotOffline(Dem));
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

    mod kiem_tra {
        use super::super::*;
        use crate::tram::KhoGia;
        use std::sync::atomic::Ordering::SeqCst;
        use std::time::{Duration, Instant};

        fn cho(kt: &KiemTra) {
            kt.moc().recv_timeout(Duration::from_secs(5)).unwrap();
        }

        fn chay(kt: &KiemTra, cn: ChucNang) -> bool {
            match kt.bat_dau(cn) {
                Ok(l) => {
                    kt.ket_thuc(&l, true);
                    cho(kt);
                    true
                }
                Err(_) => false,
            }
        }

        #[test]
        fn may_moi_cho_2_lan_roi_chan() {
            let kho = Arc::new(KhoGia::moi(2));
            let kt = KiemTra::gia(kho.clone());
            kt.mo_app();
            cho(&kt);
            assert!(chay(&kt, ChucNang::Dem) && chay(&kt, ChucNang::Tach), "Đếm 1 + Tách 1 = 2 lượt chung");
            assert_eq!(kt.bat_dau(ChucNang::MucLuc).unwrap_err(), QuyetDinh::HetLuot(ChucNang::MucLuc));
            assert_eq!(kt.bat_dau(ChucNang::Dem).unwrap_err(), QuyetDinh::HetLuot(ChucNang::Dem));
            assert_eq!(kho.da_dung(), 2);
        }

        #[test]
        fn quyet_dinh_khong_goi_mang() {
            let kho = Arc::new(KhoGia::moi(2));
            let kt = KiemTra::gia(kho.clone());
            let t = Instant::now();
            for _ in 0..100 {
                assert_eq!(kt.quyet_dinh(ChucNang::Dem), QuyetDinh::Cho);
            }
            assert_eq!(kho.so_goi.load(SeqCst), 0);
            assert!(t.elapsed() < Duration::from_millis(200), "{:?}", t.elapsed());
        }

        #[test]
        fn thoat_giua_chung_van_bi_tinh() {
            let kho = Arc::new(KhoGia::moi(2));
            let kt = KiemTra::gia(kho.clone());
            kho.mat_mang.store(true, SeqCst);
            let _bo_do = kt.bat_dau(ChucNang::Tach).unwrap(); // app chết giữa chừng: không ket_thuc
            let kt2 = KiemTra::moi(Some(kho.clone()), SoLuot::moi(kt.chung.so.noi.clone()), kt.chung.ma.clone(), "PC".into(), "AA".into(), Box::new(|| ("?".into(), "?".into())));
            assert_eq!(kt2.chung.so.cho_ghi().len(), 1);
            kho.mat_mang.store(false, SeqCst);
            kt2.mo_app();
            cho(&kt2);
            assert_eq!(kho.da_dung(), 1, "lần bỏ dở gửi trạm khi mở lại");
            assert!(kt2.chung.so.cho_ghi().is_empty());
        }

        #[test]
        fn dung_khong_co_ket_qua_hoan_luot() {
            let kho = Arc::new(KhoGia::moi(2));
            let kt = KiemTra::gia(kho.clone());
            for _ in 0..5 {
                let l = kt.bat_dau(ChucNang::Dem).unwrap();
                kt.ket_thuc(&l, false);
            }
            cho(&kt);
            assert_eq!((kho.da_dung(), kho.so_goi.load(SeqCst)), (0, 1), "chỉ 1 lần hỏi trạm (chưa có trả lời ký), không ghi lượt nào");
        }

        #[test]
        fn dung_co_ket_qua_tinh_luot() {
            let kho = Arc::new(KhoGia::moi(2));
            let kt = KiemTra::gia(kho.clone());
            assert!(chay(&kt, ChucNang::Tach));
            assert_eq!(kho.da_dung(), 1);
            assert!(kt.chung.so.cho_ghi().is_empty(), "trạm xác nhận → xoá khỏi sổ");
        }

        #[test]
        fn timeout_khong_dem_trung() {
            let kho = Arc::new(KhoGia::moi(5));
            let kt = KiemTra::gia(kho.clone());
            kho.loi_sau_ghi.store(true, SeqCst);
            assert!(chay(&kt, ChucNang::Dem));
            assert_eq!(kt.chung.so.cho_ghi().len(), 1, "lỗi → giữ lại để gửi lại");
            kt.mo_app();
            cho(&kt);
            assert_eq!(kho.da_dung(), 1, "gửi lại cùng mã lần không cộng thêm");
            assert!(kt.chung.so.cho_ghi().is_empty());
        }

        #[test]
        fn mot_luong_gui() {
            let mut k = KhoGia::moi(100);
            k.cho = Duration::from_millis(30);
            let kho = Arc::new(k);
            let kt = KiemTra::gia(kho.clone());
            // Chưa được trạm xác nhận thì tối đa 2 lần liên tiếp (luật offline)
            for _ in 0..2 {
                let l = kt.bat_dau(ChucNang::Dem).unwrap();
                kt.ket_thuc(&l, true);
            }
            kt.mo_app();
            kt.thong_ke("thong_ke_dem", |_| serde_json::json!({}));
            cho(&kt);
            assert_eq!(kho.dong_thoi_toi_da.load(SeqCst), 1);
            assert_eq!(kho.da_dung(), 2);
            assert_eq!(kt.quyet_dinh(ChucNang::Dem), QuyetDinh::Cho, "đã xác nhận → chạy tiếp được");
            assert_eq!(*kho.thong_ke_da_gui.lock().unwrap(), ["thong_ke_dem"]);
        }

        #[test]
        fn ip_doc_lai_moi_loi_goi() {
            use std::sync::atomic::AtomicUsize;
            let kho = Arc::new(KhoGia::moi(9));
            let n = Arc::new(AtomicUsize::new(0));
            let n2 = n.clone();
            let d = std::env::temp_dir().join(format!("dema4_kiem_tra_ip_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            let kt = KiemTra::moi(Some(kho.clone()), SoLuot::moi(vec![d]), "SM-0000-0000-0002".into(), "PC".into(), "AA".into(), Box::new(move || (format!("10.0.0.{}", n2.fetch_add(1, SeqCst) + 1), "?".into())));
            kt.mo_app();
            kt.mo_app();
            cho(&kt);
            assert_eq!(*kho.ip.lock().unwrap(), ["10.0.0.1", "10.0.0.2"]);
        }

        #[test]
        fn khoa_sau_lan_chay_ke() {
            let kho = Arc::new(KhoGia::moi(100));
            let kt = KiemTra::gia(kho.clone());
            kt.mo_app();
            cho(&kt);
            *kho.trang_thai.lock().unwrap() = "khoa"; // Sao Mai khoá sau khi app đã hỏi trạm
            assert!(chay(&kt, ChucNang::Dem), "chưa biết bị khoá → lần này vẫn chạy");
            assert_eq!(kt.bat_dau(ChucNang::Dem).unwrap_err(), QuyetDinh::HetLuot(ChucNang::Dem));
        }

        #[test]
        fn khoa_offline_van_khoa() {
            let kho = Arc::new(KhoGia::moi(100));
            let kt = KiemTra::gia(kho.clone());
            *kho.trang_thai.lock().unwrap() = "khoa";
            kt.mo_app();
            cho(&kt);
            kho.mat_mang.store(true, SeqCst);
            let kt2 = KiemTra::moi(Some(kho.clone()), SoLuot::moi(kt.chung.so.noi.clone()), kt.chung.ma.clone(), "PC".into(), "AA".into(), Box::new(|| ("?".into(), "?".into())));
            kt2.mo_app();
            cho(&kt2);
            assert_eq!(kt2.quyet_dinh(ChucNang::Tach), QuyetDinh::HetLuot(ChucNang::Tach), "trả lời ký Khoá lưu đĩa, mất mạng vẫn chặn");
        }

        /// Xoá / sửa câu trả lời ký trên đĩa không mở thêm lượt: có trạm thì hỏi trạm trước khi quyết định.
        #[test]
        fn xoa_tra_loi_tren_dia_khong_vuot_han() {
            let kho = Arc::new(KhoGia::moi(0));
            let kt = KiemTra::gia(kho.clone());
            assert_eq!(kt.bat_dau(ChucNang::Dem).unwrap_err(), QuyetDinh::HetLuot(ChucNang::Dem));
            assert_eq!(kho.so_goi.load(SeqCst), 1, "hỏi trạm đúng 1 lần, lần sau dùng trả lời đã có");
            assert!(kt.bat_dau(ChucNang::Tach).is_err());
            assert_eq!(kho.so_goi.load(SeqCst), 1);
        }

        /// Mất mạng thật: không có trả lời ký → luật offline (máy mới vẫn dùng thử được 2 lần).
        #[test]
        fn mat_mang_khong_co_tra_loi_van_cho_2_lan() {
            let kho = Arc::new(KhoGia::moi(0));
            kho.mat_mang.store(true, SeqCst);
            let kt = KiemTra::gia(kho.clone());
            assert!(chay(&kt, ChucNang::Dem) && chay(&kt, ChucNang::Dem));
            assert_eq!(kt.bat_dau(ChucNang::Dem).unwrap_err(), QuyetDinh::HetLuotOffline(ChucNang::Dem));
        }

        #[test]
        fn khong_co_tram_chi_2_lan() {
            let d = std::env::temp_dir().join(format!("dema4_kiem_tra_khong_tram_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            let kt = KiemTra::moi(None, SoLuot::moi(vec![d]), "SM-0000-0000-0003".into(), "PC".into(), "AA".into(), Box::new(|| ("?".into(), "?".into())));
            assert!(chay(&kt, ChucNang::Dem) && chay(&kt, ChucNang::Dem));
            assert_eq!(kt.bat_dau(ChucNang::Dem).unwrap_err(), QuyetDinh::HetLuotOffline(ChucNang::Dem));
            assert!(!kt.thong_ke("thong_ke_dem", |_| serde_json::json!({})));
        }
    }
}
