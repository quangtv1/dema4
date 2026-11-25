// Kiểm trên file chạy thật (bản build `--features dev-keys`, khoá thử ở SMKS_DEV_PKI) các việc của màn Ký số:
//   A. Ảnh thu nhỏ các trang (văn bản từ 2 trang): bấm thì lướt tới trang; trang có khung chữ ký có dấu.
//   B. Lượt ký Chính / Nháy: ký nháy mặc định mọi trang, góc dưới trái (đối diện chữ ký chính).
//   C. Chuột phải: xoá khung, thêm khung ở chỗ bấm; chọn khung rồi phím Delete để xoá; không xoá được khung cuối.
//   D. Ký hai lượt: ký nháy rồi ký chính lên bản đã ký nháy. Mỗi lượt đúng một chữ ký; file cuối có hai chữ ký,
//      khối hiển thị đúng chỗ (PyMuPDF), cả hai chữ ký nguyên vẹn (`verify_file` của app).
// Hai lượt dùng cùng khoá thử (bản dev-keys chỉ có một); hai USB token thật cần người phụ trách thử.
// Cấu hình ghi vào thư mục riêng (SMKS_CONFIG_DIR).
//
// Cách dùng: node app/tests/ks-nhay.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9356;
const MM = 72 / 25.4;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const src = path.join(work, 'vb');
const cfgDir = path.join(work, 'cfg');
const shots = path.join(work, 'anh');
for (const d of [src, cfgDir, shots]) fs.mkdirSync(d, { recursive: true });
const longDoc = path.join(src, 'hop dong.pdf');
const shortDoc = path.join(src, 'van ban 2.pdf');
fs.copyFileSync(path.join(repo, 'fixtures/pdf/word-hybrid-xref.pdf'), longDoc);
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), shortDoc);
const readCfg = () => JSON.parse(fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}

const app = spawn(exe, [], {
  env: { ...process.env, SMKS_DEV_PKI: pki, SMKS_CONFIG_DIR: cfgDir, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
  stdio: 'ignore',
});

/** Khối chữ ký hiển thị trong file, nhóm theo tên field (PyMuPDF, toạ độ gốc trên trái, tỉ lệ trang). */
const widgetsOf = (file) => {
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, pymupdf
d = pymupdf.open(sys.argv[1])
r = {}
for i, p in enumerate(d):
    for w in p.widgets():
        if w.field_type != pymupdf.PDF_WIDGET_TYPE_SIGNATURE or w.rect.width == 0:
            continue
        r.setdefault(w.field_name, []).append({"page": i + 1, "x": round(w.rect.x0 / p.rect.width, 4), "y": round(w.rect.y0 / p.rect.height, 4),
            "w": round(w.rect.width / p.rect.width, 4), "h": round(w.rect.height / p.rect.height, 4)})
print(json.dumps(r))`, file], { encoding: 'utf8' });
  if (py.status !== 0) throw new Error(`PyMuPDF: ${py.stderr}`);
  return JSON.parse(py.stdout);
};

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
    } else if (message.method === 'Runtime.exceptionThrown') {
      consoleErrors.push(message.params.exceptionDetails?.exception?.description ?? 'exception');
    } else if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
      consoleErrors.push(message.params.args.map((a) => a.value ?? a.description).join(' '));
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
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: String(e) }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    const v = reply.result?.result?.value;
    if (v?.err) console.log(`  (lỗi trong trang: ${v.err})`);
    return v?.ok;
  };
  const waitFor = async (body, ms = 20000) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      const v = await run(body);
      if (v) return v;
      await sleep(250);
    }
    return null;
  };
  const shot = async (name) => {
    const r = await send('Page.captureScreenshot', { format: 'png' });
    if (r.result?.data) fs.writeFileSync(path.join(shots, name), Buffer.from(r.result.data, 'base64'));
  };
  const mouse = (type, x, y, button = 'left') =>
    send('Input.dispatchMouseEvent', { type, x, y, button, buttons: type === 'mousePressed' ? (button === 'right' ? 2 : 1) : 0, clickCount: 1 });
  const click = async (p, button = 'left') => {
    await mouse('mouseMoved', p.x, p.y);
    await mouse('mousePressed', p.x, p.y, button);
    await mouse('mouseReleased', p.x, p.y, button);
    await sleep(250);
  };
  const key = async (k, code = k) => {
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: k, code, windowsVirtualKeyCode: k === 'Delete' ? 46 : 27 });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: k === 'Delete' ? 46 : 27 });
    await sleep(300);
  };
  /** Các khung trên bản xem trước: trang, toạ độ tỉ lệ, tâm trên màn hình (sau khi cuộn khung vào giữa). */
  const boxes = () =>
    run(`const sheets = [...document.querySelectorAll('.trang-ds > .giay')];
      return [...document.querySelectorAll('.giay .chuky-o')].map((b) => { const s = b.parentElement.getBoundingClientRect(); const r = b.getBoundingClientRect();
        return { i: Number(b.dataset.i), page: sheets.indexOf(b.parentElement) + 1, x: +((r.left - s.left) / s.width).toFixed(4), y: +((r.top - s.top) / s.height).toFixed(4),
                 w: +(r.width / s.width).toFixed(4), h: +(r.height / s.height).toFixed(4), chon: b.classList.contains('chon') }; }).sort((a, b) => a.page - b.page || a.x - b.x);`);
  const boxCenter = (i) =>
    run(`const b = document.querySelector('.giay .chuky-o[data-i="${i}"]'); b.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 200));
      const r = b.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  const indexOnPage = (page) =>
    run(`const sheets = [...document.querySelectorAll('.trang-ds > .giay')];
      const b = [...document.querySelectorAll('.giay .chuky-o')].find((x) => sheets.indexOf(x.parentElement) + 1 === ${page}); return b ? Number(b.dataset.i) : null;`);
  const waitBoxes = (n) => waitFor(`return document.querySelectorAll('.giay .chuky-o').length === ${n} && 1`, 15000);
  const marks = () => run(`return [...document.querySelectorAll('app-page-thumbs .tn-trang.co-ky')].map((b) => Number(b.dataset.trang)).join(',')`);
  const menuItems = () => waitFor(`const m = document.querySelector('.ky-menu'); return m && [...m.querySelectorAll('.km-muc')].map((b) => b.textContent.replace(/\\s+/g, ' ').trim() + (b.disabled ? ' (tắt)' : '')).join(' | ')`, 5000);
  const clickMenu = (text) =>
    run(`const b = [...document.querySelectorAll('.ky-menu .km-muc')].find((x) => x.textContent.includes(${JSON.stringify(text)}) && !x.disabled); if (!b) return false; b.click(); return true;`);
  const selectFile = async (name) => {
    await run(`[...document.querySelectorAll('.ds .dong')].find((r) => r.textContent.includes(${JSON.stringify(name)}))?.click(); return 1;`);
    await waitFor(`return document.querySelector('.dieuhuong .tt b')?.textContent === ${JSON.stringify(name)} && !document.querySelector('.xem-tt') && !!document.querySelector('.giay canvas') && 1`, 20000);
    await sleep(600);
  };
  /** Ký các văn bản đang nạp bằng khoá thử; trả "tổng/thành công/lỗi". */
  const signAll = async () => {
    await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((b) => !b.disabled)?.click(); return 1;`);
    await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
    const note = await run(`return document.querySelector('app-popup .hint.mo-ta')?.textContent.replace(/\\s+/g, ' ').trim()`);
    await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
       r?.querySelector('input')?.click(); return !!r;`);
    await run(`[...document.querySelectorAll('app-popup button.nut.chinh')].find((b) => b.textContent.includes('Ký số'))?.click(); return 1;`);
    const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((b) => b.textContent).join('/')`, 120000);
    await run(`[...document.querySelectorAll('app-popup button')].find((b) => b.textContent.includes('Đóng'))?.click(); return 1;`);
    await sleep(1500);
    return { done, note };
  };
  const dropFolder = async (folder, n) => {
    await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(folder)}], position: { x: 500, y: 400 } } }); return 1;`);
    return waitFor(`return document.querySelectorAll('.ds .dong').length === ${n} && ${n}`, 20000);
  };
  await send('Runtime.enable');

  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(800);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1200);
  check('Thả thư mục: 2 văn bản', (await dropFolder(src, 2)) === 2);
  await selectFile('hop dong.pdf');
  await waitBoxes(1);

  // ---- A. Ảnh thu nhỏ trang ------------------------------------------------------------------------
  const thumbs = await waitFor(`const t = [...document.querySelectorAll('app-page-thumbs .tn-trang')];
     return t.length === 3 && t.every((b) => b.querySelector('canvas').width > 50) && t.map((b) => b.querySelector('canvas').width + 'x' + b.querySelector('canvas').height).join(' ')`, 15000);
  check('A1. Văn bản 3 trang: cột ảnh thu nhỏ có 3 trang, đã vẽ', !!thumbs, String(thumbs));
  const inked = await run(`const c = document.querySelector('app-page-thumbs .tn-trang canvas'); const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
     let n = 0; for (let i = 0; i < d.length; i += 4) if (d[i] < 128) n++; return n;`);
  check('A2. Ảnh thu nhỏ trang 1 có nội dung (điểm tối)', inked > 50, String(inked));
  check('A3. Lượt Chính (mặc định): chỉ trang 3 có dấu khung chữ ký', (await marks()) === '3', await marks());
  await run(`document.querySelector('app-page-thumbs .tn-trang[data-trang="2"]').click(); return 1;`);
  await sleep(500);
  const went = await run(`const v = document.querySelector('.vungxem').getBoundingClientRect(); const s = document.querySelectorAll('.trang-ds > .giay')[1].getBoundingClientRect();
     return { dTop: Math.round(s.top - v.top), chon: document.querySelector('app-page-thumbs .tn-trang.chon')?.dataset.trang };`);
  check('A4. Bấm ảnh trang 2: vùng xem lướt tới đầu trang 2, ảnh trang 2 được tô', !!went && Math.abs(went.dTop - 12) <= 3 && went.chon === '2', JSON.stringify(went));
  await shot('1-anh-thu-nho.png');
  await selectFile('van ban 2.pdf');
  const noThumbs = await run(`return !document.querySelector('app-page-thumbs')`);
  check('A5. Văn bản 1 trang: không có cột ảnh thu nhỏ', noThumbs === true);
  await selectFile('hop dong.pdf');

  // ---- B. Lượt ký nháy ------------------------------------------------------------------------------
  await run(`[...document.querySelectorAll('.luot-ky .seg')].find((b) => b.textContent.trim() === 'Nháy').click(); return 1;`);
  await waitBoxes(3);
  const nb = await boxes();
  const leftX = (15 * MM) / 595.32;
  check(
    'B1. Lượt Nháy: 3 khung, mỗi trang một khung, góc dưới trái cách lề 15 / 20 mm',
    nb?.length === 3 && nb.map((b) => b.page).join(',') === '1,2,3' && nb.every((b) => Math.abs(b.x - leftX) < 0.004 && Math.abs(b.y + b.h - (1 - (20 * MM) / 841.92)) < 0.004),
    JSON.stringify(nb),
  );
  await sleep(700);
  check('B2. config.json ghi lượt ký "initial"; ảnh thu nhỏ: cả 3 trang có dấu', readCfg().signature.kind === 'initial' && (await marks()) === '1,2,3', `${readCfg().signature.kind}; ${await marks()}`);

  // ---- C. Chuột phải, Delete -------------------------------------------------------------------------
  const i1 = await indexOnPage(1);
  await click(await boxCenter(i1), 'right');
  const items1 = await menuItems();
  check('C1. Chuột phải vào khung: menu có Xoá khung chữ ký, Thêm vào mọi trang', /Xoá khung chữ ký/.test(items1 ?? '') && /mọi trang/.test(items1 ?? ''), String(items1));
  await shot('2-menu-chuot-phai.png');
  await clickMenu('Xoá khung chữ ký');
  await waitBoxes(2);
  await sleep(700);
  const own1 = readCfg().signature.fileInitialPositions?.[longDoc];
  check('C2. Xoá khung trang 1: còn khung trang 2, 3; lưu là khung riêng của văn bản này', (await boxes()).map((b) => b.page).join(',') === '2,3' && own1?.length === 2 && (await marks()) === '2,3',
    `${JSON.stringify(own1?.map((b) => b.page))}; dấu ${await marks()}`);
  // Chọn khung trang 2 (bấm chuột trái, không kéo) rồi phím Delete.
  const i2 = await indexOnPage(2);
  await click(await boxCenter(i2));
  const selected = await run(`return document.querySelector('.giay .chuky-o.chon')?.dataset.i`);
  await key('Delete');
  await waitBoxes(1);
  check('C3. Chọn khung trang 2 rồi bấm Delete: khung bị xoá, còn khung trang 3', selected === String(i2) && (await boxes()).map((b) => b.page).join(',') === '3', `chọn ${selected}`);
  // Khung cuối cùng (TD-56): xoá được, văn bản thành "Chưa gán chữ ký" (bỏ qua khi ký); nút ↻ gán lại như trước.
  const i3 = await indexOnPage(3);
  await click(await boxCenter(i3));
  await key('Delete');
  const hint = await waitFor(`return document.querySelector('.ky-nhac')?.textContent.trim()`, 3000);
  const skipped = await waitFor(`return !!document.querySelector('.ks-chuagan') && document.querySelectorAll('.trang-ds .chuky-o').length === 0
     && [...document.querySelectorAll('.ds .dong.chon .chip-tt')].map((c) => c.textContent.trim()).join() === 'Chưa gán chữ ký' && 1`, 5000);
  const empty = await run(`const s = document.querySelectorAll('.trang-ds > .giay')[2]; s.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 200));
     const r = s.getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height * 0.5 };`);
  await click(empty, 'right');
  const items2 = await menuItems();
  await key('Escape');
  const menuGone = await run(`return !document.querySelector('.ky-menu')`);
  check('C4. Xoá khung cuối: văn bản "Chưa gán chữ ký" (hết khung, cảnh báo, trạng thái trong danh sách); chuột phải trên trang vẫn có "Thêm khung chữ ký ở đây"; Esc đóng menu',
    /chưa gán chữ ký/.test(hint ?? '') && !!skipped && /Thêm khung chữ ký ở đây/.test(items2 ?? '') && menuGone, `${hint} | ${items2}`);
  await run(`document.querySelector('.ve-mau')?.click(); return 1;`);
  await waitBoxes(1);
  check('C4b. Nút ↻ khi chưa gán chữ ký: gán lại, khung trang 3 như trước', (await boxes()).map((b) => b.page).join(',') === '3' && !(await run(`return !!document.querySelector('.ks-chuagan')`)));
  // Chuột phải vào chỗ trống giữa trang 1 → Thêm khung ở đây.
  const spot = await run(`const s = document.querySelectorAll('.trang-ds > .giay')[0]; s.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 200));
     const r = s.getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height * 0.45 };`);
  await click(spot, 'right');
  const items3 = await menuItems();
  await clickMenu('Thêm khung chữ ký ở đây');
  await waitBoxes(2);
  const added = (await boxes()).find((b) => b.page === 1);
  check('C5. Chuột phải chỗ trống trang 1 → "Thêm khung chữ ký ở đây": khung mới ở trang 1, tâm tại chỗ bấm, cùng cỡ khung đang có',
    /Thêm khung chữ ký ở đây/.test(items3 ?? '') && !!added && Math.abs(added.x + added.w / 2 - 0.5) < 0.01 && Math.abs(added.y + added.h / 2 - 0.45) < 0.01 && Math.abs(added.w - nb[0].w) < 0.002,
    `${items3} | ${JSON.stringify(added)}`);
  await shot('3-sau-khi-them-xoa.png');
  // Văn bản 1 trang không bị ảnh hưởng: vẫn khung nháy mặc định.
  await selectFile('van ban 2.pdf');
  await waitBoxes(1);
  const other = await boxes();
  check('C6. Văn bản khác không đổi: 1 khung nháy mặc định ở trang 1', other?.length === 1 && other[0].page === 1 && Math.abs(other[0].x - leftX) < 0.004, JSON.stringify(other));
  await selectFile('hop dong.pdf');
  await waitBoxes(2);

  // ---- D. Ký nháy rồi ký chính ---------------------------------------------------------------------
  const pass1 = await signAll();
  check('D1. Lượt 1 (Ký nháy): ký 2 văn bản; hộp Xác nhận ghi lượt "Ký nháy"', pass1.done === '2/2/0' && /lượt Ký nháy/.test(pass1.note ?? ''), `${pass1.done} | ${pass1.note}`);
  const out1 = `${src}_DaKySo`;
  const long1 = path.join(out1, 'hop dong_signed.pdf');
  const short1 = path.join(out1, 'van ban 2_signed.pdf');
  const w1 = widgetsOf(long1);
  const f1 = Object.values(w1);
  check('D2. File 3 trang sau lượt 1: đúng 1 chữ ký, hiện ở trang 1 (khung thêm) và trang 3 (khung còn lại)',
    f1.length === 1 && f1[0].map((w) => w.page).join(',') === '1,3' && f1[0].some((w) => w.page === 1 && Math.abs(w.x - added.x) < 0.006 && Math.abs(w.y - added.y) < 0.006),
    JSON.stringify(w1));
  const ws1 = Object.values(widgetsOf(short1));
  check('D3. File 1 trang sau lượt 1: 1 chữ ký nháy góc dưới trái trang 1', ws1.length === 1 && ws1[0].length === 1 && Math.abs(ws1[0][0].x - leftX) < 0.006, JSON.stringify(ws1));

  // Lượt 2: chữ ký chính lên các bản đã ký nháy.
  await run(`[...document.querySelectorAll('.luot-ky .seg')].find((b) => b.textContent.trim() === 'Chính').click(); return 1;`);
  check('D4. Nạp thư mục bản đã ký nháy (DaKySo)', (await dropFolder(out1, 2)) === 2);
  await selectFile('hop dong_signed.pdf');
  await waitBoxes(1);
  const mb = await boxes();
  check('D5. Lượt Chính: 1 khung ở trang 3 góc dưới phải (khung nháy của lượt trước không áp)', mb?.length === 1 && mb[0].page === 3 && Math.abs(mb[0].x + mb[0].w - (1 - leftX)) < 0.004, JSON.stringify(mb));
  const pass2 = await signAll();
  check('D6. Lượt 2 (Chữ ký chính): ký 2 văn bản; hộp Xác nhận ghi lượt "Chữ ký chính"', pass2.done === '2/2/0' && /lượt Chữ ký chính/.test(pass2.note ?? ''), `${pass2.done} | ${pass2.note}`);
  const long2 = path.join(`${out1}_DaKySo`, 'hop dong_signed_signed.pdf');
  const w2 = widgetsOf(long2);
  const pages2 = Object.values(w2).map((l) => l.map((w) => w.page).join('+')).sort();
  check('D7. File cuối có 2 chữ ký: nháy (trang 1 + 3), chính (trang 3 góc dưới phải)', Object.keys(w2).length === 2 && pages2.join(',') === '1+3,3', JSON.stringify(w2));
  const report = await run(`return await window.__TAURI_INTERNALS__.invoke('verify_file', { path: ${JSON.stringify(long2)}, revocation: false })`);
  const sigs = report?.signatures ?? [];
  // Từ TD-58 sau mỗi lượt ký có thêm bản cập nhật DSS (ký dài hạn), nên chữ ký sau không còn phủ tới byte cuối;
  // phần sau mỗi chữ ký chỉ được là chữ ký / DSS.
  check('D8. verify_file: 2 chữ ký, không chữ ký nào bị sửa (khoá thử nên "người ký không thuộc CA Ban Cơ yếu"); sau mỗi chữ ký chỉ có chữ ký / DSS; file có DSS',
    sigs.length === 2 && sigs.every((x) => x.verdict !== 'invalidSignature' && x.verdict !== 'modifiedAfterSigning' && x.changes === 'signatures') &&
      sigs[1].changeNotes.every((n) => /DSS/.test(n.text)) && (report?.dss?.ocsps ?? 0) >= 1,
    JSON.stringify(sigs.map((x) => ({ v: x.verdict, page: x.page, whole: x.coversWholeFile, changes: x.changes, notes: x.changeNotes }))));
  const firstBytes = fs.readFileSync(long1);
  check('D9. Bản sau là bản trước nối thêm (bản đã ký nháy là tiền tố từng byte)', fs.readFileSync(long2).subarray(0, firstBytes.length).equals(firstBytes));
  await shot('4-sau-hai-luot.png');

  check('Không có lỗi trong console của giao diện', consoleErrors.length === 0, consoleErrors.slice(0, 3).join(' | '));
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  // Đóng như người dùng để app tự dọn; không thoát thì mới buộc dừng.
  spawnSync('taskkill', ['/PID', String(app.pid)], { stdio: 'ignore' });
  for (let i = 0; i < 40 && app.exitCode === null; i++) await sleep(250);
  if (app.exitCode === null) app.kill();
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
