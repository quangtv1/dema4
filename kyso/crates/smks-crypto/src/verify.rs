//! Xác minh chữ ký mức mật mã (thuần Rust): chữ ký trên một giá trị băm, một `SignerInfo` của CMS, và token
//! dấu thời gian RFC 3161. Kiểm chuỗi chứng thư và thu hồi nằm ở module `chain` (dùng CryptoAPI của Windows).

use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::asn1::{ObjectIdentifier, OctetString};
use der::{Decode, Encode};
use sha2::Digest;
use smks_core::{ErrorCode, Result, SmksError};
use x509_tsp::TstInfo;

use crate::cert::{Cert, KeyKind};

pub const OID_DATA: &str = "1.2.840.113549.1.7.1";
pub const OID_SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
pub const OID_CONTENT_TYPE: &str = "1.2.840.113549.1.9.3";
pub const OID_MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
pub const OID_TST_INFO: &str = "1.2.840.113549.1.9.16.1.4";
pub const OID_RSA_PSS: &str = "1.2.840.113549.1.1.10";
pub const OID_SIGNING_CERTIFICATE: &str = "1.2.840.113549.1.9.16.2.12";
pub const OID_SIGNING_CERTIFICATE_V2: &str = "1.2.840.113549.1.9.16.2.47";

fn fail(code: ErrorCode, message: impl Into<String>) -> SmksError {
    SmksError::new(code, message)
}

/// Thuật toán băm gặp trong chữ ký PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestAlg {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl DigestAlg {
    pub fn from_oid(oid: &ObjectIdentifier) -> Option<Self> {
        Some(match oid.to_string().as_str() {
            "1.3.14.3.2.26" => Self::Sha1,
            "2.16.840.1.101.3.4.2.1" => Self::Sha256,
            "2.16.840.1.101.3.4.2.2" => Self::Sha384,
            "2.16.840.1.101.3.4.2.3" => Self::Sha512,
            _ => return None,
        })
    }

    /// SHA-1 đã yếu (va chạm được), chỉ nên chấp nhận kèm cảnh báo.
    pub fn is_weak(self) -> bool {
        self == Self::Sha1
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }

    pub fn digest(self, parts: &[&[u8]]) -> Vec<u8> {
        fn run<D: Digest>(parts: &[&[u8]]) -> Vec<u8> {
            let mut h = D::new();
            for p in parts {
                h.update(p);
            }
            h.finalize().to_vec()
        }
        match self {
            Self::Sha1 => run::<sha1::Sha1>(parts),
            Self::Sha256 => run::<sha2::Sha256>(parts),
            Self::Sha384 => run::<sha2::Sha384>(parts),
            Self::Sha512 => run::<sha2::Sha512>(parts),
        }
    }
}

/// Kiểm `signature` trên giá trị băm `digest` (băm bằng `digest_alg`) với khoá công khai của `cert`.
/// `signature_oid` là `signatureAlgorithm` của SignerInfo: `rsaEncryption`, `shaXWithRSAEncryption`, hoặc
/// `ecdsa-with-SHAx`. RSA-PSS cần tham số: dùng [`verify_digest_signature_with`].
pub fn verify_digest_signature(
    cert: &Cert,
    digest_alg: DigestAlg,
    digest: &[u8],
    signature_oid: &str,
    signature: &[u8],
) -> Result<()> {
    verify_digest_signature_with(cert, digest_alg, digest, signature_oid, None, signature)
}

/// Như [`verify_digest_signature`], kèm `parameters` của `signatureAlgorithm` (cần cho RSA-PSS).
/// Thuật toán chưa hỗ trợ trả mã [`ErrorCode::SignatureUnsupported`] (không kết luận chữ ký sai).
pub fn verify_digest_signature_with(
    cert: &Cert,
    digest_alg: DigestAlg,
    digest: &[u8],
    signature_oid: &str,
    parameters: Option<&der::Any>,
    signature: &[u8],
) -> Result<()> {
    let bad = || {
        fail(
            ErrorCode::SignatureInvalid,
            "Chữ ký số không khớp với nội dung và chứng thư.",
        )
    };
    match cert.key_kind() {
        KeyKind::Rsa => {
            if !signature_oid.starts_with("1.2.840.113549.1.1.") {
                return Err(fail(
                    ErrorCode::SignatureUnsupported,
                    format!("Thuật toán chữ ký {signature_oid} chưa được hỗ trợ với khoá RSA."),
                ));
            }
            use rsa::Pkcs1v15Sign;
            use rsa::pkcs1::DecodeRsaPublicKey;
            let key = rsa::PublicKey::from_pkcs1_der(cert.public_key_bytes()).map_err(|e| {
                fail(
                    ErrorCode::SignatureInvalid,
                    format!("Khoá công khai RSA không đọc được: {e}"),
                )
            })?;
            if signature_oid == OID_RSA_PSS {
                let pss_hash = pss_hash(parameters).ok_or_else(|| {
                    fail(
                        ErrorCode::SignatureUnsupported,
                        "Tham số RSA-PSS dùng thuật toán băm chưa được hỗ trợ.",
                    )
                })?;
                if pss_hash != digest_alg {
                    return Err(fail(
                        ErrorCode::SignatureUnsupported,
                        "RSA-PSS dùng thuật toán băm khác với băm của chữ ký: chưa được hỗ trợ.",
                    ));
                }
                fn pss<D: sha2::Digest + sha2::digest::FixedOutputReset>() -> rsa::pss::Pss<D> {
                    // Độ dài salt tự dò khi kiểm (không bắt buộc đúng bằng tham số).
                    rsa::pss::Pss {
                        blinded: false,
                        digest: D::new(),
                        salt_len: None,
                    }
                }
                let ok = match digest_alg {
                    DigestAlg::Sha1 => key.verify(pss::<sha1::Sha1>(), digest, signature),
                    DigestAlg::Sha256 => key.verify(pss::<sha2::Sha256>(), digest, signature),
                    DigestAlg::Sha384 => key.verify(pss::<sha2::Sha384>(), digest, signature),
                    DigestAlg::Sha512 => key.verify(pss::<sha2::Sha512>(), digest, signature),
                };
                return ok.map_err(|_| bad());
            }
            let scheme = match digest_alg {
                DigestAlg::Sha1 => Pkcs1v15Sign::new::<sha1::Sha1>(),
                DigestAlg::Sha256 => Pkcs1v15Sign::new::<sha2::Sha256>(),
                DigestAlg::Sha384 => Pkcs1v15Sign::new::<sha2::Sha384>(),
                DigestAlg::Sha512 => Pkcs1v15Sign::new::<sha2::Sha512>(),
            };
            key.verify(scheme, digest, signature).map_err(|_| bad())
        }
        KeyKind::EcP256 => {
            use p256::ecdsa::signature::hazmat::PrehashVerifier;
            let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(cert.public_key_bytes())
                .map_err(|_| bad())?;
            let sig = p256::ecdsa::Signature::from_der(signature).map_err(|_| bad())?;
            key.verify_prehash(digest, &sig).map_err(|_| bad())
        }
        KeyKind::EcP384 => {
            use p384::ecdsa::signature::hazmat::PrehashVerifier;
            let key = p384::ecdsa::VerifyingKey::from_sec1_bytes(cert.public_key_bytes())
                .map_err(|_| bad())?;
            let sig = p384::ecdsa::Signature::from_der(signature).map_err(|_| bad())?;
            key.verify_prehash(digest, &sig).map_err(|_| bad())
        }
        KeyKind::Other => Err(fail(
            ErrorCode::SignatureUnsupported,
            "Chứng thư dùng thuật toán khoá chưa được hỗ trợ.",
        )),
    }
}

/// Đọc một phần tử DER: (tag, nội dung, phần còn lại). Chỉ dạng độ dài xác định.
pub(crate) fn tlv(bytes: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *bytes.first()?;
    let first = *bytes.get(1)?;
    let (len, head) = if first < 0x80 {
        (usize::from(first), 2)
    } else {
        let n = usize::from(first & 0x7F);
        if n == 0 || n > 4 {
            return None;
        }
        let mut len = 0usize;
        for i in 0..n {
            len = (len << 8) | usize::from(*bytes.get(2 + i)?);
        }
        (len, 2 + n)
    };
    let end = head.checked_add(len)?;
    Some((tag, bytes.get(head..end)?, bytes.get(end..)?))
}

fn oid_in_algorithm(alg_identifier_content: &[u8]) -> Option<String> {
    let (tag, oid, _) = tlv(alg_identifier_content)?;
    if tag != 0x06 {
        return None;
    }
    let oid = ObjectIdentifier::from_bytes(oid).ok()?;
    Some(oid.to_string())
}

/// Thuật toán băm trong `RSASSA-PSS-params` (mặc định SHA-1 theo RFC 4055).
pub(crate) fn pss_hash_of(parameters: Option<&der::Any>) -> Option<DigestAlg> {
    pss_hash(parameters)
}

fn pss_hash(parameters: Option<&der::Any>) -> Option<DigestAlg> {
    let Some(p) = parameters else {
        return Some(DigestAlg::Sha1);
    };
    let der = p.to_der().ok()?;
    let (tag, seq, _) = tlv(&der)?;
    if tag != 0x30 {
        return None;
    }
    match tlv(seq) {
        Some((0xA0, explicit, _)) => {
            let (tag, alg, _) = tlv(explicit)?;
            if tag != 0x30 {
                return None;
            }
            let oid = ObjectIdentifier::new(&oid_in_algorithm(alg)?).ok()?;
            DigestAlg::from_oid(&oid)
        }
        _ => Some(DigestAlg::Sha1),
    }
}

/// Băm chứng thư người ký ghi trong `signingCertificate` / `signingCertificateV2` (ESS, RFC 5035), nếu có:
/// (thuật toán, giá trị). `None` khi không có thuộc tính; `Some(Err)` khi có nhưng không đọc được.
fn ess_cert_hash(info: &SignerInfo) -> Option<std::result::Result<(DigestAlg, Vec<u8>), ()>> {
    let (v2, attr) = match signed_attribute(info, OID_SIGNING_CERTIFICATE_V2) {
        Some(a) => (true, a),
        None => (false, signed_attribute(info, OID_SIGNING_CERTIFICATE)?),
    };
    let parse = || -> Option<(DigestAlg, Vec<u8>)> {
        let der = attr.to_der().ok()?;
        let (_, signing_certificate, _) = tlv(&der)?; // SigningCertificate(V2) ::= SEQUENCE
        let (_, certs, _) = tlv(signing_certificate)?; // certs SEQUENCE OF
        let (_, first, _) = tlv(certs)?; // ESSCertID(v2) đầu tiên: của người ký
        let (tag, content, rest) = tlv(first)?;
        if v2 && tag == 0x30 {
            let oid = ObjectIdentifier::new(&oid_in_algorithm(content)?).ok()?;
            let alg = DigestAlg::from_oid(&oid)?;
            let (tag, hash, _) = tlv(rest)?;
            (tag == 0x04).then(|| (alg, hash.to_vec()))
        } else if tag == 0x04 {
            Some((
                if v2 {
                    DigestAlg::Sha256
                } else {
                    DigestAlg::Sha1
                },
                content.to_vec(),
            ))
        } else {
            None
        }
    };
    Some(parse().ok_or(()))
}

/// Giải mã `ContentInfo` chứa `SignedData`.
pub fn parse_signed_data(der_bytes: &[u8]) -> Result<SignedData> {
    let info = ContentInfo::from_der(trim_der(der_bytes)).map_err(|e| {
        fail(
            ErrorCode::SignatureInvalid,
            format!("Khối chữ ký không phải CMS hợp lệ: {e}"),
        )
    })?;
    if info.content_type.to_string() != OID_SIGNED_DATA {
        return Err(fail(
            ErrorCode::SignatureInvalid,
            "Khối chữ ký không phải CMS SignedData.",
        ));
    }
    info.content.decode_as::<SignedData>().map_err(|e| {
        fail(
            ErrorCode::SignatureInvalid,
            format!("SignedData không đọc được: {e}"),
        )
    })
}

/// `/Contents` của PDF được đệm số 0 sau DER: cắt đúng độ dài DER.
pub fn trim_der(bytes: &[u8]) -> &[u8] {
    der_length(bytes).map_or(bytes, |len| &bytes[..len.min(bytes.len())])
}

fn der_length(bytes: &[u8]) -> Option<usize> {
    let first = *bytes.get(1)?;
    if first < 0x80 {
        return Some(2 + usize::from(first));
    }
    let n = usize::from(first & 0x7F);
    if n == 0 || n > 4 {
        return None;
    }
    let mut len = 0usize;
    for i in 0..n {
        len = (len << 8) | usize::from(*bytes.get(2 + i)?);
    }
    Some(2 + n + len)
}

/// Mọi chứng thư nhúng trong SignedData.
pub fn embedded_certificates(sd: &SignedData) -> Vec<Cert> {
    sd.certificates
        .as_ref()
        .map(|set| {
            set.0
                .iter()
                .filter_map(|c| match c {
                    CertificateChoices::Certificate(c) => Cert::from_parsed(c.clone()).ok(),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Chứng thư của người ký theo `sid` (issuer + serial, hoặc subjectKeyIdentifier).
pub fn find_signer_certificate(sd: &SignedData, info: &SignerInfo) -> Option<Cert> {
    let certs = embedded_certificates(sd);
    match &info.sid {
        SignerIdentifier::IssuerAndSerialNumber(ias) => certs.into_iter().find(|c| {
            let tbs = c.parsed().tbs_certificate();
            tbs.issuer() == &ias.issuer && tbs.serial_number() == &ias.serial_number
        }),
        SignerIdentifier::SubjectKeyIdentifier(ski) => certs.into_iter().find(|c| {
            use x509_cert::ext::pkix::SubjectKeyIdentifier;
            matches!(c.parsed().tbs_certificate().get_extension::<SubjectKeyIdentifier>(),
                Ok(Some((_, own))) if own.0.as_bytes() == ski.0.as_bytes())
        }),
    }
}

/// Giá trị một signed attribute (bản DER của phần tử đầu trong SET).
pub fn signed_attribute(info: &SignerInfo, oid: &str) -> Option<der::Any> {
    let oid = ObjectIdentifier::new(oid).ok()?;
    info.signed_attrs
        .as_ref()?
        .iter()
        .find(|a| a.oid == oid)
        .and_then(|a| a.values.iter().next().cloned())
}

/// Kết quả kiểm một SignerInfo.
#[derive(Debug, Clone)]
pub struct SignerCheck {
    pub digest_alg: DigestAlg,
    pub signer: Cert,
    /// Có thuộc tính `signingCertificate(V2)` gắn chữ ký với đúng chứng thư (và đã khớp).
    pub cert_bound: bool,
}

/// Kiểm SignerInfo: `messageDigest` khớp `content_digest` (băm nội dung bằng thuật toán của SignerInfo, do
/// bên gọi tính qua `content`), rồi chữ ký trên DER của signed attributes.
pub fn verify_signer_info(
    sd: &SignedData,
    info: &SignerInfo,
    content: &[&[u8]],
) -> Result<SignerCheck> {
    let digest_alg = DigestAlg::from_oid(&info.digest_alg.oid).ok_or_else(|| {
        fail(
            ErrorCode::SignatureInvalid,
            format!("Thuật toán băm {} chưa được hỗ trợ.", info.digest_alg.oid),
        )
    })?;
    let signer = find_signer_certificate(sd, info).ok_or_else(|| {
        fail(
            ErrorCode::SignatureInvalid,
            "Khối chữ ký không kèm chứng thư của người ký.",
        )
    })?;
    let content_digest = digest_alg.digest(content);
    let signature_oid = info.signature_algorithm.oid.to_string();
    let parameters = info.signature_algorithm.parameters.as_ref();
    let signature = info.signature.as_bytes();
    match &info.signed_attrs {
        Some(attrs) => {
            let md = signed_attribute(info, OID_MESSAGE_DIGEST)
                .and_then(|v| v.decode_as::<OctetString>().ok())
                .ok_or_else(|| {
                    fail(
                        ErrorCode::SignatureInvalid,
                        "Thiếu thuộc tính messageDigest.",
                    )
                })?;
            if md.as_bytes() != content_digest.as_slice() {
                return Err(fail(
                    ErrorCode::SignatureInvalid,
                    "Nội dung đã bị sửa sau khi ký (messageDigest không khớp).",
                ));
            }
            let attrs_der = attrs.to_der().map_err(|e| {
                fail(
                    ErrorCode::SignatureInvalid,
                    format!("Signed attributes lỗi: {e}"),
                )
            })?;
            let attrs_digest = digest_alg.digest(&[&attrs_der]);
            verify_digest_signature_with(
                &signer,
                digest_alg,
                &attrs_digest,
                &signature_oid,
                parameters,
                signature,
            )?;
        }
        None => verify_digest_signature_with(
            &signer,
            digest_alg,
            &content_digest,
            &signature_oid,
            parameters,
            signature,
        )?,
    }
    // Chứng thư nhúng phải đúng chứng thư được ghi băm trong chữ ký (chống tráo chứng thư cùng khoá).
    let cert_bound = match ess_cert_hash(info) {
        None => false,
        Some(Err(())) => {
            return Err(fail(
                ErrorCode::SignatureInvalid,
                "Thuộc tính signingCertificate của chữ ký bị hỏng.",
            ));
        }
        Some(Ok((alg, hash))) => {
            if alg.digest(&[signer.der()]) != hash {
                return Err(fail(
                    ErrorCode::SignatureInvalid,
                    "Chứng thư nhúng trong chữ ký không phải chứng thư đã ký (signingCertificate không khớp).",
                ));
            }
            true
        }
    };
    Ok(SignerCheck {
        digest_alg,
        signer,
        cert_bound,
    })
}

/// Thông tin đọc được từ một token dấu thời gian đã kiểm.
#[derive(Debug, Clone)]
pub struct TimestampInfo {
    /// `genTime`, giây Unix (UTC).
    pub gen_time: i64,
    pub serial_hex: String,
    pub policy: String,
    pub tsa: Cert,
    /// Mọi chứng thư nhúng trong token (để dựng chuỗi của TSA).
    pub certificates: Vec<Cert>,
}

/// Kiểm token RFC 3161 phủ đúng `data` (với chữ ký PAdES: `data` là giá trị chữ ký của người ký): imprint
/// khớp, chữ ký của TSA đúng, chứng thư TSA có EKU `timeStamping`. Chuỗi tới Root G2 kiểm ở `chain`.
pub fn verify_timestamp_token(token_der: &[u8], data: &[u8]) -> Result<TimestampInfo> {
    verify_timestamp_token_parts(token_der, &[data])
}

/// Như [`verify_timestamp_token`] với dữ liệu nhiều đoạn (dấu thời gian tài liệu: hai đoạn `/ByteRange`).
pub fn verify_timestamp_token_parts(token_der: &[u8], data: &[&[u8]]) -> Result<TimestampInfo> {
    let bad = |m: String| fail(ErrorCode::TimestampInvalid, m);
    let sd = parse_signed_data(token_der).map_err(|e| bad(e.message))?;
    if sd.encap_content_info.econtent_type.to_string() != OID_TST_INFO {
        return Err(bad("Token dấu thời gian không chứa TSTInfo.".into()));
    }
    let econtent = sd
        .encap_content_info
        .econtent
        .as_ref()
        .ok_or_else(|| bad("Token dấu thời gian không có nội dung.".into()))?;
    let octets: OctetString = econtent
        .decode_as()
        .map_err(|e| bad(format!("TSTInfo lỗi: {e}")))?;
    let tst = TstInfo::from_der(octets.as_bytes())
        .map_err(|e| bad(format!("TSTInfo không đọc được: {e}")))?;
    let imprint_alg = DigestAlg::from_oid(&tst.message_imprint.hash_algorithm.oid)
        .ok_or_else(|| bad("Thuật toán băm của dấu thời gian không hỗ trợ.".into()))?;
    if tst.message_imprint.hashed_message.as_bytes() != imprint_alg.digest(data).as_slice() {
        return Err(bad(
            "Dấu thời gian không thuộc chữ ký này (messageImprint không khớp).".into(),
        ));
    }
    let info = sd
        .signer_infos
        .0
        .iter()
        .next()
        .ok_or_else(|| bad("Token dấu thời gian không có SignerInfo.".into()))?;
    let check = verify_signer_info(&sd, info, &[octets.as_bytes()]).map_err(|e| {
        let code = if e.code == ErrorCode::SignatureUnsupported {
            ErrorCode::SignatureUnsupported
        } else {
            ErrorCode::TimestampInvalid
        };
        fail(
            code,
            format!(
                "Chữ ký của máy chủ dấu thời gian không hợp lệ: {}",
                e.message
            ),
        )
    })?;
    if !check
        .signer
        .has_eku(smks_core::consts::oid::EKU_TIME_STAMPING)
    {
        return Err(bad(
            "Chứng thư máy chủ dấu thời gian không có quyền timeStamping.".into(),
        ));
    }
    let gen_time = i64::try_from(tst.gen_time.to_unix_duration().as_secs()).unwrap_or(0);
    Ok(TimestampInfo {
        gen_time,
        serial_hex: crate::cert::hex_upper(tst.serial_number.as_bytes()),
        policy: tst.policy.to_string(),
        tsa: check.signer,
        certificates: embedded_certificates(&sd),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::SigningKey;
    use crate::testpki::TestPki;

    #[test]
    fn p256_signature_round_trip_and_tamper() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let digest = DigestAlg::Sha256.digest(&[b"noi dung"]);
        let digest: [u8; 32] = digest.try_into().unwrap();
        let sig = key.sign_digest(&digest).unwrap();
        verify_digest_signature(
            &pki.signer,
            DigestAlg::Sha256,
            &digest,
            "1.2.840.10045.4.3.2",
            &sig,
        )
        .unwrap();
        let mut other = digest;
        other[0] ^= 1;
        assert!(
            verify_digest_signature(
                &pki.signer,
                DigestAlg::Sha256,
                &other,
                "1.2.840.10045.4.3.2",
                &sig
            )
            .is_err()
        );
        // Khoá của CA không xác minh được chữ ký của người ký.
        assert!(
            verify_digest_signature(
                &pki.ca,
                DigestAlg::Sha256,
                &digest,
                "1.2.840.10045.4.3.2",
                &sig
            )
            .is_err()
        );
    }

    #[test]
    fn ess_and_pss_parameters() {
        // signingCertificateV2 do `cms` dựng: khớp chứng thư người ký.
        let pki = TestPki::new();
        let key = pki.software_key();
        let digest: [u8; 32] = DigestAlg::Sha256.digest(&[b"x"]).try_into().unwrap();
        let der = crate::cms::build_cades(&digest, &key, &[], None).unwrap();
        let sd = parse_signed_data(&der).unwrap();
        let info = sd.signer_infos.0.iter().next().unwrap();
        let (alg, hash) = ess_cert_hash(info).unwrap().unwrap();
        assert_eq!(alg, DigestAlg::Sha256);
        assert_eq!(hash, pki.signer.sha256().to_vec());
        // RSASSA-PSS-params: không có tham số → SHA-1; [0] SHA-256.
        assert_eq!(pss_hash(None), Some(DigestAlg::Sha1));
        let sha256_params = der::Any::from_der(&[
            0x30, 0x0F, 0xA0, 0x0D, 0x30, 0x0B, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03,
            0x04, 0x02, 0x01,
        ])
        .unwrap();
        assert_eq!(pss_hash(Some(&sha256_params)), Some(DigestAlg::Sha256));
    }

    #[test]
    fn trims_zero_padding_after_der() {
        let mut padded = vec![0x30, 0x03, 0x02, 0x01, 0x05];
        padded.extend([0u8; 10]);
        assert_eq!(trim_der(&padded), &[0x30, 0x03, 0x02, 0x01, 0x05]);
        let long = [0x30, 0x82, 0x01, 0x00];
        assert_eq!(der_length(&long), Some(4 + 256));
    }
}
