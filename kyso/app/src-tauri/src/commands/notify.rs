//! Thông báo Telegram và đăng ký thiết bị: nối `smks-notify` với IPC và với các lô nghiệp vụ.
//!
//! Nghiệp vụ (ký số, tạo văn bản) không biết gì về Telegram. Ở đây có hai bộ gom ([`SignReport`], [`MergeReport`])
//! nghe các sự kiện tiến độ sẵn có của lô và, **khi lô đã kết thúc**, báo cho [`Notifier`] tối đa ba sự kiện của
//! lô đó: thành công, thất bại, bị hủy. Một lô 200 văn bản vì vậy ra một tin nhắn, không phải 200.
//!
//! [`Notifier::emit`] chỉ ghi nhật ký và xếp hàng rồi về ngay; gửi chạy ở luồng nền. Mọi lỗi của Telegram dừng lại ở
//! đó, không quay lại kết quả ký.

use std::path::Path;

use smks_core::consts::app::APP_FOLDER_NAME;
use smks_core::{ErrorCode, SmksError};
use smks_merge::batch::BatchEvent;
use smks_notify::config;
use smks_notify::event::{EventType, Failure, Metadata};
use smks_notify::service::{Draft, Notifier, Options, Sealer, StatusView};
use smks_notify::state::Flags;
use smks_notify::time;
use smks_sign::job::JobEvent;
use tauri::State;

use super::blocking;

/// Mục đích (entropy DPAPI) của bí mật thiết bị do gateway cấp.
const SECRET_PURPOSE: &str = "notify/gateway-device";

/// Bí mật thiết bị được mã hoá DPAPI theo tài khoản Windows trước khi ghi `notify-state.json`.
struct DpapiSealer;

impl Sealer for DpapiSealer {
    fn seal(&self, plain: &str) -> Option<String> {
        smks_crypto::pin_vault::seal_secret(SECRET_PURPOSE, plain)
            .inspect_err(|e| tracing::warn!("{}", e.message))
            .ok()
    }

    fn open(&self, sealed: &str) -> Option<String> {
        smks_crypto::pin_vault::open_secret(SECRET_PURPOSE, sealed)
            .ok()
            .map(|plain| plain.as_str().to_owned())
    }
}

/// Trạng thái dùng chung: bộ ghi nhận sự kiện và gửi thông báo.
#[derive(Debug)]
pub struct NotifyState {
    notifier: Notifier,
}

impl NotifyState {
    /// Đọc cấu hình quản trị (`notify.json`, nếu có) và khởi động luồng gửi nền. Không có cấu hình thì bộ ghi nhận
    /// vẫn chạy (nhật ký sự kiện trên máy), nhưng không hỏi đăng ký và không gửi gì.
    pub fn start() -> Self {
        // Bài kiểm tự động chạy với thư mục dữ liệu riêng: khi đó chỉ nhận cấu hình ở biến SMKS_NOTIFY_CONFIG, để
        // cấu hình thật trên máy không lẫn vào bài kiểm.
        let test_mode = std::env::var_os("SMKS_CONFIG_DIR").is_some_and(|d| !d.is_empty());
        let dir =
            super::config::app_dir().unwrap_or_else(|_| std::env::temp_dir().join(APP_FOLDER_NAME));
        let admin = config::load(test_mode);
        if let Some(problem) = &admin.problem {
            tracing::warn!("cấu hình thông báo: {problem}");
        }
        Self {
            notifier: Notifier::start(Options {
                dir,
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                user_name: std::env::var("USERNAME").unwrap_or_default(),
                admin,
                sealer: Box::new(DpapiSealer),
                transport: None,
            }),
        }
    }

    pub fn notifier(&self) -> &Notifier {
        &self.notifier
    }
}

// ----------------------------------------------------------------------------------------------
// Lô ký
// ----------------------------------------------------------------------------------------------

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Người dùng bấm Hủy ở hộp PIN của middleware (`SCARD_W_CANCELLED_BY_USER`, `ERROR_CANCELLED`).
fn is_user_cancel(error: &SmksError) -> bool {
    let m = error.message.to_ascii_lowercase();
    m.contains("8010006e") || m.contains("800704c7") || m.contains("cancelled by the user")
}

/// Gom kết quả của một lô ký; báo một lần khi lô kết thúc.
#[derive(Debug)]
pub struct SignReport {
    transaction_id: String,
    signer: String,
    key_kind: &'static str,
    signed: Vec<String>,
    timestamped: usize,
    failed: Vec<Failure>,
    total: usize,
    paused: usize,
    stopped: bool,
    token_removed: bool,
}

impl SignReport {
    /// `signer`: tên trên chứng thư ký (thông tin công khai của chữ ký).
    pub fn new(signer: String, hardware: bool, dev: bool) -> Self {
        Self {
            transaction_id: time::transaction_id("KS"),
            signer,
            key_kind: if dev {
                "khoá thử"
            } else if hardware {
                "USB Token"
            } else {
                "chứng thư trên máy"
            },
            signed: Vec::new(),
            timestamped: 0,
            failed: Vec::new(),
            total: 0,
            paused: 0,
            stopped: false,
            token_removed: false,
        }
    }

    /// Nghe sự kiện tiến độ của lô. Văn bản chỉ được tính "đã ký" khi lõi báo `success` (đã ký và kiểm lại xong).
    pub fn observe(&mut self, event: &JobEvent) {
        match event {
            JobEvent::Manifest { total, .. } => self.total = *total,
            JobEvent::File { paused: true, .. } => {}
            JobEvent::File {
                name,
                success: true,
                trusted_time,
                ..
            } => {
                self.signed.push(name.clone());
                self.timestamped += usize::from(trusted_time.is_some());
            }
            JobEvent::File { name, reason, .. } => self.failed.push(Failure {
                name: name.clone(),
                code: String::new(),
                message: reason.clone().unwrap_or_default(),
            }),
            JobEvent::Summary {
                total,
                paused,
                stopped,
                token_removed,
                ..
            } => {
                self.total = *total;
                self.paused = *paused;
                self.stopped = *stopped;
                self.token_removed = *token_removed;
            }
        }
    }

    fn signature_type(&self) -> String {
        let level = if self.timestamped == self.signed.len() {
            "PAdES B-T (có dấu thời gian)"
        } else if self.timestamped == 0 {
            "PAdES B-B (không có dấu thời gian)"
        } else {
            "PAdES (một số văn bản không có dấu thời gian)"
        };
        format!("{level} · {}", self.key_kind)
    }

    /// Lô đã kết thúc: báo thành công / thất bại / bị hủy theo những gì thật sự xảy ra.
    pub fn finish(mut self, notifier: &Notifier) {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        // Lô ký song song nên file xong không theo thứ tự: xếp lại theo tên cho dễ đọc.
        self.signed.sort();
        self.failed.sort_by(|a, b| a.name.cmp(&b.name));
        let tx = self.transaction_id.clone();
        let tx = tx.as_str();
        if !self.signed.is_empty() {
            let mut m = Metadata {
                signature_type: self.signature_type(),
                signer: self.signer.clone(),
                total_count: count(self.total),
                ..Metadata::default()
            };
            m.set_documents(self.signed.iter().cloned());
            notifier.emit(Draft::new(EventType::SignSuccess, tx, m));
        }
        if !self.failed.is_empty() || self.token_removed {
            let (code, message) = if self.token_removed {
                (
                    ErrorCode::TokenRemoved,
                    ErrorCode::TokenRemoved.default_message().to_owned(),
                )
            } else {
                (
                    ErrorCode::SignFailed,
                    self.failed
                        .first()
                        .map(|f| f.message.clone())
                        .unwrap_or_default(),
                )
            };
            let mut m = Metadata {
                signer: self.signer.clone(),
                error_code: code.as_u32().to_string(),
                error_message: message,
                total_count: count(self.total),
                pending_count: if self.token_removed {
                    count(self.paused)
                } else {
                    0
                },
                ..Metadata::default()
            };
            m.set_failures(self.failed.iter().cloned());
            notifier.emit(Draft::new(EventType::SignFailed, tx, m));
        }
        if self.stopped && !self.token_removed && self.paused > 0 {
            let m = Metadata {
                signer: self.signer,
                total_count: count(self.total),
                pending_count: count(self.paused),
                note: format!(
                    "Người dùng bấm Tạm dừng; đã ký {} văn bản.",
                    self.signed.len()
                ),
                ..Metadata::default()
            };
            notifier.emit(Draft::new(EventType::SignCancelled, tx, m));
        }
    }
}

/// Lô ký không bắt đầu được (cấu hình sai, không mở được token, người dùng hủy ở hộp PIN).
pub fn sign_start_failed(notifier: &Notifier, files: &[String], error: &SmksError) {
    let mut m = Metadata {
        error_code: error.code.as_u32().to_string(),
        error_message: error.message.clone(),
        ..Metadata::default()
    };
    m.set_documents(files.iter().cloned());
    let event_type = if is_user_cancel(error) {
        EventType::SignCancelled
    } else {
        EventType::SignFailed
    };
    notifier.emit(Draft::new(event_type, &time::transaction_id("KS"), m));
}

/// Mở token / ký thử ở hộp Xác nhận ký thất bại (PIN sai, token bị khoá, không thấy chứng thư, hủy hộp PIN).
pub fn token_failed(notifier: &Notifier, error: &SmksError) {
    let cancelled = is_user_cancel(error);
    let m = Metadata {
        error_code: error.code.as_u32().to_string(),
        error_message: error.message.clone(),
        note: if cancelled {
            "Dừng ở bước mở USB Token, chưa ký văn bản nào.".into()
        } else {
            String::new()
        },
        ..Metadata::default()
    };
    let event_type = if cancelled {
        EventType::SignCancelled
    } else {
        EventType::SignFailed
    };
    notifier.emit(Draft::new(event_type, &time::transaction_id("KS"), m));
}

// ----------------------------------------------------------------------------------------------
// Lô tạo văn bản
// ----------------------------------------------------------------------------------------------

/// `Giấy mời.docx → PDF`.
pub fn document_type(template_path: &str, format_extension: &str) -> String {
    format!(
        "{} → {}",
        file_name(Path::new(template_path)),
        format_extension
            .trim_start_matches('.')
            .to_ascii_uppercase()
    )
}

/// Gom kết quả của một lô tạo văn bản; báo một lần khi lô kết thúc.
#[derive(Debug)]
pub struct MergeReport {
    transaction_id: String,
    document_type: String,
    created: Vec<String>,
    failed: Vec<Failure>,
    total: usize,
    stopped: bool,
}

impl MergeReport {
    pub fn new(document_type: String, total: usize) -> Self {
        Self {
            transaction_id: time::transaction_id("TVB"),
            document_type,
            created: Vec::new(),
            failed: Vec::new(),
            total,
            stopped: false,
        }
    }

    /// Nghe sự kiện tiến độ của lô. Văn bản chỉ được tính "đã tạo" khi lõi báo `ok` (file đã nằm ở thư mục lưu).
    pub fn observe(&mut self, event: &BatchEvent) {
        match event {
            BatchEvent::Manifest { total, .. } => self.total = *total,
            BatchEvent::File {
                file_name,
                status,
                reason,
                ..
            } => {
                if *status == "ok" {
                    self.created.push(file_name.clone());
                } else {
                    self.failed.push(Failure {
                        name: file_name.clone(),
                        code: String::new(),
                        message: reason.clone().unwrap_or_default(),
                    });
                }
            }
            BatchEvent::Summary { total, stopped, .. } => {
                self.total = *total;
                self.stopped = *stopped;
            }
        }
    }

    pub fn finish(self, notifier: &Notifier) {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let tx = self.transaction_id.as_str();
        if !self.created.is_empty() {
            let mut m = Metadata {
                document_type: self.document_type.clone(),
                total_count: count(self.total),
                ..Metadata::default()
            };
            m.set_documents(self.created.iter().cloned());
            notifier.emit(Draft::new(EventType::DocumentCreated, tx, m));
        }
        if !self.failed.is_empty() {
            let mut m = Metadata {
                document_type: self.document_type.clone(),
                error_code: ErrorCode::MergeFailed.as_u32().to_string(),
                error_message: self
                    .failed
                    .first()
                    .map(|f| f.message.clone())
                    .unwrap_or_default(),
                total_count: count(self.total),
                ..Metadata::default()
            };
            m.set_failures(self.failed.iter().cloned());
            notifier.emit(Draft::new(EventType::DocumentFailed, tx, m));
        }
        let pending = self
            .total
            .saturating_sub(self.created.len() + self.failed.len());
        if self.stopped && pending > 0 {
            let m = Metadata {
                document_type: self.document_type,
                total_count: count(self.total),
                pending_count: count(pending),
                ..Metadata::default()
            };
            notifier.emit(Draft::new(EventType::DocumentCancelled, tx, m));
        }
    }
}

/// Lô tạo văn bản không bắt đầu được (mẫu / dữ liệu / cấu hình ghép không hợp lệ).
pub fn merge_start_failed(notifier: &Notifier, document_type: String, error: &SmksError) {
    let m = Metadata {
        document_type,
        error_code: error.code.as_u32().to_string(),
        error_message: error.message.clone(),
        ..Metadata::default()
    };
    notifier.emit(Draft::new(
        EventType::DocumentFailed,
        &time::transaction_id("TVB"),
        m,
    ));
}

// ----------------------------------------------------------------------------------------------
// IPC của hộp đăng ký thiết bị và hộp cấu hình thông báo
// ----------------------------------------------------------------------------------------------

/// Trạng thái thông báo: đã cấu hình chưa, người dùng đã trả lời đăng ký chưa, kết nối, hàng đợi. Không trả Bot
/// Token hay Chat ID đầy đủ.
#[tauri::command]
pub async fn notify_status(state: State<'_, NotifyState>) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || Ok(notifier.status())).await
}

/// Người dùng trả lời hộp đăng ký thiết bị. Đồng ý thì gửi thông tin thiết bị **một lần**; `include_ip` /
/// `include_mac` là lựa chọn của người dùng. Từ chối thì không gửi gì và không hỏi lại.
#[tauri::command]
pub async fn notify_consent(
    state: State<'_, NotifyState>,
    accept: bool,
    include_ip: Option<bool>,
    include_mac: Option<bool>,
) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || {
        notifier.set_consent(
            accept,
            include_ip.unwrap_or(false),
            include_mac.unwrap_or(false),
        );
        Ok(notifier.status())
    })
    .await
}

/// Hủy đăng ký thiết bị: báo một lần rồi ngừng gửi.
#[tauri::command]
pub async fn notify_unregister(state: State<'_, NotifyState>) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || {
        notifier.unregister();
        Ok(notifier.status())
    })
    .await
}

/// Đổi các công tắc bật / tắt của người dùng. Lỗi khi quản trị viên đã khoá.
#[tauri::command]
pub async fn notify_set_prefs(
    state: State<'_, NotifyState>,
    prefs: Flags,
) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || {
        notifier
            .set_prefs(prefs)
            .map_err(|m| SmksError::new(ErrorCode::BadRequest, m))?;
        Ok(notifier.status())
    })
    .await
}

/// Kiểm tra kết nối tới Telegram / gateway; kết quả nằm trong `connection` của trạng thái trả về.
#[tauri::command]
pub async fn notify_check(state: State<'_, NotifyState>) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || {
        notifier.check_connection();
        Ok(notifier.status())
    })
    .await
}

/// Gửi một tin nhắn thử nghiệm.
#[tauri::command]
pub async fn notify_test(state: State<'_, NotifyState>) -> Result<StatusView, SmksError> {
    let notifier = state.notifier.clone();
    blocking(move || {
        notifier
            .send_test()
            .map_err(|m| SmksError::new(ErrorCode::NotifyFailed, m))?;
        Ok(notifier.status())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, success: bool, paused: bool, stamped: bool) -> JobEvent {
        JobEvent::File {
            index: 0,
            name: name.into(),
            success,
            output_path: None,
            reason: (!success).then(|| "File PDF được mã hóa".into()),
            verdict: None,
            trusted_time: stamped.then(|| "20/11/2026 10:00:00".into()),
            paused,
            done: 0,
            total: 0,
        }
    }

    #[test]
    fn sign_report_counts_only_finished_files() {
        let mut r = SignReport::new("Nguyễn Văn A".into(), true, false);
        r.observe(&file("a.pdf", true, false, true));
        r.observe(&file("b.pdf", false, false, false));
        r.observe(&file("c.pdf", false, true, false));
        r.observe(&JobEvent::Summary {
            job_id: "ky-1".into(),
            total: 3,
            success: 1,
            failed: 1,
            paused: 1,
            stopped: true,
            token_removed: false,
            elapsed_ms: 1,
            output_folder: String::new(),
            clock_note: None,
        });
        assert_eq!(r.signed, ["a.pdf"]);
        assert_eq!(r.failed.len(), 1, "văn bản tạm dừng không phải lỗi");
        assert_eq!(r.failed[0].name, "b.pdf");
        assert_eq!((r.paused, r.stopped, r.total), (1, true, 3));
        assert_eq!(
            r.signature_type(),
            "PAdES B-T (có dấu thời gian) · USB Token"
        );
        assert!(r.transaction_id.starts_with("KS-"));
    }

    #[test]
    fn cancel_is_recognised_from_middleware_codes() {
        let cancel = SmksError::new(
            ErrorCode::SignTokenInvalid,
            "Không dùng được khóa bí mật để ký: The action was cancelled by the user. (0x8010006E)",
        );
        let wrong_pin = SmksError::from(ErrorCode::PinIncorrect);
        assert!(is_user_cancel(&cancel));
        assert!(!is_user_cancel(&wrong_pin));
        assert_eq!(
            document_type(r"C:\mau\Giấy mời.docx", ".pdf"),
            "Giấy mời.docx → PDF"
        );
    }
}
