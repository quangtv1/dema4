---
phase: 4
title: "Gan license GUI va dong lenh"
status: pending
priority: P1
dependencies: [1, 3]
effort: "1.5 ngày"
---

# Phase 4: Gắn license vào giao diện và dòng lệnh

## Overview
Bấm Đếm / Tách quyết định **tức thì** trên bản sao license trong bộ nhớ (không gọi mạng); được thì chạy, không thì popup chặn. Mọi gọi trạm chạy ở luồng ngầm. Áp cho cả CLI.

## Requirements
- Bấm Đếm / Tách không chờ mạng; chỉ nút **Kiểm tra lại** trên popup mới chờ (hiện "Đang kiểm tra…", bỏ cuộc sau 4 s).
- Gọi trạm chỉ ở 3 thời điểm: lúc mở app, sau mỗi lần chạy xong (`ghi_lan` trả luôn trạng thái mới), khi bấm Kiểm tra lại. **Không hẹn giờ làm mới.**
- Popup chặn: **logo Sao Mai** (~64 px, giữa, bấm mở saomaiso.com, dùng lại `tro_giup::logo`) trên cùng; **icon ổ khoá** (nét mảnh, màu đỏ, ~30 px) **bên trái** 2 dòng lý do; icon nằm trên cột nhãn, 2 dòng lý do cân trái thẳng cột giá trị của bảng Mã máy / Điện thoại / Email bên dưới; lý do **một mẫu chung, 2 dòng**: `Lý do: Số lần Đếm còn lại (0)` / `Liên hệ với Sao Mai để được hỗ trợ.` (Đếm / Tách theo nút vừa bấm; máy bị Khoá cũng hiện còn lại 0, không nói là bị khoá; hết lượt offline thêm dòng nhỏ "Chưa kết nối được máy chủ — kết nối mạng rồi bấm Kiểm tra lại."); mã máy + Copy; SĐT **0979824615** (Copy); email **hotro@saomaiso.com** (bấm mở `mailto:` kèm mã máy trong tiêu đề, Copy); web **saomaiso.com**; nút **Kiểm tra lại**, **Đóng**.
- Dòng lệnh: chặn → in thông báo + mã máy, mã thoát 3.
- Hộp Thông tin hiện mã máy (có Copy).

## Architecture
- `license::KiemTra` (chia sẻ qua `Arc`, trạng thái trong `Mutex`):
  - `ban_sao: Option<TrangThaiMay>` (chỉ nhận từ câu trả lời đã kiểm chữ ký); `chay_tu_ban_sao: HanMuc`.
  - `quyet_dinh(cn)` **thuần, không mạng**: có bản sao → `quyet_dinh(Some(ban_sao + chay_tu_ban_sao), offline, cn)`; chưa có → luật offline.
  - `lam_moi_ngam()`: thread `kiem_tra` → thay bản sao, `chay_tu_ban_sao = 0`; có lần offline chờ → `ghi_lan(.., bu_offline = n)` rồi `xoa`. Chỉ gọi lúc mở app.
  - `da_chay(cn)`: tăng `chay_tu_ban_sao` ngay rồi thread `ghi_lan` (có kèm ghi bù offline nếu đang chờ); lỗi → `BoDemOffline::cong(cn)`; được → thay bản sao bằng trạng thái trạm trả về, `chay_tu_ban_sao = 0`.
  - `kiem_tra_lai() -> Receiver<QuyetDinh>` cho nút Kiểm tra lại.
- GUI: `UngDungChinh` giữ `Arc<KiemTra>`; `quyet_dinh` ở đầu `gui.rs::bat_dau_dem` và `nut_tach.rs::bat_dau_tach`; Cho → chạy như cũ, khác → `HopLicense`.
- Hằng liên hệ trong `hop_license.rs`: `SDT = "0979824615"`, `EMAIL = "hotro@saomaiso.com"`, `WEB = gioi_thieu::WEB`.
- Điểm "chạy xong có kết quả": Đếm — `luu_va_thong_bao` khi có ≥ 1 file; Tách — `nhan_ket_qua_tach` khi `kq.dong` không rỗng.
- CLI: đọc đồng bộ 1 lần (timeout 4 s) → `quyet_dinh`; `da_chay` ghi đồng bộ trước khi thoát.
- Test GUI cũ: `UngDungChinh` tạo `KiemTra` với `KhoGia` hạn mức lớn qua `#[cfg(test)]`.

## Related Code Files
- Create: `dema4_rs/src/hop_license.rs`
- Modify: `dema4_rs/src/license.rs`, `dema4_rs/src/gui.rs`, `dema4_rs/src/gui_tach/nut_tach.rs`, `dema4_rs/src/gui_tach.rs`, `dema4_rs/src/main.rs`, `dema4_rs/src/tro_giup.rs`

## Tests trước
1. `may_moi_cho_2_lan_roi_chan`: KhoGia rỗng → `lam_moi_ngam` → lần 1, 2 Cho (`da_chay`), lần 3 HetLuot.
2. `quyet_dinh_khong_goi_mang`: 100 lần `quyet_dinh` → kho bị gọi 0 lần; mỗi lần < 1 ms.
3. `khong_chay_lo_khi_chua_ghi_kip`: KhoGia chậm 1 s; hạn 2, đã dùng 1 → `da_chay` → `quyet_dinh` ngay → HetLuot.
4. `mat_mang_dung_offline`: KhoGia lỗi → 2 Cho, lần 3 HetLuotOffline; kho lành + `lam_moi_ngam` → ghi bù 2, bộ đếm về 0.
5. `khoa_sau_lan_chay_ke`: bản sao còn lượt; kho đặt Khoá; lần kế vẫn Cho; `da_chay` (kho trả Khoá) → lần sau nữa Khoa.
5b. `khong_goi_tram_khi_ranh`: mở app (1 lời gọi), 100 lần `quyet_dinh` không chạy → kho bị gọi đúng 1 lần.
5c. `tram_qua_tai_khong_mat_luot`: kho trả lỗi 503 khi `da_chay` → bộ đếm offline +1; lần `da_chay` sau thành công → ghi kèm bù 1.
6. GUI (kittest): Đếm khi hết lượt → popup có mã máy ngay khung hình đó, không đếm; Kiểm tra lại sau khi kho nâng hạn mức → popup đóng, đếm chạy.
7. GUI: Tách hết lượt → popup, không có luồng tách.
8. CLI: `chay_tach` hết lượt → mã thoát 3, không ghi file.
9. Ảnh chụp popup (`chup_hop`: `het_luot`) — kiểm có logo giữa, icon ổ khoá, 2 dòng lý do, SĐT 0979824615, email hotro@saomaiso.com, mã máy.

## Implementation Steps
1. Test 1–5 (đỏ) → code `KiemTra` → xanh.
2. Test 6–8 (đỏ) → `hop_license.rs`, gắn `quyet_dinh` / `da_chay` / `lam_moi_ngam`, sửa CLI, harness test cũ.
3. `cargo test --release` xanh; chụp ảnh popup.

## Success Criteria
- [ ] Test 1–8 xanh; test cũ không vỡ.
- [ ] Bấm Đếm / Tách chạy ngay kể cả mạng chậm / rút cáp.
- [ ] Hộp Thông tin hiện mã máy.

## Risk Assessment
- Không làm mới định kỳ: Khoá / giảm hạn mức có hiệu lực sau tối đa 1 lần chạy (hoặc khi mở lại app); nâng hạn mức có hiệu lực ngay khi bấm Kiểm tra lại.
- Nhiều cửa sổ app cùng lúc: có thể lố tối đa (số cửa sổ − 1) lần; chấp nhận.
- Ghi lần dùng lỗi sau khi chạy → bộ đếm offline, ghi bù sau; không mất lượt.
