// Kiểm tra Tạo văn bản qua IPC thật của file chạy: gọi các command như giao diện gọi (qua
// window.__TAURI_INTERNALS__ trong WebView2), kể cả chạy lô qua Channel và đọc bản xem trước qua asset protocol.
// Không dùng giao diện; kiểm lõi + quyền + hợp đồng dữ liệu.
//
// Cách dùng: node app/tests/tvb-ipc.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục kết quả>
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, outDir] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9336;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

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
  /** Chạy một biểu thức async trong trang, trả về giá trị JSON hoặc { lỗi }. */
  const run = async (body) => {
    const reply = await send('Runtime.evaluate', {
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: e }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    return reply.result?.result?.value ?? { err: reply.result?.exceptionDetails?.text };
  };
  for (let i = 0; i < 40; i++) {
    if ((await run('return !!window.__TAURI_INTERNALS__')).ok) break;
    await sleep(250);
  }

  const tpl = path.join(repo, 'fixtures/merge/3_hop_dong_lao_dong.docx');
  const data = path.join(repo, 'fixtures/merge/3_hop_dong_lao_dong.xlsx');
  const inv = (cmd, args) => run(`return await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);

  const info = await inv('app_info', {});
  check('app_info có userName, computerName, wordAvailable', !!info.ok?.userName && info.ok?.wordAvailable === true, JSON.stringify(info.ok ?? info.err));

  const template = await inv('merge_inspect_template', { path: tpl });
  check('merge_inspect_template: 18 biến', template.ok?.fields?.length === 18, `${template.ok?.fields?.length ?? JSON.stringify(template.err)}`);

  const table = await inv('merge_read_table', { path: data, sheet: null, headerRow: null });
  check('merge_read_table: 4 dòng, sheet DuLieu', table.ok?.rows?.length === 4 && table.ok?.sheet === 'DuLieu', JSON.stringify(table.err ?? ''));

  const map = await inv('merge_auto_map', { fields: template.ok.fields, columns: table.ok.columns });
  const mapped = Object.values(map.ok ?? {}).filter((v) => v !== null).length;
  check('merge_auto_map ghép đủ 18 biến', mapped === 18, `${mapped}`);

  const mappings = Object.fromEntries(Object.entries(map.ok).map(([f, c]) => [f, c === null ? { kind: 'none' } : { kind: 'column', column: c }]));
  const request = {
    templatePath: tpl,
    data: { kind: 'table', path: data, sheet: 'DuLieu', headerRow: table.ok.headerRow },
    mappings,
    fileNamePattern: '{so_hop_dong}_{ho_ten}',
    format: 'pdf',
    outputFolder: outDir,
  };
  const plan = await inv('merge_plan', { request });
  check('merge_plan: 4 file, không cảnh báo', plan.ok?.files?.length === 4 && plan.ok.warnings.length === 0, plan.ok?.files?.map((f) => f.fileName).join(', ') ?? JSON.stringify(plan.err));

  const bad = await inv('merge_plan', { request: { ...request, mappings: { ho_ten: { kind: 'column', column: 99 } } } });
  check('Lỗi nghiệp vụ trả về { code, message } tiếng Việt', bad.err?.code === 1044 && /Cột số 99/.test(bad.err?.message), JSON.stringify(bad.err));

  const preview = await inv('merge_preview', { request, rowNumber: 3 });
  check('merge_preview: PDF 3 trang', preview.ok?.pageCount === 3 && fs.existsSync(preview.ok.pdfPath), JSON.stringify(preview.ok ?? preview.err));
  if (preview.ok) {
    const fetched = await run(`const url = window.__TAURI_INTERNALS__.convertFileSrc(${JSON.stringify(preview.ok.pdfPath)}, 'asset');
      const r = await fetch(url); const b = new Uint8Array(await r.arrayBuffer());
      return { status: r.status, size: b.length, head: String.fromCharCode(...b.slice(0, 5)) };`);
    check('Giao diện đọc được bản xem trước qua asset protocol', fetched.ok?.status === 200 && fetched.ok?.head === '%PDF-', JSON.stringify(fetched.ok ?? fetched.err));
    const denied = await run(`const r = await fetch(window.__TAURI_INTERNALS__.convertFileSrc(${JSON.stringify(tpl)}, 'asset')); return r.status;`);
    check('Asset protocol KHÔNG cho đọc file ngoài phạm vi (file mẫu)', denied.ok !== 200, `status ${denied.ok ?? JSON.stringify(denied.err)}`);
  }

  // Chạy lô qua Channel như giao diện làm.
  const started = Date.now();
  const batch = await run(`
    const events = [];
    const done = new Promise((resolve) => {
      const id = window.__TAURI_INTERNALS__.transformCallback((message) => {
        // Channel của Tauri gửi { index, message } hoặc { message } tuỳ bản; lấy phần có 'type'.
        const event = message?.type ? message : message?.message ?? message;
        events.push(event);
        if (event?.type === 'summary') resolve();
      });
      window.__smksChannel = id;
    });
    const jobId = await window.__TAURI_INTERNALS__.invoke('merge_run', {
      request: ${JSON.stringify(request)},
      onEvent: '__CHANNEL__:' + window.__smksChannel,
    });
    await Promise.race([done, new Promise((r) => setTimeout(r, 120000))]);
    return { jobId, events };`);
  const events = batch.ok?.events ?? [];
  const summary = events.find((e) => e?.type === 'summary');
  check('merge_run: manifest, 4 sự kiện file, summary', events[0]?.type === 'manifest' && events.filter((e) => e?.type === 'file').length === 4 && !!summary,
    `${events.map((e) => e?.type).join(',')} ${batch.err ? JSON.stringify(batch.err) : ''}`);
  check('merge_run: 4 thành công, 0 lỗi', summary?.success === 4 && summary?.failed === 0, `${(Date.now() - started) / 1000}s, ${JSON.stringify(summary)}`);
  const files = summary ? fs.readdirSync(summary.outputFolder) : [];
  check('4 file PDF trong thư mục kết quả, không còn .tmp', files.length === 4 && files.every((f) => f.endsWith('.pdf')), files.join(', '));
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
