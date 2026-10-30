// Gắn icon.ico của dự án vào file exe khi build cho Windows.
fn main() {
    println!("cargo:rerun-if-changed=../icon.ico");
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
