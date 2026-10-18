---
phase: 40-release-pipeline-installer
plan: 01
subsystem: overlay
tags: [single-instance, installer, win32]
requires: []
provides:
  - "ff --quit hidden flag (exit 0 not running/exited, 1 still running at timeout, 2 unreachable)"
  - "instance::quit_running / quit_running_named / QuitOutcome / quit_exit_code / is_quit_args"
affects: [40-05, 41]
tech-stack:
  added: [windows-sys feature Win32_System_Diagnostics_ToolHelp]
key-files:
  modified:
    - src/overlay/instance.rs
    - src/overlay/app.rs
    - src/overlay/mod.rs
    - Cargo.toml
key-decisions:
  - "One quit request per --quit run, withdrawn with ResetEvent on timeout"
  - "Reuse QuitEvent::TrayQuit via request_quit; no new quit_step variant"
requirements-completed: [INST-04]
duration: 40min
completed: 2026-10-18
---

# Phase 40 Plan 01: `ff --quit` Summary

`ff --quit` asks the running instance to quit through a new `-quit` auto-reset event in the single-instance gate, with the same behaviour as tray Quit (clean: quits; unapplied edits: Unapplied changes popup).

## Tasks

1. Gate: `-quit` event, `Guard::take_quit_request`, `quit_running_named` (opens, never creates, the mutex; collects same-exe `ff.exe` processes via Toolhelp before signalling; waits on their handles, else on the mutex name vanishing; sends the request once; withdraws it on timeout). Commit 2896a80.
2. Wiring: `OverlayApp::request_quit` shared by the tray arm and the `take_quit_request` poll; `--quit` branch first in `launch()`. Commit d6b1037.

## Verification

- `cargo test --locked --lib overlay::instance`: 35 passed (new: quit_exit_code_maps_each_outcome, quit_flag_is_exactly_one_argument, quit_timeout_is_ten_seconds, quit_on_a_free_name_reports_not_running, no_quit_request_without_a_signal, a_polling_instance_quits_on_request, an_instance_that_never_polls_times_out_and_the_request_is_withdrawn).
- `cargo test --locked --lib overlay`: 875 passed.
- Debug `ff.exe --quit` with no debug instance: exit 0 at once. Run with temp FERRISFILTER_HOME/GAME_DIR; the release instance was never touched.
- `cargo fmt --check` shows only pre-existing diffs in app.rs:1328, sync/league.rs, tests/isolation_guard.rs (not touched here).

## Deviations

None in behaviour. An accidental `cargo fmt` reformatted three unrelated files; they were restored with `git checkout -- <file>` before committing.

## For Phase 41

The updater can reuse `instance::quit_running()` logic or rely on the installer's PrepareToInstall running `ff --quit`; exit codes 0/1/2.

## For the Release Review

- Hidden `ff --quit` flag (installer use, undocumented in help): closes a running `ff` the same way tray Quit does. Possibly a CHANGELOG line under installer work.
- Manual end-to-end check (installed ff running, clean and with unapplied edits) is deferred to plan 40-07 UAT.

## Known Stubs

None.

## Self-Check: PASSED

Commits 2896a80 and d6b1037 exist; modified files present.
