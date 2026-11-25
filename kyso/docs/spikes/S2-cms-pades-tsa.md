# S2: CMS, dấu thời gian RFC 3161, ghi nối bản PDF

- **Kết luận (18/11/2026):**
  - **S2a: PASS.** Ký nối bản bằng khoá phần mềm, có dấu thời gian thật từ `tsa.ca.gov.vn`; pyHanko 0.38.1 xác
    nhận 9/9 chữ ký trên 6 file (90 phép kiểm đạt, 0 không đạt) và phát hiện đúng file bị sửa.
  - **S2b: CHƯA CHẠY.** Chưa ký bằng token thật (B-02) và chưa mở bằng Adobe Acrobat Reader. Tiêu chí "Adobe
    báo chữ ký hợp lệ" của kế hoạch **chưa được kiểm**.

## Mục tiêu

Một PDF được ký nối bản, có dấu thời gian từ `tsa.ca.gov.vn`; Adobe Acrobat Reader báo chữ ký hợp lệ và
pyHanko xác nhận cấu trúc PAdES B-T.

## Phương án kỹ thuật đã làm

Mã: `spikes/s2-cms/` (toolchain MSVC).

1. **PKI thử** (`gen_pki.py`, thư viện `cryptography`): một CA thử và một chứng thư người ký RSA 2048. Khoá
   bí mật chỉ nằm trong `out/` (không commit).
2. **CMS/CAdES** dựng tay từ các kiểu dữ liệu của `cms` 0.3.0-rc.2 (không dùng builder của crate):
   - Signed attributes: `contentType`, `messageDigest`, `signingCertificateV2` (chỉ có `certHash`, giống bản cũ).
     Không có `signing-time` (ADR 0006).
   - Chữ ký tính trên `SHA-256(DER(signedAttrs))`. **Việc ký là một hàm nhận băm và trả chữ ký**, nên thay
     khoá phần mềm bằng `NCryptSignHash` / `CryptSignHashW` không đụng tới phần còn lại.
   - `SignerIdentifier` là issuer + serial; thuật toán chữ ký ghi `rsaEncryption` với tham số NULL (như .NET).
   - `certificates`: chứng thư người ký và CA.
3. **Dấu thời gian** bằng `x509-tsp` 0.2.0-rc.0 và `ureq` (HTTP thường): `TimeStampReq` với băm SHA-256 của
   giá trị chữ ký, nonce 8 byte, `certReq = true`. Kiểm PKIStatus, `messageImprint`, nonce trước khi dùng.
   Token gắn vào unsigned attribute `signatureTimeStampToken`.
4. **Ghi nối bản** (đọc bằng `lopdf` 0.47, tự ghi phần nối):
   - Ghi đè Catalog (thêm hoặc cập nhật `/AcroForm`, `/SigFlags` luôn bật đủ bit 1 và 2) và Page (`/Annots`),
     xử lý cả khi `/AcroForm`, `/Fields`, `/Annots` là tham chiếu gián tiếp; giữ đúng số generation.
   - Field chữ ký gộp widget, ẩn (`/Rect [0 0 0 0]`), `/SubFilter /ETSI.CAdES.detached`.
   - `/ByteRange` 4 số rộng 10 ký tự, `/Contents` 32.768 byte; offset được ghi nhớ lúc ghi, không dò lại.
   - Xref mới cùng kiểu với xref cuối: bảng, hoặc stream `/W [1 4 2]` không nén; có `/Prev`.

## Kết quả thực tế

### TSA của Ban Cơ yếu

| Mục | Giá trị đo được |
|---|---|
| Địa chỉ | `http://tsa.ca.gov.vn/` |
| Phản hồi | HTTP 200, `application/timestamp-reply`, 3.780 byte, khoảng 0,02 giây mỗi lần (5 lần gọi) |
| PKIStatus | 0 (granted) |
| Policy | `2.16.704.1.1.1.2` |
| Chứng thư TSA | `CN=Máy chủ cấp dấu thời gian 02, O=Ban Cơ yếu Chính phủ, C=VN`, hiệu lực 03/06/2024 – 01/06/2034, có extKeyUsage |
| Cấp bởi | **Trực tiếp bởi Root G2** (`CN=RootCA chuyên dùng Chính phủ G2`), không qua CA trung gian |
| Chứng thư kèm trong token | Chứng thư TSA và Root G2 |
| Kích thước | Token 3.753 byte; cả CMS 6.237 byte, bằng 19% chỗ trống 32.768 byte |

Đây là câu trả lời cho R15.

### Kiểm bằng pyHanko (`validate.py`)

Gốc tin cậy: CA thử (người ký) và `certs/rootcag2.crt` (dấu thời gian). Không tải gì từ mạng, không kiểm thu hồi.

| File | Kiểu xref của file gốc | Số chữ ký | Kết quả |
|---|---|---|---|
| `word.signed.pdf` (PDF do Word xuất ở spike S7) | bảng, **hybrid-reference** | 1 | Đạt (pyHanko phải đọc ở chế độ không chặt) |
| `pdfwriter.signed.pdf` | bảng | 1 | Đạt, chế độ chặt |
| `xrefstream.signed.pdf` | stream | 1 | Đạt, chế độ chặt |
| `word.signed2.pdf` | bảng, hybrid | 2 | Cả hai đạt |
| `pdfwriter.signed2.pdf` | bảng | 2 | Cả hai đạt |
| `xrefstream.signed2.pdf` | stream | 2 | Cả hai đạt |
| `pdfwriter.tampered.pdf` (đổi 1 bit ở offset 1000) | bảng | 1 | **Bị phát hiện**: không nguyên vẹn |

Mỗi chữ ký qua 10 phép kiểm: nguyên vẹn, chữ ký đúng, người ký tin cậy, phạm vi hợp lệ, `SubFilter` đúng, có
`signingCertificateV2`, không có `signing-time`, có dấu thời gian, dấu thời gian đúng, TSA tin cậy về Root G2.

Khi ký chồng, chữ ký thứ nhất được pyHanko xếp `ENTIRE_REVISION / FORM_FILLING`: phần nối thêm của chữ ký thứ
hai được coi là thay đổi hợp lệ, **chữ ký trước không bị phá**.

Bằng chứng: `docs/spikes/evidence/S2/` (`pyhanko-validate.txt`, `pyhanko-validate-tampered.txt`,
`word.signed2.pdf`, `xrefstream.signed2.pdf`, `ca.cert.pem` của CA thử để ai cũng kiểm lại được).

## Trả lời các câu hỏi của spike

- **Crate `cms` có đủ dùng không (R3)?** Đủ, theo cách dùng kiểu dữ liệu và tự dựng `SignerInfo`. `cms`
  0.3.0-rc.2 và `x509-tsp` 0.2.0-rc.0 cùng dùng `der` 0.8.3 và `x509-cert` 0.3.0 nên ghép được với nhau. Cả hai
  vẫn là bản RC.
- **Chứng thư TSA:** xem bảng trên.
- **Kích thước CMS:** 6.237 byte với RSA 2048 và hai chứng thư; chỗ trống 32.768 byte còn dư nhiều.
- **So byte với file do bản .NET ký:** **chưa làm**, vì chưa có file do bản 1.4.0 ký.

## Phát hiện

1. **PDF do Word xuất là file hybrid-reference** (bảng xref kèm `/XRefStm`). pyHanko ở chế độ chặt mặc định
   từ chối kiểm chữ ký trên loại file này; phải đọc với `strict=False`. Đây là loại file đầu vào phổ biến
   nhất của ứng dụng (kể cả từ chức năng Tạo văn bản), nên CI phải kiểm ở chế độ không chặt cho các file
   này. Bản cũ không đọc `/XRefStm` (xem K-16).
2. Bộ ghi nối bản của spike đã sửa ba giới hạn của bản cũ: tham chiếu gián tiếp, số generation, `/SigFlags`.

## Chưa thử trong spike này

- Adobe Acrobat Reader, Foxit (S2b).
- Token thật, khoá ECDSA (chữ ký `r‖s` phải đổi sang DER).
- File có object stream (`/ObjStm`) chứa Catalog hoặc Page; file mã hoá; file có `/DocMDP`.
- Chữ ký có khối hiển thị (spike S3 mới làm phần khối chữ, chưa gắn vào widget).
- Kiểm chữ ký của token TSA và chuỗi chứng thư **bằng code Rust** (hiện mới do pyHanko kiểm).
- Nonce sinh bằng bộ sinh ngẫu nhiên mật mã (spike dùng băm của đồng hồ; sản phẩm không được làm vậy).
- Ký song song nhiều file, thử lại khi TSA lỗi.

## Rủi ro còn lại

R2 giảm mạnh về mặt cấu trúc, nhưng chưa đóng cho tới khi Adobe xác nhận. R3 giảm. R15 đóng. R10 chưa đo
(mới gọi TSA 5 lần, tuần tự).
