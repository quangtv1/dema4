# Phân tích prototype giao diện `App_Ky_So/sao-mai-ky-so_1.html`

> Tài liệu khảo sát phục vụ việc viết lại Sao Mai Ký số 1.5.0 (Rust + Tauri 2 + Angular 21).
> Nguồn: `App_Ky_So/sao-mai-ky-so_1.html` — 1675 dòng, 318 935 byte. Đối chiếu: `KE_HOACH_SAO_MAI_KY_SO_RUST.md` mục 2, 12, 13, 20.
>
> **Quy ước**
> - `L123` = dòng 123 của file prototype gốc. `L12–22` = khoảng dòng.
> - Chữ trong `code` hoặc trong ngoặc kép là **trích nguyên văn** từ file.
> - "Mô phỏng" = dữ liệu/hành vi giả để trình diễn; "Logic thật" = thuật toán chạy thật trong prototype, port được.
> - Chỗ nào ghi **(chưa chắc)** là suy luận, không có bằng chứng trực tiếp trong file.
>
> **Cách đọc:** file được đọc toàn bộ từ dòng 1 đến 1675. Ba dòng 7, 37, 534 chứa ảnh base64 rất dài (7,6 KB; 7,7 KB; 139 KB) nên phần chuỗi base64 được thay bằng nhãn kích thước khi đọc; mọi nội dung khác giữ nguyên, số dòng không đổi. Các số liệu đếm (số lần tham chiếu hàm, class không dùng, kích thước ảnh) lấy bằng script quét trên file gốc.

---

## 0. Tóm tắt nhanh

| Hạng mục | Số liệu |
|---|---|
| Màn hình | 6 mã màn (`home`, `kyso`, `tvb`, `kt`, `xemtruoc`, `tucauhinh`) do **5 hàm vẽ** (`manHome`, `manKySo`, `manTVB`, `manKT`, `manTC`); `xemtruoc` và `tucauhinh` dùng chung `manTC()` |
| Hộp thoại | **10** hộp trong `modalHTML()` (L1430–1444) + **3** hộp con của Cấu hình (`cfgSub()` L1603–1630) = 13. Trong đó hộp `ghep` **không mở được** |
| Lớp nổi nhỏ | 2 menu thả (Chức năng, Hỗ trợ), menu lọc trạng thái, menu chọn nguồn biến, menu ⋯ dòng token, tooltip `!` |
| Kỹ thuật | Một file, không framework, không thư viện ngoài. `<body>` chỉ có `<div id="app">`; toàn bộ DOM sinh bằng chuỗi HTML và gán `innerHTML` mỗi lần đổi trạng thái (`render()` L725) |
| Sự kiện | Uỷ quyền ở `document`: `click`→`ACTS[data-act]`, `change`→`CHG[data-chg]`, `input`→`INP[data-inp]`, `keydown`, `mousedown` (kéo) |
| Lưu trữ | Không có `localStorage`, không `fetch`, không `@media`, không `@font-face`. Trạng thái chỉ nằm trong biến `S` |

---

## 1. Cấu trúc file

### 1.1 Khối lớn

| Dòng | Khối | Nội dung |
|---|---|---|
| L1–7 | Đầu trang | `<!doctype html>`, `<html lang="vi">`, meta charset/viewport, `<title>Sao Mai Ký số</title>` (L6), favicon PNG base64 (L7) |
| L8–526 | `<style>` | Toàn bộ CSS (một khối) |
| L527–530 | `<body>` | `<p class="ghichu">Prototype giao diện — Sao Mai Ký số (không phải phần mềm thật, dữ liệu minh hoạ)</p>` (L529) và `<div class="cuaso" id="app"></div>` (L530). Không có markup tĩnh nào khác |
| L531–1673 | `<script>` | Toàn bộ JS, `"use strict"` (L532) |
| L1674–1675 | Đóng | `</body></html>` |

### 1.2 Các đoạn trong `<style>` (theo chú thích trong file)

| Dòng | Chú thích nguyên văn | Nội dung |
|---|---|---|
| L9–11 | `SAO MAI KÝ SỐ — layout theo prototype sm-dema4, màu thương hiệu Sao Mai` | Tiêu đề khối CSS |
| L12–22 | — | `:root` biến CSS |
| L23–31 | — | Reset, `body`, `.ghichu`, `.an`, `svg.i` |
| L33–42 | `Cửa sổ` | `.cuaso`, `.tieude`, `.cap` |
| L44–62 | `Thanh menu` | `.menubar`, `.mb`, `.ketnoi`, `.menu`, `.mi`, `.mi-sep` |
| L64–88 | `Màn chức năng` | `.man`, `.hang`, `.o` (ô có nút xoá), `.fld`, `.phai-hang`, `.hop` |
| L90–115 | `Nút` | `.nut`, `.seg-nhom/.seg`, `.chip`, `.nutma`, `.lk` |
| L117 | `tooltip ⓘ` | **Rỗng** (chỉ có chú thích; CSS tooltip thật ở L465–478) |
| L119–195 | `Khung bảng + tab` | `.khung`, `.tabs/.tab`, `.than`, `.cottrai`, `.keo`, `.ds`, `.dong`, `.chip-tt`, `.daytrai`, `.tim`, `.rail-*`, `.loc-*`, `.trung-*`, `.trong-ds`, `.phai`, `.dieuhuong`, `.vungxem` |
| L197–211 | `trang giấy` | `.giay`, `.giay-in`, `.nhan-trang`, `.bao` |
| L213–220 | `khung chữ ký trên trang` | `.chuky`, `.dau-son`, `.tt-ky`, `.tay` |
| L222–277 | `văn bản mẫu` | `.doc`, `.bien`, `.bdong`, `.bchip`, `.lt-*` (menu chọn nguồn), `.xls-*` (bảng Excel) |
| L279–291 | `Panel giới thiệu (mở đầu)` | `.mo-dau`, `.buoc`, `.logo-goc`, `.nhan-pt` |
| L293–313 | `Chân màn` | `.chan`, `.tiendo`, `.td-line`, `.td-tt`, ghi đè tab cho `#manKySo` (L305–307) |
| L315–320 | `Nhật ký` | `.log` |
| L322–348 | `Trang chủ` | `#manHome`, `.tc-*`, `.logo-dong` |
| L350–368 | `Xem trước / tự cấu hình` | `.thumb`, `.mini`, `.drawer`, `.ab-luoi`, `.ab-file`, `.tick` |
| L370–378 | `Tạo văn bản` | `.form-truong`, `.map` |
| L380–397 | `Hộp thoại` | `.phu`, `.popup`, `.luoi-tt`, `.logott`, `.icon-lon`, `.so-lon` |
| L399–431 | `cấu hình` | `.popup.cauhinh`, `.tabch`, `.cfg-than`, `.pc`, `.the`, `.row`, `.sw`, `.grid-lbl`, `.mitem`, `.rowmenu`, `.bang-tk`, `.pill-ok` |
| L433–443 | `hướng dẫn` | `.hd-nav`, `.nav`, `.hd-the`, `kbd` |
| L445–463 | `khung chữ ký kéo thả…` / `trang văn bản…` | `.chuky-o` và một loạt **ghi đè** (`#manHome`, `.giay > .nhan-trang`, `.mini .o-ky`…) |
| L465–478 | `Nút ! : bấm để xem thông tin` | `.tip` |
| L480–498 | `Nâng cao` | `.nut.nangcao`, `.nc-panel`, `.the-nc`, `@keyframes ncMo` |
| L500–512 | `Ô chọn vị trí chữ ký (trang giấy nhỏ, 6 điểm)` | `.vt-pick`, `.vt-trang`, `.vt-o` |
| L514–518 | `Thanh Hiển thị nổi trên trang preview` | `.tb-noi`, `.seg-nhom.nho` |
| L520–524 | `bỏ mọi vạch xanh bên trái` | 4 dòng ghi đè |

### 1.3 Các đoạn trong `<script>` (theo chú thích trong file)

| Dòng | Chú thích nguyên văn | Thành phần |
|---|---|---|
| L533–534 | `ẢNH THƯƠNG HIỆU` | `IMG={app,logo,sao,full}` (4 PNG base64, cùng một dòng L534) |
| L536–593 | `ICON (nét 1 màu, kiểu Lucide)` | `P` (57 icon viewBox 24), `ic()`, `caret()` |
| L595–602 | `Icon nét lớn cho thẻ chức năng + panel giới thiệu (viewBox 100, cùng nét với prototype)` | `LA` (4 icon), `la()` |
| L604–613 | — | `esc()`, `tip()`, `pad()`, `clamp()`, `boDau()` |
| L615–638 | `DỮ LIỆU MẪU` | `FILES`, `ONE_FILE`, `FAIL`, `soTrang()`, `MODE_LABEL`, `PROFILES_DEF`, `TK_DEF`, `PAPER`, `POS_LABEL`, `TOKENS_DEF`, `AVAILABLE` |
| L640–655 | `TRẠNG THÁI` | `S` |
| L657–667 | `Vị trí khung chữ ký (đơn vị %, trên trang A4 dọc 520×736) theo mẫu cấu hình` | `sigRect()`, `ksProfile()` |
| L669–723 | `VỎ ỨNG DỤNG` | `TITLES`, `titlebarHTML()`, `menuDefs()`, `MENU_ACTS`, `menubarHTML()` |
| L724–781 | `RENDER` | `render()`, `go()`, `openModal()`, `closeModal()`, `openCfg()`, `oInput()`, `giay()`, `zoomBar()`, `introHTML()` |
| L782–820 | `ĐIỀU PHỐI SỰ KIỆN` | `ACTS/CHG/INP`, listener `click/change/input/keydown`, các action chung |
| L822–857 | `KÉO THẢ / ĐỔI CỠ` | `DRAG`, listener `mousedown`, `dragMove()`, `dragUp()` |
| L859–890 | `LOGO ĐỘNG (cùng thông số logo_sao_mai.rs của prototype)` | IIFE hoạt ảnh, `logoDongHTML()`, `advBtn()` |
| L892–937 | `VĂN BẢN MẪU + KHUNG CHỮ KÝ` | `docQD()`, `sigInner()`, `sigBox()`, `trang()`, `miniTrang()`, `spin()`, `chipTT()` |
| L939–954 | `TRANG CHỦ` | `manHome()` |
| L955–1147 | `KÝ SỐ VĂN BẢN` | `ksFiles()`…`ksStats()`, `manKySo()`, `ksStop()`, `ksStart()`, action `ks.*`, `dup.*` |
| L1149–1304 | `TẠO VĂN BẢN` | `TVB_*`, `tvbTenBase()`, `docCN()`, `manTVB()`, action `tvb.*` |
| L1306–1361 | `KIỂM TRA CHỮ KÝ` | `SIGS`, `manKT()`, action `kt.*` |
| L1363–1366 | `XEM TRƯỚC CHỮ KÝ` | Chỉ còn `XT_N=8`, `ACTS['cfg.open']`, `CHG['xt.profile']` (đoạn sót lại) |
| L1368–1423 | `TỰ CẤU HÌNH VỊ TRÍ (kiểu ABBYY 60/40)` | `viTriPick()`, `hienThiBar()`, `tcTpl()`, `tcRect()`, `tcSet()`, `manTC()`, action `tc.*`, `xt.*` |
| L1425–1444 | `HỘP THOẠI` | `popup()`, `modalHTML()` |
| L1446–1488 | `Ký số: nhập PIN` | `mPin()`, `mTrung()`, `mDone()`, `mTaoFile()`, `mGhep()` |
| L1490–1539 | `Hỗ trợ` | `HD`, `mHuongDan()`, `mCapNhat()`, `mThongTin()`, `mDriver()` |
| L1541–1638 | `Cấu hình (hộp thoại có tab, như prototype)` | `cfgP()`, `cfgSetP()`, `cfgSetT()`, `opt()`, `cfgPreview()`, `cfgSig()`, `cfgToken()`, `cfgFolder()`, `cfgSub()`, `mCauHinh()` |
| L1640–1667 | `Hành động hộp thoại` | action `pin.*`, `done.log`, `tvb.kySo`, `hd.*`, `cfg.*` |
| L1669–1670 | `KHỞI ĐỘNG` | `render();` |

---

## 2. Design tokens

### 2.1 `:root` — chép nguyên văn (L12–22)

```css
:root{
  --accent:#0a54b8; --accent-h:#1567cc; --accent-a:#063f8f; --brand2:#1088e8; --star:#f5a000; --orange:#e08a12;
  --bg:#f2f3f5; --surface:#fff; --panel:#f7f8fa; --view:#e7e9ed;
  --line:#dfe3e8; --line2:#eceff2; --ctl:#c9cfd6; --ctl-b:#b3bac3;
  --tx:#1b2029; --tx2:#5a6371; --tx3:#8b94a1; --ink:#3c4858;
  --ok:#127a2d; --ok-bg:#eaf6ec; --ok-b:#bfe0c6; --danger:#c42b1c; --danger-bg:#fdeeec; --danger-b:#f3c2bd;
  --warn-bg:#fff6e0; --warn-b:#f2dc9e;
  --sel:#e7f0fc; --sel-b:#a8c8f0; --hover:#f0f3f7;
  --font:'Segoe UI Variable Text','Segoe UI',system-ui,-apple-system,'Helvetica Neue',Arial,sans-serif;
  --mono:'Cascadia Mono',Consolas,'Courier New',monospace;
}
```

- **Chỉ có một theme (sáng).** Không có `prefers-color-scheme`, không có `[data-theme]`, không có theme tối.
- Số lần dùng `var(--x)` trong file: `--accent` 60, `--line` 42, `--tx2` 35, `--tx3` 29, `--tx` 24, `--line2` 20, `--panel` 15, `--sel` 15, `--hover` 13, `--danger` 13, `--ok` 12, `--ctl` 7, `--font` 7, `--ink` 6, `--accent-a` 4, `--mono` 4, `--danger-bg` 4, `--ok-bg` 3, `--ok-b` 3, `--danger-b` 3, `--sel-b` 3, `--surface` 2, `--view` 2, `--ctl-b` 2, `--accent-h` 1, `--bg` 1, `--warn-bg` 1, `--warn-b` 1, `--orange` 1.
- **Biến khai báo nhưng không dùng:** `--brand2`, `--star` (0 lần). `--orange` chỉ dùng trong `.nhan-pt` (L291), mà class này không xuất hiện trong markup.

### 2.2 Nền tảng (L23–31) — chép nguyên văn

```css
*{box-sizing:border-box}
html,body{margin:0;height:100%}
body{background:#b8c0cb;font:13px/1.4 var(--font);color:var(--tx);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;padding:18px 16px;-webkit-font-smoothing:antialiased}
.ghichu{margin:0;font-size:12px;color:#444c58}
button,input,select{font:inherit;color:inherit}
a{color:var(--accent);text-decoration:none} a:hover{text-decoration:underline}
.an{display:none!important}
svg.i{width:16px;height:16px;fill:none;stroke:currentColor;stroke-width:1.75;stroke-linecap:round;stroke-linejoin:round;flex-shrink:0}
svg.i.nho{width:14px;height:14px}
```

- **Cỡ chữ gốc:** `13px`, `line-height:1.4`, `-webkit-font-smoothing:antialiased`.
- `body{background:#b8c0cb; … padding:18px 16px}` và `.ghichu` là **phông nền trình diễn** quanh cửa sổ giả, không thuộc app thật.
- **Font:** chỉ dùng font hệ thống. Văn bản mẫu dùng `'Times New Roman',Times,serif` (L223). Không nhúng font nào.

### 2.3 Thang cỡ chữ (giá trị `font-size` xuất hiện trong CSS)

`10px` · `10.5px` · `11px` · `11.5px` · `12px` (nhiều nhất, 32 chỗ) · `12.5px` · `13px` (gốc) · `13.5px` · `14px` · `14.5px` · `15px` · `16px` · `18px` · `22px` · `24px`.
Font rút gọn: `600 11px/1.5` (nhãn trang L200), `700 7px/1.1` (dấu son L218), `9px/1.38` (chữ trong khung ký L219), `12.5px/1.7 var(--mono)` (nhật ký L316), `11px/16px` (`kbd` L443), `700 10.5px/1` (nút `!` L467), `400 12.5px/1.6` (bong bóng tooltip L471).
`font-weight`: 400, 500 (1 chỗ, L257), 600 (chủ đạo cho nhấn mạnh), 700.

### 2.4 Bán kính (`border-radius`)

| Giá trị | Dùng cho (ví dụ) |
|---|---|
| `10px` | cửa sổ `.cuaso` (L35), `.popup` (L383), `.lt-menu` (L248), công tắc `.sw` (L415) |
| `9px` | `.menu` (L55), `.loc-menu` (L168), huy hiệu `.nut .huy` (L102) |
| `8px` | `.khung` (L120), `.the` (L412), `.bao` (L206), `.rowmenu` (L426), `.hd-the` (L440), bong bóng tooltip (L471), `.nc-panel` (L485), `.ab-file` (L362) |
| `7px` | `.pc` (L408), `.bang-tk` (L427) |
| `6px` | **chuẩn cho control**: `.nut`, `.fld`, `.mb`, `.mi`, `.seg-nhom`, `.chip`, `.nutma`, `.tim`, `.lt-nut`… |
| `5px` | `.o .xoa`, `.lk.tron`, `.rail-stt`, `.loc-nut`, ô số `.dieuhuong input.so`, `.mitem` |
| `4px` | `.tag-pill`, `.pill-ok`, `.bchip`, `kbd`, `.trung-dong .co` |
| `3px` / `2px` / `1px` | thanh tiến độ, khung chữ ký, tay nắm, trang giấy nhỏ |
| `12px` | thẻ Trang chủ `.tc-the` (L333) |
| `20px`, `999px`, `50%` | pill đếm (L175), nhãn trang/thanh nổi, hình tròn |

### 2.5 Bóng đổ (`box-shadow`) — nguyên văn

| Giá trị | Dùng ở |
|---|---|
| `0 20px 56px rgba(16,32,64,.28),0 3px 10px rgba(16,32,64,.12)` | `.cuaso` L35 |
| `0 28px 64px rgba(16,32,64,.3)` | `.popup` L383 |
| `0 14px 32px rgba(16,32,64,.18)` | `.menu` L55 |
| `0 10px 30px rgba(16,32,64,.18)` | `.lt-menu` L248 |
| `0 10px 24px rgba(16,32,64,.18)` | `.rowmenu` L426 |
| `0 8px 24px rgba(16,32,64,.16)` | `.loc-menu` L168 |
| `0 12px 28px rgba(16,24,40,.28)` | bong bóng tooltip L471 |
| `0 8px 22px rgba(10,84,184,.14)` | `.tc-the:hover` L337 |
| `0 4px 14px rgba(16,32,64,.12)` | `.tb-noi` L515 |
| `0 2px 12px rgba(16,32,64,.16)` | trang giấy `.giay` L198 |
| `0 2px 6px rgba(16,32,64,.12)` | `.giay > .nhan-trang` L454 |
| `0 1px 3px rgba(16,32,64,.1)` | `.mini` L354, `.vt-trang` L502 |
| `inset 0 -1px 0 rgba(0,0,0,.04)` | `.fld` L76 |
| `inset 0 -2px 0 var(--accent)` | `.fld:focus` L78 |
| `inset 0 0 0 1px var(--accent)` | `.pc.chon` L410 |
| `0 0 0 2px var(--sel)` | `.lt-nut.mo` L244 |
| `0 0 0 3px rgba(31,154,63,.15)` | chấm xanh `.ketnoi i` L53 |
| `0 0 0 3px rgba(245,160,0,.45)` | khung chữ ký đang chọn `.chuky.noi` L217 |
| `inset 3px 0 0 var(--accent)` | `.dong.dang` L138 — **bị huỷ** bởi L522 |

### 2.6 Lớp chồng (`z-index`)

`1` (đầu bảng dính) · `2` (khung chữ ký, tay nắm) · `3` (thanh kéo cột, vạch tiến độ) · `4` (nhãn trang) · `5` (nút thu gọn) · `6` (`.tb-noi`) · `20` (thẻ Trang chủ khi hover) · `30` (`.loc-menu`, `.rowmenu`) · `40` (`.menubar`, `.lt-menu`) · `50` (`.menu`) · `60` (`.phu` — lớp phủ hộp thoại) · `70` (`.phu.tren` — hộp con) · `95`/`96` (bong bóng và mũi tên tooltip).

### 2.7 Chuyển động

- `transition`: `width .25s` (tiến độ L299, L303) · `border-color .15s,box-shadow .15s,transform .15s` (thẻ Trang chủ L333) · `stroke .15s` (L334) · `left .12s` (công tắc L416) · `color .12s,background .12s` (nút `!` L467) · `transform .15s` (mũi tên Nâng cao L482) · `background .1s,border-color .1s` (ô vị trí L505).
- `@keyframes quay{to{transform:rotate(360deg)}}` dùng `.quay{animation:quay .9s linear infinite}` (L147).
- `@keyframes ncMo{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:none}}` dùng `animation:ncMo .14s ease-out` (L485, L490).
- Hoạt ảnh logo chạy bằng `requestAnimationFrame` (mục 6.3.11).

### 2.8 Breakpoint

**Không có.** File không chứa `@media`. Co giãn duy nhất: `.cuaso{width:1280px;max-width:100%;height:800px;max-height:calc(100vh - 60px)}` (L34) và `.lt-menu{width:290px;max-width:84vw}` (L248). Bố cục bên trong dùng flex, không có điểm gãy.

### 2.9 Kích thước khung chính

| Thành phần | Giá trị | Dòng |
|---|---|---|
| Cửa sổ `.cuaso` | `1280px × 800px`, viền `1px solid #c3c9d1`, bo `10px` | L34–35 |
| Thanh tiêu đề `.tieude` | cao `32px`; nút `.cap` `46×32` | L36, L40 |
| Thanh menu `.menubar` | cao `36px`; nút `.mb` cao `28px`; menu `min-width:262px`; mục `.mi` cao `32px` | L45–56 |
| Thân màn `.man` | `padding:10px 12px 0`, `gap:8px` | L65 |
| Hàng nhập `.hang` | `min-height:32px`, nhãn `.nhan` rộng `132px` | L66–67 |
| Ô nhập `.fld` | cao `30px`; `.fld.so` rộng `56px`; `.fld.nho` cao `28px` | L76, L82, L238 |
| Nút `.nut` | cao `30px`, `min-width:74px`; `.nho` cao `26px`; `.rong` `min-width:120px` | L91, L100–101 |
| Hàng tab `.tabs` | cao `38px` | L121 |
| Thanh điều hướng `.dieuhuong` | cao `40px` | L187 |
| Đáy cột trái `.daytrai` | cao `38px` | L148 |
| Đầu danh sách `.ds-dau` | cao `30px`; dòng `.dong` `min-height:34px` | L134–135 |
| Rail `.rail-ds` | rộng `42px`; nút số `30×24` | L153, L157 |
| Chân màn `.chan` | cao `50px` | L294 |
| Trang giấy | `520×736` px ở 100% (`.giay-in` L199; `trang()` L924) | |
| Vùng xem `.vungxem` | `padding:22px`, `gap:16px`, nền `var(--view)` | L195 |
| Hộp thoại | đầu `48px` (L384), thân `padding:16px 18px` (L387), chân `padding:12px 18px` (L388), `max-height:calc(100% - 36px)` (L383) | |
| Hộp Cấu hình | `1150px × 684px` | L400 |
| Thẻ Trang chủ | `162×146`, khoảng cách `22px`; logo rộng `78px`; chân `32px` | L325, L332–333, L343 |

### 2.10 Màu viết cứng (chưa thành biến) — nên cân nhắc khi lập `tokens.css`

| Màu | Vai trò | Dòng |
|---|---|---|
| `#1f9a3f` | xanh "đang hoạt động": chấm `.ketnoi`, số rail đã ký, tiến độ xong, dấu tick, `.icon-lon.ok`, chấm hộp Ghép trường | L53, L159, L300, L304, L367, L395, L1483 |
| `#c42b1c` | đỏ nút Đóng khi hover, viền dấu son (trùng `--danger`) | L41, L218, L1554 |
| `#232b36` | nền bong bóng tooltip | L470–475 |
| `#161a20` / `#7b8491` / `#d0d5dc` | nền / giờ / chữ nhật ký | L316–320 |
| `#9aa3ae`, `#ff9aa5`, `#6ccb7f`, `#7fb2ff`, `#fff` | màu nhãn nhật ký INFO+TOKEN / LỖI / OK / … / XONG (viết trong JS) | L986, L996–1002 |
| `#0a3d91` | chữ thông tin trong khung chữ ký | L219 |
| `#fff4ce` `#f2e2a8` `#7a5c00` | biến trung tính `.bien`; dòng tiêu đề Excel | L227, L276 |
| `#e6f6ec` `#a9dfbb` `#1c7a3b` | biến đã ghép (`.da`) | L228, L233 |
| `#ffe9cc` `#f3c98f` `#9a5300` | biến chưa ghép (`.chua`) | L229, L234 |
| `#7a1b12` / `#13461f` / `#5c4300` | chữ trong `.bao.loi` / `.bao.ok` / `.bao.canh` | L209–211 |
| `#868e98` `#f6f7f9` `#5a6371` | công tắc `.sw` khi tắt | L415–416 |
| `#b7c1ce` `#a7b2bf` | nút chính khi `disabled` | L99 |
| `#e6eaf0`, `#f4f5f7`, `#fbfcfd`, `#fafbfc` | nền nút nhấn / disabled / readonly / hover thẻ | L93, L80, L94, L79, L409, L443 |
| `#f0f2f5`, `#f3f5f8`, `#dde2e8`, `#e3e6ea`, `#d5d9df`, `#cfd4da` | đường kẻ phụ, đầu bảng Excel, rãnh tiến độ, vạch trang nhỏ, vách chân trang, viền `kbd` | L135, L273, L298, L356, L345, L443 |
| `#b4bcc6`, `#b5bcc5`, `#4a5462`, `#9aa2ae`, `#aab2bd` | icon trạng thái rỗng, chấm tắt, icon menu, viền nút `!`, viền ô vị trí | L185, L54, L48, L467, L505 |
| `#FFD06A` | vệt sao trong hoạt ảnh logo | L885–886 |
| `#f5a000` | vạch trái dòng tiêu đề Excel (bằng giá trị `--star`) | L277 |
| `#8a5a00` | cảnh báo "Chỉ nên lưu PIN…" | L1613 |
| `rgba(16,24,40,.32)` | lớp phủ hộp thoại | L381 |
| `rgba(10,84,184,.05)` / `.08` / `.1` / `.3` | nền/viền khung chữ ký (cùng sắc `--accent`) | L214–215, L357, L449 |

---

## 3. Danh sách màn hình và hộp thoại

Điều hướng: `S.screen` (L642) quyết định hàm vẽ trong `render()` (L733–741); `go(screen)` (L751–757) đổi màn. `S.modal` quyết định hộp thoại (`modalHTML()` L1430).

Bộ khung chung của 4 màn chức năng (Ký số, Tạo văn bản, Kiểm tra, Vị trí): `.man` → 1–3 hàng nhập `.hang` → (tuỳ) `.nc-panel` Nâng cao → `.khung` (hàng `.tabs` + `.than` = `.cottrai` | `.keo` | `.phai`) → `.chan` (chân màn 50px: thông tin bên trái, nút bên phải).

### 3.1 Trang chủ — `#manHome` · `manHome()` L940–954 · CSS L323–348, L456–458

- **Bố cục:** cột giữa màn, nền trắng. Trên: logo động + tiêu đề. Giữa: hàng 4 thẻ. Dưới cùng (dính đáy): thanh chân `.tc-day`.
- **Đầu `.tc-dau`** (L947–948):
  - Logo `<a class="logo-dinh" href="https://saomaiso.com" target="_blank" rel="noopener" title="saomaiso.com">` chứa `logoDongHTML()`.
  - `<h1>` "Ký số văn bản điện tử".
  - `<p>` "Chọn chức năng để bắt đầu" + tooltip `!` (nguyên văn ở Phụ lục A, L948).
- **4 thẻ `.tc-the`** (mảng `cards` L941–946; mỗi thẻ là `<button data-act="go" data-arg="…">` chứa nút `!`, icon lớn `la()`, tên đậm):

| `id` (đích) | Icon `LA` | Tên thẻ | Phím | Mô tả tooltip (`tom`) |
|---|---|---|---|---|
| `kyso` | `ky` | "Ký số văn bản" | `Ctrl+K` | "Ký 1 file hoặc cả thư mục PDF/DOCX bằng USB Token; chèn ảnh chữ ký, con dấu theo khổ giấy; tự nhập lại PIN khi token hết phiên." |
| `tvb` | `tao` | "Tạo văn bản" | `Ctrl+N` | "Sinh hàng loạt văn bản từ mẫu DOCX/HTML và dữ liệu Excel (hoặc nhập trực tiếp); xuất PDF hoặc DOCX." |
| `kt` | `kiemtra` | "Kiểm tra chữ ký" | `Ctrl+E` | "Xác minh chữ ký trong file PDF: người ký, thời điểm ký, chứng thư và văn bản có bị sửa sau khi ký." |
| `tucauhinh` | `vitri` | "Vị trí chữ ký" | (không) | "Kéo thả, đổi cỡ khung chữ ký trên trang — một khung cho tất cả hoặc riêng từng file." |

  Phím tắt **không hiện trên thẻ**; nó được nối vào cuối tooltip: `c.tom+(c.k?'\nPhím tắt: '+c.k:'')` (L950). CSS `.tc-the .phim` (L336) có nhưng markup không sinh phần tử này.
- **Chân `.tc-day`** (L951–952): trái "saomaiso.com" (link `https://saomaiso.com`) | "hotro@saomaiso.com" (link `mailto:`); phải "Phiên bản" **"1.5.0"** + tooltip `!` "Người dùng: [Tên người dùng]\nTên PC: PC-KYSO-01\nBản quyền: Giải pháp số Sao Mai".
- **Trạng thái:** không có. Hover/focus thẻ: viền `--accent`, nâng 2px, icon đổi màu `--accent` (L337–338).

### 3.2 Ký số văn bản — `#manKySo` · `manKySo()` L1006–1111

**Hàng 1 (L1016–1018)**
- Nhãn "Văn bản cần ký:".
- Ô đường dẫn `#ksPath` (chỉ đọc, có nút xoá `ks.clear`): placeholder "Chọn 1 file hoặc cả thư mục cần ký"; giá trị mô phỏng `D:\VanBan\CanKy` (thư mục) hoặc `D:\VanBan\CanKy\QD_015_2026_SaoMai.pdf` (1 file).
- Nhóm nút đoạn `.seg-nhom`: "1 file" (`title="Chọn 1 văn bản"`, `ks.pick`/`file`) · "Thư mục" (`title="Chọn cả thư mục"`, `ks.pick`/`folder`). Nút tương ứng được tô `chon` sau khi nạp.
- Bên phải (196px): checkbox "Xóa file gốc" (`CHG ks.delOld`) + tooltip "Bật: thay file gốc bằng file đã ký.\nTắt: giữ file gốc; file đã ký lưu vào thư mục DaKySo, thêm hậu tố _signed."

**Hàng 2 (L1020–1021)**
- Nhãn "Chữ ký / con dấu:"; ô đường dẫn giá trị `D:\ChuKy\condau_saomai.png` (nút xoá là `noop`); nút "Chọn ảnh" rộng 176px (`noop`).
- Bên phải: nút "Cấu hình chữ ký" (`go`→`xemtruoc`, `title="Vị trí, cỡ khung chữ ký, kiểu hiển thị cho từng văn bản"`).

**Khung tab (L1023–1036)**
- Tab "Văn bản (n)" (`ks.tab`/`files`, icon `list`, n = số file đang hiện) · tab "Nhật ký" (`ks.tab`/`log`, icon `terminal`).
- Riêng màn này tab đang chọn **không gạch chân**, chỉ đổi màu chữ sang `--accent` (L305–307).
- Bên phải hàng tab (`.phai-tab`), khi đã chạy (`hasRun`): nhãn trạng thái `.td-tt` + "x/y":
  - "Đang ký x/y" (class `dang`, icon quay) · "Đã xong x/y" (class `xong`, icon `checkC`) · "Tạm dừng x/y" (class `dung`, icon `stop`).
  - Ở tab Nhật ký thêm 2 nút chữ: "Lưu nhật ký…" (`noop`) và "Xóa" (`ks.clearLog`).
- **Vạch tiến độ** `.td-line` cao 2px chạy dọc đáy hàng tab, `width = dPct%`; màu `--accent` khi chạy, `#1f9a3f` khi xong, `--tx3` khi dừng (L303–304, L1035).

**Tab Văn bản — cột trái (mở)** `.cottrai` rộng `S.leftW.kyso` = 340px (L1045–1061)
- Chưa nạp: "Chưa chọn văn bản" (icon `folderOpen`).
- Đầu bảng (lưới `28px minmax(0,1fr) 104px`): "#" · "Tên văn bản" · nút lọc "Trạng thái ▾".
- Dòng `.dong` (`ks.sel`): số thứ tự, icon (`doc` nếu `.docx`, ngược lại `file`), tên file, chip trạng thái. `title` = `"<tên> · <n> trang"`.
- **Chip trạng thái** (`chipTT()` L932–937): "Chờ ký" (`cho`, icon đồng hồ, màu `--tx3`) · "Đang ký" (`dang`, icon quay, `--accent`, đậm) · "Đã ký" (`ok`, `checkC`, `--ok`) · "Lỗi" (`loi`, `alert`, `--danger`).
- **Menu lọc** `.loc-menu` (L1054–1056): "Tất cả" · "Chờ ký" · "Đã ký" · "Lỗi" · "Đang ký", mỗi mục có pill đếm (`ksCount`). Khi đang lọc, nút đổi nhãn thành tên trạng thái, thêm icon phễu và màu `--accent`.
- Không có kết quả: "Không có văn bản ở trạng thái “…”" hoặc "Không có văn bản khớp “…”".
- Đáy `.daytrai` (L1060): ô tìm `#ksTim` placeholder "Tìm theo tên file" (disabled khi chưa nạp) · đếm "`n / tổng`" (khi đang tìm/lọc) hoặc "`tổng` văn bản" · nút thu gọn (`title="Thu gọn danh sách"`, icon `panelL`).
- Thanh kéo `.keo` (`title="Kéo để đổi độ rộng cột"`).

**Tab Văn bản — cột trái (thu gọn = rail)** `.rail-ds` rộng 42px (L1063–1070)
- Đầu "#"; danh sách nút số `.rail-stt` mang class `st-ok` / `st-loi` / `st-dang` / `st-cho` và `chon`; `title="<tên> · <trạng thái>"`; đáy là nút mở (`title="Mở danh sách văn bản"`, icon `panelR`).
- **Mặc định là rail:** `S.ks.listOpen:false` (L644) và `go('kyso')` luôn đặt lại `S.ks.listOpen=false` (L755). Tức là mỗi lần vào màn Ký số, danh sách ở dạng thu gọn.

**Tab Văn bản — cột phải** `.phai`
- Chưa nạp → panel giới thiệu (`introHTML`, L1074–1077): tiêu đề "Ký số văn bản" + tooltip; đoạn "Ký 1 văn bản hoặc cả thư mục bằng chứng thư số trên USB Token."; 3 bước: **1 "Chọn văn bản"** — "Bấm 1 file hoặc Thư mục." (2 chữ là link bấm được) · **2 "Cắm USB Token"** — "Ký bằng chứng thư mặc định trong Cấu hình." · **3 "Ký số"** — "Nhập mã PIN, xem kết quả từng file."
- Không có file khớp → "Không có văn bản khớp “…”".
- Có file → thanh `.dieuhuong` (L1083–1084): nút "Văn bản trước" · ô số `#ksSo` (`CHG ks.so`) · "/ n" · nút "Văn bản sau" · **tên file đậm** · tooltip `!` (`"<n> trang · ký trang cuối\nVị trí: mẫu Khổ A4 · dưới phải\nHiển thị: ảnh + thông tin"`) · chip trạng thái · vách · zoom.
- Vùng xem `#ksView` (L1085–1089):
  - Nếu lỗi: hộp `.bao.loi` "**Không ký được**" + lý do (`FAIL[i]`) + tooltip "Sửa lỗi rồi bấm Ký số để ký lại.\nFile lỗi được đặt hậu tố _loi."
  - Trang giấy mô phỏng (`docQD()`), nhãn phủ góc trên phải: "Đã ký số" (`ok`) · "Lỗi" (`loi`) · "Đang ký…" (`dang`) · không có nhãn khi chờ.
  - Khung chữ ký (`sigBox`) theo `ksRect(i)`; **ẩn** khi kiểu hiển thị là "Ẩn" hoặc file lỗi; có class `da-ky` (viền liền, nền trắng mờ) khi đã ký.

**Tab Nhật ký** `#ksLog` (L1037–1040; dữ liệu từ `ksStats()` L984–1005): nền tối, chữ mono; mỗi dòng: giờ · nhãn (rộng 56px, có màu) · nội dung. Nhãn dùng: `INFO`, `TOKEN`, `LỖI`, `OK`, `…`, `XONG`. Tự cuộn xuống cuối sau mỗi lần vẽ (L746).

**Chân màn `.chan` (L1095–1109)** — phần chữ theo trạng thái:

| Trạng thái | Nội dung trái |
|---|---|
| Chưa nạp | "Chọn văn bản cần ký để bắt đầu." |
| Đang chạy | "Đã chạy **mm:ss** · còn khoảng **mm:ss**" (hoặc " · đang ước lượng…" khi chưa xong file nào) + "Đã ký **n**" + "Lỗi **n**" (chỉ hiện khi có lỗi) |
| Tạm dừng | "Tạm dừng · đã chạy **X** · còn khoảng **Y** cho n văn bản" + đếm |
| Sẵn sàng | "Sẵn sàng ký **n** văn bản" (+ " đang hiện" nếu ô tìm có chữ) + tooltip |
| Xong | "Đã ký xong **n** văn bản · tổng thời gian **X**" + đếm + tooltip |

Nút phải: "Mở thư mục" (chỉ khi đã xong; `noop`) · nút chính đổi nhãn: **"Ký số"** → **"Dừng"** (đang chạy, nút thường) → **"Ký tiếp"** (đã có kết quả, còn file chưa ký) → **"Ký lại"** (đã xong hết; `ks.resign`). Nút "Ký số" `disabled` khi chưa nạp hoặc không còn file.

### 3.3 Tạo văn bản — `#manTVB` · `manTVB()` L1182–1280

**Hàng 1 (L1186–1189)**
- Nhãn "Mẫu văn bản:" + tooltip; ô đường dẫn placeholder "Chọn mẫu .docx hoặc .html", giá trị mô phỏng `D:\Mau\GIAY_CHUNG_NHAN.docx`; nút "Chọn" (`tvb.pickTpl`).
- Bên phải (chỉ khi nguồn là Excel): "Sheet" + `<select>` (`Sheet1`/`Sheet2`/`Sheet3`, rộng 112px) · vách · "Dòng tiêu đề" + tooltip "Dòng chứa tên cột trong file Excel." + ô số (`min="1"`, rộng 66px) · nút mũi tên xem/ẩn bảng (`title="Xem dữ liệu Excel"` / `"Ẩn dữ liệu Excel"`). Cả ba `disabled` khi chưa chọn file dữ liệu.

**Hàng 2**
- Nguồn Excel (L1191–1192): "File dữ liệu:"; placeholder "Chọn file Excel hoặc CSV"; giá trị `D:\Mau\DanhSach_HocVien.xlsx`; nút "Chọn" (`tvb.pickData`); nút **Nâng cao**.
- Nguồn Nhập trực tiếp (L1201): "Ảnh chèn:" + nút "Ảnh thẻ…" + nút "Con dấu…" (cả hai `noop`) + nút **Nâng cao**.

**Bảng xem nhanh Excel `.xls-bang`** (L1193–1199; khi `xlsOpen` và đã có dữ liệu): đầu "**DanhSach_HocVien.xlsx**" + "`Sheet1 · 12 dòng · dòng tiêu đề 1`" + link "Đóng"; lưới cuộn (cao tối đa 224px): cột "#" + 7 cột có khoá (`Họ tên`, `Ngày sinh`, `Đơn vị công tác`, `Khóa đào tạo`, `Xếp loại`, `Số hiệu`, `Ngày cấp`; độ rộng theo `COLW`). Bấm một dòng = chọn bản ghi (`tvb.row`). Dòng có số thứ tự bằng "Dòng tiêu đề" được tô vàng (class `tieude`).

**Panel Nâng cao `.nc-panel`** (L1203–1209, khoá `S.adv.tvb`):
- "Nguồn dữ liệu": nút đoạn "Từ Excel" · "Nhập trực tiếp".
- "Định dạng": nút đoạn "PDF" · "DOCX".
- "Tên file" + tooltip; ô `#tvbTen` (mặc định `{so}_{hoten}`).

**Khung tab (L1210–1212)**
- Tab "Mẫu (x/9 biến)" khi đã sẵn sàng và nguồn Excel; ngược lại nhãn là "Mẫu gốc". Tab "Kết quả (12)". Tab mặc định: `ketqua` (L645).
- Bên phải (chỉ ở tab Mẫu khi sẵn sàng): checkbox "Tô màu trường".

**Cột trái** (rộng `S.leftW.tvb` = 360px) có 4 dạng:
1. Chưa sẵn sàng: "Chưa có dữ liệu"; ô tìm placeholder "Tìm" (disabled).
2. Excel + tab **Mẫu** (L1219–1237): bảng "Biến" | "Lấy từ" | (trạng thái). 9 biến (`TVB_VARS`): `{hoten}`, `{ngaysinh}`, `{donvi}`, `{khoahoc}`, `{xeploai}`, `{so}`, `{ngaycap}`, `{anh_the}`, `{con_dau}`. Chip biến xanh (`da`) nếu đã ghép, cam (`chua`) nếu chưa. Nút nguồn hiện tên cột / giá trị cố định / "(giá trị cố định)" / "Chọn…". Cột cuối: tick xanh hoặc chấm đỏ "!" (`title="Chưa ghép nguồn"`).
   - **Menu chọn nguồn `.lt-menu`** (rộng 290px; 3 biến cuối mở ngược lên): ô `#tvbPickInp` placeholder "Gõ giá trị cố định rồi Enter…"; nhóm "Cột trong file dữ liệu"; 10 cột (`TVB_COLS`), mỗi cột kèm số giá trị khác nhau (hoặc "—"); nút "Bỏ ghép" (chỉ khi đã ghép).
   - Đáy: "Chưa ghép n biến" (đỏ) hoặc "Đã ghép đủ 9 biến" (xanh).
3. Excel + tab **Kết quả** (L1238–1245): bảng "#" | "Tên file sẽ tạo"; ô tìm `#tvbTim` placeholder "Tìm theo tên file / nội dung"; đếm "`n / 12`" hoặc "`12 dòng`"; nút thu gọn. Không khớp: "Không có dòng khớp". Cột này thu gọn được thành rail số (chỉ ở tab Kết quả hoặc khi chưa sẵn sàng — biến `canCol` L1213).
4. Nhập trực tiếp (L1247–1249): form `.form-truong` 7 ô theo `TVB_FIELDS`: "Họ tên", "Ngày sinh", "Đơn vị công tác", "Tên khóa đào tạo", "Xếp loại", "Số hiệu", "Ngày cấp".

**Cột phải**
- Chưa sẵn sàng → panel giới thiệu (L1264–1266): "Tạo văn bản từ mẫu"; "Điền dữ liệu vào mẫu để sinh hàng loạt văn bản."; bước **1 "Chọn mẫu"** — "Bấm Chọn mẫu .docx/.html." · **2 "Chọn dữ liệu"** — "File Excel hoặc nhập trực tiếp." · **3 "Tạo văn bản"** — "Xem trước từng văn bản rồi bấm Tạo."; dòng thêm: "hoặc mở ví dụ mẫu" (`tvb.demo`).
- Sẵn sàng → `.dieuhuong` (L1269–1271): (tab Kết quả + Excel) nút trước/sau, ô số `#tvbSo`, "/ 12"; tên file đậm (tab Mẫu: "GIAY_CHUNG_NHAN.docx" + tooltip "Biến xanh = đã ghép nguồn, cam = chưa ghép.\nBấm biến trên trang để sửa nguồn."); zoom. Vùng xem `#tvbView`: trang "GIẤY CHỨNG NHẬN" (`docCN()` L1170–1181) — tab Mẫu hiện `{biến}` (tô màu nếu bật "Tô màu trường"), tab Kết quả hiện giá trị.

**Chân màn (L1275–1278):** "Chọn mẫu văn bản và nguồn dữ liệu để bắt đầu." hoặc "Sẽ tạo **n** văn bản **PDF**" + tooltip. Nút chính "Tạo văn bản" (disabled) / "Tạo n văn bản" (`tvb.create`).

### 3.4 Kiểm tra chữ ký số — `#manKT` · `manKT()` L1311–1356

**Hàng 1 (L1313–1314):** "Văn bản cần kiểm tra:"; placeholder "Chọn file PDF đã ký số"; giá trị `D:\VanBan\CanKy\DaKySo\QD_001_2026_SaoMai_signed.pdf`; nút "Chọn file" (`kt.pick`). Bên phải (210px): chưa nạp → tooltip "Hỗ trợ PDF có một hoặc nhiều chữ ký, kể cả ký nháy."; đã nạp → chỉ báo xanh "n chữ ký hợp lệ" + nút tròn `title="Kiểm tra lại"` (`kt.pick`).

**Tab:** "Chữ ký (n)" (`sig`, icon `shield`) · "Văn bản" (`vb`, icon `doc`).

**Tab Chữ ký**
- Cột trái (rộng `S.leftW.kt` = 300px, L1318–1320): chưa nạp "Chưa chọn văn bản"; đã nạp: mỗi dòng cao tối thiểu 54px gồm icon tick xanh, tên người ký, thời điểm (dòng phụ).
- Cột phải: chưa nạp → giới thiệu (L1323–1324): "Kiểm tra chữ ký số"; "Xác minh chữ ký số trong văn bản PDF."; bước "Chọn văn bản" / "Xem từng chữ ký" — "Hợp lệ hay không, ai ký, lúc nào." / "Xuất báo cáo" — "Lưu kết quả ra PDF."
- Đã nạp (L1326–1340): `.dieuhuong`: nút trước · "Chữ ký **i** / n" · nút sau · nút mắt `title="Xem vị trí trên văn bản"` (chuyển tab Văn bản). Thân (rộng tối đa 640px):
  - Hộp `.bao.ok`: "**Chữ ký hợp lệ**" + tooltip "Văn bản không bị chỉnh sửa sau khi ký.\nChứng thư còn hiệu lực tại thời điểm ký."
  - Thẻ: "Người ký", "Đơn vị", "Thời điểm ký"; nút **"Chi tiết chứng thư"** (nút Nâng cao, khoá `S.adv.kt`).
  - Mở rộng: "Vai trò", "Lý do ký", "Vị trí chữ ký", "Nhà cung cấp CA", "Số serial", "Hiệu lực" ("Đến [dd/mm/yyyy]"); tiêu đề "Chuỗi chứng thư" + 3 dòng thụt dần 18px/bậc, mỗi dòng có tick xanh: "[Root CA quốc gia]" → "[Nhà cung cấp CA]" → tên người ký.

**Tab Văn bản (L1344–1349):** chưa nạp → giới thiệu (bước "Chọn văn bản" / "Xem vị trí" — "Khung cam là chữ ký đang chọn." / "Đối chiếu" — "Xem chi tiết ở tab Chữ ký."). Đã nạp: `.dieuhuong` "Trang **1** / 1" + tooltip + zoom; trang giấy có nhãn "Toàn vẹn" (`nhan-trang ok`) và 2 khung chữ ký `da-ky`; khung đang chọn có thêm class `noi` (quầng cam `rgba(245,160,0,.45)`).

**Chân màn (L1353–1354):** "Chọn file PDF đã ký để kiểm tra." hoặc tick + "Văn bản toàn vẹn · **n/n** chữ ký hợp lệ". Nút "Xuất báo cáo" (`noop`, disabled khi chưa nạp) · nút chính "Kiểm tra" (`kt.pick`) / "Kiểm tra file khác" (`kt.clear`).

**Trạng thái:** prototype **chỉ vẽ trường hợp tất cả hợp lệ**. Không có giao diện cho chữ ký không hợp lệ, văn bản bị sửa, chứng thư hết hạn/thu hồi, không xác định.

### 3.5 Xem trước & vị trí chữ ký — `#manTC` · `manTC()` L1384–1413

Dùng cho cả `S.screen==='xemtruoc'` (vào từ nút "Cấu hình chữ ký" ở màn Ký số) và `'tucauhinh'` (vào từ menu / thẻ Trang chủ). Khác nhau duy nhất: `go('xemtruoc')` đặt `S.adv.tc=true` (L753) nên panel Nâng cao mở sẵn; và dấu ✓ trên menu (mục 5).

**Hàng 1 (L1388–1391):** nút nhỏ "Quay lại" (`go`→`kyso`) · tiêu đề "Xem trước & vị trí chữ ký" · "Chế độ" + `<select>` ("Tất cả file" = `all`, "Từng file" = `per`; rộng 150px) · tooltip · vách · "Cỡ" + thanh trượt (`min="50" max="200" step="10"`, rộng 110px) + "n%" · nút **Nâng cao**.

**Panel Nâng cao (L1392–1402, khoá `S.adv.tc`):** "Theo mẫu" + tooltip + `<select>` các mẫu (tên + " (mặc định)") · "Vị trí theo mẫu" + `<select>` 6 vị trí ("Trên trái", "Trên giữa", "Trên phải", "Dưới trái", "Dưới giữa", "Dưới phải") · "Hiển thị" + `<select>` ("Ảnh + thông tin", "Chỉ ảnh", "Chỉ thông tin", "Ẩn").

**Khung:**
- Cột trái (rộng `S.leftW.xt` = 480px): lưới `#tcGrid` 8 ô `.ab-file` (124px; trang nhỏ 98×138 có vạch chữ, khung chữ ký nét đứt; dấu tick xanh ở góc nếu file có khung riêng trong chế độ Từng file; tên file bên dưới).
- Cột phải: `.dieuhuong`: nút trước · tên file đậm · nút sau · zoom (mặc định 66%). Vùng xem `#tcView`: (nếu kiểu là "Ẩn") hộp `.bao.canh` "**Chữ ký đang để ẩn — khung chỉ để tham khảo**"; trang `#tcPage` với khung `#tcSig` kéo được + 4 tay nắm góc.

**Chân màn (L1410–1411):** icon khung + "X **n%** · Y **n%** · **W × H%**" (+ " · đặt riêng **k/8** file" ở chế độ Từng file). Nút "Đặt lại" (`tc.reset`) · "Thoát" (`go`→`kyso`) · nút chính "Áp dụng" (`tc.apply`).

### 3.6 Hộp thoại

Khung chung `popup(o)` (L1426–1429): lớp phủ `.phu` (bấm nền để đóng — `phu.nen`), `.popup[role=dialog][aria-label]`, đầu `.dau-p` (icon + tiêu đề + nút ✕), thân, chân `.cuoi` (phần tử có class `trai` bị đẩy sang trái).

| # | Mã `S.modal` | Hàm · dòng | Tiêu đề · rộng | Mở từ | Nội dung · nút |
|---|---|---|---|---|---|
| 1 | `pin` | `mPin()` L1447–1454 | "Nhập mã PIN" · 430 | `ks.sign` (L1139), `dup.overwrite`, `dup.keep` | Thẻ token: tên chứng thư, "Ký n văn bản đang hiện", tooltip. Ô "Mã PIN" `#pinO` (`type=password`, **điền sẵn `12345678`**), nút mắt `title="Hiện/ẩn mã PIN"`. Checkbox đã tích "Ghi nhớ trong phiên làm việc này". Nút "Hủy" · "Xác nhận ký" (`pin.ok`) |
| 2 | `trung` | `mTrung()` L1455–1467 | "File đã ký / trùng tên" · 560 | `ks.sign` khi có file trùng; `ks.resign` | Hộp đỏ "**n** file đã ký trùng tên trong thư mục `<outDir>`. Bạn muốn xử lý thế nào?" (hoặc "…trong ghi đè ngay lên file gốc" khi bật Xóa file gốc). Danh sách file + nhãn "đã tồn tại". Chú giải "**Ghi đè**: thay file đã ký cũ bằng bản mới." / "**Giữ cả hai**: tạo bản mới, thêm hậu tố (1), (2)… — không đụng file cũ." Nút "Hủy" (trái) · "Giữ cả hai" · "Ghi đè" (chính) |
| 3 | `done` | `mDone()` L1468–1474 | "Ký số" · 440 | Tự mở khi chạy xong (L1124) | Vòng tick xanh, "Hoàn thành ký số"; 3 số lớn "Tổng số" / "Thành công" / "Lỗi"; "Đã lưu vào thư mục DaKySo" + tooltip. Nút "Nhật ký" (trái, `done.log`) · "Mở thư mục" · "Đóng" (cả hai chỉ đóng hộp) |
| 4 | `taofile` | `mTaoFile()` L1475–1479 | "Tạo văn bản" · 470 | `tvb.create` | "Tạo văn bản thành công"; "Đã tạo **n** văn bản PDF từ mẫu GIAY_CHUNG_NHAN.docx" / "Lưu tại D:\VanBan\KetQua". Nút "Ký số các văn bản này" (trái, `tvb.kySo`) · "Mở thư mục" · "Đóng" |
| 5 | `ghep` | `mGhep()` L1480–1488 | "Ghép trường" · 560 | **Không có nơi nào mở** (`ACTS['tvb.ghep']` L1298 không được gắn vào phần tử nào) | "Mỗi trường trong mẫu lấy giá trị từ một cột Excel. Trường khớp tên cột được ghép tự động."; lưới "Trường trong mẫu" → "Cột Excel"; mục "Khung ảnh"; chân "● xanh = đã ghép · ● đỏ = chưa ghép"; nút "Xong" |
| 6 | `huongdan` | `mHuongDan()` L1512–1520 | "Hướng dẫn sử dụng" · 940×620 | Menu Hỗ trợ, `F1` | Trái: 7 mục (`HD` L1491–1511): "Bắt đầu nhanh", "Ký số văn bản", "Vị trí chữ ký", "Tạo văn bản từ mẫu", "Kiểm tra chữ ký", "Cấu hình", "Câu hỏi thường gặp". Phải: "Hướng dẫn › <mục>", tiêu đề, đoạn dẫn, các thẻ `.hd-the` (`ol`/`ul`). Chân: "Phím tắt: F1 Hướng dẫn · Ctrl+K Ký số · Ctrl+N Tạo văn bản · Ctrl+E Kiểm tra"; nút mục trước / mục sau (ghi tên mục); "Đóng" |
| 7 | `capnhat` | `mCapNhat()` L1521–1526 | "Kiểm tra cập nhật" · 480 | Menu Hỗ trợ | "Đã có phiên bản mới 1.5.0"; "Đang dùng 1.4.2 · phát hành 28/10/2026"; hộp "Có gì mới" 4 gạch đầu dòng; "Tải từ máy chủ cập nhật của Sao Mai; không gửi dữ liệu văn bản nào." Nút "Để sau" · "Tải và cài đặt" (cả hai chỉ đóng) |
| 8 | `thongtin` | `mThongTin()` L1527–1532 | "Thông tin phần mềm" · 460 | Menu Hỗ trợ | Logo đầy đủ (rộng 150px). Lưới: "Ứng dụng" **Sao Mai Ký số** · "Phiên bản" "1.5.0 (build 2026.10.28)" · "Chức năng" "Ký số · Tạo văn bản · Kiểm tra chữ ký" · "Đơn vị" "Giải pháp số Sao Mai" · "Website" saomaiso.com · "Email hỗ trợ" hotro@saomaiso.com · "Giấy phép" "Bản quyền thương mại · [mã giấy phép]". Cuối: "© 2026 Giải pháp số Sao Mai. Sử dụng chữ ký số theo quy định pháp luật Việt Nam về giao dịch điện tử." Nút "Đóng" |
| 9 | `driver` | `mDriver()` L1533–1539 | "Cài đặt driver USB Token" · 500 | Menu Hỗ trợ | "Driver được nhận diện khi cắm USB Token. Cài driver đúng loại token của đơn vị." 2 dòng mô phỏng: "[Tên USB Token]" — "Đã cài · phiên bản [x.y]" — chỉ báo "Đang hoạt động"; "[USB Token 2]" — "Chưa cài" — nút "Tải driver" (không có handler). Nút "Đóng" |
| 10 | `cauhinh` | `mCauHinh()` L1631–1638 | "Cấu hình" · 1150×684 | Nút menu "Cấu hình" | Xem 3.7 |
| 11 | (con) `S.tokenDetail>=0` | `cfgSub()` L1605–1609 | "Chi tiết chứng thư số" · 470 | Menu ⋯ → "Chi tiết…" | Lưới: "Tên chứng thư", "Nhà cung cấp", "Số serial", "Hiệu lực đến", "Thiết bị", "Chủ sở hữu", "Trạng thái". Nút "Kiểm tra token" (không handler) · "Đóng" |
| 12 | (con) `S.tokenSetting>=0` | `cfgSub()` L1610–1623 | "Cài đặt tự động ký" · 510 | Menu ⋯ → "Cài đặt tự động ký…" | Tên token; công tắc "Lưu mã PIN trên máy này" + tooltip; (khi bật) ô "Mã PIN" `#pinCfg` + nút mắt + cảnh báo "Chỉ nên lưu PIN trên máy cá nhân."; công tắc "Tự nhập lại PIN khi hết phiên" (mờ 45% khi chưa bật lưu PIN); nút Nâng cao (`tkSet`) → "Số lần thử lại" (1/2/3 lần), "Khi nhập PIN lỗi" ("Tạm dừng và hiện hộp nhập PIN" / "Dừng hẳn tiến trình ký") — hai mục này chỉ hiện khi tự nhập lại đang bật; "Giữ phiên token" ("Theo cài đặt token" / "15 phút" / "30 phút" / "Đến khi đóng app") luôn hiện. Nút "Hủy" · "Lưu" (cả hai chỉ đóng) |
| 13 | (con) `S.tokenAdd` | `cfgSub()` L1624–1628 | "Thêm chứng thư số / USB Token" · 540 | Nút "Thêm token" | "Token phát hiện được" + tooltip + nút "Quét lại" (không handler); danh sách `AVAILABLE`: tên, CA, "Đã kết nối", nút "Thêm". Nút "Xong" |

Hộp con (11–13) dùng `.phu.tren` (z-index 70), đóng bằng `cfg.subClose` / bấm nền (`cfg.subNen`).

### 3.7 Hộp Cấu hình — `mCauHinh()` L1631–1638

- **3 tab** (`.tabch`): "Chữ ký theo khổ giấy" (`sig`, icon `stamp`) · "USB Token & mã PIN" (`token`, icon `usb`) · "Thư mục & tên file" (`folder`, icon `folder`). Mở lại ở tab dùng lần trước (`S.cfgTab`).
- **Chân:** link "Khôi phục mặc định" (trái, `cfg.reset`) · chú thích "Thay đổi áp dụng ngay khi bấm Lưu" → "✓ Đã lưu cấu hình" sau khi lưu · "Hủy" (`modal.close`) · nút chính "Lưu cấu hình" (`cfg.save`).

**Tab "Chữ ký theo khổ giấy" — `cfgSig()` L1558–1579** (3 cột):
1. Trái 220px: danh sách thẻ mẫu `.pc` (tên, nhãn "Mặc định", dòng phụ `"A4 dọc · dưới phải"`); nút "Thêm mẫu".
2. Giữa: tiêu đề = tên mẫu + tooltip ("Mẫu tự áp dụng cho các văn bản khổ A4.\nKhi ký, app chọn mẫu theo khổ giấy của từng văn bản."); link "Xóa mẫu" (chỉ khi có >1 mẫu và mẫu không phải mặc định). Lưới nhãn 110px:
   - "Khổ giấy": `<select>` "A1 (594 × 841)", "A2 (420 × 594)", "A3 (297 × 420)", "A4 (210 × 297)", "A5 (148 × 210)" + 2 chip "Dọc" / "Ngang".
   - "Vị trí": ô chọn 6 điểm (`viTriPick`) + nhãn vị trí viết hoa chữ đầu.
   - "Tỉ lệ ảnh": thanh trượt `min="40" max="200" step="10"` + "n%".
   - "Hiển thị": "Ảnh + thông tin người ký" / "Chỉ ảnh chữ ký / con dấu" / "Chỉ thông tin người ký" / "Ẩn (không hiện trên trang)".
   - Nút **Nâng cao** (`cfgSig`) → "Tên mẫu" (ô chữ) · "Trang ký" ("Trang cuối" / "Trang đầu" / "Tất cả các trang (ký nháy)") · "Cách lề (mm)": "Ngang" (`min="0" max="100"`) và "Dọc" (`min="0" max="150"`) · "Ảnh chữ ký": `<select>` (`condau_saomai.png`, `chuky_giamdoc.png`, `chuky_nhay.png`) + nút "Chọn…" (không handler).
3. Phải 240px: "Xem trước" — trang thu nhỏ (`cfgPreview`) + dòng `"A4 dọc · 100% · dưới phải"`.

**Tab "USB Token & mã PIN" — `cfgToken()` L1580–1591:** thẻ có tiêu đề "Chứng thư số / USB Token" + tooltip; nút "Làm mới" (không handler) · nút chính "Thêm token". Bảng `.bang-tk` (cột `minmax(0,1fr) 170px 120px 36px`): "Tên chứng thư" | "Nhà cung cấp" | "Trạng thái" | ⋯. Dòng đầu có nhãn "Đang dùng". Trạng thái: "Đã kết nối" (chấm xanh) / "Chưa cắm" (chấm xám). Menu ⋯ `.rowmenu`: "Chi tiết…" · "Cài đặt tự động ký…" · "Đặt làm mặc định" · (vách) · "Xóa khỏi danh sách" (đỏ).

**Tab "Thư mục & tên file" — `cfgFolder()` L1592–1602:** thẻ 2 dòng: "Thư mục lưu file đã ký" + tooltip + ô chữ (320px) + nút "Chọn…" (không handler) · "Hậu tố khi ký thành công" + tooltip + ô chữ (200px). Nút **Nâng cao** (`cfgFolder`) → "Hậu tố khi ký lỗi" + ô chữ · "Ghi đè khi trùng tên" + chữ "Bật"/"Tắt" + công tắc.

---

## 4. Component dùng chung

| Component | Class CSS (dòng) | Hàm JS (dòng) | Hành vi |
|---|---|---|---|
| **Thanh tiêu đề tự vẽ** | `.tieude`, `.ico-app`, `.ten`, `.cap`, `.cap.x` (L36–42) | `titlebarHTML()` L671–676; `TITLES` L670 | Icon 16px + "**Sao Mai Ký số** — <tên màn>" + 3 nút "Thu nhỏ" / "Phóng to" / "Đóng" (`aria-label`). **3 nút không có `data-act`** — chỉ để nhìn |
| **Menu bar** | `.menubar`, `.mb-o`, `.mb`, `.mb.mo`, `.menu`, `.mi`, `.mi.dang`, `.phim`, `.dau`, `.mi-sep`, `.mb-phai` (L45–62) | `menuDefs()` L677–697; `menubarHTML()` L699–723; `ACTS['menu.toggle' / 'menu.item' / 'menu.direct']` L815–817 | Xem mục 5 |
| **Chỉ báo token** | `.ketnoi`, `.ketnoi.tat`, `.ketnoi i` (L52–54) | trong `menubarHTML()` L720–721 | Icon USB + chấm + "USB Token" (xanh) hoặc "Chưa cắm token" (xám) + tooltip. Cũng dùng lại cho "n chữ ký hợp lệ", "Đang hoạt động", cột trạng thái token |
| **Tooltip `!`** | `.tip`, `.tip.mo`, `.tip.trai`, `.tip.tren` (L466–478) | `tip(t,cls,key)` L605–610; `ACTS['tip']` L812; `S.tip` | Hình tròn 17px chữ "!"; **bấm mới mở** (không hover). Nội dung lấy từ `data-tip` qua `::after{content:attr(data-tip)}`, `white-space:pre-line` (xuống dòng bằng `\n`), rộng 170–300px. Mặc định bong bóng ở dưới, canh phải; `trai` = canh trái; `tren` = mở lên trên. Chỉ một tooltip mở tại một thời điểm. Đóng: bấm lại, bấm ra ngoài (L788), `Esc` (L801). Có `role="button" tabindex="0" aria-label="Xem thông tin"`; `Enter`/`Space` kích hoạt (L802) |
| **Nút Nâng cao** | `.nut.nangcao`, `.mo`, `.mui` (L481–484); panel `.nc-panel` (L485–489, L497–498, L521); `.the-nc` (L492) | `advBtn(key,label)` L888–890; `ACTS['adv']` L813; `S.adv` | Nút nhỏ icon bánh răng + nhãn (mặc định "Nâng cao") + mũi tên xoay 180° khi mở; `aria-expanded`. Khoá đang dùng: `tc`, `tvb`, `kt` (nhãn "Chi tiết chứng thư"), `cfgSig`, `cfgFolder`, `tkSet`. Panel xuất hiện với hoạt ảnh `ncMo .14s` |
| **Cột trái kéo đổi rộng** | `.cottrai` (L130), `.keo`, `.keo.dang` (L131–132) | `mousedown` L824–838; `dragMove` L841; `S.leftW` | Thanh 7px giữa hai cột, con trỏ `col-resize`, hiện vạch `--accent` khi hover/kéo |
| **Rail thu gọn** | `.rail-ds`, `.rail-dau`, `.rail-ds-list`, `.rail-stt`, `.st-ok/.st-loi/.st-dang`, `.chon` (L153–160) | `ACTS['ks.panel']` L1133; `ACTS['tvb.panel']` L1289 | Cột 42px chỉ có số thứ tự, tô màu theo trạng thái |
| **Zoom** | `.zm` (L193), `.nutma` | `zoomBar(key)` L772–775; `ACTS['zoom']` L820 | Nút "Thu nhỏ" · "`n %`" · nút "Phóng to"; bước 10, giới hạn 40–160 |
| **Ô đường dẫn có nút xoá** | `.o`, `.o.trong`, `.o .xoa`, `.fld[readonly]` (L70–81) | `oInput(o)` L763–767 | Ô chỉ đọc + nút ✕ tròn (`title="Xoá đường dẫn"`); ẩn nút khi rỗng (`.o.trong`) |
| **Chip trạng thái** | `.chip-tt`, `.ok/.loi/.cho/.dang` (L144–146) | `chipTT(st)` L932–937; `spin()` L931 | Icon + chữ, không nền |
| **Nhãn trên trang** | `.nhan-trang` + `ok/loi/dang/cho` (L200–205, L454–455) | chuỗi trong `manKySo` L1087, `manKT` L1348 | Pill nổi mép trên phải trang giấy |
| **Hộp thông báo** | `.bao` + `loi/ok/canh` (L206–211) | — | Icon + chữ đậm + mô tả |
| **Trang giấy** | `.giay`, `.giay-in` (L198–199) | `trang(inner,z,id,over)` L923–926 | Khung `round(520·z) × round(736·z)`; nội dung 520×736 được `transform:scale(z)` |
| **Khung chữ ký** | `.chuky-o`, `.chuky`, `.da-ky`, `.noi`, `.keo-duoc`, `.dau-son`, `.tt-ky`, `.tay` (L214–220, L446–449) | `sigBox(r,o)` L915–922; `sigInner(mode)` L908–914 | Vị trí/cỡ theo %; viền nét đứt `--accent`; nội dung: dấu tròn đỏ "ĐÃ KÝ SỐ" và/hoặc khối chữ "**Ký bởi: [Tên chứng thư số]**", "Ngày ký: 24/10/2026 14:22", "Serial: [số serial]"; 4 tay nắm 10×10 ở 4 góc |
| **Trang thu nhỏ** | `.mini`, `.vach`, `.o-ky`, `.tick` (L354–357, L367–368) | `miniTrang(r,tick)` L927–930 | 8 vạch chữ ở `top` 10,16,…,52%; khung ký nét đứt; tick xanh |
| **Ô chọn 6 vị trí** | `.vt-pick`, `.vt-trang`, `.vt-o`, `.vt-tl…vt-br`, `.vt-nhan` (L501–512) | `viTriPick(p,act)` L1369–1375 | Trang nhỏ 54×70 có 6 nút 13×9 (trên/dưới × trái/giữa/phải) |
| **Nút** | `.nut`, `.chinh`, `.nho`, `.rong` (L91–102) | — | Nút thường nền trắng; nút chính nền `--accent` chữ trắng đậm |
| **Nút đoạn** | `.seg-nhom`, `.seg`, `.seg.chon` (L103–108) | — | Nhóm nút liền nhau, mục chọn nền `--accent` |
| **Chip chọn** | `.chip`, `.chip.chon` (L109–111) | — | Dùng cho "Dọc" / "Ngang" |
| **Nút icon / link** | `.nutma` (L112–113); `.lk`, `.lk.tron` (L114–115) | — | Nút trong suốt 28px; link gạch chân; link tròn không gạch chân |
| **Công tắc** | `.sw`, `.sw.on` (L415–417) | `ACTS['cfg.toggle']` L1653 | 40×20, kèm chữ "Bật"/"Tắt" bên trái |
| **Tab** | `.tabs`, `.tab`, `.tab.chon`, `.dem`, `.phai-tab` (L121–128); tab cấu hình `.tabch` (L401–405) | — | Gạch chân 2px `--accent` (trừ màn Ký số) |
| **Danh sách** | `.ds`, `.ds-dau`, `.dong`, `.chon`, `.so`, `.ten`, `.phuchu` (L133–143); rỗng `.trong-ds` (L184–185) | — | Lưới CSS, cột định bằng `style="grid-template-columns:…"` nội tuyến |
| **Ô tìm** | `.tim` (L149–151) | `INP['ks.tim']`, `INP['tvb.tim']` | Icon kính lúp + `<input type="search">` |
| **Menu lọc** | `.loc-th`, `.loc-nut`, `.loc-menu`, `.loc-item`, `.loc-dem` (L162–175) | `ACTS['ks.filtToggle' / 'ks.filtSet']` L1134–1135 | |
| **Menu chọn nguồn** | `.lt-pick`, `.lt-nut`, `.lt-menu`, `.lt-menu.len`, `.lt-col`, `.lt-bo` (L241–262) | `ACTS['tvb.pickOpen' / 'tvb.pickCol' / 'tvb.pickBo']` L1293–1295; `keydown` L799 | |
| **Panel giới thiệu** | `.mo-dau`, `.logo-cn`, `.buoc`, `.them`, `.logo-goc` (L280–290) | `introHTML(o)` L776–781 | Icon lớn 70px, `<h2>`, đoạn mô tả, 3 thẻ bước đánh số, logo đầy đủ 60px ở góc phải dưới (bấm về Trang chủ) |
| **Chân màn** | `.chan`, `.dan`, `.so-ok`, `.so-loi` (L294–301) | — | |
| **Hộp thoại** | `.phu`, `.phu.tren`, `.popup`, `.dau-p`, `.than-p`, `.cuoi`, `.trai` (L381–389) | `popup(o)` L1426–1429; `openModal` / `closeModal` L758–759 | |
| **Lưới khoá–giá trị** | `.luoi-tt`, `.k` (L390–391) | — | Cột nhãn 118px |
| **Thẻ / dòng cài đặt** | `.the` (L412), `.row` (L413), `.hint` (L418), `.divider` (L419), `.grid-lbl` (L420–421) | — | |
| **Số lớn / icon lớn** | `.so-lon` (L396–397), `.icon-lon` + `ok/xanh` (L393–395) | — | |
| **`kbd`** | L443 | — | Phím tắt trong Hướng dẫn |
| **Icon** | `svg.i`, `svg.i.nho`, `.mui`, `.la`, `.logo-cn` | `ic(n,cls)` L592; `caret()` L593; `la(n,cls)` L602 | 57 icon nét (viewBox 24) + 4 icon lớn (viewBox 100) |

**Ghi chú port:**
- Tên class là tiếng Việt không dấu và **nhiều class modifier dùng chung nghĩa khác nhau** (`chon`, `dang`, `mo`, `nho`, `trai`, `tren`, `so`, `ten`, `x`…). Có một va chạm thật: `.tieude` vừa là thanh tiêu đề (L36) vừa là dòng tiêu đề Excel `.xls-row.tieude` (L276–277), nên dòng Excel đó nhận luôn `height:32px; padding-left:12px; user-select:none` của thanh tiêu đề. `.trai` vừa là "đẩy sang trái trong chân hộp thoại" (L389) vừa là biến thể tooltip (L473). Khi tách thành component Angular cần đặt lại phạm vi.
- Tooltip làm bằng `::after` nên không chứa được HTML và bị cắt bởi `overflow:hidden` của tổ tiên (`.cuaso`, `.popup`, `.khung`); vì vậy prototype phải chọn tay `trai`/`tren` cho từng vị trí gọi.
- Nút `!` nằm **bên trong** `<button class="tc-the">` (L950) — phần tử tương tác lồng nhau.

---

## 5. Menu bar và phím tắt

### 5.1 Cấu trúc menu (`menuDefs()` L677–697) — nguyên văn

```
[grid] Chức năng ▾
    [home]   Trang chủ                         ✓ khi S.screen==='home'
    ────────────
    [pen]    Ký số văn bản          Ctrl+K     ✓ khi 'kyso' HOẶC 'xemtruoc'
    [tao]    Tạo văn bản            Ctrl+N     ✓ khi 'tvb'
    [shield] Kiểm tra chữ ký số     Ctrl+E     ✓ khi 'kt'
    [frame]  Vị trí chữ ký (kéo thả)           ✓ khi 'tucauhinh'
[gear] Cấu hình                                 (nút trực tiếp, không thả menu) → openCfg(S.cfgTab)
[help] Hỗ trợ ▾
    [book]    Hướng dẫn sử dụng     F1         → hộp 'huongdan'
    [usb]     Cài đặt driver USB Token…        → hộp 'driver'
    [refresh] Kiểm tra cập nhật…               → hộp 'capnhat'
    ────────────
    [info]    Thông tin phần mềm               → hộp 'thongtin'
                                     (canh phải)  [usb] ● USB Token  (!)
```

- Mục hiện tại có nền `--sel` (class `dang`) và dấu "✓" ở cột `.dau` (L713).
- Nút "Cấu hình" sáng (`mo`) khi hộp Cấu hình đang mở (L705). Nút "Chức năng"/"Hỗ trợ" sáng khi menu của nó mở (L707).
- Bấm ra ngoài `.menubar` thì đóng menu (L787). Chọn một mục thì đóng menu rồi chạy hành động (L816).
- Bên phải: `ic('usb','nho')` + `.ketnoi` ("USB Token" / "Chưa cắm token") + tooltip "Chứng thư: …\nNhà cung cấp: …\nThiết bị: …\nTrạng thái: …" lấy từ `S.tokens[0]` (L720–721).

### 5.2 Phím tắt (`keydown` L797–809) — đúng thứ tự xét trong mã

| Thứ tự | Điều kiện | Hành động |
|---|---|---|
| 1 | `Enter` trong ô `#tvbPickInp` | Ghi giá trị cố định cho biến đang chọn, đóng menu nguồn (L799) |
| 2 | `Escape` và menu chọn nguồn đang mở (`S.tvb.pick!=null`) | Đóng menu nguồn (L800) |
| 3 | `Escape` và có tooltip mở (`S.tip`) | Đóng tooltip (L801) |
| 4 | `Enter` hoặc `Space` khi tiêu điểm ở nút `!` | Bấm nút `!` (L802) |
| 5 | `Escape` | (a) nếu có hộp con Cấu hình (`tokenDetail` / `tokenSetting` / `tokenAdd`) → đóng hộp con; (b) ngược lại nếu có hộp thoại (`S.modal`) → `closeModal()`; (c) ngược lại nếu có menu (`S.menu`) → đóng menu (L803) |
| 6 | `F1` | `preventDefault`, mở Hướng dẫn (L804) |
| 7 | `Ctrl` + `K` / `N` / `E` (không `Shift`, không `Alt`; không phân biệt hoa thường) | `preventDefault`, `go('kyso' / 'tvb' / 'kt')` (L805–808) |

**Thứ tự `Esc` tóm tắt:** menu chọn nguồn (Tạo văn bản) → tooltip → hộp con của Cấu hình → hộp thoại → menu bar. Mỗi lần nhấn chỉ đóng một tầng.

**`Esc` không xử lý:** menu ⋯ của dòng token (`S.rowMenu`) — nhấn `Esc` lúc này đóng luôn cả hộp Cấu hình; menu lọc trạng thái (`S.ks.filtOpen`); không dừng lô ký đang chạy.

**Lưu ý:** phím `Ctrl+K/N/E` và `F1` hoạt động cả khi đang mở hộp thoại (`go()` đặt `S.modal=null`, L756) và cả khi đang ký (timer vẫn chạy khi sang màn khác).

### 5.3 Thứ tự xử lý `click` (L784–794)

Trước khi chạy action, listener tự đóng các lớp nổi nếu bấm ra ngoài: menu bar (ngoài `.menubar`) → tooltip (ngoài `.tip`) → menu ⋯ (ngoài `.rowmenu` và không phải nút ⋯) → menu lọc (ngoài `.loc-th`) → menu chọn nguồn (ngoài `.lt-pick`). Sau đó tìm `closest('[data-act]')` và gọi `ACTS[...]`.

---

## 6. Logic JS

### 6.1 Biến trạng thái `S` (L641–655) — nguyên văn giá trị khởi tạo

```js
var S={
  screen:'home', menu:null, modal:null,
  leftW:{kyso:340,tvb:360,kt:300,xt:480},
  ks:{loaded:false,src:'folder',idx:0,running:false,sel:0,tab:'files',tim:'',delOld:false,mode:'Ảnh + thông tin',vitri:'auto',zoom:80,done:{},queue:[],step:0,t0:null,acc:0,listOpen:false,existed:{},dupMode:'overwrite',filt:'all',filtOpen:false},
  tvb:{tpl:false,data:false,source:'excel',view:'ketqua',row:0,fmt:'PDF',tim:'',zoom:80,khung:true,direct:null,ten:'{so}_{hoten}',listOpen:true,map:{hoten:'Họ tên',ngaysinh:'Ngày sinh',donvi:'Đơn vị công tác',khoahoc:'Khóa đào tạo',xeploai:'Xếp loại',so:'Số hiệu',ngaycap:'Ngày cấp'},defv:{},pick:null,hrow:1,xlsOpen:false,sheet:'Sheet1'},
  kt:{loaded:false,sel:0,tab:'sig',zoom:80},
  tc:{mode:'all',file:0,frame:null,per:{},prof:0,zoom:66},
  cfgTab:'sig',cfgSel:0,rowMenu:-1,tokenDetail:-1,tokenSetting:-1,tokenAdd:false,pinType:'password',saved:false,
  hdSel:0,
  profiles:PROFILES_DEF.map(function(p){return Object.assign({},p);}),
  tk:Object.assign({},TK_DEF),
  tokens:TOKENS_DEF.map(function(t){return Object.assign({},t);}),
  adv:{}, tip:null,
  timer:null
};
```

Trường phát sinh lúc chạy: `S._cuon` (id vùng xem cần cuộn tới khung chữ ký), `S.tvb._xlsScroll`.

| Trường | Ý nghĩa |
|---|---|
| `ks.done` | map `chỉ số file → 'ok' / 'loi'` |
| `ks.queue`, `ks.step` | hàng đợi của lần chạy hiện tại và vị trí đang ký |
| `ks.t0`, `ks.acc` | mốc bắt đầu lần chạy hiện tại (ms) và thời gian cộng dồn các lần chạy trước (ms) |
| `ks.existed` | map file đã có bản ký trong thư mục đích (để hỏi trùng tên) |
| `ks.vitri` | `'auto'` = theo mẫu khổ giấy; `'keo'` = tự cấu hình (sau khi bấm Áp dụng). Mã còn nhận cả chỉ số mẫu (`+v`, L667) nhưng không nơi nào gán |
| `ks.mode` | kiểu hiển thị **toàn cục** dùng cho xem trước Ký số và màn Vị trí |
| `tc.frame` | khung chung (chế độ Tất cả file); `null` = lấy theo mẫu |
| `tc.per` | map `chỉ số file → khung riêng` (chế độ Từng file) |
| `tc.prof` | chỉ số mẫu đang lấy vị trí |
| `tvb.map` | map `biến → tên cột` hoặc `'__fixed__'`; `tvb.defv` giữ giá trị cố định |

**Trường khai báo nhưng không dùng:** `S.ks.idx` (0 chỗ đọc); `S.ks.dupMode` (chỉ được gán ở L1130, L1141, L1142, không nơi nào đọc).

### 6.2 Dữ liệu giả / mô phỏng

| Thứ | Dòng | Ghi chú |
|---|---|---|
| `FILES` — 24 tên file (21 `.pdf`, 3 `.docx` ở chỉ số 6, 8, 23) | L616 | Danh sách "thư mục" giả |
| `ONE_FILE='QD_015_2026_SaoMai.pdf'` | L617 | Chế độ 1 file |
| `FAIL={7:'file đang được mở bởi chương trình khác',17:'file PDF bị đặt mật khẩu, không thể ký'}` | L618 | File chỉ số 7 (`ThongBao_NghiLe.pdf`) và 17 (`HopDong_LD_0231.pdf`) luôn lỗi khi ký thư mục |
| `soTrang(i)=1+((i*7+3)%5)` | L619 | Số trang giả (1–5) |
| Đường dẫn giả | L628, L1016, L1020, L1186, L1191, L1277, L1313 | `D:\VanBan\CanKy`, `D:\VanBan\CanKy\DaKySo`, `D:\ChuKy\condau_saomai.png`, `D:\Mau\GIAY_CHUNG_NHAN.docx`, `D:\Mau\DanhSach_HocVien.xlsx`, `D:\VanBan\KetQua` |
| Timer giả lập ký | L1119–1127 | `setInterval` mỗi **380 ms**/file (thư mục) hoặc **600 ms** (1 file); mỗi nhịp đánh dấu xong 1 file |
| `existed` ban đầu `{2:true,9:true}` khi chọn thư mục | L1130 | Giả lập 2 file đã có bản ký (`CV_115_PhongHC.pdf`, `QD_003_2026_SaoMai.pdf`) để hộp trùng tên hiện ngay lần đầu |
| Giờ nhật ký `stamp(sec)` → `'20:52:00'` + sec | L982 | Đồng hồ giả |
| Dòng nhật ký `TOKEN` "Phiên token hết hạn – tự động nhập lại mã PIN từ cấu hình… thành công" | L996 | Luôn xuất hiện trước file chỉ số 12 khi ký thư mục, không phụ thuộc cài đặt |
| `TOKENS_DEF` (2 token chữ giữ chỗ), `AVAILABLE` (2 token "vừa cắm") | L631–638 | Toàn chữ giữ chỗ dạng `[Tên chứng thư số]` |
| PIN `'12345678'` | L628, L1451 | Điền sẵn trong hộp PIN và trong cấu hình |
| `TVB_ROWS` — 12 bản ghi học viên; `TVB_FIELDS`, `TVB_COLS`, `COL_KEY`, `COLW` | L1150–1167 | Dữ liệu Excel giả |
| `SIGS` — 2 chữ ký (một "Người ký chính", một "Ký nháy") | L1307–1310 | Kết quả kiểm tra giả, luôn hợp lệ |
| `docQD()` (Quyết định), `docCN()` (Giấy chứng nhận) | L893–907, L1170–1181 | Trang văn bản giả bằng HTML; **không** dùng pdf.js |
| `HD` — nội dung 7 mục hướng dẫn | L1491–1511 | Văn bản thật có thể tái sử dụng làm nháp nội dung |
| Phiên bản, ngày, tên máy | L952, L1522, L1529 | "1.5.0", "1.4.2", "28/10/2026", "build 2026.10.28", "PC-KYSO-01" |
| Các nút `noop` / không handler | — | "Chọn ảnh", xoá ô ảnh, "Lưu nhật ký…", "Mở thư mục" (cả 3 chỗ), "Ảnh thẻ…", "Con dấu…", "Xuất báo cáo", "Làm mới", "Quét lại", "Kiểm tra token", "Tải driver", "Chọn…" (thư mục, ảnh chữ ký), 3 nút thanh tiêu đề |
| Chọn file/thư mục/mẫu/dữ liệu | L1130, L1281–1285, L1357 | Chỉ bật cờ `loaded`/`tpl`/`data`; không mở hộp chọn file |

### 6.3 Logic thật — công thức và hằng số

#### 6.3.1 Tiến độ, tốc độ, ETA (L1008–1012)

```
vis      = các file đang hiện (sau tìm + lọc)
remain   = số file trong vis chưa có kết quả (done[i] rỗng)
vok/verr = số file trong vis có done = 'ok' / 'loi'
dDone    = vok + verr
dPct     = nVis ? round(dDone / nVis * 100) : 0          // bề rộng vạch tiến độ
elapsedMs= acc + (running && t0 ? now - t0 : 0)
rateMs   = dDone > 0 ? elapsedMs / dDone : 0
etaMs    = rateMs > 0 ? round(rateMs * remain) : 0
allXong  = !running && remain === 0 && hasDone
```
- Khi dừng: `acc += now - t0; t0 = null` (`ksStop()` L1112). Khi chạy lại: `t0 = now` (`ksStart()` L1118).
- Khi `dDone === 0` và đang chạy hiện " · đang ước lượng…".
- Định dạng thời lượng `fmtDur(ms)` (L983): `s = max(0, round(ms/1000))`, `m = floor(s/60)`; nếu `m >= 60` → `"<giờ>h<phút 2 chữ số>m"`, ngược lại `"mm:ss"`.

#### 6.3.2 Tìm không dấu (L613, L963–975)

```js
function boDau(s){return s.normalize('NFD').replace(/[\u0300-\u036f]/g,'').replace(/đ/g,'d').replace(/Đ/g,'D');}
q = boDau(k.tim.trim().toLowerCase());
khớp ⇔ boDau(tenFile.toLowerCase()).indexOf(q) >= 0
```
Ở Tạo văn bản (L1239–1242) khớp theo **tên file sẽ tạo** hoặc chuỗi nối các giá trị trường bằng `' · '`.

#### 6.3.3 Lọc trạng thái và đếm (L958–976)

- `ksStatus(i)`: `'dang'` nếu đang chạy và `queue[step]===i`; ngược lại `done[i]` nếu có; ngược lại `'cho'`.
- `ksVisible()`: rỗng nếu chưa nạp; giữ file khớp từ khoá **và** (lọc = `'all'` hoặc trạng thái = lọc).
- `ksCount(st)`: đếm theo trạng thái **chỉ áp từ khoá tìm**, không áp bộ lọc (để hiện số ở từng mục menu lọc).
- `ksRemain()` = `ksVisible()` bỏ file đã có kết quả. **Hàng đợi ký = `ksRemain()` tại thời điểm bấm** (`ksStart()` L1116) → chỉ ký file đang hiện. File lỗi coi như đã có kết quả nên "Ký tiếp" không ký lại file lỗi.

#### 6.3.4 Luồng ký (L1113–1143)

```
ks.sign:   chưa nạp → bỏ qua | đang chạy → ksStop() | không còn file → bỏ qua
           | có file trùng (ksDups) → hộp 'trung' | ngược lại → hộp 'pin'
dup.overwrite / dup.keep → ghi dupMode → hộp 'pin'
pin.ok → ksStart(): queue = ksRemain(); step = 0; running = true; t0 = now; sel = queue[0]
mỗi nhịp: done[file] = lỗi/ok; nếu ok thì existed[file] = true; step++;
          hết hàng đợi → ksStop(); mở hộp 'done'; ngược lại sel = queue[step]
ks.resign: với mọi file đang hiện → existed = true, xoá done; đặt lại queue/step/acc/t0 → hộp 'trung'
ks.clearLog: nếu không chạy → xoá done, queue, step, acc, t0
ks.pick / ks.clear: dừng timer, đặt lại toàn bộ trạng thái lô
```
Trong lúc chạy, `sel` đi theo file đang ký (xem trước tự đổi), dòng đang ký được `scrollIntoView({block:'nearest'})` (L748).

#### 6.3.5 Tên file kết quả (L998, L1458)

```js
list[i].replace(/\.(pdf|docx)$/, S.tk.suffix + '.$1')   // QD_001.pdf → QD_001_signed.pdf ; X.docx → X_signed.docx
```
Biểu thức phân biệt hoa thường (`.PDF` sẽ không khớp).

#### 6.3.6 Vị trí khung chữ ký từ mẫu — `sigRect(p)` (L658–666)

Đơn vị trả về: **% của trang** (x, w theo bề rộng; y, h theo bề cao), làm tròn 1 chữ số thập phân.

```
w  = clamp(24 * scale/100 + 10, 16, 62)        // % bề rộng trang
h  = w * 0.27                                   // % bề cao trang
mx = p.mx / 210 * 100                           // lề ngang mm → %  (luôn chia cho 210)
my = p.my / 297 * 100                           // lề dọc  mm → %  (luôn chia cho 297)
x  = cột 'l' → mx | cột 'r' → 100 - w - mx | cột 'c' → (100 - w) / 2
y  = hàng 't' → my | hàng 'b' → 100 - h - my
```
- `pos` là 2 ký tự: ký tự đầu là hàng (`t`/`b`), ký tự sau là cột (`l`/`c`/`r`).
- Công thức **luôn dùng kích thước A4 (210×297)** và không xét `paper`/`orient` của mẫu. Vị trí giữa (`c`) bỏ qua lề ngang.
- Với thanh trượt 40–200%, `w` chạy từ 19,6 đến 58 (không chạm mốc kẹp 16/62).
- Trên trang 520×736, tỉ lệ rộng:cao của khung tính theo điểm ảnh = `520 / (0.27 × 736)` ≈ **2,62 : 1**.
- Giá trị tính ra cho 5 mẫu mặc định (vị trí `br`):

| Mẫu | scale | mx, my | x | y | w | h |
|---|---|---|---|---|---|---|
| Khổ A4 | 100 | 15, 20 | 58.9 | 84.1 | 34.0 | 9.2 |
| Khổ A5 | 70 | 10, 12 | 68.4 | 88.7 | 26.8 | 7.2 |
| Khổ A3 | 130 | 20, 20 | 49.3 | 82.1 | 41.2 | 11.1 |
| Khổ A2 | 160 | 25, 25 | 39.7 | 78.5 | 48.4 | 13.1 |
| Khổ A1 | 200 | 30, 30 | 27.7 | 74.2 | 58.0 | 15.7 |

  (Khung mẫu A4 trùng đúng `SIGS[0].rect={x:58.9,y:84.1,w:34,h:9.2}` ở L1308.)
- Chọn khung cho file `i` ở màn Ký số — `ksRect(i)` (L978–981): nếu `vitri==='keo'` → `(mode==='per' && per[i]) || frame || sigRect(profiles[tc.prof||0])`; ngược lại `sigRect(ksProfile())`, mà `ksProfile()` (L667) **luôn trả `S.profiles[0]`** khi `vitri` là `'auto'` hoặc `'keo'`.

#### 6.3.7 Khung xem trước trong Cấu hình — `cfgPreview(p)` (L1546–1557)

```
[pw, ph] = PAPER[p.paper] (mm), mặc định [210,297]; nếu orient === 'Ngang' thì hoán đổi
sc = min(200 / pw, 250 / ph);  W = round(pw * sc);  H = round(ph * sc)        // khung tối đa 200×250 px
sw = clamp(round(W * 0.32 * scale/100), 26, W - 8)
sh = min(round(sw * 0.42), H - 8)
mx = round(p.mx / pw * W);  my = round(p.my / ph * H)
L  = 'l' → mx | 'r' → W - sw - mx | 'c' → round((W - sw) / 2)
T  = 't' → my | 'b' → H - sh - my
```
- `PAPER={A1:[594,841],A2:[420,594],A3:[297,420],A4:[210,297],A5:[148,210]}` (L629).
- 14 vạch chữ, độ rộng (%): `92,86,96,70,88,94,60,90,84,96,72,88,90,64` (L1552).
- Khung ẩn hẳn khi kiểu "Ẩn". Vòng tròn đỏ `(sh-6)×(sh-6)` khi có ảnh; 3 vạch chữ khi có thông tin.
- Công thức này **khác** `sigRect` (0,32·W và tỉ lệ 0,42 so với `24·scale/100+10` và 0,27) — xem mục 10.

#### 6.3.8 Kéo và đổi cỡ khung chữ ký (L824–857)

- Bắt đầu: `mousedown` chuột trái trên phần tử có `data-keo`. `di` = di chuyển (đặt trên `.chuky-o`), `doi` = đổi cỡ (đặt trên 4 tay nắm, `data-arg` = `tl`/`tr`/`bl`/`br`), `cot` = kéo cột.
- Lấy kích thước trang hiện tại: `r = #tcPage.getBoundingClientRect()` (đã gồm zoom) → `pw = r.width`, `ph = r.height`.
- Mỗi lần `mousemove`:
```
dx = (clientX - startX) / pw * 100;   dy = (clientY - startY) / ph * 100      // đổi sang %
di chuyển:  x = clamp(o.x + dx, 0, 100 - o.w);  y = clamp(o.y + dy, 0, 100 - o.h)
đổi cỡ:  góc có 'r':  w = clamp(o.w + dx, 10, 100 - o.x)
         góc có 'l':  w = clamp(o.w - dx, 10, o.x + o.w);  x = o.x + o.w - w
         góc có 'b':  h = clamp(o.h + dy,  4, 100 - o.y)
         góc có 't':  h = clamp(o.h - dy,  4, o.y + o.h);  y = o.y + o.h - h
```
- Hằng số: **rộng tối thiểu 10%, cao tối thiểu 4%**; khung luôn nằm trong trang (0–100%). **Không khoá tỉ lệ** rộng/cao.
- Trong lúc kéo chỉ sửa `style` trực tiếp (không vẽ lại); khi `mouseup` mới ghi `tcSet({x,y,w,h})` với giá trị **làm tròn về số nguyên %** (`Math.round`, L850) rồi `render()`.
- Con trỏ: `move` khi di chuyển; `nwse-resize` cho `tl`/`br`, `nesw-resize` cho `tr`/`bl` (L919); `document.body.style.cursor` được đặt trong lúc kéo.
- Dùng sự kiện **mouse** (không phải pointer) → không hỗ trợ cảm ứng/bút.

#### 6.3.9 Thanh trượt "Cỡ" và khung theo chế độ (L1381–1383, L1387, L1418–1420)

```
tcTpl()   = sigRect(profiles[tc.prof || 0])                 // khung của mẫu
tcRect(f) = base = tc.frame || tcTpl();  mode==='per' ? (tc.per[f] || base) : base
tcSet(r)  = mode==='per' ? tc.per[tc.file] = r : tc.frame = r
co (giá trị thanh trượt) = round(fr.w / def.w * 100)        // % so với bề rộng khung của mẫu
tc.size:  k = value/100;  fr.w = round1(def.w * k);  fr.h = round1(def.h * k);
          fr.x = clamp(fr.x, 0, 100 - fr.w);  fr.y = clamp(fr.y, 0, 100 - fr.h)
tc.reset: mode==='per' ? xoá tc.per[tc.file] : tc.frame = null
tc.apply: ks.vitri = 'keo';  go('kyso')
xt.profile (đổi mẫu) / xt.pos (đổi vị trí theo mẫu): tcSet(tcTpl())   // ghi đè khung hiện tại bằng khung mẫu
```
Thanh trượt `min=50 max=200 step=10`. Sau khi kéo góc, `co` có thể không chia hết cho 10.

#### 6.3.10 Kéo đổi độ rộng cột trái (L829, L841)

```
maxW = par ? max(320, par.clientWidth - 260) : 560
w    = clamp(orig + (clientX - startX), 180, maxW)
```
Rộng tối thiểu **180px**; cột phải luôn còn ít nhất 260px. Mặc định: `kyso` 340, `tvb` 360, `kt` 300, `xt` 480.

#### 6.3.11 Hoạt ảnh logo bằng `requestAnimationFrame` (L859–887)

Hằng số (L861): `CK=5` (chu kỳ, giây) · `TV=0.7` (thời gian sao bay, giây) · `TN=0.3` (thời gian nhấp nháy, giây) · `D=[[100,110],[170,100],[236,62],[212,23]]` (4 điểm điều khiển Bézier bậc 3 trong hệ `viewBox="0 0 300 246"`).

```
bez(t): u = 1 - t;  k = [u³, 3u²t, 3ut², t³];  điểm = Σ k[j] · D[j]
em(t)  = t² · (3 - 2t)                                   // smoothstep
t = ((now - t0) / 1000) mod CK

Pha bay (t < TV):
  u = em(t / TV);  p = bez(u)
  to = 1 + 1.4 · max(0, (u - 0.75) / 0.25)               // đầu sao phóng to ở 25% cuối
  đầu sao (.ld-dau): opacity = 1;  transform = translate(p) scale(to)
  mỗi hạt vệt k ∈ {6,5,4,3,2,1}:  v = u - k · 0.04
      v ≤ 0 → opacity 0;  ngược lại: (cx,cy) = bez(v);  opacity = 0.55 · (1 - k/7)
  ảnh ngôi sao (.ld-sao): opacity = max(0.2, 1 - t/0.15 · 0.8);  transform = ''

Pha nhấp nháy (TV ≤ t < TV + TN):
  đầu sao và vệt: opacity 0
  s = (t - TV) / TN
  ngôi sao: opacity = min(1, 0.2 + 0.8 · s / 0.4);  transform = scale(1 + 0.5 · sin(π · s))

Pha nghỉ (t ≥ TV + TN): ngôi sao opacity 1, không transform
```
- Markup (`logoDongHTML()` L884–887): `<span class="logo-dong" id="logoDong">` gồm ảnh nền `IMG.logo` (300×246), ảnh ngôi sao `IMG.sao` (46×46) và một SVG `viewBox="0 0 300 246"` phủ lên.
- Hạt vệt: 6 hình tròn `fill="#FFD06A"`, bán kính `2 + 0.7·(7-k)` → k=6…1: 2.7, 3.4, 4.1, 4.8, 5.5, 6.2; opacity đỉnh 0.08, 0.16, 0.24, 0.31, 0.39, 0.47.
- Đầu sao: `<circle r="9" fill="#FFD06A" opacity=".45"/>` + `<circle r="4.5" fill="#fff"/>`.
- Vị trí ngôi sao (L328): `left:63%; top:.41%; width:15.33%; transform-origin:50% 50%` (15,33% × 300 ≈ 46px).
- Vòng lặp chạy liên tục kể cả khi không ở Trang chủ (chỉ bỏ qua khi không tìm thấy `#logoDong`).

#### 6.3.12 Zoom và trang giấy (L820, L923–926)

`zoom = clamp(zoom ± 10, 40, 160)`. Trang: khung ngoài `round(520·z) × round(736·z)`, nội dung `transform:scale(z)` với `transform-origin:0 0`. Mặc định: Ký số 80, Tạo văn bản 80, Kiểm tra 80, Vị trí 66.

#### 6.3.13 Mẫu tên file ở Tạo văn bản (L1158–1163)

```js
s = pat.replace(/\{([^}]+)\}/g, (m,k) => r[k.trim()] == null ? '' : String(r[k.trim()]));
s = boDau(s).replace(/[\\/:*?"<>|]+/g,'-').replace(/\s+/g,'_').replace(/^[_\-.]+|[_\-.]+$/g,'');
return s || ('vanban_' + (i + 1));
tên file = s + '.' + fmt.toLowerCase()
```
Ví dụ: `{so}_{hoten}` với `so='45/CN-SM'`, `hoten='Nguyễn Văn An'` → `45-CN-SM_Nguyen_Van_An.pdf`. **Không xử lý trùng tên.**

#### 6.3.14 Ghép biến (L1168–1169, L1210, L1293–1295, L799)

- `tvbMapped(k)`: chưa có `map[k]` → chưa ghép; `map[k]==='__fixed__'` → đã ghép khi `defv[k]` khác rỗng; ngược lại đã ghép.
- `colInfo(c).n` = số giá trị **khác nhau** trong cột (đếm khoá của một object) — hiện bên phải mỗi cột trong menu.
- Gõ giá trị rồi `Enter` → `map[k]='__fixed__'`, `defv[k]=giá trị`. "Bỏ ghép" → xoá cả `map[k]` và `defv[k]`.
- Bản xem Kết quả: biến cố định hiện `defv[k]`; biến khác **luôn hiện `r[k]`** bất kể đang ghép với cột nào (mô phỏng).

#### 6.3.15 Tooltip: mã định danh (L605–610)

`raw = key ? 'k|' + key : text + '|' + cls`; băm `h = (h*31 + charCode) | 0`; `id = 'tip' + (h >>> 0).toString(36)`. Tham số `key` dùng khi nội dung thay đổi theo trạng thái (để tooltip không tự đóng lúc vẽ lại): `ksFile`, `ksChan`, `'loi'+i`, `'kt'+sel`, và id từng thẻ Trang chủ.

#### 6.3.16 Cơ chế vẽ lại (L725–750) — không cần port nguyên xi

`render()` gán lại `innerHTML` toàn bộ rồi: khôi phục tiêu điểm theo `id` và vùng chọn chữ; khôi phục `scrollTop/Left` của các phần tử có `data-giu`; cuộn nhật ký xuống cuối; nếu `S._cuon` có giá trị thì cuộn vùng xem để **khung chữ ký nằm giữa theo chiều dọc** (`scrollTop += cr.top - vr.top - clientHeight/2 + cr.height/2`, L747); cuộn dòng đang ký vào tầm nhìn; cuộn dòng tiêu đề Excel vào giữa. `spin()` (L931) đặt `animation-delay` âm theo `Date.now() % 900` để icon quay không giật khi vẽ lại. Với Angular chỉ cần giữ hai hành vi: tự canh giữa khung chữ ký khi đổi file, và tự cuộn tới dòng đang ký.

#### 6.3.17 Nhận diện khổ giấy

**Prototype không có logic nhận diện khổ giấy.** Chỉ có lời văn ("App tự chọn mẫu theo khổ giấy của từng văn bản", L1499, L1563). Trong mã, `ksProfile()` luôn trả mẫu đầu tiên và trang xem trước luôn là 520×736 (tỉ lệ A4 dọc).

### 6.4 Bảng handler

`ACTS` (click): `noop`, `tip`, `adv`, `go`, `menu.toggle`, `menu.item`, `menu.direct`, `modal.close`, `phu.nen`, `zoom` (L811–820) · `ks.pick`, `ks.clear`, `ks.tab`, `ks.panel`, `ks.filtToggle`, `ks.filtSet`, `ks.sel`, `ks.nav`, `ks.sign`, `ks.resign`, `dup.overwrite`, `dup.keep`, `ks.clearLog` (L1130–1143) · `tvb.pickTpl`, `tvb.clearTpl`, `tvb.pickData`, `tvb.clearData`, `tvb.demo`, `tvb.source`, `tvb.view`, `tvb.row`, `tvb.panel`, `tvb.xls`, `tvb.pickOpen`, `tvb.pickCol`, `tvb.pickBo`, `tvb.nav`, `tvb.fmt`, `tvb.ghep`, `tvb.create` (L1281–1299) · `kt.pick`, `kt.clear`, `kt.tab`, `kt.sel`, `kt.nav` (L1357–1361) · `cfg.open` (L1365) · `tc.mode`, `tc.file`, `tc.nav`, `tc.reset`, `tc.apply`, `ks.modeSet`, `xt.pos` (L1414–1422) · `pin.eye`, `pin.ok`, `done.log`, `tvb.kySo`, `hd.sel`, `hd.nav`, `cfg.tab`, `cfg.sel`, `cfg.orient`, `cfg.pos`, `cfg.addProfile`, `cfg.delProfile`, `cfg.toggle`, `cfg.rowmenu`, `cfg.tokenEdit`, `cfg.tokenSetting`, `cfg.tokenActive`, `cfg.tokenDel`, `cfg.openAdd`, `cfg.addToken`, `cfg.subClose`, `cfg.subNen`, `cfg.reset`, `cfg.save` (L1641–1664).

`CHG` (change): `ks.so`, `ks.delOld`, `ks.mode` (L1144–1146) · `tvb.hrow`, `tvb.sheet`, `tvb.so`, `tvb.khung`, `tvb.f` (L1291–1302) · `xt.profile` (L1366) · `tc.modeSel`, `tc.size`, `xt.pos` (L1415–1423) · `cfg.p`, `cfg.num`, `cfg.t` (L1665–1667).

`INP` (input): `ks.tim` (L1147) · `tvb.tim`, `tvb.ten` (L1303–1304).

**Handler không được phần tử nào gọi (mã chết):** `ACTS['tvb.ghep']` (L1298), `ACTS['cfg.open']` (L1365), `ACTS['tc.mode']` (L1414), `ACTS['ks.modeSet']` (L1421), `ACTS['xt.pos']` (L1422; bản `CHG['xt.pos']` mới là bản được dùng).
**Hàm không được gọi:** `giay()` (L768; trùng chức năng với `trang()`), `hienThiBar()` (L1376; kéo theo CSS `.tb-noi` L515–518 không dùng).

---

## 7. Mô hình dữ liệu cấu hình ngầm định

### 7.1 Mẫu chữ ký theo khổ giấy (`PROFILES_DEF` L621–627)

| Trường | Kiểu / miền giá trị | Nguồn |
|---|---|---|
| `name` | chuỗi tự do; mẫu mới tên "Mẫu mới" | L1571, L1651 |
| `isDefault` | boolean; chỉ "Khổ A4" là `true`; **không có UI đổi mẫu mặc định** | L622 |
| `paper` | `'A1'` / `'A2'` / `'A3'` / `'A4'` / `'A5'` | L1564 |
| `orient` | `'Dọc'` / `'Ngang'` | L1564 |
| `page` | `'Trang cuối'` / `'Trang đầu'` / `'Tất cả các trang'` | L1572 |
| `pos` | `'tl'` / `'tc'` / `'tr'` / `'bl'` / `'bc'` / `'br'` (nhãn `POS_LABEL` L630: "trên trái", "trên giữa", "trên phải", "dưới trái", "dưới giữa", "dưới phải") | L1370 |
| `mx` | số, mm, lề ngang; ô nhập `min=0 max=100` | L1573 |
| `my` | số, mm, lề dọc; ô nhập `min=0 max=150` | L1573 |
| `scale` | số, %, thanh trượt `min=40 max=200 step=10` | L1566 |
| `mode` | `'Ảnh + thông tin'` / `'Chỉ ảnh'` / `'Chỉ thông tin'` / `'Ẩn'` | L1567 |
| `img` | tên file ảnh: `condau_saomai.png` / `chuky_giamdoc.png` / `chuky_nhay.png` | L1574 |

Giá trị mặc định (nguyên văn):

| name | isDefault | paper | orient | page | pos | mx | my | scale | mode | img |
|---|---|---|---|---|---|---|---|---|---|---|
| Khổ A4 | true | A4 | Dọc | Trang cuối | br | 15 | 20 | 100 | Ảnh + thông tin | condau_saomai.png |
| Khổ A5 | false | A5 | Dọc | Trang cuối | br | 10 | 12 | 70 | Ảnh + thông tin | condau_saomai.png |
| Khổ A3 | false | A3 | Ngang | Trang cuối | br | 20 | 20 | 130 | Ảnh + thông tin | condau_saomai.png |
| Khổ A2 | false | A2 | Ngang | Trang cuối | br | 25 | 25 | 160 | Ảnh + thông tin | condau_saomai.png |
| Khổ A1 | false | A1 | Ngang | Trang cuối | br | 30 | 30 | 200 | Ảnh + thông tin | condau_saomai.png |

Thao tác: "Thêm mẫu" sao chép mẫu đang chọn, đổi tên "Mẫu mới", `isDefault=false` (L1651). "Xóa mẫu" chỉ khi còn >1 mẫu và mẫu không phải mặc định (L1652). `cfg.num` ghi `+value || 0`, không kẹp trong mã (L1666).

### 7.2 Cài đặt token/PIN và thư mục (`TK_DEF` L628)

```js
var TK_DEF={savePin:true,pin:'12345678',autoPin:true,retries:'2 lần',onFail:'Tạm dừng và hiện hộp nhập PIN',keep:'Theo cài đặt token',outDir:'D:\\VanBan\\CanKy\\DaKySo',suffix:'_signed',suffixErr:'_loi',overwrite:false};
```

| Trường | Miền giá trị | Mặc định |
|---|---|---|
| `savePin` | boolean ("Lưu mã PIN trên máy này") | **`true`** |
| `pin` | chuỗi, lưu dạng rõ | `'12345678'` |
| `autoPin` | boolean; chỉ bật được khi `savePin` (L1653); hiệu lực = `savePin && autoPin` (L1611) | **`true`** |
| `retries` | `'1 lần'` / `'2 lần'` / `'3 lần'` | `'2 lần'` |
| `onFail` | `'Tạm dừng và hiện hộp nhập PIN'` / `'Dừng hẳn tiến trình ký'` | cái đầu |
| `keep` | `'Theo cài đặt token'` / `'15 phút'` / `'30 phút'` / `'Đến khi đóng app'` | cái đầu |
| `outDir` | chuỗi đường dẫn tuyệt đối | `D:\VanBan\CanKy\DaKySo` |
| `suffix` | chuỗi | `_signed` |
| `suffixErr` | chuỗi | `_loi` |
| `overwrite` | boolean ("Ghi đè khi trùng tên") | `false` |

Các cài đặt PIN nằm trong **một object dùng chung** (`S.tk`) dù hộp "Cài đặt tự động ký" mở theo từng token.

### 7.3 Token (`TOKENS_DEF` L631–634)

`{name, ca, serial, valid, device, owner, status, connected}`. Phần tử **chỉ số 0 là token đang dùng/mặc định** (nhãn "Đang dùng", chỉ báo menu bar, hộp PIN). "Đặt làm mặc định" = đưa lên đầu mảng (L1657). "Xóa khỏi danh sách" chỉ khi còn >1 (L1658). "Thêm" = đẩy một bản ghi từ `AVAILABLE` (`{name, ca, device}`) vào cuối (L1660; không loại khỏi `AVAILABLE` nên thêm trùng được). `status`: "Đã kết nối" / "Chưa cắm".

### 7.4 Tuỳ chọn lô ký (trong `S.ks`)

`delOld` (boolean "Xóa file gốc", mặc định `false`) · `mode` (kiểu hiển thị toàn cục, mặc định "Ảnh + thông tin") · `vitri` (`'auto'` / `'keo'`) · ảnh chữ ký `D:\ChuKy\condau_saomai.png` (chuỗi cứng, không thuộc `S`).

### 7.5 Khung tự cấu hình (trong `S.tc`)

`mode` (`'all'` / `'per'`) · `frame` (`{x,y,w,h}` theo %, hoặc `null`) · `per` (map chỉ số file → `{x,y,w,h}`) · `prof` (chỉ số mẫu). Giá trị sau khi kéo là số nguyên %; sau thanh trượt là 1 chữ số thập phân. Không có trường trang (page) cho khung.

### 7.6 Tạo văn bản (trong `S.tvb`)

`source` (`'excel'` / `'direct'`) · `fmt` (`'PDF'` / `'DOCX'`) · `ten` (mẫu tên, mặc định `{so}_{hoten}`) · `sheet` (`'Sheet1'`…) · `hrow` (dòng tiêu đề, ≥1, mặc định 1) · `map` / `defv` (ghép biến) · `khung` (tô màu trường, mặc định `true`) · `direct` (bản ghi nhập tay, khởi tạo bằng bản sao dòng đầu, L1184). Thư mục lưu `D:\VanBan\KetQua` là chuỗi cứng, không có trường cấu hình.

### 7.7 Giao diện

`leftW:{kyso:340,tvb:360,kt:300,xt:480}` (kẹp 180…`max(320, bề rộng cha − 260)`) · `zoom`: `ks 80`, `tvb 80`, `kt 80`, `tc 66` (kẹp 40–160, bước 10) · tab đang mở: `ks.tab`, `tvb.view`, `kt.tab`, `cfgTab`, `hdSel` · `ks.listOpen`, `tvb.listOpen` · `adv` (các panel Nâng cao). Các giá trị này trùng với ví dụ `ui` trong `config.json` ở mục 11 của kế hoạch.

---

## 8. Đối chiếu với kế hoạch

### 8.1 Mục 12 — từng mã chức năng

**Khung ứng dụng (SH)**

| Mã | Có trong prototype? | Ở đâu | Khác biệt so với kế hoạch |
|---|---|---|---|
| SH-01 | Có (một phần) | `.cuaso` L34; tiêu đề L672; `TITLES` L670; icon L7, L37 | Prototype là **1280×800** (đã gồm thanh tiêu đề tự vẽ 32px), kế hoạch ghi 1280×**820**. Không có kích thước tối thiểu (chỉ `max-width:100%`). Tiêu đề đúng dạng "Sao Mai Ký số — <màn>"; `<title>` của trang (L6) thì cố định. Hai màn `xemtruoc`/`tucauhinh` cùng tên "Xem trước & vị trí chữ ký" |
| SH-02 | Không | — | Không áp dụng cho bản HTML |
| SH-03 | Có | `menuDefs()` L677–697 | Khớp. Chi tiết thêm: có vách sau "Trang chủ"; nhãn chính xác "Kiểm tra chữ ký số", "Vị trí chữ ký (kéo thả)", "Cài đặt driver USB Token…", "Kiểm tra cập nhật…", "Thông tin phần mềm"; "Cấu hình" là nút bấm thẳng (không thả menu); "Ký số văn bản" có ✓ cả khi ở màn `xemtruoc` |
| SH-04 | Có | `keydown` L797–809 | Khớp. Thứ tự `Esc` thực tế có thêm 2 tầng đứng trước: menu chọn nguồn biến và tooltip. `Esc` không đóng riêng menu ⋯ và menu lọc |
| SH-05 | Có (tĩnh) | L720–721 | **Không có cập nhật 2 giây**; đọc tĩnh `S.tokens[0]`. Nhãn khi có token là "USB Token" (không phải tên token). Tooltip đúng 4 dòng: Chứng thư, Nhà cung cấp, Thiết bị, Trạng thái |
| SH-06 | Có | Mục 4 | Khớp. Zoom 40–160 bước 10 (L820). Tooltip mở bằng **bấm**, không phải hover |
| SH-07 | Không | — | Không lưu gì giữa các lần chạy (không `localStorage`); chỉ nhớ trong phiên |
| SH-08 | Không | — | Không có handler `drop`/`dragover` nào |
| SH-09 | Có (chỉ vẽ) | `titlebarHTML()` L671–676; CSS L36–42 | 3 nút cửa sổ không có hành vi |

**Trang chủ (HM)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| HM-01 | Có | L859–887; `<h1>` L947 | Khớp. Có thêm dòng phụ "Chọn chức năng để bắt đầu" + tooltip; logo là link tới `https://saomaiso.com` |
| HM-02 | Có | L941–950 | Phím tắt nằm **trong tooltip**, không in trên thẻ; thẻ "Vị trí chữ ký" không có phím tắt |
| HM-03 | Có | L951–952 | Tooltip có thêm dòng "Bản quyền: Giải pháp số Sao Mai". Phiên bản là chuỗi cứng "1.5.0" |

**Ký số văn bản (KS)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| KS-01 | Có (một phần) | L1016–1017 | Chỉ "1 file" và "Thư mục". **Không có chọn nhiều file** |
| KS-02 | Có (vẽ) | L1020 | Nút "Chọn ảnh" là `noop`. Không có ràng buộc "png/jpg ≤ 5MB" (nơi khác chỉ nhắc PNG: L948, L1493) |
| KS-03 | Có | L1021 | Đi tới `xemtruoc`, panel Nâng cao mở sẵn |
| KS-04 | Có, khác vị trí và tên | L1018 | Prototype: checkbox **"Xóa file gốc"** ngay trên màn Ký số. Kế hoạch: "Thay file gốc" trong Cấu hình nâng cao |
| KS-05 | Có (một phần) | L1024, L1049–1058 | Đủ số thứ tự, icon, tên, 4 chip. **Không có nhãn "Đã có N chữ ký"** |
| KS-06 | Có | L613, L963–975, L1060 | Khớp |
| KS-07 | Có | L1054–1057 | Khớp; thứ tự mục: Tất cả, Chờ ký, Đã ký, Lỗi, Đang ký |
| KS-08 | Có | L1063–1070; CSS L153–160 | **Rail là trạng thái mặc định** mỗi lần vào màn Ký số (L644, L755), không phải tuỳ chọn phụ |
| KS-09 | Có (mô phỏng) | L1083–1089 | Trang là HTML giả, không pdf.js; mỗi file chỉ một trang; có nút trước/sau, ô số, zoom |
| KS-10 | Có | L1086–1087 | Khớp: "Đã ký số" / "Lỗi" / "Đang ký…"; hộp "Không ký được" + lý do + tooltip cách xử lý |
| KS-11 | Có (một phần) | L1084 | Có số trang, trang ký, vị trí, kiểu hiển thị. **Không có "khổ giấy nhận diện được"** (chỉ ghi "mẫu Khổ A4") |
| KS-12 | Có | L984–1005, L1033, L1037–1040 | "Lưu nhật ký…" là `noop`. Nút "Xóa" thực chất **xoá cả kết quả lô** (L1143) vì nhật ký sinh từ trạng thái |
| KS-13 | Có | L1027–1035, L1097–1098, L977 | Khớp. Có thêm trạng thái "Tạm dừng" và "Đã xong" |
| KS-14 | Có | L1105–1108 | Khớp chuỗi nhãn. Nút tên là "Mở thư mục" (`noop`), chỉ hiện khi đã xong hết |
| KS-15 | Có | L976, L1116 | Nút ghi **"Ký số"**, không ghi số lượng. Câu "Ký n văn bản đang hiện" nằm trong hộp PIN (L1450); chân màn ghi "Sẵn sàng ký n văn bản đang hiện" chỉ khi có từ khoá tìm (L1100) |
| KS-16 | Có | L1455–1467 | Tiêu đề "File đã ký / trùng tên". Thứ tự nút: Hủy · Giữ cả hai · **Ghi đè** (nút chính) |
| KS-17 | Có dạng hộp PIN | L1447–1454 | Prototype là hộp "Nhập mã PIN" có ô PIN và "Ghi nhớ trong phiên làm việc này"; nút chính đã tên "Xác nhận ký". Thông tin có: token, số file, (tooltip) CA, thiết bị, kiểu hiển thị. **Không có nơi lưu** |
| KS-18 | Có | L1468–1474 | Khớp; nhãn "Tổng số" / "Thành công" / "Lỗi" |
| KS-19 | Không | — | |
| KS-20 | Không | — | Chỉ có dòng nhật ký giả về token hết phiên (L996) |
| KS-21 | Khác | L616, L998 | Prototype coi DOCX như PDF: ký ra `…_signed.docx`. Không có bước chuyển sang PDF hay thông báo |
| KS-22 | Có (mô phỏng) | L1478, L1644 | Nút "Ký số các văn bản này" nạp lại danh sách `FILES` giả, không phải file vừa tạo |
| KS-23 | Có (mô phỏng) | L996, L1610–1623 | Xem CH-08 |

**Xem trước và vị trí chữ ký (VT)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| VT-01 | Có | L1389 | `<select>` "Tất cả file" / "Từng file" |
| VT-02 | Có | L1404–1405, L927–930 | Luôn hiện **8 file đầu của `FILES`** (`XT_N=8`), không theo danh sách đã nạp |
| VT-03 | Có | L824–857, L915–922 | Dùng sự kiện mouse; rộng tối thiểu 10%, cao tối thiểu 4%; không khoá tỉ lệ |
| VT-04 | Có | L1391, L1420 | Khớp, bước 10 |
| VT-05 | Có | L1410 | Khớp: "X n% · Y n% · W × H%" và "đặt riêng k/8 file" |
| VT-06 | Có | L1392–1402 | Vị trí neo là `<select>` 6 mục (không phải ô 6 điểm như trong Cấu hình). Đổi "Vị trí theo mẫu" **ghi thẳng vào mẫu cấu hình** (L1423) |
| VT-07 | Có | L1411, L1419 | Khớp; có thêm nút "Quay lại" ở góc trên trái |
| VT-08 | **Không** | — | Màn Vị trí không có chọn trang. Trang ký chỉ có ở mẫu trong Cấu hình (3 lựa chọn, không có "trang cụ thể") |
| VT-09 | Có | L1409 | Khớp nguyên văn "Chữ ký đang để ẩn — khung chỉ để tham khảo" |

**Tạo văn bản (TV)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| TV-01 | Có (mô phỏng) | L1186 | |
| TV-02 | Chỉ có chữ | L1186, L1265 | Placeholder "Chọn mẫu .docx hoặc .html"; mẫu giả luôn là `.docx` |
| TV-03 | Có | L1188, L1191 | Sheet: 3 mục cứng; dòng tiêu đề: ô số `min=1` |
| TV-04 | Chỉ có chữ | L1191 | Placeholder "Chọn file Excel hoặc CSV" |
| TV-05 | Có | L1193–1199 | Bấm dòng = **chọn bản ghi để xem trước**, không phải đặt dòng tiêu đề. Dòng tiêu đề đặt bằng ô số; bảng chỉ hiện 7/10 cột |
| TV-06 | Có | L1201, L1205, L1247–1249 | Nút "Ảnh thẻ…", "Con dấu…" là `noop` |
| TV-07 | Có | L1219–1237 | Khớp. Dấu trạng thái chưa ghép là chấm **đỏ** "!", còn chip biến là **cam** |
| TV-08 | Có | L1212, L1171 | |
| TV-09 | Có | L1164, L1176, L1179 | |
| TV-10 | Có (mô phỏng) | L1238–1245, L1270–1272 | Xem trước là HTML, không phải PDF thật |
| TV-11 | Có (một phần) | L1203–1209 | Có Nguồn dữ liệu, Định dạng, Tên file. **Không có ô thư mục lưu** dù tooltip chân màn ghi "Đổi định dạng, thư mục, tên file ở Nâng cao." (L1277) |
| TV-12 | Không | — | Chỉ có chỉ báo "Chưa ghép n biến" ở tab Mẫu; bấm Tạo là thành công ngay |
| TV-13 | Không | — | |
| TV-14 | Một phần | L1299, L1475–1479 | Có hộp "Tạo văn bản thành công" với "Ký số các văn bản này", "Mở thư mục". **Không có tiến độ, không có dừng** |
| TV-15 | Có (mô phỏng) | L1266, L1285 | Link "mở ví dụ mẫu" chỉ bật cờ mẫu + dữ liệu |

**Kiểm tra chữ ký số (KT)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| KT-01 | Có (một phần) | L1313–1314 | Có chọn file và nút "Kiểm tra lại". Không có kéo thả |
| KT-02 | Có | L1318–1320 | Danh sách chỉ có icon trạng thái, tên, thời điểm |
| KT-03 | Có | L1329–1331 | Khớp nhãn "Người ký", "Đơn vị", "Thời điểm ký" |
| KT-04 | Có (một phần) | L1332–1338 | Có: Vai trò, Lý do ký, Vị trí chữ ký, Nhà cung cấp CA, Số serial, Hiệu lực, Chuỗi chứng thư. **Không có:** nơi ký, TSA, tình trạng thu hồi |
| KT-05 | Có | L1344–1349 | Khớp; chỉ "Trang 1 / 1" |
| KT-06 | Có | L1353 | Khớp nguyên văn |
| KT-07 | Có (vẽ) | L1354 | `noop` |
| KT-08 | Không | — | |

**Cấu hình (CH)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| CH-01 | Có | L1558–1563, L1651–1652 | Không có cách đổi mẫu mặc định |
| CH-02 | Có | L1564–1567 | Khớp (tỉ lệ 40–200 bước 10) |
| CH-03 | Có | L1569–1576 | Khớp. Lề ngang 0–100, dọc 0–150 mm |
| CH-04 | Có | L1546–1557, L1577 | |
| CH-05 | Không | — | Đúng như kế hoạch ghi "prototype thiếu". Khối chữ cố định 3 dòng: "Ký bởi", "Ngày ký", "Serial" (L912) |
| CH-06 | Không | — | |
| CH-07 | Có | L1580–1591, L1603–1629 | Menu ⋯ gồm "Chi tiết…", **"Cài đặt tự động ký…"**, "Đặt làm mặc định", "Xóa khỏi danh sách". **"Kiểm tra token" là nút trong hộp Chi tiết**, không nằm trong menu ⋯. "Làm mới" không có hành vi |
| CH-08 | Có | L1610–1623 | Mặc định prototype là **bật** lưu PIN và tự nhập lại (`savePin:true, autoPin:true`); kế hoạch D2 yêu cầu tắt sẵn. PIN lưu dạng rõ trong `S.tk.pin` |
| CH-09 | Có (một phần) | L1592–1602 | Thư mục lưu là **một ô đường dẫn** (không có chọn "DaKySo hay cố định"). Trùng tên là công tắc 2 trạng thái Bật/Tắt (không có "hỏi") |
| CH-10 | Có | L1636, L1663–1664 | Khớp nhãn. Nhưng mọi thay đổi **ghi thẳng vào `S`** ngay khi sửa; "Hủy" không hoàn tác; "Lưu cấu hình" chỉ bật cờ `saved`. "Khôi phục mặc định" không đặt lại danh sách token |

**Hỗ trợ (HT)**

| Mã | Có? | Ở đâu | Khác biệt |
|---|---|---|---|
| HT-01 | Có | L1491–1520 | Đúng 7 mục. Có nút mục trước/mục sau và dòng phím tắt ở chân hộp |
| HT-02 | Có (mô phỏng) | L1533–1539 | 2 dòng chữ giữ chỗ; nút tên **"Tải driver"** (không phải "Cài"); không nhắc bit4id, TokenManager, WebView2 |
| HT-03 | Có (mô phỏng) | L1521–1526 | Chỉ vẽ trạng thái "có bản mới"; không có trạng thái "đang kiểm tra", "đã mới nhất", "lỗi mạng" |
| HT-04 | Có | L1527–1532 | Khớp đủ 7 trường |

### 8.2 Mục 2 — bằng chứng trong prototype cho D1–D9

| QĐ | Bằng chứng | Ghi chú |
|---|---|---|
| D1 | Chú thích L859 "cùng thông số logo_sao_mai.rs của prototype"; L10 "layout theo prototype sm-dema4" | File này tự mô tả là bản dựng lại theo một prototype khác; chưa chắc prototype kia là gì |
| D2 | Hộp PIN L1447–1454; cài đặt L1610–1623; mặc định L628; nhật ký L996; quảng bá ở L942, L1497, L1523 | Tính năng tự nhập PIN được nhắc ở 5 nơi (thẻ Trang chủ, Hướng dẫn, "Có gì mới", nhật ký, Cấu hình) — nếu lùi sang 1.6 phải sửa hết các câu này |
| D3 | Checkbox L1018; hộp trùng tên L1461–1466; công tắc L1599 | |
| D4 | `outDir`, `suffix`, `suffixErr` L628; L1086, L1102, L1472 | Prototype nói file lỗi "được đặt hậu tố _loi" nhưng không thể hiện file nào được tạo/đổi tên |
| D5 | `FILES` có `.docx` L616; regex L998 | Prototype ngầm định ký DOCX trực tiếp |
| D6 | Khối chữ "Ngày ký: 24/10/2026 14:22" L912; "Thời điểm ký" L1331 | Không phân biệt giờ máy và giờ TSA |
| D7 | L1521–1526: "Tải từ máy chủ cập nhật của Sao Mai; không gửi dữ liệu văn bản nào." | |
| D8 | L1186 ".docx hoặc .html"; L1206 PDF/DOCX | |
| D9 | L671–676 | |

### 8.3 Mục 13 — chức năng bản cũ

Xác nhận **cả 9 dòng** của bảng mục 13 đều không có trong prototype: không có ô lý do ký/nơi ký trong Cấu hình (chỉ thấy "Lý do ký" ở kết quả Kiểm tra, L1335); không có font/cỡ/màu chữ; chỉ có **một** ảnh chữ ký cho mỗi mẫu (không tách dấu đỏ và chữ ký tươi, không lặp theo trang chẵn/lẻ/tuỳ chọn); không có độ đậm/độ dày nét/màu mực; "Chỉ ảnh" là một kiểu hiển thị (L1567); không kiểm tra vị trí trước khi ký; không đóng dấu khi tạo văn bản; không cảnh báo theo dòng; không chọn nhiều file.

### 8.4 Mục 20 — ba điểm chưa nhất quán kế hoạch đã nêu

Cả ba đều **xác nhận đúng** (chi tiết và trích dẫn ở mục 10, các số 1, 2, 3). Mục 10 bổ sung thêm các điểm kế hoạch chưa nêu.

---

## 9. Chênh lệch hai chiều

### 9.1 Có trong prototype mà kế hoạch (mục 12) không nhắc

1. **Panel giới thiệu 3 bước** ở trạng thái rỗng của Ký số, Tạo văn bản, Kiểm tra (cả 2 tab), kèm logo góc phải dưới bấm về Trang chủ (`introHTML` L776–781).
2. **Rail là mặc định** ở màn Ký số (L644, L755).
3. Trạng thái tiến trình **"Tạm dừng"** và **"Đã xong"** trên hàng tab; chân màn có 5 trạng thái chữ (L1096–1103).
4. Ô **nhập số thứ tự** để nhảy tới file/bản ghi ở cả Ký số (`#ksSo`) và Tạo văn bản (`#tvbSo`).
5. Xem trước **tự đi theo file đang ký** và tự cuộn khung chữ ký vào giữa (L747–748, L1125).
6. Hộp PIN có **"Ghi nhớ trong phiên làm việc này"** và nút hiện/ẩn PIN (L1451–1452).
7. Nhật ký có nhãn `TOKEN` và bảng màu theo nhãn (L986–1002).
8. Trường **"Vai trò"** (Người ký chính / Ký nháy) trong chi tiết chữ ký (L1308–1309, L1335).
9. Hộp **"Thêm chứng thư số / USB Token"** với nút "Quét lại"; nhãn **"Đang dùng"** trên token đầu (L1583, L1624–1628).
10. Hộp **"Chi tiết chứng thư số"** có "Chủ sở hữu", "Thiết bị" (L1607).
11. Tuỳ chọn "Giữ phiên token" có 4 giá trị cụ thể; "Khi nhập PIN lỗi" có 2 giá trị cụ thể (L1618–1619).
12. Danh sách ảnh chữ ký chọn sẵn cho mẫu (L1574).
13. Hộp Hướng dẫn có **nút mục trước/sau** và dòng phím tắt ở chân (L1518).
14. Nút "Cấu hình" mở lại đúng tab lần trước và sáng lên khi hộp mở (L688, L705).
15. Nút "Quay lại" ở màn Vị trí (L1388).
16. Hai lối vào màn Vị trí với khác biệt nhỏ (`xemtruoc` mở sẵn Nâng cao).
17. Nút mũi tên **xem/ẩn bảng Excel** và link "Đóng" (L1188, L1195).
18. Gõ **giá trị cố định + Enter** ngay trong menu chọn nguồn (L799, L1227).
19. Tooltip chân Trang chủ có "Bản quyền: Giải pháp số Sao Mai" (L952); hộp Thông tin có dòng bản quyền và câu pháp lý (L1530).
20. Menu lọc có cả mục "Đang ký".
21. Ký DOCX giữ nguyên đuôi `.docx` (L998).
22. Chú thích "(kiểu ABBYY 60/40)" cho bố cục màn Vị trí (L1368) — cột trái mặc định 480px.

### 9.2 Kế hoạch yêu cầu mà prototype không có (phải tự thiết kế)

| Mã | Thiếu gì |
|---|---|
| SH-02 | Chạy một bản |
| SH-05 | Cập nhật token định kỳ; trạng thái chuyển tiếp khi rút token |
| SH-07 | Lưu độ rộng cột/zoom/tab giữa các lần chạy |
| SH-08, KT-01 | Kéo thả file/thư mục vào cửa sổ (không có vùng thả, không có trạng thái hover khi kéo) |
| KS-01 | Chọn nhiều file |
| KS-02 | Kiểm tra định dạng/dung lượng ảnh; thông báo lỗi ảnh |
| KS-05, D4 | Nhãn "Đã có N chữ ký" |
| KS-09 | Xem trước PDF thật (nhiều trang, trang xoay, khổ khác A4); trạng thái đang tải / lỗi tải |
| KS-11 | Khổ giấy nhận diện được |
| KS-15 | Nhãn nút có số lượng |
| KS-17 | Hộp "Xác nhận ký" không có ô PIN, có nơi lưu |
| KS-19 | Kiểm tra vị trí trước khi ký và danh sách cảnh báo |
| KS-20 | Rút token giữa chừng |
| KS-21 | Thông báo chuyển DOCX → PDF |
| VT-08 | Chọn trang đặt chữ ký |
| TV-11 | Ô chọn thư mục lưu |
| TV-12 | Kiểm tra trước khi tạo (biến chưa ghép, ô trống, tên trùng) |
| TV-13 | Đóng dấu đỏ / chữ ký tươi |
| TV-14 | Tiến độ và nút dừng khi tạo lô |
| KT-04 | Nơi ký, TSA, tình trạng thu hồi |
| KT-08 | Bật/tắt kiểm tra thu hồi trực tuyến |
| KT (chung) | **Mọi trạng thái không hợp lệ**: chữ ký sai, văn bản bị sửa, chứng thư hết hạn/thu hồi/không tin cậy, không xác định, file không có chữ ký |
| CH-01 | Đổi mẫu mặc định |
| CH-05, CH-06 | Lý do ký, nơi ký, font, cỡ, màu, dòng thông tin; độ đậm, độ dày nét, màu mực |
| CH-09 | Chế độ thư mục con `DaKySo` so với thư mục cố định; "Chép file lỗi vào DaKySo" |
| CH-10 | Hành vi nháp/huỷ thật sự |
| D3 | Hộp xác nhận lần đầu khi bật thay file gốc |
| HT-02 | Trạng thái bit4id, TokenManager, WebView2; nút Cài |
| HT-03 | Các trạng thái khác của kiểm tra cập nhật |
| Mục 13 | "Thêm dấu / chữ ký tươi" trong Nâng cao của màn Vị trí |
| Chung | Hộp báo lỗi chung, trạng thái đang tải, thông báo khi không có token/không có chứng thư, token chưa cắm khi bấm Ký số |

---

## 10. Điểm không nhất quán trong prototype

| # | Vấn đề | Dòng | Trích nguyên văn |
|---|---|---|---|
| 1 | **Số phiên bản lệch nhau.** Hộp cập nhật nói đang dùng 1.4.2 và có bản mới 1.5.0, trong khi Trang chủ và Thông tin nói đang là 1.5.0 | L1522 / L952 / L1529 | "Đã có phiên bản mới 1.5.0" · "Đang dùng 1.4.2 · phát hành 28/10/2026" / `Phiên bản<b>1.5.0</b>` / `'1.5.0 (build 2026.10.28)'` |
| 2 | **Thư mục DaKySo: thư mục con hay đường dẫn cố định.** Lời văn nói thư mục con của thư mục nguồn; dữ liệu là một đường dẫn tuyệt đối sửa được | L948, L1018, L1594 / L628, L1594 | "Giữ nguyên file gốc; file đã ký lưu riêng thư mục DaKySo." · "Mặc định tạo thư mục con DaKySo bên trong thư mục cần ký." / `outDir:'D:\\VanBan\\CanKy\\DaKySo'` |
| 3 | **Hộp "Ghép trường" không mở được.** Có hàm vẽ và handler nhưng không phần tử nào mang `data-act="tvb.ghep"`. Hướng dẫn vẫn nhắc tới | L1298, L1441, L1480–1488 / L1502 | `ACTS['tvb.ghep']=function(){openModal('ghep');};` / "Kiểm tra Ghép trường (tự khớp theo tên cột)." |
| 4 | **Màu "chưa ghép" lệch nhau:** hộp Ghép trường nói đỏ, tab Mẫu nói cam (chip cam, nhưng dấu "!" lại đỏ) | L1487 / L1271 / L229, L237 | "● xanh = đã ghép · ● đỏ = chưa ghép" / "Biến xanh = đã ghép nguồn, cam = chưa ghép." |
| 5 | **Chú thích nhắc tới file Rust không tồn tại trong thư mục** | L859 | `/* ============================ LOGO ĐỘNG (cùng thông số logo_sao_mai.rs của prototype) ============================ */` |
| 6 | **Chú thích tự gọi một "prototype" khác** | L10, L595, L762, L1541 | "layout theo prototype sm-dema4" · "cùng nét với prototype" · "ô nhập có nút xoá (giống prototype)" · "Cấu hình (hộp thoại có tab, như prototype)" |
| 7 | **Chú thích tooltip rỗng và lệch ký hiệu:** L117 gọi là "ⓘ" nhưng không có CSS; tooltip thật dùng "!" | L117 / L465 | `/* tooltip ⓘ */` / `/* ---------- Nút ! : bấm để xem thông tin ---------- */` |
| 8 | **Hai công thức khung chữ ký khác nhau** cho cùng một mẫu: xem trước Ký số/Vị trí và xem trước trong Cấu hình | L660 / L1549 | `var w=clamp(24*p.scale/100+10,16,62), h=w*0.27;` / `sw=clamp(Math.round(W*0.32*(p.scale/100)),26,W-8),sh=Math.min(Math.round(sw*0.42),H-8)` |
| 9 | **`sigRect` bỏ qua khổ giấy và chiều giấy** của mẫu (luôn chia cho 210/297), trong khi mẫu A3/A2/A1 mặc định là "Ngang" | L661 | `var mx=p.mx/210*100, my=p.my/297*100;` |
| 10 | **Không có chọn mẫu theo khổ giấy** dù giao diện khẳng định có | L667 / L1563, L1499 | `if(v==='auto'\|\|v==='keo')return S.profiles[0];` / "Khi ký, app chọn mẫu theo khổ giấy của từng văn bản." · "App tự chọn mẫu theo khổ giấy của từng văn bản." |
| 11 | **Hai nguồn "kiểu hiển thị":** mỗi mẫu có `mode` riêng (chỉ ảnh hưởng khung xem trước trong Cấu hình), còn màn Ký số và Vị trí dùng `S.ks.mode` toàn cục | L622–626, L1567 / L644, L1146, L1400 | `mode:'Ảnh + thông tin'` (trong mẫu) / `data-chg="ks.mode"` |
| 12 | **Hai nguồn "ảnh chữ ký":** ô "Chữ ký / con dấu" trên màn Ký số và trường `img` của mẫu | L1020 / L1574 | `'D:\\ChuKy\\condau_saomai.png'` / `data-arg="img"` |
| 13 | **"Xóa file gốc" không nhất quán với kết quả:** bật lên thì hộp trùng tên và tooltip nói ghi đè file gốc, nhưng nhật ký và hộp Hoàn thành vẫn ghi `→ …_signed.pdf` và "lưu tại DaKySo" | L1018, L1461, L1100 / L998, L1002, L1472 | "Bật: thay file gốc bằng file đã ký." · `'ghi đè ngay lên file gốc'` / `list[ab]+' → '+list[ab].replace(...)` · "Đã lưu vào thư mục DaKySo" |
| 14 | **Câu hộp trùng tên sai ngữ pháp khi bật "Xóa file gốc":** thành "…trùng tên trong ghi đè ngay lên file gốc" | L1461–1462 | `var noi=k.delOld?'ghi đè ngay lên file gốc':'thư mục '+S.tk.outDir;` … `' file đã ký trùng tên trong '+noi+'.'` |
| 15 | **Hai cơ chế trùng tên không nối với nhau:** công tắc "Ghi đè khi trùng tên" trong Cấu hình (`S.tk.overwrite`) không ảnh hưởng tới hộp hỏi; lựa chọn trong hộp (`dupMode`) chỉ được ghi, không được đọc | L1599 / L1141–1142 | "Ghi đè khi trùng tên" · "Tắt: tự thêm (1), (2)… vào tên file mới." / `S.ks.dupMode='overwrite'` |
| 16 | **Hậu tố lỗi `_loi`:** lời văn nói file lỗi "được đặt hậu tố" nhưng không nơi nào thể hiện file đó (nhật ký chỉ ghi lý do) | L1086, L1102, L1472 / L997 | "File lỗi được đặt hậu tố _loi." · "File lỗi thêm hậu tố _loi." / `list[ab]+' – '+FAIL[ab]` |
| 17 | **"Sửa lỗi rồi bấm Ký số để ký lại" không đúng với mã:** file lỗi tính là đã xong nên "Ký tiếp" bỏ qua; chỉ "Ký lại" (ký lại toàn bộ file đang hiện) mới chạm tới | L1086 / L976, L1140 | "Sửa lỗi rồi bấm Ký số để ký lại." / `return !k.done[i];` |
| 18 | **Nút "Xóa" nhật ký xoá luôn kết quả ký** (mọi chip về "Chờ ký") | L1033, L1143 | `ACTS['ks.clearLog']=function(){if(!S.ks.running){S.ks.done={};S.ks.queue=[];…}` |
| 19 | **Màn Vị trí không dùng danh sách đã nạp:** luôn 8 file đầu của `FILES`, kể cả khi đang ở chế độ 1 file hoặc chưa nạp gì | L1364, L1405 | `var XT_N=8;` · `for(var i=0;i<XT_N;i++){…esc(FILES[i])…}` |
| 20 | **Đổi "Vị trí theo mẫu" ở màn Vị trí sửa thẳng mẫu cấu hình** mà không qua "Lưu cấu hình" | L1423 | `CHG['xt.pos']=function(el){S.profiles[S.tc.prof\|\|0].pos=el.value;tcSet(tcTpl());render();};` |
| 21 | **Cấu hình không có nháp:** sửa là ghi ngay; "Hủy" không hoàn tác; chú thích chân lại nói ngược | L1543–1544, L1636, L1664 | "Thay đổi áp dụng ngay khi bấm Lưu" · `ACTS['cfg.save']=function(){S.saved=true;render();};` |
| 22 | **Hộp "Cài đặt tự động ký" mở theo token nhưng lưu chung;** "Hủy" và "Lưu" đều chỉ đóng hộp | L1611–1622 | `foot:'<button class="nut" data-act="cfg.subClose">Hủy</button><button class="nut chinh" data-act="cfg.subClose">Lưu</button>'` |
| 23 | **Dòng nhật ký tự nhập PIN luôn xuất hiện** (trước file thứ 13 khi ký thư mục) dù cài đặt bật hay tắt | L996 | `if(isF&&ab===12)lines.push({…tag:'TOKEN',…text:'Phiên token hết hạn – tự động nhập lại mã PIN từ cấu hình… thành công'});` |
| 24 | **Tên chức năng không thống nhất:** "Kiểm tra chữ ký số" (menu, tiêu đề) và "Kiểm tra chữ ký" (thẻ Trang chủ, Hướng dẫn); "Vị trí chữ ký (kéo thả)" (menu), "Vị trí chữ ký" (thẻ), "Xem trước & vị trí chữ ký" (tiêu đề màn); "Tạo văn bản" và "Tạo văn bản từ mẫu" | L685, L944, L1504 / L686, L945, L670 / L684, L1501 | |
| 25 | **Hai mã màn cho một màn** và dấu ✓ menu lệch: vào từ nút "Cấu hình chữ ký" thì ✓ nằm ở "Ký số văn bản", vào từ menu thì ✓ ở "Vị trí chữ ký (kéo thả)" | L683, L686, L738–739 | `on:cur==='kyso'\|\|cur==='xemtruoc'` |
| 26 | **Tooltip tên file nói một đằng làm một nẻo:** "Ký tự không hợp lệ được bỏ, trùng tên thêm (1), (2)…" nhưng mã thay bằng "-" và không xử lý trùng | L1207 / L1160 | `.replace(/[\\/:*?"<>\|]+/g,'-')` |
| 27 | **Tooltip chân màn Tạo văn bản dùng chuỗi cứng** thay vì mẫu tên đang nhập, và hứa có ô thư mục | L1277 | "Lưu tại: D:\VanBan\KetQua\nTên file: {so}_{hoten}\nĐổi định dạng, thư mục, tên file ở Nâng cao." |
| 28 | **"Bấm biến trên trang để sửa nguồn" không hoạt động** — phần tử `.bien` trên trang không có `data-act` | L1271 / L1171 | `'<span class="bien '+(tvbMapped(k)?'da':'chua')+'">'+txt+'</span>'` |
| 29 | **Dòng tiêu đề Excel tô lên dòng dữ liệu:** bảng không có dòng tên cột thật; dòng dữ liệu thứ `hrow` bị tô làm "tiêu đề" | L1197 | `var isH=(i+1)===S.tvb.hrow;` |
| 30 | **Nhãn trường lệch:** biến `khoahoc` là "Tên khóa đào tạo" trong form nhưng "Khóa đào tạo" trong cột; hộp Ghép trường ghi "Đơn vị" thay vì "Đơn vị công tác" | L1150 / L1165 / L1482 | |
| 31 | **Đổi cột ghép không đổi kết quả xem trước** (mô phỏng) | L1172 | `return esc(r[k]);` |
| 32 | **Va chạm tên class `.tieude`** giữa thanh tiêu đề và dòng tiêu đề Excel | L36 / L276–277 | `.tieude{height:32px;…}` / `.xls-row.tieude{background:#fff4ce;color:#7a5c00}` |
| 33 | **CSS chồng lấn / tự ghi đè:** `.nc-panel` khai `grid-template-columns` 3 lần (L485, L497, L498) và viền trái 3px bị L521 huỷ; `#manHome` khai 2 lần (L323, L456–458); `.dong.dang`, `.nav.chon::before`, `.thumb` bị L522–524 huỷ | L485–524 | `/* bỏ mọi vạch xanh bên trái */` |
| 34 | **CSS không dùng:** `.an` (L29), `.gan` (L75), `.huy` (L102), `.tiendo` (L298–300), `.nhan-pt` (L291), `.tc-diem` (L339–342), `.thumb` (L351–355), `.drawer` + `.muc` (L358–360), `.pos-grid` (L422), `.sub` (L414), `.tb-noi` (L515–516), `.seg-nhom.nho` (L517–518), `.tc-the .phim` (L336), `.lt-col .vn em` (L258), `.chuky.keo-duoc` (L216; class `keo-duoc` thực tế gắn vào `.chuky-o`). `.phai-hang{width:430px}` (L83) không bao giờ có hiệu lực vì mọi chỗ dùng đều kèm `.hep` | | |
| 35 | **JS không dùng:** hàm `giay()` (L768), `hienThiBar()` (L1376); handler `cfg.open`, `tc.mode`, `ks.modeSet`, `ACTS['xt.pos']`; trường `S.ks.idx`, `S.ks.dupMode`; nhánh `S.tc.mode==='config'` (L752, L754); biến `done` trong `ksStats()` (L988); nhánh `keo?…:'Theo mẫu…'` (L1410, vì `keo=true` cố định ở L1386); 7 icon `monitor`, `play`, `lock`, `user`, `layers`, `mail`, `globe` | | |
| 36 | **Thứ tự nút hộp Trùng tên đặt "Ghi đè" làm nút chính** trong khi mặc định cấu hình là không ghi đè | L1466 / L628 | `<button class="nut chinh rong" data-act="dup.overwrite">…Ghi đè</button>` / `overwrite:false` |
| 37 | **"Thêm token" có thể thêm trùng** (không loại khỏi danh sách phát hiện) | L1660 | |
| 38 | **Chi tiết nhỏ:** dấu `;;` thừa cuối L1018; thời gian nhật ký giả "20:52:xx" không khớp "Ngày ký: 24/10/2026 14:22" trong khung chữ ký (L982 / L912) | | |
| 39 | **Nút "Mở thư mục" ở hộp Hoàn thành và hộp Tạo văn bản chỉ đóng hộp** (`modal.close`), còn ở chân màn Ký số là `noop` | L1473, L1478 / L1105 | |
| 40 | **Mặc định bật lưu PIN** trong khi chính tooltip cảnh báo "Chỉ nên bật trên máy cá nhân" | L628 / L1612 | `savePin:true,pin:'12345678',autoPin:true` |

---

## 11. Tài nguyên nhúng

### 11.1 Ảnh base64 (tất cả PNG, RGBA 8-bit)

| Tên | Kích thước | Dung lượng | Khai báo | Dùng ở |
|---|---|---|---|---|
| Icon ứng dụng | 64×64 | 5 697 byte (7 596 ký tự base64) | **Lặp 3 lần, cùng nội dung:** favicon L7; nền `.tieude .ico-app` L37; `IMG.app` L534 | Hình: chữ "S" xanh có quỹ đạo và ngôi sao cam 4 cánh, không có chữ. Dùng làm favicon và icon 16px trên thanh tiêu đề. `IMG.app` **không được tham chiếu** trong JS |
| `IMG.logo` | 300×246 | 61 532 byte (82 044 ký tự) | L534 | Hình: logo đầy đủ (chữ "S" xanh + dòng chữ "SAO MAI" / "DIGITAL SOLUTIONS") **đã bỏ ngôi sao** ở góc trên phải. Dùng làm ảnh nền của logo động Trang chủ (L886); ngôi sao được phủ riêng bằng `IMG.sao` |
| `IMG.sao` | 46×46 | 2 874 byte (3 832 ký tự) | L534 | Hình: ngôi sao cam 4 cánh. Phủ lên logo động, có hoạt ảnh mờ/nhấp nháy (L886, CSS L328) |
| `IMG.full` | 200×164 | 34 074 byte (45 432 ký tự) | L534 | Hình: logo đầy đủ **có ngôi sao**. Dùng ở góc phải dưới panel giới thiệu (L780, rộng 60px) và đầu hộp Thông tin (L1528, rộng 150px, `alt="Sao Mai Digital Solutions"`) |

Tổng base64 ≈ 153 KB / 319 KB dung lượng file. Khi port nên tách thành file ảnh trong `assets/` và dùng một bản icon.

### 11.2 SVG nội tuyến

- **57 icon nét** trong `P` (L537–591), `viewBox="0 0 24 24"`, vẽ qua `ic()`; style chung ở `svg.i` (L30): `fill:none; stroke:currentColor; stroke-width:1.75; stroke-linecap:round; stroke-linejoin:round`. Chú thích ghi "kiểu Lucide" — đây là path chép tay, **không** nạp thư viện Lucide. Danh sách: `grid, gear, help, home, pen, tao, shield, frame, refresh, info, book, usb, file, doc, folder, folderOpen, image, eye, x, xc, search, chevL, chevR, chevD, chevU, panelL, panelR, filter, zoomIn, zoomOut, check, checkC, alert, warn, clock, terminal, list, monitor, save, trash, back, table, link, stamp, key, download, stop, play, edit, star, lock, user, plus, dots, layers, mail, globe`. Không dùng: `monitor, play, lock, user, layers, mail, globe`.
- **4 icon lớn** trong `LA` (L596–601), `viewBox="0 0 100 100"`: `ky`, `tao`, `kiemtra`, `vitri`; mỗi path tự mang `stroke-width` (2 / 2.2 / 2.4 / 2.5 / 3). Dùng ở thẻ Trang chủ (52px, class `la`) và panel giới thiệu (70px, class `logo-cn`).
- Icon quay `spin()` (L931): một cung tròn `M21 12a9 9 0 1 1-6.2-8.56`.
- 3 icon thanh tiêu đề (L673–675), `viewBox="0 0 10 10"`.
- SVG phủ của logo động (L885–886), `viewBox="0 0 300 246"`.

### 11.3 Font

Không nhúng, không tải font. Dùng font hệ thống theo thứ tự `--font` và `--mono` (mục 2.1); văn bản mẫu dùng Times New Roman.

### 11.4 Thư viện ngoài / CDN

**Không có.** Không có thẻ `<script src>`, không có `<link rel="stylesheet">`, không `@import`, không `fetch`. Thẻ `<link>` duy nhất là favicon (L7). URL ngoài duy nhất là liên kết cho người dùng bấm: `https://saomaiso.com` (L947, L951, L1529) và `mailto:hotro@saomaiso.com` (L951, L1529).

---

## Phụ lục A — Toàn bộ nội dung tooltip `!`

File có 31 chỗ gọi `tip()`. Bảng dưới có thêm 3 dòng L1074, L1264, L1323 là nội dung truyền qua `introHTML()` rồi hiện ở chỗ gọi L779. `\n` là xuống dòng. Phần trong `{…}` là giá trị động.

| Dòng | Vị trí | Nội dung |
|---|---|---|
| L721 | Menu bar, chỉ báo token | "Chứng thư: {tên}\nNhà cung cấp: {CA}\nThiết bị: {thiết bị}\nTrạng thái: {trạng thái}" |
| L779 | Tiêu đề panel giới thiệu | (theo từng màn, xem L1074, L1264, L1323) |
| L948 | Trang chủ, cạnh "Chọn chức năng để bắt đầu" | "Ký ngay trên máy — văn bản không gửi lên máy chủ.\nGiữ nguyên file gốc; file đã ký lưu riêng thư mục DaKySo.\nĐịnh dạng: PDF · DOCX · ảnh chữ ký PNG.\nPhím tắt: F1 hướng dẫn · Ctrl+K ký số · Ctrl+N tạo văn bản · Ctrl+E kiểm tra." |
| L950 | 4 thẻ Trang chủ | {mô tả thẻ} + "\nPhím tắt: {phím}" |
| L952 | Trang chủ, cạnh phiên bản | "Người dùng: [Tên người dùng]\nTên PC: PC-KYSO-01\nBản quyền: Giải pháp số Sao Mai" |
| L1018 | Ký số, "Xóa file gốc" | "Bật: thay file gốc bằng file đã ký.\nTắt: giữ file gốc; file đã ký lưu vào thư mục DaKySo, thêm hậu tố {_signed}." |
| L1074 | Ký số, giới thiệu | "Hỗ trợ PDF và DOCX.\nChèn hình chữ ký, con dấu theo mẫu từng khổ giấy.\nFile gốc giữ nguyên trừ khi chọn Xóa file gốc." |
| L1084 | Ký số, cạnh tên file | "{n} trang · ký {trang cuối}\nVị trí: {mẫu Khổ A4 · dưới phải \| tự cấu hình (kéo thả)}\nHiển thị: {ảnh + thông tin}" |
| L1086 | Ký số, hộp lỗi | "Sửa lỗi rồi bấm Ký số để ký lại.\nFile lỗi được đặt hậu tố {_loi}." |
| L1100 | Ký số, chân màn (sẵn sàng) | "Ký các văn bản đang hiện ở tab Văn bản.\nChứng thư: {tên}\nHiển thị: {kiểu}\nVị trí: {…}\nLưu tại: {thay file gốc \| thư mục}" |
| L1102 | Ký số, chân màn (xong) | "Lưu tại: {thư mục}\nFile lỗi thêm hậu tố {_loi}.\nXem lý do ở tab Nhật ký." |
| L1186 | Tạo văn bản, "Mẫu văn bản:" | "Mẫu có các trường dạng {hoten}, {so}… và khung ảnh; trường sẽ được thay bằng dữ liệu." |
| L1188 | Tạo văn bản, "Dòng tiêu đề" | "Dòng chứa tên cột trong file Excel." |
| L1207 | Tạo văn bản, "Tên file" | "Ghép tên trường trong ngoặc {}, ví dụ {so}_{hoten}.\nKý tự không hợp lệ được bỏ, trùng tên thêm (1), (2)…" |
| L1264 | Tạo văn bản, giới thiệu | "Mỗi dòng Excel tạo một văn bản, hoặc nhập trực tiếp một văn bản.\nChèn ảnh thẻ, con dấu vào khung.\nXuất PDF hoặc DOCX." |
| L1271 | Tạo văn bản, tab Mẫu | "Biến xanh = đã ghép nguồn, cam = chưa ghép.\nBấm biến trên trang để sửa nguồn." |
| L1277 | Tạo văn bản, chân màn | "Lưu tại: D:\VanBan\KetQua\nTên file: {so}_{hoten}\nĐổi định dạng, thư mục, tên file ở Nâng cao." |
| L1314 | Kiểm tra, hàng 1 | "Hỗ trợ PDF có một hoặc nhiều chữ ký, kể cả ký nháy." |
| L1323 | Kiểm tra, giới thiệu | "Kiểm tra người ký, thời điểm ký, hiệu lực chứng thư và văn bản có bị sửa sau khi ký." |
| L1329 | Kiểm tra, "Chữ ký hợp lệ" | "Văn bản không bị chỉnh sửa sau khi ký.\nChứng thư còn hiệu lực tại thời điểm ký." |
| L1348 | Kiểm tra, tab Văn bản | "Văn bản có {n} chữ ký.\nKhung cam: chữ ký đang chọn." |
| L1390 | Vị trí, cạnh "Chế độ" | "Tất cả file: một khung áp cho mọi văn bản.\nTừng file: chọn văn bản bên trái rồi đặt khung riêng.\nCả hai đều có thể lấy vị trí Theo mẫu ở Nâng cao rồi kéo chỉnh.\nKéo khung để di chuyển, kéo góc để đổi cỡ." |
| L1396 | Vị trí, "Theo mẫu" | "Lấy vị trí & cỡ chữ ký từ mẫu khổ giấy (lưu trong Cấu hình).\nÁp cho khung hiện tại, sau đó vẫn kéo chỉnh được." |
| L1450 | Hộp PIN | "Nhà cung cấp: {CA}\nThiết bị: {thiết bị}\nHiển thị: {kiểu}" (+ "\nXóa file gốc sau khi ký" nếu bật) |
| L1472 | Hộp Hoàn thành | "Lưu tại: {thư mục}" (+ "\n{n} file lỗi thêm hậu tố {_loi}; xem lý do ở Nhật ký.") |
| L1563 | Cấu hình, tên mẫu | "Mẫu tự áp dụng cho các văn bản khổ {A4}.\nKhi ký, app chọn mẫu theo khổ giấy của từng văn bản." |
| L1588 | Cấu hình, tab Token | "Bấm ⋯ ở mỗi dòng để xem chi tiết, đặt mặc định, xóa hoặc mở Cài đặt tự động ký (lưu PIN, tự nhập lại khi hết phiên)." |
| L1594 | Cấu hình, thư mục lưu | "Mặc định tạo thư mục con DaKySo bên trong thư mục cần ký." |
| L1595 | Cấu hình, hậu tố thành công | "Ví dụ: QD_001_2026{_signed}.pdf" |
| L1598 | Cấu hình, hậu tố lỗi | "Ví dụ: QD_001_2026{_loi}.pdf — tách riêng file ký lỗi." |
| L1599 | Cấu hình, ghi đè | "Tắt: tự thêm (1), (2)… vào tên file mới." |
| L1612 | Tự động ký, lưu PIN | "Khi đã lưu, ai dùng máy này cũng ký được bằng token đang cắm.\nChỉ nên bật trên máy cá nhân." |
| L1614 | Tự động ký, tự nhập lại | "Khi ký hàng loạt mà token hết phiên, app tự nhập lại PIN đã lưu và ký tiếp.\nCần bật Lưu mã PIN." |
| L1625 | Thêm token | "Cắm USB Token vào máy — token sẽ hiện ở đây.\nKhông thấy token? Kiểm tra cổng USB và driver (Hỗ trợ › Cài đặt driver USB Token)." |

## Phụ lục B — Nội dung nhật ký mô phỏng (`ksStats()` L984–1005)

| Nhãn | Màu | Nội dung |
|---|---|---|
| `INFO` | `#9aa3ae` | "Chưa chọn văn bản. Bấm "1 file" hoặc "Thư mục" để bắt đầu." |
| `INFO` | | "Đã nạp thư mục D:\VanBan\CanKy ({n} văn bản đang hiện). Bấm "Ký số" để bắt đầu." / "Đã nạp {file}. Bấm "Ký số" để bắt đầu." |
| `INFO` | | "Bắt đầu ký {n} văn bản (theo danh sách đang hiện)" / "Bắt đầu ký {file}" |
| `INFO` | | "USB Token sẵn sàng – chứng thư: {tên}" |
| `INFO` | | "Chữ ký: condau_saomai.png – hiển thị: {kiểu} – vị trí: {tự cấu hình \| theo khổ giấy}" |
| `TOKEN` | `#9aa3ae` | "Phiên token hết hạn – tự động nhập lại mã PIN từ cấu hình… thành công" |
| `OK` | `#6ccb7f` | "{file} → {file_signed}" |
| `LỖI` | `#ff9aa5` | "{file} – {lý do}" |
| `…` | `#7fb2ff` | "Đang ký {file}" |
| `INFO` | | "Đã dừng. Còn {n} văn bản chưa ký." |
| `INFO` | | "File đã ký lưu tại {thư mục}" |
| `XONG` | `#fff` | "{ok}/{tổng} thành công, {lỗi} lỗi" |

## Phụ lục C — Nội dung chữ của Hướng dẫn và "Có gì mới"

**Hướng dẫn (`HD` L1491–1511)** — tên mục, câu dẫn, tiêu đề các khối:

| # | Icon | Tên mục | Câu dẫn (`intro`) | Khối (kiểu danh sách) |
|---|---|---|---|---|
| 1 | `star` | "Bắt đầu nhanh" | "Tổng quan để ký văn bản lần đầu với Sao Mai Ký số." | "Chuẩn bị" (`ul`, 3 ý) · "Ký lần đầu" (`ol`, 4 bước) |
| 2 | `pen` | "Ký số văn bản" | "Ký 1 file hoặc cả thư mục PDF/DOCX." | "Các bước" (`ol`, 3) · "Khi token hết phiên" (`ul`, 3) |
| 3 | `frame` | "Vị trí chữ ký" | "Đặt chữ ký theo mẫu khổ giấy hoặc kéo thả tay." | "Theo mẫu" (`ul`, 2) · "Kéo thả" (`ul`, 2) |
| 4 | `tao` | "Tạo văn bản từ mẫu" | "Điền dữ liệu vào mẫu .docx/.html để sinh hàng loạt văn bản." | "Từ Excel" (`ol`, 3) · "Nhập trực tiếp" (`ul`, 2) |
| 5 | `shield` | "Kiểm tra chữ ký" | "Xác minh văn bản đã ký có hợp lệ và toàn vẹn." | "Các bước" (`ol`, 3) |
| 6 | `gear` | "Cấu hình" | "Mẫu chữ ký, USB Token, thư mục lưu." | "Các mục" (`ul`, 3) |
| 7 | `help` | "Câu hỏi thường gặp" | "Lỗi hay gặp và cách xử lý." | "Không nhận USB Token" (`ul`, 3) · "Ký lỗi một số file" (`ul`, 2) |

Nội dung từng ý nằm nguyên văn ở L1493–1510; một số ý có thẻ `<kbd>` (ví dụ `<kbd>Ctrl</kbd>+<kbd>K</kbd>`). Các câu phụ thuộc quyết định D2/D4 cần rà lại khi chuyển sang Markdown: "Bấm Ký số và nhập mã PIN." (L1494), cả khối "Khi token hết phiên" (L1497), "USB Token & mã PIN: thêm token, đặt mặc định, cài đặt tự động ký." (L1507), "Kiểm tra Ghép trường (tự khớp theo tên cột)." (L1502).

**"Có gì mới" trong hộp Kiểm tra cập nhật (L1523)** — 4 ý nguyên văn: "Kéo thả vị trí chữ ký riêng từng file." · "Tự nhập lại mã PIN khi token hết phiên lúc ký hàng loạt." · "Tách hậu tố tên file cho văn bản ký lỗi." · "Kiểm tra chữ ký số thành chức năng riêng."

## Phụ lục D — Từ điển tên class (tiếng Việt không dấu → nghĩa)

`cuaso` cửa sổ · `tieude` tiêu đề · `ten` tên · `cap` nút cửa sổ (caption) · `man` màn · `hang` hàng · `nhan` nhãn · `nhanphu` nhãn phụ · `o` ô (bọc input) · `xoa` xoá · `trong` trống · `fld` ô nhập · `so` số · `phai-hang` phần phải của hàng · `hep` hẹp · `hop` hộp (checkbox) · `nut` nút · `chinh` chính · `nho` nhỏ · `rong` rộng · `nutma` nút mờ (nút icon) · `lk` liên kết · `tron` trơn · `khung` khung · `than` thân · `cottrai` cột trái · `keo` kéo · `ds` danh sách · `ds-dau` đầu danh sách · `dong` dòng · `chon` (đang) chọn · `dang` đang (chạy / hiện tại) · `phuchu` phụ chú · `chip-tt` chip trạng thái · `cho` chờ · `loi` lỗi · `quay` quay · `daytrai` đáy cột trái · `tim` tìm · `dem` đếm · `thu` thu gọn · `loc` lọc · `trung` trùng · `phai` phải · `dieuhuong` điều hướng · `tong` tổng · `tt` tiêu đề / trạng thái · `dan` dãn (phần co giãn) · `zm` zoom · `ngan` vách ngăn · `vungxem` vùng xem · `giay` giấy · `nhan-trang` nhãn trang · `bao` báo · `canh` cảnh báo · `chuky` chữ ký · `da-ky` đã ký · `noi` nổi · `keo-duoc` kéo được · `dau-son` dấu son · `tt-ky` thông tin ký · `tay` tay nắm · `bien` biến · `da` đã (ghép) · `chua` chưa (ghép) · `bcham` chấm báo · `lt` "lấy từ" · `len` lên · `bo` bỏ · `mo-dau` mở đầu · `buoc` bước · `them` thêm · `logo-goc` logo góc · `chan` chân · `tiendo` tiến độ · `xong` xong · `dung` dừng · `tc-` Trang chủ · `the` thẻ · `luoi` lưới · `day` đáy · `phu` lớp phủ · `tren` trên · `dau-p` đầu popup · `than-p` thân popup · `cuoi` cuối (chân popup) · `trai` trái · `luoi-tt` lưới thông tin · `icon-lon` icon lớn · `so-lon` số lớn · `cauhinh` cấu hình · `tabch` tab cấu hình · `bang-tk` bảng token · `nguy` nguy hiểm · `hd-` hướng dẫn · `tip` tooltip · `mo` mở · `nangcao` nâng cao · `nc-panel` panel nâng cao · `cum` cụm · `vt-` vị trí · `mui` mũi tên · `an` ẩn · `ghichu` ghi chú.
