---
status: in-progress
created: 2026-10-29
---
# Cập nhật app theo prototype chiều 29/10 → v0.4.0

Nguồn chuẩn: `../261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html` (bản 29/10 tối).
So bằng ảnh chụp prototype (Chrome headless 1.5x) với ảnh kittest của app.

## Phase 1 — Thanh menu, icon, Thông tin
- Icon đầu mục menu: Chức năng (lưới 4 ô), Cấu hình (bánh răng), Trợ giúp (dấu hỏi tròn).
- Icon app mới `icon.png` / `icon.ico` (đã thay file, chỉ cần build).
- Thông tin: logo Sao Mai (bỏ slogan, `assets/logo-sao-mai-300.png`) + dòng Website saomaiso.com (mở trình duyệt).

## Phase 2 — Thanh công cụ tab Hồ sơ
- Nút lọc hẹp (128 px), menu lọc 156 px; mục menu có icon (chồng trang / mã vạch / trang trống / trang ? / xoay), ✓ ở cuối dòng.
- Bố cục: `[Tất cả ▾][Điểm cắt] | (S│I│B) (↶│↷│🗑)` … `↻ ↷180 | − % + | ⇅ Sắp xếp ▾`.
- 2 nhóm nền #EDF1F6 bo 4, nút không viền 26×20, vạch ngăn mảnh; nhãn đang gắn = nền #0067C0 chữ trắng, một phần = viền xanh.
- Hoàn tác/Làm lại: mũi tên chữ U nét mảnh; Xoá trang = icon thùng rác (bỏ chữ). Bỏ "N chọn"; nhãn sắp xếp mặc định "Sắp xếp".
- Menu Sắp xếp: icon (danh sách số / tăng / giảm), ✓ cuối dòng.
- Dòng trạng thái dưới lưới: bỏ Hoàn tác/Làm lại.

## Phase 3 — Cột trái
- Dải đáy 30 px (#FAFAFA, viền trên): ô tìm (chìm, chỉ nổi khi bấm / đang có chữ) + nút thu gọn.
- Tìm theo tên (không phân biệt hoa thường), mỗi tab 1 chuỗi; "Tất cả" luôn hiện; không khớp → dòng báo.
- Thu gọn: còn cột # 36 px (∑ cho Tất cả), không cuộn tay, tự cuộn tới hồ sơ/văn bản đang xem; mở app luôn mở rộng (không lưu cfg).
- Đánh dấu hồ sơ chứa trang đang xem (vạch xanh trái + nền #EEF4FB) và tự cuộn tới, khi xem Tất cả.
- Mọi cột số canh giữa, chỉ cột tên canh trái.

## Phase 4 — Tab Văn bản
- Bỏ "N chọn" ở thanh điều hướng → dòng đáy "Bỏ trắng: … · Đã chọn: n trang".
- Nút Bỏ khỏi văn bản: icon trang giấy gạch chéo.

## Phase 5 — Thu phóng, tên văn bản, thời gian phân tích
- Lưới (2 tab): nấc đến 1075 px = 250 % (100 % = 430 px của hộp phóng to), co vừa bề ngang khung; nhãn % giữa −/+; mặc định 110 px = 26 %.
- 1/2 trang: thêm 175–250 %.
- Mẫu tên mặc định `{hoso}.{n}` (Hướng dẫn: `<hồ sơ>.1.pdf`).
- Hàng tab: đang chạy "Đang phân tích xy% (hh:mm:ss)" (đếm sống), xong "Phân tích xong (hh:mm:ss)", dừng "Đã dừng ở x% (hh:mm:ss)".
- Chọn thư mục (chưa Phân tích): cột trái hiện ngay tên các PDF (+ số trang nếu đã có cache); cột khác điền dần khi Phân tích.

## Phase 6 — Kiểm tra, phát hành v0.4.0
- cargo test toàn bộ + ảnh kittest so prototype; Cargo version 0.4.0; README nếu đổi cfg.
- Commit, push, tag v0.4.0, theo dõi CI.

## Ngoài phạm vi
- Menu chuột phải (giữ như cũ).
