//! Trang mô phỏng từ DOCX (chưa cần Word / LibreOffice): đầu trang, thân (đoạn văn + bảng), chân trang —
//! chỉ chữ, căn lề, đậm. Dùng cho khung xem mẫu (biến hiện thành chip) và xem nhanh mục lục đã ghép của 1 hồ sơ.

use super::docx::{Docx, PART_CHINH};
use super::xml::{self, The};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Can {
    Trai,
    Giua,
    Phai,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Doan {
    pub chu: String,
    pub can: Can,
    pub dam: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Khoi {
    Doan(Doan),
    /// Hàng → ô; mỗi ô gồm các đoạn của nó.
    Bang(Vec<Vec<Vec<Doan>>>),
}

fn doan(p: &The) -> Doan {
    let ppr = p.con_dau("w:pPr");
    let can = match ppr.and_then(|x| x.con_dau("w:jc")).and_then(|j| j.thuoc_tinh("w:val")) {
        Some("center") => Can::Giua,
        Some("right") | Some("end") => Can::Phai,
        _ => Can::Trai,
    };
    // Đậm: định dạng run mặc định của đoạn, hoặc run đầu tiên có chữ
    let dam_cua = |rpr: Option<&The>| rpr.and_then(|r| r.con_dau("w:b")).is_some_and(|b| !matches!(b.thuoc_tinh("w:val"), Some("0") | Some("false")));
    let mut runs = Vec::new();
    p.hau_due("w:r", &mut runs);
    let run_chu = runs.into_iter().find(|r| !r.chu_w_t().trim().is_empty());
    let dam = dam_cua(ppr.and_then(|x| x.con_dau("w:rPr"))) || dam_cua(run_chu.and_then(|r| r.con_dau("w:rPr")));
    Doan { chu: p.chu_w_t(), can, dam }
}

fn khoi_cua(cha: &The, ra: &mut Vec<Khoi>) {
    for t in cha.the_con() {
        match t.ten.as_str() {
            "w:p" => ra.push(Khoi::Doan(doan(t))),
            "w:tbl" => {
                let hang = t
                    .the_con()
                    .filter(|h| h.ten == "w:tr")
                    .map(|h| {
                        h.the_con()
                            .filter(|o| o.ten == "w:tc")
                            .map(|o| {
                                // Bảng lồng trong ô: lấy chữ theo đoạn, bỏ cấu trúc bảng con
                                let mut p = Vec::new();
                                o.hau_due("w:p", &mut p);
                                p.into_iter().map(doan).collect()
                            })
                            .collect()
                    })
                    .collect();
                ra.push(Khoi::Bang(hang));
            }
            // Nội dung trong khối điều khiển (w:sdt) vẫn hiển thị
            "w:sdt" => {
                if let Some(c) = t.con_dau("w:sdtContent") {
                    khoi_cua(c, ra);
                }
            }
            _ => {}
        }
    }
}

/// Đầu trang (header) + thân + chân trang (footer), theo thứ tự đó.
pub fn mo_phong(bytes: &[u8]) -> Result<Vec<Khoi>, String> {
    let d = Docx::doc(bytes)?;
    let mut ra = Vec::new();
    let phan = d.dau_chan_trang();
    let doc_phan = |ten: &str, ra: &mut Vec<Khoi>| -> Result<(), String> {
        let nut = d.xml(ten)?;
        if let Some(g) = xml::goc(&nut) {
            khoi_cua(g, ra);
        }
        Ok(())
    };
    for ten in phan.iter().filter(|t| t.contains("header")) {
        doc_phan(ten, &mut ra)?;
    }
    let tai_lieu = d.xml(PART_CHINH)?;
    let than = xml::goc(&tai_lieu).and_then(|g| g.con_dau("w:body")).ok_or("document.xml thiếu w:body")?;
    khoi_cua(than, &mut ra);
    for ten in phan.iter().filter(|t| t.contains("footer")) {
        doc_phan(ten, &mut ra)?;
    }
    // Bỏ đoạn trống ở đầu / cuối cho gọn
    while matches!(ra.last(), Some(Khoi::Doan(d)) if d.chu.trim().is_empty()) {
        ra.pop();
    }
    Ok(ra)
}

#[cfg(test)]
mod tests {
    use super::super::{ghep, gom_nhom, mau, Nguon, RangBuoc};
    use super::*;

    fn chu_tat_ca(k: &[Khoi]) -> String {
        k.iter()
            .map(|k| match k {
                Khoi::Doan(d) => d.chu.clone(),
                Khoi::Bang(h) => h.iter().flatten().flatten().map(|d| d.chu.clone()).collect::<Vec<_>>().join("|"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn mau_co_bang_va_bien() {
        for (ten, b) in mau::MAU_CO_SAN {
            let k = mo_phong(b).unwrap_or_else(|e| panic!("{ten}: {e}"));
            let m = mau::bien_dich(b).unwrap();
            let chu = chu_tat_ca(&k);
            assert!(k.iter().any(|k| matches!(k, Khoi::Bang(h) if !h.is_empty())), "{ten}: có bảng");
            for b in m.bien_dong.iter().chain(&m.bien_ho_so).chain(&m.bien_tu_dong) {
                assert!(chu.contains(&format!("{{{b}}}")), "{ten}: thấy biến {{{b}}}");
            }
        }
        let k = mo_phong(mau::MAU_CO_SAN[0].1).unwrap();
        assert!(k.iter().any(|k| matches!(k, Khoi::Doan(d) if d.can == Can::Giua && d.dam)), "có đoạn căn giữa, đậm (tiêu đề)");
    }

    #[test]
    fn ket_qua_ghep_da_thay_bien() {
        let m = mau::bien_dich(mau::MAU_CO_SAN[0].1).unwrap();
        let bang = super::super::doc_bang::BangDuLieu {
            tieu_de: vec!["HS".into(), "TY".into()],
            dong: vec![vec!["HS-01".into(), "Quyết định A".into()], vec!["HS-01".into(), "Tờ trình B".into()]],
            so_dong: vec![2, 3],
        };
        let rb = RangBuoc { cot_nhom: "HS".into(), bien: vec![("so_ho_so".into(), Nguon::Cot("HS".into())), ("trich_yeu".into(), Nguon::Cot("TY".into()))] };
        let hs = gom_nhom::gom(&bang, &rb).unwrap();
        let chu = chu_tat_ca(&mo_phong(&ghep::ghep(&m, &bang, &hs[0], &rb).unwrap()).unwrap());
        assert!(chu.contains("HS-01") && chu.contains("Quyết định A") && chu.contains("Tờ trình B"));
        assert!(!chu.contains("{so_ho_so}") && !chu.contains("{trich_yeu}"));
        assert!(mo_phong(b"rac").is_err());
    }
}
