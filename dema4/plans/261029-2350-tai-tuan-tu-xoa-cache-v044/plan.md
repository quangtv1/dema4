---
status: completed
---
# v0.4.4 — Đọc danh sách theo cây, hiển thị tuần tự, Xoá cache

## Việc đã làm
1. Duyệt thư mục bằng jwalk (đọc song song, ra đúng thứ tự): PDF thư mục cha → các thư mục con (tên tự nhiên); đẩy từng lô theo thư mục. Vạch chạy dưới tiêu đề bảng khi đang đọc.
2. Phân tích nhiều luồng nhưng hiển thị tuần tự: `XepThuTu` (hang_doi.rs) giữ kết quả về sớm tới lượt (hồ sơ, trang); trang bị xoá không chặn hàng.
3. `TienDoHoSo` mốc đơn điệu: hồ sơ đang xử lý chỉ tiến; báo xong đúng thứ tự → nhảy trang cuối từng hồ sơ.
4. Nút "Xoá cache" trong hộp Cấu hình (cân phải hàng tab), mờ khi không có cache.

## Kiểm thử
- `duyet_cay_pdf_truoc_roi_thu_muc_con_ten_tu_nhien`, `tien_do_ho_so_moc_chi_tien_va_bao_xong_dung_thu_tu`, `phan_tich_lan_dau_theo_thu_tu_cot_trai`, `xep_thu_tu_dua_ra_lan_luot_trang_bo_khong_chan`.
- Toàn bộ `cargo test --release` + ảnh chụp kittest.
