# ADR 0010: Toolchain Rust và target build

- **Trạng thái:** Đã chấp nhận và đã thực hiện, 18/11/2026. Người phụ trách đồng ý cài workload C++ (Q-01); máy
  phát triển hiện có MSVC 14.51.36231 và Windows SDK 10.0.26100.0. Mọi lệnh cargo chạy bằng toolchain MSVC
  theo `rust-toolchain.toml`, không còn dùng biến `RUSTUP_TOOLCHAIN`.
- **Bối cảnh:** Khảo sát ngày 18/11/2026 cho thấy máy phát triển có ba toolchain 1.100.0 (msvc, gnu,
  gnullvm) nhưng không toolchain nào link được code gọi Windows API:
  - MSVC: Visual Studio 2026 Community đã cài nhưng không có workload C++ và Windows SDK (`link.exe` không có).
  - GNU: `dlltool.exe` đi kèm rustup báo `CreateProcess` vì thiếu `as.exe`; crate dùng `raw-dylib`
    (`windows-sys`, `windows`) không build được.
  - gnullvm: thiếu `x86_64-w64-mingw32-clang`.
- **Quyết định đề xuất:**
  1. Bản phát hành và CI build bằng `x86_64-pc-windows-msvc`.
  2. `rust-toolchain.toml` ghim `channel = "1.100.0"`, không ghi host triple.
  3. Máy phát triển cài "Desktop development with C++" và Windows SDK.
  4. Cho tới khi cài xong, các crate thuần Rust được kiểm bằng
     `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu` (cùng phiên bản), đặt theo từng lệnh.
- **Hệ quả:** Trước khi có (3) không làm được: `smks-crypto::{store,key,chain}`, Word COM, WebView2
  PrintToPdf, khung Tauri, spike S1, S5, S6, S7; và cả những phần tưởng là thuần Rust nhưng kéo `getrandom`
  hoặc `windows-sys`: đọc Excel (`calamine`), DOCX (`zip`), `lopdf`, ký và xác minh bằng `rsa`/`p256`. Bảng
  crate đã kiểm ở cuối `docs/RISK_REGISTER.md`. Vẫn làm được: khối chữ ký (`pdf-writer`, `ttf-parser`,
  `subsetter`), bộ ghi nối bản tự viết, cấu trúc ASN.1 của CMS và TSP.
- **Lưu ý:** Kết quả test trên GNU không thay cho test trên MSVC; mọi mục DONE kiểm bằng GNU phải được chạy
  lại trên MSVC khi B-01 được gỡ.
