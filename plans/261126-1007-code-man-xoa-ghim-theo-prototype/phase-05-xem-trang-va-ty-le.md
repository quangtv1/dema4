---
phase: 5
title: "Xem trang: 2 khung cuộn đồng bộ, kiểu xem, tỉ lệ"
status: completed
priority: P1
dependencies: [2]
---

# Phase 5: Xem trang và tỉ lệ

## Overview
Hiện tại Gốc và Đích nằm trong **1** `ScrollArea` chung. Prototype có **2 khung cuộn riêng** (mỗi khung 1 thanh cuộn) **cuộn đồng bộ**, kiểu xem Liên tiếp / Từng trang, vừa ngang / vừa cao tính chính xác theo khung thật, và tỉ lệ cập nhật khi đổi cỡ.

## Bảng hành vi
| Thao tác | Kết quả |
|---|---|
| Cuộn khung Gốc / Đích | Khung kia cuộn theo (cùng offset dọc + ngang) — trừ khi tắt "Cuộn Gốc \| Đích cùng nhau" trong Cấu hình |
| Cuộn (Liên tiếp) | Số trang ở thanh đáy, ô thu nhỏ (viền xanh 2 px + số nền xanh) đổi theo trang ở 35% chiều cao khung |
| ◂ ▸ / PageUp / PageDown / bấm ô thu nhỏ / gõ số + Enter | Liên tiếp: cuộn cả hai khung tới đầu trang (−6 px); Từng trang: chỉ hiện trang đó (cả 2 khung) |
| Kiểu xem Liên tiếp / Từng trang (thanh đáy + Cấu hình) | Đổi ngay, giữ trang đang xem |
| Vừa ngang (phím `=`) | Tỉ lệ = (rộng khung xem − 24)/rộng trang; trang phủ hết bề ngang, lề 12 mỗi bên; Gốc và Đích bằng nhau |
| Vừa cao | Tỉ lệ = (cao khung − 24)/cao trang |
| Đang vừa khung + đổi cỡ (kéo danh sách, thu nhỏ, cửa sổ, ẩn Đích, bật cột thu nhỏ) | Tính lại ngay theo khung **sau** khi đổi; ô tỉ lệ hiện % làm tròn |
| Ô tỉ lệ | 25, 41, 50, 55, 70, 85, 100, 125, 150 (+ mục % hiện tại khi đang vừa khung) |
| − / + (phím `-` `+`) | ±10% (20–150%); Ctrl + cuộn chuột ±5% |
| Nút vừa ngang / vừa cao đang dùng | nền `#dbe7fb`, viền `#93b4ea` |
| Mở 1 file / thư mục | Trang 1, vừa ngang |
| Ẩn Đích (›) | Gốc phủ hết; nút ‹ ở tiêu đề Gốc |

## Requirements
- Functional: như bảng. Ảnh trang vẫn từ worker `xem_truoc.rs` (giữ cache, `CANH_HIEN_TOI_DA`, đánh dấu "đang cập nhật…").
- Non-functional: cuộn 2 khung mượt ở 18 trang × 2; không tạo texture lại khi chỉ cuộn.

## Architecture
- `khung_trang.rs`: 2 `ScrollArea::both().id_salt("xg_goc"/"xg_dich")`. Mỗi khung đọc `state.offset` sau khi vẽ; khung nào đổi (so với `offset_cuoi`) thì ghi `cuon_toi = Some(offset)` cho khung kia ở frame sau (`scroll_offset(..)`) — tránh vòng lặp bằng cờ nguồn.
- `TyLe { VuaNgang, VuaCao, CoDinh(f32) }` thay `ty_le: Option<f32> + vua_cao`.
- Tỉ lệ vừa khung tính từ `ui.available_size()` **của khung xem trong frame hiện tại** (egui bố cục ngay tại chỗ nên không bị trễ 1 frame như HTML).
- `KieuXem { LienTiep, TungTrang }` (thay `lien_tiep: bool`).

## Related Code Files
- Modify: `gui_xoa_ghim/khung_trang.rs`, `gui_xoa_ghim.rs` (trạng thái, phím `[ ] PgUp PgDn = + -`)
- Reuse: `xem_truoc.rs` (không đổi giao thức)

## Implementation Steps
1. Thay trạng thái tỉ lệ / kiểu xem bằng enum; sửa các chỗ dùng.
2. Tách 1 ScrollArea → 2, đồng bộ offset; test kittest: cuộn Gốc → Đích cùng offset; tắt đồng bộ → không theo.
3. Theo dõi trang hiện tại theo vị trí cuộn; đồng bộ ô thu nhỏ + ô số.
4. Từng trang: chỉ vẽ trang `trang` ở cả 2 khung.
5. Thanh đáy: ô số nhập, combo kiểu xem, 2 nút vừa, combo tỉ lệ, − +.
6. Test đơn vị tính tỉ lệ vừa ngang / vừa cao; kittest trang Gốc rộng = trang Đích rộng ở 3 bề rộng danh sách.

## Success Criteria
- [ ] Bảng hành vi qua test.
- [ ] Vừa ngang: |rộng trang − (rộng khung − 24)| ≤ 1 px ở cả 2 khung.

## Risk Assessment
- Đồng bộ cuộn hai chiều dễ rung khi 2 khung khác chiều cao nội dung (Đích chưa có ảnh) → luôn cấp chỗ trang bằng kích thước trang (placeholder), nên cao bằng nhau.
