# Sao Mai Ký số 1.5.0 (Rust + Tauri 2)

> **Người dùng:** giới thiệu phần mềm và cách sử dụng ở [GIOI_THIEU.md](GIOI_THIEU.md); tải bản chạy ở mục
> **Releases**.

Bản viết lại của ứng dụng Sao Mai Ký số: lõi Rust, vỏ Tauri 2, giao diện Angular 21.
Ứng dụng đã có Tạo văn bản, Ký số (USB Token, ký nháy / ký chính) và thông báo Telegram; xem tiến độ ở
[docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md).

- **Hướng dẫn chạy (cài môi trường, chạy thử, build, bài kiểm): [docs/HUONG_DAN_CHAY.md](docs/HUONG_DAN_CHAY.md)**
- Kế hoạch: [KE_HOACH_SAO_MAI_KY_SO_RUST.md](KE_HOACH_SAO_MAI_KY_SO_RUST.md)
- Trạng thái hiện tại: [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md)
- Bảng công việc: [docs/TASK_BOARD.md](docs/TASK_BOARD.md)
- Đối chiếu bản .NET cũ: [docs/MIGRATION_MAP.md](docs/MIGRATION_MAP.md)
- Thông báo Telegram và đăng ký thiết bị: [docs/THONG_BAO_TELEGRAM.md](docs/THONG_BAO_TELEGRAM.md)

## Cấu trúc

```
crates/smks-core     hằng số, mã lỗi, cấu hình config.json, lập kế hoạch dấu
crates/smks-crypto   ghim CA, kho chứng thư, khoá (CNG/CAPI), CMS, TSA, chuỗi tin cậy, lưu PIN (DPAPI)
crates/smks-pdf      đọc PDF, ghi nối bản, khối chữ ký
crates/smks-sign     ký một file, lô ký, xác minh
crates/smks-merge    tạo văn bản từ DOCX + Excel/CSV, xuất PDF qua Word, xem trước
crates/smks-cli      công cụ dòng lệnh kiểm thử lõi (ca, errors, certs, sign, verify, inspect, merge)
crates/smks-notify   sự kiện, đăng ký thiết bị, thông báo Telegram
crates/smks-gateway  Notification Gateway giữ Bot Token (máy chủ)
app/src-tauri        vỏ ứng dụng Tauri 2 (crate smks-app): cửa sổ, IPC, log
app/ui               giao diện Angular 21
app/tests            bài kiểm trên file chạy thật (xem docs/HUONG_DAN_CHAY.md mục 8)
spikes/              mã thử nghiệm giai đoạn 0, không phải code sản phẩm
certs/               5 chứng thư CA Ban Cơ yếu, ghim bằng SHA-256
fixtures/            file mẫu cho test, không chứa dữ liệu thật
docs/                tài liệu quản lý dự án, ADR, spike
```

`Kytl-main/` (bản .NET 1.4.0) và `App_Ky_So/` (prototype giao diện) là nguồn tham chiếu chỉ để đọc,
không nằm trong repo này.

## Yêu cầu máy phát triển

- Windows 10/11 x64.
- Rust theo `rust-toolchain.toml` (rustup tự cài).
- **Visual Studio có workload "Desktop development with C++" và Windows SDK.** Thiếu thành phần này thì không
  build được bất kỳ crate nào gọi Windows API, kể cả lõi ký số và Tauri.
- Node.js 22.12 trở lên, npm 10.
- WebView2 Runtime.
- Microsoft Word (cho Tạo văn bản), middleware token bit4id và SafeNet, USB token thật (cho ký số).

## Lệnh thường dùng

Chi tiết và các bài kiểm trên file chạy: [docs/HUONG_DAN_CHAY.md](docs/HUONG_DAN_CHAY.md).

```powershell
# Giao diện phải được build trước: crate smks-app nhúng thư mục dist lúc biên dịch.
cd app/ui; npm ci; npm run build; cd ../..

# Chạy thử khi phát triển (tự chạy ng serve, sửa giao diện tải lại ngay)
cd app; ui/node_modules/.bin/tauri dev; cd ..

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p smks-cli -- ca       # kiểm tra 5 chứng thư CA nhúng khớp giá trị ghim
cargo run -p smks-cli -- errors   # bảng mã lỗi

# Build file chạy (chưa đóng gói) và kiểm tra khói
cd app; ui/node_modules/.bin/tauri build --no-bundle; cd ..
node app/tests/smoke.mjs target/release/SaoMai-KySo.exe
```

## Quy ước

- `unsafe` chỉ nằm trong module `ffi` của từng crate, có chú thích `// SAFETY:`.
- Không `unwrap()`, `expect()`, `panic!` trong code sản phẩm (clippy chặn; test được miễn).
- Lỗi nghiệp vụ dùng `smks_core::SmksError` với mã lỗi và thông báo tiếng Việt.
- Nhánh `feat/…`, `fix/…`, `refactor/…`, `docs/…`, `build/…`; Conventional Commits, mô tả tiếng Việt.
- Không commit khoá bí mật (`*.pfx`, `*.p12`), PIN, file `.env`, bộ cài driver, `notify.json` / `gateway.env` có Bot Token thật.
