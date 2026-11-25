// Kiểm tra Ký số qua IPC thật của file chạy (build có feature dev-keys, khoá thử ở SMKS_DEV_PKI): gọi các
// command như giao diện gọi qua window.__TAURI_INTERNALS__, chạy lô qua Channel với TSA thật, Dừng giữa lô,
// đọc file kết quả qua asset protocol, xác minh bằng verify_file. Không dùng giao diện.
//
// Cách dùng: node app/tests/ks-ipc.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9338;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

// Thư mục làm việc: 3 PDF mẫu (tên có số để kiểm sắp xếp tự nhiên) và một file _daky_ của bản cũ.
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
const fixtures = ['word-hybrid-xref.pdf', 'xref-stream.pdf', 'table-xref.pdf'];
fixtures.forEach((f, i) => fs.copyFileSync(path.join(repo, 'fixtures/pdf', f), path.join(work, `van ban ${[1, 2, 10][i]}.pdf`)));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(work, 'cu_daky_20260101_101010.pdf'));
const originals = Object.fromEntries(fs.readdirSync(work).map((n) => [n, fs.readFileSync(path.join(work, n))]));

const app = spawn(exe, [], {
  env: { ...process.env, SMKS_DEV_PKI: pki, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
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
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: e }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    return reply.result?.result?.value ?? { err: reply.result?.exceptionDetails?.text };
  };
  const inv = (cmd, args = {}) => run(`return await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);
  for (let i = 0; i < 40; i++) {
    if ((await run('return !!window.__TAURI_INTERNALS__')).ok) break;
    await sleep(250);
  }

  // 1. Kho chứng thư: có khoá thử (dev-keys) ở đầu, các chứng thư của máy kèm lý do.
  const scan = await inv('cert_list');
  const certs = scan.ok?.certificates ?? [];
  const dev = certs.find((c) => c.keyProvider === 'Khoá phần mềm (chỉ để thử)');
  check('cert_list: có khoá thử và lý do cho chứng thư không ký được', !!dev && certs.filter((c) => !c.canSign).every((c) => !!c.reason), `${certs.length} chứng thư; ${scan.err ? JSON.stringify(scan.err) : ''}`);

  // 2. Liệt kê file.
  const list = await inv('sign_list_files', { sources: [work] });
  const names = (list.ok ?? []).map((f) => f.name);
  check('sign_list_files: sắp xếp tự nhiên, bỏ file _daky_ của bản cũ', JSON.stringify(names) === JSON.stringify(['van ban 1.pdf', 'van ban 2.pdf', 'van ban 10.pdf']), names.join(', '));
  const missing = await inv('sign_list_files', { sources: [path.join(work, 'khong-co')] });
  check('sign_list_files: nguồn không có trả lỗi {code, message}', missing.err?.code === 1029 && !!missing.err?.message, JSON.stringify(missing.err));

  // Ký cả thư mục (TD-50): lấy cả thư mục con, bỏ thư mục DaKySo cũ; nơi lưu dự kiến là <tên>_DaKySo cùng cấp, giữ cấu trúc.
  const tree = `${work}-cay`;
  fs.rmSync(tree, { recursive: true, force: true });
  fs.mkdirSync(path.join(tree, 'PhongA', 'To 1'), { recursive: true });
  fs.mkdirSync(path.join(tree, 'DaKySo'), { recursive: true });
  fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(tree, 'a.pdf'));
  fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(tree, 'PhongA', 'b.pdf'));
  fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(tree, 'PhongA', 'To 1', 'c.pdf'));
  fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(tree, 'DaKySo', 'a_signed.pdf'));
  const treeList = (await inv('sign_list_files', { sources: [tree] })).ok ?? [];
  check('sign_list_files thư mục có thư mục con: đủ 3 file, tên tương đối, có root, bỏ DaKySo cũ',
    JSON.stringify(treeList.map((f) => f.name)) === JSON.stringify(['a.pdf', 'PhongA\\b.pdf', 'PhongA\\To 1\\c.pdf']) && treeList.every((f) => f.root === tree),
    treeList.map((f) => `${f.name} (${f.root})`).join(', '));
  const treePlan = (await inv('sign_plan_outputs', {
    files: treeList.map((f) => f.path),
    output: { subfolder: 'DaKySo', suffix: '_signed', fixedFolder: null, overwrite: false, replaceOriginal: false, sourceRoots: [tree] },
  })).ok ?? [];
  const mirror = `${tree}_DaKySo`;
  check('sign_plan_outputs có sourceRoots: lưu vào <tên>_DaKySo cùng cấp, giữ thư mục con',
    JSON.stringify(treePlan.map((p) => p.output)) ===
      JSON.stringify([path.join(mirror, 'a_signed.pdf'), path.join(mirror, 'PhongA', 'b_signed.pdf'), path.join(mirror, 'PhongA', 'To 1', 'c_signed.pdf')]),
    treePlan.map((p) => p.output).join(', '));

  // 3. Mở token (khoá thử), chứng thư không có thì báo lỗi rõ.
  const bad = await inv('token_check', { thumbprint: '00'.repeat(20) });
  check('token_check: thumbprint không có trong kho → lỗi 1027', bad.err?.code === 1027, JSON.stringify(bad.err));
  const token = await inv('token_check', { thumbprint: dev?.thumbprint ?? '' });
  check('token_check: ký thử khoá thử đạt', token.ok?.dev === true && !!token.ok?.commonName, JSON.stringify(token.ok ?? token.err));

  // 4. Chạy lô qua Channel với TSA thật.
  const files = (list.ok ?? []).map((f) => f.path);
  const request = {
    files,
    appearance: { mode: 'text', kind: 'main', placements: [], fontFamily: 'Times New Roman', fontSize: 10, textColor: '#000000', reason: 'Ký số tài liệu', location: 'Việt Nam', sealImage: null, wetImage: null, signatureImage: null, sealDarkness: 140, wetDarkness: 140, wetStroke: 100, wetColor: null, stamps: [] },
    output: { subfolder: 'DaKySo', suffix: '_signed', fixedFolder: null, overwrite: false },
    placements: {},
    requireTrusted: false,
  };
  const runJob = (req, cancelAfterFirst) => run(`
    const events = [];
    let jobId = null;
    const done = new Promise((resolve) => {
      const id = window.__TAURI_INTERNALS__.transformCallback((message) => {
        const event = message?.type ? message : message?.message ?? message;
        events.push(event);
        if (${cancelAfterFirst} && event?.type === 'manifest') {
          window.__TAURI_INTERNALS__.invoke('job_cancel', { jobId: event.jobId });
        }
        if (event?.type === 'summary') resolve();
      });
      window.__ksChannel = id;
    });
    jobId = await window.__TAURI_INTERNALS__.invoke('sign_run', {
      thumbprint: ${JSON.stringify(dev?.thumbprint ?? '')},
      request: ${JSON.stringify(req)},
      onEvent: '__CHANNEL__:' + window.__ksChannel,
    });
    await Promise.race([done, new Promise((r) => setTimeout(r, 120000))]);
    return { jobId, events };`);
  const started = Date.now();
  const batch = await runJob(request, false);
  const events = batch.ok?.events ?? [];
  const summary = events.find((e) => e?.type === 'summary');
  check('sign_run: manifest, 3 sự kiện file, summary', events[0]?.type === 'manifest' && events.filter((e) => e?.type === 'file').length === 3 && !!summary,
    `${events.map((e) => e?.type).join(',')} ${batch.err ? JSON.stringify(batch.err) : ''}`);
  check('sign_run: 3 thành công, 0 lỗi, thư mục DaKySo', summary?.success === 3 && summary?.failed === 0 && /DaKySo$/.test(summary?.outputFolder ?? ''), `${(Date.now() - started) / 1000}s, ${JSON.stringify(summary)}`);
  const fileEvents = events.filter((e) => e?.type === 'file');
  check('Mỗi file có dấu thời gian, kết luận "người ký không thuộc CA" (khoá thử)', fileEvents.every((e) => e.success && e.trustedTime && e.verdict === 'untrustedSigner'), JSON.stringify(fileEvents.map((e) => [e.name, e.verdict, e.trustedTime])));
  const out = path.join(work, 'DaKySo');
  const outFiles = fs.existsSync(out) ? fs.readdirSync(out).sort() : [];
  check('3 file *_signed.pdf, không còn .tmp', JSON.stringify(outFiles) === JSON.stringify(['van ban 10_signed.pdf', 'van ban 1_signed.pdf', 'van ban 2_signed.pdf']), outFiles.join(', '));
  check('File gốc giữ nguyên từng byte', Object.entries(originals).every(([n, b]) => fs.readFileSync(path.join(work, n)).equals(b)));

  // 5. Xem kết quả qua asset protocol: chỉ file đã liệt kê / kết quả được phép.
  const signedPath = path.join(out, 'van ban 1_signed.pdf');
  const granted = await inv('sign_preview', { path: signedPath });
  const fetched = await run(`const r = await fetch(window.__TAURI_INTERNALS__.convertFileSrc(${JSON.stringify(signedPath)}, 'asset'));
      const b = new Uint8Array(await r.arrayBuffer()); return { status: r.status, head: String.fromCharCode(...b.slice(0, 5)) };`);
  check('sign_preview cho đọc file đã ký qua asset protocol', granted.ok === signedPath && fetched.ok?.status === 200 && fetched.ok?.head === '%PDF-', JSON.stringify(fetched.ok ?? fetched.err));
  const refused = await inv('sign_preview', { path: path.join(repo, 'fixtures/pdf/table-xref.pdf') });
  check('sign_preview từ chối file chưa liệt kê', !!refused.err, JSON.stringify(refused.err));

  // 6. Xác minh file vừa ký.
  const report = await inv('verify_file', { path: signedPath, revocation: false });
  const sig = report.ok?.signatures?.[0];
  check('verify_file: 1 chữ ký, dấu thời gian của TSA Ban Cơ yếu, khối hiển thị trang 1', report.ok?.signatures?.length === 1 && sig?.timestampAuthority === 'Máy chủ cấp dấu thời gian 02' && sig?.visible && sig?.page === 1, JSON.stringify(sig));

  // 7. Ký lại: giữ cả hai (thêm (1)); Tạm dừng ngay sau manifest: file chưa xử lý là "tạm dừng", không phải lỗi.
  const stopped = await runJob(request, true);
  const sEvents = stopped.ok?.events ?? [];
  const sSummary = sEvents.find((e) => e?.type === 'summary');
  const pausedFiles = sEvents.filter((e) => e?.type === 'file' && e.paused);
  check('Tạm dừng giữa lô: summary stopped, tổng = thành công + lỗi + tạm dừng, file tạm dừng có paused = true và không tính là lỗi',
    sSummary?.stopped === true && sSummary.success + sSummary.failed + sSummary.paused === 3 && pausedFiles.length === sSummary.paused && sSummary.failed === 0 &&
      pausedFiles.every((e) => !e.success && e.reason === 'Chưa ký: đã tạm dừng.'),
    JSON.stringify(sSummary));
  const again = await runJob(request, false);
  const aSummary = (again.ok?.events ?? []).find((e) => e?.type === 'summary');
  const outAfter = fs.readdirSync(out);
  check('Ký lại giữ cả hai bản (thêm " (1)")', aSummary?.success === 3 && outAfter.some((n) => n.includes('_signed (1).pdf')), outAfter.join(', '));
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
