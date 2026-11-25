---
phase: 7
title: "Nguồn đám mây: S3/GCS, Google Drive"
status: pending
priority: P2
dependencies: [6]
---

# Giai đoạn 7: Nguồn đám mây (sau bản đầu)

## Yêu cầu
- Kho S3 + Google Cloud Storage (HMAC) qua 1 bộ đọc chuẩn S3: liệt kê `.pdf` theo tiền tố, tải về thư mục tạm trong `.sm-dema4/boc_tach/tam/`, xoá sau khi đọc. Nhà cung cấp: AWS, GCS, MinIO, Cloudflare R2, Wasabi.
- Google Drive: đăng nhập OAuth (loopback, quyền chỉ đọc `drive.readonly`), chọn thư mục; hoặc dùng thư mục Drive for Desktop như nguồn Máy tính (đã dùng được từ bản đầu).
- Cấu hình › Nguồn văn bản: bật các ô đang "Sắp có", Kiểm tra kết nối ("✓ Đọc được n bucket").

## Kiến trúc
- `src/boc_tach/nguon.rs`: trait `NguonVanBan { liet_ke(), tai(i) }` + `MayTinh`, `S3` (ký SigV4 tự viết bằng `sha2`/HMAC hoặc crate nhỏ), `GoogleDrive`.

## Rủi ro
- OAuth desktop cần client ID của đơn vị — chốt trước khi làm.
- Ký SigV4 dễ sai múi giờ / mã hoá đường dẫn → test với MinIO cục bộ.
