---
phase: 41-check-for-updates
plan: 04
subsystem: overlay
tags: [about-tab, status-line, update-check]
requires: [41-02, 41-03]
provides:
  - "about_view::{UpdateView, AboutAction, about_tab(ui, skin, &UpdateView) -> Option<AboutAction>}"
  - "shell::event_from with a leading update message slot"
  - "OverlayApp.update / update_message, start_update_check, per-frame poll"
affects: [41-05]
key-files:
  modified: [src/overlay/about_view.rs, src/overlay/shell.rs, src/overlay/app.rs, src/overlay/update_job.rs]
key-decisions:
  - "Both tasks in one commit: the new about_tab signature and event_from arity cannot compile without the app.rs call sites"
  - "update_job.rs keeps #![allow(dead_code)] (comment reworded): begin_install, cancel_install, install_intended, shutdown and InstallClick are wired in 41-05"
requirements-completed: [UPD-01, UPD-02, UPD-03]
duration: 40min
completed: 2026-10-18
---

# Phase 41 Plan 04: About button and Check click Summary

The About tab now has the state-driven 240pt update button (Check for updates / Checking… / Downloading… / Install <version> and quit), a click on Check starts the worker, and the whole outcome shows only in the one status line.

## Tasks

1. About tab button and status event slot, and 2. App state, poll, Check click: 6bdacbb (single commit, see key-decisions)

## What changed

- `about_view.rs`: `UpdateView` (plain data), `AboutAction { Check, InstallAndQuit }`; button sits 8pt below the version line and 8pt above the repo link (the old 4pt gap is now 8pt and unconditional); disabled states use `on_disabled_hover_text`.
- `shell.rs`: `event_from(update, apply, config, sync, list_status)`; update wins.
- `app.rs`: fields `update`, `update_message`; `update_kind` mapping; poll after `leagues.poll()` with repaint (plus a 100 ms repaint timer while Checking/Downloading); `start_update_check` (single caller: the `AboutAction::Check` branch, bound to a local so 41-05 adds its Install branch beside it). `update_message` is cleared in `clear_events_on_conceal`, `clear_events_on_edit` (so also in `start_update_check`), `run_apply`, and where Sync economy starts. The `UpdateJob` state is never touched there, so Ready survives conceal, edits and tab switches.

## Verification

- `cargo test --locked --lib overlay::`: 903 passed, 0 failed (new: `button_copy_matches_the_spec`, click-returns-action test, four-view full and cramped renders, event precedence, `pressing_check_reports_checking_then_the_outcome`, `conceal_keeps_ready_and_clears_the_update_message`, `starting_an_apply_clears_the_update_message`).
- `cargo fmt --check` clean. `cargo clippy --locked --all-targets`: no diagnostics in about_view.rs, app.rs, shell.rs from this work; the only update_job.rs warnings are the dead-code ones covered by the allow. The `-D warnings` gate from the plan was not used (about 125 pre-existing clippy errors).
- Tests use the `cfg(test)` twin of `check_for_update_blocking` (returns Offline), so no network or installer is touched; target dir was outside the repo (`C:/Users/jonat/ff-target-41`).

## Deviations from Plan

- [Rule 3] `cargo fmt` also reformatted `src/sync/league.rs` and `tests/isolation_guard.rs` (unrelated, pre-existing formatting); those changes were reverted and not committed.
- The plan said to remove `#![allow(dead_code)]` if everything is used; the install-side API is still unused until 41-05, so it stays.

## Decisions

- The click-action test scans down the page centre for the button instead of exposing its rect, keeping `about_tab`'s signature as specified.

## For the Release Review

Nothing for README/CHANGELOG yet beyond what the phase as a whole adds (an About-tab "Check for updates" button; status messages in the status line).

## Self-Check: PASSED

Commit 6bdacbb exists; modified files present.
