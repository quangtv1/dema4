---
title: "Tách văn bản tự động (index Code 128 + trang trắng) trong sm-dema4"
description: "Thay pipeline Python b1–b4: tự phát hiện trang index (Code 128) và trang trắng, cho người sửa nhãn, cắt hồ sơ PDF thành các văn bản"
status: pending
priority: P2
branch: "main"
tags: [rust, egui, mupdf, zxing-cpp, onnxruntime, tdd]
blockedBy: []
blocks: []
created: "2026-10-27T10:37:28.328Z"
createdBy: "ck:plan"
source: skill
---

# Tách văn bản tự động (index Code 128 + trang trắng) trong sm-dema4

## Overview
Mỗi hồ sơ scan là 1 PDF; các văn bản cách nhau bởi **trang index** có mã vạch Code 128 (mẫu: `NAMSAO-TACH`, kèm Kodak Patch T). Scan 2 mặt sinh **trang trắng** (kể cả bìa màu). Hiện làm tay 8/11 bước bằng WSL + Python (b1 xuất JPG → người phân loại → b3 cắt → b4 gom).
Mục tiêu: chức năng **"Tách văn bản"** trong sm-dema4: tự đánh dấu → người thêm/sửa/bỏ nhãn Index/Trắng/Split → xem trước từng văn bản → cắt ra `<nguồn>_vanban/…/<hoso>/<tên theo mẫu>.pdf` (giữ cấu trúc thư mục) + `ketqua_tach.txt`. Không cần WSL/Python.

Nguồn: brainstorm `temp/function2/plans/reports/brainstorm-261027-1700-tach-van-ban-tu-dong-rust-report.md` (repo gitignore `temp/`).
Đo trên mẫu 896 trang (zxing-cpp + PyMuPDF): index 209/209 đúng, 0 thừa/thiếu; trắng 0,0000 vs nội dung min 0,0105; 17 s/10 lõi.

**Đặc tả giao diện = prototype đã duyệt** [`prototype-tach-van-ban.html`](./prototype-tach-van-ban.html) (mở trực tiếp bằng trình duyệt). Phase 5–6 mô tả bằng chữ + ánh xạ sang egui; lệch nhau → theo prototype.

## Quyết định đã chốt
**Lõi**
- Rust thay toàn bộ Python; nhãn: **index** (điểm cắt, trang bị bỏ), **blank** (bị bỏ), **split** (cắt trước trang, trang được giữ — khi lúc scan quên chèn tờ index); còn lại là content.
- **Đầu vào = thư mục cha, quét mọi tầng thư mục con** (bỏ thư mục ẩn/hệ thống, `.pdf`/`.PDF`), mỗi PDF là 1 hồ sơ. Vd mẫu 2: `mau2/HOP 462/4766/4766.pdf`.
- Trang trắng: **chỉ số v3** (100 dpi, nền cục bộ, cụm nét **đậm** + 0,3 × mực màu) + **2 ngưỡng** mặc định **1,0 % / 3,0 %** (chốt bằng nhãn tay mẫu 2): dưới ngưỡng trắng và không có mực màu → Trắng; còn lại dưới ngưỡng nghi → cờ **Nghi trắng**, người dùng tự phân loại; trên → Content. Chữ thấm từ mặt sau, vân giấy **là trắng**; trang chỉ có dấu đỏ/xanh hoặc viết tay màu nhạt → **Nghi trắng**.
- Nhãn lưu **1 file `<thư mục đầu vào>/.sm-dema4/index_and_blank.txt`** (cùng thư mục cache) cho mọi hồ sơ, đường dẫn tương đối từ gốc thư mục đầu vào (`HOP 462/4766/4766.pdf;3;index`), định dạng cũ `đường_dẫn;trang;index|blank` + giá trị mới `split` (1-based; b3 cũ bỏ qua `split`, không lỗi).
- Đầu ra `<thư mục cha>/<tên nguồn>_vanban/` (cha trong Cấu hình, mặc định cạnh thư mục nguồn), **giữ nguyên cấu trúc thư mục con + thêm 1 tầng thư mục hồ sơ** chứa các văn bản: `mau2/HOP 462/4766/4766.pdf` → `mau2_vanban/HOP 462/4766/4766/4766_part1.pdf`, `…_part2.pdf`; chỉ tạo thư mục cho PDF (file khác, thư mục không có PDF không chép) (mặc định `{hoso}_part{n}`). Hồ sơ không có nhãn nào → chép nguyên file thành văn bản 1. `ketqua_tach.txt` cũng ghi vào `<thư mục đầu vào>/.sm-dema4/` (không thêm cột; đường dẫn đầu ra trong log là tuyệt đối). Chạy lại: ghi đè + xoá part thừa (chỉ file đúng mẫu); log ghi đè.
- **Phân tích 2 bước**: (1) đọc dung lượng ảnh nén mọi trang (không render, vài giây/100k trang) → lưới sắp/duyệt được ngay; (2) phân tích nền, ưu tiên trang đang hiện trên lưới; người dùng duyệt trong lúc chạy. Mỗi trang render 150 dpi **một lần** dùng chung cho đọc mã, dò trắng (thu về 100 dpi), mô hình hướng trang, ảnh nhỏ.
- **Chiều trang**: mô hình **PP-LCNet_x1_0_doc_ori** (ONNX, nhúng trong exe, chạy bằng **`ort`**, dự phòng `tract`) **2 tầng**: chạy 1 lần, tin cậy ≥ 0,8 thì kết luận; còn lại chạy thêm 4 góc xoay, tin cậy ≥ 0,5 thì kết luận (mẫu 2: quyết 97,5 % số lần, sai 0) → gắn cờ **Nghi xoay** + góc gợi ý; không tự xoay. Duyệt trong bộ lọc Nghi xoay: Enter lần 1 = đặt góc đề xuất, R / Shift+R / C chỉnh, Enter lần 2 hoặc [2. Xác nhận và tiếp] = chốt rồi sang trang kế; đã xác nhận ghi `xoay_trang.txt` cột `xac_nhan` (không gắn cờ lại). Bật/tắt + độ tin cậy trong Cấu hình.
- **Đọc Code 128 ở mọi trang** bằng **zxing-cpp** (mẫu 2 scan màu: 63 % index nặng như trang chữ → không lọc theo dung lượng), 1 lần render 150 dpi + 3 lượt đọc trên cùng ảnh. **Index phải đúng 100 %** (là vị trí cắt): mẫu 2 đạt 1.780/1.780, 0 thừa. Index = Code 128 có giá trị khớp mã index trong Cấu hình (mặc định `NAMSAO-TACH`, lệch ≤ 3 ký tự, kể cả mã đọc lỗi checksum); đổi Cấu hình → gán lại theo mã đã lưu, không render. Mã khác (vd `Van-ban-khac`) không gán index, người duyệt tự quyết. **Ô mã để trống → mọi mã Code 128 đều là index.**
- **Thư mục `<thư mục đầu vào>/.sm-dema4/`** chứa `index_and_blank.txt` (nhãn, nguồn sự thật), `xoay_trang.txt` (góc xoay + trang Nghi xoay đã xác nhận), `ketqua_tach.txt` (log tách) và cache: kết quả tự động + chỉ số trắng thô (`dam`, `mau`) + giá trị mã đọc được + phân bố hướng trang tầng 1/2 (~40 byte/trang; đổi ngưỡng / mã index / độ tin cậy không cần render lại) + ảnh nhỏ của trang Index/Trắng/Nghi trắng/đã hiện; mở lại tức thì; khoá theo kích thước + thời điểm sửa PDF.

**Giao diện** (chi tiết: phase 5, 6 — bản chuẩn là prototype)
- Menu **Chức năng ▾** (Đếm trang A4 · Tách văn bản · Ký văn bản (giữ chỗ, màn trống "đang phát triển"); mỗi mục 1 icon nét, không đánh số) + **Cấu hình** + **Trợ giúp ▾** (Kiểm tra cập nhật · Hướng dẫn sử dụng F1 — 2 tab Đếm trang A4 / Tách văn bản, phím tắt cuối tab Tách · Thông tin). Tiêu đề cửa sổ: icon 012-3 + `Đếm trang A4 (Giải pháp số Sao Mai)` / `Tách văn bản PDF (Giải pháp số Sao Mai)`. Màn Đếm giữ nguyên chức năng.
- **Cấu hình** (hộp thoại 2 tab **Đếm trang A4** (hệ số + thước khổ liên tục A7→A0 (không màu, mốc chuẩn vạch chấm) với 7 tay kéo ranh giới + 1 tay ngưỡng A0 lớn (1399 mm); trang A0 lớn quy theo diện tích ÷ A4 (mặc định) hoặc hệ số cố định ≥ hệ số A0 + ô thử cạnh dài, mặc định giữa 2 khổ; DPI/chữ ký vẫn ở màn Đếm) | **Tách PDF**, mô tả chỉ hiện khi đang nhập hoặc ở tooltip ⓘ; mọi popup hẹp ~340 px, nút nhỏ): ngưỡng Trắng + ngưỡng Nghi trắng (2 ô số + thanh khoảng kiểu cắt clip: khung viền trên vùng Nghi trắng caro, kéo 2 mép hoặc dời cả khung) · **bật/tắt quét trang trắng / mã index / chiều trang** (tắt để nhanh, gán tay) + **Độ tin cậy ≥ 0,80** cho chiều trang (0,50–0,95, mô tả ở tooltip) · loại mã index (Code 128 mặc định · QR code · Cụm ký tự — 2 loại sau chưa có chức năng) + giá trị (Code 128 trống = mọi mã) · lưu kết quả vào (thư mục cha) · mẫu tên văn bản · lưu nhãn Tự động/Bằng tay (Ctrl+S). Lưu trong **`sm-dema4.cfg`** cạnh exe (dự phòng thư mục người dùng), kèm độ rộng cột trái, cỡ ảnh, tỷ lệ/chế độ xem, trạng thái ẩn dòng thư mục. Tab Tách PDF: nhãn sát ô, mọi ô thẳng mép trái/phải, dòng cách đều.
- Hàng thư mục: Chọn · Phân tích; **ẩn/hiện được** bằng nút icon nét ở góc phải hàng tab (vùng duyệt cao thêm, nhớ trong cfg); **Phân tích lại = làm lại từ đầu** (bỏ cache + mọi sửa tay; hỏi nếu đã sửa).
- Hai tab **Hồ sơ (y)** | **Văn bản (x)**, cột trái co dãn; vạch tiến độ 2 px dưới hàng tab, chỉ hiện khi chạy; duyệt được trong lúc phân tích nền.
- **Tab Hồ sơ**: bảng `# | Hồ sơ | Trang | Index(+split) | Trắng | VB` (hàng Tất cả = lọc) + lưới gộp mọi trang (`hoso.stt`), ảo hoá ~700k trang; lọc bằng menu thả xuống Tất cả/Index/Trắng/Nghi trắng/Nghi xoay + nút riêng Điểm cắt (= trang Split); sắp theo dung lượng; −/+ và Ctrl+cuộn; chọn kiểu Explorer (kéo khung, Ctrl, Shift, Shift+End/Home, Ctrl+A).
  - Nhãn 2 nhóm nút **[S]** Split | **[I · B]** Index, Trắng (không có nút C — chưa gắn nhãn là nội dung; phím C bỏ nhãn), bật/tắt (bấm lại là huỷ); nút sáng đậm khi cả vùng chọn cùng nhãn, viền khi một phần.
  - **Split** chỉ ở tab này, trên mọi trang Content: phím S hoặc nút ✂ góc trên trái ảnh.
  - **Xoá trang** (Del) mọi trang đang chọn: thành nội dung (bỏ nhãn) + ẩn khỏi lưới, vẫn có trong file tách, văn bản/thông số tính lại ngay + Khôi phục (menu chuột phải chỗ trống hoặc Ctrl+Z); trang đã xoá không còn tốn phân tích/ảnh; bấm đúp phóng to.
  - **Xoay** ↻90° (R) / ↻180 (Shift+R) cho trang đang chọn: hiện ngay, lưu `.sm-dema4/xoay_trang.txt`, áp `/Rotate` khi tách, không sửa file gốc.
  - **Bàn phím kiểu Explorer** (mũi tên, Shift/Ctrl+mũi tên, Ctrl+Space, Home/End, Ctrl+A, Esc, Enter) + **menu chuột phải** (nhãn, xoay, xoá, chọn tất cả) — cả 2 tab.
  - **Duyệt 2 loại Nghi** (chỉ trong bộ lọc Nghi trắng / Nghi xoay; nhóm khác phím giữ nghĩa thường, bấm nhãn góc trên ảnh thì mở bộ lọc đó) bằng thanh hỏi trên lưới, tự sang trang kế tiếp: **Nghi trắng** — Enter/B = Trắng, C = giữ nội dung, I = Index; **Nghi xoay** — `<hs>.<stt> (đề xuất xoay 180°) → [1. Xoay đề xuất] → [2. Xác nhận và tiếp]`, nút Enter sẽ bấm tô xanh; R chỉnh, C về gốc; chỉ rời danh sách khi xác nhận. Menu lọc có vạch ngăn giữa nhãn đã chốt và 2 loại Nghi.
  - **Ctrl+Z / Ctrl+Y** hoàn tác / làm lại **riêng từng nhóm** (Tất cả · Index · Trắng · Nghi trắng · Nghi xoay · Điểm cắt · tab Văn bản), 100 bước mỗi nhóm; trang đã sửa ở nhóm khác thì bỏ qua + báo.
  - Dòng đáy: còn trong view · đã xoá · đã chọn · 2 icon Hoàn tác / Làm lại (tooltip ghi nhóm + bước + phím) · `Tự động lưu index_and_blank.txt` + chấm đỏ nhỏ khi chưa lưu.
- **Tab Văn bản** (không đặt nhãn; chọn trang trong văn bản để **xoay** hoặc **Bỏ khỏi văn bản** = gán Trắng): bảng `# | Tên văn bản | Trang` (✂ cam = mở đầu bằng Split); ‹ › / số + Enter / PgUp PgDn (←→ khi chưa chọn trang) liên tục mọi hồ sơ; xoay ở đây = chốt Nghi xoay; Bỏ khỏi văn bản cũng bỏ cờ Nghi trắng; 3 chế độ Lưới · 1 trang vừa ngang · 2 trang liên tục, luôn cân giữa; −/+ và Ctrl+cuộn.
- Chân màn Tách: `Thông tin: Hồ sơ: x; Văn bản: y; Số trang: z; Quy A4: n` · nút **Tách** (tooltip thư mục ra; **chỉ tách khi Nghi trắng và Nghi xoay rỗng**, còn thì hộp "Chưa tách được" + nút đi duyệt). Xong → popup (hồ sơ, văn bản, trang bỏ, sao nguyên, ghi ra, thời gian; Mở thư mục · Mở log · Đóng; dòng "Thông tin tách thư mục đã được ghi lại") + **tin Telegram** cùng kiểu tin đếm.
- Nút chính xanh lá, rê chuột → xanh dương #0067C0.

## Phases (TDD: mỗi phase viết test trước, đỏ → xanh)
| Phase | Name | Effort | Status |
|-------|------|--------|--------|
| 1 | [Spike và bộ test vàng](./phase-01-spike-va-bo-test-vang.md) | 1–1,5 ngày (+ zxing-cpp, ort, CI Windows) | Pending |
| 2 | [Lõi nhãn trang và dải cắt](./phase-02-loi-nhan-trang-va-dai-cat.md) | 0,5 ngày | Pending |
| 3 | [Phát hiện tự động (đọc dung lượng trước, phân tích nền, cache)](./phase-03-phat-hien-tu-dong.md) | 2,5 ngày (+ mô hình hướng trang 2 tầng) | Pending |
| 4 | [Cắt PDF và lệnh tách](./phase-04-cat-pdf-va-lenh-tach.md) | 0,5–1 ngày | Pending |
| 5 | [Khung màn Tách + tab Hồ sơ + Cấu hình](./phase-05-giao-dien-duyet-va-tach.md) | 4,5 ngày (+ duyệt Nghi trắng/Nghi xoay, hoàn tác theo nhóm) | Pending |
| 6 | [Tab Văn bản + phóng to trang + nút Tách](./phase-06-tab-van-ban-phong-to-va-tach.md) | 3 ngày (+ chặn Tách khi còn trang Nghi) | Pending |
| 7 | [Phát hành](./phase-07-phat-hanh.md) | 0,5 ngày | Pending |

Thứ tự: 1 → 2 → (3 ∥ 4) → 5 → 6 → 7. Tổng ước lượng **~13–14 ngày** (giao diện phase 5 + 6 chiếm ~7,5 ngày vì lưới tự dựng + chọn kiểu Explorer + 3 chế độ xem + duyệt 2 loại Nghi + hoàn tác theo nhóm).

## Acceptance criteria (toàn plan)
- Mẫu 1 `temp/function2/mau`: index 209/209, blank 1/1.
- Mẫu 2 `~/Desktop/mau2` (21.113 trang, nhãn tay `index_and_blank (mau2).txt`): index 1.780/1.780, 0 thừa; Trắng tự động ≥ 99,5 % đúng; trắng thật lọt Content ≤ 1 % (phần mập mờ + trang có mực màu nằm ở Nghi trắng); toàn mẫu ≤ ~8 phút/10 lõi.
- Cắt từ cùng file nhãn: 211 văn bản, số trang + nội dung từng part khớp `mau/pdf_vanban` (b3).
- Hướng trang: mẫu 2 xoay giả 200 trang × 4 chiều ⇒ 2 tầng quyết ≥ 97 % số lần, sai 0.
- GUI: Phân tích → sửa nhãn (chọn/kéo khung, S/I/B + phím C, ✂, xoá/khôi phục, duyệt hết Nghi trắng + Nghi xoay, hoàn tác/làm lại theo nhóm) → xem từng văn bản → Tách (bị chặn khi còn trang Nghi), kết quả khớp CLI; lưu/mở lại giữ nguyên nhãn (gồm split), góc xoay, trang Nghi xoay đã xác nhận, trang Nghi trắng đã giữ nội dung và cấu hình; thao tác khớp prototype.
- Split + Index + Trắng trộn bất kỳ: không sinh văn bản rỗng (test thuộc tính phase 2); hồ sơ không nhãn → file ra giống từng byte file gốc; thư mục ra giữ cấu trúc nguồn.
- 100k trang: sắp theo dung lượng và duyệt được ≤ 10 s sau khi bấm Phân tích; lưới cuộn mượt, RAM không tăng theo số trang đã cuộn; mở lại thư mục đã phân tích → lưới đầy ảnh < 1 s.
- Exe Windows chạy được, không WSL/Python; màn Đếm không đổi hành vi (toàn bộ test cũ xanh).

## Ràng buộc
- Không phá hợp đồng hiện có: CLI `sm-dema4 <thư mục>` (đếm), file `_dem.txt`, tin Telegram đếm.
- Đã sửa sẵn (chưa commit, gộp vào bản phát hành của plan này): tiêu đề cửa sổ `Đếm trang A4 (Giải pháp số Sao Mai)` ở `dema4_rs/src/gui.rs`.
- Dữ liệu mẫu 270 MB nằm trong `temp/` (gitignore) → test vàng là `#[ignore]` + biến môi trường `DEMA4_MAU_TACH`, chạy tay; CI chỉ chạy test đơn vị với PDF sinh trong test.
- `sm-dema4.cfg` không chứa bí mật; không commit file cfg.
- Code comment/tên test mô tả hành vi, không chứa mã phase.

## Bước tiếp theo
1. **Đóng băng plan**: tạo nhánh `feat/tach-van-ban` từ `main`; commit plan + prototype + báo cáo (`docs: kế hoạch tách văn bản tự động`), chưa gộp sửa `gui.rs`.
2. **Phase 1 — spike (1–1,5 ngày)**, mục tiêu gỡ rủi ro build trước khi viết thật:
   - thêm `zxing-cpp` (bundled) + `ort` (liên kết tĩnh), mô hình `dema4_rs/assets/doc_ori_b1.onnx` (6,8 MB, Apache-2.0, ghi NOTICE);
   - test `#[ignore]`: đọc mã 209/209 mẫu 1; cắt PDF 2 cách, chọn 1; hướng trang khớp ONNX Runtime Python;
   - chạy CI trên nhánh bằng `gh workflow run build.yml --ref feat/tach-van-ban` (workflow chỉ tự chạy ở `main`/tag), tải exe Windows chạy thử trên máy sạch (không DLL kèm);
   - cổng quyết định: `ort` không liên kết tĩnh được → `tract`; `zxing-cpp` không build được trên Windows → feature `zxingcpp` của `mupdf-sys`; ghi kết quả vào `reports/`.
3. **Phase 2 → (3 ∥ 4) → 5 → 6 → 7**, TDD từng phase (test đỏ → xanh), 1 commit/phase, CI xanh trước khi sang phase sau; test vàng mẫu 1 + mẫu 2 chạy tay ở cuối phase 3, 4, 6.
4. **Phát hành v0.3.0** (phase 7): gộp sửa tiêu đề `gui.rs`, README, tag → CI tạo Release.

## Dependencies
Không có plan khác đang mở.

## Câu hỏi mở
- Cần thêm hồ sơ scan 2 mặt nhiều trang trắng để chốt ngưỡng trắng (mẫu chỉ có 1 trang trắng) — không chặn triển khai (ngưỡng chỉnh được, GUI cho sửa nhãn).
- ~~Chờ nhãn tay mẫu 2 để chốt chỉ số trắng~~ — đã xong ở Session 29 (chỉ số v3, 1,0 % / 3,0 %).
- ~~Lưu "giữ nội dung" / trang đã xoá~~ — đã chốt 2026-10-28: `index_and_blank.txt` **giữ đúng định dạng cũ** (để dùng tiếp với Python); trạng thái duyệt (giữ nội dung, đã xoá khỏi lưới, cờ đang chỉnh) **lưu trong cache `.sm-dema4/cache/`**, mở lại giữ nguyên trạng thái cuối; Phân tích lại xoá.

## Validation Log

### Session 1 — kiểm chứng API và mẫu
| Chủ đề | Quyết định |
|---|---|
| Luật nhận index | Mọi Code 128; lọc giá trị tuỳ chọn |
| Thư mục ra | `<thư mục nguồn>_vanban` |
| Chạy lại | Ghi đè + xoá part thừa; log ghi đè |
| Màn duyệt | Lưới ảnh đầy đủ |

Verification: 13 claim · 12 đúng · 1 sai: `rxing::Luma8LuminanceSource` không có trong rxing 0.9.3 → dùng `rxing::helpers::detect_in_luma_with_hints` (đã sửa phase 1). Đã xác minh: `tim_file_pdf`, `SuKien`, `chup_giao_dien`, `XANH_COBALT`, mupdf `Page::to_pixmap`, `PdfDocument::{graft_object, insert_page, delete_pages, save_with_options}`; mẫu 211 dòng log = 211 part, 209 nhãn index.

### Session 2–22 — lặp prototype giao diện (2026-10-27)
Gộp các quyết định giao diện; bản cuối nằm ở mục "Quyết định đã chốt" và phase 5–6:
- Ngưỡng trắng, mã index, mẫu tên, chế độ lưu nhãn → hộp **Cấu hình** trên menu; **có** file cấu hình lưu lâu dài.
- Menu "Chức năng" thay thanh tab chức năng; tab duyệt **Hồ sơ | Văn bản**; bỏ thanh tiến độ to, thay vạch 2 px.
- Tab Hồ sơ: lưới gộp mọi hồ sơ, bảng hồ sơ có hàng Tất cả, ẩn KB, 1 nút Sắp xếp có menu, −/+ và Ctrl+cuộn 7 nấc, chọn kiểu Explorer (10 kịch bản đã thử tự động trên prototype), ô ôm ảnh + khe rộng, bỏ kính lúp, Xoá trang/Khôi phục, tooltip cả nút vô hiệu.
- Tab Văn bản: tên theo mẫu Cấu hình, cột "Trang", điều hướng liên tục, 3 chế độ xem cân giữa, Lưới trái → phải, bỏ dòng "VB1 tr.…" và kính lúp.
- Nút chính đổi xanh dương khi rê chuột; Tách có tooltip thư mục ra.

### Session 23 — đánh giá lại sau khi duyệt prototype
- Phase 5 cũ mâu thuẫn prototype 9 chỗ (lưới theo hồ sơ chọn, kính lúp, dòng VB1, lọc "đã đánh dấu", hàng ngưỡng trên màn chính, …) → viết lại.
- Ước lượng 1,5–2 ngày cho toàn bộ GUI không thực tế → tách phase 5 (khung + Hồ sơ + Cấu hình, 3 ngày) và phase 6 (Văn bản + phóng to + Tách, 2 ngày); phát hành thành phase 7.
- Bổ sung: module `cau_hinh.rs` + `sm-dema4.cfg`; bảng ánh xạ prototype → egui 0.37; tiêu chí cuộn 50k trang.

### Session 24 — hiệu năng 100k trang (2026-10-27)
Đo: màn Đếm 100k trang 0,8–3 s, RAM ≤ 240 MB → giữ nguyên. Mẫu thật: render ảnh nhỏ ~10 ms, đọc lại từ đĩa 0,15 ms; đọc dung lượng 896 trang 0,56 s; index 18–94 KB vs nội dung ≥ 229 KB; trắng 453 KB.
| Chủ đề | Quyết định |
|---|---|
| Cache | Trong thư mục đầu vào `.sm-dema4/`, chỉ ảnh nhỏ cần thiết (thư mục trên Shared Drive) — phase 3 |
| Phân tích | 2 bước: đọc dung lượng → phân tích nền; duyệt trong lúc chạy — phase 3, 5 (thứ tự ưu tiên sửa ở session 26) |
| Đọc mã vạch | ~~Chỉ trang < 35 % trung vị~~ → bỏ ở session 26 |
| Thói quen sắp + Shift + xoá | Shift+End/Home, Ctrl+A; trang đã xoá huỷ việc phân tích/ảnh — phase 5 |
| Xoá trang | Bỏ qua trang có nhãn Index/Trắng, báo số trang giữ lại — phase 5 |

### Session 25 — Split, phím tắt, cấu trúc đầu ra (2026-10-27)
| Chủ đề | Quyết định |
|---|---|
| Index ảo | Đổi tên **Split**: nhãn trên trang content, cắt trước trang, giữ trang; ghi `split` trong file nhãn — phase 2, 5 |
| Nơi đặt Split | Chỉ tab Hồ sơ: phím S hoặc ✂ góc trên trái ảnh; tab Văn bản chỉ xem — phase 5, 6 |
| Phím tắt | C (Content, thay N) · I · B · S (thay V); bấm lại là huỷ — phase 5 |
| Phân tích lại | Làm lại từ đầu, bỏ cache + mọi sửa tay; hỏi nếu đã sửa — phase 3, 5 |
| Trạng thái lưu | Chấm đỏ nhỏ góc trên phải khi chưa lưu, thay chữ "● Chưa lưu" — phase 5 |
| Đầu ra | Giữ cấu trúc thư mục nguồn; thư mục cha trong Cấu hình; hồ sơ không nhãn → chép nguyên thành văn bản 1 — phase 4, 5, 6 |
| `ketqua_tach.txt` | Không thêm cột "Mở đầu" |
| Split ở mọi trang Content | Bỏ dải khi đầu > cuối **hoặc** dải toàn trắng (kiểm 1.000.000 tổ hợp, 0 sai) — phase 2, 5 |
| Nút nhãn | Sáng đậm khi cả vùng chọn cùng nhãn, viền khi một phần — phase 5 |
| Tiêu đề cửa sổ | `Đếm trang A4 (Giải pháp số Sao Mai)` (đã sửa trong app) / `Tách văn bản PDF (Giải pháp số Sao Mai)` — phase 5 |
| Dòng chân màn Tách | `Thông tin: Hồ sơ: x; Văn bản: y; Số trang: z; Quy A4: n` (trang của các văn bản sẽ tách) — phase 3, 6 |
| Cập nhật plan theo prototype cuối | Viết lại mục Giao diện trong plan.md; phase 5 thêm kích thước trang vào dữ liệu trang; phase 7 gộp đổi tiêu đề Đếm vào bản phát hành |
| Hoàn tác | Ctrl+Z ở tab Hồ sơ cho gán C/I/B/S, ✂, Xoá, Khôi phục; dòng Thông tin (Số trang, Quy A4) cập nhật tức thì — phase 5, 6 |
| Telegram | Gửi tin mỗi lần tách xong, cùng kiểu tin đếm; popup có dòng công khai "Thông tin tách thư mục đã được ghi lại" — phase 6, 7 |

### Session 26 — đo mẫu 2 (2026-10-28)
Mẫu 2: 234 PDF, 21.113 trang màu; 484 s/10 lõi; đọc mã ~90 % thời gian. Báo cáo `reports/do-mau2-trang-trang-index-261028-0010-report.md`.
| Chủ đề | Quyết định |
|---|---|
| Đọc mã vạch | Mọi trang (bỏ lọc 35 % trung vị: sót 63 % index mẫu 2) — phase 3, 5 |
| Mã index | Mặc định `NAMSAO-TACH`, lệch ≤ 3 ký tự; `Van-ban-khac`/mã khác bỏ qua, người duyệt bằng mắt — phase 3, 5 |
| Trang trắng | ~~Chỉ số v2; dấu đỏ/xanh = trắng~~ → thay ở session 29 |
| Ngưỡng | ~~Tạm 0,10 % / 0,50 %~~ → chốt 1,0 % / 3,0 % ở session 29 |
| Ưu tiên phân tích | Trang đang hiện trên lưới (thay "trang nhẹ trước") — phase 3 |

### Session 27 — đầu vào nhiều tầng thư mục (2026-10-28)
Kiểm lại: quét mọi tầng đã có (dùng `tim_file_pdf`), đầu ra giữ cấu trúc + tầng hồ sơ đã có (phase 4). Bổ sung: ghi rõ ở plan.md + ví dụ mẫu 2; vị trí/đường dẫn `index_and_blank.txt` (gốc đầu vào, tương đối) — phase 2; file không phải PDF không chép; test cây 3 tầng + test vàng mẫu 2 — phase 4; tên hồ sơ trùng ở thư mục khác hiện kèm thư mục cha — phase 5. Sửa dòng cũ "Index = mọi Code 128" (mâu thuẫn session 26).

### Session 28 — lặp prototype lần cuối (2026-10-28)
| Chủ đề | Quyết định |
|---|---|
| Vị trí file | `index_and_blank.txt` + `ketqua_tach.txt` vào `<đầu vào>/.sm-dema4/` cùng cache; Phân tích lại chỉ xoá cache — phase 2–6 (2026-10-28) |
| Cấu hình + popup | 2 tab Đếm / Tách PDF; bớt mô tả (tooltip, gợi ý khi nhập); popup hẹp, nút nhỏ — phase 5 (2026-10-28) |
| Ranh giới khổ | Thước liên tục A7→A0 trong Cấu hình > Đếm, 7 tay kéo (mm, mặc định giữa 2 khổ) + tay ngưỡng A0 lớn 1399 mm (1189–2500); lõi đếm dùng ranh giới này cho khổ + Sát ngưỡng; ghi vào `_dem.txt` khi khác mặc định — phase 5 (2026-10-28) |
| Xoay trang | ↻90°/↻180 ở tab Hồ sơ + tab Văn bản, lưu `xoay_trang.txt`, áp /Rotate trên file đầu ra; tab Văn bản chọn trang để xoay / Bỏ khỏi văn bản; Ctrl+Z chung — phase 2, 4, 5, 6 (2026-10-28) |
| Nút nhãn | Nhóm [S] riêng, nhóm [I · B] riêng; bỏ nút C (giữ phím C) — phase 5 (2026-10-28) |
| Xoá trang | Xoá được mọi trang (bỏ nhãn → nội dung, ẩn); thay quy tắc "bỏ qua trang có nhãn" (session 24) — phase 5 (2026-10-28) |
| Explorer | Bàn phím + menu chuột phải 2 tab; tab Văn bản đổi văn bản bằng PageUp/PageDown (←/→ khi chưa chọn trang) — phase 5, 6 |
| Nhãn 2 lớp | `nhan_tu_dong` + `nhan_tay`, một hàm `sua()` cho mọi thay đổi (hoàn tác, chưa lưu, số liệu cộng dồn) — phase 5 (2026-10-28) |
| Trợ giúp | Menu Trợ giúp: kiểm tra cập nhật qua GitHub Releases (mở trang tải, không tự cài), hướng dẫn F1 (2 tab Đếm trang A4 / Tách văn bản; phím tắt cuối tab Tách), thông tin — phase 5, 7 (2026-10-28) |
| Lớn hơn A0 | Theo diện tích ÷ A4 (mặc định, không thấp hơn hệ số A0) hoặc hệ số cố định ≥ hệ số A0 — phase 5 |
| Kích thước hộp | Cấu hình và Hướng dẫn rộng 560 px, thân cao cố định ~470 px → không đổi kích thước/vị trí khi chuyển tab; popup thông báo ~340 px — phase 5 |
| Ngưỡng trắng | Thanh khoảng 0–10 % (thang căn bậc hai; session 29 đổi từ 0–2 %) kiểu cắt clip: vùng Trắng nền trắng / Nghi trắng caro / Có chữ xám đậm; khung chỉ có viền: kéo mép = đổi 1 ngưỡng, kéo cả khung = dời cả hai giữ độ rộng; đồng bộ với 2 ô số — phase 5 (2026-10-28) |
| Loại mã index | Hộp chọn `Code 128` (mặc định) / `QR code` / `Cụm ký tự` + ô giá trị; QR và Cụm ký tự chỉ giữ chỗ trong cfg, chưa có chức năng (phân tích bỏ đọc mã) — phase 3, 5 (2026-10-28) |
| Bật/tắt quét | Tab Tách PDF nhóm "1. Tự động nhận dạng khi Phân tích": ô bật/tắt Trang trắng, Mã index (mặc định bật) + dòng ước tốc độ; tắt để nhanh hơn, gán tay thay; nhóm "2. Kết quả tách" — phase 3, 5 (2026-10-28) |
| Bố cục tab Tách PDF | Cột nhãn ~104 px, ô kéo tới cùng mép phải, khoảng dòng ~24 px; chú thích thanh ngưỡng chữ 12 px, ô mẫu 18×11 — phase 5 (2026-10-28) |
| Ẩn dòng thư mục | Nút icon nét (vạch + mũi tên) góc phải hàng tab ẩn/hiện dòng "Thư mục hồ sơ PDF · Chọn · Phân tích"; nhớ trong cfg — phase 5 (2026-10-28) |
| Hàng tab | Gắn sự kiện theo id từng hàng tab (lỗi prototype: tab Cấu hình gắn nhầm hàng tab Hướng dẫn) — phase 5 |

### Whole-Plan Consistency Sweep
Đã rà plan.md + 7 phase (grep: kính lúp, VB1, "đã đánh dấu", Xoá khỏi view, thanh tab, KB ↑/↓, hằng số ngưỡng, ô nhập ngưỡng trên màn chính, phase-06-phat-hanh). Đã sửa phase 3 (ngưỡng/mã chỉnh trong Cấu hình, sắp theo dung lượng) và phase 7 (README + `.gitignore` cho `sm-dema4.cfg`). Session 24: rà lại plan.md + phase 3, 5 (ngưỡng 50k → 100k, Xoá trang bỏ qua trang có nhãn, Cấu hình thêm "Đọc mã vạch mọi trang"). Từ khoá cũ chỉ còn trong Validation Log và câu "Không có icon kính lúp" ở phase 6. Session 25: rà plan.md + 7 phase (grep: N · I · B, phím N/V, index ảo, Nội dung + ẩn, `_vanban/<hoso>`, "● Chưa lưu"); đã sửa phase 2, 3, 4, 5, 6. Rà theo prototype cuối (Split, C/I/S/B, nút sáng, ✂, chấm chưa lưu, Phân tích lại, Lưu kết quả vào, cấu trúc đầu ra, dòng Thông tin, tiêu đề, Telegram tách): plan.md + 7 phase khớp; grep N/I/B, phím N/V, index ảo, "sm-dema4 —" không còn ngoài Validation Log. Mâu thuẫn còn lại: 0. Session 28: rà lại theo prototype cuối (Cấu hình 2 tab + thước khổ + Lớn hơn A0, Hướng dẫn 2 tab, Trợ giúp, bàn phím/chuột phải, xoay, PageUp/PageDown): bỏ mục "đọc mã vạch mọi trang" khỏi Cấu hình (prototype không có; luôn đọc mọi trang), gom các dòng quyết định rời rạc vào bảng Session 28. Sau đó cập nhật theo prototype: thanh khoảng ngưỡng (khung viền, caro), loại mã index, bố cục tab Tách PDF, ẩn dòng thư mục — plan.md + phase 3, 5 khớp. Mâu thuẫn còn lại: 0.

### Session 29 — đo với nhãn tay mẫu 2 (2026-10-28)
Nhãn tay `~/Desktop/mau2/index_and_blank (mau2).txt`: 1.780 index, 3.561 trắng. Báo cáo `reports/do-chinh-xac-mau2-voi-nhan-tay-261028-1919-report.md`.
| Chủ đề | Quyết định |
|---|---|
| Đọc mã | **zxing-cpp** thay rxing (rxing sót 4/68 mã); 1 lần render 150 dpi; lượt LocalAverage + mã lỗi checksum → GlobalHistogram → ảnh thu 0,9; mẫu 2: 1.780/1.780, 0 thừa — phase 1, 3, 7 |
| Index | Code 128 khớp mã trong Cấu hình (lệch ≤ 3); đổi Cấu hình thì gán lại theo mã đã lưu; index bắt buộc đúng 100 % — phase 3, 5 |
| Trang trắng | Chỉ số v3 (nét đậm ≥ 60 + 0,3 × mực màu), ngưỡng mặc định 1,0 % / 3,0 %; Trắng 99,85 % đúng, lọt Content 0,95 % — phase 3, 5 |
| Dấu đỏ / viết tay màu nhạt | Vào **Nghi trắng**, người dùng tự phân loại (thay "dấu đỏ = trắng" session 26) — phase 3, 5 |
| Thanh ngưỡng | Thang 0–10 %, mốc 0 · 0,5 · 1 · 2 · 3 · 5 · 10 — phase 5, prototype |

### Session 30 — nhận chiều trang (2026-10-28)
Báo cáo `reports/de-xuat-nhan-chieu-chu-trang-261028-2028-report.md` (cách tự viết, Tesseract) + `reports/do-mo-hinh-nhan-huong-trang-261028-2115-report.md` (mô hình).
| Chủ đề | Quyết định |
|---|---|
| Mô hình | PP-LCNet_x1_0_doc_ori (97,8 % đúng; 100 % khi tin cậy ≥ 0,8) thay cách tự viết (86,5 %) và Tesseract OSD (75 %); chạy bằng `tract-onnx` (khớp ONNX Runtime, 20 ms/trang); exe thêm vài chục MB chấp nhận — phase 1, 3, 7 (đổi sang `ort` ở dòng dưới) |
| Giao diện | Cờ Nghi xoay + bộ lọc + thanh hỏi; Enter / nút / bấm nhãn góc = xoay theo gợi ý rồi chọn trang Nghi xoay kế tiếp; C = bỏ qua; ô bật/tắt Chiều trang trong Cấu hình — phase 5, prototype `#nghi-xoay` |
| Duyệt Nghi trắng | Giống Nghi xoay: thanh hỏi + nút "Trắng?" trên ảnh; Enter/B = Trắng, C = giữ nội dung, I = Index, rồi tự sang trang Nghi trắng kế tiếp — phase 5, prototype `#nghi-trang` |
| Menu lọc | Vạch ngăn giữa Trắng và Nghi trắng (nhãn đã chốt / cờ chờ duyệt) — phase 5 |
| Chặn Tách | Nghi trắng và Nghi xoay phải rỗng mới Tách; còn thì hộp "Chưa tách được" + nút đi duyệt từng loại — phase 6, prototype `#tach-chua-duyet` |
| Thư viện chạy mô hình | **`ort`** (ONNX Runtime, liên kết tĩnh, 8,7 ms/lần) thay `tract` (20,5 ms); `tract` là dự phòng nếu `ort` không build tĩnh được trên CI Windows; gọi qua 1 hàm để đổi được — phase 1, 3, 7 (2026-10-28) |
| Hai tầng hướng trang | Tầng 1 = 1 lần (tc ≥ 0,8); tầng 2 = 4 góc xoay cho trang còn lại (tc ≥ 0,5); mẫu 2: 780/800 quyết, sai 0, +~5 ms/trang bình quân; bỏ OCR Tesseract và bỏ phiếu dòng chữ (không hơn, chậm hơn nhiều) — phase 3 (2026-10-28) |
| Độ tin cậy hướng trang | Ô "Độ tin cậy ≥ 0,80" (0,50–0,95) cạnh Chiều trang trong Cấu hình > Tách PDF, mô tả trong tooltip ⓘ; đổi thì gán lại cờ không render lại — phase 3, 5, prototype `#cau-hinh-tin-cay` (2026-10-28) |
| Xác nhận xoay | Nghi xoay chỉ rời danh sách khi **xác nhận** (nút "2. Xác nhận và tiếp" ở thanh hỏi hoặc Enter lần 2; thanh hỏi `<hs>.<stt> (đề xuất xoay 180°) → [1. Xoay đề xuất] → [2. Xác nhận và tiếp]`, nút Enter sẽ bấm tô xanh + ghi (Enter)); Enter lần 1 = đặt góc gợi ý (không cộng dồn), R chỉnh, C về 0°; xoá trang giữ cờ; I/B chỉ ẩn cờ; xoay ở tab Văn bản = chốt; đã xác nhận (kể cả 0°) lưu `xoay_trang.txt` cột `xac_nhan`, không gắn cờ lại — phase 2, 5, prototype `#nghi-xoay` (2026-10-28) |
| Hoàn tác / làm lại | Bỏ nút Khôi phục ở dòng đáy, thay 2 icon Hoàn tác (Ctrl+Z) / Làm lại (Ctrl+Y, Ctrl+Shift+Z) có tooltip ghi tên bước; Khôi phục trang đã xoá còn ở menu chuột phải chỗ trống — phase 5 (2026-10-28) |
| Hoàn tác theo nhóm | Mỗi nhóm (Tất cả/Index/Trắng/Nghi trắng/Nghi xoay/Điểm cắt/tab Văn bản) có hoàn tác + làm lại riêng; bước lưu trang đổi trước/sau; trang đã sửa tiếp ở nhóm khác thì bỏ qua + báo — phase 5, prototype (2026-10-28) |

### Rà khớp prototype (2026-10-28, sau session 30)
Đã đối chiếu prototype với plan.md + 7 phase: sửa render 150 dpi dùng chung (plan.md), Ctrl+Z → hoàn tác/làm lại theo nhóm + Ctrl+Y, dòng đáy 2 icon, duyệt 2 loại Nghi, chặn Tách, tên nút "1. Xoay đề xuất" / "2. Xác nhận và tiếp" (phase 2, 5, 6), cfg `quet_xoay`/`tin_cay_xoay`, mô hình dữ liệu trang thêm cờ/góc xoay, menu chuột phải, Hướng dẫn (bước 1, bảng nhãn, phím C/Enter/Ctrl+Y); tiêu chí nghiệm thu thêm hướng trang + chặn Tách; ước lượng 11,5 → 13 ngày. Prototype sửa chữ cũ: Hướng dẫn bước 1, phím C, hộp Phân tích lại.
| Phạm vi duyệt Nghi | Thanh hỏi + Enter/B/C/I kiểu duyệt + tự nhảy trang chỉ trong bộ lọc Nghi trắng / Nghi xoay; nhóm khác Enter = Xem lớn, B/I/C như thường; bấm nhãn góc → mở bộ lọc; Ctrl+A → Enter → Enter chốt cả loạt giữ, không hỏi lại — phase 5, prototype `#nghi-trang-tat` (2026-10-28) |
| Icon menu | Chức năng (Đếm trang A4: tờ giấy · Tách văn bản: kéo) và Trợ giúp (Kiểm tra cập nhật: mũi tên vòng · Hướng dẫn: dấu hỏi tròn · Thông tin: chữ i tròn) có icon nét 1 màu kiểu Lucide sát lề trái; ✓ mục đang dùng + phím F1 dời về cuối dòng; bỏ số thứ tự 1. 2. — phase 5 (2026-10-28) |
| Ký văn bản | Thêm mục thứ 3 trong Chức năng, icon bút; chỉ giữ chỗ (màn trống "Chức năng đang phát triển"), ngoài phạm vi plan này; khoảng cách dòng 2 menu thống nhất (đệm 7 px) — phase 5, prototype `#ky-van-ban` (2026-10-28) |
| Hướng dẫn sử dụng | Viết lại cho dễ đọc: 1 lề trái chung, tiêu đề mục gạch dưới, bước số tròn, bảng cột tên 120 px, phím dạng nút, câu ngắn; tab Tách: Các bước (5) · Nhãn trang · Duyệt trang Nghi · Phím tắt (3 nhóm) — phase 5, prototype `#huong-dan` (2026-10-28) |

### Rà khớp prototype lần 2 (2026-10-28, agent độc lập)
Sửa ~30 điểm: plan.md (Chiều trang 2 bước, cấu hình chiều trang + độ tin cậy, `.sm-dema4/` thêm `xoay_trang.txt` + cache ~40 byte/trang, PgUp/PgDn, câu hỏi cũ đã xong, ước lượng 13–14 ngày, nghiệm thu lưu/mở lại); phase 1 (ẩn số (c) `ort`); phase 2 (định dạng `xoay_trang.txt` 0 chỉ với `xac_nhan`, `trang_giu` có góc, `nguyen_ven` khi không xoay); phase 3 (cờ Nghi xoay, `quet_xoay`, cache, Phân tích lại xoá `xoay_trang.txt`, test đổi độ tin cậy); phase 4 (CLI không chặn Nghi, áp `xoay_trang.txt`); phase 5 (Dừng bấm được + khoá Chức năng, Phân tích lại xoá gì, cột nhãn 118 px / 6 dòng, đổi mã index có hiệu lực ngay, bảng khoá cfg + mặc định, menu chuột phải theo bộ lọc, Backspace, Khôi phục theo hồ sơ đang lọc, tooltip hoàn tác ghi nhóm, `TrangMeta` 2 lớp nhãn + cờ + `dam`/`mau` + phân bố hướng ~36 byte, viết lại 17 bước test + tiêu chí); phase 6 (nhóm hoàn tác Văn bản, xoay ở tab Văn bản = chốt, thứ tự kiểm khi Tách, tiêu đề popup, test chặn Tách); phase 7 (README 5 bước, test vàng mẫu 2 + hướng trang). Prototype: Thông tin thư viện, Dừng bấm được + khoá Chức năng, tooltip ‹ › PgUp/PgDn, tooltip Trắng theo số đặt, Tất cả (n) không tính trang xoá, sao nguyên trừ hồ sơ có trang xoay, chú thích 0–10 %, Hướng dẫn "nút − / +".
| Lưu trạng thái duyệt | `index_and_blank.txt` giữ đúng định dạng cũ (`index`/`blank`/`split`) cho Python; trạng thái "giữ nội dung", trang đã xoá khỏi lưới, Nghi xoay đang chỉnh lưu trong cache `.sm-dema4/cache/` (mở lại giữ nguyên trạng thái cuối; PDF đổi hoặc Phân tích lại thì mất) — phase 2, 3, 5 (2026-10-28) |
