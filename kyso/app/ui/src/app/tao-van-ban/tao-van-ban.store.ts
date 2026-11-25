import { Injectable, computed, effect, inject, signal, untracked } from '@angular/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import {
  FieldSource,
  IpcService,
  MergeEvent,
  MergePlan,
  MergeRequest,
  PlannedFile,
  PreviewResult,
  TableInfo,
  TableRow,
  TemplateInfo,
  errorMessage,
} from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { baseName, extName, searchKey } from '../shared/text';

export type View = 'mau' | 'ketqua';
export type RowStatus = 'cho' | 'dang' | 'ok' | 'loi';

export interface FileResult {
  status: 'ok' | 'error';
  reason: string | null;
  outputPath: string | null;
}

/** Một lần chạy `mergeRun`. Danh sách file chụp lại lúc bấm Tạo để trạng thái từng dòng không lệch khi plan đổi. */
export interface Job {
  jobId: string | null;
  running: boolean;
  stopping: boolean;
  startedAt: number;
  finishedAt: number | null;
  total: number;
  /** Số file đã xử lý xong (kể cả lỗi), theo sự kiện `file`. */
  current: number;
  files: PlannedFile[];
  /** Kết quả theo thứ tự trong lô (chỉ số `index` của sự kiện). */
  results: (FileResult | null)[];
  outputFolder: string;
  summary: Extract<MergeEvent, { type: 'summary' }> | null;
  error: string | null;
}

export const TEMPLATE_EXTS = ['docx'];
export const DATA_EXTS = ['xlsx', 'xls', 'csv'];

/** Thời gian chờ trước khi gọi lại lõi sau một thay đổi (ms). */
const PLAN_DEBOUNCE = 300;
const PREVIEW_DEBOUNCE = 500;
/** Chờ khi chỉ đổi dòng đang xem (giữ phím mũi tên lướt danh sách không gọi lõi từng dòng). */
const ROW_DEBOUNCE = 60;
const HIGHLIGHT_KEY = 'smks.tvb.toMauBien';
const SOURCE_OPEN_KEY = 'smks.tvb.moKhungNguon';

/** Đọc cờ bật/tắt đã nhớ theo máy; không đọc được (chế độ riêng tư…) thì dùng mặc định. */
function readFlag(key: string, fallback: boolean): boolean {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : value === '1';
  } catch {
    return fallback;
  }
}

function writeFlag(key: string, on: boolean): void {
  try {
    localStorage.setItem(key, on ? '1' : '0');
  } catch {
    // Không ghi được: chỉ mất phần ghi nhớ.
  }
}

/** Từ khoá (đã bỏ dấu, viết liền) của biến hay dùng làm tên file: số hiệu, mã, họ tên, tên. */
const NAME_HINTS = ['so', 'sohieu', 'ma', 'id', 'hoten', 'ten'];

/**
 * Mẫu tên file mặc định. Theo prototype là `{so}_{hoten}`; nếu mẫu không có đủ hai biến đó thì lấy tối đa hai biến
 * đầu tiên "giống tên" (khớp NAME_HINTS sau khi bỏ dấu, gạch dưới), không có thì lấy hai biến đầu của mẫu.
 * Mẫu không có biến nào → chuỗi rỗng (lõi tự đặt tên).
 */
export function defaultPattern(fields: string[]): string {
  if (fields.includes('so') && fields.includes('hoten')) {
    return '{so}_{hoten}';
  }
  const norm = (f: string) => searchKey(f).replace(/[_\s-]/g, '');
  const hinted = fields.filter((f) => NAME_HINTS.includes(norm(f)));
  const picked = [...hinted, ...fields.filter((f) => !hinted.includes(f))].slice(0, 2);
  return picked.map((f) => `{${f}}`).join('_');
}

/**
 * Trạng thái và nghiệp vụ của màn Tạo văn bản. Đặt ở service gốc để giữ nguyên khi đổi sang màn khác rồi quay lại
 * (prototype giữ `S.tvb` như vậy) và để lô đang chạy không mất khi rời màn.
 */
@Injectable({ providedIn: 'root' })
export class TaoVanBanStore {
  private readonly ipc = inject(IpcService);
  private readonly ui = inject(UiService);

  readonly available = this.ipc.available;

  // ---- Đầu vào ------------------------------------------------------------------------------
  readonly template = signal<TemplateInfo | null>(null);
  readonly table = signal<TableInfo | null>(null);
  readonly source = signal<'table' | 'manual'>('table');
  readonly manualValues = signal<Record<string, string>>({});
  readonly mappings = signal<Record<string, FieldSource>>({});
  readonly pattern = signal('{so}_{hoten}');
  readonly format = signal<'pdf' | 'docx'>('pdf');
  readonly outputFolder = signal('');
  private patternTouched = false;
  private formatTouched = false;

  readonly templateBusy = signal(false);
  readonly tableBusy = signal(false);
  /** Đang đọc lại mẫu và dữ liệu từ đĩa (nút Tải lại). */
  readonly reloadBusy = signal(false);
  /** Tăng sau mỗi lần Tải lại: đường dẫn không đổi nhưng nội dung file có thể đã đổi, nên phải gọi lại lõi. */
  private readonly rev = signal(0);
  /** Lỗi khi chọn / đọc file (hiện ở dải báo lỗi dưới hàng nhập). */
  readonly loadError = signal<string | null>(null);

  // ---- Hiển thị -----------------------------------------------------------------------------
  readonly view = signal<View>('ketqua');
  readonly selectedRow = signal<number | null>(null);
  readonly search = signal('');
  readonly listOpen = signal(true);
  readonly xlsOpen = signal(false);
  readonly advOpen = signal(false);
  readonly zoom = signal(100);
  /** Tô màu biến (tab Mẫu) và giá trị đã điền (tab Kết quả) trên bản xem trước. Nhớ theo máy. */
  readonly highlight = signal(readFlag(HIGHLIGHT_KEY, true));
  /** Khung chọn mẫu / dữ liệu đang mở (thu gọn thì còn một dòng tóm tắt). Nhớ theo máy. */
  readonly sourceOpen = signal(readFlag(SOURCE_OPEN_KEY, true));
  readonly leftW = signal(360);

  // ---- Kết quả từ lõi -----------------------------------------------------------------------
  readonly plan = signal<MergePlan | null>(null);
  readonly planBusy = signal(false);
  readonly planError = signal<string | null>(null);
  readonly preview = signal<PreviewResult | null>(null);
  readonly previewBusy = signal(false);
  readonly previewError = signal<string | null>(null);
  readonly job = signal<Job | null>(null);
  /** Đồng hồ cho dòng tiến độ; chỉ chạy khi đang tạo. */
  readonly now = signal(Date.now());

  private planSeq = 0;
  private previewSeq = 0;
  private mapSeq = 0;
  private tableSeq = 0;
  private lastPreviewKey: string | null = null;
  private lastPreviewReq: string | null = null;
  private ticker: ReturnType<typeof setInterval> | null = null;

  // ---- Suy ra -------------------------------------------------------------------------------
  readonly fields = computed(() => this.template()?.fields ?? []);
  readonly isTable = computed(() => this.source() === 'table');
  readonly ready = computed(() => !!this.template() && (this.source() === 'manual' || !!this.table()));
  readonly running = computed(() => this.job()?.running ?? false);

  readonly wordAvailable = computed(() => this.plan()?.wordAvailable ?? this.ui.info()?.wordAvailable ?? true);

  /** Yêu cầu gửi lõi; `null` khi chưa đủ mẫu + dữ liệu. So sánh theo nội dung để không gọi lại thừa. */
  readonly request = computed<MergeRequest | null>(
    () => {
      const tpl = this.template();
      if (!tpl) {
        return null;
      }
      const folder = this.outputFolder().trim();
      const base = {
        templatePath: tpl.path,
        fileNamePattern: this.pattern(),
        format: this.format(),
        outputFolder: folder ? folder : null,
      };
      if (this.source() === 'manual') {
        const values: Record<string, string> = {};
        for (const f of tpl.fields) {
          values[f] = this.manualValues()[f] ?? '';
        }
        return { ...base, data: { kind: 'manual', values }, mappings: {} };
      }
      const table = this.table();
      if (!table) {
        return null;
      }
      return {
        ...base,
        data: { kind: 'table', path: table.path, sheet: table.sheet, headerRow: table.headerRow },
        mappings: this.mappings(),
      };
    },
    { equal: (a, b) => JSON.stringify(a) === JSON.stringify(b) },
  );

  readonly mappedCount = computed(() => this.fields().filter((f) => this.isMapped(f)).length);

  /** Danh sách ở tab Kết quả: của lần chạy gần nhất nếu còn, ngược lại của `mergePlan`. */
  readonly files = computed<PlannedFile[]>(() => this.job()?.files ?? this.plan()?.files ?? []);

  readonly rowsByNumber = computed(() => {
    const map = new Map<number, TableRow>();
    for (const r of this.table()?.rows ?? []) {
      map.set(r.rowNumber, r);
    }
    return map;
  });

  readonly filteredFiles = computed(() => {
    const q = searchKey(this.search());
    const list = this.files().map((file, index) => ({ file, index }));
    if (!q) {
      return list;
    }
    return list.filter(
      ({ file }) => searchKey(file.fileName).includes(q) || searchKey(this.rowText(file.rowNumber)).includes(q),
    );
  });

  readonly selectedIndex = computed(() => {
    const row = this.selectedRow();
    const i = this.files().findIndex((f) => f.rowNumber === row);
    return i >= 0 ? i : 0;
  });

  readonly currentFile = computed<PlannedFile | null>(() => this.files()[this.selectedIndex()] ?? null);

  /** Có gì cần hỏi lại trước khi chạy: biến chưa ghép, cảnh báo chung, cảnh báo từng dòng. */
  readonly hasWarnings = computed(() => {
    const p = this.plan();
    return !!p && (p.unmappedFields.length > 0 || p.warnings.length > 0 || p.files.some((f) => f.warnings.length > 0));
  });

  readonly canRun = computed(
    () =>
      this.available &&
      this.ready() &&
      !!this.plan() &&
      !this.planBusy() &&
      !this.planError() &&
      this.files().length > 0 &&
      !(this.format() === 'pdf' && !this.wordAvailable()),
  );

  constructor() {
    // Mặc định DOCX khi máy không có Word (chỉ khi người dùng chưa tự chọn định dạng).
    effect(() => {
      const info = this.ui.info();
      if (info && !info.wordAvailable && !this.formatTouched) {
        this.format.set('docx');
      }
    });

    // Gọi lại mergePlan (có debounce) mỗi khi mẫu / dữ liệu / ghép biến / tên file / định dạng / thư mục đổi.
    effect((onCleanup) => {
      const req = this.request();
      this.rev();
      untracked(() => this.clearFinishedJob());
      if (!req || !this.available) {
        this.planSeq++;
        this.plan.set(null);
        this.planBusy.set(false);
        this.planError.set(null);
        return;
      }
      this.planBusy.set(true);
      const timer = setTimeout(() => void this.runPlan(req), PLAN_DEBOUNCE);
      onCleanup(() => clearTimeout(timer));
    });

    // Xem trước: tab Mẫu = PDF của mẫu gốc; tab Kết quả = PDF của dòng đang chọn. Tạm dừng khi đang tạo lô.
    effect((onCleanup) => {
      const ready = this.ready();
      const view = this.view();
      const tpl = this.template();
      const req = this.request();
      const row = this.selectedRow();
      const manual = this.source() === 'manual';
      const mark = this.highlight();
      const rev = this.rev();
      let key: string | null = null;
      let call: (() => Promise<PreviewResult>) | null = null;
      if (this.available && ready && tpl && !this.running()) {
        if (view === 'mau') {
          key = 'mau|' + rev + '|' + mark + '|' + tpl.path;
          call = () => this.ipc.mergePreviewTemplate(tpl.path, mark);
        } else if (req && (manual || row !== null)) {
          const rowNumber = manual ? null : row;
          key = 'kq|' + rev + '|' + mark + '|' + JSON.stringify(req) + '|' + rowNumber;
          call = () => this.ipc.mergePreview(req, rowNumber, mark);
        }
      }
      if (!key || !call || key === this.lastPreviewKey) {
        // Không cần gọi: bỏ kết quả của lời gọi cũ (nếu còn đang chạy) để nó không đè lên bản đang hiện.
        this.previewSeq++;
        untracked(() => {
          this.previewBusy.set(false);
          // Không còn gì để xem (ví dụ sheet mới không có dòng dữ liệu): bỏ bản cũ để không hiện nhầm.
          // Đang chạy lô thì giữ bản đang hiện.
          if (!key && !this.running()) {
            this.lastPreviewKey = null;
            this.preview.set(null);
            this.previewError.set(null);
          }
        });
        return;
      }
      const run = call;
      const runKey = key;
      // Chỉ đổi dòng (hoặc tab) thì gọi gần như ngay: lõi đã giữ Word chạy sẵn và có bộ đệm. Đổi nội dung
      // (gõ nhập trực tiếp, ghép biến…) thì chờ người dùng ngừng tay.
      const reqKey = (view === 'mau' ? 'mau' : JSON.stringify(req)) + '|' + mark;
      const delay = reqKey === this.lastPreviewReq ? ROW_DEBOUNCE : PREVIEW_DEBOUNCE;
      this.lastPreviewReq = reqKey;
      untracked(() => this.previewBusy.set(true));
      const timer = setTimeout(() => void this.runPreview(runKey, run), delay);
      onCleanup(() => clearTimeout(timer));
    });
  }

  // ---- Chọn file ----------------------------------------------------------------------------

  async pickTemplate(): Promise<void> {
    const path = await this.pickFile('Mẫu văn bản Word', TEMPLATE_EXTS);
    if (path) {
      await this.loadTemplate(path);
    }
  }

  async pickData(): Promise<void> {
    const path = await this.pickFile('Dữ liệu Excel hoặc CSV', DATA_EXTS);
    if (path) {
      this.source.set('table');
      await this.loadTable(path, null, null);
    }
  }

  async pickFolder(): Promise<void> {
    if (!this.available || this.running()) {
      return;
    }
    try {
      const path = await openDialog({ directory: true, multiple: false, title: 'Chọn thư mục lưu văn bản' });
      if (typeof path === 'string') {
        this.outputFolder.set(path);
      }
    } catch (error) {
      this.loadError.set(errorMessage(error));
    }
  }

  /** File thả vào cửa sổ: .docx thành mẫu, .xlsx/.xls/.csv thành dữ liệu. */
  async dropFiles(paths: string[]): Promise<void> {
    if (this.running()) {
      return;
    }
    const tpl = paths.find((p) => TEMPLATE_EXTS.includes(extName(p)));
    const data = paths.find((p) => DATA_EXTS.includes(extName(p)));
    if (!tpl && !data) {
      this.loadError.set(
        `Không dùng được ${paths.map(baseName).join(', ')}. Thả file mẫu .docx hoặc file dữ liệu .xlsx, .xls, .csv.`,
      );
      return;
    }
    if (tpl) {
      await this.loadTemplate(tpl);
    }
    if (data) {
      this.source.set('table');
      await this.loadTable(data, null, null);
    }
  }

  async loadTemplate(path: string): Promise<void> {
    this.templateBusy.set(true);
    this.loadError.set(null);
    try {
      const info = await this.ipc.mergeInspectTemplate(path);
      this.template.set(info);
      this.lastPreviewKey = null;
      if (!this.patternTouched) {
        this.pattern.set(defaultPattern(info.fields));
      }
      this.manualValues.update((old) => Object.fromEntries(info.fields.map((f) => [f, old[f] ?? ''])));
      await this.remap();
    } catch (error) {
      this.loadError.set(`Không đọc được mẫu ${baseName(path)}: ${errorMessage(error)}`);
    } finally {
      this.templateBusy.set(false);
    }
  }

  async loadTable(path: string, sheet: string | null, headerRow: number | null): Promise<void> {
    const seq = ++this.tableSeq;
    this.tableBusy.set(true);
    this.loadError.set(null);
    try {
      const info = await this.ipc.mergeReadTable(path, sheet, headerRow);
      if (seq !== this.tableSeq) {
        return;
      }
      this.table.set(info);
      this.selectedRow.set(info.rows[0]?.rowNumber ?? null);
      await this.remap();
    } catch (error) {
      if (seq === this.tableSeq) {
        this.loadError.set(`Không đọc được dữ liệu ${baseName(path)}: ${errorMessage(error)}`);
      }
    } finally {
      if (seq === this.tableSeq) {
        this.tableBusy.set(false);
      }
    }
  }

  // ---- Sửa mẫu rồi tải lại -------------------------------------------------------------------

  /** Mở file mẫu bằng ứng dụng mở .docx của máy (Microsoft Word) để người dùng sửa. */
  async editTemplate(): Promise<void> {
    const tpl = this.template();
    if (!tpl || !this.available) {
      return;
    }
    try {
      await this.ipc.shellOpen(tpl.path);
    } catch (error) {
      this.loadError.set(`Không mở được mẫu ${baseName(tpl.path)}: ${errorMessage(error)}`);
    }
  }

  /**
   * Đọc lại mẫu và dữ liệu từ đĩa sau khi người dùng sửa và lưu. Giữ dòng đang xem và cách ghép biến: giá trị cố
   * định giữ nguyên, ghép theo cột đi theo tên cột (cột có thể đổi chỗ), biến mới hoặc biến mất cột thì ghép tự
   * động. Không đọc lại được mẫu thì giữ nguyên mọi thứ và báo lỗi; chỉ file dữ liệu lỗi thì vẫn nhận mẫu mới,
   * giữ bảng cũ và báo lỗi.
   */
  async reload(): Promise<void> {
    const tpl = this.template();
    if (!tpl || !this.available || this.running() || this.reloadBusy() || this.templateBusy() || this.tableBusy()) {
      return;
    }
    const oldTable = this.table();
    const seq = this.tableSeq;
    this.reloadBusy.set(true);
    this.loadError.set(null);
    const errors: string[] = [];
    try {
      let info: TemplateInfo;
      try {
        info = await this.ipc.mergeInspectTemplate(tpl.path);
      } catch (error) {
        // Không có mẫu thì không có gì để làm mới: giữ nguyên bản đang hiện, chỉ báo lỗi.
        this.loadError.set(`Không đọc lại được mẫu ${baseName(tpl.path)}: ${errorMessage(error)}`);
        return;
      }
      let table = oldTable;
      if (oldTable) {
        try {
          table = await this.ipc.mergeReadTable(oldTable.path, oldTable.sheet, oldTable.headerRow);
        } catch (error) {
          errors.push(`Không đọc lại được dữ liệu ${baseName(oldTable.path)}: ${errorMessage(error)}`);
        }
      }
      // Trong lúc chờ người dùng đã chọn file khác, đổi sheet hoặc bỏ file: không đè lên lựa chọn mới.
      if (seq !== this.tableSeq || this.template() !== tpl || this.table() !== oldTable) {
        return;
      }
      const mappings = await this.keepMappings(info.fields, oldTable, table);
      if (seq !== this.tableSeq || this.template() !== tpl || this.table() !== oldTable) {
        return;
      }
      this.template.set(info);
      if (!this.patternTouched) {
        this.pattern.set(defaultPattern(info.fields));
      }
      this.manualValues.update((old) => Object.fromEntries(info.fields.map((f) => [f, old[f] ?? ''])));
      if (table && table !== oldTable) {
        this.table.set(table);
        const row = this.selectedRow();
        if (!table.rows.some((r) => r.rowNumber === row)) {
          this.selectedRow.set(table.rows[0]?.rowNumber ?? null);
        }
      }
      this.mappings.set(mappings);
      this.mapSeq++;
      this.rev.update((v) => v + 1);
      if (errors.length) {
        this.loadError.set(errors.join(' '));
      }
    } finally {
      this.reloadBusy.set(false);
    }
  }

  /** Cách ghép biến sau khi tải lại (xem `reload`). */
  private async keepMappings(
    fields: string[],
    oldTable: TableInfo | null,
    table: TableInfo | null,
  ): Promise<Record<string, FieldSource>> {
    const old = this.mappings();
    if (!table) {
      return Object.fromEntries(Object.entries(old).filter(([f, s]) => fields.includes(f) && s.kind === 'fixed'));
    }
    let auto: Record<string, number | null> = {};
    try {
      auto = await this.ipc.mergeAutoMap(fields, table.columns);
    } catch {
      // Không ghép tự động được: biến mới để trống, người dùng tự chọn.
    }
    const next: Record<string, FieldSource> = {};
    for (const f of fields) {
      const src = old[f];
      if (src?.kind === 'fixed') {
        next[f] = src;
        continue;
      }
      if (src?.kind === 'column') {
        const name = oldTable?.columns.find((c) => c.column === src.column)?.name;
        const same = table.columns.find((c) => (name ? c.name === name : c.column === src.column));
        if (same) {
          next[f] = { kind: 'column', column: same.column };
          continue;
        }
      }
      const col = auto[f];
      next[f] = typeof col === 'number' ? { kind: 'column', column: col } : { kind: 'none' };
    }
    return next;
  }

  /**
   * Đổi sheet / dòng tiêu đề: đọc lại bảng nhưng không tự mở bảng dữ liệu (khác prototype L1291–1292, theo yêu cầu
   * người phụ trách ngày 20/11/2026); bảng chỉ mở khi bấm "Xem dữ liệu", đang mở thì giữ mở.
   */
  changeSheet(sheet: string): void {
    const t = this.table();
    if (t && sheet !== t.sheet) {
      void this.loadTable(t.path, sheet, null);
    }
  }

  changeHeaderRow(value: number): void {
    const t = this.table();
    const row = Math.max(1, Math.floor(value) || 1);
    if (t && row !== t.headerRow) {
      void this.loadTable(t.path, t.sheet, row);
    }
  }

  clearTemplate(): void {
    this.template.set(null);
    this.mappings.set({});
    this.resetPreview();
  }

  clearData(): void {
    this.tableSeq++;
    this.tableBusy.set(false);
    this.table.set(null);
    this.selectedRow.set(null);
    this.xlsOpen.set(false);
    // Bỏ các ghép theo cột (cột không còn), giữ giá trị cố định.
    this.mappings.update((m) => Object.fromEntries(Object.entries(m).filter(([, s]) => s.kind === 'fixed')));
    this.resetPreview();
  }

  // ---- Ghép biến ----------------------------------------------------------------------------

  isMapped(field: string): boolean {
    const s = this.mappings()[field];
    return !!s && (s.kind === 'column' || (s.kind === 'fixed' && s.value !== ''));
  }

  setColumn(field: string, column: number): void {
    this.mappings.update((m) => ({ ...m, [field]: { kind: 'column', column } }));
  }

  setFixed(field: string, value: string): void {
    const v = value.trim();
    if (v) {
      this.mappings.update((m) => ({ ...m, [field]: { kind: 'fixed', value: v } }));
    }
  }

  unmap(field: string): void {
    this.mappings.update((m) => ({ ...m, [field]: { kind: 'none' } }));
  }

  /** Ghép tự động bằng `mergeAutoMap`; giữ các giá trị cố định người dùng đã gõ. */
  private async remap(): Promise<void> {
    const seq = ++this.mapSeq;
    const tpl = this.template();
    const table = this.table();
    const fixed = Object.entries(this.mappings()).filter(([, s]) => s.kind === 'fixed');
    if (!tpl || !table) {
      this.mappings.set(Object.fromEntries(fixed));
      return;
    }
    try {
      const auto = await this.ipc.mergeAutoMap(tpl.fields, table.columns);
      if (seq !== this.mapSeq) {
        return;
      }
      const keep = new Map(fixed);
      const next: Record<string, FieldSource> = {};
      for (const f of tpl.fields) {
        const col = auto[f];
        next[f] = keep.get(f) ?? (typeof col === 'number' ? { kind: 'column', column: col } : { kind: 'none' });
      }
      this.mappings.set(next);
    } catch (error) {
      if (seq === this.mapSeq) {
        this.mappings.set(Object.fromEntries(fixed));
        this.loadError.set(`Không ghép tự động được: ${errorMessage(error)}`);
      }
    }
  }

  // ---- Tuỳ chọn -----------------------------------------------------------------------------

  setPattern(value: string): void {
    this.patternTouched = true;
    this.pattern.set(value);
  }

  setFormat(value: 'pdf' | 'docx'): void {
    this.formatTouched = true;
    this.format.set(value);
  }

  setSource(value: 'table' | 'manual'): void {
    this.source.set(value);
  }

  setManual(field: string, value: string): void {
    this.manualValues.update((v) => ({ ...v, [field]: value }));
  }

  // ---- Điều hướng ---------------------------------------------------------------------------

  rowText(rowNumber: number): string {
    return (this.rowsByNumber().get(rowNumber)?.values ?? []).filter(Boolean).join(' · ');
  }

  selectIndex(index: number): void {
    const list = this.files();
    if (list.length === 0) {
      return;
    }
    const i = Math.max(0, Math.min(list.length - 1, index));
    this.selectedRow.set(list[i].rowNumber);
  }

  setHighlight(on: boolean): void {
    this.highlight.set(on);
    writeFlag(HIGHLIGHT_KEY, on);
  }

  setSourceOpen(open: boolean): void {
    this.sourceOpen.set(open);
    writeFlag(SOURCE_OPEN_KEY, open);
  }

  zoomBy(step: number): void {
    this.zoom.update((z) => Math.max(40, Math.min(160, z + step)));
  }

  /**
   * Dòng "Đang tạo": dòng đầu tiên chưa có kết quả. Giả định lõi xử lý tuần tự theo thứ tự của `manifest.files`
   * (sự kiện chỉ báo khi một file xong, không báo lúc bắt đầu).
   */
  readonly runningIndex = computed(() => {
    const job = this.job();
    return job?.running ? job.results.findIndex((x) => x === null) : -1;
  });

  rowStatus(index: number): RowStatus | null {
    const job = this.job();
    if (!job) {
      return null;
    }
    const r = job.results[index];
    if (r) {
      return r.status === 'ok' ? 'ok' : 'loi';
    }
    return index === this.runningIndex() ? 'dang' : 'cho';
  }

  // ---- Chạy lô ------------------------------------------------------------------------------

  /** Bấm nút chính: đang chạy thì Dừng; có cảnh báo thì hỏi lại; ngược lại chạy luôn. */
  primaryAction(): void {
    if (this.running()) {
      void this.stop();
      return;
    }
    if (!this.canRun()) {
      return;
    }
    if (this.hasWarnings()) {
      this.ui.open({ kind: 'tvb-confirm' });
      return;
    }
    void this.run();
  }

  async run(): Promise<void> {
    const req = this.request();
    const plan = this.plan();
    if (!req || !plan || this.running()) {
      return;
    }
    this.ui.close();
    this.ui.sourcePick.set(null);
    this.job.set({
      jobId: null,
      running: true,
      stopping: false,
      startedAt: Date.now(),
      finishedAt: null,
      total: plan.files.length,
      current: 0,
      files: plan.files,
      results: plan.files.map(() => null),
      outputFolder: plan.outputFolder,
      summary: null,
      error: null,
    });
    this.view.set('ketqua');
    this.startTicker();
    try {
      const jobId = await this.ipc.mergeRun(req, (event) => this.onEvent(event));
      this.patchJob((j) => ({ ...j, jobId: j.jobId ?? jobId }));
    } catch (error) {
      this.stopTicker();
      this.patchJob((j) => ({ ...j, running: false, finishedAt: Date.now(), error: errorMessage(error) }));
    }
  }

  async stop(): Promise<void> {
    const job = this.job();
    if (!job?.running || !job.jobId) {
      return;
    }
    this.patchJob((j) => ({ ...j, stopping: true }));
    try {
      await this.ipc.jobCancel(job.jobId);
    } catch (error) {
      this.patchJob((j) => ({ ...j, stopping: false, error: errorMessage(error) }));
    }
  }

  async openOutputFolder(): Promise<void> {
    const folder = this.job()?.outputFolder;
    if (folder) {
      try {
        await this.ipc.shellOpen(folder);
      } catch (error) {
        this.patchJob((j) => ({ ...j, error: errorMessage(error) }));
      }
    }
  }

  private onEvent(event: MergeEvent): void {
    switch (event.type) {
      case 'manifest':
        this.patchJob((j) => {
          // Lấy tên file thật lõi sẽ tạo; giữ số dòng / cảnh báo của plan theo cùng thứ tự.
          const files = event.files.map((fileName, i) => ({
            rowNumber: j.files[i]?.rowNumber ?? i + 1,
            fileName,
            warnings: j.files[i]?.warnings ?? [],
          }));
          return {
            ...j,
            jobId: event.jobId,
            total: event.total,
            outputFolder: event.outputFolder,
            files,
            results: files.map((_, i) => j.results[i] ?? null),
          };
        });
        break;
      case 'file':
        this.patchJob((j) => {
          const results = [...j.results];
          results[event.index] = { status: event.status, reason: event.reason, outputPath: event.outputPath };
          const files = [...j.files];
          if (files[event.index]) {
            files[event.index] = { ...files[event.index], fileName: event.fileName, rowNumber: event.rowNumber };
          }
          return { ...j, jobId: event.jobId, results, files, current: event.current, total: event.total };
        });
        break;
      case 'summary':
        this.stopTicker();
        this.patchJob((j) => ({
          ...j,
          jobId: event.jobId,
          running: false,
          stopping: false,
          finishedAt: Date.now(),
          total: event.total,
          outputFolder: event.outputFolder,
          summary: event,
        }));
        if (this.ui.screen() === 'tvb') {
          this.ui.open({ kind: 'tvb-done' });
        }
        break;
    }
  }

  private patchJob(fn: (job: Job) => Job): void {
    this.job.update((j) => (j ? fn(j) : j));
  }

  /** Thay đổi đầu vào sau khi lô đã xong thì bỏ trạng thái của lô cũ, quay về danh sách theo plan. */
  private clearFinishedJob(): void {
    const job = this.job();
    if (job && !job.running) {
      this.job.set(null);
      if (this.ui.dialog()?.kind === 'tvb-done') {
        this.ui.close();
      }
    }
  }

  private startTicker(): void {
    this.stopTicker();
    this.now.set(Date.now());
    this.ticker = setInterval(() => this.now.set(Date.now()), 1000);
  }

  private stopTicker(): void {
    if (this.ticker) {
      clearInterval(this.ticker);
      this.ticker = null;
    }
    this.now.set(Date.now());
  }

  // ---- Gọi lõi ------------------------------------------------------------------------------

  private async runPlan(req: MergeRequest): Promise<void> {
    const seq = ++this.planSeq;
    try {
      const plan = await this.ipc.mergePlan(req);
      if (seq !== this.planSeq) {
        return;
      }
      this.plan.set(plan);
      this.planError.set(null);
      const row = this.selectedRow();
      if (!plan.files.some((f) => f.rowNumber === row)) {
        this.selectedRow.set(plan.files[0]?.rowNumber ?? null);
      }
    } catch (error) {
      if (seq === this.planSeq) {
        this.plan.set(null);
        this.planError.set(errorMessage(error));
      }
    } finally {
      if (seq === this.planSeq) {
        this.planBusy.set(false);
      }
    }
  }

  private async runPreview(key: string, call: () => Promise<PreviewResult>): Promise<void> {
    const seq = ++this.previewSeq;
    this.previewBusy.set(true);
    try {
      const result = await call();
      if (seq !== this.previewSeq) {
        return;
      }
      this.lastPreviewKey = key;
      this.preview.set(result);
      this.previewError.set(null);
    } catch (error) {
      if (seq === this.previewSeq) {
        // Không ghi lastPreviewKey để lần đổi sau (hoặc quay lại) thử lại được.
        this.lastPreviewKey = null;
        this.preview.set(null);
        this.previewError.set(errorMessage(error));
      }
    } finally {
      if (seq === this.previewSeq) {
        this.previewBusy.set(false);
      }
    }
  }

  private resetPreview(): void {
    this.previewSeq++;
    this.lastPreviewKey = null;
    this.preview.set(null);
    this.previewBusy.set(false);
    this.previewError.set(null);
  }

  private async pickFile(name: string, extensions: string[]): Promise<string | null> {
    if (!this.available || this.running()) {
      return null;
    }
    try {
      const path = await openDialog({ multiple: false, directory: false, filters: [{ name, extensions }] });
      return typeof path === 'string' ? path : null;
    } catch (error) {
      this.loadError.set(errorMessage(error));
      return null;
    }
  }
}
