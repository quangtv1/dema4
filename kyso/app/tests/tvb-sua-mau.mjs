// Kiểm tra trên file chạy thật ba việc của màn Tạo văn bản:
//   A. Bôi đen và sao chép chữ ngay trên bản xem trước (lớp chữ của pdf.js nằm trên canvas).
//   B. Nút "Tải lại": đọc lại mẫu và dữ liệu đã sửa trên đĩa, giữ dòng đang xem và cách ghép biến.
//   C. Nút "Sửa mẫu": mở file mẫu bằng Microsoft Word thật; sửa và lưu trong Word (qua COM, tài liệu vẫn mở) rồi
//      Tải lại. Phần C cần Word; máy không có Word thì bỏ qua và ghi rõ.
//
// Cần Python có openpyxl (để sửa file .docx / .xlsx trên đĩa như một người dùng khác đã sửa).
// Bài kiểm ghi vào bộ nhớ tạm của Windows (phần sao chép) rồi trả lại chữ đang có ở đó.
//
// Cách dùng: node app/tests/tvb-sua-mau.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> [thư mục ảnh]
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, shots] = process.argv.slice(2).map((p) => (p ? path.resolve(p) : p));
const debugPort = 9341;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
const note = (text) => console.log(`[GHI CHU] ${text}`);

fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
const tpl = path.join(work, 'mau_khen_thuong.docx');
const data = path.join(work, 'du_lieu_khen_thuong.xlsx');
fs.copyFileSync(path.join(repo, 'fixtures/merge/2_quyet_dinh_khen_thuong.docx'), tpl);
fs.copyFileSync(path.join(repo, 'fixtures/merge/2_quyet_dinh_khen_thuong.xlsx'), data);
if (shots) fs.mkdirSync(shots, { recursive: true });

const ps = (script) => {
  const r = spawnSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8', timeout: 90000 });
  return { out: (r.stdout ?? '').trim(), err: (r.stderr ?? '').trim(), code: r.status };
};
const py = (code, ...args) => {
  const r = spawnSync('python', ['-X', 'utf8', '-c', code, ...args], { encoding: 'utf8', timeout: 60000 });
  if (r.status !== 0) throw new Error(`python: ${r.stderr}`);
  return (r.stdout ?? '').trim();
};
/**
 * Hàm PowerShell `Get-WordDoc <pid> <đường dẫn>`: lấy đối tượng Document của file đang mở trong cửa sổ Word của
 * đúng tiến trình `pid` (AccessibleObjectFromWindow trên khung `_WwG`, OBJID_NATIVEOM). Word mới mở có thể từ chối
 * lời gọi COM (RPC_E_CALL_REJECTED) vài giây nên thử lại.
 */
const WORD_DOC_BY_PID = `
Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices;
public static class WordWin {
  public delegate bool P(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(P p, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, P p, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("oleacc.dll")] static extern int AccessibleObjectFromWindow(IntPtr h, uint id, ref Guid iid, [MarshalAs(UnmanagedType.IDispatch)] out object o);
  static string Cls(IntPtr h) { var s = new StringBuilder(64); GetClassName(h, s, 64); return s.ToString(); }
  public static object Window(uint pid) {
    IntPtr top = IntPtr.Zero, edit = IntPtr.Zero;
    EnumWindows((h, l) => { uint id; GetWindowThreadProcessId(h, out id);
      if (id == pid && IsWindowVisible(h) && Cls(h) == "OpusApp") { top = h; return false; } return true; }, IntPtr.Zero);
    if (top == IntPtr.Zero) return null;
    EnumChildWindows(top, (h, l) => { if (Cls(h) == "_WwG") { edit = h; return false; } return true; }, IntPtr.Zero);
    if (edit == IntPtr.Zero) return null;
    var iid = new Guid("00020400-0000-0000-C000-000000000046"); object o;
    return AccessibleObjectFromWindow(edit, 0xFFFFFFF0, ref iid, out o) == 0 ? o : null;
  }
}
'@
function Get-WordDoc([uint32]$procId, [string]$path) {
  for ($i = 0; $i -lt 40; $i++) {
    try {
      $win = [WordWin]::Window($procId)
      if ($win) { foreach ($x in $win.Application.Documents) { if ($x.FullName -eq $path) { return $x } } }
    } catch {}
    Start-Sleep -Milliseconds 500
  }
  throw "khong lay duoc tai lieu $path trong Word PID $procId"
}
`;
/** PID các tiến trình Word đang có cửa sổ hiện ra (Word người dùng đang mở). */
const visibleWordPids = () =>
  ps(`(Get-Process WINWORD -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object Id) -join ','`).out.split(',').filter(Boolean);
const wordPids = () =>
  ps(`(Get-Process WINWORD -ErrorAction SilentlyContinue | ForEach-Object Id) -join ','`).out.split(',').filter(Boolean).sort();

/** Thêm một đoạn "<chữ>" vào cuối mẫu (sửa thẳng word/document.xml), như người dùng sửa và lưu ở nơi khác. */
const ADD_PARAGRAPH = `
import sys, zipfile, shutil, os
p, text = sys.argv[1], sys.argv[2]
tmp = p + '.moi'
with zipfile.ZipFile(p) as zin, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        raw = zin.read(item.filename)
        if item.filename == 'word/document.xml':
            xml = raw.decode('utf-8')
            para = '<w:p><w:r><w:t xml:space="preserve">' + text + '</w:t></w:r></w:p>'
            i = xml.rfind('<w:sectPr')
            assert i > 0
            raw = (xml[:i] + para + xml[i:]).encode('utf-8')
        zout.writestr(item, raw)
os.replace(tmp, p)
print('ok')
`;
/** Thêm một dòng dữ liệu (chép dòng cuối, đổi số quyết định và họ tên). */
const ADD_ROW = `
import sys, openpyxl
p = sys.argv[1]
wb = openpyxl.load_workbook(p)
ws = wb.active
last = [c.value for c in ws[ws.max_row]]
last[0] = '399/QĐ-ABC'
last[3] = 'Phạm Thị Thêm Mới'
ws.append(last)
wb.save(p)
print(ws.max_row - 1)
`;

const wordBefore = wordPids();
const app = spawn(exe, [], {
  env: {
    ...process.env,
    SMKS_CONFIG_DIR: path.join(work, 'cau-hinh'),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
  },
  stdio: 'ignore',
});
let socket;
let clipboardBefore = null;
let clipboardTouched = false;

try {
  let target;
  for (let i = 0; i < 80 && !target; i++) {
    await sleep(250);
    try {
      target = (await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json()).find((t) => t.type === 'page');
    } catch {}
  }
  if (!target) throw new Error('không kết nối được WebView2');
  socket = new WebSocket(target.webSocketDebuggerUrl);
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
    } else if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
      consoleErrors.push(message.params.args.map((a) => a.value ?? a.description).join(' '));
    } else if (message.method === 'Runtime.exceptionThrown') {
      consoleErrors.push(message.params.exceptionDetails?.exception?.description ?? 'exception');
    } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
      consoleErrors.push(message.params.entry.text);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const id = nextId++;
      pending.set(id, resolve);
      socket.send(JSON.stringify({ id, method, params }));
    });
  const run = async (body, userGesture = false) => {
    const reply = await send('Runtime.evaluate', {
      expression: `(async () => { ${body} })()`,
      returnByValue: true,
      awaitPromise: true,
      userGesture,
    });
    return reply.result?.result?.value;
  };
  const waitFor = async (body, timeoutMs = 20000) => {
    const end = Date.now() + timeoutMs;
    while (Date.now() < end) {
      const value = await run(body);
      if (value) return value;
      await sleep(250);
    }
    return null;
  };
  const shot = async (name) => {
    if (!shots) return;
    const image = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(path.join(shots, name), Buffer.from(image.result.data, 'base64'));
  };
  const click = (selector, text = '') =>
    run(`const el = [...document.querySelectorAll(${JSON.stringify(selector)})]
           .find((e) => e.textContent.replace(/\\s+/g, ' ').includes(${JSON.stringify(text)}) && !e.disabled);
         if (!el) return false; el.click(); return true;`);
  const mouse = (type, x, y, extra = {}) => send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1, ...extra });
  /** Chữ của lớp chữ trên bản xem trước (mọi trang), dạng dựng sẵn. */
  const LAYER_TEXT = `[...document.querySelectorAll('.vungxem .giay .textLayer')].map((l) => l.textContent).join('\\n').normalize('NFC')`;
  /** Chờ bản xem trước vẽ xong (không còn "Đang tạo…") và lớp chữ có `text`. */
  const waitText = (text, timeoutMs = 40000) =>
    waitFor(`return !document.querySelector('.xem-tt') && (${LAYER_TEXT}).includes(${JSON.stringify(text)}) && 1`, timeoutMs);
  const tabCount = () => run(`return [...document.querySelectorAll('.tabs .tab .dem')].map((e) => e.textContent.trim()).join(' ')`);
  const reload = async () => {
    const ok = await click('.dieuhuong .tai-lai');
    await sleep(150);
    await waitFor(`const b = document.querySelector('.dieuhuong .tai-lai'); return b && !b.disabled && 1`, 30000);
    return ok;
  };
  await send('Runtime.enable');
  await send('Log.enable');

  // ---- Mở màn, nạp mẫu và dữ liệu -----------------------------------------------------------------
  const home = await waitFor(`return document.querySelectorAll('.tc-the').length === 3 && 1`);
  if (!home) {
    await shot('sm-0-khong-thay-trang-chu.png');
    throw new Error('không thấy Trang chủ');
  }
  await sleep(800);
  await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true })); return true;`);
  const screen = await waitFor(`return document.title.includes('Tạo văn bản') && 1`);
  if (!screen) {
    await shot('sm-0-khong-mo-duoc-man.png');
    throw new Error('Ctrl+N không mở màn Tạo văn bản');
  }
  await sleep(800);
  await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {
      target: { kind: 'Webview', label: 'main' }, event: 'tauri://drag-drop',
      payload: { paths: ${JSON.stringify([tpl, data])}, position: { x: 400, y: 300 } } }); return true;`);
  const plan = await waitFor(`const b = document.querySelector('.chan button.nut.chinh');
     return b && /Tạo \\d+ văn bản/.test(b.textContent) && !b.disabled && b.textContent.trim();`, 30000);
  check('Nạp mẫu và dữ liệu: "Tạo 5 văn bản"', plan === 'Tạo 5 văn bản', String(plan));
  if (!plan) {
    await shot('sm-0-khong-nap-duoc.png');
    throw new Error('không nạp được mẫu và dữ liệu');
  }

  // ---- A. Lớp chữ, bôi đen, sao chép ---------------------------------------------------------------
  const hasText = await waitText('Nguyễn Văn An', 60000);
  check('A1. Bản xem trước có lớp chữ, đọc được nội dung văn bản của dòng 1', !!hasText);
  const geometry = await run(`const sheet = document.querySelector('.vungxem .giay'); const c = sheet.querySelector('canvas');
     const l = sheet.querySelector('.textLayer'); const cr = c.getBoundingClientRect(); const lr = l.getBoundingClientRect();
     const spans = [...l.querySelectorAll('span')].filter((s) => s.textContent.trim());
     const out = spans.filter((s) => { const r = s.getBoundingClientRect();
       return r.left < cr.left - 2 || r.right > cr.right + 2 || r.top < cr.top - 2 || r.bottom > cr.bottom + 2; }).length;
     return { spans: spans.length, out, dw: Math.abs(cr.width - lr.width), dh: Math.abs(cr.height - lr.height),
              dx: Math.abs(cr.left - lr.left), dy: Math.abs(cr.top - lr.top), color: getComputedStyle(spans[0]).color };`);
  check(
    'A2. Lớp chữ trùng khít trang (cùng vị trí, cùng cỡ với canvas), chữ trong suốt, không tràn ra ngoài',
    geometry && geometry.spans > 20 && geometry.out === 0 && geometry.dw <= 1 && geometry.dh <= 1 && geometry.dx <= 1 && geometry.dy <= 1 &&
      /rgba\(0, 0, 0, 0\)/.test(geometry.color),
    JSON.stringify(geometry),
  );

  // Chữ của lớp chữ nằm đúng chỗ chữ trên canvas: trong ô của một dòng chữ phải có điểm ảnh tối, và ngay
  // phía trên dòng đó (khoảng cách dòng) thì gần như không.
  const ink = await run(`const sheet = document.querySelector('.vungxem .giay'); const c = sheet.querySelector('canvas');
     const span = [...sheet.querySelectorAll('.textLayer span')].find((s) => s.textContent.normalize('NFC').includes('QUYẾT ĐỊNH'));
     if (!span) return null;
     const cr = c.getBoundingClientRect(); const r = span.getBoundingClientRect(); const k = c.width / cr.width;
     const dark = (x, y, w, h) => { const d = c.getContext('2d').getImageData(Math.round((x - cr.left) * k), Math.round((y - cr.top) * k), Math.max(1, Math.round(w * k)), Math.max(1, Math.round(h * k))).data;
       let n = 0; for (let i = 0; i < d.length; i += 4) if (d[i] + d[i + 1] + d[i + 2] < 300) n++; return n / (d.length / 4); };
     return { text: span.textContent, inside: dark(r.left, r.top, r.width, r.height), left: dark(r.left - r.width * 0.4, r.top, r.width * 0.3, r.height), right: dark(r.right + r.width * 0.1, r.top, r.width * 0.3, r.height), w: Math.round(r.width), h: Math.round(r.height) };`);
  check(
    'A3. Ô chữ "QUYẾT ĐỊNH" của lớp chữ phủ đúng chữ trên trang (trong ô có mực, hai bên ô trống)',
    ink && ink.inside > 0.08 && ink.left < 0.01 && ink.right < 0.01,
    JSON.stringify(ink),
  );

  // Kéo chuột thật (CDP) từ đầu tới cuối một dòng chữ.
  const line = await run(`const span = [...document.querySelectorAll('.vungxem .giay .textLayer span')].find((s) => s.textContent.normalize('NFC').includes('Về việc khen thưởng'));
     if (!span) return null; span.scrollIntoView({ block: 'center' }); const r = span.getBoundingClientRect();
     return { x1: r.left + 1, x2: r.right - 1, y: r.top + r.height / 2, text: span.textContent.normalize('NFC') };`);
  let selected = '';
  if (line) {
    await mouse('mousePressed', line.x1, line.y);
    for (let i = 1; i <= 8; i++) await mouse('mouseMoved', line.x1 + ((line.x2 - line.x1) * i) / 8, line.y);
    const selecting = await run(`return !!document.querySelector('.textLayer.selecting')`);
    await mouse('mouseReleased', line.x2, line.y);
    await sleep(150);
    selected = (await run(`return document.getSelection().toString().normalize('NFC')`)) ?? '';
    const cleared = await run(`return !document.querySelector('.textLayer.selecting')`);
    check('A4. Kéo chuột qua một dòng chữ thì bôi đen đúng dòng đó', selected.trim().length > 10 && line.text.includes(selected.trim()) && selecting && cleared, `"${selected.trim()}" / dòng: "${line.text.trim()}"`);
    await sleep(200);
    await shot('sm-1-boi-den.png');
  } else {
    check('A4. Kéo chuột qua một dòng chữ thì bôi đen đúng dòng đó', false, 'không thấy dòng "Về việc khen thưởng"');
  }

  // Sao chép thật vào bộ nhớ tạm của Windows rồi đọc lại bằng PowerShell.
  clipboardBefore = ps(`$t = Get-Clipboard -Raw -ErrorAction SilentlyContinue; if ($null -ne $t) { [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($t)) }`).out;
  clipboardTouched = true;
  const copied = await run(`return document.execCommand('copy')`, true);
  await sleep(300);
  const clip = Buffer.from(ps(`$t = Get-Clipboard -Raw; [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([string]$t))`).out, 'base64').toString('utf8');
  check('A5. Sao chép (Ctrl+C): bộ nhớ tạm của Windows có đúng chữ đã bôi đen, tiếng Việt dựng sẵn', copied === true && selected.trim().length > 10 && clip.trim() === selected.trim() && clip === clip.normalize('NFC'), `"${clip.trim()}"`);

  // Bôi đen nhiều dòng: có xuống dòng giữa các dòng.
  const multi = await run(`const spans = [...document.querySelectorAll('.vungxem .giay .textLayer span')].filter((s) => s.textContent.trim());
     const a = spans.find((s) => s.textContent.normalize('NFC').includes('QUYẾT ĐỊNH')); const b = spans.find((s) => s.textContent.normalize('NFC').includes('Về việc khen thưởng'));
     const range = document.createRange(); range.setStartBefore(a); range.setEndAfter(b);
     const sel = document.getSelection(); sel.removeAllRanges(); sel.addRange(range); const t = sel.toString().normalize('NFC'); sel.removeAllRanges(); return t;`);
  check('A6. Bôi đen qua hai dòng: chữ lấy ra có xuống dòng giữa hai dòng', /QUYẾT ĐỊNH\s*\n\s*Về việc khen thưởng/.test(multi ?? ''), JSON.stringify(multi));

  // Đổi zoom: lớp chữ vẽ lại theo cỡ mới, vẫn trùng trang.
  await click('.dieuhuong .nutma[title^="Phóng to"]');
  await click('.dieuhuong .nutma[title^="Phóng to"]');
  const zoomed = await waitFor(`const sheet = document.querySelector('.vungxem .giay'); const c = sheet?.querySelector('canvas'); const l = sheet?.querySelector('.textLayer');
     if (!c || !l || document.querySelector('.dieuhuong .zm').textContent.trim() !== '120 %') return null;
     const cr = c.getBoundingClientRect(); const lr = l.getBoundingClientRect();
     return Math.round(cr.width) === 624 && Math.abs(cr.width - lr.width) <= 1 && Math.abs(cr.height - lr.height) <= 1 && l.querySelectorAll('span').length > 20 && 1;`, 15000);
  check('A7. Phóng to 120 %: lớp chữ vẽ lại, vẫn trùng khít trang', !!zoomed);
  await click('.dieuhuong .nutma[title^="Thu nhỏ"]');
  await click('.dieuhuong .nutma[title^="Thu nhỏ"]');

  // ---- B. Tải lại sau khi file trên đĩa đổi ----------------------------------------------------------
  const buttons = await run(`return [...document.querySelectorAll('.dieuhuong .sua-mau, .dieuhuong .tai-lai')].map((b) => b.textContent.trim() + (b.disabled ? ' (tắt)' : '')).join(' | ')`);
  check('B1. Thanh xem trước có nút "Sửa mẫu" và "Tải lại"', buttons === 'Sửa mẫu | Tải lại', String(buttons));

  // Ghép tay {chuc_vu} sang cột "Ghi chú" (không phải cột tự ghép) để xem Tải lại có giữ không.
  await click('.tabs .tab', 'Mẫu');
  await waitFor(`return document.querySelectorAll('.bdong').length === 10`);
  await run(`[...document.querySelectorAll('.bdong')].find((r) => r.title === 'chuc_vu').querySelector('.lt-nut').click(); return 1;`);
  await waitFor(`return !!document.querySelector('.lt-menu .lt-col')`, 5000);
  await click('.lt-menu .lt-col', 'Ghi chú');
  const label = () => run(`return [...document.querySelectorAll('.bdong')].find((r) => r.title === 'chuc_vu')?.querySelector('.lt-nut span')?.textContent.trim()`);
  const mappedBefore = await label();
  // Xem dòng 3 ở tab Kết quả.
  await click('.tabs .tab', 'Kết quả');
  await waitFor(`return document.querySelectorAll('.ds .dong').length === 5`);
  await run(`document.querySelectorAll('.ds .dong')[2].click(); return 1;`);
  await waitText('Nguyễn Văn An', 1000);
  const row3 = await waitFor(`return !document.querySelector('.xem-tt') && document.querySelector('.ds .dong.chon .so')?.textContent.trim() === '3' && document.querySelector('.dieuhuong .tt b')?.textContent`, 20000);

  // Người dùng sửa mẫu (thêm một đoạn có biến mới) và thêm một dòng dữ liệu, lưu lại.
  py(ADD_PARAGRAPH, tpl, 'Ghi chú thêm: {ghi_chu} (đoạn mới sửa)');
  const rows = py(ADD_ROW, data);
  const stale = await run(`return (${LAYER_TEXT}).includes('đoạn mới sửa')`);
  const clicked = await reload();
  const fresh = await waitText('đoạn mới sửa', 60000);
  check('B2. Sửa mẫu trên đĩa rồi bấm "Tải lại": bản xem trước có đoạn mới (trước khi bấm thì chưa)', stale === false && clicked && !!fresh);
  const counts = await tabCount();
  const listed = await waitFor(`return document.querySelectorAll('.ds .dong').length === 6 && document.querySelector('.chan button.nut.chinh').textContent.trim()`, 20000);
  check('B3. Tải lại đọc cả dữ liệu: thêm 1 dòng thì danh sách thành 6, nút chính "Tạo 6 văn bản"', rows === '6' && listed === 'Tạo 6 văn bản', `${rows} dòng | ${listed} | ${counts}`);
  const stillRow3 = await run(`return document.querySelector('.ds .dong.chon .so')?.textContent.trim() + ' ' + document.querySelector('.dieuhuong .tt b')?.textContent`);
  check('B4. Tải lại giữ dòng đang xem (dòng 3)', !!row3 && stillRow3 === `3 ${row3}`, `${stillRow3}`);
  await click('.tabs .tab', 'Mẫu');
  const mappedText = await waitFor(`return [...document.querySelectorAll('.daytrai .dem')].map((e) => e.textContent.trim()).find((t) => /biến/.test(t))`, 10000);
  const mappedAfter = await label();
  const newField = await run(`return [...document.querySelectorAll('.bdong')].find((r) => r.title === 'ghi_chu')?.querySelector('.lt-nut span')?.textContent.trim()`);
  check(
    'B5. Tải lại giữ cách ghép tay ({chuc_vu} vẫn lấy từ "Ghi chú"), biến mới {ghi_chu} được ghép tự động',
    mappedBefore === 'Ghi chú' && mappedAfter === 'Ghi chú' && newField === 'Ghi chú' && mappedText === 'Đã ghép đủ 11 biến',
    `trước: ${mappedBefore}; sau: ${mappedAfter}; {ghi_chu}: ${newField}; ${mappedText}`,
  );
  const tplText = await waitText('{ghi_chu}', 40000);
  check('B6. Tab Mẫu sau khi tải lại: bản xem trước mẫu gốc có biến mới {ghi_chu}', !!tplText);
  await sleep(300);
  await shot('sm-2-tai-lai.png');

  // Mẫu bị xoá mất rồi bấm Tải lại: báo lỗi, giữ nguyên bản đang có; có lại file thì tải lại được.
  const hidden = tpl + '.an';
  fs.renameSync(tpl, hidden);
  await reload();
  const errorText = await waitFor(`return document.querySelector('.bao.loi.dai .dan')?.textContent.trim()`, 10000);
  const kept = await run(`return document.querySelectorAll('.bdong').length === 11 && !!document.querySelector('.vungxem canvas')`);
  fs.renameSync(hidden, tpl);
  await reload();
  const errorGone = await waitFor(`return !document.querySelector('.bao.loi.dai') && 1`, 10000);
  check('B7. File mẫu không còn: Tải lại báo lỗi, giữ bản đang có; có lại file thì hết lỗi', /Không đọc lại được mẫu mau_khen_thuong\.docx/.test(errorText ?? '') && kept === true && !!errorGone, String(errorText));

  // ---- C. Sửa mẫu bằng Word thật ------------------------------------------------------------------------
  const hasWord = ps(`[bool](Get-ItemProperty 'Registry::HKEY_CLASSES_ROOT\\Word.Application\\CLSID' -ErrorAction SilentlyContinue)`).out === 'True';
  if (!hasWord) {
    note('Máy không có Microsoft Word: bỏ qua phần C (mở mẫu bằng Word).');
  } else {
    const pidsBeforeEdit = wordPids();
    const visibleBeforeEdit = visibleWordPids();
    await click('.dieuhuong .sua-mau');
    const escaped = tpl.replace(/'/g, "''");
    const opened = ps(`$w = $null; for ($i = 0; $i -lt 60 -and -not $w; $i++) { Start-Sleep -Milliseconds 500
        $w = Get-Process WINWORD -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like '*mau_khen_thuong*' } | Select-Object -First 1 }
      if ($w) { "$($w.Id)|$($w.MainWindowTitle)" }`).out;
    const openedPid = opened.split('|')[0];
    if (opened) {
      note(
        visibleBeforeEdit.includes(openedPid)
          ? `Word ${openedPid} của người dùng đang mở nhận file mẫu`
          : pidsBeforeEdit.includes(openedPid)
            ? `file mẫu mở vào phiên Word ẩn ${openedPid} có từ trước (phiên xem trước của app được giao lại cho người dùng)`
            : `Word mới ${openedPid}`,
      );
    }
    check('C1. Bấm "Sửa mẫu": Microsoft Word hiện cửa sổ có tên file mẫu', !!opened, opened || 'không thấy cửa sổ Word');
    const noError = await run(`return !document.querySelector('.bao.loi.dai')`);
    check('C2. Ứng dụng không báo lỗi khi mở mẫu', noError === true);
    if (opened) {
      // Sửa trong chính cửa sổ Word đó, lưu, tài liệu vẫn mở. Lấy tài liệu qua khung soạn thảo (_WwG) của đúng
      // tiến trình vừa mở: BindToMoniker / GetActiveObject có thể bám nhầm một phiên Word ẩn khác trên máy.
      const edited = ps(`${WORD_DOC_BY_PID}
        $d = Get-WordDoc ${openedPid} '${escaped}'
        $d.Content.InsertAfter(' Sửa trong Word {them_tu_word}'); $d.Save(); "$($d.Saved)|$($d.ReadOnly)|$($d.Application.Documents.Count)"`);
      check('C3. Sửa và lưu trong Word (tài liệu vẫn mở)', edited.out.startsWith('True|False'), edited.out || edited.err.slice(0, 300));
      await reload();
      const fromWord = await waitText('{them_tu_word}', 60000);
      const fields = await run(`return document.querySelectorAll('.bdong').length + ' | ' + [...document.querySelectorAll('.daytrai .dem')].map((e) => e.textContent.trim()).join(' ')`);
      check('C4. Bấm "Tải lại" khi Word còn mở file: mẫu có nội dung vừa sửa, thêm biến {them_tu_word} chưa ghép', !!fromWord && fields === '12 | Chưa ghép 1 biến', String(fields));
      await sleep(300);
      await shot('sm-3-sau-khi-sua-bang-word.png');
      // Dọn: đóng tài liệu; Word do lần bấm này mở (không có từ trước) và không còn tài liệu nào thì thoát.
      const closed = ps(`${WORD_DOC_BY_PID}
        $d = Get-WordDoc ${openedPid} '${escaped}'; $a = $d.Application; $d.Close(0)
        if ('${visibleBeforeEdit.includes(openedPid) ? 'nguoi-dung' : 'kiem'}' -eq 'nguoi-dung') { 'giu (Word cua nguoi dung)'; return }
        # Word vừa đóng tài liệu có thể từ chối lời gọi vài giây: thử lại.
        for ($i = 0; $i -lt 20; $i++) { try { if ($a.Documents.Count -eq 0) { $a.Quit(0); 'thoat'; return } else { 'giu (con tai lieu khac)'; return } } catch { Start-Sleep -Milliseconds 500 } }
        'khong goi duoc Word'`);
      note(`đóng tài liệu trong Word: ${closed.out || closed.err.slice(0, 200)}`);
      for (let i = 0; i < 20 && wordPids().includes(openedPid); i++) await sleep(500);
    }
  }

  check('Không có lỗi trong console của giao diện', consoleErrors.length === 0, consoleErrors.slice(0, 3).join(' | '));
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  if (clipboardTouched) {
    if (clipboardBefore) {
      ps(`Set-Clipboard -Value ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('${clipboardBefore}')))`);
    } else {
      note('bộ nhớ tạm trước khi kiểm không có chữ nên không trả lại được; hiện còn dòng chữ bài kiểm đã sao chép.');
    }
  }
  try {
    socket?.close();
  } catch {}
  // Đóng cửa sổ như người dùng (WM_CLOSE) để ứng dụng tự đóng phiên Word ẩn của xem trước; không thoát mới buộc dừng.
  spawnSync('taskkill', ['/PID', String(app.pid)], { stdio: 'ignore' });
  for (let i = 0; i < 60 && app.exitCode === null; i++) await sleep(250);
  if (app.exitCode === null) app.kill();
  await sleep(1500);
  const left = wordPids().filter((p) => !wordBefore.includes(p));
  check('Sau khi đóng ứng dụng không còn tiến trình Word nào do bài kiểm mở', left.length === 0, left.length ? `còn PID ${left.join(', ')}` : `như trước: ${wordBefore.join(', ') || 'không có'}`);
}
const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
