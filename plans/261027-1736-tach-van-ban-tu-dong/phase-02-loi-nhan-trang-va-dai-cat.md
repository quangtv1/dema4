---
phase: 2
title: "Lõi nhãn trang và dải cắt"
status: completed
priority: P1
dependencies: [1]
effort: "0,5 ngày"
---

# Phase 2: Lõi nhãn trang và dải cắt

## Overview
Mô hình dữ liệu thuần (không MuPDF, không GUI): nhãn từng trang, đọc/ghi `index_and_blank.txt`, tính dải văn bản từ nhãn. Đây là phần quyết định đúng/sai kết quả cắt nên phủ test kỹ nhất.

## Requirements
- `Nhan { NoiDung, Index, Trang, Split }` (Trang = blank; Split = cắt trước trang này, trang được giữ — dùng khi lúc scan quên chèn tờ index). Hồ sơ: đường dẫn + số trang + map trang→nhãn (chỉ lưu index/blank/split). Mỗi trang đúng 1 nhãn.
- Đọc `index_and_blank.txt`: dòng `đường_dẫn;trang;index|blank|split` (`split` mới; file cũ chỉ index/blank đọc như trước; b3 Python gặp `split` thì bỏ qua, không lỗi) (khoảng trắng hai bên, chữ hoa/thường loại nhãn); bỏ dòng trống; dòng sai định dạng/trang ngoài [1, tổng] → báo lỗi có số dòng, không bỏ im lặng; trùng trang → nhãn sau thắng + cảnh báo.
- **Xoay trang** (thuộc tính riêng, độc lập nhãn): `xoay: map trang → 90|180|270`, lưu `.sm-dema4/xoay_trang.txt` định dạng `đường_dẫn;trang;0|90|180|270[;xac_nhan]` (không có dòng = 0°, chưa duyệt; **0 chỉ hợp lệ khi có `xac_nhan`**, `0` không kèm `xac_nhan` → lỗi có số dòng); trang Nghi xoay đã xác nhận (nút "2. Xác nhận và tiếp" / Enter lần 2, hoặc xoay ở tab Văn bản) ghi thêm cột `xac_nhan` (vd `…;12;0;xac_nhan` = đã xem, giữ chiều gốc) để không gắn cờ lại; trạng thái duyệt khác (Nghi trắng "giữ nội dung", trang đã xoá khỏi lưới) **không** ghi vào `index_and_blank.txt` (giữ đúng định dạng cũ cho Python) mà lưu trong cache phase 3; đọc/ghi + kiểm tra giá trị hợp lệ như file nhãn. Văn bản mang theo góc xoay từng trang giữ (xem kiểu `VanBan` dưới).
- Ghi: sắp theo file rồi trang tăng dần; ổn định (đọc → ghi → đọc giống hệt).
- Dải văn bản (giữ đúng ngữ nghĩa `create_ranges` của b3):
  - Cắt tại mỗi trang index; trang index không thuộc văn bản nào.
  - Cắt **trước** mỗi trang split; trang split là trang đầu văn bản mới. Split chỉ nằm trên trang content.
  - **Bỏ dải** khi (1) đầu > cuối (split/index sát nhau, split trang 1, index trang cuối) **hoặc** (2) dải chỉ còn trang trắng. Thiếu (2) thì sai ~22 % tổ hợp (vd trắng ở trang 1 + split trang 2 sinh văn bản rỗng). Đã kiểm 1.000.000 tổ hợp ngẫu nhiên so với định nghĩa chuẩn (trang content/split được giữ, mỗi văn bản không rỗng, ngắt tại index/split): 0 sai.
  - Hồ sơ không có nhãn nào **và không có trang xoay** → 1 văn bản cả hồ sơ, đánh dấu `nguyen_ven` để phase 4 chép nguyên file (có trang xoay thì phải ghi lại file).
  - Index liền nhau, index ở trang đầu/cuối → không sinh văn bản rỗng.
  - Trang blank trong dải bị loại; dải chỉ còn toàn blank → **không** sinh part, ghi cảnh báo (b3 cũ sẽ lỗi/ra PDF rỗng).
  - Kết quả: `Vec<VanBan { so: usize (1..), tu, den, bo: Vec<usize>, trang_giu: Vec<(usize /* trang */, u16 /* góc 0|90|180|270 */)> }>`; đánh số part liên tục chỉ trên part thực sự sinh ra.

## Related Code Files
- Modify: `dema4_rs/src/tach_vb.rs`

## Implementation Steps (tests first)
1. Test bảng dải cắt (port & mở rộng case của b3):
   - không nhãn nào → 1 văn bản cả hồ sơ, `nguyen_ven = true`; chỉ có blank → 1 văn bản, `nguyen_ven = false`;
   - split tại 4 trong 10 trang (không index) → [1–3], [4–10]; split + index trộn; split ngay sau index / trang 1 → không thêm văn bản; split liền nhau → văn bản 1 trang; trắng rồi split (dải toàn trắng) → bỏ;
   - **test thuộc tính**: sinh ngẫu nhiên nhãn cho hồ sơ 1–12 trang, so với hàm chuẩn viết thẳng theo định nghĩa (proptest hoặc vòng lặp có hạt cố định);
   - index tại 3, 8 trong 10 trang → [1–2], [4–7], [9–10];
   - index liền nhau 3,4; index trang 1; index trang cuối;
   - blank trong dải được loại; dải toàn blank bị bỏ + cảnh báo; blank trùng index (nhãn sau thắng).
2. Test vàng (không cần PDF): đọc `mau/index_and_blank.txt`, tính dải với số trang 491/405 → so khớp 211 dòng `mau/ketqua_tach.txt` (dải trang, tên part). `#[ignore]` + `DEMA4_MAU_TACH`.
3. Test đọc/ghi: khứ hồi giống hệt (gồm `split`, file xoay có cột `xac_nhan`, dòng `…;0;xac_nhan`; góc lạ như 45 và `0` không kèm `xac_nhan` bị từ chối có số dòng); hồ sơ không nhãn nhưng có trang xoay → `nguyen_ven = false`; dòng lỗi báo đúng số dòng; trang ngoài phạm vi bị từ chối.
4. Hiện thực tới khi xanh.

## Success Criteria
- [x] Mọi case bảng xanh (+ 200.000 tổ hợp ngẫu nhiên khớp hàm chuẩn); test vàng khớp 211/211 dải với b3 (b3 ghi danh sách trang trắng đếm từ 0 trong log — đã đối chiếu: nhãn trang 53, part6 có 28 trang).
- [x] Đọc/ghi khứ hồi ổn định; lỗi định dạng có số dòng. Code: `dema4_rs/src/tach_vb/nhan.rs`.

## Risk Assessment
- Ngữ nghĩa lệch b3 (đặc biệt index liền nhau / cuối file) → khoá bằng test vàng từ `ketqua_tach.txt`.
- Đường dẫn trong file nhãn tương đối → resolve theo **thư mục gốc truyền vào** (app: thư mục đầu vào = cha của `.sm-dema4/`; file kiểu cũ của mẫu 1 `pdf_goc/…`: thư mục chứa file nhãn); test cả hai dạng tuyệt đối/tương đối. App ghi **1 file `<đầu vào>/.sm-dema4/index_and_blank.txt`**, đường dẫn tương đối nhiều tầng có dấu cách (`HOP 462/4766/4766.pdf`), dùng `/` cả trên Windows; đọc chấp nhận cả `\`.
