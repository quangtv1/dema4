# Fixtures

File mẫu cho test tự động. **Không đưa dữ liệu thật của khách hàng vào đây.**

| Thư mục | Nội dung | Nguồn |
|---|---|---|
| `merge/` | 3 cặp mẫu `.docx` + `.xlsx` cho Tạo văn bản | Chép nguyên từ `Kytl-main/mau-tao-van-ban/` ngày 18/11/2026 |
| `table/tricky.xlsx` | 20 cột định dạng dễ lệch, tiêu đề ở dòng 2 từ cột B, một dòng trống, một ô gộp, một cột không tiêu đề, một sheet thứ hai | Do Excel 16 tạo bằng `spikes/s4-excel/make-tricky.ps1` ngày 18/11/2026. Kết quả mong đợi (ClosedXML ở `en-US`, `vi-VN`, và chữ Excel hiển thị) ở `docs/spikes/evidence/S4/` |
| `pdf/word-hybrid-xref.pdf` | Mẫu quyết định khen thưởng do Word xuất (bảng xref kèm `/XRefStm`, loại file hay gặp nhất) | Spike S7 / S2, 18/11/2026 |
| `pdf/table-xref.pdf` | PDF xref dạng bảng, có khối chữ tiếng Việt nhúng font | Spike S2 (`pdf-writer`), 18/11/2026 |
| `pdf/xref-stream.pdf` | Cùng nội dung, xref dạng stream | Spike S2, 18/11/2026 |
| `pdf/verify/` | 7 PDF cho test kiểm tra chữ ký (TD-57): ký có TSA, sửa trang rồi ký thêm, dấu thời gian tài liệu, thêm chú thích, thêm DSS, chứng nhận DocMDP P=1 rồi thêm chú thích, RSA-PSS. Chỉ có chứng thư thử, không có khoá | pyHanko ký bằng khoá thử spike S2 + TSA Ban Cơ yếu, `python -I tools/sinh_mau_kiem_tra.py` ngày 24/11/2026 |
| `certs/tsa-ban-co-yeu-02.cer` | Chứng thư công khai "Máy chủ cấp dấu thời gian 02" (không có khoá) | Lấy từ token thật của `tsa.ca.gov.vn` ngày 19/11/2026 |

Dữ liệu lớn để thử Tạo văn bản bằng tay (3 file Excel × 2 sheet × 100 người, kèm mẫu `.docx`) nằm ở
`mau-tao-van-ban/du-lieu-100-nguoi/`, sinh lại bằng `python -I tools/sinh_du_lieu_mau.py`. Toàn bộ là dữ
liệu giả.

Bộ mẫu dài từ 1 đến 4 trang (4 cặp `.docx` + `.xlsx`, mỗi file Excel 100 dòng ở sheet `DuLieu`) nằm ở
`mau-tao-van-ban/mau-nhieu-trang/`, sinh lại bằng `python -I tools/sinh_mau_nhieu_trang.py` (cần
`python-docx`, `openpyxl`): thông báo nghỉ phép 1 trang, quyết định nâng lương 2 trang, thỏa thuận bảo mật
3 trang, hợp đồng đào tạo 4 trang. Khung văn bản lấy từ 3 mẫu gốc. Toàn bộ là dữ liệu giả.

Các PDF đặc biệt khác (trang xoay, nhiều trang, mã hoá, `/DocMDP`) được test tự dựng lúc chạy.

Khoá bí mật và chứng thư test (`*.pfx`, `*.p12`, `*.key`) không được commit; `.gitignore` đã chặn.
