---
phase: 5
title: "Kiểm thử, đối chiếu prototype, tài liệu"
status: pending
priority: P1
dependencies: [1, 2, 3, 4]
---

# Giai đoạn 5: Kiểm thử, đối chiếu prototype, tài liệu

## Yêu cầu
- Sửa 8 test đang dựa vào popup (`gui.rs`: `cau_hinh_huy_va_o_sai_khoa_luu`, test Bóc tách chọn / đặt mặc định nhà cung cấp, chụp `cau_hinh_*`, test Mục lục mở Cấu hình; `gui_xoa_ghim/tests.rs` chụp Cấu hình) theo hành vi mới — Lưu không đóng, đóng bằng Quay lại; nhãn tab → mục sidebar.
- Test mới: mở từ từng màn → đúng chức năng; Ctrl+M / Ctrl+B đổi và nhớ sau khi mở lại; cửa sổ thu nhỏ chặn chuột vào màn phía sau.
- Chụp kittest 1600 × 900 cho 11 trạng thái prototype, đặt cạnh ảnh prototype, ghi chênh lệch còn lại.
- Tài liệu: README mục **Cấu hình** (trang toàn cửa sổ, sidebar, Lưu trên tiêu đề, cửa sổ thu nhỏ, Dữ liệu ứng dụng); hướng dẫn: ảnh `03-dem-cau-hinh`, `04-tach-cau-hinh`, `05-muc-luc-cau-hinh`, `06-cau-hinh-*`, `07-xoa-ghim-cau-hinh` + câu chữ "hộp Cấu hình / tab" → "trang Cấu hình / mục"; F1 (`tro_giup.rs`) cùng nội dung.
- Ảnh tài liệu từ PDF mẫu giả, đường dẫn trung tính (không lộ tên tài khoản).
- Phiên bản: gộp vào 2.6.0 (chưa phát hành).

## Tiêu chí xong
- [ ] `cargo test --release` qua hết, build release không cảnh báo.
- [ ] 11 ảnh đối chiếu khớp bố cục prototype.
- [ ] README + docx hướng dẫn cập nhật; commit theo conventional commits (không tag / phát hành khi chưa được duyệt).
