---
phase: 6
title: "Tab Văn bản + phóng to trang + nút Tách"
status: pending
priority: P1
dependencies: [4, 5]
effort: "3 ngày"
---

# Phase 6: Tab Văn bản + phóng to trang + nút Tách

## Overview
Xem trước từng văn bản sẽ được tạo (trước khi tách), phóng to 1 trang, và chạy Tách từ GUI. Đặc tả = `prototype-tach-van-ban.html`.

## Requirements

### Tab Văn bản
- **Trái**: bảng `# | Tên văn bản | Trang`; `#` đánh số liên tục mọi hồ sơ; Tên văn bản **đúng mẫu tên trong Cấu hình** (không đuôi .pdf); Trang = số trang giữ lại; tiêu đề dính; vạch ngăn giữa các hồ sơ; bấm hàng để mở; hàng đang xem ra ngoài vùng nhìn → **tự cuộn vào giữa cột**.
- **Thanh điều hướng**: `‹` · ô số `n` / tổng · `›` (mũi tên mảnh, không viền, rê chuột mới đổi nền; không có chữ "Văn bản") · tên văn bản + số trang · bên phải **− / +** (kèm % ở chế độ 1/2 trang) · 3 chế độ xem **Lưới · 1 trang vừa chiều ngang · 2 trang liên tục** (icon, tooltip).
- Điều hướng **liên tục qua mọi văn bản của mọi hồ sơ**: ‹ › , số + Enter (ngoài phạm vi → kẹp), **PageUp / PageDown** (luôn), ← / → **khi chưa chọn trang nào** (đang chọn trang thì phím mũi tên đi trong trang kiểu Explorer như tab Hồ sơ; không chạy khi đang gõ ô số); ‹ › vô hiệu chỉ ở đầu/cuối toàn bộ.
- **Nội dung**: đúng các trang sẽ nằm trong file part (đã bỏ index/trắng, đúng thứ tự — cùng hàm dải cắt phase 2); nhãn ảnh `i/n · gốc tr.X`; dòng đáy `Bỏ trắng: tr.…` nếu có.
- **Lưới** xếp trái → phải, trên → dưới từ góc trái; cỡ ảnh 6 nấc ~90–300 px (−/+ hoặc Ctrl + cuộn).
- **1 trang / 2 trang**: cuộn liên tục, **luôn cân giữa khung**; tỷ lệ 50–200 % so với vừa chiều ngang (−/+ hoặc Ctrl + cuộn); > 100 % thì cuộn ngang, thanh cuộn đặt ở giữa.
- Không có icon kính lúp; **bấm đúp** trang → hộp phóng to.
- **Menu chuột phải** trên trang: Xem lớn (Enter) · Xoay phải 90° (R) · Xoay 180° (Shift+R) · Bỏ khỏi văn bản (Del) · Chọn tất cả (Ctrl+A) · Bỏ chọn (Esc).
- **Chọn trang trong văn bản đang xem** — **dùng chung bộ chọn kiểu Explorer với tab Hồ sơ** (phản hồi lúc nhấn chuột, kéo khung, Ctrl/Shift, bấm lại để bỏ, bấm chỗ trống bỏ chọn); đổi văn bản = bỏ chọn. Bên phải thanh điều hướng, **ngay trái nút zoom**: `x chọn` · icon **Bỏ khỏi văn bản** (Del, tên hiện ở tooltip) · **↻ 90°** (R) · **↻180** (Shift+R) — cùng kiểu icon với tab Hồ sơ — bỏ = gán nhãn Trắng cho trang đó (không có trong file tách; khác "Xoá trang" ở tab Hồ sơ chỉ ẩn khỏi lưới); văn bản còn 0 trang thì biến mất khỏi danh sách; Bỏ khỏi văn bản cũng bỏ cờ Nghi trắng của trang đó. **Xoay ở tab này (nút, R, menu) với trang Nghi xoay = chốt luôn** (đã xem trang to): bỏ cờ, ghi `xac_nhan`. Ctrl+Z / Ctrl+Y hoàn tác / làm lại trong **nhóm riêng "Văn bản"** (không đụng lịch sử các nhóm của tab Hồ sơ). Trang xoay hiện xoay ở cả 3 chế độ xem + hộp phóng to.
- Không đặt Split ở đây (Split đặt ở tab Hồ sơ). Văn bản mở đầu bằng Split: dấu ✂ cam trước tên ở danh sách, dòng "mở đầu bằng Split" trên thanh điều hướng, trang đầu viền cam.
- Sửa nhãn ở tab Hồ sơ → danh sách văn bản tính lại, giữ vị trí nếu còn hợp lệ.

### Hộp phóng to (dùng chung tab Hồ sơ và Văn bản)
- Mở bằng bấm đúp; thanh trên: `‹ <hồ sơ>.<stt> · nhãn (i/n) ›` · `−  100%  +` (50–250 %) · `✕`.
- ‹ › / ←→ chuyển trang theo **thứ tự đang xem** (tab Hồ sơ: theo lọc + sắp hiện tại; tab Văn bản: trang của văn bản); Esc / ✕ / bấm nền tối để đóng; +/− phóng.
- Render ~100–150 dpi theo tỷ lệ, trên luồng nền, có ảnh nhỏ tạm trong lúc chờ.

### Dòng chân màn Tách
- **Điều kiện Tách**: danh sách **Nghi trắng** và **Nghi xoay** phải **rỗng** (đếm trên mọi hồ sơ, bỏ trang đã Xoá khỏi lưới). Còn trang chờ duyệt → không tách, hiện hộp "Chưa tách được": "Còn x trang Nghi trắng và y trang Nghi xoay chưa duyệt. Duyệt hết (Nghi trắng: Enter / B / C / I · Nghi xoay: Xác nhận và tiếp) rồi bấm Tách lại." + nút **Duyệt Nghi trắng** / **Duyệt Nghi xoay** (chỉ hiện loại còn; bấm → về tab Hồ sơ, bỏ lọc hồ sơ, bật bộ lọc tương ứng, chọn sẵn trang đầu) + Đóng. Tooltip nút Tách ghi thêm số còn phải duyệt. Loại quét bị tắt trong Cấu hình thì không sinh cờ nên không chặn. Test: còn 1 cờ ⇒ không ghi file nào, hộp hiện đúng số; duyệt hết ⇒ tách bình thường; Xoá trang có cờ ⇒ không còn chặn.
- Trái nút Tách: `Thông tin: Hồ sơ: x; Văn bản: y; Số trang: z; Quy A4: n` (số in đậm, định dạng số kiểu Việt Nam như màn Đếm). Số trang = trang nằm trong các văn bản sẽ tách (đã bỏ index/trắng); Quy A4 tính trên đúng các trang đó. Cập nhật **ngay** khi gán I/B/S/C hoặc Ctrl+Z (tính lại từ bảng nhãn, cộng dồn theo hồ sơ để không quét lại 100k trang mỗi lần); trống khi chưa phân tích. Vd mẫu: gán Index 1 trang A4 → Số trang −1, Quy A4 −1; trang A3 → Quy A4 −2.

### Nút Tách
- Góc dưới phải, cách mép dưới; tooltip `Ghi ra: <thư mục ra> (giữ cấu trúc thư mục gốc)` + khi còn trang Nghi thêm " · Phải duyệt hết Nghi trắng (x) và Nghi xoay (y) trước khi tách"; vô hiệu khi chưa phân tích/đang chạy.
- **Thứ tự kiểm khi bấm Tách**: (1) còn trang Nghi → hộp "Chưa tách được" (nút theo thứ tự **Đóng · Duyệt Nghi xoay · Duyệt Nghi trắng** (xanh), chỉ hiện nút của loại còn; bấm Duyệt → về tab Hồ sơ, bỏ lọc hồ sơ, bật bộ lọc, chọn sẵn trang đầu) và dừng; (2) "Chưa lưu" → hỏi Lưu / Bỏ / Huỷ; (3) tách.
- Chạy phase 4 nền, tiến độ trên vạch mảnh dưới tab; xong → popup tiêu đề **"Đã tách xong `<tên thư mục>`"**: hồ sơ · văn bản · trang bỏ (index, trắng) · sao nguyên (số hồ sơ không có nhãn **và không có trang xoay**, chép nguyên file) · ghi ra · thời gian; nút **Mở thư mục** (thư mục ra) **· Mở log** (`<đầu vào>/.sm-dema4/ketqua_tach.txt`) **· Đóng** (3 nút cùng cỡ như popup màn Đếm); dòng nhỏ cuối popup "Thông tin tách thư mục đã được ghi lại" (công khai việc gửi thống kê, như popup Đếm).
- **Gửi Telegram mỗi lần tách xong** (luồng nền, im lặng khi lỗi mạng; chỉ khi có `telegram.txt`/cấu hình như màn Đếm), cùng kiểu với tin Đếm:
  ```
  Tên PC: <b>…</b>; Địa chỉ Mac: <b>…</b>
  - Thư mục tách: <b>…</b>;
  - Hồ sơ: <b>n</b>; Văn bản: <b>n</b>; Số trang: <b>n</b>; Bỏ index: <b>n</b>; Bỏ trắng: <b>n</b>; Split: <b>n</b>; Sao nguyên: <b>n</b>; Hồ sơ lỗi: <b>n</b>
  - Thời điểm tách: <b>dd.mm.yyyy, hh:mm:ss</b>  (m:ss)
  ```
  Dùng lại `telegram::gui_tin`, giới hạn độ dài và mã hoá HTML sẵn có; thêm hàm soạn tin tách cạnh `soan_tin` + test chuỗi như test tin Đếm. Tách bị dừng giữa chừng vẫn gửi (số liệu phần đã làm).

## Architecture (egui 0.37)
| Prototype | Hiện thực egui |
|---|---|
| Danh sách văn bản dài | `TableBuilder` + `scroll_to_row(chỉ_số, Some(Align::Center))` khi ra ngoài vùng nhìn |
| 1 / 2 trang cuộn liên tục | `ScrollArea::both()`; bề ngang trang = bề ngang khung × tỷ lệ (2 trang: /2); khung hẹp hơn nội dung → căn giữa bằng lề tính tay; > 100 % đặt `scroll_offset.x` ở giữa sau khi đổi tỷ lệ |
| Render trang lớn | Cùng hàng đợi render của phase 5, khoá cache theo (trang, cỡ px) |
| Hộp phóng to | `egui::Modal`; phím xử lý trong modal khi đang mở |

## Related Code Files
- Modify: `dema4_rs/src/gui_tach.rs` (tab Văn bản, hộp phóng to, nút Tách, popup)
- Modify: `dema4_rs/src/telegram.rs` (soạn tin thống kê tách)
- Dùng: `dema4_rs/src/tach_vb.rs` (dải cắt phase 2, cắt PDF phase 4)

## Implementation Steps (tests first)
0. Bàn phím + menu chuột phải tab Văn bản: chưa chọn → ←/→ đổi văn bản; có chọn → ←/→/↑/↓, Shift/Ctrl như tab Hồ sơ; PageUp/PageDown luôn đổi văn bản; Enter xem lớn; menu phải: Xem lớn · Xoay 90°/180° · Bỏ khỏi văn bản · Chọn tất cả · Bỏ chọn.
1. Mô hình điều hướng (thuần): danh sách liên tục mọi văn bản; ‹ › qua ranh giới hồ sơ; số + Enter kẹp phạm vi; đổi nhãn → giữ vị trí hợp lệ.
2. Trang hiển thị của văn bản = trang của part sẽ ghi (so với hàm phase 4 trên mẫu nhỏ).
3. Thu phóng: nấc cỡ lưới; tỷ lệ 1/2 trang; tính lề căn giữa và `scroll_offset.x` giữa khi > 100 %.
4. Hộp phóng to: thứ tự theo lọc/sắp đang hiện; Esc đóng; ‹ › vô hiệu ở đầu/cuối.
5. Tách: còn ≥ 1 trang Nghi trắng / Nghi xoay ⇒ không ghi file nào, hộp "Chưa tách được" đúng số + đúng nút; bấm Duyệt ⇒ đúng bộ lọc, chọn sẵn trang đầu; trang Nghi đã Xoá khỏi lưới ⇒ không chặn; quét bị tắt ⇒ không có cờ nên không chặn; tooltip nút Tách ghi số còn phải duyệt; kiểm Nghi trước rồi mới hỏi Chưa lưu; hỏi khi Chưa lưu; nút vô hiệu đúng lúc; popup tiêu đề + số liệu đúng (sao nguyên không tính hồ sơ có trang xoay); tin Telegram tách đúng mẫu (test chuỗi cố định, không gọi mạng).
5b. Tab Văn bản: xoay trang Nghi xoay ⇒ bỏ cờ + `xac_nhan`; Bỏ khỏi văn bản trang Nghi trắng ⇒ Trắng + bỏ cờ; văn bản còn 0 trang ⇒ biến mất, vị trí nhảy văn bản kề; Ctrl+Z ở tab này chỉ đảo bước nhóm Văn bản; tooltip ‹ › ghi PgUp / PgDn.
6. egui_kittest + chụp giao diện `#[ignore]`: tab Văn bản 3 chế độ (50 %, 75 % 2 trang, 150 %), hộp phóng to, popup xong.

## Success Criteria
- [ ] Duyệt hết 211 văn bản mẫu bằng ‹ › / bàn phím, tên đúng mẫu Cấu hình.
- [ ] 1/2 trang luôn cân giữa ở mọi tỷ lệ; Lưới xếp trái → phải.
- [ ] Phân tích → sửa nhãn → duyệt hết Nghi trắng / Nghi xoay → xem văn bản → Tách mẫu chỉ trong GUI; kết quả khớp phase 4.
- [ ] Tách bị chặn đúng khi còn trang Nghi; không file nào được ghi trong trường hợp đó.

## Risk Assessment
- Render trang lớn tốn RAM (A4 150 dpi ≈ 8 MB RGBA) → cache trang lớn riêng, giới hạn ~20 trang.
- Căn giữa + cuộn ngang trong egui phải tự tính → có test cho hàm tính lề/offset.
