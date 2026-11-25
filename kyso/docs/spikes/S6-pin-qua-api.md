# S6: Truyền PIN cho middleware qua API (phục vụ bản 1.6)

- **Kết luận:** CHƯA CHẠY (18/11/2026). Không thuộc phạm vi 1.5.0.
- **Chặn bởi:** B-01, B-02. Ngoài ra cần ý kiến pháp chế (R11) trước khi đưa vào sản phẩm.

## Mục tiêu

Biết middleware bit4id và SafeNet có nhận PIN qua `NCRYPT_PIN_PROPERTY` (CNG) hoặc `PP_SIGNATURE_PIN` (CAPI)
hay không, để quyết định D2 cho bản 1.6.

## Phương án kỹ thuật

Chạy sau S1, dùng lại lớp bọc khoá của S1; đặt thuộc tính PIN trước khi ký; ghi lại hành vi khi PIN đúng,
PIN sai (theo dõi số lần thử còn lại để không khoá token), và khi phiên hết hạn.

## Lưu ý an toàn

- Chỉ chạy trên token dành riêng cho phát triển.
- PIN nhập từ bàn phím lúc chạy, không ghi vào file, log hay lịch sử lệnh.
- Dừng ngay khi còn 2 lần thử để tránh khoá token.

## Code thử nghiệm, kết quả, bằng chứng

Chưa có.
