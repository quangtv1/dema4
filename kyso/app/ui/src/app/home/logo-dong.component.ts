import { ChangeDetectionStrategy, Component, ElementRef, afterNextRender, inject, viewChild, viewChildren, DestroyRef } from '@angular/core';

/** 4 điểm điều khiển Bézier bậc 3 trong hệ `viewBox="0 0 300 246"` (prototype L861). */
const D: ReadonlyArray<readonly [number, number]> = [
  [100, 110],
  [170, 100],
  [236, 62],
  [212, 23],
];
/** Chu kỳ, thời gian sao bay, thời gian nhấp nháy (giây). */
const CK = 5;
const TV = 0.7;
const TN = 0.3;
/** Hạt vệt k = 6…1, bán kính 2 + 0.7·(7−k) (L885). */
const TRAIL = [6, 5, 4, 3, 2, 1].map((k) => ({ k, r: (2 + 0.7 * (7 - k)).toFixed(1) }));

function bez(t: number): [number, number] {
  const u = 1 - t;
  const k = [u * u * u, 3 * u * u * t, 3 * u * t * t, t * t * t];
  return [0, 1].map((i) => k[0] * D[0][i] + k[1] * D[1][i] + k[2] * D[2][i] + k[3] * D[3][i]) as [number, number];
}

function smooth(t: number): number {
  return t * t * (3 - 2 * t);
}

/**
 * Logo động Trang chủ (prototype L859–887): ngôi sao bay theo đường cong rồi nhấp nháy, lặp mỗi 5 giây.
 * Chạy bằng requestAnimationFrame, sửa thẳng thuộc tính SVG và `element.style` (CSSOM, hợp lệ với CSP);
 * vòng lặp dừng khi component bị huỷ (prototype chạy mãi kể cả khi không ở Trang chủ).
 */
@Component({
  selector: 'app-logo-dong',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'logo-dong' },
  template: `
    <img alt="Sao Mai" src="img/logo.png" />
    <img #sao class="ld-sao" alt="" src="img/sao.png" />
    <svg viewBox="0 0 300 246" aria-hidden="true">
      @for (p of trail; track p.k) {
        <circle #vet class="ld-vet" [attr.r]="p.r" fill="#FFD06A" opacity="0" [attr.data-k]="p.k" />
      }
      <g #dau class="ld-dau" opacity="0">
        <circle r="9" fill="#FFD06A" opacity=".45" />
        <circle r="4.5" fill="#fff" />
      </g>
    </svg>
  `,
})
export class LogoDongComponent {
  protected readonly trail = TRAIL;
  private readonly sao = viewChild.required<ElementRef<HTMLImageElement>>('sao');
  private readonly dau = viewChild.required<ElementRef<SVGGElement>>('dau');
  private readonly vet = viewChildren<ElementRef<SVGCircleElement>>('vet');

  constructor() {
    let frame = 0;
    const t0 = performance.now();
    afterNextRender(() => {
      const tick = (now: number) => {
        this.draw(((now - t0) / 1000) % CK);
        frame = requestAnimationFrame(tick);
      };
      frame = requestAnimationFrame(tick);
    });
    inject(DestroyRef).onDestroy(() => cancelAnimationFrame(frame));
  }

  private draw(t: number): void {
    const sao = this.sao().nativeElement;
    const dau = this.dau().nativeElement;
    const vet = this.vet().map((ref) => ref.nativeElement);
    if (t < TV) {
      const u = smooth(t / TV);
      const p = bez(u);
      const to = 1 + 1.4 * Math.max(0, (u - 0.75) / 0.25);
      dau.setAttribute('opacity', '1');
      dau.setAttribute('transform', `translate(${p[0]} ${p[1]}) scale(${to.toFixed(3)})`);
      for (const c of vet) {
        const k = Number(c.getAttribute('data-k'));
        const v = u - k * 0.04;
        if (v <= 0) {
          c.setAttribute('opacity', '0');
          continue;
        }
        const q = bez(v);
        c.setAttribute('cx', String(q[0]));
        c.setAttribute('cy', String(q[1]));
        c.setAttribute('opacity', (0.55 * (1 - k / 7)).toFixed(2));
      }
      sao.style.opacity = String(Math.max(0.2, 1 - (t / 0.15) * 0.8));
      sao.style.transform = '';
    } else {
      dau.setAttribute('opacity', '0');
      for (const c of vet) {
        c.setAttribute('opacity', '0');
      }
      if (t < TV + TN) {
        const s = (t - TV) / TN;
        sao.style.opacity = String(Math.min(1, 0.2 + (0.8 * s) / 0.4));
        sao.style.transform = `scale(${(1 + 0.5 * Math.sin(Math.PI * s)).toFixed(3)})`;
      } else {
        sao.style.opacity = '1';
        sao.style.transform = '';
      }
    }
  }
}
