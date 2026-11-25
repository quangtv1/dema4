//! Cấu hình quản trị của thông báo: file `notify.json` đặt **ngoài** file chạy.
//!
//! Bot Token và Chat ID không nằm trong mã nguồn, không nằm trong EXE và không nằm trong git: quản trị viên đặt
//! file này lên máy (hoặc, tốt hơn, chỉ ghi địa chỉ gateway và khoá đăng ký, còn Bot Token nằm ở máy chủ).
//!
//! Thứ tự tìm:
//! 1. biến môi trường `SMKS_NOTIFY_CONFIG` (đường dẫn file);
//! 2. `notify.json` cạnh file chạy (thư mục cài đặt, người dùng thường không ghi được);
//! 3. `%PROGRAMDATA%\SaoMai-KySo\notify.json` (quản trị viên tạo; người dùng thường chỉ đọc).
//!
//! Không có file nào thì tính năng tắt hẳn: không hỏi đăng ký, không gửi gì.
//!
//! Tên khoá viết hoa như tài liệu triển khai: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, `NOTIFICATION_ENABLED`,
//! `DEVICE_REGISTRATION_ENABLED`, `SIGN_NOTIFICATION_ENABLED`, `DOCUMENT_NOTIFICATION_ENABLED`,
//! `FAILURE_NOTIFICATION_ENABLED`, `GATEWAY_URL`, `GATEWAY_ENROLL_KEY`, `LOCK_SETTINGS`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer};
use smks_core::consts::app::APP_FOLDER_NAME;

pub const FILE_NAME: &str = "notify.json";
/// Biến môi trường chỉ tới file cấu hình (ưu tiên cao nhất; bài kiểm tự động dùng).
pub const ENV_PATH: &str = "SMKS_NOTIFY_CONFIG";

const fn yes() -> bool {
    true
}

const fn default_timeout() -> u64 {
    10
}

/// Nhận chuỗi hoặc số (Chat ID của nhóm là số âm, quản trị viên hay ghi không có ngoặc kép).
fn string_or_number<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => s.trim().to_owned(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    })
}

/// Kênh gửi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Qua Notification Gateway (Bot Token nằm ở máy chủ).
    Gateway,
    /// Gọi thẳng Bot API (Bot Token nằm trong `notify.json` trên máy).
    Direct,
}

impl Mode {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Direct => "direct",
        }
    }
}

/// Nội dung `notify.json`.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct AdminConfig {
    #[serde(default = "yes")]
    pub notification_enabled: bool,
    #[serde(default = "yes")]
    pub device_registration_enabled: bool,
    #[serde(default = "yes")]
    pub sign_notification_enabled: bool,
    #[serde(default = "yes")]
    pub document_notification_enabled: bool,
    #[serde(default = "yes")]
    pub failure_notification_enabled: bool,
    /// Khoá các công tắc trên giao diện: người dùng không tự tắt / bật được từng loại thông báo.
    #[serde(default)]
    pub lock_settings: bool,
    /// Địa chỉ gateway, ví dụ `https://thongbao.donvi.vn`. Có thì dùng gateway, bỏ qua Bot Token trên máy.
    #[serde(default)]
    pub gateway_url: String,
    /// Khoá đăng ký thiết bị với gateway (không phải Bot Token).
    #[serde(default)]
    pub gateway_enroll_key: String,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default, deserialize_with = "string_or_number")]
    pub telegram_chat_id: String,
    /// Địa chỉ Bot API khác mặc định (máy chủ Bot API tự dựng, hoặc máy chủ giả khi kiểm thử).
    #[serde(default)]
    pub telegram_api_base: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Ô "Gửi kèm IP nội bộ" ở hộp đăng ký thiết bị được đánh dấu sẵn (người dùng vẫn bỏ được).
    #[serde(default = "yes")]
    pub send_ip_default: bool,
    /// Ô "Gửi kèm địa chỉ MAC" ở hộp đăng ký thiết bị được đánh dấu sẵn (người dùng vẫn bỏ được).
    #[serde(default)]
    pub send_mac_default: bool,
    /// Tự đăng ký thiết bị ở lần chạy đầu, không hiện hộp hỏi đồng ý (TD-60); gửi IP / MAC theo
    /// `SEND_IP_DEFAULT` / `SEND_MAC_DEFAULT`.
    #[serde(default)]
    pub auto_register: bool,
}

impl std::fmt::Debug for AdminConfig {
    /// Không in Bot Token hay khoá đăng ký ra log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminConfig")
            .field("notification_enabled", &self.notification_enabled)
            .field("mode", &self.mode())
            .field("gateway_url", &self.gateway_url)
            .field(
                "telegram_bot_token",
                &crate::text::mask(&self.telegram_bot_token),
            )
            .field(
                "telegram_chat_id",
                &crate::text::mask(&self.telegram_chat_id),
            )
            .finish_non_exhaustive()
    }
}

impl AdminConfig {
    pub fn from_json(text: &str) -> Result<Self, String> {
        // Bỏ BOM (Notepad của Windows hay thêm).
        let mut config: Self = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("không đọc được JSON ({e})"))?;
        for field in [
            &mut config.gateway_url,
            &mut config.gateway_enroll_key,
            &mut config.telegram_bot_token,
            &mut config.telegram_api_base,
        ] {
            *field = field.trim().to_owned();
        }
        config.gateway_url = config.gateway_url.trim_end_matches('/').to_owned();
        config.telegram_api_base = config.telegram_api_base.trim_end_matches('/').to_owned();
        config.timeout_seconds = config.timeout_seconds.clamp(3, 60);
        for (name, url) in [
            ("GATEWAY_URL", &config.gateway_url),
            ("TELEGRAM_API_BASE", &config.telegram_api_base),
        ] {
            if !url.is_empty() && !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(format!("{name} phải bắt đầu bằng https:// hoặc http://"));
            }
        }
        Ok(config)
    }

    /// Kênh gửi theo những gì đã khai; `None` khi thiếu thông tin.
    pub fn mode(&self) -> Option<Mode> {
        if !self.gateway_url.is_empty() {
            (!self.gateway_enroll_key.is_empty()).then_some(Mode::Gateway)
        } else if !self.telegram_bot_token.is_empty() && !self.telegram_chat_id.is_empty() {
            Some(Mode::Direct)
        } else {
            None
        }
    }

    /// Điều còn thiếu để gửi được (hiện trên giao diện cho quản trị viên).
    pub fn problem(&self) -> Option<String> {
        if self.mode().is_some() {
            return None;
        }
        Some(if self.gateway_url.is_empty() {
            "File cấu hình thiếu GATEWAY_URL + GATEWAY_ENROLL_KEY (hoặc TELEGRAM_BOT_TOKEN + TELEGRAM_CHAT_ID).".into()
        } else {
            "File cấu hình có GATEWAY_URL nhưng thiếu GATEWAY_ENROLL_KEY.".into()
        })
    }
}

/// Kết quả tìm và đọc cấu hình.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub config: Option<AdminConfig>,
    /// File đã đọc (kể cả khi file hỏng).
    pub source: Option<PathBuf>,
    /// File có nhưng không dùng được: lý do.
    pub problem: Option<String>,
}

/// Các nơi tìm `notify.json`, theo thứ tự ưu tiên. `only_env`: chỉ nhận file ở biến môi trường (khi ứng dụng chạy
/// với thư mục dữ liệu riêng của bài kiểm tự động, để cấu hình thật trên máy không lẫn vào bài kiểm).
pub fn candidates(only_env: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(p) = std::env::var_os(ENV_PATH).filter(|p| !p.is_empty()) {
        paths.push(PathBuf::from(p));
    }
    if only_env {
        return paths;
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        paths.push(dir.join(FILE_NAME));
    }
    if let Some(base) = std::env::var_os("PROGRAMDATA").filter(|p| !p.is_empty()) {
        paths.push(PathBuf::from(base).join(APP_FOLDER_NAME).join(FILE_NAME));
    }
    paths
}

/// Cấu hình nhúng lúc build (`SMKS_NOTIFY_EMBEDDED_FILE`, xem `build.rs`): khoá 32 byte rồi dữ liệu đã làm rối.
static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/notify_embedded.bin"));

/// Tên hiển thị của nguồn cấu hình nhúng.
pub const EMBEDDED_SOURCE: &str = "nhúng trong SaoMai-KySo.exe";

fn unscramble(blob: &[u8]) -> Option<String> {
    let (key, data) = (blob.get(..32)?, blob.get(32..)?);
    if data.is_empty() {
        return None;
    }
    let plain: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()] ^ (i as u8).wrapping_mul(31))
        .collect();
    String::from_utf8(plain).ok()
}

/// Cấu hình nhúng trong file chạy (nếu bản build có nhúng).
pub fn embedded() -> Option<String> {
    unscramble(EMBEDDED)
}

/// Đọc file cấu hình đầu tiên tìm thấy; không có file nào thì dùng cấu hình nhúng trong file chạy (nếu có). File
/// trên máy được ưu tiên để quản trị viên đổi nơi nhận mà không cần build lại.
pub fn load(only_env: bool) -> Loaded {
    let loaded = load_files(only_env);
    if loaded.source.is_some() || only_env {
        return loaded;
    }
    match embedded() {
        None => loaded,
        Some(text) => match AdminConfig::from_json(&text) {
            Ok(config) => Loaded {
                problem: config.problem(),
                config: Some(config),
                source: Some(PathBuf::from(EMBEDDED_SOURCE)),
            },
            Err(e) => Loaded {
                config: None,
                problem: Some(format!("Cấu hình nhúng trong ứng dụng không hợp lệ: {e}.")),
                source: Some(PathBuf::from(EMBEDDED_SOURCE)),
            },
        },
    }
}

fn load_files(only_env: bool) -> Loaded {
    for path in candidates(only_env) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Loaded {
                    config: None,
                    problem: Some(format!("Không đọc được {}: {e}", path.display())),
                    source: Some(path),
                };
            }
        };
        return match AdminConfig::from_json(&text) {
            Ok(config) => Loaded {
                problem: config.problem(),
                config: Some(config),
                source: Some(path),
            },
            Err(e) => Loaded {
                config: None,
                problem: Some(format!(
                    "File cấu hình {} không hợp lệ: {e}.",
                    path.display()
                )),
                source: Some(path),
            },
        };
    }
    Loaded::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscrambles_build_blob() {
        let key: Vec<u8> = (1..=32).collect();
        let text = r#"{ "TELEGRAM_BOT_TOKEN": "x", "TELEGRAM_CHAT_ID": -1 }"#;
        let mut blob = key.clone();
        blob.extend(
            text.bytes()
                .enumerate()
                .map(|(i, b)| b ^ key[i % 32] ^ (i as u8).wrapping_mul(31)),
        );
        assert_eq!(unscramble(&blob).as_deref(), Some(text));
        assert!(
            !blob.windows(5).any(|w| w == b"TELEG"),
            "không còn chữ thường"
        );
        assert_eq!(unscramble(&key), None);
        assert_eq!(unscramble(&[]), None);
        // Bản build không nhúng gì (bài kiểm chạy không đặt biến).
        assert_eq!(embedded(), None);
    }

    #[test]
    fn direct_mode_accepts_numeric_chat_id_and_defaults_flags_on() {
        let c = AdminConfig::from_json(
            "\u{feff}{ \"TELEGRAM_BOT_TOKEN\": \" 123:ABCdef \", \"TELEGRAM_CHAT_ID\": -1001234567890 }",
        )
        .unwrap();
        assert_eq!(c.mode(), Some(Mode::Direct));
        assert_eq!(c.telegram_bot_token, "123:ABCdef");
        assert_eq!(c.telegram_chat_id, "-1001234567890");
        assert!(c.notification_enabled && c.sign_notification_enabled && !c.lock_settings);
        assert_eq!(c.timeout_seconds, 10);
        assert!(c.problem().is_none());
        // Bot Token không lộ khi in Debug.
        assert!(!format!("{c:?}").contains("ABCdef"));
    }

    #[test]
    fn gateway_wins_and_needs_enroll_key() {
        let c = AdminConfig::from_json(
            r#"{ "GATEWAY_URL": "https://tb.vn/", "GATEWAY_ENROLL_KEY": "k", "TELEGRAM_BOT_TOKEN": "x", "TELEGRAM_CHAT_ID": "1",
                 "SIGN_NOTIFICATION_ENABLED": false, "LOCK_SETTINGS": true, "TIMEOUT_SECONDS": 999 }"#,
        )
        .unwrap();
        assert_eq!(c.mode(), Some(Mode::Gateway));
        assert_eq!(c.gateway_url, "https://tb.vn");
        assert!(!c.sign_notification_enabled && c.lock_settings);
        assert_eq!(c.timeout_seconds, 60);

        // Gợi ý ô IP / MAC ở hộp đăng ký: mặc định IP bật, MAC tắt; quản trị viên đổi được.
        let d = AdminConfig::from_json(r#"{ "TELEGRAM_BOT_TOKEN": "x", "TELEGRAM_CHAT_ID": -1 }"#)
            .unwrap();
        assert!(d.send_ip_default && !d.send_mac_default);
        let d = AdminConfig::from_json(
            r#"{ "TELEGRAM_BOT_TOKEN": "x", "TELEGRAM_CHAT_ID": -1, "SEND_IP_DEFAULT": false, "SEND_MAC_DEFAULT": true }"#,
        )
        .unwrap();
        assert!(!d.send_ip_default && d.send_mac_default);
        let missing = AdminConfig::from_json(r#"{ "GATEWAY_URL": "https://tb.vn" }"#).unwrap();
        assert_eq!(missing.mode(), None);
        assert!(missing.problem().unwrap().contains("GATEWAY_ENROLL_KEY"));
        assert!(AdminConfig::from_json("{}").unwrap().problem().is_some());
        assert!(AdminConfig::from_json(r#"{ "GATEWAY_URL": "ftp://x" }"#).is_err());
        assert!(AdminConfig::from_json("khong phai json").is_err());
    }
}
