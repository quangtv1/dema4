import { Injectable, computed, effect, inject, signal, untracked } from '@angular/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import {
  CertEntry,
  CertScan,
  IpcService,
  PIN_ERROR_CODES,
  PlannedOutput,
  PreviewResult,
  ImageLayout,
  SignAppearance,
  SignKind,
  SignEvent,
  SignFileItem,
  SignJobRequest,
  SignPlacement,
  SignatureMode,
  TokenInfo,
  Verdict,
  errorMessage,
} from '../core/ipc.service';
import { ConfigService } from '../core/config.service';
import { UiService } from '../core/ui.service';
import type { SigBoxView } from '../tao-van-ban/pdf-view.component';
import { baseName, clamp, extName, searchKey } from '../shared/text';

export type KsView = 'files' | 'log';
export type KsStatus = 'cho' | 'dang' | 'ok' | 'loi' | 'dung' | 'bo';
export type KsFilter = 'all' | KsStatus;

/** Nhãn trạng thái (prototype `STATUS_LABEL`). */
export const STATUS_LABEL: Record<KsStatus, string> = {
  ok: 'Đã ký',
  loi: 'Lỗi',
  dang: 'Đang ký',
  cho: 'Chờ ký',
  dung: 'Tạm dừng',
  bo: 'Chưa gán chữ ký',
};

/** Thứ tự mục của menu lọc (prototype `FILTS`). */
export const FILTERS: { id: KsFilter; label: string }[] = [
  { id: 'all', label: 'Tất cả' },
  { id: 'cho', label: 'Chờ ký' },
  { id: 'dung', label: 'Tạm dừng' },
  { id: 'ok', label: 'Đã ký' },
  { id: 'loi', label: 'Lỗi' },
  { id: 'dang', label: 'Đang ký' },
  { id: 'bo', label: 'Chưa gán chữ ký' },
];

export const MODE_LABEL: Record<SignatureMode, string> = {
  imageAndText: 'Ảnh + thông tin',
  text: 'Chỉ thông tin',
  image: 'Chỉ ảnh',
  hidden: 'Ẩn',
};

export const VERDICT_LABEL: Record<Verdict, string> = {
  notApplicable: '',
  unsigned: 'Chưa có chữ ký số',
  validTrusted: 'Hợp lệ',
  revocationUnknown: 'Hợp lệ, chưa kiểm tra được thu hồi',
  timeMismatch: 'Thời gian ký bất thường',
  noTimestamp: 'Không có dấu thời gian',
  untrustedTsa: 'Dấu thời gian không tin cậy',
  unsupported: 'Không xác định (thuật toán chưa hỗ trợ)',
  untrustedSigner: 'Người ký không thuộc CA Ban Cơ yếu',
  modifiedAfterSigning: 'Văn bản bị sửa sau khi ký',
  invalidSignature: 'Chữ ký không hợp lệ',
};

export const IMAGE_EXTS = ['png', 'jpg', 'jpeg'];


const SOURCE_OPEN_KEY = 'smks.ks.moKhungNguon';
const LAST_CERT_KEY = 'smks.ks.chungThu';

/** Thời điểm ký hiện trong khung chữ ký xem trước (dd/MM/yyyy HH:mm). */
/** Kiểu hiển thị có ảnh (cần ảnh dấu / chữ ký tươi). */
export function isImageMode(mode: SignatureMode): boolean {
  return mode === 'image' || mode === 'imageAndText';
}

export function fmtSignTime(d: Date): string {
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export interface FileResult {
  success: boolean;
  outputPath: string | null;
  reason: string | null;
  verdict: Verdict | null;
  trustedTime: string | null;
  /** Chưa ký vì lô tạm dừng (không phải lỗi). */
  paused: boolean;
}

/** Một lượt `sign_run` (Ký số / Ký tiếp / Ký lại). */
export interface KsJob {
  jobId: string | null;
  running: boolean;
  stopping: boolean;
  startedAt: number;
  finishedAt: number | null;
  /** Đường dẫn gửi trong `request.files`; `index` của sự kiện trỏ vào đây. */
  files: string[];
  /** Số file của lượt này đã có kết quả. */
  done: number;
  outputFolder: string;
  /** Đường dẫn file đã ký đầu tiên (để Mở thư mục khi lõi không trả thư mục chung). */
  firstOutput: string | null;
  summary: Extract<SignEvent, { type: 'summary' }> | null;
  error: string | null;
}

export type LogTag = 'INFO' | 'TOKEN' | 'OK' | 'LỖI' | 'XONG';

export interface LogLine {
  id: number;
  time: string;
  tag: LogTag;
  text: string;
}

function readStr(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStr(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Không ghi được: chỉ mất phần ghi nhớ.
  }
}

/** Vị trí ảnh so với chữ: nhãn ngắn và mô tả. */
export const IMAGE_LAYOUTS: { id: ImageLayout; label: string; title: string }[] = [
  { id: 'left', label: 'Trái', title: 'Ảnh bên trái, thông tin bên phải' },
  { id: 'right', label: 'Phải', title: 'Thông tin bên trái, ảnh bên phải' },
  { id: 'top', label: 'Trên', title: 'Ảnh ở trên, thông tin ở dưới' },
  { id: 'bottom', label: 'Dưới', title: 'Thông tin ở trên, ảnh ở dưới' },
];

const round4 = (v: number) => Math.round(v * 10000) / 10000;

function pad(n: number): string {
  return n < 10 ? '0' + n : String(n);
}

function clock(d = new Date()): string {
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** Ngày dd/MM/yyyy từ giây unix. */
export function fmtDate(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
}

/** Thư mục chứa một đường dẫn Windows / POSIX. */
function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i > 0 ? path.slice(0, i) : path;
}

/**
 * Trạng thái và nghiệp vụ của màn Ký số. Service gốc: giữ nguyên khi đổi màn rồi quay lại, lô đang ký không mất.
 * Ứng dụng không bao giờ hỏi hay lưu mã PIN: middleware của token tự hiện hộp nhập PIN khi `token_check` mở khoá.
 */
@Injectable({ providedIn: 'root' })
export class KySoStore {
  private readonly ipc = inject(IpcService);
  private readonly ui = inject(UiService);

  readonly available = this.ipc.available;

  // ---- Đầu vào ------------------------------------------------------------------------------
  /** Đường dẫn hiện ở ô "Văn bản cần ký". */
  readonly sourcePath = signal('');
  /** Kiểu nguồn đã nạp (tô nút "1 file" / "Thư mục"); null khi chưa nạp hoặc thả nhiều mục. */
  readonly sourceKind = signal<'file' | 'folder' | null>(null);
  readonly files = signal<SignFileItem[]>([]);
  readonly listBusy = signal(false);
  readonly loadError = signal<string | null>(null);

  /** Ảnh chữ ký / con dấu, kiểu hiển thị, vị trí khung, font… lấy từ `config.json` (hộp Cấu hình). */
  private readonly cfg = inject(ConfigService);
  readonly sig = computed(() => this.cfg.config()?.signature ?? null);
  readonly imagePath = computed(() => this.sig()?.imagePath ?? '');
  readonly mode = computed<SignatureMode>(() => this.sig()?.display ?? 'text');
  /** Lượt ký sắp tới: chữ ký chính hay ký nháy. Mỗi lượt ra đúng một chữ ký trên mỗi văn bản. */
  readonly kind = computed<SignKind>(() => this.sig()?.kind ?? 'main');
  /** Khung tự đặt dùng chung cho cả lô của lượt ký đang chọn; rỗng: theo mẫu (chính: khổ giấy; nháy: mọi trang). */
  readonly sharedBoxes = computed<SignPlacement[]>(() => {
    const s = this.sig();
    return (this.kind() === 'main' ? s?.positions : s?.initialPositions) ?? [];
  });
  /**
   * Khung riêng từng file của lượt ký đang chọn. Chữ ký chính: chỉ áp khi bật chế độ "Từng file" (màn Vị trí chữ
   * ký); ký nháy: luôn áp (chỉ đặt bằng thêm / xoá khung trên màn này).
   */
  readonly ownPositions = computed<Record<string, SignPlacement[]>>(() => {
    const s = this.sig();
    if (this.kind() === 'initial') {
      return s?.fileInitialPositions ?? {};
    }
    return s?.perFile ? (s.filePositions ?? {}) : {};
  });
  /** Văn bản "chưa gán chữ ký" của lượt ký đang chọn (TD-56): đã xoá hết khung, bỏ qua khi ký số. */
  readonly skipped = computed<ReadonlySet<string>>(() => {
    const s = this.sig();
    return new Set((this.kind() === 'main' ? s?.skipFiles : s?.initialSkipFiles) ?? []);
  });
  /** Kiểu hiển thị riêng từng file đang áp (cùng chế độ "Từng file"); rỗng khi tắt. */
  readonly ownModes = computed<Record<string, SignatureMode>>(() => {
    const s = this.sig();
    return s?.perFile ? (s.fileDisplays ?? {}) : {};
  });
  /** Số file đang nạp có cấu hình riêng (vị trí hoặc kiểu hiển thị). */
  readonly ownCount = computed(() => {
    const own = this.ownPositions();
    const modes = this.ownModes();
    return this.files().filter((f) => own[f.path]?.length || modes[f.path]).length;
  });
  /** Địa chỉ ảnh qua asset protocol (sau khi lõi cấp quyền đọc). */
  readonly imageUrl = signal<string | null>(null);
  /** Chỗ các khối chữ ký trong file đang xem (lõi tính, như khi ký). */
  readonly boxes = signal<SignPlacement[]>([]);
  private boxSeq = 0;
  /** File của các khung đang vẽ; đổi file thì bỏ khung cũ ngay để vùng xem không cuộn theo khung của file trước. */
  private boxPath: string | null = null;
  private certAutoScan = false;

  // ---- Hiển thị -----------------------------------------------------------------------------
  readonly view = signal<KsView>('files');
  readonly search = signal('');
  readonly filter = signal<KsFilter>('all');
  readonly filterOpen = signal(false);
  readonly listOpen = signal(true);
  readonly leftW = signal(340);
  readonly zoom = signal(100);
  readonly selectedPath = signal<string | null>(null);
  readonly sourceOpen = signal(readStr(SOURCE_OPEN_KEY) !== '0');

  // ---- Kết quả ------------------------------------------------------------------------------
  /** Kết quả theo đường dẫn file gốc; giữ qua các lượt Ký tiếp, xoá khi nạp lại hoặc Ký lại. */
  readonly results = signal<Record<string, FileResult>>({});
  readonly job = signal<KsJob | null>(null);
  /** Thời gian đã chạy của các lượt trước (cộng dồn qua Dừng → Ký tiếp). */
  readonly accMs = signal(0);
  readonly now = signal(Date.now());
  readonly log = signal<LogLine[]>([]);
  private logId = 0;
  private ticker: ReturnType<typeof setInterval> | null = null;
  private listSeq = 0;

  // ---- Xem trước ----------------------------------------------------------------------------
  readonly preview = signal<PreviewResult | null>(null);
  readonly previewBusy = signal(false);
  readonly previewError = signal<string | null>(null);
  private previewSeq = 0;
  private lastPreviewKey: string | null = null;

  // ---- Xác nhận ký --------------------------------------------------------------------------
  readonly certScan = signal<CertScan | null>(null);
  readonly certBusy = signal(false);
  readonly certError = signal<string | null>(null);
  readonly chosenCert = signal<string | null>(null);
  /** Mã PIN gõ ở hộp Xác nhận ký; xoá ngay sau khi gửi đi, không lưu ở đâu khác trong giao diện. */
  readonly pinInput = signal('');
  /** "Lưu PIN trên máy này" (tắt sẵn mỗi lần mở hộp). */
  readonly savePin = signal(false);
  /** Thumbprint (chữ hoa) các chứng thư có PIN lưu trên máy. */
  readonly savedPins = signal<ReadonlySet<string>>(new Set());
  readonly tokenBusy = signal(false);
  readonly tokenError = signal<string | null>(null);
  /** Lượt ký sẽ chạy sau khi xác nhận: tất cả file, hay chỉ file chưa ký được. */
  private pendingScope: 'all' | 'remain' = 'all';
  readonly pendingFiles = signal<string[]>([]);

  // ---- File trùng tên, Xóa file gốc -----------------------------------------------------------
  /** "Xóa file gốc" (ADR 0003): chỉ trong phiên làm việc này, tắt sẵn mỗi lần mở app. */
  readonly replaceOriginal = signal(false);
  /** File của lượt sắp ký đã có bản trùng tên ở nơi lưu (hộp "File đã ký / trùng tên"). */
  readonly dups = signal<PlannedOutput[]>([]);
  readonly planBusy = signal(false);
  /** Lựa chọn ở hộp trùng tên cho lượt sắp ký: ghi đè hay giữ cả hai; null: theo cấu hình. */
  private dupOverwrite: boolean | null = null;

  // ---- Suy ra -------------------------------------------------------------------------------
  readonly loaded = computed(() => this.files().length > 0);
  readonly running = computed(() => this.job()?.running ?? false);
  readonly locked = computed(() => !this.available || this.running() || this.tokenBusy());
  readonly hasImage = computed(() => !!this.imagePath());
  /** Kiểu hiển thị của file đang xem: kiểu riêng của file, không có thì kiểu chung. */
  readonly curMode = computed<SignatureMode>(() => {
    const cur = this.current();
    return (cur && this.ownModes()[cur.path]) || this.mode();
  });
  /** File đang xem có kiểu hiển thị riêng. */
  readonly curOwnMode = computed(() => {
    const cur = this.current();
    return !!cur && !!this.ownModes()[cur.path];
  });

  /** File đang ký: file đầu tiên của lượt chạy chưa có kết quả (lõi ký tuần tự theo `request.files`). */
  readonly runningPath = computed(() => {
    const job = this.job();
    if (!job?.running) {
      return null;
    }
    const res = this.results();
    return job.files.find((p) => !res[p]) ?? null;
  });

  readonly okCount = computed(() => this.files().filter((f) => this.results()[f.path]?.success).length);
  /** File lỗi thật (không tính file tạm dừng). */
  readonly errCount = computed(() => {
    const res = this.results();
    return this.files().filter((f) => res[f.path] && !res[f.path].success && !res[f.path].paused).length;
  });
  /** File chưa ký vì lô tạm dừng. */
  readonly pausedCount = computed(() => {
    const res = this.results();
    return this.files().filter((f) => res[f.path]?.paused).length;
  });
  readonly doneCount = computed(() => this.okCount() + this.errCount());
  /** Văn bản chưa gán chữ ký (chưa ký) — bỏ qua khi ký. */
  readonly skipCount = computed(() => {
    const res = this.results();
    return this.files().filter((f) => this.skipped().has(f.path) && !res[f.path]?.success).length;
  });
  /** File chưa ký được (chưa chạy hoặc lỗi, không tính văn bản chưa gán chữ ký) — phạm vi của Ký tiếp. */
  readonly remainCount = computed(() => this.files().length - this.okCount() - this.skipCount());
  readonly hasRun = computed(() => this.running() || Object.keys(this.results()).length > 0);
  readonly allDone = computed(() => !this.running() && this.loaded() && this.remainCount() === 0 && this.hasRun());

  readonly elapsedMs = computed(() => {
    const j = this.job();
    const cur = j?.running ? this.now() - j.startedAt : 0;
    return this.accMs() + cur;
  });

  /** Ước lượng còn lại cho `n` file theo tốc độ trung bình đã chạy. */
  etaMs(n: number): number {
    const done = this.doneCount();
    return done > 0 ? Math.round((this.elapsedMs() / done) * n) : 0;
  }

  readonly visible = computed(() => {
    const q = searchKey(this.search());
    const f = this.filter();
    return this.files()
      .map((file, index) => ({ file, index }))
      .filter(({ file }) => (!q || searchKey(file.name).includes(q)) && (f === 'all' || this.status(file.path) === f));
  });

  /** Đếm cho menu lọc: theo ô tìm, chưa áp bộ lọc trạng thái (như `ksCount`). */
  readonly filterCounts = computed(() => {
    const q = searchKey(this.search());
    const counts: Record<KsFilter, number> = { all: 0, cho: 0, dang: 0, ok: 0, loi: 0, dung: 0, bo: 0 };
    for (const file of this.files()) {
      if (q && !searchKey(file.name).includes(q)) {
        continue;
      }
      counts.all++;
      counts[this.status(file.path)]++;
    }
    return counts;
  });

  /** Vị trí của file đang chọn trong danh sách đang hiện (không có thì file đầu). */
  readonly selectedPos = computed(() => {
    const vis = this.visible();
    const i = vis.findIndex((v) => v.file.path === this.selectedPath());
    return i >= 0 ? i : 0;
  });

  readonly current = computed<SignFileItem | null>(() => this.visible()[this.selectedPos()]?.file ?? null);

  readonly currentResult = computed<FileResult | null>(() => {
    const cur = this.current();
    return cur ? (this.results()[cur.path] ?? null) : null;
  });

  /** Chứng thư hiện ở hộp Xác nhận ký: bỏ các chứng thư người dùng đã xoá khỏi danh sách (tab USB Token). */
  readonly shownCerts = computed(() => {
    const hidden = new Set(this.cfg.config()?.token?.hiddenCerts ?? []);
    return (this.certScan()?.certificates ?? []).filter((c) => !hidden.has(c.thumbprint.toUpperCase()));
  });
  /** Số chứng thư tìm thấy nhưng đã xoá khỏi danh sách. */
  readonly hiddenFound = computed(() => (this.certScan()?.certificates.length ?? 0) - this.shownCerts().length);
  readonly signableCerts = computed(() => this.shownCerts().filter((c) => c.canSign));
  /** Chứng thư đang chọn có PIN lưu trên máy. */
  readonly chosenHasPin = computed(() => {
    const t = this.chosenCert();
    return !!t && this.savedPins().has(t.toUpperCase());
  });

  /** Tên người ký để vẽ khung xem trước: chứng thư đang chọn, hoặc chứng thư dùng lần trước. */
  readonly signerName = computed(() => {
    const thumb = this.chosenCert() ?? readStr(LAST_CERT_KEY);
    return this.certScan()?.certificates.find((c) => c.thumbprint === thumb)?.commonName ?? null;
  });

  /** Cấu hình hiển thị gửi lõi (khi ký và khi tính chỗ khung xem trước). */
  readonly appearance = computed<SignAppearance>(() => {
    const s = this.sig();
    const mode = s?.display ?? 'text';
    const image = s?.imagePath || null;
    // Gửi ảnh khi kiểu chung hoặc kiểu riêng của file nào đó có ảnh (lõi đọc ảnh một lần cho cả lô).
    const usesImage = !!image && [mode, ...Object.values(this.ownModes())].some(isImageMode);
    const wet = s?.imageKind === 'wetSignature';
    return {
      mode,
      kind: this.kind(),
      placements: mode === 'hidden' ? [] : this.sharedBoxes(),
      profiles: s?.profiles ?? [],
      fontFamily: s?.font.family ?? 'Times New Roman',
      fontSize: s?.font.size ?? 10,
      textColor: s?.font.color ?? '#000000',
      reason: s?.reason ?? 'Ký số tài liệu',
      location: s?.location ?? 'Việt Nam',
      sealImage: usesImage && !wet ? image : null,
      wetImage: usesImage && wet ? image : null,
      signatureImage: usesImage ? (wet ? 'wetSignature' : 'seal') : null,
      imageLayout: s?.imageLayout ?? 'left',
      imageShare: s?.imageShare ?? null,
      imageGap: s?.imageGap ?? null,
      imageBox: s?.imageBox && s?.textBox ? s.imageBox : null,
      textBox: s?.imageBox && s?.textBox ? s.textBox : null,
      sealDarkness: s?.image.darkness ?? 140,
      wetDarkness: s?.image.darkness ?? 140,
      wetStroke: s?.image.stroke ?? 100,
      wetColor: s?.image.inkColor ?? null,
      stamps: [],
    };
  });

  /** Nơi lưu file đã ký theo cấu hình. */
  readonly output = computed(() => {
    const o = this.cfg.config()?.output;
    const fixed = o?.mode === 'fixed' && o.fixedFolder ? o.fixedFolder : null;
    return {
      subfolder: o?.subfolderName ?? 'DaKySo',
      suffix: o?.suffix ?? '_signed',
      fixedFolder: fixed,
      overwrite: o?.overwrite ?? false,
      replaceOriginal: this.replaceOriginal(),
      sourceRoots: [...new Set(this.files().flatMap((f) => (f.root ? [f.root] : [])))],
    };
  });

  /** Câu mô tả file đã ký đi đâu (hộp Xác nhận, tooltip): "lưu vào …" hoặc "thay vào chỗ file gốc …". */
  readonly outputNote = computed(() => {
    const o = this.output();
    if (o.replaceOriginal) {
      return 'thay vào chỗ file gốc (file gốc chuyển vào Thùng rác sau khi chữ ký kiểm lại hợp lệ)';
    }
    const roots = o.sourceRoots.map((r) => `${baseName(r)}_${o.subfolder || 'DaKySo'}`).join(', ');
    const where = roots
      ? `thư mục ${roots} ${o.fixedFolder ? 'trong ' + o.fixedFolder : 'cạnh thư mục gốc'} (giữ nguyên thư mục con)`
      : o.fixedFolder
        ? `thư mục ${o.fixedFolder}`
        : `thư mục ${o.subfolder} cạnh file gốc`;
    return `lưu vào ${where}, thêm hậu tố ${o.suffix}`;
  });

  /** Khung chữ ký trên bản xem trước; không vẽ khi kiểu "Ẩn", file lỗi, hoặc đang xem file đã ký (có khối thật). */
  readonly sigBoxes = computed<SigBoxView[]>(() => {
    const mode = this.curMode();
    const cur = this.current();
    if (mode === 'hidden' || !cur || cur.unreadable) {
      return [];
    }
    const res = this.results()[cur.path];
    if ((res?.success && !this.running()) || this.skipped().has(cur.path)) {
      return [];
    }
    const a = this.appearance();
    const editable = !this.locked();
    const lines = [`Ký bởi: ${this.signerName() ?? '[Tên chứng thư số]'}`, `Ngày ký: ${fmtSignTime(new Date(this.now()))}`];
    return this.boxes().map((placement) => ({
      placement,
      mode,
      layout: a.imageLayout,
      imageShare: a.imageShare,
      imageGap: a.imageGap,
      imageBox: a.imageBox,
      textBox: a.textBox,
      imageUrl: mode === 'text' ? null : this.imageUrl(),
      lines,
      fontPt: a.fontSize,
      fontFamily: a.fontFamily,
      color: a.textColor,
      editable,
    }));
  });

  constructor() {
    // Ảnh đã chọn: xin lõi cấp quyền đọc để vẽ trong khung chữ ký. Ảnh nhớ từ lần trước không còn thì bỏ.
    effect(() => {
      const path = this.imagePath();
      untracked(() => {
        this.imageUrl.set(null);
        if (!path || !this.available) {
          return;
        }
        this.ipc.signImagePreview(path).then(
          (granted) => {
            if (this.imagePath() === path) {
              this.imageUrl.set(convertFileSrc(granted));
            }
          },
          (error) => {
            if (this.imagePath() === path) {
              this.clearImage();
              this.loadError.set(`Không mở được ảnh chữ ký / con dấu ${baseName(path)}: ${errorMessage(error)}`);
            }
          },
        );
      });
    });
    // Cấu hình hỏng / đã sửa / lưu lỗi: báo trên màn (lõi đã giữ bản sao file hỏng và dùng mặc định).
    effect(() => {
      const w = this.cfg.warning();
      if (w) {
        untracked(() => this.loadError.set(w));
      }
    });
    // Chỗ khung xem trước: lõi tính theo vị trí kéo tay hoặc mẫu theo khổ giấy của file đang xem.
    effect((onCleanup) => {
      const cur = this.current();
      const a = this.appearance();
      const mode = cur ? this.modeFor(cur.path) : a.mode;
      const signer = this.signerName() ?? '[Tên chứng thư số]';
      const seq = ++this.boxSeq;
      if (!cur || cur.unreadable || mode === 'hidden' || !this.available) {
        untracked(() => this.boxes.set([]));
        return;
      }
      if (cur.path !== this.boxPath) {
        this.boxPath = cur.path;
        untracked(() => this.boxes.set([]));
      }
      const placements = this.placementFor(cur.path);
      const timer = setTimeout(() => {
        this.ipc.signPositions(cur.path, { ...a, mode, placements }, signer).then(
          (list) => seq === this.boxSeq && this.boxes.set(list),
          () => seq === this.boxSeq && this.boxes.set([]),
        );
      }, 80);
      onCleanup(() => clearTimeout(timer));
    });
    // Có văn bản thì đọc kho chứng thư một lần (không hỏi PIN) để khung xem trước có tên người ký.
    effect(() => {
      if (this.loaded() && this.available && !this.certAutoScan) {
        this.certAutoScan = true;
        untracked(() => {
          if (!this.certScan() && !this.certBusy()) {
            void this.scanCerts();
          }
        });
      }
    });
    // Xem trước: file đã ký nếu file đó ký thành công (lõi chỉ cấp quyền đọc file kết quả khi lượt ký xong),
    // ngược lại là file gốc.
    effect((onCleanup) => {
      const cur = this.current();
      const res = cur ? this.results()[cur.path] : undefined;
      const running = this.running();
      let path: string | null = null;
      if (this.available && cur && this.view() === 'files') {
        path = res?.success && res.outputPath && !running ? res.outputPath : cur.path;
      }
      if (!path) {
        this.previewSeq++;
        untracked(() => {
          this.previewBusy.set(false);
          if (!cur) {
            this.lastPreviewKey = null;
            this.preview.set(null);
            this.previewError.set(null);
          }
        });
        return;
      }
      if (path === this.lastPreviewKey) {
        // Quay lại đúng file đang hiện: bỏ lời gọi cũ (nếu còn chạy) để nó không đè lên.
        this.previewSeq++;
        untracked(() => this.previewBusy.set(false));
        return;
      }
      const target = path;
      untracked(() => this.previewBusy.set(true));
      const timer = setTimeout(() => void this.runPreview(target), 60);
      onCleanup(() => clearTimeout(timer));
    });
  }

  status(path: string): KsStatus {
    const r = this.results()[path];
    if (r?.success) {
      return 'ok';
    }
    if (this.skipped().has(path)) {
      return 'bo';
    }
    if (r) {
      return r.paused ? 'dung' : 'loi';
    }
    return path === this.runningPath() ? 'dang' : 'cho';
  }

  // ---- Chọn nguồn ---------------------------------------------------------------------------

  async pick(kind: 'file' | 'folder'): Promise<void> {
    if (this.locked()) {
      return;
    }
    try {
      const path =
        kind === 'folder'
          ? await openDialog({ directory: true, multiple: false, title: 'Chọn thư mục văn bản cần ký' })
          : await openDialog({
              directory: false,
              multiple: false,
              title: 'Chọn văn bản cần ký',
              filters: [{ name: 'Văn bản PDF', extensions: ['pdf'] }],
            });
      if (typeof path === 'string') {
        await this.loadSources([path], kind);
      }
    } catch (error) {
      this.loadError.set(errorMessage(error));
    }
  }

  /** File / thư mục thả vào cửa sổ. */
  async dropPaths(paths: string[]): Promise<void> {
    if (this.locked() || paths.length === 0) {
      return;
    }
    const kind = paths.length === 1 ? (extName(paths[0]) === 'pdf' ? 'file' : extName(paths[0]) ? null : 'folder') : null;
    await this.loadSources(paths, kind);
  }

  async loadSources(sources: string[], kind: 'file' | 'folder' | null): Promise<void> {
    const seq = ++this.listSeq;
    this.listBusy.set(true);
    this.loadError.set(null);
    try {
      const list = await this.ipc.signListFiles(sources);
      if (seq !== this.listSeq) {
        return;
      }
      const label = sources.length === 1 ? sources[0] : `${sources[0]} (+${sources.length - 1})`;
      if (list.length === 0) {
        this.loadError.set(`Không có file PDF nào trong ${sources.map(baseName).join(', ')}.`);
        this.addLog('LỖI', `Không có file PDF nào trong ${label}.`);
        return;
      }
      this.sourcePath.set(label);
      this.sourceKind.set(kind);
      this.files.set(list);
      this.resetRun();
      this.selectedPath.set(list[0].path);
      this.search.set('');
      this.filter.set('all');
      this.view.set('files');
      const what =
        kind === 'folder' ? `thư mục ${sources[0]}` : kind === 'file' ? `file ${baseName(sources[0])}` : label;
      const notes: string[] = [];
      const signed = list.filter((f) => f.signatureCount > 0).length;
      const bad = list.filter((f) => f.unreadable).length;
      if (signed) {
        notes.push(`${signed} file đã có chữ ký`);
      }
      if (bad) {
        notes.push(`${bad} file không đọc được`);
      }
      this.addLog(
        'INFO',
        `Đã nạp ${what} (${list.length} văn bản${notes.length ? '; ' + notes.join(', ') : ''}). Bấm "Ký số" để bắt đầu.`,
      );
    } catch (error) {
      if (seq === this.listSeq) {
        this.loadError.set(`Không đọc được ${sources.map(baseName).join(', ')}: ${errorMessage(error)}`);
        this.addLog('LỖI', `Không đọc được ${sources.join(', ')}: ${errorMessage(error)}`);
      }
    } finally {
      if (seq === this.listSeq) {
        this.listBusy.set(false);
      }
    }
  }

  clearSource(): void {
    if (this.locked()) {
      return;
    }
    this.listSeq++;
    this.listBusy.set(false);
    this.sourcePath.set('');
    this.sourceKind.set(null);
    this.files.set([]);
    this.selectedPath.set(null);
    this.search.set('');
    this.filter.set('all');
    this.resetRun();
  }

  async pickImage(): Promise<void> {
    if (this.locked()) {
      return;
    }
    try {
      const path = await openDialog({
        directory: false,
        multiple: false,
        title: 'Chọn ảnh chữ ký / con dấu',
        filters: [{ name: 'Ảnh chữ ký, con dấu', extensions: IMAGE_EXTS }],
      });
      if (typeof path === 'string') {
        // Chọn ảnh khi đang "Chỉ thông tin" thì chuyển sang "Ảnh + thông tin".
        this.cfg.patch((c) => ({
          ...c,
          signature: {
            ...c.signature,
            imagePath: path,
            display: c.signature.display === 'text' ? 'imageAndText' : c.signature.display,
          },
        }));
      }
    } catch (error) {
      this.loadError.set(errorMessage(error));
    }
  }

  clearImage(): void {
    this.cfg.patch((c) => ({
      ...c,
      signature: {
        ...c.signature,
        imagePath: null,
        display: isImageMode(c.signature.display) ? 'text' : c.signature.display,
        fileDisplays: Object.fromEntries(
          Object.entries(c.signature.fileDisplays ?? {}).map(([p, m]) => [p, isImageMode(m) ? 'text' : m] as const),
        ),
      },
    }));
  }

  /**
   * Ô "Hiển thị" của màn Ký số: file đang xem có kiểu riêng thì sửa kiểu riêng đó, ngược lại sửa kiểu chung
   * (như kéo khung, `moveBox`).
   */
  setMode(mode: SignatureMode): void {
    if (isImageMode(mode) && !this.hasImage()) {
      return;
    }
    const cur = this.current();
    const own = this.curOwnMode();
    this.cfg.patch((c) =>
      own && cur
        ? { ...c, signature: { ...c.signature, fileDisplays: { ...c.signature.fileDisplays, [cur.path]: mode } } }
        : { ...c, signature: { ...c.signature, display: mode } },
    );
  }

  /** Kiểu hiển thị khi ký một file: kiểu riêng của file (chế độ "Từng file"), không có thì kiểu chung. */
  modeFor(path: string): SignatureMode {
    return this.ownModes()[path] ?? this.mode();
  }

  /** Khung tự đặt của một file: riêng của file, không có thì khung dùng chung; rỗng: theo mẫu. */
  placementFor(path: string): SignPlacement[] {
    const own = this.ownPositions()[path];
    return own?.length ? own : this.sharedBoxes();
  }

  /** File đang xem có khung tự đặt (riêng hoặc dùng chung), hoặc đã xoá hết khung (chưa gán chữ ký). */
  readonly currentManual = computed(() => {
    const cur = this.current();
    return !!cur && (this.placementFor(cur.path).length > 0 || this.skipped().has(cur.path));
  });

  /** Văn bản đang xem chưa gán chữ ký (đã xoá hết khung). */
  readonly currentSkipped = computed(() => {
    const cur = this.current();
    return !!cur && this.skipped().has(cur.path);
  });

  /** Đánh dấu / bỏ đánh dấu "chưa gán chữ ký" cho lượt ký đang chọn. */
  private writeSkip(path: string, on: boolean): void {
    const key = this.kind() === 'main' ? 'skipFiles' : 'initialSkipFiles';
    this.cfg.patch((c) => {
      const list = (c.signature[key] ?? []).filter((p) => p !== path);
      return { ...c, signature: { ...c.signature, [key]: on ? [...list, path] : list } };
    });
  }

  /** File đang xem có khung riêng (thêm / xoá khung, hoặc chế độ "Từng file"). */
  readonly currentOwn = computed(() => {
    const cur = this.current();
    return !!cur && !!this.ownPositions()[cur.path]?.length;
  });

  /** Trang (từ 1) có khung chữ ký trong văn bản đang xem, để đánh dấu ảnh thu nhỏ. */
  readonly boxPages = computed<ReadonlySet<number>>(() => new Set(this.sigBoxes().map((b) => b.placement.page)));

  /** Vị trí ảnh so với chữ (kiểu "Ảnh + thông tin"); áp cho mọi văn bản. */
  setImageLayout(imageLayout: ImageLayout): void {
    if (this.locked()) {
      return;
    }
    // Chọn Trái / Phải / Trên / Dưới thì bỏ kiểu "Tự do" (TD-52).
    this.cfg.patch((c) => ({ ...c, signature: { ...c.signature, imageLayout, imageBox: null, textBox: null } }));
  }

  /** Đổi loại lượt ký (chính / nháy). */
  setKind(kind: SignKind): void {
    if (this.locked() || kind === this.kind()) {
      return;
    }
    this.boxes.set([]);
    this.cfg.patch((c) => ({ ...c, signature: { ...c.signature, kind } }));
  }

  /** Ghi danh sách khung riêng của file đang xem cho lượt ký đang chọn; rỗng: bỏ khung riêng. */
  private writeOwn(path: string, list: SignPlacement[]): void {
    const key = this.kind() === 'main' ? 'filePositions' : 'fileInitialPositions';
    this.cfg.patch((c) => {
      const map = { ...(c.signature[key] ?? {}) };
      if (list.length) {
        map[path] = list;
      } else {
        delete map[path];
      }
      // Khung riêng của chữ ký chính chỉ áp khi bật "Từng file".
      const perFile = key === 'filePositions' && list.length ? true : c.signature.perFile;
      return { ...c, signature: { ...c.signature, [key]: map, perFile } };
    });
  }

  private writeShared(list: SignPlacement[]): void {
    const key = this.kind() === 'main' ? 'positions' : 'initialPositions';
    this.cfg.patch((c) => ({ ...c, signature: { ...c.signature, [key]: list } }));
  }

  /**
   * Kéo khung thứ `index` trên màn Ký số. File đang xem có khung riêng thì sửa khung riêng. Không có: văn bản chỉ
   * có một khung (chữ ký chính) thì sửa khung dùng chung cho cả lô như trước; nhiều khung (ký nháy) thì thành khung
   * riêng của văn bản này, vì danh sách trang cụ thể không áp đúng cho văn bản có số trang khác.
   */
  moveBox(index: number, p: SignPlacement): void {
    const cur = this.current();
    if (!cur || this.locked()) {
      return;
    }
    const list = this.boxes().map((b, i) => (i === index ? p : b));
    if (!list.length) {
      list.push(p);
    }
    // Hiện ngay chỗ vừa thả, không chờ lõi tính lại.
    this.boxes.set(list);
    if (this.currentOwn() || list.length > 1) {
      this.writeOwn(cur.path, list);
    } else {
      this.writeShared(list);
    }
  }

  /**
   * Xoá khung thứ `index` của văn bản đang xem (khung riêng của văn bản này). Xoá khung cuối cùng thì văn bản thành
   * "chưa gán chữ ký" (TD-56): không vẽ khung, bỏ qua khi ký số; thêm khung lại thì hết.
   */
  deleteBox(index: number): boolean {
    const cur = this.current();
    const list = this.boxes();
    if (!cur || this.locked() || this.currentSkipped() || index < 0 || index >= list.length) {
      return false;
    }
    if (list.length === 1) {
      this.writeSkip(cur.path, true);
      return true;
    }
    const next = list.filter((_, i) => i !== index);
    this.boxes.set(next);
    this.writeOwn(cur.path, next);
    return true;
  }

  /**
   * Thêm khung vào văn bản đang xem tại trang `page`, tâm ở (`xRatio`, `yRatio`). Cỡ lấy theo khung đang có (ưu
   * tiên khung cùng trang), ép vào trong trang.
   */
  addBox(page: number, xRatio: number, yRatio: number): void {
    const cur = this.current();
    if (!cur || this.locked()) {
      return;
    }
    // Văn bản chưa gán chữ ký: khung mới là khung duy nhất (cỡ theo khung lõi tính sẵn), bỏ đánh dấu.
    const skipped = this.currentSkipped();
    const list = skipped ? [] : this.boxes();
    const like = list.find((b) => b.page === page) ?? list[0] ?? this.boxes()[0];
    const w = like?.widthRatio ?? 0.29;
    const h = like?.heightRatio ?? 0.06;
    if (skipped) {
      this.writeSkip(cur.path, false);
    }
    const box: SignPlacement = {
      page,
      xRatio: round4(clamp(xRatio - w / 2, 0, 1 - w)),
      yRatio: round4(clamp(yRatio - h / 2, 0, 1 - h)),
      widthRatio: w,
      heightRatio: h,
    };
    const next = [...list, box];
    this.boxes.set(next);
    this.writeOwn(cur.path, next);
  }

  /** Chép khung thứ `index` sang mọi trang chưa có khung (cùng chỗ trên trang). */
  copyBoxToAllPages(index: number, pageCount: number): void {
    const cur = this.current();
    const list = this.boxes();
    const src = list[index];
    if (!cur || !src || this.locked() || pageCount < 2) {
      return;
    }
    const used = new Set(list.map((b) => b.page));
    const added: SignPlacement[] = [];
    for (let n = 1; n <= pageCount; n++) {
      if (!used.has(n)) {
        added.push({ ...src, page: n });
      }
    }
    if (!added.length) {
      return;
    }
    const next = [...list, ...added].sort((a, b) => a.page - b.page);
    this.boxes.set(next);
    this.writeOwn(cur.path, next);
  }

  /** Về mặc định: bỏ khung riêng của văn bản đang xem; không có khung riêng thì bỏ khung dùng chung (theo mẫu). */
  resetBoxes(): void {
    const cur = this.current();
    if (!cur || this.locked()) {
      return;
    }
    if (this.currentSkipped()) {
      this.writeSkip(cur.path, false);
      return;
    }
    if (this.currentOwn()) {
      this.writeOwn(cur.path, []);
    } else {
      this.writeShared([]);
    }
  }

  /** Mô tả lượt ký và vị trí khung cho nhật ký. */
  placementText(): string {
    const kind = this.kind() === 'main' ? 'chữ ký chính' : 'ký nháy';
    const list = this.sharedBoxes();
    const own = this.ownCount();
    const pct = (v: number) => Math.round(v * 100);
    const p = list[0];
    const base = p
      ? list.length > 1
        ? `${kind}, ${list.length} khung tự đặt`
        : `${kind}, tự đặt: trang ${p.page || 'cuối'}, cách trái ${pct(p.xRatio)}%, cách trên ${pct(p.yRatio)}%, rộng ${pct(p.widthRatio)}% trang`
      : this.kind() === 'main'
        ? `${kind}, theo mẫu chữ ký theo khổ giấy`
        : `${kind}, mọi trang`;
    return own ? `${base}; ${own} file có cấu hình riêng (khung / kiểu hiển thị)` : base;
  }

  // ---- Điều hướng ---------------------------------------------------------------------------

  selectPos(pos: number): void {
    const vis = this.visible();
    if (vis.length === 0) {
      return;
    }
    const i = Math.max(0, Math.min(vis.length - 1, Math.floor(pos) || 0));
    this.selectedPath.set(vis[i].file.path);
  }

  setFilter(f: KsFilter): void {
    this.filter.set(f);
    this.filterOpen.set(false);
  }

  setSourceOpen(open: boolean): void {
    this.sourceOpen.set(open);
    writeStr(SOURCE_OPEN_KEY, open ? '1' : '0');
  }

  zoomBy(step: number): void {
    this.zoom.update((z) => Math.max(40, Math.min(160, z + step)));
  }

  clearLog(): void {
    this.log.set([]);
  }

  // ---- Ký -----------------------------------------------------------------------------------

  /** Nút chính: Tạm dừng khi đang chạy; ngược lại mở hộp Xác nhận ký (Ký số / Ký tiếp / Ký lại). */
  primaryAction(): void {
    if (this.running()) {
      void this.stop();
      return;
    }
    if (!this.available || !this.loaded() || this.tokenBusy()) {
      return;
    }
    const scope = this.allDone() || !this.hasRun() ? 'all' : 'remain';
    const res = this.results();
    // Văn bản chưa gán chữ ký (đã xoá hết khung) bỏ qua (TD-56).
    const skipped = this.skipped();
    const files = this.files()
      .filter((f) => (scope === 'all' || !res[f.path]?.success) && !skipped.has(f.path))
      .map((f) => f.path);
    if (files.length === 0) {
      if (this.skipCount()) {
        this.addLog('INFO', `Không có văn bản nào để ký: ${this.skipCount()} văn bản chưa gán chữ ký (đã xoá hết khung).`);
      }
      return;
    }
    this.pendingScope = scope;
    this.pendingFiles.set(files);
    this.tokenError.set(null);
    this.dupOverwrite = null;
    void this.checkDups(files);
  }

  /** Hỏi trước khi ký nếu nơi lưu đã có file trùng tên (prototype `ksDups` / `mTrung`); không có thì sang Xác nhận ký. */
  private async checkDups(files: string[]): Promise<void> {
    if (this.planBusy()) {
      return;
    }
    let dups: PlannedOutput[] = [];
    if (!this.output().replaceOriginal) {
      this.planBusy.set(true);
      try {
        dups = (await this.ipc.signPlanOutputs(files, this.output())).filter((p) => p.exists);
      } catch (error) {
        this.loadError.set(`Không kiểm được file trùng tên: ${errorMessage(error)}`);
      } finally {
        this.planBusy.set(false);
      }
    }
    this.dups.set(dups);
    if (dups.length) {
      this.ui.open({ kind: 'ks-dup' });
    } else {
      this.openConfirm();
    }
  }

  /** Hộp trùng tên: Ghi đè (thay bản đã ký cũ) hoặc Giữ cả hai (thêm (1), (2)…), rồi sang Xác nhận ký. */
  chooseDup(overwrite: boolean): void {
    this.dupOverwrite = overwrite;
    this.openConfirm();
  }

  private openConfirm(): void {
    this.pinInput.set('');
    this.savePin.set(false);
    this.ui.open({ kind: 'ks-confirm' });
    void this.scanCerts();
  }

  /** Bật "Xóa file gốc" cần xác nhận (ADR 0003); tắt thì tắt ngay. */
  setReplace(on: boolean): void {
    if (this.locked()) {
      return;
    }
    if (on) {
      this.ui.open({ kind: 'ks-replace' });
    } else {
      this.replaceOriginal.set(false);
    }
  }

  confirmReplace(): void {
    this.replaceOriginal.set(true);
    this.ui.close();
  }

  async scanCerts(): Promise<void> {
    this.certBusy.set(true);
    this.certError.set(null);
    try {
      void this.loadPins();
      const scan = await this.ipc.certList();
      this.certScan.set(scan);
      const signable = this.signableCerts();
      const last = readStr(LAST_CERT_KEY);
      const keep = signable.find((c) => c.thumbprint === this.chosenCert()) ?? signable.find((c) => c.thumbprint === last);
      this.chosenCert.set((keep ?? signable[0])?.thumbprint ?? null);
    } catch (error) {
      this.certScan.set(null);
      this.certError.set(errorMessage(error));
    } finally {
      this.certBusy.set(false);
    }
  }

  chooseCert(cert: CertEntry): void {
    if (cert.canSign && !this.tokenBusy()) {
      if (cert.thumbprint !== this.chosenCert()) {
        this.pinInput.set('');
      }
      this.chosenCert.set(cert.thumbprint);
      this.tokenError.set(null);
    }
  }

  async loadPins(): Promise<void> {
    if (!this.available) {
      return;
    }
    try {
      this.savedPins.set(new Set((await this.ipc.pinSaved()).map((t) => t.toUpperCase())));
    } catch {
      this.savedPins.set(new Set());
    }
  }

  /** Xoá PIN đã lưu của chứng thư đang chọn (hộp Xác nhận ký). */
  async forgetPin(): Promise<void> {
    const t = this.chosenCert();
    if (!t || this.tokenBusy()) {
      return;
    }
    try {
      await this.ipc.pinForget(t);
      this.addLog('TOKEN', 'Đã xoá mã PIN đã lưu của chứng thư.');
    } catch (error) {
      this.tokenError.set(errorMessage(error));
    }
    await this.loadPins();
  }

  /** Đóng hộp Xác nhận ký. Khi đang mở token thì vẫn đóng được; kết quả mở token khi đó bị bỏ qua. */
  cancelConfirm(): void {
    this.ui.close();
  }

  /** Bấm "Ký số" trong hộp xác nhận: mở token (ký thử), rồi chạy lô. */
  async confirm(): Promise<void> {
    const thumb = this.chosenCert();
    const cert = this.signableCerts().find((c) => c.thumbprint === thumb);
    if (!thumb || !cert || this.tokenBusy() || this.running()) {
      return;
    }
    writeStr(LAST_CERT_KEY, thumb);
    // Lấy PIN ra rồi xoá ô nhập ngay: PIN chỉ đi một lần sang lõi.
    const pin = this.pinInput().trim() || null;
    const save = !!pin && this.savePin();
    this.pinInput.set('');
    this.tokenBusy.set(true);
    this.tokenError.set(null);
    let info: TokenInfo;
    try {
      info = await this.ipc.tokenCheck(thumb, pin, save);
    } catch (error) {
      this.tokenBusy.set(false);
      const msg = errorMessage(error);
      this.tokenError.set(msg);
      this.addLog('LỖI', `Không mở được token (${cert.commonName}): ${msg}`);
      if (PIN_ERROR_CODES.includes((error as { code?: number })?.code ?? 0)) {
        // PIN đã lưu sai thì lõi đã xoá: đọc lại để hộp hiện ô nhập PIN.
        void this.loadPins();
      }
      return;
    }
    this.tokenBusy.set(false);
    this.addLog(
      'TOKEN',
      `Token sẵn sàng – chứng thư: ${info.commonName} (${info.issuerCommonName}) · ${info.provider}${info.dev ? ' · chứng thư thử nghiệm' : ''}`,
    );
    if (info.pinSource) {
      const from = info.pinSource === 'saved' ? 'Tự nhập mã PIN đã lưu' : 'Đưa mã PIN vừa nhập cho token';
      this.addLog(
        'TOKEN',
        info.pinHandoff === 'accepted'
          ? `${from}: middleware nhận PIN do ứng dụng đưa vào.`
          : `${from}: middleware không nhận PIN do ứng dụng đưa vào, PIN được nhập ở hộp của token.`,
      );
    }
    if (info.pinSaved) {
      this.addLog('TOKEN', 'Đã lưu mã PIN trên máy (mã hoá theo tài khoản Windows); lần sau app tự nhập.');
      void this.loadPins();
    } else if (info.pinNote) {
      this.addLog('TOKEN', info.pinNote);
    }
    if (this.ui.dialog()?.kind !== 'ks-confirm') {
      // Người dùng đã đóng hộp trong lúc chờ token: không tự ký.
      return;
    }
    this.ui.close();
    await this.run(thumb, info.commonName);
  }

  private async run(thumbprint: string, certName: string): Promise<void> {
    const files = this.pendingFiles();
    if (files.length === 0 || this.running()) {
      return;
    }
    if (this.pendingScope === 'all') {
      this.results.set({});
      this.accMs.set(0);
    } else {
      // Ký tiếp: bỏ kết quả lỗi cũ của các file sẽ ký lại.
      const again = new Set(files);
      this.results.update((r) => Object.fromEntries(Object.entries(r).filter(([p]) => !again.has(p))));
    }
    const image = this.imagePath() || null;
    const mode = this.mode();
    const appearance = this.appearance();
    const usesImage = !!appearance.signatureImage;
    const modes = Object.fromEntries(
      files.flatMap((f) => {
        const own = this.ownModes()[f];
        return own ? [[f, own] as const] : [];
      }),
    );
    const request: SignJobRequest = {
      files,
      appearance,
      output: { ...this.output(), overwrite: this.dupOverwrite ?? this.output().overwrite },
      placements: Object.fromEntries(
        files.flatMap((f) => {
          const own = this.ownPositions()[f];
          return own?.length ? [[f, own] as const] : [];
        }),
      ),
      modes,
      requireTrusted: false,
    };
    this.job.set({
      jobId: null,
      running: true,
      stopping: false,
      startedAt: Date.now(),
      finishedAt: null,
      files,
      done: 0,
      outputFolder: '',
      firstOutput: null,
      summary: null,
      error: null,
    });
    this.view.set('files');
    this.filterOpen.set(false);
    this.startTicker();
    if (this.skipCount()) {
      this.addLog('INFO', `Bỏ qua ${this.skipCount()} văn bản chưa gán chữ ký (đã xoá hết khung).`);
    }
    this.addLog(
      'INFO',
      `Bắt đầu ký ${files.length} văn bản – chứng thư: ${certName} – hiển thị: ${MODE_LABEL[mode]}` +
        (Object.keys(modes).length ? ` (${Object.keys(modes).length} file có kiểu riêng)` : '') +
        (usesImage ? ` – ảnh: ${baseName(image ?? '')}` : '') +
        ` – vị trí: ${this.placementText()}` +
        (request.output.replaceOriginal ? ' – xóa file gốc (vào Thùng rác)' : ''),
    );
    try {
      const jobId = await this.ipc.signRun(thumbprint, request, (event) => this.onEvent(event));
      this.patchJob((j) => ({ ...j, jobId: j.jobId ?? jobId }));
    } catch (error) {
      const msg = errorMessage(error);
      this.finishTimer(null);
      this.patchJob((j) => ({ ...j, running: false, finishedAt: Date.now(), error: msg }));
      this.addLog('LỖI', `Không chạy được lô ký: ${msg}`);
    }
  }

  async stop(): Promise<void> {
    const job = this.job();
    if (!job?.running || !job.jobId || job.stopping) {
      return;
    }
    this.patchJob((j) => ({ ...j, stopping: true }));
    this.addLog('INFO', 'Đang tạm dừng… (văn bản đang ký dở sẽ ký xong)');
    try {
      await this.ipc.jobCancel(job.jobId);
    } catch (error) {
      this.patchJob((j) => ({ ...j, stopping: false }));
      this.addLog('LỖI', `Không tạm dừng được: ${errorMessage(error)}`);
    }
  }

  async openOutputFolder(): Promise<void> {
    const job = this.job();
    const folder = job?.summary?.outputFolder || job?.outputFolder || (job?.firstOutput ? dirName(job.firstOutput) : '');
    if (!folder) {
      return;
    }
    try {
      await this.ipc.shellOpen(folder);
    } catch (error) {
      this.addLog('LỖI', `Không mở được thư mục ${folder}: ${errorMessage(error)}`);
      this.loadError.set(`Không mở được thư mục ${folder}: ${errorMessage(error)}`);
    }
  }

  readonly outputFolder = computed(() => {
    const job = this.job();
    return job?.summary?.outputFolder || job?.outputFolder || (job?.firstOutput ? dirName(job.firstOutput) : '');
  });

  private onEvent(event: SignEvent): void {
    switch (event.type) {
      case 'manifest':
        this.patchJob((j) => ({ ...j, jobId: event.jobId, outputFolder: event.outputFolder }));
        if (event.clockNote) {
          this.addLog('INFO', event.clockNote);
        } else if (event.clock === 'tsa') {
          this.addLog('INFO', 'Giờ ký lấy theo máy chủ dấu thời gian Ban Cơ yếu. Ký dài hạn: nhúng dữ liệu kiểm tra thu hồi (OCSP / CRL) vào từng văn bản.');
        }
        break;
      case 'file': {
        const job = this.job();
        const path = job?.files[event.index];
        if (!job || !path) {
          break;
        }
        this.results.update((r) => ({
          ...r,
          [path]: {
            success: event.success,
            outputPath: event.outputPath,
            reason: event.reason,
            verdict: event.verdict,
            trustedTime: event.trustedTime,
            paused: event.paused,
          },
        }));
        this.patchJob((j) => ({ ...j, done: event.done, firstOutput: j.firstOutput ?? event.outputPath }));
        if (event.success) {
          const v = event.verdict && event.verdict !== 'validTrusted' ? VERDICT_LABEL[event.verdict] : '';
          const replaced = !!event.outputPath && event.outputPath === job.files[event.index];
          const note = event.reason && /Không thay file gốc|dữ liệu kiểm tra dài hạn/.test(event.reason) ? ' · ' + event.reason : '';
          this.addLog(
            'OK',
            `${event.name} → ${replaced ? 'thay file gốc (bản gốc vào Thùng rác)' : event.outputPath ? baseName(event.outputPath) : '(đã ký)'}${v ? ' · ' + v : ''}${note}`,
          );
        } else if (!event.paused) {
          this.addLog('LỖI', `${event.name} – ${event.reason ?? 'Không ký được'}`);
        }
        break;
      }
      case 'summary':
        // Xem lại file đang chọn từ đầu: với "Xóa file gốc", file đã ký nằm đúng đường dẫn file gốc.
        this.lastPreviewKey = null;
        this.finishTimer(event.elapsedMs);
        this.patchJob((j) => ({
          ...j,
          jobId: event.jobId,
          running: false,
          stopping: false,
          finishedAt: Date.now(),
          outputFolder: event.outputFolder || j.outputFolder,
          summary: event,
        }));
        if (event.tokenRemoved) {
          this.addLog('TOKEN', 'Đã rút USB token khi đang ký.');
        }
        if (event.stopped) {
          this.addLog('INFO', `Đã tạm dừng. Còn ${this.remainCount()} văn bản chưa ký; bấm "Ký tiếp" để ký tiếp.`);
        }
        if (this.outputFolder()) {
          this.addLog('INFO', `File đã ký lưu tại ${this.outputFolder()}`);
        }
        this.addLog(
          'XONG',
          `${event.success}/${event.total} thành công, ${event.failed} lỗi${event.paused ? `, ${event.paused} tạm dừng` : ''} · ${Math.round(event.elapsedMs / 1000)} giây`,
        );
        if (this.ui.screen() === 'kyso') {
          this.ui.open({ kind: 'ks-done' });
        }
        break;
    }
  }

  private patchJob(fn: (job: KsJob) => KsJob): void {
    this.job.update((j) => (j ? fn(j) : j));
  }

  private resetRun(): void {
    this.results.set({});
    this.job.set(null);
    this.accMs.set(0);
    this.resetPreview();
    if (this.ui.dialog()?.kind === 'ks-done') {
      this.ui.close();
    }
  }

  private addLog(tag: LogTag, text: string): void {
    this.log.update((l) => [...l, { id: ++this.logId, time: clock(), tag, text }]);
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

  /** Kết thúc lượt: cộng thời gian lượt này vào tổng (ưu tiên số của lõi). */
  private finishTimer(elapsedMs: number | null): void {
    const j = this.job();
    this.stopTicker();
    if (j?.running) {
      this.accMs.update((a) => a + (elapsedMs ?? Date.now() - j.startedAt));
    }
  }

  // ---- Xem trước ----------------------------------------------------------------------------

  private async runPreview(path: string): Promise<void> {
    const seq = ++this.previewSeq;
    this.previewBusy.set(true);
    try {
      const pdfPath = await this.ipc.signPreview(path);
      if (seq !== this.previewSeq) {
        return;
      }
      this.lastPreviewKey = path;
      this.preview.set({ pdfPath, pageCount: 0, warnings: [] });
      this.previewError.set(null);
    } catch (error) {
      if (seq === this.previewSeq) {
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
}
