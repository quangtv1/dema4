---
phase: 1
title: "Nền tảng thiết kế + thanh menu chung"
status: completed
priority: P1
dependencies: []
---

# Phase 1: Nền tảng thiết kế + thanh menu chung

## Overview
Tạo bộ hằng số thiết kế (màu, cỡ) đúng CSS prototype, bộ biểu tượng SVG dùng lại nguyên văn từ prototype, các widget dùng chung (nút có biểu tượng, ô đường dẫn có ⊗ / nhãn, thanh trượt + ô số, ô lọc gộp, tooltip tức thì, thông báo nhanh). Đổi thanh menu chung của app thành **Tệp tin ▾ · Chức năng ▾ · Cấu hình · Trợ giúp ▾** + **^** bên phải.

## Bảng thông số
Trích từ `<style>` của prototype. Đơn vị: px logic egui (`pixels_per_point` giữ như app).

### Màu
| Tên hằng (Rust) | Giá trị | Dùng cho |
|---|---|---|
| `XANH` | `#2563eb` | nút chính, radio/checkbox, viền chọn, nhãn vùng vẽ |
| `XANH_DAM_CHU` | `#1f5fbf` | chữ link, file đang chọn, mục menu hiện tại |
| `XANH_THANH` | `#3b82f6` | phần đã kéo + viền nút tròn của thanh trượt |
| `XANH_NUT_MO` | `#9db7e6` | nút Xóa ghim khi mờ |
| `CHU` | `#1f2328` | chữ thường |
| `CHU_PHU` | `#6b7280` | chữ phụ / muted |
| `VIEN` | `#c9ccd1` | viền nút / ô nhập |
| `VIEN_KHUNG` | `#dcdfe3` | viền khung, thanh menu, đáy |
| `VIEN_NHAT` | `#e2e4e8` | vạch dọc, ranh giới |
| `NEN_MENU` | `#f7f7f7` | thanh menu |
| `NEN_NUT` / hover | `#f6f7f8` / `#eceef1` | nút thường |
| `NEN_TIEU_DE_KHUNG` | `#f2f3f5` | tiêu đề Gốc / Đích |
| `NEN_XEM` | `#e4e6ea` | nền khung xem trang |
| `NEN_DAY` | `#f7f8f9` | thanh đáy, cột trang thu nhỏ |
| `NEN_DS_TIEU_DE` | `#f3f4f6` | tiêu đề Danh sách, dải thu nhỏ |
| `NEN_DONG_CHON` / hover | `#dcebfa` / `#f1f5fb` | dòng file |
| `VUNG_GOC_CANH` | nền `rgba(76,175,80,.26)` (chọn `.34`), viền `#43a047` 1 px (chọn 2 px) |
| `VUNG_VE` | nền `rgba(37,99,235,.18)` (chọn `.26`), viền `#2563eb` |
| `VUNG_BAO_VE` | nền `rgba(217,70,239,.20)`, viền `#c026d3` **1,5 px nét đứt** |
| `VUNG_VO_DOI` | nền `rgba(249,115,22,.22)` (chọn `.32`), viền `#ea580c` 1,5 px |
| `QUET_WINDOW` | viền `#2563eb` liền, nền `rgba(37,99,235,.08)` |
| `QUET_CROSSING` | viền `#16a34a` đứt, nền `rgba(22,163,74,.08)` |
| `TOOLTIP` | nền `#1f2937`, chữ trắng 11,5 px, bo 4 |
| `NHAN_DIEN_TO` | nền `rgba(232,84,96,.30)`, viền `rgba(200,40,55,.75)` (khối chữ AI) |

### Kích thước
| Thành phần | Thông số |
|---|---|
| Chữ cơ bản | 12,5 px; menu 13 px; nhãn phụ 11–11,5 px |
| Thanh menu | cao 30; mục đệm ngang 12, biểu tượng–chữ 6; dropdown rộng ≥ 230, mục đệm 6×14, bo 4, bóng `0 6 18 .16` |
| Nút thường `.btn` | cao 26, đệm ngang 10, viền 1, bo 3 |
| Nút biểu tượng `.ibtn` | 28×26 (nhỏ 22×22); nút ◂ ▸ văn bản **44×22** |
| Nút có biểu tượng + chữ `.bnut` | **88×26**, biểu tượng–chữ 5 |
| Ô nhập `.fld` | cao 26, đệm 8, bo 3 |
| ⊗ tròn | Ø16, nền `#9ca3af` (hover `#dc2626`), cách mép phải 5 |
| Nhãn "mặc định" | 10,5 px, nền `#f1f3f5`, cách mép phải 7 |
| Danh sách | rộng mặc định **282** (kéo 200–520), dải thu nhỏ **34**; dòng cao 24, kẻ đứt `#e6e8eb` |
| Tiêu đề Danh sách | cao 32; hàng sắp xếp cao 28 |
| Thanh công cụ | đệm 12/12/10/14; biểu tượng trang 58×78; cột lọc trang cách 7 |
| Ranh giới thanh công cụ | rộng 4, viền trái/phải `#e2e4e8`, nền `#f5f6f8` |
| Thanh trượt | rãnh cao 4 bo 2; nút Ø14 viền 2 `#3b82f6`, bóng `0 1 3 .25`, hover phóng 1,15 |
| Ô số cạnh thanh trượt | 58×22, viền `#d0d4d9`, bo 4, nền `#f8f9fa` (focus viền xanh + quầng 2) |
| Tiêu đề khung Gốc / Đích | cao 30, chữ giữa (đệm ngang 70 để tránh nút) |
| Cột Trang thu nhỏ | rộng **150** (+1 viền); ảnh 96×136, cách 14 |
| Khung xem | trang cách nhau 18, đệm trên 6 dưới 30 |
| Thanh đáy | cao 40, đệm ngang 12 |
| Nút Xóa ghim | cao 28, đệm 16, chữ đậm; nút ▾ đệm 8, cách 1 |
| Hộp thoại | bo 6, bóng `0 12 40 .3`, nền mờ `rgba(0,0,0,.28)`; Cấu hình rộng 820 |
| Tay nắm vùng | 9×9, viền 1,5 `#1f5fbf`, lệch ra ngoài 5 |
| Nhãn tên vùng | 10 px đậm 600, đệm 0×4, nằm trên vùng 15 px; hiện khi chọn / rê |

## Requirements
- Functional:
  - Module hằng số `gui_xoa_ghim/thiet_ke.rs` (màu + cỡ ở bảng trên), không rải số ma thuật trong code màn.
  - Bộ biểu tượng: chép **nguyên SVG** từ prototype vào `dema4_rs/assets/xoa_ghim/*.svg` (tệp tin, chức năng, cấu hình, trợ giúp, caret, PDF, thư mục, ◂ ▸, vừa ngang, vừa cao, khiên, lớp, đồng hồ, mắt, …), nạp bằng `egui_extras` (feature `svg` đã bật) qua `include_image!`; tô màu bằng `tint` khi cần.
  - Biểu tượng trang giấy Góc / Cạnh / Tùy biến: vẽ bằng `Painter` theo đúng toạ độ SVG prototype (cần vùng bấm riêng từng góc / cạnh / nửa − +).
  - Widget dùng chung trong `gui_xoa_ghim/widget.rs`:
    - `nut_bieu_tuong_chu(icon, chu, rong=88)`;
    - `o_duong_dan(text, trailing: ⊗ | nhãn "mặc định" | nhãn "ghi đè")` — chỉ đọc, chữ cắt `…`, tooltip đầy đủ;
    - `thanh_truot(value, range, khoa) -> Response` + `o_so(value, hau_to)` (DragValue kiểu prototype);
    - `o_loc_gop(trang_thai, so_trang)` (TextEdit + nút ▾ + popup);
    - `tooltip_ngay(resp, text)` — hiện **không trễ**, kể cả nút mờ (đặt `interaction.tooltip_delay = 0` cục bộ hoặc vẽ tay vào `Order::Tooltip`);
    - `thong_bao_nhanh(text)` (toast 2,4 s, đáy giữa màn).
  - Thanh menu chung (sửa `gui.rs::thanh_menu`):
    - **Tệp tin ▾** đầu tiên: Mở file PDF… (Ctrl+O), Mở thư mục… (Ctrl+Shift+O), Gần đây ▸, —, Mở thư mục kết quả, —, Xuất / Nhập phiên làm việc…, —, Đóng thư mục.
    - **Chức năng ▾**: chỉ danh sách chức năng (mục hiện tại đậm xanh). Bỏ "Gần đây", "Xuất / Nhập phiên" khỏi đây (đã chuyển sang Tệp tin).
    - **Cấu hình** (mở hộp, ở màn Xóa ghim mở thẳng thẻ Xóa ghim), **Trợ giúp ▾** (Kiểm tra cập nhật…, Hướng dẫn sử dụng F1, Thông tin).
    - Góc phải: **^** thu gọn / mở thanh công cụ (chỉ màn Xóa ghim có; màn khác ẩn).
    - Menu Tệp tin có ở **đầu mọi module** và **chạy thật** theo từng module (người dùng chốt):

      | Module | Mở file PDF… | Mở thư mục… | Gần đây | Mở thư mục kết quả | Xuất / Nhập phiên | Đóng thư mục |
      |---|---|---|---|---|---|---|
      | Trang chủ | mở Tách với file | chọn chức năng như ô Trang chủ | ✓ | — mờ | — mờ | — mờ |
      | Đếm trang A4 | đếm 1 file | ✓ | ✓ (Đếm) | — mờ (không ghi file) | — mờ | ✓ |
      | Tách văn bản | ✓ (như kéo thả 1 PDF) | ✓ | ✓ | ✓ | ✓ | ✓ |
      | Sinh mục lục | ✓ | ✓ | ✓ | ✓ | — mờ | ✓ |
      | Bóc tách dữ liệu | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
      | Xóa ghim | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
      | Chuẩn hoá tên, Ký, AIP (đang phát triển) | — mờ | — mờ | ✓ | — mờ | — mờ | — mờ |

      Mỗi ô ✓ gọi đúng hàm mở có sẵn của module (kiểm từng module khi làm; ô nào module chưa có hàm thì mờ + tooltip lý do). Mục mờ luôn có tooltip.
- Non-functional: **chỉ màn Xóa ghim** dùng Segoe UI (người dùng chốt): nạp `segoeui.ttf`/`segoeuib.ttf` (Windows; macOS dùng phông hệ thống tương đương, fallback Arial) vào `FontFamily::Name("xoa_ghim")`, đặt `TextStyle` của màn này sang family đó trong `ManXoaGhim::ve`; các màn khác giữ phông hiện tại. Không thêm crate mới.

## Architecture
- `gui_xoa_ghim/thiet_ke.rs`: `pub const` màu (`Color32::from_rgba_unmultiplied`), cỡ (`f32`).
- `gui_xoa_ghim/bieu_tuong.rs`: `fn svg(ten) -> egui::Image` + painter cho biểu tượng trang giấy.
- `gui_xoa_ghim/widget.rs`: widget dùng chung (trả `Response`, tự `widget_info` cho kittest).
- `gui.rs`: `thanh_menu` tách `menu_tep_tin(ui, man)` dùng chung; hành động gửi qua enum `ViecTepTin` để mỗi màn tự xử lý (Xóa ghim, Tách, Bóc tách).

## Related Code Files
- Create: `dema4_rs/src/gui_xoa_ghim/thiet_ke.rs`, `.../bieu_tuong.rs`, `.../widget.rs`, `dema4_rs/assets/xoa_ghim/*.svg`
- Modify: `dema4_rs/src/gui.rs` (thanh_menu, menu Tệp tin theo module, nạp phông riêng), `dema4_rs/src/gui_xoa_ghim.rs` (mod mới), các màn có mục Tệp tin: `gui_tach.rs`, `gui_boc_tach.rs`, `gui_muc_luc.rs`, `dem_pdf.rs` (chỉ nối hàm mở có sẵn)
- Delete: phần tab Tệp tin/Xem/Chỉnh sửa/Cài đặt + `TabCongCu` trong `gui_xoa_ghim.rs`, `thanh_tren.rs` (commit 6ad178b)

## Implementation Steps
1. Trích bảng màu/cỡ → `thiet_ke.rs`; viết test nhỏ đảm bảo hằng khớp chuỗi hex trong bảng (đọc lại bảng bằng tay khi review).
2. Chép SVG từ `prototype-xoa-ghim.html` (đối tượng `IC`, `ICF`, các `<svg>` inline) ra `assets/xoa_ghim/`; tên file kebab-case theo nghĩa.
3. Viết `bieu_tuong.rs`, `widget.rs`; kittest cho từng widget (tên truy cập, trạng thái mờ, tooltip).
4. Sửa `thanh_menu`: thêm Tệp tin, dời Gần đây / Xuất–Nhập phiên; ^ ở phải; xoá tab cũ của màn Xóa ghim.
5. Phông riêng màn Xóa ghim; kiểm các màn khác không đổi phông (ảnh kittest trước / sau giống nhau).

## Success Criteria
- [ ] Menu chung đúng thứ tự, đúng mục, đúng phím tắt hiển thị; dropdown khớp kích thước bảng.
- [ ] Mọi màn khác vẫn mở / chuyển được; test kittest các màn qua.
- [ ] Widget dùng chung có test; tooltip ◂ ▸ hiện ngay cả khi nút mờ.

## Risk Assessment
- Menu Tệp tin chạy thật ở mọi module chạm vào nhiều màn → làm theo bảng, test từng mục ở từng màn.
- `include_image!` SVG cần loader — đã có `egui_extras::install_image_loaders`? Kiểm khi làm; nếu chưa thì gọi trong `cai_giao_dien`.
