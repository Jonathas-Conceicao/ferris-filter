---
phase: 37-game-folder-resolver-picker
plan: 04
subsystem: overlay
tags: [rfd, folder-dialog, threading]
requires: []
provides:
  - "overlay::folder_dialog::FolderPick (start_with / start / poll / is_open / since_close / note_focused)"
  - "existing_ancestor, keep_visible, CLOSE_GRACE, DIALOG_TITLE"
affects: [37-05]
tech-stack:
  added: ["rfd 0.17.2 (cfg(windows), default features off)"]
  patterns: ["std::thread + mpsc worker; UI try_recv only"]
key-files:
  created: [src/overlay/folder_dialog.rs]
  modified: [Cargo.toml, Cargo.lock, src/overlay/mod.rs]
requirements-completed: [GAME-02]
duration: ~25 min (dominated by a 12 min first build)
completed: 2026-10-18
---

# Phase 37 Plan 04: folder_dialog Summary

Non-blocking native folder pick (rfd on a worker thread, parented to the overlay via `set_parent(frame)`) plus the pure helpers plan 05 needs.

## Commits
- fc7000b chore(37-04): add rfd 0.17 (Windows, default features off)
- 6c47df9 feat(37-04): folder_dialog worker pick and keep-visible helpers

## Provenance check (T-37-SC)
- `cargo info rfd@0.17`: version 0.17.2, license MIT, repository https://github.com/PolyMeilex/rfd. Passed.
- `cargo tree -i raw-window-handle`: a single version, 0.6.2 (used by eframe, egui-winit, rfd, wgpu).
- windows-sys: Cargo.lock shows rfd depending on `windows-sys 0.61.2`, the version the crate already uses (no incompatible second copy from rfd). Incidentally the lock also moved two other dependents from windows-sys 0.52.0 to 0.59.0 (cargo re-resolution; no other behaviour change).
- `cargo tree -i rfd`: only ferrisfilter depends on it.

## Decisions
- Primary design compiled as planned: `rfd::FileDialog` with `set_parent(frame)` is `Send` in 0.17.2 and eframe's `Frame` satisfies the raw-window-handle 0.6 traits, so the OwnerHwnd fallback and the unparented/WindowLevel last resort were not needed.
- The module carries `#![allow(dead_code)]` (with a comment) because nothing uses it until plan 05; plan 05 should remove that line.
- Non-Windows `start` returns false.

## Verification
- `cargo test --lib overlay::folder_dialog`: 11 passed (pick, cancel, panic, single-dialog, repaint, existing_ancestor, keep_visible, grace, note_focused, verbatim path).
- `cargo test --test isolation_guard`: 5 passed. `cargo build --bin ff`: ok.
- Tests ran with FERRISFILTER_HOME / FERRISFILTER_GAME_DIR under a temp dir and CARGO_TARGET_DIR outside the repo; no real dialog opened.

## Deviations from Plan
None, apart from the module-level dead_code allow noted above.

## Human verification
- In game (plan 05 wiring): the dialog appears above the always-on-top overlay and the overlay does not hide while it is open or just after it closes. If the dialog is behind the overlay, apply the WindowLevel contingency described in the module doc.

## For the Release Review
- New third-party dependency `rfd` (MIT, Windows only): README credits / dependency notes may want a mention; no CHANGELOG/README edits made here.

## Self-Check: PASSED
Files and both commits exist.
