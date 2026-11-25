//! Các command giao diện gọi qua IPC. Mỗi command phải được khai trong `build.rs` và cấp quyền trong
//! `capabilities/default.json`. Hợp đồng phía giao diện: `app/ui/src/app/core/ipc.service.ts`.
//!
//! Command nặng (đọc file, gọi Word) là `async` và chạy việc trên luồng chặn riêng: command không `async`
//! của Tauri chạy trên luồng giao diện và sẽ làm đơ cửa sổ.

pub mod app;
pub mod config;
pub mod driver;
pub mod merge;
pub mod notify;
pub mod pin;
pub mod report;
pub mod sign;

use smks_core::{ErrorCode, SmksError};

/// Chạy việc nặng trên luồng chặn của Tauri.
pub async fn blocking<T, F>(work: F) -> Result<T, SmksError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, SmksError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| {
            SmksError::new(
                ErrorCode::InternalError,
                format!("Tác vụ nền bị dừng bất thường: {e}"),
            )
        })?
}
