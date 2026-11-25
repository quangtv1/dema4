# Sổ rủi ro

> Cập nhật: 18/11/2026. R1–R12 lấy từ kế hoạch mục 18; R13 trở đi phát hiện ở giai đoạn 0.
> Khả năng và ảnh hưởng: Thấp / TB / Cao.

## Blocker đang mở

| Mã | Blocker | Chặn việc gì | Cần gì để gỡ |
|---|---|---|---|
| ~~B-01~~ | **ĐÃ GỠ ngày 18/11/2026**: đã cài workload C++ (MSVC 14.51.36231, Windows SDK 10.0.26100.0); 32 test qua trên `1.100.0-x86_64-pc-windows-msvc`. Nội dung cũ: Máy phát triển không có toolchain Rust nào link được code gọi Windows API. MSVC: thiếu `link.exe` và Windows SDK (VS 2026 đã cài nhưng không có workload C++). GNU: `dlltool` báo `CreateProcess` vì thiếu `as.exe`. gnullvm: thiếu `x86_64-w64-mingw32-clang` | S1, S4, S5, S6, S7; `smks-crypto::{store,key,chain}`; Word COM; khung Tauri; build phát hành. **Chặn cả crate thuần dữ liệu có phụ thuộc `windows-sys` hoặc `getrandom`**: `calamine`, `zip`, `lopdf`, `rsa`, `p256`, `rand`, `reqwest`… (bảng ở cuối file) | Cài "Desktop development with C++" và Windows SDK (Q-01) |
| B-02 | Không có USB token, không có middleware bit4id hay SafeNet trên máy; kho chứng thư không có chứng thư nào của Ban Cơ yếu | S1, S6, mốc M0, mọi kiểm thử ký thật | Token thật và bộ cài driver (Q-02) |
| B-03 | Chưa có Adobe Acrobat Reader, chưa cài pyHanko | Nghiệm thu S2, M0, M1 | Cho phép cài (Q-03) |
| B-04 | Repo chưa có remote và CI runner | Chưa chạy được workflow CI | Tạo repo từ xa khi sẵn sàng |

## Rủi ro

| # | Rủi ro | Khả năng | Ảnh hưởng | Cách giảm | Trạng thái |
|---|---|---|---|---|---|
| R1 | Middleware token hành xử khác nhau với CNG/CAPI gọi từ Rust | TB | Cao | Spike S1 với cả hai loại token; giữ cả hai đường CNG và CAPI | Mở, chờ B-01, B-02 |
| R2 | Chữ ký không được Adobe công nhận do sai ASN.1 hoặc ByteRange | TB | Cao | Spike S2a PASS theo pyHanko (9/9 chữ ký, 3 kiểu file, ký chồng, phát hiện sửa). **Chưa kiểm bằng Adobe**; chưa so byte với file bản .NET ký | Đang giảm |
| R3 | Crate RustCrypto thiếu chức năng | TB | TB | Spike S2a: dùng kiểu dữ liệu của `cms` 0.3.0-rc.2 và `x509-tsp` 0.2.0-rc.0, tự dựng `SignerInfo`; đủ dùng và hai crate ghép được với nhau. Còn rủi ro vì cả hai là bản RC: ghim đúng phiên bản trong `Cargo.lock`, theo dõi bản chính thức | Đang giảm |
| R4 | `lopdf` không đọc được PDF lạ | TB | TB | Parser dự phòng port từ `PdfRevisionInfo.cs`; fixtures đa dạng | Mở |
| R5 | `calamine` hiển thị giá trị khác Excel | Cao | TB | Spike S4 xong: 3 file mẫu khớp 193/193 ô. Với ô số phải tự áp định dạng (calamine cho đúng mã định dạng). Test hồi quy bằng `fixtures/table/tricky.xlsx` | Đang giảm |
| R6 | Word COM treo hoặc để lại `WINWORD.EXE` | TB | TB | Spike S7 PASS: luồng STA riêng, Word tự thoát sau `Quit`, kill theo PID làm lời gọi COM trả lỗi ngay. Còn phải thử khi người dùng đang mở Word và trên Office 2016/2019 | Đang giảm |
| R7 | WebView2 `PrintToPdf` không ổn với một số mẫu HTML | Thấp | Thấp | Spike S5 PASS. Lưu ý: `PrintToPdf` báo thành công cả khi in nhầm trang trắng, nên phải kiểm kết quả; khổ giấy phải truyền qua cài đặt in. Chưa thử HTML có ảnh và in liên tiếp | Đang giảm |
| R8 | Nhóm chưa quen Rust và FFI Windows | TB | Cao | Lớp bọc RAII cho mọi handle; lint chặn `unsafe` ngoài module `ffi` (đã bật) | Đang giảm |
| R9 | Phạm vi phình to vì prototype nhiều chức năng | Cao | TB | Chia M/S/L; chỉ M chặn phát hành | Mở |
| R10 | TSA chậm hoặc không ổn định khi ký lô lớn | TB | TB | Giới hạn 8, thử lại 1 lần, báo lỗi từng file | Mở |
| R11 | Pháp lý: lưu PIN, tự ký | Thấp | Cao | Người phụ trách quyết định làm trong 1.5.0 (20/11/2026, TD-41) và chấp nhận rủi ro. Giảm thiểu: tắt sẵn, có cảnh báo, DPAPI theo tài khoản Windows, xoá được, PIN sai thì xoá ngay và không tự thử lại; vẫn cần người bấm "Ký số" (không tự ký khi không có người) | Mở, người phụ trách đã chấp nhận |
| R12 | Kiểm tra thu hồi chậm hoặc lỗi khi offline | TB | Thấp | Timeout ngắn, kết luận "chưa kiểm tra được" | Mở |
| R13 | Môi trường phát triển không build được phần Windows API (B-01), và cả các crate thuần dữ liệu kéo `windows-sys`/`getrandom` | Đã xảy ra | Rất cao | Đã cài C++ build tools ngày 18/11/2026; mọi kiểm tra chạy lại trên MSVC đều qua | **Đóng** |
| R14 | Nhánh G1 sắp hết hạn: `rootca.crt` hết hạn 05/03/2030, `cp.crt` hết hạn 04/03/2030 (đọc từ chứng thư bằng `smks-cli ca`) | Chắc chắn | TB | Ký mới bằng G1 sẽ ngừng trước 2030; xác minh chữ ký G1 cũ vẫn chạy vì dựng chuỗi tại thời điểm TSA đóng dấu. Cần test riêng cho trường hợp "CA đã hết hạn nhưng chữ ký có dấu thời gian hợp lệ" | Mở |
| R15 | Chưa biết chứng thư TSA do CA nào dưới Root G2 cấp và có nằm trong token trả về không | TB | TB | Đã đo ở spike S2a: "Máy chủ cấp dấu thời gian 02" do Root G2 cấp trực tiếp, hết hạn 01/06/2034; token kèm cả chứng thư TSA và Root G2 | **Đóng** |
| R16 | Thay đổi nhìn thấy được so với bản cũ mà người dùng không được báo: nội dung khối chữ ký (K-03), tên file tạo văn bản (K-06), vị trí ép vào trang (K-07), file `_daky_` hiện lại (K-08) | TB | TB | Chốt Q-05, Q-06, Q-07; ghi vào ghi chú phát hành | Mở |
| R17 | File CA ở dạng PEM không chuẩn làm hỏng bước nạp | Đã xảy ra | Cao | Đã xử lý (TD-14) và có test | **Đóng** |
| R18 | Tái hiện sai công thức độ đậm khi chuyển từ `/Decode` sang xử lý điểm ảnh (hai đầu phải kẹp về [0,1] trước) | TB | Thấp | Đã ghi công thức ở `MIGRATION_MAP.md` 3.8; test so với giá trị `/Decode` của bản cũ | Mở |
| R19 | Chữ ký ECDSA từ `NCryptSignHash` ở dạng `r‖s`, CMS cần DER | TB | Cao | Ghi ở `MIGRATION_MAP.md` 3.2; test với khoá ECDSA phần mềm | Mở |
| R20 | Kết luận của agent phân tích prototype/frontend có thể sót hoặc sai chi tiết | TB | TB | Kiểm lại tại chỗ từng phần khi bắt tay vào port màn tương ứng | Mở |
| R21 | Kết quả đọc số từ Excel của bản cũ phụ thuộc vùng văn hoá của máy (K-11) | Chắc chắn | TB | Chốt Q-08; test bằng `fixtures/table/tricky.xlsx` với hai bộ kết quả mong đợi | Mở |
| R22 | Kéo thả: HTML5 drag-drop trong WebView và sự kiện kéo thả file của Tauri loại trừ nhau trên Windows (K-14) | TB | TB | Dùng pointer events cho thao tác trong trang; thử ngay khi có khung Tauri | Mở, chờ B-01 |
| R23 | pdf.js đọc file qua asset protocol của Tauri chưa được thử (CSP, tên file tiếng Việt, thiếu `cmaps`/`wasm` làm PDF scan hiện thiếu ảnh) | TB | TB | Thử ngay khi có khung Tauri, trước khi làm màn Ký số | Mở, chờ B-01 |
| R25 | Font người dùng chọn thiếu glyph tiếng Việt (đã gặp với Georgia): chữ ký hiện ô vuông trên văn bản đã ký, không sửa lại được | Chắc chắn với Georgia | Cao | Kiểm glyph trước khi ký; chốt Q-09. Mọi font trong danh sách phải qua test đủ bộ chữ tiếng Việt | Mở |
| R26 | Chứng thư TSA hiện tại hết hạn 01/06/2034; tên có số thứ tự ("02") nên TSA có thể đổi chứng thư | Thấp | TB | Không ghim chứng thư TSA; chỉ yêu cầu chuỗi về Root G2 và EKU `timeStamping` | Mở |
| R27 | File hybrid-reference (PDF do Word xuất): bộ đọc không hiểu `/XRefStm` sẽ không thấy object nằm trong object stream; công cụ kiểm chặt từ chối loại file này (K-16) | Chắc chắn gặp | TB | Bộ đọc phải xử lý `/XRefStm`; fixtures có PDF do Word xuất; CI kiểm không chặt cho loại này | Mở |
| R28 | Phiên Word ẩn giữ sẵn (xem trước, chạy lô) nhận nhầm tài liệu người dùng tự mở; nếu ứng dụng Quit hay buộc đóng Word lúc đó thì mất phần chưa lưu của họ | Thấp | Cao | Khi rảnh, mỗi giây phiên kiểm `Documents.Count`; có tài liệu lạ thì hiện Word lên, ngừng dùng phiên, không Quit, không buộc đóng (kể cả khi ứng dụng thoát). Kiểm bằng `word::tests::foreign_document_hands_session_over` (giả lập qua `GetActiveObject`): PASS 19/11/2026 | Đang theo dõi |
| R24 | Mang lỗi của giao diện cũ sang bản mới (khớp kết quả theo tên file, một `subscription` dùng chung cho liệt kê và ký, không giải phóng canvas pdf.js…) | TB | TB | 32 điểm đã liệt kê ở `FRONTEND_ANALYSIS.md` mục 11; rà lại khi port từng phần | Mở |

## Crate nào build được bằng toolchain GNU (chỉ còn giá trị tham khảo)

> Từ khi B-01 được gỡ (18/11/2026), dự án build bằng MSVC và bảng dưới không còn là ràng buộc.

Kiểm ngày 18/11/2026 bằng `cargo tree --target x86_64-pc-windows-gnu` (xét cây phụ thuộc; `getrandom` 0.4 và
`windows-sys`/`windows-link` đều cần `raw-dylib`, thứ đang hỏng theo B-01). Riêng `calamine` đã thử build thật
và thất bại đúng như dự đoán.

| Build được | Không build được cho tới khi gỡ B-01 |
|---|---|
| `der`, `x509-cert`, `cms`, `x509-tsp`, `sha2`, `base64ct` | `rsa`, `p256`, `rand` (kéo `getrandom`) |
| `pdf-writer`, `ttf-parser`, `subsetter`, `image` (png, jpeg), `flate2` | `lopdf` (kéo `getrandom`) |
| `quick-xml`, `csv`, `encoding_rs`, `regex`, `unicode-normalization` | `zip` → `calamine`, đọc/ghi DOCX |
| `tokio` (rt, sync, time), `ureq` không TLS, `time`, `serde`, `clap` không màu | `reqwest` (rustls, `ring`), `chrono`, `uuid`, `walkdir` |
| | `tracing-subscriber`, `tracing-appender`, `tempfile`, `insta` |

Hệ quả: trước khi gỡ B-01 chỉ làm được khối chữ ký (Form XObject, subset font, ảnh), bộ ghi nối bản tự viết,
cấu trúc ASN.1 của CMS và TSP. Không ký thử, không xác minh, không đọc Excel/DOCX, không ghi log ra file.
