# Đo mẫu 2 (Desktop/mau2): thời gian, index, trang trắng

Mẫu: 234 PDF, 21.113 trang, scan màu (DeviceRGB), 9,3 GB, cấu trúc `HOP x/<id>/<id>.pdf`. **Không có nhãn chuẩn** → độ chính xác trang trắng ước lượng bằng xem ảnh.
Công cụ: zxing-cpp + PyMuPDF, cùng thuật toán phase 3; máy 10 lõi. Script: scratchpad `do_mau2.py`, `do_trang2.py`.

## Thời gian
| Việc | ms/trang/lõi |
|---|---|
| Đọc dung lượng (/Length) | 0,2 |
| Dò trắng 40 dpi | 19,8 |
| Đọc Code 128 (100 → 150 dpi) | 185,4 |
Tổng 21.113 trang: **484 s** (đọc mã mọi trang) → ~23 ms/trang thực; 100k trang ≈ 38 phút. Đọc mã chiếm ~90 %.

## Index
- 1.777 trang đọc được Code 128: `NAMSAO-TACH` 1.747; đọc lệch ký tự (`NAM5AO-T,CH`…) 28; **`Van-ban-khac` 2** (giá trị mã khác). 452 trang (25 %) cần 150 dpi.
- **Quy tắc "chỉ đọc mã trang < 35 % trung vị dung lượng" HỎNG trên mẫu này**: 1.123/1.777 index (63 %) nặng ≥ 35 % trung vị (tối đa 100 %). Scan màu nên tờ index nặng như trang chữ. Trang nghi trắng cũng không nhẹ (trung vị 60 % trung vị hồ sơ).

## Trang trắng
- Chỉ số cũ (40 dpi xám, lệch nền ≥ 60, ngưỡng 0,20 %): 3.282 trang bị gọi trắng (15,5 %). Xem mẫu: dải < 0,05 % phần lớn trắng thật nhưng có trang chữ rất nhạt/dấu đỏ; dải 0,05–0,20 % (417 trang) **đa số là trang có chữ nhạt, bảng, dấu** → ước **~20 % trang bị gọi trắng là sai** (≈ 600–700 trang).
- Chỉ số thử v2 (100 dpi màu, nền cục bộ khối 16 px, lệch ≥ 18 hoặc mực đỏ/xanh, đếm cụm 8×8): tách tốt hơn; < 0,10 % gần như toàn trắng; 0,10–0,50 % là **vùng mập mờ** (trắng có viền đen/bóng mép + chữ rất nhạt + dấu + có thể chữ thấm từ mặt sau). 79 ms/trang/lõi.
- Không có nhãn chuẩn nên chưa tính được precision/recall thật.

## Đề xuất cập nhật plan
1. Bỏ mặc định "chỉ đọc mã trang nhẹ" → **đọc mã mọi trang** (mặc định); giữ sắp theo dung lượng chỉ như tiện ích xem.
2. Trang trắng 2 ngưỡng (chỉ số v2): dưới ngưỡng thấp → Trắng; vùng giữa → bộ lọc **"Nghi trắng"** để người duyệt nhanh; trên ngưỡng cao → Content.
3. Cần nhãn chuẩn của mau2 (nếu đã tách tay) để chốt ngưỡng và đo chính xác.

## Câu hỏi mở
- mau2 đã có `index_and_blank.txt` làm tay chưa?
- Trang chỉ có dấu đỏ / chữ thấm từ mặt sau tính là Trắng hay Content?
- `Van-ban-khac` là loại tờ phân cách khác hay mã lạ?
