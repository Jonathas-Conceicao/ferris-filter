---
phase: 41-check-for-updates
plan: 05
subsystem: overlay
tags: [update-check, install-and-quit, screenshot-mode]
requires: [41-03, 41-04]
provides:
  - "OverlayApp::install_and_quit, pending_install slot, shutdown hand-off"
  - "launch(): installer spawned after eframe::run_native returns"
  - "screenshot --update-state"
affects: []
key-files:
  modified: [src/overlay/app.rs, src/overlay/mod.rs, src/overlay/update_job.rs, src/overlay/screenshot.rs, CLAUDE.md]
  created: [.planning/phases/41-check-for-updates/41-MANUAL-VERIFICATION.md]
key-decisions:
  - "pending_install is a trailing parameter of OverlayApp::new only; build() creates a fresh slot and new() overwrites it, so the test constructors that call build() did not change"
  - "update_job.rs: #![allow(dead_code)] removed; inject is cfg_attr-allowed outside test/debug, install_intended is cfg(test) (only tests read it)"
requirements-completed: [UPD-02, UPD-03]
completed: 2026-10-18
---

# Phase 41 Plan 05: Install and quit hand-off Summary

Install click re-checks the verified installer, goes through `request_quit` (Unapplied changes popup), and the installer is spawned only after `eframe::run_native` returned and `begin_shutdown` released hotkey, tray and instance name.

## Tasks and commits

1. Install and quit, intent clearing, shutdown hand-off, spawn after run_native: 40554b9
2. Screenshot-mode `--update-state` and captures: 5baeb2e
3. CLAUDE.md note, 41-MANUAL-VERIFICATION.md, full suite: 795bab6

## What changed

- `app.rs`: `install_and_quit()` (Failed shows the message and returns to Idle, NotReady no-op, Proceed calls `request_quit`); `dispatch_popup` calls `cancel_install()` on `CloseConfirm` and on a refused Apply; `begin_shutdown` hands `update.shutdown()` into `pending_install`; `app.rs` never calls `spawn_installer`.
- `mod.rs launch()`: slot shared with the app, `spawn_installer` after `run_native`, failure shown via `instance::notify_error("Could not start the FerrisFilter installer: <cause>. It was saved to <path>.")`. `apply.rs` untouched.
- `screenshot.rs` / `new_screenshot`: `--update-state idle|checking|downloading|ready:<ver>:<date>`; ready also sets the `<ver> available, released <date>` status.
- CLAUDE.md: flag list, About sentence, "Check for updates (Phase 41)" note.

## Verification

- New app tests (6): clean install click hands off; unapplied + Cancel clears intent and a later quit deletes the installer; refused Apply clears intent; Quit without applying hands off; plain quit deletes the folder and launches nothing; installer gone shows the spec message and returns to Idle. Screenshot parse test added.
- `cargo test --locked`: 1673 lib tests passed, 0 failed (integration tests incl. artifact_guard, generate_snapshot green). Target dir outside the repo.
- Clippy: no diagnostics in the touched files after the change (the `-D warnings` gate is unusable, about 125 pre-existing errors). Formatted only my files with rustfmt.
- Screenshots (temp home/game/USERPROFILE under `%TEMP%\ff41shots`, not committed): idle shows `Check for updates`; checking and downloading show the dimmed `Checking…` / `Downloading...` button; ready shows `Install 0.8.0 and quit` with status `0.8.0 available, released 2026-11-02`; up-to-date, rate-limit and checksum statuses show in the status line (long ones cut with an ellipsis). In every capture the button is centred under the version line with a visible gap above it and the repo link below. Idle/uptodate/checksum show the Check button; the downloading capture's status line reads the unrelated `The filter does not exist yet` (no filter in the temp game folder).

## Deviations from Plan

- Constructor: see key-decisions (slot set in `new`, not a `build` parameter).
- `update_job.rs` dead-code attributes as in key-decisions.
- `--update-state` parse validation is slightly stricter than the plan text (a `ready:` value with an empty version or non-10-char date is rejected).

## Deferred manual checks

All live checks (real release, real installer, UAC/SmartScreen, offline, rate limit) are in `41-MANUAL-VERIFICATION.md`, status deferred and non-blocking.

## For the Release Review

- README/CHANGELOG: About tab **Check for updates** button (manual only, hash-verified installer, Install and quit through the normal Quit path, nothing runs at start-up).
- Needs a published final release before the first live check can be done (`releases/latest` 404 today).

## Self-Check: PASSED

Commits 40554b9 and 5baeb2e exist; 41-MANUAL-VERIFICATION.md and CLAUDE.md edits committed.
