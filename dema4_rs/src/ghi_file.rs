//! Ghi file an toàn dùng chung: ghi ra file tạm cạnh đích rồi đổi tên đè — tắt app / mất điện giữa chừng
//! thì file đích vẫn là bản cũ nguyên vẹn hoặc bản mới đầy đủ, không bao giờ nửa vời.

use std::path::Path;

/// Ghi `noi_dung` vào `p` qua file tạm `<p>.tmp` + đổi tên; tạo thư mục cha nếu chưa có.
pub fn ghi_an_toan(p: &Path, noi_dung: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(cha) = p.parent() {
        std::fs::create_dir_all(cha)?;
    }
    let mut ten_tam = p.as_os_str().to_owned();
    ten_tam.push(".tmp");
    let tam = std::path::PathBuf::from(ten_tam);
    std::fs::write(&tam, noi_dung)?;
    std::fs::rename(&tam, p).inspect_err(|_| {
        let _ = std::fs::remove_file(&tam);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghi_tao_thu_muc_ghi_de_va_khong_de_file_tam() {
        let d = std::env::temp_dir().join(format!("dema4_ghi_file_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let f = d.join("con/nhan.txt");
        ghi_an_toan(&f, "a").unwrap();
        ghi_an_toan(&f, "b").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "b");
        assert!(!d.join("con/nhan.txt.tmp").exists(), "không để lại file tạm");
        let _ = std::fs::remove_dir_all(&d);
    }
}
