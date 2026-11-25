//! Khai báo danh sách command của ứng dụng để Tauri chỉ cho giao diện gọi những command đã được cấp quyền
//! trong `capabilities/` (mỗi command sinh ra một quyền `allow-<tên>`). Thêm command mới thì thêm vào đây.

const COMMANDS: &[&str] = &[
    "app_info",
    "shell_open",
    "merge_inspect_template",
    "merge_read_table",
    "merge_auto_map",
    "merge_plan",
    "merge_preview",
    "merge_preview_template",
    "merge_run",
    "job_cancel",
    "cert_list",
    "token_check",
    "token_forget",
    "pin_saved",
    "pin_forget",
    "driver_status",
    "open_link",
    "sign_list_files",
    "sign_preview",
    "sign_image_preview",
    "sign_positions",
    "sign_layouts",
    "sign_plan_outputs",
    "config_get",
    "config_defaults",
    "config_save",
    "sign_run",
    "verify_file",
    "report_export_pdf",
    "notify_status",
    "notify_consent",
    "notify_unregister",
    "notify_set_prefs",
    "notify_check",
    "notify_test",
];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(error) = tauri_build::try_build(attributes) {
        // cargo in stderr của build script khi nó thất bại.
        eprintln!("tauri-build lỗi: {error:?}");
        std::process::exit(1);
    }
}
