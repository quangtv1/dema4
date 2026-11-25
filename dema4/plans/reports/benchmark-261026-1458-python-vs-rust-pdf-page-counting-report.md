# Benchmark: Python (PyMuPDF) vs Rust prototype — đếm khổ trang PDF

Ngày: 2026-10-26 · Máy: macOS arm64, 10 core · PyMuPDF 1.26.7 · Rust 1.99.1

## Phương pháp
- Python: bản headless của lõi `dema4.py` (bỏ Qt, giữ nguyên logic + định dạng dòng).
- Rust: prototype 3 backend — `mupdf` 0.8 (MuPDF), `pdfium-render` 0.9.4 (PDFium build 8111), `pdf` 0.10 (pdf-rs, Rust thuần).
- So khớp: `diff` từng dòng file chi tiết + tổng hợp so với Python.
- Dữ liệu: `02/` (119 file, 6 file 0 byte trong `loi/`), `352.pdf` (331MB, 629 trang), tập lớn 2.261 file / 3,2GB (02 x20 + 352), 15 file hỏng tự tạo (cắt 10% cuối, sai startxref, rác giữa file).
- Dữ liệu có: 20 trang xoay 90/180/270, 1 trang CropBox≠MediaBox, khổ A3/A4/A5 + luật A4 ngang→A5.

## Kết quả độ khớp (số dòng lệch)
| Bộ | mupdf | pdfium | pdf-rs |
|---|---|---|---|
| 02 / 352 / tập lớn | 0 | 0 | 0 |
| 15 file hỏng (Python mở được 15) | 0 (mở 15) | lỗi 5 (cut_tail) | lỗi 10 (cut_tail + bad_startxref) |

## Tốc độ (tập lớn 2.261 file, ổ cục bộ, cache ấm)
| Bản | Thời gian | RAM tối đa |
|---|---|---|
| Python PyMuPDF (tuần tự) | 0,78–1,86s (thường ~1s) | 48MB |
| Rust mupdf 8 luồng | 0,23–0,26s | 20MB |
| Rust mupdf 1 luồng | 1,13s | — |
| Rust pdfium 1 luồng (PDFium không thread-safe) | 0,115s | 19MB |
| Rust pdf-rs 8 luồng | 0,44s | 427MB (đọc cả file vào RAM) |

Drive (thư mục `02/`, cache ấm): Python 0,093s · mupdf 0,011s · pdfium 0,014s · pdf-rs 0,030s.
Drive lần đầu (cache lạnh, chỉ Python đo được): 23,9s → nút thắt là tải file từ Drive, không phải CPU.

## Kết luận
- Rust + `mupdf`: khớp 100% kể cả file hỏng, nhanh ~4x, RAM ít hơn. Ứng viên chính.
- `pdfium`: nhanh nhất, nhưng không mở được file bị cắt cụt mà bản hiện tại đếm được → làm sai lệch kết quả.
- `pdf-rs`: loại — chịu lỗi kém, đọc toàn bộ file vào RAM (xấu khi dữ liệu trên Drive).
- Với khối lượng thực tế (vài trăm–vài nghìn file), Python đã ~1s; lợi ích chính của Rust là exe gọn + đếm song song, không phải tốc độ tuyệt đối.

## Câu hỏi chưa giải quyết
- Chưa build/đo trên Windows (đích thật). `mupdf-sys` cần MSVC + clang khi build.
- Chưa đo được cache lạnh của Drive cho bản Rust (cache đã ấm sau lần chạy Python đầu).
- MuPDF là AGPL — giống PyMuPDF hiện tại, không đổi tình trạng giấy phép, nhưng cần xác nhận nếu phân phối ra ngoài.
- Prototype nằm ở scratchpad phiên làm việc, chưa đưa vào dự án.
