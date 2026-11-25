//! Spike S2a: ký nối bản một PDF theo PAdES B-T bằng khoá phần mềm.
//!
//! - CMS SignedData detached dựng tay từ các kiểu của crate `cms` (không dùng builder), để việc ký
//!   là một hàm `ký(băm) -> chữ ký` thay được bằng token về sau.
//! - Signed attributes: contentType, messageDigest, signingCertificateV2. Không có signing-time (ADR 0006).
//! - Dấu thời gian RFC 3161 lấy thật từ TSA của Ban Cơ yếu, gắn vào unsigned attribute.
//! - Ghi nối bản: ghi đè Catalog và Page, thêm field chữ ký ẩn và từ điển /Sig, xref cùng kiểu với file gốc.
//!
//! Cách dùng: s2-cms <thư mục pki> <vào.pdf> <ra.pdf> [--no-tsa]

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::time::Duration;

use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo, SignerInfos,
};
use der::asn1::{Int, ObjectIdentifier, OctetString, SetOfVec};
use der::{Any, Decode, Encode};
use lopdf::{Dictionary, Document, Object, ObjectId, xref::XrefType};
use rsa::pkcs8::DecodePrivateKey;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha2::{Digest, Sha256};
use smks_core::consts::signing as k;
use spki::AlgorithmIdentifierOwned;
use x509_cert::Certificate;
use x509_cert::attr::Attribute;
use x509_tsp::{MessageImprint, TimeStampReq, TimeStampResp, TspVersion, TstInfo};

type AnyResult<T> = Result<T, Box<dyn Error>>;

const OID_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
const OID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const OID_CONTENT_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
const OID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const OID_SIGNING_CERT_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.47");
const OID_TIMESTAMP_TOKEN: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.14");
const OID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const OID_RSA_ENCRYPTION: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
const OID_EXT_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.37");

fn main() -> AnyResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("Cách dùng: s2-cms <thư mục pki> <vào.pdf> <ra.pdf> [--no-tsa]".into());
    }
    let use_tsa = !args.iter().any(|a| a == "--no-tsa");
    let pki = std::path::Path::new(&args[0]);

    let signer_der = std::fs::read(pki.join("signer.cert.der"))?;
    let ca_der = std::fs::read(pki.join("ca.cert.der"))?;
    let key = RsaPrivateKey::from_pkcs8_der(&std::fs::read(pki.join("signer.key.pk8.der"))?)?;
    let signer_cert = Certificate::from_der(&signer_der)?;
    let ca_cert = Certificate::from_der(&ca_der)?;
    let signer_name = common_name(&signer_cert);

    let original = std::fs::read(&args[1])?;
    let mut prepared = prepare_incremental(&original, &signer_name)?;
    println!(
        "File gốc {} byte, xref kiểu {}, thêm {} byte; /ByteRange {:?}",
        original.len(),
        prepared.xref_kind,
        prepared.bytes.len() - original.len(),
        prepared.byte_range
    );

    // Băm phần được ký: toàn bộ file trừ chuỗi hex của /Contents.
    let [_, len1, start2, len2] = prepared.byte_range;
    let mut hasher = Sha256::new();
    hasher.update(&prepared.bytes[..len1]);
    hasher.update(&prepared.bytes[start2..start2 + len2]);
    let document_digest = hasher.finalize();

    // Việc ký là một hàm nhận băm SHA-256: sau này thay bằng NCryptSignHash / CryptSignHashW.
    let sign_digest = |digest: &[u8]| -> AnyResult<Vec<u8>> {
        Ok(key.sign(Pkcs1v15Sign::new::<rsa::sha2::Sha256>(), digest)?)
    };

    let cms_der = build_cades(
        document_digest.as_slice(),
        &signer_cert,
        &signer_der,
        &[ca_cert],
        &sign_digest,
        use_tsa,
    )?;
    println!("CMS dài {} byte (chỗ trống {} byte)", cms_der.len(), k::SIGNATURE_CONTENTS_BYTES);
    if cms_der.len() > k::SIGNATURE_CONTENTS_BYTES {
        return Err("Khối chữ ký vượt chỗ trống đã chừa.".into());
    }

    let hex = hex_upper(&cms_der);
    let start = prepared.contents_hex_start;
    prepared.bytes[start..start + hex.len()].copy_from_slice(hex.as_bytes());
    std::fs::write(&args[2], &prepared.bytes)?;
    println!("Đã ghi {} ({} byte)", args[2], prepared.bytes.len());
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// CMS / CAdES
// ----------------------------------------------------------------------------------------------

fn sha256_algorithm() -> AlgorithmIdentifierOwned {
    AlgorithmIdentifierOwned { oid: OID_SHA256, parameters: None }
}

fn attribute(oid: ObjectIdentifier, value: Any) -> AnyResult<Attribute> {
    Ok(Attribute { oid, values: SetOfVec::try_from(vec![value])? })
}

fn build_cades(
    document_digest: &[u8],
    signer: &Certificate,
    signer_der: &[u8],
    extra_certs: &[Certificate],
    sign_digest: &dyn Fn(&[u8]) -> AnyResult<Vec<u8>>,
    use_tsa: bool,
) -> AnyResult<Vec<u8>> {
    // signingCertificateV2 giống bản .NET cũ: chỉ có certHash, băm SHA-256 mặc định, không issuerSerial.
    // SEQUENCE { SEQUENCE { SEQUENCE { OCTET STRING } } }
    let cert_hash = Sha256::digest(signer_der);
    let mut signing_cert = vec![0x30, 0x26, 0x30, 0x24, 0x30, 0x22, 0x04, 0x20];
    signing_cert.extend_from_slice(cert_hash.as_slice());

    let signed_attrs = SetOfVec::try_from(vec![
        attribute(OID_CONTENT_TYPE, Any::encode_from(&OID_DATA)?)?,
        attribute(OID_MESSAGE_DIGEST, Any::encode_from(&OctetString::new(document_digest)?)?)?,
        attribute(OID_SIGNING_CERT_V2, Any::from_der(&signing_cert)?)?,
    ])?;

    // Chữ ký tính trên DER của signedAttrs dưới dạng SET OF (thẻ 0x31).
    let to_sign = Sha256::digest(signed_attrs.to_der()?);
    let signature = sign_digest(to_sign.as_slice())?;

    let unsigned_attrs = if use_tsa {
        let token_der = request_timestamp(&signature)?;
        Some(SetOfVec::try_from(vec![attribute(OID_TIMESTAMP_TOKEN, Any::from_der(&token_der)?)?])?)
    } else {
        None
    };

    let tbs = signer.tbs_certificate();
    let signer_info = SignerInfo {
        version: CmsVersion::V1,
        sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: tbs.issuer().clone(),
            serial_number: tbs.serial_number().clone(),
        }),
        digest_alg: sha256_algorithm(),
        signed_attrs: Some(signed_attrs),
        signature_algorithm: AlgorithmIdentifierOwned {
            oid: OID_RSA_ENCRYPTION,
            parameters: Some(Any::from_der(&[0x05, 0x00])?),
        },
        signature: OctetString::new(signature)?,
        unsigned_attrs,
    };

    let mut certificates = vec![CertificateChoices::Certificate(signer.clone())];
    certificates.extend(extra_certs.iter().cloned().map(CertificateChoices::Certificate));

    let signed_data = SignedData {
        version: CmsVersion::V1,
        digest_algorithms: SetOfVec::try_from(vec![sha256_algorithm()])?,
        encap_content_info: EncapsulatedContentInfo { econtent_type: OID_DATA, econtent: None },
        certificates: Some(CertificateSet(SetOfVec::try_from(certificates)?)),
        crls: None,
        signer_infos: SignerInfos(SetOfVec::try_from(vec![signer_info])?),
    };
    let content_info =
        ContentInfo { content_type: OID_SIGNED_DATA, content: Any::encode_from(&signed_data)? };
    Ok(content_info.to_der()?)
}

// ----------------------------------------------------------------------------------------------
// Dấu thời gian RFC 3161
// ----------------------------------------------------------------------------------------------

fn request_timestamp(signature: &[u8]) -> AnyResult<Vec<u8>> {
    let imprint = Sha256::digest(signature);

    // Nonce 8 byte. Spike không dùng bộ sinh ngẫu nhiên mật mã; sản phẩm sẽ dùng.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let nonce_bytes = &Sha256::digest(nanos.to_le_bytes())[..k::TSA_NONCE_BYTES];
    let mut positive = nonce_bytes.to_vec();
    positive[0] &= 0x7F;
    let nonce = Int::new(&positive)?;

    let request = TimeStampReq {
        version: TspVersion::V1,
        message_imprint: MessageImprint {
            hash_algorithm: spki::AlgorithmIdentifier { oid: OID_SHA256, parameters: None },
            hashed_message: OctetString::new(imprint.as_slice())?,
        },
        req_policy: None,
        nonce: Some(nonce.clone()),
        cert_req: true,
        extensions: None,
    };
    let request_der = request.to_der()?;

    let started = std::time::Instant::now();
    let mut response = ureq::post(k::TSA_URL)
        .config()
        .timeout_global(Some(Duration::from_secs(k::TSA_TIMEOUT_SECONDS)))
        .build()
        .header("Content-Type", k::TSA_CONTENT_TYPE)
        .send(&request_der[..])?;
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response.body_mut().read_to_vec()?;
    println!(
        "TSA {}: HTTP {status}, {content_type}, {} byte, {:.2}s",
        k::TSA_URL,
        body.len(),
        started.elapsed().as_secs_f32()
    );

    let reply = TimeStampResp::from_der(&body)?;
    let pki_status = reply.status.status as u8;
    if pki_status > 1 {
        return Err(format!("TSA từ chối, PKIStatus = {pki_status}").into());
    }
    let token = reply.time_stamp_token.ok_or("Phản hồi TSA không có token")?;
    let token_der = token.to_der()?;

    // Kiểm cấu trúc token: messageImprint và nonce phải khớp yêu cầu.
    let token_data: SignedData = token.content.decode_as()?;
    let econtent = token_data.encap_content_info.econtent.as_ref().ok_or("Token không có TSTInfo")?;
    let tst_octets: OctetString = econtent.decode_as()?;
    let tst = TstInfo::from_der(tst_octets.as_bytes())?;
    if tst.message_imprint.hashed_message.as_bytes() != imprint.as_slice() {
        return Err("messageImprint của token không khớp yêu cầu".into());
    }
    if tst.nonce.as_ref() != Some(&nonce) {
        return Err("nonce của token không khớp yêu cầu".into());
    }
    println!(
        "Token hợp cấu trúc: PKIStatus {pki_status}, genTime {}, serial {}, policy {}, {} byte",
        tst.gen_time.to_date_time(),
        hex_upper(tst.serial_number.as_bytes()),
        tst.policy,
        token_der.len()
    );
    println!("Chứng thư trong token (trả lời R15):");
    if let Some(certs) = &token_data.certificates {
        for choice in certs.0.iter() {
            if let CertificateChoices::Certificate(cert) = choice {
                let tbs = cert.tbs_certificate();
                let has_eku = tbs
                    .extensions()
                    .is_some_and(|exts| exts.iter().any(|e| e.extn_id == OID_EXT_KEY_USAGE));
                println!(
                    "  - chủ thể: {}\n    cấp bởi: {}\n    hiệu lực: {} đến {} | có extKeyUsage: {has_eku}",
                    tbs.subject(),
                    tbs.issuer(),
                    tbs.validity().not_before,
                    tbs.validity().not_after
                );
            }
        }
    } else {
        println!("  (token không kèm chứng thư)");
    }
    Ok(token_der)
}

// ----------------------------------------------------------------------------------------------
// Ghi nối bản
// ----------------------------------------------------------------------------------------------

struct Prepared {
    bytes: Vec<u8>,
    /// [0, độ dài phần đầu, vị trí bắt đầu phần sau, độ dài phần sau]
    byte_range: [usize; 4],
    /// Vị trí chữ số hex đầu tiên của /Contents (ngay sau dấu `<`).
    contents_hex_start: usize,
    xref_kind: &'static str,
}

fn prepare_incremental(original: &[u8], signer_name: &str) -> AnyResult<Prepared> {
    let doc = Document::load_mem(original)?;
    if doc.trailer.has(b"Encrypt") {
        return Err("File PDF được mã hoá (/Encrypt) nên không ký nối bản được.".into());
    }
    let prev_xref = last_startxref(original)?;
    let uses_xref_stream =
        matches!(doc.reference_table.cross_reference_type, XrefType::CrossReferenceStream);

    let root_id = doc.trailer.get(b"Root")?.as_reference()?;
    let page_id = *doc.get_pages().get(&1).ok_or("File PDF không có trang nào")?;
    let trailer_size = doc.trailer.get(b"Size").and_then(Object::as_i64).unwrap_or(0) as u32;
    let mut next_id = trailer_size.max(doc.max_id + 1);
    let field_id: ObjectId = (next_id, 0);
    next_id += 1;
    let sig_id: ObjectId = (next_id, 0);
    next_id += 1;

    // Các object có sẵn cần ghi đè, giữ đúng số generation.
    let mut overrides: BTreeMap<ObjectId, Object> = BTreeMap::new();

    // Catalog -> /AcroForm -> /Fields, chấp nhận cả dạng trực tiếp lẫn tham chiếu gián tiếp.
    let mut catalog = doc.get_dictionary(root_id)?.clone();
    match catalog.get(b"AcroForm").ok().cloned() {
        None => {
            let mut form = Dictionary::new();
            form.set("Fields", vec![Object::Reference(field_id)]);
            form.set("SigFlags", 3);
            catalog.set("AcroForm", form);
            overrides.insert(root_id, Object::Dictionary(catalog));
        }
        Some(Object::Reference(form_id)) => {
            let mut form = doc.get_dictionary(form_id)?.clone();
            add_field(&doc, &mut form, field_id, &mut overrides)?;
            overrides.insert(form_id, Object::Dictionary(form));
        }
        Some(Object::Dictionary(mut form)) => {
            add_field(&doc, &mut form, field_id, &mut overrides)?;
            catalog.set("AcroForm", form);
            overrides.insert(root_id, Object::Dictionary(catalog));
        }
        Some(_) => return Err("/AcroForm của Catalog không hợp lệ".into()),
    }

    // Page -> /Annots.
    let mut page = doc.get_dictionary(page_id)?.clone();
    match page.get(b"Annots").ok().cloned() {
        None => {
            page.set("Annots", vec![Object::Reference(field_id)]);
            overrides.insert(page_id, Object::Dictionary(page));
        }
        Some(Object::Reference(annots_id)) => {
            let mut annots = doc.get_object(annots_id)?.as_array()?.clone();
            annots.push(Object::Reference(field_id));
            overrides.insert(annots_id, Object::Array(annots));
        }
        Some(Object::Array(mut annots)) => {
            annots.push(Object::Reference(field_id));
            page.set("Annots", annots);
            overrides.insert(page_id, Object::Dictionary(page));
        }
        Some(_) => return Err("/Annots của trang không hợp lệ".into()),
    }

    // Field chữ ký gộp với widget, ẩn (Rect rỗng), như bản cũ khi không hiện khối chữ ký.
    let mut field = Dictionary::new();
    field.set("Type", Object::Name(b"Annot".to_vec()));
    field.set("Subtype", Object::Name(b"Widget".to_vec()));
    field.set("FT", Object::Name(b"Sig".to_vec()));
    field.set("Ff", 0);
    field.set("T", pdf_text(&format!("{}{}", k::SIGNATURE_FIELD_NAME_PREFIX, field_id.0)));
    field.set("V", Object::Reference(sig_id));
    field.set("P", Object::Reference(page_id));
    field.set("F", 4);
    field.set("Rect", vec![Object::Integer(0); 4]);

    let mut out = original.to_vec();
    if !matches!(out.last(), Some(b'\n' | b'\r')) {
        out.push(b'\n');
    }
    let mut offsets: BTreeMap<ObjectId, usize> = BTreeMap::new();

    for (id, object) in &overrides {
        offsets.insert(*id, out.len());
        write!(ByteWriter(&mut out), "{} {} obj\n", id.0, id.1)?;
        write_object(&mut out, object);
        out.extend_from_slice(b"\nendobj\n");
    }
    offsets.insert(field_id, out.len());
    write!(ByteWriter(&mut out), "{} 0 obj\n", field_id.0)?;
    write_object(&mut out, &Object::Dictionary(field));
    out.extend_from_slice(b"\nendobj\n");

    // Từ điển chữ ký: /ByteRange và /Contents là chỗ trống có độ dài cố định, điền sau.
    offsets.insert(sig_id, out.len());
    write!(ByteWriter(&mut out), "{} 0 obj\n<</Type /Sig/Filter /{}/SubFilter /{}", sig_id.0, k::PDF_FILTER, k::PDF_SUB_FILTER)?;
    out.extend_from_slice(b"/Name ");
    write_object(&mut out, &pdf_text(signer_name));
    out.extend_from_slice(b"/Reason ");
    write_object(&mut out, &pdf_text(k::DEFAULT_REASON));
    out.extend_from_slice(b"/Location ");
    write_object(&mut out, &pdf_text(k::DEFAULT_LOCATION));
    write!(ByteWriter(&mut out), "/M ({})", pdf_date_now_vietnam())?;
    out.extend_from_slice(b"/ByteRange [");
    let byte_range_at = out.len();
    let placeholder_len = k::BYTE_RANGE_FIELD_WIDTH * 4 + 3;
    out.extend(std::iter::repeat_n(b' ', placeholder_len));
    out.extend_from_slice(b"]/Contents <");
    let contents_hex_start = out.len();
    out.extend(std::iter::repeat_n(b'0', k::SIGNATURE_CONTENTS_BYTES * 2));
    out.extend_from_slice(b">>>\nendobj\n");

    // Xref cùng kiểu với xref cuối của file gốc.
    let xref_offset = out.len();
    let mut trailer = Dictionary::new();
    trailer.set("Root", Object::Reference(root_id));
    if let Ok(info) = doc.trailer.get(b"Info") {
        trailer.set("Info", info.clone());
    }
    if let Ok(id) = doc.trailer.get(b"ID") {
        trailer.set("ID", id.clone());
    }
    trailer.set("Prev", prev_xref as i64);

    if uses_xref_stream {
        let xref_id: ObjectId = (next_id, 0);
        next_id += 1;
        offsets.insert(xref_id, xref_offset);
        let mut index = Vec::new();
        let mut rows = Vec::new();
        for (id, offset) in &offsets {
            index.push(Object::Integer(i64::from(id.0)));
            index.push(Object::Integer(1));
            rows.push(1u8);
            rows.extend_from_slice(&(*offset as u32).to_be_bytes());
            rows.extend_from_slice(&id.1.to_be_bytes());
        }
        trailer.set("Type", Object::Name(b"XRef".to_vec()));
        trailer.set("Size", i64::from(next_id));
        trailer.set("Index", index);
        trailer.set("W", vec![1.into(), 4.into(), 2.into()]);
        trailer.set("Length", rows.len() as i64);
        write!(ByteWriter(&mut out), "{} 0 obj\n", xref_id.0)?;
        write_object(&mut out, &Object::Dictionary(trailer));
        out.extend_from_slice(b"\nstream\n");
        out.extend_from_slice(&rows);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    } else {
        trailer.set("Size", i64::from(next_id));
        out.extend_from_slice(b"xref\n");
        for (id, offset) in &offsets {
            write!(ByteWriter(&mut out), "{} 1\n{offset:010} {:05} n\r\n", id.0, id.1)?;
        }
        out.extend_from_slice(b"trailer\n");
        write_object(&mut out, &Object::Dictionary(trailer));
        out.push(b'\n');
    }
    write!(ByteWriter(&mut out), "startxref\n{xref_offset}\n%%EOF\n")?;

    // /ByteRange: mỗi số rộng 10 ký tự, đệm khoảng trắng bên phải như bản cũ.
    let gap_start = contents_hex_start - 1;
    let gap_end = contents_hex_start + k::SIGNATURE_CONTENTS_BYTES * 2 + 1;
    let byte_range = [0, gap_start, gap_end, out.len() - gap_end];
    let text = byte_range
        .iter()
        .map(|v| format!("{v:<width$}", width = k::BYTE_RANGE_FIELD_WIDTH))
        .collect::<Vec<_>>()
        .join(" ");
    if text.len() != placeholder_len {
        return Err("/ByteRange thật dài khác chỗ trống đã chừa".into());
    }
    out[byte_range_at..byte_range_at + placeholder_len].copy_from_slice(text.as_bytes());

    Ok(Prepared {
        bytes: out,
        byte_range,
        contents_hex_start,
        xref_kind: if uses_xref_stream { "stream" } else { "table" },
    })
}

fn add_field(
    doc: &Document,
    form: &mut Dictionary,
    field_id: ObjectId,
    overrides: &mut BTreeMap<ObjectId, Object>,
) -> AnyResult<()> {
    match form.get(b"Fields").ok().cloned() {
        Some(Object::Reference(fields_id)) => {
            let mut fields = doc.get_object(fields_id)?.as_array()?.clone();
            fields.push(Object::Reference(field_id));
            overrides.insert(fields_id, Object::Array(fields));
        }
        Some(Object::Array(mut fields)) => {
            fields.push(Object::Reference(field_id));
            form.set("Fields", fields);
        }
        _ => form.set("Fields", vec![Object::Reference(field_id)]),
    }
    // Bản cũ không sửa /SigFlags đã có giá trị khác 3; ở đây luôn bật đủ hai bit.
    let flags = form.get(b"SigFlags").and_then(Object::as_i64).unwrap_or(0);
    form.set("SigFlags", flags | 3);
    Ok(())
}

fn last_startxref(bytes: &[u8]) -> AnyResult<usize> {
    let tail_start = bytes.len().saturating_sub(2048);
    let tail = &bytes[tail_start..];
    let at = tail
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or("Không tìm thấy startxref ở cuối file PDF")?;
    let digits: String = tail[at + 9..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|b| *b as char)
        .collect();
    Ok(digits.parse()?)
}

/// Chuỗi văn bản PDF: ASCII in được thì viết thường, còn lại viết UTF-16BE có BOM (như bản cũ).
fn pdf_text(text: &str) -> Object {
    if text.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, lopdf::StringFormat::Hexadecimal)
    }
}

/// `D:yyyyMMddHHmmss+07'00'` theo giờ Việt Nam.
fn pdf_date_now_vietnam() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + 7 * 3600;
    let days = seconds / 86400;
    let (hour, minute, second) = ((seconds % 86400) / 3600, (seconds % 3600) / 60, seconds % 60);
    // Thuật toán đổi số ngày sang năm-tháng-ngày (lịch Gregory).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("D:{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}+07'00'")
}

struct ByteWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for ByteWriter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// Ghi một object của lopdf ra cú pháp PDF. Không hỗ trợ stream (spike không cần ghi đè stream).
fn write_object(out: &mut Vec<u8>, object: &Object) {
    match object {
        Object::Null => out.extend_from_slice(b"null"),
        Object::Boolean(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
        Object::Integer(value) => out.extend_from_slice(value.to_string().as_bytes()),
        Object::Real(value) => out.extend_from_slice(value.to_string().as_bytes()),
        Object::Name(name) => {
            out.push(b'/');
            for byte in name {
                if (0x21..0x7F).contains(byte) && !b"()<>[]{}/%#".contains(byte) {
                    out.push(*byte);
                } else {
                    out.extend_from_slice(format!("#{byte:02X}").as_bytes());
                }
            }
        }
        Object::String(bytes, _) => {
            out.push(b'<');
            out.extend_from_slice(hex_upper(bytes).as_bytes());
            out.push(b'>');
        }
        Object::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                write_object(out, item);
            }
            out.push(b']');
        }
        Object::Dictionary(dict) => {
            out.extend_from_slice(b"<<");
            for (key, value) in dict.iter() {
                write_object(out, &Object::Name(key.clone()));
                out.push(b' ');
                write_object(out, value);
            }
            out.extend_from_slice(b">>");
        }
        Object::Reference(id) => out.extend_from_slice(format!("{} {} R", id.0, id.1).as_bytes()),
        Object::Stream(_) => out.extend_from_slice(b"null"),
    }
}

fn hex_upper(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02X}");
    }
    text
}

fn common_name(cert: &Certificate) -> String {
    let subject = cert.tbs_certificate().subject().to_string();
    subject
        .split(',')
        .find_map(|part| part.trim().strip_prefix("CN="))
        .unwrap_or(&subject)
        .to_owned()
}
