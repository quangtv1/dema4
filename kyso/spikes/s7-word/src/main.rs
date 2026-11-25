//! Spike S7: xuất DOCX ra PDF bằng Microsoft Word qua COM (gọi muộn bằng IDispatch) từ Rust.
//!
//! - Mọi lời gọi COM nằm trên một luồng STA riêng; luồng chính gửi việc qua channel.
//! - Chuỗi lệnh giống `WordPdfConverter.cs` của bản .NET 1.4.0:
//!   `Visible = false`, `DisplayAlerts = 0`, `Documents.Open(path, false, true, false)`,
//!   `ExportAsFixedFormat(pdf, 17)`, `Close(0)`, `Quit(0)`.
//! - Ghi nhớ PID của WINWORD.EXE do chính spike mở để buộc đóng khi quá thời gian.
//!
//! Cách dùng:
//!   s7-word <thư mục kết quả> [--repeat N] [--kill-test] <file.docx>...
//!
//! `--kill-test`: giả lập Word treo bằng cách buộc đóng tiến trình Word ngay khi đang chuyển file đầu
//! tiên, để xem lời gọi COM báo lỗi gì và luồng STA có thoát sạch không.

use std::collections::BTreeSet;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, CLSIDFromProgID, COINIT_APARTMENTTHREADED, CoCreateInstance,
    CoInitializeEx, CoUninitialize, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET,
    DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO, IDispatch,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, HSTRING, PCWSTR, w};

const LOCALE_USER_DEFAULT: u32 = 0x0400;
const WD_EXPORT_FORMAT_PDF: i32 = 17;
const WD_DO_NOT_SAVE_CHANGES: i32 = 0;
const WD_ALERTS_NONE: i32 = 0;
const FILE_TIMEOUT: Duration = Duration::from_secs(120);

/// Lớp bọc gọi muộn tối thiểu quanh IDispatch.
struct Dispatch(IDispatch);

impl Dispatch {
    fn id_of(&self, name: &str) -> Result<i32, String> {
        let wide = HSTRING::from(name);
        let names = [PCWSTR(wide.as_ptr())];
        let mut id = 0i32;
        // SAFETY: `names` trỏ tới chuỗi UTF-16 kết thúc bằng 0 còn sống suốt lời gọi; `id` hợp lệ.
        unsafe { self.0.GetIDsOfNames(&GUID::zeroed(), names.as_ptr(), 1, LOCALE_USER_DEFAULT, &mut id) }
            .map_err(|e| format!("Không tìm thấy thành viên {name}: {e}"))?;
        Ok(id)
    }

    /// `args` theo thứ tự tự nhiên; hàm tự đảo lại theo quy ước của IDispatch.
    fn invoke(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        mut args: Vec<VARIANT>,
    ) -> Result<VARIANT, String> {
        let id = self.id_of(name)?;
        args.reverse();
        let mut named = DISPID_PROPERTYPUT;
        let is_put = flags == DISPATCH_PROPERTYPUT;
        let params = DISPPARAMS {
            rgvarg: if args.is_empty() { null_mut() } else { args.as_mut_ptr() },
            rgdispidNamedArgs: if is_put { &mut named } else { null_mut() },
            cArgs: args.len() as u32,
            cNamedArgs: u32::from(is_put),
        };
        let mut result = VARIANT::default();
        let mut exception = EXCEPINFO::default();
        let mut bad_arg = 0u32;
        // SAFETY: mọi con trỏ trỏ tới biến cục bộ còn sống suốt lời gọi; `params` mô tả đúng `args`.
        let outcome = unsafe {
            self.0.Invoke(
                id,
                &GUID::zeroed(),
                LOCALE_USER_DEFAULT,
                flags,
                &params,
                Some(&mut result),
                Some(&mut exception),
                Some(&mut bad_arg),
            )
        };
        let description = exception.bstrDescription.to_string();
        // SAFETY: ba BSTR này do bên được gọi cấp phát (hoặc rỗng) và không còn được dùng nữa.
        unsafe {
            ManuallyDrop::drop(&mut exception.bstrSource);
            ManuallyDrop::drop(&mut exception.bstrDescription);
            ManuallyDrop::drop(&mut exception.bstrHelpFile);
        }
        match outcome {
            Ok(()) => Ok(result),
            Err(e) => Err(format!(
                "{name}: 0x{:08X} {}{}",
                e.code().0,
                e.message(),
                if description.is_empty() { String::new() } else { format!(" | {description}") }
            )),
        }
    }

    fn get_object(&self, name: &str) -> Result<Dispatch, String> {
        let value = self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())?;
        IDispatch::try_from(&value)
            .map(Dispatch)
            .map_err(|e| format!("{name} không trả về đối tượng: {e}"))
    }

    fn put(&self, name: &str, value: VARIANT) -> Result<(), String> {
        self.invoke(name, DISPATCH_PROPERTYPUT, vec![value]).map(|_| ())
    }

    fn call(&self, name: &str, args: Vec<VARIANT>) -> Result<VARIANT, String> {
        self.invoke(name, DISPATCH_METHOD, args)
    }
}

struct Job {
    docx: PathBuf,
    pdf: PathBuf,
    reply: mpsc::Sender<(Result<(), String>, Duration)>,
}

fn winword_pids() -> BTreeSet<u32> {
    let mut pids = BTreeSet::new();
    // SAFETY: ảnh chụp tiến trình là handle hợp lệ cho tới khi CloseHandle; `entry` có dwSize đúng.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return pids;
        };
        let mut entry =
            PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case("WINWORD.EXE") {
                pids.insert(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    pids
}

/// Chờ tiến trình thoát trong `wait`; nếu `force` thì buộc đóng trước. Trả về tiến trình đã thoát chưa.
fn end_process(pid: u32, force: bool, wait: Duration) -> bool {
    // SAFETY: handle mở ra được đóng lại trước khi rời hàm.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) else {
            // Không mở được: coi như tiến trình đã thoát.
            return !winword_pids().contains(&pid);
        };
        if force {
            let _ = TerminateProcess(handle, 1);
        }
        let exited = WaitForSingleObject(handle, wait.as_millis() as u32) == WAIT_OBJECT_0;
        let _ = CloseHandle(handle);
        exited
    }
}

fn word_thread(jobs: mpsc::Receiver<Job>, ready: mpsc::Sender<Result<Vec<u32>, String>>) {
    // SAFETY: khởi tạo COM cho riêng luồng này; CoUninitialize được gọi ở cuối hàm khi đã thành công.
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
        let _ = ready.send(Err(format!("CoInitializeEx: {e}")));
        return;
    }

    let run = || -> Result<(), String> {
        let before = winword_pids();
        // SAFETY: ProgID là chuỗi hằng; CoCreateInstance trả về giao diện đã AddRef.
        let app: IDispatch = unsafe {
            let clsid = CLSIDFromProgID(w!("Word.Application"))
                .map_err(|e| format!("Máy chưa cài Microsoft Word: {e}"))?;
            CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)
                .map_err(|e| format!("Không khởi động được Microsoft Word: {e}"))?
        };
        let app = Dispatch(app);
        let own: Vec<u32> = winword_pids().difference(&before).copied().collect();

        app.put("Visible", VARIANT::from(false))?;
        app.put("DisplayAlerts", VARIANT::from(WD_ALERTS_NONE))?;
        let documents = app.get_object("Documents")?;
        let _ = ready.send(Ok(own));

        for job in jobs {
            let started = Instant::now();
            let outcome = convert(&documents, &job.docx, &job.pdf);
            let _ = job.reply.send((outcome, started.elapsed()));
        }

        drop(documents);
        // Nếu Word đã bị buộc đóng thì Quit sẽ lỗi; bỏ qua.
        let _ = app.call("Quit", vec![VARIANT::from(WD_DO_NOT_SAVE_CHANGES)]);
        Ok(())
    };

    if let Err(e) = run() {
        let _ = ready.send(Err(e));
    }
    // SAFETY: cặp với CoInitializeEx thành công ở trên; mọi đối tượng COM của luồng đã được thả.
    unsafe { CoUninitialize() };
}

fn convert(documents: &Dispatch, docx: &Path, pdf: &Path) -> Result<(), String> {
    let opened = documents.call(
        "Open",
        vec![
            VARIANT::from(docx.to_string_lossy().as_ref()),
            VARIANT::from(false), // ConfirmConversions
            VARIANT::from(true),  // ReadOnly
            VARIANT::from(false), // AddToRecentFiles
        ],
    )?;
    let document = IDispatch::try_from(&opened)
        .map(Dispatch)
        .map_err(|e| format!("Documents.Open không trả về tài liệu: {e}"))?;

    let exported = document.call(
        "ExportAsFixedFormat",
        vec![VARIANT::from(pdf.to_string_lossy().as_ref()), VARIANT::from(WD_EXPORT_FORMAT_PDF)],
    );
    let closed = document.call("Close", vec![VARIANT::from(WD_DO_NOT_SAVE_CHANGES)]);
    exported.map(|_| ())?;
    closed.map(|_| ())
}

fn main() -> Result<(), String> {
    let mut repeat = 1usize;
    let mut kill_test = false;
    let mut positional = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repeat" => {
                repeat = args.next().and_then(|n| n.parse().ok()).ok_or("--repeat cần một số")?;
            }
            "--kill-test" => kill_test = true,
            _ => positional.push(arg),
        }
    }
    if positional.len() < 2 {
        return Err("Cách dùng: s7-word <thư mục kết quả> [--repeat N] [--kill-test] <file.docx>...".into());
    }
    let out_dir = std::path::absolute(&positional[0]).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let inputs: Vec<PathBuf> = positional[1..]
        .iter()
        .map(|p| std::path::absolute(p).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    let (job_tx, job_rx) = mpsc::channel::<Job>();
    let (ready_tx, ready_rx) = mpsc::channel();
    let started = Instant::now();
    let worker = std::thread::Builder::new()
        .name("word-sta".into())
        .spawn(move || word_thread(job_rx, ready_tx))
        .map_err(|e| e.to_string())?;

    let own_pids = ready_rx
        .recv_timeout(Duration::from_secs(60))
        .map_err(|_| "Word không khởi động trong 60 giây".to_owned())??;
    println!("Word sẵn sàng sau {:.2}s; PID do spike mở: {own_pids:?}", started.elapsed().as_secs_f32());
    if own_pids.len() != 1 {
        println!("CẢNH BÁO: không xác định được đúng một PID của Word ({} PID mới).", own_pids.len());
    }

    let mut failures = 0usize;
    let mut total = 0usize;
    'outer: for round in 0..repeat {
        for input in &inputs {
            total += 1;
            let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let pdf = out_dir.join(if repeat > 1 { format!("{stem}.{round}.pdf") } else { format!("{stem}.pdf") });
            let _ = std::fs::remove_file(&pdf);
            let (reply_tx, reply_rx) = mpsc::channel();
            if job_tx.send(Job { docx: input.clone(), pdf: pdf.clone(), reply: reply_tx }).is_err() {
                println!("LỖI: luồng Word đã dừng.");
                failures += 1;
                break 'outer;
            }

            if kill_test && total == 1 {
                std::thread::sleep(Duration::from_millis(150));
                for pid in &own_pids {
                    let exited = end_process(*pid, true, Duration::from_secs(10));
                    println!("--kill-test: buộc đóng WINWORD PID {pid}, đã thoát: {exited}");
                }
            }

            match reply_rx.recv_timeout(FILE_TIMEOUT) {
                Ok((Ok(()), elapsed)) => {
                    let size = std::fs::metadata(&pdf).map(|m| m.len()).unwrap_or(0);
                    println!("OK   {:>6.2}s {:>8} byte  {}", elapsed.as_secs_f32(), size, pdf.display());
                    if size == 0 {
                        failures += 1;
                    }
                }
                Ok((Err(message), elapsed)) => {
                    failures += 1;
                    println!("LỖI  {:>6.2}s {}  {}", elapsed.as_secs_f32(), input.display(), message);
                    if kill_test {
                        break 'outer;
                    }
                }
                Err(_) => {
                    failures += 1;
                    println!("QUÁ THỜI GIAN {}s: {}", FILE_TIMEOUT.as_secs(), input.display());
                    for pid in &own_pids {
                        end_process(*pid, true, Duration::from_secs(10));
                    }
                    break 'outer;
                }
            }
        }
    }

    // Đóng channel để luồng STA gọi Quit rồi thoát.
    drop(job_tx);
    let deadline = Instant::now() + Duration::from_secs(30);
    while !worker.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("Luồng STA đã thoát: {}", worker.is_finished());

    for pid in &own_pids {
        let exited = end_process(*pid, false, Duration::from_secs(15));
        if exited {
            println!("WINWORD PID {pid} đã tự thoát sau Quit.");
        } else {
            let killed = end_process(*pid, true, Duration::from_secs(10));
            println!("WINWORD PID {pid} KHÔNG tự thoát; buộc đóng: {killed}");
        }
    }
    println!(
        "Tổng: {total} lượt, {failures} lỗi, {:.2}s. WINWORD còn chạy trên máy: {:?}",
        started.elapsed().as_secs_f32(),
        winword_pids()
    );
    if failures == 0 { Ok(()) } else { Err(format!("{failures} lượt lỗi")) }
}
