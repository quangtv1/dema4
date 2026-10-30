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
- 1 lần = 1 lần bấm Đếm / Tách **có kết quả** (≥ 1 file đếm được / ≥ 1 văn bản ghi ra), kể cả khi Dừng hoặc lỗi giữa chừng; không có kết quả → hoàn lượt. Lượt được **đặt chỗ trên đĩa trước khi chạy** nên thoát app / Ctrl+C không lách được.
- Mặc định 2 Đếm + 2 Tách; ô hạn mức trống trên Sheet = 2; ô trạng thái trống = Dùng thử.
- Offline: tối đa min(2, số còn lại theo câu trả lời ký gần nhất lưu trên đĩa) mỗi chức năng; máy đã bị Khoá vẫn khoá khi offline; có mạng lại thì ghi bù (chống trùng bằng mã lần chạy).
- Mã máy chỉ từ ID hệ điều hành (MachineGuid / IOPlatformUUID; dự phòng ID ngẫu nhiên lưu đĩa); tên máy / MAC chỉ là thông tin.
- Áp cho cả giao diện và dòng lệnh (`--tach`, đếm CLI, kể cả `--khong-ghi`).
- **Không chờ mạng khi bấm**: quyết định trên bản sao license trong bộ nhớ. **Không làm mới định kỳ** — chỉ gọi trạm lúc mở app, sau mỗi lần chạy xong (ghi lần dùng + nhận trạng thái mới trong cùng 1 lời gọi) và khi bấm Kiểm tra lại (~2–3 lời gọi / lần dùng). Đặt Khoá: máy chạy được tối đa thêm 1 lần rồi bị chặn.
- Repo `quangtv1/dema4` **giữ công khai**; phát hành như hiện nay.
- Thống kê Ngưỡng trắng + Telegram (GUI và CLI) chuyển qua trạm; khoá service account + token gỡ khỏi app. **Thu hồi khoá service account và đổi token bot ngay (phase 5 bước 0), trước khi tạo tab license.**
- **Rủi ro đã chấp nhận**: spreadsheet + trạm ở Shared Drive TTCNTT (thành viên Drive đọc được khoá RSA + token); bản v0.4.x–v0.5.x vẫn tải được trên Releases và không giới hạn lượt.
- **Telegram qua trạm + hàng đợi**: token + id nhóm lưu ở tab `Cấu hình`, chỉ trạm đọc; app gửi số liệu có cấu trúc, trạm soạn tin rồi **xếp vào tab `Hàng đợi Telegram`**; trigger mỗi 1 phút gửi tối đa 15 lời gọi Telegram (tin Tách + CSV tính 2; Telegram ~20 tin/phút/nhóm), lỗi 429 để lại lần sau → không mất tin, chậm tối đa vài phút. Bỏ token nhúng và `telegram.txt` khỏi app.
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

Thứ tự: phase 5 bước 0 (thu hồi bí mật) làm **ngay**; 1 và 2 song song → 3 → 4; 5 (các bước còn lại) sau 2; 6 cuối. Ước tính tổng ~7,5 ngày (1 + 2,5 + 1 + 1,5 + 0,5 + 1).

## Acceptance criteria (cả kế hoạch)
- Máy mới có mạng: tab `license` tự có dòng của máy; lần thứ 3 Đếm (hoặc Tách) bị chặn bằng popup có mã máy + Copy + Kiểm tra lại.
- Chủ Sheet nâng hạn mức → khách bấm Kiểm tra lại dùng được ngay; đặt Khoá → máy chạy được tối đa 1 lần nữa rồi bị chặn (hoặc chặn ngay khi mở lại app / bấm Kiểm tra lại), kể cả còn lượt.
- App không gọi trạm khi người dùng không thao tác (không có hẹn giờ làm mới).
- 30 tin Telegram tới trạm trong 1 phút → cả 30 tới nhóm trong ≤ 3 phút, không mất tin.
- Offline: 2 lần mỗi chức năng, lần 3 chặn; có mạng lại → nhật ký có dòng "bù offline".
- Exe không chứa khoá ghi Sheet và không chứa token Telegram; câu trả lời giả (sửa 1 ký tự / chữ ký máy khác / phát lại / trang HTML) bị từ chối.
- Không đếm trùng khi mạng chậm / trạm khởi động lạnh; thoát app giữa chừng vẫn bị tính; đổi tên máy không ra máy mới.
- `cargo test` không gọi trạm thật và không ghi vào sổ lượt thật.
- Đổi token / nhóm Telegram trên tab `Cấu hình` → tin kế tiếp đi đúng nhóm mới, không cần phát hành lại.
- Bấm Đếm / Tách quyết định tức thì (không gọi mạng); mất mạng / trạm lỗi không treo giao diện.
- `cargo test --release` xanh; test license không gọi mạng thật (trừ test `#[ignore]`).

## Dependencies
Không phụ thuộc plan khác. Plan `261027-1736-tach-van-ban-tu-dong` (pending) không trùng phạm vi.

## Liên hệ trên popup
- Logo Sao Mai giữa (bấm mở saomaiso.com) · icon ổ khoá đỏ bên trái lý do · lý do 2 dòng "Lý do: Số lần … còn lại (0)" / "Liên hệ với Sao Mai để được hỗ trợ." · mã máy · SĐT **0979824615** · Email **hotro@saomaiso.com** · Web **saomaiso.com**.

## Câu hỏi còn mở
Không.

## Red Team Review

### Session — 2026-10-30
**Findings:** 15 (13 accepted, 2 rejected) · gộp từ ~40 phát hiện của 4 người phản biện (bảo mật, lỗi vận hành, giả định, phạm vi).
**Severity breakdown:** 3 Critical, 8 High, 4 Medium

| # | Finding | Severity | Disposition | Applied To |
|---|---------|----------|-------------|------------|
| 1 | Đếm trùng lượt khi app hết giờ chờ nhưng trạm vẫn ghi | Critical | Accept | Phase 1, 2, 4 (mã lần chạy, 1 luồng gửi, khoá trạm 2,5 s < chờ app 8 s) |
| 2 | Thoát app / Ctrl+C / CLI song song / `--khong-ghi` không bị tính | Critical | Accept | Phase 1, 4 (đặt chỗ trên đĩa trước khi chạy) |
| 3 | Token Telegram công khai trên repo; khoá service account trong exe v0.5.x; thu hồi sau khi tạo tab license | Critical | Accept | Phase 5 bước 0 (làm ngay), plan.md |
| 4 | Mã máy từ tên máy + MAC | High | Accept | Phase 1 |
| 5 | Bộ kiểm license đặt sai struct; test gọi trạm thật | High | Accept | Phase 4 (`UngDung`, `ManTach`, CLI; Default test dùng kho giả) |
| 6 | Offline bỏ qua trạng thái đã biết (Khoá, hết lượt) | High | Accept | Phase 1, 4 (lưu câu trả lời ký trên đĩa) |
| 7 | Trạm chậm dần, đăng ký rác, giới hạn theo mã máy vô tác dụng, đốt quota máy khác | High | Accept | Phase 2 (tổng trong dòng, giới hạn toàn cục, ≤ 3 lần / lời gọi) |
| 8 | Apps Script không trả 503; curl thiếu `-L` | High | Accept | Phase 2, 3, 5 |
| 9 | Tiếng Việt làm hỏng chữ ký / trạng thái Khoá | High | Accept | Phase 1, 2 (mã ASCII, UTF-8, thả xuống, lạ → Khoá) |
| 10 | Chèn công thức / HTML | High | Accept | Phase 2, 6 |
| 11 | Hàng đợi Telegram tranh khoá, gửi trùng, vượt 20 tin / phút | High | Accept | Phase 2 |
| 12 | Chỗ gọi bị sót (`SoLieuDem` không có, CLI đếm gửi Telegram, `co_telegram`, đổi tên `telegram.rs`) | Medium | Accept | Phase 3 (không đổi tên, liệt kê đủ 4 chỗ gọi) |
| 13 | Thiếu phụ thuộc (`getrandom`, registry), `reg.exe` nháy cửa sổ, dán PEM, test Apps Script không lặp lại được, ước lượng thấp | Medium | Accept | Phase 1, 2 (test Node), plan.md (~7,5 ngày) |
| 14 | Bỏ chữ ký RSA | Medium | Reject | Chủ sản phẩm chọn phương án B có chữ ký; giả trạm (hosts / proxy) dễ hơn tự build |
| 15 | Bỏ hàng đợi Telegram | Medium | Reject | Chủ sản phẩm đã duyệt hàng đợi (không mất tin); đã sửa theo #11 |

Quyết định của chủ sản phẩm trong phiên: có kết quả là tính (kể cả Dừng / lỗi); giữ bản cũ trên Releases; giữ spreadsheet trong Shared Drive (2 mục sau ghi là rủi ro đã chấp nhận).

Báo cáo chi tiết: `reports/from-code-reviewer-to-planner-red-team-*-plan-review-report.md`.

### Whole-Plan Consistency Sweep
- Đã rà `plan.md` + 6 file phase sau khi áp 13 điểm: bỏ "làm mới 5 phút", "503", "chờ 4 s", `SoLieuDem`, đổi tên `telegram.rs`, mã máy từ tên + MAC, "luôn 2 lần offline", thu hồi bí mật sau phát hành, khoá vùng / repo riêng tư.
- Thời gian chờ thống nhất: trạm chờ khoá 2,5 s, app chờ lời gọi nền 8 s, Kiểm tra lại ≤ 8 s.
- Ngân sách Telegram thống nhất: 15 lời gọi / lượt (tin Tách + CSV = 2).
- Không còn mâu thuẫn chưa giải quyết.
