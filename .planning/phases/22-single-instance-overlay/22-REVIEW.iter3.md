---
phase: 22-single-instance-overlay
reviewed: 2026-10-13T00:00:00Z
depth: standard
iteration: 2
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

# Phase 22: Code Review Report (iteration 2)

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This pass re-reviewed the phase-22 diff (`git diff 5333605`) after the iteration-1 fixes for WR-01, WR-02, WR-03 and IN-03. For `app.rs`, only the changed regions were read: the new `instance` field, the constructor argument, and the per-frame `mark_ready`/`take_show_request`/`ack` block, along with how that block interacts with `visibility::advance` and the Quit path.

The iteration-1 fixes hold up:

- **Retry/ack loop.** `request_until_acked` re-sends the request every 100 ms and waits in slices. The deadline arithmetic saturates, and the `as u32` cast is bounded by `RETRY_INTERVAL_MS`.
- **Withdrawal.** A request is withdrawn on timeout.
- **Ready wait.** `-ready` is waited on before any request is sent, and it is reset both when an instance is created and when it is dropped.
- **Win32 error reads.** `GetLastError` is read immediately after each `Create*` call, both in `open_event` and in `acquire_named`.
- **Ordering.** Events are created before the mutex, and the mutex is closed before the events.
- **Handles.** Every handle is closed exactly once: the event-failure branch nulls handles after `close_all`, and `Drop`/`close_all` skip null handles.
- **Fail-closed.** `Failed(u32)` returns `None` before the hotkey or tray is created. On `ERROR_ACCESS_DENIED`, the outcome is `Unreachable(Some(5))`.
- **2 s window.** For a ready instance, the locked ~2 s ack window still applies: the ready wait returns at once, then `ACK_TIMEOUT_MS` runs.
- **`Win32_Security`.** The feature is needed, because the `CreateEventW`/`CreateMutexW` signatures take `SECURITY_ATTRIBUTES`.
- **Removed `Send` impl.** With it gone, `Guard` (raw `HANDLE`s) is `!Send`. That is sound, because `AppCreator` has no `Send` bound.
- **Tests.** Tests use per-PID, per-label `Local\FerrisFilter-test-*` names and never call `acquire()`/`notify()`, so no MessageBox is shown. All 25 `overlay::instance` tests pass (`cargo test --lib overlay::instance`, 0.70 s).

The accepted residuals in 22-REVIEW-FIX.md were checked, are correct as described, and are not re-raised. One gap remains that those residuals do not cover: an instance that has been told to Quit keeps advertising readiness, and can even answer, until eframe drops the app. A relaunch in that window ends with no stripe at all. In some variants it also shows a message that is false.

## Warnings

### WR-01: A relaunch during Quit ends with no stripe and a false "not responding" / "already running" message

**File:** `src/overlay/app.rs:1316-1323`, `src/overlay/app.rs:1733-1736`, `src/overlay/instance.rs:175-193`, `src/overlay/instance.rs:361-387`, `src/overlay/instance.rs:439-455`

**Issue:** Quit only sends `ViewportCommand::Close`. The `Guard` lives inside `OverlayApp`, so the mutex stays held and `-ready` stays set until eframe has torn down the window and dropped the app. That happens after the stripe has already disappeared from the screen. A user who quits and relaunches right away (for example, to restart into a new build) hits one of three outcomes:

1. **The launcher reaches `acquire_named` before the Guard drops.** It gets `AlreadyRunning`. The ready wait returns at once, because `-ready` is still set. `request_until_acked` then re-sends for 2 s to a process that no longer runs frames, and the launcher shows **"FerrisFilter is already running but not responding"**, then exits. Nothing is running afterwards.
2. **The Guard drops while the launcher is in the ready wait.** `Drop` resets `-ready`, so the launcher blocks for the full `STARTUP_TIMEOUT_MS` (10 s) and then shows the same false message.
3. **The Guard drops between the launcher's `acquire_named` and `signal_running`.** `signal_running` has to create `-ready` itself, so the result is `Unreachable(None)`. The user is then told "FerrisFilter is already running, but this launch could not ask it to show its stripe ... Use its tray icon", but there is no tray icon.

eframe may also run one more `ui()` frame after the Close command. In that case the block at 1317-1322 acks the request, the launcher exits silently as `Signaled`, and the dying process shows nothing.

No second stripe is ever created, so this is not a fail-open. But in all of these cases the user has asked for a stripe, gets none, and in most of them receives a false explanation. Residual 3 in REVIEW-FIX covers only the case where the mutex has already been closed. It does not cover these.

**Fix:** Two changes, which work best together:

```rust
// app.rs — make the guard retire as soon as Quit is decided
instance: Option<instance::Guard>,
...
if quit_requested {
    // Release the instance name now, not when eframe drops the app: a relaunch from here on
    // becomes the new first instance instead of signalling a process that is going away.
    self.instance = None;
    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
}
// and guard the per-frame poll:
if let Some(guard) = self.instance.as_mut() { guard.mark_ready(); if guard.take_show_request() { ... } }
```

```rust
// instance.rs — a launcher whose request failed re-checks whether it is now the first instance
Acquired::AlreadyRunning => {
    let outcome = signal_running(&names, STARTUP_TIMEOUT_MS, ACK_TIMEOUT_MS);
    if outcome != Wait::Signaled {
        if let Acquired::First(g) = acquire_named(&names) {
            return Some(g); // the running instance exited while we were asking
        }
    }
    if let Some(message) = signal_message(outcome) { notify(&message); }
    None
}
```

Re-acquiring cannot open a second stripe, because the mutex is still the arbiter. It also covers outcome 3 and a crash of the first instance during the wait. Add a Windows test for it: drop the first `Guard` while `signal_running` is waiting, then assert that the launcher path reports `First`.

## Info

### IN-01: The exact `answered == 1` assertions can flake when a retry lands between consume and ack

**File:** `src/overlay/instance.rs:665`, `src/overlay/instance.rs:739` (mechanism at `instance.rs:342-357` and `instance.rs:646-649`)

**Issue:** The responder calls `take_show_request()` (which consumes `show`) and then `ack()`. If the launcher's 100 ms slice expires between those two calls, the launcher re-sends `show` and then receives the ack. That leaves a request pending that no launcher owns. The responder answers it on its next 50 ms poll before `stop` is seen, so `answered` becomes 2 and the test fails. The window is microseconds wide, so this is rare, but `assert_eq!(…, 1)` turns it into a test failure. The same mechanism in production causes one redundant reveal-and-ack on an already-shown stripe, plus a stale `ack` that the next launcher's `ResetEvent(ack)` clears. That is harmless.

**Fix:** Assert `>= 1` in both tests (as `concurrent_launchers_are_each_answered` already does). Alternatively, set `stop` before the responder's next poll by joining on the launcher result first and then signalling `stop` from a channel that the responder checks before polling.

### IN-02: A request can still be honoured just after the launcher reports "not responding"

**File:** `src/overlay/instance.rs:342-357`, `src/overlay/instance.rs:380-385`

**Issue:** The loop always calls `SetEvent(show)` at the top of an iteration, including the final one, whose slice is 0 ms. It then returns `TimedOut`, and `request_show` withdraws the request with `ResetEvent(show)`. A first instance that recovers between those two calls still consumes the request: the stripe pops up while (or just before) the "not responding" box is shown. This is the IN-02 outcome from iteration 1, narrowed to a sub-millisecond window rather than eliminated.

A related edge case: a concurrent launcher's single `ResetEvent(ack)` (`instance.rs:378`) can wipe an ack meant for a launcher in its last slice. The retry loop that is supposed to recover it has no time left, so that launcher reports "not responding" even though its request was handled.

**Fix:** Skip the re-send when the remaining time is zero. Optionally, give the retry loop one final `WaitForSingleObject(ack, RETRY_INTERVAL_MS)` grace slice after the deadline without re-sending, so a wiped ack still gets one re-answer window:

```rust
let remaining = deadline.saturating_duration_since(Instant::now());
if remaining.is_zero() { return Wait::TimedOut; }
unsafe { SetEvent(show) };
```

### IN-03: `mark_ready` advertises readiness before the HWND is cached

**File:** `src/overlay/app.rs:1232-1234`, `src/overlay/app.rs:1316-1323`, `src/overlay/app.rs:1330-1337`

**Issue:** `mark_ready()` runs on the first frame even if `win::from_frame` has not produced a window yet. A show request answered while `self.window` is `None` is acked, but no `Reveal` effect can be performed. The state machine still advances, so `visibility` would read Shown while the window was never revealed. Today this cannot happen: the stripe starts visible, so a request in that state never sets `toggle_requested`. But the readiness signal is tied to "first frame" rather than "able to reveal".

**Fix:** Call `mark_ready()` only once `self.window.is_some()` (the existing `STARTUP_TIMEOUT_MS` absorbs the extra frames). Alternatively, document why acking without a window is safe given the visible-at-start invariant.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
