---
phase: 3
title: "Nội dung Đếm, Tách, Mục lục, Bóc tách"
status: pending
priority: P2
dependencies: [1]
---

# Giai đoạn 3: Nội dung Đếm, Tách, Mục lục, Bóc tách trong cột 880

## Tổng quan
Đưa thân 4 tab hiện có vào thẻ của trang mới, đúng bố cục prototype, giữ nguyên mọi điều khiển và hành vi (kéo thước khổ, thanh ngưỡng, kiểm tra kết nối, ghi thử Sheet…).

## Yêu cầu
| Chức năng | Thẻ | Ghi chú |
|---|---|---|
| Đếm trang A4 | 1. Hệ số quy A4 (lưới 4 cột A0…A7 + Lớn hơn A0) · 2. Ranh giới khổ theo cạnh dài (thước kéo được + chú thích + **Thử cạnh dài → khổ**) | thước giãn theo bề rộng thẻ (~840) |
| Tách văn bản | 1. Tự động nhận dạng khi Phân tích (Trang trắng + thanh ngưỡng, Mã index, Chiều trang + ước tốc độ) · 2. Kết quả tách | giữ `thanh_nguong` |
| Sinh mục lục | File kết quả (Thư mục ra, Mẫu tên file + ví dụ, File đã có, Định dạng) | |
| Bóc tách dữ liệu | 3 bước Nguồn → Mô hình đọc → Ghi kết quả (nút bước rộng đều) + thẻ theo bước | `TabBocTach::ve` hiện có, chỉnh bề rộng |

- Mô tả 1 dòng dưới tên chức năng (theo prototype).
- Biểu tượng (i) giữ tooltip hiện có.
- Bỏ "Xoá cache" / "Dọn dữ liệu cũ" khỏi hàng tab (chuyển giai đoạn 4).

## File
- Sửa: `gui_tach/hop_cau_hinh.rs` (`tab_dem`, `tab_tach`, `tab_muc_luc`: `nhom()` → thẻ mới, cột nhãn 200), `gui_boc_tach/cau_hinh.rs` (bỏ giả định 780 px), `gui_tach/thuoc_kho.rs` (bề rộng theo tham số nếu đang cố định)

## Các bước
1. Hàm thẻ / dòng chung trong `gui_cau_hinh/khung.rs` (`the(ui, tieu_de, tag, |ui|)`, `dong(ui, nhan, |ui|, phu)`).
2. Chuyển từng tab, chụp ảnh so prototype sau mỗi tab.
3. Đếm: thêm kết quả "Thử cạnh dài → A4 / A0 lớn" cạnh ô (đã có hàm phân khổ `xac_dinh_kho_a`).

## Tiêu chí xong
- [ ] Test hiện có của từng tab (mặc định, ô sai, thước, Bóc tách chọn nhà cung cấp / đặt mặc định) vẫn qua.
- [ ] Ảnh `cau_hinh_dem / tach / muc_luc / boc_tach` khớp `anh-dem`, `anh-tach`, `anh-ml`, `anh-bt-doc`, `anh-bt-ghi` về bố cục.

## Rủi ro
- Thước khổ đang tính toạ độ theo bề rộng hộp 560 → kiểm lại vùng bắt chuột tay kéo sau khi giãn.
