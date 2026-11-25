// Kiểm màn Ký số với USB token THẬT bằng thao tác giao diện (WebView2 qua CDP). Cần người ngồi máy:
//   Lượt A: ký 3 file — người dùng nhập PIN ở hộp của middleware; test ghi lại cửa sổ PIN có gắn (owner) vào
//           cửa sổ chính của app không.
//   Lượt B: ký nhiều file — người dùng RÚT TOKEN khi lô đang chạy; app phải dừng, file đã ký vẫn hợp lệ.
//   Lượt C: cắm lại token, bấm ký tiếp — các file còn lại được ký, không trùng tên.
// Test không nhận, không gõ PIN. Không ghi tên/thumbprint chứng thư ra màn hình.
//
// Cách dùng: node app/tests/ks-token-ui.mjs <SaoMai-KySo.exe> <smks-cli.exe> <thư mục repo> <thư mục làm việc> [số file lượt B]
import { spawn, spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const [exe, cli, repo, work] = process.argv.slice(2, 6).map((p) => path.resolve(p));
const countB = Number(process.argv[6] ?? 200);
const debugPort = 9351;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
const log = (text) => console.log(`${new Date().toLocaleTimeString('vi-VN')} ${text}`);
const sha = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');

// Thư mục làm việc.
fs.rmSync(work, { recursive: true, force: true });
const dirA = path.join(work, 'A');
const dirB = path.join(work, 'B');
const shots = path.join(work, 'anh');
for (const d of [dirA, dirB, shots]) fs.mkdirSync(d, { recursive: true });
const fixtures = ['word-hybrid-xref.pdf', 'xref-stream.pdf', 'table-xref.pdf'].map((f) => path.join(repo, 'fixtures/pdf', f));
fixtures.forEach((f, i) => fs.copyFileSync(f, path.join(dirA, `a ${i + 1}.pdf`)));
for (let i = 0; i < countB; i++) fs.copyFileSync(fixtures[i % 3], path.join(dirB, `b ${String(i + 1).padStart(3, '0')}.pdf`));
const hashB = Object.fromEntries(fs.readdirSync(dirB).map((n) => [n, sha(path.join(dirB, n))]));

// Liệt kê cửa sổ cấp cao đang hiện: pid, owner, tiêu đề (để xem hộp PIN gắn vào cửa sổ nào).
const winScript = path.join(work, 'windows.ps1');
fs.writeFileSync(winScript, `
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class W {
  public delegate bool P(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(P p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint c);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  public static List<string> All() {
    var r = new List<string>();
    EnumWindows((h, l) => {
      if (!IsWindowVisible(h)) return true;
      uint pid; GetWindowThreadProcessId(h, out pid);
      var t = new StringBuilder(256); GetWindowText(h, t, 256);
      var c = new StringBuilder(256); GetClassName(h, c, 256);
      r.Add(h.ToInt64() + "\\t" + pid + "\\t" + GetWindow(h, 4).ToInt64() + "\\t" + c + "\\t" + t);
      return true;
    }, IntPtr.Zero);
    return r;
  }
}
"@
[W]::All() | ForEach-Object { $_ }
`);
const windows = () => {
  const out = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', winScript], { encoding: 'utf8' }).stdout ?? '';
  return out.split(/\r?\n/).filter(Boolean).map((l) => {
    const [hwnd, pid, owner, cls, title] = l.split('\t');
    return { hwnd: Number(hwnd), pid: Number(pid), owner: Number(owner), cls, title: title ?? '' };
  });
};

const verifyAll = (dir) => {
  const names = fs.existsSync(dir) ? fs.readdirSync(dir) : [];
  const bad = [];
  for (const n of names) {
    const r = spawnSync(cli, ['verify', path.join(dir, n)], { encoding: 'utf8' });
    if (!/: Hợp lệ/.test(r.stdout ?? '') || !/gốc Root G2/.test(r.stdout ?? '')) bad.push(`${n}: ${(r.stdout ?? '').split('\n')[0]}`);
  }
  return { names, bad };
};

// App chỉ cho chạy một bản: còn bản khác đang mở thì lần chạy này chỉ đưa cửa sổ cũ lên, test sẽ lái nhầm.
if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}
// Cổng debug đang có tiến trình khác nghe (vd. Edge headless còn sót) thì test sẽ lái nhầm trang đó.
try {
  await fetch(`http://127.0.0.1:${debugPort}/json`);
  console.error(`LOI: cổng ${debugPort} đang bận; đóng tiến trình đang nghe cổng này trước khi kiểm.`);
  process.exit(1);
} catch {}
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
  const run = async (body) => {
    const reply = await send('Runtime.evaluate', {
      expression: `(async () => { try { return { ok: await (async () => { ${body} })() }; } catch (e) { return { err: String(e) }; } })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    return reply.result?.result?.value?.ok;
  };
  const waitFor = async (body, ms = 20000, every = 250) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      const v = await run(body);
      if (v) return v;
      await sleep(every);
    }
    return null;
  };
  const shot = async (name) => {
    const image = await send('Page.captureScreenshot', { format: 'png' });
    if (image.result?.data) fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
  };
  const clickText = (selector, text) =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find((e) => e.textContent.includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  const drop = (paths) =>
    run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {
        target: { kind: 'Webview', label: 'main' }, event: 'tauri://drag-drop',
        payload: { paths: ${JSON.stringify(paths)}, position: { x: 500, y: 400 } } }); return 'ok';`);
  const summary = () =>
    run(`const p = document.querySelector('app-popup'); if (!p || !p.querySelector('.so-lon')) return null;
         const n = [...p.querySelectorAll('.so-lon b')].map((b) => Number(b.textContent));
         return { title: p.querySelector('.tieu-p')?.textContent.trim(), total: n[0], success: n[1], failed: n[2], paused: n[3] ?? 0,
                  hint: p.querySelector('.hint')?.textContent.trim(),
                  errors: [...p.querySelectorAll('.trung-dong.loi .ten span')].map((s) => s.textContent.trim()) };`);
  const tokenThumb = async () => {
    const scan = await run(`return await window.__TAURI_INTERNALS__.invoke('cert_list')`);
    return (scan?.certificates ?? []).find((c) => c.source === 'usbToken' && c.canSign)?.thumbprint ?? null;
  };

  // Mở màn Ký số.
  await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
  const title = await waitFor(`return /— Ký số/.test(document.title) && !!document.querySelector('.nguon.ks') && document.title`);
  check('Mở màn Ký số từ Trang chủ', !!title, String(title));
  // Màn vừa mở còn đang gắn bộ nghe kéo thả (bất đồng bộ); thả ngay thì sự kiện bị bỏ qua.
  await sleep(1500);

  // Mở hộp Xác nhận ký, chọn chứng thư USB Token, bấm Ký số. Trả về cửa sổ lạ thấy trong lúc mở token.
  const confirmWithToken = async (label) => {
    await clickText('.chan button.nut.chinh', 'Ký');
    const listed = await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
    const usb = await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.querySelector('.ct-the') && !l.classList.contains('tat'));
        if (!r) return null; const i = r.querySelector('input'); if (!i.checked) i.click(); return r.querySelector('.ct-phu')?.textContent.trim();`);
    check(`${label}: hộp Xác nhận liệt kê chứng thư, có chứng thư "USB Token" ký được`, !!listed && !!usb, `${listed} chứng thư; ${usb}`);
    await shot(`${label}-xac-nhan.png`);
    await clickText('app-popup button.nut.chinh', 'Ký số');
    const seen = new Map();
    const end = Date.now() + 180000;
    // Trong lúc "Đang mở token…" (middleware có thể hỏi PIN), ghi lại các cửa sổ thuộc tiến trình app.
    while (Date.now() < end) {
      const busy = await run(`return !!document.querySelector('app-popup .ct-bao') && document.querySelector('app-popup .ct-bao').textContent.includes('Đang mở token')`);
      if (!busy) break;
      for (const w of windows()) if (w.pid === app.pid) seen.set(w.hwnd, w);
      await sleep(1000);
    }
    const tokenError = await run(`return document.querySelector('app-popup .ct-bao.loi, app-popup .bao.loi')?.textContent.trim() ?? null`);
    return { seen: [...seen.values()], tokenError };
  };

  // ===== Lượt A: 3 file, nhập PIN =====
  await drop([dirA]);
  const rowsA = await waitFor(`return document.querySelectorAll('.ds .dong').length === 3 && 3`, 20000);
  check('Lượt A: thả thư mục, danh sách 3 văn bản', rowsA === 3, String(rowsA));
  log('>>> LƯỢT A: hộp PIN của Bit4id sắp hiện — NHẬP PIN.');
  const a = await confirmWithToken('A');
  const main = a.seen.find((w) => /Sao Mai/.test(w.title) && w.owner === 0);
  // Hộp thoại Win32 (#32770) của middleware; bỏ qua cửa sổ ẩn của Tauri (single-instance, event target).
  const pinWins = a.seen.filter((w) => w.cls === '#32770');
  check('Lượt A: mở được token (không lỗi)', !a.tokenError, a.tokenError ?? '');
  check('Lượt A: hộp của middleware thuộc tiến trình app và có owner là cửa sổ chính', !!main && pinWins.length > 0 && pinWins.every((w) => w.owner === main.hwnd),
    JSON.stringify(a.seen.map((w) => ({ cls: w.cls, title: w.title, laChinh: w === main, ownerLaChinh: !!main && w.owner === main.hwnd }))));
  const sumA = await waitFor(`return document.querySelector('app-popup .so-lon') && 1`, 120000).then(summary);
  await shot('A-ket-qua.png');
  check('Lượt A: 3 thành công, 0 lỗi', sumA?.total === 3 && sumA?.success === 3 && sumA?.failed === 0, JSON.stringify(sumA));
  const vA = verifyAll(path.join(`${dirA}_DaKySo`));
  check('Lượt A: 3 file trong DaKySo, smks-cli verify "Hợp lệ", gốc Root G2', vA.names.length === 3 && vA.bad.length === 0, vA.bad.join(' | ') || vA.names.join(', '));
  await clickText('app-popup button', 'Đóng');

  // ===== Lượt B: nhiều file, rút token giữa lô =====
  await drop([dirB]);
  const rowsB = await waitFor(`return document.querySelectorAll('.ds .dong').length === ${countB} && ${countB}`, 30000);
  check(`Lượt B: danh sách ${countB} văn bản`, rowsB === countB, String(rowsB));
  const b = await confirmWithToken('B');
  check('Lượt B: dùng lại phiên token, không hỏi PIN lần nữa', !b.tokenError && b.seen.filter((w) => w.cls === '#32770').length === 0, JSON.stringify(b.seen.map((w) => w.cls)));
  const startedB = await waitFor(`return document.querySelector('.chan .so-ok b') && Number(document.querySelector('.chan .so-ok b').textContent) > 0`, 60000);
  log(`>>> LƯỢT B đang ký (${countB} file) — RÚT USB TOKEN RA NGAY.`);
  await sleep(1500);
  await shot('B-dang-ky.png');
  const sumB = await waitFor(`return document.querySelector('app-popup .so-lon') && 1`, 600000, 500).then(summary);
  await shot('B-ket-qua.png');
  const removed = /rút USB token/i.test(sumB?.hint ?? '');
  check('Lượt B: đã bắt đầu ký trước khi rút', !!startedB);
  check('Lượt B: app báo đã rút token, tạm dừng giữa lô (thành công < tổng, tổng = thành công + lỗi + tạm dừng)', removed && sumB.success < countB && sumB.success + sumB.failed + sumB.paused === countB, JSON.stringify({ ...sumB, errors: [...new Set(sumB?.errors ?? [])] }));
  const outB = path.join(`${dirB}_DaKySo`);
  const vB = verifyAll(outB);
  check('Lượt B: số file ra = số thành công, không còn .tmp, mọi file ra đều "Hợp lệ"', vB.names.length === sumB?.success && !vB.names.some((n) => /\.tmp$/i.test(n)) && vB.bad.length === 0,
    `${vB.names.length} file; ${vB.bad.slice(0, 3).join(' | ')}`);
  check('Lượt B: file gốc giữ nguyên', Object.entries(hashB).every(([n, h]) => sha(path.join(dirB, n)) === h));
  await clickText('app-popup button', 'Đóng');

  // ===== Lượt C: cắm lại, ký tiếp =====
  log('>>> LƯỢT C: CẮM LẠI USB TOKEN (chờ tối đa 5 phút).');
  let back = null;
  for (let i = 0; i < 150 && !back; i++) {
    back = await tokenThumb();
    if (!back) await sleep(2000);
  }
  check('Lượt C: cắm lại thì chứng thư token hiện lại trong kho', !!back);
  if (back && removed) {
    await sleep(2000);
    const label = await run(`return document.querySelector('.chan button.nut.chinh')?.textContent.trim()`);
    log(`>>> LƯỢT C: nút chính "${label}"; middleware có thể hỏi PIN lại — NHẬP PIN.`);
    const c = await confirmWithToken('C');
    check('Lượt C: mở lại được token', !c.tokenError, c.tokenError ?? '');
    const sumC = await waitFor(`return document.querySelector('app-popup .so-lon') && 1`, 600000, 500).then(summary);
    await shot('C-ket-qua.png');
    check('Lượt C: ký tiếp đúng số file còn lại, 0 lỗi', sumC?.total === countB - (sumB?.success ?? 0) && sumC?.failed === 0, `nút "${label}"; ${JSON.stringify(sumC)}`);
    const vC = verifyAll(outB);
    check(`Lượt C: DaKySo đủ ${countB} file, không trùng " (1)", mọi file "Hợp lệ"`, vC.names.length === countB && !vC.names.some((n) => n.includes(' (1)')) && vC.bad.length === 0,
      `${vC.names.length} file; ${vC.bad.slice(0, 3).join(' | ')}`);
  }
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
