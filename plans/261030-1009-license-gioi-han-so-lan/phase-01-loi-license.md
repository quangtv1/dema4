---
phase: 1
title: "Loi license"
status: pending
priority: P1
dependencies: []
effort: "1 ngày"
---

# Phase 1: Lõi license (thuần logic, không mạng)

## Overview
Module `src/license.rs`: mã máy, kiểu dữ liệu, hàm quyết định cho chạy / chặn, **sổ lượt trên đĩa** (đặt chỗ trước khi chạy, lần chạy chờ ghi lên trạm, câu trả lời có chữ ký gần nhất). Không gọi mạng → test bằng unit test.

## Requirements
- Mã máy ổn định: không đổi khi đổi tên máy, bật VPN, cắm dock / card mạng. <!-- Red Team: mã máy chỉ từ ID hệ điều hành -->
- Quyết định thuần: (câu trả lời ký gần nhất nếu có, các lần chạy chưa được trạm xác nhận, chức năng) → Cho / Chặn.
- **Đặt chỗ trước khi chạy** (ghi đĩa ngay khi bấm Đếm / Tách): chạy xong có kết quả → giữ; không có kết quả → hoàn; app chết giữa chừng → lần mở sau coi là đã dùng. <!-- Red Team: chống thoát app để khỏi bị tính -->
- Offline: cho tối đa min(2, số còn lại theo câu trả lời ký gần nhất); trạng thái Khoá trong câu trả lời gần nhất → chặn cả khi offline. <!-- Red Team: offline không bỏ qua trạng thái đã biết -->

## Architecture
```rust
pub enum ChucNang { Dem, Tach }
pub enum TrangThai { DungThu, DaCap, Khoa }                  // từ mã ASCII dung_thu | da_cap | khoa; lạ → Khoa
pub struct HanMuc { pub dem: u32, pub tach: u32 }
pub struct TrangThaiMay { pub han: HanMuc, pub da_dung: HanMuc, pub trang_thai: TrangThai }
pub struct ThongTinMay { pub ma: String, pub ten: String, pub mac: String, pub ip_noi_bo: String, pub ip_public: String, pub phien_ban: String }
pub enum QuyetDinh { Cho, HetLuot(ChucNang), HetLuotOffline(ChucNang) }   // Khoá hiển thị như HetLuot (còn lại 0)

pub fn ma_may() -> String                                    // "SM-XXXX-XXXX-XXXX"
fn ma_may_tu(ma_hdh: &str) -> String                         // thuần, test được
pub fn quyet_dinh(da_ky: Option<&TrangThaiMay>, cho_ghi: &[LanChay], cn: ChucNang) -> QuyetDinh

/// Sổ lượt trên đĩa (1 file JSON nhỏ, ghi nguyên tử: file tạm + đổi tên, khoá file khi ghi)
pub struct SoLuot { duong_dan: [PathBuf; 2] }               // 2 thư mục cha khác nhau: dữ liệu ẩn cạnh exe + thư mục người dùng
pub struct LanChay { pub ma: String /* 16 byte ngẫu nhiên hex */, pub cn: ChucNang, pub luc: String, pub trang_thai: DatCho | DaChay }
impl SoLuot {
    fn dat_cho(&self, cn) -> LanChay;           // trước khi chạy
    fn xong(&self, ma: &str, co_ket_qua: bool); // có kết quả → DaChay; không → xoá (hoàn lượt)
    fn da_xac_nhan(&self, ma: &[String]);       // trạm đã ghi → xoá đúng các mã đó
    fn cho_ghi(&self) -> Vec<LanChay>;          // DaChay + DatCho còn sót khi mở app (coi là DaChay)
    fn luu_tra_loi(&self, ky: &TraLoiKy); fn tra_loi(&self) -> Option<TraLoiKy>;   // câu trả lời ký gần nhất
}
```
- Mã máy: chỉ băm ID hệ điều hành — Windows `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` đọc bằng `RegGetValueW` (windows-sys feature `Win32_System_Registry`, **không** gọi `reg.exe` để khỏi nháy cửa sổ); macOS `IOPlatformUUID` qua `ioreg` (không có cửa sổ). Không đọc được → ID ngẫu nhiên tạo 1 lần, lưu cạnh sổ lượt. Tên máy, MAC chỉ gửi làm thông tin.
- Ngẫu nhiên: thêm phụ thuộc trực tiếp `getrandom` (mã lần chạy, nonce).
- Sổ lượt đọc cả 2 nơi, gộp theo mã lần chạy (hợp hai tập); thiếu 1 nơi vẫn đủ.
- `quyet_dinh`: có câu trả lời ký → Khoa → HetLuot; `da_dung + số lần chờ ghi (theo cn)` ≥ `han` → HetLuot; có mạng không quan trọng ở đây. Chưa có câu trả lời ký nào → luật offline: số lần chờ ghi (cn) < 2 → Cho, không thì HetLuotOffline. Có câu trả lời nhưng trạm không liên lạc được → dùng câu trả lời đó, thêm giới hạn chờ ghi ≤ 2.

## Related Code Files
- Create: `dema4_rs/src/license.rs`
- Modify: `dema4_rs/src/main.rs` (`mod license`), `dema4_rs/Cargo.toml` (`getrandom`, windows-sys `Win32_System_Registry`)
- Tham khảo: `dema4_rs/src/tach_vb/du_lieu_an.rs`, `dema4_rs/src/cau_hinh.rs` (`thu_muc_nguoi_dung`), `dema4_rs/src/telegram.rs` (`ten_pc`, `dia_chi_mac`)

## Tests trước
1. `ma_may_chi_tu_id_he_dieu_hanh`: cùng ID → cùng mã; định dạng `^SM-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}$`; ID rỗng → mã ngẫu nhiên được lưu, lần 2 đọc lại đúng mã đó.
2. `quyet_dinh_theo_tra_loi_ky`: dưới hạn → Cho; bằng hạn → HetLuot; Khoa còn lượt → HetLuot; lần chờ ghi cộng vào đã dùng.
3. `quyet_dinh_offline`: chưa có trả lời ký → 0, 1 Cho, 2 HetLuotOffline; có trả lời ký còn 1 → chỉ 1 lần; trả lời ký Khoá + offline → chặn.
4. `dat_cho_va_hoan`: đặt chỗ → `xong(false)` → không còn trong `cho_ghi`; `xong(true)` → còn; đặt chỗ không `xong` (giả app chết) → mở lại thấy trong `cho_ghi`.
5. `so_luot_hai_noi_va_ghi_nguyen_tu`: xoá 1 file → vẫn đủ; file hỏng → bỏ qua file đó; `da_xac_nhan` chỉ xoá đúng mã đã xác nhận, không xoá mã khác thêm sau.
6. `trang_thai_la_coi_la_khoa`: mã "abc" → Khoa.

## Implementation Steps
1. Viết 6 test (thư mục tạm), chạy đỏ.
2. Code `ma_may`, `quyet_dinh`, `SoLuot` (ghi nguyên tử, khoá file `fs2`-free: dùng file khoá `.lock` tạo bằng `create_new`, chờ ≤ 1 s).
3. `cargo test --release license` xanh.

## Success Criteria
- [ ] 6 test xanh, không gọi mạng.
- [ ] Đổi tên máy → mã máy không đổi (thử tay).

## Risk Assessment
- Cài lại hệ điều hành → MachineGuid đổi → máy mới (chấp nhận; chủ Sheet cấp lại).
- Xoá cả 2 file sổ lượt khi chưa từng có câu trả lời ký → thêm 2 lượt offline (chấp nhận; chỉ có tác dụng nếu đồng thời chặn trạm).
