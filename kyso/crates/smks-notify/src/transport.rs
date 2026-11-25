//! Kênh gửi thông báo và phân loại lỗi gửi.

use std::time::Duration;

use crate::event::Event;

/// Lỗi gửi. Cách xử lý của hàng đợi phụ thuộc loại lỗi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    /// Không tới được máy chủ (mất mạng, hết thời gian chờ, máy chủ lỗi 5xx): thử lại sau.
    Network(String),
    /// Máy chủ từ chối vì cấu hình (Bot Token sai, bot không ở trong nhóm, khoá đăng ký sai, thiết bị bị thu hồi):
    /// thử lại sau, để tự gửi được khi quản trị viên sửa cấu hình.
    Config(String),
    /// Gateway chưa biết thiết bị này (hoặc bí mật thiết bị không còn đúng): cần đăng ký lại rồi gửi.
    Unregistered(String),
    /// Máy chủ yêu cầu chờ (HTTP 429): số giây, lý do.
    RateLimited(i64, String),
    /// Riêng tin này bị từ chối (nội dung không hợp lệ): bỏ, không thử lại.
    Rejected(String),
}

impl SendError {
    pub fn message(&self) -> &str {
        match self {
            Self::Network(m)
            | Self::Config(m)
            | Self::Unregistered(m)
            | Self::RateLimited(_, m)
            | Self::Rejected(m) => m,
        }
    }

    /// Nhãn ngắn cho nhật ký.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Network(_) => "network",
            Self::Config(_) => "config",
            Self::Unregistered(_) => "unregistered",
            Self::RateLimited(..) => "rate-limited",
            Self::Rejected(_) => "rejected",
        }
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// Ngữ cảnh của một lần gửi.
#[derive(Debug, Clone, Copy, Default)]
pub struct SendContext<'a> {
    /// Bí mật thiết bị do gateway cấp (chế độ gateway).
    pub credential: Option<&'a str>,
    /// Với sự kiện đăng ký ở chế độ gateway: có báo lên Telegram không.
    pub announce: bool,
}

/// Kết quả gửi thành công.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Delivery {
    /// Bí mật thiết bị mới do gateway cấp khi đăng ký.
    pub credential: Option<String>,
}

/// Một kênh gửi. Mọi lời gọi đều chặn (chạy trên luồng nền) và có thời gian chờ.
pub trait Transport: Send + Sync {
    /// Gửi một sự kiện.
    fn send(&self, event: &Event, context: &SendContext<'_>) -> Result<Delivery, SendError>;

    /// Kiểm tra kết nối mà không gửi thông tin thiết bị; trả về mô tả ngắn để hiện trên giao diện.
    fn check(&self, context: &SendContext<'_>) -> Result<String, SendError>;
}

/// Bộ gọi HTTP dùng chung: có thời gian chờ, TLS của hệ điều hành, mã HTTP lỗi không thành lỗi Rust (để đọc được
/// thân phản hồi).
pub(crate) fn agent(timeout: Duration) -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .new_agent()
}

/// Phản hồi HTTP đã đọc hết: mã trạng thái và thân dạng JSON (`Null` nếu không phải JSON).
pub(crate) struct Reply {
    pub status: u16,
    pub json: serde_json::Value,
}

/// POST JSON (hoặc GET khi `body` là `None`). Lỗi trả về là chuỗi **chưa** che bí mật: bên gọi phải che trước khi
/// dùng.
pub(crate) fn call(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&serde_json::Value>,
) -> Result<Reply, String> {
    let result = match body {
        Some(json) => {
            let mut request = agent.post(url).header("Content-Type", "application/json");
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.send(json.to_string().as_bytes())
        }
        None => {
            let mut request = agent.get(url);
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.call()
        }
    };
    let mut response = result.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(256 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    Ok(Reply {
        status,
        json: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    })
}
