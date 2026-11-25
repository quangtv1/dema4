# Lộ trình triển khai

> Cập nhật: 18/11/2026. **Không theo lịch tuần.** Ngày 18/11/2026 người phụ trách yêu cầu bỏ các ước lượng
> theo tuần ở kế hoạch mục 14; Claude trực tiếp triển khai liên tục, thứ tự do phụ thuộc và blocker quyết định.

| Giai đoạn | Nội dung | Mốc | Trạng thái |
|---|---|---|---|
| 0 | Khảo sát, spike S1–S7, nền móng | M0 | **Đang làm**: khảo sát và nền móng lõi xong; spike chưa chạy; M0 bị chặn |
| 1 | Lõi ký số và PDF đầy đủ | M1 | Chưa bắt đầu |
| 2 | Vỏ Tauri, Angular, màn Ký số | M2 | Chưa bắt đầu |
| 3 | Vị trí chữ ký và Cấu hình | M3 | Chưa bắt đầu |
| 4 | Kiểm tra chữ ký | M4 | Chưa bắt đầu |
| 5 | Tạo văn bản | M5 | Chưa bắt đầu |
| 6 | Hỗ trợ, bộ cài, cập nhật | M6 | Chưa bắt đầu |
| 7 | Kiểm thử tổng, pilot, phát hành | M7 | Chưa bắt đầu |

## Giai đoạn 0, chi tiết

| Hạng mục | Trạng thái |
|---|---|
| Khảo sát repo, môi trường, backend .NET | Xong |
| Phân tích prototype | Xong, chờ rà soát |
| Phân tích frontend cũ | Xong, chờ rà soát |
| ADR D1–D9, ADR toolchain | Xong |
| Workspace, lint, `smks-core` (hằng số, mã lỗi, kế hoạch dấu), ghim CA, khung CLI | Xong, kiểm trên toolchain GNU |
| CI | Đã viết, chưa chạy |
| Fixtures | 3 cặp mẫu tạo văn bản, `tricky.xlsx` kèm kết quả mong đợi; chưa có PDF |
| Khung Tauri + Angular | Xong: exe chạy được, kiểm tra khói 10/10 |
| Spike S1–S7 | S2a, S3a, S4, S5, S7 đạt. S1, S2b, S6 chờ token; S3b chờ Adobe/Foxit |
| **M0** | **Chưa đạt** |

## Thứ tự làm việc khi bị chặn bởi môi trường

Đã kiểm cây phụ thuộc của các crate dự kiến (bảng ở cuối `RISK_REGISTER.md`). Trong lúc chờ gỡ B-01 chỉ
làm được, theo thứ tự:

1. Spike S3a (subset font, Form XObject).
2. Giai đoạn 1: Signature Appearance, Geometry, bộ ghi nối bản tự viết, cấu trúc ASN.1 của CMS và TSP.
3. Phần logic thuần: cấu hình, đặt tên file, tự map trường, đọc CSV.

Các phần **bắt buộc chờ B-01**: kho chứng thư, khoá CNG/CAPI, chuỗi CA và thu hồi, ký thử và xác minh
(`rsa`, `p256`), đọc PDF bằng `lopdf`, đọc Excel, đọc/ghi DOCX, gọi TSA qua thư viện HTTP đầy đủ, ghi log ra
file, Word COM, WebView2, Tauri, bộ cài.

## Giai đoạn 1, thứ tự theo master prompt

1. Certificate Store · 2. Signing Key CNG/CAPI · 3. Token Session · 4. CMS/CAdES · 5. Timestamp RFC 3161 ·
6. Certificate Chain G1/G2 · 7. Revocation · 8. PDF Reader · 9. Incremental Writer · 10. Signature
Appearance · 11. Geometry · 12. Stamp Planner (**đã xong ở F-04**) · 13. Job Processing · 14. CLI
sign/verify/inspect.

Điều kiện M1: ký được bộ PDF fixtures; Adobe và pyHanko xác nhận; phát hiện file bị sửa; kiểm được chữ ký
nối bản; không làm hỏng chữ ký có trước.

## Điều kiện các mốc sau

| Mốc | Điều kiện |
|---|---|
| M2 | Từ giao diện: chọn thư mục, chọn token, ký nhiều PDF, xem tiến độ, dừng, ký tiếp, mở kết quả |
| M3 | Đủ chức năng ký của bản cũ, thêm mẫu theo khổ giấy và vị trí từng file |
| M4 | Kết luận đúng với: file app ký, file phần mềm khác ký, file bị sửa, CA lạ, file có chú thích sau ký |
| M5 | 3 cặp mẫu ra kết quả tương đương bản .NET, đối chiếu từng trang và từng trường |
| M6 | Bộ cài chạy sạch trên Windows 10/11 mới; cài mới, nâng cấp từ 1.4.0, gỡ đúng |
| M7 | Đạt toàn bộ tiêu chí nghiệm thu, có bản phát hành 1.5.0 |
