# Brainstorm: License giới hạn số lần Đếm / Tách

Ngày 2026-10-30 · Trạng thái: đã chốt phương án A (+ 2 biện pháp giảm rủi ro), chờ lập kế hoạch.

## 1. Vấn đề & mục tiêu
- Mục tiêu: **kiểm soát phân phối** — biết máy / đơn vị nào dùng, cấp hoặc thu hồi quyền từng máy từ Google Sheet; người dùng thử phải liên hệ Sao Mai.
- Không nhằm chống bẻ khoá chuyên nghiệp.

## 2. Quyết định của người dùng
| Mục | Chốt |
|---|---|
| Mục tiêu | Kiểm soát phân phối |
| Phương án | **A — app đọc/ghi Sheet trực tiếp** bằng khoá service account nhúng |
| 1 lần | Mỗi lần bấm Đếm / Tách chạy xong (không lỗi toàn bộ, không Dừng ngay) |
| Offline | Luôn tối đa 2 lần (kể cả máy đã cấp) |
| Hạn mức mặc định | 2 Đếm · 2 Tách |
| Repo | Chuyển riêng tư; phát hành qua **repo công khai riêng** chỉ chứa file exe |
| AGPL / MuPDF | Tạm chấp nhận rủi ro, xử lý sau |

## 3. Hiện trạng code liên quan
- `google_sheet.rs`: ghi Sheet bằng khoá service account nhúng lúc build (quyền đọc + ghi cả spreadsheet).
- `telegram.rs`: tên máy, MAC, IP nội bộ, IP public.
- Dữ liệu cục bộ cạnh exe (`sm-dema4.cfg`, `.sm-dema4/`).
- Kiểm tra cập nhật đọc GitHub API công khai của repo hiện tại.

## 4. Phương án đã cân nhắc
| | A — Sheet trực tiếp (chọn) | B — trạm Apps Script + chữ ký RSA | C — file license ký số |
|---|---|---|---|
| Công sức | ~2 ngày | ~4–5 ngày | ~2–3 ngày + cấp tay |
| Khoá trong exe | Có (rủi ro) | Không | Không |
| Ai giữ số lần dùng | Sheet (nhật ký) | Sheet | Máy khách |
| Chặn tự nâng hạn mức | Có nhờ khoá vùng | Có | Có |
| Chặn reset số lần | Không (để dấu vết) | Có | Không |

## 5. Thiết kế phương án A
**Mã máy**: SHA-256(tên máy + MAC + MachineGuid Windows / IOPlatformUUID macOS) → `SM-XXXX-XXXX-XXXX`; hiện trên popup, có nút Copy.

**Tab `license`** (1 dòng / máy) — cột khoá vùng (chỉ chủ Sheet sửa) đánh dấu 🔒:
Mã máy · Tên máy · MAC · IP public · Đơn vị 🔒 · Hạn mức Đếm 🔒 · Đã dùng Đếm (công thức) · Hạn mức Tách 🔒 · Đã dùng Tách (công thức) · Trạng thái 🔒 (Dùng thử / Đã cấp / Khoá) · Ngày đăng ký · Lần dùng cuối · Phiên bản · Ghi chú.
Tab `license_nhat_ky`: Thời điểm · Mã máy · Chức năng (Đếm/Tách) · Nguồn (online / bù offline) · Phiên bản. "Đã dùng" = `COUNTIFS` trên tab này.
Tab `Mô tả license` bên phải giải thích từng cột.

**Luồng**
- Bấm Đếm / Tách, có mạng (timeout 4 s): đọc dòng của máy; chưa có → thêm dòng (2/2, Dùng thử). Còn lượt + không Khoá → chạy; xong → thêm 1 dòng nhật ký. Hết lượt / Khoá → popup chặn.
- Không mạng: bộ đếm offline lưu 2 nơi (thư mục dữ liệu cạnh exe + AppData/registry); < 2 → chạy; có mạng lại → ghi bù nhật ký rồi xoá bộ đếm.
- Popup: "Đã dùng hết lượt … của máy này. Liên hệ Sao Mai để được cấp thêm" + mã máy (Copy) + saomaiso.com / SĐT / email + nút **Kiểm tra lại**.

**Thiết lập Sheet (làm tay 1 lần)**: khoá vùng các cột 🔒 và cột công thức chỉ cho chủ Sheet; service account vẫn thêm dòng / sửa cột thông tin.

## 6. Rủi ro còn lại (đã chấp nhận)
- Có khoá trong exe → xoá dòng nhật ký để reset số lần; đọc / sửa thống kê; xoá dữ liệu tab không khoá. Giảm nhẹ: khoá vùng, sao lưu Sheet định kỳ (Version history).
- Mã máy đổi khi cài lại hệ điều hành → máy cũ cần cấp lại.
- AGPL: người nhận exe vẫn có quyền đòi mã nguồn và bỏ phần license.
- Repo riêng tư: GitHub Actions miễn phí 2.000 phút/tháng, macOS ×10 → ~8 lần phát hành/tháng; cần sửa CI đẩy file sang repo phát hành công khai và sửa nút Kiểm tra cập nhật.
- Google Sheets API: 60 lần đọc/phút/người dùng — đủ cho quy mô hiện tại.

## 7. Tiêu chí hoàn thành
- Máy mới, có mạng: Sheet tự thêm dòng 2/2 Dùng thử; lần 3 hiện popup chặn kèm mã máy.
- Sao Mai sửa hạn mức trên Sheet → bấm Kiểm tra lại → dùng tiếp được.
- Trạng thái Khoá → chặn ngay kể cả còn lượt.
- Offline: chạy được 2 lần, lần 3 chặn; có mạng lại → nhật ký có 2 dòng "bù offline".
- Service account không sửa được cột 🔒 (thử ghi → Sheet từ chối).
- Mất mạng / Sheet lỗi giữa chừng không làm treo app (timeout, thông báo rõ).

## 8. Bước tiếp
1. Lập kế hoạch triển khai (license trong app, CI + repo phát hành, sửa Kiểm tra cập nhật).
2. Thông tin liên hệ cho popup (SĐT, email).
3. Thiết lập khoá vùng trên Sheet (hướng dẫn trong kế hoạch).

## Câu hỏi còn mở
- SĐT / email hiển thị trên popup.
- Tên repo phát hành công khai (đề xuất `quangtv1/dema4-releases`).
- Hạn mức cho máy dev / máy nội bộ Sao Mai (đề xuất trạng thái "Đã cấp", hạn mức lớn).
