// Kiểm màn "Xem trước & vị trí chữ ký" (nút "Cấu hình vị trí" của màn Ký số) trên app thật, bản build
// `--features dev-keys` (khoá thử ở SMKS_DEV_PKI). Cấu hình ghi vào thư mục riêng (SMKS_CONFIG_DIR).
// Kéo khung bằng chuột thật (CDP): một khung cho tất cả file, khung riêng từng file, thanh Cỡ, Theo mẫu, Áp dụng /
// Thoát; ký 3 file rồi đo vị trí khối chữ ký trong từng file đã ký (PyMuPDF).
//
// Cách dùng: node app/tests/ks-vitri.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử> <ảnh .png>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki, image] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9355;
const MM = 72 / 25.4;
const A4 = { w: 595.32, h: 841.92 };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
const near = (a, b, tol = 0.006) => Math.abs(a - b) < tol;

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const src = path.join(work, 'vb');
const shots = path.join(work, 'anh');
for (const d of [cfgDir, src, shots]) fs.mkdirSync(d, { recursive: true });
const names = ['van ban 1.pdf', 'van ban 2.pdf', 'van ban 3.pdf'];
fs.copyFileSync(path.join(repo, 'fixtures/pdf/word-hybrid-xref.pdf'), path.join(src, names[0]));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, names[1]));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/xref-stream.pdf'), path.join(src, names[2]));
const cfgFile = path.join(cfgDir, 'config.json');
const readCfg = () => JSON.parse(fs.readFileSync(cfgFile, 'utf8'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
try {
  await fetch(`http://127.0.0.1:${debugPort}/json`);
  console.error(`LOI: cổng ${debugPort} đang bận`);
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
    const m = JSON.parse(event.data);
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m);
      pending.delete(m.id);
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
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: String(e?.message ?? e) }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    const v = reply.result?.result?.value;
    return v?.err ? { __err: v.err } : v?.ok;
  };
  const waitFor = async (body, ms = 20000) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      const v = await run(body);
      if (v && !v.__err) return v;
      await sleep(250);
    }
    return null;
  };
  for (let i = 0; i < 40; i++) {
    if (await run('return !!window.__TAURI_INTERNALS__')) break;
    await sleep(250);
  }
  const inv = (cmd, args = {}) => run(`return await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);
  const shot = async (name) => {
    const r = await send('Page.captureScreenshot', { format: 'png' });
    if (r.result?.data) fs.writeFileSync(path.join(shots, name), Buffer.from(r.result.data, 'base64'));
  };
  const clickText = (sel, text) =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  const setField = (sel, value, ev = 'change', index = 0) =>
    run(`const el = document.querySelectorAll(${JSON.stringify(sel)})[${index}]; if (!el) return false;
         el.value = ${JSON.stringify(String(value))}; el.dispatchEvent(new Event(${JSON.stringify(ev)}, { bubbles: true })); return true;`);
  const boxes = () =>
    run(`return [...document.querySelectorAll('.trang-ds > .giay')].flatMap((s, i) => [...s.querySelectorAll('.chuky-o')].map((b) => {
        const r = b.getBoundingClientRect(), p = s.getBoundingClientRect();
        return { page: i + 1, x: (r.left - p.left) / p.width, y: (r.top - p.top) / p.height, w: r.width / p.width, h: r.height / p.height,
                 cx: r.left + r.width / 2, cy: r.top + r.height / 2 }; }));`);
  const minis = () =>
    run(`return [...document.querySelectorAll('.vi-tri .ab-file')].map((f) => ({ name: f.querySelector('.ten')?.textContent, chon: f.classList.contains('chon'),
        tick: !!f.querySelector('.tick'), o: [...f.querySelectorAll('.o-ky')].map((o) => ({ x: parseFloat(o.style.left) / 100, y: parseFloat(o.style.top) / 100,
        w: parseFloat(o.style.width) / 100, h: parseFloat(o.style.height) / 100 })) }));`);
  const foot = () => run(`return document.querySelector('.vi-tri .chan .dan')?.textContent.replace(/\\s+/g, ' ').trim()`);
  const mouse = (type, x, y) => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1 });
  const drag = async (dx, dy) => {
    await run(`document.querySelector('.trang-ds > .giay .chuky-o')?.scrollIntoView({ block: 'center' }); return 1;`);
    await sleep(300);
    const b = (await boxes())?.[0];
    if (!b) return false;
    await mouse('mouseMoved', b.cx, b.cy);
    await mouse('mousePressed', b.cx, b.cy);
    for (let i = 1; i <= 8; i++) await mouse('mouseMoved', b.cx + (dx * i) / 8, b.cy + (dy * i) / 8);
    await mouse('mouseReleased', b.cx + dx, b.cy + dy);
    await sleep(900);
    return true;
  };
  // Khung chữ ký nằm trọn trong vùng xem, ở giữa (hoặc vùng xem đã cuộn hết cỡ về phía khung).
  const focused = () =>
    run(`const v = document.querySelector('.vungxem'); const b = document.querySelector('.trang-ds > .giay .chuky-o'); if (!v || !b) return null;
      const vr = v.getBoundingClientRect(); const r = b.getBoundingClientRect();
      const dy = r.top + r.height / 2 - (vr.top + v.clientHeight / 2);
      const atEnd = (dy > 0 && v.scrollTop >= v.scrollHeight - v.clientHeight - 1) || (dy < 0 && v.scrollTop <= 0);
      const inView = r.top >= vr.top - 1 && r.bottom <= vr.top + v.clientHeight + 1;
      return { ok: inView && (Math.abs(dy) < 30 || atEnd), dy: Math.round(dy), scrollTop: Math.round(v.scrollTop),
        page: [...document.querySelectorAll('.trang-ds > .giay')].indexOf(b.parentElement) + 1 };`);
  const sameBox = (a, b, tol = 0.006) => !!a && !!b && near(a.x, b.x, tol) && near(a.y, b.y, tol) && near(a.w, b.w, tol) && near(a.h, b.h, tol);
  const selectFile = async (i) => {
    await run(`document.querySelectorAll('.vi-tri .ab-file')[${i}].click(); return 1;`);
    await waitFor(`return document.querySelector('.vi-tri .dieuhuong .tt b')?.textContent === ${JSON.stringify(names[i])}`);
    await waitFor(`return document.querySelectorAll('.trang-ds > .giay .chuky-o').length > 0`);
    await sleep(700);
  };

  // Chuẩn bị: ảnh dấu + kiểu "Ảnh + thông tin" (khung theo khung kéo, không theo cỡ chữ), nạp 3 văn bản.
  const def = await inv('config_defaults');
  def.signature.imagePath = image;
  def.signature.display = 'imageAndText';
  await inv('config_save', { config: def });
  await send('Page.reload');
  await sleep(1500);
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  // Chức năng riêng "Vị trí chữ ký (kéo thả)" đã bỏ: không còn thẻ ở Trang chủ, không còn mục trong menu Chức năng.
  const cards = await run(`return [...document.querySelectorAll('.tc-the b')].map((b) => b.textContent.trim())`);
  await clickText('.menubar button.mb', 'Chức năng');
  const menuItems = await waitFor(`const m = document.querySelector('.menubar .menu'); return m && [...m.querySelectorAll('button')].map((b) => b.textContent.replace(/\\s+/g, ' ').trim())`);
  await clickText('.menubar button.mb', 'Chức năng');
  check('Không còn chức năng "Vị trí chữ ký" ở Trang chủ và menu Chức năng',
    Array.isArray(cards) && cards.length === 3 && !cards.some((c) => /Vị trí/.test(c)) && Array.isArray(menuItems) && menuItems.length > 0 && !menuItems.some((t) => /Vị trí chữ ký/.test(t)),
    JSON.stringify({ cards, menuItems }));
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  check('Thả thư mục: 3 văn bản', (await waitFor(`return document.querySelectorAll('.ds .dong').length === 3 && 3`)) === 3);

  // 1. Nút "Cấu hình vị trí" mở màn Xem trước & vị trí chữ ký (không phải hộp Cấu hình).
  await clickText('.nguon button.nut', 'Cấu hình vị trí');
  const opened = await waitFor(`return /Xem trước & vị trí chữ ký/.test(document.title) && document.querySelectorAll('.vi-tri .ab-file').length === 3
      && document.querySelectorAll('.vi-tri .ab-file .o-ky').length === 3 && document.querySelectorAll('.trang-ds > .giay .chuky-o').length === 1 && !document.querySelector('.popup.cauhinh')`);
  let b = await boxes();
  const m0 = await minis();
  const modeVal = await run(`return document.querySelector('.vi-tri .hang select.fld').value`);
  check('Nút "Cấu hình vị trí" mở màn Xem trước & vị trí chữ ký: lưới 3 file có khung thu nhỏ, chế độ "Tất cả file", không mở hộp Cấu hình', !!opened && modeVal === 'all', `${modeVal}`);
  check('Khung theo mẫu mặc định: trang cuối (3), góc dưới phải lề 15 / 20 mm, rộng 170 pt',
    b?.length === 1 && b[0].page === 3 && near(b[0].x + b[0].w, 1 - (15 * MM) / A4.w) && near(b[0].y + b[0].h, 1 - (20 * MM) / A4.h) && near(b[0].w, 170 / 595), JSON.stringify(b?.[0]));
  check('Ảnh thu nhỏ: khung đúng chỗ như bản xem trước', m0?.every((m) => m.o.length === 1 && sameBox(m.o[0], b[0], 0.01)), JSON.stringify(m0?.map((m) => m.o[0])));
  const tpl = b[0];
  await sleep(400);
  const f0 = await focused();
  check('Mở màn: vùng xem tự cuộn tới khung chữ ký ở trang cuối (3)', !!f0 && f0.ok && f0.page === 3 && f0.scrollTop > 0, JSON.stringify(f0));
  await shot('1-man-vi-tri.png');
  // Ctrl+→ sang file 2 rồi Ctrl+← về file 1 (cuộn lên đầu trước): mỗi lần chuyển, vùng xem tự cuộn tới khung.
  const ctrl = (key) => run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}', ctrlKey: true, bubbles: true })); return 1;`);
  await ctrl('ArrowRight');
  await waitFor(`return document.querySelector('.vi-tri .dieuhuong .tt b')?.textContent === ${JSON.stringify(names[1])}`);
  await waitFor(`return document.querySelectorAll('.trang-ds > .giay .chuky-o').length > 0`);
  await sleep(700);
  const f1 = await focused();
  await run(`document.querySelector('.vungxem').scrollTop = 0; return 1;`);
  await ctrl('ArrowLeft');
  await waitFor(`return document.querySelector('.vi-tri .dieuhuong .tt b')?.textContent === ${JSON.stringify(names[0])}`);
  await waitFor(`return document.querySelectorAll('.trang-ds > .giay').length === 3 && document.querySelectorAll('.trang-ds > .giay .chuky-o').length > 0`);
  await sleep(700);
  const f2 = await focused();
  check('Ctrl+→ / Ctrl+←: mỗi lần chuyển file, vùng xem tự cuộn tới khung chữ ký', !!f1?.ok && !!f2?.ok && f2.page === 3 && f2.scrollTop > 0, JSON.stringify({ f1, f2 }));

  // Giữ Ctrl + lăn chuột (chuột thật qua CDP) trên vùng xem: phóng to / thu nhỏ, giữ chỗ dưới con trỏ; lăn thường thì cuộn.
  const zoomLabel = () => run(`return document.querySelector('.vi-tri .zm')?.textContent.trim()`);
  const under = (y) =>
    run(`const v = document.querySelector('.vungxem'); const r = v.getBoundingClientRect();
      return { fy: (v.scrollTop + ${y} - r.top) / v.scrollHeight, top: v.scrollTop, dpr: window.devicePixelRatio, vv: window.visualViewport?.scale ?? 1 };`);
  const wheel = (x, y, deltaY, ctrl) => send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: 0, deltaY, modifiers: ctrl ? 2 : 0 });
  const vc = await run(`const r = document.querySelector('.vungxem').getBoundingClientRect(); return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height * 0.4) };`);
  const z0 = await zoomLabel();
  const u0 = await under(vc.y);
  await wheel(vc.x, vc.y, -100, true);
  await waitFor(`return document.querySelector('.vi-tri .zm')?.textContent.trim() === '76 %'`);
  await sleep(1200);
  const u1 = await under(vc.y);
  await wheel(vc.x, vc.y, 100, true);
  await waitFor(`return document.querySelector('.vi-tri .zm')?.textContent.trim() === '66 %'`);
  await sleep(1200);
  const u2 = await under(vc.y);
  const z2 = await zoomLabel();
  check('Ctrl + lăn chuột: lên phóng to 66 → 76 %, xuống thu nhỏ về 66 %; chỗ dưới con trỏ giữ nguyên; trang giao diện không bị phóng',
    z0 === '66 %' && z2 === '66 %' && Math.abs(u1.fy - u0.fy) < 0.02 && Math.abs(u2.fy - u0.fy) < 0.02 && u1.dpr === u0.dpr && u1.vv === 1,
    JSON.stringify({ z0, z2, u0, u1, u2 }));
  await wheel(vc.x, vc.y, -300, false);
  await sleep(600);
  const u3 = await under(vc.y);
  check('Lăn chuột không giữ Ctrl: chỉ cuộn, không đổi zoom', (await zoomLabel()) === '66 %' && u3.top < u2.top, JSON.stringify({ before: u2.top, after: u3.top }));

  // 2. Tất cả file: kéo khung → mọi file cùng một khung; chưa Áp dụng thì config.json chưa đổi.
  await drag(-220, -230);
  b = await boxes();
  const m1 = await minis();
  check('Kéo khung (Tất cả file): khung dời chỗ, mọi ảnh thu nhỏ cùng chỗ mới', !!b?.[0] && !sameBox(b[0], tpl) && m1.every((m) => sameBox(m.o[0], b[0], 0.01)), JSON.stringify({ box: b?.[0], minis: m1.map((m) => m.o[0]) }));
  check('Chân màn: "Tự đặt" và toạ độ X / Y', /Tự đặt/.test((await foot()) ?? '') && /X \d+%/.test((await foot()) ?? ''), await foot());
  check('Chưa bấm Áp dụng: config.json chưa có vị trí', readCfg().signature.positions.length === 0);

  // 3. Thanh Cỡ 150%: khung rộng gấp 1,5 lần cỡ theo mẫu, giữ góc trên trái.
  const before = b[0];
  await setField('.vi-tri .hang input[type=range]', 150, 'input');
  await sleep(500);
  b = await boxes();
  const label = await run(`return document.querySelector('.vi-tri .vt-co')?.textContent`);
  check('Cỡ 150%: khung rộng / cao gấp 1,5 lần mẫu, giữ góc trên trái', !!b?.[0] && near(b[0].w, tpl.w * 1.5) && near(b[0].h, tpl.h * 1.5) && near(b[0].x, before.x) && near(b[0].y, before.y) && label === '150%', `${label} ${JSON.stringify(b?.[0])}`);
  const shared = b[0];

  // 4. Từng file: file 2 kéo riêng → chỉ file 2 có dấu tích, file khác giữ khung chung.
  await setField('.vi-tri .hang select.fld', 'per');
  await selectFile(1);
  await drag(160, 120);
  b = await boxes();
  const own2 = b?.[0];
  let m = await minis();
  check('Từng file: kéo khung file 2 → chỉ file 2 có dấu tích, file 1 / 3 giữ khung chung',
    !!own2 && !sameBox(own2, shared) && m[1].tick && !m[0].tick && !m[2].tick && sameBox(m[0].o[0], shared, 0.01) && sameBox(m[2].o[0], shared, 0.01) && sameBox(m[1].o[0], own2, 0.01),
    JSON.stringify(m.map((x) => ({ tick: x.tick, o: x.o[0] }))));
  check('Chân màn: "Riêng file này", đặt riêng 1/3 file', /Riêng file này/.test((await foot()) ?? '') && /đặt riêng 1\/3 file/.test((await foot()) ?? ''), await foot());
  await shot('2-tung-file.png');

  // 5. Nâng cao › Vị trí theo mẫu "Dưới trái" cho file 3.
  await selectFile(2);
  await clickText('.vi-tri .hang button.nangcao', 'Nâng cao');
  await waitFor(`return !!document.querySelector('.vi-tri .vt-nc')`);
  await setField('.vi-tri .vt-nc select.fld', 'bl', 'change', 1);
  await sleep(1200);
  b = await boxes();
  const own3 = b?.[0];
  m = await minis();
  check('Vị trí theo mẫu "Dưới trái" (file 3): khung góc dưới trái lề 15 / 20 mm, cỡ theo mẫu; file 3 có dấu tích',
    !!own3 && near(own3.x, (15 * MM) / A4.w) && near(own3.y + own3.h, 1 - (20 * MM) / A4.h) && near(own3.w, tpl.w) && m[2].tick,
    JSON.stringify(own3));
  check('Chân màn: đặt riêng 2/3 file', /đặt riêng 2\/3 file/.test((await foot()) ?? ''), await foot());
  await shot('3-theo-mau.png');

  // 6. Đặt lại (file 3) rồi lại Dưới trái; Áp dụng → về Ký số, config.json đúng.
  await clickText('.vi-tri .chan button', 'Đặt lại');
  await sleep(600);
  m = await minis();
  check('Đặt lại (file 3): bỏ khung riêng, về khung chung', !m[2].tick && sameBox(m[2].o[0], shared, 0.01), JSON.stringify(m[2]));
  await setField('.vi-tri .vt-nc select.fld', 'br', 'change', 1);
  await sleep(300);
  await setField('.vi-tri .vt-nc select.fld', 'bl', 'change', 1);
  await sleep(1200);

  // 6b. Từng file: kiểu hiển thị riêng cho file 3 ("Chỉ thông tin"), các file khác giữ kiểu chung "Ảnh + thông tin".
  await setField('.vi-tri select.vt-kieu', 'text');
  await sleep(1200);
  const hasImg = () => run(`return !!document.querySelector('.trang-ds > .giay .chuky-o img')`);
  const own3t = (await boxes())?.[0];
  const ft3 = await foot();
  const img3 = await hasImg();
  m = await minis();
  check('Hiển thị riêng file 3 "Chỉ thông tin": khung không còn ảnh, chân màn báo kiểu riêng, file 3 có dấu tích',
    !img3 && /hiển thị riêng: Chỉ thông tin/.test(ft3 ?? '') && m[2].tick && !!own3t, `${img3} ${ft3}`);
  await selectFile(0);
  const v0 = await run(`return document.querySelector('.vi-tri select.vt-kieu').value`);
  const img0 = await hasImg();
  await selectFile(2);
  const v2 = await run(`return document.querySelector('.vi-tri select.vt-kieu').value`);
  check('File 1 vẫn kiểu chung "Ảnh + thông tin" (có ảnh); về file 3 thấy lại kiểu riêng', v0 === 'imageAndText' && img0 && v2 === 'text', `${v0} ${img0} ${v2}`);
  await clickText('.vi-tri .chan button.chinh', 'Áp dụng');
  await waitFor(`return /— Ký số/.test(document.title)`);
  await sleep(800);
  const c = readCfg().signature;
  const p1 = path.join(src, names[1]);
  const p2 = path.join(src, names[2]);
  check('Áp dụng: config.json có khung chung (trang cuối, cỡ 150%), khung riêng file 2 / 3, chế độ Từng file',
    !!c.positions[0] && c.positions[0].page === 0 && near(c.positions[0].widthRatio, tpl.w * 1.5) && c.perFile === true && Object.keys(c.filePositions).sort().join('|') === [p1, p2].sort().join('|') &&
      near(c.filePositions[p1][0].xRatio, own2.x) && near(c.filePositions[p2][0].xRatio, own3.x),
    JSON.stringify(c.filePositions));
  check('Áp dụng: config.json giữ kiểu chung "Ảnh + thông tin", kiểu riêng file 3 "Chỉ thông tin"',
    c.display === 'imageAndText' && JSON.stringify(c.fileDisplays) === JSON.stringify({ [p2]: 'text' }), JSON.stringify({ display: c.display, fileDisplays: c.fileDisplays }));

  // 7. Màn Ký số vẽ khung theo vị trí của từng file.
  const ksBox = async (i) => {
    await run(`document.querySelectorAll('.ds .dong')[${i}].click(); return 1;`);
    await waitFor(`return document.querySelector('.dieuhuong .tt b')?.textContent === ${JSON.stringify(names[i])} && document.querySelectorAll('.trang-ds > .giay .chuky-o').length === 1`);
    await sleep(900);
    const box = (await boxes())?.[0];
    return box && { ...box, kieu: await run(`return document.querySelector('select.kieu-ky')?.value`), img: await run(`return !!document.querySelector('.trang-ds > .giay .chuky-o img')`) };
  };
  const k0 = await ksBox(0);
  const k1 = await ksBox(1);
  const k2 = await ksBox(2);
  check('Màn Ký số: file 1 khung chung, file 2 / 3 khung riêng', sameBox(k0, shared) && sameBox(k1, own2) && sameBox(k2, own3t), JSON.stringify({ k0, k1, k2 }));
  check('Màn Ký số: ô Hiển thị theo từng file (file 1 / 2 "Ảnh + thông tin" có ảnh, file 3 "Chỉ thông tin" không ảnh)',
    k0?.kieu === 'imageAndText' && k0.img && k1?.kieu === 'imageAndText' && k1.img && k2?.kieu === 'text' && !k2.img, JSON.stringify([k0, k1, k2].map((k) => k && { kieu: k.kieu, img: k.img })));

  // 8. Thoát không lưu: đổi chế độ rồi Thoát → config.json giữ nguyên.
  await clickText('.nguon button.nut', 'Cấu hình vị trí');
  await waitFor(`return /Xem trước & vị trí chữ ký/.test(document.title) && document.querySelector('.vi-tri .hang select.fld')?.value === 'per'`);
  await setField('.vi-tri .hang select.fld', 'all');
  await clickText('.vi-tri .chan button', 'Thoát');
  await waitFor(`return /— Ký số/.test(document.title)`);
  await sleep(600);
  check('Mở lại thấy chế độ Từng file; đổi rồi Thoát: config.json giữ nguyên', readCfg().signature.perFile === true);

  // 9. Ký 3 file bằng khoá thử, đo khối chữ ký trong file đã ký.
  await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((x) => !x.disabled)?.click(); return 1;`);
  await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
  await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
     r?.querySelector('input')?.click(); return !!r;`);
  await clickText('app-popup button.nut.chinh', 'Ký số');
  const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((x) => x.textContent).join('/')`, 120000);
  check('Ký 3 văn bản', done === '3/3/0', String(done));
  await clickText('app-popup button', 'Đóng');
  const out = `${src}_DaKySo`;
  const outFiles = names.map((n) => path.join(out, n.replace('.pdf', '_signed.pdf')));
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, re, pymupdf
def has_image(d, xref, seen):
    # Duyệt các đối tượng mà dòng hình của khối chữ ký (AP /N) tham chiếu tới, tìm ảnh (/Subtype /Image).
    if xref in seen: return False
    seen.add(xref)
    obj = d.xref_object(xref)
    if re.search(r"/Subtype\\s*/Image", obj): return True
    return any(has_image(d, int(n), seen) for n in re.findall(r"(\\d+) 0 R", obj))
r = []
for f in sys.argv[1:]:
    d = pymupdf.open(f)
    out = []
    for i, p in enumerate(d):
        for w in p.widgets():
            ap = d.xref_get_key(w.xref, "AP/N")
            m = re.match(r"(\\d+) 0 R", ap[1]) if ap[0] == "xref" else None
            out.append({"page": i + 1, "x": w.rect.x0 / p.rect.width, "y": w.rect.y0 / p.rect.height, "w": w.rect.width / p.rect.width, "h": w.rect.height / p.rect.height,
                        "img": bool(m) and has_image(d, int(m.group(1)), set())})
    r.append(out)
print(json.dumps(r))`, ...outFiles], { encoding: 'utf8' });
  let ws = [];
  try {
    ws = JSON.parse(py.stdout);
  } catch {}
  const one = (list, want, page) => list?.length === 1 && list[0].page === page && sameBox(list[0], want, 0.008);
  check('File đã ký: file 1 khối ở khung chung (trang cuối = 3), file 2 / 3 ở khung riêng (trang 1)', one(ws[0], shared, 3) && one(ws[1], own2, 1) && one(ws[2], own3t, 1),
    JSON.stringify(ws) + (py.stderr ? ' ' + py.stderr.slice(0, 300) : ''));
  check('File đã ký: khối chữ ký file 1 / 2 có ảnh, file 3 chỉ có chữ (kiểu riêng)', ws[0]?.[0]?.img === true && ws[1]?.[0]?.img === true && ws[2]?.[0]?.img === false,
    JSON.stringify(ws.map((l) => l?.map((w) => w.img))));
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.stack ?? error.message}`);
  results.push(false);
} finally {
  app.kill();
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
