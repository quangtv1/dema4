// Kiểm nút "Tạm dừng" khi đang ký (màn Ký số) trên app thật, bản build `--features dev-keys` (khoá thử ở
// SMKS_DEV_PKI). Ký 60 bản chép của một PDF, bấm Tạm dừng giữa chừng: file chưa ký hiện "Tạm dừng" (không phải
// "Lỗi"), hộp kết quả có cột "Tạm dừng", "Ký tiếp" ký đúng các file còn lại.
//
// Cách dùng: node app/tests/ks-tamdung.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9363;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const src = path.join(work, 'vb');
const out = `${src}_DaKySo`;
for (const d of [cfgDir, src]) fs.mkdirSync(d, { recursive: true });
const N = 60;
for (let i = 1; i <= N; i++) fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, `van ban ${String(i).padStart(2, '0')}.pdf`));

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
  const shot = async (name) => {
    const r = await send('Page.captureScreenshot', { format: 'png' });
    if (r.result?.data) fs.writeFileSync(path.join(work, name), Buffer.from(r.result.data, 'base64'));
  };
  const clickText = (sel, text) =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  const inv = (cmd, args = {}) => run(`return await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);
  const DEV = 'Khoá phần mềm';
  // Dòng của chứng thư khoá thử trong bảng tab USB Token (theo tên chứng thư, biết sau khi gọi cert_list).
  let devName = '';
  const devRow = () => `[...document.querySelectorAll('.popup.cauhinh .bang-tk .r')].find((r) => r.querySelector('.ten-tk .dan')?.textContent === ${JSON.stringify(devName)})`;
  const openTokenTab = async () => {
    await clickText('.menubar button.mb', 'Cấu hình');
    await waitFor(`return !!document.querySelector('.popup.cauhinh')`);
    await clickText('.popup.cauhinh .tabch button', 'USB Token');
    await waitFor(`return document.querySelectorAll('.popup.cauhinh .bang-tk .r').length > 0`);
    await sleep(400);
  };
  const saveCfg = async () => {
    await clickText('.popup.cauhinh .cuoi button', 'Lưu cấu hình');
    await waitFor(`return document.querySelector('.popup.cauhinh .cuoi')?.textContent.includes('Đã lưu cấu hình')`);
  };
  const closeCfg = async () => {
    await clickText('.popup.cauhinh .cuoi button', 'Hủy');
    await sleep(300);
  };
  const openConfirm = async () => {
    fs.rmSync(out, { recursive: true, force: true });
    await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((x) => !x.disabled)?.click(); return 1;`);
    await waitFor(`return !!document.querySelector('.popup[aria-label="Xác nhận ký"] .ct-ds, .popup[aria-label="Xác nhận ký"] .bao.loi')`, 30000);
    await sleep(300);
  };
  const confirmRows = () => run(`return [...document.querySelectorAll('app-popup .ct-dong')].map((l) => l.textContent.replace(/\\s+/g, ' ').trim())`);
  const pickDev = () =>
    run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes(${JSON.stringify(DEV)}) && !l.classList.contains('tat'));
       r?.querySelector('input')?.click(); return !!r;`);
  const finishSign = async () => {
    await clickText('app-popup button.nut.chinh', 'Ký số');
    const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((x) => x.textContent).join('/')`, 120000);
    await clickText('app-popup button', 'Đóng');
    await sleep(400);
    return done;
  };
  const uiLog = async () => {
    await run(`[...document.querySelectorAll('.tabs .tab')].find((b) => b.textContent.includes('Nhật ký'))?.click(); return 1;`);
    await sleep(300);
    const t = await run(`return [...document.querySelectorAll('.log .ln .txt')].map((x) => x.textContent).join('\\n')`);
    await run(`[...document.querySelectorAll('.tabs .tab')].find((b) => b.textContent.includes('Văn bản'))?.click(); return 1;`);
    return t ?? '';
  };

  const chips = () => run(`const c = (k) => document.querySelectorAll('.ds .dong .chip-tt.' + k).length; return { ok: c('ok'), loi: c('loi'), dung: c('dung'), cho: c('cho'), dang: c('dang') }`);
  const doneDialog = () =>
    waitFor(`const p = document.querySelector('app-popup'); if (!p || !p.querySelector('.so-lon')) return null;
      return { title: p.querySelector('.tieu-p')?.textContent.trim(), nums: [...p.querySelectorAll('.so-lon > div')].map((d) => d.querySelector('span').textContent + '=' + d.querySelector('b').textContent),
        hint: p.querySelector('.hint.mo-ta')?.textContent.replace(/\\s+/g, ' ').trim(), loiList: !!p.querySelector('.trung-dong.loi') }`, 180000);
  const num = (d, k) => Number((d?.nums ?? []).find((x) => x.startsWith(k + '='))?.split('=')[1] ?? NaN);

  // Nạp 60 văn bản.
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  check(`Thả thư mục: ${N} văn bản`, (await waitFor(`return document.querySelectorAll('.ds .dong').length === ${N} && ${N}`)) === N);

  // 1. Ký, bấm "Tạm dừng" khi đã ký được vài file.
  await openConfirm();
  await pickDev();
  await clickText('app-popup button.nut.chinh', 'Ký số');
  const btn = await waitFor(`const b = [...document.querySelectorAll('.chan button')].find((x) => x.textContent.includes('Tạm dừng')); return b && b.textContent.trim()`, 30000);
  const t0 = Date.now();
  while (Date.now() - t0 < 60000 && ((await run(`return document.querySelectorAll('.ds .dong .chip-tt.ok').length`)) ?? 0) < 2) await sleep(50);
  await clickText('.chan button', 'Tạm dừng');
  const stopping = await waitFor(`return [...document.querySelectorAll('.chan button, .phai-tab .td-tt')].some((x) => x.textContent.includes('Đang tạm dừng')) && 1`, 3000);
  await shot('1-dang-tam-dung.png');
  const d1 = await doneDialog();
  await shot('2-ket-qua-tam-dung.png');
  const ok1 = num(d1, 'Thành công');
  const paused1 = num(d1, 'Tạm dừng');
  check('Khi đang ký có nút "Tạm dừng"; bấm thì hiện "Đang tạm dừng…"', btn === 'Tạm dừng' && !!stopping, `${btn} ${stopping}`);
  check('Hộp kết quả: "Đã tạm dừng", Lỗi = 0, có cột "Tạm dừng", tổng = thành công + tạm dừng, nhắc bấm Ký tiếp, không có danh sách "Văn bản lỗi"',
    d1?.title === 'Đã tạm dừng' && num(d1, 'Lỗi') === 0 && paused1 > 0 && ok1 + paused1 === N && /Ký tiếp/.test(d1.hint ?? '') && !d1.loiList, JSON.stringify(d1));
  await clickText('app-popup button', 'Đóng');
  await sleep(400);
  const c1 = await chips();
  const label = await run(`return [...document.querySelectorAll('.chan button.nut.chinh')].map((b) => b.textContent.trim()).join('|')`);
  const foot = await run(`return document.querySelector('.chan .dan')?.textContent.replace(/\\s+/g, ' ').trim()`);
  const reasons = await run(`return document.querySelectorAll('.ds .dong .ly-do').length`);
  check('Danh sách: file chưa ký hiện "Tạm dừng" (không có "Lỗi"), không ghi lý do lỗi; nút chính "Ký tiếp"; chân màn báo Tạm dừng',
    c1.dung === paused1 && c1.loi === 0 && c1.ok === ok1 && reasons === 0 && /Ký tiếp/.test(label ?? '') && /Tạm dừng/.test(foot ?? ''), `${JSON.stringify(c1)} | ${label} | ${foot}`);
  // Chọn một file tạm dừng: bản xem trước báo chưa ký vì tạm dừng.
  await run(`[...document.querySelectorAll('.ds .dong')].find((r) => r.querySelector('.chip-tt.dung'))?.click(); return 1;`);
  const note = await waitFor(`return [...document.querySelectorAll('.vungxem .bao')].map((b) => b.textContent).find((t) => t.includes('tạm dừng'))`, 5000);
  const head = await run(`return document.querySelector('.dieuhuong .chip-tt')?.textContent.trim()`);
  await shot('3-file-tam-dung.png');
  check('Xem file tạm dừng: nhãn "Tạm dừng", báo "Chưa ký: lượt ký đang tạm dừng"', head === 'Tạm dừng' && /Chưa ký: lượt ký đang tạm dừng/.test(note ?? ''), `${head} | ${note}`);
  const log1 = await uiLog();
  check('Nhật ký: "Đã tạm dừng…", dòng tổng có "tạm dừng", không có dòng lỗi cho file tạm dừng',
    /Đã tạm dừng\. Còn \d+ văn bản chưa ký/.test(log1) && log1.includes(`${paused1} tạm dừng`) && !/Chưa ký: đã tạm dừng/.test(log1),
    log1.split('\n').filter((l) => /dừng/.test(l)).join(' | '));

  // 2. Ký tiếp: ký đúng các file còn lại.
  await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((x) => !x.disabled)?.click(); return 1;`);
  await waitFor(`return !!document.querySelector('.popup[aria-label="Xác nhận ký"] .ct-ds')`, 30000);
  await sleep(300);
  await pickDev();
  await clickText('app-popup button.nut.chinh', 'Ký số');
  const d2 = await doneDialog();
  await clickText('app-popup button', 'Đóng');
  await sleep(400);
  const c2 = await chips();
  const outN = fs.readdirSync(out).filter((n) => n.endsWith('.pdf')).length;
  check('Ký tiếp: ký đúng số file tạm dừng, 0 lỗi; cả danh sách "Đã ký"; DaKySo có đủ file',
    num(d2, 'Tổng số') === paused1 && num(d2, 'Thành công') === paused1 && num(d2, 'Lỗi') === 0 && c2.ok === N && c2.dung === 0 && outN === N,
    `${JSON.stringify(d2?.nums)} | ${JSON.stringify(c2)} | ${outN} file`);
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
