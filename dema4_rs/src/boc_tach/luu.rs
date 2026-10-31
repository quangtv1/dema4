//! Kết quả bóc từng file ghi nối tiếp `boc_tach.jsonl` trong thư mục dữ liệu ẩn của thư mục nguồn (cạnh file chạy):
//! nguồn sự thật để mở lại thấy kết quả đã đọc (kể cả khi tắt app giữa chừng) và dựng lại Excel. Mỗi dòng 1 bản ghi,
//! cùng file thì bản ghi sau thắng (đọc lại, sửa tay).

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::chay::KetQua;
use super::loai_tai_lieu::LoaiTaiLieu;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanGhi {
    /// Đường dẫn tương đối so với thư mục nguồn.
    pub file: String,
    /// Tên loại tài liệu lúc đọc — đổi loại thì không dùng lại kết quả cũ.
    pub loai: String,
    #[serde(default)]
    pub so_trang: usize,
    /// biến → giá trị đã chuẩn hoá.
    #[serde(default)]
    pub gia_tri: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loi: Option<String>,
    /// Biến đã sửa tay (Lưu ở tab Kết quả) — bảng đáy tô nâu các ô này.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sua: Vec<String>,
    #[serde(default)]
    pub giay: f32,
    /// Câu trả lời thô từng trang (soát khi mô hình trả sai).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tho: Vec<(usize, String)>,
}

impl BanGhi {
    pub fn tu_ket_qua(file: &str, loai: &LoaiTaiLieu, kq: &KetQua) -> Self {
        Self {
            file: file.into(),
            loai: loai.ten.clone(),
            so_trang: kq.so_trang,
            gia_tri: loai.truong.iter().zip(&kq.gia_tri).map(|(t, g)| (t.bien.clone(), g.clone())).collect(),
            loi: None,
            sua: vec![],
            giay: kq.thoi_gian.as_secs_f32(),
            tho: kq.tho.clone(),
        }
    }

    pub fn loi(file: &str, loai: &LoaiTaiLieu, so_trang: usize, loi: &str) -> Self {
        Self { file: file.into(), loai: loai.ten.clone(), so_trang, gia_tri: BTreeMap::new(), loi: Some(loi.into()), sua: vec![], giay: 0.0, tho: vec![] }
    }

    /// Giá trị theo thứ tự trường của loại (biến thiếu → rỗng).
    pub fn theo_truong(&self, loai: &LoaiTaiLieu) -> Vec<String> {
        loai.truong.iter().map(|t| self.gia_tri.get(&t.bien).cloned().unwrap_or_default()).collect()
    }
}

pub fn file_jsonl(thu_muc_nguon: &Path) -> PathBuf {
    crate::tach_vb::du_lieu_an::thu_muc_an(thu_muc_nguon).join("boc_tach.jsonl")
}

pub fn ghi_them(f: &Path, b: &BanGhi) -> std::io::Result<()> {
    if let Some(cha) = f.parent() {
        std::fs::create_dir_all(cha)?;
    }
    // Dòng cuối ghi dở (tắt máy giữa chừng) → xuống dòng trước, không làm hỏng bản ghi mới
    let do_dang = std::fs::read(f).ok().is_some_and(|b| b.last().is_some_and(|c| *c != b'\n'));
    let mut tep = std::fs::OpenOptions::new().create(true).append(true).open(f)?;
    let dong = serde_json::to_string(b).map_err(std::io::Error::other)?;
    writeln!(tep, "{}{dong}", if do_dang { "\n" } else { "" })
}

/// Bản ghi mới nhất của từng file thuộc loại `loai`; dòng hỏng (ghi dở khi tắt máy) bỏ qua.
pub fn doc(f: &Path, loai: &str) -> HashMap<String, BanGhi> {
    let mut ds = HashMap::new();
    let Ok(s) = std::fs::read_to_string(f) else { return ds };
    for b in s.lines().filter_map(|d| serde_json::from_str::<BanGhi>(d).ok()) {
        if b.loai == loai {
            ds.insert(b.file.clone(), b);
        }
    }
    ds
}

#[cfg(test)]
mod tests {
    use super::super::loai_tai_lieu::co_san;
    use super::*;

    #[test]
    fn ghi_doc_ban_ghi_sau_thang_va_bo_dong_hong() {
        let d = std::env::temp_dir().join(format!("dema4_bt_luu_{}", std::process::id()));
        let f = d.join("boc_tach.jsonl");
        let l = &co_san()[0];
        let kq = KetQua { so_trang: 3, gia_tri: ["Tờ trình", "15/TTr", "20/01/2015", "Phòng A", "Dự toán", ""].map(String::from).to_vec(), tho: vec![(1, "{}".into())], thoi_gian: std::time::Duration::from_millis(2100) };
        ghi_them(&f, &BanGhi::tu_ket_qua("HS01/a.pdf", l, &kq)).unwrap();
        ghi_them(&f, &BanGhi::loi("HS01/b.pdf", l, 2, "Mô hình không trả JSON ở trang 1")).unwrap();
        std::fs::OpenOptions::new().append(true).open(&f).unwrap().write_all(b"{\"file\": \"ghi d").unwrap();
        let mut sua = BanGhi::tu_ket_qua("HS01/a.pdf", l, &kq);
        sua.gia_tri.insert("nguoi_ky".into(), "Trần Thị B".into());
        sua.sua = vec!["nguoi_ky".into()];
        ghi_them(&f, &sua).unwrap();
        let ds = doc(&f, &l.ten);
        assert_eq!(ds.len(), 2);
        assert_eq!(ds["HS01/a.pdf"].theo_truong(l)[5], "Trần Thị B", "bản sửa tay ghi sau thắng");
        assert_eq!(ds["HS01/a.pdf"].sua, ["nguoi_ky"]);
        assert_eq!(ds["HS01/b.pdf"].loi.as_deref(), Some("Mô hình không trả JSON ở trang 1"));
        assert!(doc(&f, "Hợp đồng").is_empty(), "loại khác không dùng lại");
        let _ = std::fs::remove_dir_all(&d);
    }
}
