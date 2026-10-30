//! Gói DOCX (zip): giữ mọi part theo thứ tự gốc, đọc / ghi lại các part XML cần sửa.

use std::io::{Cursor, Read, Write};

use super::xml::{self, Nut};

pub struct Docx {
    tep: Vec<(String, Vec<u8>)>,
}

pub const PART_CHINH: &str = "word/document.xml";

impl Docx {
    pub fn doc(bytes: &[u8]) -> Result<Self, String> {
        let mut z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("không phải file DOCX: {e}"))?;
        let mut tep = Vec::with_capacity(z.len());
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            let mut b = Vec::new(); // không tin kích thước khai trong file zip (file hỏng / độc)
            f.read_to_end(&mut b).map_err(|e| e.to_string())?;
            let ten = f.name().map_err(|e| e.to_string())?.into_owned();
            tep.push((ten, b));
        }
        if !tep.iter().any(|(t, _)| t == PART_CHINH) {
            return Err("DOCX thiếu word/document.xml".into());
        }
        Ok(Docx { tep })
    }

    pub fn ghi(&self) -> Result<Vec<u8>, String> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let tuy_chon = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (ten, b) in &self.tep {
            if ten.ends_with('/') {
                z.add_directory(ten.trim_end_matches('/'), tuy_chon).map_err(|e| e.to_string())?;
                continue;
            }
            z.start_file(ten, tuy_chon).map_err(|e| e.to_string())?;
            z.write_all(b).map_err(|e| e.to_string())?;
        }
        Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
    }

    pub fn xml(&self, ten: &str) -> Result<Vec<Nut>, String> {
        let (_, b) = self.tep.iter().find(|(t, _)| t == ten).ok_or_else(|| format!("thiếu {ten}"))?;
        xml::doc(std::str::from_utf8(b).map_err(|_| format!("{ten} không phải UTF-8"))?)
    }

    pub fn dat_xml(&mut self, ten: &str, nut: &[Nut]) {
        let b = xml::ghi(nut).into_bytes();
        match self.tep.iter_mut().find(|(t, _)| t == ten) {
            Some((_, cu)) => *cu = b,
            None => self.tep.push((ten.into(), b)),
        }
    }

    /// Part đầu trang rồi chân trang theo relationship của document.xml (như HeaderParts / FooterParts);
    /// thiếu relationship → theo tên `word/header*.xml`, `word/footer*.xml`.
    pub fn dau_chan_trang(&self) -> Vec<String> {
        if let Ok(rels) = self.xml("word/_rels/document.xml.rels") {
            if let Some(g) = xml::goc(&rels) {
                let theo_loai = |duoi: &str| -> Vec<String> {
                    g.the_con()
                        .filter(|r| r.thuoc_tinh("Type").is_some_and(|t| t.ends_with(duoi)) && r.thuoc_tinh("TargetMode") != Some("External"))
                        .filter_map(|r| r.thuoc_tinh("Target"))
                        .map(|t| t.strip_prefix('/').map_or_else(|| format!("word/{t}"), String::from))
                        .filter(|t| self.tep.iter().any(|(ten, _)| ten == t))
                        .collect()
                };
                let mut ds = theo_loai("/header");
                ds.extend(theo_loai("/footer"));
                return ds;
            }
        }
        self.dau_chan_trang_theo_ten()
    }

    fn dau_chan_trang_theo_ten(&self) -> Vec<String> {
        let la = |t: &str, dau: &str| t.strip_prefix(dau).and_then(|r| r.strip_suffix(".xml")).is_some_and(|so| so.chars().all(|c| c.is_ascii_digit()));
        let mut dau: Vec<String> = self.tep.iter().map(|(t, _)| t.clone()).filter(|t| la(t, "word/header")).collect();
        dau.extend(self.tep.iter().map(|(t, _)| t.clone()).filter(|t| la(t, "word/footer")));
        dau
    }
}
