---
phase: 41-check-for-updates
plan: 03
subsystem: overlay
tags: [worker-thread, state-machine, installer]
requires: [41-01]
provides:
  - "src/overlay/update_job.rs: UpdateState, UpdateMsg, UpdateProgress, UpdateJob, InstallClick"
affects: [41-04, 41-05]
tech-stack:
  added: []
  patterns: [third copy of EconomySync/LeagueList worker pattern with a stage message]
key-files:
  created: [src/overlay/update_job.rs]
  modified: [src/overlay/mod.rs]
key-decisions:
  - "Both tasks landed in one commit: they share one new file and the shutdown/abandon code is small"
  - "Module carries #![allow(dead_code)] until 41-04/41-05 wire it; remove then"
requirements-completed: [UPD-01, UPD-02, UPD-03]
duration: 15min
completed: 2026-10-18
---

# Phase 41 Plan 03: Update job model Summary

Framework-free `UpdateJob` behind the About-tab button: Idle/Checking/Downloading/Ready states with the UI-SPEC captions, enabled flags and tooltips, a worker thread reporting a Downloading stage before Done, and the install-on-quit intent that decides at shutdown whether the verified installer is handed off or its folder deleted.

## Tasks

1. States, worker and stage messages, and 2. install intent, click-time re-check, shutdown cleanup: 3c6a551

## Verification

- `cargo test --locked --lib overlay::update_job`: 18 passed (all behavior rows: state tables, start no-op when not Idle, Downloading then Ready, both messages in one poll, up-to-date/failed, panicking worker, inject, begin_install not-ready/missing/tampered/valid, cancel keeps Ready, shutdown with intent / without / after cancel / while Downloading / Idle, no deletion outside the `FerrisFilter-update-` prefix).
- `cargo clippy --locked --lib`: no diagnostics in update_job.rs. File formatted with rustfmt edition 2024.
- No `egui`, `Command::new` or `spawn_installer` in the file; `economy.rs` and `leagues.rs` unchanged.

## Deviations from Plan

- Single commit for both tasks (see key-decisions). Otherwise as written. The `-D warnings` clippy gate was not used because of pre-existing errors elsewhere.

## Decisions

- `begin_install` failures are rendered with UTC offset 0 (the messages carry no date).
- `abandon` deletes the published work_dir slot path and, when Ready, the installer's parent, each only if the final component starts with `WORK_PREFIX`.
- `poll` clears the work_dir slot on any non-Ready Done; on Ready the slot stays so shutdown can clean the folder.

## For the Release Review

Nothing for README/CHANGELOG from this plan (internal model only).

## Self-Check: PASSED

src/overlay/update_job.rs exists; commit 3c6a551 exists.
