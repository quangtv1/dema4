---
title: "Code lại màn Xóa ghim theo prototype đã chốt"
description: "Chuyển prototype HTML (chốt 26/11/2026) sang egui 0.37 — cùng bố cục, kích thước, màu, biểu tượng, chữ, hành vi"
status: in-progress
priority: P1
effort: ~14–16 ngày công (gồm AI)
branch: main
tags: [xoa-ghim, gui, egui, prototype]
blockedBy: []
blocks: []
related: [261125-1750-chuc-nang-xoa-ghim]
created: 2026-11-26
---

# Code lại màn Xóa ghim theo prototype đã chốt

## Nguồn chuẩn (single source of truth)

- Prototype: [`../261126-0727-prototype-man-xoa-ghim/prototype-xoa-ghim.html`](../261126-0727-prototype-man-xoa-ghim/prototype-xoa-ghim.html) — mở `#mo` (đã mở thư mục), `#mo-cfg` (Cấu hình), `#mo-tep` (menu Tệp tin).
- Ảnh chốt 1600×900: [`../261126-0727-prototype-man-xoa-ghim/assets/`](../261126-0727-prototype-man-xoa-ghim/assets/) (`anh-trong`, `anh-mo`, `anh-cau-hinh`, `anh-tep-tin`).
- Bảng thông số (màu, cỡ, khoảng cách) trích từ CSS prototype: [phase-01](./phase-01-nen-tang-thiet-ke-va-thanh-menu.md#bảng-thông-số).
- Đánh giá khả thi: [`reports/danh-gia-kha-thi-261126-1007-chuyen-prototype-sang-egui-report.md`](./reports/danh-gia-kha-thi-261126-1007-chuyen-prototype-sang-egui-report.md).

## Định nghĩa "giống 100%"

Đạt khi **mọi mục** của [checklist đối chiếu](./phase-09-kiem-thu-doi-chieu-va-phat-hanh.md#checklist-đối-chiếu-prototype) đều ✓:
1. Bố cục, kích thước (px), khoảng cách, vị trí từng thành phần khớp bảng thông số (sai số ≤ 2 px do làm tròn DPI).
2. Màu, viền, bo góc, biểu tượng (dùng **chính SVG** của prototype), chữ (nội dung, hoa/thường, đậm/thường) khớp.
3. Mọi hành vi tương tác trong prototype có mặt và cho cùng kết quả (bảng hành vi trong từng phase).
4. Ảnh chụp kittest 1600×900 của 4 trạng thái đặt cạnh ảnh prototype không lệch bố cục.

Ngoại lệ không tránh được (đã ghi trong báo cáo khả thi): cách vẽ chữ của egui ≠ trình duyệt (khử răng cưa, kerning); trang PDF là ảnh quét thật thay cho trang HTML giả; bóng đổ dùng `Shadow` của egui.

## Các giai đoạn

| # | Giai đoạn | Effort | Phụ thuộc | Trạng thái |
|---|-----------|--------|-----------|------------|
| 1 | [Nền tảng thiết kế + thanh menu chung](./phase-01-nen-tang-thiet-ke-va-thanh-menu.md) | 1,5 ngày | — | Xong |
| 2 | [Khung màn: danh sách, thanh công cụ 2 nửa, Gốc / Đích, thanh đáy, màn trống](./phase-02-khung-man-va-bo-cuc.md) | 1,5 ngày | 1 | Xong |
| 3 | [Danh sách: sắp xếp, trạng thái, lọc gộp, Lọc nâng cao](./phase-03-danh-sach-va-loc.md) | 1 ngày | 2 | Xong |
| 4 | [Vùng: biểu tượng, vẽ, chọn nhiều, kéo, chuột phải, Vô đối](./phase-04-vung-va-tuong-tac.md) | 2,5 ngày | 2 | Xong |
| 5 | [Xem trang: 2 khung cuộn đồng bộ, kiểu xem, tỉ lệ](./phase-05-xem-trang-va-ty-le.md) | 1 ngày | 2 | Xong |
| 6 | [Đầu vào / đầu ra: File / Thư mục, kiểu ghi, thư mục đích, Ghi đè](./phase-06-dau-vao-dau-ra.md) | 1,5 ngày | 2 | Xong |
| 7 | [Cấu hình thẻ Xóa ghim + tải mô hình AI](./phase-07-cau-hinh-va-mo-hinh-ai.md) | 1 ngày | 1, 6 | Xong |
| 8 | [Chạy, tiến độ, kết quả, F1, thông báo nhanh](./phase-08-chay-ket-qua-va-tro-giup.md) | 0,5 ngày | 3, 6 | Xong |
| 9 | [Kiểm thử, đối chiếu prototype, tài liệu, phát hành](./phase-09-kiem-thu-doi-chieu-va-phat-hanh.md) | 1,5 ngày | 1–8, 10 | Đang làm (chờ duyệt tag / phát hành) |
| 10 | [AI nhận diện layout (gộp từ kế hoạch Xóa ghim)](./phase-10-ai-nhan-dien-layout.md) | 3 ngày | 4, 7 | Xong |

Thứ tự làm: 1 → 2 → (3, 4, 5, 6 có thể xen) → 7 → 10 → 8 → 9.

## Kết quả

Xem [`reports/ket-qua-261126-1120-code-man-xoa-ghim-report.md`](./reports/ket-qua-261126-1120-code-man-xoa-ghim-report.md).

## Quyết định đã chốt (từ phiên thiết kế với người dùng)

- Thanh menu chung app: **Tệp tin ▾ · Chức năng ▾ · Cấu hình · Trợ giúp ▾**; góc phải chỉ còn **^** (thu gọn thanh công cụ). Bỏ tab Chỉnh sửa / Xem / Cài đặt.
- Nút **Xóa ghim (n)** + **▾** ở góc **dưới** phải (thanh đáy).
- Danh sách luôn mở rộng khi vào màn; thu nhỏ thành dải dọc chỉ khi người dùng bấm; kéo rộng 200–520 px.
- Cột Trang thu nhỏ chỉ hiện khi đã mở file / thư mục.
- Màu vùng: góc / cạnh **xanh lá**, vẽ Tùy biến **xanh lam**, bảo vệ **hồng tím nét đứt**, Vô đối **cam**.
- Cạnh luôn phủ hết chiều dài mép, chỉ kéo bề dày.
- "Xóa vùng vẽ" = xoá hết vùng Tùy biến (có hỏi); Delete = xoá vùng vẽ đang chọn ngay (Ctrl+Z).
- Kiểu ghi: **Nhân bản thư mục** (mặc định) / **Cạnh file gốc**; thư mục đích tuỳ chọn; Ghi đè bật → dòng File đích mờ.
- Nhận diện layout: tải mô hình 1 lần (37,5 MB, SHA-256) ở phase 7; **AI nhận diện gộp vào kế hoạch này** (phase 10, chuyển từ phase 7 kế hoạch Xóa ghim).
- **Xử lý theo lô 10 trang**: bật = song song tối đa 10 trang / file (nhanh hơn, tốn RAM hơn); tắt = từng trang.
- **F1 / Trợ giúp → Hướng dẫn sử dụng** ở màn Xóa ghim: bảng thao tác nhanh đúng prototype (không thêm nút).
- **Phông Segoe UI chỉ cho màn Xóa ghim** (font family riêng); các màn khác giữ phông hiện tại.
- **Mọi module** có menu **Tệp tin** ở đầu (chạy thật theo từng module) và hộp **Cấu hình** có thêm thẻ **Xóa ghim**.

## Tiêu chí hoàn thành

- [ ] Checklist đối chiếu prototype 100% ✓ (phase 9).
- [ ] `cargo test` toàn bộ qua (hiện 381); test mới cho mọi hành vi ở bảng hành vi.
- [ ] Không đổi kết quả xoá ghim (đối chiếu Python 266 trang vẫn khớp từng điểm ảnh).
- [ ] Kho cũ (`kho_xoa_ghim.bin`) đọc được; bản cũ đọc kho mới không vỡ (bỏ qua trường lạ).
- [ ] Màn khác (Đếm, Tách, Mục lục, Bóc tách) vẫn chạy; menu Tệp tin hoạt động ở mọi màn.

## Rủi ro chính

- Khối lượng giao diện lớn trong 1 màn egui → chia file theo vùng trách nhiệm (đã có `gui_xoa_ghim/*`), mỗi file < 600 dòng.
- Đổi menu chung ảnh hưởng mọi màn → làm ở phase 1, test kittest các màn khác.
- Kho: thêm trường (kiểu ghi, thư mục đích, ghi đè, lô, tự lưu) → mã hoá có phiên bản, test đọc ngược.
- "Cạnh file gốc" / "Ghi đè" đổi cách xác định trạng thái → phase 6 có thiết kế riêng + test.

## Câu hỏi đã chốt (26/11/2026)
1. Lô 10 trang → song song tối đa 10 trang / file.
2. F1 → bảng thao tác nhanh, không thêm nút.
3. AI → gộp vào kế hoạch này (phase 10).
4. Phông → chỉ màn Xóa ghim; menu Tệp tin ở đầu mọi module; thêm thẻ Xóa ghim trong Cấu hình.
