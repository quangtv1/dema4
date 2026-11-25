import { ChangeDetectionStrategy, Component, DestroyRef, ElementRef, computed, effect, inject, signal } from '@angular/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';

import { PlannedFile } from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { BigIconComponent, IconComponent } from '../shared/icon.component';
import { clamp, fmtDur } from '../shared/text';
import { TipComponent } from '../shared/tip.component';
import { AdvancedButtonComponent, PathInputComponent, PopupComponent } from '../shared/widgets';
import { PdfViewComponent } from './pdf-view.component';
import { TaoVanBanStore, View } from './tao-van-ban.store';

/** Số dòng tối đa hiện trong bảng xem nhanh dữ liệu (bảng chỉ để xem, danh sách Kết quả vẫn đủ mọi dòng). */
const XLS_MAX_ROWS = 200;

/**
 * Màn Tạo văn bản — prototype `manTVB()` L1182–1280, mục 3.3 bản phân tích. Mọi dữ liệu lấy từ lõi qua IPC;
 * trạng thái nằm trong `TaoVanBanStore`.
 */
@Component({
  selector: 'app-tao-van-ban',
  imports: [
    IconComponent,
    BigIconComponent,
    TipComponent,
    PathInputComponent,
    AdvancedButtonComponent,
    PopupComponent,
    PdfViewComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man', '(document:keydown)': 'onKey($event)' },
  templateUrl: './tao-van-ban.component.html',
})
export class TaoVanBanComponent {
  protected readonly s = inject(TaoVanBanStore);
  protected readonly ui = inject(UiService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected readonly fmtDur = fmtDur;
  protected readonly xlsMax = XLS_MAX_ROWS;

  /** Phụ lục A — nguyên văn, trừ chỗ ghi chú. */
  protected readonly tipMau =
    'Mẫu có các trường dạng {hoten}, {so}… và khung ảnh; trường sẽ được thay bằng dữ liệu.';
  protected readonly tipTieuDe = 'Dòng chứa tên cột trong file Excel.';
  protected readonly tipTen =
    'Ghép tên trường trong ngoặc {}, ví dụ {so}_{hoten}.\nTên được bỏ dấu, khoảng trắng thành _, ký tự không hợp lệ thành -, tối đa 100 ký tự; trùng tên thêm (1), (2)…';
  protected readonly tipGioiThieu =
    'Mỗi dòng Excel tạo một văn bản, hoặc nhập trực tiếp một văn bản.\nChèn ảnh thẻ, con dấu vào khung.\nXuất PDF hoặc DOCX.';
  /** Prototype ghi "Biến xanh = đã ghép…" vì nó tô biến trên trang mô phỏng; bản xem trước PDF thật không tô. */
  protected readonly tipXemMau = 'Bản xem trước của mẫu gốc, chưa điền dữ liệu.';

  protected readonly dragging = signal(false);
  protected readonly resizing = signal(false);
  /** Toạ độ của menu chọn nguồn (position: fixed). */
  protected readonly pickPos = signal<{ left: number; top: number | null; bottom: number | null }>({
    left: 0,
    top: 0,
    bottom: null,
  });

  protected readonly locked = computed(() => !this.s.available || this.s.running());
  protected readonly canPick = computed(() => this.s.available && !this.s.running());
  protected readonly noCoreTitle = computed(() =>
    this.s.available ? '' : 'Cần chạy trong ứng dụng Sao Mai Ký số (trình duyệt không có lõi).',
  );

  protected readonly sheets = computed(() => this.s.table()?.sheets ?? []);
  protected readonly xlsRows = computed(() => (this.s.table()?.rows ?? []).slice(0, XLS_MAX_ROWS));

  /** Cột trái thu gọn được thành rail số: chỉ ở tab Kết quả hoặc khi chưa sẵn sàng (biến `canCol` L1213). */
  protected readonly canCollapse = computed(() => this.s.isTable() && (!this.s.ready() || this.s.view() === 'ketqua'));
  protected readonly showList = computed(() => !this.canCollapse() || this.s.listOpen());

  protected readonly gridKq = computed(() =>
    this.s.job() ? '28px minmax(0,1fr) auto' : '28px minmax(0,1fr)',
  );

  protected readonly navTitle = computed(() =>
    this.s.view() === 'mau' ? (this.s.template()?.fileName ?? '') : (this.s.currentFile()?.fileName ?? ''),
  );

  protected readonly fmtLabel = computed(() => this.s.format().toUpperCase());

  protected readonly footerTip = computed(() => {
    const folder = this.s.plan()?.outputFolder ?? '';
    return `Lưu tại: ${folder}\nTên file: ${this.s.pattern()}\nĐổi định dạng, thư mục, tên file ở Nâng cao.`;
  });

  protected readonly tipThuMuc = computed(() =>
    this.s.isTable()
      ? 'Để trống: lưu cạnh file dữ liệu.\nMỗi lần tạo sinh một thư mục con <tên mẫu>_<ngày giờ>.'
      : 'Để trống: lưu cạnh file mẫu.\nMỗi lần tạo sinh một thư mục con <tên mẫu>_<ngày giờ>.',
  );

  protected readonly runLabel = computed(() =>
    this.s.ready() && this.s.plan() ? `Tạo ${this.s.files().length} văn bản` : 'Tạo văn bản',
  );

  protected readonly progressPct = computed(() => {
    const j = this.s.job();
    return j && j.total > 0 ? Math.round((j.current / j.total) * 100) : 0;
  });

  /** "Đang tạo x/y · đã chạy mm:ss · còn khoảng mm:ss" — công thức ETA mục 6.3.1. */
  protected readonly progressText = computed(() => {
    const j = this.s.job();
    if (!j) {
      return '';
    }
    if (j.stopping) {
      return `Đang dừng… ${j.current}/${j.total}`;
    }
    const elapsed = (j.finishedAt ?? this.s.now()) - j.startedAt;
    const done = j.current;
    const remain = Math.max(0, j.total - done);
    let text = `Đang tạo ${done}/${j.total} · đã chạy ${fmtDur(elapsed)}`;
    if (done > 0) {
      text += ` · còn khoảng ${fmtDur(Math.round((elapsed / done) * remain))}`;
    } else {
      text += ' · đang ước lượng…';
    }
    return text;
  });

  protected readonly unmappedText = computed(() =>
    (this.s.plan()?.unmappedFields ?? []).map((f) => `{${f}}`).join(', '),
  );
  protected readonly fileWarnings = computed(() => (this.s.plan()?.files ?? []).filter((f) => f.warnings.length > 0));

  protected readonly failedFiles = computed(() => {
    const j = this.s.job();
    if (!j) {
      return [];
    }
    return j.files
      .map((file, i) => ({ file, result: j.results[i] }))
      .filter((x) => x.result?.status === 'error')
      .map((x) => ({ fileName: x.file.fileName, rowNumber: x.file.rowNumber, reason: x.result?.reason ?? '' }));
  });

  protected readonly doneTitle = computed(() => {
    const sum = this.s.job()?.summary;
    if (!sum) {
      return 'Tạo văn bản';
    }
    if (sum.stopped) {
      return 'Đã dừng tạo văn bản';
    }
    return sum.failed > 0 ? 'Tạo văn bản xong, có văn bản lỗi' : 'Tạo văn bản thành công';
  });

  constructor() {
    const destroyRef = inject(DestroyRef);
    if (this.s.available) {
      // Kéo thả file vào cửa sổ: lấy đường dẫn thật từ WebView của Tauri.
      let unlisten: (() => void) | null = null;
      let destroyed = false;
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const p = event.payload;
          if (p.type === 'enter' || p.type === 'over') {
            this.dragging.set(!this.s.running());
          } else if (p.type === 'leave') {
            this.dragging.set(false);
          } else if (p.type === 'drop') {
            this.dragging.set(false);
            void this.s.dropFiles(p.paths);
          }
        })
        .then((fn) => (destroyed ? fn() : (unlisten = fn)))
        .catch(() => undefined);
      destroyRef.onDestroy(() => {
        destroyed = true;
        unlisten?.();
      });
    }

    // Giữ dòng đang tạo (khi chạy) hoặc dòng đang chọn trong tầm nhìn.
    effect(() => {
      this.s.selectedIndex();
      this.s.job();
      setTimeout(() => {
        const root = this.host.nativeElement;
        const row = root.querySelector('.ds .dong.dang-tao') ?? root.querySelector('.ds .dong.chon');
        row?.scrollIntoView({ block: 'nearest' });
      });
    });
  }

  // ---- Tab / danh sách ----------------------------------------------------------------------

  protected setView(view: View): void {
    this.ui.sourcePick.set(null);
    this.s.view.set(view);
  }

  protected toggleAdv(): void {
    this.s.advOpen.update((v) => !v);
  }

  /**
   * Ctrl+→ / Ctrl+← : văn bản sau / trước ở tab Kết quả. Không dùng khi đang gõ trong ô chữ (Ctrl+mũi tên ở
   * đó là nhảy theo từ) hoặc khi có hộp thoại.
   */
  protected onKey(event: KeyboardEvent): void {
    if (!event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {
      return;
    }
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (!step || this.ui.dialog() || isTextField(event.target)) {
      return;
    }
    if (!this.s.ready() || !this.s.isTable() || this.s.view() !== 'ketqua' || this.s.files().length === 0) {
      return;
    }
    event.preventDefault();
    // Effect ở constructor tự cuộn danh sách tới dòng đang chọn.
    this.s.selectIndex(this.s.selectedIndex() + step);
  }

  protected useManual(): void {
    this.s.setSource('manual');
  }

  protected resultReason(index: number): string | null {
    return this.s.job()?.results[index]?.reason ?? null;
  }

  protected rowTitle(file: PlannedFile, index: number): string {
    const lines = [`Dòng ${file.rowNumber}`];
    const text = this.s.rowText(file.rowNumber);
    if (text) {
      lines[0] += `: ${text}`;
    }
    const reason = this.resultReason(index);
    if (reason) {
      lines.push(`Lỗi: ${reason}`);
    }
    lines.push(...file.warnings);
    return lines.join('\n');
  }

  // ---- Ghép biến ----------------------------------------------------------------------------

  protected sourceLabel(field: string): string {
    const src = this.s.mappings()[field];
    if (!src || src.kind === 'none') {
      return 'Chọn…';
    }
    if (src.kind === 'fixed') {
      return src.value || '(giá trị cố định)';
    }
    const col = this.s.table()?.columns.find((c) => c.column === src.column);
    return col ? col.name || `Cột ${col.column}` : 'Chọn…';
  }

  protected manualValue(field: string): string {
    return this.s.manualValues()[field] ?? '';
  }

  protected fixedValue(field: string): string {
    const src = this.s.mappings()[field];
    return src?.kind === 'fixed' ? src.value : '';
  }

  protected isColumn(field: string, column: number): boolean {
    const src = this.s.mappings()[field];
    return src?.kind === 'column' && src.column === column;
  }

  protected togglePick(field: string, event: MouseEvent): void {
    this.ui.tip.set(null);
    if (this.ui.sourcePick() === field) {
      this.ui.sourcePick.set(null);
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const left = Math.max(8, Math.min(rect.left, window.innerWidth - 300));
    const below = window.innerHeight - rect.bottom;
    this.pickPos.set(
      below >= 320 || below >= rect.top
        ? { left, top: rect.bottom + 4, bottom: null }
        : { left, top: null, bottom: window.innerHeight - rect.top + 4 },
    );
    this.ui.sourcePick.set(field);
    setTimeout(() => this.host.nativeElement.querySelector<HTMLInputElement>('.lt-menu input')?.focus());
  }

  protected pickColumn(field: string, column: number): void {
    this.s.setColumn(field, column);
    this.ui.sourcePick.set(null);
  }

  /** Enter trong ô giá trị cố định: ghi giá trị (nếu có) rồi đóng menu, như L799. */
  protected applyFixed(field: string, value: string, event: Event): void {
    event.preventDefault();
    this.s.setFixed(field, value);
    this.ui.sourcePick.set(null);
  }

  protected unmap(field: string): void {
    this.s.unmap(field);
    this.ui.sourcePick.set(null);
  }

  protected closePick(): void {
    if (this.ui.sourcePick()) {
      this.ui.sourcePick.set(null);
    }
  }

  // ---- Kéo đổi độ rộng cột trái (mục 6.3.10) --------------------------------------------------

  protected startResize(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    const bar = event.currentTarget as HTMLElement;
    const parent = bar.parentElement;
    const startX = event.clientX;
    const orig = this.s.leftW();
    this.resizing.set(true);
    document.body.style.cursor = 'col-resize';
    const move = (e: PointerEvent) => {
      const maxW = parent ? Math.max(320, parent.clientWidth - 260) : 560;
      this.s.leftW.set(clamp(orig + (e.clientX - startX), 180, maxW));
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

  protected numberValue(event: Event): number {
    return Number((event.target as HTMLInputElement).value);
  }

  protected checkedValue(event: Event): boolean {
    return (event.target as HTMLInputElement).checked;
  }

  protected textValue(event: Event): string {
    return (event.target as HTMLInputElement | HTMLSelectElement).value;
  }
}

/** Ô nhận gõ chữ (Ctrl+mũi tên trong đó là nhảy theo từ). Ô chỉ đọc, ô đánh dấu, nút không tính. */
function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
    return true;
  }
  if (target instanceof HTMLInputElement) {
    return !target.readOnly && !['checkbox', 'radio', 'button', 'submit', 'range'].includes(target.type);
  }
  return false;
}
