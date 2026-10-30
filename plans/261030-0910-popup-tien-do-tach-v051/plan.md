---
status: completed
---
# v0.5.1 — Popup tiến độ tách, mẫu tên đệm số 0

## Việc đã làm
1. Prototype rồi app: bấm Tách mở popup "Đang tách" (thanh tiến độ hồ sơ · văn bản · %, văn bản đang tạo, đã xong, lỗi, đã chạy / còn ~ / xong lúc, tốc độ, Dừng). Luồng tách báo sự kiện từng văn bản / hồ sơ (`SuKienTach`).
2. Header: "Đang tách…" → "Đã tách xong (hh:mm:ss)" / "Đã dừng tách (…)", bỏ vạch tiến độ khi tách.
3. Mẫu tên văn bản nhận `{n1}`…`{n9}` đệm số 0 (`{n7}` → 0000001); dọn file cũ nhận dạng đúng.

## Kiểm thử
- `bao_tien_do_tung_van_ban_va_ho_so`, test tên `{n7}`, trạng thái "Đã tách xong" trong test tách GUI; ảnh `dang_tach.png` (chup_hop). 141 test.
