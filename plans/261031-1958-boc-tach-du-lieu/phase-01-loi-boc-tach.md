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
- Nhà cung cấp: (a) chuẩn OpenAI `/v1/chat/completions` (Máy chủ nội bộ, ChatGPT, Gemini qua địa chỉ tương thích OpenAI, OpenRouter); (b) Anthropic `/v1/messages` (Claude). `temperature 0`, `max_tokens` từ Cấu hình.
- Đọc JSON lỏng (bỏ ```json, lấy khối {…} đầu tiên). Gộp nhiều trang: trường lấy trang đầu, trừ trường mô tả nhắc "trang cuối" lấy trang cuối (hoặc trường đánh dấu trang cuối trong loại — quyết khi làm, ghi rõ).
- Chuẩn hoá: NFC; Ngày → dd/mm/yyyy (chấp nhận d/m/yyyy, "ngày x tháng y năm z"); Danh sách → khớp không dấu / không hoa với 1 giá trị, không khớp → trống; hoa chữ đầu; in hoa.
- Chạy song song N yêu cầu (Cấu hình), thời gian chờ, thử lại 1 lần; lỗi kết nối 3 file liên tiếp → dừng hàng đợi, báo.
- Hủy giữa chừng: dừng sau file đang đọc.

## Kiến trúc
- `src/boc_tach.rs` (gốc module) + `src/boc_tach/`: `loai_tai_lieu.rs` (mô hình + có sẵn + lưu/đọc), `anh.rs` (render + JPEG), `cau_lenh.rs`, `mo_hinh.rs` (trait `MoHinhDoc` + `ChuanOpenAi` + `Anthropic`), `doc_json.rs`, `chuan_hoa.rs`, `chay.rs` (hàng đợi, luồng nền, kênh sự kiện `SuKien::{BatDau(i), Xong(i, KetQua), Loi(i, String), HetHang}` giống Sinh mục lục).
- JPEG: thêm crate mã hoá JPEG nhẹ (vd `jpeg-encoder`, thuần Rust) — kiểm tra giấy phép, kích thước nhị phân; dự phòng PNG nếu không muốn thêm crate.

## File liên quan
- Tạo: `src/boc_tach.rs`, `src/boc_tach/*.rs`, `tests_data/boc_tach/` (PDF tổng hợp, JSON trả về mẫu).
- Sửa: `src/main.rs` (mod), `Cargo.toml` (crate JPEG nếu dùng).

## Các bước
1. Kiểu dữ liệu loại tài liệu + 3 loại có sẵn + lưu/đọc JSON (test vòng ghi-đọc).
2. Câu lệnh (test so khớp chuỗi với nội dung popup prototype).
3. Render + JPEG (test kích thước ảnh ≤ cạnh tối đa, đọc lại được).
4. `doc_json` + `chuan_hoa` (test bảng ví dụ: JSON lẫn chữ, ngày nhiều dạng, Danh sách có/không dấu).
5. `MoHinhDoc` 2 nhà cung cấp; test bằng máy chủ HTTP giả chạy trong test (luồng `std::net::TcpListener`), kiểm tra header (`Authorization: Bearer` / `x-api-key` + `anthropic-version`), thân yêu cầu, thời gian chờ, thử lại.
6. Hàng đợi chạy song song + hủy + dừng khi lỗi liên tiếp.
7. Test đo mô hình (`#[ignore]`, biến `DEMA4_DO_MO_HINH=<thư mục>`): chạy bộ trang thật đã gán nhãn tay (giữ ngoài repo) qua từng mô hình nội bộ, in độ chính xác từng trường + giây/trang → chọn mô hình nội bộ mặc định (xem quyết định #5 trong plan.md).

## Tiêu chí xong
- [ ] Test đơn vị mọi bước trên, không cần mạng thật.
- [ ] Một lệnh dòng lệnh ẩn (chỉ debug) chạy 1 file với mô hình thật để thử tay.

## Rủi ro
- Mô hình trả JSON hỏng → `doc_json` lỏng + ghi thô vào log jsonl để soát.
- Gemini qua địa chỉ tương thích OpenAI có khác biệt nhỏ (tham số không hỗ trợ) → test với khoá thật trước khi phát hành.
- Ảnh lớn tốn token → mặc định 200 DPI, cạnh 2048 px, JPEG 90 như OCR36.
