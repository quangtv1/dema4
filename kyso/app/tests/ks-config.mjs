// Kiểm hộp Cấu hình và mẫu chữ ký theo khổ giấy trên app thật (bản build `--features dev-keys`, khoá thử ở
// SMKS_DEV_PKI). Cấu hình ghi vào thư mục riêng của bài kiểm (SMKS_CONFIG_DIR), không đụng config.json thật.
//
// Cách dùng: node app/tests/ks-config.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử> <ảnh .png>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki, image] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9353;
const MM = 72 / 25.4;
const A4 = { w: 595.32, h: 841.92 };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
const near = (a, b, tol = 0.004) => Math.abs(a - b) < tol;

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const src = path.join(work, 'vb');
const shots = path.join(work, 'anh');
for (const d of [cfgDir, src, shots]) fs.mkdirSync(d, { recursive: true });
fs.copyFileSync(path.join(repo, 'fixtures/pdf/word-hybrid-xref.pdf'), path.join(src, 'van ban 1.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, 'van ban 2.pdf'));
const cfgFile = path.join(cfgDir, 'config.json');
const readCfg = () => JSON.parse(fs.readFileSync(cfgFile, 'utf8'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}

async function launch() {
  try {
    await fetch(`http://127.0.0.1:${debugPort}/json`);
    throw new Error(`cổng ${debugPort} đang bận`);
  } catch (e) {
    if (String(e.message).includes('đang bận')) throw e;
  }
  const app = spawn(exe, [], {
    env: { ...process.env, SMKS_DEV_PKI: pki, SMKS_CONFIG_DIR: cfgDir, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
    stdio: 'ignore',
  });
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
  return { app, socket, send, run, waitFor };
}

let session;
try {
  session = await launch();
  let { send, run, waitFor } = session;
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
                 cx: r.left + r.width / 2, cy: r.top + r.height / 2, text: b.textContent, font: b.querySelector('.tt-ky')?.style.fontFamily ?? '',
                 color: b.querySelector('.tt-ky')?.style.color ?? '' }; }));`);
  // Hộp Cấu hình mở từ menu "Cấu hình" (nút "Cấu hình vị trí" của màn Ký số mở màn Vị trí chữ ký), tab Chữ ký.
  const openCfg = async () => {
    await clickText('.menubar button.mb', 'Cấu hình');
    await waitFor(`return !!document.querySelector('.popup.cauhinh')`);
    await clickText('.popup.cauhinh .tabch button', 'Chữ ký');
    await sleep(150);
  };
  const waitBoxes = (pred, ms = 15000) =>
    waitFor(`const all = [...document.querySelectorAll('.trang-ds > .giay')].flatMap((s, i) => [...s.querySelectorAll('.chuky-o')].map(() => i + 1));
             return (${pred})(all) && all;`, ms);

  // 1. Lần đầu chạy: chưa có config.json, dùng mặc định.
  const first = await inv('config_get');
  check('config_get lần đầu: chưa có file, 5 mẫu theo khổ giấy, mẫu A4 mặc định', first?.created === true && first.config.signature.profiles.length === 5 && first.config.signature.profiles.find((p) => p.isDefault)?.paper === 'A4' && first.path === cfgFile,
    `${first?.path}; created=${first?.created}`);

  // Đặt ảnh dấu (giả lập đã chọn ảnh) qua lệnh lưu cấu hình, rồi nạp lại giao diện.
  const def = await inv('config_defaults');
  def.signature.imagePath = image;
  def.signature.display = 'imageAndText';
  const saved0 = await inv('config_save', { config: def });
  check('config_save ghi file config.json (ghi .tmp rồi đổi tên, không còn .tmp)', !!saved0?.config && fs.existsSync(cfgFile) && !fs.existsSync(cfgFile + '.tmp') && readCfg().signature.imagePath === image);
  await send('Page.reload');
  await sleep(1500);
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  const rows = await waitFor(`return document.querySelectorAll('.ds .dong').length === 2 && 2`);
  check('Thả thư mục: 2 văn bản', rows === 2);

  // 2. Mẫu mặc định (prototype): góc dưới phải trang cuối, lề 15 / 20 mm, khung 170 × 30 pt.
  await waitBoxes('(a) => a.length === 1');
  let b = await boxes();
  check('Mẫu mặc định: 1 khung, trang cuối (3), góc dưới phải cách lề 15 / 20 mm',
    b?.length === 1 && b[0].page === 3 && near(b[0].x + b[0].w, 1 - (15 * MM) / A4.w) && near(b[0].y + b[0].h, 1 - (20 * MM) / A4.h) && near(b[0].w, 170 / 595, 0.006), JSON.stringify(b?.[0] && { ...b[0], text: undefined }));
  await shot('1-mau-mac-dinh.png');

  // 3. Mở hộp Cấu hình từ menu.
  await openCfg();
  const dlg = await waitFor(`const p = document.querySelector('.popup.cauhinh'); return p && { tabs: p.querySelectorAll('.tabch button').length, cards: p.querySelectorAll('.pc').length,
      macDinh: p.querySelector('.pc.chon')?.textContent.includes('Mặc định') }`);
  check('Hộp Cấu hình: 3 tab, 5 mẫu, đang chọn mẫu mặc định', dlg?.tabs === 3 && dlg.cards === 5 && dlg.macDinh, JSON.stringify(dlg));
  await run(`document.querySelector('.popup.cauhinh .vt-o.vt-bl').click(); return 1;`);
  await setField('#cfg-page', 'last');
  await clickText('.popup.cauhinh .cfg-giua button', 'Nâng cao');
  await sleep(200);
  await setField('.cfg-le input', 25, 'change', 0);
  await setField('.cfg-le input', 30, 'change', 1);
  await setField('.cfg-font input.so', 12);
  await setField('.cfg-font input.mau', '#c00000', 'input');
  await setField('#cfg-reason', 'Phê duyệt hợp đồng');
  await setField('#cfg-loc', 'Hà Nội');
  const mini = await run(`const o = document.querySelector('.cfg-o'); return o && { left: parseFloat(o.style.left), top: parseFloat(o.style.top), w: parseFloat(o.style.width), h: parseFloat(o.style.height) }`);
  check('Xem trước mẫu: khung sang góc dưới trái, cách lề 25 / 30 mm', !!mini && near(mini.left / 100, (25 * MM) / A4.w, 0.01) && near((mini.top + mini.h) / 100, 1 - (30 * MM) / A4.h, 0.01), JSON.stringify(mini));
  await shot('2-cau-hinh-chu-ky.png');
  await clickText('.popup.cauhinh .tabch button', 'Thư mục');
  await waitFor(`return !!document.querySelector('.popup.cauhinh .cfg-tm')`);
  await setField('.cfg-tm input.cfg-ngan', 'DaKy_Test', 'change', 0);
  await setField('.cfg-tm input.cfg-ngan', '_ky', 'change', 1);
  await shot('3-cau-hinh-thu-muc.png');
  await clickText('.popup.cauhinh .cuoi button', 'Lưu cấu hình');
  const okMsg = await waitFor(`return document.querySelector('.popup.cauhinh .cuoi')?.textContent.includes('Đã lưu cấu hình')`);
  const c1 = readCfg();
  const a4 = c1.signature.profiles.find((p) => p.isDefault);
  check('Lưu cấu hình: "✓ Đã lưu", config.json có mẫu A4 dưới trái / trang cuối / lề 25·30, chữ 12 màu #c00000, lý do, nơi ký, DaKy_Test, _ky',
    !!okMsg && a4.anchor === 'bl' && a4.page === 'last' && a4.marginMm.x === 25 && a4.marginMm.y === 30 && c1.signature.font.size === 12 && c1.signature.font.color === '#c00000' &&
      c1.signature.reason === 'Phê duyệt hợp đồng' && c1.signature.location === 'Hà Nội' && c1.output.subfolderName === 'DaKy_Test' && c1.output.suffix === '_ky',
    JSON.stringify({ a4, font: c1.signature.font, out: c1.output }));
  await clickText('.popup.cauhinh .cuoi button', 'Hủy');

  // 4. Khung theo mẫu mới: trang cuối (3), góc dưới phải.
  await run(`document.querySelectorAll('.ds .dong')[0].click(); return 1;`);
  await waitBoxes('(a) => a.length === 1 && a[0] === 3');
  // Khung cũ (dưới phải) cũng ở trang 3: chờ lõi trả khung mới sang nửa trái trang rồi mới đo.
  await waitFor(`const o = document.querySelector('.trang-ds > .giay .chuky-o'); return !!o && parseFloat(o.style.left) < 50`);
  b = await boxes();
  check('Khung theo mẫu: trang 3 (cuối), góc dưới trái cách lề 25 / 30 mm',
    b?.length === 1 && b[0].page === 3 && near(b[0].x, (25 * MM) / A4.w) && near(b[0].y + b[0].h, 1 - (30 * MM) / A4.h), JSON.stringify(b?.[0] && { ...b[0], text: undefined }));
  check('Chữ trong khung theo cấu hình: màu #c00000', !!b?.[0] && /rgb\(192, 0, 0\)/.test(b[0].color), b?.[0]?.color);

  // 5. Ký nháy mọi trang.
  await openCfg();
  await waitFor(`return !!document.querySelector('.popup.cauhinh #cfg-page')`);
  await setField('#cfg-page', 'all');
  await clickText('.popup.cauhinh .cuoi button', 'Lưu cấu hình');
  await waitFor(`return document.querySelector('.popup.cauhinh .cuoi')?.textContent.includes('Đã lưu cấu hình')`);
  await clickText('.popup.cauhinh .cuoi button', 'Hủy');
  const all = await waitBoxes('(a) => a.length === 3');
  check('"Tất cả các trang (ký nháy)": 3 khung trên trang 1, 2, 3', JSON.stringify(all) === '[1,2,3]', JSON.stringify(all));
  await shot('4-ky-nhay.png');

  // 6. Kéo một trong 3 khung: văn bản nhiều khung thì thành khung riêng của văn bản đang xem (vẫn đủ 3 khung, chỉ khung
  //    vừa kéo đổi chỗ), vì danh sách trang cụ thể không áp đúng cho văn bản có số trang khác. "Dùng mẫu" thì về mẫu.
  await run(`document.querySelector('.trang-ds > .giay .chuky-o').scrollIntoView({ block: 'center' }); return 1;`);
  await sleep(300);
  b = await boxes();
  const from = { x: b[0].cx, y: b[0].cy };
  const to = { x: from.x - 200, y: from.y - 150 };
  const mouse = (type, x, y) => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1 });
  await mouse('mouseMoved', from.x, from.y);
  await mouse('mousePressed', from.x, from.y);
  for (let i = 1; i <= 8; i++) await mouse('mouseMoved', from.x + ((to.x - from.x) * i) / 8, from.y + ((to.y - from.y) * i) / 8);
  await mouse('mouseReleased', to.x, to.y);
  await sleep(1200);
  const sig6 = readCfg().signature;
  const own6 = Object.values(sig6.filePositions ?? {});
  const kept = await waitBoxes('(a) => a.length === 3');
  check('Kéo 1 trong 3 khung: vẫn 3 khung; lưu thành khung riêng của văn bản này (3 khung, bật "Từng file"), khung chung không đổi',
    JSON.stringify(kept) === '[1,2,3]' && own6.length === 1 && own6[0].length === 3 && sig6.perFile === true && sig6.positions.length === 0,
    JSON.stringify({ own: own6.map((l) => l.map((x) => x.page)), perFile: sig6.perFile, shared: sig6.positions.length }));
  await openCfg();
  const banner = await waitFor(`return document.querySelector('.popup.cauhinh .cfg-giua .bao')?.textContent.includes('vị trí tự đặt')`);
  await clickText('.popup.cauhinh .cfg-giua .bao button', 'Dùng mẫu');
  await clickText('.popup.cauhinh .cuoi button', 'Lưu cấu hình');
  await waitFor(`return document.querySelector('.popup.cauhinh .cuoi')?.textContent.includes('Đã lưu cấu hình')`);
  await clickText('.popup.cauhinh .cuoi button', 'Hủy');
  const back = await waitBoxes('(a) => a.length === 3');
  check('Hộp Cấu hình (menu Cấu hình) báo đang dùng vị trí tự đặt; "Dùng mẫu" thì bỏ khung riêng, về 3 khung theo mẫu',
    !!banner && readCfg().signature.positions.length === 0 && Object.keys(readCfg().signature.filePositions).length === 0 && back?.length === 3);

  // 7. Ký bằng khoá thử theo cấu hình.
  await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((x) => !x.disabled)?.click(); return 1;`);
  await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
  const note = await run(`return document.querySelector('app-popup .hint.mo-ta')?.textContent`);
  check('Hộp Xác nhận ký nói đúng nơi lưu theo cấu hình', /DaKy_Test/.test(note ?? '') && /_ky/.test(note ?? ''), note);
  await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
     r?.querySelector('input')?.click(); return !!r;`);
  await clickText('app-popup button.nut.chinh', 'Ký số');
  const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((x) => x.textContent).join('/')`, 120000);
  check('Ký 2 văn bản', done === '2/2/0', String(done));
  await clickText('app-popup button', 'Đóng');
  const out = `${src}_DaKy_Test`;
  const outFiles = fs.existsSync(out) ? fs.readdirSync(out).sort() : [];
  check('File ra ở thư mục vb_DaKy_Test cùng cấp, hậu tố _ky', JSON.stringify(outFiles) === JSON.stringify(['van ban 1_ky.pdf', 'van ban 2_ky.pdf']), outFiles.join(', '));
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, pymupdf
from pyhanko.pdf_utils.reader import PdfFileReader
r = {}
for f in sys.argv[1:]:
    d = pymupdf.open(f)
    ws = [{"page": i + 1, "left": w.rect.x0 / p.rect.width, "bottom": w.rect.y1 / p.rect.height} for i, p in enumerate(d) for w in p.widgets()]
    with open(f, "rb") as h:
        s = PdfFileReader(h, strict=False).embedded_signatures[0].sig_object
        r[f.split("\\\\")[-1]] = {"widgets": ws, "pages": len(d), "reason": str(s.get("/Reason")), "location": str(s.get("/Location"))}
print(json.dumps(r))`, ...outFiles.map((f) => path.join(out, f))], { encoding: 'utf8' });
  let info = {};
  try {
    info = JSON.parse(py.stdout);
  } catch {}
  const f1 = info['van ban 1_ky.pdf'];
  const f2 = info['van ban 2_ky.pdf'];
  const posOk = (f) => f && f.widgets.length === f.pages && f.widgets.every((w) => near(w.left, (25 * MM) / A4.w) && near(w.bottom, 1 - (30 * MM) / A4.h));
  check('Ký nháy: mỗi trang của mỗi file có một khối, đúng góc dưới trái lề 25 / 30 mm', posOk(f1) && posOk(f2), JSON.stringify({ f1: f1?.widgets.length + '/' + f1?.pages, f2: f2?.widgets.length + '/' + f2?.pages }) + (py.stderr ? ' ' + py.stderr.slice(0, 300) : ''));
  check('Chữ ký có /Reason, /Location theo cấu hình', f1?.reason === 'Phê duyệt hợp đồng' && f1?.location === 'Hà Nội', JSON.stringify({ reason: f1?.reason, location: f1?.location }));

  // 8. config.json hỏng: giữ bản sao, dùng mặc định, báo trên màn.
  session.socket.close();
  session.app.kill();
  await sleep(1500);
  fs.writeFileSync(cfgFile, '{ "signature": hỏng');
  session = await launch();
  ({ send, run, waitFor } = session);
  // Lúc mở app, giao diện đọc cấu hình: lõi đổi tên file hỏng thành config.json.hong-… và dùng mặc định.
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(1000);
  const backups = fs.readdirSync(cfgDir).filter((n) => n.startsWith('config.json.hong-'));
  const after = await run(`return await window.__TAURI_INTERNALS__.invoke('config_get')`);
  check('config.json hỏng: đổi tên thành đúng 1 bản config.json.hong-… (giữ nguyên nội dung), sau đó dùng mặc định, không báo lại',
    backups.length === 1 && fs.readFileSync(path.join(cfgDir, backups[0]), 'utf8').includes('hỏng') && !fs.existsSync(cfgFile) &&
      after?.created === true && after?.warning === null && after?.config?.output?.subfolderName === 'DaKySo',
    `${backups}; created=${after?.created}; warning=${after?.warning}`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  const shown = await waitFor(`return document.querySelector('.bao.loi')?.textContent.includes('cấu hình') && document.querySelector('.bao.loi').textContent.trim()`);
  check('Màn Ký số hiện thông báo cấu hình hỏng', !!shown, String(shown).slice(0, 160));
  session.socket.close();
} catch (error) {
  console.error(`LOI: ${error.stack ?? error.message}`);
  results.push(false);
} finally {
  session?.app.kill();
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
