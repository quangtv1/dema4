---
title: Muc luc ho so chuc nang thu 4
description: >-
  Port lõi print_mltl (C# WPF) sang Rust: chức năng thứ 4 Mục lục hồ sơ trong
  dema4, UI 1 màn kiểu Tách, TDD
status: pending
priority: P2
branch: main
tags:
  - muc-luc
  - docx
  - port
  - tdd
blockedBy: []
blocks: []
created: '2026-10-30T15:19:17.320Z'
createdBy: 'ck:plan'
source: skill
---

# Mục lục hồ sơ — chức năng thứ 4

## Overview
Thêm Menu Chức năng › **Mục lục hồ sơ**: đọc biên mục Excel/CSV, gom nhóm theo hồ sơ, ghép mẫu DOCX → mỗi hồ sơ 1 mục lục DOCX (+ PDF). Port lõi https://github.com/quangtv1/print_mltl (v1.12.12, C# .NET 8 WPF, chỉ Windows) sang Rust trong `dema4_rs`, UI 1 màn theo phong cách màn Tách. Brainstorm: `../reports/brainstorm-261030-2212-muc-luc-ho-so-report.md`.

**TDD**: mỗi phase viết test trước. Đối chứng merge = DOCX sinh bởi **Core gốc** (`dotnet`, chạy được macOS) trên 4 mẫu thật + Excel mẫu, so sau chuẩn hoá XML.

## Phases
| Phase | Name | Status |
|-------|------|--------|
| 1 | [Prototype HTML man Muc luc](./phase-01-prototype-html-man-muc-luc.md) | Completed |
| 2 | [Loi doc Excel gom nhom ghep DOCX](./phase-02-loi-doc-excel-gom-nhom-ghep-docx.md) | Completed |
| 3 | [Man Muc luc va license xuat DOCX](./phase-03-man-muc-luc-va-license-xuat-docx.md) | Pending |
| 4 | [Xem truoc va PDF qua Word hoac LibreOffice](./phase-04-xem-truoc-va-pdf-qua-word-hoac-libreoffice.md) | Pending |
| 5 | [Bien anh](./phase-05-bien-anh.md) | Pending |
| 6 | [Resume nhat ky thu lai loi](./phase-06-resume-nhat-ky-thu-lai-loi.md) | Pending |
| 7 | [Du lieu tu ket qua Tach](./phase-07-du-lieu-tu-ket-qua-tach.md) | Pending |

Thứ tự: 1 → 2 → 3 (phát hành được: DOCX) → 4 → 5 → 6 → 7. 5/6/7 độc lập nhau sau 3 (7 cần 2).

## Quyết định đã chốt (brainstorm 30/10)
- Port lõi sang Rust (không gọi MLHS.exe). UI 1 màn kiểu Tách, không wizard.
- PDF / xem trước: Word COM (Windows) chính, LibreOffice (`soffice`) dự phòng; không có cả hai → chỉ DOCX.
- Phạm vi: lõi + biến ảnh + Resume/log/thử lại + dữ liệu Tách (khớp theo thứ tự).
- License: chức năng thứ 3 `muc_luc` (10 lượt thử, cột hạn mức riêng, tin Telegram).

## Ràng buộc
- Repo **công khai**: không commit `sample/Biên_mục_Quảng_trị.xlsx` (dữ liệu thật) — đặt ở `temp/` (gitignore), CI dùng fixture tổng hợp. 4 mẫu DOCX (không dữ liệu cá nhân) nhúng được — kiểm nội dung trước khi commit.
- 1 exe Windows + macOS; không thêm runtime ngoài; crate mới: `calamine`, `csv`, `zip`, `quick-xml` (+ `windows`/COM ở phase 4).
- Theo `docs/` + mẫu mã hiện có (tên tiếng Việt không dấu, `khung_hop`, `cot_trai`, `gioi_thieu`, `hop_cau_hinh`).

## Acceptance (toàn plan)
- 22 hồ sơ Excel mẫu × 4 mẫu → DOCX tương đương Core gốc (test đối chứng xanh), mở được bằng Word.
- Xem trước = bản xuất (cùng DOCX → PDF → MuPDF); PDF khớp DOCX.
- License chặn đúng; tin Telegram 📑 MỤC LỤC HỒ SƠ (x/y).
- `cargo test --release` xanh trên CI Windows + macOS; phát hành phiên bản mới.

## Dependencies
Không có plan chưa xong nào chồng lấn (plan `261027-1736-tach-van-ban-tu-dong` chỉ còn trạng thái cũ, không đụng file mới). Phase 7 đọc `ketqua_tach.txt` của chức năng Tách (định dạng giữ nguyên).
