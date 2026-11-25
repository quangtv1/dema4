# ADR 0007 (D7): Kiểm tra cập nhật

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Prototype có "Kiểm tra cập nhật", tức ứng dụng có thêm một kết nối mạng ngoài TSA.
- **Quyết định:** Dùng `tauri-plugin-updater` với gói cập nhật có chữ ký minisign; máy chủ là một file JSON
  tĩnh. Chỉ kiểm tra khi người dùng bấm, hoặc lúc mở ứng dụng nếu người dùng bật.
- **Hệ quả:** Cập nhật cam kết riêng tư: chỉ gửi giá trị băm tới TSA, kiểm thu hồi với CA, kiểm phiên bản
  với máy chủ Sao Mai; không gửi nội dung văn bản. Cần nơi đặt máy chủ và cặp khoá minisign (mục 20 của kế
  hoạch). Không tự động phát hành hay đẩy bản cập nhật khi chưa được cho phép.
