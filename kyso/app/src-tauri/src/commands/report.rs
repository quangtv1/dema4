//! Xuất báo cáo kiểm tra chữ ký ra PDF (màn Kiểm tra, TD-57): in trang hiện tại của WebView2 bằng
//! `ICoreWebView2_7::PrintToPdf`. Giao diện đặt sẵn khối báo cáo chỉ hiện khi in (`@media print`), nên file PDF
//! chỉ có báo cáo, không có menu hay màn hình.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use smks_core::{ErrorCode, SmksError};

/// Thời gian chờ WebView2 in xong.
const PRINT_TIMEOUT: Duration = Duration::from_secs(60);

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::InternalError, message)
}

/// In giao diện (khối báo cáo) ra file PDF `path`.
#[tauri::command]
pub async fn report_export_pdf(
    window: tauri::WebviewWindow,
    path: String,
) -> Result<(), SmksError> {
    let target = Path::new(&path);
    let is_pdf = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    if !is_pdf
        || target
            .parent()
            .is_none_or(|p| !p.as_os_str().is_empty() && !p.is_dir())
    {
        return Err(SmksError::new(
            ErrorCode::OutputFolderInvalid,
            "Chọn tên file .pdf trong một thư mục có thật để lưu báo cáo.",
        ));
    }
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let error_tx = tx.clone();
    window
        .with_webview(move |webview| {
            #[cfg(windows)]
            if let Err(e) = ffi::print_to_pdf(&webview.controller(), &path, tx) {
                let _ = error_tx.send(Err(e));
            }
            #[cfg(not(windows))]
            {
                let _ = (webview, path, tx);
                let _ = error_tx.send(Err("Chỉ hỗ trợ trên Windows.".into()));
            }
        })
        .map_err(|e| fail(format!("Không truy cập được WebView để in: {e}")))?;
    super::blocking(move || match rx.recv_timeout(PRINT_TIMEOUT) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(fail(format!("Không in được báo cáo ra PDF: {e}"))),
        Err(_) => Err(fail("In báo cáo ra PDF quá lâu, đã dừng chờ.")),
    })
    .await
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod ffi {
    //! Lời gọi COM của WebView2. Mọi `unsafe` của module nằm ở đây.

    use std::sync::mpsc::Sender;

    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2_2, ICoreWebView2_7,
        ICoreWebView2Controller, ICoreWebView2Environment6,
    };
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{HSTRING, Interface};

    /// Bắt đầu in; kết quả gửi về `done` khi WebView2 in xong (gọi trên luồng giao diện).
    pub(super) fn print_to_pdf(
        controller: &ICoreWebView2Controller,
        path: &str,
        done: Sender<Result<(), String>>,
    ) -> Result<(), String> {
        let e = |x: windows::core::Error| x.message();
        // SAFETY: các con trỏ COM do WebView2 cấp, còn sống trong lời gọi; chỉ gọi phương thức của chúng.
        unsafe {
            let core = controller.CoreWebView2().map_err(e)?;
            let core7: ICoreWebView2_7 = core.cast().map_err(|_| {
                "WebView2 Runtime quá cũ, chưa hỗ trợ in ra PDF (cần bản 91 trở lên).".to_owned()
            })?;
            let env = core
                .cast::<ICoreWebView2_2>()
                .map_err(e)?
                .Environment()
                .map_err(e)?;
            let env6: ICoreWebView2Environment6 = env.cast().map_err(e)?;
            let settings = env6.CreatePrintSettings().map_err(e)?;
            settings
                .SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)
                .map_err(e)?;
            settings.SetShouldPrintHeaderAndFooter(false).map_err(e)?;
            settings.SetShouldPrintBackgrounds(true).map_err(e)?;
            for set in [
                settings.SetMarginTop(0.5),
                settings.SetMarginBottom(0.5),
                settings.SetMarginLeft(0.55),
                settings.SetMarginRight(0.55),
            ] {
                set.map_err(e)?;
            }
            let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
                let outcome = match result {
                    Ok(()) if ok => Ok(()),
                    Ok(()) => Err(
                        "WebView2 báo không in được (file đang mở ở chương trình khác?)."
                            .to_owned(),
                    ),
                    Err(err) => Err(err.message()),
                };
                let _ = done.send(outcome);
                Ok(())
            }));
            core7
                .PrintToPdf(&HSTRING::from(path), &settings, &handler)
                .map_err(e)
        }
    }
}
