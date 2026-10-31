---
phase: 6
title: "License, thống kê, trạm, Trang chủ, tài liệu, phát hành"
status: pending
priority: P1
dependencies: [3, 4, 5]
---

# Giai đoạn 6: Tích hợp + phát hành

## Yêu cầu
- License: `ChucNang::BocTach` (mã `boc_tach`), dùng hạn mức chung; Thử đọc không trừ lượt.
- Trạm Apps Script: thêm `boc_tach` vào `TEN_CN`, hành động `thong_ke_boc_tach` (số file, trang, trường, lỗi, mô hình — KHÔNG gửi nội dung văn bản, KHÔNG gửi khoá); triển khai trạm TRƯỚC app (quy tắc dự án); test Node.
- Thống kê / Telegram: dòng tóm tắt như các chức năng khác; popup xong giữ câu "Thông tin bóc tách đã được ghi lại".
- Trang chủ: bỏ "Đang phát triển" ở ô Bóc tách; câu "Xử lý ngay trên máy · Riêng Bóc tách gửi ảnh tới mô hình đọc đã chọn" (như prototype).
- README: mục Bóc tách dữ liệu + ảnh; sửa câu thống kê (Bóc tách gửi ảnh trang tới mô hình đọc do người dùng chọn). Hướng dẫn sử dụng (F1): tab Bóc tách.
- Bảo mật: soát diff trước khi commit — không khoá API, không file khoá JSON, không IP thật, không email thật trong ảnh tài liệu.

## Các bước
1. License + trạm (deploy trạm) + test.
2. Thống kê / Telegram.
3. Trang chủ, README, Hướng dẫn, ảnh tài liệu (dữ liệu tổng hợp, đường dẫn trung tính).
4. Kiểm thử đầy đủ + build 0 cảnh báo; xin duyệt phát hành (tag `vX.Y.Z` chỉ khi chủ sản phẩm đồng ý).

## Tiêu chí xong
- [ ] Trạm đã triển khai trước, app cũ vẫn chạy với trạm mới.
- [ ] README / Hướng dẫn đúng hành vi; ảnh không lộ dữ liệu thật.
