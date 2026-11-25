import { Injectable, inject, signal } from '@angular/core';

import { AppInfo, IpcService, errorMessage } from './ipc.service';
import { IconName } from '../shared/icons';

/**
 * Mã màn, giữ như prototype (`S.screen`, L642). `xemtruoc` (nút "Cấu hình vị trí" của màn Ký số) và `tucauhinh`
 * (menu Chức năng › Vị trí chữ ký, đã bỏ khỏi menu) của prototype cùng vẽ `manTC()`, nên gộp vào `tucauhinh`.
 */
export type Screen = 'home' | 'kyso' | 'tvb' | 'kt' | 'tucauhinh';

/** Tên màn trên tiêu đề cửa sổ (`TITLES`, L670). */
export const SCREEN_TITLES: Record<Screen, string> = {
  home: 'Trang chủ',
  kyso: 'Ký số văn bản',
  tvb: 'Tạo văn bản',
  kt: 'Kiểm tra chữ ký số',
  tucauhinh: 'Xem trước & vị trí chữ ký',
};

/** Hộp thoại đang mở. Hộp của màn Tạo văn bản / Ký số do màn đó vẽ, dữ liệu nằm trong store của màn. */
export type Dialog =
  | { kind: 'dev'; title: string; icon: IconName }
  | { kind: 'info' }
  | { kind: 'help'; topic: number }
  | { kind: 'driver' }
  /** Hộp xin đồng ý đăng ký thiết bị (lần chạy đầu) và hộp cấu hình Thông báo Telegram. */
  | { kind: 'tg-consent' }
  | { kind: 'telegram' }
  | { kind: 'tvb-confirm' }
  | { kind: 'tvb-done' }
  | { kind: 'ks-confirm' }
  | { kind: 'ks-dup' }
  | { kind: 'ks-replace' }
  | { kind: 'ks-done' }
  | { kind: 'cauhinh'; tab: ConfigTab };

/** Tab của hộp Cấu hình (prototype `S.cfgTab`). */
export type ConfigTab = 'sig' | 'token' | 'folder';

/** Trạng thái vỏ ứng dụng: màn hiện tại, menu, hộp thoại, tooltip "!" đang mở, thông tin phần mềm. */
@Injectable({ providedIn: 'root' })
export class UiService {
  private readonly ipc = inject(IpcService);

  readonly screen = signal<Screen>('home');
  /** Menu thả đang mở: `cn` (Chức năng) hoặc `ht` (Hỗ trợ). */
  readonly menu = signal<'cn' | 'ht' | null>(null);
  readonly dialog = signal<Dialog | null>(null);
  /** Mã tooltip "!" đang mở; chỉ một cái mở tại một thời điểm. */
  readonly tip = signal<number | null>(null);
  /** Biến đang mở menu chọn nguồn ở màn Tạo văn bản (cần ở đây vì phím Esc xét nó trước tiên). */
  readonly sourcePick = signal<string | null>(null);

  readonly info = signal<AppInfo | null>(null);
  readonly infoError = signal<string | null>(null);

  constructor() {
    if (this.ipc.available) {
      this.ipc.appInfo().then(
        (info) => this.info.set(info),
        (error: unknown) => this.infoError.set(errorMessage(error)),
      );
    }
  }

  /** Đổi màn: đóng menu và hộp thoại như `go()` của prototype (L751–757). */
  go(screen: Screen): void {
    this.menu.set(null);
    this.dialog.set(null);
    this.tip.set(null);
    this.sourcePick.set(null);
    this.screen.set(screen);
  }

  open(dialog: Dialog): void {
    this.menu.set(null);
    this.tip.set(null);
    this.dialog.set(dialog);
  }

  close(): void {
    this.dialog.set(null);
  }

  /** Hộp Cấu hình, mở ở tab cho trước (mặc định tab dùng lần trước). */
  openConfig(tab?: ConfigTab): void {
    this.lastConfigTab = tab ?? this.lastConfigTab;
    this.open({ kind: 'cauhinh', tab: this.lastConfigTab });
  }

  lastConfigTab: ConfigTab = 'sig';

  openDev(title: string, icon: IconName): void {
    this.open({ kind: 'dev', title, icon });
  }
}
