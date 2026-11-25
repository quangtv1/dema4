//! Gọi Telegram Bot API (`sendMessage`, `getMe`, `getChat`).
//!
//! Bot Token chỉ xuất hiện trong URL của lời gọi. Mọi chuỗi lỗi đi ra khỏi module này đều đã được che token
//! ([`crate::text::redact`]), và `Debug` của [`TelegramClient`] không in token.

use std::time::Duration;

use serde_json::json;

use crate::event::Event;
use crate::template;
use crate::text::{clean, redact};
use crate::transport::{Delivery, SendContext, SendError, Transport, agent, call};

pub const DEFAULT_API_BASE: &str = "https://api.telegram.org";

pub struct TelegramClient {
    api_base: String,
    token: String,
    chat_id: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for TelegramClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramClient")
            .field("api_base", &self.api_base)
            .field("token", &crate::text::mask(&self.token))
            .finish_non_exhaustive()
    }
}

impl TelegramClient {
    /// `api_base` rỗng thì dùng `https://api.telegram.org`.
    pub fn new(api_base: &str, token: &str, chat_id: &str, timeout: Duration) -> Self {
        Self {
            api_base: if api_base.is_empty() {
                DEFAULT_API_BASE.to_owned()
            } else {
                api_base.trim_end_matches('/').to_owned()
            },
            token: token.to_owned(),
            chat_id: chat_id.to_owned(),
            agent: agent(timeout),
        }
    }

    fn safe(&self, text: &str) -> String {
        clean(&redact(text, &[&self.token]), 300)
    }

    /// Gọi một phương thức của Bot API; trả về trường `result`.
    fn method(&self, name: &str, body: &serde_json::Value) -> Result<serde_json::Value, SendError> {
        let url = format!("{}/bot{}/{name}", self.api_base, self.token);
        let reply = call(&self.agent, &url, &[], Some(body)).map_err(|e| {
            SendError::Network(format!("Không kết nối được Telegram: {}", self.safe(&e)))
        })?;
        if reply.status == 200 && reply.json["ok"] == true {
            return Ok(reply.json["result"].clone());
        }
        let description = self.safe(reply.json["description"].as_str().unwrap_or(""));
        let detail = if description.is_empty() {
            format!("HTTP {}", reply.status)
        } else {
            format!("HTTP {}: {description}", reply.status)
        };
        let lower = description.to_lowercase();
        Err(match reply.status {
            429 => SendError::RateLimited(
                reply.json["parameters"]["retry_after"]
                    .as_i64()
                    .unwrap_or(30),
                format!("Telegram yêu cầu gửi chậm lại ({detail})."),
            ),
            401 | 404 => SendError::Config(format!(
                "Telegram từ chối Bot Token ({detail}). Kiểm tra TELEGRAM_BOT_TOKEN; token có thể đã bị thu hồi."
            )),
            403 => SendError::Config(format!(
                "Bot không được phép gửi vào nhóm này ({detail}). Thêm bot vào nhóm và kiểm tra TELEGRAM_CHAT_ID."
            )),
            400 if lower.contains("chat not found")
                || lower.contains("chat_id")
                || lower.contains("upgraded to a supergroup")
                || lower.contains("kicked")
                || lower.contains("deactivated") =>
            {
                SendError::Config(format!(
                    "Telegram không tìm thấy nhóm nhận tin ({detail}). Kiểm tra TELEGRAM_CHAT_ID và bot đã ở trong nhóm chưa."
                ))
            }
            400..=499 => SendError::Rejected(format!("Telegram từ chối tin nhắn ({detail}).")),
            _ => SendError::Network(format!("Telegram đang lỗi ({detail}).")),
        })
    }

    /// Gửi một tin HTML vào nhóm đã cấu hình.
    pub fn send_message(&self, html: &str) -> Result<(), SendError> {
        self.method(
            "sendMessage",
            &json!({
                "chat_id": self.chat_id,
                "text": html,
                "parse_mode": "HTML",
                "disable_web_page_preview": true,
            }),
        )
        .map(|_| ())
    }

    /// Tên bot (`@ten_bot`): kiểm tra Bot Token.
    pub fn bot_name(&self) -> Result<String, SendError> {
        let me = self.method("getMe", &json!({}))?;
        Ok(format!(
            "@{}",
            clean(me["username"].as_str().unwrap_or("?"), 64)
        ))
    }

    /// Tên nhóm nhận tin: kiểm tra Chat ID và bot có trong nhóm.
    pub fn chat_title(&self) -> Result<String, SendError> {
        let chat = self.method("getChat", &json!({ "chat_id": self.chat_id }))?;
        let title = chat["title"]
            .as_str()
            .or_else(|| chat["username"].as_str())
            .or_else(|| chat["first_name"].as_str())
            .unwrap_or("?");
        Ok(clean(title, 80))
    }
}

impl Transport for TelegramClient {
    fn send(&self, event: &Event, _context: &SendContext<'_>) -> Result<Delivery, SendError> {
        self.send_message(&template::render(event))?;
        Ok(Delivery::default())
    }

    fn check(&self, _context: &SendContext<'_>) -> Result<String, SendError> {
        let bot = self.bot_name()?;
        let chat = self.chat_title()?;
        Ok(format!("Bot {bot} gửi được vào nhóm «{chat}»."))
    }
}
