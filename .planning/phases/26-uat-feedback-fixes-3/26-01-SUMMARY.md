---
phase: 26-uat-feedback-fixes-3
plan: 01
subsystem: models/styles
tags: [styles, ids, tags, uat3-02]
requires: []
provides:
  - Style.id and StyleSet id API (assign_missing_ids, new_custom_id, id_of, name_of_id, resolve_tag, tag_for, rename_custom)
  - styled::rewrite_styled_with(doc, styles, retag)
  - committed ids in styles.json
affects: [26-05, 26-07]
key-files:
  modified:
    - src/models/config.rs
    - src/loader.rs
    - src/styled.rs
    - src/emitter.rs
    - styles.json
decisions:
  - "Load-time ids for customs are seeded from an FNV-1a hash of the style name, so an id-less file yields identical ids on every load; new customs use process entropy."
  - "assign_missing_ids_with is cfg(test) only (collision-test hook)."
metrics:
  tasks: 3
  completed: 2026-10-14
requirements: [UAT3-02]
---

# Phase 26 Plan 01: Stable style ids Summary

Every style now carries an id (first key on disk): built-ins are `builtin:<name>`, customs a 6-8 char base36 id. Tags `# ff:style <id> <name>` resolve by id with a legacy name fallback, and a retagging rewrite can migrate old tags.

## Commits
- 892c4f9: Style.id, id generation/assignment, validation (DuplicateId/BuiltinId/ReservedId/InvalidId), rename_custom, loader assigns before validate
- d59f494: resolve_tag/tag_for/TagMatch, rewrite_styled_with (retag), id-aware drop_unknown_style_tags, StyleReport.retagged
- (third) styles.json ids (builtin:* for nine built-ins; Recycle q8m3vz, kite t5hw2k, exceptional n7c4rj) plus committed_styles_have_fixed_ids test

## Deviations from Plan

**1. [Rule 1 - Bug] Inline legacy `styles` comparison in `loader::check_legacy_styles`**
- Found during: full test run. An id-less inline style differed from the id-bearing file style, rejecting configs that previously matched.
- Fix: the inline copy takes the file style's id before comparing.
- Files: src/loader.rs (committed with Task 2).

**2. Test fix:** `styleset_keeps_file_order_with_extra_user_style` cloned a style with its builtin id; the test now clears the id.

Note: `set_meta` writes the canonical `# ff:style <value>` spelling, so a retagged line becomes `# ff:style ...` (a legacy `#ff:style` line is respelled only when retagged).

## Verification
Full `cargo test` passes (887 lib tests plus integration, including artifact_guard and generate_snapshot, no fixture regenerated); `cargo fmt --check` clean. No Cargo.toml change.

## Known Stubs
None.

## Self-Check: PASSED
