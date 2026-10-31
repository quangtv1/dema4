/**
 * Script mẫu: ghi kết quả Bóc tách dữ liệu (dema4) vào Google Sheet của đơn vị.
 *
 * Cách dùng:
 *  1. Mở Google Sheet › Tiện ích mở rộng › Apps Script, xoá mã sẵn có, dán toàn bộ file này, Lưu.
 *  2. Triển khai › Triển khai mới › Loại: Ứng dụng web; Thực thi với tư cách: Tôi; Ai có quyền truy cập: Bất kỳ ai.
 *  3. Chép URL kết thúc bằng /exec vào dema4 › Cấu hình › Bóc tách › Ghi kết quả › URL Web app.
 * URL này ghi được vào Sheet: không gửi cho người ngoài (dema4 lưu URL đã mã hoá).
 *
 * dema4 gửi: { tab, tieu_de: [..], dong: [[..], ..], khoa: null | "<Tên file>" }
 *  - khoa = null: thêm các dòng vào cuối tab;
 *  - khoa = tên file (sửa tay): ghi đè dòng có cột A trùng tên file, chưa có thì thêm.
 * Trả về { ok: true } hoặc { ok: false, loi: "..." }.
 */
function doPost(e) {
  try {
    var yc = JSON.parse(e.postData.contents);
    var ss = SpreadsheetApp.getActiveSpreadsheet();
    var sh = ss.getSheetByName(yc.tab) || ss.insertSheet(yc.tab);
    if (sh.getLastRow() === 0) ghiDong(sh, 1, yc.tieu_de);
    yc.dong.forEach(function (d) {
      var hang = yc.khoa ? timHang(sh, yc.khoa) : 0;
      ghiDong(sh, hang || sh.getLastRow() + 1, d);
    });
    return traLoi({ ok: true });
  } catch (err) {
    return traLoi({ ok: false, loi: String(err) });
  }
}

/** Ghi 1 dòng dạng chữ (@): Sheet không tự đổi số ký hiệu / ngày thành số. */
function ghiDong(sh, hang, d) {
  var o = sh.getRange(hang, 1, 1, d.length);
  o.setNumberFormat('@');
  o.setValues([d]);
}

/** Dòng cuối cùng có cột A = khoa (0 = không có). */
function timHang(sh, khoa) {
  var n = sh.getLastRow();
  if (n === 0) return 0;
  var v = sh.getRange(1, 1, n, 1).getValues();
  for (var i = v.length - 1; i >= 0; i--) if (v[i][0] === khoa) return i + 1;
  return 0;
}

function traLoi(o) {
  return ContentService.createTextOutput(JSON.stringify(o)).setMimeType(ContentService.MimeType.JSON);
}
