import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked } from '@angular/core';

import { CertEntry, DriverStatus, IpcService, errorMessage } from '../core/ipc.service';
import { UiService } from '../core/ui.service';
import { IconComponent } from '../shared/icon.component';
import { PopupComponent } from '../shared/widgets';
import { HELP, HelpPart, helpParts } from './help-content';

/**
 * Hộp thoại của vỏ ứng dụng:
 * - "Chức năng đang phát triển" (Kiểm tra cập nhật…);
 * - "Thông tin phần mềm" (prototype `mThongTin()` L1527–1532) lấy số liệu từ `appInfo()`;
 * - "Hướng dẫn sử dụng" (HT-01, prototype `mHuongDan()` L1512–1520), nội dung ở `help-content.ts`;
 * - "Cài đặt driver USB Token" (HT-02, prototype `mDriver()` L1533–1539): kiểm tra driver thật trên máy, chưa có
 *   thì hướng dẫn tải ở trang phần mềm của Ban Cơ yếu Chính phủ.
 */
@Component({
  selector: 'app-shell-dialogs',
  imports: [PopupComponent, IconComponent, NgTemplateOutlet],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @let d = ui.dialog();
    @if (d && d.kind === 'dev') {
      <app-popup [title]="d.title" [icon]="d.icon" size="w440" (closed)="ui.close()">
        <div class="than-p giua">
          <div class="tieu-p">Chức năng đang phát triển</div>
          <div class="hint mo-ta">Chức năng này sẽ có trong bản cập nhật sau.</div>
        </div>
        <button foot class="nut chinh" type="button" (click)="ui.close()">Đóng</button>
      </app-popup>
    } @else if (d && d.kind === 'info') {
      <app-popup title="Thông tin phần mềm" icon="info" size="w460" (closed)="ui.close()">
        <div class="logott"><img alt="Sao Mai Digital Solutions" src="img/full.png" /></div>
        <div class="than-p">
          @if (ui.info(); as info) {
            <div class="luoi-tt">
              <span class="k">Ứng dụng</span><span><b>{{ info.name }}</b></span>
              <span class="k">Phiên bản</span><span>{{ info.version }}</span>
              <span class="k">Chức năng</span><span>Ký số · Tạo văn bản (Kiểm tra chữ ký đang phát triển)</span>
              <span class="k">Microsoft Word</span>
              <span>{{ info.wordAvailable ? 'Có — xuất được PDF' : 'Không tìm thấy — chỉ xuất được DOCX' }}</span>
              <span class="k">CA nhúng</span><span>{{ info.pinnedCaCount }} chứng thư đã kiểm tra</span>
              <span class="k">Người dùng</span><span>{{ info.userName }} · {{ info.computerName }}</span>
              <span class="k">Đơn vị</span><span>Giải pháp số Sao Mai</span>
              <span class="k">Website</span><span><a href="https://saomaiso.com" target="_blank" rel="noopener">saomaiso.com</a></span>
              <span class="k">Email hỗ trợ</span><span><a href="mailto:hotro@saomaiso.com">hotro@saomaiso.com</a></span>
            </div>
          } @else {
            <div class="bao canh">
              <span>
                <b>Không đọc được thông tin phần mềm</b>
                {{ ipc.available ? (ui.infoError() ?? 'Đang đọc…') : 'Đang chạy trong trình duyệt, không có lõi ứng dụng.' }}
              </span>
            </div>
          }
          <div class="hint ban-quyen">
            © 2026 Giải pháp số Sao Mai. Sử dụng chữ ký số theo quy định pháp luật Việt Nam về giao dịch điện tử.
          </div>
        </div>
        <button foot class="nut chinh" type="button" (click)="ui.close()">Đóng</button>
      </app-popup>
    } @else if (d && d.kind === 'help') {
      @let topic = help[d.topic];
      <app-popup title="Hướng dẫn sử dụng" icon="book" size="w940 hd" (closed)="ui.close()">
        <div class="hd-than">
          <nav class="hd-nav" aria-label="Mục hướng dẫn">
            @for (t of help; track t.id; let i = $index) {
              <button type="button" class="nav" [class.chon]="t === topic" (click)="go(i)"><svg [icon]="t.icon"></svg>{{ t.name }}</button>
            }
          </nav>
          <div class="hd-noi">
            <div class="hd-cot">
              <div class="hint">Hướng dẫn › {{ topic.name }}</div>
              <h2>{{ topic.name }}</h2>
              <p class="hd-mo">{{ topic.intro }}</p>
              @for (b of topic.blocks; track b.h) {
                <div class="hd-the">
                  <h3>{{ b.h }}</h3>
                  @if (b.type === 'ol') {
                    <ol>
                      @for (it of b.items; track $index) {
                        <li><ng-container *ngTemplateOutlet="dong; context: { $implicit: parts(it) }" /></li>
                      }
                    </ol>
                  } @else {
                    <ul>
                      @for (it of b.items; track $index) {
                        <li><ng-container *ngTemplateOutlet="dong; context: { $implicit: parts(it) }" /></li>
                      }
                    </ul>
                  }
                </div>
              }
            </div>
          </div>
        </div>
        <span foot class="hint trai hd-phim">Phím tắt: <kbd>F1</kbd> Hướng dẫn · <kbd>Ctrl</kbd>+<kbd>K</kbd> Ký số · <kbd>Ctrl</kbd>+<kbd>N</kbd> Tạo văn bản · <kbd>Ctrl</kbd>+<kbd>E</kbd> Kiểm tra</span>
        @if (d.topic > 0) {
          <button foot class="nut" type="button" (click)="go(d.topic - 1)"><svg icon="chevL" class="nho"></svg>{{ help[d.topic - 1].name }}</button>
        }
        @if (d.topic < help.length - 1) {
          <button foot class="nut" type="button" (click)="go(d.topic + 1)">{{ help[d.topic + 1].name }}<svg icon="chevR" class="nho"></svg></button>
        }
        <button foot class="nut chinh" type="button" (click)="ui.close()">Đóng</button>
      </app-popup>
    } @else if (d && d.kind === 'driver') {
      @let st = drv();
      <app-popup title="Cài đặt driver USB Token" icon="usb" size="w560" (closed)="ui.close()">
        <div class="than-p drv">
          @if (drvBusy() && !st) {
            <div class="xem-tt"><svg icon="spin" class="quay"></svg>Đang kiểm tra máy…</div>
          } @else if (drvError(); as e) {
            <div class="bao loi dai"><svg icon="alert"></svg><span><b>Không kiểm tra được driver</b>{{ e }}</span></div>
          }
          @if (st) {
            @if (st.anyInstalled) {
              <div class="bao xanh dai"><svg icon="checkC"></svg><span><b>Máy đã có driver USB Token</b>Cắm token rồi ký như thường. Nếu token của bạn là loại khác, cài thêm driver tương ứng bên dưới.</span></div>
            } @else {
              <div class="bao canh dai"><svg icon="warn"></svg><span><b>Máy chưa có driver USB Token</b>Cần cài driver thì Windows mới đọc được chứng thư số trên token.</span></div>
            }
            @if (st.smartCardDisabled) {
              <div class="bao loi dai"><svg icon="alert"></svg><span><b>Dịch vụ Smart Card của Windows đang bị tắt</b>Token sẽ không hoạt động. Nhờ quản trị máy bật dịch vụ "Smart Card" (SCardSvr) ở chế độ Automatic hoặc Manual.</span></div>
            }
            <div class="drv-ds">
              @for (x of st.drivers; track x.id) {
                <div class="drv-dong" [title]="x.evidence ?? ''">
                  <svg icon="usb"></svg>
                  <span class="drv-ten"><b>{{ x.name }}</b><span class="hint">{{ x.tokens }}</span></span>
                  @if (x.installed) {
                    <span class="ketnoi"><i></i>Đã cài{{ x.version ? ' · ' + x.version : '' }}</span>
                  } @else {
                    <span class="ketnoi tat"><i></i>Chưa cài</span>
                  }
                </div>
              }
            </div>
            <div class="drv-token hint">
              @if (tokens().length) {
                <svg icon="checkC" class="nho"></svg>Đang nhận {{ tokens().length }} chứng thư trên USB Token: {{ tokenNames() }}.
              } @else {
                <svg icon="info" class="nho"></svg>Chưa thấy chứng thư nào trên USB Token. Cắm token rồi bấm <b>Kiểm tra lại</b>.
              }
            </div>
            <div class="hd-the drv-hd">
              <h3>{{ st.anyInstalled ? 'Cài thêm hoặc cài lại driver' : 'Cách tải và cài driver' }}</h3>
              <ol>
                <li>Bấm <b>Mở trang tải phần mềm</b>: trang của Ban Cơ yếu Chính phủ ({{ st.downloadUrl }}) mở trong trình duyệt.</li>
                <li>Tải driver đúng loại token: <b>Bit4id</b> cho token đời mới, <b>VGCA TokenManager</b> (SafeNet eToken) cho token đời cũ. Không rõ loại thì xem nhãn trên token hoặc hỏi nơi cấp chứng thư.</li>
                <li>Chạy file vừa tải và làm theo hướng dẫn cài đặt (cần quyền quản trị máy); khởi động lại máy nếu bộ cài yêu cầu.</li>
                <li>Cắm USB Token rồi bấm <b>Kiểm tra lại</b>.</li>
              </ol>
            </div>
          }
          @if (linkError(); as e) {
            <div class="bao loi dai"><svg icon="alert"></svg><span>{{ e }}</span></div>
          }
        </div>
        <button foot class="nut trai" type="button" [disabled]="drvBusy()" (click)="checkDrivers()"><svg icon="refresh"></svg>Kiểm tra lại</button>
        <button foot class="nut" type="button" (click)="openDriverPage()"><svg icon="download"></svg>Mở trang tải phần mềm</button>
        <button foot class="nut chinh" type="button" (click)="ui.close()">Đóng</button>
      </app-popup>
    }

    <!-- Một dòng, không khoảng trắng giữa các khối: xuống dòng sẽ thành dấu cách trước dấu câu. -->
    <ng-template #dong let-parts>@for (p of parts; track $index) {@switch (p.t) {@case ('b') {<b>{{ p.v }}</b>}@case ('kbd') {<kbd>{{ p.v }}</kbd>}@default {{{ p.v }}}}}</ng-template>
  `,
})
export class ShellDialogsComponent {
  protected readonly ui = inject(UiService);
  protected readonly ipc = inject(IpcService);

  protected readonly help = HELP;
  private readonly partCache = new Map<string, HelpPart[]>();

  protected readonly drv = signal<DriverStatus | null>(null);
  protected readonly drvBusy = signal(false);
  protected readonly drvError = signal<string | null>(null);
  protected readonly linkError = signal<string | null>(null);
  /** Chứng thư đang thấy trên USB Token (đọc kho chứng thư, không hỏi PIN). */
  protected readonly tokens = signal<CertEntry[]>([]);
  protected readonly tokenNames = computed(() => [...new Set(this.tokens().map((c) => c.commonName))].join(', '));

  constructor() {
    effect(() => {
      if (this.ui.dialog()?.kind === 'driver') {
        untracked(() => void this.checkDrivers());
      }
    });
  }

  protected parts(text: string): HelpPart[] {
    let p = this.partCache.get(text);
    if (!p) {
      p = helpParts(text);
      this.partCache.set(text, p);
    }
    return p;
  }

  protected go(topic: number): void {
    this.ui.open({ kind: 'help', topic: Math.max(0, Math.min(topic, this.help.length - 1)) });
    setTimeout(() => document.querySelector('.hd-noi')?.scrollTo({ top: 0 }));
  }

  protected async checkDrivers(): Promise<void> {
    if (!this.ipc.available || this.drvBusy()) {
      return;
    }
    this.drvBusy.set(true);
    this.drvError.set(null);
    this.linkError.set(null);
    try {
      const [status, scan] = await Promise.all([this.ipc.driverStatus(), this.ipc.certList().catch(() => null)]);
      this.drv.set(status);
      this.tokens.set((scan?.certificates ?? []).filter((c) => c.source === 'usbToken'));
    } catch (error) {
      this.drvError.set(errorMessage(error));
    } finally {
      this.drvBusy.set(false);
    }
  }

  protected async openDriverPage(): Promise<void> {
    this.linkError.set(null);
    try {
      await this.ipc.openLink('driver');
    } catch (error) {
      this.linkError.set(errorMessage(error));
    }
  }
}
