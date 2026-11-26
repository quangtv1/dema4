---
phase: 6
title: "Phát hành bản 1 (chưa AI)"
status: in-progress
priority: P2
dependencies: [5]
---

# Phase 6: Phát hành bản 1 (chưa AI)

## Requirements
- README, F1, hướng dẫn docx / pdf (chương Xóa ghim), ảnh chụp, phiên bản, soát mã; ghi chú phát hành.
- Thứ tự: bản vá 2.4.x (giai đoạn 1) đã phát hành → trạm đã triển khai → phát hành app.
- Commit + tag + push khi user đồng ý.

## Success Criteria
- [ ] Toàn bộ test xanh (trừ 5 lỗi có sẵn); CI 2 nền tảng xanh; thử trên Windows thật.

## Tiến độ (2026-11-26)
- v2.5.0; README (chức năng, ghi chú nâng cấp, mục Xóa ghim + ảnh, kho `kho_xoa_ghim.bin`, Xuất / Nhập phiên ở màn Xóa ghim); F1 tab Xóa ghim; hướng dẫn docx chương 7 Xóa ghim + sự cố + phụ lục + lịch sử 1.3, PDF xuất bằng Word (PDF bị `.gitignore` — chỉ cập nhật trên máy).
- Ảnh tài liệu chụp từ PDF mẫu **giả** (không dùng bộ mẫu thật: có tên người, chữ ký, repo công khai) — test `chup_man_xoa_ghim`; Trang chủ có dòng Xóa ghim; ảnh chú thích Trang chủ đánh số lại.
- Thêm: Xuất / Nhập phiên bật ở màn Xóa ghim (trước chỉ Tách / Bóc tách), nhập xong nạp lại vùng.
- Sửa test chập chờn `boc_tach_tiep_tuc…`: chụp số lượt trước khi luồng ghi trạm lần 1 xong → +2; chờ `moc()` rồi mới chụp.
- Còn: tag `v2.5.0` (chờ user) → CI 2 nền tảng; thử trên Windows thật.
