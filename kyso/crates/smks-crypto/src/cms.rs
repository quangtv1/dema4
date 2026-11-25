//! Dựng CMS SignedData detached theo CAdES / PAdES B-T (kế hoạch 6.3, ADR 0006). Port `PdfSigner.CadesSigner`
//! của bản cũ, khác ở chỗ không đưa `signing-time` (thời điểm ký nằm ở `/M` của từ điển chữ ký).
//!
//! - Signed attributes: `contentType = id-data`, `messageDigest`, `signingCertificateV2` (chỉ `certHash`
//!   SHA-256, như bản cũ).
//! - Chữ ký tính trên `SHA-256(DER(signedAttrs))` bằng [`SigningKey`], nên khoá trên token hay khoá phần mềm
//!   đều dùng chung đường này.
//! - Dấu thời gian (nếu có) phủ giá trị chữ ký, gắn vào unsigned attribute `signatureTimeStampToken`.
//! - `certificates`: chứng thư người ký và các CA trung gian (như `X509IncludeOption.WholeChain`, trừ gốc).

use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo, SignerInfos,
};
use der::asn1::{ObjectIdentifier, OctetString, SetOfVec};
use der::{Any, Decode, Encode};
use sha2::{Digest, Sha256};
use smks_core::consts::oid;
use smks_core::{ErrorCode, Result, SmksError};
use spki::AlgorithmIdentifierOwned;
use x509_cert::attr::Attribute;

use crate::cert::Cert;
use crate::key::{SigAlg, SigningKey};
use crate::verify::{OID_CONTENT_TYPE, OID_DATA, OID_MESSAGE_DIGEST, OID_SIGNED_DATA};

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignFailed, message)
}

fn oid_of(text: &str) -> Result<ObjectIdentifier> {
    ObjectIdentifier::new(text).map_err(|e| fail(format!("OID {text} lỗi: {e}")))
}

fn attribute(oid: &str, value: Any) -> Result<Attribute> {
    Ok(Attribute {
        oid: oid_of(oid)?,
        values: SetOfVec::try_from(vec![value]).map_err(|e| fail(e.to_string()))?,
    })
}

fn encode_err(e: der::Error) -> SmksError {
    fail(format!("Không mã hoá được khối chữ ký: {e}"))
}

/// `signingCertificateV2` chỉ có `certHash` (băm mặc định SHA-256 nên không ghi thuật toán):
/// `SEQUENCE { SEQUENCE OF { ESSCertIDv2 { OCTET STRING } } }`.
fn signing_certificate_v2(cert: &Cert) -> Vec<u8> {
    let mut out = vec![0x30, 0x26, 0x30, 0x24, 0x30, 0x22, 0x04, 0x20];
    out.extend_from_slice(&cert.sha256());
    out
}

/// Hàm lấy dấu thời gian: nhận giá trị chữ ký, trả về DER của TimeStampToken.
pub type TimestampFn<'a> = dyn Fn(&[u8]) -> Result<Vec<u8>> + 'a;

/// Dựng CMS cho băm SHA-256 `document_digest` của các byte theo `/ByteRange`.
pub fn build_cades(
    document_digest: &[u8; 32],
    key: &dyn SigningKey,
    chain: &[Cert],
    timestamp: Option<&TimestampFn<'_>>,
) -> Result<Vec<u8>> {
    let signer = key.certificate();
    let signed_attrs = SetOfVec::try_from(vec![
        attribute(
            OID_CONTENT_TYPE,
            Any::encode_from(&oid_of(OID_DATA)?).map_err(encode_err)?,
        )?,
        attribute(
            OID_MESSAGE_DIGEST,
            Any::encode_from(&OctetString::new(document_digest.as_slice()).map_err(encode_err)?)
                .map_err(encode_err)?,
        )?,
        attribute(
            oid::SIGNING_CERTIFICATE_V2,
            Any::from_der(&signing_certificate_v2(signer)).map_err(encode_err)?,
        )?,
    ])
    .map_err(encode_err)?;

    // Chữ ký phủ DER của signedAttrs dưới dạng SET OF (thẻ 0x31).
    let to_sign: [u8; 32] = Sha256::digest(signed_attrs.to_der().map_err(encode_err)?).into();
    let signature = key.sign_digest(&to_sign)?;

    let unsigned_attrs = match timestamp {
        Some(get) => {
            let token = get(&signature)?;
            Some(
                SetOfVec::try_from(vec![attribute(
                    oid::SIGNATURE_TIME_STAMP_TOKEN,
                    Any::from_der(&token)
                        .map_err(|e| fail(format!("Token dấu thời gian lỗi: {e}")))?,
                )?])
                .map_err(encode_err)?,
            )
        }
        None => None,
    };

    let tbs = signer.parsed().tbs_certificate();
    let signature_algorithm = match key.algorithm() {
        // Như .NET: rsaEncryption với tham số NULL.
        SigAlg::RsaPkcs1Sha256 => AlgorithmIdentifierOwned {
            oid: oid_of(SigAlg::RsaPkcs1Sha256.signature_oid())?,
            parameters: Some(Any::from_der(&[0x05, 0x00]).map_err(encode_err)?),
        },
        SigAlg::EcdsaSha256 => AlgorithmIdentifierOwned {
            oid: oid_of(SigAlg::EcdsaSha256.signature_oid())?,
            parameters: None,
        },
    };
    let sha256 = AlgorithmIdentifierOwned {
        oid: oid_of(oid::SHA256)?,
        parameters: None,
    };
    let signer_info = SignerInfo {
        version: CmsVersion::V1,
        sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: tbs.issuer().clone(),
            serial_number: tbs.serial_number().clone(),
        }),
        digest_alg: sha256.clone(),
        signed_attrs: Some(signed_attrs),
        signature_algorithm,
        signature: OctetString::new(signature).map_err(encode_err)?,
        unsigned_attrs,
    };

    let mut certificates = vec![CertificateChoices::Certificate(signer.parsed().clone())];
    for cert in chain {
        if cert.der() != signer.der() {
            certificates.push(CertificateChoices::Certificate(cert.parsed().clone()));
        }
    }
    let signed_data = SignedData {
        version: CmsVersion::V1,
        digest_algorithms: SetOfVec::try_from(vec![sha256]).map_err(encode_err)?,
        encap_content_info: EncapsulatedContentInfo {
            econtent_type: oid_of(OID_DATA)?,
            econtent: None,
        },
        certificates: Some(CertificateSet(
            SetOfVec::try_from(certificates).map_err(encode_err)?,
        )),
        crls: None,
        signer_infos: SignerInfos(SetOfVec::try_from(vec![signer_info]).map_err(encode_err)?),
    };
    let content_info = ContentInfo {
        content_type: oid_of(OID_SIGNED_DATA)?,
        content: Any::encode_from(&signed_data).map_err(encode_err)?,
    };
    content_info.to_der().map_err(encode_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpki::TestPki;
    use crate::verify::{DigestAlg, parse_signed_data, signed_attribute, verify_signer_info};

    #[test]
    fn builds_pades_attributes_and_verifies() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let content: &[&[u8]] = &[b"phan dau ", b"phan sau"];
        let digest: [u8; 32] = DigestAlg::Sha256.digest(content).try_into().unwrap();
        let der = build_cades(&digest, &key, std::slice::from_ref(&pki.ca), None).unwrap();

        let sd = parse_signed_data(&der).unwrap();
        assert!(sd.encap_content_info.econtent.is_none(), "phải là detached");
        assert_eq!(sd.certificates.as_ref().unwrap().0.len(), 2);
        let info = sd.signer_infos.0.iter().next().unwrap();
        // Không có signing-time (PAdES), có signingCertificateV2 với băm đúng chứng thư.
        assert!(signed_attribute(info, oid::SIGNING_TIME).is_none());
        let scv2 = signed_attribute(info, oid::SIGNING_CERTIFICATE_V2)
            .unwrap()
            .to_der()
            .unwrap();
        assert!(scv2.ends_with(&pki.signer.sha256()));
        let check = verify_signer_info(&sd, info, content).unwrap();
        assert_eq!(check.signer.thumbprint(), pki.signer.thumbprint());
        // Nội dung khác thì messageDigest không khớp.
        let err = verify_signer_info(&sd, info, &[b"phan dau ", b"phan sax"]).unwrap_err();
        assert_eq!(err.code, ErrorCode::SignatureInvalid);
    }

    #[test]
    fn timestamp_callback_receives_signature_and_token_is_attached() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let digest = [9u8; 32];
        let seen = std::cell::RefCell::new(Vec::new());
        // Token giả chỉ để kiểm cách gắn: một SEQUENCE rỗng.
        let fake = |sig: &[u8]| -> Result<Vec<u8>> {
            seen.borrow_mut().extend_from_slice(sig);
            Ok(vec![0x30, 0x00])
        };
        let der = build_cades(&digest, &key, &[], Some(&fake)).unwrap();
        let sd = parse_signed_data(&der).unwrap();
        let info = sd.signer_infos.0.iter().next().unwrap();
        assert_eq!(seen.borrow().as_slice(), info.signature.as_bytes());
        let unsigned = info.unsigned_attrs.as_ref().unwrap();
        assert_eq!(
            unsigned.iter().next().unwrap().oid.to_string(),
            oid::SIGNATURE_TIME_STAMP_TOKEN
        );
    }
}
