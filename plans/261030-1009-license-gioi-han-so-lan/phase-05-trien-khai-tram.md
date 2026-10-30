---
phase: 5
title: "Trien khai tram va Sheet that"
status: pending
priority: P1
dependencies: [2]
effort: "0.5 ngày (một phần thao tác của chủ Sheet)"
---

# Phase 5: Triển khai trạm lên Sheet thật

## Overview
Gắn `license.gs` vào spreadsheet Monitor_dema4, tạo khoá RSA, deploy web app, chép URL + khoá công khai vào app. Thu hồi quyền của service account cũ sau khi bản mới đã phát hành.

## Requirements
- Chỉ tài khoản Sao Mai có quyền sửa spreadsheet sau khi thu hồi service account.
- Khoá bí mật chỉ nằm trong Script Properties (không commit, không gửi qua chat).

## Implementation Steps
1. Tạo cặp khoá trên máy chủ Sheet:
   `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out license_priv.pem` ;
   `openssl pkey -in license_priv.pem -pubout -out license_pub.pem`.
2. Monitor_dema4 › Tiện ích mở rộng › Apps Script → dán `license.gs` + `appsscript.json`; Project Settings › Script Properties: `KHOA_BI_MAT` = nội dung `license_priv.pem`. Xoá file `license_priv.pem` khỏi máy sau khi dán (giữ 1 bản sao lưu an toàn ngoài máy nếu muốn).
3. Deploy › New deployment › Web app: Execute as **Me**, Who has access **Anyone** → chép URL.
4. Chép `license_pub.pem` vào `dema4_rs/assets/`, URL vào hằng trong `license.rs`.
5. Chạy `#[ignore]` `kho_tram_that` → xanh; tab `license`, `license_nhat_ky`, `Mô tả license` tự xuất hiện.
6. Cấp máy dev / nội bộ: điền Hạn mức lớn + Trạng thái "Đã cấp".
7. **Sau khi v0.6.0 đã phát hành và các máy đã cập nhật**: gỡ quyền service account khỏi spreadsheet (Chia sẻ › xoá email service account) và vô hiệu hoá khoá trong Google Cloud Console. Bản ≤ v0.5.x sẽ không ghi thống kê được nữa (chấp nhận).

## Success Criteria
- [ ] `curl -X POST <url> -d '{"hanh_dong":"kiem_tra",…}'` trả `du_lieu` + `chu_ky`, kiểm được bằng `license_pub.pem`.
- [ ] Sau bước 7: thử dùng khoá service account cũ ghi Sheet → 403.

## Risk Assessment
- Đổi deployment (mỗi lần sửa script) có thể đổi URL → dùng "Manage deployments › Edit › New version" để giữ URL.
- Lộ khoá bí mật → tạo cặp mới, cập nhật Script Properties + phát hành app mới (khoá công khai mới).
