// Kiểm "Xoá chứng thư khỏi danh sách" và "Lưu mã PIN / tự nhập PIN" (D2, TD-41) trên app thật, bản build
// `--features dev-keys` (khoá thử ở SMKS_DEV_PKI; khoá thử không có PIN nên lõi giả lập "middleware nhận PIN").
// Cấu hình và pin.bin ghi vào thư mục riêng (SMKS_CONFIG_DIR). PIN của bài kiểm là chuỗi giả, không phải PIN thật.
// Kiểm thêm: PIN rõ không có trong pin.bin, config.json, log của ứng dụng, ô nhập sau khi gửi.
//
// Cách dùng: node app/tests/ks-pin.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9359;
const PIN = 'Pq7391x8';
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
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(src, 'van ban 1.pdf'));
const readCfg = () => JSON.parse(fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8'));
const pinFile = path.join(cfgDir, 'pin.bin');
const started = Date.now();

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

  // Nạp 1 văn bản.
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  check('Thả thư mục: 1 văn bản', (await waitFor(`return document.querySelectorAll('.ds .dong').length === 1 && 1`)) === 1);
  const scan = await inv('cert_list');
  const dev = scan?.certificates?.find((c) => (c.keyProvider ?? '').includes(DEV));
  const devThumb = dev?.thumbprint?.toUpperCase();
  devName = dev?.commonName ?? '';
  check('Có chứng thư khoá thử trong danh sách', !!devThumb);

  // 1. Tab USB Token: xoá chứng thư khoá thử khỏi danh sách → Lưu → config.json; hộp Xác nhận ký không còn nó.
  await openTokenTab();
  const n0 = await run(`return document.querySelectorAll('.popup.cauhinh .bang-tk .r').length`);
  await run(`${devRow()}?.querySelector('button[title^="Xoá khỏi danh sách"]')?.click(); return 1;`);
  await sleep(300);
  const n1 = await run(`return document.querySelectorAll('.popup.cauhinh .bang-tk .r').length`);
  const gone = await run(`return !${devRow()}`);
  const hiddenNote = await run(`return document.querySelector('.popup.cauhinh .tk-an')?.textContent.replace(/\\s+/g, ' ').trim()`);
  await shot('1-xoa-khoi-danh-sach.png');
  check('Bấm xoá: dòng chứng thư biến mất, báo "Đã xoá 1 chứng thư khỏi danh sách", có "Hiện lại tất cả"', n1 === n0 - 1 && gone && /Đã xoá 1 chứng thư/.test(hiddenNote ?? '') && /Hiện lại tất cả/.test(hiddenNote ?? ''), `${n0} → ${n1}; ${hiddenNote}`);
  await saveCfg();
  check('Lưu cấu hình: config.json có token.hiddenCerts = [thumbprint khoá thử]', JSON.stringify(readCfg().token?.hiddenCerts) === JSON.stringify([devThumb]));
  await closeCfg();
  const still = (await inv('cert_list'))?.certificates?.some((c) => c.thumbprint.toUpperCase() === devThumb);
  check('Chứng thư vẫn còn trong kho (cert_list vẫn trả về)', !!still);
  await openConfirm();
  let rows = await confirmRows();
  check('Hộp Xác nhận ký không còn chứng thư đã xoá khỏi danh sách', Array.isArray(rows) && !rows.some((r) => r.includes(DEV)), `${rows?.length} dòng`);
  await clickText('app-popup button', 'Hủy');
  await sleep(300);

  // 2. Hiện lại tất cả.
  await openTokenTab();
  await clickText('.popup.cauhinh .tk-an button', 'Hiện lại tất cả');
  await sleep(300);
  const back = await run(`return !!${devRow()}`);
  await saveCfg();
  check('"Hiện lại tất cả" + Lưu: chứng thư hiện lại, hiddenCerts rỗng', back && readCfg().token?.hiddenCerts?.length === 0);
  await closeCfg();

  // 3. Hộp Xác nhận ký: nhập PIN + "Lưu PIN" → ký; pin.bin chỉ có bản mã.
  await openConfirm();
  await pickDev();
  await sleep(200);
  const hasInput = await run(`return !!document.querySelector('app-popup .ct-pin-o input[type=password]') && document.querySelector('app-popup .ct-pin-luu-o input').disabled`);
  await run(`const i = document.querySelector('app-popup .ct-pin-o input'); i.value = ${JSON.stringify(PIN)}; i.dispatchEvent(new Event('input', { bubbles: true })); return 1;`);
  await sleep(150);
  await run(`document.querySelector('app-popup .ct-pin-luu-o input').click(); return 1;`);
  await sleep(200);
  const warn = await run(`return document.querySelector('app-popup .ct-pin .bao.canh')?.textContent`);
  await shot('2-nhap-pin.png');
  check('Ô Mã PIN (dạng mật khẩu), "Lưu PIN" khoá khi chưa nhập; tích Lưu thì hiện cảnh báo', !!hasInput && /mã hoá theo tài khoản Windows/.test(warn ?? ''), String(warn));
  let done = await finishSign();
  let log = await uiLog();
  check('Ký 1/1 với PIN vừa nhập; nhật ký: middleware nhận PIN, đã lưu PIN', done === '1/1/0' && /middleware nhận PIN/.test(log) && /Đã lưu mã PIN trên máy/.test(log), `${done} | ${log.split('\n').filter((l) => /PIN/.test(l)).join(' | ')}`);
  const pinText = fs.existsSync(pinFile) ? fs.readFileSync(pinFile, 'utf8') : '';
  let pinJson = null;
  try {
    pinJson = JSON.parse(pinText);
  } catch {}
  check('pin.bin có bản mã của đúng chứng thư, không chứa PIN rõ', !!pinJson && Object.keys(pinJson.pins ?? {}).join() === devThumb && !pinText.includes(PIN),
    `${Object.keys(pinJson?.pins ?? {}).length} mục; ${pinText.length} byte`);
  check('config.json không chứa PIN', !fs.readFileSync(path.join(cfgDir, 'config.json'), 'utf8').includes(PIN));

  // 4. Phiên token đóng (như mở app lần sau) → hộp Xác nhận ký báo đã lưu PIN, không hỏi; ký tự nhập PIN đã lưu.
  await inv('token_forget');
  await openConfirm();
  await pickDev();
  await sleep(300);
  const savedUi = await run(`return { chip: [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes(${JSON.stringify(DEV)}))?.textContent.includes('Đã lưu PIN'),
      msg: document.querySelector('app-popup .ct-pin-luu')?.textContent.replace(/\\s+/g, ' ').trim(), input: !!document.querySelector('app-popup .ct-pin-o input') }`);
  await shot('3-da-luu-pin.png');
  check('Lần sau: chứng thư có nhãn "Đã lưu PIN", hộp báo app tự nhập, không có ô nhập PIN', savedUi?.chip && /tự nhập/.test(savedUi.msg ?? '') && !savedUi.input, JSON.stringify(savedUi));
  done = await finishSign();
  log = await uiLog();
  check('Ký 1/1, nhật ký "Tự nhập mã PIN đã lưu"', done === '1/1/0' && /Tự nhập mã PIN đã lưu: middleware nhận PIN/.test(log), done);

  // 5. Tab USB Token: nhãn "Đã lưu PIN", nút xoá PIN → pin.bin bị xoá.
  await openTokenTab();
  const tag = await run(`return ${devRow()}?.querySelector('.tk-pin')?.textContent`);
  await run(`${devRow()}?.querySelector('button[title^="Xoá mã PIN"]')?.click(); return 1;`);
  const tagGone = await waitFor(`return !${devRow()}?.querySelector('.tk-pin') && 1`, 5000);
  check('Cấu hình › USB Token: nhãn "Đã lưu PIN"; bấm xoá PIN thì nhãn mất, pin.bin bị xoá', tag === 'Đã lưu PIN' && !!tagGone && !fs.existsSync(pinFile), `${tag}; pin.bin ${fs.existsSync(pinFile) ? 'còn' : 'đã xoá'}`);
  await closeCfg();
  check('pin_saved rỗng', JSON.stringify(await inv('pin_saved')) === '[]');

  // 6. PIN rõ không có trong log của ứng dụng (các file log ghi trong lúc kiểm).
  const logDir = path.join(process.env.LOCALAPPDATA ?? '', 'SaoMai-KySo', 'logs');
  const logs = fs.existsSync(logDir) ? fs.readdirSync(logDir).map((n) => path.join(logDir, n)).filter((f) => fs.statSync(f).mtimeMs >= started - 1000) : [];
  const leaked = logs.filter((f) => fs.readFileSync(f, 'utf8').includes(PIN));
  const mentions = logs.some((f) => /pin\W{0,2}typed/.test(fs.readFileSync(f, 'utf8')));
  check('Log của ứng dụng không chứa PIN (có ghi việc đưa PIN, không ghi giá trị)', logs.length > 0 && leaked.length === 0 && mentions, `${logs.length} file log; lộ ${leaked.length}`);
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
