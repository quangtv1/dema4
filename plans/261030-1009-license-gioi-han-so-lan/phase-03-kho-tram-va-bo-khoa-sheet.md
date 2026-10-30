---
phase: 3
title: Kho tram va bo khoa Sheet khoi exe
status: completed
priority: P1
dependencies:
  - 1
  - 2
effort: 1 ngày
---

# Phase 3: Kho trạm + bỏ khoá service account và token khỏi app

## Overview
Phía app: trait `KhoLicense` (bản thật gọi trạm + kiểm chữ ký RSA; bản giả cho test). Gửi thống kê (Ngưỡng trắng + Telegram, cả GUI và CLI) qua trạm. Xoá khoá service account và token Telegram khỏi app — **không đổi tên `telegram.rs`** (giữ `SoLieuTach`, hàm thông tin máy, cờ hiển thị dòng công khai). <!-- Red Team: chỗ gọi bị sót -->

## Requirements
- Câu trả lời chỉ được tin khi: `ok:true`, JSON hợp lệ, chữ ký đúng (khoá công khai nhúng), `ma_may` đúng, `nonce` đúng lời gọi. Mọi thứ khác (HTML, không phải JSON, `ok:false`) = lỗi → luật offline.
- Timeout 8 s cho lời gọi nền (trạm chờ khoá ≤ 2,5 s); nút Kiểm tra lại chờ ≤ 8 s có hiển thị.
- Sau phase này exe không chứa khoá ghi Sheet, không chứa token Telegram.

## Architecture
```rust
pub trait KhoLicense: Send + Sync {
    fn kiem_tra(&self, may: &ThongTinMay) -> Result<TraLoiKy, String>;
    fn ghi_lan(&self, may: &ThongTinMay, lan: &[LanChay]) -> Result<TraLoiKy, String>;   // TraLoiKy gồm da_ghi:[ma_lan]
    fn thong_ke_dem(&self, may: &ThongTinMay, kq: &KetQuaDemTomTat) -> Result<(), String>;
    fn thong_ke_tach(&self, may: &ThongTinMay, so_lieu: &SoLieuTach, nguong: Option<&ThongKeNguong>) -> Result<(), String>;
}
pub struct KhoTram { url: &'static str, khoa_cong_khai: RsaPublicKey }
#[cfg(test)] pub struct KhoGia { … }
```
- Nonce 16 byte từ `getrandom` mỗi lời gọi. ureq 3 tự theo chuyển hướng 302 của Apps Script (đã kiểm `ureq-proto` redirect).
- `KetQuaDemTomTat`: rút từ `KetQuaDem` (số file, trang, quy A4, cảnh báo, lỗi, thời gian) — **không có `SoLieuDem`**.
- Các chỗ gửi Telegram / Sheet phải chuyển (đủ danh sách): `gui.rs:524` (đếm GUI, đặt `co_telegram`), `main.rs:80-82` (đếm CLI), `nut_tach.rs:162-190` (`gui_thong_ke`: tin, CSV, Sheet), test `google_sheet.rs:182`.
- `co_telegram`: = "đã gửi được số liệu lên trạm" (giữ dòng "Thông tin … đã được ghi lại").
- Xoá: `build.rs` phần khoá Sheet, env `SM_DEMA4_SHEET_KEY_JSON` + secret trong `build.yml`, `google_sheet.rs`, trong `telegram.rs` các hàm `doc_cau_hinh`, `gui_tin`, `gui_file`, token nhúng, đọc `telegram.txt`.

## Related Code Files
- Modify: `dema4_rs/src/license.rs` (hoặc `license/kho.rs`), `dema4_rs/src/telegram.rs`, `dema4_rs/src/gui.rs`, `dema4_rs/src/main.rs`, `dema4_rs/src/gui_tach/nut_tach.rs`, `dema4_rs/build.rs`, `.github/workflows/build.yml`, `.gitignore`
- Create: `dema4_rs/assets/license_pub.pem`
- Delete: `dema4_rs/src/google_sheet.rs`

## Tests trước
1. `chu_ky_hop_le`, `chu_ky_sai_bi_tu_choi` (sửa 1 ký tự, sai ma_may, sai nonce), `tra_loi_html_la_loi`, `ok_false_la_loi`.
2. `chu_ky_tu_apps_script_kiem_duoc`: cặp du_lieu / chu_ky thật lấy từ trạm thử (sau phase 5) + khoá công khai thật.
3. `khong_con_khoa_va_token`: mã nguồn không còn `private_key`, không còn mẫu token `\d{8,}:[A-Za-z0-9_-]{30,}`, không đọc `telegram.txt`.
4. `so_lieu_gui_tram_du_truong`: JSON thống kê đếm / tách đủ trường, không có chữ tự do ngoài tên thư mục.
5. `#[ignore]` `kho_tram_that`.

## Implementation Steps
1. Test 1, 3, 4 (đỏ) → `KhoTram`, `KhoGia` → xanh.
2. Chuyển 4 chỗ gọi; xoá khoá / token; `cargo test --release` xanh; CI build không cần secret Sheet.
3. Sau phase 5: test 2, 5.

## Success Criteria
- [ ] Test xanh; `strings` exe không có `BEGIN PRIVATE KEY` / chuỗi dạng token bot.
- [ ] Đếm (GUI + CLI) / Tách xong: tin + CSV tới nhóm, hàng Ngưỡng trắng có trên Sheet (qua trạm).

## Risk Assessment
- Định dạng chữ ký Apps Script ↔ crate `rsa` → test 2 bắt sớm.
- Đổi mẫu tin phải sửa ở trạm (JS) — ghi trong README apps-script.
