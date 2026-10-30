//! Ghép một hồ sơ vào mẫu đã biên dịch → bytes DOCX (DocxMerger + OpenXmlHelpers của bản gốc):
//! gộp run cùng định dạng → nhân hàng mẫu theo từng văn bản (thay biến dòng) → xoá hàng mẫu →
//! thay biến cấp hồ sơ / hằng toàn tài liệu, `{trang_so}`/`{tong_so_trang}` thành field PAGE / NUMPAGES.

use std::collections::BTreeSet;

use super::doc_bang::BangDuLieu;
use super::docx::{Docx, PART_CHINH};
use super::gom_nhom::HoSo;
use super::mau::MauDaBien;
use super::token;
use super::xml::{self, Nut, The};
use super::RangBuoc;

/// Run "thuần chữ": có `w:t`, mọi thẻ con chỉ là `w:rPr` / `w:t`.
fn la_run_chu(r: &The) -> bool {
    r.con_dau("w:t").is_some() && r.the_con().all(|c| c.ten == "w:rPr" || c.ten == "w:t")
}

fn rpr_xml(r: &The) -> String {
    r.con_dau("w:rPr").map(|p| xml::ghi(&[Nut::The(p.clone())])).unwrap_or_default()
}

fn noi_chu_w_t(r: &mut The, them: &str) {
    if let Some(t) = r.con_dau_mut("w:t") {
        let moi = t.chu() + them;
        t.dat_chu(&moi);
        t.dat_thuoc_tinh("xml:space", "preserve");
    }
}

fn chu_w_t_dau(r: &The) -> String {
    r.con_dau("w:t").map(The::chu).unwrap_or_default()
}

/// Áp `f` cho mọi đoạn `w:p` (kể cả đoạn lồng trong ô / hộp chữ).
fn moi_doan(goc: &mut The, f: &mut dyn FnMut(&mut The)) {
    goc.moi_hau_due_mut("w:p", f);
}

/// Gộp các run liền kề (trong cùng đoạn) cùng định dạng — chỉ run thuần chữ.
fn gop_run(goc: &mut The) {
    moi_doan(goc, &mut |p| {
        let mut moi: Vec<Nut> = Vec::with_capacity(p.con.len());
        let (mut truoc, mut rpr_truoc): (Option<usize>, String) = (None, String::new());
        for n in std::mem::take(&mut p.con) {
            let Nut::The(r) = &n else {
                moi.push(n);
                continue;
            };
            if r.ten != "w:r" {
                moi.push(n);
                continue;
            }
            let (don, rpr) = (la_run_chu(r), rpr_xml(r));
            if let (true, Some(i)) = (don, truoc) {
                if rpr == rpr_truoc {
                    let them = chu_w_t_dau(r);
                    if let Nut::The(dich) = &mut moi[i] {
                        noi_chu_w_t(dich, &them);
                    }
                    continue;
                }
            }
            truoc = don.then_some(moi.len());
            rpr_truoc = rpr;
            moi.push(n);
        }
        p.con = moi;
    });
}

/// Gộp các run bị Word cắt GIỮA một token `{…}` về run mở token (giữ định dạng run đó); chỉ gộp khi token thật sự đóng.
fn noi_token_cat_vun(goc: &mut The) {
    moi_doan(goc, &mut |p| {
        let mut i = 0;
        loop {
            let run: Vec<usize> = p.con.iter().enumerate().filter(|(_, n)| matches!(n, Nut::The(t) if t.ten == "w:r")).map(|(k, _)| k).collect();
            if i >= run.len() {
                break;
            }
            let chu_cua = |k: usize| match &p.con[k] {
                Nut::The(r) if la_run_chu(r) => Some(chu_w_t_dau(r)),
                _ => None,
            };
            let Some(t) = chu_cua(run[i]).filter(|t| token::duoi_mo(t)) else {
                i += 1;
                continue;
            };
            let mut duoi = t[t.rfind('{').unwrap()..].to_string();
            let mut dong_o = None;
            for (j, &k) in run.iter().enumerate().skip(i + 1) {
                let Some(nt) = chu_cua(k) else { break };
                duoi.push_str(&nt);
                if token::dau_dong(&duoi) {
                    dong_o = Some(j);
                    break;
                }
                if !token::duoi_mo(&duoi) {
                    break;
                }
            }
            if let Some(j) = dong_o {
                let them: String = run[i + 1..=j].iter().map(|&k| chu_cua(k).unwrap()).collect();
                if let Nut::The(r) = &mut p.con[run[i]] {
                    noi_chu_w_t(r, &them);
                }
                for &k in run[i + 1..=j].iter().rev() {
                    p.con.remove(k);
                }
            }
            i += 1;
        }
    });
}

fn tao_run(rpr: Option<&The>, chu: &str) -> The {
    let mut r = The::moi("w:r");
    if let Some(p) = rpr {
        r.con.push(Nut::The(p.clone()));
    }
    let mut t = The::moi("w:t");
    t.dat_thuoc_tinh("xml:space", "preserve");
    t.dat_chu(chu);
    r.con.push(Nut::The(t));
    r
}

fn tao_field(rpr: Option<&The>, lenh: &str) -> The {
    let mut f = The::moi("w:fldSimple");
    f.dat_thuoc_tinh("w:instr", lenh);
    f.con.push(Nut::The(tao_run(rpr, "1")));
    f
}

struct BoThay<'a> {
    gia_tri: &'a dyn Fn(&str) -> Option<String>,
    tu_dong: Option<&'a BTreeSet<String>>,
    anh: &'a BTreeSet<String>,
}

/// Thay token trong một run; trả về các nút thay thế khi phải dựng lại run (field / ảnh), None nếu sửa tại chỗ.
fn thay_trong_run(r: &mut The, b: &BoThay) -> Option<Vec<Nut>> {
    let s = r.con_dau("w:t")?.chu();
    let ds = token::tim(&s);
    if ds.is_empty() {
        return None;
    }
    let co_field = b.tu_dong.is_some_and(|tu| ds.iter().any(|(_, _, n)| tu.contains(*n)));
    let co_anh = ds.iter().any(|(_, _, n)| b.anh.contains(*n));
    if !co_field && !co_anh {
        let moi = token::thay(&s, b.gia_tri);
        let t = r.con_dau_mut("w:t").unwrap();
        t.dat_chu(&moi);
        t.dat_thuoc_tinh("xml:space", "preserve");
        return None;
    }
    let rpr = r.con_dau("w:rPr").cloned();
    let mut manh = Vec::new();
    let mut cuoi = 0;
    for (i, n, ten) in ds {
        if i > cuoi {
            manh.push(Nut::The(tao_run(rpr.as_ref(), &s[cuoi..i])));
        }
        if b.tu_dong.is_some_and(|tu| tu.contains(ten)) {
            manh.push(Nut::The(tao_field(rpr.as_ref(), if ten == "trang_so" { "PAGE" } else { "NUMPAGES" })));
        } else if b.anh.contains(ten) {
            // Ảnh theo đường dẫn: giai đoạn biến ảnh; chưa có ảnh hợp lệ → run rỗng như bản gốc
            manh.push(Nut::The(The::moi("w:r")));
        } else {
            let v = (b.gia_tri)(ten).unwrap_or_else(|| s[i..i + n].to_string());
            manh.push(Nut::The(tao_run(rpr.as_ref(), &v)));
        }
        cuoi = i + n;
    }
    if cuoi < s.len() {
        manh.push(Nut::The(tao_run(rpr.as_ref(), &s[cuoi..])));
    }
    Some(manh)
}

fn thay_de_quy(the: &mut The, b: &BoThay) {
    let mut i = 0;
    while i < the.con.len() {
        if let Nut::The(c) = &mut the.con[i] {
            if c.ten == "w:r" {
                if let Some(manh) = thay_trong_run(c, b) {
                    let n = manh.len();
                    the.con.splice(i..=i, manh);
                    i += n;
                    continue;
                }
            }
            thay_de_quy(c, b);
        }
        i += 1;
    }
}

/// Thay mọi `{bien}` trong phạm vi (sau khi nối token bị cắt vụn).
fn thay_token(pham_vi: &mut The, b: &BoThay) {
    noi_token_cat_vun(pham_vi);
    thay_de_quy(pham_vi, b);
}

pub fn ghep(mau: &MauDaBien, bang: &BangDuLieu, hs: &HoSo, rb: &RangBuoc) -> Result<Vec<u8>, String> {
    let mut docx = Docx::doc(&mau.bytes)?;
    let mut tai_lieu = docx.xml(PART_CHINH)?;
    {
        let than = xml::goc_mut(&mut tai_lieu).and_then(|g| g.con_dau_mut("w:body")).ok_or("document.xml thiếu w:body")?;
        gop_run(than);

        // 1) Nhân hàng mẫu theo từng văn bản của hồ sơ
        let mut dem = 0;
        let mut xong = false;
        than.moi_hau_due_mut("w:tbl", &mut |bang_xml| {
            if xong || dem != mau.bang {
                dem += 1;
                return;
            }
            dem += 1;
            xong = true;
            let hang: Vec<usize> = bang_xml.con.iter().enumerate().filter(|(_, n)| matches!(n, Nut::The(t) if t.ten == "w:tr")).map(|(k, _)| k).collect();
            let Some(&vi_tri) = hang.get(mau.hang_mau) else { return };
            let Nut::The(mau_hang) = bang_xml.con[vi_tri].clone() else { return };
            let mut moi = Vec::with_capacity(hs.dong.len());
            for &d in &hs.dong {
                let mut h = mau_hang.clone();
                let gia_tri = |ten: &str| if mau.bien_dong.contains(ten) { rb.cot(ten).map(|c| bang.o(d, c).to_string()) } else { None };
                thay_token(&mut h, &BoThay { gia_tri: &gia_tri, tu_dong: None, anh: &mau.bien_anh_token });
                moi.push(Nut::The(h));
            }
            bang_xml.con.splice(vi_tri..=vi_tri, moi);
        });

        // 2) Biến cấp hồ sơ / hằng + field tự động trên thân
        let gia_tri = |ten: &str| hs.gia_tri.get(ten).cloned();
        thay_token(than, &BoThay { gia_tri: &gia_tri, tu_dong: Some(&mau.bien_tu_dong), anh: &mau.bien_anh_token });
    }
    docx.dat_xml(PART_CHINH, &tai_lieu);
    // … và đầu trang / chân trang
    for ten in docx.dau_chan_trang() {
        let mut phan = docx.xml(&ten)?;
        if let Some(g) = xml::goc_mut(&mut phan) {
            let gia_tri = |t: &str| hs.gia_tri.get(t).cloned();
            thay_token(g, &BoThay { gia_tri: &gia_tri, tu_dong: Some(&mau.bien_tu_dong), anh: &mau.bien_anh_token });
        }
        docx.dat_xml(&ten, &phan);
    }
    docx.ghi()
}

/// Tóm tắt nội dung DOCX để so sánh: mỗi đoạn một dòng (chữ + `[PAGE]`…), mỗi bảng số hàng — theo part.
#[cfg(test)]
pub fn tom_tat(bytes: &[u8]) -> Vec<String> {
    fn doan(t: &The, ra: &mut String) {
        for n in &t.con {
            if let Nut::The(c) = n {
                match c.ten.as_str() {
                    "w:rPr" if t.ten == "w:r" => {
                        let mut dd: Vec<String> = c.the_con().map(|x| format!("{}{}", x.ten, x.thuoc_tinh("w:val").map(|v| format!("={v}")).unwrap_or_default())).collect();
                        dd.sort();
                        ra.push_str(&format!("⟨{}⟩", dd.join(",")));
                    }
                    "w:tab" => ra.push('\t'),
                    "w:br" => ra.push('\n'),
                    "w:t" => ra.push_str(&c.chu()),
                    "w:fldSimple" => ra.push_str(&format!("[{}]", c.thuoc_tinh("w:instr").unwrap_or("").trim())),
                    _ => doan(c, ra),
                }
            }
        }
    }
    fn duyet(t: &The, ra: &mut Vec<String>) {
        for n in &t.con {
            if let Nut::The(c) = n {
                match c.ten.as_str() {
                    "w:p" => {
                        let mut s = String::new();
                        doan(c, &mut s);
                        ra.push(s);
                    }
                    "w:tbl" => {
                        ra.push(format!("<bảng {} hàng>", c.the_con().filter(|h| h.ten == "w:tr").count()));
                        duyet(c, ra);
                    }
                    _ => duyet(c, ra),
                }
            }
        }
    }
    let d = Docx::doc(bytes).unwrap();
    let mut ra = Vec::new();
    for ten in std::iter::once(PART_CHINH.to_string()).chain(d.dau_chan_trang()) {
        ra.push(format!("# {ten}"));
        duyet(xml::goc(&d.xml(&ten).unwrap()).unwrap(), &mut ra);
    }
    ra
}

#[cfg(test)]
mod tests {
    use super::super::{gom_nhom, mau, Nguon};
    use super::*;

    fn bang_mau() -> BangDuLieu {
        BangDuLieu {
            tieu_de: ["HS", "STT", "SKH", "Ngày", "TG", "TY", "Tờ", "GC"].map(String::from).to_vec(),
            dong: vec![
                ["HS-1", "1", "01/QĐ", "01/01/2020", "UBND", "Trích yếu <A> & B", "01", ""].map(String::from).to_vec(),
                ["HS-1", "2", "02/BC", "02/01/2020", "UBND", "Báo cáo", "02-03", "Bản sao"].map(String::from).to_vec(),
                ["HS-1", "3", "03/TTr", "03/01/2020", "Sở", "Tờ trình", "04", ""].map(String::from).to_vec(),
            ],
            so_dong: vec![2, 3, 4],
        }
    }

    fn rb_mau01() -> RangBuoc {
        let c = |b: &str, c: &str| (b.to_string(), Nguon::Cot(c.into()));
        RangBuoc {
            cot_nhom: "HS".into(),
            bien: vec![c("so_ho_so", "HS"), ("don_vi".into(), Nguon::Hang("ĐƠN VỊ".into())), c("stt", "STT"), c("so_ky_hieu", "SKH"), c("ngay_thang", "Ngày"), c("tac_gia", "TG"), c("trich_yeu", "TY"), c("to_so", "Tờ"), c("ghi_chu", "GC")],
        }
    }

    #[test]
    fn ghep_nhan_hang_mau_va_thay_bien() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[0].1).unwrap();
        let (b, rb) = (bang_mau(), rb_mau01());
        let hs = &gom_nhom::gom(&b, &rb).unwrap()[0];
        let ra = ghep(&m, &b, hs, &rb).unwrap();
        let tt = tom_tat(&ra);
        let s = tt.join("\n");
        assert!(s.contains("<bảng 4 hàng>"), "tiêu đề + 3 văn bản, hàng mẫu biến mất:\n{s}");
        assert!(s.contains("ĐƠN VỊ") && s.contains("HS-1") && s.contains("Trích yếu <A> & B") && s.contains("02-03"));
        assert!(s.contains("{chi_nhanh}") && s.contains("{nguoi_lap}"), "biến chưa ghép giữ nguyên token");
        assert!(!s.contains("{stt}") && !s.contains("{trich_yeu}"));
    }

    #[test]
    fn field_trang_o_chan_trang() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[3].1).unwrap();
        let rb = RangBuoc { cot_nhom: "HS".into(), bien: vec![("so_ho_so".into(), Nguon::Cot("HS".into())), ("trang_so".into(), Nguon::TuDong), ("tong_so_trang".into(), Nguon::TuDong)] };
        let b = bang_mau();
        let ra = ghep(&m, &b, &gom_nhom::gom(&b, &rb).unwrap()[0], &rb).unwrap();
        let s = tom_tat(&ra).join("\n");
        let dong = s.lines().find(|l| l.contains("[PAGE]")).unwrap_or_else(|| panic!("{s}"));
        assert!(dong.contains("Trang") && dong.contains("[NUMPAGES]") && !s.contains("{trang_so}"), "{dong}");
    }

    #[test]
    fn noi_token_bi_word_cat_vun() {
        let x = r#"<w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Số: {so_</w:t></w:r><w:proofErr/><w:r><w:t>ho</w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t>_so} tiếp</w:t></w:r><w:r><w:t>{ lạc</w:t></w:r><w:r><w:t>chuẩn}</w:t></w:r></w:p></w:body>"#;
        let mut n = xml::doc(x).unwrap();
        let g = xml::goc_mut(&mut n).unwrap();
        noi_token_cat_vun(g);
        let ra = xml::ghi(&n);
        assert!(ra.contains(r#"<w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">Số: {so_ho_so} tiếp</w:t></w:r>"#), "{ra}");
        assert!(ra.contains("<w:proofErr/>") && ra.contains("<w:t>{ lạc</w:t>"), "'{{' lạc chuẩn không bị gộp: {ra}");
    }

    #[test]
    fn gop_run_cung_dinh_dang() {
        let x = r#"<w:body><w:p><w:r><w:t>A</w:t></w:r><w:r><w:t>B</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>C</w:t></w:r><w:r><w:tab/><w:t>D</w:t></w:r><w:r><w:t>E</w:t></w:r></w:p></w:body>"#;
        let mut n = xml::doc(x).unwrap();
        gop_run(xml::goc_mut(&mut n).unwrap());
        assert_eq!(xml::ghi(&n), r#"<w:body><w:p><w:r><w:t xml:space="preserve">AB</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>C</w:t></w:r><w:r><w:tab/><w:t>D</w:t></w:r><w:r><w:t>E</w:t></w:r></w:p></w:body>"#);
    }

    /// So với DOCX do lõi gốc C# sinh (temp/muc_luc: Excel thật, không có trong repo) — 4 mẫu × 22 hồ sơ.
    #[test]
    #[ignore = "cần temp/muc_luc (dữ liệu thật + DOCX đối chứng)"]
    fn doi_chung_core_goc() {
        let goc = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../temp/muc_luc");
        let bang = super::super::doc_bang::doc(&goc.join("bien_muc_mau.xlsx"), Some("KhanhLinh"), 1).unwrap();
        for (i, (ten, b)) in mau::MAU_CO_SAN.iter().enumerate() {
            let rb = RangBuoc::tu_json(&std::fs::read_to_string(goc.join(format!("rb-0{}.json", i + 1))).unwrap());
            let m = mau::bien_dich(b).unwrap();
            let ho_so = gom_nhom::gom(&bang, &rb).unwrap();
            assert_eq!(ho_so.len(), 22, "{ten}");
            for hs in &ho_so {
                let ta = tom_tat(&ghep(&m, &bang, hs, &rb).unwrap());
                let goc_cs = tom_tat(&std::fs::read(goc.join(format!("doi_chung_ra/mau-0{}/{:03}.docx", i + 1, hs.chi_so))).unwrap());
                assert_eq!(ta, goc_cs, "{ten} hồ sơ {} ({})", hs.chi_so, hs.khoa);
            }
        }
    }
}
