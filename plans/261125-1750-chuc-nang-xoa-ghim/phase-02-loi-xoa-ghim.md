---
phase: 2
title: "Lõi xoá ghim (khớp Python) + đối chiếu"
status: completed
priority: P1
dependencies: []
---

# Phase 2: Lõi xoá ghim + đối chiếu bản Python

## Overview
Module `xoa_ghim/`: vùng + xử lý ảnh, đúng thuật toán `core/processor.py` (XoaGhim), Rust thuần, chưa AI (AI cắm vào sau qua danh sách hình chữ nhật cần bảo vệ).

## Requirements
- **Vùng**: 4 góc 130×130 px @120 DPI, 4 cạnh sâu 50 px @120 DPI × 100% chiều dài (co giãn theo DPI), vùng vẽ tay theo tỉ lệ trang; ngưỡng mặc định góc trên trái 3, góc khác 5, cạnh 8; góc / cạnh nới 10 px ra ngoài mép.
- **Loại vùng**: xoá / bảo vệ / **xoá vô đối** (bỏ qua mọi bảo vệ, chạy trước). Áp trang: tất cả / lẻ / chẵn / 1 trang.
- **Xử lý vùng**: nền = trung vị vùng giữa-phải trang hoặc nền trắng; vết = xám tối hơn nền > ngưỡng; **"Bảo vệ nét đen"** (< 50 góc / cạnh, < 80 khác) là **tuỳ chọn riêng, mặc định bật** như Python (không gắn với AI — red team #3); **"Bảo vệ dấu đỏ / chữ ký xanh"** tuỳ chọn, mặc định tắt, giống nhau ở xem trước và xuất; đóng 2 lần + giãn 3 lần, nhân elip `max(5, 5·dpi/120)` lẻ; tô nền.
- Vùng bảo vệ (tay; sau này + AI) nới 5 px → trừ khỏi vùng xoá; mảnh < 100 px² bỏ.
- ~~**C1**~~ trừ lại nét đen + đỏ / xanh sau đóng / giãn — **đã thử, bỏ** (2026-11-25): trên bộ mẫu giữ thêm 586 nghìn điểm / 233 trang nhưng đó phần lớn là **lõi đen vết ghim / gỉ** (cũng < 50) và chữ còn lại rỗng viền → kém hơn Python. Giữ đúng cách Python (bảo vệ chỉ ở điểm mầm, giãn phủ lên). Chữ sát ghim: để C3 (lọc theo mảng liền) ở GĐ8.
- **N4**: chỉ đọc / xử lý phần ảnh trong vùng (cần cho xem trước ở độ phân giải gốc).
- Đóng / giãn nhân elip cài theo đoạn chạy từng hàng (không chậm ở nhân 13 px @300 DPI). Không OpenCV.

## Đối chiếu (red team #1)
- Script Python (scratchpad, không vào repo) gọi `core/processor.py`, xuất cho mỗi trang mẫu: **ảnh trang đã vẽ** (PNG), vùng / tham số (JSON), **mặt nạ trước giãn + sau giãn**, ảnh kết quả. Rust đọc **đúng ảnh đó** (không tự vẽ lại — MuPDF hai bên khác bản) → so mặt nạ trước giãn (khớp ~tuyệt đối), mặt nạ sau giãn + ảnh kết quả (IoU mặt nạ ≥ 0,99).
- Vùng bảo vệ: Python dùng Shapely (bo tròn) ↔ Rust mặt nạ vuông → so IoU, không so % điểm.
- Bộ mẫu tối thiểu (user): ≥ 30 trang có ghim, ≥ 2 máy scan, có màu + xám + đen trắng, có dấu đỏ, chữ sát ghim.
- **Bộ mẫu đã có** (user, 2026-11-25): `temp/mau_xoaghim/` (không vào repo) — 108 PDF / 266 trang, mỗi trang 1 ảnh JPEG màu 300 DPI, không lớp chữ; kèm `_clean.pdf` bản Python (DPI Gốc, JPEG 90%, AI bật) + `.xoaghim.json` (vùng chung: góc trên trái + 4 cạnh, ngưỡng 5; 64 vùng tay ở 24 file — xoá, 1 vùng vô đối, lọc trang `all` / `none`; `per_file_zones` theo trang). Có dấu đỏ, chữ sát ghim.
  - `_clean.pdf` đã nén JPEG lại → chỉ dùng để **nhìn** đối chiếu; so số vẫn qua ảnh / mặt nạ script Python xuất.
  - Thiếu: ảnh xám / đen trắng, máy scan thứ 2 → test các nhánh đó bằng trang tổng hợp; bổ sung mẫu thật khi có.
  - Đầu vào chỉ là **PDF scan thuần ảnh** (user chốt 2026-11-25): không có lớp chữ OCR → không xử lý / không test trường hợp có lớp chữ.
  - Bộ đọc `.xoaghim.json` (nhập vùng từ bản Python) **không** trong phạm vi — chỉ dùng làm cấu hình cho test đối chiếu.

## Related Code Files
- Create: `dema4_rs/src/xoa_ghim.rs`, `dema4_rs/src/xoa_ghim/vung.rs`, `dema4_rs/src/xoa_ghim/xu_ly.rs`
- Modify: `dema4_rs/src/main.rs` (khai báo module)

## Tests (viết trước)
- Nhân elip / đóng / giãn khớp OpenCV trên ma trận nhỏ; trung vị; vùng → pixel ở 120 / 200 / 300 DPI khớp `to_pixels_with_edge_padding`; nét đen (bật) / đỏ xanh (bật) giữ, kể cả sau giãn (C1); ngoài vùng không đổi; vùng bảo vệ không bị đụng; xoá vô đối xoá cả trong vùng bảo vệ; lẻ / chẵn / 1 trang.
- Đối chiếu (`--ignored`, đường dẫn qua biến môi trường) như trên.

## Success Criteria
- [x] Khớp Python: **khớp từng điểm ảnh 266/266 trang** (IoU 1,0), vùng dựng như hàng loạt XoaGhim (vùng chung ngưỡng theo thanh trượt, vùng riêng trang **ngưỡng cố định 7** như code Python). Rust ~9 s / 266 trang gồm giải mã PNG (song song).
- [x] C1: đo → kém hơn → bỏ (xem trên).

## Kết quả
- Code: `dema4_rs/src/xoa_ghim.rs`, `xoa_ghim/{vung,hinh_thai,xu_ly,doi_chieu}.rs`; dev-dep `png` (đã có trong cây phụ thuộc). 14 test đơn vị + 1 test đối chiếu (`DEMA4_DOI_CHIEU_XG=<thư mục> cargo test doi_chieu -- --ignored`).
- Script sinh dữ liệu đối chiếu: `temp/sinh_doi_chieu_xoaghim.py` (thư mục `temp/` không vào git): `python3 temp/sinh_doi_chieu_xoaghim.py <repo xoaghim> temp/mau_xoaghim <thư mục ra>` (~1,7 GB PNG).
- Khớp OpenCV 4.12: xám dùng hệ số 15 bit (9798 / 19235 / 3735), không phải 14 bit; H / V khớp tuyệt đối; S lệch 1 ở 15 / 32 896 cặp (v, chênh) → chỉ ảnh hưởng bảo vệ đỏ / xanh (mặc định tắt; hàng loạt Python cũng tắt).
- Vùng bảo vệ: Rust dùng mặt nạ điểm (lề 5 bo góc, bỏ mảnh < 100) thay Shapely — bộ mẫu không có vùng bảo vệ nên chưa đối chiếu được, chỉ test đơn vị.
- Phát hiện ở Python: xem trước dùng ngưỡng vùng riêng theo thanh trượt (5) nhưng xuất hàng loạt dùng 7 → xem trước ≠ xuất. Bản Rust: 1 ngưỡng mỗi vùng, dùng cho cả hai.
