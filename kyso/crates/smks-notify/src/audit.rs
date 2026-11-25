//! Nhật ký sự kiện và trạng thái gửi: `<thư mục dữ liệu>\nhat-ky-su-kien\events-<yyyy-mm>.jsonl`.
//!
//! Mỗi dòng là một JSON:
//! - `{"kind":"event", "at":…, "disposition":"queued"|"skipped", "reason":…, "event":{…}}` — ghi **ngay khi nghiệp
//!   vụ kết thúc**, kể cả khi thông báo đang tắt hoặc người dùng chưa đồng ý gửi (khi đó chỉ nằm trên máy);
//! - `{"kind":"delivery", "at":…, "event_id":…, "event_type":…, "result":"sent"|"retry"|"gave-up"|"rejected"|
//!   "dropped", "attempts":…, "error":…}` — mỗi lần thử gửi.
//!
//! Nhật ký chỉ có mô tả sự kiện (tên file, số lượng, mã lỗi). Không có nội dung văn bản, dữ liệu chữ ký, mã PIN hay
//! Bot Token.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde_json::json;

use crate::event::Event;
use crate::time;

pub const FOLDER_NAME: &str = "nhat-ky-su-kien";

#[derive(Debug)]
pub struct Audit {
    dir: PathBuf,
    /// Tuần tự hoá việc ghi giữa luồng nghiệp vụ và luồng gửi.
    gate: Mutex<()>,
}

impl Audit {
    pub fn new(app_dir: &Path) -> Self {
        Self {
            dir: app_dir.join(FOLDER_NAME),
            gate: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn append(&self, now: i64, line: &serde_json::Value) {
        let _guard = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        let path = self
            .dir
            .join(format!("events-{}.jsonl", time::month_key(now)));
        let result = std::fs::create_dir_all(&self.dir).and_then(|()| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            writeln!(file, "{line}")
        });
        if let Err(e) = result {
            tracing::warn!("không ghi được nhật ký sự kiện: {e}");
        }
    }

    /// Ghi nhận một sự kiện nghiệp vụ. `skipped`: lý do không đưa vào hàng đợi gửi (không có thì đã xếp hàng).
    pub fn event(&self, event: &Event, skipped: Option<&str>) {
        let now = time::now();
        self.append(
            now,
            &json!({
                "kind": "event",
                "at": time::iso(now),
                "disposition": if skipped.is_some() { "skipped" } else { "queued" },
                "reason": skipped,
                "event": event,
            }),
        );
    }

    /// Ghi nhận kết quả một lần gửi.
    pub fn delivery(&self, event: &Event, result: &str, attempts: u32, error: Option<&str>) {
        let now = time::now();
        self.append(
            now,
            &json!({
                "kind": "delivery",
                "at": time::iso(now),
                "event_id": event.event_id,
                "event_type": event.event_type,
                "transaction_id": event.transaction_id,
                "result": result,
                "attempts": attempts,
                "error": error,
            }),
        );
    }
}
