//! Thông báo hoạt động của Sao Mai Ký số về hệ thống quản trị (Telegram).
//!
//! Crate này độc lập với nghiệp vụ ký số và với Tauri. Ứng dụng chỉ gọi [`service::Notifier::emit`] ở nơi nghiệp
//! vụ **đã kết thúc**; mọi việc còn lại (ghi nhật ký sự kiện, xét quyền gửi, xếp hàng, gửi lại, chống trùng) nằm ở
//! đây và chạy trên luồng nền, nên Telegram hỏng hay mất mạng không ảnh hưởng tới việc ký.
//!
//! - [`event`]: kiểu sự kiện thống nhất (`event_id`, `event_type`, `device_id`, `user_id`, `timestamp`,
//!   `transaction_id`, `status`, `metadata`).
//! - [`device`]: mã thiết bị (ngẫu nhiên, không lấy từ phần cứng) và thông tin thiết bị ở mức tối thiểu.
//! - [`template`]: nội dung tin nhắn Telegram (HTML) cho từng loại sự kiện.
//! - [`config`]: cấu hình quản trị `notify.json` đặt **ngoài** file chạy (Bot Token không nằm trong EXE).
//! - [`state`]: trạng thái trên máy (đồng ý / từ chối, đã đăng ký chưa, tuỳ chọn bật tắt).
//! - [`queue`]: hàng đợi lưu đĩa, giới hạn số lần thử, chống trùng.
//! - [`transport`], [`telegram`], [`gateway`]: kênh gửi (trực tiếp Bot API, hoặc qua Notification Gateway).
//! - [`audit`]: nhật ký sự kiện và trạng thái gửi (JSONL, không chứa nội dung văn bản hay bí mật).
//! - [`service`]: ghép các phần trên thành [`service::Notifier`].
//!
//! Không bao giờ đưa vào sự kiện: nội dung văn bản, file PDF, khoá bí mật, mã PIN, mật khẩu, dữ liệu chữ ký.

pub mod audit;
pub mod config;
pub mod device;
pub mod event;
pub mod gateway;
pub mod queue;
pub mod service;
pub mod state;
pub mod telegram;
pub mod template;
pub mod text;
pub mod time;
pub mod transport;

pub use event::{Event, EventType, Failure, Metadata, Status};
pub use service::{Draft, Notifier};
