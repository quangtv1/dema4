//! Xử lý yêu cầu của gateway, tách khỏi tầng HTTP để kiểm thử được.
//!
//! Quy tắc:
//! - **Đăng ký** (`POST /v1/devices/register`): cần khoá đăng ký chung (`GATEWAY_ENROLL_KEY`). Gateway cấp cho thiết
//!   bị một bí mật riêng, chỉ lưu SHA-256 của bí mật đó.
//! - **Sự kiện** (`POST /v1/events`): cần `Authorization: Bearer <device_id>:<bí mật>` của thiết bị đã đăng ký và
//!   chưa bị thu hồi. Gateway tự soạn tin nhắn từ sự kiện (máy trạm không gửi được nội dung tuỳ ý), bỏ sự kiện trùng
//!   `event_id`, giới hạn tần suất theo thiết bị, và tôn trọng các công tắc bật / tắt đặt ở máy chủ.
//! - Bot Token chỉ nằm trong biến môi trường của máy chủ; không ghi ra log, không trả về cho máy trạm.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use smks_notify::event::{Category, Event, EventType};
use smks_notify::state::Flags;
use smks_notify::time;
use smks_notify::transport::{SendContext, SendError, Transport};

/// Cỡ thân yêu cầu tối đa.
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// Số sự kiện tối đa mỗi thiết bị trong một phút.
pub const MAX_EVENTS_PER_MINUTE: usize = 30;
/// Số `event_id` gần nhất được nhớ để bỏ sự kiện trùng.
const MAX_SEEN: usize = 5000;
/// Kết quả kiểm tra Telegram được dùng lại trong khoảng này.
const HEALTH_CACHE_SECONDS: u64 = 60;

const DEVICES_FILE: &str = "devices.json";
const SEEN_FILE: &str = "seen-events.json";

pub type Reply = (u16, Value);

fn fail(status: u16, error: &str) -> Reply {
    (status, json!({ "ok": false, "error": error }))
}

fn sha256_hex(text: &str) -> String {
    let digest: [u8; 32] = Sha256::digest(text.as_bytes()).into();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// So sánh hai chuỗi không lộ vị trí khác nhau qua thời gian chạy (so SHA-256 của chúng).
fn same_secret(a: &str, b: &str) -> bool {
    let x: [u8; 32] = Sha256::digest(a.as_bytes()).into();
    let y: [u8; 32] = Sha256::digest(b.as_bytes()).into();
    x.iter().zip(y).fold(0u8, |acc, (p, q)| acc | (p ^ q)) == 0
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Device {
    /// SHA-256 (hex) của bí mật thiết bị. Bí mật rõ chỉ trả cho thiết bị một lần lúc đăng ký.
    pub secret_sha256: String,
    pub hostname: String,
    pub registered_at: String,
    pub last_seen: String,
    /// Quản trị viên thu hồi: thiết bị không gửi được và không đăng ký lại được.
    pub revoked: bool,
}

/// Danh sách thiết bị (`devices.json`, quản trị viên sửa tay được) và các `event_id` đã nhận.
#[derive(Debug, Default)]
pub struct Store {
    dir: PathBuf,
    pub devices: BTreeMap<String, Device>,
    seen: VecDeque<String>,
    loaded_at: Option<SystemTime>,
}

fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

impl Store {
    pub fn open(dir: &Path) -> Self {
        let mut store = Self {
            dir: dir.to_path_buf(),
            ..Self::default()
        };
        store.reload();
        store.seen = std::fs::read_to_string(dir.join(SEEN_FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        store
    }

    fn devices_path(&self) -> PathBuf {
        self.dir.join(DEVICES_FILE)
    }

    fn modified(&self) -> Option<SystemTime> {
        std::fs::metadata(self.devices_path())
            .and_then(|m| m.modified())
            .ok()
    }

    fn reload(&mut self) {
        let path = self.devices_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(devices) => self.devices = devices,
                Err(e) => log(&format!(
                    "CẢNH BÁO: {} không đọc được ({e}); giữ danh sách thiết bị đang có trong bộ nhớ",
                    path.display()
                )),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.devices.clear(),
            Err(e) => log(&format!("CẢNH BÁO: không đọc được {}: {e}", path.display())),
        }
        self.loaded_at = self.modified();
    }

    /// Đọc lại `devices.json` nếu quản trị viên vừa sửa (thu hồi / khôi phục thiết bị không cần khởi động lại).
    pub fn refresh(&mut self) {
        if self.modified() != self.loaded_at {
            self.reload();
        }
    }

    pub fn save_devices(&mut self) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(&self.devices).map_err(std::io::Error::other)?;
        write_atomic(&self.devices_path(), &text)?;
        self.loaded_at = self.modified();
        Ok(())
    }

    fn seen(&self, event_id: &str) -> bool {
        self.seen.iter().any(|id| id == event_id)
    }

    fn mark_seen(&mut self, event_id: &str) {
        self.seen.push_back(event_id.to_owned());
        while self.seen.len() > MAX_SEEN {
            self.seen.pop_front();
        }
        let result = serde_json::to_string(&self.seen)
            .map_err(std::io::Error::other)
            .and_then(|text| write_atomic(&self.dir.join(SEEN_FILE), &text));
        if let Err(e) = result {
            log(&format!("CẢNH BÁO: không ghi được {SEEN_FILE}: {e}"));
        }
    }
}

/// Ghi một dòng log ra stderr. Không bao giờ truyền bí mật vào đây.
pub fn log(message: &str) {
    eprintln!("{} {message}", time::iso(time::now()));
}

pub struct Gateway {
    enroll_key: String,
    flags: Flags,
    telegram: Arc<dyn Transport>,
    store: Mutex<Store>,
    recent: Mutex<HashMap<String, VecDeque<Instant>>>,
    health: Mutex<Option<(Instant, String)>>,
}

impl std::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gateway")
            .field("flags", &self.flags)
            .finish_non_exhaustive()
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Deserialize)]
struct RegisterBody {
    event: Event,
    #[serde(default)]
    announce: bool,
}

impl Gateway {
    pub fn new(enroll_key: &str, flags: Flags, telegram: Arc<dyn Transport>, store: Store) -> Self {
        Self {
            enroll_key: enroll_key.to_owned(),
            flags,
            telegram,
            store: Mutex::new(store),
            recent: Mutex::new(HashMap::new()),
            health: Mutex::new(None),
        }
    }

    fn category_enabled(&self, event_type: EventType) -> bool {
        let f = self.flags;
        let category = match event_type.category() {
            Category::Device => f.device,
            Category::Sign => f.sign,
            Category::Document => f.document,
            Category::Test => true,
        };
        f.enabled && category && (f.failure || !event_type.is_failure())
    }

    /// `true` nếu `key` còn trong hạn mức `limit` lần mỗi phút.
    fn within_rate(&self, key: &str, limit: usize) -> bool {
        let mut recent = lock(&self.recent);
        let now = Instant::now();
        let times = recent.entry(key.to_owned()).or_default();
        while times
            .front()
            .is_some_and(|t| now.duration_since(*t).as_secs() >= 60)
        {
            times.pop_front();
        }
        if times.len() >= limit {
            return false;
        }
        times.push_back(now);
        true
    }

    fn forward(&self, event: &Event) -> Result<(), Reply> {
        self.telegram
            .send(event, &SendContext::default())
            .map(|_| ())
            .map_err(|error| {
                log(&format!(
                    "LỖI: chưa chuyển được {} của {}: {error}",
                    event.event_type.code(),
                    event.device_id
                ));
                match error {
                    SendError::Rejected(_) => fail(422, "Telegram từ chối nội dung sự kiện"),
                    SendError::RateLimited(seconds, _) => (
                        429,
                        json!({ "ok": false, "error": "Telegram yêu cầu gửi chậm lại", "retry_after": seconds }),
                    ),
                    SendError::Config(_) => fail(
                        503,
                        "gateway chưa gửi được tới Telegram (kiểm tra Bot Token / Chat ID ở máy chủ)",
                    ),
                    _ => fail(502, "gateway chưa kết nối được Telegram"),
                }
            })
    }

    /// Tách `device_id` và bí mật từ `Authorization: Bearer <device_id>:<bí mật>`.
    fn parse_bearer(header: Option<&str>) -> Option<(&str, &str)> {
        header?.strip_prefix("Bearer ")?.trim().split_once(':')
    }

    /// `Ok(device_id)` nếu thiết bị đã đăng ký, chưa bị thu hồi và bí mật đúng.
    fn authenticate(&self, header: Option<&str>) -> Result<String, Reply> {
        let Some((device_id, secret)) = Self::parse_bearer(header) else {
            return Err(fail(401, "thiếu thông tin xác thực thiết bị"));
        };
        let mut store = lock(&self.store);
        store.refresh();
        match store.devices.get(device_id) {
            None => Err(fail(401, "thiết bị chưa đăng ký")),
            Some(d) if d.revoked => Err(fail(403, "thiết bị đã bị thu hồi")),
            Some(d) if !same_secret(&d.secret_sha256, &sha256_hex(secret)) => {
                Err(fail(401, "bí mật thiết bị không đúng"))
            }
            Some(_) => Ok(device_id.to_owned()),
        }
    }

    pub fn register(&self, enroll_key: Option<&str>, body: &[u8]) -> Reply {
        if !enroll_key.is_some_and(|k| same_secret(k, &self.enroll_key)) {
            log("TỪ CHỐI đăng ký: khoá đăng ký không đúng");
            return fail(403, "khoá đăng ký không đúng");
        }
        let Ok(RegisterBody {
            mut event,
            announce,
        }) = serde_json::from_slice(body)
        else {
            return fail(400, "thân yêu cầu không phải JSON đăng ký hợp lệ");
        };
        event.sanitize();
        if event.event_type != EventType::DeviceRegistered || event.device_id.len() < 8 {
            return fail(400, "yêu cầu đăng ký không hợp lệ");
        }
        if !self.within_rate(&format!("register:{}", event.device_id), 10) {
            return (
                429,
                json!({ "ok": false, "error": "đăng ký quá nhiều lần", "retry_after": 60 }),
            );
        }
        let (is_new, retried) = {
            let mut store = lock(&self.store);
            store.refresh();
            if store
                .devices
                .get(&event.device_id)
                .is_some_and(|d| d.revoked)
            {
                log(&format!(
                    "TỪ CHỐI đăng ký: thiết bị {} đã bị thu hồi",
                    event.device_id
                ));
                return fail(403, "thiết bị đã bị thu hồi");
            }
            (
                !store.devices.contains_key(&event.device_id),
                store.seen(&event.event_id),
            )
        };
        // Báo lên Telegram TRƯỚC khi lưu: báo lỗi thì máy trạm đăng ký lại và thiết bị vẫn được coi là mới.
        let announced =
            announce && is_new && !retried && self.category_enabled(EventType::DeviceRegistered);
        if announced && let Err(reply) = self.forward(&event) {
            return reply;
        }
        let secret = time::random_hex::<32>();
        let now = time::iso(time::now());
        let mut store = lock(&self.store);
        let registered_at = store
            .devices
            .get(&event.device_id)
            .map(|d| d.registered_at.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| now.clone());
        store.devices.insert(
            event.device_id.clone(),
            Device {
                secret_sha256: sha256_hex(&secret),
                hostname: event.metadata.hostname.clone(),
                registered_at,
                last_seen: now,
                revoked: false,
            },
        );
        if let Err(e) = store.save_devices() {
            log(&format!("LỖI: không ghi được {DEVICES_FILE}: {e}"));
            return fail(500, "gateway không lưu được thiết bị");
        }
        store.mark_seen(&event.event_id);
        log(&format!(
            "ĐĂNG KÝ thiết bị {} ({}){}",
            event.device_id,
            event.metadata.hostname,
            match (is_new, announced) {
                (true, true) => ", đã báo Telegram",
                (true, false) => ", không báo Telegram",
                (false, _) => ", cấp lại bí mật",
            }
        ));
        (200, json!({ "ok": true, "device_secret": secret }))
    }

    pub fn events(&self, authorization: Option<&str>, body: &[u8]) -> Reply {
        let device_id = match self.authenticate(authorization) {
            Ok(id) => id,
            Err(reply) => return reply,
        };
        if !self.within_rate(&device_id, MAX_EVENTS_PER_MINUTE) {
            return (
                429,
                json!({ "ok": false, "error": "thiết bị gửi quá nhiều sự kiện", "retry_after": 60 }),
            );
        }
        let Ok(mut event) = serde_json::from_slice::<Event>(body) else {
            return fail(400, "thân yêu cầu không phải sự kiện hợp lệ");
        };
        event.sanitize();
        if event.device_id != device_id {
            return fail(400, "mã thiết bị trong sự kiện không khớp thiết bị gửi");
        }
        if event.event_type == EventType::DeviceRegistered || event.event_id.is_empty() {
            return fail(400, "loại sự kiện không được nhận ở đây");
        }
        if lock(&self.store).seen(&event.event_id) {
            return (200, json!({ "ok": true, "duplicate": true }));
        }
        let enabled = self.category_enabled(event.event_type);
        if enabled && let Err(reply) = self.forward(&event) {
            return reply;
        }
        let mut store = lock(&self.store);
        store.mark_seen(&event.event_id);
        if event.event_type == EventType::DeviceUnregistered {
            store.devices.remove(&device_id);
        } else if let Some(device) = store.devices.get_mut(&device_id) {
            device.last_seen = time::iso(time::now());
        }
        if let Err(e) = store.save_devices() {
            log(&format!("CẢNH BÁO: không ghi được {DEVICES_FILE}: {e}"));
        }
        log(&format!(
            "{} {} từ {} ({})",
            if enabled {
                "CHUYỂN"
            } else {
                "BỎ QUA (đang tắt)"
            },
            event.event_type.code(),
            device_id,
            event.transaction_id
        ));
        (200, json!({ "ok": true, "skipped": !enabled }))
    }

    pub fn health(&self, authorization: Option<&str>) -> Reply {
        let telegram = {
            let mut cache = lock(&self.health);
            match cache.as_ref() {
                Some((at, text)) if at.elapsed().as_secs() < HEALTH_CACHE_SECONDS => text.clone(),
                _ => {
                    let text = match self.telegram.check(&SendContext::default()) {
                        Ok(_) => "ok".to_owned(),
                        Err(e) => e.to_string(),
                    };
                    *cache = Some((Instant::now(), text.clone()));
                    text
                }
            }
        };
        let device = authorization.map(|_| match self.authenticate(authorization) {
            Ok(_) => "registered",
            Err((403, _)) => "revoked",
            Err(_) => "unknown",
        });
        (
            200,
            json!({
                "ok": true,
                "service": "smks-gateway",
                "version": env!("CARGO_PKG_VERSION"),
                "telegram": telegram,
                "device": device,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use smks_notify::event::{Metadata, Status};
    use smks_notify::transport::Delivery;

    use super::*;

    #[derive(Default)]
    struct Fake {
        sent: Mutex<Vec<String>>,
        fail: Mutex<Option<SendError>>,
    }

    impl Transport for Fake {
        fn send(&self, event: &Event, _c: &SendContext<'_>) -> Result<Delivery, SendError> {
            if let Some(e) = lock(&self.fail).clone() {
                return Err(e);
            }
            lock(&self.sent).push(smks_notify::template::render(event));
            Ok(Delivery::default())
        }

        fn check(&self, _c: &SendContext<'_>) -> Result<String, SendError> {
            Ok("ok".into())
        }
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "smks-gateway-test-{name}-{}-{}",
            std::process::id(),
            time::random_hex::<4>()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn event(event_type: EventType, device: &str, id: &str) -> Event {
        let mut metadata = Metadata {
            hostname: "PC-1".into(),
            ..Metadata::default()
        };
        metadata.set_documents(["a.pdf"]);
        Event {
            event_id: id.into(),
            event_type,
            device_id: device.into(),
            user_id: "an.nv".into(),
            timestamp: 1_794_986_277,
            transaction_id: "KS-1".into(),
            status: Status::Success,
            metadata,
        }
    }

    fn register_body(device: &str, id: &str) -> Vec<u8> {
        json!({ "event": event(EventType::DeviceRegistered, device, id), "announce": true })
            .to_string()
            .into_bytes()
    }

    fn bytes(e: &Event) -> Vec<u8> {
        serde_json::to_vec(e).unwrap()
    }

    const KEY: &str = "khoa-dang-ky-thu-nghiem";
    const DEV: &str = "SMKS-AAAAAAAA-0001";

    fn gateway(data: &Path, flags: Flags) -> (Gateway, Arc<Fake>) {
        let fake = Arc::new(Fake::default());
        (
            Gateway::new(KEY, flags, fake.clone(), Store::open(data)),
            fake,
        )
    }

    #[test]
    fn enrollment_issues_a_per_device_secret_and_announces_once() {
        let data = dir("register");
        let (gw, fake) = gateway(&data, Flags::default());
        assert_eq!(gw.register(None, &register_body(DEV, "e1")).0, 403);
        assert_eq!(gw.register(Some("sai"), &register_body(DEV, "e1")).0, 403);
        assert_eq!(gw.register(Some(KEY), b"khong phai json").0, 400);
        let not_registration = json!({ "event": event(EventType::SignSuccess, DEV, "e0") });
        assert_eq!(
            gw.register(Some(KEY), not_registration.to_string().as_bytes())
                .0,
            400
        );
        assert!(lock(&fake.sent).is_empty());

        let (status, reply) = gw.register(Some(KEY), &register_body(DEV, "e1"));
        assert_eq!(status, 200);
        let secret = reply["device_secret"].as_str().unwrap().to_owned();
        assert_eq!(secret.len(), 64);
        assert_eq!(lock(&fake.sent).len(), 1);
        assert!(lock(&fake.sent)[0].contains("[SAO MAI SIGN - NEW DEVICE]"));

        // File thiết bị chỉ có SHA-256 của bí mật.
        let saved = std::fs::read_to_string(data.join(DEVICES_FILE)).unwrap();
        assert!(!saved.contains(&secret) && saved.contains(&sha256_hex(&secret)));

        // Đăng ký lại (máy trạm mất bí mật): cấp bí mật mới, không báo Telegram lần hai; bí mật cũ hết hiệu lực.
        let (_, again) = gw.register(Some(KEY), &register_body(DEV, "e2"));
        let secret2 = again["device_secret"].as_str().unwrap().to_owned();
        assert_ne!(secret, secret2);
        assert_eq!(lock(&fake.sent).len(), 1);
        let old = format!("Bearer {DEV}:{secret}");
        let new = format!("Bearer {DEV}:{secret2}");
        let e = bytes(&event(EventType::SignSuccess, DEV, "e3"));
        assert_eq!(gw.events(Some(&old), &e).0, 401);
        assert_eq!(gw.events(Some(&new), &e).0, 200);
        std::fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn events_need_a_valid_device_and_are_deduplicated() {
        let data = dir("events");
        let (gw, fake) = gateway(&data, Flags::default());
        let (_, reply) = gw.register(Some(KEY), &register_body(DEV, "e1"));
        let auth = format!("Bearer {DEV}:{}", reply["device_secret"].as_str().unwrap());
        let e = event(EventType::SignSuccess, DEV, "e2");

        assert_eq!(gw.events(None, &bytes(&e)).0, 401);
        assert_eq!(gw.events(Some("Bearer khong-co:abc"), &bytes(&e)).0, 401);
        assert_eq!(
            gw.events(Some(&format!("Bearer {DEV}:sai")), &bytes(&e)).0,
            401
        );
        assert_eq!(gw.events(Some(&auth), b"{").0, 400);
        let other = event(EventType::SignSuccess, "SMKS-KHAC-0000", "e9");
        assert_eq!(
            gw.events(Some(&auth), &bytes(&other)).0,
            400,
            "không gửi hộ thiết bị khác"
        );
        assert_eq!(lock(&fake.sent).len(), 1, "chỉ có tin đăng ký");

        assert_eq!(
            gw.events(Some(&auth), &bytes(&e)),
            (200, json!({ "ok": true, "skipped": false }))
        );
        assert_eq!(
            gw.events(Some(&auth), &bytes(&e)),
            (200, json!({ "ok": true, "duplicate": true }))
        );
        assert_eq!(
            lock(&fake.sent).len(),
            2,
            "gửi lại cùng event_id không ra tin thứ hai"
        );
        assert!(lock(&fake.sent)[1].contains("Tên văn bản: a.pdf"));

        // Telegram lỗi: báo 502 để máy trạm gửi lại; sự kiện chưa bị coi là đã nhận.
        *lock(&fake.fail) = Some(SendError::Network("mất mạng".into()));
        let e3 = event(EventType::SignFailed, DEV, "e3");
        assert_eq!(gw.events(Some(&auth), &bytes(&e3)).0, 502);
        *lock(&fake.fail) = None;
        assert_eq!(gw.events(Some(&auth), &bytes(&e3)).0, 200);
        assert_eq!(lock(&fake.sent).len(), 3);

        // Thu hồi bằng cách sửa devices.json (không khởi động lại): thiết bị không gửi và không đăng ký lại được.
        let path = data.join(DEVICES_FILE);
        let mut devices: BTreeMap<String, Device> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        devices.get_mut(DEV).unwrap().revoked = true;
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&path, serde_json::to_string(&devices).unwrap()).unwrap();
        let e4 = event(EventType::SignSuccess, DEV, "e4");
        assert_eq!(gw.events(Some(&auth), &bytes(&e4)).0, 403);
        assert_eq!(gw.register(Some(KEY), &register_body(DEV, "e5")).0, 403);
        assert_eq!(gw.health(Some(&auth)).1["device"], "revoked");
        assert_eq!(gw.health(None).1["telegram"], "ok");
        assert_eq!(lock(&fake.sent).len(), 3);
        std::fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn server_switches_rate_limit_and_unregister() {
        let data = dir("flags");
        let flags = Flags {
            sign: false,
            ..Flags::default()
        };
        let (gw, fake) = gateway(&data, flags);
        let (_, reply) = gw.register(Some(KEY), &register_body(DEV, "e1"));
        let auth = format!("Bearer {DEV}:{}", reply["device_secret"].as_str().unwrap());
        let sign = event(EventType::SignSuccess, DEV, "e2");
        assert_eq!(
            gw.events(Some(&auth), &bytes(&sign)),
            (200, json!({ "ok": true, "skipped": true }))
        );
        let doc = event(EventType::DocumentCreated, DEV, "e3");
        assert_eq!(gw.events(Some(&auth), &bytes(&doc)).0, 200);
        assert_eq!(
            lock(&fake.sent).len(),
            2,
            "đăng ký + tạo văn bản; ký số đang tắt ở máy chủ"
        );

        // Hạn mức: tối đa 30 sự kiện một phút cho mỗi thiết bị.
        let mut limited = false;
        for i in 0..MAX_EVENTS_PER_MINUTE + 2 {
            let e = event(EventType::DocumentCreated, DEV, &format!("r{i}"));
            limited |= gw.events(Some(&auth), &bytes(&e)).0 == 429;
        }
        assert!(limited);

        // Hủy đăng ký: gateway xoá thiết bị, bí mật cũ không dùng được nữa.
        let data2 = dir("unregister");
        let (gw2, fake2) = gateway(&data2, Flags::default());
        let (_, reply) = gw2.register(Some(KEY), &register_body(DEV, "e1"));
        let auth2 = format!("Bearer {DEV}:{}", reply["device_secret"].as_str().unwrap());
        let bye = event(EventType::DeviceUnregistered, DEV, "e2");
        assert_eq!(gw2.events(Some(&auth2), &bytes(&bye)).0, 200);
        assert!(lock(&fake2.sent)[1].contains("DEVICE UNREGISTERED"));
        let after = event(EventType::SignSuccess, DEV, "e3");
        assert_eq!(gw2.events(Some(&auth2), &bytes(&after)).0, 401);
        std::fs::remove_dir_all(data).unwrap();
        std::fs::remove_dir_all(data2).unwrap();
    }
}
