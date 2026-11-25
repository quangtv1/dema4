//! Mã PIN lưu trên máy (kế hoạch D2): `%APPDATA%\SaoMai-KySo\pin.bin`.
//!
//! File chỉ chứa bản mã DPAPI của từng chứng thư (`smks_crypto::pin_vault`), dạng JSON
//! `{"version":1,"pins":{"<THUMBPRINT>":"<Base64>"}}`. Ghi `.tmp` rồi đổi tên. Không ghi PIN rõ ra đĩa hay log.
//! Lưu chỉ khi người dùng đánh dấu "Lưu PIN trên máy này" và middleware đã nhận PIN do ứng dụng đưa vào; PIN đã
//! lưu mà token báo sai thì xoá ngay, không thử lại (tránh khoá token).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use smks_core::{ErrorCode, SmksError};
use zeroize::Zeroizing;

const FILE_NAME: &str = "pin.bin";

#[derive(Debug, Default, Serialize, Deserialize)]
struct PinFile {
    version: u32,
    pins: BTreeMap<String, String>,
}

fn path() -> Result<PathBuf, SmksError> {
    Ok(super::config::app_dir()?.join(FILE_NAME))
}

fn key(thumbprint: &str) -> String {
    thumbprint.trim().to_ascii_uppercase()
}

fn read() -> PinFile {
    let Ok(path) = path() else {
        return PinFile::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write(file: &PinFile) -> Result<(), SmksError> {
    let path = path()?;
    let fail = |e: std::io::Error| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không ghi được file mã PIN đã lưu: {e}"),
        )
    };
    if file.pins.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(fail(e)),
            _ => Ok(()),
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(fail)?;
    }
    let text = serde_json::to_string_pretty(file).map_err(|e| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không ghi được file mã PIN đã lưu: {e}"),
        )
    })?;
    let tmp = path.with_file_name(format!("{FILE_NAME}.tmp"));
    std::fs::write(&tmp, text).map_err(fail)?;
    std::fs::rename(&tmp, &path).map_err(fail)
}

/// PIN đã lưu của chứng thư; bản mã hỏng / của máy khác thì xoá và coi như chưa lưu.
pub(crate) fn saved(thumbprint: &str) -> Option<Zeroizing<String>> {
    let k = key(thumbprint);
    let sealed = read().pins.get(&k).cloned()?;
    match smks_crypto::pin_vault::open(&k, &sealed) {
        Ok(pin) => Some(pin),
        Err(e) => {
            tracing::warn!(code = e.code.as_u32(), "{}", e.message);
            let _ = forget(&k);
            None
        }
    }
}

pub(crate) fn store(thumbprint: &str, pin: &str) -> Result<(), SmksError> {
    let k = key(thumbprint);
    let sealed = smks_crypto::pin_vault::seal(&k, pin)?;
    let mut file = read();
    file.version = 1;
    file.pins.insert(k, sealed);
    write(&file)?;
    tracing::info!("đã lưu mã PIN (DPAPI) cho một chứng thư");
    Ok(())
}

pub(crate) fn forget(thumbprint: &str) -> Result<bool, SmksError> {
    let mut file = read();
    let removed = file.pins.remove(&key(thumbprint)).is_some();
    if removed {
        file.version = 1;
        write(&file)?;
        tracing::info!("đã xoá mã PIN đã lưu của một chứng thư");
    }
    Ok(removed)
}

/// Thumbprint các chứng thư đang có PIN lưu trên máy.
#[tauri::command]
pub fn pin_saved() -> Vec<String> {
    read().pins.into_keys().collect()
}

/// Xoá PIN đã lưu của một chứng thư.
#[tauri::command]
pub async fn pin_forget(thumbprint: String) -> Result<bool, SmksError> {
    super::blocking(move || forget(&thumbprint)).await
}
