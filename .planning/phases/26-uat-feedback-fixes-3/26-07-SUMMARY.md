---
phase: 26-uat-feedback-fixes-3
plan: 07
subsystem: overlay
tags: [overlay, config, styles, rename, uat3-01, uat3-02, uat3-03]
requires: [26-03, 26-05]
provides:
  - per-group styles table with Order column
  - ConfigAction::RenameStyle and inline rename field for custom styles
  - sync_working_styles (RULES tab follows the CONFIG tab's in-memory style set)
key-files:
  modified:
    - src/overlay/config_view.rs
    - src/overlay/app.rs
decisions:
  - "The custom header is omitted when there are no custom styles (Custom heading and + ADD STYLE still drawn)."
  - "A custom row's click-to-expand target starts at the Preview column so it never overlaps the Name text field."
  - "Rename drafts live in ConfigUi.rename_drafts keyed by style id and are dropped as soon as they equal the name."
metrics:
  tasks: 2
  completed: 2026-10-14
requirements: [UAT3-01, UAT3-02, UAT3-03]
---

# Phase 26 Plan 07: Styles table, rename and live style set Summary

CONFIG Styles now reads heading, header, rows per group (Built-in: Visibility | Name | Preview; Custom: plus Order holding the arrows), custom names are inline text fields, and the RULES tab judges tags and offers styles from the unapplied CONFIG set.

## Commits
- 3f1ea04: config_view.rs table per group, `COL_ORDER`, `rename_field`, `rename_hint`, `ConfigAction::RenameStyle`; minimal app.rs `RenameStyle` arm (in-memory edit, clears draft, error to CONFIG status).
- Task 2 commit: `sync_working_styles` called once per frame after `poll_styles` when config is Ok and a working filter exists; app tests.

## Deviations from Plan
- Task 1 included the full RenameStyle handler (not just a stub) since it was a few lines; Task 2 added the sync and tests.
- Rename field commits on `lost_focus` and may re-offer the same action on the next frame if the model has not yet applied it; the app applies it after the first, so the repeat is a no-op (the view test tolerates the repeat).

## Known limitation
- A rule carrying a legacy name-only tag (`# ff:style kite`, from before ids) cannot follow a rename made before the next Apply migrates it to `<id> name`; it shows as unknown. Tags written by this version are canonical. Not addressed (would need a rules.rs change outside this plan).

## Verification
Full `cargo test` passes (949 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched. STATE.md/ROADMAP.md not modified; body.rs, rule_list.rs, conditions.rs not edited.

## Known Stubs
None.

## Self-Check: PASSED
