# ADR 0002 (D2): Mã PIN

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Prototype có hộp "Nhập PIN của app", "Lưu mã PIN", "Tự nhập lại PIN". Bản cũ cố ý không
  chạm vào PIN; hộp PIN là của middleware token.
- **Quyết định:** Bản 1.5.0 không nhận, không lưu, không ghi log PIN. Hộp "Nhập PIN" của prototype đổi thành
  hộp "Xác nhận ký" (token, số file, kiểu hiển thị, nơi lưu). Hộp PIN của middleware hiện sau đó, gắn vào
  cửa sổ ứng dụng.
- **Hệ quả:** CH-08 và KS-23 lùi sang 1.6, chỉ làm nếu spike S6 đạt và có ý kiến pháp chế.
- **Ghi chú khảo sát:** Bản cũ gắn HWND bằng thuộc tính CNG `"HWND Handle"` chỉ cho khoá RSA, và
  `CryptSetProvParam(NULL, PP_CLIENT_HWND)` cho CAPI. Bản mới gắn cho cả RSA và ECDSA.
