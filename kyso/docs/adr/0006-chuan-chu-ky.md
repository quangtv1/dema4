# ADR 0006 (D6): Chuẩn chữ ký PAdES B-T

- **Trạng thái:** Đã chấp nhận, 18/11/2026.
- **Bối cảnh:** Bản cũ đưa `signing-time` vào signed attributes. ETSI EN 319 142-1 quy định PAdES baseline
  không dùng thuộc tính này; thời điểm ký nằm ở `/M`.
- **Quyết định:** Signed attributes gồm `contentType`, `messageDigest`, `signingCertificateV2`. Không có
  `signing-time`. Dấu thời gian RFC 3161 trên giá trị chữ ký, gắn ở unsigned attribute
  `signatureTimeStampToken`. `/SubFilter /ETSI.CAdES.detached`.
- **Hệ quả:** Phải kiểm bằng Adobe Acrobat Reader và pyHanko (spike S2, CI). Bộ xác minh vẫn đọc
  `signing-time` của file do phần mềm khác ký, giữ xử lý lệch +7 giờ và kết luận `TimeMismatch` của bản cũ.
- **Ghi chú khảo sát:** `signingCertificateV2` của bản cũ chỉ có `certHash` (SHA-256 mặc định, không có
  `issuerSerial`); bản mới giữ đúng dạng này để so byte được với file do bản .NET ký.
