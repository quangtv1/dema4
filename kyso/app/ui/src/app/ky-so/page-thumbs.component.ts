import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterRenderEffect,
  computed,
  effect,
  inject,
  input,
  output,
  untracked,
} from '@angular/core';

import { PdfViewComponent, THUMB_WIDTH } from '../tao-van-ban/pdf-view.component';

/**
 * Cột ảnh thu nhỏ các trang của văn bản đang xem (văn bản từ 2 trang): bấm một trang thì vùng xem cuộn tới trang
 * đó; trang đang xem được tô; trang có khung chữ ký có dấu. Ảnh chỉ vẽ khi lướt tới (văn bản dài không vẽ hết một
 * lượt), vẽ lại khi đổi văn bản.
 */
@Component({
  selector: 'app-page-thumbs',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'trang-nho', role: 'navigation', 'aria-label': 'Các trang' },
  template: `
    @for (n of pages(); track n) {
      <button
        class="tn-trang"
        type="button"
        [attr.data-trang]="n"
        [class.chon]="n === view().currentPage()"
        [class.co-ky]="marks().has(n)"
        [title]="marks().has(n) ? 'Trang ' + n + ' · có khung chữ ký' : 'Trang ' + n"
        (click)="view().goToPage(n)"
        (contextmenu)="onMenu($event, n)"
      >
        <span class="tn-giay"><canvas></canvas></span>
        <span class="tn-so">{{ n }}</span>
      </button>
    }
  `,
})
export class PageThumbsComponent {
  readonly view = input.required<PdfViewComponent>();
  /** Trang (từ 1) có khung chữ ký. */
  readonly marks = input<ReadonlySet<number>>(new Set());
  /** Bật thì chuột phải vào một trang báo `pageMenu` (màn chứa mở menu thêm / xoá khung ở trang đó). */
  readonly pageTools = input(false);
  readonly pageMenu = output<{ page: number; clientX: number; clientY: number }>();

  protected readonly pages = computed(() => Array.from({ length: this.view().pageCount() }, (_, i) => i + 1));

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private observer: IntersectionObserver | null = null;
  private drawn = new Set<number>();

  constructor() {
    // Văn bản mới: quên các ảnh đã vẽ.
    effect(() => {
      this.view().docVersion();
      untracked(() => (this.drawn = new Set()));
    });
    // Sau mỗi lần vẽ danh sách: theo dõi ảnh nào lọt vào tầm nhìn thì vẽ ảnh đó.
    afterRenderEffect(() => {
      this.pages();
      this.view().docVersion();
      this.observe();
    });
    // Trang đang xem luôn nằm trong cột ảnh thu nhỏ.
    effect(() => {
      const n = this.view().currentPage();
      untracked(() =>
        this.host.nativeElement.querySelector(`[data-trang="${n}"]`)?.scrollIntoView({ block: 'nearest' }),
      );
    });
    inject(DestroyRef).onDestroy(() => this.observer?.disconnect());
  }

  protected onMenu(e: MouseEvent, page: number): void {
    if (!this.pageTools()) {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    this.view().goToPage(page);
    this.pageMenu.emit({ page, clientX: e.clientX, clientY: e.clientY });
  }

  private observe(): void {
    this.observer?.disconnect();
    this.observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (!e.isIntersecting) {
            continue;
          }
          const el = e.target as HTMLElement;
          const n = Number(el.dataset['trang']);
          const canvas = el.querySelector('canvas');
          if (n && canvas && !this.drawn.has(n)) {
            this.drawn.add(n);
            void this.view().renderThumb(n, canvas, THUMB_WIDTH);
          }
        }
      },
      { root: this.host.nativeElement, rootMargin: '200px 0px' },
    );
    this.host.nativeElement.querySelectorAll<HTMLElement>('.tn-trang').forEach((el) => this.observer?.observe(el));
  }
}
