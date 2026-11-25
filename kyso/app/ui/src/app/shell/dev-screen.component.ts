import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';

import { SCREEN_TITLES, Screen, UiService } from '../core/ui.service';
import { BigIconComponent } from '../shared/icon.component';
import { BigIconName } from '../shared/icons';

const ICON: Partial<Record<Screen, BigIconName>> = { kyso: 'ky', kt: 'kiemtra', tucauhinh: 'vitri' };

/** Màn của chức năng chưa làm: chỉ ghi rõ "Chức năng đang phát triển", không dựng giao diện giả. */
@Component({
  selector: 'app-dev-screen',
  imports: [BigIconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man' },
  template: `
    <div class="dang-phat-trien">
      <div class="mo-dau">
        <svg class="logo-cn" [bigIcon]="icon()"></svg>
        <h2>{{ title() }}</h2>
        <p>Chức năng đang phát triển.</p>
        <button class="logo-goc" type="button" title="Trang chủ" (click)="ui.go('home')">
          <img alt="Sao Mai" src="img/full.png" />
        </button>
      </div>
    </div>
  `,
})
export class DevScreenComponent {
  protected readonly ui = inject(UiService);
  readonly screen = input.required<Screen>();
  protected readonly title = computed(() => SCREEN_TITLES[this.screen()]);
  protected readonly icon = computed(() => ICON[this.screen()] ?? 'ky');
}
