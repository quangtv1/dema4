//! Ký một file PDF: dựng phần nối → CMS PAdES B-T (khoá trên token hoặc khoá thử) → dấu thời gian → điền
//! `/Contents` → kiểm lại chữ ký vừa ký (kế hoạch 6.6, bước 6).

use smks_core::Result;
use smks_crypto::cert::Cert;
use smks_crypto::cms::build_cades;
use smks_crypto::key::SigningKey;
use smks_crypto::tsa::TsaClient;
use smks_pdf::incremental::{SignRequest, prepare};

use crate::verify::{SignatureReport, Verdict, VerifyOptions, check_just_signed, verify_pdf};

/// Kết quả ký một file.
#[derive(Debug, Clone)]
pub struct SignedPdf {
    pub bytes: Vec<u8>,
    /// Kết quả kiểm lại chữ ký vừa ký.
    pub check: SignatureReport,
}

/// Ký `input`. `chain`: CA trung gian nhúng kèm (không gồm gốc). `tsa`: `None` thì không lấy dấu thời gian
/// (thử nghiệm, hoặc lô ký theo giờ máy tính khi không lấy được giờ máy chủ dấu thời gian, TD-54).
pub fn sign_pdf(
    input: &[u8],
    request: &SignRequest,
    key: &dyn SigningKey,
    chain: &[Cert],
    tsa: Option<&TsaClient>,
) -> Result<SignedPdf> {
    let prepared = prepare(input, request)?;
    let digest = prepared.digest();
    let stamp = |signature: &[u8]| -> Result<Vec<u8>> {
        let client = tsa.ok_or_else(|| {
            smks_core::SmksError::new(smks_core::ErrorCode::InternalError, "Thiếu TSA.")
        })?;
        Ok(client.timestamp(signature)?.token_der)
    };
    let cms = build_cades(
        &digest,
        key,
        chain,
        tsa.map(|_| &stamp as &smks_crypto::cms::TimestampFn<'_>),
    )?;
    let bytes = prepared.finish(&cms)?;
    let check = check_just_signed(&bytes, tsa.is_some())?;
    Ok(SignedPdf { bytes, check })
}

/// Kết quả thêm dữ liệu kiểm tra dài hạn.
#[derive(Debug, Clone)]
pub struct LtvOutcome {
    pub bytes: Vec<u8>,
    /// Đã nhúng dữ liệu thu hồi cho chứng thư người ký (PAdES B-LT).
    pub long_term: bool,
    /// Lưu ý cho người dùng khi chưa nhúng đủ (vẫn giữ file B-T).
    pub note: Option<String>,
}

/// Thêm dữ liệu kiểm tra dài hạn (TD-58) cho chữ ký cuối của `signed`: chứng thư, OCSP / CRL của chuỗi người ký và
/// chuỗi TSA, ghi vào `/DSS` bằng một bản cập nhật tăng dần. Chỉ chuỗi về gốc Ban Cơ yếu mới cần (khoá thử thì
/// chỉ có chuỗi TSA). Không lấy được dữ liệu thì giữ nguyên file (B-T) và trả lưu ý; không bao giờ làm hỏng chữ ký.
pub fn add_ltv(signed: &[u8], cache: &smks_crypto::ltv::LtvCache) -> LtvOutcome {
    let keep = |note: Option<String>| LtvOutcome {
        bytes: signed.to_vec(),
        long_term: false,
        note,
    };
    let chains = crate::verify::last_signature_chains(signed);
    if chains.is_empty() {
        return keep(None);
    }
    let data = smks_crypto::ltv::collect(
        &chains.iter().map(|c| c.certs.clone()).collect::<Vec<_>>(),
        cache,
    );
    let signer_ok = chains.iter().find(|c| c.signer).is_some_and(|c| {
        c.trusted
            && !data
                .missing
                .iter()
                .any(|m| m.starts_with(&format!("{}:", c.certs[0].common_name())))
    });
    let trusted_signer = chains.iter().any(|c| c.signer && c.trusted);
    let note = (trusted_signer && !data.missing.is_empty()).then(|| {
        format!(
            "Chưa nhúng đủ dữ liệu kiểm tra dài hạn (không lấy được trạng thái thu hồi: {}). Văn bản vẫn có chữ ký và dấu thời gian hợp lệ.",
            data.missing.join("; ")
        )
    });
    let dss = smks_pdf::incremental::DssData {
        certs: data.certs,
        ocsps: data.ocsps,
        crls: data.crls,
    };
    if dss.ocsps.is_empty() && dss.crls.is_empty() {
        return keep(note);
    }
    let Ok(bytes) = smks_pdf::incremental::append_dss(signed, &dss) else {
        return keep(Some("Không ghi được dữ liệu kiểm tra dài hạn vào file; văn bản giữ chữ ký và dấu thời gian.".into()));
    };
    // Kiểm lại: chữ ký vừa ký vẫn khớp, phần thêm chỉ là dữ liệu kiểm tra.
    let fine = verify_pdf(&bytes, VerifyOptions::default()).is_ok_and(|r| {
        r.signatures.last().is_some_and(|s| {
            !matches!(
                s.verdict,
                Verdict::InvalidSignature | Verdict::ModifiedAfterSigning
            ) && s.changes == smks_pdf::revdiff::ChangeLevel::Signatures
        })
    });
    if !fine {
        return keep(Some("Kiểm lại sau khi thêm dữ liệu kiểm tra dài hạn không đạt nên giữ bản chỉ có dấu thời gian.".into()));
    }
    LtvOutcome {
        bytes,
        long_term: signer_ok,
        note,
    }
}
