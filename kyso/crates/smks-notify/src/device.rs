//! Mã thiết bị và thông tin thiết bị ở mức cần cho quản trị.
//!
//! - Mã thiết bị là số ngẫu nhiên sinh một lần, lưu trong thư mục dữ liệu của ứng dụng. **Không** suy ra từ phần
//!   cứng: không đọc serial ổ cứng, mã bo mạch hay bất kỳ mã định danh phần cứng nào.
//! - Chỉ thu thập: tên máy, tên và phiên bản hệ điều hành, kiến trúc CPU, phiên bản ứng dụng; IP nội bộ và địa chỉ
//!   MAC **chỉ khi người dùng cho phép** ở hộp đăng ký.
//! - Chỉ đọc khi người dùng mở hộp đăng ký / cấu hình hoặc khi gửi đăng ký; không theo dõi nền.

use std::net::{IpAddr, UdpSocket};

use serde::Serialize;

use crate::event::Metadata;
use crate::time;

#[cfg(windows)]
#[allow(unsafe_code)]
mod ffi;

/// Mã thiết bị mới: `SMKS-` + UUID ngẫu nhiên.
pub fn new_device_id() -> String {
    format!("SMKS-{}", time::uuid_v4().to_ascii_uppercase())
}

/// Thông tin thiết bị gửi kèm sự kiện đăng ký.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub hostname: String,
    pub os: String,
    pub os_version: String,
    pub architecture: String,
    pub app_version: String,
    /// Rỗng khi người dùng không cho gửi, hoặc máy không có mạng.
    pub local_ip: String,
    /// Rỗng khi người dùng không cho gửi.
    pub mac_address: String,
}

/// IP nội bộ của card mạng đi ra ngoài. `connect` trên socket UDP chỉ chọn tuyến, không gửi gói nào.
fn local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    // 192.0.2.1 thuộc dải TEST-NET-1 (RFC 5737), không phải máy thật.
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

fn format_mac(bytes: [u8; 6]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(windows)]
fn os_name_and_version() -> (String, String) {
    let key = windows_registry::LOCAL_MACHINE
        .open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .ok();
    let text = |name: &str| {
        key.as_ref()
            .and_then(|k| k.get_string(name).ok())
            .unwrap_or_default()
    };
    let build = text("CurrentBuildNumber");
    let mut name = text("ProductName");
    // Windows 11 vẫn ghi ProductName là "Windows 10 …"; phân biệt bằng số build.
    if build.parse::<u32>().is_ok_and(|b| b >= 22_000) {
        name = name.replacen("Windows 10", "Windows 11", 1);
    }
    if name.is_empty() {
        name = "Windows".into();
    }
    let display = text("DisplayVersion");
    let ubr = key
        .as_ref()
        .and_then(|k| k.get_u32("UBR").ok())
        .map(|n| format!(".{n}"))
        .unwrap_or_default();
    let version = match (display.is_empty(), build.is_empty()) {
        (false, false) => format!("{display} (build {build}{ubr})"),
        (true, false) => format!("build {build}{ubr}"),
        _ => display,
    };
    (name, version)
}

#[cfg(not(windows))]
fn os_name_and_version() -> (String, String) {
    (std::env::consts::OS.to_owned(), String::new())
}

/// Kiến trúc CPU của máy (không phải của file chạy): `AMD64`, `ARM64`…
fn architecture() -> String {
    std::env::var("PROCESSOR_ARCHITEW6432")
        .or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE"))
        .unwrap_or_else(|_| std::env::consts::ARCH.to_owned())
}

/// Tên máy tính.
pub fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

impl DeviceInfo {
    /// Đọc thông tin thiết bị. `include_ip` / `include_mac`: lựa chọn của người dùng ở hộp đăng ký.
    pub fn collect(app_version: &str, include_ip: bool, include_mac: bool) -> Self {
        let (os, os_version) = os_name_and_version();
        let ip = (include_ip || include_mac).then(local_ip).flatten();
        #[cfg(windows)]
        let mac = include_mac
            .then(|| ip.and_then(ffi::mac_for_ip))
            .flatten()
            .map(format_mac)
            .unwrap_or_default();
        #[cfg(not(windows))]
        let mac = String::new();
        Self {
            hostname: hostname(),
            os,
            os_version,
            architecture: architecture(),
            app_version: app_version.to_owned(),
            local_ip: if include_ip {
                ip.map(|i| i.to_string()).unwrap_or_default()
            } else {
                String::new()
            },
            mac_address: mac,
        }
    }

    /// Chép vào dữ liệu kèm sự kiện đăng ký.
    pub fn fill(&self, metadata: &mut Metadata) {
        metadata.hostname.clone_from(&self.hostname);
        metadata.os.clone_from(&self.os);
        metadata.os_version.clone_from(&self.os_version);
        metadata.architecture.clone_from(&self.architecture);
        metadata.app_version.clone_from(&self.app_version);
        metadata.local_ip.clone_from(&self.local_ip);
        metadata.mac_address.clone_from(&self.mac_address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_random_not_hardware_derived() {
        let a = new_device_id();
        let b = new_device_id();
        assert_ne!(a, b, "hai lần sinh trên cùng máy phải khác nhau");
        assert!(a.starts_with("SMKS-") && a.len() == 41);
    }

    #[test]
    fn optional_fields_are_empty_unless_allowed() {
        let basic = DeviceInfo::collect("1.5.0", false, false);
        assert_eq!(basic.app_version, "1.5.0");
        assert!(basic.local_ip.is_empty() && basic.mac_address.is_empty());
        assert!(!basic.os.is_empty() && !basic.architecture.is_empty());

        let full = DeviceInfo::collect("1.5.0", true, true);
        // Máy không có mạng thì cả hai vẫn rỗng; có mạng thì đúng dạng.
        if !full.mac_address.is_empty() {
            assert_eq!(full.mac_address.len(), 17);
            assert!(!full.local_ip.is_empty());
        }
        assert_eq!(
            format_mac([0, 0x1A, 0x2B, 0x3C, 0x4D, 0xFF]),
            "00-1A-2B-3C-4D-FF"
        );
    }
}
