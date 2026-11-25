# S3: Khối chữ ký tiếng Việt có nhúng subset font

- **Kết luận (18/11/2026):**
  - **S3a: PASS với 4 trên 5 font** (Times New Roman, Arial, Verdana, Courier New), kiểm bằng pdf.js và pyHanko.
  - **Georgia: FAIL.** Font Georgia của Windows không có glyph cho phần lớn chữ tiếng Việt có dấu. Đây là
    giới hạn của font, không phải của cách nhúng. Cần quyết định (Q-09).
  - **S3b: CHƯA CHẠY.** Chưa kiểm bằng Adobe Acrobat Reader, Foxit và trình xem PDF gốc của Edge.
- **Chặn bởi:** S3b cần Adobe Reader và Foxit (B-03).

## Mục tiêu

Khối chữ `Ký bởi: Nguyễn …` hiển thị đúng dấu trong Adobe, Foxit, Edge; copy chữ ra đúng.

## Phương án kỹ thuật đã làm

Mã: `spikes/s3-font/` (Rust, chạy bằng toolchain GNU; các crate dùng không kéo `windows-sys`).

1. Đọc font từ `C:\Windows\Fonts` bằng `ttf-parser` 0.25; chuẩn hoá chuỗi về NFC; tra glyph cho từng ký tự.
2. `subsetter` 0.2.6 tạo subset và **đánh số lại glyph liền nhau** (`GlyphRemapper`); mã ký tự trong content
   stream là số glyph mới, 2 byte big-endian.
3. `pdf-writer` 0.15 ghi: font `Type0` (`/Encoding /Identity-H`) → `CIDFontType2` (`/CIDToGIDMap /Identity`,
   `/W` theo thứ tự glyph mới) → `FontDescriptor` → `FontFile2` (nén Flate, có `/Length1`) và CMap
   `/ToUnicode`.
4. Mỗi font một Form XObject, `/BBox` bằng kích thước đo được: bề rộng dòng dài nhất, cao = số dòng ×
   (`ascender − descender + lineGap`) theo bảng `hhea`.
5. Trang thử: 5 khối, mỗi khối 5 dòng (tên có dấu, ngày giờ, đủ bộ nguyên âm có dấu chữ hoa và chữ thường,
   một dòng nhập ở dạng NFD), kẻ khung đỏ theo kích thước đo.

## Kết quả thực tế

### Báo cáo của chương trình

| Font | File gốc | Subset | Subset đã nén | Số glyph | Rộng × cao khối (pt, cỡ 10) | Ký tự thiếu glyph |
|---|---|---|---|---|---|---|
| Times New Roman | 1.195.688 | 62.436 | 36.267 | 145 | 372,61 × 57,50 | không |
| Arial | 1.036.584 | 52.200 | 28.021 | 142 | 379,45 × 57,50 | không |
| Verdana | 243.304 | 31.124 | 17.307 | 142 | 382,84 × 60,77 | không |
| Georgia | 219.712 | 28.304 | 18.277 | 64 | 488,30 × 56,81 | **78 ký tự**, gồm `ở ễ ị ế` và toàn bộ nhóm `ẮẰẲẴẶ ẤẦẨẪẬ ẾỀỂỄỆ ỐỒỔỖỘ ƠỚỜỞỠỢ ƯỨỪỬỮỰ` |
| Courier New | 806.504 | 46.744 | 25.645 | 142 | 360,06 × 56,64 | không |

### Kiểm chứng độc lập

| Phép kiểm | Công cụ | Kết quả |
|---|---|---|
| Trích văn bản (tương đương "copy chữ") rồi so từng dòng với chuỗi mong đợi | pdf.js 6.4.299 chạy trên Node (`verify/verify.mjs`) | **21/25 dòng khớp**. 20/20 dòng của 4 font đạt, kể cả dòng nhập dạng NFD. 4 dòng lệch đều của Georgia: mọi ký tự thiếu glyph trích ra thành `ở` |
| Hiển thị | pdf.js 6.4.299 vẽ lên canvas trong Edge chạy ẩn, chụp qua giao thức DevTools (`verify/screenshot.mjs`) | 4 font hiện đúng mọi dấu, chữ nằm gọn trong khung đo. Georgia hiện ô vuông ở các ký tự thiếu glyph |
| Cấu trúc PDF | pyHanko 0.38.1 (`PdfFileReader`) | Đọc được 5 Form XObject; mỗi font đúng `Type0` / `CIDFontType2` / `Identity-H`; `FontFile2` giải nén ra đúng `Length1` và là TrueType hợp lệ |

Bằng chứng: `docs/spikes/evidence/S3/s3-font.pdf`, `expected.txt`, `pdfjs-in-edge.png`.

### Việc thử không thành

- Chụp trang PDF bằng trình xem gốc của Edge ở chế độ chạy ẩn (`--headless --screenshot`): ảnh ra trang
  trắng. Edge chạy ẩn không hiện trình xem PDF, nên **chưa kiểm được bộ dựng PDF gốc của Edge**.

## Trả lời các câu hỏi của spike

- `subsetter` **đánh số lại** glyph. `/W`, `/ToUnicode` và content stream đều phải dùng số mới.
- Chiều cao dòng tính theo `hhea` cho Times New Roman là 11,5pt ở cỡ 10pt. **Chưa đối chiếu** với
  `XFont.GetHeight()` của PDFsharp ở bản cũ; cần làm khi port `appearance` để kích thước khối không lệch.
- Kích thước subset cho khoảng 145 glyph: 17–36KB sau nén. Khối chữ ký thật dùng ít glyph hơn nên nhỏ hơn.
- Chuỗi nhập ở dạng NFD ra đúng sau khi chuẩn hoá NFC.

## Phát hiện

**Georgia không dùng được cho tiếng Việt.** Georgia là một trong 5 font bản cũ cho chọn
(`SigningConstants.AppearanceFontFamilies`). Bản cũ vẽ bằng PDFsharp với cùng file font nên nhiều khả năng
cũng ra ô vuông, nhưng điều này **chưa được kiểm trên bản 1.4.0**. Xem K-15, Q-09.

## Việc còn lại (S3b)

- Mở `docs/spikes/evidence/S3/s3-font.pdf` bằng Adobe Acrobat Reader, Foxit và Edge: nhìn dấu, bôi đen và
  copy thử một dòng.
- Thử chữ ký thật: khối nằm trong widget của field chữ ký (không phải trên trang), có `/Matrix` xoay.

## Rủi ro còn lại

Hành vi của Adobe và Foxit với font nhúng dạng này chưa được xác nhận. Kích thước khối có thể lệch nhẹ so
với bản cũ cho tới khi đối chiếu cách đo.
