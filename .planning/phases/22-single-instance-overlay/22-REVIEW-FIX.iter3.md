---
phase: 22-single-instance-overlay
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/22-single-instance-overlay/22-REVIEW.md
iteration: 2
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 22: Code Review Fix Report (iteration 2)

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/22-single-instance-overlay/22-REVIEW.md (iteration 2)
**Iteration:** 2

**Summary:**
- Findings in scope: 1 (WR-01; fix scope critical_warning)
- Fixed: 1
- Skipped: 0
- Also fixed: IN-01 (test flake, own commit) and the first half of IN-02 (own commit). IN-03 is accepted, with the reason given below.

**Verification:** the full `cargo test` passes: 753 lib tests (up from 750) plus 90 in the
integration and doc-test targets, all passing. `overlay::instance` went from 25 to 28 tests.
`cargo fmt --check` is clean. Clippy's total count is unchanged at 95. One new `collapsible_if`
in `instance.rs` came up during the work and was fixed with a let-chain. Every `app.rs` warning
was already there before this pass; they only moved down by the lines added. `artifact_guard`
and `generate_snapshot` pass, and no fixture changed. The tests use per-PID
`Local\FerrisFilter-test-*` names and call `launch_named` (which returns the message), never
`acquire()`/`notify()`, so no MessageBox appears. `ff` was built, not launched, and no process
was killed.

## Orchestrator design decisions (made on the user's behalf)

- **Iteration 2, WR-01:** the guard can now be dropped on Quit (`Option<Guard>` in `OverlayApp`), and a launcher whose signal fails re-runs `acquire_named` once before showing any message. IN-01 is fixed because it is a test flake. IN-02 and IN-03 were to be fixed only if trivial and safe, and otherwise recorded as accepted.
- **Carried forward from iteration 1:**
  - Launchers re-send their request every 100 ms (`RETRY_INTERVAL_MS`) within the locked 2 s ack window.
  - A `-ready` event, together with a 10 s `STARTUP_TIMEOUT_MS`, means a launch during startup is not reported as "not responding".
  - A `CreateMutexW` failure fails closed with a MessageBox naming the error. `ERROR_ACCESS_DENIED` still means "an elevated instance is running".
  - A first instance whose events failed keeps running with none, and later launches report it as unreachable.
  - The unneeded `unsafe impl Send for Guard` was removed.

## Fixed Issues

### WR-01: A relaunch during Quit ends with no stripe and a false "not responding" / "already running" message

**Files modified:** `src/overlay/app.rs`, `src/overlay/instance.rs`, `src/overlay/mod.rs`, `CLAUDE.md`, `CHANGELOG.md`, `.planning/phases/22-single-instance-overlay/22-MANUAL-VERIFICATION.md`
**Commit:** 3bfb1e6
**Applied fix:**
- **Quit releases the name.** `OverlayApp.instance` is now `Option<instance::Guard>`. The tray Quit arm sets it to `None` before that frame's show-request poll. As a result, the mutex is released and `-ready` is reset at once, rather than when eframe finally drops the app, and a request that arrives in the same frame or later is never acked by the closing process. The per-frame `mark_ready`/`take_show_request`/`ack` block runs under `if let Some(guard)`.
- **A failed signal re-checks the name.** The launch decision moved into a testable `launch_named(names, startup_ms, ack_ms) -> Launch { Run(Guard), Exit(Option<String>) }`, and `acquire()` became a thin wrapper that shows the message. When `signal_running` returns anything but `Signaled`, `launch_named` calls `acquire_named` once more and runs as the first instance if the name is now free. That covers outcome 1, outcome 3 (`Unreachable(None)` from a just-released name), and a crash of the first instance during the wait. The mutex is still the arbiter, so a second stripe cannot open.
- **Tests.** New tests:
  - `a_launch_whose_instance_quits_mid_request_runs_instead`: a ready but unanswering instance drops its Guard 150 ms into a 500 ms ack window, and the launch returns `Run` and holds the name.
  - `a_launch_to_a_responding_instance_exits_without_a_message`
  - `a_launch_to_a_still_running_unresponsive_instance_reports_it`: the re-check never overrides a live holder.
- **Docs.** CLAUDE.md, the CHANGELOG `[Unreleased]` entry, the `launch()` comment and the module doc describe the early release and the re-check. `22-MANUAL-VERIFICATION.md` gained check 11 (relaunch right after Quit).
- **Status:** fixed. This is a logic change in the launch path. The automated tests cover it, but manual check 11 is the real-process confirmation.

### IN-01 (in scope by orchestrator allowance): exact `answered == 1` assertions can flake

**Files modified:** `src/overlay/instance.rs`
**Commit:** 4e06afa
**Applied fix:** `a_responding_first_instance_acks_the_show_signal` and `a_launch_during_startup_waits_for_ready` now assert `>= 1`, the same as `concurrent_launchers_are_each_answered`. A comment in each explains the harmless double answer.

### IN-02 (partial, trivial part only): a request re-sent on the final zero-length slice

**Files modified:** `src/overlay/instance.rs`
**Commit:** 39093e9
**Applied fix:** `request_until_acked` now checks the remaining time *before* each `SetEvent(show)` and returns `TimedOut` when under 1 ms is left, because the wait is in whole milliseconds. It used to check *after* each wait, so it no longer sends a request it has no time to wait for. The optional final "grace slice" for an ack wiped by a concurrent launcher was **not** added (see residuals).

## Accepted residuals (documented, not fixed)

- **From iteration 1, still true:**
  - If two launchers reach an events-failed instance at the same moment, one of them can see the other's temporary `-ready` and show "not responding" after 10 s instead of "unreachable".
  - An events-failed instance reports the failure only through `eprintln!`.
  - The events/mutex creation ordering windows are safe as described in iteration 1.
- **WR-01, outcome 2 (narrowed, not eliminated):** suppose a launcher is in the ready wait and the running instance's Guard drops. That now means a Quit or crash landed after the launcher's `acquire_named` but before `-ready` was set, which is mostly a startup-time quit. `Drop` resets `-ready`, so the launcher still waits out `STARTUP_TIMEOUT_MS` (10 s). The re-check then makes it start a fresh stripe, where it used to show a false message. The cost is a delay, and the outcome is correct.
- **IN-02, second half:** a concurrent launcher's single `ResetEvent(ack)` can still wipe an ack meant for a launcher in its very last slice, and that launcher reports "not responding" although its request was handled. This needs two launches within the same final ~100 ms of a 2 s window against a slow instance. It is rare and harmless (the stripe is shown, and no second stripe opens), so the grace slice was not worth the extra protocol surface.
- **IN-03:** `mark_ready` is not gated on `self.window.is_some()`. Gating it would make a persistently failing `win::from_frame` turn every later launch into a 10 s wait and a "not responding" message. Today, an ack without a window is safe because the stripe starts visible (`ShownUnfocused`), so a request in that state never sets `toggle_requested` and needs no reveal.

## Not addressed

None beyond the accepted residuals above.

---

_Fixed: 2026-10-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
