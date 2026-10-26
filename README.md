# dema4

Đếm số trang A4 trong thư mục PDF: phân loại khổ giấy A0–A7, quy đổi ra số trang A4 theo hệ số, cảnh báo trang sát ngưỡng chuyển khổ và trang có DPI bất thường.

- `dema4_rs/` — bản Rust (giao diện egui + chế độ dòng lệnh). Đây là bản được build qua GitHub Actions.
- Các file `*.py` — bản gốc viết bằng Python (PyQt5 + PyMuPDF).

## Build bản Rust

Cần Rust (rustup). Trên Windows cần thêm Visual Studio Build Tools (C++) và LLVM.

```
cd dema4_rs
cargo build --release
```

File chạy: `dema4_rs/target/release/dema4_rs` (Windows: `dema4_rs.exe`).

GitHub Actions tự build cho macOS (arm64) và Windows (x64) mỗi khi push lên `main`.
Tạo tag `vX.Y.Z` để sinh Release kèm file chạy.

## Chế độ dòng lệnh

```
dema4_rs <thư mục>             # đếm, ghi <thư mục>/<tên>_dem.txt và _dem_chitiet.txt
dema4_rs <thư mục> --dpi       # kiểm tra DPI ảnh scan (chậm hơn)
dema4_rs <thư mục> --khong-ghi # chỉ in kết quả, không ghi file
```

## Gửi thống kê về Telegram (tuỳ chọn)

Đặt file `telegram.txt` cạnh file chạy:

```
token=<bot token>
chat_id=<id nhóm>
```

Không có file này thì app không gửi gì. File `telegram.txt` đã được loại khỏi repo (chứa token).
