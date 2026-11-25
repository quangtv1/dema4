// Kiểm menu Hỗ trợ: "Hướng dẫn sử dụng" (HT-01, F1) và "Cài đặt driver USB Token" (HT-02) trên app thật.
// Bản build `--features dev-keys` (để giả lập máy chưa có driver bằng SMKS_DEV_NO_DRIVER). Kết quả nhận diện
// driver được đối chiếu với registry / file của máy bằng PowerShell (độc lập với app).
// Không bấm "Mở trang tải phần mềm" (sẽ mở trình duyệt thật); chỉ kiểm lệnh open_link từ chối trang lạ.
//
// Cách dùng: node app/tests/ht-ui.mjs <SaoMai-KySo.exe> <thư mục làm việc>
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [exe, work] = process.argv.slice(2).map((p) => path.resolve(p));
const debugPort = 9361;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};
fs.rmSync(work, { recursive: true, force: true });
const cfgDir = path.join(work, 'cfg');
fs.mkdirSync(cfgDir, { recursive: true });

/** Đối chiếu độc lập: driver theo cách nhận diện của bộ cài cũ. */
const ps = spawnSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', `
$b = (Test-Path 'HKLM:\\SOFTWARE\\WOW6432Node\\bit4id') -or (Test-Path "\${env:ProgramFiles(x86)}\\Bit4Id\\Universal MW\\uninst.exe")
$e = ((Get-ChildItem 'HKLM:\\SOFTWARE\\Microsoft\\Cryptography\\Defaults\\Provider').PSChildName | Where-Object { $_ -like 'eToken*' }).Count -gt 0 -or (Test-Path "$env:WINDIR\\System32\\eTOKCSP.dll")
$v = (Get-ChildItem 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall','HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall' | ForEach-Object { Get-ItemProperty $_.PSPath } | Where-Object { $_.DisplayName -like 'Bit4id*' } | Select-Object -First 1).DisplayVersion
ConvertTo-Json -Compress @{ bit4id = $b; etoken = $e; bit4idVersion = $v }`], { encoding: 'utf8' });
const machine = JSON.parse(ps.stdout || '{}');

if ((spawnSync('tasklist', ['/FI', `IMAGENAME eq ${path.basename(exe)}`], { encoding: 'utf8' }).stdout ?? '').includes(path.basename(exe))) {
  console.error(`LOI: ${path.basename(exe)} đang chạy; đóng trước khi kiểm.`);
  process.exit(1);
}

async function session(extraEnv, body) {
  try {
    await fetch(`http://127.0.0.1:${debugPort}/json`);
    throw new Error(`cổng ${debugPort} đang bận`);
  } catch (e) {
    if (String(e.message).includes('đang bận')) throw e;
  }
  const app = spawn(exe, [], {
    env: { ...process.env, SMKS_CONFIG_DIR: cfgDir, ...extraEnv, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}` },
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
    const run = async (code) => {
      const reply = await send('Runtime.evaluate', {
        expression: `(async () => { try { return { ok: await (async () => { ${code} })() }; } catch (e) { return { err: String(e?.message ?? e) }; } })()`,
        returnByValue: true,
        awaitPromise: true,
      });
      const v = reply.result?.result?.value;
      return v?.err ? { __err: v.err } : v?.ok;
    };
    const waitFor = async (code, ms = 20000) => {
      const end = Date.now() + ms;
      while (Date.now() < end) {
        const v = await run(code);
        if (v && !v.__err) return v;
        await sleep(250);
      }
      return null;
    };
    const shot = async (name) => {
      const r = await send('Page.captureScreenshot', { format: 'png' });
      if (r.result?.data) fs.writeFileSync(path.join(work, name), Buffer.from(r.result.data, 'base64'));
    };
    const clickText = (sel, text) =>
      run(`const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)}) && !e.disabled);
           if (!el) return false; el.click(); return true;`);
    const key = (k) => send('Input.dispatchKeyEvent', { type: 'keyDown', key: k, code: k, windowsVirtualKeyCode: k === 'F1' ? 112 : 27 }).then(() =>
      send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code: k, windowsVirtualKeyCode: k === 'F1' ? 112 : 27 }));
    await waitFor(`return document.querySelectorAll('.tc-the').length === 3`);
    await body({ run, waitFor, shot, clickText, key });
    socket.close();
  } finally {
    app.kill();
    await sleep(800);
  }
}

const POP = (t) => `document.querySelector('.popup[aria-label="${t}"]')`;
const openHelpMenu = async (h, item) => {
  await h.clickText('.menubar button.mb', 'Hỗ trợ');
  await h.waitFor(`return !!document.querySelector('.menubar .menu')`);
  await h.clickText('.menubar .menu button', item);
};

try {
  await session({}, async (h) => {
    // 1. F1 mở Hướng dẫn: 9 mục, mục đầu "Bắt đầu nhanh", có phím vẽ bằng <kbd>, chữ đậm, không còn hộp "đang phát triển".
    await h.key('F1');
    const hd = await h.waitFor(`const p = ${POP('Hướng dẫn sử dụng')}; return p && { nav: [...p.querySelectorAll('.hd-nav .nav')].map((b) => b.textContent.trim()),
        chon: p.querySelector('.hd-nav .nav.chon')?.textContent.trim(), h2: p.querySelector('.hd-cot h2')?.textContent, kbd: p.querySelectorAll('.hd-cot kbd').length,
        b: p.querySelectorAll('.hd-cot li b').length, raw: /\\*\\*|\\[\\[/.test(p.querySelector('.hd-cot').textContent),
        space: [...p.querySelectorAll('.hd-cot li')].map((li) => li.textContent).filter((t) => /\\s[,;:)]|\\s\\.(\\s|$)|\\(\\s/.test(t)), dev: !!${POP('Hướng dẫn sử dụng')}?.textContent.includes('đang phát triển trong bản cập nhật') }`);
    await h.shot('1-huong-dan.png');
    check('F1 mở Hướng dẫn sử dụng: 9 mục, đang ở "Bắt đầu nhanh", phím tắt vẽ thành <kbd>, chữ đậm, không còn ký hiệu thô, không có dấu cách thừa trước dấu câu',
      hd?.nav?.length === 9 && hd.chon === 'Bắt đầu nhanh' && hd.h2 === 'Bắt đầu nhanh' && hd.kbd >= 1 && hd.b >= 3 && !hd.raw && !hd.dev && hd.space.length === 0, JSON.stringify(hd));
    // 2. Nút sau / chọn mục bên trái.
    await h.clickText(`.popup[aria-label="Hướng dẫn sử dụng"] .cuoi button`, 'Ký số văn bản');
    const t2 = await h.waitFor(`return ${POP('Hướng dẫn sử dụng')}?.querySelector('.hd-cot h2')?.textContent === 'Ký số văn bản' && 'Ký số văn bản'`);
    await h.clickText(`.popup[aria-label="Hướng dẫn sử dụng"] .hd-nav .nav`, 'Câu hỏi thường gặp');
    const t3 = await h.waitFor(`const p = ${POP('Hướng dẫn sử dụng')}; return p?.querySelector('.hd-cot h2')?.textContent === 'Câu hỏi thường gặp' && { next: [...p.querySelectorAll('.cuoi button')].map((b) => b.textContent.trim()), text: p.querySelector('.hd-cot').textContent }`);
    await h.shot('2-hoi-dap.png');
    check('Nút chuyển mục và danh mục bên trái: tới "Ký số văn bản", rồi "Câu hỏi thường gặp" (mục cuối không có nút sau)',
      t2 === 'Ký số văn bản' && !!t3 && !t3.next.some((x) => x.includes('Câu hỏi')) && /hotro@saomaiso\.com/.test(t3.text) && /Cài đặt driver USB Token/.test(t3.text), JSON.stringify(t3?.next));
    await h.key('Escape');
    await sleep(300);
    const closed = await h.run(`return !${POP('Hướng dẫn sử dụng')}`);
    await openHelpMenu(h, 'Hướng dẫn sử dụng');
    const viaMenu = await h.waitFor(`return ${POP('Hướng dẫn sử dụng')}?.querySelector('.hd-nav .nav.chon')?.textContent.trim()`);
    check('Esc đóng; menu Hỗ trợ › Hướng dẫn sử dụng mở lại từ mục đầu', closed && viaMenu === 'Bắt đầu nhanh', String(viaMenu));
    await h.clickText(`.popup[aria-label="Hướng dẫn sử dụng"] .cuoi button`, 'Đóng');
    await sleep(300);

    // 3. Cài đặt driver USB Token: trạng thái khớp máy (đối chiếu độc lập).
    await openHelpMenu(h, 'Cài đặt driver USB Token');
    const dr = await h.waitFor(`const p = ${POP('Cài đặt driver USB Token')}; return p && p.querySelector('.drv-ds') && { rows: [...p.querySelectorAll('.drv-dong')].map((r) => ({ ten: r.querySelector('.drv-ten b')?.textContent, tt: r.querySelector('.ketnoi')?.textContent.trim() })),
        bao: p.querySelector('.bao b')?.textContent, buttons: [...p.querySelectorAll('.cuoi button')].map((b) => b.textContent.trim()), url: p.querySelector('.drv-hd')?.textContent.includes('https://ca.gov.vn/tai-phan-mem') }`);
    await h.shot('3-driver.png');
    const b = dr?.rows?.find((r) => /Bit4id/.test(r.ten ?? ''));
    const e = dr?.rows?.find((r) => /TokenManager/.test(r.ten ?? ''));
    const bitOk = machine.bit4id ? /Đã cài/.test(b?.tt ?? '') && (!machine.bit4idVersion || (b?.tt ?? '').includes(machine.bit4idVersion)) : /Chưa cài/.test(b?.tt ?? '');
    const etOk = machine.etoken ? /Đã cài/.test(e?.tt ?? '') : /Chưa cài/.test(e?.tt ?? '');
    const anyOk = (machine.bit4id || machine.etoken) ? dr?.bao === 'Máy đã có driver USB Token' : dr?.bao === 'Máy chưa có driver USB Token';
    check('Hộp Cài đặt driver: trạng thái Bit4id / TokenManager khớp registry của máy (cả phiên bản), có hướng dẫn tải ở ca.gov.vn và nút mở trang',
      bitOk && etOk && anyOk && dr.url && dr.buttons.some((x) => x.includes('Mở trang tải phần mềm')) && dr.buttons.some((x) => x.includes('Kiểm tra lại')),
      JSON.stringify({ machine, app: dr?.rows, bao: dr?.bao }));
    const bad = await h.run(`try { await window.__TAURI_INTERNALS__.invoke('open_link', { id: 'https://evil.example' }); return 'mở'; } catch (e) { return e?.message ?? String(e); }`);
    check('Lệnh open_link chỉ mở trang cố định, từ chối địa chỉ lạ', bad !== 'mở' && /Không có trang/.test(String(bad)), String(bad));
  });

  // 4. Giả lập máy chưa có driver (chỉ bản dev-keys): báo chưa có, hướng dẫn 4 bước tải ở trang của Ban Cơ yếu.
  await session({ SMKS_DEV_NO_DRIVER: '1' }, async (h) => {
    await openHelpMenu(h, 'Cài đặt driver USB Token');
    const dr = await h.waitFor(`const p = ${POP('Cài đặt driver USB Token')}; return p && p.querySelector('.drv-ds') && { bao: p.querySelector('.bao b')?.textContent,
        rows: [...p.querySelectorAll('.drv-dong .ketnoi')].map((x) => x.textContent.trim()), h3: p.querySelector('.drv-hd h3')?.textContent, steps: p.querySelectorAll('.drv-hd li').length,
        text: p.querySelector('.drv-hd')?.textContent }`);
    await h.shot('4-chua-co-driver.png');
    check('Máy chưa có driver: cảnh báo "Máy chưa có driver USB Token", mọi dòng "Chưa cài", 4 bước tải ở https://ca.gov.vn/tai-phan-mem (Bit4id / VGCA TokenManager)',
      dr?.bao === 'Máy chưa có driver USB Token' && dr.rows.every((r) => r === 'Chưa cài') && dr.h3 === 'Cách tải và cài driver' && dr.steps === 4 &&
        /ca\.gov\.vn\/tai-phan-mem/.test(dr.text) && /Bit4id/.test(dr.text) && /VGCA TokenManager/.test(dr.text), JSON.stringify(dr && { ...dr, text: undefined }));
  });
} catch (error) {
  console.error(`LOI: ${error.stack ?? error.message}`);
  results.push(false);
}
const ok = results.length > 0 && results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
