import { Injectable, computed, inject, signal } from '@angular/core';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';

import { CheckState, IpcService, PreviewResult, SignatureVerify, Verdict, VerifyReport, errorMessage } from '../core/ipc.service';

/** Nhãn kết luận (giống lõi `Verdict::label`). */
export const VERDICT_TEXT: Record<Verdict, string> = {
  notApplicable: 'Không áp dụng',
  unsigned: 'Văn bản chưa có chữ ký số',
  validTrusted: 'Chữ ký hợp lệ',
  revocationUnknown: 'Hợp lệ, chưa kiểm tra được thu hồi',
  timeMismatch: 'Thời gian ký bất thường',
  noTimestamp: 'Không có dấu thời gian',
  untrustedTsa: 'Dấu thời gian không tin cậy',
  unsupported: 'Không xác định (thuật toán chưa hỗ trợ)',
  untrustedSigner: 'Người ký không thuộc CA Ban Cơ yếu',
  modifiedAfterSigning: 'Văn bản bị sửa sau khi ký',
  invalidSignature: 'Chữ ký không hợp lệ',
};

/** Màu của một kết luận: xanh (hợp lệ), vàng (cần lưu ý), đỏ (không hợp lệ / không tin cậy). */
export type Tone = 'ok' | 'warn' | 'fail';

export function verdictTone(v: Verdict): Tone {
  switch (v) {
    case 'validTrusted':
      return 'ok';
    case 'revocationUnknown':
    case 'noTimestamp':
    case 'unsupported':
    case 'notApplicable':
    case 'unsigned':
      return 'warn';
    default:
      return 'fail';
  }
}

export function checkTone(s: CheckState): Tone {
  return s === 'ok' ? 'ok' : s === 'fail' ? 'fail' : 'warn';
}

/** Trạng thái màn Kiểm tra chữ ký số (KT-01…KT-08). */
@Injectable({ providedIn: 'root' })
export class KiemTraStore {
  private readonly ipc = inject(IpcService);
  readonly available = this.ipc.available;

  readonly path = signal('');
  readonly report = signal<VerifyReport | null>(null);
  readonly busy = signal(false);
  readonly error = signal<string | null>(null);
  /** Kiểm thu hồi qua CRL/OCSP (cần mạng); mặc định bật như Nghị định 68/2024 Điều 31. */
  readonly revocation = signal(true);
  readonly tab = signal<'sig' | 'vb'>('sig');
  readonly selected = signal(0);
  readonly detailOpen = signal(false);
  readonly zoom = signal(80);
  readonly leftW = signal(300);
  readonly preview = signal<PreviewResult | null>(null);
  /** Lúc kiểm (giờ máy), để ghi vào báo cáo. */
  readonly checkedAt = signal('');
  readonly exporting = signal(false);
  readonly exportNote = signal<{ ok: boolean; text: string; path?: string } | null>(null);
  private seq = 0;

  readonly loaded = computed(() => this.report() !== null);
  readonly signatures = computed(() => this.report()?.signatures ?? []);
  readonly current = computed<SignatureVerify | null>(() => this.signatures()[this.selected()] ?? null);
  readonly fileName = computed(() => this.path().split(/[\\/]/).pop() ?? '');
  /** Số chữ ký (không tính dấu thời gian tài liệu). */
  readonly signatureCount = computed(() => this.signatures().filter((s) => s.kind === 'signature').length);
  readonly validCount = computed(() => this.signatures().filter((s) => s.verdict === 'validTrusted').length);
  /** Văn bản toàn vẹn: không chữ ký nào sai / bị sửa sau khi ký. */
  readonly intact = computed(
    () => this.signatures().length > 0 && this.signatures().every((s) => s.verdict !== 'invalidSignature' && s.verdict !== 'modifiedAfterSigning'),
  );

  async pick(): Promise<void> {
    if (!this.available || this.busy()) {
      return;
    }
    try {
      const path = await openDialog({
        directory: false,
        multiple: false,
        title: 'Chọn văn bản PDF đã ký số',
        filters: [{ name: 'Văn bản PDF', extensions: ['pdf'] }],
      });
      if (typeof path === 'string') {
        await this.load(path);
      }
    } catch (e) {
      this.error.set(errorMessage(e));
    }
  }

  /** Kéo thả: lấy file .pdf đầu tiên. */
  async dropPaths(paths: string[]): Promise<void> {
    const pdf = paths.find((p) => p.toLowerCase().endsWith('.pdf'));
    if (!pdf) {
      this.error.set('Chỉ kiểm tra được file PDF. Thả một file .pdf đã ký số.');
      return;
    }
    await this.load(pdf);
  }

  async load(path: string): Promise<void> {
    const seq = ++this.seq;
    this.busy.set(true);
    this.error.set(null);
    this.exportNote.set(null);
    this.path.set(path);
    try {
      const report = await this.ipc.verifyFile(path, this.revocation());
      if (seq !== this.seq) {
        return;
      }
      this.report.set(report);
      this.selected.set(0);
      this.checkedAt.set(nowText());
      const pdfPath = await this.ipc.signPreview(path).catch(() => null);
      if (seq === this.seq) {
        this.preview.set(pdfPath ? { pdfPath, pageCount: 0, warnings: [] } : null);
      }
    } catch (e) {
      if (seq === this.seq) {
        this.report.set(null);
        this.preview.set(null);
        this.error.set(errorMessage(e));
      }
    } finally {
      if (seq === this.seq) {
        this.busy.set(false);
      }
    }
  }

  /** Kiểm lại file đang mở (ví dụ sau khi bật / tắt kiểm thu hồi). */
  async reload(): Promise<void> {
    if (this.path()) {
      await this.load(this.path());
    }
  }

  setRevocation(on: boolean): void {
    this.revocation.set(on);
    if (this.loaded()) {
      void this.reload();
    }
  }

  clear(): void {
    this.seq++;
    this.path.set('');
    this.report.set(null);
    this.preview.set(null);
    this.error.set(null);
    this.exportNote.set(null);
    this.busy.set(false);
    this.selected.set(0);
  }

  select(i: number): void {
    const n = this.signatures().length;
    if (n) {
      this.selected.set(Math.min(Math.max(i, 0), n - 1));
    }
  }

  zoomBy(step: number): void {
    this.zoom.set(Math.min(160, Math.max(40, this.zoom() + step)));
  }

  /** Lưu báo cáo ra PDF: hỏi chỗ lưu rồi in khối báo cáo của màn. */
  async exportReport(): Promise<void> {
    if (!this.loaded() || this.exporting()) {
      return;
    }
    this.exportNote.set(null);
    try {
      const base = this.fileName().replace(/\.pdf$/i, '');
      const target = await saveDialog({
        title: 'Lưu báo cáo kiểm tra chữ ký',
        defaultPath: this.path().replace(/[^\\/]*$/, '') + `${base}_BaoCaoKiemTra.pdf`,
        filters: [{ name: 'Văn bản PDF', extensions: ['pdf'] }],
      });
      if (!target) {
        return;
      }
      const path = /\.pdf$/i.test(target) ? target : `${target}.pdf`;
      this.exporting.set(true);
      // Đợi khối báo cáo vẽ xong theo dữ liệu mới nhất.
      await new Promise((r) => setTimeout(r, 50));
      await this.ipc.reportExportPdf(path);
      this.exportNote.set({ ok: true, text: `Đã lưu báo cáo: ${path.split(/[\\/]/).pop()}`, path });
    } catch (e) {
      this.exportNote.set({ ok: false, text: errorMessage(e) });
    } finally {
      this.exporting.set(false);
    }
  }

  async openExported(): Promise<void> {
    const p = this.exportNote()?.path;
    if (p) {
      await this.ipc.shellOpen(p).catch((e) => this.exportNote.set({ ok: false, text: errorMessage(e) }));
    }
  }
}

function nowText(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getDate())}/${p(d.getMonth() + 1)}/${d.getFullYear()} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}
