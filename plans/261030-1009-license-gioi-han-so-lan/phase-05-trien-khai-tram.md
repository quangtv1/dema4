---
phase: 5
title: "Trien khai tram va Sheet that"
status: pending
priority: P1
dependencies: [2]
effort: "0.5 ngày (thao tác của chủ Sheet)"
---

# Phase 5: Triển khai trạm lên Sheet thật

## Overview
Thu hồi bí mật cũ **trước**, thử quyền web app, rồi gắn trạm vào Monitor_dema4 (giữ trong Shared Drive TTCNTT — đã chấp nhận rủi ro).

## Implementation Steps
0. **Ngay bây giờ, không chờ phát hành** <!-- Red Team: bí mật đã lộ -->:
   - Đổi token bot: BotFather › `/revoke` (token cũ nằm trong repo công khai và lịch sử git).
   - Vô hiệu hoá khoá service account trong Google Cloud Console và gỡ email service account khỏi chia sẻ Monitor_dema4 (khoá nằm trong exe v0.5.x công khai). Từ lúc này v0.5.x ngừng ghi Sheet / gửi Telegram.
   - Chỉ sau bước này mới tạo bất kỳ tab license / Cấu hình nào.
1. **Thử quyền "Anyone"**: tạo web app mẫu `doPost` trả `{"ok":true}`, deploy Anyone, gọi bằng `curl -L --data '{}' <url>` từ máy ngoài tên miền → phải ra JSON (không phải trang đăng nhập). Không được → dừng, chọn tài khoản khác cho trạm.
2. Tạo cặp khoá: `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out license_priv.pem`; `openssl pkey -in license_priv.pem -pubout -out license_pub.pem`.
3. Monitor_dema4 › Tiện ích mở rộng › Apps Script: dán `license.gs`, `loi.js`, `appsscript.json`; Script Properties `KHOA_BI_MAT` (nội dung `license_priv.pem`), `KHOA_CONG_KHAI`. Chạy `tuKiemKhoa()` → OK. Xoá file khoá bí mật khỏi máy.
4. Deploy › New deployment › Web app (Execute as Me, Anyone) → chép URL; sửa sau này dùng Manage deployments › New version để giữ URL.
5. Tab `Cấu hình`: `telegram_token` (token **mới** ở bước 0), `telegram_chat_id`.
6. Chạy `caiTrigger()` (trigger `guiHangDoi` mỗi phút, không trùng) và cấp quyền UrlFetch.
7. Chép `license_pub.pem` vào `dema4_rs/assets/`, URL vào hằng `license.rs`; chạy `kho_tram_that` → tab `license`, `license_nhat_ky`, `Hàng đợi Telegram`, `Mô tả license` xuất hiện; thử `thong_ke_dem` → tin tới nhóm trong ≤ 1 phút.
8. Cột Trạng thái: thêm danh sách thả xuống (Dùng thử / Đã cấp / Khoá). Cấp máy dev / nội bộ: hạn mức lớn + Đã cấp.
9. Chạy test đồng thời (phase 2 test 9) bằng script ngoài.

## Success Criteria
- [ ] Bước 0 xong: token cũ gửi tin → 401; khoá service account cũ ghi Sheet → 403.
- [ ] `curl -L --data '{"hanh_dong":"kiem_tra",…}' <url>` trả `ok:true` + `chu_ky` kiểm được bằng `license_pub.pem`.

## Risk Assessment
- **Đã chấp nhận**: Shared Drive TTCNTT — thành viên Drive xem được khoá RSA và token.
- **Đã chấp nhận**: các bản v0.4.x–v0.5.x vẫn tải được trên Releases và không có giới hạn lượt.
- Lộ khoá bí mật → cặp khoá mới + Script Properties + phát hành app mới.
