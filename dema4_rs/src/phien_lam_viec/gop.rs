//! Luật gộp thuần (không đọc / ghi file) của Nhập phiên.
//!
//! - Tách: theo hồ sơ, dựa trên mốc sửa (`moc_sua`): bản có mốc lớn hơn thắng, **cho mọi thay đổi** kể cả nhãn tự động
//!   (luật đã chốt). Bằng mốc → giữ bản máy này. Bia mộ (chữ ký rỗng) cũng là 1 bản sửa: mới hơn thì xoá nhãn bên kia.
//! - Bóc tách: theo (file, loại): bản lỗi không bao giờ thắng bản đọc được; còn lại `luc` lớn hơn thắng, bằng → giữ máy này.

use crate::boc_tach::luu::BanGhi;
use crate::tach_vb::moc_sua::Moc;

/// Mốc / thời điểm lớn hơn giờ máy nhận + 1 ngày coi là đồng hồ máy gửi sai: kẹp về giờ máy nhận.
pub const LECH_TOI_DA_MS: u64 = 86_400_000;

/// Kẹp mốc từ gói: `(mốc đã kẹp, có bị kẹp không)`.
pub fn kep(luc: u64, bay_gio: u64) -> (u64, bool) {
    if luc > bay_gio.saturating_add(LECH_TOI_DA_MS) { (bay_gio, true) } else { (luc, false) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KetQua {
    /// Lấy toàn bộ phần của hồ sơ / bản ghi từ gói.
    LayGoi,
    /// Giữ bản máy này.
    GiuMay,
    /// Hai bên giống hệt — không cần làm gì.
    Giong,
}

/// Quyết định cho 1 hồ sơ Tách. `goi` / `may`: mốc sửa 2 bên (đã kẹp đồng hồ; máy này có nhãn mà chưa có mốc → nơi gọi
/// truyền mốc 0). Gói không có mốc cho hồ sơ (nhãn từ trước khi có mốc) → không bao giờ xoá / ghi đè máy này.
pub fn ho_so(goi: Option<Moc>, may: Option<Moc>) -> KetQua {
    match (goi, may) {
        (None, _) => KetQua::GiuMay,
        (Some(_), None) => KetQua::LayGoi,
        (Some(g), Some(m)) if g.chu_ky == m.chu_ky => KetQua::Giong,
        (Some(g), Some(m)) if g.luc > m.luc => KetQua::LayGoi,
        _ => KetQua::GiuMay,
    }
}

/// Quyết định cho 1 bản ghi Bóc tách cùng (file, loại). `goi.luc` đã kẹp đồng hồ.
pub fn ban_ghi(goi: &BanGhi, may: Option<&BanGhi>) -> KetQua {
    let Some(m) = may else { return KetQua::LayGoi };
    if goi.gia_tri == m.gia_tri && goi.loi == m.loi && goi.sua == m.sua {
        return KetQua::Giong;
    }
    match (goi.loi.is_some(), m.loi.is_some()) {
        (true, false) => KetQua::GiuMay, // lỗi không đè kết quả tốt (kể cả sửa tay)
        (false, true) => KetQua::LayGoi,
        _ if goi.luc > m.luc => KetQua::LayGoi,
        _ => KetQua::GiuMay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moc(luc: u64, chu_ky: u64) -> Option<Moc> {
        Some(Moc { luc, chu_ky })
    }

    #[test]
    fn ho_so_sau_cung_thang_bang_moc_giu_may() {
        assert_eq!(ho_so(moc(5, 1), None), KetQua::LayGoi, "chỉ gói có");
        assert_eq!(ho_so(None, moc(5, 1)), KetQua::GiuMay, "chỉ máy có");
        assert_eq!(ho_so(None, None), KetQua::GiuMay, "gói không có mốc → không đụng máy này");
        assert_eq!(ho_so(moc(9, 1), moc(5, 1)), KetQua::Giong, "cùng nội dung");
        assert_eq!(ho_so(moc(9, 1), moc(5, 2)), KetQua::LayGoi, "gói mới hơn");
        assert_eq!(ho_so(moc(5, 1), moc(9, 2)), KetQua::GiuMay, "máy mới hơn");
        assert_eq!(ho_so(moc(5, 1), moc(5, 2)), KetQua::GiuMay, "bằng mốc → giữ máy này");
        assert_eq!(ho_so(moc(3, 1), moc(9, 0)), KetQua::GiuMay, "bia mộ máy này mới hơn → không hồi nhãn cũ của gói");
        assert_eq!(ho_so(moc(9, 0), moc(3, 1)), KetQua::LayGoi, "bia mộ trong gói mới hơn → xoá nhãn máy này");
    }

    #[test]
    fn kep_dong_ho_lech() {
        assert_eq!(kep(1_000, 1_000), (1_000, false));
        assert_eq!(kep(u64::MAX, 1_000), (1_000, true));
        assert_eq!(kep(1_000 + LECH_TOI_DA_MS, 1_000), (1_000 + LECH_TOI_DA_MS, false));
    }

    fn bg(luc: u64, loi: bool, gt: &str) -> BanGhi {
        let mut gia_tri = std::collections::BTreeMap::new();
        gia_tri.insert("so".to_string(), gt.to_string());
        BanGhi { file: "a.pdf".into(), loai: "VB".into(), so_trang: 1, gia_tri, loi: loi.then(|| "500".into()), sua: vec![], canh_bao: None, giay: 0.0, tho: vec![], luc }
    }

    #[test]
    fn ban_ghi_loi_khong_thang_ban_tot_con_lai_sau_cung_thang() {
        assert_eq!(ban_ghi(&bg(9, true, ""), Some(&bg(1, false, "15/TTr"))), KetQua::GiuMay, "lỗi mới hơn vẫn không đè kết quả tốt");
        assert_eq!(ban_ghi(&bg(1, false, "15/TTr"), Some(&bg(9, true, ""))), KetQua::LayGoi, "kết quả tốt thay bản lỗi");
        assert_eq!(ban_ghi(&bg(9, false, "16"), Some(&bg(1, false, "15"))), KetQua::LayGoi);
        assert_eq!(ban_ghi(&bg(1, false, "16"), Some(&bg(9, false, "15"))), KetQua::GiuMay, "sửa tay máy này mới hơn");
        assert_eq!(ban_ghi(&bg(9, false, "15"), Some(&bg(1, false, "15"))), KetQua::Giong);
        assert_eq!(ban_ghi(&bg(1, false, "15"), None), KetQua::LayGoi);
    }
}
