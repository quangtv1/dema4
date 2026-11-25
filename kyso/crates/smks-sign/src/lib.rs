//! Ký và xác minh PDF: ghép `smks-pdf` (cấu trúc file) với `smks-crypto` (khoá, CMS, TSA, chuỗi CA).
//!
//! - [`sign`]: ký một file, kiểm lại ngay sau khi ký.
//! - [`verify`]: xác minh mọi chữ ký của một file theo cách kết luận của bản cũ.

mod ffi;
pub mod job;
pub mod sign;
pub mod verify;

#[cfg(test)]
mod tests {
    use smks_core::consts::signing as k;
    use smks_crypto::testpki::TestPki;
    use smks_pdf::incremental::SignRequest;

    use crate::sign::sign_pdf;
    use crate::verify::{Verdict, VerifyOptions, verify_pdf};

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/pdf")
                .join(name),
        )
        .unwrap()
    }

    fn request() -> SignRequest {
        SignRequest {
            signer_name: "Nguyễn Văn Thử".into(),
            reason: k::DEFAULT_REASON.into(),
            location: k::DEFAULT_LOCATION.into(),
            signed_at: 1_795_057_200,
            widgets: vec![],
            stamps: vec![],
        }
    }

    #[test]
    fn sign_verify_tamper_and_append() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let original = fixture("word-hybrid-xref.pdf");
        let signed = sign_pdf(
            &original,
            &request(),
            &key,
            std::slice::from_ref(&pki.ca),
            None,
        )
        .unwrap();
        assert_eq!(signed.check.signed_by.as_deref(), Some("Nguyễn Văn Thử"));
        let report = verify_pdf(&signed.bytes, VerifyOptions::default()).unwrap();
        assert_eq!(report.signatures.len(), 1);
        // Không có TSA nhưng người ký vẫn được kiểm (TD-57): CA thử không thuộc Ban Cơ yếu. Chữ ký khớp nội dung.
        assert_eq!(report.verdict, Verdict::UntrustedSigner);
        let state = |r: &crate::verify::SignatureReport, id: &str| {
            r.checks.iter().find(|c| c.id == id).map(|c| c.state)
        };
        use crate::verify::CheckState;
        let s = &report.signatures[0];
        assert_eq!(state(s, "integrity"), Some(CheckState::Ok));
        assert_eq!(state(s, "signer"), Some(CheckState::Ok), "{:?}", s.checks);
        assert_eq!(state(s, "time"), Some(CheckState::Warn));
        assert_eq!(state(s, "chain"), Some(CheckState::Fail));
        assert!(s.display_time.is_some());

        // Sửa 1 byte trong vùng được ký.
        let mut tampered = signed.bytes.clone();
        tampered[1000] ^= 0x01;
        let r = verify_pdf(&tampered, VerifyOptions::default()).unwrap();
        assert_eq!(r.verdict, Verdict::InvalidSignature, "{:?}", r.reason);

        // Nối thêm byte sau chữ ký.
        let mut appended = signed.bytes.clone();
        appended.extend_from_slice(b"\n% them sau khi ky\n");
        let r = verify_pdf(&appended, VerifyOptions::default()).unwrap();
        assert_eq!(r.verdict, Verdict::InvalidSignature);
        assert!(r.reason.unwrap().contains("nối thêm"));

        // Ký chồng: cả hai chữ ký vẫn khớp nội dung.
        let twice = sign_pdf(&signed.bytes, &request(), &key, &[], None).unwrap();
        let r = verify_pdf(&twice.bytes, VerifyOptions::default()).unwrap();
        assert_eq!(r.signatures.len(), 2);
        assert!(
            r.signatures
                .iter()
                .all(|s| s.verdict == Verdict::UntrustedSigner),
            "{:?}",
            r.signatures
        );
        assert!(!r.signatures[0].covers_whole_file);
        // Sau chữ ký đầu chỉ có thêm chữ ký: không phải "sửa sau khi ký".
        assert_eq!(
            r.signatures[0].changes,
            smks_pdf::revdiff::ChangeLevel::Signatures,
            "{:?}",
            r.signatures[0].change_notes
        );
        assert_eq!(state(&r.signatures[0], "integrity"), Some(CheckState::Ok));
    }

    /// Bộ mẫu `fixtures/pdf/verify/` do pyHanko ký (`tools/sinh_mau_kiem_tra.py`, TD-57). Khoá thử nên người
    /// ký luôn "không thuộc CA Ban Cơ yếu"; ở đây kiểm các điều kiện còn lại và phần sau chữ ký.
    #[test]
    fn verify_fixtures_signed_by_pyhanko() {
        use crate::verify::{CheckState, SignatureKind, SignatureReport};
        use smks_pdf::revdiff::ChangeLevel;
        let check =
            |r: &SignatureReport, id: &str| r.checks.iter().find(|c| c.id == id).map(|c| c.state);
        let verify = |name: &str| {
            verify_pdf(
                &fixture(&format!("verify/{name}")),
                VerifyOptions::default(),
            )
            .unwrap()
        };

        // Ký có dấu thời gian TSA Ban Cơ yếu: toàn vẹn, khớp chứng thư, giờ TSA đúng; chuỗi người ký trượt.
        let r = verify("ky-tsa.pdf");
        let s = &r.signatures[0];
        assert_eq!(s.verdict, Verdict::UntrustedSigner);
        assert_eq!(check(s, "integrity"), Some(CheckState::Ok));
        assert_eq!(check(s, "signer"), Some(CheckState::Ok));
        assert_eq!(check(s, "time"), Some(CheckState::Ok));
        assert_eq!(check(s, "chain"), Some(CheckState::Fail));
        assert_eq!(
            s.timestamp_authority.as_deref(),
            Some("Máy chủ cấp dấu thời gian 02")
        );
        assert_eq!(s.changes, ChangeLevel::None);

        // Sửa nội dung trang rồi ký thêm: chữ ký đầu "bị sửa sau khi ký", chữ ký sau không bị ảnh hưởng.
        let r = verify("sua-roi-ky-them.pdf");
        assert_eq!(r.verdict, Verdict::ModifiedAfterSigning);
        assert_eq!(r.signatures[0].verdict, Verdict::ModifiedAfterSigning);
        assert_eq!(r.signatures[0].changes, ChangeLevel::Other);
        assert_eq!(check(&r.signatures[0], "integrity"), Some(CheckState::Fail));
        assert!(
            r.signatures[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("Trang 1 bị sửa")
        );
        assert_eq!(r.signatures[1].verdict, Verdict::UntrustedSigner);
        assert_eq!(r.signatures[1].changes, ChangeLevel::None);

        // Dấu thời gian tài liệu của TSA Ban Cơ yếu: hợp lệ; chữ ký trước nó không bị coi là sửa.
        let r = verify("dau-thoi-gian-tai-lieu.pdf");
        assert_eq!(r.signatures.len(), 2);
        assert_eq!(r.signatures[1].kind, SignatureKind::DocTimestamp);
        assert_eq!(
            r.signatures[1].verdict,
            Verdict::ValidTrusted,
            "{:?}",
            r.signatures[1].reason
        );
        assert_eq!(r.signatures[0].verdict, Verdict::UntrustedSigner);
        assert_eq!(r.signatures[0].changes, ChangeLevel::Signatures);
        assert_eq!(r.valid_count, 1);

        // Thêm chú thích sau khi ký: được phép nhưng cảnh báo.
        let r = verify("them-chu-thich.pdf");
        let s = &r.signatures[0];
        assert_eq!(s.verdict, Verdict::UntrustedSigner);
        assert_eq!(s.changes, ChangeLevel::Annotations);
        assert_eq!(check(s, "integrity"), Some(CheckState::Warn));
        assert!(
            s.warnings.iter().any(|w| w.contains("chú thích")),
            "{:?}",
            s.warnings
        );

        // Chỉ thêm DSS (dữ liệu kiểm tra dài hạn) sau chữ ký: không phải sửa, không phải byte lạ.
        let r = verify("them-dss.pdf");
        let s = &r.signatures[0];
        assert_eq!(s.verdict, Verdict::UntrustedSigner, "{:?}", s.reason);
        assert_eq!(s.changes, ChangeLevel::Signatures);
        assert_eq!(check(s, "integrity"), Some(CheckState::Ok));

        // Chữ ký chứng nhận DocMDP P=1 rồi thêm chú thích: vượt quyền.
        let r = verify("chung-nhan-p1-them-chu-thich.pdf");
        let s = &r.signatures[0];
        assert_eq!(s.docmdp, Some(1));
        assert_eq!(s.verdict, Verdict::ModifiedAfterSigning);
        assert!(s.reason.as_deref().unwrap().contains("DocMDP P=1"));

        // RSA-PSS: kiểm được (trước đây báo "không hợp lệ"); không có dấu thời gian nhưng vẫn kiểm người ký.
        let r = verify("rsa-pss.pdf");
        let s = &r.signatures[0];
        assert_eq!(s.verdict, Verdict::UntrustedSigner, "{:?}", s.reason);
        assert_eq!(check(s, "integrity"), Some(CheckState::Ok));
        assert_eq!(check(s, "time"), Some(CheckState::Warn));

        // File chưa ký.
        let r = verify_pdf(&fixture("table-xref.pdf"), VerifyOptions::default()).unwrap();
        assert_eq!(r.verdict, Verdict::Unsigned);
        assert!(r.signatures.is_empty());
    }

    /// Dữ liệu kiểm tra dài hạn (TD-58): CRL nhúng trong `/DSS` được dùng khi không kiểm thu hồi qua mạng.
    #[test]
    fn embedded_crl_gives_revocation_status_offline() {
        use crate::verify::{CheckState, RevocationStatus};
        use smks_pdf::incremental::{DssData, append_dss};
        let pki = TestPki::new();
        let key = pki.software_key();
        // Chứng thư thử cấp lúc chạy test: giờ ký phải nằm trong hạn của nó.
        let mut req = request();
        req.signed_at = crate::verify::clock_now();
        let signed = sign_pdf(
            &fixture("xref-stream.pdf"),
            &req,
            &key,
            std::slice::from_ref(&pki.ca),
            None,
        )
        .unwrap();
        let dss = |crl: Vec<u8>| DssData {
            certs: vec![pki.signer.der().to_vec(), pki.ca.der().to_vec()],
            ocsps: vec![],
            crls: vec![crl],
        };
        // CRL lập sau lúc ký, không có người ký: "chưa bị thu hồi" theo dữ liệu nhúng, chữ ký đầu không bị coi là sửa.
        let good = append_dss(&signed.bytes, &dss(pki.crl(None))).unwrap();
        let r = verify_pdf(&good, VerifyOptions::default()).unwrap();
        let s = &r.signatures[0];
        assert!(s.long_term && s.revocation_embedded, "{s:?}");
        assert_eq!(s.revocation, RevocationStatus::Good);
        assert_eq!(s.changes, smks_pdf::revdiff::ChangeLevel::Signatures);
        let cert = s.checks.iter().find(|c| c.id == "certificate").unwrap();
        assert_eq!(cert.state, CheckState::Ok, "{cert:?}");
        assert!(cert.detail.contains("dữ liệu nhúng"));
        assert_eq!(r.dss.map(|d| (d.certs, d.crls)), Some((2, 1)));
        // CRL ghi người ký bị thu hồi trước lúc ký (không có dấu thời gian): không tin chữ ký.
        let revoked = append_dss(&signed.bytes, &dss(pki.crl(Some(1_790_000_000)))).unwrap();
        let r = verify_pdf(&revoked, VerifyOptions::default()).unwrap();
        let s = &r.signatures[0];
        assert_eq!(s.revocation, RevocationStatus::Revoked);
        assert_eq!(s.verdict, Verdict::UntrustedSigner);
        let cert = s.checks.iter().find(|c| c.id == "certificate").unwrap();
        assert_eq!(cert.state, CheckState::Fail);
        assert!(cert.detail.contains("đã bị thu hồi"), "{cert:?}");
        // Không có DSS: như trước, chưa kiểm thu hồi.
        let r = verify_pdf(&signed.bytes, VerifyOptions::default()).unwrap();
        assert!(!r.signatures[0].long_term);
        assert_eq!(r.signatures[0].revocation, RevocationStatus::NotChecked);
        assert!(r.dss.is_none());
    }

    /// Ký dài hạn với TSA và OCSP thật: `cargo test -p smks-sign -- --ignored real_ltv`.
    #[test]
    #[ignore]
    fn real_ltv_embeds_tsa_revocation() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let tsa = smks_crypto::tsa::TsaClient::default();
        let signed = sign_pdf(
            &fixture("table-xref.pdf"),
            &request(),
            &key,
            &[],
            Some(&tsa),
        )
        .unwrap();
        let out = crate::sign::add_ltv(&signed.bytes, &smks_crypto::ltv::LtvCache::new());
        assert!(out.bytes.starts_with(&signed.bytes));
        // Khoá thử không có OCSP: không đánh dấu người ký dài hạn, không nhắc (chuỗi không thuộc Ban Cơ yếu).
        assert!(!out.long_term);
        assert!(out.note.is_none());
        let d = smks_pdf::incremental::read_dss(&out.bytes)
            .unwrap()
            .unwrap();
        assert!(!d.ocsps.is_empty(), "OCSP của TSA");
        let r = verify_pdf(&out.bytes, VerifyOptions::default()).unwrap();
        assert_eq!(r.signatures.len(), 1);
        assert_eq!(
            r.signatures[0].changes,
            smks_pdf::revdiff::ChangeLevel::Signatures
        );
        assert_eq!(r.signatures[0].verdict, Verdict::UntrustedSigner);
    }

    /// Ký với TSA thật: `cargo test -p smks-sign -- --ignored real_tsa`.
    #[test]
    #[ignore]
    fn real_tsa_signature_is_timestamped_but_test_ca_untrusted() {
        let pki = TestPki::new();
        let key = pki.software_key();
        let tsa = smks_crypto::tsa::TsaClient::default();
        let signed = sign_pdf(
            &fixture("xref-stream.pdf"),
            &request(),
            &key,
            &[],
            Some(&tsa),
        )
        .unwrap();
        let r = verify_pdf(&signed.bytes, VerifyOptions::default()).unwrap();
        let s = &r.signatures[0];
        println!(
            "{:?} | {:?} | TSA {:?} lúc {:?}",
            s.verdict, s.reason, s.timestamp_authority, s.trusted_time
        );
        assert!(s.has_timestamp);
        assert_eq!(
            s.timestamp_authority.as_deref(),
            Some("Máy chủ cấp dấu thời gian 02")
        );
        // CA thử không thuộc Ban Cơ yếu.
        assert_eq!(s.verdict, Verdict::UntrustedSigner);
    }
}
