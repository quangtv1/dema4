import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';

import { UiService } from '../core/ui.service';

let nextId = 1;

/**
 * Nút "!" bấm để xem thông tin (prototype `tip()` L605–610, CSS L466–478).
 * Nội dung hiện qua `::after{content:attr(data-tip)}`, xuống dòng bằng `\n`. Thêm class `trai` / `tren` để
 * đổi hướng bong bóng như prototype. Đóng khi bấm lại, bấm ra ngoài hoặc nhấn Esc (xử lý ở `App`).
 */
@Component({
  selector: 'span[tip]',
  template: '!',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    class: 'tip',
    role: 'button',
    tabindex: '0',
    'aria-label': 'Xem thông tin',
    '[class.mo]': 'open()',
    '[attr.data-tip]': 'tip()',
    '[attr.aria-expanded]': 'open()',
    '(click)': 'toggle($event)',
    '(keydown.enter)': 'toggle($event)',
    '(keydown.space)': 'toggle($event)',
  },
})
export class TipComponent {
  private readonly ui = inject(UiService);
  private readonly id = nextId++;

  readonly tip = input.required<string>();
  protected readonly open = computed(() => this.ui.tip() === this.id);

  protected toggle(event: Event): void {
    // Nút "!" có thể nằm trong một nút khác (thẻ Trang chủ): không để cú bấm lan ra ngoài.
    event.preventDefault();
    event.stopPropagation();
    this.ui.menu.set(null);
    this.ui.sourcePick.set(null);
    this.ui.tip.set(this.open() ? null : this.id);
  }
}
