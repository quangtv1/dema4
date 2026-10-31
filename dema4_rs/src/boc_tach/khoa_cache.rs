//! Khoá API / bí mật của Bóc tách lưu trong cache `.sm-dema4/boc_tach/khoa.bin`, mã hoá AES-256-GCM.
//! Khoá giải mã = SHA-256(bí mật lúc build ‖ mã máy): bí mật lấy từ biến `DEMA4_KHOA_CACHE` khi build (secret CI,
//! không có trong repo); gắn mã máy nên chép file sang máy khác không giải mã được. Build thiếu biến → không ghi
//! khoá ra đĩa (chỉ giữ trong phiên). Giới hạn: bí mật nằm trong file chạy — chống đọc trộm file cache, không
//! chống người dịch ngược app trên chính máy đó.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use rsa::sha2::{Digest, Sha256};

const BI_MAT: Option<&str> = option_env!("DEMA4_KHOA_CACHE");

/// Bản build này có lưu khoá ra đĩa được không.
pub fn co_luu() -> bool {
    BI_MAT.is_some_and(|s| !s.is_empty())
}

pub fn file_mac_dinh() -> PathBuf {
    crate::tach_vb::du_lieu_an::goc_du_lieu().join("boc_tach").join("khoa.bin")
}

fn khoa(bi_mat: &str, ma_may: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bi_mat.as_bytes());
    h.update([0]);
    h.update(ma_may.as_bytes());
    h.finalize().into()
}

/// base64(nonce 12 byte ‖ bản mã).
fn ma_hoa(k: &[u8; 32], ro: &[u8]) -> Option<String> {
    let mut nonce = [0u8; 12];
    getrandom::getrandom(&mut nonce).ok()?;
    let ma = Aes256Gcm::new(k.into()).encrypt(Nonce::from_slice(&nonce), ro).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode([&nonce[..], &ma].concat()))
}

fn giai_ma(k: &[u8; 32], s: &str) -> Option<Vec<u8>> {
    let b = base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()?;
    if b.len() < 12 {
        return None;
    }
    Aes256Gcm::new(k.into()).decrypt(Nonce::from_slice(&b[..12]), &b[12..]).ok()
}

/// Đọc các khoá (tên → giá trị); thiếu file / sai máy / sai bản build → rỗng (người dùng nhập lại).
pub fn doc_tu(f: &Path, bi_mat: &str, ma_may: &str) -> BTreeMap<String, String> {
    std::fs::read_to_string(f)
        .ok()
        .and_then(|s| giai_ma(&khoa(bi_mat, ma_may), &s))
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn ghi_vao(f: &Path, bi_mat: &str, ma_may: &str, ds: &BTreeMap<String, String>) -> std::io::Result<()> {
    if let Some(cha) = f.parent() {
        std::fs::create_dir_all(cha)?;
    }
    let ro = serde_json::to_vec(ds).map_err(std::io::Error::other)?;
    let s = ma_hoa(&khoa(bi_mat, ma_may), &ro).ok_or_else(|| std::io::Error::other("không mã hoá được khoá"))?;
    std::fs::write(f, s)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Khoá đã lưu của máy này (bản build không có bí mật → rỗng).
pub fn doc() -> BTreeMap<String, String> {
    match BI_MAT.filter(|s| !s.is_empty()) {
        Some(b) => doc_tu(&file_mac_dinh(), b, &crate::license::ma_may()),
        None => BTreeMap::new(),
    }
}

/// Ghi khoá; `Ok(false)` = bản build không lưu được (khoá chỉ dùng trong phiên).
pub fn ghi(ds: &BTreeMap<String, String>) -> std::io::Result<bool> {
    match BI_MAT.filter(|s| !s.is_empty()) {
        Some(b) => ghi_vao(&file_mac_dinh(), b, &crate::license::ma_may(), ds).map(|_| true),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ma_hoa_giai_ma_va_sai_may() {
        let d = std::env::temp_dir().join(format!("dema4_bt_khoacache_{}", std::process::id()));
        let f = d.join("khoa.bin");
        let ds = BTreeMap::from([("gemini".to_string(), "AIza-khoa-thu-123".to_string()), ("noi_bo".to_string(), String::new())]);
        ghi_vao(&f, "bi-mat-build", "MAY-A", &ds).unwrap();
        let tren_dia = std::fs::read_to_string(&f).unwrap();
        assert!(!tren_dia.contains("AIza") && !tren_dia.contains("gemini"), "file không chứa khoá rõ");
        assert_eq!(doc_tu(&f, "bi-mat-build", "MAY-A"), ds);
        assert!(doc_tu(&f, "bi-mat-build", "MAY-B").is_empty(), "chép sang máy khác không đọc được");
        assert!(doc_tu(&f, "bi-mat-khac", "MAY-A").is_empty(), "bản build khác không đọc được");
        // Ghi 2 lần ra 2 bản mã khác nhau (nonce ngẫu nhiên)
        ghi_vao(&f, "bi-mat-build", "MAY-A", &ds).unwrap();
        assert_ne!(std::fs::read_to_string(&f).unwrap(), tren_dia);
        std::fs::write(&f, "hỏng").unwrap();
        assert!(doc_tu(&f, "bi-mat-build", "MAY-A").is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }
}
