//! Loại tài liệu: bộ trường cần trích, mô tả gửi mô hình đọc, chuẩn hoá, trang đọc. 3 loại có sẵn; người dùng
//! sửa / thêm / xoá, lưu JSON trong `.sm-dema4/boc_tach/loai_tai_lieu.json` cạnh file chạy.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Trang nào của văn bản được gửi cho mô hình đọc (mỗi trang 1 yêu cầu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrangDoc {
    Dau,
    #[default]
    DauCuoi,
    TatCa,
}

impl TrangDoc {
    /// Các trang (đánh số từ 1) cần đọc của văn bản `n` trang.
    pub fn cac_trang(self, n: usize) -> Vec<usize> {
        match (self, n) {
            (_, 0) => vec![],
            (TrangDoc::Dau, _) | (TrangDoc::DauCuoi, 1) => vec![1],
            (TrangDoc::DauCuoi, _) => vec![1, n],
            (TrangDoc::TatCa, _) => (1..=n).collect(),
        }
    }
}

/// Cách làm sạch giá trị mô hình trả về (thứ tự = thứ tự trong ô chọn của popup Chỉnh sửa loại).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChuanHoa {
    #[default]
    GiuNguyen,
    DanhSach,
    Ngay,
    HoaChuDau,
    InHoa,
}

impl ChuanHoa {
    pub const TAT_CA: [ChuanHoa; 5] = [ChuanHoa::GiuNguyen, ChuanHoa::DanhSach, ChuanHoa::Ngay, ChuanHoa::HoaChuDau, ChuanHoa::InHoa];

    pub fn ten(self) -> &'static str {
        match self {
            ChuanHoa::GiuNguyen => "Giữ nguyên",
            ChuanHoa::DanhSach => "Danh sách",
            ChuanHoa::Ngay => "Ngày dd/mm/yyyy",
            ChuanHoa::HoaChuDau => "Viết hoa chữ đầu",
            ChuanHoa::InHoa => "Chữ in hoa",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truong {
    /// Tên cột trong Excel / Google Sheet.
    pub ten_cot: String,
    /// Khoá JSON gửi mô hình và tên biến dùng ở Sinh mục lục.
    pub bien: String,
    /// Mô tả gửi mô hình đọc; nhắc "trang đầu" / "trang cuối" để chọn trang lấy giá trị khi gộp.
    pub mo_ta: String,
    #[serde(default)]
    pub chuan_hoa: ChuanHoa,
    /// Giá trị hợp lệ khi `chuan_hoa = DanhSach`.
    #[serde(default)]
    pub danh_sach: Vec<String>,
    /// Để trống → báo Kiểm tra ở tab Kết quả.
    #[serde(default)]
    pub bat_buoc: bool,
}

/// Trang ưu tiên lấy giá trị của trường khi đọc nhiều trang.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayTrang {
    Dau,
    Cuoi,
}

impl Truong {
    pub fn moi(ten_cot: &str, mo_ta: &str, chuan_hoa: ChuanHoa, bat_buoc: bool) -> Self {
        Self { ten_cot: ten_cot.into(), bien: bien_tu_ten(ten_cot), mo_ta: mo_ta.into(), chuan_hoa, danh_sach: vec![], bat_buoc }
    }

    /// Mô tả nhắc "trang cuối" → lấy ở trang cuối, còn lại lấy ở trang đầu.
    pub fn lay_trang(&self) -> LayTrang {
        if self.mo_ta.to_lowercase().contains("trang cuối") {
            LayTrang::Cuoi
        } else {
            LayTrang::Dau
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoaiTaiLieu {
    pub ten: String,
    #[serde(default)]
    pub trang_doc: TrangDoc,
    /// Đoạn mở đầu câu lệnh: bố cục chung của loại tài liệu.
    #[serde(default)]
    pub huong_dan: String,
    pub truong: Vec<Truong>,
}

/// Biến từ tên cột: bỏ dấu, chữ thường, ký tự khác chữ / số thành `_` ("Số, ký hiệu" → "so_ky_hieu").
pub fn bien_tu_ten(ten_cot: &str) -> String {
    crate::muc_luc::khop::ten_file_khong_dau(ten_cot).to_lowercase()
}

pub const DS_LOAI_VAN_BAN: [&str; 8] = ["Quyết định", "Công văn", "Tờ trình", "Báo cáo", "Kế hoạch", "Thông báo", "Giấy mời", "Kết luận"];

/// 3 loại có sẵn. Văn bản hành chính đúng như prototype đã duyệt.
pub fn co_san() -> Vec<LoaiTaiLieu> {
    use ChuanHoa::*;
    let mut loai_vb = Truong::moi("Loại văn bản", "Loại văn bản ở trang đầu, chỉ chọn một trong danh sách", DanhSach, true);
    loai_vb.bien = "loai_van_ban".into();
    loai_vb.danh_sach = DS_LOAI_VAN_BAN.iter().map(|s| s.to_string()).collect();
    let mut so_kh = Truong::moi("Số, ký hiệu", "Số, ký hiệu văn bản ở trang đầu (vd 123/QĐ-UBND, 45/CV-VP)", GiuNguyen, true);
    so_kh.bien = "so_ky_hieu".into();
    vec![
        LoaiTaiLieu {
            ten: "Văn bản hành chính".into(),
            trang_doc: TrangDoc::DauCuoi,
            huong_dan: "Văn bản hành chính Việt Nam theo Nghị định 30/2020/NĐ-CP: quốc hiệu góc trên phải, cơ quan ban hành góc trên trái, khối chữ ký cuối văn bản.".into(),
            truong: vec![
                loai_vb,
                so_kh,
                Truong::moi("Ngày tháng", "Ngày ban hành ở trang đầu, định dạng dd/mm/yyyy", Ngay, true),
                Truong::moi("Tác giả", "Tên đầy đủ cơ quan, tổ chức ban hành, góc trên trái trang đầu", GiuNguyen, true),
                Truong::moi("Trích yếu", "Trích yếu nội dung ở trang đầu: văn bản về việc gì (dòng dưới tên loại văn bản)", GiuNguyen, true),
                Truong::moi("Người ký", "Họ tên người ký ở khối chữ ký, trang cuối văn bản", GiuNguyen, false),
            ],
        },
        LoaiTaiLieu {
            ten: "Hồ sơ đất đai".into(),
            trang_doc: TrangDoc::DauCuoi,
            huong_dan: "Giấy tờ đất đai Việt Nam (giấy chứng nhận quyền sử dụng đất, đơn đăng ký, biên bản): thông tin thửa đất và người sử dụng đất.".into(),
            truong: vec![
                Truong::moi("Số thửa", "Số thửa đất ở trang đầu", GiuNguyen, true),
                Truong::moi("Tờ bản đồ", "Số tờ bản đồ ở trang đầu", GiuNguyen, true),
                Truong::moi("Địa chỉ thửa đất", "Địa chỉ thửa đất (xã, huyện, tỉnh) ở trang đầu", GiuNguyen, false),
                Truong::moi("Diện tích", "Diện tích thửa đất kèm đơn vị (vd 120,5 m²) ở trang đầu", GiuNguyen, false),
                Truong::moi("Người sử dụng đất", "Họ tên người / tổ chức sử dụng đất ở trang đầu", HoaChuDau, true),
                Truong::moi("Ngày ký", "Ngày ký hoặc ngày cấp ở trang cuối, định dạng dd/mm/yyyy", Ngay, false),
            ],
        },
        LoaiTaiLieu {
            ten: "Hợp đồng".into(),
            trang_doc: TrangDoc::TatCa,
            huong_dan: "Hợp đồng (kinh tế, dân sự, lao động) bằng tiếng Việt: số hợp đồng và các bên ở trang đầu, chữ ký các bên ở trang cuối.".into(),
            truong: vec![
                Truong::moi("Số hợp đồng", "Số hợp đồng ở trang đầu (vd 12/2015/HĐKT)", GiuNguyen, true),
                Truong::moi("Ngày ký", "Ngày ký hợp đồng ở trang đầu, định dạng dd/mm/yyyy", Ngay, true),
                Truong::moi("Bên A", "Tên bên A (bên giao / bên mua) ở trang đầu", GiuNguyen, true),
                Truong::moi("Bên B", "Tên bên B (bên nhận / bên bán) ở trang đầu", GiuNguyen, true),
                Truong::moi("Nội dung", "Nội dung chính của hợp đồng: hợp đồng về việc gì, ở trang đầu", GiuNguyen, false),
                Truong::moi("Giá trị", "Tổng giá trị hợp đồng kèm đơn vị tiền, nếu có", GiuNguyen, false),
            ],
        },
    ]
}

/// `<dữ liệu ẩn cạnh exe>/boc_tach/loai_tai_lieu.json`.
pub fn file_mac_dinh() -> PathBuf {
    crate::tach_vb::du_lieu_an::goc_du_lieu().join("boc_tach").join("loai_tai_lieu.json")
}

/// Đọc danh sách loại; chưa có file / file hỏng / rỗng → loại có sẵn.
pub fn doc(duong_dan: &Path) -> Vec<LoaiTaiLieu> {
    std::fs::read_to_string(duong_dan)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<LoaiTaiLieu>>(&s).ok())
        .filter(|ds| !ds.is_empty())
        .unwrap_or_else(co_san)
}

pub fn ghi(duong_dan: &Path, ds: &[LoaiTaiLieu]) -> std::io::Result<()> {
    if let Some(cha) = duong_dan.parent() {
        std::fs::create_dir_all(cha)?;
    }
    let s = serde_json::to_string_pretty(ds).map_err(std::io::Error::other)?;
    std::fs::write(duong_dan, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bien_sinh_tu_ten_cot() {
        assert_eq!(bien_tu_ten("Số, ký hiệu"), "so_ky_hieu");
        assert_eq!(bien_tu_ten("Ngày tháng"), "ngay_thang");
        assert_eq!(bien_tu_ten("Người sử dụng đất"), "nguoi_su_dung_dat");
    }

    #[test]
    fn van_ban_hanh_chinh_dung_prototype() {
        let vb = &co_san()[0];
        let bien: Vec<_> = vb.truong.iter().map(|t| t.bien.as_str()).collect();
        assert_eq!(bien, ["loai_van_ban", "so_ky_hieu", "ngay_thang", "tac_gia", "trich_yeu", "nguoi_ky"]);
        assert_eq!(vb.truong[0].danh_sach.len(), 8);
        assert_eq!(vb.truong[5].lay_trang(), LayTrang::Cuoi);
        assert_eq!(vb.truong[1].lay_trang(), LayTrang::Dau);
    }

    #[test]
    fn trang_doc_theo_so_trang() {
        assert_eq!(TrangDoc::DauCuoi.cac_trang(1), [1]);
        assert_eq!(TrangDoc::DauCuoi.cac_trang(5), [1, 5]);
        assert_eq!(TrangDoc::Dau.cac_trang(5), [1]);
        assert_eq!(TrangDoc::TatCa.cac_trang(3), [1, 2, 3]);
        assert!(TrangDoc::TatCa.cac_trang(0).is_empty());
    }

    #[test]
    fn ghi_doc_vong_va_file_hong() {
        let d = std::env::temp_dir().join(format!("dema4_bt_loai_{}", std::process::id()));
        let f = d.join("loai.json");
        let mut ds = co_san();
        ds[0].truong[0].danh_sach.push("Nghị quyết".into());
        ghi(&f, &ds).unwrap();
        assert_eq!(doc(&f), ds);
        std::fs::write(&f, "{hỏng").unwrap();
        assert_eq!(doc(&f), co_san());
        let _ = std::fs::remove_dir_all(&d);
    }
}
