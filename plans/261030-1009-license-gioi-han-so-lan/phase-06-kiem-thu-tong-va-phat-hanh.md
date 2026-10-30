---
phase: 6
title: "Kiem thu tong va phat hanh"
status: pending
priority: P1
dependencies: [3, 4, 5]
effort: "1 ngày"
---

# Phase 6: Kiểm thử tổng và phát hành v0.6.0

## Overview
Chạy thật với trạm Apps Script trên Mac + Windows theo ma trận tình huống; cập nhật tài liệu; phát hành v0.6.0; sau đó thu hồi service account cũ, đổi token bot (phase 5 bước 8–10).

## Requirements
- Mọi tiêu chí trong `plan.md` đạt trên máy thật.
- Tài liệu giải thích cho người dùng và cho chủ Sheet.

## Ma trận kiểm thử tay
| # | Tình huống | Mong đợi |
|---|---|---|
| 1 | Máy mới, có mạng, Đếm 3 lần | Lần 1–2 chạy; Sheet có dòng máy, G = 2; lần 3 popup Hết lượt Đếm |
| 2 | Tương tự với Tách | Như trên, H = 2 |
| 3 | Chủ Sheet đặt J = 5 → Kiểm tra lại | Popup đóng, Đếm chạy |
| 4 | Chủ Sheet đặt L = Khoá | Chạy được tối đa 1 lần nữa rồi chặn; mở lại app / Kiểm tra lại → chặn ngay |
| 5 | Tắt mạng, Đếm 3 lần | 2 lần chạy, lần 3 popup offline |
| 6 | Bật mạng, mở lại app | Nhật ký thêm 2 dòng "bù offline", bộ đếm offline về 0 |
| 7 | Xoá 1 file đếm offline | Vẫn tính 2 (lấy nơi còn lại) |
| 8 | Mạng chập chờn / rút cáp khi bấm | Đếm / Tách bắt đầu ngay, giao diện không đứng |
| 9 | CLI `--tach` khi hết lượt | In thông báo + mã máy, mã thoát 3 |
| 10 | Gửi câu trả lời giả (sửa `du_lieu` / chữ ký máy khác) vào app qua proxy | App từ chối, dùng luật offline |
| 11 | `strings` exe tìm khoá | Không có `BEGIN PRIVATE KEY` / `private_key` |
| 12 | Tách xong | Hàng mới ở tab Ngưỡng trắng + tin + CSV ở nhóm Telegram (qua trạm) |
| 13 | Đổi `telegram_chat_id` trên tab Cấu hình sang nhóm thử | Tin kế tiếp tới nhóm thử, không cần cài lại app |
| 14 | `strings` exe tìm token bot | Không có |
| 15 | Script đẩy 30 lời gọi `thong_ke_dem` (mã thử khác nhau) trong 1 phút | Cả 30 tin tới nhóm trong ≤ 3 phút, không mất tin |
| 16 | 20 máy (luồng giả lập) cùng `ghi_lan` trong 5 giây | Không lỗi mất lượt; lời gọi lỗi 503 được ghi bù sau |
| 17 | Để app mở 1 giờ không thao tác | Nhật ký thực thi Apps Script: chỉ 1 lời gọi lúc mở app |

## Related Code Files
- Modify: `README.md` (mục License: cách hoạt động, mã máy, liên hệ; mục Gửi thống kê thêm mã máy), `dema4_rs/src/tro_giup.rs` (Hướng dẫn: mục License), `dema4_rs/Cargo.toml` (0.6.0)
- Create: `plans/261030-1009-license-gioi-han-so-lan/reports/kiem-thu-license-report.md`

## Implementation Steps
1. Chạy `cargo test --release` (xanh) + test `#[ignore]` `kho_tram_that`.
2. Chạy ma trận trên Mac (build local) và Windows (bản CI tag rc).
3. Ghi kết quả vào báo cáo kiểm thử; sửa lỗi phát sinh (test trước rồi sửa).
4. Cập nhật README (mục License, Gửi thống kê: qua trạm, mã máy) + Hướng dẫn; bump 0.6.0; tag `v0.6.0`.
5. Sau khi phát hành: thu hồi service account cũ, đổi token bot, chia sẻ lại spreadsheet (phase 5 bước 8–10).

## Success Criteria
- [ ] 17 tình huống đạt (1–14 trên cả Mac và Windows; 15–17 một lần).
- [ ] Release v0.6.0 có 2 file; service account cũ đã thu hồi.
- [ ] Báo cáo kiểm thử lưu trong plan.

## Risk Assessment
- Bản cũ ≤ v0.5.x mất ghi thống kê sau khi thu hồi service account (chấp nhận; nhắc cập nhật).
- Máy dev bị tính lượt khi test → cấp "Đã cấp" cho mã máy dev trước khi chạy ma trận (trừ tình huống 1–2 dùng máy / mã thử).
