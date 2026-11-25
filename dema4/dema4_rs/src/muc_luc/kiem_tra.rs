//! Kiểm tra trước khi tạo (không sinh tài liệu): cột tồn tại, biến đã ghép, hồ sơ liên tiếp, khoá trống.

use super::doc_bang::BangDuLieu;
use super::gom_nhom::{self, HoSo};
use super::mau::MauDaBien;
use super::{Nguon, RangBuoc};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KetQuaKiemTra {
    /// Chặn việc tạo (thiếu cột nhóm, cột không tồn tại, hồ sơ không liên tiếp).
    pub loi: Vec<String>,
    /// Không chặn (biến chưa ghép giữ nguyên `{bien}` trong file, hồ sơ khoá trống).
    pub canh_bao: Vec<String>,
    pub bien_chua_ghep: Vec<String>,
    pub ho_so: Vec<HoSo>,
}

pub fn kiem_tra(mau: &MauDaBien, bang: &BangDuLieu, rb: &RangBuoc) -> KetQuaKiemTra {
    let mut kq = KetQuaKiemTra::default();
    if rb.cot_nhom.is_empty() || bang.cot(&rb.cot_nhom).is_none() {
        kq.loi.push(format!("Chưa chọn cột gom nhóm hợp lệ ('{}')", rb.cot_nhom));
    }
    for (b, n) in &rb.bien {
        if let Nguon::Cot(c) = n {
            if bang.cot(c).is_none() {
                kq.loi.push(format!("Biến {{{b}}}: không có cột '{c}' trong dữ liệu"));
            }
        }
    }
    kq.bien_chua_ghep = mau
        .thu_tu
        .iter()
        .filter(|b| (mau.bien_dong.contains(*b) || mau.bien_ho_so.contains(*b)) && rb.nguon(b).is_none())
        .cloned()
        .collect();
    if !kq.bien_chua_ghep.is_empty() {
        kq.canh_bao.push(format!("Biến chưa ghép (giữ nguyên trong file): {}", kq.bien_chua_ghep.iter().map(|b| format!("{{{b}}}")).collect::<Vec<_>>().join(", ")));
    }
    if !mau.bien_anh_token.is_empty() || !mau.bien_anh.is_empty() {
        kq.canh_bao.push("Mẫu có biến ảnh — phiên bản này chưa chèn ảnh (giữ ảnh gốc / bỏ trống)".into());
    }
    if !kq.loi.is_empty() {
        return kq;
    }
    match gom_nhom::gom(bang, rb) {
        Ok(hs) => {
            let trong = hs.iter().filter(|h| h.khoa.is_empty()).count();
            if trong > 0 {
                kq.canh_bao.push(format!("{trong} hồ sơ có cột gom nhóm trống"));
            }
            kq.ho_so = hs;
        }
        Err(e) => kq.loi.push(format!("Hồ sơ '{}' không liên tiếp: gặp lại ở dòng {} (các dòng cùng hồ sơ phải nằm liền nhau)", e.khoa, e.dong)),
    }
    kq
}

#[cfg(test)]
mod tests {
    use super::super::mau;
    use super::*;

    #[test]
    fn kiem_tra_bao_loi_va_canh_bao() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[2].1).unwrap();
        let bang = BangDuLieu { tieu_de: vec!["HS".into(), "TY".into()], dong: vec![vec!["A".into(), "x".into()], vec!["B".into(), "y".into()], vec!["A".into(), "z".into()]], so_dong: vec![2, 3, 4] };
        let rb = RangBuoc { cot_nhom: "HS".into(), bien: vec![("ho_so_so".into(), Nguon::Cot("HS".into())), ("trich_yeu".into(), Nguon::Cot("TY".into()))] };
        let kq = kiem_tra(&m, &bang, &rb);
        assert_eq!(kq.loi.len(), 1, "{:?}", kq.loi);
        assert!(kq.loi[0].contains("'A' không liên tiếp") && kq.loi[0].contains("dòng 4"), "{:?}", kq.loi);
        assert!(kq.bien_chua_ghep.contains(&"so_ky_hieu".to_string()) && !kq.bien_chua_ghep.contains(&"trich_yeu".to_string()));
        let rb2 = RangBuoc { cot_nhom: "Không có".into(), bien: vec![("ho_so_so".into(), Nguon::Cot("Sai".into()))] };
        assert_eq!(kiem_tra(&m, &bang, &rb2).loi.len(), 2);
        let bang_ok = BangDuLieu { dong: bang.dong[..2].to_vec(), ..bang };
        let kq = kiem_tra(&m, &bang_ok, &rb);
        assert!(kq.loi.is_empty() && kq.ho_so.len() == 2);
    }
}
