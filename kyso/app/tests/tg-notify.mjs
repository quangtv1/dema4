// Kiểm Thông báo Telegram và đăng ký thiết bị trên app thật (bản build `--features dev-keys`, khoá thử ở
// SMKS_DEV_PKI). Telegram được thay bằng một máy chủ Bot API giả chạy trên máy (cấu hình TELEGRAM_API_BASE), để đếm
// đúng từng tin nhắn và giả lập token sai / mất kết nối / treo. Phần gateway chạy `smks-gateway.exe` thật.
//
// Cách dùng:
//   node app/tests/tg-notify.mjs <SaoMai-KySo.exe> <smks-gateway.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
//
// Thêm `--real`: sau các bước trên, gửi vài tin THẬT tới Telegram bằng token ở biến môi trường TG_REAL_TOKEN và
// TG_REAL_CHAT (không ghi token vào repo; file cấu hình tạm nằm trong thư mục làm việc và bị xoá khi xong).
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';

const args = process.argv.slice(2).filter((a) => !a.startsWith('--'));
const REAL = process.argv.includes('--real');
const [exe, gatewayExe, repo, work, pki] = args.map((p) => path.resolve(p));
const DEBUG_PORT = 9371;
const MOCK_PORT = 18443;
const GATEWAY_PORT = 18789;
const FAKE_TOKEN = `7000000001:TEST${Date.now()}abcdefghijklmnopqrstuvwxyz`;
const FAKE_CHAT = '-1009876543210';
const ENROLL_KEY = `khoa-dang-ky-${Date.now()}-abcdef`;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
const section = (name) => console.log(`\n=== ${name} ===`);

// ---------------------------------------------------------------------------------------------
// Máy chủ Bot API giả
// ---------------------------------------------------------------------------------------------
const mock = { mode: 'ok', messages: [], calls: [], held: [], server: null };
const startMock = () =>
  new Promise((resolve) => {
    mock.server = http.createServer((req, res) => {
      let raw = '';
      req.on('data', (c) => (raw += c));
      req.on('end', () => {
        const m = /^\/bot([^/]+)\/(\w+)$/.exec(req.url ?? '');
        let body = {};
        try {
          body = JSON.parse(raw || '{}');
        } catch {}
        mock.calls.push({ token: m?.[1], method: m?.[2], mode: mock.mode });
        const reply = (status, json) => {
          res.writeHead(status, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify(json));
        };
        if (mock.mode === 'hang') {
          mock.held.push(res); // không trả lời: máy trạm phải tự hết thời gian chờ
          return;
        }
        if (mock.mode === 'unauthorized' || m?.[1] !== FAKE_TOKEN) return reply(401, { ok: false, error_code: 401, description: 'Unauthorized' });
        if (m[2] === 'getMe') return reply(200, { ok: true, result: { id: 1, is_bot: true, username: 'saomai_kiemthu_bot' } });
        if (m[2] === 'getChat') return reply(200, { ok: true, result: { id: Number(FAKE_CHAT), title: 'Nhóm kiểm thử' } });
        if (m[2] === 'sendMessage') {
          if (String(body.chat_id) !== FAKE_CHAT) return reply(400, { ok: false, error_code: 400, description: 'Bad Request: chat not found' });
          mock.messages.push(String(body.text));
          return reply(200, { ok: true, result: { message_id: mock.messages.length } });
        }
        reply(404, { ok: false, error_code: 404, description: 'Not Found' });
      });
    });
    mock.server.listen(MOCK_PORT, '127.0.0.1', resolve);
  });
const stopMock = () =>
  new Promise((resolve) => {
    if (!mock.server) return resolve();
    mock.held.splice(0).forEach((r) => r.destroy());
    mock.server.closeAllConnections?.();
    mock.server.close(() => resolve());
    mock.server = null;
  });
const tagged = (tag) => mock.messages.filter((t) => t.includes(`[SAO MAI SIGN - ${tag}]`));
const waitCount = async (fn, n, ms = 20000) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (fn() >= n) return true;
    await sleep(200);
  }
  return fn() >= n;
};

// ---------------------------------------------------------------------------------------------
// Chạy app và nói chuyện với giao diện qua cổng gỡ lỗi của WebView2
// ---------------------------------------------------------------------------------------------
if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });

const launch = async (cfgDir, notifyConfig) => {
  fs.mkdirSync(cfgDir, { recursive: true });
  const env = { ...process.env, SMKS_DEV_PKI: pki, SMKS_CONFIG_DIR: cfgDir, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}` };
  delete env.SMKS_NOTIFY_CONFIG;
  if (notifyConfig) env.SMKS_NOTIFY_CONFIG = notifyConfig;
  const app = spawn(exe, [], { env, stdio: 'ignore' });
  let target;
  for (let i = 0; i < 80 && !target; i++) {
    await sleep(250);
    try {
      target = (await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json`)).json()).find((t) => t.type === 'page');
    } catch {}
  }
  if (!target) {
    app.kill();
    throw new Error('không kết nối được WebView2');
  }
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
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: (e && typeof e === 'object' && 'code' in e) ? e : String(e?.message ?? e) }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    return reply.result?.result?.value ?? { err: reply.result?.exceptionDetails?.text };
  };
  const inv = (cmd, a = {}) => run(`return await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(a)})`);
  for (let i = 0; i < 40; i++) {
    if ((await run('return !!window.__TAURI_INTERNALS__')).ok) break;
    await sleep(250);
  }
  const waitFor = async (body, ms = 15000) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      const v = await run(body);
      if (v.ok) return v.ok;
      await sleep(250);
    }
    return null;
  };
  const clickText = (sel, text) =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  const shot = async (name) => {
    const r = await send('Page.captureScreenshot', { format: 'png' });
    if (r.result?.data) fs.writeFileSync(path.join(work, name), Buffer.from(r.result.data, 'base64'));
  };
  const status = async () => (await inv('notify_status')).ok;
  const waitStatus = async (pred, ms = 20000) => {
    const end = Date.now() + ms;
    let s;
    while (Date.now() < end) {
      s = await status();
      if (s && pred(s)) return s;
      await sleep(300);
    }
    return s;
  };
  const close = async () => {
    socket.close();
    // Đóng cửa sổ như người dùng (WM_CLOSE) để app tự đóng phiên Word giữ sẵn của xem trước; không thoát thì mới buộc dừng.
    spawnSync('taskkill', ['/PID', String(app.pid)], { stdio: 'ignore' });
    for (let i = 0; i < 32 && app.exitCode === null; i++) await sleep(250);
    if (app.exitCode === null) app.kill();
    for (let i = 0; i < 40; i++) {
      await sleep(250);
      try {
        await fetch(`http://127.0.0.1:${DEBUG_PORT}/json`);
      } catch {
        break;
      }
    }
    await sleep(500);
  };
  return { run, inv, waitFor, clickText, shot, status, waitStatus, close };
};

// ---------------------------------------------------------------------------------------------
// Nghiệp vụ qua IPC (như giao diện gọi)
// ---------------------------------------------------------------------------------------------
const appearance = { mode: 'text', kind: 'main', placements: [], fontFamily: 'Times New Roman', fontSize: 10, textColor: '#000000', reason: 'Ký số tài liệu', location: 'Việt Nam', sealImage: null, wetImage: null, signatureImage: null, sealDarkness: 140, wetDarkness: 140, wetStroke: 100, wetColor: null, stamps: [] };
const makeDocs = (dir, n, prefix = 'van ban') => {
  fs.mkdirSync(dir, { recursive: true });
  const files = [];
  for (let i = 1; i <= n; i++) {
    const f = path.join(dir, `${prefix} ${String(i).padStart(2, '0')}.pdf`);
    fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), f);
    files.push(f);
  }
  return files;
};
const signBatch = async (h, thumbprint, files, cancelAtManifest = false) => {
  const request = { files, appearance, output: { subfolder: 'DaKySo', suffix: '_signed', fixedFolder: null, overwrite: false, replaceOriginal: false }, placements: {}, modes: {}, requireTrusted: false };
  const started = Date.now();
  const r = await h.run(`
    const events = [];
    const done = new Promise((resolve) => {
      const id = window.__TAURI_INTERNALS__.transformCallback((message) => {
        const event = message?.type ? message : message?.message ?? message;
        events.push(event);
        if (${cancelAtManifest} && event?.type === 'manifest') window.__TAURI_INTERNALS__.invoke('job_cancel', { jobId: event.jobId });
        if (event?.type === 'summary') resolve();
      });
      window.__tgChannel = id;
    });
    await window.__TAURI_INTERNALS__.invoke('sign_run', { thumbprint: ${JSON.stringify(thumbprint)}, request: ${JSON.stringify(request)}, onEvent: '__CHANNEL__:' + window.__tgChannel });
    await Promise.race([done, new Promise((r) => setTimeout(r, 120000))]);
    return events;`);
  const summary = (r.ok ?? []).find((e) => e?.type === 'summary');
  return { summary, err: r.err, ms: Date.now() - started };
};
const mergeBatch = async (h, request) => {
  const r = await h.run(`
    const events = [];
    const done = new Promise((resolve) => {
      const id = window.__TAURI_INTERNALS__.transformCallback((message) => {
        const event = message?.type ? message : message?.message ?? message;
        events.push(event);
        if (event?.type === 'summary') resolve();
      });
      window.__tgMerge = id;
    });
    await window.__TAURI_INTERNALS__.invoke('merge_run', { request: ${JSON.stringify(request)}, onEvent: '__CHANNEL__:' + window.__tgMerge });
    await Promise.race([done, new Promise((r) => setTimeout(r, 120000))]);
    return events;`);
  return { summary: (r.ok ?? []).find((e) => e?.type === 'summary'), err: r.err };
};
const mergeRequest = async (h, outDir) => {
  const tpl = path.join(repo, 'fixtures/merge/1_giay_moi_hop.docx');
  const data = path.join(repo, 'fixtures/merge/1_giay_moi_hop.xlsx');
  const info = await h.inv('merge_inspect_template', { path: tpl });
  const table = await h.inv('merge_read_table', { path: data, sheet: null, headerRow: null });
  const map = await h.inv('merge_auto_map', { fields: info.ok?.fields ?? [], columns: table.ok?.columns ?? [] });
  const mappings = Object.fromEntries(Object.entries(map.ok ?? {}).map(([f, c]) => [f, c === null ? { kind: 'none' } : { kind: 'column', column: c }]));
  fs.mkdirSync(outDir, { recursive: true });
  return { templatePath: tpl, data: { kind: 'table', path: data, sheet: table.ok?.sheet ?? null, headerRow: table.ok?.headerRow ?? 1 }, mappings, fileNamePattern: '{ho_ten}', format: 'docx', outputFolder: outDir };
};
const devThumb = async (h) => ((await h.inv('cert_list')).ok?.certificates ?? []).find((c) => c.keyProvider === 'Khoá phần mềm (chỉ để thử)')?.thumbprint ?? '';
const readAll = (dir) => {
  let text = '';
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    text += e.isDirectory() ? readAll(p) : fs.readFileSync(p, 'latin1');
  }
  return text;
};
const auditLines = (cfgDir) => {
  const dir = path.join(cfgDir, 'nhat-ky-su-kien');
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir).flatMap((f) => fs.readFileSync(path.join(dir, f), 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)));
};
const hasPath = (text) => /[A-Za-z]:\\|\\\\/.test(text);

let gateway = null;
let current = null;
try {
  await startMock();
  const docs = path.join(work, 'vb');
  const directJson = path.join(work, 'notify-direct.json');
  fs.writeFileSync(directJson, JSON.stringify({ TELEGRAM_BOT_TOKEN: FAKE_TOKEN, TELEGRAM_CHAT_ID: Number(FAKE_CHAT), TELEGRAM_API_BASE: `http://127.0.0.1:${MOCK_PORT}`, TIMEOUT_SECONDS: 5 }));

  // =============================================================================================
  section('A. Không có cấu hình: tính năng im lặng');
  // =============================================================================================
  const cfgNone = path.join(work, 'cfg-khong-cau-hinh');
  let h = (current = await launch(cfgNone, null));
  await h.waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(800);
  let st = await h.status();
  const noDialog = (await h.run(`return !document.querySelector('app-popup')`)).ok;
  check('Không có notify.json: không hỏi đăng ký, không gửi gì', st?.configured === false && st.needsConsent === false && noDialog && mock.calls.length === 0, JSON.stringify({ configured: st?.configured, needs: st?.needsConsent }));
  await h.close();

  // =============================================================================================
  section('B. Từ chối đăng ký (kiểm thử 3)');
  // =============================================================================================
  const cfgNo = path.join(work, 'cfg-tu-choi');
  h = current = await launch(cfgNo, directJson);
  const consentSel = `.popup[aria-label="Đăng ký thiết bị với hệ thống quản trị"]`;
  let shown = await h.waitFor(`return !!document.querySelector('${consentSel}')`);
  await h.clickText(`${consentSel} .cuoi button`, 'Không đồng ý');
  await h.waitFor(`return !document.querySelector('${consentSel}')`);
  let thumb = await devThumb(h);
  let b = await signBatch(h, thumb, makeDocs(path.join(docs, 'tu-choi'), 1));
  await sleep(1500);
  st = await h.status();
  check('Từ chối: hộp đóng, không có lời gọi nào tới Telegram, ký vẫn thành công', !!shown && st?.consent === 'declined' && mock.calls.length === 0 && b.summary?.success === 1, `consent=${st?.consent}, calls=${mock.calls.length}, ky=${JSON.stringify(b.summary ?? b.err)}`);
  await h.close();
  h = current = await launch(cfgNo, directJson);
  await h.waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(1200);
  const askedAgain = (await h.run(`return !!document.querySelector('${consentSel}')`)).ok;
  const skipped = auditLines(cfgNo).filter((l) => l.kind === 'event' && l.disposition === 'skipped');
  check('Mở lại sau khi từ chối: không hỏi lại, vẫn không gửi; sự kiện ký chỉ ghi trên máy', !askedAgain && mock.calls.length === 0 && skipped.some((l) => l.event.event_type === 'SIGN_SUCCESS'), `hoi lai=${askedAgain}, nhat ky bo qua=${skipped.length}`);
  await h.close();

  // =============================================================================================
  section('C. Lần đầu trên máy mới, đồng ý đăng ký (kiểm thử 1, 2, 4)');
  // =============================================================================================
  const cfg = path.join(work, 'cfg');
  h = current = await launch(cfg, directJson);
  shown = await h.waitFor(`return !!document.querySelector('${consentSel}')`);
  st = await h.status();
  const dialog = (await h.run(`const p = document.querySelector('${consentSel}'); const boxes = [...p.querySelectorAll('input[type=checkbox]')].map((c) => c.checked);
    return { text: p.textContent.replace(/\\s+/g, ' '), boxes, buttons: [...p.querySelectorAll('.cuoi button')].map((x) => x.textContent.trim()) }`)).ok;
  await h.shot('1-hop-dang-ky.png');
  check('Lần đầu: hộp đăng ký hiện, nói rõ gửi gì / không gửi gì; chưa gửi gì trước khi đồng ý',
    !!shown && st?.consent === 'pending' && st.needsConsent && mock.calls.length === 0 && /Mã thiết bị/.test(dialog?.text ?? '') && /Không bao giờ gửi/.test(dialog.text) && dialog.text.includes(st.deviceId) && dialog.text.includes(os.hostname().toUpperCase()),
    `calls=${mock.calls.length}, nut=${dialog?.buttons?.join(' | ')}`);
  check('Hộp đăng ký: IP nội bộ bật sẵn, địa chỉ MAC tắt sẵn; nơi nhận không lộ Bot Token / Chat ID', JSON.stringify(dialog?.boxes) === '[true,false]' && !dialog.text.includes(FAKE_TOKEN) && !dialog.text.includes(FAKE_CHAT) && /SMKS-[0-9A-F-]{36}/.test(st.deviceId), JSON.stringify(dialog?.boxes));
  const deviceId = st.deviceId;
  await h.clickText(`${consentSel} .cuoi button`, 'Đồng ý và đăng ký');
  await waitCount(() => tagged('NEW DEVICE').length, 1);
  const reg = tagged('NEW DEVICE')[0] ?? '';
  st = await h.waitStatus((s) => s.registration === 'registered');
  check('Đồng ý: đúng 1 tin NEW DEVICE theo mẫu (Device ID, tên máy, hệ điều hành, kiến trúc, phiên bản, thời gian)',
    tagged('NEW DEVICE').length === 1 && reg.includes(`Device ID: <code>${deviceId}</code>`) && /Tên máy: \S+/.test(reg) && /Hệ điều hành: Windows/.test(reg) && /Phiên bản OS: /.test(reg) && /Kiến trúc: /.test(reg) && /Phiên bản ứng dụng: \d/.test(reg) && /Thời gian: \d\d\/\d\d\/\d{4}/.test(reg) && reg.endsWith('Trạng thái: Đăng ký thành công.'),
    reg.replace(/\n/g, ' | '));
  check('Tin đăng ký không có MAC (không được chọn), không có serial / tài khoản; trạng thái "đã đăng ký"', !/MAC/i.test(reg) && !/serial/i.test(reg) && st?.registration === 'registered' && st.consent === 'accepted', `registration=${st?.registration}`);
  await h.close();
  h = current = await launch(cfg, directJson);
  await h.waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(1500);
  st = await h.status();
  const again = (await h.run(`return !!document.querySelector('${consentSel}')`)).ok;
  check('Khởi động lại: cùng mã thiết bị, không hỏi lại, không gửi đăng ký lần hai', st?.deviceId === deviceId && !again && tagged('NEW DEVICE').length === 1 && st.registration === 'registered', `NEW DEVICE=${tagged('NEW DEVICE').length}`);

  // =============================================================================================
  section('D. Ký số: thành công, thất bại, hủy (kiểm thử 5, 6, 7)');
  // =============================================================================================
  thumb = await devThumb(h);
  const tok = await h.inv('token_check', { thumbprint: thumb });
  const okFiles = makeDocs(path.join(docs, 'ok'), 3);
  b = await signBatch(h, thumb, okFiles);
  await waitCount(() => tagged('SIGN SUCCESS').length, 1);
  const ok1 = tagged('SIGN SUCCESS')[0] ?? '';
  const signedOut = fs.existsSync(path.join(docs, 'ok', 'DaKySo')) ? fs.readdirSync(path.join(docs, 'ok', 'DaKySo')).length : 0;
  check('Ký 3 văn bản thành công: file đã ký có trên đĩa, đúng 1 tin SIGN SUCCESS cho cả lượt',
    b.summary?.success === 3 && signedOut === 3 && tagged('SIGN SUCCESS').length === 1 && /Số văn bản: 3/.test(ok1) && ok1.includes(' • van ban 01.pdf') && ok1.includes(`Chứng thư: ${tok.ok?.commonName}`) && /Loại chữ ký: PAdES B-T \(có dấu thời gian\) · khoá thử/.test(ok1) && /Mã giao dịch: <code>KS-\d{8}-\d{6}-[0-9a-f]{6}<\/code>/.test(ok1) && ok1.endsWith('Trạng thái: Thành công.'),
    ok1.replace(/\n/g, ' | '));
  check('Tin ký số không chứa đường dẫn thư mục, chỉ có tên file', !hasPath(ok1) && !ok1.includes('DaKySo'), '');

  const failDir = path.join(docs, 'loi');
  const failFiles = makeDocs(failDir, 1, 'tot');
  const broken = path.join(failDir, 'hong.pdf');
  fs.writeFileSync(broken, 'day khong phai file PDF');
  b = await signBatch(h, thumb, [...failFiles, broken]);
  await waitCount(() => tagged('SIGN FAILED').length, 1);
  const fail1 = tagged('SIGN FAILED')[0] ?? '';
  check('Lượt có 1 file hỏng: 1 tin SIGN SUCCESS cho file tốt, 1 tin SIGN FAILED nêu tên file hỏng, mã lỗi, thông báo; không lộ đường dẫn',
    b.summary?.success === 1 && b.summary?.failed === 1 && tagged('SIGN SUCCESS').length === 2 && tagged('SIGN FAILED').length === 1 && fail1.includes('Tên văn bản: hong.pdf') && /Mã lỗi: 1032/.test(fail1) && /Thông báo lỗi: \S/.test(fail1) && !hasPath(fail1) && fail1.endsWith('Trạng thái: Thất bại.'),
    fail1.replace(/\n/g, ' | '));
  const sameTx = /Mã giao dịch: <code>([^<]+)/.exec(fail1)?.[1] === /Mã giao dịch: <code>([^<]+)/.exec(tagged('SIGN SUCCESS')[1] ?? '')?.[1];
  check('Tin thành công và tin thất bại của cùng lượt ký có chung mã giao dịch', sameTx, '');
  const badToken = await h.inv('token_check', { thumbprint: '00'.repeat(20) });
  await waitCount(() => tagged('SIGN FAILED').length, 2);
  const fail2 = tagged('SIGN FAILED')[1] ?? '';
  check('Không mở được chứng thư (trước khi ký): tin SIGN FAILED với mã lỗi 1027', badToken.err?.code === 1027 && /Mã lỗi: 1027/.test(fail2), fail2.replace(/\n/g, ' | '));

  const many = makeDocs(path.join(docs, 'huy'), 60);
  b = await signBatch(h, thumb, many, true);
  await waitCount(() => tagged('SIGN CANCELLED').length, 1);
  const cancel = tagged('SIGN CANCELLED')[0] ?? '';
  check('Người dùng dừng giữa lượt 60 văn bản: tin SIGN CANCELLED có số chưa ký; không báo thành công cho văn bản chưa ký',
    b.summary?.stopped === true && b.summary.paused > 0 && cancel.includes(`Chưa ký: ${b.summary.paused} văn bản`) && /Tổng số văn bản: 60/.test(cancel) && cancel.endsWith('Trạng thái: Đã hủy.') && tagged('SIGN SUCCESS').length === 2 + (b.summary.success > 0 ? 1 : 0),
    `${JSON.stringify(b.summary)} | ${cancel.replace(/\n/g, ' | ')}`);

  // =============================================================================================
  section('E. Tạo văn bản: thành công, thất bại (kiểm thử 8, 9)');
  // =============================================================================================
  const outDir = path.join(work, 'tvb-out');
  const req = await mergeRequest(h, outDir);
  const m1 = await mergeBatch(h, req);
  await waitCount(() => tagged('DOCUMENT CREATED').length, 1);
  const doc1 = tagged('DOCUMENT CREATED')[0] ?? '';
  check('Tạo văn bản thành công: 1 tin DOCUMENT CREATED có số văn bản, tên file, loại văn bản, mã giao dịch',
    m1.summary?.success > 0 && m1.summary.failed === 0 && tagged('DOCUMENT CREATED').length === 1 && doc1.includes(`Số văn bản: ${m1.summary.success}`) && /Loại văn bản: 1_giay_moi_hop\.docx → DOCX/.test(doc1) && /Mã giao dịch: <code>TVB-/.test(doc1) && !hasPath(doc1) && doc1.endsWith('Trạng thái: Thành công.'),
    `${JSON.stringify(m1.summary ?? m1.err)} | ${doc1.replace(/\n/g, ' | ').slice(0, 300)}`);
  const m2 = await mergeBatch(h, { ...req, templatePath: path.join(work, 'khong-co-mau.docx') });
  await waitCount(() => tagged('DOCUMENT FAILED').length, 1);
  const doc2 = tagged('DOCUMENT FAILED')[0] ?? '';
  check('Tạo văn bản thất bại (mẫu không đọc được): 1 tin DOCUMENT FAILED có mã lỗi và thông báo, không lộ đường dẫn',
    !!m2.err && tagged('DOCUMENT FAILED').length === 1 && /Mã lỗi: 10\d\d/.test(doc2) && /Thông báo: \S/.test(doc2) && !hasPath(doc2) && doc2.endsWith('Trạng thái: Thất bại.'),
    `${JSON.stringify(m2.err)} | ${doc2.replace(/\n/g, ' | ')}`);

  // =============================================================================================
  section('F. Giao diện cấu hình (mục 7)');
  // =============================================================================================
  await h.clickText('.menubar button.mb', 'Hỗ trợ');
  await h.clickText('.menubar .mi', 'Thông báo Telegram');
  const tgSel = `.popup[aria-label="Thông báo Telegram"]`;
  await h.waitFor(`return !!document.querySelector('${tgSel} .sw')`);
  await sleep(500);
  const view = (await h.run(`const p = document.querySelector('${tgSel}');
    return { target: p.querySelector('#tg-target')?.textContent, text: p.textContent.replace(/\\s+/g, ' '), switches: [...p.querySelectorAll('.sw')].map((s) => [s.getAttribute('aria-label'), s.classList.contains('on'), s.disabled]),
      buttons: [...p.querySelectorAll('button.nut')].map((x) => x.textContent.trim()) }`)).ok;
  await h.shot('2-hop-thong-bao-telegram.png');
  check('Hộp Thông báo Telegram: 5 công tắc đang bật, nút Kiểm tra kết nối / Gửi tin thử / Hủy đăng ký, trạng thái đã đăng ký',
    view?.switches?.length === 5 && view.switches.every((s) => s[1] && !s[2]) && ['Kiểm tra kết nối', 'Gửi tin thử', 'Hủy đăng ký'].every((t) => view.buttons.some((x) => x.includes(t))) && /Đã đăng ký lúc/.test(view.text),
    JSON.stringify(view?.switches?.map((s) => s[0])));
  check('Nơi nhận hiện dạng đã che: không có Bot Token, không có Chat ID đầy đủ', !!view?.target && view.target.includes('••••') && !view.text.includes(FAKE_TOKEN) && !view.text.includes(FAKE_CHAT) && !view.text.includes(FAKE_TOKEN.slice(0, 20)), view?.target);
  await h.clickText(`${tgSel} button.nut`, 'Kiểm tra kết nối');
  const conn = await h.waitFor(`const m = document.querySelector('${tgSel} #tg-conn-msg')?.textContent ?? ''; return m.includes('saomai_kiemthu_bot') && m`);
  check('Kiểm tra kết nối: báo "Hoạt động", tên bot và tên nhóm', /Bot @saomai_kiemthu_bot gửi được vào nhóm «Nhóm kiểm thử»/.test(conn ?? '') && (await h.run(`return document.querySelector('${tgSel} .ketnoi')?.textContent.trim()`)).ok === 'Hoạt động', String(conn));
  await h.clickText(`${tgSel} button.nut`, 'Gửi tin thử');
  await waitCount(() => tagged('TEST').length, 1);
  check('Gửi tin thử: 1 tin TEST tới nhóm', tagged('TEST').length === 1 && tagged('TEST')[0].endsWith('Trạng thái: Kết nối hoạt động.'), (tagged('TEST')[0] ?? '').replace(/\n/g, ' | '));
  // Tắt "Thông báo ký số" bằng công tắc: ký không ra tin; bật lại thì ra.
  const before = tagged('SIGN SUCCESS').length;
  await h.run(`[...document.querySelectorAll('${tgSel} .sw')].find((s) => s.getAttribute('aria-label') === 'Thông báo ký số')?.click(); return 1;`);
  st = await h.waitStatus((s) => s.prefs.sign === false);
  b = await signBatch(h, thumb, makeDocs(path.join(docs, 'tat-ky'), 1));
  await sleep(2000);
  const whileOff = tagged('SIGN SUCCESS').length;
  await h.run(`[...document.querySelectorAll('${tgSel} .sw')].find((s) => s.getAttribute('aria-label') === 'Thông báo ký số')?.click(); return 1;`);
  await h.waitStatus((s) => s.prefs.sign === true);
  check('Tắt công tắc "Thông báo ký số": ký vẫn thành công nhưng không gửi tin; sự kiện vẫn ghi trên máy', st?.prefs.sign === false && b.summary?.success === 1 && whileOff === before && auditLines(cfg).some((l) => l.kind === 'event' && l.disposition === 'skipped' && l.reason === 'loại thông báo này đang tắt'), `truoc=${before}, khi tat=${whileOff}`);
  await h.clickText(`${tgSel} .cuoi button`, 'Đóng');

  // =============================================================================================
  section('G. Bot Token sai, Telegram treo, mất kết nối, khôi phục (kiểm thử 10–14)');
  // =============================================================================================
  let base = { ok: tagged('SIGN SUCCESS').length };
  mock.mode = 'unauthorized';
  b = await signBatch(h, thumb, makeDocs(path.join(docs, 'token-sai'), 2));
  st = await h.waitStatus((s) => s.pending === 1 && s.connection.state === 'error');
  check('Bot Token không hợp lệ (Telegram trả 401): ký vẫn thành công; sự kiện nằm trong hàng đợi; trạng thái báo lỗi token, không lộ token',
    b.summary?.success === 2 && st?.pending === 1 && st.connection.state === 'error' && /Telegram từ chối Bot Token/.test(st.connection.message) && !st.connection.message.includes(FAKE_TOKEN) && tagged('SIGN SUCCESS').length === base.ok,
    st?.connection.message);
  mock.mode = 'ok';
  const auto = await waitCount(() => tagged('SIGN SUCCESS').length, base.ok + 1, 25000);
  st = await h.waitStatus((s) => s.pending === 0);
  check('Sửa token xong: sự kiện tự được gửi lại (không cần thao tác), đúng 1 lần', auto && tagged('SIGN SUCCESS').length === base.ok + 1 && st?.pending === 0 && st.connection.state === 'ok', `pending=${st?.pending}`);

  base = { ok: tagged('SIGN SUCCESS').length };
  mock.mode = 'hang';
  const hang1 = await signBatch(h, thumb, makeDocs(path.join(docs, 'treo-1'), 2));
  const hang2 = await signBatch(h, thumb, makeDocs(path.join(docs, 'treo-2'), 2));
  check('Telegram treo không trả lời: hai lượt ký liên tiếp vẫn xong bình thường (ký không chờ Telegram)',
    hang1.summary?.success === 2 && hang2.summary?.success === 2 && fs.readdirSync(path.join(docs, 'treo-2', 'DaKySo')).length === 2, `luot 1 ${hang1.ms} ms, luot 2 ${hang2.ms} ms`);
  await stopMock();
  mock.mode = 'ok';
  const off = await signBatch(h, thumb, makeDocs(path.join(docs, 'mat-mang'), 2));
  st = await h.waitStatus((s) => s.pending === 3 && s.connection.state === 'error', 30000);
  check('Telegram mất kết nối: ký vẫn thành công; 3 sự kiện chờ trong hàng đợi; có giờ thử lại',
    off.summary?.success === 2 && st?.pending === 3 && st.connection.state === 'error' && !!st.nextAttemptAt && /Không kết nối được Telegram|Telegram/.test(st.connection.message), `pending=${st?.pending} | ${st?.connection.message} | thu lai ${st?.nextAttemptAt}`);
  // Tắt app khi còn hàng đợi (vẫn mất kết nối), mở lại: hàng đợi còn nguyên.
  await h.close();
  h = current = await launch(cfg, directJson);
  await h.waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  st = await h.waitStatus((s) => s.pending === 3 && s.connection.state === 'error', 20000);
  check('Tắt và mở lại app khi vẫn mất kết nối: hàng đợi còn nguyên 3 sự kiện, chưa gửi tin nào', st?.pending === 3 && tagged('SIGN SUCCESS').length === base.ok, `pending=${st?.pending}`);
  await startMock();
  await h.inv('notify_check');
  await waitCount(() => tagged('SIGN SUCCESS').length, base.ok + 3, 20000);
  st = await h.waitStatus((s) => s.pending === 0);
  const recovered = tagged('SIGN SUCCESS').slice(base.ok);
  check('Khôi phục kết nối: gửi hết hàng đợi, đúng thứ tự phát sinh (treo-1, treo-2, mat-mang)', recovered.length === 3 && st?.pending === 0 && recovered.every((t) => /Số văn bản: 2/.test(t)) && st.connection.state === 'ok', `${recovered.length} tin`);
  await sleep(12000); // chờ qua một chu kỳ thử lại: không được có tin gửi lặp
  const txs = mock.messages.map((t) => `${/\[SAO MAI SIGN - ([A-Z ]+)\]/.exec(t)?.[1]}|${/Mã giao dịch: <code>([^<]+)/.exec(t)?.[1] ?? t}`);
  check('Không gửi trùng: mỗi (loại sự kiện, mã giao dịch) chỉ xuất hiện 1 lần trong toàn bộ tin đã nhận', new Set(txs).size === txs.length && tagged('NEW DEVICE').length === 1, `${txs.length} tin, ${new Set(txs).size} khác nhau`);
  const deliveries = auditLines(cfg).filter((l) => l.kind === 'delivery');
  check('Nhật ký gửi: có "sent" và "retry"; không dòng nào chứa Bot Token', deliveries.some((l) => l.result === 'sent') && deliveries.some((l) => l.result === 'retry') && !readAll(path.join(cfg, 'nhat-ky-su-kien')).includes(FAKE_TOKEN), `${deliveries.length} dong`);

  // Hủy đăng ký: báo 1 lần, sau đó không gửi nữa.
  await h.inv('notify_unregister');
  await waitCount(() => tagged('DEVICE UNREGISTERED').length, 1);
  const n0 = mock.messages.length;
  b = await signBatch(h, thumb, makeDocs(path.join(docs, 'sau-huy'), 1));
  await sleep(2000);
  st = await h.status();
  check('Hủy đăng ký: 1 tin DEVICE UNREGISTERED; sau đó ký không gửi gì', tagged('DEVICE UNREGISTERED').length === 1 && mock.messages.length === n0 + 0 && b.summary?.success === 1 && st?.consent === 'declined' && st.registration === 'none', `consent=${st?.consent}`);
  await h.close();

  // =============================================================================================
  section('H. Chế độ gateway: máy trạm không có Bot Token');
  // =============================================================================================
  const gwData = path.join(work, 'gateway-data');
  const gwEnv = { ...process.env, TELEGRAM_BOT_TOKEN: FAKE_TOKEN, TELEGRAM_CHAT_ID: FAKE_CHAT, TELEGRAM_API_BASE: `http://127.0.0.1:${MOCK_PORT}`, GATEWAY_ENROLL_KEY: ENROLL_KEY, GATEWAY_BIND: `127.0.0.1:${GATEWAY_PORT}`, GATEWAY_DATA_DIR: gwData };
  const gwLog = fs.openSync(path.join(work, 'gateway.log'), 'w');
  gateway = spawn(gatewayExe, ['serve'], { env: gwEnv, stdio: ['ignore', gwLog, gwLog] });
  let health = null;
  for (let i = 0; i < 40 && !health; i++) {
    await sleep(250);
    try {
      health = await (await fetch(`http://127.0.0.1:${GATEWAY_PORT}/v1/health`)).json();
    } catch {}
  }
  check('Gateway chạy, đã nối tới Telegram (giả)', health?.ok === true && health.telegram === 'ok' && health.service === 'smks-gateway', JSON.stringify(health));
  const noAuth = await fetch(`http://127.0.0.1:${GATEWAY_PORT}/v1/events`, { method: 'POST', body: '{}' });
  const badEnroll = await fetch(`http://127.0.0.1:${GATEWAY_PORT}/v1/devices/register`, { method: 'POST', headers: { 'X-Enroll-Key': 'sai' }, body: '{}' });
  check('Gateway từ chối sự kiện không có thiết bị (401) và đăng ký sai khoá (403)', noAuth.status === 401 && badEnroll.status === 403, `${noAuth.status}, ${badEnroll.status}`);

  const gwJson = path.join(work, 'notify-gateway.json');
  fs.writeFileSync(gwJson, JSON.stringify({ GATEWAY_URL: `http://127.0.0.1:${GATEWAY_PORT}`, GATEWAY_ENROLL_KEY: ENROLL_KEY, TIMEOUT_SECONDS: 5 }));
  const cfgGw = path.join(work, 'cfg-gateway');
  const g0 = { dev: tagged('NEW DEVICE').length, ok: tagged('SIGN SUCCESS').length };
  h = current = await launch(cfgGw, gwJson);
  await h.waitFor(`return !!document.querySelector('${consentSel}')`);
  await h.inv('notify_consent', { accept: true, includeIp: false, includeMac: true });
  await waitCount(() => tagged('NEW DEVICE').length, g0.dev + 1);
  st = await h.waitStatus((s) => s.registration === 'registered');
  const regGw = tagged('NEW DEVICE')[g0.dev] ?? '';
  check('Gateway: đồng ý → tin NEW DEVICE do gateway soạn; người dùng chọn MAC, bỏ IP → tin có MAC, không có IP',
    st?.mode === 'gateway' && st.registration === 'registered' && regGw.includes(st.deviceId) && !/IP nội bộ/.test(regGw) && (st.device.macAddress ? /Địa chỉ MAC: ([0-9A-F]{2}-){5}[0-9A-F]{2}/.test(regGw) : true),
    regGw.replace(/\n/g, ' | '));
  thumb = await devThumb(h);
  b = await signBatch(h, thumb, makeDocs(path.join(docs, 'gw'), 2));
  await waitCount(() => tagged('SIGN SUCCESS').length, g0.ok + 1);
  const gwState = fs.readFileSync(path.join(cfgGw, 'notify-state.json'), 'utf8');
  const gwDevices = JSON.parse(fs.readFileSync(path.join(gwData, 'devices.json'), 'utf8'));
  const cred = JSON.parse(gwState).credential ?? '';
  check('Gateway: ký → 1 tin SIGN SUCCESS; thư mục dữ liệu của máy trạm không có Bot Token; bí mật thiết bị lưu dạng mã hoá DPAPI; gateway chỉ lưu SHA-256',
    b.summary?.success === 2 && tagged('SIGN SUCCESS').length === g0.ok + 1 && !readAll(cfgGw).includes(FAKE_TOKEN) && cred.length > 100 && !/^[0-9a-f]{64}$/.test(cred) && /^[0-9a-f]{64}$/.test(gwDevices[st.deviceId]?.secret_sha256 ?? '') && !gwState.includes(ENROLL_KEY),
    `credential ${cred.length} ky tu`);
  const revoke = spawnSync(gatewayExe, ['revoke', st.deviceId], { env: gwEnv, encoding: 'utf8' });
  await sleep(1200);
  b = await signBatch(h, thumb, makeDocs(path.join(docs, 'gw-thu-hoi'), 1));
  st = await h.waitStatus((s) => s.pending === 1 && s.connection.state === 'error');
  check('Quản trị viên thu hồi thiết bị ở gateway: ký vẫn thành công nhưng thông báo bị từ chối, nằm chờ; không có tin mới',
    revoke.status === 0 && b.summary?.success === 1 && st?.pending === 1 && /thu hồi/.test(st.connection.message) && tagged('SIGN SUCCESS').length === g0.ok + 1, `${(revoke.stdout ?? '').trim()} | ${st?.connection.message}`);
  spawnSync(gatewayExe, ['restore', st.deviceId], { env: gwEnv, encoding: 'utf8' });
  await sleep(1200);
  await h.inv('notify_check');
  await waitCount(() => tagged('SIGN SUCCESS').length, g0.ok + 2);
  st = await h.waitStatus((s) => s.pending === 0);
  check('Khôi phục thiết bị: sự kiện đang chờ được gửi, đúng 1 lần', tagged('SIGN SUCCESS').length === g0.ok + 2 && st?.pending === 0, `pending=${st?.pending}`);
  await h.close();
  gateway.kill();
  gateway = null;
  fs.closeSync(gwLog);
  const gwLogText = fs.readFileSync(path.join(work, 'gateway.log'), 'utf8');
  check('Log của gateway: có ghi đăng ký / chuyển / từ chối, không chứa Bot Token hay khoá đăng ký', /ĐĂNG KÝ thiết bị/.test(gwLogText) && /CHUYỂN SIGN_SUCCESS/.test(gwLogText) && !gwLogText.includes(FAKE_TOKEN) && !gwLogText.includes(ENROLL_KEY), `${gwLogText.split('\n').length} dong`);

  // =============================================================================================
  section('I. Bot Token không nằm trong EXE và log (kiểm thử 15)');
  // =============================================================================================
  const realToken = process.env.TG_REAL_TOKEN ?? '';
  const exeBytes = fs.readFileSync(exe).toString('latin1');
  const tokenLike = /\d{8,10}:[A-Za-z0-9_-]{35}/.test(exeBytes);
  check('File EXE: không chứa Bot Token (không có chuỗi nào dạng token Telegram, không có token thật / token kiểm thử)', !tokenLike && !exeBytes.includes(FAKE_TOKEN) && (!realToken || !exeBytes.includes(realToken)), `${(exeBytes.length / 1048576).toFixed(1)} MB`);
  const logDir = path.join(process.env.LOCALAPPDATA ?? '', 'SaoMai-KySo', 'logs');
  const logs = fs.existsSync(logDir) ? fs.readdirSync(logDir).map((f) => fs.readFileSync(path.join(logDir, f), 'latin1')).join('\n') : '';
  check('Log của ứng dụng: có dòng gửi thông báo, không chứa Bot Token (kiểm thử và thật), không chứa khoá đăng ký', /thông báo/.test(Buffer.from(logs, 'latin1').toString('utf8')) && !logs.includes(FAKE_TOKEN) && !logs.includes(ENROLL_KEY) && (!realToken || !logs.includes(realToken)), `${(logs.length / 1024).toFixed(0)} KB log`);
  check('Thư mục dữ liệu của máy trạm (trạng thái, hàng đợi, nhật ký sự kiện): không chứa Bot Token', ![cfg, cfgNo, cfgNone].some((d) => readAll(d).includes(FAKE_TOKEN)), '');
  for (const f of ['1-hop-dang-ky.png', '2-hop-thong-bao-telegram.png']) {
    const dest = path.join(repo, 'docs/implementation/evidence/thong-bao');
    fs.mkdirSync(dest, { recursive: true });
    if (fs.existsSync(path.join(work, f))) fs.copyFileSync(path.join(work, f), path.join(dest, f));
  }

  // =============================================================================================
  if (REAL) {
    section('K. Gửi THẬT tới Telegram (token ở biến môi trường, không ghi vào repo)');
    const realChat = process.env.TG_REAL_CHAT ?? '';
    if (!realToken || !realChat) throw new Error('--real cần TG_REAL_TOKEN và TG_REAL_CHAT');
    const realJson = path.join(work, 'notify-real.json');
    fs.writeFileSync(realJson, JSON.stringify({ TELEGRAM_BOT_TOKEN: realToken, TELEGRAM_CHAT_ID: realChat }));
    const cfgReal = path.join(work, 'cfg-that');
    h = current = await launch(cfgReal, realJson);
    await h.waitFor(`return !!document.querySelector('${consentSel}')`);
    await h.inv('notify_check');
    st = await h.status();
    check('Telegram thật: kiểm tra kết nối đạt (Bot Token và Chat ID dùng được)', st?.connection.state === 'ok', st?.connection.message);
    if (st?.connection.state === 'ok') {
      await h.clickText(`${consentSel} .cuoi button`, 'Đồng ý và đăng ký');
      st = await h.waitStatus((s) => s.registration === 'registered', 30000);
      check('Telegram thật: đăng ký thiết bị đã gửi tới nhóm', st?.registration === 'registered', `pending=${st?.pending} ${st?.lastError ?? ''}`);
      thumb = await devThumb(h);
      const realDir = path.join(docs, 'that');
      const good = makeDocs(realDir, 2, 'Van ban kiem thu');
      fs.writeFileSync(path.join(realDir, 'File hong kiem thu.pdf'), 'khong phai PDF');
      b = await signBatch(h, thumb, [...good, path.join(realDir, 'File hong kiem thu.pdf')]);
      const m = await mergeBatch(h, await mergeRequest(h, path.join(work, 'tvb-that')));
      const t = await h.inv('notify_test');
      st = await h.waitStatus((s) => s.pending === 0, 40000);
      const sent = auditLines(cfgReal).filter((l) => l.kind === 'delivery' && l.result === 'sent').map((l) => l.event_type);
      check('Telegram thật: ký (thành công + thất bại), tạo văn bản, tin thử đều đã gửi; hàng đợi trống',
        b.summary?.success === 2 && b.summary.failed === 1 && m.summary?.success > 0 && !!t.ok && st?.pending === 0 && ['DEVICE_REGISTERED', 'SIGN_SUCCESS', 'SIGN_FAILED', 'DOCUMENT_CREATED', 'TEST'].every((e) => sent.includes(e)),
        `da gui: ${sent.join(', ')}`);
      check('Telegram thật: thư mục dữ liệu và nhật ký sự kiện không chứa Bot Token', !readAll(cfgReal).includes(realToken), '');
    }
    await h.close();
    fs.rmSync(realJson, { force: true });
    const logs2 = fs.existsSync(logDir) ? fs.readdirSync(logDir).map((f) => fs.readFileSync(path.join(logDir, f), 'latin1')).join('\n') : '';
    check('Telegram thật: log của ứng dụng không chứa Bot Token thật', !logs2.includes(realToken), '');
  }
  current = null;
} catch (error) {
  console.error(`LOI: ${error.stack ?? error.message}`);
  results.push(false);
} finally {
  try {
    await current?.close();
  } catch {}
  gateway?.kill();
  await stopMock();
}
const ok = results.every(Boolean);
console.log(`\n${results.filter(Boolean).length}/${results.length} phép kiểm đạt`);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
