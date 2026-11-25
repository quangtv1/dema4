//! Đọc thông tin một chứng thư X.509 (thuần Rust, dùng chung cho kho chứng thư, ký và xác minh).

use der::asn1::ObjectIdentifier;
use der::{Decode, Encode};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use smks_core::{ErrorCode, Result, SmksError};
use x509_cert::Certificate;
use x509_cert::ext::pkix::{ExtendedKeyUsage, KeyUsage, KeyUsages};

/// `commonName`.
const OID_CN: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const OID_O: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.10");
const OID_OU: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.11");
/// `rsaEncryption`.
pub const OID_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
/// `id-ecPublicKey`.
pub const OID_EC: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
pub const OID_P256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
pub const OID_P384: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.132.0.34");

/// Loại khoá công khai của chứng thư.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Rsa,
    EcP256,
    EcP384,
    Other,
}

fn invalid(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::SignTokenInvalid, message)
}

/// Chứng thư đã giải mã kèm bản DER gốc (băm, nhúng vào CMS phải dùng đúng các byte gốc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cert {
    der: Vec<u8>,
    parsed: Certificate,
}

impl Cert {
    pub fn from_der(der: &[u8]) -> Result<Self> {
        let parsed = Certificate::from_der(der)
            .map_err(|e| invalid(format!("Chứng thư số không đọc được: {e}")))?;
        Ok(Self {
            der: der.to_vec(),
            parsed,
        })
    }

    pub fn from_parsed(parsed: Certificate) -> Result<Self> {
        let der = parsed
            .to_der()
            .map_err(|e| invalid(format!("Chứng thư số không mã hoá lại được: {e}")))?;
        Ok(Self { der, parsed })
    }

    pub fn der(&self) -> &[u8] {
        &self.der
    }

    pub fn parsed(&self) -> &Certificate {
        &self.parsed
    }

    /// Chuỗi chủ thể dạng RFC 4514 (`CN=…,O=…`).
    pub fn subject(&self) -> String {
        self.parsed.tbs_certificate().subject().to_string()
    }

    pub fn issuer(&self) -> String {
        self.parsed.tbs_certificate().issuer().to_string()
    }

    /// CN của chủ thể; không có thì trả cả chuỗi chủ thể (như `GetNameInfo(SimpleName)` của .NET).
    pub fn common_name(&self) -> String {
        common_name(self.parsed.tbs_certificate().subject()).unwrap_or_else(|| self.subject())
    }

    pub fn issuer_common_name(&self) -> String {
        common_name(self.parsed.tbs_certificate().issuer()).unwrap_or_else(|| self.issuer())
    }

    /// Đơn vị của chủ thể: các `OU` rồi `O`, nối bằng ", " (rỗng nếu không có).
    pub fn organization(&self) -> String {
        let name = self.parsed.tbs_certificate().subject();
        let mut parts = attribute(name, OID_OU);
        parts.extend(attribute(name, OID_O));
        parts.join(", ")
    }

    /// Số serial dạng hex in hoa (như `X509Certificate2.SerialNumber`).
    pub fn serial_hex(&self) -> String {
        hex_upper(self.parsed.tbs_certificate().serial_number().as_bytes())
    }

    /// Thumbprint SHA-1 hex in hoa, như Windows và bản cũ dùng để định danh chứng thư.
    pub fn thumbprint(&self) -> String {
        hex_upper(&Sha1::digest(&self.der))
    }

    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(&self.der).into()
    }

    /// Thời điểm bắt đầu và hết hiệu lực, tính bằng giây Unix.
    pub fn validity(&self) -> (i64, i64) {
        let v = self.parsed.tbs_certificate().validity();
        let secs = |t: &x509_cert::time::Time| {
            i64::try_from(t.to_unix_duration().as_secs()).unwrap_or(i64::MAX)
        };
        (secs(&v.not_before), secs(&v.not_after))
    }

    pub fn is_valid_at(&self, unix_seconds: i64) -> bool {
        let (from, to) = self.validity();
        from <= unix_seconds && unix_seconds <= to
    }

    /// Không có KeyUsage, hoặc có `digitalSignature` / `nonRepudiation` (như bản cũ).
    pub fn allows_signing(&self) -> bool {
        match self.parsed.tbs_certificate().get_extension::<KeyUsage>() {
            Ok(Some((_, usage))) => {
                usage.0.contains(KeyUsages::DigitalSignature)
                    || usage.0.contains(KeyUsages::NonRepudiation)
            }
            _ => true,
        }
    }

    /// Có Extended Key Usage `oid` (ví dụ `timeStamping` của chứng thư TSA).
    pub fn has_eku(&self, oid: &str) -> bool {
        let Ok(oid) = ObjectIdentifier::new(oid) else {
            return false;
        };
        matches!(
            self.parsed.tbs_certificate().get_extension::<ExtendedKeyUsage>(),
            Ok(Some((_, eku))) if eku.0.contains(&oid)
        )
    }

    pub fn key_kind(&self) -> KeyKind {
        let spki = self.parsed.tbs_certificate().subject_public_key_info();
        if spki.algorithm.oid == OID_RSA {
            return KeyKind::Rsa;
        }
        if spki.algorithm.oid == OID_EC {
            let curve = spki
                .algorithm
                .parameters
                .as_ref()
                .and_then(|p| p.decode_as::<ObjectIdentifier>().ok());
            return match curve {
                Some(c) if c == OID_P256 => KeyKind::EcP256,
                Some(c) if c == OID_P384 => KeyKind::EcP384,
                _ => KeyKind::Other,
            };
        }
        KeyKind::Other
    }

    /// Byte khoá công khai (PKCS#1 với RSA, điểm SEC1 với EC).
    pub fn public_key_bytes(&self) -> &[u8] {
        self.parsed
            .tbs_certificate()
            .subject_public_key_info()
            .subject_public_key
            .raw_bytes()
    }

    /// Chứng thư này do `issuer` cấp (so tên; chữ ký được kiểm ở `verify`).
    pub fn issued_by(&self, issuer: &Cert) -> bool {
        self.parsed.tbs_certificate().issuer() == issuer.parsed.tbs_certificate().subject()
    }

    pub fn is_self_issued(&self) -> bool {
        self.issued_by(self)
    }
}

fn common_name(name: &x509_cert::name::Name) -> Option<String> {
    attribute(name, OID_CN).last().cloned()
}

/// Mọi giá trị của thuộc tính `oid` trong tên, theo thứ tự.
fn attribute(name: &x509_cert::name::Name, oid: ObjectIdentifier) -> Vec<String> {
    // CN cuối cùng (gần chủ thể nhất) là CN chính, như .NET.
    name.as_ref()
        .iter()
        .flat_map(|rdn| rdn.as_ref().iter())
        .filter(|atv| atv.oid == oid)
        .filter_map(|atv| directory_string(&atv.value))
        .collect()
}

/// Giải mã DirectoryString: UTF8String, PrintableString, BMPString, IA5String, TeletexString (coi như Latin-1).
fn directory_string(value: &der::Any) -> Option<String> {
    use der::{Tag, Tagged};
    let bytes = value.value();
    match value.tag() {
        Tag::Utf8String | Tag::PrintableString | Tag::Ia5String | Tag::VisibleString => {
            String::from_utf8(bytes.to_vec()).ok()
        }
        Tag::BmpString => {
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes(*c))
                .collect();
            String::from_utf16(&units).ok()
        }
        Tag::TeletexString => Some(bytes.iter().map(|b| char::from(*b)).collect()),
        _ => None,
    }
}

pub fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02X}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::PINNED_CAS;

    fn ca(file: &str) -> Cert {
        let pinned = PINNED_CAS.iter().find(|c| c.file_name() == file).unwrap();
        Cert::from_der(&pinned.verified_der().unwrap()).unwrap()
    }

    #[test]
    fn reads_vietnamese_common_names_and_chain_links() {
        let root = ca("rootcag2.crt");
        let sub = ca("cpg2.crt");
        assert_eq!(root.common_name(), "RootCA chuyên dùng Chính phủ G2");
        assert!(root.is_self_issued());
        assert!(!sub.is_self_issued());
        assert!(sub.issued_by(&ca("dcscag2.crt")) || sub.issued_by(&root));
        assert_eq!(root.thumbprint().len(), 40);
        assert_eq!(root.key_kind(), KeyKind::Rsa);
        let (from, to) = root.validity();
        assert!(from < to);
    }
}
