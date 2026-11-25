/** Bỏ dấu tiếng Việt để tìm kiếm (prototype `boDau()` L613). */
export function boDau(s: string): string {
  return s
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/đ/g, 'd')
    .replace(/Đ/g, 'D');
}

/** Chuỗi so khớp tìm kiếm: bỏ dấu, chữ thường (mục 6.3.2). */
export function searchKey(s: string): string {
  return boDau(s.trim().toLowerCase());
}

export function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

/** Định dạng thời lượng như `fmtDur()` của prototype (L983): `mm:ss`, từ 60 phút trở lên là `<giờ>h<phút>m`. */
export function fmtDur(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  const m = Math.floor(s / 60);
  const pad = (n: number) => (n < 10 ? '0' + n : String(n));
  if (m >= 60) {
    return Math.floor(m / 60) + 'h' + pad(m % 60) + 'm';
  }
  return pad(m) + ':' + pad(s % 60);
}

/** Tên file trong một đường dẫn Windows hoặc POSIX. */
export function baseName(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i >= 0 ? path.slice(i + 1) : path;
}

/** Đuôi file viết thường, không có dấu chấm. */
export function extName(path: string): string {
  const name = baseName(path);
  const i = name.lastIndexOf('.');
  return i > 0 ? name.slice(i + 1).toLowerCase() : '';
}
