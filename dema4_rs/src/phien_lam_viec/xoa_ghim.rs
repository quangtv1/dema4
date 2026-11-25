//! Phần Xóa ghim trong gói phiên (mục tuỳ chọn `xoa_ghim.bin` — bản cũ không đọc mục này nên vẫn nhập được gói):
//! vùng riêng + nhật ký của các PDF trong thư mục (theo id nội dung) và vùng chung + tuỳ chọn của thư mục.
//!
//! Nhập: chỉ nhận cho PDF có trên máy này; vùng giữ bản có mốc sửa sau (mốc gói kẹp như phần Tách); nội dung
//! sai / ngoài giới hạn bị bỏ. Nhật ký chỉ nhận khi máy này **chưa có** nhật ký của PDF đó — nhật ký của máy này chỉ
//! tới file kết quả có thật ở đây, thay bằng nhật ký máy khác sẽ thành "kết quả mất" và mất chặn ghi đè.

use std::collections::HashSet;
use std::path::Path;

use crate::tach_vb::kho::{Id, Kho};
use crate::xoa_ghim::kho::{self as xg, NhatKy, ThuMuc, VungFile};

pub(super) const F_XOA_GHIM: &str = "xoa_ghim.bin";
/// Khoá trong gói của vùng chung thư mục (máy nhận đổi sang khoá thư mục của mình).
const KHOA_THU_MUC: Id = (0, 0);

/// Đóng gói phần Xóa ghim của thư mục `goc` có các PDF `ids`. Bản ghi: `[loại][mã 8][kích thước 8][độ dài 4][nội dung]`.
pub(super) fn dong_goi(k: &mut Kho, goc: &Path, ids: &HashSet<Id>) -> Vec<u8> {
    let mut b = Vec::new();
    let mut them = |loai: u8, id: Id, nd: Vec<u8>| {
        b.push(loai);
        b.extend(id.0.to_le_bytes());
        b.extend(id.1.to_le_bytes());
        b.extend((nd.len() as u32).to_le_bytes());
        b.extend(nd);
    };
    if let Some(nd) = k.khac(xg::VUNG_THU_MUC, xg::khoa_thu_muc(goc)) {
        them(xg::VUNG_THU_MUC, KHOA_THU_MUC, nd);
    }
    for loai in [xg::VUNG_FILE, xg::NHAT_KY] {
        let mut khoa: Vec<Id> = k.khoa_khac(loai).into_iter().filter(|id| ids.contains(id)).collect();
        khoa.sort_unstable();
        for id in khoa {
            if let Some(nd) = k.khac(loai, id) {
                them(loai, id, nd);
            }
        }
    }
    b
}

fn doc(b: &[u8]) -> Vec<(u8, Id, &[u8])> {
    let mut ds = Vec::new();
    let mut i = 0;
    while i + 21 <= b.len() {
        let loai = b[i];
        let id = (u64::from_le_bytes(b[i + 1..i + 9].try_into().unwrap()), u64::from_le_bytes(b[i + 9..i + 17].try_into().unwrap()));
        let n = u32::from_le_bytes(b[i + 17..i + 21].try_into().unwrap()) as usize;
        if i + 21 + n > b.len() {
            break;
        }
        ds.push((loai, id, &b[i + 21..i + 21 + n]));
        i += 21 + n;
    }
    ds
}

/// Kiểm nội dung theo loại và kẹp mốc sửa (`kep`) → nội dung ghi được + mốc; sai → `None`.
fn kiem(loai: u8, nd: &[u8], mut kep: impl FnMut(u64) -> u64) -> Option<(Vec<u8>, u64)> {
    Some(match loai {
        xg::VUNG_THU_MUC => {
            let mut x = ThuMuc::giai_ma(nd)?;
            x.luc = kep(x.luc);
            (x.ma_hoa(), x.luc)
        }
        xg::VUNG_FILE => {
            let mut x = VungFile::giai_ma(nd)?;
            x.luc = kep(x.luc);
            (x.ma_hoa(), x.luc)
        }
        xg::NHAT_KY => {
            let mut x = NhatKy::giai_ma(nd)?;
            x.luc = kep(x.luc);
            (x.ma_hoa(), x.luc)
        }
        _ => return None,
    })
}

/// Kết quả nhập phần Xóa ghim.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KetQuaNhap {
    pub nhan: usize,
    pub giu: usize,
    pub bo: usize,
    pub lech_dong_ho: usize,
}

/// Gộp phần Xóa ghim của gói vào kho `k` cho thư mục `goc` (PDF có trên máy: `co`). Chưa đẩy xuống đĩa.
pub(super) fn nhap(k: &mut Kho, goc: &Path, b: &[u8], co: &HashSet<Id>, bay_gio: u64) -> KetQuaNhap {
    let mut kq = KetQuaNhap::default();
    for (loai, id, nd) in doc(b) {
        let khoa = match loai {
            xg::VUNG_THU_MUC if id == KHOA_THU_MUC => xg::khoa_thu_muc(goc),
            xg::VUNG_FILE | xg::NHAT_KY if co.contains(&id) => id,
            _ => {
                kq.bo += 1;
                continue;
            }
        };
        let Some((nd, luc)) = kiem(loai, nd, |l| {
            let (l, lech) = super::gop::kep(l, bay_gio);
            kq.lech_dong_ho += lech as usize;
            l
        }) else {
            kq.bo += 1;
            continue;
        };
        let co_may = k.khac(loai, khoa);
        let luc_may = co_may.as_deref().and_then(xg::luc_cua);
        if (loai == xg::NHAT_KY && co_may.is_some()) || luc_may.is_some_and(|m| m >= luc) {
            kq.giu += 1;
        } else {
            k.ghi_khac(loai, khoa, &nd);
            kq.nhan += 1;
        }
    }
    kq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nhat_ky_may_nay_khong_bi_thay_vung_moc_sau_thang() {
        let goc = Path::new("/thu_muc");
        let nk = |luc, kq| NhatKy { luc, ket_qua: kq, ma_cau_hinh: 0, so_trang: 1, dpi: 0, ai: false, trang: vec![] };
        let mut goi_kho = Kho::bo_nho();
        xg::ghi_nhat_ky(&mut goi_kho, (1, 1), &nk(900, (8, 8)));
        xg::ghi_nhat_ky(&mut goi_kho, (2, 2), &nk(900, (9, 9)));
        xg::ghi_vung_file(&mut goi_kho, (1, 1), &VungFile { luc: 900, vung: vec![] });
        let ids: HashSet<Id> = [(1, 1), (2, 2)].into_iter().collect();
        let b = dong_goi(&mut goi_kho, goc, &ids);
        let mut may = Kho::bo_nho();
        xg::ghi_nhat_ky(&mut may, (1, 1), &nk(100, (5, 5)));
        let kq = nhap(&mut may, goc, &b, &ids, 1_000);
        assert_eq!(xg::doc_nhat_ky(&mut may, (1, 1)).unwrap().ket_qua, (5, 5), "nhật ký máy này giữ dù cũ hơn");
        assert_eq!(xg::doc_nhat_ky(&mut may, (2, 2)).unwrap().ket_qua, (9, 9), "chưa có thì nhận");
        assert_eq!((kq.nhan, kq.giu), (1, 1));
    }
}
