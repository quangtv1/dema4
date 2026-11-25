# ADR 0008 (D8): Mẫu HTML và xuất DOCX trong Tạo văn bản

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Bản cũ chỉ có mẫu DOCX và chỉ xuất PDF qua Word.
- **Quyết định:** Bản 1.5.0 có thêm xuất DOCX (file đã điền, không cần Word) và mẫu HTML in ra PDF bằng
  WebView2 `PrintToPdf` (không cần Word).
- **Hệ quả:** Mẫu HTML (TV-02, mức S) phụ thuộc spike S5. Mọi giá trị điền vào HTML phải được escape.
