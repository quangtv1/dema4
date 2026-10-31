//! Ghi kết quả bóc ra Excel `<tên thư mục nguồn>_boc_tach.xlsx`: dòng 1 = tên cột (Tên file, Hồ sơ, Số trang, các
//! trường theo thứ tự loại tài liệu), mỗi file 1 dòng. Ô trường định dạng văn bản (`@`) để Excel không đổi số ký hiệu /
//! ngày thành số. Tên cột trùng biến Sinh mục lục (so khớp không dấu) → file nạp thẳng làm file biên mục.

use std::path::{Path, PathBuf};

use rust_xlsxwriter::{Format, Workbook};

use super::loai_tai_lieu::LoaiTaiLieu;

/// 1 dòng Excel.
#[derive(Debug, Clone, PartialEq)]
pub struct DongExcel {
    pub ten_file: String,
    pub ho_so: String,
    pub so_trang: usize,
    /// Theo thứ tự trường của loại.
    pub gia_tri: Vec<String>,
    pub loi: Option<String>,
}

pub const TAB: &str = "Boc_tach";

/// `<thư mục ghi>/<tên nguồn>_boc_tach.xlsx`; thư mục ghi trống = thư mục cha của thư mục nguồn (cạnh thư mục nguồn).
pub fn duong_dan(thu_muc_nguon: &Path, thu_muc_ghi: &str) -> PathBuf {
    let ten = thu_muc_nguon.file_name().map_or_else(|| "van_ban".into(), |s| s.to_string_lossy().into_owned());
    let cha = if thu_muc_ghi.trim().is_empty() { thu_muc_nguon.parent().map(Path::to_path_buf).unwrap_or_else(|| thu_muc_nguon.to_path_buf()) } else { PathBuf::from(thu_muc_ghi.trim()) };
    cha.join(format!("{ten}_boc_tach.xlsx"))
}

/// Tiêu đề cột (dùng chung cho Excel và Google Sheet).
pub fn tieu_de(loai: &LoaiTaiLieu, co_loi: bool) -> Vec<String> {
    let mut t: Vec<String> = ["Tên file", "Hồ sơ", "Số trang"].iter().map(|s| s.to_string()).chain(loai.truong.iter().map(|t| t.ten_cot.clone())).collect();
    if co_loi {
        t.push("Lỗi bóc tách".into());
    }
    t
}

/// Dòng dạng chữ (Google Sheet).
pub fn dong_chu(d: &DongExcel, co_loi: bool) -> Vec<String> {
    let mut v = vec![d.ten_file.clone(), d.ho_so.clone(), d.so_trang.to_string()];
    v.extend(d.gia_tri.iter().cloned());
    if co_loi {
        v.push(d.loi.clone().unwrap_or_default());
    }
    v
}

/// Ghi file; file đang mở (khoá ghi) → ghi bản có hậu tố thời gian bên cạnh. Trả đường dẫn đã ghi.
pub fn ghi(duong_dan: &Path, loai: &LoaiTaiLieu, dong: &[DongExcel]) -> Result<PathBuf, String> {
    match ghi_vao(duong_dan, loai, dong) {
        Ok(()) => Ok(duong_dan.to_path_buf()),
        Err(e) if duong_dan.exists() => {
            let goc = duong_dan.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            let khac = duong_dan.with_file_name(format!("{goc}_{}.xlsx", chrono::Local::now().format("%Y%m%d_%H%M%S")));
            ghi_vao(&khac, loai, dong).map(|_| khac).map_err(|e2| format!("{e}; {e2}"))
        }
        Err(e) => Err(e),
    }
}

fn ghi_vao(duong_dan: &Path, loai: &LoaiTaiLieu, dong: &[DongExcel]) -> Result<(), String> {
    let co_loi = dong.iter().any(|d| d.loi.is_some());
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(TAB).map_err(|e| e.to_string())?;
    let dam = Format::new().set_bold().set_background_color(0xF3F3F3);
    let chu = Format::new().set_num_format("@");
    let td = tieu_de(loai, co_loi);
    for (c, t) in td.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, t, &dam).map_err(|e| e.to_string())?;
        let rong = match c {
            0 => 28.0,
            1 => 14.0,
            2 => 9.0,
            _ if loai.truong.get(c - 3).is_some_and(|t| t.bien == "trich_yeu" || t.bien == "noi_dung") => 60.0,
            _ => 20.0,
        };
        ws.set_column_width(c as u16, rong).map_err(|e| e.to_string())?;
    }
    for (r, d) in dong.iter().enumerate() {
        let r = r as u32 + 1;
        ws.write_string_with_format(r, 0, &d.ten_file, &chu).map_err(|e| e.to_string())?;
        ws.write_string_with_format(r, 1, &d.ho_so, &chu).map_err(|e| e.to_string())?;
        ws.write_number(r, 2, d.so_trang as f64).map_err(|e| e.to_string())?;
        for (k, g) in d.gia_tri.iter().enumerate() {
            ws.write_string_with_format(r, 3 + k as u16, g, &chu).map_err(|e| e.to_string())?;
        }
        if let (true, Some(l)) = (co_loi, &d.loi) {
            ws.write_string(r, 3 + loai.truong.len() as u16, l).map_err(|e| e.to_string())?;
        }
    }
    ws.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    if let Some(cha) = duong_dan.parent() {
        std::fs::create_dir_all(cha).map_err(|e| format!("Không tạo được thư mục {}: {e}", cha.display()))?;
    }
    wb.save(duong_dan).map_err(|e| format!("Không ghi được {}: {e}", duong_dan.display()))
}

#[cfg(test)]
mod tests {
    use super::super::loai_tai_lieu::co_san;
    use super::*;

    fn dong() -> Vec<DongExcel> {
        vec![
            DongExcel { ten_file: "HS-01/VB01.pdf".into(), ho_so: "HS-01".into(), so_trang: 3, gia_tri: ["Tờ trình", "15/TTr-TCKH", "20/01/2015", "Phòng A", "Dự toán", "Trần Thị B"].map(String::from).to_vec(), loi: None },
            DongExcel { ten_file: "HS-01/VB02.pdf".into(), ho_so: "HS-01".into(), so_trang: 2, gia_tri: vec![String::new(); 6], loi: Some("Máy chủ báo lỗi 500".into()) },
        ]
    }

    #[test]
    fn duong_dan_canh_thu_muc_nguon_hoac_thu_muc_cau_hinh() {
        assert_eq!(duong_dan(Path::new("/tmp/SoHoa/Van_ban"), ""), Path::new("/tmp/SoHoa/Van_ban_boc_tach.xlsx"));
        assert_eq!(duong_dan(Path::new("/tmp/SoHoa/Van_ban"), "/tmp/Ra"), Path::new("/tmp/Ra/Van_ban_boc_tach.xlsx"));
    }

    #[test]
    fn ghi_doc_lai_va_nap_duoc_vao_sinh_muc_luc() {
        let d = std::env::temp_dir().join(format!("dema4_bt_excel_{}", std::process::id()));
        let f = d.join("Van_ban_boc_tach.xlsx");
        let l = &co_san()[0];
        assert_eq!(ghi(&f, l, &dong()).unwrap(), f);
        let bang = crate::muc_luc::doc_bang::doc(&f, None, 1).unwrap();
        assert_eq!(bang.tieu_de, ["Tên file", "Hồ sơ", "Số trang", "Loại văn bản", "Số, ký hiệu", "Ngày tháng", "Tác giả", "Trích yếu", "Người ký", "Lỗi bóc tách"]);
        assert_eq!(bang.dong[0][4], "15/TTr-TCKH");
        assert_eq!(bang.dong[0][5], "20/01/2015", "ngày giữ dạng chữ, không thành số");
        assert_eq!(bang.dong[1][9], "Máy chủ báo lỗi 500");
        // Biến mẫu Sinh mục lục tự khớp tên cột (so khớp không dấu)
        let khop: Vec<_> = ["so_ky_hieu", "ngay_thang", "tac_gia", "trich_yeu"].iter().map(|b| bang.tieu_de.iter().any(|t| crate::muc_luc::khop::chuan_hoa(t) == crate::muc_luc::khop::chuan_hoa(b))).collect();
        assert_eq!(khop, [true; 4]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
