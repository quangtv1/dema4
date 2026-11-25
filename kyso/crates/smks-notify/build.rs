//! Nhúng cấu hình thông báo vào file chạy (yêu cầu người phụ trách ngày 24/11/2026: chỉ phát một file exe).
//!
//! Khi build với biến môi trường `SMKS_NOTIFY_EMBEDDED_FILE=<đường dẫn notify.json>`, nội dung file được nhúng vào
//! `smks-notify` (và vào `SaoMai-KySo.exe`). File nằm ngoài repo; mã nguồn và git không chứa Bot Token. Nội dung được
//! làm rối bằng khoá ngẫu nhiên sinh mỗi lần build để không đọc được bằng cách tìm chuỗi trong exe. Đây chỉ là làm
//! rối, không phải mã hoá: ai có exe và đủ công sức vẫn lấy lại được token (xem `docs/THONG_BAO_TELEGRAM.md`).
//! Không đặt biến thì không nhúng gì.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

const VAR: &str = "SMKS_NOTIFY_EMBEDDED_FILE";

fn random_key() -> Vec<u8> {
    let mut key = Vec::with_capacity(32);
    for i in 0..4u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        key.extend_from_slice(&h.finish().to_le_bytes());
    }
    key
}

fn main() {
    println!("cargo:rerun-if-env-changed={VAR}");
    let out = match std::env::var_os("OUT_DIR") {
        Some(dir) => std::path::PathBuf::from(dir).join("notify_embedded.bin"),
        None => {
            eprintln!("build.rs của smks-notify: thiếu OUT_DIR");
            std::process::exit(1);
        }
    };
    let mut blob = Vec::new();
    if let Some(path) = std::env::var_os(VAR).filter(|p| !p.is_empty()) {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "build.rs của smks-notify: không đọc được {VAR} ({}): {e}",
                    path.to_string_lossy()
                );
                std::process::exit(1);
            }
        };
        let key = random_key();
        blob.extend_from_slice(&key);
        blob.extend(
            data.iter()
                .enumerate()
                .map(|(i, b)| b ^ key[i % key.len()] ^ (i as u8).wrapping_mul(31)),
        );
    }
    if let Err(e) = std::fs::write(&out, &blob) {
        eprintln!(
            "build.rs của smks-notify: không ghi được {}: {e}",
            out.display()
        );
        std::process::exit(1);
    }
}
