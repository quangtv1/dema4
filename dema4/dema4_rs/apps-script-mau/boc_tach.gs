/**
 * Script mẫu: ghi kết quả Bóc tách dữ liệu (dema4) vào Google Sheet của đơn vị.
 *
 * Cách dùng:
 *  1. Mở Google Sheet › Tiện ích mở rộng › Apps Script, xoá mã sẵn có, dán toàn bộ file này, Lưu.
 *  2. Triển khai › Triển khai mới › Loại: Ứng dụng web; Thực thi với tư cách: Tôi; Ai có quyền truy cập: Bất kỳ ai.
 *  3. Chép URL kết thúc bằng /exec vào dema4 › Cấu hình › Bóc tách › Ghi kết quả › URL Web app.
 * 4. Đặt BI_MAT bên dưới thành 1 chuỗi ngẫu nhiên, nhập đúng chuỗi đó vào ô "Mã bí mật" trong dema4
 *    (ai có URL mà không có mã thì không ghi được). dema4 lưu URL và mã đã mã hoá.
 *
 * dema4 gửi: { tab, tieu_de: [..], dong: [[..], ..], ghi_lai, bi_mat }
 *  ghi_lai = true: tab = bản sao kết quả (giống file Excel) — ghi cả bảng từ dòng 2, xoá dòng thừa phía dưới.
 *    dema4 chỉ gửi khi dữ liệu có thay đổi, tối đa 1 lần / phút.
 *  ghi_lai = false ("Ghi thử 1 dòng"): thêm dòng cuối tab.
 * Trả về { ok: true } hoặc { ok: false, loi: "..." }.
 */
var BI_MAT = ''; // ĐẶT GIÁ TRỊ: chuỗi ngẫu nhiên, trùng ô "Mã bí mật" trong dema4

function doPost(e) {
  // Khoá: nhiều máy cùng ghi 1 Sheet không đè dòng của nhau
  var khoaGhi = LockService.getScriptLock();
  try {
    var yc = JSON.parse(e.postData.contents);
    if (BI_MAT && yc.bi_mat !== BI_MAT) return traLoi({ ok: false, loi: 'Sai mã bí mật' });
    khoaGhi.waitLock(30000);
    var ss = SpreadsheetApp.getActiveSpreadsheet();
    var sh = ss.getSheetByName(yc.tab) || ss.insertSheet(yc.tab);
    if (sh.getLastRow() === 0) {
      ghiDong(sh, 1, yc.tieu_de);
      dinhDangTab(sh, yc.tieu_de);
    }
    if (yc.ghi_lai) {
      ghiDong(sh, 1, yc.tieu_de);
      if (yc.dong.length) ghiBang(sh, 2, yc.dong);
      var thua = sh.getLastRow() - yc.dong.length - 1;
      if (thua > 0) sh.getRange(yc.dong.length + 2, 1, thua, sh.getMaxColumns()).clearContent();
    } else if (yc.dong.length) {
      ghiBang(sh, sh.getLastRow() + 1, yc.dong);
    }
    return traLoi({ ok: true });
  } catch (err) {
    return traLoi({ ok: false, loi: String(err) });
  } finally {
    if (khoaGhi.hasLock()) khoaGhi.releaseLock();
  }
}

/** Ghi 1 dòng dạng chữ (@): Sheet không tự đổi số ký hiệu / ngày thành số. */
function ghiDong(sh, hang, d) {
  ghiBang(sh, hang, [d]);
}

/** Ghi nhiều dòng trong 1 lần (setValues 1 khối), dạng chữ (@). */
function ghiBang(sh, hang, ds) {
  var o = sh.getRange(hang, 1, ds.length, ds[0].length);
  o.setNumberFormat('@');
  o.setValues(ds);
}

/**
 * Tab mới: giống file Excel của dema4 — mọi ô xuống dòng + căn giữa dọc, tiêu đề đậm nền xám cố định,
 * cột Trích yếu / Nội dung rộng nhất, bộ lọc trên toàn bảng.
 */
function dinhDangTab(sh, tieuDe) {
  var n = tieuDe.length;
  sh.getRange(1, 1, sh.getMaxRows(), n).setWrap(true).setVerticalAlignment('middle');
  sh.getRange(1, 1, 1, n).setFontWeight('bold').setBackground('#f3f3f3');
  sh.setFrozenRows(1);
  var rong = { 'trích yếu': 480, 'nội dung': 480, 'tên file': 220, 'lỗi bóc tách': 280, 'hồ sơ': 110, 'số trang': 70 };
  tieuDe.forEach(function (t, i) { sh.setColumnWidth(i + 1, rong[String(t).toLowerCase()] || 160); });
  if (!sh.getFilter()) sh.getRange(1, 1, sh.getMaxRows(), n).createFilter();
}

function traLoi(o) {
  return ContentService.createTextOutput(JSON.stringify(o)).setMimeType(ContentService.MimeType.JSON);
}
