//! PKI thử dựng lúc chạy test (P-256, khoá cố định theo hạt giống). Không có khoá bí mật nào nằm trong repo.
//! Chỉ có khi chạy test hoặc bật `dev-keys`, không bao giờ vào bản phát hành; lỗi ở đây là lỗi dựng test nên
//! được phép dừng chương trình.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::new_without_default)]

use std::str::FromStr;
use std::time::Duration;

use der::asn1::ObjectIdentifier;
use p256::ecdsa::{DerSignature, SigningKey as P256Key};
use spki::{SubjectPublicKeyInfo, SubjectPublicKeyInfoRef};
use x509_cert::TbsCertificate;
use x509_cert::builder::profile::BuilderProfile;
use x509_cert::builder::{Builder, CertificateBuilder};
use x509_cert::ext::pkix::{BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages};
use x509_cert::ext::{Extension, ToExtension};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::time::Validity;

use crate::cert::Cert;
use crate::key::software::SoftwareKey;

struct Profile {
    subject: Name,
    issuer: Option<Name>,
    ca: bool,
    eku: Option<ObjectIdentifier>,
}

impl BuilderProfile for Profile {
    fn get_issuer(&self, subject: &Name) -> Name {
        self.issuer.clone().unwrap_or_else(|| subject.clone())
    }

    fn get_subject(&self) -> Name {
        self.subject.clone()
    }

    fn build_extensions(
        &self,
        _spk: SubjectPublicKeyInfoRef<'_>,
        _issuer_spk: SubjectPublicKeyInfoRef<'_>,
        tbs: &TbsCertificate,
    ) -> x509_cert::builder::Result<Vec<Extension>> {
        let subject = tbs.subject();
        let mut out = vec![
            BasicConstraints {
                ca: self.ca,
                path_len_constraint: None,
            }
            .to_extension(subject, &[])?,
        ];
        let usage = if self.ca {
            KeyUsages::KeyCertSign | KeyUsages::CRLSign
        } else {
            KeyUsages::DigitalSignature | KeyUsages::NonRepudiation
        };
        out.push(KeyUsage(usage).to_extension(subject, &[])?);
        if let Some(oid) = self.eku {
            out.push(ExtendedKeyUsage(vec![oid]).to_extension(subject, &[])?);
        }
        Ok(out)
    }
}

pub fn key(seed: u8) -> P256Key {
    let mut bytes = [seed; 32];
    bytes[0] = 0x42;
    P256Key::from_slice(&bytes).unwrap()
}

/// Chứng thư `cn` cho khoá `subject_key`, do `issuer` (chứng thư + khoá) cấp; `None` thì tự ký.
pub fn issue(
    cn: &str,
    subject_key: &P256Key,
    issuer: Option<(&Cert, &P256Key)>,
    ca: bool,
    eku: Option<&str>,
) -> Cert {
    let profile = Profile {
        subject: Name::from_str(&format!("CN={cn},O=Sao Mai thử nghiệm,C=VN")).unwrap(),
        issuer: issuer.map(|(c, _)| c.parsed().tbs_certificate().subject().clone()),
        ca,
        eku: eku.map(|o| ObjectIdentifier::new(o).unwrap()),
    };
    let spki = SubjectPublicKeyInfo::from_key(subject_key.verifying_key()).unwrap();
    let serial = SerialNumber::from(
        u32::from(subject_key.verifying_key().to_sec1_point(true).as_bytes()[1]) + 1000,
    );
    let builder = CertificateBuilder::new(
        profile,
        serial,
        Validity::from_now(Duration::from_secs(3600 * 24 * 365)).unwrap(),
        spki,
    )
    .unwrap();
    let signer = issuer.map_or(subject_key, |(_, k)| k);
    let cert = builder.build::<_, DerSignature>(signer).unwrap();
    Cert::from_parsed(cert).unwrap()
}

/// CA thử và một người ký do CA đó cấp.
#[allow(missing_debug_implementations)]
pub struct TestPki {
    pub ca: Cert,
    pub signer: Cert,
    pub signer_key: P256Key,
}

impl TestPki {
    pub fn new() -> Self {
        let ca_key = key(1);
        let ca = issue("CA thử", &ca_key, None, true, None);
        let signer_key = key(2);
        let signer = issue(
            "Nguyễn Văn Thử",
            &signer_key,
            Some((&ca, &ca_key)),
            false,
            None,
        );
        Self {
            ca,
            signer,
            signer_key,
        }
    }

    pub fn software_key(&self) -> SoftwareKey {
        SoftwareKey::from_parts_p256(self.signer.clone(), self.signer_key.clone())
    }

    /// CRL của CA thử (lập lúc gọi, hạn 1 ngày); `revoked_at`: thu hồi chứng thư người ký từ thời điểm đó.
    pub fn crl(&self, revoked_at: Option<i64>) -> Vec<u8> {
        use x509_cert::builder::CrlBuilder;
        use x509_cert::crl::RevokedCert;
        use x509_cert::ext::pkix::CrlNumber;
        use x509_cert::time::Time;
        let next =
            Time::try_from(std::time::SystemTime::now() + Duration::from_secs(86_400)).unwrap();
        let revoked = revoked_at.map(|t| RevokedCert {
            serial_number: self
                .signer
                .parsed()
                .tbs_certificate()
                .serial_number()
                .clone(),
            revocation_date: Time::try_from(
                std::time::UNIX_EPOCH + Duration::from_secs(u64::try_from(t).unwrap()),
            )
            .unwrap(),
            crl_entry_extensions: None,
        });
        let builder = CrlBuilder::new(
            self.ca.parsed(),
            CrlNumber(der::asn1::Uint::new(&[1]).unwrap()),
        )
        .unwrap()
        .with_next_update(Some(next))
        .with_certificates(revoked.into_iter());
        let crl = builder.build::<_, DerSignature>(&key(1)).unwrap();
        der::Encode::to_der(&crl).unwrap()
    }
}
