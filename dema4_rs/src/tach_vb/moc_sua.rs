//! Mốc sửa từng hồ sơ (`moc_sua.txt` trong thư mục dữ liệu làm dở): `khoá;unix_ms;chữ_ký_hex` mỗi dòng.
//! Dùng để gộp phiên làm việc giữa 2 máy theo luật "bản sửa sau cùng thắng" (cho mọi thay đổi, kể cả nhãn tự động).
//!
//! Chữ ký = băm nội dung của hồ sơ (dòng nhãn + góc xoay + cờ duyệt từng trang). Mốc chỉ tăng khi chữ ký **khác**
//! bản đã lưu, nên ghi lại y nguyên không đổi mốc. Hồ sơ từng có mục mà nay không còn gì (xoá hết nhãn, Làm lại)
//! được giữ dạng **bia mộ** (chữ ký `CHU_KY_RONG`, mốc mới) — gộp sẽ không hồi lại nhãn cũ từ máy kia.

use std::collections::BTreeMap;
use std::path::Path;

pub const TEN_FILE: &str = "moc_sua.txt";
/// Chữ ký của hồ sơ không còn nhãn / xoay / cờ duyệt nào.
pub const CHU_KY_RONG: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moc {
    /// Thời điểm sửa (unix ms, đồng hồ máy đã sửa).
    pub luc: u64,
    pub chu_ky: u64,
}

/// FNV-1a 64 của chuỗi mô tả nội dung hồ sơ; chuỗi rỗng → `CHU_KY_RONG`.
pub fn chu_ky(noi_dung: &str) -> u64 {
    if noi_dung.is_empty() {
        return CHU_KY_RONG;
    }
    let h = noi_dung.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    if h == CHU_KY_RONG { 1 } else { h }
}

/// Đọc file mốc; dòng hỏng bỏ qua, file không có → rỗng.
pub fn doc(f: &Path) -> BTreeMap<String, Moc> {
    let Ok(s) = std::fs::read_to_string(f) else { return BTreeMap::new() };
    s.lines()
        .filter_map(|d| {
            let mut c = d.rsplitn(3, ';');
            let ck = u64::from_str_radix(c.next()?.trim(), 16).ok()?;
            let luc = c.next()?.trim().parse().ok()?;
            let khoa = c.next()?;
            (!khoa.is_empty()).then(|| (super::nhan::chuan_duong_dan(khoa), Moc { luc, chu_ky: ck }))
        })
        .collect()
}

fn noi_dung(m: &BTreeMap<String, Moc>) -> String {
    m.iter().map(|(k, v)| format!("{k};{};{:x}\n", v.luc, v.chu_ky)).collect()
}

/// Gộp chữ ký hiện tại vào mốc đã lưu: hồ sơ có chữ ký khác → mốc = `bay_gio`; hồ sơ trong file mà không còn
/// trong `hien_tai` → bia mộ. Chỉ ghi file khi có thay đổi; trả `true` nếu đã ghi.
pub fn cap_nhat(f: &Path, hien_tai: &BTreeMap<String, u64>, bay_gio: u64) -> std::io::Result<bool> {
    let mut m = doc(f);
    let mut doi = false;
    for (k, &ck) in hien_tai {
        match m.get(k) {
            Some(cu) if cu.chu_ky == ck => {}
            None if ck == CHU_KY_RONG => {} // chưa từng có gì, nay vẫn không có → không cần mục
            _ => {
                m.insert(k.clone(), Moc { luc: bay_gio, chu_ky: ck });
                doi = true;
            }
        }
    }
    for (k, cu) in m.iter_mut() {
        if !hien_tai.contains_key(k) && cu.chu_ky != CHU_KY_RONG {
            *cu = Moc { luc: bay_gio, chu_ky: CHU_KY_RONG };
            doi = true;
        }
    }
    if doi {
        crate::ghi_file::ghi_an_toan(f, noi_dung(&m))?;
    }
    Ok(doi)
}

/// Chữ ký từng hồ sơ chỉ từ nhãn (dòng lệnh: chưa có góc xoay / cờ duyệt). Cùng cách viết với `BangTrang::chu_ky_ho_so`.
pub fn chu_ky_tu_nhan(bang: &super::nhan::BangNhan) -> BTreeMap<String, u64> {
    bang.iter()
        .map(|(k, trang)| (k.clone(), chu_ky(&trang.iter().filter_map(|(so, n)| n.chu().map(|c| format!("n{so}:{c};"))).collect::<String>())))
        .collect()
}

/// Unix ms hiện tại.
pub fn bay_gio_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tam(ten: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("dema4_moc_{ten}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join(TEN_FILE)
    }

    fn bang(ds: &[(&str, &str)]) -> BTreeMap<String, u64> {
        ds.iter().map(|(k, nd)| (k.to_string(), chu_ky(nd))).collect()
    }

    #[test]
    fn chi_ho_so_doi_noi_dung_moi_tang_moc() {
        let f = tam("doi");
        assert!(cap_nhat(&f, &bang(&[("A.pdf", "2:index"), ("B.pdf", "3:blank"), ("C.pdf", "1:index")]), 100).unwrap());
        assert!(!cap_nhat(&f, &bang(&[("A.pdf", "2:index"), ("B.pdf", "3:blank"), ("C.pdf", "1:index")]), 200).unwrap(), "ghi lại y nguyên → không đổi mốc, không ghi file");
        cap_nhat(&f, &bang(&[("A.pdf", "2:index"), ("B.pdf", "3:blank;b:4:2"), ("C.pdf", "1:index")]), 300).unwrap();
        let m = doc(&f);
        assert_eq!((m["A.pdf"].luc, m["B.pdf"].luc, m["C.pdf"].luc), (100, 300, 100), "chỉ B (đổi cờ duyệt) tăng mốc");
        let _ = std::fs::remove_dir_all(f.parent().unwrap());
    }

    #[test]
    fn xoa_het_nhan_thanh_bia_mo_khong_mat_muc() {
        let f = tam("bia_mo");
        cap_nhat(&f, &bang(&[("A.pdf", "2:index"), ("B.pdf", "3:blank")]), 100).unwrap();
        // A bị xoá hết nhãn: không còn trong bảng (hoặc chữ ký rỗng)
        cap_nhat(&f, &bang(&[("B.pdf", "3:blank")]), 200).unwrap();
        let m = doc(&f);
        assert_eq!(m["A.pdf"], Moc { luc: 200, chu_ky: CHU_KY_RONG }, "giữ mục bia mộ với mốc mới");
        assert!(!cap_nhat(&f, &bang(&[("B.pdf", "3:blank")]), 300).unwrap(), "bia mộ không bị đóng lại mỗi lần ghi");
        // Hồ sơ chưa từng có gì → không sinh mục
        cap_nhat(&f, &bang(&[("B.pdf", "3:blank"), ("D.pdf", "")]), 400).unwrap();
        assert!(!doc(&f).contains_key("D.pdf"));
        let _ = std::fs::remove_dir_all(f.parent().unwrap());
    }

    #[test]
    fn dong_hong_bo_qua_khoa_co_dau_cham_phay_va_nfd() {
        let f = tam("hong");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        let nfd = "Ha\u{0309}i/a;b.pdf";
        std::fs::write(&f, format!("A.pdf;100;ff\ndòng hỏng\nB.pdf;x;1\n{nfd};5;a\n")).unwrap();
        let m = doc(&f);
        assert_eq!(m.len(), 2);
        assert_eq!(m["A.pdf"], Moc { luc: 100, chu_ky: 0xff });
        assert_eq!(m["H\u{1ea3}i/a;b.pdf"].luc, 5, "khoá chuẩn hoá NFC, tên file có ';' vẫn đọc đúng");
        let _ = std::fs::remove_dir_all(f.parent().unwrap());
    }
}
