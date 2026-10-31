//! Kết nối trạm license (Google Apps Script): gọi HTTPS, kiểm chữ ký RSA câu trả lời, gửi số liệu thống kê.
//! App không giữ khoá ghi Sheet hay token Telegram — chỉ URL trạm + khoá công khai (`assets/license_pub.pem`).

use crate::license::{LanChay, TraLoiKy, TrangThaiMay};
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;

/// URL web app trạm (README apps-script, bước 5; sửa script dùng New version để giữ URL). Trống = app dùng luật offline, không gửi thống kê.
pub const URL_TRAM: &str = "https://script.google.com/macros/s/AKfycbyR90faddKygBf70AcElFIT9Rfxo0jZG78DDqBUPLN5OEbqQxOxj8MesqpzQMs6hzpRzw/exec";
const KHOA_CONG_KHAI_PEM: &str = include_str!("../assets/license_pub.pem");
/// Trạm trả lời ~3 s (lần đầu sau lúc nghỉ ~7 s, gồm chuyển hướng 302 của Apps Script); lời gọi chạy nền nên chờ rộng tay.
const CHO_TOI_DA: Duration = Duration::from_secs(15);

/// Thông tin máy gửi kèm mỗi lời gọi; IP đọc lại mỗi lần (máy có thể đã chuyển mạng).
#[derive(Debug, Clone, PartialEq)]
pub struct ThongTinMay {
    pub ma: String,
    pub ten: String,
    pub mac: String,
    pub ip_noi_bo: String,
    pub ip_public: String,
    pub phien_ban: String,
}

/// Câu trả lời trạm đã kiểm chữ ký.
#[derive(Debug, Clone, PartialEq)]
pub struct KetQuaTram {
    pub ky: TraLoiKy,
    pub may: TrangThaiMay,
    pub da_ghi: Vec<String>,
}

/// Nơi lưu hạn mức / lần chạy: trạm thật, hoặc kho giả trong test.
pub trait KhoLicense: Send + Sync {
    fn kiem_tra(&self, may: &ThongTinMay) -> Result<KetQuaTram, String>;
    /// `online`: lần chạy xong trong phiên này (nguồn "online"); các lần khác là bù offline.
    fn ghi_lan(&self, may: &ThongTinMay, lan: &[LanChay], online: &std::collections::HashSet<String>) -> Result<KetQuaTram, String>;
    /// `hanh_dong`: "thong_ke_dem" | "thong_ke_tach"; `them`: các trường số liệu (`so_lieu`, `nguong`).
    fn thong_ke(&self, may: &ThongTinMay, hanh_dong: &str, them: Value) -> Result<(), String>;
    /// Câu trả lời đã lưu trên đĩa còn tin được không (chữ ký, mã máy; không kiểm nonce).
    fn doc_da_luu(&self, tl: &TraLoiKy, ma_may: &str) -> Option<TrangThaiMay>;
}

pub struct KhoTram {
    url: &'static str,
    khoa: rsa::RsaPublicKey,
}

impl KhoTram {
    /// None khi chưa triển khai trạm (URL trống hoặc chưa có khoá công khai).
    pub fn moi() -> Option<Self> {
        use rsa::pkcs8::DecodePublicKey;
        let khoa = rsa::RsaPublicKey::from_public_key_pem(KHOA_CONG_KHAI_PEM.trim()).ok()?;
        (!URL_TRAM.is_empty()).then_some(KhoTram { url: URL_TRAM, khoa })
    }

    fn goi(&self, may: &ThongTinMay, hanh_dong: &str, nonce: &str, them: Value) -> Result<String, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(CHO_TOI_DA)).http_status_as_error(false).build().into();
        let mut ph = agent.post(self.url).header("Content-Type", "application/json").send(yeu_cau(may, hanh_dong, nonce, them).to_string()).map_err(|e| format!("không kết nối được trạm ({e})"))?;
        ph.body_mut().read_to_string().map_err(|e| format!("không đọc được trả lời trạm ({e})"))
    }

    fn goi_ky(&self, may: &ThongTinMay, hanh_dong: &str, them: Value) -> Result<KetQuaTram, String> {
        let nonce = crate::license::ngau_nhien_hex();
        let tl = phan_tich_phan_hoi(&self.goi(may, hanh_dong, &nonce, them)?)?;
        doc_tra_loi(&tl, &self.khoa, &may.ma, Some(&nonce))
    }

}

impl KhoLicense for KhoTram {
    fn kiem_tra(&self, may: &ThongTinMay) -> Result<KetQuaTram, String> {
        self.goi_ky(may, "kiem_tra", json!({}))
    }

    fn ghi_lan(&self, may: &ThongTinMay, lan: &[LanChay], online: &std::collections::HashSet<String>) -> Result<KetQuaTram, String> {
        self.goi_ky(may, "ghi_lan", json!({ "lan": ds_lan(lan, online) }))
    }

    fn thong_ke(&self, may: &ThongTinMay, hanh_dong: &str, them: Value) -> Result<(), String> {
        let than = self.goi(may, hanh_dong, &crate::license::ngau_nhien_hex(), them)?;
        match serde_json::from_str::<Value>(&than) {
            Ok(v) if v["ok"] == true => Ok(()),
            Ok(v) => Err(v["loi"].as_str().unwrap_or("trạm báo lỗi").to_string()),
            Err(_) => Err("trạm trả về không phải JSON".into()),
        }
    }

    fn doc_da_luu(&self, tl: &TraLoiKy, ma_may: &str) -> Option<TrangThaiMay> {
        doc_tra_loi(tl, &self.khoa, ma_may, None).ok().map(|k| k.may)
    }
}

fn yeu_cau(may: &ThongTinMay, hanh_dong: &str, nonce: &str, them: Value) -> Value {
    let mut v = json!({
        "hanh_dong": hanh_dong,
        "ma_may": may.ma,
        "nonce": nonce,
        "may": { "ten": may.ten, "mac": may.mac, "ip_noi_bo": may.ip_noi_bo, "ip_public": may.ip_public, "phien_ban": may.phien_ban },
    });
    if let (Some(dst), Value::Object(src)) = (v.as_object_mut(), them) {
        dst.extend(src);
    }
    v
}

fn ds_lan(lan: &[LanChay], online: &std::collections::HashSet<String>) -> Value {
    lan.iter().map(|l| json!({ "ma_lan": l.ma, "cn": l.cn.ma(), "luc": l.luc, "nguon": if online.contains(&l.ma) { "online" } else { "bu_offline" } })).collect()
}

/// Thân trả lời của trạm → cặp dữ liệu / chữ ký. HTML (trang đăng nhập, lỗi Google), `ok:false`, thiếu trường → lỗi.
pub fn phan_tich_phan_hoi(than: &str) -> Result<TraLoiKy, String> {
    let v: Value = serde_json::from_str(than).map_err(|_| "trạm trả về không phải JSON".to_string())?;
    if v["ok"] != true {
        return Err(v["loi"].as_str().unwrap_or("trạm báo lỗi").to_string());
    }
    match (v["du_lieu"].as_str(), v["chu_ky"].as_str()) {
        (Some(d), Some(c)) => Ok(TraLoiKy { du_lieu: d.into(), chu_ky: c.into() }),
        _ => Err("trả lời thiếu chữ ký".into()),
    }
}

/// Chỉ tin câu trả lời có chữ ký đúng (RSA PKCS#1 v1.5 SHA-256, như `Utilities.computeRsaSha256Signature`),
/// đúng mã máy, và đúng nonce của lời gọi (`None` khi đọc lại bản đã lưu).
pub fn doc_tra_loi(tl: &TraLoiKy, khoa: &rsa::RsaPublicKey, ma_may: &str, nonce: Option<&str>) -> Result<KetQuaTram, String> {
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::signature::Verifier;
    let ky = base64::engine::general_purpose::STANDARD.decode(tl.chu_ky.trim()).map_err(|_| "chữ ký không phải base64".to_string())?;
    let ky = Signature::try_from(ky.as_slice()).map_err(|_| "chữ ký sai định dạng".to_string())?;
    VerifyingKey::<rsa::sha2::Sha256>::new(khoa.clone()).verify(tl.du_lieu.as_bytes(), &ky).map_err(|_| "chữ ký không hợp lệ".to_string())?;
    giai_du_lieu(tl, ma_may, nonce)
}


/// Phần dữ liệu (đã kiểm chữ ký) → trạng thái máy; sai mã máy / nonce → lỗi.
fn giai_du_lieu(tl: &TraLoiKy, ma_may: &str, nonce: Option<&str>) -> Result<KetQuaTram, String> {
    let v: Value = serde_json::from_str(&tl.du_lieu).map_err(|_| "dữ liệu ký không phải JSON".to_string())?;
    if v["ma_may"].as_str() != Some(ma_may) {
        return Err("câu trả lời không phải của máy này".into());
    }
    if nonce.is_some_and(|n| v["nonce"].as_str() != Some(n)) {
        return Err("câu trả lời không khớp lời gọi".into());
    }
    let so = |k: &str| v[k].as_u64().map(|x| x.min(u32::MAX as u64) as u32);
    // Trạm hạn mức chung gửi `han` / `da_dung`; trả lời ký cũ (lưu đĩa từ trạm theo từng chức năng) → cộng các chức năng
    let tong = |k: &str| -> Result<u32, String> {
        match so(k) {
            Some(x) => Ok(x),
            None => {
                let (dem, tach) = (so(&format!("{k}_dem")).ok_or_else(|| format!("thiếu {k}"))?, so(&format!("{k}_tach")).ok_or_else(|| format!("thiếu {k}"))?);
                Ok(dem.saturating_add(tach).saturating_add(so(&format!("{k}_muc_luc")).unwrap_or(0)))
            }
        }
    };
    let may = TrangThaiMay {
        han: tong("han")?,
        da_dung: tong("da_dung")?,
        trang_thai: crate::license::trang_thai_tu_ma(v["trang_thai"].as_str().unwrap_or("")),
    };
    let da_ghi = v["da_ghi"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    Ok(KetQuaTram { ky: tl.clone(), may, da_ghi })
}

fn duong_dan_tuyet_doi(thu_muc: &str) -> String {
    std::path::absolute(thu_muc).map(|p| p.display().to_string()).unwrap_or_else(|_| thu_muc.to_string())
}

/// Số liệu 1 lần Đếm gửi trạm (trạm tự soạn tin Telegram).
pub fn so_lieu_dem(thu_muc: &str, kq: &crate::dem_pdf::KetQuaDem, thoi_gian: Duration) -> Value {
    json!({ "so_lieu": {
        "thu_muc": duong_dan_tuyet_doi(thu_muc),
        "so_file": kq.dong_tong_hop.len(),
        "so_trang": kq.tong_so_trang,
        "quy_a4": kq.quy_a4.unwrap_or(0.0),
        "canh_bao": kq.so_trang_canh_bao,
        "file_loi": kq.so_file_loi,
        "thoi_gian_ms": thoi_gian.as_millis() as u64,
    }})
}

/// Số liệu 1 lần Sinh mục lục gửi trạm (tin 📑 SINH MỤC LỤC).
pub fn so_lieu_muc_luc(du_lieu: &str, mau: &str, ho_so: usize, van_ban: usize, tk: &crate::muc_luc::sinh::TongKet, pdf: usize) -> Value {
    json!({ "so_lieu": {
        "du_lieu": duong_dan_tuyet_doi(du_lieu),
        "mau": mau,
        "ho_so": ho_so,
        "van_ban": van_ban,
        "docx": tk.docx,
        "pdf": pdf,
        "loi": tk.loi.len(),
        "thoi_gian_ms": tk.thoi_gian.as_millis() as u64,
    }})
}

/// Số liệu 1 lần Tách + thống kê ngưỡng trắng (tin, file CSV, 1 hàng tab "Ngưỡng trắng" — trạm ghi).
pub fn so_lieu_tach(may: &ThongTinMay, thu_muc: &str, sl: &crate::telegram::SoLieuTach, thoi_gian: Duration, thong_ke: Option<&crate::tach_vb::thong_ke_nguong::ThongKeNguong>, nguong: (f32, f32)) -> Value {
    use crate::tach_vb::thong_ke_nguong as tkn;
    let mut v = json!({ "so_lieu": {
        "thu_muc": duong_dan_tuyet_doi(thu_muc),
        "ho_so": sl.ho_so, "van_ban": sl.van_ban, "so_trang": sl.so_trang, "bo_index": sl.bo_index, "bo_trang": sl.bo_trang,
        "split": sl.split, "sao_nguyen": sl.sao_nguyen, "ho_so_loi": sl.ho_so_loi,
        "thoi_gian_ms": thoi_gian.as_millis() as u64,
    }});
    let Some(tk) = thong_ke else { return v };
    let bay_gio = chrono::Local::now();
    let ten = std::path::Path::new(thu_muc.trim_end_matches(['/', '\\'])).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let dau = [
        bay_gio.format("%Y-%m-%d %H:%M:%S").to_string(),
        may.ten.clone(),
        may.mac.clone(),
        may.ip_noi_bo.clone(),
        may.ip_public.clone(),
        ten.clone(),
        may.phien_ban.clone(),
        format!("{:.2}", nguong.0),
        format!("{:.2}", nguong.1),
    ];
    let csv = tkn::csv(tk, &tkn::KHOA_CSV.iter().copied().zip(dau.iter().cloned()).collect::<Vec<_>>());
    v["nguong"] = json!({
        "t1": nguong.0,
        "t2": nguong.1,
        "tk": {
            "trang_tu_dong": tk.trang_tu_dong, "trang_tu_dong_sua": tk.trang_tu_dong_sua, "nghi": tk.nghi, "nghi_thanh_trang": tk.nghi_thanh_trang,
            "nghi_giu": tk.nghi_giu, "trang_lot": tk.trang_lot, "de_xuat": tk.de_xuat.map(|(a, b)| [a, b]),
            "xoay_xac_nhan": tk.xoay_xac_nhan, "xoay_chinh": tk.xoay_chinh, "xoay_0": tk.xoay_0,
        },
        "csv": csv,
        "ten_csv": format!("nguong_{ten}_{}.csv", bay_gio.format("%Y%m%d-%H%M%S")),
        "tieu_de": tkn::tieu_de_sheet(),
        "mo_ta": tkn::mo_ta_sheet(),
        "dong": tkn::dong_sheet(tk, dau),
    });
    v
}

/// Kho giả cho test: giữ hạn mức / đã dùng trong bộ nhớ, chống trùng theo mã lần như trạm, đếm lời gọi.
#[cfg(test)]
pub struct KhoGia {
    pub han: std::sync::Mutex<u32>,
    pub trang_thai: std::sync::Mutex<&'static str>,
    /// Mọi lời gọi lỗi (mất mạng).
    pub mat_mang: std::sync::atomic::AtomicBool,
    /// Ghi xong rồi mới báo lỗi (giả hết thời gian chờ).
    pub loi_sau_ghi: std::sync::atomic::AtomicBool,
    pub cho: Duration,
    dung: std::sync::Mutex<u32>,
    da_ghi: std::sync::Mutex<std::collections::HashSet<String>>,
    pub so_goi: std::sync::atomic::AtomicUsize,
    dang_goi: std::sync::atomic::AtomicUsize,
    pub dong_thoi_toi_da: std::sync::atomic::AtomicUsize,
    /// IP nội bộ của từng lời gọi, theo thứ tự.
    pub ip: std::sync::Mutex<Vec<String>>,
    pub thong_ke_da_gui: std::sync::Mutex<Vec<String>>,
}

#[cfg(test)]
impl KhoGia {
    /// `han`: tổng hạn mức chung mọi chức năng.
    pub fn moi(han: u32) -> Self {
        KhoGia {
            han: std::sync::Mutex::new(han),
            trang_thai: std::sync::Mutex::new("dung_thu"),
            mat_mang: Default::default(),
            loi_sau_ghi: Default::default(),
            cho: Duration::ZERO,
            dung: Default::default(),
            da_ghi: Default::default(),
            so_goi: Default::default(),
            dang_goi: Default::default(),
            dong_thoi_toi_da: Default::default(),
            ip: Default::default(),
            thong_ke_da_gui: Default::default(),
        }
    }

    pub fn da_dung(&self) -> u32 {
        *self.dung.lock().unwrap()
    }

    fn goi<R>(&self, may: &ThongTinMay, f: impl FnOnce() -> Result<R, String>) -> Result<R, String> {
        use std::sync::atomic::Ordering::SeqCst;
        self.so_goi.fetch_add(1, SeqCst);
        let n = self.dang_goi.fetch_add(1, SeqCst) + 1;
        self.dong_thoi_toi_da.fetch_max(n, SeqCst);
        self.ip.lock().unwrap().push(may.ip_noi_bo.clone());
        std::thread::sleep(self.cho);
        let kq = if self.mat_mang.load(SeqCst) { Err("mất mạng".into()) } else { f() };
        self.dang_goi.fetch_sub(1, SeqCst);
        kq
    }

    fn tra_loi(&self, may: &ThongTinMay, da_ghi: Vec<String>) -> KetQuaTram {
        let (han, dung) = (*self.han.lock().unwrap(), self.da_dung());
        let du_lieu = json!({ "ma_may": may.ma, "nonce": "", "han": han, "da_dung": dung, "trang_thai": *self.trang_thai.lock().unwrap(), "da_ghi": da_ghi });
        giai_du_lieu(&TraLoiKy { du_lieu: du_lieu.to_string(), chu_ky: "gia".into() }, &may.ma, None).unwrap()
    }
}

#[cfg(test)]
impl KhoLicense for KhoGia {
    fn kiem_tra(&self, may: &ThongTinMay) -> Result<KetQuaTram, String> {
        self.goi(may, || Ok(self.tra_loi(may, Vec::new())))
    }

    fn ghi_lan(&self, may: &ThongTinMay, lan: &[LanChay], _online: &std::collections::HashSet<String>) -> Result<KetQuaTram, String> {
        self.goi(may, || {
            let mut ghi = Vec::new();
            for l in lan {
                if self.da_ghi.lock().unwrap().insert(l.ma.clone()) {
                    *self.dung.lock().unwrap() += 1;
                }
                ghi.push(l.ma.clone());
            }
            if self.loi_sau_ghi.swap(false, std::sync::atomic::Ordering::SeqCst) {
                return Err("hết thời gian chờ".into());
            }
            Ok(self.tra_loi(may, ghi))
        })
    }

    fn thong_ke(&self, may: &ThongTinMay, hanh_dong: &str, _them: Value) -> Result<(), String> {
        self.goi(may, || {
            self.thong_ke_da_gui.lock().unwrap().push(hanh_dong.into());
            Ok(())
        })
    }

    fn doc_da_luu(&self, tl: &TraLoiKy, ma_may: &str) -> Option<TrangThaiMay> {
        giai_du_lieu(tl, ma_may, None).ok().map(|k| k.may)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::TrangThai;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    use base64::Engine;

    fn cap_khoa() -> (SigningKey<rsa::sha2::Sha256>, rsa::RsaPublicKey) {
        let bi_mat = rsa::RsaPrivateKey::new(&mut rand_core::OsRng, 1024).unwrap();
        let cong_khai = bi_mat.to_public_key();
        (SigningKey::new(bi_mat), cong_khai)
    }

    fn ky(k: &SigningKey<rsa::sha2::Sha256>, du_lieu: &str) -> TraLoiKy {
        TraLoiKy { du_lieu: du_lieu.into(), chu_ky: base64::engine::general_purpose::STANDARD.encode(k.sign(du_lieu.as_bytes()).to_bytes()) }
    }

    const DL: &str = r#"{"ma_may":"SM-2145-6787-A9FB","nonce":"n1","han_dem":200,"han_tach":2,"da_dung_dem":37,"da_dung_tach":1,"trang_thai":"da_cap","da_ghi":["0000000000000000000000000000000a"]}"#;

    #[test]
    fn chu_ky_hop_le() {
        let (k, pk) = cap_khoa();
        let kq = doc_tra_loi(&ky(&k, DL), &pk, "SM-2145-6787-A9FB", Some("n1")).unwrap();
        assert_eq!((kq.may.han, kq.may.da_dung, kq.may.trang_thai), (202, 38, TrangThai::DaCap), "trả lời ký cũ theo chức năng → cộng tổng");
        assert_eq!(kq.da_ghi, ["0000000000000000000000000000000a"]);
        let moi = DL.replace("\"trang_thai\"", "\"han\":50,\"da_dung\":7,\"trang_thai\"");
        let kq = doc_tra_loi(&ky(&k, &moi), &pk, "SM-2145-6787-A9FB", Some("n1")).unwrap();
        assert_eq!((kq.may.han, kq.may.da_dung), (50, 7), "trạm hạn mức chung: đọc han / da_dung");
        assert!(doc_tra_loi(&ky(&k, DL), &pk, "SM-2145-6787-A9FB", None).is_ok(), "đọc lại từ đĩa: không kiểm nonce");
    }

    #[test]
    fn chu_ky_sai_bi_tu_choi() {
        let (k, pk) = cap_khoa();
        let mut tl = ky(&k, DL);
        tl.du_lieu = tl.du_lieu.replace("200", "900");
        assert!(doc_tra_loi(&tl, &pk, "SM-2145-6787-A9FB", Some("n1")).is_err(), "sửa dữ liệu");
        assert!(doc_tra_loi(&ky(&k, DL), &pk, "SM-0000-0000-0000", Some("n1")).is_err(), "câu trả lời của máy khác");
        assert!(doc_tra_loi(&ky(&k, DL), &pk, "SM-2145-6787-A9FB", Some("n2")).is_err(), "phát lại câu trả lời cũ");
        let (k2, _) = cap_khoa();
        assert!(doc_tra_loi(&ky(&k2, DL), &pk, "SM-2145-6787-A9FB", Some("n1")).is_err(), "ký bằng khoá khác");
    }

    #[test]
    fn phan_hoi_khong_hop_le_la_loi() {
        assert!(phan_tich_phan_hoi("<!DOCTYPE html><html>Moved Temporarily</html>").is_err());
        assert_eq!(phan_tich_phan_hoi(r#"{"ok":false,"loi":"ban"}"#), Err("ban".into()));
        assert!(phan_tich_phan_hoi(r#"{"ok":true}"#).is_err(), "thiếu chữ ký");
        assert_eq!(phan_tich_phan_hoi(r#"{"ok":true,"du_lieu":"{}","chu_ky":"YQ=="}"#), Ok(TraLoiKy { du_lieu: "{}".into(), chu_ky: "YQ==".into() }));
    }

    /// Câu trả lời thật của trạm đã triển khai (Utilities.computeRsaSha256Signature) kiểm được bằng khoá công khai nhúng.
    #[test]
    fn chu_ky_tu_apps_script_kiem_duoc() {
        use rsa::pkcs8::DecodePublicKey;
        let khoa = rsa::RsaPublicKey::from_public_key_pem(KHOA_CONG_KHAI_PEM.trim()).expect("khoá công khai nhúng hợp lệ");
        let tl = TraLoiKy { du_lieu: "{\"ma_may\":\"SM-0000-0000-0000\",\"nonce\":\"thu1\",\"han_dem\":2,\"han_tach\":2,\"da_dung_dem\":0,\"da_dung_tach\":0,\"trang_thai\":\"dung_thu\",\"da_ghi\":[]}".into(), chu_ky: "ZwEMMvTPwXdWL9ADG/PuSChGAlUdWCU7ScsUztQvmFEXXwHAGH/SsGbDyG6U2Z7/5lJ4x0FkifvFekvPDNNOoWpCDeCzR2YkOEcIR784pD0atoW50OQ0948vqCU2kkU2G9p8e7Hye1vPpOHyrRYXdutg+V9N2P+HTpkw6JTj4AcSp0ypp0b801aDNsE10bkOwEDzZqP/xXL/2aJXfRBnhFW7zboCKAUZ6k9Y+Ts1U4DHMa+60CQ+EyOqleRzB1yxWIZpPbJZfE7+xrfq6EF48/f1XBYnusfHrsx/YZiArvuRfEwNHIL7i/HX9BW830rh0mqPPVcTcleNKFSab8viKg==".into() };
        let kq = doc_tra_loi(&tl, &khoa, "SM-0000-0000-0000", Some("thu1")).unwrap();
        assert_eq!((kq.may.han, kq.may.trang_thai), (4, crate::license::TrangThai::DungThu));
        assert!(KhoTram::moi().is_some(), "URL + khoá đã điền");
    }

    /// Gọi trạm thật (cần mạng): đăng ký máy thử, câu trả lời ký đúng.
    #[test]
    #[ignore = "gọi trạm thật"]
    fn kho_tram_that() {
        let kho = KhoTram::moi().unwrap();
        let mut m = may();
        m.ma = "SM-0000-0000-0000".into();
        let kq = kho.kiem_tra(&m).unwrap();
        println!("{:?}", kq.may);
    }

    fn may() -> ThongTinMay {
        ThongTinMay { ma: "SM-2145-6787-A9FB".into(), ten: "PC-01".into(), mac: "AA:BB".into(), ip_noi_bo: "10.0.0.2".into(), ip_public: "1.2.3.4".into(), phien_ban: "0.6.0".into() }
    }

    #[test]
    fn yeu_cau_dung_dinh_dang_tram() {
        use crate::license::{ChucNang, TrangThaiLan};
        let lan = |ma: &str| LanChay { ma: ma.into(), cn: ChucNang::Tach, luc: "x".into(), trang_thai: TrangThaiLan::DaChay };
        let v = yeu_cau(&may(), "ghi_lan", "n1", json!({ "lan": ds_lan(&[lan("a"), lan("b")], &["b".to_string()].into()) }));
        assert_eq!((v["hanh_dong"].as_str(), v["ma_may"].as_str(), v["nonce"].as_str()), (Some("ghi_lan"), Some("SM-2145-6787-A9FB"), Some("n1")));
        assert_eq!(v["may"]["ip_public"], "1.2.3.4");
        assert_eq!((v["lan"][0]["nguon"].as_str(), v["lan"][1]["nguon"].as_str(), v["lan"][1]["cn"].as_str()), (Some("bu_offline"), Some("online"), Some("tach")));
    }

    /// Đủ trường trạm đọc (loi.js: soanTinDem / soanTinTach / dongNguong / themNguong); chữ tự do chỉ có đường dẫn thư mục.
    #[test]
    fn so_lieu_muc_luc_du_truong() {
        let tk = crate::muc_luc::sinh::TongKet { docx: 21, bo_qua: 0, loi: vec![("a.docx".into(), "x".into())], da_dung: false, thoi_gian: Duration::from_millis(16_400) };
        let v = so_lieu_muc_luc("/a/Bien muc.xlsx", "Mẫu 01 – Quảng Trị", 22, 326, &tk, 0)["so_lieu"].clone();
        assert_eq!((v["ho_so"].as_u64(), v["van_ban"].as_u64(), v["docx"].as_u64(), v["pdf"].as_u64(), v["loi"].as_u64(), v["thoi_gian_ms"].as_u64()), (Some(22), Some(326), Some(21), Some(0), Some(1), Some(16_400)));
        assert_eq!(v["mau"], "Mẫu 01 – Quảng Trị");
        assert!(v["du_lieu"].as_str().unwrap().ends_with("Bien muc.xlsx"));
    }

    #[test]
    fn so_lieu_gui_tram_du_truong() {
        use crate::tach_vb::thong_ke_nguong::ThongKeNguong;
        let kq = crate::dem_pdf::KetQuaDem { dong_tong_hop: vec![String::new(); 4], dong_chi_tiet: vec![], tong_hop: "bí mật".into(), so_file_loi: 1, tong_so_trang: 78, quy_a4: Some(128.5), so_trang_canh_bao: 2 };
        let d = so_lieu_dem("/a/b", &kq, Duration::from_millis(65_400));
        assert_eq!(d["so_lieu"], json!({ "thu_muc": "/a/b", "so_file": 4, "so_trang": 78, "quy_a4": 128.5, "canh_bao": 2, "file_loi": 1, "thoi_gian_ms": 65_400 }));
        assert!(!d.to_string().contains("bí mật"), "không gửi nội dung kết quả đếm");

        let sl = crate::telegram::SoLieuTach { ho_so: 2, van_ban: 211, so_trang: 476, bo_index: 209, bo_trang: 1, ..Default::default() };
        let t = so_lieu_tach(&may(), "/a/ho_so", &sl, Duration::from_secs(2), None, (1.0, 3.2));
        assert_eq!(t["so_lieu"]["van_ban"], 211);
        assert!(t.get("nguong").is_none());
        let tk = ThongKeNguong { o: vec![(0, 0); 101], trang_tu_dong: 120, de_xuat: Some((0.9, 3.4)), ..Default::default() };
        let t = so_lieu_tach(&may(), "/a/ho_so", &sl, Duration::from_secs(2), Some(&tk), (1.0, 3.2));
        let ng = &t["nguong"];
        assert_eq!((ng["t1"].as_f64(), ng["tk"]["trang_tu_dong"].as_u64(), ng["tk"]["de_xuat"][1].as_f64().map(|x| (x * 10.0).round())), (Some(1.0), Some(120), Some(34.0)));
        assert!(ng["ten_csv"].as_str().unwrap().starts_with("nguong_ho_so_") && ng["csv"].as_str().unwrap().contains("may=PC-01"));
        assert_eq!(ng["dong"].as_array().unwrap().len(), ng["tieu_de"].as_array().unwrap().len());
        assert_eq!(ng["dong"][5], "ho_so");
    }

    /// Mã nguồn app không còn khoá ghi Sheet / token Telegram / đọc file token cũ.
    #[test]
    fn khong_con_khoa_va_token() {
        let goc = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mau_token = |s: &str| s.split(|c: char| !(c.is_ascii_alphanumeric() || c == ':' || c == '_' || c == '-')).any(|w| w.split_once(':').is_some_and(|(a, b)| a.len() >= 8 && a.bytes().all(|x| x.is_ascii_digit()) && b.len() >= 30));
        let mut tep = vec![goc.join("build.rs")];
        for d in ["src", "src/gui_tach", "src/tach_vb"] {
            for m in std::fs::read_dir(goc.join(d)).unwrap().flatten() {
                if m.path().extension().is_some_and(|e| e == "rs") {
                    tep.push(m.path());
                }
            }
        }
        for p in tep {
            let s = std::fs::read_to_string(&p).unwrap();
            let ten = p.display().to_string();
            assert!(!s.contains(&["private", "_key"].concat()), "{ten} còn khoá service account");
            assert!(!mau_token(&s), "{ten} còn token bot Telegram");
            assert!(!s.contains(&["telegram", ".txt"].concat()), "{ten} còn đọc file token");
        }
    }
}
