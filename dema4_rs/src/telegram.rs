//! Gửi thống kê sử dụng về nhóm Telegram qua Bot API.
//! Cấu hình đặt trong file `telegram.txt` cạnh file chạy; không có file thì không gửi gì.
//!
//! ```text
//! token=123456789:AAH...
//! chat_id=-1001234567890
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

pub const TEN_FILE_CAU_HINH: &str = "telegram.txt";
const API_TELEGRAM: &str = "https://api.telegram.org";
/// Telegram giới hạn một tin nhắn 4096 ký tự
const DO_DAI_TOI_DA: usize = 4000;

#[derive(Debug, Clone, PartialEq)]
pub struct CauHinh {
    pub token: String,
    pub chat_id: String,
}

/// Đường dẫn file cấu hình: cạnh file chạy
pub fn duong_dan_cau_hinh() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(TEN_FILE_CAU_HINH))
}

/// Đọc cấu hình cạnh file chạy; None nếu không có file hoặc thiếu token / chat_id
// Token + chat_id nhúng thẳng vào file chạy: exe build ra gửi Telegram ngay, không cần file txt.
// Lưu ý: ai có file exe cũng có thể moi token ra. Đổi token: @BotFather -> /revoke rồi sửa ở đây.
const TOKEN_NHUNG: &str = "7776430644:AAHw7eeDVRXSOsmOiOUl2ktiRSIoqTK4ykc";
const CHAT_ID_NHUNG: &str = "-878766449";

fn cau_hinh_nhung() -> Option<CauHinh> {
    if TOKEN_NHUNG.is_empty() || CHAT_ID_NHUNG.is_empty() {
        return None;
    }
    Some(CauHinh { token: TOKEN_NHUNG.to_string(), chat_id: CHAT_ID_NHUNG.to_string() })
}

/// Ưu tiên telegram.txt cạnh file chạy (nếu có, để đổi nhóm mà không build lại); nếu không thì dùng cấu hình nhúng
pub fn doc_cau_hinh() -> Option<CauHinh> {
    duong_dan_cau_hinh().and_then(|p| doc_cau_hinh_tu(&p)).or_else(cau_hinh_nhung)
}

pub fn doc_cau_hinh_tu(duong_dan: &Path) -> Option<CauHinh> {
    phan_tich_cau_hinh(&std::fs::read_to_string(duong_dan).ok()?)
}

/// Mỗi dòng dạng khoa=gia_tri; dòng trống hoặc bắt đầu bằng # được bỏ qua
fn phan_tich_cau_hinh(noi_dung: &str) -> Option<CauHinh> {
    let (mut token, mut chat_id) = (None, None);
    for dong in noi_dung.lines().map(str::trim).filter(|d| !d.is_empty() && !d.starts_with('#')) {
        if let Some((khoa, gia_tri)) = dong.split_once('=') {
            let gia_tri = gia_tri.trim().to_string();
            match khoa.trim() {
                "token" => token = Some(gia_tri),
                "chat_id" => chat_id = Some(gia_tri),
                _ => {}
            }
        }
    }
    match (token, chat_id) {
        (Some(token), Some(chat_id)) if !token.is_empty() && !chat_id.is_empty() => Some(CauHinh { token, chat_id }),
        _ => None,
    }
}

/// Tên máy tính (trên Windows là Computer name)
pub fn ten_pc() -> String {
    whoami::devicename().unwrap_or_else(|_| "?".into())
}

/// Địa chỉ MAC của card mạng chính, dạng AA:BB:CC:DD:EE:FF
pub fn dia_chi_mac() -> String {
    match mac_address::get_mac_address() {
        Ok(Some(mac)) => mac.to_string().to_uppercase(),
        _ => "?".into(),
    }
}

/// Nội dung tin thống kê sau mỗi lần đếm xong một thư mục, theo mẫu:
/// Tên PC: ...; Địa chỉ Mac: ...
/// - Thư mục đếm: ...;
/// - File PDF: ..; Số trang: ..; Quy ra A4: ..; Cảnh báo: ..; File lỗi: ..
/// - Thời điểm đếm: dd.mm.yyyy, hh:mm:ss  (x phút, y giây)
pub fn noi_dung_thong_ke(thu_muc: &str, kq: &crate::dem_pdf::KetQuaDem, thoi_gian: Duration) -> String {
    let thoi_diem = chrono::Local::now().format("%d.%m.%Y, %H:%M:%S").to_string();
    soan_tin(&ten_pc(), &dia_chi_mac(), thu_muc, kq, &thoi_diem, thoi_gian)
}

/// Đường dẫn tuyệt đối, cắt còn 1000 ký tự cuối: giữ tin dưới giới hạn 4096 ký tự mà không cắt giữa thẻ HTML.
fn thu_muc_gon(thu_muc: &str) -> String {
    let thu_muc = std::path::absolute(thu_muc).map(|p| p.display().to_string()).unwrap_or_else(|_| thu_muc.to_string());
    if thu_muc.chars().count() > 1000 { format!("…{}", thu_muc.chars().rev().take(1000).collect::<Vec<_>>().into_iter().rev().collect::<String>()) } else { thu_muc }
}

/// Giá trị in đậm (HTML của Telegram); mã hoá & < > để đường dẫn lạ không làm hỏng định dạng.
fn dam(gia_tri: &str) -> String {
    format!("<b>{}</b>", gia_tri.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"))
}

fn soan_tin(ten_pc: &str, mac: &str, thu_muc: &str, kq: &crate::dem_pdf::KetQuaDem, thoi_diem: &str, thoi_gian: Duration) -> String {
    use crate::gui::{so_nguyen, so_thuc, thoi_gian_phut_giay};
    let thu_muc = thu_muc_gon(thu_muc);
    let b = dam;
    format!(
        "Tên PC: {}; Địa chỉ Mac: {}\n\
         - Thư mục đếm: {};\n\
         - File PDF: {}; Số trang: {}; Quy ra A4: {}; Cảnh báo: {}; File lỗi: {}\n\
         - Thời điểm đếm: {}  ({})",
        b(ten_pc),
        b(mac),
        b(&thu_muc),
        b(&so_nguyen(kq.dong_tong_hop.len())),
        b(&so_nguyen(kq.tong_so_trang)),
        b(&so_thuc(kq.quy_a4.unwrap_or(0.0))),
        b(&so_nguyen(kq.so_trang_canh_bao)),
        b(&so_nguyen(kq.so_file_loi)),
        b(thoi_diem),
        thoi_gian_phut_giay(thoi_gian),
    )
}

/// Số liệu 1 lần tách (tin thống kê tách).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SoLieuTach {
    pub ho_so: usize,
    pub van_ban: usize,
    pub so_trang: usize,
    pub bo_index: usize,
    pub bo_trang: usize,
    pub split: usize,
    pub sao_nguyen: usize,
    pub ho_so_loi: usize,
}

/// Tin thống kê sau mỗi lần tách xong (cùng kiểu tin Đếm).
pub fn noi_dung_tach(thu_muc: &str, sl: &SoLieuTach, thoi_gian: Duration) -> String {
    let thoi_diem = chrono::Local::now().format("%d.%m.%Y, %H:%M:%S").to_string();
    soan_tin_tach(&ten_pc(), &dia_chi_mac(), thu_muc, sl, &thoi_diem, thoi_gian)
}

fn soan_tin_tach(ten_pc: &str, mac: &str, thu_muc: &str, sl: &SoLieuTach, thoi_diem: &str, thoi_gian: Duration) -> String {
    use crate::gui::{so_nguyen, thoi_gian_phut_giay};
    let n = |x: usize| dam(&so_nguyen(x));
    format!(
        "Tên PC: {}; Địa chỉ Mac: {}\n\
         - Thư mục tách: {};\n\
         - Hồ sơ: {}; Văn bản: {}; Số trang: {}; Bỏ index: {}; Bỏ trắng: {}; Split: {}; Sao nguyên: {}; Hồ sơ lỗi: {}\n\
         - Thời điểm tách: {}  ({})",
        dam(ten_pc),
        dam(mac),
        dam(&thu_muc_gon(thu_muc)),
        n(sl.ho_so),
        n(sl.van_ban),
        n(sl.so_trang),
        n(sl.bo_index),
        n(sl.bo_trang),
        n(sl.split),
        n(sl.sao_nguyen),
        n(sl.ho_so_loi),
        dam(thoi_diem),
        thoi_gian_phut_giay(thoi_gian),
    )
}

/// 2 dòng thống kê ngưỡng trắng nối vào tin tách (ngưỡng %, các sửa của người dùng, đề xuất lô, xoay).
pub fn dong_nguong(tk: &crate::tach_vb::thong_ke_nguong::ThongKeNguong, nguong_trang: f32, nguong_nghi: f32) -> String {
    use crate::gui::so_nguyen;
    let n = |x: u32| dam(&so_nguyen(x as usize));
    let pt = |x: f32| format!("{x:.2}").replace('.', ",");
    let de_xuat = tk.de_xuat.map_or("chưa đủ dữ liệu".to_string(), |(a, b)| format!("{} / {}", pt(a), pt(b)));
    format!(
        "\n- Ngưỡng: {}; Trắng tự động: {} (sửa {}); Nghi trắng: {} → Trắng {} · giữ {}; Trắng lọt: {}\n- Đề xuất lô: {}; Xoay: xác nhận {} · chỉnh {} · 0° {}",
        dam(&format!("{} % / {} %", pt(nguong_trang), pt(nguong_nghi))),
        n(tk.trang_tu_dong),
        n(tk.trang_tu_dong_sua),
        n(tk.nghi),
        n(tk.nghi_thanh_trang),
        n(tk.nghi_giu),
        n(tk.trang_lot),
        dam(&de_xuat),
        n(tk.xoay_xac_nhan),
        n(tk.xoay_chinh),
        n(tk.xoay_0),
    )
}

/// Webhook Google Sheet (Apps Script) nhận 1 dòng thống kê mỗi lần tách. Nhúng như token Telegram; trống = không gửi.
const TACH_WEBHOOK_NHUNG: &str = "";

/// Dòng `webhook=` trong telegram.txt (nếu có), không thì URL nhúng.
pub fn doc_webhook() -> Option<String> {
    let tu_file = duong_dan_cau_hinh().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| {
        s.lines().filter_map(|d| d.trim().strip_prefix("webhook")).filter_map(|d| d.trim_start().strip_prefix('=')).map(|v| v.trim().to_string()).find(|v| !v.is_empty())
    });
    tu_file.or_else(|| (!TACH_WEBHOOK_NHUNG.is_empty()).then(|| TACH_WEBHOOK_NHUNG.to_string()))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build().into()
}

/// Gửi file (CSV thống kê) vào nhóm bằng sendDocument (multipart dựng tay).
pub fn gui_file(cau_hinh: &CauHinh, ten_file: &str, noi_dung: &[u8]) -> Result<(), String> {
    gui_file_toi(API_TELEGRAM, cau_hinh, ten_file, noi_dung)
}

fn gui_file_toi(api: &str, cau_hinh: &CauHinh, ten_file: &str, noi_dung: &[u8]) -> Result<(), String> {
    let ranh = format!("----sm-dema4-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    let ten_file = ten_file.replace(['"', '\r', '\n'], "_");
    let mut than = format!("--{ranh}\r\nContent-Disposition: form-data; name=\"chat_id\"\r\n\r\n{}\r\n", cau_hinh.chat_id).into_bytes();
    than.extend(format!("--{ranh}\r\nContent-Disposition: form-data; name=\"document\"; filename=\"{ten_file}\"\r\nContent-Type: text/csv\r\n\r\n").as_bytes());
    than.extend_from_slice(noi_dung);
    than.extend(format!("\r\n--{ranh}--\r\n").as_bytes());
    let mut ph = agent()
        .post(format!("{api}/bot{}/sendDocument", cau_hinh.token))
        .header("Content-Type", format!("multipart/form-data; boundary={ranh}"))
        .send(&than[..])
        .map_err(|e| format!("không kết nối được Telegram ({e})"))?;
    let ma = ph.status().as_u16();
    let tra = ph.body_mut().read_to_string().unwrap_or_default();
    if ma == 200 && tra.contains("\"ok\":true") { Ok(()) } else { Err(format!("Telegram trả lỗi {ma}")) }
}

/// POST 1 dòng JSON tới webhook Apps Script (không thử lại; lỗi bỏ qua ở nơi gọi).
pub fn gui_webhook(url: &str, json: &str) -> Result<(), String> {
    agent().post(url).header("Content-Type", "application/json").send(json).map(|_| ()).map_err(|e| e.to_string())
}

pub fn gui_tin(cau_hinh: &CauHinh, noi_dung: &str) -> Result<(), String> {
    gui_tin_toi(API_TELEGRAM, cau_hinh, noi_dung)
}

/// Gọi sendMessage; trả về lỗi kèm mô tả của Telegram (ví dụ sai token, bot chưa vào nhóm)
fn gui_tin_toi(api: &str, cau_hinh: &CauHinh, noi_dung: &str) -> Result<(), String> {
    let noi_dung: String = if noi_dung.chars().count() > DO_DAI_TOI_DA {
        noi_dung.chars().take(DO_DAI_TOI_DA).chain("…".chars()).collect()
    } else {
        noi_dung.to_string()
    };
    let mut phan_hoi = agent()
        .post(format!("{api}/bot{}/sendMessage", cau_hinh.token))
        .send_form([("chat_id", cau_hinh.chat_id.as_str()), ("text", noi_dung.as_str()), ("parse_mode", "HTML")])
        .map_err(|e| format!("không kết nối được Telegram ({e})"))?;
    let ma = phan_hoi.status().as_u16();
    let than = phan_hoi.body_mut().read_to_string().unwrap_or_default();
    if ma == 200 && than.contains("\"ok\":true") {
        return Ok(());
    }
    // Lấy trường "description" trong JSON trả về để báo lỗi dễ hiểu
    let mo_ta = than.split("\"description\":\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or(&than);
    Err(format!("Telegram trả lỗi {ma}: {mo_ta}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn tin_nhan_dung_mau() {
        let kq = crate::dem_pdf::KetQuaDem {
            dong_tong_hop: vec![String::new(); 4],
            dong_chi_tiet: vec![],
            tong_hop: String::new(),
            so_file_loi: 0,
            tong_so_trang: 78,
            quy_a4: Some(128.0),
            so_trang_canh_bao: 0,
        };
        let tin = soan_tin("ABC", "AA:BB:CC:DD:EE:FF", "/Users/quangtv/Documents", &kq, "26.10.2026, 19:05:30", Duration::from_millis(65_400));
        assert_eq!(
            tin,
            "Tên PC: <b>ABC</b>; Địa chỉ Mac: <b>AA:BB:CC:DD:EE:FF</b>\n\
             - Thư mục đếm: <b>/Users/quangtv/Documents</b>;\n\
             - File PDF: <b>4</b>; Số trang: <b>78</b>; Quy ra A4: <b>128</b>; Cảnh báo: <b>0</b>; File lỗi: <b>0</b>\n\
             - Thời điểm đếm: <b>26.10.2026, 19:05:30</b>  (1 phút, 5,4 giây)"
        );
    }

    #[test]
    fn tin_tach_dung_mau() {
        let sl = SoLieuTach { ho_so: 2, van_ban: 211, so_trang: 476, bo_index: 209, bo_trang: 1, split: 0, sao_nguyen: 0, ho_so_loi: 0 };
        let tin = soan_tin_tach("ABC", "AA:BB", "/a/<b>", &sl, "29.10.2026, 11:30:00", Duration::from_millis(1_200));
        assert_eq!(
            tin,
            "Tên PC: <b>ABC</b>; Địa chỉ Mac: <b>AA:BB</b>\n\
             - Thư mục tách: <b>/a/&lt;b&gt;</b>;\n\
             - Hồ sơ: <b>2</b>; Văn bản: <b>211</b>; Số trang: <b>476</b>; Bỏ index: <b>209</b>; Bỏ trắng: <b>1</b>; Split: <b>0</b>; Sao nguyên: <b>0</b>; Hồ sơ lỗi: <b>0</b>\n\
             - Thời điểm tách: <b>29.10.2026, 11:30:00</b>  (0 phút, 1,2 giây)"
        );
    }

    #[test]
    fn dong_nguong_dung_mau() {
        use crate::tach_vb::thong_ke_nguong::ThongKeNguong;
        let tk = ThongKeNguong { trang_tu_dong: 120, trang_tu_dong_sua: 2, nghi: 30, nghi_thanh_trang: 25, nghi_giu: 5, trang_lot: 1, de_xuat: Some((0.9, 3.4)), xoay_xac_nhan: 7, xoay_chinh: 1, xoay_0: 2, ..Default::default() };
        assert_eq!(
            dong_nguong(&tk, 1.0, 3.2),
            "\n- Ngưỡng: <b>1,00 % / 3,20 %</b>; Trắng tự động: <b>120</b> (sửa <b>2</b>); Nghi trắng: <b>30</b> → Trắng <b>25</b> · giữ <b>5</b>; Trắng lọt: <b>1</b>\n\
             - Đề xuất lô: <b>0,90 / 3,40</b>; Xoay: xác nhận <b>7</b> · chỉnh <b>1</b> · 0° <b>2</b>"
        );
        assert!(dong_nguong(&ThongKeNguong::default(), 1.0, 3.2).contains("<b>chưa đủ dữ liệu</b>"));
    }

    #[test]
    fn gui_file_multipart_dung_dinh_dang() {
        let (api, luong) = may_chu_gia("200 OK|{\"ok\":true}");
        let cau_hinh = CauHinh { token: "123:ABC".into(), chat_id: "-1001".into() };
        assert_eq!(gui_file_toi(&api, &cau_hinh, "nguong_a.csv", b"v_tu;v_den\n0.0;0.1\n"), Ok(()));
        let request = luong.join().unwrap();
        assert!(request.starts_with("POST /bot123:ABC/sendDocument "));
        assert!(request.contains("multipart/form-data; boundary=----sm-dema4-"));
        assert!(request.contains("name=\"chat_id\"\r\n\r\n-1001\r\n"));
        assert!(request.contains("filename=\"nguong_a.csv\"") && request.contains("v_tu;v_den\n0.0;0.1\n"));
    }

    #[test]
    fn doc_cau_hinh_bo_qua_chu_thich_va_dong_trong() {
        let cau_hinh = phan_tich_cau_hinh("# chú thích\n\ntoken = 123:ABC\nchat_id=-1001\n");
        assert_eq!(cau_hinh, Some(CauHinh { token: "123:ABC".into(), chat_id: "-1001".into() }));
        assert_eq!(phan_tich_cau_hinh("token=123:ABC\n"), None);
        assert_eq!(phan_tich_cau_hinh("token=\nchat_id=-1\n"), None);
    }

    /// Máy chủ giả lập Telegram trên máy: trả về `phan_hoi` và gửi lại request nhận được
    fn may_chu_gia(phan_hoi: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let nghe = TcpListener::bind("127.0.0.1:0").unwrap();
        let dia_chi = format!("http://{}", nghe.local_addr().unwrap());
        let luong = std::thread::spawn(move || {
            let (mut ket_noi, _) = nghe.accept().unwrap();
            let mut buf = vec![0u8; 8192];
            let mut nhan = Vec::new();
            // Đọc tới hết phần thân theo Content-Length
            loop {
                let n = ket_noi.read(&mut buf).unwrap();
                nhan.extend_from_slice(&buf[..n]);
                let s = String::from_utf8_lossy(&nhan).to_string();
                if let Some((dau, than)) = s.split_once("\r\n\r\n") {
                    let do_dai = dau.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    if than.len() >= do_dai {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let (ma, than) = phan_hoi.split_once('|').unwrap();
            write!(ket_noi, "HTTP/1.1 {ma}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{than}", than.len()).unwrap();
            String::from_utf8_lossy(&nhan).to_string()
        });
        (dia_chi, luong)
    }

    #[test]
    fn gui_tin_dung_dia_chi_va_noi_dung() {
        let (api, luong) = may_chu_gia("200 OK|{\"ok\":true,\"result\":{}}");
        let cau_hinh = CauHinh { token: "123:ABC".into(), chat_id: "-1001".into() };
        assert_eq!(gui_tin_toi(&api, &cau_hinh, "Đã đếm xong appt"), Ok(()));
        let request = luong.join().unwrap();
        assert!(request.starts_with("POST /bot123:ABC/sendMessage "));
        assert!(request.contains("chat_id=-1001"));
        assert!(request.contains("text=%C4%90%C3%A3")); // "Đã" đã mã hoá URL
        assert!(request.contains("parse_mode=HTML"));
    }

    #[test]
    fn gui_tin_bao_mo_ta_loi_cua_telegram() {
        let (api, luong) = may_chu_gia("400 Bad Request|{\"ok\":false,\"error_code\":400,\"description\":\"Bad Request: chat not found\"}");
        let cau_hinh = CauHinh { token: "123:ABC".into(), chat_id: "-1".into() };
        assert_eq!(gui_tin_toi(&api, &cau_hinh, "x"), Err("Telegram trả lỗi 400: Bad Request: chat not found".into()));
        luong.join().unwrap();
    }
}
