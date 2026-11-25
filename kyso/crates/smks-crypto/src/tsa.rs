//! Dấu thời gian RFC 3161 (kế hoạch 6.4, port `TimestampClient.cs`).
//!
//! - `TimeStampReq` v1, messageImprint SHA-256 của giá trị chữ ký, nonce 64-bit từ bộ sinh ngẫu nhiên của hệ
//!   điều hành, `certReq = true`; POST `application/timestamp-query` tới `http://tsa.ca.gov.vn/`.
//! - Tối đa 8 yêu cầu đồng thời, thời hạn 180 giây; lỗi mạng thì thử lại một lần, TSA từ chối thì không.
//! - Kiểm phản hồi: PKIStatus, nonce, messageImprint, chữ ký của TSA, EKU `timeStamping` (module `verify`).
//!   Chuỗi chứng thư TSA tới Root G2 kiểm ở `chain`.

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

use der::asn1::{Int, ObjectIdentifier, OctetString};
use der::{Decode, Encode};
use sha2::{Digest, Sha256};
use smks_core::consts::{oid, signing as k};
use smks_core::{ErrorCode, Result, SmksError};
use x509_tsp::{MessageImprint, TimeStampReq, TimeStampResp, TspVersion, TstInfo};

use crate::verify::{TimestampInfo, verify_timestamp_token};

fn fail(message: impl Into<String>) -> SmksError {
    SmksError::new(ErrorCode::TimestampInvalid, message)
}

/// Đèn hiệu đếm đơn giản giới hạn số yêu cầu đồng thời.
#[derive(Debug)]
struct Gate {
    free: Mutex<usize>,
    wake: Condvar,
}

impl Gate {
    fn enter(&self) -> GateTicket<'_> {
        let mut free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
        while *free == 0 {
            free = self.wake.wait(free).unwrap_or_else(PoisonError::into_inner);
        }
        *free -= 1;
        GateTicket(self)
    }
}

struct GateTicket<'a>(&'a Gate);

impl Drop for GateTicket<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        self.0.wake.notify_one();
    }
}

/// Máy khách TSA dùng chung cho cả lô ký.
#[derive(Debug)]
pub struct TsaClient {
    url: String,
    timeout: Duration,
    gate: Gate,
}

/// Token kèm thông tin đã kiểm.
#[derive(Debug, Clone)]
pub struct Timestamp {
    pub token_der: Vec<u8>,
    pub info: TimestampInfo,
}

impl Default for TsaClient {
    fn default() -> Self {
        Self::new(k::TSA_URL)
    }
}

impl TsaClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            timeout: Duration::from_secs(k::TSA_TIMEOUT_SECONDS),
            gate: Gate {
                free: Mutex::new(k::MAX_TSA_CONCURRENCY),
                wake: Condvar::new(),
            },
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Lấy dấu thời gian cho giá trị chữ ký `signature`.
    pub fn timestamp(&self, signature: &[u8]) -> Result<Timestamp> {
        self.request(signature, self.timeout, true)
    }

    /// Giờ của máy chủ TSA (TD-54): `genTime` (giây Unix) của một token trên dữ liệu ngẫu nhiên, đã kiểm như token
    /// thật (nonce, imprint, chữ ký của TSA). Thời hạn `timeout`, không thử lại để mất mạng thì biết sớm.
    pub fn server_time(&self, timeout: Duration) -> Result<i64> {
        let mut data = [0u8; 32];
        getrandom::fill(&mut data)
            .map_err(|e| fail(format!("Không sinh được số ngẫu nhiên: {e}")))?;
        Ok(self.request(&data, timeout, false)?.info.gen_time)
    }

    fn request(&self, signature: &[u8], timeout: Duration, retry: bool) -> Result<Timestamp> {
        let imprint: [u8; 32] = Sha256::digest(signature).into();
        let nonce = random_nonce()?;
        let request = TimeStampReq {
            version: TspVersion::V1,
            message_imprint: MessageImprint {
                hash_algorithm: spki::AlgorithmIdentifier {
                    oid: ObjectIdentifier::new(oid::SHA256).map_err(|e| fail(e.to_string()))?,
                    parameters: None,
                },
                hashed_message: OctetString::new(imprint.as_slice())
                    .map_err(|e| fail(e.to_string()))?,
            },
            req_policy: None,
            nonce: Some(nonce.clone()),
            cert_req: true,
            extensions: None,
        };
        let body = request
            .to_der()
            .map_err(|e| fail(format!("Không dựng được yêu cầu dấu thời gian: {e}")))?;

        let reply = {
            let _ticket = self.gate.enter();
            match self.post(&body, timeout) {
                Err(Transport::Network(first)) if !retry => {
                    return Err(fail(format!(
                        "Không kết nối được máy chủ dấu thời gian {}: {first}",
                        self.url
                    )));
                }
                Err(Transport::Network(first)) => self.post(&body, timeout).map_err(|e| match e {
                    Transport::Network(second) => fail(format!(
                        "Không kết nối được máy chủ dấu thời gian {} (đã thử 2 lần): {second}. Lần đầu: {first}",
                        self.url
                    )),
                    Transport::Rejected(m) => fail(m),
                })?,
                Err(Transport::Rejected(m)) => return Err(fail(m)),
                Ok(bytes) => bytes,
            }
        };
        let token_der = check_reply(&reply, &imprint, &nonce)?;
        let info = verify_timestamp_token(&token_der, signature)?;
        Ok(Timestamp { token_der, info })
    }

    fn post(&self, body: &[u8], timeout: Duration) -> std::result::Result<Vec<u8>, Transport> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .new_agent();
        let mut response = agent
            .post(&self.url)
            .header("Content-Type", k::TSA_CONTENT_TYPE)
            .send(body)
            .map_err(|e| Transport::Network(e.to_string()))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(Transport::Rejected(format!(
                "Máy chủ dấu thời gian trả về HTTP {status}. Không lấy được dấu thời gian."
            )));
        }
        response
            .body_mut()
            .with_config()
            .limit(1024 * 1024)
            .read_to_vec()
            .map_err(|e| Transport::Network(e.to_string()))
    }
}

enum Transport {
    /// Lỗi mạng: được thử lại.
    Network(String),
    /// Máy chủ trả lời nhưng từ chối: không thử lại.
    Rejected(String),
}

fn random_nonce() -> Result<Int> {
    let mut bytes = [0u8; k::TSA_NONCE_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| fail(format!("Không sinh được số ngẫu nhiên: {e}")))?;
    // Số dương, không có byte 0 thừa ở đầu.
    bytes[0] = (bytes[0] & 0x7F) | 0x40;
    Int::new(&bytes).map_err(|e| fail(e.to_string()))
}

/// Kiểm PKIStatus, imprint và nonce; trả về DER của TimeStampToken.
fn check_reply(reply: &[u8], imprint: &[u8; 32], nonce: &Int) -> Result<Vec<u8>> {
    let response = TimeStampResp::from_der(reply).map_err(|e| {
        fail(format!(
            "Phản hồi của máy chủ dấu thời gian không đọc được: {e}"
        ))
    })?;
    let status = response.status.status as u8;
    // 0 granted, 1 grantedWithMods.
    if status > 1 {
        let text = response
            .status
            .status_string
            .as_ref()
            .map(|s| format!("{s:?}"))
            .unwrap_or_default();
        return Err(fail(format!(
            "Máy chủ dấu thời gian từ chối (PKIStatus = {status}) {text}"
        )));
    }
    let token = response
        .time_stamp_token
        .ok_or_else(|| fail("Phản hồi của máy chủ dấu thời gian không có token."))?;
    let token_der = token.to_der().map_err(|e| fail(e.to_string()))?;
    let sd = crate::verify::parse_signed_data(&token_der).map_err(|e| fail(e.message))?;
    let econtent = sd
        .encap_content_info
        .econtent
        .as_ref()
        .ok_or_else(|| fail("Token dấu thời gian không có TSTInfo."))?;
    let octets: OctetString = econtent
        .decode_as()
        .map_err(|e: der::Error| fail(e.to_string()))?;
    let tst = TstInfo::from_der(octets.as_bytes()).map_err(|e| fail(e.to_string()))?;
    if tst.message_imprint.hashed_message.as_bytes() != imprint {
        return Err(fail(
            "Dấu thời gian trả về không khớp yêu cầu (messageImprint).",
        ));
    }
    if tst.nonce.as_ref() != Some(nonce) {
        return Err(fail("Dấu thời gian trả về không khớp yêu cầu (nonce)."));
    }
    Ok(token_der)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_is_positive_and_random() {
        let a = random_nonce().unwrap();
        let b = random_nonce().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.as_bytes().len(), k::TSA_NONCE_BYTES);
        assert!(a.as_bytes()[0] & 0x80 == 0);
    }

    #[test]
    fn gate_limits_concurrency() {
        let gate = std::sync::Arc::new(Gate {
            free: Mutex::new(2),
            wake: Condvar::new(),
        });
        let active = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let threads: Vec<_> = (0..6)
            .map(|_| {
                let (gate, active, peak) = (gate.clone(), active.clone(), peak.clone());
                std::thread::spawn(move || {
                    let _t = gate.enter();
                    let now = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    peak.fetch_max(now, std::sync::atomic::Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(30));
                    active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(peak.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn unreachable_server_is_reported_after_retry() {
        let client = TsaClient {
            url: "http://127.0.0.1:9/".into(),
            timeout: Duration::from_secs(3),
            gate: Gate {
                free: Mutex::new(1),
                wake: Condvar::new(),
            },
        };
        let err = client.timestamp(b"chu ky").unwrap_err();
        assert_eq!(err.code, ErrorCode::TimestampInvalid);
        assert!(err.message.contains("đã thử 2 lần"), "{}", err.message);
        // Hỏi giờ: không thử lại.
        let err = client.server_time(Duration::from_secs(3)).unwrap_err();
        assert!(
            err.message.contains("Không kết nối được"),
            "{}",
            err.message
        );
        assert!(!err.message.contains("đã thử 2 lần"), "{}", err.message);
    }

    /// Hỏi giờ TSA thật của Ban Cơ yếu: `cargo test -p smks-crypto -- --ignored real_tsa_time`.
    #[test]
    #[ignore]
    fn real_tsa_time_is_close_to_clock() {
        let t = TsaClient::default()
            .server_time(Duration::from_secs(15))
            .unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        println!("genTime {t}, máy {now}, lệch {}s", t - now);
        assert!((t - now).abs() < 24 * 3600);
    }

    /// Gọi TSA thật của Ban Cơ yếu: `cargo test -p smks-crypto -- --ignored real_tsa`.
    #[test]
    #[ignore]
    fn real_tsa_issues_verifiable_token() {
        let client = TsaClient::default();
        let stamp = client.timestamp(b"gia tri chu ky thu").unwrap();
        println!(
            "TSA: {} | genTime {} | serial {} | policy {} | token {} byte",
            stamp.info.tsa.common_name(),
            stamp.info.gen_time,
            stamp.info.serial_hex,
            stamp.info.policy,
            stamp.token_der.len()
        );
        assert!(stamp.info.tsa.has_eku(oid::EKU_TIME_STAMPING));
        // Lưu chứng thư công khai của TSA làm fixture cho test chuỗi (đặt biến SMKS_SAVE_TSA_CERT=<đường dẫn>).
        if let Ok(path) = std::env::var("SMKS_SAVE_TSA_CERT") {
            std::fs::write(path, stamp.info.tsa.der()).unwrap();
        }
        // Token phủ đúng chữ ký đã gửi, không phủ chữ ký khác.
        assert!(verify_timestamp_token(&stamp.token_der, b"gia tri khac").is_err());
    }
}
