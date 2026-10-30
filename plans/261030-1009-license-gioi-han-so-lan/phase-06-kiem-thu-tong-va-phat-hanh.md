---
phase: 6
title: Kiem thu tong va phat hanh
status: completed
priority: P1
dependencies:
  - 3
  - 4
  - 5
effort: 1 ngày
---

# Phase 6: Kiểm thử tổng và phát hành v1.0.0

## Overview
Chạy thật với trạm Apps Script trên Mac + Windows theo ma trận tình huống; cập nhật tài liệu; phát hành v0.6.0; sau khi đã thu hồi bí mật cũ (phase 5 bước 0).

## Requirements
- Mọi tiêu chí trong `plan.md` đạt trên máy thật.
- Tài liệu giải thích cho người dùng và cho chủ Sheet.

## Ma trận kiểm thử tay
| # | Tình huống | Mong đợi |
|---|---|---|
| 1 | Máy mới, có mạng, Đếm 3 lần | Lần 1–2 chạy; Sheet có dòng máy, Đã dùng Đếm = 2; lần 3 popup |
| 2 | Tương tự với Tách | Như trên |
| 3 | Chủ Sheet đặt Hạn mức Đếm = 5 → Kiểm tra lại | Popup đóng, Đếm chạy |
| 4 | Chủ Sheet đặt Khoá | Chạy được tối đa 1 lần nữa rồi chặn; mở lại app → chặn ngay; rút mạng → vẫn chặn |
| 5 | Máy mới, rút mạng, Đếm 3 lần | 2 lần chạy, lần 3 popup offline |
| 6 | Cắm mạng, mở lại app | Nhật ký thêm 2 dòng "bù offline", không trùng |
| 7 | Tách gần xong → Thoát luôn | Lượt vẫn bị tính (lần mở sau ghi lên Sheet) |
| 8 | Bấm Dừng sau 1 file / trước file nào | Có kết quả → tính; chưa có → không tính |
| 9 | Mạng chập chờn khi bấm | Bắt đầu chạy ngay, không đứng; không bị đếm trùng |
| 10 | Đổi tên máy | Mã máy không đổi, vẫn dòng cũ |
| 11 | CLI `--tach` / `--khong-ghi` khi hết lượt | In lý do + mã máy, mã thoát 3 |
| 12 | Gửi câu trả lời giả qua proxy | App từ chối, dùng câu trả lời ký cũ / offline |
| 13 | Tên thư mục `=IMPORTXML(...)` / `<a href=…>` | Ô Sheet là chữ thuần; tin Telegram hiển thị nguyên văn, không có link |
| 14 | Đếm / Tách xong | Tin + CSV tới nhóm, hàng Ngưỡng trắng (qua trạm) |
| 15 | Đổi `telegram_chat_id` | Tin kế tiếp tới nhóm mới, không cài lại app |
| 16 | 30 tin trong 1 phút | Tới đủ trong ≤ 3 phút, không trùng |
| 17 | App mở 1 giờ không thao tác | Nhật ký thực thi trạm: chỉ lời gọi lúc mở app |
| 18 | Đang mở app, chuyển Wi-Fi cơ quan → 4G rồi Đếm | Dòng license và dòng nhật ký lần đó có IP mới |

## Related Code Files
- Modify: `README.md` (mục License: cách hoạt động, mã máy, liên hệ; mục Gửi thống kê thêm mã máy), `dema4_rs/src/tro_giup.rs` (Hướng dẫn: mục License), `dema4_rs/Cargo.toml` (0.6.0)
- Create: `plans/261030-1009-license-gioi-han-so-lan/reports/kiem-thu-license-report.md`

## Implementation Steps
1. Chạy `cargo test --release` + `node --test dema4_rs/apps-script/test` (xanh) + test `#[ignore]` `kho_tram_that`.
2. Chạy ma trận trên Mac (build local) và Windows (bản CI tag rc).
3. Ghi kết quả vào báo cáo kiểm thử; sửa lỗi phát sinh (test trước rồi sửa).
4. Cập nhật README (mục License, Gửi thống kê: qua trạm, mã máy) + Hướng dẫn; bump 0.6.0; tag `v0.6.0`.
5. Xác nhận phase 5 bước 0 đã làm (token mới, khoá service account đã vô hiệu).

## Success Criteria
- [ ] 18 tình huống đạt (1–13, 18 trên cả Mac và Windows; 14–17 một lần).
- [ ] Release v0.6.0 có 2 file; service account cũ đã thu hồi.
- [ ] Báo cáo kiểm thử lưu trong plan.

## Risk Assessment
- Bản cũ ≤ v0.5.x: không còn gửi thống kê; vẫn dùng được không giới hạn (đã chấp nhận).
- Máy dev bị tính lượt khi test → cấp "Đã cấp" cho mã máy dev trước khi chạy ma trận (trừ tình huống 1–2 dùng máy / mã thử).
