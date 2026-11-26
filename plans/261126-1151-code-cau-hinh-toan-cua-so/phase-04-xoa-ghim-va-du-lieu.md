---
phase: 4
title: "Xóa ghim + Dữ liệu ứng dụng"
status: completed
priority: P2
dependencies: [1, 2]
---

# Giai đoạn 4: Xóa ghim + Dữ liệu ứng dụng

## Tổng quan
Tab Xóa ghim (4 thẻ đã có) vào cột 880 kèm dải "Thư mục đang mở"; trang mới **Dữ liệu ứng dụng** gom thao tác dùng chung.

## Yêu cầu
- **Xóa ghim**:
  - Dải xanh nhạt trên cùng: "Thư mục đang mở: **<đường dẫn>** — thẻ *theo thư mục* áp cho thư mục này". Chưa mở: dải cam "Mở 1 file hoặc thư mục ở màn Xóa ghim để chỉnh phần theo thư mục" (thẻ theo thư mục mờ).
  - 2 cột như hiện tại. Bỏ dòng chú thích cuối (đã có ở sidebar).
- **Dữ liệu ứng dụng**:
  - Thẻ **Thư mục dữ liệu**: đường dẫn thật (`goc_du_lieu()`), ô chỉ đọc + **Mở thư mục** (`mo_bang_he_dieu_hanh`), mô tả nội dung.
  - Thẻ **Dọn dẹp**:
    - **Xoá cache phân tích (Tách)**: thư mục Tách đang mở + dung lượng cache (tính 1 lần khi mở trang, luồng nền); không có cache → nút mờ, "Thư mục đang mở chưa có cache". Hỏi lại → `tach.xoa_cache()`.
    - **Dọn dữ liệu cũ**: hỏi lại → chạy ngầm như hiện tại, toast.
  - Footer tiêu đề: không có Khôi phục mặc định.

## File
- Tạo: `dema4_rs/src/gui_cau_hinh/du_lieu.rs`
- Sửa: `gui_xoa_ghim/the_cau_hinh.rs` (dải thư mục, bỏ chú thích), `gui_tach.rs` (hàm `kich_cache() -> Option<u64>` + thư mục cache đang mở), `gui.rs` (cờ `xoa_cache`, `don_cu` giữ nguyên đường đi)

## Các bước
1. Dải thư mục Xóa ghim (cần đường dẫn thư mục: thêm vào `CauHinhThuMuc` hoặc field riêng của `TabXoaGhim`).
2. `du_lieu.rs`: 2 thẻ, 2 hộp hỏi.
3. Dung lượng cache ở luồng nền (`mpsc`), hiện "đang tính…" khi chưa xong.

## Tiêu chí xong
- [ ] Ảnh khớp `anh-xg.png`, `anh-dl.png`.
- [ ] Test: Xoá cache qua trang Dữ liệu (hỏi → xác nhận → `co_cache` false); Dọn dữ liệu cũ hỏi lại trước khi chạy.

## Rủi ro
- Tính dung lượng thư mục cache lớn (nhiều nghìn file) → chỉ đếm khi mở trang Dữ liệu, không chặn khung hình.
