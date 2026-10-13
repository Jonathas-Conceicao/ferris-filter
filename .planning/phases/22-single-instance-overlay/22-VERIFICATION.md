---
phase: 22-single-instance-overlay
verified: 2026-10-13T00:00:00Z
status: human_needed
score: 5/5 must-haves verified in code (GUI behaviour pending human UAT)
overrides_applied: 0
human_verification:
  - test: "22-MANUAL-VERIFICATION.md check 1 - launch ff again while the stripe is visible"
    expected: "Stripe stays shown (reveal, never toggle), second process exits within about 2 s, one tray icon, one ff.exe"
    why_human: "Real window, tray and process count cannot be driven headlessly"
  - test: "Check 2 and 4 - stripe hidden to tray, launch again (and Explorer double-click)"
    expected: "Stripe reappears within about a second; second process exits; no error dialog"
    why_human: "Depends on eframe's hidden-window repaint heartbeat plus request_repaint_after(50ms) waking update(); unprovable by grep"
  - test: "Check 3 - PoE2 focused, stripe hidden, launch ff"
    expected: "Stripe appears over the game (bypassing the hotkey foreground gate) without taking keyboard focus"
    why_human: "Foreground and focus behaviour is a live-OS property"
  - test: "Checks 5, 8, 9 - repeated, during-startup and simultaneous launches"
    expected: "Exactly one stripe, one tray icon, one ff.exe; Ctrl+Shift+Z still toggles; no spurious not-responding box"
    why_human: "Hotkey and tray state across real processes"
  - test: "Checks 6, 7, 10 - crash recovery, debug beside release, not-responding box"
    expected: "No stale lock after a kill; separate debug name; message box and no second stripe"
    why_human: "Requires killing or launching real processes"
  - test: "Check 11 - relaunch immediately after tray Quit"
    expected: "Fresh stripe, old tray icon gone, new tooltip names Ctrl+Shift+Z with no failure line, hotkey works"
    why_human: "The hotkey-then-tray-then-name release ordering (WR-01 fix, d717c10) is an OS-resource race no test observes"
---

# Phase 22: Single-Instance Overlay Verification Report

**Phase Goal:** Launching `ff` twice never produces two stripes.
**Status:** human_needed (no code gaps found; the GUI-observable parts need UAT)
**Re-verification:** No, initial verification

## Observable Truths (ROADMAP success criteria + plan must-haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: launching `ff` while running brings up the running stripe and the second process exits | VERIFIED (code) | `mod.rs:43` `let Some(instance_guard) = instance::acquire() else { return Ok(()); }`. `launch_named` returns `Exit` after `Signaled`. The running side polls `take_show_request()` and acks in `app.rs:1330-1337`. |
| 2 | SC2: works when the running stripe is hidden to the tray | VERIFIED in code, runtime wake-up UNCERTAIN | The poll is per-frame and independent of the hotkey gate. It sets `toggle_requested` only when `visibility == Hidden`, so it is a reveal and never a toggle. Whether a hidden egui window keeps running `update()` is routed to human checks 2 and 4. |
| 3 | SC3: one stripe window and a working hotkey after repeated launches | VERIFIED (code) | `acquire()` is the first statement of `launch()`, before `hotkey::register_toggle` (`mod.rs:53`) and before the tray build. A second process therefore never registers a hotkey or builds a tray icon. The mutex is the arbiter, including the re-check path. In-process tests cover the protocol. |
| 4 | Not-responding box after about 2 s, never a second stripe | VERIFIED | `ACK_TIMEOUT_MS=2000`, `NOT_RESPONDING_MESSAGE` is the locked text, and `launch_named` returns `Exit(Some(msg))`. The test `a_launch_to_a_still_running_unresponsive_instance_reports_it` passes. |
| 5 | Debug and release use different names, and tests never use production names | VERIFIED | `names(debug)` appends `-debug`. Tests use `Local\FerrisFilter-test-<pid>-<label>` and call `launch_named`, never `acquire()`. |

**Score:** 5/5 verified in code. Truth 2's runtime behaviour awaits human checks.

## Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/overlay/instance.rs` (964 lines) | VERIFIED | Real mutex, event and MessageBox implementation. Every `unsafe` block carries a SAFETY comment. No `todo!`. |
| `Cargo.toml` | VERIFIED | windows-sys features include `Win32_Security` and `Win32_System_Threading`. No new crate. |
| `src/overlay/mod.rs` | VERIFIED | `pub mod instance;` is present. The gate comes before the hotkey and tray, and the guard is passed into `OverlayApp::new`. |
| `src/overlay/app.rs` | VERIFIED | `instance: Option<Guard>` field. The poll runs after the tray MenuEvent block and before `visibility::advance`. It does not reference `foreground_is_game_or_overlay`. |
| `src/overlay/hotkey.rs` and `tray.rs` | VERIFIED | `release_toggle` unregisters the hotkey and drops the manager. `TrayHandles::release` sets `tray_icon = None`. |
| `22-MANUAL-VERIFICATION.md` | VERIFIED | 11 checks, with Expected and Result fields. Check 11 was updated by the WR-01 fix. |

## Key Links

| From | To | Status |
|------|----|--------|
| `launch()` | `instance::acquire` | WIRED (first statement) |
| `update()` | `Guard::take_show_request` / `ack` / `mark_ready` | WIRED |
| Quit arm | `release_toggle`, then `tray.release()`, then `instance = None` | WIRED, in the order the review fix specifies (`app.rs:1306-1313`) |

## Post-Plan Review-Fix Changes (checked against final code)

| Item | Status |
|------|--------|
| Retry/ready protocol (`RETRY_INTERVAL_MS=100`, `-ready` event) | Present (`request_until_acked`, `request_show`) |
| `STARTUP_TIMEOUT_MS=10_000` | Present, with a unit test |
| Fail closed on mutex error (`Acquired::Failed`, `check_failed_message`) | Present. `ERROR_ACCESS_DENIED` maps to `AlreadyRunning`, then `Unreachable`. |
| Re-acquire after a failed signal | Present, with all three re-check arms (IN-02) |
| Guard, hotkey and tray released on Quit | Present |

## Behavioural Spot-Checks

| Command | Result | Status |
|---------|--------|--------|
| `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib overlay` | 287 passed, 0 failed (includes the `overlay::instance` pure and Windows tests) | PASS |

The full `cargo test` run (753 lib tests plus integration tests) is as reported in the REVIEW-FIX report. I re-ran only the overlay subset.

## Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| INST-01 | 22-01 and 22-02 (both declare `[INST-01]`) | SATISFIED in code, UAT pending | `REQUIREMENTS.md:130` is ticked `[x]`. Traceability row 209 reads "Complete (UAT pending)". |

There are no orphaned requirements. INST-01 is the only ID mapped to Phase 22 (`ROADMAP.md:294`).

## Docs Accuracy

- **CLAUDE.md overlay note:** accurate to the final code. It covers:
  - the 2 s not-responding box
  - the 10 s startup wait
  - several launches at once
  - Quit releasing the name
  - the re-check
  - fail-closed on a mutex error
  - the `-overlay-debug` name
  - the `instance.rs` path
- **CHANGELOG `[Unreleased]`:** accurate. It mentions the show/ack/ready events and the fail-closed behaviour.
- **Minor, non-blocking:** the CLAUDE.md and CHANGELOG text do not mention that Quit also unregisters the hotkey and removes the tray icon first. The REVIEW-FIX report says no doc change was needed, and the behaviour described is still correct.

## Anti-Patterns

No TBD, FIXME or XXX markers and no `todo!` in `instance.rs`. The `launch()` comments describe the Quit ordering accurately.

## Accepted Residuals (documented in REVIEW-FIX, not gaps)

- Two launchers racing a Quit can lead the loser to show a misleading "not responding" box (IN-01). No second stripe results.
- After a crash, the hotkey may fail to register if the relaunch wins the name during process teardown. The tray tooltip reports it.
- A startup-time quit makes a launcher wait out the 10 s startup budget before its re-check.

## Gaps Summary

No code gaps. Every roadmap criterion and plan must-have is implemented, wired and covered by passing in-process tests. The status is `human_needed` because GUI reveal-from-tray, focus behaviour, tray and hotkey state, and the Quit-relaunch ordering can only be confirmed by running `22-MANUAL-VERIFICATION.md`. INST-01 stays "Complete (UAT pending)" until then.

---

_Verified: 2026-10-13_
_Verifier: Claude (gsd-verifier)_
