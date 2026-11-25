// Kiểm vị trí ảnh so với chữ trong kiểu "Ảnh + thông tin" trên file chạy thật (bản `--features dev-keys`, khoá thử ở
// SMKS_DEV_PKI): chuột phải vào khung › Vị trí ảnh (Trái / Phải / Trên / Dưới) đổi khung xem trước và `config.json`;
// hộp Cấu hình có cùng lựa chọn; ký thật với bố cục "Dưới" và "Phải" rồi đo trên ảnh trang PDF (PyMuPDF) mực xanh
// của ảnh chữ ký nằm đúng phía so với chữ đen.
//
// Cách dùng: node app/tests/ks-bocuc.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử> <ảnh .png mực xanh>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki, image] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9357;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const shots = path.join(work, 'anh');
const dirA = path.join(work, 'a');
const dirB = path.join(work, 'b');
const dirC = path.join(work, 'c');
const dirD = path.join(work, 'd');
for (const d of [cfgDir, shots, dirA, dirB, dirC, dirD]) fs.mkdirSync(d, { recursive: true });
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirA, 'van ban a.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirB, 'van ban b.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirC, 'van ban c.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirD, 'van ban d.pdf'));
const readCfg = () => JSON.parse(fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
const app = spawn(exe, [], {
  env: { ...process.env, SMKS_DEV_PKI: pki, SMKS_CONFIG_DIR: cfgDir, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
  stdio: 'ignore',
});

/**
 * Vẽ vùng khối chữ ký của file đã ký (PyMuPDF, 4 lần) rồi tính trọng tâm điểm mực xanh (ảnh chữ ký) và điểm đen
 * (chữ "Ký bởi / Ngày ký") theo tỉ lệ khối.
 */
const inkOf = (file) => {
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, pymupdf
d = pymupdf.open(sys.argv[1])
p = d[0]
w = [x for x in p.widgets() if x.field_type == pymupdf.PDF_WIDGET_TYPE_SIGNATURE][0]
r = w.rect
pix = p.get_pixmap(matrix=pymupdf.Matrix(4, 4), clip=r)
blue = [0, 0, 0, 10**9, -1]; black = [0, 0, 0, 10**9, -1]; cols = [0] * 100
for y in range(pix.height):
    for x in range(pix.width):
        c = pix.pixel(x, y)
        R, G, B = c[0], c[1], c[2]
        a = blue if (B > 110 and B - R > 60 and B - G > 50) else black if (R < 70 and G < 70 and B < 70) else None
        if a is not None:
            a[0] += x; a[1] += y; a[2] += 1; a[3] = min(a[3], x); a[4] = max(a[4], x)
            cols[x * 100 // pix.width] += 1
f = lambda a: {"x": round(a[0] / a[2] / pix.width, 3), "y": round(a[1] / a[2] / pix.height, 3), "n": a[2],
               "x0": round(a[3] / pix.width, 3), "x1": round(a[4] / pix.width, 3)} if a[2] else None
print(json.dumps({"w": round(r.width, 1), "h": round(r.height, 1), "blue": f(blue), "black": f(black), "cols": cols}))`, file], { encoding: 'utf8' });
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
  const setField = (sel, value) =>
    run(`const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return false;
         el.value = ${JSON.stringify(String(value))}; el.dispatchEvent(new Event('change', { bubbles: true })); return true;`);
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
  const rightClickBox = async () => {
    const c = await run(`const b = document.querySelector('.giay .chuky-o'); b.scrollIntoView({ block: 'center' }); await new Promise((r) => setTimeout(r, 200));
      const r = b.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 };`);
    await mouse('mouseMoved', c.x, c.y);
    await mouse('mousePressed', c.x, c.y, 'right');
    await mouse('mouseReleased', c.x, c.y, 'right');
    return waitFor(`return !!document.querySelector('.ky-menu .km-bocuc') && 1`, 5000);
  };
  const pickLayout = (label) =>
    run(`const b = [...document.querySelectorAll('.ky-menu .km-bocuc .seg')].find((x) => x.textContent.trim() === ${JSON.stringify(label)}); if (!b) return false; b.click(); return true;`);
  /** Khung xem trước: tỉ lệ khung, hướng xếp, chỗ ảnh so với chữ (tâm, px). */
  const preview = () =>
    run(`const o = document.querySelector('.giay .chuky-o'); const c = o?.querySelector('.chuky'); const img = c?.querySelector('img'); const t = c?.querySelector('.tt-ky');
      if (!o || !img || !t || !img.complete || !img.naturalWidth) return null;
      const s = o.parentElement.getBoundingClientRect(); const r = o.getBoundingClientRect(); const a = img.getBoundingClientRect(); const b = t.getBoundingClientRect();
      return { cls: [...c.classList].filter((x) => x.startsWith('bo-')).join(''), dir: getComputedStyle(c).flexDirection,
        wPt: Math.round((r.width / s.width) * 595.32), hPt: Math.round((r.height / s.height) * 841.92),
        imgX: Math.round(a.left + a.width / 2 - r.left), imgY: Math.round(a.top + a.height / 2 - r.top), txtX: Math.round(b.left + b.width / 2 - r.left), txtY: Math.round(b.top + b.height / 2 - r.top),
        inside: a.left >= r.left - 1 && a.right <= r.right + 1 && a.top >= r.top - 1 && a.bottom <= r.bottom + 1,
        textFits: t.scrollWidth <= t.clientWidth + 1 && b.left >= r.left - 1 && b.right <= r.right + 1 };`);
  const layoutIs = (cls) => waitFor(`const c = document.querySelector('.giay .chuky-o .chuky'); const i = c?.querySelector('img'); return !!c && c.classList.contains(${JSON.stringify(cls)}) && i?.complete && i.naturalWidth > 0 && 1`, 10000);
  const dropFolder = async (folder) => {
    await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(folder)}], position: { x: 500, y: 400 } } }); return 1;`);
    await waitFor(`return document.querySelectorAll('.ds .dong').length === 1 && !!document.querySelector('.giay .chuky-o img') && 1`, 20000);
    await sleep(800);
  };
  const signAll = async () => {
    await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((b) => !b.disabled)?.click(); return 1;`);
    await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
    await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
       r?.querySelector('input')?.click(); return !!r;`);
    await run(`[...document.querySelectorAll('app-popup button.nut.chinh')].find((b) => b.textContent.includes('Ký số'))?.click(); return 1;`);
    const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((b) => b.textContent).join('/')`, 120000);
    await run(`[...document.querySelectorAll('app-popup button')].find((b) => b.textContent.includes('Đóng'))?.click(); return 1;`);
    await sleep(1200);
    return done;
  };
  await send('Runtime.enable');

  // Ảnh chữ ký và kiểu "Ảnh + thông tin" (cấu hình riêng của bài kiểm).
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`const c = await window.__TAURI_INTERNALS__.invoke('config_defaults');
             c.signature.imagePath = ${JSON.stringify(image)}; c.signature.display = 'imageAndText'; c.signature.imageKind = 'wetSignature';
             await window.__TAURI_INTERNALS__.invoke('config_save', { config: c }); return 1;`);
  await send('Page.reload');
  await sleep(1500);
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(500);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title)`);
  await sleep(1200);
  await dropFolder(dirA);

  const p0 = await preview();
  check('Mặc định: ảnh bên trái chữ, khung 170 × 30 pt', !!p0 && p0.cls === 'bo-left' && p0.dir === 'row' && p0.imgX < p0.txtX && p0.wPt === 170 && p0.hPt === 30, JSON.stringify(p0));

  const menu = await rightClickBox();
  const labels = await run(`return [...document.querySelectorAll('.ky-menu .km-bocuc .seg')].map((b) => b.textContent.trim() + (b.classList.contains('chon') ? '*' : '')).join(' ')`);
  check('Chuột phải vào khung: menu có "Vị trí ảnh" Trái / Phải / Trên / Dưới, đang chọn Trái', !!menu && labels === 'Trái* Phải Trên Dưới', String(labels));
  await shot('1-menu-vi-tri-anh.png');

  await pickLayout('Phải');
  await layoutIs('bo-right');
  await sleep(400);
  const pR = await preview();
  check('Chọn "Phải": ảnh bên phải chữ, khung giữ 170 × 30 pt, ảnh nằm trong khung', !!pR && pR.dir === 'row-reverse' && pR.imgX > pR.txtX && pR.wPt === 170 && pR.inside, JSON.stringify(pR));

  // Bấm "Trên" đúng lúc cấu hình của lần bấm "Phải" đang được lưu (lưu sau 0,4 giây): lựa chọn mới không được mất.
  await pickLayout('Trên');
  await layoutIs('bo-top');
  await waitFor(`const o = document.querySelector('.giay .chuky-o'); const s = o?.parentElement.getBoundingClientRect(); const r = o?.getBoundingClientRect(); return r && Math.round((r.height / s.height) * 841.92) === 66 && 1`, 10000);
  await sleep(400);
  const pT = await preview();
  check('Chọn "Trên": khung đổi thành 130 × 66 pt, ảnh ở trên chữ, trong khung; chữ thu nhỏ cho vừa, không bị cắt',
    !!pT && pT.dir === 'column' && pT.imgY < pT.txtY && pT.wPt === 130 && pT.hPt === 66 && pT.inside && pT.textFits, JSON.stringify(pT));
  await shot('2-anh-tren.png');

  await pickLayout('Dưới');
  await layoutIs('bo-bottom');
  await sleep(400);
  const pB = await preview();
  check('Chọn "Dưới": chữ ở trên, ảnh ở dưới', !!pB && pB.dir === 'column-reverse' && pB.imgY > pB.txtY && pB.hPt === 66, JSON.stringify(pB));
  const sel = await run(`return [...document.querySelectorAll('.ky-menu .km-bocuc .seg.chon')].map((b) => b.textContent.trim()).join()`);
  await run(`document.body.click(); return 1;`);
  await sleep(800);
  check('Menu vẫn mở khi đổi, nút đang chọn là "Dưới"; config.json có imageLayout "bottom"', sel === 'Dưới' && readCfg().signature.imageLayout === 'bottom', `${sel}; ${readCfg().signature.imageLayout}`);

  // Hộp Cấu hình có cùng lựa chọn, khung thu nhỏ theo bố cục.
  await run(`[...document.querySelectorAll('.menubar button.mb')].find((b) => b.textContent.includes('Cấu hình'))?.click(); return 1;`);
  await waitFor(`return !!document.querySelector('.popup.cauhinh')`);
  await run(`[...document.querySelectorAll('.popup.cauhinh .tabch button')].find((b) => b.textContent.includes('Chữ ký'))?.click(); return 1;`);
  await sleep(200);
  const cfgChips = await waitFor(`const g = [...document.querySelectorAll('.popup.cauhinh .cfg-kho')].find((x) => x.textContent.includes('Trái'));
     return g && [...g.querySelectorAll('.chip')].map((c) => c.textContent.trim() + (c.classList.contains('chon') ? '*' : '')).join(' ') + ' | ' + document.querySelector('.popup.cauhinh .cfg-o')?.className`, 8000);
  check('Hộp Cấu hình: "Vị trí ảnh" Trái / Phải / Trên / Dưới đang chọn Dưới; khung xem trước theo bố cục', /Trái Phải Trên Dưới\*/.test(cfgChips ?? '') && /bo-bottom/.test(cfgChips ?? ''), String(cfgChips));
  await shot('3-hop-cau-hinh.png');
  await run(`[...document.querySelectorAll('.popup.cauhinh .cuoi button')].find((b) => b.textContent.includes('Hủy'))?.click(); return 1;`);
  await sleep(500);

  // Ký thật với "Dưới".
  const doneB = await signAll();
  const inkB = inkOf(path.join(`${dirA}_DaKySo`, 'van ban a_signed.pdf'));
  check('Ký với "Dưới": khối 130 × 66 pt; trong PDF mực xanh của ảnh nằm dưới chữ đen',
    doneB === '1/1/0' && Math.round(inkB.w) === 130 && Math.round(inkB.h) === 66 && !!inkB.blue && !!inkB.black && inkB.blue.y > 0.5 && inkB.black.y < 0.5,
    `${doneB} ${JSON.stringify(inkB)}`);

  // Ký thật với "Phải".
  await dropFolder(dirB);
  await rightClickBox();
  await pickLayout('Phải');
  await run(`document.body.click(); return 1;`);
  await layoutIs('bo-right');
  await sleep(800);
  const doneR = await signAll();
  const inkR = inkOf(path.join(`${dirB}_DaKySo`, 'van ban b_signed.pdf'));
  check('Ký với "Phải": khối 170 × 30 pt; trong PDF mực xanh của ảnh nằm bên phải chữ đen',
    doneR === '1/1/0' && Math.round(inkR.w) === 170 && Math.round(inkR.h) === 30 && !!inkR.blue && !!inkR.black && inkR.blue.x > 0.6 && inkR.black.x < 0.45,
    `${doneR} ${JSON.stringify(inkR)}`);

  // Màn "Cấu hình vị trí" (TD-51): vị trí ảnh, phần ảnh chiếm, khoảng cách; sửa trên bản nháp, Áp dụng mới lưu.
  await dropFolder(dirC);
  await run(`[...document.querySelectorAll('.nguon button.nut')].find((b) => b.textContent.includes('Cấu hình vị trí'))?.click(); return 1;`);
  await waitFor(`return !!document.querySelector('.vi-tri .hang') && 1`);
  await run(`const b = document.querySelector('.vi-tri .nangcao'); if (b && !b.classList.contains('mo')) b.click(); return 1;`);
  const seg0 = await waitFor(`const g = document.querySelector('.vi-tri .vt-bocuc'); return g && [...g.querySelectorAll('.seg')].map((b) => b.textContent.trim() + (b.classList.contains('chon') ? '*' : '')).join(' ') + ' | ' +
     [...document.querySelectorAll('.vi-tri .vt-so')].map((x) => x.textContent.trim()).join(' / ')`, 10000);
  check('Màn Cấu hình vị trí (Nâng cao): có Vị trí ảnh Trái / Phải* / Trên / Dưới / Tự do, Ảnh chiếm và Khoảng cách đang "Tự động"',
    seg0 === 'Trái Phải* Trên Dưới Tự do | Tự động / Tự động', String(seg0));
  const setRange = (cls, v) => run(`const r = document.querySelector('.vi-tri ${cls}'); r.value = '${v}'; r.dispatchEvent(new Event('input', { bubbles: true })); return 1;`);
  await run(`[...document.querySelectorAll('.vi-tri .vt-bocuc .seg')].find((b) => b.textContent.trim() === 'Trái')?.click(); return 1;`);
  await setRange('.vt-anh', 20);
  await setRange('.vt-khoang', 20);
  const vt = await waitFor(`const c = document.querySelector('.vi-tri .giay .chuky-o .chuky'); const i = c?.querySelector('img'); const t = c?.querySelector('.tt-ky');
     if (!c || !i || !t || !c.classList.contains('bo-left') || !c.classList.contains('tu-chia') || !i.complete) return null;
     const r = c.getBoundingClientRect(); const a = i.getBoundingClientRect(); const b = t.getBoundingClientRect();
     return { img: +(a.width / r.width).toFixed(3), gap: +((b.left - a.right) / r.width).toFixed(3), so: [...document.querySelectorAll('.vi-tri .vt-so')].map((x) => x.textContent.trim()).join(' / ') };`, 10000);
  check('Kéo Ảnh chiếm 20%, Khoảng cách 20%: xem trước ảnh rộng ~20% khung, chữ cách ảnh ~20%',
    !!vt && Math.abs(vt.img - 0.2) < 0.02 && Math.abs(vt.gap - 0.2) < 0.02 && vt.so === '20% / 20%', JSON.stringify(vt));
  await shot('4-cau-hinh-vi-tri.png');
  check('Chưa Áp dụng: config.json chưa đổi (vẫn Phải, chưa có imageShare)',
    readCfg().signature.imageLayout === 'right' && readCfg().signature.imageShare == null, JSON.stringify({ l: readCfg().signature.imageLayout, s: readCfg().signature.imageShare }));
  await run(`[...document.querySelectorAll('.vi-tri button')].find((b) => b.textContent.includes('Áp dụng'))?.click(); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.giay .chuky-o .chuky.tu-chia') && 1`, 10000);
  await sleep(900);
  const sig = readCfg().signature;
  check('Áp dụng: config.json có imageLayout "left", imageShare 20, imageGap 20; màn Ký số xem trước theo cách chia mới',
    sig.imageLayout === 'left' && sig.imageShare === 20 && sig.imageGap === 20, JSON.stringify({ l: sig.imageLayout, s: sig.imageShare, g: sig.imageGap }));
  const doneC = await signAll();
  const inkC = inkOf(path.join(`${dirC}_DaKySo`, 'van ban c_signed.pdf'));
  // Cột có mực (theo % bề ngang khối): ảnh trong 0–20%, bỏ trống 20–40% (khoảng cách), chữ bắt đầu ngay sau 40%.
  const textStart = inkC.cols.findIndex((n, i) => i >= 21 && n > 0);
  const imageEnd = inkC.cols.slice(0, 21).findLastIndex((n) => n > 0);
  check('Ký với ảnh 20% / cách 20%: trong PDF ảnh nằm trong 20% đầu khối, khoảng 20–40% trống, chữ bắt đầu từ ~40% khối',
    doneC === '1/1/0' && !!inkC.blue && inkC.blue.x1 <= 0.21 && imageEnd >= 0 && imageEnd <= 20 && textStart >= 40 && textStart <= 42,
    `${doneC} ảnh tới ${imageEnd}%, chữ từ ${textStart}%; ${JSON.stringify({ ...inkC, cols: undefined })}`);

  // Kiểu "Tự do" (TD-52): ô sửa phóng to, kéo ô ảnh / ô thông tin bằng chuột thật; Áp dụng mới lưu; ký ra đúng chỗ.
  await dropFolder(dirD);
  await run(`[...document.querySelectorAll('.nguon button.nut')].find((b) => b.textContent.includes('Cấu hình vị trí'))?.click(); return 1;`);
  await waitFor(`return !!document.querySelector('.vi-tri .vt-bocuc') && 1`, 10000);
  await run(`[...document.querySelectorAll('.vi-tri .vt-bocuc .seg')].find((b) => b.textContent.trim() === 'Tự do')?.click(); return 1;`);
  const ed0 = await waitFor(`const e = document.querySelector('.vi-tri .vt-sua'); const a = e?.querySelector('.vt-o.anh'); const c = e?.querySelector('.vt-o.chu');
     if (!e || !a || !c) return null; const r = e.getBoundingClientRect(); const ra = a.getBoundingClientRect(); const rc = c.getBoundingClientRect();
     return { w: Math.round(r.width), h: Math.round(r.height), anh: [+((ra.left - r.left) / r.width).toFixed(2), +(ra.width / r.width).toFixed(2)],
              chu: [+((rc.left - r.left) / r.width).toFixed(2), +(rc.width / r.width).toFixed(2)], seg: document.querySelector('.vi-tri .vt-bocuc .seg.chon')?.textContent.trim(),
              thanh: !!document.querySelector('.vi-tri .vt-anh') };`, 10000);
  check('Bấm "Tự do": hiện ô sửa phóng to (đúng tỉ lệ khối 170 × 30), bắt đầu từ cách chia đang dùng (ảnh 0–20%, chữ từ 40%), ẩn hai thanh',
    !!ed0 && ed0.seg === 'Tự do' && Math.abs(ed0.w / ed0.h - 170 / 30) < 0.3 && ed0.anh[0] === 0 && ed0.anh[1] === 0.2 && ed0.chu[0] === 0.4 && !ed0.thanh, JSON.stringify(ed0));
  // Kéo bằng chuột (giữ nút trái trong lúc di như ks-sigbox): ô thông tin sang trái sát mép (−40%), ô ảnh sang phải (+80%).
  const hold = (type, x, y) => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1 });
  const dragBy = async (sel, fx) => {
    const c = await run(`const e = document.querySelector('.vi-tri .vt-sua'); const o = e.querySelector('${sel}'); const r = o.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: e.getBoundingClientRect().width };`);
    const to = c.x + fx * c.w;
    await hold('mouseMoved', c.x, c.y);
    await hold('mousePressed', c.x, c.y);
    for (let i = 1; i <= 8; i++) await hold('mouseMoved', c.x + ((to - c.x) * i) / 8, c.y);
    await hold('mouseReleased', to, c.y);
    await sleep(300);
  };
  // Kéo ảnh trước (ô chữ đang ở 40–100%, chưa đè lên ảnh), rồi kéo ô chữ; ô vừa bấm nằm trên.
  await dragBy('.vt-o.anh', 0.8);
  const onTop1 = await run(`return document.querySelector('.vi-tri .vt-o.anh').classList.contains('tren')`);
  await dragBy('.vt-o.chu', -0.4);
  const onTop2 = await run(`return document.querySelector('.vi-tri .vt-o.chu').classList.contains('tren') && !document.querySelector('.vi-tri .vt-o.anh').classList.contains('tren')`);
  // Kéo góc ô thông tin: cao bằng nửa khối (−50% chiều cao).
  const corner = await run(`const e = document.querySelector('.vi-tri .vt-sua'); const t = e.querySelector('.vt-o.chu .tay').getBoundingClientRect();
    return { x: t.left + t.width / 2, y: t.top + t.height / 2, h: e.getBoundingClientRect().height };`);
  await hold('mouseMoved', corner.x, corner.y);
  await hold('mousePressed', corner.x, corner.y);
  for (let i = 1; i <= 6; i++) await hold('mouseMoved', corner.x, corner.y - (corner.h * 0.5 * i) / 6);
  await hold('mouseReleased', corner.x, corner.y - corner.h * 0.5);
  await sleep(400);
  const ed1 = await run(`const e = document.querySelector('.vi-tri .vt-sua'); const r = e.getBoundingClientRect();
    const f = (s) => { const b = e.querySelector(s).getBoundingClientRect(); return [(b.left - r.left) / r.width, (b.top - r.top) / r.height, b.width / r.width, b.height / r.height].map((v) => +v.toFixed(2)); };
    const p = document.querySelector('.vi-tri .giay .chuky-o .chuky'); return { anh: f('.vt-o.anh'), chu: f('.vt-o.chu'), trang: p?.classList.contains('tu-do') ?? false };`);
  check('Kéo thả: ô ảnh sang phải (80–100%), ô thông tin sang trái (0–60%), kéo góc còn nửa chiều cao; ô vừa bấm nằm trên; khung trên trang đổi theo (tu-do)',
    !!ed1 && onTop1 && onTop2 && ed1.anh[0] === 0.8 && ed1.chu[0] === 0 && ed1.chu[2] === 0.6 && Math.abs(ed1.chu[3] - 0.5) < 0.03 && ed1.trang, JSON.stringify(ed1));
  await shot('5-tu-do.png');
  // Hiển thị chỉ thông tin / chỉ ảnh: hàng Vị trí ảnh và ô sửa vẫn hiện (chỉnh sẵn), ô không dùng mờ đi, có dòng nhắc.
  await setField('.vi-tri select.vt-kieu', 'text');
  await sleep(300);
  const hienT = await run(`return { hang: !!document.querySelector('.vi-tri .vt-bocuc'), sua: !!document.querySelector('.vi-tri .vt-sua'),
    nhac: !!document.querySelector('.vi-tri .vt-chidung'), moAnh: !!document.querySelector('.vi-tri .vt-o.anh.mo'), tayKhoi: !!document.querySelector('.vi-tri .vt-sua > .tay.khoi') };`);
  await setField('.vi-tri select.vt-kieu', 'image');
  await sleep(300);
  const hienA = await run(`return { hang: !!document.querySelector('.vi-tri .vt-bocuc'), sua: !!document.querySelector('.vi-tri .vt-sua'),
    moChu: !!document.querySelector('.vi-tri .vt-o.chu.mo'), tayKhoi: !!document.querySelector('.vi-tri .vt-sua > .tay.khoi') };`);
  await setField('.vi-tri select.vt-kieu', 'imageAndText');
  await sleep(300);
  const hienB = await run(`return { nhac: !!document.querySelector('.vi-tri .vt-chidung'), mo: document.querySelectorAll('.vi-tri .vt-o.mo').length };`);
  check('Hiển thị "Chỉ thông tin" / "Chỉ ảnh": vẫn hiện hàng Vị trí ảnh và ô sửa, ô không dùng mờ đi, có dòng nhắc; về "Ảnh + thông tin" thì hết',
    hienT.hang && hienT.sua && hienT.nhac && hienT.moAnh && !hienT.tayKhoi && hienA.hang && hienA.sua && hienA.moChu && hienA.tayKhoi && !hienB.nhac && hienB.mo === 0,
    JSON.stringify({ hienT, hienA, hienB }));
  // Kéo góc khung ngoài của ô sửa: thu cả khối còn 80% rộng / cao; ô sửa đi theo chuột, khung trên trang đổi cỡ, hai ô giữ tỉ lệ.
  const blkInfo = `const e = document.querySelector('.vi-tri .vt-sua'); const r = e.getBoundingClientRect(); const t = e.querySelector(':scope > .tay.khoi')?.getBoundingClientRect();
    const g = document.querySelector('.vi-tri .giay'); const b = g.querySelector('.chuky-o').getBoundingClientRect(); const gr = g.getBoundingClientRect();
    const c = e.querySelector('.vt-o.chu').getBoundingClientRect();
    return { x: t ? t.left + t.width / 2 : null, y: t ? t.top + t.height / 2 : null, w: r.width, h: r.height, pw: b.width, ph: b.height,
             wr: b.width / gr.width, hr: b.height / gr.height, chuW: +(c.width / r.width).toFixed(2) };`;
  const blk0 = await run(blkInfo);
  await hold('mouseMoved', blk0.x, blk0.y);
  await hold('mousePressed', blk0.x, blk0.y);
  for (let i = 1; i <= 6; i++) await hold('mouseMoved', blk0.x - (blk0.w * 0.2 * i) / 6, blk0.y - (blk0.h * 0.2 * i) / 6);
  await sleep(300);
  const blkMid = await run(blkInfo);
  await hold('mouseReleased', blk0.x - blk0.w * 0.2, blk0.y - blk0.h * 0.2);
  await sleep(500);
  const blk1 = await run(blkInfo);
  check('Kéo góc khung ngoài: khối còn ~80% rộng và cao trên trang, ô sửa đi theo chuột rồi vừa lại khi thả, ô thông tin vẫn 60% bề ngang',
    blk0.x != null && Math.abs(blkMid.w / blk0.w - 0.8) < 0.03 && Math.abs(blk1.pw / blk0.pw - 0.8) < 0.03 && Math.abs(blk1.ph / blk0.ph - 0.8) < 0.03 &&
      Math.abs(blk1.w - blk0.w) < 2 && blk1.chuW === 0.6,
    JSON.stringify({ blk0, blkMid, blk1 }));
  // Kéo theo cạnh (chỉ một chiều): giữ nút trái, kéo tâm tay nắm đi (dx, dy) px.
  const dragHandle = async (selJs, dx, dy, mid) => {
    const c = await run(`const t = ${selJs}?.getBoundingClientRect(); return t ? { x: t.left + t.width / 2, y: t.top + t.height / 2 } : null;`);
    if (!c) return null;
    await hold('mouseMoved', c.x, c.y);
    await hold('mousePressed', c.x, c.y);
    for (let i = 1; i <= 6; i++) await hold('mouseMoved', c.x + (dx * i) / 6, c.y + (dy * i) / 6);
    await sleep(250);
    const during = mid ? await run(mid) : null;
    await hold('mouseReleased', c.x + dx, c.y + dy);
    await sleep(500);
    return { c, during };
  };
  const pageBox = `const g = document.querySelector('.vi-tri .giay'); const b = g.querySelector('.chuky-o').getBoundingClientRect(); const gr = g.getBoundingClientRect();
    return { l: b.left - gr.left, r: b.right - gr.left, t: b.top - gr.top, b: b.bottom - gr.top, w: b.width, h: b.height };`;
  // 1) Cạnh trái khung ngoài kéo sang trái 25% ô sửa: khối rộng thêm 25%, cao giữ nguyên, cạnh phải đứng yên (trên trang và trong ô sửa).
  const pg0 = await run(pageBox);
  const sua0 = await run(`const r = document.querySelector('.vi-tri .vt-sua').getBoundingClientRect(); return { l: r.left, r: r.right, w: r.width };`);
  const kl = await dragHandle(`document.querySelector('.vi-tri .vt-sua > .tay.khoi.l')`, -0.25 * sua0.w, 0,
    `const r = document.querySelector('.vi-tri .vt-sua').getBoundingClientRect(); return { l: r.left, r: r.right, w: r.width };`);
  const pg1 = await run(pageBox);
  check('Kéo cạnh trái khung ngoài sang ngang: khối rộng thêm ~25%, cao giữ nguyên, cạnh phải đứng yên; trong lúc kéo ô sửa giãn sang trái theo chuột',
    !!kl && Math.abs(pg1.w / pg0.w - 1.25) < 0.04 && Math.abs(pg1.h - pg0.h) < 1 && Math.abs(pg1.r - pg0.r) < 1.5 &&
      Math.abs(kl.during.r - sua0.r) < 2 && Math.abs(kl.during.w / sua0.w - 1.25) < 0.03,
    JSON.stringify({ pg0, pg1, sua0, during: kl?.during }));
  // 2) Cạnh trên của ô thông tin kéo xuống 10% chiều cao: y 0 → 0,1, cao 0,5 → 0,4, bề ngang không đổi.
  const hSua = await run(`return document.querySelector('.vi-tri .vt-sua').getBoundingClientRect().height`);
  await run(`document.querySelector('.vi-tri .vt-o.chu').dispatchEvent(new PointerEvent('pointerover')); return 1;`);
  const ct = await dragHandle(`document.querySelector('.vi-tri .vt-o.chu .tay.t')`, 0, 0.1 * hSua);
  const ed2 = await run(`const e = document.querySelector('.vi-tri .vt-sua'); const r = e.getBoundingClientRect(); const b = e.querySelector('.vt-o.chu').getBoundingClientRect();
    return [(b.left - r.left) / r.width, (b.top - r.top) / r.height, b.width / r.width, b.height / r.height].map((v) => +v.toFixed(2));`);
  check('Kéo cạnh trên ô thông tin theo chiều dọc: y 0,1, cao 0,4, x và bề ngang giữ nguyên (0 / 0,6)',
    !!ct && ed2[0] === 0 && ed2[2] === 0.6 && Math.abs(ed2[1] - 0.1) < 0.02 && Math.abs(ed2[3] - 0.4) < 0.02, JSON.stringify(ed2));
  // 3) Khung trên trang: cạnh trên kéo lên 10 px, chỉ đổi chiều cao.
  const pg2 = await run(pageBox);
  const pt = await dragHandle(`document.querySelector('.vi-tri .giay .chuky-o .tay.t')`, 0, -10);
  const pg3 = await run(pageBox);
  check('Khung trên trang: kéo cạnh trên lên 10 px thì cao thêm 10 px, bề ngang và cạnh dưới giữ nguyên',
    !!pt && Math.abs(pg3.h - pg2.h - 10) < 1.5 && Math.abs(pg3.w - pg2.w) < 1 && Math.abs(pg3.b - pg2.b) < 1.5, JSON.stringify({ pg2, pg3 }));
  const blkF = await run(blkInfo);
  await shot('6-keo-canh.png');
  check('Chưa Áp dụng: config.json chưa có imageBox', readCfg().signature.imageBox == null, JSON.stringify(readCfg().signature.imageBox));
  await run(`[...document.querySelectorAll('.vi-tri button')].find((b) => b.textContent.includes('Áp dụng'))?.click(); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.giay .chuky-o .chuky.tu-do') && 1`, 10000);
  await sleep(900);
  const sigD = readCfg().signature;
  const posD = sigD.positions?.[0];
  check('Áp dụng: cỡ khối vừa kéo được lưu (positions)', !!posD && Math.abs(posD.widthRatio - blkF.wr) < 0.005 && Math.abs(posD.heightRatio - blkF.hr) < 0.005,
    JSON.stringify({ posD, wr: blkF.wr, hr: blkF.hr }));
  check('Áp dụng: config.json có imageBox (x 0,8, rộng 0,2) và textBox (x 0, rộng 0,6, y ~0,1, cao ~0,4)',
    sigD.imageBox && sigD.textBox && Math.abs(sigD.imageBox.x - 0.8) < 0.01 && Math.abs(sigD.imageBox.w - 0.2) < 0.01 && sigD.textBox.x === 0 && Math.abs(sigD.textBox.w - 0.6) < 0.01 &&
      Math.abs(sigD.textBox.y - 0.1) < 0.03 && Math.abs(sigD.textBox.h - 0.4) < 0.03,
    JSON.stringify({ i: sigD.imageBox, t: sigD.textBox }));
  const doneD = await signAll();
  const inkD = inkOf(path.join(`${dirD}_DaKySo`, 'van ban d_signed.pdf'));
  const firstImg = inkD.cols.findIndex((n, i) => i >= 61 && n > 0);
  const lastTxt = inkD.cols.slice(0, 61).findLastIndex((n) => n > 0);
  check('Ký với kiểu Tự do: trong PDF chữ nằm trong 0–60% bề ngang, ảnh (mực xanh) trong 80–100%, 60–80% trống',
    doneD === '1/1/0' && !!inkD.blue && inkD.blue.x0 >= 0.79 && lastTxt >= 0 && lastTxt <= 60 && firstImg >= 79,
    `${doneD} chữ tới ${lastTxt}%, ảnh từ ${firstImg}%; ${JSON.stringify({ ...inkD, cols: undefined })}`);

  check('Không có lỗi trong console của giao diện', consoleErrors.length === 0, consoleErrors.slice(0, 3).join(' | '));
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  spawnSync('taskkill', ['/PID', String(app.pid)], { stdio: 'ignore' });
  for (let i = 0; i < 40 && app.exitCode === null; i++) await sleep(250);
  if (app.exitCode === null) app.kill();
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
