// Kiểm giờ ký (TD-54) trên file chạy thật (bản `--features dev-keys`, khoá thử ở SMKS_DEV_PKI):
// 1) Có mạng: giờ ký lấy theo máy chủ dấu thời gian Ban Cơ yếu — Nhật ký ghi nguồn giờ, file đã ký có dấu thời gian,
//    `/M` (giờ ký trong file, cùng giá trị với dòng "Ngày ký") khớp `genTime` của dấu thời gian.
// 2) Không có mạng (SMKS_TSA_URL trỏ tới cổng đóng, chỉ bản dev-keys đọc biến này): vẫn ký được theo giờ máy tính,
//    file không có dấu thời gian, Nhật ký và hộp kết quả ghi rõ.
// Đọc file đã ký bằng pyHanko / asn1crypto (độc lập với lõi).
//
// Cách dùng: node app/tests/ks-gioky.mjs <SaoMai-KySo.exe> <thư mục repo> <thư mục làm việc> <thư mục khoá thử>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, repo, work, pki] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9358;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
const shots = path.join(work, 'anh');
const dirOn = path.join(work, 'co-mang');
const dirOff = path.join(work, 'mat-mang');
for (const d of [cfgDir, shots, dirOn, dirOff]) fs.mkdirSync(d, { recursive: true });
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirOn, 'van ban.pdf'));
fs.copyFileSync(path.join(repo, 'fixtures/pdf/table-xref.pdf'), path.join(dirOff, 'van ban.pdf'));

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}

/** `/M` và `genTime` (giây Unix) của chữ ký cuối trong file, đọc bằng pyHanko + asn1crypto. */
const timesOf = (file) => {
  const py = spawnSync('python', ['-I', '-c', `
import sys, json, re, datetime
from pyhanko.pdf_utils.reader import PdfFileReader
with open(sys.argv[1], 'rb') as f:
    r = PdfFileReader(f)
    s = r.embedded_signatures[-1]
    m = str(s.sig_object['/M'])
    g = re.match(r"D:(\\d{14})([+-])(\\d\\d)'(\\d\\d)", m)
    t = datetime.datetime.strptime(g.group(1), '%Y%m%d%H%M%S')
    off = (int(g.group(3)) * 3600 + int(g.group(4)) * 60) * (1 if g.group(2) == '+' else -1)
    m_unix = int(t.replace(tzinfo=datetime.timezone.utc).timestamp()) - off
    ua = s.signer_info['unsigned_attrs']
    gen = None
    if ua.native:
        for a in ua:
            if a['type'].native == 'signature_time_stamp_token':
                tst = a['values'][0]['content']['encap_content_info']['content'].parsed
                gen = int(tst['gen_time'].native.timestamp())
    from pyhanko.sign.validation.dss import DocumentSecurityStore
    try:
        d = DocumentSecurityStore.read_dss(r)
        dss = {"certs": len(list(d.certs)), "ocsps": len(d.ocsps), "crls": len(d.crls)}
    except Exception:
        dss = None
    print(json.dumps({"m": m, "mUnix": m_unix, "genTime": gen, "dss": dss}))`, file], { encoding: 'utf8' });
  if (py.status !== 0) throw new Error(`pyHanko: ${py.stderr}`);
  return JSON.parse(py.stdout);
};

/** Mở app, ký file trong `folder` bằng khoá thử; trả về hộp kết quả, Nhật ký và giờ trong file đã ký. */
const signOnce = async (folder, extraEnv, shotName) => {
  const app = spawn(exe, [], {
    env: {
      ...process.env,
      ...extraEnv,
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
    await send('Runtime.enable');
    await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
    // Kiểu "Chỉ thông tin" để không cần ảnh.
    await run(`const c = await window.__TAURI_INTERNALS__.invoke('config_defaults'); c.signature.display = 'text';
               await window.__TAURI_INTERNALS__.invoke('config_save', { config: c }); return 1;`);
    await send('Page.reload');
    await sleep(1500);
    await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
    await sleep(500);
    await run(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true })); return 1;`);
    await waitFor(`return /— Ký số/.test(document.title)`);
    await sleep(1200);
    await run(`await window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', { target: { kind: 'Webview', label: 'main' },
      event: 'tauri://drag-drop', payload: { paths: [${JSON.stringify(folder)}], position: { x: 500, y: 400 } } }); return 1;`);
    await waitFor(`return document.querySelectorAll('.ds .dong').length === 1 && !!document.querySelector('.giay .chuky-o') && 1`, 20000);
    await sleep(800);
    const t0 = Math.floor(Date.now() / 1000);
    await run(`[...document.querySelectorAll('.chan button.nut.chinh')].find((b) => !b.disabled)?.click(); return 1;`);
    await waitFor(`return document.querySelectorAll('app-popup .ct-dong').length`, 30000);
    await run(`const r = [...document.querySelectorAll('app-popup .ct-dong')].find((l) => l.textContent.includes('Khoá phần mềm') && !l.classList.contains('tat'));
       r?.querySelector('input')?.click(); return !!r;`);
    await run(`[...document.querySelectorAll('app-popup button.nut.chinh')].find((b) => b.textContent.includes('Ký số'))?.click(); return 1;`);
    const done = await waitFor(`const p = document.querySelector('app-popup .so-lon'); return p && [...p.querySelectorAll('b')].map((b) => b.textContent).join('/')`, 120000);
    const t1 = Math.floor(Date.now() / 1000);
    const dialog = await run(`const p = document.querySelector('app-popup .mo-ta'); return { text: p?.textContent.replace(/\\s+/g, ' ').trim() ?? '', note: !!document.querySelector('app-popup .ks-giomay') };`);
    const r = await send('Page.captureScreenshot', { format: 'png' });
    if (r.result?.data) fs.writeFileSync(path.join(shots, shotName), Buffer.from(r.result.data, 'base64'));
    await run(`[...document.querySelectorAll('app-popup button')].find((b) => b.textContent.includes('Nhật ký'))?.click(); return 1;`);
    await sleep(600);
    const log = (await run(`return [...document.querySelectorAll('.log .ln .txt')].map((e) => e.textContent.trim())`)) ?? [];
    socket.close();
    const signed = path.join(`${folder}_DaKySo`, 'van ban_signed.pdf');
    return { done, dialog, log, t0, t1, times: fs.existsSync(signed) ? timesOf(signed) : null, consoleErrors };
  } finally {
    app.kill();
    await sleep(1500);
  }
};

try {
  // 1) Có mạng.
  const on = await signOnce(dirOn, {}, '1-co-mang.png');
  check('Có mạng: ký 1/1', on.done === '1/1/0', String(on.done));
  check('Có mạng: Nhật ký ghi "Giờ ký lấy theo máy chủ dấu thời gian Ban Cơ yếu", hộp kết quả không nhắc giờ máy tính',
    on.log.some((l) => l.includes('Giờ ký lấy theo máy chủ dấu thời gian Ban Cơ yếu')) && !on.dialog.note, JSON.stringify({ log: on.log, dialog: on.dialog }));
  const ton = on.times;
  check('Có mạng: file có dấu thời gian; giờ ký /M khớp genTime của Ban Cơ yếu (lệch ≤ 5 giây, không sau genTime)',
    !!ton && ton.genTime != null && Math.abs(ton.mUnix - ton.genTime) <= 5 && ton.mUnix <= ton.genTime + 1, JSON.stringify(ton));

  check('Có mạng: ký dài hạn — file có /DSS với phản hồi OCSP (pyHanko đọc), Nhật ký ghi "Ký dài hạn"',
    !!ton?.dss && ton.dss.ocsps >= 1 && ton.dss.certs >= 2 && on.log.some((l) => l.includes('Ký dài hạn')), JSON.stringify({ dss: ton?.dss }));

  // 2) Không có mạng (máy chủ dấu thời gian không kết nối được).
  const off = await signOnce(dirOff, { SMKS_TSA_URL: 'http://127.0.0.1:9/' }, '2-mat-mang.png');
  check('Không có mạng: vẫn ký được 1/1', off.done === '1/1/0', String(off.done));
  check('Không có mạng: hộp kết quả nhắc "Ký theo giờ máy tính … không có dấu thời gian"',
    off.dialog.note && /Ký theo giờ máy tính/.test(off.dialog.text) && /không có dấu thời gian/.test(off.dialog.text), JSON.stringify(off.dialog));
  // Từ TD-57, chữ ký không có dấu thời gian vẫn được kiểm người ký: với khoá thử, kết luận sau khi ký là "Người ký không
  // thuộc CA Ban Cơ yếu" (token thật của Ban Cơ yếu thì là "Không có dấu thời gian"). Việc file không có dấu thời gian
  // được kiểm ở mục sau bằng pyHanko.
  check('Không có mạng: Nhật ký ghi lý do (không lấy được giờ máy chủ Ban Cơ yếu, ký theo giờ máy tính, văn bản không có dấu thời gian)',
    off.log.some((l) => /Không lấy được giờ của máy chủ dấu thời gian Ban Cơ yếu/.test(l) && /giờ máy tính/.test(l) && /không có dấu thời gian/i.test(l)),
    JSON.stringify(off.log));
  const toff = off.times;
  check('Không có mạng: file không có dấu thời gian, không có /DSS, giờ ký /M là giờ máy tính lúc ký',
    !!toff && toff.genTime == null && toff.dss == null && toff.mUnix >= off.t0 - 2 && toff.mUnix <= off.t1 + 2, JSON.stringify({ ...toff, t0: off.t0, t1: off.t1 }));

  check('Không có lỗi trong console của giao diện', on.consoleErrors.length === 0 && off.consoleErrors.length === 0,
    [...on.consoleErrors, ...off.consoleErrors].slice(0, 3).join(' | '));
} catch (error) {
  console.error(`LOI: ${error.message}`);
  results.push(false);
}
const ok = results.length > 0 && results.every(Boolean);
console.log(`KET QUA: ${ok ? 'DAT' : 'KHONG DAT'} (${results.filter(Boolean).length}/${results.length})`);
process.exit(ok ? 0 : 1);
