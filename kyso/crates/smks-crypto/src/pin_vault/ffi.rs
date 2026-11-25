//! DPAPI (`CryptProtectData` / `CryptUnprotectData`) cho [`super`]. Mọi `unsafe` của module nằm ở đây.

use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use zeroize::{Zeroize, Zeroizing};

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr().cast_mut(),
    }
}

/// Chép bộ đệm do DPAPI cấp rồi giải phóng (xoá trắng trước khi trả về hệ thống nếu `wipe`).
fn take(out: CRYPT_INTEGER_BLOB, wipe: bool) -> Vec<u8> {
    if out.pbData.is_null() {
        return Vec::new();
    }
    // SAFETY: DPAPI cấp `pbData` dài `cbData` byte bằng LocalAlloc; chỉ đọc trong phạm vi đó.
    let slice = unsafe { std::slice::from_raw_parts_mut(out.pbData, out.cbData as usize) };
    let copy = slice.to_vec();
    if wipe {
        slice.zeroize();
    }
    // SAFETY: bộ đệm cấp bởi DPAPI, giải phóng đúng một lần bằng LocalFree.
    let _ = unsafe { LocalFree(Some(HLOCAL(out.pbData.cast()))) };
    copy
}

pub(super) fn protect(plain: &[u8], entropy: &[u8]) -> Result<Vec<u8>, String> {
    let input = blob(plain);
    let extra = blob(entropy);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: các blob trỏ tới bộ đệm còn sống trong lời gọi; DPAPI chỉ đọc chúng. Không hiện giao diện.
    unsafe {
        CryptProtectData(
            &input,
            None,
            Some(&extra),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|e| format!("{} (0x{:08X})", e.message(), e.code().0))?;
    Ok(take(out, false))
}

pub(super) fn unprotect(sealed: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    let input = blob(sealed);
    let extra = blob(entropy);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: như `protect`.
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            Some(&extra),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|e| format!("{} (0x{:08X})", e.message(), e.code().0))?;
    Ok(Zeroizing::new(take(out, true)))
}
