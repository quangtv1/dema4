import { Injectable, inject, signal } from '@angular/core';

import { IpcService, NotifyFlags, NotifyStatus, errorMessage } from './ipc.service';
import { UiService } from './ui.service';

/** Việc đang chạy của hộp Thông báo Telegram (để khoá nút và hiện "Đang…"). */
export type NotifyBusy = 'consent' | 'prefs' | 'check' | 'test' | 'unregister';

/**
 * Trạng thái thông báo Telegram và đăng ký thiết bị. Lõi (`smks-notify`) giữ toàn bộ logic và bí mật; ở đây chỉ
 * đọc trạng thái và chuyển lựa chọn của người dùng xuống lõi.
 *
 * Lúc mở ứng dụng: nếu máy có cấu hình quản trị và người dùng chưa trả lời thì mở hộp đăng ký thiết bị. Chưa trả
 * lời (đóng hộp, "Để sau") thì ứng dụng không gửi gì và hỏi lại ở lần mở sau.
 */
@Injectable({ providedIn: 'root' })
export class NotifyService {
  private readonly ipc = inject(IpcService);
  private readonly ui = inject(UiService);

  readonly status = signal<NotifyStatus | null>(null);
  readonly error = signal<string | null>(null);
  readonly busy = signal<NotifyBusy | null>(null);

  constructor() {
    if (this.ipc.available) {
      void this.start();
    }
  }

  private async start(): Promise<void> {
    await this.refresh();
    if (this.status()?.needsConsent && !this.ui.dialog()) {
      this.ui.open({ kind: 'tg-consent' });
    }
  }

  private async run(what: NotifyBusy | null, call: () => Promise<NotifyStatus>): Promise<boolean> {
    if (this.busy()) {
      return false;
    }
    this.busy.set(what);
    this.error.set(null);
    try {
      this.status.set(await call());
      return true;
    } catch (error) {
      this.error.set(errorMessage(error));
      return false;
    } finally {
      this.busy.set(null);
    }
  }

  refresh(): Promise<boolean> {
    return this.run(null, () => this.ipc.notifyStatus());
  }

  /** Trả lời hộp đăng ký thiết bị. */
  consent(accept: boolean, includeIp: boolean, includeMac: boolean): Promise<boolean> {
    return this.run('consent', () => this.ipc.notifyConsent(accept, includeIp, includeMac));
  }

  unregister(): Promise<boolean> {
    return this.run('unregister', () => this.ipc.notifyUnregister());
  }

  setPrefs(prefs: NotifyFlags): Promise<boolean> {
    return this.run('prefs', () => this.ipc.notifySetPrefs(prefs));
  }

  check(): Promise<boolean> {
    return this.run('check', () => this.ipc.notifyCheck());
  }

  async test(): Promise<boolean> {
    const ok = await this.run('test', () => this.ipc.notifyTest());
    if (!ok) {
      // Gửi thử lỗi: đọc lại để dòng Kết nối hiện lý do.
      const message = this.error();
      await this.run(null, () => this.ipc.notifyStatus());
      this.error.set(message);
    }
    return ok;
  }
}
