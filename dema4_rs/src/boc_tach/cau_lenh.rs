//! Câu lệnh gửi mô hình đọc cho 1 trang — đúng nội dung popup Câu lệnh của prototype đã duyệt.

use super::loai_tai_lieu::{ChuanHoa, LoaiTaiLieu};

/// Câu lệnh cho trang `trang` (từ 1) của văn bản `tong` trang.
pub fn cau_lenh(loai: &LoaiTaiLieu, trang: usize, tong: usize) -> String {
    let mut s = String::new();
    if !loai.huong_dan.trim().is_empty() {
        s += loai.huong_dan.trim();
        s.push('\n');
    }
    s += &if trang == 1 {
        format!("Đây là TRANG ĐẦU (trang 1/{tong}) của văn bản.\n")
    } else if trang == tong {
        format!("Đây là TRANG CUỐI (trang {tong}/{tong}) của văn bản.\n")
    } else {
        format!("Đây là trang {trang}/{tong} của văn bản.\n")
    };
    s += "Trích xuất các dữ liệu trong ảnh. Trường không có trên trang này để chuỗi rỗng \"\". Tuyệt đối không suy đoán.\nTrả về đúng JSON theo mẫu:\n{";
    let dong: Vec<String> = loai
        .truong
        .iter()
        .map(|t| {
            let ds = if t.chuan_hoa == ChuanHoa::DanhSach && !t.danh_sach.is_empty() { format!(": {}", t.danh_sach.join(" | ")) } else { String::new() };
            format!("\n  \"{}\": \"{}{}\"", t.bien, mo_ta_json(&t.mo_ta), ds)
        })
        .collect();
    s += &dong.join(",");
    s += "\n}";
    s
}

/// Mô tả nằm trong chuỗi JSON mẫu: bỏ xuống dòng, thay `"` để mẫu vẫn là JSON hợp lệ.
fn mo_ta_json(mo_ta: &str) -> String {
    mo_ta.split_whitespace().collect::<Vec<_>>().join(" ").replace('"', "'")
}

#[cfg(test)]
mod tests {
    use super::super::loai_tai_lieu::co_san;
    use super::*;

    #[test]
    fn trang_dau_khop_prototype() {
        let s = cau_lenh(&co_san()[0], 1, 4);
        let mong_doi = "Văn bản hành chính Việt Nam theo Nghị định 30/2020/NĐ-CP: quốc hiệu góc trên phải, cơ quan ban hành góc trên trái, khối chữ ký cuối văn bản.\n\
Đây là TRANG ĐẦU (trang 1/4) của văn bản.\n\
Trích xuất các dữ liệu trong ảnh. Trường không có trên trang này để chuỗi rỗng \"\". Tuyệt đối không suy đoán.\n\
Trả về đúng JSON theo mẫu:\n{\n  \"loai_van_ban\": \"Loại văn bản ở trang đầu, chỉ chọn một trong danh sách: Quyết định | Công văn | Tờ trình | Báo cáo | Kế hoạch | Thông báo | Giấy mời | Kết luận\",\n  \"so_ky_hieu\": \"Số, ký hiệu văn bản ở trang đầu (vd 123/QĐ-UBND, 45/CV-VP)\",\n  \"ngay_thang\": \"Ngày ban hành ở trang đầu, định dạng dd/mm/yyyy\",\n  \"tac_gia\": \"Tên đầy đủ cơ quan, tổ chức ban hành, góc trên trái trang đầu\",\n  \"trich_yeu\": \"Trích yếu nội dung ở trang đầu: văn bản về việc gì (dòng dưới tên loại văn bản)\",\n  \"nguoi_ky\": \"Họ tên người ký ở khối chữ ký, trang cuối văn bản\"\n}";
        assert_eq!(s, mong_doi);
    }

    #[test]
    fn trang_cuoi_giua_va_mau_la_json() {
        let l = &co_san()[0];
        assert!(cau_lenh(l, 4, 4).contains("Đây là TRANG CUỐI (trang 4/4) của văn bản."));
        assert!(cau_lenh(l, 2, 4).contains("Đây là trang 2/4 của văn bản."));
        let s = cau_lenh(l, 1, 1);
        let mau = &s[s.find('{').unwrap()..];
        assert!(serde_json::from_str::<serde_json::Value>(mau).is_ok());
    }
}
