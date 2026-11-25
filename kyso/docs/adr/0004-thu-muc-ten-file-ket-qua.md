# ADR 0004 (D4): Thư mục và tên file kết quả

- **Trạng thái:** Đã chấp nhận, 18/11/2026. Còn một điểm mở (Q-05).
- **Bối cảnh:** Bản cũ lưu cạnh file gốc với tên `<tên>_daky_<yyyyMMdd_HHmmss>.pdf` và bỏ qua các file có
  hậu tố đó khi quét thư mục. Prototype lưu vào `DaKySo\` với hậu tố `_signed`.
- **Quyết định:** Lưu `<thư mục nguồn>\DaKySo\<tên><hậu tố>.pdf`, hậu tố mặc định `_signed`, đổi được; có
  thể chọn thư mục cố định. Khi quét không vào thư mục con `DaKySo`. File đã có chữ ký vẫn hiện, kèm nhãn
  "Đã có N chữ ký". File lỗi không bị đổi tên; tuỳ chọn chép vào `DaKySo` với hậu tố `_loi`.
- **Hệ quả:** Trùng tên phải hỏi Ghi đè / Giữ cả hai / Huỷ (KS-16).
- **Điểm mở (Q-05):** Thư mục của người dùng bản cũ đang có sẵn các file `_daky_…` nằm cạnh file gốc. Theo
  quyết định này chúng sẽ hiện lại trong danh sách. Cần chốt: tiếp tục bỏ qua theo mẫu
  `_daky_\d{8}_\d{6}$`, hay hiện với nhãn. Hằng số nhận diện đã giữ ở
  `smks_core::consts::signing::LEGACY_SIGNED_FILE_SUFFIX`.
