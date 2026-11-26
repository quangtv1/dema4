---
phase: 2
title: "Thay đổi, Lưu / Huỷ / Mặc định, hỏi khi rời"
status: completed
priority: P1
dependencies: [1]
---

# Giai đoạn 2: Thay đổi theo chức năng, Lưu / Huỷ / Mặc định

## Tổng quan
Biết chức năng nào đang có thay đổi chưa lưu (chấm cam sidebar, nhãn tiêu đề), lưu / huỷ tất cả, khôi phục mặc định riêng chức năng đang xem, hỏi lại khi rời trang.

## Yêu cầu
- `doi(tab)`:
  - Đếm: `he_so, ranh_gioi, a0_lon, a0_lon_he_so, a0_nguong_mm` + ô gõ (`o_he_so`, `o_a0_he_so`) chưa khớp số.
  - Tách: `quet_trang, quet_ma, quet_xoay, nguong_trang, nguong_nghi, loai_ma, ma_index, tin_cay_xoay, thu_muc_ra, mau_ten, luu_nhan` + ô gõ ngưỡng / tin cậy.
  - Mục lục: `muc_luc.{thu_muc_ra, mau_ten, ghi_de, kem_pdf}`.
  - Bóc tách: cả `boc_tach` + khoá API (so với khoá lúc mở).
  - Xóa ghim: `xoa_ghim` (chung) + tuỳ chọn thư mục (`CauHinhThuMuc`).
  - Dữ liệu ứng dụng: không có thay đổi.
- Nhãn tiêu đề: "● n chức năng chưa lưu" (rê chuột: tên các chức năng) / "✓ Đã lưu".
- **Lưu** (Ctrl+S): khoá khi `loi()` khác rỗng (giữ thông báo ô đỏ hiện có) — báo ở nhãn "Sửa ô đang báo đỏ"; lưu xong ở lại trang, toast "Đã lưu n chức năng".
- **Huỷ thay đổi**: nháp = bản đang dùng (mọi chức năng), đồng bộ lại ô gõ.
- **Khôi phục mặc định**: chỉ chức năng đang xem, dùng hàm `mac_dinh_*` sẵn có (+ `TabXoaGhim::mac_dinh`); không lưu ngay — toast "…: đã về mặc định — bấm Lưu để giữ".
- Rời trang khi `doi` bất kỳ: hộp "Lưu thay đổi trước khi quay lại?" liệt kê chức năng — **Không lưu** · Ở lại · **Lưu và quay lại** (khoá khi có lỗi ô).

## Kiến trúc
- `gui_cau_hinh/phan.rs`: `pub fn chi_phan(tab, c: &CauHinh) -> CauHinh` (sao trường của tab vào `CauHinh::default()`); `mac_dinh_*` hiện có viết lại bằng nó khi trùng tập trường (DRY) — Mục lục / Bóc tách giữ hàm riêng vì mặc định chỉ một phần.
- `HopCauHinh` lưu `goc: CauHinh`, `khoa_goc`, `xg_goc` để so; `KetQuaTrang::Luu(CauHinh)` → app áp rồi gọi `hop.da_luu(&cfg_moi)` cập nhật `goc`.

## File
- Tạo: `dema4_rs/src/gui_cau_hinh/phan.rs`
- Sửa: `gui_tach/hop_cau_hinh.rs`, `gui_boc_tach/cau_hinh.rs` (nếu cần lấy tập trường), `gui_xoa_ghim/the_cau_hinh.rs` (`PartialEq` đã có), `gui.rs` (áp không đóng)

## Các bước
1. `chi_phan` + test: mỗi tab đổi 1 trường của tab khác → `doi` không bật nhầm.
2. Trạng thái gốc / nháp; nhãn, chấm cam.
3. Nút Lưu / Huỷ / Mặc định + Ctrl+S; toast (dùng `widget::toast`).
4. Hộp hỏi khi rời (dùng `khung_hop::hop` 440 px).

## Tiêu chí xong
- [ ] Test đơn vị `doi` cho 5 chức năng; test kittest: sửa Tách → chấm cam Tách, Lưu → hết chấm, `cfg` đổi, trang vẫn mở; Esc khi chưa lưu → hộp hỏi; Không lưu → đóng, `cfg` không đổi.
- [ ] Ô sai (mẫu tên thiếu `{n}`) khoá Lưu và "Lưu và quay lại".

## Rủi ro
- Trường mới thêm sau này quên vào `chi_phan` → có test liệt kê: mọi trường `CauHinh` hoặc thuộc 1 tab, hoặc nằm trong danh sách "trạng thái giao diện".
