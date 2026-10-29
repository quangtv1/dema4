//! Thống kê ngưỡng trắng của 1 lần tách (sau khi người dùng duyệt bằng mắt) để gửi về chỉnh ngưỡng mặc định:
//! histogram chỉ số trắng × nhãn cuối, các sửa của người dùng, ngưỡng đề xuất cho lô, thống kê Nghi xoay.
//! Chỉ số đếm — không ảnh, không tên file PDF, không nội dung trang.

use super::chi_so_trang::LoaiTrang;
use super::nhan::Nhan;

/// 100 ô 0,1 % (0–10 %) + 1 ô gom ≥ 10 %.
pub const SO_O: usize = 101;
/// t1: dưới ngưỡng này tỷ lệ trang có nội dung ≤ 0,2 %.
const NOI_DUNG_TOI_DA: f64 = 0.002;
/// t2: trên ngưỡng này trang trắng lọt ≤ 1 % tổng trang trắng.
const LOT_TOI_DA: f64 = 0.01;
const TRANG_TOI_THIEU: u32 = 50;

/// 1 trang có chỉ số trắng.
#[derive(Debug, Clone, Copy)]
pub struct MauTrang {
    /// Chỉ số v = đậm + 0,3·màu (tỷ lệ, 0.01 = 1 %).
    pub v: f32,
    /// Phân loại tự động theo ngưỡng đang dùng.
    pub tu_dong: LoaiTrang,
    /// Nhãn cuối sau khi duyệt.
    pub nhan_cuoi: Nhan,
}

/// 1 trang được đề xuất xoay: góc đề xuất, góc cuối, đã xác nhận chưa.
#[derive(Debug, Clone, Copy)]
pub struct MauXoay {
    pub de_xuat: u16,
    pub goc: u16,
    pub xac_nhan: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThongKeNguong {
    /// (trắng, nội dung) theo ô.
    pub o: Vec<(u32, u32)>,
    pub trang_tu_dong: u32,
    /// Trắng tự động mà người dùng giữ nội dung.
    pub trang_tu_dong_sua: u32,
    pub nghi: u32,
    pub nghi_thanh_trang: u32,
    pub nghi_giu: u32,
    /// Máy cho là có chữ nhưng người dùng gán Trắng.
    pub trang_lot: u32,
    /// Ngưỡng đề xuất (%) — None khi chưa đủ trang trắng.
    pub de_xuat: Option<(f32, f32)>,
    pub xoay_xac_nhan: u32,
    pub xoay_chinh: u32,
    pub xoay_0: u32,
}

fn o_cua(v: f32) -> usize {
    ((v.max(0.0) * 1000.0) as usize).min(SO_O - 1)
}

pub fn thong_ke(trang: &[MauTrang], xoay: &[MauXoay]) -> ThongKeNguong {
    let mut t = ThongKeNguong { o: vec![(0, 0); SO_O], ..Default::default() };
    for m in trang {
        let la_trang = m.nhan_cuoi == Nhan::Trang;
        let o = &mut t.o[o_cua(m.v)];
        if la_trang { o.0 += 1 } else { o.1 += 1 }
        match m.tu_dong {
            LoaiTrang::Trang => {
                t.trang_tu_dong += 1;
                t.trang_tu_dong_sua += !la_trang as u32;
            }
            LoaiTrang::NghiTrang => {
                t.nghi += 1;
                if la_trang { t.nghi_thanh_trang += 1 } else { t.nghi_giu += 1 }
            }
            LoaiTrang::NoiDung => t.trang_lot += la_trang as u32,
        }
    }
    for x in xoay.iter().filter(|x| x.xac_nhan) {
        if x.goc == 0 {
            t.xoay_0 += 1;
        } else if x.goc == x.de_xuat {
            t.xoay_xac_nhan += 1;
        } else {
            t.xoay_chinh += 1;
        }
    }
    t.de_xuat = de_xuat(&t.o);
    t
}

/// t1 = mép trên lớn nhất mà dưới đó tỷ lệ nội dung ≤ 0,2 %; t2 = mép nhỏ nhất mà trên đó trắng lọt ≤ 1 %.
pub fn de_xuat(o: &[(u32, u32)]) -> Option<(f32, f32)> {
    let tong_trang: u32 = o.iter().map(|x| x.0).sum();
    if tong_trang < TRANG_TOI_THIEU {
        return None;
    }
    let (mut ct, mut cn, mut t1) = (0u32, 0u32, 0usize);
    for (i, &(tr, nd)) in o.iter().enumerate() {
        ct += tr;
        cn += nd;
        if ct + cn > 0 && (cn as f64) / ((ct + cn) as f64) <= NOI_DUNG_TOI_DA {
            t1 = i + 1;
        }
    }
    let mut con_tren = tong_trang;
    let mut t2 = o.len();
    for (i, &(tr, _)) in o.iter().enumerate() {
        if (con_tren as f64) / (tong_trang as f64) <= LOT_TOI_DA {
            t2 = i;
            break;
        }
        con_tren -= tr;
    }
    let pt = |i: usize| (i as f32 * 0.1).min(10.0);
    Some((pt(t1), pt(t2.max(t1))))
}

/// CSV histogram: dòng đầu `# khoá=giá trị;…`, sau đó `v_tu;v_den;trang;noi_dung` (%).
pub fn csv(tk: &ThongKeNguong, dau: &[(&str, String)]) -> String {
    let mut s = format!("# {}\n", dau.iter().map(|(k, v)| format!("{k}={}", v.replace([';', '\n'], " "))).collect::<Vec<_>>().join(";"));
    s += "v_tu;v_den;trang;noi_dung\n";
    for (i, &(tr, nd)) in tk.o.iter().enumerate() {
        let den = if i == SO_O - 1 { "100".to_string() } else { format!("{:.1}", (i + 1) as f32 * 0.1) };
        s += &format!("{:.1};{den};{tr};{nd}\n", i as f32 * 0.1);
    }
    s
}

fn chuoi_json(s: &str) -> String {
    let mut kq = String::from('"');
    for c in s.chars() {
        match c {
            '"' => kq += "\\\"",
            '\\' => kq += "\\\\",
            c if (c as u32) < 0x20 => kq += &format!("\\u{:04x}", c as u32),
            c => kq.push(c),
        }
    }
    kq + "\""
}

/// 1 dòng JSON gửi Google Sheet (Apps Script `doPost`): các trường đầu + số liệu + histogram `trắng|nội dung` theo ô.
pub fn json(tk: &ThongKeNguong, dau: &[(&str, String)]) -> String {
    let mut truong: Vec<String> = dau.iter().map(|(k, v)| format!("{}:{}", chuoi_json(k), chuoi_json(v))).collect();
    let so = [
        ("trang_tu_dong", tk.trang_tu_dong),
        ("trang_tu_dong_sua", tk.trang_tu_dong_sua),
        ("nghi", tk.nghi),
        ("nghi_thanh_trang", tk.nghi_thanh_trang),
        ("nghi_giu", tk.nghi_giu),
        ("trang_lot", tk.trang_lot),
        ("xoay_xac_nhan", tk.xoay_xac_nhan),
        ("xoay_chinh", tk.xoay_chinh),
        ("xoay_0", tk.xoay_0),
    ];
    truong.extend(so.iter().map(|(k, v)| format!("\"{k}\":{v}")));
    let (t1, t2) = tk.de_xuat.map_or((String::new(), String::new()), |(a, b)| (format!("{a:.1}"), format!("{b:.1}")));
    truong.push(format!("\"de_xuat_t1\":{}", chuoi_json(&t1)));
    truong.push(format!("\"de_xuat_t2\":{}", chuoi_json(&t2)));
    let hist = tk.o.iter().map(|(a, b)| format!("{a}|{b}")).collect::<Vec<_>>().join(",");
    truong.push(format!("\"histogram\":{}", chuoi_json(&hist)));
    format!("{{{}}}", truong.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(v: f32, tu_dong: LoaiTrang, nhan_cuoi: Nhan) -> MauTrang {
        MauTrang { v, tu_dong, nhan_cuoi }
    }

    #[test]
    fn dem_sua_cua_nguoi_dung_va_xoay() {
        let ds = [
            m(0.001, LoaiTrang::Trang, Nhan::Trang),
            m(0.005, LoaiTrang::Trang, Nhan::NoiDung),
            m(0.015, LoaiTrang::NghiTrang, Nhan::Trang),
            m(0.020, LoaiTrang::NghiTrang, Nhan::NoiDung),
            m(0.025, LoaiTrang::NghiTrang, Nhan::Index),
            m(0.040, LoaiTrang::NoiDung, Nhan::Trang),
            m(0.200, LoaiTrang::NoiDung, Nhan::Split),
        ];
        let xoay = [
            MauXoay { de_xuat: 180, goc: 180, xac_nhan: true },
            MauXoay { de_xuat: 90, goc: 270, xac_nhan: true },
            MauXoay { de_xuat: 90, goc: 0, xac_nhan: true },
            MauXoay { de_xuat: 90, goc: 90, xac_nhan: false },
        ];
        let t = thong_ke(&ds, &xoay);
        assert_eq!((t.trang_tu_dong, t.trang_tu_dong_sua), (2, 1));
        assert_eq!((t.nghi, t.nghi_thanh_trang, t.nghi_giu), (3, 1, 2));
        assert_eq!(t.trang_lot, 1);
        assert_eq!((t.xoay_xac_nhan, t.xoay_chinh, t.xoay_0), (1, 1, 1));
        assert_eq!(t.o[1], (1, 0));
        assert_eq!(t.o[5], (0, 1));
        assert_eq!(t.o[40], (1, 0));
        assert_eq!(t.o[100], (0, 1), "≥ 10 % gom ô cuối");
        assert_eq!(t.de_xuat, None, "chưa đủ 50 trang trắng");
    }

    #[test]
    fn de_xuat_nguong_lo() {
        // 100 trang trắng ở 0–0,8 %, 1000 trang chữ từ 2 %; thêm 1 trang trắng lọt ở 5 %
        let mut o = vec![(0u32, 0u32); SO_O];
        for i in 0..8 {
            o[i].0 += 12;
        }
        o[3].0 += 4;
        o[50].0 += 1;
        for i in 20..100 {
            o[i].1 += 12;
        }
        o[100].1 += 40;
        // Tách bạch: trắng hết ở 0,8 %, chữ bắt đầu từ 2 % → không cần vùng nghi: (2, 2)
        assert_eq!(de_xuat(&o), Some((2.0, 2.0)));
        // Chồng lấn: trắng 0–3 %, chữ từ 0,5 % → Trắng dưới 0,5 %, Nghi tới 3 %
        let mut o = vec![(0u32, 0u32); SO_O];
        for (i, x) in o.iter_mut().enumerate() {
            if i < 30 {
                x.0 = 10;
            }
            if i >= 5 {
                x.1 = 10;
            }
        }
        assert_eq!(de_xuat(&o), Some((0.5, 3.0)));
    }

    #[test]
    fn csv_va_json() {
        let t = thong_ke(&[m(0.012, LoaiTrang::NghiTrang, Nhan::Trang)], &[]);
        let c = csv(&t, &[("may", "PC;1".into()), ("t1", "1.00".into())]);
        let dong: Vec<&str> = c.lines().collect();
        assert_eq!(dong[0], "# may=PC 1;t1=1.00");
        assert_eq!(dong[1], "v_tu;v_den;trang;noi_dung");
        assert_eq!(dong[2 + 12], "1.2;1.3;1;0");
        assert_eq!(dong.last().unwrap(), &"10.0;100;0;0");
        assert_eq!(dong.len(), 2 + SO_O);
        let j = json(&t, &[("thu_muc", "C:\\a \"b\"".into())]);
        assert!(j.starts_with("{\"thu_muc\":\"C:\\\\a \\\"b\\\"\",\"trang_tu_dong\":0,"), "{j}");
        assert!(j.contains("\"nghi_thanh_trang\":1") && j.contains("\"de_xuat_t1\":\"\"") && j.ends_with("0|0\"}"));
    }
}
