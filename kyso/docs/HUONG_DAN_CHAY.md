# Hướng dẫn chạy Sao Mai Ký số 1.5.0

Cách cài môi trường, chạy thử khi phát triển, build file chạy, chạy bài kiểm và dựng Notification Gateway.
Mọi lệnh chạy trong PowerShell, đứng ở thư mục gốc repo (`sao_mai_ky_so`), trừ khi ghi khác.

## 1. Chuẩn bị máy

| Thành phần | Ghi chú |
|---|---|
| Windows 10/11 x64 | |
| Rust | Cài [rustup](https://rustup.rs); phiên bản đúng tự cài theo `rust-toolchain.toml` (1.100.0, MSVC) |
| Visual Studio 2022 Build Tools | Workload **Desktop development with C++** và **Windows SDK**. Thiếu thì không build được crate nào |
| Node.js 22.12 trở lên, npm 10 | Cho giao diện Angular và các bài kiểm `app/tests/*.mjs` |
| WebView2 Runtime | Windows 11 có sẵn; Windows 10 cài từ Microsoft |
| Microsoft Word | Cho Tạo văn bản (xuất PDF, xem trước). Không có Word thì chỉ xuất DOCX |
| Middleware USB Token | bit4id hoặc SafeNet, cùng token thật, để ký thật. Không có token thì dùng bản `dev-keys` (mục 4) |
| Python 3 + `pymupdf`, `pyhanko`, `openpyxl` | Chỉ cần để chạy một số bài kiểm (đọc PDF đã ký, sửa file Excel) |

Kiểm tra nhanh:

```powershell
rustc --version; cargo --version; node --version; npm --version
```

## 2. Lần đầu sau khi lấy mã nguồn

```powershell
cd app/ui
npm ci
npm run build      # crate smks-app nhúng thư mục dist lúc biên dịch: phải build giao diện trước
cd ../..
cargo build --workspace
```

## 3. Chạy khi phát triển

```powershell
cd app
ui/node_modules/.bin/tauri dev
```

Lệnh này tự chạy `ng serve` (giao diện ở `http://localhost:4200`, sửa giao diện là tải lại ngay) và mở
`target/debug/smks-app.exe`. Sửa code Rust thì Tauri tự build lại.

Lưu ý: app **chỉ chạy một bản**. Khi đang `tauri dev`, mở bản phát hành sẽ chỉ đưa cửa sổ đang mở lên trước. Tắt
`smks-app.exe` trước khi chạy bản phát hành hoặc bài kiểm.

## 4. Build file chạy

### Bản phát hành (giao cho người dùng)

```powershell
cd app
ui/node_modules/.bin/tauri build --no-bundle
cd ..
.\target\release\SaoMai-KySo.exe
```

`tauri build` tự chạy `npm run build` trước. Kết quả: `target\release\SaoMai-KySo.exe` (khoảng 15 MB, không cần cài
gì thêm ngoài WebView2).

### Bản `dev-keys` (chỉ để thử, không giao cho người dùng)

Ký bằng khoá phần mềm thử thay cho USB token, để thử màn Ký số trên máy không có token:

```powershell
cd app
ui/node_modules/.bin/tauri build --no-bundle --features dev-keys
cd ..
$env:SMKS_DEV_PKI = "$PWD\spikes\s2-cms\out\pki"
.\target\release\SaoMai-KySo.exe
```

Trong hộp Xác nhận ký sẽ có thêm chứng thư "Khoá phần mềm". Chữ ký ra có kết quả kiểm "người ký không thuộc CA Ban Cơ
yếu" (đúng, vì là CA thử). Bản này và bản phát hành cùng ghi ra `target\release\SaoMai-KySo.exe`: build lại bản
phát hành trước khi giao.

### Notification Gateway (máy chủ thông báo Telegram)

```powershell
cargo build -p smks-gateway --release
# → target\release\smks-gateway.exe
```

## 5. Biến môi trường

| Biến | Dùng cho |
|---|---|
| `SMKS_CONFIG_DIR` | Thư mục chứa `config.json` thay cho `%APPDATA%\SaoMai-KySo` (bài kiểm dùng để không đụng cấu hình thật) |
| `SMKS_DEV_PKI` | Thư mục khoá thử (`signer.cert.der`, `signer.key.pk8.der`, `ca.cert.der`); chỉ bản `dev-keys` |
| `SMKS_DEV_NO_DRIVER` | `1`: giả lập máy chưa cài driver USB Token; chỉ bản `dev-keys` |
| `SMKS_NOTIFY_CONFIG` | Đường dẫn `notify.json` (cấu hình thông báo Telegram) thay cho chỗ mặc định |
| `TELEGRAM_API_BASE` | Địa chỉ Bot API khác `https://api.telegram.org` (bài kiểm dùng máy chủ giả) |

## 6. Dữ liệu và log của ứng dụng

| Nơi | Nội dung |
|---|---|
| `%APPDATA%\SaoMai-KySo\config.json` | Cấu hình (mẫu chữ ký, vị trí khung, nơi lưu, lượt ký Chính / Nháy). Phiên bản 3; bản cũ tự nâng cấp. File hỏng được đổi tên thành `config.json.hong-<giờ>` |
| `%APPDATA%\SaoMai-KySo\pin.bin` | PIN đã lưu (mã hoá DPAPI), chỉ khi người dùng bật "Lưu PIN" |
| `%APPDATA%\SaoMai-KySo\notify-*.json`, `nhat-ky-su-kien\` | Trạng thái, hàng đợi và nhật ký thông báo Telegram |
| `%LOCALAPPDATA%\SaoMai-KySo\logs\` | Log, mỗi ngày một file, giữ 30 ngày |
| `notify.json` cạnh file chạy hoặc `%PROGRAMDATA%\SaoMai-KySo\notify.json` | Cấu hình thông báo do quản trị viên đặt; không có thì tính năng thông báo tắt hẳn |

## 7. Kiểm tra trước khi giao

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p smks-app --all-targets --features dev-keys -- -D warnings
cargo test --workspace
cargo run -p smks-cli -- ca        # 5 chứng thư CA nhúng khớp giá trị ghim
```

Test cần Word hoặc mạng (TSA thật) bị bỏ qua mặc định; chạy tay bằng `cargo test -p <crate> -- --ignored`.

## 8. Bài kiểm trên file chạy thật (`app/tests`)

Các bài này mở `SaoMai-KySo.exe`, điều khiển giao diện qua cổng gỡ lỗi của WebView2 rồi in `KET QUA: DAT` hoặc
`KET QUA: KHONG DAT`. Trước khi chạy: tắt mọi bản app đang mở (kể cả `tauri dev`). Thư mục làm việc nên để trong
`%TEMP%` (bài kiểm xoá và tạo lại thư mục này).

Đặt biến cho gọn:

```powershell
$E   = "target\release\SaoMai-KySo.exe"
$PKI = "spikes\s2-cms\out\pki"
$W   = "$env:TEMP\smks-kiem"
$IMG = "$PWD\Hai_Chu_Ky.png"
```

**Bản phát hành:**

```powershell
node app/tests/smoke.mjs $E
node app/tests/tvb-ui.mjs $E . "$W\tvb-ui"
node app/tests/tvb-ipc.mjs $E . "$W\tvb-ipc"
node app/tests/tvb-sua-mau.mjs $E . "$W\tvb-sua-mau"     # cần Word, Python + openpyxl; ghi tạm vào bộ nhớ tạm (clipboard)
```

**Bản `dev-keys`** (build theo mục 4):

```powershell
node app/tests/ks-ipc.mjs     $E . "$W\ks-ipc"     $PKI
node app/tests/ks-sigbox.mjs  $E . "$W\ks-sigbox"  $PKI $IMG
node app/tests/ks-config.mjs  $E . "$W\ks-config"  $PKI $IMG
node app/tests/ks-vitri.mjs   $E . "$W\ks-vitri"   $PKI $IMG
node app/tests/ks-trung.mjs   $E . "$W\ks-trung"   $PKI      # thư mục làm việc phải ở ổ cố định (dùng Thùng rác)
node app/tests/ks-pin.mjs     $E . "$W\ks-pin"     $PKI
node app/tests/ks-tamdung.mjs $E . "$W\ks-tamdung" $PKI
node app/tests/ks-nhay.mjs    $E . "$W\ks-nhay"    $PKI      # ký nháy / chính, thêm / xoá khung, ảnh thu nhỏ trang
node app/tests/ks-bocuc.mjs   $E . "$W\ks-bocuc"   $PKI $IMG # vị trí ảnh so với chữ (trái / phải / trên / dưới)
node app/tests/ht-ui.mjs      $E "$W\ht-ui"
node app/tests/tg-notify.mjs  $E target\release\smks-gateway.exe . "$W\tg" $PKI
```

`tg-notify.mjs` thêm `--real` để gửi tin thật tới Telegram, với token và Chat ID trong biến `TG_REAL_TOKEN`,
`TG_REAL_CHAT` (không ghi vào file nào).

**Cần USB token thật và người thao tác** (rút / cắm token, nhập PIN):

```powershell
cargo build -p smks-cli --release
node app/tests/ks-token-ui.mjs $E target\release\smks-cli.exe . "$W\ks-token" 200
```

## 9. Thông báo Telegram

Hướng dẫn đầy đủ (tạo bot, `notify.json`, dựng gateway, thu hồi thiết bị): [THONG_BAO_TELEGRAM.md](THONG_BAO_TELEGRAM.md).
Tóm tắt:

- Máy trạm: chép `installer\notify.gateway.example.json` (qua gateway, nên dùng) hoặc `installer\notify.direct.example.json`
  thành `notify.json` cạnh `SaoMai-KySo.exe`, điền giá trị thật. Không commit file này.
- Gateway: chép `installer\gateway.env.example` thành `gateway.env`, điền `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`,
  `GATEWAY_ENROLL_KEY`, rồi:

```powershell
.\target\release\smks-gateway.exe --env-file gateway.env check     # thử token và nhóm
.\target\release\smks-gateway.exe --env-file gateway.env serve     # nghe ở GATEWAY_BIND, mặc định 127.0.0.1:8787
.\target\release\smks-gateway.exe --env-file gateway.env devices   # danh sách thiết bị
.\target\release\smks-gateway.exe --env-file gateway.env revoke <mã thiết bị>
```

## 10. Gặp lỗi thường gặp

| Hiện tượng | Cách xử lý |
|---|---|
| `link.exe not found`, thiếu `windows.h` / Windows SDK | Cài Visual Studio Build Tools, workload C++ và Windows SDK (mục 1) |
| Build Rust báo không thấy thư mục `dist` của giao diện | Chạy `npm run build` trong `app/ui` trước (mục 2) |
| Bài kiểm báo "không kết nối được WebView2" hoặc "không thấy trang" | Đang có bản app khác mở (thường là `tauri dev`): tắt `smks-app.exe`, `SaoMai-KySo.exe` rồi chạy lại |
| Bài kiểm `ks-*` không thấy "Khoá phần mềm" | Đang dùng bản phát hành: build lại với `--features dev-keys` và đặt `SMKS_DEV_PKI` |
| Xem trước Tạo văn bản báo không có Word | Cài Microsoft Word, hoặc chọn định dạng DOCX ở Nâng cao |
| Sau khi tắt app còn `WINWORD.EXE` chạy ẩn | Thường do app bị buộc dừng (một số bài kiểm cũ dùng `app.kill()`). Đóng các tiến trình `WINWORD.EXE` có dòng lệnh `/Automation -Embedding` mà không có cửa sổ |
| Build cảnh báo "bundle initial exceeded maximum budget" | Chỉ là cảnh báo dung lượng gói giao diện, không chặn build |
