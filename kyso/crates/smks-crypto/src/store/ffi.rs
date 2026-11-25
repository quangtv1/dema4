//! Lời gọi CryptoAPI / CNG cho [`super`]. Mọi `unsafe` của module kho chứng thư nằm ở đây, mỗi khối có SAFETY.
//! Handle được bọc RAII: kho (`CertCloseStore`), ngữ cảnh chứng thư (`CertFreeCertificateContext`), khoá
//! (`NCryptFreeObject` / `CryptReleaseContext`), băm CAPI (`CryptDestroyHash`).

use std::ffi::c_void;

use windows::Win32::Security::Cryptography::{
    AT_SIGNATURE, BCRYPT_PKCS1_PADDING_INFO, BCRYPT_SHA256_ALGORITHM, CALG_SHA_256, CERT_CONTEXT,
    CERT_KEY_PROV_INFO_PROP_ID, CERT_KEY_SPEC, CERT_NCRYPT_KEY_SPEC, CERT_OPEN_STORE_FLAGS,
    CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W, CERT_STORE_READONLY_FLAG,
    CERT_SYSTEM_STORE_CURRENT_USER, CERT_SYSTEM_STORE_LOCAL_MACHINE,
    CRYPT_ACQUIRE_ALLOW_NCRYPT_KEY_FLAG, CRYPT_KEY_PROV_INFO, CertCloseStore,
    CertDuplicateCertificateContext, CertEnumCertificatesInStore, CertFreeCertificateContext,
    CertGetCertificateContextProperty, CertOpenStore, CryptAcquireCertificatePrivateKey,
    CryptCreateHash, CryptDestroyHash, CryptReleaseContext, CryptSetHashParam, CryptSetProvParam,
    CryptSignHashW, HCERTSTORE, HCRYPTPROV_OR_NCRYPT_KEY_HANDLE, HP_HASHVAL, NCRYPT_FLAGS,
    NCRYPT_HANDLE, NCRYPT_KEY_HANDLE, NCRYPT_PAD_PKCS1_FLAG, NCRYPT_PIN_PROPERTY,
    NCRYPT_WINDOW_HANDLE_PROPERTY, NCryptFreeObject, NCryptSetProperty, NCryptSignHash,
    PKCS_7_ASN_ENCODING, PP_CLIENT_HWND, PP_KEYEXCHANGE_PIN, PP_SIGNATURE_PIN, X509_ASN_ENCODING,
};
use windows::core::{BOOL, PCWSTR, w};
use zeroize::Zeroizing;

use super::StoreLocation;

/// Kho chứng thư đang mở.
pub(super) struct Store(HCERTSTORE);

impl Drop for Store {
    fn drop(&mut self) {
        // SAFETY: handle mở bởi CertOpenStore và chỉ đóng một lần ở đây.
        let _ = unsafe { CertCloseStore(Some(self.0), 0) };
    }
}

/// Một ngữ cảnh chứng thư (đếm tham chiếu của Windows).
pub(super) struct Context(*const CERT_CONTEXT);

// SAFETY: CERT_CONTEXT là đối tượng chỉ đọc, đếm tham chiếu an toàn luồng; ta không sửa nội dung của nó.
unsafe impl Send for Context {}
// SAFETY: như trên; mọi truy cập qua `&Context` chỉ đọc.
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: con trỏ có được từ CertDuplicateCertificateContext / CertCreate…, giải phóng đúng một lần.
        let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
    }
}

impl Context {
    pub(super) fn der(&self) -> Vec<u8> {
        // SAFETY: ngữ cảnh hợp lệ tới khi Drop; pbCertEncoded trỏ tới cbCertEncoded byte do Windows quản lý.
        unsafe {
            let ctx = &*self.0;
            std::slice::from_raw_parts(ctx.pbCertEncoded, ctx.cbCertEncoded as usize).to_vec()
        }
    }

    /// Thông tin nhà cung cấp khoá ghi trong kho (không mở khoá, nên không làm middleware hỏi PIN).
    pub(super) fn key_provider(&self) -> Option<ProvInfo> {
        let mut size = 0u32;
        // SAFETY: lần gọi đầu chỉ hỏi kích thước; con trỏ ngữ cảnh hợp lệ.
        unsafe {
            CertGetCertificateContextProperty(self.0, CERT_KEY_PROV_INFO_PROP_ID, None, &mut size)
        }
        .ok()?;
        if size == 0 {
            return None;
        }
        // Bộ đệm căn theo u64 vì CRYPT_KEY_PROV_INFO chứa con trỏ.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: bộ đệm đủ `size` byte, căn 8 byte; Windows ghi CRYPT_KEY_PROV_INFO cùng các chuỗi nó trỏ tới
        // vào trong chính bộ đệm này.
        unsafe {
            CertGetCertificateContextProperty(
                self.0,
                CERT_KEY_PROV_INFO_PROP_ID,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                &mut size,
            )
        }
        .ok()?;
        // SAFETY: Windows vừa ghi một CRYPT_KEY_PROV_INFO hợp lệ ở đầu bộ đệm; các PWSTR trỏ vào bộ đệm còn sống.
        unsafe {
            let info = &*buffer.as_ptr().cast::<CRYPT_KEY_PROV_INFO>();
            Some(ProvInfo {
                provider: wide(info.pwszProvName.0),
            })
        }
    }

    /// Mở khoá bí mật gắn với chứng thư. Middleware của token có thể hỏi PIN ở lần ký đầu.
    pub(super) fn acquire_key(&self) -> std::result::Result<KeyHandle, String> {
        let mut handle = HCRYPTPROV_OR_NCRYPT_KEY_HANDLE::default();
        let mut spec = CERT_KEY_SPEC(0);
        let mut caller_free = BOOL(0);
        // SAFETY: các con trỏ ra trỏ tới biến cục bộ; ngữ cảnh hợp lệ.
        unsafe {
            CryptAcquireCertificatePrivateKey(
                self.0,
                CRYPT_ACQUIRE_ALLOW_NCRYPT_KEY_FLAG,
                None,
                &mut handle,
                Some(&mut spec),
                Some(&mut caller_free),
            )
        }
        .map_err(|e| format!("{} (0x{:08X})", e.message(), e.code().0))?;
        let free = caller_free.as_bool();
        Ok(if spec == CERT_NCRYPT_KEY_SPEC {
            KeyHandle::Ncrypt {
                handle: NCRYPT_KEY_HANDLE(handle.0),
                free,
            }
        } else {
            KeyHandle::Capi {
                prov: handle.0,
                spec: spec.0,
                free,
            }
        })
    }

    /// Chép ngữ cảnh (tăng đếm tham chiếu).
    #[cfg(test)]
    pub(super) fn duplicate(&self) -> Context {
        // SAFETY: ngữ cảnh hợp lệ; bản sao được Context mới giải phóng.
        Context(unsafe { CertDuplicateCertificateContext(Some(self.0)) })
    }

    /// Nhận quyền sở hữu một ngữ cảnh do API khác tạo (chỉ dùng trong test).
    #[cfg(test)]
    pub(super) fn from_owned(ptr: *const CERT_CONTEXT) -> Option<Context> {
        (!ptr.is_null()).then_some(Context(ptr))
    }
}

pub(super) struct ProvInfo {
    pub provider: String,
}

/// Chuỗi UTF-16 kết thúc bằng 0.
///
/// # Safety
/// `ptr` rỗng hoặc trỏ tới chuỗi UTF-16 kết thúc bằng 0 còn sống.
unsafe fn wide(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: theo hợp đồng của hàm, chuỗi kết thúc bằng 0.
    unsafe {
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

pub(super) fn open_my(location: StoreLocation) -> std::result::Result<Store, String> {
    let scope = match location {
        StoreLocation::CurrentUser => CERT_SYSTEM_STORE_CURRENT_USER,
        StoreLocation::LocalMachine => CERT_SYSTEM_STORE_LOCAL_MACHINE,
    };
    let flags =
        CERT_OPEN_STORE_FLAGS(scope | CERT_STORE_READONLY_FLAG.0 | CERT_STORE_OPEN_EXISTING_FLAG.0);
    let name = w!("MY");
    // SAFETY: provider hệ thống nhận pvPara là tên kho dạng PCWSTR hằng.
    unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            None,
            flags,
            Some(name.as_ptr().cast::<c_void>()),
        )
    }
    .map(Store)
    .map_err(|e| format!("{} (0x{:08X})", e.message(), e.code().0))
}

/// Mọi chứng thư trong kho (mỗi phần tử là bản sao có đếm tham chiếu riêng).
pub(super) fn enumerate(store: &Store) -> Vec<Context> {
    let mut out = Vec::new();
    let mut previous: *mut CERT_CONTEXT = std::ptr::null_mut();
    loop {
        // SAFETY: `previous` là null hoặc ngữ cảnh vừa trả về từ chính kho này; hàm giải phóng nó và trả về
        // ngữ cảnh kế tiếp (hoặc null khi hết).
        let next = unsafe {
            CertEnumCertificatesInStore(
                store.0,
                (!previous.is_null()).then_some(previous.cast_const()),
            )
        };
        if next.is_null() {
            break;
        }
        // SAFETY: `next` hợp lệ; bản sao do Context giữ, `next` vẫn thuộc vòng duyệt.
        out.push(Context(unsafe {
            CertDuplicateCertificateContext(Some(next))
        }));
        previous = next;
    }
    out
}

/// Lỗi của lời gọi CryptoAPI / CNG: thông báo của Windows kèm mã HRESULT (để nhận ra PIN sai, token bị khoá).
#[derive(Debug)]
pub(super) struct FfiError {
    pub message: String,
    pub code: i32,
}

impl From<windows::core::Error> for FfiError {
    fn from(e: windows::core::Error) -> Self {
        Self {
            message: format!("{} (0x{:08X})", e.message(), e.code().0),
            code: e.code().0,
        }
    }
}

/// Khoá bí mật đã mở.
pub(super) enum KeyHandle {
    Ncrypt {
        handle: NCRYPT_KEY_HANDLE,
        free: bool,
    },
    Capi {
        prov: usize,
        spec: u32,
        free: bool,
    },
}

// SAFETY: handle CNG/CAPI dùng được từ luồng khác; StoreKey bọc KeyHandle trong Mutex nên mỗi lúc chỉ một
// luồng dùng (token chỉ xử lý một lệnh một lúc).
unsafe impl Send for KeyHandle {}

impl Drop for KeyHandle {
    fn drop(&mut self) {
        match *self {
            // SAFETY: chỉ giải phóng khi CryptAcquireCertificatePrivateKey báo bên gọi phải giải phóng.
            KeyHandle::Ncrypt { handle, free: true } => {
                let _ = unsafe { NCryptFreeObject(NCRYPT_HANDLE::from(handle)) };
            }
            // SAFETY: như trên, cho HCRYPTPROV.
            KeyHandle::Capi {
                prov, free: true, ..
            } => {
                let _ = unsafe { CryptReleaseContext(prov, 0) };
            }
            _ => {}
        }
    }
}

impl KeyHandle {
    pub(super) fn is_cng(&self) -> bool {
        matches!(self, KeyHandle::Ncrypt { .. })
    }

    /// Gắn cửa sổ cha cho hộp PIN của middleware (CNG: thuộc tính "HWND Handle" của khoá).
    pub(super) fn set_window(&self, hwnd: isize) {
        if let KeyHandle::Ncrypt { handle, .. } = *self {
            let bytes = hwnd.to_ne_bytes();
            // SAFETY: handle khoá hợp lệ; dữ liệu là giá trị HWND theo kích thước con trỏ. Lỗi được bỏ qua (một số
            // provider không hỗ trợ thuộc tính này), như bản cũ.
            let _ = unsafe {
                NCryptSetProperty(
                    NCRYPT_HANDLE::from(handle),
                    NCRYPT_WINDOW_HANDLE_PROPERTY,
                    &bytes,
                    NCRYPT_FLAGS(0),
                )
            };
        }
    }

    /// Đưa mã PIN cho middleware (spike S6): CNG đặt thuộc tính `SmartCardPin` của khoá, CAPI đặt
    /// `PP_SIGNATURE_PIN` / `PP_KEYEXCHANGE_PIN` của provider. `Err`: provider không nhận (hoặc PIN sai nếu provider
    /// kiểm ngay); khi đó middleware vẫn tự hỏi PIN lúc ký như thường. Bộ đệm PIN được xoá trắng sau khi dùng.
    pub(super) fn set_pin(&self, pin: &str) -> std::result::Result<(), FfiError> {
        match *self {
            KeyHandle::Ncrypt { handle, .. } => {
                // UTF-16LE có ký tự kết thúc, như hộp PIN của Windows truyền vào KSP.
                let bytes: Zeroizing<Vec<u8>> = Zeroizing::new(
                    pin.encode_utf16()
                        .chain(std::iter::once(0))
                        .flat_map(u16::to_le_bytes)
                        .collect(),
                );
                // SAFETY: handle khoá hợp lệ; `bytes` sống tới hết lời gọi.
                unsafe {
                    NCryptSetProperty(
                        NCRYPT_HANDLE::from(handle),
                        NCRYPT_PIN_PROPERTY,
                        &bytes,
                        NCRYPT_FLAGS(0),
                    )
                }
                .map_err(FfiError::from)
            }
            KeyHandle::Capi { prov, spec, .. } => {
                if !pin.is_ascii() {
                    return Err(FfiError {
                        message: "Mã PIN có ký tự ngoài bảng ASCII, CSP đời cũ không nhận.".into(),
                        code: 0,
                    });
                }
                let bytes: Zeroizing<Vec<u8>> =
                    Zeroizing::new(pin.bytes().chain(std::iter::once(0)).collect());
                let param = if spec == AT_SIGNATURE.0 || spec == 0 {
                    PP_SIGNATURE_PIN
                } else {
                    PP_KEYEXCHANGE_PIN
                };
                // SAFETY: prov là HCRYPTPROV hợp lệ; `bytes` là chuỗi ANSI kết thúc bằng 0, sống tới hết lời gọi.
                unsafe { CryptSetProvParam(prov, param, bytes.as_ptr(), 0) }.map_err(FfiError::from)
            }
        }
    }

    /// Ký băm SHA-256. `rsa`: dùng PKCS#1 v1.5; ngược lại ECDSA (CNG trả `r ‖ s`).
    pub(super) fn sign(
        &self,
        digest: &[u8; 32],
        rsa: bool,
    ) -> std::result::Result<Vec<u8>, FfiError> {
        match *self {
            KeyHandle::Ncrypt { handle, .. } => sign_cng(handle, digest, rsa),
            KeyHandle::Capi { prov, spec, .. } => sign_capi(prov, spec, digest),
        }
    }
}

fn sign_cng(
    handle: NCRYPT_KEY_HANDLE,
    digest: &[u8; 32],
    rsa: bool,
) -> std::result::Result<Vec<u8>, FfiError> {
    let padding = BCRYPT_PKCS1_PADDING_INFO {
        pszAlgId: BCRYPT_SHA256_ALGORITHM,
    };
    let (info, flags) = if rsa {
        (
            Some((&raw const padding).cast::<c_void>()),
            NCRYPT_PAD_PKCS1_FLAG,
        )
    } else {
        (None, NCRYPT_FLAGS(0))
    };
    let err = FfiError::from;
    let mut size = 0u32;
    // SAFETY: lần đầu chỉ hỏi kích thước chữ ký; `padding` sống tới hết hàm.
    unsafe { NCryptSignHash(handle, info, digest, None, &mut size, flags) }.map_err(err)?;
    let mut signature = vec![0u8; size as usize];
    // SAFETY: bộ đệm đủ `size` byte.
    unsafe { NCryptSignHash(handle, info, digest, Some(&mut signature), &mut size, flags) }
        .map_err(err)?;
    signature.truncate(size as usize);
    Ok(signature)
}

struct CapiHash(usize);

impl Drop for CapiHash {
    fn drop(&mut self) {
        // SAFETY: handle băm tạo bởi CryptCreateHash, huỷ đúng một lần.
        let _ = unsafe { CryptDestroyHash(self.0) };
    }
}

fn sign_capi(prov: usize, spec: u32, digest: &[u8; 32]) -> std::result::Result<Vec<u8>, FfiError> {
    let err = FfiError::from;
    let mut raw = 0usize;
    // SAFETY: prov là HCRYPTPROV hợp lệ; `raw` nhận handle băm.
    unsafe { CryptCreateHash(prov, CALG_SHA_256, 0, 0, &mut raw) }.map_err(err)?;
    let hash = CapiHash(raw);
    // SAFETY: HP_HASHVAL đọc đúng 32 byte băm SHA-256 từ `digest`.
    unsafe { CryptSetHashParam(hash.0, HP_HASHVAL, digest.as_ptr(), 0) }.map_err(err)?;
    let spec = if spec == 0 { AT_SIGNATURE.0 } else { spec };
    let mut size = 0u32;
    // SAFETY: lần đầu chỉ hỏi kích thước.
    unsafe { CryptSignHashW(hash.0, spec, PCWSTR::null(), 0, None, &mut size) }.map_err(err)?;
    let mut signature = vec![0u8; size as usize];
    // SAFETY: bộ đệm đủ `size` byte.
    unsafe {
        CryptSignHashW(
            hash.0,
            spec,
            PCWSTR::null(),
            0,
            Some(signature.as_mut_ptr()),
            &mut size,
        )
    }
    .map_err(err)?;
    signature.truncate(size as usize);
    // CAPI trả chữ ký little-endian; PKCS#1 cần big-endian.
    signature.reverse();
    Ok(signature)
}

/// Cửa sổ cha cho hộp PIN của mọi CSP (CAPI), gọi một lần khi có cửa sổ chính (giống bản cũ).
pub(super) fn set_capi_window(hwnd: isize) {
    let value = hwnd;
    // SAFETY: PP_CLIENT_HWND với hProv = 0 đặt giá trị toàn cục; pbData trỏ tới HWND còn sống trong lời gọi.
    let _ = unsafe { CryptSetProvParam(0, PP_CLIENT_HWND, (&raw const value).cast::<u8>(), 0) };
}

// ----------------------------------------------------------------------------------------------
// Khoá tạm cho test (tạo trong kho khoá của người dùng hiện tại rồi xoá ngay sau test)
// ----------------------------------------------------------------------------------------------

#[cfg(test)]
pub(super) mod testkeys {
    use windows::Win32::Foundation::SYSTEMTIME;
    use windows::Win32::Security::Cryptography::{
        CERT_CREATE_SELFSIGN_FLAGS, CERT_X500_NAME_STR, CRYPT_ALGORITHM_IDENTIFIER,
        CRYPT_INTEGER_BLOB, CRYPT_KEY_FLAGS, CertCreateSelfSignCertificate, CertStrToNameW,
        CryptAcquireContextW, CryptGenKey, MS_ENH_RSA_AES_PROV_W, MS_KEY_STORAGE_PROVIDER,
        NCRYPT_LENGTH_PROPERTY, NCRYPT_PROV_HANDLE, NCryptCreatePersistedKey, NCryptDeleteKey,
        NCryptFinalizeKey, NCryptOpenStorageProvider, PROV_RSA_AES, szOID_ECDSA_SHA256,
        szOID_RSA_SHA256RSA,
    };
    use windows::core::{HSTRING, PCSTR, PWSTR};

    use super::*;

    /// Khoá tạm cùng chứng thư tự ký trỏ tới nó; Drop xoá khoá khỏi kho khoá.
    pub struct TempKey {
        pub context: Context,
        cleanup: Cleanup,
    }

    enum Cleanup {
        Cng(NCRYPT_KEY_HANDLE, NCRYPT_PROV_HANDLE),
        Capi(HSTRING),
    }

    impl Drop for TempKey {
        fn drop(&mut self) {
            match &self.cleanup {
                // SAFETY: handle khoá tạo trong test; NCryptDeleteKey xoá và giải phóng nó.
                Cleanup::Cng(key, prov) => unsafe {
                    let _ = NCryptDeleteKey(*key, 0);
                    let _ = NCryptFreeObject(NCRYPT_HANDLE(prov.0));
                },
                Cleanup::Capi(container) => {
                    let mut prov = 0usize;
                    // SAFETY: CRYPT_DELETEKEYSET xoá container tạm theo tên.
                    let _ = unsafe {
                        CryptAcquireContextW(
                            &mut prov,
                            container,
                            MS_ENH_RSA_AES_PROV_W,
                            PROV_RSA_AES,
                            0x10,
                        )
                    };
                }
            }
        }
    }

    fn name_blob(cn: &str) -> Vec<u8> {
        let text = HSTRING::from(format!("CN={cn}, O=Sao Mai thử nghiệm, C=VN"));
        let mut size = 0u32;
        // SAFETY: lần đầu hỏi kích thước, lần sau ghi vào bộ đệm đủ lớn.
        unsafe {
            CertStrToNameW(
                X509_ASN_ENCODING,
                &text,
                CERT_X500_NAME_STR,
                None,
                None,
                &mut size,
                None,
            )
            .unwrap();
            let mut out = vec![0u8; size as usize];
            CertStrToNameW(
                X509_ASN_ENCODING,
                &text,
                CERT_X500_NAME_STR,
                None,
                Some(out.as_mut_ptr()),
                &mut size,
                None,
            )
            .unwrap();
            out.truncate(size as usize);
            out
        }
    }

    fn self_sign(
        key: HCRYPTPROV_OR_NCRYPT_KEY_HANDLE,
        cn: &str,
        prov_info: &CRYPT_KEY_PROV_INFO,
        sig_oid: PCSTR,
    ) -> Context {
        let mut name = name_blob(cn);
        let blob = CRYPT_INTEGER_BLOB {
            cbData: name.len() as u32,
            pbData: name.as_mut_ptr(),
        };
        let alg = CRYPT_ALGORITHM_IDENTIFIER {
            pszObjId: windows::core::PSTR(sig_oid.0.cast_mut()),
            ..Default::default()
        };
        let start = SYSTEMTIME {
            wYear: 2025,
            wMonth: 1,
            wDay: 1,
            ..Default::default()
        };
        let end = SYSTEMTIME {
            wYear: 2035,
            wMonth: 1,
            wDay: 1,
            ..Default::default()
        };
        // SAFETY: mọi con trỏ trỏ tới biến cục bộ sống suốt lời gọi.
        let ptr = unsafe {
            CertCreateSelfSignCertificate(
                Some(key),
                &blob,
                CERT_CREATE_SELFSIGN_FLAGS(0),
                Some(prov_info),
                Some(&alg),
                Some(&start),
                Some(&end),
                None,
            )
        };
        Context::from_owned(ptr).expect("CertCreateSelfSignCertificate")
    }

    /// Khoá CNG (Microsoft Software KSP) tạm: `ecdsa` thì P-256, ngược lại RSA 2048.
    pub fn cng(ecdsa: bool) -> TempKey {
        let name = HSTRING::from(format!(
            "SaoMai-test-{}",
            std::process::id() as u64 * 1000 + u64::from(ecdsa)
        ));
        let alg = if ecdsa {
            windows::Win32::Security::Cryptography::NCRYPT_ECDSA_P256_ALGORITHM
        } else {
            windows::Win32::Security::Cryptography::NCRYPT_RSA_ALGORITHM
        };
        let mut prov = NCRYPT_PROV_HANDLE::default();
        let mut key = NCRYPT_KEY_HANDLE::default();
        // SAFETY: các handle ra trỏ tới biến cục bộ; tên khoá là HSTRING còn sống.
        unsafe {
            NCryptOpenStorageProvider(&mut prov, MS_KEY_STORAGE_PROVIDER, 0).unwrap();
            NCryptCreatePersistedKey(
                prov,
                &mut key,
                alg,
                &name,
                CERT_KEY_SPEC(0),
                windows::Win32::Security::Cryptography::NCRYPT_OVERWRITE_KEY_FLAG,
            )
            .unwrap();
            if !ecdsa {
                NCryptSetProperty(
                    NCRYPT_HANDLE::from(key),
                    NCRYPT_LENGTH_PROPERTY,
                    &2048u32.to_le_bytes(),
                    NCRYPT_FLAGS(0),
                )
                .unwrap();
            }
            NCryptFinalizeKey(key, NCRYPT_FLAGS(0)).unwrap();
        }
        let info = CRYPT_KEY_PROV_INFO {
            pwszContainerName: PWSTR(name.as_ptr().cast_mut()),
            pwszProvName: PWSTR(MS_KEY_STORAGE_PROVIDER.0.cast_mut()),
            dwProvType: 0,
            dwFlags: CRYPT_KEY_FLAGS(0),
            cProvParam: 0,
            rgProvParam: std::ptr::null_mut(),
            dwKeySpec: 0,
        };
        let oid = if ecdsa {
            szOID_ECDSA_SHA256
        } else {
            szOID_RSA_SHA256RSA
        };
        let context = self_sign(
            HCRYPTPROV_OR_NCRYPT_KEY_HANDLE(key.0),
            "Khoá CNG thử",
            &info,
            oid,
        );
        TempKey {
            context,
            cleanup: Cleanup::Cng(key, prov),
        }
    }

    /// Khoá CAPI (Microsoft Enhanced RSA and AES CSP) tạm, RSA 2048, AT_SIGNATURE.
    pub fn capi() -> TempKey {
        let container = HSTRING::from(format!("SaoMai-test-capi-{}", std::process::id()));
        let mut prov = 0usize;
        let mut key = 0usize;
        // SAFETY: tạo container mới (CRYPT_NEWKEYSET = 8) và sinh khoá ký 2048 bit.
        unsafe {
            let _ = CryptAcquireContextW(
                &mut prov,
                &container,
                MS_ENH_RSA_AES_PROV_W,
                PROV_RSA_AES,
                0x10,
            );
            CryptAcquireContextW(
                &mut prov,
                &container,
                MS_ENH_RSA_AES_PROV_W,
                PROV_RSA_AES,
                8,
            )
            .unwrap();
            CryptGenKey(
                prov,
                windows::Win32::Security::Cryptography::ALG_ID(AT_SIGNATURE.0),
                CRYPT_KEY_FLAGS(2048 << 16),
                &mut key,
            )
            .unwrap();
        }
        let info = CRYPT_KEY_PROV_INFO {
            pwszContainerName: PWSTR(container.as_ptr().cast_mut()),
            pwszProvName: PWSTR(MS_ENH_RSA_AES_PROV_W.0.cast_mut()),
            dwProvType: PROV_RSA_AES,
            dwFlags: CRYPT_KEY_FLAGS(0),
            cProvParam: 0,
            rgProvParam: std::ptr::null_mut(),
            dwKeySpec: AT_SIGNATURE.0,
        };
        let context = self_sign(
            HCRYPTPROV_OR_NCRYPT_KEY_HANDLE(prov),
            "Khoá CAPI thử",
            &info,
            szOID_RSA_SHA256RSA,
        );
        // SAFETY: khoá và ngữ cảnh CSP của lần tạo không còn dùng; chứng thư mở lại khoá qua prov info.
        unsafe {
            let _ = windows::Win32::Security::Cryptography::CryptDestroyKey(key);
            let _ = CryptReleaseContext(prov, 0);
        }
        TempKey {
            context,
            cleanup: Cleanup::Capi(container),
        }
    }
}
