//! Đọc biên mục: XLSX / XLSB / XLS (calamine) hoặc CSV. "Header là dòng" = số dòng VẬT LÝ như Excel hiển thị.
//! Định dạng ô giống bản gốc (InvariantCulture): số nguyên không phần thập phân, ngày `dd/MM/yyyy`.

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BangDuLieu {
    pub tieu_de: Vec<String>,
    /// Mỗi dòng đủ số ô theo tiêu đề (thiếu → chuỗi rỗng).
    pub dong: Vec<Vec<String>>,
    /// Số dòng vật lý (1-based, như Excel hiển thị) của từng dòng dữ liệu — để báo lỗi đúng chỗ.
    pub so_dong: Vec<usize>,
}

impl BangDuLieu {
    /// Chỉ số cột theo tên tiêu đề; trùng tên → cột sau cùng (như bản gốc ghi đè từ điển).
    pub fn cot(&self, ten: &str) -> Option<usize> {
        self.tieu_de.iter().rposition(|t| t == ten)
    }

    /// Ô (dòng, tên cột); không có cột → rỗng.
    pub fn o(&self, dong: usize, cot: &str) -> &str {
        self.cot(cot).and_then(|c| self.dong.get(dong)?.get(c)).map_or("", String::as_str)
    }
}

pub fn la_csv(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv") || e.eq_ignore_ascii_case("tsv"))
}

/// Tên các sheet (CSV → rỗng).
pub fn danh_sach_sheet(p: &Path) -> Result<Vec<String>, String> {
    if la_csv(p) {
        return Ok(Vec::new());
    }
    let wb = open_workbook_auto(p).map_err(|e| format!("không mở được {}: {e}", p.display()))?;
    Ok(wb.sheet_names())
}

/// `dong_tieu_de`: số dòng vật lý (1-based) của hàng tiêu đề.
pub fn doc(p: &Path, sheet: Option<&str>, dong_tieu_de: usize) -> Result<BangDuLieu, String> {
    tu_ma_tran(doc_ma_tran(p, sheet, dong_tieu_de)?, dong_tieu_de)
}

/// Ô thô theo dòng vật lý (`[i]` = dòng i + 1), chưa tách tiêu đề — để xem nhanh các dòng đầu sheet.
/// `dong_tieu_de` chỉ dùng dò dấu phân cách của CSV.
pub fn doc_ma_tran(p: &Path, sheet: Option<&str>, dong_tieu_de: usize) -> Result<Vec<Vec<String>>, String> {
    if la_csv(p) { doc_csv(p, dong_tieu_de.max(1)) } else { doc_excel(p, sheet) }
}

/// `ma_tran[i]` = dòng vật lý i + 1 (dòng trống = vec rỗng).
pub fn tu_ma_tran(ma_tran: Vec<Vec<String>>, dong_tieu_de: usize) -> Result<BangDuLieu, String> {
    let dong_tieu_de = dong_tieu_de.max(1);
    if ma_tran.len() < dong_tieu_de {
        return Err(format!("Không đủ dữ liệu: file không có tới dòng {dong_tieu_de}."));
    }
    let mut tieu_de: Vec<String> = ma_tran[dong_tieu_de - 1].iter().map(|s| s.trim().to_string()).collect();
    while tieu_de.last().is_some_and(String::is_empty) {
        tieu_de.pop();
    }
    if tieu_de.is_empty() {
        return Err(format!("Dòng {dong_tieu_de} trống — chọn đúng dòng tiêu đề (Header dòng)."));
    }
    let (mut dong, mut so_dong) = (Vec::new(), Vec::new());
    for (i, d) in ma_tran.into_iter().enumerate().skip(dong_tieu_de) {
        let d: Vec<String> = (0..tieu_de.len()).map(|k| d.get(k).map_or(String::new(), |s| s.trim().to_string())).collect();
        if d.iter().any(|s| !s.is_empty()) {
            dong.push(d);
            so_dong.push(i + 1);
        }
    }
    Ok(BangDuLieu { tieu_de, dong, so_dong })
}

fn doc_excel(p: &Path, sheet: Option<&str>) -> Result<Vec<Vec<String>>, String> {
    let mut wb = open_workbook_auto(p).map_err(|e| format!("không mở được {}: {e}", p.display()))?;
    let ten = match sheet {
        Some(s) => s.to_string(),
        None => wb.sheet_names().first().cloned().ok_or("file không có sheet nào")?,
    };
    let vung = wb.worksheet_range(&ten).map_err(|e| format!("không đọc được sheet '{ten}': {e}"))?;
    // Vùng bắt đầu ở ô có dữ liệu đầu tiên → dịch về toạ độ vật lý (dòng 1, cột A)
    let (r0, c0) = vung.start().map_or((0, 0), |(r, c)| (r as usize, c as usize));
    let mut ra: Vec<Vec<String>> = vec![Vec::new(); r0];
    for hang in vung.rows() {
        let mut d = vec![String::new(); c0];
        d.extend(hang.iter().map(o_excel));
        ra.push(d);
    }
    Ok(ra)
}

fn o_excel(o: &Data) -> String {
    match o {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => so(*f),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        // as_datetime xử lý cả hệ ngày 1904 (file tạo trên Mac); ngày ngoài phạm vi → giữ dạng số
        Data::DateTime(d) => d.as_datetime().map_or_else(|| so(d.as_f64()), ngay),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    }
}

/// Số: nguyên → không phần thập phân, không mũ; lẻ → dạng ngắn nhất.
fn so(f: f64) -> String {
    if f.is_finite() && f == f.floor() { format!("{f:.0}") } else { f.to_string() }
}

/// Ngày giờ → `dd/MM/yyyy`, hoặc `dd/MM/yyyy HH:mm:ss` khi có giờ (như bản gốc).
fn ngay(t: chrono::NaiveDateTime) -> String {
    if t.time() == chrono::NaiveTime::MIN { t.format("%d/%m/%Y").to_string() } else { t.format("%d/%m/%Y %H:%M:%S").to_string() }
}

/// Mẫu đầu file là UTF-8 hợp lệ (chuỗi nhiều byte bị cắt ở cuối mẫu vẫn tính là hợp lệ).
fn la_utf8(b: &[u8]) -> bool {
    match std::str::from_utf8(b) {
        Ok(_) => true,
        Err(e) => e.error_len().is_none(),
    }
}

fn doc_csv(p: &Path, dong_tieu_de: usize) -> Result<Vec<Vec<String>>, String> {
    let b = std::fs::read(p).map_err(|e| format!("không đọc được {}: {e}", p.display()))?;
    let chu = match encoding_rs::Encoding::for_bom(&b) {
        Some((enc, n)) => enc.decode_without_bom_handling(&b[n..]).0.into_owned(),
        None if la_utf8(&b[..b.len().min(256 * 1024)]) => String::from_utf8_lossy(&b).into_owned(),
        None => encoding_rs::WINDOWS_1258.decode_without_bom_handling(&b).0.into_owned(),
    };
    // Dò dấu phân cách trên chính dòng tiêu đề (bản gốc xét dòng 1 — sai khi có dòng tiêu đề phụ phía trên;
    // xét nhiều dòng thì dấu phẩy trong trích yếu làm lệch)
    let dong = chu.lines().nth(dong_tieu_de - 1).unwrap_or("");
    let dem = |c: char| dong.matches(c).count();
    let (phay, cham_phay, tab) = (dem(','), dem(';'), dem('\t'));
    let ngan = if tab > phay && tab > cham_phay { '\t' } else if cham_phay > phay { ';' } else { ',' };
    Ok(tach_csv(&chu, ngan))
}

/// Tách CSV (RFC 4180: ngoặc kép, `""`, ô nhiều dòng) thành ma trận theo DÒNG VẬT LÝ: dòng trống giữ chỗ,
/// bản ghi nhiều dòng chiếm đủ số dòng của nó (thư viện `csv` bỏ dòng trống và đánh số dòng sai sau đó).
fn tach_csv(chu: &str, ngan: char) -> Vec<Vec<String>> {
    let mut ma_tran: Vec<Vec<String>> = Vec::new();
    let (mut ban_ghi, mut o) = (Vec::new(), String::new());
    let (mut trong_ngoac, mut dong, mut dong_bat_dau, mut co_noi_dung) = (false, 1usize, 1usize, false);
    let mut it = chu.chars().peekable();
    while let Some(c) = it.next() {
        if trong_ngoac {
            match c {
                '"' if it.peek() == Some(&'"') => {
                    o.push('"');
                    it.next();
                }
                '"' => trong_ngoac = false,
                '\n' => {
                    o.push('\n');
                    dong += 1;
                }
                '\r' => {}
                _ => o.push(c),
            }
            continue;
        }
        match c {
            '"' => {
                trong_ngoac = true;
                co_noi_dung = true;
            }
            '\r' => {}
            '\n' => {
                ban_ghi.push(std::mem::take(&mut o));
                ma_tran.resize(dong_bat_dau - 1, Vec::new());
                ma_tran.push(if co_noi_dung || ban_ghi.len() > 1 { std::mem::take(&mut ban_ghi) } else { Vec::new() });
                ban_ghi.clear();
                (dong, co_noi_dung) = (dong + 1, false);
                dong_bat_dau = dong;
            }
            c if c == ngan => {
                ban_ghi.push(std::mem::take(&mut o));
                co_noi_dung = true;
            }
            _ => {
                o.push(c);
                co_noi_dung = true;
            }
        }
    }
    if co_noi_dung || !ban_ghi.is_empty() {
        ban_ghi.push(o);
        ma_tran.resize(dong_bat_dau - 1, Vec::new());
        ma_tran.push(ban_ghi);
    }
    ma_tran
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tam(ten: &str, noi_dung: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("dema4_bang_{}_{ten}", std::process::id()));
        std::fs::write(&p, noi_dung).unwrap();
        p
    }

    #[test]
    fn doc_csv_bom_cham_phay_xuong_dong_trong_o() {
        let p = tam("a.csv", "\u{feff}tiêu đề\nSố HS;Trích yếu;Ghi chú\n01;\"Dòng 1\ndòng 2\"; x \n\n;;\n02;Văn bản B\n".as_bytes());
        let b = doc(&p, None, 2).unwrap();
        assert_eq!(b.tieu_de, ["Số HS", "Trích yếu", "Ghi chú"]);
        assert_eq!(b.so_dong, [3, 7], "dòng vật lý tính cả dòng trống và ô nhiều dòng");
        assert_eq!(b.dong, [vec!["01", "Dòng 1\ndòng 2", "x"], vec!["02", "Văn bản B", ""]], "bỏ dòng trắng, thiếu ô → rỗng");
        assert_eq!(b.o(1, "Trích yếu"), "Văn bản B");
        assert_eq!(b.o(0, "không có"), "");
        let _ = std::fs::remove_file(p);
    }

    /// Dòng trống phía trên vẫn được đếm; dấu phẩy trong trích yếu không làm sai dấu phân cách `;`.
    #[test]
    fn csv_dong_trong_va_dau_phay_trong_noi_dung() {
        let mut nd = String::from("Tiêu đề, phụ, có, phẩy\n\nSố HS;TY\n");
        for i in 0..19 {
            nd += &format!("{i};Quyết định, về việc, giao, đất\n");
        }
        let p = tam("c.csv", nd.as_bytes());
        let b = doc(&p, None, 3).unwrap();
        assert_eq!(b.tieu_de, ["Số HS", "TY"]);
        assert_eq!((b.dong.len(), b.o(0, "TY")), (19, "Quyết định, về việc, giao, đất"));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn tach_csv_dong_vat_ly() {
        let m = tach_csv("a;\"b\nc\";\"x\"\"y\"\r\n\n1;2\n", ';');
        assert_eq!(m, [vec!["a", "b\nc", "x\"y"], vec![], vec![], vec!["1", "2"]].map(|v| v.into_iter().map(String::from).collect::<Vec<_>>()), "bản ghi 2 dòng chiếm dòng 1–2, dòng 3 trống");
    }

    #[test]
    fn doc_csv_windows_1258() {
        // "Mã,Tác\n1,Đà\n" theo bảng mã Windows-1258 (byte cao đơn lẻ → không phải UTF-8)
        let p = tam("b.csv", b"M\xe3,T\xe1c\n1,\xd0\xe0\n");
        let bang = doc(&p, None, 1).unwrap();
        assert_eq!((bang.tieu_de[1].as_str(), bang.dong[0][1].as_str()), ("Tác", "Đà"));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn dinh_dang_o_nhu_ban_goc() {
        assert_eq!(so(12345.0), "12345");
        assert_eq!(so(1.5), "1.5");
        let d = |v, m1904| calamine::ExcelDateTime::new(v, calamine::ExcelDateTimeType::DateTime, m1904);
        assert_eq!(o_excel(&Data::DateTime(d(41556.0, false))), "09/10/2013");
        assert_eq!(o_excel(&Data::DateTime(d(41556.5, false))), "09/10/2013 12:00:00");
        assert_eq!(o_excel(&Data::DateTime(d(40094.0, true))), "09/10/2013", "hệ ngày 1904 (Excel Mac)");
        assert_eq!(o_excel(&Data::DateTime(d(1e15, false))), "1000000000000000", "ngày tràn phạm vi → không panic");
        assert_eq!(o_excel(&Data::Bool(true)), "TRUE");
    }

    #[test]
    fn header_vuot_qua_so_dong_bao_loi() {
        assert!(tu_ma_tran(vec![vec!["a".into()]], 3).is_err());
        let b = tu_ma_tran(vec![vec![], vec!["".into(), "A".into(), "".into()], vec![], vec!["".into(), "1".into()]], 2).unwrap();
        assert_eq!((b.tieu_de.clone(), b.dong.clone()), (vec!["".to_string(), "A".into()], vec![vec!["".to_string(), "1".into()]]), "cột trống đầu giữ, đuôi bỏ");
        assert_eq!(b.so_dong, [4], "số dòng vật lý");
        assert!(tu_ma_tran(vec![vec![], vec!["x".into()]], 1).unwrap_err().contains("Dòng 1 trống"));
    }
}

#[cfg(test)]
mod tests_xlsx {
    use super::*;

    /// File tổng hợp: tiêu đề ở dòng 4 (vật lý), dữ liệu bắt đầu cột C; ngày, giờ, số lẻ.
    #[test]
    fn doc_xlsx_header_dong_vat_ly() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests_data/muc_luc/bien_muc_tong_hop.xlsx");
        assert_eq!(danh_sach_sheet(&p).unwrap(), ["Bien muc", "Trang 2"]);
        let b = doc(&p, Some("Bien muc"), 4).unwrap();
        assert_eq!(&b.tieu_de[2..], ["Số hồ sơ", "STT", "Số, ký hiệu", "Ngày tháng", "Tác giả", "Trích yếu", "Tờ số", "Số trang"]);
        assert_eq!(b.tieu_de[..2], ["", ""], "cột A, B trống vẫn giữ vị trí");
        assert_eq!(b.dong.len(), 3);
        assert_eq!((b.o(0, "STT"), b.o(0, "Ngày tháng"), b.o(1, "Ngày tháng"), b.o(1, "Số trang")), ("1", "05/01/2015", "12/02/2015 08:30:00", "2.5"));
        assert_eq!(b.o(2, "Trích yếu"), "Báo cáo <quý I>");
        assert!(doc(&p, Some("Không có"), 1).is_err());
    }
}
