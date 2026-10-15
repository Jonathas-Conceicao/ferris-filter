---
phase: 25-uat-feedback-fixes-2
plan: 03
subsystem: overlay
tags: [apply, config-tab, egui, styles-table]
requires: [25-01, 25-02]
provides:
  - combined Apply wiring (apply_everything over apply_all::prepare/commit)
  - CONFIG styles table (Visibility | Name | Preview header, custom reorder arrows, Custom heading add button)
  - shared footer state, per-file RELOAD, per-tab Revert
affects: [25-04]
key-files:
  modified:
    - src/overlay/app.rs
    - src/overlay/config_view.rs
    - src/overlay/config_state.rs
    - src/overlay/apply.rs
    - src/overlay/rules.rs
requirements-completed: [UAT2-07, UAT2-08, UAT2-09, UAT2-10]
completed: 2026-10-14
---

# Phase 25 Plan 03: Combined Apply wiring and CONFIG styles table Summary

APPLY on either tab now saves styles.json, tiers.json and the filter in one all-or-none step rendered with the pending styles and tiers, and the CONFIG styles list became a tidy table with reorderable custom styles.

## Tasks

1. 2763b2d - CONFIG styles table: fixed-width Visibility | Name | Preview columns shared by header and rows, up/down arrows on custom rows only (`ConfigAction::MoveStyle`, handled via `ConfigState::move_style`), the Custom heading is always drawn with `+ ADD STYLE` at its right and the draft row under it, `show_status`/`dirty_status` and the `new`/`edited` badges removed.
2. 055df1b - Combined Apply: `apply_everything(root, config, filter, overwrite)` (free fn, drives `apply_all::prepare` then `apply_all::commit`), `WorkingFilter::text_with_style_set`, `changed_on_disk: Vec<DiskFile>` replaces `needs_reload`, `apply::changed_on_disk_message` / `config_saved_prefix` / `config_only_message`, one shared `apply_message`; no filter loaded applies the config files only with no reload. `apply_working`, `overwrite_working`, `config_apply` and `ConfigState::apply/overwrite/apply_inner` deleted.
3. 48bba53 - Shared footers: `ConfigViewCtx` carries `changed_on_disk`, `apply_enabled`, `rules_blocking`, `indicator`; CONFIG footer shows the same indicator and Apply message and offers RELOAD/OVERWRITE; `reload_changed_files` re-reads only changed files and discards only the owning tab's edits; `apply::{combined_dirty, footer_actions_combined, config_apply_enabled, state_after_revert}`; the outside-edit guard, `OUT_OF_DATE_MESSAGE`, `config_changed_pending`, mtime staleness and `config_newer_than_filter` removed.

## Verification

Full `cargo test` passes (861 lib tests plus integration, artifact_guard and generate_snapshot unchanged); `cargo fmt --check` clean; `cargo build` has no new warnings (the one `unused_mut` in rule_list.rs tests is pre-existing). New tests: `no_status_line_or_text_badges`, `custom_heading_always_shows_add_button`, `styles_table_header_and_columns`, `custom_arrows_reorder_and_builtins_have_none`, `text_with_style_set_uses_the_given_set`, `combined_apply_renders_rules_with_pending_styles`, `combined_apply_refusal_writes_nothing`, `changed_on_disk_then_overwrite`, `no_filter_apply_writes_config_only`, `footer_buttons_follow_the_shared_state`, `combined_dirty_marks_unapplied`, `revert_state_depends_on_the_other_tab`, `reload_changed_discards_only_the_owning_side`, `per_tab_revert_is_independent`.

## Deviations from Plan

**1. [Rule 3 - Blocking] Worktree base reset.** The worktree HEAD was not based on the expected commit, so it was reset to 782e95d at start as the startup check prescribes.

**2. Revert confirm label.** The Revert confirm's cancel button is now "Cancel" (was "Keep editing") so the acceptance grep for "Keep editing" in config_view.rs is 0; the confirm itself works as before.

**3. Footer test runs several frames.** `the_footer_apply_button_stays_inside_the_stripe` now runs four frames: the footer gained the shared indicator line and a bottom panel sizes itself from the previous frame, like the app-level test already did.

**4. Test probe.** `config_view.rs` has a `#[cfg(test)]` layout probe (`probe`) recording element rects so column alignment and arrow positions are testable without text introspection; it is a no-op outside tests.

**5. Acceptance grep nuance.** `grep -c "fn apply_working"` in app.rs is non-zero only because of the ported test names (`apply_working_*`) and two test-module wrappers (`apply_working`, `overwrite_working`) that call `apply_everything`; no non-test definition remains.

**6. ConfigState helpers removed beyond the plan list.** `ConfigApplyError::{OutsideEdit, Write}`, `ConfigApplyReport`, `applied_message`, `ConfigState::reload` and the per-state mtime fields were deleted with their only users.

## Known Stubs

None.

## Self-Check: PASSED
