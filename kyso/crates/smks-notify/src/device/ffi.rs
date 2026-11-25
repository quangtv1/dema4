//! Địa chỉ MAC của card mạng mang một địa chỉ IP (`GetAdaptersAddresses`). Mọi `unsafe` của module nằm ở đây.

use std::net::{IpAddr, Ipv4Addr};

use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

/// Địa chỉ MAC (6 byte) của card mạng đang có địa chỉ IPv4 `ip`.
pub(super) fn mac_for_ip(ip: IpAddr) -> Option<[u8; 6]> {
    let IpAddr::V4(wanted) = ip else {
        return None;
    };
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    for _ in 0..3 {
        // Bộ đệm u64 để đủ căn lề cho IP_ADAPTER_ADDRESSES_LH.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: `first` trỏ tới bộ đệm còn sống dài ít nhất `size` byte; hàm chỉ ghi trong phạm vi đó và báo cỡ
        // cần qua `size` khi thiếu.
        let rc = unsafe {
            GetAdaptersAddresses(u32::from(AF_INET.0), flags, None, Some(first), &mut size)
        };
        if rc == ERROR_BUFFER_OVERFLOW.0 {
            continue;
        }
        if rc != 0 {
            return None;
        }
        let mut adapter = first;
        while !adapter.is_null() {
            // SAFETY: danh sách liên kết do hệ điều hành dựng trong `buffer`; con trỏ khác null trỏ tới phần tử
            // hợp lệ chừng nào `buffer` còn sống.
            let a = unsafe { &*adapter };
            let mut unicast = a.FirstUnicastAddress;
            while !unicast.is_null() {
                // SAFETY: như trên, phần tử của danh sách địa chỉ nằm trong `buffer`.
                let u = unsafe { &*unicast };
                let sockaddr = u.Address.lpSockaddr;
                // SAFETY: `lpSockaddr` khác null trỏ tới SOCKADDR dài `iSockaddrLength`; chỉ đọc như SOCKADDR_IN
                // sau khi kiểm họ địa chỉ là AF_INET và đủ độ dài.
                let found = !sockaddr.is_null()
                    && u.Address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN>()
                    && unsafe { (*sockaddr).sa_family } == AF_INET
                    && Ipv4Addr::from(
                        unsafe { (*sockaddr.cast::<SOCKADDR_IN>()).sin_addr.S_un.S_addr }
                            .to_ne_bytes(),
                    ) == wanted;
                if found {
                    if a.PhysicalAddressLength != 6 {
                        return None;
                    }
                    let mut mac = [0u8; 6];
                    mac.copy_from_slice(&a.PhysicalAddress[..6]);
                    return (mac != [0u8; 6]).then_some(mac);
                }
                unicast = u.Next;
            }
            adapter = a.Next;
        }
        return None;
    }
    None
}
