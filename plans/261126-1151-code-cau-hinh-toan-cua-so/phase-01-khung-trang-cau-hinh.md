---
phase: 1
title: "Khung trang Cấu hình"
status: completed
priority: P1
dependencies: []
---

# Giai đoạn 1: Khung trang Cấu hình

## Tổng quan
Thay popup `kh::hop` bằng trang Cấu hình: toàn cửa sổ (mặc định) hoặc cửa sổ thu nhỏ 1120 × 720; sidebar trái 240 / 56 px; hàng tiêu đề chức năng có nút; gắn vào `gui.rs`.

## Bảng thông số

| Thành phần | Giá trị (từ CSS prototype) |
|---|---|
| Sidebar | rộng 240 (thu nhỏ 56), nền `#f7f8fa`, viền phải `#dcdfe3` |
| Dòng "Quay lại" | cao 44, chữ `#1f5fbf`, "Quay lại **<màn>**", viền dưới `#e3e5e8`; nút thu nhỏ 40 × 44 bên phải |
| Nhóm | "CHỨC NĂNG" / "ỨNG DỤNG" 11 px chữ hoa, `#6b7280`, đệm 16/16/8 (thu nhỏ: vạch 1 px) |
| Mục | cao 40, lề 8, bo 5, biểu tượng 16 + chữ 13; hover `#eceef1`; chọn nền `#dcebfa` chữ `#1f5fbf` đậm + vạch trái 3 px `#2563eb`; chấm cam 7 px `#f59e0b` bên phải (thu nhỏ: góc trên phải) |
| Chú thích đáy sidebar | 11,5 px `#6b7280`, viền trên (ẩn khi thu nhỏ) |
| Vùng nội dung | nền `#f3f4f6`; cột **880** giữa (cửa sổ thu nhỏ: lề 20) |
| Tiêu đề | đệm 18/4/14, biểu tượng 16 xanh + tên 18 semibold, mô tả 12 `#6b7280`, viền dưới `#e3e5e8`; bên phải: link "Khôi phục mặc định" · nhãn chưa lưu (nền `#fff7ed`, chữ `#b45309`, viền `#fed7aa`; đã lưu: `#f0fdf4`/`#15803d`/`#bbf7d0`, bo 10) · Huỷ thay đổi · **Lưu** (xanh `#2563eb`) · nút cửa sổ 28 × 26 không viền |
| Thẻ | nền trắng, viền `#e5e7eb`, bo 6, đệm 12/16/8, tiêu đề 13,5 semibold, khoảng cách thẻ 14 |
| Dòng nhãn | lưới 200 px + phần còn lại, kẻ `#f0f1f3`, đệm 8 |
| Cửa sổ thu nhỏ | 1120 × 720 (≤ màn − 40), bo 8, bóng lớn, nền sau `rgba(17,24,39,.32)`; thanh tiêu đề 36 nền `#f7f8fa`: ⚙ Cấu hình … ✕ |
| Biểu tượng | module: tài liệu (Đếm), kéo (Tách), danh sách (Mục lục), khung chữ (Bóc tách), ghim (Xóa ghim), CSDL (Dữ liệu); cửa sổ: ⧉ 2 ô chồng (→ thu nhỏ) / □ (→ toàn cửa sổ); sidebar: khung có cột trái (tô đặc khi thu nhỏ) — SVG lấy nguyên từ prototype |

## Yêu cầu
- Mở từ menu **Cấu hình** hoặc nút "tải mô hình" / "Mở Cấu hình" ở các màn: chọn sẵn chức năng của màn hiện tại (Trang chủ → Đếm trang A4).
- Đang mở: nút menu **Cấu hình** sáng (`#dcebfa`); bấm lại không làm gì. Đang chạy việc: giữ khoá như hiện nay.
- Esc / Quay lại / ✕ / bấm nền mờ → đóng (giai đoạn 2 thêm hỏi khi chưa lưu). Ctrl+M đổi kiểu cửa sổ, Ctrl+B thu sidebar; nhớ cả 2 (cấu hình app `ch_cua_so_nho`, `ch_thu_ds`).
- Toàn cửa sổ: màn chức năng **không vẽ**; cửa sổ thu nhỏ: màn vẽ dưới lớp mờ, không nhận chuột / phím (lớp `Area` chặn).
- Tiêu đề cửa sổ hệ điều hành: "DEMA4 Sao Mai — Cấu hình".

## Kiến trúc
- Mới `gui_cau_hinh.rs` + `gui_cau_hinh/{khung.rs, phan.rs, du_lieu.rs, bieu_tuong.rs}`; dùng lại `gui_xoa_ghim::widget` (nút, tooltip) và `thiet_ke` (màu) — chuyển 2 module này lên dùng chung nếu cần (`pub(crate)`).
- `HopCauHinh` giữ trạng thái nháp + tab; thêm `kieu: CuaSo { Toan, Nho }`, `thu_ds: bool`, `tu: Man` (màn quay lại).
- `TabCauHinh` thêm `DuLieu`; thứ tự sidebar = thứ tự enum.
- `ve(&mut self, ui) -> KetQuaTrang` (toàn cửa sổ, vẽ trong `CentralPanel` của app) và `ve_cua_so(&mut self, ctx) -> KetQuaTrang` (lớp mờ + `Area`).

## File
- Tạo: `dema4_rs/src/gui_cau_hinh.rs`, `dema4_rs/src/gui_cau_hinh/khung.rs`, `.../bieu_tuong.rs`
- Sửa: `dema4_rs/src/gui_tach/hop_cau_hinh.rs` (bỏ `kh::hop`, thân tab gọi từ khung mới), `dema4_rs/src/gui.rs` (mở / vẽ / đóng, menu sáng, khoá F1 khi đang sửa ô), `dema4_rs/src/cau_hinh.rs` (2 khoá mới + test đọc / ghi), `dema4_rs/src/main.rs` (khai báo module)

## Các bước
1. Thêm 2 trường + khoá cấu hình; test đọc / ghi.
2. Viết `khung.rs`: sidebar (đủ / thu nhỏ), hàng tiêu đề (nhận closure vẽ nút), thân cuộn dọc cột 880, cửa sổ thu nhỏ.
3. `HopCauHinh::ve*` dùng khung; tạm giữ nút Lưu / Huỷ (giai đoạn 2 đổi logic).
4. `gui.rs`: nhánh vẽ theo `hop_cau_hinh`; bỏ đóng khi Lưu; menu Cấu hình sáng; tiêu đề cửa sổ; phím Esc / Ctrl+M / Ctrl+B (bỏ qua khi đang gõ ô).
5. Accessibility: mục sidebar `Role::Tab` + selected; nút cửa sổ / thu nhỏ có tên.

## Tiêu chí xong
- [ ] Mở từ 6 màn đều vào đúng chức năng; Quay lại về đúng màn.
- [ ] Ảnh 1600 × 900 khung (sidebar, tiêu đề, cột 880) trùng bố cục `anh-tach.png`, `anh-cua-so-nho.png`, `anh-thu-nho.png` (≤ 2 px).
- [ ] Đổi chức năng không làm cột nội dung đổi bề rộng.

## Rủi ro
- `CentralPanel` của từng màn đang tự vẽ → nhánh Cấu hình phải đặt trước `match self.man` để không vẽ 2 lớp.
- Phím tắt: Ctrl+M chưa màn nào dùng; phím **B** (không Ctrl) là nhãn "Trang" ở Tách / phóng to và không kiểm Ctrl → ở cửa sổ thu nhỏ màn phía sau **không được xử lý phím** (bỏ qua `phim()` của màn khi Cấu hình đang mở) — có test: Ctrl+B khi mở Cấu hình trên màn Tách không gán nhãn.
