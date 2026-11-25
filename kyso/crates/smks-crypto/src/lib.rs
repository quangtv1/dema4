//! Lõi mật mã của Sao Mai Ký số (kế hoạch mục 6).
//!
//! - [`trust`]: chứng thư CA của Ban Cơ yếu nhúng trong binary, ghim bằng SHA-256.
//! - [`cert`]: đọc thông tin chứng thư X.509.
//! - [`key`]: giao diện khoá ký; khoá phần mềm chỉ để thử (feature `dev-keys`).
//! - [`cms`]: dựng CMS/CAdES theo PAdES B-T.
//! - [`tsa`]: lấy dấu thời gian RFC 3161 từ máy chủ của Ban Cơ yếu.
//! - [`verify`]: xác minh chữ ký, SignerInfo, token dấu thời gian.
//! - `store`, `chain` (chỉ Windows): kho chứng thư và khoá CNG/CAPI trên USB token; chuỗi CA và thu hồi.
//! - `pin_vault` (chỉ Windows): mã hoá mã PIN lưu trên máy bằng DPAPI.

pub mod cert;
#[cfg(windows)]
pub mod chain;
pub mod cms;
pub mod key;
pub mod ltv;
#[cfg(windows)]
pub mod pin_vault;
#[cfg(windows)]
pub mod store;
pub mod trust;
pub mod tsa;
pub mod verify;

/// PKI thử dựng lúc chạy (chỉ có khi test hoặc bật `dev-keys`).
#[cfg(any(test, feature = "dev-keys"))]
pub mod testpki;
