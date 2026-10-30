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

/// Thêm 1 hàng vào tab `tab`; tab chưa có → tạo tab, ghi hàng `tieu_de` trước, rồi (nếu có `mo_ta`) tạo tab mô tả
/// ngay bên phải và ghi các hàng mô tả. Gọi từ luồng nền.
pub fn ghi_dong(tab: &str, tieu_de: &[&str], dong: Vec<Value>, mo_ta: Option<(&str, Vec<Vec<Value>>)>) -> Result<(), String> {
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
            let kq = match them_hang(&t, tab, &[dau, dong])? {
                (200, _) => Ok(()),
                (ma, v) => Err(format!("Sheet trả lỗi {ma}: {v}")),
            };
            // Tab mô tả: lỗi (vd đã có sẵn) không ảnh hưởng hàng dữ liệu vừa ghi
            if let Some((tab_mt, hang)) = mo_ta {
                if tao_tab(&t, tab_mt).is_ok() {
                    let _ = them_hang(&t, tab_mt, &hang);
                }
            }
            kq
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

    /// Tạo sẵn tab "Ngưỡng trắng" (chỉ hàng tiêu đề) + tab mô tả bên phải — không ghi hàng số liệu nào.
    #[test]
    #[ignore = "ghi thật lên Google Sheet"]
    fn tao_tab_nguong_trang() {
        use crate::tach_vb::thong_ke_nguong as tk;
        let t = token().unwrap();
        tao_tab(&t, tk::TAB_SHEET).unwrap();
        let dau: Vec<Value> = tk::tieu_de_sheet().into_iter().map(Value::from).collect();
        assert_eq!(them_hang(&t, tk::TAB_SHEET, &[dau]).unwrap().0, 200);
        tao_tab(&t, tk::TAB_MO_TA).unwrap();
        assert_eq!(them_hang(&t, tk::TAB_MO_TA, &tk::mo_ta_sheet()).unwrap().0, 200);
    }

    /// Tạo tab "Mẫu license" (dữ liệu giả, thả xuống Trạng thái, tô màu cột) + "Hướng dẫn cấp lượt" — để chủ Sheet xem trước.
    #[test]
    #[ignore = "ghi thật lên Google Sheet"]
    fn tao_mau_license() {
        let t = token().unwrap();
        let goi = |yeu_cau: Value| -> Value {
            let mut ph = agent().post(format!("{API}/{SPREADSHEET_ID}:batchUpdate")).header("Authorization", format!("Bearer {t}")).header("Content-Type", "application/json").send(json!({ "requests": yeu_cau }).to_string()).unwrap();
            let (ma, v) = doc_json(&mut ph);
            assert_eq!(ma, 200, "{v}");
            v
        };
        let v = goi(json!([
            { "addSheet": { "properties": { "title": "Hướng dẫn cấp lượt" } } },
            { "addSheet": { "properties": { "title": "Mẫu license", "gridProperties": { "frozenRowCount": 1, "frozenColumnCount": 1 } } } }
        ]));
        let id_hd = v["replies"][0]["addSheet"]["properties"]["sheetId"].as_i64().unwrap();
        let id = v["replies"][1]["addSheet"]["properties"]["sheetId"].as_i64().unwrap();
        let s = |x: &str| Value::from(x);
        let tieu_de: Vec<Value> = ["Mã máy", "Tên máy", "MAC", "IP nội bộ", "IP public", "Phiên bản", "Ngày đăng ký", "Lần dùng cuối", "Đã dùng Đếm", "Đã dùng Tách", "Hạn mức Đếm", "Hạn mức Tách", "Trạng thái", "Đơn vị", "Ghi chú"].map(s).into();
        let hang = vec![
            tieu_de,
            vec![s("SM-7F3A-91C2-04BE"), s("PC-SOHOA-01"), s("AA:BB:CC:11:22:33"), s("192.168.1.21"), s("117.5.1.10"), s("0.6.0"), s("2026-11-02 08:15"), s("2026-11-02 09:40"), 2.into(), 2.into(), s(""), s(""), s("Dùng thử"), s(""), s("Máy mới: để trống hạn mức = 2 / 2")],
            vec![s("SM-1C9E-55A0-B7D2"), s("KT-LAPTOP"), s("AA:BB:CC:44:55:66"), s("10.0.0.8"), s("113.160.2.3"), s("0.6.0"), s("2026-11-01 14:02"), s("2026-11-03 16:20"), 37.into(), 12.into(), 200.into(), 100.into(), s("Đã cấp"), s("Đơn vị A"), s("Nâng hạn mức: sửa ô K, L + chọn Đã cấp")],
            vec![s("SM-ACD0-3E19-6F44"), s("PC-THU-VIEN"), s("AA:BB:CC:77:88:99"), s("192.168.0.30"), s("14.232.5.6"), s("0.6.0"), s("2026-10-31 10:00"), s("2026-11-01 11:11"), 2.into(), 1.into(), s(""), s(""), s("Khoá"), s(""), s("Khoá máy: chọn Khoá (hiện còn lại 0)")],
            vec![s("SM-5B27-E0C4-9A81"), s("NOI-BO-SAOMAI"), s("AA:BB:CC:00:00:01"), s("192.168.10.246"), s("117.5.147.169"), s("0.6.0"), s("2026-10-30 12:00"), s("2026-11-03 17:00"), 154.into(), 88.into(), 100000.into(), 100000.into(), s("Đã cấp"), s("Sao Mai (nội bộ)"), s("Máy nội bộ / dev: hạn mức rất lớn")],
        ];
        assert_eq!(them_hang(&t, "Mẫu license", &hang).unwrap().0, 200);
        let hd: Vec<Vec<Value>> = [
            "CÁCH NÂNG SỐ LẦN ĐẾM / TÁCH CHO 1 MÁY (tab 'Mẫu license' là dữ liệu giả để xem bố cục; tab 'license' thật do trạm tự tạo khi triển khai)",
            "1. Khách gửi mã máy (hiện trên popup 'Liên hệ Sao Mai', có nút Copy; email tự điền mã máy vào tiêu đề).",
            "2. Mở tab license → Ctrl+F (Mac: Cmd+F) tìm mã máy, vd SM-1C9E-55A0-B7D2.",
            "3. Sửa cột K 'Hạn mức Đếm' và / hoặc L 'Hạn mức Tách' thành TỔNG số lần được phép (không phải số cộng thêm). Vd đã dùng 37, cho thêm 100 → gõ 137 (hoặc số tròn 200).",
            "4. Cột M 'Trạng thái' chọn 'Đã cấp'. Cột N ghi đơn vị, cột O ghi chú (số hợp đồng, ngày cấp…).",
            "5. Báo khách bấm 'Kiểm tra lại' trên popup → dùng được ngay, không cần cài lại app.",
            "",
            "THAO TÁC KHÁC",
            "• Khoá máy: cột M chọn 'Khoá' → máy chạy được tối đa thêm 1 lần rồi bị chặn (mở lại app → chặn ngay). Popup vẫn ghi 'Số lần … còn lại (0)'.",
            "• Mở khoá: chọn lại 'Đã cấp' hoặc 'Dùng thử'.",
            "• Đưa về dùng thử: xoá trống K, L, chọn 'Dùng thử' (số Đã dùng không về 0 — đã dùng ≥ 2 thì vẫn hết lượt).",
            "• Máy nội bộ / dev: K = L = 100000, M = 'Đã cấp'.",
            "",
            "KHÔNG ĐƯỢC SỬA",
            "✗ Cột A–J (thông tin máy, Đã dùng Đếm / Tách): trạm tự ghi; sửa 'Đã dùng' bị ghi đè lần chạy sau và lệch với tab license_nhat_ky.",
            "✗ Không xoá dòng của máy: máy tự đăng ký lại thành 'Dùng thử' với đã dùng = 0.",
            "✗ Không gõ tay trạng thái ngoài danh sách thả xuống (vd 'Khóa' dấu khác, 'đã cấp' chữ thường) — giá trị lạ bị coi là Khoá.",
            "",
            "Màu ở tab mẫu: xám = cột trạm ghi · vàng = cột Sao Mai sửa.",
        ].iter().map(|d| vec![s(d)]).collect();
        assert_eq!(them_hang(&t, "Hướng dẫn cấp lượt", &hd).unwrap().0, 200);
        let vung = |c0: i64, c1: i64, r0: i64, r1: i64| json!({ "sheetId": id, "startRowIndex": r0, "endRowIndex": r1, "startColumnIndex": c0, "endColumnIndex": c1 });
        let mau = |r: f64, g: f64, b: f64| json!({ "red": r, "green": g, "blue": b });
        let ghi_chu = |c: i64, chu: &str| json!({ "updateCells": { "range": vung(c, c + 1, 0, 1), "rows": [{ "values": [{ "note": chu }] }], "fields": "note" } });
        goi(json!([
            { "repeatCell": { "range": vung(0, 10, 0, 1), "cell": { "userEnteredFormat": { "backgroundColor": mau(0.93, 0.93, 0.93), "textFormat": { "bold": true } } }, "fields": "userEnteredFormat(backgroundColor,textFormat)" } },
            { "repeatCell": { "range": vung(10, 15, 0, 1), "cell": { "userEnteredFormat": { "backgroundColor": mau(1.0, 0.95, 0.8), "textFormat": { "bold": true } } }, "fields": "userEnteredFormat(backgroundColor,textFormat)" } },
            { "repeatCell": { "range": vung(10, 15, 1, 5), "cell": { "userEnteredFormat": { "backgroundColor": mau(1.0, 0.95, 0.8) } }, "fields": "userEnteredFormat.backgroundColor" } },
            { "setDataValidation": { "range": vung(12, 13, 1, 1000), "rule": { "condition": { "type": "ONE_OF_LIST", "values": [{ "userEnteredValue": "Dùng thử" }, { "userEnteredValue": "Đã cấp" }, { "userEnteredValue": "Khoá" }] }, "showCustomUi": true, "strict": true } } },
            ghi_chu(10, "Số lần Đếm được phép (tổng). Để trống = 2. Muốn nâng: gõ số mới, vd 200."),
            ghi_chu(11, "Số lần Tách được phép (tổng). Để trống = 2."),
            ghi_chu(12, "Chọn trong danh sách: Dùng thử / Đã cấp / Khoá. Giá trị khác bị coi là Khoá."),
            ghi_chu(8, "Trạm tự cộng — không sửa tay."),
            { "autoResizeDimensions": { "dimensions": { "sheetId": id, "dimension": "COLUMNS", "startIndex": 0, "endIndex": 15 } } },
            { "updateDimensionProperties": { "range": { "sheetId": id_hd, "dimension": "COLUMNS", "startIndex": 0, "endIndex": 1 }, "properties": { "pixelSize": 900 }, "fields": "pixelSize" } },
            { "repeatCell": { "range": { "sheetId": id_hd, "startRowIndex": 0, "endRowIndex": 1 }, "cell": { "userEnteredFormat": { "textFormat": { "bold": true } } }, "fields": "userEnteredFormat.textFormat" } }
        ]));
    }

    /// Ghi thật 1 hàng vào tab "kiem_tra" của Sheet (cần khoá + Sheet đã chia sẻ cho service account).
    #[test]
    #[ignore = "ghi thật lên Google Sheet"]
    fn ghi_that_len_sheet() {
        let luc = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        ghi_dong("kiem_tra", &["thoi_diem", "noi_dung"], vec![luc.into(), "sm-dema4 ghi thử".into()], None).unwrap();
    }
}
