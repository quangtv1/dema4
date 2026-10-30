---
phase: 2
title: "Tram Apps Script"
status: pending
priority: P1
dependencies: []
effort: "2.5 ngày"
---

# Phase 2: Trạm Apps Script (bên duy nhất ghi Sheet)

## Overview
Web app Google Apps Script chạy bằng quyền tài khoản Sao Mai (Execute as Me, Who has access Anyone). App gọi qua HTTPS POST; trạm ghi tab `license`, `license_nhat_ky`, `Ngưỡng trắng`, `Hàng đợi Telegram`; ký RSA mọi câu trả lời license. Logic viết thành **hàm thuần** (không gọi SpreadsheetApp / UrlFetchApp trực tiếp) để test bằng **Node** trong repo; lớp mỏng `doPost` nối hàm thuần với Sheet. <!-- Red Team: test lặp lại được -->

## Requirements
- Hành động: `kiem_tra` (đọc / đăng ký máy mới), `ghi_lan` (ghi danh sách lần chạy có mã, trả trạng thái mới đã ký), `thong_ke_dem`, `thong_ke_tach` (xếp tin vào hàng đợi; Tách ghi thêm 1 hàng Ngưỡng trắng).
- **Chống trùng**: mỗi lần chạy có `ma_lan` 32 hex; trạm bỏ qua mã đã ghi (tra cột mã lần trong nhật ký qua `CacheService` + `TextFinder`). Một lời gọi `ghi_lan` nhận tối đa 3 lần (1 vừa chạy + ≤ 2 bù offline). <!-- Red Team: đếm trùng / đốt quota máy khác -->
- **Tổng đã dùng lưu ngay trong dòng `license`** (cộng +1 mỗi lần mới), nhật ký chỉ để đối chiếu; không quét nhật ký mỗi lời gọi. <!-- Red Team: trạm chậm dần -->
- **Giới hạn toàn cục** (CacheService): ≤ 30 đăng ký máy mới / giờ, ≤ 120 lời gọi / phút; vượt → `{ok:false, loi:"ban"}`. `kiem_tra` không ghi đè thông tin dòng đã có (chỉ cập nhật Lần dùng cuối / Phiên bản / IP).
- **Lỗi luôn là JSON** `{ok:false, loi}` (Apps Script không trả được mã 503); khoá `LockService.waitLock(2500)` — ngắn hơn thời gian chờ của app. <!-- Red Team: không có 503 -->
- **Kiểm tra đầu vào chặt**: mã máy `^SM-[0-9A-F]{4}(-[0-9A-F]{4}){2}$`, ma_lan `^[0-9a-f]{32}$`, chức năng ∈ {dem, tach}, IP theo mẫu IPv4/IPv6, phiên bản `^\d+\.\d+\.\d+$`, tên máy / thư mục: bỏ ký tự điều khiển, cắt 100 / 200 ký tự. **Mọi ô chữ ghi dạng văn bản thuần** (`setNumberFormat("@")` + `setValues`, không `USER_ENTERED`; ký tự đầu `= + - @` thêm `'`). <!-- Red Team: chèn công thức -->
- Trạng thái trả về bằng mã ASCII `dung_thu | da_cap | khoa`; cột Trạng thái trên Sheet là **danh sách thả xuống** (Dùng thử / Đã cấp / Khoá); giá trị lạ → `khoa`. Chữ ký: `Utilities.computeRsaSha256Signature(du_lieu, khoa, Utilities.Charset.UTF_8)`. <!-- Red Team: tiếng Việt -->
- **Telegram**: token + id nhóm ở tab `Cấu hình`; trạm tự soạn tin từ số liệu, **escape HTML** mọi trường chữ.
- **Hàng đợi**: dòng có `ma` riêng; `guiHangDoi` (trigger mỗi phút) dưới khoá ngắn đánh dấu tối đa N dòng "đang gửi" + thời điểm, nhả khoá, gửi, rồi dưới khoá ngắn đánh dấu "đã gửi" theo `ma` (không theo số dòng). Ngân sách **15 lời gọi Telegram / lượt** (tin Tách + CSV = 2). "Đang gửi" quá 5 phút → trả về "chờ". 429 → dừng lượt. Hàng đợi > 2.000 dòng chờ → bỏ tin mới (ghi log). <!-- Red Team: hàng đợi tranh khoá, gửi trùng -->

## Architecture
```
POST {url}  JSON: { hanh_dong, ma_may, nonce, may:{ten, mac, ip_noi_bo, ip_public, phien_ban}, lan?:[{ma_lan, cn, luc, nguon}], so_lieu? }
→ { ok:true, du_lieu:"<json>", chu_ky:"<base64>" } | { ok:false, loi:"<mã lỗi>" }
du_lieu = { ma_may, nonce, han_dem, han_tach, da_dung_dem, da_dung_tach, trang_thai, da_ghi:[ma_lan…] }
```
- Tab `license`: A Mã máy · B Tên máy · C MAC · D IP nội bộ · E IP public · F Phiên bản · G Ngày đăng ký · H Lần dùng cuối · I Đã dùng Đếm · J Đã dùng Tách (trạm ghi) · K Hạn mức Đếm · L Hạn mức Tách · M Trạng thái (thả xuống) · N Đơn vị · O Ghi chú (Sao Mai sửa K–O; trống K/L = 2, trống M = Dùng thử).
- Tab `license_nhat_ky`: Thời điểm · Mã máy · Mã lần · Chức năng · Nguồn (online / bù offline) · Phiên bản.
- Tab `Hàng đợi Telegram`: Mã · Thời điểm · Loại · Nội dung · Tên CSV · CSV · Số lần thử · Trạng thái · Lúc đánh dấu.
- Tab `Mô tả license` bên phải.
- Mẫu tin Telegram: chép từ `telegram.rs` (`noi_dung_thong_ke`, `noi_dung_tach`, `dong_nguong`); **trước khi** sửa Rust, lưu đầu ra hiện tại làm chuỗi mẫu cố định cho test Node.

## Related Code Files
- Create: `dema4_rs/apps-script/license.gs` (lớp mỏng), `dema4_rs/apps-script/loi.js` (hàm thuần, dùng chung Apps Script + Node), `dema4_rs/apps-script/appsscript.json`, `dema4_rs/apps-script/README.md`
- Create: `dema4_rs/apps-script/test/loi.test.js` (Node `node --test`), `dema4_rs/apps-script/test/mau-tin/*.txt` (đầu ra Rust hiện tại)

## Tests trước (Node, chạy được trên CI)
1. `dang_ky_may_moi`: máy mới → dòng 2/2 dung_thu; gọi lại không tạo dòng thứ 2, không ghi đè tên.
2. `ghi_lan_chong_trung`: cùng `ma_lan` gửi 2 lần → đã dùng +1; 4 lần trong 1 lời gọi → từ chối.
3. `tong_luu_trong_dong`: 10 lần ghi → cột Đã dùng = 10 mà không đọc nhật ký.
4. `kiem_dau_vao`: mã sai mẫu, IP lạ, `=IMPORTXML(...)` trong tên → bị làm sạch / từ chối; ô ghi ra bắt đầu bằng `'`.
5. `trang_thai_la_la_khoa`, `chu_ky_utf8_tieng_viet`: tên có dấu → ký + kiểm (khoá test) khớp.
6. `soan_tin_khop_mau_rust`: so với `mau-tin/*.txt`; tên thư mục có `<a href>` → bị escape.
7. `gioi_han_toan_cuc`: 31 đăng ký / giờ → lần 31 `ban`.
8. `hang_doi_ngan_sach`: 10 tin Tách (20 lời gọi) → lượt 1 gửi 7 tin (14 lời gọi) + không quá 15; 429 → dừng; "đang gửi" quá 5 phút → về chờ; đánh dấu theo `ma`.
9. Đồng thời (ngoài trình soạn, sau phase 5): script ngoài bắn 20 POST song song → không mất / trùng lượt.

## Implementation Steps
1. Lưu mẫu tin Rust hiện tại ra `test/mau-tin/`.
2. Viết test Node 1–8 (đỏ) → viết `loi.js` → xanh.
3. Viết `license.gs` nối Sheet / UrlFetch / Cache / Lock với `loi.js`; hàm `caiTrigger()`, `tuKiemKhoa()` (ký + kiểm bằng khoá công khai trong Script Properties, báo khoá dán sai).
4. README: tạo khoá, dán (khoá PEM: thay `\n` → xuống dòng, `tuKiemKhoa()` báo lỗi nếu hỏng), deploy, giữ URL khi sửa (New version).

## Success Criteria
- [ ] `node --test dema4_rs/apps-script/test` xanh (thêm vào CI).
- [ ] `tuKiemKhoa()` báo OK trên trạm thật.

## Risk Assessment
- Hạn mức Apps Script (~30 lời gọi đồng thời; UrlFetch, trigger theo loại tài khoản) — app chịu lỗi, giới hạn toàn cục chặn lạm dụng.
- **Đã chấp nhận**: spreadsheet ở Shared Drive TTCNTT → thành viên Drive mở được Apps Script, đọc được khoá RSA bí mật (Script Properties) và token (tab Cấu hình). Hệ quả: một thành viên có thể ký câu trả lời giả cho máy bất kỳ. Giảm nhẹ: chỉ người tin cậy trong Drive; lộ thì tạo cặp khoá mới + phát hành app mới.
- Quyền "Anyone" có thể bị chặn ở tên miền giáo dục → phase 5 thử bằng web app mẫu trước khi viết trạm thật.
