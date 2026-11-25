//! Hàng đợi thông báo lưu đĩa (`notify-queue.json`).
//!
//! - Sự kiện vào hàng đợi trước, gửi sau: mất mạng hay tắt ứng dụng thì sự kiện vẫn còn và được gửi ở lần sau.
//! - Gửi theo đúng thứ tự phát sinh. Sự kiện đầu hàng chưa gửi được thì các sự kiện sau cùng chờ (lỗi mạng hay lỗi
//!   cấu hình ảnh hưởng mọi tin như nhau).
//! - **Có giới hạn, không lặp vô hạn:** mỗi sự kiện thử tối đa [`MAX_ATTEMPTS`] lần, giãn cách tăng dần tới 1 giờ;
//!   quá [`MAX_AGE_SECONDS`] thì bỏ; hàng đợi giữ tối đa [`MAX_ITEMS`] sự kiện (đầy thì bỏ sự kiện cũ nhất).
//! - **Chống trùng:** mỗi sự kiện có `event_id` và một khoá chống trùng; khoá đã vào hàng đợi hoặc đã gửi thì không
//!   nhận lần hai (nhớ [`MAX_RECENT`] khoá gần nhất, kể cả sau khi khởi động lại).

use std::collections::VecDeque;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::state::{set_aside, write_atomic};

pub const FILE_NAME: &str = "notify-queue.json";
pub const MAX_ATTEMPTS: u32 = 20;
pub const MAX_ITEMS: usize = 500;
pub const MAX_AGE_SECONDS: i64 = 7 * 24 * 3600;
pub const MAX_RECENT: usize = 1000;

/// Giãn cách (giây) trước lần thử kế tiếp, sau `attempts` lần đã thất bại.
pub const fn backoff(attempts: u32) -> i64 {
    match attempts {
        0 | 1 => 10,
        2 => 30,
        3 => 120,
        4 => 300,
        5 => 900,
        6 => 1800,
        _ => 3600,
    }
}

const fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub event: Event,
    pub dedup_key: String,
    /// Chỉ dùng cho sự kiện đăng ký ở chế độ gateway: có báo lên Telegram không.
    #[serde(default = "yes")]
    pub announce: bool,
    #[serde(default)]
    pub attempts: u32,
    /// Giây unix sớm nhất được thử lại.
    #[serde(default)]
    pub next_attempt: i64,
    pub enqueued_at: i64,
    #[serde(default)]
    pub last_error: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Queue {
    pub version: u32,
    pub items: VecDeque<Item>,
    /// Khoá chống trùng của các sự kiện đã nhận (đang chờ hoặc đã gửi), mới nhất ở cuối.
    pub recent: VecDeque<String>,
}

/// Kết quả đưa một sự kiện vào hàng đợi.
#[derive(Debug, PartialEq, Eq)]
pub enum Push {
    Queued,
    /// Đã có sự kiện cùng mã hoặc cùng khoá chống trùng: bỏ qua.
    Duplicate,
    /// Đã nhận, nhưng hàng đợi đầy nên bỏ sự kiện cũ nhất.
    QueuedDroppedOldest(Box<Item>),
}

/// Kết quả ghi nhận một lần gửi thất bại.
#[derive(Debug, PartialEq, Eq)]
pub enum Failed {
    /// Sẽ thử lại sau thời điểm này.
    RetryAt(i64),
    /// Hết số lần thử: đã bỏ khỏi hàng đợi.
    GaveUp(Box<Item>),
    /// Không còn trong hàng đợi.
    Missing,
}

impl Queue {
    /// Đọc hàng đợi; file hỏng thì giữ lại ở `.hong` và bắt đầu hàng đợi trống.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!("hàng đợi thông báo hỏng ({e}); bắt đầu hàng đợi trống");
                set_aside(path);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        let result = serde_json::to_string(self)
            .map_err(std::io::Error::other)
            .and_then(|text| write_atomic(path, &text));
        if let Err(e) = result {
            tracing::warn!("không ghi được hàng đợi thông báo: {e}");
        }
    }

    fn remember(&mut self, key: &str) {
        self.recent.push_back(key.to_owned());
        while self.recent.len() > MAX_RECENT {
            self.recent.pop_front();
        }
    }

    pub fn push(&mut self, item: Item) -> Push {
        let seen = self.recent.iter().any(|k| *k == item.dedup_key)
            || self
                .items
                .iter()
                .any(|i| i.event.event_id == item.event.event_id || i.dedup_key == item.dedup_key);
        if seen {
            return Push::Duplicate;
        }
        self.version = 1;
        self.remember(&item.dedup_key);
        self.items.push_back(item);
        if self.items.len() > MAX_ITEMS
            && let Some(oldest) = self.items.pop_front()
        {
            return Push::QueuedDroppedOldest(Box::new(oldest));
        }
        Push::Queued
    }

    /// Sự kiện đầu hàng nếu đã tới lúc gửi; `Err(giây phải chờ)` nếu chưa; `Ok(None)` khi hàng đợi trống.
    pub fn head_due(&self, now: i64) -> Result<Option<&Item>, i64> {
        match self.items.front() {
            None => Ok(None),
            Some(item) if item.next_attempt > now => Err(item.next_attempt - now),
            Some(item) => Ok(Some(item)),
        }
    }

    pub fn remove(&mut self, event_id: &str) -> Option<Item> {
        let index = self
            .items
            .iter()
            .position(|i| i.event.event_id == event_id)?;
        self.items.remove(index)
    }

    /// Ghi nhận một lần gửi thất bại. `retry_after`: số giây máy chủ yêu cầu chờ (HTTP 429).
    pub fn mark_failed(
        &mut self,
        event_id: &str,
        now: i64,
        error: &str,
        retry_after: Option<i64>,
    ) -> Failed {
        let Some(index) = self.items.iter().position(|i| i.event.event_id == event_id) else {
            return Failed::Missing;
        };
        let item = &mut self.items[index];
        item.attempts += 1;
        item.last_error = error.to_owned();
        if item.attempts >= MAX_ATTEMPTS {
            return match self.items.remove(index) {
                Some(item) => Failed::GaveUp(Box::new(item)),
                None => Failed::Missing,
            };
        }
        let wait = backoff(item.attempts).max(retry_after.unwrap_or(0).clamp(0, 3600));
        item.next_attempt = now + wait;
        Failed::RetryAt(item.next_attempt)
    }

    /// Bỏ các sự kiện đã quá hạn; trả về chúng để ghi nhật ký.
    pub fn expire(&mut self, now: i64) -> Vec<Item> {
        let mut expired = Vec::new();
        self.items.retain(|item| {
            let old = now - item.enqueued_at > MAX_AGE_SECONDS;
            if old {
                expired.push(item.clone());
            }
            !old
        });
        expired
    }

    /// Cho thử gửi ngay mọi sự kiện đang chờ (khi mở ứng dụng, hoặc người dùng bấm kiểm tra kết nối). Số lần đã
    /// thử giữ nguyên, nên giới hạn số lần thử vẫn có hiệu lực.
    pub fn make_all_due(&mut self, now: i64) {
        for item in &mut self.items {
            item.next_attempt = item.next_attempt.min(now);
        }
    }

    /// Bỏ mọi sự kiện không thoả `keep`; trả về số đã bỏ.
    pub fn purge(&mut self, keep: impl Fn(&Item) -> bool) -> usize {
        let before = self.items.len();
        self.items.retain(|i| keep(i));
        before - self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventType, Metadata, Status};

    fn item(id: &str, key: &str, at: i64) -> Item {
        Item {
            event: Event {
                event_id: id.into(),
                event_type: EventType::SignSuccess,
                device_id: "d".into(),
                user_id: "u".into(),
                timestamp: at,
                transaction_id: "t".into(),
                status: Status::Success,
                metadata: Metadata::default(),
            },
            dedup_key: key.into(),
            announce: true,
            attempts: 0,
            next_attempt: 0,
            enqueued_at: at,
            last_error: String::new(),
        }
    }

    #[test]
    fn duplicates_are_refused_even_after_the_first_was_sent() {
        let mut q = Queue::default();
        assert_eq!(q.push(item("e1", "k1", 0)), Push::Queued);
        assert_eq!(
            q.push(item("e1", "k-khac", 0)),
            Push::Duplicate,
            "cùng event_id"
        );
        assert_eq!(
            q.push(item("e2", "k1", 0)),
            Push::Duplicate,
            "cùng khoá chống trùng"
        );
        assert!(q.remove("e1").is_some());
        assert_eq!(
            q.push(item("e3", "k1", 0)),
            Push::Duplicate,
            "đã gửi rồi vẫn nhớ khoá"
        );
        assert_eq!(q.push(item("e4", "k2", 0)), Push::Queued);
        assert_eq!(q.items.len(), 1);
    }

    #[test]
    fn retries_back_off_and_stop_at_the_limit() {
        let mut q = Queue::default();
        q.push(item("e1", "k1", 1000));
        q.push(item("e2", "k2", 1000));
        assert_eq!(q.head_due(1000).unwrap().unwrap().event.event_id, "e1");
        assert_eq!(
            q.mark_failed("e1", 1000, "mất mạng", None),
            Failed::RetryAt(1010)
        );
        assert_eq!(
            q.head_due(1005),
            Err(5),
            "đầu hàng chưa tới lượt thì cả hàng chờ"
        );
        assert_eq!(
            q.mark_failed("e1", 1010, "mất mạng", None),
            Failed::RetryAt(1040)
        );
        assert_eq!(
            q.mark_failed("e1", 1040, "429", Some(600)),
            Failed::RetryAt(1640),
            "tôn trọng retry_after của máy chủ"
        );
        q.make_all_due(2000);
        assert!(q.head_due(2000).unwrap().is_some());
        let mut now = 2000;
        let mut tries = 3;
        loop {
            match q.mark_failed("e1", now, "mất mạng", None) {
                Failed::RetryAt(at) => {
                    assert!(at - now <= 3600, "giãn cách tối đa 1 giờ");
                    now = at;
                    tries += 1;
                }
                Failed::GaveUp(dropped) => {
                    assert_eq!(dropped.attempts, MAX_ATTEMPTS);
                    break;
                }
                Failed::Missing => panic!("mất sự kiện"),
            }
            assert!(tries < 100, "không được lặp vô hạn");
        }
        assert_eq!(tries + 1, MAX_ATTEMPTS);
        assert_eq!(q.items.len(), 1, "sự kiện sau vẫn còn");
        assert_eq!(q.mark_failed("khong-co", now, "", None), Failed::Missing);
    }

    #[test]
    fn expiry_capacity_and_persistence() {
        let dir = crate::state::test_dir("queue");
        let path = dir.join(FILE_NAME);
        let mut q = Queue::default();
        q.push(item("cu", "k-cu", 0));
        q.push(item("moi", "k-moi", MAX_AGE_SECONDS));
        let expired = q.expire(MAX_AGE_SECONDS + 10);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].event.event_id, "cu");
        q.save(&path);
        assert_eq!(Queue::load(&path), q, "hàng đợi còn nguyên sau khi mở lại");

        let mut full = Queue::default();
        for i in 0..MAX_ITEMS {
            assert_eq!(
                full.push(item(&format!("e{i}"), &format!("k{i}"), 0)),
                Push::Queued
            );
        }
        match full.push(item("them", "k-them", 0)) {
            Push::QueuedDroppedOldest(dropped) => assert_eq!(dropped.event.event_id, "e0"),
            other => panic!("{other:?}"),
        }
        assert_eq!(full.items.len(), MAX_ITEMS);
        assert_eq!(full.purge(|i| i.event.event_id == "them"), MAX_ITEMS - 1);

        std::fs::write(&path, "hong").unwrap();
        assert_eq!(Queue::load(&path), Queue::default());
        assert!(dir.join("notify-queue.json.hong").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
