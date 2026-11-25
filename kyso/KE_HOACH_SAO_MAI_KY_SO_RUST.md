# Kế hoạch xây dựng Sao Mai Ký số bằng Rust

> Phiên bản tài liệu: 0.1 (bản nháp để chốt) · Ngày lập: 18/11/2026
>
> Nguồn tham chiếu:
> - Giao diện, chức năng: `App_Ky_So/sao-mai-ky-so_1.html` (prototype v1.5.0)
> - Nghiệp vụ, thuật toán: `Kytl-main/` (bản .NET 9 + Angular v1.4.0 đang chạy thật)
> - Mẫu dữ liệu: `Kytl-main/mau-tao-van-ban/` (3 cặp mẫu .docx + .xlsx)

---

## Mục lục

1. [Mục tiêu và phạm vi](#1-mục-tiêu-và-phạm-vi)
2. [Các quyết định cần chốt trước khi code](#2-các-quyết-định-cần-chốt-trước-khi-code)
3. [Kiến trúc tổng thể](#3-kiến-trúc-tổng-thể)
4. [Cấu trúc repo và các crate](#4-cấu-trúc-repo-và-các-crate)
5. [Ánh xạ từ backend .NET sang Rust](#5-ánh-xạ-từ-backend-net-sang-rust)
6. [Thiết kế chi tiết lõi ký số](#6-thiết-kế-chi-tiết-lõi-ký-số)
7. [Thiết kế chi tiết lõi PDF](#7-thiết-kế-chi-tiết-lõi-pdf)
8. [Thiết kế chi tiết Tạo văn bản](#8-thiết-kế-chi-tiết-tạo-văn-bản)
9. [Thiết kế chi tiết Kiểm tra chữ ký](#9-thiết-kế-chi-tiết-kiểm-tra-chữ-ký)
10. [Giao tiếp giao diện và lõi (IPC)](#10-giao-tiếp-giao-diện-và-lõi-ipc)
11. [Dữ liệu trên máy người dùng](#11-dữ-liệu-trên-máy-người-dùng)
12. [Danh sách chức năng chi tiết theo màn hình](#12-danh-sách-chức-năng-chi-tiết-theo-màn-hình)
13. [Chức năng bản cũ không có trong prototype](#13-chức-năng-bản-cũ-không-có-trong-prototype)
14. [Lộ trình theo giai đoạn](#14-lộ-trình-theo-giai-đoạn)
15. [Kiểm thử](#15-kiểm-thử)
16. [Đóng gói, cài đặt, cập nhật](#16-đóng-gói-cài-đặt-cập-nhật)
17. [Bảo mật](#17-bảo-mật)
18. [Rủi ro và cách giảm](#18-rủi-ro-và-cách-giảm)
19. [Quy ước code và quy trình](#19-quy-ước-code-và-quy-trình)
20. [Chuẩn bị trước khi bắt đầu](#20-chuẩn-bị-trước-khi-bắt-đầu)
21. [Phụ lục A: Hằng số mang từ bản cũ](#phụ-lục-a-hằng-số-mang-từ-bản-cũ)
22. [Phụ lục B: Mã lỗi](#phụ-lục-b-mã-lỗi)
23. [Phụ lục C: Danh sách crate dự kiến](#phụ-lục-c-danh-sách-crate-dự-kiến)

---

## 1. Mục tiêu và phạm vi

### 1.1 Mục tiêu

Viết lại ứng dụng **Sao Mai Ký số** thành ứng dụng desktop Windows, lõi bằng **Rust**, có:

- Giao diện và luồng thao tác **giống prototype** `sao-mai-ky-so_1.html`.
- Toàn bộ nghiệp vụ đã chạy ổn ở bản .NET 1.4.0 (ký PAdES, dấu thời gian, kiểm tra chuỗi CA Ban Cơ yếu, ký nối bản, tạo văn bản từ Word + Excel).
- Các chức năng mới trong prototype: Trang chủ, Kiểm tra chữ ký, mẫu chữ ký theo khổ giấy, nhật ký, ETA, quản lý token, hướng dẫn, cập nhật.

### 1.2 Vì sao viết lại

| Vấn đề ở bản .NET 1.4.0 | Bản Rust giải quyết thế nào |
|---|---|
| Mở một web server trên `127.0.0.1`, CORS `*`, không xác thực; trang web lạ có thể dò cổng để đọc PDF hoặc kích hoạt ký | Không còn HTTP server. Giao diện gọi lõi qua IPC của Tauri, có phân quyền (capabilities) |
| File kéo thả phải tải lên bản sao trong `%Temp%` vì trình duyệt không biết đường dẫn thật | Tauri trả về **đường dẫn thật** khi kéo thả, bỏ hẳn cơ chế tải lên |
| Bộ cài khoảng 70MB+ do .NET self-contained | Ứng dụng Tauri khoảng 10–20MB (chưa tính driver) |
| Parser PDF bằng regex, không xử lý được `/Annots`, `/AcroForm` gián tiếp, `/DocMDP` trong object stream | Đọc cấu trúc bằng parser thật (`lopdf`), chỉ phần ghi nối bản là tự viết |
| Không kiểm tra thu hồi chứng thư (`RevocationMode.NoCheck`) | Dựng chuỗi bằng CryptoAPI của Windows, có kiểm tra OCSP/CRL |
| Bộ xác minh coi mọi byte nối thêm sau chữ ký là chữ ký hỏng | Phân biệt "hợp lệ, có thay đổi sau khi ký" và "hỏng", giống cách Adobe báo |
| Không có test tự động | Có test đơn vị, bộ PDF mẫu, CI |

### 1.3 Ngoài phạm vi bản 1.5.0

- Ký file DOCX theo chuẩn chữ ký OOXML (xem quyết định D5). Bản 1.5.0 chỉ hỗ trợ "chuyển DOCX sang PDF rồi ký".
- Ký LTV (nhúng OCSP/CRL vào DSS, PAdES B-LT/B-LTA). Để bản 1.6.
- Chạy trên macOS/Linux. Bản này chỉ hỗ trợ Windows 10/11 x64, vì phụ thuộc Windows Cert Store, CNG/CAPI và Word COM.
- Ký từ xa, HSM, ký qua dịch vụ đám mây.

---

## 2. Các quyết định cần chốt trước khi code

Mỗi quyết định có **phương án đề xuất** (in đậm). Kế hoạch bên dưới viết theo phương án đề xuất; chốt khác thì sửa lại các mục liên quan.

### D1. Công nghệ giao diện

| Phương án | Mô tả | Ưu | Nhược |
|---|---|---|---|
| **A. Tauri 2 + giao diện web (TypeScript)** | Vỏ ứng dụng và toàn bộ lõi bằng Rust; giao diện HTML/CSS chạy trong WebView2 | Giữ **gần như nguyên** HTML/CSS của prototype; dùng lại được pdf.js và logic kéo thả từ `ktl.fe`; nhóm đã quen Angular | Vẫn có lớp web, cần WebView2 |
| B. Tauri 2 + Leptos (Rust biên dịch ra WASM) | Như A nhưng giao diện viết bằng Rust | Toàn bộ code là Rust | Phải viết lại toàn bộ giao diện; hệ sinh thái component ít; dùng pdf.js qua JS interop |
| C. UI native: Slint hoặc egui | Không dùng web | Không cần WebView2, file nhỏ nhất | Phải vẽ lại toàn bộ giao diện, khó giống prototype; phải tự làm xem trước PDF (pdfium) và kéo thả khung; tốn công nhất |

**Đề xuất: A**, giao diện viết bằng **Angular 21** (standalone + signals) với CSS lấy từ prototype, bỏ PrimeNG để giao diện giống prototype hơn. Nếu nhóm muốn framework nhẹ hơn thì dùng SolidJS hoặc Svelte 5; kiến trúc lõi không đổi.

> Ghi chú: trong prototype có dòng chú thích "cùng thông số `logo_sao_mai.rs` của prototype". Nếu đã có một prototype Rust native trước đó (egui/Slint), cần xem lại trước khi chốt D1.

### D2. Mã PIN

Prototype có hộp **Nhập PIN của app**, "Lưu mã PIN trên máy", "Tự nhập lại PIN khi hết phiên". Bản cũ cố ý **không chạm vào PIN**; hộp PIN là của middleware token.

**Đề xuất:**
- **1.5.0:** giữ hộp PIN của middleware như bản cũ. Hộp "Nhập PIN" trong prototype đổi thành hộp **"Xác nhận ký"** (thông tin token, số file, nút Xác nhận). Middleware tự hiện hộp PIN, được gắn vào cửa sổ app.
- **1.6.0 (tuỳ chọn):** chỉ làm "Lưu PIN / Tự nhập lại" nếu spike S6 chứng minh được middleware nhận PIN qua `NCRYPT_PIN_PROPERTY` (CNG) hoặc `PP_SIGNATURE_PIN` (CAPI) với cả token bit4id và SafeNet. PIN được mã hoá bằng DPAPI (phạm vi CurrentUser), tắt sẵn, có cảnh báo "chỉ bật trên máy cá nhân".

### D3. Xoá hoặc thay file gốc

Prototype có checkbox "Xóa file gốc" và tuỳ chọn "Ghi đè". Bản cũ có quy tắc **không bao giờ ghi đè file gốc**.

**Đề xuất:** mặc định giữ file gốc, lưu file đã ký vào thư mục `DaKySo`. Tuỳ chọn **"Thay file gốc"** chỉ thực hiện **sau khi** chữ ký đã được kiểm tra hợp lệ; file gốc được đưa vào **Thùng rác** (Recycle Bin, khôi phục được) chứ không xoá hẳn; lần đầu bật có hộp xác nhận.

### D4. Thư mục và tên file kết quả

Prototype: lưu vào `<thư mục nguồn>\DaKySo\`, hậu tố `_signed`, file lỗi có hậu tố `_loi`, trùng tên thì hỏi Ghi đè hoặc Giữ cả hai.

**Đề xuất:**
- Lưu `<thư mục nguồn>\DaKySo\<tên><hậu tố>.pdf`, hậu tố mặc định `_signed`, đổi được trong Cấu hình. Có thể chọn một thư mục lưu cố định.
- Khi quét thư mục **không quét thư mục con `DaKySo`**, nên đổi hậu tố cũng không gây ký chồng (đây là lỗi mà README bản cũ cảnh báo).
- File đã có chữ ký vẫn hiện trong danh sách, có nhãn "Đã có N chữ ký" để người dùng biết; vẫn cho ký thêm (ký nháy rồi ký chính là trường hợp hợp lệ).
- Hậu tố `_loi`: **không đổi tên file gốc**. Mặc định chỉ ghi lỗi vào Nhật ký. Tuỳ chọn "Chép file lỗi vào DaKySo với hậu tố `_loi`" để gom file cần xử lý lại.

### D5. Ký DOCX

Ký DOCX đúng chuẩn cần chữ ký XML (XML-DSig + XAdES trong gói OOXML, có canonicalization C14N). Đây là một bộ ký riêng, rủi ro cao, thư viện Rust còn yếu.

**Đề xuất:** 1.5.0 khi người dùng chọn file `.docx` ở màn Ký số thì app **chuyển sang PDF qua Word rồi ký PDF**, có thông báo rõ. Ký DOCX gốc để giai đoạn sau, cần spike riêng.

### D6. Chuẩn chữ ký

Bản cũ đưa thuộc tính `signing-time` vào CMS. PAdES baseline (ETSI EN 319 142-1) quy định **không** đưa thuộc tính này; thời điểm ký nằm ở `/M` của từ điển chữ ký.

**Đề xuất:** tuân thủ PAdES B-T: bỏ `signing-time`, giữ `signingCertificateV2`, có dấu thời gian RFC 3161. Kiểm tra lại bằng Adobe Acrobat Reader và một bộ kiểm tra độc lập (pyHanko). Bộ xác minh vẫn đọc được `signing-time` của file do phần mềm khác ký (giữ cách xử lý lệch giờ +7 của bản cũ).

### D7. Kiểm tra cập nhật

Prototype có "Kiểm tra cập nhật". Như vậy app sẽ có kết nối mạng thứ hai ngoài TSA.

**Đề xuất:** dùng `tauri-plugin-updater` (gói cập nhật có chữ ký), máy chủ là một file JSON tĩnh trên `saomaiso.com` hoặc GitHub Releases. Chỉ kiểm tra khi người dùng bấm, hoặc lúc mở app nếu người dùng bật. Cập nhật lại phần cam kết riêng tư: "chỉ gửi giá trị băm tới TSA, kiểm tra thu hồi với CA, kiểm tra phiên bản với máy chủ Sao Mai; không gửi nội dung văn bản".

### D8. Mẫu HTML và xuất DOCX trong Tạo văn bản

**Đề xuất:** làm cả hai ở 1.5.0 vì chi phí thấp:
- Xuất DOCX: chỉ là file DOCX đã điền, không cần Word.
- Mẫu HTML: điền biến vào HTML rồi in ra PDF bằng chính WebView2 (`PrintToPdf`), **không cần cài Word**. Cần spike S5.

### D9. Thanh tiêu đề

Prototype vẽ thanh tiêu đề riêng. **Đề xuất:** 1.5.0 dùng thanh tiêu đề native của Windows (không lỗi snap layout, kéo cửa sổ, DPI). Thanh tiêu đề tự vẽ (`decorations: false`) để sau nếu thiết kế bắt buộc.

---

## 3. Kiến trúc tổng thể

```
┌──────────────────────────── SaoMai-KySo.exe (Tauri 2) ────────────────────────────┐
│                                                                                   │
│  WebView2: giao diện (Angular + CSS từ prototype + pdf.js)                         │
│     │  invoke("sign_batch", …)            ▲ Channel<SignEvent> (tiến độ)           │
│     ▼                                      │                                       │
│  smks-app (src-tauri): commands, quản lý job, state, cấu hình, menu, hộp thoại      │
│     │                                                                             │
│     ├── smks-crypto : kho chứng thư, khoá CNG/CAPI, CMS/CAdES, TSA, dựng chuỗi CA  │
│     ├── smks-pdf    : đọc PDF, ghi nối bản, khối chữ ký, đóng ảnh, đo trang         │
│     ├── smks-merge  : DOCX template, Excel/CSV, Word COM, HTML → PDF               │
│     └── smks-core   : kiểu dữ liệu, mã lỗi, hằng số, cấu hình, lập kế hoạch dấu     │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘
        │ Windows Cert Store + CNG/CAPI            │ HTTP                 │ COM
        ▼                                          ▼                      ▼
  Middleware token (bit4id / SafeNet)       tsa.ca.gov.vn         Microsoft Word
```

Nguyên tắc:

1. **Lõi không phụ thuộc giao diện.** Các crate `smks-*` không biết Tauri. Mọi chức năng chạy được qua `smks-cli`, dùng để test và gỡ lỗi.
2. **Tác vụ dài chạy nền** trên tokio, báo tiến độ qua `tauri::ipc::Channel`, huỷ được qua `CancellationToken`. Ba loại sự kiện giữ nguyên như bản cũ: `manifest`, `file`, `summary`.
3. **Không bao giờ ghi đè file gốc** (trừ tuỳ chọn D3). Ghi ra `*.tmp` rồi đổi tên.
4. **Vị trí lưu theo tỉ lệ 0–1** so với khổ trang hiển thị, như bản cũ.
5. **Thông báo lỗi tiếng Việt** hiển thị thẳng cho người dùng, nói rõ nguyên nhân và cách xử lý.

---

## 4. Cấu trúc repo và các crate

```
sao-mai-ky-so/
├─ Cargo.toml                    # workspace
├─ rust-toolchain.toml           # ghim phiên bản Rust stable
├─ crates/
│  ├─ smks-core/                 # kiểu dữ liệu, lỗi, hằng số, config, stamp planner, đặt tên file
│  ├─ smks-crypto/               # cert store, signing key, cms, tsa, chain, verify (phần mật mã)
│  ├─ smks-pdf/                  # reader, incremental writer, appearance, stamp, geometry, sigscan
│  ├─ smks-merge/                # docx, html template, table (xlsx/csv), convert (word, webview)
│  └─ smks-cli/                  # công cụ dòng lệnh để test: sign, verify, merge, inspect
├─ app/
│  ├─ src-tauri/                 # crate smks-app: main.rs, commands/, jobs/, state, capabilities/
│  └─ ui/                        # Angular: shell, pages/, components/, services/, styles/
├─ certs/                        # rootca.crt, cp.crt, rootcag2.crt, dcscag2.crt, cpg2.crt
├─ fixtures/                     # PDF, DOCX, XLSX mẫu cho test (không chứa dữ liệu thật)
├─ installer/                    # script NSIS, hook cài driver, drivers/ (không commit)
├─ docs/                         # tài liệu này, ADR, hướng dẫn sử dụng (nguồn cho màn Hướng dẫn)
└─ .github/workflows/            # CI
```

Chứng thư CA được nhúng vào binary bằng `include_bytes!` và kiểm tra SHA-256 lúc khởi động. Như vậy không thể xảy ra lỗi "thiếu file `.crt` nên mọi chữ ký bị coi là không tin cậy" như bản cũ.

---

## 5. Ánh xạ từ backend .NET sang Rust

| Bản .NET (`Kytl-main/ktl.be/…`) | Bản Rust | Ghi chú |
|---|---|---|
| `CertificateProvider.cs` | `smks-crypto::store` | `CertOpenStore` + `CertEnumCertificatesInStore` trên `CurrentUser\My`, `LocalMachine\My`; gộp theo thumbprint |
| `StoreSigningKey.cs` | `smks-crypto::key::{cng, capi}` | `CryptAcquireCertificatePrivateKey`, `NCryptSignHash` hoặc `CryptSignHashW`; gắn HWND cho hộp PIN |
| `PdfSigner.cs` (`CadesSigner`) | `smks-crypto::cms` | Dựng SignedData detached, SHA-256, `signingCertificateV2`, nhúng chuỗi chứng thư |
| `TimestampClient.cs` | `smks-crypto::tsa` | RFC 3161, nonce, giới hạn 8 yêu cầu đồng thời, timeout 180 giây |
| `SignatureVerifier.cs` | `smks-crypto::verify` + `smks-pdf::sigscan` | Thêm kiểm tra thu hồi, thêm phân loại "có thay đổi sau khi ký" |
| `PdfRevisionInfo.cs` | `smks-pdf::reader` | `lopdf` để đọc object; tự đọc `startxref`/trailer/`/Prev` để biết offset |
| `PdfIncrementalSigner.cs` | `smks-pdf::incremental` | Port gần như nguyên thuật toán: ghi đè Catalog, Page, thêm field, chừa `/Contents`, điền `/ByteRange` |
| `PdfObjectWriter.cs` | `smks-pdf::writer` | Dùng `pdf-writer` (cho phép chọn số object) |
| `PdfAppearanceBuilder.cs` | `smks-pdf::appearance` | Không cần "cấy" từ PDF tạm nữa: tự dựng Form XObject, nhúng subset font bằng `subsetter` + `ttf-parser` |
| `PdfImageStamper.cs` | `smks-pdf::stamp` | Đóng ảnh khi tạo văn bản (file mới, chưa có chữ ký) |
| `PdfPageInspector.cs`, `SignaturePlacementResolver.cs` | `smks-pdf::geometry` | Đo khổ trang, xoay trang, dịch khối vào trong trang, nhận diện khổ A1–A5 |
| `StampPlanner.cs` | `smks-core::stamp_plan` | Quy tắc trang: cụ thể, tất cả, chẵn, lẻ, `1,3,5-7` |
| `WordTemplateEngine.cs` | `smks-merge::docx` | `zip` + `quick-xml`, xử lý trường bị Word tách ra nhiều run |
| `ExcelReader.cs` | `smks-merge::table` | `calamine` (xlsx/xls), `csv` + `encoding_rs` |
| `WordPdfConverter.cs` | `smks-merge::convert::word` | COM late-binding (`IDispatch`) qua crate `windows`, chạy trên luồng STA riêng |
| `DocumentMergeService.cs` | `smks-app::jobs::merge` | |
| `SignService.cs` | `smks-app::jobs::sign` | |
| `FileUploadService.cs` | **Bỏ** | Tauri cho đường dẫn thật khi kéo thả |
| Controllers + SSE | `smks-app::commands` + `Channel` | |
| `ktl.be.desktop/Program.cs` | `app/src-tauri/src/main.rs` | Hộp thoại native, `CryptSetProvParam(PP_CLIENT_HWND)`, mở thư mục |
| `SigningConstants.cs`, `SignatureConstants.cs`… | `smks-core::consts` | Xem Phụ lục A |
| `ErrorCodes.cs`, `ErrorMessages.cs` | `smks-core::error` | Giữ mã 1025–1048, mã mới bắt đầu từ 1049 (Phụ lục B) |
| `installer/SaoMai-KySo.nsi` | `installer/SaoMai-KySo.nsi` (sửa lại) | Giữ logic nhận diện driver đã chạy ổn |
| `ktl.fe` (Angular) | `app/ui` | Dùng lại: models, logic kéo thả của `stamp-designer`, `pdf-pages` (pdf.js), `utils` |

---

## 6. Thiết kế chi tiết lõi ký số

### 6.1 Liệt kê chứng thư (`store`)

- Quét `CurrentUser\My` rồi `LocalMachine\My`; gộp theo thumbprint, ưu tiên bản ký được.
- Với mỗi chứng thư lấy: CN, Subject, Issuer, serial, thumbprint, hạn dùng, có khoá bí mật hay không, tên provider (CNG: `NCRYPT_PROVIDER_HANDLE_PROPERTY`; CAPI: `PP_NAME`).
- **Có phải token phần cứng:** tên provider chứa một trong các dấu hiệu `smart card, bit4id, vgca, token, etoken, safenet, gemalto, thales, epass, feitian` và không phải provider phần mềm của Microsoft (danh sách ở Phụ lục A).
- **Ký được** khi: có khoá bí mật, còn hạn, KeyUsage có `digitalSignature` hoặc `nonRepudiation` (hoặc không có KeyUsage), chuỗi về Root G1 (qua `cp.crt`) hoặc Root G2 (qua `dcscag2.crt`, `cpg2.crt`).
- Lỗi cụ thể cho từng trường hợp không ký được (giữ nguyên câu chữ của bản cũ).

### 6.2 Khoá ký (`key`)

```rust
pub trait SigningKey: Send + Sync {
    fn certificate(&self) -> &Certificate;          // x509-cert
    fn algorithm(&self) -> SigAlg;                  // RsaPkcs1Sha256 | EcdsaP256Sha256 | EcdsaP384Sha384
    fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>>;
}
```

- `CngKey`: `NCryptSignHash` với `BCRYPT_PKCS1_PADDING_INFO { pszAlgId: "SHA256" }`. Trước khi ký đặt thuộc tính `NCRYPT_WINDOW_HANDLE_PROPERTY` ("HWND Handle") bằng HWND cửa sổ chính để hộp PIN không bị chìm.
- `CapiKey`: `CryptCreateHash(CALG_SHA_256)` + `CryptSetHashParam(HP_HASHVAL)` + `CryptSignHashW`; nhớ đảo thứ tự byte (CAPI trả little-endian). Gọi `CryptSetProvParam(NULL, PP_CLIENT_HWND, hwnd)` khi tạo cửa sổ (giống bản cũ).
- `SoftwareKey` (chỉ bật với feature `dev-keys`, **không bao giờ có trong bản phát hành**): đọc PFX để test và chạy CI.
- **Kiểm tra token trước lô** (giống `Verify` của bản cũ): ký thử 32 byte ngẫu nhiên. Đây là lúc middleware hỏi PIN. Kết quả lưu trong `TokenSession` để dùng cho cả lô, tránh hỏi PIN hai lần (bản cũ bị do đăng ký Scoped).
- **Mọi lệnh ký tuần tự** (một `Mutex` cho mỗi token), vì token chỉ xử lý được một lệnh một lúc. Gọi TSA thì chạy song song.
- **Theo dõi token:** cứ 2 giây kiểm tra chứng thư còn trong kho không. Mất thì huỷ lô, báo rõ file nào chưa ký. Nếu được thì lắng nghe thêm `WM_DEVICECHANGE` để phản ứng nhanh hơn.

### 6.3 Dựng CMS/CAdES (`cms`)

Đầu vào: các byte theo `/ByteRange`. Các bước:

1. `messageDigest = SHA-256(dữ liệu)`.
2. Signed attributes (đúng thứ tự DER SET OF): `contentType = id-data`, `messageDigest`, `signingCertificateV2` (ESSCertIDv2 với hash SHA-256 của chứng thư người ký). Không đưa `signing-time` (quyết định D6).
3. Ký `SHA-256(DER(signedAttrs))` bằng `SigningKey`.
4. Gửi `SHA-256(signatureValue)` tới TSA, nhận TimeStampToken.
5. Thêm unsigned attribute `id-aa-signatureTimeStampToken` (`1.2.840.113549.1.9.16.2.14`).
6. `certificates` = chứng thư người ký + CA trung gian tìm được (giống `X509IncludeOption.WholeChain`).
7. Mã hoá DER. Nếu dài hơn chỗ trống `/Contents` (32KB) thì báo lỗi.

Thư viện: dùng các kiểu ASN.1 của `cms`, `x509-cert`, `der`, `spki`. Nếu builder của crate `cms` không hỗ trợ ký ngoài (external signer) và thêm unsigned attribute sau khi ký, thì tự dựng `SignerInfo` bằng `der` (khoảng 200 dòng). Spike S2 sẽ quyết định cách nào.

### 6.4 Dấu thời gian (`tsa`)

- `TimeStampReq { version 1, messageImprint SHA-256, nonce 64-bit ngẫu nhiên, certReq = true }`, `Content-Type: application/timestamp-query`, POST tới `http://tsa.ca.gov.vn/`.
- Kiểm tra phản hồi: `status` granted, nonce khớp, messageImprint khớp, chữ ký của token hợp lệ, chứng thư TSA có EKU `timeStamping` (`1.3.6.1.5.5.7.3.8`) và chuỗi về Root G2.
- Semaphore giới hạn 8 yêu cầu đồng thời; timeout 180 giây; thử lại 1 lần nếu lỗi mạng (không thử lại nếu TSA từ chối).
- Dùng HTTP thường là chấp nhận được vì token có chữ ký và được kiểm tra; ghi chú điều này trong tài liệu bảo mật.

### 6.5 Dựng và kiểm tra chuỗi chứng thư (`chain`)

Dùng CryptoAPI của Windows thay cho tự viết (chính .NET cũng dùng cách này):

- `CertCreateCertificateChainEngine` với `hExclusiveRoot` = Root G1 hoặc Root G2 đã ghim, `hExclusiveTrustedPeople` rỗng. Tạo hai engine, một cho mỗi nhánh.
- `CertGetCertificateChain` với store phụ chứa CA trung gian và chứng thư trong CMS, `pTime` = thời điểm TSA đóng dấu.
- Thu hồi: `CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT`. Kết quả chia ba: *không bị thu hồi*, *đã thu hồi*, *không kiểm tra được (offline)*. Trường hợp offline thì vẫn ký và báo "chưa kiểm tra được thu hồi".
- Ghim: SHA-256 của các chứng thư CA phải khớp với Phụ lục A, không khớp thì từ chối chạy nhánh đó.

### 6.6 Luồng một lô ký

```
1. Người dùng bấm Ký → giao diện gọi sign_batch(request, channel)
2. Kiểm tra cấu hình (màu, cỡ chữ, ảnh ≤ 5MB, png/jpg) → lỗi thì trả ngay, không mở job
3. Mở TokenSession (ký thử, hỏi PIN) → lỗi thì trả lỗi SignTokenInvalid
4. Liệt kê file: sắp xếp tự nhiên (số được đệm 0), bỏ thư mục DaKySo, gộp trùng
5. Gửi sự kiện manifest (danh sách file, thư mục kết quả)
6. Xử lý song song (2–8 luồng theo số CPU):
     đọc file → kiểm tra (/Encrypt, /DocMDP P=1, cấu trúc) → chọn mẫu theo khổ trang
     → dựng khối chữ ký → ghi nối bản → CMS (khoá tuần tự, TSA song song)
     → ghi .tmp → kiểm tra lại chữ ký vừa ký → hợp lệ thì đổi tên, không thì xoá
     → gửi sự kiện file (ok / lỗi + lý do)
7. Token bị rút hoặc người dùng bấm Dừng → huỷ; file chưa xử lý được báo trạng thái "chưa ký"
8. Gửi sự kiện summary (tổng, thành công, lỗi, thời gian chạy, thư mục)
9. Ký tiếp: gọi lại sign_batch với danh sách file còn lại (giao diện giữ trạng thái)
```

---

## 7. Thiết kế chi tiết lõi PDF

### 7.1 Đọc (`reader`)

- Tự đọc phần đuôi file: `startxref`, chuỗi xref table/stream qua `/Prev`, trailer mới nhất, `/Size`, `/Root`, `/Info`, `/ID`, generation number của từng object.
- Dùng `lopdf` để resolve object (kể cả object nằm trong object stream, tham chiếu gián tiếp), duyệt cây trang có kế thừa `MediaBox`/`CropBox`/`Rotate`.
- Từ chối khi: có `/Encrypt` (lỗi 1033); có chữ ký chứng thực `/Perms /DocMDP` với `/P 1` (lỗi 1034, đọc đúng qua `/Reference /TransformParams`, không dò bằng regex).
- Nếu `lopdf` không đọc được file (PDF hỏng nhẹ) thì thử parser dự phòng port từ `PdfRevisionInfo.cs`; vẫn không được thì báo lỗi 1035.

### 7.2 Ghi nối bản (`incremental`)

Port thuật toán `PdfIncrementalSigner.cs`, sửa các giới hạn:

- Ghi lại Catalog: thêm hoặc cập nhật `/AcroForm` (`/Fields`, `/SigFlags 3`). Nếu `/AcroForm` hoặc `/Fields` là tham chiếu gián tiếp thì ghi lại chính object đó (bản cũ báo lỗi ở trường hợp này).
- Ghi lại Page: thêm widget vào `/Annots`. Nếu `/Annots` gián tiếp thì ghi lại mảng đó.
- Giữ **generation number** đúng khi ghi đè object (bản cũ mặc định là 0).
- **Một field chữ ký, nhiều widget** (qua `/Kids`) cho chế độ "Tất cả các trang (ký nháy)".
- Chữ ký ẩn: widget có `/Rect [0 0 0 0]`, cờ `/F 4`.
- `/V` → từ điển `/Sig`: `/Filter /Adobe.PPKLite`, `/SubFilter /ETSI.CAdES.detached`, `/Name`, `/Reason`, `/Location`, `/M (D:yyyyMMddHHmmss+07'00')`, `/ByteRange` (chỗ trống cố định 4 × 10 ký tự), `/Contents <…32768 byte…>`.
- Xref mới cùng kiểu với xref cuối của file (table hoặc stream `/W [1 4 2]`), có `/Prev`.
- Thêm `\n` nếu file gốc không kết thúc bằng xuống dòng.

### 7.3 Khối chữ ký (`appearance`)

Bốn kiểu hiển thị như prototype:

| Kiểu | Nội dung Form XObject |
|---|---|
| Ảnh + thông tin | Ảnh (dấu hoặc chữ ký) bên trái, chữ bên phải |
| Chỉ ảnh | Ảnh phủ kín khung (tương đương `EmbedSignatureInImage` bản cũ) |
| Chỉ thông tin | Chữ |
| Ẩn | Chữ ký không hiện trên trang |

- **Chữ:** mặc định các dòng `Ký bởi: <CN>`, `Ngày ký: dd/MM/yyyy HH:mm`, có thể thêm `Serial`, `Lý do`, `Nơi ký`. Font Times New Roman, Arial, Verdana, Georgia, Courier New lấy từ `C:\Windows\Fonts`. Nhúng **subset** dạng Type0/CIDFontType2, Identity-H, có `/ToUnicode` để copy chữ ra được. Chuẩn hoá chuỗi NFC trước khi vẽ (tiếng Việt dựng sẵn). Cỡ 6–24pt, tự giảm cỡ chữ nếu tràn khung.
- **Ảnh:** giải mã PNG/JPG bằng `image`; kênh alpha tách thành `/SMask`. Xử lý **độ đậm** (40–250, mặc định 140), **độ dày nét** (0–200, mặc định 100; giãn alpha theo 8 hướng như bản cũ) và **màu mực** (#RRGGBB) trên điểm ảnh trước khi nén Flate. Cách này đơn giản hơn cách sửa `/Decode` + palette của bản cũ.
- **Trang xoay** 90/180/270: chuyển toạ độ khung từ hệ hiển thị sang hệ người dùng và thêm `/Matrix` cho Form XObject (port `ToUserSpace`).
- **Cỡ chữ theo khổ trang:** cỡ chữ cấu hình tính cho A4 (595×842pt), nhân theo tỉ lệ khổ thật (giống bản cũ).

### 7.4 Hình học trang và mẫu theo khổ giấy (`geometry`)

- Khung hiển thị = CropBox ∩ MediaBox, có tính `/Rotate`.
- **Nhận diện khổ A1–A5** theo kích thước điểm (A5 420×595, A4 595×842, A3 842×1191, A2 1191×1684, A1 1684×2384), sai số ±3%, xét cả dọc và ngang.
- **Chọn mẫu:** mẫu có khổ trùng → mẫu mặc định (nếu không trùng khổ nào, ví dụ Letter, thì dùng mẫu mặc định co giãn theo tỉ lệ).
- **Từ mẫu ra toạ độ:** vị trí neo (6 vị trí), lề ngang/dọc (mm), tỉ lệ (%) → khung theo tỉ lệ 0–1.
- **Ép khung vào trong trang** (lề an toàn 8pt) và cảnh báo trước khi ký (lệnh `check_positions`, tương đương `kiem-tra-vi-tri` bản cũ).

### 7.5 Đóng ảnh khi tạo văn bản (`stamp`)

File vừa tạo chưa có chữ ký nên được phép ghi lại toàn bộ bằng `lopdf`: thêm ảnh vào content stream của trang theo kế hoạch của `stamp_plan`.

---

## 8. Thiết kế chi tiết Tạo văn bản

### 8.1 Nguồn dữ liệu (`table`)

- **Excel** (`.xlsx`, `.xls`) bằng `calamine`: chọn sheet, chọn **dòng tiêu đề** (mặc định 1), bỏ dòng trống, ngày tháng định dạng `dd/MM/yyyy`.
  - Rủi ro: `calamine` không áp dụng định dạng số của ô như `ClosedXML.GetFormattedString()` (ví dụ `0001`, `1.234.567`, %). Cách xử lý: đọc thêm `styles.xml` để áp dụng các định dạng phổ biến; test với 3 file mẫu và một bộ file thật của khách hàng (spike S4).
- **CSV:** tự nhận diện mã hoá (UTF-8 có/không BOM, Windows-1258/1252) bằng `encoding_rs`; tự nhận diện dấu phân cách (`,` `;` tab).
- **Nhập trực tiếp:** form điền một bản ghi (prototype).

### 8.2 Mẫu DOCX (`docx`)

- Mở zip, xử lý `word/document.xml`, `header*.xml`, `footer*.xml`, và thêm `footnotes.xml`, text box (`w:txbxContent`) mà bản cũ chưa xử lý.
- Trường dạng `{ten_truong}`; ghép chuỗi các `<w:t>` trong cùng một đoạn để tìm, rồi thay vào run đầu tiên và xoá phần còn lại (port `ReplaceFields`).
- **Biến ảnh** (`{anh_the}`, `{con_dau}`): trong mẫu đặt một **ảnh giữ chỗ** có *Alt Text* là tên biến. Khi điền, thay dữ liệu ảnh trong `word/media/…` mà ảnh đó trỏ tới, giữ nguyên kích thước khung. Giá trị lấy từ cột chứa đường dẫn ảnh, hoặc một file cố định.
- Xuất DOCX: ghi zip đã điền. Xuất PDF: qua Word COM.

### 8.3 Mẫu HTML (`html`)

- Biến `{ten}` trong văn bản; ảnh qua `<img data-field="anh_the">`.
- Escape HTML cho mọi giá trị.
- In ra PDF bằng một WebView2 ẩn (`ICoreWebView2_7::PrintToPdf`, khổ và lề lấy từ CSS `@page`). Cần spike S5.

### 8.4 Word COM (`convert::word`)

- Một luồng **STA** riêng giữ một phiên `Word.Application` cho cả lô (`Visible = false`, `DisplayAlerts = 0`); các job gửi yêu cầu qua channel.
- `Documents.Open(path, ConfirmConversions=false, ReadOnly=true, AddToRecentFiles=false)` → `ExportAsFixedFormat(pdf, 17)` → `Close(0)`.
- Timeout cho mỗi file (ví dụ 120 giây). Quá thời gian thì kill tiến trình `WINWORD.EXE` do app mở (theo PID) và mở phiên mới.
- Không có Word: mẫu DOCX chỉ xuất được DOCX; mẫu HTML vẫn xuất được PDF. Báo lỗi 1045 rõ ràng.

### 8.5 Map trường và đặt tên file

- **Tự map:** bỏ dấu, `đ → d`, `_ → khoảng trắng`, so khớp theo từ (port `NormalizeWords`).
- Mỗi biến chọn **một cột** hoặc **giá trị cố định**, hoặc bỏ ghép.
- **Mẫu tên file** như `{so}_{hoten}`: bỏ dấu, thay ký tự cấm bằng `-`, khoảng trắng thành `_`, cắt còn 100 ký tự, trùng thì thêm `(1)`, `(2)`. Rỗng thì dùng `vanban_<số dòng>`.
- **Kiểm tra trước khi tạo** (port `Validate`): biến chưa ghép, ô trống theo từng dòng, tên trùng.
- Thư mục kết quả: `<thư mục lưu>\<tên mẫu>_<yyyyMMdd_HHmmss>\`.

---

## 9. Thiết kế chi tiết Kiểm tra chữ ký

- Tìm mọi field `/FT /Sig` qua AcroForm (không dò regex toàn file như bản cũ); với mỗi chữ ký lấy `/ByteRange`, `/Contents`, `/Name`, `/Reason`, `/Location`, `/M`, trang và `/Rect` của widget.
- Với mỗi chữ ký, kiểm tra:
  1. `/ByteRange` bắt đầu từ 0 và khoảng hở chỉ chứa `/Contents` (port `GapHoldsOnlyContents`).
  2. messageDigest khớp, chữ ký của người ký hợp lệ.
  3. Dấu thời gian: có token, chữ ký TSA hợp lệ, imprint khớp, TSA thuộc chuỗi G2.
  4. Chứng thư người ký: chuỗi về G1 hoặc G2 **tại thời điểm TSA đóng dấu**, tình trạng thu hồi.
  5. Phạm vi: chữ ký có phủ hết file không. Không phủ hết thì phân tích phần nối thêm: chỉ có chữ ký hoặc chú thích (hợp lệ, "có thay đổi sau khi ký") hay sửa nội dung trang.
  6. Giờ tự khai so với giờ TSA (giữ xử lý lệch +7 giờ của bản cũ).
- **Kết luận** cho mỗi chữ ký: Hợp lệ · Hợp lệ, có thay đổi sau khi ký · Hợp lệ, chưa kiểm tra được thu hồi · Không tin cậy (CA lạ) · Dấu thời gian không tin cậy · Không có dấu thời gian · Đã bị sửa (không hợp lệ) · Chứng thư đã bị thu hồi.
- **Kết luận cho văn bản:** "Toàn vẹn" khi chữ ký cuối phủ hết file và mọi chữ ký đều hợp lệ.
- **Xuất báo cáo PDF:** dựng HTML báo cáo (tên file, SHA-256 của file, từng chữ ký, chuỗi chứng thư, thời điểm kiểm tra) rồi in bằng WebView2 `PrintToPdf`.

---

## 10. Giao tiếp giao diện và lõi (IPC)

### 10.1 Commands

| Command | Tương đương bản cũ | Mô tả |
|---|---|---|
| `cert_list(only_signable)` | `GET sign/chung-thu-so` | Danh sách chứng thư kèm lý do không ký được |
| `token_check(thumbprint)` | `POST sign/kiem-tra-token` | Ký thử, bật hộp PIN |
| `token_status()` | (mới) | Token mặc định có đang cắm không (menu bar) |
| `sign_list_files(sources)` | `POST sign/danh-sach-file` | Mở rộng file và thư mục thành danh sách PDF/DOCX, kèm số trang, khổ, số chữ ký có sẵn |
| `sign_check_positions(req)` | `POST sign/kiem-tra-vi-tri` | Cảnh báo khối chữ ký tràn trang |
| `sign_batch(req, channel)` | `POST sign/ky-thu-muc` (SSE) | Chạy lô ký, trả `job_id` |
| `job_cancel(job_id)` | (huỷ request) | Dừng lô |
| `verify_file(path, online)` | `POST sign/kiem-tra-chu-ky` | Kiểm tra chữ ký chi tiết |
| `verify_export_report(path, out)` | (mới) | Xuất báo cáo PDF |
| `merge_read_table(path, sheet, header_row)` | `GET tao-van-ban/danh-sach-header` | Danh sách sheet, cột, dữ liệu xem nhanh |
| `merge_read_template(path)` | (mới) | Danh sách biến trong mẫu (cả biến ảnh) |
| `merge_auto_map(template, table)` | `GET tao-van-ban/map-tu-dong` | |
| `merge_validate(req)` | `POST tao-van-ban/kiem-tra` | |
| `merge_preview(req, row)` | `POST tao-van-ban/xem-truoc` | Trả đường dẫn PDF xem trước trong thư mục tạm |
| `merge_run(req, channel)` | `POST tao-van-ban/sinh-file` (SSE) | |
| `config_get()` / `config_save(cfg)` / `config_reset()` | localStorage | |
| `driver_status()` / `driver_install(kind)` | (mới) | Kiểm tra và cài driver token (cần UAC) |
| `app_check_update()` / `app_install_update()` | (mới) | |
| `shell_open(path)` | `openPath:` | Mở file hoặc thư mục bằng Explorer |
| `log_export(job_id, out)` | (mới) | Lưu nhật ký lô ra `.txt` |

Hộp thoại chọn file và thư mục dùng `tauri-plugin-dialog` ngay ở giao diện. Kéo thả dùng sự kiện `onDragDropEvent` của Tauri (có đường dẫn thật).

### 10.2 Sự kiện tiến độ

```ts
type JobEvent =
  | { type: 'manifest'; jobId: string; files: string[]; outputFolder: string; total: number }
  | { type: 'file';     jobId: string; index: number; file: string; status: 'ok' | 'error' | 'skipped';
      outputPath?: string; reason?: string; warnings?: string[]; current: number; total: number;
      log: LogLine[] }
  | { type: 'log';      jobId: string; line: LogLine }        // ví dụ TOKEN, INFO
  | { type: 'summary';  jobId: string; total: number; success: number; failed: number;
      elapsedMs: number; outputFolder: string; stoppedReason?: 'user' | 'token_removed' };

type LogLine = { time: string; tag: 'INFO' | 'OK' | 'LỖI' | 'TOKEN' | 'XONG'; text: string };
```

ETA tính ở giao diện: `(thời gian đã chạy / số file xong) × số file còn lại`, như prototype.

### 10.3 Xem trước PDF

- Giao diện dùng pdf.js đọc file qua asset protocol của Tauri (`convertFileSrc`).
- Scope của asset protocol để trống lúc khởi động. Lõi chỉ cho phép (`allow_file`) đúng những file người dùng đã chọn hoặc file tạm do app tạo. Như vậy giao diện không đọc được file tuỳ ý (khác với `sign/xem-file` của bản cũ).

---

## 11. Dữ liệu trên máy người dùng

| Đường dẫn | Nội dung |
|---|---|
| `%APPDATA%\SaoMai-KySo\config.json` | Cấu hình (mẫu chữ ký, token, thư mục, giao diện) |
| `%APPDATA%\SaoMai-KySo\pin.bin` | Chỉ khi bật lưu PIN (D2, bản 1.6): PIN mã hoá DPAPI |
| `%LOCALAPPDATA%\SaoMai-KySo\logs\` | Log theo ngày (`tracing-appender`), giữ 30 ngày. Không cần cấp quyền thư mục cài đặt như bản cũ |
| `%TEMP%\SaoMai-KySo\xem-truoc\` | Bản xem trước, dọn sau 60 phút |
| `%TEMP%\SaoMai-KySo\tao-van-ban\<guid>\` | Thư mục làm việc của job, xoá khi job xong |
| `<thư mục nguồn>\DaKySo\` | File đã ký (mặc định) |

Ví dụ `config.json`:

```json
{
  "version": 1,
  "signature": {
    "reason": "Ký số tài liệu",
    "location": "Việt Nam",
    "imagePath": "D:\\ChuKy\\condau_saomai.png",
    "font": { "family": "Times New Roman", "size": 10, "color": "#000000" },
    "textLines": ["signer", "time"],
    "profiles": [
      { "id": "a4", "name": "Khổ A4", "isDefault": true, "paper": "A4", "orientation": "portrait",
        "page": "last", "anchor": "br", "marginMm": { "x": 15, "y": 20 }, "scale": 100,
        "display": "image_and_text", "imagePath": null }
    ],
    "image": { "darkness": 140, "stroke": 100, "inkColor": null },
    "stamps": []
  },
  "tokens": {
    "known": [{ "thumbprint": "…", "name": "…", "isDefault": true }],
    "autoPin": { "enabled": false, "retries": 2, "onFail": "pause", "keepSession": "token" }
  },
  "output": {
    "mode": "subfolder", "subfolderName": "DaKySo", "fixedFolder": null,
    "suffix": "_signed", "copyFailedWithSuffix": false, "failedSuffix": "_loi",
    "onDuplicate": "ask", "replaceOriginal": false
  },
  "verify": { "onlineRevocation": true },
  "update": { "checkOnStartup": false },
  "ui": { "leftWidth": { "kyso": 340, "tvb": 360, "kt": 300, "xt": 480 }, "zoom": { "ks": 80, "tvb": 80, "kt": 80, "tc": 66 } }
}
```

Cấu hình có trường `version`. Khi đổi cấu trúc thì viết hàm nâng cấp, không làm mất cấu hình cũ.

---

## 12. Danh sách chức năng chi tiết theo màn hình

Mức ưu tiên: **M** = bắt buộc ở 1.5.0 · **S** = nên có ở 1.5.0, có thể lùi sang 1.5.x · **L** = để sau (1.6+).

### 12.1 Khung ứng dụng (SH)

| Mã | Chức năng | Ưu tiên | Tiêu chí hoàn thành |
|---|---|---|---|
| SH-01 | Cửa sổ 1280×820, tối thiểu 1024×700, tiêu đề "Sao Mai Ký số — \<màn\>", icon | M | Đổi màn thì tiêu đề đổi theo |
| SH-02 | Chỉ chạy một bản (`tauri-plugin-single-instance`); mở lần hai thì đưa cửa sổ cũ lên trước | M | |
| SH-03 | Menu bar: Chức năng (Trang chủ, Ký số, Tạo văn bản, Kiểm tra chữ ký, Vị trí chữ ký) · Cấu hình · Hỗ trợ (Hướng dẫn, Cài driver, Kiểm tra cập nhật, Thông tin) | M | Có dấu ✓ ở màn hiện tại, hiện phím tắt |
| SH-04 | Phím tắt `F1`, `Ctrl+K`, `Ctrl+N`, `Ctrl+E`, `Esc` | M | `Esc` đóng popup con trước, rồi popup, rồi menu |
| SH-05 | Chỉ báo token ở menu bar (cứ 2 giây cập nhật), tooltip: chứng thư, CA, thiết bị, trạng thái | M | Rút token thì đổi thành "Chưa cắm token" trong ≤ 3 giây |
| SH-06 | Component chung: tooltip `!`, nút Nâng cao, cột trái kéo đổi rộng, rail thu gọn, zoom 40–160%, ô đường dẫn có nút xoá, chip trạng thái | M | Giống prototype |
| SH-07 | Nhớ độ rộng cột, zoom, tab đang mở giữa các lần chạy | S | |
| SH-08 | Kéo thả file/thư mục vào cửa sổ ở màn Ký số, Tạo văn bản, Kiểm tra | M | Dùng đường dẫn thật |
| SH-09 | Thanh tiêu đề tự vẽ như prototype (D9) | L | |

### 12.2 Trang chủ (HM)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| HM-01 | Logo Sao Mai có hoạt ảnh (port đoạn `requestAnimationFrame` của prototype), tiêu đề "Ký số văn bản điện tử" | M |
| HM-02 | 4 thẻ chức năng, mỗi thẻ có tooltip mô tả và phím tắt | M |
| HM-03 | Chân trang: saomaiso.com, hotro@saomaiso.com, phiên bản (lấy từ `Cargo.toml`), tooltip tên người dùng Windows, tên máy | M |

### 12.3 Ký số văn bản (KS)

| Mã | Chức năng | Ưu tiên | Ghi chú |
|---|---|---|---|
| KS-01 | Chọn **1 file** (PDF/DOCX) hoặc **Thư mục**, nút xoá đường dẫn | M | Chọn nhiều file cùng lúc như bản cũ: M |
| KS-02 | Ô ảnh "Chữ ký / con dấu", nút Chọn ảnh (png/jpg ≤ 5MB) | M | |
| KS-03 | Nút **Cấu hình chữ ký** → màn Vị trí chữ ký | M | |
| KS-04 | Tuỳ chọn "Thay file gốc" (D3) | S | Bản 1.5.0 đặt trong Cấu hình nâng cao |
| KS-05 | Tab **Văn bản**: số thứ tự, icon PDF/DOCX, tên, chip Chờ ký / Đang ký / Đã ký / Lỗi, nhãn "Đã có N chữ ký" | M | |
| KS-06 | Tìm theo tên, không phân biệt dấu | M | |
| KS-07 | Lọc theo trạng thái, có đếm số lượng | M | |
| KS-08 | Thu gọn danh sách thành rail số thứ tự có màu trạng thái | S | |
| KS-09 | Xem trước trang ký thật bằng pdf.js, khung chữ ký theo mẫu hoặc theo vị trí đã kéo; nút trước/sau, nhập số thứ tự, zoom | M | Bản cũ dùng `pdf-pages` |
| KS-10 | Nhãn phủ trên trang: Đã ký số / Lỗi / Đang ký; hộp báo lỗi có lý do và cách xử lý | M | |
| KS-11 | Tooltip trên file: số trang, trang ký, vị trí, kiểu hiển thị, khổ giấy nhận diện được | M | |
| KS-12 | Tab **Nhật ký**: giờ, nhãn, nội dung; nút Lưu nhật ký (.txt), Xoá | M | |
| KS-13 | Thanh tiến độ dưới hàng tab, trạng thái "Đang ký x/y", thời gian đã chạy, ETA, đếm Đã ký / Lỗi | M | |
| KS-14 | Nút **Ký số → Dừng → Ký tiếp → Ký lại**, Mở thư mục kết quả | M | Ký tiếp chỉ ký file còn lại |
| KS-15 | Chỉ ký các file **đang hiện** theo bộ lọc/tìm kiếm (như prototype) | M | Ghi rõ trên nút: "Ký 12 văn bản đang hiện" |
| KS-16 | Hộp **File trùng tên**: liệt kê file đã tồn tại trong DaKySo, Ghi đè / Giữ cả hai / Huỷ | M | |
| KS-17 | Hộp **Xác nhận ký** (thay hộp Nhập PIN, D2): token, số file, kiểu hiển thị, nơi lưu | M | Hộp PIN của middleware hiện sau đó |
| KS-18 | Hộp **Hoàn thành**: Tổng / Thành công / Lỗi, Nhật ký, Mở thư mục | M | |
| KS-19 | Kiểm tra vị trí trước khi ký, báo file nào khung bị dịch vào trong | M | Bản cũ có |
| KS-20 | Rút token giữa chừng: dừng lô, báo file chưa ký | M | Bản cũ có |
| KS-21 | File DOCX: chuyển sang PDF qua Word rồi ký (D5) | S | |
| KS-22 | Nhận danh sách file từ Tạo văn bản ("Ký số các văn bản này") | M | Bản cũ có |
| KS-23 | Tự nhập lại PIN khi token hết phiên (D2) | L | |

### 12.4 Xem trước và vị trí chữ ký (VT)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| VT-01 | Chế độ **Tất cả file** hoặc **Từng file** | M |
| VT-02 | Lưới ảnh thu nhỏ các file bên trái, đánh dấu ✓ file có khung riêng | M |
| VT-03 | Kéo khung để di chuyển, kéo 4 góc để đổi cỡ (dùng lại logic pointer của `stamp-designer`) | M |
| VT-04 | Thanh trượt cỡ 50–200% so với cỡ của mẫu | M |
| VT-05 | Hiện X, Y, W × H (%); đếm số file đặt riêng | M |
| VT-06 | Nâng cao: lấy vị trí **theo mẫu khổ giấy**, chọn 6 vị trí neo, chọn kiểu hiển thị | M |
| VT-07 | Đặt lại · Thoát · **Áp dụng** (màn Ký số chuyển sang "tự cấu hình") | M |
| VT-08 | Chọn trang đặt chữ ký (trang cụ thể / đầu / cuối / tất cả) cho từng khung | M |
| VT-09 | Cảnh báo khi kiểu hiển thị là Ẩn ("khung chỉ để tham khảo") | M |

### 12.5 Tạo văn bản (TV)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| TV-01 | Chọn mẫu `.docx` | M |
| TV-02 | Chọn mẫu `.html` (D8) | S |
| TV-03 | Chọn dữ liệu Excel; chọn **Sheet**; chọn **dòng tiêu đề** | M |
| TV-04 | Dữ liệu CSV | S |
| TV-05 | Xem nhanh bảng Excel trong app, bấm dòng để chọn, tô dòng tiêu đề | M |
| TV-06 | **Nhập trực tiếp** một bản ghi (form), nút chèn Ảnh thẻ và Con dấu | S |
| TV-07 | Tab **Mẫu**: danh sách biến, nguồn "Lấy từ" (cột kèm số giá trị khác nhau / giá trị cố định / bỏ ghép), xanh là đã ghép, cam là chưa | M |
| TV-08 | "Tô màu trường" trên trang mẫu | S |
| TV-09 | Biến ảnh `anh_the`, `con_dau` | S |
| TV-10 | Tab **Kết quả**: danh sách tên file sẽ tạo, tìm theo tên hoặc nội dung, xem trước từng dòng (PDF thật) | M |
| TV-11 | Nâng cao: định dạng PDF/DOCX, mẫu tên file `{so}_{hoten}`, thư mục lưu | M |
| TV-12 | Kiểm tra trước khi tạo: biến chưa ghép, ô trống, tên trùng | M |
| TV-13 | Đóng dấu đỏ / chữ ký tươi lên PDF khi tạo (trang cụ thể, tất cả, chẵn, lẻ, tuỳ chọn) | M |
| TV-14 | Chạy lô có tiến độ, dừng được; hộp **Tạo văn bản thành công** có Ký số ngay, Mở thư mục | M |
| TV-15 | "Mở ví dụ mẫu" (dùng 3 mẫu trong `mau-tao-van-ban`, cài kèm app) | S |

### 12.6 Kiểm tra chữ ký số (KT)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| KT-01 | Chọn hoặc kéo thả file PDF đã ký; nút Kiểm tra lại | M |
| KT-02 | Tab **Chữ ký**: danh sách chữ ký (trạng thái, người ký, thời điểm) | M |
| KT-03 | Chi tiết: kết luận, Người ký, Đơn vị (`O` trong Subject), Thời điểm ký (giờ TSA) | M |
| KT-04 | Chi tiết chứng thư: lý do, nơi ký, trang và vị trí, CA, serial, hiệu lực, **cây chuỗi chứng thư**, TSA, tình trạng thu hồi | M |
| KT-05 | Tab **Văn bản**: hiện vị trí các chữ ký trên trang (pdf.js), khung cam là chữ ký đang chọn, nhãn Toàn vẹn | M |
| KT-06 | Chân màn: "Văn bản toàn vẹn · n/n chữ ký hợp lệ" | M |
| KT-07 | **Xuất báo cáo** PDF | S |
| KT-08 | Kiểm tra thu hồi trực tuyến (bật/tắt trong Cấu hình) | M |

### 12.7 Cấu hình (CH)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| CH-01 | Tab **Chữ ký theo khổ giấy**: danh sách mẫu A1–A5, mẫu mặc định, Thêm/Xoá mẫu | M |
| CH-02 | Mỗi mẫu: khổ, dọc/ngang, 6 vị trí neo, tỉ lệ ảnh 40–200%, kiểu hiển thị | M |
| CH-03 | Nâng cao của mẫu: tên mẫu, trang ký (cuối / đầu / tất cả – ký nháy), lề (mm), ảnh chữ ký riêng | M |
| CH-04 | Khung xem trước mẫu | M |
| CH-05 | Lý do ký, nơi ký, font, cỡ chữ, màu chữ, các dòng thông tin (bản cũ có, prototype thiếu) | M |
| CH-06 | Ảnh: độ đậm, độ dày nét, màu mực (bản cũ có) | S |
| CH-07 | Tab **USB Token & mã PIN**: bảng token (tên, CA, trạng thái), Làm mới, Thêm token (từ danh sách đang cắm), menu ⋯: Chi tiết, Đặt mặc định, Xoá, Kiểm tra token | M |
| CH-08 | Cài đặt tự động ký (lưu PIN, tự nhập lại, số lần thử, khi PIN sai, giữ phiên) | L |
| CH-09 | Tab **Thư mục & tên file**: thư mục lưu (DaKySo hoặc cố định), hậu tố thành công, hậu tố lỗi, xử lý khi trùng tên | M |
| CH-10 | Khôi phục mặc định · Huỷ · Lưu cấu hình ("✓ Đã lưu cấu hình") | M |

### 12.8 Hỗ trợ (HT)

| Mã | Chức năng | Ưu tiên |
|---|---|---|
| HT-01 | **Hướng dẫn sử dụng**: 7 mục như prototype, nội dung viết bằng Markdown trong `docs/huong-dan/`, nhúng lúc build | M |
| HT-02 | **Cài driver USB Token**: trạng thái bit4id, TokenManager, WebView2 (cùng cách nhận diện với bộ cài cũ), nút Cài (chạy bộ cài driver kèm app, xin quyền quản trị) | S |
| HT-03 | **Kiểm tra cập nhật**: phiên bản mới, "Có gì mới", Tải và cài đặt (D7) | S |
| HT-04 | **Thông tin phần mềm**: ứng dụng, phiên bản + ngày build, chức năng, đơn vị, website, email, giấy phép | M |

---

## 13. Chức năng bản cũ không có trong prototype

Những chức năng dưới đây **bản cũ đã có**; nếu bỏ đi người dùng hiện tại sẽ thấy mất chức năng. Đề xuất giữ lại, đặt vào mục Nâng cao để không làm rối giao diện.

| Chức năng bản cũ | Đề xuất |
|---|---|
| Lý do ký, nơi ký | Giữ ở Cấu hình (CH-05) |
| Font, cỡ, màu chữ khối chữ ký | Giữ ở Cấu hình (CH-05) |
| Dấu đỏ và chữ ký tươi là **hai ảnh riêng**, mỗi ảnh có nhiều khối, lặp theo trang (tất cả/chẵn/lẻ/`1,3,5-7`), vị trí riêng từng trang | Giữ ở Nâng cao của màn Vị trí chữ ký ("Thêm dấu / chữ ký tươi") |
| Độ đậm, độ dày nét, màu mực | Giữ (CH-06) |
| Nhồi chữ ký số vào ảnh | Chính là kiểu hiển thị "Chỉ ảnh" |
| Kiểm tra vị trí trước khi ký | Giữ (KS-19) |
| Đóng dấu khi tạo văn bản | Giữ (TV-13) |
| Cảnh báo theo từng dòng khi tạo văn bản | Giữ (TV-12) |
| Chọn nhiều file PDF lẻ cùng lúc | Giữ (KS-01) |

---

## 14. Lộ trình theo giai đoạn

Ước lượng thô cho **2 lập trình viên** (một người chuyên lõi Rust, một người làm giao diện và tích hợp), làm toàn thời gian. Tổng khoảng **17–20 tuần** đến bản phát hành 1.5.0. Sau giai đoạn 0 cần ước lượng lại.

### Giai đoạn 0: Spike và nền móng (2 tuần)

Mục tiêu: xác nhận các điểm rủi ro nhất trước khi đầu tư lớn.

| Spike | Câu hỏi cần trả lời | Đạt khi |
|---|---|---|
| S1 | Gọi được `NCryptSignHash` / `CryptSignHashW` từ Rust với token **bit4id** và **SafeNet**; hộp PIN hiện trên cửa sổ Tauri | Ký được 32 byte bằng cả hai loại token, hộp PIN không bị chìm |
| S2 | Dựng CMS + lấy dấu thời gian từ `tsa.ca.gov.vn`, ghi nối bản vào một PDF đơn giản | **Adobe Reader báo chữ ký hợp lệ**, pyHanko xác nhận cấu trúc PAdES B-T |
| S3 | Khối chữ ký tiếng Việt có nhúng subset font | Hiển thị đúng dấu trong Adobe, Foxit, Edge; copy chữ ra đúng |
| S4 | `calamine` đọc 3 file Excel mẫu với kết quả giống ClosedXML | Không lệch giá trị nào (ngày, số có số 0 đứng đầu, số tiền) |
| S5 | WebView2 `PrintToPdf` từ Tauri cho mẫu HTML | Ra PDF đúng khổ A4, đúng font |
| S6 | (cho bản 1.6) Middleware có nhận PIN qua `NCRYPT_PIN_PROPERTY` / `PP_SIGNATURE_PIN` không | Có báo cáo kết quả, quyết định D2 cho 1.6 |
| S7 | Word COM từ Rust trên luồng STA | Xuất được 3 mẫu DOCX ra PDF, đóng Word sạch |

Đồng thời dựng nền:
- Workspace Cargo, CI (fmt, clippy, test, build), khung Tauri 2 với Angular, `tracing`, cấu trúc lỗi.
- Port hằng số (Phụ lục A) và mã lỗi (Phụ lục B).
- Bộ `fixtures/` (xem 15.2).

**Mốc M0:** bản demo dòng lệnh `smks-cli sign file.pdf --thumbprint …` ký được một file bằng token thật, Adobe báo hợp lệ.

### Giai đoạn 1: Lõi ký số đầy đủ (4 tuần)

- `smks-crypto`: store, key (CNG + CAPI + dev), cms, tsa, chain (G1/G2, thu hồi).
- `smks-pdf`: reader, incremental (đủ các trường hợp ở 7.2), appearance (4 kiểu hiển thị, ảnh, xoay trang), geometry (nhận diện khổ, ép khung vào trang).
- `smks-core`: stamp_plan, chọn mẫu theo khổ, đặt tên file kết quả.
- Job ký song song, khoá tuần tự, theo dõi token, huỷ, kiểm tra sau khi ký.
- `smks-cli`: `sign`, `verify`, `inspect` (in cấu trúc PDF, số trang, khổ).
- Test đơn vị và test với bộ PDF mẫu.

**Mốc M1:** `smks-cli` ký được cả bộ PDF mẫu; mọi file qua kiểm tra của Adobe và pyHanko; khi cố ý làm hỏng một byte thì bộ xác minh phát hiện.

### Giai đoạn 2: Vỏ ứng dụng và màn Ký số (3 tuần)

- Shell: menu bar, phím tắt, chỉ báo token, component chung (SH-01…SH-08).
- Trang chủ (HM-01…03).
- Màn Ký số (KS-01…KS-20, KS-22).
- Commands `cert_list`, `token_check`, `sign_list_files`, `sign_check_positions`, `sign_batch`, `job_cancel`, `shell_open`, `log_export`.
- Đọc/ghi `config.json` (phần chữ ký và thư mục).

**Mốc M2:** người dùng thử ký được cả thư mục từ giao diện, có tiến độ, dừng, ký tiếp, nhật ký.

### Giai đoạn 3: Vị trí chữ ký và Cấu hình (2–3 tuần)

- Màn Vị trí chữ ký (VT-01…09).
- Hộp Cấu hình: 3 tab (CH-01…CH-07, CH-09, CH-10), dấu và chữ ký tươi nâng cao (mục 13).

**Mốc M3:** đủ chức năng ký như bản cũ + mẫu theo khổ giấy + vị trí từng file.

### Giai đoạn 4: Kiểm tra chữ ký (2 tuần)

- `smks-pdf::sigscan`, `smks-crypto::verify` đầy đủ (mục 9).
- Màn Kiểm tra (KT-01…08), xuất báo cáo.
- Dùng chung bộ xác minh này cho bước kiểm tra sau khi ký ở giai đoạn 1.

**Mốc M4:** kiểm tra đúng kết luận với bộ file mẫu gồm: file app tự ký, file ký bằng phần mềm khác của Ban Cơ yếu, file bị sửa, file ký bằng CA lạ, file có chú thích thêm sau khi ký.

### Giai đoạn 5: Tạo văn bản (3–4 tuần)

- `smks-merge`: table (Excel, CSV), docx (cả text box, footnote, biến ảnh), word COM, html + PrintToPdf.
- Màn Tạo văn bản (TV-01…15), job tạo văn bản, đóng dấu, chuyển sang Ký số.

**Mốc M5:** 3 mẫu trong `mau-tao-van-ban` ra đúng kết quả như bản cũ (so sánh từng trang); thêm mẫu HTML và xuất DOCX.

### Giai đoạn 6: Hỗ trợ, bộ cài, cập nhật (2 tuần)

- Hướng dẫn, Thông tin, Cài driver, Kiểm tra cập nhật (HT-01…04).
- Bộ cài NSIS (mục 16): gỡ bản .NET cũ, cài driver, WebView2.
- Ký Authenticode cho exe và bộ cài.

**Mốc M6:** bộ cài chạy sạch trên máy ảo Windows 10 và Windows 11 chưa cài gì; nâng cấp được từ bản 1.4.0 mà không mất file của người dùng.

### Giai đoạn 7: Kiểm thử tổng, pilot, phát hành (2–3 tuần)

- Kiểm thử theo ma trận (15.4), sửa lỗi.
- Pilot với 3–5 người dùng thật (khác đơn vị, khác loại token), thu phản hồi.
- Viết ghi chú phát hành, cập nhật README, tài liệu cam kết riêng tư.

**Mốc M7:** phát hành 1.5.0.

### Sau 1.5.0

| Bản | Nội dung |
|---|---|
| 1.5.x | Các mục **S** chưa kịp làm |
| 1.6 | Tự nhập lại PIN (nếu S6 đạt), LTV (PAdES B-LT: nhúng OCSP/CRL vào DSS), thanh tiêu đề tự vẽ |
| 1.7 | Ký DOCX gốc (OOXML/XAdES), cần spike riêng |

---

## 15. Kiểm thử

### 15.1 Các tầng test

| Tầng | Nội dung | Công cụ |
|---|---|---|
| Đơn vị | Parser xref, ghi nối bản, ByteRange, ASN.1 CMS/TSP, nhận diện khổ, stamp_plan, đặt tên file, NormalizeWords, thay trường DOCX | `cargo test`, `insta` (snapshot) |
| Tích hợp lõi | Ký bằng `SoftwareKey` + CA test + TSA giả lập; xác minh lại; đọc bằng `lopdf` sau khi ký | `cargo test --features dev-keys` |
| Kiểm chứng độc lập | Mọi file đầu ra trong CI được kiểm tra bằng **pyHanko** (cấu hình trust root là CA test) | Python trong CI |
| Giao diện | Logic component, service | Test của Angular |
| Đầu-cuối | Mở app, chọn thư mục, ký, kiểm tra kết quả | WebDriver của Tauri (`tauri-driver`), chạy tay trên máy có token |
| Thủ công | Ký bằng token thật, mở bằng Adobe Acrobat Reader | Checklist trước mỗi bản phát hành |

TSA giả lập: một server nhỏ trong test dùng khoá test để trả TimeStampResp, không phụ thuộc mạng. Chỉ một bài test đánh dấu `#[ignore]` gọi `tsa.ca.gov.vn` thật, chạy tay.

### 15.2 Bộ file mẫu (`fixtures/`)

PDF:
- Xuất từ Word, LibreOffice, Chrome "In ra PDF", máy scan (ảnh).
- Xref dạng table; xref dạng stream; có object stream; file đã cập nhật nối bản nhiều lần; file hybrid.
- `/Annots` và `/AcroForm` là tham chiếu gián tiếp; object có generation khác 0.
- Trang xoay 90/180/270; CropBox khác MediaBox; khổ A5, A4, A3 ngang, A2, A1, Letter.
- Đã có 1 và 2 chữ ký (ký bằng app này và bằng phần mềm khác); có `/DocMDP P=1`, `P=2`; có mã hoá; file hỏng nhẹ.
- File lớn (≥ 50MB, ≥ 500 trang) để đo hiệu năng.

DOCX/XLSX: 3 cặp mẫu trong `mau-tao-van-ban`; mẫu có trường bị tách run, trường trong header/footer/text box/footnote, biến ảnh; Excel có ngày, số có số 0 đứng đầu, số tiền, ô gộp, nhiều sheet, dòng tiêu đề không ở dòng 1; CSV UTF-8 BOM và Windows-1258.

Không đưa dữ liệu thật của khách hàng vào repo.

### 15.3 Hiệu năng mục tiêu

| Chỉ số | Mục tiêu |
|---|---|
| Khởi động đến khi thấy Trang chủ | < 1,5 giây |
| Liệt kê thư mục 500 PDF | < 3 giây |
| Ký 100 PDF 1–5 trang, mạng ổn định | ≤ thời gian của bản 1.4.0 (đo cùng máy, cùng token) |
| Bộ nhớ khi ký lô 500 file | < 500MB |
| Kích thước bộ cài (chưa tính driver) | < 25MB |

### 15.4 Ma trận kiểm thử thủ công

| Trục | Giá trị |
|---|---|
| Hệ điều hành | Windows 10 22H2, Windows 11 23H2/24H2 |
| Token | bit4id (đời mới), SafeNet eToken (trước 2022) |
| Chứng thư | Nhánh G1, nhánh G2, đã hết hạn, CA lạ |
| Word | Office 2016, 2019, 365; máy không có Word |
| Mạng | Bình thường, mất mạng (TSA lỗi), proxy công ty |
| Màn hình | 100%, 125%, 150% DPI |
| Người dùng | Quản trị và không quản trị |

---

## 16. Đóng gói, cài đặt, cập nhật

- Build: `cargo tauri build` ra `SaoMai-KySo.exe`.
- **Bộ cài:** tận dụng script `SaoMai-KySo.nsi` của bản cũ (đã chạy ổn), sửa lại:
  - Nguồn file: exe của Tauri + thư mục `drivers\` (không còn `wwwroot\`, không còn thư mục `Log` cần `icacls`).
  - Giữ thứ tự: gỡ bản KTL cũ (khoá `…\Uninstall\KTL`) → **gỡ bản .NET 1.x** (khoá `…\Uninstall\SaoMai-KySo`, chạy `Uninstall.exe /S`) → đóng `SaoMai-KySo.exe` đang chạy → chép app → lối tắt → mục gỡ cài đặt → WebView2, bit4id, TokenManager.
  - Giữ cách nhận diện driver bằng trạng thái thật: `HKLM\SOFTWARE\bit4id`; provider bắt đầu bằng `eToken` trong `HKLM\SOFTWARE\Microsoft\Cryptography\Defaults\Provider` hoặc `System32\eTOKCSP.dll`; WebView2 qua `EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`.
  - Lưu script dạng UTF-8 có BOM (lưu ý của bản cũ).
- Không cần chuyển cấu hình cũ: cấu hình bản cũ nằm trong localStorage của WebView2, không đọc ra một cách tin cậy được. Lần đầu chạy, app tạo cấu hình mặc định và hiện thông báo "Đã nâng cấp lên 1.5.0, vui lòng kiểm tra lại Cấu hình".
- **Ký Authenticode** cho `SaoMai-KySo.exe` và bộ cài (chứng thư ký code OV/EV) để không bị SmartScreen chặn và để người dùng tin app ký số.
- **Cập nhật** (D7): `tauri-plugin-updater` với khoá minisign; máy chủ trả JSON `{version, notes, pub_date, platforms.windows-x86_64.{url, signature}}`. Gói cập nhật là bộ cài NSIS chạy ở chế độ cập nhật.
- Phiên bản lấy **một nguồn duy nhất** từ `app/src-tauri/Cargo.toml` (truyền vào NSIS bằng `/DAPP_VERSION=…` lúc build), tránh lỗi phải sửa ba chỗ như bản cũ.

---

## 17. Bảo mật

| Chủ đề | Biện pháp |
|---|---|
| Bề mặt tấn công | Không mở cổng mạng. Capabilities của Tauri chỉ cấp đúng commands cần dùng; tắt các plugin không dùng |
| Đọc file | Asset protocol chỉ được đọc những file người dùng đã chọn (10.3) |
| Nội dung web | CSP chặt (`default-src 'self'`), không tải script từ ngoài, mọi dữ liệu hiển thị đều escape |
| Mã PIN | 1.5.0 không xử lý PIN. 1.6 (nếu có): DPAPI CurrentUser, tắt sẵn, cảnh báo, xoá được |
| Khoá dev | `SoftwareKey` và CA test chỉ có khi bật feature `dev-keys`; CI kiểm tra bản release không chứa feature này |
| Ghim CA | Nhúng chứng thư, kiểm tra SHA-256 lúc khởi động |
| Log | Không ghi PIN, không ghi nội dung văn bản; chỉ ghi đường dẫn, tên chứng thư, mã lỗi |
| Cập nhật | Gói cập nhật có chữ ký minisign; tải qua HTTPS |
| Chuỗi cung ứng | `cargo deny` (giấy phép, lỗ hổng), `cargo audit` trong CI; `npm audit`; khoá phiên bản bằng `Cargo.lock`, `package-lock.json` |
| Driver bên thứ ba | Chỉ dùng file lấy trực tiếp từ nhà phát hành, kiểm tra chữ ký Authenticode trước khi đưa vào bộ cài |

---

## 18. Rủi ro và cách giảm

| # | Rủi ro | Khả năng | Ảnh hưởng | Cách giảm |
|---|---|---|---|---|
| R1 | Middleware token hành xử khác nhau với CNG/CAPI gọi từ Rust (hộp PIN, HWND, khoá không nhận) | TB | Cao | Spike S1 với cả hai loại token ngay tuần đầu; giữ cả hai đường CNG và CAPI |
| R2 | Chữ ký không được Adobe công nhận do sai chi tiết ASN.1 hoặc ByteRange | TB | Cao | Spike S2; so sánh byte với file do bản .NET ký; kiểm tra bằng pyHanko trong CI |
| R3 | Crate RustCrypto (`cms`, `x509-tsp`) thiếu chức năng | TB | TB | Tự dựng bằng `der` (đã dự trù trong 6.3) |
| R4 | `lopdf` không đọc được PDF lạ | TB | TB | Parser dự phòng port từ bản cũ; bộ fixtures đa dạng |
| R5 | `calamine` hiển thị giá trị khác Excel | Cao | TB | Spike S4; áp dụng định dạng số; cảnh báo trong màn Kết quả |
| R6 | Word COM treo hoặc để lại tiến trình WINWORD | TB | TB | Luồng STA riêng, timeout, kill theo PID |
| R7 | WebView2 `PrintToPdf` không ổn với một số mẫu HTML | Thấp | Thấp | Spike S5; mẫu HTML là tính năng S |
| R8 | Nhóm chưa quen Rust và FFI Windows | TB | Cao | Người làm lõi phải có kinh nghiệm Rust; viết sẵn lớp bọc an toàn cho FFI; review chặt các khối `unsafe` |
| R9 | Phạm vi phình to vì prototype nhiều chức năng | Cao | TB | Chia rõ M/S/L (mục 12), chỉ M chặn phát hành |
| R10 | TSA `tsa.ca.gov.vn` chậm hoặc không ổn định khi ký lô lớn | TB | TB | Giới hạn 8 đồng thời, thử lại 1 lần, báo lỗi rõ cho từng file, ký tiếp được |
| R11 | Pháp lý: lưu PIN, tự ký khi không có người | Thấp | Cao | Lùi sang 1.6, chỉ làm sau khi có ý kiến pháp chế |
| R12 | Kiểm tra thu hồi làm chậm hoặc lỗi khi offline | TB | Thấp | Timeout ngắn, kết luận "chưa kiểm tra được thu hồi", không chặn ký |

---

## 19. Quy ước code và quy trình

### 19.1 Rust

- Rust stable, ghim bằng `rust-toolchain.toml`; `edition = "2021"` hoặc mới hơn.
- `cargo fmt`, `cargo clippy -- -D warnings` bắt buộc qua.
- Lỗi: `thiserror` trong các crate lõi; mỗi lỗi nghiệp vụ có `code: u32` (Phụ lục B) và câu thông báo tiếng Việt. `anyhow` chỉ dùng trong `smks-cli`.
- `unsafe` chỉ nằm trong module `ffi` của từng crate, có chú thích `// SAFETY:`, có lớp bọc RAII (`Drop`) cho mọi handle Windows (`HCERTSTORE`, `NCRYPT_KEY_HANDLE`, `HCRYPTPROV`, COM).
- Thời gian: múi giờ Việt Nam cố định UTC+07:00 (Việt Nam không đổi giờ mùa hè).
- Hằng số trong `smks-core::consts`, không viết cứng trong code xử lý.
- Ghi file kết quả ra `*.tmp` rồi đổi tên.

### 19.2 Giao diện

- Component standalone, quản lý state bằng `signal`/`computed`.
- Biến CSS (màu, khoảng cách, font) lấy từ phần `<style>` của prototype, đặt trong `styles/tokens.css`.
- Mọi lời gọi lõi đi qua một service bọc `invoke`, có kiểu TypeScript sinh từ Rust (dùng `specta` hoặc `ts-rs`) để hai bên luôn khớp.

### 19.3 Git và review

- Nhánh `feat/…`, `fix/…`, `refactor/…`, `docs/…`, `build/…`; Conventional Commits, mô tả tiếng Việt (giữ quy ước bản cũ).
- PR cần ít nhất 1 người review; PR đụng tới `smks-crypto` hoặc `smks-pdf::incremental` cần chạy test token thật và mở file bằng Adobe trước khi gộp.
- Mỗi quyết định kiến trúc ghi thành ADR ngắn trong `docs/adr/` (bắt đầu từ D1–D9 ở mục 2).

### 19.4 CI (Windows runner)

```
fmt → clippy → cargo test (feature dev-keys) → pyHanko kiểm tra file đầu ra
→ npm ci + ng build + test UI → cargo tauri build → cargo deny / cargo audit
→ (nhánh main) đóng gói NSIS, đính kèm artifact
```

---

## 20. Chuẩn bị trước khi bắt đầu

- [ ] Chốt các quyết định D1–D9 (mục 2).
- [ ] Ít nhất **2 USB token thật**: một bit4id (đời mới), một SafeNet (trước 2022), có chứng thư còn hạn của Ban Cơ yếu, dành riêng cho phát triển.
- [ ] Bộ cài driver: `bit4id_xpki_1.4.10.764-ng-user-vgca-pkimgr-bwc.exe`, `VGCAv1_Installer_1.0.21.0402.exe`, `MicrosoftEdgeWebView2Setup.exe` (đã kiểm tra chữ ký Authenticode).
- [ ] Máy phát triển có Microsoft Word; máy ảo Windows 10 và 11 sạch để thử bộ cài.
- [ ] Adobe Acrobat Reader để kiểm tra chữ ký; Python + pyHanko cho CI.
- [ ] Bộ PDF/DOCX/XLSX mẫu (15.2), có cả file do các phần mềm ký số khác tạo ra.
- [ ] Chứng thư ký code (Authenticode) cho exe và bộ cài.
- [ ] Nơi đặt máy chủ cập nhật (D7) và cặp khoá minisign.
- [ ] Repo git mới, CI runner Windows.
- [ ] Thiết kế chốt cuối cho các màn (prototype hiện còn vài chỗ chưa nhất quán: hộp Kiểm tra cập nhật ghi "đang dùng 1.4.2" nhưng Thông tin ghi 1.5.0; tooltip Trang chủ nói thư mục con DaKySo nhưng cấu hình là đường dẫn cố định; hộp Ghép trường không mở được từ đâu).

---

## Phụ lục A: Hằng số mang từ bản cũ

Nguồn: `ktl.be.shared/Constants/**`.

### A.1 Ký số và dấu thời gian

| Hằng số | Giá trị |
|---|---|
| TSA URL | `http://tsa.ca.gov.vn/` |
| Content-Type yêu cầu / phản hồi | `application/timestamp-query` / `application/timestamp-reply` |
| Timeout TSA | 180 giây |
| Số yêu cầu TSA đồng thời | 8 |
| SubFilter | `ETSI.CAdES.detached` |
| Thuật toán băm | SHA-256 (`2.16.840.1.101.3.4.2.1`) |
| Chỗ trống `/Contents` | 32.768 byte (65.536 ký tự hex) |
| Độ rộng mỗi số trong `/ByteRange` | 10 ký tự |
| Lý do / nơi ký mặc định | `Ký số tài liệu` / `Việt Nam` |
| Số luồng ký | `clamp(số CPU, 2, 8)` |
| Chu kỳ kiểm tra token | 2 giây |
| Dữ liệu ký thử | 32 byte ngẫu nhiên |
| Tiền tố tên field chữ ký | `SaoMai_Signature_<số object>` |

### A.2 OID

| Tên | OID |
|---|---|
| signatureTimeStampToken | `1.2.840.113549.1.9.16.2.14` |
| signingCertificateV2 | `1.2.840.113549.1.9.16.2.47` |
| signingTime | `1.2.840.113549.1.9.5` |
| EKU timeStamping | `1.3.6.1.5.5.7.3.8` |

### A.3 CA được ghim (SHA-256)

| File | Vai trò | SHA-256 |
|---|---|---|
| `rootca.crt` | Root G1 | `F7BC89054CEC7F12730D7B8CD3EB7955B163835087E5EB873EDFB584763B3F90` |
| `cp.crt` | Sub CA G1 | `75E969C07BA35C29C72BBC1BFB880F20F3891A619B00B77EB02CA6160736A1FE` |
| `rootcag2.crt` | Root G2 (cũng là gốc của TSA) | `F04BA4B459A7C9A1B971AD9B1CB833E695A80C79AEA63B174B66A70EDFE2FDB8` |
| `dcscag2.crt` | CA trung gian G2 | `E4B402E3CB4CACC0A20D21FA49BCE76BEB4EB47BA7FFB7A8F71C554DBE561A81` |
| `cpg2.crt` | CA trung gian G2 | `D8CAB7A4AEF92E6BB2509D7C61CEE7B44F3731B2D21074963530277B366FAE4B` |

### A.4 Xác minh

| Hằng số | Giá trị |
|---|---|
| Lệch giờ "giờ địa phương ghi thành UTC" | 7 giờ, cửa sổ ±3600 giây |
| Dung sai đồng hồ TSA | 60 giây |

### A.5 Khối chữ ký và ảnh

| Hằng số | Giá trị |
|---|---|
| Cỡ khối mặc định | 170 × 30 pt (trên trang tham chiếu 595 × 842 pt) |
| Font | Times New Roman (mặc định), Arial, Verdana, Georgia, Courier New |
| Cỡ chữ | 10pt mặc định, 6–24pt |
| Màu chữ | `#000000` |
| Lề an toàn trong trang | 8 pt |
| Ảnh cho phép | `.png`, `.jpg`, `.jpeg`, ≤ 5MB |
| Độ đậm | 40–250, mặc định 140 |
| Độ dày nét | 0–200, mặc định 100, bán kính = `rộng × 0,35/158 × nét/100`, 8 hướng |
| Định dạng màu | `^#[0-9A-Fa-f]{6}$` |

### A.6 Nhận diện token

- Dấu hiệu phần cứng: `smart card, bit4id, vgca, token, etoken, safenet, gemalto, thales, epass, feitian`
- Provider phần mềm (loại trừ): `microsoft software key storage provider`, `microsoft enhanced cryptographic provider`, `microsoft base cryptographic provider`, `microsoft strong cryptographic provider`, `microsoft enhanced rsa and aes cryptographic provider`
- Provider thực tế: bit4id: `Bit4id Universal Middleware Provider`, `Bit4id Key Storage Provider`; SafeNet: CSP `eToken Base Cryptographic Provider`, KSP `SafeNet Smart Card Key Storage Provider`

### A.7 Tạo văn bản

| Hằng số | Giá trị |
|---|---|
| Mẫu trường | `\{([^{}]+)\}` |
| Định dạng ngày trong ô | `dd/MM/yyyy` |
| Định dạng thời gian thư mục kết quả | `yyyyMMdd_HHmmss` |
| Độ dài tối đa phần tên file | 100 ký tự |
| Giữ bản xem trước | 60 phút |
| Mẫu trang tuỳ chọn | `^(\d+)(?:\s*-\s*(\d+))?$`, phân tách bằng `,` |
| Word | ProgID `Word.Application`, xuất PDF = 17, không lưu = 0, tắt cảnh báo = 0 |

---

## Phụ lục B: Mã lỗi

Giữ nguyên mã của bản cũ để log và tài liệu hỗ trợ không bị lệch.

| Mã | Tên | Thông báo mặc định |
|---|---|---|
| 1 | System | Lỗi hệ thống |
| 400 | BadRequest | Yêu cầu không hợp lệ |
| 404 | NotFound | Không tìm thấy |
| 500 | InternalError | Lỗi không xác định |
| 1025 | TimestampInvalid | Dấu thời gian/chữ ký số của file không hợp lệ |
| 1026 | SignatureInvalid | Nội dung file đã bị sửa sau khi ký số |
| 1027 | CertificateNotFound | Không đọc được chứng thư số nào trên máy |
| 1028 | CertificateNotSignable | Không có chứng thư số nào dùng để ký được |
| 1029 | SignSourceNotFound | Không tìm thấy file hoặc thư mục cần ký |
| 1030 | SignNoPdfFound | Không có file PDF nào để ký |
| 1031 | SignTokenInvalid | Chứng thư số không dùng để ký được |
| 1032 | SignFailed | Ký số file thất bại |
| 1033 | SignPdfEncrypted | File PDF được mã hóa nên không ký số được |
| 1034 | SignPdfLocked | File PDF bị khóa không cho ký thêm (/DocMDP) |
| 1035 | SignPdfStructureUnsupported | Cấu trúc file PDF không hỗ trợ ký nối bản |
| 1037 | SignPositionInvalid | Vị trí đặt chữ ký không hợp lệ |
| 1040 | StampImageInvalid | Ảnh dấu đỏ hoặc chữ ký tươi không hợp lệ |
| 1042 | MergeTemplateInvalid | File mẫu không hợp lệ |
| 1043 | MergeDataInvalid | File dữ liệu không hợp lệ |
| 1044 | MergeMappingInvalid | Cấu hình ghép trường không hợp lệ |
| 1045 | MergeConverterUnavailable | Máy chưa cài Microsoft Word để xuất PDF |
| 1046 | MergeFailed | Tạo văn bản thất bại |
| 1047 | MergeStampInvalid | Cấu hình ảnh dấu đỏ hoặc chữ ký tươi không hợp lệ |
| 1048 | FileUploadInvalid | (không dùng nữa, giữ chỗ) |
| 1049 | TokenRemoved | Đã rút USB token khi đang ký nên dừng tiến trình |
| 1050 | OutputExists | File kết quả đã tồn tại |
| 1051 | OutputFolderInvalid | Không ghi được vào thư mục lưu kết quả |
| 1052 | RevocationUnknown | Chưa kiểm tra được tình trạng thu hồi chứng thư |
| 1053 | CertificateRevoked | Chứng thư số đã bị thu hồi |
| 1054 | DocxConvertFailed | Không chuyển được DOCX sang PDF để ký |
| 1055 | HtmlRenderFailed | Không in được mẫu HTML ra PDF |
| 1056 | TableSheetNotFound | Không tìm thấy sheet trong file Excel |
| 1057 | ConfigInvalid | File cấu hình hỏng, đã khôi phục mặc định |
| 1058 | UpdateFailed | Không kiểm tra hoặc tải được bản cập nhật |
| 1059 | DriverInstallFailed | Cài driver token không thành công |

---

## Phụ lục C: Danh sách crate dự kiến

Dùng phiên bản ổn định mới nhất tại thời điểm bắt đầu, khoá bằng `Cargo.lock`. Mục có dấu (\*) cần spike xác nhận.

| Nhóm | Crate | Dùng cho |
|---|---|---|
| Vỏ app | `tauri` 2, `tauri-build` | Cửa sổ, IPC, bundler |
| | `tauri-plugin-dialog`, `tauri-plugin-opener`, `tauri-plugin-single-instance`, `tauri-plugin-updater` | Hộp thoại, mở thư mục, một bản chạy, cập nhật |
| Windows | `windows` (windows-rs) | Cert store, NCrypt, CAPI, CertGetCertificateChain, DPAPI, COM (Word), ShellExecute (cài driver), Registry, Recycle Bin, WebView2 PrintToPdf |
| | `webview2-com` (\*) | Gọi `ICoreWebView2_7::PrintToPdf` |
| ASN.1 / X.509 | `der`, `x509-cert`, `spki`, `const-oid` | Đọc chứng thư, dựng cấu trúc |
| | `cms` (\*) | SignedData |
| | `x509-tsp` (\*) | TimeStampReq/Resp; không đủ thì tự dựng bằng `der` |
| Mật mã | `sha2`, `rsa`, `p256`, `p384`, `ecdsa`, `rand` | Băm, xác minh chữ ký của file người khác ký, nonce |
| PDF | `lopdf` | Đọc cấu trúc, đóng ảnh khi tạo văn bản |
| | `pdf-writer` | Ghi object cho phần nối bản và Form XObject |
| | `ttf-parser`, `subsetter` | Nhúng subset font tiếng Việt |
| | `image`, `flate2` | Ảnh dấu/chữ ký, nén stream |
| Office | `zip`, `quick-xml` | DOCX |
| | `calamine` (\*) | Excel |
| | `csv`, `encoding_rs` | CSV |
| Mạng | `reqwest` (rustls) | TSA |
| Bất đồng bộ | `tokio`, `tokio-util` (CancellationToken), `futures` | Job song song |
| Tiện ích | `serde`, `serde_json`, `thiserror`, `anyhow` (CLI), `regex`, `unicode-normalization`, `chrono` hoặc `time`, `uuid`, `walkdir` | |
| Log | `tracing`, `tracing-subscriber`, `tracing-appender` | |
| Kiểu cho TS | `specta` + `tauri-specta`, hoặc `ts-rs` | Sinh kiểu TypeScript từ Rust |
| Test | `insta`, `tempfile`, `assert_fs` | |
| Giao diện | Angular 21, `pdfjs-dist`, `@tauri-apps/api`, các plugin JS tương ứng | |
| CI | `cargo-deny`, `cargo-audit`, pyHanko | |
