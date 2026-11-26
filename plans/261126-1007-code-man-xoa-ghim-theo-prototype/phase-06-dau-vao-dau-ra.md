---
phase: 6
title: "Đầu vào / đầu ra: File / Thư mục, kiểu ghi, thư mục đích, Ghi đè"
status: completed
priority: P1
dependencies: [2]
---

# Phase 6: Đầu vào / đầu ra

## Overview
Nửa phải thanh công cụ + phần lõi đi kèm: mở **1 file** hoặc **cả thư mục**, ⊗ đóng, **kiểu ghi** Nhân bản thư mục / Cạnh file gốc, **thư mục đích tuỳ chọn**, **Ghi đè file gốc** (bật trong Cấu hình), ẩn file kết quả `*_xoaghim.pdf` khi quét. Đây là phần duy nhất đổi lõi `xoa_ghim/*` và định dạng kho.

## Giao diện (nửa phải, lưới nhãn 62 px)
| Hàng | Nội dung |
|---|---|
| Đầu vào: | ô đường dẫn (chưa mở: chữ mờ "Chọn 1 file PDF hoặc cả thư mục…"; đã mở: đường dẫn file / thư mục + ⊗ tròn cuối ô → đóng, về màn ban đầu) · nút **[PDF] File** 88×26 · nút **[📁] Thư mục** 88×26 |
| File đích: | combo 150 px **Nhân bản thư mục / Cạnh file gốc** · ô đường dẫn mẫu (`…/Lo_01_xoaghim/{thư mục con}/{gốc}.pdf` hoặc `<thư mục file gốc>/{gốc}_xoaghim.pdf`; dài > 42 ký tự → `…/` + 3 cấp cuối; tooltip đủ) + nhãn **mặc định** / ⊗ (khi đã chọn thư mục khác) · nút **[📁] Thư mục** 88×26 (mờ khi Cạnh file gốc hoặc Ghi đè) |
| (trống) | ☐ Nhận diện layout (+ "tải mô hình") · ☐ Tô nền trắng · [Xóa vùng vẽ] · [Hoàn tác] — tự xuống dòng gọn |
| Khi bật Ghi đè | combo + nút mờ; ô ghi "Ghi đè lên file gốc" màu `#b45309`, nhãn "ghi đè" nền `#fffbeb` |
| Tiêu đề Đích | theo kiểu ghi: `<đích>/<tương đối>` · `<thư mục>/<tên>_xoaghim.pdf` · Ghi đè = đường dẫn nguồn |

## Lõi — thiết kế
1. **Đích của từng file** — hàm duy nhất `xoa_ghim::xuat::dich_cua(nguon_goc, file, cau_hinh_ghi) -> PathBuf`:
   - `NhanBan { thu_muc: None }` → `thu_muc_ra_hau_to(goc, None, "_xoaghim")` + đường dẫn tương đối (như hiện tại).
   - `NhanBan { thu_muc: Some(d) }` → `d` + tương đối; **từ chối** nếu `d` nằm trong nguồn (`nam_trong`) — báo lỗi ở ô.
   - `CanhFile` → cùng thư mục, tên `<gốc>_xoaghim.pdf`.
   - `GhiDe` → chính file nguồn (ghi file tạm cùng thư mục rồi đổi tên nguyên tử — `ghi_chac` đã làm vậy).
2. **Quét**: `ds_viec` bỏ mọi file tên kết thúc `_xoaghim.pdf` (không phân biệt hoa thường) và thư mục `*_xoaghim` (đã có) — áp cho mọi kiểu ghi.
3. **Trạng thái**:
   - Nhân bản / Cạnh file: giữ `trang_thai(nk, id_dich, ma)` hiện có với `dich_cua(...)` mới.
   - Ghi đè: sau khi ghi, nội dung nguồn = kết quả → tra `nguon_theo_ket_qua` (đã có: id kết quả → id nguồn): nếu id file hiện tại là 1 kết quả đã biết → `DaXoa { dung_cau_hinh: ma == nk.ma_cau_hinh }`; file chưa từng xử lý → `ChuaXoa`. Không có "mất kết quả" ở chế độ này.
4. **Kho** (`xoa_ghim/kho.rs::ThuMuc`): thêm `ghi: CauHinhGhi { kieu: NhanBan|CanhFile, thu_muc: Option<PathBuf>, ghi_de: bool }`. Mã hoá: **thêm khối đuôi có phiên bản** sau dữ liệu cũ; `giai_ma` bản cũ đọc thiếu khối → mặc định NhanBan / None / false; bản cũ đọc kho mới bỏ qua phần đuôi (kiểm thử bằng mẫu nhị phân bản 2.5.x). Không đưa `ghi` vào `ma_cau_hinh` (vị trí ghi không đổi nội dung).
5. **Mở 1 file**: `ManXoaGhim.mot_file: Option<PathBuf>`; danh sách = [file]; vùng chung lấy theo thư mục cha (như mở thư mục rồi chọn file); thư mục mặc định = `<cha>_xoaghim`.
6. **⊗ Đầu vào**: `dong()` — huỷ worker, xoá danh sách / lọc / chọn vùng, `ds_mo = None` (danh sách về mặc định mở rộng).

## Related Code Files
- Modify: `xoa_ghim/xuat.rs` (`dich_cua`, `ds_viec` lọc `_xoaghim.pdf`, trạng thái ghi đè), `xoa_ghim/lo.rs` (dùng `dich_cua`), `xoa_ghim/kho.rs` (`CauHinhGhi` + mã hoá có phiên bản), `gui_xoa_ghim.rs` (mot_file, dong, mo_pdf), `gui_xoa_ghim/thanh_tren.rs` (nửa phải), `gui_xoa_ghim/chay.rs` (đích theo kiểu ghi, nút "Mở thư mục kết quả")
- Reuse: `tach_vb::tach::{thu_muc_ra_hau_to, nam_trong}`, `xuat::ghi_chac`, `kho::nguon_theo_ket_qua`

## Implementation Steps
1. `CauHinhGhi` + mã hoá có phiên bản + test: đọc kho 2.5.0 (mẫu nhị phân tạo bằng code cũ trong test), ghi–đọc vòng tròn.
2. `dich_cua` + test 4 kiểu (gồm từ chối thư mục đích nằm trong nguồn, đường dẫn tương đối nhiều cấp).
3. `ds_viec` bỏ `*_xoaghim.pdf` + test.
4. Trạng thái ghi đè qua `nguon_theo_ket_qua` + test (ghi đè 1 file rồi đọc lại trạng thái = DaXoa; đổi vùng → dung_cau_hinh=false).
5. Giao diện nửa phải như bảng; ô đường dẫn rút gọn + tooltip.
6. Mở 1 file / thư mục từ nút, menu Tệp tin, biểu tượng màn trống, kéo thả (đã có).
7. kittest: đổi kiểu ghi → tiêu đề Đích đổi; bật Ghi đè → combo / nút mờ.

## Success Criteria
- [ ] Test lõi qua; kho cũ đọc được; bản 2.5.0 đọc kho mới không vỡ.
- [ ] Không bao giờ quét lại file `*_xoaghim.pdf`.
- [ ] Ghi đè: nếu lỗi giữa chừng, file nguồn còn nguyên (ghi tạm + đổi tên).

## Risk Assessment
- **Ghi đè mất dữ liệu gốc** — chỉ bật trong Cấu hình, có cảnh báo; ghi nguyên tử; không xoá gì khác.
- Bản cũ đọc kho mới: phải kiểm thật bằng binary 2.5.0 (build tag `v2.5.0` hoặc mẫu bytes cố định trong test).
- Kiểu ghi Cạnh file + license / thống kê trạm: không đổi (đếm theo file ra).
