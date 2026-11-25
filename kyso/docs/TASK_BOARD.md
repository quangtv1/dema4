# Bảng công việc

> Cập nhật: 20/11/2026.
> Trạng thái: TODO · IN_PROGRESS · BLOCKED · REVIEW · TESTING · DONE.
> Chỉ ghi DONE khi có bằng chứng kiểm thử (cột cuối trỏ tới `TEST_REPORT.md` hoặc file kết quả).

## Giai đoạn 0: khảo sát

| ID | Việc | File tạo/sửa | Phụ thuộc | Tiêu chí hoàn thành | Trạng thái | Bằng chứng |
|---|---|---|---|---|---|---|
| P0-01 | Khảo sát repo và môi trường máy | `docs/TEST_REPORT.md` (mục Môi trường) | — | Biết đủ công cụ có/thiếu | DONE | TEST_REPORT, mục Môi trường |
| P0-02 | Đọc backend .NET, lập bảng đối chiếu | `docs/MIGRATION_MAP.md` | P0-01 | Mỗi class cần port có mục hành vi đã xác minh; khác biệt với kế hoạch được liệt kê | DONE | `MIGRATION_MAP.md` (10 khác biệt K-01…K-10) |
| P0-03 | Phân tích prototype HTML | `docs/implementation/PROTOTYPE_ANALYSIS.md` | — | Đủ 11 phần; trích dẫn có số dòng | REVIEW | Đã kiểm tại chỗ 3 trích dẫn (dòng 34, 628, 859); chưa rà toàn bộ |
| P0-04 | Phân tích frontend Angular cũ | `docs/implementation/FRONTEND_ANALYSIS.md` | — | Đủ 11 phần; bảng tái sử dụng | REVIEW | Đã kiểm tại chỗ 3 trích dẫn (khoá localStorage, `sign-step.ts:232`, `measureText`); chưa rà toàn bộ |
| P0-05 | Ghi ADR cho D1–D9 và toolchain | `docs/adr/0001…0010` | P0-02 | Mỗi quyết định một ADR | DONE | 10 file ADR |
| P0-06 | Lập kế hoạch spike S1–S7 | `docs/spikes/*` | P0-02 | Mỗi spike có mục tiêu, phương án, câu hỏi, điều kiện chặn | DONE | 7 file spike, tất cả ghi CHƯA CHẠY |
| P0-07 | Bộ tài liệu quản lý dự án | `docs/*.md` | — | Đủ 7 file theo master prompt mục 8 | DONE | Các file trong `docs/` |

## Giai đoạn 0: nền móng

| ID | Việc | File tạo/sửa | Phụ thuộc | Tiêu chí hoàn thành | Trạng thái | Bằng chứng |
|---|---|---|---|---|---|---|
| F-01 | Git, Cargo workspace, ghim toolchain, lint, định dạng | `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.gitignore`, `.gitattributes` | — | fmt, check, clippy sạch | DONE | TEST_REPORT #1–3 (toolchain GNU) |
| F-02 | Hằng số (Phụ lục A) | `crates/smks-core/src/consts.rs` | P0-02 | Khớp mã nguồn cũ; có test | DONE | TEST_REPORT #4 |
| F-03 | Mã lỗi (Phụ lục B) | `crates/smks-core/src/error.rs` | P0-02 | Mã cũ giữ nguyên số; mã mới 1049–1059; có test | DONE | TEST_REPORT #4 |
| F-04 | Lập kế hoạch dấu (port `StampPlanner.cs`) | `crates/smks-core/src/stamp_plan.rs` | F-03 | Cùng quy tắc và câu thông báo với bản cũ; có test | DONE | TEST_REPORT #4 (16 test) |
| F-05 | Nhúng và ghim 5 chứng thư CA | `crates/smks-crypto/src/trust.rs`, `certs/*.crt` | F-03 | 5/5 khớp SHA-256; phát hiện được CA bị thay | DONE | TEST_REPORT #4, #5, #7 |
| F-06 | Khung CLI (`ca`, `errors`) | `crates/smks-cli/src/main.rs` | F-05 | Chạy được, mã thoát đúng | DONE | TEST_REPORT #5, #6 |
| F-07 | CI cơ bản | `.github/workflows/ci.yml` | F-01 | Workflow xanh trên runner Windows | REVIEW | Đã viết; chưa chạy (B-04) |
| F-08 | Bộ fixtures | `fixtures/merge/*`, `fixtures/table/*`, `fixtures/pdf/*` | — | Có 3 cặp mẫu và bộ PDF theo kế hoạch 15.2 | IN_PROGRESS | Có 3 cặp DOCX/XLSX, `table/tricky.xlsx`, 3 PDF (xref bảng, stream, hybrid của Word), chứng thư TSA công khai; PDF xoay/nhiều trang/mã hoá/DocMDP chưa có file riêng. Thêm dữ liệu lớn 3 file × 2 sheet × 100 người ở `mau-tao-van-ban/du-lieu-100-nguoi/` (sinh bằng `tools/sinh_du_lieu_mau.py`, TEST_REPORT #89); bộ mẫu dài 1–4 trang, 4 cặp × 100 dòng ở `mau-tao-van-ban/mau-nhieu-trang/` (`tools/sinh_mau_nhieu_trang.py`, TEST_REPORT #104) |
| F-09 | Chạy lại toàn bộ kiểm tra bằng toolchain MSVC | — | Q-01 | fmt, clippy, test xanh trên MSVC | DONE | TEST_REPORT #21–24 |
| F-10 | Khung Tauri 2 + Angular 21, `tracing`, quyền tối thiểu | `app/src-tauri/*`, `app/ui/*`, `app/tests/smoke.mjs` | P0-03, P0-04 | `tauri build` ra exe; mở được cửa sổ; giao diện gọi được lõi | DONE | TEST_REPORT #37–40: exe 4,98MB, kiểm tra khói 10/10 |
| F-11 | Kiểu cấu hình `config.json` có `version` và hàm nâng cấp | `crates/smks-core/src/config.rs` | P0-03, P0-04 | Đọc/ghi vòng tròn; file hỏng trả lỗi 1057 và mặc định | DONE | TEST_REPORT #74, #75 (đọc/ghi file ở `app/src-tauri/src/commands/config.rs`) |
| F-12 | Sắp tự nhiên và đặt tên file kết quả (D4) | `crates/smks-core/src/naming.rs` | Q-05 | Test theo quy tắc ở `MIGRATION_MAP.md` 3.10 và ADR 0004 | TODO | |
| F-13 | Thư mục `spikes/` ngoài workspace và quy ước chạy spike | `spikes/README.md` | — | Spike build độc lập, không ảnh hưởng workspace | DONE | `spikes/s4-excel/` có workspace riêng; `cargo test --workspace` ở gốc không đổi |

## Giai đoạn 0: spike

| ID | Việc | Phụ thuộc | Trạng thái | Ghi chú |
|---|---|---|---|---|
| S4 | `calamine` so với ClosedXML trên 3 file mẫu và `tricky.xlsx` | — | DONE | **PASS**: 3 file mẫu khớp 193/193 ô. Ô số cần tự áp định dạng (TEST_REPORT #11–13, #25–26) |
| S2a | CMS + TSA thật + ghi nối bản bằng khoá phần mềm, kiểm bằng pyHanko | — | DONE | **PASS** (TEST_REPORT #31–36): 9/9 chữ ký trên 6 file, 3 kiểu xref, ký chồng, phát hiện sửa |
| S3a | Form XObject chữ tiếng Việt có subset font | F-13 | DONE | PASS 4/5 font bằng pdf.js và pyHanko; Georgia FAIL do font thiếu glyph (Q-09). TEST_REPORT #16–19 |
| S3b | Kiểm khối chữ bằng Adobe, Foxit, trình xem gốc của Edge | S3a, Q-03 | BLOCKED | B-03. Người phụ trách có thể mở `docs/spikes/evidence/S3/s3-font.pdf` để kiểm tay |
| S1 | Ký bằng token thật qua CNG/CAPI | Q-02 | DONE (bit4id) | **PASS** (TEST_REPORT #71): bit4id TokenME EVO v2 + Universal MW 1.4.10.839; liệt kê không hỏi PIN, hộp PIN của middleware, ký + xác minh + pyHanko 10/10. Chưa có SafeNet |
| S2b | Ký bằng token, Adobe công nhận | S1, S2a, Q-03 | BLOCKED | B-03 (đã có file ký bằng token, chờ Adobe) |
| S5 | WebView2 `PrintToPdf` | F-10 | DONE | **PASS** (TEST_REPORT #41–43). Khổ giấy phải truyền qua cài đặt in; lề lấy từ CSS |
| S7 | Word COM trên luồng STA | — | DONE | **PASS** (TEST_REPORT #27–30). Chưa thử khi người dùng đang mở Word |
| S6 | Truyền PIN qua API (cho 1.6) | S1 | BLOCKED | B-02; ngoài phạm vi 1.5.0 |

## Giao diện và Tạo văn bản (làm trước theo TD-25)

| ID | Việc | Trạng thái | Bằng chứng |
|---|---|---|---|
| TV-core | `smks-merge`: bảng tính, định dạng số, điền DOCX, ghép biến, tên file, chạy lô, Word COM | DONE | TEST_REPORT #44–47 |
| TV-ipc | Lệnh IPC `merge_*`, `job_cancel`, `shell_open`, plugin dialog, asset scope cho bản xem trước | DONE | TEST_REPORT #49 (12/12 qua IPC thật) |
| TV-cli | `smks-cli merge` | DONE | TEST_REPORT #46 |
| UI-shell | Menu, phím tắt, Trang chủ (logo động), Thông tin phần mềm theo prototype; màn chưa làm hiện "đang phát triển" | DONE | TEST_REPORT #50, #51; ảnh `docs/implementation/evidence/tao-van-ban/1-trang-chu.png` |
| UI-tvb | Màn Tạo văn bản theo prototype, nối IPC: kéo thả/chọn file, sheet, dòng tiêu đề, ghép biến, nhập trực tiếp, Nâng cao (PDF/DOCX, tên file, thư mục lưu), xem trước PDF bằng pdf.js, chạy lô có tiến độ và Dừng, hộp kết quả | DONE | TEST_REPORT #51 (12/12 trên app thật); ảnh `docs/implementation/evidence/tao-van-ban/` |
| TV-preview | Xem trước nhanh: Word giữ sẵn, bộ đệm theo nội dung, tạo trước dòng lân cận, chốt giao lại phiên Word (TD-31, R28) | DONE | TEST_REPORT #52–56 |
| UI-tvb-3 | Tô màu biến trên bản xem trước (ô "Tô màu biến"), zoom mặc định 100 %, hai ô đường dẫn cùng độ rộng, thanh sheet kiểu Excel trong bảng dữ liệu (TD-32) | DONE | TEST_REPORT #57–59 |
| UI-tvb-4 | Sắp xếp lại khung nguồn theo góp ý người dùng (19/11/2026): 2 dòng, ô đường dẫn giãn hết phần giữa, Sheet / Dòng tiêu đề / Xem dữ liệu / Nâng cao thành khối 2 cột dồn mép phải; Nâng cao thêm đúng 2 dòng (Tên file + Định dạng, Thư mục lưu + Nguồn), có ví dụ tên file; nút thu gọn khung thành một dòng tóm tắt (nhớ theo máy); phím Ctrl+→ / Ctrl+← đổi văn bản đang xem | DONE | TEST_REPORT #60 |
| UI-tvb-5 | Nút **Sửa mẫu** (mở mẫu bằng Word) và **Tải lại** (đọc lại mẫu và dữ liệu, giữ ghép biến và dòng đang xem); bôi đen và sao chép chữ trên bản xem trước (TD-47) | DONE | TEST_REPORT #97; ảnh `docs/implementation/evidence/tao-van-ban/8-boi-den-sao-chep.png`, `9-sua-mau-tai-lai.png` |
| UI-tvb-2 | Bấm biến trên trang, kiểm tay chọn file bằng hộp thoại và nút Dừng trên giao diện | TODO | Chưa có bài kiểm tra tự động cho hộp thoại chọn file và Dừng |
| TV-13 | Đóng dấu đỏ/chữ ký tươi khi tạo | TODO | |
| TV-02 | Mẫu HTML (in bằng WebView2, spike S5) | TODO | |
| TV-09 | Biến ảnh `anh_the`, `con_dau` | TODO | |
| TV-15 | Mở ví dụ mẫu | TODO | |

## Ký số (giai đoạn 1, làm từ 19/11/2026)

| ID | Việc | Trạng thái | Bằng chứng |
|---|---|---|---|
| KS-crypto | `smks-crypto`: cert, key, store (CNG/CAPI), cms, tsa, verify, chain (gốc ghim, thu hồi) | DONE (chưa thử token thật) | TEST_REPORT #63–65 |
| KS-pdf | `smks-pdf`: reader, geometry, appearance (subset font, ảnh), incremental, sigscan | DONE | TEST_REPORT #66 |
| KS-sign | `smks-sign`: ký một file + kiểm lại sau khi ký; xác minh theo 7 mức của bản cũ | DONE | TEST_REPORT #67–69 |
| KS-cli | `smks-cli certs`, `sign`, `verify`, `inspect` | DONE | TEST_REPORT #68 |
| KS-job | Lô ký: quét thư mục, `DaKySo`, tên file, song song, khoá token tuần tự, theo dõi token, Dừng, sự kiện manifest/file/summary | DONE | `smks-sign::job` tests (4); TEST_REPORT #70 |
| KS-ipc | `cert_list`, `token_check`, `token_forget`, `sign_list_files`, `sign_preview`, `sign_run`, `verify_file`; `job_cancel` cho cả lô ký; HWND cho hộp PIN | DONE | TEST_REPORT #70 (`app/tests/ks-ipc.mjs` 15/15) |
| KS-ui | Màn Ký số theo prototype, nối IPC | IN_PROGRESS | Kiểm với token thật qua giao diện (#72). Khung chữ ký kéo thả / đổi cỡ trên bản xem trước (#73). Hộp Cấu hình: mẫu theo khổ giấy, thông tin chữ ký, thư mục & tên file, danh sách chứng thư; khung xem trước theo mẫu (trang cuối, ký nháy) (#75). Màn Xem trước & vị trí chữ ký (nút "Cấu hình chữ ký", menu Vị trí chữ ký): một khung cho tất cả file hoặc riêng từng file, thanh Cỡ, Theo mẫu, Áp dụng / Thoát (#77). Hộp File đã ký / trùng tên (KS-16): Hủy / Giữ cả hai / Ghi đè; Xóa file gốc (KS-04, ADR 0003): xác nhận khi bật, file gốc vào Thùng rác chỉ sau khi chữ ký kiểm lại hợp lệ (#78, #79). Còn: lưu PIN (1.6) |
| KS-nhay | Lượt ký Chính / Nháy (ký nháy mặc định mọi trang); thêm / xoá khung bằng chuột phải và phím Delete; ảnh thu nhỏ trang; `config.json` phiên bản 3 (TD-48) | DONE (khoá thử) | TEST_REPORT #98; ảnh `docs/implementation/evidence/ky-so/nhay-*.png`. **Chưa thử với hai USB token thật** |
| KS-bocuc | Vị trí ảnh so với chữ (Trái / Phải / Trên / Dưới) trong kiểu "Ảnh + thông tin": menu chuột phải và hộp Cấu hình (TD-49) | DONE | TEST_REPORT #99 |
| KS-anhchu | Màn Cấu hình vị trí: vị trí ảnh, phần ảnh chiếm, khoảng cách ảnh – chữ; đổi tên nút "Cấu hình chữ ký" thành "Cấu hình vị trí" (TD-51) | DONE | TEST_REPORT #101 |
| KS-tudo | Kiểu "Tự do": kéo thả ô ảnh và ô thông tin trong ô sửa phóng to ở màn Cấu hình vị trí (TD-52) | DONE | TEST_REPORT #102 |
| KS-tudo-khoi | Vị trí ảnh luôn hiện dù Hiển thị kiểu nào; kéo cạnh / góc khung ngoài ô sửa để đổi cỡ cả khối; mọi khung có tay nắm 4 cạnh để kéo ngang / dọc (TD-53) | DONE | TEST_REPORT #103 |
| KS-gioky | Giờ ký theo máy chủ dấu thời gian Ban Cơ yếu; không có mạng thì ký theo giờ máy tính, không có dấu thời gian (TD-54) | DONE | TEST_REPORT #104 |
| KS-vitri-trang | Màn Cấu hình vị trí: ảnh thu nhỏ từng trang, nhiều khung trên nhiều trang, chuột phải vào ảnh thu nhỏ / trang để thêm, xoá khung (TD-55) | DONE | TEST_REPORT #105 |
| KS-chuagan | Cho xoá hết khung: văn bản "Chưa gán chữ ký" (cảnh báo, trạng thái), bỏ qua khi ký số (TD-56) | DONE | TEST_REPORT #106 |
| KS-verify | Sửa phần kiểm tra chữ ký theo rà soát: DocTimeStamp, sửa sau chữ ký, không TSA vẫn kiểm người ký, thu hồi tại thời điểm ký, nhãn file chưa ký, thuật toán chưa hỗ trợ, keyUsage (V1–V8, TD-57) | DONE | TEST_REPORT #107, #108 |
| KS-ltv | Ký dài hạn PAdES B-LT: OCSP / CRL của Ban Cơ yếu nhúng vào `/DSS` lúc ký, dùng khi kiểm không có mạng (TD-58) | DONE (chưa thử token thật) | TEST_REPORT #109 |
| KT-man | Màn Kiểm tra chữ ký số: chọn / kéo thả file, 5 điều kiện Điều 30, chi tiết chứng thư, vị trí chữ ký trên văn bản, xuất báo cáo PDF (TD-57) | DONE | TEST_REPORT #108 (`app/tests/kt-ui.mjs` 16/16) |
| KS-thumuc | Ký cả thư mục: lấy cả thư mục con, lưu vào thư mục song song `<tên>_DaKySo` giữ nguyên cấu trúc (TD-50) | DONE | TEST_REPORT #100 |
| KS-token | Thử với USB token thật (bit4id, SafeNet), hộp PIN gắn cửa sổ | IN_PROGRESS | bit4id: `smks-cli` (#71) và giao diện app (#72) đạt, hộp PIN gắn cửa sổ chính. **Lỗi mở:** rút token giữa lô, app treo đến khi cắm lại mới báo (#72). Chưa có SafeNet |
| KS-pin | Lưu PIN / tự nhập PIN (D2, KS-23, CH-08) và xoá chứng thư khỏi danh sách (CH-07) | IN_PROGRESS | Đã kiểm bằng khoá thử (TEST_REPORT #85, #86). Chưa kiểm S6 với token bit4id thật (TD-41) |
| HT-01/02 | Hướng dẫn sử dụng (F1); Cài đặt driver USB Token: kiểm tra máy, hướng dẫn tải ở trang của Ban Cơ yếu | DONE | TEST_REPORT #88, TD-43. Chưa có nút cài driver kèm app (chờ đưa bộ cài vào gói) |

## Thông báo Telegram và đăng ký thiết bị (TD-46, 20/11/2026)

| ID | Việc | Trạng thái | Bằng chứng |
|---|---|---|---|
| TB-core | `smks-notify`: sự kiện, nhật ký sự kiện, thiết bị, mẫu tin, hàng đợi (thử lại có giới hạn, chống trùng), kênh Telegram trực tiếp và kênh gateway | DONE | TEST_REPORT #92 (27 test) |
| TB-gateway | `smks-gateway`: đăng ký thiết bị bằng khoá đăng ký, bí mật riêng từng thiết bị, thu hồi, chống trùng, hạn mức, công tắc ở máy chủ; lệnh `check`, `devices`, `revoke`, `restore`, `remove` | DONE | TEST_REPORT #92 (3 test), #93 phần H, #95 (`check` với Telegram thật). Chưa kiểm sau reverse proxy TLS, trên Linux |
| TB-ipc | Lệnh `notify_status`, `notify_consent`, `notify_unregister`, `notify_set_prefs`, `notify_check`, `notify_test`; bộ gom lô ký / lô tạo văn bản | DONE | TEST_REPORT #93 |
| TB-ui | Hộp Đăng ký thiết bị (lần chạy đầu), hộp Hỗ trợ › Thông báo Telegram, mục Hướng dẫn | DONE | TEST_REPORT #93 phần C, F; ảnh `docs/implementation/evidence/thong-bao/` |
| TB-real | Gửi thật tới Telegram bằng token của người phụ trách | DONE | TEST_REPORT #95 (5 tin). Chờ người phụ trách xác nhận bằng mắt trong nhóm |
| TB-pack | Bản phát hành, gói chạy ngay (zip), file cấu hình mẫu, tài liệu | DONE | TEST_REPORT #94, #96; `installer/installer-output/` (không commit) |
| TB-clean | Chạy trên máy Windows sạch (chưa cài Rust / Node.js) | TODO | Chưa có máy để thử; mới kiểm bảng import của file chạy |
| TB-token | Kiểm với USB Token thật: PIN sai, hủy hộp PIN, rút token giữa lượt | TODO | Cần người phụ trách thao tác với token |
| TB-ext | Nối `DOCUMENT_EXPORTED`, `DOCUMENT_DELETED` vào nghiệp vụ; bài kiểm trên app cho DOCUMENT CANCELLED | TODO | |
| TB-nsis | Bộ cài NSIS đặt `notify.json` vào `%PROGRAMDATA%` | TODO | Phụ thuộc bộ cài của kế hoạch mục 16 |

## Mốc

| Mốc | Điều kiện | Trạng thái |
|---|---|---|
| M0 | `smks-cli sign` ký một PDF bằng token thật; Adobe báo hợp lệ; pyHanko xác nhận PAdES B-T | **CHƯA ĐẠT**: ký bằng token và pyHanko đã đạt (#71); còn chờ Adobe Reader (B-03) |
| M1–M7 | Xem `IMPLEMENTATION_ROADMAP.md` | Chưa bắt đầu |

## Việc kế tiếp theo thứ tự đề xuất

B-01 đã gỡ. Thứ tự:

1. Thử pdf.js qua asset protocol (R23) và kéo thả (R22) trên khung ứng dụng, cùng lúc làm màn Ký số.
2. Giai đoạn 1 theo thứ tự ở `IMPLEMENTATION_ROADMAP.md`.
3. S1 đã đạt với bit4id; S2b → M0 chờ Adobe Reader (B-03).
