---
phase: 9
title: "Kiểm thử, đối chiếu prototype, tài liệu, phát hành"
status: in-progress
priority: P1
dependencies: [1, 2, 3, 4, 5, 6, 7, 8, 10]
---

# Phase 9: Kiểm thử, đối chiếu prototype, phát hành

## Overview
Chứng minh "giống 100%" bằng ảnh + checklist, chạy toàn bộ test, cập nhật tài liệu / hướng dẫn / ảnh, phát hành bản mới (cần người dùng đồng ý trước khi gắn tag).

## Đối chiếu ảnh
- Test bỏ qua mặc định `chup_man_xoa_ghim_doi_chieu` (env `DEMA4_MAU_XG=../temp/Ho_so_mau`, 1600×900, ppp 1.0) xuất 4 ảnh cùng trạng thái với prototype: `trong`, `mo`, `cau-hinh`, `tep-tin` → `plans/261126-1007-…/reports/doi-chieu/`.
- Script Python ghép từng cặp (prototype | egui) cạnh nhau + lớp chồng 50% → `doi-chieu-<ten>.png` để duyệt bằng mắt.
- Dữ liệu mẫu giả (HS_001…), **không** dùng tài liệu thật (repo công khai).

## Checklist đối chiếu prototype
Mỗi mục ✓ khi đúng vị trí, kích thước (±2 px), màu, chữ, hành vi.

**Thanh menu**
- [ ] Tệp tin ▾ · Chức năng ▾ · Cấu hình · Trợ giúp ▾; biểu tượng xám; ^ bên phải
- [ ] Dropdown Tệp tin đủ 7 mục + 3 vạch, phím tắt bên phải
- [ ] Tệp tin ở đầu **mọi module**, mỗi mục chạy đúng bảng phase 1 (mục mờ có tooltip)
- [ ] Hộp Cấu hình có thẻ Xóa ghim ở mọi màn
**Danh sách**
- [ ] Mặc định mở rộng; thu nhỏ → dải 34 px chữ dọc + ô đếm; kéo 200–520; bấm đúp 282
- [ ] Không hamburger; "Danh sách (k/n)" + "đang hiện m"; nút thu nhỏ
- [ ] Sắp xếp Tên / Trang 1 cột; ô tick tất cả; dòng 24 px kẻ đứt; chọn nền `#dcebfa` chữ xanh
- [ ] 4 biểu tượng trạng thái + tooltip; ô lọc gộp (6 trạng thái + gõ số trang)
- [ ] Lọc nâng cao thu gọn mặc định; A0–A7, Ngang / Dọc, dải trang, Áp dụng / Bỏ lọc, "· đang lọc"
**Thanh công cụ**
- [ ] 2 nửa, ranh giới trùng ranh giới Gốc | Đích (3 bề rộng danh sách)
- [ ] Biểu tượng Góc / Cạnh / Tùy biến đúng hình, bấm từng phần, màu bật
- [ ] Cột Tất cả / Trang lẻ / Trang chẵn / Từng trang (+ Vô đối) theo vùng chọn / đang vẽ
- [ ] Vùng (combo) · Rộng / Cao / Ngưỡng: rãnh 4 px, phần kéo xanh, nút tròn 14, ô số 58×22 có đơn vị
- [ ] Đầu vào + ⊗ + [File] [Thư mục] 88×26
- [ ] File đích: combo kiểu ghi + đường dẫn rút gọn + mặc định / ⊗ + [Thư mục]
- [ ] Nhận diện layout (+ tải mô hình) · Tô nền trắng · Xóa vùng vẽ · Hoàn tác (xuống dòng gọn)
**Gốc / Đích**
- [ ] Tiêu đề giữa; ◂ ▸ 44×22 luôn hiện + tooltip tức thì; (i/n); › thu gọn Đích
- [ ] Cột thu nhỏ chỉ khi đã mở; ảnh 96×136, trang hiện tại viền + số xanh
- [ ] Khung xem Gốc = Đích; vừa ngang chính xác
- [ ] Cuộn đồng bộ; Liên tiếp / Từng trang
- [ ] Vùng: màu 4 loại, nhãn, 8 tay nắm (cạnh 1), chọn nhiều ẩn tay nắm
- [ ] Quét Window (xanh liền) / Crossing (xanh lá đứt); tooltip kích thước khi kéo
- [ ] Chuột phải: 2 menu đúng mục
- [ ] Màn trống: 2 biểu tượng + logo Sao Mai → Trang chủ
**Thanh đáy**
- [ ] Zone chung/riêng chữ thường; điều hướng giữa; vừa ngang / cao có trạng thái; tỉ lệ; − +
- [ ] ▶ Xóa ghim (n) + ▾ góc phải
**Hộp thoại**
- [ ] Cấu hình (4 nhóm, nhãn phạm vi, mô hình AI 4 trạng thái); dòng Lô 10 trang ghi "Nhanh hơn, tốn RAM hơn" (khác chữ prototype — đã chốt)
**AI**
- [ ] Nhận diện layout bật: khối chữ tô đỏ dưới lớp vùng; vùng xoá tránh khối; Vô đối xoá đè
- [ ] 4 bước đối chiếu Python đạt; không mô hình / không mạng vẫn xoá ghim thường
- [ ] Xác nhận Xóa vùng vẽ; tiến độ / kết quả; F1; toast

## Kiểm thử khác
- [ ] `cargo test` (≥ 381 + mới) — Windows (CI) + macOS.
- [ ] Đối chiếu Python 266 trang (AI tắt) vẫn khớp từng điểm ảnh.
- [ ] Lô 10 trang bật / tắt cho **cùng** file kết quả (từng byte ảnh trang).
- [ ] Hiệu năng: thư mục 500 file / 5000 trang — mở, lọc, cuộn không giật.
- [ ] Kho 2.5.0 → bản mới → mở lại bản 2.5.0 (đọc ngược).
- [ ] Các màn khác: kittest + mở tay (menu Tệp tin mới).

## Tài liệu & phát hành
- README (mục Xóa ghim), `docs/huong-dan/tao-huong-dan.js` chương 7 + ảnh `docs/huong-dan/anh/07-*.png`, `docs/anh/xoa-ghim.png` (chụp từ dữ liệu giả), `tro_giup.rs` HD_XOA_GHIM.
- Phiên bản: **2.6.0** (đổi menu chung + kho có trường mới). Commit theo conventional commits, không nhắc AI / mã giai đoạn.
- Gắn tag + phát hành: **chỉ khi người dùng đồng ý**.

## Success Criteria
- [ ] Checklist 100% ✓, ảnh đối chiếu được người dùng duyệt.
- [ ] CI xanh 2 nền tảng.
