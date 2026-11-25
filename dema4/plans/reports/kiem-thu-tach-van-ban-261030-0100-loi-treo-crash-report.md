# Kiểm thử chức năng Tách văn bản — lỗi treo / chậm / crash

Ngày: 2026-10-30 · Bản: 0.4.4 + sửa chưa phát hành · Máy: macOS, 10 nhân, 16 GB.

## Cách thử
- Đọc luồng: `nut_tach.rs` → `tach::tach_thu_muc` (rayon theo hồ sơ) → `tach_ho_so` → `cat_pdf::cat_bang_ghep` / `fs::copy`.
- Chạy file release qua CLI `--tach` trên 4 bộ dữ liệu: mẫu thật (2 hồ sơ 900 trang), stress (10 bản 270 MB, 1.080 văn bản), bộ PDF lỗi (hỏng, rỗng, 0 trang, mật khẩu, nhãn vượt số trang, tên dài 200 ký tự, tên có `& # % @`), bộ quyền (thư mục cha chỉ đọc, thư mục hồ sơ bị file chiếm chỗ).
- Đo RAM/CPU bằng `/usr/bin/time -l`.

## Kết quả đo
| Bộ | Thời gian | RAM đỉnh | Ghi chú |
|---|---|---|---|
| 10 × 270 MB, 1.080 văn bản | 7,0 s | 242 MB | MuPDF đọc luồng ảnh lười, không nạp cả file; không có `.tmp` sót |

Giao diện: hàm vẽ nút Tách không đụng ổ đĩa mỗi khung hình (`thu_muc_ra` thuần); tách chạy luồng nền; mở thư mục/log dùng `spawn` không chờ.

## Lỗi tìm được và đã sửa
1. **PDF có mật khẩu người dùng → tách ra file hỏng, báo thành công** (nghiêm trọng). MuPDF mở và đếm trang được nhưng luồng dữ liệu là rác. Sửa: `needs_password` → lỗi hồ sơ "PDF có mật khẩu…" ở cả lúc phân tích (`doc_thong_tin_file`) và lúc tách (`so_trang_pdf`). Chỉ mật khẩu chủ sở hữu: tách bình thường (đã kiểm chứng chữ trang ra đúng).
2. **Hồ sơ không mở được lúc phân tích không hiện ở đâu** (`HoSoPhanTich.loi` không ai đọc). Sửa: tên đỏ ở cột trái + lý do khi di chuột; dòng trạng thái thêm "· N hồ sơ không mở được".
3. **Lỗi/cảnh báo tách chỉ hiện 5 dòng trong popup rồi mất**; `canh_bao` (dải toàn trắng) bị bỏ hẳn. Sửa: ghi dòng `# Lỗi …` / `# Cảnh báo …` cuối `ketqua_tach.txt`; popup báo "… và N lỗi khác — Mở log", số cảnh báo.
4. **Hồ sơ 0 trang im lặng không ra gì**. Sửa: thành cảnh báo.
5. **Ghi thẳng file đích**: tắt/kill app giữa chừng để lại PDF cụt trông như hoàn chỉnh. Sửa: ghi `x.n.pdf.tmp` rồi đổi tên; file tạm bỏ dở của lần trước được dọn.
6. **Panic trong 1 hồ sơ làm mất cả lô** (rayon lan panic → chỉ còn "luồng tách dừng bất thường", không log). Sửa: `catch_unwind` từng hồ sơ → lỗi riêng hồ sơ đó; bọc cả luồng tách.
7. **Đóng cửa sổ khi đang phân tích/tách không hỏi gì**. Sửa: hộp "Đang xử lý" — Thoát luôn / Huỷ.
8. Thông báo khó hiểu: "no objects found" → "file hỏng hoặc không phải PDF"; thêm diễn giải đường dẫn quá dài (Windows 206), không có quyền ghi, ổ đĩa đầy.

Test mới: `pdf_mat_khau_bao_loi_khong_ghi_file_hong`, `ho_so_loi_khong_chan_ho_so_khac_khong_sot_file_tam_va_ghi_log`, `ho_so_hong_bao_o_trang_thai_popup_va_log`. Toàn bộ: 139 qua.

## Đã thử, hoạt động đúng (không sửa)
File hỏng / rỗng → lỗi riêng; nhãn vượt số trang → lỗi; thư mục cha chỉ đọc → lỗi từng hồ sơ, hồ sơ khác vẫn chạy; thư mục hồ sơ bị file chiếm → lỗi riêng; tên dài / ký tự lạ → ổn trên macOS; Dừng bỏ hồ sơ chưa bắt đầu; thư mục ra nằm trong nguồn bị chặn từ đầu.

## Chưa thử được (cần máy Windows)
- Tên hồ sơ trùng tên thiết bị Windows (`CON.pdf`, `NUL.pdf`, `LPT1.pdf`): file ra có thể "ghi thành công" mà không tồn tại.
- Đường dẫn ra > 260 ký tự (đã có thông báo diễn giải, chưa chạy thật).
- File đích đang mở trong trình xem PDF (Windows khoá file): đổi tên file tạm sẽ lỗi → báo lỗi hồ sơ (đúng ý), nhưng chưa chạy thật.
- Thuộc tính Hidden của thư mục `.sm-dema4`.
