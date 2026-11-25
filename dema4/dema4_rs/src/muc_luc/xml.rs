//! Cây XML tối giản cho các part WordprocessingML: giữ nguyên tên thẻ có tiền tố (`w:p`, `w:r`…), thuộc tính
//! theo thứ tự gốc, nút khác (khai báo, chú thích) dạng thô — đủ để gộp run, nhân hàng bảng, thay token rồi ghi lại.

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

#[derive(Debug, Clone, PartialEq)]
pub enum Nut {
    The(The),
    /// Chữ đã giải mã thực thể (`&amp;` → `&`); ghi lại sẽ mã hoá.
    Chu(String),
    /// Khai báo / chú thích / CDATA / PI: chép nguyên văn.
    Tho(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct The {
    pub ten: String,
    pub thuoc_tinh: Vec<(String, String)>,
    pub con: Vec<Nut>,
}

impl The {
    pub fn moi(ten: &str) -> Self {
        The { ten: ten.into(), thuoc_tinh: Vec::new(), con: Vec::new() }
    }

    pub fn thuoc_tinh(&self, ten: &str) -> Option<&str> {
        self.thuoc_tinh.iter().find(|(k, _)| k == ten).map(|(_, v)| v.as_str())
    }

    pub fn dat_thuoc_tinh(&mut self, ten: &str, gia_tri: &str) {
        match self.thuoc_tinh.iter_mut().find(|(k, _)| k == ten) {
            Some((_, v)) => *v = gia_tri.into(),
            None => self.thuoc_tinh.push((ten.into(), gia_tri.into())),
        }
    }

    /// Thẻ con trực tiếp.
    pub fn the_con(&self) -> impl Iterator<Item = &The> {
        self.con.iter().filter_map(|n| if let Nut::The(t) = n { Some(t) } else { None })
    }

    pub fn con_dau(&self, ten: &str) -> Option<&The> {
        self.the_con().find(|t| t.ten == ten)
    }

    pub fn con_dau_mut(&mut self, ten: &str) -> Option<&mut The> {
        self.con.iter_mut().find_map(|n| match n {
            Nut::The(t) if t.ten == ten => Some(t),
            _ => None,
        })
    }

    /// Chữ của mọi `w:t` hậu duệ, theo thứ tự tài liệu.
    pub fn chu_w_t(&self) -> String {
        let mut s = String::new();
        fn gom(t: &The, s: &mut String) {
            for n in &t.con {
                if let Nut::The(c) = n {
                    if c.ten == "w:t" {
                        s.push_str(&c.chu());
                    } else {
                        gom(c, s);
                    }
                }
            }
        }
        gom(self, &mut s);
        s
    }

    /// Chữ trực tiếp của thẻ (nối các nút chữ con).
    pub fn chu(&self) -> String {
        self.con.iter().filter_map(|n| if let Nut::Chu(c) = n { Some(c.as_str()) } else { None }).collect()
    }

    pub fn dat_chu(&mut self, chu: &str) {
        self.con = vec![Nut::Chu(chu.into())];
    }

    /// Hậu duệ tên `ten` theo thứ tự tài liệu (kể cả lồng nhau).
    pub fn hau_due<'a>(&'a self, ten: &str, ra: &mut Vec<&'a The>) {
        for n in &self.con {
            if let Nut::The(c) = n {
                if c.ten == ten {
                    ra.push(c);
                }
                c.hau_due(ten, ra);
            }
        }
    }

    /// Gọi `f` cho mọi hậu duệ tên `ten` (tiền thứ tự). `f` được sửa thẻ đó nhưng không đi vào con của nó sau khi sửa.
    pub fn moi_hau_due_mut(&mut self, ten: &str, f: &mut dyn FnMut(&mut The)) {
        for n in &mut self.con {
            if let Nut::The(c) = n {
                if c.ten == ten {
                    f(c);
                }
                c.moi_hau_due_mut(ten, f);
            }
        }
    }
}

fn ma_hoa(s: &str, thuoc_tinh: bool, ra: &mut String) {
    for ch in s.chars() {
        match ch {
            '&' => ra.push_str("&amp;"),
            '<' => ra.push_str("&lt;"),
            '>' => ra.push_str("&gt;"),
            '"' if thuoc_tinh => ra.push_str("&quot;"),
            // Thuộc tính: xuống dòng / tab ghi dạng tham chiếu, nếu không đọc lại sẽ thành dấu cách
            '\n' if thuoc_tinh => ra.push_str("&#xA;"),
            '\t' if thuoc_tinh => ra.push_str("&#x9;"),
            '\r' => ra.push_str("&#xD;"),
            // Ký tự điều khiển XML 1.0 không cho phép (từ dữ liệu Excel) → bỏ
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            _ => ra.push(ch),
        }
    }
}

fn ghi_nut(n: &Nut, ra: &mut String) {
    match n {
        Nut::Chu(c) => ma_hoa(c, false, ra),
        Nut::Tho(t) => ra.push_str(t),
        Nut::The(t) => {
            ra.push('<');
            ra.push_str(&t.ten);
            for (k, v) in &t.thuoc_tinh {
                ra.push(' ');
                ra.push_str(k);
                ra.push_str("=\"");
                ma_hoa(v, true, ra);
                ra.push('"');
            }
            if t.con.is_empty() {
                ra.push_str("/>");
            } else {
                ra.push('>');
                for c in &t.con {
                    ghi_nut(c, ra);
                }
                ra.push_str("</");
                ra.push_str(&t.ten);
                ra.push('>');
            }
        }
    }
}

/// Ghi danh sách nút gốc (khai báo + phần tử gốc) ra chuỗi XML.
pub fn ghi(goc: &[Nut]) -> String {
    let mut ra = String::new();
    for n in goc {
        ghi_nut(n, &mut ra);
    }
    ra
}

fn tham_chieu(ten: &str) -> Option<String> {
    Some(match ten {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        _ => {
            let so = ten.strip_prefix('#')?;
            let ma = match so.strip_prefix('x').or_else(|| so.strip_prefix('X')) {
                Some(h) => u32::from_str_radix(h, 16).ok()?,
                None => so.parse().ok()?,
            };
            char::from_u32(ma)?.to_string()
        }
    })
}

/// Đọc XML thành danh sách nút gốc.
pub fn doc(xml: &str) -> Result<Vec<Nut>, String> {
    let mut r = Reader::from_str(xml);
    r.config_mut().trim_text(false);
    let mut ngan: Vec<The> = Vec::new();
    let mut goc: Vec<Nut> = Vec::new();
    fn them(ngan: &mut [The], goc: &mut Vec<Nut>, n: Nut) {
        match ngan.last_mut() {
            Some(t) => {
                // Gộp chữ liền kề (chữ bị cắt quanh tham chiếu thực thể)
                if let (Nut::Chu(moi), Some(Nut::Chu(cu))) = (&n, t.con.last_mut()) {
                    cu.push_str(moi);
                    return;
                }
                t.con.push(n)
            }
            None => goc.push(n),
        }
    }
    let the_tu = |e: &quick_xml::events::BytesStart| -> Result<The, String> {
        let ten = e.name().as_ref().to_string();
        let mut thuoc_tinh = Vec::new();
        for a in e.attributes().with_checks(false) {
            let a = a.map_err(|x| x.to_string())?;
            let k = a.key.as_ref().to_string();
            let v = a.normalized_value(XmlVersion::Implicit1_0).map_err(|x| x.to_string())?.into_owned();
            thuoc_tinh.push((k, v));
        }
        Ok(The { ten, thuoc_tinh, con: Vec::new() })
    };
    loop {
        let truoc = r.buffer_position() as usize;
        let ev = r.read_event().map_err(|e| format!("XML lỗi ở byte {}: {e}", r.buffer_position()))?;
        let sau = r.buffer_position() as usize;
        match ev {
            Event::Start(e) => ngan.push(the_tu(&e)?),
            Event::Empty(e) => {
                let t = the_tu(&e)?;
                them(&mut ngan, &mut goc, Nut::The(t));
            }
            Event::End(_) => {
                let t = ngan.pop().ok_or("thẻ đóng thừa")?;
                them(&mut ngan, &mut goc, Nut::The(t));
            }
            Event::Text(_) => them(&mut ngan, &mut goc, Nut::Chu(xml[truoc..sau].to_string())),
            Event::GeneralRef(e) => {
                let ten = e.as_ref().to_string();
                match tham_chieu(&ten) {
                    Some(c) => them(&mut ngan, &mut goc, Nut::Chu(c)),
                    // Thực thể lạ: chép nguyên để ghi lại đúng như cũ
                    None => them(&mut ngan, &mut goc, Nut::Tho(format!("&{ten};"))),
                }
            }
            Event::Eof => break,
            _ => them(&mut ngan, &mut goc, Nut::Tho(xml[truoc..sau].to_string())),
        }
    }
    if !ngan.is_empty() {
        return Err("thiếu thẻ đóng".into());
    }
    Ok(goc)
}

/// Phần tử gốc (thẻ đầu tiên) của tài liệu.
pub fn goc_mut(nut: &mut [Nut]) -> Option<&mut The> {
    nut.iter_mut().find_map(|n| if let Nut::The(t) = n { Some(t) } else { None })
}

pub fn goc(nut: &[Nut]) -> Option<&The> {
    nut.iter().find_map(|n| if let Nut::The(t) = n { Some(t) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_ghi_vong_giu_nguyen() {
        let x = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="urn:w"><!-- chú thích --><w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve"> A &amp; B &lt;{x}&gt; &#272;</w:t></w:r></w:p><w:sectPr w:a="1 &quot;2&quot;"/></w:body></w:document>"#;
        let n = doc(x).unwrap();
        let ra = ghi(&n);
        assert_eq!(ra, x.replace("&#272;", "Đ"));
        assert_eq!(goc(&n).unwrap().chu_w_t(), " A & B <{x}> Đ");
        assert_eq!(doc(&ra).unwrap(), doc(&ra.clone()).unwrap());
    }

    #[test]
    fn ma_hoa_xuong_dong_va_ky_tu_dieu_khien() {
        let mut t = The::moi("a");
        t.dat_thuoc_tinh("descr", "dòng 1\ndòng 2\tx");
        t.dat_chu("a\u{0}b\r\nc");
        let ra = ghi(&[Nut::The(t)]);
        assert_eq!(ra, "<a descr=\"dòng 1&#xA;dòng 2&#x9;x\">ab&#xD;\nc</a>");
        let doc_lai = doc(&ra).unwrap();
        assert_eq!(goc(&doc_lai).unwrap().thuoc_tinh("descr"), Some("dòng 1\ndòng 2\tx"), "đọc lại không mất xuống dòng");
    }

    #[test]
    fn hau_due_theo_thu_tu() {
        let n = doc("<a><b i=\"1\"><b i=\"2\"/></b><c><b i=\"3\"/></c></a>").unwrap();
        let mut ds = Vec::new();
        goc(&n).unwrap().hau_due("b", &mut ds);
        assert_eq!(ds.iter().map(|t| t.thuoc_tinh("i").unwrap()).collect::<String>(), "123");
    }

    #[test]
    fn xml_hong_bao_loi() {
        assert!(doc("<a><b></a>").is_err());
    }
}
