//! Vùng xoá ghim: vị trí (góc / cạnh có sẵn hoặc vẽ tay), loại, trang áp dụng, đổi sang điểm ảnh.
//!
//! Kích thước góc / độ sâu cạnh tính bằng điểm ảnh ở 120 dpi rồi co giãn theo dpi ảnh; vùng vẽ tay
//! theo tỉ lệ trang. Phép đổi (kể cả cắt số thực về số nguyên) giữ đúng như XoaGhim để cùng cấu hình
//! cho cùng khung điểm ảnh.

/// DPI gốc của các kích thước tính bằng điểm ảnh (`rong_px`, `cao_px`).
pub const DPI_GOC: f64 = 120.0;
/// Góc / cạnh nới thêm ra phía mép trang (điểm ảnh, không đổi theo dpi) để bắt cả vết sát mép.
const NOI_MEP: i64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViTri {
    GocTrenTrai,
    GocTrenPhai,
    GocDuoiTrai,
    GocDuoiPhai,
    CanhTrai,
    CanhPhai,
    CanhTren,
    CanhDuoi,
    /// Vùng vẽ tay: `x`, `y`, `rong`, `cao` là tỉ lệ trang.
    Tay,
}

impl ViTri {
    /// 4 góc rồi 4 cạnh (thứ tự xử lý như XoaGhim khi bật nhiều vùng có sẵn).
    pub const CO_SAN: [ViTri; 8] = [ViTri::GocTrenTrai, ViTri::GocTrenPhai, ViTri::GocDuoiTrai, ViTri::GocDuoiPhai, ViTri::CanhTrai, ViTri::CanhPhai, ViTri::CanhTren, ViTri::CanhDuoi];

    /// Góc / cạnh có sẵn (ngưỡng nét đen thấp hơn vùng vẽ tay, có nới ra mép).
    pub fn la_goc_canh(self) -> bool {
        self != ViTri::Tay
    }

    pub fn la_goc(self) -> bool {
        matches!(self, ViTri::GocTrenTrai | ViTri::GocTrenPhai | ViTri::GocDuoiTrai | ViTri::GocDuoiPhai)
    }

    pub fn ten(self) -> &'static str {
        match self {
            ViTri::GocTrenTrai => "Góc trên trái",
            ViTri::GocTrenPhai => "Góc trên phải",
            ViTri::GocDuoiTrai => "Góc dưới trái",
            ViTri::GocDuoiPhai => "Góc dưới phải",
            ViTri::CanhTrai => "Cạnh trái",
            ViTri::CanhPhai => "Cạnh phải",
            ViTri::CanhTren => "Cạnh trên",
            ViTri::CanhDuoi => "Cạnh dưới",
            ViTri::Tay => "Vùng vẽ",
        }
    }
}

/// Vùng có sẵn với kích thước / ngưỡng mặc định như XoaGhim: góc 130 × 130 điểm ảnh @120 dpi (góc trên trái ngưỡng 3,
/// góc khác 5); cạnh sâu 50 điểm ảnh @120 dpi suốt chiều dài (trái / phải ngưỡng 8, trên / dưới 5) — lưu dạng mức xoá.
pub fn co_san(vi_tri: ViTri) -> Vung {
    let v = Vung { vi_tri, loai: LoaiVung::Xoa, ap_trang: ApTrang::TatCa, x: 0.0, y: 0.0, rong: 0.12, cao: 0.12, rong_px: 130, cao_px: 130, muc: 52 };
    match vi_tri {
        ViTri::GocTrenTrai => Vung { muc: 56, ..v },
        ViTri::CanhTrai | ViTri::CanhPhai => Vung { rong: 0.08, cao: 1.0, rong_px: 50, cao_px: 0, muc: 46, ..v },
        ViTri::CanhTren | ViTri::CanhDuoi => Vung { rong: 1.0, cao: 0.05, rong_px: 0, cao_px: 50, ..v },
        _ => v,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoaiVung {
    /// Xoá vết, tránh mọi vùng bảo vệ.
    Xoa,
    /// Vùng không được xoá.
    BaoVe,
    /// Xoá vô đối: chạy trước, không xét vùng bảo vệ (vẫn giữ nét đen nếu bật).
    VoDoi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApTrang {
    TatCa,
    /// Trang 1, 3, 5… (đếm từ 1).
    Le,
    Chan,
    /// Đúng 1 trang (chỉ số từ 0).
    MotTrang(u32),
}

impl ApTrang {
    pub fn ap_dung(self, trang: u32) -> bool {
        match self {
            ApTrang::TatCa => true,
            ApTrang::Le => trang % 2 == 0,
            ApTrang::Chan => trang % 2 == 1,
            ApTrang::MotTrang(t) => t == trang,
        }
    }
}

/// Hình chữ nhật điểm ảnh `[x1, x2) × [y1, y2)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hcn {
    pub x1: i64,
    pub y1: i64,
    pub x2: i64,
    pub y2: i64,
}

impl Hcn {
    pub fn rong(&self) -> i64 {
        self.x2 - self.x1
    }
    pub fn cao(&self) -> i64 {
        self.y2 - self.y1
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vung {
    pub vi_tri: ViTri,
    pub loai: LoaiVung,
    pub ap_trang: ApTrang,
    /// Tỉ lệ trang (0..1). Góc / cạnh chỉ dùng `rong` / `cao` khi kích thước điểm ảnh tương ứng = 0.
    pub x: f64,
    pub y: f64,
    pub rong: f64,
    pub cao: f64,
    /// Kích thước ở 120 dpi; 0 = theo tỉ lệ. Góc: cả hai; cạnh trái / phải: `rong_px` (độ sâu); trên / dưới: `cao_px`.
    pub rong_px: u32,
    pub cao_px: u32,
    /// Mức xoá 0–100 (cao = mạnh). 0–60: độ nhạy nhận vết (0 ≈ ngưỡng 30 cũ, 60 ≈ ngưỡng 1); 60–100: phủ dần phần
    /// giấy còn lại trong vùng về màu giấy, 100 = một màu giấy đặc.
    pub muc: u8,
}

/// Mức xoá có độ nhạy cao nhất; trên mức này là phủ nền.
pub const MUC_NHAY_MAX: u8 = 60;

/// Đổi ngưỡng kiểu cũ (1–30: điểm tối hơn nền quá ngưỡng là vết) sang mức xoá — đổi qua đổi lại giữ đúng ngưỡng.
pub fn muc_tu_nguong(nguong: u8) -> u8 {
    ((30.0 - nguong.clamp(1, 30) as f64) * MUC_NHAY_MAX as f64 / 29.0).round() as u8
}

impl Vung {
    /// Ngưỡng nhận vết (độ tối hơn nền) của mức xoá.
    pub fn nguong_vet(&self) -> i32 {
        (30.0 - self.muc.min(MUC_NHAY_MAX) as f64 * 29.0 / MUC_NHAY_MAX as f64).round().clamp(1.0, 30.0) as i32
    }

    /// Phần phủ màu giấy lên giấy còn lại trong vùng (0 = không phủ, 1 = đặc).
    pub fn phu_nen(&self) -> f32 {
        (self.muc.saturating_sub(MUC_NHAY_MAX) as f32 / (100 - MUC_NHAY_MAX) as f32).min(1.0)
    }

    /// Khung điểm ảnh của vùng trên ảnh `w` × `h` vẽ ở `dpi` (chưa nới mép), đã cắt vào ảnh.
    pub fn khung(&self, w: i64, h: i64, dpi: f64) -> Hcn {
        let k = dpi / DPI_GOC;
        let wf = w as f64;
        let hf = h as f64;
        // Python: int(px * k) nếu px > 0, ngược lại int(tỉ lệ * cạnh ảnh)
        let theo = |px: u32, ti_le: f64, canh: f64| if px > 0 { (px as f64 * k) as i64 } else { (ti_le * canh) as i64 };
        let (x, y, kw, kh) = match self.vi_tri {
            ViTri::GocTrenTrai | ViTri::GocTrenPhai | ViTri::GocDuoiTrai | ViTri::GocDuoiPhai => {
                let kw = theo(self.rong_px, self.rong, wf);
                let kh = theo(self.cao_px, self.cao, hf);
                let (x, y) = match self.vi_tri {
                    ViTri::GocTrenTrai => (0, 0),
                    ViTri::GocTrenPhai => (w - kw, 0),
                    ViTri::GocDuoiTrai => (0, h - kh),
                    _ => (w - kw, h - kh),
                };
                (x, y, kw, kh)
            }
            ViTri::CanhTren | ViTri::CanhDuoi => {
                let kw = (self.rong * wf) as i64;
                let kh = theo(self.cao_px, self.cao, hf);
                (0, if self.vi_tri == ViTri::CanhTren { 0 } else { h - kh }, kw, kh)
            }
            ViTri::CanhTrai | ViTri::CanhPhai => {
                let kw = theo(self.rong_px, self.rong, wf);
                let kh = (self.cao * hf) as i64;
                (if self.vi_tri == ViTri::CanhTrai { 0 } else { w - kw }, 0, kw, kh)
            }
            // Vùng vẽ chạm mép phải / dưới (x + rộng = 1): tới đúng cạnh ảnh — cắt phần lẻ riêng x và rộng làm hở 1–2 điểm ảnh
            ViTri::Tay => {
                let (x, y) = ((self.x * wf) as i64, (self.y * hf) as i64);
                let kw = if self.x + self.rong >= 1.0 - 1e-9 { w - x } else { (self.rong * wf) as i64 };
                let kh = if self.y + self.cao >= 1.0 - 1e-9 { h - y } else { (self.cao * hf) as i64 };
                (x, y, kw, kh)
            }
        };
        let x = x.min(w - 1).max(0);
        let y = y.min(h - 1).max(0);
        Hcn { x1: x, y1: y, x2: x + kw.min(w - x), y2: y + kh.min(h - y) }
    }

    /// Khung dùng để xoá: góc / cạnh nới 10 điểm ảnh ra phía mép trang (vùng vẽ tay giữ nguyên),
    /// cắt vào ảnh. `None` khi rỗng.
    pub fn khung_xoa(&self, w: i64, h: i64, dpi: f64) -> Option<Hcn> {
        let mut k = self.khung(w, h, dpi);
        let (trai, phai, tren, duoi) = match self.vi_tri {
            ViTri::GocTrenTrai => (true, false, true, false),
            ViTri::GocTrenPhai => (false, true, true, false),
            ViTri::GocDuoiTrai => (true, false, false, true),
            ViTri::GocDuoiPhai => (false, true, false, true),
            ViTri::CanhTrai => (true, false, false, false),
            ViTri::CanhPhai => (false, true, false, false),
            ViTri::CanhTren => (false, false, true, false),
            ViTri::CanhDuoi => (false, false, false, true),
            ViTri::Tay => (false, false, false, false),
        };
        if trai {
            k.x1 = (k.x1 - NOI_MEP).max(0);
        }
        if phai {
            k.x2 = (k.x2 + NOI_MEP).min(w);
        }
        if tren {
            k.y1 = (k.y1 - NOI_MEP).max(0);
        }
        if duoi {
            k.y2 = (k.y2 + NOI_MEP).min(h);
        }
        // Cắt lại như lúc xử lý: góc trái trên trong ảnh, rộng / cao không vượt ảnh
        let x1 = k.x1.min(w - 1).max(0);
        let y1 = k.y1.min(h - 1).max(0);
        let x2 = x1 + (k.x2 - k.x1).min(w - x1);
        let y2 = y1 + (k.y2 - k.y1).min(h - y1);
        (x2 > x1 && y2 > y1).then_some(Hcn { x1, y1, x2, y2 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn goc(vi_tri: ViTri, px: u32) -> Vung {
        Vung { vi_tri, loai: LoaiVung::Xoa, ap_trang: ApTrang::TatCa, x: 0.0, y: 0.0, rong: 0.12, cao: 0.12, rong_px: px, cao_px: px, muc: 52 }
    }

    #[test]
    fn goc_co_dinh_co_gian_theo_dpi_va_noi_ra_mep() {
        // 130 px @120 → 325 @300; góc trên phải nới 10 sang phải (bị cắt ở mép) và lên trên (bị cắt ở 0)
        let v = goc(ViTri::GocTrenPhai, 130);
        assert_eq!(v.khung(2480, 3508, 300.0), Hcn { x1: 2155, y1: 0, x2: 2480, y2: 325 });
        assert_eq!(v.khung_xoa(2480, 3508, 300.0), Some(Hcn { x1: 2155, y1: 0, x2: 2480, y2: 325 }));
        // 130 × 200/120 = 216,67 → 216
        let v = goc(ViTri::GocDuoiTrai, 130);
        assert_eq!(v.khung(1654, 2339, 200.0), Hcn { x1: 0, y1: 2123, x2: 216, y2: 2339 });
        assert_eq!(goc(ViTri::GocTrenTrai, 130).khung(992, 1403, 120.0), Hcn { x1: 0, y1: 0, x2: 130, y2: 130 });
    }

    #[test]
    fn canh_sau_co_dinh_dai_theo_ti_le_va_vung_tay_theo_ti_le() {
        let canh = Vung { vi_tri: ViTri::CanhPhai, rong: 0.022, cao: 1.0, rong_px: 22, cao_px: 0, ..goc(ViTri::CanhPhai, 0) };
        // 22 × 2,5 = 55 sâu, sát mép phải
        assert_eq!(canh.khung(2487, 3516, 300.0), Hcn { x1: 2432, y1: 0, x2: 2487, y2: 3516 });
        let tren = Vung { vi_tri: ViTri::CanhTren, rong: 1.0, cao: 0.0142, rong_px: 0, cao_px: 20, ..goc(ViTri::CanhTren, 0) };
        assert_eq!(tren.khung(2487, 3516, 300.0), Hcn { x1: 0, y1: 0, x2: 2487, y2: 50 });
        let tay = Vung { vi_tri: ViTri::Tay, x: 0.5, y: 0.25, rong: 0.1, cao: 0.9, ..goc(ViTri::Tay, 0) };
        // cao 0,9 từ 0,25 vượt trang → cắt ở đáy; vùng tay không nới mép
        assert_eq!(tay.khung(1000, 2000, 300.0), Hcn { x1: 500, y1: 500, x2: 600, y2: 2000 });
        assert_eq!(tay.khung_xoa(1000, 2000, 300.0), Some(Hcn { x1: 500, y1: 500, x2: 600, y2: 2000 }));
    }

    #[test]
    fn trang_le_chan_mot_trang() {
        assert!(ApTrang::Le.ap_dung(0) && !ApTrang::Le.ap_dung(1));
        assert!(ApTrang::Chan.ap_dung(1) && !ApTrang::Chan.ap_dung(2));
        assert!(ApTrang::MotTrang(3).ap_dung(3) && !ApTrang::MotTrang(3).ap_dung(2));
        assert!(ApTrang::TatCa.ap_dung(7));
    }
}

#[cfg(test)]
mod tests_sat_mep {
    use super::*;

    /// Vùng vẽ kéo tới mép phải / dưới: khung tới đúng cạnh ảnh, không hở vì cắt phần lẻ.
    #[test]
    fn vung_ve_cham_mep_toi_dung_canh_anh() {
        let z = Vung { vi_tri: ViTri::Tay, x: 0.3, y: 0.37, rong: 0.7, cao: 0.63, ..co_san(ViTri::GocTrenTrai) };
        let k = z.khung(2479, 3508, 300.0);
        assert_eq!((k.x2, k.y2), (2479, 3508));
        let giua = Vung { rong: 0.4, cao: 0.2, ..z.clone() };
        let k = giua.khung(2479, 3508, 300.0);
        assert!(k.x2 < 2479 && k.y2 < 3508, "chưa chạm mép thì giữ cách tính cũ");
    }
}
