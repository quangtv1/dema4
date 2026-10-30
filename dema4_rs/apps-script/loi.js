// Logic trạm license dema4 — hàm thuần, không gọi SpreadsheetApp / UrlFetchApp trực tiếp, để chạy được cả trong
// Apps Script (license.gs nối với Sheet thật) và Node (test/loi.test.js). Mọi chữ do máy khách gửi đều không đáng tin:
// kiểm mẫu, bỏ ký tự điều khiển (tin Telegram không bị chèn dòng giả), cắt độ dài, ghi ô dạng chữ thuần.

var MAU_MA_MAY = /^SM-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}$/;
var MAU_MA_LAN = /^[0-9a-f]{32}$/;
var MAU_PHIEN_BAN = /^\d{1,3}\.\d{1,3}\.\d{1,3}$/;
var TOI_DA_LAN_MOI_LOI_GOI = 3;
var GIOI_HAN_DANG_KY_MOI_GIO = 30;
var GIOI_HAN_LOI_GOI_MOI_PHUT = 120;
var TOI_DA_HANG_DOI = 2000;
var HAN_MAC_DINH = 10; // lượt dùng thử mỗi chức năng khi ô Hạn mức trống
// Cột tab license (1-based)
var COT = { MA: 1, TEN: 2, MAC: 3, IP_NB: 4, IP_PUB: 5, PHIEN_BAN: 6, NGAY_DK: 7, LAN_CUOI: 8, DUNG_DEM: 9, DUNG_TACH: 10, HAN_DEM: 11, HAN_TACH: 12, TRANG_THAI: 13, MA_LAN: 16, DUNG_MUC_LUC: 17, HAN_MUC_LUC: 18 };
// Cột Mục lục thêm SAU cột ẩn Mã lần (P) để không xê dịch cột cũ; dòng do trạm cũ tạo có thể thiếu → mặc định
var SO_COT_MAY = 18;
// Số mã lần gần nhất giữ trong dòng máy để chống trùng (app gửi lại tối đa vài lần chưa được xác nhận)
var GIU_MA_LAN = 20;
var TEN_CN = { dem: 'Đếm', tach: 'Tách', muc_luc: 'Mục lục' };
// Cột Đã dùng / Hạn mức + khoá trong dữ liệu ký theo từng chức năng
var CN = {
  dem: { dung: COT.DUNG_DEM, han: COT.HAN_DEM },
  tach: { dung: COT.DUNG_TACH, han: COT.HAN_TACH },
  muc_luc: { dung: COT.DUNG_MUC_LUC, han: COT.HAN_MUC_LUC },
};

/** Bỏ ký tự điều khiển, cắt `toiDa` ký tự. */
function sachChu(s, toiDa) {
  return String(s == null ? '' : s).replace(/[\u0000-\u001f\u007f]/g, '').trim().slice(0, toiDa);
}

/** Ô chữ thuần trên Sheet: chuỗi bắt đầu bằng = + - @ thêm dấu nháy để Sheet không hiểu là công thức. */
function oChu(s) {
  s = String(s);
  return /^[=+\-@]/.test(s) ? "'" + s : s;
}

function ipHopLe(s) {
  s = String(s == null ? '' : s).trim();
  return s.length > 0 && s.length <= 45 && /^[0-9a-fA-F:.]+$/.test(s) ? s : '?';
}

function soKhongAm(x) {
  var n = Number(x);
  return isFinite(n) && n >= 0 ? n : 0;
}

/** Thông tin máy đã làm sạch. */
function sachMay(m) {
  m = m || {};
  return {
    ten: oChu(sachChu(m.ten, 100)),
    mac: oChu(sachChu(m.mac, 40)),
    ip_noi_bo: ipHopLe(m.ip_noi_bo),
    ip_public: ipHopLe(m.ip_public),
    phien_ban: MAU_PHIEN_BAN.test(String(m.phien_ban)) ? String(m.phien_ban) : '?',
  };
}

/** Ô Trạng thái (do Sao Mai chọn) → mã ASCII; giá trị lạ → khoa (không vô tình mở khoá). */
function maTrangThai(o) {
  var s = String(o == null ? '' : o).normalize('NFC').trim();
  if (s === '' || s === 'Dùng thử') return 'dung_thu';
  if (s === 'Đã cấp') return 'da_cap';
  return 'khoa';
}

/** Ô hạn mức: trống → 2; không phải số → 0. */
function hanMuc(o) {
  if (o === '' || o == null) return HAN_MAC_DINH;
  var n = Number(o);
  return isFinite(n) && n >= 0 ? Math.floor(n) : 0;
}

function thoiDiem(d) {
  var p = function (n) { return (n < 10 ? '0' : '') + n; };
  var t = new Date(d.getTime() + 7 * 3600 * 1000); // giờ Việt Nam, không phụ thuộc múi giờ máy chạy
  return t.getUTCFullYear() + '-' + p(t.getUTCMonth() + 1) + '-' + p(t.getUTCDate()) + ' ' + p(t.getUTCHours()) + ':' + p(t.getUTCMinutes()) + ':' + p(t.getUTCSeconds());
}

function thoiDiemTin(d) {
  var s = thoiDiem(d); // yyyy-mm-dd hh:mm:ss → dd.mm.yyyy, hh:mm:ss
  return s.slice(8, 10) + '.' + s.slice(5, 7) + '.' + s.slice(0, 4) + ', ' + s.slice(11);
}

function maNgauNhien() {
  var s = '';
  for (var i = 0; i < 32; i++) s += Math.floor(Math.random() * 16).toString(16);
  return s;
}

/** Trạng thái license của 1 dòng (mảng 15 ô). */
function trangThaiDong(d) {
  var tt = { trang_thai: maTrangThai(d[COT.TRANG_THAI - 1]) };
  Object.keys(CN).forEach(function (cn) {
    tt['han_' + cn] = hanMuc(d[CN[cn].han - 1]);
    tt['da_dung_' + cn] = soKhongAm(d[CN[cn].dung - 1]);
  });
  return tt;
}

/**
 * Xử lý 1 lời gọi. `kho`: timMay, themMay, capNhatMay, daCoMaLan, themNhatKy, choPhep, soTinCho, xepTin, themNguong.
 * Trả { ok:true, du_lieu } (license: du_lieu sẽ được ký ở license.gs) hoặc { ok:false, loi }.
 */
function xuLy(yc, kho, bayGio) {
  yc = yc || {};
  var hd = yc.hanh_dong;
  if (['kiem_tra', 'ghi_lan', 'thong_ke_dem', 'thong_ke_tach', 'thong_ke_muc_luc'].indexOf(hd) < 0) return { ok: false, loi: 'hanh_dong' };
  if (!MAU_MA_MAY.test(String(yc.ma_may))) return { ok: false, loi: 'ma_may' };
  var nonce = sachChu(yc.nonce, 64);
  if (!kho.choPhep('goi', GIOI_HAN_LOI_GOI_MOI_PHUT, 60)) return { ok: false, loi: 'ban' };
  var may = sachMay(yc.may);
  var luc = thoiDiem(bayGio);

  if (hd.indexOf('thong_ke_') === 0) return thongKe(hd, yc, may, kho, bayGio);

  var lan = [];
  if (hd === 'ghi_lan') {
    if (!Array.isArray(yc.lan) || yc.lan.length === 0 || yc.lan.length > TOI_DA_LAN_MOI_LOI_GOI) return { ok: false, loi: 'lan' };
    for (var i = 0; i < yc.lan.length; i++) {
      var l = yc.lan[i] || {};
      if (!MAU_MA_LAN.test(String(l.ma_lan)) || !TEN_CN[l.cn]) return { ok: false, loi: 'lan' };
      lan.push({ ma_lan: l.ma_lan, cn: l.cn, nguon: l.nguon === 'bu_offline' ? 'bu_offline' : 'online' });
    }
  }

  var tim = kho.timMay(yc.ma_may);
  if (!tim) {
    if (!kho.choPhep('dang_ky', GIOI_HAN_DANG_KY_MOI_GIO, 3600)) return { ok: false, loi: 'ban' };
    kho.themMay([yc.ma_may, may.ten, may.mac, may.ip_noi_bo, may.ip_public, may.phien_ban, luc, luc, 0, 0, '', '', 'Dùng thử', '', '', '', 0, '']);
    tim = kho.timMay(yc.ma_may);
    xepThongBao(kho, tinMayMoi(yc.ma_may, may, thoiDiemTin(bayGio)));
  }
  var d = tim.giaTri;
  var cot = {};
  cot[COT.IP_NB] = may.ip_noi_bo;
  cot[COT.IP_PUB] = may.ip_public;
  cot[COT.PHIEN_BAN] = may.phien_ban;
  cot[COT.LAN_CUOI] = luc;

  var daGhi = [];
  var truoc = {}, dung = {};
  Object.keys(CN).forEach(function (cn) { truoc[cn] = dung[cn] = soKhongAm(d[CN[cn].dung - 1]); });
  // Chống trùng: mã lần đã ghi nằm ở cột ẩn "Mã lần gần đây" của chính dòng máy
  var daCo = String(d[COT.MA_LAN - 1] || '').split(',').filter(Boolean);
  var them = 0;
  lan.forEach(function (l) {
    daGhi.push(l.ma_lan);
    if (daCo.indexOf(l.ma_lan) >= 0) return;
    dung[l.cn]++;
    daCo.push(l.ma_lan);
    them++;
  });
  if (them) {
    Object.keys(CN).forEach(function (cn) { if (dung[cn] !== truoc[cn]) cot[CN[cn].dung] = dung[cn]; });
    cot[COT.MA_LAN] = daCo.slice(-GIU_MA_LAN).join(',');
  }
  kho.capNhatMay(tim.hang, cot);
  Object.keys(CN).forEach(function (cn) { d[CN[cn].dung - 1] = dung[cn]; });

  var tt = trangThaiDong(d);
  // Lần chạy này làm chạm hạn mức → báo 1 lần (gửi lại / kiểm tra lại không báo thêm)
  Object.keys(CN).forEach(function (cn) {
    var han = tt['han_' + cn];
    if (truoc[cn] < han && dung[cn] >= han) xepThongBao(kho, tinHetLuot(yc.ma_may, may, cn, dung[cn], han, tt.trang_thai, thoiDiemTin(bayGio)));
  });
  var du = { ma_may: yc.ma_may, nonce: nonce, trang_thai: tt.trang_thai, da_ghi: daGhi };
  Object.keys(CN).forEach(function (cn) { du['han_' + cn] = tt['han_' + cn]; du['da_dung_' + cn] = tt['da_dung_' + cn]; });
  return { ok: true, du_lieu: du };
}

function thongKe(hd, yc, may, kho, bayGio) {
  if (kho.soTinCho() >= TOI_DA_HANG_DOI) return { ok: false, loi: 'hang_doi_day' };
  var sl = yc.so_lieu || {};
  var ng = yc.nguong;
  // (đã dùng / hạn mức) cho tiêu đề: app ghi lượt lên trạm trước rồi mới gửi thống kê, nên số đã gồm lần này
  var tim = kho.timMay(yc.ma_may), luot = null;
  if (tim) {
    var tt = trangThaiDong(tim.giaTri);
    var cn = hd.slice('thong_ke_'.length);
    luot = { da_dung: tt['da_dung_' + cn], han: tt['han_' + cn] };
  }
  var luc = thoiDiemTin(bayGio);
  var tin = hd === 'thong_ke_dem' ? tinDem(yc.ma_may, may, sl, luc, luot)
    : hd === 'thong_ke_muc_luc' ? tinMucLuc(yc.ma_may, may, sl, luc, luot)
    : tinTach(yc.ma_may, may, sl, ng, luc, luot);
  var hang = { ma: maNgauNhien(), luc: 0, trang_thai: 'cho', so_lan_thu: 0, noi_dung: tin, ten_csv: '', csv: '' };
  if (hd === 'thong_ke_tach' && ng) {
    var csv = String(ng.csv || '');
    if (csv.length > 0 && csv.length <= 20000) {
      hang.csv = csv;
      hang.ten_csv = sachChu(ng.ten_csv, 120).replace(/[^\w.\-]/g, '_') || 'nguong.csv';
    }
    if (Array.isArray(ng.dong) && ng.dong.length <= 40) {
      var dong = ng.dong.map(function (o) { return typeof o === 'number' ? o : oChu(sachChu(o, 4000)); });
      // Ô thời điểm, máy, MAC, IP lấy theo lời gọi — không tin giá trị app gửi
      dong[0] = thoiDiem(bayGio); dong[1] = may.ten; dong[2] = may.mac; dong[3] = may.ip_noi_bo; dong[4] = may.ip_public;
      var tieuDe = (Array.isArray(ng.tieu_de) ? ng.tieu_de : []).slice(0, 40).map(function (o) { return oChu(sachChu(o, 100)); });
      var moTa = (Array.isArray(ng.mo_ta) ? ng.mo_ta : []).slice(0, 60).map(function (r) { return (Array.isArray(r) ? r : []).slice(0, 3).map(function (o) { return oChu(sachChu(o, 500)); }); });
      kho.themNguong(tieuDe, moTa, dong);
    }
  }
  kho.xepTin(hang);
  return { ok: true };
}

// ---- Thông báo sự kiện license (máy mới, hết lượt) ----

/** Xếp 1 tin chữ vào hàng đợi; hàng đợi đầy thì bỏ (không làm hỏng lời gọi license). */
function xepThongBao(kho, noiDung) {
  if (kho.soTinCho() >= TOI_DA_HANG_DOI) return;
  kho.xepTin({ ma: maNgauNhien(), luc: 0, trang_thai: 'cho', so_lan_thu: 0, noi_dung: noiDung, ten_csv: '', csv: '' });
}

// ---- Soạn tin Telegram: chữ thường (không HTML), 2 phần gạch đầu dòng — máy tính / chức năng ----

var GACH = '-----------------';

/** Khung chung: tiêu đề, phần máy tính (lấy theo lời gọi), đường gạch, phần chức năng. */
function soanTin(tieuDe, maMay, may, dongChucNang) {
  return tieuDe + '\n\n' +
    'Thông tin máy tính\n' +
    '- Mã máy: ' + maMay + '\n' +
    '- Tên PC: ' + may.ten + '\n' +
    '- Địa chỉ MAC: ' + may.mac + '\n' +
    '- IP nội bộ: ' + may.ip_noi_bo + '\n' +
    '- IP public: ' + may.ip_public + '\n' +
    '- Phiên bản: ' + may.phien_ban + '\n' +
    GACH + '\n' +
    'Thông tin chức năng\n' +
    dongChucNang.map(function (d) { return '- ' + d; }).join('\n');
}

function tinMayMoi(maMay, may, luc) {
  return soanTin('🆕 MÁY MỚI ĐĂNG KÝ', maMay, may, [
    'Dùng thử: ' + HAN_MAC_DINH + ' lần Đếm · ' + HAN_MAC_DINH + ' lần Tách · ' + HAN_MAC_DINH + ' lần Sinh mục lục',
    'Thời điểm: ' + luc,
  ]);
}

function tinHetLuot(maMay, may, cn, daDung, han, trangThai, luc) {
  var tenTT = { dung_thu: 'Dùng thử', da_cap: 'Đã cấp', khoa: 'Khoá' }[trangThai] || trangThai;
  return soanTin('⛔ HẾT LƯỢT ' + TEN_CN[cn].toUpperCase(), maMay, may, [
    'Chức năng: ' + TEN_CN[cn],
    'Đã dùng: ' + daDung + '/' + han,
    'Trạng thái: ' + tenTT,
    'Thời điểm: ' + luc,
    'Cấp thêm: tab 1_license → sửa Hạn mức ' + TEN_CN[cn] + ' (tổng mới) + Trạng thái Đã cấp',
  ]);
}

function soNguyen(n) {
  var s = String(Math.floor(soKhongAm(n)));
  return s.replace(/\B(?=(\d{3})+(?!\d))/g, '.');
}
function soThuc(x) {
  var s = soKhongAm(x).toFixed(2).split('.');
  var tp = s[1].replace(/0+$/, '');
  return soNguyen(Number(s[0])) + (tp ? ',' + tp : '');
}
function phutGiay(ms) {
  var giay = soKhongAm(ms) / 1000;
  var phut = Math.floor(giay / 60);
  return phut + ' phút, ' + (giay - phut * 60).toFixed(1).replace('.', ',') + ' giây';
}
function thuMucGon(s) {
  s = sachChu(s, 4000);
  return s.length > 1000 ? '…' + s.slice(-1000) : s;
}

/** Tiêu đề kèm (đã dùng/hạn mức) khi biết dòng máy. */
function kemLuot(tieuDe, luot) {
  return luot ? tieuDe + ' (' + luot.da_dung + '/' + luot.han + ')' : tieuDe;
}

function tinDem(maMay, may, sl, luc, luot) {
  return soanTin(kemLuot('📊 ĐẾM TRANG A4', luot), maMay, may, [
    'Thư mục: ' + thuMucGon(sl.thu_muc),
    'File PDF: ' + soNguyen(sl.so_file),
    'Số trang: ' + soNguyen(sl.so_trang),
    'Quy ra A4: ' + soThuc(sl.quy_a4),
    'Cảnh báo: ' + soNguyen(sl.canh_bao),
    'File lỗi: ' + soNguyen(sl.file_loi),
    'Thời điểm: ' + luc,
    'Thời gian chạy: ' + phutGiay(sl.thoi_gian_ms),
  ]);
}

/** `ng`: thống kê ngưỡng trắng app gửi kèm ({tk, t1, t2}); không có → bỏ phần ngưỡng. */
function tinTach(maMay, may, sl, ng, luc, luot) {
  var d = [
    'Thư mục: ' + thuMucGon(sl.thu_muc),
    'Hồ sơ: ' + soNguyen(sl.ho_so),
    'Văn bản: ' + soNguyen(sl.van_ban),
    'Số trang: ' + soNguyen(sl.so_trang),
    'Bỏ index: ' + soNguyen(sl.bo_index),
    'Bỏ trắng: ' + soNguyen(sl.bo_trang),
    'Split: ' + soNguyen(sl.split),
    'Sao nguyên: ' + soNguyen(sl.sao_nguyen),
    'Hồ sơ lỗi: ' + soNguyen(sl.ho_so_loi),
    'Thời điểm: ' + luc,
    'Thời gian chạy: ' + phutGiay(sl.thoi_gian_ms),
  ];
  if (ng) {
    var tk = ng.tk || {};
    var n = function (x) { return soNguyen(x); };
    var pt = function (x) { return soKhongAm(x).toFixed(2).replace('.', ','); };
    var deXuat = Array.isArray(tk.de_xuat) && tk.de_xuat.length === 2 ? pt(tk.de_xuat[0]) + ' / ' + pt(tk.de_xuat[1]) : 'chưa đủ dữ liệu';
    d.push(
      'Ngưỡng trắng / nghi: ' + pt(ng.t1) + ' % / ' + pt(ng.t2) + ' %',
      'Trắng tự động: ' + n(tk.trang_tu_dong) + ' (sửa ' + n(tk.trang_tu_dong_sua) + ')',
      'Nghi trắng: ' + n(tk.nghi) + ' → Trắng ' + n(tk.nghi_thanh_trang) + ' · giữ ' + n(tk.nghi_giu),
      'Trắng lọt: ' + n(tk.trang_lot),
      'Đề xuất lô: ' + deXuat,
      'Xoay: xác nhận ' + n(tk.xoay_xac_nhan) + ' · chỉnh ' + n(tk.xoay_chinh) + ' · 0° ' + n(tk.xoay_0)
    );
  }
  return soanTin(kemLuot('✂️ TÁCH VĂN BẢN', luot), maMay, may, d);
}

function tinMucLuc(maMay, may, sl, luc, luot) {
  return soanTin(kemLuot('📑 SINH MỤC LỤC', luot), maMay, may, [
    'Dữ liệu: ' + thuMucGon(sl.du_lieu),
    'Mẫu: ' + sachChu(sl.mau, 200),
    'Hồ sơ: ' + soNguyen(sl.ho_so),
    'Văn bản: ' + soNguyen(sl.van_ban),
    'File ra: ' + soNguyen(sl.docx) + ' DOCX · ' + soNguyen(sl.pdf) + ' PDF',
    'Lỗi: ' + soNguyen(sl.loi),
    'Thời điểm: ' + luc,
    'Thời gian chạy: ' + phutGiay(sl.thoi_gian_ms),
  ]);
}

// ---- Hàng đợi Telegram ----

/**
 * Chọn tin để gửi trong 1 lượt: dòng "đang gửi" quá 5 phút trả về "chờ" (lượt sau gửi); lấy dòng "chờ" theo thứ tự
 * cho đến hết ngân sách lời gọi Telegram (tin có CSV tốn 2).
 */
function chonGui(ds, bayGioMs, nganSach) {
  var traVe = [], gui = [], tieu = 0;
  ds.forEach(function (r) {
    if (r.trang_thai === 'dang_gui' && bayGioMs - Number(r.luc) > 5 * 60 * 1000) traVe.push(r);
  });
  for (var i = 0; i < ds.length; i++) {
    var r = ds[i];
    if (r.trang_thai !== 'cho') continue;
    var ton = r.ten_csv ? 2 : 1;
    if (tieu + ton > nganSach) break;
    tieu += ton;
    gui.push(r);
  }
  return { traVe: traVe, gui: gui };
}

if (typeof module !== 'undefined') {
  module.exports = { GIU_MA_LAN: GIU_MA_LAN, xuLy: xuLy, oChu: oChu, sachChu: sachChu, maTrangThai: maTrangThai, hanMuc: hanMuc, tinDem: tinDem, tinTach: tinTach, tinMucLuc: tinMucLuc, tinMayMoi: tinMayMoi, tinHetLuot: tinHetLuot, soNguyen: soNguyen, soThuc: soThuc, chonGui: chonGui, thoiDiem: thoiDiem, COT: COT };
}
