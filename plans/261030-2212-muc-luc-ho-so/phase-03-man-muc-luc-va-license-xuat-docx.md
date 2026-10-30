---
phase: 3
title: "Man Muc luc va license xuat DOCX"
status: pending
priority: P1
dependencies: [1, 2]
effort: "4 ngày"
---

# Phase 3: Màn Mục lục + license, xuất DOCX (phát hành được)

## Overview
Màn egui theo prototype đã duyệt, nối lõi phase 2; tính lượt license chức năng `muc_luc`; xuất DOCX. Phát hành bản mới sau phase này.

## Requirements
- `Man::MucLuc` (mục 4 menu Chức năng, tiêu đề cửa sổ "Mục lục hồ sơ (Giải pháp số Sao Mai)"); chưa có dữ liệu → `gioi_thieu::MUC_LUC` 3 bước.
- Dòng Dữ liệu: ô đường dẫn + Chọn (rfd, nhớ thư mục, **đánh thức UI** như fix Tách), Sheet ▾, Header dòng; dòng Mẫu: 4 mẫu nhúng ▾ + Chọn DOCX.
- Cột trái Hồ sơ (`cot_trai`): #, tên (khoá nhóm), số VB, ✓/⚠ (lỗi từ kiểm tra); ô Tìm; Tất cả.
- Tab Ghép biến: bảng biến theo thứ tự xuất hiện trong mẫu, Nguồn ▾ (Cột…/Hằng), ô Hằng, giá trị của hồ sơ đang chọn; Cột gom nhóm ▾; tự khớp khi đổi file/mẫu; Kiểm tra dữ liệu (luồng nền).
- Dòng đáy: Số hồ sơ; Số văn bản; Biến thiếu + **Tạo mục lục** (khoá khi lỗi). Popup tiến độ + kết quả (Mở thư mục).
- Cấu hình › tab Mục lục: thư mục ra (mặc định theo câu trả lời prototype), mẫu tên, Ghi đè; lưu ở `.dema4_saomai.cfg` (khoá mới, mặc định khi thiếu).
- Nhớ lựa chọn gần nhất (file, sheet, mẫu, ràng buộc biến theo mẫu) trong cấu hình.
- **License**: `ChucNang::MucLuc` (`ma()` = "muc_luc"); trạm: cột Đã dùng / Hạn mức Mục lục (thêm cột cuối, không xê dịch cột cũ), `thong_ke_muc_luc` + tin `📑 MỤC LỤC HỒ SƠ (x/y)` (Thông tin máy / ----- / chức năng: Dữ liệu, Mẫu, Số hồ sơ, Số văn bản, DOCX, PDF, Lỗi, Thời gian), tin hết lượt Mục lục. App cũ không gửi `muc_luc` → không ảnh hưởng.
- 1 lượt = 1 lần Tạo có ≥ 1 DOCX ra; hết lượt → popup Liên hệ Sao Mai (đã có, thêm tên chức năng).
- CLI: `--muc-luc <excel> --mau <docx|01..04> [--ra <thư mục>]` (tuỳ chọn, cùng luật license, mã thoát 3 khi hết lượt).

## Related Code Files
- Create: `dema4_rs/src/gui_muc_luc.rs` (+ `gui_muc_luc/` nếu > 400 dòng), `dema4_rs/assets/muc_luc/*.docx`
- Modify: `dema4_rs/src/gui.rs` (Man, menu, tiêu đề), `gioi_thieu.rs`, `cau_hinh.rs`, `gui_tach/hop_cau_hinh.rs` (tab), `license.rs` (ChucNang), `hop_license.rs` (tên), `tram.rs` (`so_lieu_muc_luc`), `tro_giup.rs` (Hướng dẫn tab Mục lục), `main.rs` (CLI)
- Modify: `dema4_rs/apps-script/loi.js`, `license.gs` (cột hạn mức + tin), `test/loi.test.js`; README

## Tests trước
1. `license::quyet_dinh` cho `MucLuc` độc lập Đếm/Tách; trạm Node: `ghi_lan` cn=`muc_luc`, hạn mặc định 10, cột mới, tin `MỤC LỤC HỒ SƠ (x/y)`, hết lượt Mục lục.
2. kittest: chưa chọn → giới thiệu; chọn fixture → cột trái N hồ sơ, bảng biến tự khớp; đổi Nguồn sang Hằng → giá trị cập nhật; Tạo → popup kết quả, DOCX có trên đĩa; hết lượt → popup license, không ghi file.
3. `cau_hinh` khu hồi khoá mới; thiếu khoá → mặc định.
4. Ảnh chụp `chup_man_muc_luc` (giới thiệu, ghép, popup) khớp prototype.

## Implementation Steps
1. Test 1 (đỏ) → license + trạm (deploy trạm trước app, tương thích ngược) → xanh.
2. Test 2–3 (đỏ) → màn, cấu hình, tab Cấu hình → xanh.
3. Ảnh chụp, README + Hướng dẫn, phát hành (tag mới).

## Success Criteria
- [ ] Người dùng tạo được 22 mục lục DOCX từ Excel mẫu trên Windows + macOS.
- [ ] License + Telegram hoạt động với trạm thật.
- [ ] CI xanh, phát hành.

## Risk Assessment
- Bảng biến dài (mẫu 03: 9 cột + 1 hồ sơ) → bảng cuộn, cố định tiêu đề.
- Thêm cột license phải giữ nguyên chỉ số cột cũ (P = mã lần) → Hạn/Đã dùng Mục lục đặt Q/R, test trạm.
