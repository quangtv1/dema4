import { Injectable, inject, signal } from '@angular/core';

import { AppConfig, ConfigSaved, IpcService, SignPlacement, SignatureMode, errorMessage } from './ipc.service';

/** Khoá localStorage của bản trước (19/11/2026) — chuyển sang config.json một lần khi chưa có file cấu hình. */
const OLD_KEYS = { image: 'smks.ks.anh', mode: 'smks.ks.kieu', position: 'smks.ks.viTri' } as const;

function readOld(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function dropOld(): void {
  try {
    Object.values(OLD_KEYS).forEach((k) => localStorage.removeItem(k));
  } catch {
    // Không xoá được: lần sau vẫn bỏ qua vì đã có file cấu hình.
  }
}

/**
 * Cấu hình người dùng (`%APPDATA%\SaoMai-KySo\config.json`) dùng chung cho các màn. Lõi đọc / làm sạch / ghi;
 * ở đây giữ bản đang dùng. Sửa nhanh trên màn Ký số (ảnh, kiểu hiển thị, kéo khung) lưu sau 0,4 giây;
 * hộp Cấu hình lưu ngay khi bấm Lưu.
 */
@Injectable({ providedIn: 'root' })
export class ConfigService {
  private readonly ipc = inject(IpcService);

  readonly config = signal<AppConfig | null>(null);
  readonly path = signal('');
  /** Điều cần báo (file hỏng, giá trị đã sửa, lưu lỗi). */
  readonly warning = signal<string | null>(null);
  private loading: Promise<void> | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    if (this.ipc.available) {
      void this.load();
    }
  }

  load(): Promise<void> {
    this.loading ??= (async () => {
      try {
        const res = await this.ipc.configGet();
        this.path.set(res.path);
        this.warning.set(res.warning);
        let config = res.config;
        if (res.created) {
          config = this.migrate(config);
        }
        this.config.set(config);
        if (res.created && config !== res.config) {
          await this.save(config);
        }
        dropOld();
      } catch (error) {
        this.warning.set(`Không đọc được cấu hình: ${errorMessage(error)}`);
      }
    })();
    return this.loading;
  }

  /** Lấy ảnh / kiểu hiển thị / vị trí khung đã nhớ trong localStorage của bản trước. */
  private migrate(config: AppConfig): AppConfig {
    const image = readOld(OLD_KEYS.image);
    const mode = readOld(OLD_KEYS.mode) as SignatureMode | null;
    let position: SignPlacement | null = null;
    try {
      position = JSON.parse(readOld(OLD_KEYS.position) || 'null') as SignPlacement | null;
    } catch {
      position = null;
    }
    if (!image && !mode && !position) {
      return config;
    }
    const s = { ...config.signature };
    if (image) {
      s.imagePath = image;
    }
    if (mode === 'hidden' || mode === 'text' || mode === 'image' || mode === 'imageAndText') {
      s.display = mode;
    }
    if (position) {
      s.positions = [position];
    }
    return { ...config, signature: s };
  }

  /** Sửa nhanh: áp ngay, lưu sau 0,4 giây (gộp các lần sửa liên tiếp, ví dụ khi kéo khung). */
  patch(change: (c: AppConfig) => AppConfig): void {
    const cur = this.config();
    if (!cur) {
      return;
    }
    this.config.set(change(cur));
    if (this.saveTimer) {
      clearTimeout(this.saveTimer);
    }
    this.saveTimer = setTimeout(() => {
      this.saveTimer = null;
      const latest = this.config();
      if (latest) {
        this.save(latest).catch(() => undefined);
      }
    }, 400);
  }

  /** Lưu ngay; giữ bản lõi trả về (đã làm sạch). */
  async save(config: AppConfig): Promise<ConfigSaved> {
    if (this.saveTimer) {
      clearTimeout(this.saveTimer);
      this.saveTimer = null;
    }
    // Cấu hình đang dùng lúc bắt đầu lưu: người dùng sửa tiếp trong lúc chờ lõi (ví dụ bấm hai lựa chọn liền nhau)
    // thì giữ bản mới hơn, không ghi đè bằng bản lõi trả về; lần lưu hẹn giờ sau sẽ ghi bản mới đó.
    const before = this.config();
    try {
      const res = await this.ipc.configSave(config);
      if (this.config() === before) {
        this.config.set(res.config);
      }
      this.warning.set(res.notes.length ? `Đã sửa một số giá trị cấu hình: ${res.notes.join(' ')}` : null);
      return res;
    } catch (error) {
      this.warning.set(`Không lưu được cấu hình: ${errorMessage(error)}`);
      throw error;
    }
  }

  /** Cấu hình mặc định của lõi (Khôi phục mặc định). */
  defaults(): Promise<AppConfig> {
    return this.ipc.configDefaults();
  }
}
