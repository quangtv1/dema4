import { Injectable, computed, effect, inject, signal, untracked } from '@angular/core';

import { ConfigService } from '../core/config.service';
import {
  Anchor,
  FileLayout,
  FileLayoutItem,
  ImageLayout,
  IpcService,
  PartBox,
  PreviewResult,
  SignAppearance,
  SignPlacement,
  SignatureMode,
  SignatureProfile,
  errorMessage,
} from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { KySoStore, isImageMode } from '../ky-so/ky-so.store';
import { clamp } from '../shared/text';

/** Chế độ của màn (prototype `S.tc.mode`): một khung cho mọi file, hoặc khung riêng từng file. */
export type TcMode = 'all' | 'per';

const round4 = (v: number) => Math.round(v * 10000) / 10000;

/**
 * Màn "Xem trước & vị trí chữ ký" (prototype `manTC()` L1384–1423): đặt khung chữ ký cho cả lô hoặc riêng từng
 * file của màn Ký số. Sửa trên bản nháp; "Áp dụng" mới ghi vào `config.json` (`signature.positions`,
 * `filePositions`, `display`, `fileDisplays`, `perFile`), "Thoát" bỏ thay đổi. Chỗ khung do lõi tính (`sign_layouts`) nên trùng khi ký.
 * Màn này đặt **khung chữ ký chính**; một văn bản có thể có nhiều khung trên nhiều trang (TD-55: chuột phải trên trang
 * hoặc ảnh thu nhỏ để thêm / xoá, như màn Ký số). Khung ký nháy đặt ở màn Ký số.
 */
@Injectable({ providedIn: 'root' })
export class ViTriStore {
  private readonly ipc = inject(IpcService);
  private readonly cfg = inject(ConfigService);
  private readonly ui = inject(UiService);
  readonly ks = inject(KySoStore);

  // ---- Bản nháp ----------------------------------------------------------------------------
  readonly mode = signal<TcMode>('all');
  /** Khung dùng chung; rỗng: theo mẫu khổ giấy. */
  readonly frames = signal<SignPlacement[]>([]);
  /** Khung đầu tiên của danh sách dùng chung (màn này sửa một khung). */
  readonly frame = computed<SignPlacement | null>(() => this.frames()[0] ?? null);
  /** Khung riêng từng file (đường dẫn → khung), kể cả file không nằm trong lô đang nạp. */
  readonly perLists = signal<Record<string, SignPlacement[]>>({});
  readonly per = computed<Record<string, SignPlacement>>(() =>
    Object.fromEntries(Object.entries(this.perLists()).flatMap(([p, l]) => (l[0] ? [[p, l[0]] as const] : []))),
  );
  /** Kiểu hiển thị dùng chung. */
  readonly display = signal<SignatureMode>('text');
  /** Kiểu hiển thị riêng từng file (đường dẫn → kiểu). */
  readonly disp = signal<Record<string, SignatureMode>>({});
  /** Kiểu "Ảnh + thông tin" (TD-51): ảnh ở đâu so với chữ, phần dành cho ảnh và khoảng cách (%; null = tự động). */
  readonly imageLayout = signal<ImageLayout>('left');
  readonly imageShare = signal<number | null>(null);
  readonly imageGap = signal<number | null>(null);
  /** Kiểu "Tự do" (TD-52): ô ảnh và ô chữ kéo thả trong khối; null = theo Vị trí ảnh. */
  readonly freeBoxes = signal<{ image: PartBox; text: PartBox } | null>(null);
  /** Văn bản "chưa gán chữ ký" (TD-56): đã xoá hết khung, bỏ qua khi ký số. */
  readonly skips = signal<ReadonlySet<string>>(new Set());
  readonly dirty = signal(false);

  // ---- Hiển thị ----------------------------------------------------------------------------
  readonly selectedPath = signal<string | null>(null);
  readonly zoom = signal(66);
  readonly leftW = signal(480);
  readonly advOpen = signal(false);
  /** Mẫu chọn ở "Theo mẫu" (Nâng cao); null: mẫu mặc định. */
  readonly profileId = signal<string | null>(null);
  /** "Vị trí theo mẫu" (Nâng cao); null: điểm neo của mẫu. */
  readonly anchor = signal<Anchor | null>(null);

  readonly layouts = signal<Record<string, FileLayoutItem>>({});
  readonly layoutError = signal<string | null>(null);
  private layoutSeq = 0;

  readonly preview = signal<PreviewResult | null>(null);
  readonly previewBusy = signal(false);
  readonly previewError = signal<string | null>(null);
  private previewSeq = 0;
  private previewPath: string | null = null;

  // ---- Suy ra ------------------------------------------------------------------------------
  readonly files = this.ks.files;
  readonly locked = computed(() => this.ks.locked());

  readonly currentIndex = computed(() => {
    const list = this.files();
    const i = list.findIndex((f) => f.path === this.selectedPath());
    return i >= 0 ? i : 0;
  });
  readonly current = computed(() => this.files()[this.currentIndex()] ?? null);

  readonly profiles = computed<SignatureProfile[]>(() => this.cfg.config()?.signature.profiles ?? []);
  readonly profile = computed<SignatureProfile | null>(() => {
    const list = this.profiles();
    return list.find((p) => p.id === this.profileId()) ?? list.find((p) => p.isDefault) ?? list[0] ?? null;
  });
  readonly anchorShown = computed<Anchor>(() => this.anchor() ?? this.profile()?.anchor ?? 'br');

  /** Kiểu hiển thị của file đang xem theo bản nháp. */
  readonly curDisplay = computed<SignatureMode>(() => {
    const cur = this.current();
    return cur ? this.displayOf(cur.path) : this.display();
  });

  /** Kiểu hiển thị của khung: "Ẩn" vẫn vẽ khung để tham khảo (như prototype), theo kiểu có ảnh nếu đã chọn ảnh. */
  readonly boxMode = computed<Exclude<SignatureMode, 'hidden'>>(() => this.shown(this.curDisplay()));

  /** Cấu hình hiển thị gửi lõi để tính chỗ khung (như khi ký, trừ kiểu "Ẩn"). */
  private readonly appearance = computed<SignAppearance>(() => ({
    ...this.ks.appearance(),
    mode: this.shown(this.display()),
    kind: 'main',
    placements: this.frames(),
    imageLayout: this.imageLayout(),
    imageShare: this.imageShare(),
    imageGap: this.imageGap(),
    imageBox: this.freeBoxes()?.image ?? null,
    textBox: this.freeBoxes()?.text ?? null,
  }));

  /** Kiểu riêng từng file gửi lõi khi tính chỗ khung ("Ẩn" tính như kiểu vẽ khung tham khảo). */
  private readonly modes = computed<Record<string, SignatureMode>>(() =>
    this.mode() === 'per' ? Object.fromEntries(Object.entries(this.disp()).map(([p, m]) => [p, this.shown(m)] as const)) : {},
  );

  /** Số file trong lô có cấu hình riêng (khung hoặc kiểu hiển thị). */
  readonly ownCount = computed(() => {
    if (this.mode() !== 'per') {
      return 0;
    }
    const per = this.per();
    const disp = this.disp();
    return this.files().filter((f) => per[f.path] || disp[f.path]).length;
  });

  readonly curLayout = computed<FileLayout | null>(() => {
    const cur = this.current();
    return cur ? (this.layouts()[cur.path]?.layout ?? null) : null;
  });
  readonly curError = computed(() => {
    const cur = this.current();
    return cur ? (this.layouts()[cur.path]?.error ?? null) : null;
  });
  readonly curBox = computed(() => (this.curSkipped() ? null : (this.curLayout()?.boxes[0] ?? null)));
  private readonly tplBox = computed(() => this.curLayout()?.template[0] ?? null);

  /** Khung của file đang xem có vị trí tự đặt (riêng hoặc dùng chung). */
  readonly curManual = computed(() => {
    const cur = this.current();
    return !!cur && (!!this.placementOf(cur.path) || this.skips().has(cur.path));
  });

  /** Cỡ khung so với cỡ theo mẫu (thanh Cỡ, prototype `co`). */
  readonly sizePct = computed(() => {
    const box = this.curBox();
    const tpl = this.tplBox();
    return box && tpl && tpl.widthRatio > 0 ? Math.round((box.widthRatio / tpl.widthRatio) * 100) : 100;
  });
  /** Kiểu chỉ có chữ: cỡ khối theo cỡ chữ, khung chỉ để neo chỗ (lõi đo chữ), nên khoá thanh Cỡ. */
  readonly textOnly = computed(() => this.boxMode() === 'text');

  constructor() {
    // Chỗ khung của mọi file trong lô (ảnh thu nhỏ + bản xem trước), tính lại khi bản nháp đổi.
    effect((onCleanup) => {
      const paths = this.files()
        .filter((f) => !f.unreadable)
        .map((f) => f.path);
      const appearance = this.appearance();
      const placements = this.mode() === 'per' ? this.perLists() : {};
      const modes = this.modes();
      const signer = this.ks.signerName() ?? '[Tên chứng thư số]';
      const seq = ++this.layoutSeq;
      if (!paths.length || !this.ipc.available || this.ui.screen() !== 'tucauhinh') {
        return;
      }
      const timer = setTimeout(() => {
        this.ipc.signLayouts(paths, appearance, placements, modes, signer).then(
          (list) => {
            if (seq === this.layoutSeq) {
              this.layouts.set(Object.fromEntries(list.map((it) => [it.path, it])));
              this.layoutError.set(null);
            }
          },
          (error) => seq === this.layoutSeq && this.layoutError.set(errorMessage(error)),
        );
      }, 120);
      onCleanup(() => clearTimeout(timer));
    });
    // Bản xem trước của file đang xem.
    effect((onCleanup) => {
      const cur = this.current();
      const path = cur && !cur.unreadable && this.ipc.available && this.ui.screen() === 'tucauhinh' ? cur.path : null;
      if (path === this.previewPath) {
        return;
      }
      const seq = ++this.previewSeq;
      if (!path) {
        untracked(() => {
          this.previewPath = null;
          this.preview.set(null);
          this.previewBusy.set(false);
          this.previewError.set(cur?.unreadable ? 'Không đọc được file này.' : null);
        });
        return;
      }
      untracked(() => this.previewBusy.set(true));
      const timer = setTimeout(() => {
        this.ipc.signPreview(path).then(
          (pdfPath) => {
            if (seq === this.previewSeq) {
              this.previewPath = path;
              this.preview.set({ pdfPath, pageCount: 0, warnings: [] });
              this.previewError.set(null);
              this.previewBusy.set(false);
            }
          },
          (error) => {
            if (seq === this.previewSeq) {
              this.previewPath = null;
              this.preview.set(null);
              this.previewError.set(errorMessage(error));
              this.previewBusy.set(false);
            }
          },
        );
      }, 60);
      onCleanup(() => clearTimeout(timer));
    });
  }

  /** Mở màn: nạp bản nháp từ cấu hình đang dùng, xem đúng file đang chọn ở màn Ký số. */
  enter(): void {
    const s = this.cfg.config()?.signature;
    this.frames.set([...(s?.positions ?? [])]);
    this.perLists.set({ ...(s?.filePositions ?? {}) });
    this.display.set(s?.display ?? 'text');
    this.disp.set({ ...(s?.fileDisplays ?? {}) });
    this.imageLayout.set(s?.imageLayout ?? 'left');
    this.imageShare.set(s?.imageShare ?? null);
    this.imageGap.set(s?.imageGap ?? null);
    this.freeBoxes.set(s?.imageBox && s?.textBox ? { image: s.imageBox, text: s.textBox } : null);
    this.skips.set(new Set(s?.skipFiles ?? []));
    this.mode.set(s?.perFile ? 'per' : 'all');
    this.dirty.set(false);
    this.anchor.set(null);
    this.profileId.set(null);
    this.layouts.set({});
    this.layoutError.set(null);
    this.previewPath = null;
    this.selectedPath.set(this.ks.current()?.path ?? this.files()[0]?.path ?? null);
  }

  /** Vị trí tự đặt của một file theo bản nháp; null: theo mẫu. */
  placementOf(path: string): SignPlacement | null {
    return this.mode() === 'per' ? (this.per()[path] ?? this.frame()) : this.frame();
  }

  /** Kiểu hiển thị của một file theo bản nháp: kiểu riêng (chế độ "Từng file"), không có thì kiểu chung. */
  displayOf(path: string): SignatureMode {
    return (this.mode() === 'per' ? this.disp()[path] : undefined) ?? this.display();
  }

  /** File có cấu hình riêng (khung hoặc kiểu hiển thị) trong chế độ "Từng file". */
  isOwn(path: string): boolean {
    return this.mode() === 'per' && (!!this.per()[path] || !!this.disp()[path]);
  }

  /** File có khung riêng. */
  hasOwnFrame(path: string): boolean {
    return this.mode() === 'per' && !!this.per()[path];
  }

  private shown(m: SignatureMode): Exclude<SignatureMode, 'hidden'> {
    return m === 'hidden' ? (this.ks.hasImage() ? 'imageAndText' : 'text') : m;
  }

  setMode(mode: TcMode): void {
    this.mode.set(mode);
    this.dirty.set(true);
  }

  select(path: string): void {
    this.selectedPath.set(path);
  }

  nav(step: number): void {
    const list = this.files();
    if (list.length) {
      this.selectedPath.set(list[clamp(this.currentIndex() + step, 0, list.length - 1)].path);
    }
  }

  zoomBy(step: number): void {
    this.zoom.update((z) => clamp(z + step, 40, 160));
  }

  /** Văn bản đang xem chưa gán chữ ký (đã xoá hết khung). */
  readonly curSkipped = computed(() => {
    const cur = this.current();
    return !!cur && this.skips().has(cur.path);
  });
  /** Các khung đang hiện của file đang xem (trang thật, từ 1): gốc để sửa. Chưa gán chữ ký: không có khung. */
  readonly curBoxes = computed<SignPlacement[]>(() => (this.curSkipped() ? [] : (this.curLayout()?.boxes ?? [])));

  private setSkip(path: string, on: boolean): void {
    const next = new Set(this.skips());
    if (on) {
      next.add(path);
    } else {
      next.delete(path);
    }
    this.skips.set(next);
    this.dirty.set(true);
  }

  /**
   * Văn bản chưa gán chữ ký được thêm khung: bỏ đánh dấu, khung mới là khung duy nhất của văn bản (khung riêng;
   * đang "Tất cả file" thì chuyển sang "Từng file", các văn bản khác vẫn theo khung chung như cũ).
   */
  private assignFirst(box: SignPlacement): void {
    const cur = this.current();
    if (!cur) {
      return;
    }
    this.setSkip(cur.path, false);
    this.mode.set('per');
    this.writeBoxes([box]);
  }
  /** Trang (từ 1) có khung chữ ký của file đang xem, để đánh dấu ảnh thu nhỏ. */
  readonly boxPages = computed<ReadonlySet<number>>(() => new Set(this.curBoxes().map((b) => b.page)));

  /**
   * Ghi danh sách khung (trang thật) cho file đang xem: chế độ "Từng file" thì riêng file đó, ngược lại dùng chung
   * cho mọi file (file ít trang hơn thì khung ở trang lớn hơn rơi về trang cuối, lõi gộp khung trùng). Trang cuối
   * của file nhiều trang lưu là 0 ("trang cuối") để áp đúng cho file có số trang khác. Hiện ngay, không chờ lõi.
   */
  private writeBoxes(list: SignPlacement[]): void {
    const cur = this.current();
    if (!cur || this.locked() || !list.length) {
      return;
    }
    const pages = this.curLayout()?.pageCount ?? 0;
    const shown = list.map((p) => ({
      page: p.page,
      xRatio: round4(p.xRatio),
      yRatio: round4(p.yRatio),
      widthRatio: round4(p.widthRatio),
      heightRatio: round4(p.heightRatio),
    }));
    const stored = shown.map((p) => ({ ...p, page: pages > 1 && p.page >= pages ? 0 : p.page }));
    if (this.mode() === 'per') {
      this.perLists.update((m) => ({ ...m, [cur.path]: stored }));
    } else {
      this.frames.set(stored);
    }
    this.dirty.set(true);
    this.layouts.update((all) => {
      const it = all[cur.path];
      return it?.layout ? { ...all, [cur.path]: { ...it, layout: { ...it.layout, boxes: shown } } } : all;
    });
  }

  /** Ghi cả danh sách khung (trang thật) cho file đang xem. */
  setBoxes(list: SignPlacement[]): void {
    this.writeBoxes(list);
  }

  /** Kéo / đổi cỡ khung thứ `index` của file đang xem (chưa có khung nào thì thành khung đầu tiên). */
  setBox(index: number, p: SignPlacement): void {
    const list = this.curBoxes();
    this.writeBoxes(list.length ? list.map((b, i) => (i === index ? p : b)) : [p]);
  }

  /** Đặt một khung duy nhất cho file đang xem (prototype `tcSet`). */
  setCurrent(p: SignPlacement): void {
    this.writeBoxes([p]);
  }

  /** Sửa mọi khung của file đang xem bằng cùng một phép (đổi cỡ cả khối ở ô sửa, thanh Cỡ). */
  editAll(fn: (b: SignPlacement) => SignPlacement): void {
    const list = this.curBoxes();
    if (list.length) {
      this.writeBoxes(list.map(fn));
    }
  }

  /** Thêm khung vào trang `page`, tâm ở (`xRatio`, `yRatio`); cỡ theo khung đang có (ưu tiên cùng trang). */
  addBox(page: number, xRatio: number, yRatio: number): void {
    if (this.locked()) {
      return;
    }
    const list = this.curBoxes();
    const like = list.find((b) => b.page === page) ?? list[0] ?? this.curLayout()?.boxes[0];
    const w = like?.widthRatio ?? 0.29;
    const h = like?.heightRatio ?? 0.06;
    const box = { page, xRatio: clamp(xRatio - w / 2, 0, 1 - w), yRatio: clamp(yRatio - h / 2, 0, 1 - h), widthRatio: w, heightRatio: h };
    if (this.curSkipped()) {
      this.assignFirst(box);
    } else {
      this.writeBoxes([...list, box]);
    }
  }

  /** Thêm khung vào trang `page` ở đúng chỗ của khung đầu tiên (ảnh thu nhỏ › Thêm khung vào trang này). */
  addBoxToPage(page: number): void {
    if (this.locked()) {
      return;
    }
    if (this.curSkipped()) {
      // Chỗ và cỡ theo khung lõi tính sẵn (khung chung / theo mẫu).
      const src = this.curLayout()?.boxes[0];
      if (src) {
        this.assignFirst({ ...src, page });
      }
      return;
    }
    const src = this.curBoxes()[0];
    if (src) {
      this.writeBoxes([...this.curBoxes(), { ...src, page }].sort((a, b) => a.page - b.page));
    }
  }

  /**
   * Xoá khung thứ `index`. Xoá khung cuối cùng thì văn bản thành "chưa gán chữ ký" (TD-56): bỏ qua khi ký số; danh
   * sách khung (chung / riêng) giữ nguyên để thêm lại không mất.
   */
  deleteBox(index: number): boolean {
    const list = this.curBoxes();
    const cur = this.current();
    if (!cur || this.locked() || index < 0 || index >= list.length) {
      return false;
    }
    if (list.length === 1) {
      this.setSkip(cur.path, true);
    } else {
      this.writeBoxes(list.filter((_, i) => i !== index));
    }
    return true;
  }

  /** Xoá mọi khung ở trang `page` (hết khung thì văn bản chưa gán chữ ký). */
  deletePage(page: number): boolean {
    const list = this.curBoxes();
    const cur = this.current();
    const next = list.filter((b) => b.page !== page);
    if (!cur || this.locked() || next.length === list.length) {
      return false;
    }
    if (next.length) {
      this.writeBoxes(next);
    } else {
      this.setSkip(cur.path, true);
    }
    return true;
  }

  /** Xoá hết khung của văn bản đang xem: chưa gán chữ ký. */
  deleteAll(): void {
    const cur = this.current();
    if (cur && !this.locked() && !this.curSkipped()) {
      this.setSkip(cur.path, true);
    }
  }

  /** Chép khung thứ `index` sang mọi trang chưa có khung (cùng chỗ trên trang). */
  copyBoxToAllPages(index: number): void {
    const list = this.curBoxes();
    const src = list[index];
    const pages = this.curLayout()?.pageCount ?? 0;
    if (!src || pages < 2) {
      return;
    }
    const used = new Set(list.map((b) => b.page));
    const added: SignPlacement[] = [];
    for (let n = 1; n <= pages; n++) {
      if (!used.has(n)) {
        added.push({ ...src, page: n });
      }
    }
    if (added.length) {
      this.writeBoxes([...list, ...added].sort((a, b) => a.page - b.page));
    }
  }

  /** Thanh Cỡ: đổi cỡ mọi khung theo phần trăm cỡ của mẫu, giữ góc trên trái (prototype `tc.size`). */
  setSize(pct: number): void {
    const tpl = this.tplBox();
    if (!this.curBox() || !tpl || this.textOnly()) {
      return;
    }
    const k = clamp(pct, 50, 200) / 100;
    const w = Math.min(1, tpl.widthRatio * k);
    const h = Math.min(1, tpl.heightRatio * k);
    this.editAll((b) => ({ ...b, xRatio: clamp(b.xRatio, 0, 1 - w), yRatio: clamp(b.yRatio, 0, 1 - h), widthRatio: w, heightRatio: h }));
  }

  /**
   * Đặt lại (prototype `tc.reset`): bỏ cấu hình riêng (khung, kiểu hiển thị) của file đang xem, hoặc bỏ khung
   * dùng chung để về theo mẫu.
   */
  reset(): void {
    const cur = this.current();
    if (this.locked()) {
      return;
    }
    if (cur && this.skips().has(cur.path)) {
      this.setSkip(cur.path, false);
      return;
    }
    if (cur && this.isOwn(cur.path)) {
      const drop = <T>(m: Record<string, T>) => {
        const next = { ...m };
        delete next[cur.path];
        return next;
      };
      this.perLists.update(drop);
      this.disp.update(drop);
    } else {
      this.frames.set([]);
    }
    this.dirty.set(true);
  }

  /** Nâng cao › Theo mẫu / Vị trí theo mẫu: lấy chỗ và cỡ của mẫu cho khung hiện tại, sau đó vẫn kéo chỉnh được. */
  async applyProfile(profileId: string | null, anchor: Anchor | null): Promise<void> {
    this.profileId.set(profileId);
    this.anchor.set(anchor);
    const cur = this.current();
    const profile = this.profile();
    if (!cur || !profile || this.locked() || !this.ipc.available) {
      return;
    }
    // Chỉ một mẫu, đặt làm mặc định: lõi chọn đúng mẫu này bất kể khổ giấy của file.
    const only: SignatureProfile = { ...profile, isDefault: true, anchor: anchor ?? profile.anchor };
    try {
      const [it] = await this.ipc.signLayouts(
        [cur.path],
        { ...this.appearance(), mode: this.boxMode(), placements: [], profiles: [only] },
        {},
        {},
        this.ks.signerName() ?? '[Tên chứng thư số]',
      );
      const box = it?.layout?.template[0];
      if (box && this.current()?.path === cur.path) {
        // Khung đầu tiên lấy chỗ và cỡ theo mẫu; các khung khác (trang khác) giữ nguyên.
        this.setBox(0, box);
      } else if (it?.error) {
        this.layoutError.set(it.error);
      }
    } catch (error) {
      this.layoutError.set(errorMessage(error));
    }
  }

  /** Ô "Hiển thị": chế độ "Từng file" thì đổi kiểu riêng của file đang xem, ngược lại đổi kiểu chung. */
  setDisplay(mode: SignatureMode): void {
    const cur = this.current();
    if (this.locked() || (isImageMode(mode) && !this.ks.hasImage())) {
      return;
    }
    if (this.mode() === 'per' && cur) {
      this.disp.update((m) => ({ ...m, [cur.path]: mode }));
    } else {
      this.display.set(mode);
    }
    this.dirty.set(true);
  }

  /** Ô "Vị trí ảnh" (kiểu "Ảnh + thông tin", áp cho mọi văn bản). */
  setImageLayout(layout: ImageLayout): void {
    if (this.locked()) {
      return;
    }
    this.imageLayout.set(layout);
    this.freeBoxes.set(null);
    this.dirty.set(true);
  }

  /**
   * Nút "Tự do": bắt đầu từ cách chia đang thấy (Vị trí ảnh, Ảnh chiếm, Khoảng cách), sau đó kéo thả trong ô sửa.
   */
  startFree(): void {
    if (this.locked() || this.freeBoxes()) {
      return;
    }
    const lay = this.imageLayout();
    const stacked = lay === 'top' || lay === 'bottom';
    const s = (this.imageShare() ?? (stacked ? 55 : 35)) / 100;
    const g = (this.imageGap() ?? (stacked ? 4 : 3)) / 100;
    const rest = Math.max(0.05, 1 - s - g);
    const boxes: Record<typeof lay, { image: PartBox; text: PartBox }> = {
      left: { image: { x: 0, y: 0, w: s, h: 1 }, text: { x: 1 - rest, y: 0, w: rest, h: 1 } },
      right: { image: { x: 1 - s, y: 0, w: s, h: 1 }, text: { x: 0, y: 0, w: rest, h: 1 } },
      top: { image: { x: 0, y: 0, w: 1, h: s }, text: { x: 0, y: 1 - rest, w: 1, h: rest } },
      bottom: { image: { x: 0, y: 1 - s, w: 1, h: s }, text: { x: 0, y: 0, w: 1, h: rest } },
    };
    this.freeBoxes.set(boxes[lay]);
    this.dirty.set(true);
  }

  /** Kéo thả trong ô sửa: đặt lại một ô (đã giới hạn trong khối, cạnh tối thiểu 3%). */
  setFreeBox(part: 'image' | 'text', b: PartBox): void {
    const cur = this.freeBoxes();
    if (this.locked() || !cur) {
      return;
    }
    const r4 = (v: number) => Math.round(v * 10000) / 10000;
    const w = clamp(b.w, 0.03, 1);
    const h = clamp(b.h, 0.03, 1);
    const box = { x: r4(clamp(b.x, 0, 1 - w)), y: r4(clamp(b.y, 0, 1 - h)), w: r4(w), h: r4(h) };
    this.freeBoxes.set({ ...cur, [part]: box });
    this.dirty.set(true);
  }

  /** Thanh "Ảnh chiếm" / "Khoảng cách": số % (đã giới hạn), null = tự động như trước. */
  setImageSplit(key: 'imageShare' | 'imageGap', value: number | null): void {
    if (this.locked()) {
      return;
    }
    const v = value == null || !Number.isFinite(value) ? null : Math.round(key === 'imageShare' ? clamp(value, 10, 90) : clamp(value, 0, 30));
    this[key].set(v);
    if (value == null) {
      this.freeBoxes.set(null);
    }
    this.dirty.set(true);
  }

  /** Áp dụng: ghi bản nháp vào cấu hình rồi về màn Ký số, đang xem đúng file vừa chỉnh. */
  apply(): void {
    if (this.locked()) {
      return;
    }
    const positions = this.frames();
    const filePositions = this.perLists();
    const perFile = this.mode() === 'per';
    const display = this.display();
    const fileDisplays = this.disp();
    const imageLayout = this.imageLayout();
    const imageShare = this.imageShare();
    const imageGap = this.imageGap();
    const imageBox = this.freeBoxes()?.image ?? null;
    const textBox = this.freeBoxes()?.text ?? null;
    const skipFiles = [...this.skips()];
    this.cfg.patch((c) => ({
      ...c,
      signature: {
        ...c.signature,
        skipFiles,
        positions,
        filePositions,
        display,
        fileDisplays,
        perFile,
        imageLayout,
        imageShare,
        imageGap,
        imageBox,
        textBox,
      },
    }));
    const cur = this.current();
    if (cur) {
      this.ks.selectedPath.set(cur.path);
    }
    this.dirty.set(false);
    this.ui.go('kyso');
  }

  /** Thoát / Quay lại: bỏ bản nháp. */
  exit(): void {
    this.dirty.set(false);
    this.ui.go('kyso');
  }
}
