---
phase: 3
title: "Man Muc luc va license xuat DOCX"
status: in-progress
priority: P1
dependencies: [1, 2]
effort: "4 ngày"
---

# Phase 3: Màn Sinh mục lục + license, xuất DOCX (phát hành được)

## Overview
Màn egui theo prototype đã duyệt, nối lõi phase 2; tính lượt license chức năng `muc_luc`; xuất DOCX. Phát hành bản mới sau phase này.

## Requirements
- `Man::MucLuc`: mục 4 menu Chức năng **Sinh mục lục**; tiêu đề cửa sổ "Sinh mục lục hồ sơ (Giải pháp số Sao Mai)".
- **Bố cục đúng prototype đã duyệt** (phase 1 › "Bố cục đã duyệt"): 2 dòng đầu vào, tab Mẫu/Kết quả + cảnh báo đầu khung, sidebar kéo được, sửa nguồn biến tại chỗ, Nhóm theo ở chân sidebar, dòng đáy + Sinh mục lục, popup tiến độ / kết quả, hộp Ghép biến.
- Chọn file: rfd, nhớ thư mục, **đánh thức UI** (`request_repaint`) như fix Tách; mỗi lần 1 sheet; đọc bảng + đếm giá trị khác nhau mỗi cột ở luồng nền.
- Tự khớp biến ↔ cột khi đổi file/mẫu (bỏ dấu, chữ thường, `_`/khoảng trắng); kiểm tra dữ liệu chạy tự động sau mỗi thay đổi (không có nút Kiểm tra).
- Phase 3 chưa có xem trước thật (phase 4): khung phải hiển thị bảng văn bản của hồ sơ / danh sách biến của mẫu dạng bảng thường + ghi chú "xem trước bản in ở bản sau"; zoom ẩn.
- Cấu hình › tab **Sinh mục lục**: thư mục ra (trống = `goc_du_lieu()/muc_luc/<tên file dữ liệu không đuôi>/`), mẫu tên, Kèm PDF (khoá khi chưa có PDF), Ghi đè/Bỏ qua; lưu ở `.dema4_saomai.cfg` (khoá mới, mặc định khi thiếu). Popup kết quả có **Mở thư mục** (thư mục ẩn).
- Nhớ lựa chọn gần nhất (file, sheet, dòng tiêu đề, mẫu, nhóm theo, ràng buộc biến theo mẫu) trong cấu hình.
- **License**: `ChucNang::MucLuc` (`ma()` = "muc_luc"); trạm: cột Đã dùng / Hạn mức Mục lục (thêm cột cuối, không xê dịch cột cũ), `thong_ke_muc_luc` + tin `📑 SINH MỤC LỤC (x/y)` (Thông tin máy / ----- / chức năng: Dữ liệu, Mẫu, Số hồ sơ, Số văn bản, DOCX, PDF, Lỗi, Thời gian), tin hết lượt Mục lục. App cũ không gửi `muc_luc` → không ảnh hưởng.
- 1 lượt = 1 lần Tạo có ≥ 1 DOCX ra; hết lượt → popup Liên hệ Sao Mai (đã có, thêm tên chức năng).
- CLI: `--muc-luc <excel> --mau <docx|01..04> [--ra <thư mục>]` (tuỳ chọn, cùng luật license, mã thoát 3 khi hết lượt).

## Related Code Files
- Create: `dema4_rs/src/gui_muc_luc.rs` (+ `gui_muc_luc/` nếu > 400 dòng), `dema4_rs/assets/muc_luc/*.docx`
- Modify: `dema4_rs/src/gui.rs` (Man, menu, tiêu đề), `gioi_thieu.rs`, `cau_hinh.rs`, `gui_tach/hop_cau_hinh.rs` (tab), `license.rs` (ChucNang), `hop_license.rs` (tên), `tram.rs` (`so_lieu_muc_luc`), `tro_giup.rs` (Hướng dẫn tab Mục lục), `main.rs` (CLI)
- Modify: `dema4_rs/apps-script/loi.js`, `license.gs` (cột hạn mức + tin), `test/loi.test.js`; README

## Tests trước
1. `license::quyet_dinh` cho `MucLuc` độc lập Đếm/Tách; trạm Node: `ghi_lan` cn=`muc_luc`, hạn mặc định 10, cột mới, tin `SINH MỤC LỤC (x/y)`, hết lượt Mục lục.
2. kittest: chưa chọn → giới thiệu + sidebar trống; chọn fixture → tab Mẫu liệt kê biến tự khớp, tab Kết quả N hồ sơ; nhập giá trị cố định cho 1 biến → hết cảnh báo chưa ghép; đổi Nhóm theo → số hồ sơ đổi; Sinh → popup kết quả, DOCX có trên đĩa; hết lượt → popup license, không ghi file.
3. `cau_hinh` khu hồi khoá mới; thiếu khoá → mặc định.
4. Ảnh chụp `chup_man_muc_luc` (giới thiệu, ghép, popup) khớp prototype.

## Implementation Steps
1. Test 1 (đỏ) → license + trạm (deploy trạm trước app, tương thích ngược) → xanh.
2. Test 2–3 (đỏ) → màn, cấu hình, tab Cấu hình → xanh.
3. Ảnh chụp, README + Hướng dẫn, phát hành (tag mới).

## Tiến độ (31/10)
- [x] License `muc_luc` (app + trạm, cột Q/R), trạm triển khai @11 **trước** app (bắt buộc: app mới gửi `cn: muc_luc` tới trạm cũ sẽ làm nghẽn ghi bù Đếm/Tách).
- [x] Lõi bổ sung: `khop.rs` (tự khớp như `TextUtil.Normalize`, đếm giá trị khác nhau), `sinh.rs` (sinh hàng loạt, Dừng, ghi đè / bỏ qua), `mo_phong.rs` (trang mô phỏng: đoạn + bảng, viền theo DOCX).
- [x] Màn `gui_muc_luc` đúng bố cục duyệt: 2 dòng đầu vào, tab Mẫu / Kết quả, sidebar kéo được + sửa nguồn tại chỗ, Nhóm theo, định vị biến 2 chiều, zoom, hộp Ghép biến, popup Đang sinh / Đã sinh, license + Telegram.
- [x] Cấu hình › tab Sinh mục lục; nhớ lựa chọn gần nhất (`ml_*`); Hướng dẫn tab Sinh mục lục; README + ảnh.
- [x] Review độc lập (`reports/from-code-reviewer-to-main-sinh-muc-luc-phase3-review-report.md`): đã sửa M1 (đóng app khi đang sinh), M2 (Esc trong hộp), M3 (ràng buộc sai mẫu), M5 (hoàn lượt khi luồng chết) + 3 Low. Chưa sửa: M4 (biến mới thêm vào mẫu không tự khớp khi đã có ràng buộc lưu — cần lưu riêng "bỏ ghép" có chủ ý).
- [ ] CLI `--muc-luc` (tuỳ chọn) — để sau.
- [ ] Phát hành bản mới (chờ chủ sản phẩm duyệt + thử trên Windows).

## Success Criteria
- [ ] Người dùng tạo được 22 mục lục DOCX từ Excel mẫu trên Windows + macOS.
- [ ] License + Telegram hoạt động với trạm thật.
- [ ] CI xanh, phát hành.

## Risk Assessment
- Bảng biến dài (mẫu 03: 9 cột + 1 hồ sơ) → bảng cuộn, cố định tiêu đề.
- Thêm cột license phải giữ nguyên chỉ số cột cũ (P = mã lần) → Hạn/Đã dùng Mục lục đặt Q/R, test trạm.
