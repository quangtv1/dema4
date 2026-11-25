import { ChangeDetectionStrategy, Component, DestroyRef, computed, effect, inject, signal, viewChild } from '@angular/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';

import { CheckState, RevocationStatus, SignatureVerify, VerifyCheck } from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { BigIconComponent, IconComponent } from '../shared/icon.component';
import { IconName } from '../shared/icons';
import { TipComponent } from '../shared/tip.component';
import { AdvancedButtonComponent, PathInputComponent } from '../shared/widgets';
import { MarkBoxView, PdfViewComponent } from '../tao-van-ban/pdf-view.component';
import { KiemTraStore, Tone, VERDICT_TEXT, checkTone, verdictTone } from './kiem-tra.store';

const TONE_ICON: Record<Tone, IconName> = { ok: 'checkC', warn: 'warn', fail: 'alert' };
const REVOCATION_TEXT: Record<RevocationStatus, string> = {
  notChecked: 'Chưa kiểm tra',
  good: 'Chưa bị thu hồi (CRL/OCSP)',
  revokedAfterSigning: 'Bị thu hồi sau thời điểm ký',
  revoked: 'Đã bị thu hồi',
  unknown: 'Không kiểm tra được (mất mạng hoặc máy chủ CA không trả lời)',
};

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/**
 * Màn Kiểm tra chữ ký số (prototype `manKT()` L1311–1356, TD-57): chọn PDF đã ký, xem từng chữ ký theo 5 điều kiện
 * của Nghị định 68/2024/NĐ-CP Điều 30, vị trí chữ ký trên văn bản, xuất báo cáo ra PDF.
 */
@Component({
  selector: 'app-kiem-tra',
  imports: [IconComponent, BigIconComponent, TipComponent, PathInputComponent, AdvancedButtonComponent, PdfViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man kiem-tra', '(document:keydown)': 'onKey($event)' },
  templateUrl: './kiem-tra.component.html',
})
export class KiemTraComponent {
  protected readonly s = inject(KiemTraStore);
  protected readonly ui = inject(UiService);
  protected readonly dragging = signal(false);
  protected readonly resizing = signal(false);
  protected readonly toneIcon = TONE_ICON;
  protected readonly revText = REVOCATION_TEXT;
  private readonly pv = viewChild<PdfViewComponent>('pv');

  /** Khung đánh dấu các chữ ký hiển thị trên trang. */
  protected readonly marks = computed<MarkBoxView[]>(() => {
    const sel = this.s.selected();
    return this.s.signatures().flatMap((sig, i) =>
      sig.placement
        ? [
            {
              placement: sig.placement,
              label: String(i + 1),
              tone: this.tone(sig),
              selected: i === sel,
              title: `${i + 1}. ${this.signerName(sig)} — ${VERDICT_TEXT[sig.verdict]}`,
            },
          ]
        : [],
    );
  });
  /** Chỉ số khung đánh dấu của chữ ký đang chọn (chữ ký ẩn thì không có). */
  private readonly markIndex = computed(() => {
    const sigs = this.s.signatures();
    const sel = this.s.selected();
    if (!sigs[sel]?.placement) {
      return -1;
    }
    return sigs.slice(0, sel).filter((x) => x.placement).length;
  });

  constructor() {
    const destroyRef = inject(DestroyRef);
    if (this.s.available) {
      let unlisten: (() => void) | null = null;
      let destroyed = false;
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const p = event.payload;
          if (p.type === 'enter' || p.type === 'over') {
            this.dragging.set(!this.s.busy());
          } else if (p.type === 'leave') {
            this.dragging.set(false);
          } else if (p.type === 'drop') {
            this.dragging.set(false);
            void this.s.dropPaths(p.paths);
          }
        })
        .then((fn) => (destroyed ? fn() : (unlisten = fn)))
        .catch(() => undefined);
      destroyRef.onDestroy(() => {
        destroyed = true;
        unlisten?.();
      });
    }
    // Sang tab Văn bản hoặc đổi chữ ký đang chọn: cuộn tới khung của chữ ký đó.
    effect(() => {
      const i = this.markIndex();
      const tab = this.s.tab();
      this.marks();
      if (tab === 'vb' && i >= 0) {
        setTimeout(() => this.pv()?.focusMark(i), 60);
      }
    });
  }

  protected tone(sig: SignatureVerify): Tone {
    return verdictTone(sig.verdict);
  }

  protected verdictText(sig: SignatureVerify): string {
    return VERDICT_TEXT[sig.verdict];
  }

  protected signerName(sig: SignatureVerify): string {
    if (sig.kind === 'docTimestamp') {
      return 'Dấu thời gian tài liệu';
    }
    return sig.signedBy || 'Không rõ người ký';
  }

  protected checkIcon(state: CheckState): IconName {
    return TONE_ICON[checkTone(state)];
  }

  protected checkClass(c: VerifyCheck): string {
    return 'chip-tt ' + (c.state === 'ok' ? 'ok' : c.state === 'fail' ? 'loi' : 'dung');
  }

  protected where(sig: SignatureVerify): string {
    if (!sig.visible) {
      return sig.page ? `Chữ ký ẩn (trang ${sig.page})` : 'Chữ ký ẩn';
    }
    return sig.page ? `Trang ${sig.page}` : 'Có hiển thị';
  }

  protected scope(sig: SignatureVerify): string {
    if (sig.coversWholeFile) {
      return 'Phủ toàn bộ văn bản';
    }
    switch (sig.changes) {
      case 'signatures':
      case 'none':
        return 'Phủ bản đã ký; sau đó chỉ ký thêm / bổ sung dữ liệu kiểm tra';
      case 'formFill':
        return 'Phủ bản đã ký; sau đó có điền biểu mẫu';
      case 'annotations':
        return 'Phủ bản đã ký; sau đó có thêm chú thích';
      default:
        return 'Phủ bản đã ký; sau đó văn bản bị sửa';
    }
  }

  protected docmdpText(p: number | null): string {
    switch (p) {
      case 1:
        return 'Chữ ký chứng nhận: không cho phép thay đổi';
      case 2:
        return 'Chữ ký chứng nhận: chỉ cho điền biểu mẫu, ký thêm';
      case 3:
        return 'Chữ ký chứng nhận: cho điền biểu mẫu, ký thêm, chú thích';
      default:
        return '';
    }
  }

  /** Chữ ở chân màn. */
  protected readonly footText = computed(() => {
    const n = this.s.signatures().length;
    const ok = this.s.validCount();
    return { n, ok };
  });

  protected onMarkClick(markIndex: number): void {
    const withBox = this.s
      .signatures()
      .map((sig, i) => (sig.placement ? i : -1))
      .filter((i) => i >= 0);
    const i = withBox[markIndex];
    if (i !== undefined) {
      this.s.select(i);
    }
  }

  protected showOnPage(): void {
    this.s.tab.set('vb');
  }

  protected onRevocation(e: Event): void {
    this.s.setRevocation((e.target as HTMLInputElement).checked);
  }

  protected onKey(e: KeyboardEvent): void {
    const target = e.target instanceof Element ? e.target : null;
    if (target?.closest('input, textarea, select') || !this.s.loaded()) {
      return;
    }
    if (e.ctrlKey && e.key === 'ArrowDown') {
      e.preventDefault();
      this.s.select(this.s.selected() + 1);
    } else if (e.ctrlKey && e.key === 'ArrowUp') {
      e.preventDefault();
      this.s.select(this.s.selected() - 1);
    }
  }

  protected startResize(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    const bar = event.currentTarget as HTMLElement;
    const parent = bar.parentElement;
    const startX = event.clientX;
    const orig = this.s.leftW();
    this.resizing.set(true);
    document.body.style.cursor = 'col-resize';
    const move = (e: PointerEvent) => {
      const maxW = parent ? Math.max(320, parent.clientWidth - 360) : 520;
      this.s.leftW.set(clamp(orig + (e.clientX - startX), 220, maxW));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      document.body.style.cursor = '';
      this.resizing.set(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }
}
