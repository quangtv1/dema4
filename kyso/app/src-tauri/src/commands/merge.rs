//! Command của màn Tạo văn bản. Nghiệp vụ nằm ở `smks-merge`; ở đây chỉ nối với IPC, job và quyền đọc file.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use smks_core::consts::merge::{PREVIEW_FOLDER_NAME, PREVIEW_RETENTION_MINUTES};
use smks_core::{ErrorCode, SmksError};
use smks_merge::batch::{BatchEvent, temp_root};
use smks_merge::numfmt::NumberLocale;
use smks_merge::plan::{MergePlan, MergeRequest, OutputFormat, auto_map, prepare, vietnam_stamp};
use smks_merge::preview::{PreviewOptions, PreviewService, Rendered, clean_dir, neighbours};
use smks_merge::table::{Table, TableColumn, read_table};
use smks_merge::word::WordConverter;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::blocking;
use super::notify::{MergeReport, NotifyState};

/// Trạng thái dùng chung của Tạo văn bản.
#[derive(Debug)]
pub struct MergeState {
    /// Dấu thập phân, nhóm nghìn theo cài đặt Windows của người dùng (đọc một lần lúc khởi động).
    pub locale: NumberLocale,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    next_id: AtomicU64,
    /// Xem trước: phiên Word giữ sẵn, bộ đệm, tạo trước (TD-31). `None` nếu không mở được thư mục tạm.
    preview: Option<Arc<PreviewService>>,
}

impl MergeState {
    pub fn new(locale: NumberLocale) -> Self {
        let dir = temp_root().join(PREVIEW_FOLDER_NAME);
        // Dọn bản xem trước do lần chạy trước để lại (ứng dụng bị tắt ngang).
        clean_dir(&dir, Duration::from_secs(PREVIEW_RETENTION_MINUTES * 60));
        let converter: Option<WordConverter> = None;
        let preview = PreviewService::start(dir, Box::new(converter), PreviewOptions::default())
            .inspect_err(|e| tracing::error!("không mở được dịch vụ xem trước: {}", e.message))
            .ok()
            .map(Arc::new);
        Self {
            locale,
            jobs: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            preview,
        }
    }

    fn preview(&self) -> Result<Arc<PreviewService>, SmksError> {
        self.preview.clone().ok_or_else(|| {
            SmksError::new(ErrorCode::MergeFailed, "Không mở được chức năng xem trước.")
        })
    }

    /// Đóng phiên Word của xem trước (gọi khi ứng dụng thoát).
    pub fn shutdown(&self) {
        if let Some(preview) = &self.preview {
            preview.shutdown();
        }
    }

    fn jobs(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn register(&self) -> (String, Arc<AtomicBool>) {
        let id = format!("merge-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let cancel = Arc::new(AtomicBool::new(false));
        self.jobs().insert(id.clone(), cancel.clone());
        (id, cancel)
    }

    fn finish(&self, id: &str) {
        self.jobs().remove(id);
    }

    fn cancel(&self, id: &str) -> bool {
        self.jobs()
            .get(id)
            .map(|flag| flag.store(true, Ordering::SeqCst))
            .is_some()
    }
}

fn now_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    vietnam_stamp(i64::try_from(seconds).unwrap_or(0))
}

// ----------------------------------------------------------------------------------------------
// Mẫu và dữ liệu
// ----------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub path: String,
    pub file_name: String,
    pub kind: &'static str,
    pub fields: Vec<String>,
}

#[tauri::command]
pub async fn merge_inspect_template(
    state: State<'_, MergeState>,
    path: String,
) -> Result<TemplateInfo, SmksError> {
    let (info, bytes) = blocking(move || {
        let file = Path::new(&path);
        if !file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
        {
            return Err(SmksError::new(
                ErrorCode::MergeTemplateInvalid,
                "Mẫu văn bản phải là file Word .docx.",
            ));
        }
        let bytes = std::fs::read(file).map_err(|e| {
            SmksError::new(
                ErrorCode::MergeTemplateInvalid,
                format!("Không đọc được file mẫu: {e}"),
            )
        })?;
        let fields = smks_merge::docx::read_fields(&bytes)?;
        tracing::info!(path = %file.display(), fields = fields.len(), "đọc mẫu văn bản");
        let info = TemplateInfo {
            path: file.display().to_string(),
            file_name: file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind: "docx",
            fields,
        };
        Ok((info, bytes))
    })
    .await?;
    // Khởi động Word và tạo sẵn PDF của mẫu trong lúc người dùng chọn dữ liệu.
    if let Some(preview) = &state.preview {
        // Giao diện mặc định bật tô màu biến.
        preview.warm_template(Arc::new(bytes), true);
    }
    Ok(info)
}

#[tauri::command]
pub async fn merge_read_table(
    state: State<'_, MergeState>,
    path: String,
    sheet: Option<String>,
    header_row: Option<u32>,
) -> Result<Table, SmksError> {
    let locale = state.locale;
    blocking(move || {
        let table = read_table(Path::new(&path), sheet.as_deref(), header_row, locale)?;
        tracing::info!(path = %path, rows = table.rows.len(), columns = table.columns.len(), "đọc dữ liệu");
        Ok(table)
    })
    .await
}

#[tauri::command]
pub fn merge_auto_map(
    fields: Vec<String>,
    columns: Vec<TableColumn>,
) -> HashMap<String, Option<u32>> {
    auto_map(&fields, &columns)
}

#[tauri::command]
pub async fn merge_plan(
    state: State<'_, MergeState>,
    request: MergeRequest,
) -> Result<MergePlan, SmksError> {
    let locale = state.locale;
    blocking(move || {
        let prepared = prepare(&request, locale, &now_stamp())?;
        let can_export = request.format == OutputFormat::Docx || smks_merge::word::is_available();
        Ok(prepared.plan(can_export))
    })
    .await
}

// ----------------------------------------------------------------------------------------------
// Xem trước
// ----------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub pdf_path: String,
    pub page_count: u32,
    pub warnings: Vec<String>,
}

/// Cho giao diện đọc đúng file PDF này qua asset protocol.
fn allow_preview(
    app: &AppHandle,
    rendered: Rendered,
    warnings: Vec<String>,
) -> Result<PreviewResult, SmksError> {
    app.asset_protocol_scope()
        .allow_file(&rendered.pdf)
        .map_err(|e| {
            SmksError::new(
                ErrorCode::InternalError,
                format!("Không cấp quyền đọc bản xem trước: {e}"),
            )
        })?;
    Ok(PreviewResult {
        pdf_path: rendered.pdf.display().to_string(),
        page_count: rendered.pages,
        warnings,
    })
}

#[tauri::command]
pub async fn merge_preview(
    app: AppHandle,
    state: State<'_, MergeState>,
    request: MergeRequest,
    row_number: Option<u32>,
    highlight: Option<bool>,
) -> Result<PreviewResult, SmksError> {
    let mark = highlight.unwrap_or(false);
    let locale = state.locale;
    let preview = state.preview()?;
    blocking(move || {
        let started = Instant::now();
        let prepared = prepare(&request, locale, &now_stamp())?;
        let index = match row_number {
            Some(n) => prepared
                .records
                .iter()
                .position(|r| r.row_number == n)
                .ok_or_else(|| {
                    SmksError::new(
                        ErrorCode::MergeDataInvalid,
                        format!("File dữ liệu không có dòng số {n}."),
                    )
                })?,
            None if prepared.records.is_empty() => {
                return Err(SmksError::new(
                    ErrorCode::MergeDataInvalid,
                    "File dữ liệu không có dòng dữ liệu nào.",
                ));
            }
            None => 0,
        };
        let template = Arc::new(prepared.template);
        let records = prepared.records;
        // Dòng lân cận được tạo trước khi rảnh; dòng đang xem luôn được làm trước.
        let options = PreviewOptions::default();
        let around = neighbours(records.len(), index, options.ahead, options.behind)
            .into_iter()
            .filter_map(|i| records.get(i).map(|r| r.values.clone()))
            .collect();
        preview.prefetch(template.clone(), around, mark);
        let Some(record) = records.get(index) else {
            return Err(SmksError::from(ErrorCode::MergeDataInvalid));
        };
        let rendered = preview.render_record(template, &record.values, mark)?;
        tracing::debug!(
            row = record.row_number,
            ms = started.elapsed().as_millis() as u64,
            "xem trước"
        );
        allow_preview(&app, rendered, record.warnings.clone())
    })
    .await
}

#[tauri::command]
pub async fn merge_preview_template(
    app: AppHandle,
    state: State<'_, MergeState>,
    template_path: String,
    highlight: Option<bool>,
) -> Result<PreviewResult, SmksError> {
    let mark = highlight.unwrap_or(false);
    let preview = state.preview()?;
    blocking(move || {
        let template = std::fs::read(&template_path).map_err(|_| {
            SmksError::new(ErrorCode::MergeTemplateInvalid, "Không tìm thấy file mẫu.")
        })?;
        let rendered = preview.render_template(Arc::new(template), mark)?;
        allow_preview(&app, rendered, Vec::new())
    })
    .await
}

// ----------------------------------------------------------------------------------------------
// Chạy lô
// ----------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn merge_run(
    app: AppHandle,
    state: State<'_, MergeState>,
    notify: State<'_, NotifyState>,
    request: MergeRequest,
    on_event: Channel<BatchEvent>,
) -> Result<String, SmksError> {
    let locale = state.locale;
    let document_type =
        super::notify::document_type(&request.template_path, request.format.extension());
    // Kiểm tra trước khi mở job: lỗi cấu hình trả thẳng về giao diện (và được ghi nhận là tạo văn bản không thành).
    let prepared = blocking(move || prepare(&request, locale, &now_stamp()))
        .await
        .inspect_err(|e| {
            super::notify::merge_start_failed(notify.notifier(), document_type.clone(), e);
        })?;
    let mut report = MergeReport::new(document_type, prepared.records.len());
    let (job_id, cancel) = state.register();
    let id = job_id.clone();
    // Lô dùng phiên Word riêng: dừng tạo trước và đóng phiên Word của xem trước trong lúc chạy.
    let preview = state.preview.clone();
    if let Some(preview) = &preview {
        preview.set_paused(true);
    }
    std::thread::Builder::new()
        .name(format!("job-{job_id}"))
        .spawn(move || {
            tracing::info!(job = %id, total = prepared.records.len(), folder = %prepared.output_folder.display(), "bắt đầu tạo văn bản");
            let mut word: Option<WordConverter> = None;
            smks_merge::batch::run(&id, &prepared, &cancel, &mut word, &mut |event| {
                report.observe(&event);
                match &event {
                    BatchEvent::File { row_number, reason: Some(reason), .. } => {
                        tracing::warn!(job = %id, row = row_number, "{reason}");
                    }
                    BatchEvent::Summary { success, failed, stopped, elapsed_ms, .. } => {
                        tracing::info!(job = %id, success, failed, stopped, elapsed_ms, "kết thúc tạo văn bản");
                    }
                    _ => {}
                }
                let _ = on_event.send(event);
            });
            drop(word);
            // Lô đã kết thúc (file đã nằm ở thư mục lưu): giờ mới ghi nhận sự kiện. Chỉ xếp hàng, không chặn.
            report.finish(app.state::<NotifyState>().notifier());
            app.state::<MergeState>().finish(&id);
            if let Some(preview) = &preview {
                preview.set_paused(false);
            }
        })
        .map_err(|e| {
            state.finish(&job_id);
            if let Some(preview) = &state.preview {
                preview.set_paused(false);
            }
            SmksError::new(ErrorCode::InternalError, format!("Không tạo được luồng chạy lô: {e}"))
        })?;
    Ok(job_id)
}

#[tauri::command]
pub fn job_cancel(
    state: State<'_, MergeState>,
    sign: State<'_, super::sign::SignState>,
    job_id: String,
) {
    if state.cancel(&job_id) || sign.cancel(&job_id) {
        tracing::info!(job = %job_id, "người dùng dừng lô");
    }
}
