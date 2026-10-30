---
phase: 3
title: "Kho tram va bo khoa Sheet khoi exe"
status: pending
priority: P1
dependencies: [1, 2]
effort: "1 ngày"
---

# Phase 3: Kho license qua trạm + bỏ khoá service account khỏi exe

## Overview
Phía app: trait `KhoLicense` với bản thật gọi trạm (HTTPS + kiểm chữ ký RSA) và bản giả cho test. Chuyển ghi thống kê Ngưỡng trắng **và gửi Telegram** sang trạm, rồi **xoá khoá service account và token Telegram khỏi app** (`build.rs`, CI secret, `google_sheet.rs`, phần gửi trong `telegram.rs`, `telegram.txt`).

## Requirements
- Mọi câu trả lời license phải qua kiểm chữ ký bằng khoá công khai nhúng sẵn; sai chữ ký / sai mã máy / sai nonce / thời điểm lệch > 10 phút → coi như không có mạng (luật offline), không tin dữ liệu.
- Timeout 4 s mỗi lời gọi; lỗi trả `Err`, không panic.
- Sau phase này exe không còn chứa khoá ghi Sheet nào và không còn token Telegram.

## Architecture
```rust
pub trait KhoLicense: Send + Sync {
    fn kiem_tra(&self, may: &ThongTinMay) -> Result<TrangThaiMay, String>;                    // trạm tự đăng ký máy mới
    fn ghi_lan(&self, may: &ThongTinMay, cn: ChucNang, bu_offline: u32) -> Result<TrangThaiMay, String>; // 0 = online
}
pub struct KhoTram { url: &'static str, khoa_cong_khai: rsa::RsaPublicKey }
#[cfg(test)] pub struct KhoGia { … }   // trạng thái trong bộ nhớ, đếm số lần bị gọi, có thể chậm / lỗi giả
```
- `KhoTram`: tạo nonce ngẫu nhiên 16 byte / lời gọi; POST JSON; parse `{du_lieu, chu_ky}`; `VerifyingKey::<Sha256>::verify(du_lieu, chu_ky)`; parse `du_lieu`, đối chiếu `ma_may`, `nonce`, `luc`.
- Khoá công khai `dema4_rs/assets/license_pub.pem` (nằm trong repo, công khai được) qua `include_str!`; URL trạm là hằng.
- Thống kê: `gui_thong_ke` (nut_tach.rs) gọi trạm `thong_ke_tach`; Đếm xong (`gui.rs` chỗ `telegram::gui_tin`) gọi trạm `thong_ke_dem`; gửi số liệu có cấu trúc (struct `SoLieuDem` / `SoLieuTach` / `ThongKeNguong` đã có), chạy ngầm, im lặng khi lỗi như hiện nay.
- `telegram.rs` chỉ giữ hàm lấy thông tin máy (`ten_pc`, `dia_chi_mac`, `ip_noi_bo`, `ip_public`) — đổi tên module thành `may.rs` cho đúng nghĩa; xoá `doc_cau_hinh`, `gui_tin`, `gui_file`, token nhúng, hỗ trợ `telegram.txt`.
- Popup "Thông tin … đã được ghi lại" giữ nguyên (dòng công khai việc gửi thống kê).
- Xoá: phần nhúng khoá trong `build.rs`, env `SM_DEMA4_SHEET_KEY_JSON` + secret `SHEET_KEY_JSON` trong `build.yml`, `google_sheet.rs` (JWT, token) nếu không còn ai dùng; giữ `dema4_rs/bi_mat/` trong `.gitignore`.

## Related Code Files
- Create: `dema4_rs/src/license/kho.rs` (hoặc gộp `license.rs`), `dema4_rs/assets/license_pub.pem`
- Modify: `dema4_rs/src/gui_tach/nut_tach.rs` (thống kê qua trạm), `dema4_rs/src/gui.rs` (tin Đếm qua trạm), `dema4_rs/build.rs`, `.github/workflows/build.yml`, `dema4_rs/src/main.rs`, `.gitignore` (bỏ dòng telegram.txt nếu không còn dùng)
- Rename: `dema4_rs/src/telegram.rs` → `dema4_rs/src/may.rs` (chỉ còn thông tin máy)
- Delete: `dema4_rs/src/google_sheet.rs` (khi không còn tham chiếu)

## Tests trước
1. `chu_ky_hop_le_duoc_chap_nhan`: tạo cặp khoá RSA trong test, ký `du_lieu` mẫu → `kiem_tra_tra_loi` trả trạng thái đúng.
2. `chu_ky_sai_bi_tu_choi`: đổi 1 ký tự trong `du_lieu` → Err; chữ ký của máy khác (sai `ma_may`) → Err; sai `nonce` → Err; `luc` lệch 1 giờ → Err.
3. `chu_ky_tu_apps_script_kiem_duoc`: hằng `du_lieu` + `chu_ky` chụp từ trạm thử (phase 2) + khoá công khai thật → Ok (chứng minh Apps Script và Rust tương thích).
4. `kho_gia_hanh_vi`: đăng ký, ghi lần, ghi bù, chậm, lỗi.
5. `#[ignore]` `kho_tram_that`: gọi trạm thật với mã thử, nhận trạng thái hợp lệ.
6. `exe_khong_con_khoa_sheet`: build không đọc `bi_mat/sheet_key.json`; chuỗi `"private_key"` không có trong mã nguồn được build (grep trong test).
7. `khong_con_token_telegram`: mã nguồn không còn mẫu token `\d{8,}:[A-Za-z0-9_-]{30,}` và không đọc `telegram.txt`.
8. `so_lieu_gui_tram_dung_dang`: JSON gửi cho `thong_ke_dem` / `thong_ke_tach` có đủ trường, không có trường chữ tự do.

## Implementation Steps
1. Viết test 1–4, 6 (đỏ) → code `KhoTram` (parse + verify), `KhoGia`.
2. Lấy 1 cặp `du_lieu` / `chu_ky` thật từ trạm thử, viết test 3 → xanh.
3. Chuyển thống kê sang trạm; xoá phần khoá service account; `cargo test --release` xanh; build CI không cần secret Sheet.

## Success Criteria
- [ ] Test 1–4, 6 xanh; test 3 xanh với dữ liệu thật từ Apps Script.
- [ ] `strings dema4_saomai | grep -c "BEGIN PRIVATE KEY"` = 0; không còn chuỗi dạng token bot Telegram.
- [ ] Đếm / Tách xong: tin + CSV vẫn tới nhóm Telegram (qua trạm).
- [ ] Tách xong vẫn có hàng mới ở tab Ngưỡng trắng (qua trạm).

## Risk Assessment
- Khác biệt định dạng chữ ký Apps Script ↔ crate `rsa` → test 3 bắt sớm.
- Đồng hồ máy khách sai > 10 phút → mọi câu trả lời bị từ chối → máy rơi về luật offline. Giảm nhẹ: không so `luc` với đồng hồ máy mà so nonce (đủ chống phát lại); `luc` chỉ để ghi log. Chốt khi code: **dùng nonce, bỏ kiểm `luc`**.
- Đổi mẫu tin: phải sửa ở trạm (JS), không ở app — ghi chú trong `apps-script/README.md`.
