---
phase: 4
title: "Thiet lap Sheet"
status: pending
priority: P1
dependencies: []
effort: "0.5 ngày (một phần thao tác tay của chủ Sheet)"
---

# Phase 4: Thiết lập Sheet (tab, công thức, khoá vùng, mô tả)

## Overview
Dựng tab `license`, `license_nhat_ky`, `Mô tả license` trên spreadsheet Monitor_dema4. Phần tạo tab / tiêu đề / công thức làm bằng test `#[ignore]` (service account); phần **khoá vùng phải do chủ Sheet làm bằng tay** — service account tự tạo khoá vùng thì chính nó vẫn được sửa, vô nghĩa.

## Requirements
- Hàng 1 tab `license`: tiêu đề A–N; G1, H1, I1 là ARRAYFORMULA tự tính cho mọi hàng.
- Khoá vùng `license!G:N` và toàn bộ hàng 1 của cả 2 tab: chỉ chủ Sheet.
- Tab mô tả bên phải giải thích từng cột + cách cấp / khoá máy.

## Architecture
Công thức (đặt ở hàng tiêu đề, `USER_ENTERED`):
- G1: `={"Đã dùng Đếm"; ARRAYFORMULA(IF(A2:A="",, COUNTIFS(license_nhat_ky!B:B, A2:A, license_nhat_ky!C:C, "Đếm")))}`
- H1: như G1 với `"Tách"`.
- I1: `={"Lần dùng cuối"; ARRAYFORMULA(IF(A2:A="",, IFERROR(MAXIFS(license_nhat_ky!A:A, license_nhat_ky!B:B, A2:A))))}` (định dạng ngày giờ).
- Thời điểm nhật ký ghi dạng `2026-10-30 10:00:00` với `USER_ENTERED` để Sheet hiểu là ngày giờ.

## Related Code Files
- Modify: `dema4_rs/src/google_sheet.rs` (test `#[ignore]` `tao_tab_license`), `dema4_rs/src/license.rs` (hằng tên tab, tiêu đề, hàng mô tả — dùng chung với phase 2)

## Tests trước
1. `mo_ta_license_du_cot`: số hàng mô tả = số cột + ghi chú; mỗi hàng 3 ô.
2. `#[ignore]` `tao_tab_license`: tạo 3 tab + tiêu đề + công thức; đọc lại G1 là "Đã dùng Đếm".
3. Kiểm tra tay sau khi khoá vùng: `#[ignore]` `service_account_khong_sua_duoc_vung_khoa` — ghi vào `license!J2` → mong đợi lỗi 403.

## Implementation Steps
1. Viết test 1, đỏ → hằng + hàm mô tả → xanh.
2. Viết + chạy `tao_tab_license` (1 lần).
3. **Chủ Sheet làm tay** (ghi vào tab mô tả và README):
   1. Mở Monitor_dema4 → tab `license` → chọn cột G:N → Dữ liệu › Bảo vệ trang tính và dải ô → "Chỉ bạn".
   2. Bảo vệ hàng 1 của `license` và `license_nhat_ky` tương tự.
   3. Cấp máy nội bộ / dev: điền J, K (vd 100000) và L = "Đã cấp".
4. Chạy test 3, xác nhận 403.

## Success Criteria
- [ ] 3 tab có trên Sheet, tab mô tả nằm bên phải.
- [ ] Thêm tay 1 dòng nhật ký → G/H/I của máy tương ứng tự cập nhật.
- [ ] Service account ghi vào J2 bị từ chối.

## Risk Assessment
- Quên khoá vùng → ai có khoá trong exe tự nâng hạn mức. Ghi rõ bước này trong tiêu chí phát hành (phase 6).
- ARRAYFORMULA chậm khi nhật ký rất dài (> 100k dòng) → khi đó chuyển sang tổng hợp định kỳ (để sau).
