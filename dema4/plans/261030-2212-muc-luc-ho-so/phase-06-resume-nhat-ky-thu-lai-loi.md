---
phase: 6
title: "Resume nhat ky thu lai loi"
status: pending
priority: P3
dependencies: [3]
effort: "2 ngày"
---

# Phase 6: Chạy tiếp (Resume), nhật ký, thử lại hồ sơ lỗi

## Overview
Mẻ lớn bị dừng / tắt app / Word treo vẫn chạy tiếp được; mỗi hồ sơ có dòng nhật ký; hồ sơ lỗi thử lại riêng.

## Requirements
- Checkpoint `muc_luc_tien_do.json` trong thư mục ra: khoá mẻ (hash file dữ liệu + mẫu + ràng buộc), danh sách hồ sơ đã xong (DOCX/PDF). Bật "Chạy tiếp" (Cấu hình) → bỏ qua hồ sơ đã xong khi khoá mẻ trùng.
- Nhật ký `muc_luc_log.txt` (UTF-8, tab, có tiêu đề — cùng kiểu `ketqua_tach.txt`): STT, hồ sơ, số VB, DOCX, PDF, trạng thái, lỗi. Lỗi phân loại: Dữ liệu · Mẫu · Ghi file · PDF.
- `muc_luc_loi.json` → popup kết quả có nút **Thử lại hồ sơ lỗi** (chỉ chạy các hồ sơ đó).
- Nút **Dừng** trong popup tiến độ: dừng trước hồ sơ kế tiếp, giữ checkpoint; lượt license vẫn tính nếu đã có ≥ 1 DOCX (như Tách).

## Related Code Files
- Create: `dema4_rs/src/muc_luc/tien_do.rs`
- Modify: pipeline tạo (phase 3), popup kết quả, Cấu hình

## Tests trước
1. `resume_bo_qua_ho_so_da_xong`: chạy 22, dừng sau 10 → chạy lại chỉ ghép 12.
2. `khoa_me_doi_thi_chay_lai_tu_dau`: đổi ràng buộc → không resume.
3. `log_va_loi_json`: hồ sơ lỗi dữ liệu → log có dòng lỗi, `muc_luc_loi.json` liệt kê; thử lại chỉ chạy hồ sơ đó.
4. `dung_giua_chung_tinh_luot` (KhoGia).

## Implementation Steps
1. Test đỏ → `tien_do.rs` → nối pipeline → UI nút Dừng / Thử lại → xanh.

## Success Criteria
- [ ] Tắt app giữa mẻ 1000 hồ sơ, mở lại chạy tiếp đúng chỗ.
- [ ] Log đọc được bằng Excel (tab).

## Risk Assessment
- Ghi checkpoint mỗi hồ sơ tốn I/O → ghi gộp mỗi 1 s / 20 hồ sơ, ghi nguyên tử (tạm + đổi tên).
