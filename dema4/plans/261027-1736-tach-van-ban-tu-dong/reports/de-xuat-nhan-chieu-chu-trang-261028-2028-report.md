# Đề xuất: tự nhận chiều chữ của trang (0 / 90 / 180 / 270°)

Mục tiêu: lúc Phân tích, gợi ý trang scan bị xoay để người dùng xoay bằng ↻. Plan đã có sẵn xoay tay, lưu `xoay_trang.txt` và áp /Rotate lên file ra.
Cách thử:
- **xoay giả**: lấy 600 trang có chữ của mau2, xoay mỗi trang 4 chiều (2.400 lần đoán), coi trang gốc là đứng xuôi. Thực tế mau2 có trang gốc bị xoay thật, nên tỷ lệ đúng đo ra là mức thấp;
- **chạy thật**: chạy trên 15.606 trang có chữ của mau2.

Script trong scratchpad: `chieu_chu3.py`, `thu_osd.py`, `chieu_ca_mau.py`.

## Thư viện hiện có
| Thư viện | Có nhận chiều không |
|---|---|
| MuPDF (crate `mupdf` 0.8) | Có feature `tesseract` (kèm Tesseract + Leptonica, C++), nhưng API MuPDF chỉ OCR chữ; **không có hàm nhận chiều (OSD)**. Muốn dùng phải tự bọc FFI API Tesseract và mang thêm `osd.traineddata` (~10 MB). |
| zxing-cpp | Chỉ cho chiều của mã vạch, tức chỉ có ở trang index. Trang index bị bỏ khi tách nên không cần. |

## Kết quả đo
| Cách | Đúng (xoay giả 4 chiều) | Thời gian/trang |
|---|---|---|
| Tesseract 5.5 OSD (`--psm 0`, 150 dpi) | 75,0 % (độ tin cậy ≥ 5: phủ 38 %, đúng 92,7 %) | ~458 ms |
| **Tự viết** trên ảnh 100 dpi đã có | 85,6 % trên mọi trang; bỏ 25 % kém tin: đúng 93,8 %; bỏ 50 %: đúng 96,3 % | **~3 ms** |

Cách tự viết hoạt động như sau:
- Bỏ đường kẻ bảng (các đoạn mực ngang/dọc dài ≥ 40 px).
- **Dòng chữ ngang hay dọc**: trong mỗi dải hẹp, đếm tỷ lệ hàng trắng giữa các dòng chữ.
- **Chữ xuôi hay ngược**: trong mỗi dòng, so lượng mực phía trên lõi chữ với phía dưới. Phía trên có dấu tiếng Việt, chữ cao (b d h k l t), chữ hoa; phía dưới chỉ có g p q y.
- **Độ tin cậy** = min(độ chênh ngang/dọc, độ chênh trên/dưới).

Trên mau2 thật, với độ tin cậy ≥ 0,3, máy gợi ý xoay 258 trang (180° 143, 90° 58, 270° 57). Trong 24 ảnh xem mẫu, khoảng 16–18 trang bị xoay thật (bảng ngang, viết tay dọc, trang lộn ngược). Số còn lại là gợi ý sai, ở bản vẽ, mặt bằng, bảng ít chữ.

Vì sao còn sai:
- bảng biểu và bản vẽ chỉ có số và đường nét, không có dòng chữ;
- chữ đánh máy in hoa toàn dòng (không có chữ cao/thấp).

## Phương án
1. **Đề xuất — tự viết, gợi ý "Nghi xoay" (không tự xoay)**
   - Chạy trong cùng lượt Phân tích: không render thêm, không thêm thư viện, ~3 ms/trang.
   - Chỉ gắn cờ **Nghi xoay ↻90 / ↻180 / ↻270** khi độ tin cậy ≥ ngưỡng (mặc định 0,3). Cờ giống Nghi trắng: không phải nhãn, không ảnh hưởng cắt.
   - GUI:
     - bộ lọc thêm mục "Nghi xoay (n)";
     - ảnh có biểu tượng ↻ kèm góc gợi ý;
     - phím **Enter** (hoặc nút "Xoay theo gợi ý") áp góc gợi ý cho vùng chọn, ghi vào `xoay_trang.txt`; **C** bỏ cờ;
     - Ctrl+Z dùng chung.
   - Cấu hình: thêm ô bật/tắt "Chiều trang" vào nhóm "1. Tự động nhận dạng khi Phân tích".
   - Tự xoay luôn là không được: đúng ~94–96 % thì 100k trang sẽ có hàng nghìn trang bị xoay sai mà không ai biết.
2. **Tesseract OSD qua feature `tesseract` của mupdf-sys**
   - Kém hơn trên dữ liệu này (75 %), chậm hơn ~150 lần.
   - Cần tự bọc FFI C++, exe nặng thêm ~10–15 MB, build Windows nặng hơn.
   - **Không đề xuất.**
3. **Mô hình học máy nhỏ (CNN, ONNX) — để sau**
   - Có thể đạt > 99 %, nhưng cần runtime `ort` (~10–20 MB) và dữ liệu huấn luyện.
   - Dữ liệu có thể lấy từ `xoay_trang.txt` mà người dùng tích luỹ khi duyệt bằng phương án 1.
   - Xét khi phương án 1 không đủ.

## Nếu chọn phương án 1, cần sửa
- Phase 3:
  - hàm `doan_chieu(mat_na) -> (goc, tin_cay)` chạy trên ảnh 100 dpi;
  - cache thêm góc + độ tin cậy (2 byte/trang);
  - test xoay giả trên trang mẫu: đúng ≥ 85 % mọi trang, ≥ 95 % khi tin cậy ≥ 0,3.
- Phase 5: bộ lọc, biểu tượng, phím Enter, ô bật/tắt; prototype thêm demo.
- Thời gian Phân tích tăng ~3 %.

## Câu hỏi mở
- Đo độ chính xác thật cần nhãn xoay tay: cần danh sách trang bị xoay ở mau2 (hoặc `xoay_trang.txt` sau khi duyệt).
- Phím áp gợi ý: Enter, hay chỉ dùng nút?
