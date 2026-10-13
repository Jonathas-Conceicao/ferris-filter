---
phase: 18-filter-as-store-economy-section
plan: 06
subsystem: overlay
tags: [overlay, egui, filter-as-store]
requires: [18-01, 18-04, 18-05]
provides:
  - "overlay::apply: ApplyOutcome::Unchanged, is_changed_on_disk, FilterState, FooterActions, footer_actions, filter_state_message"
  - "OverlayApp rooted at paths::app_root with a LoadedFilter; APPLY / ADD ECONOMY SECTION / RELOAD / SYNC ECONOMY"
affects: [18-07, 19]
key-files:
  modified: [src/overlay/app.rs, src/overlay/apply.rs, src/overlay/mod.rs]
  deleted: [src/overlay/store.rs, tests/rule_editor.rs]
key-decisions:
  - "Footer button rules and status text live as pure functions in overlay::apply; app.rs only draws them"
  - "Tasks 1 and 2 committed together: Task 1 alone leaves app.rs referencing the deleted store and cannot compile"
requirements-completed: []
duration: 30min
completed: 2026-10-12
---

# Phase 18 Plan 06: Overlay on the filter-as-store API Summary

The overlay now loads the game-folder `dynamic.filter` (absent means empty) and applies through `dynamic::economy_body` + `dynamic::apply_file`, the same path as the CLI. Everything is rooted at `paths::app_root()`, including SYNC ECONOMY. The JSON store (`overlay::store`) and `tests/rule_editor.rs` are deleted. The rule list is replaced by a status block and the notice "Rule editing returns in the next update."

## Behaviour
- APPLY is enabled only when the filter has an economy section and no reload is pending. ADD ECONOMY SECTION is a separate button shown for an absent or section-less filter; APPLY never creates a section.
- A changed-on-disk refusal sets `needs_reload`: only RELOAD (and SYNC) remain; RELOAD re-reads the file and resets state.
- A v0.4 filter is announced as "will be converted on the next APPLY (a .ffbak backup is kept)"; nothing is written on open.
- Root and filter-load errors are rendered in the stripe, not panics. No `unwrap`/`expect` added.
- `edit.rs`, `rule_editor.rs`, `view_model.rs` untouched and still compiled/tested.

## Verification
- `cargo build --bins` clean, no warnings. Full `cargo test`: 518 lib tests (147 in `overlay::`) plus all integration suites pass, 0 failures. Fixtures and snapshots unchanged.
- Acceptance greps: no `store::`/`store_path`/`compile_store`/`write_filter`/`WorkingRules`/`baseline`/`REVERT` in app.rs code; no `current_dir` outside `paths.rs`.

## Deviations from Plan
- Tasks 1 and 2 share one commit (db235f3) because Task 1 alone does not compile; a follow-up comment repair is a882028.
- The pre-existing test `the_apply_button_stays_on_screen_...` was kept (it exercises the bottom-panel arrangement, not rule rows); the row-header tests went with `row_header_text`.

## Human verification (not run; no display available)
- Double-click `ff.exe` from Explorer: stripe appears, filter path and status shown, no panic.
- Against a scratch copy (USERPROFILE / FERRISFILTER_HOME pointed at a temp folder, never the real PoE2 folder): ADD ECONOMY SECTION, APPLY, edit the file externally then APPLY (expect refusal and RELOAD), open a v0.4 filter (expect conversion notice and `.ffbak` after APPLY).

## Requirements
STORE-01/04/05, EDIT-09/10, ECON-04 are delivered in the overlay, but 18-07 (legacy API removal, docs) is outstanding and the GUI paths are unverified manually, so none are marked complete.

## Known Stubs
None (the Phase 19 notice is intentional).

## Self-Check: PASSED
