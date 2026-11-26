---
phase: 4
title: "Vùng: biểu tượng, vẽ, chọn nhiều, kéo, chuột phải, Vô đối"
status: completed
priority: P1
dependencies: [2]
---

# Phase 4: Vùng và tương tác

## Overview
Đưa toàn bộ tương tác vùng của prototype vào egui: biểu tượng Góc / Cạnh bấm từng góc / cạnh, Tùy biến − / +, vẽ bằng chuột, chọn 1 / nhiều (Shift, Ctrl+A, quét kiểu AutoCAD Window / Crossing), kéo di chuyển / kéo nhóm / đổi cỡ 8 tay nắm (cạnh chỉ 1), tooltip kích thước khi kéo, chuột phải, Delete, hoàn tác, cột lọc trang theo vùng đang chọn, Vô đối, khối Vùng / Rộng / Cao / Ngưỡng kiểu mới, hàng tuỳ chọn Nhận diện layout / Tô nền trắng / Xóa vùng vẽ / Hoàn tác.

## Mô hình dữ liệu (giữ tương thích kho)
- Dùng nguyên `Vung { vi_tri, loai: Xoa|BaoVe|VoDoi, ap_trang: TatCa|Le|Chan|MotTrang(i), x, y, rong, cao, nguong, .. }` — đã đủ cho prototype (Vô đối = `LoaiVung::VoDoi`, cạnh `co_san()` đã phủ hết chiều dài).
- Phạm vi lưu: góc / cạnh = chung thư mục (`ThuMuc.vung`); vẽ Tùy biến = riêng file (`VungFile`) — như hiện tại (`PhamVi`).
- Chọn: `BoVung.chon: Option<ChonVung>` → `chon: Vec<ChonVung>` (thứ tự = thứ tự chọn, phần tử cuối = vùng chính). Cập nhật `vung_chon*`, `xoa_chon`, `sua_chon`.
- Hoàn tác: `lich_su` (đã có, `BUOC_HOAN_TAC`) — mọi thao tác dưới đây ghi 1 bước; đặt `BUOC_HOAN_TAC = 100`.

## Bảng hành vi
| Thao tác | Kết quả |
|---|---|
| Bấm 1 trong 4 ô góc trên biểu tượng **Góc** | Thêm vùng góc đó (xanh lá) và chọn nó; bấm lại → bỏ. Ô đang bật tô `XANH`, viền `XANH_DAM_CHU`; rê → `#cfe0fb`; tooltip "Góc trên trái — bấm để thêm/bỏ" |
| Bấm 1 trong 4 dải trên biểu tượng **Cạnh** | Như trên với cạnh (dải trên/dưới dài hết ngang, trái/phải hết dọc) |
| Phạm vi vùng góc / cạnh mới | Theo cột lọc trang hiện tại: Tất cả / Lẻ / Chẵn; Từng trang → trang đang xem |
| Nửa **−** biểu tượng Tùy biến | Bật / tắt chế độ vẽ vùng **xoá** (nền `#dbeafe` viền `XANH`); bỏ chọn vùng; lọc trang → Từng trang (trừ đang Vô đối) |
| Nửa **+** | Vẽ vùng **bảo vệ** (nền `#fce7f3`, viền `#db2777`) |
| Esc | Thôi vẽ → đóng menu → bỏ chọn (theo thứ tự) |
| Kéo chuột trên trang Gốc khi đang vẽ | Hình chữ nhật mới theo chuột; nhả < 1,5% trang → huỷ; con trỏ chữ thập; tooltip "Xóa 1 · 20% × 10% (42 × 30 mm)" theo chuột |
| Vùng vẽ mới | Tên "Xóa n" / "Bảo vệ n" / "Vô đối n"; phạm vi theo cột lọc trang (Vô đối & Từng trang → trang đang vẽ) |
| Bấm vùng (không vẽ) | Chọn 1; nhãn tên hiện trên vùng; 8 tay nắm (cạnh: 1 tay nắm mép trong) |
| Shift + bấm vùng | Thêm / bỏ khỏi nhóm chọn |
| Kéo thân vùng | Di chuyển trong trang (cạnh: không di chuyển) |
| Kéo vùng thuộc nhóm đang chọn (> 1) | Kéo cả nhóm (cạnh đứng yên), giữ trong trang |
| Kéo tay nắm | Đổi cỡ (tối thiểu 1,5% trang); cạnh chỉ đổi bề dày |
| Kéo trên vùng trống (không vẽ) sang **phải** | Khung xanh liền — chọn vùng **nằm trọn** (Window); Shift = thêm |
| Kéo sang **trái** | Khung xanh lá đứt — chọn vùng **chạm** (Crossing) |
| Bấm vùng trống | Bỏ chọn |
| Ctrl/Cmd + A | Chọn mọi vùng |
| Delete / Backspace | Xoá **ngay** các vùng vẽ Tùy biến đang chọn; toast "Đã xoá n vùng · Ctrl+Z để hoàn tác"; nếu chỉ chọn góc / cạnh → toast "Vùng cạnh / góc: bỏ bằng cách bấm lại trên biểu tượng" |
| Ctrl/Cmd + Z | Hoàn tác (100 bước) |
| Chuột phải trên vùng | Menu: tiêu đề "tên · phạm vi"; Xóa vùng (Delete); Hoàn tác (Ctrl+Z); —; Đổi thành vùng bảo vệ / xoá; Nhân bản vùng; —; Áp dụng cho: Mọi trang / Trang lẻ / Trang chẵn / Chỉ trang này (i) (mục hiện tại ✓ xanh) |
| Chuột phải chỗ trống | "Trang i"; Vẽ vùng xoá; Vẽ vùng bảo vệ; —; Hoàn tác; Xoá hết vùng vẽ |
| Cột lọc trang khi đang chọn 1 vùng (không vẽ) | Hiện phạm vi của vùng đó; đổi radio = đổi phạm vi vùng (ghi hoàn tác). **Vô đối** hiện khi đang vẽ xoá hoặc chọn 1 vùng xoá Tùy biến; chọn Vô đối = vùng thành `LoaiVung::VoDoi` + `MotTrang` |
| Đang vẽ | Radio là phạm vi cho vùng **sắp vẽ** (không đụng vùng đang chọn) |
| Combo **Vùng** | Liệt kê "tên · phạm vi"; khi chọn nhiều → "n vùng đang chọn"; chưa chọn → "— chọn vùng —" |
| Rộng / Cao / Ngưỡng | Sửa vùng chính khi chọn **đúng 1**; khoá khi chọn nhiều / không chọn; cạnh ngang khoá Rộng, cạnh dọc khoá Cao; góc/cạnh đổi cỡ giữ mép bám trang. Ô số: gõ + Enter |
| Nút **Xóa vùng vẽ** | Hộp xác nhận "Xóa n vùng vẽ?" liệt kê ≤ 8 vùng (ô màu theo loại) + "… và k vùng khác"; Hủy / Xóa n vùng (đỏ); Enter = xoá, Esc = huỷ; góc / cạnh giữ nguyên |
| Nút **Hoàn tác** | = Ctrl+Z; mờ khi hết bước |
| Ô **Nhận diện layout** | Mờ + link "tải mô hình" (mở Cấu hình) khi chưa có mô hình; cùng giá trị với Cấu hình |
| Ô **Tô nền trắng** | Cùng giá trị `TuyChon.nen_trang` với Cấu hình |
| Sửa vùng bất kỳ | Khung Đích báo "đang cập nhật…" (đã có) rồi vẽ lại; file đã xoá → "đã đổi vùng" |

## Vẽ (painter)
- Thứ tự lớp trên trang Gốc: ảnh trang → (khi bật AI) tô khối chữ `NHAN_DIEN_TO` → vùng (màu theo bảng phase 1) → nhãn tên (khi chọn / rê) → tay nắm → khung quét.
- Nét đứt (bảo vệ, Crossing): `Shape::dashed_line` 4/3.
- Nhãn: nền màu loại, chữ trắng 10 px đậm, bo 2, nằm trên mép trên vùng 15 px.
- Con trỏ: di chuyển `Move`, tay nắm `ResizeNwSe/NeSw/Vertical/Horizontal`, vẽ `Crosshair`, cạnh `Default`.

## Related Code Files
- Modify: `gui_xoa_ghim/sua_vung.rs` (chọn nhiều, nhóm, xoá vùng vẽ, nhân bản, đổi loại, đổi phạm vi), `gui_xoa_ghim/khung_trang.rs` (`vung_tren_trang`: vẽ + tương tác mới), `gui_xoa_ghim/thanh_tren.rs` (biểu tượng bấm từng phần, cột lọc trang, khối Vùng, hàng tuỳ chọn), `gui_xoa_ghim/tests.rs`
- Reuse: `canh_keo_duoc`, `keo_vung` (mở rộng cho nhóm), `BoVung::bat_tat`, `them_ve`

## Implementation Steps
1. `sua_vung.rs`: `chon: Vec<ChonVung>` + API `chon_mot/chon_them/bo/chon_het/quet(rect, window)`, `xoa_vung_ve(ids)`, `doi_loai`, `nhan_ban`, `doi_pham_vi`, `keo_nhom`; test đơn vị cho từng hàm (gồm hoàn tác).
2. Biểu tượng: vẽ 3 biểu tượng bằng painter đúng toạ độ SVG; vùng bấm từng ô góc / dải cạnh / nửa − +.
3. `vung_tren_trang`: tách state máy kéo `Keo { Moi, DiChuyen, Nhom, DoiCo(canh), Quet{x0,y0,them} }`; tooltip kích thước theo chuột (`Order::Tooltip`).
4. Chuột phải: `Response::context_menu` trên trang (lấy vùng dưới chuột lúc mở).
5. Phím: Delete, Ctrl+A, Ctrl+Z, Esc (đã có một phần — gộp về 1 chỗ `phim()`).
6. Khối Vùng: combo, thanh trượt + ô số (widget phase 1), khoá theo loại.
7. Hàng tuỳ chọn + hộp xác nhận + toast.
8. kittest: mô phỏng kéo bằng `harness.drag_at` (hoặc `Event::PointerMoved/Button`) cho: vẽ, đổi cỡ, quét Window/Crossing, kéo nhóm, Delete + hoàn tác, chuột phải đổi phạm vi.

## Success Criteria
- [ ] Mọi dòng bảng hành vi có test / kittest qua.
- [ ] Kết quả xoá ghim của bộ vùng lưu không đổi (đối chiếu Python).
- [ ] Ảnh kittest vùng (chọn 1, chọn nhiều, đang quét) khớp màu / viền / nhãn prototype.

## Risk Assessment
- Cảm ứng kéo trong egui khi trang có `zoom` lớn: dùng toạ độ tỉ lệ trang (như prototype) để không lệch.
- Kho: chọn nhiều chỉ ở RAM — không đổi định dạng kho.
- Va chạm phím Delete với ô nhập: chỉ xử lý khi không có widget đang nhận phím (`ctx.wants_keyboard_input()`).
