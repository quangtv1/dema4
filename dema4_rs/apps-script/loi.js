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
var HAN_MAC_DINH = 10; // tổng lượt dùng thử (chung mọi chức năng) khi ô Hạn mức trống
// Cột tab license (1-based). Một hạn mức + một số đã dùng chung cho Đếm / Tách / Sinh mục lục
var COT = { MA: 1, TEN: 2, MAC: 3, IP_NB: 4, IP_PUB: 5, PHIEN_BAN: 6, NGAY_DK: 7, LAN_CUOI: 8, DUNG: 9, HAN: 10, TRANG_THAI: 11, DON_VI: 12, GHI_CHU: 13, MA_LAN: 14 };
var SO_COT_MAY = 14;
// Số mã lần gần nhất giữ trong dòng máy để chống trùng (app gửi lại tối đa vài lần chưa được xác nhận)
var GIU_MA_LAN = 20;
var TEN_CN = { dem: 'Đếm', tach: 'Tách', muc_luc: 'Mục lục' };

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

/** Ô hạn mức: trống → HAN_MAC_DINH; không phải số → 0. */
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

/** Trạng thái license của 1 dòng: tổng hạn mức + tổng đã dùng (mọi chức năng). */
function trangThaiDong(d) {
  return { trang_thai: maTrangThai(d[COT.TRANG_THAI - 1]), han: hanMuc(d[COT.HAN - 1]), da_dung: soKhongAm(d[COT.DUNG - 1]) };
}

/**
 * Dữ liệu ký gửi app: `han` / `da_dung` (tổng) + khoá cũ theo từng chức năng gán CÙNG số tổng — app bản cũ
 * (đọc han_dem / da_dung_tach…) cũng chặn theo tổng mà không cần cập nhật.
 */
function duLieuKy(maMay, nonce, tt, daGhi) {
  var du = { ma_may: maMay, nonce: nonce, trang_thai: tt.trang_thai, da_ghi: daGhi, han: tt.han, da_dung: tt.da_dung };
  Object.keys(TEN_CN).forEach(function (cn) { du['han_' + cn] = tt.han; du['da_dung_' + cn] = tt.da_dung; });
  return du;
}

/** Tiêu đề cột J (Đã dùng Tách) của bảng cũ — có nghĩa là bảng còn theo từng chức năng, cần chuyển. */
var TIEU_DE_COT_J_CU = 'Đã dùng Tách';

/**
 * Dòng bảng cũ (18 cột: I/J/Q đã dùng Đếm/Tách/Mục lục, K/L/R hạn mức, M trạng thái, N đơn vị, O ghi chú, P mã lần)
 * → dòng bảng mới 14 cột: Đã dùng = tổng 3 chức năng, Hạn mức = tổng 3 hạn mức cũ (ô trống tính HAN_MAC_DINH).
 */
function chuyenDongCu(r) {
  var o = function (i) { return r[i] == null ? '' : r[i]; };
  var dung = soKhongAm(o(8)) + soKhongAm(o(9)) + soKhongAm(o(16));
  var han = hanMuc(o(10)) + hanMuc(o(11)) + hanMuc(o(17));
  return [o(0), o(1), o(2), o(3), o(4), o(5), o(6), o(7), dung, han, o(12), o(13), o(14), o(15)];
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
    kho.themMay([yc.ma_may, may.ten, may.mac, may.ip_noi_bo, may.ip_public, may.phien_ban, luc, luc, 0, '', 'Dùng thử', '', '', '']);
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
  var truoc = soKhongAm(d[COT.DUNG - 1]), dung = truoc, cnCuoi = null;
  // Chống trùng: mã lần đã ghi nằm ở cột ẩn "Mã lần gần đây" của chính dòng máy
  var daCo = String(d[COT.MA_LAN - 1] || '').split(',').filter(Boolean);
  lan.forEach(function (l) {
    daGhi.push(l.ma_lan);
    if (daCo.indexOf(l.ma_lan) >= 0) return;
    dung++; // mỗi lần dùng bất kỳ chức năng nào = 1 lượt
    cnCuoi = l.cn;
    daCo.push(l.ma_lan);
  });
  if (dung !== truoc) {
    cot[COT.DUNG] = dung;
    cot[COT.MA_LAN] = daCo.slice(-GIU_MA_LAN).join(',');
  }
  kho.capNhatMay(tim.hang, cot);
  d[COT.DUNG - 1] = dung;

  var tt = trangThaiDong(d);
  // Lần chạy này làm chạm hạn mức → báo 1 lần (gửi lại / kiểm tra lại không báo thêm)
  if (truoc < tt.han && dung >= tt.han) xepThongBao(kho, tinHetLuot(yc.ma_may, may, cnCuoi, dung, tt.han, tt.trang_thai, thoiDiemTin(bayGio)));
  return { ok: true, du_lieu: duLieuKy(yc.ma_may, nonce, tt, daGhi) };
}

function thongKe(hd, yc, may, kho, bayGio) {
  if (kho.soTinCho() >= TOI_DA_HANG_DOI) return { ok: false, loi: 'hang_doi_day' };
  var sl = yc.so_lieu || {};
  var ng = yc.nguong;
  // (đã dùng / hạn mức) cho tiêu đề: app ghi lượt lên trạm trước rồi mới gửi thống kê, nên số đã gồm lần này
  var tim = kho.timMay(yc.ma_may), luot = null;
  if (tim) {
    var tt = trangThaiDong(tim.giaTri);
    luot = { da_dung: tt.da_dung, han: tt.han };
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
    'Dùng thử: ' + HAN_MAC_DINH + ' lần (dùng chung Đếm · Tách · Sinh mục lục)',
    'Thời điểm: ' + luc,
  ]);
}

/** `cn`: chức năng của lần chạy làm chạm hạn mức (hạn mức dùng chung mọi chức năng). */
function tinHetLuot(maMay, may, cn, daDung, han, trangThai, luc) {
  var tenTT = { dung_thu: 'Dùng thử', da_cap: 'Đã cấp', khoa: 'Khoá' }[trangThai] || trangThai;
  return soanTin('⛔ HẾT LƯỢT', maMay, may, [
    'Lần cuối: ' + (TEN_CN[cn] || '?'),
    'Đã dùng: ' + daDung + '/' + han,
    'Trạng thái: ' + tenTT,
    'Thời điểm: ' + luc,
    'Cấp thêm: tab 1_license → sửa Hạn mức (tổng mới) + Trạng thái Đã cấp',
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
  module.exports = { chuyenDongCu: chuyenDongCu, TIEU_DE_COT_J_CU: TIEU_DE_COT_J_CU, GIU_MA_LAN: GIU_MA_LAN, HAN_MAC_DINH: HAN_MAC_DINH, SO_COT_MAY: SO_COT_MAY, xuLy: xuLy, oChu: oChu, sachChu: sachChu, maTrangThai: maTrangThai, hanMuc: hanMuc, tinDem: tinDem, tinTach: tinTach, tinMucLuc: tinMucLuc, tinMayMoi: tinMayMoi, tinHetLuot: tinHetLuot, soNguyen: soNguyen, soThuc: soThuc, chonGui: chonGui, thoiDiem: thoiDiem, COT: COT };
}
