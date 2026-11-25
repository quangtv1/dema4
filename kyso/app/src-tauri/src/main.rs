//! Điểm vào của ứng dụng Sao Mai Ký số.

// Bản phát hành không mở cửa sổ console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod logging;

use tauri::Manager;

use commands::merge::MergeState;
use commands::notify::NotifyState;
use commands::sign::SignState;

fn main() {
    // Giữ `_log_guard` tới hết chương trình để log kịp ghi xuống đĩa.
    let _log_guard = logging::init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "khởi động Sao Mai Ký số"
    );

    // Chứng thư CA nhúng phải khớp giá trị ghim; không khớp thì không chạy tiếp.
    if let Err(error) = smks_crypto::trust::verify_pinned_cas() {
        tracing::error!(code = error.code.as_u32(), "{}", error.message);
        std::process::exit(2);
    }

    let locale = smks_merge::word::user_number_locale();
    tracing::info!(decimal = %locale.decimal, group = %locale.group, "định dạng số theo cài đặt Windows");

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Mở lần hai thì đưa cửa sổ đang có lên trước (SH-02).
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(MergeState::new(locale))
        .manage(SignState::default())
        // Ghi nhận sự kiện và thông báo Telegram (smks-notify): chạy nền, không có notify.json thì không gửi gì.
        .manage(NotifyState::start())
        .setup(|app| {
            // Hộp PIN của middleware token hiện trên cửa sổ chính, không bị chìm (như bản cũ).
            if let Some(window) = app.get_webview_window("main")
                && let Ok(hwnd) = window.hwnd()
            {
                smks_crypto::store::set_window_handle(hwnd.0 as isize);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::shell_open,
            commands::merge::merge_inspect_template,
            commands::merge::merge_read_table,
            commands::merge::merge_auto_map,
            commands::merge::merge_plan,
            commands::merge::merge_preview,
            commands::merge::merge_preview_template,
            commands::merge::merge_run,
            commands::merge::job_cancel,
            commands::sign::cert_list,
            commands::sign::token_check,
            commands::sign::token_forget,
            commands::pin::pin_saved,
            commands::pin::pin_forget,
            commands::driver::driver_status,
            commands::driver::open_link,
            commands::sign::sign_list_files,
            commands::sign::sign_preview,
            commands::sign::sign_image_preview,
            commands::sign::sign_positions,
            commands::sign::sign_layouts,
            commands::sign::sign_plan_outputs,
            commands::config::config_get,
            commands::config::config_defaults,
            commands::config::config_save,
            commands::sign::sign_run,
            commands::sign::verify_file,
            commands::report::report_export_pdf,
            commands::notify::notify_status,
            commands::notify::notify_consent,
            commands::notify::notify_unregister,
            commands::notify::notify_set_prefs,
            commands::notify::notify_check,
            commands::notify::notify_test,
        ])
        .build(tauri::generate_context!());

    match result {
        Ok(app) => app.run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Đóng phiên Word giữ sẵn của xem trước, rồi buộc đóng phiên Word còn sót (đang tạo lô dở).
                app.state::<MergeState>().shutdown();
                smks_merge::word::kill_all_sessions();
                tracing::info!("thoát ứng dụng");
            }
        }),
        Err(error) => {
            tracing::error!("ứng dụng dừng vì lỗi: {error}");
            std::process::exit(1);
        }
    }
}
