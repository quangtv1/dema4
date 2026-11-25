//! Dữ liệu kiểm tra dài hạn (PAdES B-LT, TD-58): hỏi OCSP / tải CRL cho từng chứng thư trong chuỗi, kiểm chữ ký
//! của phản hồi, để nhúng vào file lúc ký (DSS) và để đọc lại khi kiểm tra không có mạng.
//!
//! CA Ban Cơ yếu có OCSP (`ocsp.ca.gov.vn`, `ocsp.ca.dcs.vn`) và CRL (`ca.gov.vn/pki/pub/crl/…`). Ưu tiên OCSP
//! (nhỏ, mới); không được thì CRL. Phản hồi chỉ được dùng khi chữ ký của nó đúng: OCSP do chính CA cấp ký, hoặc
//! do chứng thư OCSP (EKU `OCSPSigning`) mà CA cấp ký; CRL do CA cấp ký.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use der::asn1::{Null, ObjectIdentifier, OctetString};
use der::{Decode, Encode};
use sha1::Sha1;
use sha2::Digest;
use smks_core::{ErrorCode, Result, SmksError};
use x509_cert::crl::CertificateList;
use x509_cert::ext::pkix::name::{DistributionPointName, GeneralName};
use x509_cert::ext::pkix::{AuthorityInfoAccessSyntax, CrlDistributionPoints};
use x509_ocsp::{
    BasicOcspResponse, CertId, CertStatus, OcspRequest, OcspResponse, OcspResponseStatus, Request,
    ResponderId, TbsRequest,
};

use crate::cert::Cert;
use crate::verify::{DigestAlg, verify_digest_signature_with};

/// Thời hạn mỗi lần hỏi OCSP / tải CRL.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
/// CRL lớn nhất chịu tải (byte).
const CRL_MAX_BYTES: u64 = 20 * 1024 * 1024;
/// Giữ dữ liệu thu hồi đã lấy trong một lô ký bao lâu.
const CACHE_TTL: Duration = Duration::from_secs(20 * 60);

const OID_AD_OCSP: &str = "1.3.6.1.5.5.7.48.1";
const OID_OCSP_BASIC: &str = "1.3.6.1.5.5.7.48.1.1";
const OID_EKU_OCSP_SIGNING: &str = "1.3.6.1.5.5.7.3.9";
const OID_SHA1: &str = "1.3.14.3.2.26";

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::RevocationUnknown, message)
}

/// Trạng thái thu hồi đọc từ một phản hồi đã kiểm chữ ký.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevStatus {
    Good,
    /// Bị thu hồi, ngày thu hồi (giây Unix).
    Revoked(i64),
    /// OCSP trả "unknown".
    Unknown,
}

/// Kết quả đọc một phản hồi OCSP hoặc CRL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevInfo {
    pub status: RevStatus,
    /// Thời điểm phản hồi xác nhận trạng thái (`producedAt` của OCSP, `thisUpdate` của CRL).
    pub produced_at: i64,
    pub next_update: Option<i64>,
}

/// Dữ liệu thu hồi đã lấy cho một chứng thư.
#[derive(Debug, Clone)]
pub enum Fetched {
    Ocsp(Vec<u8>, RevInfo),
    Crl(Vec<u8>, RevInfo),
}

impl Fetched {
    pub fn info(&self) -> RevInfo {
        match self {
            Self::Ocsp(_, i) | Self::Crl(_, i) => *i,
        }
    }
}

fn unix(t: &x509_cert::time::Time) -> i64 {
    i64::try_from(t.to_unix_duration().as_secs()).unwrap_or(0)
}

fn gen_unix(t: &x509_ocsp::OcspGeneralizedTime) -> i64 {
    i64::try_from(t.0.to_unix_duration().as_secs()).unwrap_or(0)
}

/// Thuật toán băm của một `signatureAlgorithm` (RSA PKCS#1, RSA-PSS, ECDSA).
fn digest_for(oid: &str, params: Option<&der::Any>) -> Option<DigestAlg> {
    let alg = |s: &str| DigestAlg::from_oid(&ObjectIdentifier::new(s).ok()?);
    match oid {
        "1.2.840.113549.1.1.5" | "1.2.840.10045.4.1" => Some(DigestAlg::Sha1),
        "1.2.840.113549.1.1.11" | "1.2.840.10045.4.3.2" => Some(DigestAlg::Sha256),
        "1.2.840.113549.1.1.12" | "1.2.840.10045.4.3.3" => Some(DigestAlg::Sha384),
        "1.2.840.113549.1.1.13" | "1.2.840.10045.4.3.4" => Some(DigestAlg::Sha512),
        crate::verify::OID_RSA_PSS => crate::verify::pss_hash_of(params),
        other => alg(other),
    }
}

/// Kiểm `signature` của `signer` trên `tbs` (DER) theo `signatureAlgorithm`.
fn verify_signed(
    signer: &Cert,
    tbs: &[u8],
    alg: &spki::AlgorithmIdentifierOwned,
    signature: &[u8],
) -> bool {
    let oid = alg.oid.to_string();
    let Some(digest_alg) = digest_for(&oid, alg.parameters.as_ref()) else {
        return false;
    };
    let digest = digest_alg.digest(&[tbs]);
    verify_digest_signature_with(
        signer,
        digest_alg,
        &digest,
        &oid,
        alg.parameters.as_ref(),
        signature,
    )
    .is_ok()
}

/// Chứng thư `cert` do `issuer` ký thật (không chỉ trùng tên).
pub fn issued_and_signed_by(cert: &Cert, issuer: &Cert) -> bool {
    let parsed = cert.parsed();
    let Ok(tbs) = parsed.tbs_certificate().to_der() else {
        return false;
    };
    cert.issued_by(issuer)
        && verify_signed(
            issuer,
            &tbs,
            parsed.signature_algorithm(),
            parsed.signature().raw_bytes(),
        )
}

fn cert_id(cert: &Cert, issuer: &Cert) -> Result<CertId> {
    let name = issuer
        .parsed()
        .tbs_certificate()
        .subject()
        .to_der()
        .map_err(|e| fail(e.to_string()))?;
    Ok(CertId {
        hash_algorithm: spki::AlgorithmIdentifierOwned {
            oid: ObjectIdentifier::new_unwrap(OID_SHA1),
            parameters: Some(Null.into()),
        },
        issuer_name_hash: OctetString::new(Sha1::digest(&name).to_vec())
            .map_err(|e| fail(e.to_string()))?,
        issuer_key_hash: OctetString::new(Sha1::digest(issuer.public_key_bytes()).to_vec())
            .map_err(|e| fail(e.to_string()))?,
        serial_number: cert.parsed().tbs_certificate().serial_number().clone(),
    })
}

/// `CertID` trong phản hồi trỏ đúng `cert` của `issuer` (băm SHA-1 hoặc SHA-2).
fn matches(id: &CertId, cert: &Cert, issuer: &Cert) -> bool {
    let Some(alg) = DigestAlg::from_oid(&id.hash_algorithm.oid) else {
        return false;
    };
    let Ok(name) = issuer.parsed().tbs_certificate().subject().to_der() else {
        return false;
    };
    id.serial_number == *cert.parsed().tbs_certificate().serial_number()
        && id.issuer_name_hash.as_bytes() == alg.digest(&[&name])
        && id.issuer_key_hash.as_bytes() == alg.digest(&[issuer.public_key_bytes()])
}

/// DER của yêu cầu OCSP cho `cert` (không nonce, không ký).
pub fn ocsp_request(cert: &Cert, issuer: &Cert) -> Result<Vec<u8>> {
    let request = OcspRequest {
        tbs_request: TbsRequest {
            request_list: vec![Request {
                req_cert: cert_id(cert, issuer)?,
                single_request_extensions: None,
            }],
            ..Default::default()
        },
        optional_signature: None,
    };
    request.to_der().map_err(|e| fail(e.to_string()))
}

/// Đọc và kiểm một phản hồi OCSP cho `cert` (do `issuer` cấp).
pub fn check_ocsp(response: &[u8], cert: &Cert, issuer: &Cert) -> Result<RevInfo> {
    let resp = OcspResponse::from_der(response)
        .map_err(|e| fail(format!("Phản hồi OCSP không đọc được: {e}")))?;
    if resp.response_status != OcspResponseStatus::Successful {
        return Err(fail(format!(
            "Máy chủ OCSP từ chối ({:?}).",
            resp.response_status
        )));
    }
    let bytes = resp
        .response_bytes
        .ok_or_else(|| fail("Phản hồi OCSP không có nội dung."))?;
    if bytes.response_type.to_string() != OID_OCSP_BASIC {
        return Err(fail("Phản hồi OCSP không phải dạng cơ bản."));
    }
    let basic = BasicOcspResponse::from_der(bytes.response.as_bytes())
        .map_err(|e| fail(format!("Phản hồi OCSP không đọc được: {e}")))?;
    let tbs = basic
        .tbs_response_data
        .to_der()
        .map_err(|e| fail(e.to_string()))?;
    // Người ký phản hồi: CA cấp, hoặc chứng thư OCSP do CA cấp.
    let mut signers = vec![issuer.clone()];
    for c in basic.certs.iter().flatten() {
        if let Ok(c) = Cert::from_parsed(c.clone())
            && c.has_eku(OID_EKU_OCSP_SIGNING)
            && issued_and_signed_by(&c, issuer)
        {
            signers.push(c);
        }
    }
    let responder_ok = |c: &Cert| match &basic.tbs_response_data.responder_id {
        ResponderId::ByName(n) => c.parsed().tbs_certificate().subject() == n,
        ResponderId::ByKey(k) => k.as_bytes() == Sha1::digest(c.public_key_bytes()).as_slice(),
    };
    let signed = signers.iter().any(|s| {
        responder_ok(s)
            && verify_signed(
                s,
                &tbs,
                &basic.signature_algorithm,
                basic.signature.raw_bytes(),
            )
    });
    if !signed {
        return Err(fail(
            "Chữ ký của phản hồi OCSP không hợp lệ hoặc không do CA cấp chứng thư ký.",
        ));
    }
    let single = basic
        .tbs_response_data
        .responses
        .iter()
        .find(|r| matches(&r.cert_id, cert, issuer))
        .ok_or_else(|| fail("Phản hồi OCSP không nói về chứng thư này."))?;
    let status = match &single.cert_status {
        CertStatus::Good(_) => RevStatus::Good,
        CertStatus::Revoked(info) => RevStatus::Revoked(gen_unix(&info.revocation_time)),
        CertStatus::Unknown(_) => RevStatus::Unknown,
    };
    Ok(RevInfo {
        status,
        produced_at: gen_unix(&basic.tbs_response_data.produced_at),
        next_update: single.next_update.as_ref().map(gen_unix),
    })
}

/// Đọc và kiểm một CRL cho `cert` (do `issuer` cấp).
pub fn check_crl(crl: &[u8], cert: &Cert, issuer: &Cert) -> Result<RevInfo> {
    let list = CertificateList::<x509_cert::certificate::Rfc5280>::from_der(crl)
        .map_err(|e| fail(format!("CRL không đọc được: {e}")))?;
    let tbs = &list.tbs_cert_list;
    if tbs.issuer != *issuer.parsed().tbs_certificate().subject() {
        return Err(fail("CRL không do CA cấp chứng thư phát hành."));
    }
    let tbs_der = tbs.to_der().map_err(|e| fail(e.to_string()))?;
    if !verify_signed(
        issuer,
        &tbs_der,
        &list.signature_algorithm,
        list.signature.raw_bytes(),
    ) {
        return Err(fail("Chữ ký của CRL không hợp lệ."));
    }
    let serial = cert.parsed().tbs_certificate().serial_number();
    let status = tbs
        .revoked_certificates
        .iter()
        .flatten()
        .find(|r| r.serial_number == *serial)
        .map_or(RevStatus::Good, |r| {
            RevStatus::Revoked(unix(&r.revocation_date))
        });
    Ok(RevInfo {
        status,
        produced_at: unix(&tbs.this_update),
        next_update: tbs.next_update.as_ref().map(unix),
    })
}

fn uri(name: &GeneralName) -> Option<String> {
    match name {
        GeneralName::UniformResourceIdentifier(u) => {
            let s = u.as_str().to_owned();
            (s.starts_with("http://") || s.starts_with("https://")).then_some(s)
        }
        _ => None,
    }
}

/// Địa chỉ OCSP trong Authority Information Access.
pub fn ocsp_urls(cert: &Cert) -> Vec<String> {
    let Ok(Some((_, aia))) = cert
        .parsed()
        .tbs_certificate()
        .get_extension::<AuthorityInfoAccessSyntax>()
    else {
        return Vec::new();
    };
    aia.0
        .iter()
        .filter(|a| a.access_method.to_string() == OID_AD_OCSP)
        .filter_map(|a| uri(&a.access_location))
        .collect()
}

/// Địa chỉ CRL trong CRL Distribution Points.
pub fn crl_urls(cert: &Cert) -> Vec<String> {
    let Ok(Some((_, dps))) = cert
        .parsed()
        .tbs_certificate()
        .get_extension::<CrlDistributionPoints>()
    else {
        return Vec::new();
    };
    dps.0
        .iter()
        .filter_map(|dp| match &dp.distribution_point {
            Some(DistributionPointName::FullName(names)) => {
                Some(names.iter().filter_map(uri).collect::<Vec<_>>())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

fn http(url: &str, body: Option<&[u8]>, limit: u64) -> std::result::Result<Vec<u8>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = match body {
        Some(b) => agent
            .post(url)
            .header("Content-Type", "application/ocsp-request")
            .send(b),
        None => agent.get(url).call(),
    }
    .map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("HTTP {status}"));
    }
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// Lấy dữ liệu thu hồi cho `cert` qua mạng: OCSP trước, không được thì CRL. Phản hồi đã kiểm chữ ký.
pub fn fetch(cert: &Cert, issuer: &Cert) -> Result<Fetched> {
    let mut errors = Vec::new();
    if !ocsp_urls(cert).is_empty() {
        let request = ocsp_request(cert, issuer)?;
        for url in ocsp_urls(cert) {
            match http(&url, Some(&request), 1024 * 1024) {
                Ok(bytes) => match check_ocsp(&bytes, cert, issuer) {
                    Ok(info) if info.status != RevStatus::Unknown => {
                        return Ok(Fetched::Ocsp(bytes, info));
                    }
                    Ok(_) => errors.push(format!("{url}: OCSP trả \"không biết\"")),
                    Err(e) => errors.push(format!("{url}: {}", e.message)),
                },
                Err(e) => errors.push(format!("{url}: {e}")),
            }
        }
    }
    for url in crl_urls(cert) {
        match http(&url, None, CRL_MAX_BYTES) {
            Ok(bytes) => match check_crl(&bytes, cert, issuer) {
                Ok(info) => return Ok(Fetched::Crl(bytes, info)),
                Err(e) => errors.push(format!("{url}: {}", e.message)),
            },
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    if errors.is_empty() {
        return Err(fail(format!(
            "Chứng thư \"{}\" không ghi địa chỉ OCSP / CRL.",
            cert.common_name()
        )));
    }
    Err(fail(errors.join("; ")))
}

/// Một mục đệm: lúc lấy và kết quả (dữ liệu, hoặc lý do không lấy được).
type CacheEntry = (Instant, std::result::Result<Fetched, String>);

/// Bộ nhớ đệm dữ liệu thu hồi trong một lô ký (cùng người ký, cùng TSA: chỉ hỏi một lần).
#[derive(Debug, Default)]
pub struct LtvCache {
    items: Mutex<HashMap<String, CacheEntry>>,
}

impl LtvCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dữ liệu thu hồi cho `cert`, lấy lại khi quá hạn.
    pub fn get(&self, cert: &Cert, issuer: &Cert) -> std::result::Result<Fetched, String> {
        let key = format!("{}|{}", issuer.thumbprint(), cert.serial_hex());
        if let Ok(items) = self.items.lock()
            && let Some((at, value)) = items.get(&key)
            && at.elapsed() < CACHE_TTL
        {
            return value.clone();
        }
        let value = fetch(cert, issuer).map_err(|e| e.message);
        if let Ok(mut items) = self.items.lock() {
            items.insert(key, (Instant::now(), value.clone()));
        }
        value
    }
}

/// Dữ liệu để nhúng vào DSS.
#[derive(Debug, Clone, Default)]
pub struct LtvData {
    pub certs: Vec<Vec<u8>>,
    pub ocsps: Vec<Vec<u8>>,
    pub crls: Vec<Vec<u8>>,
    /// Chứng thư không lấy được dữ liệu thu hồi, kèm lý do.
    pub missing: Vec<String>,
}

/// Gom dữ liệu kiểm tra dài hạn cho các chuỗi (mỗi chuỗi từ chứng thư lá tới gốc). Gốc không cần thu hồi.
pub fn collect(chains: &[Vec<Cert>], cache: &LtvCache) -> LtvData {
    let mut out = LtvData::default();
    for chain in chains {
        for c in chain {
            if !out.certs.iter().any(|d| d == c.der()) {
                out.certs.push(c.der().to_vec());
            }
        }
        for pair in chain.windows(2) {
            let (cert, issuer) = (&pair[0], &pair[1]);
            match cache.get(cert, issuer) {
                Ok(Fetched::Ocsp(b, _)) => {
                    if !out.ocsps.contains(&b) {
                        out.ocsps.push(b);
                    }
                }
                Ok(Fetched::Crl(b, _)) => {
                    if !out.crls.contains(&b) {
                        out.crls.push(b);
                    }
                }
                Err(e) => out.missing.push(format!("{}: {e}", cert.common_name())),
            }
        }
    }
    out
}

/// Tìm trong dữ liệu nhúng (OCSP, CRL) phản hồi hợp lệ cho `cert`; ưu tiên phản hồi mới nhất.
pub fn find_embedded(
    ocsps: &[Vec<u8>],
    crls: &[Vec<u8>],
    cert: &Cert,
    issuer: &Cert,
) -> Option<RevInfo> {
    ocsps
        .iter()
        .filter_map(|o| check_ocsp(o, cert, issuer).ok())
        .chain(crls.iter().filter_map(|c| check_crl(c, cert, issuer).ok()))
        .filter(|i| i.status != RevStatus::Unknown)
        .max_by_key(|i| i.produced_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TSA_02: &[u8] = include_bytes!("../../../fixtures/certs/tsa-ban-co-yeu-02.cer");

    fn root_g2() -> Cert {
        let ca = crate::trust::PINNED_CAS
            .iter()
            .find(|c| c.file_name() == "rootcag2.crt")
            .unwrap();
        Cert::from_der(&ca.verified_der().unwrap()).unwrap()
    }

    #[test]
    fn urls_and_request_for_real_tsa_certificate() {
        let tsa = Cert::from_der(TSA_02).unwrap();
        let root = root_g2();
        assert!(issued_and_signed_by(&tsa, &root));
        assert_eq!(ocsp_urls(&tsa), vec!["http://ocsp.ca.gov.vn".to_owned()]);
        assert_eq!(
            crl_urls(&tsa),
            vec!["http://ca.gov.vn/pki/pub/crl/rootcag2.crl".to_owned()]
        );
        let req = ocsp_request(&tsa, &root).unwrap();
        let parsed = OcspRequest::from_der(&req).unwrap();
        assert!(matches(
            &parsed.tbs_request.request_list[0].req_cert,
            &tsa,
            &root
        ));
        // Phản hồi rác thì báo lỗi, không coi là "còn hiệu lực".
        assert!(check_ocsp(b"\x30\x03\x0a\x01\x00", &tsa, &root).is_err());
        assert!(check_crl(&[0x30, 0x00], &tsa, &root).is_err());
    }

    #[test]
    fn crl_of_test_ca_good_revoked_and_tampered() {
        let pki = crate::testpki::TestPki::new();
        let good = pki.crl(None);
        let info = check_crl(&good, &pki.signer, &pki.ca).unwrap();
        assert_eq!(info.status, RevStatus::Good);
        assert!(info.next_update.unwrap() > info.produced_at);
        let revoked = pki.crl(Some(1_790_000_000));
        assert_eq!(
            check_crl(&revoked, &pki.signer, &pki.ca).unwrap().status,
            RevStatus::Revoked(1_790_000_000)
        );
        // CA khác (gốc Ban Cơ yếu) không phải người phát hành CRL này.
        assert!(check_crl(&good, &pki.signer, &root_g2()).is_err());
        // Sửa một byte trong phần được ký.
        let mut bad = good.clone();
        let n = bad.len();
        bad[n / 3] ^= 1;
        assert!(check_crl(&bad, &pki.signer, &pki.ca).is_err());
        // Tìm trong dữ liệu nhúng: lấy phản hồi mới nhất hợp lệ.
        assert_eq!(
            find_embedded(&[], &[bad, good], &pki.signer, &pki.ca).map(|i| i.status),
            Some(RevStatus::Good)
        );
        assert!(find_embedded(&[], &[], &pki.signer, &pki.ca).is_none());
    }

    /// Hỏi OCSP và CRL thật của Ban Cơ yếu: `cargo test -p smks-crypto -- --ignored real_ocsp`.
    #[test]
    #[ignore]
    fn real_ocsp_and_crl_of_ban_co_yeu() {
        let tsa = Cert::from_der(TSA_02).unwrap();
        let root = root_g2();
        let f = fetch(&tsa, &root).unwrap();
        println!("{:?}", f.info());
        assert!(matches!(f, Fetched::Ocsp(..)));
        assert_eq!(f.info().status, RevStatus::Good);
        let crl = http(&crl_urls(&tsa)[0], None, CRL_MAX_BYTES).unwrap();
        let info = check_crl(&crl, &tsa, &root).unwrap();
        assert_eq!(info.status, RevStatus::Good);
        // Phản hồi sửa 1 byte (trong phần được ký) thì không còn hợp lệ.
        let Fetched::Ocsp(mut bytes, _) = f else {
            unreachable!()
        };
        let n = bytes.len();
        bytes[n / 2] ^= 1;
        assert!(check_ocsp(&bytes, &tsa, &root).is_err());
    }
}
