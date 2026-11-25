//! Kho chứng thư Windows và khoá ký trên USB token (kế hoạch 6.1, 6.2; port `CertificateProvider.cs`,
//! `StoreSigningKey.cs`).
//!
//! - Quét `CurrentUser\My` rồi `LocalMachine\My`, gộp theo thumbprint, ưu tiên bản ký được.
//! - "Có khoá bí mật" và tên provider đọc từ thuộc tính của chứng thư trong kho, **không mở khoá** (khác bản
//!   cũ) nên liệt kê không làm middleware hỏi PIN.
//! - Khoá là token phần cứng khi tên provider chứa dấu hiệu token và không phải provider phần mềm của
//!   Microsoft (Phụ lục A).
//! - [`StoreKey`]: khoá CNG (`NCryptSignHash`, PKCS#1 v1.5 / ECDSA) hoặc CAPI (`CryptSignHashW`). Mọi lệnh ký
//!   qua một `Mutex` vì token chỉ xử lý một lệnh một lúc. Hộp PIN gắn vào cửa sổ chính.

use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};

use serde::Serialize;
use smks_core::consts::{signing as k, token};
use smks_core::{ErrorCode, Result, SmksError};

use crate::cert::Cert;
use crate::key::{SigAlg, SigningKey, ecdsa_raw_to_der};

#[allow(unsafe_code)]
mod ffi;

/// Vị trí kho chứng thư.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreLocation {
    CurrentUser,
    LocalMachine,
}

/// Nguồn của chứng thư, như bản cũ: token phần cứng, kho người dùng, kho máy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CertSource {
    UsbToken,
    Local,
    Server,
}

/// Một chứng thư trong kho, kèm lý do không ký được (nếu có).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertEntry {
    pub thumbprint: String,
    pub common_name: String,
    pub subject: String,
    pub issuer: String,
    pub issuer_common_name: String,
    pub serial_number: String,
    /// Giây Unix.
    pub valid_from: i64,
    pub valid_to: i64,
    pub source: CertSource,
    pub key_provider: Option<String>,
    pub has_private_key: bool,
    pub is_expired: bool,
    pub allows_signing: bool,
    pub is_trusted: bool,
    pub can_sign: bool,
    pub reason: Option<String>,
}

/// Kết quả quét kho.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertScan {
    pub certificates: Vec<CertEntry>,
    pub diagnostics: Vec<String>,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// `dd/MM/yyyy HH:mm:ss` theo giờ Việt Nam (như bản cũ hiển thị trong thông báo).
pub fn format_vietnam_time(unix: i64) -> String {
    let secs = unix + i64::from(smks_core::consts::app::VIETNAM_UTC_OFFSET_SECONDS);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{day:02}/{month:02}/{year:04} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Provider là token phần cứng: có dấu hiệu token và không phải provider phần mềm của Microsoft.
pub fn is_hardware_provider(provider: &str) -> bool {
    let p = provider.to_lowercase();
    !p.is_empty()
        && !token::SOFTWARE_PROVIDER_MARKERS
            .iter()
            .any(|m| p.contains(m))
        && token::HARDWARE_PROVIDER_MARKERS
            .iter()
            .any(|m| p.contains(m))
}

const REASON_NO_KEY: &str =
    "Chứng thư số không có khóa bí mật trên máy này (chỉ xem được, không ký được).";
const REASON_NO_USAGE: &str =
    "Chứng thư số không được cấp quyền ký (KeyUsage thiếu digitalSignature/nonRepudiation).";
const REASON_UNTRUSTED: &str = "Chứng thư số không thuộc CA Ban Cơ yếu (chain không về Root G1 lẫn Root G2 đã ghim). \
Chữ ký tạo bằng chứng thư này sẽ bị đánh giá là không hợp lệ.";

fn expired_reason(cert: &Cert) -> String {
    let (from, to) = cert.validity();
    format!(
        "Chứng thư số ngoài thời hạn hiệu lực ({} - {}).",
        format_vietnam_time(from),
        format_vietnam_time(to)
    )
}

/// Quét kho chứng thư. `trusted` quyết định chứng thư có về CA Ban Cơ yếu không (module `chain`).
pub fn list_certificates(trusted: &dyn Fn(&Cert) -> bool) -> CertScan {
    let now = now_unix();
    let mut entries: Vec<CertEntry> = Vec::new();
    let mut diagnostics = Vec::new();
    for (location, source) in [
        (StoreLocation::CurrentUser, CertSource::Local),
        (StoreLocation::LocalMachine, CertSource::Server),
    ] {
        let label = match location {
            StoreLocation::CurrentUser => "CurrentUser\\My",
            StoreLocation::LocalMachine => "LocalMachine\\My",
        };
        let store = match ffi::open_my(location) {
            Ok(store) => store,
            Err(e) => {
                diagnostics.push(format!("{label}: không mở được ({e})"));
                continue;
            }
        };
        let contexts = ffi::enumerate(&store);
        diagnostics.push(format!("{label}: {} chứng thư", contexts.len()));
        for ctx in contexts {
            let Ok(cert) = Cert::from_der(&ctx.der()) else {
                continue;
            };
            let prov = ctx.key_provider();
            let has_private_key = prov.is_some();
            let provider = prov.map(|p| p.provider).filter(|p| !p.is_empty());
            let is_hardware = provider.as_deref().is_some_and(is_hardware_provider);
            let is_expired = !cert.is_valid_at(now);
            let allows_signing = cert.allows_signing();
            let is_trusted = trusted(&cert);
            let reason = if !has_private_key {
                Some(REASON_NO_KEY.to_owned())
            } else if is_expired {
                Some(expired_reason(&cert))
            } else if !allows_signing {
                Some(REASON_NO_USAGE.to_owned())
            } else if !is_trusted {
                Some(REASON_UNTRUSTED.to_owned())
            } else {
                None
            };
            let (valid_from, valid_to) = cert.validity();
            entries.push(CertEntry {
                thumbprint: cert.thumbprint(),
                common_name: cert.common_name(),
                subject: cert.subject(),
                issuer: cert.issuer(),
                issuer_common_name: cert.issuer_common_name(),
                serial_number: cert.serial_hex(),
                valid_from,
                valid_to,
                source: if is_hardware {
                    CertSource::UsbToken
                } else {
                    source
                },
                key_provider: provider,
                has_private_key,
                is_expired,
                allows_signing,
                is_trusted,
                can_sign: reason.is_none(),
                reason,
            });
        }
    }
    CertScan {
        certificates: merge(entries),
        diagnostics,
    }
}

/// Gộp theo thumbprint (giữ bản ký được), rồi xếp: ký được trước, sau đó theo tên.
fn merge(entries: Vec<CertEntry>) -> Vec<CertEntry> {
    let mut out: Vec<CertEntry> = Vec::new();
    for entry in entries {
        match out
            .iter_mut()
            .find(|e| e.thumbprint.eq_ignore_ascii_case(&entry.thumbprint))
        {
            Some(existing) if !existing.can_sign && entry.can_sign => *existing = entry,
            Some(_) => {}
            None => out.push(entry),
        }
    }
    out.sort_by(|a, b| {
        b.can_sign.cmp(&a.can_sign).then_with(|| {
            a.common_name
                .to_lowercase()
                .cmp(&b.common_name.to_lowercase())
        })
    });
    out
}

fn find_context(thumbprint: &str) -> Option<ffi::Context> {
    for location in [StoreLocation::CurrentUser, StoreLocation::LocalMachine] {
        let Ok(store) = ffi::open_my(location) else {
            continue;
        };
        for ctx in ffi::enumerate(&store) {
            if Cert::from_der(&ctx.der())
                .is_ok_and(|c| c.thumbprint().eq_ignore_ascii_case(thumbprint))
            {
                return Some(ctx);
            }
        }
    }
    None
}

/// Chứng thư còn trong kho không (theo dõi token bị rút giữa lô).
pub fn is_present(thumbprint: &str) -> bool {
    find_context(thumbprint).is_some()
}

static WINDOW: AtomicIsize = AtomicIsize::new(0);

/// Đặt cửa sổ cha cho hộp PIN của middleware (gọi một lần khi có cửa sổ chính).
pub fn set_window_handle(hwnd: isize) {
    WINDOW.store(hwnd, Ordering::SeqCst);
    ffi::set_capi_window(hwnd);
}

/// Khoá ký gắn với một chứng thư trong kho Windows.
pub struct StoreKey {
    cert: Cert,
    algorithm: SigAlg,
    provider: String,
    hardware: bool,
    _context: ffi::Context,
    key: Mutex<ffi::KeyHandle>,
}

impl std::fmt::Debug for StoreKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreKey")
            .field("cert", &self.cert.common_name())
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}

/// HRESULT của thẻ thông minh khi PIN sai / bị khoá (WinSCard, dùng chung cho KSP và CSP).
const SCARD_W_WRONG_CHV: i32 = 0x8010_006B_u32 as i32;
const SCARD_E_INVALID_CHV: i32 = 0x8010_002A_u32 as i32;
const SCARD_W_CHV_BLOCKED: i32 = 0x8010_006C_u32 as i32;

/// Lỗi PIN của middleware thành lỗi nghiệp vụ riêng (để giao diện xoá PIN đã lưu, không tự thử lại).
fn pin_error(code: i32) -> Option<SmksError> {
    match code {
        SCARD_W_WRONG_CHV | SCARD_E_INVALID_CHV => Some(SmksError::new(
            ErrorCode::PinIncorrect,
            "Mã PIN của token không đúng. Lưu ý: nhập sai nhiều lần token sẽ bị khoá.",
        )),
        SCARD_W_CHV_BLOCKED => Some(SmksError::new(
            ErrorCode::PinBlocked,
            "Token đã bị khoá do nhập sai mã PIN quá số lần cho phép. Liên hệ nơi cấp chứng thư để mở khoá (PUK).",
        )),
        _ => None,
    }
}

/// Middleware có nhận mã PIN do ứng dụng đưa vào không (spike S6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinHandoff {
    /// Provider nhận thuộc tính PIN; lúc ký sẽ không hỏi lại (nếu middleware tôn trọng thuộc tính này).
    Accepted,
    /// Provider không nhận; middleware sẽ tự hiện hộp PIN như thường.
    NotSupported,
}

fn token_error(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignTokenInvalid, message)
}

impl StoreKey {
    /// Mở khoá của chứng thư có `thumbprint`, kiểm như `StoreSigningKey.Verify` của bản cũ (trừ ký thử, xem
    /// [`StoreKey::preflight`]).
    pub fn open(thumbprint: &str) -> Result<Self> {
        let context = find_context(thumbprint).ok_or_else(|| {
            SmksError::new(
                ErrorCode::CertificateNotFound,
                "Không tìm thấy chứng thư số trên máy. Kiểm tra USB token đã cắm và middleware (VGCA PKI Manager/bit4id xPKI) đã chạy chưa.",
            )
        })?;
        Self::from_context(context)
    }

    fn from_context(context: ffi::Context) -> Result<Self> {
        let cert = Cert::from_der(&context.der())?;
        let prov = context.key_provider().ok_or_else(|| {
            token_error("Chứng thư số không có khóa bí mật trên máy này nên không ký được.")
        })?;
        if !cert.is_valid_at(now_unix()) {
            return Err(token_error(expired_reason(&cert)));
        }
        if !cert.allows_signing() {
            return Err(token_error(REASON_NO_USAGE));
        }
        let algorithm = SigAlg::for_certificate(&cert)
            .ok_or_else(|| token_error("Chứng thư số không dùng thuật toán RSA hoặc ECDSA."))?;
        let key = context.acquire_key().map_err(|e| {
            token_error(format!(
                "Không dùng được khóa bí mật để ký (token chưa cắm, PIN bị hủy, hoặc middleware chưa sẵn sàng): {e}"
            ))
        })?;
        if algorithm == SigAlg::EcdsaSha256 && !key.is_cng() {
            return Err(token_error(
                "Khoá ECDSA trên CSP đời cũ (CAPI) không được hỗ trợ.",
            ));
        }
        let hardware = is_hardware_provider(&prov.provider);
        Ok(Self {
            cert,
            algorithm,
            provider: prov.provider,
            hardware,
            _context: context,
            key: Mutex::new(key),
        })
    }

    pub fn is_hardware(&self) -> bool {
        self.hardware
    }

    /// Đưa mã PIN cho middleware trước khi ký thử. PIN sai mà provider kiểm ngay thì trả lỗi
    /// [`ErrorCode::PinIncorrect`] / [`ErrorCode::PinBlocked`]; provider không nhận thì trả
    /// [`PinHandoff::NotSupported`] và để middleware tự hỏi. Không ghi PIN vào log hay thông báo.
    pub fn set_pin(&self, pin: &str) -> Result<PinHandoff> {
        let key = self
            .key
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match key.set_pin(pin) {
            Ok(()) => Ok(PinHandoff::Accepted),
            Err(e) => match pin_error(e.code) {
                Some(err) => Err(err),
                None => Ok(PinHandoff::NotSupported),
            },
        }
    }

    /// Ký thử 32 byte ngẫu nhiên rồi xác minh bằng khoá công khai (lúc này middleware hỏi PIN), như bản cũ.
    pub fn preflight(&self) -> Result<()> {
        let mut probe = [0u8; k::PREFLIGHT_TEST_DATA_BYTES];
        getrandom::fill(&mut probe)
            .map_err(|e| token_error(format!("Không sinh được dữ liệu ký thử: {e}")))?;
        let digest: [u8; 32] =
            sha2::Digest::finalize(<sha2::Sha256 as sha2::Digest>::new_with_prefix(probe)).into();
        let signature = self.sign_digest(&digest).map_err(|e| {
            if matches!(e.code, ErrorCode::PinIncorrect | ErrorCode::PinBlocked) {
                return e;
            }
            token_error(format!(
                "Không dùng được khóa bí mật để ký (token chưa cắm, PIN bị hủy, hoặc middleware chưa sẵn sàng): {}",
                e.message
            ))
        })?;
        crate::verify::verify_digest_signature(
            &self.cert,
            crate::verify::DigestAlg::Sha256,
            &digest,
            self.algorithm.signature_oid(),
            &signature,
        )
        .map_err(|_| {
            token_error("Khoá trên token không khớp chứng thư số (ký thử không xác minh được).")
        })
    }
}

impl SigningKey for StoreKey {
    fn certificate(&self) -> &Cert {
        &self.cert
    }

    fn algorithm(&self) -> SigAlg {
        self.algorithm
    }

    fn sign_digest(&self, digest: &[u8; 32]) -> Result<Vec<u8>> {
        let key = self
            .key
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let hwnd = WINDOW.load(Ordering::SeqCst);
        if hwnd != 0 {
            key.set_window(hwnd);
        }
        let rsa = self.algorithm == SigAlg::RsaPkcs1Sha256;
        let raw = key.sign(digest, rsa).map_err(|e| {
            pin_error(e.code).unwrap_or_else(|| {
                SmksError::new(
                    ErrorCode::SignFailed,
                    format!("Token không ký được: {}", e.message),
                )
            })
        })?;
        if rsa {
            Ok(raw)
        } else {
            ecdsa_raw_to_der(&raw).ok_or_else(|| {
                SmksError::new(
                    ErrorCode::SignFailed,
                    "Token trả về chữ ký ECDSA không hợp lệ.",
                )
            })
        }
    }

    fn provider(&self) -> String {
        self.provider.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_provider_markers() {
        assert!(is_hardware_provider("bit4id Universal Middleware Provider"));
        assert!(is_hardware_provider(
            "Microsoft Smart Card Key Storage Provider"
        ));
        assert!(!is_hardware_provider(
            "Microsoft Software Key Storage Provider"
        ));
        assert!(!is_hardware_provider(
            "Microsoft Enhanced RSA and AES Cryptographic Provider"
        ));
        assert!(!is_hardware_provider(""));
    }

    #[test]
    fn vietnam_time_format() {
        // 2026-11-19 03:00:00 UTC = 10:00 giờ Việt Nam.
        assert_eq!(format_vietnam_time(1_795_057_200), "19/11/2026 10:00:00");
    }

    #[test]
    fn listing_does_not_fail_without_certificates() {
        let scan = list_certificates(&|_| false);
        assert_eq!(scan.diagnostics.len(), 2, "{:?}", scan.diagnostics);
        for c in &scan.certificates {
            assert_eq!(c.can_sign, c.reason.is_none());
        }
    }

    fn sign_and_verify(key: &StoreKey) {
        key.preflight().unwrap();
        let digest = [0x5Au8; 32];
        let sig = key.sign_digest(&digest).unwrap();
        crate::verify::verify_digest_signature(
            key.certificate(),
            crate::verify::DigestAlg::Sha256,
            &digest,
            key.algorithm().signature_oid(),
            &sig,
        )
        .unwrap();
    }

    /// Tạo khoá tạm trong kho khoá của người dùng rồi xoá: `cargo test -p smks-crypto -- --ignored store_key`.
    #[test]
    #[ignore]
    fn store_key_signs_with_cng_rsa_ecdsa_and_capi() {
        for (label, temp) in [
            ("CNG RSA", ffi::testkeys::cng(false)),
            ("CNG ECDSA P-256", ffi::testkeys::cng(true)),
            ("CAPI RSA", ffi::testkeys::capi()),
        ] {
            let key = StoreKey::from_context(temp.context.duplicate()).unwrap();
            println!(
                "{label}: {} | provider {} | {:?}",
                key.certificate().common_name(),
                key.provider(),
                key.algorithm()
            );
            assert!(!key.is_hardware());
            // Provider phần mềm của Microsoft không có PIN: đưa PIN không làm hỏng khoá, ký vẫn được.
            let handoff = key.set_pin("123456").unwrap();
            println!("{label}: đưa PIN → {handoff:?}");
            sign_and_verify(&key);
            drop(key);
            drop(temp);
        }
    }
}
