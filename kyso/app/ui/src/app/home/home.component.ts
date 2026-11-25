import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { Screen, UiService } from '../core/ui.service';
import { IpcService } from '../core/ipc.service';
import { BigIconComponent } from '../shared/icon.component';
import { BigIconName } from '../shared/icons';
import { TipComponent } from '../shared/tip.component';
import { LogoDongComponent } from './logo-dong.component';

interface Card {
  id: Screen;
  icon: BigIconName;
  title: string;
  key: string;
  tom: string;
}

/**
 * Thẻ chức năng, nguyên văn `manHome()` L941–946, trừ thẻ "Vị trí chữ ký" (kéo thả) đã bỏ theo yêu cầu người phụ
 * trách ngày 20/11/2026: vị trí chữ ký đặt ở nút "Cấu hình vị trí" của màn Ký số.
 */
const CARDS: Card[] = [
  {
    id: 'kyso',
    icon: 'ky',
    title: 'Ký số văn bản',
    key: 'Ctrl+K',
    tom: 'Ký 1 file hoặc cả thư mục PDF/DOCX bằng USB Token; chèn ảnh chữ ký, con dấu theo khổ giấy; tự nhập lại PIN khi token hết phiên.',
  },
  {
    id: 'tvb',
    icon: 'tao',
    title: 'Tạo văn bản',
    key: 'Ctrl+N',
    tom: 'Sinh hàng loạt văn bản từ mẫu DOCX/HTML và dữ liệu Excel (hoặc nhập trực tiếp); xuất PDF hoặc DOCX.',
  },
  {
    id: 'kt',
    icon: 'kiemtra',
    title: 'Kiểm tra chữ ký',
    key: 'Ctrl+E',
    tom: 'Xác minh chữ ký trong file PDF: người ký, thời điểm ký, chứng thư và văn bản có bị sửa sau khi ký.',
  },
];

/** Trang chủ — prototype `manHome()` L940–954, mục 3.1 bản phân tích. */
@Component({
  selector: 'app-home',
  imports: [BigIconComponent, TipComponent, LogoDongComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'man trang-chu' },
  template: `
    <div class="tc-dau">
      <a class="logo-dinh" href="https://saomaiso.com" target="_blank" rel="noopener" title="saomaiso.com"><app-logo-dong /></a>
      <h1>Ký số văn bản điện tử</h1>
      <p>Chọn chức năng để bắt đầu <span [tip]="introTip"></span></p>
    </div>
    <div class="tc-luoi">
      @for (c of cards; track c.id) {
        <button class="tc-the" type="button" (click)="ui.go(c.id)">
          <span class="tren" [tip]="c.tom + (c.key ? '\\nPhím tắt: ' + c.key : '')"></span>
          <svg class="la" [bigIcon]="c.icon"></svg>
          <b>{{ c.title }}</b>
        </button>
      }
    </div>
    <div class="tc-day">
      <span>
        <span><a href="https://saomaiso.com" target="_blank" rel="noopener">saomaiso.com</a></span>
        <span><a href="mailto:hotro@saomaiso.com">hotro@saomaiso.com</a></span>
      </span>
      <span class="phien-ban">Phiên bản<b>{{ version() }}</b><span class="tren" [tip]="userTip()"></span></span>
    </div>
  `,
})
export class HomeComponent {
  protected readonly ui = inject(UiService);
  private readonly ipc = inject(IpcService);
  protected readonly cards = CARDS;

  /** Phụ lục A, L948 — nguyên văn. */
  protected readonly introTip =
    'Ký ngay trên máy — văn bản không gửi lên máy chủ.\nGiữ nguyên file gốc; file đã ký lưu riêng thư mục DaKySo.\nĐịnh dạng: PDF · DOCX · ảnh chữ ký PNG.\nPhím tắt: F1 hướng dẫn · Ctrl+K ký số · Ctrl+N tạo văn bản · Ctrl+E kiểm tra.';

  protected readonly version = computed(() => this.ui.info()?.version ?? '—');

  /** Phụ lục A, L952, thay giá trị mô phỏng bằng dữ liệu thật từ `appInfo()`. */
  protected readonly userTip = computed(() => {
    const info = this.ui.info();
    if (info) {
      return `Người dùng: ${info.userName}\nTên PC: ${info.computerName}\nBản quyền: Giải pháp số Sao Mai`;
    }
    const reason = this.ipc.available
      ? (this.ui.infoError() ?? 'Đang đọc thông tin…')
      : 'Không có lõi ứng dụng (đang chạy trong trình duyệt).';
    return `${reason}\nBản quyền: Giải pháp số Sao Mai`;
  });
}
