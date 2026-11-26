---
phase: 1
title: "Bản vá 2.4.x: bản cũ không làm hỏng dữ liệu của bản mới"
status: completed
priority: P1
dependencies: []
---

# Phase 1: Bản vá 2.4.x — tương thích ngược

## Overview
Phát hành **trước** bản có Xóa ghim, để máy còn 2.4 gặp dữ liệu của bản mới không xoá mất (red team #10, #11).

## Requirements
- `gan_day.rs`: đọc danh sách "Làm tiếp gần đây" **từng mục**, bỏ qua mục có chức năng lạ, khi ghi lại **giữ nguyên** các mục lạ (hiện `gan_day.rs:91` hỏng 1 mục → rỗng cả danh sách → ghi đè).
- `license.rs`: khi đọc file lượt, mục có mã chức năng lạ (`license.rs:194` đang bỏ) được **giữ nguyên** (JSON thô) khi ghi lại và vẫn tính vào giới hạn offline (`TOI_DA_OFFLINE`).
- Không đổi định dạng file nào; không tăng `PHIEN_BAN_GOI` (gói đọc theo tên mục — `phien_lam_viec.rs:350-355` — mục lạ đã được bỏ qua).

## Related Code Files
- Modify: `dema4_rs/src/gan_day.rs`, `dema4_rs/src/license.rs`, `dema4_rs/Cargo.toml` (phiên bản)

## Tests (viết trước)
- Danh sách gần đây có 1 mục `"XoaGhim"` → đọc được các mục khác; lưu lại → mục lạ còn nguyên.
- File lượt có mục `cn: "xoa_ghim"` chờ gửi → đọc / sửa / ghi → mục còn; đếm offline tính cả mục đó.

## Success Criteria
- [ ] Toàn bộ test xanh (trừ 5 lỗi có sẵn); CI 2 nền tảng xanh; phát hành 2.4.x (khi user đồng ý).
