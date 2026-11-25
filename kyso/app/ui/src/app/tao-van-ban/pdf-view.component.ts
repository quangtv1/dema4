import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { PDFDocumentLoadingTask, PDFDocumentProxy, RenderTask, TextLayer } from 'pdfjs-dist';

import { ImageLayout, PartBox, PreviewResult, SignPlacement } from '../core/ipc.service';
import { IconComponent } from '../shared/icon.component';

type PdfJs = typeof import('pdfjs-dist');

/** Thư mục chứa worker, wasm, cmaps, phông chuẩn, ICC của pdf.js; chép vào gói qua `assets` trong angular.json. */
const PDFJS_DIR = 'pdfjs/';
/** Ở zoom 100%, trang A4 (595,28 pt) rộng 520 px như trang giấy của prototype (`.giay-in` L199). */
const BASE_SCALE = 520 / 595.28;

let pdfjsPromise: Promise<PdfJs> | null = null;

/** Nạp pdf.js lần đầu khi cần (tách gói riêng, không nằm trong gói khởi động). */
function loadPdfJs(): Promise<PdfJs> {
  pdfjsPromise ??= import('pdfjs-dist').then((pdfjs) => {
    pdfjs.GlobalWorkerOptions.workerSrc = assetUrl('pdf.worker.min.mjs');
    return pdfjs;
  });
  return pdfjsPromise;
}

function assetUrl(path: string): string {
  return new URL(PDFJS_DIR + path, document.baseURI).href;
}

/** Khung chữ ký vẽ trên bản xem trước (màn Ký số). Toạ độ theo tỉ lệ trang, gốc góc trên trái, như lõi. */
export interface SigBoxView {
  placement: SignPlacement;
  mode: 'text' | 'imageAndText' | 'image';
  /** Ảnh nằm ở đâu so với chữ (kiểu "Ảnh + thông tin"); mặc định bên trái. */
  layout?: ImageLayout;
  /** Phần khối dành cho ảnh và khoảng cách ảnh – chữ (% khối theo chiều xếp); `null` = tự động như lõi (TD-51). */
  imageShare?: number | null;
  imageGap?: number | null;
  /** Kiểu "Tự do" (TD-52): ô ảnh và ô chữ theo tỉ lệ khối; có cả hai thì bỏ qua `layout`, `imageShare`, `imageGap`. */
  imageBox?: PartBox | null;
  textBox?: PartBox | null;
  imageUrl: string | null;
  lines: string[];
  /** Cỡ chữ (pt) trên trang A4; trang khổ khác nhân theo tỉ lệ khổ như lõi. */
  fontPt: number;
  fontFamily?: string;
  color?: string;
  /** Kéo để đổi chỗ, kéo cạnh hoặc góc để đổi cỡ. */
  editable: boolean;
}

/** Khung đánh dấu chữ ký đã có trong file (màn Kiểm tra): viền màu theo kết luận, nhãn số thứ tự. */
export interface MarkBoxView {
  placement: SignPlacement;
  label: string;
  tone: 'ok' | 'warn' | 'fail';
  selected: boolean;
  title: string;
}

/** Chuột phải trên trang (màn Ký số mở menu): khung bị bấm (nếu có), trang và chỗ bấm theo tỉ lệ trang. */
export interface BoxMenuEvent {
  /** Chỉ số khung trong `sigBoxes`; null: bấm vào chỗ trống của trang. */
  index: number | null;
  page: number;
  xRatio: number;
  yRatio: number;
  clientX: number;
  clientY: number;
}

/** Bề rộng ảnh thu nhỏ trang (px CSS). */
export const THUMB_WIDTH = 84;

/** Cạnh nhỏ nhất của khung khi kéo đổi cỡ (px). */
const MIN_BOX_PX = 16;
/** Tay nắm đổi cỡ khung chữ ký: 4 góc và 4 cạnh (kéo ngang / dọc riêng một chiều). */
const CORNERS = ['tl', 'tr', 'bl', 'br', 't', 'b', 'l', 'r'] as const;

function isCancelled(error: unknown): boolean {
  return !!error && typeof error === 'object' && (error as { name?: string }).name === 'RenderingCancelledException';
}

/** Giải phóng bộ nhớ canvas trước khi bỏ khỏi DOM (WebView2 giữ bộ đệm điểm ảnh tới khi GC). */
function releaseCanvases(root: Element): void {
  root.querySelectorAll('canvas').forEach((c) => {
    c.width = 0;
    c.height = 0;
  });
}

/**
 * Vùng xem trước PDF thật bằng pdf.js: đọc file qua asset protocol (`convertFileSrc`), vẽ mọi trang lên canvas.
 * Đổi file thì huỷ lượt vẽ cũ, huỷ tài liệu cũ và giải phóng canvas. Đổi zoom thì vẽ lại từ tài liệu đã nạp.
 * Nội dung chiếu (`ng-content`) hiện phía trên các trang: trạng thái "đang tạo", cảnh báo, lỗi.
 * Bật `selectable` thì mỗi trang có thêm lớp chữ trong suốt của pdf.js nằm trên canvas để bôi đen và sao chép.
 */
@Component({
  selector: 'div[pdfView]',
  exportAs: 'pdfView',
  imports: [IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    class: 'vungxem',
    '(wheel)': 'onWheel($event)',
    '(pointerdown)': 'cancelFocus()',
    '(copy)': 'onCopy($event)',
    '(window:pointerup)': 'endSelecting()',
    '(scroll)': 'trackPage()',
    '(contextmenu)': 'onContextMenu($event)',
    '(document:keydown)': 'onKey($event)',
  },
  template: `
    <ng-content />
    @if (error(); as message) {
      <div class="bao loi">
        <svg icon="alert"></svg>
        <span><b>Không hiển thị được bản xem trước</b>{{ message }}</span>
      </div>
    }
    <div #pages class="trang-ds"></div>
  `,
})
export class PdfViewComponent {
  readonly preview = input<PreviewResult | null>(null);
  readonly zoom = input<number>(80);
  /** Các khung chữ ký (ký nháy: mỗi trang một khung). Rỗng: không vẽ. */
  readonly sigBoxes = input<SigBoxView[]>([]);
  /** Cho bôi đen và sao chép chữ trên bản xem trước (màn Tạo văn bản). */
  readonly selectable = input(false);
  /** Vị trí mới sau khi người dùng kéo / đổi cỡ khung chữ ký. */
  readonly sigBoxChange = output<SignPlacement>();
  /** Như `sigBoxChange`, kèm chỉ số khung trong `sigBoxes` (màn có nhiều khung). */
  readonly sigBoxEdit = output<{ index: number; placement: SignPlacement }>();
  /** Bật menu chuột phải, chọn khung và phím Delete trên khung chữ ký (màn Ký số). */
  readonly boxTools = input(false);
  /** Chuột phải trên trang. */
  readonly boxMenu = output<BoxMenuEvent>();
  /** Phím Delete / Backspace khi đang chọn một khung. */
  readonly sigBoxDelete = output<number>();
  /** Giữ Ctrl + lăn chuột: +10 phóng to, -10 thu nhỏ (màn chứa đổi `zoom`). */
  readonly zoomStep = output<number>();
  /** Khung đánh dấu chữ ký đã có (chỉ hiển thị, không kéo được). */
  readonly marks = input<MarkBoxView[]>([]);
  /** Bấm vào một khung đánh dấu: chỉ số trong `marks`. */
  readonly markClick = output<number>();

  protected readonly error = signal<string | null>(null);
  /** Số trang của văn bản đang hiện (0: chưa có). */
  readonly pageCount = signal(0);
  /** Trang đang ở giữa vùng xem (từ 1). */
  readonly currentPage = signal(1);
  /** Tăng mỗi lần nạp văn bản mới, để ảnh thu nhỏ vẽ lại. */
  readonly docVersion = signal(0);
  /** Khung đang chọn (chỉ số trong `sigBoxes`); Delete xoá khung này. */
  readonly selectedBox = signal<number | null>(null);
  /** Không dùng `required`: effect đầu tiên có thể chạy trước khi khung nhìn được tạo. */
  private readonly pages = viewChild<ElementRef<HTMLDivElement>>('pages');

  private doc: PDFDocumentProxy | null = null;
  private loading: PDFDocumentLoadingTask | null = null;
  private tasks: RenderTask[] = [];
  private textLayers: TextLayer[] = [];
  private loadGen = 0;
  private renderGen = 0;
  private boxes: HTMLDivElement[] = [];
  private dragging = false;
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  /** Văn bản vừa mở chưa cuộn tới khung chữ ký (prototype `S._cuon`). */
  private focusPending = false;
  /** Các trang đang hiện là của văn bản vừa mở (chưa vẽ xong thì trang cũ còn trên màn). */
  private pagesFresh = false;
  /** Điểm dưới con trỏ khi Ctrl + lăn (tỉ lệ trong nội dung + chỗ trong vùng xem) để giữ nguyên sau khi vẽ lại. */
  private zoomAnchor: { fx: number; fy: number; ox: number; oy: number; at: number } | null = null;
  private wheelAcc = 0;
  private wheelAt = 0;

  constructor() {
    effect(() => {
      const preview = this.preview();
      untracked(() => void this.load(preview));
    });
    effect(() => {
      this.zoom();
      untracked(() => void this.renderAll());
    });
    effect(() => {
      this.marks();
      untracked(() => this.placeMarks());
    });
    effect(() => {
      const n = this.sigBoxes().length;
      untracked(() => {
        const sel = this.selectedBox();
        if (sel !== null && sel >= n) {
          this.selectedBox.set(null);
        }
        this.placeBoxes();
      });
    });
    inject(DestroyRef).onDestroy(() => {
      this.loadGen++;
      this.renderGen++;
      this.cancelTasks();
      void this.dropDocument();
      const host = this.pages()?.nativeElement;
      if (host) {
        releaseCanvases(host);
      }
    });
  }

  private async load(preview: PreviewResult | null): Promise<void> {
    const gen = ++this.loadGen;
    this.renderGen++;
    this.focusPending = !!preview;
    this.pagesFresh = false;
    this.zoomAnchor = null;
    this.cancelTasks();
    await this.dropDocument();
    this.error.set(null);
    this.selectedBox.set(null);
    if (!preview) {
      this.pageCount.set(0);
      this.docVersion.update((v) => v + 1);
      this.clearPages();
      return;
    }
    try {
      const pdfjs = await loadPdfJs();
      const response = await fetch(convertFileSrc(preview.pdfPath), { cache: 'no-store' });
      if (!response.ok) {
        throw new Error(`Không đọc được file xem trước (HTTP ${response.status}).`);
      }
      const data = new Uint8Array(await response.arrayBuffer());
      if (gen !== this.loadGen) {
        return;
      }
      this.loading = pdfjs.getDocument({
        data,
        wasmUrl: assetUrl('wasm/'),
        cMapUrl: assetUrl('cmaps/'),
        cMapPacked: true,
        standardFontDataUrl: assetUrl('standard_fonts/'),
        iccUrl: assetUrl('iccs/'),
        enableXfa: false,
      });
      const doc = await this.loading.promise;
      this.loading = null;
      if (gen !== this.loadGen) {
        void doc.loadingTask.destroy();
        return;
      }
      this.doc = doc;
      this.pageCount.set(doc.numPages);
      this.currentPage.set(1);
      this.docVersion.update((v) => v + 1);
      await this.renderAll();
    } catch (error) {
      if (gen === this.loadGen) {
        this.clearPages();
        this.error.set(error instanceof Error ? error.message : String(error));
      }
    }
  }

  /** Vẽ mọi trang vào các canvas mới ngoài DOM rồi thay một lần, để đổi zoom không nháy trắng. */
  private async renderAll(): Promise<void> {
    const doc = this.doc;
    if (!doc) {
      return;
    }
    const gen = ++this.renderGen;
    this.cancelTasks();
    const pdfjs = this.selectable() ? await loadPdfJs() : null;
    if (gen !== this.renderGen) {
      return;
    }
    const scale = (this.zoom() / 100) * BASE_SCALE;
    const ratio = window.devicePixelRatio || 1;
    const sheets: HTMLDivElement[] = [];
    try {
      for (let n = 1; n <= doc.numPages; n++) {
        const page = await doc.getPage(n);
        if (gen !== this.renderGen) {
          releaseSheets(sheets);
          return;
        }
        const viewport = page.getViewport({ scale });
        const canvas = document.createElement('canvas');
        canvas.width = Math.floor(viewport.width * ratio);
        canvas.height = Math.floor(viewport.height * ratio);
        canvas.style.width = Math.floor(viewport.width) + 'px';
        canvas.style.height = Math.floor(viewport.height) + 'px';
        const sheet = document.createElement('div');
        sheet.className = 'giay';
        // Số điểm ảnh mỗi pt và tỉ lệ khổ so với A4 (lõi: min(rộng/595, cao/842)) để khung chữ ký vẽ đúng cỡ chữ.
        const wPt = viewport.width / scale;
        const hPt = viewport.height / scale;
        sheet.dataset['pxPerPt'] = String(scale);
        sheet.dataset['pageScale'] = String(Math.min(wPt / 595, hPt / 842));
        sheet.appendChild(canvas);
        sheets.push(sheet);
        const task = page.render({
          canvas,
          viewport,
          transform: ratio !== 1 ? [ratio, 0, 0, ratio, 0, 0] : undefined,
        });
        this.tasks.push(task);
        await task.promise;
        if (pdfjs) {
          await this.addTextLayer(pdfjs, page, viewport, sheet, scale);
        }
      }
    } catch (error) {
      releaseSheets(sheets);
      if (!isCancelled(error) && gen === this.renderGen) {
        this.error.set(error instanceof Error ? error.message : String(error));
      }
      return;
    }
    if (gen !== this.renderGen) {
      releaseSheets(sheets);
      return;
    }
    this.tasks = [];
    this.textLayers = [];
    const host = this.pages()?.nativeElement;
    if (!host) {
      releaseSheets(sheets);
      return;
    }
    releaseCanvases(host);
    host.replaceChildren(...sheets);
    this.pagesFresh = true;
    this.boxes = [];
    this.placeBoxes();
    this.placeMarks();
    this.restoreZoomAnchor();
    this.trackPage();
  }

  // ---- Lớp chữ (bôi đen, sao chép) ------------------------------------------------------------------

  /**
   * Thêm lớp chữ của pdf.js lên trang: các `span` trong suốt đặt đúng chỗ chữ trên canvas. Kích thước theo biến
   * CSS `--total-scale-factor` (pdf.js tính bằng `calc`), nên phải đặt trên trang trước khi vẽ. Lớp chữ lỗi thì
   * trang vẫn hiện bình thường, chỉ không bôi đen được.
   */
  private async addTextLayer(
    pdfjs: PdfJs,
    page: Awaited<ReturnType<PDFDocumentProxy['getPage']>>,
    viewport: ReturnType<typeof page.getViewport>,
    sheet: HTMLDivElement,
    scale: number,
  ): Promise<void> {
    const layer = document.createElement('div');
    layer.className = 'textLayer';
    sheet.style.setProperty('--total-scale-factor', String(scale));
    sheet.appendChild(layer);
    const textLayer = new pdfjs.TextLayer({
      textContentSource: page.streamTextContent(),
      container: layer,
      viewport,
    });
    this.textLayers.push(textLayer);
    try {
      await textLayer.render();
    } catch {
      layer.remove();
      return;
    }
    // Như trình xem của pdf.js: khi đang kéo chọn, một lớp phủ kín trang để vùng chọn không nhảy khi con trỏ
    // đi qua chỗ trống giữa các dòng.
    const end = document.createElement('div');
    end.className = 'endOfContent';
    layer.appendChild(end);
    layer.addEventListener('mousedown', () => layer.classList.add('selecting'));
  }

  protected endSelecting(): void {
    if (!this.selectable()) {
      return;
    }
    this.pages()
      ?.nativeElement.querySelectorAll('.textLayer.selecting')
      .forEach((layer) => layer.classList.remove('selecting'));
  }

  /** Ctrl+C: đưa chữ đang bôi đen vào bộ nhớ tạm ở dạng dựng sẵn (NFC), bỏ ký tự rỗng pdf.js có thể để lại. */
  protected onCopy(event: ClipboardEvent): void {
    if (!this.selectable() || !event.clipboardData) {
      return;
    }
    const text = document.getSelection()?.toString() ?? '';
    if (!text) {
      return;
    }
    event.clipboardData.setData('text/plain', text.replace(/\u0000/g, '').normalize('NFC'));
    event.preventDefault();
  }

  private clearPages(): void {
    const host = this.pages()?.nativeElement;
    this.boxes = [];
    if (host) {
      releaseCanvases(host);
      host.replaceChildren();
    }
  }

  // ---- Khung chữ ký ------------------------------------------------------------------------------

  private sheets(): HTMLElement[] {
    const host = this.pages()?.nativeElement;
    return host ? (Array.from(host.children) as HTMLElement[]) : [];
  }

  /** Vẽ lại các khung chữ ký theo `sigBoxes`; trang 0 hoặc vượt số trang thì đặt ở trang cuối như lõi. */
  private placeBoxes(): void {
    if (this.dragging) {
      return;
    }
    for (const b of this.boxes) {
      b.remove();
    }
    this.boxes = [];
    const sheets = this.sheets();
    if (sheets.length === 0) {
      return;
    }
    this.sigBoxes().forEach((view, index) => {
      const p = view.placement;
      const n = p.page <= 0 ? sheets.length : Math.min(Math.round(p.page), sheets.length);
      const sheet = sheets[Math.max(n, 1) - 1];
      const box = this.buildBox(view, sheet, index);
      sheet.appendChild(box);
      fitText(box);
      this.boxes.push(box);
    });
    if (this.focusPending && this.pagesFresh && this.boxes.length > 0) {
      this.focusPending = false;
      this.focusBox();
    }
  }

  private markEls: HTMLElement[] = [];

  /** Vẽ lại các khung đánh dấu theo `marks`. */
  private placeMarks(): void {
    for (const m of this.markEls) {
      m.remove();
    }
    this.markEls = [];
    const sheets = this.sheets();
    if (sheets.length === 0) {
      return;
    }
    this.marks().forEach((mark, index) => {
      const p = mark.placement;
      const n = p.page <= 0 ? sheets.length : Math.min(Math.round(p.page), sheets.length);
      const sheet = sheets[Math.max(n, 1) - 1];
      const el = document.createElement('div');
      el.className = `ky-dau ${mark.tone}`;
      el.classList.toggle('noi', mark.selected);
      el.dataset['i'] = String(index);
      el.style.left = `${p.xRatio * 100}%`;
      el.style.top = `${p.yRatio * 100}%`;
      el.style.width = `${p.widthRatio * 100}%`;
      el.style.height = `${p.heightRatio * 100}%`;
      el.title = mark.title;
      const tag = document.createElement('span');
      tag.className = 'ky-dau-so';
      tag.textContent = mark.label;
      el.appendChild(tag);
      el.addEventListener('click', () => this.markClick.emit(index));
      sheet.appendChild(el);
      this.markEls.push(el);
    });
  }

  /** Cuộn để khung đánh dấu `index` nằm giữa vùng xem. */
  focusMark(index: number): void {
    const el = this.markEls[index];
    if (!el) {
      return;
    }
    this.focusPending = false;
    const view = this.host.nativeElement;
    const vr = view.getBoundingClientRect();
    const br = el.getBoundingClientRect();
    view.scrollTop += br.top - vr.top - view.clientHeight / 2 + br.height / 2;
    view.scrollLeft += br.left - vr.left - view.clientWidth / 2 + br.width / 2;
  }

  /** Người dùng tự cuộn hoặc bấm vào vùng xem trước khi khung chữ ký kịp hiện: không cuộn nữa. */
  protected cancelFocus(): void {
    this.focusPending = false;
  }

  /**
   * Lăn chuột: cuộn như thường. Giữ Ctrl: phóng to / thu nhỏ một nấc mỗi khấc chuột (bàn di chuột gửi nhiều sự
   * kiện nhỏ thì cộng dồn), giữ nguyên chỗ dưới con trỏ.
   */
  protected onWheel(e: WheelEvent): void {
    this.cancelFocus();
    if (!e.ctrlKey) {
      return;
    }
    e.preventDefault();
    const now = performance.now();
    if (now - this.wheelAt > 300) {
      this.wheelAcc = 0;
    }
    this.wheelAt = now;
    this.wheelAcc += e.deltaMode === 1 ? e.deltaY * 40 : e.deltaMode === 2 ? e.deltaY * 400 : e.deltaY;
    if (Math.abs(this.wheelAcc) < 50) {
      return;
    }
    const step = this.wheelAcc < 0 ? 10 : -10;
    this.wheelAcc = 0;
    const view = this.host.nativeElement;
    const r = view.getBoundingClientRect();
    const ox = e.clientX - r.left;
    const oy = e.clientY - r.top;
    this.zoomAnchor = {
      fx: (view.scrollLeft + ox) / Math.max(view.scrollWidth, 1),
      fy: (view.scrollTop + oy) / Math.max(view.scrollHeight, 1),
      ox,
      oy,
      at: now,
    };
    this.zoomStep.emit(step);
  }

  /** Sau khi vẽ lại theo zoom mới: cuộn để điểm đã ghi lại nằm đúng dưới con trỏ (chỉ ngay sau Ctrl + lăn). */
  private restoreZoomAnchor(): void {
    const a = this.zoomAnchor;
    this.zoomAnchor = null;
    if (!a || performance.now() - a.at > 2000) {
      return;
    }
    const view = this.host.nativeElement;
    view.scrollLeft = a.fx * view.scrollWidth - a.ox;
    view.scrollTop = a.fy * view.scrollHeight - a.oy;
  }

  /** Cuộn để khung chữ ký đầu tiên (trang nhỏ nhất) nằm giữa vùng xem, như prototype khi chọn / chuyển file. */
  private focusBox(): void {
    const view = this.host.nativeElement;
    const sheets = this.sheets();
    const at = (b: HTMLElement) => sheets.indexOf(b.parentElement as HTMLElement);
    const box = this.boxes.reduce((a, b) => (at(b) < at(a) ? b : a));
    const vr = view.getBoundingClientRect();
    const br = box.getBoundingClientRect();
    view.scrollTop += br.top - vr.top - view.clientHeight / 2 + br.height / 2;
    view.scrollLeft += br.left - vr.left - view.clientWidth / 2 + br.width / 2;
  }

  private buildBox(view: SigBoxView, sheet: HTMLElement, index: number): HTMLDivElement {
    const p = view.placement;
    const box = document.createElement('div');
    box.className = 'chuky-o';
    box.dataset['i'] = String(index);
    box.classList.toggle('chon', this.boxTools() && this.selectedBox() === index);
    box.style.left = `${p.xRatio * 100}%`;
    box.style.top = `${p.yRatio * 100}%`;
    box.style.width = `${p.widthRatio * 100}%`;
    box.style.height = `${p.heightRatio * 100}%`;
    box.classList.toggle('keo-duoc', view.editable);
    box.title = view.editable
      ? this.boxTools()
        ? 'Kéo để đổi vị trí chữ ký (kể cả sang trang khác), kéo cạnh hoặc góc để đổi cỡ.\nChuột phải: xoá, thêm khung. Chọn rồi bấm Delete để xoá.'
        : 'Kéo để đổi vị trí chữ ký (kể cả sang trang khác), kéo cạnh hoặc góc để đổi cỡ'
      : '';
    if (view.editable) {
      box.addEventListener('pointerdown', (e) => this.startDrag(e, box));
    }
    const inner = document.createElement('div');
    inner.className = `chuky ${view.mode}`;
    if (view.mode === 'imageAndText') {
      inner.classList.add(`bo-${view.layout ?? 'left'}`);
      applySplit(inner, view);
    }
    if (view.mode !== 'text' && view.imageUrl) {
      const img = document.createElement('img');
      img.src = view.imageUrl;
      img.alt = '';
      img.draggable = false;
      inner.appendChild(img);
    }
    if (view.mode !== 'image') {
      const pxPerPt = Number(sheet.dataset['pxPerPt']) || 1;
      const pageScale = Number(sheet.dataset['pageScale']) || 1;
      const text = document.createElement('div');
      text.className = 'tt-ky';
      text.style.fontSize = `${view.fontPt * pageScale * pxPerPt}px`;
      if (view.fontFamily) {
        text.style.fontFamily = `'${view.fontFamily}'`;
      }
      if (view.color) {
        text.style.color = view.color;
      }
      for (const line of view.lines) {
        const row = document.createElement('div');
        row.textContent = line;
        text.appendChild(row);
      }
      inner.appendChild(text);
    }
    box.appendChild(inner);
    if (view.editable) {
      for (const c of CORNERS) {
        const handle = document.createElement('div');
        handle.className = `tay ${c}`;
        handle.dataset['tay'] = c;
        box.appendChild(handle);
      }
    }
    return box;
  }

  private sheetAt(x: number, y: number): HTMLElement | null {
    for (const s of this.sheets()) {
      const r = s.getBoundingClientRect();
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
        return s;
      }
    }
    return null;
  }

  /** Kéo khung (đổi chỗ, có thể sang trang khác) hoặc kéo góc (đổi cỡ); thả thì báo vị trí mới. */
  private startDrag(e: PointerEvent, box: HTMLDivElement): void {
    const sheet0 = box.parentElement;
    if (!sheet0 || e.button !== 0) {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const index = Number(box.dataset['i'] ?? 0);
    if (this.boxTools()) {
      this.selectBox(index);
    }
    const corner = (e.target as HTMLElement).closest<HTMLElement>('.tay')?.dataset['tay'] ?? null;
    let sheet = sheet0;
    const s0 = sheet.getBoundingClientRect();
    const b0 = box.getBoundingClientRect();
    const start = { x: b0.left - s0.left, y: b0.top - s0.top, w: b0.width, h: b0.height };
    const grab = { x: e.clientX - b0.left, y: e.clientY - b0.top };
    const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));
    let rect = { ...start };
    const apply = () => {
      const W = sheet.clientWidth || 1;
      const H = sheet.clientHeight || 1;
      box.style.left = `${(rect.x / W) * 100}%`;
      box.style.top = `${(rect.y / H) * 100}%`;
      box.style.width = `${(rect.w / W) * 100}%`;
      box.style.height = `${(rect.h / H) * 100}%`;
    };
    const move = (ev: PointerEvent) => {
      if (!corner) {
        const over = this.sheetAt(ev.clientX, ev.clientY);
        if (over && over !== sheet) {
          sheet = over;
          sheet.appendChild(box);
        }
        const s = sheet.getBoundingClientRect();
        const w = Math.min(rect.w, s.width);
        const h = Math.min(rect.h, s.height);
        rect = {
          x: clamp(ev.clientX - grab.x - s.left, 0, s.width - w),
          y: clamp(ev.clientY - grab.y - s.top, 0, s.height - h),
          w,
          h,
        };
      } else {
        const W = sheet.clientWidth;
        const H = sheet.clientHeight;
        const dx = ev.clientX - e.clientX;
        const dy = ev.clientY - e.clientY;
        let left = start.x;
        let top = start.y;
        let right = start.x + start.w;
        let bottom = start.y + start.h;
        if (corner.includes('l')) left = clamp(start.x + dx, 0, right - MIN_BOX_PX);
        if (corner.includes('r')) right = clamp(right + dx, left + MIN_BOX_PX, W);
        if (corner.includes('t')) top = clamp(start.y + dy, 0, bottom - MIN_BOX_PX);
        if (corner.includes('b')) bottom = clamp(bottom + dy, top + MIN_BOX_PX, H);
        rect = { x: left, y: top, w: right - left, h: bottom - top };
      }
      apply();
    };
    const end = () => {
      box.removeEventListener('pointermove', move);
      box.removeEventListener('pointerup', end);
      box.removeEventListener('pointercancel', end);
      this.dragging = false;
      const W = sheet.clientWidth || 1;
      const H = sheet.clientHeight || 1;
      const page = this.sheets().indexOf(sheet) + 1;
      const round = (v: number) => Math.round(v * 10000) / 10000;
      const placement: SignPlacement = {
        page: Math.max(page, 1),
        xRatio: round(rect.x / W),
        yRatio: round(rect.y / H),
        widthRatio: round(rect.w / W),
        heightRatio: round(rect.h / H),
      };
      this.sigBoxChange.emit(placement);
      this.sigBoxEdit.emit({ index, placement });
      this.placeBoxes();
    };
    this.dragging = true;
    box.setPointerCapture(e.pointerId);
    box.addEventListener('pointermove', move);
    box.addEventListener('pointerup', end);
    box.addEventListener('pointercancel', end);
  }

  // ---- Chọn, xoá, menu chuột phải ----------------------------------------------------------------

  private selectBox(index: number | null): void {
    this.selectedBox.set(index);
    for (const b of this.boxes) {
      b.classList.toggle('chon', index !== null && b.dataset['i'] === String(index));
    }
  }

  /** Chuột phải trên trang: chọn khung bị bấm (nếu có) rồi báo màn chứa mở menu. Ngoài trang thì bỏ qua. */
  protected onContextMenu(e: MouseEvent): void {
    if (!this.boxTools()) {
      return;
    }
    e.preventDefault();
    const sheet = this.sheetAt(e.clientX, e.clientY);
    if (!sheet) {
      return;
    }
    const boxEl = (e.target as HTMLElement).closest<HTMLElement>('.chuky-o');
    const index = boxEl ? Number(boxEl.dataset['i'] ?? 0) : null;
    this.selectBox(index);
    const r = sheet.getBoundingClientRect();
    const ratio = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 10000) / 10000;
    this.boxMenu.emit({
      index,
      page: this.sheets().indexOf(sheet) + 1,
      xRatio: ratio((e.clientX - r.left) / r.width),
      yRatio: ratio((e.clientY - r.top) / r.height),
      clientX: e.clientX,
      clientY: e.clientY,
    });
  }

  /** Delete / Backspace: xoá khung đang chọn (không khi đang gõ chữ trong ô nhập). Esc: bỏ chọn. */
  protected onKey(e: KeyboardEvent): void {
    const sel = this.selectedBox();
    if (!this.boxTools() || sel === null || e.ctrlKey || e.altKey || e.metaKey) {
      return;
    }
    const t = e.target;
    if (t instanceof HTMLElement && (t.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName))) {
      return;
    }
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      this.sigBoxDelete.emit(sel);
    } else if (e.key === 'Escape') {
      this.selectBox(null);
    }
  }

  // ---- Trang (ảnh thu nhỏ) -------------------------------------------------------------------------

  /** Trang đang ở giữa vùng xem. */
  protected trackPage(): void {
    const sheets = this.sheets();
    if (!sheets.length) {
      return;
    }
    const view = this.host.nativeElement.getBoundingClientRect();
    const mid = view.top + view.height / 2;
    let best = 0;
    let bestDist = Infinity;
    sheets.forEach((s, i) => {
      const r = s.getBoundingClientRect();
      const d = mid < r.top ? r.top - mid : mid > r.bottom ? mid - r.bottom : 0;
      if (d < bestDist) {
        bestDist = d;
        best = i;
      }
    });
    if (this.currentPage() !== best + 1) {
      this.currentPage.set(best + 1);
    }
  }

  /** Cuộn tới đầu trang `n` (từ 1). */
  goToPage(n: number): void {
    const sheets = this.sheets();
    const sheet = sheets[Math.min(Math.max(n, 1), sheets.length) - 1];
    if (!sheet) {
      return;
    }
    this.focusPending = false;
    const view = this.host.nativeElement;
    view.scrollTop += sheet.getBoundingClientRect().top - view.getBoundingClientRect().top - 12;
    this.currentPage.set(n);
  }

  /** Vẽ trang `n` thu nhỏ lên `canvas` rộng `width` px CSS. Bỏ qua nếu văn bản đã đổi trong lúc vẽ. */
  async renderThumb(n: number, canvas: HTMLCanvasElement, width: number): Promise<void> {
    const doc = this.doc;
    if (!doc || n < 1 || n > doc.numPages) {
      return;
    }
    const version = this.docVersion();
    const page = await doc.getPage(n);
    if (version !== this.docVersion()) {
      return;
    }
    const base = page.getViewport({ scale: 1 });
    const viewport = page.getViewport({ scale: width / base.width });
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.floor(viewport.width * ratio);
    canvas.height = Math.floor(viewport.height * ratio);
    canvas.style.width = Math.floor(viewport.width) + 'px';
    canvas.style.height = Math.floor(viewport.height) + 'px';
    try {
      await page.render({ canvas, viewport, transform: ratio !== 1 ? [ratio, 0, 0, ratio, 0, 0] : undefined }).promise;
    } catch {
      // Văn bản đổi giữa chừng: ảnh thu nhỏ sẽ được vẽ lại.
    }
  }

  private cancelTasks(): void {
    for (const t of this.tasks) {
      t.cancel();
    }
    this.tasks = [];
    for (const t of this.textLayers) {
      t.cancel();
    }
    this.textLayers = [];
  }

  private async dropDocument(): Promise<void> {
    const loading = this.loading;
    const doc = this.doc;
    this.loading = null;
    this.doc = null;
    try {
      // pdf.js 6: huỷ tài liệu qua loading task của nó (giải phóng cả phía worker).
      await (doc?.loadingTask ?? loading)?.destroy();
    } catch {
      // Huỷ tài liệu cũ không thành công thì bỏ qua: tài liệu mới không phụ thuộc vào nó.
    }
  }
}

/** Chữ trong khung tràn bề rộng thì thu nhỏ cỡ chữ cho vừa, như lõi thu chữ khi dựng khối chữ ký. */
/**
 * Phần ảnh / khoảng cách tự chỉnh (TD-51), giống lõi `image_text_boxes`: trái / phải tính theo bề ngang khối, trên /
 * dưới theo chiều cao. Ảnh vừa ô của nó, giữ tỉ lệ, căn giữa.
 */
function applySplit(inner: HTMLElement, view: SigBoxView): void {
  if (view.imageBox && view.textBox) {
    inner.classList.add('tu-do');
    const put = (k: string, b: PartBox) => {
      inner.style.setProperty(`--${k}x`, `${b.x * 100}%`);
      inner.style.setProperty(`--${k}y`, `${b.y * 100}%`);
      inner.style.setProperty(`--${k}w`, `${b.w * 100}%`);
      inner.style.setProperty(`--${k}h`, `${b.h * 100}%`);
    };
    put('a', view.imageBox);
    put('c', view.textBox);
    return;
  }
  const stacked = view.layout === 'top' || view.layout === 'bottom';
  if (view.imageShare != null) {
    inner.classList.add('tu-chia');
    inner.style.setProperty('--anh', `${view.imageShare}%`);
  }
  if (view.imageGap != null) {
    // Khoảng cách theo % khối: dùng đơn vị container query để trên / dưới tính theo chiều cao khối.
    inner.style.gap = stacked ? `${view.imageGap}cqh` : `${view.imageGap}cqw`;
  }
}

function fitText(box: HTMLElement): void {
  const t = box.querySelector<HTMLElement>('.tt-ky');
  if (!t || t.clientWidth <= 0) {
    return;
  }
  // Kiểu "Tự do": ô chữ có chiều cao riêng, chữ thu cả theo chiều cao như lõi.
  const free = !!t.closest('.tu-do');
  const rowsH = [...t.children].reduce((sum, r) => sum + (r as HTMLElement).offsetHeight, 0);
  const fw = t.scrollWidth > t.clientWidth + 1 ? t.clientWidth / t.scrollWidth : 1;
  const fh = free && rowsH > t.clientHeight + 1 ? t.clientHeight / rowsH : 1;
  const f = Math.min(fw, fh);
  const size = parseFloat(t.style.fontSize);
  if (f < 1 && size > 0) {
    t.style.fontSize = `${size * f}px`;
  }
}

function releaseSheets(sheets: HTMLDivElement[]): void {
  for (const s of sheets) {
    releaseCanvases(s);
  }
}
