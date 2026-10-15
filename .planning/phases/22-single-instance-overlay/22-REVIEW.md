---
phase: 22-single-instance-overlay
reviewed: 2026-10-13T00:00:00Z
depth: standard
iteration: 3
files_reviewed: 4
files_reviewed_list:
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/overlay/app.rs
  - Cargo.toml
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 22: Code Review Report (iteration 3)

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This pass re-reviewed the phase-22 diff (`git diff 5333605`), with most of the attention on the iteration-2 changes:
- `OverlayApp.instance: Option<Guard>`, which tray Quit drops early
- the `launch_named` re-acquire after a failed signal
- the `request_until_acked` pre-send deadline check
- the relaxed `>= 1` test asserts

The mutex is still the only arbiter, and I found no path that lets two processes hold the name at once. Every handle in `acquire_named`, `signal_running` and `Guard::drop` is closed exactly once, with null checks. No handle leaks on any early return. `Guard` holds raw `HANDLE`s, so it is `!Send`, and it is only used on the main thread. Every `unsafe` block carries a correct `// SAFETY:` comment. The tests use per-PID `Local\FerrisFilter-test-*` names, and all 17 labels are distinct. They call `launch_named` and `signal_running`, never `acquire()` or `notify()`, so no MessageBox can appear. `cargo test --lib overlay::instance` passes 28/28.

There is one new Warning. The early release on Quit gives up the instance name **before** the closing process gives up its global hotkey. A relaunch in that window becomes the new first instance, but it cannot register `Ctrl+Shift+Z`, and it keeps that failure for its whole lifetime. That breaks the CONTEXT requirement of "exactly one stripe and one working hotkey".

I did not re-raise the accepted residuals in 22-REVIEW-FIX.md: events-failed concurrency, the outcome-2 startup delay, the IN-02 grace slice, and IN-03 `mark_ready` without a window. They still hold as described.

## Warnings

### WR-01: Quit releases the instance name before the hotkey, so an immediate relaunch runs with a dead `Ctrl+Shift+Z`

**File:** `src/overlay/app.rs:1300-1305` (with `src/overlay/mod.rs:41-51` and `src/overlay/hotkey.rs:49-62`)

**Issue:** The tray Quit arm sets `self.instance = None`, which releases the mutex at once. The process keeps running after that:
- It finishes the frame (the body render) and only then sends `ViewportCommand::Close` (line 1741).
- eframe handles the close on a later loop iteration, and the process exits after that.

For that whole time the old process still owns:
- `self.hotkey_manager`, which keeps `Ctrl+Shift+Z` registered through `RegisterHotKey` against its hidden hwnd
- the tray icon
- the visible stripe

A relaunch in this window goes through `acquire_named` and gets `Acquired::First`. `launch()` then calls `hotkey::register_toggle()` straight away, before eframe starts. The combo is still registered by the old process, so `manager.register` fails, and the new instance gets `HotkeyStatus::Failed(...)` with `hotkey_manager: None`. Registration is never retried, so the new and only stripe has no working hotkey until the user quits and relaunches again.

This is a regression from iteration 2. Before it, the name was held until the app (and its `GlobalHotKeyManager`) was dropped, so a relaunch could not win the name while the hotkey was still taken. It is the exact case manual check 11 ("relaunch right after Quit") is meant to exercise, and it also briefly shows two tray icons and two stripes.

**Fix:** In the Quit arm, release the hotkey (and the tray icon) **before** the instance name. The order stays the same as before iteration 2, and the early release is kept. `GlobalHotKeyManager::drop` calls `DestroyWindow` on its hwnd, which frees the hotkeys bound to that window. It has to run on this thread, and it does.

```rust
Some(OverlayCommand::Quit) => {
    quit_requested = true;
    // Give up everything a new instance must own before giving up the name it
    // races for: the hotkey registration first, then the single-instance guard.
    self.hotkey_manager = None;
    self.instance = None;
}
```

If the tray icon should also not be duplicated, make `TrayHandles::tray_icon` droppable (e.g. a `release()` that `take()`s it) and call it here too. Alternatively, make `register_toggle` retry a few times over ~500 ms when the error is "already registered". That also covers a crash-restart, where the old process's hotkey is freed only at process teardown. Add a manual-verification note that check 11 must confirm the relaunched stripe's tray tooltip does **not** say "Ctrl+Shift+Z unavailable".

## Info

### IN-01: When two re-checks race, the loser reports "not responding" while a fresh stripe is starting

**File:** `src/overlay/instance.rs:458-465`

**Issue:** Suppose two launchers were both waiting on an instance that quit. Both time out and both re-run `acquire_named`. One wins (`Run`). The other gets `AlreadyRunning` and returns `signal_message(outcome)` from its first attempt, so the user sees "FerrisFilter is already running but not responding" while the winner's stripe comes up. No second stripe opens; only the message is wrong.

**Fix:** On a re-check that returns `AlreadyRunning`, signal once more (`signal_running(names, startup_ms, ack_ms)`) against the new holder and report that outcome. Or accept this as a residual alongside the others.

### IN-02: A failed re-check silently discards its own error code

**File:** `src/overlay/instance.rs:460-465`

**Issue:** If the re-acquire returns `Acquired::Failed(code)`, the `let`-chain falls through to `signal_message(outcome)`, which reports the original signal outcome. The fail-closed reason (`check_failed_message(code)`) is lost. The behaviour is still safe, because the launch exits, but the message misattributes the cause.

**Fix:** Match all three arms of the re-acquire: `First(g) => Run(g)`, `Failed(code) => Exit(Some(check_failed_message(code)))`, `AlreadyRunning => Exit(signal_message(outcome))`.

### IN-03: `a_launch_whose_instance_quits_mid_request_runs_instead` does not prove the re-check path ran

**File:** `src/overlay/instance.rs:776-800`

**Issue:** Suppose the quitting thread's 150 ms sleep finishes before the launcher's first `acquire_named`, for example because of scheduler delay on a loaded CI box. The launcher then gets `First` directly and the test still passes. A regression that removed the re-check would go undetected in that schedule.

**Fix:** Have the quitting thread drop its guard only after it has seen a show request, i.e. poll `take_show_request()` until it returns true and then `drop(g)` without acking. That guarantees the launcher was already in `request_until_acked` when the name was released.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
