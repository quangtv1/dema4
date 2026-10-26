// Gắn icon.ico của dự án vào file exe khi build cho Windows
fn main() {
    println!("cargo:rerun-if-changed=../icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("../icon.ico")
            .compile()
            .expect("không gắn được icon cho exe");
    }
}
