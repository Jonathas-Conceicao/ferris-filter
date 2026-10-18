---
phase: 40-release-pipeline-installer
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 3
files_reviewed: 19
files_reviewed_list:
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
  - .gitattributes
  - .gitignore
  - Cargo.toml
  - build.rs
  - installer/ferrisfilter.iss
  - installer/package.sh
  - installer/portable-readme.txt
  - installer/release-check.sh
  - installer/release-check.test.sh
  - installer/verify-exe.sh
  - src/overlay/app.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/sync/client.rs
  - tools/ff-art/Cargo.toml
  - tools/ff-art/src/main.rs
  - assets/poe2/manifest.toml
findings:
  critical: 0
  warning: 0
  info: 12
  total: 12
status: issues_found
---

# Phase 40: Code Review Report (iteration 3, final)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found (Info only)

## Summary

This re-review covers the iteration-2 fix commits `12e9771`, `4668e8f`, `7475055` and `0e6d8ca`,
against `149d030`. All four findings are fixed, and I found no regressions at Critical or Warning
level.

- **CR-01 (real CHANGELOG case):** fixed. The case now takes the newest `## [X.Y.Z] - ...`
  heading (CR-tolerant, pre-release headings skipped) and runs it with `FF_CARGO_VERSION`, so it
  never needs cargo and no longer depends on `[Unreleased]`. Today it resolves to `0.6.0`.
  `bash installer/release-check.test.sh` passes. The 40-07 caveat still holds: the
  `v0.7.0-rc.1` dry-run tag needs a non-empty `[Unreleased]`.
- **WR-01 (owner-process test):** fixed. I traced both tests. The first one really does go
  through `Some(process) => has_exited(process)`: the pid is overwritten with the child's pid
  before `-ready`, so `owner_process` does not short-circuit on `GetCurrentProcessId`. The kept
  `Child` handle stops the pid from being reused, and `OpenProcess` on an exited but still-held
  process returns a signalled handle. On CI, `PING.EXE` sits under `%SystemRoot%\System32` on
  `windows-latest`. Output is nulled, the names are per-process `Local\FerrisFilter-test-<pid>-<label>`,
  and `Sleeper::drop` kills the child on panic. I ran `cargo test --lib overlay::instance` three
  times in a row: 38/38 passed each time, in 0.70 s. The timing margins (300/600 ms StillRunning
  windows against a 60 s child, 3000 ms for Exited) can only fail if an assertion is wrong, not
  because the runner is slow. One weakness: the second test can pass without exercising its
  scenario (IN-11).
- **WR-02 (uninstall quit after confirm):** fixed as far as the code goes. `usUninstall` comes
  after the confirmation prompt and before the uninstall log and `[UninstallRun]` are processed.
  The Inno 6 `Abort` remarks list `CurUninstallStepChanged(usUninstall)` among the points where
  Abort ends the uninstaller. The script is still ASCII-only (0 bytes >= 0x80), and the
  `AppMutex` forbidden-word grep counts 0 (case-insensitive). It is uncompiled, so 40-07 still has
  to confirm the three behaviours the fix report lists.
- **WR-03 (deferred clean quit):** fixed. `quit_after_reload` is separate from
  `quit_after_apply`. `run_deferred_quit` checks `reload.is_running()`, which stays true until the
  later `reload.poll()` at `app.rs:3535` takes the result, so the deferral lags one frame and never
  fires early. It then calls `request_quit()` again, which re-checks `quit_dirty()`. Both popup
  outcomes (`OpenConfirm` and `KeepConfirm`) clear the flag, so a Cancel in the popup is not
  followed by a quit. I checked every path that writes `quit_after_reload` (lines 2507, 2510,
  2516, 2537, 2542), and none of them leaves it stuck at true. Apply & quit still shuts down
  without re-checking, on purpose (IN-12).

No new BLOCKER or WARNING. The nine iteration-2 Info items are still open, since none of the
fix commits touched those lines. They are listed again below, along with three new Info items.

## Info

### IN-01: `--quit` still creates `-ready`/`-quit` events, and the comment says it does not

**File:** `src/overlay/mod.rs:45-48`, `src/overlay/instance.rs:655-656`
**Issue:** Carried over. `open_event` calls `CreateEventW`. Yet the `mod.rs` comment says "It
only opens the instance objects, never creates them".
**Fix:** Use `OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, name)` on the quit path, or reword
the comment so it says only the mutex is never created.

### IN-02: `OpenMutexW` is given `PROCESS_SYNCHRONIZE`

**File:** `src/overlay/instance.rs:606`
**Issue:** Carried over. The value is the same, but the constant belongs to the wrong family.
**Fix:** Use `windows_sys::Win32::Foundation::SYNCHRONIZE` (or the `Storage::FileSystem` one).

### IN-03: The `Win32_System_Diagnostics_ToolHelp` feature is unused

**File:** `Cargo.toml:50`
**Issue:** Carried over. Nothing uses ToolHelp any more, since WR-04 removed the process scan.
**Fix:** Drop the feature and re-lock.

### IN-04: PID read and `OpenProcess` are not atomic; the fallback is silent

**File:** `src/overlay/instance.rs:418-427`
**Issue:** Carried over. A pid reused between `published_pid` and `OpenProcess` would make the
quit wait for an unrelated process. Separately, a failed `OpenProcess` falls back to the mutex
wait and leaves no trace that it did.
**Fix:** Re-read the pid after opening the handle and compare the two. Log when the fallback is
taken while a section exists.

### IN-05: The profile comment sits above `[build-dependencies]`

**File:** `Cargo.toml:57-63`
**Issue:** Carried over.
**Fix:** Move `[build-dependencies]` above the "Optimise dependencies ..." comment.

### IN-06: `LegalCopyright` holds a license identifier

**File:** `build.rs:85`
**Issue:** Carried over.
**Fix:** `"Copyright (C) 2026 Jonathas Conceicao. GPL-3.0-or-later."`

### IN-07: `verify-exe.sh` matches the version as a substring

**File:** `installer/verify-exe.sh:21-29`
**Issue:** Carried over. A check for `ProductVersion0.7.0` also passes on `0.7.0-rc.1`.
**Fix:** Anchor the version, for example `grep -a -E -q "ProductVersion${version//./\\.}([^0-9A-Za-z.+-]|$)"`.

### IN-08: The icon downscale uses straight alpha; frame sizes are not validated

**File:** `tools/ff-art/src/main.rs:584-612`
**Issue:** Carried over.
**Fix:** Resize premultiplied data, and add `anyhow::ensure!((1..=256).contains(&s), ...)`.

### IN-09: Silent install and uninstall abort when ff is still running

**File:** `installer/ferrisfilter.iss:97-98, 117-122`
**Issue:** Carried over, and it now covers the `usUninstall` path too. Under
`/SUPPRESSMSGBOXES`, the Retry box returns `IDCANCEL`, so a silent install or uninstall over a
running ff that has edits fails, with no retry. The script header does not say so.
**Fix:** Document it in the header comment (lines 14-19).

### IN-10: The frame-loop comment still describes the old deferral

**File:** `src/overlay/app.rs:3432-3434`
**Issue:** The comment says "Apply & quit succeeded, or a quit request arrived mid-reload: leave
once the in-game reload ... has finished". Since `4668e8f`, a deferred clean quit no longer just
leaves. It is asked again and can open the Unapplied changes popup. The doc comment on
`run_deferred_quit` is accurate, but this inline comment contradicts it.
**Fix:** "Apply & quit leaves once the in-game reload has finished; a quit request that came
mid-reload is asked again then (see `run_deferred_quit`)."

### IN-11: `an_instance_releasing_its_name_early_is_not_taken_as_its_exit` never checks that the name was released

**File:** `src/overlay/instance.rs` (test added in `0e6d8ca`, after `quit_follows_the_published_owner_process_not_the_name`)
**Issue:** The test asserts `StillRunning`, but `StillRunning` is also the result when the
instance thread never sees the quit request and never drops its guard. That would happen if
`take_quit_request` regressed, or if the request were withdrawn before the thread polled. In
that case the mutex is still held, and even the mutex fallback reports `StillRunning`. So the
test can pass without running the "early name release" scenario. Today the thread polls every
10 ms against a 600 ms window, so in practice the scenario does run. The problem is that nothing
asserts it.
**Fix:** Have the thread report that it dropped the guard (for example `released_tx.send(())`
after `drop(g)`). Then, before or right after the `quit_running_named` call, assert
`released_rx.recv_timeout(Duration::from_millis(600)).is_ok()`, and assert the mutex is gone,
for example via `quit_running_named` with a fresh call returning `NotRunning` once the child has
ended.

### IN-12: Apply & quit still shuts down without re-checking

**File:** `src/overlay/app.rs:2531-2546`
**Issue:** The fixer chose this on purpose. `quit_after_apply` calls `begin_shutdown()` as soon
as the reload worker is idle. If the user summons the window during that sub-second reload
(the game took focus, so the window was hidden) and edits something, the edit is lost without
the popup. The user has just chosen to quit, so the risk is small. The opposite case goes the
other way: if the user keeps working during a deferred *clean* quit and then presses Apply, the
`request_quit` that was asked again quits the overlay as soon as that Apply's reload finishes,
which they might not expect after they kept working. Both behaviours are defensible, but neither
is documented in CLAUDE.md's quit notes.
**Fix:** Optionally run the same `quit_dirty()` re-check for `quit_after_apply`. It costs nothing
when nothing changed. Consider also clearing `quit_after_reload` whenever the user starts an
Apply from the bottom bar, so new work cancels a quit that was still waiting.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
