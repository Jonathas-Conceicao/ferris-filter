---
phase: 22-single-instance-overlay
fixed_at: 2026-10-13T00:00:00Z
review_path: .planning/phases/22-single-instance-overlay/22-REVIEW.md
iteration: 3
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 22: Code Review Fix Report (iteration 3)

**Fixed at:** 2026-10-13
**Source review:** .planning/phases/22-single-instance-overlay/22-REVIEW.md (iteration 3)
**Iteration:** 3 (final)

**Summary:**
- Findings in scope: 1 (WR-01; fix scope critical_warning)
- Fixed: 1
- Skipped: 0
- Also fixed, each in its own commit: IN-02 and IN-03. IN-01 is accepted, with the reason given below.

**Verification:** the full `cargo test` passes: 753 lib tests plus 90 in the integration and
doc-test targets. `overlay::instance` has 28 tests, and they passed 5 runs in a row after the IN-03
change. `cargo fmt --check` is clean. The clippy warning set (counted per message) is identical
before and after this pass, so there are no new warnings in the touched files. `artifact_guard` and
`generate_snapshot` pass, and nothing under `tests/` changed. The tests use per-PID
`Local\FerrisFilter-test-*` names and call `launch_named`, never `acquire()`/`notify()`, so no
MessageBox appears. Everything was built in `CARGO_TARGET_DIR=%TEMP%/ff-target`. `ff` was not
launched, and no process was killed.

## Orchestrator design decisions (made on the user's behalf)

- **Iteration 3:**
  - Tray Quit releases the hotkey and then the tray icon, both **before** the instance name.
  - The early name release from iteration 2 is kept.
  - The hotkey is unregistered explicitly if the crate's `Drop` does not do it (it does not; see WR-01).
  - IN-02 and IN-03 were to be fixed if small and safe, and both were.
  - IN-01 may be accepted.
- **Iteration 2, still in force:**
  - `OverlayApp.instance: Option<Guard>` is dropped on tray Quit.
  - A launcher whose signal fails re-runs `acquire_named` once.
  - Test asserts allow a redundant answer (`>= 1`).
  - `request_until_acked` checks the deadline before each send.
- **Iteration 1, still in force:**
  - Launchers re-send their request every 100 ms within the 2 s ack window.
  - The `-ready` event and the 10 s `STARTUP_TIMEOUT_MS` startup wait.
  - A `CreateMutexW` failure fails closed with a message naming the error. `ERROR_ACCESS_DENIED` means an elevated instance.
  - An instance whose events failed keeps running with none, and later launches report it unreachable.
  - No `unsafe impl Send for Guard`.

## Fixed Issues

### WR-01: Quit releases the instance name before the hotkey, so an immediate relaunch runs with a dead `Ctrl+Shift+Z`

**Files modified:** `src/overlay/app.rs`, `src/overlay/hotkey.rs`, `src/overlay/tray.rs`, `src/overlay/mod.rs`, `.planning/phases/22-single-instance-overlay/22-MANUAL-VERIFICATION.md`
**Commit:** d717c10
**Applied fix:**
- **Crate source check.** I read `global-hotkey` 0.8.0's source (`~/.cargo/registry/.../global-hotkey-0.8.0/src/platform_impl/windows/mod.rs`). Its Windows `impl Drop for GlobalHotKeyManager` only calls `DestroyWindow(self.hwnd)`. It does **not** call `UnregisterHotKey`. Windows does free hotkeys bound to a window when that window is destroyed, but the `RegisterHotKey` documentation does not promise it. The fix therefore unregisters explicitly before dropping the manager.
- **New `hotkey::release_toggle(manager)`.** It calls `manager.unregister(HotKey::new(Some(TOGGLE_MODIFIERS), TOGGLE_CODE))`, which is the same id `register_toggle` registered. It ignores an error, since that means the hotkey is already gone, and then drops the manager. It runs on the main thread, where the hotkey was registered.
- **New `TrayHandles::release()`.** It sets `tray_icon = None`. tray-icon 0.24.2's `Drop` calls `remove_tray_icon` and then `DestroyWindow` on its hidden window, the same work as the normal end-of-process drop and on the same (main) thread. The menu event has already been delivered through the channel by the time `update` handles it, so dropping the icon does not happen inside that window's message handling. The menu ids stay in `TrayHandles`, so `command_for_menu` keeps working. The tray icon could be dropped cleanly, so it is.
- **Quit arm order.** It is now: `hotkey_manager.take()` then `release_toggle`, then `tray.release()`, then `instance = None`. The `#[allow(dead_code)]` on `hotkey_manager` was removed because the field is now read. The field docs and the `launch()` comment describe the order.
- **Manual check 11.** It now also confirms the following:
  - The old tray icon disappears at Quit, so there are never two icons.
  - The relaunched stripe's tooltip names `Ctrl+Shift+Z` and does not say it is unavailable.
  - The menu has no hotkey-failure line.
  - `Ctrl+Shift+Z` toggles the new stripe.
- **Docs.** CLAUDE.md and the CHANGELOG still describe the behaviour correctly ("Quit releases the name at once, so a relaunch starts a fresh stripe"), so they were left unchanged.
- **Status:** fixed: requires human verification. This is an ordering and OS-resource fix that no headless test can observe, so manual check 11 is the confirmation.

### IN-02 (in scope by orchestrator allowance): a failed re-check silently discards its own error code

**Files modified:** `src/overlay/instance.rs`
**Commit:** e79812b
**Applied fix:** after a failed signal, `launch_named` now matches all three re-check arms:
- `First(g)` → `Run(g)`
- `Failed(code)` → `Exit(Some(check_failed_message(code)))`
- `AlreadyRunning` → `Exit(signal_message(outcome))`

A `Signaled` outcome still exits without a message and without a re-check.

### IN-03 (in scope by orchestrator allowance): the quits-mid-request test did not prove the re-check path ran

**Files modified:** `src/overlay/instance.rs`
**Commit:** 4f71fba
**Applied fix:** the quitting thread in `a_launch_whose_instance_quits_mid_request_runs_instead` now polls `take_show_request()` every 5 ms, for up to 5 s, and drops its guard (without acking) only after it sees a request. It returns whether it saw one, and the test asserts that it did. As a result, the launcher is provably inside `request_until_acked` when the name is released, so the test passes only through the re-check.

## Accepted residuals (documented, not fixed)

- **IN-01 (iteration 3):**
  - **The case:** two launchers wait on an instance that quits, and both re-check. The loser gets `AlreadyRunning` and shows "already running but not responding" while the winner's stripe comes up.
  - **Why accepted:** no second stripe opens. It needs two launches during the same Quit, and the only fault is a misleading message.
  - **Why not fixed:** signalling the new holder again would add another wait to a path that is already up to 12 s.
- **WR-01 crash-restart (iteration 3):**
  - **The case:** if the first instance crashes rather than Quits, its hotkey is freed only at process teardown. A relaunch that wins the name during that teardown can still fail to register the hotkey.
  - **Why accepted:** this is crash-only and narrow. The tray tooltip reports it, and a Quit and relaunch recovers.
  - **Not done:** the review's optional `register_toggle` retry was left out.
- **From iteration 2, still true:**
  - **Outcome 2 delay:** a startup-time quit makes a launcher wait out `STARTUP_TIMEOUT_MS` before the re-check starts it. The outcome is correct; only the delay remains.
  - **IN-02 grace slice:** a concurrent launcher's `ResetEvent(ack)` can wipe an ack in another launcher's final slice. This is rare and harmless.
  - **IN-03 `mark_ready` without a window:** it is safe, because the stripe starts visible.
- **From iteration 1, still true:**
  - Events-failed concurrency can show "not responding" instead of "unreachable".
  - An events-failed instance reports the failure only through `eprintln!`.
  - The creation-ordering windows are safe as described.

## Not addressed

None beyond the accepted residuals above.

---

_Fixed: 2026-10-13_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
