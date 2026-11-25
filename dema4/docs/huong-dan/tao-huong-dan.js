// Tạo "Hướng dẫn sử dụng dema4-saomai" (docx) theo bố cục sách hướng dẫn kiểu Nhật (取扱説明書):
// bìa · lời nói đầu · ký hiệu · mục lục · từng chương (sơ đồ đánh số, bước thao tác, kết quả) · xử lý sự cố · phụ lục · lịch sử sửa đổi.
// Chạy: cd docs/huong-dan && npm i docx@9 (một lần, thư mục tạm) && node tao-huong-dan.js
// Ảnh trong anh/ chụp từ app bằng các test `chup_*` (dữ liệu mẫu trung tính), số đỏ = chú thích trong bảng ngay dưới ảnh.
const fs = require('fs');
const path = require('path');
const {
  Document, Packer, Paragraph, TextRun, ImageRun, Table, TableRow, TableCell, WidthType, ShadingType, BorderStyle,
  AlignmentType, HeadingLevel, PageBreak, Header, Footer, PageNumber, TableOfContents, LevelFormat, TabStopType,
} = require('docx');

const PHIEN_BAN = '2.2.0';
const NGAY = '02/11/2026';
// HD_DIR: chạy bản chép ở thư mục khác (nơi đã cài docx) mà vẫn đọc ảnh / ghi file ở docs/huong-dan
const GOC = process.env.HD_DIR || __dirname;
const ANH = path.join(GOC, 'anh');
const RONG_ND = 9026; // A4, lề 1" → bề rộng nội dung (DXA)
const FONT = 'Arial';
const MAU = { chinh: '1F4E79', phu: '5F6B7A', vien: 'BFC8D4', nen: 'F2F5F9' };

// ---- Khối dựng ----
// Chữ có **đậm** và `mã`
function chu(s, o = {}) {
  const ra = [];
  s.split(/(\*\*[^*]+\*\*|`[^`]+`)/).forEach((p) => {
    if (!p) return;
    if (p.startsWith('**')) ra.push(new TextRun({ text: p.slice(2, -2), bold: true, ...o }));
    else if (p.startsWith('`')) ra.push(new TextRun({ text: p.slice(1, -1), font: 'Consolas', ...o }));
    else ra.push(new TextRun({ text: p, ...o }));
  });
  return ra;
}
const P = (s, o = {}) => new Paragraph({ children: chu(s, o.run), spacing: { after: 100, line: 300 }, ...o.p });
const H1 = (s) => new Paragraph({ heading: HeadingLevel.HEADING_1, pageBreakBefore: true, children: [new TextRun(s)] });
const H2 = (s) => new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun(s)] });
const H3 = (s) => new Paragraph({ heading: HeadingLevel.HEADING_3, children: [new TextRun(s)] });
const dau = (s) => new Paragraph({ numbering: { reference: 'cham', level: 0 }, children: chu(s), spacing: { after: 60, line: 290 } });

// Hình: số theo chương "Hình 4-2"
let chuong = 0, soHinh = 0;
function batDauChuong(ten) { chuong += 1; soHinh = 0; return H1(`Chương ${chuong}. ${ten}`); }
function hinh(file, chuThich, rongPx = 600) {
  const f = path.join(ANH, file);
  const buf = fs.readFileSync(f);
  const w = buf.readUInt32BE(16), h = buf.readUInt32BE(20); // PNG IHDR
  const cao = Math.round(rongPx * h / w);
  soHinh += 1;
  return [
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 120, after: 40 }, children: [new ImageRun({ type: 'png', data: buf, transformation: { width: rongPx, height: cao }, altText: { title: chuThich, description: chuThich, name: file } })] }),
    new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [new TextRun({ text: `Hình ${chuong}-${soHinh}. ${chuThich}`, italics: true, size: 18, color: MAU.phu })] }),
  ];
}

const vien = (c = MAU.vien) => ({ style: BorderStyle.SINGLE, size: 4, color: c });
const VIEN_O = { top: vien(), bottom: vien(), left: vien(), right: vien() };
function o(noiDung, rong, o2 = {}) {
  const ds = (Array.isArray(noiDung) ? noiDung : [noiDung]).map((s) => new Paragraph({ children: chu(s, o2.run), spacing: { after: 40, line: 270 }, alignment: o2.canh }));
  return new TableCell({ borders: VIEN_O, width: { size: rong, type: WidthType.DXA }, margins: { top: 70, bottom: 70, left: 110, right: 110 }, shading: o2.nen ? { fill: o2.nen, type: ShadingType.CLEAR, color: 'auto' } : undefined, children: ds });
}
// Bảng: hàng đầu là tiêu đề (nền xanh đậm, chữ trắng)
function bang(tieuDe, hang, rong) {
  const tong = rong.reduce((a, b) => a + b, 0);
  return [
    new Table({
      width: { size: tong, type: WidthType.DXA }, columnWidths: rong,
      rows: [
        new TableRow({ tableHeader: true, children: tieuDe.map((t, i) => o(t, rong[i], { nen: MAU.chinh, run: { bold: true, color: 'FFFFFF' } })) }),
        ...hang.map((r, j) => new TableRow({ cantSplit: true, children: r.map((t, i) => o(t, rong[i], { nen: j % 2 ? MAU.nen : undefined })) })),
      ],
    }),
    new Paragraph({ spacing: { after: 120 }, children: [] }),
  ];
}
// Bảng chú thích ảnh có số: [số, tên, mô tả]
const chuThichSo = (ds) => bang(['Số', 'Tên', 'Chức năng'], ds.map((d, i) => [`${i + 1}`, `**${d[0]}**`, d[1]]), [700, 2400, 5926]);

// Hộp ký hiệu kiểu sách Nhật: CHÚ Ý (đỏ) · LƯU Ý (cam) · MẸO (xanh lá) · THAM KHẢO (xanh dương)
const LOAI = { chuy: ['CHÚ Ý', 'C62828', 'FDECEA'], luuy: ['LƯU Ý', 'B26A00', 'FFF4E0'], meo: ['MẸO', '2E7D32', 'EAF5EA'], tk: ['THAM KHẢO', '1565C0', 'E8F1FB'] };
function hop(loai, dong) {
  const [nhan, mau, nen] = LOAI[loai];
  const bien = { top: vien(mau), bottom: vien(mau), right: vien(mau), left: { style: BorderStyle.SINGLE, size: 24, color: mau } };
  return [
    new Table({
      width: { size: RONG_ND, type: WidthType.DXA }, columnWidths: [RONG_ND],
      rows: [new TableRow({ cantSplit: true, children: [new TableCell({ borders: bien, width: { size: RONG_ND, type: WidthType.DXA }, shading: { fill: nen, type: ShadingType.CLEAR, color: 'auto' }, margins: { top: 90, bottom: 90, left: 160, right: 160 },
        children: [new Paragraph({ spacing: { after: 40 }, children: [new TextRun({ text: nhan, bold: true, color: mau })] }), ...dong.map((s) => new Paragraph({ children: chu(s), spacing: { after: 40, line: 280 } }))] })] })],
    }),
    new Paragraph({ spacing: { after: 120 }, children: [] }),
  ];
}
// Bước thao tác: "Bước n" đậm + nội dung + (ảnh) + "→ Kết quả"
let soBuoc = 0;
const batDauThaoTac = () => { soBuoc = 0; };
function buoc(noiDung, ketQua) {
  soBuoc += 1;
  const ra = [new Paragraph({ spacing: { before: 120, after: 60, line: 300 }, children: [new TextRun({ text: `Bước ${soBuoc}  `, bold: true, color: 'FFFFFF', shading: { fill: MAU.chinh, type: ShadingType.CLEAR, color: 'auto' } }), new TextRun(' '), ...chu(noiDung)] })];
  if (ketQua) ra.push(new Paragraph({ indent: { left: 720 }, spacing: { after: 80 }, children: [new TextRun({ text: '→ Kết quả: ', bold: true, color: MAU.phu }), ...chu(ketQua, { color: MAU.phu })] }));
  return ra;
}

// ---- Nội dung ----
const nd = [];
const them = (...k) => k.forEach((x) => (Array.isArray(x) ? nd.push(...x) : nd.push(x)));

// Bìa
them(
  new Paragraph({ spacing: { before: 2600 }, alignment: AlignmentType.CENTER, children: [new TextRun({ text: 'dema4-saomai', bold: true, size: 64, color: MAU.chinh })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [new TextRun({ text: 'Ứng dụng hỗ trợ nghiệp vụ số hoá tài liệu', size: 30, color: MAU.phu })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, border: { top: vien(MAU.chinh), bottom: vien(MAU.chinh) }, spacing: { before: 400, after: 400 }, children: [new TextRun({ text: 'HƯỚNG DẪN SỬ DỤNG', bold: true, size: 48 })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: 'Đếm trang A4 · Tách văn bản · Sinh mục lục hồ sơ · Bóc tách dữ liệu', size: 24 })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 2400 }, children: [new TextRun({ text: `Phiên bản phần mềm ${PHIEN_BAN}  ·  Ngày ban hành ${NGAY}`, size: 22, color: MAU.phu })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: 'Giải pháp số Sao Mai  ·  saomaiso.com', size: 22, color: MAU.phu })] }),
);

// Lời nói đầu
them(
  new Paragraph({ heading: HeadingLevel.HEADING_1, pageBreakBefore: true, children: [new TextRun('Lời nói đầu')] }),
  P('Cảm ơn quý đơn vị đã sử dụng **dema4-saomai**. Tài liệu này hướng dẫn chi tiết cách sử dụng 4 chức năng chính của phần mềm: **Đếm trang A4**, **Tách văn bản**, **Sinh mục lục hồ sơ** và **Bóc tách dữ liệu**.'),
  P('Trước khi sử dụng, xin đọc kỹ **Chương 1. Trước khi sử dụng**, đặc biệt các mục **CHÚ Ý** về an toàn dữ liệu. Nên giữ tài liệu ở nơi dễ tra cứu khi cần.'),
  H2('Cách đọc tài liệu'),
  dau('Mỗi chương ứng với một chức năng. Đầu chương có **sơ đồ màn hình đánh số đỏ** và bảng giải thích tên gọi, chức năng từng phần.'),
  dau('Các thao tác được viết thành **Bước 1, Bước 2…** theo đúng thứ tự thực hiện; sau mỗi bước quan trọng có dòng **→ Kết quả** để đối chiếu.'),
  dau('Tên nút, menu, ô nhập trên màn hình được viết **đậm**; phím được viết như `Enter`, `Ctrl+Z`; tên file, thư mục như `<thư mục>_dem.txt`.'),
  dau('Hình minh hoạ chụp từ phần mềm với dữ liệu mẫu; màn hình thực tế có thể khác đôi chút theo phiên bản và kích thước cửa sổ.'),
  H2('Ký hiệu trong tài liệu'),
  ...bang(['Ký hiệu', 'Ý nghĩa'], [
    ['**CHÚ Ý**', 'Nội dung phải tuân thủ; nếu bỏ qua có thể mất dữ liệu, sai kết quả hoặc lộ thông tin.'],
    ['**LƯU Ý**', 'Điều kiện, giới hạn cần biết để thao tác đúng.'],
    ['**MẸO**', 'Cách làm nhanh hơn hoặc thuận tiện hơn.'],
    ['**THAM KHẢO**', 'Thông tin bổ sung, mục liên quan trong tài liệu.'],
  ], [2000, 7026]),
  ...hop('tk', ['Trong phần mềm, bấm `F1` hoặc menu **Trợ giúp › Hướng dẫn sử dụng** để xem hướng dẫn nhanh của từng chức năng.']),
);

// Mục lục
them(
  new Paragraph({ heading: HeadingLevel.HEADING_1, pageBreakBefore: true, children: [new TextRun('Mục lục')] }),
  new TableOfContents('Mục lục', { hyperlink: true, headingStyleRange: '1-2' }),
  P('(Nếu mục lục trống: trong Word bấm chuột phải vào vùng này › **Update Field** › **Update entire table**.)', { run: { italics: true, color: MAU.phu, size: 18 } }),
);

// ===== Chương 1 =====
them(batDauChuong('Trước khi sử dụng'));
them(H2('1.1 Giới thiệu phần mềm'),
  P('**dema4-saomai** là ứng dụng máy tính hỗ trợ các khâu của nghiệp vụ số hoá tài liệu lưu trữ. Phần mềm chạy trên máy người dùng, không cần cài đặt, gồm các chức năng:'),
  ...bang(['Chức năng', 'Mục đích', 'Đầu ra'], [
    ['**Đếm trang A4**', 'Đếm số trang mọi file PDF trong thư mục (cả thư mục con), phân loại khổ A0–A7 và quy đổi ra số trang A4.', '`<thư mục>_dem.txt`, `_dem_chitiet.txt`'],
    ['**Tách văn bản**', 'Tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã vạch), bỏ trang trắng, gợi ý trang bị xoay; người dùng duyệt trước khi tách.', 'Thư mục `<gốc>_vanban` chứa `<hồ sơ>.1.pdf`, `<hồ sơ>.2.pdf`…'],
    ['**Sinh mục lục hồ sơ**', 'Từ file biên mục Excel/CSV và mẫu DOCX, sinh mục lục văn bản cho từng hồ sơ.', 'Mỗi hồ sơ 1 file DOCX (mặc định `MLHS_{so_ho_so}`)'],
    ['**Bóc tách dữ liệu**', 'Đọc văn bản PDF bằng mô hình AI để lấy số ký hiệu, ngày tháng, tác giả, trích yếu, người ký…', '`<thư mục>_boc_tach.xlsx`, Google Sheet (tuỳ chọn)'],
    ['Chuẩn hoá tên · Ký văn bản · Đóng gói AIP', 'Đang phát triển.', '—'],
  ], [2300, 4426, 2300]),
);
them(H2('1.2 Môi trường hoạt động'),
  ...bang(['Hạng mục', 'Yêu cầu'], [
    ['Hệ điều hành', 'Windows 10/11 64-bit (`dema4_saomai.exe`) · macOS máy chip Apple (`dema4_saomai`)'],
    ['Cài đặt', 'Không cần — một file chạy duy nhất.'],
    ['Bộ nhớ', 'Khuyến nghị RAM 8 GB trở lên khi tách / bóc tách thư mục lớn.'],
    ['Mạng', 'Cần khi: kiểm tra lượt chạy, kiểm tra cập nhật, Bóc tách bằng dịch vụ đám mây, ghi Google Sheet. Các chức năng khác chạy được khi mất mạng (lượt dùng được ghi bù khi có mạng).'],
    ['Định dạng hỗ trợ', 'PDF (kể cả PDF scan) · Excel `.xlsx .xlsb .xls` · CSV · mẫu Word `.docx`'],
  ], [2200, 6826]),
);
them(H2('1.3 Tải về và mở lần đầu'),
  batDauThaoTac,
);
batDauThaoTac();
them(...buoc('Tải file chạy ở trang **github.com/quangtv1/dema4/releases** (bản mới nhất): Windows chọn `dema4_saomai.exe`, macOS chọn `dema4_saomai`.'),
  ...buoc('Chép file vào một thư mục cố định (ví dụ `D:\\SoHoa\\Ung_dung`). Phần mềm lưu cấu hình và dữ liệu làm việc cạnh file chạy.'),
  ...buoc('Mở file. Lần đầu, hệ điều hành có thể chặn vì file chưa ký số với Microsoft / Apple:', null),
  dau('**Windows** (SmartScreen): bấm **More info › Run anyway**.'),
  dau('**macOS**: **System Settings › Privacy & Security › Open Anyway**.'),
  ...buoc('Phần mềm mở vào **Trang chủ**.', 'Thấy các ô chức năng như Hình 2-1.'),
  ...hop('meo', ['Kiểm tra bản mới: menu **Trợ giúp › Kiểm tra cập nhật**. Nếu có bản mới, bấm **Tải bản mới** rồi thay file chạy cũ (giữ nguyên thư mục để không mất cấu hình).']),
  ...hinh('02-cap-nhat.png', 'Hộp Kiểm tra cập nhật', 420),
);
them(H2('1.4 Lượt chạy (bản quyền)'),
  P('Mỗi máy có một **mã máy** riêng và một **hạn mức lượt chạy dùng chung** cho mọi chức năng. Mỗi lần chạy có kết quả (Đếm, Tách, Sinh mục lục, Bóc tách có ít nhất 1 file đọc được) tính 1 lượt; thao tác thử (ví dụ **Thử đọc** ở Bóc tách) không tính.'),
  P('Khi hết lượt, phần mềm hiện hộp **Liên hệ Sao Mai** kèm mã máy. Gửi mã máy cho Sao Mai để được cấp thêm lượt, sau đó bấm **Kiểm tra lại**.'),
  ...hinh('01-het-luot.png', 'Hộp Liên hệ Sao Mai khi hết lượt (mã máy minh hoạ)', 460),
);
them(H2('1.5 An toàn dữ liệu'),
  ...hop('chuy', [
    'File PDF gốc và thư mục đầu vào **không bị sửa**. Kết quả luôn ghi ra thư mục / file riêng.',
    'Khi dùng **Bóc tách dữ liệu** với dịch vụ đám mây (Gemini, Claude, GPT, OpenRouter), ảnh trang văn bản được gửi tới máy chủ của hãng. **Không dùng dịch vụ đám mây cho tài liệu mật** — hãy dùng máy chủ nội bộ.',
    'Kết quả Bóc tách (kể cả phần sửa tay) được gửi về trạm Sao Mai để sao lưu và nâng chất lượng bóc tách. Thông tin thống kê lượt chạy (tên máy, thư mục, số lượng) cũng được ghi nhận.',
  ]),
  ...hop('luuy', ['Nhãn, cache, log của Tách văn bản và kết quả Sinh mục lục nằm trong thư mục ẩn `.sm-dema4` cạnh file chạy. Không xoá thư mục này khi đang xử lý dở.']),
);

// ===== Chương 2 =====
them(batDauChuong('Màn hình chính và thao tác chung'));
them(H2('2.1 Tên gọi các phần của Trang chủ'), ...hinh('01-trang-chu-chu-thich.png', 'Trang chủ'),
  ...chuThichSo([
    ['Chức năng', 'Menu chuyển giữa các chức năng và về Trang chủ.'],
    ['Cấu hình', 'Mở hộp Cấu hình (4 tab: Đếm trang A4 / Tách PDF / Sinh mục lục / Bóc tách).'],
    ['Trợ giúp', 'Hướng dẫn sử dụng (`F1`), Kiểm tra cập nhật, Thông tin phần mềm.'],
    ['Logo Sao Mai', 'Logo động; bấm để mở website saomaiso.com.'],
    ['Ô chức năng', 'Bấm để mở chức năng. Rê chuột xem tóm tắt mục đích.'],
    ['Ô đang phát triển', 'Chức năng sắp có (Chuẩn hoá tên, Ký văn bản, Đóng gói AIP).'],
    ['Thông tin nhanh', 'Đặc điểm chính: xử lý trên máy, giữ nguyên file gốc, lượt chạy, định dạng hỗ trợ…'],
    ['Thanh trạng thái', 'Website, email hỗ trợ, tên máy, MAC, IP, phiên bản đang dùng.'],
  ]),
);
them(H2('2.2 Màn giới thiệu của từng chức năng'),
  P('Khi mở một chức năng mà chưa chọn đủ đầu vào, phần mềm hiện **màn giới thiệu**: mô tả ngắn và 3 bước thực hiện. Cột trái được **thu gọn**; khi chọn đủ đầu vào (hoặc cấu hình lần trước đã đủ) cột trái tự mở ra.'),
  ...hinh('04-tach-gioi-thieu.png', 'Màn giới thiệu Tách văn bản (cột trái thu gọn)'),
  ...hop('meo', ['Các chữ gạch chân trong 3 bước (ví dụ **Chọn**, **Phân tích**) bấm được như nút tương ứng. Bấm logo Sao Mai góc dưới phải để về Trang chủ.']),
);
them(H2('2.3 Hướng dẫn nhanh (F1)'),
  P('Bấm `F1` ở bất kỳ màn nào để mở **Hướng dẫn sử dụng** của chức năng đang dùng: các bước, nhãn, phím tắt.'),
  ...hinh('02-huong-dan-f1.png', 'Hộp Hướng dẫn sử dụng — tab Tách văn bản', 520),
);

// ===== Chương 3: Đếm =====
them(batDauChuong('Đếm trang A4'));
them(H2('3.1 Tổng quan'),
  P('Đếm số trang của mọi file PDF trong thư mục (gồm thư mục con), xếp khổ theo **cạnh dài** của trang (không phụ thuộc dọc / ngang) và quy đổi ra **số trang A4** theo hệ số. Có thể kiểm tra thêm DPI và chữ ký số.'),
  ...hinh('03-dem-gioi-thieu.png', 'Màn giới thiệu Đếm trang A4'),
);
them(H2('3.2 Thao tác'));
batDauThaoTac();
them(...buoc('Mở **Chức năng › Đếm trang A4**. Bấm **Chọn**, chọn thư mục chứa PDF (quét cả thư mục con).'),
  ...buoc('Tuỳ chọn: tích **Check ký số** (kiểm tra file đã ký số) và/hoặc **Check DPI** (độ phân giải) — chạy chậm hơn.'),
  ...buoc('Bấm **Đếm**. Có thể bấm **Dừng** để ngừng giữa chừng.', 'Bảng hiện từng trang: khổ, hướng, DPI, ký số, cảnh báo; hàng tab trên bảng đếm số trang theo khổ.'),
  ...hinh('03-dem-ket-qua.png', 'Kết quả Đếm trang A4'),
  ...buoc('Lọc bằng hàng tab **Tất cả / A0…A7**, hoặc bấm số **Cảnh báo**, **File lỗi**, **Ký số** ở dòng tổng. Bấm tên file để mở PDF.'),
  ...buoc('Khi xong, popup kết quả có **Mở kết quả** và **Copy dữ liệu** (dán thẳng vào Excel).', 'Tự lưu `<thư mục>_dem.txt` (tổng hợp) và `<thư mục>_dem_chitiet.txt` (từng trang) cạnh thư mục.'),
);
them(H2('3.3 Cách quy đổi ra A4'),
  ...bang(['Khổ', 'A0', 'A1', 'A2', 'A3', 'A4', 'A5', 'A6', 'A7'], [['Hệ số mặc định', '16', '8', '4', '2', '1', '0,5', '0,25', '0,125']], [1826, 900, 900, 900, 900, 900, 900, 900, 900]),
  P('**Quy A4** = Σ (số trang mỗi khổ × hệ số). Ví dụ 10 trang A3 + 5 trang A4 = 10×2 + 5×1 = **25**.'),
  P('**Sát ngưỡng**: cạnh dài đã đi ≥ 80 % quãng tới khổ kề bên → nên mở file xem lại. Trang lớn hơn A0 (cạnh dài > 1399 mm) quy theo diện tích ÷ A4 hoặc hệ số cố định (đặt ở Cấu hình).'),
);
them(H2('3.4 Cấu hình Đếm trang A4'),
  P('Menu **Cấu hình › Đếm trang A4**: hệ số quy A4 cho từng khổ, cách tính trang lớn hơn A0, ranh giới giữa các khổ (kéo vạch trên thước; gõ **Thử cạnh dài** để xem trang thuộc khổ nào). **Lưu** áp cho lần đếm sau; **Mặc định** trả về giá trị gốc.'),
  ...hinh('03-dem-cau-hinh.png', 'Cấu hình › Đếm trang A4', 520),
);

// ===== Chương 4: Tách =====
them(batDauChuong('Tách văn bản'));
them(H2('4.1 Tổng quan'),
  P('Tách mỗi hồ sơ PDF scan thành nhiều văn bản. Phần mềm tự đọc mã vạch trên **tờ index** (tờ phân cách), nhận trang trắng và đoán trang bị xoay; người dùng **duyệt và sửa nhãn** trước khi tách. File PDF gốc không bị sửa.'),
  ...bang(['Nhãn trang', 'Ý nghĩa', 'Khi tách'], [
    ['**Index** (I)', 'Tờ mã vạch phân cách văn bản.', 'Là điểm cắt, trang bị bỏ.'],
    ['**Trắng** (B)', 'Trang trắng, bìa, chữ thấm mặt sau.', 'Trang bị bỏ.'],
    ['**Split** (S)', 'Văn bản mới bắt đầu từ trang này (dùng khi thiếu tờ index).', 'Là điểm cắt, trang được giữ.'],
    ['**Nội dung** (C)', 'Trang chưa gắn nhãn.', 'Giữ trong văn bản.'],
    ['**Nghi trắng**', 'Máy đoán là trắng nhưng chưa chắc.', '**Phải duyệt** trước khi tách.'],
    ['**Nghi xoay**', 'Máy đoán trang bị xoay (90° / 180° / 270°).', '**Phải duyệt** trước khi tách.'],
  ], [2000, 4026, 3000]),
);
them(H2('4.2 Tên gọi các phần — tab Hồ sơ'), ...hinh('04-tach-chu-thich.png', 'Tab Hồ sơ đang lọc nhóm Nghi trắng'),
  ...chuThichSo([
    ['Thư mục hồ sơ PDF', 'Thư mục chứa các hồ sơ PDF; bấm **Chọn** để đổi, ⓧ để đóng thư mục.'],
    ['Phân tích / Phân tích lại', 'Đọc mã vạch, nhận trang trắng, đoán xoay cho mọi hồ sơ.'],
    ['Tab Hồ sơ / Văn bản', '**Hồ sơ**: duyệt nhãn từng trang. **Văn bản**: xem trước văn bản sẽ tách ra.'],
    ['Cột hồ sơ', 'Danh sách file PDF (số trang, Index, Trắng, số văn bản). Bấm 1 hồ sơ để chỉ xem hồ sơ đó. Dòng tô xanh nhạt = file chứa trang đang chọn.'],
    ['Bộ lọc nhóm', 'Tất cả · Index · Trắng · Nghi trắng · Nghi xoay; **Điểm cắt** = các trang Index + Split. Mỗi nhóm nhớ trang đang làm.'],
    ['Nút gán nhãn', '**S I B** gán nhãn, hoàn tác / làm lại, xoá khỏi lưới.'],
    ['Xoay · cỡ ảnh · sắp xếp', 'Xoay 90° / 180°, phóng to / thu nhỏ ảnh lưới, sắp xếp (vd theo Dung lượng để dồn trang index / trắng).'],
    ['Dải gán nhãn', 'Chỉ ở nhóm Nghi trắng / Nghi xoay: chọn nhãn rồi **Xong (Enter)**.'],
    ['Ô trang', 'Ảnh trang + tên `<hồ sơ>.<trang>`. Nhãn góc **Trắng?** / **90°?** = trang nghi.'],
    ['Trạng thái lưu', 'Tự động lưu nhãn vào `index_and_blank.txt`; thông báo gán nhãn hiện ngay phía trên.'],
    ['Tách', 'Tách mọi hồ sơ theo nhãn. Chỉ chạy khi đã duyệt hết Nghi trắng / Nghi xoay.'],
    ['Tìm hồ sơ', 'Lọc cột hồ sơ theo tên; nút ‹| thu gọn cột trái.'],
  ]),
);
them(H2('4.3 Quy trình tách'));
batDauThaoTac();
them(...buoc('Mở **Chức năng › Tách văn bản**, bấm **Chọn**, chọn thư mục hồ sơ PDF.'),
  ...buoc('Bấm **Phân tích**.', 'Lưới trang hiện ngay; nhãn tự động (Index, Trắng, Nghi trắng, Nghi xoay) đến dần theo từng hồ sơ. Có thể **Dừng** và phân tích tiếp lần sau.'),
  ...hinh('04-tach-ho-so-anh-that.png', 'Tab Hồ sơ sau khi phân tích (dữ liệu mẫu)'),
  ...buoc('Kiểm tra nhãn ở nhóm **Tất cả**. Chọn trang (bấm, `Ctrl`+bấm, `Shift`+bấm, kéo khung) rồi bấm `S` / `I` / `B` để gán, `C` để bỏ nhãn.'),
  ...buoc('Duyệt hết nhóm **Nghi trắng** và **Nghi xoay** (xem 4.4, 4.5).', 'Số trong ngoặc của 2 nhóm về 0.'),
  ...buoc('Chuyển sang tab **Văn bản** để xem trước từng văn bản sẽ tách (chế độ Lưới · 1 trang · 2 trang). Có thể xoay hoặc **Bỏ khỏi văn bản**.'),
  ...hinh('04-tach-van-ban.png', 'Tab Văn bản'),
  ...buoc('Bấm **Tách**.', 'Tạo thư mục `<gốc>_vanban` giữ nguyên cấu trúc thư mục con; mỗi hồ sơ một thư mục chứa `<hồ sơ>.1.pdf`, `<hồ sơ>.2.pdf`…'),
  ...hinh('04-tach-xong.png', 'Popup Tách xong', 460),
  ...hop('luuy', ['Nếu còn trang Nghi trắng / Nghi xoay chưa duyệt, bấm **Tách** sẽ hiện hộp nhắc kèm nút đi duyệt.']),
  ...hinh('04-tach-chua-duyet.png', 'Hộp nhắc khi còn trang chưa duyệt', 460),
);
them(H2('4.4 Duyệt Nghi trắng'),
  P('Chọn bộ lọc **Nghi trắng**. Với trang đang chọn, dải vàng hiện **Trang … gán nhãn:**.'),
  ...hinh('04-tach-file-dang-xem.png', 'Thông báo gán nhãn ngay trên dòng Tự động lưu; dòng hồ sơ chứa trang đang chọn tô xanh nhạt'),
);
batDauThaoTac();
them(...buoc('Bấm `B` (Trắng — mặc định), `C` (Nội dung), `I` (Index) hoặc `S` (Split) để **chọn** nhãn thật của trang (nút chuyển xanh). Lúc này trang **chưa** bị đổi.'),
  ...buoc('Bấm `Enter` (hoặc **Xong**) để áp.', 'Trang rời khỏi nhóm Nghi trắng; phần mềm tự chọn trang nghi kế tiếp; dòng vàng **“… được gán nhãn: …”** hiện phía trên **Tự động lưu**.'),
  ...hop('chuy', ['Trong nhóm Nghi trắng, chỉ `Enter` / **Xong** mới áp nhãn. Bấm nhãn **Trắng?** trên ảnh cũng chỉ chọn sẵn Trắng, chưa áp.', 'Phím `Backspace` xoá trang khỏi lưới (sau 150 ms); phím do bộ gõ tiếng Việt (Unikey/EVKey) tự gửi khi gõ dấu sẽ được bỏ qua.']),
);
them(H2('4.5 Duyệt Nghi xoay'),
  ...hinh('04-tach-nghi-xoay-luoi.png', 'Nhóm Nghi xoay — dải cam'),
);
batDauThaoTac();
them(...buoc('Bấm `Enter` (**1. Xoay thử**): trang được xoay theo góc máy đề xuất.'),
  ...buoc('Nếu chưa đúng: `R` xoay thêm 90°, `Shift+R` xoay 180°, `C` về như cũ.'),
  ...buoc('Bấm `Enter` lần nữa (**2. Đúng rồi, tiếp**).', 'Trang rời nhóm Nghi xoay, chuyển sang trang kế; thông báo “… đã xác nhận xoay …°”.'),
);
them(H2('4.6 Hộp phóng to 1 trang'),
  P('Bấm đúp vào ảnh (hoặc `Enter` ở nhóm thường) để mở hộp phóng to. Bên trái là **cột ảnh nhỏ** các trang của nhóm đang xem; bấm ảnh nào để xem trang đó.'),
  ...hinh('04-tach-phong-to-tat-ca.png', 'Hộp phóng to — nhóm Tất cả: cột ảnh + dải gán nhãn'),
  ...bang(['Nhóm', 'Trong hộp phóng to'], [
    ['Tất cả · Index · Trắng · Điểm cắt', '`B` `C` `I` `S` chọn nhãn → `Enter` / **Xong** áp và sang trang kế. Chưa chọn gì thì `Enter` giữ nhãn hiện tại, chỉ sang trang.'],
    ['Nghi trắng', 'Như trên, mặc định Trắng; trang đã duyệt rời cột ảnh.'],
    ['Nghi xoay', '`Enter` xoay thử → `Enter` đúng rồi, tiếp; `R` / `Shift+R` / `C` chỉnh tay.'],
  ], [2800, 6226]),
  P('Sau khi áp, hộp luôn **tiến về cuối danh sách**; trang không còn thuộc nhóm thì rời cột ảnh; hết cuối danh sách thì quay về đầu; hết trang thì hộp tự đóng. `←` `→` chuyển trang, `+` `−` phóng to / thu nhỏ, `Esc` đóng.'),
  ...hinh('04-tach-phong-to-nghi-trang.png', 'Hộp phóng to — nhóm Nghi trắng'),
);
them(H2('4.7 Phím tắt'),
  ...bang(['Nhóm', 'Phím / thao tác'], [
    ['Chọn trang', 'Bấm · `Ctrl`+bấm thêm/bớt · `Shift`+bấm chọn dải · kéo khung · `←` `→` `↑` `↓` `Home` `End` `PgUp` `PgDn` · `Ctrl+A` chọn hết · `Esc` bỏ chọn'],
    ['Xem', '`Enter` hoặc bấm đúp: xem lớn · chuột phải: menu thao tác'],
    ['Nhãn', '`S` `I` `B` gán / bấm lại để huỷ · `C` bỏ nhãn (Nghi trắng: chọn rồi `Enter`)'],
    ['Xoay · xoá', '`R` 90° · `Shift+R` 180° · `Delete` / `Backspace` xoá khỏi lưới (tab Văn bản: bỏ khỏi văn bản)'],
    ['Hoàn tác', '`Ctrl+Z` hoàn tác · `Ctrl+Y` làm lại — riêng từng nhóm lọc'],
    ['Khác', '`Ctrl+S` lưu nhãn (khi lưu bằng tay) · `Ctrl`+cuộn chuột: cỡ ảnh · `F1` hướng dẫn'],
  ], [2000, 7026]),
);
them(H2('4.8 Cấu hình Tách PDF'),
  P('Menu **Cấu hình › Tách PDF**: ngưỡng nhận trang trắng, loại mã vạch, độ tin cậy nhận xoay, mẫu tên file kết quả (`{hoso}.{n}`), thư mục kết quả, lưu nhãn tự động hoặc bằng tay.'),
  ...hinh('04-tach-cau-hinh.png', 'Cấu hình › Tách PDF', 520),
);

// ===== Chương 5: Mục lục =====
them(batDauChuong('Sinh mục lục hồ sơ'));
them(H2('5.1 Tổng quan'),
  P('Từ **file biên mục** (Excel/CSV: mỗi văn bản một dòng, các dòng cùng hồ sơ liền nhau) và **mẫu mục lục** DOCX, sinh mục lục văn bản cho từng hồ sơ. Kết quả Bóc tách dữ liệu dùng được trực tiếp làm file biên mục.'),
  ...hinh('05-muc-luc-gioi-thieu.png', 'Màn giới thiệu Sinh mục lục'),
);
them(H2('5.2 Thao tác'));
batDauThaoTac();
them(...buoc('**File biên mục**: bấm **Chọn** file `.xlsx .xlsb .xls` hoặc `.csv`; chọn **Sheet** và **Dòng tiêu đề** (số dòng như Excel hiển thị). Nút ⌄ cuối dòng hiện 10 dòng đầu để kiểm tra.'),
  ...buoc('**Mẫu mục lục**: chọn 1 trong 4 mẫu có sẵn hoặc **Chọn** mẫu DOCX riêng (biến viết dạng `{ten_bien}`).', 'Cột trái mở ra, tab **Mẫu** hiện trang mẫu và bảng biến.'),
  ...hinh('05-muc-luc-mau.png', 'Tab Mẫu: biến tự khớp với cột cùng tên'),
  ...buoc('Biến tự khớp với cột cùng tên. Bấm ô **Lấy từ** để chọn cột khác hoặc gõ giá trị cố định rồi `Enter`. **Nhóm theo** (chân cột trái) = cột dùng để tách hồ sơ.'),
  ...hinh('05-muc-luc-lay-tu.png', 'Chọn nguồn cho biến'),
  ...buoc('Tab **Kết quả**: xem trước mục lục từng hồ sơ; dấu **!** = hồ sơ có ô trống ở cột đã ghép.'),
  ...hinh('05-muc-luc-ket-qua.png', 'Tab Kết quả'),
  ...buoc('Bấm **Sinh mục lục** (tuỳ chọn **Kèm PDF**).', 'Mỗi hồ sơ một file DOCX, tên theo mẫu ở Cấu hình (mặc định `MLHS_{so_ho_so}`); popup kết quả có **Mở thư mục**.'),
);
them(H2('5.3 Màu biến'),
  ...bang(['Loại biến', 'Ý nghĩa'], [
    ['Thông tin hồ sơ', 'In một lần (tiêu đề, số hồ sơ…), lấy ở dòng đầu của hồ sơ.'],
    ['Bảng văn bản', 'Biến trong hàng mẫu của bảng: mỗi văn bản (mỗi dòng Excel) thêm một hàng.'],
    ['Tự động', '`{trang_so}` `{tong_so_trang}`: Word tự tính khi in.'],
    ['Chưa ghép', 'Giữ nguyên `{bien}` trong file kết quả.'],
  ], [2400, 6626]),
  ...hop('meo', ['Hộp **Ghép biến…** cho phép ghép nhiều biến cùng lúc; rê chuột lên dòng biến để thấy vị trí trên trang mẫu.']),
  ...hinh('05-muc-luc-ghep-bien.png', 'Hộp Ghép biến', 520),
  ...hinh('05-muc-luc-cau-hinh.png', 'Cấu hình › Sinh mục lục (thư mục kết quả, mẫu tên file)', 520),
);

// ===== Chương 6: Bóc tách =====
them(batDauChuong('Bóc tách dữ liệu'));
them(H2('6.1 Tổng quan'),
  P('Đọc văn bản PDF bằng **mô hình AI đọc ảnh** để lấy các trường thông tin (số ký hiệu, ngày tháng, tác giả, trích yếu, người ký…). Mô hình có thể là **máy chủ nội bộ** (vLLM / Ollama / LM Studio) hoặc dịch vụ đám mây (**Gemini, Claude, GPT, OpenRouter**).'),
  ...hop('chuy', ['Mỗi trang cần đọc được gửi tới mô hình đã chọn. Với dịch vụ đám mây, ảnh văn bản ra khỏi mạng đơn vị — **không dùng cho tài liệu mật**. Khoá API được lưu mã hoá trên máy.']),
);
them(H2('6.2 Tên gọi các phần — tab Kết quả bóc'), ...hinh('06-boc-tach-chu-thich.png', 'Tab Kết quả bóc'),
  ...chuThichSo([
    ['Nguồn văn bản', 'Máy tính (thư mục trên máy, ổ mạng, Google Drive for Desktop) hoặc nguồn khác.'],
    ['Chọn', 'Chọn thư mục PDF (gồm thư mục con).'],
    ['Loại tài liệu', 'Văn bản hành chính, Hồ sơ đất đai, Hợp đồng hoặc loại riêng — quyết định các trường cần đọc.'],
    ['Mô hình đọc · Kiểm tra', 'Mô hình đã khai ở Cấu hình › Bóc tách; **Kiểm tra** thử kết nối.'],
    ['Ghi ra', 'Excel (luôn ghi) và Google Sheet (tuỳ chọn); **Mở Sheet**.'],
    ['Tab Trường dữ liệu / Kết quả bóc', 'Cấu hình trường và thử đọc / xem kết quả từng file.'],
    ['Danh sách file', 'Tên file, số trang, số trường đọc được (vd 6/6) hoặc **Lỗi**.'],
    ['Xem PDF', 'Trang PDF của file đang xem (1 trang hoặc 2 trang).'],
    ['Thanh công cụ', 'Phóng to, xem 1/2 trang, **ReOCR** (đọc lại file này), **Lưu** (lưu phần sửa tay).'],
    ['Các trường', 'Giá trị đã đọc; sửa tay trực tiếp rồi **Lưu**.'],
    ['Ô tìm / lọc', 'Lọc theo tên file, `lỗi`, `5/6`…'],
    ['Bảng kết quả', 'Mọi file một bảng; **Mở Excel** / **Mở GSheet**.'],
    ['Bắt đầu / Chạy lại (n)', 'Đọc mọi file / đọc lại các file đang hiện sau khi lọc.'],
  ]),
);
them(H2('6.3 Chuẩn bị mô hình đọc (Cấu hình › Bóc tách)'),
  P('Mở **Cấu hình › Bóc tách › Mô hình đọc**, chọn nhà cung cấp:'),
  ...bang(['Nhà cung cấp', 'Cần khai'], [
    ['Máy chủ nội bộ', 'Địa chỉ chuẩn OpenAI `http://<máy>:<cổng>/v1` (vLLM 8000 · Ollama 11434 · LM Studio 1234) và model.'],
    ['Gemini', 'Kiểu khoá **Vertex AI (express)** hoặc **AI Studio**, dán khoá API. Model mặc định `gemini-3.7-flash`.'],
    ['Claude · GPT · OpenRouter', 'Dán khoá API; địa chỉ và danh sách model có sẵn.'],
  ], [2400, 6626]),
  P('Bấm **Kiểm tra kết nối** để thử. Địa chỉ dịch vụ đám mây sửa được khi đi qua proxy riêng (bắt buộc `https://`).'),
  ...hinh('06-cau-hinh-noi-bo.png', 'Khai máy chủ nội bộ', 520),
  ...hinh('06-cau-hinh-gemini.png', 'Khai Gemini (Vertex AI)', 520),
);
them(H2('6.4 Thao tác bóc tách'));
batDauThaoTac();
them(...buoc('Mở **Chức năng › Bóc tách dữ liệu**. **Nguồn văn bản**: bấm **Chọn** thư mục PDF.'),
  ...buoc('**Mô hình đọc**: chọn mô hình; bấm **Kiểm tra**.'),
  ...buoc('**Loại tài liệu**: chọn loại.', 'Đủ 3 đầu vào → màn giới thiệu chuyển sang các tab, cột trái mở ra.'),
  ...hinh('06-boc-tach-gioi-thieu.png', 'Màn giới thiệu: các bước đã xong có dấu ✓'),
  ...buoc('Tab **Trường dữ liệu**: xem các trường, **Câu lệnh** gửi mô hình; bút chì (hoặc bấm đúp 1 dòng) để sửa tên cột, mô tả, chuẩn hoá, thứ tự.'),
  ...hinh('06-boc-tach-truong.png', 'Tab Trường dữ liệu'),
  ...buoc('**Thử đọc** file đang xem để kiểm tra (không trừ lượt chạy).'),
  ...hinh('06-boc-tach-thu-doc.png', 'Kết quả Thử đọc'),
  ...buoc('Bấm **Bắt đầu**.', 'Tab **Kết quả bóc** hiện từng file đang đọc; thanh đáy đếm file, trường, thời gian.'),
  ...hinh('06-boc-tach-dang-boc.png', 'Đang bóc tách'),
  ...buoc('Kiểm tra từng file, sửa tay ô sai rồi **Lưu**. File đọc sai có thể **ReOCR** (đọc lại riêng file đó).'),
  ...buoc('Lọc file lỗi (gõ `lỗi` hoặc `5/6` vào ô tìm) rồi **Chạy lại (n)** để đọc lại các file đang hiện.'),
  ...hinh('06-boc-tach-loc-loi.png', 'Lọc file lỗi để chạy lại'),
  ...buoc('Khi xong, popup tổng kết có **Mở Excel**, **Mở GSheet**.', '`<thư mục>_boc_tach.xlsx` cạnh thư mục nguồn (hoặc thư mục đặt ở Cấu hình): mỗi file 1 dòng, nạp thẳng vào Sinh mục lục.'),
  ...hinh('06-boc-tach-xong.png', 'Popup Bóc tách xong', 520),
);
them(H2('6.5 Ghi Google Sheet'),
  P('Ở **Cấu hình › Bóc tách › Ghi kết quả**: dán link Google Sheet, tên tab (để trống = tên thư mục nguồn), cách ghi **Khoá service account** hoặc **Apps Script**; bấm **Ghi thử 1 dòng** để kiểm tra quyền. Bật ô **Google Sheet** ở màn chính để ghi.'),
  P('Tab trên Sheet là bản sao kết quả như file Excel. Có thay đổi (đọc xong file, Lưu, ReOCR) thì phần mềm tự ghi lại cả tab, **tối đa 1 lần mỗi phút**; mất mạng thì tự thử lại.'),
  ...hinh('06-cau-hinh-ghi.png', 'Cấu hình › Bóc tách › Ghi kết quả', 520),
  ...hop('luuy', ['Nếu đơn vị đã dán script Apps Script mẫu từ bản trước 2.1.2, cần dán lại script mới (`dema4_rs/apps-script-mau/boc_tach.gs`) để tránh ghi trùng dòng.']),
);

// ===== Chương 7: Xử lý sự cố =====
them(batDauChuong('Xử lý sự cố'));
them(P('Khi gặp sự cố, tra bảng dưới đây trước khi liên hệ hỗ trợ.'),
  ...bang(['Hiện tượng', 'Nguyên nhân thường gặp', 'Cách xử lý'], [
    ['Không mở được file chạy', 'Hệ điều hành chặn file chưa ký số.', 'Xem mục 1.3 (Run anyway / Open Anyway).'],
    ['Hiện hộp **Liên hệ Sao Mai**', 'Hết lượt chạy hoặc máy bị khoá.', 'Gửi mã máy cho Sao Mai, sau đó bấm **Kiểm tra lại**.'],
    ['Bấm **Tách** không chạy', 'Còn trang Nghi trắng / Nghi xoay chưa duyệt.', 'Duyệt hết 2 nhóm (mục 4.4, 4.5).'],
    ['Trang biến mất khi đang gõ nhãn', 'Bộ gõ tiếng Việt gửi phím Backspace.', 'Bản 2.2.0 đã bỏ qua phím này; nếu lỡ xoá, bấm `Ctrl+Z`.'],
    ['Đếm báo **File lỗi**', 'PDF hỏng hoặc có mật khẩu.', 'Mở thử file bằng trình đọc PDF; xuất lại PDF.'],
    ['Bóc tách báo lỗi 404 (Gemini)', 'Model không có trên dự án / vùng.', 'Chọn model khác trong danh sách (vd `gemini-3.7-flash`).'],
    ['Bóc tách báo lỗi kết nối', 'Sai địa chỉ / khoá API, máy chủ nội bộ tắt, mất mạng.', '**Kiểm tra kết nối** ở Cấu hình; kiểm tra mạng.'],
    ['Không ghi được Google Sheet', 'Chưa chia sẻ Sheet cho service account; sai link / mã bí mật Apps Script.', '**Ghi thử 1 dòng** để xem lỗi chi tiết.'],
    ['Sinh mục lục thiếu dữ liệu', 'Biến chưa ghép hoặc sai **Dòng tiêu đề**.', 'Kiểm tra tab Mẫu (biến chưa ghép) và nút ⌄ xem 10 dòng đầu.'],
  ], [2500, 3100, 3426]),
);

// ===== Phụ lục =====
them(batDauChuong('Phụ lục'));
them(H2('A. Tệp và thư mục phần mềm tạo ra'),
  ...bang(['Tệp / thư mục', 'Vị trí', 'Nội dung'], [
    ['`.dema4_saomai.cfg`', 'Cạnh file chạy (ẩn)', 'Cấu hình người dùng.'],
    ['`.sm-dema4/`', 'Cạnh file chạy (ẩn)', 'Nhãn, cache, log Tách; kết quả Sinh mục lục mặc định.'],
    ['`<thư mục>_dem.txt`, `_dem_chitiet.txt`', 'Cạnh thư mục đếm', 'Kết quả Đếm trang A4.'],
    ['`<gốc>_vanban/`', 'Cạnh thư mục gốc', 'Văn bản đã tách.'],
    ['`index_and_blank.txt`', 'Trong `.sm-dema4/`', 'Nhãn đã duyệt của Tách văn bản.'],
    ['`<thư mục>_boc_tach.xlsx`', 'Cạnh thư mục nguồn', 'Kết quả Bóc tách dữ liệu.'],
  ], [3000, 2400, 3626]),
);
them(H2('B. Thuật ngữ'),
  ...bang(['Thuật ngữ', 'Giải thích'], [
    ['Tờ index', 'Tờ có mã vạch đặt giữa các văn bản khi scan để phân cách.'],
    ['Điểm cắt', 'Trang bắt đầu văn bản mới (Index hoặc Split).'],
    ['Quy A4', 'Số trang quy đổi về khổ A4 theo hệ số.'],
    ['File biên mục', 'Bảng Excel/CSV mô tả văn bản, mỗi văn bản một dòng.'],
    ['Mô hình đọc', 'Mô hình AI nhận ảnh trang và trả về các trường thông tin.'],
    ['ReOCR', 'Đọc lại một file bằng mô hình đang chọn.'],
    ['Mã máy', 'Mã định danh máy dùng để cấp lượt chạy.'],
  ], [2400, 6626]),
);
them(H2('C. Liên hệ hỗ trợ'),
  ...bang(['Kênh', 'Thông tin'], [
    ['Đơn vị', 'Giải pháp số Sao Mai'],
    ['Website', 'saomaiso.com'],
    ['Email', 'hotro@saomaiso.com'],
    ['Điện thoại', '0979 824 615'],
    ['Bản mới', 'github.com/quangtv1/dema4/releases'],
  ], [2400, 6626]),
  ...hop('tk', ['Khi liên hệ, xin gửi kèm **mã máy** (menu **Trợ giúp › Thông tin**), phiên bản phần mềm và ảnh chụp màn hình lỗi.']),
);
them(H2('D. Lịch sử sửa đổi'),
  ...bang(['Phiên bản tài liệu', 'Ngày', 'Phiên bản phần mềm', 'Nội dung'], [
    ['1.0', NGAY, PHIEN_BAN, 'Ban hành lần đầu.'],
  ], [2000, 1700, 2000, 3326]),
);

// ---- Tài liệu ----
const doc = new Document({
  creator: 'Giải pháp số Sao Mai',
  title: 'Hướng dẫn sử dụng dema4-saomai',
  description: `Hướng dẫn sử dụng dema4-saomai phiên bản ${PHIEN_BAN}`,
  features: { updateFields: true },
  styles: {
    default: { document: { run: { font: FONT, size: 22 } } },
    paragraphStyles: [
      { id: 'Heading1', name: 'Heading 1', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 34, bold: true, color: MAU.chinh, font: FONT }, paragraph: { keepNext: true, spacing: { before: 120, after: 240 }, outlineLevel: 0, border: { bottom: vien(MAU.chinh) } } },
      { id: 'Heading2', name: 'Heading 2', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 27, bold: true, color: MAU.chinh, font: FONT }, paragraph: { keepNext: true, keepLines: true, spacing: { before: 280, after: 120 }, outlineLevel: 1 } },
      { id: 'Heading3', name: 'Heading 3', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 23, bold: true, font: FONT }, paragraph: { keepNext: true, spacing: { before: 200, after: 80 }, outlineLevel: 2 } },
    ],
  },
  numbering: { config: [{ reference: 'cham', levels: [{ level: 0, format: LevelFormat.BULLET, text: '•', alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 540, hanging: 280 } } } }] }] },
  sections: [{
    properties: { page: { size: { width: 11906, height: 16838 }, margin: { top: 1440, bottom: 1300, left: 1440, right: 1440 } }, titlePage: true },
    headers: {
      default: new Header({ children: [new Paragraph({ border: { bottom: vien() }, tabStops: [{ type: TabStopType.RIGHT, position: RONG_ND }], children: [new TextRun({ text: 'dema4-saomai — Hướng dẫn sử dụng', size: 17, color: MAU.phu }), new TextRun({ text: `\tPhiên bản ${PHIEN_BAN}`, size: 17, color: MAU.phu })] })] }),
      first: new Header({ children: [new Paragraph({ children: [] })] }),
    },
    footers: {
      default: new Footer({ children: [new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: '– ', size: 18, color: MAU.phu }), new TextRun({ children: [PageNumber.CURRENT], size: 18, color: MAU.phu }), new TextRun({ text: ' –', size: 18, color: MAU.phu })] })] }),
      first: new Footer({ children: [new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: '© Giải pháp số Sao Mai', size: 18, color: MAU.phu })] })] }),
    },
    children: nd.filter((x) => typeof x !== 'function'),
  }],
});

const ra = path.join(GOC, 'huong-dan-su-dung-dema4.docx');
Packer.toBuffer(doc).then((b) => { fs.writeFileSync(ra, b); console.log('Đã tạo', ra, `${(b.length / 1048576).toFixed(1)} MB`); });
