---
phase: 7
title: "Cấu hình thẻ Xóa ghim + tải mô hình AI"
status: completed
priority: P1
dependencies: [1, 6]
---

# Phase 7: Cấu hình thẻ Xóa ghim + tải mô hình AI

## Overview
Thêm thẻ **Xóa ghim** vào hộp Cấu hình chung (`gui_tach/hop_cau_hinh.rs`) đúng ảnh `anh-cau-hinh.png`: 4 nhóm dạng thẻ có biểu tượng + nhãn phạm vi lưu, mỗi dòng "biểu tượng · tên + mô tả · điều khiển phải". Làm luồng **tải mô hình AI** (nút, tiến độ, huỷ, kiểm SHA-256, chọn file ngoại tuyến). Phần **chạy nhận diện** (YOLO DocLayNet → khối chữ bảo vệ + tô đỏ khi xem) ở [phase 10](./phase-10-ai-nhan-dien-layout.md) của kế hoạch này.

## Bố cục hộp (rộng 820, nền thân `#f8f9fa`)
- Tiêu đề: ⚙ Cấu hình … ✕. Hàng thẻ có biểu tượng: Đếm trang · Tách văn bản · Mục lục · Bóc tách · **Xóa ghim** (gạch xanh 2 px).
- Lưới 2 cột, mỗi nhóm là thẻ viền `#e5e7eb` bo 6, tiêu đề đậm 13 px + nhãn phạm vi bên phải (**theo thư mục** nền `#eff6ff` chữ `#1d4ed8`; **chung** nền `#f1f5f9` chữ `#475569`).

| Cột | Nhóm (phạm vi) | Dòng (biểu tượng — tên — mô tả — điều khiển) |
|---|---|---|
| Trái | 🛡 Nhận diện & bảo vệ (theo thư mục) | 📄đỏ Nhận diện layout — "Tìm khối chữ, bảng; không xoá vào nội dung" / "Cần tải mô hình AI (37,5 MB)" — [⤓ Tải mô hình][…] ☐ · ✒ Bảo vệ nét đen — "Không coi điểm rất tối là vết bẩn — giữ nét chữ" ☑ · ◎đỏ Bảo vệ dấu đỏ / chữ ký xanh — "Giữ con dấu, chữ ký mực màu" ☐ |
| Trái | ⏱ Hiệu năng & lưu (chung) | ≋ Xử lý theo lô 10 trang — "Máy yếu / file lớn: đỡ tốn bộ nhớ" ☑ · 🕒 Tự lưu vùng — "0 = lưu ngay khi sửa" [0 phút] |
| Phải | ⤓ Xuất file (theo thư mục) | ▦ Độ phân giải (DPI) — "Gốc = giữ như bản quét" [Gốc▾] · ⇲ Chất lượng nén JPEG — "Thấp hơn = file nhỏ hơn" [90%▾] · ◐ Nén đen trắng — "Trang đen trắng / xám ghi gọn hơn" ☐ · 🪣 Tô nền trắng — "Tô vết bằng màu trắng thay màu nền đo được" ☐ · ⚠cam Ghi đè file gốc — "Không tạo thư mục _xoaghim, thay thẳng file nguồn" ☐ (+ khung cảnh báo vàng khi bật) |
| Phải | 👁 Hiển thị (chung) | ▭ Kiểu xem [Liên tiếp▾] · ▦ Cột trang thu nhỏ ☑ · ◫ Khung Đích — "Xem kết quả bên phải" ☑ · 🔗 Cuộn Gốc \| Đích cùng nhau ☑ |

- Chân: [Khôi phục mặc định] · chú thích "theo thư mục: lưu cùng vùng của thư mục đang mở · chung: mọi thư mục" · [Xong] (xanh).
- Bấm cả dòng (nhãn) = bật / tắt ô tick.
- Chưa mở thư mục: nhóm "theo thư mục" mờ + dòng "Mở thư mục để chỉnh".

## Mô hình AI (trạng thái ở nút cạnh "Nhận diện layout")
| Trạng thái | Hiển thị |
|---|---|
| Chưa có | [⤓ Tải mô hình] (tooltip "Tải 37,5 MB từ Release của dema4, kiểm SHA-256 rồi lưu vào thư mục dữ liệu; dùng ngoại tuyến sau đó") + […] (chọn file có sẵn) · ô tick mờ |
| Đang tải | [Hủy · 45%] + thanh tiến độ dưới mô tả "16,9 / 37,5 MB" |
| Kiểm tra | "Đang kiểm tra tệp (SHA-256)…" |
| Sẵn sàng | "✓ Sẵn sàng" xanh mờ (tooltip tên file) · ô tick dùng được, **tự bật** lần đầu |
| Lỗi (mạng / SHA sai) | chữ đỏ ngắn + nút thử lại |

- Nguồn: Release riêng repo dema4, tag `mo-hinh-xoa-ghim-1`, file `yolov12s-doclaynet.onnx`; SHA-256 `b5c0defaf2748d3a02438311be854a12a4b47698633ba7f91a08567197fe927a`; tải bằng `ureq` (đã có) ở luồng nền, ghi `*.part` rồi đổi tên vào `<thư mục dữ liệu>/mo_hinh/`; huỷ = xoá `.part`.
- Chọn file: `rfd` chọn `.onnx`, kiểm cùng SHA rồi chép vào thư mục mô hình.

## Lưu trữ
| Tuỳ chọn | Nơi lưu |
|---|---|
| Nhận diện layout, nét đen, đỏ/xanh, Tô nền trắng | `TuyChon` (thêm `nhan_dien: bool`) trong `ThuMuc` — vào `ma_cau_hinh` |
| DPI, Nén, Nén đen trắng | `CauHinhXuat` (đã có) |
| Ghi đè | `ThuMuc.ghi.ghi_de` (phase 6) |
| Lô 10 trang, Tự lưu (phút), Kiểu xem, Cột thu nhỏ, Khung Đích, Cuộn cùng | `cau_hinh` app (chung) — mục `xoa_ghim` mới |

- **Lô 10 trang** (đã chốt): bật = xử lý song song tối đa 10 trang của 1 file (nhanh hơn, tốn RAM hơn); tắt = từng trang (như hiện nay). Mô tả dòng đổi thành **"Nhanh hơn, tốn RAM hơn"** (thay "Máy yếu / file lớn: đỡ tốn bộ nhớ" của prototype cho đúng nghĩa). Cài trong `xoa_ghim_pdf`: vẽ + xoá 10 trang song song (rayon), ghép vào PDF đích theo đúng thứ tự trang.
- **Tự lưu N phút**: 0 = ghi kho ngay (hiện tại); > 0 = gom thay đổi, ghi mỗi N phút + khi đổi file / đóng thư mục / đóng app / chạy.
- Đồng bộ 2 chiều: ô tick "Nhận diện layout", "Tô nền trắng" ở thanh công cụ ↔ hộp Cấu hình (cùng biến).

## Related Code Files
- Modify: `gui_tach/hop_cau_hinh.rs` (`TabCauHinh::XoaGhim` + vẽ thẻ), `cau_hinh.rs` (mục xoa_ghim chung), `xoa_ghim/xu_ly.rs` (`TuyChon.nhan_dien`), `xoa_ghim/kho.rs` (mã hoá TuyChon có phiên bản), `gui.rs` (mở Cấu hình đúng thẻ khi ở màn Xóa ghim)
- Create: `xoa_ghim/mo_hinh.rs` (tải / kiểm / chọn file / trạng thái)
- Reuse: biểu tượng phase 1, `ureq` (đã có). Thêm `sha2` làm phụ thuộc trực tiếp (đã có sẵn trong cây phụ thuộc qua `rsa` → không tải thêm mã mới).

## Implementation Steps
1. `mo_hinh.rs` + test (SHA đúng / sai bằng file giả nhỏ; huỷ xoá `.part`; không mạng → lỗi gọn).
2. Trường mới `TuyChon.nhan_dien`, mục `cau_hinh.xoa_ghim` + test đọc cấu hình cũ.
3. Thẻ Xóa ghim trong hộp Cấu hình đúng bảng; khôi phục mặc định.
4. Nối ô tick thanh công cụ ↔ hộp; link "tải mô hình" mở thẳng thẻ.
5. Tự lưu N phút + lô 10 trang.
6. kittest ảnh hộp (`anh-cau-hinh.png`); trạng thái tải mô phỏng bằng nguồn giả cục bộ.

## Success Criteria
- [ ] Ảnh kittest hộp Cấu hình khớp `anh-cau-hinh.png`.
- [ ] Tải thật thành công trên Windows + macOS; SHA sai bị từ chối; huỷ sạch.
- [ ] Không có mô hình / không mạng → Xóa ghim vẫn chạy bằng ngưỡng.

## Risk Assessment
- Release mô hình chưa tồn tại → tạo Release `mo-hinh-xoa-ghim-1` (hỏi người dùng trước khi đăng).
- Giấy phép AGPL của mô hình (đã chấp nhận trong kế hoạch cũ) — ghi NOTICE kèm.
