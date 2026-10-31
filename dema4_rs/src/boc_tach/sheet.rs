//! Ghi kết quả bóc lên Google Sheet, 2 cách:
//! - khoá service account: ký JWT RS256 (`rsa` + `sha2`) → token OAuth → Sheets API (tạo tab + dòng tiêu đề nếu chưa có,
//!   `values:append` thêm dòng, ghi đè dòng theo Tên file khi sửa tay);
//! - Apps Script: POST JSON tới URL web app của đơn vị (script mẫu `apps-script/boc_tach.gs`).
//! Hàng đợi chạy nền: mất mạng không chặn bóc tách, dòng chưa gửi được thử lại sau.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::{json, Value};

const URL_TOKEN: &str = "https://oauth2.googleapis.com/token";
const URL_SHEETS: &str = "https://sheets.googleapis.com/v4/spreadsheets";
const PHAM_VI: &str = "https://www.googleapis.com/auth/spreadsheets";

#[derive(Clone)]
pub enum CachGhi {
    /// email + khoá riêng PEM đọc từ file JSON service account (không chép vào cấu hình / log).
    ServiceAccount { email: String, khoa_pem: String },
    AppsScript { url: String },
}

pub struct DichSheet {
    cach: CachGhi,
    sheet_id: String,
    tab: String,
    url_token: String,
    url_sheets: String,
    token: Mutex<Option<(String, Instant)>>,
    /// Đã có tab + dòng tiêu đề.
    da_chuan_bi: Mutex<bool>,
}

/// ID sheet trong link `https://docs.google.com/spreadsheets/d/<id>/edit…` (hoặc dán thẳng ID).
pub fn id_sheet(link: &str) -> Option<String> {
    let s = link.trim();
    let id = match s.split_once("/spreadsheets/d/") {
        Some((_, sau)) => sau.split(['/', '?', '#']).next().unwrap_or(""),
        None if !s.contains('/') => s,
        None => "",
    };
    (id.len() >= 20 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then(|| id.to_string())
}

/// Đọc file khoá JSON service account → (email, khoá PEM).
pub fn doc_khoa_sa(duong_dan: &str) -> Result<(String, String), String> {
    let s = std::fs::read_to_string(duong_dan.trim()).map_err(|e| format!("Không đọc được file khoá: {e}"))?;
    let v: Value = serde_json::from_str(&s).map_err(|_| "File khoá không phải JSON service account".to_string())?;
    match (v["client_email"].as_str(), v["private_key"].as_str()) {
        (Some(e), Some(k)) => Ok((e.to_string(), k.to_string())),
        _ => Err("File khoá thiếu client_email / private_key".into()),
    }
}

fn b64url(b: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

/// JWT RS256 xin token OAuth cho service account (hiệu lực 1 giờ).
pub fn jwt(email: &str, khoa_pem: &str, aud: &str, bay_gio: u64) -> Result<String, String> {
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::signature::{SignatureEncoding, Signer};
    let khoa = rsa::RsaPrivateKey::from_pkcs8_pem(khoa_pem).map_err(|_| "Khoá riêng trong file JSON không đọc được".to_string())?;
    let dau = b64url(br#"{"alg":"RS256","typ":"JWT"}"#);
    let than = b64url(json!({ "iss": email, "scope": PHAM_VI, "aud": aud, "iat": bay_gio, "exp": bay_gio + 3600 }).to_string().as_bytes());
    let noi_dung = format!("{dau}.{than}");
    let ky = rsa::pkcs1v15::SigningKey::<rsa::sha2::Sha256>::new(khoa).sign(noi_dung.as_bytes());
    Ok(format!("{noi_dung}.{}", b64url(&ky.to_bytes())))
}

/// Mã hoá phần đường dẫn URL (tên tab có dấu, khoảng trắng, dấu nháy…).
fn ma_url(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).http_status_as_error(false).build().into()
}

/// Trả lời HTTP → JSON; mã lỗi → thông báo của Google / script.
fn tra_loi(kq: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, String> {
    let mut ph = kq.map_err(|e| format!("Không kết nối được Google: {e}"))?;
    let ma = ph.status().as_u16();
    let than = ph.body_mut().read_to_string().unwrap_or_default();
    let v: Value = serde_json::from_str(&than).unwrap_or(Value::Null);
    if (200..300).contains(&ma) {
        return Ok(v);
    }
    let tb = v["error"]["message"].as_str().or(v["error_description"].as_str()).or(v["error"].as_str()).map(str::to_string).unwrap_or_else(|| than.chars().take(160).collect());
    Err(match ma {
        403 => format!("Không có quyền ghi Sheet — chia sẻ Sheet cho email service account quyền Biên tập ({tb})"),
        404 => format!("Không tìm thấy Sheet — kiểm tra link ({tb})"),
        _ => format!("Google báo lỗi {ma}: {tb}"),
    })
}

impl DichSheet {
    pub fn moi(cach: CachGhi, link: &str, tab: &str) -> Result<Self, String> {
        let sheet_id = match &cach {
            CachGhi::ServiceAccount { .. } => id_sheet(link).ok_or("Link Google Sheet không đúng")?,
            CachGhi::AppsScript { url } if !url.trim().starts_with("https://") && !url.trim().starts_with("http://") => return Err("URL Web app không đúng".into()),
            CachGhi::AppsScript { .. } => String::new(),
        };
        let tab = if tab.trim().is_empty() { "Boc_tach".to_string() } else { tab.trim().to_string() };
        Ok(Self { cach, sheet_id, tab, url_token: URL_TOKEN.into(), url_sheets: URL_SHEETS.into(), token: Mutex::new(None), da_chuan_bi: Mutex::new(false) })
    }

    /// Đổi địa chỉ Google sang máy chủ giả (test).
    #[cfg(test)]
    fn dia_chi_thu(mut self, url: &str) -> Self {
        self.url_token = format!("{url}/token");
        self.url_sheets = format!("{url}/sheets");
        self
    }

    fn token(&self, email: &str, khoa_pem: &str) -> Result<String, String> {
        if let Some((t, _)) = self.token.lock().unwrap().as_ref().filter(|(_, het)| Instant::now() < *het) {
            return Ok(t.clone());
        }
        let bay_gio = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let j = jwt(email, khoa_pem, URL_TOKEN, bay_gio)?;
        let than = format!("grant_type={}&assertion={j}", ma_url("urn:ietf:params:oauth:grant-type:jwt-bearer"));
        let v = tra_loi(agent().post(&self.url_token).header("Content-Type", "application/x-www-form-urlencoded").send(than))?;
        let t = v["access_token"].as_str().ok_or("Google không trả token")?.to_string();
        *self.token.lock().unwrap() = Some((t.clone(), Instant::now() + Duration::from_secs(3000)));
        Ok(t)
    }

    fn pham_vi(&self, o: &str) -> String {
        ma_url(&format!("'{}'!{o}", self.tab.replace('\'', "''")))
    }

    /// Có tab + dòng tiêu đề (tạo nếu chưa có) — chỉ làm 1 lần.
    fn chuan_bi(&self, tk: &str, tieu_de: &[String]) -> Result<(), String> {
        if *self.da_chuan_bi.lock().unwrap() {
            return Ok(());
        }
        let a = agent();
        let auth = format!("Bearer {tk}");
        let v = tra_loi(a.get(&format!("{}/{}?fields=sheets.properties.title", self.url_sheets, self.sheet_id)).header("Authorization", &auth).call())?;
        let co_tab = v["sheets"].as_array().is_some_and(|ds| ds.iter().any(|s| s["properties"]["title"].as_str() == Some(self.tab.as_str())));
        if !co_tab {
            let yc = json!({ "requests": [{ "addSheet": { "properties": { "title": self.tab } } }] });
            tra_loi(a.post(&format!("{}/{}:batchUpdate", self.url_sheets, self.sheet_id)).header("Authorization", &auth).header("Content-Type", "application/json").send(yc.to_string()))?;
        }
        let v = tra_loi(a.get(&format!("{}/{}/values/{}", self.url_sheets, self.sheet_id, self.pham_vi("1:1"))).header("Authorization", &auth).call())?;
        if v["values"].as_array().is_none_or(|d| d.is_empty()) {
            self.them_sa(tk, &[tieu_de.to_vec()])?;
        }
        *self.da_chuan_bi.lock().unwrap() = true;
        Ok(())
    }

    fn them_sa(&self, tk: &str, dong: &[Vec<String>]) -> Result<(), String> {
        let url = format!("{}/{}/values/{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS", self.url_sheets, self.sheet_id, self.pham_vi("A1"));
        tra_loi(agent().post(&url).header("Authorization", &format!("Bearer {tk}")).header("Content-Type", "application/json").send(json!({ "values": dong }).to_string())).map(|_| ())
    }

    fn apps_script(&self, url: &str, tieu_de: &[String], dong: &[Vec<String>], khoa: Option<&str>) -> Result<(), String> {
        let yc = json!({ "tab": self.tab, "tieu_de": tieu_de, "dong": dong, "khoa": khoa });
        let v = tra_loi(agent().post(url.trim()).header("Content-Type", "application/json").send(yc.to_string()))?;
        if v["ok"] == true {
            Ok(())
        } else {
            Err(v["loi"].as_str().map_or_else(|| "Apps Script không trả {\"ok\": true} — kiểm tra script và quyền triển khai Web app".into(), |l| format!("Apps Script báo lỗi: {l}")))
        }
    }

    /// Thêm các dòng vào cuối tab.
    pub fn them(&self, tieu_de: &[String], dong: &[Vec<String>]) -> Result<(), String> {
        match &self.cach {
            CachGhi::ServiceAccount { email, khoa_pem } => {
                let tk = self.token(email, khoa_pem)?;
                self.chuan_bi(&tk, tieu_de)?;
                self.them_sa(&tk, dong)
            }
            CachGhi::AppsScript { url } => self.apps_script(url, tieu_de, dong, None),
        }
    }

    /// Ghi đè dòng có cột A = `ten_file` (sửa tay); chưa có thì thêm.
    pub fn ghi_de(&self, tieu_de: &[String], ten_file: &str, dong: &[String]) -> Result<(), String> {
        match &self.cach {
            CachGhi::ServiceAccount { email, khoa_pem } => {
                let tk = self.token(email, khoa_pem)?;
                self.chuan_bi(&tk, tieu_de)?;
                let auth = format!("Bearer {tk}");
                let v = tra_loi(agent().get(&format!("{}/{}/values/{}", self.url_sheets, self.sheet_id, self.pham_vi("A:A"))).header("Authorization", &auth).call())?;
                let hang = v["values"].as_array().and_then(|ds| ds.iter().rposition(|d| d[0].as_str() == Some(ten_file)));
                match hang {
                    Some(h) => {
                        let url = format!("{}/{}/values/{}?valueInputOption=RAW", self.url_sheets, self.sheet_id, self.pham_vi(&format!("A{}", h + 1)));
                        tra_loi(agent().put(&url).header("Authorization", &auth).header("Content-Type", "application/json").send(json!({ "values": [dong] }).to_string())).map(|_| ())
                    }
                    None => self.them_sa(&tk, &[dong.to_vec()]),
                }
            }
            CachGhi::AppsScript { url } => self.apps_script(url, tieu_de, &[dong.to_vec()], Some(ten_file)),
        }
    }
}

/// Trạng thái hàng đợi gửi Sheet (hiện ở popup xong / bảng đáy).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrangThaiGui {
    pub da_gui: usize,
    pub cho: usize,
    pub loi: Option<String>,
}

enum Viec {
    Them(Vec<String>),
    GhiDe(String, Vec<String>),
}

/// Hàng đợi gửi Sheet trên luồng nền: gom dòng gửi 1 lần; lỗi → giữ lại, thử lại sau 5 → 60 s.
pub struct HangDoi {
    gui: Sender<Viec>,
    pub tt: Arc<Mutex<TrangThaiGui>>,
}

impl HangDoi {
    pub fn moi(dich: DichSheet, tieu_de: Vec<String>) -> Self {
        let (gui, nhan) = mpsc::channel();
        let tt = Arc::new(Mutex::new(TrangThaiGui::default()));
        let tt2 = tt.clone();
        std::thread::spawn(move || luong_gui(dich, tieu_de, nhan, tt2));
        Self { gui, tt }
    }

    pub fn them(&self, dong: Vec<String>) {
        self.tt.lock().unwrap().cho += 1;
        let _ = self.gui.send(Viec::Them(dong));
    }

    pub fn ghi_de(&self, ten_file: String, dong: Vec<String>) {
        self.tt.lock().unwrap().cho += 1;
        let _ = self.gui.send(Viec::GhiDe(ten_file, dong));
    }

    pub fn trang_thai(&self) -> TrangThaiGui {
        self.tt.lock().unwrap().clone()
    }
}

fn luong_gui(dich: DichSheet, tieu_de: Vec<String>, nhan: Receiver<Viec>, tt: Arc<Mutex<TrangThaiGui>>) {
    let mut cho: Vec<Viec> = Vec::new();
    let mut doi = Duration::from_millis(300);
    let mut lan_loi = 0u32;
    loop {
        match nhan.recv_timeout(doi) {
            Ok(v) => {
                cho.push(v);
                while let Ok(v) = nhan.try_recv() {
                    cho.push(v);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) if cho.is_empty() => return,
            Err(RecvTimeoutError::Disconnected) => {}
        }
        if cho.is_empty() {
            continue;
        }
        // Gom các dòng Thêm liền nhau thành 1 yêu cầu; Ghi đè gửi riêng
        let mut kq = Ok(0);
        while !cho.is_empty() && kq.is_ok() {
            let so_them = cho.iter().take_while(|v| matches!(v, Viec::Them(_))).count();
            kq = if so_them > 0 {
                let dong: Vec<Vec<String>> = cho[..so_them].iter().map(|v| if let Viec::Them(d) = v { d.clone() } else { vec![] }).collect();
                dich.them(&tieu_de, &dong).map(|_| so_them)
            } else if let Viec::GhiDe(t, d) = &cho[0] {
                dich.ghi_de(&tieu_de, t, d).map(|_| 1)
            } else {
                Ok(0)
            };
            if let Ok(n) = kq {
                cho.drain(..n);
                let mut t = tt.lock().unwrap();
                t.da_gui += n;
                t.cho = t.cho.saturating_sub(n);
                t.loi = None;
            }
        }
        match kq {
            Ok(_) => {
                lan_loi = 0;
                doi = Duration::from_millis(300);
            }
            Err(e) => {
                tt.lock().unwrap().loi = Some(e);
                lan_loi += 1;
                doi = Duration::from_secs((5u64 << lan_loi.min(4)).min(60));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boc_tach::mo_hinh::tests::may_chu_gia;

    fn khoa_thu() -> (String, rsa::RsaPublicKey) {
        use rsa::pkcs8::{EncodePrivateKey, LineEnding};
        let k = rsa::RsaPrivateKey::new(&mut rand_core::OsRng, 1024).unwrap();
        (k.to_pkcs8_pem(LineEnding::LF).unwrap().to_string(), k.to_public_key())
    }

    #[test]
    fn id_sheet_tu_link() {
        assert_eq!(id_sheet("https://docs.google.com/spreadsheets/d/1AbCdEfGhIjKlMnOpQrStUvWxYz_-123/edit#gid=0").as_deref(), Some("1AbCdEfGhIjKlMnOpQrStUvWxYz_-123"));
        assert_eq!(id_sheet("1AbCdEfGhIjKlMnOpQrStUvWxYz_-123").as_deref(), Some("1AbCdEfGhIjKlMnOpQrStUvWxYz_-123"));
        assert_eq!(id_sheet("https://example.com/x"), None);
        assert_eq!(ma_url("'Bóc tách'!A1"), "%27B%C3%B3c%20t%C3%A1ch%27%21A1");
    }

    #[test]
    fn jwt_ky_dung_khoa() {
        use rsa::signature::Verifier;
        let (pem, cong_khai) = khoa_thu();
        let j = jwt("dema4@du-an.iam.gserviceaccount.com", &pem, URL_TOKEN, 1_800_000_000).unwrap();
        let phan: Vec<&str> = j.split('.').collect();
        assert_eq!(phan.len(), 3);
        let than: Value = serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(phan[1]).unwrap()).unwrap();
        assert_eq!((than["iss"].as_str(), than["exp"].as_u64()), (Some("dema4@du-an.iam.gserviceaccount.com"), Some(1_800_003_600)));
        let ky = rsa::pkcs1v15::Signature::try_from(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(phan[2]).unwrap().as_slice()).unwrap();
        rsa::pkcs1v15::VerifyingKey::<rsa::sha2::Sha256>::new(cong_khai).verify(format!("{}.{}", phan[0], phan[1]).as_bytes(), &ky).unwrap();
        assert!(jwt("x", "không phải khoá", URL_TOKEN, 0).is_err());
    }

    #[test]
    fn service_account_tao_tab_tieu_de_them_va_ghi_de() {
        let (pem, _) = khoa_thu();
        let ok = |v: Value| (200, vec![], v.to_string());
        let (dc, nhan) = may_chu_gia(vec![
            ok(json!({ "access_token": "tk-1", "expires_in": 3600 })),
            ok(json!({ "sheets": [{ "properties": { "title": "Sheet1" } }] })),
            ok(json!({})),
            ok(json!({ "range": "'Boc_tach'!A1:Z1" })),
            ok(json!({})),
            ok(json!({})),
            ok(json!({ "values": [["Tên file"], ["a.pdf"], ["b.pdf"]] })),
            ok(json!({})),
        ]);
        let d = DichSheet::moi(CachGhi::ServiceAccount { email: "e@x.iam.gserviceaccount.com".into(), khoa_pem: pem }, "https://docs.google.com/spreadsheets/d/1AbCdEfGhIjKlMnOpQrStUvWxYz/edit", "Boc_tach").unwrap().dia_chi_thu(dc.trim_end_matches("/v1"));
        let td: Vec<String> = ["Tên file", "Số ký hiệu"].map(String::from).to_vec();
        d.them(&td, &[vec!["a.pdf".into(), "1/QĐ".into()], vec!["b.pdf".into(), "2/QĐ".into()]]).unwrap();
        d.ghi_de(&td, "b.pdf", &["b.pdf".into(), "2/QĐ-UBND".into()]).unwrap();
        let yc: Vec<String> = nhan.try_iter().collect();
        assert!(yc[0].starts_with("POST /token") && yc[0].contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=ey"));
        assert!(yc[1].starts_with("GET /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz?fields=") && yc[1].contains("Bearer tk-1"));
        assert!(yc[2].starts_with("POST /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz:batchUpdate") && yc[2].contains("addSheet"), "chưa có tab → tạo");
        assert!(yc[4].contains(":append") && yc[4].contains("Số ký hiệu"), "tab trống → ghi tiêu đề");
        assert!(yc[5].contains(":append") && yc[5].contains("1/QĐ") && yc[5].contains("2/QĐ"), "gom 2 dòng 1 yêu cầu");
        assert!(yc[6].starts_with("GET /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz/values/%27Boc_tach%27%21A%3AA"));
        assert!(yc[7].starts_with("PUT /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz/values/%27Boc_tach%27%21A3?") && yc[7].contains("2/QĐ-UBND"), "ghi đè dòng 3 (b.pdf)");
        assert_eq!(yc.len(), 8, "token dùng lại, không chuẩn bị lại tab");
    }

    #[test]
    fn apps_script_va_hang_doi_thu_lai_khi_loi() {
        let (dc, nhan) = may_chu_gia(vec![(500, vec![], "lỗi".into()), (200, vec![], json!({ "ok": true }).to_string()), (200, vec![], json!({ "ok": false, "loi": "Sai tab" }).to_string())]);
        let d = DichSheet::moi(CachGhi::AppsScript { url: format!("{dc}/exec") }, "", "").unwrap();
        let td = vec!["Tên file".to_string()];
        assert!(d.them(&td, &[vec!["a.pdf".into()]]).unwrap_err().contains("500"));
        d.them(&td, &[vec!["a.pdf".into()]]).unwrap();
        assert_eq!(d.ghi_de(&td, "a.pdf", &["a.pdf".into()]).unwrap_err(), "Apps Script báo lỗi: Sai tab");
        let yc: Vec<String> = nhan.try_iter().collect();
        assert!(yc[1].contains("\"tab\":\"Boc_tach\"") && yc[1].contains("\"dong\":[[\"a.pdf\"]]"));
        assert!(yc[2].contains("\"khoa\":\"a.pdf\""));
        assert!(DichSheet::moi(CachGhi::AppsScript { url: "abc".into() }, "", "").is_err());
    }

    #[test]
    fn hang_doi_gom_dong_va_dem() {
        let (dc, nhan) = may_chu_gia(vec![(200, vec![], json!({ "ok": true }).to_string()), (200, vec![], json!({ "ok": true }).to_string())]);
        let h = HangDoi::moi(DichSheet::moi(CachGhi::AppsScript { url: format!("{dc}/exec") }, "", "Kq").unwrap(), vec!["Tên file".into()]);
        h.them(vec!["a.pdf".into()]);
        h.them(vec!["b.pdf".into()]);
        for _ in 0..100 {
            if h.trang_thai().da_gui == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(h.trang_thai(), TrangThaiGui { da_gui: 2, cho: 0, loi: None });
        assert!(nhan.try_iter().count() <= 2);
    }
}
