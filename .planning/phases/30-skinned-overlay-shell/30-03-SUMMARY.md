---
phase: 30-skinned-overlay-shell
plan: 03
subsystem: ui
tags: [egui, overlay, popup, modal, skin, screenshot]
requires: [30-01, 30-02]
provides:
  - shell::{Popup, PopupFlags, active_popup, PopupData, PopupSpec, popup_spec} (tested popup model, stacking and copy)
  - One skinned ff_modal per frame in OverlayApp with dispatch_popup routing to the unchanged v0.5 handlers
  - Debug-only `ff --screenshot ... --popup <name>` for all seven popups
affects: [30-04, 30-05]
tech-stack:
  added: []
  patterns: [pure spec fns + one draw site, deferred dispatch to existing handlers, flag-per-popup state kept]
key-files:
  created: []
  modified: [src/overlay/shell.rs, src/overlay/app.rs, src/overlay/rule_list.rs, src/overlay/config_view.rs, src/overlay/screenshot.rs, src/skin/popup.rs]
decisions:
  - "Existing flags (quit_confirm, pending_switch, list.confirm_delete, list.confirm_economy_remove, config_ui.delete_armed, config_ui.revert_armed) remain the 'popup open' state; only changed_popup is new, because changed_on_disk stays non-empty after Keep editing"
  - "The 30-02 interim Discard-config modal was folded into the popup system (no duplicate)"
  - "Popup box widened from 520 to 580pt so the three-button Quit row fits one line"
metrics:
  tasks: 2
  completed: 2026-10-16
---

# Phase 30 Plan 03: Popups Summary

Every confirmation and choice in the overlay is now one skinned modal at a time (Quit, then Changed-on-disk, then Switch filter, Delete rule, Delete style, Remove economy, Discard config), with Escape or a backdrop click picking the safe last button. All inline confirm frames and the plain `egui::Modal` quit confirm are gone.

## What was built
- **shell.rs:** `Popup`, `PopupFlags`, `active_popup` (stacking), `PopupData` and `popup_spec` (titles, bodies, button labels/order/danger/tooltips from the UI contract), caption constants. Table tests cover stacking, all seven specs and "last button is safe and never danger".
- **rule_list.rs:** Delete and economy Remove only set their flags; inline frames removed; `EconomyFooterAction::Remove` dropped (the popup calls `stage_remove_economy`).
- **config_view.rs:** inline delete-style frame, inline switch-filter frame and the `pending_switch` ctx field removed; `delete_prompt` / `revert_prompt` stay as pub copy helpers; unused `accent_button` and a stale `ctx` parameter removed.
- **app.rs:** `changed_popup` flag (set when a refusal stores `changed_on_disk` and when the bottom-bar Overwrite slot is clicked); `popup_flags`, `popup_spec_for`, `dispatch_popup`, `run_row_actions` (extracted from the deferred block); exactly one `ff_modal(` call per frame. Dispatch uses the same handlers as v0.5: Quit maps to `QuitChoice` and the `quit_step` body is verbatim (`quit_next_frame` deferral, `quit_after_apply` wait), Reload/Overwrite go through `reload_pressed` / `run_apply(true)` on RULES and `ConfigAction::Reload/Overwrite` on CONFIG, switch via `ConfirmSwitch/CancelSwitch`, delete via `RowAction::Delete`, style delete via `ConfigAction::DeleteStyle`, discard via `ConfigAction::Revert`. Flags are never cleared by hiding the stripe.
- **screenshot.rs / app.rs (debug only):** `--popup changed|switch|quit|delete-rule|delete-style|remove-economy|discard-config` with harmless sample data; unknown names are a parse error; choices are never dispatched.
- **Tests:** four new app tests (`delete_popup_choice_deletes_the_rule_and_keep_does_not`, `changed_on_disk_popup_keep_editing_closes_but_overwrite_slot_reopens`, `quit_popup_cancel_keeps_running_and_discard_defers_one_frame`, `remaining_popups_close_on_the_safe_choice_without_acting`).

## Commits
- 2f5781f feat(30-03): popup model and removal of inline confirms
- 40b4b9d feat(30-03): one skinned modal per frame with v0.5 dispatch, --popup screenshots

## Verification
- Full `cargo test` green (1129 unit tests plus integration suites); `tests/fixtures` untouched; `cargo fmt --all --check` clean.
- Clippy warnings: 126 (was 127 after 30-02; baseline 128).
- `grep ff_modal( app.rs` = 1, `egui::Modal::new` = 0, `ViewportCommand::Close` in src/overlay = 1, `quit_next_frame = true` and `quit_after_apply = true` each present.
- Screenshots from a temp home (`%TEMP%\ff-shot-30`, popup-quit/changed/delete-rule.png) viewed:
  - **quit:** window dimmed, game box centred with "Unapplied changes" in small caps over the gold rule, body "Apply them before quitting?", buttons Apply & quit, Quit without applying (red), Cancel. First render wrapped Cancel to a second line; after widening the box to 580pt it was re-rendered and viewed: all three buttons sit on one line.
  - **changed:** "File changed on disk", body names dynamic.filter, buttons Reload, Overwrite (red), Keep editing; the bottom bar behind shows the Reload and red Overwrite slots.
  - **delete-rule:** "Delete this rule?", body "Rule 1: Show", buttons Delete (red) and Keep over the expanded row.
  PNGs are not committed.

## Deviations from Plan
**1. [Rule 3 - Blocking] `--popup delete-rule` also expands the row.** The list drops a pending delete whose row is not expanded (an existing reset site, kept), so the screenshot sets `list.expanded` as well. Real use is unaffected: Delete is only clickable on the expanded row.

**2. Task 1 test updates were minimal.** No rule_list test clicked "Yes, delete"/"Yes, remove", so none needed changing; the config_view switch-guard test only lost its `pending_switch` line (renamed `open_drafts_render`).

**3. Tooling note.** A few intermediate cargo builds ran without `CARGO_TARGET_DIR` (each shell call starts fresh) and went into the git-ignored repo `target/`; verification and screenshots used the temp target.

## Known Stubs
None.

## Known Issues (not fixed)
- CONFIG Styles table "Visibility"/"Name" caption overlap (from 30-02) is still open.

## Threat Flags
None. T-30-10 (Escape/backdrop map to Keep editing, Overwrite is a red explicit choice calling the unchanged guarded `run_apply(true)`), T-30-11 (`quit_step` untouched, Escape = Cancel), T-30-12 (destructive actions only on button 0) and T-30-14 (screenshot popups debug-only, no dispatch) are implemented as planned. No keystroke path was added.

## Self-Check: PASSED
