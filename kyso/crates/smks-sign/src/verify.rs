//! Xác minh chữ ký trong PDF (port `SignatureVerifier.cs`, kế hoạch mục 9), theo các điều kiện của Nghị định
//! 68/2024/NĐ-CP, Điều 30–32 (rà soát 24/11/2026, `docs/implementation/KIEM_TRA_CHU_KY.md`).
//!
//! Mỗi chữ ký có một kết luận. Kết luận chung là kết luận **nặng nhất**:
//! hợp lệ < hợp lệ nhưng chưa kiểm được thu hồi < lệch giờ < không có dấu thời gian < TSA không tin cậy
//! < không xác định (thuật toán chưa hỗ trợ) < người ký không tin cậy < sửa sau khi ký < chữ ký sai.
//!
//! Năm điều kiện của Điều 30 được báo riêng trong `checks`:
//! 1. **Toàn vẹn:** chữ ký khớp nội dung theo `/ByteRange`. Sau chữ ký, mỗi bản cập nhật tăng dần chỉ được
//!    thêm chữ ký / dấu thời gian / DSS (điền biểu mẫu, chú thích thì cảnh báo; chữ ký chứng nhận DocMDP thì
//!    theo quyền `/P`). Có byte lạ không phải bản cập nhật nối sau chữ ký cuối thì chữ ký cuối bị coi là sai.
//! 2. **Người ký khớp chứng thư:** chứng thư nhúng khớp `signingCertificate(V2)` nếu có.
//! 3. **Thời gian ký:** dấu thời gian RFC 3161 của TSA Ban Cơ yếu (chuỗi về Root G2); `signing-time` tự khai lệch
//!    đúng +7 giờ là ghi nhầm giờ địa phương (tương thích ngược), đứng sau dấu thời gian quá 60 giây là "lệch".
//! 4. **Chứng thư còn hiệu lực tại thời điểm ký:** hạn dùng, KeyUsage, thu hồi (CRL/OCSP). Bị thu hồi **sau**
//!    thời điểm trên dấu thời gian thì chữ ký vẫn hợp lệ.
//! 5. **Đường dẫn chứng thực:** chuỗi về Root G1 hoặc Root G2 đã ghim.
//!
//! Chữ ký không có dấu thời gian vẫn kiểm đủ người ký, tại giờ tự khai (không chứng minh được).
//! Dấu thời gian tài liệu (`ETSI.RFC3161`) được kiểm như một token RFC 3161 phủ `/ByteRange`.

use cms::signed_data::SignerInfo;
use der::Encode;
use serde::Serialize;
use smks_core::consts::{oid, verify as vk};
use smks_core::{ErrorCode, Result, SmksError};
use smks_crypto::cert::Cert;
use smks_crypto::verify::{
    self as cv, TimestampInfo, parse_signed_data, signed_attribute, verify_signer_info,
    verify_timestamp_token, verify_timestamp_token_parts,
};
use smks_pdf::geometry::Placement;
use smks_pdf::revdiff::{self, ChangeLevel, ChangeNote};
use smks_pdf::sigscan::{self, SignatureField};

/// Kết luận cho một chữ ký, từ nhẹ đến nặng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    NotApplicable,
    /// File không có chữ ký nào (chỉ dùng cho kết luận chung).
    Unsigned,
    ValidTrusted,
    /// Đạt mọi điều kiện nhưng không tải được CRL/OCSP để kiểm thu hồi.
    RevocationUnknown,
    TimeMismatch,
    NoTimestamp,
    UntrustedTsa,
    /// Thuật toán chưa hỗ trợ: không kết luận được đúng hay sai.
    Unsupported,
    UntrustedSigner,
    /// Chữ ký khớp bản đã ký, nhưng sau đó nội dung văn bản bị sửa.
    ModifiedAfterSigning,
    InvalidSignature,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotApplicable => "Không áp dụng",
            Self::Unsigned => "Văn bản chưa có chữ ký số",
            Self::ValidTrusted => "Hợp lệ",
            Self::RevocationUnknown => "Hợp lệ, chưa kiểm tra được thu hồi",
            Self::TimeMismatch => "Thời gian ký bất thường",
            Self::NoTimestamp => "Không có dấu thời gian",
            Self::UntrustedTsa => "Dấu thời gian không tin cậy",
            Self::Unsupported => "Không xác định (thuật toán chưa hỗ trợ)",
            Self::UntrustedSigner => "Người ký không thuộc CA Ban Cơ yếu",
            Self::ModifiedAfterSigning => "Văn bản bị sửa sau khi ký",
            Self::InvalidSignature => "Chữ ký không hợp lệ",
        }
    }
}

/// Tình trạng thu hồi của chứng thư người ký.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RevocationStatus {
    NotChecked,
    Good,
    /// Bị thu hồi sau thời điểm ký (theo dấu thời gian): chữ ký vẫn hợp lệ.
    RevokedAfterSigning,
    Revoked,
    Unknown,
}

/// Loại mục ký.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureKind {
    Signature,
    /// Dấu thời gian tài liệu (`/DocTimeStamp`): không có người ký, chỉ có TSA.
    DocTimestamp,
}

/// Kết quả một điều kiện.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckState {
    Ok,
    Warn,
    Fail,
    /// Không kiểm được (thiếu dữ liệu, thuật toán chưa hỗ trợ, bước trước đã hỏng).
    Unknown,
}

/// Một điều kiện của Điều 30 cho một chữ ký.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// `integrity`, `signer`, `time`, `certificate`, `chain`.
    pub id: &'static str,
    pub label: &'static str,
    pub state: CheckState,
    pub detail: String,
}

/// Một chứng thư trong chuỗi (từ người ký tới gốc).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainItem {
    pub name: String,
    pub serial: String,
    pub valid_from: String,
    pub valid_to: String,
}

/// Kết quả một chữ ký.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureReport {
    pub field_name: String,
    pub kind: SignatureKind,
    pub verdict: Verdict,
    pub reason: Option<String>,
    pub signed_by: Option<String>,
    /// Đơn vị (OU, O) trong chứng thư người ký.
    pub organization: Option<String>,
    pub signer_serial: Option<String>,
    pub issuer: Option<String>,
    /// Hiệu lực chứng thư người ký, giờ Việt Nam.
    pub cert_valid_from: Option<String>,
    pub cert_valid_to: Option<String>,
    /// "Root G1" / "Root G2" nếu người ký tin cậy.
    pub signer_root: Option<String>,
    pub chain: Vec<ChainItem>,
    /// Thời điểm TSA đóng dấu, giờ Việt Nam `dd/MM/yyyy HH:mm:ss`.
    pub trusted_time: Option<String>,
    /// Giây Unix của dấu thời gian.
    pub trusted_unix: Option<i64>,
    /// `signing-time` trong CMS (phần mềm khác), giờ Việt Nam.
    pub signing_time: Option<String>,
    /// `/M` của từ điển chữ ký (chuỗi gốc).
    pub pdf_time: Option<String>,
    /// Giờ ký hiển thị: giờ dấu thời gian, không có thì giờ tự khai.
    pub display_time: Option<String>,
    pub timestamp_authority: Option<String>,
    pub has_timestamp: bool,
    pub revocation: RevocationStatus,
    /// Tình trạng thu hồi lấy từ dữ liệu nhúng trong file (DSS), không phải hỏi qua mạng.
    pub revocation_embedded: bool,
    /// Ngày thu hồi (nếu bị thu hồi), giờ Việt Nam.
    pub revoked_at: Option<String>,
    /// File có dữ liệu kiểm tra dài hạn (OCSP / CRL trong DSS) cho chứng thư người ký (PAdES B-LT).
    pub long_term: bool,
    pub page: Option<u32>,
    pub visible: bool,
    pub placement: Option<Placement>,
    pub covers_whole_file: bool,
    pub sub_filter: String,
    pub digest_algorithm: Option<String>,
    /// `/Reason`, `/Location`, `/Name` của từ điển chữ ký.
    pub sign_reason: Option<String>,
    pub location: Option<String>,
    /// Quyền DocMDP nếu là chữ ký chứng nhận.
    pub docmdp: Option<u8>,
    /// Mức thay đổi sau chữ ký này và mô tả.
    pub changes: ChangeLevel,
    pub change_notes: Vec<ChangeNote>,
    pub checks: Vec<Check>,
    /// Cảnh báo không làm đổi kết luận (SHA-1, chú thích thêm sau khi ký, thu hồi sau khi ký…).
    pub warnings: Vec<String>,
}

/// Kết quả cả file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfReport {
    pub verdict: Verdict,
    pub reason: Option<String>,
    pub signatures: Vec<SignatureReport>,
    /// Số mục (chữ ký và dấu thời gian tài liệu) kết luận "Hợp lệ".
    pub valid_count: usize,
    /// Đã kiểm thu hồi (CRL/OCSP).
    pub revocation_checked: bool,
    /// Dữ liệu kiểm tra dài hạn trong file (`/DSS`), nếu có.
    pub dss: Option<DssSummary>,
}

/// Số mục trong `/DSS`.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DssSummary {
    pub certs: usize,
    pub ocsps: usize,
    pub crls: usize,
}

/// Tuỳ chọn xác minh.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyOptions {
    /// Kiểm thu hồi qua CRL/OCSP (cần mạng).
    pub check_revocation: bool,
}

fn vn(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = smks_pdf::object::vietnam_parts(unix);
    format!("{d:02}/{mo:02}/{y:04} {h:02}:{mi:02}:{s:02}")
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Số ngày từ 1970-01-01 (lịch Gregory, thuật toán của H. Hinnant).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Ngày giờ PDF `D:YYYYMMDDHHmmSS+HH'mm'` → giây Unix. Thiếu phần sau thì lấy giá trị mặc định của chuẩn.
pub fn parse_pdf_date(text: &str) -> Option<i64> {
    let t = text.trim().strip_prefix("D:").unwrap_or(text.trim());
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return None;
    }
    let part = |from: usize, len: usize, default: i64| -> i64 {
        digits
            .get(from..from + len)
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    };
    let (y, mo, d) = (part(0, 4, 1970), part(4, 2, 1), part(6, 2, 1));
    let (h, mi, s) = (part(8, 2, 0), part(10, 2, 0), part(12, 2, 0));
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let rest = &t[digits.len()..];
    let offset = match rest.chars().next() {
        Some(sign @ ('+' | '-')) => {
            let nums: Vec<i64> = rest[1..]
                .split(|c: char| !c.is_ascii_digit())
                .filter(|p| !p.is_empty())
                .filter_map(|p| p.parse().ok())
                .collect();
            let secs =
                nums.first().copied().unwrap_or(0) * 3600 + nums.get(1).copied().unwrap_or(0) * 60;
            if sign == '+' { secs } else { -secs }
        }
        _ => 0,
    };
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s - offset)
}

/// Xác minh mọi chữ ký của file PDF.
pub fn verify_pdf(bytes: &[u8], options: VerifyOptions) -> Result<PdfReport> {
    let fields = sigscan::scan(bytes)?;
    if fields.is_empty() {
        return Ok(PdfReport {
            verdict: Verdict::Unsigned,
            reason: Some("Văn bản không có chữ ký số nào.".into()),
            signatures: Vec::new(),
            valid_count: 0,
            revocation_checked: options.check_revocation,
            dss: None,
        });
    }
    let ctx = FileContext::new(bytes, &fields);
    let signatures: Vec<SignatureReport> = fields
        .iter()
        .map(|f| verify_field(bytes, f, &ctx, options))
        .collect();
    let worst = signatures
        .iter()
        .max_by_key(|s| s.verdict)
        .map(|s| (s.verdict, s.reason.clone()));
    let (verdict, reason) = worst.unwrap_or((Verdict::Unsigned, None));
    let valid_count = signatures
        .iter()
        .filter(|s| s.verdict == Verdict::ValidTrusted)
        .count();
    Ok(PdfReport {
        verdict,
        reason,
        signatures,
        valid_count,
        revocation_checked: options.check_revocation,
        dss: ctx.dss.as_ref().map(|d| DssSummary {
            certs: d.certs.len(),
            ocsps: d.ocsps.len(),
            crls: d.crls.len(),
        }),
    })
}

/// Thông tin chung của file dùng cho từng chữ ký.
struct FileContext {
    /// Cuối bản cập nhật của chữ ký cuối cùng.
    outermost_end: usize,
    /// Sau chữ ký cuối là bản cập nhật tăng dần đúng dạng (có `startxref` mới), không phải byte lạ.
    tail_is_update: bool,
    /// Sau chữ ký cuối chỉ có khoảng trắng / byte 0.
    tail_is_blank: bool,
    /// (cuối bản, là dấu thời gian tài liệu) của mọi mục ký.
    ends: Vec<(usize, bool)>,
    /// Dữ liệu kiểm tra dài hạn (`/DSS`) của file.
    dss: Option<smks_pdf::incremental::DssData>,
}

impl FileContext {
    fn new(bytes: &[u8], fields: &[SignatureField]) -> Self {
        let ends: Vec<(usize, bool)> = fields
            .iter()
            .map(|f| {
                (
                    f.byte_range[2].saturating_add(f.byte_range[3]),
                    f.doc_timestamp,
                )
            })
            .collect();
        let outermost_end = ends.iter().map(|e| e.0).max().unwrap_or(0);
        let tail = bytes.get(outermost_end..).unwrap_or_default();
        let tail_is_blank = tail.iter().all(|b| b.is_ascii_whitespace() || *b == 0);
        let tail_is_update = !tail_is_blank
            && smks_pdf::reader::last_startxref(bytes).is_ok_and(|x| x >= outermost_end)
            && tail
                .windows(5)
                .rposition(|w| w == b"%%EOF")
                .is_some_and(|at| {
                    tail[at + 5..]
                        .iter()
                        .all(|b| b.is_ascii_whitespace() || *b == 0)
                });
        Self {
            outermost_end,
            tail_is_update,
            tail_is_blank,
            ends,
            dss: smks_pdf::incremental::read_dss(bytes).ok().flatten(),
        }
    }
}

/// Bộ gom kết luận: giữ kết luận nặng nhất và lý do của nó.
struct Builder {
    r: SignatureReport,
}

impl Builder {
    fn new(field: &SignatureField) -> Self {
        let kind = if field.doc_timestamp {
            SignatureKind::DocTimestamp
        } else {
            SignatureKind::Signature
        };
        Self {
            r: SignatureReport {
                field_name: field.field_name.clone(),
                kind,
                verdict: Verdict::ValidTrusted,
                reason: None,
                signed_by: field.name.clone().filter(|n| !n.trim().is_empty()),
                organization: None,
                signer_serial: None,
                issuer: None,
                cert_valid_from: None,
                cert_valid_to: None,
                signer_root: None,
                chain: Vec::new(),
                trusted_time: None,
                trusted_unix: None,
                signing_time: None,
                pdf_time: field.signing_time.clone(),
                display_time: None,
                timestamp_authority: None,
                has_timestamp: false,
                revocation: RevocationStatus::NotChecked,
                revocation_embedded: false,
                revoked_at: None,
                long_term: false,
                page: field.page,
                visible: field.visible,
                placement: field.placement,
                covers_whole_file: field.covers_whole_file,
                sub_filter: field.sub_filter.clone(),
                digest_algorithm: None,
                sign_reason: field.reason.clone().filter(|r| !r.trim().is_empty()),
                location: field.location.clone().filter(|l| !l.trim().is_empty()),
                docmdp: field.docmdp,
                changes: ChangeLevel::None,
                change_notes: Vec::new(),
                checks: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }

    fn flag(&mut self, verdict: Verdict, reason: impl Into<String>) {
        if verdict > self.r.verdict || self.r.reason.is_none() && verdict == self.r.verdict {
            self.r.verdict = verdict;
            self.r.reason = Some(reason.into());
        }
    }

    fn warn(&mut self, text: impl Into<String>) {
        let text = text.into();
        if !self.r.warnings.contains(&text) {
            self.r.warnings.push(text);
        }
    }

    fn check(
        &mut self,
        id: &'static str,
        label: &'static str,
        state: CheckState,
        detail: impl Into<String>,
    ) {
        self.r.checks.push(Check {
            id,
            label,
            state,
            detail: detail.into(),
        });
    }

    fn done(mut self) -> SignatureReport {
        if self.r.verdict == Verdict::ValidTrusted {
            self.r.reason = None;
        }
        const ORDER: [&str; 5] = ["integrity", "signer", "time", "certificate", "chain"];
        self.r
            .checks
            .sort_by_key(|c| ORDER.iter().position(|o| *o == c.id).unwrap_or(ORDER.len()));
        self.r.display_time = self
            .r
            .trusted_time
            .clone()
            .or_else(|| self.r.signing_time.clone())
            .or_else(|| self.r.pdf_time.as_deref().and_then(parse_pdf_date).map(vn));
        self.r
    }
}

const L_INTEGRITY: &str = "Văn bản toàn vẹn";
const L_SIGNER: &str = "Thông tin người ký khớp chứng thư";
const L_TIME: &str = "Thời gian ký";
const L_CERT: &str = "Chứng thư còn hiệu lực tại thời điểm ký";
const L_CHAIN: &str = "Chứng thư thuộc CA Ban Cơ yếu (Root G1 / G2)";

fn verify_field(
    bytes: &[u8],
    field: &SignatureField,
    ctx: &FileContext,
    options: VerifyOptions,
) -> SignatureReport {
    let mut b = Builder::new(field);
    let Some(parts) = field.signed_parts(bytes) else {
        b.flag(
            Verdict::InvalidSignature,
            "Vùng ký (/ByteRange) nằm ngoài file.",
        );
        b.check(
            "integrity",
            L_INTEGRITY,
            CheckState::Fail,
            "Vùng ký (/ByteRange) nằm ngoài file.",
        );
        return b.done();
    };
    let end = field.byte_range[2] + field.byte_range[3];
    if field.doc_timestamp {
        verify_doc_timestamp(&mut b, field, &parts, options);
    } else {
        verify_signature(&mut b, field, &parts, options, ctx.dss.as_ref());
    }
    // Phần sau chữ ký: sửa đổi, quyền DocMDP, byte lạ nối thêm.
    integrity_after(&mut b, bytes, field, end, ctx);
    b.done()
}

/// Thêm điều kiện "toàn vẹn" (đặt đầu danh sách) theo phần sau chữ ký.
fn integrity_after(
    b: &mut Builder,
    bytes: &[u8],
    field: &SignatureField,
    end: usize,
    ctx: &FileContext,
) {
    // Chữ ký không khớp nội dung đã ghi điều kiện "toàn vẹn" ở bước trước.
    if b.r.checks.iter().any(|c| c.id == "integrity") {
        return;
    }
    let mut state = CheckState::Ok;
    let mut detail = "Chữ ký khớp nội dung; không có thay đổi sau khi ký.".to_owned();
    if !field.range_ok {
        b.flag(
            Verdict::InvalidSignature,
            "Vùng ký (/ByteRange) không hợp lệ — không phủ từ đầu file hoặc có byte lạ trong khoảng hở.",
        );
        insert_check(
            b,
            "integrity",
            L_INTEGRITY,
            CheckState::Fail,
            "Vùng ký (/ByteRange) không hợp lệ.",
        );
        return;
    }
    let outermost = end == ctx.outermost_end;
    if outermost && end != bytes.len() && !ctx.tail_is_update {
        if ctx.tail_is_blank {
            b.warn("Sau chữ ký cuối có thêm khoảng trắng (không ảnh hưởng nội dung).");
        } else {
            b.flag(
                Verdict::InvalidSignature,
                "File có byte nằm ngoài mọi vùng ký (nội dung được nối thêm sau khi ký).",
            );
            insert_check(
                b,
                "integrity",
                L_INTEGRITY,
                CheckState::Fail,
                "Có dữ liệu lạ nối thêm sau chữ ký cuối.",
            );
            return;
        }
    }
    if end < bytes.len() && !(outermost && ctx.tail_is_blank) {
        match revdiff::changes_after(bytes, end) {
            Ok(ch) => {
                b.r.changes = ch.level;
                b.r.change_notes = ch.notes.clone();
                let allowed = match field.docmdp {
                    Some(1) => ChangeLevel::Signatures,
                    Some(2) => ChangeLevel::FormFill,
                    Some(_) => ChangeLevel::Annotations,
                    None => ChangeLevel::Annotations,
                };
                // DocMDP P=1: không được ký thêm (dấu thời gian tài liệu / DSS vẫn được).
                let later_signature = ctx.ends.iter().any(|(e, ts)| *e > end && !ts);
                let p1_violation = field.docmdp == Some(1) && later_signature;
                if ch.level > allowed || p1_violation {
                    let why = if ch.level == ChangeLevel::Other {
                        format!(
                            "Văn bản bị sửa sau khi ký: {}",
                            ch.text_at(ChangeLevel::Other)
                        )
                    } else {
                        let over: Vec<&str> = ch
                            .notes
                            .iter()
                            .filter(|n| {
                                n.level > allowed
                                    || p1_violation && n.level == ChangeLevel::Signatures
                            })
                            .map(|n| n.text.as_str())
                            .collect();
                        format!(
                            "Thay đổi sau khi ký vượt quyền của chữ ký chứng nhận (DocMDP P={}): {}",
                            field.docmdp.unwrap_or(2),
                            over.join(" ")
                        )
                    };
                    b.flag(Verdict::ModifiedAfterSigning, why.clone());
                    state = CheckState::Fail;
                    detail = why;
                } else {
                    match ch.level {
                        ChangeLevel::None | ChangeLevel::Signatures => {
                            detail = "Chữ ký khớp nội dung; sau đó chỉ ký thêm / bổ sung dữ liệu kiểm tra.".into();
                        }
                        ChangeLevel::FormFill | ChangeLevel::Annotations => {
                            let notes: Vec<&str> = ch
                                .notes
                                .iter()
                                .filter(|n| n.level >= ChangeLevel::FormFill)
                                .map(|n| n.text.as_str())
                                .collect();
                            for n in &notes {
                                b.warn(format!("Sau khi ký: {n}"));
                            }
                            state = CheckState::Warn;
                            detail = format!(
                                "Chữ ký khớp nội dung; sau khi ký có thay đổi được phép: {}",
                                notes.join(" ")
                            );
                        }
                        ChangeLevel::Other => {}
                    }
                }
            }
            Err(e) => {
                b.warn(format!(
                    "Không so được với các bản cập nhật sau chữ ký: {}",
                    e.message
                ));
                state = CheckState::Unknown;
                detail = "Chữ ký khớp bản đã ký; không đọc được phần cập nhật sau đó.".into();
            }
        }
    }
    insert_check(b, "integrity", L_INTEGRITY, state, detail);
}

fn insert_check(
    b: &mut Builder,
    id: &'static str,
    label: &'static str,
    state: CheckState,
    detail: impl Into<String>,
) {
    b.r.checks.insert(
        0,
        Check {
            id,
            label,
            state,
            detail: detail.into(),
        },
    );
}

fn verify_signature(
    b: &mut Builder,
    field: &SignatureField,
    parts: &[&[u8]; 2],
    options: VerifyOptions,
    dss: Option<&smks_pdf::incremental::DssData>,
) {
    let sd = match parse_signed_data(&field.contents) {
        Ok(sd) => sd,
        Err(e) => {
            let why = format!(
                "Khối chữ ký trong file đã hỏng, không giải mã được (CMS lỗi: {}).",
                e.message
            );
            b.flag(Verdict::InvalidSignature, why.clone());
            b.check("integrity", L_INTEGRITY, CheckState::Fail, why);
            return;
        }
    };
    let mut infos = sd.signer_infos.0.iter();
    let Some(info) = infos.next() else {
        let why = "Khối chữ ký trong file không chứa người ký nào (SignerInfos rỗng).";
        b.flag(Verdict::InvalidSignature, why);
        b.check("integrity", L_INTEGRITY, CheckState::Fail, why);
        return;
    };
    if infos.next().is_some() {
        b.warn("Khối chữ ký có nhiều người ký; chỉ kiểm người ký đầu tiên.");
    }
    let signer = cv::find_signer_certificate(&sd, info);
    if let Some(cert) = &signer {
        b.r.signed_by = Some(cert.common_name());
        b.r.organization = Some(cert.organization()).filter(|o| !o.is_empty());
        b.r.signer_serial = Some(cert.serial_hex());
        b.r.issuer = Some(cert.issuer_common_name());
        let (from, to) = cert.validity();
        b.r.cert_valid_from = Some(vn(from));
        b.r.cert_valid_to = Some(vn(to));
    }
    // 1. Toàn vẹn + 2. người ký khớp chứng thư.
    let ok = match verify_signer_info(&sd, info, parts) {
        Ok(check) => {
            b.r.digest_algorithm = Some(check.digest_alg.name().to_owned());
            if check.digest_alg.is_weak() {
                b.warn(format!(
                    "Chữ ký dùng thuật toán băm {} đã yếu; nên ký lại bằng SHA-256.",
                    check.digest_alg.name()
                ));
            }
            if check.cert_bound {
                b.check(
                    "signer",
                    L_SIGNER,
                    CheckState::Ok,
                    "Chứng thư nhúng đúng là chứng thư đã ký (signingCertificate khớp).",
                );
            } else {
                b.check(
                    "signer",
                    L_SIGNER,
                    CheckState::Warn,
                    "Chữ ký khớp khoá của chứng thư nhúng; chữ ký không ghi băm chứng thư (định dạng cũ) nên không đối chiếu thêm được.",
                );
            }
            true
        }
        Err(e) if e.code == ErrorCode::SignatureUnsupported => {
            b.flag(Verdict::Unsupported, e.message.clone());
            b.check(
                "integrity",
                L_INTEGRITY,
                CheckState::Unknown,
                e.message.clone(),
            );
            b.check("signer", L_SIGNER, CheckState::Unknown, e.message);
            false
        }
        Err(e) => {
            let why = if e.message.contains("messageDigest") {
                "Nội dung tài liệu đã bị sửa sau khi ký (chữ ký không khớp nội dung).".to_owned()
            } else {
                e.message
            };
            b.flag(Verdict::InvalidSignature, why.clone());
            if why.contains("signingCertificate") {
                b.check("signer", L_SIGNER, CheckState::Fail, why);
            } else {
                b.check("integrity", L_INTEGRITY, CheckState::Fail, why);
            }
            false
        }
    };
    // 3. Thời gian ký.
    let signing_unix = signed_attribute(info, oid::SIGNING_TIME).and_then(|v| decode_time(&v));
    let mut claimed = signing_unix;
    b.r.signing_time = signing_unix.map(vn);
    let stamp = match timestamp_token(info) {
        None => None,
        Some(token) => {
            b.r.has_timestamp = true;
            match verify_timestamp_token(&token, info.signature.as_bytes()) {
                Ok(s) => Some(s),
                Err(e) => {
                    let (v, why) = if e.code == ErrorCode::SignatureUnsupported {
                        (Verdict::Unsupported, e.message)
                    } else {
                        (
                            Verdict::UntrustedTsa,
                            format!("Chữ ký của TSA không hợp lệ với chữ ký này: {}", e.message),
                        )
                    };
                    b.flag(v, why.clone());
                    b.check("time", L_TIME, CheckState::Fail, why);
                    None
                }
            }
        }
    };
    let mut trusted_time = None;
    if let Some(stamp) = &stamp {
        b.r.timestamp_authority = Some(stamp.tsa.common_name());
        b.r.trusted_time = Some(vn(stamp.gen_time));
        b.r.trusted_unix = Some(stamp.gen_time);
        if tsa_trusted(b, stamp, options) {
            trusted_time = Some(stamp.gen_time);
        }
    }
    // Lệch giờ giữa giờ tự khai và giờ TSA.
    if let (Some(signing), Some(at)) = (signing_unix, b.r.trusted_unix) {
        let mut delta = signing - at;
        let quirk = vk::SIGNER_LOCAL_TIME_QUIRK_HOURS * 3600;
        if (delta - quirk).abs() <= vk::SIGNER_LOCAL_TIME_QUIRK_WINDOW_SECONDS {
            delta -= quirk;
            b.warn(format!(
                "Giờ ký tự khai ghi theo giờ địa phương (UTC+{}) nhưng đánh dấu UTC — sai chuẩn PKCS#9, chấp nhận theo diện tương thích ngược.",
                vk::SIGNER_LOCAL_TIME_QUIRK_HOURS
            ));
            b.r.signing_time = Some(vn(signing - quirk));
            claimed = Some(signing - quirk);
        }
        if delta > vk::TSA_CLOCK_SKEW_GRACE_SECONDS {
            let why = format!(
                "Giờ ký tự khai đứng SAU thời điểm TSA đóng dấu {delta}s — không thể xảy ra trong trình tự ký hợp lệ."
            );
            b.flag(Verdict::TimeMismatch, why.clone());
            b.check("time", L_TIME, CheckState::Fail, why);
        }
    }
    if !b.r.checks.iter().any(|c| c.id == "time") {
        if let Some(at) = trusted_time {
            b.check(
                "time",
                L_TIME,
                CheckState::Ok,
                format!(
                    "Dấu thời gian của {} lúc {}.",
                    b.r.timestamp_authority.clone().unwrap_or_default(),
                    vn(at)
                ),
            );
        } else if stamp.is_none() && !b.r.has_timestamp {
            b.flag(
                Verdict::NoTimestamp,
                "Chữ ký không có dấu thời gian tin cậy (RFC 3161).",
            );
            b.check(
                "time",
                L_TIME,
                CheckState::Warn,
                "Không có dấu thời gian: giờ ký là giờ máy người ký tự ghi, không chứng minh được.",
            );
        }
    }
    // 4 + 5. Chứng thư người ký: kiểm tại giờ TSA, không có thì tại giờ tự khai.
    let claimed = claimed.or_else(|| field.signing_time.as_deref().and_then(parse_pdf_date));
    let Some(cert) = signer else {
        let why = "Chữ ký không nhúng chứng thư số của người ký.";
        b.flag(Verdict::UntrustedSigner, why);
        b.check("certificate", L_CERT, CheckState::Fail, why);
        b.check("chain", L_CHAIN, CheckState::Fail, why);
        return;
    };
    if !ok
        && b.r.verdict == Verdict::InvalidSignature
        && !b.r.checks.iter().any(|c| c.id == "signer")
    {
        b.check(
            "signer",
            L_SIGNER,
            CheckState::Unknown,
            "Chữ ký không khớp nội dung nên không đối chiếu được.",
        );
    }
    let at = trusted_time.or(claimed);
    let embedded = cv::embedded_certificates(&sd);
    signer_chain(
        b,
        &cert,
        &embedded,
        at,
        trusted_time.is_some(),
        options,
        dss,
    );
}

/// Chuỗi TSA về Root G2 tại thời điểm đóng dấu (kèm thu hồi nếu được yêu cầu). Ghi điều kiện "thời gian".
fn tsa_trusted(b: &mut Builder, stamp: &TimestampInfo, options: VerifyOptions) -> bool {
    match chain_check(
        &stamp.tsa,
        &stamp.certificates,
        Some(stamp.gen_time),
        options.check_revocation,
    ) {
        Ok(c) if c.trusted && c.root == Some("Root G2") && c.time_valid => {
            if c.revocation == RevocationStatus::Revoked
                && !c.revoked_at.is_some_and(|r| r > stamp.gen_time)
            {
                let why = "Chứng thư của máy chủ dấu thời gian đã bị thu hồi.";
                b.flag(Verdict::UntrustedTsa, why);
                b.check("time", L_TIME, CheckState::Fail, why);
                return false;
            }
            true
        }
        Ok(c) if c.trusted && c.root == Some("Root G2") => {
            let why = "Chứng thư của máy chủ dấu thời gian hết hạn tại thời điểm đóng dấu.";
            b.flag(Verdict::UntrustedTsa, why);
            b.check("time", L_TIME, CheckState::Fail, why);
            false
        }
        _ => {
            let why = "Dấu thời gian không thuộc TSA Ban Cơ yếu (chain không về Root G2).";
            b.flag(Verdict::UntrustedTsa, why);
            b.check("time", L_TIME, CheckState::Fail, why);
            false
        }
    }
}

fn signer_chain(
    b: &mut Builder,
    cert: &Cert,
    embedded: &[Cert],
    at: Option<i64>,
    at_is_trusted: bool,
    options: VerifyOptions,
    dss: Option<&smks_pdf::incremental::DssData>,
) {
    let when = at.map(vn).unwrap_or_else(|| "thời điểm kiểm".into());
    let report = match chain_check(cert, embedded, at, options.check_revocation) {
        Ok(r) => r,
        Err(e) => {
            let why = format!("Không kiểm được chuỗi chứng thư: {}", e.message);
            b.flag(Verdict::UntrustedSigner, why.clone());
            b.check("certificate", L_CERT, CheckState::Unknown, why.clone());
            b.check("chain", L_CHAIN, CheckState::Unknown, why);
            return;
        }
    };
    b.r.chain = report.chain.clone();
    // 5. Đường dẫn chứng thực.
    if report.trusted {
        b.r.signer_root = report.root.map(str::to_owned);
        b.check(
            "chain",
            L_CHAIN,
            CheckState::Ok,
            format!(
                "Chuỗi chứng thư về {} đã ghim.",
                report.root.unwrap_or("gốc Ban Cơ yếu")
            ),
        );
    } else {
        let why =
            "Chứng thư số người ký không thuộc CA Ban Cơ yếu (chain không về Root G1 lẫn Root G2).";
        b.flag(Verdict::UntrustedSigner, why);
        b.check("chain", L_CHAIN, CheckState::Fail, why);
    }
    // 4. Hiệu lực tại thời điểm ký.
    if !cert.allows_signing() {
        let why = "Chứng thư không được phép dùng để ký số (KeyUsage không có digitalSignature / nonRepudiation).";
        b.flag(Verdict::UntrustedSigner, why);
        b.check("certificate", L_CERT, CheckState::Fail, why);
        return;
    }
    if !report.time_valid {
        let why = format!("Chứng thư hết hạn hoặc chưa có hiệu lực tại thời điểm ký ({when}).");
        b.flag(Verdict::UntrustedSigner, why.clone());
        b.check("certificate", L_CERT, CheckState::Fail, why);
        return;
    }
    let time_note = if at_is_trusted {
        format!("Còn hạn tại thời điểm trên dấu thời gian ({when})")
    } else {
        format!("Còn hạn tại giờ ký tự khai ({when})")
    };
    // Dữ liệu thu hồi nhúng lúc ký (PAdES B-LT, TD-58): dùng khi không kiểm qua mạng hoặc mạng không trả lời.
    let mut revocation = report.revocation;
    let mut revoked_at = report.revoked_at;
    let mut file_note = String::new();
    if let (Some(d), Some(issuer)) = (dss, report.certs.get(1))
        && let Some(info) = smks_crypto::ltv::find_embedded(&d.ocsps, &d.crls, cert, issuer)
    {
        use smks_crypto::ltv::RevStatus;
        // Phản hồi chứng minh trạng thái tại lúc ký: tạo sau lúc ký, hoặc còn hạn (nextUpdate) tới lúc ký.
        let covers =
            at.is_none_or(|a| info.produced_at >= a || info.next_update.is_some_and(|n| n >= a));
        let evidence = match info.status {
            RevStatus::Good => covers,
            RevStatus::Revoked(_) => true,
            RevStatus::Unknown => false,
        };
        b.r.long_term = evidence;
        if evidence
            && matches!(
                revocation,
                RevocationStatus::NotChecked | RevocationStatus::Unknown
            )
        {
            b.r.revocation_embedded = true;
            file_note = format!(
                " theo dữ liệu nhúng trong file (phản hồi lúc {})",
                vn(info.produced_at)
            );
            match info.status {
                RevStatus::Revoked(t) => {
                    revocation = RevocationStatus::Revoked;
                    revoked_at = Some(t);
                }
                _ => revocation = RevocationStatus::Good,
            }
        }
    }
    b.r.revocation = revocation;
    b.r.revoked_at = revoked_at.map(vn);
    let report = ChainCheck {
        revocation,
        revoked_at,
        ..report
    };
    match report.revocation {
        RevocationStatus::NotChecked => b.check(
            "certificate",
            L_CERT,
            CheckState::Warn,
            format!("{time_note}; chưa kiểm tra thu hồi (bật \"Kiểm tra thu hồi\", cần mạng)."),
        ),
        RevocationStatus::Good => b.check(
            "certificate",
            L_CERT,
            CheckState::Ok,
            format!("{time_note}; chưa bị thu hồi (CRL/OCSP){file_note}."),
        ),
        RevocationStatus::Unknown => {
            let why =
                "Không tải được CRL/OCSP để kiểm thu hồi (mất mạng hoặc máy chủ CA không trả lời).";
            b.flag(Verdict::RevocationUnknown, why);
            b.check(
                "certificate",
                L_CERT,
                CheckState::Warn,
                format!("{time_note}; {why}"),
            );
        }
        RevocationStatus::Revoked => {
            let after = at_is_trusted && report.revoked_at.zip(at).is_some_and(|(r, a)| r > a);
            if after {
                b.r.revocation = RevocationStatus::RevokedAfterSigning;
                let note = format!(
                    "Chứng thư bị thu hồi ngày {}, sau thời điểm ký: chữ ký vẫn hợp lệ.",
                    b.r.revoked_at.clone().unwrap_or_default()
                );
                b.warn(note.clone());
                b.check(
                    "certificate",
                    L_CERT,
                    CheckState::Ok,
                    format!("{time_note}; {note}"),
                );
            } else {
                let why = match &b.r.revoked_at {
                    Some(d) if at_is_trusted => {
                        format!("Chứng thư số người ký đã bị thu hồi ngày {d}, trước thời điểm ký.")
                    }
                    Some(d) => format!(
                        "Chứng thư số người ký đã bị thu hồi ngày {d}; chữ ký không có dấu thời gian nên không chứng minh được đã ký trước đó."
                    ),
                    None => "Chứng thư số người ký đã bị thu hồi.".to_owned(),
                };
                b.flag(Verdict::UntrustedSigner, why.clone());
                b.check("certificate", L_CERT, CheckState::Fail, why);
            }
        }
        RevocationStatus::RevokedAfterSigning => {}
    }
}

/// Dấu thời gian tài liệu: token RFC 3161 phủ hai đoạn `/ByteRange`.
fn verify_doc_timestamp(
    b: &mut Builder,
    field: &SignatureField,
    parts: &[&[u8]; 2],
    options: VerifyOptions,
) {
    b.r.has_timestamp = true;
    let stamp = match verify_timestamp_token_parts(&field.contents, parts) {
        Ok(s) => s,
        Err(e) => {
            let (v, why) = if e.code == ErrorCode::SignatureUnsupported {
                (Verdict::Unsupported, e.message)
            } else if e.message.contains("messageImprint") {
                (Verdict::InvalidSignature, "Nội dung tài liệu đã bị sửa sau khi đóng dấu thời gian (messageImprint không khớp).".to_owned())
            } else {
                (
                    Verdict::InvalidSignature,
                    format!("Dấu thời gian tài liệu không hợp lệ: {}", e.message),
                )
            };
            b.flag(v, why.clone());
            b.check("integrity", L_INTEGRITY, CheckState::Fail, why);
            return;
        }
    };
    b.r.signed_by = Some(stamp.tsa.common_name());
    b.r.organization = Some(stamp.tsa.organization()).filter(|o| !o.is_empty());
    b.r.timestamp_authority = Some(stamp.tsa.common_name());
    b.r.signer_serial = Some(stamp.tsa.serial_hex());
    b.r.issuer = Some(stamp.tsa.issuer_common_name());
    let (from, to) = stamp.tsa.validity();
    b.r.cert_valid_from = Some(vn(from));
    b.r.cert_valid_to = Some(vn(to));
    b.r.trusted_time = Some(vn(stamp.gen_time));
    b.r.trusted_unix = Some(stamp.gen_time);
    if tsa_trusted(b, &stamp, options) {
        b.check(
            "time",
            L_TIME,
            CheckState::Ok,
            format!(
                "Dấu thời gian tài liệu của {} lúc {}.",
                stamp.tsa.common_name(),
                vn(stamp.gen_time)
            ),
        );
        b.check(
            "chain",
            L_CHAIN,
            CheckState::Ok,
            "Máy chủ dấu thời gian thuộc Ban Cơ yếu (Root G2).",
        );
    } else {
        b.check(
            "chain",
            L_CHAIN,
            CheckState::Fail,
            "Máy chủ dấu thời gian không thuộc Ban Cơ yếu.",
        );
    }
    if let Ok(c) = chain_check(&stamp.tsa, &stamp.certificates, Some(stamp.gen_time), false) {
        b.r.chain = c.chain;
        b.r.signer_root = c.root.map(str::to_owned);
    }
}

/// Kết quả kiểm chuỗi đã đổi sang kiểu của báo cáo.
struct ChainCheck {
    trusted: bool,
    root: Option<&'static str>,
    time_valid: bool,
    revocation: RevocationStatus,
    revoked_at: Option<i64>,
    chain: Vec<ChainItem>,
    /// Từ chứng thư được kiểm tới gốc.
    certs: Vec<Cert>,
}

#[cfg(windows)]
fn chain_check(
    cert: &Cert,
    extra: &[Cert],
    at: Option<i64>,
    revocation: bool,
) -> Result<ChainCheck> {
    use smks_crypto::chain::{Revocation, validate};
    let r = validate(cert, extra, at, revocation)?;
    let status = match r.revocation {
        Revocation::NotChecked => RevocationStatus::NotChecked,
        Revocation::Good => RevocationStatus::Good,
        Revocation::Revoked => RevocationStatus::Revoked,
        Revocation::Unknown => RevocationStatus::Unknown,
    };
    let chain = r
        .elements
        .iter()
        .map(|c| {
            let (from, to) = c.validity();
            ChainItem {
                name: c.common_name(),
                serial: c.serial_hex(),
                valid_from: vn(from),
                valid_to: vn(to),
            }
        })
        .collect();
    Ok(ChainCheck {
        trusted: r.trusted,
        root: r.root,
        time_valid: r.time_valid,
        revocation: status,
        revoked_at: r.revoked_at,
        chain,
        certs: r.elements,
    })
}

#[cfg(not(windows))]
fn chain_check(_: &Cert, _: &[Cert], _: Option<i64>, _: bool) -> Result<ChainCheck> {
    Err(SmksError::new(
        ErrorCode::InternalError,
        "Kiểm chuỗi chứng thư chỉ có trên Windows.",
    ))
}

fn timestamp_token(info: &SignerInfo) -> Option<Vec<u8>> {
    let oid = der::asn1::ObjectIdentifier::new(oid::SIGNATURE_TIME_STAMP_TOKEN).ok()?;
    info.unsigned_attrs
        .as_ref()?
        .iter()
        .find(|a| a.oid == oid)
        .and_then(|a| a.values.iter().next())
        .and_then(|v| v.to_der().ok())
}

fn decode_time(value: &der::Any) -> Option<i64> {
    let time: x509_cert::time::Time = value.decode_as().ok()?;
    i64::try_from(time.to_unix_duration().as_secs()).ok()
}

/// Một chuỗi chứng thư của chữ ký cuối (từ chứng thư lá tới gốc), để lấy dữ liệu kiểm tra dài hạn.
#[derive(Debug, Clone)]
pub struct SignatureChain {
    pub certs: Vec<Cert>,
    /// Chuỗi của người ký (không phải của TSA).
    pub signer: bool,
    /// Về gốc Ban Cơ yếu đã ghim.
    pub trusted: bool,
}

/// Chuỗi người ký và chuỗi TSA của chữ ký thường cuối cùng trong file.
pub fn last_signature_chains(bytes: &[u8]) -> Vec<SignatureChain> {
    let Ok(fields) = sigscan::scan(bytes) else {
        return Vec::new();
    };
    let Some(field) = fields.iter().rev().find(|f| !f.doc_timestamp) else {
        return Vec::new();
    };
    let Ok(sd) = parse_signed_data(&field.contents) else {
        return Vec::new();
    };
    let Some(info) = sd.signer_infos.0.iter().next() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let embedded = cv::embedded_certificates(&sd);
    if let Some(cert) = cv::find_signer_certificate(&sd, info)
        && let Some(c) = ordered_chain(&cert, &embedded)
    {
        out.push(SignatureChain {
            certs: c.0,
            signer: true,
            trusted: c.1,
        });
    }
    if let Some(token) = timestamp_token(info)
        && let Ok(stamp) = verify_timestamp_token(&token, info.signature.as_bytes())
        && let Some(c) = ordered_chain(&stamp.tsa, &stamp.certificates)
    {
        out.push(SignatureChain {
            certs: c.0,
            signer: false,
            trusted: c.1,
        });
    }
    out
}

#[cfg(windows)]
fn ordered_chain(cert: &Cert, extra: &[Cert]) -> Option<(Vec<Cert>, bool)> {
    let r = smks_crypto::chain::validate(cert, extra, None, false).ok()?;
    (!r.elements.is_empty()).then_some((r.elements, r.trusted))
}

#[cfg(not(windows))]
fn ordered_chain(_: &Cert, _: &[Cert]) -> Option<(Vec<Cert>, bool)> {
    None
}

/// Kiểm lại chữ ký **vừa ký** (bước cuối của lô ký): chữ ký cuối khớp nội dung, phủ tới cuối file, dấu thời
/// gian (nếu có) đúng và thuộc TSA Ban Cơ yếu. Không xét người ký có tin cậy hay không (đã kiểm khi mở khoá).
pub fn check_just_signed(bytes: &[u8], expect_timestamp: bool) -> Result<SignatureReport> {
    let report = verify_pdf(bytes, VerifyOptions::default())?;
    let last = report.signatures.last().cloned().ok_or_else(|| {
        SmksError::new(
            ErrorCode::SignFailed,
            "Không tìm thấy chữ ký vừa ký trong file kết quả.",
        )
    })?;
    let bad = |m: String| {
        Err(SmksError::new(
            ErrorCode::SignFailed,
            format!("Kiểm tra sau khi ký không đạt: {m}"),
        ))
    };
    if !last.covers_whole_file {
        return bad("chữ ký vừa ký không phủ tới cuối file.".into());
    }
    match last.verdict {
        Verdict::InvalidSignature | Verdict::ModifiedAfterSigning => {
            bad(last.reason.clone().unwrap_or_default())
        }
        Verdict::UntrustedTsa if expect_timestamp => bad(last.reason.clone().unwrap_or_default()),
        _ if expect_timestamp && !last.has_timestamp => bad("thiếu dấu thời gian.".into()),
        _ => Ok(last),
    }
}

/// Giờ hiện tại theo giờ Việt Nam (dùng cho dòng "Ngày ký").
pub fn now_vietnam() -> (i64, String) {
    let now = unix_now();
    (now, vn(now))
}

/// Giây Unix hiện tại theo đồng hồ máy.
pub fn clock_now() -> i64 {
    unix_now()
}

/// Một thời điểm (giây Unix) theo giờ Việt Nam, dạng `dd/mm/yyyy hh:mm:ss`.
pub fn vietnam_text(unix: i64) -> String {
    vn(unix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_dates() {
        // 24/11/2026 10:30:00 giờ Việt Nam = 03:30:00 UTC.
        let utc = days_from_civil(2026, 11, 24) * 86_400 + 3 * 3600 + 30 * 60;
        assert_eq!(parse_pdf_date("D:20261124103000+07'00'"), Some(utc));
        assert_eq!(parse_pdf_date("D:20261124033000Z"), Some(utc));
        assert_eq!(parse_pdf_date("D:20261124033000"), Some(utc));
        assert_eq!(
            parse_pdf_date("D:2026"),
            Some(days_from_civil(2026, 1, 1) * 86_400)
        );
        assert_eq!(parse_pdf_date("D:20261324"), None);
        assert_eq!(parse_pdf_date("xx"), None);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn verdict_order_puts_modification_above_untrusted_signer() {
        assert!(Verdict::ModifiedAfterSigning > Verdict::UntrustedSigner);
        assert!(Verdict::InvalidSignature > Verdict::ModifiedAfterSigning);
        assert!(Verdict::RevocationUnknown > Verdict::ValidTrusted);
        assert!(Verdict::RevocationUnknown < Verdict::NoTimestamp);
        assert!(Verdict::Unsupported > Verdict::UntrustedTsa);
    }
}
