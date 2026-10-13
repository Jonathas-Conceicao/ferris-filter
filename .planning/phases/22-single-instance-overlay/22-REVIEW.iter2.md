---
phase: 22-single-instance-overlay
reviewed: 2026-10-13T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/overlay/app.rs
  - Cargo.toml
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 22: Code Review Report

**Reviewed:** 2026-10-13
**Depth:** standard (phase-22 diff vs `5333605`; app.rs limited to the changed regions and the visibility/toggle path)
**Files Reviewed:** 4
**Status:** issues_found

## Summary

The gate is mostly sound. Handles are owned and closed exactly once, and `Drop` skips nulls on
the fail-open paths. `GetLastError` is read immediately after `CreateMutexW`, with every wide
string built beforehand. The guard is acquired before the hotkey and tray are created. Test
names are per-PID, carry a per-test label and never touch the production names. I ran
`cargo test --lib overlay::instance` and all 13 tests pass. I also checked the comment's claim
that eframe 0.36 keeps invisible windows ticking: `eframe-0.36.2/src/native/run.rs:27` sets
`INVISIBLE_WINDOW_REPAINT_INTERVAL` to 100 ms, so a stripe hidden to the tray still polls the
show event.

The defects are in the show/ack protocol. It is one auto-reset `show` event answered by one
auto-reset `ack`, under a fixed 2-second deadline. It cannot answer more than one waiting
launcher, and it does not cover a first instance that is still starting up. In both cases a
healthy overlay produces the "not responding" box. Separately, the fail-open paths can break
the phase's "never two stripes" guarantee, and they report the problem only through
`eprintln!`, which the release GUI binary cannot show. No critical issues were found.

## Warnings

### WR-01: Concurrent second launches coalesce into one show and one ack, so all but one of them report "not responding"

**File:** `src/overlay/instance.rs:197-212` (with `src/overlay/app.rs:1311-1317`)
**Issue:** Take launches B and C close together. This happens with an impatient double
double-click, or when Explorer opens two items. Both call `ResetEvent(ack)` and
`SetEvent(show)`, then wait on `ack`. `show` is auto-reset, so the two signals merge into one
pending request. The first instance consumes it once per frame and calls `SetEvent(ack)` once.
An auto-reset event releases exactly one waiter. The other launcher waits the full 2 s, gets
`Wait::TimedOut` and shows "FerrisFilter is already running but not responding", even though
the stripe was revealed. There is a second problem in the same code: C's `ResetEvent(ack)` can
run after the first instance set `ack` but before B's wait reaches the kernel, which wipes B's
answer. No test covers more than one signaller.
**Fix:** Have the waiter re-assert its request until it is answered, instead of relying on a
single edge:
```rust
let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
unsafe { ResetEvent(ack) };
loop {
    unsafe { SetEvent(show) };
    let code = unsafe { WaitForSingleObject(ack, 100) };
    if classify_wait(code) == Wait::Signaled { break Wait::Signaled; }
    if std::time::Instant::now() >= deadline { break Wait::TimedOut; }
}
```
Because each launcher keeps setting `show`, every one of them eventually receives its own ack.
An alternative is a manual-reset `ack` that the first instance pulses only after a show
request. Add a test with two concurrent `signal_running` callers against one responder, and
assert that both return `Signaled`.

### WR-02: A launch during the first instance's startup always times out and shows "not responding"

**File:** `src/overlay/instance.rs:189-194`, `src/overlay/mod.rs:38-103`
**Issue:** The events exist as soon as `acquire_named` returns. The first ack is only possible
inside `OverlayApp::update`, though, and that runs after the tray is built, after `run_native`
creates the window and GPU context, and after `OverlayApp::new`, which resolves paths and
loads and compiles `dynamic.json`, `config.json`, styles and prices. On a cold start this can
take more than `ACK_TIMEOUT_MS` (2000 ms). It is also exactly when users relaunch, because "the
stripe hasn't appeared yet" is the usual reason for double-clicking again. The second launch
then shows "already running but not responding" for an instance that is fine. The stale
request is consumed on the first frame and the ack is set with no waiter (see IN-02).
**Fix:** Combine the WR-01 retry loop with a deadline that covers startup, for example 5 s.
Alternatively, tell the startup case apart: the first instance creates a manual-reset
`-ready` event and sets it on its first `update`. The second instance waits longer, or shows a
"FerrisFilter is starting" message instead of "not responding", while `-ready` is unset.

### WR-03: Fail-open paths can produce two stripes or a permanent false "not responding", and give no visible diagnostic

**File:** `src/overlay/instance.rs:174-181, 189-193`
**Issue:** There are two fail-open paths:
1. **`CreateMutexW` fails with anything other than `ERROR_ACCESS_DENIED`.** For example,
   `ERROR_INVALID_HANDLE` is returned when any other object type already holds the name. The
   process continues as `First`. The same failure repeats on every later launch, so each launch
   opens another stripe, another tray icon and another hotkey attempt. This directly
   contradicts CONTEXT's "It never opens a second stripe" and "exactly one stripe and one
   working hotkey".
2. **`CreateEventW` fails in the first instance.** The first instance runs with
   `show`/`ack == null`. Every later launch takes the `AlreadyRunning` path and creates fresh
   events nobody polls, so it always shows "not responding" while the overlay is healthy.

Both paths report only through `eprintln!`. Release `ff` is a GUI-subsystem binary with no
console (CLAUDE.md), so the user never sees why.

**Fix:** Decide the policy explicitly and make it visible:
- **Mutex failure:** fail closed by showing a MessageBox that names the error code and
  exiting. Or, if failing open is kept, record the reason in `OverlayApp` and show it in the
  tray tooltip, the way `HotkeyStatus::Failed` is (D-10).
- **Event failure:** treat it like a mutex failure. Release the mutex so the instance is not a
  "deaf" first instance that turns every later launch into a false timeout. Or surface it the
  same way.

If fail-open is the intended trade-off, document in 22-CONTEXT/CLAUDE.md that it breaks the
"never two stripes" invariant.

## Info

### IN-01: A show request in the same frame as focus loss or a toggle can leave the stripe hidden but acknowledged

**File:** `src/overlay/app.rs:1311-1323`
**Issue:** The request sets `toggle_requested` only when `self.visibility == Hidden` at that
moment. If the stripe is `ShownFocused` and loses focus in the same frame, nothing is set.
`advance(ShownFocused, false, false)` then returns `Hidden`, but the ack is still sent. The
same happens when a hotkey press or tray left-click in that frame already set
`toggle_requested` while shown. The launcher exits believing the stripe was revealed, which
breaks the "reveal, never a toggle" rule. Focus loss usually happens a human-scale interval
before the second process starts, so this is unlikely.
**Fix:** Track a local `show_requested` flag, and after `advance` force
`if show_requested && next == Visibility::Hidden { next = Visibility::ShownUnfocused }`.
`effect_for` then issues the `Reveal`.

### IN-02: A timed-out request is still delivered later

**File:** `src/overlay/instance.rs:206-219`
**Issue:** When `signal_running` times out, it closes its handles but leaves `show` signalled.
The first instance keeps the event object alive, so if it recovers (or finishes starting up,
see WR-02), it reveals the stripe after the user was told it is "not responding". It also sets
`ack` with no waiter, and `ack` stays signalled until the next launcher's `ResetEvent`. That
reset makes the leftover ack harmless, but the late reveal contradicts the message just shown.
**Fix:** On timeout, call `ResetEvent(show)` before closing the handles to withdraw the
request. With the WR-01 retry loop this happens naturally.

### IN-03: `unsafe impl Send for Guard` is unnecessary

**File:** `src/overlay/instance.rs:114-115`
**Issue:** The `Guard` never crosses threads. eframe's `AppCreator` has no `Send` bound
(`eframe-0.36.2/src/epi.rs:49`), and the threaded test creates its `Guard` inside the spawned
thread. The impl is not unsound, since kernel handles are process-wide, but it is an
`unsafe impl` that nothing needs, in a module whose header promises minimal, justified
`unsafe`.
**Fix:** Remove it, or add a comment naming the consumer that requires it.

### IN-04: An elevated first instance yields a misleading "not responding" message

**File:** `src/overlay/instance.rs:167-173, 197-201`
**Issue:** On `ERROR_ACCESS_DENIED`, `acquire` returns `AlreadyRunning`, and `signal_running`'s
`CreateEventW` calls then fail for the same reason. The result is `TimedOut`, so the box says
"already running but not responding" even though the instance is responding and is only
running at a different integrity level. This is the correct outcome (no second stripe) with
the wrong explanation.
**Fix:** Pass the access-denied case through to a distinct message, such as "FerrisFilter is
already running as administrator".

### IN-05: Protocol tests cover only one signaller and a ready first instance

**File:** `src/overlay/instance.rs:344-434`
**Issue:** The Windows tests isolate their names properly (PID plus label, never production),
but they only exercise one launcher against a responder that is already polling. The coalescing
failure in WR-01 and the startup window in WR-02 are therefore untested, and the suite passes
with both defects present.
**Fix:** Add two tests:
- Two threads calling `signal_running` concurrently against one polling responder; assert that
  both return `Signaled`.
- A responder that starts polling after a delay; assert the intended startup behaviour.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
