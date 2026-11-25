# ADR 0005 (D5): Ký DOCX

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Ký DOCX đúng chuẩn cần XML-DSig và XAdES trong gói OOXML; thư viện Rust còn yếu, rủi ro cao.
- **Quyết định:** Bản 1.5.0 chuyển DOCX sang PDF qua Word rồi ký PDF, có thông báo rõ cho người dùng. Ký
  DOCX gốc để bản 1.7, cần spike riêng.
- **Hệ quả:** KS-21 (mức S) phụ thuộc Word COM (spike S7). Máy không có Word thì báo lỗi 1054.
