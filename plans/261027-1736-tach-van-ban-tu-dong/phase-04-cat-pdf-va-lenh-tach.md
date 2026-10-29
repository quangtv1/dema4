---
phase: 4
title: "Cắt PDF và lệnh tách"
status: in-progress
priority: P1
dependencies: [2]
effort: "0,5–1 ngày"
---

# Phase 4: Cắt PDF và lệnh tách

## Overview
Từ hồ sơ + nhãn → ghi các part PDF và `ketqua_tach.txt`; thêm lệnh dòng lệnh để chạy không cần GUI (dùng cho test vàng và người dùng nâng cao).

## Requirements
- **Thư mục ra** = `<thư mục cha>/<tên thư mục nguồn>_vanban`; thư mục cha lấy từ Cấu hình "Lưu kết quả vào", trống = cha của thư mục nguồn (kết quả nằm ngang hàng thư mục nguồn, không ghi lẫn vào nguồn).
- **Giữ nguyên cấu trúc thư mục nguồn (mọi tầng) + thêm 1 tầng thư mục hồ sơ**: PDF `<nguồn>/<a>/<b>/<hoso>.pdf` → thư mục `<ra>/<a>/<b>/<hoso>/` chứa các văn bản `<tên theo mẫu>.pdf`. Vd mẫu 2: `mau2/HOP 462/4766/4766.pdf` → `mau2_vanban/HOP 462/4766/4766/4766_part1.pdf`. PDF nằm ngay gốc nguồn → `<ra>/<hoso>/`. Chỉ ghi kết quả của PDF; file khác và thư mục không có PDF không chép. Giữ nguyên tên thư mục (dấu cách, tiếng Việt có dấu). Mẫu mặc định `{hoso}_part{n}` (= b3 + b4 cũ); biến `{hoso}` `{n}` `{n3}`; bắt buộc có `{n}`/`{n3}`; ký tự không hợp lệ cho tên file bị thay `_`.
- **Xoay**: trang có góc xoay → trang trong file đầu ra đặt `/Rotate = (Rotate gốc + góc) mod 360`; chỉ sửa đối tượng trang ở file đầu ra, **không ghi vào PDF gốc**; ảnh không bị nén/vẽ lại.
- **Hồ sơ không có nhãn nào và không xoay trang nào** (không index/blank/split) → **chép nguyên file** (byte y hệt, giữ cả chữ ký số) thành văn bản 1: `<ra>/…/<hoso>/<tên mẫu với n=1>.pdf`; không mở/ghi lại bằng MuPDF.
- Dùng cách cắt chọn ở phase 1; không nén lại ảnh.
- `ketqua_tach.txt` ở **`<thư mục đầu vào>/.sm-dema4/`** (cùng file nhãn + cache; đường dẫn file đầu ra ghi tuyệt đối) (UTF-8, tab, có tiêu đề), **ghi đè** mỗi lần chạy, STT liên tục (không thêm cột mới; hồ sơ chép nguyên: dải `1-<tổng>`): `STT, File đầu vào, Dải trang (1-based), File đầu ra, Trang trắng đã bỏ (1-based), Số trang`. Bỏ cột lệnh qpdf (sai từ b3, không dùng).
- Part đã tồn tại → ghi đè; xoá part thừa cũ trong thư mục `<hoso>/` tương ứng để không lẫn kết quả lần trước.
- Song song theo hồ sơ (rayon), tiến độ + hủy như phase 3; một hồ sơ lỗi không chặn hồ sơ khác.
- CLI: `sm-dema4 --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục>]` (mặc định nhãn đọc/ghi ở `<thư mục pdf>/.sm-dema4/index_and_blank.txt`; `--nhan` cho phép dùng file nhãn kiểu cũ ở nơi khác, vd mẫu 1). CLI **không phân tích** nên không có cờ Nghi trắng / Nghi xoay và không chặn như nút Tách trong GUI; có `<thư mục pdf>/.sm-dema4/xoay_trang.txt` thì áp góc xoay như GUI
  - Không có `--nhan` → chạy phát hiện (phase 3) rồi cắt luôn, đồng thời ghi `index_and_blank.txt` để kiểm tra.
  - Không đổi hành vi `sm-dema4 <thư mục>` (đếm).

## Related Code Files
- Modify: `dema4_rs/src/tach_vb.rs`
- Modify: `dema4_rs/src/main.rs` (nhánh `--tach`)

## Implementation Steps (tests first)
1. Test đơn vị (PDF sinh trong test, 10 trang có nhãn): đúng số part, đúng trang từng part (so hash nội dung trang), blank bị loại, split giữ trang đầu văn bản mới, log đúng cột 1-based, chạy lại không trùng dòng, part thừa cũ bị xoá.
1b. Cấu trúc thư mục: nguồn có thư mục con 2 cấp → ra đúng `<ra>/<a>/<b>/<hoso>/`; hai hồ sơ trùng tên ở hai thư mục con khác nhau không đè nhau; Cấu hình cha trống → `<cha nguồn>/<tên>_vanban`, có giá trị → `<cha>/<tên>_vanban`.
1b'. Cấu trúc như mẫu 2 (3 tầng, tên có dấu cách `HOP 462/4766/4766.pdf`) + PDF ở gốc + thư mục chỉ có file .docx/.jpg → đúng đường dẫn ra, không tạo thư mục thừa. Test vàng `#[ignore]` (`DEMA4_MAU2`): tách cả mẫu 2 → số thư mục hồ sơ = 234, cây thư mục ra khớp cây nguồn.
1d. Xoay: trang gốc /Rotate 0 xoay 90 → ra 90; gốc 90 xoay 270 → ra 0; file gốc sau khi tách giống từng byte trước khi tách; hồ sơ không nhãn nhưng có xoay → cắt bằng MuPDF (không chép nguyên).
1c. Hồ sơ không nhãn → file ra giống từng byte file gốc, tên `<hoso>_part1.pdf`; hồ sơ chỉ có blank → vẫn cắt bằng MuPDF.
2. Test tên hồ sơ có `_` và dấu tiếng Việt → thư mục đúng (lỗi b4 cũ); mẫu `{hoso}_{n3}` → `L1_05_13_001.pdf`; mẫu thiếu `{n}` bị từ chối; dọn part thừa dùng đúng mẫu đang áp.
3. Test vàng `#[ignore]` (DEMA4_MAU_TACH): cắt `mau/pdf_goc` theo `mau/index_and_blank.txt` → 211 part; từng part: số trang + hash ảnh từng trang khớp `mau/pdf_vanban`.
4. Test CLI: `--tach` thiếu thư mục → mã thoát ≠ 0, thông báo rõ; có `xoay_trang.txt` → trang ra mang đúng `/Rotate`; lệnh đếm cũ vẫn chạy như trước.
5. Hiện thực tới khi xanh.

## Success Criteria
- [x] 211/211 part khớp b3 (số trang + băm ảnh từng trang); cắt 0,8–0,9 s. Code: `dema4_rs/src/tach_vb/tach.rs`, CLI `--tach` ở `main.rs`.
- [x] Log 1-based nhất quán, chạy lại idempotent (part thừa cũ bị xoá, file khác giữ nguyên).
- [x] Toàn bộ test đếm cũ xanh.
- [ ] Còn lại: CLI không có `--nhan` và chưa có nhãn → chạy phát hiện (cần phase 3; hiện báo "chưa có file nhãn", mã thoát 2); test vàng cấu trúc mẫu 2 (234 thư mục hồ sơ) — chạy khi có nhãn mẫu 2 theo đúng đường dẫn `HOP …/…/….pdf`.

## Risk Assessment
- Ghi đè/xoá part cũ nhầm thư mục người dùng → chỉ xoá file khớp mẫu tên đang áp trong thư mục `<hoso>/` của thư mục ra; test bảo vệ file khác không bị đụng.
- Thư mục cha trong Cấu hình nằm **bên trong** thư mục nguồn → lần quét sau sẽ thấy cả kết quả: từ chối ở Cấu hình (báo lỗi), test.
- Đĩa đầy/không quyền ghi → lỗi theo hồ sơ, báo rõ, không panic.
