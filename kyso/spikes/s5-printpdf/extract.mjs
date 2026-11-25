// Spike S5: trích văn bản từng trang bằng pdf.js (dùng node_modules của spike S3).
import fs from 'node:fs';
import { getDocument } from '../s3-font/verify/node_modules/pdfjs-dist/legacy/build/pdf.mjs';
const data = new Uint8Array(fs.readFileSync(process.argv[2]));
const doc = await getDocument({ data, disableFontFace: true, verbosity: 0 }).promise;
for (let n = 1; n <= doc.numPages; n++) {
  const page = await doc.getPage(n);
  const content = await page.getTextContent();
  const lines = new Map();
  for (const item of content.items) {
    if (typeof item.str !== 'string' || !item.str.trim()) continue;
    const y = Math.round(item.transform[5]);
    lines.set(y, (lines.get(y) ?? '') + item.str);
  }
  const first = content.items.find((i) => typeof i.str === 'string' && i.str.trim());
  console.log(`--- trang ${n}: chữ đầu tiên tại x=${first ? first.transform[4].toFixed(1) : '?'}pt, y=${first ? first.transform[5].toFixed(1) : '?'}pt`);
  for (const [, text] of [...lines.entries()].sort((a, b) => b[0] - a[0])) console.log('  ' + text);
}
