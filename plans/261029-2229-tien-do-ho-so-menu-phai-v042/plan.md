---
status: completed
created: 2026-10-29
---
# Tiến độ từng hồ sơ, nhảy trang cuối, menu chuột phải, PgUp/PgDn → v0.4.2

Nguồn chuẩn: `../261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html` (bản 29/10 22:30).

## Phase 1 — Tiến độ hồ sơ khi Phân tích (cột trái)
- Đếm trang đã xong / cần phân tích theo từng hồ sơ (từ sự kiện kết quả của hàng đợi).
- Hồ sơ đang dở: vạch xanh 2 px dưới chân dòng dài theo %, cả khi cột thu gọn; xong thì mất.
- Hồ sơ đang xử lý (vừa có trang xong gần nhất) ra ngoài vùng nhìn → cột trái tự cuộn tới; người dùng cuộn / bấm cột trái → tạm dừng 5 s.

## Phase 2 — Nhảy tới trang cuối khi hồ sơ xong (lưới Hồ sơ)
- Hồ sơ phân tích xong → lưới cuộn tới ảnh cuối của hồ sơ đó (giữa vùng nhìn); người dùng vừa cuộn / bấm / gõ phím trên lưới → tạm dừng 5 s; đang lọc 1 hồ sơ thì chỉ khi chính hồ sơ đó xong.

## Phase 3 — Menu chuột phải
- Icon nét mảnh đầu mỗi mục (bảng icon theo prototype), ✓ cuối dòng, đệm dòng 3 px, rộng tối thiểu 190 px.

## Phase 4 — PageUp / PageDown tab Hồ sơ
- Dời tiêu điểm 1 vùng nhìn (số hàng thấy được × số cột), Shift chọn dải, Ctrl chỉ dời tiêu điểm. Hướng dẫn thêm dòng phím.

## Phase 5 — Test, phát hành v0.4.2
- Unit test: đếm tiến độ hồ sơ, bước PgUp/PgDn; cargo test toàn bộ; ảnh kittest; tag v0.4.2, theo dõi CI.

## Đã làm trước
- Bỏ tô nền / vạch đầu dòng hồ sơ đang xem (chỉ dòng chọn có nền).

## Thêm trong lúc làm
- Hộp phóng to 1 trang: nhóm S│I│B và 90│180│Xoá + phím tắt S I B C R Shift+R Del; viền theo nhãn như ô lưới.
