# Rà soát phần kiểm tra chữ ký (24/11/2026)

> **Cập nhật cùng ngày:** V1–V8 và V10 đã sửa theo TD-57 (TEST_REPORT #108). Người phụ trách quyết: V9 giữ nguyên (chỉ tin
> CA Ban Cơ yếu); ký dài hạn PAdES B-LT đã làm theo TD-58 (TEST_REPORT #109).

Đối chiếu `smks-sign::verify`, `smks-crypto::{verify, chain, trust}` với Nghị định 68/2024/NĐ-CP (Điều 30–32) và
với bộ kiểm độc lập pyHanko. Bằng chứng chạy thật: TEST_REPORT #107.

## Căn cứ pháp lý (Nghị định 68/2024/NĐ-CP, đọc bản sao trên vcci.com.vn, chưa đối chiếu Công báo)

- **Điều 30:** chữ ký hợp lệ khi đủ 5 điều kiện:
  1. Dữ liệu toàn vẹn, kiểm được bằng khoá công khai.
  2. Thông tin người ký khớp chứng thư.
  3. Thời gian ký phù hợp.
  4. Chứng thư còn hiệu lực (Điều 31).
  5. Đường dẫn chứng thực hợp lệ (Điều 32).
- **Điều 31:** chứng thư phải đáp ứng cả ba điều sau:
  - Còn thời hạn **tại thời điểm ký số**.
  - Đúng phạm vi sử dụng.
  - Trạng thái kiểm qua CRL hoặc OCSP còn hoạt động **tại thời điểm ký số**.
- **Điều 32:** chứng thư người ký, chứng thư trung gian và chứng thư gốc đều còn hiệu lực tại thời điểm ký. Gốc phải
  khớp gốc do tổ chức cung cấp dịch vụ công bố.
- **Không tìm thấy** quy định bắt buộc lưu dữ liệu kiểm tra dài hạn (LTV).
- Chưa đọc được Thông tư 15/2025/TT-BKHCN (yêu cầu kỹ thuật phần mềm kiểm tra chữ ký): các trang luật trả 403.

## Đã làm đúng

| Điều kiện | Mã hiện tại |
|---|---|
| Toàn vẹn | Kiểm `messageDigest` trên `/ByteRange` và chữ ký trên signed attributes. Hỗ trợ RSA PKCS#1 v1.5, ECDSA P-256 / P-384. `/ByteRange` phải phủ từ đầu file, khoảng hở đúng bằng `/Contents`. Có byte nối thêm sau chữ ký cuối thì báo sai |
| Gốc khớp gốc công bố | Kho gốc độc quyền chỉ có Root G1 / Root G2 ghim SHA-256. Gốc khác cài trong Windows không có tác dụng |
| Hiệu lực tại thời điểm ký | Chuỗi người ký kiểm tại `genTime` của dấu thời gian, không theo giờ máy |
| Dấu thời gian | Kiểm `messageImprint` khớp chữ ký, chữ ký của TSA, EKU `timeStamping`, chuỗi TSA về Root G2 |
| Thời gian ký phù hợp | So `signing-time` với `genTime`: lệch quá 60 giây là "bất thường". Chấp nhận lệch đúng +7 giờ như bản cũ |

## Phát hiện

| # | Mức | Vấn đề | Bằng chứng |
|---|---|---|---|
| V1 | Cao | **Dấu thời gian tài liệu** (`/DocTimeStamp`, `ETSI.RFC3161`, Adobe và các bản ký LTV hay thêm) bị kiểm như chữ ký thường, nên báo **"Chữ ký không hợp lệ / nội dung bị sửa"** cho file đúng | Chạy thật #107 (5): dấu của TSA Ban Cơ yếu thật, pyHanko báo đúng |
| V2 | Cao | **Sửa nội dung sau chữ ký rồi ký thêm** không bị phát hiện: chữ ký đầu vẫn báo như cũ. Adobe / pyHanko báo "tài liệu đã bị thay đổi sau chữ ký này" | #107 (4b): pyHanko `modified=OTHER`, smks không nhắc gì |
| V3 | Cao | Chữ ký **không có dấu thời gian** thì dừng ở "Không có dấu thời gian", **không kiểm người ký**. Chữ ký của CA lạ trông nhẹ hơn thực tế. Bản ký lúc mất mạng (TD-54) rơi vào trường hợp này | #107 (2): khoá thử ra `noTimestamp`, không ra `untrustedSigner` |
| V4 | Cao | **Thu hồi:** `verify_file` mặc định không kiểm (`revocation: false`), nên chưa đáp ứng Điều 31 khoản 2. Khi có kiểm thì CryptoAPI lấy trạng thái **hiện tại**: thu hồi *sau* lúc ký (kể cả chứng thư hết hạn bị "tự động thu hồi" theo Điều 18) vẫn có thể làm chữ ký cũ thành "không hợp lệ". "Không kiểm được" (mất mạng) vẫn ra "Hợp lệ" | Đọc mã. Chưa thử với chứng thư bị thu hồi thật |
| V5 | Vừa | File **chưa ký** ra kết luận `noTimestamp` (nhãn "Không có dấu thời gian") | #107 (0) |
| V6 | Vừa | Thuật toán **chưa hỗ trợ** (RSA-PSS, băm lạ) báo "Chữ ký không hợp lệ" thay vì "không xác định / chưa hỗ trợ". **SHA-1** được chấp nhận, không cảnh báo | Đọc mã (`verify_digest_signature`) |
| V7 | Vừa | Không kiểm **keyUsage / EKU** của chứng thư người ký (bỏ qua `NOT_VALID_FOR_USAGE`), không kiểm thuộc tính `signing-certificate-v2` (chống tráo chứng thư). Điều 31 điểm b yêu cầu đúng phạm vi sử dụng | Đọc mã (`chain::interpret`) |
| V8 | Thấp | Chỉ đọc `SignerInfo` đầu tiên; không đọc DSS / VRI (dữ liệu thu hồi nhúng sẵn); không xét quyền DocMDP của chữ ký chứng nhận | Đọc mã |
| V9 | Phạm vi | Chỉ tin CA Ban Cơ yếu (G1/G2). Chữ ký số công cộng (VNPT, Viettel, … gốc NEAC) ra "Người ký không thuộc CA Ban Cơ yếu". Công cụ của NEAC và Thông tư 04/2019 kiểm cả hai loại | Thiết kế (TD cũ), cần người phụ trách quyết |
| V10 | Thiếu | **Chưa có màn Kiểm tra chữ ký** (PROJECT_STATUS, việc kế tiếp số 2). `verify_file` có IPC nhưng giao diện chưa dùng | — |

## Đề xuất thứ tự sửa

1. V1, V2, V3, V5: sửa trong `smks-sign::verify`, có test fixture tạo bằng pyHanko.
2. V4: kiểm thu hồi mặc định khi có mạng, so ngày thu hồi với thời điểm ký, "không kiểm được" thành kết luận riêng.
3. V6, V7.
4. Màn Kiểm tra chữ ký (V10), hiển thị theo 5 điều kiện của Điều 30.
5. Hỏi người phụ trách về V9 và LTV (nhúng OCSP/CRL lúc ký, PAdES B-LT).
