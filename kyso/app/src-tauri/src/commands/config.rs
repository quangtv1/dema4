//! Cấu hình người dùng: đọc / ghi `%APPDATA%\SaoMai-KySo\config.json` (F-11).
//!
//! Kiểu dữ liệu, mặc định và làm sạch giá trị ở `smks_core::config`. Ở đây chỉ có I/O: ghi `.tmp` rồi đổi tên
//! (không làm hỏng file đang có khi ghi dở); file hỏng thì đổi tên thành `config.json.hong-<giờ>` rồi dùng mặc
//! định, không xoá cấu hình của người dùng.

use std::path::PathBuf;

use serde::Serialize;
use smks_core::config::AppConfig;
use smks_core::consts::app::APP_FOLDER_NAME;
use smks_core::{ErrorCode, SmksError};

use super::blocking;

const FILE_NAME: &str = "config.json";

/// `%APPDATA%\SaoMai-KySo\config.json`; biến môi trường `SMKS_CONFIG_DIR` đổi thư mục (bài kiểm tự động dùng
/// thư mục riêng, không đụng cấu hình thật của người dùng).
fn config_path() -> Result<PathBuf, SmksError> {
    Ok(app_dir()?.join(FILE_NAME))
}

/// Thư mục dữ liệu của ứng dụng: `%APPDATA%\SaoMai-KySo`, hoặc `SMKS_CONFIG_DIR` khi kiểm tự động.
pub(crate) fn app_dir() -> Result<PathBuf, SmksError> {
    if let Some(dir) = std::env::var_os("SMKS_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    std::env::var_os("APPDATA")
        .map(|base| PathBuf::from(base).join(APP_FOLDER_NAME))
        .ok_or_else(|| {
            SmksError::new(
                ErrorCode::InternalError,
                "Không tìm thấy thư mục %APPDATA% của người dùng.",
            )
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigLoad {
    pub config: AppConfig,
    pub path: String,
    /// Chưa có file cấu hình (lần đầu chạy): đang dùng mặc định.
    pub created: bool,
    /// Có điều cần báo người dùng (file hỏng, giá trị đã sửa).
    pub warning: Option<String>,
}

fn load() -> Result<ConfigLoad, SmksError> {
    let path = config_path()?;
    let shown = path.display().to_string();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConfigLoad {
                config: AppConfig::default(),
                path: shown,
                created: true,
                warning: None,
            });
        }
        Err(e) => {
            tracing::warn!("không đọc được cấu hình {shown}: {e}");
            return Ok(ConfigLoad {
                config: AppConfig::default(),
                path: shown,
                created: false,
                warning: Some(format!(
                    "Không đọc được file cấu hình ({e}); đang dùng cấu hình mặc định."
                )),
            });
        }
    };
    match AppConfig::from_json(&text) {
        Ok((config, notes)) => {
            for n in &notes {
                tracing::info!("cấu hình: {n}");
            }
            let warning = (!notes.is_empty())
                .then(|| format!("Đã sửa một số giá trị cấu hình: {}", notes.join(" ")));
            Ok(ConfigLoad {
                config,
                path: shown,
                created: false,
                warning,
            })
        }
        Err(error) => {
            // Đổi tên file hỏng (không xoá) để người dùng / hỗ trợ xem, rồi dùng mặc định. Đổi tên chứ không
            // chép, để lần mở sau không báo lại và không sinh thêm bản sao.
            let stamp = smks_sign::verify::now_vietnam()
                .1
                .replace(['/', ':', ' '], "");
            let backup = path.with_file_name(format!("{FILE_NAME}.hong-{stamp}"));
            let kept = std::fs::rename(&path, &backup).is_ok();
            tracing::warn!(code = error.code.as_u32(), backup = %backup.display(), "{}", error.message);
            Ok(ConfigLoad {
                config: AppConfig::default(),
                path: shown,
                created: false,
                warning: Some(if kept {
                    format!("{} Bản cũ được giữ ở {}.", error.message, backup.display())
                } else {
                    error.message
                }),
            })
        }
    }
}

fn save(mut config: AppConfig) -> Result<(AppConfig, Vec<String>), SmksError> {
    let notes = config.sanitize();
    let path = config_path()?;
    let fail = |what: &str, e: std::io::Error| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không lưu được cấu hình ({what} {}): {e}", path.display()),
        )
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| fail("tạo thư mục", e))?;
    }
    let tmp = path.with_file_name(format!("{FILE_NAME}.tmp"));
    std::fs::write(&tmp, config.to_json()?).map_err(|e| fail("ghi", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail("đổi tên", e)
    })?;
    tracing::info!("đã lưu cấu hình {}", path.display());
    Ok((config, notes))
}

/// Cấu hình mặc định của lõi (nút Khôi phục mặc định).
#[tauri::command]
pub fn config_defaults() -> AppConfig {
    AppConfig::default()
}

/// Đọc cấu hình (không có file thì mặc định; file hỏng thì giữ bản sao và dùng mặc định).
#[tauri::command]
pub async fn config_get() -> Result<ConfigLoad, SmksError> {
    blocking(load).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSaved {
    pub config: AppConfig,
    /// Giá trị đã được sửa cho hợp lệ trước khi lưu.
    pub notes: Vec<String>,
}

/// Làm sạch rồi lưu cấu hình; trả lại cấu hình đã lưu.
#[tauri::command]
pub async fn config_save(config: AppConfig) -> Result<ConfigSaved, SmksError> {
    let (config, notes) = blocking(move || save(config)).await?;
    Ok(ConfigSaved { config, notes })
}
