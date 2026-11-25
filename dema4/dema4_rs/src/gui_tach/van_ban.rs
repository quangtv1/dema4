//! Danh sách văn bản sẽ tách (tab Văn bản, dòng Thông tin, nút Tách): liên tục qua mọi hồ sơ, mỗi văn bản là
//! đúng các trang của file part sẽ ghi (cùng hàm dải cắt `tinh_dai` với lệnh tách).

use super::bang_trang::BangTrang;
use crate::dem_pdf::QuyTac;
use crate::tach_vb::nhan::Nhan;

#[derive(Debug, Clone, PartialEq)]
pub struct MucVanBan {
    pub ho_so: u32,
    /// Số part trong hồ sơ (1..).
    pub so: usize,
    /// Dải trang gốc [tu, den] (1-based).
    pub tu: usize,
    pub den: usize,
    /// Trang giữ lại theo thứ tự (chỉ số trong bảng trang).
    pub trang: Vec<u32>,
    /// Trang trắng bị bỏ trong dải (số trang gốc).
    pub bo: Vec<usize>,
    /// Trang đầu mang nhãn Split (văn bản mở đầu bằng điểm cắt tay).
    pub mo_dau_split: bool,
}

/// Tổng cho dòng Thông tin.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TongVanBan {
    pub so_van_ban: usize,
    pub so_trang: usize,
    pub quy_a4: f64,
}

/// Mọi văn bản của mọi hồ sơ, theo thứ tự hồ sơ rồi số part.
pub fn tinh(b: &BangTrang) -> Vec<MucVanBan> {
    let mut ds = Vec::new();
    for h in 0..b.ho_so.len() {
        for vb in b.van_ban(h).van_ban {
            let trang: Vec<u32> = vb.trang_giu.iter().map(|&(so, _)| b.chi_so(h, so)).collect();
            let mo_dau_split = trang.first().is_some_and(|&k| b.trang[k as usize].nhan() == Nhan::Split);
            ds.push(MucVanBan { ho_so: h as u32, so: vb.so, tu: vb.tu, den: vb.den, trang, bo: vb.bo, mo_dau_split });
        }
    }
    ds
}

/// Số văn bản, số trang giữ lại, Quy A4 trên đúng các trang đó.
pub fn tong(b: &BangTrang, ds: &[MucVanBan], quy_tac: &QuyTac) -> TongVanBan {
    let mut t = TongVanBan { so_van_ban: ds.len(), ..Default::default() };
    for &k in ds.iter().flat_map(|v| &v.trang) {
        let p = &b.trang[k as usize];
        let (w, h) = (p.rong as f64, p.cao as f64);
        t.so_trang += 1;
        t.quy_a4 += quy_tac.quy_a4(quy_tac.kho(w, h), w, h);
    }
    t.quy_a4 = crate::dem_pdf::lam_tron_2(t.quy_a4);
    t
}

/// Vị trí mới sau khi danh sách đổi (sửa nhãn): cùng hồ sơ + cùng trang đầu nếu còn; không thì văn bản đầu
/// của hồ sơ đó bắt đầu từ sau trang đầu cũ, rồi văn bản cuối của hồ sơ; hồ sơ hết văn bản → kẹp chỉ số.
pub fn giu_vi_tri(cu: Option<&MucVanBan>, vi_tri_cu: usize, moi: &[MucVanBan]) -> usize {
    if moi.is_empty() {
        return 0;
    }
    let Some(cu) = cu else { return vi_tri_cu.min(moi.len() - 1) };
    let cung_hs: Vec<usize> = (0..moi.len()).filter(|&i| moi[i].ho_so == cu.ho_so).collect();
    if let Some(&i) = cung_hs.iter().find(|&&i| moi[i].tu == cu.tu) {
        return i;
    }
    if let Some(&i) = cung_hs.iter().find(|&&i| moi[i].tu >= cu.tu).or(cung_hs.last()) {
        return i;
    }
    vi_tri_cu.min(moi.len() - 1)
}

/// Ô số + Enter: số 1-based gõ vào → chỉ số, kẹp trong phạm vi.
pub fn tu_o_so(chu: &str, tong: usize) -> Option<usize> {
    let n: i64 = chu.trim().parse().ok()?;
    (tong > 0).then(|| (n.clamp(1, tong as i64) - 1) as usize)
}

/// Tên văn bản theo mẫu Cấu hình (không đuôi .pdf); tên hồ sơ = tên file PDF bỏ đuôi.
pub fn ten(mau: &str, duong_dan_ho_so: &str, so: usize) -> String {
    let file = duong_dan_ho_so.rsplit('/').next().unwrap_or(duong_dan_ho_so);
    let ho_so = file.rsplit_once('.').map_or(file, |(t, _)| t);
    let t = crate::tach_vb::tach::ten_van_ban(mau, ho_so, so);
    t.strip_suffix(".pdf").unwrap_or(&t).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_tach::bang_trang::{Loc, Nhom, Sua, TrangMeta};
    use crate::tach_vb::phan_tich::NhanTuDong;

    const TC: Nhom = Nhom::Loc(Loc::TatCa);

    /// a (6 trang: tr.2 Index, tr.4 Trắng), b (3 trang, tr.2 khổ A3).
    fn bang() -> BangTrang {
        let mut trang = Vec::new();
        for (h, n) in [(0u32, 6u32), (1, 3)] {
            for so in 1..=n {
                let td = Some(NhanTuDong { nhan: Nhan::NoiDung, nghi_trang: false, nghi_xoay: None });
                trang.push(TrangMeta { ho_so: h, so, dung_luong: 0, rong: 595.0, cao: 842.0, tu_dong: td, sua: Sua::default() });
            }
        }
        trang[1].tu_dong = Some(NhanTuDong { nhan: Nhan::Index, nghi_trang: false, nghi_xoay: None });
        trang[3].tu_dong = Some(NhanTuDong { nhan: Nhan::Trang, nghi_trang: false, nghi_xoay: None });
        (trang[7].rong, trang[7].cao) = (842.0, 1191.0);
        BangTrang::moi(vec!["x/a.pdf".into(), "b.pdf".into()], trang)
    }

    #[test]
    fn van_ban_lien_tuc_qua_ho_so_dung_trang_part() {
        let b = bang();
        let ds = tinh(&b);
        assert_eq!(ds.iter().map(|v| (v.ho_so, v.so, v.tu, v.den)).collect::<Vec<_>>(), [(0, 1, 1, 1), (0, 2, 3, 6), (1, 1, 1, 3)]);
        assert_eq!(ds[1].trang, [2, 4, 5], "bỏ index tr.2 và trắng tr.4");
        assert_eq!(ds[1].bo, [4]);
        let t = tong(&b, &ds, &QuyTac::default());
        assert_eq!((t.so_van_ban, t.so_trang, t.quy_a4), (3, 7, 8.0), "6 trang A4 + 1 trang A3");
        assert_eq!(ten("{hoso}_part{n}", "x/a.pdf", 2), "a_part2");
        assert_eq!(ten("{hoso}-{n3}", "b.pdf", 1), "b-001");
    }

    #[test]
    fn gan_nhan_doi_danh_sach_giu_vi_tri() {
        let mut b = bang();
        let cu = tinh(&b);
        // Đang xem a part2 (tr.3–6); gán Index tr.1 → a chỉ còn văn bản tr.3–6 (giờ là part1)
        b.gan(TC, &[0], &[], Nhan::Index);
        let moi = tinh(&b);
        assert_eq!(moi[0].tu, 3);
        assert_eq!(giu_vi_tri(Some(&cu[1]), 1, &moi), 0);
        // Split tr.5 → part tr.3–4 và tr.5–6; vẫn đứng ở văn bản bắt đầu tr.3; văn bản mới mở đầu bằng Split
        b.gan(TC, &[4], &[], Nhan::Split);
        let moi2 = tinh(&b);
        assert_eq!(giu_vi_tri(Some(&moi[0]), 0, &moi2), 0);
        assert!(moi2[1].mo_dau_split && !moi2[0].mo_dau_split);
        // Đang xem văn bản tr.5–6, bỏ Split → nhảy về văn bản cuối của hồ sơ đó
        b.gan(TC, &[4], &[], Nhan::Split);
        assert_eq!(giu_vi_tri(Some(&moi2[1]), 1, &tinh(&b)), 0);
        // Danh sách rỗng
        assert_eq!(giu_vi_tri(Some(&moi2[1]), 5, &[]), 0);
    }

    #[test]
    fn o_so_kep_pham_vi() {
        assert_eq!(tu_o_so("5", 211), Some(4));
        assert_eq!(tu_o_so("0", 211), Some(0));
        assert_eq!(tu_o_so("999", 211), Some(210));
        assert_eq!(tu_o_so("abc", 211), None);
        assert_eq!(tu_o_so("1", 0), None);
    }
}
