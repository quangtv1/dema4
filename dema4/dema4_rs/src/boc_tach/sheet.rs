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
    /// `bi_mat`: mã chung với hằng `BI_MAT` trong script (rỗng = script không kiểm tra).
    AppsScript { url: String, bi_mat: String },
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
            CachGhi::AppsScript { url, .. } if !url.trim().starts_with("https://") && !url.trim().starts_with("http://") => return Err("URL Web app không đúng".into()),
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
        let v = tra_loi(a.get(&format!("{}/{}?fields=sheets.properties(title,sheetId)", self.url_sheets, self.sheet_id)).header("Authorization", &auth).call())?;
        let mut sheet_id = v["sheets"].as_array().and_then(|ds| ds.iter().find(|s| s["properties"]["title"].as_str() == Some(self.tab.as_str()))).and_then(|s| s["properties"]["sheetId"].as_i64());
        if sheet_id.is_none() {
            let yc = json!({ "requests": [{ "addSheet": { "properties": { "title": self.tab } } }] });
            let v = tra_loi(a.post(&format!("{}/{}:batchUpdate", self.url_sheets, self.sheet_id)).header("Authorization", &auth).header("Content-Type", "application/json").send(yc.to_string()))?;
            sheet_id = v["replies"][0]["addSheet"]["properties"]["sheetId"].as_i64();
        }
        let v = tra_loi(a.get(&format!("{}/{}/values/{}", self.url_sheets, self.sheet_id, self.pham_vi("1:1"))).header("Authorization", &auth).call())?;
        if v["values"].as_array().is_none_or(|d| d.is_empty()) {
            self.them_sa(tk, &[tieu_de.to_vec()])?;
            // Tab mới (chưa có tiêu đề): định dạng 1 lần — không đè cách trình bày người dùng đã sửa ở tab cũ
            if let Some(id) = sheet_id {
                let yc = json!({ "requests": dinh_dang_tab(id, tieu_de) });
                tra_loi(a.post(&format!("{}/{}:batchUpdate", self.url_sheets, self.sheet_id)).header("Authorization", &auth).header("Content-Type", "application/json").send(yc.to_string()))?;
            }
        }
        *self.da_chuan_bi.lock().unwrap() = true;
        Ok(())
    }

}

/// Yêu cầu định dạng tab (giống file Excel): mọi ô xuống dòng + căn giữa dọc, tiêu đề đậm nền xám cố định,
/// cột theo tên (Trích yếu rộng nhất), bộ lọc trên toàn bảng.
fn dinh_dang_tab(sheet_id: i64, tieu_de: &[String]) -> Vec<Value> {
    let n = tieu_de.len();
    let mut yc = vec![
        json!({ "repeatCell": {
            "range": { "sheetId": sheet_id, "startColumnIndex": 0, "endColumnIndex": n },
            "cell": { "userEnteredFormat": { "wrapStrategy": "WRAP", "verticalAlignment": "MIDDLE" } },
            "fields": "userEnteredFormat(wrapStrategy,verticalAlignment)" } }),
        json!({ "repeatCell": {
            "range": { "sheetId": sheet_id, "startRowIndex": 0, "endRowIndex": 1, "startColumnIndex": 0, "endColumnIndex": n },
            "cell": { "userEnteredFormat": { "textFormat": { "bold": true }, "backgroundColor": { "red": 0.953, "green": 0.953, "blue": 0.953 } } },
            "fields": "userEnteredFormat(textFormat,backgroundColor)" } }),
        json!({ "updateSheetProperties": { "properties": { "sheetId": sheet_id, "gridProperties": { "frozenRowCount": 1 } }, "fields": "gridProperties.frozenRowCount" } }),
        json!({ "setBasicFilter": { "filter": { "range": { "sheetId": sheet_id, "startRowIndex": 0, "startColumnIndex": 0, "endColumnIndex": n } } } }),
    ];
    for (c, t) in tieu_de.iter().enumerate() {
        yc.push(json!({ "updateDimensionProperties": {
            "range": { "sheetId": sheet_id, "dimension": "COLUMNS", "startIndex": c, "endIndex": c + 1 },
            "properties": { "pixelSize": super::excel::rong_cot(t).1 }, "fields": "pixelSize" } }));
    }
    yc
}

impl DichSheet {
    fn them_sa(&self, tk: &str, dong: &[Vec<String>]) -> Result<(), String> {
        let url = format!("{}/{}/values/{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS", self.url_sheets, self.sheet_id, self.pham_vi("A1"));
        tra_loi(agent().post(&url).header("Authorization", &format!("Bearer {tk}")).header("Content-Type", "application/json").send(json!({ "values": dong }).to_string())).map(|_| ())
    }

    fn apps_script(&self, url: &str, bi_mat: &str, tieu_de: &[String], dong: &[Vec<String>], ghi_lai: bool) -> Result<(), String> {
        // ghi_lai: script xoá dữ liệu dưới dòng tiêu đề rồi ghi cả bảng; không thì thêm dòng cuối tab
        let yc = json!({ "tab": self.tab, "tieu_de": tieu_de, "dong": dong, "ghi_lai": ghi_lai, "bi_mat": bi_mat });
        let v = tra_loi(agent().post(url.trim()).header("Content-Type", "application/json").send(yc.to_string()))?;
        if v["ok"] == true {
            Ok(())
        } else {
            Err(v["loi"].as_str().map_or_else(|| "Apps Script không trả {\"ok\": true} — kiểm tra script và quyền triển khai Web app".into(), |l| format!("Apps Script báo lỗi: {l}")))
        }
    }

    /// Thêm dòng cuối tab (chỉ dùng cho "Ghi thử 1 dòng" — không đụng dữ liệu đang có).
    pub fn them(&self, tieu_de: &[String], dong: &[Vec<String>]) -> Result<(), String> {
        match &self.cach {
            CachGhi::ServiceAccount { email, khoa_pem } => {
                let tk = self.token(email, khoa_pem)?;
                self.chuan_bi(&tk, tieu_de)?;
                self.them_sa(&tk, dong)
            }
            CachGhi::AppsScript { url, bi_mat } => self.apps_script(url, bi_mat, tieu_de, dong, false),
        }
    }

    /// Ghi lại toàn bộ tab = bản sao kết quả (giống file Excel): ghi cả bảng từ A1 rồi xoá phần thừa phía dưới.
    /// Service account: 2 lần gọi mỗi lần ghi, dù đổi 1 dòng hay 1.000 dòng; định dạng / bộ lọc của tab giữ nguyên.
    pub fn ghi_lai(&self, tieu_de: &[String], dong: &[Vec<String>]) -> Result<(), String> {
        match &self.cach {
            CachGhi::ServiceAccount { email, khoa_pem } => {
                let tk = self.token(email, khoa_pem)?;
                self.chuan_bi(&tk, tieu_de)?;
                let auth = format!("Bearer {tk}");
                let mut bang = vec![tieu_de.to_vec()];
                bang.extend(dong.iter().cloned());
                let url = format!("{}/{}/values/{}?valueInputOption=RAW", self.url_sheets, self.sheet_id, self.pham_vi("A1"));
                tra_loi(agent().put(&url).header("Authorization", &auth).header("Content-Type", "application/json").send(json!({ "values": bang }).to_string()))?;
                // Ghi trước, xoá sau: người đang xem Sheet không thấy tab trống giữa chừng
                let thua = self.pham_vi(&format!("A{}:ZZ", bang.len() + 1));
                tra_loi(agent().post(&format!("{}/{}/values/{thua}:clear", self.url_sheets, self.sheet_id)).header("Authorization", &auth).header("Content-Type", "application/json").send("{}"))?;
                Ok(())
            }
            CachGhi::AppsScript { url, bi_mat } => self.apps_script(url, bi_mat, tieu_de, dong, true),
        }
    }
}

/// Trạng thái đồng bộ Sheet (bảng đáy, popup xong).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrangThaiGui {
    /// Số dòng của lần ghi gần nhất.
    pub so_dong: usize,
    pub lan_ghi: Option<std::time::SystemTime>,
    /// Có thay đổi chưa ghi (sẽ ghi trong ≤ 1 phút).
    pub con_doi: bool,
    pub loi: Option<String>,
}

/// 2 lần ghi lại toàn bộ cách nhau ít nhất chừng này: dữ liệu đổi liên tục cũng chỉ ghi 1 lần / phút (Sheet không khoá).
pub const KHOANG_GHI: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Chung {
    thay_doi: bool,
    xin_bang: bool,
    bang: Option<(Vec<String>, Vec<Vec<String>>)>,
    tt: TrangThaiGui,
}

/// Đồng bộ chạy nền theo cờ: giao diện chỉ `danh_dau()` khi dữ liệu đổi (không gọi API); luồng nền thấy cờ và
/// đã qua `khoang` kể từ lần ghi trước thì xin bảng (giao diện nộp bằng `nop` — dựng bảng tối đa 1 lần / phút) rồi gọi
/// `ghi` với cả bảng (Sheet người dùng: ghi lại cả tab; trạm: sao lưu). Lỗi → giữ cờ, thử lại sau 5 → 60 s.
pub struct DongBoSheet {
    chung: Arc<Mutex<Chung>>,
    _dung: Sender<()>,
}

impl DongBoSheet {
    pub fn moi(ghi: impl Fn(&[String], &[Vec<String>]) -> Result<(), String> + Send + 'static, ctx: Option<eframe::egui::Context>, khoang: Duration) -> Self {
        let chung = Arc::new(Mutex::new(Chung::default()));
        let (dung, nhan_dung) = mpsc::channel::<()>();
        let c2 = chung.clone();
        std::thread::spawn(move || luong_dong_bo(ghi, c2, nhan_dung, ctx, khoang));
        Self { chung, _dung: dung }
    }

    /// Dữ liệu vừa đổi (đọc xong 1 file, Lưu sửa tay, ReOCR).
    pub fn danh_dau(&self) {
        let mut c = self.chung.lock().unwrap();
        c.thay_doi = true;
        c.tt.con_doi = true;
    }

    /// Luồng nền đang chờ bảng để ghi.
    pub fn xin_bang(&self) -> bool {
        self.chung.lock().unwrap().xin_bang
    }

    pub fn nop(&self, tieu_de: Vec<String>, dong: Vec<Vec<String>>) {
        let mut c = self.chung.lock().unwrap();
        c.bang = Some((tieu_de, dong));
        c.xin_bang = false;
    }

    pub fn trang_thai(&self) -> TrangThaiGui {
        self.chung.lock().unwrap().tt.clone()
    }
}

fn luong_dong_bo(ghi: impl Fn(&[String], &[Vec<String>]) -> Result<(), String>, chung: Arc<Mutex<Chung>>, dung: Receiver<()>, ctx: Option<eframe::egui::Context>, khoang: Duration) {
    let mut lan_ghi: Option<Instant> = None;
    let mut thu_lai_luc: Option<Instant> = None;
    let mut lan_loi = 0u32;
    loop {
        // Kênh đóng (giao diện bỏ đồng bộ này) → ghi nốt nếu đang có bảng rồi thôi
        let het = matches!(dung.recv_timeout(Duration::from_millis(250)), Err(RecvTimeoutError::Disconnected));
        let bay_gio = Instant::now();
        let bang = {
            let mut c = chung.lock().unwrap();
            let den_luc = lan_ghi.is_none_or(|t| bay_gio - t >= khoang) && thu_lai_luc.is_none_or(|t| bay_gio >= t);
            if c.thay_doi && den_luc && c.bang.is_none() && !c.xin_bang {
                c.thay_doi = false;
                c.xin_bang = true;
                // Đánh thức giao diện để nộp bảng (giao diện ngủ khi không có thao tác)
                if let Some(ctx) = &ctx {
                    ctx.request_repaint();
                }
            }
            c.bang.take()
        };
        if let Some((td, dong)) = bang {
            lan_ghi = Some(Instant::now());
            let kq = ghi(&td, &dong);
            let mut c = chung.lock().unwrap();
            match kq {
                Ok(()) => {
                    lan_loi = 0;
                    thu_lai_luc = None;
                    c.tt.so_dong = dong.len();
                    c.tt.lan_ghi = Some(std::time::SystemTime::now());
                    c.tt.loi = None;
                    c.tt.con_doi = c.thay_doi;
                }
                Err(e) => {
                    c.thay_doi = true;
                    c.tt.loi = Some(e);
                    lan_loi += 1;
                    thu_lai_luc = Some(Instant::now() + Duration::from_secs((5u64 << lan_loi.min(4)).min(60)));
                }
            }
        } else if het {
            return;
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
    fn service_account_ghi_lai_toan_bo_tab() {
        let (pem, _) = khoa_thu();
        let ok = |v: Value| (200, vec![], v.to_string());
        let (dc, nhan) = may_chu_gia(vec![
            ok(json!({ "access_token": "tk-1", "expires_in": 3600 })),
            ok(json!({ "sheets": [{ "properties": { "title": "Sheet1", "sheetId": 0 } }] })),
            ok(json!({ "replies": [{ "addSheet": { "properties": { "sheetId": 777, "title": "Van_ban" } } }] })),
            ok(json!({ "range": "'Van_ban'!A1:Z1" })),
            ok(json!({})),
            ok(json!({})),
            ok(json!({})),
            ok(json!({})),
            ok(json!({})),
            ok(json!({})),
        ]);
        let d = DichSheet::moi(CachGhi::ServiceAccount { email: "e@x.iam.gserviceaccount.com".into(), khoa_pem: pem }, "https://docs.google.com/spreadsheets/d/1AbCdEfGhIjKlMnOpQrStUvWxYz/edit", "Van_ban").unwrap().dia_chi_thu(dc.trim_end_matches("/v1"));
        let td: Vec<String> = ["Tên file", "Số ký hiệu"].map(String::from).to_vec();
        d.ghi_lai(&td, &[vec!["a.pdf".into(), "1/QĐ".into()], vec!["b.pdf".into(), "2/QĐ".into()]]).unwrap();
        d.ghi_lai(&td, &[vec!["a.pdf".into(), "1/QĐ".into()], vec!["b.pdf".into(), "2/QĐ-UBND".into()]]).unwrap();
        let yc: Vec<String> = nhan.try_iter().collect();
        assert!(yc[0].starts_with("POST /token") && yc[0].contains("assertion=ey"));
        assert!(yc[2].contains(":batchUpdate") && yc[2].contains("addSheet"), "chưa có tab → tạo");
        assert!(yc[5].contains(":batchUpdate") && yc[5].contains("\"sheetId\":777") && yc[5].contains("WRAP") && yc[5].contains("MIDDLE") && yc[5].contains("setBasicFilter"), "tab mới → định dạng");
        assert!(yc[6].starts_with("PUT /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz/values/%27Van_ban%27%21A1?valueInputOption=RAW") && yc[6].contains("[[\"Tên file\",\"Số ký hiệu\"],[\"a.pdf\""), "ghi cả bảng từ A1");
        assert!(yc[7].starts_with("POST /sheets/1AbCdEfGhIjKlMnOpQrStUvWxYz/values/%27Van_ban%27%21A4%3AZZ:clear"), "xoá phần thừa dưới bảng");
        assert!(yc[8].starts_with("PUT ") && yc[8].contains("2/QĐ-UBND") && yc[9].contains(":clear"), "lần sau: lại 2 lần gọi, không chuẩn bị lại tab");
        assert_eq!(yc.len(), 10);
    }

    #[test]
    fn apps_script_ghi_lai_va_them() {
        let (dc, nhan) = may_chu_gia(vec![(500, vec![], "lỗi".into()), (200, vec![], json!({ "ok": true }).to_string()), (200, vec![], json!({ "ok": false, "loi": "Sai tab" }).to_string())]);
        let d = DichSheet::moi(CachGhi::AppsScript { url: format!("{dc}/exec"), bi_mat: "bm".into() }, "", "").unwrap();
        let td = vec!["Tên file".to_string()];
        assert!(d.ghi_lai(&td, &[vec!["a.pdf".into()]]).unwrap_err().contains("500"));
        d.ghi_lai(&td, &[vec!["a.pdf".into()]]).unwrap();
        assert_eq!(d.them(&td, &[vec!["thu".into()]]).unwrap_err(), "Apps Script báo lỗi: Sai tab");
        let yc: Vec<String> = nhan.try_iter().collect();
        assert!(yc[1].contains("\"ghi_lai\":true") && yc[1].contains("\"bi_mat\":\"bm\"") && yc[1].contains("\"tab\":\"Boc_tach\""));
        assert!(yc[2].contains("\"ghi_lai\":false"), "Ghi thử: chỉ thêm dòng");
        assert!(DichSheet::moi(CachGhi::AppsScript { url: "abc".into(), bi_mat: String::new() }, "", "").is_err());
    }

    /// Ghi thật lên Google Sheet: DEMA4_SA_KEY=<file khoá .json> DEMA4_SHEET=<link> [DEMA4_TAB=<tab>].
    #[test]
    #[ignore]
    fn ghi_that_len_google_sheet() {
        let (Ok(khoa), Ok(link)) = (std::env::var("DEMA4_SA_KEY"), std::env::var("DEMA4_SHEET")) else { return };
        let (email, khoa_pem) = doc_khoa_sa(&khoa).unwrap();
        let tab = std::env::var("DEMA4_TAB").unwrap_or_else(|_| "Boc_tach_thu".into());
        let d = DichSheet::moi(CachGhi::ServiceAccount { email: email.clone(), khoa_pem: khoa_pem.clone() }, &link, &tab).unwrap();
        let td: Vec<String> = ["Tên file", "Hồ sơ", "Số trang", "Số, ký hiệu", "Ngày tháng", "Trích yếu"].map(String::from).to_vec();
        let dong = |n: usize, hau: &str| (1..=n).map(|i| vec![format!("VB{i:02}.pdf"), "HS-01".into(), "2".into(), format!("{i}/QĐ-UBND{hau}"), "01/02/2015".into(), format!("Trích yếu văn bản {i} rất dài để kiểm tra xuống dòng và căn giữa theo chiều dọc của ô")]).collect::<Vec<_>>();
        d.ghi_lai(&td, &dong(5, "")).unwrap();
        d.ghi_lai(&td, &dong(3, "-sua")).unwrap();
        let tk = d.token(&email, &khoa_pem).unwrap();
        let v = tra_loi(agent().get(&format!("{}/{}/values/{}", d.url_sheets, d.sheet_id, d.pham_vi("A1:Z"))).header("Authorization", &format!("Bearer {tk}")).call()).unwrap();
        let o = v["values"].as_array().unwrap();
        assert_eq!(o.len(), 4, "tiêu đề + 3 dòng, 2 dòng thừa đã xoá: {o:?}");
        assert_eq!(o[3][3], "3/QĐ-UBND-sua");
        assert_eq!(o[1][4], "01/02/2015", "ngày giữ dạng chữ");
        let p = tra_loi(agent().get(&format!("{}/{}?fields=sheets(properties(title,gridProperties.frozenRowCount),basicFilter.range,data.columnMetadata.pixelSize)", d.url_sheets, d.sheet_id)).header("Authorization", &format!("Bearer {tk}")).call()).unwrap();
        let t = p["sheets"].as_array().unwrap().iter().find(|t| t["properties"]["title"] == tab.as_str()).unwrap();
        assert_eq!(t["properties"]["gridProperties"]["frozenRowCount"], 1);
        assert!(t["basicFilter"]["range"].is_object(), "có bộ lọc");
        assert_eq!(t["data"][0]["columnMetadata"][5]["pixelSize"], 480, "Trích yếu rộng nhất");
    }

    #[test]
    fn dong_bo_theo_co_toi_da_1_lan_moi_khoang() {
        let ok = json!({ "ok": true }).to_string();
        let (dc, nhan) = may_chu_gia(vec![(200, vec![], ok.clone()), (200, vec![], ok)]);
        let dich = DichSheet::moi(CachGhi::AppsScript { url: format!("{dc}/exec"), bi_mat: String::new() }, "", "Kq").unwrap();
        let ds = DongBoSheet::moi(move |td, d| dich.ghi_lai(td, d), None, Duration::from_millis(1500));
        let nop = |ds: &DongBoSheet, n: usize| {
            for _ in 0..100 {
                if ds.xin_bang() {
                    ds.nop(vec!["Tên file".into()], (0..n).map(|i| vec![format!("{i}.pdf")]).collect());
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        std::thread::sleep(Duration::from_millis(400));
        assert!(!ds.xin_bang(), "chưa có thay đổi → không xin bảng, không gọi API");
        ds.danh_dau();
        assert!(nop(&ds, 2), "thay đổi đầu tiên → ghi ngay");
        for _ in 0..100 {
            if ds.trang_thai().so_dong == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Đổi liên tục ngay sau đó: chưa tới khoảng → chưa ghi
        for _ in 0..5 {
            ds.danh_dau();
        }
        std::thread::sleep(Duration::from_millis(500));
        assert!(!ds.xin_bang() && ds.trang_thai().con_doi, "trong khoảng chờ: chỉ giữ cờ");
        assert!(nop(&ds, 3), "hết khoảng → ghi lại 1 lần với bảng mới nhất");
        for _ in 0..100 {
            if ds.trang_thai().so_dong == 3 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let yc: Vec<String> = nhan.try_iter().collect();
        assert_eq!(yc.len(), 2, "5 lần đánh dấu liên tiếp → 1 lần ghi");
        assert!(yc[1].contains("2.pdf") && !ds.trang_thai().con_doi);
    }
}
