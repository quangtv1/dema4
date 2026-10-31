---
phase: 1
title: "Lõi bóc tách (không giao diện)"
status: completed
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
- Hai bộ gọi, `temperature 0`, `max_tokens` từ Cấu hình, **mỗi yêu cầu 1 ảnh trang**:
  - Chuẩn OpenAI `POST {địa chỉ}/chat/completions` — `content:[{type:text}, {type:image_url, image_url:{url:"data:image/jpeg;base64,…"}}]`; `Authorization: Bearer` chỉ khi có khoá (vLLM nội bộ thường không cần). `GET {địa chỉ}/models` liệt kê model + kiểm tra kết nối (Gemini trả tên có tiền tố `models/` → bỏ khi hiển thị/gửi). Dùng cho Nội bộ, ChatGPT, OpenRouter, Gemini.
  - Anthropic `POST https://api.anthropic.com/v1/messages` — `x-api-key`, `anthropic-version: 2023-06-01`, ảnh `{type:image, source:{type:base64, media_type:image/jpeg, data}}`; `GET /v1/models` để liệt kê. Dùng cho Claude.
- Đọc nhiều trang (đầu+cuối / mọi trang) = nhiều yêu cầu, mỗi yêu cầu kèm câu lệnh "TRANG ĐẦU/CUỐI (i/n)", rồi gộp.
- Mặc định song song theo nhà cung cấp: Nội bộ 1 (máy thử nghiệm yếu, tăng trong Nâng cao), dịch vụ đám mây 2. HTTP 429 / 503 → chờ theo `Retry-After` (không có: 2, 4, 8 s), tối đa 3 lần. 401/403 → dừng hàng đợi, báo "Khoá API sai hoặc hết quyền". 400 có "image" → báo "Máy chủ / model không nhận ảnh này".
- Đọc JSON lỏng (bỏ ```json, lấy khối {…} đầu tiên). Gộp nhiều trang: trường lấy trang đầu, trừ trường mô tả nhắc "trang cuối" lấy trang cuối (hoặc trường đánh dấu trang cuối trong loại — quyết khi làm, ghi rõ).
- Chuẩn hoá: NFC; Ngày → dd/mm/yyyy (chấp nhận d/m/yyyy, "ngày x tháng y năm z"); Danh sách → khớp không dấu / không hoa với 1 giá trị, không khớp → trống; hoa chữ đầu; in hoa.
- Chạy song song N yêu cầu (Cấu hình), thời gian chờ, thử lại 1 lần; lỗi kết nối 3 file liên tiếp → dừng hàng đợi, báo.
- Hủy giữa chừng: dừng sau file đang đọc.

## Kiến trúc
- `src/boc_tach.rs` (gốc module) + `src/boc_tach/`: `loai_tai_lieu.rs` (mô hình + có sẵn + lưu/đọc), `anh.rs` (render + JPEG), `cau_lenh.rs`, `mo_hinh.rs` (trait `MoHinhDoc` + `ChuanOpenAi` + `Anthropic`; mô hình giả cho test), `doc_json.rs`, `chuan_hoa.rs`, `chay.rs` (hàng đợi, luồng nền, kênh sự kiện `SuKien::{BatDau(i), Xong(i, KetQua), Loi(i, String), HetHang}` giống Sinh mục lục).
- JPEG: thêm crate mã hoá JPEG nhẹ (vd `jpeg-encoder`, thuần Rust) — kiểm tra giấy phép, kích thước nhị phân; dự phòng PNG nếu không muốn thêm crate.

## File liên quan
- Tạo: `src/boc_tach.rs`, `src/boc_tach/*.rs`, `tests_data/boc_tach/` (PDF tổng hợp, JSON trả về mẫu).
- Sửa: `src/main.rs` (mod), `Cargo.toml` (crate JPEG nếu dùng).

## Các bước
1. Kiểu dữ liệu loại tài liệu + 3 loại có sẵn + lưu/đọc JSON (test vòng ghi-đọc).
2. Câu lệnh (test so khớp chuỗi với nội dung popup prototype).
3. Render + JPEG (test kích thước ảnh ≤ cạnh tối đa, đọc lại được).
4. `doc_json` + `chuan_hoa` (test bảng ví dụ: JSON lẫn chữ, ngày nhiều dạng, Danh sách có/không dấu).
5. `ChuanOpenAi` + `Anthropic`; test bằng máy chủ HTTP giả chạy trong test (luồng `std::net::TcpListener`): thân yêu cầu (đúng 1 ảnh, temperature), header (`Authorization` có/không, `x-api-key` + `anthropic-version`), `/models`, thời gian chờ, 429 + `Retry-After`, 401 dừng hàng đợi.
6. Hàng đợi chạy song song + hủy + dừng khi lỗi liên tiếp.
7. Test `#[ignore]` gọi thật (biến `DEMA4_BT_NCC` = noibo|openai|gemini|claude|openrouter, `DEMA4_BT_DIA_CHI`, `DEMA4_BT_KHOA`, `DEMA4_BT_MODEL`, `DEMA4_BT_PDF`): 1 file mẫu, in JSON đã chuẩn hoá + thời gian. Địa chỉ / khoá chỉ qua biến môi trường, không ghi file.

## Tiêu chí xong
- [ ] Test đơn vị mọi bước trên, không cần mạng thật.
- [ ] Test gọi thật (bước 7) qua máy chủ vLLM nội bộ (Vintern-1B) ra đủ trường; các dịch vụ đám mây thử khi có khoá.

## Rủi ro
- Mô hình trả JSON hỏng → `doc_json` lỏng + ghi thô vào log jsonl để soát.
- Gemini qua địa chỉ tương thích OpenAI có khác biệt nhỏ (tham số không hỗ trợ, tên model có tiền tố `models/` trong `/models`) → bỏ tiền tố khi hiển thị/gửi; chỉ gửi tham số tối thiểu (không `response_format`); test với khoá thật ở bước 7.
- Gói miễn phí Gemini giới hạn số yêu cầu/phút → song song 2 + xử lý 429; nhắc trong Cấu hình.
- Gọi thử thật (2026-11-01, vLLM + Vintern-1B, 1 trang ~1600 px): 0,9 s, 942 token vào / 129 ra, JSON hợp lệ. Lỗi quan sát được — `chuan_hoa` phải xử lý:
  - Lặp lại nhãn mô tả trong giá trị ("Loại văn bản: Quyết định") → bỏ tiền tố trùng phần mô tả trước dấu ":".
  - Số ký hiệu kèm chữ "Số " ("Số 36/BC-NTTS") → bỏ tiền tố "Số" cho trường có biến `so_ky_hieu` / mô tả chứa "số và ký hiệu".
  - Ngày bằng chữ ("6 tháng 4 năm 2015") → dd/mm/yyyy (đã có trong yêu cầu).
  - Chọn sai giá trị Danh sách (văn bản "BC" bị gán "Quyết định") → mô hình 1B yếu; ghi nhận, không sửa bằng luật đoán trong bản đầu.
- Máy chủ LAN trả model không đọc được ảnh → báo lỗi rõ ở Thử đọc ("Model không nhận ảnh").
- Ảnh lớn tốn token → mặc định 200 DPI, cạnh 2048 px, JPEG 90 như OCR36.
