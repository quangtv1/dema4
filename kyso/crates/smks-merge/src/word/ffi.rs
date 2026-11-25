//! Lời gọi Windows API cho [`super`]: COM (IDispatch), danh sách tiến trình, cài đặt vùng.
//! Mọi `unsafe` của crate nằm ở đây, mỗi khối có chú thích SAFETY.

use std::collections::BTreeSet;
use std::mem::ManuallyDrop;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_SDECIMAL, LOCALE_STHOUSAND};
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

use super::Job;
use crate::numfmt::NumberLocale;
use smks_core::consts::merge::{
    WORD_ALERTS_NONE, WORD_DO_NOT_SAVE_CHANGES, WORD_EXPORT_FORMAT_PDF,
};

const LOCALE_USER_DEFAULT: u32 = 0x0400;
/// `wdStatisticPages`.
const WD_STATISTIC_PAGES: i32 = 2;
/// `wdAlertsAll`: trả lại hộp thoại bình thường khi giao Word cho người dùng.
const WORD_ALERTS_ALL: i32 = -1;
/// Chu kỳ kiểm tra tài liệu lạ khi phiên rảnh.
const IDLE_CHECK: Duration = Duration::from_secs(1);

pub(super) fn word_registered() -> bool {
    // SAFETY: tham số là chuỗi hằng kết thúc bằng 0.
    unsafe { CLSIDFromProgID(w!("Word.Application")) }.is_ok()
}

pub(super) fn user_number_locale() -> NumberLocale {
    let read = |kind: u32| -> Option<char> {
        let mut buffer = [0u16; 8];
        // SAFETY: bộ đệm hợp lệ trong suốt lời gọi; tên vùng null nghĩa là vùng mặc định của người dùng.
        let length = unsafe { GetLocaleInfoEx(PCWSTR::null(), kind, Some(&mut buffer)) };
        let length = usize::try_from(length).ok()?.checked_sub(1)?;
        String::from_utf16(&buffer[..length]).ok()?.chars().next()
    };
    match (read(LOCALE_SDECIMAL), read(LOCALE_STHOUSAND)) {
        (Some(decimal), Some(group)) if decimal != group => NumberLocale { decimal, group },
        (Some(decimal), _) => NumberLocale {
            decimal,
            group: if decimal == ',' { '.' } else { ',' },
        },
        _ => NumberLocale::default(),
    }
}

/// Lớp bọc gọi muộn tối thiểu quanh IDispatch.
struct Dispatch(IDispatch);

impl Dispatch {
    fn id_of(&self, name: &str) -> std::result::Result<i32, String> {
        let wide = HSTRING::from(name);
        let names = [PCWSTR(wide.as_ptr())];
        let mut id = 0i32;
        // SAFETY: `names` trỏ tới chuỗi UTF-16 kết thúc bằng 0 còn sống suốt lời gọi; `id` hợp lệ.
        unsafe {
            self.0.GetIDsOfNames(
                &GUID::zeroed(),
                names.as_ptr(),
                1,
                LOCALE_USER_DEFAULT,
                &mut id,
            )
        }
        .map_err(|e| format!("không tìm thấy {name}: 0x{:08X}", e.code().0))?;
        Ok(id)
    }

    /// `args` theo thứ tự tự nhiên; hàm tự đảo lại theo quy ước của IDispatch.
    fn invoke(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        mut args: Vec<VARIANT>,
    ) -> std::result::Result<VARIANT, String> {
        let id = self.id_of(name)?;
        args.reverse();
        let mut named = DISPID_PROPERTYPUT;
        let is_put = flags == DISPATCH_PROPERTYPUT;
        let params = DISPPARAMS {
            rgvarg: if args.is_empty() {
                null_mut()
            } else {
                args.as_mut_ptr()
            },
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
        // SAFETY: ba BSTR do bên được gọi cấp phát (hoặc rỗng) và không còn được dùng.
        unsafe {
            ManuallyDrop::drop(&mut exception.bstrSource);
            ManuallyDrop::drop(&mut exception.bstrDescription);
            ManuallyDrop::drop(&mut exception.bstrHelpFile);
        }
        outcome.map(|()| result).map_err(|e| {
            let detail = if description.is_empty() {
                e.message()
            } else {
                description
            };
            format!("{name} lỗi 0x{:08X}: {detail}", e.code().0)
        })
    }

    fn object(&self, name: &str) -> std::result::Result<Dispatch, String> {
        let value = self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())?;
        IDispatch::try_from(&value)
            .map(Dispatch)
            .map_err(|e| format!("{name} không trả về đối tượng: {e}"))
    }

    fn put(&self, name: &str, value: VARIANT) -> std::result::Result<(), String> {
        self.invoke(name, DISPATCH_PROPERTYPUT, vec![value])
            .map(|_| ())
    }

    fn call(&self, name: &str, args: Vec<VARIANT>) -> std::result::Result<VARIANT, String> {
        self.invoke(name, DISPATCH_METHOD, args)
    }
}

pub(super) fn word_pids() -> BTreeSet<u32> {
    let mut pids = BTreeSet::new();
    // SAFETY: handle ảnh chụp hợp lệ tới khi CloseHandle; `entry.dwSize` đặt đúng kích thước.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return pids;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length])
                .eq_ignore_ascii_case("WINWORD.EXE")
            {
                pids.insert(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    pids
}

/// Chờ tiến trình thoát trong `wait`; `force` thì buộc đóng trước. Trả về tiến trình đã thoát chưa.
pub(super) fn end_process(pid: u32, force: bool, wait: Duration) -> bool {
    // SAFETY: handle mở ra được đóng trước khi rời hàm.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) else {
            return !word_pids().contains(&pid);
        };
        if force {
            let _ = TerminateProcess(handle, 1);
        }
        let exited = WaitForSingleObject(handle, wait.as_millis() as u32) == WAIT_OBJECT_0;
        let _ = CloseHandle(handle);
        exited
    }
}

/// Số tài liệu đang mở trong phiên. Tài liệu của phiên được đóng ngay sau mỗi việc, nên lúc rảnh mà còn tài
/// liệu nghĩa là người dùng đã mở tài liệu của họ vào đúng phiên Word ẩn này.
fn open_documents(documents: &Dispatch) -> i32 {
    documents
        .invoke("Count", DISPATCH_PROPERTYGET, Vec::new())
        .ok()
        .and_then(|v| i32::try_from(&v).ok())
        .unwrap_or(0)
}

/// Thân luồng STA: mở Word, xử lý việc tới khi channel đóng, rồi Quit.
///
/// Khi rảnh, mỗi giây kiểm tra phiên có tài liệu lạ không. Có thì hiện Word lên cho người dùng, ngừng nhận
/// việc, đặt `handed_over` và **không** Quit (Quit sẽ bỏ thay đổi chưa lưu của họ).
pub(super) fn run_session(
    jobs: mpsc::Receiver<Job>,
    ready: mpsc::Sender<std::result::Result<Vec<u32>, String>>,
    handed_over: Arc<AtomicBool>,
) {
    // SAFETY: khởi tạo COM cho riêng luồng này; CoUninitialize ở cuối khi khởi tạo thành công.
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
        let _ = ready.send(Err(format!("CoInitializeEx: {e}")));
        return;
    }
    let outcome = (|| -> std::result::Result<(), String> {
        let before = word_pids();
        // SAFETY: ProgID là chuỗi hằng; CoCreateInstance trả về giao diện đã AddRef.
        let app: IDispatch = unsafe {
            let clsid = CLSIDFromProgID(w!("Word.Application")).map_err(|e| e.message())?;
            CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER).map_err(|e| e.message())?
        };
        let app = Dispatch(app);
        let own: Vec<u32> = word_pids().difference(&before).copied().collect();
        app.put("Visible", VARIANT::from(false))?;
        app.put("DisplayAlerts", VARIANT::from(WORD_ALERTS_NONE))?;
        let documents = app.object("Documents")?;
        let _ = ready.send(Ok(own.clone()));

        let foreign;
        loop {
            match jobs.recv_timeout(IDLE_CHECK) {
                Ok(job) => {
                    let _ = job.reply.send(convert(&documents, &job));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if open_documents(&documents) > 0 {
                        foreign = true;
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    foreign = open_documents(&documents) > 0;
                    break;
                }
            }
        }
        drop(documents);
        if foreign {
            handed_over.store(true, Ordering::SeqCst);
            // Ứng dụng thoát cũng không được buộc đóng Word này nữa.
            super::live_pids().retain(|pid| !own.contains(pid));
            let _ = app.put("DisplayAlerts", VARIANT::from(WORD_ALERTS_ALL));
            let _ = app.put("Visible", VARIANT::from(true));
        } else {
            // Word đã bị buộc đóng thì Quit lỗi; bỏ qua.
            let _ = app.call("Quit", vec![VARIANT::from(WORD_DO_NOT_SAVE_CHANGES)]);
        }
        Ok(())
    })();
    if let Err(message) = outcome {
        let _ = ready.send(Err(message));
    }
    // SAFETY: cặp với CoInitializeEx thành công; mọi đối tượng COM của luồng đã được thả.
    unsafe { CoUninitialize() };
}

fn convert(documents: &Dispatch, job: &Job) -> std::result::Result<u32, String> {
    let opened = documents.call(
        "Open",
        vec![
            VARIANT::from(job.docx.to_string_lossy().as_ref()),
            VARIANT::from(false), // ConfirmConversions
            VARIANT::from(true),  // ReadOnly
            VARIANT::from(false), // AddToRecentFiles
        ],
    )?;
    let document = IDispatch::try_from(&opened)
        .map(Dispatch)
        .map_err(|e| format!("Documents.Open không trả về tài liệu: {e}"))?;
    let pages = document
        .call("ComputeStatistics", vec![VARIANT::from(WD_STATISTIC_PAGES)])
        .ok()
        .and_then(|v| i32::try_from(&v).ok())
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    let exported = document.call(
        "ExportAsFixedFormat",
        vec![
            VARIANT::from(job.pdf.to_string_lossy().as_ref()),
            VARIANT::from(WORD_EXPORT_FORMAT_PDF),
        ],
    );
    let closed = document.call("Close", vec![VARIANT::from(WORD_DO_NOT_SAVE_CHANGES)]);
    exported?;
    closed?;
    Ok(pages)
}
