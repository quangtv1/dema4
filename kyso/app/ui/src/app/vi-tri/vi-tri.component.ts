import { ChangeDetectionStrategy, Component, ElementRef, computed, effect, inject, signal } from '@angular/core';

import { Anchor, PartBox, SignFileItem, SignPlacement, SignatureMode } from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { IMAGE_LAYOUTS, MODE_LABEL, fmtSignTime } from '../ky-so/ky-so.store';
import { POS_LABEL } from '../shell/config-dialog.component';
import { BigIconComponent, IconComponent } from '../shared/icon.component';
import { clamp } from '../shared/text';
import { TipComponent } from '../shared/tip.component';
import { PageThumbsComponent } from '../ky-so/page-thumbs.component';
import { BoxMenuEvent, PdfViewComponent, SigBoxView } from '../tao-van-ban/pdf-view.component';
import { ViTriStore } from './vi-tri.store';

/** Thứ tự lựa chọn "Vị trí theo mẫu" và "Hiển thị" như prototype (L1394, L1397). */
const ANCHORS: Anchor[] = ['tl', 'tc', 'tr', 'bl', 'bc', 'br'];
const MODES: SignatureMode[] = ['imageAndText', 'image', 'text', 'hidden'];
/** Vạch giả dòng chữ trên ảnh thu nhỏ (prototype `miniTrang`). */
const VACH = [10, 16, 22, 28, 34, 40, 46, 52].map((top, i) => ({ top, right: i % 3 ? 5 : 22 }));

const pct = (v: number) => Math.round(v * 100);
/** Cỡ khối nhỏ nhất (pt) khi kéo cạnh / góc khung ngoài của ô sửa. */
const BLOCK_MIN_PT = { w: 30, h: 12 };
/** Cạnh nhỏ nhất của ô ảnh / ô chữ (tỉ lệ khối, như `PART_BOX_MIN` của lõi). */
const PART_MIN = 0.03;
/** Tay nắm đổi cỡ: 4 góc và 4 cạnh (t trên, b dưới, l trái, r phải); "br" đứng đầu. */
const HANDLES = ['br', 'bl', 'tr', 'tl', 'r', 'l', 'b', 't'] as const;

/** Kéo cạnh / góc `d` một đoạn (dx, dy) cùng đơn vị với hộp; cạnh đối diện đứng yên, không ra ngoài [0, max]. */
function resizeBy(
  b: { x: number; y: number; w: number; h: number },
  d: string,
  dx: number,
  dy: number,
  min: { w: number; h: number },
  max: { x: number; y: number },
): { x: number; y: number; w: number; h: number } {
  let l = b.x;
  let t = b.y;
  let r = b.x + b.w;
  let bt = b.y + b.h;
  if (d.includes('l')) l = clamp(b.x + dx, 0, r - min.w);
  if (d.includes('r')) r = clamp(r + dx, l + min.w, max.x);
  if (d.includes('t')) t = clamp(b.y + dy, 0, bt - min.h);
  if (d.includes('b')) bt = clamp(bt + dy, t + min.h, max.y);
  return { x: l, y: t, w: r - l, h: bt - t };
}

/** Ảnh thu nhỏ của một file: khổ trang và các khung chữ ký trên trang có khung đầu tiên. */
interface Mini {
  w: number;
  h: number;
  boxes: SignPlacement[];
  own: boolean;
  /** Chưa gán chữ ký (đã xoá hết khung): bỏ qua khi ký số. */
  skip: boolean;
  error: string | null;
}

/**
 * Màn "Xem trước & vị trí chữ ký" — prototype `manTC()` L1384–1423, chỉ mở từ nút "Cấu hình vị trí" của màn Ký số
 * (chức năng riêng "Vị trí chữ ký (kéo thả)" ở menu Chức năng và Trang chủ đã bỏ ngày 20/11/2026). Lưới file bên
 * trái, trang xem trước có khung kéo thả bên phải.
 */
@Component({
  selector: 'app-vi-tri',
  imports: [IconComponent, BigIconComponent, TipComponent, PdfViewComponent, PageThumbsComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man vi-tri', '(document:keydown)': 'onKey($event)', '(document:pointerdown)': 'onDocDown($event)' },
  templateUrl: './vi-tri.component.html',
})
export class ViTriComponent {
  protected readonly t = inject(ViTriStore);
  protected readonly ui = inject(UiService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected readonly anchors = ANCHORS;
  protected readonly modes = MODES;
  protected readonly modeLabel = MODE_LABEL;
  protected readonly posLabel = POS_LABEL;
  protected readonly vach = VACH;
  protected readonly pct = pct;
  protected readonly cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
  protected readonly resizing = signal(false);

  protected readonly tipMode =
    'Tất cả file: một khung áp cho mọi văn bản.\nTừng file: chọn văn bản bên trái rồi đặt khung riêng.\n' +
    'Cả hai đều có thể lấy vị trí Theo mẫu ở Nâng cao rồi kéo chỉnh.\nKéo khung để di chuyển, kéo cạnh hoặc góc để đổi cỡ.\n' +
    'Chưa kéo: chữ ký đặt theo mẫu chữ ký theo khổ giấy (hộp Cấu hình).';
  protected readonly tipProfile = 'Lấy vị trí & cỡ chữ ký từ mẫu khổ giấy (lưu trong Cấu hình).\nÁp cho khung hiện tại, sau đó vẫn kéo chỉnh được.';
  protected readonly sizeTitle = computed(() =>
    this.t.textOnly() ? 'Kiểu "Chỉ thông tin": cỡ khối theo cỡ chữ (đổi ở hộp Cấu hình), khung chỉ để đặt chỗ.' : 'Cỡ khung so với cỡ theo mẫu',
  );

  /** Khung trên bản xem trước của file đang xem. */
  protected readonly sigBoxes = computed<SigBoxView[]>(() => {
    const boxes = this.t.curBoxes();
    if (!boxes.length) {
      return [];
    }
    const ks = this.t.ks;
    const a = ks.appearance();
    const mode = this.t.boxMode();
    const lines = [`Ký bởi: ${ks.signerName() ?? '[Tên chứng thư số]'}`, `Ngày ký: ${fmtSignTime(new Date(ks.now()))}`];
    return boxes.map((placement) => ({
      placement,
      layout: this.t.imageLayout(),
      imageShare: this.t.imageShare(),
      imageGap: this.t.imageGap(),
      imageBox: this.t.freeBoxes()?.image ?? null,
      textBox: this.t.freeBoxes()?.text ?? null,
      mode,
      imageUrl: mode === 'text' ? null : ks.imageUrl(),
      lines,
      fontPt: a.fontSize,
      fontFamily: a.fontFamily,
      color: a.textColor,
      editable: !this.t.locked(),
    }));
  });

  /** Ảnh thu nhỏ của từng file trong lưới bên trái. */
  protected readonly minis = computed<Record<string, Mini>>(() => {
    const out: Record<string, Mini> = {};
    const layouts = this.t.layouts();
    for (const f of this.t.files()) {
      const l = layouts[f.path]?.layout;
      const ratio = l && l.pageWidth > 0 ? l.pageHeight / l.pageWidth : 297 / 210;
      const w = ratio >= 1 ? 98 : 124;
      const h = clamp(Math.round(w * ratio), 56, 138);
      const page = l?.boxes[0]?.page;
      out[f.path] = {
        w,
        h,
        boxes: l && !this.t.skips().has(f.path) ? l.boxes.filter((b) => b.page === page) : [],
        own: this.t.isOwn(f.path),
        skip: this.t.skips().has(f.path),
        error: f.unreadable ? 'Không đọc được file này' : (layouts[f.path]?.error ?? null),
      };
    }
    return out;
  });

  /** Chân màn: toạ độ khung đang xem (prototype L1410). */
  protected readonly footText = computed(() => {
    const box = this.t.curBox();
    if (this.t.curSkipped()) {
      return 'Chưa gán chữ ký · văn bản này bỏ qua khi ký số';
    }
    if (!box) {
      return '';
    }
    const where = this.t.curManual() ? (this.t.hasOwnFrame(this.t.current()?.path ?? '') ? 'Riêng file này' : 'Tự đặt') : 'Theo mẫu khổ giấy';
    const path = this.t.current()?.path ?? '';
    const ownDisp = this.t.mode() === 'per' && !!this.t.disp()[path];
    const n = this.t.curBoxes().length;
    const pages = n > 1 ? `${n} khung · trang ${[...this.t.boxPages()].sort((a, b) => a - b).join(', ')}` : `trang ${box.page}`;
    return `${where} · ${pages}${ownDisp ? ` · hiển thị riêng: ${MODE_LABEL[this.t.curDisplay()]}` : ''}`;
  });

  constructor() {
    this.t.enter();
    // Giữ ô file đang chọn trong tầm nhìn.
    effect(() => {
      this.t.selectedPath();
      setTimeout(() => this.host.nativeElement.querySelector('.ab-file.chon')?.scrollIntoView({ block: 'nearest' }));
    });
  }

  protected fileTitle(f: SignFileItem): string {
    const m = this.minis()[f.path];
    const lines = [f.name];
    if (m?.skip) {
      lines.push('Chưa gán chữ ký: bỏ qua khi ký số');
    } else if (m?.own) {
      lines.push('Có vị trí riêng');
    }
    if (m?.error) {
      lines.push(m.error);
    }
    return lines.join('\n');
  }

  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && this.menu()) {
      this.menu.set(null);
      return;
    }
    if (!event.ctrlKey || event.shiftKey || event.altKey || event.metaKey || this.ui.dialog()) {
      return;
    }
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    const target = event.target;
    if (!step || (target instanceof HTMLElement && target.closest('input:not([type=range]),select,textarea'))) {
      return;
    }
    event.preventDefault();
    this.t.nav(step);
  }

  protected onProfile(event: Event): void {
    const id = (event.target as HTMLSelectElement).value;
    void this.t.applyProfile(id, null);
  }

  protected onAnchor(event: Event): void {
    void this.t.applyProfile(this.t.profile()?.id ?? null, (event.target as HTMLSelectElement).value as Anchor);
  }

  protected onDisplay(event: Event): void {
    this.t.setDisplay((event.target as HTMLSelectElement).value as SignatureMode);
  }

  protected onMode(event: Event): void {
    this.t.setMode((event.target as HTMLSelectElement).value === 'per' ? 'per' : 'all');
  }

  protected onSize(event: Event): void {
    this.t.setSize(Number((event.target as HTMLInputElement).value));
  }

  // ---- Ảnh so với chữ (TD-51) ---------------------------------------------------------------

  protected readonly imageLayouts = IMAGE_LAYOUTS;
  private readonly stacked = computed(() => this.t.imageLayout() === 'top' || this.t.imageLayout() === 'bottom');
  /** Chỗ của thanh khi đang "Tự động": gần đúng phần lõi tự chia (ảnh 2:1 trong khung mặc định). */
  protected readonly shareShown = computed(() => this.t.imageShare() ?? (this.stacked() ? 55 : 35));
  protected readonly gapShown = computed(() => this.t.imageGap() ?? (this.stacked() ? 4 : 3));
  protected readonly tipSplit =
    'Ảnh chiếm: phần khung dành cho ảnh (trái / phải: theo bề ngang, trên / dưới: theo chiều cao).\n' +
    'Khoảng cách: giữa ảnh và thông tin, theo % khung.\nTự động: chia như mặc định. Áp cho mọi văn bản.';

  // ---- Ô sửa phóng to của kiểu "Tự do" (TD-52) ----------------------------------------------

  /** Cỡ khối (pt) của file đang xem; chưa có thì khối mặc định 170 × 30 pt. */
  private readonly boxPt = computed(() => {
    const l = this.t.curLayout();
    const b = this.t.curBox();
    return l && b ? { w: b.widthRatio * l.pageWidth, h: b.heightRatio * l.pageHeight } : { w: 170, h: 30 };
  });
  protected readonly handles = HANDLES;
  /** Dời ô sửa khi kéo cạnh trái / trên của khung ngoài để cạnh đối diện đứng yên, khung đi theo chuột. */
  protected readonly edShift = signal<string | null>(null);
  /** Tỉ lệ px / pt giữ nguyên khi đang kéo góc khung ngoài (khung đi theo chuột), thả ra thì vừa lại ô sửa. */
  private readonly frozenScale = signal<number | null>(null);
  /** Ô sửa: rộng tối đa 480 px, cao 60–240 px, đúng tỉ lệ khối. */
  protected readonly editor = computed(() => {
    const { w, h } = this.boxPt();
    const fixed = this.frozenScale();
    if (fixed) {
      return { w: Math.round(w * fixed), h: Math.round(h * fixed), scale: fixed };
    }
    const ratio = h > 0 && w > 0 ? h / w : 30 / 170;
    let ew = 480;
    let eh = ew * ratio;
    if (eh > 240) {
      eh = 240;
      ew = eh / ratio;
    } else if (eh < 60) {
      eh = 60;
      ew = Math.min(720, eh / ratio);
    }
    return { w: Math.round(ew), h: Math.round(eh), scale: ew / Math.max(w, 1) };
  });
  protected readonly editorLines = computed(() => [
    `Ký bởi: ${this.t.ks.signerName() ?? '[Tên chứng thư số]'}`,
    `Ngày ký: ${fmtSignTime(new Date(this.t.ks.now()))}`,
  ]);
  /** Cỡ chữ (px) trong ô chữ của ô sửa: theo cỡ chữ thật, thu cho vừa ô như lõi. */
  protected readonly editorFont = computed(() => {
    const boxes = this.t.freeBoxes();
    const a = this.t.ks.appearance();
    const ed = this.editor();
    const base = a.fontSize * ed.scale;
    if (!boxes) {
      return base;
    }
    const ctx = document.createElement('canvas').getContext('2d');
    if (!ctx) {
      return base;
    }
    ctx.font = `${base}px '${a.fontFamily}'`;
    const textW = Math.max(...this.editorLines().map((l) => ctx.measureText(l).width), 1);
    const textH = this.editorLines().length * base * 1.15;
    const f = Math.min(1, (boxes.text.w * ed.w) / textW, (boxes.text.h * ed.h) / textH);
    return base * f;
  });

  /** Ô vừa bấm nằm trên ô kia (hai ô chồng nhau vẫn chọn lại được). */
  protected readonly topPart = signal<'image' | 'text'>('text');

  /** Kéo ô ảnh / ô chữ để dời, kéo góc dưới phải để đổi cỡ. */
  protected dragPart(part: 'image' | 'text', event: PointerEvent): void {
    const start = this.t.freeBoxes()?.[part];
    if (!start || event.button !== 0 || this.t.locked()) {
      return;
    }
    this.topPart.set(part);
    event.preventDefault();
    event.stopPropagation();
    const d = (event.target as HTMLElement).dataset['d'] ?? '';
    const { w: ew, h: eh } = this.editor();
    const x0 = event.clientX;
    const y0 = event.clientY;
    const move = (e: PointerEvent) => {
      const dx = (e.clientX - x0) / ew;
      const dy = (e.clientY - y0) / eh;
      const next: PartBox = d
        ? resizeBy(start, d, dx, dy, { w: PART_MIN, h: PART_MIN }, { x: 1, y: 1 })
        : { ...start, x: start.x + dx, y: start.y + dy };
      this.t.setFreeBox(part, next);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  /** Kéo cạnh / góc khung ngoài của ô sửa: đổi cỡ cả khối chữ ký trên trang, cạnh đối diện đứng yên; ô ảnh / ô chữ giãn theo. */
  protected dragBlock(event: PointerEvent): void {
    const boxes = this.t.curBoxes();
    const l = this.t.curLayout();
    if (!boxes.length || !l || event.button !== 0 || this.t.locked() || this.t.textOnly()) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const d = (event.target as HTMLElement).dataset['d'] ?? 'br';
    const scale = this.editor().scale;
    this.frozenScale.set(scale);
    const x0 = event.clientX;
    const y0 = event.clientY;
    const starts = boxes.map((b) => ({ x: b.xRatio, y: b.yRatio, w: b.widthRatio, h: b.heightRatio }));
    const min = { w: BLOCK_MIN_PT.w / l.pageWidth, h: BLOCK_MIN_PT.h / l.pageHeight };
    const move = (e: PointerEvent) => {
      const dx = (e.clientX - x0) / scale / l.pageWidth;
      const dy = (e.clientY - y0) / scale / l.pageHeight;
      // Mọi khung của văn bản (nhiều trang) đổi cỡ cùng một đoạn; ô sửa vẽ theo khung đầu tiên.
      const next = starts.map((s) => resizeBy(s, d, dx, dy, min, { x: 1, y: 1 }));
      const sx = (next[0].x - starts[0].x) * l.pageWidth * scale;
      const sy = (next[0].y - starts[0].y) * l.pageHeight * scale;
      this.edShift.set(sx || sy ? `translate(${sx}px, ${sy}px)` : null);
      this.t.setBoxes(boxes.map((b, i) => ({ ...b, xRatio: next[i].x, yRatio: next[i].y, widthRatio: next[i].w, heightRatio: next[i].h })));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      this.frozenScale.set(null);
      this.edShift.set(null);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  // ---- Nhiều khung: menu chuột phải trên trang và trên ảnh thu nhỏ (TD-55) ---------------------

  /** Menu đang mở: chuột phải trên trang (`index`: khung bị bấm) hoặc trên ảnh thu nhỏ (`thumb`). */
  protected readonly menu = signal<(BoxMenuEvent & { thumb: boolean; left: number; top: number }) | null>(null);
  /** Nhắc ngắn phía trên trang (xoá khung cuối cùng…). */
  protected readonly hint = signal<string | null>(null);
  private hintTimer: ReturnType<typeof setTimeout> | null = null;

  protected openBoxMenu(e: BoxMenuEvent): void {
    this.showMenu({ ...e, thumb: false });
  }

  protected openPageMenu(e: { page: number; clientX: number; clientY: number }): void {
    this.showMenu({ index: null, page: e.page, xRatio: 0.5, yRatio: 0.5, clientX: e.clientX, clientY: e.clientY, thumb: true });
  }

  private showMenu(e: BoxMenuEvent & { thumb: boolean }): void {
    if (this.t.locked() || (!this.t.curBoxes().length && !this.t.curSkipped())) {
      this.menu.set(null);
      return;
    }
    // Menu rộng khoảng 260 px, cao khoảng 150 px: đặt cạnh con trỏ, không tràn khỏi cửa sổ.
    const left = Math.max(8, Math.min(e.clientX, window.innerWidth - 268));
    const top = Math.max(8, Math.min(e.clientY, window.innerHeight - 160));
    this.menu.set({ ...e, left, top });
  }

  protected onDocDown(event: PointerEvent): void {
    const target = event.target instanceof Element ? event.target : null;
    if (this.menu() && !target?.closest('.ky-menu')) {
      this.menu.set(null);
    }
  }

  protected menuAdd(): void {
    const m = this.menu();
    this.menu.set(null);
    if (m?.thumb) {
      this.t.addBoxToPage(m.page);
    } else if (m) {
      this.t.addBox(m.page, m.xRatio, m.yRatio);
    }
  }

  protected menuDelete(): void {
    const m = this.menu();
    this.menu.set(null);
    if (!m) {
      return;
    }
    if (m.index !== null) {
      this.t.deleteBox(m.index);
    } else {
      this.t.deletePage(m.page);
    }
    this.skipHint();
  }

  protected menuDeleteAll(): void {
    this.menu.set(null);
    this.t.deleteAll();
    this.skipHint();
  }

  /** Vừa xoá hết khung: nhắc văn bản chưa gán chữ ký. */
  private skipHint(): void {
    if (this.t.curSkipped()) {
      this.showHint('Đã xoá hết khung: văn bản chưa gán chữ ký, sẽ bỏ qua khi ký số. Chuột phải trên trang để thêm khung lại.');
    }
  }

  protected menuCopyAll(): void {
    const m = this.menu();
    this.menu.set(null);
    if (m) {
      this.t.copyBoxToAllPages(m.index ?? Math.max(0, this.t.curBoxes().findIndex((b) => b.page === m.page)));
    }
  }

  /** Phím Delete trên khung đang chọn. */
  protected deleteBox(index: number): void {
    this.t.deleteBox(index);
    this.skipHint();
  }

  private showHint(text: string): void {
    this.hint.set(text);
    if (this.hintTimer) {
      clearTimeout(this.hintTimer);
    }
    this.hintTimer = setTimeout(() => this.hint.set(null), 4000);
  }

  protected onSplit(key: 'imageShare' | 'imageGap', event: Event): void {
    this.t.setImageSplit(key, Number((event.target as HTMLInputElement).value));
  }

  protected modeDisabled(mode: SignatureMode): boolean {
    return (mode === 'image' || mode === 'imageAndText') && !this.t.ks.hasImage();
  }

  // ---- Kéo đổi độ rộng cột trái -------------------------------------------------------------

  protected startResize(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    const parent = (event.currentTarget as HTMLElement).parentElement;
    const startX = event.clientX;
    const orig = this.t.leftW();
    this.resizing.set(true);
    document.body.style.cursor = 'col-resize';
    const move = (e: PointerEvent) => {
      const maxW = parent ? Math.max(320, parent.clientWidth - 260) : 560;
      this.t.leftW.set(clamp(orig + (e.clientX - startX), 180, maxW));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      document.body.style.cursor = '';
      this.resizing.set(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }
}
