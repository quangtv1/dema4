// Kiểm tra khói cho file chạy của ứng dụng: mở được cửa sổ, giao diện nạp được, gọi được lõi qua IPC,
// và chỉ chạy một bản. Điều khiển WebView2 qua giao thức DevTools (bật bằng biến môi trường chỉ cho
// tiến trình thử này), không chụp màn hình desktop.
//
// Cách dùng: node app/tests/smoke.mjs <đường dẫn SaoMai-KySo.exe> [file PNG để lưu ảnh giao diện]
// Mã thoát 0 khi mọi phép kiểm đạt.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const exe = path.resolve(process.argv[2]);
const screenshotPath = process.argv[3] ? path.resolve(process.argv[3]) : null;
const debugPort = 9335;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`[${ok ? 'DAT' : 'KHONG'}] ${name}${detail ? ` — ${detail}` : ''}`);
};

const env = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
};
const app = spawn(exe, [], { env, stdio: 'ignore' });
let appExited = false;
app.on('exit', () => {
  appExited = true;
});

try {
  let target;
  for (let i = 0; i < 80 && !target && !appExited; i++) {
    await sleep(250);
    try {
      const list = await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json();
      target = list.find((t) => t.type === 'page');
    } catch {
      // WebView2 chưa sẵn sàng.
    }
  }
  check('Ứng dụng khởi động và WebView2 sẵn sàng', Boolean(target), target ? target.url : 'không thấy trang');
  if (!target) throw new Error('dừng');

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
  const evaluate = async (expression) =>
    (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result
      ?.result?.value;
  const text = (testId) =>
    evaluate(`document.querySelector('[data-testid="${testId}"]')?.textContent?.trim() ?? null`);

  let version = null;
  for (let i = 0; i < 40 && !version; i++) {
    await sleep(250);
    version = await evaluate(`[...document.querySelectorAll('.tc-day *')].map((e) => e.textContent).join(' ').match(/1\.5\.0[\w.-]*/)?.[0] ?? null`);
  }
  check('Giao diện nạp và gọi được lõi qua IPC (phiên bản ở chân Trang chủ)', Boolean(version), version ?? 'không có');
  check('Trang chủ có 3 thẻ chức năng (đã bỏ thẻ Vị trí chữ ký)', (await evaluate(`document.querySelectorAll('.tc-the').length`)) === 3);
  check('Tiêu đề trang', (await evaluate('document.title')) === 'Sao Mai Ký số — Trang chủ');
  check('Ngôn ngữ trang là tiếng Việt', (await evaluate('document.documentElement.lang')) === 'vi');

  // CSS của component phải được áp (CSP không được chặn các thẻ <style> Angular chèn lúc chạy).
  const css = (selector, property) =>
    evaluate(`getComputedStyle(document.querySelector('${selector}')).getPropertyValue('${property}')`);
  const cardLayout = await css('.tc-the', 'display');
  check('CSS được áp: thẻ Trang chủ không còn kiểu mặc định của nút', cardLayout !== 'inline-block', cardLayout);
  const bodyBg = await css('body', 'background-color');
  check('Biến thiết kế được áp: nền body', bodyBg !== 'rgba(0, 0, 0, 0)', bodyBg);

  // Command không được cấp quyền phải bị từ chối.
  const denied = await evaluate(
    `window.__TAURI_INTERNALS__.invoke('plugin:app|version').then(() => 'DUOC PHEP', (e) => 'bi tu choi: ' + e)`,
  );
  check('Command ngoài danh sách cấp quyền bị từ chối', String(denied).startsWith('bi tu choi'), String(denied).slice(0, 90));

  const size = await evaluate('JSON.stringify([window.innerWidth, window.innerHeight])');
  console.log(`     kích thước vùng giao diện: ${size}`);

  if (screenshotPath) {
    const shot = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(screenshotPath, Buffer.from(shot.result.data, 'base64'));
    console.log(`     đã lưu ảnh giao diện: ${screenshotPath}`);
  }

  // Mở bản thứ hai: phải tự thoát, bản đầu vẫn chạy (SH-02).
  const second = spawn(exe, [], { stdio: 'ignore' });
  const secondExit = await Promise.race([
    new Promise((resolve) => second.on('exit', (code) => resolve(code))),
    sleep(8000).then(() => 'timeout'),
  ]);
  if (secondExit === 'timeout') second.kill();
  check('Bản thứ hai tự thoát (chỉ chạy một bản)', secondExit !== 'timeout' && !appExited, `mã thoát ${secondExit}`);
  socket.close();
} catch (error) {
  if (error.message !== 'dừng') console.error(`LOI: ${error.message}`);
  results.push(false);
} finally {
  app.kill();
}

const ok = results.every(Boolean);
console.log(ok ? 'KET QUA: DAT' : 'KET QUA: KHONG DAT');
process.exit(ok ? 0 : 1);
