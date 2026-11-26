---
phase: 2
title: "Khung màn: danh sách, thanh công cụ 2 nửa, Gốc / Đích, thanh đáy, màn trống"
status: completed
priority: P1
dependencies: [1]
---

# Phase 2: Khung màn và bố cục

## Overview
Dựng toàn bộ khung màn đúng prototype (ảnh `anh-trong.png`, `anh-mo.png`): cột Danh sách kéo rộng / thu nhỏ, thanh công cụ chia đôi thẳng ranh giới Gốc | Đích, khung Gốc (cột trang thu nhỏ + khung xem) và Đích rộng bằng nhau, thanh đáy, màn trống có 2 biểu tượng + logo Sao Mai về Trang chủ.

## Bố cục (từ trên xuống, từ trái sang)
```
[ Thanh menu chung 30px ................................................ ^ ]
[ Danh sách | Thanh công cụ: nửa trái (Góc Cạnh Tùy biến · lọc trang · Vùng/Rộng/Cao/Ngưỡng) ║ nửa phải (Đầu vào / File đích / tuỳ chọn) ]
[ (282px)   | Tiêu đề Gốc (◂ ▸ (i/n) · "Gốc: …" giữa)                ║ Tiêu đề Đích ("Đích: …" giữa · ›) ]
[ kéo 200–  | Trang thu nhỏ 150 | khung xem Gốc (W)                  ║ khung xem Đích (W) ]
[ 520 / dải | Thanh đáy 40: Zone chung/riêng · ◂ ▸ [trang]/n · Liên tiếp · ⟷ ↕ · 96% · − + ·······  ▶ Xóa ghim (n) ▾ ]
[ dọc 34    |                                                                                 ]
```

## Requirements
### Cột Danh sách
- Mặc định **mở rộng** mỗi lần vào màn (kể cả chưa mở gì). Trạng thái người dùng thu nhỏ giữ đến khi bấm mở lại hoặc ⊗ đóng thư mục (⊗ → về mặc định mở rộng).
- Tiêu đề (cao 32, nền `NEN_DS_TIEU_DE`): `Danh sách (k/n)` + `đang hiện m` (khi có lọc) + nút thu nhỏ (biểu tượng `|◂`) ở phải. **Không** có biểu tượng hamburger.
- Dải thu nhỏ 34 px: nút mở rộng (`▸|`) trên cùng, chữ **"Danh sách" xoay dọc** (đọc từ dưới lên) 12,5 px `#374151`, ô đếm xanh khi đã mở; bấm bất kỳ đâu trên dải → mở rộng.
- Kéo mép phải (vùng 6 px, con trỏ ↔, tô `rgba(37,99,235,.35)` khi rê/kéo) đổi rộng 200–520; bấm đúp → 282. Lưu độ rộng trong `cau_hinh` app (chung).
- Chưa mở: thân cột chỉ dòng "Chưa mở file hoặc thư mục." (không nút).
### Thanh công cụ (cao ~114, viền đáy `VIEN_KHUNG`)
- Chia 2 nửa bằng ranh giới 4 px **trùng x** với ranh giới Gốc | Đích bên dưới mọi lúc (kéo cột danh sách, thu nhỏ, đổi cỡ cửa sổ, bật / tắt cột thu nhỏ).
- Công thức (như prototype): nửa trái `flex: 1 1 (rộng_thu_nhỏ + 26)px` (26 = tổng đệm ngang), nửa phải `flex: 1 1 0`; khi không có cột thu nhỏ cả hai `flex 1 1 0`. Trong egui: tính `w_trai = (W − 4 + rộng_thu_nhỏ)/2`, `w_phai = W − 4 − w_trai` rồi `allocate_ui_with_layout` từng nửa — dùng **chung** hàm này cho thanh công cụ và hàng khung xem.
- Nửa trái: 3 biểu tượng (phase 4) · cột lọc trang (radio, `white-space:nowrap`) · vạch · khối Vùng (combo + 3 thanh trượt, co giãn, tối thiểu 200).
- Nửa phải: lưới 2 cột (nhãn 62 px) — Đầu vào, File đích, hàng tuỳ chọn (phase 6). Hàng tuỳ chọn **tự xuống dòng gọn** (không bẻ giữa chữ).
- Nút ^ ở thanh menu ẩn / hiện cả thanh công cụ.
### Khung Gốc / Đích
- Tiêu đề (cao 30, nền `NEN_TIEU_DE_KHUNG`): chữ **giữa khung**, cắt `…`, tooltip đường dẫn đầy đủ.
  - Gốc: trái có nút ◂ ▸ **44×22 luôn hiện** + `(i/n)`; tooltip tức thì "Văn bản trước ( [ )" / "Văn bản sau ( ] )"; mờ ở đầu / cuối (tooltip vẫn hiện). Mở 1 file → không có nút.
  - Đích: phải có › thu gọn khung Đích; khi Đích ẩn, nút ‹ nằm phải tiêu đề Gốc.
- Cột Trang thu nhỏ 150 px **chỉ khi đã mở** và bật trong Cấu hình.
- Khung xem Gốc và Đích **rộng bằng nhau**: Gốc = 151 + W, Đích = W (dùng hàm chia ở trên).
### Thanh đáy (cao 40)
- Trái: `Zone chung: x; Zone riêng: y` chữ thường (không đậm), rộng cố định 260.
- Giữa: ◂ ▸ · ô số trang (gõ + Enter) `/ n` · Kiểu xem (Liên tiếp / Từng trang) · Vừa ngang · Vừa cao (đang dùng: nền `#dbe7fb` viền `#93b4ea`) · tỉ lệ (25…150 + mục tính sẵn khi vừa khung) · − +.
- Phải (rộng 260): **▶ Xóa ghim (n)** + **▾** (phase 8).
### Màn trống (chưa mở)
- Khung Gốc: 2 biểu tượng lớn giữa khung (PDF "Mở file pdf", thư mục "Mở thư mục"), xám `#8a8f96`, hover nền `rgba(0,0,0,.05)`; cùng đáy chữ.
- Góc dưới phải vùng xem: **logo Sao Mai** (`assets/logo-sao-mai.png`, 74×61) + chữ "Trang chủ" 11 px; opacity .85 → 1 khi rê; bấm → `chuyen_man(Man::TrangChu)`.
- Khung Đích trống, tiêu đề "Gốc:" / "Đích:".

## Architecture
- `gui_xoa_ghim.rs::ve()` chuyển sang thứ tự: `thanh_menu` (gui.rs) → `Panel::left("xg_ds")` (danh sách hoặc dải) → `Panel::top` thanh công cụ → `Panel::bottom` thanh đáy → `CentralPanel` (hàng tiêu đề + 2 khung).
- Hàm chung `chia_doi(rong_tong, rong_thu_nho) -> (f32, f32)` trong `khung_trang.rs`; thanh công cụ gọi lại sau khi biết `rong_tong` của CentralPanel (đo `ctx.available_rect()` trừ panel trái — hoặc vẽ thanh công cụ **bên trong** CentralPanel để cùng gốc toạ độ; chọn cách sau cho chắc trùng).
- Chữ dọc: `epaint::TextShape { angle: -π/2, .. }`.
- Trạng thái mới trong `ManXoaGhim`: `ds_mo: Option<bool>` (None = mặc định mở), `ds_rong: f32`, `an_cong_cu: bool` (đã có), `an_dich` (đã có).

## Related Code Files
- Modify: `gui_xoa_ghim.rs` (ve, trạng thái), `gui_xoa_ghim/danh_sach.rs` (tiêu đề, dải dọc, kéo rộng), `gui_xoa_ghim/thanh_tren.rs` (chia 2 nửa), `gui_xoa_ghim/khung_trang.rs` (tiêu đề giữa, ◂ ▸, rộng bằng nhau, màn trống + logo)
- Reuse: `logo_sao_mai.rs` (texture logo), `gui.rs::chuyen_man`

## Implementation Steps
1. Viết `chia_doi()` + test đơn vị (có / không cột thu nhỏ, Đích ẩn).
2. Dựng lại `ve()` theo thứ tự panel mới; thanh công cụ vẽ trong CentralPanel trên hàng tiêu đề.
3. Danh sách: tiêu đề mới, dải thu nhỏ chữ dọc, kéo mép (Sense::drag trên rect 6 px), bấm đúp.
4. Tiêu đề Gốc / Đích: căn giữa (`Align2::CENTER_CENTER` + cắt chữ theo rộng còn lại), nút 44×22 + tooltip tức thì.
5. Màn trống + logo → Trang chủ.
6. Thanh đáy theo thứ tự prototype (nút chạy đặt chỗ, logic ở phase 8).
7. kittest: chụp 1600×900 trạng thái trống / đã mở; test đo x ranh giới thanh công cụ = x ranh giới khung (±1 px) ở 3 bề rộng danh sách.

## Success Criteria
- [ ] Ảnh kittest `trong` và `mo` khớp bố cục `anh-trong.png`, `anh-mo.png`.
- [ ] Ranh giới trùng nhau (test), khung xem Gốc = Đích (test).
- [ ] Danh sách: mặc định mở, thu nhỏ / mở lại, kéo 200–520, bấm đúp 282, ⊗ về mặc định.

## Risk Assessment
- `Panel::left` của egui tự xử lý resize nhưng tay nắm khác prototype → tự làm tay nắm (rect 6 px) và đặt `resizable(false)` + `exact_size`.
- Chữ dọc trong egui đôi khi lệch baseline → căn bằng `galley.size()` sau xoay; kiểm ảnh.
