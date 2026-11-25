//! Command chung của ứng dụng.

use std::path::Path;

use serde::Serialize;
use smks_core::{ErrorCode, SmksError};
use smks_crypto::trust::PINNED_CAS;

/// Thông tin ứng dụng hiển thị ở chân Trang chủ và hộp Thông tin phần mềm.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    /// Số chứng thư CA nhúng đã qua kiểm tra ghim lúc khởi động.
    pub pinned_ca_count: usize,
    pub user_name: String,
    pub computer_name: String,
    pub word_available: bool,
}

#[tauri::command]
pub async fn app_info() -> Result<AppInfo, SmksError> {
    super::blocking(|| {
        Ok(AppInfo {
            name: "Sao Mai Ký số",
            version: env!("CARGO_PKG_VERSION"),
            pinned_ca_count: PINNED_CAS.len(),
            user_name: std::env::var("USERNAME").unwrap_or_default(),
            computer_name: std::env::var("COMPUTERNAME").unwrap_or_default(),
            word_available: smks_merge::word::is_available(),
        })
    })
    .await
}

/// Đuôi file được phép mở bằng Explorer (không mở file chạy được).
const OPENABLE_EXTENSIONS: [&str; 2] = ["pdf", "docx"];

/// Mở thư mục, hoặc file kết quả `.pdf`/`.docx`, bằng Explorer.
#[tauri::command]
pub fn shell_open(path: String) -> Result<(), SmksError> {
    let target = Path::new(&path);
    let allowed = target.is_dir()
        || (target.is_file()
            && target.extension().is_some_and(|e| {
                OPENABLE_EXTENSIONS
                    .iter()
                    .any(|x| e.eq_ignore_ascii_case(x))
            }));
    if !allowed {
        return Err(SmksError::new(
            ErrorCode::NotFound,
            format!("Không mở được: {path}"),
        ));
    }
    std::process::Command::new("explorer.exe")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| SmksError::new(ErrorCode::System, format!("Không mở được Explorer: {e}")))
}
