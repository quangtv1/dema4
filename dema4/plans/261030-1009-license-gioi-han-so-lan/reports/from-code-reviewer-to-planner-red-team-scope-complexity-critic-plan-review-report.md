# Red-team review: Scope & Complexity Critic: license-gioi-han-so-lan

Reviewer: code-reviewer (hostile, YAGNI lens + Contract Verifier). Date: 2026-10-30.
Stated goal: (a) PR soft licensing, meaning 2 trial runs per function and then a "contact Sao Mai" popup; (b) stop users from deliberately tampering with the Google Sheet; (c) the repo stays public.
The plan itself accepts that anyone who rebuilds from source bypasses the license (plan.md:18).

## Contract verification (grep -rn, dema4_rs/src)

| Symbol the plan removes or changes | Current callers | Covered by plan? |
|---|---|---|
| `google_sheet::ghi_dong` | 2: `gui_tach/nut_tach.rs:190` (prod), `google_sheet.rs:182` (self-test) | Yes (phase 3) |
| `google_sheet::co_khoa` | 2: `gui_tach/nut_tach.rs:158`, `google_sheet.rs:111` | Implicitly |
| `mod google_sheet` / build key | `main.rs:16`, `build.rs:1-10`, `.github/workflows/build.yml:63-64` | Yes |
| `telegram::doc_cau_hinh` | 4: `gui.rs:524`, `gui.rs:851`, `main.rs:80`, `gui_tach/nut_tach.rs:157` | **No** for `main.rs:80` (CLI Đếm) and `gui.rs:524` (`co_telegram` init) |
| `telegram::gui_tin` | 3: `gui.rs:854`, `main.rs:82`, `gui_tach/nut_tach.rs:170` | **No** for `main.rs:82` |
| `telegram::gui_file` | 1: `gui_tach/nut_tach.rs:186` | Yes |
| `telegram::noi_dung_thong_ke` / `noi_dung_tach` / `dong_nguong` | `gui.rs:852`, `main.rs:81`; `nut_tach.rs:166`; `nut_tach.rs:168` | Partially (`main.rs:81` missed) |
| `telegram::SoLieuTach` (struct in `telegram.rs:150`) | 5: `nut_tach.rs:74,82,114,156,285` + test `telegram.rs:302` | **No**: the rename to `may.rs` "keeps only machine info", which leaves this struct without a home |
| `telegram::{ten_pc,dia_chi_mac,ip_noi_bo,ip_public}` | 4: `nut_tach.rs:175-178` | Yes (kept) |
| `use crate::telegram` / `mod telegram` | `gui.rs:14`, `gui_tach/nut_tach.rs:24`, `main.rs:22` | Implicitly by the rename |
| `co_telegram` field (derived from `doc_cau_hinh`) | `gui.rs:498,524,856,1238,1290,1872`; `nut_tach.rs:91,116` | **No** |
| Telegram unit tests | 8 tests at `telegram.rs:271-388` | Not mentioned (deleted or ported?) |
| `bat_dau_dem` | def `gui.rs:685`; callers `gui.rs:880`, `gui.rs:892`, test `gui.rs:1621` | Yes via the `UngDungChinh` harness |
| `bat_dau_tach` | def `nut_tach.rs:273`; callers `nut_tach.rs:270`, `nut_tach.rs:450`, test `nut_tach.rs:556` (on `ManTach::default()` at `:551`, **not** `UngDungChinh`) | **No** for the ManTach-only test |
| `chay_tach` | `main.rs:38`; tests `main.rs:182,183,196,197,218` (197 and 218 assert exit 0) | **No** test-harness plan |
| `chay_dong_lenh` (CLI Đếm) | `main.rs:43` | **No**: not named in any phase |
| `luu_va_thong_bao` / `nhan_ket_qua_tach` | `gui.rs:1347`, test `gui.rs:1600` / `gui_tach.rs:1015`, test `nut_tach.rs:536` | Production paths only |
| `tro_giup::logo`, `gioi_thieu::WEB` | `tro_giup.rs:18` (`pub(crate)`), `gioi_thieu.rs:73` | OK. There is also a duplicate constant `tro_giup::WEBSITE` at `tro_giup.rs:15` |
| `SoLieuDem` (plan says "đã có", i.e. "already exists") | **0 matches**: the struct does not exist | Plan claim is false |

## Finding 1: RSA signatures, nonce and the key ceremony protect against an attacker the plan already lets win
- **Severity:** High
- **Location:** plan.md:18 and :54; phase-02 lines 20, 28-31, 60, test 3; phase-03 lines 16, 26-30, tests 1-3; phase-05 steps 1-4; phase-06 matrix #10
- **Flaw:** A user who can forge station responses needs a local TLS-intercepting proxy with a trusted root CA. That user can more easily clone the public repo and make `quyet_dinh` return `Cho`, which the plan explicitly accepts (plan.md:18). Signing defends a door that sits next to an open wall. Ordinary users, the stated target, cannot MITM HTTPS at all.
- **Failure scenario:** The plan spends about 1 to 1.5 days on keygen, PKCS#8 in Script Properties, `include_str!` of the public key, nonce plumbing, Apps Script↔`rsa` compatibility (phase-03 risk at line 64), and a key-rotation runbook (phase-05:41). It adds a new failure mode: a signature mismatch silently drops every user to the offline rules, so paying customers get blocked after 2 runs. None of this reduces real bypasses.
- **Evidence:** plan.md:18 ("Không chặn được người tự build lại từ mã nguồn (chấp nhận)", meaning rebuilding from source is accepted as unblockable); phase-03-kho-tram-va-bo-khoa-sheet.md:16, :64-65; phase-02-tram-apps-script.md:20, :31
- **Suggested fix:** Drop RSA and the nonce for v0.6.0. Use plain HTTPS JSON from the station; TLS already authenticates the station. If forgery is ever observed in the field, add signing later. That is a backward-compatible app change.

## Finding 2: The Telegram queue, trigger, Cấu hình tab, JS message port and 429 logic are scope creep that forces an account migration
- **Severity:** High
- **Location:** plan.md:30, :32, :52, :55; phase-02 lines 16-18, 22, 35-36, tests 6-10; phase-05 steps 2, 5, 6, 9, 10; phase-06 #12-15
- **Flaw:** Neither goal mentions Telegram. The plan builds a durable queue in a Sheet tab, a 1-minute time trigger, retry counters, 429 `retry_after` handling, 7-day cleanup, and a per-machine message cap. It also reimplements three Rust message templates in JS plus the 21-column Ngưỡng trắng row (`thong_ke_nguong::COT`/`dong_sheet`) with string-equality tests. The trigger's runtime cost is the only reason the plan requires moving Monitor_dema4 to a Google Workspace owner (plan.md:32, phase-02:68). The "30 messages in 1 minute" acceptance criterion is invented and has no basis in current usage. Meanwhile `license_nhat_ky` already logs every run with machine, function, version and IP, which duplicates the Đếm Telegram stats.
- **Failure scenario:** About 1.5 days go to the queue, the port and the tests. The spreadsheet ownership transfer is an organisational blocker outside the developer's control and stalls the release. The JS templates and the Rust `thong_ke_nguong` drift apart, so the CSV or Sheet columns silently diverge.
- **Evidence:** dema4_rs/src/telegram.rs:110-211 (templates that would be ported); dema4_rs/src/tach_vb/thong_ke_nguong.rs:138, :177 (21-column row already built in Rust); phase-02-tram-apps-script.md:17, :68
- **Suggested fix:** MVP option 1: the app sends the already-built `dong_sheet` row and the CSV text. The station appends the row (type and length check only) and calls Telegram **synchronously** in `doPost` with the token from Script Properties. No queue, no trigger, no Cấu hình tab, no Workspace requirement. MVP option 2: drop Telegram for Đếm entirely, since `license_nhat_ky` covers it. Rotate the token either way. It is committed in the public repo at `telegram.rs:31`, so "exe has no token" (phase-03 test 7, matrix #14) is moot until rotation happens.

## Finding 3: The CLI Đếm path and the `--khong-ghi` loophole are missing, and the existing CLI tests will break or consume real quota
- **Severity:** High
- **Location:** phase-03 "Related Code Files"; phase-04 lines 19, 32, test 8
- **Flaw:** Phase 3 lists the Telegram call sites in `gui.rs` and `nut_tach.rs` only. The CLI Đếm path in `chay_dong_lenh` also calls `doc_cau_hinh` and `gui_tin`, and the plan never names it. Phase 4 defines a "run" as "≥ 1 file counted" but only tests `chay_tach`. `--khong-ghi` prints the full result and returns before any save or stats hook, so where `da_chay` would go is undefined, and the hook is likely skipped. Separately, `chay_tach` tests assert exit 0 and the plan gives no injection seam for the CLI. They will either call the real station or write the real `BoDemOffline` files in the developer's user directory and then start failing after 2 runs.
- **Failure scenario:** `sm-dema4 <thư mục> --khong-ghi` gives unlimited free counting on stdout. `cargo test --release` turns red on the third local run, or burns dev-machine quota on the live Sheet.
- **Evidence:** dema4_rs/src/main.rs:43, :65-68 (`khong_ghi` returns before saving), :80-82 (Telegram in CLI); dema4_rs/src/main.rs:197, :218 (`assert_eq!(chay_tach(..), 0)`)
- **Suggested fix:** Gate `chay_dong_lenh` before counting starts, regardless of `--khong-ghi`, and add a CLI Đếm test. Inject the license checker into `chay_tach`/`chay_dong_lenh` (parameter, or `cfg(test)` default to allow) and route `BoDemOffline` to a temp dir under test.

## Finding 4: The rename to `may.rs` orphans `SoLieuTach` and `co_telegram`, and the plan relies on a `SoLieuDem` that does not exist
- **Severity:** High
- **Location:** phase-03 lines 31-33, 39
- **Flaw:** Phase 3 says the payload uses "struct `SoLieuDem` / `SoLieuTach` / `ThongKeNguong` đã có" (already exist). `SoLieuDem` has zero matches; Đếm carries `KetQuaDem`. `SoLieuTach` is defined in `telegram.rs`, which the plan reduces to machine info only, so a stats DTO either ends up in a module named `may` or moves somewhere unspecified. Five call sites change. `co_telegram` is derived from `doc_cau_hinh().is_some()`, which the plan deletes, and the plan says the disclosure popup "giữ nguyên" (stays unchanged) without saying what the flag becomes. The plan also says nothing about the 8 Telegram unit tests.
- **Failure scenario:** The implementer invents `SoLieuDem` and its wire shape ad hoc, and that shape diverges from the JS validator. Or `co_telegram` is hard-coded to false and the public "stats are sent" disclosure silently disappears. That is a privacy-disclosure regression.
- **Evidence:** dema4_rs/src/telegram.rs:150 (`pub struct SoLieuTach`); grep `SoLieuDem` = 0 hits; dema4_rs/src/gui.rs:524, :856, :1290 (`co_telegram` drives the disclosure line); dema4_rs/src/gui_tach/nut_tach.rs:74, :82, :156, :285
- **Suggested fix:** Do not rename. Keep `telegram.rs` (or rename later in a separate chore), delete only `doc_cau_hinh`/`gui_tin`/`gui_file`/token, and leave `SoLieuTach` where it is. Set `co_telegram = true` whenever stats are sent to the station. Define the Đếm payload explicitly, listing the fields from `KetQuaDem` that are sent.

## Finding 5: Per-machine rate limiting and auto-registration do not meet the "stop tampering" goal; they only add work
- **Severity:** High
- **Location:** phase-02 lines 18, 23, 70, test 7
- **Flaw:** The rate limit is keyed on `ma_may`, which the caller declares itself and which is only regex-checked. An abuser rotates IDs, so the `CacheService` limiter (≤10 messages and ≤30 calls per 10 minutes) stops nothing. It hurts the victim instead: spamming `ghi_lan` with someone's visible machine ID (shown on the popup and in screenshots) burns their quota and then rate-limits their real calls, which pushes them onto the offline rules. `kiem_tra` auto-registers any well-formed ID, so a script can fill the `license` tab with junk rows. That is exactly the "phá Sheet" (vandalising the Sheet) the owner wants to stop.
- **Failure scenario:** A curious user reads the public `license.gs` and the URL and loops `kiem_tra` with random IDs. Thousands of rows appear, the owner's tab becomes unusable, and nothing in the plan bounds it.
- **Evidence:** phase-02-tram-apps-script.md:23 (validation is format only), :70 (the plan admits the spam vector and mitigates it per `ma_may`); dema4_rs/src/google_sheet.rs:15 (the same Sheet ID is public in the repo)
- **Suggested fix:** Drop the per-ID limiter (YAGNI; it is ineffective). Bound the damage with a cheap global guard instead: cap new registrations per hour globally, and stop writing once it is exceeded. Accept that ID-spoofing DoS exists and document it. Do not pretend a per-ID cache fixes it.

## Finding 6: The offline counter stored in two places is security theatre in a public repo
- **Severity:** Medium
- **Location:** phase-01 lines 18, 32-33, test 4; phase-06 #7
- **Flaw:** The second path is published in the open-source `license.rs`. Anyone motivated to delete one file deletes both. The only users it stops are those who would never find either file. It also writes into the real user directory, which pollutes the dev machine during tests (see Finding 3).
- **Failure scenario:** Extra code, tests and a matrix row for zero added deterrence. A "file hỏng" (corrupt-file) path is still treated as 0, so both files can be reset by corrupting them.
- **Evidence:** phase-01-loi-license.md:32 (`BoDemOffline { noi: [PathBuf; 2] }`); phase-01-loi-license.md:51 (corrupt file → 0)
- **Suggested fix:** One file under `du_lieu_an`. Keep the "max 2 offline" rule; it is a user decision and not challenged here.

## Finding 7: The `KhoLicense` trait, `license/kho.rs` split, `Arc<Mutex>` state and `Receiver` API add layers for one real implementation
- **Severity:** Medium
- **Location:** phase-03 lines 22-27, 37; phase-04 lines 23-28
- **Flaw:** There is one production store. The trait exists only so that `KhoGia` can be tested. Phase 3 also hedges on file layout ("`license/kho.rs` (hoặc gộp `license.rs`)", i.e. "or merge into license.rs"). Phase 4 adds `chay_tu_ban_sao`, piggybacked `bu_offline`, and a `Receiver<QuyetDinh>` channel API. It also adds tests 5b and 5c, which overlap with tests 2 and 4. That is three abstractions for a "2 tries then popup" feature.
- **Failure scenario:** Most of phase 4's 1.5 days goes to a state machine whose interleavings (in-flight `da_chay`, then `kiem_tra_lai`, then `lam_moi_ngam` racing over `chay_tu_ban_sao = 0`) have no test. Two concurrent responses reset the local delta and undercount, which is the opposite of what test 3 asserts.
- **Evidence:** phase-04-gan-license-gui-va-dong-lenh.md:26-27 (both `lam_moi_ngam` and `da_chay` set `chay_tu_ban_sao = 0` from background threads); phase-03-kho-tram-va-bo-khoa-sheet.md:37
- **Suggested fix:** Single `license.rs`. Make the station call a `fn(&YeuCau) -> Result<TrangThaiMay>` field (or `Box<dyn Fn>`), swapped in tests. Make `da_chay` write-through: a sequential background worker that owns the state. Drop 5b and 5c.

## Finding 8: Apps Script in-editor tests and the 17-case manual matrix are mostly phantom or duplicate verification
- **Severity:** Medium
- **Location:** phase-02 tests 1-11 and line 40; phase-06 matrix and line 52
- **Flaw:** The 11 Apps Script tests run by hand in the browser editor against a copied spreadsheet. They are not in CI and cannot be repeated after edits. `test_dong_thoi` makes 10 *sequential* calls and calls it a concurrency test. `test_chu_ky_hop_le` "verifies" with a hand-rolled check, which the plan admits is not the real check. In the matrix, #11 and #14 duplicate phase-3 tests 6 and 7; #15 and #16 are load tests for the Telegram queue and RSA path that Findings 1 and 2 cut; and rows 1-14 are repeated on both Mac and Windows.
- **Failure scenario:** About 1 day of manual ceremony, with green results that do not prove concurrency or compatibility.
- **Evidence:** phase-02-tram-apps-script.md:44, :47, :49; phase-06-kiem-thu-tong-va-phat-hanh.md:32, :35-37, :52
- **Suggested fix:** Keep the Rust tests with the fake store and one `#[ignore]` real-station smoke test. For the station, use one curl script covering register, `ghi_lan` and limit reached. Shrink the matrix to about 7 rows (new machine, limit reached, owner raises the limit, Khoá, offline 3x, CLI Đếm/Tách blocked, stats row lands). Run the full set on Windows only; it is the customer platform.

## Finding 9: No MVP cut, and the effort estimate contradicts the phase sum
- **Severity:** Medium
- **Location:** plan.md:46; phase front-matter `effort`
- **Flaw:** The phases sum to 0.5 + 1.5 + 1 + 1.5 + 0.5 + 1 = **6 days**, but plan.md says "~5 ngày" (about 5 days). Nothing is marked deferrable. The owner's PR goal (popup after 2 runs) could ship with: station `kiem_tra`/`ghi_lan` (no signing), the popup, one offline file, and the stats row moved to the station so the SA key can be revoked.
- **Failure scenario:** The release slips behind organisational steps (Workspace ownership transfer, trigger authorisation, token in a Sheet tab) that the PR goal does not need.
- **Evidence:** plan.md:46; phase-01-loi-license.md:7; phase-02-tram-apps-script.md:7; phase-03-kho-tram-va-bo-khoa-sheet.md:7; phase-04-gan-license-gui-va-dong-lenh.md:7; phase-05-trien-khai-tram.md:7; phase-06-kiem-thu-tong-va-phat-hanh.md:7
- **Suggested fix:** Add an explicit "MVP v0.6.0" vs "Deferred" section. The deferred list covers RSA/nonce, the Telegram queue/trigger/Cấu hình tab, the rate limiter, the dual offline file, the module rename, and the Apps Script test suite. Re-estimate to about 3 days.

## Finding 10: Storing the Telegram token in a Sheet tab undoes the "only Sao Mai sees secrets" posture
- **Severity:** Medium
- **Location:** plan.md:30; phase-02 line 18; phase-05 steps 5 and 10
- **Flaw:** The plan already keeps one secret in Script Properties (the RSA key). It then puts the bot token in a visible spreadsheet tab and relies on the sharing settings (phase-05:32 admits "ai xem Sheet cũng thấy", i.e. anyone who can view the Sheet sees it). That adds a tab, a reader, a missing-config test, and an operational rule, all to support "change group without redeploy". That convenience was not requested, and Script Properties already provides it.
- **Failure scenario:** Someone later shares Monitor_dema4 view-only with a unit (the Đơn vị column suggests this will happen) and the token leaks again.
- **Evidence:** phase-05-trien-khai-tram.md:26, :32; phase-02-tram-apps-script.md:18
- **Suggested fix:** Put the token and chat ID in Script Properties. Delete the Cấu hình tab, `test_doc_cau_hinh`, and matrix #13.

## Unresolved questions
- Is Telegram stats delivery a hard requirement for v0.6.0, or can `license_nhat_ky` replace the Đếm notifications?
- Does the owner accept migrating Monitor_dema4 to a Workspace account? The plan makes this mandatory only because of the trigger.
- Should `--khong-ghi` CLI runs count against quota?

Status: DONE_WITH_CONCERNS
Summary: The plan meets the PR goal but carries about 2-3 days of gold plating: RSA/nonce against an attacker the plan already accepts, a Telegram queue and trigger that force a Workspace migration, an ineffective per-ID rate limiter, a dual offline file, a module rename, and manual test ceremony. Contract checks found missed callers (CLI Đếm `main.rs:80-82`, `co_telegram`, `SoLieuTach`'s home, a ManTach-only test, CLI tests asserting exit 0), a non-existent `SoLieuDem`, and a `--khong-ghi` quota bypass.
