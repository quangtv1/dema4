//! [`Notifier`]: nơi duy nhất ứng dụng gọi để ghi nhận sự kiện và quản lý đăng ký thiết bị.
//!
//! Nguyên tắc:
//! - [`Notifier::emit`] **không chặn và không trả lỗi**: ghi nhật ký, xét quyền gửi, đưa vào hàng đợi rồi về ngay.
//!   Việc gửi chạy trên luồng nền; Telegram hỏng, mất mạng, cấu hình sai đều không ảnh hưởng nghiệp vụ.
//! - Không gửi gì khi: chưa có cấu hình quản trị, người dùng chưa đồng ý (hoặc đã từ chối), hoặc loại thông báo đó
//!   đang tắt (quản trị viên tắt trong `notify.json`, hoặc người dùng tắt trên giao diện).
//! - Sự kiện đăng ký chỉ phát một lần khi người dùng đồng ý; các lần mở ứng dụng sau không đăng ký lại.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::Serialize;

use crate::audit::Audit;
use crate::config::{AdminConfig, Loaded, Mode};
use crate::device::{self, DeviceInfo};
use crate::event::{Category, Event, EventType, Metadata};
use crate::gateway::GatewayClient;
use crate::queue::{self, Failed, Item, Push, Queue};
use crate::state::{self, Consent, Flags, State};
use crate::telegram::TelegramClient;
use crate::text::mask;
use crate::time;
use crate::transport::{SendContext, SendError, Transport};

/// Thời gian luồng gửi ngủ khi không có gì để gửi (có sự kiện mới thì được đánh thức ngay).
const IDLE: Duration = Duration::from_secs(300);

/// Mã hoá bí mật thiết bị trước khi lưu xuống đĩa (ứng dụng dùng DPAPI của Windows).
pub trait Sealer: Send + Sync {
    fn seal(&self, plain: &str) -> Option<String>;
    fn open(&self, sealed: &str) -> Option<String>;
}

/// Không mã hoá: chỉ cho kiểm thử.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainSealer;

impl Sealer for PlainSealer {
    fn seal(&self, plain: &str) -> Option<String> {
        Some(plain.to_owned())
    }

    fn open(&self, sealed: &str) -> Option<String> {
        Some(sealed.to_owned())
    }
}

pub struct Options {
    /// Thư mục dữ liệu của ứng dụng (`%APPDATA%\SaoMai-KySo`).
    pub dir: PathBuf,
    pub app_version: String,
    /// Tài khoản Windows đang dùng ứng dụng.
    pub user_name: String,
    pub admin: Loaded,
    pub sealer: Box<dyn Sealer>,
    /// Thay kênh gửi dựng từ cấu hình (kiểm thử).
    pub transport: Option<Arc<dyn Transport>>,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

/// Sự kiện do nghiệp vụ báo lên; [`Notifier`] điền nốt mã sự kiện, thiết bị, người dùng, thời gian.
#[derive(Debug, Clone)]
pub struct Draft {
    pub event_type: EventType,
    /// Mã giao dịch của lần ký / lần tạo văn bản; rỗng thì tự sinh.
    pub transaction_id: String,
    pub metadata: Metadata,
    /// Khoá chống trùng theo nghiệp vụ (ví dụ `sign-ok:<mã giao dịch>`); không có thì mỗi lần gọi là một sự kiện.
    pub dedup_key: Option<String>,
}

impl Draft {
    pub fn new(event_type: EventType, transaction_id: &str, metadata: Metadata) -> Self {
        Self {
            event_type,
            transaction_id: transaction_id.to_owned(),
            metadata,
            dedup_key: Some(format!("{}:{transaction_id}", event_type.code())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    /// `unknown` (chưa kiểm tra), `ok`, `error`.
    pub state: &'static str,
    pub message: String,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone)]
struct Connection {
    state: &'static str,
    message: String,
    checked_at: Option<i64>,
}

/// Trạng thái cho giao diện cấu hình. Không chứa Bot Token hay Chat ID đầy đủ.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusView {
    /// Có cấu hình quản trị dùng được.
    pub configured: bool,
    pub config_source: Option<String>,
    pub config_problem: Option<String>,
    /// `gateway` hoặc `direct`.
    pub mode: Option<&'static str>,
    /// Nơi nhận, đã che bí mật.
    pub target: String,
    /// Quản trị viên khoá các công tắc.
    pub locked: bool,
    pub admin: Flags,
    pub prefs: Flags,
    pub effective: Flags,
    pub consent: Consent,
    /// Cần hỏi người dùng (đã có cấu hình, chưa trả lời).
    pub needs_consent: bool,
    pub device_id: String,
    /// `none`, `pending` (đã đồng ý, chưa gửi tới nơi), `registered`.
    pub registration: &'static str,
    pub registered_at: Option<String>,
    pub include_ip: bool,
    pub include_mac: bool,
    /// Thông tin thiết bị (để người dùng xem trước ở hộp đăng ký; IP và MAC chỉ gửi khi được chọn).
    pub device: DeviceInfo,
    pub connection: ConnectionView,
    /// Số sự kiện đang chờ gửi.
    pub pending: usize,
    pub next_attempt_at: Option<String>,
    pub last_error: Option<String>,
    pub last_sent_at: Option<String>,
    pub audit_dir: String,
}

struct Inner {
    dir: PathBuf,
    app_version: String,
    user_name: String,
    admin: Option<AdminConfig>,
    source: Option<String>,
    problem: Option<String>,
    mode: Option<Mode>,
    transport: Option<Arc<dyn Transport>>,
    sealer: Box<dyn Sealer>,
    state: Mutex<State>,
    queue: Mutex<Queue>,
    connection: Mutex<Connection>,
    wake: Mutex<Option<Sender<()>>>,
    audit: Audit,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Xem module [`crate::service`].
#[derive(Clone)]
pub struct Notifier {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Notifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notifier")
            .field("mode", &self.inner.mode)
            .finish_non_exhaustive()
    }
}

fn build_transport(admin: &AdminConfig, device_id: &str) -> Option<Arc<dyn Transport>> {
    let timeout = Duration::from_secs(admin.timeout_seconds);
    Some(match admin.mode()? {
        Mode::Gateway => Arc::new(GatewayClient::new(
            &admin.gateway_url,
            &admin.gateway_enroll_key,
            device_id,
            timeout,
        )),
        Mode::Direct => Arc::new(TelegramClient::new(
            &admin.telegram_api_base,
            &admin.telegram_bot_token,
            &admin.telegram_chat_id,
            timeout,
        )),
    })
}

impl Notifier {
    /// Mở trạng thái và hàng đợi trên đĩa, khởi động luồng gửi nền.
    pub fn start(options: Options) -> Self {
        Self::start_with(options, true)
    }

    /// Như [`Self::start`]; `background = false` thì không tạo luồng nền, bên gọi tự gọi [`Self::pump`] (kiểm thử).
    pub fn start_with(options: Options, background: bool) -> Self {
        let (state, _) = State::load(&options.dir.join(state::FILE_NAME));
        let mut queue = Queue::load(&options.dir.join(queue::FILE_NAME));
        // Mở ứng dụng: thử gửi ngay những gì còn tồn từ lần trước (số lần đã thử vẫn giữ).
        queue.make_all_due(time::now());
        let admin = options.admin.config;
        let mode = admin.as_ref().and_then(AdminConfig::mode);
        let transport = options.transport.or_else(|| {
            admin
                .as_ref()
                .and_then(|a| build_transport(a, &state.device_id))
        });
        let inner = Arc::new(Inner {
            audit: Audit::new(&options.dir),
            dir: options.dir,
            app_version: options.app_version,
            user_name: options.user_name,
            source: options.admin.source.map(|p| p.display().to_string()),
            problem: options.admin.problem,
            admin,
            mode,
            transport,
            sealer: options.sealer,
            state: Mutex::new(state),
            queue: Mutex::new(queue),
            connection: Mutex::new(Connection {
                state: "unknown",
                message: String::new(),
                checked_at: None,
            }),
            wake: Mutex::new(None),
        });
        let notifier = Self { inner };
        if background {
            let (sender, receiver) = channel();
            let worker = notifier.clone();
            match std::thread::Builder::new()
                .name("thong-bao".into())
                .spawn(move || worker.run(&receiver))
            {
                Ok(_) => *lock(&notifier.inner.wake) = Some(sender),
                Err(e) => tracing::error!("không tạo được luồng gửi thông báo: {e}"),
            }
        }
        tracing::info!(
            mode = notifier.inner.mode.map_or("-", Mode::code),
            configured = notifier.inner.transport.is_some(),
            "thông báo Telegram"
        );
        notifier.auto_register();
        notifier.note_app_version();
        notifier
    }

    /// Quản trị viên bật `AUTO_REGISTER`: máy chưa trả lời thì tự đăng ký, không hỏi (TD-60). Máy đã từ chối ở bản
    /// cũ thì giữ nguyên lựa chọn đó.
    fn auto_register(&self) {
        let Some(admin) = self.inner.admin.as_ref() else {
            return;
        };
        if !admin.auto_register
            || !admin.notification_enabled
            || self.inner.transport.is_none()
            || lock(&self.inner.state).consent != Consent::Pending
        {
            return;
        }
        tracing::info!("tự đăng ký thiết bị theo cấu hình quản trị");
        self.set_consent(true, admin.send_ip_default, admin.send_mac_default);
    }

    fn state_path(&self) -> PathBuf {
        self.inner.dir.join(state::FILE_NAME)
    }

    fn queue_path(&self) -> PathBuf {
        self.inner.dir.join(queue::FILE_NAME)
    }

    fn wake(&self) {
        if let Some(sender) = lock(&self.inner.wake).as_ref() {
            let _ = sender.send(());
        }
    }

    fn update_state(&self, change: impl FnOnce(&mut State)) {
        let mut state = lock(&self.inner.state);
        change(&mut state);
        state.save(&self.state_path());
    }

    // ------------------------------------------------------------------------------------------
    // Quyền gửi
    // ------------------------------------------------------------------------------------------

    fn admin_flags(&self) -> Flags {
        self.inner
            .admin
            .as_ref()
            .map_or_else(Flags::default, |a| Flags {
                enabled: a.notification_enabled,
                device: a.device_registration_enabled,
                sign: a.sign_notification_enabled,
                document: a.document_notification_enabled,
                failure: a.failure_notification_enabled,
            })
    }

    fn locked(&self) -> bool {
        self.inner.admin.as_ref().is_some_and(|a| a.lock_settings)
    }

    /// Công tắc có hiệu lực: quản trị viên **và** người dùng cùng bật (quản trị viên khoá thì chỉ theo quản trị).
    fn effective_flags(&self) -> Flags {
        let admin = self.admin_flags();
        if self.locked() {
            admin
        } else {
            admin.and(lock(&self.inner.state).prefs)
        }
    }

    fn enabled(&self, event_type: EventType) -> bool {
        let f = self.effective_flags();
        let category = match event_type.category() {
            Category::Device => f.device,
            Category::Sign => f.sign,
            Category::Document => f.document,
            Category::Test => true,
        };
        f.enabled && category && (f.failure || !event_type.is_failure())
    }

    /// Lý do không gửi sự kiện loại này (không có thì được gửi).
    fn skip_reason(&self, event_type: EventType) -> Option<&'static str> {
        if self.inner.admin.is_none() {
            return Some("chưa có cấu hình thông báo trên máy");
        }
        if self.inner.transport.is_none() {
            return Some("cấu hình thông báo thiếu thông tin");
        }
        match lock(&self.inner.state).consent {
            Consent::Pending => return Some("người dùng chưa trả lời đăng ký thiết bị"),
            Consent::Declined => return Some("người dùng không đồng ý gửi thông tin"),
            Consent::Accepted => {}
        }
        (!self.enabled(event_type)).then_some("loại thông báo này đang tắt")
    }

    // ------------------------------------------------------------------------------------------
    // Ghi nhận sự kiện
    // ------------------------------------------------------------------------------------------

    fn build(&self, event_type: EventType, transaction_id: &str, mut metadata: Metadata) -> Event {
        if metadata.hostname.is_empty() {
            metadata.hostname = device::hostname();
        }
        if metadata.app_version.is_empty() {
            metadata.app_version.clone_from(&self.inner.app_version);
        }
        if metadata.user_name.is_empty() {
            metadata.user_name.clone_from(&self.inner.user_name);
        }
        let mut event = Event {
            event_id: time::uuid_v4(),
            event_type,
            device_id: lock(&self.inner.state).device_id.clone(),
            user_id: self.inner.user_name.clone(),
            timestamp: time::now(),
            transaction_id: if transaction_id.is_empty() {
                time::transaction_id("EV")
            } else {
                transaction_id.to_owned()
            },
            status: event_type.status(),
            metadata,
        };
        event.sanitize();
        event
    }

    /// Đưa vào hàng đợi; `false` nếu trùng sự kiện đã nhận.
    fn enqueue(&self, event: &Event, dedup_key: String, announce: bool) -> bool {
        let now = time::now();
        let pushed = {
            let mut queue = lock(&self.inner.queue);
            let pushed = queue.push(Item {
                event: event.clone(),
                dedup_key,
                announce,
                attempts: 0,
                next_attempt: now,
                enqueued_at: now,
                last_error: String::new(),
            });
            if pushed != Push::Duplicate {
                queue.save(&self.queue_path());
            }
            pushed
        };
        match pushed {
            Push::Duplicate => return false,
            Push::QueuedDroppedOldest(old) => {
                tracing::warn!(
                    event = old.event.event_type.code(),
                    "hàng đợi thông báo đầy, bỏ sự kiện cũ nhất"
                );
                self.inner.audit.delivery(
                    &old.event,
                    "dropped",
                    old.attempts,
                    Some("hàng đợi đầy"),
                );
            }
            Push::Queued => {}
        }
        self.wake();
        true
    }

    /// Ghi nhận một sự kiện nghiệp vụ **đã kết thúc**. Không chặn, không trả lỗi.
    pub fn emit(&self, draft: Draft) {
        let event = self.build(draft.event_type, &draft.transaction_id, draft.metadata);
        let key = draft.dedup_key.unwrap_or_else(|| event.event_id.clone());
        let mut skipped = self.skip_reason(event.event_type);
        if skipped.is_none() && !self.enqueue(&event, key, true) {
            skipped = Some("trùng sự kiện đã ghi nhận");
        }
        tracing::info!(
            event = event.event_type.code(),
            tx = %event.transaction_id,
            queued = skipped.is_none(),
            "sự kiện nghiệp vụ"
        );
        self.inner.audit.event(&event, skipped);
    }

    /// Phát hiện ứng dụng vừa được cập nhật (phiên bản khác lần chạy trước).
    fn note_app_version(&self) {
        let current = self.inner.app_version.clone();
        let previous = {
            let mut state = lock(&self.inner.state);
            if state.last_app_version == current {
                return;
            }
            let previous = std::mem::replace(&mut state.last_app_version, current.clone());
            state.save(&self.state_path());
            previous
        };
        if previous.is_empty() {
            return;
        }
        let device_id = lock(&self.inner.state).device_id.clone();
        self.emit(Draft {
            event_type: EventType::AppUpdated,
            transaction_id: time::transaction_id("CN"),
            metadata: Metadata {
                previous_version: previous,
                ..Metadata::default()
            },
            dedup_key: Some(format!("app-updated:{device_id}:{current}")),
        });
    }

    // ------------------------------------------------------------------------------------------
    // Đăng ký thiết bị
    // ------------------------------------------------------------------------------------------

    fn registration_event(&self, note: &str) -> Event {
        let (ip, mac) = {
            let state = lock(&self.inner.state);
            (state.include_ip, state.include_mac)
        };
        let mut metadata = Metadata {
            note: note.to_owned(),
            ..Metadata::default()
        };
        DeviceInfo::collect(&self.inner.app_version, ip, mac).fill(&mut metadata);
        self.build(
            EventType::DeviceRegistered,
            &time::transaction_id("DK"),
            metadata,
        )
    }

    /// Người dùng trả lời hộp đăng ký thiết bị. Đồng ý: lưu lựa chọn và phát sự kiện đăng ký **một lần**. Từ chối:
    /// lưu lại để không hỏi nữa, không gửi gì.
    pub fn set_consent(&self, accept: bool, include_ip: bool, include_mac: bool) {
        if !accept {
            self.unregister();
            return;
        }
        let now = time::now();
        {
            let mut state = lock(&self.inner.state);
            if state.consent == Consent::Accepted {
                return;
            }
            state.consent = Consent::Accepted;
            state.consent_at = Some(now);
            state.include_ip = include_ip;
            state.include_mac = include_mac;
            state.registered_at = Some(now);
            state.registration_delivered_at = None;
            state.save(&self.state_path());
        }
        tracing::info!(
            include_ip,
            include_mac,
            "người dùng đồng ý đăng ký thiết bị"
        );
        let event = self.registration_event("");
        let key = format!("device-registered:{}:{now}", event.device_id);
        let announce = self.enabled(EventType::DeviceRegistered);
        let skipped = if self.inner.transport.is_none() {
            Some("cấu hình thông báo thiếu thông tin")
        } else if announce || self.inner.mode == Some(Mode::Gateway) {
            // Gateway cần lời đăng ký để cấp bí mật thiết bị, kể cả khi không báo lên Telegram.
            self.enqueue(&event, key, announce);
            None
        } else {
            self.update_state(|s| s.registration_delivered_at = Some(now));
            Some("thông báo đăng ký thiết bị đang tắt")
        };
        self.inner.audit.event(&event, skipped);
    }

    /// Hủy đăng ký (hoặc từ chối ở hộp đăng ký): báo một lần nếu thiết bị đang đăng ký, rồi ngừng gửi và bỏ các sự
    /// kiện đang chờ.
    pub fn unregister(&self) {
        let was_accepted = lock(&self.inner.state).consent == Consent::Accepted;
        if was_accepted {
            self.emit(Draft {
                event_type: EventType::DeviceUnregistered,
                transaction_id: time::transaction_id("HDK"),
                metadata: Metadata::default(),
                dedup_key: None,
            });
        }
        let notice_queued = {
            let mut queue = lock(&self.inner.queue);
            let removed = queue.purge(|i| i.event.event_type == EventType::DeviceUnregistered);
            if removed > 0 {
                queue.save(&self.queue_path());
            }
            !queue.items.is_empty()
        };
        let now = time::now();
        self.update_state(|s| {
            s.consent = Consent::Declined;
            s.consent_at = Some(now);
            s.registered_at = None;
            s.registration_delivered_at = None;
            if !notice_queued {
                s.credential = None;
            }
        });
        tracing::info!(
            was_accepted,
            "người dùng không đồng ý / hủy đăng ký thiết bị"
        );
        self.wake();
    }

    /// Người dùng đổi các công tắc trên giao diện. Lỗi khi quản trị viên đã khoá.
    pub fn set_prefs(&self, prefs: Flags) -> Result<(), String> {
        if self.locked() {
            return Err("Quản trị viên đã khoá các tuỳ chọn thông báo trên máy này.".into());
        }
        self.update_state(|s| s.prefs = prefs);
        tracing::info!(?prefs, "đổi tuỳ chọn thông báo");
        Ok(())
    }

    // ------------------------------------------------------------------------------------------
    // Gửi
    // ------------------------------------------------------------------------------------------

    fn credential(&self) -> Option<String> {
        let sealed = lock(&self.inner.state).credential.clone()?;
        self.inner.sealer.open(&sealed)
    }

    fn store_credential(&self, credential: Option<String>) {
        let Some(plain) = credential else { return };
        let sealed = self.inner.sealer.seal(&plain);
        if sealed.is_none() {
            tracing::warn!(
                "không mã hoá được bí mật thiết bị để lưu; sẽ đăng ký lại ở lần gửi sau"
            );
        }
        self.update_state(|s| s.credential = sealed);
    }

    fn set_connection(&self, state: &'static str, message: String) {
        *lock(&self.inner.connection) = Connection {
            state,
            message,
            checked_at: Some(time::now()),
        };
    }

    /// Đăng ký lại với gateway khi gateway không còn biết thiết bị (không tạo sự kiện nghiệp vụ mới).
    fn reregister(&self, transport: &dyn Transport) -> Result<(), SendError> {
        let event = self.registration_event("Đăng ký lại: gateway chưa có thông tin thiết bị này");
        let context = SendContext {
            credential: None,
            announce: self.enabled(EventType::DeviceRegistered),
        };
        let delivery = transport.send(&event, &context)?;
        self.store_credential(delivery.credential);
        self.inner.audit.delivery(&event, "re-registered", 1, None);
        Ok(())
    }

    /// Gửi một sự kiện; gateway báo chưa biết thiết bị thì đăng ký lại một lần rồi gửi lại.
    fn deliver(&self, transport: &dyn Transport, item: &Item) -> Result<(), SendError> {
        let mut reregistered = false;
        loop {
            let credential = self.credential();
            let context = SendContext {
                credential: credential.as_deref(),
                announce: item.announce,
            };
            match transport.send(&item.event, &context) {
                Ok(delivery) => {
                    self.store_credential(delivery.credential);
                    return Ok(());
                }
                Err(SendError::Unregistered(reason))
                    if !reregistered && item.event.event_type != EventType::DeviceRegistered =>
                {
                    reregistered = true;
                    tracing::info!("{reason} Đăng ký lại thiết bị với gateway.");
                    self.reregister(transport)?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Gửi các sự kiện đã tới lượt; trả về thời gian nên chờ trước lần gọi kế tiếp. Luồng nền gọi hàm này; kiểm thử
    /// gọi trực tiếp.
    pub fn pump(&self) -> Duration {
        let Some(transport) = self.inner.transport.clone() else {
            return IDLE;
        };
        loop {
            let now = time::now();
            let item = {
                let mut queue = lock(&self.inner.queue);
                let expired = queue.expire(now);
                if !expired.is_empty() {
                    queue.save(&self.queue_path());
                }
                for old in &expired {
                    self.inner.audit.delivery(
                        &old.event,
                        "gave-up",
                        old.attempts,
                        Some("quá hạn 7 ngày"),
                    );
                }
                match queue.head_due(now) {
                    Ok(None) => return IDLE,
                    Err(wait) => {
                        return Duration::from_secs(u64::try_from(wait).unwrap_or(1).max(1))
                            .min(IDLE);
                    }
                    Ok(Some(item)) => item.clone(),
                }
            };
            let event = &item.event;
            // Người dùng đã hủy đăng ký: chỉ còn được gửi lời báo hủy.
            let consent = lock(&self.inner.state).consent;
            if consent != Consent::Accepted && event.event_type != EventType::DeviceUnregistered {
                self.drop_item(event, "dropped", item.attempts, "người dùng đã hủy đăng ký");
                continue;
            }
            match self.deliver(transport.as_ref(), &item) {
                Ok(()) => {
                    {
                        let mut queue = lock(&self.inner.queue);
                        queue.remove(&event.event_id);
                        queue.save(&self.queue_path());
                    }
                    self.update_state(|s| {
                        s.last_sent_at = Some(now);
                        match event.event_type {
                            EventType::DeviceRegistered => s.registration_delivered_at = Some(now),
                            EventType::DeviceUnregistered => s.credential = None,
                            _ => {}
                        }
                    });
                    self.set_connection("ok", "Đã gửi thông báo gần nhất thành công.".into());
                    self.inner
                        .audit
                        .delivery(event, "sent", item.attempts + 1, None);
                    tracing::info!(event = event.event_type.code(), id = %event.event_id, attempts = item.attempts + 1, "đã gửi thông báo");
                }
                Err(SendError::Rejected(reason)) => {
                    self.drop_item(event, "rejected", item.attempts + 1, &reason);
                }
                Err(error) => {
                    let retry_after = match &error {
                        SendError::RateLimited(seconds, _) => Some(*seconds),
                        _ => None,
                    };
                    let outcome = {
                        let mut queue = lock(&self.inner.queue);
                        let outcome =
                            queue.mark_failed(&event.event_id, now, error.message(), retry_after);
                        queue.save(&self.queue_path());
                        outcome
                    };
                    self.set_connection("error", error.message().to_owned());
                    match outcome {
                        Failed::GaveUp(gone) => {
                            tracing::warn!(event = event.event_type.code(), id = %event.event_id, attempts = gone.attempts, "bỏ thông báo sau khi hết số lần thử: {error}");
                            self.inner.audit.delivery(
                                event,
                                "gave-up",
                                gone.attempts,
                                Some(error.message()),
                            );
                        }
                        Failed::RetryAt(at) => {
                            tracing::warn!(event = event.event_type.code(), id = %event.event_id, kind = error.kind(), retry_in = at - now, "chưa gửi được thông báo: {error}");
                            self.inner.audit.delivery(
                                event,
                                "retry",
                                item.attempts + 1,
                                Some(error.message()),
                            );
                        }
                        Failed::Missing => {}
                    }
                }
            }
        }
    }

    fn drop_item(&self, event: &Event, result: &str, attempts: u32, reason: &str) {
        {
            let mut queue = lock(&self.inner.queue);
            queue.remove(&event.event_id);
            queue.save(&self.queue_path());
        }
        tracing::warn!(event = event.event_type.code(), id = %event.event_id, "bỏ thông báo: {reason}");
        self.inner
            .audit
            .delivery(event, result, attempts, Some(reason));
    }

    fn run(&self, wake: &Receiver<()>) {
        loop {
            let wait = self.pump();
            match wake.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while wake.try_recv().is_ok() {}
        }
    }

    /// Cho thử gửi ngay mọi sự kiện đang chờ.
    pub fn flush(&self) {
        {
            let mut queue = lock(&self.inner.queue);
            queue.make_all_due(time::now());
            queue.save(&self.queue_path());
        }
        self.wake();
    }

    /// Kiểm tra kết nối tới Telegram / gateway (chặn; không gửi thông tin thiết bị). Thành công thì gửi luôn các sự
    /// kiện đang chờ.
    pub fn check_connection(&self) -> bool {
        let Some(transport) = self.inner.transport.clone() else {
            self.set_connection(
                "error",
                self.inner
                    .problem
                    .clone()
                    .unwrap_or_else(|| "Chưa có cấu hình thông báo trên máy này.".into()),
            );
            return false;
        };
        let credential = self.credential();
        let context = SendContext {
            credential: credential.as_deref(),
            announce: false,
        };
        match transport.check(&context) {
            Ok(detail) => {
                tracing::info!("kiểm tra kết nối thông báo: đạt");
                self.set_connection("ok", detail);
                self.flush();
                true
            }
            Err(error) => {
                tracing::warn!(kind = error.kind(), "kiểm tra kết nối thông báo: {error}");
                self.set_connection("error", error.message().to_owned());
                false
            }
        }
    }

    /// Gửi ngay một tin nhắn thử nghiệm (chặn, không qua hàng đợi). Tin thử có tên máy và mã thiết bị nên chỉ gửi
    /// khi người dùng đã đồng ý đăng ký.
    pub fn send_test(&self) -> Result<(), String> {
        let Some(transport) = self.inner.transport.clone() else {
            return Err(self
                .inner
                .problem
                .clone()
                .unwrap_or_else(|| "Chưa có cấu hình thông báo trên máy này.".into()));
        };
        if lock(&self.inner.state).consent != Consent::Accepted {
            return Err(
                "Tin thử có tên máy và mã thiết bị, nên cần đồng ý đăng ký thiết bị trước khi gửi."
                    .into(),
            );
        }
        if !self.effective_flags().enabled {
            return Err("Thông báo Telegram đang tắt.".into());
        }
        let event = self.build(
            EventType::Test,
            &time::transaction_id("THU"),
            Metadata::default(),
        );
        self.inner.audit.event(&event, None);
        let item = Item {
            event: event.clone(),
            dedup_key: event.event_id.clone(),
            announce: false,
            attempts: 0,
            next_attempt: 0,
            enqueued_at: event.timestamp,
            last_error: String::new(),
        };
        match self.deliver(transport.as_ref(), &item) {
            Ok(()) => {
                self.update_state(|s| s.last_sent_at = Some(event.timestamp));
                self.set_connection("ok", "Đã gửi tin nhắn thử nghiệm.".into());
                self.inner.audit.delivery(&event, "sent", 1, None);
                tracing::info!("đã gửi tin nhắn thử nghiệm");
                Ok(())
            }
            Err(error) => {
                self.set_connection("error", error.message().to_owned());
                self.inner
                    .audit
                    .delivery(&event, "failed", 1, Some(error.message()));
                tracing::warn!(
                    kind = error.kind(),
                    "không gửi được tin nhắn thử nghiệm: {error}"
                );
                Err(error.message().to_owned())
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // Trạng thái cho giao diện
    // ------------------------------------------------------------------------------------------

    fn target(&self) -> String {
        match (&self.inner.admin, self.inner.mode) {
            (Some(a), Some(Mode::Gateway)) => format!("Gateway {}", a.gateway_url),
            (Some(a), Some(Mode::Direct)) => format!(
                "Telegram trực tiếp · nhóm {} · Bot Token {}",
                mask(&a.telegram_chat_id),
                mask(&a.telegram_bot_token)
            ),
            _ => String::new(),
        }
    }

    pub fn status(&self) -> StatusView {
        let state = lock(&self.inner.state).clone();
        let (pending, next_attempt, last_error) = {
            let queue = lock(&self.inner.queue);
            let head = queue.items.front();
            (
                queue.items.len(),
                head.map(|i| i.next_attempt),
                head.map(|i| i.last_error.clone()).filter(|e| !e.is_empty()),
            )
        };
        let connection = lock(&self.inner.connection).clone();
        let configured = self.inner.transport.is_some();
        let admin = self.admin_flags();
        let registration = match (state.consent, state.registration_delivered_at) {
            (Consent::Accepted, Some(_)) => "registered",
            (Consent::Accepted, None) => "pending",
            _ => "none",
        };
        StatusView {
            configured,
            config_source: self.inner.source.clone(),
            config_problem: self.inner.problem.clone(),
            mode: self.inner.mode.map(Mode::code),
            target: self.target(),
            locked: self.locked(),
            admin,
            prefs: state.prefs,
            effective: self.effective_flags(),
            consent: state.consent,
            needs_consent: configured && admin.enabled && state.consent == Consent::Pending,
            device_id: state.device_id.clone(),
            registration,
            registered_at: state.registered_at.map(time::display),
            // Chưa trả lời hộp đăng ký: gợi ý theo quản trị viên (`SEND_IP_DEFAULT`, `SEND_MAC_DEFAULT`).
            include_ip: if state.consent == Consent::Pending {
                self.inner.admin.as_ref().is_none_or(|a| a.send_ip_default)
            } else {
                state.include_ip
            },
            include_mac: if state.consent == Consent::Pending {
                self.inner
                    .admin
                    .as_ref()
                    .is_some_and(|a| a.send_mac_default)
            } else {
                state.include_mac
            },
            device: DeviceInfo::collect(&self.inner.app_version, true, true),
            connection: ConnectionView {
                state: connection.state,
                message: connection.message,
                checked_at: connection.checked_at.map(time::display),
            },
            pending,
            next_attempt_at: next_attempt
                .filter(|at| *at > time::now())
                .map(time::display),
            last_error,
            last_sent_at: state.last_sent_at.map(time::display),
            audit_dir: self.inner.audit.dir().display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::state::test_dir;
    use crate::transport::Delivery;

    /// Kênh gửi giả: trả lần lượt các kết quả đã dặn (hết thì thành công), ghi lại mọi lần gọi.
    #[derive(Default)]
    struct Fake {
        script: Mutex<VecDeque<Result<Delivery, SendError>>>,
        calls: Mutex<Vec<Call>>,
    }

    /// `(loại sự kiện, event_id, bí mật thiết bị đã dùng, announce)`.
    type Call = (EventType, String, Option<String>, bool);

    impl Fake {
        fn will(&self, results: impl IntoIterator<Item = Result<Delivery, SendError>>) {
            lock(&self.script).extend(results);
        }

        fn calls(&self) -> Vec<Call> {
            lock(&self.calls).clone()
        }

        fn types(&self) -> Vec<EventType> {
            self.calls().into_iter().map(|c| c.0).collect()
        }
    }

    impl Transport for Fake {
        fn send(&self, event: &Event, context: &SendContext<'_>) -> Result<Delivery, SendError> {
            lock(&self.calls).push((
                event.event_type,
                event.event_id.clone(),
                context.credential.map(str::to_owned),
                context.announce,
            ));
            lock(&self.script)
                .pop_front()
                .unwrap_or(Ok(Delivery::default()))
        }

        fn check(&self, _context: &SendContext<'_>) -> Result<String, SendError> {
            Ok("ok".into())
        }
    }

    fn admin(json: &str) -> Loaded {
        let config = AdminConfig::from_json(json).unwrap();
        Loaded {
            problem: config.problem(),
            config: Some(config),
            source: None,
        }
    }

    const DIRECT: &str = r#"{ "TELEGRAM_BOT_TOKEN": "1:fake", "TELEGRAM_CHAT_ID": "-1" }"#;
    const GATEWAY: &str = r#"{ "GATEWAY_URL": "http://127.0.0.1:9", "GATEWAY_ENROLL_KEY": "k" }"#;

    fn start(dir: &std::path::Path, admin: Loaded, fake: &Arc<Fake>) -> Notifier {
        Notifier::start_with(
            Options {
                dir: dir.to_path_buf(),
                app_version: "1.5.0".into(),
                user_name: "an.nv".into(),
                admin,
                sealer: Box::new(PlainSealer),
                transport: Some(fake.clone()),
            },
            false,
        )
    }

    fn sign_ok(tx: &str) -> Draft {
        let mut m = Metadata::default();
        m.set_documents(["a.pdf"]);
        Draft::new(EventType::SignSuccess, tx, m)
    }

    fn audit_lines(dir: &std::path::Path) -> Vec<serde_json::Value> {
        let folder = dir.join(crate::audit::FOLDER_NAME);
        let mut lines = Vec::new();
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            for line in std::fs::read_to_string(entry.path()).unwrap().lines() {
                lines.push(serde_json::from_str(line).unwrap());
            }
        }
        lines
    }

    #[test]
    fn auto_register_sends_device_info_without_asking() {
        let dir = test_dir("tu-dang-ky");
        let fake = Arc::new(Fake::default());
        let json = r#"{ "TELEGRAM_BOT_TOKEN": "1:fake", "TELEGRAM_CHAT_ID": "-1", "AUTO_REGISTER": true, "SEND_MAC_DEFAULT": true }"#;
        let n = start(&dir, admin(json), &fake);
        let s = n.status();
        assert!(!s.needs_consent, "không hỏi");
        assert_eq!(s.consent, Consent::Accepted);
        assert!(s.include_ip && s.include_mac);
        n.pump();
        assert_eq!(fake.types(), vec![EventType::DeviceRegistered]);
        assert_eq!(n.status().registration, "registered");
        // Mở lại: không đăng ký lần hai.
        drop(n);
        let again = start(&dir, admin(json), &fake);
        again.pump();
        assert_eq!(fake.types(), vec![EventType::DeviceRegistered]);
        // Đã từ chối (bản cũ): giữ nguyên, không tự gửi.
        let dir2 = test_dir("tu-dang-ky-tu-choi");
        let fake2 = Arc::new(Fake::default());
        start(&dir2, admin(DIRECT), &fake2).set_consent(false, false, false);
        let n2 = start(&dir2, admin(json), &fake2);
        n2.pump();
        assert!(fake2.calls().is_empty());
        assert_eq!(n2.status().consent, Consent::Declined);
    }

    #[test]
    fn nothing_is_sent_without_config_or_consent() {
        let dir = test_dir("consent");
        let fake = Arc::new(Fake::default());
        // Không có cấu hình: không hỏi, không gửi, nhưng sự kiện vẫn vào nhật ký trên máy.
        let none = Notifier::start_with(
            Options {
                dir: dir.clone(),
                app_version: "1.5.0".into(),
                user_name: "an.nv".into(),
                admin: Loaded::default(),
                sealer: Box::new(PlainSealer),
                transport: None,
            },
            false,
        );
        assert!(!none.status().configured && !none.status().needs_consent);
        none.emit(sign_ok("KS-0"));
        none.pump();

        // Có cấu hình, chưa trả lời: cần hỏi, chưa gửi.
        let n = start(&dir, admin(DIRECT), &fake);
        let s = n.status();
        assert!(s.configured && s.needs_consent && s.registration == "none");
        assert_eq!(s.mode, Some("direct"));
        assert!(
            !s.target.contains("fake"),
            "không lộ Bot Token: {}",
            s.target
        );
        n.emit(sign_ok("KS-1"));
        n.pump();
        assert!(fake.calls().is_empty());
        assert!(n.send_test().is_err(), "chưa đồng ý thì không gửi tin thử");

        // Từ chối: không hỏi lại, không gửi; mở lại ứng dụng vẫn vậy.
        n.set_consent(false, false, false);
        n.emit(sign_ok("KS-2"));
        n.pump();
        let again = start(&dir, admin(DIRECT), &fake);
        assert_eq!(again.status().consent, Consent::Declined);
        assert!(!again.status().needs_consent);
        again.emit(sign_ok("KS-3"));
        again.pump();
        assert!(fake.calls().is_empty(), "từ chối thì không có lời gọi nào");
        assert_eq!(again.status().pending, 0);

        let lines = audit_lines(&dir);
        let skipped: Vec<_> = lines
            .iter()
            .filter(|l| l["kind"] == "event" && l["disposition"] == "skipped")
            .collect();
        assert_eq!(skipped.len(), 4, "mọi sự kiện đều được ghi nhận trên máy");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn registration_is_sent_once_and_survives_restart() {
        let dir = test_dir("register");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(DIRECT), &fake);
        let device_id = n.status().device_id;
        n.set_consent(true, true, false);
        assert_eq!(n.status().registration, "pending");
        n.set_consent(true, true, false); // bấm hai lần
        n.pump();
        assert_eq!(fake.types(), [EventType::DeviceRegistered]);
        let s = n.status();
        assert_eq!(s.registration, "registered");
        assert!(s.registered_at.is_some() && s.last_sent_at.is_some());
        assert_eq!(s.connection.state, "ok");

        // Mở lại ứng dụng: cùng mã thiết bị, không đăng ký lại, không hỏi lại.
        let again = start(&dir, admin(DIRECT), &fake);
        again.pump();
        let s = again.status();
        assert_eq!(s.device_id, device_id);
        assert!(!s.needs_consent && s.registration == "registered");
        again.set_consent(true, false, false);
        again.pump();
        assert_eq!(
            fake.types(),
            [EventType::DeviceRegistered],
            "không đăng ký trùng"
        );

        // Hủy đăng ký: báo một lần, sau đó không gửi nữa.
        again.emit(sign_ok("KS-truoc-khi-huy"));
        again.unregister();
        again.pump();
        again.emit(sign_ok("KS-sau-khi-huy"));
        again.pump();
        assert_eq!(
            fake.types(),
            [EventType::DeviceRegistered, EventType::DeviceUnregistered]
        );
        assert_eq!(again.status().registration, "none");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn offline_events_wait_in_order_and_are_not_duplicated() {
        let dir = test_dir("offline");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(DIRECT), &fake);
        n.set_consent(true, false, false);
        n.pump();

        // Mất mạng: ký vẫn xong (emit không chặn, không lỗi), sự kiện nằm trong hàng đợi.
        fake.will([Err(SendError::Network("mất mạng".into()))]);
        n.emit(sign_ok("KS-1"));
        n.emit(sign_ok("KS-1")); // nghiệp vụ báo trùng cùng một giao dịch
        n.emit(sign_ok("KS-2"));
        let wait = n.pump();
        assert!(wait >= Duration::from_secs(1) && wait <= Duration::from_secs(10));
        let s = n.status();
        assert_eq!(s.pending, 2, "giao dịch trùng không vào hàng đợi hai lần");
        assert_eq!(s.connection.state, "error");
        assert_eq!(s.last_error.as_deref(), Some("mất mạng"));
        assert!(s.next_attempt_at.is_some());
        assert_eq!(
            fake.calls().len(),
            2,
            "đầu hàng lỗi thì chưa thử sự kiện sau"
        );
        n.pump();
        assert_eq!(fake.calls().len(), 2, "chưa tới lượt thì không gọi máy chủ");

        // Tắt ứng dụng khi còn hàng đợi, mở lại khi đã có mạng: gửi đủ, đúng thứ tự, mỗi sự kiện một lần.
        let again = start(&dir, admin(DIRECT), &fake);
        assert_eq!(again.status().pending, 2);
        again.pump();
        let calls = fake.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(
            calls[1].1, calls[2].1,
            "gửi lại đúng sự kiện đã lỗi (cùng event_id)"
        );
        assert_ne!(calls[2].1, calls[3].1);
        assert_eq!(again.status().pending, 0);
        again.flush();
        again.pump();
        again.emit(sign_ok("KS-2")); // đã gửi rồi: không gửi lại dù mở lại ứng dụng
        again.pump();
        assert_eq!(fake.calls().len(), 4, "không gửi trùng");

        let lines = audit_lines(&dir);
        let results: Vec<_> = lines
            .iter()
            .filter(|l| l["kind"] == "delivery")
            .map(|l| l["result"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(results, ["sent", "retry", "sent", "sent"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bad_token_keeps_events_until_fixed_and_rejected_message_is_dropped() {
        let dir = test_dir("config");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(DIRECT), &fake);
        n.set_consent(true, false, false);
        n.pump();

        fake.will([
            Err(SendError::Config("Telegram từ chối Bot Token".into())),
            Err(SendError::RateLimited(120, "chậm lại".into())),
            Err(SendError::Rejected("tin không hợp lệ".into())),
        ]);
        n.emit(sign_ok("KS-1"));
        n.emit(sign_ok("KS-2"));
        n.pump();
        assert_eq!(
            n.status().pending,
            2,
            "token sai: giữ lại chờ quản trị viên sửa"
        );
        assert_eq!(n.status().connection.state, "error");
        n.flush();
        let wait = n.pump();
        assert!(
            wait > Duration::from_secs(60),
            "tôn trọng retry_after: {wait:?}"
        );
        n.flush();
        n.pump();
        // Lần 3: KS-1 bị từ chối riêng → bỏ; KS-2 gửi được.
        assert_eq!(n.status().pending, 0);
        assert_eq!(fake.calls().len(), 5);
        assert_eq!(n.status().connection.state, "ok");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn switches_filter_events_and_admin_lock_wins() {
        let dir = test_dir("flags");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(DIRECT), &fake);
        n.set_consent(true, false, false);
        n.pump();
        n.set_prefs(Flags {
            failure: false,
            document: false,
            ..Flags::default()
        })
        .unwrap();
        n.emit(Draft::new(
            EventType::SignFailed,
            "KS-1",
            Metadata::default(),
        ));
        n.emit(Draft::new(
            EventType::SignCancelled,
            "KS-1",
            Metadata::default(),
        ));
        n.emit(Draft::new(
            EventType::DocumentCreated,
            "TVB-1",
            Metadata::default(),
        ));
        n.emit(sign_ok("KS-2"));
        n.pump();
        assert_eq!(
            fake.types(),
            [EventType::DeviceRegistered, EventType::SignSuccess]
        );
        n.set_prefs(Flags {
            enabled: false,
            ..Flags::default()
        })
        .unwrap();
        n.emit(sign_ok("KS-3"));
        n.pump();
        assert_eq!(fake.calls().len(), 2, "tắt công tắc chung thì không gửi gì");
        assert!(n.send_test().is_err());

        // Quản trị viên tắt thông báo ký và khoá: người dùng không bật lại được.
        let dir2 = test_dir("flags-lock");
        let fake2 = Arc::new(Fake::default());
        let locked = start(
            &dir2,
            admin(
                r#"{ "TELEGRAM_BOT_TOKEN": "1:fake", "TELEGRAM_CHAT_ID": "-1", "SIGN_NOTIFICATION_ENABLED": false,
                       "DEVICE_REGISTRATION_ENABLED": false, "LOCK_SETTINGS": true }"#,
            ),
            &fake2,
        );
        assert!(locked.set_prefs(Flags::default()).is_err());
        locked.set_consent(true, false, false);
        locked.emit(sign_ok("KS-1"));
        locked.emit(Draft::new(
            EventType::DocumentCreated,
            "TVB-1",
            Metadata::default(),
        ));
        locked.pump();
        assert_eq!(fake2.types(), [EventType::DocumentCreated]);
        assert_eq!(
            locked.status().registration,
            "registered",
            "tắt thông báo đăng ký: ghi nhận trên máy, không gửi"
        );
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(dir2).unwrap();
    }

    #[test]
    fn gateway_keeps_device_secret_and_reregisters_when_unknown() {
        let dir = test_dir("gateway");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(GATEWAY), &fake);
        assert_eq!(n.status().mode, Some("gateway"));
        fake.will([Ok(Delivery {
            credential: Some("bi-mat-1".into()),
        })]);
        n.set_consent(true, false, false);
        n.emit(sign_ok("KS-1"));
        n.pump();
        let calls = fake.calls();
        assert_eq!(calls[0].0, EventType::DeviceRegistered);
        assert!(calls[0].3, "đăng ký có báo lên Telegram");
        assert_eq!(
            calls[1].2.as_deref(),
            Some("bi-mat-1"),
            "sự kiện sau dùng bí mật được cấp"
        );

        // Gateway mất dữ liệu thiết bị: đăng ký lại một lần rồi gửi lại đúng sự kiện.
        fake.will([
            Err(SendError::Unregistered("không biết thiết bị".into())),
            Ok(Delivery {
                credential: Some("bi-mat-2".into()),
            }),
        ]);
        n.emit(sign_ok("KS-2"));
        n.pump();
        let calls = fake.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[3].0, EventType::DeviceRegistered);
        assert_eq!(calls[4].0, EventType::SignSuccess);
        assert_eq!(calls[4].2.as_deref(), Some("bi-mat-2"));
        assert_eq!(calls[2].1, calls[4].1);
        assert_eq!(n.status().pending, 0);

        // Thiết bị bị thu hồi ở gateway: lỗi cấu hình, giữ trong hàng đợi (có giới hạn), không lặp đăng ký.
        fake.will([Err(SendError::Config("thiết bị đã bị thu hồi".into()))]);
        n.emit(sign_ok("KS-3"));
        n.pump();
        assert_eq!(fake.calls().len(), 6);
        assert_eq!(n.status().pending, 1);

        // Tắt thông báo đăng ký ở chế độ gateway: vẫn đăng ký để lấy bí mật, nhưng không báo lên Telegram.
        let dir2 = test_dir("gateway-silent");
        let fake2 = Arc::new(Fake::default());
        let quiet = start(
            &dir2,
            admin(
                r#"{ "GATEWAY_URL": "http://127.0.0.1:9", "GATEWAY_ENROLL_KEY": "k", "DEVICE_REGISTRATION_ENABLED": false }"#,
            ),
            &fake2,
        );
        quiet.set_consent(true, false, false);
        quiet.pump();
        let calls = fake2.calls();
        assert_eq!(calls.len(), 1);
        assert!(!calls[0].3, "announce = false");
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(dir2).unwrap();
    }

    #[test]
    fn app_update_is_reported_once() {
        let dir = test_dir("update");
        let fake = Arc::new(Fake::default());
        let n = start(&dir, admin(DIRECT), &fake);
        n.set_consent(true, false, false);
        n.pump();
        let newer = |version: &str| {
            Notifier::start_with(
                Options {
                    dir: dir.clone(),
                    app_version: version.into(),
                    user_name: "an.nv".into(),
                    admin: admin(DIRECT),
                    sealer: Box::new(PlainSealer),
                    transport: Some(fake.clone()),
                },
                false,
            )
        };
        newer("1.5.1").pump();
        newer("1.5.1").pump();
        assert_eq!(
            fake.types(),
            [EventType::DeviceRegistered, EventType::AppUpdated]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
