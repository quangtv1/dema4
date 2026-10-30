---
title: "License gioi han so lan Dem va Tach qua Google Sheet"
description: "Mỗi máy được 2 lần Đếm + 2 lần Tách dùng thử; hạn mức / trạng thái quản lý trên tab license của Google Sheet; offline tối đa 2 lần; repo chuyển riêng tư, phát hành qua repo công khai riêng."
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
Phương án A đã chốt ở brainstorm (`plans/reports/brainstorm-261030-1009-license-gioi-han-so-lan-report.md`): app đọc/ghi tab `license` + `license_nhat_ky` trực tiếp bằng khoá service account đang nhúng; cột hạn mức / trạng thái / số đã dùng được **khoá vùng** (chỉ chủ Sheet sửa) và "Đã dùng" là **công thức đếm nhật ký**. Chế độ `--tdd`: mỗi phase viết test trước, chạy đỏ, rồi mới code cho xanh.

## Quyết định đã chốt
- 1 lần = 1 lần bấm Đếm / Tách chạy xong và có kết quả (≥ 1 file đếm được / ≥ 1 văn bản ghi ra).
- Mặc định 2 Đếm + 2 Tách; ô hạn mức trống trên Sheet = 2; ô trạng thái trống = Dùng thử.
- Offline: luôn tối đa 2 lần (mỗi chức năng), kể cả máy đã cấp; có mạng lại thì ghi bù nhật ký.
- Áp cho cả giao diện và dòng lệnh (`--tach`, đếm CLI).
- **Không chờ mạng khi bấm**: quyết định trên bản sao license trong bộ nhớ (đọc ngầm lúc mở app, làm mới 5 phút / sau mỗi lần chạy / khi bấm Kiểm tra lại); ghi nhật ký ngầm sau khi chạy. Thay đổi trên Sheet có hiệu lực chậm tối đa 5 phút.
- Repo `quangtv1/dema4` chuyển riêng tư; file exe phát hành ở repo công khai `quangtv1/dema4-releases`.
- AGPL / MuPDF: tạm chấp nhận rủi ro.

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Loi license](./phase-01-loi-license.md) | Pending |
| 2 | [Kho license Google Sheet](./phase-02-kho-license-google-sheet.md) | Pending |
| 3 | [Gan license GUI va dong lenh](./phase-03-gan-license-gui-va-dong-lenh.md) | Pending |
| 4 | [Thiet lap Sheet](./phase-04-thiet-lap-sheet.md) | Pending |
| 5 | [Repo rieng tu va kenh phat hanh](./phase-05-repo-rieng-tu-va-kenh-phat-hanh.md) | Pending |
| 6 | [Kiem thu tong va phat hanh](./phase-06-kiem-thu-tong-va-phat-hanh.md) | Pending |

Thứ tự: 1 → 2 → 3; 4 làm song song với 2–3 (thao tác tay trên Sheet, cần trước khi test thật ở 6); 5 độc lập, làm trước khi phát hành; 6 cuối.

## Acceptance criteria (cả kế hoạch)
- Máy mới có mạng: tab `license` tự có dòng của máy; lần thứ 3 Đếm (hoặc Tách) bị chặn bằng popup có mã máy + nút Copy + Kiểm tra lại.
- Chủ Sheet nâng hạn mức → khách bấm Kiểm tra lại dùng được ngay; đặt Khoá → máy bị chặn trong ≤ 5 phút (hoặc ngay sau lần chạy kế tiếp), kể cả còn lượt.
- Offline: 2 lần mỗi chức năng, lần 3 chặn; có mạng lại → nhật ký có dòng "bù offline".
- Service account không sửa được các cột khoá vùng (thử ghi → Sheet trả 403).
- Bấm Đếm / Tách quyết định tức thì (không gọi mạng); mất mạng / Sheet lỗi không treo giao diện (mọi gọi mạng ở luồng ngầm, bỏ cuộc sau 4 s).
- `cargo test --release` xanh; test license không gọi mạng thật.

## Dependencies
Không phụ thuộc plan khác. Plan `261027-1736-tach-van-ban-tu-dong` (pending) không trùng phạm vi.

## Câu hỏi còn mở
- SĐT / email hiển thị trên popup liên hệ.
- Tên repo phát hành (đề xuất `quangtv1/dema4-releases`).
