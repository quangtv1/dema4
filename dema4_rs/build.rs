// Gắn icon.ico của dự án vào file exe khi build cho Windows; nhúng khoá service account Google Sheet
// (biến môi trường SM_DEMA4_SHEET_KEY_JSON trên CI, hoặc file bi_mat/sheet_key.json khi build trên máy).
// Không có khoá → nhúng chuỗi rỗng, app bỏ qua việc ghi Sheet.
fn main() {
    println!("cargo:rerun-if-changed=../icon.ico");
    println!("cargo:rerun-if-env-changed=SM_DEMA4_SHEET_KEY_JSON");
    println!("cargo:rerun-if-changed=bi_mat/sheet_key.json");
    let khoa = std::env::var("SM_DEMA4_SHEET_KEY_JSON").ok().filter(|s| !s.trim().is_empty()).or_else(|| std::fs::read_to_string("bi_mat/sheet_key.json").ok()).unwrap_or_default();
    let ra = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("sheet_key.json");
    std::fs::write(ra, khoa).expect("không ghi được khoá Sheet vào OUT_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // ONNX Runtime dựng sẵn cho Windows luôn kèm DirectML (GPU) → exe đòi DirectML/D3D12/DXCore ngay lúc mở.
        // App chỉ chạy CPU: nạp trễ để máy thiếu các DLL này (Windows 10 cũ) vẫn mở được app.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            for dll in ["DirectML.dll", "d3d12.dll", "dxcore.dll"] {
                println!("cargo:rustc-link-arg-bins=/DELAYLOAD:{dll}");
            }
            println!("cargo:rustc-link-arg-bins=delayimp.lib");
        }
        winresource::WindowsResource::new()
            .set_icon("../icon.ico")
            .compile()
            .expect("không gắn được icon cho exe");
    }
}
