---
phase: 1
title: "Lõi bóc tách (không giao diện)"
status: pending
priority: P1
dependencies: []
---

# Giai đoạn 1: Lõi bóc tách

## Tổng quan
Module thuần Rust, test được không cần GUI: mô tả loại tài liệu, render trang → ảnh, dựng câu lệnh, gọi mô hình đọc, đọc JSON, gộp theo trường, chuẩn hoá. Tương đương `pipeline.py` của OCR36.

## Yêu cầu
- Loại tài liệu = tên, trang đọc mặc định (đầu / đầu+cuối / mọi trang), hướng dẫn chung, danh sách trường. Trường = tên cột, biến (tự sinh từ tên cột), mô tả gửi mô hình (ghi trang trong mô tả), chuẩn hoá (Giữ nguyên · Danh sách · Ngày dd/mm/yyyy · Viết hoa chữ đầu · Chữ in hoa), danh sách giá trị (khi Danh sách), bắt buộc.
- 3 loại có sẵn: Văn bản hành chính (6 trường như prototype), Hồ sơ đất đai, Hợp đồng; người dùng thêm / sửa / xoá / lưu thành loại mới. Lưu JSON trong `.sm-dema4/boc_tach/loai_tai_lieu.json`.
- Render trang chọn theo loại → JPEG (DPI, cạnh tối đa, chất lượng lấy từ Cấu hình) → base64 data URL.
- Câu lệnh: hướng dẫn chung + "Đây là TRANG ĐẦU/CUỐI (i/n)…" + mẫu JSON {biến: mô tả [: a | b | c nếu Danh sách]} — đúng nội dung popup Câu lệnh của prototype.
- Bộ gọi duy nhất chuẩn OpenAI `POST {địa chỉ}/chat/completions`: `messages=[{role:user, content:[{type:text}, {type:image_url, image_url:{url:"data:image/jpeg;base64,…"}}…]}]`, `temperature 0`, `max_tokens` từ Cấu hình; `Authorization: Bearer` chỉ khi có khoá (máy chủ LAN thường không cần). `GET {địa chỉ}/models` để liệt kê model + kiểm tra kết nối.
- Mặc định theo nhà cung cấp: Nội bộ song song 4, chờ 120 s (GPU, ảnh lớn); Gemini song song 2, chờ 60 s. HTTP 429 / 503 → chờ theo `Retry-After` (không có thì 2 s, 4 s, 8 s), tối đa 3 lần, không tính là file lỗi cho tới khi hết lượt thử. 401/403 → dừng hàng đợi ngay, báo "Khoá API sai hoặc hết quyền".
- Đọc JSON lỏng (bỏ ```json, lấy khối {…} đầu tiên). Gộp nhiều trang: trường lấy trang đầu, trừ trường mô tả nhắc "trang cuối" lấy trang cuối (hoặc trường đánh dấu trang cuối trong loại — quyết khi làm, ghi rõ).
- Chuẩn hoá: NFC; Ngày → dd/mm/yyyy (chấp nhận d/m/yyyy, "ngày x tháng y năm z"); Danh sách → khớp không dấu / không hoa với 1 giá trị, không khớp → trống; hoa chữ đầu; in hoa.
- Chạy song song N yêu cầu (Cấu hình), thời gian chờ, thử lại 1 lần; lỗi kết nối 3 file liên tiếp → dừng hàng đợi, báo.
- Hủy giữa chừng: dừng sau file đang đọc.

## Kiến trúc
- `src/boc_tach.rs` (gốc module) + `src/boc_tach/`: `loai_tai_lieu.rs` (mô hình + có sẵn + lưu/đọc), `anh.rs` (render + JPEG), `cau_lenh.rs`, `mo_hinh.rs` (trait `MoHinhDoc` để test thay bằng mô hình giả + `ChuanOpenAi`), `doc_json.rs`, `chuan_hoa.rs`, `chay.rs` (hàng đợi, luồng nền, kênh sự kiện `SuKien::{BatDau(i), Xong(i, KetQua), Loi(i, String), HetHang}` giống Sinh mục lục).
- JPEG: thêm crate mã hoá JPEG nhẹ (vd `jpeg-encoder`, thuần Rust) — kiểm tra giấy phép, kích thước nhị phân; dự phòng PNG nếu không muốn thêm crate.

## File liên quan
- Tạo: `src/boc_tach.rs`, `src/boc_tach/*.rs`, `tests_data/boc_tach/` (PDF tổng hợp, JSON trả về mẫu).
- Sửa: `src/main.rs` (mod), `Cargo.toml` (crate JPEG nếu dùng).

## Các bước
1. Kiểu dữ liệu loại tài liệu + 3 loại có sẵn + lưu/đọc JSON (test vòng ghi-đọc).
2. Câu lệnh (test so khớp chuỗi với nội dung popup prototype).
3. Render + JPEG (test kích thước ảnh ≤ cạnh tối đa, đọc lại được).
4. `doc_json` + `chuan_hoa` (test bảng ví dụ: JSON lẫn chữ, ngày nhiều dạng, Danh sách có/không dấu).
5. `ChuanOpenAi`; test bằng máy chủ HTTP giả chạy trong test (luồng `std::net::TcpListener`): thân yêu cầu (ảnh data URL, temperature), có/không header `Authorization`, `/models`, thời gian chờ, 429 + `Retry-After`, 401 dừng hàng đợi.
6. Hàng đợi chạy song song + hủy + dừng khi lỗi liên tiếp.
7. Test `#[ignore]` gọi thật (biến `DEMA4_BT_DIA_CHI`, `DEMA4_BT_KHOA`, `DEMA4_BT_MODEL`, `DEMA4_BT_PDF`): 1 file mẫu tổng hợp qua máy chủ LAN và Gemini Flash, in JSON + thời gian. Khoá chỉ qua biến môi trường, không ghi file.

## Tiêu chí xong
- [ ] Test đơn vị mọi bước trên, không cần mạng thật.
- [ ] Test gọi thật (bước 7) ra đủ 6 trường với máy chủ LAN và với Gemini Flash.

## Rủi ro
- Mô hình trả JSON hỏng → `doc_json` lỏng + ghi thô vào log jsonl để soát.
- Gemini qua địa chỉ tương thích OpenAI có khác biệt nhỏ (tham số không hỗ trợ, tên model có tiền tố `models/` trong `/models`) → bỏ tiền tố khi hiển thị/gửi; chỉ gửi tham số tối thiểu (không `response_format`); test với khoá thật ở bước 7.
- Gói miễn phí Gemini giới hạn số yêu cầu/phút → song song 2 + xử lý 429; nhắc trong Cấu hình.
- Máy chủ LAN trả model không đọc được ảnh → báo lỗi rõ ở Thử đọc ("Model không nhận ảnh").
- Ảnh lớn tốn token → mặc định 200 DPI, cạnh 2048 px, JPEG 90 như OCR36.
