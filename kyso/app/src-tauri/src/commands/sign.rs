//! Command của màn Ký số. Nghiệp vụ nằm ở `smks-sign` / `smks-crypto`; ở đây chỉ nối IPC, phiên token, job.
//!
//! Luồng (kế hoạch 6.6): `cert_list` → người dùng chọn chứng thư trong hộp "Xác nhận ký" → `token_check`
//! (ký thử, lúc này middleware hỏi PIN; khoá được giữ cho cả lô để không hỏi PIN hai lần) → `sign_run`
//! (chạy nền, tiến độ qua Channel) → `job_cancel` khi bấm Dừng.
//!
//! Mã PIN (D2, spike S6): hộp Xác nhận ký có thể gửi PIN người dùng gõ, hoặc dùng PIN đã lưu (DPAPI,
//! [`super::pin`]). PIN được đưa cho middleware trước khi ký thử; middleware không nhận thì tự hiện hộp PIN như cũ.
//! PIN sai thì báo lỗi ngay, PIN đã lưu thì xoá, **không bao giờ tự thử lại** (tránh khoá token).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use smks_core::config::{Placement, SignatureMode};
use smks_core::{ErrorCode, SmksError};
use smks_crypto::cert::Cert;
use smks_crypto::key::SigningKey;
use smks_crypto::store::{CertScan, PinHandoff, StoreKey};
use smks_crypto::tsa::TsaClient;
use smks_sign::job::{Appearance, FileItem, JobEvent, JobRequest, OutputOptions, PlannedOutput};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use zeroize::Zeroizing;

use super::blocking;
use super::notify::{NotifyState, SignReport};

/// Khoá đã mở và ký thử thành công, dùng lại cho các lô sau tới khi đổi chứng thư.
struct Session {
    thumbprint: String,
    key: Arc<dyn SigningKey>,
    chain: Vec<Cert>,
    /// Khoá thử (feature `dev-keys`): không đòi kết quả "Hợp lệ" vì CA thử không thuộc Ban Cơ yếu.
    dev: bool,
}

/// Trạng thái dùng chung của màn Ký số.
pub struct SignState {
    session: Mutex<Option<Session>>,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    next_id: AtomicU64,
    tsa: Arc<TsaClient>,
    /// File PDF giao diện được phép đọc qua asset protocol (đã liệt kê hoặc là kết quả ký).
    previewable: Mutex<HashSet<PathBuf>>,
}

impl std::fmt::Debug for SignState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignState").finish_non_exhaustive()
    }
}

impl Default for SignState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            jobs: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            tsa: Arc::new(tsa_client()),
            previewable: Mutex::new(HashSet::new()),
        }
    }
}

/// Máy chủ dấu thời gian Ban Cơ yếu. Chỉ bản build thử (`dev-keys`) cho đổi địa chỉ qua `SMKS_TSA_URL` để kiểm
/// trường hợp mất mạng (TD-54); bản phát hành luôn dùng `tsa.ca.gov.vn`.
fn tsa_client() -> TsaClient {
    #[cfg(feature = "dev-keys")]
    if let Ok(url) = std::env::var("SMKS_TSA_URL")
        && !url.trim().is_empty()
    {
        return TsaClient::new(url.trim());
    }
    TsaClient::default()
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl SignState {
    /// Dừng lô ký `job_id`; trả về có lô đó không.
    pub fn cancel(&self, job_id: &str) -> bool {
        lock(&self.jobs)
            .get(job_id)
            .map(|f| f.store(true, Ordering::SeqCst))
            .is_some()
    }

    fn allow_preview(&self, paths: impl IntoIterator<Item = PathBuf>) {
        lock(&self.previewable).extend(paths);
    }
}

// ----------------------------------------------------------------------------------------------
// Chứng thư và token
// ----------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn cert_list() -> Result<CertScan, SmksError> {
    blocking(|| {
        #[cfg_attr(not(feature = "dev-keys"), allow(unused_mut))]
        let mut scan =
            smks_crypto::store::list_certificates(&smks_crypto::chain::is_trusted_signer);
        #[cfg(feature = "dev-keys")]
        if let Some(entry) = dev::entry() {
            scan.certificates.insert(0, entry);
        }
        for c in &scan.certificates {
            tracing::info!(
                cn = %c.common_name, source = ?c.source, can_sign = c.can_sign,
                provider = c.key_provider.as_deref().unwrap_or("-"), "chứng thư"
            );
        }
        Ok(scan)
    })
    .await
}

/// Thông tin token sau khi ký thử thành công.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub thumbprint: String,
    pub common_name: String,
    pub issuer_common_name: String,
    pub provider: String,
    pub hardware: bool,
    pub valid_to: i64,
    /// Khoá thử (chỉ khi build với feature `dev-keys`).
    pub dev: bool,
    /// PIN dùng lần mở này: `typed` (vừa gõ), `saved` (đã lưu), không có thì middleware tự hỏi.
    pub pin_source: Option<&'static str>,
    /// Middleware có nhận PIN do ứng dụng đưa vào không (spike S6): `accepted` / `notSupported`.
    pub pin_handoff: Option<&'static str>,
    /// Vừa lưu PIN trên máy (người dùng chọn "Lưu PIN" và middleware đã nhận PIN).
    pub pin_saved: bool,
    /// Lý do không lưu PIN dù người dùng đã chọn.
    pub pin_note: Option<String>,
}

/// Mở khoá và ký thử. `pin`: đưa cho middleware trước khi ký thử (nếu có); trả về middleware có nhận không.
fn open_session(
    thumbprint: &str,
    pin: Option<&str>,
) -> Result<(Session, Option<PinHandoff>), SmksError> {
    #[cfg(feature = "dev-keys")]
    if let Some(session) = dev::session(thumbprint)? {
        // Khoá thử không có PIN; giả lập kết quả để bài kiểm tự động đi hết luồng lưu PIN.
        return Ok((session, pin.map(|_| PinHandoff::Accepted)));
    }
    let key = StoreKey::open(thumbprint)?;
    if !smks_crypto::chain::is_trusted_signer(key.certificate()) {
        return Err(SmksError::new(
            ErrorCode::SignTokenInvalid,
            "Chứng thư số không thuộc CA Ban Cơ yếu (chain không về Root G1 lẫn Root G2 đã ghim). Chữ ký tạo bằng chứng thư này sẽ bị đánh giá là không hợp lệ.",
        ));
    }
    let handoff = pin.map(|p| key.set_pin(p)).transpose()?;
    if let Some(h) = handoff {
        tracing::info!(provider = %key.provider(), "đưa mã PIN cho middleware: {h:?}");
    }
    key.preflight()?;
    let chain = smks_crypto::chain::intermediates_for(key.certificate());
    Ok((
        Session {
            thumbprint: thumbprint.to_owned(),
            key: Arc::new(key),
            chain,
            dev: false,
        },
        handoff,
    ))
}

/// PIN dùng khi mở token: PIN người dùng vừa gõ, không có thì PIN đã lưu.
enum PinSource {
    None,
    Typed(Zeroizing<String>),
    Saved(Zeroizing<String>),
}

/// Mở phiên với PIN theo `source`. PIN đã lưu mà token báo sai / bị khoá thì xoá PIN đã lưu và báo, không thử lại.
fn open_with(
    thumbprint: &str,
    source: &PinSource,
) -> Result<(Session, Option<PinHandoff>), SmksError> {
    let pin = match source {
        PinSource::None => None,
        PinSource::Typed(p) | PinSource::Saved(p) => Some(p.as_str()),
    };
    open_session(thumbprint, pin).map_err(|mut e| {
        if matches!(source, PinSource::Saved(_))
            && matches!(e.code, ErrorCode::PinIncorrect | ErrorCode::PinBlocked)
        {
            let _ = super::pin::forget(thumbprint);
            e.message = format!(
                "{} Mã PIN đã lưu trên máy không còn đúng nên đã được xoá; hãy nhập PIN mới.",
                e.message
            );
        }
        e
    })
}

fn token_info(session: &Session) -> TokenInfo {
    let cert = session.key.certificate();
    TokenInfo {
        thumbprint: session.thumbprint.clone(),
        common_name: cert.common_name(),
        issuer_common_name: cert.issuer_common_name(),
        provider: session.key.provider(),
        hardware: smks_crypto::store::is_hardware_provider(&session.key.provider()),
        valid_to: cert.validity().1,
        dev: session.dev,
        pin_source: None,
        pin_handoff: None,
        pin_saved: false,
        pin_note: None,
    }
}

/// Mở khoá và ký thử (middleware hỏi PIN ở đây, trừ khi nhận PIN do ứng dụng đưa vào). Giữ phiên cho các lô
/// sau. `pin`: PIN người dùng gõ ở hộp Xác nhận ký (trống thì dùng PIN đã lưu, nếu có); `save_pin`: lưu PIN vừa gõ
/// trên máy (DPAPI) khi mở được token và middleware đã nhận PIN.
#[tauri::command]
pub async fn token_check(
    state: State<'_, SignState>,
    notify: State<'_, NotifyState>,
    thumbprint: String,
    pin: Option<String>,
    save_pin: Option<bool>,
) -> Result<TokenInfo, SmksError> {
    let typed = pin.filter(|p| !p.is_empty()).map(Zeroizing::new);
    if let Some(p) = &typed {
        smks_crypto::pin_vault::check_pin(p)?;
    }
    let reuse = lock(&state.session)
        .as_ref()
        .filter(|s| s.thumbprint.eq_ignore_ascii_case(&thumbprint))
        .map(token_info);
    if let Some(mut info) = reuse {
        if save_pin == Some(true) {
            info.pin_note =
                Some("Token đang mở sẵn nên chưa lưu PIN; lưu ở lần mở token sau.".into());
        }
        return Ok(info);
    }
    let want_save = save_pin == Some(true) && typed.is_some();
    let (session, info) = blocking(move || {
        let source = match typed {
            Some(p) => PinSource::Typed(p),
            None => super::pin::saved(&thumbprint).map_or(PinSource::None, PinSource::Saved),
        };
        let (session, handoff) = open_with(&thumbprint, &source)?;
        let mut info = token_info(&session);
        info.pin_source = match source {
            PinSource::None => None,
            PinSource::Typed(_) => Some("typed"),
            PinSource::Saved(_) => Some("saved"),
        };
        info.pin_handoff = handoff.map(|h| match h {
            PinHandoff::Accepted => "accepted",
            PinHandoff::NotSupported => "notSupported",
        });
        if want_save && let PinSource::Typed(p) = &source {
            if handoff == Some(PinHandoff::Accepted) {
                match super::pin::store(&thumbprint, p) {
                    Ok(()) => info.pin_saved = true,
                    Err(e) => info.pin_note = Some(e.message),
                }
            } else {
                info.pin_note = Some(
                    "Middleware của token không nhận mã PIN do ứng dụng đưa vào nên không lưu PIN (lưu cũng không tự nhập được)."
                        .into(),
                );
            }
        }
        Ok((session, info))
    })
    .await
    // Không mở được token (PIN sai, token bị khoá, hủy hộp PIN…): ghi nhận lần ký không thành.
    .inspect_err(|e| super::notify::token_failed(notify.notifier(), e))?;
    tracing::info!(
        cn = %info.common_name, provider = %info.provider,
        pin = info.pin_source.unwrap_or("-"), handoff = info.pin_handoff.unwrap_or("-"),
        "đã mở token, ký thử đạt"
    );
    *lock(&state.session) = Some(session);
    Ok(info)
}

/// Quên phiên token (người dùng đổi chứng thư, hoặc token bị rút).
#[tauri::command]
pub fn token_forget(state: State<'_, SignState>) {
    *lock(&state.session) = None;
}

// ----------------------------------------------------------------------------------------------
// File
// ----------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn sign_list_files(
    state: State<'_, SignState>,
    sources: Vec<String>,
) -> Result<Vec<FileItem>, SmksError> {
    let files = blocking(move || {
        let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
        smks_sign::job::list_pdf_files(&sources)
    })
    .await?;
    state.allow_preview(files.iter().map(|f| PathBuf::from(&f.path)));
    Ok(files)
}

/// Cho giao diện đọc file PDF qua asset protocol (chỉ file đã liệt kê hoặc file kết quả của lô ký).
#[tauri::command]
pub fn sign_preview(
    app: AppHandle,
    state: State<'_, SignState>,
    path: String,
) -> Result<String, SmksError> {
    let p = PathBuf::from(&path);
    if !lock(&state.previewable).contains(&p) || !p.is_file() {
        return Err(SmksError::new(
            ErrorCode::NotFound,
            "Không mở được file này để xem trước.",
        ));
    }
    app.asset_protocol_scope().allow_file(&p).map_err(|e| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không cấp quyền đọc file: {e}"),
        )
    })?;
    Ok(path)
}

/// Cho giao diện đọc ảnh chữ ký / con dấu người dùng vừa chọn (vẽ khung chữ ký trên bản xem trước).
/// Chỉ nhận file .png / .jpg / .jpeg có thật; không mở rộng quyền đọc sang file khác.
#[tauri::command]
pub fn sign_image_preview(app: AppHandle, path: String) -> Result<String, SmksError> {
    let p = PathBuf::from(&path);
    let is_image = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["png", "jpg", "jpeg"].contains(&e.to_ascii_lowercase().as_str()));
    if !is_image || !p.is_file() {
        return Err(SmksError::new(
            ErrorCode::StampImageInvalid,
            "Ảnh chữ ký / con dấu phải là file .png hoặc .jpg có thật.",
        ));
    }
    app.asset_protocol_scope().allow_file(&p).map_err(|e| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không cấp quyền đọc ảnh: {e}"),
        )
    })?;
    Ok(path)
}

/// Chỗ các khối chữ ký sẽ đặt trong một file đã liệt kê (tỉ lệ trang, gốc trên trái), để giao diện vẽ khung
/// xem trước đúng như khi ký: vị trí kéo tay, hoặc mẫu theo khổ giấy (trang đầu / cuối / mọi trang).
#[tauri::command]
pub async fn sign_positions(
    state: State<'_, SignState>,
    path: String,
    appearance: Appearance,
    signer: String,
) -> Result<Vec<Placement>, SmksError> {
    let p = PathBuf::from(&path);
    if !lock(&state.previewable).contains(&p) {
        return Err(SmksError::new(
            ErrorCode::NotFound,
            "Không mở được file này để xem trước.",
        ));
    }
    blocking(move || {
        let bytes = std::fs::read(&p).map_err(|e| {
            SmksError::new(
                ErrorCode::NotFound,
                format!("Không đọc được {}: {e}", p.display()),
            )
        })?;
        smks_sign::job::preview_positions(&bytes, &appearance, &appearance.placements, &signer)
    })
    .await
}

/// Nơi lưu dự kiến của từng file và file nào đã có bản trùng tên (hộp "File đã ký / trùng tên", KS-16).
#[tauri::command]
pub async fn sign_plan_outputs(
    files: Vec<String>,
    output: OutputOptions,
) -> Result<Vec<PlannedOutput>, SmksError> {
    blocking(move || {
        let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
        Ok(smks_sign::job::plan_outputs(&files, &output))
    })
    .await
}

/// Bố cục chữ ký của một file trên màn Vị trí chữ ký; `error` khi không đọc được file đó.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLayoutItem {
    pub path: String,
    pub layout: Option<smks_sign::job::FileLayout>,
    pub error: Option<String>,
}

/// Bố cục chữ ký của nhiều file một lượt (ảnh thu nhỏ của màn Vị trí chữ ký): khung riêng của file trong
/// `placements`, không có thì `appearance.placements`, rỗng nữa thì theo mẫu khổ giấy. Kiểu hiển thị riêng
/// của file trong `modes` (cỡ khung "Chỉ thông tin" theo cỡ chữ). Chỉ đọc file đã liệt kê.
#[tauri::command]
pub async fn sign_layouts(
    state: State<'_, SignState>,
    paths: Vec<String>,
    appearance: Appearance,
    placements: HashMap<String, Vec<Placement>>,
    modes: HashMap<String, SignatureMode>,
    signer: String,
) -> Result<Vec<FileLayoutItem>, SmksError> {
    let allowed: Vec<bool> = {
        let previewable = lock(&state.previewable);
        paths
            .iter()
            .map(|p| previewable.contains(&PathBuf::from(p)))
            .collect()
    };
    blocking(move || {
        Ok(paths
            .into_iter()
            .zip(allowed)
            .map(|(path, ok)| {
                let result = if ok {
                    std::fs::read(&path)
                        .map_err(|e| format!("Không đọc được file: {e}"))
                        .and_then(|bytes| {
                            let manual = placements
                                .get(&path)
                                .filter(|l| !l.is_empty())
                                .unwrap_or(&appearance.placements);
                            let a =
                                smks_sign::job::with_mode(&appearance, modes.get(&path).copied());
                            smks_sign::job::preview_layout(&bytes, &a, manual, &signer)
                                .map_err(|e| e.message)
                        })
                } else {
                    Err("Không mở được file này để xem trước.".to_owned())
                };
                match result {
                    Ok(layout) => FileLayoutItem {
                        path,
                        layout: Some(layout),
                        error: None,
                    },
                    Err(error) => FileLayoutItem {
                        path,
                        layout: None,
                        error: Some(error),
                    },
                }
            })
            .collect())
    })
    .await
}

// ----------------------------------------------------------------------------------------------
// Lô ký
// ----------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn sign_run(
    app: AppHandle,
    state: State<'_, SignState>,
    notify: State<'_, NotifyState>,
    thumbprint: String,
    request: JobRequest,
    on_event: Channel<JobEvent>,
) -> Result<String, SmksError> {
    let names: Vec<String> = request
        .files
        .iter()
        .map(|f| {
            f.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect();
    // Lô không bắt đầu được: ghi nhận lần ký không thành. Lô đã chạy thì `SignReport` báo khi lô kết thúc.
    start_sign(app, &state, thumbprint, request, on_event)
        .await
        .inspect_err(|e| super::notify::sign_start_failed(notify.notifier(), &names, e))
}

async fn start_sign(
    app: AppHandle,
    state: &SignState,
    thumbprint: String,
    mut request: JobRequest,
    on_event: Channel<JobEvent>,
) -> Result<String, SmksError> {
    let check = request.clone();
    blocking(move || smks_sign::job::validate(&check)).await?;
    // Dùng phiên token đã ký thử; chưa có (hoặc khác chứng thư) thì mở lại.
    let has_session = lock(&state.session)
        .as_ref()
        .is_some_and(|s| s.thumbprint.eq_ignore_ascii_case(&thumbprint));
    if !has_session {
        let tp = thumbprint.clone();
        // Mở lại phiên (token hết phiên / ứng dụng vừa mở): tự nhập PIN đã lưu nếu có (KS-23).
        let session = blocking(move || {
            let source = super::pin::saved(&tp).map_or(PinSource::None, PinSource::Saved);
            open_with(&tp, &source).map(|(s, _)| s)
        })
        .await?;
        *lock(&state.session) = Some(session);
    }
    let (key, chain, dev) = {
        let guard = lock(&state.session);
        let s = guard
            .as_ref()
            .ok_or_else(|| SmksError::new(ErrorCode::SignTokenInvalid, "Chưa mở được token."))?;
        (s.key.clone(), s.chain.clone(), s.dev)
    };
    request.require_trusted = !dev;
    let job_id = format!("ky-{}", state.next_id.fetch_add(1, Ordering::Relaxed));
    let cancel = Arc::new(AtomicBool::new(false));
    lock(&state.jobs).insert(job_id.clone(), cancel.clone());
    let tsa = state.tsa.clone();
    let id = job_id.clone();
    let mut report = SignReport::new(
        key.certificate().common_name(),
        smks_crypto::store::is_hardware_provider(&key.provider()),
        dev,
    );
    std::thread::Builder::new()
        .name(format!("job-{job_id}"))
        .spawn(move || {
            tracing::info!(job = %id, total = request.files.len(), "bắt đầu ký");
            let monitor = move || dev || smks_crypto::store::is_present(&thumbprint);
            let mut outputs = Vec::new();
            smks_sign::job::run(&id, &request, key.as_ref(), &chain, Some(&tsa), &cancel, Some(&monitor), &mut |event| {
                report.observe(&event);
                match &event {
                    JobEvent::File { paused: true, .. } => {}
                    JobEvent::File { name, success: false, reason, .. } => {
                        tracing::warn!(job = %id, file = %name, "{}", reason.as_deref().unwrap_or(""));
                    }
                    JobEvent::File { output_path: Some(p), .. } => outputs.push(PathBuf::from(p)),
                    JobEvent::Summary { success, failed, paused, stopped, token_removed, elapsed_ms, .. } => {
                        tracing::info!(job = %id, success, failed, paused, stopped, token_removed, elapsed_ms, "kết thúc ký");
                    }
                    _ => {}
                }
                let _ = on_event.send(event);
            });
            // Lô đã kết thúc (ký và kiểm lại xong từng file): giờ mới ghi nhận sự kiện. Chỉ xếp hàng, không chặn.
            report.finish(app.state::<NotifyState>().notifier());
            let state = app.state::<SignState>();
            state.allow_preview(outputs);
            lock(&state.jobs).remove(&id);
            if !dev && !smks_crypto::store::is_present(&key.certificate().thumbprint()) {
                *lock(&state.session) = None;
            }
        })
        .map_err(|e| SmksError::new(ErrorCode::InternalError, format!("Không tạo được luồng ký: {e}")))?;
    Ok(job_id)
}

/// Xác minh chữ ký của một file (màn Kiểm tra, và để xem lại file vừa ký). File kiểm được thì giao diện được
/// mở xem trước (`sign_preview`) để vẽ vị trí chữ ký.
#[tauri::command]
pub async fn verify_file(
    state: State<'_, SignState>,
    path: String,
    revocation: Option<bool>,
) -> Result<smks_sign::verify::PdfReport, SmksError> {
    let p = PathBuf::from(&path);
    let report = blocking(move || {
        let bytes = std::fs::read(Path::new(&path)).map_err(|e| {
            SmksError::new(
                ErrorCode::SignSourceNotFound,
                format!("Không đọc được file: {e}"),
            )
        })?;
        smks_sign::verify::verify_pdf(
            &bytes,
            smks_sign::verify::VerifyOptions {
                check_revocation: revocation.unwrap_or(false),
            },
        )
    })
    .await?;
    let is_pdf = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    if is_pdf {
        state.allow_preview([p]);
    }
    Ok(report)
}

// ----------------------------------------------------------------------------------------------
// Khoá thử (chỉ khi build với feature dev-keys; không bao giờ có trong bản phát hành)
// ----------------------------------------------------------------------------------------------

#[cfg(feature = "dev-keys")]
mod dev {
    //! Đọc khoá thử từ thư mục `SMKS_DEV_PKI` (`signer.cert.der`, `signer.key.pk8.der`, `ca.cert.der`).

    use std::path::PathBuf;
    use std::sync::Arc;

    use smks_core::SmksError;
    use smks_crypto::cert::Cert;
    use smks_crypto::key::software::SoftwareKey;
    use smks_crypto::store::{CertEntry, CertSource};

    use super::Session;

    fn dir() -> Option<PathBuf> {
        std::env::var_os("SMKS_DEV_PKI").map(PathBuf::from)
    }

    fn cert() -> Option<Cert> {
        Cert::from_der(&std::fs::read(dir()?.join("signer.cert.der")).ok()?).ok()
    }

    pub fn entry() -> Option<CertEntry> {
        let cert = cert()?;
        let (valid_from, valid_to) = cert.validity();
        Some(CertEntry {
            thumbprint: cert.thumbprint(),
            common_name: cert.common_name(),
            subject: cert.subject(),
            issuer: cert.issuer(),
            issuer_common_name: cert.issuer_common_name(),
            serial_number: cert.serial_hex(),
            valid_from,
            valid_to,
            source: CertSource::Local,
            key_provider: Some("Khoá phần mềm (chỉ để thử)".into()),
            has_private_key: true,
            is_expired: false,
            allows_signing: true,
            is_trusted: false,
            can_sign: true,
            reason: Some("Khoá thử (bản build dev-keys) — chữ ký sẽ bị đánh giá là không thuộc CA Ban Cơ yếu.".into()),
        })
    }

    pub fn session(thumbprint: &str) -> Result<Option<Session>, SmksError> {
        let Some(cert) = cert() else { return Ok(None) };
        if !cert.thumbprint().eq_ignore_ascii_case(thumbprint) {
            return Ok(None);
        }
        let Some(d) = dir() else { return Ok(None) };
        let key_bytes = std::fs::read(d.join("signer.key.pk8.der"))
            .map_err(|e| SmksError::new(smks_core::ErrorCode::SignTokenInvalid, e.to_string()))?;
        let key = SoftwareKey::from_pkcs8_der(cert, &key_bytes)?;
        let chain = std::fs::read(d.join("ca.cert.der"))
            .ok()
            .and_then(|b| Cert::from_der(&b).ok())
            .into_iter()
            .collect();
        Ok(Some(Session {
            thumbprint: thumbprint.to_owned(),
            key: Arc::new(key),
            chain,
            dev: true,
        }))
    }
}
