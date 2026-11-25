// Kiểm hộp "File đã ký / trùng tên" (KS-16) và "Xóa file gốc" (ADR 0003) trên app thật, bản build
// `--features dev-keys` (khoá thử ở SMKS_DEV_PKI). Cấu hình ghi vào thư mục riêng (SMKS_CONFIG_DIR).
// "Xóa file gốc" đưa file gốc (bản chép của bài kiểm trong thư mục làm việc) vào Thùng rác thật của Windows;
// bài kiểm tìm đúng hai file đó trong Thùng rác (vị trí gốc = thư mục làm việc) rồi dọn đúng hai file đó.
//
// Cách dùng: node app/tests/ks-trung.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc (ổ cố định)> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9357;
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
for (const d of [cfgDir, src, out]) fs.mkdirSync(d, { recursive: true });
const names = ['van ban 1.pdf', 'van ban 2.pdf'];
const fixtures = ['word-hybrid-xref.pdf', 'table-xref.pdf'].map((f) => path.join(repo, 'fixtures/pdf', f));
names.forEach((n, i) => fs.copyFileSync(fixtures[i], path.join(src, n)));
const marker = Buffer.from('%PDF-1.4\n% ban da ky cu cua bai kiem\n');
fs.writeFileSync(path.join(out, 'van ban 2_signed.pdf'), marker);
const outList = () => fs.readdirSync(out).filter((n) => n.endsWith('.pdf')).sort();

/** Các mục trong Thùng rác có vị trí gốc là thư mục `dir`. */
function recycled(dir, remove = false) {
  const script = `
$ErrorActionPreference = 'Stop'
$rb = (New-Object -ComObject Shell.Application).Namespace(10)
$r = @()
foreach ($i in $rb.Items()) {
  if ($rb.GetDetailsOf($i, 1) -eq '${dir.replace(/'/g, "''")}') {
    $r += [pscustomobject]@{ name = $i.Name; size = $i.Size; path = $i.Path }
    if (${remove ? '$true' : '$false'}) {
      Remove-Item -LiteralPath $i.Path -Force
      $info = Join-Path (Split-Path $i.Path) ((Split-Path $i.Path -Leaf) -replace '^\\$R', '$I')
      if (Test-Path -LiteralPath $info) { Remove-Item -LiteralPath $info -Force }
    }
  }
}
ConvertTo-Json -Compress -InputObject @($r)`;
  const r = spawnSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
  try {
    return JSON.parse(r.stdout || '[]');
  } catch {
    return { error: (r.stderr || r.stdout).slice(0, 300) };
  }
}

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
  const popup = (title) => `document.querySelector('.popup[aria-label=${JSON.stringify(title).replace(/'/g, "\\'")}]')`;
  const dupDialog = () =>
    waitFor(`const p = ${popup('File đã ký / trùng tên')}; return p && { rows: [...p.querySelectorAll('.trung-dong .ten b')].map((b) => b.textContent),
        chinh: [...p.querySelectorAll('button.nut.chinh')].map((b) => b.textContent.trim()) }`, 8000);
  const startSign = () => run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((x) => !x.disabled)?.click(); return 1;`);
  const signWithDevKey = async () => {
    await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
    await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
       r?.querySelector('input')?.click(); return !!r;`);
    const note = await run(`return document.querySelector('app-popup .hint.mo-ta')?.textContent`);
    await clickText('app-popup button.nut.chinh', 'Ký số');
    const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((x) => x.textContent).join('/')`, 120000);
    await clickText('app-popup button', 'Đóng');
    await sleep(400);
    return { done, note };
  };

  // Nạp 2 văn bản; DaKySo đã có "van ban 2_signed.pdf" (bản cũ).
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks, .nguon-gon')`);
  await sleep(1500);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(src)}], position: { x: 500, y: 400 } } }); return 1;`);
  check('Thả thư mục: 2 văn bản', (await waitFor(`return document.querySelectorAll('.ds .dong').length === 2 && 2`)) === 2);

  // 1. Ký số: hộp trùng tên liệt kê đúng file đã có; Hủy thì không ký.
  await startSign();
  let dlg = await dupDialog();
  await shot('1-trung-ten.png');
  check('Bấm Ký số: hộp "File đã ký / trùng tên" liệt kê van ban 2_signed.pdf; nút chính "Giữ cả hai" (cấu hình không ghi đè)',
    JSON.stringify(dlg?.rows) === '["van ban 2_signed.pdf"]' && /Giữ cả hai/.test(dlg?.chinh?.join() ?? ''), JSON.stringify(dlg));
  await clickText(`.popup[aria-label="File đã ký / trùng tên"] button`, 'Hủy');
  await sleep(500);
  const none = await run(`return !document.querySelector('app-popup') && document.querySelectorAll('.chip-tt.ok').length === 0`);
  check('Hủy: không mở hộp Xác nhận ký, không ký', !!none);

  // 2. Giữ cả hai: bản cũ giữ nguyên, bản mới thêm (1).
  await startSign();
  await dupDialog();
  await clickText(`.popup[aria-label="File đã ký / trùng tên"] button`, 'Giữ cả hai');
  let r = await signWithDevKey();
  check('Giữ cả hai: ký 2/2, bản cũ giữ nguyên, bản mới "van ban 2_signed (1).pdf"',
    r.done === '2/2/0' && JSON.stringify(outList()) === JSON.stringify(['van ban 1_signed.pdf', 'van ban 2_signed (1).pdf', 'van ban 2_signed.pdf']) &&
      fs.readFileSync(path.join(out, 'van ban 2_signed.pdf')).equals(marker),
    `${r.done}; ${outList().join(', ')}`);

  // 3. Ký lại → hộp liệt kê cả 2 file; Ghi đè thay bản đã ký cũ, không sinh thêm (2).
  await startSign();
  dlg = await dupDialog();
  check('Ký lại: hộp trùng tên liệt kê cả 2 file', JSON.stringify(dlg?.rows) === '["van ban 1_signed.pdf","van ban 2_signed.pdf"]', JSON.stringify(dlg?.rows));
  await clickText(`.popup[aria-label="File đã ký / trùng tên"] button`, 'Ghi đè');
  r = await signWithDevKey();
  const v2 = fs.readFileSync(path.join(out, 'van ban 2_signed.pdf'));
  check('Ghi đè: ký 2/2, "van ban 2_signed.pdf" thay bằng bản ký mới, không thêm file',
    r.done === '2/2/0' && outList().length === 3 && !v2.equals(marker) && v2.includes('/ByteRange'), `${r.done}; ${outList().join(', ')}`);
  const originalsKept = names.every((n, i) => fs.readFileSync(path.join(src, n)).equals(fs.readFileSync(fixtures[i])));
  check('Chưa bật Xóa file gốc: file gốc giữ nguyên từng byte', originalsKept);

  // 4. Xóa file gốc: bật phải xác nhận; Hủy thì vẫn tắt.
  await run(`document.querySelector('.ng-c34 input[type=checkbox]').click(); return 1;`);
  const askDlg = await waitFor(`return ${popup('Xóa file gốc')}?.textContent`, 5000);
  await shot('2-xoa-file-goc.png');
  const stillOff = await run(`return document.querySelector('.ng-c34 input[type=checkbox]').checked`);
  check('Tích "Xóa file gốc": hỏi xác nhận (Thùng rác, chỉ sau khi kiểm lại hợp lệ), ô chưa tích', /Thùng rác/.test(askDlg ?? '') && /kiểm lại hợp lệ/.test(askDlg ?? '') && stillOff === false, String(stillOff));
  await clickText(`.popup[aria-label="Xóa file gốc"] button`, 'Hủy');
  await sleep(300);
  check('Hủy xác nhận: "Xóa file gốc" vẫn tắt', (await run(`return document.querySelector('.ng-c34 input[type=checkbox]').checked`)) === false);
  await run(`document.querySelector('.ng-c34 input[type=checkbox]').click(); return 1;`);
  await waitFor(`return !!${popup('Xóa file gốc')}`, 5000);
  await clickText(`.popup[aria-label="Xóa file gốc"] button`, 'Bật Xóa file gốc');
  await sleep(300);
  check('Xác nhận: "Xóa file gốc" bật', (await run(`return document.querySelector('.ng-c34 input[type=checkbox]').checked`)) === true);

  // 5. Ký với Xóa file gốc: không hỏi trùng tên, file đã ký thay chỗ file gốc, file gốc vào Thùng rác.
  const sizes = fixtures.map((f) => fs.statSync(f).size);
  await startSign();
  await sleep(800);
  const noDup = await run(`return !${popup('File đã ký / trùng tên')}`);
  r = await signWithDevKey();
  check('Xóa file gốc: không hỏi trùng tên; hộp Xác nhận nói thay vào chỗ file gốc, không nói "giữ nguyên"; ký 2/2', !!noDup && /thay vào chỗ file gốc/.test(r.note ?? '') && !/giữ nguyên/.test(r.note ?? '') && r.done === '2/2/0', `${r.done}; ${r.note}`);
  const replaced = names.map((n) => fs.readFileSync(path.join(src, n)));
  check('Hai file trong thư mục gốc nay là bản đã ký (có /ByteRange), DaKySo không thêm file',
    replaced.every((b, i) => !b.equals(fs.readFileSync(fixtures[i])) && b.includes('/ByteRange')) && outList().length === 3 &&
      fs.readdirSync(src).filter((n) => n.endsWith('.tmp')).length === 0, outList().join(', '));
  const py = spawnSync('python', ['-I', '-c', `
import sys, json
from pyhanko.pdf_utils.reader import PdfFileReader
r = []
for f in sys.argv[1:]:
    with open(f, "rb") as h:
        r.append(len(PdfFileReader(h, strict=False).embedded_signatures))
print(json.dumps(r))`, ...names.map((n) => path.join(src, n))], { encoding: 'utf8' });
  check('pyHanko đọc được chữ ký trong hai file đã thay', py.stdout.trim() === '[1, 1]', py.stdout.trim() + (py.stderr ? ' ' + py.stderr.slice(0, 200) : ''));
  const bin = recycled(src);
  const binOk = Array.isArray(bin) && bin.length === 2 && names.every((n, i) => bin.some((b) => n.startsWith(b.name.replace(/\.pdf$/, '')) && b.size === sizes[i]));
  check('File gốc nằm trong Thùng rác (vị trí gốc = thư mục văn bản, đúng kích thước bản gốc)', binOk, JSON.stringify(bin));
  const log = await run(`document.querySelectorAll('.tabs .tab')[1].click(); await new Promise((r) => setTimeout(r, 300));
      return [...document.querySelectorAll('.log .ln')].map((l) => l.textContent).join('\\n')`);
  check('Nhật ký ghi "thay file gốc"', /xóa file gốc \(vào Thùng rác\)/.test(log ?? '') && (log?.match(/thay file gốc \(bản gốc vào Thùng rác\)/g) ?? []).length === 2);
  socket.close();
} catch (error) {
  console.error(`LOI: ${error.stack ?? error.message}`);
  results.push(false);
} finally {
  app.kill();
  // Dọn đúng các file của bài kiểm khỏi Thùng rác.
  const left = recycled(src, true);
  if (Array.isArray(left) && left.length) {
    console.log(`Đã dọn ${left.length} file của bài kiểm khỏi Thùng rác.`);
  }
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
