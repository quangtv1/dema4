//! Gọi mô hình đọc ảnh — 5 nhà cung cấp như prototype, 2 giao thức:
//! - chuẩn OpenAI `POST {địa chỉ}/chat/completions` (Nội bộ vLLM / Ollama…, ChatGPT, Gemini qua địa chỉ tương thích OpenAI, OpenRouter);
//! - Anthropic `POST {địa chỉ}/messages` (Claude).
//! Mỗi yêu cầu đúng 1 ảnh trang (máy chủ vLLM nhỏ chỉ nhận 1 ảnh / yêu cầu).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NhaCungCap {
    NoiBo,
    ChatGpt,
    Gemini,
    Claude,
    OpenRouter,
}

impl NhaCungCap {
    /// Thứ tự ô trong Cấu hình › Bóc tách.
    pub const TAT_CA: [NhaCungCap; 5] = [NhaCungCap::NoiBo, NhaCungCap::ChatGpt, NhaCungCap::Gemini, NhaCungCap::Claude, NhaCungCap::OpenRouter];

    pub fn ten(self) -> &'static str {
        match self {
            NhaCungCap::NoiBo => "Nội bộ",
            NhaCungCap::ChatGpt => "ChatGPT",
            NhaCungCap::Gemini => "Gemini",
            NhaCungCap::Claude => "Claude",
            NhaCungCap::OpenRouter => "OpenRouter",
        }
    }

    /// Mã dùng trong file cấu hình / file khoá.
    pub fn ma(self) -> &'static str {
        match self {
            NhaCungCap::NoiBo => "noi_bo",
            NhaCungCap::ChatGpt => "chat_gpt",
            NhaCungCap::Gemini => "gemini",
            NhaCungCap::Claude => "claude",
            NhaCungCap::OpenRouter => "open_router",
        }
    }

    pub fn tu_ma(ma: &str) -> Option<Self> {
        Self::TAT_CA.into_iter().find(|n| n.ma() == ma)
    }

    pub fn chi_so(self) -> usize {
        Self::TAT_CA.iter().position(|n| *n == self).unwrap_or(0)
    }

    /// Nơi lấy khoá API (dòng gợi ý dưới ô Khoá API).
    pub fn noi_lay_khoa(self) -> &'static str {
        match self {
            NhaCungCap::NoiBo => "",
            NhaCungCap::ChatGpt => "platform.openai.com › API keys",
            NhaCungCap::Gemini => "aistudio.google.com › Get API key",
            NhaCungCap::Claude => "console.anthropic.com › API keys",
            NhaCungCap::OpenRouter => "openrouter.ai › Keys",
        }
    }

    /// Tên công ty cho câu cảnh báo gửi ảnh ra ngoài.
    pub fn cong_ty(self) -> &'static str {
        match self {
            NhaCungCap::NoiBo => "máy chủ nội bộ",
            NhaCungCap::ChatGpt => "OpenAI",
            NhaCungCap::Gemini => "Google",
            NhaCungCap::Claude => "Anthropic",
            NhaCungCap::OpenRouter => "OpenRouter",
        }
    }

    /// Địa chỉ cố định của dịch vụ đám mây; Nội bộ do người dùng nhập (`http://<ip máy GPU>:<cổng>/v1`).
    pub fn dia_chi_mac_dinh(self) -> &'static str {
        match self {
            NhaCungCap::NoiBo => "",
            NhaCungCap::ChatGpt => "https://api.openai.com/v1",
            NhaCungCap::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai",
            NhaCungCap::Claude => "https://api.anthropic.com/v1",
            NhaCungCap::OpenRouter => "https://openrouter.ai/api/v1",
        }
    }

    pub fn la_dam_may(self) -> bool {
        self != NhaCungCap::NoiBo
    }

    /// Số yêu cầu song song mặc định: máy nội bộ thử nghiệm yếu → 1; dịch vụ đám mây 2 (gói miễn phí giới hạn yêu cầu / phút).
    pub fn song_song_mac_dinh(self) -> usize {
        if self.la_dam_may() { 2 } else { 1 }
    }

    pub fn cho_mac_dinh_s(self) -> u64 {
        if self.la_dam_may() { 60 } else { 120 }
    }
}

/// Thông số gọi 1 mô hình (lấy từ Cấu hình › Bóc tách).
#[derive(Debug, Clone, PartialEq)]
pub struct KetNoi {
    pub ncc: NhaCungCap,
    pub dia_chi: String,
    pub khoa: String,
    pub model: String,
    pub cho_toi_da: Duration,
    pub max_tokens: u32,
}

impl KetNoi {
    pub fn moi(ncc: NhaCungCap, dia_chi: &str, khoa: &str, model: &str) -> Self {
        let dia_chi = if dia_chi.trim().is_empty() { ncc.dia_chi_mac_dinh() } else { dia_chi.trim() };
        Self { ncc, dia_chi: dia_chi.into(), khoa: khoa.trim().into(), model: model.trim().into(), cho_toi_da: Duration::from_secs(ncc.cho_mac_dinh_s()), max_tokens: 2048 }
    }

    fn url(&self, duoi: &str) -> String {
        format!("{}/{duoi}", self.dia_chi.trim_end_matches('/'))
    }

    fn agent(&self) -> ureq::Agent {
        ureq::Agent::config_builder().timeout_global(Some(self.cho_toi_da)).http_status_as_error(false).build().into()
    }

    fn header<B>(&self, mut rq: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        if self.ncc == NhaCungCap::Claude {
            rq = rq.header("x-api-key", &self.khoa).header("anthropic-version", "2023-06-01");
        } else if !self.khoa.is_empty() {
            rq = rq.header("Authorization", &format!("Bearer {}", self.khoa));
        }
        rq
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoiDoc {
    /// Không kết nối được / hết thời gian chờ.
    Mang(String),
    /// 401 / 403: dừng cả hàng đợi.
    Khoa(String),
    /// 429 / 503 / 529: chờ rồi thử lại (`cho` từ header `Retry-After`).
    QuaTai { cho: Option<Duration> },
    /// 400 nhắc tới ảnh: model / máy chủ không nhận ảnh này.
    AnhKhongNhan(String),
    /// Mã HTTP khác.
    May(u16, String),
    /// Trả lời không đọc được (không có nội dung, không có JSON…).
    TraLoi(String),
}

impl std::fmt::Display for LoiDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoiDoc::Mang(e) => write!(f, "Không kết nối được mô hình đọc: {e}"),
            LoiDoc::Khoa(e) => write!(f, "Khoá API sai hoặc hết quyền ({e})"),
            LoiDoc::QuaTai { .. } => write!(f, "Dịch vụ đang quá tải / vượt giới hạn yêu cầu, thử lại sau"),
            LoiDoc::AnhKhongNhan(e) => write!(f, "Máy chủ / model không nhận ảnh này: {e}"),
            LoiDoc::May(m, e) => write!(f, "Máy chủ báo lỗi {m}: {e}"),
            LoiDoc::TraLoi(e) => write!(f, "{e}"),
        }
    }
}

/// Mô hình đọc 1 ảnh trang + câu lệnh → câu trả lời chữ (thường là JSON). Test thay bằng mô hình giả.
pub trait MoHinhDoc: Send + Sync {
    fn doc(&self, cau_lenh: &str, jpeg: &[u8]) -> Result<String, LoiDoc>;
}

impl MoHinhDoc for KetNoi {
    fn doc(&self, cau_lenh: &str, jpeg: &[u8]) -> Result<String, LoiDoc> {
        let b64 = super::anh::base64(jpeg);
        if self.ncc == NhaCungCap::Claude {
            let than = json!({
                "model": self.model, "max_tokens": self.max_tokens, "temperature": 0,
                "messages": [{ "role": "user", "content": [
                    { "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": b64 } },
                    { "type": "text", "text": cau_lenh },
                ]}],
            });
            return noi_dung_anthropic(&self.post("messages", &than)?);
        }
        let mut than = json!({
            "model": self.model, "temperature": 0,
            "messages": [{ "role": "user", "content": [
                { "type": "text", "text": cau_lenh },
                { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{b64}") } },
            ]}],
        });
        // Model OpenAI đời mới chỉ nhận max_completion_tokens; các máy chủ tương thích khác dùng max_tokens
        let khoa_token = if self.ncc == NhaCungCap::ChatGpt { "max_completion_tokens" } else { "max_tokens" };
        than[khoa_token] = json!(self.max_tokens);
        match self.post("chat/completions", &than) {
            // Một số model (GPT-5, o-series) chỉ nhận temperature mặc định → gửi lại không có temperature
            Err(LoiDoc::May(400, e)) if e.contains("temperature") => {
                than.as_object_mut().map(|o| o.remove("temperature"));
                noi_dung_openai(&self.post("chat/completions", &than)?)
            }
            kq => noi_dung_openai(&kq?),
        }
    }
}

impl KetNoi {
    fn post(&self, duoi: &str, than: &Value) -> Result<Value, LoiDoc> {
        let rq = self.header(self.agent().post(&self.url(duoi)).header("Content-Type", "application/json"));
        tra_loi(rq.send(than.to_string()))
    }

    /// Danh sách model (`GET /models`) — dùng cho ô chọn Model và Kiểm tra kết nối.
    pub fn liet_ke_model(&self) -> Result<Vec<String>, LoiDoc> {
        let v = tra_loi(self.header(self.agent().get(&self.url("models"))).call())?;
        let mut ds: Vec<String> = v["data"]
            .as_array()
            .or(v["models"].as_array())
            .map(|a| a.iter().filter_map(|m| m["id"].as_str().or(m["name"].as_str())).map(|s| s.trim_start_matches("models/").to_string()).collect())
            .unwrap_or_default();
        ds.sort();
        ds.dedup();
        Ok(ds)
    }
}

/// Mã HTTP → lỗi / JSON trả về.
fn tra_loi(kq: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, LoiDoc> {
    let mut ph = kq.map_err(|e| LoiDoc::Mang(e.to_string()))?;
    let ma = ph.status().as_u16();
    let cho = ph.headers().get("retry-after").and_then(|h| h.to_str().ok()).and_then(|s| s.trim().parse::<u64>().ok()).map(Duration::from_secs);
    let than = ph.body_mut().read_to_string().map_err(|e| LoiDoc::Mang(e.to_string()))?;
    let v: Value = serde_json::from_str(&than).unwrap_or(Value::Null);
    let thong_bao = || {
        let m = v["error"]["message"].as_str().or(v["message"].as_str()).or(v["error"].as_str()).map(str::to_string);
        m.unwrap_or_else(|| than.chars().take(200).collect())
    };
    match ma {
        200..=299 if !v.is_null() => Ok(v),
        200..=299 => Err(LoiDoc::TraLoi("Máy chủ trả về không phải JSON".into())),
        401 | 403 => Err(LoiDoc::Khoa(thong_bao())),
        429 | 503 | 529 => Err(LoiDoc::QuaTai { cho }),
        400 | 422 if thong_bao().to_lowercase().contains("image") => Err(LoiDoc::AnhKhongNhan(thong_bao())),
        _ => Err(LoiDoc::May(ma, thong_bao())),
    }
}

fn noi_dung_openai(v: &Value) -> Result<String, LoiDoc> {
    let c = &v["choices"][0];
    let nd = &c["message"]["content"];
    let s = match nd {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""),
        _ => String::new(),
    };
    het_token(s, c["finish_reason"].as_str() == Some("length"))
}

fn noi_dung_anthropic(v: &Value) -> Result<String, LoiDoc> {
    let s = v["content"].as_array().map(|a| a.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
    het_token(s, v["stop_reason"].as_str() == Some("max_tokens"))
}

fn het_token(s: String, het: bool) -> Result<String, LoiDoc> {
    if !s.trim().is_empty() {
        Ok(s)
    } else if het {
        Err(LoiDoc::TraLoi("Mô hình dùng hết số token tối đa trước khi trả lời — tăng Số token tối đa trong Cấu hình".into()))
    } else {
        Err(LoiDoc::TraLoi("Mô hình không trả nội dung".into()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::mpsc;

    /// Máy chủ HTTP giả: trả lần lượt các phản hồi (mã, header, thân), gửi lại nội dung mỗi yêu cầu nhận được.
    pub(crate) fn may_chu_gia(phan_hoi: Vec<(u16, Vec<(&'static str, &'static str)>, String)>) -> (String, mpsc::Receiver<String>) {
        let nghe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dia_chi = format!("http://{}/v1", nghe.local_addr().unwrap());
        let (gui, nhan) = mpsc::channel();
        std::thread::spawn(move || {
            for (ma, header, than) in phan_hoi {
                let Ok((mut k, _)) = nghe.accept() else { return };
                let mut r = BufReader::new(k.try_clone().unwrap());
                let (mut yc, mut dai) = (String::new(), 0usize);
                loop {
                    let mut dong = String::new();
                    if r.read_line(&mut dong).unwrap_or(0) == 0 || dong == "\r\n" {
                        break;
                    }
                    if let Some(v) = dong.to_lowercase().strip_prefix("content-length:") {
                        dai = v.trim().parse().unwrap_or(0);
                    }
                    yc += &dong;
                }
                let mut b = vec![0; dai];
                r.read_exact(&mut b).unwrap();
                yc += "\r\n";
                yc += &String::from_utf8_lossy(&b);
                let _ = gui.send(yc);
                let them: String = header.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
                let _ = write!(k, "HTTP/1.1 {ma} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{them}\r\n{than}", than.len());
            }
        });
        (dia_chi, nhan)
    }

    fn openai_ok(noi_dung: &str) -> String {
        json!({ "choices": [{ "message": { "content": noi_dung }, "finish_reason": "stop" }] }).to_string()
    }

    #[test]
    fn noi_bo_gui_1_anh_khong_khoa() {
        let (dc, nhan) = may_chu_gia(vec![(200, vec![], openai_ok("{\"a\":\"1\"}"))]);
        let kn = KetNoi::moi(NhaCungCap::NoiBo, &dc, "", "5CD-AI/Vintern-1B-v3_5");
        assert_eq!(kn.doc("Câu lệnh", &[0xFF, 0xD8, 1]).unwrap(), "{\"a\":\"1\"}");
        let yc = nhan.recv().unwrap();
        assert!(yc.starts_with("POST /v1/chat/completions"));
        assert!(!yc.to_lowercase().contains("authorization:"), "không khoá → không gửi Authorization");
        let than: Value = serde_json::from_str(&yc[yc.find("\r\n\r\n").unwrap() + 4..]).unwrap();
        let nd = than["messages"][0]["content"].as_array().unwrap();
        assert_eq!(nd.iter().filter(|p| p["type"] == "image_url").count(), 1);
        assert!(nd[1]["image_url"]["url"].as_str().unwrap().starts_with("data:image/jpeg;base64,/9gB"));
        assert_eq!(than["temperature"], 0);
        assert_eq!(than["max_tokens"], 2048);
    }

    #[test]
    fn claude_dung_giao_thuc_anthropic() {
        let tl = json!({ "content": [{ "type": "text", "text": "{\"b\":\"2\"}" }], "stop_reason": "end_turn" }).to_string();
        let (dc, nhan) = may_chu_gia(vec![(200, vec![], tl)]);
        let kn = KetNoi::moi(NhaCungCap::Claude, &dc, "khoa-thu", "claude-haiku-4-5");
        assert_eq!(kn.doc("CL", &[1, 2]).unwrap(), "{\"b\":\"2\"}");
        let yc = nhan.recv().unwrap();
        assert!(yc.starts_with("POST /v1/messages"));
        assert!(yc.contains("x-api-key: khoa-thu") && yc.contains("anthropic-version: 2023-06-01"));
        assert!(yc.contains("\"media_type\":\"image/jpeg\""));
    }

    #[test]
    fn chatgpt_bo_temperature_khi_model_khong_nhan() {
        let loi = json!({ "error": { "message": "Unsupported value: 'temperature' does not support 0" } }).to_string();
        let (dc, nhan) = may_chu_gia(vec![(400, vec![], loi), (200, vec![], openai_ok("ok"))]);
        let kn = KetNoi::moi(NhaCungCap::ChatGpt, &dc, "k", "gpt-5-mini");
        assert_eq!(kn.doc("CL", &[1]).unwrap(), "ok");
        let (y1, y2) = (nhan.recv().unwrap(), nhan.recv().unwrap());
        assert!(y1.to_lowercase().contains("authorization: bearer k") && y1.contains("max_completion_tokens") && y1.contains("temperature"));
        assert!(!y2.contains("temperature"));
    }

    #[test]
    fn ma_loi_thanh_loai_loi() {
        let (dc, _n) = may_chu_gia(vec![
            (401, vec![], json!({ "error": { "message": "API key not valid" } }).to_string()),
            (429, vec![("Retry-After", "7")], "{}".into()),
            (400, vec![], json!({ "object": "error", "message": "At most 1 image(s) may be provided in one request." }).to_string()),
            (500, vec![], "lỗi máy".into()),
            (200, vec![], json!({ "choices": [{ "message": { "content": "" }, "finish_reason": "length" }] }).to_string()),
        ]);
        let kn = KetNoi::moi(NhaCungCap::Gemini, &dc, "k", "gemini-2.5-flash");
        assert_eq!(kn.doc("", &[]), Err(LoiDoc::Khoa("API key not valid".into())));
        assert_eq!(kn.doc("", &[]), Err(LoiDoc::QuaTai { cho: Some(Duration::from_secs(7)) }));
        assert!(matches!(kn.doc("", &[]), Err(LoiDoc::AnhKhongNhan(e)) if e.contains("At most 1 image")));
        assert_eq!(kn.doc("", &[]), Err(LoiDoc::May(500, "lỗi máy".into())));
        assert!(matches!(kn.doc("", &[]), Err(LoiDoc::TraLoi(e)) if e.contains("token")));
    }

    #[test]
    fn liet_ke_model_bo_tien_to_gemini() {
        let ds = json!({ "data": [{ "id": "models/gemini-2.5-flash" }, { "id": "models/gemini-2.5-pro" }] }).to_string();
        let (dc, nhan) = may_chu_gia(vec![(200, vec![], ds)]);
        let kn = KetNoi::moi(NhaCungCap::Gemini, &dc, "k", "");
        assert_eq!(kn.liet_ke_model().unwrap(), ["gemini-2.5-flash", "gemini-2.5-pro"]);
        assert!(nhan.recv().unwrap().starts_with("GET /v1/models"));
    }

    #[test]
    fn khong_ket_noi_duoc() {
        let mut kn = KetNoi::moi(NhaCungCap::NoiBo, "http://127.0.0.1:9/v1", "", "m");
        kn.cho_toi_da = Duration::from_secs(2);
        assert!(matches!(kn.doc("", &[]), Err(LoiDoc::Mang(_))));
    }

    #[test]
    fn dia_chi_mac_dinh_dich_vu_dam_may() {
        assert_eq!(KetNoi::moi(NhaCungCap::Gemini, "", "k", "m").dia_chi, "https://generativelanguage.googleapis.com/v1beta/openai");
        assert_eq!(KetNoi::moi(NhaCungCap::NoiBo, " http://may:8001/v1/ ", "", "m").url("models"), "http://may:8001/v1/models");
    }
}
