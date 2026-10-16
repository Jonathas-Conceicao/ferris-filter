---
phase: 30-skinned-overlay-shell
plan: 02
subsystem: ui
tags: [egui, overlay, shell, skin, status-line, screenshot]
requires: [30-01]
provides:
  - Skinned shell composition in OverlayApp::ui (docked 660pt window, header, four tabs, content, bottom bar)
  - shell.rs pure decisions (ShellTab, tab_markers, status_for, event_from, bar_state) plus header_art, last_sync_text, Economy and About tabs
  - One prioritised status line replacing the 96pt outcome area and every pinned footer
  - Debug-only `ff --screenshot` mode
affects: [30-03, 30-04, 30-05]
tech-stack:
  added: []
  patterns: [pure decision fns + thin egui draw, deferred actions kept, debug-only cfg for test tooling]
key-files:
  created: [src/overlay/shell.rs, src/overlay/screenshot.rs]
  modified: [src/overlay/app.rs, src/overlay/mod.rs, src/overlay/config_view.rs, src/overlay/rule_list.rs, src/overlay/tray.rs, src/overlay/apply.rs, src/overlay/economy.rs, src/sync/prices.rs]
decisions:
  - "Config tab keeps ConfigViewCtx's old footer fields (apply_enabled etc.); a new skin: Option<&Skin> field lets section headings use ff_section while headless tests pass None"
  - "Config Revert opens an interim skinned ff_modal Discard confirm from config_ui.revert_armed so the action stays reachable until the popup plan; Plan 03 can adopt or replace it"
  - "Screenshot mode only forces visibility::advance's result to ShownFocused; the hotkey/tray polls are untouched because nothing is registered or built in that mode"
metrics:
  tasks: 3
  completed: 2026-10-16
---

# Phase 30 Plan 02: Skinned overlay shell Summary

`ff` now opens as the docked, skinned 660pt window: Ferris background, game-art header, four stone tabs (Rules, Config, Economy, About), the v0.5 RULES and CONFIG editors inside it, and one bottom bar (Revert, Reload, Apply) with a single prioritised status line. Hotkey, tray, single instance, Quit and the in-game reload logic are untouched.

## What was built
- **shell.rs (pure):** `ShellTab`, `tab_markers` (` *` only on Rules/Config), `status_for` (the 9-row priority table), `event_from`, `bar_state` (Overwrite replaces Apply while a file changed on disk, disabled-Apply tooltip chain), message consts. 20 table-driven tests.
- **shell.rs (draw):** `header_art` (the one swap point for custom header art), `last_sync_text` (local `YYYY-MM-DD HH:MM` from the prices.json mtime, `Never` when absent), `economy_tab` (League, Last sync, Sync economy), `about_tab` (identity, credits, pinned disclaimer).
- **app.rs:** `dock_left` on the first cached HWND, immediately before every `win::reveal`, and again once when `native_pixels_per_point` changes. Layered paint (background, frame, header with plate-hover filter path and "Hide (Ctrl+Shift+Z)" close, tabs with tooltips, clipped content, bottom bar). `bottom_bar`, `rules_content`, `config_body` (inside `ff_scroll_area`). Deleted: `Tab`, `tab_strip`, `OutcomeLine`/`outcome_lines`/`outcome_area`, the RULES footer panel, the status block, `rule_count_text`, and the five footer-geometry tests (replaced by `shell_bar_stays_inside_the_button_bar_with_a_long_list`).
- **rule_list.rs:** `search_row` (hint `Search N rules, classes, base types…`, Clear, `+ Add rule` with tooltip), 34pt skinned `economy_footer` (inline Remove confirm kept for Plan 03).
- **config_view.rs:** `show_footer`/`footer_buttons` and their tests removed; headings are `ff_section`; `show_sections` no longer owns a scroll area.
- **screenshot.rs (debug only):** arg parsing, `guard_env` (HOME, USERPROFILE and the derived game folder must canonicalise under temp; `..` refused), `stage_filter`, `save_png`.
- Mixed-case retirements: `DAMAGED_MARKERS_MESSAGE`, filter-state messages, economy success messages, economy row tooltip.

## Commits
- c2530b8 feat(30-02): shell pure pieces (tabs, status priority, bottom bar)
- 626ac02 feat(30-02): skinned shell composition in ui()
- 9f93c72 feat(30-02): debug-only screenshot mode for ff

## Verification
- Full `cargo test` green (1122 unit tests plus integration suites); `tests/fixtures` byte-identical; `cargo fmt --all --check` clean; release `ff` builds and its binary contains no screenshot strings.
- Clippy warning count: baseline 128, now 127.
- Single `ViewportCommand::Close` send site in `src/overlay` (the screenshot exit routes through it).
- Guard check: `FERRISFILTER_HOME=<temp> USERPROFILE=C:/Users/jonat ff --screenshot x.png` printed `ff: screenshot mode refuses to run: USERPROFILE (C:/Users/jonat) is not under the temp folder (\\?\C:\Users\jonat\AppData\Local\Temp)` and exited without a window.
- Screenshots rendered from a temp home (`%TEMP%\ff-shot-30`: rules, config, economy, about, and a long `--status`) and viewed: 660pt wide at full work-area height, header with FerrisFilter, four tabs, frame, Ferris faintly behind rows, bottom bar with three buttons, the status line cut with an ellipsis, nothing wider than the frame. Not committed (Plan 05 renders the final set).

## Deviations from Plan
**1. [Rule 1 - Bug] shell module was not registered in the Task 1 commit**
- **Found during:** Task 2 build. The `mod.rs` files are CRLF, so the Task 1 text replace for `pub mod rules;` silently missed; the Task 1 tests ran without the shell tests.
- **Fix:** registered `pub(crate) mod shell;` in the Task 2 commit; all shell tests pass there.
- **Commit:** 626ac02

**2. [Rule 2 - Missing functionality] Interim Discard-config confirm**
- Revert on CONFIG sets `revert_armed` per plan, but with the inline footer gone nothing would show the confirm until Plan 03. Added a small `ff_modal` ("Discard config changes?", Discard / Keep editing) in `ui()`.

**3. Broader sweep result.** No user-visible all-caps button names remain in `src/overlay` strings. `apply::APPLY_BLOCKED_TOOLTIP` still says `"grammar error"` (owned by Plan 04). Remaining all-caps tokens are one test assertion string and a `to_ascii_uppercase` check.

**4. Commits.** Task 2 went in as one commit rather than three sub-commits.

## Known Stubs
None. The screenshot `--popup` argument is parsed and stored but unused until Plan 03 renders popups (documented in `screenshot.rs`).

## Known Issues (not fixed)
- CONFIG Styles table: the "Visibility" column caption overlaps "Name" at the narrower content width (visible in the config screenshot); the table is restyled in a later phase.
- `layout.rs` is no longer used by production code (kept with its tests for Phase 35).

## Threat Flags
None. T-30-05 mitigated by `guard_env`/`stage_filter` and the cfg gate; T-30-06/08 unchanged code paths.

## Self-Check: PASSED
