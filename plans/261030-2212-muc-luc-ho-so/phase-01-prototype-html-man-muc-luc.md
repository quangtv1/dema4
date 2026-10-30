---
phase: 1
title: Prototype HTML man Muc luc
status: in-progress
priority: P1
dependencies: []
effort: 1 ngày
---

# Phase 1: Prototype HTML màn Mục lục

## Overview
Dựng màn Mục lục trong prototype HTML hiện có (nguồn chân lý UI) để chủ sản phẩm duyệt bố cục trước khi code.

## Requirements
- Thêm mục 4 **Mục lục hồ sơ** vào menu Chức năng của prototype; cùng font, màu, khung, nút, bảng với màn Tách.
- Trạng thái cần xem (hash URL như `#het-luot`):
  - `#muc-luc` — màn giới thiệu 3 bước (Chọn dữ liệu · Ghép biến · Tạo mục lục).
  - `#muc-luc-ghep` — đã chọn Excel + mẫu: dòng Dữ liệu (file · Sheet ▾ · Header dòng), dòng Mẫu (4 mẫu ▾ · Chọn DOCX); cột trái Hồ sơ (#, tên, số VB, ✓/⚠); tab **Ghép biến**: bảng Biến · Nguồn (Cột ▾ / Hằng / Ảnh / Từ Tách) · Giá trị hồ sơ đang chọn; cột gom nhóm; Kiểm tra dữ liệu.
  - `#muc-luc-xem` — tab **Xem trước**: ảnh trang, zoom, ◀ Hồ sơ i/n ▶, ghi chú nguồn render (Word / LibreOffice / không có → "chỉ xuất DOCX").
  - `#muc-luc-tao` / `#muc-luc-xong` — popup tiến độ (giống "Đang tách") và popup kết quả (số DOCX/PDF, lỗi, Mở thư mục, Thử lại lỗi).
  - Cấu hình › tab **Mục lục**: thư mục ra, mẫu tên `MLHS_{so_ho_so}`, PDF, Ghi đè, Chạy tiếp (Resume).
- Dòng đáy: Thông tin (Số hồ sơ; Số văn bản; Biến thiếu) + nút **Tạo mục lục** (khoá khi còn lỗi).
- Dữ liệu giả lấy theo Mẫu 01 (7 cột) — không dùng tên người thật.

## Related Code Files
- Modify: `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`

## Implementation Steps
1. Đọc cấu trúc prototype (menu, `.phu`/`.popup`, bảng cột trái, `hl-*`, Cấu hình tab).
2. Thêm khối HTML + CSS dùng lại class sẵn có; JS chuyển trạng thái theo hash.
3. Chụp ảnh từng trạng thái (Chrome headless `--screenshot`) gửi duyệt; sửa theo góp ý.

## Tests / Validation
- Mở từng hash không lỗi console; ảnh chụp 6 trạng thái.
- Duyệt của chủ sản phẩm = điều kiện sang phase 3 (phase 2 chạy song song được).

## Success Criteria
- [ ] 6 trạng thái hiển thị đúng, đồng bộ phong cách Tách.
- [ ] Chủ sản phẩm duyệt; chốt câu hỏi mở: thư mục ra mặc định, nhiều sheet.

## Risk Assessment
- Prototype phình to → chỉ thêm khối mới, không sửa màn cũ.
