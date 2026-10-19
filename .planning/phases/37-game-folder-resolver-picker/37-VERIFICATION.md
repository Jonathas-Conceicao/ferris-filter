---
phase: 37-game-folder-resolver-picker
verified: 2026-10-18T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Click Browse on the Config tab with the overlay always-on-top over the game"
    expected: "The native folder dialog appears above the overlay, is usable, and the picked folder is shown and restored on next launch"
    why_human: "Native dialog z-order against an always-on-top window cannot be checked from code"
  - test: "Run on a machine whose Documents is redirected to OneDrive or localized (for example Documentos)"
    expected: "With no FERRISFILTER_GAME_DIR and no saved folder, the Config tab shows Documents\\My Games\\Path of Exile 2 under the redirected location"
    why_human: "Real OS known-folder resolution (dirs::document_dir) needs a real redirected profile"
  - test: "Hover and press the refresh button beside the filter selector"
    expected: "Normal, hover and pressed game art (round arrow on a square) show correctly"
    why_human: "Visual check"
  - test: "Drop a new .filter and a new .mp3 into the game folder, press refresh with unapplied rule edits"
    expected: "Both appear in the filter dropdown and the Alert sound dropdown; nothing is written; the rule edits survive"
    why_human: "End-to-end with the real game folder; the dropdown pick of the active entry decision from the fix round also needs an eye"
  - test: "Reload in game after Apply and ff-cli apply defaults, after the review-fix behaviour changes"
    expected: "Fresh install opens the folder's filter; ff-cli apply targets the filter saved in ff"
    why_human: "In-game and cross-binary behaviour; the review fixes were flagged as needing human verification"
---

# Phase 37: Game Folder Resolver and Picker Verification Report

**Phase Goal:** Users find their Path of Exile 2 folder automatically, or pick it themselves, on any Windows setup.
**Status:** human_needed (all automated checks pass)
**Re-verification:** No

## Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Default resolves to OS Documents + `My Games\Path of Exile 2`, no hard-coded path | VERIFIED | `src/locations.rs` `game_dir_from` / `game_dir` use `dirs::document_dir` plus `GAME_SUBDIRS`. Order is env, then saved absolute path, then Documents. No `OneDrive` or `Documentos` string in `src` or `tests` outside comments. |
| 2 | Config tab shows the folder with Browse; the pick is restored on the next launch | VERIFIED | `config_view.rs` Browse row emits `ConfigAction::BrowseFolder`; `app.rs` `apply_folder` and `adopt_settings` save `game_folder` through `settings::save`. `settings.rs` loads and validates `game_folder` (absolute only, with a warning otherwise). The `game_folder_*` settings tests pass. |
| 3 | Changing the folder with unapplied rule edits opens Switch filter? first | VERIFIED | `on_folder_picked` sets `pending_folder` when `working.is_dirty()`. `Popup::SwitchFilter` is shown, and `ConfirmSwitch` or `CancelSwitch` handle `pending_folder`. |
| 4 | Missing folder: status line says so and points to the picker | VERIFIED | `game_folder_missing_text` returns `Game folder not found: <path>`. `shell.rs` status priority sets `open_tab: Some(ShellTab::Config)`, so a click opens the picker tab. The wording itself does not name Browse; the click-through is the pointer, which CONTEXT specifies. |
| 5 | New files plus the refresh button (game art) list in the filter and Alert sound dropdowns, with no restart, write or lost edit | VERIFIED | `ConfigAction::RefreshLists` calls `refresh_filters()` (a `read_dir` listing only) and `sound_files::refresh(ctx)`. It does not touch `working`, so edits are kept. `ff_icon_button` uses the `refresh_normal/hover/down.png` assets, registered in `manifest.toml` and `skin/textures.rs`. The refresh stays live when the folder is missing. |

**Score:** 5/5

## Requirements Coverage

| Requirement | Plans | Status |
|---|---|---|
| GAME-01 | 37-01, 37-03, 37-07 | SATISFIED |
| GAME-02 | 37-01, 37-03, 37-04, 37-05, 37-06 | SATISFIED |
| GAME-03 | 37-05 | SATISFIED |
| GAME-04 | 37-05, 37-06 | SATISFIED |
| GAME-05 | 37-05, 37-06 | SATISFIED |
| GAME-06 | 37-02, 37-06 | SATISFIED |

All six IDs appear in plan frontmatter and in REQUIREMENTS.md (marked Complete). There are no orphaned requirements.

## Behavioral Spot-Checks

Targeted `cargo test --lib` (game_folder, folder, refresh, switch, settings::, locations::, sound_files) ran with `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` set to a temp dir. Result: 139 passed, 0 failed. The orchestrator's full suite also passed (lib 1502 after fixes).

## Anti-Patterns

No TBD/FIXME/XXX was introduced in the files I inspected. 37-REVIEW.md is clean.

## Notes

- The review-fix round changed behaviour. `Settings::auto_filter` is never serialized, and a click on the active but unselected filter entry now saves it. CLI `apply` follows the saved filter. These are covered by unit tests (`the_automatic_filter_is_read_but_never_saved` and others), but the fixer flagged them for human review.
- I did not run the screenshot mode. The Config row layout is covered only by unit tests plus the human checks above.

## Gaps Summary

No gaps. The remaining items are manual-only and are deferred by the author to the end of the milestone.

_Verified: 2026-10-18_
_Verifier: Claude (gsd-verifier)_
