//! Notification Gateway của Sao Mai Ký số.
//!
//! Máy trạm gửi sự kiện tới đây; gateway giữ Bot Token, kiểm tra thiết bị rồi chuyển thành tin nhắn Telegram.
//!
//! Cấu hình bằng **biến môi trường** (hoặc file `KEY=VALUE` qua `--env-file`, để ngoài git):
//!
//! | Biến | Ý nghĩa |
//! |---|---|
//! | `TELEGRAM_BOT_TOKEN` | Bot Token (bắt buộc) |
//! | `TELEGRAM_CHAT_ID` | Nhóm nhận tin (bắt buộc) |
//! | `GATEWAY_ENROLL_KEY` | Khoá đăng ký thiết bị, tối thiểu 16 ký tự (bắt buộc) |
//! | `GATEWAY_BIND` | Địa chỉ nghe, mặc định `127.0.0.1:8787` |
//! | `GATEWAY_DATA_DIR` | Thư mục lưu `devices.json`, mặc định `gateway-data` |
//! | `NOTIFICATION_ENABLED`, `DEVICE_REGISTRATION_ENABLED`, `SIGN_NOTIFICATION_ENABLED`, `DOCUMENT_NOTIFICATION_ENABLED`, `FAILURE_NOTIFICATION_ENABLED` | Bật / tắt từng loại, mặc định bật |
//! | `TELEGRAM_API_BASE` | Địa chỉ Bot API khác mặc định (tuỳ chọn) |
//!
//! Lệnh: `smks-gateway [--env-file <file>] [serve | check | devices | revoke <id> | restore <id> | remove <id>]`.
//!
//! Gateway nói HTTP thường; đặt sau reverse proxy có TLS (Caddy, nginx, IIS) khi mở ra ngoài mạng nội bộ.

mod server;

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use serde_json::json;
use smks_notify::gateway::{HEADER_ENROLL, PATH_EVENTS, PATH_HEALTH, PATH_REGISTER};
use smks_notify::state::Flags;
use smks_notify::telegram::TelegramClient;
use smks_notify::transport::{SendContext, Transport};
use tiny_http::{Header, Method, Request, Response, Server};

use server::{Gateway, MAX_BODY_BYTES, Store, log};

const WORKERS: usize = 4;

/// Giá trị đọc từ `--env-file` (biến môi trường thật được ưu tiên hơn).
static FILE_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

fn env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .or_else(|| FILE_ENV.get().and_then(|file| file.get(name).cloned()))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Công tắc: mặc định bật; `false`, `0`, `off`, `no` là tắt.
fn env_flag(name: &str) -> bool {
    !matches!(
        env(name).to_ascii_lowercase().as_str(),
        "false" | "0" | "off" | "no"
    )
}

/// Nạp file `KEY=VALUE` (dòng trống và dòng bắt đầu bằng `#` bị bỏ qua). Biến môi trường thật thắng giá trị trong
/// file.
fn load_env_file(path: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("không đọc được {}: {e}", path.display()))?;
    let mut values = HashMap::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            );
        }
    }
    let _ = FILE_ENV.set(values);
    Ok(())
}

fn data_dir() -> PathBuf {
    let dir = env("GATEWAY_DATA_DIR");
    PathBuf::from(if dir.is_empty() { "gateway-data" } else { &dir })
}

fn telegram() -> Result<TelegramClient, String> {
    let token = env("TELEGRAM_BOT_TOKEN");
    let chat = env("TELEGRAM_CHAT_ID");
    if token.is_empty() || chat.is_empty() {
        return Err("thiếu TELEGRAM_BOT_TOKEN hoặc TELEGRAM_CHAT_ID trong biến môi trường".into());
    }
    Ok(TelegramClient::new(
        &env("TELEGRAM_API_BASE"),
        &token,
        &chat,
        Duration::from_secs(15),
    ))
}

fn header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

fn handle(gateway: &Gateway, request: &mut Request) -> server::Reply {
    let path = request.url().split('?').next().unwrap_or("").to_owned();
    let method = request.method().clone();
    if method == Method::Get && path == PATH_HEALTH {
        return gateway.health(header(request, "Authorization"));
    }
    if method != Method::Post || (path != PATH_REGISTER && path != PATH_EVENTS) {
        return (404, json!({ "ok": false, "error": "không có địa chỉ này" }));
    }
    let mut body = Vec::new();
    let read = request
        .as_reader()
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut body);
    if read.is_err() {
        return (
            400,
            json!({ "ok": false, "error": "không đọc được thân yêu cầu" }),
        );
    }
    if body.len() > MAX_BODY_BYTES {
        return (413, json!({ "ok": false, "error": "thân yêu cầu quá lớn" }));
    }
    if path == PATH_REGISTER {
        gateway.register(header(request, HEADER_ENROLL), &body)
    } else {
        gateway.events(header(request, "Authorization"), &body)
    }
}

fn serve() -> Result<(), String> {
    let enroll_key = env("GATEWAY_ENROLL_KEY");
    if enroll_key.chars().count() < 16 {
        return Err("GATEWAY_ENROLL_KEY phải có ít nhất 16 ký tự".into());
    }
    let client = telegram()?;
    let flags = Flags {
        enabled: env_flag("NOTIFICATION_ENABLED"),
        device: env_flag("DEVICE_REGISTRATION_ENABLED"),
        sign: env_flag("SIGN_NOTIFICATION_ENABLED"),
        document: env_flag("DOCUMENT_NOTIFICATION_ENABLED"),
        failure: env_flag("FAILURE_NOTIFICATION_ENABLED"),
    };
    let dir = data_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("không tạo được thư mục dữ liệu {}: {e}", dir.display()))?;
    let store = Store::open(&dir);
    let bind = {
        let b = env("GATEWAY_BIND");
        if b.is_empty() {
            "127.0.0.1:8787".to_owned()
        } else {
            b
        }
    };
    let http = Server::http(&bind).map_err(|e| format!("không nghe được ở {bind}: {e}"))?;
    log(&format!(
        "smks-gateway {} nghe ở http://{bind}; dữ liệu ở {}; {} thiết bị; công tắc {flags:?}",
        env!("CARGO_PKG_VERSION"),
        dir.display(),
        store.devices.len()
    ));
    match client.check(&SendContext::default()) {
        Ok(detail) => log(&format!("Telegram: {detail}")),
        Err(e) => log(&format!("CẢNH BÁO: chưa gửi được tới Telegram: {e}")),
    }
    let gateway = Arc::new(Gateway::new(&enroll_key, flags, Arc::new(client), store));
    let http = Arc::new(http);
    let json_type = Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    )
    .ok();
    let mut workers = Vec::new();
    for index in 0..WORKERS {
        let http = http.clone();
        let gateway = gateway.clone();
        let json_type = json_type.clone();
        let worker = std::thread::Builder::new()
            .name(format!("gateway-{index}"))
            .spawn(move || {
                while let Ok(mut request) = http.recv() {
                    let (status, body) = handle(&gateway, &mut request);
                    let mut response =
                        Response::from_string(body.to_string()).with_status_code(status);
                    if let Some(h) = &json_type {
                        response.add_header(h.clone());
                    }
                    let _ = request.respond(response);
                }
            })
            .map_err(|e| format!("không tạo được luồng xử lý: {e}"))?;
        workers.push(worker);
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

/// Sửa `revoked` của một thiết bị, hoặc xoá hẳn (`None`).
fn set_revoked(device_id: &str, revoked: Option<bool>) -> Result<(), String> {
    let mut store = Store::open(&data_dir());
    match revoked {
        Some(flag) => {
            let device = store
                .devices
                .get_mut(device_id)
                .ok_or_else(|| format!("không có thiết bị {device_id}"))?;
            device.revoked = flag;
        }
        None => {
            store
                .devices
                .remove(device_id)
                .ok_or_else(|| format!("không có thiết bị {device_id}"))?;
        }
    }
    store
        .save_devices()
        .map_err(|e| format!("không ghi được danh sách thiết bị: {e}"))?;
    println!(
        "{device_id}: {}",
        match revoked {
            Some(true) => "đã thu hồi (gateway đang chạy tự nhận thay đổi)",
            Some(false) => "đã khôi phục",
            None => "đã xoá khỏi danh sách",
        }
    );
    Ok(())
}

fn list_devices() {
    let store = Store::open(&data_dir());
    if store.devices.is_empty() {
        println!("Chưa có thiết bị nào đăng ký.");
    }
    for (id, d) in &store.devices {
        println!(
            "{id}  {:<20}  đăng ký {}  gần nhất {}  {}",
            d.hostname,
            d.registered_at,
            d.last_seen,
            if d.revoked {
                "ĐÃ THU HỒI"
            } else {
                "đang dùng"
            }
        );
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--env-file") {
        let path = args
            .get(i + 1)
            .cloned()
            .ok_or("--env-file cần đường dẫn file")?;
        load_env_file(Path::new(&path))?;
        args.drain(i..=i + 1);
    }
    let id = || {
        args.get(1)
            .map(String::as_str)
            .ok_or("cần mã thiết bị (xem bằng lệnh `devices`)")
    };
    match args.first().map_or("serve", String::as_str) {
        "serve" => serve(),
        "check" => {
            let detail = telegram()?
                .check(&SendContext::default())
                .map_err(|e| e.to_string())?;
            println!("{detail}");
            Ok(())
        }
        "devices" => {
            list_devices();
            Ok(())
        }
        "revoke" => set_revoked(id()?, Some(true)),
        "restore" => set_revoked(id()?, Some(false)),
        "remove" => set_revoked(id()?, None),
        other => Err(format!(
            "không có lệnh `{other}`. Dùng: smks-gateway [--env-file <file>] [serve | check | devices | revoke <id> | restore <id> | remove <id>]"
        )),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("LỖI: {error}");
        std::process::exit(1);
    }
}
