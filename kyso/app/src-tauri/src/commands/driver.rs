//! Hỗ trợ › Cài đặt driver USB Token (HT-02): kiểm tra máy đã có driver token chưa, chưa có thì hướng dẫn tải ở
//! trang phần mềm của Ban Cơ yếu Chính phủ.
//!
//! Nhận diện giống bộ cài NSIS của bản cũ (`installer/SaoMai-KySo.nsi`, `DetectBit4id`, `DetectTokenManager`):
//! - Bit4id Universal MW (token đời mới): khoá `HKLM\SOFTWARE\WOW6432Node\bit4id`, hoặc file
//!   `%ProgramFiles(x86)%\Bit4Id\Universal MW\uninst.exe`.
//! - VGCA TokenManager / SafeNet eToken (token đời cũ): CSP tên bắt đầu bằng `eToken` trong
//!   `HKLM\SOFTWARE\Microsoft\Cryptography\Defaults\Provider`, hoặc `System32\eTOKCSP.dll`.
//!
//! Phiên bản lấy từ mục gỡ cài đặt của Windows (`DisplayVersion`). Chỉ đọc registry, không cần quyền quản trị.

use std::path::PathBuf;

use serde::Serialize;
use smks_core::{ErrorCode, SmksError};

/// Trang tải phần mềm (driver USB Token, công cụ ký) của Ban Cơ yếu Chính phủ; chuyển tới
/// `dichvucong.ca.gov.vn/tai-phan-mem` (kiểm ngày 20/11/2026).
pub const DRIVER_DOWNLOAD_URL: &str = "https://ca.gov.vn/tai-phan-mem";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverItem {
    pub id: &'static str,
    pub name: &'static str,
    /// Loại token dùng driver này.
    pub tokens: &'static str,
    pub installed: bool,
    pub version: Option<String>,
    /// Dấu hiệu tìm thấy (khoá registry / file), để hỗ trợ kỹ thuật đối chiếu.
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverStatus {
    pub drivers: Vec<DriverItem>,
    /// Có ít nhất một driver token.
    pub any_installed: bool,
    /// Dịch vụ "Smart Card" của Windows (SCardSvr) bị tắt (Start = 4): token không làm việc được.
    pub smart_card_disabled: bool,
    pub download_url: &'static str,
}

#[cfg(windows)]
mod probe {
    use windows_registry::LOCAL_MACHINE;

    const UNINSTALL: [&str; 2] = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    pub fn key_exists(path: &str) -> bool {
        LOCAL_MACHINE.open(path).is_ok()
    }

    /// Tên các khoá con.
    pub fn subkeys(path: &str) -> Vec<String> {
        LOCAL_MACHINE
            .open(path)
            .and_then(|k| k.keys().map(Iterator::collect))
            .unwrap_or_default()
    }

    /// `(DisplayName, DisplayVersion)` của chương trình đã cài đầu tiên có tên khớp `matches`.
    pub fn installed_program(matches: &dyn Fn(&str) -> bool) -> Option<(String, Option<String>)> {
        UNINSTALL.iter().find_map(|root| {
            let key = LOCAL_MACHINE.open(root).ok()?;
            key.keys().ok()?.find_map(|name| {
                let app = key.open(&name).ok()?;
                let display = app.get_string("DisplayName").ok()?;
                matches(&display).then(|| (display, app.get_string("DisplayVersion").ok()))
            })
        })
    }

    pub fn service_start(name: &str) -> Option<u32> {
        LOCAL_MACHINE
            .open(format!(r"SYSTEM\CurrentControlSet\Services\{name}"))
            .ok()?
            .get_u32("Start")
            .ok()
    }
}

fn env_path(var: &str, rest: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(|base| PathBuf::from(base).join(rest))
}

#[cfg(windows)]
fn status() -> DriverStatus {
    // Bit4id Universal MW.
    let bit4id_key = r"SOFTWARE\WOW6432Node\bit4id";
    let bit4id_file =
        env_path("ProgramFiles(x86)", r"Bit4Id\Universal MW\uninst.exe").filter(|p| p.is_file());
    let bit4id_prog = probe::installed_program(&|n| n.to_ascii_lowercase().starts_with("bit4id"));
    let bit4id_evidence = if probe::key_exists(bit4id_key) {
        Some(format!(r"HKLM\{bit4id_key}"))
    } else {
        bit4id_file.as_ref().map(|p| p.display().to_string())
    };
    let bit4id = DriverItem {
        id: "bit4id",
        name: "Bit4id Universal Middleware",
        tokens: "USB Token đời mới của Ban Cơ yếu (bit4id)",
        installed: bit4id_evidence.is_some() || bit4id_prog.is_some(),
        version: bit4id_prog.and_then(|(_, v)| v),
        evidence: bit4id_evidence,
    };

    // VGCA TokenManager / SafeNet eToken.
    let providers = r"SOFTWARE\Microsoft\Cryptography\Defaults\Provider";
    let etoken_csp = probe::subkeys(providers)
        .into_iter()
        .find(|n| n.starts_with("eToken"));
    let etoken_dll = env_path("WINDIR", r"System32\eTOKCSP.dll").filter(|p| p.is_file());
    let etoken_prog = probe::installed_program(&|n| {
        let n = n.to_ascii_lowercase();
        n.contains("tokenmanager")
            || n.contains("token manager")
            || n.contains("safenet")
            || n.contains("vgca")
    });
    let etoken_evidence = etoken_csp
        .map(|n| format!(r"CSP {n}"))
        .or_else(|| etoken_dll.map(|p| p.display().to_string()));
    let etoken = DriverItem {
        id: "etoken",
        name: "VGCA TokenManager (SafeNet eToken)",
        tokens: "USB Token đời cũ của Ban Cơ yếu (SafeNet eToken)",
        installed: etoken_evidence.is_some(),
        version: etoken_prog.and_then(|(_, v)| v),
        evidence: etoken_evidence,
    };

    let drivers = vec![bit4id, etoken];
    DriverStatus {
        any_installed: drivers.iter().any(|d| d.installed),
        drivers,
        smart_card_disabled: probe::service_start("SCardSvr") == Some(4),
        download_url: DRIVER_DOWNLOAD_URL,
    }
}

#[cfg(not(windows))]
fn status() -> DriverStatus {
    DriverStatus {
        drivers: Vec::new(),
        any_installed: false,
        smart_card_disabled: false,
        download_url: DRIVER_DOWNLOAD_URL,
    }
}

/// Trạng thái driver USB Token trên máy.
#[tauri::command]
pub async fn driver_status() -> Result<DriverStatus, SmksError> {
    super::blocking(|| {
        #[cfg_attr(not(feature = "dev-keys"), allow(unused_mut))]
        let mut s = status();
        // Chỉ bản build thử (`dev-keys`): giả lập máy chưa có driver để kiểm giao diện hướng dẫn tải.
        #[cfg(feature = "dev-keys")]
        if std::env::var_os("SMKS_DEV_NO_DRIVER").is_some() {
            for d in &mut s.drivers {
                d.installed = false;
                d.version = None;
                d.evidence = None;
            }
            s.any_installed = false;
        }
        for d in &s.drivers {
            tracing::info!(
                driver = d.id,
                installed = d.installed,
                version = d.version.as_deref().unwrap_or("-"),
                "driver token"
            );
        }
        Ok(s)
    })
    .await
}

/// Mở trang web cố định bằng trình duyệt mặc định. Chỉ nhận mã trang, không nhận địa chỉ tuỳ ý từ giao diện.
#[tauri::command]
pub fn open_link(id: String) -> Result<(), SmksError> {
    let url = match id.as_str() {
        "driver" => DRIVER_DOWNLOAD_URL,
        "home" => "https://saomaiso.com",
        _ => {
            return Err(SmksError::new(
                ErrorCode::BadRequest,
                format!("Không có trang \"{id}\"."),
            ));
        }
    };
    std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| SmksError::new(ErrorCode::System, format!("Không mở được trình duyệt: {e}")))
}
