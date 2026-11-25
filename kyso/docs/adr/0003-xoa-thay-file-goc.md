# ADR 0003 (D3): Xoá hoặc thay file gốc

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Prototype có "Xóa file gốc" và "Ghi đè". Bản cũ không bao giờ ghi đè file gốc.
- **Quyết định:** Mặc định giữ file gốc. Tuỳ chọn "Thay file gốc" chỉ chạy sau khi chữ ký trên file kết quả
  đã được xác minh hợp lệ; file gốc được đưa vào Thùng rác, không xoá hẳn; lần đầu bật có hộp xác nhận.
- **Hệ quả:** File kết quả luôn ghi ra `*.tmp`, xác minh, rồi mới đổi tên. Tuỳ chọn này đặt trong Cấu hình
  nâng cao (KS-04, mức S).
