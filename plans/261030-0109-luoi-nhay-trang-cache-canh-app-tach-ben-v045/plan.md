---
status: completed
---
# v0.4.5 — Lưới nhảy trang, cache cạnh app, đổi tên app, Tách bền hơn

## Việc đã làm
1. Phân tích: về tab Hồ sơ / lưới Tất cả; mỗi hồ sơ xong chọn + cuộn tới trang cuối; xong hết chọn trang cuối lưới.
2. Dòng thông tin: `(i) Số hồ sơ; Số văn bản; Tổng số trang; Số trang (Quy A4)`; thanh trạng thái `Trong lưới · Đã xoá · Đã chọn`.
3. Cột tên hồ sơ kèm đuôi gốc `.pdf/.PDF`; cột văn bản `.pdf`.
4. Dữ liệu ẩn (nhãn, xoay, log, cache) chuyển sang `<thư mục app>/.sm-dema4/<tên>-<băm>/` (`du_lieu_an.rs`), mỗi thư mục đầu vào 1 chỗ riêng, tự chuyển bản cũ; Windows đặt Hidden.
5. File chạy `dema4_saomai` / `dema4_saomai.exe`; release không hậu tố nền tảng.
6. Tách: chặn PDF mật khẩu (phân tích + tách), ghi qua `.tmp` rồi đổi tên, `catch_unwind` từng hồ sơ, lỗi/cảnh báo vào log, hồ sơ 0 trang cảnh báo, hồ sơ không mở được hiện đỏ + trạng thái, hỏi khi đóng app đang xử lý, thông báo lỗi dễ hiểu.

## Kiểm thử
- 139 test; báo cáo kiểm thử Tách: `plans/reports/kiem-thu-tach-van-ban-261030-0100-loi-treo-crash-report.md`.
