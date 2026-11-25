# Phân tích frontend cũ `ktl.fe` để port sang Rust + Tauri 2

- Ngày phân tích: 2026-11-18.
- Đối tượng: `Kytl-main/ktl.fe/` (bản .NET 9 + Angular 21, sản phẩm v1.4.0).
- Đã đọc hết từng file: `package.json`, `angular.json`, `tsconfig.json`, `tsconfig.app.json`, `.postcssrc.json`, `.editorconfig`, `.gitignore` và toàn bộ 87 file trong `src/`. Không đọc `node_modules`, `package-lock.json`, `dist`, `.angular` (chỉ tra phiên bản gói đang cài trong `node_modules/*/package.json`).
- Mọi phát hiện đều từ đọc code tĩnh. Chưa chạy ứng dụng để tái hiện lỗi nào.

Quy ước trong tài liệu:

- Đường dẫn file ghi tương đối so với `Kytl-main/ktl.fe/`. Ký hiệu `file:12-34` là số dòng trong file gốc.
- **Đối chiếu backend**: một số chỗ cần biết phía .NET làm gì mới kết luận được (trục Y, định dạng đường dẫn, nội dung sự kiện). Các chỗ đó ghi rõ file `.cs` đã mở. Backend không được đọc toàn bộ, chỉ đọc đúng đoạn cần đối chiếu.
- **Chưa chắc**: nhận định chưa kiểm chứng được bằng code, cần thử khi làm spike.

---

## 0. Tóm tắt kết luận

1. **Frontend cũ chỉ có 2 màn hình**: Ký số (`pages/ky-so`) và Tạo văn bản (`pages/tao-van-ban`), cộng một dialog "Cấu hình chữ ký". Không có Trang chủ, Kiểm tra chữ ký, màn Cấu hình riêng, Hỗ trợ. Các màn đó ở bản mới không có code cũ để dùng lại.
2. **Hệ toạ độ vị trí**: `xRatio`, `yRatio`, `widthRatio`, `heightRatio` là tỉ lệ 0–1 so với **trang đang hiển thị** (khung pdf.js `getViewport({ scale: 1 })`, đã áp `/Rotate`). Gốc ở **góc trên bên trái**, X sang phải, **Y hướng xuống**. `(xRatio, yRatio)` là góc trên trái của khối. Frontend không có dòng code nào xử lý trang xoay; việc đổi sang hệ toạ độ PDF nằm ở backend.
3. **Cấu hình cũ**: đúng một khoá localStorage `saomai_kyso_sign_config`, giá trị là nguyên object `ISignConfig` (16 trường). Không có số phiên bản, không kiểm tra hợp lệ. Bản desktop cũ nạp giao diện từ `http://127.0.0.1:<cổng ngẫu nhiên>` nên nhiều khả năng cấu hình không còn sau khi mở lại ứng dụng (mục 4.4).
4. **Dùng lại được gần nguyên**: `models/*`, `shared/models/pdf-pages.models.ts`, `shared/constants/document-merge.constants.ts`, `sign.constants.ts`, `path-picker.constants.ts`; 3 hàm trong `utils.ts` (`clamp`, `compareDocSeq`, `normalizeFolderPath`); toàn bộ phần tính toán của `stamp-designer.ts`; `pdf-pages.ts`; `sign-handoff.service.ts`, `sign-cert-state.service.ts`, `app-route-reuse.strategy.ts`.
5. **Phải thay hẳn**: `sse-stream.ts` và bản sao trong `sign.service.ts` (thay bằng `Channel`), `desktop.service.ts` (cầu nối Photino `window.external`), `file-upload.service.ts` (bỏ), lớp HTTP của `sign.service.ts` và `document-merge.service.ts`, `base-component.ts` (gắn chặt PrimeNG), toàn bộ template HTML (PrimeNG + Tailwind).
6. **Bẫy lớn nhất khi sang Tauri**: thả ảnh mẫu vào trang trong `stamp-designer` dùng kéo thả HTML5 (`draggable` + `dragstart`/`drop`). Trên Windows, Tauri phải tắt `dragDropEnabled` thì HTML5 drag and drop mới chạy, mà tắt thì mất `onDragDropEvent` (kéo file từ Explorer có đường dẫn thật). Cần viết lại thao tác thả ảnh mẫu bằng pointer events (mục 7.6).
7. **Kích thước khối chữ ký số do trình duyệt đo**: `widthRatio`/`heightRatio` của khối chữ ký tính bằng `canvas.measureText` theo tên chứng thư và font đang chọn, rồi gửi cho backend dùng. Mô hình "mẫu theo khổ giấy" của bản mới không khớp với cách này (mục 7.9).
8. **Sự kiện tiến độ**: frontend cũ dựa vào `percent` do backend tính, khớp dòng theo **tên file** (ký số) hoặc `rowNumber` (tạo văn bản), và hiển thị chi tiết chữ ký từ sự kiện `file`. `JobEvent` trong kế hoạch mục 10.2 chưa có các trường này (mục 5.6).
9. **Lỗi đáng tránh mang sang**: một biến `sub` dùng chung cho "liệt kê file" và "ký" làm đổi nguồn giữa chừng sẽ huỷ lô ký và treo nút; khớp kết quả theo tên file; trạng thái "đã xác thực PIN" không bao giờ bị xoá; không có nút huỷ ký; `normalizeFolderPath` làm hỏng đường dẫn UNC (mục 11).
10. **Ứng dụng chạy zoneless** (không có `zone.js`), state bằng `signal`. Callback từ `Channel` của Tauri chỉ cần ghi vào signal là giao diện cập nhật.

---

## 1. Phụ thuộc và cấu hình build

### 1.1 `package.json`

`name: "ktl.fe"`, `version: "1.0.0"` (không trùng số phiên bản sản phẩm 1.4.0), `private: true`, `packageManager: "npm@10.9.3"`. Scripts: `ng`, `start` (`ng serve`), `build` (`ng build`), `watch` (`ng build --watch --configuration development`). Prettier: `printWidth: 100`, `singleQuote: true`, HTML dùng parser `angular`.

| Gói | Khai báo | Bản đang cài | Dùng ở đâu | Bản mới |
|---|---|---|---|---|
| `@angular/core`, `common`, `compiler`, `forms`, `platform-browser`, `router` | `^21.2.17` | `@angular/core` 21.2.25 | Toàn bộ | Giữ |
| `@angular/animations` | `^21.2.17` | | Chỉ để `provideAnimationsAsync()` cho PrimeNG (`src/app.config.ts:4,18`) | Bỏ cùng PrimeNG |
| `@primeuix/themes` | `^2.0.3` | 2.0.3 | Preset `Aura` (`src/app.config.ts:8,19-26`) | Bỏ |
| `primeng` | `^21.1.9` | 21.1.10 | 11 file `.ts`, 16 file `.html` (mục 10.3) | Bỏ |
| `primeicons` | `^7.0.0` | 7.0.0 | `src/styles.css:2` và class `pi pi-*` trong 21 file | Bỏ hoặc thay bộ icon của prototype |
| `pdfjs-dist` | `^6.4.299` | 6.4.299 | `app/shared/components/pdf-pages/pdf-pages.ts:18-23` | Giữ |
| `moment` | `^2.31.0` | 2.31.0 | Chỉ `app/shared/utils.ts:1,127-139`; nơi gọi thật duy nhất là `sign-config-dialog.ts:90` | Bỏ được |
| `rxjs` | `~7.8.0` | 7.8.2 | `Observable`, `Subscription`, `forkJoin`, `filter` | Giữ (ít dùng hơn) |
| `tslib` | `^2.3.0` | 2.8.1 | `importHelpers` | Giữ |
| `@angular/build` (dev) | `^21.2.16` | 21.2.26 | Builder | Giữ |
| `@angular/cli` (dev) | `^21.2.16` | 21.2.26 | | Giữ |
| `@angular/compiler-cli` (dev) | `^21.2.17` | | | Giữ |
| `tailwindcss` (dev) | `^4.1.12` | 4.3.3 | `src/styles.css:1`, class tiện ích trong mọi template | Tuỳ quyết định (mục 10.4) |
| `@tailwindcss/postcss` (dev) | `^4.1.12` | 4.3.3 | `.postcssrc.json` | Đi cùng Tailwind |
| `postcss` (dev) | `^8.5.3` | 8.5.30 | | Đi cùng Tailwind |
| `typescript` (dev) | `~5.9.2` | 5.9.3 | | Giữ |

Những thứ **không có**:

- Không có `zone.js` (không trong `package.json`, không trong `node_modules`, `angular.json` không khai `polyfills`). Ứng dụng chạy zoneless, là mặc định của Angular 21.
- Không có framework test, không có file `*.spec.ts`, `angular.json` không có target `test`. Không có ESLint.
- Không có file cấu hình Tailwind (`tailwind.config.*`), không có `@theme` trong CSS, không có plugin `tailwindcss-primeui`. Hệ quả: class `text-primary` ở `app/pages/ky-so/ky-so.html:13` không sinh ra CSS nào.

### 1.2 `angular.json`

Một project `ktl.fe`, `sourceRoot: "src"`, `prefix: "app"`.

| Mục | Giá trị |
|---|---|
| Builder build | `@angular/build:application` |
| `browser` | `src/main.ts` |
| `tsConfig` | `tsconfig.app.json` |
| `styles` | `src/styles.css` |
| `assets` | (1) `{ "glob": "**/*", "input": "public" }` (gồm `favicon.ico`, `logo.png`); (2) `{ "glob": "pdf.worker.min.mjs", "input": "node_modules/pdfjs-dist/build" }` |
| `outputPath` | Không khai, dùng mặc định |
| `defaultConfiguration` (build) | `production` |
| Builder serve | `@angular/build:dev-server`, mặc định `development` |

Ba configuration của build:

| Configuration | `fileReplacements` | Budgets | Khác |
|---|---|---|---|
| `production` | Không | `initial`: cảnh báo 2MB, lỗi 3MB. `anyComponentStyle`: cảnh báo 4kB, lỗi 8kB | `outputHashing: "all"` |
| `desktop` | `environment.ts` → `environment.desktop.ts` | Giống `production` | `outputHashing: "all"` |
| `development` | `environment.ts` → `environment.development.ts` | Không | `optimization: false`, `extractLicenses: false`, `sourceMap: true` |

Ghi chú:

- `environment.ts` và `environment.desktop.ts` giống hệt nhau (`production: true`, `baseUrl: ''`). `environment.development.ts` có `baseUrl: 'http://localhost:5247'`.
- `serve` chỉ có `production` và `development`, không có `desktop`.
- `stamp-designer.scss` (250 dòng) và `sign-result-dialog.scss` (203 dòng) là hai file style lớn nhất, cần để ý ngưỡng `anyComponentStyle` 8kB nếu giữ nguyên budget.

**Cách nạp worker pdf.js**: quy tắc assets thứ hai chép `pdf.worker.min.mjs` (1.264.342 byte) vào gốc thư mục build, không băm tên. Code trỏ tới nó bằng `document.baseURI` (`pdf-pages.ts:23`):

```ts
pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdf.worker.min.mjs', document.baseURI).toString();
```

Dòng này chạy ngay khi module `pdf-pages.ts` được import. Các thư mục `wasm/`, `cmaps/`, `standard_fonts/`, `iccs/` của `pdfjs-dist` **không** được chép (mục 8.6).

### 1.3 `tsconfig.json`, `.postcssrc.json`

- `strict: true`, `noImplicitOverride`, `noPropertyAccessFromIndexSignature`, `noImplicitReturns`, `noFallthroughCasesInSwitch`, `isolatedModules`, `experimentalDecorators`, `importHelpers`, `target: "ES2022"`, `module: "preserve"`.
- `angularCompilerOptions`: `strictInjectionParameters`, `strictInputAccessModifiers`, `strictTemplates` đều `true`.
- `.postcssrc.json`: `{ "plugins": { "@tailwindcss/postcss": {} } }`.

---

## 2. Bản đồ file

Cột "Dòng" là số dòng của file.

### 2.1 Gốc `src/`

| File | Dòng | Vai trò |
|---|---|---|
| `src/index.html` | 13 | Trang gốc: `lang="vi"`, `<title>Sao Mai - Ký số</title>`, `<base href="/">`, favicon, `<app-root>` |
| `src/main.ts` | 5 | `bootstrapApplication(App, appConfig)` |
| `src/app.ts` | 9 | Component gốc `app-root`, chỉ có `<router-outlet />` |
| `src/app.config.ts` | 32 | Providers: router, `provideHttpClient(withFetch())`, `provideAnimationsAsync()`, `providePrimeNG` (preset Aura, `darkModeSelector: '.app-dark'`), `MessageService`, `ConfirmationService`, `DialogService`, `AppRouteReuseStrategy` |
| `src/app.routes.ts` | 20 | Route gốc dùng `Layout`; `''` chuyển sang `ky-so`; nạp lười `ky-so` và `tao-van-ban` |
| `src/styles.css` | 206 | Import Tailwind và PrimeIcons; font Segoe UI 14px; `user-select: none` toàn trang; scrollbar; biến `--layout-*`; CSS khung topbar, thanh trạng thái, vùng chính; `.table-frame` |
| `src/environments/environment.ts` | 6 | `production: true`, `baseUrl: ''` |
| `src/environments/environment.desktop.ts` | 6 | Giống hệt `environment.ts` |
| `src/environments/environment.development.ts` | 6 | `production: false`, `baseUrl: 'http://localhost:5247'` |

### 2.2 `src/app/layout/`

| File | Dòng | Vai trò |
|---|---|---|
| `layout.ts` | 18 | Component khung `app-layout`; inject `LayoutService` để kích hoạt nó |
| `layout.html` | 14 | `<p-toast />`, `<p-confirmdialog />`, topbar, `<router-outlet />`, footer |
| `layout.css` | 4 | `:host { display: block; height: 100%; }` |
| `topbar.ts` | 22 | Thanh trên: logo `logo.png`, tên "Sao Mai - Ký số", `<app-menu />` |
| `menu.ts` | 35 | Hai mục điều hướng: "Ký số" (`/ky-so`, `pi-verified`), "Tạo văn bản" (`/tao-van-ban`, `pi-clone`) |
| `footer.ts` | 15 | Thanh trạng thái tĩnh: "Sẵn sàng", "Sao Mai - Ký số · Sao Mai Digital Solutions" |
| `service/layout.service.ts` | 51 | Zoom toàn trang theo bề rộng cửa sổ (thiết kế 1440px, zoom tối thiểu 0,6) và chặn hành vi trình duyệt khi không ở dev mode (menu chuột phải, Ctrl+lăn chuột, F5, Ctrl+R/P/U/F/G/J/+/-/=/0, Alt+←/→) |

### 2.3 `src/app/models/`

| File | Dòng | Vai trò |
|---|---|---|
| `sign.models.ts` | 175 | Kiểu dữ liệu ký số: chứng thư, kết quả kiểm token, tiến độ, danh sách file, vị trí, cấu hình, request |
| `document-merge.models.ts` | 125 | Kiểu dữ liệu tạo văn bản: header Excel, ghép trường, khối dấu, request, xem trước, kiểm tra, tiến độ |
| `file-upload.models.ts` | 5 | `IUploadedFile` (kết quả tải file lên thư mục tạm) |

### 2.4 `src/app/pages/ky-so/`

| File | Dòng | Vai trò |
|---|---|---|
| `ky-so.router.ts` | 6 | Route `''` → `KySo`, `data: { title: 'Ký số tài liệu', reuse: true }` |
| `ky-so.ts` | 125 | Trang Ký số: danh sách chứng thư, xác thực PIN, nguồn file, tóm tắt cấu hình, nhận bàn giao từ Tạo văn bản |
| `ky-so.html` | 88 | Bố cục 2 cột (nguồn file, cấu hình), thông báo hướng dẫn, `<app-sign-step>`, `<app-sign-config-dialog>` |
| `ky-so.scss` | 0 | Rỗng |
| `sign-step/sign-step.ts` | 261 | Liệt kê file, kiểm tra vị trí, chạy ký qua SSE, cập nhật bảng, mở thư mục kết quả |
| `sign-step/sign-step.html` | 182 | Thanh nguồn, khối kết quả kiểm tra vị trí, thanh tiến độ, bảng file (`p-table`), nút "Ký số" |
| `sign-step/sign-step.scss` | 3 | `:host { display: block; }` |
| `sign-step/sign-result-dialog/sign-result-dialog.ts` | 51 | Dialog kết quả ký: tính trạng thái thành công hay có lỗi |
| `sign-step/sign-result-dialog/sign-result-dialog.html` | 56 | Số file đã ký, số lỗi, thư mục lưu, nút "Mở thư mục" |
| `sign-step/sign-result-dialog/sign-result-dialog.scss` | 203 | Style thẻ kết quả (`srd-*`) |
| `sign-config-dialog/sign-config-dialog.ts` | 395 | Dialog cấu hình chữ ký: bản nháp cấu hình, vị trí riêng từng file, kéo khối chữ ký số, đo kích thước khối bằng canvas |
| `sign-config-dialog/sign-config-dialog.html` | 366 | Nội dung chiếu vào `stamp-designer`: thông tin ký, khối chữ ký số, hiệu chỉnh ảnh, chọn file xem trước; template lớp chữ ký `#signatureLayer` |
| `sign-config-dialog/sign-config-dialog.scss` | 103 | Style các phần cấu hình và `.signature-block` |

### 2.5 `src/app/pages/tao-van-ban/`

| File | Dòng | Vai trò |
|---|---|---|
| `tao-van-ban.router.ts` | 7 | Route `''` → `TaoVanBan`, `data: { title: MERGE_PAGE_TITLE, reuse: true }` |
| `tao-van-ban.ts` | 218 | Wizard 4 bước, giữ toàn bộ state, dựng request, tự đọc dữ liệu, kiểm tra |
| `tao-van-ban.html` | 126 | Thanh bước, thân bước (`@switch`), thanh điều hướng dưới |
| `tao-van-ban.scss` | 117 | Style `.wizard` |
| `merge-source-step/merge-source-step.ts` | 35 | Bước 1: chọn mẫu Word và file Excel, báo sai đuôi file |
| `merge-source-step/merge-source-step.html` | 104 | Hai `app-path-picker`, trạng thái đọc dữ liệu, danh sách trường và cột |
| `merge-mapping-step/merge-mapping-step.ts` | 91 | Bước 2: ghép trường với cột, cột đặt tên file, ghép tự động lại, xoá ghép |
| `merge-mapping-step/merge-mapping-step.html` | 117 | Lưới ghép trường, khối kiểm tra dữ liệu |
| `merge-mapping-step/merge-mapping-step.scss` | 7 | `.mapping-grid` |
| `merge-stamp-step/merge-stamp-step.ts` | 115 | Bước 3: tạo bản xem trước theo dòng, cache URL xem trước, chuyển chế độ "Đặt vị trí" và "Bản hoàn chỉnh" |
| `merge-stamp-step/merge-stamp-step.html` | 31 | Bọc `app-stamp-designer`, điều hướng dòng dữ liệu |
| `merge-run-step/merge-run-step.ts` | 197 | Bước 4: kiểm tra lại, chạy tạo file qua SSE, dừng, mở file, bàn giao sang Ký số |
| `merge-run-step/merge-run-step.html` | 141 | Thư mục xuất, tên file, ô "Ký số sau khi tạo", tổng quan, bảng tiến trình |
| `merge-run-step/merge-run-step.scss` | 7 | `.run-grid` |
| `merge-result-dialog/merge-result-dialog.ts` | 22 | Dialog kết quả tạo file |
| `merge-result-dialog/merge-result-dialog.html` | 50 | Số thành công, số lỗi, thư mục kết quả |

### 2.6 `src/app/services/`

| File | Dòng | Vai trò |
|---|---|---|
| `desktop.service.ts` | 97 | Cầu nối với vỏ Photino qua `window.external.sendMessage/receiveMessage`: hộp thoại chọn file, thư mục, mở đường dẫn; `apiBase` |
| `file-picker.service.ts` | 58 | Lớp bọc chọn file theo loại (`word`, `excel`, `pdf`, `image`, `folder`), kiểm tra đuôi file, tải file kéo thả |
| `file-upload.service.ts` | 22 | `POST /core/document/tai-len` (multipart) |
| `sign.service.ts` | 128 | 6 lời gọi API ký số, dựng URL xem file, bản sao riêng của hàm đọc SSE |
| `document-merge.service.ts` | 50 | 4 lời gọi API tạo văn bản, dựng URL xem file và xem ảnh |
| `sign-config-state.service.ts` | 18 | Giữ `ISignConfig` trong signal, đọc và ghi localStorage |
| `sign-cert-state.service.ts` | 7 | Giữ kết quả xác thực PIN gần nhất (`verifiedCert`) |
| `sign-handoff.service.ts` | 24 | Chuyển danh sách đường dẫn từ Tạo văn bản sang Ký số |

### 2.7 `src/app/shared/`

| File | Dòng | Vai trò |
|---|---|---|
| `utils.ts` | 163 | Lớp `Utils` gồm 23 hàm tĩnh; chỉ 6 hàm được gọi từ bên ngoài |
| `sse-stream.ts` | 58 | `streamSse<T>()`: POST bằng `fetch`, đọc luồng `data: …\n\n` thành `Observable` |
| `import.shared.ts` | 39 | Mảng `SharedImports`: `CommonModule`, `NgClass`, `FormsModule`, `ReactiveFormsModule` và 15 module PrimeNG |
| `app-route-reuse.strategy.ts` | 44 | Giữ lại component của route có `data.reuse === true` khi chuyển tab |
| `components/base/base-component.ts` | 153 | Lớp cha của hầu hết component: toast, confirm, kiểm tra response, hook `onRouteActivated` |
| `components/stamp-designer/stamp-designer.ts` | 557 | Đặt, kéo, đổi cỡ khối ảnh (dấu đỏ, chữ ký tươi) trên trang PDF; quy tắc trang; vị trí riêng từng trang |
| `components/stamp-designer/stamp-designer.html` | 422 | Cột trái (ảnh mẫu, danh sách khối, thuộc tính), cột phải (xem trước + lớp phủ) |
| `components/stamp-designer/stamp-designer.scss` | 250 | Style ô ảnh mẫu, hàng khối, nút chọn, chip trang, `.stamp-layer`, `.stamp-block` |
| `components/pdf-pages/pdf-pages.ts` | 187 | Hiển thị PDF bằng pdf.js: đo trang, render lười theo vùng nhìn, co giãn theo bề rộng |
| `components/pdf-pages/pdf-pages.html` | 26 | Mỗi trang một `<canvas>` + lớp phủ `#pageOverlay` + nhãn số trang |
| `components/pdf-pages/pdf-pages.scss` | 51 | Style vùng cuộn và trang |
| `components/pdf-source-picker/pdf-source-picker.ts` | 122 | Danh sách nguồn (file PDF hoặc thư mục): chọn, dán, kéo thả, bỏ |
| `components/pdf-source-picker/pdf-source-picker.html` | 69 | Hai nút chọn, ô dán đường dẫn (khi chạy web), danh sách nguồn |
| `components/pdf-source-picker/pdf-source-picker.scss` | 20 | `.source-drop` |
| `components/path-picker/path-picker.ts` | 96 | Chọn một đường dẫn (file hoặc thư mục): hộp thoại, kéo thả, dán |
| `components/path-picker/path-picker.html` | 112 | Vùng thả hoặc hàng gọn, ô dán đường dẫn (khi chạy web) |
| `components/breadcrumb/breadcrumb.ts` | 14 | Bọc `p-breadcrumb`; chỉ `ky-so` dùng |
| `components/breadcrumb/breadcrumb.html` | 1 | `<p-breadcrumb …>` |
| `components/breadcrumb/breadcrumb.scss` | 4 | Bỏ padding của `.p-breadcrumb` |
| `components/data-table/data-table.ts` | 171 | Bảng dùng chung kiểu CRUD (phân trang, đổi cỡ cột, lưu bề rộng cột). **Không nơi nào dùng** |
| `components/data-table/data-table.html` | 100 | Template `p-table` + `p-paginator` |
| `components/data-table/data-table.scss` | 29 | Ghi đè style `.p-datatable` |
| `constants/api.constants.ts` | 2 | `API_VERSION`, `API_PREFIX` |
| `constants/sign.constants.ts` | 62 | Hằng số ký số, cấu hình mặc định, khoá localStorage |
| `constants/document-merge.constants.ts` | 92 | Enum và nhãn của wizard, khối dấu, kiểu lặp, chế độ xem trước |
| `constants/path-picker.constants.ts` | 11 | Loại đường dẫn và đuôi file cho phép |
| `constants/data-table.constants.ts` | 11 | `CellViewTypes` (chỉ `data-table` dùng) |
| `models/pdf-pages.models.ts` | 9 | `IPdfPageMetric`, `IPdfPage` |
| `models/request-paging.base.models.ts` | 25 | Vỏ response `IBaseResponse`, `IBaseResponseWithData<T>` và kiểu phân trang (không dùng) |
| `models/environment.models.ts` | 4 | `IEnvironment` |
| `models/data-table.models.ts` | 20 | `IColumn`, `ICustomEmit` (chỉ `data-table` dùng) |

---

## 3. Models và constants (chép nguyên văn)

### 3.1 `src/app/models/sign.models.ts` (toàn bộ file)

```ts
import { ECertSource } from '../shared/constants/sign.constants';
import { IMergeStamp } from './document-merge.models';

export interface ISignatureDetail {
  signedBy?: string;
  verdict: number;
  reason?: string;
  trustedTime?: string;
  signingTime?: string;
}

export interface ISignCert {
  subject: string;
  commonName: string;
  issuer: string;
  issuerCommonName: string;
  serialNumber: string;
  thumbprint: string;
  source: ECertSource;
  keyProvider?: string | null;
  validFrom: string;
  validTo: string;
  hasPrivateKey: boolean;
  isExpired: boolean;
  allowsSigning: boolean;
  isTrusted: boolean;
  canSign: boolean;
  reason?: string | null;
}

export interface ITokenVerify {
  valid: boolean;
  thumbprint: string;
  commonName?: string | null;
  issuerCommonName?: string | null;
  foundInStore: boolean;
  hasPrivateKey: boolean;
  notExpired: boolean;
  allowsSigning: boolean;
  canSignTest: boolean;
  trustedChain: boolean;
  failedCheck?: string | null;
  reason?: string | null;
}

export interface ISignFileResult {
  fileName: string;
  outputPath?: string | null;
  success: boolean;
  reason?: string | null;
  verdict: number;
  verifyReason?: string | null;
  trustedTime?: string | null;
  signedTime?: string | null;
  signatureCount?: number;
  validSignatureCount?: number;
  signatures?: ISignatureDetail[];
}

export interface ISignResult {
  outputFolder: string;
  totalFiles: number;
  successCount: number;
  failedCount: number;
}

export interface ISignProgress {
  targetType: string;
  target: string;
  status: number;
  code: number;
  message: string;
  completed: boolean;
  current: number;
  total: number;
  percent: number;
  files?: string[];
  file?: ISignFileResult;
  summary?: ISignResult;
}

export interface ISignFileItem {
  name: string;
  fullPath: string;
  sizeBytes: number;
  alreadySigned: boolean;
}

export interface ISignFileList {
  files: ISignFileItem[];
  total: number;
}

export interface ISignPreview {
  index: number;
  total: number;
  hasPrev: boolean;
  hasNext: boolean;
  files: ISignFileItem[];
}

export interface ISignPosition {
  pageNumber: number;
  xRatio: number;
  yRatio: number;
  widthRatio: number;
  heightRatio: number;
}

export interface ISignLayout {
  signaturePosition: ISignPosition | null;
  stamps: IMergeStamp[];
}

export interface ISignFileConfig extends ISignLayout {
  path: string;
}

export interface ISignConfig extends ISignLayout {
  reason: string;
  location: string;
  fontFamily: string;
  fontSize: number;
  textColor: string;
  showSignature: boolean;
  embedSignatureInImage: boolean;
  sealImagePath: string;
  wetSignatureImagePath: string;
  sealDarkness: number;
  wetSignatureDarkness: number;
  wetSignatureStroke: number;
  wetSignatureColor: string | null;
  fileOverrides: ISignFileConfig[];
}

export interface ISignPositionCheckRequest {
  sourcePaths: string[];
  showSignature: boolean;
  signaturePosition: ISignPosition | null;
  fileOverrides: ISignFileConfig[];
}

export interface ISignPositionFile {
  fileName: string;
  fullPath: string;
  pageCount: number;
  blockCount: number;
  readable: boolean;
  fitted: boolean;
  issues: string[];
}

export interface ISignPositionCheck {
  totalFiles: number;
  fittedCount: number;
  adjustedCount: number;
  unreadableCount: number;
  files: ISignPositionFile[];
}

export interface ISignRequest extends ISignConfig {
  thumbprint: string;
  sourcePaths: string[];
}

export interface ISignFileRow {
  name: string;
  signStatus: 'idle' | 'processing' | 'signed' | 'failed';
  signMessage?: string;
  signedTime?: string | null;
  trustedTime?: string | null;
  signatureCount?: number;
  validSignatureCount?: number;
  signatures?: ISignatureDetail[];
}
```

Ghi chú sử dụng thực tế:

- `ISignPreview` và hàm `SignService.getPreview` (`sign.service.ts:40-45`) không được component nào gọi.
- `ISignFileItem.sizeBytes` và `alreadySigned` có nhận về nhưng không hiển thị ở đâu. `SignStep` chỉ giữ lại `name` (`sign-step.ts:115`).
- `ISignCert`: giao diện chỉ dùng `commonName`, `thumbprint`, `serialNumber`, `issuerCommonName` (`ky-so.html:23-26`). `ITokenVerify`: chỉ dùng `valid`, `reason`, `thumbprint`.
- `ISignProgress`: frontend chỉ đọc `percent`, `targetType`, `file`, `summary`. Các trường `status`, `code`, `message`, `completed`, `current`, `total`, `files`, `target` không được đọc ở luồng ký.
- `verdict: number`: template coi `verdict === 0` là hợp lệ (`sign-step.html:149`). Phía backend `0` là `TimestampVerdict.ValidTrusted` (đối chiếu `ktl.be.infrastructure.external/Documents/Dtos/TimestampVerdict.cs:5`).
- `ISignLayout.stamps` dùng lại kiểu `IMergeStamp` của Tạo văn bản.

### 3.2 `src/app/models/document-merge.models.ts` (toàn bộ file)

```ts
import {
  EMergeStampKind,
  EMergeStampRepeatMode,
  EMergeStampScope,
  MergeRowStatus
} from '../shared/constants/document-merge.constants';

export interface IExcelHeader {
  columnNumber: number;
  name: string;
}

export interface IMergeFieldMapping {
  fieldName: string;
  columnNumber: number | null;
  headerName?: string | null;
  autoMapped?: boolean;
}

export interface IMergeMapping {
  templatePath: string;
  dataPath: string;
  totalRows: number;
  headers: IExcelHeader[];
  fields: IMergeFieldMapping[];
}

export interface IMergeStampArea {
  xRatio: number;
  yRatio: number;
  widthRatio: number;
  heightRatio: number;
}

export interface IMergeStampPosition extends IMergeStampArea {
  pageNumber: number;
}

export interface IMergeStamp {
  kind: EMergeStampKind;
  scope: EMergeStampScope;
  repeatMode: EMergeStampRepeatMode;
  customPages: string | null;
  pageNumber: number;
  xRatio: number;
  yRatio: number;
  widthRatio: number;
  heightRatio: number;
  pagePositions: IMergeStampPosition[];
}

export interface IMergeRequest {
  templatePath: string;
  dataPath: string;
  mappings: IMergeFieldMapping[];
  fileNameColumnNumber: number | null;
  outputFolder: string | null;
  sealImagePath: string | null;
  wetSignatureImagePath: string | null;
  stamps: IMergeStamp[];
}

export interface IMergePreviewRequest extends IMergeRequest {
  rowNumber: number | null;
}

export interface IMergePreview {
  rowNumber: number;
  fileName: string;
  filePath: string;
  pageCount: number;
  warnings: string[];
}

export interface IMergeRowCheck {
  rowNumber: number;
  fileName: string;
  warnings: string[];
}

export interface IMergeValidation {
  totalRows: number;
  mappedFieldCount: number;
  unmappedFields: string[];
  warnings: string[];
  warningRowCount: number;
  rows: IMergeRowCheck[];
}

export interface IMergeFileResult {
  rowNumber: number;
  fileName: string;
  outputPath?: string | null;
  success: boolean;
  reason?: string | null;
  warnings: string[];
}

export interface IMergeResult {
  outputFolder: string;
  totalFiles: number;
  successCount: number;
  failedCount: number;
}

export interface IMergeProgress {
  targetType: string;
  target: string;
  status: number;
  code: number;
  message: string;
  completed: boolean;
  current: number;
  total: number;
  percent: number;
  files?: string[];
  file?: IMergeFileResult;
  summary?: IMergeResult;
}

export interface IMergeRunRow extends IMergeRowCheck {
  status: MergeRowStatus;
  message?: string | null;
  outputPath?: string | null;
}
```

Ý nghĩa của `IMergeStamp` (suy ra từ `stamp-designer.ts`):

| Trường | Ý nghĩa |
|---|---|
| `kind` | Loại ảnh: `1` dấu đỏ, `2` chữ ký tươi |
| `scope` | `0` chỉ trang đặt (`pageNumber`), `1` lặp theo `repeatMode` |
| `repeatMode` | `0` tất cả, `1` trang chẵn, `2` trang lẻ, `3` tuỳ chọn (`customPages`) |
| `customPages` | Chuỗi kiểu `1,3,5-7`; `null` khi không dùng |
| `pageNumber` | Trang gốc nơi người dùng thả khối (đánh số từ 1). Không sửa được sau khi thả |
| `xRatio`, `yRatio`, `widthRatio`, `heightRatio` | Vị trí và kích thước chung, áp cho mọi trang không có vị trí riêng |
| `pagePositions` | Vị trí riêng từng trang; mỗi phần tử `{ pageNumber, xRatio, yRatio, widthRatio, heightRatio }` |

### 3.3 `src/app/models/file-upload.models.ts` (toàn bộ file)

```ts
export interface IUploadedFile {
  fileName: string;
  filePath: string;
  sizeBytes: number;
}
```

### 3.4 `src/app/shared/models/*.ts`

`pdf-pages.models.ts`:

```ts
export interface IPdfPageMetric {
  number: number;
  widthPt: number;
  heightPt: number;
}

export interface IPdfPage extends IPdfPageMetric {
  cssWidth: number;
  cssHeight: number;
}
```

`request-paging.base.models.ts`:

```ts
export interface IBaseRequestPaging {
  pageSize: number
  pageNumber: number
  keyword?: string
}

export type IBaseResponse = {
  status: number
  code: number
  message: string
}

export type IBaseResponseWithData<T> = IBaseResponse & {
  data: T
}

export type IBaseResponsePaging<T> = IBaseResponse & {
  data: IBaseResponsePagingData<T>
}

export type IBaseResponsePagingData<T> = {
  customData: any
  items: T[]
  totalItems: number
}
```

Thành công khi `status === 1` (`base-component.ts:76`). `IBaseRequestPaging`, `IBaseResponsePaging`, `IBaseResponsePagingData` không được dùng.

`environment.models.ts`:

```ts
export interface IEnvironment {
  production: boolean;
  baseUrl: string;
}
```

`data-table.models.ts` (chỉ component `data-table` không dùng tới mới cần):

```ts
export type IColumn = {
  header: string,
  field?: string,
  headerContainerClass?: string,
  headerContainerStyle?: string,
  cellClass?: string,
  cellStyle?: string,
  cellViewType?: string,
  cellRender?: string,
  dateFormat?: string,
  customComponent?: any,
  clickable?: boolean,
  isFrozenRight?: boolean,
  statusSeverityFunction?: (rowData: any) => 'success' | 'secondary' | 'info' | 'warn' | 'danger' | 'contrast' | null | undefined;
}

export type ICustomEmit<T> = {
  type: string,
  data: T
}
```

### 3.5 `src/app/shared/constants/sign.constants.ts` (toàn bộ file)

```ts
import { ISignConfig } from '../../models/sign.models';

export enum ECertSource {
  Local = 0,
  Server = 1,
  UsbToken = 2
}

export const SIGN_PAGE_URL = '/ky-so';

export type SignRowStatus = 'idle' | 'processing' | 'signed' | 'failed';

export const SIGN_BLOCK_MIN_SCALE = 0.5;
export const SIGN_BLOCK_MAX_SCALE = 4;

export const SIGN_IMAGE_BLOCK_WIDTH_PT = 100;

export const SIGN_FONT_FAMILIES = ['Times New Roman', 'Arial', 'Verdana', 'Georgia', 'Courier New'];
export const SIGN_FONT_SIZE_MIN = 6;
export const SIGN_FONT_SIZE_MAX = 24;
export const SIGN_FONT_SIZE_STEP = 0.5;
export const SIGN_LINE_HEIGHT_RATIO = 1.15;
export const SIGN_REFERENCE_PAGE_WIDTH_PT = 595;
export const SIGN_REFERENCE_PAGE_HEIGHT_PT = 842;
export const SIGN_PAGE_MARGIN_PT = 8;
export const SIGN_SIGNER_PLACEHOLDER = 'Người ký';
export const SIGN_TIME_PREVIEW_FORMAT = 'YYYY-MM-DDTHH:mm:ss.SSSSSSS[+07:00]';

export const SIGN_IMAGE_DARKNESS_MIN = 40;
export const SIGN_IMAGE_DARKNESS_MAX = 250;
export const SIGN_IMAGE_DARKNESS_BRIGHTNESS_FACTOR = 0.5;
export const SIGN_IMAGE_DARKNESS_MIN_BRIGHTNESS = 0.5;
export const SIGN_IMAGE_STROKE_MIN = 0;
export const SIGN_IMAGE_STROKE_MAX = 200;

export const SIGN_CONFIG_STORAGE_KEY = 'saomai_kyso_sign_config';

export const SIGN_DEFAULT_CONFIG: ISignConfig = {
  reason: '',
  location: '',
  fontFamily: 'Times New Roman',
  fontSize: 10,
  textColor: '#000000',
  showSignature: true,
  embedSignatureInImage: false,
  sealImagePath: '',
  wetSignatureImagePath: '',
  sealDarkness: 140,
  wetSignatureDarkness: 140,
  wetSignatureStroke: 100,
  wetSignatureColor: null,
  signaturePosition: null,
  stamps: [],
  fileOverrides: []
};

export const SIGN_STATUS_LABELS: Record<SignRowStatus, string> = {
  idle: 'Chưa ký',
  processing: 'Đang ký',
  signed: 'Đã ký',
  failed: 'Ký lỗi'
};
```

Ghi chú: `SIGN_STATUS_LABELS` và `SignRowStatus` không được dùng (template `sign-step.html` viết cứng nhãn; `ISignFileRow.signStatus` khai lại union). `ECertSource` chỉ dùng làm kiểu của `ISignCert.source`.

### 3.6 `src/app/shared/constants/document-merge.constants.ts` (toàn bộ file)

```ts
export enum EMergeStep {
  Source = 0,
  Mapping = 1,
  Stamp = 2,
  Run = 3
}

export const MERGE_STEPS: { step: EMergeStep; label: string }[] = [
  { step: EMergeStep.Source, label: 'Nguồn' },
  { step: EMergeStep.Mapping, label: 'Ghép trường' },
  { step: EMergeStep.Stamp, label: 'Xem trước' },
  { step: EMergeStep.Run, label: 'Tạo file' }
];

export const MERGE_PAGE_TITLE = 'Tạo văn bản';

export enum EMergeStampKind {
  Seal = 1,
  WetSignature = 2
}

export const MERGE_STAMP_KIND_LABELS: Record<EMergeStampKind, string> = {
  [EMergeStampKind.Seal]: 'Dấu đỏ',
  [EMergeStampKind.WetSignature]: 'Chữ ký tươi'
};

export enum EMergeStampScope {
  CurrentPage = 0,
  Repeat = 1
}

export const MERGE_STAMP_SCOPE_OPTIONS: { label: string; value: EMergeStampScope }[] = [
  { label: 'Trang hiện tại', value: EMergeStampScope.CurrentPage },
  { label: 'Toàn bộ', value: EMergeStampScope.Repeat }
];

export enum EMergeStampRepeatMode {
  All = 0,
  Even = 1,
  Odd = 2,
  Custom = 3
}

export const MERGE_STAMP_REPEAT_OPTIONS: { label: string; value: EMergeStampRepeatMode }[] = [
  { label: 'Tất cả', value: EMergeStampRepeatMode.All },
  { label: 'Trang chẵn', value: EMergeStampRepeatMode.Even },
  { label: 'Trang lẻ', value: EMergeStampRepeatMode.Odd },
  { label: 'Tùy chọn', value: EMergeStampRepeatMode.Custom }
];

export enum EMergeStampEditScope {
  AllPages = 'all',
  CurrentPage = 'page'
}

export const MERGE_STAMP_EDIT_SCOPE_OPTIONS: { label: string; value: EMergeStampEditScope }[] = [
  { label: 'Mọi trang đã chọn', value: EMergeStampEditScope.AllPages },
  { label: 'Chỉ trang đang kéo', value: EMergeStampEditScope.CurrentPage }
];

export const MERGE_PAGE_CHIP_LIMIT = 24;

export const MERGE_CUSTOM_PAGE_PATTERN = /^(\d+)(?:\s*-\s*(\d+))?$/;
export const MERGE_CUSTOM_PAGE_SEPARATOR = ',';

export enum EMergePreviewMode {
  Edit = 'edit',
  Final = 'final'
}

export const MERGE_PREVIEW_MODE_OPTIONS: { label: string; value: EMergePreviewMode; icon: string }[] = [
  { label: 'Đặt vị trí', value: EMergePreviewMode.Edit, icon: 'pi pi-arrows-alt' },
  { label: 'Bản hoàn chỉnh', value: EMergePreviewMode.Final, icon: 'pi pi-eye' }
];

export const MERGE_TEMPLATE_EXTENSION = '.docx';
export const MERGE_DATA_EXTENSION = '.xlsx';

export type MergeRowStatus = 'idle' | 'processing' | 'done' | 'failed';

export const MERGE_ROW_STATUS_LABELS: Record<MergeRowStatus, string> = {
  idle: 'Chờ tạo',
  processing: 'Đang tạo',
  done: 'Đã tạo',
  failed: 'Lỗi'
};

export const MERGE_PROGRESS_TARGET = {
  manifest: 'manifest',
  file: 'file',
  summary: 'summary'
} as const;
```

Ghi chú: `EMergeStampEditScope` và `EMergePreviewMode` là state của giao diện, không gửi lên backend. `MERGE_PREVIEW_MODE_OPTIONS[].icon` là class PrimeIcons.

### 3.7 `src/app/shared/constants/path-picker.constants.ts` (toàn bộ file)

```ts
export type PathPickerKind = 'word' | 'excel' | 'pdf' | 'folder' | 'image';

export const PATH_PICKER_EXTENSIONS: Record<PathPickerKind, string[]> = {
  word: ['.docx'],
  excel: ['.xlsx'],
  pdf: ['.pdf'],
  image: ['.png', '.jpg', '.jpeg'],
  folder: []
};

export const PATH_PICKER_DROPPED_LABEL = 'Đã kéo thả vào ứng dụng';
```

### 3.8 `api.constants.ts` và `data-table.constants.ts` (toàn bộ file)

```ts
export const API_VERSION = 'v1';
export const API_PREFIX = `/api/${API_VERSION}`;
```

```ts
export class CellViewTypes {
  static INDEX: string ='INDEX'
  static DATE: string ='DATE'
  static CURRENCY: string ='CURRENCY'
  static CHECKBOX: string ='CHECKBOX'
  static CUSTOM_COMP: string ='CUSTOM_COMP'
  static LINK_BLANK: string ='LINK_BLANK'
  static STATUS: string ='STATUS'
  static SECRET: string = 'SECRET'
  static BOOL_CHECK: string = 'BOOL_CHECK'
}
```

### 3.9 Hằng số nằm ngoài thư mục `constants`

| Vị trí | Hằng số | Giá trị |
|---|---|---|
| `shared/components/pdf-pages/pdf-pages.ts:25` | `PAGE_MIN_WIDTH_PX` | `320` |
| `pdf-pages.ts:26` | `PAGE_MAX_WIDTH_PX` | `1100` |
| `pdf-pages.ts:27` | `HOST_PADDING_PX` | `40` |
| `pdf-pages.ts:28` | `RESIZE_SETTLE_MS` | `250` |
| `pdf-pages.ts:29` | `RENDER_SCALE` | `Math.max(window.devicePixelRatio \|\| 1, 2)` |
| `pdf-pages.ts:30` | `RENDER_ROOT_MARGIN` | `'300px'` |
| `layout/service/layout.service.ts:3` | `DESIGN_WIDTH_PX` | `1440` |
| `layout.service.ts:4` | `MIN_ZOOM` | `0.6` |
| `pages/ky-so/sign-config-dialog/sign-config-dialog.ts:57-61` | `REFERENCE_PAGE` | `{ number: 1, widthPt: 595, heightPt: 842 }` |
| `sign-config-dialog.html:93` | Cỡ chữ ô mẫu | `draft().fontSize * 1.2` (px) |
| `sign-config-dialog.html:252` | Màu khởi tạo bộ chọn màu mực khi `wetSignatureColor` là `null` | `'#1e3a8a'` |
| `shared/components/base/base-component.ts:19-20` | `MAX_PAGE_SIZE`, `START_PAGE_NUMBER` | `10`, `1` (không dùng) |
| `base-component.ts:93,102,111` | Thời gian hiện toast | `life: 4000` (ms) |
| `shared/components/data-table/data-table.ts:45` | `STORAGE_PREFIX` | `'tbl_col_widths_'` |
| `pages/tao-van-ban/merge-mapping-step/merge-mapping-step.ts:14-18` | `FIELD_STATUS` | Nhãn `'✓ Tự động'`, `'Đã chọn'`, `'Chưa ghép'` kèm class Tailwind |
| `src/styles.css:55-61` | Biến CSS | `--layout-topbar-height: 3rem`, `--layout-statusbar-height: 1.75rem`, `--layout-border: #d9dee7`, `--layout-chrome-bg: #f3f4f6`, `--layout-accent: #4f46e5` |

---

## 4. Cấu hình lưu trên máy

### 4.1 Các khoá lưu trữ

| Khoá | Nội dung | Đọc | Ghi |
|---|---|---|---|
| `saomai_kyso_sign_config` (`SIGN_CONFIG_STORAGE_KEY`, `sign.constants.ts:36`) | JSON của nguyên object `ISignConfig` | `services/sign-config-state.service.ts:9-12`, qua `Utils.getLocalStorage` (`utils.ts:4-10`). Đọc **một lần** khi service được tạo | `sign-config-state.service.ts:14-17`, qua `Utils.setLocalStorage` (`utils.ts:12-14`). Nơi gọi duy nhất: `KySo.onConfigSave` (`ky-so.ts:92-95`) khi bấm "Lưu cấu hình" trong dialog |
| `tbl_col_widths_<tableKey>` (`data-table.ts:45`) | JSON `Record<number, number>`: chỉ số cột → bề rộng px | `data-table.ts:61-70` | `data-table.ts:72-76` |

Khoá thứ hai chỉ tồn tại trên lý thuyết vì component `data-table` không được dùng ở đâu.

Không dùng `sessionStorage` (chỉ có 4 hàm bọc trong `utils.ts:20-34`, không ai gọi), không IndexedDB, không cookie.

### 4.2 Cách đọc và giá trị mặc định

```ts
// sign-config-state.service.ts:9-17
readonly config = signal<ISignConfig>({
  ...SIGN_DEFAULT_CONFIG,
  ...Utils.getLocalStorage(SIGN_CONFIG_STORAGE_KEY)
});

save(config: ISignConfig): void {
  this.config.set(config);
  Utils.setLocalStorage(SIGN_CONFIG_STORAGE_KEY, config);
}
```

```ts
// utils.ts:4-14
public static getLocalStorage(key: string) {
    try {
        return JSON.parse(localStorage.getItem(key) || '');
    } catch (error) {
        return {};
    }
}

public static setLocalStorage(key: string, data: any) {
    localStorage.setItem(key, JSON.stringify(data));
}
```

- Khi chưa có khoá, `JSON.parse('')` ném lỗi và hàm trả `{}`, nên cấu hình bằng `SIGN_DEFAULT_CONFIG`.
- Gộp nông (shallow): trường nào có trong localStorage thì ghi đè nguyên trường đó. Không có số phiên bản, không kiểm tra kiểu, không có hàm nâng cấp. Một `stamps[i]` thiếu `pagePositions` sẽ làm `stamp-designer.ts:348` ném lỗi.
- Khi mở dialog, bản nháp được dựng lại bằng `structuredClone({ ...SIGN_DEFAULT_CONFIG, ...this.config() })` (`sign-config-dialog.ts:134`).

Cấu trúc và giá trị mặc định (`sign.constants.ts:38-55`):

| Trường | Kiểu | Mặc định | Giới hạn | Nơi sửa |
|---|---|---|---|---|
| `reason` | `string` | `''` | Cắt khoảng trắng khi lưu (`sign-config-dialog.ts:286`) | Ô "Lý do ký" |
| `location` | `string` | `''` | Cắt khoảng trắng khi lưu (`:287`) | Ô "Nơi ký" |
| `fontFamily` | `string` | `'Times New Roman'` | Một trong `SIGN_FONT_FAMILIES` | "Font chữ" |
| `fontSize` | `number` | `10` | 6–24, bước 0,5 (pt, tính cho trang tham chiếu 595×842) | "Cỡ chữ" |
| `textColor` | `string` | `'#000000'` | `p-colorpicker format="hex"` | "Màu chữ" |
| `showSignature` | `boolean` | `true` | | Công tắc "Khối chữ ký số" |
| `embedSignatureInImage` | `boolean` | `false` | | "Nhồi chữ ký số vào ảnh" |
| `sealImagePath` | `string` | `''` | Đường dẫn tuyệt đối, đã qua `normalizeFolderPath` (dấu `/`) | Ô ảnh "Dấu đỏ" |
| `wetSignatureImagePath` | `string` | `''` | Như trên | Ô ảnh "Chữ ký tươi" |
| `sealDarkness` | `number` | `140` | 40–250 (%) | "Độ đậm dấu đỏ" |
| `wetSignatureDarkness` | `number` | `140` | 40–250 (%) | "Độ đậm chữ ký tươi" |
| `wetSignatureStroke` | `number` | `100` | 0–200 (%) | "Độ dày nét chữ ký tươi" |
| `wetSignatureColor` | `string \| null` | `null` | `null` nghĩa là giữ màu gốc của ảnh | "Màu mực chữ ký tươi" |
| `signaturePosition` | `ISignPosition \| null` | `null` | `null` nghĩa là góc trên bên phải trang 1 | Kéo khối chữ ký, "Trang đặt khối", nút "Mặc định" |
| `stamps` | `IMergeStamp[]` | `[]` | | Thả, kéo khối ảnh |
| `fileOverrides` | `ISignFileConfig[]` | `[]` | Mỗi phần tử `{ path, signaturePosition, stamps }` | Công tắc "Vị trí riêng file này" |

### 4.3 Ví dụ giá trị lưu

Ví dụ dưới đây dựng từ interface để minh hoạ hình dạng dữ liệu, không lấy từ máy thật:

```json
{
  "reason": "Phê duyệt văn bản",
  "location": "Hà Nội",
  "fontFamily": "Times New Roman",
  "fontSize": 10,
  "textColor": "#000000",
  "showSignature": true,
  "embedSignatureInImage": false,
  "sealImagePath": "D:/ChuKy/condau.png",
  "wetSignatureImagePath": "",
  "sealDarkness": 140,
  "wetSignatureDarkness": 140,
  "wetSignatureStroke": 100,
  "wetSignatureColor": null,
  "signaturePosition": { "pageNumber": 1, "xRatio": 0.62, "yRatio": 0.05, "widthRatio": 0.31, "heightRatio": 0.027 },
  "stamps": [
    {
      "kind": 1, "scope": 1, "repeatMode": 3, "customPages": "1,3-4", "pageNumber": 1,
      "xRatio": 0.7, "yRatio": 0.8, "widthRatio": 0.168, "heightRatio": 0.119,
      "pagePositions": [{ "pageNumber": 3, "xRatio": 0.1, "yRatio": 0.8, "widthRatio": 0.168, "heightRatio": 0.119 }]
    }
  ],
  "fileOverrides": [
    { "path": "D:\\HoSo\\vb_01.pdf", "signaturePosition": null, "stamps": [] }
  ]
}
```

Hai điểm về định dạng:

- Enum lưu dạng **số** (`kind`, `scope`, `repeatMode`). Nếu struct Rust dùng `serde` mặc định thì enum ra dạng chuỗi, không khớp.
- `fileOverrides[].path` là `fullPath` do backend trả (`sign-config-dialog.ts:183`), tức đường dẫn Windows dùng `\` (đối chiếu `ktl.be/ktl.be.applications/Document/Implements/SignService.cs:127`). Các đường dẫn còn lại trong cấu hình và danh sách nguồn dùng `/`. Hai định dạng cùng tồn tại trong một object.

### 4.4 Cấu hình cũ có thật sự được giữ lại giữa các lần chạy không

localStorage gắn với origin (giao thức + host + cổng). Vỏ desktop cũ khởi động máy chủ nội bộ ở cổng do hệ điều hành cấp rồi nạp giao diện từ địa chỉ đó (đối chiếu `ktl.be/ktl.be.desktop/Program.cs:28-29,48`):

```csharp
app.Urls.Clear();
app.Urls.Add("http://127.0.0.1:0");
...
.Load(new Uri(url));
```

Cổng `0` nghĩa là mỗi lần chạy nhận một cổng khác, origin đổi theo, và localStorage của lần chạy trước không còn nhìn thấy. Suy ra: ở bản 1.4.0, cấu hình chữ ký nhiều khả năng **trở về mặc định mỗi lần mở lại ứng dụng**. Đây là suy luận từ code, chưa chắc, cần kiểm chứng trên một máy đang cài 1.4.0.

Hệ quả cho bản mới: không cần (và gần như không thể) viết bước chuyển dữ liệu từ localStorage cũ sang `config.json`. Cấu trúc ở mục 4.2 chỉ dùng để bảo đảm không thiếu trường.

### 4.5 State chỉ nằm trong bộ nhớ

Những thứ sau **không** được lưu, mất khi đóng ứng dụng:

- Chứng thư đang chọn (`KySo.thumbprint`, `ky-so.ts:34`) và kết quả xác thực PIN (`SignCertStateService.verifiedCert`).
- Danh sách nguồn cần ký (`KySo.sources`, `ky-so.ts:37`).
- Toàn bộ state của Tạo văn bản (`tao-van-ban.ts:41-58`): đường dẫn mẫu, dữ liệu, thư mục xuất, bảng ghép trường, cột đặt tên file, ảnh dấu, danh sách khối dấu. Ảnh dấu và khối dấu của Tạo văn bản tách biệt với cấu hình ký số.
- Ô "Ký số sau khi tạo" (`merge-run-step.ts:39`).

State này sống qua việc chuyển tab nhờ `AppRouteReuseStrategy` (cả hai route đều có `data.reuse: true`), không phải nhờ lưu trữ.

### 4.6 Đối chiếu với `config.json` trong kế hoạch (mục 11)

Bảng này chỉ nêu chỗ khớp và chỗ lệch, không phải thiết kế. Các chỗ lệch cần chốt trước khi viết `smks-core::config`.

| Trường cũ | Trường trong ví dụ `config.json` | Nhận xét |
|---|---|---|
| `reason`, `location` | `signature.reason`, `signature.location` | Mặc định cũ là chuỗi rỗng; ví dụ mới là `"Ký số tài liệu"`, `"Việt Nam"` |
| `fontFamily`, `fontSize`, `textColor` | `signature.font.family`, `.size`, `.color` | Khớp, cùng mặc định |
| `showSignature` | `profiles[].display` (kiểu "Ẩn") | Cũ là một cờ chung; mới theo từng mẫu |
| `embedSignatureInImage` | `profiles[].display` (kiểu "Chỉ ảnh", theo kế hoạch mục 7.3) | |
| `sealImagePath`, `wetSignatureImagePath` | `signature.imagePath`, `profiles[].imagePath` | Cũ có hai ảnh theo loại; ví dụ mới có một ảnh chung và ảnh theo mẫu. Chưa thấy chỗ cho hai loại ảnh của `stamps` |
| `sealDarkness`, `wetSignatureDarkness` | `signature.image.darkness` | Cũ có **hai** giá trị độ đậm; ví dụ mới có một |
| `wetSignatureStroke` | `signature.image.stroke` | Khớp |
| `wetSignatureColor` | `signature.image.inkColor` | Khớp, đều cho phép `null` |
| `signaturePosition` (tỉ lệ 0–1 + trang) | `profiles[]` (`page`, `anchor`, `marginMm`, `scale`) | Hai mô hình khác nhau, không đổi qua lại trực tiếp được |
| `stamps` | `signature.stamps` | Khớp nếu giữ `IMergeStamp` |
| `fileOverrides` | Không có | Cần quyết định có giữ "vị trí riêng từng file" không. Nếu giữ thì đây là dữ liệu theo lô, không nên nằm trong cấu hình lâu dài (mục 11, lỗi số 8) |

---

## 5. Luồng Ký số

### 5.1 Các thành phần tham gia

```
KySo (pages/ky-so/ky-so.ts)
├─ PdfSourcePicker        chọn file, thư mục            → sources: string[]
├─ p-select chứng thư     chọn chứng thư                → thumbprint
├─ SignConfigDialog       cấu hình, bọc StampDesigner   → ISignConfig (lưu localStorage)
└─ SignStep               liệt kê, kiểm tra vị trí, ký  (chỉ hiện khi có thumbprint và có nguồn)
   └─ SignResultDialog    kết quả

Service: SignService (HTTP + SSE), SignConfigStateService (cấu hình),
         SignCertStateService (chứng thư đã xác thực), SignHandoffService (nhận file từ Tạo văn bản),
         DesktopService + FilePickerService (hộp thoại, mở thư mục)
```

Route `ky-so` có `data.reuse: true` (`ky-so.router.ts:5`), nên component `KySo` không bị huỷ khi chuyển sang tab Tạo văn bản. `ngOnInit` chỉ chạy một lần trong cả phiên.

### 5.2 Từng bước

| # | Bước | Hàm, vị trí | Việc làm |
|---|---|---|---|
| 0 | Khởi tạo | `KySo.ngOnInit`, `ky-so.ts:64-67` | Gọi `takeHandoff()` rồi `loadCertificates()` |
| 1a | Chọn file | `PdfSourcePicker.addFiles`, `pdf-source-picker.ts:40-43` → `FilePickerService.browsePdfFiles`, `file-picker.service.ts:34-38` → `DesktopService.pickPdfFiles`, `desktop.service.ts:59-65` | Gửi `'pickPdf'` cho vỏ desktop, nhận về `'pdf:<đường dẫn 1>\|<đường dẫn 2>…'`. Cho chọn nhiều file |
| 1b | Chọn thư mục | `addFolder`, `pdf-source-picker.ts:45-49` → `browse('folder')`, `file-picker.service.ts:28-32` → `pickFolder`, `desktop.service.ts:27-33` | Gửi `'pickFolder'`, nhận `'folder:<đường dẫn>'` |
| 1c | Gộp nguồn | `addSources`, `pdf-source-picker.ts:111-115` | Chuẩn hoá bằng `Utils.normalizeFolderPath`, bỏ rỗng, bỏ trùng, nối vào cuối `sources` |
| 2 | Nạp chứng thư | `KySo.loadCertificates`, `ky-so.ts:73-86` → `SignService.getCertificates(true)`, `sign.service.ts:26-31` | `GET …/sign/chung-thu-so?onlySignable=true`. Lỗi thì báo "Không thể đọc chứng thư số trên máy. Vui lòng thử lại sau." |
| 2b | Chọn chứng thư | `p-select`, `ky-so.html:20-31`; `onCertChange`, `ky-so.ts:88-90` | Nhãn `commonName`, giá trị `thumbprint`, lọc theo `commonName,serialNumber,issuerCommonName` |
| 3 | Xác thực PIN (không bắt buộc) | `KySo.verify`, `ky-so.ts:97-119` → `SignService.verifyToken`, `sign.service.ts:33-38` | `POST …/sign/kiem-tra-token` với `{ thumbprint }`. `valid` thì ghi `certState.verifiedCert` và báo "Chứng thư số đã sẵn sàng để ký."; không thì xoá `verifiedCert` và báo `reason` |
| 4 | Mở cấu hình | Nút "Cấu hình", `ky-so.html:51-57`; `SignConfigDialog.openEffect`, `sign-config-dialog.ts:131-138` | Dựng bản nháp từ cấu hình hiện tại, gọi `loadFiles()` |
| 4b | Nạp file để xem trước | `SignConfigDialog.loadFiles`, `sign-config-dialog.ts:297-315` → `SignService.getFiles`, `sign.service.ts:58-63` | `POST …/sign/danh-sach-file` với `{ sourcePaths }`. URL xem trước: `SignService.pdfFileUrl(fullPath)`, `sign.service.ts:54-56` |
| 4c | Lưu cấu hình | `SignConfigDialog.onSave`, `sign-config-dialog.ts:282-295` → `KySo.onConfigSave`, `ky-so.ts:92-95` → `SignConfigStateService.save` | Cắt khoảng trắng `reason`, `location`; tính lại kích thước khối chữ ký (`fitSignature`) cho cấu hình chung và mọi `fileOverrides`; ghi localStorage; báo "Đã lưu cấu hình chữ ký." |
| 5 | Liệt kê file | `effect` trong constructor `SignStep`, `sign-step.ts:44-56` → `startCheck`, `sign-step.ts:99-123` | Chạy khi `sources` đổi. `POST …/sign/danh-sach-file`. Mỗi file thành một dòng `{ name, signStatus: 'idle' }` |
| 5b | Kiểm tra vị trí | `SignStep.checkPositions`, `sign-step.ts:125-151` → `SignService.checkPositions`, `sign.service.ts:47-52` | Chạy trong `startCheck` và mỗi khi `config` đổi. `POST …/sign/kiem-tra-vi-tri` |
| 6 | Bấm "Ký số" | `SignStep.onSign`, `sign-step.ts:153-181` | Nếu chứng thư đã xác thực trong phiên thì ký luôn; chưa thì gọi `verifyToken` trước |
| 6b | Chạy ký | `SignStep.startSign`, `sign-step.ts:183-221` → `SignService.signFolder`, `sign.service.ts:65-70` | `POST …/sign/ky-thu-muc`, đọc SSE. Đặt mọi dòng sang `'processing'` |
| 7 | Nhận sự kiện | `SignStep.handleSignEvent`, `sign-step.ts:223-251` | Xem mục 5.4 |
| 8 | Kết quả | `SignResultDialog`, `sign-result-dialog.ts:25-50`; `SignStep.openFolder`, `sign-step.ts:253-260` | Hiện số đã ký, số lỗi, thư mục lưu. "Mở thư mục" gọi `DesktopService.openPath`; không được thì chép đường dẫn vào clipboard |

Điều kiện hiển thị (`ky-so.html:68-79`): chưa chọn chứng thư thì hiện "Chọn chứng thư số để bắt đầu."; chưa có nguồn thì hiện "Chọn file hoặc thư mục cần ký."; đủ cả hai mới tạo `<app-sign-step>`.

Chi tiết đáng chú ý:

- Trang Ký số đặt `[droppable]="false"` cho `PdfSourcePicker` (`ky-so.html:8`): **không** nhận kéo thả file ở màn này.
- Một nguồn được coi là thư mục khi không kết thúc bằng `.pdf` (`pdf-source-picker.ts:32-34`). Việc mở rộng thư mục thành danh sách file do backend làm: chỉ lấy file `.pdf` ngay trong thư mục (không đệ quy), bỏ file có tên khớp `_daky_\d{8}_\d{6}$` (đối chiếu `SignService.cs:110-117`, `SigningConstants.cs:33-35`).
- Khi chạy trên trình duyệt thường (không có vỏ desktop), `PdfSourcePicker` hiện thêm ô dán đường dẫn (`pdf-source-picker.html:32-45`, `pdf-source-picker.ts:51-62`).
- `canSign()` (`sign-step.ts:88-97`) không xét kết quả kiểm tra vị trí. Vị trí tràn trang hay file không đọc được đều không chặn nút "Ký số".
- Bảng sắp xếp theo tên kiểu tự nhiên bằng `Utils.compareDocSeq` (`sign-step.ts:74-76`).
- Danh sách file được gọi API **hai nơi độc lập** (`sign-step.ts:111` và `sign-config-dialog.ts:305`), cộng thêm `kiem-tra-vi-tri` cũng tự liệt kê lại. Phía backend, mỗi lần liệt kê là một lần kiểm tra chữ ký của từng file (đối chiếu `SignService.cs:122`).

### 5.3 Request gửi đi

**Kiểm tra vị trí** (`sign-step.ts:131-136`), kiểu `ISignPositionCheckRequest`:

```ts
{
  sourcePaths: sources,
  showSignature: config.showSignature,
  signaturePosition: config.signaturePosition,
  fileOverrides: config.fileOverrides
}
```

Request này không gửi `stamps` của cấu hình chung, nên bước kiểm tra chỉ xét khối chữ ký số, không xét khối dấu.

**Ký** (`sign-step.ts:187-191`), kiểu `ISignRequest`:

```ts
const req: ISignRequest = {
  ...this.config(),
  thumbprint: this.thumbprint(),
  sourcePaths: sources
};
```

Tức 18 trường: 16 trường của `ISignConfig` (`reason`, `location`, `fontFamily`, `fontSize`, `textColor`, `showSignature`, `embedSignatureInImage`, `sealImagePath`, `wetSignatureImagePath`, `sealDarkness`, `wetSignatureDarkness`, `wetSignatureStroke`, `wetSignatureColor`, `signaturePosition`, `stamps`, `fileOverrides`) cộng `thumbprint` và `sourcePaths`. `fileOverrides` được gửi nguyên, kể cả phần tử của file không nằm trong lô này.

Không có trường nào về thư mục kết quả, hậu tố tên file, xử lý trùng tên. Backend tự đặt file kết quả cạnh file gốc với tên `<tên>_daky_<yyyyMMdd_HHmmss>.pdf` (đối chiếu `SignService.cs:354-356`).

### 5.4 Xử lý sự kiện SSE

Mỗi sự kiện là một `ISignProgress`. Hàm xử lý (`sign-step.ts:223-251`):

```ts
private handleSignEvent(ev: ISignProgress): void {
  if (typeof ev.percent === 'number') {
    this.percent.set(ev.percent);
  }

  if (ev.targetType === 'file' && ev.file) {
    const f = ev.file;
    this.fileRows.update((rows) =>
      rows.map((r) =>
        r.name === f.fileName
          ? {
              ...r,
              signStatus: f.success ? 'signed' : 'failed',
              signMessage: f.success ? undefined : f.verifyReason || f.reason || undefined,
              signedTime: f.signedTime,
              trustedTime: f.trustedTime,
              signatureCount: f.signatureCount,
              validSignatureCount: f.validSignatureCount,
              signatures: f.signatures
            }
          : r
      )
    );
  } else if (ev.targetType === 'summary' && ev.summary) {
    this.percent.set(100);
    this.signResult.set(ev.summary);
    this.showResultDialog.set(true);
  }
}
```

| Sự kiện | Xử lý |
|---|---|
| `manifest` | Không có nhánh riêng. Chỉ `percent` (bằng 0) được ghi. `files` và `target` bị bỏ qua |
| `file` | Tìm dòng theo `r.name === f.fileName`, đặt `'signed'` hoặc `'failed'`. Lý do lỗi ưu tiên `verifyReason` rồi tới `reason`. Chép `signedTime`, `trustedTime`, `signatureCount`, `validSignatureCount`, `signatures` |
| `summary` | Đặt tiến độ 100, lưu `ev.summary`, mở dialog kết quả |
| Luồng kết thúc (`complete`) | `signing.set(false)` (`sign-step.ts:219`) |

- `percent` do backend tính (`Math.Round(Current * 100.0 / Total)`, đối chiếu `SignProgressDto.cs:23`). Frontend không tự tính.
- Mọi dòng chuyển sang `'processing'` cùng lúc ngay khi bắt đầu (`sign-step.ts:196-204`) vì backend ký song song. Không có khái niệm "file đang ký".
- Lý do lỗi của từng file chỉ nằm trong thuộc tính `title` của ô trạng thái (`sign-step.html:99-104`), phải rê chuột mới thấy.
- `ev.message` không được hiển thị ở đâu.

### 5.5 Huỷ, lỗi, token bị rút

**Huỷ.** Không có nút huỷ ký. Cách duy nhất để request bị huỷ là `Subscription.unsubscribe()`, khi đó `AbortController.abort()` được gọi (`sign.service.ts:125`). Việc này xảy ra ở ba chỗ:

- `SignStep.ngOnDestroy` (`sign-step.ts:80-82`). Chỉ chạy khi `<app-sign-step>` bị gỡ, tức khi danh sách nguồn về rỗng. Chuyển tab không gỡ component (route được giữ lại), nên lô ký vẫn chạy nền.
- `startCheck` (`sign-step.ts:105`) và `startSign` (`sign-step.ts:193`) đều `this.sub?.unsubscribe()` trước khi gán `sub` mới. Vì cùng một biến `sub` giữ cả request liệt kê lẫn request ký, đổi nguồn giữa lúc đang ký sẽ huỷ lô ký (mục 11, lỗi số 1).

**Lỗi.** Nhánh `error` của `startSign` (`sign-step.ts:208-218`):

```ts
error: (err: IBaseResponse | Error) => {
  this.signing.set(false);
  this.fileRows.update((rows) =>
    rows.map((r) => (r.signStatus === 'processing' ? { ...r, signStatus: 'idle' as const } : r))
  );
  if ('status' in err) {
    this.isResponseSucceed(err);
    return;
  }
  this.messageError('Không kết nối được tới dịch vụ ký số. Vui lòng thử lại sau.');
},
```

Có hai loại lỗi: backend từ chối ngay từ đầu và trả JSON `{ status, code, message }` thay vì luồng sự kiện (hàm đọc SSE ném nguyên object đó, `sign.service.ts:90-93`), khi đó hiện `message` của backend; mọi lỗi khác hiện câu chung. Các dòng đang `'processing'` trở về `'idle'`.

Lỗi của từng file không đi vào nhánh này mà đến qua sự kiện `file` với `success: false`.

**Token bị rút.** Frontend không có code nào riêng cho trường hợp này. Mọi thứ do backend làm (đối chiếu `SignService.cs:308-326, 419-462`): cứ 2 giây kiểm tra token; mất token thì huỷ lô, gửi sự kiện `file` thất bại cho từng file chưa xong với lý do "Đã rút USB token khi đang ký nên dừng tiến trình.", rồi gửi `summary` có `message` "Đã rút token khi đang ký. Ký thành công {n}/{tổng} file.". Người dùng chỉ thấy các dòng "Ký lỗi" và dialog "Ký số hoàn tất — có lỗi"; câu thông báo trong `summary.message` không được hiện. `verifiedCert` cũng không bị xoá (mục 11, lỗi số 3).

**Ký lại.** Bấm "Ký số" lần nữa sẽ ký lại toàn bộ file trong danh sách, kể cả file vừa ký xong, và tạo thêm bản mới với mốc thời gian mới.

### 5.6 Chênh lệch với `JobEvent` trong kế hoạch (mục 10.2)

| Frontend cũ đang dùng | `JobEvent` trong kế hoạch | Việc cần làm |
|---|---|---|
| `percent` có sẵn trong mọi sự kiện | Không có `percent`; có `current`, `total` ở sự kiện `file` | Tính ở giao diện |
| Khớp dòng ký theo `file.fileName` | `index`, `file` | Khớp theo `index` hoặc đường dẫn đầy đủ. Đừng khớp theo tên |
| Khớp dòng tạo văn bản theo `file.rowNumber` (`merge-run-step.ts:162`) | Không có `rowNumber` | Thêm `rowNumber` hoặc bảo đảm `index` trùng thứ tự `rows` |
| `file.success: boolean` | `status: 'ok' \| 'error' \| 'skipped'` | Sửa điều kiện |
| `file.signedTime`, `trustedTime`, `signatureCount`, `validSignatureCount`, `signatures[]` (`signingTime`, `trustedTime`, `verdict`, `reason`), `verifyReason` | Chưa có | Nếu bản mới còn các cột "Chữ ký hợp lệ", "Time ký", "Dấu thời gian" thì sự kiện `file` phải mang các trường này |
| `summary.successCount`, `failedCount`, `totalFiles` | `success`, `failed`, `total` | Đổi tên |
| Kết thúc khi luồng HTTP đóng | Kết thúc khi nhận `summary` | Lớp bọc `Channel` phải tự `complete` sau `summary` |
| Huỷ bằng cách huỷ request | Lệnh `job_cancel(job_id)` | Teardown của Observable phải gọi `job_cancel` |
| Không phân biệt lý do dừng | `stoppedReason: 'user' \| 'token_removed'` | Hiện thông báo riêng |

---

## 6. Luồng Tạo văn bản

### 6.1 Wizard và state

`TaoVanBan` (`pages/tao-van-ban/tao-van-ban.ts`) là wizard 4 bước theo `EMergeStep`: `Source` "Nguồn", `Mapping` "Ghép trường", `Stamp` "Xem trước", `Run` "Tạo file". Component cha giữ toàn bộ state (`tao-van-ban.ts:41-58`); mỗi bước là một component con được tạo và huỷ theo `@switch (step())` (`tao-van-ban.html:42-86`), nhận state qua `input`/`model`.

Quy tắc điều hướng:

- `goToStep` (`tao-van-ban.ts:143-146`) chỉ cho lùi về bước đã qua, và không cho khi `busy()`.
- `canNext` (`tao-van-ban.ts:107-119`): bước Nguồn cần `sourceReady()`; bước Ghép trường cần `fields().length > 0` (trường chưa ghép vẫn được đi tiếp); bước Xem trước cần `!stampEnabled() || stamps().length > 0`.
- `busy = reading || validating || running` (`tao-van-ban.ts:60`).

### 6.2 Từng bước

| # | Bước | Hàm, vị trí | Việc làm |
|---|---|---|---|
| 1 | Chọn mẫu Word, file Excel | `MergeSourceStep`, hai `app-path-picker` (`merge-source-step.html:9-31`) | `kind="word"` chấp nhận `.docx`, `kind="excel"` chấp nhận `.xlsx`. Sai đuôi thì báo đỏ (`merge-source-step.ts:27-34`) |
| 1b | Tự đọc dữ liệu | `autoRead`, `tao-van-ban.ts:126-132` → `readData`, `tao-van-ban.ts:177-200` → `DocumentMergeService.autoMap`, `document-merge.service.ts:25-29` | Khi một trong hai đường dẫn đổi, đủ điều kiện và bản ghép hiện có không còn khớp: `GET …/tao-van-ban/map-tu-dong?templatePath=…&dataPath=…`. Kết quả ghi vào `mapping`, `fields`; đặt lại `fileNameColumnNumber = null` và `stamps = []`. Có bộ đếm `readVersion` để bỏ response cũ |
| 1c | "Tiếp theo" | `next`, `tao-van-ban.ts:152-167` | `mappingFresh()` thì sang bước 2; không thì `readData(true)` rồi mới sang |
| 2 | Ghép trường | `MergeMappingStep.onColumnChange`, `merge-mapping-step.ts:56-65` | Đổi cột của một trường, đặt `autoMapped: false` |
| 2b | Ghép tự động lại | `remap`, `merge-mapping-step.ts:75-90` | Gọi lại `map-tu-dong`, báo "Đã ghép tự động theo tên cột Excel." |
| 2c | Xoá ghép | `clearMapping`, `merge-mapping-step.ts:67-73` | Hỏi xác nhận rồi đặt mọi `columnNumber` về `null` |
| 2d | Cột đặt tên file | `p-select`, `merge-mapping-step.html:8-19` | `null` nghĩa là "Theo số dòng Excel". Ví dụ tên: `<tên mẫu>_<giá trị cột>.pdf` (`merge-mapping-step.ts:46-49`) |
| 2e | Kiểm tra | `TaoVanBan.validate`, `tao-van-ban.ts:202-217` → `DocumentMergeService.validate`, `document-merge.service.ts:31-33` | `POST …/tao-van-ban/kiem-tra` với `baseRequest()`. "Tiếp theo" ở bước 2 tự gọi nếu chưa có kết quả |
| 3 | Xem trước | `MergeStampStep.loadPreview`, `merge-stamp-step.ts:74-114` → `DocumentMergeService.preview`, `document-merge.service.ts:35-37` | `POST …/tao-van-ban/xem-truoc`. Trả `IMergePreview`; URL để pdf.js đọc là `previewFileUrl(filePath)` (`document-merge.service.ts:39-41`) |
| 3b | Đóng dấu | `StampDesigner` (mục 7) | Bật công tắc "Dấu & chữ ký", chọn ảnh, thả khối lên trang |
| 3c | Bản hoàn chỉnh | Chuyển `mode` sang `Final` | Gọi lại `xem-truoc` kèm ảnh và `stamps` để backend đóng dấu thật vào bản xem trước |
| 4 | Kiểm tra lại | `MergeRunStep.ngOnInit` → `validate`, `merge-run-step.ts:67-96` | `POST …/tao-van-ban/kiem-tra` với `request()` đầy đủ (có khối dấu). Dựng bảng dòng với `status: 'idle'` |
| 4b | Tạo file | `TaoVanBan.startRun` → `MergeRunStep.start`, `merge-run-step.ts:98-124` → `DocumentMergeService.merge`, `document-merge.service.ts:47-49` | `POST …/tao-van-ban/sinh-file`, đọc SSE |
| 4c | Dừng | `MergeRunStep.stop`, `merge-run-step.ts:126-137` | Hỏi xác nhận, `unsubscribe()` (huỷ request), báo "Đã dừng tạo file." |
| 4d | Kết quả | `handleProgress`, `merge-run-step.ts:152-184`; `MergeResultDialog` | Xem mục 6.4 |
| 5 | Sang Ký số | `MergeRunStep.goToSign`, `merge-run-step.ts:139-141` → `SignHandoffService.open` | Xem mục 6.6 |

### 6.3 Request

Có ba biến thể dựng từ cùng state.

`baseRequest` (`tao-van-ban.ts:85-94`), dùng cho kiểm tra ở bước 2 và làm nền cho xem trước:

```ts
baseRequest = computed<IMergeRequest>(() => ({
  templatePath: Utils.normalizeFolderPath(this.templatePath()),
  dataPath: Utils.normalizeFolderPath(this.dataPath()),
  mappings: this.fields().map(({ fieldName, columnNumber }) => ({ fieldName, columnNumber })),
  fileNameColumnNumber: this.fileNameColumnNumber(),
  outputFolder: Utils.normalizeFolderPath(this.outputFolder()) || null,
  sealImagePath: null,
  wetSignatureImagePath: null,
  stamps: []
}));
```

`request` (`tao-van-ban.ts:96-105`), dùng cho bước 4:

```ts
request = computed<IMergeRequest>(() =>
  this.stampEnabled()
    ? {
        ...this.baseRequest(),
        sealImagePath: this.sealImagePath() || null,
        wetSignatureImagePath: this.wetSignatureImagePath() || null,
        stamps: this.stamps()
      }
    : this.baseRequest()
);
```

Request xem trước (`merge-stamp-step.ts:84-91`):

```ts
const final = mode === EMergePreviewMode.Final;
const request: IMergePreviewRequest = {
  ...this.request(),
  rowNumber,
  sealImagePath: final ? this.sealImagePath() || null : null,
  wetSignatureImagePath: final ? this.wetSignatureImagePath() || null : null,
  stamps: final ? this.stamps() : []
};
```

Ghi chú:

- `mappings` chỉ gửi `fieldName` và `columnNumber`; `headerName`, `autoMapped` bị bỏ.
- `outputFolder` rỗng gửi thành `null`. Khi đó backend lấy thư mục chứa file Excel làm gốc rồi tạo thư mục con `<tên mẫu>_<yyyyMMdd_HHmmss>` (đối chiếu `DocumentMergeService.cs:227-236`). Nếu file Excel là file kéo thả (nằm trong thư mục tạm) thì backend báo lỗi yêu cầu chọn thư mục lưu (`DocumentMergeService.cs:221-225`). Bản mới có đường dẫn thật khi kéo thả nên trường hợp này biến mất.
- `mappingFresh` (`tao-van-ban.ts:76-83`) so `mapping.templatePath` và `mapping.dataPath` do backend trả về với đường dẫn hiện tại sau khi chuẩn hoá. Cách này chỉ đúng vì backend trả lại nguyên chuỗi đã nhận (đối chiếu `DocumentMergeService.cs:72-73`). Lệnh `merge_auto_map` bản mới phải giữ tính chất này, hoặc frontend đổi cách so.
- `validationReset` (`tao-van-ban.ts:134-137`) xoá kết quả kiểm tra mỗi khi `baseRequest` đổi. `outputFolder` nằm trong `baseRequest`, nên đổi thư mục xuất ở bước 4 cũng xoá kết quả kiểm tra của cha (mục 11, lỗi số 24).

**Xem trước và cache.** Ở chế độ `Edit`, URL xem trước được cache theo `rowNumber` trong `editPreviews` (`merge-stamp-step.ts:42, 75-82, 104`). Chế độ `Final` không cache. Nút "Tạo lại bản xem trước" xoá cache của dòng đang xem (`merge-stamp-step.ts:67-72`). Có bộ đếm `previewVersion` để bỏ response cũ. Nếu tạo bản `Final` thất bại thì tự quay về `Edit` (`merge-stamp-step.ts:99-102`). Cache mất khi rời bước 3 vì component bị huỷ.

### 6.4 Xử lý sự kiện SSE

`handleProgress` (`merge-run-step.ts:152-184`):

```ts
private handleProgress(ev: IMergeProgress): void {
  if (typeof ev.percent === 'number') this.percent.set(ev.percent);

  if (ev.targetType === MERGE_PROGRESS_TARGET.manifest) {
    this.resultFolder.set(ev.target);
    this.markNextProcessing();
  } else if (ev.targetType === MERGE_PROGRESS_TARGET.file && ev.file) {
    const file = ev.file;
    this.rows.update((rows) =>
      rows.map((row) =>
        row.rowNumber === file.rowNumber
          ? {
              ...row,
              status: file.success ? 'done' : 'failed',
              message: file.success ? null : file.reason,
              outputPath: file.outputPath,
              warnings: [...new Set([...row.warnings, ...(file.warnings ?? [])])]
            }
          : row
      )
    );
    this.markNextProcessing();
  } else if (ev.targetType === MERGE_PROGRESS_TARGET.summary && ev.summary) {
    this.percent.set(100);
    this.resultFolder.set(ev.summary.outputFolder);
    this.result.set(ev.summary);
    if (this.signAfter() && ev.summary.failedCount === 0) {
      this.goToSign();
      return;
    }
    this.showResult.set(true);
  }
}
```

| Sự kiện | Xử lý |
|---|---|
| `manifest` | `ev.target` là thư mục kết quả, ghi vào `resultFolder`. Đánh dấu dòng `'idle'` đầu tiên là `'processing'` |
| `file` | Tìm dòng theo `rowNumber`; đặt `'done'` hoặc `'failed'`; ghi `message`, `outputPath`; gộp cảnh báo (bỏ trùng). Đánh dấu dòng `'idle'` kế tiếp là `'processing'` |
| `summary` | Tiến độ 100. Nếu bật "Ký số sau khi tạo" và không có lỗi thì chuyển thẳng sang Ký số, không mở dialog. Còn lại mở dialog kết quả |
| `complete` | `running = false`, `finished = true` (`merge-run-step.ts:119-122`) |

`markNextProcessing` (`merge-run-step.ts:186-192`) giả định backend xử lý tuần tự theo đúng thứ tự dòng. Giả định này đúng với bản cũ (đối chiếu `DocumentMergeService.cs:279-308`). Nếu bản Rust chạy song song thì dấu "Đang tạo" sẽ sai dòng.

### 6.5 Dừng và lỗi

- **Dừng** (`merge-run-step.ts:126-137`): sau khi xác nhận, `unsubscribe()` huỷ request; `running = false`, `finished = true`; dòng `'processing'` về `'idle'`. Sự kiện `summary` không bao giờ tới nên không có dialog kết quả, nhưng nút "Mở thư mục kết quả" vẫn dùng được vì `resultFolder` đã có từ `manifest`. File đã tạo được giữ lại.
- **Lỗi** (`merge-run-step.ts:110-118`): cùng mẫu với luồng ký. JSON lỗi của backend thì hiện `message`; còn lại hiện "Không kết nối được tới dịch vụ tạo văn bản. Vui lòng thử lại sau.".
- `MergeRunStep.ngOnDestroy` (`merge-run-step.ts:71-73`) huỷ request nếu component bị gỡ. Trong lúc chạy, wizard khoá điều hướng (`busy`), và chuyển tab không gỡ component, nên thực tế việc tạo file chạy nền được.

### 6.6 Chuyển sang Ký số (`sign-handoff.service.ts`)

Toàn bộ service:

```ts
@Injectable({ providedIn: 'root' })
export class SignHandoffService {
  private router = inject(Router);

  private readonly pendingSources = signal<string[]>([]);

  open(paths: string[]): void {
    if (paths.length === 0) return;
    this.pendingSources.set(paths.map((path) => Utils.normalizeFolderPath(path)));
    this.router.navigateByUrl(SIGN_PAGE_URL);
  }

  take(): string[] {
    const paths = this.pendingSources();
    this.pendingSources.set([]);
    return paths;
  }
}
```

- Bên gửi: `MergeRunStep.goToSign` gọi `open(signPaths())`. `signPaths` là `outputPath` của các dòng `'done'` (`merge-run-step.ts:58-62`). Được gọi tự động sau `summary` (khi bật "Ký số sau khi tạo" và không lỗi) hoặc khi bấm nút "Ký số N file" (`merge-run-step.html:76-78`). Nút này chỉ hiện khi đã chạy xong, ô "Ký số sau khi tạo" đang bật và có ít nhất một file thành công.
- Bên nhận: `KySo.takeHandoff` (`ky-so.ts:121-124`), gọi trong `ngOnInit` và trong `onRouteActivated`. Nếu có đường dẫn thì `this.sources.set(paths)`: **thay thế** danh sách nguồn hiện có, không nối thêm.
- `onRouteActivated` là hook của `BaseComponent` (`base-component.ts:24-42`), được gọi khi có `NavigationEnd` mà URL bắt đầu bằng đường dẫn của route chứa component. Cần hook này vì `KySo` được giữ lại bởi `AppRouteReuseStrategy` nên `ngOnInit` không chạy lại.
- Bản mới có thể bỏ hook: cho `KySo` dùng `effect` đọc thẳng signal `pendingSources`.

---

## 7. `stamp-designer`

File: `src/app/shared/components/stamp-designer/stamp-designer.ts` (557 dòng), `.html` (422 dòng), `.scss` (250 dòng). Trong mục này, số dòng không ghi tên file là của `stamp-designer.ts`.

### 7.1 Vai trò và giao diện component

Component gồm hai cột: cột trái (ảnh mẫu, danh sách khối, thuộc tính khối đang chọn) và cột phải (PDF xem trước do `app-pdf-pages` vẽ, phủ lên mỗi trang một lớp `.stamp-layer` chứa các khối). Nó được dùng ở hai nơi:

| Nơi dùng | Cấu hình | Nội dung chiếu vào |
|---|---|---|
| Bước 3 Tạo văn bản (`merge-stamp-step.html`) | Có công tắc bật tắt, có nút chuyển "Đặt vị trí" và "Bản hoàn chỉnh" | `[designerNav]`: chọn dòng dữ liệu |
| Dialog cấu hình chữ ký (`sign-config-dialog.html:23-40`) | `[toggleable]="false"`, `[enabled]="true"`, `[modeSwitch]="false"` | `[designerConfig]`: thông tin ký và khối chữ ký số; `[designerImage]`: hiệu chỉnh ảnh; `[designerNav]`: chọn file; `[pageExtra]`: template lớp chữ ký số |

Đầu vào, đầu ra (`:57-71`):

| Tên | Loại | Mặc định | Ý nghĩa |
|---|---|---|---|
| `previewUrl` | `input<string \| null>` | `null` | URL file PDF cho pdf.js |
| `previewLoading` | `input<boolean>` | `false` | Đang tạo bản xem trước |
| `previewWarnings` | `input<string[]>` | `[]` | Cảnh báo hiện ở chế độ `Final` |
| `emptyText` | `input<string>` | `'Đang tạo bản xem trước.'` | |
| `toggleable` | `input<boolean>` | `true` | Có công tắc bật tắt ở tiêu đề |
| `title` | `input<string>` | `'Dấu & chữ ký'` | |
| `modeSwitch` | `input<boolean>` | `true` | Có nút chuyển chế độ |
| `imageFilters` | `input<Partial<Record<EMergeStampKind, string>>>` | `{}` | Chuỗi CSS `filter` cho từng loại ảnh |
| `pageExtra` | `input<TemplateRef<{ $implicit: IPdfPage }> \| null>` | `null` | Template vẽ thêm vào mỗi trang |
| `pagesLoaded` | `output<IPdfPageMetric[]>` | | Phát khi pdf.js đo xong các trang |
| `enabled` | `model<boolean>` | `false` | |
| `mode` | `model<EMergePreviewMode>` | `Edit` | |
| `sealImagePath`, `wetSignatureImagePath` | `model<string>` | `''` | |
| `stamps` | `model<IMergeStamp[]>` | `[]` | Danh sách khối |

State nội bộ (`:83-89, 112-123`): `pageMetrics`, `activeIndex` (khối đang chọn, `-1` là không chọn), `activePage` (bản sao trên trang nào đang được chỉnh), `editScope`, `imageAspects` (tỉ lệ cao/rộng của từng loại ảnh), `tileDragKind`, `uploadingKind`, và các biến thường `paletteKind`, `paletteGrabX`, `paletteGrabY`, `drag`.

Lớp phủ chỉ được vẽ khi `enabled() && mode() === EMode.Edit` (`stamp-designer.html:365`). `canPlace` (`:108-110`) còn đòi có ít nhất một trang và không đang tải.

### 7.2 Hệ toạ độ

- **Đơn vị**: tỉ lệ 0–1. `xRatio`, `widthRatio` chia cho bề rộng trang; `yRatio`, `heightRatio` chia cho chiều cao trang.
- **So với gì**: hình chữ nhật của phần tử `.stamp-layer`. Phần tử này phủ kín trang hiển thị: `.pdf-pages__page` có kích thước `cssWidth × cssHeight` (`pdf-pages.html:3`), bên trong là `div.absolute.inset-0` (`pdf-pages.html:6`), bên trong nữa là `.stamp-layer { position: absolute; inset: 0; }` (`stamp-designer.scss:169-172`). `cssHeight = cssWidth × heightPt / widthPt` (`pdf-pages.ts:58`), còn `widthPt`, `heightPt` lấy từ `page.getViewport({ scale: 1 })` của pdf.js (`pdf-pages.ts:153-154`). Tức là so với **trang như người dùng nhìn thấy**, tính bằng point PDF.
- **Gốc**: góc trên bên trái của trang hiển thị.
- **Trục**: X sang phải, **Y hướng xuống**. `(xRatio, yRatio)` là góc trên trái của khối.

Bằng chứng trong code: vị trí con trỏ được đổi sang tỉ lệ bằng `(event.clientX - rect.left) / rect.width` và `(event.clientY - rect.top) / rect.height` với `rect = host.getBoundingClientRect()` (`:520-522`); khối được vẽ bằng `left`, `top`, `width`, `height` theo phần trăm (`stamp-designer.html:374-377`).

Đổi sang point trên trang hiển thị kích thước `W × H` point:

```
x_trái   = xRatio × W
y_trên   = yRatio × H            (đo từ mép trên xuống)
rộng     = widthRatio × W
cao      = heightRatio × H
y_đáy    = H − (yRatio + heightRatio) × H   (đo từ mép dưới lên, hệ PDF khi trang không xoay)
```

Đối chiếu backend cho thấy cùng cách hiểu: `SignaturePlacementResolver.cs:42` tính `bottom = pageBox.Top - placement.YRatio * pageBox.Height - height`; `SignService.cs:527` tính `Y = pageBox.Height - placement.YRatio * pageBox.Height - placement.HeightRatio * pageBox.Height`; `PdfImageStamper.cs:49-53` vẽ ảnh bằng PDFsharp (hệ gốc trên trái) với `stamp.YRatio * pageHeight` dùng thẳng.

Phép tính là tỉ số giữa hai số cùng hệ toạ độ client nên không phụ thuộc `devicePixelRatio`, bề rộng hiển thị của trang, hay CSS `zoom` mà `LayoutService` đặt lên phần tử gốc.

Chưa chắc: trang có `/UserUnit` khác 1 (pdf.js đưa hệ số này vào viewport; frontend không xét).

### 7.3 Trang xoay

Frontend **không có code xử lý xoay**: tìm chuỗi `rotat` trong toàn bộ `src/` không ra kết quả nào. Mọi thứ dựa vào pdf.js: `getViewport({ scale: 1 })` được gọi không truyền `rotation` (`pdf-pages.ts:153`), nên pdf.js áp `/Rotate` của trang. Với trang xoay 90 hoặc 270 độ, `widthPt` và `heightPt` đã đổi chỗ, canvas vẽ trang đã xoay, và tỉ lệ của khối tính trên trang đã xoay.

Việc đổi từ hệ hiển thị sang hệ toạ độ gốc của PDF nằm hoàn toàn ở backend (đối chiếu `PdfPageGeometryDto.cs:11-13` đổi chỗ rộng, cao cho 90/270; `PdfIncrementalSigner.cs:320-332` hàm `ToUserSpace`).

Điều kiện để bản mới không lệch: `smks-pdf::geometry` phải cho ra đúng khung hiển thị mà pdf.js dùng (CropBox giao MediaBox, đã áp `/Rotate`). Nếu hai bên khác nhau thì khối sẽ lệch dù tỉ lệ đúng.

### 7.4 Kích thước gốc và giữ tỉ lệ ảnh

Chép nguyên văn (`:229-249`):

```ts
private loadImageAspect(kind: EMergeStampKind, path: string): void {
  if (!path) return;
  const image = new Image();
  image.onload = () => {
    if (image.naturalWidth > 0) {
      this.imageAspects.update((map) => ({ ...map, [kind]: image.naturalHeight / image.naturalWidth }));
    }
  };
  image.onerror = () =>
    this.messageError(
      `Không mở được ảnh ${this.kindLabel(kind).toLowerCase()}. Kiểm tra lại file có tồn tại và là ảnh PNG hoặc JPEG.`
    );
  image.src = this.documentMergeService.imageUrl(path);
}

private baseSize(kind: EMergeStampKind, page: IPdfPageMetric): { width: number; height: number } {
  return {
    width: Math.min(SIGN_IMAGE_BLOCK_WIDTH_PT / page.widthPt, 1),
    height: Math.min((SIGN_IMAGE_BLOCK_WIDTH_PT * (this.imageAspects()[kind] ?? 1)) / page.heightPt, 1)
  };
}
```

- Khối gốc rộng **100 pt** (`SIGN_IMAGE_BLOCK_WIDTH_PT`), cao `100 × (cao ảnh / rộng ảnh)` pt. Hai kích thước được đổi sang tỉ lệ theo trục riêng nên hình dạng ảnh đúng trên trang đó. Ví dụ trang A4 595 × 842 pt, ảnh vuông: `widthRatio = 100/595 ≈ 0,168`, `heightRatio = 100/842 ≈ 0,119`.
- Tỉ lệ ảnh lấy bằng cách nạp ảnh qua `new Image()` mỗi khi đường dẫn ảnh đổi (`aspectEffect`, `:132-139`). Chưa nạp xong thì dùng `1` (ảnh vuông).
- Mọi thao tác đổi cỡ đều tính lại từ `baseSize` nhân một hệ số (mục 7.7), nên tỉ lệ ảnh luôn được giữ, trừ khi chạm trần `1` (khối rộng hoặc cao bằng cả trang).
- Ảnh trong khối vẽ bằng `object-fit: fill` (`stamp-designer.scss:210-215`): ảnh bị kéo giãn theo đúng `widthRatio`, `heightRatio` đã lưu.

Hệ quả của việc lưu rộng và cao thành hai tỉ lệ độc lập: khối đặt trên trang A4 dọc rồi lặp sang trang khổ khác hoặc trang ngang sẽ bị méo, vì mỗi tỉ lệ nhân với kích thước của trang đích. Điều này cũng xảy ra khi một cấu hình chung áp cho nhiều file khác khổ (mục 11, lỗi số 13).

### 7.5 Thả ảnh mẫu vào trang

Thao tác này dùng **kéo thả HTML5**, không phải pointer events. Ảnh mẫu là `<img [attr.draggable]="canPlace()" (dragstart)="onPaletteDragStart($event, kind)">` (`stamp-designer.html:40-51`). Lớp trang nghe `(dragover)` và `(drop)` (`stamp-designer.html:366`).

Chép nguyên văn (`:445-489`):

```ts
onPaletteDragStart(event: DragEvent, kind: EMergeStampKind): void {
  if (!this.canPlace()) {
    event.preventDefault();
    return;
  }
  event.dataTransfer?.setData('text/plain', String(kind));
  this.paletteKind = kind;

  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  this.paletteGrabX = rect.width > 0 ? (event.clientX - rect.left) / rect.width : 0;
  this.paletteGrabY = rect.height > 0 ? (event.clientY - rect.top) / rect.height : 0;
}

onPageDragOver(event: DragEvent): void {
  event.preventDefault();
}

onPageDrop(event: DragEvent, page: IPdfPage): void {
  event.preventDefault();
  if (!this.canPlace()) return;

  const kind = this.paletteKind;
  const size = this.baseSize(kind, page);
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const x = (event.clientX - rect.left) / rect.width - this.paletteGrabX * size.width;
  const y = (event.clientY - rect.top) / rect.height - this.paletteGrabY * size.height;

  this.stamps.update((list) => [
    ...list,
    {
      kind,
      scope: EMergeStampScope.CurrentPage,
      repeatMode: EMergeStampRepeatMode.All,
      customPages: null,
      pageNumber: page.number,
      xRatio: Utils.clamp(x, 0, 1 - size.width),
      yRatio: Utils.clamp(y, 0, 1 - size.height),
      widthRatio: size.width,
      heightRatio: size.height,
      pagePositions: []
    }
  ]);
  this.activeIndex.set(this.stamps().length - 1);
  this.activePage.set(page.number);
}
```

- `paletteGrabX`, `paletteGrabY` là vị trí nắm trong ảnh mẫu, tính theo tỉ lệ 0–1 của chính ảnh mẫu. Khi thả, điểm đó của khối mới trùng với con trỏ.
- Khối mới luôn có `scope: CurrentPage`, `repeatMode: All`, `customPages: null`, `pagePositions: []`, kích thước gốc.
- Loại ảnh lấy từ biến `this.paletteKind`, không đọc lại `dataTransfer`. `onPageDragOver` chấp nhận mọi thứ. Vì vậy thả bất kỳ thứ gì vào trang (ví dụ một file kéo từ Explorer) cũng tạo ra một khối của loại ảnh kéo gần nhất, mặc định là dấu đỏ (mục 11, lỗi số 12).
- Một khối không thể kéo sang trang khác, và giao diện không có chỗ sửa `pageNumber`. Muốn đổi trang gốc phải xoá khối rồi thả lại.

### 7.6 Kéo thả HTML5 không dùng lại được như cũ trên Tauri

Trên Windows, cấu hình cửa sổ `dragDropEnabled` của Tauri mặc định là `true`. Khi đó webview bắt sự kiện kéo thả ở mức native để phát `onDragDropEvent` (có đường dẫn thật), và các sự kiện kéo thả HTML5 trong trang không chạy (theo tài liệu cấu hình của Tauri 2; nên xác nhận lại ngay ở spike). Muốn dùng `draggable`/`dragstart`/`drop` thì phải đặt `dragDropEnabled: false`, nhưng như vậy mất `onDragDropEvent`, thứ mà kế hoạch (mục 10.1) dùng để nhận file kéo từ Explorer.

Bốn chỗ trong code cũ dựa vào kéo thả HTML5:

| Chỗ | Vị trí | Dữ liệu |
|---|---|---|
| Thả ảnh mẫu vào trang | `stamp-designer.ts:445-489`, `stamp-designer.html:47,50,366` | Kéo trong trang (không phải file) |
| Thả file ảnh vào ô ảnh mẫu | `stamp-designer.ts:171-203`, `stamp-designer.html:34-36` | `File` → tải lên thư mục tạm |
| Thả file PDF vào danh sách nguồn | `pdf-source-picker.ts:72-109` | `File[]` → tải lên |
| Thả file vào ô chọn đường dẫn | `path-picker.ts:58-95` | `File` → tải lên |

Hướng xử lý hợp với kế hoạch: giữ `dragDropEnabled: true`; ba chỗ nhận file chuyển sang `onDragDropEvent`; riêng thao tác thả ảnh mẫu vào trang viết lại bằng pointer events (cùng kiểu với di chuyển khối ở mục 7.7), giữ nguyên công thức tính `x`, `y` trong `onPageDrop`.

Lưu ý khi chuyển ba chỗ nhận file: `onDragDropEvent` là sự kiện của cả webview, kèm toạ độ, không gắn với phần tử DOM. Phải tự xác định vùng thả nào nằm dưới con trỏ. Toạ độ sự kiện tính theo pixel vật lý, còn trang đang bị `LayoutService` đặt CSS `zoom`, nên phép quy đổi cần thử kỹ (chưa chắc về cách quy đổi chính xác).

### 7.7 Di chuyển và đổi cỡ bằng pointer events

Mỗi khối `.stamp-block` nghe `pointerdown` (chế độ `'move'`), `pointermove`, `pointerup`. Bản sao đang chọn có thêm 4 nút góc `.stamp-block__handle--{nw,ne,sw,se}`, mỗi nút nghe cùng ba sự kiện với chế độ là tên góc (`stamp-designer.html:387-389, 402-410`). CSS đặt `touch-action: none` cho khối và nút góc, `pointer-events: none` cho ảnh bên trong.

Chép nguyên văn (`:491-556`):

```ts
onStampPointerDown(event: PointerEvent, item: IStampItem, mode: DragMode): void {
  event.preventDefault();
  event.stopPropagation();
  this.activeIndex.set(item.index);
  this.activePage.set(item.page);

  const host = (event.currentTarget as HTMLElement).closest('.stamp-layer') as HTMLElement | null;
  if (!host) return;

  const rect = host.getBoundingClientRect();
  this.drag = {
    mode,
    index: item.index,
    page: item.page,
    toPage: this.writesToPage(item.stamp, item.page),
    host,
    grabX: (event.clientX - rect.left) / rect.width - item.area.xRatio,
    grabY: (event.clientY - rect.top) / rect.height - item.area.yRatio
  };
  (event.target as HTMLElement).setPointerCapture(event.pointerId);
}

onStampPointerMove(event: PointerEvent): void {
  const drag = this.drag;
  const stamp = drag ? this.stamps()[drag.index] : null;
  const metric = drag ? this.pageMetrics().find((p) => p.number === drag.page) : null;
  if (!drag || !stamp || !metric) return;

  const current = drag.toPage ? this.areaOf(stamp, drag.page) : stamp;
  const rect = drag.host.getBoundingClientRect();
  const pointerX = (event.clientX - rect.left) / rect.width;
  const pointerY = (event.clientY - rect.top) / rect.height;

  let width = current.widthRatio;
  let height = current.heightRatio;
  let left = pointerX - drag.grabX;
  let top = pointerY - drag.grabY;

  if (drag.mode !== 'move') {
    const right = current.xRatio + current.widthRatio;
    const bottom = current.yRatio + current.heightRatio;
    const fromRight = drag.mode === 'ne' || drag.mode === 'se';
    const fromBottom = drag.mode === 'sw' || drag.mode === 'se';
    const draggedWidth = fromRight ? pointerX - current.xRatio : right - pointerX;

    const base = this.baseSize(stamp.kind, metric);
    const scale = Utils.clamp(draggedWidth / base.width, SIGN_BLOCK_MIN_SCALE, SIGN_BLOCK_MAX_SCALE);
    width = Math.min(base.width * scale, 1);
    height = Math.min(base.height * scale, 1);
    left = fromRight ? current.xRatio : right - width;
    top = fromBottom ? current.yRatio : bottom - height;
  }

  this.applyArea(drag.index, drag.page, drag.toPage, {
    xRatio: Utils.clamp(left, 0, 1 - width),
    yRatio: Utils.clamp(top, 0, 1 - height),
    widthRatio: width,
    heightRatio: height
  });
}

onStampPointerUp(event: PointerEvent): void {
  const target = event.target as HTMLElement;
  if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  this.drag = null;
}
```

Diễn giải:

- **Bắt đầu** (`pointerdown`): chọn khối và bản sao trên trang đó; ghi `grabX`, `grabY` là khoảng cách (theo tỉ lệ) từ góc trên trái của khối tới con trỏ; quyết định một lần cho cả thao tác là sẽ ghi vào vị trí chung hay vị trí riêng của trang (`toPage`, mục 7.8); gọi `setPointerCapture` để nhận `pointermove` cả khi con trỏ ra ngoài khối.
- **Di chuyển** (`mode === 'move'`): `left = pointerX − grabX`, `top = pointerY − grabY`; rộng, cao giữ nguyên.
- **Đổi cỡ** (kéo một góc): chỉ **vị trí ngang** của con trỏ có tác dụng. `draggedWidth` là khoảng cách ngang từ con trỏ tới cạnh đối diện. Hệ số `scale = clamp(draggedWidth / base.width, 0.5, 4)`. Rộng mới `= base.width × scale`, cao mới `= base.height × scale`, mỗi giá trị bị chặn trên bởi `1`. Góc đối diện đứng yên. Vậy khối rộng từ 50 pt đến 400 pt và luôn giữ tỉ lệ ảnh, trừ khi chạm trần `1`.
- **Giới hạn biên**: sau mọi phép tính, `xRatio` bị kẹp trong `[0, 1 − width]`, `yRatio` trong `[0, 1 − height]`. Khối không thể ra ngoài trang, không có lề an toàn.
- **Kết thúc** (`pointerup`): nhả capture, xoá `drag`.
- Đổi cỡ không dùng `grabX`, `grabY`, nên cạnh khối nhảy tới đúng vị trí con trỏ khi bắt đầu kéo (lệch tối đa bằng bán kính nút góc).

Bảng các chỗ kẹp biên trong toàn bộ luồng:

| Chỗ | Vị trí | Công thức |
|---|---|---|
| Thả khối mới | `:480-481` | `clamp(x, 0, 1 − size.width)`, `clamp(y, 0, 1 − size.height)` |
| Di chuyển, đổi cỡ | `:545-546` | `clamp(left, 0, 1 − width)`, `clamp(top, 0, 1 − height)` |
| "Kích cỡ gốc" | `:438-439` | `Math.min(area.xRatio, 1 − base.width)`, `Math.min(area.yRatio, 1 − base.height)` |
| Khối chữ ký số | `sign-config-dialog.ts:201-202, 225-226, 265-266` | `clamp(…, 0, 1 − size.widthRatio)`, `clamp(…, 0, 1 − size.heightRatio)` |
| Backend (đối chiếu) | `StampPlanner.cs:156-163`, `SignService.cs:602-603`, `SignaturePlacementResolver.cs:47-48` | Kẹp lại lần nữa |

Hàm "Kích cỡ gốc" (`:429-443`):

```ts
resetStampSize(): void {
  const stamp = this.activeStamp();
  const page = this.activeCopyPage();
  const metric = page !== null ? this.pageMetrics().find((p) => p.number === page) : null;
  if (!stamp || page === null || !metric) return;

  const area = this.areaOf(stamp, page);
  const base = this.baseSize(stamp.kind, metric);
  this.applyArea(this.activeIndex(), page, this.writesToPage(stamp, page), {
    xRatio: Math.min(area.xRatio, 1 - base.width),
    yRatio: Math.min(area.yRatio, 1 - base.height),
    widthRatio: base.width,
    heightRatio: base.height
  });
}
```

### 7.8 Nhiều khối, quy tắc trang, vị trí riêng từng trang

**Nhiều khối.** `stamps` là mảng; mỗi phần tử là một khối độc lập. Khối được nhận diện bằng **chỉ số trong mảng** (`activeIndex`, `IStampItem.index`), không có id. Xoá một khối làm chỉ số các khối sau dịch đi; code đặt `activeIndex = -1` sau khi xoá (`:416-420`). Một loại ảnh (dấu đỏ, chữ ký tươi) có thể có nhiều khối. Bỏ ảnh của một loại sẽ xoá mọi khối loại đó (`:205-214`).

**Quy tắc trang.** Chép nguyên văn (`:251-294`, `:302-326`):

```ts
resolvePages(stamp: IMergeStamp): number[] {
  const count = this.pageCount();
  if (stamp.scope === EMergeStampScope.CurrentPage) {
    return stamp.pageNumber <= count ? [stamp.pageNumber] : [];
  }

  const pages = Array.from({ length: count }, (_, i) => i + 1);
  switch (stamp.repeatMode) {
    case EMergeStampRepeatMode.Even:
      return pages.filter((page) => page % 2 === 0);
    case EMergeStampRepeatMode.Odd:
      return pages.filter((page) => page % 2 === 1);
    case EMergeStampRepeatMode.Custom: {
      const ranges = this.parseCustomPages(stamp.customPages);
      return ranges ? pages.filter((page) => ranges.some((r) => page >= r.from && page <= r.to)) : [];
    }
    default:
      return pages;
  }
}

customPagesError(stamp: IMergeStamp): string | null {
  if (stamp.scope !== EMergeStampScope.Repeat || stamp.repeatMode !== EMergeStampRepeatMode.Custom) return null;
  return this.parseCustomPages(stamp.customPages) ? null : 'Nhập danh sách trang dạng 1,3,5-7.';
}

private parseCustomPages(value: string | null): { from: number; to: number }[] | null {
  const parts = (value ?? '')
    .split(MERGE_CUSTOM_PAGE_SEPARATOR)
    .map((part) => part.trim())
    .filter((part) => part);
  if (parts.length === 0) return null;

  const ranges: { from: number; to: number }[] = [];
  for (const part of parts) {
    const match = MERGE_CUSTOM_PAGE_PATTERN.exec(part);
    if (!match) return null;
    const from = Number(match[1]);
    const to = match[2] ? Number(match[2]) : from;
    if (from < 1 || to < from) return null;
    ranges.push({ from, to });
  }
  return ranges;
}
```

```ts
togglePage(page: number): void {
  const stamp = this.activeStamp();
  if (!stamp) return;

  const pages = new Set(this.resolvePages(stamp));
  if (pages.has(page)) pages.delete(page);
  else pages.add(page);

  this.updateActiveStamp({
    scope: EMergeStampScope.Repeat,
    repeatMode: EMergeStampRepeatMode.Custom,
    customPages: this.compressPages([...pages])
  });
}

private compressPages(pages: number[]): string {
  const sorted = [...pages].sort((a, b) => a - b);
  const parts: string[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const from = sorted[i];
    while (i + 1 < sorted.length && sorted[i + 1] === sorted[i] + 1) i++;
    parts.push(from === sorted[i] ? `${from}` : `${from}-${sorted[i]}`);
  }
  return parts.join(MERGE_CUSTOM_PAGE_SEPARATOR);
}
```

| Lựa chọn trên giao diện | Giá trị | Trang được đóng |
|---|---|---|
| "Trang hiện tại" | `scope = 0` | Chỉ `pageNumber`; rỗng nếu `pageNumber` lớn hơn số trang |
| "Toàn bộ" + "Tất cả" | `scope = 1`, `repeatMode = 0` | Mọi trang |
| "Toàn bộ" + "Trang chẵn" | `repeatMode = 1` | 2, 4, 6… |
| "Toàn bộ" + "Trang lẻ" | `repeatMode = 2` | 1, 3, 5… |
| "Toàn bộ" + "Tùy chọn" | `repeatMode = 3`, `customPages` | Các trang trong chuỗi `1,3,5-7`; chuỗi sai thì rỗng |

- Chuỗi tuỳ chọn: tách theo dấu phẩy, bỏ phần rỗng, mỗi phần khớp `^(\d+)(?:\s*-\s*(\d+))?$`, `from ≥ 1`, `to ≥ from`. Một phần sai làm cả chuỗi sai.
- Bấm một chip trang (hoặc nút "Thêm trang N", "Bỏ trang N") gọi `togglePage`: khối tự chuyển sang `Repeat` + `Custom`, `customPages` được viết lại ở dạng nén (`1,3-5`).
- Chip trang chỉ hiện khi số trang không quá `MERGE_PAGE_CHIP_LIMIT = 24` (`:107`); nhiều hơn thì hiện chuỗi tóm tắt và một nút cho trang đang chỉnh (`stamp-designer.html:248-286`).
- Backend làm lại đúng phép tính này (đối chiếu `StampPlanner.cs:115-145, 167-204`), nhưng khác ở chỗ chuỗi sai hoặc rỗng: frontend chỉ hiện dòng chữ đỏ và coi như không có trang nào, backend ném lỗi và từ chối cả request (mục 11, lỗi số 15).

**Khối hiển thị trên từng trang.** Chép nguyên văn (`:328-353`):

```ts
pageItems(page: IPdfPage): IStampItem[] {
  return this.stamps().flatMap((stamp, index): IStampItem[] => {
    const applied = this.resolvePages(stamp).includes(page.number);
    const origin = stamp.pageNumber === page.number;
    if (!origin && !applied) return [];
    return [
      {
        index,
        page: page.number,
        stamp,
        area: this.areaOf(stamp, page.number),
        ghost: !origin,
        own: this.hasOwnPosition(stamp, page.number),
        excluded: origin && !applied
      }
    ];
  });
}

areaOf(stamp: IMergeStamp, page: number): IMergeStampArea {
  return stamp.pagePositions.find((position) => position.pageNumber === page) ?? stamp;
}

hasOwnPosition(stamp: IMergeStamp, page: number): boolean {
  return stamp.pagePositions.some((position) => position.pageNumber === page);
}
```

Một khối hiện trên trang gốc và trên mọi trang được áp. Ba cờ quyết định cách vẽ (`stamp-designer.html:370-373`, `stamp-designer.scss:180-208`):

| Cờ | Điều kiện | Hiển thị |
|---|---|---|
| `ghost` | Không phải trang gốc | Mờ 0,45 (trừ khi có vị trí riêng hoặc đang chọn) |
| `own` | Trang có phần tử trong `pagePositions` | Viền cam, nhãn "Riêng" |
| `excluded` | Là trang gốc nhưng quy tắc trang không áp cho trang đó | Mờ 0,4, viền đỏ đứt |

Mọi bản sao đều kéo và đổi cỡ được, kể cả bản mờ.

**Vị trí riêng từng trang.** Chép nguyên văn (`:371-395`):

```ts
resetPagePosition(page: number): void {
  const stamp = this.activeStamp();
  if (!stamp) return;
  this.updateActiveStamp({ pagePositions: stamp.pagePositions.filter((position) => position.pageNumber !== page) });
}

resetPagePositions(): void {
  this.updateActiveStamp({ pagePositions: [] });
}

private writesToPage(stamp: IMergeStamp, page: number): boolean {
  if (stamp.scope !== EMergeStampScope.Repeat) return false;
  return this.editScope() === EMergeStampEditScope.CurrentPage || this.hasOwnPosition(stamp, page);
}

private applyArea(index: number, page: number, toPage: boolean, area: IMergeStampArea): void {
  this.stamps.update((list) =>
    list.map((stamp, i) => {
      if (i !== index) return stamp;
      if (!toPage) return { ...stamp, ...area };
      const others = stamp.pagePositions.filter((position) => position.pageNumber !== page);
      return { ...stamp, pagePositions: [...others, { pageNumber: page, ...area }] };
    })
  );
}
```

Một thao tác kéo ghi vào đâu:

| `scope` | Lựa chọn "Khi kéo thả" (`editScope`) | Trang đang kéo đã có vị trí riêng | Ghi vào |
|---|---|---|---|
| `CurrentPage` | (không hiện) | Bất kể | Vị trí chung của khối |
| `Repeat` | "Mọi trang đã chọn" | Chưa | Vị trí chung: mọi trang chưa có vị trí riêng cùng dịch theo |
| `Repeat` | "Mọi trang đã chọn" | Rồi | Vị trí riêng của trang đó |
| `Repeat` | "Chỉ trang đang kéo" | Bất kể | Vị trí riêng của trang đó (tạo mới nếu chưa có) |

- `editScope` là state chung của component, không lưu theo từng khối, mặc định "Mọi trang đã chọn" (`:86`).
- "Đồng bộ trang N" gọi `resetPagePosition(N)`; "Đồng bộ tất cả" gọi `resetPagePositions()` (`stamp-designer.html:217-244`).
- Backend dùng vị trí riêng nếu có, không thì vị trí chung (đối chiếu `StampPlanner.cs:99`), và từ chối khi một trang có hai vị trí riêng (`StampPlanner.cs:62-66`). `applyArea` luôn lọc bỏ phần tử cũ của trang trước khi thêm nên không sinh trùng.

### 7.9 Khối chữ ký số và vị trí riêng từng file (trong `sign-config-dialog.ts`)

Phần này không nằm trong `stamp-designer` nhưng dùng chung lớp phủ: dialog truyền template `#signatureLayer` (`sign-config-dialog.html:343-366`) qua input `pageExtra`, và `stamp-designer` vẽ nó vào trong `.stamp-layer` của từng trang (`stamp-designer.html:414-416`). Trong mục này, số dòng không ghi tên file là của `sign-config-dialog.ts`.

**Kích thước khối chữ ký do trình duyệt đo.** Chép nguyên văn (`:343-385`):

```ts
private fitSignature(position: ISignPosition | null): ISignPosition | null {
  if (!position) return null;
  const metric =
    this.pageMetrics().find((item) => item.number === position.pageNumber) ?? REFERENCE_PAGE;
  return { ...position, ...this.signatureSize(metric) };
}

private defaultSignatureBox(page: IPdfPageMetric): ISignatureBox {
  const size = this.signatureSize(page);
  const margin = SIGN_PAGE_MARGIN_PT * this.pageScale(page);
  return {
    xRatio: Math.max(1 - size.widthRatio - margin / page.widthPt, 0),
    yRatio: Math.min(margin / page.heightPt, 1 - size.heightRatio),
    ...size
  };
}

private signatureSize(page: IPdfPageMetric): { widthRatio: number; heightRatio: number } {
  const fontPt = this.pageFontPt(page);
  let textWidth = 0;
  if (this.measureContext) {
    this.measureContext.font = `${fontPt}px "${this.draft().fontFamily}"`;
    textWidth = Math.max(
      this.measureContext.measureText(this.signerLabel()).width,
      this.measureContext.measureText(this.previewTime).width
    );
  }
  return {
    widthRatio: Math.min(textWidth / page.widthPt, 1),
    heightRatio: Math.min((fontPt * SIGN_LINE_HEIGHT_RATIO * 2) / page.heightPt, 1)
  };
}

private pageFontPt(page: IPdfPageMetric): number {
  return this.draft().fontSize * this.pageScale(page);
}

private pageScale(page: IPdfPageMetric): number {
  return Math.min(
    page.widthPt / SIGN_REFERENCE_PAGE_WIDTH_PT,
    page.heightPt / SIGN_REFERENCE_PAGE_HEIGHT_PT
  );
}
```

- Khối gồm hai dòng: tên người ký (`certName` hoặc `'Người ký'`) và một chuỗi thời gian mẫu. Chuỗi mẫu tạo một lần khi component được dựng: `Utils.formatDateView(new Date(), 'YYYY-MM-DDTHH:mm:ss.SSSSSSS[+07:00]')` (`:90`).
- Cỡ chữ trên trang `= fontSize × min(rộng trang / 595, cao trang / 842)`.
- Bề rộng khối `=` bề rộng dòng dài hơn, đo bằng `canvas.measureText` với font đang chọn. Chiều cao `= cỡ chữ × 1,15 × 2`.
- `widthRatio`, `heightRatio` tính ra được lưu vào `signaturePosition` và gửi cho backend. Backend nhận hai số này làm kích thước khối khi chúng lớn hơn 0; không thì dùng tỉ lệ mặc định `170/595` và `30/842` (đối chiếu `SignService.cs:592-597`; phần dựng khối trong `PdfIncrementalSigner.cs` chưa đọc hết).
- Vị trí mặc định (`signaturePosition: null`): góc trên bên phải trang 1, cách mép `8 × pageScale` pt.
- Khi lưu, `fitSignature` tính lại kích thước cho cấu hình chung và cho **mọi** `fileOverrides` bằng số đo trang của **file đang xem** (`:282-295`); trang không tìm thấy thì dùng trang tham chiếu 595 × 842.

**Hiển thị và kéo khối chữ ký.** Chép nguyên văn (`:212-276`):

```ts
signatureBox(page: IPdfPage): ISignatureBox | null {
  if (!this.draft().showSignature) return null;

  const position = this.layout().signaturePosition;
  if (!position) return page.number === 1 ? this.defaultSignatureBox(page) : null;
  if (position.pageNumber !== page.number) return null;

  const size = this.signatureSize(page);
  const anchorRight = position.xRatio + position.widthRatio / 2 > 0.5;
  const left = anchorRight
    ? position.xRatio + position.widthRatio - size.widthRatio
    : position.xRatio;
  return {
    xRatio: Utils.clamp(left, 0, 1 - size.widthRatio),
    yRatio: Utils.clamp(position.yRatio, 0, 1 - size.heightRatio),
    ...size
  };
}

signatureFontPx(page: IPdfPage): number {
  return (this.pageFontPt(page) * page.cssWidth) / page.widthPt;
}

onSignaturePointerDown(event: PointerEvent, page: IPdfPage): void {
  const box = this.signatureBox(page);
  const host = (event.currentTarget as HTMLElement).closest('.stamp-layer') as HTMLElement | null;
  if (!box || !host) return;

  event.preventDefault();
  event.stopPropagation();
  this.signatureActive.set(true);

  const rect = host.getBoundingClientRect();
  this.drag = {
    host,
    page,
    grabX: (event.clientX - rect.left) / rect.width - box.xRatio,
    grabY: (event.clientY - rect.top) / rect.height - box.yRatio
  };
  (event.target as HTMLElement).setPointerCapture(event.pointerId);
}

onSignaturePointerMove(event: PointerEvent): void {
  const drag = this.drag;
  if (!drag) return;

  const rect = drag.host.getBoundingClientRect();
  const size = this.signatureSize(drag.page);
  const left = (event.clientX - rect.left) / rect.width - drag.grabX;
  const top = (event.clientY - rect.top) / rect.height - drag.grabY;
  this.updateLayout({
    signaturePosition: {
      pageNumber: drag.page.number,
      xRatio: Utils.clamp(left, 0, 1 - size.widthRatio),
      yRatio: Utils.clamp(top, 0, 1 - size.heightRatio),
      ...size
    }
  });
}

onSignaturePointerUp(event: PointerEvent): void {
  const target = event.target as HTMLElement;
  if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  this.drag = null;
}
```

- Khối chữ ký chỉ di chuyển được, không đổi cỡ được (cỡ đi theo cỡ chữ).
- **Neo phải**: nếu tâm khối đã lưu nằm ở nửa phải trang (`xRatio + widthRatio / 2 > 0.5`) thì khi bề rộng thay đổi, cạnh phải được giữ nguyên; ngược lại giữ cạnh trái. Backend làm đúng quy tắc này (đối chiếu `SignaturePlacementResolver.cs:38-41`).
- Khối chỉ kéo được trong trang nó đang nằm. Đổi trang bằng ô "Trang đặt khối" (`setSignaturePage`, `:191-206`). Nút "Mặc định" đặt `signaturePosition` về `null` (`:208-210`).
- Code tìm `.stamp-layer` bằng `closest`, tức phụ thuộc vào tên class trong template của `stamp-designer`.

**Vị trí riêng từng file.** Chép nguyên văn (`:108-117`, `:168-189`, `:317-341`):

```ts
ownLayout = computed<ISignLayout | null>(() => {
  const file = this.currentFile();
  if (!file) return null;
  return this.draft().fileOverrides.find((item) => item.path === file.fullPath) ?? null;
});
layout = computed<ISignLayout>(() => this.ownLayout() ?? this.draft());
ownLayoutCount = computed(() => {
  const paths = this.files().map((file) => file.fullPath);
  return this.draft().fileOverrides.filter((item) => paths.includes(item.path)).length;
});
```

```ts
toggleOwnLayout(enabled: boolean): void {
  const file = this.currentFile();
  if (!file) return;

  const others = this.draft().fileOverrides.filter((item) => item.path !== file.fullPath);
  if (!enabled) {
    this.patch({ fileOverrides: others });
    return;
  }

  const draft = this.draft();
  this.patch({
    fileOverrides: [
      ...others,
      {
        path: file.fullPath,
        signaturePosition: draft.signaturePosition ? { ...draft.signaturePosition } : null,
        stamps: structuredClone(draft.stamps)
      }
    ]
  });
}
```

```ts
private updateLayout(value: Partial<ISignLayout>): void {
  const own = this.ownLayout();
  const file = this.currentFile();
  if (!own || !file) {
    this.patch(value);
    return;
  }

  this.patch({
    fileOverrides: this.draft().fileOverrides.map((item) =>
      item.path === file.fullPath ? { ...item, ...value } : item
    )
  });
}

private removeStampsOf(kind: EMergeStampKind): void {
  this.draft.update((draft) => ({
    ...draft,
    stamps: draft.stamps.filter((stamp) => stamp.kind !== kind),
    fileOverrides: draft.fileOverrides.map((item) => ({
      ...item,
      stamps: item.stamps.filter((stamp) => stamp.kind !== kind)
    }))
  }));
}
```

- Một file có "vị trí riêng" khi `fileOverrides` có phần tử với `path` bằng `fullPath` của file. Khoá so sánh là chuỗi đường dẫn nguyên văn do backend trả.
- Bật công tắc "Vị trí riêng file này": chép vị trí chữ ký và danh sách khối của cấu hình chung sang phần tử mới. Tắt: xoá phần tử.
- Mọi thay đổi vị trí chữ ký hoặc khối ảnh ghi vào phần tử riêng nếu file đang xem có, không thì ghi vào cấu hình chung (`updateLayout`).
- Phần "riêng" chỉ gồm `signaturePosition` và `stamps`. Font, màu, ảnh, độ đậm luôn dùng chung.

**Xem trước độ đậm.** Dialog chỉ mô phỏng độ đậm bằng CSS filter (`:387-394`):

```ts
private darknessFilter(darkness: number): string {
  const factor = darkness / 100;
  const brightness = Math.max(
    SIGN_IMAGE_DARKNESS_MIN_BRIGHTNESS,
    1 - (factor - 1) * SIGN_IMAGE_DARKNESS_BRIGHTNESS_FACTOR
  );
  return `contrast(${factor}) brightness(${brightness})`;
}
```

Độ dày nét (`wetSignatureStroke`) và màu mực (`wetSignatureColor`) không có mô phỏng nào trên giao diện.

### 7.10 Việc cần sửa khi port `stamp-designer`

| Chỗ phụ thuộc | Vị trí | Thay bằng |
|---|---|---|
| `DocumentMergeService.imageUrl(path)` để hiện ảnh và đo tỉ lệ | `:149-152`, `:241` | Hàm đổi đường dẫn thành URL của Tauri (`convertFileSrc`), truyền vào qua một service nhỏ |
| `FilePickerService.canBrowse`, `browse('image')` | `:162-169`; `stamp-designer.html:93-103` | Hộp thoại của `tauri-plugin-dialog`. Bỏ ô "Dán đường dẫn (web)" |
| `FilePickerService.upload(file)` khi thả file ảnh vào ô | `:181-203` | `onDragDropEvent` cho đường dẫn thật; bỏ bước tải lên |
| Kéo thả HTML5 ảnh mẫu vào trang | `:445-489` | Pointer events (mục 7.6) |
| `BaseComponent`: `messageWarning`, `messageError`, `confirmDelete`, `isResponseSucceed` | `:164, 188, 196, 200, 226, 238, 423` | Service thông báo và xác nhận mới |
| PrimeNG | `p-toggleswitch` ×1, `p-button` ×4, `p-selectbutton` ×1, `pTooltip` ×6, `pInputText` ×2; `SharedImports`, `SelectButtonModule`, `ToggleSwitchModule` (`:2-5, 48`) | Phần tử HTML + CSS của prototype |
| Class Tailwind | Toàn bộ `stamp-designer.html`; `host: { class: 'flex min-h-0 flex-1 gap-4' }` (`:51`) | CSS của prototype |
| Biến `--layout-accent`, `--layout-border` | `stamp-designer.scss:1-2` | Token màu của prototype |

Phần không cần sửa: toàn bộ các hàm chép ở mục 7.4, 7.7, 7.8 (trừ `loadImageAspect` phải đổi nguồn URL). Nên tách chúng ra một file thuần TypeScript không phụ thuộc Angular để viết unit test, và dùng chung bộ dữ liệu test với `smks-core::stamp_plan` cho `resolvePages` và `parseCustomPages`.

---

## 8. `pdf-pages`

File: `src/app/shared/components/pdf-pages/pdf-pages.ts` (187 dòng), `.html` (26 dòng), `.scss` (51 dòng). Số dòng không ghi tên file là của `pdf-pages.ts`.

### 8.1 Giao diện component

| Tên | Loại | Mặc định | Ý nghĩa |
|---|---|---|---|
| `url` | `input<string \| null>` | `null` | URL file PDF |
| `busy` | `input<boolean>` | `false` | Bên ngoài đang bận, hiện vòng quay |
| `emptyText` | `input<string>` | `'Chưa có file để xem trước.'` | |
| `loaded` | `output<IPdfPageMetric[]>` | | Phát sau khi đo xong mọi trang |
| `failed` | `output<void>` | | Phát khi không mở được file |
| `#pageOverlay` | `contentChild<TemplateRef<{ $implicit: IPdfPage }>>` | | Template phủ lên mỗi trang, nhận `IPdfPage` |

Template (`pdf-pages.html`): mỗi trang là một `div.pdf-pages__page` có kích thước px cố định, chứa `<canvas data-page="N">`, lớp phủ, và nhãn "Trang N/Tổng". Component không có nút phóng to, thu nhỏ, không có chuyển trang, không có chọn chữ.

### 8.2 Phiên bản và worker

- `pdfjs-dist` `^6.4.299`, bản đang cài 6.4.299.
- `import * as pdfjs from 'pdfjs-dist'` (`:18`).
- Worker: `pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdf.worker.min.mjs', document.baseURI).toString();` (`:23`), file do `angular.json` chép từ `node_modules/pdfjs-dist/build` (mục 1.2).

### 8.3 Nạp tài liệu

Chép nguyên văn (`:131-166`):

```ts
private async open(url: string | null): Promise<void> {
  const version = ++this.loadVersion;
  this.pageObserver?.disconnect();
  this.rendered.clear();
  this.metrics.set([]);
  this.doc = null;
  void this.task?.destroy();
  this.task = null;

  if (!url) {
    this.loading.set(false);
    return;
  }

  this.loading.set(true);
  try {
    const task = pdfjs.getDocument({ url });
    this.task = task;
    const doc = await task.promise;

    const metrics: IPdfPageMetric[] = [];
    for (let number = 1; number <= doc.numPages; number++) {
      const viewport = (await doc.getPage(number)).getViewport({ scale: 1 });
      metrics.push({ number, widthPt: viewport.width, heightPt: viewport.height });
    }

    if (version !== this.loadVersion) return;
    this.doc = doc;
    this.metrics.set(metrics);
    this.loaded.emit(metrics);
  } catch {
    if (version === this.loadVersion) this.failed.emit();
  } finally {
    if (version === this.loadVersion) this.loading.set(false);
  }
}
```

- Được gọi mỗi khi `url` đổi (`urlEffect`, `:81-84`).
- `getDocument` chỉ nhận `{ url }`. pdf.js tự tải file qua HTTP.
- Lấy `getPage` cho **mọi trang, tuần tự** để đo kích thước trước khi hiện bất cứ thứ gì. File vài trăm trang sẽ chờ lâu.
- `widthPt`, `heightPt` là kích thước viewport ở `scale: 1`, đã áp xoay trang (mục 7.3).

### 8.4 Kích thước hiển thị, render, scale

Bề rộng trang trên màn hình (`:86-102`): một `ResizeObserver` theo dõi vùng chứa; `pageWidth = clamp(clientWidth − 40, 320, 1100)` px. Mọi trang cùng bề rộng; chiều cao theo tỉ lệ từng trang (`:53-60`). Khi vùng chứa đổi cỡ, kích thước CSS đổi ngay (canvas bị kéo giãn), và sau 250 ms không đổi nữa thì xoá danh sách "đã render" để vẽ lại.

Render lười (`:104-129`): một `IntersectionObserver` với `rootMargin: '300px'` theo dõi mọi canvas; canvas nào giao vùng nhìn thì gọi `renderPage`.

Chép nguyên văn (`:168-186`):

```ts
private async renderPage(pageNumber: number): Promise<void> {
  const doc = this.doc;
  const meta = this.pages().find((page) => page.number === pageNumber);
  const canvas = this.canvases().find(
    (ref) => Number(ref.nativeElement.dataset['page']) === pageNumber
  )?.nativeElement;
  if (!doc || !meta || !canvas || this.rendered.has(pageNumber)) return;

  this.rendered.add(pageNumber);
  const page = await doc.getPage(pageNumber);
  const viewport = page.getViewport({ scale: (meta.cssWidth * RENDER_SCALE) / meta.widthPt });
  canvas.width = viewport.width;
  canvas.height = viewport.height;

  const context = canvas.getContext('2d');
  if (context) {
    await page.render({ canvas, canvasContext: context, viewport }).promise;
  }
}
```

- Hệ số vẽ: `scale = cssWidth × RENDER_SCALE / widthPt`, với `RENDER_SCALE = max(devicePixelRatio, 2)`. Canvas luôn có ít nhất gấp đôi số pixel so với kích thước CSS.
- Ví dụ trang A4 (595 pt) ở bề rộng tối đa 1100 px, `RENDER_SCALE = 2`: `scale ≈ 3,70`, tương đương khoảng 266 DPI; canvas 2200 × 3113 px, khoảng 27 MB bộ nhớ ảnh cho một trang.
- Ở bề rộng tối thiểu 320 px: `scale ≈ 1,08`, khoảng 77 DPI.

### 8.5 Cache và huỷ

| Việc | Hiện trạng |
|---|---|
| Cache tài liệu | Không. Đổi `url` là tải và đo lại từ đầu |
| Cache trang đã vẽ | Chỉ có `Set<number> rendered` để không vẽ lại. Canvas đã vẽ không bao giờ được giải phóng, kể cả khi cuộn ra xa |
| Cache URL xem trước | Nằm ngoài component: `merge-stamp-step.ts:42` (theo dòng dữ liệu, chỉ chế độ `Edit`) |
| Huỷ nạp tài liệu | Có: `task.destroy()` khi mở URL mới (`:137`) và khi component bị huỷ (`:77`); bộ đếm `loadVersion` loại kết quả cũ |
| Huỷ render | **Không**. `RenderTask` không được giữ lại, không gọi `cancel()` |
| Lỗi render | Không bắt. `renderPage` được gọi bằng `void` (`:115`) nên lỗi thành promise bị từ chối không ai xử lý |

### 8.6 Những thứ của pdf.js chưa được cấu hình

`getDocument` không truyền `wasmUrl`, `cMapUrl`, `standardFontDataUrl`, `iccUrl`, và `angular.json` không chép các thư mục `wasm/` (`openjpeg.wasm`, `jbig2.wasm`, `qcms_bg.wasm`), `cmaps/` (169 file), `standard_fonts/` (16 file), `iccs/` của `pdfjs-dist`. Hệ quả có thể có: PDF scan nén JPEG 2000 hoặc JBIG2 hiện thiếu ảnh; PDF không nhúng font chuẩn hoặc dùng CMap CJK hiện sai chữ. Chưa chắc, cần thử với bộ `fixtures/` (đặc biệt là văn bản scan).

### 8.7 Cách lấy URL file

Bản cũ đi qua ba API `GET`, tham số là đường dẫn tuyệt đối đã `encodeURIComponent`:

| Dùng cho | Hàm | URL | Nơi gọi |
|---|---|---|---|
| PDF cần ký | `SignService.pdfFileUrl`, `sign.service.ts:54-56` | `${base}/core/document/sign/xem-file?path=…` | `sign-config-dialog.ts:103-106` |
| PDF xem trước của Tạo văn bản | `DocumentMergeService.previewFileUrl`, `document-merge.service.ts:39-41` | `${base}/core/document/tao-van-ban/xem-file?path=…` | `merge-stamp-step.ts:103` |
| Ảnh dấu, chữ ký (cả hai luồng) | `DocumentMergeService.imageUrl`, `document-merge.service.ts:43-45` | `${base}/core/document/tao-van-ban/xem-anh?path=…` | `stamp-designer.ts:151, 241` |

`base = apiBase + '/api/v1'`. `apiBase` là chuỗi rỗng khi chạy thật (cùng origin với máy chủ nội bộ) và `http://localhost:5247` ở dev mode (`desktop.service.ts:23-25`).

Bản mới (kế hoạch mục 10.3) dùng `convertFileSrc(path)` và asset protocol. Những điểm cần thử ở spike, đều chưa chắc:

1. pdf.js tải bằng `fetch` từ origin của ứng dụng sang origin của asset protocol. Cần CSP cho phép (`connect-src` cho PDF, `img-src` cho ảnh, `worker-src` cho worker) và response có header CORS phù hợp. Nếu không được, phương án dự phòng là đọc byte qua `invoke` rồi gọi `getDocument({ data })`, tốn bộ nhớ hơn với file lớn.
2. Đường dẫn có ký tự tiếng Việt, khoảng trắng, `#`, `%` sau khi `convertFileSrc` mã hoá.
3. Bản xem trước tạo lại ở cùng một đường dẫn có thể bị webview trả bản cũ từ cache; thêm tham số phiên bản vào URL nếu cần.
4. File `pdf.worker.min.mjs` phải được phục vụ với kiểu MIME JavaScript thì module worker mới chạy.

### 8.8 Việc cần sửa khi port `pdf-pages`

- Logic giữ nguyên được. Không phụ thuộc PrimeNG module nào; chỉ có 2 class icon `pi pi-file-pdf`, `pi pi-spin pi-spinner` và vài class Tailwind trong `pdf-pages.html`.
- Nên sửa luôn các điểm ở mục 11 (lỗi số 17 đến 21): giữ `RenderTask` để huỷ, bắt lỗi render, giải phóng canvas ngoài vùng nhìn, đặt `root` cho `IntersectionObserver`, chép thêm `wasm/`, `cmaps/`, `standard_fonts/`.
- Kế hoạch có màn Kiểm tra chữ ký và thanh zoom (`ui.zoom` trong `config.json`). Component hiện không có khái niệm zoom do người dùng chọn; bề rộng trang luôn bám theo vùng chứa.

---

## 9. `utils.ts`, `sse-stream.ts`, `base-component.ts`, `desktop.service.ts`, `file-picker.service.ts`

### 9.1 `src/app/shared/utils.ts`

Lớp `Utils` có 23 hàm tĩnh. Chỉ 6 hàm được gọi từ bên ngoài (đếm bằng tìm kiếm trong `src/`).

| Hàm | Dòng | Việc làm | Số nơi gọi | Bản mới |
|---|---|---|---|---|
| `getLocalStorage(key)` | 4-10 | `JSON.parse(localStorage.getItem(key) \|\| '')`, lỗi thì trả `{}` | 1 (`sign-config-state.service.ts:11`) | Thay bằng lệnh `config_get` |
| `setLocalStorage(key, data)` | 12-14 | `localStorage.setItem(key, JSON.stringify(data))` | 1 (`sign-config-state.service.ts:16`) | Thay bằng lệnh `config_save` |
| `clearLocalStorage()` | 16-18 | `localStorage.clear()` | 0 | Bỏ |
| `setSessionStorage`, `getSessionStorage`, `clearSessionStorage`, `removeSessionStorage` | 20-34 | Bọc `sessionStorage` | 0 | Bỏ |
| `refreshData(data)` | 36-38 | Sao chép sâu bằng JSON | 0 | Bỏ (đã có `structuredClone`) |
| `convertLowerCase(string)` | 40-45 | Hạ chữ cái đầu | 0 | Bỏ |
| `reverseDateString(date)` | 47-49 | Đảo thứ tự các phần ngày | 0 | Bỏ |
| `replaceAll(str, find, replace)` | 51-54 | Thay mọi chỗ khớp (có escape regex) | Chỉ nội bộ (`transformPercent`) | Bỏ |
| `transformMoney`, `transformPercent` | 56-88 | Định dạng số theo `vi-VN` | 0 | Bỏ |
| `log(titleError, error)` | 90-92 | `console.log` có màu | 0 | Bỏ |
| `convertParamUrl(name, value)` | 94-96 | Ghép tham số URL | 0 | Bỏ |
| `makeRandom(lengthOfCode, possible)` | 98-105 | Chuỗi ngẫu nhiên. Tham số `possible` bị ghi đè ngay dòng đầu | 0 | Bỏ |
| `convertVietnameseToEng(str, isKeepCase)` | 107-125 | Bỏ dấu tiếng Việt. `isKeepCase` không dùng | 0 | Bỏ (nếu cần đặt tên file thì việc này ở lõi Rust) |
| `formatDateCallApi(date, format)` | 127-132 | Định dạng ngày bằng `moment` | 0 | Bỏ |
| `formatDateView(date, format)` | 134-139 | Định dạng ngày bằng `moment`, mặc định `DD/MM/YYYY` | 1 (`sign-config-dialog.ts:90`) | Viết lại vài dòng không cần `moment`, hoặc bỏ nếu khối chữ ký mới không cần chuỗi mẫu này |
| `base64UrlEncode(arrayBuffer)` | 141-144 | Base64 URL-safe | 0 | Bỏ |
| `compareDocSeq(a, b)` | 146-148 | So sánh tên kiểu tự nhiên: `localeCompare(…, undefined, { numeric: true, sensitivity: 'base' })` | 1 (`sign-step.ts:75`) | **Port thẳng** |
| `clamp(value, min, max)` | 150-152 | `Math.min(Math.max(value, min), max)` | 12 (`stamp-designer.ts`, `sign-config-dialog.ts`) | **Port thẳng** |
| `normalizeFolderPath(path)` | 154-162 | Chuẩn hoá đường dẫn (xem dưới) | 17 | **Port có sửa** |

`normalizeFolderPath` nguyên văn:

```ts
static normalizeFolderPath(path: string): string {
    if (!path) return '';
    let p = path.trim();
    if ((p.startsWith('"') && p.endsWith('"')) || (p.startsWith("'") && p.endsWith("'"))) {
        p = p.slice(1, -1).trim();
    }
    p = p.replace(/\\/g, '/').replace(/\/{2,}/g, '/');
    return p.replace(/\/+$/, '');
}
```

Các bước: cắt khoảng trắng; bỏ cặp nháy bao ngoài (trường hợp dán đường dẫn chép từ Explorer); đổi `\` thành `/`; gộp nhiều `/` liền nhau thành một; bỏ `/` ở cuối.

| Đầu vào | Kết quả | Nhận xét |
|---|---|---|
| `"D:\HoSo\a.pdf"` (có nháy) | `D:/HoSo/a.pdf` | Đúng ý |
| `D:\HoSo\` | `D:/HoSo` | Đúng ý |
| `\\maychu\chiase\a.pdf` | `/maychu/chiase/a.pdf` | **Hỏng đường dẫn UNC** |
| `D:\` | `D:` | Mất dấu gốc ổ đĩa; trên Windows `D:` là "thư mục hiện hành của ổ D" |

Bản mới cần quyết định một dạng đường dẫn chuẩn dùng chung cho giao diện, lõi và `config.json`. Hộp thoại và sự kiện kéo thả của Tauri trả đường dẫn Windows dùng `\`. Hàm này còn được dùng để lấy tên file bằng `split('/').pop()` (`pdf-source-picker.ts:28-30`, `path-picker.ts:34`, `tao-van-ban.ts:62-64`), nên đổi dạng chuẩn phải sửa cả các chỗ đó.

### 9.2 `src/app/shared/sse-stream.ts`

Một hàm `streamSse<T>(url, body): Observable<T>` (dòng 3-58):

1. Tạo `AbortController`.
2. `fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body), signal })`.
3. Nếu `!res.ok || !res.body`: `subscriber.error(new Error('HTTP ' + status))`.
4. Nếu `Content-Type` của response chứa `application/json`: `subscriber.error(await res.json())`. Đây là đường backend dùng để từ chối request trước khi bắt đầu luồng (đối chiếu `SignController.cs:128-136`).
5. Ngược lại đọc luồng bằng `res.body.getReader()` và `TextDecoder`; cắt theo `'\n\n'`; đoạn nào bắt đầu bằng `data:` thì `JSON.parse` phần sau và `subscriber.next`.
6. Hết luồng: `subscriber.complete()`.
7. Lỗi: nếu không phải do chủ động huỷ thì `subscriber.error(err)`.
8. Hàm dọn dẹp của Observable: `controller.abort()`.

Chỉ `DocumentMergeService.merge` dùng hàm này (`document-merge.service.ts:47-49`). `SignService` có một bản sao y hệt làm phương thức riêng (`sign.service.ts:72-127`).

**Bản mới: thay toàn bộ** bằng `Channel` của Tauri. Bốn khác biệt về hành vi phải giữ đúng khi thay:

| Hành vi cũ | Tương đương mới |
|---|---|
| `unsubscribe()` huỷ request, backend dừng lô | Hàm dọn dẹp phải gọi `job_cancel(job_id)` |
| Luồng đóng thì `complete` | Tự `complete` khi nhận sự kiện `summary` |
| JSON lỗi trả ngay từ đầu thành `error` | `invoke` bị từ chối (lệnh trả `Err`) thành `error` |
| Lỗi mạng thành `error` với câu chung | Không còn lỗi mạng; còn lỗi IPC |

Phác thảo lớp bọc (chưa kiểm chứng, chỉ để chỉ ra chỗ cần chú ý):

```ts
import { Channel, invoke } from '@tauri-apps/api/core';
import { Observable } from 'rxjs';

export function runJob<E extends { type: string }>(command: string, args: Record<string, unknown>): Observable<E> {
  return new Observable<E>((subscriber) => {
    const channel = new Channel<E>();
    let jobId: string | null = null;
    let finished = false;
    let cancelled = false;

    channel.onmessage = (event) => {
      subscriber.next(event);
      if (event.type === 'summary') {
        finished = true;
        subscriber.complete();
      }
    };

    invoke<string>(command, { ...args, channel }).then(
      (id) => {
        jobId = id;
        if (cancelled && !finished) void invoke('job_cancel', { jobId: id });
      },
      (error) => subscriber.error(error)
    );

    return () => {
      cancelled = true;
      if (!finished && jobId) void invoke('job_cancel', { jobId });
    };
  });
}
```

Điểm cần chú ý: người dùng có thể huỷ trước khi `invoke` trả `job_id`, nên phải nhớ ý định huỷ và gửi `job_cancel` ngay khi có id.

### 9.3 `src/app/shared/components/base/base-component.ts`

Lớp trừu tượng `BaseComponent` (`@Directive()`), là lớp cha của `KySo`, `SignStep`, `SignConfigDialog`, `SignResultDialog`, `TaoVanBan`, `MergeMappingStep`, `MergeStampStep`, `MergeRunStep`, `StampDesigner`, `PdfSourcePicker`, `PathPicker`. Nó inject `Router`, `ActivatedRoute`, và ba service của PrimeNG: `MessageService`, `DialogService`, `ConfirmationService` (dòng 12-16).

| Thành phần | Dòng | Việc làm | Bản mới |
|---|---|---|---|
| Constructor | 24-42 | Đăng ký `router.events`, lọc `NavigationEnd`; nếu URL hiện tại bắt đầu bằng đường dẫn của route chứa component thì gọi `onRouteActivated()`. Tự huỷ đăng ký bằng `takeUntilDestroyed()` | Bỏ. Nơi duy nhất cần là `KySo` để nhận bàn giao (mục 6.6) |
| `ngOnInit()`, `onRouteActivated()` | 44-46 | Rỗng, để lớp con ghi đè | Bỏ |
| `isFormInvalid()`, `getErrorMessage()`, `getError()` | 48-73 | Hỗ trợ Reactive Forms | Bỏ (không nơi nào dùng) |
| Trường `ValidationMessages`, `MAX_PAGE_SIZE`, `START_PAGE_NUMBER`, `form`, `loading` | 18-22 | Di sản của mẫu CRUD | Bỏ (không dùng) |
| `isResponseSucceed(res, isShowErrorMsg = true, successMsg = '')` | 75-86 | `res.status === 1` là thành công; có `successMsg` thì hiện toast xanh; thất bại thì hiện `res.message` hoặc "Có sự cố xảy ra. Vui lòng thử lại sau." | Bỏ. Lệnh Tauri trả kết quả hoặc ném lỗi, không còn vỏ `{ status, code, message, data }` |
| `messageSuccess(msg)`, `messageWarning(msg)`, `messageError(msg)` | 88-113 | `MessageService.add` với `severity` tương ứng, `closable: true`, `life: 4000`, chỉ có `detail` | Thay bằng service thông báo mới (toast theo prototype) |
| `confirmDelete(opt, acceptCallback)` | 115-133 | Hộp xác nhận, nút "Thoát" và "Xóa" (đỏ), icon cảnh báo | Thay bằng service xác nhận mới. Dùng ở `stamp-designer.ts:423` |
| `confirmAction(opt, acceptCallback)` | 135-152 | Hộp xác nhận, nút "Thoát" và "Tiếp tục" | Thay. Dùng ở `merge-mapping-step.ts:68`, `merge-run-step.ts:127` |

Nên thay kế thừa bằng inject trực tiếp hai service nhỏ (thông báo, xác nhận). Component nào cần thì inject, không cần lớp cha chung.

### 9.4 `src/app/services/desktop.service.ts`

Cầu nối với vỏ Photino qua `window.external`. Giao thức là chuỗi văn bản (đối chiếu `ktl.be/ktl.be.desktop/Program.cs:76-121`):

| Hàm | Dòng | Gửi đi | Nhận về | Bản mới |
|---|---|---|---|---|
| `isDesktop` (getter) | 19-21 | | `true` khi có `window.external.sendMessage` | Bỏ, hoặc đổi thành kiểm tra đang chạy trong Tauri để còn chạy thử trên trình duyệt |
| `apiBase` (getter) | 23-25 | | Dev mode: `environment.baseUrl`; còn lại `''` | Bỏ |
| `pickFolder()` | 27-33 | `pickFolder` | `folder:<đường dẫn>` (rỗng nếu huỷ) | `open({ directory: true })` của `@tauri-apps/plugin-dialog` |
| `pickImage()` | 35-41 | `pickImage` | `image:<đường dẫn>` | `open({ filters: [{ name: 'Ảnh', extensions: ['png', 'jpg', 'jpeg'] }] })` |
| `pickWord()` | 43-49 | `pickWord` | `word:<đường dẫn>` | `open` với `extensions: ['docx']` |
| `pickExcel()` | 51-57 | `pickExcel` | `excel:<đường dẫn>` | `open` với `extensions: ['xlsx']` |
| `pickPdfFiles()` | 59-65 | `pickPdf` | `pdf:<p1>\|<p2>\|…` | `open({ multiple: true, filters: [{ name: 'PDF', extensions: ['pdf'] }] })` |
| `openPath(path)` | 67-71 | `openPath:<đường dẫn>` | Không có phản hồi; hàm trả `true` ngay nếu đang ở desktop và `path` không rỗng | Lệnh `shell_open(path)` theo kế hoạch mục 10.1; nên trả lỗi khi đường dẫn không tồn tại |
| `onMessage(message)` | 73-96 | | Phân loại theo tiền tố, gọi hàm `resolve` đang chờ | Bỏ |

Đặc điểm cần biết:

- Mỗi loại hộp thoại chỉ có **một** biến giữ `resolve` đang chờ (dòng 7-11). Gọi hai lần liên tiếp cùng loại thì Promise đầu không bao giờ kết thúc. Không có timeout.
- Người dùng huỷ hộp thoại: vỏ desktop gửi tiền tố kèm chuỗi rỗng, Promise trả `''` hoặc `[]`.
- Khi không chạy trong vỏ desktop, mọi hàm chọn trả `''` hoặc `[]` ngay; `openPath` trả `false` và nơi gọi chuyển sang chép đường dẫn vào clipboard (`sign-step.ts:253-260`, `merge-run-step.ts:143-150`).
- Tiêu đề và bộ lọc hộp thoại nằm ở phía vỏ desktop: "Chọn thư mục", "Chọn ảnh" (`png`, `jpg`, `jpeg`), "Chọn file template Word" (`docx`), "Chọn file PDF" (`pdf`, nhiều file), "Chọn file dữ liệu Excel" (`xlsx`).

### 9.5 `src/app/services/file-picker.service.ts`

| Hàm | Dòng | Việc làm | Bản mới |
|---|---|---|---|
| `canBrowse` (getter) | 15-17 | `desktop.isDesktop` | Luôn `true` trong Tauri. Bỏ các nhánh "chạy trên web" ở `path-picker.ts:41-44`, `pdf-source-picker.ts:117-121`, `stamp-designer.ts:163-166` và ba ô dán đường dẫn trong template |
| `extensions(kind)` | 19-21 | Trả `PATH_PICKER_EXTENSIONS[kind]` | **Port thẳng** |
| `accepts(kind, fileName)` | 23-26 | Lấy đuôi file từ dấu chấm cuối, hạ chữ thường, so với danh sách | **Port thẳng**. Vẫn cần để lọc đường dẫn nhận từ kéo thả |
| `browse(kind)` | 28-32 | Mở hộp thoại theo loại, chuẩn hoá đường dẫn | Giữ chữ ký hàm, bên trong gọi `tauri-plugin-dialog` |
| `browsePdfFiles()` | 34-38 | Chọn nhiều file PDF | Như trên |
| `upload(file)` | 40-42 | Chuyển tiếp sang `FileUploadService.upload` | **Bỏ** |
| `openDialog(kind)` (private) | 44-57 | Ánh xạ loại sang hàm của `DesktopService` | Thay bằng bảng bộ lọc cho `open()` |

### 9.6 Các service còn lại

**`file-upload.service.ts`** (bỏ): `upload(file)` gửi `POST ${apiBase}/api/v1/core/document/tai-len` dạng multipart, trường `file`; trả `{ fileName, filePath, sizeBytes }` với `filePath` nằm trong thư mục tạm. Lý do tồn tại: webview không cho biết đường dẫn thật của file kéo thả. Tauri cho đường dẫn thật nên cả service và mọi nhánh `uploading` bỏ được.

**`sign.service.ts` và `document-merge.service.ts`** (viết lại phần gọi): nên giữ tên và chữ ký các hàm để component ít phải sửa. Bảng ánh xạ, cột cuối theo kế hoạch mục 10.1:

| Hàm cũ | API cũ (tiền tố `/api/v1/core/document`) | Lệnh mới |
|---|---|---|
| `SignService.getCertificates(onlySignable)` | `GET /sign/chung-thu-so?onlySignable=` | `cert_list(only_signable)` |
| `SignService.verifyToken(thumbprint)` | `POST /sign/kiem-tra-token` | `token_check(thumbprint)` |
| `SignService.getPreview(sourcePath, index)` | `GET /sign/xem-truoc?sourcePath=&index=` | Bỏ (không ai gọi) |
| `SignService.checkPositions(request)` | `POST /sign/kiem-tra-vi-tri` | `sign_check_positions(req)` |
| `SignService.pdfFileUrl(fullPath)` | `GET /sign/xem-file?path=` | `convertFileSrc(path)` |
| `SignService.getFiles(sourcePaths)` | `POST /sign/danh-sach-file` | `sign_list_files(sources)` |
| `SignService.signFolder(req)` | `POST /sign/ky-thu-muc` (SSE) | `sign_batch(req, channel)` |
| `DocumentMergeService.autoMap(templatePath, dataPath)` | `GET /tao-van-ban/map-tu-dong?templatePath=&dataPath=` | `merge_auto_map(template, table)` |
| `DocumentMergeService.validate(request)` | `POST /tao-van-ban/kiem-tra` | `merge_validate(req)` |
| `DocumentMergeService.preview(request)` | `POST /tao-van-ban/xem-truoc` | `merge_preview(req, row)` |
| `DocumentMergeService.previewFileUrl(filePath)` | `GET /tao-van-ban/xem-file?path=` | `convertFileSrc(path)` |
| `DocumentMergeService.imageUrl(path)` | `GET /tao-van-ban/xem-anh?path=` | `convertFileSrc(path)` |
| `DocumentMergeService.merge(request)` | `POST /tao-van-ban/sinh-file` (SSE) | `merge_run(req, channel)` |
| `FileUploadService.upload(file)` | `POST /tai-len` | Bỏ |

Frontend cũ **không gọi** `tao-van-ban/danh-sach-header` và `sign/kiem-tra-chu-ky` (hai dòng có trong bảng của kế hoạch). Giao diện cho `merge_read_table`, `merge_read_template`, `verify_file` phải viết mới.

Mọi response cũ có vỏ `IBaseResponseWithData<T>`; các hàm trả `Observable`. Bản mới có thể giữ `Observable` bằng `from(invoke(…))` để component sửa ít, hoặc chuyển hẳn sang `Promise`.

**`sign-config-state.service.ts`** (sửa): hiện đọc đồng bộ ngay trong lúc khởi tạo trường. `config_get` là bất đồng bộ, nên cần nạp cấu hình trước khi dựng giao diện (ví dụ bằng `provideAppInitializer`) hoặc cho signal bắt đầu bằng giá trị mặc định rồi cập nhật.

**`sign-cert-state.service.ts`**, **`sign-handoff.service.ts`**, **`app-route-reuse.strategy.ts`**: dùng lại nguyên. Không phụ thuộc HTTP hay PrimeNG.

**`layout/service/layout.service.ts`** (sửa): hai việc độc lập.

- `applyZoom` (dòng 45-50): `zoom = min(1, max(innerWidth / 1440, 0.6))`, đặt `zoom` lên `document.documentElement` và biến `--app-vh = innerHeight / zoom`. Có giữ hay không tuỳ bố cục của prototype.
- `blockBrowserBehaviors` (dòng 16-43), chỉ chạy khi không ở dev mode: chặn menu chuột phải ngoài ô nhập, chặn Ctrl+lăn chuột, chặn `F5`, Ctrl/⌘ + `r p u f g j + - = 0`, Alt + ←/→. Webview của Tauri cũng là WebView2 nên các phím tắt này vẫn cần xét; một số có thể tắt bằng cấu hình cửa sổ thay vì bằng code (chưa chắc phím nào).

---

## 10. Bảng tái sử dụng

### 10.1 Theo file

"Nguyên" là chép sang dùng được ngay. "Sửa" là giữ phần lớn logic, cột ghi chú nêu chỗ phải sửa. "Bỏ" là không mang sang. Template HTML của mọi trang đều phải viết lại vì dùng PrimeNG và Tailwind; bảng chỉ tách riêng những template đáng nói.

| File | Kết luận | Ghi chú |
|---|---|---|
| `src/main.ts`, `src/app.ts` | Nguyên | |
| `src/index.html` | Sửa | Giữ `lang`, `title`; kiểm tra `<base href>` với cách Tauri phục vụ file |
| `src/app.config.ts` | Sửa | Bỏ `provideHttpClient`, `provideAnimationsAsync`, `providePrimeNG`, `MessageService`, `ConfirmationService`, `DialogService`. Giữ router, `RouteReuseStrategy`, `provideBrowserGlobalErrorListeners` |
| `src/app.routes.ts` | Sửa | Thêm route cho các màn mới |
| `src/styles.css` | Bỏ | Thay bằng CSS của prototype. Ý tưởng đáng giữ: `user-select: none` toàn trang trừ ô nhập (dòng 9-28), chặn kéo ảnh và liên kết (dòng 30-33) |
| `src/environments/*` | Bỏ | Không còn `baseUrl` |
| `app/layout/*` (trừ service) | Bỏ | Khung mới theo prototype và quyết định D9 về thanh tiêu đề |
| `app/layout/service/layout.service.ts` | Sửa | Mục 9.6 |
| `app/models/sign.models.ts` | Sửa | Giữ: `ISignCert`, `ITokenVerify`, `ISignFileItem`, `ISignFileList`, `ISignPosition`, `ISignLayout`, `ISignFileConfig`, `ISignPositionCheck*`, `ISignatureDetail`. Sửa theo `JobEvent`: `ISignProgress`, `ISignFileResult`, `ISignResult`, `ISignFileRow`. Sửa theo `config.json`: `ISignConfig`, `ISignRequest`. Bỏ: `ISignPreview` |
| `app/models/document-merge.models.ts` | Sửa | Giữ gần hết. Sửa theo `JobEvent`: `IMergeProgress`, `IMergeFileResult`, `IMergeResult` |
| `app/models/file-upload.models.ts` | Bỏ | |
| `app/shared/models/pdf-pages.models.ts` | Nguyên | |
| `app/shared/models/request-paging.base.models.ts` | Bỏ | Thay bằng kiểu lỗi của IPC |
| `app/shared/models/environment.models.ts`, `data-table.models.ts` | Bỏ | |
| `app/shared/constants/document-merge.constants.ts` | Nguyên | Chỉ sửa hai chuỗi icon `pi pi-arrows-alt`, `pi pi-eye` (dòng 72-73). Thêm đuôi file nếu bản mới nhận CSV, XLS, mẫu HTML |
| `app/shared/constants/sign.constants.ts` | Sửa | Bỏ `SIGN_CONFIG_STORAGE_KEY`. `SIGN_DEFAULT_CONFIG` theo cấu hình mới. Bỏ hoặc dùng thật `SIGN_STATUS_LABELS`. Các hằng số khác giữ, và phải trùng với `smks-core::consts` |
| `app/shared/constants/path-picker.constants.ts` | Nguyên | `PATH_PICKER_DROPPED_LABEL` hết tác dụng khi không còn tải file lên |
| `app/shared/constants/api.constants.ts`, `data-table.constants.ts` | Bỏ | |
| `app/shared/utils.ts` | Sửa | Giữ `clamp`, `compareDocSeq`, `normalizeFolderPath` (sửa UNC). Bỏ 20 hàm còn lại và `moment` |
| `app/shared/sse-stream.ts` | Bỏ | Thay bằng lớp bọc `Channel` (mục 9.2) |
| `app/shared/import.shared.ts` | Bỏ | Toàn module PrimeNG |
| `app/shared/app-route-reuse.strategy.ts` | Nguyên | |
| `app/shared/components/base/base-component.ts` | Bỏ | Thay bằng service thông báo, xác nhận (mục 9.3) |
| `app/shared/components/stamp-designer/stamp-designer.ts` | Sửa | Mục 7.10. Phần tính toán giữ nguyên |
| `stamp-designer.html` | Viết lại | Giữ cấu trúc lớp phủ ở dòng 364-419 (vòng lặp khối, style phần trăm, sự kiện pointer, nút góc) |
| `stamp-designer.scss` | Sửa | Phần `.stamp-layer`, `.stamp-block`, `.stamp-block__handle` (dòng 169-250) là CSS thuần, dùng lại được; đổi hai biến màu |
| `app/shared/components/pdf-pages/pdf-pages.ts` | Sửa | Mục 8.8. Logic giữ nguyên |
| `pdf-pages.html`, `pdf-pages.scss` | Sửa | Đổi icon và vài class tiện ích |
| `app/shared/components/pdf-source-picker/pdf-source-picker.ts` | Sửa | Giữ `addSources`, `removeSource`, `clearSources`, `sourceName`, `isFolderSource`. Bỏ `onDrop` kiểu tải lên, `droppedSources`, `uploading`, `ensureDesktop`, `addSourceInput`, `onPaste` |
| `app/shared/components/path-picker/path-picker.ts` | Sửa | Giữ `pick`, `clear`, `setValue` và các `computed`. Bỏ `onDrop` kiểu tải lên, `droppedPath`, `uploading` |
| `app/shared/components/breadcrumb/*` | Bỏ | Chỉ bọc `p-breadcrumb` |
| `app/shared/components/data-table/*` | Bỏ | Không nơi nào dùng |
| `app/services/desktop.service.ts` | Bỏ | Thay bằng `tauri-plugin-dialog` và lệnh `shell_open` |
| `app/services/file-picker.service.ts` | Sửa | Mục 9.5 |
| `app/services/file-upload.service.ts` | Bỏ | |
| `app/services/sign.service.ts`, `document-merge.service.ts` | Viết lại | Giữ tên hàm, đổi ruột sang `invoke` (mục 9.6) |
| `app/services/sign-config-state.service.ts` | Sửa | Nạp và lưu qua `config_get`, `config_save` |
| `app/services/sign-cert-state.service.ts`, `sign-handoff.service.ts` | Nguyên | |
| `app/pages/ky-so/ky-so.ts` | Sửa | Giữ các signal và `computed`. Sửa `configSummary` (lỗi số 7), thêm nạp lại chứng thư (lỗi số 6) |
| `app/pages/ky-so/sign-step/sign-step.ts` | Sửa | Giữ khung luồng. Sửa các lỗi số 1 đến 5; viết lại `handleSignEvent` theo `JobEvent` |
| `app/pages/ky-so/sign-step/sign-result-dialog/*` | Viết lại | Logic chỉ vài dòng |
| `app/pages/ky-so/sign-config-dialog/sign-config-dialog.ts` | Sửa | Giữ: bản nháp, `patch`, vị trí riêng từng file, kéo khối chữ ký. Phần đo kích thước khối (`signatureSize`, `fitSignature`) phụ thuộc quyết định về mô hình mẫu chữ ký của bản mới |
| `app/pages/tao-van-ban/tao-van-ban.ts` | Sửa | Giữ gần nguyên wizard và các `computed` dựng request. Sửa lỗi số 24 |
| `app/pages/tao-van-ban/merge-source-step/*.ts`, `merge-mapping-step/*.ts`, `merge-stamp-step/*.ts`, `merge-run-step/*.ts` | Sửa | Đổi service, bỏ `BaseComponent`; `handleProgress` theo `JobEvent` |
| `app/pages/tao-van-ban/merge-result-dialog/*` | Viết lại | |

### 10.2 Theo hàm (những hàm đáng kể)

| Hàm | Vị trí | Kết luận |
|---|---|---|
| `baseSize`, `resolvePages`, `parseCustomPages`, `compressPages`, `togglePage`, `pageItems`, `areaOf`, `hasOwnPosition`, `ownPages`, `writesToPage`, `applyArea`, `resetStampSize`, `resetPagePosition(s)`, `stampSummary`, `appliedSummary` | `stamp-designer.ts:244-443` | Nguyên |
| `onStampPointerDown`, `onStampPointerMove`, `onStampPointerUp` | `stamp-designer.ts:491-556` | Nguyên, cộng thêm xử lý `pointercancel` (lỗi số 11) |
| `onPaletteDragStart`, `onPageDragOver`, `onPageDrop` | `stamp-designer.ts:445-489` | Sửa: chuyển sang pointer events, giữ công thức |
| `loadImageAspect`, `imageUrl` | `stamp-designer.ts:149-152, 229-242` | Sửa: đổi nguồn URL |
| `browseImage`, `onTileDragOver`, `onTileDragLeave`, `onTileDrop` | `stamp-designer.ts:162-203` | Sửa: hộp thoại và kéo thả của Tauri |
| `open`, `renderPage`, `observePages` và ba `effect` | `pdf-pages.ts:81-186` | Nguyên, cộng các sửa lỗi ở mục 8.8 |
| `signatureBox`, `defaultSignatureBox`, `signatureSize`, `pageFontPt`, `pageScale`, `fitSignature` | `sign-config-dialog.ts:212-229, 343-385` | Tuỳ quyết định mô hình khối chữ ký. Nếu lõi Rust tự tính kích thước khối thì giao diện chỉ cần phần hiển thị |
| `toggleOwnLayout`, `updateLayout`, `removeStampsOf`, `ownLayout`, `layout` | `sign-config-dialog.ts:108-117, 168-189, 317-341` | Nguyên nếu giữ tính năng vị trí riêng từng file |
| `darknessFilter` | `sign-config-dialog.ts:387-394` | Nguyên |
| `baseRequest`, `request`, `sourceReady`, `mappingFresh`, `canNext`, `next`, `readData`, `validate` | `tao-van-ban.ts:70-217` | Nguyên, đổi lời gọi service |
| `loadPreview` và cache `editPreviews` | `merge-stamp-step.ts:74-114` | Nguyên, đổi cách tạo URL |
| `handleProgress`, `markNextProcessing` | `merge-run-step.ts:152-192` | Sửa theo `JobEvent` |
| `handleSignEvent` | `sign-step.ts:223-251` | Sửa theo `JobEvent`, khớp theo `index` |
| `streamSse` | `sse-stream.ts`, `sign.service.ts:72-127` | Bỏ |

### 10.3 Chỗ dùng PrimeNG cần gỡ

Số lần xuất hiện trong template (đếm bằng tìm kiếm thẻ `<p-…>` và directive):

| Thành phần | Số lần | Nơi dùng | Gợi ý thay |
|---|---|---|---|
| `p-button` | 37 | 13 template | `<button>` + class của prototype |
| `pTooltip` | 16 | `stamp-designer` 6, `sign-config-dialog` 4, `merge-stamp-step` 3, `path-picker` 1, `merge-run-step` 1, `merge-source-step` 1 | Thuộc tính `title` hoặc tooltip CSS |
| `p-select` | 6 | `ky-so` 1 (chứng thư), `sign-config-dialog` 2 (font, file), `merge-mapping-step` 2 (cột), `merge-stamp-step` 1 (dòng) | Cần một ô chọn có tìm kiếm: 5 trong 6 chỗ bật `[filter]="true"` |
| `pInputText` | 6 | `sign-config-dialog` 2, `stamp-designer` 2, `path-picker` 1, `pdf-source-picker` 1 | `<input>` |
| `p-toggleswitch` | 4 | `sign-config-dialog` 3, `stamp-designer` 1 | Checkbox kiểu công tắc |
| `p-dialog` | 3 | `sign-config-dialog`, `sign-result-dialog`, `merge-result-dialog` | `<dialog>` hoặc lớp phủ của prototype |
| `p-slider` | 3 | `sign-config-dialog` | `<input type="range">` |
| `pTemplate` | 3 | `sign-step` 2, `merge-mapping-step` 1 | Bỏ cùng `p-table`, `p-select` |
| `p-colorpicker` | 2 | `sign-config-dialog` | `<input type="color">` |
| `p-inputnumber` | 2 | `sign-config-dialog` (cỡ chữ có hậu tố ` pt` và nút tăng giảm; số trang) | `<input type="number">` + nút |
| `p-progressbar` | 2 | `sign-step`, `merge-run-step` | Thanh tiến độ của prototype |
| `p-message` | 2 | `ky-so` | Khối thông báo tĩnh |
| `p-table` | 1 | `sign-step` (cuộn, có đường kẻ) | `<table>` |
| `p-selectbutton` | 1 | `stamp-designer` (chế độ xem) | Nhóm nút |
| `p-checkbox`, `p-tag`, `p-divider`, `p-breadcrumb` | 1 mỗi loại | `merge-run-step`, `ky-so`, `ky-so`, `breadcrumb` | Phần tử HTML thường |
| `p-toast`, `p-confirmdialog` | 1 mỗi loại | `layout.html:1-2` | Thành phần của service thông báo, xác nhận mới |

Ngoài template:

- `src/app.config.ts:5-8, 18-29`: `providePrimeNG`, preset `Aura`, ba service.
- `base-component.ts:4-5, 14-16`: `MessageService`, `DialogService`, `ConfirmationService`.
- `import.shared.ts`: 15 module.
- `ky-so.ts:2`, `breadcrumb.ts:2`: kiểu `MenuItem`.
- Import riêng: `TagModule` (`ky-so.ts`, `sign-step.ts`), `ProgressBarModule` (`sign-step.ts`, `merge-run-step.ts`), `TableModule` (`sign-step.ts`), `ColorPickerModule`, `SliderModule`, `ToggleSwitchModule` (`sign-config-dialog.ts`), `SelectButtonModule`, `ToggleSwitchModule` (`stamp-designer.ts`), `ToastModule`, `ConfirmDialogModule` (`layout.ts`).
- CSS ghi đè class của PrimeNG: `src/styles.css:17-20` (`.p-component`), `:25` (`.p-datatable-tbody`); `sign-result-dialog.scss:5-13` (`.p-dialog-content`); `breadcrumb.scss`; `data-table.scss`.
- Icon PrimeIcons (`pi pi-*`): 21 file. Trong đó có 5 file chứa tên icon ngay trong code TypeScript: `menu.ts:32-33`, `sign-result-dialog.ts:32,38`, `document-merge.constants.ts:72-73`, `base-component.ts:121`, `ky-so.ts:29`.

### 10.4 Chỗ dùng Tailwind

Mọi template dùng class tiện ích Tailwind 4, kể cả giá trị tuỳ ý như `bg-[#f5f7fa]`, `border-[var(--layout-border)]`, `grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]`. Ngoài template, class Tailwind còn nằm trong code TypeScript:

| Vị trí | Nội dung |
|---|---|
| `stamp-designer.ts:51` | `host: { class: 'flex min-h-0 flex-1 gap-4' }` |
| `merge-source-step.ts:13`, `merge-mapping-step.ts:25`, `merge-stamp-step.ts:19` | `host: { class: 'flex min-h-0 flex-1 flex-col' }` |
| `merge-run-step.ts:27` | `host: { class: 'flex min-h-0 flex-1 flex-col gap-4' }` |
| `merge-mapping-step.ts:14-18` | `FIELD_STATUS`: `pillClass`, `barClass` |
| `path-picker.ts:21` | Mặc định `iconClass = 'bg-slate-100 text-slate-500'`; giá trị truyền vào ở `merge-source-step.html:13,28`, `merge-run-step.html:9` |

Kế hoạch (mục 19.2) nói biến CSS lấy từ prototype nhưng không nói có giữ Tailwind hay không. Nếu bỏ, các chuỗi class trong code ở bảng trên phải đổi cùng template. Nếu giữ làm lớp tiện ích, template vẫn phải viết lại vì PrimeNG.

---

## 11. Điểm đáng lưu ý và lỗi tiềm ẩn

Tất cả các mục dưới đây rút ra từ đọc code, chưa chạy ứng dụng để tái hiện. Mức độ: **Cao** là có thể làm hỏng một lô việc hoặc cho kết quả sai; **Trung bình** là sai hành vi hoặc trải nghiệm xấu trong tình huống dễ gặp; **Thấp** là hiếm gặp hoặc chỉ ảnh hưởng hiển thị. Số thứ tự được các mục khác trong tài liệu tham chiếu tới.

### 11.1 Luồng Ký số

**1. Một biến `sub` giữ cả request liệt kê lẫn request ký; đổi nguồn giữa lúc ký sẽ huỷ lô ký và treo giao diện.** Mức độ: Cao.

Vị trí: `sign-step.ts:44-56` (effect), `:78` (khai báo `sub`), `:99-111` (`startCheck`), `:193, 206-220` (`startSign`); `ky-so.html:8`; `ky-so.ts:121-124`.

Effect trong constructor gọi `startCheck()` mỗi khi `sources` đổi, không xét `signing()`. `startCheck` gọi `this.sub?.unsubscribe()`, mà lúc đang ký `sub` chính là request ký, nên lô ký bị huỷ. Vì đã `unsubscribe`, cả `error` lẫn `complete` đều không chạy, `signing` giữ nguyên `true`: nút "Ký số" quay mãi, nút "Kiểm tra lại" bị khoá. Cách thoát duy nhất là xoá hết nguồn rồi thêm lại. Có hai đường dẫn tới tình huống này: `PdfSourcePicker` trên trang Ký số không bị khoá khi đang ký, và `takeHandoff` từ Tạo văn bản thay danh sách nguồn bất kể trang Ký số đang làm gì.

Bản mới: tách biến cho từng việc; khoá ô chọn nguồn khi đang ký; từ chối hoặc xếp hàng bàn giao khi đang có lô chạy.

**2. Kết quả ký được khớp vào dòng theo tên file.** Mức độ: Cao.

Vị trí: `sign-step.ts:115` (chỉ giữ `name`), `:232` (`r.name === f.fileName`).

Hai file cùng tên ở hai thư mục nguồn khác nhau cho hai dòng cùng `name`. Sự kiện đầu tiên cập nhật cả hai dòng; sự kiện thứ hai ghi đè cả hai. Trạng thái hiển thị sai dù backend ký đúng. Sự kiện `file` của backend cũ cũng chỉ mang tên file (đối chiếu `SignService.cs:357, 401`), nên giao thức cũ không phân biệt được.

Bản mới: giữ `fullPath` trong dòng, khớp theo `index` hoặc đường dẫn đầy đủ.

**3. Trạng thái "đã xác thực" không bao giờ bị xoá.** Mức độ: Trung bình.

Vị trí: `sign-cert-state.service.ts:6`; `sign-step.ts:158`; `ky-so.ts:60-62`.

`verifiedCert` chỉ bị đặt lại khi một lần kiểm tra khác thất bại. Rút token, lô ký thất bại vì token, hay để ứng dụng mở qua đêm đều không xoá nó. Lần bấm "Ký số" sau đó bỏ qua bước kiểm tra và đi thẳng vào ký; nhãn "Đã xác thực" vẫn hiện.

Bản mới: xoá khi nhận `stoppedReason: 'token_removed'`, khi `token_status()` báo mất token, và khi lô ký thất bại vì lỗi khoá.

**4. Không có nút huỷ ký; lô ký tiếp tục chạy khi rời màn hình.** Mức độ: Trung bình.

Vị trí: `sign-step.html:166-175` (chỉ có nút "Ký số"); `sign-step.ts:80-82`; `ky-so.router.ts:5`.

Request ký chỉ bị huỷ khi `SignStep` bị gỡ. Chuyển sang tab Tạo văn bản không gỡ nó, nên lô ký chạy nền mà không có chỉ báo nào ở khung ứng dụng.

Bản mới: có `job_cancel`; cần nút "Dừng" và chỉ báo lô đang chạy.

**5. `checkPositions` không huỷ request cũ và không có số phiên bản.** Mức độ: Trung bình.

Vị trí: `sign-step.ts:125-151`.

Hàm được gọi khi nguồn đổi và khi cấu hình đổi. `Subscription` không được giữ. Hai lần gọi gối nhau thì response về sau thắng, kể cả khi nó thuộc lần gọi cũ; `positionChecking` cũng bị tắt bởi response về trước. So sánh: `readData` và `loadPreview` bên Tạo văn bản có bộ đếm phiên bản (`tao-van-ban.ts:180, 185`; `merge-stamp-step.ts:93, 97`).

**6. Danh sách chứng thư chỉ nạp một lần cho cả phiên.** Mức độ: Trung bình.

Vị trí: `ky-so.ts:64-71`.

`loadCertificates()` chỉ được gọi trong `ngOnInit`. Route được giữ lại nên `ngOnInit` không chạy lại, và `onRouteActivated` chỉ gọi `takeHandoff`. Không có nút nạp lại. Cắm token sau khi mở ứng dụng thì chứng thư không xuất hiện cho tới khi khởi động lại.

**7. Chip "N file riêng" ở trang Ký số luôn bằng 0.** Mức độ: Thấp.

Vị trí: `ky-so.ts:44-45, 52`.

`ownCount` so `fileOverrides[].path` với các phần tử của `sources`. `path` là đường dẫn file kiểu Windows do backend trả (dấu `\`); `sources` là đường dẫn file hoặc **thư mục** đã chuẩn hoá sang `/`. Hai tập này không bao giờ có phần tử bằng nhau. Trong dialog, `ownLayoutCount` so đúng với `files()[].fullPath` (`sign-config-dialog.ts:114-117`) nên hiển thị đúng.

**8. `fileOverrides` không bao giờ được dọn và được lưu vĩnh viễn.** Mức độ: Trung bình.

Vị trí: `sign-config-dialog.ts:168-189, 282-295`; `sign-config-state.service.ts:14-17`; `sign-step.ts:131-136, 187-191`.

Vị trí riêng của một file chỉ bị xoá khi người dùng mở lại đúng file đó và tắt công tắc. Phần tử của các file đã ký xong, đã xoá, đã đổi tên vẫn nằm trong cấu hình, được ghi xuống đĩa, và được gửi kèm mọi request kiểm tra vị trí và ký.

Bản mới: vị trí riêng từng file là dữ liệu của một lô, nên sống cùng danh sách nguồn chứ không nằm trong cấu hình lâu dài.

**9. `fitSignature` tính kích thước khối cho mọi file bằng số đo trang của file đang xem.** Mức độ: Thấp.

Vị trí: `sign-config-dialog.ts:288-292, 343-348`.

Khi lưu, mỗi phần tử `fileOverrides` được tính lại `widthRatio`, `heightRatio` theo `pageMetrics()` hiện tại, vốn thuộc file đang mở trong dialog. File khác có khổ trang khác sẽ nhận tỉ lệ sai; trang không tồn tại ở file đang xem thì dùng 595 × 842.

**10. Kích thước khối chữ ký số phụ thuộc phép đo chữ của trình duyệt và tên chứng thư lúc lưu.** Mức độ: Trung bình.

Vị trí: `sign-config-dialog.ts:120, 128, 360-374`.

Bề rộng khối là kết quả `canvas.measureText` của tên chứng thư đang chọn và một chuỗi thời gian mẫu. Số này được lưu vào cấu hình và backend dùng làm bề rộng khối. Đổi sang chứng thư có tên dài hơn mà không mở lại dialog thì bề rộng đã lưu không còn khớp. Kết quả cũng phụ thuộc font có sẵn trên máy.

Bản mới: lõi Rust có số đo font thật (`ttf-parser`), nên để lõi tính kích thước khối; giao diện chỉ gửi vị trí neo.

### 11.2 `stamp-designer` và dialog cấu hình

**11. Trạng thái kéo không được xoá khi con trỏ bị huỷ; không kiểm tra nút chuột.** Mức độ: Trung bình.

Vị trí: `stamp-designer.ts:491-511, 552-556`; `stamp-designer.html:387-389, 406-408`; `sign-config-dialog.ts:235-252, 272-276`; `sign-config-dialog.html:358-360`.

Chỉ có `pointerdown`, `pointermove`, `pointerup`. Không nghe `pointercancel` hay `lostpointercapture`. Nếu thao tác bị ngắt mà không có `pointerup` (cửa sổ mất tiêu điểm, hộp thoại hệ thống bật lên, thao tác chạm bị hệ điều hành chiếm), `this.drag` vẫn còn. Vì `pointermove` cũng phát khi chỉ rê chuột qua khối, lần rê tiếp theo sẽ kéo khối đi dù không bấm. `pointerdown` cũng không xét `event.button`, nên chuột phải và chuột giữa đều bắt đầu kéo.

**12. Thả bất kỳ thứ gì vào trang đều tạo một khối.** Mức độ: Trung bình.

Vị trí: `stamp-designer.ts:458-489`.

`onPageDragOver` chấp nhận mọi kiểu dữ liệu, `onPageDrop` dùng `this.paletteKind` (mặc định `Seal`) mà không đọc `dataTransfer`. Thả một file từ Explorer vào trang tạo ra khối dấu đỏ, kể cả khi chưa chọn ảnh dấu đỏ. Khối không có ảnh vẫn nằm trong `stamps` và backend sẽ từ chối cả request với lỗi "Khối ảnh N cần ảnh dấu đỏ nhưng chưa chọn ảnh." (đối chiếu `StampPlanner.cs:36-41`).

**13. Tỉ lệ ảnh sai trong ba tình huống.** Mức độ: Trung bình.

Vị trí: `stamp-designer.ts:132-139, 205-214, 229-249`; `stamp-designer.scss:210-215`.

(a) Tỉ lệ ảnh được đo bằng một lần nạp ảnh riêng; nếu người dùng thả khối trước khi đo xong, khối dùng tỉ lệ `1` và ảnh bị kéo thành vuông. Hai lần đổi ảnh liên tiếp cũng có thể ghi tỉ lệ của ảnh cũ nếu ảnh cũ nạp xong sau.

(b) Đổi sang ảnh khác cho cùng loại không cập nhật các khối đã đặt: chúng giữ `widthRatio`, `heightRatio` của ảnh cũ cho tới khi bấm "Kích cỡ gốc".

(c) Khối lặp sang trang khác khổ hoặc khác hướng bị méo, vì rộng và cao là hai tỉ lệ độc lập (mục 7.4). Backend vẽ đúng theo bốn tỉ lệ nhận được (đối chiếu `PdfImageStamper.cs:49-53`).

Bản mới: lõi nên tự tính chiều cao từ chiều rộng và tỉ lệ ảnh thật, hoặc lưu bề rộng theo point.

**14. Khối không di chuyển được khi `scope` là "Trang hiện tại" mà còn sót vị trí riêng.** Mức độ: Thấp.

Vị trí: `stamp-designer.ts:347-349, 381-395, 408-410, 519`.

`onScopeChange` chỉ đổi `scope`, không xoá `pagePositions`. Nếu khối từng có vị trí riêng ở trang gốc rồi được đổi về "Trang hiện tại", `areaOf` vẫn trả vị trí riêng để hiển thị, còn `writesToPage` trả `false` nên thao tác kéo ghi vào vị trí chung. Khối đứng yên trên màn hình trong khi vị trí chung thay đổi ngầm. Backend dùng vị trí riêng (đối chiếu `StampPlanner.cs:99`), nên kết quả khớp với màn hình, nhưng người dùng không kéo được khối.

**15. Cho lưu và gửi chuỗi trang tuỳ chọn sai.** Mức độ: Trung bình.

Vị trí: `stamp-designer.ts:272-275, 302-315`; `sign-config-dialog.ts:282-295`; `tao-van-ban.ts:114-115`.

`customPagesError` chỉ hiện dòng chữ đỏ. Nút "Lưu cấu hình" và nút "Tiếp theo" không xét lỗi này. Bỏ chọn hết các chip trang cũng cho ra `customPages` rỗng. Backend ném lỗi cho chuỗi sai hoặc rỗng (đối chiếu `StampPlanner.cs:167-204`), nên cả lô ký hoặc cả lượt tạo file bị từ chối với một câu báo lỗi, thay vì bị chặn ngay ở giao diện.

**16. Khối đang chọn không được đặt lại khi danh sách khối đổi từ bên ngoài.** Mức độ: Thấp.

Vị trí: `stamp-designer.ts:84-85, 100`; `sign-config-dialog.html:37`.

Khối được nhận diện bằng chỉ số. Khi đổi file xem trước trong dialog cấu hình (file kia có vị trí riêng), `stamps` trở thành một danh sách khác nhưng `activeIndex`, `activePage` giữ nguyên, nên bảng thuộc tính trỏ sang khối cùng chỉ số của file mới.

### 11.3 `pdf-pages`

**17. Render chồng lên cùng một canvas sau khi đổi cỡ; không huỷ render; lỗi không được bắt.** Mức độ: Trung bình.

Vị trí: `pdf-pages.ts:95-99, 113-116, 168-186`.

Sau khi vùng chứa đổi cỡ, `rendered.clear()` rồi quan sát lại, nên `renderPage` được gọi lại cho các trang đang thấy. Nếu lượt vẽ trước của trang đó chưa xong, pdf.js từ chối vẽ hai lượt cùng lúc trên một canvas. Lỗi này không ai bắt (`void this.renderPage(…)`), và vì số trang đã nằm trong `rendered` nên trang không được vẽ lại cho tới lần đổi cỡ sau. Tương tự, đổi `url` trong lúc đang vẽ để lại promise lỗi không ai xử lý.

**18. Canvas đã vẽ không bao giờ được giải phóng.** Mức độ: Cao với file nhiều trang.

Vị trí: `pdf-pages.ts:29, 176-185`.

Mỗi trang A4 ở bề rộng tối đa chiếm khoảng 27 MB (mục 8.4). Cuộn hết một file 100 trang là giữ khoảng 2,7 GB ảnh trong bộ nhớ của webview.

**19. Đo mọi trang tuần tự trước khi hiện trang đầu.** Mức độ: Trung bình.

Vị trí: `pdf-pages.ts:152-155`.

`await doc.getPage(n)` cho từng trang trong một vòng lặp. Thời gian chờ tăng tuyến tính theo số trang.

**20. `rootMargin: '300px'` có thể không có tác dụng.** Mức độ: Thấp. Chưa chắc.

Vị trí: `pdf-pages.ts:111-119`.

`IntersectionObserver` được tạo không có `root`, tức gốc là khung nhìn của cả trang. Các canvas lại nằm trong một vùng cuộn riêng (`.pdf-pages { overflow: auto }`). Theo đặc tả, phần bị vùng cuộn che thì không tính là giao, và `rootMargin` chỉ nới gốc. Nếu đúng vậy, trang chỉ bắt đầu vẽ khi đã lộ ra, không được vẽ trước 300 px như ý định.

**21. Thiếu `wasm`, `cmaps`, `standard_fonts` của pdf.js.** Mức độ: Trung bình. Chưa chắc.

Vị trí: `pdf-pages.ts:147`; `angular.json` (assets).

Xem mục 8.6. Văn bản scan là loại file phổ biến của người dùng sản phẩm này.

### 11.4 Hạ tầng dùng chung

**22. Hàm đọc SSE xử lý thiếu vài trường hợp, và bị chép hai nơi.** Mức độ: Thấp (sẽ bỏ).

Vị trí: `sse-stream.ts:16-24, 36-45`; bản sao `sign.service.ts:72-127`.

Kiểm tra `!res.ok` đứng trước kiểm tra JSON, nên response lỗi HTTP có thân JSON mất thông báo của backend, chỉ còn `HTTP 500`. Chỉ tách theo `\n\n`. Phần còn lại trong bộ đệm khi luồng đóng bị bỏ. `JSON.parse` lỗi thì báo lỗi nhưng không huỷ request, backend vẫn chạy tiếp.

**23. `normalizeFolderPath` làm hỏng đường dẫn UNC và gốc ổ đĩa.** Mức độ: Trung bình.

Vị trí: `utils.ts:154-162`.

Xem bảng ở mục 9.1. Người dùng để hồ sơ trên ổ mạng (`\\maychu\…`) sẽ không ký được qua đường dẫn UNC. Hàm này được gọi ở 17 chỗ, gồm mọi đường dẫn nguồn, ảnh, mẫu, dữ liệu, thư mục xuất.

### 11.5 Tạo văn bản

**24. Đổi thư mục xuất ở bước 4 làm bước 3 mất danh sách dòng.** Mức độ: Trung bình.

Vị trí: `tao-van-ban.ts:85-94, 134-137`; `tao-van-ban.html:70`; `merge-stamp-step.ts:45-60`.

`outputFolder` nằm trong `baseRequest`. Chọn thư mục ở bước 4 làm `baseRequest` đổi, `validationReset` đặt `validation` về `null`. Bấm "Quay lại" về bước 3 thì `rows` rỗng: không có dòng nào để xem trước, vùng xem trước hiện mãi câu "Đang tạo bản xem trước đã nhồi dữ liệu.". Phải lùi tiếp về bước 2 rồi "Tiếp theo" để kiểm tra lại.

Bản mới: tách `outputFolder` khỏi request dùng để kiểm tra.

**25. Dấu "Đang tạo" dựa trên giả định xử lý tuần tự; kiểm tra ở bước 4 không tự chạy lại.** Mức độ: Thấp.

Vị trí: `merge-run-step.ts:186-192`; `merge-run-step.ts:67-69, 75-96`.

`markNextProcessing` đánh dấu dòng `'idle'` đầu tiên sau mỗi sự kiện. Đúng với backend cũ, sai nếu bản mới chạy song song. Bước 4 chỉ kiểm tra một lần lúc vào bước; `start()` dùng `request()` hiện tại nên vẫn đúng, nhưng bảng tổng quan không phản ánh thay đổi cho tới khi bấm "Kiểm tra lại".

**26. "Bản hoàn chỉnh" không tự làm mới khi đổi hoặc bỏ ảnh.** Mức độ: Thấp.

Vị trí: `merge-stamp-step.ts:54-60`; `stamp-designer.ts:125-130, 205-214`; `stamp-designer.html:56-76`.

Ở chế độ `Final`, hai nút "Đổi ảnh", "Bỏ ảnh" trên ô ảnh mẫu vẫn bấm được. Bản xem trước chỉ được tạo lại khi đổi dòng hoặc đổi chế độ, nên sau khi đổi ảnh, màn hình vẫn là bản cũ. Bỏ ảnh làm `stamps` rỗng, lựa chọn "Bản hoàn chỉnh" bị khoá nhưng chế độ vẫn là `Final`.

**27. Bàn giao sang Ký số thay thế danh sách nguồn.** Mức độ: Thấp.

Vị trí: `ky-so.ts:121-124`.

`this.sources.set(paths)` xoá các nguồn người dùng đã chọn trước đó, không hỏi. Kết hợp với lỗi số 1 nếu trang Ký số đang ký dở.

### 11.6 Các điểm khác

**28. `DesktopService`: mỗi loại hộp thoại chỉ giữ một lời hứa đang chờ, không có timeout; `openPath` không biết kết quả.** Mức độ: Thấp (sẽ bỏ).

Vị trí: `desktop.service.ts:7-11, 27-71`.

Nếu vỏ desktop không trả lời, `await` treo mãi. `openPath` trả `true` kể cả khi đường dẫn không tồn tại (vỏ desktop lặng lẽ bỏ qua, đối chiếu `Program.cs:113-120`), nên nơi gọi không chuyển sang phương án chép vào clipboard.

**29. Ảnh kéo thả nằm trong thư mục tạm nhưng đường dẫn được lưu vào cấu hình.** Mức độ: Thấp (hết khi có đường dẫn thật).

Vị trí: `stamp-designer.ts:192-197`; `sign-config-dialog.ts:148-156`; `sign-config-state.service.ts:14-17`.

Thả một file ảnh vào ô ảnh mẫu sẽ tải nó lên thư mục tạm và ghi đường dẫn tạm vào `sealImagePath` hoặc `wetSignatureImagePath`. Nếu thư mục tạm bị dọn, cấu hình trỏ tới file không còn. Chưa chắc về chính sách dọn của backend (chưa đọc).

**30. Xem trước không mô phỏng độ dày nét và màu mực.** Mức độ: Thấp.

Vị trí: `sign-config-dialog.ts:123-126, 387-394`.

Chỉ độ đậm có CSS filter. Kéo thanh "Độ dày nét chữ ký tươi" hay đổi "Màu mực chữ ký tươi" không làm ảnh trên màn hình thay đổi; người dùng chỉ thấy kết quả sau khi ký.

**31. Danh sách file được liệt kê nhiều lần cho cùng một bộ nguồn.** Mức độ: Trung bình (hiệu năng).

Vị trí: `sign-step.ts:111`; `sign-config-dialog.ts:305`; cộng `kiem-tra-vi-tri` và `ky-thu-muc` tự liệt kê lại ở backend.

Mỗi lần liệt kê, backend mở và kiểm tra chữ ký của từng file (đối chiếu `SignService.cs:122`). Với thư mục vài trăm file, mở dialog cấu hình là chờ thêm một lượt.

Bản mới: gọi `sign_list_files` một lần, giữ kết quả trong một service dùng chung.

**32. Nhiều hàm nặng được gọi trực tiếp trong template.** Mức độ: Thấp.

Vị trí: `stamp-designer.html:256-260, 367`; `sign-config-dialog.html:344`.

`pageItems(page)` chạy cho mỗi trang ở mỗi lượt cập nhật giao diện và gọi `resolvePages` cho mỗi khối; `resolvePages(stamp)` còn được gọi hai lần cho mỗi chip trang; `signatureBox(page)` gọi `measureText` hai lần cho mỗi trang. Trong lúc kéo, mỗi `pointermove` là một lượt. Nên đổi sang `computed`.

### 11.7 Subscription và rò rỉ

Không thấy rò rỉ nghiêm trọng. Cụ thể:

- `BaseComponent` đăng ký `router.events` cho mỗi thể hiện của 11 lớp component, đều tự huỷ bằng `takeUntilDestroyed()` (`base-component.ts:25-28`). Component được giữ lại bởi `AppRouteReuseStrategy` vẫn nhận sự kiện khi đang tách khỏi màn hình; đây là chủ ý (mục 6.6).
- Hai request dài (`SignStep.sub`, `MergeRunStep.sub`) được huỷ trong `ngOnDestroy`.
- Các lời gọi `HttpClient` một lần (`loadCertificates`, `verify`, `checkPositions`, `loadFiles`, `remap`, `validate`) không giữ `Subscription`. Chúng tự kết thúc sau một response nên không rò rỉ, nhưng callback có thể chạy sau khi component đã bị gỡ hoặc sau một lời gọi mới hơn (lỗi số 5). `SignConfigDialog.loadFiles` (`sign-config-dialog.ts:297-315`) cũng không có số phiên bản.
- `PdfPages` dọn `IntersectionObserver`, `ResizeObserver`, timer và tác vụ nạp khi bị huỷ (`pdf-pages.ts:72-79`).
- `LayoutService` gắn listener lên `window`, `document` và không gỡ (`layout.service.ts:10, 17-42`); service sống suốt phiên nên chấp nhận được.
- `DataTable` đăng ký `customEmit` không gỡ (`data-table.ts:53`); component không được dùng.

### 11.8 Các điểm nhỏ

- `ky-so.html:13`: class `text-primary` không sinh CSS (không có plugin hay `@theme` định nghĩa màu `primary`).
- `src/styles.css:180-187`: selector `app-home` không ứng với component nào. Cùng khối đó đặt `display: flex` cho `app-sign-step`, nhưng `sign-step.scss:1-3` đặt `:host { display: block; }`. Xét độ ưu tiên selector thì luật của component (Angular đổi `:host` thành selector thuộc tính) thắng selector thẻ, tức `block`; khi đó bảng file không co giãn theo chiều cao như ý định của `.table-frame`. Chưa chạy thử để xác nhận.
- `ky-so.ts:50`: nhãn tóm tắt ghi `"N dấu, chữ ký tươi"` cho tổng số khối, không tách theo loại.
- `sign-step.ts:154-157`: mọi lý do khiến `canSign()` sai đều báo cùng một câu "Không có file PDF nào để ký. Vui lòng kiểm tra lại.". Nút bị khoá trong các trường hợp đó nên câu này gần như không bao giờ hiện.
- `sign-step.html:99-104`: lý do lỗi của từng file chỉ có trong `title`.
- `utils.ts:98-99`: `makeRandom` ghi đè tham số `possible`. `base-component.ts:60-63`: `getErrorMessage` ném lỗi nếu `ValidationMessages[fieldName]` chưa khai. Cả hai không được gọi.
- `data-table.html:36`: `bypassSecurityTrustHtml` trên dữ liệu ô. Component không được dùng; đừng mang sang.
- `package.json`: `version` là `1.0.0`, không phản ánh phiên bản sản phẩm.
- `environment.desktop.ts` trùng `environment.ts`; cấu hình build `desktop` không khác `production` ở điểm nào ngoài phép thay file vô tác dụng này.
- Bảo mật: `sign/xem-file`, `tao-van-ban/xem-file`, `tao-van-ban/xem-anh` nhận đường dẫn tuyệt đối do giao diện gửi. Kế hoạch mục 10.3 đã xử lý bằng scope của asset protocol.

---

## 12. Những điều cần chốt trước khi viết giao diện mới

Rút từ các mục trên, xếp theo mức độ chặn việc:

1. **Kéo thả** (mục 7.6): giữ `dragDropEnabled: true` và viết lại thao tác thả ảnh mẫu bằng pointer events, hay ngược lại. Quyết định này ảnh hưởng 4 component.
2. **Mô hình khối chữ ký số** (mục 4.6, 7.9, lỗi số 10): giao diện đo và gửi kích thước như cũ, hay lõi tự tính từ mẫu theo khổ giấy. Kéo theo số phận của `signaturePosition`, `fileOverrides`, và phần lớn `sign-config-dialog.ts`.
3. **Dạng đường dẫn chuẩn** (mục 9.1, lỗi số 7 và 23): `\` hay `/`, có hỗ trợ UNC không, khoá so sánh đường dẫn giữa giao diện và lõi.
4. **Enum số hay chuỗi** trên IPC và trong `config.json` (mục 4.3). Code cũ dùng số ở mọi nơi.
5. **Nội dung `JobEvent`** (mục 5.6): thêm chi tiết chữ ký cho sự kiện `file` của lô ký, thêm `rowNumber` cho lô tạo văn bản, hay bỏ các cột tương ứng trên giao diện.
6. **Cách nạp PDF cho pdf.js** (mục 8.7): asset protocol hay truyền byte qua `invoke`. Cần một spike nhỏ với file có tên tiếng Việt và file scan.
7. **Tailwind** (mục 10.4): giữ làm lớp tiện ích hay bỏ hẳn.

