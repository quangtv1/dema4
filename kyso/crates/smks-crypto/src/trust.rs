//! Chứng thư CA của Ban Cơ yếu Chính phủ, nhúng trong binary và ghim bằng SHA-256.
//!
//! Bản .NET 1.4.0 đọc các file `.crt` cạnh file chạy (`SignatureVerifier.LoadPinnedCert`); thiếu
//! file nào thì mọi chữ ký thuộc nhánh đó bị coi là không tin cậy. Ở đây chứng thư được nhúng
//! lúc biên dịch nên không thể thiếu, và vẫn được kiểm tra lại với giá trị ghim trước khi dùng.
//!
//! Giá trị ghim là SHA-256 của chứng thư dạng DER, giống `GetCertHashString(SHA256)` của bản cũ.

use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};
use smks_core::consts::ca;
use smks_core::{ErrorCode, Result, SmksError};
use x509_cert::Certificate;
use x509_cert::der::Decode;

const PEM_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const PEM_END: &str = "-----END CERTIFICATE-----";

/// Giải mã khối `CERTIFICATE` đầu tiên trong văn bản PEM.
///
/// Cố ý dễ tính về cách xuống dòng: `cpg2.crt` của Ban Cơ yếu ngắt dòng Base64 ở 76 ký tự,
/// bộ đọc PEM chặt theo RFC 7468 (tối đa 64 ký tự) sẽ từ chối. Bản cũ dùng
/// `X509Certificate2.CreateFromPem` cũng chấp nhận dạng này. Tính toàn vẹn không dựa vào bước
/// này mà dựa vào phép so SHA-256 của DER với giá trị ghim.
fn decode_pem_certificate(pem: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let text = std::str::from_utf8(pem).map_err(|_| "nội dung không phải văn bản".to_owned())?;
    let start = text.find(PEM_BEGIN).ok_or("thiếu dòng BEGIN CERTIFICATE")? + PEM_BEGIN.len();
    let length = text[start..]
        .find(PEM_END)
        .ok_or("thiếu dòng END CERTIFICATE")?;
    let body: String = text[start..start + length]
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    Base64::decode_vec(&body).map_err(|e| format!("Base64 không hợp lệ: {e}"))
}

/// Vai trò của một chứng thư CA trong chuỗi tin cậy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaRole {
    /// Gốc của nhánh G1.
    RootG1,
    /// CA cấp dưới của nhánh G1.
    SubCaG1,
    /// Gốc của nhánh G2, cũng là gốc của chuỗi TSA.
    RootG2,
    /// CA trung gian của nhánh G2.
    IntermediateG2,
}

impl CaRole {
    /// Tên gọi hiển thị.
    pub const fn label(self) -> &'static str {
        match self {
            CaRole::RootG1 => "Root G1",
            CaRole::SubCaG1 => "Sub CA G1",
            CaRole::RootG2 => "Root G2",
            CaRole::IntermediateG2 => "CA trung gian G2",
        }
    }
}

/// Một chứng thư CA nhúng trong binary kèm giá trị ghim.
#[derive(Clone, Copy)]
pub struct PinnedCa {
    role: CaRole,
    file_name: &'static str,
    sha256_hex: &'static str,
    pem: &'static [u8],
}

impl std::fmt::Debug for PinnedCa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinnedCa")
            .field("role", &self.role)
            .field("file_name", &self.file_name)
            .field("sha256_hex", &self.sha256_hex)
            .finish_non_exhaustive()
    }
}

/// Năm chứng thư CA được ghim, theo `SignatureConstants.cs` của bản cũ.
pub static PINNED_CAS: [PinnedCa; 5] = [
    PinnedCa {
        role: CaRole::RootG1,
        file_name: ca::ROOT_G1_FILE,
        sha256_hex: ca::ROOT_G1_SHA256,
        pem: include_bytes!("../../../certs/rootca.crt"),
    },
    PinnedCa {
        role: CaRole::SubCaG1,
        file_name: ca::SUB_CA_G1_FILE,
        sha256_hex: ca::SUB_CA_G1_SHA256,
        pem: include_bytes!("../../../certs/cp.crt"),
    },
    PinnedCa {
        role: CaRole::RootG2,
        file_name: ca::ROOT_G2_FILE,
        sha256_hex: ca::ROOT_G2_SHA256,
        pem: include_bytes!("../../../certs/rootcag2.crt"),
    },
    PinnedCa {
        role: CaRole::IntermediateG2,
        file_name: ca::DCS_CA_G2_FILE,
        sha256_hex: ca::DCS_CA_G2_SHA256,
        pem: include_bytes!("../../../certs/dcscag2.crt"),
    },
    PinnedCa {
        role: CaRole::IntermediateG2,
        file_name: ca::CP_G2_FILE,
        sha256_hex: ca::CP_G2_SHA256,
        pem: include_bytes!("../../../certs/cpg2.crt"),
    },
];

/// Thông tin đọc được từ một chứng thư CA đã qua kiểm tra ghim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaInfo {
    pub role: CaRole,
    pub file_name: &'static str,
    pub subject: String,
    pub issuer: String,
    /// Số serial dạng hex viết hoa, big-endian.
    pub serial_hex: String,
    /// Thời điểm bắt đầu và hết hiệu lực, giờ UTC.
    pub not_before: String,
    pub not_after: String,
    pub sha256_hex: String,
}

impl PinnedCa {
    pub const fn role(&self) -> CaRole {
        self.role
    }

    pub const fn file_name(&self) -> &'static str {
        self.file_name
    }

    /// Giá trị SHA-256 được ghim (hex viết hoa).
    pub const fn pinned_sha256_hex(&self) -> &'static str {
        self.sha256_hex
    }

    /// Giải mã PEM và trả về DER, chỉ khi SHA-256 của DER khớp giá trị ghim.
    pub fn verified_der(&self) -> Result<Vec<u8>> {
        let der = decode_pem_certificate(self.pem)
            .map_err(|detail| self.error(format!("không đọc được nội dung PEM ({detail})")))?;

        let actual = sha256_hex(&der);
        if !actual.eq_ignore_ascii_case(self.sha256_hex) {
            return Err(self.error(format!(
                "SHA-256 là {actual}, khác giá trị ghim {}",
                self.sha256_hex
            )));
        }
        Ok(der)
    }

    /// Chứng thư đã qua kiểm tra ghim.
    pub fn certificate(&self) -> Result<Certificate> {
        let der = self.verified_der()?;
        Certificate::from_der(&der)
            .map_err(|e| self.error(format!("không phải chứng thư X.509 hợp lệ ({e})")))
    }

    /// Thông tin tóm tắt của chứng thư đã qua kiểm tra ghim.
    pub fn info(&self) -> Result<CaInfo> {
        let der = self.verified_der()?;
        let cert = Certificate::from_der(&der)
            .map_err(|e| self.error(format!("không phải chứng thư X.509 hợp lệ ({e})")))?;
        let tbs = cert.tbs_certificate();
        Ok(CaInfo {
            role: self.role,
            file_name: self.file_name,
            subject: tbs.subject().to_string(),
            issuer: tbs.issuer().to_string(),
            serial_hex: hex_upper(tbs.serial_number().as_bytes()),
            not_before: tbs.validity().not_before.to_string(),
            not_after: tbs.validity().not_after.to_string(),
            sha256_hex: sha256_hex(&der),
        })
    }

    fn error(&self, detail: String) -> SmksError {
        SmksError::new(
            ErrorCode::InternalError,
            format!(
                "Chứng thư CA {} ({}) nhúng trong ứng dụng không dùng được: {detail}. \
                 Bản cài có thể đã bị hỏng hoặc bị sửa, vui lòng cài lại từ nguồn chính thức.",
                self.file_name,
                self.role.label()
            ),
        )
    }
}

/// Kiểm tra toàn bộ chứng thư CA nhúng khớp giá trị ghim. Gọi một lần lúc khởi động.
pub fn verify_pinned_cas() -> Result<()> {
    PINNED_CAS
        .iter()
        .try_for_each(|ca| ca.verified_der().map(|_| ()))
}

/// SHA-256 dạng hex viết hoa.
pub fn sha256_hex(data: &[u8]) -> String {
    hex_upper(Sha256::digest(data).as_slice())
}

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_embedded_cas_match_their_pins() {
        verify_pinned_cas().unwrap();
        for ca in &PINNED_CAS {
            assert_eq!(
                sha256_hex(&ca.verified_der().unwrap()),
                ca.pinned_sha256_hex()
            );
        }
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // FIPS 180-2, chuỗi "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
    }

    #[test]
    fn wrong_pin_is_rejected_with_internal_error() {
        let tampered = PinnedCa {
            sha256_hex: ca::ROOT_G2_SHA256,
            ..PINNED_CAS[0]
        };
        let err = tampered.verified_der().unwrap_err();
        assert_eq!(err.code, ErrorCode::InternalError);
        assert!(err.message.contains("khác giá trị ghim"), "{}", err.message);
        assert!(err.message.contains(ca::ROOT_G1_FILE), "{}", err.message);
    }

    #[test]
    fn pem_with_lines_longer_than_64_columns_is_accepted() {
        // cpg2.crt ngắt dòng ở 76 ký tự; đây là lý do không dùng bộ đọc PEM chặt theo RFC 7468.
        let cpg2 = PINNED_CAS
            .iter()
            .find(|c| c.file_name() == ca::CP_G2_FILE)
            .unwrap();
        let longest = cpg2
            .pem
            .split(|b| *b == b'\n')
            .map(<[u8]>::len)
            .max()
            .unwrap();
        assert!(
            longest > 64,
            "cpg2.crt không còn dòng dài hơn 64 ký tự: {longest}"
        );
        cpg2.verified_der().unwrap();
    }

    #[test]
    fn non_certificate_pem_is_rejected() {
        let not_pem = PinnedCa {
            pem: b"khong phai PEM",
            ..PINNED_CAS[0]
        };
        assert_eq!(
            not_pem.verified_der().unwrap_err().code,
            ErrorCode::InternalError
        );
    }

    #[test]
    fn hierarchy_is_two_roots_each_issuing_its_own_cas() {
        let info = |file: &str| {
            PINNED_CAS
                .iter()
                .find(|c| c.file_name() == file)
                .unwrap()
                .info()
                .unwrap()
        };
        let root_g1 = info(ca::ROOT_G1_FILE);
        let root_g2 = info(ca::ROOT_G2_FILE);
        assert_eq!(root_g1.issuer, root_g1.subject);
        assert_eq!(root_g2.issuer, root_g2.subject);
        assert_ne!(root_g1.subject, root_g2.subject);

        assert_eq!(info(ca::SUB_CA_G1_FILE).issuer, root_g1.subject);
        // Cả hai CA trung gian G2 đều do Root G2 cấp trực tiếp (không nối tiếp nhau).
        assert_eq!(info(ca::DCS_CA_G2_FILE).issuer, root_g2.subject);
        assert_eq!(info(ca::CP_G2_FILE).issuer, root_g2.subject);
    }

    #[test]
    fn every_ca_parses_as_x509_and_roles_are_complete() {
        for ca in &PINNED_CAS {
            let info = ca.info().unwrap();
            assert!(!info.subject.is_empty(), "{}", ca.file_name());
            assert!(!info.serial_hex.is_empty(), "{}", ca.file_name());
        }
        let count = |role| PINNED_CAS.iter().filter(|c| c.role() == role).count();
        assert_eq!(count(CaRole::RootG1), 1);
        assert_eq!(count(CaRole::SubCaG1), 1);
        assert_eq!(count(CaRole::RootG2), 1);
        assert_eq!(count(CaRole::IntermediateG2), 2);
    }
}
