# Đo mô hình học máy nhận hướng trang (0/90/180/270°) trên mau2

Cách thử:
- lấy 200 trang có chữ của mau2 (cùng bộ chọn ngẫu nhiên), xoay giả mỗi trang 4 chiều = 800 lần đoán;
- 8 trang gốc vốn bị xoay sẵn (cả 2 mô hình lệch cùng một góc ở cả 4 lần, xem ảnh thấy đúng là bị xoay) → sửa nhãn cho 8 trang này;
- mỗi lần đo dùng 1 luồng CPU (M1 Pro), ảnh 100 dpi.

Script trong scratchpad: `thu_ml_chieu*.py`, `so_ket_hop.py`, `pplc_ca_mau.py`, `thu_tract/`.

## Kết quả
| Cách | Đúng | Khi bỏ 10 % lần kém tin nhất | ms/trang |
|---|---|---|---|
| **PP-LCNet_x1_0_doc_ori** (PaddleX, ONNX 6,8 MB, Apache-2.0) | **97,8 %** | **100 %** | 13 (ONNX Runtime) · **20 (tract, Rust)** |
| RapidOrientation (PyPI `rapid-orientation`) | 97,8 % | 100 % | 14 — cùng mô hình PP-LCNet, kết quả trùng 100 % |
| docTR `mobilenet_v3_small_page_orientation` | 75,0 % | — (bỏ 50 % kém tin: 95,5 %) | 36 |
| Tesseract 5.5 OSD | 75,0 % | — (tin cậy ≥ 5: phủ 38 %, đúng 92,7 %) | 458 |
| Tự viết (dòng chữ + dấu tiếng Việt) | 86,5 % | 93,1 % | 3 |
| Cả 3 (PP-LCNet + Rapid + tự viết) cùng ý | 99,85 % trên 84,8 % số lần | — | — |

Ngưỡng tin cậy của PP-LCNet (xác suất lớp cao nhất; mô hình gần như không vượt 0,95):

| Tin cậy ≥ | Phủ | Đúng | Sai |
|---|---|---|---|
| 0,5 | 97,5 % | 98,85 % | 9 |
| 0,7 | 93,5 % | 99,73 % | 2 |
| **0,8** | **88,4 %** | **100 %** | **0** |
| 0,9 | 71,5 % | 100 % | 0 |

Chạy thật trên 15.606 trang có chữ của mau2:
- ở mức tin cậy ≥ 0,8, máy gợi ý xoay **545 trang** (180° 262, 270° 184, 90° 99), khoảng 3,5 %;
- xem 32 ảnh mẫu thì khoảng 30 trang bị xoay thật (bảng ngang, viết tay dọc, trang lộn ngược). Số còn lại là bản vẽ mặt bằng hoặc khó phán đoán.

Còn 1.622 trang tin cậy < 0,8: không gắn cờ. Nếu trang thật sự bị xoay mà nằm trong số này thì người duyệt tự xoay bằng R.

## Đưa vào Rust
- **`tract-onnx` 0.21** (thuần Rust, không cần thư viện C++):
  - kết quả khớp ONNX Runtime tới 6 chữ số;
  - 20 ms/lần/1 luồng, nạp mô hình 47 ms;
  - cần sửa chiều batch `DynamicDimension.0` thành 1 trước khi dùng. Script đã làm, file ra `doc_ori_b1.onnx`.
- Nhúng mô hình bằng `include_bytes!`: exe nặng thêm ~6,8 MB mô hình + mã tract (đo ở phase 1).
- Phương án dự phòng: crate `ort` (ONNX Runtime) nhanh hơn (13 ms) nhưng cần thư viện động ~15–20 MB đi kèm exe.
- Tiền xử lý:
  1. ảnh 150 dpi RGB đã render (dùng chung với đọc mã);
  2. thu cạnh ngắn về 256, cắt giữa 224×224;
  3. chuẩn hoá ImageNet (trung bình 0,485/0,456/0,406, độ lệch 0,229/0,224/0,225), CHW;
  4. lớp ra `[0, 90, 180, 270]` = góc ảnh đang bị xoay ngược kim → **góc cần xoay theo kim = [0, 270, 180, 90]**.

## Quyết định (người dùng chốt 2026-10-28)
- Làm phương án gắn cờ **Nghi xoay**, không tự xoay; dùng PP-LCNet thay cho cách tự viết (dung lượng thêm vài chục MB chấp nhận được).
- Enter, nút "Xoay theo gợi ý" hoặc bấm nhãn góc trên ảnh khi trang đang được chọn → xoay, rồi **chọn trang Nghi xoay kế tiếp để hỏi tiếp**.

## Bổ sung: tăng độ chính xác (cùng 800 lần đoán, ms/trang 1 luồng ONNX Runtime Python)
Script: `thu_tta.py`, `thu_dong.py`.
| Cách | Đúng | Phủ khi 0 lỗi | ms/trang |
|---|---|---|---|
| PP-LCNet 1 vùng giữa (hiện tại) | 97,8 % | 88,4 % (tc ≥ 0,8) | 13 |
| 3 / 5 vùng dọc trang | 97,8 / 97,6 % | 84,9 / 85,8 % | 32 / 51 |
| **1 vùng × 4 góc xoay** (xoay ảnh 0/90/180/270, dịch kết quả về rồi cộng) | **98,5 %** | **96,0 % (tc ≥ 0,6)** | **40** |
| 3 vùng × 4 góc xoay | 97,5 % | 96,5 % (tc ≥ 0,6) | 112 |
| Bỏ phiếu theo dòng chữ: PP-OCRv5_mobile_det (4,6 MB) + PP-LCNet_x1_0_textline_ori (6,5 MB), tối đa 24 dòng | 94,9 % | — (tc ≥ 0,9 vẫn sai 3) | 411 |
| Trang + dòng chữ (trung bình xác suất) | 98,9 % | 95,0 % (tc ≥ 0,6) | 424 |

Nhận xét:
- **Chạy 4 góc xoay** là cách lợi nhất:
  - dùng cùng mô hình, không thêm file;
  - ở mức không sai lần nào, phần trang được gợi ý tăng từ 88 % lên **96 %**. Hạ được ngưỡng tin cậy từ 0,8 xuống 0,6, nên gắn cờ đúng thêm nhiều trang mà vẫn không sai;
  - tốn gấp ~3 lần thời gian (Rust + `ort` ước ~35 ms/trang).
- Thêm nhiều vùng cắt dọc trang không giúp gì.
- Bỏ phiếu theo dòng chữ:
  - kết hợp với mô hình trang thì đúng cao nhất (98,9 %), nhưng chậm hơn ~30 lần (0,4 s/trang ≈ +1,5 giờ cho 100k trang trên 8 lõi) và thêm 2 mô hình;
  - tác dụng chính là lấy đúng 8 trang mà mô hình trang không chắc;
  - chỉ đáng dùng nếu chạy riêng cho các trang có tin cậy thấp (~10 % trang), khi đó thêm ~40 ms/trang tính bình quân.
- Lâu dài: tinh chỉnh (fine-tune) PP-LCNet bằng chính dữ liệu của đơn vị (`xoay_trang.txt` + trang đứng xuôi đã duyệt), làm bằng PaddleX ngoài app rồi xuất ONNX thay file.

## Bổ sung 2: tầng 2 cho trang mô hình không chắc (script `thu_ocr_chieu.py`, `thu_tang2.py`)
Tầng 1 (1 lần, tin cậy ≥ 0,8): quyết 707/800 lần, sai 0. Còn **93 lần khó** (bảng, bản vẽ, chữ mờ). Trên 93 lần này:

| Tầng 2 | Đúng (đoán hết) | Chỉ quyết khi chắc, không sai lần nào | Thời gian/trang |
|---|---|---|---|
| Không làm gì thêm (giữ kết quả tầng 1) | 75/93 | 41 (tc ≥ 0,7; vẫn sai 2) | 0 |
| **4 góc xoay** | 81/93 | **73 (tc ≥ 0,5, sai 0)** | 70 ms |
| Dòng chữ (det + textline) | 81/93 | — (tc ≥ 0,7 vẫn sai 7) | 403 ms |
| 4 góc + dòng chữ | 85/93 | 57 (tc ≥ 0,7, sai 0) | ~470 ms |
| OCR tiếng Việt Tesseract, 4 hướng, chọn hướng ra nhiều chữ nhất | 38/93 | — | 7,7 s |

→ **Hai tầng**: tầng 1 cho mọi trang, tầng 2 = **4 góc xoay** chỉ cho trang tin cậy < 0,8 (~12 %), quyết khi tin cậy ≥ 0,5.
- Kết quả: quyết được **780/800 (97,5 %) mà không sai lần nào**, so với 707/800 khi chỉ có tầng 1.
- 20 lần còn lại không gắn cờ, người duyệt tự xoay bằng R.
- Chi phí bình quân thêm ~5 ms/trang (Rust + `ort` ước ~4 ms).
- OCR và bỏ phiếu theo dòng chữ đều không hơn trên nhóm trang khó, nên bỏ.
- Mô hình ảnh + ngôn ngữ lớn chạy trong máy (cỡ vài GB, ~10 s/trang trên CPU) hoặc API đám mây (phải gửi ảnh hồ sơ ra ngoài): không đề xuất.

## Câu hỏi mở
- Ngưỡng 0,8 mới đo trên 800 lần đoán của mau2. Cần kiểm thêm trên bộ hồ sơ khác, hoặc dùng `xoay_trang.txt` người dùng tích luỹ.
- Có thể tinh chỉnh (fine-tune) PP-LCNet bằng `xoay_trang.txt` sau này nếu cần phủ thêm 1.622 trang tin cậy thấp.
