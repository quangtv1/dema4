//! Kết quả Bóc tách theo id nội dung PDF: `boc_tach.jsonl` vẫn riêng từng thư mục (theo đường dẫn), mỗi bản ghi kèm
//! `id`; kho giữ con trỏ id → file jsonl đang có kết quả. Mở thư mục mà file chưa có kết quả nhưng kết quả nằm ở thư
//! mục khác (đổi tên, di chuyển, thư mục cha / con) → chép về theo đường dẫn mới.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::luu::{self, BanGhi};
use crate::tach_vb::du_lieu_an;
use crate::tach_vb::kho::{self, Id};

/// Ghi 1 kết quả của thư mục `goc` (kèm cập nhật con trỏ trong kho khi có id).
pub fn ghi(goc: &Path, b: &BanGhi) -> std::io::Result<()> {
    let f = luu::file_jsonl(goc);
    luu::ghi_them(&f, b)?;
    du_lieu_an::ghi_goc(goc);
    if let Some(id) = b.id {
        let mut k = kho::kho();
        k.ghi_boc(id, &f);
        let _ = k.day();
    }
    Ok(())
}

/// Bổ sung kết quả cho các file (tên tương đối, id) của thư mục `goc` chưa có kết quả ở đây mà có ở thư mục khác (theo
/// id). Bản ghi cũ không có id được nhận ra qua thư mục gốc của file jsonl kia + đường dẫn. Trả số bản ghi đã chép.
pub fn bo_sung(goc: &Path, files: &[(String, Option<Id>)]) -> usize {
    let f_day = luu::file_jsonl(goc);
    let o_day = luu::doc_tat_ca(&std::fs::read_to_string(&f_day).unwrap_or_default());
    let co: HashSet<&str> = o_day.keys().map(|(f, _)| f.as_str()).collect();
    if !o_day.is_empty() {
        // Thư mục gốc của file kết quả này: kết quả bản cũ (không id) ở đây tìm lại được từ chỗ khác
        du_lieu_an::ghi_goc(goc);
    }
    let mut can: HashMap<PathBuf, HashMap<Id, String>> = HashMap::new();
    {
        let mut k = kho::kho();
        for (ten, id) in files {
            let Some(id) = *id else { continue };
            if co.contains(ten.as_str()) {
                // Kết quả có sẵn ở đây (kể cả bản cũ không id): đăng ký con trỏ để chỗ khác tìm được
                if k.boc(id).is_none() {
                    k.ghi_boc(id, &f_day);
                }
                continue;
            }
            if let Some(p) = k.boc(id).filter(|p| *p != f_day) {
                can.entry(p).or_default().insert(id, ten.clone());
            }
        }
        let _ = k.day();
    }
    let mut moi: Vec<BanGhi> = Vec::new();
    for (p, can_id) in can {
        let Ok(nd) = std::fs::read_to_string(&p) else { continue };
        let goc_kia = p.parent().and_then(du_lieu_an::goc_cua);
        for mut b in luu::doc_tat_ca(&nd).into_values() {
            // Bản ghi cũ không id: tính id của file theo thư mục gốc của file kết quả kia (hiếm, 1 lần / file)
            let id = b.id.or_else(|| {
                let f = goc_kia.as_ref()?.join(&b.file);
                let n = std::fs::metadata(&f).ok()?.len();
                Some((crate::tach_vb::cache_trang::van_tay_file(&f, n).ok()?, n))
            });
            if let Some(ten) = id.and_then(|id| can_id.get(&id)) {
                b.file = ten.clone();
                b.id = id;
                moi.push(b);
            }
        }
    }
    if moi.is_empty() || luu::ghi_them_nhieu(&f_day, &moi).is_err() {
        return 0;
    }
    du_lieu_an::ghi_goc(goc);
    let mut k = kho::kho();
    for b in &moi {
        if let Some(id) = b.id {
            k.ghi_boc(id, &f_day);
        }
    }
    let _ = k.day();
    moi.len()
}
