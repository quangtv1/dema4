---
phase: 3
title: "Kho Xóa ghim + xuất PDF"
status: completed
priority: P1
dependencies: [2]
---

# Phase 3: Kho Xóa ghim + xuất PDF

## Overview
Lưu vùng + nhật ký theo id nội dung PDF trong kho riêng; xuất PDF kết quả bằng **vẽ lại trang như Python** (thay ảnh tại chỗ để giai đoạn 8).

## Requirements
### Kho `<goc_du_lieu()>/kho_xoa_ghim.bin` (red team #13, #14)
- Là **lần mở thứ 2** của `Kho` hiện có (`Kho::mo(file)`), **không tách module**, không đổi định dạng / hành vi kho Tách; thêm vào `kho.rs` 1 bảng chung `khac: HashMap<(loại, Id), vị trí>` mà `nap` / `nen` giữ lại (~30 dòng). Test kho cũ phải xanh nguyên.
- Bản ghi:
  - `VUNG_XG`: vùng riêng từng file / trang (khoá id PDF): danh sách vùng (loại, toạ độ tỉ lệ / px@120, ngưỡng, áp trang).
  - `VUNG_THU_MUC_XG`: vùng chung + tuỳ chọn của thư mục (khoá `khoa_duong_dan(thư mục)`; đổi tên thư mục → chọn lại vùng chung — ghi rõ trong F1).
  - `XOA_GHIM` (nhật ký, user chốt giữ): thời điểm (ms u64), id PDF kết quả, mã cấu hình (băm vùng + tuỳ chọn), số trang, DPI, AI bật / tắt, trang đã xử lý (dải nén). ~50 B / PDF.
  - Không lưu kết quả AI (chỉ RAM khi đang làm); không có `CAU_HINH_XG`.
- Ghi: vùng ghi trễ ngắn sau mỗi thay đổi; nhật ký ghi **sau khi** file ra đã đổi tên thành công.
- Nhận file theo nội dung: dùng chung id + bảng đường dẫn kho Tách, tra theo lô qua `ids_cua` ngoài luồng UI.
- Khoá: cần cả 2 kho thì khoá Tách → Xóa ghim, không giữ 2 khoá qua I/O. Khoá 1 cửa sổ không giữ được (`kho.rs:731` cho qua) → kho Xóa ghim **chỉ đọc**, báo người dùng.
- Dọn dữ liệu cũ: lấy tập id còn sống → dọn kho Xóa ghim → dọn kho Tách.
- Xuất / Nhập phiên (user chốt giữ): thêm mục **tuỳ chọn** `xoa_ghim.bin` vào gói, **không tăng `PHIEN_BAN_GOI`**; nhập: gộp theo thời điểm (kẹp lệch đồng hồ như `gop::kep`), bản sau thắng; kiểm dữ liệu (toạ độ trong trang, x1 < x2, ngưỡng 0–255, số vùng giới hạn); vùng nhập vào hiện cho người dùng xem trước khi dùng.

### Trạng thái "đã xóa" (red team #4)
- Chỉ "đã xóa" khi file ra ở đường dẫn tính lại từ nguồn hiện tại **tồn tại và id khớp** id trong nhật ký; ngược lại "kết quả mất / đã đổi" và được chạy lại. Nhập từ máy khác không có file → "chưa xóa trên máy này".

### Xuất PDF
- Thư mục ra `<tên nguồn>_xoaghim` (cạnh nguồn hoặc thư mục ở Cấu hình, dùng chung ô với Tách); giữ thư mục con; **giữ tên file gốc**; không có file chữ; file gốc không sửa.
- Hàm mới `thu_muc_ra_hau_to(nguon, cha, hau_to)`; `thu_muc_ra` cũ bọc lại → 0 nơi gọi phải đổi.
- **Chuẩn hoá** NFC + không phân biệt hoa / thường (Win / Mac) trước kiểm "ra trong nguồn"; file ra đã có mà nhật ký chỉ ra thuộc **nguồn khác** → không ghi đè, hỏi; quét thư mục cha bỏ `*_xoaghim` (red team #5). Chạy lại cùng nguồn → ghi đè (user chốt, nhắc F1 nên Xóa ghim trước Tách).
- Ghi `.tmp` → `sync_all` → đổi tên → (đồng bộ thư mục khi được) → ghi nhật ký.
- Trang không có vùng áp dụng → chép nguyên trang. Trang có vùng → vẽ ở DPI (Gốc = DPI ảnh scan lớn nhất, chặn 600 như Python; hoặc 72–300) → xử lý → JPEG (chất lượng chọn) / "Nén đen trắng" → trang mới.
- Song song theo file trên **pool rayon riêng** của Xóa ghim; Dừng được.

## Related Code Files
- Create: `dema4_rs/src/xoa_ghim/kho.rs`, `dema4_rs/src/xoa_ghim/xuat.rs`
- Modify: `dema4_rs/src/tach_vb/kho.rs` (bảng `khac`), `dema4_rs/src/tach_vb/tach.rs` (`thu_muc_ra_hau_to`), `dema4_rs/src/phien_lam_viec.rs` (+ tests), `dema4_rs/src/gui.rs` (Dọn dữ liệu cũ)

## Tests (viết trước)
- Kho: test kho Tách cũ xanh; kho Xóa ghim ghi / mở lại / bản ghi hỏng bỏ qua / nén giữ loại mới; bản 2.4 nén `kho_tach.bin` không đụng kho Xóa ghim; thứ tự Dọn; chỉ đọc khi khoá cửa sổ lỗi.
- Gói phiên: có / không có `xoa_ghim.bin`; gói mới nhập được vào logic 2.4 (`PHIEN_BAN_GOI` = 2); gộp theo thời điểm; dữ liệu sai bị bỏ.
- Xuất: tên giữ nguyên, thư mục con, ra trong nguồn (khác hoa / thường, NFD) bị chặn, nguồn khác cùng tên không bị ghi đè, không sót `.tmp`, trạng thái sau khi xoá / di chuyển thư mục ra; trang không xử lý giống byte nội dung; PDF số; file hỏng → lỗi của file đó; 100k PDF đo dung lượng + thời gian mở kho.

## Success Criteria
- [ ] Kết quả mở được ở Tách / Đếm (kiểm khi có màn — GĐ4); trạng thái luôn đúng với file trên đĩa (test `trang_thai`).

## Kết quả (2026-11-25)
- `tach_vb/kho.rs`: loại bản ghi ≥ `KHAC_TU` (5) giữ trong bảng `khac`, `nen` giữ lại; `ghi_khac` / `khac` / `khoa_khac` / `don_khac` / `id_con_song`; `mo_chi_doc`; `giu_khoa_that()` (phân biệt "không khoá được"). Test kho cũ xanh.
- `xoa_ghim/kho.rs`: `VUNG_FILE`=5, `VUNG_THU_MUC`=6, `NHAT_KY`=7; mỗi nội dung mở đầu phiên bản + mốc; kiểm toạ độ / số vùng ≤ 500 / trang trong số trang / DPI ≤ 600 / chất lượng 1–100. Kho chỉ đọc khi tiến trình không giữ khoá app. Đo 100k PDF (nhật ký + 3 vùng): 26 MB, mở 0,03 s.
- `xoa_ghim/xuat.rs`: chép nguyên trang không vùng (`cat_pdf::ghep_trang` tách từ `cat_bang_ghep`), trang có vùng vẽ ở DPI ảnh scan (chặn 600) → JPEG **4:2:0** (như OpenCV; mặc định bộ mã hoá 4:4:4 ở q ≥ 90 làm file to hơn ~18%) / "Nén đen trắng" (2 mức Flate, xám JPEG ≤ 90); ghi tạm → `sync_all` → đổi tên → đồng bộ thư mục → nhật ký; không đè kết quả của nguồn khác; pool rayon riêng (½ nhân, 1–8).
- Thử trên 12 file mẫu thật: dung lượng 15,9 MB ≈ Python 15,9 MB; ảnh trang khớp bản Python cùng cấu hình (lệch chỉ do nén JPEG). Trang khác `_clean.pdf` ở 2 trang là do bản Python khi đó bật AI.
- `tach.rs`: `thu_muc_ra_hau_to` + `nam_trong` (NFC + chữ thường) — Tách dùng chung.
- Gói phiên: mục tuỳ chọn `xoa_ghim.bin` (vùng thư mục + vùng file + nhật ký), `PHIEN_BAN_GOI` vẫn 2; nhập: mốc sau thắng (kẹp lệch đồng hồ), dữ liệu sai bỏ, chỉ PDF có trên máy; báo cáo nhập có dòng Xóa ghim.
- Dọn dữ liệu cũ: id còn sống (kho Tách) → dọn kho Xóa ghim → dọn kho Tách; mở app: nén lần lượt 2 kho, không giữ 2 khoá.
- Review độc lập (0 hồi quy kho Tách / gói phiên), đã sửa: gói lạ khai số dải khổng lồ → cấp ~34 GB (giờ kiểm khớp số byte); ổ không khoá được → kho Xóa ghim chỉ đọc làm nhập phiên dừng giữa chừng (giờ chỉ đọc khi **biết chắc** nơi khác giữ khoá); nhật ký nhập thay nhật ký máy này → "kết quả mất" (giờ chỉ nhận khi máy này chưa có); chặn ghi đè quét mọi nhật ký mỗi file (giờ bảng id kết quả → nguồn dựng 1 lần / lượt).
- Để GĐ4 (cần màn): báo "kho chỉ đọc", vùng ghi trễ sau mỗi thay đổi, vùng nhập từ gói hiện cho xem trước khi dùng. Lấy id nguồn qua `id_cua` / `ids_cua` (ghi bảng đường dẫn của kho Tách) — không thì Dọn dữ liệu cũ coi PDF chỉ dùng ở Xóa ghim là đã mất.
