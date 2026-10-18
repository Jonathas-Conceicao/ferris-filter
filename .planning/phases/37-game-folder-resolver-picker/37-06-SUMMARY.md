---
phase: 37-game-folder-resolver-picker
plan: 06
subsystem: overlay
tags: [rust, egui, config-tab, game-folder, docs]
requires: [37-02, 37-03, 37-05, 37-07]
provides:
  - "Config tab Filter section: Game folder row (Browse) and Filter row (dropdown + refresh)"
  - "ConfigViewCtx.game_dir_missing / game_dir_pinned / default_game_dir / folder_dialog_open"
  - "CLAUDE.md updated for the phase 37 behaviour"
key-files:
  modified: [src/overlay/config_view.rs, src/overlay/app.rs, CLAUDE.md]
requirements-completed: [GAME-02, GAME-04, GAME-05, GAME-06]
completed: 2026-10-18
---

# Phase 37 Plan 06: Config tab rows and docs Summary

The Config tab's Filter section now has a labelled Game folder row (path, Browse) and a Filter row (dropdown plus the game's refresh art button), wired to the 37-05 actions; CLAUDE.md matches the new behaviour.

## Commits

- 0158364: feat(37-06): Game folder and Filter rows with Browse and refresh on the Config tab
- 2b811a4: fix(37-06): Browse width fits its caption so the Game folder row does not wrap
- af83195: docs(37-06): CLAUDE.md game folder resolver, picker and refresh

## What was built

- `show_filter`: the GAME FOLDER FILTER caption and the "Default: dynamic.filter" hover are gone. Row 1 ("Game folder"): truncated path (full path on hover, WARN when missing, "(no Documents folder)" when none), Browse (disabled when pinned or the dialog is open; tooltip "Set by FERRISFILTER_GAME_DIR" or "Default: <path>"). Row 2 ("Filter"): the unchanged dropdown (hover is the full filter path), 10pt gap, `ff_icon_button(&skin.refresh, "Refresh", enabled)` (headless: a "Refresh" text button), tooltip "Refresh the file lists". Refresh and the dropdown are off when the folder is missing; Browse stays on.
- `app.rs` fills the four new ctx fields; the two `#[allow(dead_code)]` markers from 37-05 were removed.
- Tests: `browse_and_refresh_push_their_actions`, `browse_is_off_when_pinned_or_while_the_dialog_is_open`, `a_missing_folder_disables_refresh_but_not_browse` (click scan over the rows, headless), `the_removed_captions_stay_removed`.

## Verification

- `cargo test --lib -- overlay::config_view`: 59 passed. `cargo test --lib -- overlay::app::`: 107 passed (the three baseline names ran and passed here). All with FERRISFILTER_HOME / FERRISFILTER_GAME_DIR under a temp dir.
- Screenshots (debug `ff --screenshot`, both env vars under `C:\Users\jonat\AppData\Local\Temp\ff37s`, nothing in the repo), viewed:
  1. normal, 3 filters: `...\ff37s\state1.png`
  2. long nested path: `...\state2.png` (path cut with an ellipsis, Browse never overlaps)
  3. missing folder: `...\state3-config.png` (path amber, dropdown "(game folder not found)" and refresh greyed, Browse live, status "Game folder not found: ...") and `...\state3-rules.png` (same status on the Rules tab)
  4. `--popup switch`: `...\state4.png`
  5. folder with no `.filter`: `...\state5.png` (dropdown keeps `dynamic.filter`, status "The filter does not exist yet")
  Pixel checks: refresh uses the game art (not the R fallback), square button, 10pt gap after the dropdown (dropdown ends x=520, button starts 530), button right edge lines up with Browse's right edge (563); Browse sits on the same line as the label.

## Deviations from Plan

- [Rule 1] First layout wrapped Browse to a second line: the button is `max(caption + 44, min)` wide, so 96 was too narrow. `BROWSE_WIDTH` is now 120.
- `cargo fmt -- <file>` reformatted `tests/isolation_guard.rs`; reverted, not committed.
- ROADMAP.md progress and requirements are left to the orchestrator note below.

## Decisions

- Rows use `ff_field_row` (alternating ROW_EVEN/ROW_ODD strips inside the section frame) rather than a bespoke layout.
- CLAUDE.md: kept the author's own in-game folder line with a short note; added the Config-tab sentences into the existing Filter paragraph, not a new paragraph.

## Human verification

- Folder dialog opens above the always-on-top overlay in game; overlay stays visible while open and after it closes; hotkey does nothing meanwhile.
- Pick a folder: selection follows same name, then first `.filter`; relaunch restores it. With rule edits pending, Switch filter? asks first, Stay keeps everything.
- Drop a `.filter` and an `.mp3` into the game folder, press refresh: both appear, no write, no edit lost.
- Refresh art hover and pressed states, at 100% and 150% DPI.
- On this OneDrive-redirected machine the default resolves to the same folder as before.

## For the Release Review

- CHANGELOG (GAME-01..06): game folder resolved from `FERRISFILTER_GAME_DIR`, then the picked folder, then the OS Documents folder (no more USERPROFILE/OneDrive\Documentos path); Config tab Game folder row with Browse; refresh button re-lists filters and sounds; missing folder reported on the status line (click opens Config); CLI apply/install honour the saved folder.
- README: new dependency `rfd` (MIT, Windows only); new committed GGG art `assets/poe2/common/refresh_*.png` (Credits for `assets/poe2/` already cover it).
- Dev/test: screenshot mode now needs FERRISFILTER_HOME and FERRISFILTER_GAME_DIR under temp; tests pin FERRISFILTER_GAME_DIR.

## Self-Check: PASSED

Commits 0158364, 2b811a4, af83195 exist; modified files present.
