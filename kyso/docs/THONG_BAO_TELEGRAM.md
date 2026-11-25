# Thông báo Telegram và đăng ký thiết bị

> Cập nhật: 20/11/2026. Tài liệu triển khai, cấu hình và kiểm thử. Quyết định thiết kế: TD-46 trong
> `TECHNICAL_DECISIONS.md`. Kết quả kiểm: `TEST_REPORT.md` #92–#96.

Sao Mai Ký số có thể báo về một nhóm Telegram của đơn vị quản trị khi:

- một thiết bị đăng ký lần đầu (sau khi người dùng đồng ý);
- một lượt ký số kết thúc: thành công, thất bại, bị dừng;
- một lượt tạo văn bản kết thúc: thành công, thất bại, bị dừng;
- thiết bị hủy đăng ký, ứng dụng được cập nhật.

Tính năng **tắt hẳn** cho tới khi quản trị viên đặt file cấu hình `notify.json` lên máy, và **không gửi gì** cho
tới khi người dùng bấm Đồng ý ở hộp đăng ký thiết bị.

## 1. Kiến trúc

```
 nghiệp vụ (ký số, tạo văn bản)          crate smks-notify (không phụ thuộc Tauri)
 ───────────────────────────────         ─────────────────────────────────────────────────────────
 lô kết thúc ─► SignReport / MergeReport ─► Notifier.emit()  ──► nhật ký sự kiện (JSONL, trên máy)
 (app/src-tauri/src/commands/notify.rs)        │  xét: có cấu hình? đã đồng ý? loại này đang bật?
                                               ▼
                                          hàng đợi lưu đĩa ──► luồng nền ──► kênh gửi
                                          (chống trùng, thử lại có giới hạn)      │
                                                             ┌────────────────────┴───────────────┐
                                                   chế độ gateway (khuyến nghị)          chế độ trực tiếp
                                              máy trạm ─HTTPS─► smks-gateway ─► Telegram   máy trạm ─HTTPS─► Telegram
                                              (Bot Token chỉ ở máy chủ)                    (Bot Token trong notify.json)
```

| Thành phần | Vị trí | Việc |
|---|---|---|
| Event Tracking | `crates/smks-notify/src/event.rs`, `audit.rs` | Kiểu sự kiện thống nhất, nhật ký sự kiện và trạng thái gửi |
| Device Registration | `crates/smks-notify/src/device.rs`, `state.rs` | Mã thiết bị, thông tin thiết bị, trạng thái đồng ý / đăng ký |
| Telegram Notification | `crates/smks-notify/src/{template,queue,transport,telegram,gateway,service}.rs` | Mẫu tin, hàng đợi, thử lại, chống trùng, hai kênh gửi |
| Notification Gateway | `crates/smks-gateway` (`smks-gateway.exe`) | Giữ Bot Token, xác thực thiết bị, soạn và chuyển tin |
| Nối vào app | `app/src-tauri/src/commands/notify.rs` | Lệnh IPC `notify_*`, bộ gom kết quả lô ký / lô tạo văn bản |
| Giao diện | `app/ui/src/app/shell/notify-dialogs.component.ts` | Hộp đăng ký thiết bị, hộp **Hỗ trợ › Thông báo Telegram** |

Nghiệp vụ ký số không đổi. `sign.rs` và `merge.rs` chỉ thêm một bộ gom nghe sự kiện tiến độ sẵn có; sự kiện chỉ
được ghi **sau khi lô kết thúc** (file đã ký và đã kiểm lại; file đã nằm ở thư mục lưu). `emit()` chỉ ghi nhật ký
và xếp hàng rồi về ngay, nên Telegram hỏng, treo hay mất mạng không làm chậm và không làm hỏng việc ký.

## 2. Hai chế độ gửi

| | Gateway (khuyến nghị) | Trực tiếp |
|---|---|---|
| Bot Token nằm ở | Biến môi trường của máy chủ gateway | `notify.json` trên **từng máy trạm** |
| Máy trạm cần | `GATEWAY_URL` + `GATEWAY_ENROLL_KEY` | `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID` |
| Xác thực thiết bị | Mỗi thiết bị một bí mật riêng do gateway cấp (lưu DPAPI) | Không có |
| Thu hồi một thiết bị | `smks-gateway revoke <mã>` | Không làm được riêng lẻ; phải đổi token cho mọi máy |
| Đổi Bot Token | Sửa biến môi trường ở máy chủ, khởi động lại gateway; máy trạm không đổi gì | Thay `notify.json` trên mọi máy |
| Ai soạn nội dung tin | Gateway (máy trạm không gửi được nội dung tuỳ ý) | Máy trạm |
| Cần máy chủ | Có | Không |

**Giới hạn của chế độ trực tiếp, cần biết trước khi chọn:** Bot Token nằm trong file trên máy trạm, ai đọc được
file đó (kể cả người dùng của máy) là lấy được token và gửi được tin tuỳ ý vào nhóm. Chỉ nên dùng cho vài máy tin
cậy, hoặc để thử. Khi triển khai rộng, dùng gateway.

Ở cả hai chế độ, Bot Token **không** nằm trong mã nguồn, **không** nằm trong `SaoMai-KySo.exe`, không vào git và
không ghi ra log (đã kiểm, `TEST_REPORT.md` #94).

## 3. Cấu hình máy trạm: `notify.json`

Ứng dụng tìm file theo thứ tự, lấy file đầu tiên có:

1. đường dẫn trong biến môi trường `SMKS_NOTIFY_CONFIG`;
2. `notify.json` cạnh `SaoMai-KySo.exe`;
3. `%PROGRAMDATA%\SaoMai-KySo\notify.json`.

Đặt file ở nơi người dùng thường **không ghi được** (thư mục cài trong `Program Files`, hoặc `%PROGRAMDATA%` do
quản trị viên tạo): người dùng không tự đổi được nơi nhận. Giao diện chỉ hiện nơi nhận ở dạng đã che
(`nhóm ••••7890 · Bot Token ••••AbCd`, tức 4 ký tự cuối) và không có chỗ sửa.

Không có file nào ở trên thì dùng **cấu hình nhúng trong exe** (TD-59): build với
`SMKS_NOTIFY_EMBEDDED_FILE=<đường dẫn notify.json>` để phát một file exe duy nhất. File nằm ngoài repo; nội dung
được làm rối (không phải mã hoá: ai có exe vẫn lấy lại được Bot Token). Giao diện ghi nguồn "nhúng trong
SaoMai-KySo.exe".

Khi ứng dụng chạy với `SMKS_CONFIG_DIR` (bài kiểm tự động) thì chỉ nhận file ở `SMKS_NOTIFY_CONFIG`, để cấu hình
thật trên máy không lẫn vào bài kiểm.

| Khoá | Mặc định | Ý nghĩa |
|---|---|---|
| `GATEWAY_URL` | | Địa chỉ gateway, ví dụ `https://thongbao.donvi.vn`. Có khoá này thì dùng chế độ gateway |
| `GATEWAY_ENROLL_KEY` | | Khoá đăng ký thiết bị với gateway (không phải Bot Token) |
| `TELEGRAM_BOT_TOKEN` | | Bot Token, chỉ dùng ở chế độ trực tiếp |
| `TELEGRAM_CHAT_ID` | | Nhóm nhận tin, chỉ dùng ở chế độ trực tiếp; ghi số hoặc chuỗi đều được |
| `NOTIFICATION_ENABLED` | `true` | Công tắc chung |
| `DEVICE_REGISTRATION_ENABLED` | `true` | Tin đăng ký / hủy đăng ký thiết bị, cập nhật ứng dụng |
| `SIGN_NOTIFICATION_ENABLED` | `true` | Tin ký số |
| `DOCUMENT_NOTIFICATION_ENABLED` | `true` | Tin tạo văn bản |
| `FAILURE_NOTIFICATION_ENABLED` | `true` | Tin thất bại / bị hủy |
| `LOCK_SETTINGS` | `false` | `true`: người dùng không đổi được các công tắc trên giao diện |
| `TIMEOUT_SECONDS` | `10` | Thời gian chờ mỗi lần gửi (3–60) |
| `SEND_IP_DEFAULT` | `true` | Ô "Gửi kèm IP nội bộ" ở hộp đăng ký thiết bị đánh dấu sẵn |
| `SEND_MAC_DEFAULT` | `false` | Ô "Gửi kèm địa chỉ MAC" ở hộp đăng ký thiết bị đánh dấu sẵn (người dùng vẫn bỏ được) |
| `AUTO_REGISTER` | `false` | `true`: lần chạy đầu tự đăng ký thiết bị, không hiện hộp hỏi đồng ý (TD-60); IP / MAC theo hai khoá trên |
| `TELEGRAM_API_BASE` | | Địa chỉ Bot API khác mặc định (máy chủ Bot API tự dựng; bài kiểm dùng máy chủ giả) |

Một loại thông báo được gửi khi **quản trị viên và người dùng cùng bật**. Quản trị viên tắt thì người dùng không
bật lại được.

File mẫu (không có bí mật thật): `installer/notify.gateway.example.json`, `installer/notify.direct.example.json`.
Sửa `notify.json` xong phải mở lại ứng dụng.

## 4. Dựng Notification Gateway

`smks-gateway.exe` là một file chạy độc lập (build: `cargo build -p smks-gateway --release`).

1. Tạo file môi trường từ `installer/gateway.env.example`, điền `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`,
   `GATEWAY_ENROLL_KEY` (chuỗi ngẫu nhiên từ 16 ký tự, ví dụ sinh bằng
   `powershell -c "[guid]::NewGuid().ToString('N') + [guid]::NewGuid().ToString('N')"`). Để file ngoài git, chỉ
   tài khoản chạy gateway đọc được. Cũng có thể đặt thẳng thành biến môi trường của dịch vụ.
2. Kiểm tra bot: `smks-gateway --env-file gateway.env check` → `Bot @ten_bot gửi được vào nhóm «…».`
3. Chạy: `smks-gateway --env-file gateway.env serve`. Mặc định nghe `127.0.0.1:8787`; đổi bằng `GATEWAY_BIND`.
4. Gateway nói HTTP thường. Khi máy trạm ở ngoài mạng nội bộ tin cậy, đặt sau reverse proxy có TLS (Caddy, nginx,
   IIS) và ghi địa chỉ `https://…` vào `GATEWAY_URL` của máy trạm.
5. Trên máy trạm: `notify.json` chỉ cần `GATEWAY_URL` và `GATEWAY_ENROLL_KEY`.

| Biến môi trường của gateway | Mặc định | Ý nghĩa |
|---|---|---|
| `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` | bắt buộc | Bot và nhóm nhận tin |
| `GATEWAY_ENROLL_KEY` | bắt buộc | Khoá đăng ký thiết bị, từ 16 ký tự |
| `GATEWAY_BIND` | `127.0.0.1:8787` | Địa chỉ nghe |
| `GATEWAY_DATA_DIR` | `gateway-data` | Nơi lưu `devices.json`, `seen-events.json` |
| `NOTIFICATION_ENABLED`, `DEVICE_REGISTRATION_ENABLED`, `SIGN_NOTIFICATION_ENABLED`, `DOCUMENT_NOTIFICATION_ENABLED`, `FAILURE_NOTIFICATION_ENABLED` | bật | Tắt bằng `false` / `0` / `off` / `no`. Tắt ở máy chủ thì không máy trạm nào gửi được loại đó |

Giao thức (máy trạm gọi):

| Lời gọi | Xác thực | Việc |
|---|---|---|
| `GET /v1/health` | không bắt buộc | Gateway và Telegram có hoạt động không; kèm thiết bị thì báo `registered` / `revoked` / `unknown` |
| `POST /v1/devices/register` | `X-Enroll-Key` | Đăng ký thiết bị, cấp bí mật riêng (gateway chỉ lưu SHA-256), báo NEW DEVICE |
| `POST /v1/events` | `Authorization: Bearer <mã thiết bị>:<bí mật>` | Nhận một sự kiện, soạn tin, gửi Telegram |

Gateway bỏ sự kiện trùng `event_id`, giới hạn 30 sự kiện/phút mỗi thiết bị, thân yêu cầu tối đa 64 KB, và làm sạch
lại mọi chuỗi nhận từ máy trạm trước khi soạn tin.

Quản lý thiết bị (đọc `GATEWAY_DATA_DIR`; gateway đang chạy tự nhận thay đổi, không cần khởi động lại):

```
smks-gateway --env-file gateway.env devices            liệt kê thiết bị
smks-gateway --env-file gateway.env revoke  <mã>       thu hồi: không gửi được, không đăng ký lại được
smks-gateway --env-file gateway.env restore <mã>       bỏ thu hồi
smks-gateway --env-file gateway.env remove  <mã>       xoá khỏi danh sách (thiết bị sẽ tự đăng ký lại)
```

## 5. Tạo và kiểm tra Telegram Bot

1. Trên Telegram, nhắn `@BotFather` lệnh `/newbot`, đặt tên; BotFather trả **Bot Token** dạng `123456789:AA…`.
2. Tạo nhóm nhận tin, thêm bot vào nhóm.
3. Lấy **Chat ID** của nhóm: gửi một tin trong nhóm rồi mở
   `https://api.telegram.org/bot<TOKEN>/getUpdates`, đọc `chat.id` (nhóm thường là số âm).
4. Kiểm tra:
   - gateway: `smks-gateway --env-file gateway.env check`;
   - trong ứng dụng: **Hỗ trợ › Thông báo Telegram › Kiểm tra kết nối** (báo tên bot và tên nhóm), rồi
     **Gửi tin thử**.

**Thu hồi và đổi token:** nhắn `@BotFather` lệnh `/revoke` (token cũ hết hiệu lực ngay), lấy token mới, rồi sửa biến
môi trường của gateway và khởi động lại gateway (chế độ gateway), hoặc thay `notify.json` trên các máy (chế độ trực
tiếp). Trong lúc token sai, ứng dụng vẫn ký bình thường, giữ thông báo trong hàng đợi và tự gửi khi token đúng trở
lại.

Lưu ý nhóm thường khi được nâng thành supergroup sẽ đổi Chat ID (thành dạng `-100…`); khi đó ứng dụng báo
"Telegram không tìm thấy nhóm nhận tin" và cần sửa `TELEGRAM_CHAT_ID`.

## 6. Đăng ký thiết bị và quyền riêng tư

- **Mã thiết bị** là số ngẫu nhiên (`SMKS-<UUID>`) sinh ở lần chạy đầu, lưu ở
  `%APPDATA%\SaoMai-KySo\notify-state.json`. Không suy ra từ phần cứng. Thư mục `%APPDATA%` là của từng tài khoản
  Windows, nên mỗi tài khoản Windows trên một máy là một thiết bị riêng và được hỏi đồng ý riêng.
- Lần mở đầu tiên trên máy đã có `notify.json`, ứng dụng hiện hộp **Đăng ký thiết bị với hệ thống quản trị**: liệt kê
  thông tin sẽ gửi, những gì không bao giờ gửi, nơi nhận (đã che).
  - **Đồng ý và đăng ký**: gửi một lần `device_id`, `hostname`, `os`, `os_version`, `architecture`, `app_version`,
    `registered_at`. `local_ip` chỉ gửi khi ô "Gửi kèm IP nội bộ" được đánh dấu (bật sẵn), `mac_address` chỉ gửi khi
    ô "Gửi kèm địa chỉ MAC" được đánh dấu (tắt sẵn).
  - **Không đồng ý**: lưu lại, không gửi gì, không hỏi lại.
  - **Để sau** / đóng hộp: chưa gửi gì; lần mở sau hỏi lại.
- Trạng thái đăng ký lưu trên máy nên các lần mở sau không đăng ký lại.
- **Hủy đăng ký** ở hộp Thông báo Telegram: gửi một tin DEVICE UNREGISTERED rồi ngừng gửi; đăng ký lại được.
- Không thu thập: serial ổ cứng, mã phần cứng, mật khẩu, thông tin tài khoản. Không có theo dõi nền: thông tin
  thiết bị chỉ được đọc khi mở hộp đăng ký / hộp cấu hình hoặc khi gửi đăng ký.
- Sự kiện ký / tạo văn bản mang: tên máy, tài khoản Windows, thời gian, **tên file** (không có thư mục), tên trên
  chứng thư ký, kết quả, mã lỗi và thông báo lỗi đã bỏ đường dẫn. Không bao giờ có nội dung văn bản, file PDF, mã
  PIN, khoá bí mật hay dữ liệu chữ ký.

## 7. Sự kiện

| Loại | Khi nào | Công tắc |
|---|---|---|
| `DEVICE_REGISTERED` | Người dùng đồng ý đăng ký | đăng ký thiết bị |
| `DEVICE_UNREGISTERED` | Người dùng hủy đăng ký | đăng ký thiết bị |
| `APP_UPDATED` | Mở ứng dụng với phiên bản khác lần trước | đăng ký thiết bị |
| `SIGN_SUCCESS` | Lượt ký kết thúc, có văn bản đã ký và kiểm lại xong | ký số |
| `SIGN_FAILED` | Có văn bản lỗi; không mở được token (PIN sai, token bị khoá, không thấy chứng thư); rút token giữa lượt | ký số + thất bại |
| `SIGN_CANCELLED` | Người dùng bấm Tạm dừng khi còn văn bản chưa ký; hủy ở hộp PIN của middleware | ký số + thất bại |
| `DOCUMENT_CREATED` | Lượt tạo văn bản kết thúc, có file đã tạo | tạo văn bản |
| `DOCUMENT_FAILED` | Có dòng lỗi; mẫu / dữ liệu / cấu hình ghép không hợp lệ | tạo văn bản + thất bại |
| `DOCUMENT_CANCELLED` | Người dùng bấm Dừng khi còn dòng chưa tạo | tạo văn bản + thất bại |
| `TEST` | Nút Gửi tin thử | công tắc chung |
| `DOCUMENT_EXPORTED`, `DOCUMENT_DELETED` | Đã có kiểu và mẫu tin, **chưa nối vào nghiệp vụ** | tạo văn bản |

Một lượt (lô) là **một giao dịch**: một lượt ký 200 văn bản ra một tin SIGN SUCCESS (liệt kê 10 tên đầu, còn lại
đếm), kèm một tin SIGN FAILED nếu có văn bản lỗi và một tin SIGN CANCELLED nếu bị dừng; ba tin cùng mã giao dịch
(`KS-<ngày>-<giờ>-<6 hex>`). Lý do: Telegram giới hạn khoảng 20 tin/phút cho một nhóm.

Mỗi sự kiện có `event_id`, `event_type`, `device_id`, `user_id`, `timestamp`, `transaction_id`, `status`, `metadata`.

**Nhật ký sự kiện** nằm ở `%APPDATA%\SaoMai-KySo\nhat-ky-su-kien\events-<yyyy-mm>.jsonl`. Mọi sự kiện đều được ghi
ở đây ngay khi nghiệp vụ kết thúc, kể cả khi thông báo tắt hoặc người dùng không đồng ý gửi (khi đó ghi
`"disposition":"skipped"` kèm lý do, và không rời khỏi máy). Mỗi lần thử gửi thêm một dòng `"kind":"delivery"` với
kết quả `sent` / `retry` / `gave-up` / `rejected` / `dropped`.

## 8. Mất mạng, lỗi, gửi lại

- Sự kiện vào hàng đợi `%APPDATA%\SaoMai-KySo\notify-queue.json` trước, gửi sau, đúng thứ tự phát sinh.
- Lỗi mạng, hết thời gian chờ, Telegram 5xx / 429, token sai, thiết bị bị thu hồi: giữ lại và thử lại sau 10 giây,
  30 giây, 2 phút, 5 phút, 15 phút, 30 phút, rồi mỗi giờ (tôn trọng `retry_after` của Telegram).
- **Có giới hạn:** tối đa 20 lần thử cho một sự kiện, quá 7 ngày thì bỏ, hàng đợi tối đa 500 sự kiện (đầy thì bỏ
  sự kiện cũ nhất). Mọi lần bỏ đều ghi vào nhật ký sự kiện.
- Mở ứng dụng, hoặc bấm **Kiểm tra kết nối** thành công, thì thử gửi ngay những gì đang chờ.
- **Chống trùng:** mỗi sự kiện có `event_id`; nghiệp vụ báo trùng cùng (loại, mã giao dịch) thì chỉ nhận một lần;
  ứng dụng nhớ 1000 khoá gần nhất qua các lần khởi động; gateway bỏ `event_id` đã nhận.
- **Trường hợp còn có thể trùng:** Telegram đã nhận tin nhưng phản hồi bị mất trên đường về (hết thời gian chờ đúng
  lúc đó). Bot API không có khoá chống trùng nên lần gửi lại sẽ ra tin thứ hai. Cửa sổ này rất hẹp; nhận diện bằng
  mã giao dịch giống nhau.

## 9. Build và đóng gói

```powershell
cd app\ui; npm ci; cd ..\..
cd app; ui\node_modules\.bin\tauri build --no-bundle; cd ..       # target\release\SaoMai-KySo.exe
cargo build -p smks-gateway --release                              # target\release\smks-gateway.exe
```

`SaoMai-KySo.exe` chạy độc lập, không cần Rust hay Node.js trên máy người dùng. Máy cần:

- Windows 10/11 x64;
- **WebView2 Runtime** (có sẵn trên Windows 11 và trên Windows 10 đã cập nhật Edge; thiếu thì tải "Evergreen
  Bootstrapper" ở trang Microsoft Edge WebView2);
- Microsoft Word nếu muốn xuất PDF ở Tạo văn bản; driver USB Token để ký.

Gói chạy ngay (không cần cài): chép `SaoMai-KySo.exe` vào một thư mục, đặt `notify.json` cạnh nó (nếu dùng thông
báo), chạy file exe. Bộ cài NSIS của kế hoạch mục 16 **chưa làm**.

## 10. Kiểm thử

Tự động, trên app thật (bản build `--features dev-keys`, khoá thử), Telegram thay bằng máy chủ Bot API giả để đếm
đúng từng tin và giả lập lỗi; phần gateway chạy `smks-gateway.exe` thật:

```powershell
node app/tests/tg-notify.mjs target/release/SaoMai-KySo.exe target/release/smks-gateway.exe . $env:TEMP\smks-tg-notify spikes/s2-cms/out/pki
```

Thêm `--real` và đặt `TG_REAL_TOKEN`, `TG_REAL_CHAT` trong môi trường để gửi thêm 5 tin thật (token không được ghi
vào repo; file cấu hình tạm bị xoá khi xong).

| # | Tình huống của đề bài | Phần của bài kiểm |
|---|---|---|
| 1 | Chạy lần đầu trên máy mới | C |
| 2 | Đồng ý đăng ký thiết bị | C |
| 3 | Từ chối đăng ký thiết bị | B |
| 4 | Khởi động lại, không đăng ký trùng | C |
| 5 | Ký số thành công | D |
| 6 | Ký số thất bại | D |
| 7 | Người dùng hủy ký số | D |
| 8 | Tạo văn bản thành công | E |
| 9 | Tạo văn bản thất bại | E |
| 10 | Bot Token không hợp lệ | G |
| 11 | Telegram mất kết nối | G |
| 12 | Khôi phục kết nối, gửi lại hàng đợi | G |
| 13 | Không gửi trùng | G |
| 14 | Ký số bình thường khi Telegram lỗi | G (token sai, treo, mất kết nối) |
| 15 | Không có Bot Token trong EXE và log | I |

Ngoài ra: A (không cấu hình thì im lặng), F (giao diện cấu hình), H (gateway: đăng ký, thu hồi, khôi phục).
Test đơn vị: `cargo test -p smks-notify -p smks-gateway`.

## 11. Giới hạn và việc chưa làm

- Chưa kiểm trên một máy Windows sạch (chưa từng cài Rust / Node.js); mới kiểm thư viện mà file exe cần.
- Chưa kiểm với USB Token thật sau khi thêm tính năng này (bài kiểm dùng khoá thử); đường ký không đổi.
- Hủy ở hộp **Xác nhận ký** của ứng dụng (chưa bấm Ký số) không tạo sự kiện: chưa có nghiệp vụ nào bắt đầu.
- Hủy ở hộp PIN của middleware được nhận diện qua mã lỗi `0x8010006E` / `0x800704C7` trong thông báo của
  middleware; middleware trả mã khác thì sự kiện được ghi là SIGN FAILED. Chưa kiểm với token thật.
- `DOCUMENT_EXPORTED`, `DOCUMENT_DELETED` chưa nối vào nghiệp vụ.
- Gateway chưa có TLS riêng (dùng reverse proxy), chưa có giao diện quản trị (dùng dòng lệnh), chưa build / kiểm
  trên Linux.
- Bộ cài NSIS chưa làm; `notify.json` hiện phải chép tay.
