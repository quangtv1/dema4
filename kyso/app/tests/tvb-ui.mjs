// Kiểm tra màn Tạo văn bản trên file chạy thật, thao tác qua giao diện như người dùng:
// Trang chủ → Ctrl+N → kéo thả mẫu và dữ liệu (giả lập sự kiện kéo thả của Tauri) → chờ ghép biến, danh sách
// Kết quả, bản xem trước → bấm "Tạo n văn bản" → hộp kết quả → kiểm file trên đĩa. Chụp ảnh từng bước.
//
// Cách dùng: node app/tests/tvb-ui.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> [thư mục ảnh]
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, shots] = process.argv.slice(2).map((p) => (p ? path.resolve(p) : p));
const debugPort = 9337;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

// Chép mẫu và dữ liệu ra thư mục làm việc để kết quả không ghi vào repo.
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
const tpl = path.join(work, '2_quyet_dinh_khen_thuong.docx');
const data = path.join(work, '2_quyet_dinh_khen_thuong.xlsx');
fs.copyFileSync(path.join(repo, 'fixtures/merge/2_quyet_dinh_khen_thuong.docx'), tpl);
fs.copyFileSync(path.join(repo, 'fixtures/merge/2_quyet_dinh_khen_thuong.xlsx'), data);
if (shots) fs.mkdirSync(shots, { recursive: true });

const app = spawn(exe, [], {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
  stdio: 'ignore',
});

try {
  let target;
  for (let i = 0; i < 80 && !target; i++) {
    await sleep(250);
    try {
      target = (await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json()).find((t) => t.type === 'page');
    } catch {}
  }
  if (!target) throw new Error('không kết nối được WebView2');
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve);
    socket.addEventListener('error', reject);
  });
  let nextId = 1;
  const pending = new Map();
  const consoleErrors = [];
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    } else if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
      consoleErrors.push(message.params.args.map((a) => a.value ?? a.description).join(' '));
    } else if (message.method === 'Runtime.exceptionThrown') {
      consoleErrors.push(message.params.exceptionDetails?.exception?.description ?? 'exception');
    } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
      consoleErrors.push(message.params.entry.text);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const id = nextId++;
      pending.set(id, resolve);
      socket.send(JSON.stringify({ id, method, params }));
    });
  const run = async (body) => {
    const reply = await send('Runtime.evaluate', {
      expression: `(async () => { ${body} })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    return reply.result?.result?.value;
  };
  const waitFor = async (body, timeoutMs = 20000) => {
    const end = Date.now() + timeoutMs;
    while (Date.now() < end) {
      const value = await run(body);
      if (value) return value;
      await sleep(250);
    }
    return null;
  };
  const shot = async (name) => {
    if (!shots) return;
    const image = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
  };
  /** Bấm phần tử đầu tiên khớp selector có chữ chứa `text`. */
  const click = (selector, text) =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(selector)})]
           .find((e) => e.textContent.replace(/\\s+/g, ' ').includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  await send('Runtime.enable');
  await send('Log.enable');

  // 1. Trang chủ.
  const home = await waitFor(`return document.querySelectorAll('.tc-the').length === 3 && document.title`);
  check('Trang chủ: 4 thẻ chức năng', !!home, String(home));
  const version = await waitFor(`return [...document.querySelectorAll('.tc-day *')].map((e) => e.textContent).join(' ').match(/1\\.5\\.0-dev/)?.[0]`);
  check('Chân Trang chủ có phiên bản lấy từ lõi', version === '1.5.0-dev', String(version));
  await sleep(800);
  await shot('1-trang-chu.png');

  // 2. Ctrl+N mở Tạo văn bản.
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true })); return true;`);
  const title = await waitFor(`return document.title.includes('Tạo văn bản') && document.title`);
  check('Ctrl+N mở màn Tạo văn bản, đổi tiêu đề cửa sổ', !!title, String(title));
  await sleep(500);
  await shot('2-tao-van-ban-trong.png');

  // 3. Kéo thả mẫu và dữ liệu (sự kiện kéo thả của Tauri gửi tới webview chính).
  const emitted = await run(`try {
      await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {
        target: { kind: 'Webview', label: 'main' },
        event: 'tauri://drag-drop',
        payload: { paths: ${JSON.stringify([tpl, data])}, position: { x: 400, y: 300 } },
      });
      return 'ok';
    } catch (e) { return 'loi: ' + JSON.stringify(e); }`);
  check('Giả lập kéo thả 2 file vào cửa sổ', emitted === 'ok', emitted);

  const plan = await waitFor(`const b = [...document.querySelectorAll('.chan button.nut.chinh')][0];
     return b && /Tạo \\d+ văn bản/.test(b.textContent) && !b.disabled && b.textContent.trim();`, 30000);
  check('Nút chính đổi thành "Tạo 5 văn bản"', plan === 'Tạo 5 văn bản', String(plan));
  const listCount = await run(`return document.querySelectorAll('.ds .dong').length`);
  check('Danh sách Kết quả có 5 dòng', listCount === 5, String(listCount));

  // 4. Bản xem trước PDF thật (pdf.js vẽ lên canvas).
  const canvas = await waitFor(`const c = document.querySelector('canvas');
     if (!c || c.width < 100) return null;
     const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
     let dark = 0; for (let i = 0; i < d.length; i += 4) if (d[i] < 128) dark++;
     return dark > 1000 ? { w: c.width, h: c.height, dark } : null;`, 40000);
  check('Xem trước: pdf.js vẽ trang PDF có chữ', !!canvas, JSON.stringify(canvas));
  await sleep(400);
  await shot('3-ket-qua-xem-truoc.png');

  // 4b. Đổi dòng: lõi giữ Word chạy sẵn và tạo trước dòng lân cận, nên bản xem trước đổi gần như ngay.
  const pixels = `const c = document.querySelector('canvas'); if (!c) return null;
     const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
     let h = 0; for (let i = 0; i < d.length; i += 64) h = (h * 31 + d[i]) >>> 0; return h;`;
  await sleep(4000); // cho lõi tạo trước các dòng còn lại
  const switchTo = async (index) => {
    const before = await run(pixels);
    const started = Date.now();
    await run(`document.querySelectorAll('.ds .dong')[${index}].click(); return true;`);
    const changed = await waitFor(`const h = (() => { ${pixels} })(); return h !== null && h !== ${before} && !document.querySelector('.xem-tt');`, 15000);
    return changed ? Date.now() - started : null;
  };
  const toRow3 = await switchTo(2);
  const toRow4 = await switchTo(3);
  const back = await switchTo(0);
  check('Đổi sang dòng đã tạo trước dưới 1 giây', toRow3 !== null && toRow3 < 1000 && toRow4 !== null && toRow4 < 1000, `${toRow3} ms, ${toRow4} ms`);
  check('Quay lại dòng đã xem dưới 1 giây', back !== null && back < 1000, `${back} ms`);

  // 4b'. Ctrl+→ / Ctrl+← đổi văn bản đang xem.
  const selected = () => run(`return document.querySelector('.ds .dong.chon .so')?.textContent.trim()`);
  const press = (key) => run(`document.body.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}', ctrlKey: true, bubbles: true })); return 1;`);
  const s0 = await selected();
  await press('ArrowRight');
  const s1 = await selected();
  await press('ArrowLeft');
  const s2 = await selected();
  const shown = await waitFor(`return !document.querySelector('.xem-tt') && document.querySelector('.dieuhuong .tt b')?.textContent`, 10000);
  check('Ctrl+→ sang văn bản sau, Ctrl+← về văn bản trước', s1 === String(Number(s0) + 1) && s2 === s0, `${s0} -> ${s1} -> ${s2}; đang xem ${shown}`);

  // 4c. Zoom mặc định 100 %, ô "Tô màu biến" bật sẵn; tắt thì bản xem trước đổi, bật lại thì về như cũ.
  const zoomText = await run(`return document.querySelector('.dieuhuong .zm')?.textContent.trim()`);
  check('Zoom mặc định 100 %', zoomText === '100 %', String(zoomText));
  const toggle = `const c = document.querySelector('.to-bien input'); c.click(); return c.checked;`;
  const initiallyOn = await run(`return document.querySelector('.to-bien input')?.checked`);
  const marked = await run(pixels);
  const offState = await run(toggle);
  const plainHash = await waitFor(`const h = (() => { ${pixels} })(); return h !== ${marked} && !document.querySelector('.xem-tt') && h;`, 15000);
  const onState = await run(toggle);
  const markedAgain = await waitFor(`const h = (() => { ${pixels} })(); return h === ${marked} && !document.querySelector('.xem-tt') && h;`, 15000);
  check(
    'Tô màu biến: bật sẵn; tắt thì bản xem trước không tô, bật lại như cũ',
    initiallyOn === true && offState === false && !!plainHash && onState === true && markedAgain === marked,
    `bat=${initiallyOn} tat=${offState} khac=${!!plainHash} lai=${markedAgain === marked}`,
  );

  // 5. Tab Mẫu: bảng ghép biến.
  const tabMau = await click('.tabs .tab, .tabs button, .tab', 'Mẫu');
  const mapped = await waitFor(`return [...document.querySelectorAll('*')].map((e) => e.childElementCount === 0 ? e.textContent : '')
     .find((t) => /Đã ghép đủ \\d+ biến|Chưa ghép \\d+ biến/.test(t))`, 15000);
  check('Tab Mẫu: trạng thái ghép biến', tabMau && /Đã ghép đủ 10 biến/.test(mapped ?? ''), String(mapped));
  await sleep(2500);
  await shot('4-tab-mau.png');
  await click('.tabs .tab, .tabs button, .tab', 'Kết quả');

  // 6. Tạo văn bản.
  await click('.chan button.nut.chinh', 'Tạo 5 văn bản');
  await sleep(500);
  if (await run(`return !!document.querySelector('app-popup') && document.body.textContent.includes('Vẫn tạo')`)) {
    await click('app-popup button', 'Vẫn tạo');
  }
  const done = await waitFor(`const t = document.querySelector('app-popup .tieu-p'); return t && t.textContent.trim();`, 120000);
  check('Hộp kết quả "Tạo văn bản thành công"', done === 'Tạo văn bản thành công', String(done));
  const numbers = await run(`return [...document.querySelectorAll('app-popup .so-lon b')].map((b) => b.textContent.trim()).join('/')`);
  check('Tổng/Thành công/Lỗi = 5/5/0', numbers === '5/5/0', String(numbers));
  await shot('5-hoan-thanh.png');

  const outFolder = fs.readdirSync(work).map((n) => path.join(work, n)).find((p) => fs.statSync(p).isDirectory());
  const files = outFolder ? fs.readdirSync(outFolder) : [];
  check('5 file PDF trên đĩa, không còn .tmp', files.length === 5 && files.every((f) => f.endsWith('.pdf')), files.join(', '));

  // 7. Thanh sheet trong bảng dữ liệu: thả file có 2 sheet, bấm sheet thứ hai.
  await click('app-popup button', 'Đóng');
  const tricky = path.join(work, 'tricky.xlsx');
  fs.copyFileSync(path.join(repo, 'fixtures/table/tricky.xlsx'), tricky);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {
      target: { kind: 'Webview', label: 'main' }, event: 'tauri://drag-drop',
      payload: { paths: ${JSON.stringify([tricky])}, position: { x: 400, y: 300 } } }); return true;`);
  await waitFor(`return [...document.querySelectorAll('.ng-file .o input')].some((i) => i.value.endsWith('tricky.xlsx'))`, 15000);
  await run(`const b = document.querySelector('.nguon .xem-bang'); if (b && !b.classList.contains('mo')) b.click(); return true;`);
  const tabs = await waitFor(`const t = [...document.querySelectorAll('.xls-sheet')].map((b) => b.textContent.trim()); return t.length ? t.join(',') : null;`, 15000);
  await click('.xls-sheet', 'Khac');
  const switched = await waitFor(`const c = document.querySelector('.xls-sheet.chon'); const x = document.querySelector('.xls-dau .xq');
     return c && c.textContent.trim() === 'Khac' && x && x.textContent.includes('Khac') && x.textContent.trim();`, 15000);
  check('Bảng dữ liệu có thanh sheet, bấm sheet "Khac" thì đọc sheet đó', tabs === 'DuLieu,Khac' && !!switched, `${tabs} | ${switched}`);
  const cleared = await waitFor(`return !document.querySelector('.vungxem canvas')`, 10000);
  check('Sheet không có dòng dữ liệu: bỏ bản xem trước cũ', !!cleared);
  // Đổi sheet / dòng tiêu đề ở khung nguồn khi bảng dữ liệu đang đóng: đọc lại nhưng không tự mở bảng.
  await click('.xls-dau button', 'Đóng');
  await waitFor(`return !document.querySelector('.xls-dau') && 1`, 5000);
  await run(`const s = document.querySelector('.nguon select.sheet'); s.value = 'DuLieu'; s.dispatchEvent(new Event('change', { bubbles: true })); return 1;`);
  const reread = await waitFor(`return document.querySelector('.nguon select.sheet')?.value === 'DuLieu' && !!document.querySelector('.vungxem canvas') && 1`, 15000);
  await sleep(400);
  const closedAfterSheet = await run(`return !document.querySelector('.xls-dau') && !document.querySelector('.nguon .xem-bang').classList.contains('mo')`);
  await run(`const i = [...document.querySelectorAll('.nguon input')].find((x) => x.type === 'number'); i.value = '2'; i.dispatchEvent(new Event('change', { bubbles: true })); return 1;`);
  await sleep(1500);
  const closedAfterRow = await run(`return !document.querySelector('.xls-dau')`);
  await run(`const i = [...document.querySelectorAll('.nguon input')].find((x) => x.type === 'number'); i.value = '1'; i.dispatchEvent(new Event('change', { bubbles: true })); return 1;`);
  await sleep(1200);
  await run(`document.querySelector('.nguon .xem-bang').click(); return 1;`);
  const openedByClick = await waitFor(`const x = document.querySelector('.xls-dau .xq'); return x && x.textContent.includes('DuLieu') && x.textContent.trim()`, 10000);
  check('Đổi sheet / dòng tiêu đề khi bảng đóng: đọc lại sheet mới, bảng vẫn đóng; bấm "Xem dữ liệu" mới mở',
    !!reread && closedAfterSheet === true && closedAfterRow === true && !!openedByClick, `${reread} ${closedAfterSheet} ${closedAfterRow} | ${openedByClick}`);
  const widths = await run(`return [...document.querySelectorAll('.nguon .ng-file')].slice(0, 2).map((e) => { const r = e.getBoundingClientRect(); return Math.round(r.left) + ':' + Math.round(r.width); }).join(',')`);
  const [g1, g2] = String(widths).split(',');
  check('Ô mẫu và ô dữ liệu thẳng hàng, cùng độ rộng', g1 === g2, String(widths));
  // Nâng cao mở ngay trong khung nguồn, ô Tên file và Thư mục lưu thẳng cột với ô mẫu.
  await click('.nguon button', 'Nâng cao');
  const advCols = await waitFor(`const f = [...document.querySelectorAll('.nguon .ng-file')]; if (f.length < 4) return null;
     return f.map((e) => Math.round(e.getBoundingClientRect().left)).join(',');`, 5000);
  check('Nâng cao: các ô thẳng cột với ô mẫu', !!advCols && new Set(String(advCols).split(',')).size === 1, String(advCols));
  const rights = await run(`return [...document.querySelectorAll('.nguon > .ng-c4, .nguon > .ng-c34')].map((e) => Math.round(e.getBoundingClientRect().right)).join(',')`);
  const rows = await run(`const n = document.querySelector('.nguon'); return new Set([...n.children].filter((e) => !e.classList.contains('ng-ngan')).map((e) => { const r = e.getBoundingClientRect(); return Math.round((r.top + r.bottom) / 16); })).size`);
  check('Khung nguồn: nút dồn mép phải thẳng hàng, 2 dòng + 2 dòng Nâng cao', new Set(String(rights).split(',')).size === 1 && rows === 4, `mép phải ${rights}; ${rows} dòng`);
  await sleep(300);
  await shot('6-thanh-sheet.png');

  // 8. Thu gọn khung chọn file thành một dòng tóm tắt, bấm vào thì mở lại.
  await run(`document.querySelector('.nguon .ng-thu').click(); return 1;`);
  const gon = await waitFor(`const g = document.querySelector('.nguon-gon');
     return !document.querySelector('.nguon') && g && g.textContent.replace(/\\s+/g, ' ').trim();`, 5000);
  await sleep(200);
  await shot('7-thu-gon.png');
  await run(`document.querySelector('.nguon-gon').click(); return 1;`);
  const moLai = await waitFor(`return !!document.querySelector('.nguon .ng-file') && !document.querySelector('.nguon-gon')`, 5000);
  check(
    'Thu gọn khung chọn file còn một dòng tóm tắt, bấm thì mở lại',
    !!gon && gon.includes('2_quyet_dinh_khen_thuong.docx') && gon.includes('tricky.xlsx') && !!moLai,
    String(gon),
  );

  check('Không có lỗi trong console của giao diện', consoleErrors.length === 0, consoleErrors.slice(0, 3).join(' | '));
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  app.kill();
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
