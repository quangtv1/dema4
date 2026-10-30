# Brainstorm: chức năng thứ 4 "Mục lục hồ sơ" (port print_mltl vào dema4)

Ngày: 30/10/2026 · Nguồn: https://github.com/quangtv1/print_mltl (v1.12.12, commit 7acdeba) · Chế độ: mặc định (không --html/--wiki)

## Vấn đề / yêu cầu
- Người số hoá cần **mục lục cho từng hồ sơ** (DOCX/PDF theo mẫu cơ quan) từ dữ liệu biên mục Excel/CSV; hiện là app riêng (WPF, chỉ Windows, giao diện khác) → muốn **một app dema4** cho Đếm · Tách · Ký · Mục lục.
- Kết quả mong đợi: Menu Chức năng thêm mục 4 **Mục lục hồ sơ**; chọn Excel/CSV + mẫu DOCX → ghép biến → xem trước → tạo DOCX (+PDF) mỗi hồ sơ 1 file.
- Tiêu chí: 4 mẫu thật (Quảng Trị, Quảng Ninh, Đống Đa, Vĩnh Phúc) + `sample/Biên_mục_Quảng_trị.xlsx` (326 dòng / 22 hồ sơ) ra DOCX tương đương bản gốc; xem trước = bản xuất; license tính lượt.
- Ràng buộc: Rust + egui, 1 exe Windows + macOS, phong cách UI hiện tại, **prototype HTML trước**, license qua trạm Apps Script.

## Bản gốc (đọc mã)
- C# .NET 8 WPF, ~3.200 dòng: Core ~1.300 (Reading 214, Grouping 79, Templating 579, Pipeline 237, Validation 66, Output 47), App UI ~2.500, Pdf.WordInterop 134.
- Wizard 4 bước: Nguồn · Ghép biến (Cột / Hằng / Tự động, cột gom nhóm, tự khớp) · Xem trước (Word→PDF→ảnh) · Tạo (DOCX luôn, PDF tuỳ chọn, đa luồng, Resume, Audit).
- Template: biến `{x}`, hàng mẫu trong bảng tự nhận (hàng nhiều biến cấp dòng nhất), `{trang_so}`/`{tong_so_trang}` → field PAGE/NUMPAGES, biến ảnh (Alt Text `image_*` hoặc token `{image_*}`), vá token bị Word cắt vụn.
- PDF + xem trước cần **Microsoft Word** (COM). Không Word → chỉ DOCX.

## Phương án đã cân nhắc
| | Ưu | Nhược | Kết luận |
|---|---|---|---|
| Port lõi sang Rust trong dema4 | 1 exe, UI đồng bộ, license chung, Mac + Win | Công lớn (3–4 tuần), phải tái hiện chính xác merge DOCX | **Chọn** |
| Gọi MLHS.exe có sẵn | Vài giờ | 2 app, UI khác, chỉ Windows, không thiết kế lại được | Loại |
| PDF/xem trước: Word COM | Trung thực nhất (mẫu thiết kế bằng Word) | Chỉ Windows có Word | **Chọn** (chính) |
| + LibreOffice nếu có | Phủ macOS / máy không Word | Bố cục có thể lệch, nhiều máy không cài | **Chọn** (dự phòng) |
| Tự vẽ xem trước | Không phụ thuộc | Không giống DOCX, không PDF | Loại |

## Giải pháp chốt
- **UI 1 màn kiểu Tách** (không wizard): dòng Dữ liệu (file · Sheet · Header dòng) + dòng Mẫu (4 mẫu nhúng sẵn ▾ · Chọn DOCX); cột trái Hồ sơ (#, tên, số VB, trạng thái hợp lệ); tab **Ghép biến** (Biến · Nguồn Cột/Hằng/Ảnh/Từ Tách · Giá trị hồ sơ đang chọn · cột gom nhóm · Kiểm tra dữ liệu) | **Xem trước** (ảnh trang của chính DOCX đã ghép, zoom, ◀ ▶ hồ sơ); dòng đáy Thông tin + **Tạo mục lục** → popup tiến độ → popup kết quả (mở thư mục, log, thử lại lỗi). Chưa có dữ liệu → màn giới thiệu 3 bước.
- **Cấu hình › tab Mục lục**: thư mục ra, mẫu tên `MLHS_{so_ho_so}`, PDF, ghi đè, Resume.
- **License**: chức năng `muc_luc` (10 lượt thử, cột hạn mức riêng trên Sheet, tin Telegram 📑 MỤC LỤC HỒ SƠ (x/y)); 1 lượt = 1 lần Tạo có ra file.
- **Kỹ thuật**: `calamine` + `csv` (đọc), gom nhóm liên tiếp; merge DOCX bằng `zip` + `quick-xml` (vá token, hàng mẫu, PAGE/NUMPAGES, ảnh); PDF: Word COM 1 luồng STA có watchdog, hoặc `soffice --headless --convert-to pdf`; ảnh xem trước bằng MuPDF sẵn có; 4 mẫu nhúng trong exe.
- **Dữ liệu từ Tách**: khớp **theo thứ tự** — tên PDF hồ sơ = giá trị cột gom nhóm; văn bản thứ n ↔ dòng thứ n của nhóm; điền Tờ số / Số trang; lệch số VB → báo lỗi hồ sơ đó.

## Giai đoạn (mỗi giai đoạn phát hành được)
0. Prototype HTML màn Mục lục (giới thiệu · ghép biến · xem trước · popup tạo/kết quả · tab Cấu hình) → duyệt.
1. Lõi + UI, xuất DOCX: đọc Excel/CSV, ghép biến, gom nhóm, kiểm tra, 4 mẫu, license — ~1,5 tuần.
2. Xem trước + PDF (Word COM, LibreOffice) — ~4–5 ngày.
3. Biến ảnh — ~3 ngày.
4. Resume / nhật ký / thử lại hồ sơ lỗi — ~2 ngày.
5. Dữ liệu từ kết quả Tách (khớp theo thứ tự) — ~3 ngày.

## Rủi ro
- Merge DOCX phải tương đương bản gốc (token cắt vụn, Mẫu 04 dựng từ PDF) → test so sánh DOCX trên 4 mẫu + Excel mẫu (văn bản hiển thị, số hàng bảng, field).
- Word COM từ Rust (STA, late-bound `IDispatch`, kill WINWORD treo) — phần khó nhất giai đoạn 2.
- LibreOffice lệch bố cục → ghi rõ "xem trước gần đúng" khi dùng LibreOffice.
- Excel lớn: đọc theo dòng, không giữ toàn bộ; giữ RAM thấp như bản gốc.
- Kích thước exe tăng (calamine, quick-xml, 4 mẫu DOCX ~vài trăm KB) — chấp nhận.

## Đo thành công
- 22 hồ sơ mẫu ra 22 DOCX đúng nội dung (so với bản gốc), mở được trong Word; PDF khớp DOCX.
- Xem trước 1 hồ sơ < 2 s khi Word đã mở; tạo 22 hồ sơ DOCX < 5 s.
- License chặn đúng khi hết lượt; tin Telegram đủ thông tin.

## Câu hỏi còn mở
- (Đã chốt 30/10) Thư mục ra mặc định `.sm-dema4/muc_luc/<tên file dữ liệu>/` cạnh exe; mỗi lần 1 sheet.
