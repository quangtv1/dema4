# Độ chính xác phát hiện Index / Trắng trên mau2 so với nhãn tay

Nhãn chuẩn: `~/Desktop/mau2/index_and_blank (mau2).txt`. Gồm 5.340 dòng: 1.780 index và 3.561 blank. Ghép trang theo tên file. 232/234 file có nhãn; `4613` và `4727` không có dòng nào nên coi là toàn Content.
Kết quả máy:
- mã đọc ở 100 → 150 dpi (`ket_qua.tsv` cũ);
- các chỉ số trắng tính lại cho đủ 21.113 trang.

Script nằm trong scratchpad: `do_v2_ca_mau.py`, `dac_trung.py`, `so_nhan_mau2.py`, `hai_nguong_v3.py`.

## Index (Code 128, sai lệch ≤ 3 ký tự so với `NAMSAO-TACH`)
| | Số trang |
|---|---|
| Nhãn tay | 1.780 |
| Máy gán index | 1.775 (**đúng 100 %**, không gán thừa trang nào) |
| Máy sót | 5 trang (**độ phủ 99,72 %**) |

5 trang sót:
- 2 trang mã `Van-ban-khac` (4632 tr.29, 4705 tr.90). **Người gán coi là index**, trong khi plan hiện để là "không gán".
- 2 trang không đọc được ở 150 dpi nhưng **đọc được ở 200 dpi** (4743 tr.76, 4762 tr.122).
- 1 trang mã bị rách/cong (4705 tr.50): 300 dpi vẫn không đọc được, phải gán tay.

Nếu (a) coi mọi Code 128 đọc được là index và (b) thử lại ở 200 dpi khi trang không có mã nào ở 150 dpi, thì máy được **1.779/1.780**.

## Trắng
### Ngưỡng tạm hiện tại (chỉ số v2, 0,10 % / 0,50 %): **không đạt**
- Máy gọi Trắng 774 trang và đúng cả 774, nhưng **chỉ bắt được 21,7 %** số trang trắng.
- 55,9 % trang trắng thật bị đẩy sang Content.
- Nguyên nhân: người gán tính là Trắng cả các loại trang sau, mà v2 lại đếm chúng là "mực":
  - mặt sau có chữ thấm ngược;
  - trang chỉ có dấu đỏ;
  - giấy pơ-luya có vân;
  - bìa màu;
  - trang có viền/bóng mép.

  Ngay cả nới ngưỡng của v2 thì độ phủ 99 % cũng kéo theo 2.582 trang chữ.

### Chỉ số mới v3: cụm nét **đậm** + một phần mực màu
- Render 100 dpi RGB, bỏ 8 % mép, tính nền cục bộ theo khối 16 px (giống v2).
- `dam` = tỷ lệ khối 8×8 có ≥ 6 điểm **tối hơn nền ≥ 60** và không phải mực màu. Chữ thấm ngược và vân giấy đều nhạt nên bị loại.
- `mau` = tỷ lệ khối 8×8 có ≥ 6 điểm mực đỏ/xanh.
- **v3 = dam + 0,3 × mau**. Dấu đỏ đơn lẻ vẫn thấp, còn trang viết tay mực đỏ/xanh dày thì vượt ngưỡng.
- Tốc độ như v2 (cùng một lần render).

So khả năng tách (AUC; độ phủ khi máy đúng ≥ 99 %):

| Chỉ số | AUC | Độ phủ |
|---|---|---|
| v2 | 0,9944 | 93,5 % |
| chỉ tính nét đậm | 0,9983 | 98,4 % |
| **v3** | 0,9982 | **98,6 %** |

### Hai ngưỡng với v3 (19.333 trang không phải index, 3.561 trắng)
| Trắng < a / Nghi < b | Máy gọi Trắng (đúng %) | Nghi trắng: trắng / content | Trắng lọt sang Content | Content bị gọi Trắng |
|---|---|---|---|---|
| 1,0 / 2,5 % | 3.353 (99,79 %) | 191 = 164 / 27 | 51 (1,43 %) | 7 |
| **1,0 / 3,0 %** | **3.353 (99,79 %)** | **221 = 181 / 40** | **34 (0,95 %)** | **7** |
| 1,5 / 4,0 % | 3.467 (99,57 %) | 159 = 87 / 72 | 22 (0,62 %) | 15 |

**Đề xuất mặc định 1,0 % / 3,0 %**. Mức này đạt cả hai mục tiêu ở phase 3: Trắng tự động đúng ≥ 99 % (đạt 99,79 %) và trang trắng lọt sang Content ≤ 1 % (đạt 0,95 %). Người duyệt chỉ phải xem 221 trang Nghi trắng, khoảng 1 % số trang.

Lỗi còn lại (xem ảnh):
- **Content → Trắng**:
  - trang chỉ có vài dòng viết tay màu nhạt;
  - trang chữ đánh máy rất mờ;
  - bìa in chữ đỏ.

  Một vài trang trong số này nhãn tay có thể bàn lại, ví dụ trang gần như trống chỉ có ô dấu mờ.
- **Trắng → Content**: mặt sau thấm mực đậm, vết bẩn hoặc mép đen lấn vào trong vùng 8 % mép.

## Chốt sau trao đổi (2026-10-28)
Người dùng chốt:
- index = Code 128 khớp mã trong Cấu hình, đổi Cấu hình thì theo; index bắt buộc đúng 100 %;
- trang dấu đỏ / viết tay màu nhạt → Nghi trắng, để người dùng tự phân loại.

### Index: quy trình đạt 100 % (đo lại, `ma150_v2.py`)
Render 1 lần 150 dpi xám, 3 lượt đọc trên cùng ảnh (zxing-cpp):

| Lượt | Số index tìm thấy |
|---|---|
| LocalAverage + mã lỗi checksum | 1.757 |
| + GlobalHistogram | +20 |
| + ảnh thu 0,9 | +1 |
| + mã lỗi checksum khớp ≤ 3 ký tự (2 trang) | **1.780/1.780, 0 thừa** |

- Mã hợp lệ trên toàn mẫu chỉ có 2 giá trị: `NAMSAO-TACH` 1.775 và `Van-ban-khac` 2.
- Với mã index `NAMSAO-TACH` trong Cấu hình, 2 trang `Van-ban-khac` (nhãn tay ghi là index) **không** được gán → người duyệt gán tay. Muốn máy tự bắt cả 2 trang này thì để trống ô mã.
- Thời gian (10 tiến trình song song): render 159 ms/trang, đọc mã 32 ms/trang.
- Crate Rust `zxing-cpp` 0.5.3 có đủ `ReturnErrors` và `Binarizer`.

### Trắng: chỉ số v3, mặc định 1,0 % / 3,0 %, trang có mực màu (`mau` ≥ 0,05 %) → Nghi trắng
| | Số trang |
|---|---|
| Máy gọi Trắng | 2.732 (đúng 99,85 %; 4 trang chữ đánh máy rất mờ bị gọi Trắng) |
| Nghi trắng | 842 = 799 trắng / 43 chữ (≈ 4 % trang) |
| Trắng lọt Content | 34 (0,95 %) |

Nếu không đẩy trang có mực màu sang Nghi trắng: Nghi chỉ còn 307 trang nhưng Trắng sai 6 trang.

Đã cập nhật: phase 1, 3, 5, 7, plan.md (session 29), prototype (thang 0–10 %, mặc định 1,00 / 3,00).

## Câu hỏi mở
- Kiểm trên hồ sơ khác mẫu 2 trước khi coi ngưỡng là ổn định.
