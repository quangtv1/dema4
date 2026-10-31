//! Làm sạch giá trị mô hình trả về theo cột Chuẩn hoá của trường, cộng các lỗi hay gặp của mô hình nhỏ
//! (lặp lại nhãn mô tả "Loại văn bản: Quyết định", số ký hiệu kèm chữ "Số", ngày viết bằng chữ).

use super::loai_tai_lieu::{ChuanHoa, Truong};
use crate::muc_luc::khop::chuan_hoa as dang_so;

pub fn chuan_hoa(gia_tri: &str, t: &Truong) -> String {
    let mut s = gia_tri.split_whitespace().collect::<Vec<_>>().join(" ");
    s = s.trim_matches(|c| matches!(c, '"' | '\'' | '“' | '”')).trim().to_string();
    s = bo_nhan_lap(&s, t);
    if dang_so(&t.ten_cot).starts_with("so") {
        s = bo_chu_so(&s);
    }
    match t.chuan_hoa {
        ChuanHoa::GiuNguyen => s,
        ChuanHoa::DanhSach => khop_danh_sach(&s, &t.danh_sach),
        ChuanHoa::Ngay => ngay(&s).unwrap_or(s),
        ChuanHoa::HoaChuDau => hoa_chu_dau(&s),
        ChuanHoa::InHoa => s.to_uppercase(),
    }
}

/// "Loại văn bản: Quyết định" → "Quyết định" khi phần trước dấu ":" là tên cột hoặc đầu mô tả.
fn bo_nhan_lap(s: &str, t: &Truong) -> String {
    if let Some((truoc, sau)) = s.split_once(':') {
        let nhan = dang_so(truoc);
        if nhan.len() >= 2 && (nhan == dang_so(&t.ten_cot) || dang_so(&t.mo_ta).starts_with(&nhan)) {
            return sau.trim().to_string();
        }
    }
    s.to_string()
}

/// "Số 36/BC-NTTS" / "Số: 36/…" → "36/BC-NTTS" (chỉ khi phần còn lại bắt đầu bằng chữ số).
fn bo_chu_so(s: &str) -> String {
    let dau: String = s.chars().take(2).collect::<String>().to_lowercase();
    if dau == "số" || dau == "so" {
        let con = s.chars().skip(2).collect::<String>();
        let con = con.trim_start_matches([' ', ':', '.']);
        if con.starts_with(|c: char| c.is_ascii_digit()) {
            return con.to_string();
        }
    }
    s.to_string()
}

/// Khớp không dấu / không hoa với 1 giá trị trong danh sách; chứa giá trị dài nhất cũng tính; không khớp → rỗng.
fn khop_danh_sach(s: &str, ds: &[String]) -> String {
    let k = dang_so(s);
    if k.is_empty() {
        return String::new();
    }
    if let Some(g) = ds.iter().find(|g| dang_so(g) == k) {
        return g.clone();
    }
    ds.iter().filter(|g| !dang_so(g).is_empty() && k.contains(&dang_so(g))).max_by_key(|g| dang_so(g).len()).cloned().unwrap_or_default()
}

/// Ngày → dd/mm/yyyy: "6/4/2015", "06-04-2015", "2015-04-06", "ngày 6 tháng 4 năm 2015", "Hà Nội, ngày 06 tháng 4 năm 2015".
fn ngay(s: &str) -> Option<String> {
    let so: Vec<&str> = s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).collect();
    let hop_le = |d: u32, m: u32, y: u32| (1..=31).contains(&d) && (1..=12).contains(&m) && (1800..=2200).contains(&y);
    for w in so.windows(3) {
        let (a, b, c) = (w[0].parse::<u32>().ok()?, w[1].parse::<u32>().ok()?, w[2].parse::<u32>().ok()?);
        if w[2].len() == 4 && hop_le(a, b, c) {
            return Some(format!("{a:02}/{b:02}/{c}"));
        }
        if w[0].len() == 4 && hop_le(c, b, a) {
            return Some(format!("{c:02}/{b:02}/{a}"));
        }
    }
    None
}

/// "NGUYỄN VĂN A" → "Nguyễn Văn A".
fn hoa_chu_dau(s: &str) -> String {
    s.split(' ')
        .map(|tu| {
            let mut c = tu.chars();
            c.next().map(|d| d.to_uppercase().chain(c.flat_map(char::to_lowercase)).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::super::loai_tai_lieu::co_san;
    use super::*;

    fn truong(bien: &str) -> Truong {
        co_san()[0].truong.iter().find(|t| t.bien == bien).unwrap().clone()
    }

    #[test]
    fn loi_quan_sat_tu_vintern() {
        // Câu trả lời thật của vLLM + Vintern-1B (1 trang văn bản mẫu)
        assert_eq!(chuan_hoa("Loại văn bản: Quyết định", &truong("loai_van_ban")), "Quyết định");
        assert_eq!(chuan_hoa("Số 36/BC-NTTS", &truong("so_ky_hieu")), "36/BC-NTTS");
        assert_eq!(chuan_hoa("6 tháng 4 năm 2015", &truong("ngay_thang")), "06/04/2015");
    }

    #[test]
    fn ngay_nhieu_dang() {
        let t = truong("ngay_thang");
        for (vao, ra) in [("6/4/2015", "06/04/2015"), ("06-04-2015", "06/04/2015"), ("2015-04-06", "06/04/2015"), ("Hà Nội, ngày 06 tháng 04 năm 2015", "06/04/2015"), ("", ""), ("không rõ", "không rõ")] {
            assert_eq!(chuan_hoa(vao, &t), ra, "{vao}");
        }
        assert_eq!(chuan_hoa("32/13/2015", &t), "32/13/2015", "ngày sai giữ nguyên để người dùng sửa");
    }

    #[test]
    fn danh_sach_khong_dau_khong_hoa() {
        let t = truong("loai_van_ban");
        assert_eq!(chuan_hoa("quyet dinh", &t), "Quyết định");
        assert_eq!(chuan_hoa("CÔNG VĂN", &t), "Công văn");
        assert_eq!(chuan_hoa("Quyết định số 12", &t), "Quyết định");
        assert_eq!(chuan_hoa("Nghị quyết", &t), "");
    }

    #[test]
    fn hoa_chu_dau_in_hoa_giu_nguyen() {
        let mut t = truong("nguoi_ky");
        assert_eq!(chuan_hoa("  NGUYỄN   VĂN A ", &t), "NGUYỄN VĂN A", "giữ nguyên chỉ gộp khoảng trắng");
        t.chuan_hoa = ChuanHoa::HoaChuDau;
        assert_eq!(chuan_hoa("NGUYỄN VĂN A", &t), "Nguyễn Văn A");
        let mut t = truong("tac_gia");
        assert_eq!(chuan_hoa("\"UBND huyện\"", &t), "UBND huyện");
        t.chuan_hoa = ChuanHoa::InHoa;
        assert_eq!(chuan_hoa("ubnd huyện", &t), "UBND HUYỆN");
    }

    #[test]
    fn khong_bo_nham() {
        // Trích yếu có dấu ":" nhưng không phải nhãn; số ký hiệu không có chữ "Số"
        assert_eq!(chuan_hoa("V/v: phê duyệt dự toán", &truong("trich_yeu")), "V/v: phê duyệt dự toán");
        assert_eq!(chuan_hoa("Sở Tài chính", &truong("so_ky_hieu")), "Sở Tài chính");
        assert_eq!(chuan_hoa("12/QĐ-UBND", &truong("so_ky_hieu")), "12/QĐ-UBND");
    }
}
