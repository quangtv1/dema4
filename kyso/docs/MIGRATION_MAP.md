# Bảng đối chiếu .NET 1.4.0 → Rust 1.5.0

> Lập ngày 18/11/2026 từ việc đọc trực tiếp mã nguồn `Kytl-main/ktl.be/` (không suy từ tên class).
> Mỗi nhận định ghi file nguồn để kiểm lại. Cột "Trạng thái" chỉ ghi DONE khi đã có code và test.

## 1. Phạm vi đã đọc

| Đã đọc toàn bộ | Chưa đọc chi tiết (chỉ là lớp vận chuyển HTTP, sẽ bị thay bằng IPC) |
|---|---|
| `ktl.be.shared/Constants/**`, `ErrorCodes.cs`, `ErrorMessages.cs` | `Controllers/**`, `BaseController.cs` |
| `CertificateProvider`, `StoreSigningKey`, `PdfSigner`, `TimestampClient`, `SignatureVerifier`, `PdfSignatureReader` | `FileUploadService.cs` (bị bỏ) |
| `PdfIncrementalSigner`, `PdfRevisionInfo`, `PdfObjectWriter`, `PdfAppearanceBuilder`, `PdfImageStamper`, `PdfPageInspector`, `SignaturePlacementResolver` | `ktl.be.api/Program.cs` (chỉ đọc phần đăng ký DI và CORS) |
| `SignService`, `StampPlanner`, `DocumentMergeService`, `WordTemplateEngine`, `ExcelReader`, `WordPdfConverter` | `build-desktop.ps1` |
| `ktl.be.desktop/Program.cs`, `installer/SaoMai-KySo.nsi`, các DTO liên quan | |

Frontend cũ (`ktl.fe`) và prototype được phân tích riêng: `docs/implementation/FRONTEND_ANALYSIS.md`,
`docs/implementation/PROTOTYPE_ANALYSIS.md`. Hai tài liệu này do agent lập; tôi đã kiểm tại chỗ một số trích
dẫn (xem `TASK_BOARD.md`, P0-03 và P0-04) nhưng chưa rà toàn bộ.

Điểm cần nhớ từ frontend cũ: toạ độ vị trí là tỉ lệ 0–1 của trang như pdf.js hiển thị ở `scale: 1` (đã áp
`/Rotate`), gốc trên trái, Y hướng xuống; frontend không xử lý xoay trang, nên `smks-pdf::geometry` phải cho
đúng khung mà pdf.js dùng. Giao diện cũ đo bề rộng khối chữ ký bằng `canvas.measureText` để vẽ khung xem
trước; lúc ký, backend đo lại bằng PDFsharp (mục 3.9).

Thư viện bản cũ dùng: PDFsharp 6.2.4, System.Security.Cryptography.Pkcs 9.0.21, ClosedXML 0.105.1,
DocumentFormat.OpenXml 3.1.1, Photino.NET 4.0.16, NLog 6, .NET 9.

## 2. Ánh xạ file → module

| File .NET | Module Rust | Trạng thái | Ghi chú port |
|---|---|---|---|
| `SigningConstants.cs`, `SignatureConstants.cs`, `StampImageConstants.cs`, `DocumentMergeConstants.cs`, `AppConstants.cs` | `smks-core::consts` | **DONE** | Đã đối chiếu từng giá trị, có test |
| `ErrorCodes.cs`, `ErrorMessages.cs` | `smks-core::error` | **DONE** | Xem K-01 |
| `StampPlanner.cs` | `smks-core::stamp_plan` | **DONE** | Tách phần đọc file ảnh ra khỏi lõi, 16 test |
| `SignatureVerifier.LoadPinnedCert`, `*.crt` | `smks-crypto::trust` | **DONE** | Nhúng bằng `include_bytes!`, kiểm SHA-256; xem K-02 |
| `CertificateProvider.cs` | `smks-crypto::store` | **DONE** (chưa thử token thật) | Mục 3.1; TD-35. TEST_REPORT #63–65 |
| `StoreSigningKey.cs` | `smks-crypto::{key, store::StoreKey}` | **DONE** (CNG RSA/ECDSA, CAPI thử bằng khoá tạm; chưa thử token thật) | Mục 3.2. TEST_REPORT #64 |
| `PdfSigner.cs` (`CadesSigner`) | `smks-crypto::cms`, `smks-sign::sign` | **DONE** | Mục 3.3; không `signing-time` (ADR 0006). TEST_REPORT #63, #68 |
| `TimestampClient.cs` | `smks-crypto::tsa` | **DONE** | Mục 3.4; thêm kiểm chữ ký của TSA bằng Rust. TEST_REPORT #63 |
| `SignatureVerifier.cs` | `smks-crypto::{chain,verify}`, `smks-pdf::sigscan`, `smks-sign::verify` | **DONE** phần lõi (thêm thu hồi; phân loại "thay đổi sau khi ký" làm ở M4) | Mục 3.5. TEST_REPORT #65, #67–69 |
| `PdfSignatureReader.cs` | `smks-pdf::sigscan` | TODO | Dò regex + nhìn lùi 6000 byte; thay bằng đọc AcroForm |
| `PdfRevisionInfo.cs` | `smks-pdf::reader` | **DONE** | Mục 3.6; đọc qua `lopdf`, hiểu file hybrid-reference (K-16). TEST_REPORT #66 |
| `PdfIncrementalSigner.cs`, `PdfObjectWriter.cs` | `smks-pdf::{incremental,object}` | **DONE** | Mục 3.7; sửa 3 giới hạn của bản cũ (tham chiếu gián tiếp, generation, `/SigFlags`). TEST_REPORT #66, #68 |
| `PdfAppearanceBuilder.cs` | `smks-pdf::appearance` | **DONE** (chưa có màn chỉnh dấu) | Mục 3.8; tự dựng Form XObject, subset font, ảnh xử lý theo điểm ảnh cùng công thức. TD-33. TEST_REPORT #66, #69 |
| `SignaturePlacementResolver.cs`, `PdfPageInspector.cs`, `PdfPageGeometryDto.cs` | `smks-pdf::geometry` | **DONE** | Mục 3.9; thêm nhận diện khổ A1–A5. TEST_REPORT #66 |
| `PdfImageStamper.cs` | `smks-pdf::stamp` | TODO | Vẽ ảnh thẳng vào content stream, không chỉnh độ đậm/nét |
| `SignService.cs` | `smks-app::jobs::sign` (+ phần thuần ở `smks-core`) | TODO | Mục 3.10 |
| `ExcelReader.cs` | `smks-merge::table` + `numfmt` | **DONE** | Khớp ClosedXML 0.105.1 trên 4 file ở cả en-US và vi-VN; thêm chọn sheet, dòng tiêu đề, `.xls`, CSV |
| `WordTemplateEngine.cs` | `smks-merge::docx` | **DONE** | Thêm footnote, endnote; giữ nguyên mọi byte khác của mẫu |
| `WordPdfConverter.cs` | `smks-merge::word` | **DONE** | Thêm thời hạn 120 giây/file, buộc đóng theo PID, đếm trang |
| `DocumentMergeService.cs` | `smks-merge::{plan,batch}` + `smks-app::commands::merge` | **DONE** (trừ đóng dấu) | Tên file theo TD-27; đóng dấu đỏ/chữ ký tươi khi tạo chưa làm |
| `FileUploadService.cs`, `FileUploadController.cs` | bỏ | — | Tauri trả đường dẫn thật khi kéo thả |
| Controllers + SSE | `smks-app::commands` + `Channel` | TODO | Ba loại sự kiện `manifest`/`file`/`summary` giữ nguyên |
| `ktl.be.desktop/Program.cs` | `app/src-tauri/src/main.rs` | TODO | Cửa sổ 1280×820, tối thiểu 1024×700; `CryptSetProvParam(NULL, PP_CLIENT_HWND=1, &hwnd, 0)` |
| `installer/SaoMai-KySo.nsi` | `installer/SaoMai-KySo.nsi` | TODO | Giai đoạn 6 |

## 3. Hành vi đã xác minh của bản cũ

### 3.1 `CertificateProvider.cs`

- Quét `CurrentUser\My` (nguồn `Local`) rồi `LocalMachine\My` (nguồn `Server`). Gộp theo thumbprint, ưu tiên
  bản `CanSign`; sắp `CanSign` giảm dần rồi theo CN.
- Tên provider: `RSACng.Key.Provider.Provider`, hoặc `CspKeyContainerInfo.ProviderName`, hoặc `ECDsaCng`.
  **Bản cũ gọi `GetRSAPrivateKey()` cho mọi chứng thư chỉ để lấy tên provider.** Bản Rust đọc thuộc tính
  `CERT_KEY_PROV_INFO_PROP_ID` của chứng thư, không cần mở khoá.
- Token phần cứng: tên provider khác rỗng, không chứa dấu hiệu phần mềm nào, và chứa ít nhất một dấu hiệu
  phần cứng (so khớp không phân biệt hoa thường).
- `AllowsSigning`: không có KeyUsage, hoặc có `DigitalSignature`, hoặc có `NonRepudiation`.
- `IsExpired` so `VietnamNow` (giờ VN, không có múi giờ) với `NotBefore`/`NotAfter` (giờ địa phương của máy):
  sai nếu máy không đặt múi giờ UTC+7. Bản Rust so theo UTC.
- `CanSign = HasPrivateKey && !IsExpired && AllowsSigning && IsTrusted`.
- Lý do không ký được, theo thứ tự ưu tiên và nguyên văn:
  1. `Chứng thư số không có khóa bí mật trên máy này (chỉ xem được, không ký được).`
  2. `Chứng thư số ngoài thời hạn hiệu lực ({ValidFrom} - {ValidTo}).` (định dạng `dd/MM/yyyy HH:mm:ss`)
  3. `Chứng thư số không được cấp quyền ký (KeyUsage thiếu digitalSignature/nonRepudiation).`
  4. `Chứng thư số không thuộc CA Ban Cơ yếu (chain không về Root G1 lẫn Root G2 đã ghim). Chữ ký tạo bằng chứng thư này sẽ bị đánh giá là không hợp lệ.`

### 3.2 `StoreSigningKey.cs`

- `Verify(thumbprint)` kiểm theo thứ tự: tìm trong kho → có khoá bí mật → còn hạn → KeyUsage → **ký thử**
  → chuỗi CA tin cậy. Kết quả và handle khoá được giữ lại để ký cả lô.
- Ký thử: `SignData(32 byte ngẫu nhiên, SHA-256, PKCS#1 v1.5)` với RSA, `SignData(…, SHA-256)` với ECDSA.
  Đây là lúc middleware hỏi PIN.
- HWND cho hộp PIN: chỉ đặt thuộc tính `"HWND Handle"` cho `RSACng`. `ECDsaCng` không được đặt (thiếu sót).
  Đường CAPI dựa vào `CryptSetProvParam(PP_CLIENT_HWND)` gọi một lần lúc tạo cửa sổ.
- Đăng ký DI là **Scoped** (`ktl.be.api/Program.cs:60`), nên cache của lần gọi `kiem-tra-token` không dùng
  được cho lần gọi `ky-thu-muc`: ký thử chạy hai lần. Khớp nhận định của kế hoạch mục 6.2.
- Sau mỗi lô, `SignService` gọi `ClearCache()`.
- `IsTokenPresent` = chứng thư còn trong kho (không thử ký).
- **Bẫy khi port ECDSA:** `NCryptSignHash` trả `r‖s` (IEEE P1363), còn CMS cần `SEQUENCE { r, s }` dạng DER.
  Bản cũ không gặp vì `SignedCms.ComputeSignature` tự xử lý.
- Thứ tự kiểm ở bản Rust: đề xuất kiểm chuỗi CA **trước** khi ký thử để không hỏi PIN cho chứng thư sẽ bị
  từ chối. Câu thông báo giữ nguyên.

### 3.3 `PdfSigner.cs` (`CadesSigner`)

- `SignedCms` detached, `SubjectIdentifierType.IssuerAndSerialNumber`, băm SHA-256,
  `X509IncludeOption.WholeChain`.
- Signed attributes: `contentType` và `messageDigest` (thư viện tự thêm), `signingTime = UtcNow`,
  `signingCertificateV2`.
- `signingCertificateV2` dựng tay: `SEQUENCE { SEQUENCE { SEQUENCE { OCTET STRING sha256(cert) } } }`,
  tức ESSCertIDv2 chỉ có `certHash`, bỏ `hashAlgorithm` (mặc định SHA-256) và `issuerSerial`.
- Khoá ký được bọc bằng `SemaphoreSlim(1,1)`: mỗi lúc chỉ một lệnh ký.
- Dấu thời gian lấy trên `SHA-256(signatureValue)`, gắn vào unsigned attribute
  `1.2.840.113549.1.9.16.2.14`.
- Chỗ trống chữ ký `32 * 1024` byte. Ghi `<output>.tmp` rồi đổi tên; nếu file đích đã có thì xoá trước.
- `WholeChain` dựng chuỗi theo kho chứng thư của Windows nên nội dung `certificates` phụ thuộc máy. Bản Rust
  nhúng chứng thư người ký và CA trung gian đã ghim để kết quả ổn định.

### 3.4 `TimestampClient.cs`

- `Rfc3161TimestampRequest.CreateFromHash(hash, SHA256, policy = null, nonce = 8 byte, certReq = true)`.
- POST tới `http://tsa.ca.gov.vn/`, `Content-Type: application/timestamp-query`, timeout 180 giây, tối đa 8
  yêu cầu đồng thời. HTTP khác 2xx thì báo `TSA trả về HTTP {mã}. Không lấy được dấu thời gian.`
- `ProcessResponse` kiểm trạng thái, nonce và băm. **Không thử lại, không kiểm chữ ký hay chuỗi của token
  ở bước này**; việc đó nằm ở bước tự kiểm sau khi ký. Bản Rust kiểm ngay khi nhận token (kế hoạch 6.4).

### 3.5 `SignatureVerifier.cs`

- Dò mọi `/ByteRange [a b c d]` bằng regex trên toàn file. Khối không đọc được thì đếm vào `SkippedBlocks`.
- Mỗi khối: `/Contents` lấy từ khoảng hở; `RangeSane = (a == 0) && khoảng hở chỉ chứa một chuỗi hex <…>`.
- Mỗi người ký, kiểm theo thứ tự: chữ ký CMS (`CheckSignature(verifySignatureOnly)`) → `RangeSane` → **khối
  ngoài cùng phải phủ tới hết file** → có token → giải mã token → chữ ký TSA khớp → TSA về Root G2 → có
  chứng thư người ký → người ký về G1 hoặc G2 → so giờ tự khai với giờ TSA.
- Chỉ khối có `SignedEnd` lớn nhất bị bắt phủ hết file, nên nhiều chữ ký nối bản vẫn hợp lệ. Nhưng **bất kỳ
  byte nào sau chữ ký cuối đều thành `InvalidSignature`**. Bản Rust tách thành "hợp lệ, có thay đổi sau
  khi ký" (kế hoạch mục 9).
- Chuỗi CA: `X509ChainTrustMode.CustomRootTrust`, `RevocationMode.NoCheck`, `VerificationTime` = giờ TSA;
  phải kết thúc đúng ở gốc đã ghim (so thumbprint). Nhánh G1 thêm `cp.crt` vào kho phụ; mọi lần dựng chuỗi
  đều thêm `dcscag2.crt`, `cpg2.crt`.
- Xếp hạng kết luận (nặng nhất thắng): `InvalidSignature` 6 > `UntrustedSigner` 5 > `UntrustedTsa` 4 >
  `NoTimestamp` 3 > `TimeMismatch` 2 > `ValidTrusted` 1 > `NotApplicable` 0.
- Lệch giờ: nếu `|giờ tự khai − giờ TSA − 7h| ≤ 3600s` thì coi là ghi nhầm giờ địa phương thành UTC, trừ 7
  giờ rồi mới xét. Sau đó nếu giờ tự khai sau giờ TSA quá 60 giây thì `TimeMismatch`.

### 3.6 `PdfRevisionInfo.cs`

- Tìm `startxref` trong 2048 byte cuối, đi theo `/Prev`. Đọc được xref table và xref stream (`/W`, `/Index`,
  `FlateDecode`, PNG predictor ≥ 10 nhưng **chỉ kiểu lọc 0, 1, 2**; Average và Paeth bị bỏ qua). Không xử
  lý `/XRefStm` của file hybrid.
- Object trong object stream đọc qua `/N`, `/First`.
- Số generation bị bỏ qua ở mọi nơi.
- `/Encrypt` chỉ tìm trong trailer mới nhất → lỗi 1033.
- `/DocMDP`: dò chuỗi `/DocMDP` trên toàn file rồi tìm `/P 1` trong 400 ký tự sau đó → lỗi 1034. Có thể
  nhận nhầm hoặc bỏ sót (ví dụ khi `/TransformParams` là tham chiếu gián tiếp).
- Cây trang: regex `/Kids [...]`, kế thừa `MediaBox`, `CropBox`, `Rotate`. Thiếu `MediaBox` thì dùng A4.
  `CropBox` được cắt theo `MediaBox`. Góc xoay không chia hết cho 90 thì coi là 0.
- `DisplayBox`: đổi chỗ rộng và cao khi xoay 90 hoặc 270.

### 3.7 `PdfIncrementalSigner.cs`

- Số object mới bắt đầu từ `/Size`: field, giá trị chữ ký, object của khối hiển thị, widget (khi nhiều
  khối), chú thích Stamp, xref stream (nếu dùng).
- Một khối chữ ký: field và widget gộp chung một object. Nhiều khối: field có `/Kids`, mỗi widget một object.
- Chữ ký ẩn: không có khối nào thì tạo widget ở trang 1, `/Rect [0 0 0 0]`, không có `/AP`. Mọi widget `/F 4`.
- Dấu đỏ và chữ ký tươi khi ký là **chú thích `/Subtype /Stamp`** trong cùng bản cập nhật nối bản (không
  sửa content stream). Khi bật "nhồi chữ ký số vào ảnh" thì chính các ảnh đó trở thành widget chữ ký.
- AcroForm mới: `/AcroForm<</Fields[N 0 R]/SigFlags 3/DA(/Helv 0 Tf 0 g )>>`. AcroForm có sẵn: nối vào
  `/Fields`; chỉ thêm `/SigFlags 3` khi chưa có khoá này, **không sửa nếu đã có giá trị khác** (lỗi tiềm ẩn).
- Báo lỗi 1035 khi `/AcroForm`, `/Fields` hoặc `/Annots` là tham chiếu gián tiếp.
- Từ điển chữ ký: `/Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /Name /Reason
  /Location /M (D:yyyyMMddHHmmss+07'00') /ByteRange [4 số, mỗi số rộng 10] /Contents <65536 số 0>`.
- Chuỗi PDF: ASCII in được thì viết `(…)` có thoát `( ) \`; còn lại viết hex UTF-16BE có BOM `FEFF`.
- Vị trí `/Contents` và `/ByteRange` được tìm lại bằng `LastIndexOf` trên toàn văn bản (dễ vỡ); bản Rust
  ghi nhớ offset lúc ghi.
- Xref mới cùng kiểu với xref cuối. Table: mỗi dòng `%010d 00000 n\r\n`. Stream: `/W [1 4 2]`, không nén,
  có `/Index`. Trailer chép `/Root`, `/Info`, `/ID`, thêm `/Prev`. Thêm `\n` đầu nếu file gốc không kết
  thúc bằng xuống dòng.
- Đổi toạ độ sang hệ người dùng theo góc xoay (`ToUserSpace`), với `crop` là CropBox, `rect` ở hệ hiển thị:

  | Xoay | x | y | rộng | cao |
  |---|---|---|---|---|
  | 0 | `rect.X` | `rect.Y` | `rect.W` | `rect.H` |
  | 90 | `crop.W − rect.Top` | `rect.X` | `rect.H` | `rect.W` |
  | 180 | `crop.W − rect.Right` | `crop.H − rect.Top` | `rect.W` | `rect.H` |
  | 270 | `rect.Y` | `crop.H − rect.Right` | `rect.H` | `rect.W` |

  Kết quả cộng thêm `crop.X`, `crop.Y`.

### 3.8 `PdfAppearanceBuilder.cs`

- Khối chữ gồm **đúng hai dòng: CN của người ký, và thời điểm ký dạng
  `yyyy-MM-ddTHH:mm:ss.fffffff+07:00`**. Không có tiền tố "Ký bởi:", "Ngày ký:". Xem K-03.
- Kích thước khối chữ = bề rộng dòng dài nhất × `2 × chiều cao dòng` của font. Cỡ chữ theo trang =
  `cỡ cấu hình × min(rộng trang/595, cao trang/842)`.
- Cách làm: vẽ bằng PDFsharp lên một PDF tạm rồi "cấy" content stream và resources sang file thật, đánh
  lại số object. Bản Rust tự dựng Form XObject nên bỏ được bước này.
- `/Matrix` theo góc xoay: 90 → `[0 1 -1 0 0 0]`, 180 → `[-1 0 0 -1 0 0]`, 270 → `[0 -1 1 0 0 0]`.
  `/BBox [0 0 rộng cao]`.
- Ảnh: co cho vừa khung, giữ tỉ lệ, căn giữa.
- **Độ dày nét:** bán kính `r = rộng ảnh × 0,35/158 × nét/100`. Nếu `r > 0` thì vẽ ảnh 9 lần trong khung
  thu nhỏ mỗi cạnh `r`: 8 lần lệch `r` theo 8 hướng và 1 lần ở giữa. Dấu đỏ luôn dùng nét 0.
- **Độ đậm:** `f = đậm/100`; `sáng = max(0,5; 1 − (f − 1) × 0,5)`; `thấp = sáng × (1 − f)/2`;
  `cao = thấp + sáng × f`. Hai đầu bị **kẹp về [0, 1] trước khi dùng**, rồi ghi vào `/Decode` của ảnh.
  Ví dụ mặc định 140: `/Decode [0 0.96 …]`. Khi port sang xử lý điểm ảnh phải dùng hai đầu đã kẹp:
  `ra = thấp' + mẫu × (cao' − thấp')`.
- **Màu mực** (chỉ chữ ký tươi): với kênh màu `c` của mực, đầu thấp và cao thành `c + (1 − c) × mức`. Ảnh
  xám thì đổi sang bảng màu Indexed. Ảnh đã có `/Decode`, ảnh `ImageMask` và ảnh SMask không bị đụng tới.

### 3.9 `SignaturePlacementResolver.cs`

- Toạ độ đặt là tỉ lệ của khổ hiển thị, **gốc trên trái, Y hướng xuống**:
  `đáy = Top − YRatio × cao − cao khối`.
- Neo phải khi tâm khung đã kéo nằm ở nửa phải (`XRatio + WidthRatio/2 > 0,5`): mép phải của khối trùng
  mép phải của khung. Ngược lại neo trái.
- Không có vị trí: góc trên phải, lề `8pt × min(rộng/595, cao/842)`.
- Khối bị thu về tối đa bằng khổ trang rồi ép vào trong trang. Bản cũ **không cộng lề an toàn 8pt** khi ép
  (lề chỉ dùng cho vị trí mặc định); kế hoạch mục 7.4 ghi "ép khung vào trong trang (lề an toàn 8pt)".
- Khối chữ ký dùng **kích thước chữ đo được**, không dùng rộng/cao của khung đã kéo. Ảnh thì dùng đúng tỉ lệ
  rộng/cao của khung.

### 3.10 `SignService.cs`

- Liệt kê file: chỉ thư mục cấp một (không quét thư mục con), chỉ `.pdf`, bỏ tên khớp
  `_daky_\d{8}_\d{6}$`, sắp tự nhiên (đệm mỗi dãy số thành 12 chữ số, so không phân biệt hoa thường), gộp
  trùng theo đường dẫn đầy đủ.
- `AlreadySigned` được tính bằng cách **chạy toàn bộ bộ xác minh trên từng file** lúc liệt kê. Bản Rust
  chỉ quét cấu trúc chữ ký.
- Kiểm trước lô: ký thử → màu mực `^#[0-9A-Fa-f]{6}$` (lỗi 400) → ảnh tồn tại, đúng đuôi, ≤ 5MB (lỗi
  1040) → có ít nhất một PDF (lỗi 1030). Font ngoài danh sách thì về Times New Roman; cỡ chữ, độ đậm, nét
  bị kẹp vào khoảng cho phép.
- File kết quả: cùng thư mục với file gốc, `<tên>_daky_<yyyyMMdd_HHmmss>.pdf`, một mốc thời gian cho cả lô.
- Song song `clamp(số CPU, 2, 8)`. Mỗi file: đếm trang → dựng tuỳ chọn → ký → **xác minh lại; chữ ký cuối
  phải `ValidTrusted`**, không thì xoá file kết quả và báo lỗi 1025.
- Theo dõi token mỗi 2 giây; mất chứng thư thì huỷ lô. File chưa xử lý được báo lỗi với câu
  `Đã rút USB token khi đang ký nên dừng tiến trình.` hoặc `Tiến trình ký bị dừng.`
- `CheckPositions` so khung yêu cầu với khung sau khi ép, sai số 0,5pt. **Nó dùng rộng/cao của khung đã kéo,
  trong khi lúc ký thật khối chữ dùng kích thước chữ đo được**, nên cảnh báo có thể lệch với kết quả ký.
- Mặc định khung chữ ký khi thiếu kích thước: rộng `170/595`, cao `30/842`.

### 3.11 `ExcelReader.cs`

- Chỉ đọc **sheet đầu tiên**. Dòng tiêu đề là dòng đầu của vùng có dữ liệu. Ô tiêu đề trống thì bỏ cột đó.
- Ô kiểu ngày: `dd/MM/yyyy`, bất kể định dạng của ô; phần giờ bị bỏ. Ô khác: `GetFormattedString().Trim()`.
- **`GetFormattedString()` định dạng số theo vùng văn hoá của Windows**: cùng một ô ra `1,234,567` trên máy
  `en-US` và `1.234.567` trên máy `vi-VN`. Số General ra tối đa 15 chữ số có nghĩa (`1E-07`,
  `1.23456789012346E+16`). Đã đo bằng spike S4, xem `docs/spikes/S4-excel-calamine.md` và K-11.
- Bỏ dòng mà mọi cột đều trống. Số cột và số dòng là số thật trên sheet.

### 3.12 `WordTemplateEngine.cs`

- Xử lý thân văn bản, mọi header và footer. Duyệt **mọi `w:p` kể cả `w:p` nằm trong text box**, nên text
  box trong các phần này đã được xử lý. Footnote và endnote thì chưa. Xem K-05.
- Trong mỗi đoạn: nối các `w:t`, tìm `\{([^{}]+)\}`, tên trường được cắt khoảng trắng hai đầu. Thay từ
  trường cuối về đầu; giá trị đặt vào `w:t` đầu tiên mà trường chạm tới, phần còn lại của trường bị xoá;
  đặt `xml:space="preserve"`.
- Trường không có trong bảng giá trị thì giữ nguyên văn bản `{…}`.

### 3.13 `WordPdfConverter.cs`

- `Documents.Open(path, ConfirmConversions = false, ReadOnly = true, AddToRecentFiles = false)` →
  `ExportAsFixedFormat(pdf, 17)` → `Close(0)`. Một phiên Word cho cả lô, `Visible = false`,
  `DisplayAlerts = 0`, kết thúc bằng `Quit(0)`.
- Không có timeout, không kill tiến trình treo.

### 3.14 `DocumentMergeService.cs`

- Tự map: chuẩn hoá cả tên trường và tên cột (`_` → khoảng trắng, `đ` → `d`, bỏ dấu, chữ thường, chỉ giữ
  chữ và số, nối bằng một khoảng trắng) rồi so **bằng nhau hoàn toàn**.
- Tên file: `<tên mẫu>_<giá trị cột đặt tên>`; ký tự cấm và khoảng trắng thành `_`, cắt `_` và `.` hai
  đầu, tối đa 100 ký tự. Trống thì `<tên mẫu>_<số dòng>`. Trùng thì thêm `_<số dòng>`. Xem K-06.
- Thư mục kết quả: `<thư mục lưu hoặc thư mục chứa file Excel>\<tên mẫu>_<yyyyMMdd_HHmmss>\`.
- Chạy tuần tự từng dòng trong một phiên Word. Mỗi dòng: điền DOCX → Word xuất PDF → đếm trang → lập kế
  hoạch dấu → đóng ảnh → chuyển sang thư mục kết quả.
- Cảnh báo theo dòng: trường đã ghép nhưng ô trống; cột đặt tên trống; giá trị đặt tên trùng.
- Bản xem trước nằm ở `%TEMP%\SaoMai-KySo\tao-van-ban\xem-truoc\`, dọn sau 60 phút.

## 4. Khác biệt giữa kế hoạch và mã nguồn cũ

| Mã | Kế hoạch viết | Mã nguồn cũ thực tế | Xử lý |
|---|---|---|---|
| K-01 | Phụ lục B: thông báo mặc định của 400, 404, 500, 1042, 1043, 1044, 1046 | Bản cũ: "Request không hợp lệ", "Không tìm thấy trong hệ thống", "Lỗi server", "File template Word không hợp lệ", "File dữ liệu Excel không hợp lệ", "Cấu hình map trường không hợp lệ", "Sinh file thất bại". Có thêm mã 409 nhưng không nơi nào dùng | Dùng câu của kế hoạch vì bản mới có mẫu HTML và CSV. Số mã giữ nguyên. Bỏ 409 |
| K-02 | Mục 4: nhúng CA bằng `include_bytes!` và kiểm SHA-256 | `cpg2.crt` ngắt dòng Base64 ở 76 ký tự; bộ đọc PEM chặt theo RFC 7468 từ chối file này | Giữ nguyên file gốc, tự giải mã PEM dễ tính, kiểm SHA-256 trên DER. Đã có test |
| K-03 | Mục 7.3: các dòng mặc định `Ký bởi: <CN>`, `Ngày ký: dd/MM/yyyy HH:mm` | Hai dòng trần: CN, và giờ dạng ISO có 7 chữ số thập phân | **Cần xác nhận**: đây là thay đổi hiển thị trên văn bản đã ký so với bản cũ |
| K-04 | Mục 9: danh sách kết luận không có "giờ ký tự khai lệch" | Có kết luận `TimeMismatch` | Giữ `TimeMismatch` để không mất nghiệp vụ |
| K-05 | Mục 8.2: text box là thứ "bản cũ chưa xử lý" | Text box trong thân, header, footer đã được xử lý; chỉ footnote/endnote chưa | Sửa lại nhận định; phần việc thật sự mới là footnote, endnote và biến ảnh |
| K-06 | Mục 8.5: mẫu tên `{so}_{hoten}`, bỏ dấu, ký tự cấm thành `-`, trùng thêm `(1)`, trống dùng `vanban_<số dòng>` | Một cột đặt tên, có tiền tố tên mẫu, giữ dấu, ký tự cấm thành `_`, trùng thêm `_<số dòng>` | **Cần xác nhận**: tên file kết quả sẽ khác bản cũ |
| K-07 | Mục 7.4: ép khung vào trang với lề an toàn 8pt | Ép sát mép trang, lề 8pt chỉ dùng cho vị trí mặc định | Theo kế hoạch; ghi nhận là thay đổi nhỏ về vị trí |
| K-08 | D4: file đã có chữ ký vẫn hiện trong danh sách | Bản cũ ẩn hẳn file `_daky_…` | **Cần xác nhận** cách xử lý file `_daky_…` do bản cũ để lại trong thư mục người dùng |
| K-09 | Mục 6.3: `certificates` = người ký + CA trung gian tìm được | `WholeChain` phụ thuộc kho chứng thư của máy | Dùng CA đã ghim để kết quả ổn định |
| K-11 | Mục 8.1: ví dụ `1.234.567`, ngầm coi kết quả đọc số là cố định | Kết quả phụ thuộc vùng văn hoá của máy (đo ở spike S4: 16/57 ô khác nhau giữa `en-US` và `vi-VN`) | **Cần xác nhận** chính sách vùng văn hoá (Q-08) |
| K-12 | Mục 10.2: `JobEvent` | Giao diện cũ còn dùng `percent`, chi tiết chữ ký từng file (`signatures[]`, `signedTime`, `trustedTime`), `rowNumber` của tạo văn bản | Bổ sung các trường này khi định nghĩa IPC (giai đoạn 2) |
| K-13 | Mục 11: `config.json` mẫu | Không có chỗ cho vị trí kéo tay dùng chung (`signaturePosition`) và vị trí riêng từng file (`fileOverrides`) mà bản cũ có | Đã thêm `signature.position` (TD-37), `signature.filePositions` + `perFile` (TD-38) |
| K-14 | Mục 5: "dùng lại logic kéo thả của `stamp-designer`" | Thả ảnh mẫu vào trang dùng kéo thả HTML5; trên Windows Tauri chỉ cho HTML5 drag-drop khi tắt `dragDropEnabled`, mà tắt thì mất sự kiện kéo thả file có đường dẫn thật | Viết lại phần thả ảnh bằng pointer events; công thức giữ nguyên. Cần thử khi có khung Tauri |
| K-15 | Phụ lục A.5: 5 font, trong đó có Georgia | Georgia của Windows thiếu glyph cho 78 ký tự tiếng Việt thử nghiệm (đo ở spike S3); chữ ra ô vuông. Bản cũ dùng cùng file font, chưa kiểm trên 1.4.0 | **Cần xác nhận** cách xử lý Georgia (Q-09) |
| K-16 | Mục 15.2 coi file hybrid là một loại fixture trong nhiều loại | PDF do Word xuất (đầu vào phổ biến nhất, kể cả từ Tạo văn bản) **là file hybrid-reference**. Bản cũ không đọc `/XRefStm`. pyHanko ở chế độ chặt từ chối kiểm chữ ký trên loại file này (đo ở spike S2) | Bộ đọc phải hiểu `/XRefStm`; CI kiểm các file này bằng `strict=False`; đưa PDF do Word xuất vào fixtures |
| K-10 | Phụ lục A.5 | Thiếu `DarknessBrightnessFactor = 0,5`, `DarknessMinBrightness = 0,5` | Đã thêm vào `consts::stamp_image` |

## 5. Lỗi và giới hạn của bản cũ cần sửa khi port

1. `/SigFlags` có sẵn với giá trị khác 3 không được cập nhật.
2. Generation number luôn là 0 khi ghi đè object và trong xref.
3. Không hỗ trợ `/AcroForm`, `/Fields`, `/Annots` gián tiếp; không hỗ trợ file hybrid (`/XRefStm`).
4. Predictor PNG kiểu Average, Paeth trong xref stream bị bỏ qua.
5. `/DocMDP` và `/Encrypt` nhận diện bằng regex.
6. Không kiểm thu hồi; không kiểm chữ ký token TSA ngay khi nhận.
7. `ECDsaCng` không được gắn HWND cho hộp PIN.
8. So hạn chứng thư bằng giờ không có múi giờ.
9. Liệt kê file chạy cả bộ xác minh; kiểm tra vị trí dùng kích thước khác lúc ký thật.
10. Word COM không có timeout.
