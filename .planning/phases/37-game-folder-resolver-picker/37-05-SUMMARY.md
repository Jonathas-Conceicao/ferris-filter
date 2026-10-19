---
phase: 37-game-folder-resolver-picker
plan: 05
subsystem: overlay
tags: [rust, egui, game-folder, folder-dialog, status-line]
requires: [37-01, 37-03, 37-04]
provides:
  - "OverlayApp::on_folder_picked / apply_folder / adopt_settings / folder_dialog_open"
  - "ConfigAction::BrowseFolder, ConfigAction::RefreshLists (handled; widgets come in 37-06)"
  - "StatusInputs.game_folder_missing, StatusLine.open_tab"
affects: [37-06]
key-files:
  modified: [src/overlay/app.rs, src/overlay/shell.rs, src/overlay/config_view.rs, src/overlay/folder_dialog.rs]
decisions:
  - "The three tasks were committed as one commit (a44cdcc): they all edit app.rs and the pieces do not compile separately (status fields, actions and ui() wiring reference each other)"
  - "Clicking the missing-folder status reuses picked_tab, so it also runs the normal Config-tab entry (disarm, refresh_filters)"
  - "build() now calls refresh_filters() once so game_dir_missing and the filter list are right at launch (one directory read)"
  - "default_game_dir and folder_dialog_open carry #[allow(dead_code)] until 37-06 reads them"
metrics:
  tasks: 3
  completed: 2026-10-18
---

# Phase 37 Plan 05: Folder pick state machine Summary

The overlay can now change, persist and refresh its game folder, and reports a missing one on the status line with a click-through to the Config tab. Only the Config tab widgets remain (plan 06).

## Commits

- a44cdcc: feat(37-05): game folder pick flow, missing-folder status, refresh action and dialog worker

## What was built

- **Status (GAME-04):** `StatusInputs.game_folder_missing` row ranks after `filter_error` and before `syncing` (Error, `open_tab = Config`). `StatusLine.open_tab` makes the bar status clickable; the click sets the tab. The flag `game_dir_missing` is computed only in `refresh_filters` (and so at build).
- **Pick (GAME-02/03):** `on_folder_picked` ignores a pinned folder and a pick equal (ASCII case-insensitive) to the current one; with dirty rules it sets `pending_folder` and opens the existing Switch filter? popup (target shows the path); otherwise `apply_folder`. `apply_folder` chooses the filter via `settings::choose_filter` and saves `game_folder` + `filter` in one `settings::save` through the new shared `adopt_settings` (also used by `switch_to`). Stay clears the pick and writes nothing. CONFIG is never touched.
- **Fresh install:** with no saved `filter`, build selects the folder's filter in memory (nothing written at launch).
- **Refresh (GAME-05):** `RefreshLists` = `refresh_filters` + `sound_files::refresh`, nothing else.
- **Dialog worker:** `ui()` polls `folder_pick` first, starts it after the frame's config actions (where `frame` is in scope; never in screenshot mode). Visibility uses `keep_visible(real_focus, is_open, since_close)` with `note_focused()` on real focus and a repaint scheduled for the grace; the hotkey toggle is dropped while the dialog is open. `viewport().focused` still has a single consumer. The module-level `#![allow(dead_code)]` in `folder_dialog.rs` was removed.

## Verification

- `cargo test --lib -- overlay::app:: overlay::config_view overlay::visibility`: 174 passed, 0 failed.
- Targeted new tests (12 app, 3 shell) + `overlay::folder_dialog`: 35 passed.
- `cargo test --test isolation_guard`: 5 passed. `cargo build --bin ff`: ok, no warnings.
- All runs used FERRISFILTER_HOME / FERRISFILTER_GAME_DIR under a temp dir; no real dialog opened. Full `cargo test --lib` not run (per instructions).

## Deviations from Plan

None in behaviour. Z-order contingency (WindowLevel Normal while the dialog is open) is documented as a comment at the start site only: 37-04 parented the dialog via `set_parent(frame)`, so the unparented last resort did not apply.

## Known Stubs

None. `BrowseFolder` / `RefreshLists` have no widget yet (plan 06 by design).

## Threat Flags

None. T-37-14..18 mitigations implemented as planned (filter names still joined through `filter_target`; dirty rules gate the pick; single save; pick applied only via `poll()` on the UI thread; existence read only in `refresh_filters`).

## Human verification

- In game: Browse (once plan 06 lands) shows the folder dialog above the always-on-top overlay; the overlay does not hide while it is open or in the half second after it closes; the hotkey does nothing while it is open. If the dialog is behind the overlay, apply the WindowLevel contingency (Normal at start, AlwaysOnTop when `poll()` returns).
- Pick a folder with unapplied rule edits: the Switch filter? popup names the folder; Stay leaves everything; Switch and discard moves and survives a restart.
- Point at a missing folder: status reads `Game folder not found: <path>` and a click opens Config.

## For the Release Review

- CLAUDE.md: `ff-settings.json` row (new `game_folder` key, written on pick), overlay Config notes (pick behaviour, Switch filter? popup also used for folder picks, missing-folder status, refresh action), and the status-line priority list (missing folder after load failure, before sync running).
- CHANGELOG (GAME-02..05): picked game folder applies at once and is remembered; missing folder is reported; refresh re-lists filters and sounds.

## Self-Check: PASSED

a44cdcc exists; modified files present.
