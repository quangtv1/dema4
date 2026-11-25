// Trạm license dema4 — lớp mỏng nối logic thuần (loi.gs = loi.js) với Google Sheet / Cache / Lock / UrlFetch.
// Triển khai: xem README.md cùng thư mục. Bí mật chỉ ở Script Properties: KHOA_BI_MAT (khoá RSA PKCS#8 PEM),
// TELEGRAM_TOKEN, TELEGRAM_CHAT_ID.

var TAB = {
  MAY: '1_license',
  NGUONG: '2_nguong_trang',
  HANG_DOI: '3_hang_doi_telegram',
};
var TIEU_DE = {
  MAY: ['Mã máy', 'Tên máy', 'MAC', 'IP nội bộ', 'IP public', 'Phiên bản', 'Ngày đăng ký', 'Lần dùng cuối', 'Đã dùng', 'Hạn mức', 'Trạng thái', 'Đơn vị', 'Ghi chú', 'Mã lần gần đây'],
  HANG_DOI: ['Mã', 'Thời điểm', 'Nội dung', 'Tên CSV', 'CSV', 'Số lần thử', 'Trạng thái', 'Lúc đánh dấu'],
};
/** Ghi chú (note) ô tiêu đề — thay cho tab mô tả. Ô A1 tab license kèm hướng dẫn cấp lượt. */
var GHI_CHU = {
  MAY: [
    'Mã máy: băm từ ID hệ điều hành (không đổi khi đổi tên máy / card mạng). Khách đọc mã trên popup "Liên hệ Sao Mai".\n\n' +
      'HẠN MỨC DÙNG CHUNG: mỗi lần Đếm, Tách hay Sinh mục lục đều tính 1 lượt vào cột Đã dùng.\n' +
      'CẤP THÊM LƯỢT: Ctrl/Cmd+F tìm mã máy → sửa Hạn mức thành TỔNG mới (vd đã dùng 37, cho thêm 100 → 137) → Trạng thái = Đã cấp → khách bấm "Kiểm tra lại".\n' +
      'KHOÁ: Trạng thái = Khoá (máy chạy tối đa thêm 1 lần rồi bị chặn). Mở khoá: chọn lại Đã cấp / Dùng thử.\n' +
      'MÁY NỘI BỘ: Hạn mức 100000 + Đã cấp.\n' +
      'KHÔNG: xoá dòng máy (đăng ký lại từ 0), sửa cột Đã dùng / Mã lần, gõ trạng thái ngoài danh sách.\n\n' +
      'Tiêu đề vàng = cột Sao Mai sửa; xanh = trạm tự ghi.',
    'Tên máy tính ở lần liên lạc gần nhất (trạm ghi).',
    'Địa chỉ MAC card mạng chính (trạm ghi).',
    'IP trong mạng LAN ở lần liên lạc gần nhất (trạm ghi).',
    'IP ra Internet ở lần liên lạc gần nhất (trạm ghi).',
    'Phiên bản app ở lần liên lạc gần nhất (trạm ghi).',
    'Lần đầu máy gọi trạm (trạm ghi).',
    'Lần gần nhất máy gọi trạm (trạm ghi).',
    'Tổng số lần đã dùng mọi chức năng (Đếm, Tách, Sinh mục lục); +1 mỗi lần chạy có kết quả. Chữ đỏ = hết lượt. Không sửa tay.',
    'TỔNG số lần được phép, dùng chung mọi chức năng (không phải số cộng thêm). Trống = 10 (dùng thử). Sao Mai sửa.',
    'Dùng thử / Đã cấp / Khoá — chọn trong danh sách. Giá trị khác bị coi là Khoá. Sao Mai sửa.',
    'Tự do: đơn vị sử dụng. Sao Mai sửa.',
    'Tự do: số hợp đồng, ngày cấp… Sao Mai sửa.',
    '20 mã lần chạy gần nhất — trạm dùng để không cộng trùng khi app gửi lại. Cột ẩn, không sửa tay.',
  ],
  HANG_DOI: [
    'Mã dòng (trạm tạo). Tab ẩn: tin Telegram chờ gửi, trigger gửi mỗi phút (tối đa 15 lời gọi/lượt), tự dọn sau 7 ngày. ' +
      'Token + id nhóm ở Apps Script › Project Settings › Script Properties: TELEGRAM_TOKEN, TELEGRAM_CHAT_ID.',
    'Lúc xếp tin.', 'Nội dung tin (chữ thường).', 'Tên file CSV gửi kèm (lần Tách).', 'Nội dung CSV histogram ngưỡng trắng.',
    'Số lần gửi lỗi (5 lần → loi).', 'cho · dang_gui · da_gui · loi', 'Lúc đánh dấu đang gửi (ms).',
  ],
};
/** Cột lưu dạng chữ (không để Sheets đổi "0.6.0" thành ngày, mã hex thành số). */
var COT_CHU = ['Mã máy', 'Phiên bản', 'Mã lần gần đây', 'Mã', 'MAC', 'Địa chỉ MAC', 'IP nội bộ', 'IP public'];

function bang(ten, tieuDe, ghiChu) {
  var ss = SpreadsheetApp.getActiveSpreadsheet();
  var sh = ss.getSheetByName(ten);
  if (!sh) {
    sh = ss.insertSheet(ten, ss.getNumSheets());
    if (tieuDe && tieuDe.length) {
      sh.getRange(1, 1, 1, tieuDe.length).setValues([tieuDe]).setFontWeight('bold');
      sh.setFrozenRows(1);
      tieuDe.forEach(function (t, i) { if (COT_CHU.indexOf(t) >= 0) sh.getRange(1, i + 1, sh.getMaxRows(), 1).setNumberFormat('@'); });
      if (ghiChu && ghiChu.length === tieuDe.length) sh.getRange(1, 1, 1, tieuDe.length).setNotes([ghiChu]);
    }
    if (ten === TAB.MAY) dinhDangBangMay(sh);
    if (ten === TAB.HANG_DOI) sh.hideSheet();
  }
  return sh;
}

/** Trình bày tab license: danh sách Trạng thái, màu tiêu đề (xanh = trạm ghi, vàng = Sao Mai sửa), ẩn cột Mã lần,
 *  chữ đỏ ô Đã dùng khi hết lượt. */
function dinhDangBangMay(sh) {
  var quyTac = SpreadsheetApp.newDataValidation().requireValueInList(['Dùng thử', 'Đã cấp', 'Khoá'], true).setAllowInvalid(false).build();
  sh.getRange(2, COT.TRANG_THAI, sh.getMaxRows() - 1, 1).setDataValidation(quyTac);
  sh.getRange(1, 1, 1, COT.DUNG).setBackground('#1F4E79').setFontColor('#FFFFFF');
  sh.getRange(1, COT.HAN, 1, COT.GHI_CHU - COT.HAN + 1).setBackground('#BF8F00').setFontColor('#FFFFFF');
  sh.getRange(1, COT.MA_LAN).setBackground('#1F4E79').setFontColor('#FFFFFF');
  sh.hideColumns(COT.MA_LAN);
  var vung = sh.getRange(2, COT.DUNG, sh.getMaxRows() - 1, 1);
  var het = SpreadsheetApp.newConditionalFormatRule().whenFormulaSatisfied('=AND($A2<>"",$I2>=IF($J2="",' + HAN_MAC_DINH + ',$J2))').setFontColor('#C00000').setBold(true).setRanges([vung]).build();
  sh.setConditionalFormatRules([het]);
}

/**
 * Bảng license cũ (cột riêng từng chức năng) → bảng hạn mức chung 14 cột. Chạy 1 lần trong khoá trạm ở lời gọi
 * đầu tiên sau khi deploy: sao tab cũ thành "1_license_cu_<ngày>" (ẩn), cộng đã dùng / hạn mức của 3 chức năng.
 */
function chuyenBangCu(sh) {
  if (String(sh.getRange(1, 10).getValue()).trim() !== TIEU_DE_COT_J_CU) return;
  var ss = sh.getParent();
  ss.insertSheet(TAB.MAY + '_cu_' + thoiDiem(new Date()).slice(0, 10), ss.getNumSheets(), { template: sh }).hideSheet();
  var n = sh.getLastRow() - 1;
  var cot = Math.max(sh.getLastColumn(), 18);
  var moi = n > 0 ? sh.getRange(2, 1, n, cot).getValues().filter(function (r) { return String(r[0]).trim() !== ''; }).map(chuyenDongCu) : [];
  sh.getRange(1, 1, sh.getMaxRows(), sh.getMaxColumns()).clear().clearNote().clearDataValidations();
  sh.setConditionalFormatRules([]);
  sh.showColumns(1, sh.getMaxColumns());
  if (sh.getMaxColumns() > SO_COT_MAY) sh.deleteColumns(SO_COT_MAY + 1, sh.getMaxColumns() - SO_COT_MAY);
  TIEU_DE.MAY.forEach(function (t, i) { if (COT_CHU.indexOf(t) >= 0) sh.getRange(1, i + 1, sh.getMaxRows(), 1).setNumberFormat('@'); });
  sh.getRange(1, 1, 1, SO_COT_MAY).setValues([TIEU_DE.MAY]).setFontWeight('bold').setNotes([GHI_CHU.MAY]);
  sh.setFrozenRows(1);
  if (moi.length) sh.getRange(2, 1, moi.length, SO_COT_MAY).setValues(moi);
  dinhDangBangMay(sh);
}

/** Ghi chú tiêu đề tab Ngưỡng trắng từ mô tả app gửi kèm: [Cột, Ý nghĩa, Cách đọc]. */
function ghiChuTuMoTa(tieuDe, moTa) {
  return tieuDe.map(function (t) {
    var r = (moTa || []).filter(function (x) { return x[0] === t; })[0];
    return r ? [r[1], r[2]].filter(Boolean).join('\n') : '';
  });
}

function themDong(sh, rows) {
  if (!rows.length) return;
  // Hết dòng trống thì nới lưới (getRange ngoài lưới báo lỗi); dòng mới chép định dạng dòng trên (cột chữ giữ dạng chữ)
  var thieu = sh.getLastRow() + rows.length - sh.getMaxRows();
  if (thieu > 0) sh.insertRowsAfter(sh.getMaxRows(), thieu + 500);
  sh.getRange(sh.getLastRow() + 1, 1, rows.length, rows[0].length).setValues(rows);
}

function khoSheet() {
  var cache = CacheService.getScriptCache();
  var may = bang(TAB.MAY, TIEU_DE.MAY, GHI_CHU.MAY);
  chuyenBangCu(may);
  var hangDoi = bang(TAB.HANG_DOI, TIEU_DE.HANG_DOI, GHI_CHU.HANG_DOI);
  return {
    timMay: function (ma) {
      var o = may.getRange('A:A').createTextFinder(ma).matchEntireCell(true).findNext();
      if (!o) return null;
      return { hang: o.getRow(), giaTri: may.getRange(o.getRow(), 1, 1, SO_COT_MAY).getValues()[0] };
    },
    themMay: function (r) { themDong(may, [r]); },
    capNhatMay: function (hang, cot) { Object.keys(cot).forEach(function (c) { may.getRange(hang, Number(c)).setValue(cot[c]); }); },
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
    themNguong: function (tieuDe, moTa, dong) { themDong(bang(TAB.NGUONG, tieuDe, ghiChuTuMoTa(tieuDe, moTa)), [dong]); },
  };
}

// ---- Sao lưu kết quả Bóc tách lên Drive ----
// Cây thư mục: <thư mục chứa Sheet trạm>/Bóc tách dữ liệu/<mã máy>/<tên thư mục nguồn>_<băm>  (Google Sheet)
var THU_MUC_SAO_LUU = 'Bóc tách dữ liệu';
var RONG_COT_SAO_LUU = { 'trích yếu': 480, 'nội dung': 480, 'tên file': 220, 'lỗi bóc tách': 280, 'hồ sơ': 110, 'số trang': 70 };

/** Thư mục con `ten` trong `cha` (chưa có thì tạo). */
function thuMucCon(cha, ten) {
  var it = cha.getFoldersByName(ten);
  return it.hasNext() ? it.next() : cha.createFolder(ten);
}

/** Thư mục "Bóc tách dữ liệu" cùng cấp Sheet trạm; id nhớ ở Script Properties, tạo trong khoá (không tạo trùng). */
function thuMucSaoLuu() {
  var pr = PropertiesService.getScriptProperties();
  var id = pr.getProperty('THU_MUC_SAO_LUU');
  if (id) {
    try {
      var cu = DriveApp.getFolderById(id);
      if (!cu.isTrashed()) return cu;
    } catch (e) { /* bị xoá hẳn → tạo lại */ }
  }
  var khoa = LockService.getScriptLock();
  khoa.waitLock(20000);
  try {
    var cha = DriveApp.getFileById(SpreadsheetApp.getActiveSpreadsheet().getId()).getParents().next();
    var tm = thuMucCon(cha, THU_MUC_SAO_LUU);
    pr.setProperty('THU_MUC_SAO_LUU', tm.getId());
    return tm;
  } finally {
    khoa.releaseLock();
  }
}

/** Ghi lại cả file sao lưu: tab "Kết quả" (giống Excel của app) + tab "Thông tin" (máy, thư mục, loại, mô hình…). */
function ghiSaoLuu(maMay, tenFile, tieuDe, dong, thongTin) {
  var tm = thuMucCon(thuMucSaoLuu(), maMay);
  var it = tm.getFilesByName(tenFile);
  var ss;
  if (it.hasNext()) {
    ss = SpreadsheetApp.openById(it.next().getId());
  } else {
    ss = SpreadsheetApp.create(tenFile);
    DriveApp.getFileById(ss.getId()).moveTo(tm);
  }
  var kq = ss.getSheetByName('Kết quả') || ss.getSheets()[0].setName('Kết quả');
  if (kq.getFilter()) kq.getFilter().remove();
  kq.clear();
  var bang = [tieuDe].concat(dong);
  var n = tieuDe.length;
  if (kq.getMaxColumns() < n) kq.insertColumnsAfter(kq.getMaxColumns(), n - kq.getMaxColumns());
  if (kq.getMaxRows() < bang.length + 1) kq.insertRowsAfter(kq.getMaxRows(), bang.length + 1 - kq.getMaxRows());
  var vung = kq.getRange(1, 1, bang.length, n);
  vung.setNumberFormat('@').setValues(bang).setWrap(true).setVerticalAlignment('middle');
  kq.getRange(1, 1, 1, n).setFontWeight('bold').setBackground('#f3f3f3');
  kq.setFrozenRows(1);
  tieuDe.forEach(function (t, i) { kq.setColumnWidth(i + 1, RONG_COT_SAO_LUU[String(t).toLowerCase()] || 160); });
  vung.createFilter();
  var tt = ss.getSheetByName('Thông tin') || ss.insertSheet('Thông tin');
  tt.clear();
  tt.getRange(1, 1, thongTin.length, 2).setNumberFormat('@').setValues(thongTin.map(function (r) { return [r[0], String(r[1])]; }));
  tt.getRange(1, 1, thongTin.length, 1).setFontWeight('bold');
  tt.setColumnWidth(1, 140).setColumnWidth(2, 480);
}

/** Kho cho lời gọi sao lưu: chạy NGOÀI khoá trạm (ghi file mất vài giây, không chặn kiểm tra license của máy khác). */
function khoSaoLuu() {
  var cache = CacheService.getScriptCache();
  return {
    choPhep: function (khoa, gioiHan, giay) {
      var k = 'dem_' + khoa + '_' + Math.floor(Date.now() / 1000 / giay);
      var n = Number(cache.get(k) || 0) + 1;
      cache.put(k, String(n), giay + 60);
      return n <= gioiHan;
    },
    saoLuu: ghiSaoLuu,
  };
}

/** Chạy 1 lần trong trình soạn Apps Script sau khi đẩy mã: cấp quyền Drive + tạo thư mục "Bóc tách dữ liệu". */
function caiSaoLuu() {
  var tm = thuMucSaoLuu();
  console.log('Thư mục sao lưu: ' + tm.getName() + ' — ' + tm.getUrl());
  return tm.getUrl();
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
    if (yc.hanh_dong === 'sao_luu_boc_tach') return traJson(xuLy(yc, khoSaoLuu(), new Date()));
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

/** Cấu hình Telegram ở Script Properties (không nằm trên Sheet): TELEGRAM_TOKEN, TELEGRAM_CHAT_ID. */
function cauHinh(khoa) {
  return (PropertiesService.getScriptProperties().getProperty(khoa) || '').trim();
}

/** Trigger mỗi phút: gửi tối đa 15 lời gọi Telegram; gửi ngoài khoá, đánh dấu theo mã dòng. */
function guiHangDoi() {
  var token = cauHinh('TELEGRAM_TOKEN'), chat = cauHinh('TELEGRAM_CHAT_ID');
  if (!token || !chat) return;
  var sh = bang(TAB.HANG_DOI, TIEU_DE.HANG_DOI, GHI_CHU.HANG_DOI);
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
  var tin = UrlFetchApp.fetch(goc + 'sendMessage', { method: 'post', muteHttpExceptions: true, payload: { chat_id: chat, text: r.noi_dung, disable_web_page_preview: 'true' } });
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
