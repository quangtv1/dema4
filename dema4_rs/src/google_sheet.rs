//! Ghi thống kê lên Google Sheet bằng service account (khoá nhúng lúc build, xem build.rs — không có trong repo).
//! Dùng chung: `ghi_dong(tab, tiêu_đề, dòng)` thêm 1 hàng vào tab; tab chưa có thì tạo + ghi hàng tiêu đề.
//!
//! Lưu ý: ai có file exe cũng trích được khoá — Sheet phải chia sẻ cho email service account (quyền Chỉnh sửa).

use base64::Engine;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Sheet "Monitor_dema4" nhận thống kê.
pub const SPREADSHEET_ID: &str = "10IWldAIS8ZPjizNjoomzFw4xd3Tk6J8WE2dscIp_bSQ";
const KHOA_NHUNG: &str = include_str!(concat!(env!("OUT_DIR"), "/sheet_key.json"));
const PHAM_VI: &str = "https://www.googleapis.com/auth/spreadsheets";
const API: &str = "https://sheets.googleapis.com/v4/spreadsheets";

struct KhoaDichVu {
    email: String,
    khoa: rsa::RsaPrivateKey,
    token_uri: String,
}

/// Token truy cập còn hạn (dùng lại ~55 phút cho các lần ghi sau).
static TOKEN: Mutex<Option<(String, u64)>> = Mutex::new(None);

fn bay_gio() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn doc_khoa(json_khoa: &str) -> Result<KhoaDichVu, String> {
    let v: Value = serde_json::from_str(json_khoa).map_err(|e| format!("khoá Sheet hỏng: {e}"))?;
    let lay = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string).ok_or(format!("khoá Sheet thiếu {k}"));
    let khoa = rsa::RsaPrivateKey::from_pkcs8_pem(&lay("private_key")?).map_err(|e| format!("private_key không đọc được: {e}"))?;
    Ok(KhoaDichVu { email: lay("client_email")?, khoa, token_uri: lay("token_uri").unwrap_or_else(|_| "https://oauth2.googleapis.com/token".into()) })
}

/// Có khoá nhúng (bản build có khoá) → ghi Sheet được.
pub fn co_khoa() -> bool {
    !KHOA_NHUNG.trim().is_empty()
}

/// JWT RS256 xin token theo chuẩn OAuth "service account".
fn jwt(k: &KhoaDichVu, luc: u64) -> String {
    let b64 = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
    let dau = b64(br#"{"alg":"RS256","typ":"JWT"}"#);
    let than = b64(json!({ "iss": k.email, "scope": PHAM_VI, "aud": k.token_uri, "iat": luc, "exp": luc + 3600 }).to_string().as_bytes());
    let noi_dung = format!("{dau}.{than}");
    let chu_ky = SigningKey::<rsa::sha2::Sha256>::new(k.khoa.clone()).sign(noi_dung.as_bytes());
    format!("{noi_dung}.{}", b64(&chu_ky.to_bytes()))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).http_status_as_error(false).build().into()
}

fn doc_json(ph: &mut ureq::http::Response<ureq::Body>) -> (u16, Value) {
    let ma = ph.status().as_u16();
    let than = ph.body_mut().read_to_string().unwrap_or_default();
    (ma, serde_json::from_str(&than).unwrap_or(Value::String(than)))
}

fn token() -> Result<String, String> {
    if let Some((t, het)) = TOKEN.lock().unwrap().clone() {
        if bay_gio() + 60 < het {
            return Ok(t);
        }
    }
    let k = doc_khoa(KHOA_NHUNG)?;
    let luc = bay_gio();
    let mut ph = agent()
        .post(&k.token_uri)
        .send_form([("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"), ("assertion", jwt(&k, luc).as_str())])
        .map_err(|e| format!("không kết nối được Google ({e})"))?;
    let (ma, v) = doc_json(&mut ph);
    let t = v.get("access_token").and_then(Value::as_str).ok_or(format!("Google từ chối khoá ({ma}): {v}"))?.to_string();
    let het = luc + v.get("expires_in").and_then(Value::as_u64).unwrap_or(3600);
    *TOKEN.lock().unwrap() = Some((t.clone(), het));
    Ok(t)
}

fn them_hang(t: &str, tab: &str, hang: &[Vec<Value>]) -> Result<(u16, Value), String> {
    let pham_vi = format!("'{}'!A1", tab.replace('\'', "''"));
    let url = format!("{API}/{SPREADSHEET_ID}/values/{}:append?valueInputOption=USER_ENTERED&insertDataOption=INSERT_ROWS", ma_hoa_url(&pham_vi));
    let mut ph = agent().post(&url).header("Authorization", format!("Bearer {t}")).header("Content-Type", "application/json").send(json!({ "values": hang }).to_string()).map_err(|e| e.to_string())?;
    Ok(doc_json(&mut ph))
}

fn tao_tab(t: &str, tab: &str) -> Result<(), String> {
    let mut ph = agent()
        .post(format!("{API}/{SPREADSHEET_ID}:batchUpdate"))
        .header("Authorization", format!("Bearer {t}"))
        .header("Content-Type", "application/json")
        .send(json!({ "requests": [{ "addSheet": { "properties": { "title": tab } } }] }).to_string())
        .map_err(|e| e.to_string())?;
    match doc_json(&mut ph) {
        (200, _) => Ok(()),
        (ma, v) => Err(format!("không tạo được tab {tab} ({ma}): {v}")),
    }
}

fn ma_hoa_url(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Thêm 1 hàng vào tab `tab`; tab chưa có → tạo tab, ghi hàng `tieu_de` trước. Gọi từ luồng nền.
pub fn ghi_dong(tab: &str, tieu_de: &[&str], dong: Vec<Value>) -> Result<(), String> {
    if !co_khoa() {
        return Err("bản build không có khoá Sheet".into());
    }
    let t = token()?;
    match them_hang(&t, tab, std::slice::from_ref(&dong))? {
        (200, _) => Ok(()),
        // Tab chưa có: Google trả 400 "Unable to parse range"
        (400, v) if v.to_string().contains("parse range") => {
            tao_tab(&t, tab)?;
            let dau: Vec<Value> = tieu_de.iter().map(|s| Value::from(*s)).collect();
            match them_hang(&t, tab, &[dau, dong])? {
                (200, _) => Ok(()),
                (ma, v) => Err(format!("Sheet trả lỗi {ma}: {v}")),
            }
        }
        (ma, v) => Err(format!("Sheet trả lỗi {ma}: {v}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_ky_dung_va_xac_minh_duoc() {
        use rsa::pkcs1v15::VerifyingKey;
        use rsa::signature::Verifier;
        let khoa = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 1024).unwrap();
        let k = KhoaDichVu { email: "a@b.iam.gserviceaccount.com".into(), khoa: khoa.clone(), token_uri: "https://oauth2.googleapis.com/token".into() };
        let j = jwt(&k, 1_000);
        let phan: Vec<&str> = j.split('.').collect();
        assert_eq!(phan.len(), 3);
        let giai = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).unwrap();
        let than: Value = serde_json::from_slice(&giai(phan[1])).unwrap();
        assert_eq!((than["iss"].as_str(), than["exp"].as_u64(), than["scope"].as_str()), (Some("a@b.iam.gserviceaccount.com"), Some(4_600), Some(PHAM_VI)));
        let chu_ky = rsa::pkcs1v15::Signature::try_from(giai(phan[2]).as_slice()).unwrap();
        VerifyingKey::<rsa::sha2::Sha256>::new(khoa.to_public_key()).verify(format!("{}.{}", phan[0], phan[1]).as_bytes(), &chu_ky).unwrap();
    }

    #[test]
    fn doc_khoa_bao_loi_ro_rang() {
        assert!(doc_khoa("{").err().unwrap().contains("hỏng"));
        assert!(doc_khoa(r#"{"client_email":"x"}"#).err().unwrap().contains("private_key"));
        assert_eq!(ma_hoa_url("'nguong'!A1"), "%27nguong%27%21A1");
    }

    /// Ghi thật 1 hàng vào tab "kiem_tra" của Sheet (cần khoá + Sheet đã chia sẻ cho service account).
    #[test]
    #[ignore = "ghi thật lên Google Sheet"]
    fn ghi_that_len_sheet() {
        let luc = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        ghi_dong("kiem_tra", &["thoi_diem", "noi_dung"], vec![luc.into(), "sm-dema4 ghi thử".into()]).unwrap();
    }
}
