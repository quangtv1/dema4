---
phase: 3
title: "Gan license GUI va dong lenh"
status: pending
priority: P1
dependencies: [1, 2]
effort: "1.5 ngày"
---

# Phase 3: Gắn license vào giao diện và dòng lệnh

## Overview
Bấm Đếm / Tách quyết định **tức thì** trên bản sao license trong bộ nhớ (không gọi mạng); được thì chạy, không thì popup chặn. Mọi gọi mạng (đọc, đăng ký, ghi nhật ký, ghi bù) chạy ở luồng ngầm. Áp cho cả CLI.

## Requirements
- Bấm Đếm / Tách không chờ mạng; chỉ nút **Kiểm tra lại** trên popup mới chờ (hiện "Đang kiểm tra…", bỏ cuộc sau 4 s).
- Bản sao license làm mới ngầm: lúc mở app, mỗi 5 phút, sau mỗi lần ghi nhật ký.
- Popup chặn: lý do (hết lượt Đếm / Tách, máy bị khoá, hết 2 lần offline), mã máy + nút Copy, saomaiso.com, SĐT, email, nút **Kiểm tra lại** và **Đóng**.
- Lần khởi động có mạng đầu tiên: đăng ký máy (nếu chưa có) và ghi bù lần offline đang chờ — chạy nền, im lặng khi lỗi.
- Dòng lệnh: chặn → in thông báo + mã máy, mã thoát 3.

## Architecture
- `license::KiemTra` (chia sẻ qua `Arc`, trạng thái trong `Mutex`):
  - `ban_sao: Option<(TrangThaiMay, Instant)>` — dòng license đọc được gần nhất; `chay_tu_ban_sao: HanMuc` — số lần chạy kể từ lúc đọc (chưa thấy trên Sheet).
  - `quyet_dinh(cn) -> QuyetDinh` **thuần, không mạng**: có bản sao → `quyet_dinh(Some(ban_sao + chay_tu_ban_sao), offline, cn)`; chưa có → luật offline.
  - `lam_moi_ngam()`: thread `doc` (timeout 4 s); chưa có dòng → `dang_ky`; được → thay bản sao, `chay_tu_ban_sao = 0`; ghi bù offline nếu có rồi `xoa`. Gọi lúc mở app, mỗi 5 phút (bộ hẹn giờ kiểm tra ở `ve()`), sau mỗi `da_chay`.
  - `da_chay(cn)`: tăng `chay_tu_ban_sao` ngay (đồng bộ) rồi thread `ghi_lan`; lỗi → `BoDemOffline::cong(cn)`; xong → `lam_moi_ngam()`.
  - `kiem_tra_lai() -> Receiver<QuyetDinh>`: cho nút Kiểm tra lại — đọc ngay, trả kết quả qua kênh.
- GUI: `UngDungChinh` giữ `Arc<KiemTra>`:
  - Đếm (`gui.rs::bat_dau_dem`) và Tách (`nut_tach.rs::bat_dau_tach`): gọi `quyet_dinh` đầu hàm → Cho thì chạy như cũ, khác thì mở `HopLicense` (không có bước chờ).
  - Điểm "chạy xong có kết quả": Đếm — `luu_va_thong_bao` khi `kq` có ≥ 1 file; Tách — `nhan_ket_qua_tach` khi `kq.dong` không rỗng. Gọi `da_chay` ở luồng nền.
- Module UI mới `src/hop_license.rs` (popup, dùng `gui_tach::khung_hop`).
- CLI (`main.rs`): không có luồng ngầm sống lâu → đọc đồng bộ 1 lần (timeout 4 s) rồi `quyet_dinh`; `da_chay` ghi đồng bộ trước khi thoát.
- Test: `KiemTra` nhận `KhoGia` qua `#[cfg(test)]` constructor; harness GUI cũ tạo `UngDungChinh` với kho giả hạn mức lớn để không vỡ test hiện có.
- Hằng liên hệ (`SDT`, `EMAIL`, `WEB = gioi_thieu::WEB`) đặt trong `hop_license.rs`.

## Related Code Files
- Create: `dema4_rs/src/hop_license.rs`
- Modify: `dema4_rs/src/license.rs`, `dema4_rs/src/gui.rs` (Đếm, khởi động, giữ `KiemTra`), `dema4_rs/src/gui_tach/nut_tach.rs` (Tách), `dema4_rs/src/gui_tach.rs` (truyền `KiemTra`), `dema4_rs/src/main.rs` (CLI), `dema4_rs/src/tro_giup.rs` (Thông tin: hiện mã máy)

## Tests trước
1. `may_moi_dang_ky_va_cho_2_lan`: KhoGia rỗng → `lam_moi_ngam` (chờ xong) → lần 1, 2 Cho (mỗi lần `da_chay`), lần 3 HetLuot; KhoGia có dòng đăng ký.
2. `quyet_dinh_khong_goi_mang`: KhoGia đếm số lần bị gọi; 100 lần `quyet_dinh` → 0 lần gọi kho; mỗi lần < 1 ms.
3. `khong_chay_lo_khi_chua_ghi_kip`: KhoGia làm chậm `ghi_lan` 1 s; hạn 2, đã dùng 1 → `da_chay` → `quyet_dinh` ngay sau → HetLuot (nhờ `chay_tu_ban_sao`).
4. `mat_mang_dung_offline`: KhoGia trả Err → 2 lần Cho, lần 3 HetLuotOffline; kho lành + `lam_moi_ngam` → nhật ký 2 dòng bù, bộ đếm về 0.
5. `khoa_chan_ngay_sau_lam_moi`: bản sao còn lượt; chủ Sheet đặt Khoá; `lam_moi_ngam` → Khoa.
6. GUI (kittest): bấm Đếm khi hết lượt → popup có mã máy ngay trong khung hình đó, không bắt đầu đếm; bấm Kiểm tra lại sau khi kho giả nâng hạn mức → popup đóng, đếm chạy.
7. GUI: Tách hết lượt → popup, không có luồng tách.
8. CLI: `chay_tach` với kho hết lượt → mã thoát 3, không ghi file nào.
9. Ảnh chụp popup (`chup_hop`: `het_luot`).

## Implementation Steps
1. Viết test 1–5 (logic `KiemTra` với `KhoGia`), đỏ → code `KiemTra` → xanh.
2. Viết test 6–8, đỏ.
3. Gắn `quyet_dinh` đầu `bat_dau_dem` / `bat_dau_tach`; thêm `hop_license.rs`; gắn `da_chay` ở điểm "xong có kết quả"; gọi `lam_moi_ngam` lúc mở app + hẹn giờ 5 phút.
4. Sửa CLI; cập nhật harness test cũ dùng kho giả hạn mức lớn.
5. Toàn bộ `cargo test --release` xanh; chụp ảnh popup, so với prototype (nếu có).

## Success Criteria
- [ ] Test 1–8 xanh; test cũ không vỡ.
- [ ] Bấm Đếm / Tách bắt đầu chạy ngay, kể cả khi mạng chậm / rút cáp (chỉ Kiểm tra lại mới chờ ≤ 4 s).
- [ ] Hộp Thông tin hiện mã máy (có Copy).

## Risk Assessment
- Bản sao cũ tối đa 5 phút: Khoá máy / giảm hạn mức có hiệu lực chậm ≤ 5 phút (đã chấp nhận); nâng hạn mức có hiệu lực ngay khi khách bấm Kiểm tra lại.
- Mở nhiều cửa sổ app cùng lúc trên 1 máy: mỗi cửa sổ có bản sao riêng → có thể lố tối đa (số cửa sổ − 1) lần; chấp nhận (hiếm).
- Ghi `da_chay` thất bại sau khi đã chạy (mạng rớt) → rơi về bộ đếm offline, ghi bù sau; không mất lượt đếm.
- Test GUI hiện có gọi `bat_dau_dem` / `bam_tach` trực tiếp → cần đường tắt test (kho giả cho phép) để không phải đổi hết test.
