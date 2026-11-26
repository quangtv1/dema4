---
title: "Code Cấu hình toàn cửa sổ theo prototype đã chốt"
description: "Đổi hộp Cấu hình (popup, tab trên) thành trang toàn cửa sổ có sidebar chức năng bên trái, cột nội dung cố định 880 px, nút Lưu trên tiêu đề, chế độ cửa sổ thu nhỏ, sidebar thu nhỏ"
status: pending
priority: P1
effort: ~3,5 ngày công
branch: main
tags: [cau-hinh, gui, egui, prototype]
blockedBy: []
blocks: []
related: [261126-1007-code-man-xoa-ghim-theo-prototype]
created: 2026-11-26
---

# Code Cấu hình toàn cửa sổ theo prototype

## Nguồn chuẩn

- Prototype đã chốt: [`../261126-1129-prototype-cau-hinh-toan-cua-so/prototype-cau-hinh.html`](../261126-1129-prototype-cau-hinh-toan-cua-so/prototype-cau-hinh.html) — `#dem` `#tach` `#ml` `#bt-doc` `#bt-ghi` `#xg` `#dl` `#chua-luu` `#roi`, thêm `&nho` (cửa sổ thu nhỏ), `&thu` (sidebar thu nhỏ).
- Ảnh chốt 1600 × 900: [`../261126-1129-prototype-cau-hinh-toan-cua-so/assets/`](../261126-1129-prototype-cau-hinh-toan-cua-so/assets/) (11 ảnh).
- Bảng thông số: [phase-01](./phase-01-khung-trang-cau-hinh.md#bảng-thông-số).

## Quyết định đã chốt (với người dùng)

- Cấu hình **không còn là popup**: chiếm toàn vùng màn chức năng (thanh menu chung giữ nguyên, nút **Cấu hình** sáng).
- Danh sách chức năng ở **sidebar trái 240 px** (Chức năng: Đếm trang A4 · Tách văn bản · Sinh mục lục · Bóc tách dữ liệu · Xóa ghim; Ứng dụng: Dữ liệu ứng dụng). Bỏ hàng tab trên.
- **Kích thước cố định mọi chức năng**: cột nội dung 880 px (cửa sổ thu nhỏ: hết bề ngang cửa sổ trừ lề 20).
- Lưu kiểu **nút trên tiêu đề**: Khôi phục mặc định · nhãn "n chức năng chưa lưu" · Huỷ thay đổi · **Lưu** (Ctrl+S) · nút cửa sổ. Chức năng có thay đổi: chấm cam ở sidebar. Lưu **không đóng** trang.
- **Cửa sổ thu nhỏ** (nút ⧉ / □ cạnh Lưu, Ctrl+M): khung 1120 × 720 nổi trên màn chức năng (mờ phía sau), thanh tiêu đề chỉ có ✕.
- **Sidebar thu nhỏ** (nút khung-cột cạnh "Quay lại", Ctrl+B): dải 56 px chỉ biểu tượng, tooltip tên, chấm cam vẫn hiện.
- **Quay lại** / Esc / ✕ / bấm nền mờ: còn thay đổi → hộp "Lưu thay đổi trước khi quay lại?" (Không lưu · Ở lại · Lưu và quay lại).
- **Xoá cache** và **Dọn dữ liệu cũ** chuyển sang trang **Dữ liệu ứng dụng** (có hỏi lại), kèm thư mục dữ liệu + Mở thư mục.
- Nội dung từng chức năng giữ đủ mọi tuỳ chọn hiện có.

## Các giai đoạn

| # | Giai đoạn | Effort | Phụ thuộc | Trạng thái |
|---|-----------|--------|-----------|------------|
| 1 | [Khung trang Cấu hình: toàn cửa sổ / cửa sổ thu nhỏ, sidebar, tiêu đề, gắn vào app](./phase-01-khung-trang-cau-hinh.md) | 1 ngày | — | Pending |
| 2 | [Theo dõi thay đổi theo chức năng, Lưu / Huỷ / Mặc định, hỏi khi rời](./phase-02-thay-doi-luu-huy.md) | 0,5 ngày | 1 | Pending |
| 3 | [Nội dung Đếm, Tách, Mục lục, Bóc tách trong cột 880](./phase-03-noi-dung-cac-chuc-nang.md) | 0,75 ngày | 1 | Pending |
| 4 | [Xóa ghim + Dữ liệu ứng dụng](./phase-04-xoa-ghim-va-du-lieu.md) | 0,5 ngày | 1, 2 | Pending |
| 5 | [Kiểm thử, đối chiếu prototype, tài liệu](./phase-05-kiem-thu-doi-chieu-tai-lieu.md) | 0,75 ngày | 1–4 | Pending |

## Kiến trúc tóm tắt

- Giữ kiểu `HopCauHinh` (đổi tên thành `TrangCauHinh` ở bước cuối giai đoạn 1 nếu không vướng test) ở `gui_tach/hop_cau_hinh.rs`; tách phần khung mới ra `gui_cau_hinh/` (`khung.rs`: sidebar + tiêu đề + cửa sổ; `du_lieu.rs`: trang Dữ liệu ứng dụng; `phan.rs`: nhóm trường theo chức năng).
- `gui.rs`: khi `hop_cau_hinh.is_some()` — toàn cửa sổ: **không** vẽ màn chức năng, vẽ trang Cấu hình trong `CentralPanel`; cửa sổ thu nhỏ: vẽ màn chức năng (không nhận chuột / phím) rồi lớp mờ + `Area` 1120 × 720.
- `ve()` trả `KetQuaTrang { Mo, Dong, Luu(CauHinh) }` — `Luu` không đóng; app áp cấu hình như hiện tại (`ap_cau_hinh`, `ap_cau_hinh_thu_muc`, khoá Bóc tách).
- Thay đổi theo chức năng: `phan::chi_phan(tab, &CauHinh) -> CauHinh` (chỉ giữ trường của tab, còn lại mặc định) → `doi(tab) = chi_phan(tab, nhap) != chi_phan(tab, goc)`; Bóc tách cộng khoá API, Xóa ghim cộng tuỳ chọn thư mục.
- Lưu bền: 2 khoá cấu hình app mới `ch_cua_so_nho`, `ch_thu_ds`.

## Rủi ro chính

- 8 test kittest đang dựa vào hành vi popup (Lưu / Huỷ đóng hộp, nhãn tab) → sửa theo hành vi mới (giai đoạn 5), không nới điều kiện.
- Màn chức năng không được vẽ khi Cấu hình toàn cửa sổ → việc nền vẫn chạy (luồng riêng), nhưng hàng nhận kết quả (`nhan_*`) chỉ chạy khi màn vẽ lại; Cấu hình đã bị khoá khi đang chạy nên không mất tiến độ. Thêm kiểm: mở Cấu hình khi Bóc tách đang tải danh sách model.
- Tab Bóc tách hiện vẽ cho hộp rộng 780 → cột 880 rộng hơn, cần soát ô bị kéo giãn.

## Câu hỏi còn mở

- Không có.
