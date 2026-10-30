# Trạm license dema4 (Google Apps Script)

Trạm là bên **duy nhất** ghi Google Sheet: đăng ký máy, đếm lượt (chống trùng theo mã lần chạy), ký RSA câu trả lời,
xếp và gửi tin Telegram (token ở tab `Cấu hình`), ghi tab `Ngưỡng trắng`. App chỉ giữ khoá **công khai**.

| File | Vai trò |
|---|---|
| `loi.js` | Logic thuần (kiểm đầu vào, đếm lượt, soạn tin, chọn tin gửi). Dán vào Apps Script thành file `loi`. |
| `license.gs` | Lớp nối Sheet / Cache / Lock / UrlFetch; `doPost`, `guiHangDoi`, `caiTrigger`, `tuKiemKhoa`. |
| `appsscript.json` | Múi giờ, quyền, web app Anyone. |
| `test/` | Test Node: `node --test dema4_rs/apps-script/test/loi.test.js`. |

## Triển khai (chủ Sheet)
0. **Trước hết** đổi token bot Telegram (BotFather › `/revoke`) và vô hiệu khoá service account cũ (Google Cloud Console), gỡ email service account khỏi chia sẻ spreadsheet.
1. Thử quyền: deploy 1 web app chỉ có `doGet` → `curl -L <url>` từ máy ngoài tên miền phải ra `{"ok":true…}`.
2. Tạo khoá: `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out license_priv.pem` và `openssl pkey -in license_priv.pem -pubout -out license_pub.pem`.
3. Spreadsheet › Tiện ích mở rộng › Apps Script: tạo file `loi` (dán `loi.js`), `license` (dán `license.gs`), bật hiện `appsscript.json` (Project Settings) rồi dán nội dung.
4. Project Settings › Script Properties: `KHOA_BI_MAT` = toàn bộ `license_priv.pem` (dán nguyên, có thể thay xuống dòng bằng `\n`). Chạy `tuKiemKhoa` → Execution log có `chu_ky=…`. Xoá `license_priv.pem` khỏi máy.
5. Deploy › New deployment › Web app (Execute as **Me**, Who has access **Anyone**) → chép URL. Sửa script sau này: Manage deployments › Edit › **New version** (giữ nguyên URL).
6. Tab `Cấu hình` (tự tạo ở lời gọi đầu, hoặc tạo tay): dòng `telegram_token` | token mới, `telegram_chat_id` | id nhóm.
7. Chạy `caiTrigger` 1 lần, cấp quyền khi Google hỏi.
8. Gửi `license_pub.pem` + URL cho người build app (khoá công khai không phải bí mật).

## Bản đang chạy
- Dự án Apps Script gắn Monitor_dema4 (tạo bằng `clasp create --parentId <id Sheet>`); deployment web app `AKfycbyR90…` — URL nằm ở `URL_TRAM` (`src/tram.rs`).
- Sửa script: chép `loi.js`, `license.gs` (đổi tên `license.js`), `appsscript.json` vào thư mục có `.clasp.json` → `clasp push --force` → `clasp update-deployment <deploymentId>` (giữ URL). clasp 3 có thể báo "already up to date" khi chỉ xoá file: sửa 1 file rồi push lại, kiểm bằng `clasp pull` vào thư mục trống.

## Cấp lượt
Tab `license` → tìm mã máy → sửa **Hạn mức Đếm / Tách** (tổng số lần) → Trạng thái **Đã cấp** → khách bấm **Kiểm tra lại**. Xem tab `Mô tả license`.

## Đổi mẫu tin Telegram
Sửa `soanTinDem` / `soanTinTach` / `dongNguong` trong `loi.js` + mẫu trong `test/mau-tin/`, chạy test Node, dán lại vào Apps Script (New version).
