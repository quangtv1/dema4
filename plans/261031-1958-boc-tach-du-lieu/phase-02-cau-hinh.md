---
phase: 2
title: "Cấu hình › Bóc tách + khoá + kiểm tra kết nối"
status: pending
priority: P1
dependencies: [1]
---

# Giai đoạn 2: Cấu hình › Bóc tách

## Tổng quan
Tab thứ 4 "Bóc tách" trong hộp Cấu hình (rộng hơn khi mở tab này), đúng prototype `#cau-hinh-boc-tach`, `-gemini`, `-nguon`.

## Yêu cầu
- Sơ đồ luồng 3 ô: Nguồn văn bản → Mô hình đọc → Ghi kết quả (ô là nút chuyển mục; dòng phụ tóm tắt lựa chọn).
- Mô hình đọc: ① 5 ô nhà cung cấp 1 hàng (Nội bộ · ChatGPT · Gemini · Claude · OpenRouter), dòng dưới = model đã cài hoặc "Chưa cài đặt" (xám nghiêng); "★ Mặc định / Đặt làm mặc định · Xoá" cuối hàng bước 1. ② Địa chỉ (tự điền + khoá ô cho dịch vụ đám mây; Nội bộ: người dùng nhập `http://<ip máy GPU>:<cổng>/v1`, gợi ý `:8000` vLLM / `:11434` Ollama), Khoá API (ô che), nhắc chỗ lấy khoá. ③ Model (danh sách) + Kiểm tra kết nối → "✓ Kết nối được · n model đọc được ảnh · t s". Cảnh báo gửi ảnh ra ngoài với dịch vụ đám mây. "▸ Nâng cao": song song, chờ tối đa, ảnh gửi đi (DPI, cạnh, JPEG).
- Nguồn văn bản: ô Máy tính / Google Drive / GCS / Kho S3 (giai đoạn 7 mới bật Drive/GCS/S3; bản đầu hiện mờ "Sắp có" — sửa prototype trước khi làm, xin duyệt).
- Ghi kết quả: thư mục Excel; Google Sheet (link, tab, cách ghi Service account / Apps Script, file khoá JSON + email cần chia sẻ, Ghi thử 1 dòng).
- Cấu hình sửa đúng mô hình đang chọn ở màn chính.
- Ô nhập / ô chọn cùng kiểu khối đầu vào (cao 26, viền xám, chữ 13).

## Kiến trúc
- `cau_hinh.rs`: thêm `CauHinhBocTach { mo_hinh: Vec<MoHinh>, mac_dinh, song_song, cho_toi_da_s, dpi, canh_toi_da, jpeg, thu_muc_excel, sheet: CauHinhSheet }`; `serde(default)` để file cấu hình cũ vẫn đọc được.
- Khoá API / secret (quyết định #2): `src/boc_tach/khoa_cache.rs` — `ma_hoa(&str) -> String` / `giai_ma(&str) -> Option<String>`, AES-256-GCM, nonce ngẫu nhiên 12 byte, lưu dạng base64 `nonce‖bản mã` trong `.sm-dema4/boc_tach/khoa.bin` (quyền 0600 trên macOS). Khoá = SHA-256(`option_env!("DEMA4_KHOA_CACHE")` ‖ `license::ma_may()`). Thiếu biến khi build → không ghi khoá ra đĩa, ô Khoá API báo "Khoá chỉ dùng trong phiên này". Giải mã lỗi (máy khác / bản build khác) → coi như chưa có khoá, yêu cầu nhập lại. Không bao giờ đưa khoá vào log / thống kê / file cấu hình rõ.
- CI: thêm secret `DEMA4_KHOA_CACHE` vào workflow build (Windows/macOS); không in giá trị ra log.
- `gui_tach/hop_cau_hinh.rs`: `TabCauHinh::BocTach`, vẽ tách sang `gui_boc_tach/cau_hinh.rs` cho gọn.
- Kiểm tra kết nối: gọi `GET /models` trên luồng nền (chuẩn OpenAI hoặc Anthropic tuỳ nhà cung cấp), hiện kết quả.

## File liên quan
- Sửa: `src/cau_hinh.rs`, `src/gui_tach/hop_cau_hinh.rs`, workflow CI (thêm secret).
- Tạo: `src/gui_boc_tach/cau_hinh.rs`, `src/boc_tach/khoa_cache.rs` (test: mã hoá → giải mã đúng; sai mã máy → `None`; bản mã không chứa chuỗi khoá gốc). Crate mới: `aes-gcm` (MIT/Apache).

## Các bước
1. Kiểu cấu hình + đọc/ghi + test tương thích file cũ.
2. Vẽ tab (3 mục) theo prototype; test kittest bấm ô nhà cung cấp đổi địa chỉ / model.
3. Kiểm tra kết nối (máy chủ giả trong test).
4. Ảnh chụp so với `cau-hinh-boc-tach*.png`.

## Tiêu chí xong
- [ ] Lưu / Huỷ / Mặc định đúng như các tab khác; cửa sổ thấp vẫn thấy nút Lưu (như test `cau_hinh_khong_tran_cua_so`).
- [ ] Khoá không lộ trong log khi bật log chi tiết.

## Rủi ro
- Hộp Cấu hình hiện cao cố định → tab Bóc tách cần cuộn; giữ chân hộp luôn hiện.
