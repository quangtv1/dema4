// Spike S3: trích văn bản bằng pdf.js (bộ đọc độc lập với code sinh PDF) rồi so với chuỗi mong đợi.
// Cách dùng: node verify.mjs <file.pdf> <expected.txt>
import fs from 'node:fs';
import { getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';

const [pdfPath, expectedPath] = process.argv.slice(2);
const data = new Uint8Array(fs.readFileSync(pdfPath));
const doc = await getDocument({ data, disableFontFace: true, verbosity: 0 }).promise;
const page = await doc.getPage(1);
const content = await page.getTextContent();

const lines = new Map();
for (const item of content.items) {
  if (typeof item.str !== 'string' || item.str.length === 0) continue;
  const y = Math.round(item.transform[5] * 10) / 10;
  lines.set(y, (lines.get(y) ?? '') + item.str);
}
const got = [...lines.entries()].sort((a, b) => b[0] - a[0]).map(([, text]) => text);
const expected = fs.readFileSync(expectedPath, 'utf8').split('\n').filter((l) => l.length > 0);

let mismatches = 0;
const count = Math.max(got.length, expected.length);
for (let i = 0; i < count; i++) {
  if (got[i] === expected[i]) continue;
  mismatches++;
  console.log(`LECH dong ${i + 1}`);
  console.log(`  mong doi: ${expected[i]}`);
  console.log(`  pdf.js  : ${got[i]}`);
}
console.log(`pdf.js ${(await import('pdfjs-dist/package.json', { with: { type: 'json' } })).default.version}: ${count - mismatches}/${count} dong khop, ${mismatches} dong lech`);
process.exit(mismatches === 0 ? 0 : 1);
