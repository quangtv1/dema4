# Spike giai đoạn 0

> Cập nhật: 18/11/2026. Các spike không cần token đã chạy xong: S2a, S3a, S4, S5, S7 đạt. Còn S1, S2b, S6 (chờ token)
> và S3b (chờ Adobe, Foxit). Không spike nào được ghi PASS khi chưa có bằng chứng chạy thật.

| Spike | Câu hỏi | Trạng thái | Chặn bởi | Làm được ngay phần nào |
|---|---|---|---|---|
| [S1](S1-usb-token.md) | Ký bằng `NCryptSignHash` / `CryptSignHashW` từ Rust với token bit4id và SafeNet, hộp PIN không chìm | CHƯA CHẠY | B-02 | Không |
| [S2](S2-cms-pades-tsa.md) | CMS + RFC 3161 + ghi nối bản, Adobe và pyHanko công nhận | **S2a PASS**: pyHanko xác nhận 9/9 chữ ký trên 6 file, có dấu thời gian thật; S2b CHƯA CHẠY | B-02 (token), Adobe Reader | S2a xong |
| [S3](S3-font-tieng-viet.md) | Nhúng subset font tiếng Việt, hiển thị và copy đúng | **S3a PASS 4/5 font** (pdf.js, pyHanko); Georgia FAIL do font thiếu glyph; S3b CHƯA CHẠY | B-03 (Adobe, Foxit) | Đã làm hết phần làm được |
| [S4](S4-excel-calamine.md) | `calamine` cho kết quả giống ClosedXML với 3 file mẫu | **PASS**: 3 file mẫu khớp 193/193 ô; ô số cần tự áp định dạng | — | Xong |
| [S5](S5-html-printtopdf.md) | WebView2 `PrintToPdf` từ Tauri | **PASS**: A4, font nhúng, tiếng Việt đúng; khổ giấy phải truyền qua cài đặt in | — | Xong; chưa thử in nhiều tài liệu liên tiếp và HTML có ảnh |
| [S6](S6-pin-qua-api.md) | Middleware nhận PIN qua `NCRYPT_PIN_PROPERTY` / `PP_SIGNATURE_PIN` (cho bản 1.6) | CHƯA CHẠY | B-02 | Không |
| [S7](S7-word-com.md) | Word COM từ Rust trên luồng STA | **PASS**: 3 mẫu ra PDF trùng bản tham chiếu, Word thoát sạch, 60 lượt liên tiếp ổn | — | Xong; chưa thử khi người dùng đang mở Word |

Mã spike đặt ở `spikes/<mã>/` tại gốc repo (ngoài workspace chính) để không lẫn với code sản phẩm.

Mỗi file spike có đủ: mục tiêu, phương án kỹ thuật, code thử nghiệm, kết quả thực tế, bằng chứng, kết luận
PASS/FAIL, rủi ro còn lại.
