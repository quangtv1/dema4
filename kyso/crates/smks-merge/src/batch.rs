//! Chạy một lô tạo văn bản (port `RunMergeAsync`/`ProcessRow` của `DocumentMergeService.cs`).
//!
//! Không phụ thuộc Tauri: tiến độ đổ về một hàm `emit`, dừng bằng cờ `cancel`. Ứng dụng và `smks-cli` cùng
//! dùng module này.
//!
//! Mỗi file: điền mẫu → (PDF) Word xuất trong thư mục tạm → chuyển vào thư mục kết quả qua file `.tmp`.
//! Một file lỗi không làm dừng lô. Word treo thì phiên bị bỏ và mở phiên mới cho file sau.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::Serialize;
use smks_core::consts::app::APP_FOLDER_NAME;
use smks_core::consts::merge::TEMP_FOLDER_NAME;

use crate::plan::{OutputFormat, Prepared};

/// Sự kiện tiến độ (khớp `MergeEvent` ở giao diện).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum BatchEvent {
    Manifest {
        job_id: String,
        total: usize,
        output_folder: String,
        files: Vec<String>,
    },
    File {
        job_id: String,
        index: usize,
        row_number: u32,
        file_name: String,
        status: &'static str,
        output_path: Option<String>,
        reason: Option<String>,
        warnings: Vec<String>,
        current: usize,
        total: usize,
    },
    Summary {
        job_id: String,
        total: usize,
        success: usize,
        failed: usize,
        elapsed_ms: u64,
        output_folder: String,
        stopped: bool,
    },
}

/// Thư mục tạm của Tạo văn bản: `%TEMP%\SaoMai-KySo\tao-van-ban`.
pub fn temp_root() -> PathBuf {
    std::env::temp_dir()
        .join(APP_FOLDER_NAME)
        .join(TEMP_FOLDER_NAME)
}

/// Ghi `bytes` ra `dest` qua file `.tmp` rồi đổi tên, để lỗi giữa chừng không để lại file hỏng.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dest).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Chuyển file sang thư mục kết quả (có thể khác ổ đĩa).
pub fn move_into(src: &Path, dest: &Path) -> std::io::Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    let bytes = std::fs::read(src)?;
    write_atomic(dest, &bytes)?;
    let _ = std::fs::remove_file(src);
    Ok(())
}

/// Bộ chuyển DOCX → PDF. Ứng dụng dùng Word; kiểm thử có thể thay bằng bộ khác.
pub trait PdfConverter {
    /// Xuất `docx` ra `pdf`, trả về số trang.
    fn convert(&mut self, docx: &Path, pdf: &Path) -> smks_core::Result<u32>;

    /// Giải phóng tài nguyên (đóng Word). Được gọi trước khi báo hoàn tất lô, để ứng dụng có tắt ngay sau
    /// đó cũng không để lại tiến trình Word.
    fn close(&mut self) {}
}

#[cfg(windows)]
impl PdfConverter for Option<crate::word::WordConverter> {
    fn convert(&mut self, docx: &Path, pdf: &Path) -> smks_core::Result<u32> {
        if self
            .as_ref()
            .is_none_or(crate::word::WordConverter::is_broken)
        {
            *self = None;
            *self = Some(crate::word::WordConverter::start()?);
        }
        match self.as_mut() {
            Some(word) => word.convert(docx, pdf),
            None => Err(smks_core::SmksError::from(
                smks_core::ErrorCode::MergeConverterUnavailable,
            )),
        }
    }

    fn close(&mut self) {
        *self = None;
    }
}

/// Chạy lô. `converter` chỉ được dùng khi định dạng là PDF.
pub fn run(
    job_id: &str,
    prepared: &Prepared,
    cancel: &AtomicBool,
    converter: &mut dyn PdfConverter,
    emit: &mut dyn FnMut(BatchEvent),
) {
    let started = Instant::now();
    let total = prepared.records.len();
    let output_folder = prepared.output_folder.display().to_string();
    emit(BatchEvent::Manifest {
        job_id: job_id.to_owned(),
        total,
        output_folder: output_folder.clone(),
        files: prepared
            .records
            .iter()
            .map(|r| r.file_name.clone())
            .collect(),
    });

    let work = temp_root().join(job_id);
    let setup = std::fs::create_dir_all(&prepared.output_folder)
        .map_err(|e| format!("Không tạo được thư mục kết quả {output_folder}: {e}"))
        .and_then(|()| {
            std::fs::create_dir_all(&work).map_err(|e| format!("Không tạo được thư mục tạm: {e}"))
        });

    let (mut success, mut failed, mut stopped) = (0usize, 0usize, false);
    for (index, record) in prepared.records.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            stopped = true;
            break;
        }
        let dest = prepared.output_folder.join(&record.file_name);
        let outcome: Result<(), String> = setup.clone().and_then(|()| {
            let filled =
                crate::docx::fill(&prepared.template, &record.values).map_err(|e| e.message)?;
            match prepared.format {
                OutputFormat::Docx => {
                    write_atomic(&dest, &filled).map_err(|e| format!("Không ghi được file: {e}"))
                }
                OutputFormat::Pdf => {
                    let docx = work.join(format!("{index}.docx"));
                    let pdf = work.join(format!("{index}.pdf"));
                    std::fs::write(&docx, filled)
                        .map_err(|e| format!("Không ghi được file tạm: {e}"))?;
                    let converted = converter
                        .convert(&docx, &pdf)
                        .map(|_| ())
                        .map_err(|e| e.message);
                    let _ = std::fs::remove_file(&docx);
                    converted?;
                    move_into(&pdf, &dest).map_err(|e| format!("Không ghi được file kết quả: {e}"))
                }
            }
        });
        let (status, output_path, reason) = match outcome {
            Ok(()) => {
                success += 1;
                ("ok", Some(dest.display().to_string()), None)
            }
            Err(reason) => {
                failed += 1;
                ("error", None, Some(reason))
            }
        };
        emit(BatchEvent::File {
            job_id: job_id.to_owned(),
            index,
            row_number: record.row_number,
            file_name: record.file_name.clone(),
            status,
            output_path,
            reason,
            warnings: record.warnings.clone(),
            current: index + 1,
            total,
        });
    }
    converter.close();
    let _ = std::fs::remove_dir_all(&work);

    emit(BatchEvent::Summary {
        job_id: job_id.to_owned(),
        total,
        success,
        failed,
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        output_folder,
        stopped,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numfmt::NumberLocale;
    use crate::plan::{DataSource, MergeRequest, prepare};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    fn fixture(relative: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(relative)
            .display()
            .to_string()
    }

    /// Bộ chuyển giả: chép DOCX thành "PDF", lỗi ở lượt thứ `fail_at`.
    struct FakeConverter {
        calls: usize,
        fail_at: Option<usize>,
    }

    impl PdfConverter for FakeConverter {
        fn convert(&mut self, docx: &Path, pdf: &Path) -> smks_core::Result<u32> {
            self.calls += 1;
            if Some(self.calls) == self.fail_at {
                return Err(smks_core::SmksError::new(
                    smks_core::ErrorCode::MergeFailed,
                    "giả lập lỗi",
                ));
            }
            std::fs::copy(docx, pdf).map_err(|e| {
                smks_core::SmksError::new(smks_core::ErrorCode::MergeFailed, e.to_string())
            })?;
            Ok(1)
        }
    }

    fn prepared(format: OutputFormat, out: &Path) -> Prepared {
        let request = MergeRequest {
            template_path: fixture("fixtures/merge/2_quyet_dinh_khen_thuong.docx"),
            data: DataSource::Table {
                path: fixture("fixtures/merge/2_quyet_dinh_khen_thuong.xlsx"),
                sheet: None,
                header_row: 1,
            },
            mappings: HashMap::new(),
            file_name_pattern: "vb".into(),
            format,
            output_folder: Some(out.display().to_string()),
        };
        prepare(&request, NumberLocale::EN_US, "test").unwrap()
    }

    fn out_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("smks-batch-tests").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn docx_batch_writes_every_file_and_reports_progress() {
        let out = out_dir("docx");
        let prep = prepared(OutputFormat::Docx, &out);
        let mut events = Vec::new();
        run(
            "j1",
            &prep,
            &AtomicBool::new(false),
            &mut FakeConverter {
                calls: 0,
                fail_at: None,
            },
            &mut |e| events.push(e),
        );
        assert!(matches!(&events[0], BatchEvent::Manifest { total: 5, .. }));
        let Some(BatchEvent::Summary {
            success,
            failed,
            stopped,
            ..
        }) = events.last()
        else {
            panic!("{events:?}")
        };
        assert_eq!((*success, *failed, *stopped), (5, 0, false));
        let files: Vec<_> = std::fs::read_dir(&prep.output_folder)
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(files.len(), 5, "không còn file .tmp");
        assert!(
            files
                .iter()
                .all(|f| f.path().extension().is_some_and(|e| e == "docx"))
        );
    }

    #[test]
    fn one_failure_does_not_stop_the_batch() {
        let out = out_dir("pdf-fail");
        let prep = prepared(OutputFormat::Pdf, &out);
        let mut events = Vec::new();
        let mut converter = FakeConverter {
            calls: 0,
            fail_at: Some(2),
        };
        run(
            "j2",
            &prep,
            &AtomicBool::new(false),
            &mut converter,
            &mut |e| events.push(e),
        );
        let statuses: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let BatchEvent::File { status, .. } = e {
                    Some(*status)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(statuses, ["ok", "error", "ok", "ok", "ok"]);
        assert!(matches!(
            events.last(),
            Some(BatchEvent::Summary {
                success: 4,
                failed: 1,
                ..
            })
        ));
    }

    #[test]
    fn cancel_stops_before_next_file() {
        let out = out_dir("cancel");
        let prep = prepared(OutputFormat::Docx, &out);
        let cancel = AtomicBool::new(false);
        let count = AtomicUsize::new(0);
        let mut events = Vec::new();
        run(
            "j3",
            &prep,
            &cancel,
            &mut FakeConverter {
                calls: 0,
                fail_at: None,
            },
            &mut |e| {
                if matches!(e, BatchEvent::File { .. }) && count.fetch_add(1, Ordering::SeqCst) == 1
                {
                    cancel.store(true, Ordering::SeqCst);
                }
                events.push(e);
            },
        );
        assert!(
            matches!(
                events.last(),
                Some(BatchEvent::Summary {
                    success: 2,
                    stopped: true,
                    ..
                })
            ),
            "{events:?}"
        );
    }

    /// Ghi lại thời điểm `close` được gọi so với các sự kiện.
    struct ClosingConverter(std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>);

    impl PdfConverter for ClosingConverter {
        fn convert(&mut self, docx: &Path, pdf: &Path) -> smks_core::Result<u32> {
            std::fs::copy(docx, pdf).map(|_| 1).map_err(|e| {
                smks_core::SmksError::new(smks_core::ErrorCode::MergeFailed, e.to_string())
            })
        }
        fn close(&mut self) {
            self.0.borrow_mut().push("close");
        }
    }

    #[test]
    fn converter_is_closed_before_summary() {
        let out = out_dir("close");
        let prep = prepared(OutputFormat::Pdf, &out);
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut converter = ClosingConverter(log.clone());
        run(
            "j4",
            &prep,
            &AtomicBool::new(false),
            &mut converter,
            &mut |e| {
                if matches!(e, BatchEvent::Summary { .. }) {
                    log.borrow_mut().push("summary");
                }
            },
        );
        assert_eq!(*log.borrow(), ["close", "summary"]);
    }

    #[test]
    fn event_json_matches_ui_contract() {
        let json = serde_json::to_value(BatchEvent::Summary {
            job_id: "j".into(),
            total: 1,
            success: 1,
            failed: 0,
            elapsed_ms: 5,
            output_folder: "x".into(),
            stopped: false,
        })
        .unwrap();
        assert_eq!(json["type"], "summary");
        assert_eq!(json["jobId"], "j");
        assert_eq!(json["elapsedMs"], 5);
    }
}
