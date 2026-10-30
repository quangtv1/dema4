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
Bấm Đếm / Tách: quyết định **tức thì** trên câu trả lời ký gần nhất (đọc từ đĩa) + sổ lượt; được thì **đặt chỗ** rồi chạy; xong có kết quả thì ghi lên trạm ở luồng nền. Áp cho GUI và CLI (cả `--khong-ghi`).

## Requirements
- Bấm Đếm / Tách không chờ mạng; chỉ **Kiểm tra lại** chờ (≤ 8 s, hiện "Đang kiểm tra…").
- Gọi trạm chỉ ở: mở app (`kiem_tra` + ghi bù lần chờ), sau mỗi lần chạy xong, Kiểm tra lại. **Không hẹn giờ làm mới.**
- **Một luồng gửi duy nhất** (hàng việc trong `KiemTra`): không có 2 lời gọi trạm cùng lúc từ 1 app; câu trả lời chỉ xoá đúng các `ma_lan` trạm báo đã ghi. <!-- Red Team: tranh chấp luồng -->
- **Có kết quả là tính** (kể cả Dừng hoặc lỗi giữa chừng): Đếm có ≥ 1 file đếm được, Tách có ≥ 1 văn bản ghi ra; không có gì → hoàn lượt. <!-- Quyết định chủ sản phẩm 2026-10-30 -->
- Popup chặn theo prototype `#het-luot`: logo Sao Mai giữa (bấm mở saomaiso.com), icon ổ khoá đỏ bên trái 2 dòng lý do `Lý do: Số lần Đếm còn lại (0)` / `Liên hệ với Sao Mai để được hỗ trợ.` (Khoá cũng hiện còn lại 0; offline thêm dòng "Chưa kết nối được máy chủ — kết nối mạng rồi bấm Kiểm tra lại."), mã máy, SĐT 0979824615, email hotro@saomaiso.com (mailto kèm mã máy), saomaiso.com — mỗi dòng có Copy; nút Kiểm tra lại, Đóng.
- CLI: chặn → in lý do + mã máy, mã thoát 3. Hộp Thông tin hiện mã máy.

## Architecture
- `license::KiemTra` (`Arc`): giữ `Arc<dyn KhoLicense>`, `SoLuot`, phần cố định của máy (mã máy, tên máy, MAC — đọc 1 lần lúc mở app), 1 thread gửi + kênh việc. **IP nội bộ / IP public đọc lại mỗi lần thread gửi chuẩn bị lời gọi** (máy chuyển mạng khi app đang mở vẫn gửi IP đúng; IP public chờ tối đa 4 s, chạy ngầm nên không ảnh hưởng thao tác).
  - `quyet_dinh(cn)` thuần.
  - `bat_dau(cn) -> Option<LanChay>`: quyết định + `SoLuot::dat_cho` (ghi đĩa) **trước khi** chạy.
  - `ket_thuc(lan, co_ket_qua)`: `SoLuot::xong`; có kết quả → đẩy việc `ghi_lan(cho_ghi())` vào thread gửi.
  - Thread gửi: `ghi_lan` → `luu_tra_loi` + `da_xac_nhan(da_ghi)`; lỗi → giữ nguyên, lần sau gửi lại (trạm chống trùng nhờ `ma_lan`).
  - `mo_app()`: đẩy `kiem_tra` + ghi bù; `kiem_tra_lai() -> Receiver<QuyetDinh>`.
- **Nơi gắn** (đúng chủ sở hữu hàm): `KiemTra` truyền vào `UngDung` (Đếm: `bat_dau_dem` gui.rs:685, `luu_va_thong_bao` gui.rs:837) và `ManTach` (Tách: `bat_dau_tach`, `nhan_ket_qua_tach` trong nut_tach.rs), tạo ở `gui::chay()`; `chay_tach` / `chay_dong_lenh` (main.rs) nhận tham số `&KiemTra`, tạo ở `main()`. <!-- Red Team: đặt sai chỗ -->
- `Default` của `UngDung` / `ManTach` / `UngDungChinh` dưới `#[cfg(test)]` dùng `KhoGia` hạn mức lớn + `SoLuot` trong thư mục tạm → `cargo test` không gọi trạm thật, không đụng sổ lượt thật. Test CLI (`main.rs` tests) truyền `KiemTra` giả.
- CLI `--khong-ghi`: vẫn đặt chỗ + tính khi in kết quả. CLI ghi lên trạm đồng bộ trước khi thoát; Ctrl+C → đặt chỗ còn trên đĩa, lần chạy sau tính.

## Related Code Files
- Create: `dema4_rs/src/hop_license.rs`
- Modify: `dema4_rs/src/license.rs`, `dema4_rs/src/gui.rs`, `dema4_rs/src/gui_tach.rs`, `dema4_rs/src/gui_tach/nut_tach.rs`, `dema4_rs/src/main.rs`, `dema4_rs/src/tro_giup.rs`

## Tests trước
1. `may_moi_cho_2_lan_roi_chan`.
2. `quyet_dinh_khong_goi_mang` (100 lần → kho gọi 0 lần, < 1 ms / lần).
3. `thoat_giua_chung_van_bi_tinh`: `bat_dau` rồi bỏ (không `ket_thuc`) → KiemTra mới đọc sổ → lần đó trong `cho_ghi`, quyết định tính nó.
4. `dung_khong_co_ket_qua_hoan_luot` / `dung_co_ket_qua_tinh_luot`.
5. `timeout_khong_dem_trung`: KhoGia ghi xong nhưng trả lỗi (giả timeout) → gửi lại cùng `ma_lan` → đã dùng chỉ +1.
6. `mot_luong_gui`: 3 việc liên tiếp → kho thấy tối đa 1 lời gọi cùng lúc, thứ tự đúng.
6b. `ip_doc_lai_moi_loi_goi`: hàm lấy IP giả trả "10.0.0.1" rồi đổi "10.0.0.2" → 2 lời gọi liên tiếp gửi 2 IP khác nhau.
7. `khoa_sau_lan_chay_ke`; `khoa_offline_van_khoa` (trả lời ký Khoá lưu đĩa, kho lỗi → chặn).
8. GUI (kittest): Đếm hết lượt → popup đúng nội dung; Kiểm tra lại sau khi kho nâng hạn → đếm chạy. Tách hết lượt → popup, không có luồng tách.
9. CLI: `chay_tach` hết lượt → mã 3, không ghi file; `chay_dong_lenh --khong-ghi` 3 lần với hạn 2 → lần 3 mã 3.
10. Ảnh chụp popup (`chup_hop`: `het_luot`, `het_luot_offline`) khớp prototype.

## Implementation Steps
1. Test 1–7 (đỏ) → `KiemTra` → xanh.
2. Test 8–9 (đỏ) → `hop_license.rs`, gắn vào `UngDung` / `ManTach` / CLI, `Default` test → xanh; test cũ không vỡ.
3. Ảnh chụp popup.

## Success Criteria
- [ ] Test 1–9 xanh, test cũ xanh, `cargo test` không có lưu lượng mạng tới trạm.
- [ ] Bấm Đếm / Tách chạy ngay kể cả rút cáp.

## Risk Assessment
- Nhiều cửa sổ app: sổ lượt chung trên đĩa (khoá file) nên không lố; mỗi cửa sổ 1 thread gửi, trạm chống trùng.
- Khoá / giảm hạn mức có hiệu lực sau tối đa 1 lần chạy (hoặc khi mở lại app).
