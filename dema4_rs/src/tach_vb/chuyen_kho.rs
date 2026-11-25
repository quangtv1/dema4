//! Chuyển dữ liệu Tách bản 2.3 (theo thư mục: `viec/<tên>-<băm>/` gồm `cache/`, `index_and_blank.txt`,
//! `xoay_trang.txt`, `moc_sua.txt`) vào kho theo id nội dung — 1 lần / thư mục, lúc mở. File cũ giữ nguyên (quay về
//! bản cũ vẫn đọc được tới thời điểm chuyển), chỉ đánh dấu `da_chuyen_kho`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::cache_trang::{self, KhoaFile};
use super::kho::{self, BanTach, Id};
use super::moc_sua;
use super::nhan;

const DA_CHUYEN: &str = "da_chuyen_kho";
const F_NHAN: &str = "index_and_blank.txt";
const F_XOAY: &str = "xoay_trang.txt";

/// Chuyển dữ liệu cũ của thư mục `goc` (các PDF `pdf` với id `ids`) vào kho. Trả số PDF đã chuyển phần Tách.
/// File nhãn / xoay cũ đọc lỗi → không chuyển, không đánh dấu (lần mở sau thử lại; dữ liệu cũ không mất).
pub fn chuyen(goc: &Path, pdf: &[PathBuf], ids: &[Option<Id>]) -> usize {
    let an = super::du_lieu_an::thu_muc_an(goc);
    if !an.is_dir() || an.join(DA_CHUYEN).exists() {
        return 0;
    }
    let doc = |f: &str| std::fs::read_to_string(an.join(f)).ok();
    let nhan_cu = match doc(F_NHAN) {
        None => None,
        Some(s) => match nhan::doc_file_nhan(&s, &|_| None) {
            Ok((b, _)) => Some(b),
            Err(_) => return 0,
        },
    };
    let xoay_cu = match doc(F_XOAY) {
        None => nhan::BangXoay::new(),
        Some(s) => match nhan::doc_file_xoay(&s, &|_| None) {
            Ok(b) => b,
            Err(_) => return 0,
        },
    };
    let moc_cu = moc_sua::doc(&an.join(moc_sua::TEN_FILE));
    let thu_muc_cache = an.join("cache");
    let mut so = 0;
    let mut k = kho::kho();
    for (p, id) in pdf.iter().zip(ids) {
        let Some(id) = *id else { continue };
        let khoa = nhan::duong_dan_ghi(goc, p);
        // Kết quả phân tích + cờ duyệt (cờ duyệt bản cũ nằm trong cache)
        let cache = KhoaFile::cua(p, &khoa).and_then(|kf| cache_trang::doc(&thu_muc_cache, p, &kf)).map(|(_, t)| t);
        let mut duyet = BTreeMap::new();
        if let Some(trang) = &cache {
            for (i, t) in trang.iter().enumerate() {
                if t.trang_thai != 0 {
                    duyet.insert(i + 1, t.trang_thai);
                }
            }
            if !k.co_phan_tich(id) {
                k.ghi_phan_tich(id, trang);
            }
        }
        // Phần Tách: có file nhãn (nhãn đã lưu của mọi hồ sơ) hoặc có góc xoay / mốc
        let x = xoay_cu.get(&khoa).cloned();
        let m = moc_cu.get(&khoa).copied();
        if nhan_cu.is_none() && x.is_none() && m.is_none() {
            continue;
        }
        let n = nhan_cu.as_ref().map(|b| b.get(&khoa).cloned().unwrap_or_default()).unwrap_or_default();
        let x = x.unwrap_or_default();
        let bia_mo = m.is_some_and(|m| m.chu_ky == moc_sua::CHU_KY_RONG) && n.is_empty() && x.is_empty();
        let luc = m.map_or(0, |m| m.luc);
        // Cùng PDF ở 2 thư mục cũ: bản có mốc sau cùng thắng
        if k.tach(id).is_some_and(|cu| cu.luc >= luc) {
            continue;
        }
        let chu_ky = if bia_mo { moc_sua::CHU_KY_RONG } else { kho::chu_ky_tach(&n, &x, &duyet) };
        let so_trang = cache.as_ref().map_or(0, |t| t.len() as u32);
        k.them_tach(id, &BanTach { so_trang, nhan: n, xoay: x, duyet, luc, chu_ky, bia_mo });
        so += 1;
    }
    if k.day().is_ok() {
        let _ = std::fs::write(an.join(DA_CHUYEN), "Dữ liệu thư mục này đã chuyển vào kho_tach.bin (bản 2.4). File cũ giữ để quay về bản cũ.\n");
    }
    so
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chuyen_nhan_xoay_moc_cache_co_duyet_mot_lan() {
        let goc = std::env::temp_dir().join(format!("dema4_chuyen_kho_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&goc);
        std::fs::create_dir_all(&goc).unwrap();
        let (a, b) = (goc.join("a.pdf"), goc.join("b.pdf"));
        crate::tach_vb::tach::tests::tao_pdf(&a, 3);
        crate::tach_vb::tach::tests::tao_pdf(&b, 2);
        let an = super::super::du_lieu_an::thu_muc_an(&goc);
        let _ = std::fs::remove_dir_all(&an);
        std::fs::create_dir_all(&an).unwrap();
        std::fs::write(an.join(F_NHAN), "a.pdf;2;index\n").unwrap();
        std::fs::write(an.join(F_XOAY), "a.pdf;3;90;xac_nhan\n").unwrap();
        std::fs::write(an.join(moc_sua::TEN_FILE), "a.pdf;5000;1\n").unwrap();
        let kf = KhoaFile::cua(&a, "a.pdf").unwrap();
        let mut trang = vec![cache_trang::TrangCache::default(); 3];
        trang[0].trang_thai = cache_trang::GIU_NOI_DUNG;
        cache_trang::ghi(&an.join("cache"), &a, &kf, &trang).unwrap();
        let pdf = vec![a.clone(), b.clone()];
        let ids = kho::ids_cua(&pdf);
        assert_eq!(chuyen(&goc, &pdf, &ids), 2, "a có nhãn; b có file nhãn (không nhãn nào = Nội dung)");
        let (ia, ib) = (ids[0].unwrap(), ids[1].unwrap());
        let mut k = kho::kho();
        let ta = k.tach(ia).unwrap();
        assert_eq!((ta.nhan.get(&2), ta.xoay.get(&3).map(|x| x.goc), ta.duyet.get(&1), ta.luc), (Some(&nhan::Nhan::Index), Some(90), Some(&cache_trang::GIU_NOI_DUNG), 5000));
        assert_eq!(k.phan_tich(ia).unwrap().len(), 3);
        assert!(k.tach(ib).unwrap().nhan.is_empty());
        drop(k);
        assert_eq!(chuyen(&goc, &pdf, &ids), 0, "đã chuyển → không làm lại");
        assert!(an.join(F_NHAN).exists(), "file cũ giữ nguyên");
        let _ = std::fs::remove_dir_all(&an);
        let _ = std::fs::remove_dir_all(&goc);
    }
}
