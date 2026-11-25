import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';

import { BIG_ICONS, BigIconName, ICONS, IconName } from './icons';

/**
 * Icon nét 24×24 của prototype (`ic()` L592). Dùng: `<svg icon="doc"></svg>`, thêm class `nho`, `mui`, `quay`
 * như prototype. Nội dung SVG là hằng số trong `icons.ts` nên đánh dấu tin cậy được.
 */
@Component({
  selector: 'svg[icon]',
  template: '',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    class: 'i',
    'aria-hidden': 'true',
    '[attr.viewBox]': "'0 0 24 24'",
    '[innerHTML]': 'html()',
  },
})
export class IconComponent {
  private readonly sanitizer = inject(DomSanitizer);
  readonly icon = input.required<IconName>();
  protected readonly html = computed<SafeHtml>(() => this.sanitizer.bypassSecurityTrustHtml(ICONS[this.icon()]));
}

/** Icon nét lớn 100×100 (`la()` L602): thẻ Trang chủ (class `la`) và panel giới thiệu (class `logo-cn`). */
@Component({
  selector: 'svg[bigIcon]',
  template: '',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    'aria-hidden': 'true',
    '[attr.viewBox]': "'0 0 100 100'",
    '[innerHTML]': 'html()',
  },
})
export class BigIconComponent {
  private readonly sanitizer = inject(DomSanitizer);
  readonly bigIcon = input.required<BigIconName>();
  protected readonly html = computed<SafeHtml>(() => this.sanitizer.bypassSecurityTrustHtml(BIG_ICONS[this.bigIcon()]));
}
