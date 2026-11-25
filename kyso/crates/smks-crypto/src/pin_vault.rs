//! Mã PIN lưu trên máy (kế hoạch D2, bản 1.6 làm sớm theo yêu cầu người phụ trách ngày 20/11/2026).
//!
//! PIN mã hoá bằng DPAPI phạm vi **người dùng hiện tại** (`CryptProtectData`): chỉ đúng tài khoản Windows này trên
//! máy này giải được. Thumbprint của chứng thư làm "entropy" phụ, nên bản mã của chứng thư này không dùng được cho
//! chứng thư khác. Module chỉ mã hoá / giải mã; đọc ghi file `pin.bin` ở tầng ứng dụng. PIN rõ luôn nằm trong
//! [`Zeroizing`] để xoá trắng bộ nhớ khi bỏ, không ghi log.

use base64ct::{Base64, Encoding};
use smks_core::{ErrorCode, Result, SmksError};
use zeroize::Zeroizing;

#[allow(unsafe_code)]
mod ffi;

/// Độ dài PIN chấp nhận (token bit4id / SafeNet: 4–16 ký tự).
pub const PIN_MIN_LEN: usize = 4;
pub const PIN_MAX_LEN: usize = 32;

fn entropy(thumbprint: &str) -> Vec<u8> {
    format!("SaoMai-KySo/pin/{}", thumbprint.trim().to_ascii_uppercase()).into_bytes()
}

/// Kiểm độ dài và ký tự của PIN (không đưa PIN vào thông báo).
pub fn check_pin(pin: &str) -> Result<()> {
    let n = pin.chars().count();
    if !(PIN_MIN_LEN..=PIN_MAX_LEN).contains(&n) || pin.chars().any(char::is_control) {
        return Err(SmksError::new(
            ErrorCode::BadRequest,
            format!("Mã PIN phải dài {PIN_MIN_LEN}–{PIN_MAX_LEN} ký tự."),
        ));
    }
    Ok(())
}

/// Mã hoá PIN của chứng thư `thumbprint`; trả chuỗi Base64 để lưu.
pub fn seal(thumbprint: &str, pin: &str) -> Result<String> {
    check_pin(pin)?;
    let blob = ffi::protect(pin.as_bytes(), &entropy(thumbprint)).map_err(|e| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không mã hoá được mã PIN để lưu: {e}"),
        )
    })?;
    Ok(Base64::encode_string(&blob))
}

/// Giải mã PIN đã lưu của chứng thư `thumbprint`. Lỗi khi bản mã hỏng, của tài khoản / máy khác, hoặc của
/// chứng thư khác.
pub fn open(thumbprint: &str, sealed: &str) -> Result<Zeroizing<String>> {
    let bad = |detail: String| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không đọc được mã PIN đã lưu ({detail}); hãy nhập lại PIN."),
        )
    };
    let blob = Base64::decode_vec(sealed.trim()).map_err(|e| bad(e.to_string()))?;
    let plain = ffi::unprotect(&blob, &entropy(thumbprint)).map_err(bad)?;
    let text = String::from_utf8(plain.to_vec()).map_err(|_| bad("không phải UTF-8".into()))?;
    Ok(Zeroizing::new(text))
}

fn secret_entropy(purpose: &str) -> Vec<u8> {
    format!("SaoMai-KySo/secret/{purpose}").into_bytes()
}

/// Mã hoá một bí mật khác của ứng dụng (không phải PIN), ví dụ bí mật thiết bị do Notification Gateway cấp. Cùng
/// DPAPI phạm vi người dùng hiện tại; `purpose` làm entropy phụ nên bản mã của mục đích này không giải được bằng
/// mục đích khác.
pub fn seal_secret(purpose: &str, secret: &str) -> Result<String> {
    let blob = ffi::protect(secret.as_bytes(), &secret_entropy(purpose)).map_err(|e| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không mã hoá được dữ liệu bí mật để lưu: {e}"),
        )
    })?;
    Ok(Base64::encode_string(&blob))
}

/// Giải mã bí mật đã lưu bằng [`seal_secret`].
pub fn open_secret(purpose: &str, sealed: &str) -> Result<Zeroizing<String>> {
    let bad = |detail: String| {
        SmksError::new(
            ErrorCode::InternalError,
            format!("Không đọc được dữ liệu bí mật đã lưu ({detail})."),
        )
    };
    let blob = Base64::decode_vec(sealed.trim()).map_err(|e| bad(e.to_string()))?;
    let plain = ffi::unprotect(&blob, &secret_entropy(purpose)).map_err(bad)?;
    let text = String::from_utf8(plain.to_vec()).map_err(|_| bad("không phải UTF-8".into()))?;
    Ok(Zeroizing::new(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_round_trip_and_are_bound_to_purpose() {
        let secret = "a".repeat(64);
        let sealed = seal_secret("notify/gateway", &secret).unwrap();
        assert!(!sealed.contains(&secret));
        assert_eq!(
            open_secret("notify/gateway", &sealed).unwrap().as_str(),
            secret
        );
        assert!(open_secret("muc-dich-khac", &sealed).is_err());
        assert!(
            open("notify/gateway", &sealed).is_err(),
            "không giải được như PIN"
        );
    }

    #[test]
    fn seal_and_open_round_trip_bound_to_thumbprint() {
        let sealed = seal("AB12", "123456").unwrap();
        assert!(!sealed.contains("123456"));
        assert_eq!(open("ab12", &sealed).unwrap().as_str(), "123456");
        assert!(open("CD34", &sealed).is_err(), "bản mã gắn với thumbprint");
        assert!(open("AB12", "khong-phai-base64!").is_err());
        assert!(seal("AB12", "12").is_err(), "PIN quá ngắn");
        assert!(seal("AB12", "12\n34").is_err(), "ký tự điều khiển");
    }
}
