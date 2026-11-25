//! Liên kết tĩnh thư viện chạy của Visual C++ (như `tauri-build` làm cho ứng dụng), để `smks-gateway.exe` chạy được
//! trên máy chủ Windows chưa cài Visual C++ Redistributable.

fn main() {
    static_vcruntime::metabuild();
}
