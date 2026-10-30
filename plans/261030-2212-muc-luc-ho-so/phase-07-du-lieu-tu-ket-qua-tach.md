---
phase: 7
title: "Du lieu tu ket qua Tach"
status: pending
priority: P3
dependencies: [2, 3]
effort: "3 ngày"
---

# Phase 7: Dữ liệu từ kết quả Tách (khớp theo thứ tự)

## Overview
Biến cấp dòng (vd `{to_so}`, `{sl_trang}`) lấy từ kết quả Tách thay vì cột Excel: văn bản thứ n của hồ sơ trong Tách ↔ dòng thứ n của nhóm Excel.

## Requirements
- Nguồn mới cho biến cấp dòng: **Từ Tách ▾** = `Tờ số` (dải trang `tu-den` trong PDF gốc, định dạng `01-05`) | `Số trang` (số trang văn bản sau tách) | `Tên file văn bản`.
- Chọn thư mục hồ sơ PDF đã tách (hoặc tự nhận từ Tách gần nhất): đọc `ketqua_tach.txt` trong thư mục dữ liệu ẩn (`thu_muc_an`) — cột: STT, file vào, `tu-den`, file ra, trang bỏ, số trang.
- Khớp hồ sơ: **tên file PDF gốc (không đuôi) = giá trị cột gom nhóm** (so sau chuẩn hoá: bỏ khoảng trắng thừa, không phân biệt hoa thường). Văn bản sắp theo thứ tự trong log.
- Số văn bản Tách ≠ số dòng nhóm → hồ sơ đó **lỗi** (⚠ cột trái, kiểm tra dữ liệu chặn Tạo hoặc cho bỏ qua). Hồ sơ không có trong Tách → lỗi nếu có biến dùng nguồn Tách.
- Không ghi gì vào thư mục đầu vào.

## Related Code Files
- Create: `dema4_rs/src/muc_luc/tu_tach.rs`
- Modify: `ghep.rs` (nguồn `TuTach`), `kiem_tra.rs`, `gui_muc_luc.rs`; dùng `tach_vb::phien::thu_muc_an`, định dạng `tach::noi_dung_log`

## Tests trước
1. `doc_log_tach`: đọc log do `tach::noi_dung_log` sinh (round-trip) → map hồ sơ → [(tu, den, so_trang, file_ra)].
2. `khop_theo_thu_tu`: 2 hồ sơ, 3 + 2 văn bản ↔ Excel 3 + 2 dòng → `{to_so}` = `01-05`, `06-09`…
3. `lech_so_van_ban_bao_loi`: 3 văn bản ↔ 4 dòng → lỗi hồ sơ đó, hồ sơ khác vẫn ghép.
4. `khoa_khop_chuan_hoa`: `HS 01.pdf` ↔ ` hs 01 ` khớp.

## Implementation Steps
1. Test đỏ → `tu_tach.rs` → nối `ghep`/`kiem_tra` → UI chọn nguồn Từ Tách → xanh.

## Success Criteria
- [ ] Hồ sơ đã Tách: mục lục tự có Tờ số / Số trang đúng, không cần nhập Excel cho 2 cột này.
- [ ] Lệch dữ liệu được báo rõ tên hồ sơ + số lượng hai bên.

## Risk Assessment
- "Tờ số" theo cơ quan có thể là tờ (2 trang/tờ) chứ không phải trang → hỏi chủ sản phẩm khi tới phase (câu hỏi mở).
- Log Tách đổi định dạng sau này → test round-trip giữ ràng buộc.

## Câu hỏi mở
- Tờ số = dải trang trong PDF gốc (`tu-den`) hay đánh lại liên tục chỉ trên trang giữ (bỏ index/trắng)?
