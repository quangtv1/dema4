import { ChangeDetectionStrategy, Component, effect, inject } from '@angular/core';
import { Title } from '@angular/platform-browser';

import { SCREEN_TITLES, Screen, UiService } from './core/ui.service';
import { HomeComponent } from './home/home.component';
import { KiemTraComponent } from './kiem-tra/kiem-tra.component';
import { KySoComponent } from './ky-so/ky-so.component';
import { ConfigDialogComponent } from './shell/config-dialog.component';
import { DevScreenComponent } from './shell/dev-screen.component';
import { MenubarComponent } from './shell/menubar.component';
import { NotifyDialogsComponent } from './shell/notify-dialogs.component';
import { ShellDialogsComponent } from './shell/shell-dialogs.component';
import { TaoVanBanComponent } from './tao-van-ban/tao-van-ban.component';
import { ViTriComponent } from './vi-tri/vi-tri.component';

/** Phím Ctrl+<chữ> mở màn (L805–808). */
const CTRL_KEYS: Record<string, Screen> = { k: 'kyso', n: 'tvb', e: 'kt' };

/**
 * Vỏ ứng dụng: thanh menu, màn hiện tại (đổi bằng signal, không dùng router), hộp thoại chung,
 * phím tắt và đóng lớp nổi khi bấm ra ngoài theo đúng thứ tự của prototype (mục 5.2, 5.3).
 */
@Component({
  selector: 'app-root',
  imports: [MenubarComponent, HomeComponent, KySoComponent, KiemTraComponent, DevScreenComponent, TaoVanBanComponent, ViTriComponent, ShellDialogsComponent, ConfigDialogComponent, NotifyDialogsComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(document:keydown)': 'onKey($event)',
    '(document:click)': 'onClick($event)',
  },
  template: `
    <app-menubar />
    @switch (ui.screen()) {
      @case ('home') {
        <app-home />
      }
      @case ('kyso') {
        <app-ky-so />
      }
      @case ('tvb') {
        <app-tao-van-ban />
      }
      @case ('tucauhinh') {
        <app-vi-tri />
      }
      @case ('kt') {
        <app-kiem-tra />
      }
      @default {
        <app-dev-screen [screen]="ui.screen()" />
      }
    }
    <app-shell-dialogs />
    <app-config-dialog />
    <app-notify-dialogs />
  `,
})
export class App {
  protected readonly ui = inject(UiService);
  private readonly title = inject(Title);

  constructor() {
    effect(() => this.title.setTitle(`Sao Mai Ký số — ${SCREEN_TITLES[this.ui.screen()]}`));
  }

  /** Bấm ra ngoài thì đóng lớp nổi: menu bar → tooltip → menu chọn nguồn (L787–791). */
  protected onClick(event: MouseEvent): void {
    const target = event.target instanceof Element ? event.target : null;
    if (this.ui.menu() && !target?.closest('.menubar')) {
      this.ui.menu.set(null);
    }
    if (this.ui.tip() !== null && !target?.closest('.tip')) {
      this.ui.tip.set(null);
    }
    if (this.ui.sourcePick() && !target?.closest('.lt-pick')) {
      this.ui.sourcePick.set(null);
    }
  }

  /** Phím tắt theo đúng thứ tự xét của prototype (L797–809). Mỗi lần Esc chỉ đóng một tầng. */
  protected onKey(event: KeyboardEvent): void {
    const target = event.target instanceof Element ? event.target : null;
    const key = event.key;
    // 1. Enter trong ô giá trị cố định: ô đó tự xử lý.
    if (key === 'Enter' && target?.closest('.lt-menu')) {
      return;
    }
    // 2. Esc khi menu chọn nguồn đang mở.
    if (key === 'Escape' && this.ui.sourcePick()) {
      this.ui.sourcePick.set(null);
      return;
    }
    // 3. Esc khi có tooltip mở.
    if (key === 'Escape' && this.ui.tip() !== null) {
      this.ui.tip.set(null);
      return;
    }
    // 4. Enter / Space trên nút "!": nút đó tự xử lý.
    if ((key === 'Enter' || key === ' ') && target?.classList.contains('tip')) {
      return;
    }
    // 5. Esc: hộp thoại, rồi tới menu.
    if (key === 'Escape') {
      if (this.ui.dialog()) {
        this.ui.close();
      } else if (this.ui.menu()) {
        this.ui.menu.set(null);
      }
      return;
    }
    // 6. F1: Hướng dẫn sử dụng.
    if (key === 'F1') {
      event.preventDefault();
      if (this.ui.dialog()?.kind !== 'help') {
        this.ui.open({ kind: 'help', topic: 0 });
      }
      return;
    }
    // 7. Ctrl+K / Ctrl+N / Ctrl+E.
    if (event.ctrlKey && !event.shiftKey && !event.altKey) {
      const screen = CTRL_KEYS[key.toLowerCase()];
      if (screen) {
        event.preventDefault();
        this.ui.go(screen);
      }
    }
  }
}
