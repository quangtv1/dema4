//! Trạng thái thông báo của máy này, lưu ở `<thư mục dữ liệu>\notify-state.json`.
//!
//! Giữ: mã thiết bị, người dùng đã đồng ý / từ chối đăng ký chưa, đã đăng ký xong chưa (để không gửi đăng ký lặp
//! lại mỗi lần mở ứng dụng), các công tắc của người dùng, và bí mật thiết bị do gateway cấp (đã mã hoá DPAPI ở tầng
//! ứng dụng; file này không bao giờ chứa Bot Token).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::device::new_device_id;

pub const FILE_NAME: &str = "notify-state.json";

/// Lựa chọn của người dùng ở hộp đăng ký thiết bị.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Consent {
    /// Chưa trả lời: hỏi khi mở ứng dụng, chưa gửi gì.
    #[default]
    Pending,
    Accepted,
    Declined,
}

/// Bộ công tắc bật / tắt (của quản trị viên trong `notify.json`, hoặc của người dùng trên giao diện).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Flags {
    /// Công tắc chung.
    pub enabled: bool,
    /// Đăng ký thiết bị (và hủy đăng ký, cập nhật ứng dụng).
    pub device: bool,
    pub sign: bool,
    pub document: bool,
    /// Sự kiện thất bại / bị hủy.
    pub failure: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            enabled: true,
            device: true,
            sign: true,
            document: true,
            failure: true,
        }
    }
}

impl Flags {
    /// Cả hai bên cùng bật thì mới bật.
    pub const fn and(self, other: Self) -> Self {
        Self {
            enabled: self.enabled && other.enabled,
            device: self.device && other.device,
            sign: self.sign && other.sign,
            document: self.document && other.document,
            failure: self.failure && other.failure,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub version: u32,
    pub device_id: String,
    pub consent: Consent,
    pub consent_at: Option<i64>,
    /// Người dùng cho gửi IP nội bộ / địa chỉ MAC khi đăng ký.
    pub include_ip: bool,
    pub include_mac: bool,
    /// Lúc người dùng đồng ý đăng ký.
    pub registered_at: Option<i64>,
    /// Lúc sự kiện đăng ký tới được hệ thống quản trị.
    pub registration_delivered_at: Option<i64>,
    /// Bí mật thiết bị do gateway cấp, đã mã hoá.
    pub credential: Option<String>,
    pub prefs: Flags,
    /// Phiên bản ứng dụng ở lần chạy trước (phát hiện cập nhật).
    pub last_app_version: String,
    pub last_sent_at: Option<i64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            device_id: new_device_id(),
            consent: Consent::Pending,
            consent_at: None,
            include_ip: false,
            include_mac: false,
            registered_at: None,
            registration_delivered_at: None,
            credential: None,
            prefs: Flags::default(),
            last_app_version: String::new(),
            last_sent_at: None,
        }
    }
}

/// Ghi `text` ra `path` qua file `.tmp` rồi đổi tên (không làm hỏng file đang có khi ghi dở).
pub(crate) fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// File hỏng thì đổi tên thành `<tên>.hong` (giữ lại để xem), không xoá.
pub(crate) fn set_aside(path: &Path) {
    let mut bad = path.as_os_str().to_owned();
    bad.push(".hong");
    let _ = std::fs::rename(path, std::path::PathBuf::from(bad));
}

impl State {
    /// Đọc trạng thái. Chưa có file (lần chạy đầu trên máy): sinh mã thiết bị mới và ghi ngay, để mã không đổi ở
    /// các lần mở sau. Trả kèm `true` khi vừa tạo mới.
    pub fn load(path: &Path) -> (Self, bool) {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(mut state) => {
                    if state.device_id.trim().is_empty() {
                        state.device_id = new_device_id();
                        state.save(path);
                    }
                    return (state, false);
                }
                Err(e) => {
                    tracing::warn!("trạng thái thông báo hỏng ({e}); tạo lại, bản cũ giữ ở .hong");
                    set_aside(path);
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("không đọc được trạng thái thông báo: {e}"),
        }
        let state = Self::default();
        state.save(path);
        (state, true)
    }

    /// Ghi trạng thái; lỗi ghi chỉ vào log (thông báo là phần phụ, không làm hỏng nghiệp vụ).
    pub fn save(&self, path: &Path) {
        let result = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)
            .and_then(|text| write_atomic(path, &text));
        if let Err(e) = result {
            tracing::warn!("không ghi được trạng thái thông báo: {e}");
        }
    }
}

#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "smks-notify-test-{name}-{}-{}",
        std::process::id(),
        crate::time::random_hex::<4>()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_created_once_and_survives_restart() {
        let dir = test_dir("state");
        let path = dir.join(FILE_NAME);
        let (first, created) = State::load(&path);
        assert!(created && path.is_file());
        assert_eq!(first.consent, Consent::Pending);
        let (again, created) = State::load(&path);
        assert!(!created);
        assert_eq!(again.device_id, first.device_id);

        let mut changed = again;
        changed.consent = Consent::Declined;
        changed.prefs.sign = false;
        changed.save(&path);
        let (read, _) = State::load(&path);
        assert_eq!(read, changed);
        assert!(!dir.join("notify-state.json.tmp").exists());

        // File hỏng: giữ bản cũ, tạo trạng thái mới (chưa đồng ý).
        std::fs::write(&path, "{ hong").unwrap();
        let (fresh, created) = State::load(&path);
        assert!(created && fresh.consent == Consent::Pending);
        assert!(dir.join("notify-state.json.hong").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn flags_combine_with_and() {
        let admin = Flags {
            sign: false,
            ..Flags::default()
        };
        let user = Flags {
            document: false,
            ..Flags::default()
        };
        let both = admin.and(user);
        assert!(both.enabled && both.device && both.failure);
        assert!(!both.sign && !both.document);
    }
}
