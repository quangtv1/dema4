/*
 * Bộ icon chép từ prototype (biến `P` L537–591, viewBox 24; biến `LA` L596–601, viewBox 100).
 * Chỉ giữ những icon giao diện đang dùng. Nội dung là hằng số trong mã, không lấy từ dữ liệu người dùng.
 */
export const ICONS = {
  save: '<path d="M5 3h11l4 4v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M8 3v5h7M8 21v-7h8v7"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M5 7l1 13h12l1-13M9 7V4h6v3"/>',
  key: '<circle cx="8" cy="15" r="4"/><path d="m11 12 9-9M17 6l3 3M15 8l2 2"/>',
  stamp: '<path d="M9 13V9.5a3 3 0 1 1 6 0V13"/><path d="M4 17a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v1H4z"/><path d="M5 21h14"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  copy: '<rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
  back: '<path d="m12 19-7-7 7-7M5 12h14"/>',
  grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
  help: '<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>',
  home: '<path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V21h14V9.5"/><path d="M10 21v-6h4v6"/>',
  pen: '<path d="M12 20h9"/><path d="M16.4 3.6a2.1 2.1 0 0 1 3 3L7.4 18.6a2 2 0 0 1-.9.5l-2.9.9.9-2.9a2 2 0 0 1 .5-.9z"/>',
  tao: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h4"/><path d="M14 3v5h5"/><rect x="13" y="12" width="8" height="9" rx="1"/><path d="M13 16.5h8M17 12v9"/>',
  shield: '<path d="M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6z"/><path d="M9 12l2 2 4-4"/>',
  frame: '<path d="M5 9V5h4M15 5h4v4M19 15v4h-4M9 19H5v-4"/><rect x="9" y="9" width="6" height="6" rx="1"/>',
  refresh: '<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>',
  info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
  send: '<path d="M22 2 11 13"/><path d="m22 2-7 20-4-9-9-4z"/>',
  book: '<path d="M4 19.5V5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z"/><path d="M4 19.5A2 2 0 0 0 6 21.5h13"/><path d="M8 7h7M8 11h5"/>',
  usb: '<rect x="7" y="9" width="10" height="12" rx="2"/><path d="M9 9V3h6v6"/><path d="M11 5.5v1M13 5.5v1"/>',
  file: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/>',
  doc: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6M8 13h8M8 17h5"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  folderOpen: '<path d="M3 17V7a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v1"/><path d="M3 17a2 2 0 0 0 2 2h12.5a2 2 0 0 0 1.9-1.4L21.5 12H7.5a2 2 0 0 0-1.9 1.4z"/>',
  eye: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  x: '<path d="M18 6 6 18M6 6l12 12"/>',
  xc: '<circle cx="12" cy="12" r="9"/><path d="M9.2 9.2l5.6 5.6M14.8 9.2l-5.6 5.6"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
  chevL: '<path d="m15 18-6-6 6-6"/>',
  chevR: '<path d="m9 18 6-6-6-6"/>',
  chevD: '<path d="m6 9 6 6 6-6"/>',
  chevU: '<path d="m18 15-6-6-6 6"/>',
  panelL: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m16 15-3-3 3-3"/>',
  panelR: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m14 9 3 3-3 3"/>',
  zoomIn: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5M8 11h6M11 8v6"/>',
  zoomOut: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5M8 11h6"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  checkC: '<circle cx="12" cy="12" r="9"/><path d="m8.5 12 2.5 2.5 4.5-5"/>',
  alert: '<circle cx="12" cy="12" r="9"/><path d="M12 8v4.5M12 16h.01"/>',
  warn: '<path d="M12 3 22 20H2z"/><path d="M12 10v4M12 17h.01"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  table: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M9 4v16"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2"/>',
  pause: '<rect x="6.5" y="5" width="3.5" height="14" rx="1"/><rect x="14" y="5" width="3.5" height="14" rx="1"/>',
  edit: '<path d="M11 4H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-6"/><path d="M18.4 2.6a2 2 0 0 1 2.9 2.9L12 14.8 8 16l1.2-4z"/>',
  download: '<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>',
  image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="m21 16-5-5-9 9"/>',
  filter: '<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>',
  terminal: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3M12 15h5"/>',
  list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
  /** Cung tròn của icon quay `spin()` (L931). */
  spin: '<path d="M21 12a9 9 0 1 1-6.2-8.56"/>',
} as const;

export type IconName = keyof typeof ICONS;

export const BIG_ICONS = {
  ky: '<path d="M14 12H49L60 23V88H14Z" stroke-width="3"/><path d="M49 12V23H60" stroke-width="3"/><path d="M23 27H44" stroke-width="2.2"/><path d="M23 36H48" stroke-width="2.2"/><path d="M23 45H38" stroke-width="2.2"/><path d="M22 65L23.1 63.5L24.2 62.3L25.3 61.3L26.4 60.9L27.5 60.9L28.6 61.3L29.8 62.2L30.9 63.3L32 64.7L33.1 66L34.2 67.1L35.3 68L36.4 68.5L37.5 68.6L38.6 68.3L39.7 67.6L40.8 66.7L41.9 65.6L43 64.5L44.1 63.5L45.2 62.7L46.4 62.1L47.5 62L48.6 62.2L49.7 62.7L50.8 63.4L51.9 64.3L53 65.2" stroke-width="3"/><path d="M22 75H52" stroke-width="2.2"/><path d="M86 27L64.5 56.2L62 68L72.6 62.1L94 33Z" stroke-width="3"/><path d="M64.5 56.2L72.6 62.1" stroke-width="2.2"/><path d="M81.8 32.7L89.9 38.6" stroke-width="2.2"/>',
  tao: '<rect x="6" y="30" width="32" height="40" rx="1.5" stroke-width="3"/><path d="M6 43.3H38M6 56.6H38" stroke-width="2.2"/><path d="M17 30V70" stroke-width="2.2"/><path d="M43 50H52" stroke-width="3"/><path d="M48 45L53 50L48 55" stroke-width="3"/><path d="M68 20V13H92V69H86" stroke-width="2.2"/><path d="M58 22H80L88 30V86H58Z" stroke-width="3"/><path d="M80 22V30H88" stroke-width="3"/><path d="M64 42H80" stroke-width="2.2"/><path d="M64 51H82" stroke-width="2.2"/><path d="M64 60H74" stroke-width="2.2"/><circle cx="76" cy="74" r="5" stroke-width="2.2"/>',
  kiemtra: '<path d="M12 12H47L58 23V88H12Z" stroke-width="3"/><path d="M47 12V23H58" stroke-width="3"/><path d="M21 28H42" stroke-width="2.2"/><path d="M21 37H46" stroke-width="2.2"/><path d="M21 46H34" stroke-width="2.2"/><path d="M21 66C25 60 29 60 31 64S37 70 41 62" stroke-width="3"/><path d="M75 40L92 46.5V59C92 70.5 84.5 79 75 84C65.5 79 58 70.5 58 59V46.5Z" stroke-width="3"/><path d="M67 61.5L72.5 67L83 56" stroke-width="3"/>',
  vitri: '<path d="M16 8H62L74 20V92H16Z" stroke-width="3"/><path d="M62 8V20H74" stroke-width="3"/><path d="M25 25H50" stroke-width="2.2"/><path d="M25 34H58" stroke-width="2.2"/><path d="M25 43H44" stroke-width="2.2"/><rect x="36" y="57" width="34" height="18" rx="1" stroke-width="2.5" stroke-dasharray="4.5 3.5"/><rect x="33.5" y="54.5" width="5" height="5" stroke-width="2"/><rect x="67.5" y="54.5" width="5" height="5" stroke-width="2"/><rect x="33.5" y="72.5" width="5" height="5" stroke-width="2"/><rect x="67.5" y="72.5" width="5" height="5" stroke-width="2"/><path d="M78 76L78 94L82.5 89.5L86.5 97L90 95.3L86 88L92 88Z" stroke-width="2.4"/>',
} as const;

export type BigIconName = keyof typeof BIG_ICONS;
