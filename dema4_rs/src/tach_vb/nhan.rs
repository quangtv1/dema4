//! Nhãn trang và dải văn bản: đọc/ghi `index_and_blank.txt`, `xoay_trang.txt`, tính các văn bản sẽ tách.
//!
//! Nhãn: `index` (tờ phân cách — điểm cắt, trang bị bỏ), `blank` (trang trắng — bị bỏ),
//! `split` (cắt trước trang này, trang được giữ — khi lúc scan quên chèn tờ index); không nhãn = nội dung.
//! `index_and_blank.txt` giữ đúng định dạng cũ `đường_dẫn;trang;index|blank` (+ `split`) để script Python dùng tiếp.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nhan {
    NoiDung,
    Index,
    Trang,
    Split,
}

impl Nhan {
    fn tu_chu(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "index" => Some(Nhan::Index),
            "blank" => Some(Nhan::Trang),
            "split" => Some(Nhan::Split),
            _ => None,
        }
    }

    fn chu(self) -> Option<&'static str> {
        match self {
            Nhan::Index => Some("index"),
            Nhan::Trang => Some("blank"),
            Nhan::Split => Some("split"),
            Nhan::NoiDung => None,
        }
    }
}

/// Góc xoay thêm khi tách (theo chiều kim đồng hồ) + trang Nghi xoay đã được người dùng xác nhận.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xoay {
    pub goc: u16,
    pub xac_nhan: bool,
}

/// Lỗi ở 1 dòng của file nhãn / file xoay (dòng đếm từ 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoiDong {
    pub dong: usize,
    pub ly_do: String,
}

/// Nhãn theo hồ sơ: đường dẫn (dùng `/`) → trang (1-based) → nhãn. Không lưu trang nội dung.
pub type BangNhan = BTreeMap<String, BTreeMap<usize, Nhan>>;
/// Góc xoay theo hồ sơ: đường dẫn → trang → xoay.
pub type BangXoay = BTreeMap<String, BTreeMap<usize, Xoay>>;

/// Chuẩn hoá đường dẫn trong file: bỏ khoảng trắng hai đầu, `\` → `/`.
pub fn chuan_duong_dan(p: &str) -> String {
    p.trim().replace('\\', "/")
}

/// Đường dẫn thật của hồ sơ ghi trong file nhãn: tuyệt đối giữ nguyên; tương đối tính từ `goc`
/// (app: thư mục đầu vào = cha của `.sm-dema4/`; file kiểu cũ như mẫu 1: thư mục chứa file nhãn).
pub fn duong_dan_that(goc: &std::path::Path, p: &str) -> std::path::PathBuf {
    let p = chuan_duong_dan(p);
    let tuyet_doi = p.starts_with('/') || p.as_bytes().get(1) == Some(&b':');
    if tuyet_doi { std::path::PathBuf::from(p) } else { p.split('/').filter(|t| !t.is_empty()).fold(goc.to_path_buf(), |d, t| d.join(t)) }
}

/// Đường dẫn ghi vào file nhãn: tương đối từ `goc`, luôn dùng `/` (cả trên Windows).
pub fn duong_dan_ghi(goc: &std::path::Path, p: &std::path::Path) -> String {
    let rel = p.strip_prefix(goc).unwrap_or(p);
    rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Đọc chung 2 loại file `đường_dẫn;trang;giá_trị[;…]`: bỏ dòng trống, kiểm số trang theo `so_trang_cua`
/// (`None` = không biết số trang của file đó, chỉ kiểm ≥ 1). Mỗi dòng hợp lệ gọi `nap(đường dẫn, trang, các cột còn lại)`.
fn doc_dong<F>(noi_dung: &str, so_trang_cua: &dyn Fn(&str) -> Option<usize>, mut nap: F) -> Vec<LoiDong>
where
    F: FnMut(String, usize, &[&str], usize) -> Result<(), String>,
{
    let mut loi = Vec::new();
    for (i, dong) in noi_dung.lines().enumerate() {
        let so_dong = i + 1;
        let dong = dong.trim_start_matches('\u{feff}');
        if dong.trim().is_empty() {
            continue;
        }
        let cot: Vec<&str> = dong.split(';').collect();
        if cot.len() < 3 {
            loi.push(LoiDong { dong: so_dong, ly_do: format!("thiếu cột (cần đường_dẫn;trang;giá_trị): {dong:?}") });
            continue;
        }
        let duong_dan = chuan_duong_dan(cot[0]);
        let Ok(trang) = cot[1].trim().parse::<usize>() else {
            loi.push(LoiDong { dong: so_dong, ly_do: format!("số trang không hợp lệ: {:?}", cot[1].trim()) });
            continue;
        };
        let toi_da = so_trang_cua(&duong_dan);
        if trang == 0 || toi_da.is_some_and(|n| trang > n) {
            let pham_vi = toi_da.map_or("≥ 1".to_string(), |n| format!("1–{n}"));
            loi.push(LoiDong { dong: so_dong, ly_do: format!("trang {trang} ngoài phạm vi {pham_vi} của {duong_dan}") });
            continue;
        }
        if let Err(ly_do) = nap(duong_dan, trang, &cot[2..], so_dong) {
            loi.push(LoiDong { dong: so_dong, ly_do });
        }
    }
    loi
}

/// Đọc `index_and_blank.txt`. Trùng trang → nhãn sau thắng, trả kèm cảnh báo. Có lỗi → trả mọi dòng lỗi.
pub fn doc_file_nhan(noi_dung: &str, so_trang_cua: &dyn Fn(&str) -> Option<usize>) -> Result<(BangNhan, Vec<String>), Vec<LoiDong>> {
    let mut bang = BangNhan::new();
    let mut canh_bao = Vec::new();
    let loi = doc_dong(noi_dung, so_trang_cua, |duong_dan, trang, cot, so_dong| {
        if cot.len() != 1 {
            return Err(format!("thừa cột: {cot:?}"));
        }
        let nhan = Nhan::tu_chu(cot[0]).ok_or_else(|| format!("nhãn không hợp lệ: {:?} (cần index, blank hoặc split)", cot[0].trim()))?;
        if let Some(cu) = bang.entry(duong_dan.clone()).or_default().insert(trang, nhan) {
            if cu != nhan {
                canh_bao.push(format!("dòng {so_dong}: {duong_dan} trang {trang} có 2 nhãn, dùng nhãn sau ({})", cot[0].trim()));
            }
        }
        Ok(())
    });
    if loi.is_empty() { Ok((bang, canh_bao)) } else { Err(loi) }
}

/// Ghi `index_and_blank.txt`: sắp theo đường dẫn rồi trang; bỏ trang nội dung.
pub fn ghi_file_nhan(bang: &BangNhan) -> String {
    let mut ra = String::new();
    for (duong_dan, trang) in bang {
        for (so, nhan) in trang {
            if let Some(chu) = nhan.chu() {
                ra.push_str(&format!("{duong_dan};{so};{chu}\n"));
            }
        }
    }
    ra
}

/// Đọc `xoay_trang.txt` (`đường_dẫn;trang;0|90|180|270[;xac_nhan]`); góc 0 chỉ hợp lệ khi có `xac_nhan`.
pub fn doc_file_xoay(noi_dung: &str, so_trang_cua: &dyn Fn(&str) -> Option<usize>) -> Result<BangXoay, Vec<LoiDong>> {
    let mut bang = BangXoay::new();
    let loi = doc_dong(noi_dung, so_trang_cua, |duong_dan, trang, cot, _| {
        let goc: u16 = cot[0].trim().parse().map_err(|_| format!("góc không hợp lệ: {:?}", cot[0].trim()))?;
        if ![0, 90, 180, 270].contains(&goc) {
            return Err(format!("góc {goc} không hợp lệ (cần 0, 90, 180 hoặc 270)"));
        }
        let xac_nhan = match cot.get(1).map(|s| s.trim()) {
            None => false,
            Some("xac_nhan") if cot.len() == 2 => true,
            Some(khac) => return Err(format!("cột thứ 4 không hợp lệ: {khac:?} (chỉ nhận xac_nhan)")),
        };
        if goc == 0 && !xac_nhan {
            return Err("góc 0 chỉ ghi kèm xac_nhan".to_string());
        }
        bang.entry(duong_dan).or_default().insert(trang, Xoay { goc, xac_nhan });
        Ok(())
    });
    if loi.is_empty() { Ok(bang) } else { Err(loi) }
}

/// Ghi `xoay_trang.txt`: sắp theo đường dẫn rồi trang; bỏ trang 0° chưa xác nhận.
pub fn ghi_file_xoay(bang: &BangXoay) -> String {
    let mut ra = String::new();
    for (duong_dan, trang) in bang {
        for (so, x) in trang {
            if x.goc != 0 || x.xac_nhan {
                ra.push_str(&format!("{duong_dan};{so};{}{}\n", x.goc, if x.xac_nhan { ";xac_nhan" } else { "" }));
            }
        }
    }
    ra
}

/// Một văn bản sẽ tách ra từ hồ sơ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VanBan {
    /// Số thứ tự part, liên tục chỉ trên các văn bản thực sự sinh ra (1..).
    pub so: usize,
    /// Dải trang gốc [tu, den] của văn bản (gồm cả trang trắng bị bỏ ở giữa).
    pub tu: usize,
    pub den: usize,
    /// Trang trắng trong dải, bị bỏ khi tách.
    pub bo: Vec<usize>,
    /// Trang giữ lại theo thứ tự + góc xoay thêm (0|90|180|270).
    pub trang_giu: Vec<(usize, u16)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaiVanBan {
    pub van_ban: Vec<VanBan>,
    /// Hồ sơ không nhãn và không trang xoay: chép nguyên file thay vì ghi lại.
    pub nguyen_ven: bool,
    /// Dải chỉ còn trang trắng → không sinh part.
    pub canh_bao: Vec<String>,
}

/// Tính các văn bản của 1 hồ sơ `so_trang` trang (cùng ngữ nghĩa `create_ranges` của b3, thêm split và bỏ dải toàn trắng).
pub fn tinh_dai(so_trang: usize, nhan: &BTreeMap<usize, Nhan>, xoay: &BTreeMap<usize, Xoay>) -> DaiVanBan {
    let goc = |p: usize| xoay.get(&p).map_or(0, |x| x.goc);
    let co_xoay = xoay.values().any(|x| x.goc != 0);
    let co_nhan = nhan.values().any(|&n| n != Nhan::NoiDung);
    if !co_nhan && !co_xoay {
        let ca_ho_so = VanBan { so: 1, tu: 1, den: so_trang, bo: Vec::new(), trang_giu: (1..=so_trang).map(|p| (p, 0)).collect() };
        return DaiVanBan { van_ban: if so_trang > 0 { vec![ca_ho_so] } else { Vec::new() }, nguyen_ven: so_trang > 0, canh_bao: Vec::new() };
    }
    let mut kq = DaiVanBan { van_ban: Vec::new(), nguyen_ven: false, canh_bao: Vec::new() };
    let dong = |tu: usize, den: usize, kq: &mut DaiVanBan| {
        if tu > den {
            return;
        }
        let bo: Vec<usize> = (tu..=den).filter(|p| nhan.get(p) == Some(&Nhan::Trang)).collect();
        let trang_giu: Vec<(usize, u16)> = (tu..=den).filter(|p| nhan.get(p) != Some(&Nhan::Trang)).map(|p| (p, goc(p))).collect();
        if trang_giu.is_empty() {
            kq.canh_bao.push(format!("dải {tu}–{den} chỉ có trang trắng, không tách"));
            return;
        }
        let so = kq.van_ban.len() + 1;
        kq.van_ban.push(VanBan { so, tu, den, bo, trang_giu });
    };
    let mut tu = 1;
    for p in 1..=so_trang {
        match nhan.get(&p) {
            Some(Nhan::Index) => {
                dong(tu, p - 1, &mut kq);
                tu = p + 1;
            }
            Some(Nhan::Split) => {
                dong(tu, p - 1, &mut kq);
                tu = p;
            }
            _ => {}
        }
    }
    dong(tu, so_trang, &mut kq);
    kq
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nhan_tu(ds: &[(usize, Nhan)]) -> BTreeMap<usize, Nhan> {
        ds.iter().copied().collect()
    }
    fn dai(so_trang: usize, ds: &[(usize, Nhan)]) -> Vec<(usize, usize)> {
        tinh_dai(so_trang, &nhan_tu(ds), &BTreeMap::new()).van_ban.iter().map(|v| (v.tu, v.den)).collect()
    }
    use Nhan::*;

    #[test]
    fn khong_nhan_la_1_van_ban_nguyen_ven() {
        let kq = tinh_dai(5, &BTreeMap::new(), &BTreeMap::new());
        assert!(kq.nguyen_ven);
        assert_eq!(kq.van_ban.len(), 1);
        assert_eq!(kq.van_ban[0].trang_giu.len(), 5);
    }

    #[test]
    fn chi_co_trang_trang_khong_nguyen_ven() {
        let kq = tinh_dai(5, &nhan_tu(&[(2, Trang)]), &BTreeMap::new());
        assert!(!kq.nguyen_ven);
        assert_eq!(kq.van_ban[0].bo, vec![2]);
        assert_eq!(kq.van_ban[0].trang_giu, vec![(1, 0), (3, 0), (4, 0), (5, 0)]);
    }

    #[test]
    fn khong_nhan_nhung_co_xoay_phai_ghi_lai_file() {
        let xoay: BTreeMap<usize, Xoay> = [(3, Xoay { goc: 90, xac_nhan: false })].into_iter().collect();
        let kq = tinh_dai(4, &BTreeMap::new(), &xoay);
        assert!(!kq.nguyen_ven);
        assert_eq!(kq.van_ban[0].trang_giu, vec![(1, 0), (2, 0), (3, 90), (4, 0)]);
        // 0° đã xác nhận không tính là xoay
        let xn: BTreeMap<usize, Xoay> = [(3, Xoay { goc: 0, xac_nhan: true })].into_iter().collect();
        assert!(tinh_dai(4, &BTreeMap::new(), &xn).nguyen_ven);
    }

    #[test]
    fn index_cat_va_bi_bo() {
        assert_eq!(dai(10, &[(3, Index), (8, Index)]), vec![(1, 2), (4, 7), (9, 10)]);
        assert_eq!(dai(10, &[(3, Index), (4, Index)]), vec![(1, 2), (5, 10)]);
        assert_eq!(dai(10, &[(1, Index)]), vec![(2, 10)]);
        assert_eq!(dai(10, &[(10, Index)]), vec![(1, 9)]);
    }

    #[test]
    fn split_cat_truoc_va_giu_trang() {
        assert_eq!(dai(10, &[(4, Split)]), vec![(1, 3), (4, 10)]);
        assert_eq!(dai(10, &[(3, Index), (4, Split)]), vec![(1, 2), (4, 10)]);
        assert_eq!(dai(10, &[(1, Split)]), vec![(1, 10)]);
        assert_eq!(dai(10, &[(4, Split), (5, Split)]), vec![(1, 3), (4, 4), (5, 10)]);
        assert_eq!(dai(10, &[(2, Split), (6, Index), (8, Split)]), vec![(1, 1), (2, 5), (7, 7), (8, 10)]);
    }

    #[test]
    fn dai_toan_trang_bi_bo_va_canh_bao() {
        let kq = tinh_dai(5, &nhan_tu(&[(1, Trang), (2, Split)]), &BTreeMap::new());
        assert_eq!(kq.van_ban.iter().map(|v| (v.tu, v.den)).collect::<Vec<_>>(), vec![(2, 5)]);
        assert_eq!(kq.canh_bao.len(), 1);
        assert_eq!(kq.van_ban[0].so, 1, "đánh số chỉ trên văn bản sinh ra");
        let kq = tinh_dai(6, &nhan_tu(&[(2, Index), (3, Trang), (4, Index)]), &BTreeMap::new());
        assert_eq!(kq.van_ban.iter().map(|v| (v.so, v.tu, v.den)).collect::<Vec<_>>(), vec![(1, 1, 1), (2, 5, 6)]);
    }

    /// Hàm chuẩn viết thẳng theo định nghĩa: trang giữ = nội dung + split; văn bản mới khi gặp split
    /// hoặc có index nằm giữa 2 trang giữ liền kề; mọi văn bản không rỗng.
    fn chuan(so_trang: usize, nhan: &BTreeMap<usize, Nhan>) -> Vec<Vec<usize>> {
        let mut ds: Vec<Vec<usize>> = Vec::new();
        let mut co_index_tu_lan_truoc = false;
        for p in 1..=so_trang {
            match nhan.get(&p) {
                Some(Index) => co_index_tu_lan_truoc = true,
                Some(Trang) => {}
                n => {
                    if ds.is_empty() || co_index_tu_lan_truoc || n == Some(&Split) {
                        ds.push(Vec::new());
                    }
                    ds.last_mut().unwrap().push(p);
                    co_index_tu_lan_truoc = false;
                }
            }
        }
        ds
    }

    #[test]
    fn thuoc_tinh_khop_ham_chuan_tren_to_hop_ngau_nhien() {
        let mut hat: u64 = 0x5eed_1234;
        let mut ngau_nhien = || {
            hat ^= hat << 13;
            hat ^= hat >> 7;
            hat ^= hat << 17;
            hat
        };
        for _ in 0..200_000 {
            let n = (ngau_nhien() % 12 + 1) as usize;
            let mut nhan = BTreeMap::new();
            for p in 1..=n {
                match ngau_nhien() % 5 {
                    0 => { nhan.insert(p, Index); }
                    1 => { nhan.insert(p, Trang); }
                    2 => { nhan.insert(p, Split); }
                    _ => {}
                }
            }
            let kq = tinh_dai(n, &nhan, &BTreeMap::new());
            let ta: Vec<Vec<usize>> = kq.van_ban.iter().map(|v| v.trang_giu.iter().map(|t| t.0).collect()).collect();
            assert_eq!(ta, chuan(n, &nhan), "n={n} nhãn={nhan:?}");
            assert!(kq.van_ban.iter().enumerate().all(|(i, v)| v.so == i + 1 && !v.trang_giu.is_empty()));
        }
    }

    #[test]
    fn duong_dan_tuong_doi_va_tuyet_doi() {
        let goc = std::path::Path::new("/du_lieu/mau2");
        assert_eq!(duong_dan_that(goc, "HOP 462/4766/4766.pdf"), goc.join("HOP 462").join("4766").join("4766.pdf"));
        assert_eq!(duong_dan_that(goc, "HOP 462\\4766\\4766.pdf"), goc.join("HOP 462").join("4766").join("4766.pdf"));
        assert_eq!(duong_dan_that(goc, "/khac/a.pdf"), std::path::PathBuf::from("/khac/a.pdf"));
        assert_eq!(duong_dan_that(goc, "C:\\x\\a.pdf"), std::path::PathBuf::from("C:/x/a.pdf"));
        assert_eq!(duong_dan_ghi(goc, &goc.join("HOP 462").join("4766.pdf")), "HOP 462/4766.pdf");
    }

    fn khong_biet(_: &str) -> Option<usize> {
        None
    }

    #[test]
    fn doc_ghi_file_nhan_khu_hoi() {
        let vao = "HOP 462/4766/4766.pdf;3;index\nHOP 462/4766/4766.pdf;7;blank\na\\b.pdf;2;SPLIT\n\n  HOP 462/4766/4766.pdf ; 9 ; Index \n";
        let (bang, cb) = doc_file_nhan(vao, &khong_biet).unwrap();
        assert!(cb.is_empty());
        assert_eq!(bang["a/b.pdf"][&2], Split);
        let ra = ghi_file_nhan(&bang);
        assert_eq!(ra, "HOP 462/4766/4766.pdf;3;index\nHOP 462/4766/4766.pdf;7;blank\nHOP 462/4766/4766.pdf;9;index\na/b.pdf;2;split\n");
        assert_eq!(ghi_file_nhan(&doc_file_nhan(&ra, &khong_biet).unwrap().0), ra);
    }

    #[test]
    fn doc_file_nhan_bao_loi_co_so_dong_va_trung_trang() {
        let loi = doc_file_nhan("a.pdf;1;index\na.pdf;x;index\na.pdf;2\na.pdf;3;khac\na.pdf;0;blank\n", &khong_biet).unwrap_err();
        assert_eq!(loi.iter().map(|l| l.dong).collect::<Vec<_>>(), vec![2, 3, 4, 5]);
        let loi = doc_file_nhan("a.pdf;12;index\n", &|_| Some(10)).unwrap_err();
        assert_eq!(loi[0].dong, 1);
        let (bang, cb) = doc_file_nhan("a.pdf;2;index\na.pdf;2;blank\n", &khong_biet).unwrap();
        assert_eq!(bang["a.pdf"][&2], Trang);
        assert_eq!(cb.len(), 1);
    }

    #[test]
    fn doc_ghi_file_xoay() {
        let vao = "a.pdf;3;90\na.pdf;5;0;xac_nhan\nb.pdf;1;270;xac_nhan\n";
        let bang = doc_file_xoay(vao, &khong_biet).unwrap();
        assert_eq!(bang["a.pdf"][&5], Xoay { goc: 0, xac_nhan: true });
        assert_eq!(ghi_file_xoay(&bang), vao);
        let loi = doc_file_xoay("a.pdf;1;45\na.pdf;2;0\na.pdf;3;90;khac\na.pdf;4;90\n", &khong_biet).unwrap_err();
        assert_eq!(loi.iter().map(|l| l.dong).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    /// Mẫu 1: nhãn của b3 + số trang 491/405 ⇒ đúng 211 dải của `ketqua_tach.txt` (dải, tên part, trang trắng).
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn khop_211_dai_cua_b3() {
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let so_trang = |p: &str| match p.rsplit('/').next() { Some("L1_05_13.pdf") => Some(491), Some("L1_06_14.pdf") => Some(405), _ => None };
        let (bang, _) = doc_file_nhan(&std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap(), &so_trang).unwrap();
        let mut ta = Vec::new();
        for (duong_dan, nhan) in &bang {
            let ten = duong_dan.rsplit('/').next().unwrap();
            let stem = ten.trim_end_matches(".pdf");
            for v in tinh_dai(so_trang(duong_dan).unwrap(), nhan, &BTreeMap::new()).van_ban {
                ta.push(format!("{ten}\t{}-{}\t{stem}_part{}.pdf\t{:?}", v.tu, v.den, v.so, v.bo));
            }
        }
        let b3: Vec<String> = std::fs::read_to_string(mau.join("ketqua_tach.txt")).unwrap().lines().skip(1)
            .map(|d| {
                let c: Vec<&str> = d.split('\t').collect();
                // b3 ghi danh sách trang trắng sau khi đã trừ 1 (đếm từ 0) → cộng lại 1 để so
                let trang: Vec<usize> = c[4].trim_matches(['[', ']']).split(',').filter(|x| !x.trim().is_empty()).map(|x| x.trim().parse::<usize>().unwrap() + 1).collect();
                format!("{}\t{}\t{}\t{:?}", c[1], c[2], c[3], trang)
            })
            .collect();
        assert_eq!(ta.len(), 211);
        assert_eq!(ta, b3);
    }
}
