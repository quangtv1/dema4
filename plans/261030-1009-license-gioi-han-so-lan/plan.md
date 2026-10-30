---
title: "License gioi han so lan Dem va Tach qua Google Sheet"
description: "Mỗi máy 2 lần Đếm + 2 lần Tách dùng thử; hạn mức / trạng thái trên tab license Google Sheet qua trạm Apps Script ký RSA; exe không còn khoá Sheet; offline tối đa 2 lần; repo giữ công khai."
status: pending
priority: P2
branch: "main"
tags: [license, google-sheet, tdd]
blockedBy: []
blocks: []
created: "2026-10-30T03:31:55.957Z"
createdBy: "ck:plan"
source: skill
---

# License giới hạn số lần Đếm / Tách qua Google Sheet

## Overview
**Phương án B** (đổi từ A ngày 2026-10-30): repo giữ **công khai**; một trạm **Google Apps Script** chạy bằng quyền tài khoản Sao Mai là bên duy nhất đọc/ghi Sheet; exe **không còn khoá Sheet**; số lần dùng do trạm đếm từ nhật ký; mọi câu trả lời license **ký RSA**, app kiểm bằng khoá công khai. Mục tiêu: chặn người dùng phổ thông + nhắc liên hệ Sao Mai (PR), và chặn cố tình phá / tự cấp quota trên Sheet. Không chặn được người tự build lại từ mã nguồn (chấp nhận). Chế độ `--tdd`: mỗi phase viết test trước.

Brainstorm: `plans/reports/brainstorm-261030-1009-license-gioi-han-so-lan-report.md` (phần phương án A trong đó đã được thay bằng B).

## Quyết định đã chốt
- 1 lần = 1 lần bấm Đếm / Tách chạy xong và có kết quả (≥ 1 file đếm được / ≥ 1 văn bản ghi ra).
- Mặc định 2 Đếm + 2 Tách; ô hạn mức trống trên Sheet = 2; ô trạng thái trống = Dùng thử.
- Offline: luôn tối đa 2 lần (mỗi chức năng), kể cả máy đã cấp; có mạng lại thì ghi bù.
- Áp cho cả giao diện và dòng lệnh (`--tach`, đếm CLI).
- **Không chờ mạng khi bấm**: quyết định trên bản sao license trong bộ nhớ. **Không làm mới định kỳ** — chỉ gọi trạm lúc mở app, sau mỗi lần chạy xong (ghi lần dùng + nhận trạng thái mới trong cùng 1 lời gọi) và khi bấm Kiểm tra lại (~2–3 lời gọi / lần dùng). Đặt Khoá: máy chạy được tối đa thêm 1 lần rồi bị chặn.
- Repo `quangtv1/dema4` **giữ công khai**; phát hành như hiện nay.
- Thống kê Ngưỡng trắng chuyển qua trạm; khoá service account bị gỡ khỏi exe và thu hồi.
- **Telegram qua trạm + hàng đợi**: token + id nhóm lưu ở tab `Cấu hình`, chỉ trạm đọc; app gửi số liệu có cấu trúc, trạm soạn tin rồi **xếp vào tab `Hàng đợi Telegram`**; trigger mỗi 1 phút gửi tối đa 15 tin (Telegram ~20 tin/phút/nhóm), lỗi 429 để lại lần sau → không mất tin, chậm tối đa vài phút. Bỏ token nhúng và `telegram.txt` khỏi app.
- **Chịu tải**: trạm quá tải / lỗi → app dùng bản sao hoặc luật offline, ghi bù sau; không mất lượt, không treo.
- **Tài khoản chủ Sheet + Apps Script: Google Workspace** (vd huce.edu.vn hoặc Workspace Sao Mai) — hạn mức UrlFetch ~100.000/ngày, trigger 6 giờ/ngày (Gmail thường: 20.000 và 90 phút).
- AGPL / MuPDF: không còn vướng (repo mở).

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Lõi license](./phase-01-loi-license.md) | Pending |
| 2 | [Trạm Apps Script](./phase-02-tram-apps-script.md) | Pending |
| 3 | [Kho trạm + bỏ khoá Sheet khỏi exe](./phase-03-kho-tram-va-bo-khoa-sheet.md) | Pending |
| 4 | [Gắn license vào GUI và dòng lệnh](./phase-04-gan-license-gui-va-dong-lenh.md) | Pending |
| 5 | [Triển khai trạm lên Sheet thật](./phase-05-trien-khai-tram.md) | Pending |
| 6 | [Kiểm thử tổng và phát hành](./phase-06-kiem-thu-tong-va-phat-hanh.md) | Pending |

Thứ tự: 1 và 2 làm song song → 3 → 4; 5 sau 2 (cần trước khi test thật ở 3 và 6); 6 cuối. Ước tính tổng ~5 ngày.

## Acceptance criteria (cả kế hoạch)
- Máy mới có mạng: tab `license` tự có dòng của máy; lần thứ 3 Đếm (hoặc Tách) bị chặn bằng popup có mã máy + Copy + Kiểm tra lại.
- Chủ Sheet nâng hạn mức → khách bấm Kiểm tra lại dùng được ngay; đặt Khoá → máy chạy được tối đa 1 lần nữa rồi bị chặn (hoặc chặn ngay khi mở lại app / bấm Kiểm tra lại), kể cả còn lượt.
- App không gọi trạm khi người dùng không thao tác (không có hẹn giờ làm mới).
- 30 tin Telegram tới trạm trong 1 phút → cả 30 tới nhóm trong ≤ 3 phút, không mất tin.
- Offline: 2 lần mỗi chức năng, lần 3 chặn; có mạng lại → nhật ký có dòng "bù offline".
- Exe không chứa khoá ghi Sheet và không chứa token Telegram; câu trả lời giả (sửa 1 ký tự / chữ ký máy khác / phát lại) bị từ chối.
- Đổi token / nhóm Telegram trên tab `Cấu hình` → tin kế tiếp đi đúng nhóm mới, không cần phát hành lại.
- Bấm Đếm / Tách quyết định tức thì (không gọi mạng); mất mạng / trạm lỗi không treo giao diện.
- `cargo test --release` xanh; test license không gọi mạng thật (trừ test `#[ignore]`).

## Dependencies
Không phụ thuộc plan khác. Plan `261027-1736-tach-van-ban-tu-dong` (pending) không trùng phạm vi.

## Câu hỏi còn mở
- SĐT / email hiển thị trên popup liên hệ.
