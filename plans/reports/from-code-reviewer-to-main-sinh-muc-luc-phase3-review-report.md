# Review: Sinh mục lục (f2d2860..194ecf4)

Scope: src/gui_muc_luc*, src/muc_luc/{khop,sinh,mo_phong}.rs, license.rs, tram.rs, cau_hinh.rs, hop_cau_hinh.rs, gui.rs, apps-script/{loi.js,license.gs}.
Checks run: `cargo test --release` gave 203 passed, 0 failed, 27 ignored. Caveat: the working tree has uncommitted edits in src/gui.rs and src/tro_giup.rs, so the run used the working tree, not exactly 194ecf4. `node --test apps-script/test/loi.test.js` gave 18/18 passed. TIEU_DE.MAY and GHI_CHU.MAY both have 18 entries.
Secrets / personal data: none found in the diff. The fixture `tests_data/muc_luc/bien_muc_tong_hop.xlsx` contains only made-up data.

## High

**H1. If the app ships before the station is redeployed, Đếm/Tách get blocked too.** Locations: `apps-script/loi.js:123` (old station), `src/license.rs:393-398`.
- The old station validates `TEN_CN[l.cn]`. For `cn:"muc_luc"` it returns `{ok:false, loi:'lan'}` for the whole `ghi_lan` batch.
- `dong_bo_trong` hits `Err(_) => return` on the first failing chunk. The muc_luc run stays at the head of the queue and is re-sent on every sync.
- Later chunks with Đếm/Tách runs are never sent. `kiem_tra` is never called either, because `cho` is not empty.
- After 2 unconfirmed Đếm runs, `quyet_dinh` returns `HetLuotOffline(Dem)`. One Sinh mục lục run on a new client can therefore lock that machine out of every function.
- Fix:
  - Make "redeploy station first" a hard release gate.
  - Also harden the client: on a non-network station error (`loi == "lan"`), skip that chunk or send the runs one at a time, instead of returning.
  - Optionally, have the client send muc_luc runs only when the last signed reply contains `han_muc_luc`, i.e. the station is new.

## Medium

**M1. Closing the window during a generation run asks nothing and still charges the run.** Location: `src/gui.rs:262-268`.
- The close handler only checks `self.tach.dang_chay()`, so `muc_luc.dang_chay()` is ignored and the app exits immediately.
- `ket_thuc` is never called, and a leftover reservation is still counted (test `dat_cho_va_hoan`). The user is charged even if 0 DOCX were written, and gets half the files.
- Fix: use `self.dang_chay()` or add `|| self.muc_luc.dang_chay()`. The `hop_hoi_dong_chay` text (`viec`) should also cover "sinh mục lục".

**M2. In the Ghép biến dialog, pressing Esc in the inline editor throws away every edit.** Location: `src/gui_muc_luc/hop_ghep.rs:159` (with `bang_bien.rs:136`).
- `khung_sua` turns Esc into `Viec::DongSua`, but in the same frame `Modal::should_close()` is also true on Esc.
- That takes the `huy` branch: `self.rb = goc` and the dialog closes. The user only meant to close the dropdown and loses all bindings made since the dialog opened. Clicking the backdrop reverts silently too.
- Fix: when `self.mo_hop.is_some()`, ignore `should_close()` (Esc only closes the editor). Optionally treat a backdrop click as "keep" or ask, instead of reverting silently.

**M3. A binding saved for template A can be restored onto template B.** Locations: `src/gui_muc_luc.rs:182-191`, `:246`.
- `ghi_vao` always writes `c.mau = mau_chon`, but writes `c.bien` only when both `mau` and `du_lieu` are loaded.
- Scenario:
  1. The data file is unavailable at startup (Drive not mounted, or a read error).
  2. The user switches to template 2. The config now holds `ml_mau=2` with template 1's `ml_bien`.
  3. On the next start, `bien_luu.0 == mau_chon` passes and template 1's JSON is applied to template 2.
- Variables that exist only in template 2 are not auto-matched (only `TuDong` is merged back), so they show as "chưa ghép". Template 1's binding is lost as well.
- Fix: in `ghi_vao`, when the binding can't be written and `c.mau != self.mau_chon`, clear `c.bien`. Better: store bindings keyed by template.

**M4. A restored binding blocks auto-matching for new or renamed columns and variables.** Location: `src/gui_muc_luc.rs:247-262`.
- `retain` drops variables whose column no longer exists, then only `TuDong` variables are merged back from `tu_khop`.
- Scenario: the same custom template path is edited to add `{ngay_thang}`, or a new data file spells a column differently but normalises to the same name (e.g. "So ky hieu"). Those variables stay unbound even though `tu_khop` would match them.
- Fix: for variables that are absent from the restored binding, take the `tu_khop` result. To keep an explicit "Bỏ ghép", save it as a marker in the JSON (e.g. `{"bo":true}`) so it isn't re-matched.

**M5. A panic in the generation thread refunds the run even after files were written.** Location: `src/gui_muc_luc/chay.rs:78-80`.
- On `Disconnected`, the code builds a `TongKet` with `docx: 0`, so `ket_thuc(&lan, false)` runs.
- If `ghep` panics on hồ sơ k+1, files 1..k are already on disk but the run is not counted. This is a license leak, reachable whenever there is any panic path in `ghep` or `mo_phong` for crafted data.
- Fix: count with a shared `Arc<AtomicUsize>` incremented after each successful write, and use it on `Disconnected`. Or wrap each hồ sơ in `catch_unwind` inside `sinh::chay` and record it as an error.

## Low

- **L1** (`chay.rs:23`, `chay.rs:89`, `gui_muc_luc.rs:122/145`): the output folder name, the Telegram `du_lieu` field and the popup titles use the `self.file` text box, not the file actually loaded. Editing the path without pressing Enter, then clicking Sinh, puts the output in a folder named after the typed file (and reports that path) while the data comes from the old file. `ghi_vao` likewise saves `ml_file` and `ml_dong_tieu_de` from text that was never loaded. Fix: keep the loaded path and header row in `DuLieu` and use those.
- **L2** (`gui_muc_luc.rs:184`): `ghi_vao` writes `c.sheet = ""` whenever `du_lieu` is None. Any `cfg_doi` while loading (e.g. the sidebar-width write on the first frame) or after a read failure saves an empty sheet. If the app closes before the read finishes, or the read fails, the remembered sheet is lost. Fix: write `sheet` only when `du_lieu` is Some, as is already done for `bien`.
- **L3** (`gui_muc_luc.rs:759`): `ghep(...).and_then(mo_phong).unwrap_or_default()` swallows the error, so the Kết quả tab shows a blank page with no reason. Show the error string instead.
- **L4** (`license.gs:110`): `damBaoCotMucLuc` runs on every `doPost` inside the script lock, adding 2 Sheets calls (`getMaxColumns` and `getRange(1,17).getValue`) to every request. Fix: cache an "ok" flag in `CacheService` / `PropertiesService`. Otherwise the code is correct:
  - `copyFormatToRange(sheet, column, columnEnd, row, rowEnd)` arguments are right.
  - `setNotes([[a,b]])` has the right shape.
  - Old 16-column rows: `hanMuc('')` gives 10 and `soKhongAm('')` gives 0.
  - Column Q is written only when its value changes.
  - Column P (`MA_LAN`) stays in place.
- **L5** (`gui_muc_luc.rs:277-296`): `canh_bao_hs` costs O(rows × variables × columns), because `bang.o()` does a linear `rposition` header lookup per cell. It runs in the UI thread on every binding change, which gets slow on large files (10k+ rows). Fix: resolve column indexes once before the loop.
- **L6** (`bang_bien.rs:132`): the autofocus tracker remembers the last variable opened, so reopening the same variable doesn't focus the input again. The comment says "1 lần cho mỗi biến", which differs from what users would expect. Reset the tracker on `DongSua`.
- **L7** (`hop_cau_hinh.rs:106-107`): the doc comment "Đưa các mục Đếm trang A4 về mặc định." is now attached to `mac_dinh_muc_luc`, and `mac_dinh_dem` has lost its doc. Also, `mo_phong.rs:114` says it trims head and tail but only trims the tail.

## Verified OK (no action)

- **License call sequencing:** `ket_thuc` runs exactly once per `bat_dau`. That covers the early return in `bat_dau_sinh`, the single `sinh.take()` in `nhan_sinh`, and `DuocChay` going back through `co_the_sinh`. A run counts only when `docx > 0`. `thong_ke` is queued after the `DongBo` from `ket_thuc` on the same worker, so the reported count includes this run.
- **Background Excel read:** a new `doc_du_lieu` replaces the receiver and the old thread's send is dropped, so the latest read wins.
- **Hồ sơ index `hs`:** clamped in `kiem_tra_lai` and in the Kết quả tab. `canh_bao_hs` and `kq.ho_so` are rebuilt together, so their indexes always match.
- **Cached preview `trang_kq`:** keyed by `(hs, phien_rb)`, and `phien_rb` is bumped on every template, data or binding change.
- **Inputs during a run:** the "Đang sinh" modal blocks sidebar and page interaction. The generation thread works on `Arc` snapshots.
- **Station protocol and compatibility:**
  - Old apps receive extra fields and ignore them. New rows have 18 columns.
  - `thong_ke_*` dispatch only happens after the whitelist check.
  - The client `so_hoac` defaults to 10/0.
- **Config round-trip:** the `ml_*` keys handle `=` in values, the width clamp and the header-row bounds. `RangBuoc::json` is single-line.
- **File names:** `ten_file::sach` strips separators and `..`.
- **Slicing:** byte slicing in `trang::dan` is safe because token offsets fall on ASCII boundaries.

## Unresolved questions

1. Does Google Sheets hide new columns when they are inserted right after hidden column P (`insertColumnsAfter(16, …)`)? This only matters for sheets that had exactly 16 columns. Check once on a copy of the real sheet.
2. Release order: is the station guaranteed to be redeployed before any client with `ChucNang::MucLuc` ships? H1 depends on this.

Status: DONE_WITH_CONCERNS
Summary: Core licensing, threading and station code are sound and all tests pass. One High risk: shipping the app before the station is redeployed locks clients out of Đếm/Tách. Five Medium issues: no prompt when closing during a run, Esc in the Ghép biến dialog discarding edits, bindings restored onto the wrong template, restored bindings blocking auto-matching, and a refund after a thread panic.
Concerns/Blockers: H1 needs either an enforced release order or a client-side fix before release.
