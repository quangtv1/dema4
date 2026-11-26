---
phase: 3
title: "Danh sách: sắp xếp, trạng thái, lọc gộp, Lọc nâng cao"
status: completed
priority: P1
dependencies: [2]
---

# Phase 3: Danh sách và lọc

## Overview
Hoàn thiện cột Danh sách như prototype: sắp xếp 1 cột / lần, 4 trạng thái file bằng biểu tượng, ô lọc tên, **ô lọc gộp** (chọn trạng thái hoặc gõ số trang), **Lọc nâng cao** (khổ trang, chiều trang, dải trang) thu gọn mặc định.

## Bảng hành vi
| Hành vi | Kết quả mong đợi |
|---|---|
| Bấm "Tên file" / "Trang" | Sắp theo cột đó tăng dần; bấm lại → đảo chiều; cột đang dùng chữ xanh + ↑/↓ |
| Ô tick tiêu đề | Chọn / bỏ mọi file (tick = sẽ chạy) |
| Ô "Lọc tên file…" | Lọc theo tên (NFC, không phân biệt hoa thường) — như hiện có |
| Ô lọc gộp ▾ | Danh sách: Tất cả, Cần chạy, Chưa xóa, Đã xóa, Đã đổi vùng, Mất kết quả + dòng gợi ý "Hoặc gõ số trang: 1-3, 7, >10, <5" |
| Gõ vào ô lọc gộp | Lọc theo **số trang của file**: số, khoảng `a-b`, `>n`, `>=n`, `<n`, `<=n`, phẩy; khi có chữ thì bỏ qua lọc trạng thái; chọn trạng thái → xoá chữ |
| Biểu tượng trạng thái (14 px trước cột Trang) | ○ xám chưa xoá · ✓ xanh đã xoá đúng · ↻ cam đã đổi vùng / tuỳ chọn · ● đỏ mất kết quả — tooltip mô tả |
| Sửa vùng / tuỳ chọn | File "đã xoá đúng" chuyển "đã đổi vùng" (đã có logic `ma_cau_hinh`) |
| Lọc nâng cao (mặc định **thu gọn**) | Mở bằng bấm tiêu đề hoặc mũi tên góc phải; ô tick tiêu đề bật / tắt lọc mà không mất điều kiện; "· đang lọc" khi áp dụng |
| Điều kiện nâng cao | Có trang khổ A0–A7 (ô chưa chọn chữ xám), Chiều trang Ngang / Dọc, Trong dải trang `1-5, 7` → **Áp dụng** / **Bỏ lọc**; giữ file có ≥ 1 trang (trong dải) khớp khổ + chiều |
| Tiêu đề | `Danh sách (k/n)` (k = số tick) + `đang hiện m` khi lọc |
| Mở 1 file | Danh sách 1 dòng, `(1/1)` |
| Kiểu ghi "Cạnh file gốc" | Dòng nhắc cuối: "Đã ẩn các file kết quả *_xoaghim.pdf nằm cạnh file gốc." (lọc thật ở phase 6) |
| Không có kết quả lọc | "Không có file khớp bộ lọc." |

## Requirements
- Functional: như bảng. Phân loại khổ trang theo chuẩn ISO A (dùng lại bảng khổ của `dem_pdf.rs` — chức năng Đếm trang đã phân loại A0–A7 + ngưỡng sát).
- Non-functional: đọc khổ / chiều từng trang ở **luồng nền** cùng lúc đọc số trang (`doc_so_trang` hiện có), không chặn giao diện; 1000 file không giật (danh sách vẽ bằng `show_rows`).

## Architecture
- `gui_xoa_ghim.rs`: `so_trang_file: Vec<Option<u32>>` → `thong_tin_file: Vec<Option<ThongTinFile>>` với `ThongTinFile { so_trang, kho: Vec<(KhoA, bool /*ngang*/)> }` (nén theo đoạn liên tiếp nếu cần bộ nhớ).
- Trạng thái lọc: `sap_theo: SapTheo {Ten, Trang}`, `sap_giam`, `loc_ten`, `loc_tt: LocTrangThai`, `loc_so: String`, `loc_nc: LocNangCao { kho: [bool;8], ngang, doc, dai: String }`, `loc_nc_ap: Option<LocNangCao>`, `loc_nc_bat: bool`, `loc_nc_mo: bool` (mặc định false).
- Hàm thuần (có test): `khop_so(txt) -> Option<impl Fn(u32)->bool>`, `khop_nang_cao(&ThongTinFile, &LocNangCao) -> bool`, `thu_tu_hien(...) -> Vec<usize>`.

## Related Code Files
- Modify: `gui_xoa_ghim/danh_sach.rs`, `gui_xoa_ghim.rs` (doc_so_trang → doc_thong_tin), `gui_xoa_ghim/widget.rs` (ô lọc gộp)
- Reuse: `dem_pdf.rs` (phân loại khổ), `xoa_ghim/xuat.rs::TrangThai`, `tach_vb::phien::so_ten_tu_nhien`

## Implementation Steps
1. Hàm `khop_so` + test (các dạng cú pháp, chuỗi rác → None).
2. Đọc khổ / chiều trang ở luồng nền (mupdf `page.bounds()`, xoay theo `/Rotate`), gửi về theo lô.
3. `thu_tu_hien` mới (sắp tên tự nhiên / số trang + lọc tên + lọc gộp + nâng cao) + test.
4. Vẽ tiêu đề, hàng sắp xếp, ô lọc tên + ô lọc gộp, dòng file với biểu tượng trạng thái.
5. Khối Lọc nâng cao thu gọn / mở, Áp dụng / Bỏ lọc, ô tick bật/tắt.
6. kittest: lọc "Đã đổi vùng", gõ `>5`, nâng cao A3 + Ngang → đúng số dòng.

## Success Criteria
- [ ] Mọi dòng bảng hành vi có test hoặc kittest.
- [ ] Ảnh kittest khớp cột danh sách trong `anh-mo.png`.

## Risk Assessment
- PDF hỏng / có mật khẩu khi đọc khổ → bỏ qua file đó trong lọc nâng cao (không khớp), không báo lỗi giữa chừng.
