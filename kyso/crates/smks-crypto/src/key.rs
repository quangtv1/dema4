//! Khoá ký (kế hoạch 6.2). Phần còn lại của lõi chỉ thấy [`SigningKey`]: một hàm nhận băm SHA-256 và trả về
//! chữ ký, nên khoá trên USB token (CNG/CAPI, module `store`) và khoá phần mềm để thử dùng chung một đường.

use smks_core::Result;

use crate::cert::{Cert, KeyKind};

/// Thuật toán chữ ký. Băm luôn là SHA-256, như bản cũ (`SignData(…, SHA256, Pkcs1)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigAlg {
    RsaPkcs1Sha256,
    EcdsaSha256,
}

impl SigAlg {
    /// Thuật toán theo khoá công khai của chứng thư.
    pub fn for_certificate(cert: &Cert) -> Option<Self> {
        match cert.key_kind() {
            KeyKind::Rsa => Some(Self::RsaPkcs1Sha256),
            KeyKind::EcP256 | KeyKind::EcP384 => Some(Self::EcdsaSha256),
            KeyKind::Other => None,
        }
    }
}

/// Khoá dùng để ký.
pub trait SigningKey: Send + Sync {
    /// Chứng thư của người ký.
    fn certificate(&self) -> &Cert;

    fn algorithm(&self) -> SigAlg;

    /// Ký băm SHA-256 (32 byte). RSA trả về chữ ký PKCS#1 v1.5; ECDSA trả về dạng DER `SEQUENCE { r, s }`.
    fn sign_digest(&self, digest: &[u8; 32]) -> Result<Vec<u8>>;

    /// Tên nhà cung cấp khoá (middleware token, hoặc "khoá phần mềm"), để ghi log và hiển thị.
    fn provider(&self) -> String;
}

/// Đổi chữ ký ECDSA dạng `r ‖ s` (CNG trả về) sang DER `SEQUENCE { INTEGER r, INTEGER s }` (CMS cần).
pub fn ecdsa_raw_to_der(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.is_empty() || !raw.len().is_multiple_of(2) {
        return None;
    }
    let (r, s) = raw.split_at(raw.len() / 2);
    let mut body = Vec::with_capacity(raw.len() + 8);
    for part in [r, s] {
        let trimmed = match part.iter().position(|b| *b != 0) {
            Some(first) => &part[first..],
            None => &part[part.len() - 1..],
        };
        let pad = trimmed[0] & 0x80 != 0;
        let len = trimmed.len() + usize::from(pad);
        body.push(0x02);
        push_len(&mut body, len);
        if pad {
            body.push(0);
        }
        body.extend_from_slice(trimmed);
    }
    let mut out = Vec::with_capacity(body.len() + 4);
    out.push(0x30);
    push_len(&mut out, body.len());
    out.extend_from_slice(&body);
    Some(out)
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    if len < 0x80 {
        out.push(len as u8);
    } else if len <= 0xFF {
        out.extend_from_slice(&[0x81, len as u8]);
    } else {
        out.extend_from_slice(&[0x82, (len >> 8) as u8, len as u8]);
    }
}

/// Khoá phần mềm đọc từ PKCS#8 (RSA hoặc ECDSA P-256). Chỉ để thử: bật bằng feature `dev-keys`, không bao giờ
/// có trong bản phát hành.
#[cfg(any(test, feature = "dev-keys"))]
pub mod software {
    use p256::pkcs8::DecodePrivateKey as _;
    use rsa::Pkcs1v15Sign;
    use smks_core::{ErrorCode, Result, SmksError};

    use super::{SigAlg, SigningKey};
    use crate::cert::Cert;

    enum Inner {
        Rsa(Box<rsa::PrivateKey>),
        P256(p256::ecdsa::SigningKey),
    }

    /// Khoá phần mềm cùng chứng thư của nó.
    pub struct SoftwareKey {
        cert: Cert,
        inner: Inner,
    }

    impl std::fmt::Debug for SoftwareKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SoftwareKey")
                .field("cert", &self.cert.common_name())
                .finish_non_exhaustive()
        }
    }

    fn invalid(message: String) -> SmksError {
        SmksError::new(ErrorCode::SignTokenInvalid, message)
    }

    impl SoftwareKey {
        /// `pkcs8_der`: khoá bí mật PKCS#8 DER (không mã hoá). Khoá phải khớp chứng thư.
        pub fn from_pkcs8_der(cert: Cert, pkcs8_der: &[u8]) -> Result<Self> {
            let inner = match SigAlg::for_certificate(&cert) {
                Some(SigAlg::RsaPkcs1Sha256) => {
                    use rsa::pkcs8::DecodePrivateKey;
                    Inner::Rsa(Box::new(
                        rsa::PrivateKey::from_pkcs8_der(pkcs8_der)
                            .map_err(|e| invalid(format!("Khoá RSA không đọc được: {e}")))?,
                    ))
                }
                Some(SigAlg::EcdsaSha256) => Inner::P256(
                    p256::ecdsa::SigningKey::from_pkcs8_der(pkcs8_der)
                        .map_err(|e| invalid(format!("Khoá ECDSA P-256 không đọc được: {e}")))?,
                ),
                None => {
                    return Err(invalid(
                        "Chứng thư dùng thuật toán khoá không hỗ trợ.".into(),
                    ));
                }
            };
            let key = Self { cert, inner };
            // Kiểm khoá khớp chứng thư bằng một lần ký thử rồi xác minh.
            let probe = [7u8; 32];
            let signature = key.sign_digest(&probe)?;
            crate::verify::verify_digest_signature(
                &key.cert,
                crate::verify::DigestAlg::Sha256,
                &probe,
                key.algorithm().signature_oid(),
                &signature,
            )
            .map_err(|_| invalid("Khoá bí mật không khớp chứng thư.".into()))?;
            Ok(key)
        }

        /// Khoá P-256 kèm chứng thư của nó (PKI thử).
        pub fn from_parts_p256(cert: Cert, key: p256::ecdsa::SigningKey) -> Self {
            Self {
                cert,
                inner: Inner::P256(key),
            }
        }
    }

    impl SigningKey for SoftwareKey {
        fn certificate(&self) -> &Cert {
            &self.cert
        }

        fn algorithm(&self) -> SigAlg {
            match self.inner {
                Inner::Rsa(_) => SigAlg::RsaPkcs1Sha256,
                Inner::P256(_) => SigAlg::EcdsaSha256,
            }
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<Vec<u8>> {
            let fail = |e: String| {
                SmksError::new(
                    ErrorCode::SignFailed,
                    format!("Khoá phần mềm không ký được: {e}"),
                )
            };
            match &self.inner {
                Inner::Rsa(key) => key
                    .sign(Pkcs1v15Sign::new::<sha2::Sha256>(), digest)
                    .map_err(|e| fail(e.to_string())),
                Inner::P256(key) => {
                    use p256::ecdsa::signature::hazmat::PrehashSigner;
                    let signature: p256::ecdsa::Signature =
                        key.sign_prehash(digest).map_err(|e| fail(e.to_string()))?;
                    Ok(signature.to_der().as_bytes().to_vec())
                }
            }
        }

        fn provider(&self) -> String {
            "Khoá phần mềm (chỉ để thử)".into()
        }
    }
}

impl SigAlg {
    /// OID ghi vào `signatureAlgorithm` của SignerInfo: `rsaEncryption` (như .NET) hoặc `ecdsa-with-SHA256`.
    pub fn signature_oid(self) -> &'static str {
        match self {
            Self::RsaPkcs1Sha256 => "1.2.840.113549.1.1.1",
            Self::EcdsaSha256 => "1.2.840.10045.4.3.2",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecdsa_raw_converts_to_der_like_p256() {
        use p256::ecdsa::signature::hazmat::PrehashSigner;
        let key = p256::ecdsa::SigningKey::from_slice(&[0x11; 32]).unwrap();
        for seed in 0..20u8 {
            let digest = [seed; 32];
            let signature: p256::ecdsa::Signature = key.sign_prehash(&digest).unwrap();
            let raw = signature.to_bytes();
            assert_eq!(
                ecdsa_raw_to_der(&raw).unwrap(),
                signature.to_der().as_bytes(),
                "seed {seed}"
            );
        }
        // r có bit cao (cần đệm 0) và s có số 0 đầu (phải bỏ).
        let mut raw = [0u8; 64];
        raw[0] = 0x80;
        raw[63] = 5;
        let der = ecdsa_raw_to_der(&raw).unwrap();
        assert_eq!(&der[..5], &[0x30, 0x26, 0x02, 0x21, 0x00]);
        assert_eq!(&der[der.len() - 3..], &[0x02, 0x01, 0x05]);
        assert!(ecdsa_raw_to_der(&[1, 2, 3]).is_none());
    }
}
