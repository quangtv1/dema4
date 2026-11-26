---
phase: 8
title: "Chạy, tiến độ, kết quả, F1, thông báo nhanh"
status: completed
priority: P1
dependencies: [3, 6]
---

# Phase 8: Chạy, kết quả, trợ giúp

## Overview
Nút **▶ Xóa ghim (n)** + **▾** góc dưới phải, hộp tiến độ / kết quả kiểu prototype, bảng thao tác nhanh (F1), toast. Logic chạy hàng loạt, license, thống kê trạm **giữ nguyên** (`gui_xoa_ghim/chay.rs`, `xoa_ghim/lo.rs`) — chỉ đổi giao diện và cách đếm hiển thị.

## Bảng hành vi
| Thao tác | Kết quả |
|---|---|
| Nút Xóa ghim (n) | n = số file **đã tick và cần chạy** (chưa xoá / đã đổi vùng / mất kết quả) = `can_chay()` hiện có; mở 1 file → 0/1. n = 0 → mờ, tooltip "Mọi file đã tick đã xoá đúng vùng hiện tại"; chưa mở → "Mở file hoặc thư mục trước" |
| ▾ | Menu bật lên trên: "Chạy lại mọi file đã tick (k)" (= `bam_chay(true)`) |
| Đang chạy | Hộp 460 px: "Đang xoá ghim…", thanh 8 px xanh, "i / n file · t trang" + %, tên file đang chạy (xám 12 px), nút **Dừng** |
| Dừng | Tiêu đề "Đã dừng", "Các file chưa chạy giữ nguyên trạng thái." |
| Xong | Thanh xanh lá, "Xoá ghim xong", "Kết quả ở: <đích> · x,x giây", nút **Mở thư mục kết quả · Mở ở Tách · Đóng** (xanh). File đã chạy → ✓ |
| Hết lượt license | Hộp license hiện có (không đổi) |
| F1 / Trợ giúp → Hướng dẫn sử dụng (ở màn Xóa ghim) | Hộp "Xóa ghim — thao tác nhanh" 560 px: bảng 13 dòng phím (kiểu `<kbd>`) như prototype; nút Đóng |
| Toast | Đáy giữa, nền `#1f2937`, 12,5 px, tự ẩn 2,4 s |

## Requirements
- Functional: như bảng; dữ liệu tiến độ lấy từ kênh `lo::chay` hiện có (số file / trang / file hiện tại / thời gian).
- Bảng F1 (giữ nguyên thứ tự): Bấm góc / cạnh · − / + · Kéo → phải · Kéo ← trái · Shift + bấm · Ctrl+A · Delete · Ctrl+Z · Chuột phải · Esc · `[ ]` · PageUp / PageDown · `+ − =`.
- Hướng dẫn đầy đủ (tab HD_XOA_GHIM trong `tro_giup.rs`) vẫn mở được từ màn khác như cũ; ở màn Xóa ghim mục menu mở bảng thao tác nhanh (xem câu hỏi mở về nút "Hướng dẫn đầy đủ").

## Related Code Files
- Modify: `gui_xoa_ghim/chay.rs` (giao diện hộp, nút + ▾), `gui_xoa_ghim/khung_trang.rs` (thanh đáy gọi `nut_chay`), `gui.rs` (F1 theo màn), `tro_giup.rs` (cập nhật nội dung HD_XOA_GHIM theo giao diện mới)
- Create: `gui_xoa_ghim/tro_giup_nhanh.rs`

## Implementation Steps
1. Nút chạy + ▾ (menu bật lên trên — `egui::Popup` với `PopupAnchor` phía trên).
2. Hộp tiến độ / kết quả dùng `gui_tach::khung_hop::hop` (Modal) với bố cục prototype.
3. Bảng thao tác nhanh + F1.
4. Toast dùng chung (`widget::thong_bao_nhanh`).
5. kittest: n đúng theo trạng thái; ▾ chạy lại; hộp xong có 3 nút; F1 mở bảng 13 dòng.

## Success Criteria
- [ ] Số trên nút khớp `can_chay()` ở mọi tình huống lọc / tick.
- [ ] Ảnh hộp tiến độ / kết quả / F1 khớp prototype.
- [ ] License, thống kê trạm, "Mở ở Tách" chạy như bản 2.5.0.

## Risk Assessment
- Thấp — logic đã có và đã kiểm; chỉ đổi lớp hiển thị.
