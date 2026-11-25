// Spike S3: mở render.html trong Edge chạy ẩn, chờ pdf.js vẽ xong rồi chụp ảnh qua giao thức DevTools.
// Tự bật và tự tắt một máy chủ tĩnh trên 127.0.0.1 và một tiến trình Edge riêng (hồ sơ tạm trong out/).
// Cách dùng: node screenshot.mjs <file PNG kết quả>
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const outPng = path.resolve(process.argv[2]);
const httpPort = 8765;
const debugPort = 9333;
const edge = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const server = spawn('python', ['-I', path.join(here, 'serve.py'), root, String(httpPort)], {
  stdio: 'ignore',
});
const browser = spawn(
  edge,
  [
    '--headless=new',
    '--disable-gpu',
    '--no-first-run',
    `--user-data-dir=${path.join(root, 'out', 'edge-profile')}`,
    `--remote-debugging-port=${debugPort}`,
    `--window-size=${process.env.SHOT_SIZE ?? '1190,1000'}`,
    'about:blank',
  ],
  { stdio: 'ignore' },
);

let exitCode = 1;
try {
  let target;
  for (let i = 0; i < 40 && !target; i++) {
    await sleep(250);
    try {
      const list = await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json();
      target = list.find((t) => t.type === 'page');
    } catch {
      // Edge chưa sẵn sàng.
    }
  }
  if (!target) throw new Error('Không kết nối được DevTools của Edge.');

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
    (await send('Runtime.evaluate', { expression, returnByValue: true })).result?.result?.value;

  await send('Page.enable');
  // Tham số thứ hai (tuỳ chọn): đường dẫn PDF tính từ thư mục verify/, ví dụ ../out/khac.pdf
  const query = process.argv[3] ? `?pdf=${encodeURIComponent(process.argv[3])}` : '';
  await send('Page.navigate', { url: `http://127.0.0.1:${httpPort}/verify/render.html${query}` });

  let title = '';
  let log = '';
  for (let i = 0; i < 80; i++) {
    await sleep(250);
    title = await evaluate('document.title');
    log = (await evaluate('document.getElementById("log")?.textContent ?? ""')) ?? '';
    if (title === 'xong' || log.trim().length > 0) break;
  }
  console.log(`title: ${title}`);
  if (log.trim()) console.log(`log: ${log.trim()}`);
  if (title !== 'xong') throw new Error('pdf.js chưa vẽ xong trang.');

  const shot = await send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(outPng, Buffer.from(shot.result.data, 'base64'));
  console.log(`Da ghi ${outPng} (${fs.statSync(outPng).size} byte)`);
  await send('Browser.close');
  exitCode = 0;
} catch (error) {
  console.error(`LOI: ${error.message}`);
} finally {
  browser.kill();
  server.kill();
}
process.exit(exitCode);
