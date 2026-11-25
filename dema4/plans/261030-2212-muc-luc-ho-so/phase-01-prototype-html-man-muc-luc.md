---
phase: 1
title: Prototype HTML man Muc luc
status: completed
priority: P1
dependencies: []
effort: 1 ngày
---

# Phase 1: Prototype HTML màn Sinh mục lục

## Overview
Dựng màn Mục lục trong prototype HTML hiện có (nguồn chân lý UI) để chủ sản phẩm duyệt bố cục trước khi code.

## Bố cục đã duyệt (31/10) — nguồn chân lý cho phase 3
Tên: menu **Sinh mục lục**; tiêu đề cửa sổ + giới thiệu **Sinh mục lục hồ sơ**.
- **Dòng 1**: `File biên mục:` [đường dẫn] [Chọn] │ khối phải: `Sheet` ▾ · `Dòng tiêu đề` [n].
- **Dòng 2**: `Mẫu mục lục:` [4 mẫu nhúng + mẫu riêng ▾] [Chọn] │ khối phải: [Ghép biến…]. Ô mẫu dài bằng ô file, các nút thẳng cột.
- **Khung chính**: thanh tab bên trái rộng đúng bằng sidebar (2 tab chia đôi, co giãn theo sidebar): `Mẫu (x/y biến)` · `Kết quả (z)`; bên phải thanh tab = cảnh báo `⚠ n biến chưa ghép · m hồ sơ cần xem (…)` (bấm → nhảy hồ sơ lỗi). Chưa có dữ liệu: sidebar trống + giới thiệu 3 bước ở khung phải.
- **Sidebar kéo rộng/hẹp** (220–600 px, nhớ độ rộng); cột Biến cố định 128 px, phần dư cho cột Lấy từ.
- **Tab Mẫu**:
  - Sidebar = bảng `Biến | Lấy từ | !` chia nhóm *Thông tin hồ sơ* / *Bảng văn bản* / *Tự động*. Chip biến tô theo loại (slate / xanh rêu / xám; chưa ghép = vàng cam), cùng màu với chip trên trang mẫu. Dòng căn giữa dọc.
  - Ô **Lấy từ** sửa tại chỗ: bấm → khung xổ ngay dưới dòng: ô gõ (Enter = giá trị cố định), danh sách cột (gõ để lọc), "Từ kết quả Tách", "Bỏ ghép"; Esc / bấm ra ngoài = đóng. Kiểu hiển thị: cột = nền slate nhạt không viền; giá trị cố định = như ô nhập (trắng, viền); tự động = xám, không sửa; chưa ghép = vàng.
  - Chân sidebar: hàng **Nhóm theo** (thẳng cột với bảng, nền riêng, ô chọn không viền + biểu tượng nhóm): mọi cột Excel kèm số giá trị khác nhau `Tên cột (n)` (không tính dòng tiêu đề). Đổi → số hồ sơ / tab Kết quả / dòng đáy cập nhật.
  - Khung phải: thanh 27 px (= hàng tiêu đề bảng) `Hàng mẫu: bảng b, dòng d` + chú thích 4 màu + zoom; trang mẫu với chip biến. Rê chuột dòng sidebar ↔ chip trên trang: dòng sidebar nền xanh nhạt, chip trên trang vàng (hai chiều). Bấm chip → mở sửa dòng đó.
- **Tab Kết quả**: sidebar `# | Hồ sơ | Số dòng` (chỉ đánh dấu ⚠ ngoại lệ), chân = ô Tìm hồ sơ; thanh 27 px: ◀ i / n ▶ · `Mục lục hồ sơ X · y dòng · z trang` (chữ thường) · zoom 50–200 %; trang xem trước, dải cảnh báo vàng trên trang khi hồ sơ có lỗi.
- **Dòng đáy**: ⓘ `Hồ sơ: n; Văn bản: m` (đang chạy: `Đang sinh i/n hồ sơ · file · còn …`) │ phải: ☑ Kèm PDF · nút xanh **Sinh mục lục** (đang chạy → nút đỏ **Dừng**). Không có thanh tiến độ trên màn.
- **Popup**: *Đang sinh mục lục* (thanh tiến độ i/n · %, Đang tạo, Mẫu + Nhóm theo, Đã xong DOCX·PDF, Lỗi, Ghi ra, Thời gian, Dừng); *Đã sinh mục lục* (Hồ sơ, Văn bản, File ra, Lỗi, Ghi ra, Thời gian, dòng thống kê, Mở thư mục · Mở log · Đóng).
- **Hộp Ghép biến…** (920 px): Nhóm theo ▾ + tóm tắt; bảng `! | Biến | Lấy từ | Giá trị — hồ sơ đang xem` dùng cùng ô Lấy từ; ghi chú biến tự động; Tự khớp lại · Huỷ · Lưu.
- **Cấu hình › tab Sinh mục lục**: thư mục ra (trống = `.sm-dema4/muc_luc/<tên file>` cạnh exe), mẫu tên `MLHS_{so_ho_so}`, Kèm PDF, File đã có (Ghi đè/Bỏ qua), Chạy tiếp, PDF bằng.
- Hash xem nhanh: `#muc-luc`, `#muc-luc-mau`, `#muc-luc-mau-sua`, `#muc-luc-mau-ro`, `#muc-luc-xem`, `#muc-luc-loi`, `#muc-luc-ghep`, `#muc-luc-ghep-chon`, `#muc-luc-tao`, `#muc-luc-xong`, `#cau-hinh-muc-luc`.

## Related Code Files
- Modify: `plans/261027-1736-tach-van-ban-tu-dong/prototype-tach-van-ban.html`

## Implementation Steps
1. Đọc cấu trúc prototype (menu, `.phu`/`.popup`, bảng cột trái, `hl-*`, Cấu hình tab).
2. Thêm khối HTML + CSS dùng lại class sẵn có; JS chuyển trạng thái theo hash.
3. Chụp ảnh từng trạng thái (Chrome headless `--screenshot`) gửi duyệt; sửa theo góp ý.

## Tests / Validation
- Mở từng hash không lỗi console; ảnh chụp 6 trạng thái.
- Duyệt của chủ sản phẩm = điều kiện sang phase 3 (phase 2 chạy song song được).

## Success Criteria
- [x] 6 trạng thái hiển thị đúng, đồng bộ phong cách Tách (ảnh: `anh-prototype/`).
- [x] Chủ sản phẩm duyệt bố cục cuối (31/10) sau nhiều vòng góp ý — mô tả ở trên.
- [x] Chốt: thư mục ra mặc định = thư mục dữ liệu ẩn cạnh exe `.sm-dema4/muc_luc/<tên file dữ liệu>/`; mỗi lần 1 sheet.

## Risk Assessment
- Prototype phình to → chỉ thêm khối mới, không sửa màn cũ.
