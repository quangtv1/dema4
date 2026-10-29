//! Bộ chọn kiểu Windows Explorer dùng chung cho lưới tab Hồ sơ và trang trong văn bản (tab Văn bản).
//! Thuần dữ liệu: khoá là số hiệu trang (`u32`), thứ tự là danh sách đang hiện; giao diện chỉ đổi sự kiện
//! chuột/phím thành lời gọi ở đây.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhimPhu {
    /// Ctrl (Windows) hoặc Cmd (macOS).
    pub ctrl: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhimDiChuyen {
    Trai,
    Phai,
    Len,
    Xuong,
    DauDs,
    CuoiDs,
}

/// Việc còn chờ lúc thả chuột (Explorer quyết định khi thả để còn kéo khung từ ô đang chọn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoTha {
    KhongCo,
    /// Bấm lại ô đang chọn duy nhất → bỏ chọn.
    BoChon,
    /// Bấm 1 ô trong nhóm đang chọn → thu về riêng ô đó.
    ChiMot(u32),
    /// Bấm vùng trống không Ctrl → bỏ chọn.
    BoHet,
}

#[derive(Debug, Clone)]
pub struct BoChon {
    chon: HashSet<u32>,
    pub neo: Option<u32>,
    pub tieu_diem: Option<u32>,
    cho_tha: ChoTha,
    /// Vùng chọn lúc bắt đầu kéo khung (Ctrl: cộng dồn; không Ctrl: rỗng).
    goc_keo: HashSet<u32>,
    dang_keo: bool,
}

impl Default for BoChon {
    fn default() -> Self {
        Self { chon: HashSet::new(), neo: None, tieu_diem: None, cho_tha: ChoTha::KhongCo, goc_keo: HashSet::new(), dang_keo: false }
    }
}

impl BoChon {
    pub fn co(&self, k: u32) -> bool {
        self.chon.contains(&k)
    }

    pub fn so_luong(&self) -> usize {
        self.chon.len()
    }

    /// Các trang đang chọn theo thứ tự đang hiện.
    pub fn theo_thu_tu(&self, thu_tu: &[u32]) -> Vec<u32> {
        if self.chon.is_empty() {
            return Vec::new();
        }
        thu_tu.iter().copied().filter(|k| self.chon.contains(k)).collect()
    }

    pub fn bo_het(&mut self) {
        self.chon.clear();
        self.neo = None;
    }

    pub fn chon_rieng(&mut self, k: u32) {
        self.chon.clear();
        self.chon.insert(k);
        self.neo = Some(k);
        self.tieu_diem = Some(k);
    }

    /// Bỏ khỏi vùng chọn các trang không còn hiện (đổi lọc, xoá trang).
    pub fn giu_trong(&mut self, thu_tu: &[u32]) {
        if self.chon.is_empty() {
            return;
        }
        let hien: HashSet<u32> = thu_tu.iter().copied().collect();
        self.chon.retain(|k| hien.contains(k));
        for o in [&mut self.neo, &mut self.tieu_diem] {
            if o.is_some_and(|k| !hien.contains(&k)) {
                *o = None;
            }
        }
    }

    fn dai(&mut self, thu_tu: &[u32], tu: u32, den: u32) {
        let vi_tri = |k| thu_tu.iter().position(|&x| x == k);
        let Some(i1) = vi_tri(den) else { return };
        let i0 = vi_tri(tu).unwrap_or(i1);
        self.chon.extend(&thu_tu[i0.min(i1)..=i0.max(i1)]);
    }

    /// Nhấn chuột trái lên ô `o` (None = khe / vùng trống). Phản hồi ngay lúc nhấn như Explorer.
    pub fn nhan(&mut self, o: Option<u32>, phu: PhimPhu, thu_tu: &[u32]) {
        self.cho_tha = ChoTha::KhongCo;
        self.dang_keo = false;
        match o {
            Some(k) => {
                self.tieu_diem = Some(k);
                if let (true, Some(neo)) = (phu.shift, self.neo) {
                    if !phu.ctrl {
                        self.chon.clear();
                    }
                    self.dai(thu_tu, neo, k);
                } else if phu.ctrl {
                    if !self.chon.remove(&k) {
                        self.chon.insert(k);
                    }
                    self.neo = Some(k);
                } else if self.chon.contains(&k) {
                    self.cho_tha = if self.chon.len() == 1 { ChoTha::BoChon } else { ChoTha::ChiMot(k) };
                } else {
                    self.chon_rieng(k);
                }
            }
            None if !phu.ctrl => self.cho_tha = ChoTha::BoHet,
            None => {}
        }
        self.goc_keo = if phu.ctrl { self.chon.clone() } else { HashSet::new() };
    }

    /// Đang kéo khung: `trong_khung` = các ô giao khung theo thứ tự đang hiện (ô đầu làm neo).
    pub fn keo(&mut self, trong_khung: &[u32]) {
        self.dang_keo = true;
        self.cho_tha = ChoTha::KhongCo;
        self.chon = self.goc_keo.clone();
        self.chon.extend(trong_khung);
        if let Some(&dau) = trong_khung.first() {
            self.neo = Some(dau);
        }
    }

    /// Thả chuột trái.
    pub fn tha(&mut self) {
        if !self.dang_keo {
            match self.cho_tha {
                ChoTha::BoChon | ChoTha::BoHet => self.bo_het(),
                ChoTha::ChiMot(k) => self.chon_rieng(k),
                ChoTha::KhongCo => {}
            }
        }
        self.cho_tha = ChoTha::KhongCo;
        self.dang_keo = false;
        self.goc_keo.clear();
    }

    /// Bấm phải lên ô chưa chọn → chọn riêng ô đó (menu áp cho vùng chọn).
    pub fn bam_phai(&mut self, k: u32) {
        if !self.chon.contains(&k) {
            self.chon_rieng(k);
        }
        self.tieu_diem = Some(k);
    }

    pub fn chon_tat(&mut self, thu_tu: &[u32]) {
        self.chon.extend(thu_tu);
        if self.tieu_diem.is_none() {
            self.tieu_diem = thu_tu.first().copied();
        }
    }

    /// Ctrl+Space: bật/tắt ô tiêu điểm.
    pub fn dao_tieu_diem(&mut self) {
        if let Some(t) = self.tieu_diem {
            if !self.chon.remove(&t) {
                self.chon.insert(t);
            }
            self.neo = Some(t);
        }
    }

    /// ←/→/↑/↓/Home/End: không phím phụ = chọn riêng ô tới; Shift = dải từ neo (Ctrl+Shift cộng dải);
    /// Ctrl = chỉ dời tiêu điểm. Trả ô tiêu điểm mới (để cuộn tới).
    pub fn phim(&mut self, p: PhimDiChuyen, phu: PhimPhu, thu_tu: &[u32], so_cot: usize) -> Option<u32> {
        if thu_tu.is_empty() {
            return None;
        }
        let cuoi = thu_tu.len() - 1;
        let i = self.tieu_diem.and_then(|t| thu_tu.iter().position(|&x| x == t));
        let cot = so_cot.max(1) as isize;
        let dich = match (p, i) {
            (PhimDiChuyen::DauDs, _) => 0,
            (PhimDiChuyen::CuoiDs, _) => cuoi,
            (_, None) => 0,
            (p, Some(i)) => {
                let buoc = match p {
                    PhimDiChuyen::Trai => -1,
                    PhimDiChuyen::Phai => 1,
                    PhimDiChuyen::Len => -cot,
                    _ => cot,
                };
                (i as isize + buoc).clamp(0, cuoi as isize) as usize
            }
        };
        let kd = thu_tu[dich];
        self.tieu_diem = Some(kd);
        if phu.shift {
            let neo = match self.neo.filter(|n| thu_tu.contains(n)) {
                Some(n) => n,
                None => {
                    self.neo = Some(kd);
                    kd
                }
            };
            if !phu.ctrl {
                self.chon.clear();
            }
            self.dai(thu_tu, neo, kd);
        } else if !phu.ctrl {
            self.chon.clear();
            self.chon.insert(kd);
            self.neo = Some(kd);
        }
        Some(kd)
    }
}

/// Bố cục lưới ô cùng cỡ: ô (hàng, cột) ở `(le + cot·(rong+khe_x), hang·(cao+khe_y))` trong toạ độ nội dung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoCucLuoi {
    pub so_cot: usize,
    pub rong_o: f32,
    pub cao_o: f32,
    pub khe_x: f32,
    pub khe_y: f32,
    pub le_trai: f32,
}

impl BoCucLuoi {
    /// Số cột vừa bề ngang `rong` (ít nhất 1).
    pub fn moi(rong: f32, rong_o: f32, cao_o: f32, khe_x: f32, khe_y: f32) -> Self {
        let so_cot = (((rong + khe_x) / (rong_o + khe_x)).floor() as usize).max(1);
        Self { so_cot, rong_o, cao_o, khe_x, khe_y, le_trai: 0.0 }
    }

    /// Lưới chia đều bề ngang như CSS `repeat(auto-fill, minmax(rong_o + dem, 1fr))` + `gap: khe` của prototype:
    /// mỗi cột rộng bằng nhau, ảnh nằm giữa cột (khoảng thừa chia đều 2 bên).
    pub fn deu(rong: f32, rong_o: f32, cao_o: f32, dem: f32, khe: f32, khe_y: f32) -> Self {
        let so_cot = (((rong + khe) / (rong_o + dem + khe)).floor() as usize).max(1);
        let cot = ((rong - khe * (so_cot - 1) as f32) / so_cot as f32).max(rong_o);
        Self { so_cot, rong_o, cao_o, khe_x: cot + khe - rong_o, khe_y, le_trai: (cot - rong_o) / 2.0 }
    }

    /// Góc trái trên của ô thứ `i` (toạ độ nội dung, chưa cộng gốc).
    pub fn goc_o(&self, i: usize) -> (f32, f32) {
        (self.le_trai + (i % self.so_cot) as f32 * (self.rong_o + self.khe_x), (i / self.so_cot) as f32 * self.cao_hang())
    }

    pub fn so_hang(&self, n: usize) -> usize {
        n.div_ceil(self.so_cot)
    }

    pub fn cao_hang(&self) -> f32 {
        self.cao_o + self.khe_y
    }

    /// Vị trí (thứ tự hiện) của ô tại điểm, None nếu điểm rơi vào khe hoặc ngoài lưới.
    pub fn o_tai(&self, x: f32, y: f32, n: usize) -> Option<usize> {
        let (x, y) = (x - self.le_trai, y);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (cot, hang) = ((x / (self.rong_o + self.khe_x)) as usize, (y / self.cao_hang()) as usize);
        let trong_o = x - cot as f32 * (self.rong_o + self.khe_x) < self.rong_o && y - hang as f32 * self.cao_hang() < self.cao_o;
        let i = hang * self.so_cot + cot;
        (cot < self.so_cot && trong_o && i < n).then_some(i)
    }

    /// Vị trí các ô giao hình chữ nhật (2 góc bất kỳ), theo thứ tự hiện. Tính theo công thức, không duyệt mọi ô.
    pub fn o_giao_khung(&self, (ax, ay): (f32, f32), (bx, by): (f32, f32), n: usize) -> Vec<usize> {
        let (x0, x1) = (ax.min(bx) - self.le_trai, ax.max(bx) - self.le_trai);
        let (y0, y1) = (ay.min(by), ay.max(by));
        let buoc_x = self.rong_o + self.khe_x;
        // Ô c giao khoảng [x0, x1] khi c·buoc < x1 và c·buoc + rong > x0
        let c_dau = (((x0 - self.rong_o) / buoc_x).floor() as isize + 1).max(0) as usize;
        let c_cuoi = ((x1 / buoc_x).ceil() as isize - 1).min(self.so_cot as isize - 1);
        let h_dau = (((y0 - self.cao_o) / self.cao_hang()).floor() as isize + 1).max(0) as usize;
        let h_cuoi = (y1 / self.cao_hang()).ceil() as isize - 1;
        if c_cuoi < c_dau as isize || h_cuoi < h_dau as isize {
            return Vec::new();
        }
        let mut ds = Vec::new();
        for h in h_dau..=h_cuoi as usize {
            for c in c_dau..=c_cuoi as usize {
                let i = h * self.so_cot + c;
                if i >= n {
                    return ds;
                }
                ds.push(i);
            }
        }
        ds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KHONG: PhimPhu = PhimPhu { ctrl: false, shift: false };
    const CTRL: PhimPhu = PhimPhu { ctrl: true, shift: false };
    const SHIFT: PhimPhu = PhimPhu { ctrl: false, shift: true };
    const CTRL_SHIFT: PhimPhu = PhimPhu { ctrl: true, shift: true };

    /// Thứ tự hiện khác số hiệu trang (vd sắp theo dung lượng).
    fn thu_tu() -> Vec<u32> {
        vec![10, 3, 7, 1, 9, 4, 8, 2, 6, 5]
    }

    fn ds(b: &BoChon) -> Vec<u32> {
        b.theo_thu_tu(&thu_tu())
    }

    fn bam(b: &mut BoChon, o: Option<u32>, phu: PhimPhu) {
        b.nhan(o, phu, &thu_tu());
        b.tha();
    }

    #[test]
    fn muoi_kich_ban_chuot_nhu_prototype() {
        let t = thu_tu();
        let mut b = BoChon::default();
        // 1. bấm = chỉ trang đó (chọn ngay lúc nhấn)
        b.nhan(Some(7), KHONG, &t);
        assert_eq!(ds(&b), [7]);
        b.tha();
        // 2. bấm lại trang đang chọn duy nhất = bỏ chọn (khi thả)
        b.nhan(Some(7), KHONG, &t);
        assert_eq!(ds(&b), [7], "nhấn chưa bỏ, chờ thả");
        b.tha();
        assert!(ds(&b).is_empty());
        // 3. Ctrl+bấm thêm, 4. Ctrl+bấm bớt
        bam(&mut b, Some(3), KHONG);
        bam(&mut b, Some(9), CTRL);
        bam(&mut b, Some(8), CTRL);
        assert_eq!(ds(&b), [3, 9, 8]);
        bam(&mut b, Some(9), CTRL);
        assert_eq!(ds(&b), [3, 8]);
        // 5. bấm trang trong nhóm đang chọn → nhấn giữ nguyên nhóm, thả mới thu về 1
        b.nhan(Some(3), KHONG, &t);
        assert_eq!(ds(&b), [3, 8]);
        b.tha();
        assert_eq!(ds(&b), [3]);
        // 6. Shift+bấm = dải theo thứ tự đang hiện từ neo
        bam(&mut b, Some(4), SHIFT);
        assert_eq!(ds(&b), [3, 7, 1, 9, 4]);
        // Shift lần nữa: neo giữ nguyên, dải thay thế
        bam(&mut b, Some(7), SHIFT);
        assert_eq!(ds(&b), [3, 7]);
        // 7. Ctrl+Shift cộng dải (neo = ô Ctrl+bấm gần nhất)
        bam(&mut b, Some(2), CTRL);
        bam(&mut b, Some(5), CTRL_SHIFT);
        assert_eq!(ds(&b), [3, 7, 2, 6, 5]);
        // 8. bấm trong nhóm rồi kéo khung → vùng chọn = các ô trong khung (không Ctrl)
        b.nhan(Some(7), KHONG, &t);
        b.keo(&[1, 9]);
        b.tha();
        assert_eq!(ds(&b), [1, 9]);
        assert_eq!(b.neo, Some(1));
        // 9. Ctrl + kéo khung từ khe → cộng dồn
        b.nhan(None, CTRL, &t);
        b.keo(&[8]);
        b.keo(&[8, 2]);
        b.tha();
        assert_eq!(ds(&b), [1, 9, 8, 2]);
        // kéo khung rồi thu nhỏ khung: ô ra khỏi khung bị bỏ
        b.nhan(None, KHONG, &t);
        b.keo(&[10, 3, 7]);
        b.keo(&[10]);
        b.tha();
        assert_eq!(ds(&b), [10]);
        // 10. bấm vùng trống không kéo = bỏ chọn; Ctrl+bấm vùng trống giữ nguyên
        bam(&mut b, None, CTRL);
        assert_eq!(ds(&b), [10]);
        bam(&mut b, None, KHONG);
        assert!(ds(&b).is_empty());
    }

    #[test]
    fn shift_khi_chua_co_neo_chon_rieng_o_do() {
        let mut b = BoChon::default();
        bam(&mut b, Some(9), SHIFT);
        assert_eq!(ds(&b), [9]);
    }

    #[test]
    fn ban_phim_kieu_explorer() {
        let t = thu_tu();
        let mut b = BoChon::default();
        // Chưa có tiêu điểm → ô đầu
        assert_eq!(b.phim(PhimDiChuyen::Phai, KHONG, &t, 4), Some(10));
        assert_eq!(b.phim(PhimDiChuyen::Xuong, KHONG, &t, 4), Some(9), "↓ = +1 hàng (4 cột)");
        assert_eq!(ds(&b), [9]);
        // Shift+→ ×2: dải từ neo
        b.phim(PhimDiChuyen::Phai, SHIFT, &t, 4);
        b.phim(PhimDiChuyen::Phai, SHIFT, &t, 4);
        assert_eq!(ds(&b), [9, 4, 8]);
        // Ctrl+→ chỉ dời tiêu điểm, Ctrl+Space bật ô đó
        assert_eq!(b.phim(PhimDiChuyen::Phai, CTRL, &t, 4), Some(2));
        assert_eq!(ds(&b), [9, 4, 8]);
        b.dao_tieu_diem();
        assert_eq!(ds(&b), [9, 4, 8, 2]);
        b.dao_tieu_diem();
        assert_eq!(ds(&b), [9, 4, 8]);
        // Shift+End / Shift+Home từ neo (neo = ô Ctrl+Space gần nhất)
        b.phim(PhimDiChuyen::CuoiDs, SHIFT, &t, 4);
        assert_eq!(ds(&b), [2, 6, 5]);
        b.phim(PhimDiChuyen::DauDs, SHIFT, &t, 4);
        assert_eq!(ds(&b), [10, 3, 7, 1, 9, 4, 8, 2]);
        // Kẹp ở mép
        assert_eq!(b.phim(PhimDiChuyen::Len, KHONG, &t, 4), Some(10));
        assert_eq!(b.phim(PhimDiChuyen::Trai, KHONG, &t, 4), Some(10));
        assert_eq!(b.phim(PhimDiChuyen::CuoiDs, KHONG, &t, 4), Some(5));
        assert_eq!(b.phim(PhimDiChuyen::Xuong, KHONG, &t, 4), Some(5));
        // Ctrl+A, Esc
        b.chon_tat(&t);
        assert_eq!(b.so_luong(), 10);
        b.bo_het();
        assert_eq!(b.so_luong(), 0);
        assert_eq!(b.phim(PhimDiChuyen::Phai, KHONG, &[], 4), None);
    }

    #[test]
    fn bam_phai_va_giu_trong_danh_sach_hien() {
        let mut b = BoChon::default();
        bam(&mut b, Some(3), KHONG);
        bam(&mut b, Some(7), CTRL);
        b.bam_phai(3);
        assert_eq!(ds(&b), [3, 7], "bấm phải ô đã chọn giữ vùng chọn");
        b.bam_phai(1);
        assert_eq!(ds(&b), [1]);
        b.chon_tat(&thu_tu());
        b.giu_trong(&[3, 5]);
        assert_eq!(b.so_luong(), 2);
        assert_eq!(b.neo, None, "neo 1 không còn hiện");
    }

    #[test]
    fn bo_cuc_luoi_o_tai_va_giao_khung() {
        // 4 cột ô 100×120, khe 28×22 → bước ngang 128, bước dọc 142
        let l = BoCucLuoi::moi(4.0 * 128.0 - 28.0 + 50.0, 100.0, 120.0, 28.0, 22.0);
        assert_eq!(l.so_cot, 4);
        assert_eq!(BoCucLuoi::moi(10.0, 100.0, 120.0, 28.0, 22.0).so_cot, 1);
        assert_eq!(l.so_hang(10), 3);
        assert_eq!(l.o_tai(50.0, 50.0, 10), Some(0));
        assert_eq!(l.o_tai(110.0, 50.0, 10), None, "khe ngang");
        assert_eq!(l.o_tai(140.0, 150.0, 10), Some(5));
        assert_eq!(l.o_tai(300.0, 130.0, 10), None, "khe dọc");
        assert_eq!(l.o_tai(300.0, 300.0, 10), None, "ô thứ 11 không có");
        // Khung nằm gọn trong khe → không ô nào
        assert!(l.o_giao_khung((105.0, 10.0), (120.0, 400.0), 10).is_empty());
        // Khung chéo từ giữa ô 0 tới giữa ô 6 (hàng 1, cột 2), 2 góc đảo chiều
        assert_eq!(l.o_giao_khung((300.0, 200.0), (50.0, 50.0), 10), [0, 1, 2, 4, 5, 6]);
        // Khung vượt ra ngoài lưới bên phải/dưới → kẹp, không vượt số ô
        assert_eq!(l.o_giao_khung((-10.0, 300.0), (2000.0, 9000.0), 10), [8, 9]);
        // So với duyệt từng ô
        for (a, b) in [((0.0, 0.0), (513.0, 427.0)), ((99.9, 119.9), (128.1, 142.1)), ((-50.0, -50.0), (1.0, 1.0)), ((356.0, 284.0), (360.0, 300.0))] {
            let tung_o: Vec<usize> = (0..10)
                .filter(|&i| {
                    let (x, y) = ((i % 4) as f32 * 128.0, (i / 4) as f32 * 142.0);
                    let (x0, x1, y0, y1) = (f32::min(a.0, b.0), f32::max(a.0, b.0), f32::min(a.1, b.1), f32::max(a.1, b.1));
                    x < x1 && x + 100.0 > x0 && y < y1 && y + 120.0 > y0
                })
                .collect();
            assert_eq!(l.o_giao_khung(a, b, 10), tung_o, "{a:?} {b:?}");
        }
    }
}
