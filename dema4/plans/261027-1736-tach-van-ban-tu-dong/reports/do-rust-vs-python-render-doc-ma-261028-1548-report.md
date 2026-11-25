# Đo Rust vs Python: render + đọc Code 128 (mau2)

Máy: Apple M1 Pro, 1 luồng. Cùng trang, cùng thuật toán. Script: scratchpad `bench_ma/` (Rust), `bench_ma.py`, `bench_ma150.py` (Python).
- Python: PyMuPDF (MuPDF 1.28.2) + zxing-cpp.
- Rust: crate `mupdf` 0.8.0 (MuPDF 1.27.2) + `rxing` 0.9.3 hoặc crate `zxing-cpp` 0.5.3 (bundled, zxing-cpp 3.1.1).

## Kết quả (ms/trang = render + đọc mã)

Bộ dễ: 4597 + 4700, 165 trang, 27 index.

| Cách | Python | Rust + rxing | Rust + zxing-cpp |
|---|---|---|---|
| 100 → thử lại 150 dpi | 105 (27/27) | 137 (27/27) | — |
| 1 lần 150 dpi | **86** (27/27) | 119 (27/27) | 117 (27/27) |
| 1 lần 120 dpi | — | 39 (27/27) | — |

Bộ khó: 4762, 4760, 4750, 4749, 372 trang, 68 index; nhiều trang cần 150 dpi.

| Cách | Python | Rust + rxing | Rust + zxing-cpp |
|---|---|---|---|
| 100 → thử lại 150 dpi | 125 (68/68) | 167 (**64/68**) | 163 (68/68) |
| 1 lần 150 dpi | **94** (68/68) | 132 (**64/68**) | 128 (68/68) |
| 1 lần 120 dpi | — | (12 file: sót tới 2/3 index ở hồ sơ khó) | — |

## Nhận định
1. **~95 % thời gian là render trang (giải nén JPEG scan màu trong MuPDF, mã C)**. Đọc mã chỉ 4–8 ms. Ngôn ngữ bao ngoài gần như không ảnh hưởng.
2. **Rust hiện chậm hơn Python ~30–35 % ở render**: MuPDF trong crate là 1.27.2, trong PyMuPDF là 1.28.2 (có thể khác cờ biên dịch). Biên dịch `target-cpu=native` không đổi gì. Crate 0.8.0 đã là bản mới nhất.
3. **rxing (zxing viết lại bằng Rust) sót 4/68 mã ở hồ sơ khó** → không đạt yêu cầu "index đúng 100 %". Crate `zxing-cpp` (cùng thư viện với Python) đọc 68/68.
4. **Thắng lớn nhất là thuật toán, không phải ngôn ngữ**: render **1 lần ở 150 dpi** nhanh hơn "100 rồi thử lại 150" (mọi trang không có mã bị render 2 lần), đủ mã. Phase 3 cũ còn render riêng 40/100 dpi cho dò trắng → gộp về 1 lần 150 dpi, thu nhỏ cho dò trắng + ảnh nhỏ.
5. Ước mau2 (21.113 trang, ~8 lõi hiệu dụng): Python 1 lần 150 dpi ≈ 4 phút; Rust ≈ 5,5 phút (so với 8 phút đo trước bằng 40 dpi + 100→150).

## Rust vẫn đáng chọn vì
- Gộp vào app `sm-dema4` đang có (màn Đếm, Telegram, cfg, build CI Windows/macOS): 1 file exe, không cài Python/WSL.
- GUI lưới 100k trang ảo hoá, chọn kiểu Explorer, nhãn 2 lớp: egui chạy mượt, ít RAM; Python cần Qt (gói ~100+ MB, khởi động chậm) và vẫn phải tự ảo hoá lưới.
- Chạy nền + giao diện cùng tiến trình, luồng thật (không GIL); Python phải dùng multiprocessing + truyền ảnh qua tiến trình.

## Đề xuất cập nhật plan
- Đọc mã: **`zxing-cpp` crate (bundled)** thay `rxing`. Rủi ro: cần CMake + trình dịch C++ trên CI Windows (GitHub Actions có sẵn) → kiểm ở phase 1.
- Render **1 lần/trang ở 150 dpi** (RGB hoặc xám — đo ở phase 1): đọc mã trên ảnh xám; thu nhỏ cho chỉ số trắng v2 (giữ màu để bỏ dấu đỏ/xanh) và ảnh nhỏ.
- Phase 1 thêm việc: so render MuPDF crate vs PyMuPDF; nếu vẫn chậm ~30 % thì chấp nhận (vẫn nhanh hơn số đo cũ) hoặc thử nâng MuPDF khi crate ra bản mới.

## Câu hỏi mở
- Chưa rõ nguyên nhân render chậm 30 % (phiên bản hay cờ biên dịch): hook chặn đọc log biên dịch trong thư mục `build`.
