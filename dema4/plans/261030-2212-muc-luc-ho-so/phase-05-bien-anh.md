---
phase: 5
title: "Bien anh"
status: pending
priority: P3
dependencies: [2, 3]
effort: "3 ngày"
---

# Phase 5: Biến ảnh (chữ ký, logo, con dấu, QR)

## Overview
Port 2 cách chèn ảnh của bản gốc: đổi ruột ảnh theo Alt Text `image_*` (giữ vị trí/kích thước/bao chữ) và token chữ `{image_*}` chèn ảnh inline kích thước tự nhiên.

## Requirements
- `mau.rs`: liệt kê biến ảnh — `wp:docPr@descr|title|name` bắt đầu bằng `image` (body + header/footer); token `{image…}`.
- Nguồn biến ảnh: **Hằng** (1 file áp mọi hồ sơ) hoặc **Theo cột** (đường dẫn từ cột, dòng đầu nhóm; token trong hàng mẫu → theo từng dòng).
- Đổi ruột: thêm part `word/media/imageN.<ext>` + relationship mới, trỏ `a:blip@r:embed` sang đó; content type cho png/jpg.
- Token: run chứa `w:drawing` inline, cx/cy từ kích thước ảnh (EMU = px × 9525 theo DPI ảnh, mặc định 96).
- Ô rỗng / file thiếu / không đọc được → giữ ảnh gốc (Alt Text) hoặc run rỗng (token), ghi cảnh báo.
- UI: Nguồn "Ảnh: [Chọn file]" hoặc "Cột ▾" cho biến ảnh; xem trước hiển thị ảnh.

## Related Code Files
- Create: `dema4_rs/src/muc_luc/anh.rs`
- Modify: `mau.rs`, `ghep.rs`, `gui_muc_luc.rs`, `Cargo.toml` (đã có `image`? kiểm; nếu chưa dùng `imagesize` nhẹ để lấy kích thước)

## Tests trước
1. `liet_ke_bien_anh` fixture DOCX có 2 ảnh alt `image_logo`, `image_chu_ky`.
2. `doi_ruot_anh_giu_kich_thuoc`: sau ghép, `a:blip` trỏ part mới, `wp:extent` không đổi, part mới đúng bytes PNG.
3. `token_anh_inline`: `{image_qr}` → `w:drawing` với extent theo kích thước ảnh.
4. `anh_thieu_giu_goc`: đường dẫn không tồn tại → DOCX y như không ghép ảnh + cảnh báo.
5. Đối chứng Core gốc với 1 mẫu có ảnh (`#[ignore]`).

## Implementation Steps
1. Test 1–4 đỏ → `anh.rs` → xanh. 2. UI nguồn ảnh. 3. Đối chứng.

## Success Criteria
- [ ] Chữ ký/logo/con dấu "Behind Text" giữ đúng vị trí khi mở bằng Word.
- [ ] QR mỗi hồ sơ một ảnh theo cột.

## Risk Assessment
- Ảnh trong header/footer có rels riêng (`header1.xml.rels`) → xử lý theo part chứa ảnh, test riêng.
