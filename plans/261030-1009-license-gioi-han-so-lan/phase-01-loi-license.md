---
phase: 1
title: "Loi license"
status: pending
priority: P1
dependencies: []
effort: "0.5 ngày"
---

# Phase 1: Lõi license (thuần logic, không mạng)

## Overview
Module `src/license.rs`: mã máy, kiểu dữ liệu trạng thái máy, hàm quyết định cho chạy / chặn, bộ đếm offline lưu 2 nơi. Không gọi mạng → test được hết bằng unit test.

## Requirements
- Mã máy ổn định qua các lần chạy, không đổi khi bật VPN / cắm thêm card mạng.
- Quyết định thuần: (trạng thái online nếu có, số lần offline đã dùng, chức năng) → Cho / Chặn(lý do).
- Bộ đếm offline khó reset bằng cách xoá 1 file: lưu 2 nơi, đọc lấy giá trị lớn hơn.

## Architecture
```rust
pub enum ChucNang { Dem, Tach }
pub enum TrangThai { DungThu, DaCap, Khoa }          // ô trống trên Sheet = DungThu
pub struct HanMuc { pub dem: u32, pub tach: u32 }     // ô trống = 2
pub struct TrangThaiMay { pub han: HanMuc, pub da_dung: HanMuc, pub trang_thai: TrangThai }
pub enum QuyetDinh { Cho, HetLuot(ChucNang), Khoa, HetLuotOffline(ChucNang) }

pub fn ma_may() -> String                    // "SM-XXXX-XXXX-XXXX"
fn ma_may_tu(ten: &str, mac: &str, ma_he_dieu_hanh: &str) -> String   // thuần, test được
pub fn quyet_dinh(online: Option<&TrangThaiMay>, offline: &HanMuc, cn: ChucNang) -> QuyetDinh

pub struct BoDemOffline { noi: [PathBuf; 2] }  // <du_lieu_an goc>/license_offline.txt + <thư mục người dùng>/license_offline.txt
impl BoDemOffline { fn doc(&self) -> HanMuc /*max 2 nơi*/; fn cong(&self, cn); fn xoa(&self) }
```
- Mã hệ điều hành: Windows `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` (đọc bằng `reg query` hoặc windows-sys), macOS `ioreg -rd1 -c IOPlatformExpertDevice` → `IOPlatformUUID`; lỗi → chuỗi rỗng.
- Băm: SHA-256 (đã có `rsa::sha2`) của `ten|mac|ma_hdh`, lấy 12 ký tự hex đầu, viết hoa, nhóm 4.
- Định dạng file offline: `dem=N\ntach=M\n` (1 dòng / chức năng); số lần chờ ghi bù = chính các số này.
- `quyet_dinh`: Khoa → Khoa; online có → so `da_dung` với `han` (cộng thêm offline chưa ghi bù); online None → offline.cn < 2 thì Cho, ngược lại HetLuotOffline.

## Related Code Files
- Create: `dema4_rs/src/license.rs`
- Modify: `dema4_rs/src/main.rs` (khai báo `mod license`), `dema4_rs/src/telegram.rs` (dùng lại `ten_pc`, `dia_chi_mac`)
- Đọc tham khảo: `dema4_rs/src/tach_vb/du_lieu_an.rs` (thư mục dữ liệu cạnh app), `dema4_rs/src/cau_hinh.rs` (`thu_muc_nguoi_dung`)

## Tests trước (viết, chạy đỏ, rồi code)
1. `ma_may_on_dinh_va_dung_dinh_dang`: cùng đầu vào → cùng mã; đổi MAC → khác mã; khớp regex `^SM-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}$`.
2. `quyet_dinh_online`: dưới hạn → Cho; bằng hạn → HetLuot; Khoa còn lượt → Khoa; DaCap hạn 100 dùng 99 → Cho; offline chưa ghi bù cộng vào số đã dùng.
3. `quyet_dinh_offline`: 0, 1 → Cho; 2 → HetLuotOffline; Đếm và Tách đếm riêng.
4. `bo_dem_offline_lay_lon_nhat_hai_noi`: ghi 2 lần, xoá 1 file → vẫn đọc 2; file hỏng → coi như 0 nhưng nơi còn lại vẫn đúng; `xoa` xoá cả 2.

## Implementation Steps
1. Viết 4 test trên (thư mục tạm cho bộ đếm), chạy → đỏ.
2. Code `ma_may_tu`, `quyet_dinh`, `BoDemOffline`; `ma_may()` gọi đọc mã hệ điều hành theo `cfg!(windows/macos)`.
3. `cargo test --release license` xanh.

## Success Criteria
- [ ] 4 test xanh; không có gọi mạng trong module.
- [ ] Trên máy dev: `ma_may()` trả cùng mã qua 2 lần chạy.

## Risk Assessment
- Cài lại hệ điều hành → MachineGuid đổi → máy thành máy mới (chấp nhận; chủ Sheet cấp lại).
- `ioreg` / `reg` không có trên môi trường lạ → chuỗi rỗng, vẫn ra mã từ tên + MAC.
