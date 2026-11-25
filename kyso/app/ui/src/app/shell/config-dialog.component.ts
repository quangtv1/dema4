import { IMAGE_LAYOUTS } from '../ky-so/ky-so.store';
import { ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked } from '@angular/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import { ConfigService } from '../core/config.service';
import {
  Anchor,
  AppConfig,
  CertScan,
  IpcService,
  PagePick,
  Paper,
  SignatureMode,
  SignatureProfile,
  errorMessage,
} from '../core/ipc.service';
import { ConfigTab, UiService } from '../core/ui.service';
import { IconComponent } from '../shared/icon.component';
import { baseName } from '../shared/text';
import { TipComponent } from '../shared/tip.component';
import { AdvancedButtonComponent, PathInputComponent, PopupComponent } from '../shared/widgets';

export const POS_LABEL: Record<Anchor, string> = {
  tl: 'trên trái',
  tc: 'trên giữa',
  tr: 'trên phải',
  bl: 'dưới trái',
  bc: 'dưới giữa',
  br: 'dưới phải',
};
export const PAGE_LABEL: Record<PagePick, string> = { first: 'Trang đầu', last: 'Trang cuối', all: 'Tất cả các trang (ký nháy)' };
const PAPER_MM: Record<Paper, [number, number]> = { A1: [594, 841], A2: [420, 594], A3: [297, 420], A4: [210, 297], A5: [148, 210] };
const PAPERS: Paper[] = ['A1', 'A2', 'A3', 'A4', 'A5'];
const ANCHORS: Anchor[] = ['tl', 'tc', 'tr', 'bl', 'bc', 'br'];
const FONTS = ['Times New Roman', 'Arial', 'Verdana', 'Georgia', 'Courier New'];
const MODES: { id: SignatureMode; label: string }[] = [
  { id: 'imageAndText', label: 'Ảnh + thông tin người ký' },
  { id: 'image', label: 'Chỉ ảnh chữ ký / con dấu' },
  { id: 'text', label: 'Chỉ thông tin người ký' },
  { id: 'hidden', label: 'Ẩn (không hiện trên trang)' },
];
const PT_PER_MM = 72 / 25.4;

/** Khung xem trước của mẫu trên trang thu nhỏ (tỉ lệ %, gốc trên trái). Cách tính giống lõi (`plan_spots`). */
export function profileBox(p: SignatureProfile, s: AppConfig['signature']): { w: number; h: number; x: number; y: number; pageW: number; pageH: number } {
  const [a, b] = PAPER_MM[p.paper];
  const [wMm, hMm] = p.orientation === 'landscape' ? [b, a] : [a, b];
  const W = wMm * PT_PER_MM;
  const H = hMm * PT_PER_MM;
  const scale = Math.min(W / 595, H / 842);
  let w: number;
  let h: number;
  if (s.display === 'text') {
    // Ước lượng bề rộng 2 dòng "Ký bởi / Ngày ký" (lõi đo đúng theo font khi ký).
    const size = s.font.size * scale;
    w = 27 * size * 0.5;
    h = 2 * size * 1.15;
  } else {
    // Ảnh trên / dưới chữ: khối 130 × 66 pt như lõi (STACKED_WIDTH_PT / STACKED_HEIGHT_PT).
    const stacked = s.display === 'imageAndText' && (s.imageLayout === 'top' || s.imageLayout === 'bottom');
    w = (stacked ? 130 : 170) * scale * (p.scale / 100);
    h = (stacked ? 66 : 30) * scale * (p.scale / 100);
  }
  w = Math.min(w, W);
  h = Math.min(h, H);
  const mx = p.marginMm.x * PT_PER_MM;
  const my = p.marginMm.y * PT_PER_MM;
  const col = p.anchor[1];
  let x = col === 'l' ? mx : col === 'c' ? (W - w) / 2 : W - w - mx;
  let y = p.anchor[0] === 't' ? my : H - h - my;
  x = Math.min(Math.max(x, 0), W - w);
  y = Math.min(Math.max(y, 0), H - h);
  return { w: (w / W) * 100, h: (h / H) * 100, x: (x / W) * 100, y: (y / H) * 100, pageW: W, pageH: H };
}

export function profileSummary(p: SignatureProfile): string {
  return `${p.paper} ${p.orientation === 'landscape' ? 'ngang' : 'dọc'} · ${POS_LABEL[p.anchor]}`;
}

/**
 * Hộp Cấu hình (prototype `mCauHinh()` L1631–1638, mục 3.7): 3 tab Chữ ký theo khổ giấy · USB Token · Thư mục &
 * tên file. Sửa trên bản nháp; "Lưu cấu hình" gửi lõi làm sạch và ghi `config.json`; Hủy / ✕ bỏ bản nháp.
 */
@Component({
  selector: 'app-config-dialog',
  imports: [IconComponent, PopupComponent, TipComponent, AdvancedButtonComponent, PathInputComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './config-dialog.component.html',
})
export class ConfigDialogComponent {
  protected readonly ui = inject(UiService);
  protected readonly cfg = inject(ConfigService);
  private readonly ipc = inject(IpcService);

  protected readonly papers = PAPERS;
  protected readonly anchors = ANCHORS;
  protected readonly fonts = FONTS;
  protected readonly modes = MODES;
  protected readonly imageLayouts = IMAGE_LAYOUTS;
  protected readonly posLabel = POS_LABEL;
  protected readonly pageLabel = PAGE_LABEL;
  protected readonly pages: PagePick[] = ['first', 'last', 'all'];
  protected readonly paperMm = PAPER_MM;
  protected readonly summary = profileSummary;
  protected readonly baseName = baseName;

  protected readonly draft = signal<AppConfig | null>(null);
  protected readonly sel = signal(0);
  protected readonly saved = signal(false);
  protected readonly busy = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly advSig = signal(false);
  protected readonly advImg = signal(false);
  protected readonly advFolder = signal(false);
  protected readonly imageUrl = signal<string | null>(null);
  protected readonly certs = signal<CertScan | null>(null);
  protected readonly certBusy = signal(false);
  protected readonly certError = signal<string | null>(null);
  /** Thumbprint (chữ hoa) các chứng thư có PIN lưu trên máy. */
  protected readonly savedPins = signal<ReadonlySet<string>>(new Set());
  protected readonly pinBusy = signal(false);
  protected readonly pinError = signal<string | null>(null);

  /** Chứng thư hiện trong bảng: bỏ các chứng thư đã xoá khỏi danh sách (theo bản nháp). */
  protected readonly shownCerts = computed(() => {
    const hidden = new Set(this.draft()?.token?.hiddenCerts ?? []);
    return (this.certs()?.certificates ?? []).filter((c) => !hidden.has(c.thumbprint.toUpperCase()));
  });
  protected readonly hiddenCount = computed(() => this.draft()?.token?.hiddenCerts.length ?? 0);

  protected readonly open = computed(() => this.ui.dialog()?.kind === 'cauhinh');
  protected readonly tab = computed<ConfigTab>(() => {
    const d = this.ui.dialog();
    return d?.kind === 'cauhinh' ? d.tab : 'sig';
  });
  protected readonly profile = computed(() => {
    const d = this.draft();
    return d ? (d.signature.profiles[this.sel()] ?? d.signature.profiles[0]) : null;
  });
  protected readonly box = computed(() => {
    const d = this.draft();
    const p = this.profile();
    return d && p ? profileBox(p, d.signature) : null;
  });
  protected readonly miniSize = computed(() => {
    const b = this.box();
    if (!b) {
      return { w: 150, h: 212 };
    }
    // Vừa khung 180 × 220 px.
    const k = Math.min(180 / b.pageW, 220 / b.pageH);
    return { w: Math.round(b.pageW * k), h: Math.round(b.pageH * k) };
  });

  constructor() {
    // Mở hộp: lấy bản nháp từ cấu hình đang dùng.
    effect(() => {
      if (this.open()) {
        untracked(() => {
          if (!this.draft()) {
            const c = this.cfg.config();
            this.draft.set(c ? structuredClone(c) : null);
            const i = c?.signature.profiles.findIndex((p) => p.isDefault) ?? 0;
            this.sel.set(Math.max(i, 0));
            this.saved.set(false);
            this.error.set(null);
          }
        });
      } else {
        untracked(() => this.draft.set(null));
      }
    });
    // Cấu hình nạp xong khi hộp đã mở (lần đầu chạy app).
    effect(() => {
      const c = this.cfg.config();
      if (c && this.open()) {
        untracked(() => {
          if (!this.draft()) {
            this.draft.set(structuredClone(c));
          }
        });
      }
    });
    // Ảnh của bản nháp để vẽ trong trang mẫu.
    effect(() => {
      const path = this.draft()?.signature.imagePath ?? null;
      untracked(() => {
        this.imageUrl.set(null);
        if (path && this.ipc.available) {
          this.ipc.signImagePreview(path).then(
            (p) => this.draft()?.signature.imagePath === path && this.imageUrl.set(convertFileSrc(p)),
            () => undefined,
          );
        }
      });
    });
    effect(() => {
      if (this.open() && this.tab() === 'token') {
        // PIN có thể vừa được lưu / xoá ở hộp Xác nhận ký: đọc lại mỗi lần mở tab.
        untracked(() => (this.certs() ? void this.loadPins() : void this.scanCerts()));
      }
    });
  }

  protected setTab(tab: ConfigTab): void {
    this.ui.openConfig(tab);
  }

  // ---- Sửa bản nháp ----------------------------------------------------------------------------

  private edit(change: (c: AppConfig) => void): void {
    const d = this.draft();
    if (!d) {
      return;
    }
    const next = structuredClone(d);
    change(next);
    this.draft.set(next);
    this.saved.set(false);
  }

  protected setProfile(patch: Partial<SignatureProfile>): void {
    const i = this.sel();
    this.edit((c) => {
      const p = c.signature.profiles[i];
      if (p) {
        c.signature.profiles[i] = { ...p, ...patch };
      }
    });
  }

  protected setMargin(axis: 'x' | 'y', value: number): void {
    const p = this.profile();
    if (p && Number.isFinite(value)) {
      this.setProfile({ marginMm: { ...p.marginMm, [axis]: value } });
    }
  }

  protected setSig<K extends keyof AppConfig['signature']>(key: K, value: AppConfig['signature'][K]): void {
    this.edit((c) => (c.signature[key] = value));
  }

  protected setFont<K extends keyof AppConfig['signature']['font']>(key: K, value: AppConfig['signature']['font'][K]): void {
    this.edit((c) => (c.signature.font[key] = value));
  }

  protected setImage<K extends keyof AppConfig['signature']['image']>(key: K, value: AppConfig['signature']['image'][K]): void {
    this.edit((c) => (c.signature.image[key] = value));
  }

  protected setOut<K extends keyof AppConfig['output']>(key: K, value: AppConfig['output'][K]): void {
    this.edit((c) => (c.output[key] = value));
  }

  protected setDisplay(mode: SignatureMode): void {
    if ((mode === 'image' || mode === 'imageAndText') && !this.draft()?.signature.imagePath) {
      this.error.set('Chọn ảnh chữ ký / con dấu trước khi dùng kiểu hiển thị có ảnh.');
      return;
    }
    this.error.set(null);
    this.setSig('display', mode);
  }

  protected addProfile(): void {
    const p = this.profile();
    if (!p) {
      return;
    }
    this.edit((c) => {
      c.signature.profiles.push({ ...structuredClone(p), id: `mau-${Date.now()}`, name: 'Mẫu mới', isDefault: false });
    });
    this.sel.set((this.draft()?.signature.profiles.length ?? 1) - 1);
  }

  protected deleteProfile(): void {
    const i = this.sel();
    const d = this.draft();
    if (!d || d.signature.profiles.length <= 1 || d.signature.profiles[i]?.isDefault) {
      return;
    }
    this.edit((c) => c.signature.profiles.splice(i, 1));
    this.sel.set(Math.max(0, i - 1));
  }

  protected makeDefault(): void {
    const i = this.sel();
    this.edit((c) => c.signature.profiles.forEach((p, j) => (p.isDefault = j === i)));
  }

  /** Số file có khung riêng (chữ ký chính hoặc ký nháy). */
  protected ownCount(): number {
    const s = this.draft()?.signature;
    return new Set([...Object.keys(s?.filePositions ?? {}), ...Object.keys(s?.fileInitialPositions ?? {})]).size;
  }

  /**
   * Bỏ mọi vị trí tự đặt (dùng chung và riêng từng file): ký theo mẫu khổ giấy. Kiểu hiển thị riêng từng file
   * (nếu có) vẫn giữ và vẫn áp.
   */
  protected clearPosition(): void {
    this.edit((c) => {
      c.signature.positions = [];
      c.signature.initialPositions = [];
      c.signature.filePositions = {};
      c.signature.fileInitialPositions = {};
      c.signature.perFile = Object.keys(c.signature.fileDisplays ?? {}).length > 0 && c.signature.perFile;
    });
  }

  protected async pickImage(): Promise<void> {
    try {
      const path = await openDialog({
        directory: false,
        multiple: false,
        title: 'Chọn ảnh chữ ký / con dấu',
        filters: [{ name: 'Ảnh chữ ký, con dấu', extensions: ['png', 'jpg', 'jpeg'] }],
      });
      if (typeof path === 'string') {
        this.edit((c) => {
          c.signature.imagePath = path;
          if (c.signature.display === 'text') {
            c.signature.display = 'imageAndText';
          }
        });
      }
    } catch (error) {
      this.error.set(errorMessage(error));
    }
  }

  protected clearImage(): void {
    this.edit((c) => {
      c.signature.imagePath = null;
      if (c.signature.display === 'image' || c.signature.display === 'imageAndText') {
        c.signature.display = 'text';
      }
    });
  }

  protected async pickFolder(): Promise<void> {
    try {
      const path = await openDialog({ directory: true, multiple: false, title: 'Chọn thư mục lưu file đã ký' });
      if (typeof path === 'string') {
        this.edit((c) => {
          c.output.fixedFolder = path;
          c.output.mode = 'fixed';
        });
      }
    } catch (error) {
      this.error.set(errorMessage(error));
    }
  }

  protected async resetDefaults(): Promise<void> {
    try {
      const def = await this.cfg.defaults();
      this.draft.set(def);
      this.sel.set(Math.max(def.signature.profiles.findIndex((p) => p.isDefault), 0));
      this.saved.set(false);
    } catch (error) {
      this.error.set(errorMessage(error));
    }
  }

  protected async save(): Promise<void> {
    const d = this.draft();
    if (!d || this.busy()) {
      return;
    }
    this.busy.set(true);
    this.error.set(null);
    try {
      const res = await this.cfg.save(d);
      this.draft.set(structuredClone(res.config));
      this.saved.set(true);
      if (res.notes.length) {
        this.error.set(`Đã sửa cho hợp lệ: ${res.notes.join(' ')}`);
      }
    } catch (error) {
      this.error.set(errorMessage(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected async scanCerts(): Promise<void> {
    if (!this.ipc.available) {
      return;
    }
    this.certBusy.set(true);
    this.certError.set(null);
    try {
      void this.loadPins();
      this.certs.set(await this.ipc.certList());
    } catch (error) {
      this.certError.set(errorMessage(error));
    } finally {
      this.certBusy.set(false);
    }
  }

  private async loadPins(): Promise<void> {
    try {
      this.savedPins.set(new Set((await this.ipc.pinSaved()).map((t) => t.toUpperCase())));
    } catch (error) {
      this.pinError.set(errorMessage(error));
    }
  }

  protected hasPin(thumbprint: string): boolean {
    return this.savedPins().has(thumbprint.toUpperCase());
  }

  /** Xoá PIN đã lưu của chứng thư (ngay, không chờ "Lưu cấu hình"). */
  protected async forgetPin(thumbprint: string): Promise<void> {
    this.pinBusy.set(true);
    this.pinError.set(null);
    try {
      await this.ipc.pinForget(thumbprint);
    } catch (error) {
      this.pinError.set(errorMessage(error));
    } finally {
      await this.loadPins();
      this.pinBusy.set(false);
    }
  }

  /** Xoá chứng thư khỏi danh sách của ứng dụng (bản nháp; lưu khi bấm "Lưu cấu hình"). Kho Windows không đổi. */
  protected hideCert(thumbprint: string): void {
    const t = thumbprint.toUpperCase();
    this.edit((c) => {
      c.token ??= { hiddenCerts: [] };
      if (!c.token.hiddenCerts.includes(t)) {
        c.token.hiddenCerts.push(t);
      }
    });
  }

  protected unhideAll(): void {
    this.edit((c) => (c.token = { hiddenCerts: [] }));
  }

  protected close(): void {
    this.ui.close();
  }

  // ---- Đọc giá trị ô nhập -----------------------------------------------------------------------

  protected val(e: Event): string {
    return (e.target as HTMLInputElement | HTMLSelectElement).value;
  }

  protected num(e: Event): number {
    return Number((e.target as HTMLInputElement).value);
  }

  protected checked(e: Event): boolean {
    return (e.target as HTMLInputElement).checked;
  }

  protected fmtDate(unixSeconds: number): string {
    const d = new Date(unixSeconds * 1000);
    return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')}/${d.getFullYear()}`;
  }

  protected pct(v: number): number {
    return Math.round(v * 100);
  }
}
