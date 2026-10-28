---
phase: 7
title: "Phát hành"
status: pending
priority: P2
dependencies: [5, 6]
effort: "0,5 ngày"
---

# Phase 7: Phát hành

## Overview
Kiểm thử tổng trên Windows, cập nhật tài liệu, release v0.3.0.

## Requirements
- Build CI macOS + Windows xanh; đo kích thước exe sau khi thêm zxing-cpp + ort + mô hình hướng trang; exe chạy trên máy sạch không cần DLL onnxruntime; Thông tin ghi giấy phép ONNX Runtime (MIT) + mô hình (Apache-2.0).
- Chạy tay trên Windows (tỷ lệ phóng 125%): phân tích + duyệt + tách mẫu; so kết quả với b3.
- README: mục "Tách văn bản" (quy trình 5 bước như Hướng dẫn trong app: Chọn + Phân tích · sửa nhãn · duyệt Nghi trắng / Nghi xoay · xem văn bản · Tách; định dạng `index_and_blank.txt` + `xoay_trang.txt`; CLI `--tach`; bảng khoá `sm-dema4.cfg`); ghi giấy phép mô hình PP-LCNet (Apache-2.0) + ONNX Runtime (MIT); ghi chú bỏ hẳn WSL/Python. `.gitignore` thêm `sm-dema4.cfg`.
- Telegram: kiểm tra tay trên Windows nhóm nhận đủ 2 loại tin (đếm xong, tách xong).

- Kiểm tra cập nhật (menu Trợ giúp): `GET https://api.github.com/repos/quangtv1/dema4/releases/latest` (ureq đã có), so `tag_name` với `CARGO_PKG_VERSION`; mở `html_url` bằng trình duyệt. Test: so phiên bản (0.3.0 < 0.3.1 < 0.10.0), JSON thiếu trường / lỗi mạng → thông báo, không panic.

## Related Code Files
- Modify: `README.md`, `dema4_rs/Cargo.toml` (version 0.3.0)
- Gộp sẵn: tiêu đề cửa sổ Đếm `Đếm trang A4 (Giải pháp số Sao Mai)` (`dema4_rs/src/gui.rs`, đã sửa, chưa commit)

## Implementation Steps
1. Chạy toàn bộ test + test vàng `#[ignore]`: mẫu 1 (`DEMA4_MAU_TACH`: index 209/209, blank 1/1, cắt 211 part khớp b3), mẫu 2 (`DEMA4_MAU2` + `DEMA4_MAU2_NHAN`: index 1.780/1.780, Trắng ≥ 99,5 %, lọt ≤ 1 %, thời gian), hướng trang mẫu 2 (2 tầng: quyết ≥ 97 %, sai 0).
2. Commit, tag `v0.3.0`, theo dõi Actions, kiểm tra Release có 2 file.
3. Thử exe Windows trên máy thật.

## Success Criteria
- [ ] Test đơn vị + vàng xanh; CI xanh; Release v0.3.0 có exe.
- [ ] Người dùng tách xong mẫu trên Windows không cần công cụ nào khác.

## Risk Assessment
- Windows build lâu hơn vì zxing-cpp (C++, CMake) → cache thư mục build trên CI; không đổi feature mupdf trong plan này.
