---
title: "Bóc tách dữ liệu — chức năng thứ 4 (từ prototype đã duyệt)"
status: pending
created: 2026-10-31
blockedBy: []
blocks: []
---

# Bóc tách dữ liệu

Đọc văn bản PDF bằng mô hình đọc ảnh (VLM) → trích các trường (số ký hiệu, ngày, trích yếu, tác giả, người ký…) → ghi Excel + Google Sheet, dùng làm file biên mục cho Sinh mục lục. Ý tưởng gốc: repo OCR36 (Python/PyQt) — viết lại bằng Rust trong dema4.

## Nguồn sự thật giao diện (đã duyệt, giữ nguyên)

- Prototype: `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html` — màn `#manBocTach`, popup `phuSuaLoai` · `phuCauLenh` · `phuXnXoa` · `phuXnLuuMoi` · `phuBtXong`, Cấu hình tab `tabchBocTach`.
- Trạng thái mẫu (hash): `#boc-tach` (giới thiệu 3 bước) · `#boc-tach-truong-b` / `-b-thu` (Trường dữ liệu, trước/sau Thử đọc) · `#boc-tach-cau-lenh` · `#boc-tach-sua-loai` · `#boc-tach-xoa-loai` · `#boc-tach-luu-loai-moi` · `#boc-tach-ket-qua` · `#boc-tach-dang-chay` · `#boc-tach-loi` · `#boc-tach-loc` · `#boc-tach-gon` · `#boc-tach-xong` · `#cau-hinh-boc-tach` · `#cau-hinh-boc-tach-gemini` · `#cau-hinh-boc-tach-nguon`.
- Ảnh chụp: `plans/261031-1958-boc-tach-du-lieu/anh-prototype/`.
- Mọi thay đổi giao diện phát sinh khi code: sửa prototype trước, chủ sản phẩm duyệt, rồi mới sửa app (quy trình dự án).

## Đã có sẵn trong app

Trang chủ 7 ô (Bóc tách đang để "Đang phát triển"), menu, màn giới thiệu `gioi_thieu::BOC_TACH`, logo `Logo::BocTach`, icon `BieuTuong::BocTach`. Hạ tầng dùng lại: MuPDF render trang, `ureq`, `serde_json`, `rsa`+`sha2`, `calamine`, mẫu luồng nền + kênh sự kiện của Tách / Sinh mục lục, license hạn mức chung, trạm Apps Script, hộp Cấu hình nhiều tab.

## Giai đoạn

| # | Giai đoạn | File | Phụ thuộc | Trạng thái |
|---|---|---|---|---|
| 1 | Lõi bóc tách (không giao diện): loại tài liệu, render ảnh, câu lệnh, gọi mô hình, gộp + chuẩn hoá | [phase-01-loi-boc-tach.md](phase-01-loi-boc-tach.md) | — | pending |
| 2 | Cấu hình › Bóc tách + khoá + kiểm tra kết nối | [phase-02-cau-hinh.md](phase-02-cau-hinh.md) | 1 | pending |
| 3 | Khung màn hình + tab Trường dữ liệu + popup | [phase-03-man-truong-du-lieu.md](phase-03-man-truong-du-lieu.md) | 1, 2 | pending |
| 4 | Tab Kết quả bóc + chạy hàng loạt + popup xong | [phase-04-ket-qua-boc.md](phase-04-ket-qua-boc.md) | 3 | pending |
| 5 | Đầu ra: Excel + Google Sheet | [phase-05-dau-ra.md](phase-05-dau-ra.md) | 1 | pending |
| 6 | License, thống kê, trạm, Trang chủ, tài liệu, phát hành | [phase-06-tich-hop-phat-hanh.md](phase-06-tich-hop-phat-hanh.md) | 3–5 | pending |
| 7 | Nguồn đám mây: S3/GCS, Google Drive (sau bản đầu) | [phase-07-nguon-dam-may.md](phase-07-nguon-dam-may.md) | 6 | pending |

Bản phát hành đầu = giai đoạn 1–6 (nguồn Máy tính; mô hình chuẩn OpenAI + Claude; Excel + Sheet). Giai đoạn 7 làm sau.

## Tiêu chí hoàn thành bản đầu

- Giao diện khớp prototype đã duyệt ở mọi trạng thái mẫu (so ảnh chụp kittest với ảnh prototype).
- Chạy thật 33 file mẫu với một máy chủ chuẩn OpenAI (vd Ollama/vLLM nội bộ) và một dịch vụ đám mây: đúng số trường, đúng file lỗi, chạy lại theo bộ lọc chỉ chạy file đang hiện.
- Excel mở được, cột ngày/số ký hiệu dạng văn bản; file Excel nạp thẳng vào Sinh mục lục, biến tự ghép.
- Sheet nhận 1 dòng sau mỗi file; mất mạng giữa chừng không mất kết quả đã đọc.
- Khoá API / secret / khoá service account không xuất hiện trong log, thống kê, Telegram, repo.
- `cargo test --release` qua hết, `cargo build --release` 0 cảnh báo, test trạm Node qua.

## Quyết định đã chốt (chủ sản phẩm, 2026-11-01)

1. **Không tô vùng trên PDF** ở bản đầu — chỉ hiện trang. Không xin toạ độ trong câu lệnh.
2. **Khoá API lưu trong cache, có mã hoá, khoá giải mã chỉ chủ sản phẩm biết:** AES-256-GCM (crate `aes-gcm`), khoá = SHA-256(bí mật lúc build ‖ `license::ma_may()`). Bí mật lúc build lấy từ biến môi trường `DEMA4_KHOA_CACHE` qua `option_env!` — đặt trong secret CI, KHÔNG nằm trong repo; build thiếu biến → không lưu khoá (chỉ giữ trong RAM phiên đó) và hiện nhắc. Gắn mã máy để chép file cache sang máy khác không giải mã được. Giới hạn: bí mật nằm trong file chạy nên người cố tình dịch ngược vẫn lấy được — đủ chống đọc trộm file cache / lộ khi gửi file cấu hình, không chống được người có quyền trên máy.
3. **Đếm lượt:** mỗi lần Bắt đầu / Chạy lại = 1 lượt; **Thử đọc không trừ lượt**.
4. Câu "Xử lý ngay trên máy" (Trang chủ) và câu thống kê trong README: sửa đúng như prototype khi Bóc tách mở dùng (giai đoạn 6).
5. Mô hình nội bộ: chạy qua Ollama / LM Studio / llama.cpp (địa chỉ chuẩn OpenAI), không nhúng mô hình vào app. Ứng viên: Qwen3-VL-4B (mặc định đề xuất), Qwen3-VL-2B / Vintern-1B-v3.5 cho máy yếu — chốt sau khi đo ([báo cáo](../reports/research-261101-0027-mo-hinh-thi-giac-chay-local-boc-tach-report.md)).
