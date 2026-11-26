---
title: "Chức năng Xóa ghim (Rust, có AI bảo vệ chữ)"
description: "Chuyển XoaGhim sang dema4, Rust thuần; bản 1 chưa AI, bản 2 có AI YOLO DocLayNet qua ONNX Runtime đã nhúng"
status: pending
priority: P2
branch: "main"
tags: [xoa-ghim, onnx]
blockedBy: []
blocks: []
created: "2026-11-25T10:51:36.573Z"
createdBy: "ck:plan"
source: skill
---

# Chức năng Xóa ghim

## Overview
Chuyển XoaGhim (quangtv1/xoaghim) thành chức năng **Xóa ghim** của dema4, Rust thuần. `--tdd`. Nghiên cứu: `../reports/nghien-cuu-261125-1900-xoaghim-kem-ai-onnx-trong-rust-report.md`, `../reports/danh-gia-lai-261125-1902-xoaghim-rust-du-chuc-nang-va-cai-tien-report.md`, `../reports/soat-ke-hoach-261125-1909-xoa-ghim-report.md`; red team: `reports/from-code-reviewer-to-planner-red-team-xoa-ghim-plan-review-report.md`.

**Đã chốt (user, 2026-11-25):**
- Thứ tự: **bản 1 (chưa AI) phát hành trước**, AI + cải tiến sau — vẫn làm đủ phạm vi.
- Model AI `yolov12s-doclaynet.onnx` (chấp nhận AGPL); ONNX Runtime đã nhúng, không cần `onnxruntime.dll`; tải khi bật lần đầu, kiểm SHA lúc tải; luôn chạy AI ở trang có vùng; kết quả AI chỉ nhớ trong RAM; bỏ máy chủ GPU từ xa + 4 bộ nhận diện cũ.
- "Bảo vệ nét đen" mặc định bật (tách khỏi AI); "Bảo vệ dấu đỏ / chữ ký xanh" mặc định tắt, giống nhau ở xem trước và xuất.
- **Xem trước = kết quả xuất** (cùng độ phân giải, khớp từng điểm ảnh).
- Kết quả: `<tên nguồn>_xoaghim/`, giữ thư mục con, giữ tên file gốc, không file chữ, không sửa gốc; chạy lại cùng nguồn ghi đè.
- Dữ liệu: kho riêng `kho_xoa_ghim.bin` (cùng code kho Tách, không tách module): toạ độ vùng (chung thư mục / riêng file / riêng trang; loại xoá / bảo vệ / xoá vô đối) + nhật ký; có trong gói Xuất / Nhập phiên (gộp theo thời điểm, kiểm dữ liệu).
- Làm tất cả cải tiến (C2–C4, N3, N5 ở bản 2 theo ngưỡng số đo); bỏ N1 (bỏ qua AI) và N2 lưu AI vào kho. C1 đo trên bộ mẫu → **kém hơn** (giữ lõi đen vết ghim) → bỏ, giữ đúng cách Python; chữ sát ghim để C3.

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Bản vá 2.4.x tương thích ngược](./phase-01-ban-va-2-4-tuong-thich.md) | Completed — v2.4.1 đã phát hành (CI Windows + macOS đạt) |
| 2 | [Lõi xoá ghim (khớp Python) + đối chiếu](./phase-02-loi-xoa-ghim.md) | Completed — khớp từng điểm 266/266 trang mẫu |
| 3 | [Kho Xóa ghim + xuất PDF](./phase-03-kho-va-xuat-pdf.md) | Completed |
| 4 | [Màn Xóa ghim — xem trước, vùng, thao tác](./phase-04-man-xem-truoc-vung.md) | Completed |
| 5 | [Chạy hàng loạt, license, trạm, trạng thái](./phase-05-chay-hang-loat-license.md) | Completed — trạm đã triển khai (@16) |
| 6 | [Phát hành bản 1 (chưa AI)](./phase-06-phat-hanh-ban-1.md) | Pending |
| 7 | [AI bảo vệ chữ](./phase-07-ai-bao-ve-chu.md) | Chuyển sang [kế hoạch 261126-1007 phase 10](../261126-1007-code-man-xoa-ghim-theo-prototype/phase-10-ai-nhan-dien-layout.md) |
| 8 | [Cải tiến C2–C4, N3, N5 theo số đo](./phase-08-cai-tien-chinh-xac-toc-do.md) | Pending |
| 9 | [Phát hành bản 2](./phase-09-phat-hanh-ban-2.md) | Pending |

## Acceptance
- Bản 1: đủ chức năng dùng thật của XoaGhim trừ AI; chế độ khớp Python đạt đối chiếu (mặt nạ trước giãn khớp, IoU sau giãn ≥ 0,99, trên ảnh trang Python xuất sẵn — đạt: khớp từng điểm); xem trước khớp từng điểm ảnh với file xuất.
- Trạng thái "đã xóa" luôn đúng với file trên đĩa; không ghi đè kết quả của nguồn khác cùng tên.
- Máy còn 2.4 (sau bản vá) không mất danh sách gần đây / lượt offline; gói phiên bản mới vẫn nhập được ở 2.4.
- Trạm nhận lượt + thống kê Xóa ghim (chỉ số đếm), Telegram báo đúng.
- Bản 2: bật AI chữ / bảng sát mép không bị xoá; đối chiếu AI đạt; không mạng / không model xoá ghim thường vẫn chạy; tốc độ đo trên Windows.

## Để sau
- Mang nhãn Tách của file gốc sang file đã xóa ghim.

## Còn mở
- Bộ PDF scan có ghim (≥ 30 trang, ≥ 2 máy scan, màu + xám + đen trắng, dấu đỏ, chữ sát ghim) — cần trước khi đóng giai đoạn 2.

## Red Team Review

### Session — 2026-11-25
**Findings:** 15 (11 accepted, 4 rejected by user) — 39 phát hiện gốc từ 4 người soát, gộp còn 15.
**Severity breakdown:** 2 Critical, 10 High, 3 Medium

| # | Finding | Severity | Disposition | Applied To |
|---|---------|----------|-------------|------------|
| 1 | Cổng khớp Python < 0,1% không đạt được (MuPDF khác bản, khuếch đại, Shapely, ORT) | Critical | Accept | Phase 2, 7 |
| 2 | Không có bản dùng được sớm | Critical | Accept (đổi thứ tự) | Toàn kế hoạch |
| 3 | Bảo vệ nét đen gắn với AI → mặc định xoá nhiều chữ hơn | High | Accept | Phase 2, 4 |
| 4 | Trạng thái "đã xóa" không kiểm được; mất điện | High | Accept | Phase 3, 5 |
| 5 | Trùng thư mục ra / kiểm "ra trong nguồn" chưa chuẩn hoá | High | Accept | Phase 3 |
| 6 | Chạy lại ghi đè làm mất nhãn Tách trên file ra | High | Reject (user: ghi đè, nhắc F1) | Phase 4 F1 |
| 7 | Thay ảnh tại chỗ: tài nguyên dùng chung, CMYK / Decode, 1 bit, OCR / Thumb | High | Accept | Phase 8 |
| 8 | Bộ nhớ / luồng AI | High | Reject (user giữ phiên chung + ½ nhân) | Phase 7 (ghi rủi ro) |
| 9 | Kiểm model mỗi lần nạp, tag bất biến | High | Reject (user: kiểm lúc tải) | Phase 7; N5 vẫn dùng tag mới |
| 10 | 2.4: tăng `PHIEN_BAN_GOI` thừa; danh sách gần đây bị xoá trắng | High | Accept | Phase 1, 3 |
| 11 | License mã lạ bị bỏ ở 2.4; trạm chỉ nhận chức năng đã biết; số liệu trạm | High | Accept | Phase 1, 5 |
| 12 | Gói phiên mang dữ liệu Xóa ghim không kiểm | High | Reject (user giữ trong gói) → thêm gộp theo thời điểm + kiểm dữ liệu | Phase 3 |
| 13 | Vùng đang soạn không có bản ghi; vùng chung thư mục không có id | Medium | Accept | Phase 3, 4 |
| 14 | Tách module kho rủi ro; khoá / Dọn 2 kho | Medium | Accept | Phase 3 |
| 15 | N1 bỏ qua AI; N2 thiếu hộp; đo trên Mac | Medium | Accept phần đo; user chọn luôn chạy AI → bỏ N1; N2 chỉ RAM | Phase 7 |

### Whole-Plan Consistency Sweep
- Viết lại toàn bộ 9 giai đoạn theo thứ tự mới; đã bỏ: `CAU_HINH_XG`, `AI_XG`, N1, tăng `PHIEN_BAN_GOI`, tách module kho, thay ảnh tại chỗ ở bản 1, cổng "< 0,1% điểm khác".
- Không còn mâu thuẫn chưa giải quyết.
