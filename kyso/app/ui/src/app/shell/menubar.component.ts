import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { Screen, UiService } from '../core/ui.service';
import { IconComponent } from '../shared/icon.component';
import { IconName } from '../shared/icons';

interface MenuItem {
  label: string;
  icon: IconName;
  key?: string;
  /** Màn ứng với mục (để hiện ✓), hoặc không. */
  screen?: Screen;
  run: () => void;
}

/**
 * Thanh menu — prototype `menuDefs()` L677–697, `menubarHTML()` L699–723, mục 5.1 bản phân tích.
 * Không có thanh tiêu đề tự vẽ (cửa sổ dùng thanh tiêu đề Windows) và không có chỉ báo USB Token.
 */
@Component({
  selector: 'app-menubar',
  imports: [IconComponent, NgTemplateOutlet],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'menubar' },
  template: `
    <div class="mb-o">
      <button class="mb" type="button" aria-haspopup="menu" [class.mo]="ui.menu() === 'cn'" (click)="toggle('cn')">
        <svg icon="grid"></svg>Chức năng<svg icon="chevD" class="mui"></svg>
      </button>
      @if (ui.menu() === 'cn') {
        <div class="menu" role="menu">
          @for (it of chucNang; track it.label; let i = $index) {
            @if (i === 1) {
              <div class="mi-sep"></div>
            }
            <ng-container *ngTemplateOutlet="item; context: { $implicit: it }" />
          }
        </div>
      }
    </div>
    <div class="mb-o">
      <button class="mb" type="button" [class.mo]="cauHinhMo()" (click)="ui.openConfig()">
        <svg icon="gear"></svg>Cấu hình
      </button>
    </div>
    <div class="mb-o">
      <button class="mb" type="button" aria-haspopup="menu" [class.mo]="ui.menu() === 'ht'" (click)="toggle('ht')">
        <svg icon="help"></svg>Hỗ trợ<svg icon="chevD" class="mui"></svg>
      </button>
      @if (ui.menu() === 'ht') {
        <div class="menu" role="menu">
          @for (it of hoTro; track it.label; let i = $index) {
            @if (i === 4) {
              <div class="mi-sep"></div>
            }
            <ng-container *ngTemplateOutlet="item; context: { $implicit: it }" />
          }
        </div>
      }
    </div>

    <ng-template #item let-it>
      <button class="mi" type="button" role="menuitem" [class.dang]="isOn(it)" (click)="pick(it)">
        <svg [icon]="it.icon"></svg>
        <span class="nhan">{{ it.label }}</span>
        @if (it.key) {
          <span class="phim">{{ it.key }}</span>
        }
        <span class="dau">{{ isOn(it) ? '✓' : '' }}</span>
      </button>
    </ng-template>
  `,
})
export class MenubarComponent {
  protected readonly ui = inject(UiService);

  protected readonly chucNang: MenuItem[] = [
    { label: 'Trang chủ', icon: 'home', screen: 'home', run: () => this.ui.go('home') },
    { label: 'Ký số văn bản', icon: 'pen', key: 'Ctrl+K', screen: 'kyso', run: () => this.ui.go('kyso') },
    { label: 'Tạo văn bản', icon: 'tao', key: 'Ctrl+N', screen: 'tvb', run: () => this.ui.go('tvb') },
    { label: 'Kiểm tra chữ ký số', icon: 'shield', key: 'Ctrl+E', screen: 'kt', run: () => this.ui.go('kt') },
  ];

  protected readonly hoTro: MenuItem[] = [
    { label: 'Hướng dẫn sử dụng', icon: 'book', key: 'F1', run: () => this.ui.open({ kind: 'help', topic: 0 }) },
    { label: 'Cài đặt driver USB Token…', icon: 'usb', run: () => this.ui.open({ kind: 'driver' }) },
    { label: 'Thông báo Telegram…', icon: 'send', run: () => this.ui.open({ kind: 'telegram' }) },
    { label: 'Kiểm tra cập nhật…', icon: 'refresh', run: () => this.ui.openDev('Kiểm tra cập nhật', 'refresh') },
    { label: 'Thông tin phần mềm', icon: 'info', run: () => this.ui.open({ kind: 'info' }) },
  ];

  /** Nút "Cấu hình" sáng khi hộp Cấu hình đang mở (L705). */
  protected readonly cauHinhMo = computed(() => {
    const d = this.ui.dialog();
    return d?.kind === 'cauhinh';
  });

  protected isOn(it: MenuItem): boolean {
    return it.screen !== undefined && it.screen === this.ui.screen();
  }

  protected toggle(id: 'cn' | 'ht'): void {
    this.ui.tip.set(null);
    this.ui.menu.update((m) => (m === id ? null : id));
  }

  protected pick(it: MenuItem): void {
    this.ui.menu.set(null);
    it.run();
  }
}
