//! Sự kiện nghiệp vụ thống nhất.
//!
//! Một sự kiện chỉ mang **mô tả** của việc đã xảy ra: tên file, số lượng, mã lỗi, thông tin thiết bị ở mức tối
//! thiểu. Không có trường nào chứa nội dung văn bản, dữ liệu chữ ký, khoá hay mã PIN, và [`Event::sanitize`] cắt mọi
//! chuỗi về độ dài an toàn trước khi lưu hoặc gửi (gateway gọi lại hàm này với dữ liệu nhận từ máy trạm).

use serde::{Deserialize, Serialize};

use crate::text::{clean, safe_error_message};

/// Loại sự kiện. Tên trên dây và trong nhật ký là dạng `SIGN_SUCCESS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    DeviceRegistered,
    DeviceUnregistered,
    AppUpdated,
    SignSuccess,
    SignFailed,
    SignCancelled,
    DocumentCreated,
    DocumentFailed,
    DocumentCancelled,
    /// Dành sẵn để mở rộng (xuất văn bản riêng); bản này chưa phát sự kiện này.
    DocumentExported,
    /// Dành sẵn để mở rộng (xoá văn bản); bản này chưa phát sự kiện này.
    DocumentDeleted,
    /// Tin nhắn thử nghiệm từ giao diện cấu hình.
    Test,
}

/// Nhóm bật / tắt của sự kiện.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Device,
    Sign,
    Document,
    Test,
}

/// Kết quả nghiệp vụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Failed,
    Cancelled,
}

impl EventType {
    pub const ALL: [EventType; 12] = [
        EventType::DeviceRegistered,
        EventType::DeviceUnregistered,
        EventType::AppUpdated,
        EventType::SignSuccess,
        EventType::SignFailed,
        EventType::SignCancelled,
        EventType::DocumentCreated,
        EventType::DocumentFailed,
        EventType::DocumentCancelled,
        EventType::DocumentExported,
        EventType::DocumentDeleted,
        EventType::Test,
    ];

    pub const fn category(self) -> Category {
        match self {
            Self::DeviceRegistered | Self::DeviceUnregistered | Self::AppUpdated => {
                Category::Device
            }
            Self::SignSuccess | Self::SignFailed | Self::SignCancelled => Category::Sign,
            Self::DocumentCreated
            | Self::DocumentFailed
            | Self::DocumentCancelled
            | Self::DocumentExported
            | Self::DocumentDeleted => Category::Document,
            Self::Test => Category::Test,
        }
    }

    pub const fn status(self) -> Status {
        match self {
            Self::SignFailed | Self::DocumentFailed => Status::Failed,
            Self::SignCancelled | Self::DocumentCancelled => Status::Cancelled,
            _ => Status::Success,
        }
    }

    /// Sự kiện thất bại hoặc bị hủy (bật / tắt bằng công tắc "thông báo thất bại").
    pub const fn is_failure(self) -> bool {
        !matches!(self.status(), Status::Success)
    }

    /// Tên trên dây (`SIGN_SUCCESS`).
    pub const fn code(self) -> &'static str {
        match self {
            Self::DeviceRegistered => "DEVICE_REGISTERED",
            Self::DeviceUnregistered => "DEVICE_UNREGISTERED",
            Self::AppUpdated => "APP_UPDATED",
            Self::SignSuccess => "SIGN_SUCCESS",
            Self::SignFailed => "SIGN_FAILED",
            Self::SignCancelled => "SIGN_CANCELLED",
            Self::DocumentCreated => "DOCUMENT_CREATED",
            Self::DocumentFailed => "DOCUMENT_FAILED",
            Self::DocumentCancelled => "DOCUMENT_CANCELLED",
            Self::DocumentExported => "DOCUMENT_EXPORTED",
            Self::DocumentDeleted => "DOCUMENT_DELETED",
            Self::Test => "TEST",
        }
    }

    /// Nhãn ở dòng đầu tin nhắn (`[SAO MAI SIGN - SIGN SUCCESS]`).
    pub const fn tag(self) -> &'static str {
        match self {
            Self::DeviceRegistered => "NEW DEVICE",
            Self::DeviceUnregistered => "DEVICE UNREGISTERED",
            Self::AppUpdated => "APP UPDATED",
            Self::SignSuccess => "SIGN SUCCESS",
            Self::SignFailed => "SIGN FAILED",
            Self::SignCancelled => "SIGN CANCELLED",
            Self::DocumentCreated => "DOCUMENT CREATED",
            Self::DocumentFailed => "DOCUMENT FAILED",
            Self::DocumentCancelled => "DOCUMENT CANCELLED",
            Self::DocumentExported => "DOCUMENT EXPORTED",
            Self::DocumentDeleted => "DOCUMENT DELETED",
            Self::Test => "TEST",
        }
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Một văn bản lỗi trong lô.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Failure {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub code: String,
    pub message: String,
}

/// Số tên văn bản / số dòng lỗi tối đa mang theo một sự kiện (phần còn lại chỉ đếm).
pub const MAX_LISTED: usize = 10;

/// Dữ liệu kèm theo sự kiện. Trường rỗng không ghi ra JSON.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hostname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub app_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub os: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub os_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub architecture: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub local_ip: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mac_address: String,
    /// Tên file (không có thư mục) của tối đa [`MAX_LISTED`] văn bản.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<String>,
    /// Tổng số văn bản của sự kiện (có thể lớn hơn `documents.len()`).
    #[serde(skip_serializing_if = "is_zero")]
    pub document_count: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub document_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub signature_type: String,
    /// Tên trên chứng thư dùng để ký (thông tin công khai của chữ ký).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub signer: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_code: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<Failure>,
    /// Số văn bản chưa xử lý khi nghiệp vụ dừng giữa chừng.
    #[serde(skip_serializing_if = "is_zero")]
    pub pending_count: u32,
    /// Tổng số văn bản của lô.
    #[serde(skip_serializing_if = "is_zero")]
    pub total_count: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub previous_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl Metadata {
    /// Ghi danh sách tên văn bản: giữ [`MAX_LISTED`] tên đầu, đếm toàn bộ.
    pub fn set_documents<I, S>(&mut self, names: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut count = 0u32;
        self.documents.clear();
        for name in names {
            count = count.saturating_add(1);
            if self.documents.len() < MAX_LISTED {
                self.documents.push(name.into());
            }
        }
        self.document_count = count;
    }

    /// Ghi danh sách văn bản lỗi: giữ [`MAX_LISTED`] dòng đầu, đếm toàn bộ vào `document_count`.
    pub fn set_failures<I: IntoIterator<Item = Failure>>(&mut self, failures: I) {
        let mut count = 0u32;
        self.failures.clear();
        for failure in failures {
            count = count.saturating_add(1);
            if self.failures.len() < MAX_LISTED {
                self.failures.push(failure);
            }
        }
        self.document_count = count;
    }
}

/// Sự kiện nghiệp vụ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// UUID của sự kiện; dùng để chống gửi trùng khi gửi lại.
    pub event_id: String,
    pub event_type: EventType,
    pub device_id: String,
    /// Tài khoản Windows đang dùng ứng dụng.
    pub user_id: String,
    /// Giây unix lúc nghiệp vụ kết thúc.
    pub timestamp: i64,
    /// Mã giao dịch của lần ký / lần tạo văn bản (một lô là một giao dịch).
    pub transaction_id: String,
    pub status: Status,
    #[serde(default)]
    pub metadata: Metadata,
}

/// Chỉ giữ chữ, số và `-_.:` (mã định danh).
fn clean_id(text: &str, max: usize) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
        .take(max)
        .collect()
}

impl Event {
    /// Cắt và làm sạch mọi chuỗi; đặt lại `status` theo loại sự kiện. Gọi trước khi lưu, gửi, và ở gateway với dữ
    /// liệu nhận từ máy trạm.
    pub fn sanitize(&mut self) {
        self.event_id = clean_id(&self.event_id, 64);
        self.device_id = clean_id(&self.device_id, 64);
        self.transaction_id = clean_id(&self.transaction_id, 64);
        self.user_id = clean(&self.user_id, 64);
        self.status = self.event_type.status();
        let m = &mut self.metadata;
        for (field, max) in [
            (&mut m.hostname, 64),
            (&mut m.user_name, 64),
            (&mut m.app_version, 32),
            (&mut m.os, 64),
            (&mut m.os_version, 64),
            (&mut m.architecture, 32),
            (&mut m.local_ip, 45),
            (&mut m.mac_address, 17),
            (&mut m.document_type, 80),
            (&mut m.signature_type, 80),
            (&mut m.signer, 120),
            (&mut m.error_code, 32),
            (&mut m.previous_version, 32),
            (&mut m.note, 200),
        ] {
            *field = clean(field, max);
        }
        m.error_message = safe_error_message(&m.error_message);
        m.documents.truncate(MAX_LISTED);
        for name in &mut m.documents {
            *name = clean(name, 120);
        }
        m.failures.truncate(MAX_LISTED);
        for f in &mut m.failures {
            f.name = clean(&f.name, 120);
            f.code = clean(&f.code, 32);
            f.message = safe_error_message(&f.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_match_codes_and_status_follows_type() {
        for t in EventType::ALL {
            assert_eq!(
                serde_json::to_value(t).unwrap(),
                serde_json::Value::String(t.code().into())
            );
            assert_eq!(t.is_failure(), t.status() != Status::Success);
        }
        assert_eq!(EventType::SignCancelled.status(), Status::Cancelled);
        assert_eq!(EventType::DocumentFailed.category(), Category::Document);
        assert_eq!(
            serde_json::to_string(&Status::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn sanitize_clamps_and_strips_paths() {
        let mut m = Metadata {
            error_message: r"Không ghi được C:\Users\an\bi mat\a.pdf: đầy đĩa".into(),
            hostname: "MAY\r\nTINH".into(),
            ..Metadata::default()
        };
        m.set_documents((0..25).map(|i| format!("vb {i}.pdf")));
        let mut e = Event {
            event_id: "abc<script>".into(),
            event_type: EventType::SignFailed,
            device_id: "SMKS-1".into(),
            user_id: "an\tnv".into(),
            timestamp: 1,
            transaction_id: "KS 1".into(),
            status: Status::Success,
            metadata: m,
        };
        e.sanitize();
        assert_eq!(e.event_id, "abcscript");
        assert_eq!(e.transaction_id, "KS1");
        assert_eq!(e.user_id, "an nv");
        assert_eq!(e.status, Status::Failed, "status luôn theo loại sự kiện");
        assert_eq!(e.metadata.hostname, "MAY TINH");
        assert_eq!(e.metadata.error_message, "Không ghi được a.pdf: đầy đĩa");
        assert_eq!(e.metadata.documents.len(), MAX_LISTED);
        assert_eq!(e.metadata.document_count, 25);
        // Trường rỗng không ra JSON; đọc lại được.
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("mac_address") && !json.contains("failures"));
        assert_eq!(serde_json::from_str::<Event>(&json).unwrap(), e);
    }
}
