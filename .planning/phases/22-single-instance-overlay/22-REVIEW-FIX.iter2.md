---
phase: 22-single-instance-overlay
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/22-single-instance-overlay/22-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 22: Code Review Fix Report

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/22-single-instance-overlay/22-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (WR-01..WR-03; fix scope critical_warning)
- Fixed: 3
- Skipped: 0
- Also fixed: IN-02 (as part of WR-01/WR-02), IN-03 (own commit), IN-05 (tests added with WR-01..WR-03). IN-04 is partly addressed by WR-03 (see below). IN-01 is not addressed.

**Verification:** full `cargo test` passes (750 lib tests, up from 738, plus 90 in the integration
and doc-test targets, all passing; `overlay::instance` went from 13 to 25 tests).
`cargo fmt --check` is clean. Clippy reports no warnings in `instance.rs` or `mod.rs`. The
`app.rs` warnings it reports were already there; they only moved down by the lines added.
`artifact_guard` and `generate_snapshot` pass, and no fixture changed. Every Windows test uses
per-PID, per-label `Local\FerrisFilter-test-*` names and calls `notify` (the MessageBox) nowhere.
`ff` was built, not launched.

## Orchestrator design decisions (made on the user's behalf)

- **WR-01:** the launcher re-sends the show signal every 100 ms (`RETRY_INTERVAL_MS`) until it sees an ack or the deadline passes. The 2 s response window (`ACK_TIMEOUT_MS`, locked in CONTEXT) is unchanged for a running, ready instance.
- **WR-02:** there is a manual-reset `-ready` named event, which the first instance sets on its first `update` frame. A launcher that finds the instance not yet ready waits up to `STARTUP_TIMEOUT_MS` (10 s, a named constant) before the 2 s ack window starts. Only after that does it report "not responding". It never opens a second stripe.
- **WR-03:** if `CreateMutexW` fails for any reason other than `ERROR_ACCESS_DENIED`, the launch fails closed: a MessageBox names the Win32 error code and the process exits. `ERROR_ACCESS_DENIED` keeps meaning "an elevated instance is running". If the first instance cannot create its events, it keeps running, because it holds the mutex. The second launch's message is made accurate where feasible (details below).
- Info items were out of scope unless trivial. IN-03 was removed because the build still compiles without it. The IN-05 tests came with WR-01 and WR-02. CLAUDE.md, the CHANGELOG `[Unreleased]` entry and `22-MANUAL-VERIFICATION.md` were updated in the same commit as the behaviour change they describe.

## Fixed Issues

### WR-01: Concurrent second launches coalesce into one show and one ack

**Files modified:** `src/overlay/instance.rs`
**Commit:** d450502
**Applied fix:** `signal_running` now calls a new `request_until_acked` loop. The loop re-sets `show`
and waits on `ack` in slices of up to `RETRY_INTERVAL_MS` (100 ms) until it gets an ack or the
deadline passes. Each waiting launcher keeps its own request pending, so every one of them is
answered. On timeout the launcher withdraws its request with `ResetEvent(show)`, which also
fixes IN-02. The single `ResetEvent(ack)` before the first request is **kept on purpose**:
without it, a stale ack left by an earlier launch would count as an answer. The race it caused
(a concurrent launcher wiping another launcher's ack) is now harmless, because the wiped
launcher re-sends its request and is answered on the next frame. New tests:
`concurrent_launchers_are_each_answered` (two launchers released by a barrier against one
responder polling every 50 ms, and both return `Signaled`), `a_timed_out_request_is_withdrawn`
and a constant test. `no_show_request_without_a_signal` no longer asserts that a timed-out
request is still pending, because that was the IN-02 behaviour.

### WR-02: A launch during the first instance's startup always times out

**Files modified:** `src/overlay/instance.rs`, `src/overlay/app.rs`, `src/overlay/mod.rs`, `CLAUDE.md`, `CHANGELOG.md`, `.planning/phases/22-single-instance-overlay/22-MANUAL-VERIFICATION.md`
**Commit:** 4b0c20f
**Applied fix:**
- **Ready event and startup wait.** `InstanceNames` gained `ready` (`{base}-ready`), a manual-reset event that every party creates with the same parameters. `Guard::mark_ready(&mut self)` sets it once; `OverlayApp::update` calls it every frame just before polling, and calls after the first do nothing. `signal_running(names, startup_ms, ack_ms)` first waits up to `startup_ms` for `ready` and only then sends requests, so an instance that never becomes ready is not left holding a request. `acquire()` passes `STARTUP_TIMEOUT_MS` (10 000) and `ACK_TIMEOUT_MS` (2000).
- **Ready never outlives its instance.** `ready`, `show` and `ack` are reset when a new first instance creates them, and `ready` is reset in `Drop`. This matters when a launcher still holds the events after the previous instance has quit.
- **Tests.** New tests: `a_launch_during_startup_waits_for_ready` (the responder becomes ready after 600 ms, longer than the 300 ms ack window, and the result is `Signaled`), `a_never_ready_instance_times_out_without_leaving_a_request`, `a_launch_after_the_instance_quit_does_not_see_its_readiness`, plus a constant test.
- **Docs.** CLAUDE.md and the CHANGELOG describe the startup wait. The manual verification script gained check 8 (launch during startup) and check 9 (several launches at once), and the old check 8 (not responding) is now check 10.

### WR-03: Fail-open paths can produce two stripes or a permanent false "not responding"

**Files modified:** `src/overlay/instance.rs`, `src/overlay/mod.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 81f9f63
**Applied fix:**
- **Mutex failure fails closed.** `Acquired` gained `Failed(u32)`, which `acquire_named` returns when `CreateMutexW` fails with anything other than `ERROR_ACCESS_DENIED`. `acquire()` then shows `check_failed_message(code)` in a MessageBox and returns `None`, so no stripe, tray icon or hotkey is created. The old fail-open path that returned `First` with a null mutex is gone.
- **Event failure gives the second launch an accurate message.** The first instance now creates its three events *before* the mutex. If any of them fails, it closes all of them and keeps running with none. Because of that ordering, a healthy instance's `-ready` event exists for as long as it holds the mutex, and the `Guard` now closes the mutex before its events.
- **How the launcher reports it.** If the launcher has to create `-ready` itself (`GetLastError() != ERROR_ALREADY_EXISTS`), or cannot open an event at all, `signal_running` returns the new `Wait::Unreachable(Option<u32>)` straight away, without the 10 s wait. `signal_message` maps that outcome to "FerrisFilter is already running, but this launch could not ask it to show its stripe (Windows error N / its show signal is unavailable). Use its tray icon or Ctrl+Shift+Z instead." This also partly fixes IN-04: an elevated instance now produces this message, naming error 5, instead of "not responding".
- **Message helpers.** `notify_unresponsive()` became `notify(&str)`; nothing outside the module used it.
- **Tests.** The message functions are pure and unit-tested. New Windows tests: `a_name_held_by_another_object_type_fails_closed` (`Failed(6)`, and `First` once the conflicting object is gone), `an_instance_without_events_is_reported_unreachable_at_once` and `an_instance_whose_event_creation_failed_keeps_running_and_is_unreachable`.

**Accepted residuals (documented, not fixed):**
- If a first instance whose events failed is reached by two launchers at the same moment, the second launcher can see the first launcher's temporary `-ready` handle. It then waits out the 10 s startup budget and shows "not responding" instead of "unreachable". Either way no second stripe opens.
- A first instance whose events failed tells nobody but `eprintln!`, which is invisible in release. It does not add a line to the tray tooltip. The second launch's message is now where the user finds out.
- A launcher that arrives while the first instance is between creating its events and creating its mutex sees the events already in place, so the path is safe. A launcher that arrives while an exiting instance has closed its mutex but not yet its events becomes the new first instance, which is correct.

### IN-03 (in scope by orchestrator allowance): `unsafe impl Send for Guard` is unnecessary

**Files modified:** `src/overlay/instance.rs`
**Commit:** 3206f92
**Applied fix:** removed the impl. The `ff` binary, the library and every test still compile and pass. The Guard is moved into eframe's `AppCreator` closure, which has no `Send` bound, and the threaded tests create their Guard on the spawned thread.

## Not addressed

- **IN-01** (a show request in the same frame as focus loss or a toggle can leave the stripe hidden but acknowledged): out of scope (Info) and not trivial. The review rates it unlikely.

---

_Fixed: 2026-10-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
