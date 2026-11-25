//! Xuất DOCX ra PDF bằng Microsoft Word qua COM (port `WordPdfConverter.cs`, kiểm chứng ở spike S7).
//!
//! - Một luồng STA riêng giữ một phiên `Word.Application` (`Visible = false`, `DisplayAlerts = 0`); bên gọi
//!   gửi việc qua channel và chờ có giới hạn thời gian.
//! - Chuỗi lệnh như bản cũ: `Documents.Open(path, false, true, false)` → `ExportAsFixedFormat(pdf, 17)` →
//!   `Close(0)`; kết thúc bằng `Quit(0)`. Thêm `ComputeStatistics(2)` để biết số trang.
//! - Khác bản cũ: mỗi file có thời hạn; quá hạn thì buộc đóng đúng tiến trình Word do phiên này mở (theo
//!   PID), phiên bị đánh dấu hỏng và bên gọi mở phiên mới.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use smks_core::{ErrorCode, Result, SmksError};

#[allow(unsafe_code)]
mod ffi;

/// Thời hạn mỗi file.
pub const FILE_TIMEOUT: Duration = Duration::from_secs(120);
const START_TIMEOUT: Duration = Duration::from_secs(60);
static START_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
/// PID của các tiến trình Word do các phiên đang sống mở, để buộc đóng khi ứng dụng thoát.
static LIVE_PIDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

fn live_pids() -> std::sync::MutexGuard<'static, Vec<u32>> {
    LIVE_PIDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Buộc đóng mọi tiến trình Word do ứng dụng mở mà chưa đóng (gọi khi ứng dụng thoát giữa chừng một lô
/// hoặc một bản xem trước). Không đụng tới Word người dùng tự mở.
pub fn kill_all_sessions() {
    let pids: Vec<u32> = live_pids().drain(..).collect();
    for pid in pids {
        ffi::end_process(pid, true, Duration::from_secs(5));
    }
}

/// Máy có đăng ký `Word.Application` hay không.
pub fn is_available() -> bool {
    ffi::word_registered()
}

/// Dấu thập phân và nhóm nghìn theo cài đặt vùng của người dùng Windows (như bản cũ dùng `CurrentCulture`).
pub fn user_number_locale() -> crate::numfmt::NumberLocale {
    ffi::user_number_locale()
}

struct Job {
    docx: PathBuf,
    pdf: PathBuf,
    reply: mpsc::Sender<std::result::Result<u32, String>>,
}

/// Một phiên Word dùng cho nhiều file.
#[derive(Debug)]
pub struct WordConverter {
    jobs: Option<mpsc::Sender<Job>>,
    worker: Option<JoinHandle<()>>,
    own_pids: Vec<u32>,
    broken: bool,
    /// Người dùng đã mở tài liệu của họ vào phiên này: phiên được giao lại, không đóng, không buộc dừng.
    handed_over: Arc<AtomicBool>,
}

fn unavailable(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::MergeConverterUnavailable, message)
}

impl WordConverter {
    /// Mở Word ẩn. Báo lỗi 1045 nếu máy không có Word hoặc Word không khởi động được.
    pub fn start() -> Result<Self> {
        if !is_available() {
            return Err(unavailable(
                "Máy chưa cài Microsoft Word nên không xuất được PDF. Chọn định dạng DOCX hoặc cài Word.",
            ));
        }
        // Nhận diện PID bằng cách so danh sách WINWORD trước và sau khi tạo, nên không cho hai phiên khởi
        // động cùng lúc (ví dụ bản xem trước và một lô chạy song song).
        let _start_guard = START_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel();
        let handed_over = Arc::new(AtomicBool::new(false));
        let session_flag = handed_over.clone();
        let worker = std::thread::Builder::new()
            .name("word-sta".into())
            .spawn(move || ffi::run_session(job_rx, ready_tx, session_flag))
            .map_err(|e| unavailable(format!("Không tạo được luồng cho Microsoft Word: {e}")))?;
        let own_pids = match ready_rx.recv_timeout(START_TIMEOUT) {
            Ok(Ok(pids)) => pids,
            Ok(Err(message)) => {
                return Err(unavailable(format!(
                    "Không khởi động được Microsoft Word: {message}"
                )));
            }
            Err(_) => return Err(unavailable("Microsoft Word không khởi động trong 60 giây.")),
        };
        if own_pids.len() == 1 {
            live_pids().extend(&own_pids);
        } else {
            tracing_like_warn(&format!(
                "không xác định được đúng một tiến trình Word ({} tiến trình mới)",
                own_pids.len()
            ));
        }
        Ok(Self {
            jobs: Some(job_tx),
            worker: Some(worker),
            own_pids,
            broken: false,
            handed_over,
        })
    }

    /// Phiên đã hỏng (Word treo hoặc bị đóng, hoặc đã giao cho người dùng); bên gọi cần mở phiên mới.
    pub fn is_broken(&self) -> bool {
        self.broken || self.is_handed_over()
    }

    fn is_handed_over(&self) -> bool {
        self.handed_over.load(Ordering::SeqCst)
    }

    /// Xuất `docx` ra `pdf`. Trả về số trang.
    pub fn convert(&mut self, docx: &Path, pdf: &Path) -> Result<u32> {
        let fail = |message: String| SmksError::new(ErrorCode::MergeFailed, message);
        if self.is_broken() {
            self.broken = true;
            return Err(fail("Phiên Microsoft Word đã dừng.".into()));
        }
        let docx = std::path::absolute(docx).map_err(|e| fail(e.to_string()))?;
        let pdf = std::path::absolute(pdf).map_err(|e| fail(e.to_string()))?;
        let _ = std::fs::remove_file(&pdf);
        let (reply_tx, reply_rx) = mpsc::channel();
        let sent = self
            .jobs
            .as_ref()
            .map(|jobs| {
                jobs.send(Job {
                    docx,
                    pdf: pdf.clone(),
                    reply: reply_tx,
                })
                .is_ok()
            })
            .unwrap_or(false);
        if !sent {
            self.broken = true;
            return Err(fail("Phiên Microsoft Word đã dừng.".into()));
        }
        let started = Instant::now();
        match reply_rx.recv_timeout(FILE_TIMEOUT) {
            Ok(Ok(pages)) => {
                let size = std::fs::metadata(&pdf).map(|m| m.len()).unwrap_or(0);
                if size == 0 {
                    return Err(fail(
                        "Microsoft Word báo xuất xong nhưng không có file PDF.".into(),
                    ));
                }
                Ok(pages)
            }
            Ok(Err(message)) => {
                // Lỗi RPC nghĩa là tiến trình Word đã mất; lỗi khác (file hỏng…) thì phiên vẫn dùng được.
                if message.contains("0x800706BA")
                    || message.contains("0x800706BE")
                    || message.contains("0x80010108")
                {
                    self.broken = true;
                }
                Err(fail(format!(
                    "Microsoft Word không xuất được PDF: {message}"
                )))
            }
            // Luồng STA đã thoát (ví dụ vừa giao phiên cho người dùng): không buộc dừng gì cả.
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.broken = true;
                Err(fail("Phiên Microsoft Word đã dừng.".into()))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.broken = true;
                self.kill_own();
                Err(fail(format!(
                    "Microsoft Word không phản hồi sau {} giây nên đã bị dừng.",
                    started.elapsed().as_secs()
                )))
            }
        }
    }

    fn kill_own(&self) {
        // Chỉ buộc đóng khi biết chắc tiến trình là của phiên này và chưa giao cho người dùng.
        if self.own_pids.len() == 1 && !self.is_handed_over() {
            for pid in &self.own_pids {
                ffi::end_process(*pid, true, Duration::from_secs(10));
            }
        }
    }
}

impl Drop for WordConverter {
    fn drop(&mut self) {
        // Đóng channel: luồng STA gọi Quit rồi thoát.
        self.jobs.take();
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_secs(30);
            while !worker.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
        if self.is_handed_over() {
            live_pids().retain(|pid| !self.own_pids.contains(pid));
            return;
        }
        if self.own_pids.len() == 1 {
            for pid in &self.own_pids {
                if !ffi::end_process(*pid, false, Duration::from_secs(15)) {
                    ffi::end_process(*pid, true, Duration::from_secs(10));
                }
            }
            live_pids().retain(|pid| !self.own_pids.contains(pid));
        }
    }
}

/// Crate này không phụ thuộc thư viện log; cảnh báo hiếm gặp in ra stderr (bản phát hành không có console).
fn tracing_like_warn(message: &str) {
    eprintln!("[smks-merge::word] {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_locale_has_distinct_separators() {
        let locale = user_number_locale();
        assert_ne!(locale.decimal, locale.group);
    }

    /// Cần Microsoft Word. Giả lập người dùng mở tài liệu vào đúng phiên Word ẩn (qua `GetActiveObject` từ
    /// PowerShell): phiên phải hiện Word lên, không bị Quit hay buộc đóng.
    #[test]
    #[ignore]
    fn foreign_document_hands_session_over() {
        let mut word = WordConverter::start().unwrap();
        let pid = word.own_pids.clone();
        assert_eq!(pid.len(), 1);
        let script = "$w=[Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application'); \
                      $null=$w.Documents.Add(); 'ok'";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        if !text.contains("ok") {
            eprintln!(
                "không gắn được vào phiên Word: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        let end = Instant::now() + Duration::from_secs(10);
        while !word.is_broken() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(word.is_broken(), "phiên chưa được giao lại");
        let err = word
            .convert(Path::new("khong-co.docx"), Path::new("x.pdf"))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::MergeFailed);
        drop(word);
        kill_all_sessions();
        std::thread::sleep(Duration::from_millis(500));
        assert!(
            ffi::word_pids().contains(&pid[0]),
            "Word của người dùng bị đóng"
        );
        // Dọn: đóng Word đã giao (tài liệu trống, chưa có gì để mất).
        ffi::end_process(pid[0], true, Duration::from_secs(10));
    }

    /// Cần Microsoft Word: chạy tay bằng `cargo test -p smks-merge -- --ignored`.
    #[test]
    #[ignore]
    fn converts_sample_templates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/merge");
        let out = std::env::temp_dir().join("smks-word-test");
        std::fs::create_dir_all(&out).unwrap();
        let mut word = WordConverter::start().unwrap();
        for (name, pages) in [
            ("1_giay_moi_hop", 1),
            ("2_quyet_dinh_khen_thuong", 1),
            ("3_hop_dong_lao_dong", 3),
        ] {
            let pdf = out.join(format!("{name}.pdf"));
            assert_eq!(
                word.convert(&root.join(format!("{name}.docx")), &pdf)
                    .unwrap(),
                pages,
                "{name}"
            );
            assert!(std::fs::metadata(&pdf).unwrap().len() > 10_000);
        }
        let missing = word
            .convert(&root.join("khong-co.docx"), &out.join("x.pdf"))
            .unwrap_err();
        assert_eq!(missing.code, ErrorCode::MergeFailed);
        assert!(!word.is_broken());
        let pids = word.own_pids.clone();
        drop(word);
        std::thread::sleep(Duration::from_millis(500));
        assert!(
            pids.iter().all(|pid| !ffi::word_pids().contains(pid)),
            "Word chưa thoát"
        );
    }
}
