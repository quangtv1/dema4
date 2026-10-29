//! Hàng đợi phân tích nền: nhiều luồng, mỗi luồng giữ phiên mô hình hướng trang + file PDF đang mở.
//! Trang đang hiện trên lưới được đưa lên trước; trang đã xoá khỏi lưới bị bỏ; huỷ bằng cờ dừng (< 1 s).

use super::anh::Anh;
use super::huong_trang::MoHinhHuong;
use super::phan_tich::{gan_nhan, phan_tich_trang, CauHinhPhanTich, KetQuaTrang, NhanTuDong};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// 1 trang cần phân tích: chỉ số file trong danh sách + số trang (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViecTrang {
    pub file: usize,
    pub trang: usize,
}

pub enum SuKienPhanTich {
    Trang { viec: ViecTrang, kq: KetQuaTrang, nhan: NhanTuDong, anh_nho: Option<Anh> },
    Loi { viec: ViecTrang, loi: String },
    /// Mọi luồng đã dừng (hết việc hoặc bị huỷ).
    Xong,
}

#[derive(Default)]
struct TrangThai {
    chinh: VecDeque<ViecTrang>,
    uu_tien: VecDeque<ViecTrang>,
    da_lay: HashSet<ViecTrang>,
    bo: HashSet<ViecTrang>,
}

pub struct HangDoi {
    tt: Mutex<TrangThai>,
    dung: AtomicBool,
}

impl HangDoi {
    pub fn moi(viec: impl IntoIterator<Item = ViecTrang>) -> Arc<Self> {
        Arc::new(Self { tt: Mutex::new(TrangThai { chinh: viec.into_iter().collect(), ..Default::default() }), dung: AtomicBool::new(false) })
    }

    /// Đưa các trang (đang hiện trên lưới) lên đầu hàng đợi; trang đã làm thì bỏ qua.
    pub fn uu_tien(&self, ds: &[ViecTrang]) {
        let mut tt = self.tt.lock().unwrap();
        tt.uu_tien.clear();
        tt.uu_tien.extend(ds.iter().copied());
    }

    /// Bỏ trang khỏi hàng đợi (trang đã xoá khỏi lưới).
    pub fn bo(&self, v: ViecTrang) {
        self.tt.lock().unwrap().bo.insert(v);
    }

    pub fn dung(&self) {
        self.dung.store(true, Ordering::Relaxed);
    }

    pub fn da_dung(&self) -> bool {
        self.dung.load(Ordering::Relaxed)
    }

    fn lay(&self) -> Option<ViecTrang> {
        let mut tt = self.tt.lock().unwrap();
        loop {
            let v = match tt.uu_tien.pop_front() {
                Some(v) => v,
                None => tt.chinh.pop_front()?,
            };
            if !tt.bo.contains(&v) && tt.da_lay.insert(v) {
                return Some(v);
            }
        }
    }
}

/// Chạy `so_luong` luồng phân tích; trả các handle để chờ (GUI không cần chờ — đọc sự kiện từ `gui`).
pub fn chay(hang: Arc<HangDoi>, pdf: Arc<Vec<PathBuf>>, ch: CauHinhPhanTich, so_luong: usize, gui: Sender<SuKienPhanTich>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let luong: Vec<_> = (0..so_luong.max(1))
            .map(|_| {
                let (hang, pdf, ch, gui) = (hang.clone(), pdf.clone(), ch.clone(), gui.clone());
                std::thread::spawn(move || lam_viec(&hang, &pdf, &ch, &gui))
            })
            .collect();
        for l in luong {
            let _ = l.join();
        }
        let _ = gui.send(SuKienPhanTich::Xong);
    })
}

fn lam_viec(hang: &HangDoi, pdf: &[PathBuf], ch: &CauHinhPhanTich, gui: &Sender<SuKienPhanTich>) {
    let mut mo_hinh = if ch.quet_xoay { MoHinhHuong::nap().ok() } else { None };
    // Giữ vài file đang mở: hàng đợi đi theo thứ tự file nên thường chỉ cần 1–2
    let mut dang_mo: HashMap<usize, mupdf::Document> = HashMap::new();
    while !hang.da_dung() {
        let Some(viec) = hang.lay() else { break };
        if !dang_mo.contains_key(&viec.file) {
            if dang_mo.len() >= 4 {
                dang_mo.clear();
            }
            match mupdf::Document::open(pdf[viec.file].to_string_lossy().as_ref()) {
                Ok(d) => {
                    dang_mo.insert(viec.file, d);
                }
                Err(e) => {
                    let _ = gui.send(SuKienPhanTich::Loi { viec, loi: format!("không mở được {}: {e}", pdf[viec.file].display()) });
                    continue;
                }
            }
        }
        let doc = &dang_mo[&viec.file];
        let kq = doc.load_page(viec.trang as i32 - 1).map_err(|e| e.to_string()).and_then(|t| phan_tich_trang(&t, ch, mo_hinh.as_mut()));
        let su_kien = match kq {
            Ok((kq, anh_nho)) => SuKienPhanTich::Trang { viec, nhan: gan_nhan(&kq, ch), kq, anh_nho },
            Err(loi) => SuKienPhanTich::Loi { viec, loi },
        };
        if gui.send(su_kien).is_err() {
            break; // phía nhận đã đóng
        }
    }
}

/// Danh sách việc cho cả thư mục: mọi trang của mọi file (theo thứ tự file, trang).
pub fn viec_ca_thu_muc(so_trang: &[usize]) -> Vec<ViecTrang> {
    so_trang.iter().enumerate().flat_map(|(file, &n)| (1..=n).map(move |trang| ViecTrang { file, trang })).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tach_vb::nhan::Nhan;
    use std::sync::mpsc;

    fn v(file: usize, trang: usize) -> ViecTrang {
        ViecTrang { file, trang }
    }

    #[test]
    fn uu_tien_truoc_bo_qua_trang_da_xoa_khong_lam_lai() {
        let h = HangDoi::moi((1..=10).map(|t| v(0, t)));
        assert_eq!(h.lay(), Some(v(0, 1)));
        h.uu_tien(&[v(0, 7), v(0, 1), v(0, 8)]);
        h.bo(v(0, 8));
        h.bo(v(0, 3));
        let mut con = Vec::new();
        while let Some(x) = h.lay() {
            con.push(x.trang);
        }
        assert_eq!(con, vec![7, 2, 4, 5, 6, 9, 10]);
    }

    fn chay_va_gom(pdf: Vec<PathBuf>, so_trang: &[usize], ch: CauHinhPhanTich, truoc: impl FnOnce(&HangDoi)) -> (Vec<(ViecTrang, NhanTuDong, KetQuaTrang)>, usize) {
        let hang = HangDoi::moi(viec_ca_thu_muc(so_trang));
        truoc(&hang);
        let (gui, nhan) = mpsc::channel();
        let h = chay(hang, Arc::new(pdf), ch, 3, gui);
        let (mut ds, mut loi) = (Vec::new(), 0);
        for sk in nhan {
            match sk {
                SuKienPhanTich::Trang { viec, nhan, kq, .. } => ds.push((viec, nhan, kq)),
                SuKienPhanTich::Loi { .. } => loi += 1,
                SuKienPhanTich::Xong => break,
            }
        }
        h.join().unwrap();
        ds.sort_by_key(|x| x.0);
        (ds, loi)
    }

    #[test]
    fn phan_tich_moi_trang_dung_1_lan_va_file_loi_khong_chan() {
        let thu_muc = std::env::temp_dir().join(format!("dema4_hang_doi_{}", std::process::id()));
        std::fs::create_dir_all(&thu_muc).unwrap();
        let a = thu_muc.join("a.pdf");
        crate::tach_vb::tach::tests::tao_pdf(&a, 5);
        let hong = thu_muc.join("hong.pdf");
        std::fs::write(&hong, b"khong phai pdf").unwrap();
        let (ds, loi) = chay_va_gom(vec![a, hong], &[5, 2], CauHinhPhanTich::default(), |h| h.bo(v(0, 4)));
        assert_eq!(ds.iter().map(|x| x.0.trang).collect::<Vec<_>>(), vec![1, 2, 3, 5]);
        assert_eq!(loi, 2, "file hỏng báo lỗi từng trang, không làm dừng file khác");
        // Trang chỉ có 1 dòng chữ nhỏ "Trang n": đủ trắng → nhãn Trắng, đã quét mã, có chỉ số, không chạy mô hình hướng
        assert!(ds.iter().all(|x| x.1.nhan == Nhan::Trang && x.2.da_quet_ma && x.2.chi_so.is_some() && x.2.huong.is_none()), "{:?}", ds.iter().map(|x| (x.1, x.2.chi_so)).collect::<Vec<_>>());
        // Tắt cả 3 loại quét → không render, không nhãn tự động
        let (ds, _) = chay_va_gom(vec![thu_muc.join("a.pdf")], &[5], CauHinhPhanTich { quet_trang: false, quet_ma: false, quet_xoay: false, ..Default::default() }, |_| {});
        assert!(ds.iter().all(|x| x.2 == KetQuaTrang::default()));
        let _ = std::fs::remove_dir_all(&thu_muc);
    }

    #[test]
    fn huy_dung_nhanh_khong_panic() {
        let hang = HangDoi::moi((0..100_000).map(|t| v(0, t + 1)));
        hang.dung();
        let (gui, nhan) = mpsc::channel();
        let t0 = std::time::Instant::now();
        chay(hang, Arc::new(vec![PathBuf::from("/khong/co.pdf")]), CauHinhPhanTich::default(), 4, gui).join().unwrap();
        assert!(matches!(nhan.recv().unwrap(), SuKienPhanTich::Xong));
        assert!(t0.elapsed().as_secs_f64() < 1.0);
    }

    /// Mẫu 1 qua hàng đợi thật: index 209/209, blank 1/1, không thừa.
    #[test]
    #[ignore = "cần DEMA4_MAU_TACH (thư mục mẫu 1)"]
    fn phan_tich_mau_1() {
        let mau = crate::tach_vb::thu_muc_mau().expect("đặt DEMA4_MAU_TACH");
        let ten = ["L1_05_13.pdf", "L1_06_14.pdf"];
        let pdf: Vec<PathBuf> = ten.iter().map(|t| mau.join("pdf_goc").join(t)).collect();
        let nhan_tay = std::fs::read_to_string(mau.join("index_and_blank.txt")).unwrap();
        let (bang, _) = crate::tach_vb::nhan::doc_file_nhan(&nhan_tay, &|_| None).unwrap();
        let t0 = std::time::Instant::now();
        let so_luong = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let hang = HangDoi::moi(viec_ca_thu_muc(&[491, 405]));
        let (gui, nhan) = mpsc::channel();
        chay(hang, Arc::new(pdf), CauHinhPhanTich::default(), so_luong, gui);
        let mut sai = Vec::new();
        let (mut n_index, mut n_trang, mut n_xoay) = (0, 0, 0);
        for sk in nhan {
            let SuKienPhanTich::Trang { viec, nhan, kq, .. } = sk else { if matches!(sk, SuKienPhanTich::Xong) { break } else { panic!("lỗi trang") } };
            let chuan = bang.get(&format!("pdf_goc/{}", ten[viec.file])).and_then(|m| m.get(&viec.trang)).copied().unwrap_or(Nhan::NoiDung);
            if chuan == Nhan::Trang || nhan.nghi_xoay.is_some() || nhan.nghi_trang {
                println!("  {} tr.{}: máy {:?}, tay {:?}, chỉ số {:?}, hướng {:?}", ten[viec.file], viec.trang, nhan, chuan, kq.chi_so, kq.huong);
            }
            n_index += (nhan.nhan == Nhan::Index) as usize;
            n_trang += (nhan.nhan == Nhan::Trang) as usize;
            n_xoay += nhan.nghi_xoay.is_some() as usize;
            // Trang trắng theo nhãn tay mà có dấu màu → Nghi trắng (người dùng tự phân loại) cũng là đúng
            let sai_trang = (nhan.nhan == Nhan::Trang && chuan != Nhan::Trang) || (chuan == Nhan::Trang && nhan.nhan != Nhan::Trang && !nhan.nghi_trang);
            if (nhan.nhan == Nhan::Index) != (chuan == Nhan::Index) || sai_trang {
                sai.push((ten[viec.file], viec.trang, nhan, chuan));
            }
        }
        println!("896 trang, {} luồng: {:.1} s; index {n_index}, trắng {n_trang}, nghi xoay {n_xoay}", so_luong, t0.elapsed().as_secs_f64());
        assert!(sai.is_empty(), "{sai:?}");
        assert_eq!(n_index, 209);
    }

    /// Mẫu 2 qua hàng đợi thật, so với nhãn tay: index, bảng nhầm lẫn trắng / nghi trắng / nội dung, cờ nghi xoay, thời gian.
    #[test]
    #[ignore = "cần DEMA4_MAU2 + DEMA4_MAU2_NHAN, chạy ~10 phút"]
    fn phan_tich_mau_2() {
        let thu_muc = PathBuf::from(std::env::var_os("DEMA4_MAU2").expect("đặt DEMA4_MAU2"));
        let nhan_tay = std::fs::read_to_string(std::env::var_os("DEMA4_MAU2_NHAN").expect("đặt DEMA4_MAU2_NHAN")).unwrap();
        let ten = |p: &std::path::Path| p.file_stem().unwrap().to_string_lossy().to_string();
        let mut chuan: HashMap<(String, usize), Nhan> = HashMap::new();
        for d in nhan_tay.lines() {
            let c: Vec<&str> = d.trim().split(';').collect();
            if c.len() == 3 {
                chuan.insert((ten(std::path::Path::new(c[0])), c[1].parse().unwrap()), if c[2] == "index" { Nhan::Index } else { Nhan::Trang });
            }
        }
        let (gui_tim, _n) = mpsc::channel();
        let pdf = crate::dem_pdf::tim_file_pdf(&thu_muc.to_string_lossy(), &AtomicBool::new(false), &gui_tim);
        let so_trang: Vec<usize> = pdf.iter().map(|p| mupdf::Document::open(p.to_string_lossy().as_ref()).unwrap().page_count().unwrap() as usize).collect();
        let so_luong = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let t0 = std::time::Instant::now();
        let (gui, nhan) = mpsc::channel();
        chay(HangDoi::moi(viec_ca_thu_muc(&so_trang)), Arc::new(pdf.clone()), CauHinhPhanTich::default(), so_luong, gui);
        // bảng[nhãn tay: 0 nội dung, 1 trắng, 2 index][máy: 0 nội dung, 1 nghi trắng, 2 trắng, 3 index]
        let mut bang = [[0usize; 4]; 3];
        let (mut nghi_xoay, mut tang_2, mut tong) = (0, 0, 0);
        let mut sot_index = Vec::new();
        // DEMA4_GHI_TSV = đường dẫn → ghi số liệu từng trang để thử ngưỡng không cần render lại
        let mut tsv = std::env::var_os("DEMA4_GHI_TSV").map(|p| std::fs::File::create(p).unwrap());
        for sk in nhan {
            let SuKienPhanTich::Trang { viec, nhan, kq, .. } = sk else { if matches!(sk, SuKienPhanTich::Xong) { break } else { continue } };
            tong += 1;
            let tay = match chuan.get(&(ten(&pdf[viec.file]), viec.trang)) { Some(Nhan::Index) => 2, Some(_) => 1, None => 0 };
            if let Some(f) = tsv.as_mut() {
                use std::io::Write;
                let cs = kq.chi_so.unwrap_or_default();
                let h = kq.huong.map(|h| format!("{:?}|{:?}", h.tang_1, h.tang_2)).unwrap_or_default();
                writeln!(f, "{}\t{}\t{tay}\t{}\t{}\t{}\t{h}", ten(&pdf[viec.file]), viec.trang, cs.dam, cs.mau, (nhan.nhan == Nhan::Index) as u8).unwrap();
            }
            let may = match (nhan.nhan, nhan.nghi_trang) { (Nhan::Index, _) => 3, (Nhan::Trang, _) => 2, (_, true) => 1, _ => 0 };
            bang[tay][may] += 1;
            if tay == 2 && may != 3 { sot_index.push((ten(&pdf[viec.file]), viec.trang)); }
            nghi_xoay += nhan.nghi_xoay.is_some() as usize;
            tang_2 += kq.huong.is_some_and(|h| h.tang_2.is_some()) as usize;
        }
        let tg = t0.elapsed().as_secs_f64();
        println!("{tong} trang, {so_luong} luồng: {tg:.0} s ({:.1} ms/trang/lõi)", tg * 1e3 * so_luong as f64 / tong as f64);
        println!("tay \\ máy      nội dung  nghi trắng  trắng  index");
        for (i, t) in ["nội dung", "trắng", "index"].iter().enumerate() {
            println!("{t:10} {:>9} {:>11} {:>6} {:>6}", bang[i][0], bang[i][1], bang[i][2], bang[i][3]);
        }
        let may_trang: usize = (0..3).map(|i| bang[i][2]).sum();
        let tay_trang: usize = bang[1].iter().sum();
        println!("Trắng máy đúng {:.2} %, trắng lọt nội dung {:.2} %, nghi trắng {}, nghi xoay {nghi_xoay} (tầng 2: {tang_2}), sót index {sot_index:?}",
            bang[1][2] as f64 * 100.0 / may_trang as f64, bang[1][0] as f64 * 100.0 / tay_trang as f64, (0..3).map(|i| bang[i][1]).sum::<usize>());
        assert_eq!(bang[0][3] + bang[1][3], 0, "gán index thừa");
        assert!(sot_index.len() <= 2, "sót index {sot_index:?}");
        assert!(bang[1][2] as f64 / may_trang as f64 >= 0.995);
        assert!(bang[1][0] as f64 / tay_trang as f64 <= 0.01);
    }
}
