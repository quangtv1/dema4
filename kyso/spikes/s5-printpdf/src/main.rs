//! Spike S5: in HTML ra PDF bằng WebView2 (`ICoreWebView2_7::PrintToPdf`) từ một ứng dụng Tauri 2.
//!
//! Cửa sổ ẩn. Chờ trang khởi đầu của ứng dụng tải xong, rồi nạp HTML bằng `NavigateToString`, chờ
//! `NavigationCompleted` của chính lần nạp đó rồi in.
//!
//! Cách dùng: s5-printpdf <file.html> <ra.pdf> <css|settings>
//!   css      : không truyền cài đặt in, để xem CSS `@page` có được tôn trọng không.
//!   settings : truyền `ICoreWebView2PrintSettings` khổ A4, lề 0, in nền, không đầu trang/chân trang.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, Webview};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2_7, ICoreWebView2Environment6,
    ICoreWebView2PrintSettings,
};
use webview2_com::{NavigationCompletedEventHandler, PrintToPdfCompletedHandler};
use windows::core::{HSTRING, Interface, PWSTR};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 || !matches!(args[2].as_str(), "css" | "settings") {
        eprintln!("Cách dùng: s5-printpdf <file.html> <ra.pdf> <css|settings>");
        std::process::exit(2);
    }
    let html = Arc::new(std::fs::read_to_string(&args[0]).expect("không đọc được file HTML"));
    let output = std::path::absolute(&args[1]).expect("đường dẫn kết quả không hợp lệ");
    let _ = std::fs::remove_file(&output);
    let use_settings = args[2] == "settings";
    let started = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .on_page_load(move |webview, payload| {
            // Chỉ bắt đầu một lần, khi trang khởi đầu của ứng dụng đã tải xong.
            if payload.event() != PageLoadEvent::Finished || started.swap(true, Ordering::SeqCst) {
                return;
            }
            start_print(webview, html.clone(), output.clone(), use_settings);
        })
        .run(tauri::generate_context!())
        .expect("không chạy được ứng dụng spike");
}

fn start_print(webview: &Webview, html: Arc<String>, output: PathBuf, use_settings: bool) {
    let handle: AppHandle = webview.app_handle().clone();
    let fail_handle = handle.clone();
    let scheduled = webview.with_webview(move |platform| {
        let began = Instant::now();
        // SAFETY: closure chạy trên luồng giao diện của WebView2; mọi giao diện COM dùng ở đây đều lấy
        // từ chính webview đó và được giữ sống bằng bộ đếm tham chiếu của COM.
        let outcome = (|| -> windows::core::Result<()> {
            unsafe {
                let core = platform.controller().CoreWebView2()?;
                let core7: ICoreWebView2_7 = core.cast()?;
                let settings: Option<ICoreWebView2PrintSettings> = if use_settings {
                    let environment: ICoreWebView2Environment6 = platform.environment().cast()?;
                    let s = environment.CreatePrintSettings()?;
                    s.SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)?;
                    // Đơn vị inch: A4 = 8,27 x 11,69.
                    s.SetPageWidth(8.27)?;
                    s.SetPageHeight(11.69)?;
                    s.SetMarginTop(0.0)?;
                    s.SetMarginBottom(0.0)?;
                    s.SetMarginLeft(0.0)?;
                    s.SetMarginRight(0.0)?;
                    s.SetShouldPrintBackgrounds(true)?;
                    s.SetShouldPrintHeaderAndFooter(false)?;
                    Some(s)
                } else {
                    None
                };

                let output = HSTRING::from(output.as_os_str());
                let print_core = core.clone();
                let handler =
                    NavigationCompletedEventHandler::create(Box::new(move |_sender, _args| {
                        let mut source = PWSTR::null();
                        print_core.Source(&mut source)?;
                        let source = webview2_com::take_pwstr(source);
                        println!("NavigationCompleted: nguồn {source}");
                        if !source.starts_with("about:blank") {
                            return Ok(());
                        }
                        let exit_handle = handle.clone();
                        let done =
                            PrintToPdfCompletedHandler::create(Box::new(move |result, success| {
                                println!(
                                    "PrintToPdf: {result:?}, thành công {success}, {:.2}s",
                                    began.elapsed().as_secs_f32()
                                );
                                exit_handle.exit(if success { 0 } else { 1 });
                                Ok(())
                            }));
                        core7.PrintToPdf(&output, settings.as_ref(), &done)
                    }));
                let mut token = 0i64;
                core.add_NavigationCompleted(&handler, &mut token)?;
                core.NavigateToString(&HSTRING::from(html.as_str()))
            }
        })();
        if let Err(error) = outcome {
            eprintln!("LỖI WebView2: {error}");
            fail_handle.exit(1);
        }
    });
    if let Err(error) = scheduled {
        eprintln!("LỖI Tauri: {error}");
        std::process::exit(1);
    }
}
