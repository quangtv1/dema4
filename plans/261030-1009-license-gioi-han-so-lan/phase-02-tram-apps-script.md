---
phase: 2
title: "Tram Apps Script"
status: pending
priority: P1
dependencies: []
effort: "1.5 ngày"
---

# Phase 2: Trạm Apps Script (bên duy nhất ghi Sheet)

## Overview
Web app Google Apps Script gắn với spreadsheet Monitor_dema4, chạy bằng quyền tài khoản Sao Mai ("Execute as: Me", "Who has access: Anyone"). App gọi qua HTTPS POST; trạm đọc/ghi tab `license`, `license_nhat_ky`, `Ngưỡng trắng`, tự tạo tab + tab mô tả khi chưa có, và **ký RSA** mọi câu trả lời license. Mã nguồn trạm nằm trong repo (công khai được — bí mật duy nhất là khoá RSA trong Script Properties).

## Requirements
- Hành động: `kiem_tra` (đọc / tự đăng ký máy mới), `ghi_lan` (thêm nhật ký, trả trạng thái mới), `ghi_bu` (n lần offline), `thong_ke` (thêm 1 hàng Ngưỡng trắng).
- **Số lần dùng do trạm đếm** từ nhật ký; app không gửi con số "đã dùng".
- Câu trả lời license ký bằng RSA-SHA256 (PKCS#1 v1.5), kèm mã máy + thời điểm + nonce app gửi.
- An toàn đồng thời: `LockService.getScriptLock()` khi ghi.
- Kiểm tra đầu vào: mã máy đúng định dạng `SM-XXXX-XXXX-XXXX`, chức năng ∈ {Đếm, Tách}, `n` ≤ 10 với `ghi_bu`, chuỗi giới hạn độ dài.

## Architecture
```
POST {url}  body JSON: { hanh_dong, ma_may, nonce, may: {ten, mac, ip_noi_bo, ip_public, phien_ban}, chuc_nang?, n?, hang_thong_ke? }
→ 200 { du_lieu: "<json chuỗi>", chu_ky: "<base64>" }
du_lieu = { ma_may, nonce, luc, han_dem, han_tach, da_dung_dem, da_dung_tach, trang_thai }
```
- `Utilities.computeRsaSha256Signature(du_lieu, KHOA_BI_MAT)`; khoá bí mật PEM PKCS#8 trong `PropertiesService.getScriptProperties()`.
- Tab `license` (trạm ghi hết, Sao Mai sửa tay J–N): A Mã máy · B Tên máy · C MAC · D IP nội bộ · E IP public · F Phiên bản · G Ngày đăng ký · H Lần dùng cuối · I Đã dùng Đếm · Đã dùng Tách (trạm tính lại mỗi lần ghi) · J Hạn mức Đếm · K Hạn mức Tách · L Trạng thái · M Đơn vị · N Ghi chú. Trống J/K = 2, trống L = Dùng thử. (Sắp lại thứ tự cột cụ thể khi code; đếm từ nhật ký bằng code thay cho công thức.)
- Tab `license_nhat_ky`: Thời điểm · Mã máy · Chức năng · Nguồn (online / bù offline) · Phiên bản · IP public.
- Tab `Mô tả license` bên phải (cột · ý nghĩa · cách đọc + hướng dẫn cấp / khoá máy).
- `thong_ke` ghi tab `Ngưỡng trắng` y như hiện nay (giữ 21 cột tiếng Việt + tab mô tả).

## Related Code Files
- Create: `dema4_rs/apps-script/license.gs`, `dema4_rs/apps-script/appsscript.json`, `dema4_rs/apps-script/README.md` (cách triển khai)
- Create: `dema4_rs/apps-script/test-license.gs` (hàm test chạy trong trình soạn Apps Script)

## Tests trước
Viết trong `test-license.gs`, chạy bằng nút Run trong Apps Script trên **bản sao** spreadsheet (không đụng Sheet thật):
1. `test_dang_ky_may_moi`: `kiem_tra` mã mới → có dòng, hạn 2/2, Dùng thử; gọi lại không tạo dòng thứ 2.
2. `test_ghi_lan_cong_dung`: 2 `ghi_lan` Đếm + `ghi_bu` Tách n=1 → đã dùng 2/1; nhật ký 3 dòng, nguồn đúng.
3. `test_chu_ky_hop_le`: ký rồi tự kiểm bằng khoá công khai (hàm verify thủ công / so sánh với chữ ký tính lại) — bước kiểm chéo thật nằm ở test Rust phase 3.
4. `test_tu_choi_dau_vao_xau`: mã sai định dạng, chức năng lạ, n = 999 → lỗi, không ghi.
5. `test_dong_thoi`: 10 lời gọi `ghi_lan` liên tiếp → đã dùng = 10.

## Implementation Steps
1. Viết các hàm test (đỏ vì chưa có code), rồi viết `license.gs`: `doPost`, `kiemTra`, `ghiLan`, `ghiBu`, `thongKe`, `kyTraLoi`, `taoTabNeuThieu`.
2. Chạy test trên bản sao spreadsheet đến khi xanh.
3. Ghi README triển khai: tạo khoá RSA (`openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048`), dán khoá bí mật vào Script Properties `KHOA_BI_MAT`, Deploy › New deployment › Web app.

## Success Criteria
- [ ] 5 test Apps Script xanh trên bản sao.
- [ ] Gọi bằng `curl` tới URL thử: trả `du_lieu` + `chu_ky`.

## Risk Assessment
- Apps Script chậm (≈ 1–2 s / lời gọi, khởi động lạnh có thể 3–5 s) → app luôn gọi ngầm, không ảnh hưởng thao tác (phase 4).
- Hạn mức Apps Script tài khoản thường: ~20.000 lời gọi URL/ngày, thời gian chạy 6 phút/lời gọi, 30 lời gọi đồng thời → đủ quy mô hiện tại.
- URL trạm công khai trong exe / repo: ai cũng gọi được, nhưng chỉ làm được việc app làm (đăng ký máy, ghi 1 lần dùng); spam `ghi_lan` cho mã máy người khác chỉ làm *giảm* lượt của máy đó → giảm nhẹ bằng giới hạn tần suất theo mã máy (vd ≤ 30 lời gọi / 10 phút) qua `CacheService`.
