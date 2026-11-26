---
phase: 4
title: "Màn Xóa ghim — xem trước, vùng, thao tác"
status: completed
priority: P2
dependencies: [2, 3]
---

# Phase 4: Màn Xóa ghim — xem trước, vùng, thao tác

## Requirements
- Ô Trang chủ "Xóa ghim" (đang "Đang phát triển") → bật; menu Chức năng.
- Mở file / thư mục, kéo thả; danh sách file lọc theo tên / số trang, nạp số trang dần; `[` `]` đổi file; nạp trước file kế.
- Lưới trang / thumbnail, phóng to (dùng lại khung màn Tách); xem 1 trang / cuộn liên tục; vừa ngang / cao; zoom `+ - =`; giữ zoom khi đổi file; chỉ giữ ~10 trang quanh trang đang xem trong RAM.
- Xem trước **Gốc | Kết quả**, đồng bộ cuộn / zoom.
- **Xem trước = kết quả xuất** (user chốt): xử lý ở **đúng độ phân giải sẽ xuất**, cùng thuật toán / tham số, chỉ phần trong vùng rồi thu nhỏ để hiển thị; nhớ ảnh đã đọc để chỉnh ngưỡng / vùng cập nhật ngay.
- Vùng: 8 nút góc / cạnh + vẽ tay (chế độ Vẽ / Chọn, phím tắt), chỉnh rộng / cao / ngưỡng, Delete; loại xoá / bảo vệ / xoá vô đối; áp tất cả / lẻ / chẵn / 1 trang; phạm vi chung thư mục / riêng file / riêng trang; xoá vùng chung / riêng, từng trang / cả thư mục; bộ đếm vùng; hoàn tác nhiều bước (≥ 79, chỉ trong phiên làm việc).
- Tuỳ chọn: Bảo vệ nét đen (bật), Bảo vệ dấu đỏ / chữ ký xanh (tắt), nền trắng, DPI xuất (Gốc / 300 / 250 / 200 / 100 / 72), chất lượng JPEG, Nén đen trắng. (Ô AI thêm ở giai đoạn 7.)
- Vùng / tuỳ chọn lưu kho Xóa ghim (giai đoạn 3), mở lại thấy lại.
- F1: nên Xóa ghim **trước** khi gắn nhãn ở Tách; đổi tên thư mục → chọn lại vùng chung; Xóa ghim không phải công cụ che thông tin.

## Related Code Files
- Create: `dema4_rs/src/gui_xoa_ghim.rs` (+ thư mục con nếu cần)
- Modify: `dema4_rs/src/trang_chu.rs` (ô Xóa ghim + test `trang_chu.rs:506-509`), `dema4_rs/src/gui.rs` (`ChonChucNang`), `dema4_rs/src/tro_giup.rs`

## Tests
Harness: chọn / vẽ / xoá vùng → xem trước đổi; hoàn tác; vùng riêng trang chỉ áp trang đó; lẻ / chẵn; đóng mở lại app → vùng còn; **ảnh xem trước khớp từng điểm ảnh với ảnh trong PDF xuất**; đo thời gian xem trước lần đầu mỗi trang.

## Success Criteria
- [ ] Mọi thao tác vùng của XoaGhim làm được; xem trước khớp kết quả xuất.

## Tiến độ (2026-11-25)
- Xong: ô Trang chủ + menu Chức năng; Trang chủ hàng đầu 7 ô rộng hơn (tên 1 dòng, user yêu cầu); chọn thư mục, danh sách PDF (bỏ `*_xoaghim`), `[` `]` đổi file, PageUp/Down đổi trang; xem trước Gốc | Kết quả ở đúng DPI xuất (`xuat::ve_trang` + `xoa_ghim_trang` dùng chung với xuất; luồng nền, dồn yêu cầu, nhớ ảnh gốc trang); 8 vùng có sẵn bật / tắt (giữ thứ tự góc → cạnh), chỉnh rộng / cao / độ sâu / ngưỡng; vẽ vùng tay (Vẽ V / Chọn C), loại xoá / bảo vệ / vô đối, phạm vi chung thư mục / riêng file / riêng trang, mọi trang / lẻ / chẵn; chọn vùng bằng bấm, Delete, hoàn tác Ctrl+Z (100 bước, trong phiên), Xoá hết vùng vẽ chung / riêng file, bộ đếm; tuỳ chọn bảo vệ / nền trắng / DPI / nén / nén đen trắng; lưu kho ghi trễ 0,6 s (chung theo thư mục, riêng theo id qua `ids_cua`).
- Thêm (lượt 2): kéo thân / cạnh vùng vẽ để di chuyển / đổi cỡ, kéo cạnh phía trong của góc / cạnh có sẵn để đổi kích thước (con trỏ đổi theo cạnh); phóng 100–800% (nút, `+ - =`, Ctrl + cuộn), Gốc | Kết quả chung 1 vùng cuộn nên cuộn / phóng luôn đồng bộ; nút số trang (≤ 30 trang) / ô nhập trang; lọc tên file (NFC, không phân biệt hoa thường); kéo thả thư mục / PDF vào cửa sổ; báo "dữ liệu Xóa ghim đang mở ở nơi khác — chỉ xem"; F1 tab Xóa ghim (thao tác, lưu vùng, nên Xóa ghim trước Tách, không phải công cụ che thông tin); test lưu / mở lại / đổi tên file vẫn còn vùng riêng.
- Không làm (lý do): ảnh nhỏ trang + xem cuộn liên tục — xem trước ở độ phân giải xuất (~26 MB / trang 300 dpi), cuộn liên tục phải vẽ + xoá mọi trang; thay bằng nút số trang. Lọc theo số trang — cần mở từng PDF khi mở thư mục. Vùng nhập từ gói phiên: không có màn riêng — vùng nhập về hiện ngay trên màn khi mở thư mục, và chỉ dùng khi bấm chạy (GĐ5).
