# Trạng thái dự án

> Cập nhật: 23/11/2026 (phiên 5). Đọc file này đầu mỗi phiên, cùng `TASK_BOARD.md` và `git status`.

## Tóm tắt

- **Thứ tự làm (TD-25):** giao diện theo prototype và Tạo văn bản trước, Ký số sau. Từ 19/11/2026 làm Ký số
  (nhánh `feat/ky-so-giai-doan-1`, tách từ `build/nen-mong-giai-doan-0`).
- **Dùng được ngay:** `SaoMai-KySo.exe` có Trang chủ, menu, **Tạo văn bản chạy thật**. Lõi **Ký số** chạy thật
  qua `smks-cli` (ký PAdES B-T có dấu thời gian của Ban Cơ yếu, xác minh); màn Ký số đang làm.
- **Đã ký thật bằng USB token bit4id** qua `smks-cli` (#71) và qua giao diện app (#72): hợp lệ, về Root G2,
  pyHanko 10/10; hộp PIN gắn cửa sổ chính. **Lỗi mở:** rút token giữa lô thì app treo tới khi cắm lại (#72).
- **Thông báo Telegram và đăng ký thiết bị (TD-46, 20/11/2026): xong, đã gửi thật.** Tắt hẳn khi máy chưa có
  `notify.json`; chỉ gửi sau khi người dùng đồng ý. Tài liệu: `docs/THONG_BAO_TELEGRAM.md`.
- **Tạo văn bản (TD-47, 20/11/2026):** thêm nút Sửa mẫu (mở bằng Word), Tải lại (giữ ghép biến, dòng đang xem), bôi đen
  và sao chép chữ trên bản xem trước. Đã kiểm với Word thật (#97).
- **Ký nháy / ký chính (TD-48, 20/11/2026):** mỗi lần ký một chữ ký; chọn lượt Chính / Nháy (nháy mặc định mọi trang),
  thêm / xoá khung bằng chuột phải hoặc Delete, ảnh thu nhỏ trang. Đã kiểm bằng khoá thử (#98); chưa thử hai USB thật.
- **Vị trí ảnh so với chữ (TD-49, 20/11/2026):** ảnh trái / phải / trên / dưới thông tin, đổi bằng chuột phải vào khung hoặc
  hộp Cấu hình. Đã ký thật bằng khoá thử và đo trên PDF (#99).
- **Ký cả thư mục (TD-50, 23/11/2026):** lấy cả thư mục con; file đã ký lưu vào thư mục cùng cấp `<tên thư mục>_DaKySo`,
  giữ nguyên cấu trúc thư mục con, thư mục gốc không bị ghi thêm. Đã kiểm bằng khoá thử (#100).
- **Ảnh so với thông tin (TD-51, 23/11/2026):** nút "Cấu hình vị trí" (đổi tên từ "Cấu hình chữ ký"); ở Nâng cao chọn vị trí ảnh,
  kéo "Ảnh chiếm" và "Khoảng cách", áp cho mọi văn bản. Đã ký thật bằng khoá thử và đo trên PDF (#101).
- **Kéo thả ảnh / thông tin (TD-52, 23/11/2026):** nút "Tự do" ở hàng Vị trí ảnh mở ô sửa phóng to, kéo ô ảnh và ô thông tin,
  kéo góc đổi cỡ. Đã ký thật bằng khoá thử và đo trên PDF (#102).
- **Khung ngoài ô sửa (TD-53, 23/11/2026):** hàng Vị trí ảnh luôn hiện; kéo cạnh / góc khung ngoài để đổi cỡ cả khối; khung, ô ảnh / ô chữ và khung trên trang kéo ngang / dọc theo cạnh (#103).
- **Giờ ký (TD-54, 23/11/2026):** lấy theo máy chủ dấu thời gian Ban Cơ yếu; không có mạng thì ký theo giờ máy tính, văn bản không có
  dấu thời gian, Nhật ký và hộp kết quả ghi rõ (#104).
- **Cấu hình vị trí nhiều trang (TD-55, 23/11/2026):** ảnh thu nhỏ từng trang; chuột phải vào ảnh thu nhỏ hoặc trang để thêm / xoá
  khung, nhiều khung trên nhiều trang (#105).
- **Chưa gán chữ ký (TD-56, 23/11/2026):** xoá hết khung được; văn bản có trạng thái "Chưa gán chữ ký" và được bỏ qua khi ký số (#106).
- **Kiểm tra chữ ký (TD-57, 24/11/2026):** rà soát theo Nghị định 68/2024 Điều 30–32 (#107), sửa 8 lỗi (sửa sau khi ký, dấu thời gian
  tài liệu, không TSA vẫn kiểm người ký, thu hồi tại thời điểm ký, RSA-PSS…) và làm màn Kiểm tra chữ ký số có xuất báo cáo PDF (#108).
- **Bản phát hành một file exe có Thông báo Telegram (TD-59, 24/11/2026):** `dist/SaoMai-KySo.exe`, cấu hình gửi tới nhóm
  "Sao Mai Notification" nhúng trong exe; lần chạy đầu tự đăng ký thiết bị (không hỏi, TD-60) gửi tên máy, IP, MAC; mỗi lượt
  ký số / tạo văn bản gửi kết quả (#110–#112). Bot Token được dán vào hội thoại ngày 24/11/2026 và lấy lại được từ exe; khi lộ thì `/revoke` ở @BotFather rồi
  build lại.
- **Ký dài hạn (TD-58, 24/11/2026):** ký có mạng thì nhúng OCSP / CRL của Ban Cơ yếu vào `/DSS` (PAdES B-LT); màn Kiểm tra dùng dữ
  liệu nhúng khi không có mạng (#109). Chỉ tin CA Ban Cơ yếu, không kiểm chữ ký số công cộng (người phụ trách quyết).
- **Mốc M0: chưa đạt**: chỉ còn chờ Adobe Reader xác nhận (B-03).
- **Nhịp làm việc:** không theo lịch tuần (TD-20).

Cách chạy: `.\target\release\SaoMai-KySo.exe` (build bằng `cd app; ui\node_modules\.bin\tauri build --no-bundle`).
Ký thử bằng dòng lệnh: `cargo build -p smks-cli --release --features dev-keys`, rồi
`target\release\smks-cli.exe sign <vào.pdf> <ra.pdf> --dev-cert … --dev-key … --visible` (khoá thử của spike S2),
hoặc `--thumbprint <SHA-1>` với chứng thư trên token.

## Đã hoàn thành (có bằng chứng trong `TEST_REPORT.md`)

**Khảo sát và tài liệu**

- Đọc toàn bộ backend .NET cần port → `MIGRATION_MAP.md` (16 điểm kế hoạch khác mã nguồn cũ, K-01…K-16).
- 10 ADR, kế hoạch và kết quả 7 spike, bộ tài liệu quản lý.

**Mã sản phẩm** (toolchain MSVC 1.100.0; fmt, clippy sạch; 32 test qua)

- `smks-core`: hằng số, mã lỗi, lập kế hoạch dấu (port `StampPlanner.cs`).
- `smks-crypto::trust`: 5 chứng thư CA nhúng, kiểm SHA-256.
- `smks-cli`: lệnh `ca`, `errors`.
- `smks-app` + `app/ui`: khung Tauri 2 + Angular 21. `SaoMai-KySo.exe` (4,98MB) mở cửa sổ 1280×820, giao diện
  gọi lõi qua IPC, CSP chặt, quyền tối thiểu, chỉ chạy một bản, ghi log theo ngày. Kiểm tra khói 10/10.

**Tạo văn bản** (TEST_REPORT #44–51)

- `smks-merge`: đọc Excel/CSV khớp ClosedXML, định dạng số theo Windows, điền DOCX, ghép biến, tên file, chạy lô, Word COM.
- Giao diện màn Tạo văn bản theo prototype, nối IPC; kiểm 12/12 trên app thật bằng thao tác giao diện.
- `smks-cli merge` chạy được toàn bộ quy trình không cần giao diện.
- Xem trước nhanh (TD-31): Word giữ sẵn, bộ đệm, tạo trước dòng lân cận; đổi dòng dưới 0,3 giây (TEST_REPORT #52–56).
- Chưa làm: đóng dấu đỏ/chữ ký tươi khi tạo (TV-13), mẫu HTML (TV-02), biến ảnh (TV-09), tô màu trường, mở ví dụ mẫu.

**Ký số** (TEST_REPORT #63–81)

- `smks-crypto`: kho chứng thư (không mở khoá khi liệt kê), khoá CNG/CAPI, CMS PAdES, TSA (kiểm chữ ký TSA bằng
  Rust), chuỗi CA với kho gốc độc quyền chỉ chứa Root G1/G2, thu hồi.
- `smks-pdf`: đọc, ghi nối bản (3 kiểu xref), khối chữ ký tiếng Việt và ảnh dấu, hình học trang, quét chữ ký.
- `smks-sign`: ký + kiểm lại ngay; xác minh theo 7 mức của bản cũ; lô ký (DaKySo, song song, theo dõi token, Dừng).
- `smks-cli`: `certs`, `sign`, `verify`, `inspect`. pyHanko xác nhận 50/50 phép kiểm.
- Lệnh IPC của màn Ký số kiểm 15/15 trên app (#70); giao diện đã dựng, chưa kiểm trên app.
- Token bit4id TokenME EVO v2 + Bit4id Universal MW 1.4.10.839: ký và xác minh đạt (#71).
- Màn Ký số: khung chữ ký hiện trên bản xem trước, kéo đổi chỗ / kéo góc đổi cỡ, chữ ký thật nằm đúng khung (#73).
- Cấu hình `config.json` (F-11) và hộp Cấu hình: mẫu chữ ký theo khổ giấy (vị trí 6 điểm, trang đầu / cuối / ký nháy, lề, tỉ lệ), font, cỡ, màu chữ, lý do, nơi ký, ảnh dấu / chữ ký tay, thư mục & hậu tố, danh sách chứng thư (#74, #75). Mẫu mặc định theo prototype: dưới phải, trang cuối, lề / tỉ lệ theo khổ (Q-10, #80, #81).
- Màn Xem trước & vị trí chữ ký (nút "Cấu hình chữ ký"): một khung cho tất cả file hoặc khung riêng từng file, ký đúng khung từng file (#76, #77).
- Hộp File đã ký / trùng tên (Giữ cả hai / Ghi đè) và Xóa file gốc (file gốc vào Thùng rác, chỉ sau khi kiểm lại hợp lệ) (#78, #79).

**Thông báo Telegram** (TEST_REPORT #92–96, TD-46)

- `smks-notify`: sự kiện thống nhất, nhật ký sự kiện, đăng ký thiết bị có đồng ý, mẫu tin, hàng đợi lưu đĩa (thử lại
  có giới hạn, chống trùng), gửi qua gateway hoặc thẳng Telegram. Bot Token không nằm trong EXE.
- `smks-gateway`: máy chủ giữ Bot Token, xác thực và thu hồi từng thiết bị.
- App: hộp Đăng ký thiết bị, hộp Hỗ trợ › Thông báo Telegram; một lượt ký / tạo văn bản ra tối đa 3 tin.
- Gói chạy ngay ở `installer/installer-output/` (không commit).
- Chưa làm: kiểm trên máy Windows sạch, kiểm với USB Token thật, bộ cài NSIS, `DOCUMENT_EXPORTED` / `DOCUMENT_DELETED`.

**Spike**

| Spike | Kết quả |
|---|---|
| S2a CMS + dấu thời gian + ghi nối bản | PASS: pyHanko xác nhận 9/9 chữ ký trên 6 file; TSA thật của Ban Cơ yếu |
| S3a Khối chữ tiếng Việt nhúng subset font | PASS 4/5 font; **Georgia không có glyph tiếng Việt** |
| S4 Đọc Excel bằng calamine | PASS: 3 file mẫu khớp ClosedXML 193/193 ô; ô số phải tự áp định dạng |
| S5 In HTML ra PDF bằng WebView2 | PASS; khổ giấy phải truyền qua cài đặt in |
| S7 Word COM trên luồng STA | PASS: PDF trùng bản tham chiếu, Word thoát sạch |

## Chờ rà soát

- `implementation/PROTOTYPE_ANALYSIS.md` và `implementation/FRONTEND_ANALYSIS.md` do agent lập. Đã kiểm tại
  chỗ một số trích dẫn, chưa rà toàn bộ. Phải kiểm lại từng phần khi port màn tương ứng (R20).
- `.github/workflows/ci.yml`: đã viết, **chưa chạy trên runner** (B-04).

## Chưa làm

- Spike S2b, S3b (chờ Adobe, Foxit).
- Spike S6 (middleware bit4id có nhận PIN do app đưa vào không): code đã có (TD-41), **chờ người phụ trách thử với token thật**.
- Toàn bộ giai đoạn 1 đến 7: mã spike chưa được đưa thành module sản phẩm.

## Blocker

| Mã | Nội dung | Cần ai làm gì |
|---|---|---|
| B-02 | Đã có token bit4id và middleware (19/11/2026); chưa có SafeNet | Cấp token SafeNet nếu cần hỗ trợ |
| B-03 | Thiếu Adobe Acrobat Reader và Foxit (pyHanko đã cài) | Cho phép cài, hoặc tự mở file bằng chứng trong `docs/spikes/evidence/` để kiểm tay |
| B-04 | Chưa có remote git và CI runner | Tạo khi sẵn sàng |

B-01 (thiếu C++ build tools) đã gỡ ngày 18/11/2026.

## Phát hiện chính

1. **PDF do Word xuất là file hybrid-reference** (bảng xref kèm `/XRefStm`). Bản cũ không đọc `/XRefStm`;
   pyHanko ở chế độ chặt từ chối kiểm chữ ký trên loại file này (K-16, R27).
2. **Font Georgia không hiện được tiếng Việt có dấu** (K-15, Q-09).
3. **Số đọc từ Excel của bản cũ phụ thuộc vùng văn hoá của Windows** (K-11, Q-08).
4. Chứng thư TSA "Máy chủ cấp dấu thời gian 02" do Root G2 cấp trực tiếp, hết hạn 01/06/2034.
5. Nhánh CA G1 hết hạn tháng 3/2030 (R14).
6. Bản cũ đóng dấu đỏ và chữ ký tươi lúc ký bằng chú thích `/Stamp`, không sửa nội dung trang.
7. `PrintToPdf` của WebView2 báo thành công cả khi in nhầm trang trắng; phải kiểm kết quả.
8. Tauri tự thêm nonce vào CSP, làm `'unsafe-inline'` vô hiệu; Angular phải dùng nonce của Tauri (TD-21).
9. Prototype mặc định bật lưu PIN với PIN mẫu (ngược D2), ký DOCX ra `_signed.docx` (ngược D5).

## Câu hỏi đang chờ trả lời (`TECHNICAL_DECISIONS.md`, mục 3)

| Mã | Nội dung | Chặn việc gì |
|---|---|---|
| Q-02 | Khi nào có token và driver | S1, M0 |
| Q-05 | Xử lý file `_daky_…` do bản cũ để lại | Liệt kê file ở màn Ký số |
| Q-06 | Nội dung khối chữ ký: hai dòng trần như bản cũ, hay `Ký bởi: …` / `Ngày ký: …` | `smks-pdf::appearance` |
| Q-07 | Quy tắc đặt tên file Tạo văn bản | Giai đoạn 5 |
| Q-08 | Vùng văn hoá khi định dạng số từ Excel | `smks-merge::table` |
| Q-09 | Xử lý font Georgia | `smks-pdf::appearance` |

## Việc kế tiếp

1. Màn Ký số theo prototype, nối IPC, kiểm trên app thật (bản build `dev-keys`).
2. Thử ký dài hạn và màn Kiểm tra với USB Token thật (OCSP của chứng thư người ký, CRL của CA trung gian); mở file
   bằng Adobe Reader xem có "LTV enabled" (B-03).
3. Sửa phát hiện rút token (đầu đọc thẻ WinSCard, không chờ lời gọi đang treo), kiểm lại với token; Adobe Reader → M0.
4. Tạo văn bản còn: đóng dấu khi tạo (TV-13), mẫu HTML (TV-02), biến ảnh (TV-09).
5. Thông báo Telegram: người phụ trách xác nhận 5 tin trong nhóm; thử gói trên máy Windows sạch; thử với USB Token
   thật (PIN sai, rút token); dựng gateway thật nếu triển khai nhiều máy (TB-clean, TB-token).

## Trạng thái git

- `main` chỉ có commit khởi tạo.
- `build/nen-mong-giai-doan-0`: nền móng, giao diện, Tạo văn bản. `feat/ky-so-giai-doan-1` (tách từ nhánh trên):
  Ký số. Cả hai **chưa gộp vào `main`** (chờ review).
- Chưa có remote. Không tự push, không tự tạo release.
- Phần Thông báo Telegram (TD-46) **chưa commit** (20/11/2026), nằm trong cây làm việc của `feat/ky-so-giai-doan-1`.
- **Token đã lộ trong hội thoại:** Bot Token dùng để kiểm thử được dán vào phiên làm việc ngày 20/11/2026. Nên
  thu hồi ở `@BotFather` (`/revoke`) và cấp token mới trước khi triển khai thật.
