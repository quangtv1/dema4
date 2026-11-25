---
phase: 3
title: "Khung màn hình + tab Trường dữ liệu + popup"
status: completed
priority: P1
dependencies: [1, 2]
---

# Giai đoạn 3: Khung màn hình + tab Trường dữ liệu

## Tổng quan
Màn `Man::BocTach` thật (thay màn giới thiệu đang phát triển): khối đầu vào, màn giới thiệu 3 bước có ✓, tab Trường dữ liệu (phương án B), dải Kết quả đọc thử, popup Câu lệnh / Chỉnh sửa loại tài liệu / xác nhận.

## Yêu cầu (theo prototype)
- Khối đầu vào (chữ 13): Nguồn văn bản [Máy tính ▾][đường dẫn ⓧ][Chọn] · Loại tài liệu [— Chọn —▾] · Mô hình đọc [— Chọn —▾ ● Đã kết nối][Kiểm tra] · Ghi ra ☑ Excel ☑ Google Sheet · Mở Sheet (rê: xanh + gạch chân). Mặc định chưa chọn gì.
- Giới thiệu: 3 thẻ (nguồn / mô hình / loại), bước xong → ✓ nền xanh; đủ 3 mới hiện tab và bật Bắt đầu.
- Hàng tab: "Trường dữ liệu (n)" · "Kết quả bóc (n)" rộng cố định 170 px, gạch chọn 1 px; cảnh báo gửi ảnh ra ngoài; nút thu gọn khối đầu vào.
- Tab Trường dữ liệu: bảng 2/3 | PDF 1/3, thanh kéo giữa (mượt, theo độ lệch chuột, nhớ). Bảng chỉ xem: # | Tên cột (+chip biến) | Mô tả (Câu lệnh) | Chuẩn hoá ✎; kéo độ rộng cột ở mép tiêu đề; dòng chọn nền xanh, chữ thường; bấm đúp → popup sửa.
- Khung PDF: ‹ tên file › (căn trái) · Thử đọc · − 100% +; thanh cao bằng tiêu đề bảng (30 px).
- Dải Kết quả đọc thử ngang toàn bề rộng: mặc định mở, 2 dòng (tên cột | giá trị, chưa thử = "-"), cột đầu Tên file, Số trang; kéo độ rộng cột (Trích yếu rộng nhất); kéo mép trên đổi chiều cao; bấm thanh để thu/mở; icon gạch + mũi tên; nền tiêu đề xám nhạt; bấm cột → chọn trường.
- Popup Câu lệnh (nút Chép → clipboard). Popup Chỉnh sửa loại tài liệu (bảng đầy đủ, + Thêm trường bên phải, Xoá loại · Lưu thành loại mới… bên trái, xác nhận cho 2 nút này).
- Không tô vùng trên PDF (quyết định #1): khung PDF chỉ hiện trang.

## Kiến trúc
- `src/gui_boc_tach.rs` (`ManBocTach`, trạng thái, `ve()`), `src/gui_boc_tach/`: `dau_vao.rs`, `truong.rs` (bảng + dải đọc thử), `xem_pdf.rs` (dùng lại cache ảnh trang của Tách nếu được), `hop_loai.rs`, `hop_cau_lenh.rs`.
- Bảng dùng `egui_extras::TableBuilder` hoặc tự vẽ như bảng biến Sinh mục lục (đã có kéo cột) — chọn cách đang dùng trong repo.
- Chuyển chức năng = đặt lại (`chuyen_man` như Sinh mục lục).

## File liên quan
- Tạo: `src/gui_boc_tach.rs`, `src/gui_boc_tach/*.rs`.
- Sửa: `src/gui.rs` (Man::BocTach vẽ màn thật, reset), `src/gioi_thieu.rs` (BOC_TACH: link bấm được, ✓ theo trạng thái — có thể cần mở rộng `GioiThieu`), `src/trang_chu.rs` (bỏ "Đang phát triển" ở ô Bóc tách khi xong giai đoạn 6).

## Các bước
1. Khung + khối đầu vào + giới thiệu 3 bước.
2. Bảng trường + kéo cột + kéo thanh chia.
3. Khung PDF + ‹ › + thu phóng.
4. Thử đọc (luồng nền, giai đoạn 1) + dải kết quả.
5. Popup Câu lệnh, Chỉnh sửa loại + xác nhận.
6. Test kittest + ảnh chụp các hash tương ứng.

## Tiêu chí xong
- [ ] Ảnh kittest khớp `boc-tach.png`, `boc-tach-truong-b*.png`, `boc-tach-sua-loai.png`, `boc-tach-cau-lenh.png`, `boc-tach-xoa-loai.png`, `boc-tach-luu-loai-moi.png`.
- [ ] Kéo cột / kéo thanh / kéo chiều cao không giật; nhớ qua lần mở sau.

## Rủi ro
- egui không có grid co giãn như CSS → tự tính độ rộng cột (Mô tả lấp phần còn lại).
- Hiển thị PDF lớn → dùng chung cơ chế ảnh nhỏ / cache của Tách.
