# Kết quả code màn Xóa ghim theo prototype (26/11/2026)

## Đã làm (giai đoạn 1–8, 10; 9 trừ phát hành)

- **Thanh menu chung**: Tệp tin ▾ (Mở file PDF… / Mở thư mục… / Gần đây / Mở thư mục kết quả / Xuất · Nhập phiên / Đóng thư mục) đầu mọi chức năng; Chức năng bỏ Gần đây, Xuất / Nhập phiên; ^ thu gọn thanh công cụ (màn Xóa ghim).
- **Nền thiết kế**: `gui_xoa_ghim/thiet_ke.rs` (màu, cỡ từ CSS prototype), `bieu_tuong.rs` (chính SVG prototype), `widget.rs` (nút, ô đường dẫn ⊗ / nhãn, thanh trượt + ô số, tooltip ngay, toast); phông Segoe UI chỉ màn Xóa ghim (Windows).
- **Danh sách**: mở rộng mặc định, dải 34 px, kéo 200–520 (đúp: 282), sắp Tên / Trang, 4 trạng thái, lọc tên + lọc gộp (trạng thái / số trang), Lọc nâng cao thu gọn; lưu độ rộng.
- **Thanh công cụ 2 nửa** thẳng ranh Gốc | Đích: Góc / Cạnh bấm từng góc / cạnh, Tùy biến − / +, cột lọc trang theo vùng chọn (+ Vô đối), Vùng / Rộng / Cao / Ngưỡng; Đầu vào (⊗ File Thư mục), File đích (Nhân bản / Cạnh file gốc, mẫu đường dẫn, mặc định / ⊗ / ghi đè), Nhận diện layout + tải mô hình, Tô nền trắng, Xóa vùng vẽ (hỏi lại), Hoàn tác.
- **Gốc | Đích**: rộng bằng nhau, 2 vùng cuộn đồng bộ, tiêu đề cân giữa đường dẫn đầy đủ, ◂ ▸ 44×22 tooltip ngay, cột trang thu nhỏ chỉ khi đã mở, màn trống 2 nút lớn + logo → Trang chủ.
- **Vùng**: 4 màu, nhãn khi chọn, tay nắm theo cạnh kéo được, quét Window / Crossing, Shift, Ctrl+A, kéo nhóm, chuột phải (xoá, đổi loại, nhân bản, phạm vi), Delete ngay + toast, cỡ % cạnh con trỏ.
- **Thanh đáy**: Zone chung / riêng, trang, Liên tiếp / Từng trang, vừa ngang / cao, 25–150 %, − +, **Xóa ghim (n)** + ▾ Chạy lại mọi file đã tick.
- **Cấu hình › Xóa ghim**: 4 thẻ (theo thư mục / chung), tải / nhập mô hình có tiến độ, huỷ, lỗi; lưu theo thư mục vào kho, chung vào file cấu hình.
- **Backend** (gộp nhánh song song): `CauHinhGhi` (kiểu ghi, thư mục đích, ghi đè) bản ghi kho riêng — 2.5.0 vẫn đọc được; `dich_cua`, `ds_viec_ghi` bỏ `*_xoaghim.pdf`; trạng thái khi ghi đè qua `nguon_theo_ket_qua`, vùng riêng theo `id_goc`; **lô 10 trang song song** cho PDF giống hệt từng byte.
- **AI**: tải mô hình (`.part`, kiểm cỡ + SHA-256), ONNX khớp Python XoaGhim (3 trang, 14/14 hộp, lệch 0 px; ~0,25 s / trang A4 300 dpi); dùng khi xuất + xem trước (khung đỏ nhạt trên Gốc); lỗi mô hình → xoá như khi tắt.
- **Kiểm thử**: 408 test qua (thêm lọc / sắp xếp, xoá nhiều + hoàn tác, file đích 3 kiểu + 1 file, cấu hình theo thư mục); build release không cảnh báo. Ảnh đối chiếu 1600×900 (trống / mở / cấu hình) khớp bố cục prototype.
- **Tài liệu**: README, hướng dẫn chương 7 (docx), ảnh mới từ PDF mẫu giả (đường dẫn trung tính); phiên bản 2.6.0.

## Khác prototype còn lại

- Tab hộp Cấu hình chưa có biểu tượng, nhãn giữ tên cũ (Đếm trang A4 / Tách PDF / Sinh mục lục); chân hộp giữ Mặc định / Huỷ / Lưu chung của app (prototype: Khôi phục mặc định / Xong).
- Vẽ chữ egui ≠ trình duyệt (đã nêu trong báo cáo khả thi).

## Chưa làm / cần người dùng

- **Tag + phát hành v2.6.0** và **tải `yolov12s-doclaynet.onnx` lên release `mo-hinh-xoa-ghim-1`** (nút Tải mô hình trỏ tới đó; chưa có thì báo "chưa có trên máy chủ", vẫn nhập được file).
- Bản PDF của hướng dẫn chưa xuất lại (chỉ docx).
- Xuất / Nhập phiên chưa mang cài đặt File đích (đường dẫn theo máy).
