/**
 * Nhận thống kê ngưỡng trắng từ sm-dema4 (mỗi lần Tách gửi 1 dòng JSON) và ghi thêm 1 hàng vào sheet "nguong".
 *
 * Cài đặt:
 *  1. Tạo Google Sheet → Tiện ích mở rộng → Apps Script → dán toàn bộ file này.
 *  2. Triển khai → Triển khai mới → Loại "Ứng dụng web": Thực thi với tư cách "Tôi", Ai có quyền truy cập "Bất kỳ ai".
 *  3. Chép URL web app (…/exec) vào dòng `webhook=<URL>` trong telegram.txt cạnh file chạy
 *     (hoặc nhúng vào hằng TACH_WEBHOOK_NHUNG trong telegram.rs). Giữ URL bí mật như token Telegram.
 *
 * Dữ liệu nhận: chỉ số đếm (không ảnh, không tên file PDF, không nội dung trang).
 */
var COT = [
  'thoi_diem', 'may', 'thu_muc', 'phien_ban', 't1', 't2',
  'trang_tu_dong', 'trang_tu_dong_sua', 'nghi', 'nghi_thanh_trang', 'nghi_giu', 'trang_lot',
  'de_xuat_t1', 'de_xuat_t2', 'xoay_xac_nhan', 'xoay_chinh', 'xoay_0', 'histogram'
];

function doPost(e) {
  var lock = LockService.getScriptLock();
  lock.waitLock(10000);
  try {
    var d = JSON.parse(e.postData.contents);
    var sheet = SpreadsheetApp.getActive().getSheetByName('nguong') || SpreadsheetApp.getActive().insertSheet('nguong');
    if (sheet.getLastRow() === 0) sheet.appendRow(COT);
    sheet.appendRow(COT.map(function (k) { return d[k] === undefined ? '' : d[k]; }));
    return ContentService.createTextOutput('ok');
  } finally {
    lock.releaseLock();
  }
}
