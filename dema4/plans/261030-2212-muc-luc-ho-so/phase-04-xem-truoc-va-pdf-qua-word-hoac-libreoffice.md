---
phase: 4
title: "Xem truoc va PDF qua Word hoac LibreOffice"
status: pending
priority: P2
dependencies: [3]
effort: "5 ngày"
---

# Phase 4: Xem trước + PDF (Word COM / LibreOffice)

## Overview
DOCX → PDF bằng Word (Windows, COM) hoặc LibreOffice (`soffice`); xem trước = PDF của chính DOCX hồ sơ đang chọn, vẽ bằng MuPDF sẵn có. Tuỳ chọn xuất PDF khi Tạo.

## Requirements
- Trait `ChuyenPdf { fn chuyen(&self, docx: &Path, pdf: &Path) -> Result<()> }`; hiện thực: `Word` (Windows), `LibreOffice`, `Khong` (báo "chỉ xuất DOCX"). Chọn tự động: Word → LibreOffice → Không; hiện tên bộ chuyển trên tab Xem trước.
- **Word**: 1 luồng STA riêng, 1 instance `Word.Application` dùng lại (late-bound `IDispatch` qua crate `windows`), `Visible=false`, `DisplayAlerts=0`, `Documents.Open(ReadOnly)` → `Fields.Update` → `ExportAsFixedFormat(pdf, 17)` → `Close(false)`; watchdog timeout (30 s/file) → kill WINWORD của mình, dựng lại.
- **LibreOffice**: tìm `soffice` (PATH, `C:\Program Files\LibreOffice\program`, `/Applications/LibreOffice.app/Contents/MacOS`) → `soffice --headless --convert-to pdf --outdir <tạm> <docx>` với profile tạm riêng (`-env:UserInstallation`), timeout.
- Tab Xem trước: ghép DOCX hồ sơ đang chọn vào thư mục tạm → PDF → render trang bằng MuPDF (như `xem_trang`), zoom, ◀ ▶ hồ sơ, cache theo (hồ sơ, hash ràng buộc). Chạy nền, UI không đứng; "Đang tạo bản xem trước…".
- Tạo mục lục: bật PDF → sau DOCX, hàng đợi 1 luồng chuyển PDF; tiến độ hiện cả DOCX và PDF.

## Related Code Files
- Create: `dema4_rs/src/muc_luc/pdf.rs` (+ `pdf/word.rs` cfg(windows), `pdf/libre.rs`)
- Modify: `gui_muc_luc.rs` (tab Xem trước, tuỳ chọn PDF), `Cargo.toml` (`windows` features Com/Ole cho cfg(windows))

## Tests trước
1. `tim_soffice` theo danh sách đường dẫn giả lập.
2. `libre_chuyen_pdf` (`#[ignore]` khi không có soffice): DOCX fixture → PDF mở được bằng MuPDF, số trang ≥ 1.
3. `word_chuyen_pdf` (`#[ignore]`, Windows có Word): như trên + watchdog kill khi giả treo.
4. kittest tab Xem trước với bộ chuyển giả (trả PDF fixture) → hiện ảnh trang, ◀ ▶ đổi hồ sơ.

## Implementation Steps
1. Trait + bộ chuyển giả + test 4 → UI.
2. LibreOffice (test 1–2).
3. Word COM (test 3, chạy tay trên Windows / CI windows nếu runner có Word — thường không → ignore).

## Success Criteria
- [ ] Windows có Word: xem trước < 2 s/hồ sơ khi Word đã ấm; PDF khớp DOCX.
- [ ] macOS có LibreOffice: xem trước hoạt động, ghi "gần đúng (LibreOffice)".
- [ ] Không có cả hai: tab Xem trước báo rõ, Tạo DOCX vẫn chạy.

## Risk Assessment
- COM từ Rust phức tạp (VARIANT, BSTR, STA) → cô lập trong `pdf/word.rs`, test tay trên máy Windows của chủ sản phẩm.
- Word treo / hộp thoại ẩn → watchdog + `DisplayAlerts=0` + mở ReadOnly.
- LibreOffice lệch bố cục → nhãn "gần đúng".
