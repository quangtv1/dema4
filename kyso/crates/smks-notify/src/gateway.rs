//! Máy trạm gọi Notification Gateway (`smks-gateway`).
//!
//! Ở chế độ này máy trạm **không có Bot Token**. Máy chủ giữ token trong biến môi trường, tự soạn nội dung tin
//! nhắn từ sự kiện, và chỉ nhận sự kiện của thiết bị đã đăng ký:
//!
//! - `POST /v1/devices/register` với khoá đăng ký (`X-Enroll-Key`) → gateway cấp **bí mật riêng của thiết bị**;
//! - `POST /v1/events` với `Authorization: Bearer <device_id>:<bí mật>`;
//! - `GET /v1/health` để kiểm tra kết nối.
//!
//! Quản trị viên thu hồi một thiết bị ở máy chủ mà không ảnh hưởng thiết bị khác; đổi Bot Token ở máy chủ không cần
//! động tới máy trạm.

use std::time::Duration;

use serde_json::json;

use crate::event::{Event, EventType};
use crate::text::{clean, redact};
use crate::transport::{Delivery, SendContext, SendError, Transport, agent, call};

pub const PATH_REGISTER: &str = "/v1/devices/register";
pub const PATH_EVENTS: &str = "/v1/events";
pub const PATH_HEALTH: &str = "/v1/health";
pub const HEADER_ENROLL: &str = "X-Enroll-Key";

pub struct GatewayClient {
    base: String,
    enroll_key: String,
    device_id: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for GatewayClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayClient")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl GatewayClient {
    pub fn new(base: &str, enroll_key: &str, device_id: &str, timeout: Duration) -> Self {
        Self {
            base: base.trim_end_matches('/').to_owned(),
            enroll_key: enroll_key.to_owned(),
            device_id: device_id.to_owned(),
            agent: agent(timeout),
        }
    }

    fn bearer(&self, credential: &str) -> String {
        format!("Bearer {}:{credential}", self.device_id)
    }

    /// Gọi gateway và phân loại phản hồi. `secrets`: các chuỗi phải che khỏi thông báo lỗi.
    fn request(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&serde_json::Value>,
        secrets: &[&str],
    ) -> Result<serde_json::Value, SendError> {
        let url = format!("{}{path}", self.base);
        let reply = call(&self.agent, &url, headers, body).map_err(|e| {
            SendError::Network(format!(
                "Không kết nối được gateway: {}",
                clean(&redact(&e, secrets), 300)
            ))
        })?;
        if reply.status == 200 && reply.json["ok"] == true {
            return Ok(reply.json);
        }
        let reason = clean(
            &redact(reply.json["error"].as_str().unwrap_or(""), secrets),
            200,
        );
        let detail = if reason.is_empty() {
            format!("HTTP {}", reply.status)
        } else {
            format!("HTTP {}: {reason}", reply.status)
        };
        Err(match reply.status {
            401 => SendError::Unregistered(format!("Gateway chưa nhận thiết bị này ({detail}).")),
            403 => SendError::Config(format!(
                "Gateway từ chối thiết bị ({detail}). Thiết bị đã bị thu hồi, hoặc khoá đăng ký không đúng."
            )),
            429 => SendError::RateLimited(
                reply.json["retry_after"].as_i64().unwrap_or(60),
                format!("Gateway yêu cầu gửi chậm lại ({detail})."),
            ),
            400 | 413 | 422 => SendError::Rejected(format!("Gateway từ chối sự kiện ({detail}).")),
            404 => SendError::Config(format!(
                "Địa chỉ gateway không đúng ({detail}). Kiểm tra GATEWAY_URL."
            )),
            _ => SendError::Network(format!("Gateway chưa gửi được ({detail}).")),
        })
    }
}

impl Transport for GatewayClient {
    fn send(&self, event: &Event, context: &SendContext<'_>) -> Result<Delivery, SendError> {
        if event.event_type == EventType::DeviceRegistered {
            let body = json!({ "event": event, "announce": context.announce });
            let reply = self.request(
                PATH_REGISTER,
                &[(HEADER_ENROLL, &self.enroll_key)],
                Some(&body),
                &[&self.enroll_key],
            )?;
            let secret = reply["device_secret"].as_str().unwrap_or("");
            if secret.is_empty() {
                return Err(SendError::Network(
                    "Gateway không trả bí mật thiết bị khi đăng ký.".into(),
                ));
            }
            return Ok(Delivery {
                credential: Some(secret.to_owned()),
            });
        }
        let Some(credential) = context.credential else {
            return Err(SendError::Unregistered(
                "Thiết bị chưa đăng ký với gateway.".into(),
            ));
        };
        let body = serde_json::to_value(event)
            .map_err(|e| SendError::Rejected(format!("Không đóng gói được sự kiện: {e}")))?;
        self.request(
            PATH_EVENTS,
            &[("Authorization", &self.bearer(credential))],
            Some(&body),
            &[credential, &self.enroll_key],
        )?;
        Ok(Delivery::default())
    }

    fn check(&self, context: &SendContext<'_>) -> Result<String, SendError> {
        let auth = context.credential.map(|c| self.bearer(c));
        let headers: Vec<(&str, &str)> = auth
            .as_deref()
            .map(|a| vec![("Authorization", a)])
            .unwrap_or_default();
        let secrets: Vec<&str> = context
            .credential
            .into_iter()
            .chain([self.enroll_key.as_str()])
            .collect();
        let reply = self.request(PATH_HEALTH, &headers, None, &secrets)?;
        let telegram = reply["telegram"].as_str().unwrap_or("?");
        if telegram != "ok" {
            return Err(SendError::Config(format!(
                "Gateway hoạt động nhưng chưa gửi được tới Telegram: {}",
                clean(telegram, 200)
            )));
        }
        let device = match reply["device"].as_str() {
            Some("registered") => "thiết bị đã đăng ký",
            Some("revoked") => "thiết bị đã bị thu hồi",
            _ => "thiết bị chưa đăng ký",
        };
        Ok(format!("Gateway hoạt động, đã nối tới Telegram; {device}."))
    }
}
