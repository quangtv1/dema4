---
phase: 2
title: "Kho license Google Sheet"
status: pending
priority: P1
dependencies: [1]
effort: "0.5 ngày"
---

# Phase 2: Kho license trên Google Sheet

## Overview
Trait `KhoLicense` + bản thật đọc/ghi Sheet và bản giả trong bộ nhớ cho test. App chỉ ghi các cột thông tin (A–F tab `license`) và thêm dòng nhật ký; mọi cột số / hạn mức / trạng thái do công thức hoặc chủ Sheet quản lý.

## Requirements
- Đọc trạng thái 1 máy theo mã máy; chưa có → thêm dòng đăng ký.
- Ghi 1 lần dùng = thêm 1 dòng vào `license_nhat_ky` (chỉ thêm).
- Mọi gọi mạng có timeout ≤ 4 s; lỗi trả `Err`, không panic.

## Architecture
Bố cục tab `license` (phase 4 dựng trên Sheet):

| Cột | Ai ghi | Nội dung |
|---|---|---|
| A Mã máy · B Tên máy · C MAC · D IP public · E Phiên bản · F Ngày đăng ký | App (append `'license'!A:F`) | Thông tin máy |
| G Đã dùng Đếm · H Đã dùng Tách · I Lần dùng cuối | ARRAYFORMULA ở hàng tiêu đề 🔒 | Đếm từ nhật ký |
| J Hạn mức Đếm · K Hạn mức Tách · L Trạng thái · M Đơn vị · N Ghi chú | Chủ Sheet 🔒 | Trống = 2 / 2 / Dùng thử |

Tab `license_nhat_ky`: A Thời điểm · B Mã máy · C Chức năng (Đếm/Tách) · D Nguồn (online / bù offline) · E Phiên bản.

```rust
pub trait KhoLicense: Send + Sync {
    fn doc(&self, ma: &str) -> Result<Option<TrangThaiMay>, String>;
    fn dang_ky(&self, may: &ThongTinMay) -> Result<(), String>;      // append A–F
    fn ghi_lan(&self, ma: &str, cn: ChucNang, bu_offline: bool, so_lan: u32) -> Result<(), String>;
}
pub struct KhoSheet;          // dùng google_sheet
#[cfg(test)] pub struct KhoGia { /* Mutex<HashMap<..>> + Vec nhật ký */ }
```
- `google_sheet.rs` thêm `doc_vung(tab, a1) -> Result<Vec<Vec<String>>>` (values.get, `valueRenderOption=UNFORMATTED_VALUE`) và `them_dong(tab, a1, hang)` (append vào vùng chỉ định, không tạo tab).
- `KhoSheet::doc`: đọc `'license'!A2:L`, tìm hàng có A == mã; parse G,H (số), J,K (trống → 2), L ("Khoá" / "Đã cấp" / khác → Dùng thử).
- Ghi nhiều lần bù offline: append `so_lan` dòng nhật ký trong 1 request.

## Related Code Files
- Create: `dema4_rs/src/license/kho.rs` (hoặc gộp vào `license.rs` nếu < 200 dòng)
- Modify: `dema4_rs/src/google_sheet.rs`

## Tests trước
1. `kho_gia_dang_ky_roi_doc`: đăng ký → doc trả hạn 2/2, đã dùng 0/0, DungThu.
2. `kho_gia_ghi_lan_cong_dung_chuc_nang`: 2 lần Đếm + 1 Tách bù offline → đã dùng 2/1; nhật ký 3 dòng, nguồn đúng.
3. `parse_hang_sheet`: hàng `["SM-…","PC","AA","1.2.3.4","0.6.0","2026-10-30",3,1,"",  "", 10, "Đã cấp"]` → đã dùng 3/1, hạn 2/10, DaCap; ô trống / hàng ngắn không panic.
4. `#[ignore]` `kho_sheet_that`: đăng ký mã thử `SM-TEST-…`, đọc lại thấy dòng (chạy tay sau phase 4).

## Implementation Steps
1. Viết test 1–3 (dùng `KhoGia` + hàm parse thuần), chạy đỏ.
2. Thêm `doc_vung`, `them_dong` vào `google_sheet.rs`; viết `KhoSheet`, `KhoGia`, `parse_hang`.
3. Test xanh; test 4 chạy tay khi Sheet sẵn sàng.

## Success Criteria
- [ ] 3 test xanh không cần mạng.
- [ ] `kho_sheet_that` chạy được trên Sheet thật (sau phase 4).

## Risk Assessment
- Sheets API 60 lần đọc/phút/người dùng: 1 lần đọc mỗi lần bấm — đủ.
- Đọc cả tab khi có hàng nghìn máy: vẫn < 1 MB; nếu lớn hơn dùng `values:batchGet` theo lọc sau (YAGNI).
