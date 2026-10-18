---
phase: 41-check-for-updates
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 3
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
  critical: 0
  warning: 0
  info: 10
  total: 10
status: issues_found
---

# Phase 41: Code Review Report (iteration 3)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found (Info only)

## Narrative Findings (AI reviewer)

## Summary

This pass re-reviewed the Phase 41 diff (`ef84189..HEAD`) and focused on the two iteration-2 fix
commits, `0819e88` (WR-01) and `44859c3` (WR-02). I re-ran the update, launch and sync-ranking
unit tests and they pass (55 + 14 filtered).

- **WR-01 (sync outcome hidden by a late update message): fixed.**
  - `receive_update_message` now ranks the worker's message behind a `Running` sync as well as a
    `Finished` one.
  - `poll_economy` is now the only caller of `economy.poll()` (`app.rs:2259`; the frame at
    `app.rs:3540`). It sets `update_behind` whenever a sync outcome lands while an update message
    is showing.
  - I traced these orders:
    - check → sync → message → sync done
    - sync → check → message → sync done
    - sync → check → sync done → message
    - message → sync → Ready arrives during the sync
    - a failed Install click while a sync runs
  
  In each one, the sync outcome or the newest UI-thread message is shown, and no stale
  `update_behind` value survives. Every setter of `update_message = Some(..)` also sets
  `update_behind`. The paths that set it to `None` leave a stale flag behind, but nothing reads
  the flag while the message is `None`.
- **WR-02 (launch-time open: retry and error mapping): fixed.** `open_with_retry` retries raw OS
  error 32 three times. `NotFound`, and a path that exists but is not a file (a folder, or a
  delete-pending file whose metadata fails), map to `Gone`. Any other open error, and a read
  error during the hash, become the new `LaunchError::Open` with its cause. Its only consumer is
  `launch_error_text`, which matches every variant. `mod.rs:130-135` passes it to the error box
  unchanged. The retry costs at most 750 ms, and it runs after the window has closed, so nothing
  freezes in front of the user.

Neither fix introduced a new Critical or Warning issue. The remaining items are Info: eight are
carried from iteration 2, and two are minor new ones (IN-09, IN-10).

## Info

### IN-01: An oversized or unreadable API reply is reported as "Could not reach GitHub" (carried)

**File:** `src/update.rs` (`classify_latest`; `check_for_update_blocking`, line 942-944)
**Issue:** `FetchError::TooLarge` / `Sink`, and a failure to build the HTTP client or the
runtime, all map to `Failure::Offline`.
**Fix:** Map these to `BadResponse` or to a dedicated failure.

### IN-02: The trusted-URL check compares against the raw tag (carried)

**File:** `src/update.rs` (trusted asset URL check)
**Issue:** A tag with characters that GitHub percent-encodes (such as `+`) would be refused with
a generic "could not read" message. This is harmless for `vX.Y.Z` tags.
**Fix:** Refuse such tags up front with a clear message, or encode the tag before comparing.

### IN-03: The checksum shows the file is not corrupted; it does not prove who published it (carried)

**File:** `src/update.rs:23` (`SUMS_NAME`), `run_check`
**Issue:** `SHA256SUMS.txt` comes from the same release as the installer, so anyone who can
upload one can upload both.
**Fix:** Document this limit, or sign the sums file later.

### IN-04: An exit that skips `begin_shutdown` leaves the downloaded installer behind (carried)

**File:** `src/overlay/mod.rs:126-137`
**Issue:** A logoff, an outside `WM_CLOSE` or an eframe error skips `update.shutdown()`. The
folder then stays until the 24 h sweep.
**Fix:** Add a best-effort cleanup in `OverlayApp`'s `on_exit` / `Drop` when no hand-off was
recorded.

### IN-05: ShellExecuteW without COM / SEE_MASK_NOASYNC, and the Install click hashes on the UI thread (carried)

**File:** `src/overlay/win.rs:669-694`, `src/overlay/update_job.rs:222`
**Issue:**
- The elevation fallback (`ShellExecuteW`) runs without COM initialized and without
  `SEE_MASK_NOASYNC`, right before the process exits. It also releases the launch lock as soon as
  it returns. The fallback is unreachable in practice: the installer is
  `PrivilegesRequired=lowest`, so error 740 should not occur.
- `begin_install` hashes up to 200 MiB synchronously in the frame.

**Fix:** Use `ShellExecuteExW` with `SEE_MASK_NOASYNC` (and `CoInitializeEx`), or drop the
fallback. Optionally, hash on the worker at click time.

### IN-06: `is_dev_build` cannot trigger in production, and the update message can name a different version than About (carried)

**File:** `src/update.rs:55-57`, `src/update.rs:947`, `src/overlay/about_view.rs:45`
**Issue:**
- `compare_release` gets `env!("CARGO_PKG_VERSION")`, which never carries `-dev`. Only
  `FF_VERSION` (from `git describe`) does, so the `is_dev_build` branch and its tests cover a
  string the shipped code never sees.
- "Up to date (…)" names `CARGO_PKG_VERSION`, while About shows `FF_VERSION`. The two can differ
  on dev builds.

**Fix:** Pass the version the rule is meant for, or drop `is_dev_build` and reword the comment.
Name the same version in both places.

### IN-07: A late "failed" or "ready" update message that waits behind another event is lost on the next edit or hide (carried)

**File:** `src/overlay/app.rs:2244-2253`, `src/overlay/app.rs:2282-2287` (`clear_events_on_edit`), `src/overlay/app.rs:2213-2222` (`clear_events_on_conceal`)
**Issue:**
- A behind-ranked update message only shows once the event above it is gone. But the edit or
  hide that clears that event also clears `update_message`.
- The iteration-2 fix makes this more common: a message that arrives during a running sync is
  now always behind, and the next hide clears both.
- Losing a Ready message is harmless, because the About button still says Install.
- A `Failed` message (offline, checksum mismatch) is lost silently, and the button just goes
  back to Idle.

**Fix:** Keep a behind-ranked update message across the next clear. Alternatively, show the last
failure on the About button or its tooltip. A per-message sequence number, with `event_from`
picking the newest, would also replace the `update_behind` bookkeeping.

### IN-08: A failed launch-time check leaves the installer folder in `%TEMP%` (carried)

**File:** `src/overlay/mod.rs:130-135`, `src/update.rs:278-293`
**Issue:**
- After a `Gone`, `Mismatch`, `Open` or `Spawn` error at launch, the work folder is not
  deleted.
- For `Open` and `Spawn` this is intended, since the text names the saved file.
- A `Mismatch` leaves an unverified exe in `%TEMP%` until the 24 h sweep.

**Fix:** On `LaunchError::Check(Mismatch)`, delete the parent folder, but only if its name starts
with `WORK_PREFIX`.

### IN-09: The `Open` error text nests the OS message's own parentheses

**File:** `src/update.rs:339-343`
**Issue:** On Windows, `io::Error::to_string()` already ends in `(os error 32)`. The box therefore
reads "…to check it (The process cannot access the file because it is being used by another
process. (os error 32)), so it was not started." That is a nested parenthesis with a full stop
inside it. It is the same style as the `Spawn` text, but less readable.
**Fix:** Put the cause after a colon, as `Spawn` does:
`"Could not open the FerrisFilter {version} installer to check it: {cause}. It was not started and was saved to {}."`

### IN-10: The WR-01 regression test simulates the Sync click instead of driving it

**File:** `src/overlay/app.rs:6408-6454` (`a_sync_started_after_a_check_shows_its_outcome_over_the_check_message`)
**Issue:**
- The test sets `app.update_message = None` and calls `app.economy.start(..)` directly. It does
  not go through the real `economy_clicks.sync` branch (`app.rs:4062-4070`).
- If that branch stopped clearing the update line, or ever started the sync in another way, the
  test would still pass.
- The order behaviour itself is covered correctly.

**Fix:** Optional. Pull the click body into a small `start_sync(league, root)` method and have
both the frame and the test call it.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
