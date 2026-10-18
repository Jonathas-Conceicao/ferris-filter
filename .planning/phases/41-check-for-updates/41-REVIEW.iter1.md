---
phase: 41-check-for-updates
reviewed: 2026-10-18T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - Cargo.toml
  - src/lib.rs
  - src/update.rs
  - src/overlay/update_job.rs
  - src/overlay/about_view.rs
  - src/overlay/app.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/overlay/win.rs
findings:
  critical: 1
  warning: 4
  info: 5
  total: 10
status: issues_found
---

# Phase 41: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the Phase 41 diff (`ef84189..HEAD`): the pure update logic and HTTP seam
(`src/update.rs`), the button and worker model (`src/overlay/update_job.rs`), the About button,
the status-line hookup, the quit hand-off in `app.rs`, the spawn after the event loop in
`mod.rs`, and the ShellExecute / error-box helpers.

The pure code is solid. The version rule, the checksum parser, the URL allow-list, size caps,
cleanup on every failure in `run_check`, and the worker-panic recovery all hold up when traced.

The problems are in how the module is wired into the app:
- One path through the Quit popup leaves the install intent set, so a later, unrelated quit
  starts the installer.
- The "the update message is always newest" rule is broken by the asynchronous worker.
- The installer is not hashed again right before it is started.
- RC builds are never offered their own final release.

## Critical Issues

### CR-01: "Apply & quit" that stays open (unwritable tier edits) keeps the install intent, so a later quit starts the installer

**File:** `src/overlay/app.rs:2466-2489`
**Issue:** `install_and_quit` sets `install_on_quit` and opens the Unapplied changes popup. Picking **Apply & quit** gives `RunApply`. When `ok && self.has_unwritable_tier_edits()` (Fallback prices plus tier edits), the code calls `show_not_quit_warning` and the window stays open. That branch does **not** call `self.update.cancel_install()`. Only the sibling `else` branch (a refused Apply) does. The same happens when only unwritable tier edits are pending (`ok = !self.applicable_dirty()` is true without any Apply). From then on the intent stays armed while the overlay keeps running. The next tray Quit, `ff --quit`, or Quit without applying from a fresh popup goes through `begin_shutdown`. There `self.update.shutdown()` returns the installer, and `launch()` starts the setup wizard. The user never asked to install on that quit. This is exactly what the comment on the `else` branch and the module contract ("handed to the spawn only when the player clicked Install and the process is then shutting down") rule out. No test covers this branch: `install_click_then_refused_apply_clears_the_intent` only covers the refused-Apply branch.
**Fix:** Drop the intent whenever the RunApply step does not end in a quit:
```rust
if ok && self.has_unwritable_tier_edits() {
    show_not_quit_warning(&mut self.apply_message, self.reload_pending.as_mut());
    // The window stays open: no install on a later, unrelated quit.
    self.update.cancel_install();
} else if apply::quit_step(dirty, false, apply::QuitEvent::ApplyFinished { ok })
    == apply::QuitStep::QuitWhenIdle
{
    self.quit_after_apply = true;
} else {
    self.update.cancel_install();
}
```
A sturdier option: clear the intent at the top of every `Popup::Quit` dispatch and set it again only on the `QuitNow` / `QuitWhenIdle` outcomes. Add a test that sets Fallback with tier edits, clicks Install, picks Apply & quit, then calls `begin_shutdown()` and checks that `handed_off(&app)` is None.

## Warnings

### WR-01: A late worker message masks newer Apply / Sync outcomes, despite the "update is always newest" rule

**File:** `src/overlay/shell.rs:250-255`, `src/overlay/app.rs:3499-3502`
**Issue:** `event_from` gives `update_message` top priority. The stated reason is that "the app clears it whenever another event action starts, so when it is present it is the newest." That holds only for messages set on the UI thread. `self.update.poll()` sets `update_message` again on every frame where the worker sends `Downloading` or `Done`, and the worker can send long after the user started another action. Example: press Check, then press Apply or Sync economy while it runs (both clear `update_message`). The Apply refusal ("file changed on disk", grammar gate), the Apply outcome (`Wrote …` / `Reloaded in game`), or the sync result is then silently replaced by `Downloading 0.8.0…` or `Up to date (…)`. This can hide an Apply error that the user needs to act on. It also hides the sync outcome, because the update message wins even over a sync event that is still current.
**Fix:** Either give the update message normal precedence (below `apply_message` and the sync outcome), or record an event sequence number. Increment a counter whenever any event message is set, store it with each message, and let `event_from` pick the highest. At minimum, do not let `poll` replace an `apply_message` that is an Error.

### WR-02: The installer is not hashed again right before it is started (verify-then-spawn gap)

**File:** `src/overlay/update_job.rs:218-238`, `src/overlay/app.rs:2673-2677`, `src/overlay/mod.rs:128-137`
**Issue:** The SHA-256 is checked again only at the Install click (`begin_install`). The spawn happens later, after the Unapplied changes popup has been answered (which can take minutes), after an Apply & quit, an in-game reload wait, and the event loop teardown. `pending_install` carries only the path, not the digest, so `launch()` cannot check it again. Anything that changes the file in the meantime (an AV quarantine or repair, another process writing into `%TEMP%`) is executed unverified. That breaks the phase's "SHA-256 verified installer" guarantee, and the CLAUDE.md note ("The click re-checks the installer").
**Fix:** Hand off `(PathBuf, String)` (path plus expected hex), and call `verify_installer` in `launch()` right before `spawn_installer`. Show `notify_error` on a mismatch or a missing file. To close the window fully, open the file without `FILE_SHARE_WRITE` / `FILE_SHARE_DELETE` (via `OpenOptionsExt::share_mode`), hash through that handle, and keep it open until the child process exists.

### WR-03: Release-candidate builds are told they are up to date when their final release is out

**File:** `src/update.rs:40-43`, `src/update.rs:56-74`
**Issue:** `current_base` drops the pre-release part, so `CARGO_PKG_VERSION = "0.7.0-rc.1"` (the current `Cargo.toml` version) compares as `0.7.0`. When `v0.7.0` final is published, `compare_release("v0.7.0", …, "0.7.0-rc.1")` returns `Same`, and an RC tester sees "Up to date (0.7.0, released …)". They are never offered the final installer, and the message names a version they are not running. Semver orders `0.7.0-rc.1 < 0.7.0`. The decision ("a dev build compares on its base version") is aimed at `-dev` builds, but `-dev` appears only in `FF_VERSION`, never in `CARGO_PKG_VERSION`. So the base-only rule actually only changes the outcome for RC builds.
**Fix:** Compare the full semver of the running build against the stable tag. A running pre-release with the same major.minor.patch counts as older:
```rust
let cur = Version::parse(current).map_err(|_| ())?;
let cur_core = Version::new(cur.major, cur.minor, cur.patch);
match tag_base.cmp(&cur_core) {
    Greater => Newer(tag_base),
    Equal if !cur.pre.is_empty() => Newer(tag_base),
    Equal => Same,
    Less => Older,
}
```
Add a `c("v0.7.0", false, false, "0.7.0-rc.1") == Newer(0.7.0)` row. Report `cur` (not `base`) in the up-to-date message.

### WR-04: Quit while Downloading can leak the work folder, and delete it while the worker is still writing

**File:** `src/update.rs:572-578`, `src/overlay/update_job.rs:252-269`
**Issue:**
- The work folder is created (`tempdir_in`) before it is published to the slot (`set_slot`). A shutdown that runs between those two calls finds the slot empty, deletes nothing, and the process exits before the `TempDir` drop runs. The folder (and later a partial installer) is left in `%TEMP%` until a check made more than 24 h later.
- `abandon()` calls `remove_dir_all` on the folder while the worker thread still holds the file open and writes to it. This relies on std's default `FILE_SHARE_DELETE`, which the research note marks as an assumption. The test (`shutdown_while_downloading_deletes_the_partial_folder`) does not hold an open handle, so the assumption is not checked.

**Fix:**
- Create the folder from a path chosen up front. Publish the path to the slot first, then create the folder, so shutdown can always find it.
- Have the worker check a shared `cancelled: AtomicBool` in the download sink and stop by returning an error, so the `TempDir` drop deletes the folder itself.
- Add a test that keeps a `File` open inside the folder while `shutdown()` runs.

## Info

### IN-01: An oversized or unreadable API reply is reported as "Could not reach GitHub"

**File:** `src/update.rs:303-306`, `src/update.rs:743-745`
**Issue:** `classify_latest` maps every `Err` (including `FetchError::TooLarge` and `Sink`) to `Offline`. `check_for_update_blocking` also maps a failure to build the HTTP client or the Tokio runtime to `Offline`. Both give a misleading "check your connection" message.
**Fix:** Map `TooLarge` to `BadResponse`, and a client or runtime build failure to `BadResponse`, or to a dedicated failure.

### IN-02: The trusted-URL check compares against the raw, un-encoded tag

**File:** `src/update.rs:253-256`
**Issue:** GitHub percent-encodes the tag in `browser_download_url` (for example `+` becomes `%2B` in `v0.8.0+build`). Such a release is then refused with "Could not read the latest version from GitHub", not a precise message. This is harmless for the current `vX.Y.Z` tags.
**Fix:** Refuse tags with build metadata or characters that are not URL-safe up front, with a clear failure, or percent-encode the tag before comparing.

### IN-03: The checksum protects against corruption only, not against a compromised release

**File:** `src/update.rs:542-570`
**Issue:** `SHA256SUMS.txt` comes from the same release as the installer, so anyone who can upload one asset can upload both. That is fine for corrupted downloads, but the CLAUDE.md / UI wording ("verified") could be read as authenticity.
**Fix:** Optional: document the limit, or later sign the sums file (minisign or Authenticode check on the installer).

### IN-04: An exit that skips `begin_shutdown` leaves the downloaded installer behind

**File:** `src/overlay/mod.rs:126-137`
**Issue:** `update.shutdown()` runs only inside `begin_shutdown`. If the event loop ends another way (a session end or logoff, a WM_CLOSE from outside, an eframe error), the Ready folder is neither deleted nor launched, and it stays until a check made more than 24 h later. The overlay is also not told whether an error from `run_native` happened after an install hand-off. The spawn still goes ahead, which is probably right but is not documented.
**Fix:** Also call a best-effort cleanup from `OverlayApp`'s `on_exit` / `Drop` when no hand-off was recorded.

### IN-05: ShellExecuteW is called without COM initialized, and the click hashes the file on the UI thread

**File:** `src/overlay/win.rs:668-694`, `src/overlay/update_job.rs:222`
**Issue:**
- Microsoft's docs ask callers to initialize COM (`CoInitializeEx`, apartment-threaded) before `ShellExecute`. It usually works for a plain `.exe`, but that is not guaranteed after winit has torn down.
- `begin_install` hashes up to 200 MiB synchronously in the frame, which briefly freezes the always-on-top window over the game.

**Fix:**
- Call `CoInitializeEx(null, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)` before `ShellExecuteW` (ignore `RPC_E_CHANGED_MODE`), or use `ShellExecuteExW` with `SEE_MASK_NOASYNC`.
- Optionally hash on the worker before the hand-off.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
