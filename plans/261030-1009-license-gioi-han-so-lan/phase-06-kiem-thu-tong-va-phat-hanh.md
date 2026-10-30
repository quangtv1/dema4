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
Chạy thật với Sheet đã khoá vùng trên Mac + Windows, theo ma trận tình huống; cập nhật tài liệu; phát hành v0.6.0 qua repo phát hành.

## Requirements
- Mọi tiêu chí trong `plan.md` đạt trên máy thật.
- Tài liệu giải thích cho người dùng và cho chủ Sheet.

## Ma trận kiểm thử tay
| # | Tình huống | Mong đợi |
|---|---|---|
| 1 | Máy mới, có mạng, Đếm 3 lần | Lần 1–2 chạy; Sheet có dòng máy, G = 2; lần 3 popup Hết lượt Đếm |
| 2 | Tương tự với Tách | Như trên, H = 2 |
| 3 | Chủ Sheet đặt J = 5 → Kiểm tra lại | Popup đóng, Đếm chạy |
| 4 | Chủ Sheet đặt L = Khoá | Chặn trong ≤ 5 phút, hoặc ngay sau 1 lần chạy / Kiểm tra lại |
| 5 | Tắt mạng, Đếm 3 lần | 2 lần chạy, lần 3 popup offline |
| 6 | Bật mạng, mở lại app | Nhật ký thêm 2 dòng "bù offline", bộ đếm offline về 0 |
| 7 | Xoá 1 file đếm offline | Vẫn tính 2 (lấy nơi còn lại) |
| 8 | Mạng chập chờn / rút cáp khi bấm | Đếm / Tách bắt đầu ngay, giao diện không đứng |
| 9 | CLI `--tach` khi hết lượt | In thông báo + mã máy, mã thoát 3 |
| 10 | Service account ghi J2 | 403 |

## Related Code Files
- Modify: `README.md` (mục License: cách hoạt động, mã máy, liên hệ; mục Gửi thống kê thêm mã máy), `dema4_rs/src/tro_giup.rs` (Hướng dẫn: mục License), `dema4_rs/Cargo.toml` (0.6.0)
- Create: `plans/261030-1009-license-gioi-han-so-lan/reports/kiem-thu-license-report.md`

## Implementation Steps
1. Chạy `cargo test --release` (xanh) + các test `#[ignore]` của phase 2, 4.
2. Chạy ma trận trên Mac (build local) và Windows (bản CI tag rc).
3. Ghi kết quả vào báo cáo kiểm thử; sửa lỗi phát sinh (test trước rồi sửa).
4. Cập nhật README + Hướng dẫn; bump 0.6.0; tag `v0.6.0` → kiểm tra Release ở `dema4-releases`.
5. Xác nhận lại với chủ Sheet: khoá vùng đã bật trước khi công bố bản.

## Success Criteria
- [ ] 10 tình huống đạt trên cả Mac và Windows.
- [ ] Release v0.6.0 có 2 file ở repo phát hành; app cũ được hướng dẫn tải tay.
- [ ] Báo cáo kiểm thử lưu trong plan.

## Risk Assessment
- Phát hành khi chưa khoá vùng → chặn bằng bước 5 (checklist bắt buộc).
- Máy dev bị tính lượt khi test → cấp "Đã cấp" cho mã máy dev trước khi chạy ma trận (trừ tình huống 1–2 dùng máy / mã thử).
