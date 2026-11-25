//! Xem trước nhanh cho màn Tạo văn bản.
//!
//! Trước đây mỗi lần xem trước mở một phiên Word mới (khoảng 3 giây). Ở đây:
//!
//! - Một luồng riêng giữ **một phiên chuyển đổi chạy sẵn**, tự đóng khi rảnh [`PreviewOptions::idle_close`]
//!   hoặc khi đang chạy lô.
//! - **Bộ đệm theo nội dung**: khoá là SHA-256 của mẫu cùng giá trị đã điền, nên dòng đã xem (hoặc đổi qua
//!   lại tên file, thư mục…) trả về ngay; sửa mẫu hay dữ liệu thì khoá đổi, tự tạo lại.
//! - **Tạo trước** các dòng lân cận dòng đang xem khi rảnh; yêu cầu của người dùng luôn được làm trước, yêu
//!   cầu mới nhất trước yêu cầu cũ.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use smks_core::{ErrorCode, Result, SmksError};

use crate::batch::{PdfConverter, write_atomic};

/// Tham số của dịch vụ xem trước.
#[derive(Debug, Clone, Copy)]
pub struct PreviewOptions {
    /// Đóng phiên chuyển đổi (Word) khi không có việc trong khoảng này.
    pub idle_close: Duration,
    /// Số dòng phía sau dòng đang xem được tạo trước.
    pub ahead: usize,
    /// Số dòng phía trước dòng đang xem được tạo trước.
    pub behind: usize,
    /// Số bản xem trước giữ tối đa; quá thì bỏ bản lâu không dùng nhất.
    pub cache_limit: usize,
    /// Thời gian chờ tối đa cho một yêu cầu.
    pub wait_limit: Duration,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            idle_close: Duration::from_secs(120),
            ahead: 15,
            behind: 5,
            cache_limit: 300,
            wait_limit: Duration::from_secs(300),
        }
    }
}

/// Một bản xem trước đã tạo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub pdf: PathBuf,
    pub pages: u32,
}

type Key = [u8; 32];

#[derive(Clone)]
struct Task {
    key: Key,
    template: Arc<Vec<u8>>,
    /// `None`: xuất nguyên mẫu, không điền.
    values: Option<HashMap<String, String>>,
    /// Tô màu biến / giá trị đã điền.
    mark: bool,
}

struct Urgent {
    task: Task,
    reply: mpsc::Sender<Result<Rendered>>,
}

struct Entry {
    rendered: Rendered,
    used: u64,
}

struct Inner {
    cache: HashMap<Key, Entry>,
    /// Yêu cầu của người dùng; lấy từ cuối (mới nhất trước).
    urgent: Vec<Urgent>,
    prefetch: VecDeque<Task>,
    paused: bool,
    shutdown: bool,
    clock: u64,
    /// Phiên chuyển đổi đang mở (để biết có cần đóng khi rảnh).
    open: bool,
    /// Số lần gọi chuyển đổi, cho kiểm thử và log.
    conversions: u64,
}

impl Inner {
    /// Bản trong bộ đệm còn file trên đĩa; file bị xoá thì bỏ khỏi bộ đệm.
    fn hit(&mut self, key: &Key) -> Option<Rendered> {
        self.clock += 1;
        let clock = self.clock;
        match self.cache.get_mut(key) {
            Some(entry) if entry.rendered.pdf.is_file() => {
                entry.used = clock;
                Some(entry.rendered.clone())
            }
            Some(_) => {
                self.cache.remove(key);
                None
            }
            None => None,
        }
    }

    fn insert(&mut self, key: Key, rendered: Rendered, limit: usize) {
        self.clock += 1;
        self.cache.insert(
            key,
            Entry {
                rendered,
                used: self.clock,
            },
        );
        while self.cache.len() > limit.max(1) {
            let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            if let Some(entry) = self.cache.remove(&oldest) {
                let _ = std::fs::remove_file(&entry.rendered.pdf);
            }
        }
    }
}

struct Shared {
    inner: Mutex<Inner>,
    wake: Condvar,
    dir: PathBuf,
    options: PreviewOptions,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Dịch vụ xem trước: một luồng nền giữ phiên chuyển đổi và bộ đệm.
pub struct PreviewService {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl fmt::Debug for PreviewService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewService")
            .field("dir", &self.shared.dir)
            .finish_non_exhaustive()
    }
}

fn template_hash(template: &[u8]) -> Key {
    Sha256::digest(template).into()
}

fn template_key(template_hash: &Key, mark: bool) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(if mark {
        b"mau-to\0".as_slice()
    } else {
        b"mau\0".as_slice()
    });
    hasher.update(template_hash);
    hasher.finalize().into()
}

fn record_key(template_hash: &Key, values: &HashMap<String, String>, mark: bool) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(if mark {
        b"dong-to\0".as_slice()
    } else {
        b"dong\0".as_slice()
    });
    hasher.update(template_hash);
    let mut pairs: Vec<_> = values.iter().collect();
    pairs.sort();
    for (field, value) in pairs {
        for part in [field, value] {
            hasher.update((part.len() as u64).to_le_bytes());
            hasher.update(part.as_bytes());
        }
    }
    hasher.finalize().into()
}

fn hex(key: &Key) -> String {
    key[..12].iter().map(|b| format!("{b:02x}")).collect()
}

/// Thứ tự tạo trước quanh dòng `index` (không gồm chính nó): xen kẽ phía sau và phía trước, ưu tiên phía sau.
pub fn neighbours(len: usize, index: usize, ahead: usize, behind: usize) -> Vec<usize> {
    let mut order = Vec::new();
    for distance in 1..=ahead.max(behind) {
        if distance <= ahead && index + distance < len {
            order.push(index + distance);
        }
        if distance <= behind && distance <= index {
            order.push(index - distance);
        }
    }
    order
}

impl PreviewService {
    /// Mở dịch vụ, ghi bản xem trước vào `dir`. `converter` chỉ được khởi động khi có việc đầu tiên.
    pub fn start(
        dir: PathBuf,
        converter: Box<dyn PdfConverter + Send>,
        options: PreviewOptions,
    ) -> Result<Self> {
        std::fs::create_dir_all(&dir).map_err(|e| {
            SmksError::new(
                ErrorCode::MergeFailed,
                format!("Không tạo được thư mục xem trước: {e}"),
            )
        })?;
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                cache: HashMap::new(),
                urgent: Vec::new(),
                prefetch: VecDeque::new(),
                paused: false,
                shutdown: false,
                clock: 0,
                open: false,
                conversions: 0,
            }),
            wake: Condvar::new(),
            dir,
            options,
        });
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("xem-truoc".into())
            .spawn(move || worker(&worker_shared, converter))
            .map_err(|e| {
                SmksError::new(
                    ErrorCode::InternalError,
                    format!("Không tạo được luồng xem trước: {e}"),
                )
            })?;
        Ok(Self {
            shared,
            worker: Mutex::new(Some(worker)),
        })
    }

    /// PDF của nguyên mẫu (tab Mẫu); `mark` thì tô màu các biến.
    pub fn render_template(&self, template: Arc<Vec<u8>>, mark: bool) -> Result<Rendered> {
        let key = template_key(&template_hash(&template), mark);
        self.request(Task {
            key,
            template,
            values: None,
            mark,
        })
    }

    /// PDF của mẫu đã điền `values` (một dòng dữ liệu).
    pub fn render_record(
        &self,
        template: Arc<Vec<u8>>,
        values: &HashMap<String, String>,
        mark: bool,
    ) -> Result<Rendered> {
        let key = record_key(&template_hash(&template), values, mark);
        self.request(Task {
            key,
            template,
            values: Some(values.clone()),
            mark,
        })
    }

    /// Thay danh sách tạo trước bằng các dòng `records` (đã xếp theo thứ tự ưu tiên).
    pub fn prefetch(
        &self,
        template: Arc<Vec<u8>>,
        records: Vec<HashMap<String, String>>,
        mark: bool,
    ) {
        let hash = template_hash(&template);
        let tasks: VecDeque<Task> = records
            .into_iter()
            .map(|values| Task {
                key: record_key(&hash, &values, mark),
                template: template.clone(),
                values: Some(values),
                mark,
            })
            .collect();
        let mut inner = self.shared.lock();
        inner.prefetch = tasks;
        drop(inner);
        self.shared.wake.notify_all();
    }

    /// Tạo trước PDF của nguyên mẫu (gọi lúc vừa chọn mẫu: Word khởi động sẵn trong lúc người dùng chọn
    /// dữ liệu). Không xoá các việc tạo trước khác.
    pub fn warm_template(&self, template: Arc<Vec<u8>>, mark: bool) {
        let key = template_key(&template_hash(&template), mark);
        let mut inner = self.shared.lock();
        inner.prefetch.push_front(Task {
            key,
            template,
            values: None,
            mark,
        });
        drop(inner);
        self.shared.wake.notify_all();
    }

    /// Tạm dừng tạo trước và đóng phiên chuyển đổi (khi đang chạy lô, lô dùng phiên Word riêng).
    pub fn set_paused(&self, paused: bool) {
        let mut inner = self.shared.lock();
        inner.paused = paused;
        if paused {
            inner.prefetch.clear();
        }
        drop(inner);
        self.shared.wake.notify_all();
    }

    /// Số lần đã gọi chuyển đổi (kiểm thử, log).
    pub fn conversions(&self) -> u64 {
        self.shared.lock().conversions
    }

    /// Phiên chuyển đổi đang mở hay không.
    pub fn is_open(&self) -> bool {
        self.shared.lock().open
    }

    /// Còn việc tạo trước chưa làm hay không.
    pub fn prefetch_pending(&self) -> usize {
        self.shared.lock().prefetch.len()
    }

    fn request(&self, task: Task) -> Result<Rendered> {
        let (reply, answer) = mpsc::channel();
        {
            let mut inner = self.shared.lock();
            if let Some(hit) = inner.hit(&task.key) {
                return Ok(hit);
            }
            if inner.shutdown {
                return Err(SmksError::new(
                    ErrorCode::MergeFailed,
                    "Dịch vụ xem trước đã dừng.",
                ));
            }
            inner.urgent.push(Urgent { task, reply });
        }
        self.shared.wake.notify_all();
        match answer.recv_timeout(self.shared.options.wait_limit) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(SmksError::new(
                ErrorCode::MergeFailed,
                "Tạo bản xem trước quá lâu.",
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(SmksError::new(
                ErrorCode::MergeFailed,
                "Dịch vụ xem trước đã dừng.",
            )),
        }
    }
}

impl PreviewService {
    /// Dừng luồng nền, đóng phiên chuyển đổi, xoá bản xem trước (gọi khi ứng dụng thoát).
    pub fn shutdown(&self) {
        self.shared.lock().shutdown = true;
        self.shared.wake.notify_all();
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

enum Next {
    Urgent(Urgent),
    Prefetch(Task),
    Close,
    Shutdown,
}

fn next_work(shared: &Shared, last_work: Instant) -> Next {
    let mut inner = shared.lock();
    loop {
        if inner.shutdown {
            return Next::Shutdown;
        }
        while let Some(urgent) = inner.urgent.pop() {
            match inner.hit(&urgent.task.key) {
                // Đã có (ví dụ vừa được tạo trước): trả ngay.
                Some(hit) => {
                    let _ = urgent.reply.send(Ok(hit));
                }
                None => return Next::Urgent(urgent),
            }
        }
        if !inner.paused {
            while let Some(task) = inner.prefetch.pop_front() {
                if inner.hit(&task.key).is_none() {
                    return Next::Prefetch(task);
                }
            }
        }
        if inner.open {
            let idle = last_work.elapsed();
            if inner.paused || idle >= shared.options.idle_close {
                return Next::Close;
            }
            let remaining = shared.options.idle_close - idle;
            inner = shared
                .wake
                .wait_timeout(inner, remaining)
                .map(|(guard, _)| guard)
                .unwrap_or_else(|e| e.into_inner().0);
        } else {
            inner = shared
                .wake
                .wait(inner)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

fn render(shared: &Shared, converter: &mut dyn PdfConverter, task: &Task) -> Result<Rendered> {
    let bytes = match (&task.values, task.mark) {
        (Some(values), false) => crate::docx::fill(&task.template, values)?,
        (Some(values), true) => crate::docx::fill_marked(&task.template, values)?,
        (None, false) => task.template.as_ref().clone(),
        (None, true) => crate::docx::mark_fields(&task.template)?,
    };
    let name = hex(&task.key);
    let docx = shared.dir.join(format!("{name}.docx"));
    let pdf = shared.dir.join(format!("{name}.pdf"));
    write_atomic(&docx, &bytes).map_err(|e| {
        SmksError::new(
            ErrorCode::MergeFailed,
            format!("Không ghi được file tạm: {e}"),
        )
    })?;
    let result = converter.convert(&docx, &pdf);
    let _ = std::fs::remove_file(&docx);
    let pages = result?;
    Ok(Rendered { pdf, pages })
}

fn worker(shared: &Shared, mut converter: Box<dyn PdfConverter + Send>) {
    let mut last_work = Instant::now();
    loop {
        match next_work(shared, last_work) {
            Next::Shutdown => break,
            Next::Close => {
                converter.close();
                shared.lock().open = false;
            }
            Next::Urgent(urgent) => {
                let result = render(shared, converter.as_mut(), &urgent.task);
                finish(shared, &urgent.task, &result);
                last_work = Instant::now();
                let _ = urgent.reply.send(result);
            }
            Next::Prefetch(task) => {
                let result = render(shared, converter.as_mut(), &task);
                finish(shared, &task, &result);
                last_work = Instant::now();
            }
        }
    }
    converter.close();
    // Bản xem trước chỉ dùng trong phiên làm việc này.
    let files: Vec<PathBuf> = {
        let mut inner = shared.lock();
        inner.urgent.clear();
        inner.open = false;
        inner.cache.drain().map(|(_, e)| e.rendered.pdf).collect()
    };
    for file in files {
        let _ = std::fs::remove_file(file);
    }
}

fn finish(shared: &Shared, task: &Task, result: &Result<Rendered>) {
    let mut inner = shared.lock();
    inner.open = true;
    inner.conversions += 1;
    match result {
        Ok(rendered) => inner.insert(task.key, rendered.clone(), shared.options.cache_limit),
        // Máy không có Word: tạo trước chỉ lặp lại cùng lỗi.
        Err(e) if e.code == ErrorCode::MergeConverterUnavailable => inner.prefetch.clear(),
        Err(_) => {}
    }
}

/// Dọn file trong `dir` cũ hơn `older_than` (bản xem trước do lần chạy trước để lại).
pub fn clean_dir(dir: &Path, older_than: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let age = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok());
        if age.is_some_and(|age| age > older_than) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Chép DOCX thành "PDF", đếm số lần gọi, ghi lại lần đóng.
    struct Fake {
        calls: Arc<AtomicU32>,
        closes: Arc<AtomicU32>,
        delay: Duration,
    }

    impl PdfConverter for Fake {
        fn convert(&mut self, docx: &Path, pdf: &Path) -> Result<u32> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(self.delay);
            std::fs::copy(docx, pdf)
                .map(|_| 1)
                .map_err(|e| SmksError::new(ErrorCode::MergeFailed, e.to_string()))
        }
        fn close(&mut self) {
            self.closes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn template() -> Arc<Vec<u8>> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/merge/2_quyet_dinh_khen_thuong.docx");
        Arc::new(std::fs::read(path).unwrap())
    }

    fn values(n: usize) -> HashMap<String, String> {
        HashMap::from([
            ("ho_ten".to_string(), format!("Nguyễn Văn {n}")),
            ("so".to_string(), n.to_string()),
        ])
    }

    fn service(
        name: &str,
        options: PreviewOptions,
    ) -> (PreviewService, Arc<AtomicU32>, Arc<AtomicU32>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("smks-preview-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let calls = Arc::new(AtomicU32::new(0));
        let closes = Arc::new(AtomicU32::new(0));
        let fake = Fake {
            calls: calls.clone(),
            closes: closes.clone(),
            delay: Duration::from_millis(20),
        };
        let svc = PreviewService::start(dir.clone(), Box::new(fake), options).unwrap();
        (svc, calls, closes, dir)
    }

    fn wait_until(mut done: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < end, "quá thời gian chờ");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn same_content_is_rendered_once() {
        let (svc, calls, _, dir) = service("once", PreviewOptions::default());
        let tpl = template();
        let first = svc.render_record(tpl.clone(), &values(1), false).unwrap();
        let again = svc.render_record(tpl.clone(), &values(1), false).unwrap();
        assert_eq!(first, again);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(first.pdf.starts_with(&dir) && first.pdf.is_file());
        // Không còn DOCX tạm.
        let leftovers = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x != "pdf"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn different_values_or_template_give_new_file() {
        let (svc, calls, _, _) = service("khac", PreviewOptions::default());
        let tpl = template();
        let a = svc.render_record(tpl.clone(), &values(1), false).unwrap();
        let b = svc.render_record(tpl.clone(), &values(2), false).unwrap();
        // Cùng giá trị nhưng mẫu khác (ví dụ người dùng sửa mẫu) thì tạo lại.
        let other = Arc::new(
            std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/merge/1_giay_moi_hop.docx"),
            )
            .unwrap(),
        );
        let c = svc.render_record(other, &values(1), false).unwrap();
        let t = svc.render_template(tpl, false).unwrap();
        assert_ne!(a.pdf, b.pdf);
        assert_ne!(a.pdf, c.pdf);
        assert_ne!(a.pdf, t.pdf);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn deleted_file_is_rendered_again() {
        let (svc, calls, _, _) = service("xoa", PreviewOptions::default());
        let tpl = template();
        let a = svc.render_record(tpl.clone(), &values(1), false).unwrap();
        std::fs::remove_file(&a.pdf).unwrap();
        let b = svc.render_record(tpl, &values(1), false).unwrap();
        assert!(b.pdf.is_file());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn prefetched_rows_are_instant() {
        let (svc, calls, _, _) = service("truoc", PreviewOptions::default());
        let tpl = template();
        svc.render_record(tpl.clone(), &values(0), false).unwrap();
        svc.prefetch(tpl.clone(), (1..=4).map(values).collect(), false);
        wait_until(|| calls.load(Ordering::SeqCst) == 5 && svc.prefetch_pending() == 0);
        std::thread::sleep(Duration::from_millis(50));
        for n in 1..=4 {
            svc.render_record(tpl.clone(), &values(n), false).unwrap();
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            5,
            "dòng đã tạo trước không gọi lại Word"
        );
    }

    #[test]
    fn paused_service_does_not_prefetch_and_closes() {
        let (svc, calls, closes, _) = service("dung", PreviewOptions::default());
        let tpl = template();
        svc.render_record(tpl.clone(), &values(0), false).unwrap();
        svc.set_paused(true);
        wait_until(|| closes.load(Ordering::SeqCst) >= 1);
        assert!(!svc.is_open());
        svc.prefetch(tpl.clone(), (1..=3).map(values).collect(), false);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        svc.set_paused(false);
        svc.prefetch(tpl, (1..=3).map(values).collect(), false);
        wait_until(|| calls.load(Ordering::SeqCst) == 4);
    }

    #[test]
    fn idle_session_is_closed() {
        let options = PreviewOptions {
            idle_close: Duration::from_millis(150),
            ..PreviewOptions::default()
        };
        let (svc, _, closes, _) = service("ranh", options);
        svc.render_template(template(), false).unwrap();
        assert!(svc.is_open());
        wait_until(|| closes.load(Ordering::SeqCst) == 1);
        assert!(!svc.is_open());
    }

    #[test]
    fn cache_limit_evicts_least_recently_used() {
        let options = PreviewOptions {
            cache_limit: 2,
            ..PreviewOptions::default()
        };
        let (svc, calls, _, _) = service("gioihan", options);
        let tpl = template();
        let a = svc.render_record(tpl.clone(), &values(1), false).unwrap();
        svc.render_record(tpl.clone(), &values(2), false).unwrap();
        svc.render_record(tpl.clone(), &values(1), false).unwrap(); // dùng lại 1
        svc.render_record(tpl.clone(), &values(3), false).unwrap(); // bỏ 2
        assert!(a.pdf.is_file());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        svc.render_record(tpl, &values(2), false).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn drop_closes_converter_and_removes_files() {
        let (svc, _, closes, _) = service("tat", PreviewOptions::default());
        let a = svc.render_template(template(), false).unwrap();
        drop(svc);
        assert!(closes.load(Ordering::SeqCst) >= 1);
        assert!(!a.pdf.exists());
    }

    /// Cần Microsoft Word: `cargo test -p smks-merge --release -- --ignored --nocapture preview_with_word`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn preview_with_word_timings() {
        let dir = std::env::temp_dir().join("smks-preview-word");
        let _ = std::fs::remove_dir_all(&dir);
        let converter: Option<crate::word::WordConverter> = None;
        let svc =
            PreviewService::start(dir, Box::new(converter), PreviewOptions::default()).unwrap();
        let tpl = template();
        let timed = |n: usize| {
            let started = Instant::now();
            let r = svc.render_record(tpl.clone(), &values(n), false).unwrap();
            assert!(r.pages >= 1 && std::fs::metadata(&r.pdf).unwrap().len() > 10_000);
            started.elapsed()
        };
        let cold = timed(1);
        let warm = timed(2);
        let cached = timed(1);
        svc.prefetch(tpl.clone(), (3..=5).map(values).collect(), false);
        let end = Instant::now() + Duration::from_secs(60);
        while svc.conversions() < 5 && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
        let prefetched = timed(4);
        println!(
            "lan dau {cold:?} | dong moi, Word da mo {warm:?} | dong da xem {cached:?} | dong tao truoc {prefetched:?}"
        );
        assert!(cached < Duration::from_millis(50));
        assert!(prefetched < Duration::from_millis(50));
        assert!(warm < cold);
        // Word mở được bản tô màu (run bị tách, w:shd chèn vào w:rPr) của cả ba mẫu.
        for name in [
            "1_giay_moi_hop",
            "2_quyet_dinh_khen_thuong",
            "3_hop_dong_lao_dong",
        ] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../fixtures/merge/{name}.docx"));
            let bytes = Arc::new(std::fs::read(path).unwrap());
            let fields = crate::docx::read_fields(&bytes).unwrap();
            let map: HashMap<String, String> = fields
                .iter()
                .skip(1)
                .map(|f| (f.clone(), format!("Giá trị {f}")))
                .collect();
            let marked = svc.render_template(bytes.clone(), true).unwrap();
            let plain = svc.render_template(bytes.clone(), false).unwrap();
            let filled = svc.render_record(bytes, &map, true).unwrap();
            assert_ne!(marked.pdf, plain.pdf);
            assert_eq!(marked.pages, plain.pages, "{name}");
            assert!(filled.pages >= 1, "{name}");
            println!(
                "{name}: to mau {} trang -> {}",
                marked.pages,
                marked.pdf.display()
            );
        }
        let keep = std::env::temp_dir().join("smks-preview-word-mau");
        let _ = std::fs::create_dir_all(&keep);
        for entry in std::fs::read_dir(std::env::temp_dir().join("smks-preview-word"))
            .unwrap()
            .flatten()
        {
            let _ = std::fs::copy(entry.path(), keep.join(entry.file_name()));
        }
        drop(svc);
    }

    #[test]
    fn neighbour_order() {
        assert_eq!(neighbours(10, 4, 3, 2), vec![5, 3, 6, 2, 7]);
        assert_eq!(neighbours(3, 0, 5, 5), vec![1, 2]);
        assert_eq!(neighbours(3, 2, 5, 1), vec![1]);
        assert!(neighbours(1, 0, 5, 5).is_empty());
    }
}
