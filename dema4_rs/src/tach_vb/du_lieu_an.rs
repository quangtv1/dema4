//! Dữ liệu làm dở của từng thư mục đầu vào (nhãn, góc xoay, log tách, cache phân tích, kết quả bóc tách), trong thư mục
//! người dùng của app (không phụ thuộc chỗ đặt file chạy): `<gốc>/viec/<tên thư mục>-<băm đường dẫn>/`, gốc =
//! `%LOCALAPPDATA%\sm-dema4` / `~/Library/Application Support/sm-dema4`. Mỗi thư mục đầu vào 1 thư mục con riêng nên
//! phân tích nhiều thư mục (kể cả thư mục cha / con của nhau, hay trùng tên ở chỗ khác) không ảnh hưởng nhau,
//! và thư mục đầu vào không bị ghi gì thêm. Bản cũ (`<thư mục đầu vào>/.sm-dema4/`) được chuyển sang khi mở.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const TEN_AN: &str = ".sm-dema4";
/// File ghi đường dẫn đầy đủ của thư mục đầu vào (để người xem biết thư mục con nào của thư mục nào).
const FILE_GOC: &str = "thu_muc_goc.txt";

/// Thư mục dữ liệu làm dở của thư mục đầu vào `goc` (`<gốc>/viec/<tên>-<băm>`).
pub fn thu_muc_an(goc: &Path) -> PathBuf {
    let duong_dan = chuan_hoa(goc);
    let ten: String = Path::new(&duong_dan)
        .file_name()
        .map(|t| t.to_string_lossy().chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') { c } else { '_' }).take(40).collect())
        .unwrap_or_default();
    // Windows không phân biệt hoa thường trong đường dẫn: băm theo chữ thường, tên hiển thị giữ nguyên
    let khoa = if cfg!(windows) { duong_dan.to_lowercase() } else { duong_dan };
    goc_du_lieu().join("viec").join(format!("{}-{:016x}", ten.trim(), bam(&khoa)))
}

/// Chuyển dữ liệu cũ trong `<goc>/.sm-dema4/` sang chỗ mới (1 lần) rồi xoá bản cũ; ghi `thu_muc_goc.txt`.
/// Gọi trong luồng nền trước khi đọc cache / nhãn.
pub fn chuyen_du_lieu_cu(goc: &Path) {
    static KHOA: Mutex<()> = Mutex::new(());
    let _khoa = KHOA.lock().unwrap_or_else(|e| e.into_inner());
    let (cu, moi) = (goc.join(TEN_AN), thu_muc_an(goc));
    if cu.is_dir() {
        if !moi.exists() && chep_thu_muc(&cu, &moi).is_err() {
            let _ = std::fs::remove_dir_all(&moi); // chép dở: bỏ, lần sau thử lại, bản cũ vẫn còn
            return;
        }
        let _ = std::fs::remove_dir_all(&cu);
    }
    if moi.is_dir() && !moi.join(FILE_GOC).exists() {
        let _ = std::fs::write(moi.join(FILE_GOC), chuan_hoa(goc));
    }
}

/// Đường dẫn tuyệt đối thống nhất (cùng thư mục gõ khác kiểu → cùng chuỗi), giữ nguyên hoa thường.
fn chuan_hoa(goc: &Path) -> String {
    let p = std::fs::canonicalize(goc).or_else(|_| std::path::absolute(goc)).unwrap_or_else(|_| goc.to_path_buf());
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).trim_end_matches(['/', '\\']).to_string()
}

/// FNV-1a 64 bit: ổn định giữa các lần chạy / phiên bản (khác `DefaultHasher`).
fn bam(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// Gốc dữ liệu của app = thư mục người dùng (`cau_hinh::thu_muc_nguoi_dung`); không ghi được → thư mục tạm
/// (`goc_la_tam()` = true để giao diện báo người dùng: dữ liệu có thể mất khi dọn máy).
pub(crate) fn goc_du_lieu() -> &'static Path {
    &goc().0
}

/// Gốc đang là thư mục tạm dự phòng (thư mục người dùng không ghi được).
pub(crate) fn goc_la_tam() -> bool {
    goc().1
}

fn goc() -> &'static (PathBuf, bool) {
    static GOC: OnceLock<(PathBuf, bool)> = OnceLock::new();
    GOC.get_or_init(|| chon_goc(crate::cau_hinh::thu_muc_nguoi_dung(), &std::env::temp_dir()))
}

/// Chọn gốc: thư mục người dùng nếu ghi được, không thì `<tam>/sm-dema4` (cờ dự phòng = true).
fn chon_goc(nguoi_dung: Option<PathBuf>, tam: &Path) -> (PathBuf, bool) {
    match nguoi_dung.filter(|p| ghi_duoc(p)) {
        Some(p) => (p, false),
        None => (tam.join("sm-dema4"), true),
    }
}

fn ghi_duoc(p: &Path) -> bool {
    let thu = p.join(".ghi_thu");
    let ok = std::fs::create_dir_all(p).is_ok() && std::fs::write(&thu, b"").is_ok();
    let _ = std::fs::remove_file(&thu);
    ok
}

fn chep_thu_muc(tu: &Path, den: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(den)?;
    for muc in std::fs::read_dir(tu)? {
        let muc = muc?;
        let dich = den.join(muc.file_name());
        if muc.file_type()?.is_dir() {
            chep_thu_muc(&muc.path(), &dich)?;
        } else {
            std::fs::copy(muc.path(), dich)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moi_thu_muc_dau_vao_mot_cho_rieng_ngoai_thu_muc_dau_vao() {
        let goc = std::env::temp_dir().join(format!("dema4_an_{}", std::process::id()));
        let (a, b, a_con) = (goc.join("x/HS"), goc.join("y/HS"), goc.join("x/HS/con"));
        std::fs::create_dir_all(&a_con).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let (da, db, dc) = (thu_muc_an(&a), thu_muc_an(&b), thu_muc_an(&a_con));
        assert!(da != db && da != dc && db != dc, "trùng tên / cha-con vẫn tách riêng");
        assert!(!da.starts_with(&goc), "không ghi vào thư mục đầu vào: {}", da.display());
        assert_eq!(da, thu_muc_an(&goc.join("x/./HS/")), "cùng thư mục gõ khác kiểu → cùng chỗ");
        assert!(da.file_name().unwrap().to_string_lossy().starts_with("HS-"));
        let _ = std::fs::remove_dir_all(&goc);
    }

    #[test]
    fn goc_du_lieu_o_thu_muc_nguoi_dung_khong_canh_exe_test_khong_cham_thu_muc_that() {
        let goc = goc_du_lieu();
        assert_eq!(Some(goc.to_path_buf()), crate::cau_hinh::thu_muc_nguoi_dung(), "gốc = thư mục người dùng của app");
        assert!(goc.starts_with(std::env::temp_dir()), "test chỉ ghi trong thư mục tạm: {}", goc.display());
        let exe = std::env::current_exe().unwrap();
        assert!(!goc.starts_with(exe.parent().unwrap()), "không còn ghi cạnh file chạy");
        assert!(thu_muc_an(Path::new("/x/HS")).starts_with(goc.join("viec")), "dữ liệu làm dở nằm dưới viec/");
        assert!(!goc_la_tam(), "gốc ghi được → không phải nhánh dự phòng");
    }

    #[test]
    fn thu_muc_nguoi_dung_khong_ghi_duoc_thi_dung_thu_muc_tam_va_bao() {
        let d = std::env::temp_dir().join(format!("dema4_chon_goc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // "Thư mục người dùng" là 1 file thường → không tạo / ghi được bên trong
        std::fs::write(d.join("khong_ghi_duoc"), "").unwrap();
        assert_eq!(chon_goc(Some(d.join("khong_ghi_duoc/sm-dema4")), &d), (d.join("sm-dema4"), true));
        assert_eq!(chon_goc(None, &d), (d.join("sm-dema4"), true), "không biết thư mục người dùng (thiếu biến môi trường)");
        assert_eq!(chon_goc(Some(d.join("nd/sm-dema4")), &d), (d.join("nd/sm-dema4"), false));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn du_lieu_cu_canh_file_chay_khong_doc_khong_xoa() {
        let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let cu = exe_dir.join(TEN_AN).join(format!("HS-{}", std::process::id()));
        std::fs::create_dir_all(&cu).unwrap();
        std::fs::write(cu.join("index_and_blank.txt"), "a.pdf;2;index\n").unwrap();
        let moi = thu_muc_an(Path::new("/x/HS"));
        assert!(!moi.starts_with(&exe_dir), "cài mới: không dùng `.sm-dema4` cạnh file chạy");
        assert!(cu.join("index_and_blank.txt").exists(), "dữ liệu cũ cạnh file chạy để nguyên");
        let _ = std::fs::remove_dir_all(&cu);
    }

    #[test]
    fn chuyen_du_lieu_cu_sang_cho_moi_roi_xoa_ban_cu() {
        let goc = std::env::temp_dir().join(format!("dema4_an_cu_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(goc.join(".sm-dema4/cache")).unwrap();
        std::fs::write(goc.join(".sm-dema4/index_and_blank.txt"), "a.pdf;2;index\n").unwrap();
        std::fs::write(goc.join(".sm-dema4/cache/a.bin"), [1u8, 2, 3]).unwrap();
        let moi = thu_muc_an(&goc);
        let _ = std::fs::remove_dir_all(&moi);
        chuyen_du_lieu_cu(&goc);
        assert!(!goc.join(".sm-dema4").exists(), "bản cũ trong thư mục đầu vào đã xoá");
        assert_eq!(std::fs::read_to_string(moi.join("index_and_blank.txt")).unwrap(), "a.pdf;2;index\n");
        assert_eq!(std::fs::read(moi.join("cache/a.bin")).unwrap(), [1, 2, 3]);
        assert_eq!(std::fs::read_to_string(moi.join(FILE_GOC)).unwrap(), chuan_hoa(&goc));
        let _ = std::fs::remove_dir_all(&moi);
        let _ = std::fs::remove_dir_all(&goc);
    }
}
