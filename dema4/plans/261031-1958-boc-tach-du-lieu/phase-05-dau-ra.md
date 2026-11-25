---
phase: 5
title: "Đầu ra: Excel + Google Sheet"
status: completed
priority: P1
dependencies: [1]
---

# Giai đoạn 5: Excel + Google Sheet

## Yêu cầu
- Excel `<thư mục nguồn>_boc_tach.xlsx` (hoặc thư mục trong Cấu hình): dòng 1 = tên cột (Tên file, Số trang, các trường theo thứ tự loại tài liệu), mỗi file 1 dòng; ô Số ký hiệu / Ngày tháng định dạng văn bản (`@`) để Excel không đổi thành ngày; NFC.
- Tạo Excel khi bóc xong (và khi Lưu sửa tay); file đang mở trong Excel → ghi bản có hậu tố thời gian, báo trong popup xong.
- Tên cột trùng biến Sinh mục lục → file nạp thẳng làm file biên mục (test bằng `muc_luc::doc_bang`).
- Google Sheet ghi thêm 1 dòng sau mỗi file:
  - Service account: JWT RS256 (`rsa` + `sha2` đã có) → token OAuth → Sheets API `values:append`; tạo tab + dòng tiêu đề nếu chưa có.
  - Apps Script: POST JSON tới URL web app; cung cấp script mẫu (nút "Chép script mẫu") — đặt ở `apps-script-mau/boc_tach.gs`, KHÔNG trong `apps-script/` (thư mục trạm license).
  - Lỗi mạng: xếp hàng gửi lại, không chặn bóc tách; báo số dòng chưa ghi.
- "Ghi thử 1 dòng" trong Cấu hình.

## Kiến trúc
- `src/boc_tach/excel.rs` — thêm crate ghi xlsx (`rust_xlsxwriter`, MIT/Apache) — kiểm tra kích thước nhị phân.
- `src/boc_tach/sheet.rs` — JWT, append, hàng đợi gửi lại.
- Khoá service account chỉ đọc từ file người dùng chọn; không chép vào repo / log.

## Các bước
1. Excel + test đọc lại bằng calamine + test nạp vào Sinh mục lục.
2. JWT + append (test với máy chủ giả; thử tay với Sheet thật ngoài CI).
3. Apps Script + script mẫu trong `apps-script/boc_tach.gs` (có test Node như `loi.js` nếu có logic).
4. Hàng đợi gửi lại khi lỗi.

## Tiêu chí xong
- [ ] Excel đúng định dạng; nạp Sinh mục lục tự ghép biến.
- [ ] Sheet nhận đủ dòng sau mất mạng tạm thời.
