import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';

import { IconComponent } from './icon.component';
import { IconName } from './icons';

/**
 * Ô đường dẫn chỉ đọc có nút xoá (prototype `oInput()` L763–767). Nút xoá ẩn khi ô rỗng (`.o.trong`).
 * Dùng: `<span pathInput [value]="..." placeholder="..." (cleared)="..."></span>`.
 */
@Component({
  selector: 'span[pathInput]',
  imports: [IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'o', '[class.trong]': '!value()' },
  template: `
    <input class="fld" type="text" readonly [value]="value()" [placeholder]="placeholder()" [title]="value()" [disabled]="disabled()" />
    <button class="xoa" type="button" title="Xoá đường dẫn" [disabled]="disabled()" (click)="cleared.emit()">
      <svg icon="xc" class="nho"></svg>
    </button>
  `,
})
export class PathInputComponent {
  readonly value = input<string>('');
  readonly placeholder = input<string>('');
  readonly disabled = input<boolean>(false);
  readonly cleared = output<void>();
}

/**
 * Nút Nâng cao (prototype `advBtn()` L888–890): bánh răng + nhãn + mũi tên xoay 180° khi mở.
 * Dùng: `<button advBtn [open]="mo()" (click)="..."></button>`.
 */
@Component({
  selector: 'button[advBtn]',
  imports: [IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'nut nho nangcao', type: 'button', '[class.mo]': 'open()', '[attr.aria-expanded]': 'open()' },
  template: `<svg icon="gear" class="nho"></svg>{{ label() }}<svg icon="chevD" class="mui"></svg>`,
})
export class AdvancedButtonComponent {
  readonly open = input<boolean>(false);
  readonly label = input<string>('Nâng cao');
}

/**
 * Hộp thoại `.popup` trên lớp phủ `.phu` (prototype `popup()` L1426–1429). Bấm nền hoặc ✕ thì phát `closed`.
 * Thân hộp đặt trực tiếp làm nội dung; chân hộp đặt trong phần tử có thuộc tính `foot`.
 */
@Component({
  selector: 'app-popup',
  imports: [IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="phu" (click)="onBackdrop($event)">
      <div [class]="'popup ' + size()" role="dialog" aria-modal="true" [attr.aria-label]="title()">
        <div class="dau-p">
          @if (icon(); as name) {
            <svg [icon]="name"></svg>
          }
          <span>{{ title() }}</span>
          <button class="nutma x" type="button" aria-label="Đóng" (click)="closed.emit()"><svg icon="x"></svg></button>
        </div>
        <ng-content />
        <div class="cuoi"><ng-content select="[foot]" /></div>
      </div>
    </div>
  `,
})
export class PopupComponent {
  readonly title = input.required<string>();
  readonly icon = input<IconName | null>(null);
  /** Class độ rộng: `w440`, `w460`, `w470`, `w560`. */
  readonly size = input<string>('w440');
  readonly closed = output<void>();

  protected onBackdrop(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      this.closed.emit();
    }
  }
}
