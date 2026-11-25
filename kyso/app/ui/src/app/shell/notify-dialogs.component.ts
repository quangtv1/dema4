import { ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked } from '@angular/core';

import { IpcService, NotifyFlags, errorMessage } from '../core/ipc.service';
import { NotifyService } from '../core/notify.service';
import { UiService } from '../core/ui.service';
import { IconComponent } from '../shared/icon.component';
import { PopupComponent } from '../shared/widgets';

type FlagKey = keyof NotifyFlags;

const SWITCHES: { key: FlagKey; label: string; hint: string }[] = [
  { key: 'enabled', label: 'Bật thông báo Telegram', hint: 'Tắt thì ứng dụng không gửi thông báo nào từ máy này.' },
  { key: 'device', label: 'Thông báo đăng ký thiết bị', hint: 'Đăng ký / hủy đăng ký thiết bị, cập nhật ứng dụng.' },
  { key: 'sign', label: 'Thông báo ký số', hint: 'Mỗi lượt ký gửi một tin: tên văn bản, chứng thư, kết quả.' },
  { key: 'document', label: 'Thông báo tạo văn bản', hint: 'Mỗi lượt tạo văn bản gửi một tin: tên file đã tạo.' },
  { key: 'failure', label: 'Thông báo thất bại / hủy', hint: 'Ký hoặc tạo văn bản không thành, bị dừng giữa chừng.' },
];

/**
 * Hai hộp của tính năng Thông báo Telegram:
 * - **Đăng ký thiết bị** (`tg-consent`): hiện ở lần chạy đầu khi máy có cấu hình quản trị. Nói rõ gửi gì, gửi đi
 *   đâu, không gửi gì; chỉ gửi sau khi người dùng bấm Đồng ý. IP nội bộ và địa chỉ MAC là lựa chọn riêng.
 * - **Thông báo Telegram** (`telegram`, menu Hỗ trợ): công tắc từng loại thông báo, kiểm tra kết nối, gửi tin thử,
 *   trạng thái kết nối và đăng ký. Bot Token / Chat ID là cấu hình quản trị: chỉ hiện dạng đã che, không sửa được ở
 *   đây.
 */
@Component({
  selector: 'app-notify-dialogs',
  imports: [PopupComponent, IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @let d = ui.dialog();
    @let st = notify.status();
    @if (d && d.kind === 'tg-consent' && st) {
      <app-popup title="Đăng ký thiết bị với hệ thống quản trị" icon="send" size="w560" (closed)="ui.close()">
        <div class="than-p tg">
          <div class="bao xanh dai">
            <svg icon="info"></svg>
            <span><b>Ứng dụng xin phép gửi thông tin sử dụng về hệ thống quản trị</b>Đơn vị quản trị theo dõi việc dùng Sao Mai Ký số qua một kênh Telegram riêng. Ứng dụng chỉ gửi sau khi bạn đồng ý; đổi ý lúc nào cũng được ở Hỗ trợ › Thông báo Telegram.</span>
          </div>
          <div class="nhom-p">Gửi một lần khi đăng ký</div>
          <div class="luoi-tt">
            <span class="k">Mã thiết bị</span><span class="tg-ma">{{ st.deviceId }}</span>
            <span class="k">Tên máy</span><span>{{ st.device.hostname }}</span>
            <span class="k">Hệ điều hành</span><span>{{ st.device.os }} · {{ st.device.osVersion }}</span>
            <span class="k">Kiến trúc</span><span>{{ st.device.architecture }}</span>
            <span class="k">Phiên bản ứng dụng</span><span>{{ st.device.appVersion }}</span>
          </div>
          <label class="hop tg-chon"><input type="checkbox" [checked]="ip()" (change)="ip.set(checked($event))" />Gửi kèm IP nội bộ ({{ st.device.localIp || 'không xác định' }})</label>
          <label class="hop tg-chon"><input type="checkbox" [checked]="mac()" (change)="mac.set(checked($event))" />Gửi kèm địa chỉ MAC ({{ st.device.macAddress || 'không xác định' }})</label>
          <div class="nhom-p">Gửi sau mỗi lượt ký số hoặc tạo văn bản</div>
          <ul class="tg-ds">
            <li>Tên máy, tài khoản Windows, thời gian, kết quả (thành công / thất bại / hủy) và mã lỗi nếu có.</li>
            <li>Tên file văn bản (không gửi thư mục chứa file) và tên trên chứng thư dùng để ký.</li>
          </ul>
          <div class="nhom-p">Không bao giờ gửi</div>
          <ul class="tg-ds">
            <li>Nội dung văn bản, file PDF, mã PIN, khoá bí mật, dữ liệu chữ ký, serial ổ cứng hay mã phần cứng.</li>
          </ul>
          <div class="hint">Mã thiết bị là số ngẫu nhiên tạo trên máy này, không lấy từ phần cứng. Nơi nhận: {{ st.target }}.</div>
          @if (notify.error(); as e) {
            <div class="bao loi dai"><svg icon="alert"></svg><span>{{ e }}</span></div>
          }
        </div>
        <button foot class="nut trai" type="button" [disabled]="!!notify.busy()" (click)="ui.close()">Để sau</button>
        <button foot class="nut" type="button" [disabled]="!!notify.busy()" (click)="answer(false)">Không đồng ý</button>
        <button foot class="nut chinh" type="button" [disabled]="!!notify.busy()" (click)="answer(true)"><svg icon="check"></svg>Đồng ý và đăng ký</button>
      </app-popup>
    } @else if (d && d.kind === 'telegram') {
      <app-popup title="Thông báo Telegram" icon="send" size="w620" (closed)="ui.close()">
        <div class="than-p tg">
          @if (!st) {
            @if (notify.error(); as e) {
              <div class="bao loi dai"><svg icon="alert"></svg><span><b>Không đọc được trạng thái thông báo</b>{{ e }}</span></div>
            } @else {
              <div class="xem-tt"><svg icon="spin" class="quay"></svg>Đang đọc trạng thái…</div>
            }
          } @else if (!st.configured) {
            <div class="bao canh dai">
              <svg icon="warn"></svg>
              <span>
                <b>Máy này chưa có cấu hình thông báo</b>
                {{ st.configProblem ?? 'Ứng dụng không gửi gì. Quản trị viên đặt file notify.json cạnh file chạy của ứng dụng (hoặc trong %PROGRAMDATA%\\\\SaoMai-KySo) để bật tính năng này.' }}
              </span>
            </div>
            @if (st.configSource) {
              <div class="hint">File cấu hình: {{ st.configSource }}</div>
            }
          } @else {
            <div class="the">
              <div class="row">
                <span class="dan tg-nhan"><b>Kết nối</b></span>
                <span class="tg-gt">
                  <span class="ketnoi" [class.tat]="st.connection.state !== 'ok'" [class.loi]="st.connection.state === 'error'"><i></i>{{ connLabel() }}</span>
                  @if (st.connection.message) {
                    <span class="hint" id="tg-conn-msg">{{ st.connection.message }}</span>
                  }
                </span>
                <button class="nut nho" type="button" [disabled]="!!notify.busy()" (click)="notify.check()"><svg icon="refresh" class="nho"></svg>{{ notify.busy() === 'check' ? 'Đang kiểm tra…' : 'Kiểm tra kết nối' }}</button>
                <button class="nut nho" type="button" [disabled]="!!notify.busy() || st.consent !== 'accepted' || !st.effective.enabled" [title]="st.consent !== 'accepted' ? 'Cần đăng ký thiết bị trước (tin thử có tên máy và mã thiết bị)' : ''" (click)="notify.test()"><svg icon="send" class="nho"></svg>{{ notify.busy() === 'test' ? 'Đang gửi…' : 'Gửi tin thử' }}</button>
              </div>
              <div class="row vach-tren">
                <span class="dan tg-nhan"><b>Thiết bị</b></span>
                <span class="tg-gt">
                  <span class="ketnoi" [class.tat]="st.registration !== 'registered'"><i></i>{{ regLabel() }}</span>
                  <span class="hint tg-ma">{{ st.deviceId }}</span>
                </span>
                @if (st.consent === 'accepted') {
                  @if (confirmUnreg()) {
                    <button class="nut nho nguy" type="button" [disabled]="!!notify.busy()" (click)="unregister()">Xác nhận hủy</button>
                    <button class="nut nho" type="button" (click)="confirmUnreg.set(false)">Thôi</button>
                  } @else {
                    <button class="nut nho" type="button" [disabled]="!!notify.busy()" (click)="confirmUnreg.set(true)">Hủy đăng ký</button>
                  }
                } @else {
                  <button class="nut nho" type="button" [disabled]="!!notify.busy() || !st.admin.enabled" (click)="ui.open({ kind: 'tg-consent' })">Đăng ký thiết bị…</button>
                }
              </div>
              <div class="row vach-tren">
                <span class="dan tg-nhan"><b>Nơi nhận</b></span>
                <span class="tg-gt"><span id="tg-target">{{ st.target }}</span><span class="hint">Do quản trị viên đặt trong notify.json; không sửa được ở đây.</span></span>
              </div>
              <div class="row vach-tren">
                <span class="dan tg-nhan"><b>Hàng đợi</b></span>
                <span class="tg-gt">
                  <span id="tg-queue">{{ st.pending ? st.pending + ' sự kiện đang chờ gửi' : 'Không có sự kiện chờ gửi' }}{{ st.lastSentAt ? ' · gửi gần nhất ' + st.lastSentAt : '' }}</span>
                  @if (st.pending && (st.lastError || st.nextAttemptAt)) {
                    <span class="hint">{{ st.lastError ?? '' }}{{ st.nextAttemptAt ? ' Thử lại lúc ' + st.nextAttemptAt + '.' : '' }}</span>
                  }
                </span>
              </div>
            </div>

            <div class="the tg-sw">
              @for (s of switches; track s.key; let i = $index) {
                <div class="row" [class.vach-tren]="i > 0">
                  <span class="dan tg-cot"><b>{{ s.label }}</b><span class="hint">{{ s.hint }}</span></span>
                  <span class="cfg-bt">{{ swNote(s.key) }}</span>
                  <button type="button" class="sw" [class.on]="swOn(s.key)" [disabled]="swLocked(s.key) || !!notify.busy()" [attr.aria-pressed]="swOn(s.key)" [attr.aria-label]="s.label" (click)="toggle(s.key)"></button>
                </div>
              }
            </div>
            @if (st.locked) {
              <div class="hint">Quản trị viên đã khoá các tuỳ chọn này trên máy.</div>
            }
            @if (st.consent !== 'accepted') {
              <div class="bao canh dai"><svg icon="info"></svg><span>{{ st.consent === 'declined' ? 'Bạn đã không đồng ý gửi thông tin nên ứng dụng không gửi thông báo nào từ máy này.' : 'Chưa đăng ký thiết bị nên ứng dụng chưa gửi thông báo nào.' }} Bấm <b class="tg-il">Đăng ký thiết bị…</b> nếu muốn bật.</span></div>
            }
            <div class="hint">
              Sự kiện được ghi trên máy tại <button class="lk" type="button" (click)="openAudit()">thư mục nhật ký sự kiện</button>, kể cả khi thông báo đang tắt. Cấu hình quản trị: {{ st.configSource }}
            </div>
          }
          @if (st && notify.error(); as e) {
            <div class="bao loi dai" id="tg-error"><svg icon="alert"></svg><span>{{ e }}</span></div>
          }
          @if (note(); as n) {
            <div class="bao canh dai"><svg icon="info"></svg><span>{{ n }}</span></div>
          }
        </div>
        <button foot class="nut chinh" type="button" (click)="ui.close()">Đóng</button>
      </app-popup>
    }
  `,
})
export class NotifyDialogsComponent {
  protected readonly ui = inject(UiService);
  protected readonly notify = inject(NotifyService);
  private readonly ipc = inject(IpcService);

  protected readonly switches = SWITCHES;
  /** Lựa chọn ở hộp đăng ký: mặc định IP nội bộ bật, MAC tắt; quản trị viên đổi được trong `notify.json`. */
  protected readonly ip = signal(true);
  protected readonly mac = signal(false);
  protected readonly confirmUnreg = signal(false);
  protected readonly note = signal<string | null>(null);

  protected readonly connLabel = computed(() => {
    const c = this.notify.status()?.connection.state;
    return c === 'ok' ? 'Hoạt động' : c === 'error' ? 'Có lỗi' : 'Chưa kiểm tra';
  });

  protected readonly regLabel = computed(() => {
    const st = this.notify.status();
    if (!st) {
      return '';
    }
    if (st.registration === 'registered') {
      return `Đã đăng ký${st.registeredAt ? ' lúc ' + st.registeredAt : ''}`;
    }
    if (st.registration === 'pending') {
      return 'Đã đồng ý, đang chờ gửi đăng ký';
    }
    return st.consent === 'declined' ? 'Không đăng ký (đã từ chối)' : 'Chưa đăng ký';
  });

  constructor() {
    // Mở hộp cấu hình thì đọc lại trạng thái (hàng đợi, kết nối thay đổi theo thời gian).
    // Mở hộp đăng ký: đánh dấu sẵn theo gợi ý của quản trị viên (lõi trả về khi chưa trả lời).
    effect(() => {
      if (this.ui.dialog()?.kind === 'tg-consent') {
        const st = untracked(() => this.notify.status());
        if (st && st.consent === 'pending') {
          this.ip.set(st.includeIp);
          this.mac.set(st.includeMac);
        }
      }
    });
    effect(() => {
      if (this.ui.dialog()?.kind === 'telegram') {
        untracked(() => {
          this.confirmUnreg.set(false);
          this.note.set(null);
          void this.notify.refresh();
        });
      }
    });
  }

  protected checked(event: Event): boolean {
    return (event.target as HTMLInputElement).checked;
  }

  protected async answer(accept: boolean): Promise<void> {
    if (await this.notify.consent(accept, accept && this.ip(), accept && this.mac())) {
      this.ui.close();
    }
  }

  protected async unregister(): Promise<void> {
    this.confirmUnreg.set(false);
    await this.notify.unregister();
  }

  /** Quản trị viên tắt hoặc khoá thì người dùng không đổi được công tắc đó. */
  protected swLocked(key: FlagKey): boolean {
    const st = this.notify.status();
    return !st || st.locked || !st.admin[key];
  }

  protected swOn(key: FlagKey): boolean {
    const st = this.notify.status();
    return !!st && st.admin[key] && (st.locked || st.prefs[key]);
  }

  protected swNote(key: FlagKey): string {
    const st = this.notify.status();
    if (st && !st.admin[key]) {
      return 'Quản trị viên tắt';
    }
    return this.swOn(key) ? 'Bật' : 'Tắt';
  }

  protected async toggle(key: FlagKey): Promise<void> {
    const st = this.notify.status();
    if (!st || this.swLocked(key)) {
      return;
    }
    await this.notify.setPrefs({ ...st.prefs, [key]: !st.prefs[key] });
  }

  protected async openAudit(): Promise<void> {
    const dir = this.notify.status()?.auditDir;
    if (!dir) {
      return;
    }
    this.note.set(null);
    try {
      await this.ipc.shellOpen(dir);
    } catch (error) {
      this.note.set(`Chưa mở được thư mục nhật ký (${errorMessage(error)}). Thư mục được tạo khi có sự kiện đầu tiên.`);
    }
  }
}
