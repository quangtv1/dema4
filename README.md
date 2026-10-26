# dema4

Đếm số trang A4 trong thư mục PDF: phân loại khổ giấy A0–A7, quy đổi ra số trang A4 theo hệ số, cảnh báo trang sát ngưỡng chuyển khổ và trang có DPI bất thường.

- `dema4_rs/` — bản Rust (giao diện egui + chế độ dòng lệnh). Đây là bản được build qua GitHub Actions.

Bản gốc viết bằng Python (PyQt5 + PyMuPDF) được giữ cục bộ trong thư mục `temp/`, không đưa lên repo.

## Build bản Rust

Cần Rust (rustup). Trên Windows cần thêm Visual Studio Build Tools (C++) và LLVM.

```
cd dema4_rs
cargo build --release
```

File chạy: `dema4_rs/target/release/sm-dema4` (Windows: `sm-dema4.exe`).

GitHub Actions tự build cho macOS (arm64) và Windows (x64) mỗi khi push lên `main`.
Tạo tag `vX.Y.Z` để sinh Release kèm file chạy.

## Chế độ dòng lệnh

```
sm-dema4 <thư mục>             # đếm, ghi <thư mục>/<tên>_dem.txt và _dem_chitiet.txt
sm-dema4 <thư mục> --dpi       # kiểm tra DPI ảnh scan (chậm hơn)
sm-dema4 <thư mục> --khong-ghi # chỉ in kết quả, không ghi file
```

## Gửi thống kê về Telegram

Token và id nhóm được nhúng sẵn trong file chạy (xem `dema4_rs/src/telegram.rs`), nên exe gửi thống kê ngay, không cần file cấu hình.

Có thể ghi đè bằng file `telegram.txt` cạnh file chạy (ưu tiên hơn giá trị nhúng) để trỏ sang nhóm khác mà không build lại:

```
token=<bot token>
chat_id=<id nhóm>
```
