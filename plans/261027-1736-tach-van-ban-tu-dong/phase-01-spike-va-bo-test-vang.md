---
phase: 1
title: "Spike và bộ test vàng"
status: completed
priority: P1
dependencies: []
effort: "1–1,5 ngày"
---

# Phase 1: Spike và bộ test vàng

## Overview
Xoá 2 ẩn số kỹ thuật trước khi viết thật: (a) crate `zxing-cpp` (bundled, cần CMake + C++) build được trên CI Windows/macOS và đọc Code 128 trên ảnh render từ MuPDF đủ đúng/nhanh; (b) cách cắt PDF bằng `mupdf` 0.8 nào giữ nguyên ảnh scan và đủ nhanh; (c) crate `ort` (ONNX Runtime liên kết tĩnh) chạy mô hình hướng trang PP-LCNet nhúng trong exe, build được trên CI Windows/macOS và exe không cần DLL đi kèm (dự phòng `tract`). Đồng thời dựng **bộ test vàng** so với kết quả b3 để các phase sau dựa vào.

## Requirements
- Functional: đọc mã Code 128 từ buffer xám MuPDF; tạo PDF chỉ gồm một dải trang của PDF nguồn.
- Non-functional: đọc mã ≤ ~15 ms/trang/1 lõi (Python zxing-cpp đo 10,5 ms); so tốc độ render MuPDF của crate với PyMuPDF (đo trước: chậm hơn ~30 %, `reports/do-rust-vs-python-render-doc-ma-261028-1548-report.md`); part ra không nén lại ảnh (kích thước ≈ tổng ảnh nguồn).

## Architecture
- Đọc mã: `Page::to_pixmap` 150 dpi → xám → `zxingcpp::read().formats([Code128]).return_errors(true)` (LocalAverage), rồi `.binarizer(GlobalHistogram)`, rồi ảnh thu 0,9 — đúng quy trình phase 3 (đã đo trên mẫu 2: 1.780/1.780). <!-- Updated: 2026-10-28 - thay rxing bằng zxing-cpp theo nhãn tay mẫu 2 -->
- Cắt PDF — so 2 cách, chọn 1:
  1. `PdfDocument::new()` + `graft_object(page_obj)` + `insert_page` (mở nguồn 1 lần, ghép từng trang).
  2. Mở nguồn, `delete_pages(ngoài dải)`, `save_with_options(garbage)` (mỗi part mở lại nguồn).
  Tiêu chí: đúng trang, file part không phình (tài nguyên dùng chung không nhân bản bất thường), thời gian cho 211 part.
- Bộ test vàng: hàm test đọc `mau/pdf_vanban/*_partN.pdf` → (số trang, hash luồng ảnh từng trang). Đối chiếu bằng hash luồng ảnh nhúng (DCT) của trang nguồn tương ứng → so được nội dung, không phụ thuộc cách ghi PDF.

## Related Code Files
- Modify: `dema4_rs/Cargo.toml` (thêm `zxing-cpp` feature `bundled`, `ort` liên kết tĩnh; `tract-onnx` chỉ thêm nếu phải dùng dự phòng)
- Create: `dema4_rs/assets/doc_ori_b1.onnx` (PP-LCNet_x1_0_doc_ori, batch = 1) + `dema4_rs/assets/NOTICE-doc_ori.txt` (nguồn HuggingFace `PaddlePaddle/PP-LCNet_x1_0_doc_ori_onnx`, Apache-2.0)
- Modify: `.github/workflows/*` (CMake/C++ nếu runner thiếu)
- Create: `dema4_rs/src/tach_vb.rs` (khung module + test `#[ignore]` spike)
- Modify: `dema4_rs/src/main.rs` (`mod tach_vb;`)

## Implementation Steps (tests first)
1. Viết test `#[ignore]` `doc_ma_code128_tren_trang_index_mau` (DEMA4_MAU_TACH): trang 10 L1_05_13.pdf phải ra `NAMSAO-TACH`; trang 1 không có mã. → đỏ.
2. Thêm `zxing-cpp`, viết `doc_ma_code128(pixmap) -> Vec<MaDoc { gia_tri, loi_checksum }>` tối thiểu → xanh. In thời gian/trang. Đẩy 1 commit thử lên CI để chắc build Windows.
3. Viết test `#[ignore]` `cat_dai_trang_giu_nguyen_anh`: cắt L1_05_13 trang 1–9 → 9 trang, hash ảnh = trang nguồn 1–9 = `pdf_vanban/L1_05_13_part1.pdf`. → đỏ.
4. Hiện thực cả 2 cách cắt, đo, giữ cách tốt hơn → xanh.
5. Helper test `hash_anh_trang(pdf, trang)` dùng chung cho phase 4.
5b. Test `#[ignore]` `doan_chieu_trang`: nạp mô hình bằng `ort` (từ `include_bytes!`), chạy tensor mẫu → khớp ONNX Runtime Python; trang mẫu xoay 4 chiều → đúng góc; tầng 2 (4 góc xoay) chạy đúng. Đẩy lên CI: exe Windows/macOS **không cần DLL onnxruntime đi kèm** (chạy thử exe trên máy sạch). Không đạt → đổi sang `tract-onnx` qua cùng hàm `du_doan_huong`, chạy lại test. Đo ms/trang và exe tăng thêm.
6. Ghi kết quả spike (tốc độ render + đọc mã, build CI, cách cắt chọn, kích thước exe tăng) vào `reports/` của plan.

## Success Criteria
- [x] zxing-cpp đọc đúng 209/209 trang index trên mẫu 1, 0 thừa (mẫu 2: 1.778/1.780, 2 sót là `Van-ban-khac`); thời gian/trang đã đo; CI Windows + macOS build xanh.
- [x] Đã chọn cách cắt (ghép trang, 1,2 s / 211 part); part1 khớp hash ảnh với b3.
- [ ] (đo lại ở phase 5 khi code được gọi — LTO đang bỏ code chưa dùng) Exe tăng kích thước đã đo (zxing-cpp + ort + mô hình) — chấp nhận tới vài chục MB (người dùng đồng ý 2026-10-28).
- [x] `ort` chạy PP-LCNet 10,7 ms/lần/lõi, khớp ONNX Runtime; test chạy trên CI Windows không DLL kèm (liên kết tĩnh) → không cần `tract`.
- [x] Toàn bộ test cũ vẫn xanh.

Kết quả: `reports/ket-qua-spike-phase-1-261029-0040-report.md`.

## Risk Assessment
- `ort` tải bản ONNX Runtime dựng sẵn lúc build (cần mạng trên CI; nguồn của nhà phát triển crate) và liên kết tĩnh trên Windows MSVC có thể vướng → khoá phiên bản `ort`, cache trên CI; hỏng thì dùng `tract-onnx` (chậm hơn ~2,3 lần, cùng kết quả).
- zxing-cpp không build được trên CI Windows → cài CMake/MSVC trong workflow; vẫn hỏng thì dùng feature `zxingcpp` của mupdf-sys (cùng thư viện, tự bọc FFI). Không quay về rxing (sót 4/68 mã ở hồ sơ khó).
- `graft_object` nhân bản tài nguyên dùng chung (font/ảnh) giữa các trang → dùng cách 2 hoặc graft map chung qua FFI `pdf_new_graft_map`.
