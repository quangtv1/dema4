//! Lời gọi CryptoAPI dựng chuỗi chứng thư cho [`super`]. Mọi `unsafe` của module nằm ở đây.

use std::ffi::c_void;

use windows::Win32::Foundation::FILETIME;
use windows::Win32::Security::Cryptography::{
    CERT_CHAIN_CONTEXT, CERT_CHAIN_ENGINE_CONFIG, CERT_CHAIN_PARA,
    CERT_CHAIN_REVOCATION_ACCUMULATIVE_TIMEOUT, CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
    CERT_CONTEXT, CERT_OPEN_STORE_FLAGS, CERT_REVOCATION_INFO, CERT_STORE_ADD_USE_EXISTING,
    CERT_STORE_PROV_MEMORY, CertAddEncodedCertificateToStore, CertCloseStore,
    CertCreateCertificateChainEngine, CertCreateCertificateContext, CertFreeCertificateChain,
    CertFreeCertificateChainEngine, CertFreeCertificateContext, CertGetCertificateChain,
    CertOpenStore, HCERTCHAINENGINE, HCERTSTORE, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
};

struct MemStore(HCERTSTORE);

impl Drop for MemStore {
    fn drop(&mut self) {
        // SAFETY: kho tạo bởi CertOpenStore, đóng đúng một lần.
        let _ = unsafe { CertCloseStore(Some(self.0), 0) };
    }
}

fn mem_store(certs: &[&[u8]]) -> Result<MemStore, String> {
    // SAFETY: kho trong bộ nhớ, không cần pvPara.
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_MEMORY,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            None,
            CERT_OPEN_STORE_FLAGS(0),
            None,
        )
    }
    .map(MemStore)
    .map_err(|e| e.message())?;
    for der in certs {
        // SAFETY: `der` là bản DER của một chứng thư; Windows chép dữ liệu vào kho.
        unsafe {
            CertAddEncodedCertificateToStore(
                Some(store.0),
                X509_ASN_ENCODING,
                der,
                CERT_STORE_ADD_USE_EXISTING,
                None,
            )
        }
        .map_err(|e| format!("Không thêm được chứng thư vào kho tạm: {}", e.message()))?;
    }
    Ok(store)
}

struct Engine(HCERTCHAINENGINE);

impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: engine tạo bởi CertCreateCertificateChainEngine, giải phóng một lần.
        unsafe { CertFreeCertificateChainEngine(Some(self.0)) };
    }
}

struct Leaf(*const CERT_CONTEXT);

impl Drop for Leaf {
    fn drop(&mut self) {
        // SAFETY: ngữ cảnh tạo bởi CertCreateCertificateContext, giải phóng một lần.
        let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
    }
}

struct Chain(*mut CERT_CHAIN_CONTEXT);

impl Drop for Chain {
    fn drop(&mut self) {
        // SAFETY: chuỗi trả về từ CertGetCertificateChain, giải phóng một lần.
        unsafe { CertFreeCertificateChain(self.0) };
    }
}

/// Kết quả thô của CertGetCertificateChain.
pub(super) struct RawChain {
    pub error_status: u32,
    /// DER của từng phần tử, từ chứng thư được kiểm tới gốc.
    pub elements: Vec<Vec<u8>>,
    /// Ngày thu hồi (giây Unix) sớm nhất trong các phần tử bị thu hồi, nếu Windows trả về mục CRL / OCSP.
    pub revoked_at: Option<i64>,
}

fn unix_of(t: FILETIME) -> i64 {
    let ticks = (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    i64::try_from(ticks / 10_000_000).unwrap_or(i64::MAX) - 11_644_473_600
}

fn filetime(unix: i64) -> FILETIME {
    let ticks = (unix + 11_644_473_600).max(0) as u64 * 10_000_000;
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

/// Dựng chuỗi cho `leaf` chỉ tin các gốc trong `roots` (kho gốc độc quyền), dùng `extra` làm CA trung gian.
pub(super) fn build(
    leaf: &[u8],
    roots: &[&[u8]],
    extra: &[&[u8]],
    at_unix: Option<i64>,
    revocation: bool,
    timeout_ms: u32,
) -> Result<RawChain, String> {
    let root_store = mem_store(roots)?;
    let extra_store = mem_store(extra)?;
    let config = CERT_CHAIN_ENGINE_CONFIG {
        cbSize: size_of::<CERT_CHAIN_ENGINE_CONFIG>() as u32,
        dwUrlRetrievalTimeout: timeout_ms,
        hExclusiveRoot: root_store.0,
        ..Default::default()
    };
    let mut handle = HCERTCHAINENGINE::default();
    // SAFETY: cấu hình hợp lệ với cbSize đầy đủ (Windows 8 trở lên đọc hExclusiveRoot); kho gốc sống lâu hơn engine.
    unsafe { CertCreateCertificateChainEngine(&config, &mut handle) }
        .map_err(|e| format!("Không tạo được bộ dựng chuỗi chứng thư: {}", e.message()))?;
    let engine = Engine(handle);

    // SAFETY: `leaf` là DER của chứng thư; ngữ cảnh được Leaf giải phóng.
    let leaf_ctx = unsafe { CertCreateCertificateContext(X509_ASN_ENCODING, leaf) };
    if leaf_ctx.is_null() {
        return Err("Chứng thư không đọc được.".into());
    }
    let leaf_ctx = Leaf(leaf_ctx);

    let para = CERT_CHAIN_PARA {
        cbSize: size_of::<CERT_CHAIN_PARA>() as u32,
        ..Default::default()
    };
    let time = at_unix.map(filetime);
    let flags = if revocation {
        CERT_CHAIN_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT | CERT_CHAIN_REVOCATION_ACCUMULATIVE_TIMEOUT
    } else {
        0
    };
    let mut chain: *mut CERT_CHAIN_CONTEXT = std::ptr::null_mut();
    // SAFETY: mọi handle hợp lệ tới cuối hàm; `chain` nhận con trỏ do Windows cấp, được Chain giải phóng.
    unsafe {
        CertGetCertificateChain(
            Some(engine.0),
            leaf_ctx.0,
            time.as_ref().map(|t| t as *const FILETIME),
            Some(extra_store.0),
            &para,
            flags,
            None::<*const c_void>,
            &mut chain,
        )
    }
    .map_err(|e| format!("Không dựng được chuỗi chứng thư: {}", e.message()))?;
    let chain = Chain(chain);

    // SAFETY: chuỗi hợp lệ tới khi Chain bị huỷ; chỉ đọc mảng do Windows cấp theo đúng số phần tử.
    unsafe {
        let ctx = &*chain.0;
        let mut elements = Vec::new();
        let mut revoked_at: Option<i64> = None;
        if ctx.cChain > 0 {
            let simple = &**ctx.rgpChain;
            for i in 0..simple.cElement as usize {
                let element = &**simple.rgpElement.add(i);
                let cert = &*element.pCertContext;
                elements.push(
                    std::slice::from_raw_parts(cert.pbCertEncoded, cert.cbCertEncoded as usize)
                        .to_vec(),
                );
                // Phần tử bị thu hồi: ngày thu hồi nằm trong mục CRL (Windows dựng cả khi trả lời bằng OCSP).
                if element.TrustStatus.dwErrorStatus & 0x4 != 0
                    && let Some(info) = element.pRevocationInfo.as_ref()
                    && info.cbSize as usize >= size_of::<CERT_REVOCATION_INFO>()
                    && let Some(crl) = info.pCrlInfo.as_ref()
                    && let Some(entry) = crl.pCrlEntry.as_ref()
                {
                    let at = unix_of(entry.RevocationDate);
                    revoked_at = Some(revoked_at.map_or(at, |r| r.min(at)));
                }
            }
        }
        Ok(RawChain {
            error_status: ctx.TrustStatus.dwErrorStatus,
            elements,
            revoked_at,
        })
    }
}
