---
phase: 5
title: "Chạy hàng loạt, license, trạm, trạng thái"
status: completed
priority: P2
dependencies: [1, 3, 4]
---

# Phase 5: Chạy hàng loạt, license, trạm, trạng thái

## Requirements
- Chạy cả thư mục (tiến độ theo trang + ước lượng thời gian, Dừng); popup kết quả: Mở thư mục, Mở ở Tách.
- Trạng thái từng file (kiểm theo giai đoạn 3): đã xóa / chưa / cấu hình đã đổi / kết quả mất; xem nhật ký trong app; tuỳ chọn chạy lại chỉ file chưa xóa / đổi cấu hình.
- **License giữ nguyên cách đang chạy** cho Đếm / Tách / Mục lục / Bóc tách (không đổi cột bảng, hạn mức, chữ ký, giới hạn 2 lượt offline); Xóa ghim chỉ **thêm vào**: `ChucNang::XoaGhim` (`ma` = `"xoa_ghim"`); 1 lần chạy có kết quả = 1 lượt, hoàn lượt khi không ra file nào.
- **Trạm (Apps Script)** — triển khai **trước** khi phát hành app (red team #11): `dema4_rs/apps-script/loi.js` thêm `xoa_ghim` vào `TEN_CN` (**không thêm cột**: lượt tính chung 1 tổng hạn mức / đã dùng cho mọi chức năng — `loi.js:14,89,98`), hành động `thong_ke_xoa_ghim` + hàm báo Telegram riêng (không rơi vào nhánh Tách); test apps-script.
- Số liệu gửi trạm `so_lieu_xoa_ghim`: **chỉ số đếm** (số file, số trang, trang đã xử lý, AI bật / tắt, thời gian) — không đường dẫn, không tên file.
- Làm tiếp gần đây: `gan_day::ChucNang::XoaGhim` (an toàn với 2.4 nhờ giai đoạn 1).

## Related Code Files
- Modify: `dema4_rs/src/license.rs`, `dema4_rs/src/tram.rs`, `dema4_rs/src/gan_day.rs`, `dema4_rs/src/gui.rs`, `dema4_rs/apps-script/loi.js` (+ `dema4_rs/apps-script/test`)

## Tests
Chạy thư mục nhỏ → ra file + nhật ký; trừ / hoàn lượt; dừng giữa chừng không sót `.tmp`; trạng thái đúng sau đổi cấu hình / xoá thư mục ra; số liệu trạm không chứa đường dẫn / tên file; Làm tiếp gần đây mở lại đúng.

## Success Criteria
- [ ] Làm trọn 1 thư mục scan có ghim chỉ trong app; trạm ghi đúng lượt + Telegram báo đúng "Xóa ghim".

## Kết quả (2026-11-26)
- App: `license::ChucNang::XoaGhim` (`xoa_ghim`), `gan_day::ChucNang::XoaGhim` (mở lại đúng thư mục); `tram::so_lieu_xoa_ghim` chỉ số đếm (file, ok, lỗi, trang, trang xử lý, số vùng, DPI, AI, thời gian); `xoa_ghim/lo.rs` chạy cả thư mục trên pool riêng + đọc trạng thái từng file; màn: nút **Xóa ghim (n file)** (file chưa xoá / đổi cấu hình / mất kết quả) + menu *Chạy lại cả thư mục*, hộp tiến độ (file, trang, còn ~, Dừng sau trang đang làm), hộp kết quả (Mở thư mục, Mở ở Tách), chấm trạng thái từng file (xanh / cam / đỏ / xám); 1 lượt khi ≥ 1 file ra, không ra file → hoàn lượt; hết lượt → hộp license, được cấp thêm thì chạy tiếp; đóng app khi đang chạy → hỏi.
- Thử trọn bằng harness trên 3 file mẫu: 3/3 file, 8 trang, 0,8 s, trạng thái xanh.
- Trạm `apps-script/loi.js`: `xoa_ghim` trong `TEN_CN` (không thêm cột — lượt chung hạn mức), hành động `thong_ke_xoa_ghim` + tin `🧷 XÓA GHIM`; test Node 20/20 (`node --test test/loi.test.js`). App 2.4.x đọc dữ liệu ký theo JSON động → khoá `han_xoa_ghim` mới không ảnh hưởng.
- **Trạm đã triển khai (2026-11-26)**: `clasp pull` bản đang chạy = repo trước sửa (không lệch) → `clasp push --force` → `clasp update-deployment` giữ URL, bản @16. Kiểm không ghi Sheet: `thong_ke_xoa_ghim` + mã máy sai → `ma_may` (đã qua danh sách hành động); hành động lạ → `hanh_dong`. Script ID dự án trạm: `1UUOAuiNawSeEwJ9ynlzZtgxBTFpfLoGkEVPMPfDwGs7Y2PIge8711tMD` (dự án gắn Sheet — `clasp list` không thấy).
