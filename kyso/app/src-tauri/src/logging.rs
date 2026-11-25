//! Ghi log ra `%LOCALAPPDATA%\SaoMai-KySo\logs\`, mỗi ngày một file, giữ 30 ngày.
//!
//! Không ghi PIN và không ghi nội dung văn bản vào log; chỉ ghi đường dẫn, tên chứng thư, mã lỗi.

use std::path::PathBuf;

use smks_core::consts::app::APP_FOLDER_NAME;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

const LOG_RETENTION_DAYS: usize = 30;

fn log_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|base| PathBuf::from(base).join(APP_FOLDER_NAME).join("logs"))
}

/// Khởi tạo log. Trả về guard phải được giữ tới hết chương trình.
/// Nếu không tạo được thư mục log thì ứng dụng vẫn chạy, chỉ là không có log ra file.
pub fn init() -> Option<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("smks")
        .filename_suffix("log")
        .max_log_files(LOG_RETENTION_DAYS)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .try_init()
        .ok()?;
    Some(guard)
}
