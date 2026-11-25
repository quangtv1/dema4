// Kiểm khung chữ ký trên bản xem trước của màn Ký số (bản build `--features dev-keys`, khoá thử ở SMKS_DEV_PKI):
// ảnh đã chọn được nhớ (config.json), khung hiện đúng chỗ của mẫu mặc định, kéo đổi chỗ và kéo góc đổi cỡ bằng
// chuột thật (CDP), vị trí được nhớ, ký xong thì khối chữ ký trong file nằm đúng khung (đọc bằng PyMuPDF).
// Cấu hình ghi vào thư mục riêng của bài kiểm (SMKS_CONFIG_DIR), không đụng config.json thật.
//
// Cách dùng: node app/tests/ks-sigbox.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử> <ảnh .png>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki, image] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9352;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const src = path.join(work, 'vb');
const cfgDir = path.join(work, 'cfg');
fs.mkdirSync(cfgDir, { recursive: true });
const MM = 72 / 25.4;
const shots = path.join(work, 'anh');
fs.mkdirSync(src, { recursive: true });
fs.mkdirSync(shots, { recursive: true });
fs.copyFileSync(path.join(repo, 'fixtures/pdf/word-hybrid-xref.pdf'), path.join(src, 'van ban 1.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, 'van ban 2.pdf'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
try {
  await fetch(`http://127.0.0.1:${debugPort}/json`);
  console.error(`LOI: cổng ${debugPort} đang bận.`);
  process.exit(1);
} catch {}

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
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
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
    return reply.result?.result?.value?.ok;
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
  const mouse = (type, x, y) => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1 });
  const drag = async (from, to) => {
    await mouse('mouseMoved', from.x, from.y);
    await mouse('mousePressed', from.x, from.y);
    for (let i = 1; i <= 10; i++) {
      await mouse('mouseMoved', from.x + ((to.x - from.x) * i) / 10, from.y + ((to.y - from.y) * i) / 10);
    }
    await mouse('mouseReleased', to.x, to.y);
    await sleep(300);
  };
  const boxInfo = () =>
    run(`const b = document.querySelector('.giay .chuky-o'); if (!b) return null;
      const s = b.parentElement.getBoundingClientRect(); const r = b.getBoundingClientRect();
      const img = b.querySelector('img');
      return { page: [...document.querySelectorAll('.trang-ds > .giay')].indexOf(b.parentElement) + 1,
        x: (r.left - s.left) / s.width, y: (r.top - s.top) / s.height, w: r.width / s.width, h: r.height / s.height,
        cx: r.left + r.width / 2, cy: r.top + r.height / 2, right: r.right, bottom: r.bottom,
        sheet: { left: s.left, top: s.top, width: s.width, height: s.height },
        img: img ? img.complete && img.naturalWidth > 0 : false, text: b.textContent, editable: b.classList.contains('keo-duoc') };`);
  // Khung có nằm trọn trong vùng xem và ở giữa (hoặc vùng xem đã cuộn hết cỡ về phía khung) không.
  const focused = () =>
    run(`const v = document.querySelector('.vungxem'); const b = document.querySelector('.giay .chuky-o'); if (!v || !b) return null;
      const vr = v.getBoundingClientRect(); const r = b.getBoundingClientRect();
      const dy = r.top + r.height / 2 - (vr.top + v.clientHeight / 2);
      const atEnd = (dy > 0 && v.scrollTop >= v.scrollHeight - v.clientHeight - 1) || (dy < 0 && v.scrollTop <= 0);
      const inView = r.top >= vr.top - 1 && r.bottom <= vr.top + v.clientHeight + 1;
      return { ok: inView && (Math.abs(dy) < 30 || atEnd), dy: Math.round(dy), scrollTop: Math.round(v.scrollTop), page: [...document.querySelectorAll('.trang-ds > .giay')].indexOf(b.parentElement) + 1 };`);
  const stored = async () => {
    await sleep(700);
    try {
      return JSON.parse(fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8')).signature.positions[0] ?? null;
    } catch {
      return undefined;
    }
  };

  // Ảnh và kiểu hiển thị nhớ từ lần trước (config.json); vị trí chưa đặt.
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`const c = await window.__TAURI_INTERNALS__.invoke('config_defaults');
             c.signature.imagePath = ${JSON.stringify(image)}; c.signature.display = 'imageAndText';
             await window.__TAURI_INTERNALS__.invoke('config_save', { config: c }); return 1;`);
  await send('Page.reload');
  await sleep(1500);
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  const rows = await waitFor(`return document.querySelectorAll('.ds .dong').length === 2 && 2`);
  check('Thả thư mục: 2 văn bản', rows === 2, String(rows));
  const mode = await run(`return document.querySelector('select.kieu-ky')?.selectedOptions[0]?.textContent.trim()`);
  const imgName = await run(`return [...document.querySelectorAll('.nguon [pathInput], .nguon-gon')].map((e) => e.textContent + ' ' + (e.querySelector('input')?.value ?? '')).join(' | ')`);
  check('Ảnh và kiểu "Ảnh + thông tin" được nhớ từ lần trước', mode === 'Ảnh + thông tin', `${mode}; ${imgName}`);

  await waitFor(`const b = document.querySelector('.giay .chuky-o img'); return !!b && b.complete && b.naturalWidth > 0`, 20000);
  // Mẫu A4 mặc định đặt khung ở trang cuối: vùng xem tự cuộn tới khung, không cần cuộn tay.
  await sleep(400);
  const f0 = await focused();
  check('Mở văn bản 3 trang: vùng xem tự cuộn tới khung chữ ký ở trang 3', !!f0 && f0.ok && f0.page === 3 && f0.scrollTop > 0, JSON.stringify(f0));
  const b0 = await boxInfo();
  check('Khung chữ ký hiện trên trang cuối (3), có ảnh đã nạp và dòng "Ký bởi:", kéo được', !!b0 && b0.page === 3 && b0.img && /Ký bởi:/.test(b0.text) && b0.editable, JSON.stringify(b0 && { ...b0, sheet: undefined }));
  check('Vị trí mặc định (mẫu A4): góc dưới phải, cách lề 15 / 20 mm', !!b0 && Math.abs(b0.x + b0.w - (1 - (15 * MM) / 595.32)) < 0.004 && Math.abs(b0.y + b0.h - (1 - (20 * MM) / 841.92)) < 0.004, b0 && `x+w=${(b0.x + b0.w).toFixed(4)} y+h=${(b0.y + b0.h).toFixed(4)}`);
  await shot('1-khung-mac-dinh.png');

  // Kéo khung sang trái, lên khoảng 70% chiều cao của trang đó.
  const target1 = { x: b0.sheet.left + b0.sheet.width * 0.08 + (b0.cx - b0.sheet.left - b0.x * b0.sheet.width), y: b0.sheet.top + b0.sheet.height * 0.7 + (b0.cy - b0.sheet.top - b0.y * b0.sheet.height) };
  await drag({ x: b0.cx, y: b0.cy }, target1);
  const b1 = await boxInfo();
  const p1 = await stored();
  check('Kéo khung: khung tới chỗ thả, vị trí được nhớ (config.json)', !!b1 && Math.abs(b1.x - 0.08) < 0.01 && Math.abs(b1.y - 0.7) < 0.01 && p1?.page === 3 && Math.abs(p1.xRatio - b1.x) < 0.005 && Math.abs(p1.yRatio - b1.y) < 0.005,
    JSON.stringify(p1));

  // Kéo góc dưới phải: rộng thêm 120 px, cao thêm 40 px.
  // Khung ở 70% trang có thể nằm dưới chân màn: cuộn vào giữa vùng xem như người dùng.
  await run(`document.querySelector('.giay .chuky-o').scrollIntoView({ block: 'center' }); return 1;`);
  await sleep(300);
  const br = await run(`const r =document.querySelector('.giay .chuky-o .tay.br').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
  await drag(br, { x: br.x + 120, y: br.y + 40 });
  const b2 = await boxInfo();
  const p2 = await stored();
  check('Kéo góc: khung to ra, góc trên trái giữ nguyên', !!b2 && b2.w > b1.w + 0.15 && b2.h > b1.h + 0.03 && Math.abs(b2.x - b1.x) < 0.002 && Math.abs(b2.y - b1.y) < 0.002 && Math.abs(p2.widthRatio - b2.w) < 0.005,
    JSON.stringify(p2));
  const reset = await run(`return !!document.querySelector('.dieuhuong button[title^="Bỏ vị trí kéo tay"]')`);
  check('Có nút bỏ vị trí kéo tay khi đã kéo', reset);
  await shot('2-khung-da-keo.png');

  // Sang văn bản 2: khung giữ vị trí (áp cho cả lô).
  await run(`document.querySelectorAll('.ds .dong')[1].click(); return 1;`);
  await sleep(1500);
  const b3 = await waitFor(`return !!document.querySelector('.giay .chuky-o')`).then(boxInfo);
  check('Văn bản 2: khung cùng vị trí (áp cho cả lô)', !!b3 && Math.abs(b3.x - b2.x) < 0.003 && Math.abs(b3.y - b2.y) < 0.003, JSON.stringify(b3 && { x: b3.x, y: b3.y }));
  const f3 = await focused();
  check('Sang văn bản 2: vùng xem tự cuộn tới khung chữ ký', !!f3 && f3.ok && f3.page === 1, JSON.stringify(f3));
  // Về lại văn bản 1; cuộn vùng xem lên đầu trước để thấy nó tự cuộn lại tới khung.
  await run(`document.querySelector('.vungxem').scrollTop = 0; return 1;`);
  await run(`document.querySelectorAll('.ds .dong')[0].click(); return 1;`);
  await sleep(1500);
  await waitFor(`return !!document.querySelector('.giay .chuky-o') && document.querySelectorAll('.trang-ds > .giay').length === 3`);
  await sleep(300);
  const f4 = await focused();
  check('Về văn bản 1: vùng xem tự cuộn tới khung đã kéo ở trang 3', !!f4 && f4.ok && f4.page === 3 && f4.scrollTop > 0, JSON.stringify(f4));
  // Giữ Ctrl + lăn chuột (chuột thật qua CDP) trên vùng xem: phóng to rồi thu nhỏ một nấc.
  const vc = await run(`const r = document.querySelector('.vungxem').getBoundingClientRect(); return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };`);
  const zm = () => run(`return document.querySelector('.zm')?.textContent.trim()`);
  const zA = await zm();
  await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: vc.x, y: vc.y, deltaX: 0, deltaY: -100, modifiers: 2 });
  const zB = await waitFor(`const t = document.querySelector('.zm')?.textContent.trim(); return t === '110 %' && t`);
  await sleep(800);
  await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: vc.x, y: vc.y, deltaX: 0, deltaY: 100, modifiers: 2 });
  const zC = await waitFor(`const t = document.querySelector('.zm')?.textContent.trim(); return t === '100 %' && t`);
  check('Màn Ký số: Ctrl + lăn chuột lên phóng to 100 → 110 %, xuống thu nhỏ về 100 %', zA === '100 %' && zB === '110 %' && zC === '100 %', `${zA} → ${zB} → ${zC} (${await zm()})`);
  await sleep(800);
  await run(`document.querySelectorAll('.ds .dong')[1].click(); return 1;`);
  await sleep(1500);

  // Ký bằng khoá thử.
  await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((b) => !b.disabled)?.click(); return 1;`);
  await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
  await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
     r?.querySelector('input')?.click(); return !!r;`);
  await run(`[...document.querySelectorAll('app-popup button.nut.chinh')].find((b) => b.textContent.includes('Ký số'))?.click(); return 1;`);
  const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((b) => b.textContent).join('/')`, 120000);
  check('Ký 2 văn bản bằng khoá thử', done === '2/2/0', String(done));
  await run(`[...document.querySelectorAll('app-popup button')].find((b) => b.textContent.includes('Đóng'))?.click(); return 1;`);
  await sleep(2500);
  const hidden = await run(`return !document.querySelector('.giay .chuky-o')`);
  check('Xem file đã ký: không vẽ khung xem trước chồng lên khối thật', hidden);
  await shot('3-da-ky.png');

  // Khối chữ ký trong file nằm đúng khung (PyMuPDF: toạ độ gốc trên trái).
  const out = `${src}_DaKySo`;
  const files = fs.existsSync(out) ? fs.readdirSync(out).sort() : [];
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, pymupdf
r = []
for f in sys.argv[1:]:
    d = pymupdf.open(f)
    for i, p in enumerate(d):
        for w in p.widgets():
            r.append({"file": f.split("\\\\")[-1], "page": i + 1, "x": w.rect.x0 / p.rect.width, "y": w.rect.y0 / p.rect.height,
                      "w": w.rect.width / p.rect.width, "h": w.rect.height / p.rect.height})
print(json.dumps(r))`, ...files.map((f) => path.join(out, f))], { encoding: 'utf8' });
  const widgets = JSON.parse(py.stdout || '[]');
  const okPos = widgets.length === 2 && widgets.every((w) => w.page === (w.file.startsWith('van ban 1') ? 3 : 1) && Math.abs(w.x - p2.xRatio) < 0.005 && Math.abs(w.y - p2.yRatio) < 0.005 && Math.abs(w.w - p2.widthRatio) < 0.005 && Math.abs(w.h - p2.heightRatio) < 0.005);
  check('Khối chữ ký trong 2 file nằm đúng khung đã kéo: file 3 trang ở trang 3, file 1 trang ở trang 1 (lệch < 0,5% trang)', okPos, JSON.stringify(widgets.map((w) => ({ ...w, x: w.x.toFixed(4), y: w.y.toFixed(4), w: w.w.toFixed(4), h: w.h.toFixed(4) }))) + (py.stderr ? ' ' + py.stderr.slice(0, 200) : ''));

  // Về mặc định.
  await run(`document.querySelector('.dieuhuong button[title^="Bỏ vị trí kéo tay"]')?.click(); return 1;`);
  const p4 = await stored();
  check('Bấm nút bỏ vị trí kéo tay: config.json không còn vị trí tự đặt', p4 === null, JSON.stringify(p4));
  // localStorage dùng chung với app của người dùng: không để lại ảnh / kiểu / vị trí của bài kiểm.
  await run(`['smks.ks.anh', 'smks.ks.kieu', 'smks.ks.viTri'].forEach((k) => localStorage.removeItem(k)); return 1;`);
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
