//! Giờ Việt Nam (UTC+07:00 cố định) và số ngẫu nhiên cho mã sự kiện.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use smks_core::consts::app::VIETNAM_UTC_OFFSET_SECONDS;

/// Giây unix hiện tại.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// `(năm, tháng, ngày, giờ, phút, giây)` theo giờ Việt Nam.
fn civil(unix_seconds: i64) -> (i64, i64, i64, i64, i64, i64) {
    let seconds = unix_seconds + i64::from(VIETNAM_UTC_OFFSET_SECONDS);
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rest / 3600, (rest % 3600) / 60, rest % 60)
}

/// `20/11/2026 15:30:12` (hiển thị trong tin nhắn và giao diện).
pub fn display(unix_seconds: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix_seconds);
    format!("{d:02}/{mo:02}/{y:04} {h:02}:{mi:02}:{s:02}")
}

/// `2026-11-20T15:30:12+07:00` (nhật ký sự kiện).
pub fn iso(unix_seconds: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix_seconds);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}+07:00")
}

/// `20261120-153012` (mã giao dịch).
pub fn compact(unix_seconds: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix_seconds);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// `2026-11` (tên file nhật ký theo tháng).
pub fn month_key(unix_seconds: i64) -> String {
    let (y, mo, ..) = civil(unix_seconds);
    format!("{y:04}-{mo:02}")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// `N` byte ngẫu nhiên của hệ điều hành. Nếu hệ điều hành không cấp được (rất hiếm) thì trộn đồng hồ, bộ đếm và
/// khoá băm ngẫu nhiên của tiến trình: đủ để mã không trùng, không dùng cho mục đích bảo mật.
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    if getrandom::fill(&mut bytes).is_ok() {
        return bytes;
    }
    use std::hash::{BuildHasher, Hasher};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    for chunk in bytes.chunks_mut(8) {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
        let word = hasher.finish().to_le_bytes();
        for (b, w) in chunk.iter_mut().zip(word) {
            *b = w;
        }
    }
    bytes
}

/// Chuỗi hex thường của `N` byte ngẫu nhiên.
pub fn random_hex<const N: usize>() -> String {
    random_bytes::<N>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// UUID phiên bản 4 (ngẫu nhiên), dạng `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.
pub fn uuid_v4() -> String {
    let mut b = random_bytes::<16>();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Mã giao dịch: `<tiền tố>-<yyyymmdd-HHMMSS>-<6 hex>`, ví dụ `KS-20261120-153012-a1b2c3`.
pub fn transaction_id(prefix: &str) -> String {
    format!("{prefix}-{}-{}", compact(now()), random_hex::<3>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_utc_plus_7() {
        // 2026-11-18T07:17:57Z
        assert_eq!(display(1_794_986_277), "18/11/2026 14:17:57");
        assert_eq!(iso(1_794_986_277), "2026-11-18T14:17:57+07:00");
        assert_eq!(compact(1_794_986_277), "20261118-141757");
        // 2026-11-18T18:30:00Z đã sang ngày 19 theo giờ Việt Nam.
        assert_eq!(display(1_795_026_600), "19/11/2026 01:30:00");
        assert_eq!(month_key(1_709_225_999), "2024-02");
    }

    #[test]
    fn ids_are_well_formed_and_distinct() {
        let a = uuid_v4();
        let b = uuid_v4();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.as_bytes()[14], b'4');
        assert!(matches!(a.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        let tx = transaction_id("KS");
        assert!(tx.starts_with("KS-") && tx.len() == "KS-20261120-153012-a1b2c3".len());
        assert_eq!(random_hex::<4>().len(), 8);
    }
}
