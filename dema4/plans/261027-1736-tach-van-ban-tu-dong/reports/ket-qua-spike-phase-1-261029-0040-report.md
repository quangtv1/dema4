# Kết quả phase 1 (spike) — đọc mã, hướng trang, cắt PDF, build CI

Máy: Apple M1 Pro. Code: `dema4_rs/src/tach_vb/{doc_ma,huong_trang,cat_pdf}.rs`. Test vàng `#[ignore]`: `DEMA4_MAU_TACH`, `DEMA4_MAU2`, `DEMA4_MAU2_NHAN`.

## Đọc mã Code 128 (zxing-cpp 0.5.3, bundled)
| Mẫu | Kết quả | Thời gian (1 luồng) |
|---|---|---|
| Mẫu 1 (896 trang) | **209/209**, thừa 0 | render 150 dpi 117 ms/trang, đọc mã 28 ms/trang |
| Mẫu 2 (21.113 trang, 10 lõi) | **1.778/1.780**, thừa 0; 2 trang sót là mã `Van-ban-khac` (khác mã cấu hình — đúng quy tắc) | 530 s toàn mẫu |

Phát hiện:
- Tờ index mẫu 1 là ảnh rất nhỏ (313×101 điểm trên trang A4 ≈ **27 dpi**). Render 150 dpi phóng ~5,5 lần nên vạch nhoè: ở tỷ lệ 1,0 chỉ đọc được 17/209 trang; thu nhỏ 0,5 (trung bình vùng) đọc được 192/209; gộp 1,0 + 0,5 được 209/209.
- Mẫu 2 trang 4762 tr.122: chỉ lượt thu 0,8 đọc ra `NAMSAO-TA4H`, mã lỗi checksum lệch 1 ký tự; được nhận vì khớp mã index trong cấu hình.
- Quy trình chốt, trên 1 ảnh 150 dpi xám, không render lại:
  1. LocalAverage, giữ cả mã lỗi checksum;
  2. GlobalHistogram;
  3. thu nhỏ 0,5;
  4. thu nhỏ 0,8;
  5. thu nhỏ 0,9.

  Dừng ở lượt đầu tiên ra mã hợp lệ. Thu nhỏ bằng trung bình vùng; nội suy song tuyến tự gây răng cưa nên kém hơn.

## Hướng trang (ort 2.0.0-rc.10, ONNX Runtime liên kết tĩnh)
- Tensor mẫu khớp ONNX Runtime Python, sai < 1e-4.
- Mẫu 1: 40 trang × 4 chiều xoay giả = 160 lần, **quyết 160/160, sai 0** (tầng 2: 3 lần), **10,7 ms/lần**.

## Cắt PDF (mupdf 0.8)
| Cách | 211 part mẫu 1 | Dung lượng | Ảnh |
|---|---|---|---|
| **Ghép trang qua 1 bảng ghép chung** | **1,2 s** | 473,7 MB | băm ảnh từng trang = nguồn = part của b3 |
| Mở lại nguồn + xoá trang + dọn đối tượng thừa | 3,0 s | 473,8 MB | như trên |

→ Chọn **ghép trang** (`cat_bang_ghep`). Tự chép các thuộc tính kế thừa (Resources, MediaBox, CropBox, Rotate). Xoay: cộng vào `/Rotate`, đã test.

## CI (GitHub Actions, macOS-arm64 + Windows-x64)
- Build release xanh trên cả 2.
- Bước mới `cargo test --release tach_vb` xanh trên cả 2 (6 test). Test này chạy thật zxing-cpp (C++) và mô hình qua `ort`.
- Windows chạy được mà không có DLL onnxruntime đi kèm, tức ONNX Runtime đã liên kết tĩnh → **không cần dự phòng `tract`**. zxing-cpp build được trên Windows bằng công cụ có sẵn của runner (CMake, MSVC) → không cần dùng feature `zxingcpp` của mupdf-sys.

## Kích thước exe
- Hiện chưa đổi (14,50 MB macOS so với 14,52 MB bản v0.2.3). Lý do: code tách văn bản chưa được gọi nên tối ưu khi liên kết (LTO) bỏ hết.
- Đo lại khi nối vào giao diện (phase 5). Ước tính thêm ~15–20 MB: mô hình 6,8 MB + ONNX Runtime ~9 MB + zxing-cpp ~1–2 MB.

## Câu hỏi mở
- Không có.
