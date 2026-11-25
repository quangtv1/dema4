//! Dựng và kiểm chuỗi chứng thư tới Root G1 / Root G2 đã ghim (kế hoạch 6.5).
//!
//! Dùng CryptoAPI của Windows như bản cũ (.NET cũng vậy), với **kho gốc độc quyền** chỉ chứa hai gốc đã ghim:
//! gốc khác người dùng cài vào Windows không làm chứng thư lạ thành "tin cậy". CA trung gian đã ghim và các
//! chứng thư đi kèm chữ ký được đưa vào làm kho phụ.
//!
//! Thu hồi (`CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT`) chia ba: còn hiệu lực, đã thu hồi, không kiểm tra
//! được (mất mạng). Không kiểm được thì vẫn ký nhưng báo "chưa kiểm tra được thu hồi".

use serde::Serialize;
use smks_core::{ErrorCode, Result, SmksError};

use crate::cert::Cert;
use crate::trust::{CaRole, PINNED_CAS};

#[allow(unsafe_code)]
mod ffi;

const CERT_TRUST_IS_NOT_TIME_VALID: u32 = 0x1;
const CERT_TRUST_IS_NOT_TIME_NESTED: u32 = 0x2;
const CERT_TRUST_IS_REVOKED: u32 = 0x4;
const CERT_TRUST_IS_NOT_SIGNATURE_VALID: u32 = 0x8;
const CERT_TRUST_IS_NOT_VALID_FOR_USAGE: u32 = 0x10;
const CERT_TRUST_IS_UNTRUSTED_ROOT: u32 = 0x20;
const CERT_TRUST_REVOCATION_STATUS_UNKNOWN: u32 = 0x40;
const CERT_TRUST_IS_CYCLIC: u32 = 0x80;
const CERT_TRUST_IS_PARTIAL_CHAIN: u32 = 0x10000;
const CERT_TRUST_IS_OFFLINE_REVOCATION: u32 = 0x0100_0000;

/// Thời hạn tải CRL/OCSP khi kiểm thu hồi.
const REVOCATION_TIMEOUT_MS: u32 = 15_000;

/// Tình trạng thu hồi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Revocation {
    NotChecked,
    Good,
    Revoked,
    /// Không tải được CRL/OCSP (mất mạng, máy chủ CA không trả lời).
    Unknown,
}

/// Kết quả kiểm chuỗi.
#[derive(Debug, Clone)]
pub struct ChainReport {
    /// Chuỗi về một gốc đã ghim và mọi chữ ký trong chuỗi đúng.
    pub trusted: bool,
    /// "Root G1" hoặc "Root G2".
    pub root: Option<&'static str>,
    /// Mọi chứng thư trong chuỗi còn hiệu lực ở thời điểm kiểm.
    pub time_valid: bool,
    pub revocation: Revocation,
    /// Ngày thu hồi (giây Unix) khi `revocation` là `Revoked` và Windows cho biết ngày.
    pub revoked_at: Option<i64>,
    /// Từ chứng thư được kiểm tới gốc.
    pub elements: Vec<Cert>,
    /// Mô tả tiếng Việt các vấn đề (rỗng nếu không có).
    pub problems: Vec<String>,
}

fn pinned(role_filter: impl Fn(CaRole) -> bool) -> Vec<Vec<u8>> {
    PINNED_CAS
        .iter()
        .filter(|ca| role_filter(ca.role()))
        .filter_map(|ca| ca.verified_der().ok())
        .collect()
}

/// Kiểm chuỗi của `cert` tại thời điểm `at_unix` (None = bây giờ). `extra`: chứng thư kèm theo (ví dụ trong CMS).
pub fn validate(
    cert: &Cert,
    extra: &[Cert],
    at_unix: Option<i64>,
    check_revocation: bool,
) -> Result<ChainReport> {
    let roots = pinned(|r| matches!(r, CaRole::RootG1 | CaRole::RootG2));
    let mut intermediates = pinned(|r| matches!(r, CaRole::SubCaG1 | CaRole::IntermediateG2));
    intermediates.extend(extra.iter().map(|c| c.der().to_vec()));
    let root_refs: Vec<&[u8]> = roots.iter().map(Vec::as_slice).collect();
    let extra_refs: Vec<&[u8]> = intermediates.iter().map(Vec::as_slice).collect();
    let raw = ffi::build(
        cert.der(),
        &root_refs,
        &extra_refs,
        at_unix,
        check_revocation,
        REVOCATION_TIMEOUT_MS,
    )
    .map_err(|m| SmksError::new(ErrorCode::SignatureInvalid, m))?;
    let mut report = interpret(&raw.elements, raw.error_status, check_revocation);
    if report.revocation == Revocation::Revoked {
        report.revoked_at = raw.revoked_at;
    }
    Ok(report)
}

fn interpret(elements_der: &[Vec<u8>], status: u32, check_revocation: bool) -> ChainReport {
    let elements: Vec<Cert> = elements_der
        .iter()
        .filter_map(|d| Cert::from_der(d).ok())
        .collect();
    let root = elements.last().and_then(|last| {
        PINNED_CAS
            .iter()
            .filter(|ca| matches!(ca.role(), CaRole::RootG1 | CaRole::RootG2))
            .find(|ca| ca.verified_der().is_ok_and(|der| der == last.der()))
            .map(|ca| ca.role().label())
    });
    let mut problems = Vec::new();
    if status & (CERT_TRUST_IS_UNTRUSTED_ROOT | CERT_TRUST_IS_PARTIAL_CHAIN) != 0 || root.is_none()
    {
        problems.push(
            "Chứng thư không thuộc CA Ban Cơ yếu (chuỗi không về Root G1 hoặc Root G2 đã ghim)."
                .to_owned(),
        );
    }
    if status & CERT_TRUST_IS_NOT_SIGNATURE_VALID != 0 {
        problems.push("Chữ ký trên một chứng thư trong chuỗi không hợp lệ.".to_owned());
    }
    if status & CERT_TRUST_IS_CYCLIC != 0 {
        problems.push("Chuỗi chứng thư bị vòng lặp.".to_owned());
    }
    let ignored = CERT_TRUST_IS_NOT_TIME_VALID
        | CERT_TRUST_IS_NOT_TIME_NESTED
        | CERT_TRUST_IS_REVOKED
        | CERT_TRUST_REVOCATION_STATUS_UNKNOWN
        | CERT_TRUST_IS_OFFLINE_REVOCATION
        | CERT_TRUST_IS_NOT_VALID_FOR_USAGE;
    let other = status
        & !ignored
        & !(CERT_TRUST_IS_UNTRUSTED_ROOT
            | CERT_TRUST_IS_PARTIAL_CHAIN
            | CERT_TRUST_IS_NOT_SIGNATURE_VALID
            | CERT_TRUST_IS_CYCLIC);
    if other != 0 {
        problems.push(format!(
            "Chuỗi chứng thư có lỗi (mã trạng thái 0x{other:08X})."
        ));
    }
    let time_valid = status & CERT_TRUST_IS_NOT_TIME_VALID == 0;
    if !time_valid {
        problems.push("Có chứng thư trong chuỗi hết hiệu lực ở thời điểm kiểm.".to_owned());
    }
    let revocation = if !check_revocation {
        Revocation::NotChecked
    } else if status & CERT_TRUST_IS_REVOKED != 0 {
        problems.push("Chứng thư số đã bị thu hồi.".to_owned());
        Revocation::Revoked
    } else if status & (CERT_TRUST_REVOCATION_STATUS_UNKNOWN | CERT_TRUST_IS_OFFLINE_REVOCATION)
        != 0
    {
        Revocation::Unknown
    } else {
        Revocation::Good
    };
    let trusted = root.is_some()
        && status
            & (CERT_TRUST_IS_UNTRUSTED_ROOT
                | CERT_TRUST_IS_PARTIAL_CHAIN
                | CERT_TRUST_IS_NOT_SIGNATURE_VALID
                | CERT_TRUST_IS_CYCLIC)
            == 0
        && other == 0;
    ChainReport {
        trusted,
        root,
        time_valid,
        revocation,
        revoked_at: None,
        elements,
        problems,
    }
}

/// Chứng thư người ký có chuỗi về CA Ban Cơ yếu ở thời điểm hiện tại (không kiểm thu hồi), như
/// `SignatureVerifier.IsTrustedSignerCert` của bản cũ.
pub fn is_trusted_signer(cert: &Cert) -> bool {
    validate(cert, &[], None, false).is_ok_and(|r| r.trusted && r.time_valid)
}

/// CA trung gian (không gồm gốc) để nhúng vào CMS cùng chứng thư người ký.
pub fn intermediates_for(cert: &Cert) -> Vec<Cert> {
    validate(cert, &[], None, false)
        .map(|r| {
            let n = r.elements.len();
            r.elements
                .into_iter()
                .skip(1)
                .take(n.saturating_sub(2))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpki::TestPki;

    /// Chứng thư "Máy chủ cấp dấu thời gian 02" lấy từ token thật (chứng thư công khai, không có khoá).
    const TSA_02: &[u8] = include_bytes!("../../../fixtures/certs/tsa-ban-co-yeu-02.cer");

    #[test]
    fn real_tsa_certificate_chains_to_pinned_root_g2() {
        let tsa = Cert::from_der(TSA_02).unwrap();
        // 01/01/2026: trong thời hạn của chứng thư TSA (03/06/2024 – 01/06/2034).
        let report = validate(&tsa, &[], Some(1_767_225_600), false).unwrap();
        assert!(report.trusted, "{:?}", report.problems);
        assert_eq!(report.root, Some("Root G2"));
        assert!(report.time_valid);
        assert_eq!(report.revocation, Revocation::NotChecked);
        assert_eq!(report.elements.len(), 2);
    }

    #[test]
    fn expired_time_is_reported_but_chain_still_trusted() {
        let tsa = Cert::from_der(TSA_02).unwrap();
        // Năm 2040: chứng thư TSA đã hết hạn.
        let report = validate(&tsa, &[], Some(2_208_988_800), false).unwrap();
        assert!(report.trusted);
        assert!(!report.time_valid);
    }

    #[test]
    fn test_ca_is_not_trusted_even_when_supplied() {
        let pki = TestPki::new();
        let report = validate(&pki.signer, std::slice::from_ref(&pki.ca), None, false).unwrap();
        assert!(!report.trusted);
        assert!(report.root.is_none());
        assert!(!report.problems.is_empty());
        assert!(!is_trusted_signer(&pki.signer));
    }

    /// Kiểm thu hồi qua mạng với CA thật: `cargo test -p smks-crypto -- --ignored revocation`.
    #[test]
    #[ignore]
    fn revocation_check_against_real_ca() {
        let tsa = Cert::from_der(TSA_02).unwrap();
        let report = validate(&tsa, &[], None, true).unwrap();
        println!(
            "thu hồi: {:?}; vấn đề: {:?}",
            report.revocation, report.problems
        );
        assert!(report.trusted);
        assert_ne!(report.revocation, Revocation::Revoked);
    }
}
