---
phase: 2
title: Loi doc Excel gom nhom ghep DOCX
status: completed
priority: P1
dependencies: []
effort: 5 ngày
---

# Phase 2: Lõi — đọc Excel/CSV, gom nhóm, ghép DOCX

## Overview
Module thuần (không UI) `src/muc_luc/` port Core gốc: Reading · Grouping · Templating (TemplateCompiler + DocxMerger + OpenXmlHelpers) · Validation · FileNameBuilder. Đối chứng với Core gốc.

## Requirements
- **Đọc** (`doc_bang.rs`): XLSX / XLSB / XLS bằng `calamine` (chọn sheet, "Header là dòng" = số dòng Excel vật lý như v1.12.12), CSV bằng `csv` (UTF-8, BOM, ô có xuống dòng). Trả header + iterator dòng (`Vec<String>`), không giữ cả bảng khi không cần.
- **Gom nhóm** (`gom_nhom.rs`): dòng liên tiếp cùng giá trị cột nhóm → `HoSo { khoa, dong: Vec<Dong> }`; key đã đóng mà gặp lại → lỗi "hồ sơ không liên tiếp: <khoa> dòng <n>". Trường cấp hồ sơ lấy dòng đầu nhóm.
- **Biên dịch mẫu** (`mau.rs`): mở DOCX (zip) một lần → `MauDaBien { bytes, bang: idx, hang_mau: idx, bien_dong, bien_ho_so, bien_tu_dong, bien_anh, thu_tu_bien }`. Hàng mẫu = hàng có nhiều biến cấp dòng nhất. Vá token bị Word cắt vụn (gộp run giữa `{`…`}` về run mở), gộp run liền kề cùng định dạng.
- **Ghép** (`ghep.rs`): clone bytes → thay biến cấp hồ sơ (body + header/footer) → nhân hàng mẫu theo từng dòng → xoá hàng mẫu → `{trang_so}`/`{tong_so_trang}` thành field PAGE/NUMPAGES → bytes DOCX. Không còn token `{x}` nào bị bỏ sót khi đã bind.
- **Ràng buộc biến**: `Nguon = Cot(i) | Hang(String) | TuDong` (+ `Anh`, `TuTach` ở phase 5/7). Tự khớp cột↔biến (bỏ dấu, chữ thường, `_`/khoảng trắng như `TextUtil` gốc).
- **Kiểm tra** (`kiem_tra.rs`): file tồn tại, mapping đủ, mẫu có bảng + hàng mẫu, hồ sơ liên tiếp, cột bắt buộc → danh sách lỗi/cảnh báo.
- **Tên file**: mẫu `MLHS_{so_ho_so}` → bỏ ký tự cấm, trùng thêm `_2`, `_3` (không phân biệt hoa thường).
- Đa luồng ghép bằng `rayon` (mẫu bất biến dùng chung).

## Architecture
```
Excel/CSV ─doc_bang─▶ (header, dòng…) ─gom_nhom─▶ HoSo* ─┐
mẫu DOCX ─mau::bien_dich─▶ MauDaBien (1 lần) ────────────┴▶ ghep::ghep(mau, ho_so, rang_buoc) ─▶ bytes DOCX
```
XML: `quick-xml` reader/writer theo sự kiện trên `word/document.xml`, `word/header*.xml`, `word/footer*.xml`; giữ nguyên part khác (zip copy).

## Related Code Files
- Create: `dema4_rs/src/muc_luc.rs`, `dema4_rs/src/muc_luc/{doc_bang,gom_nhom,mau,ghep,kiem_tra,ten_file}.rs`
- Create: `dema4_rs/assets/muc_luc/mau-01..04.docx` (chép từ repo gốc sau khi kiểm không có dữ liệu cá nhân)
- Create: `dema4_rs/tests_data/muc_luc/` fixture tổng hợp (CSV + XLSX nhỏ) cho CI
- Modify: `dema4_rs/Cargo.toml` (`calamine`, `csv`, `zip`, `quick-xml`), `dema4_rs/src/main.rs` (`mod muc_luc`)

## Tests trước (đỏ → xanh)
1. `doc_xlsx_header_dong_n` / `doc_csv_bom_xuong_dong` / `doc_xlsb`: fixture tổng hợp.
2. `gom_nhom_lien_tiep` + `gom_nhom_khong_lien_tiep_bao_loi`.
3. `bien_dich_4_mau`: danh mục biến từng mẫu khớp bảng README gốc (Mẫu 01: `so_ho_so`, `don_vi`, `chi_nhanh`, `nguoi_lap` | dòng `stt`, `so_ky_hieu`, `ngay_thang`, `tac_gia`, `trich_yeu`, `to_so`, `ghi_chu`; …).
4. `va_token_cat_vun`: DOCX có `{so_` + `ho_so}` ở 2 run → nhận diện được, định dạng run mở giữ nguyên.
5. `ghep_nhan_hang_mau`: nhóm 3 dòng → bảng 3 hàng dữ liệu, hàng mẫu biến mất, không còn `{`.
6. `field_trang`: `{trang_so}` → `w:fldSimple PAGE` (hoặc fldChar).
7. `doi_chung_core_goc` (`#[ignore]`, chạy tay + trước phát hành): Excel mẫu (trong `temp/`) × 4 mẫu → so `document.xml` chuẩn hoá (text theo đoạn + số hàng bảng + field) với DOCX do Core gốc sinh.
8. `ten_file_trung_ky_tu_cam`.

## Implementation Steps
1. Dựng harness đối chứng: `dotnet` build `MucLucHoSo.Core` + console nhỏ sinh DOCX cho 4 mẫu → lưu `temp/doi_chung/` (không commit).
2. Viết test 1–8 (đỏ).
3. Cài `doc_bang`, `gom_nhom` → xanh 1–2.
4. Cài `mau` (vá token, gộp run, nhận hàng mẫu) → xanh 3–4.
5. Cài `ghep` + field + header/footer → xanh 5–7.
6. `kiem_tra`, `ten_file` → xanh 8; `cargo test --release` toàn bộ.

## Success Criteria
- [x] Test 1–6, 8 xanh; test 7 (đối chứng Core gốc, 4 mẫu × 22 hồ sơ, so chữ + định dạng run + số hàng + field) xanh trên máy dev.
- [x] 88 hồ sơ ghép ~0,4 s.

## Ghi chú thực hiện (30/10)
- Cây XML tự viết (`xml.rs`) thay OpenXML SDK; CSV tự tách (`tach_csv`) vì crate `csv` bỏ dòng trống và đánh số dòng sai sau đó.
- CSV dò dấu phân cách trên chính dòng tiêu đề (bản gốc: dòng 1).
- Excel: `calamine` feature `dates` (hệ ngày 1904, ngày tràn không panic).
- Đầu/chân trang theo relationship của document.xml.
- Review: báo cáo `reports/` (code-reviewer) — đã sửa các mục Cao/Trung bình; chấp nhận: số thực mũ nhỏ khác .NET (`1E-05`), biến ghép trùng (UI không tạo).

## Risk Assessment
- XML Word đa dạng (run lồng `w:ins`, `w:smartTag`, `w:proofErr`) → bám đúng thuật toán gốc + test đối chứng; gặp cấu trúc lạ thì giữ nguyên token và báo cảnh báo thay vì làm hỏng file.
- Mẫu 04 dựng từ PDF → chỉ đòi tương đương với Core gốc, không với PDF.
- `calamine` đọc XLSB/XLS khác ExcelDataReader (ngày tháng dạng số) → định dạng ngày như hiển thị Excel; test riêng.
