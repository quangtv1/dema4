// Kiểm nhiều khung chữ ký trên nhiều trang ở màn "Xem trước & vị trí chữ ký" (TD-55) trên file chạy thật (bản
// `--features dev-keys`, khoá thử ở SMKS_DEV_PKI): văn bản nhiều trang có cột ảnh thu nhỏ từng trang; chuột phải vào
// ảnh thu nhỏ để thêm / xoá khung ở trang đó; chuột phải trên trang để thêm khung ở chỗ bấm, xoá khung, thêm vào mọi
// trang; Áp dụng rồi ký thật, đếm khung chữ ký trên từng trang bằng PyMuPDF.
//
// Cách dùng: node app/tests/ks-nhieukhung.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9359;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const shots = path.join(work, 'anh');
const src = path.join(work, 'vb');
for (const d of [cfgDir, shots, src]) fs.mkdirSync(d, { recursive: true });
fs.copyFileSync(path.join(repo, 'fixtures/pdf/word-hybrid-xref.pdf'), path.join(src, 'a 3 trang.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, 'b 1 trang.pdf'));
const readCfg = () => JSON.parse(fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8'));

/** Số khung chữ ký (widget) trên từng trang của file đã ký. */
const widgetsOf = (file) => {
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, pymupdf
d = pymupdf.open(sys.argv[1])
print(json.dumps([sum(1 for w in p.widgets() if w.field_type == pymupdf.PDF_WIDGET_TYPE_SIGNATURE and w.rect.width > 0) for p in d]))`, file], { encoding: 'utf8' });
  if (py.status !== 0) throw new Error(`PyMuPDF: ${py.stderr}`);
  return JSON.parse(py.stdout);
};

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
const app = spawn(exe, [], {
  env: { ...process.env, SMKS_DEV_PKI: pki, SMKS_CONFIG_DIR: cfgDir, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
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
  const rightClick = async (x, y) => {
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'right', buttons: 2, clickCount: 1 });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'right', buttons: 0, clickCount: 1 });
    return waitFor(`const m = document.querySelector('.vi-tri .vt-menu'); return m && [...m.querySelectorAll('.km-dau, .km-muc')].map((b) => b.textContent.replace(/\\s+/g, ' ').trim() + (b.disabled ? ' (tắt)' : ''))`, 5000);
  };
  const clickMenu = (text) =>
    run(`const b = [...document.querySelectorAll('.vi-tri .vt-menu .km-muc')].find((x) => x.textContent.includes(${JSON.stringify(text)}) && !x.disabled); if (!b) return false; b.click(); return true;`);
  /** Tâm ảnh thu nhỏ trang `n`. */
  const thumbAt = (n) =>
    run(`const t = document.querySelector('.vi-tri .trang-nho [data-trang="${n}"]'); if (!t) throw new Error('không thấy ảnh thu nhỏ trang ${n}: ' + document.querySelectorAll('.trang-nho .tn-trang').length + ' ảnh, ' + document.title); t.scrollIntoView({ block: 'nearest' }); await new Promise((r) => setTimeout(r, 150));
      const r = t.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  /** Trang có dấu khung ở cột ảnh thu nhỏ, và các khung trên trang lớn (trang, tỉ lệ). */
  const state = () =>
    run(`return { marks: [...document.querySelectorAll('.vi-tri .trang-nho .tn-trang.co-ky')].map((t) => +t.dataset.trang),
      boxes: [...document.querySelectorAll('.vi-tri .trang-ds > .giay')].flatMap((s, i) => [...s.querySelectorAll('.chuky-o')].map((b) => {
        const r = b.getBoundingClientRect(), p = s.getBoundingClientRect(); return { page: i + 1, cx: +((r.left + r.width / 2 - p.left) / p.width).toFixed(2), cy: +((r.top + r.height / 2 - p.top) / p.height).toFixed(2) }; })),
      foot: document.querySelector('.vi-tri .chan .dan')?.textContent.replace(/\\s+/g, ' ').trim() };`);
  const marksAre = (list) => waitFor(`const m = [...document.querySelectorAll('.vi-tri .trang-nho .tn-trang.co-ky')].map((t) => +t.dataset.trang).join(','); return m === ${JSON.stringify(list.join(','))} && 1`, 8000);

  await send('Runtime.enable');
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`const c = await window.__TAURI_INTERNALS__.invoke('config_defaults'); c.signature.display = 'text';
             await window.__TAURI_INTERNALS__.invoke('config_save', { config: c }); return 1;`);
  await send('Page.reload');
  await sleep(1500);
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(500);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title)`);
  await sleep(1200);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
    event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  await waitFor(`return document.querySelectorAll('.ds .dong').length === 2 && 1`, 20000);
  await sleep(800);
  await run(`[...document.querySelectorAll('.nguon button.nut')].find((b) => b.textContent.includes('Cấu hình vị trí'))?.click(); return 1;`);
  await waitFor(`return document.querySelectorAll('.vi-tri .ab-file').length === 2 && 1`, 10000);
  await run(`[...document.querySelectorAll('.vi-tri .ab-file')].find((f) => f.textContent.includes('a 3 trang'))?.click(); return 1;`);
  const thumbs = await waitFor(`const t = document.querySelectorAll('.vi-tri .trang-nho .tn-trang'); return t.length === 3 && document.querySelectorAll('.vi-tri .trang-ds > .giay .chuky-o').length === 1 && t.length`, 15000);
  await sleep(600);
  const s0 = await state();
  check('Văn bản 3 trang: có cột ảnh thu nhỏ 3 trang, một khung (theo mẫu, trang 3) được đánh dấu', thumbs === 3 && s0.marks.join() === '3', JSON.stringify(s0));

  // 1) Chuột phải ảnh thu nhỏ trang 1 › Thêm khung vào trang 1.
  let t = await thumbAt(1);
  const m1 = await rightClick(t.x, t.y);
  await shot('1-menu-anh-thu-nho.png');
  await clickMenu('Thêm khung chữ ký vào trang 1');
  const ok1 = await marksAre([1, 3]);
  const s1 = await state();
  check('Chuột phải ảnh thu nhỏ trang 1: menu "Thêm khung chữ ký vào trang 1" → khung ở trang 1 (cùng chỗ khung trang 3), chân màn ghi "2 khung · trang 1, 3"',
    !!ok1 && JSON.stringify(m1).includes('Thêm khung chữ ký vào trang 1') && /2 khung · trang 1, 3/.test(s1.foot) &&
      s1.boxes.length === 2 && s1.boxes[0].cx === s1.boxes[1].cx && s1.boxes[0].cy === s1.boxes[1].cy,
    JSON.stringify({ m1, s1 }));

  // 2) Chuột phải giữa trang 2 (chỗ trống) › Thêm khung ở đây.
  const p2 = await run(`const s = document.querySelectorAll('.vi-tri .trang-ds > .giay')[1]; s.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 300));
    const r = s.getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height * 0.4 };`);
  const m2 = await rightClick(p2.x, p2.y);
  await clickMenu('Thêm khung chữ ký ở đây');
  const ok2 = await marksAre([1, 2, 3]);
  const s2 = await state();
  const b2 = s2.boxes.find((b) => b.page === 2);
  check('Chuột phải chỗ trống giữa trang 2 › "Thêm khung chữ ký ở đây": khung trang 2 có tâm đúng chỗ bấm (50%, 40%)',
    !!ok2 && JSON.stringify(m2).includes('trang 2') && !!b2 && Math.abs(b2.cx - 0.5) < 0.02 && Math.abs(b2.cy - 0.4) < 0.02, JSON.stringify({ m2, b2 }));

  // 3) Chuột phải ảnh thu nhỏ trang 2 › Xoá khung ở trang 2.
  t = await thumbAt(2);
  const m3 = await rightClick(t.x, t.y);
  await clickMenu('Xoá khung chữ ký ở trang 2');
  const ok3 = await marksAre([1, 3]);
  check('Chuột phải ảnh thu nhỏ trang 2 (đã có khung) › "Xoá khung chữ ký ở trang 2": còn trang 1, 3', !!ok3 && JSON.stringify(m3).includes('Xoá khung chữ ký ở trang 2'), JSON.stringify(m3));

  // 4) Chuột phải vào khung trang 1 › Xoá khung chữ ký.
  const k1 = await run(`const q = () => document.querySelectorAll('.vi-tri .trang-ds > .giay')[0].querySelector('.chuky-o'); q().scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 400));
    const r = q().getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  const m4 = await rightClick(k1.x, k1.y);
  await clickMenu('Xoá khung chữ ký');
  const ok4 = await marksAre([3]);
  check('Chuột phải vào khung trang 1 › "Xoá khung chữ ký": còn một khung (trang 3)', !!ok4 && JSON.stringify(m4).includes('Xoá khung chữ ký'), JSON.stringify(m4));

  // 5) Xoá khung cuối cùng (trang 3) → văn bản "chưa gán chữ ký": không còn khung, có cảnh báo trên trang, chân màn và lưới file.
  const boxAt = (i) =>
    run(`const q = () => document.querySelectorAll('.vi-tri .trang-ds > .giay')[${i}].querySelector('.chuky-o'); q().scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 400));
      const r = q().getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  let k = await boxAt(2);
  const m5 = await rightClick(k.x, k.y);
  const delOn = !!m5 && m5.some((x) => x.startsWith('Xoá khung chữ ký') && !x.endsWith('(tắt)'));
  await clickMenu('Xoá khung chữ ký');
  const skipA = await waitFor(`return !!document.querySelector('.vi-tri .vt-chuagan') && document.querySelectorAll('.vi-tri .trang-ds .chuky-o').length === 0 && 1`, 8000);
  await sleep(300);
  const s5 = await run(`return { foot: document.querySelector('.vi-tri .chan .dan')?.textContent.replace(/\\s+/g, ' ').trim(),
    mini: [...document.querySelectorAll('.vi-tri .ab-file')].map((f) => f.querySelector('.ten').textContent + (f.querySelector('.mini.chuagan') ? ' [chưa gán]' : '')),
    marks: document.querySelectorAll('.vi-tri .trang-nho .tn-trang.co-ky').length };`);
  await shot('3-chua-gan.png');
  check('Xoá được khung cuối cùng: văn bản "chưa gán chữ ký" — hết khung, cảnh báo trên trang, chân màn "Chưa gán chữ ký", ô trong lưới file có nhãn',
    delOn && !!skipA && /Chưa gán chữ ký/.test(s5.foot) && s5.mini.includes('a 3 trang.pdf [chưa gán]') && s5.marks === 0, JSON.stringify({ m5, s5 }));

  // 6) Chuột phải ảnh thu nhỏ trang 3 › Thêm khung → gán lại (khung riêng, chế độ chuyển "Từng file"); rồi thêm vào mọi trang.
  t = await thumbAt(3);
  const m6 = await rightClick(t.x, t.y);
  await clickMenu('Thêm khung chữ ký vào trang 3');
  const ok6 = await marksAre([3]);
  const mode6 = await run(`return document.querySelector('.vi-tri .vt-che-do').value + (document.querySelector('.vi-tri .vt-chuagan') ? ' còn cảnh báo' : '')`);
  k = await boxAt(2);
  await rightClick(k.x, k.y);
  await clickMenu('Thêm khung giống thế này vào mọi trang');
  const ok6b = await marksAre([1, 2, 3]);
  check('Văn bản chưa gán: chuột phải ảnh thu nhỏ trang 3 › "Thêm khung chữ ký vào trang 3" → gán lại, chế độ "Từng file"; thêm vào mọi trang → 3 trang',
    !!ok6 && mode6 === 'per' && !!ok6b && JSON.stringify(m6).includes('Thêm khung chữ ký vào trang 3'), JSON.stringify({ m6, mode6 }));

  // 7) Văn bản 1 trang: menu "Xoá hết khung" không có (1 khung); Xoá → chưa gán chữ ký.
  await run(`[...document.querySelectorAll('.vi-tri .ab-file')].find((f) => f.textContent.includes('b 1 trang'))?.click(); return 1;`);
  await waitFor(`return document.querySelector('.vi-tri .dieuhuong .tt b')?.textContent === 'b 1 trang.pdf' && document.querySelectorAll('.vi-tri .trang-ds > .giay .chuky-o').length === 1 && 1`, 15000);
  await sleep(600);
  k = await boxAt(0);
  await rightClick(k.x, k.y);
  await clickMenu('Xoá khung chữ ký');
  const skipB = await waitFor(`return !!document.querySelector('.vi-tri .vt-chuagan') && 1`, 8000);
  check('Văn bản 1 trang: xoá khung duy nhất → chưa gán chữ ký', !!skipB);

  // 8) Áp dụng → config; ký thật: văn bản 3 trang ký đủ 3 khung, văn bản chưa gán bị bỏ qua.
  await run(`[...document.querySelectorAll('.vi-tri button')].find((b) => b.textContent.includes('Áp dụng'))?.click(); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && 1`, 10000);
  await sleep(800);
  const sig = readCfg().signature;
  const pathA = Object.keys(sig.filePositions ?? {}).find((p) => p.endsWith('a 3 trang.pdf'));
  check('Áp dụng: skipFiles có đúng văn bản 1 trang; văn bản 3 trang có khung riêng trang 1, 2, 0; bật "Từng file"',
    (sig.skipFiles ?? []).length === 1 && sig.skipFiles[0].endsWith('b 1 trang.pdf') && !!pathA && sig.filePositions[pathA].map((p) => p.page).join() === '1,2,0' && sig.perFile === true,
    JSON.stringify({ skip: sig.skipFiles, fp: sig.filePositions, perFile: sig.perFile }));
  const rowB = await run(`return [...document.querySelectorAll('.ds .dong')].map((d) => d.querySelector('.ten-cot > span')?.textContent + ': ' + d.querySelector('.chip-tt')?.textContent.trim())`);
  check('Màn Ký số: văn bản 1 trang có trạng thái "Chưa gán chữ ký"', JSON.stringify(rowB).includes('b 1 trang.pdf: Chưa gán chữ ký'), JSON.stringify(rowB));
  await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((b) => !b.disabled)?.click(); return 1;`);
  await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
  await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
     r?.querySelector('input')?.click(); return !!r;`);
  await run(`[...document.querySelectorAll('app-popup button.nut.chinh')].find((b) => b.textContent.includes('Ký số'))?.click(); return 1;`);
  const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((b) => b.textContent).join('/')`, 120000);
  const note = await run(`return document.querySelector('app-popup .ks-bo-qua')?.textContent.replace(/\\s+/g, ' ').trim() ?? ''`);
  await shot('4-bo-qua.png');
  await run(`[...document.querySelectorAll('app-popup button')].find((b) => b.textContent.includes('Đóng'))?.click(); return 1;`);
  await sleep(800);
  const out = `${src}_DaKySo`;
  const wA = widgetsOf(path.join(out, 'a 3 trang_signed.pdf'));
  const bOut = fs.existsSync(path.join(out, 'b 1 trang_signed.pdf'));
  check('Ký thật: chỉ ký văn bản 3 trang (1/1), có khung ở cả 3 trang; văn bản chưa gán bị bỏ qua (không có file kết quả), hộp kết quả ghi "Bỏ qua 1"',
    done === '1/1/0' && wA.join() === '1,1,1' && !bOut && /Bỏ qua 1 văn bản chưa gán chữ ký/.test(note), JSON.stringify({ done, wA, bOut, note }));

  // 9) Màn Ký số: văn bản chưa gán có cảnh báo; chuột phải trên trang › Thêm khung → về "Chờ ký"; xoá lại → "Chưa gán chữ ký".
  await run(`[...document.querySelectorAll('.ds .dong')].find((d) => d.textContent.includes('b 1 trang'))?.click(); return 1;`);
  const banner = await waitFor(`return !!document.querySelector('.ks-chuagan') && document.querySelectorAll('.trang-ds > .giay').length === 1 && 1`, 15000);
  await sleep(600);
  const pg = await run(`const s = document.querySelector('.trang-ds > .giay'); s.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 300));
    const r = s.getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height * 0.5 };`);
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: pg.x, y: pg.y, button: 'none', buttons: 0 });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: pg.x, y: pg.y, button: 'right', buttons: 2, clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: pg.x, y: pg.y, button: 'right', buttons: 0, clickCount: 1 });
  await waitFor(`return !!document.querySelector('.ky-menu') && 1`, 5000);
  await run(`[...document.querySelectorAll('.ky-menu .km-muc')].find((b) => b.textContent.includes('Thêm khung chữ ký ở đây'))?.click(); return 1;`);
  const back = await waitFor(`const d = [...document.querySelectorAll('.ds .dong')].find((x) => x.textContent.includes('b 1 trang')); return d?.querySelector('.chip-tt')?.textContent.trim() === 'Chờ ký' && document.querySelectorAll('.trang-ds .chuky-o').length === 1 && 1`, 8000);
  await sleep(1200);
  const skip9 = (readCfg().signature.skipFiles ?? []).length;
  check('Màn Ký số: văn bản chưa gán có cảnh báo; chuột phải › "Thêm khung chữ ký ở đây" → về "Chờ ký", một khung, skipFiles rỗng', !!banner && !!back && skip9 === 0, `${!!banner} ${!!back} ${skip9}`);
  const kb = await run(`const b = document.querySelector('.trang-ds .chuky-o'); const r = b.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: kb.x, y: kb.y, button: 'none', buttons: 0 });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: kb.x, y: kb.y, button: 'right', buttons: 2, clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: kb.x, y: kb.y, button: 'right', buttons: 0, clickCount: 1 });
  await waitFor(`return !!document.querySelector('.ky-menu') && 1`, 5000);
  await run(`[...document.querySelectorAll('.ky-menu .km-muc')].find((b) => b.textContent.includes('Xoá khung chữ ký') && !b.disabled)?.click(); return 1;`);
  const again = await waitFor(`const d = [...document.querySelectorAll('.ds .dong')].find((x) => x.textContent.includes('b 1 trang')); return d?.querySelector('.chip-tt')?.textContent.trim() === 'Chưa gán chữ ký' && !!document.querySelector('.ks-chuagan') && 1`, 8000);
  await sleep(1200);
  check('Màn Ký số: xoá khung duy nhất → lại "Chưa gán chữ ký" (skipFiles có văn bản)', !!again && (readCfg().signature.skipFiles ?? []).length === 1, JSON.stringify(readCfg().signature.skipFiles));

  check('Không có lỗi trong console của giao diện', consoleErrors.length === 0, consoleErrors.slice(0, 3).join(' | '));
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  app.kill();
}
const ok = results.length > 0 && results.every(Boolean);
console.log(`KET QUA: ${ok ? 'DAT' : 'KHONG DAT'} (${results.filter(Boolean).length}/${results.length})`);
process.exit(ok ? 0 : 1);
