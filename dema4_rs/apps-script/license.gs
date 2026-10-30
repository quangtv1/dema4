// Trạm license dema4 — lớp mỏng nối logic thuần (loi.gs = loi.js) với Google Sheet / Cache / Lock / UrlFetch.
// Triển khai: xem README.md cùng thư mục. Bí mật duy nhất: Script Properties KHOA_BI_MAT (khoá RSA PKCS#8 PEM).

var TAB = {
  MAY: 'license',
  NHAT_KY: 'license_nhat_ky',
  HANG_DOI: 'Hàng đợi Telegram',
  MO_TA: 'Mô tả license',
  CAU_HINH: 'Cấu hình',
  NGUONG: 'Ngưỡng trắng',
  MO_TA_NGUONG: 'Mô tả ngưỡng trắng',
};
var TIEU_DE = {
  MAY: ['Mã máy', 'Tên máy', 'MAC', 'IP nội bộ', 'IP public', 'Phiên bản', 'Ngày đăng ký', 'Lần dùng cuối', 'Đã dùng Đếm', 'Đã dùng Tách', 'Hạn mức Đếm', 'Hạn mức Tách', 'Trạng thái', 'Đơn vị', 'Ghi chú'],
  NHAT_KY: ['Thời điểm', 'Mã máy', 'Mã lần', 'Chức năng', 'Nguồn', 'Phiên bản', 'IP nội bộ', 'IP public'],
  HANG_DOI: ['Mã', 'Thời điểm', 'Nội dung', 'Tên CSV', 'CSV', 'Số lần thử', 'Trạng thái', 'Lúc đánh dấu'],
  CAU_HINH: ['Khoá', 'Giá trị'],
};
var MO_TA_LICENSE = [
  ['Cột', 'Ý nghĩa', 'Ai sửa'],
  ['Mã máy', 'Băm từ ID hệ điều hành của máy (không đổi khi đổi tên máy / card mạng). Khách đọc mã này trên popup.', 'Trạm'],
  ['Tên máy · MAC · IP', 'Thông tin máy ở lần liên lạc gần nhất.', 'Trạm'],
  ['Đã dùng Đếm / Tách', 'Tổng số lần đã dùng; trạm cộng +1 mỗi lần (chống trùng theo mã lần chạy).', 'Trạm — không sửa tay'],
  ['Hạn mức Đếm / Tách', 'TỔNG số lần được phép. Trống = 2 (dùng thử).', 'Sao Mai'],
  ['Trạng thái', 'Dùng thử / Đã cấp / Khoá (chọn trong danh sách). Giá trị khác bị coi là Khoá.', 'Sao Mai'],
  ['Đơn vị · Ghi chú', 'Tự do (hợp đồng, ngày cấp…).', 'Sao Mai'],
  ['', '', ''],
  ['Nâng lượt', 'Tìm mã máy → sửa Hạn mức thành tổng mới (vd đã dùng 37, cho thêm 100 → 137) → Trạng thái Đã cấp → khách bấm Kiểm tra lại.', ''],
  ['Khoá máy', 'Trạng thái = Khoá → máy chạy được tối đa thêm 1 lần rồi bị chặn.', ''],
  ['Không được', 'Xoá dòng máy (máy đăng ký lại từ 0) · sửa cột Đã dùng · gõ trạng thái ngoài danh sách.', ''],
];

function bang(ten, tieuDe, moTa) {
  var ss = SpreadsheetApp.getActiveSpreadsheet();
  var sh = ss.getSheetByName(ten);
  if (!sh) {
    sh = ss.insertSheet(ten, ss.getNumSheets());
    if (tieuDe && tieuDe.length) {
      sh.getRange(1, 1, 1, tieuDe.length).setValues([tieuDe]).setFontWeight('bold');
      sh.setFrozenRows(1);
    }
    if (ten === TAB.MAY) {
      var quyTac = SpreadsheetApp.newDataValidation().requireValueInList(['Dùng thử', 'Đã cấp', 'Khoá'], true).setAllowInvalid(false).build();
      sh.getRange('M2:M').setDataValidation(quyTac);
      sh.getRange('K1:O1').setBackground('#FFF2CC');
      sh.getRange('A1:J1').setBackground('#EDEDED');
      bang(TAB.MO_TA, null).getRange(1, 1, MO_TA_LICENSE.length, 3).setValues(MO_TA_LICENSE);
    }
    if (moTa && moTa.length) {
      var mt = ss.getSheetByName(TAB.MO_TA_NGUONG) || ss.insertSheet(TAB.MO_TA_NGUONG, ss.getNumSheets());
      if (mt.getLastRow() === 0) mt.getRange(1, 1, moTa.length, 3).setValues(moTa.map(function (r) { return [r[0] || '', r[1] || '', r[2] || '']; }));
    }
  }
  return sh;
}

function themDong(sh, rows) {
  if (!rows.length) return;
  sh.getRange(sh.getLastRow() + 1, 1, rows.length, rows[0].length).setValues(rows);
}

function khoSheet() {
  var cache = CacheService.getScriptCache();
  var may = bang(TAB.MAY, TIEU_DE.MAY);
  var nhatKy = bang(TAB.NHAT_KY, TIEU_DE.NHAT_KY);
  var hangDoi = bang(TAB.HANG_DOI, TIEU_DE.HANG_DOI);
  return {
    timMay: function (ma) {
      var o = may.getRange('A:A').createTextFinder(ma).matchEntireCell(true).findNext();
      if (!o) return null;
      return { hang: o.getRow(), giaTri: may.getRange(o.getRow(), 1, 1, 15).getValues()[0] };
    },
    themMay: function (r) { themDong(may, [r]); },
    capNhatMay: function (hang, cot) { Object.keys(cot).forEach(function (c) { may.getRange(hang, Number(c)).setValue(cot[c]); }); },
    daCoMaLan: function (ma) {
      if (cache.get('lan_' + ma)) return true;
      return !!nhatKy.getRange('C:C').createTextFinder(ma).matchEntireCell(true).findNext();
    },
    themNhatKy: function (rows) {
      themDong(nhatKy, rows);
      rows.forEach(function (r) { cache.put('lan_' + r[2], '1', 21600); });
    },
    choPhep: function (khoa, gioiHan, giay) {
      var k = 'dem_' + khoa + '_' + Math.floor(Date.now() / 1000 / giay);
      var n = Number(cache.get(k) || 0) + 1;
      cache.put(k, String(n), giay + 60);
      return n <= gioiHan;
    },
    soTinCho: function () {
      var n = hangDoi.getLastRow() - 1;
      if (n <= 0) return 0;
      return hangDoi.getRange(2, 7, n, 1).getValues().filter(function (r) { return r[0] === 'cho'; }).length;
    },
    xepTin: function (h) { themDong(hangDoi, [[h.ma, thoiDiem(new Date()), oChu(h.noi_dung), h.ten_csv, oChu(h.csv), 0, 'cho', 0]]); },
    themNguong: function (tieuDe, moTa, dong) { themDong(bang(TAB.NGUONG, tieuDe, moTa), [dong]); },
  };
}

function traJson(o) {
  return ContentService.createTextOutput(JSON.stringify(o)).setMimeType(ContentService.MimeType.JSON);
}

function khoaBiMat() {
  var k = PropertiesService.getScriptProperties().getProperty('KHOA_BI_MAT') || '';
  return k.replace(/\\n/g, '\n').trim() + '\n';
}

function ky(chuoi) {
  return Utilities.base64Encode(Utilities.computeRsaSha256Signature(chuoi, khoaBiMat(), Utilities.Charset.UTF_8));
}

/** Điểm vào của app. Lỗi luôn trả JSON {ok:false} (Apps Script không đặt được mã HTTP lỗi). */
function doPost(e) {
  var kq;
  try {
    var yc = JSON.parse((e && e.postData && e.postData.contents) || '{}');
    var khoa = LockService.getScriptLock();
    if (!khoa.tryLock(2500)) return traJson({ ok: false, loi: 'ban' });
    try {
      kq = xuLy(yc, khoSheet(), new Date());
    } finally {
      khoa.releaseLock();
    }
    if (kq.ok && kq.du_lieu) {
      var dl = JSON.stringify(kq.du_lieu);
      kq = { ok: true, du_lieu: dl, chu_ky: ky(dl) };
    }
  } catch (err) {
    console.error(err);
    kq = { ok: false, loi: 'loi_tram' };
  }
  return traJson(kq);
}

/** Kiểm tra sống (bước thử quyền "Anyone"). */
function doGet() {
  return traJson({ ok: true, tram: 'dema4-license' });
}

function cauHinh(khoa) {
  var sh = bang(TAB.CAU_HINH, TIEU_DE.CAU_HINH);
  var n = sh.getLastRow() - 1;
  if (n <= 0) return '';
  var ds = sh.getRange(2, 1, n, 2).getValues();
  for (var i = 0; i < ds.length; i++) if (String(ds[i][0]).trim() === khoa) return String(ds[i][1]).trim();
  return '';
}

/** Trigger mỗi phút: gửi tối đa 15 lời gọi Telegram; gửi ngoài khoá, đánh dấu theo mã dòng. */
function guiHangDoi() {
  var token = cauHinh('telegram_token'), chat = cauHinh('telegram_chat_id');
  if (!token || !chat) return;
  var sh = bang(TAB.HANG_DOI, TIEU_DE.HANG_DOI);
  var khoa = LockService.getScriptLock();
  if (!khoa.tryLock(5000)) return;
  var chon;
  try {
    var n = sh.getLastRow() - 1;
    if (n <= 0) return;
    var vung = sh.getRange(2, 1, n, 8).getValues();
    var ds = vung.map(function (r, i) { return { hang: i + 2, ma: r[0], noi_dung: String(r[2]).replace(/^'/, ''), ten_csv: r[3], csv: String(r[4]).replace(/^'/, ''), so_lan_thu: Number(r[5]) || 0, trang_thai: r[6], luc: Number(r[7]) || 0 }; });
    chon = chonGui(ds, Date.now(), 15);
    chon.traVe.forEach(function (r) { sh.getRange(r.hang, 7, 1, 2).setValues([['cho', 0]]); });
    chon.gui.forEach(function (r) { sh.getRange(r.hang, 7, 1, 2).setValues([['dang_gui', Date.now()]]); });
  } finally {
    khoa.releaseLock();
  }
  var ketQua = {}; // ma → 'da_gui' | 'thu_lai' | 'loi'
  for (var i = 0; i < chon.gui.length; i++) {
    var r = chon.gui[i];
    var ma = guiTelegram(token, chat, r);
    if (ma === 429) { ketQua[r.ma] = 'cho_lai'; for (var j = i + 1; j < chon.gui.length; j++) ketQua[chon.gui[j].ma] = 'cho_lai'; break; }
    ketQua[r.ma] = ma === 200 ? 'da_gui' : (r.so_lan_thu + 1 >= 5 ? 'loi' : 'thu_lai');
  }
  if (!khoa.tryLock(5000)) return; // dòng "đang gửi" sẽ tự về "chờ" sau 5 phút
  try {
    var m = sh.getLastRow() - 1;
    var hien = sh.getRange(2, 1, m, 8).getValues();
    hien.forEach(function (row, idx) {
      var kq = ketQua[row[0]];
      if (!kq) return;
      var hang = idx + 2;
      if (kq === 'da_gui') sh.getRange(hang, 7, 1, 2).setValues([['da_gui', Date.now()]]);
      else if (kq === 'cho_lai') sh.getRange(hang, 7, 1, 2).setValues([['cho', 0]]);
      else sh.getRange(hang, 6, 1, 3).setValues([[Number(row[5]) + 1, kq === 'loi' ? 'loi' : 'cho', 0]]);
    });
    donHangDoi(sh, hien);
  } finally {
    khoa.releaseLock();
  }
}

/** Xoá dòng "đã gửi" cũ hơn 7 ngày (từ dưới lên để chỉ số dòng không lệch). */
function donHangDoi(sh, hien) {
  var han = Date.now() - 7 * 24 * 3600 * 1000;
  for (var i = hien.length - 1; i >= 0; i--) if (hien[i][6] === 'da_gui' && Number(hien[i][7]) < han) sh.deleteRow(i + 2);
}

function guiTelegram(token, chat, r) {
  var goc = 'https://api.telegram.org/bot' + token + '/';
  var tin = UrlFetchApp.fetch(goc + 'sendMessage', { method: 'post', muteHttpExceptions: true, payload: { chat_id: chat, text: r.noi_dung, parse_mode: 'HTML', disable_web_page_preview: 'true' } });
  if (tin.getResponseCode() !== 200 || !r.ten_csv) return tin.getResponseCode();
  var tep = UrlFetchApp.fetch(goc + 'sendDocument', { method: 'post', muteHttpExceptions: true, payload: { chat_id: chat, document: Utilities.newBlob(r.csv, 'text/csv', r.ten_csv) } });
  return tep.getResponseCode();
}

/** Chạy 1 lần sau khi deploy: trigger guiHangDoi mỗi phút (không tạo trùng). */
function caiTrigger() {
  ScriptApp.getProjectTriggers().forEach(function (t) { if (t.getHandlerFunction() === 'guiHangDoi') ScriptApp.deleteTrigger(t); });
  ScriptApp.newTrigger('guiHangDoi').timeBased().everyMinutes(1).create();
}

/** Kiểm khoá dán vào Script Properties: ký 1 chuỗi mẫu, in chữ ký để đối chiếu với khoá công khai (test Rust). */
function tuKiemKhoa() {
  var mau = '{"kiem_tra":"Khoá dema4 ✓"}';
  var ck = ky(mau);
  console.log('du_lieu=' + mau);
  console.log('chu_ky=' + ck);
  return ck;
}
