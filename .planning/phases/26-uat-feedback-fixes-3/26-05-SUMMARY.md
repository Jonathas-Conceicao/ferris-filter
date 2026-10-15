---
phase: 26-uat-feedback-fixes-3
plan: 05
subsystem: overlay
tags: [overlay, styles, ids, tags, uat3-02, uat3-03]
requires: [26-01, 26-03]
provides:
  - id-aware StyleState and canonical tags on Apply (rules.rs)
  - WorkingFilter::style_set accessor
  - ConfigState::rename_style, style_id, ids_unpersisted
affects: [26-07]
key-files:
  modified:
    - src/overlay/rules.rs
    - src/overlay/config_state.rs
    - src/overlay/apply_all.rs
decisions:
  - "style_rewrite_pending compares with retag=false, so a pure tag migration neither marks the filter unapplied nor triggers the quit confirm; the migration lands with the next Apply."
  - "follow_disk sets ids_unpersisted from the newly read file (replaces rather than ORs), since the disk file becomes the new baseline."
  - "rename_style trims the new name like add_style."
metrics:
  tasks: 2
  completed: 2026-10-14
requirements: [UAT3-02, UAT3-03]
---

# Phase 26 Plan 05: Id-aware overlay models Summary

Rule tags resolve through style ids, Apply writes canonical `<id> <name>` tags, load-assigned ids reach disk silently on the next Apply, and CONFIG can rename a custom style as a single revertible edit.

## Commits
- 11df4f3: rules.rs. StyleState uses `resolve_tag` (Known carries the current name, so renames keep rules linked and `style_users` counts follow). ApplyStyle writes `tag_for`. The Apply text path retags via `rewrite_styled_with(.., true)`. `style_rewrite_pending` uses `retag = false`. `style_set()` added.
- 6eff5a9: config_state.rs/apply_all.rs. `ids_unpersisted` (set in load/follow_disk via `assign_missing_ids`, joins the styles write in `prepare_write_inner`, cleared in `adopt`, never counted as an edit). `dirty_count`/`styles_changed` match by id (name fallback), with a rename counted once. `rename_style`, `style_id`.

## Deviations from Plan
- ApplyStyle on an id-less style (only reachable from a hand-built set) falls back to writing the bare name instead of refusing; loaded sets always have ids. Minor.
- `badges(name)` left unchanged: it already reads the current style by name, so id keying was not needed.
- No production change in app.rs; no test expectations in app.rs needed changing.
- Existing rules.rs tests updated to expect `builtin:S S` / `builtin:hidden hidden` tags.

## Verification
Full `cargo test` passes (923 lib tests plus integration), `cargo fmt --check` clean, fixtures untouched. body.rs / rule_list.rs not edited. A pre-existing `unused mut` warning exists in rule_list.rs (not part of this plan).

## Known Stubs
None.

## Self-Check: PASSED
