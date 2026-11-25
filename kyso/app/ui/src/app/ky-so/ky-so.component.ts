import { ChangeDetectionStrategy, Component, DestroyRef, ElementRef, computed, effect, inject, signal } from '@angular/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';

import { CertEntry, ImageLayout, SignFileItem, SignKind, SignatureMode, Verdict } from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { BigIconComponent, IconComponent } from '../shared/icon.component';
import { baseName, clamp, fmtDur } from '../shared/text';
import { TipComponent } from '../shared/tip.component';
import { PathInputComponent, PopupComponent } from '../shared/widgets';
import { BoxMenuEvent, PdfViewComponent } from '../tao-van-ban/pdf-view.component';
import {
  FILTERS,
  KsFilter,
  KsStatus,
  KySoStore,
  MODE_LABEL,
  STATUS_LABEL,
  IMAGE_LAYOUTS,
  VERDICT_LABEL,
  fmtDate,
} from './ky-so.store';
import { PageThumbsComponent } from './page-thumbs.component';

/** Nhãn lượt ký. */
export const KIND_LABEL: Record<SignKind, string> = { main: 'Chữ ký chính', initial: 'Ký nháy' };

/** Lựa chọn "Hiển thị" (kiểu khối chữ ký), đúng thứ tự trong prototype. */
const MODES: SignatureMode[] = ['imageAndText', 'text', 'image', 'hidden'];

/**
 * Màn Ký số văn bản — prototype `manKySo()` L1006–1111, mục 3.2 bản phân tích. Dữ liệu thật từ lõi qua IPC;
 * trạng thái nằm trong `KySoStore`. Hộp nhập PIN của prototype được thay bằng hộp "Xác nhận ký" (chọn chứng thư):
 * ứng dụng không hỏi, không lưu mã PIN.
 */
@Component({
  selector: 'app-ky-so',
  imports: [
    IconComponent,
    BigIconComponent,
    TipComponent,
    PathInputComponent,
    PopupComponent,
    PdfViewComponent,
    PageThumbsComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man ky-so', '(document:keydown)': 'onKey($event)', '(document:click)': 'onDocClick($event)' },
  templateUrl: './ky-so.component.html',
})
export class KySoComponent {
  protected readonly s = inject(KySoStore);
  protected readonly ui = inject(UiService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected readonly fmtDur = fmtDur;
  protected readonly fmtDate = fmtDate;
  protected readonly baseName = baseName;
  protected readonly modes = MODES;
  protected readonly modeLabel = MODE_LABEL;
  protected readonly statusLabel = STATUS_LABEL;
  protected readonly filters = FILTERS;
  protected readonly kindLabel = KIND_LABEL;
  protected readonly kinds: SignKind[] = ['main', 'initial'];
  protected readonly imageLayouts = IMAGE_LAYOUTS;
  protected readonly tipLuot =
    'Mỗi lần bấm Ký, mỗi văn bản nhận đúng một chữ ký (một chứng thư).\n' +
    'Ký nháy và ký chính của hai người: ký nháy trước bằng USB của người nháy, rồi ký chính lên bản đã ký bằng USB của người ký chính.\n' +
    'Ký nháy mặc định đặt ở mọi trang, góc dưới đối diện chữ ký chính.';

  /** Menu chuột phải trên bản xem trước (vị trí cố định theo màn hình). */
  protected readonly boxMenu = signal<(BoxMenuEvent & { left: number; top: number }) | null>(null);
  /** Nhắc ngắn trên thanh xem trước (ví dụ không xoá được khung cuối). */
  protected readonly boxHint = signal<string | null>(null);
  private hintTimer: ReturnType<typeof setTimeout> | null = null;

  protected readonly tipXoaGoc = computed(() => {
    const o = this.s.output();
    const where = o.fixedFolder ? `thư mục ${o.fixedFolder}` : `thư mục ${o.subfolder}`;
    return `Bật: thay file gốc bằng file đã ký (file gốc vào Thùng rác).\nTắt: giữ file gốc; file đã ký lưu vào ${where}, thêm hậu tố ${o.suffix}.`;
  });
  /** Nơi có file trùng tên (hộp trùng tên). */
  protected readonly dupWhere = computed(() => {
    const o = this.s.output();
    return o.fixedFolder ? `thư mục ${o.fixedFolder}` : `thư mục ${o.subfolder}`;
  });
  protected readonly tipCauHinh = 'Vị trí, cỡ khung chữ ký, kiểu hiển thị cho từng văn bản';
  protected readonly tipGioiThieu = computed(
    () => `Hỗ trợ văn bản PDF.\nFile đã ký ${this.s.outputNote()}.\nMã PIN nhập ở hộp Xác nhận ký (hoặc hộp của token); lưu PIN trên máy là tuỳ chọn, tắt sẵn.`,
  );
  protected readonly saveNote = computed(() => `${this.s.outputNote()}.${this.s.replaceOriginal() ? '' : ' File gốc giữ nguyên.'}`);

  protected readonly dragging = signal(false);
  protected readonly resizing = signal(false);

  protected readonly canPick = computed(() => !this.s.locked());
  protected readonly noCoreTitle = computed(() =>
    this.s.available ? '' : 'Cần chạy trong ứng dụng Sao Mai Ký số (trình duyệt không có lõi).',
  );

  protected readonly tdClass = computed(() => (this.s.running() ? 'dang' : this.s.allDone() ? 'xong' : 'dung'));
  protected readonly tdText = computed(() => {
    const state = this.s.running() ? 'Đang ký' : this.s.allDone() ? 'Đã xong' : 'Tạm dừng';
    return `${state} ${this.s.doneCount()}/${this.s.files().length}`;
  });
  protected readonly progressPct = computed(() => {
    const n = this.s.files().length;
    return n ? Math.round((this.s.doneCount() / n) * 100) : 0;
  });

  protected readonly runLabel = computed(() => {
    if (this.s.allDone()) {
      return 'Ký lại';
    }
    return this.s.hasRun() && this.s.remainCount() > 0 ? 'Ký tiếp' : 'Ký số';
  });

  protected readonly readyTip = computed(
    () =>
      `Ký tất cả văn bản ở tab Văn bản.\nLượt ký: ${KIND_LABEL[this.s.kind()]}\nHiển thị: ${MODE_LABEL[this.s.mode()]}\nVị trí: ${this.s.placementText()}\n` +
      `File đã ký: ${this.s.outputNote()}.`,
  );
  protected readonly doneTip = computed(
    () =>
      `${this.s.outputFolder() ? 'Lưu tại: ' + this.s.outputFolder() : 'File đã ký: ' + this.s.outputNote()}\nXem lý do lỗi ở tab Nhật ký.`,
  );

  protected readonly fileTip = computed(() => {
    const cur = this.s.current();
    if (!cur) {
      return '';
    }
    const lines = [cur.path];
    if (cur.signatureCount > 0) {
      lines.push(`Đã có ${cur.signatureCount} chữ ký`);
    }
    const pages = [...new Set(this.s.boxes().map((b) => b.page))];
    lines.push(
      `Vị trí: ${this.s.placementText()}${pages.length ? ` (trang ${pages.join(', ')})` : ''}`,
      `Hiển thị: ${MODE_LABEL[this.s.curMode()]}${this.s.curOwnMode() ? ' (riêng file này)' : ''}`,
    );
    const r = this.s.currentResult();
    if (r?.success && r.outputPath) {
      lines.push(`Đã ký: ${r.outputPath}`);
      if (r.verdict && VERDICT_LABEL[r.verdict]) {
        lines.push(`Kiểm tra: ${VERDICT_LABEL[r.verdict]}`);
      }
      if (r.trustedTime) {
        lines.push(`Dấu thời gian: ${r.trustedTime}`);
      }
    }
    return lines.join('\n');
  });

  /** Kết quả lượt ký vừa xong cho hộp Kết quả. */
  protected readonly failedFiles = computed(() => {
    const job = this.s.job();
    if (!job) {
      return [];
    }
    const res = this.s.results();
    return job.files
      .filter((p) => res[p] && !res[p].success && !res[p].paused)
      .map((p) => ({ path: p, name: baseName(p), reason: res[p].reason ?? 'Không ký được' }));
  });

  protected readonly doneTitle = computed(() => {
    const sum = this.s.job()?.summary;
    if (!sum) {
      return 'Ký số';
    }
    if (sum.stopped || sum.tokenRemoved) {
      return 'Đã tạm dừng';
    }
    return 'Ký số xong';
  });

  /** Ô "Hiển thị" chỉ là chữ: các kiểu có ảnh bị khoá khi chưa chọn ảnh. */
  protected modeDisabled(mode: SignatureMode): boolean {
    return (mode === 'image' || mode === 'imageAndText') && !this.s.hasImage();
  }

  constructor() {
    const destroyRef = inject(DestroyRef);
    if (this.s.available) {
      let unlisten: (() => void) | null = null;
      let destroyed = false;
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const p = event.payload;
          if (p.type === 'enter' || p.type === 'over') {
            this.dragging.set(!this.s.locked());
          } else if (p.type === 'leave') {
            this.dragging.set(false);
          } else if (p.type === 'drop') {
            this.dragging.set(false);
            void this.s.dropPaths(p.paths);
          }
        })
        .then((fn) => (destroyed ? fn() : (unlisten = fn)))
        .catch(() => undefined);
      destroyRef.onDestroy(() => {
        destroyed = true;
        unlisten?.();
      });
    }

    // Giữ dòng đang ký (khi chạy) hoặc dòng đang chọn trong tầm nhìn.
    effect(() => {
      this.s.selectedPath();
      this.s.runningPath();
      this.s.listOpen();
      setTimeout(() => {
        const root = this.host.nativeElement;
        const row =
          root.querySelector('.ds .dong.dang') ??
          root.querySelector('.ds .dong.chon') ??
          root.querySelector('.rail-stt.st-dang') ??
          root.querySelector('.rail-stt.chon');
        row?.scrollIntoView({ block: 'nearest' });
      });
    });

    // Nhật ký tự cuộn xuống dòng cuối.
    effect(() => {
      this.s.log();
      if (this.s.view() !== 'log') {
        return;
      }
      setTimeout(() => {
        const el = this.host.nativeElement.querySelector('.log');
        if (el) {
          el.scrollTop = el.scrollHeight;
        }
      });
    });
  }

  protected rowStatus(file: SignFileItem): KsStatus {
    return this.s.status(file.path);
  }

  protected rowReason(file: SignFileItem): string | null {
    const r = this.s.results()[file.path];
    if (!r || r.success) {
      return null;
    }
    // Tạm dừng do bấm nút: nhãn "Tạm dừng" đã đủ; rút token thì ghi lý do.
    if (r.paused) {
      return r.reason?.includes('rút USB token') ? r.reason : null;
    }
    return r.reason ?? 'Không ký được';
  }

  protected rowTitle(file: SignFileItem): string {
    const lines = [file.name, `Trạng thái: ${STATUS_LABEL[this.rowStatus(file)]}`];
    if (file.signatureCount > 0) {
      lines.push(`Đã có ${file.signatureCount} chữ ký`);
    }
    if (file.unreadable) {
      lines.push('Không đọc được file này');
    }
    const r = this.s.results()[file.path];
    if (r && !r.success) {
      lines.push(`Lỗi: ${r.reason ?? 'Không ký được'}`);
    }
    if (r?.success && r.verdict && VERDICT_LABEL[r.verdict]) {
      lines.push(`Kiểm tra: ${VERDICT_LABEL[r.verdict]}`);
    }
    return lines.join('\n');
  }

  protected certTitle(c: CertEntry): string {
    return [c.subject, `Cấp bởi: ${c.issuer}`, `Số sê-ri: ${c.serialNumber}`, `Hiệu lực: ${fmtDate(c.validFrom)} – ${fmtDate(c.validTo)}`].join('\n');
  }

  protected filterLabel(f: KsFilter): string {
    return f === 'all' ? 'Trạng thái' : STATUS_LABEL[f];
  }

  protected toggleFilter(): void {
    this.s.filterOpen.update((v) => !v);
  }

  protected verdictLabel(v: Verdict | null): string {
    return v ? VERDICT_LABEL[v] : '';
  }

  protected onDocClick(event: MouseEvent): void {
    const target = event.target instanceof Element ? event.target : null;
    if (this.s.filterOpen() && !target?.closest('.loc-th')) {
      this.s.filterOpen.set(false);
    }
    if (this.boxMenu() && !target?.closest('.ky-menu')) {
      this.boxMenu.set(null);
    }
  }

  // ---- Khung chữ ký: menu chuột phải, phím Delete ---------------------------------------------

  protected openBoxMenu(e: BoxMenuEvent): void {
    if (this.s.locked() || this.s.curMode() === 'hidden' || (!this.s.sigBoxes().length && !this.s.currentSkipped())) {
      this.boxMenu.set(null);
      return;
    }
    // Menu rộng khoảng 260 px, cao khoảng 170 px: đặt cạnh con trỏ, không tràn khỏi cửa sổ.
    const left = Math.max(8, Math.min(e.clientX, window.innerWidth - 268));
    const top = Math.max(8, Math.min(e.clientY, window.innerHeight - 178));
    this.boxMenu.set({ ...e, left, top });
  }

  protected menuDelete(): void {
    const m = this.boxMenu();
    this.boxMenu.set(null);
    if (m?.index !== null && m?.index !== undefined) {
      this.deleteBox(m.index);
    }
  }

  protected menuAdd(): void {
    const m = this.boxMenu();
    this.boxMenu.set(null);
    if (m) {
      this.s.addBox(m.page, m.xRatio, m.yRatio);
    }
  }

  protected menuCopyAll(pageCount: number): void {
    const m = this.boxMenu();
    this.boxMenu.set(null);
    if (m?.index !== null && m?.index !== undefined) {
      this.s.copyBoxToAllPages(m.index, pageCount);
    }
  }

  /** Menu chuột phải › Vị trí ảnh: đổi bố cục, giữ menu mở để thấy ngay khung đổi. */
  protected menuLayout(layout: ImageLayout): void {
    this.s.setImageLayout(layout);
  }

  protected menuReset(): void {
    this.boxMenu.set(null);
    this.s.resetBoxes();
  }

  /** Xoá khung (menu hoặc phím Delete); khung cuối cùng thì nhắc cách ký ẩn. */
  protected deleteBox(index: number): void {
    if (this.s.locked()) {
      return;
    }
    if (this.s.deleteBox(index) && this.s.currentSkipped()) {
      this.hint('Đã xoá hết khung: văn bản chưa gán chữ ký, sẽ bỏ qua khi ký số. Chuột phải trên trang để thêm khung lại.');
    }
  }

  private hint(text: string): void {
    this.boxHint.set(text);
    if (this.hintTimer) {
      clearTimeout(this.hintTimer);
    }
    this.hintTimer = setTimeout(() => this.boxHint.set(null), 4000);
  }

  /** Ctrl+→ / Ctrl+←: văn bản sau / trước. Esc đóng menu lọc. */
  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && this.boxMenu()) {
      this.boxMenu.set(null);
      return;
    }
    if (event.key === 'Escape' && this.s.filterOpen() && !this.ui.dialog()) {
      this.s.filterOpen.set(false);
      return;
    }
    if (!event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {
      return;
    }
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (!step || this.ui.dialog() || isTextField(event.target)) {
      return;
    }
    if (this.s.view() !== 'files' || this.s.visible().length === 0) {
      return;
    }
    event.preventDefault();
    this.s.selectPos(this.s.selectedPos() + step);
  }

  protected closeConfirm(): void {
    this.s.cancelConfirm();
  }

  protected showLog(): void {
    this.ui.close();
    this.s.view.set('log');
  }

  // ---- Kéo đổi độ rộng cột trái -------------------------------------------------------------

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
      this.s.leftW.set(clamp(orig + (e.clientX - startX), 200, maxW));
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

  /** Ô "Xóa file gốc": bật thì hỏi xác nhận trước (ô chỉ tích khi đã xác nhận). */
  protected onReplace(event: Event): void {
    const box = event.target as HTMLInputElement;
    const on = box.checked;
    box.checked = this.s.replaceOriginal();
    this.s.setReplace(on);
  }

  protected dirOf(path: string): string {
    const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
    return i > 0 ? path.slice(0, i) : '';
  }

  protected numberValue(event: Event): number {
    return Number((event.target as HTMLInputElement).value);
  }

  protected textValue(event: Event): string {
    return (event.target as HTMLInputElement | HTMLSelectElement).value;
  }

  protected isChecked(event: Event): boolean {
    return (event.target as HTMLInputElement).checked;
  }

  protected modeValue(event: Event): SignatureMode {
    return (event.target as HTMLSelectElement).value as SignatureMode;
  }
}

/** Ô nhận gõ chữ (Ctrl+mũi tên trong đó là nhảy theo từ). */
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
