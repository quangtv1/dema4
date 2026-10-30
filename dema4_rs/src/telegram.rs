//! Thông tin máy kèm số liệu thống kê gửi trạm license (trạm soạn tin, gửi nhóm Telegram — app không giữ token).

/// Tên máy tính (trên Windows là Computer name)
pub fn ten_pc() -> String {
    whoami::devicename().unwrap_or_else(|_| "?".into())
}

/// Địa chỉ MAC của card mạng chính, dạng AA:BB:CC:DD:EE:FF
pub fn dia_chi_mac() -> String {
    match mac_address::get_mac_address() {
        Ok(Some(mac)) => mac.to_string().to_uppercase(),
        _ => "?".into(),
    }
}

/// IP nội bộ (LAN) của card mạng ra Internet: "kết nối" UDP tới 1 địa chỉ ngoài để hệ điều hành chọn card,
/// không gửi gói tin nào. Không có mạng → "?".
pub fn ip_noi_bo() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("8.8.8.8:80").map(|_| s))
        .and_then(|s| s.local_addr())
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "?".into())
}

/// IP public (ra Internet) hỏi từ api.ipify.org, chờ tối đa 4 giây. Lỗi / không có mạng → "?". Gọi ở luồng nền.
pub fn ip_public() -> String {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(4))).build().into();
    agent
        .get("https://api.ipify.org")
        .call()
        .ok()
        .and_then(|mut r| r.body_mut().read_to_string().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= 45 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':'))
        .unwrap_or_else(|| "?".into())
}

/// Số liệu 1 lần tách (tin thống kê tách).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SoLieuTach {
    pub ho_so: usize,
    pub van_ban: usize,
    pub so_trang: usize,
    pub bo_index: usize,
    pub bo_trang: usize,
    pub split: usize,
    pub sao_nguyen: usize,
    pub ho_so_loi: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In IP nội bộ / public của máy đang chạy (cần mạng).
    #[test]
    #[ignore = "cần mạng"]
    fn in_ip_may() {
        let (a, b) = (ip_noi_bo(), ip_public());
        println!("IP nội bộ: {a} · IP public: {b}");
        assert!(a.parse::<std::net::IpAddr>().is_ok() && b.parse::<std::net::IpAddr>().is_ok());
    }
}
