// Kiểm màn Kiểm tra chữ ký số (TD-57) trên file chạy thật (bản `--features dev-keys`):
// nạp file bằng kéo thả (sự kiện tauri://drag-drop), đọc kết quả trên màn, tab Văn bản, Chi tiết chứng thư,
// bật / tắt kiểm thu hồi, xuất báo cáo ra PDF (đọc lại bằng PyMuPDF, độc lập với app).
// File mẫu: fixtures/pdf/verify (pyHanko ký) và một file app ký bằng `smks-cli sign --visible` với khoá thử.
//
// Cách dùng: node app/tests/kt-ui.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9362;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const shots = path.join(work, 'anh');
const docs = path.join(work, 'van-ban');
for (const d of [cfgDir, shots, docs]) fs.mkdirSync(d, { recursive: true });
const verifyDir = path.join(repo, 'fixtures/pdf/verify');
for (const f of ['sua-roi-ky-them.pdf', 'dau-thoi-gian-tai-lieu.pdf']) fs.copyFileSync(path.join(verifyDir, f), path.join(docs, f));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(docs, 'chua ky.pdf'));
fs.writeFileSync(path.join(docs, 'ghi chu.txt'), 'khong phai pdf');

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}

// File có chữ ký hiển thị do lõi ký (khoá thử + TSA Ban Cơ yếu).
const cli = path.join(repo, 'target/release/smks-cli.exe');
const visible = path.join(docs, 'ky hien thi.pdf');
const signed = spawnSync(cli, ['sign', path.join(repo, 'fixtures/pdf/table-xref.pdf'), visible,
  '--dev-cert', path.join(pki, 'signer.cert.der'), '--dev-key', path.join(pki, 'signer.key.pk8.der'),
  '--dev-chain', path.join(pki, 'ca.cert.der'), '--allow-untrusted', '--visible', '--at', '0.55,0.80,0.35,0.07'], { encoding: 'utf8' });
if (signed.status !== 0) {
  console.error(`LOI: smks-cli sign: ${signed.stdout}${signed.stderr}`);
  process.exit(1);
}
// Khung thật của widget chữ ký (tỉ lệ trang, gốc trên trái), đọc bằng PyMuPDF.
const widget = JSON.parse(spawnSync('python', ['-I', '-W', 'ignore', '-c', `
import sys, json, fitz
p = fitz.open(sys.argv[1])[0]; r = p.rect; w = next(p.widgets()).rect
print(json.dumps({"x": w.x0 / r.width, "y": w.y0 / r.height, "w": w.width / r.width}))`, visible], { encoding: 'utf8' }).stdout);

const app = spawn(exe, [], {
  env: {
    ...process.env,
    SMKS_DEV_PKI: pki,
    SMKS_CONFIG_DIR: cfgDir,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
  },
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
  const drop = (p) =>
    run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(p)}], position: { x: 500, y: 400 } } }); return 1;`);
  /** Đợi kiểm xong file `name` (ô đường dẫn đúng file, hết "Đang kiểm tra…"). */
  const loaded = (name, ms = 60000) =>
    waitFor(`const v = document.querySelector('.kiem-tra .hang input.fld')?.value ?? '';
      return v.endsWith(${JSON.stringify(name)}) && !document.querySelector('.kiem-tra .kt-tt .chip-tt.dang') && 1`, ms);
  const rows = () => run(`return [...document.querySelectorAll('.kiem-tra .kt-dong')].map((d) => d.textContent.replace(/\\s+/g, ' ').trim())`);
  const panel = () =>
    run(`const q = (s) => document.querySelector('.kiem-tra ' + s);
      return {
        kq: q('.kt-kq')?.className ?? '', kqText: q('.kt-kq')?.textContent.replace(/\\s+/g, ' ').trim() ?? '',
        checks: [...document.querySelectorAll('.kiem-tra .kt-dk-dong')].map((d) => d.dataset.id + ':' + d.dataset.state),
        checkText: [...document.querySelectorAll('.kiem-tra .kt-dk-dong')].map((d) => d.textContent.replace(/\\s+/g, ' ').trim()),
        vt: q('.kt-vt')?.textContent.replace(/\\s+/g, ' ').trim() ?? '',
        foot: q('.chan .dan')?.textContent.replace(/\\s+/g, ' ').trim() ?? '',
        head: q('.kt-tt')?.textContent.replace(/\\s+/g, ' ').trim() ?? '',
        canh: q('.kt-canh')?.textContent.replace(/\\s+/g, ' ').trim() ?? '',
      };`);

  await send('Runtime.enable');
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await sleep(500);

  // 1) Ctrl+E mở màn Kiểm tra: phần giới thiệu, chân màn, "Kiểm tra thu hồi" bật sẵn.
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'e', ctrlKey: true, bubbles: true })); return 1;`);
  const intro = await waitFor(`return /— Kiểm tra chữ ký số/.test(document.title) && document.querySelector('.kiem-tra .mo-dau h2')?.textContent.trim()`);
  const s1 = await run(`return { foot: document.querySelector('.kiem-tra .chan .dan')?.textContent.trim(), rev: document.querySelector('.kiem-tra .kt-thuhoi input')?.checked,
    dev: !!document.querySelector('.dang-phat-trien'), exportOff: [...document.querySelectorAll('.kiem-tra .chan button')].find((b) => b.textContent.includes('Xuất báo cáo'))?.disabled }`);
  await shot('1-gioi-thieu.png');
  check('Ctrl+E mở màn Kiểm tra (không còn "đang phát triển"): giới thiệu, chân "Chọn file PDF đã ký để kiểm tra.", kiểm thu hồi bật sẵn, Xuất báo cáo tắt',
    intro?.startsWith('Kiểm tra chữ ký số') && s1.foot === 'Chọn file PDF đã ký để kiểm tra.' && s1.rev === true && !s1.dev && s1.exportOff === true, JSON.stringify({ intro, s1 }));

  // 2) File do lõi ký (khoá thử, có TSA, chữ ký hiển thị): 5 điều kiện; người ký không thuộc CA Ban Cơ yếu.
  await drop(visible);
  await loaded('ky hien thi.pdf');
  await sleep(400);
  let r = await rows();
  let p = await panel();
  await shot('2-ky-hien-thi.png');
  check('File app ký: 1 chữ ký, kết luận "Người ký không thuộc CA Ban Cơ yếu" (đỏ), tiêu đề "0/1 hợp lệ"',
    r.length === 1 && r[0].includes('Người ký không thuộc CA Ban Cơ yếu') && p.kq.includes('loi') && p.head.includes('0/1 hợp lệ'), JSON.stringify({ r, p }));
  check('5 điều kiện Điều 30: toàn vẹn, khớp chứng thư, thời gian (TSA Ban Cơ yếu) đạt; chuỗi về Root G1/G2 không đạt',
    p.checks.join() .startsWith('integrity:ok,signer:ok,time:ok,certificate:') && p.checks[4] === 'chain:fail' && p.checks.length === 5 &&
      p.checkText[2].includes('Máy chủ cấp dấu thời gian'), JSON.stringify(p.checks.concat(p.checkText)));

  // 3) Chi tiết chứng thư.
  await run(`[...document.querySelectorAll('.kiem-tra button')].find((b) => b.textContent.includes('Chi tiết chứng thư'))?.click(); return 1;`);
  const nc = await waitFor(`const n = document.querySelector('.kiem-tra .kt-nc'); return n && { text: n.textContent.replace(/\\s+/g, ' '), chain: n.querySelectorAll('.kt-chuoi').length }`, 5000);
  check('Chi tiết chứng thư: nhà cung cấp CA, serial, hiệu lực, dấu thời gian, thu hồi, ký dài hạn, chuỗi chứng thư',
    !!nc && /Nhà cung cấp CA/.test(nc.text) && /Số serial/.test(nc.text) && /Hiệu lực\s*Từ/.test(nc.text) && /Máy chủ cấp dấu thời gian 02/.test(nc.text) &&
      /Thu hồi/.test(nc.text) && /Ký dài hạn/.test(nc.text) && nc.chain >= 1, JSON.stringify(nc));

  // 4) Tab Văn bản: trang vẽ ra, khung đánh dấu chữ ký đúng chỗ (tỉ lệ trang), nhãn "Toàn vẹn".
  await run(`[...document.querySelectorAll('.kiem-tra .tab')].find((b) => b.textContent.includes('Văn bản'))?.click(); return 1;`);
  const vb = await waitFor(`const g = document.querySelector('.kiem-tra .giay'); const m = document.querySelector('.kiem-tra .giay .ky-dau');
    if (!g || !m || !g.querySelector('canvas')) return null;
    const gr = g.getBoundingClientRect(), mr = m.getBoundingClientRect();
    return { cls: m.className, x: (mr.left - gr.left) / gr.width, y: (mr.top - gr.top) / gr.height, w: mr.width / gr.width, label: m.textContent,
      nhan: document.querySelector('.kiem-tra .nhan-trang')?.textContent.trim(), page: document.querySelector('.kiem-tra .dieuhuong .tt')?.textContent.replace(/\\s+/g, ' ').trim() }`, 20000);
  await sleep(500);
  await shot('3-van-ban.png');
  const near = (a, b) => Math.abs(a - b) < 0.02;
  check('Tab Văn bản: khung đánh dấu trùng widget chữ ký trong file (PyMuPDF), đỏ, đang chọn (quầng cam), số 1; nhãn "Toàn vẹn"; "Trang 1 / 1"',
    !!vb && near(vb.x, widget.x) && near(vb.y, widget.y) && near(vb.w, widget.w) && vb.cls.includes('fail') && vb.cls.includes('noi') && vb.label === '1' &&
      vb.nhan === 'Toàn vẹn' && vb.page === 'Trang 1 / 1', JSON.stringify({ vb, widget }));
  await run(`[...document.querySelectorAll('.kiem-tra .tab')].find((b) => b.textContent.includes('Chữ ký'))?.click(); return 1;`);

  // 5) Sửa nội dung rồi ký thêm: chữ ký 1 "Văn bản bị sửa sau khi ký"; chuyển chữ ký; chân màn.
  await drop(path.join(docs, 'sua-roi-ky-them.pdf'));
  await loaded('sua-roi-ky-them.pdf');
  await sleep(400);
  r = await rows();
  p = await panel();
  await shot('4-bi-sua.png');
  check('Sửa sau khi ký rồi ký thêm: chữ ký 1 "Văn bản bị sửa sau khi ký" (lý do nêu "Trang 1 bị sửa"), điều kiện toàn vẹn không đạt; chữ ký 2 không bị sửa',
    r.length === 2 && r[0].includes('Văn bản bị sửa sau khi ký') && !r[1].includes('bị sửa') && p.kqText.includes('Trang 1 bị sửa') &&
      p.checks[0] === 'integrity:fail' && p.vt === 'Chữ ký 1 / 2', JSON.stringify({ r, p }));
  check('Chân màn: "Văn bản có chữ ký sai hoặc bị sửa sau khi ký · 0/2 hợp lệ"', p.foot.includes('Văn bản có chữ ký sai hoặc bị sửa sau khi ký') && p.foot.includes('0/2'), p.foot);
  await run(`document.querySelector('.kiem-tra .dieuhuong button[title^="Chữ ký sau"]')?.click(); return 1;`);
  const p2 = await waitFor(`const t = document.querySelector('.kiem-tra .kt-vt')?.textContent.replace(/\\s+/g, ' ').trim(); return t === 'Chữ ký 2 / 2' &&
    [...document.querySelectorAll('.kiem-tra .kt-dk-dong')].map((d) => d.dataset.id + ':' + d.dataset.state).join()`, 5000);
  check('Nút "Chữ ký sau": sang chữ ký 2 / 2, toàn vẹn đạt', !!p2 && p2.startsWith('integrity:ok'), String(p2));

  // 6) Dấu thời gian tài liệu của TSA Ban Cơ yếu: mục thứ hai "Chữ ký hợp lệ" (xanh).
  await drop(path.join(docs, 'dau-thoi-gian-tai-lieu.pdf'));
  await loaded('dau-thoi-gian-tai-lieu.pdf');
  await sleep(300);
  r = await rows();
  await run(`[...document.querySelectorAll('.kiem-tra .kt-dong')][1]?.click(); return 1;`);
  p = await waitFor(`const k = document.querySelector('.kiem-tra .kt-kq'); return k?.className.includes('ok') && { kq: k.textContent.replace(/\\s+/g, ' ').trim(),
    the: document.querySelector('.kiem-tra .kt-the')?.textContent.replace(/\\s+/g, ' ').trim(), head: document.querySelector('.kiem-tra .kt-tt')?.textContent.trim() }`, 5000);
  await shot('5-dau-thoi-gian.png');
  check('Dấu thời gian tài liệu: mục "Dấu thời gian tài liệu" kết luận "Chữ ký hợp lệ" (xanh), máy chủ dấu thời gian của Ban Cơ yếu; tiêu đề "1/2 hợp lệ"',
    r.length === 2 && r[1].includes('Dấu thời gian tài liệu') && !!p && p.kq.startsWith('Chữ ký hợp lệ') && p.the.includes('Máy chủ cấp dấu thời gian') && p.head.includes('1/2'),
    JSON.stringify({ r, p }));

  // 7) Tắt kiểm thu hồi: kiểm lại ngay, chân màn ghi "chưa kiểm tra thu hồi".
  await drop(visible);
  await loaded('ky hien thi.pdf');
  await run(`document.querySelector('.kiem-tra .kt-thuhoi input')?.click(); return 1;`);
  await sleep(500);
  await loaded('ky hien thi.pdf');
  p = await panel();
  check('Tắt "Kiểm tra thu hồi": kiểm lại, chân màn "chưa kiểm tra thu hồi", điều kiện chứng thư ghi chưa kiểm thu hồi',
    p.foot.includes('chưa kiểm tra thu hồi') && p.checkText[3]?.includes('chưa kiểm tra thu hồi'), JSON.stringify(p));

  // 8) Xuất báo cáo ra PDF (lệnh report_export_pdf; hộp Lưu của Windows không tự động được): PDF chỉ có báo cáo.
  const pdfOut = path.join(work, 'bao cao.pdf');
  const ex = await run(`await window.__TAURI_INTERNALS__.invoke('report_export_pdf', { path: ${JSON.stringify(pdfOut)} }); return 'ok';`);
  const text = spawnSync('python', ['-I', '-c', `
import sys, json, fitz
d = fitz.open(sys.argv[1])
print(json.dumps({"pages": d.page_count, "text": "".join(p.get_text() for p in d)}))`, pdfOut], { encoding: 'utf8' });
  const bc = text.status === 0 ? JSON.parse(text.stdout) : { err: text.stderr };
  check('Xuất báo cáo PDF: có file, tiêu đề "Báo cáo kiểm tra chữ ký số", tên văn bản, kết luận, 5 điều kiện; không có menu / màn hình',
    ex === 'ok' && bc.pages >= 1 && bc.text.includes('Báo cáo kiểm tra chữ ký số') && bc.text.includes('ky hien thi.pdf') &&
      bc.text.includes('Người ký không thuộc CA Ban Cơ yếu') && bc.text.includes('Văn bản toàn vẹn') && bc.text.includes('Thời gian ký') &&
      !bc.text.includes('Văn bản cần kiểm tra:') && !bc.text.includes('Kiểm tra file khác'),
    JSON.stringify({ ex, pages: bc.pages, head: bc.text?.slice(0, 200), err: bc.err }));
  const badPath = await run(`try { await window.__TAURI_INTERNALS__.invoke('report_export_pdf', { path: 'C:\\\\khong-co-thu-muc-nay\\\\x.pdf' }); return 'ok'; } catch (e) { return String(e?.message ?? e); }`);
  check('Xuất vào thư mục không có thật: báo lỗi, không in', badPath !== 'ok' && /thư mục có thật/.test(badPath), badPath);

  // 9) Văn bản chưa ký; thả file không phải PDF.
  await drop(path.join(docs, 'chua ky.pdf'));
  await loaded('chua ky.pdf');
  p = await panel();
  const empty = await run(`return document.querySelector('.kiem-tra .phai .bao')?.textContent.replace(/\\s+/g, ' ').trim()`);
  check('Văn bản chưa ký: "Văn bản chưa có chữ ký số" ở khung phải và chân màn', /Văn bản chưa có chữ ký số/.test(empty ?? '') && p.foot.includes('Văn bản chưa có chữ ký số'), JSON.stringify({ empty, p }));
  await drop(path.join(docs, 'ghi chu.txt'));
  const err = await waitFor(`return document.querySelector('.kiem-tra .kt-loi')?.textContent.trim()`, 5000);
  check('Thả file không phải PDF: báo "Chỉ kiểm tra được file PDF"', /Chỉ kiểm tra được file PDF/.test(err ?? ''), String(err));

  // 10) Kiểm tra file khác / xoá đường dẫn: về giới thiệu.
  await run(`document.querySelector('.kiem-tra .hang .o .xoa')?.click(); return 1;`);
  const back = await waitFor(`return !!document.querySelector('.kiem-tra .mo-dau') && document.querySelector('.kiem-tra .hang input.fld').value === '' && 1`, 5000);
  check('Xoá đường dẫn: về phần giới thiệu', !!back);
  check('Không có lỗi trong console', consoleErrors.length === 0, consoleErrors.join(' | '));
  socket.close();
} finally {
  app.kill();
  await sleep(1000);
}

const passed = results.filter(Boolean).length;
console.log(`\nKết quả: ${passed}/${results.length} đạt`);
process.exit(passed === results.length ? 0 : 1);
