---
phase: 14-rule-editor-apply
plan: 03
subsystem: overlay
tags: [rust, egui, immediate-mode-gui, filter-editor]

# Dependency graph
requires:
  - phase: 14-rule-editor-apply
    plan: 01
    provides: overlay::edit's per-condition set/clear functions (style/class/rarity/defence/sockets/quality/corrupted), available_styles, class_groups
  - phase: 14-rule-editor-apply
    plan: 02
    provides: the editable accordion (WorkingRules, RuleId-keyed expansion, autosave dirty flag) and the render_rule_body placeholder hook
provides:
  - "src/overlay/rule_editor.rs — render_rule_body(ui, rule, config, id): the full EDIT-03..08 condition editor UI (style, classes, rarity, defences, sockets, quality, corrupted), every control routed through overlay::edit"
  - "app.rs's accordion loop threads &mut Rule + &Config + RuleId into the expanded row's body, distinguishing immediate per-rule condition mutation from the deferred RowAction path used for row-structural changes (move/delete/add)"
affects: [14-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Uniform enable-toggle + value architecture for every condition (rarity/defence-group/sockets/quality/corrupted), including corrupted -- no bespoke tri-state widget (RESEARCH Pattern 2)"
    - "Config-sourced option lists only: styles from edit::available_styles, classes from edit::class_groups -- never a hardcoded literal in the presentation file (PITFALLS.md #10)"
    - "RuleId + per-control discriminator salts every ComboBox/CollapsingHeader this file creates; plain value widgets (checkbox/selectable_value/DragValue) rely on the ambient id-stack scope of the row's own CollapsingHeader and need no separate salt"
    - "Row-level condition edits mutate immediately via WorkingRules::rule_mut(id) inside the render pass (no array-structure hazard), while row reordering/delete/add stay on the pre-existing deferred RowAction collect-then-apply pattern (array-structure hazard)"

key-files:
  created:
    - src/overlay/rule_editor.rs
  modified:
    - src/overlay/app.rs
    - src/overlay/mod.rs

key-decisions:
  - "app.rs's row loop now collects RuleId list up front (row_ids: Vec<RuleId>) instead of iterating working.rows() directly, so working.rule_mut(id) can hand the expanded row's body a genuine &mut Rule without holding a conflicting immutable borrow of the whole WorkingRules array"
  - "A disjoint immutable borrow of self.config (config_ref) is held across the row loop alongside the pre-existing mutable borrow of self.rules (working) -- both are field-disjoint borrows of the same &mut self, the same pattern the file already used for self.config.as_mut() one statement earlier"
  - "Comparator/value pickers (rarity op, rarity value, corrupted value) use ui.selectable_value with plain-English labels (Exactly/At Least/At Most, Normal/Magic/Rare/Unique, False/True) rather than symbolic >=/<=/== labels, both for readability and to keep the plan's own anti-comparator-string grep meaningful against this file's actual widget labels, not just its logic"
  - "Sockets and quality are two near-identical but separately-written functions (render_sockets/render_quality) rather than one generic helper -- each is auditable on its own against the EDIT-08 off-switch inventory without an extra layer of indirection"

requirements-completed: [EDIT-03, EDIT-04, EDIT-05, EDIT-06, EDIT-07, EDIT-08]

# Metrics
duration: ~65min
completed: 2026-09-07
---

# Phase 14 Plan 03: The Condition Editors Summary

**The full EDIT-03..08 condition editing vocabulary — style dropdown, grouped class checkboxes, rarity comparator+value, a three-checkbox defence group, sockets/quality DragValues, and a toggle-reusing corrupted control — all wired into the accordion's expanded row body, every mutation routed through `overlay::edit`.**

## Performance

- **Duration:** ~65 min
- **Tasks:** 3 completed
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `src/overlay/rule_editor.rs::render_rule_body(ui, rule, config, id)` renders all seven condition controls in a fixed vertical stack (label line, then control) that never widens or resizes the 460px stripe (D-01), returning `true` whenever any control changed the rule this frame for the caller's autosave dirty-tracking.
- **Style (EDIT-03/D-09):** a `ComboBox` salted `(RuleId, "style")`, options from `edit::available_styles(&config.styles)` — no literal style name anywhere in the file.
- **Classes (EDIT-04/D-10):** three `CollapsingHeader`s (WEAPONS/ARMOUR/FLASKS & CHARMS) salted `(RuleId, group_label)`, sourced from `edit::class_groups(config)`, each showing a live selected-count in its header text; checkboxes read/write through `edit::is_class_selected`/`edit::set_class` — no literal class name anywhere in the file.
- **Rarity (EDIT-05/D-11):** a `circular_toggle` deciding `Some`/`None`, then a comparator row (Exactly/At Least/At Most) and a value row (Normal/Magic/Rare/Unique) via `ui.selectable_value` — the compact `RarityComparator` string form stays the model's own `Serialize` job; this file only ever passes a typed `RarityOp`/`Rarity` pair to `edit::set_rarity`.
- **Defences (EDIT-06/D-12):** a group `circular_toggle` bound to `edit::defence_group_enabled`/`enable_defence_group`/`clear_defence_group`, with three checkboxes (Armour/Evasion/Energy Shield) wrapped in `ui.add_enabled_ui` so they grey out rather than vanish when the group is off — reaches all seven PoE2 armour combinations through three checkboxes instead of a seven-way picker, with an explicit "ticked = has this defence, unticked = has none of it" hint in the row itself.
- **Sockets/quality (EDIT-07/D-13):** a `circular_toggle` plus an `egui::DragValue` each (ranges `0..=6`/`0..=20`, speed `0.2`), seeding a minimum of 1 on enable through `edit::set_sockets_min`/`edit::set_quality_min` and clearing through `edit::clear_sockets`/`edit::clear_quality` on disable.
- **Corrupted (EDIT-07/D-13):** deliberately reuses the exact same enable-toggle + value vocabulary as every other condition (RESEARCH Pattern 2) rather than a bespoke tri-state widget — `None` is "unset," `Some(false)`/`Some(true)` is an ordinary two-way `selectable_value` pair.
- **EDIT-08 sweep:** the module's doc comment now carries an explicit condition inventory — every one of the seven controls (style excepted, which isn't a condition) has exactly one documented, independent off switch, auditable by reading the file.
- `app.rs`'s accordion row loop now threads a genuine `&mut Rule` (via `WorkingRules::rule_mut`), `&Config`, and `RuleId` into `rule_editor::render_rule_body`, while keeping the existing deferred `RowAction` pattern for anything that mutates the rule *array* itself (move/delete/add) — condition edits apply immediately since they never touch array structure.

## Task Commits

Each task was committed atomically:

1. **Task 1: The rule body shell, the style dropdown, and the class picker** - `f30e209` (feat)
2. **Task 2: Rarity, and the defence group with its three checkboxes** - `764e53e` (feat)
3. **Task 3: Sockets, quality, corrupted — and the per-condition off switch everywhere** - `afe8669` (feat)

**Plan metadata:** committed with this SUMMARY (see final commit below)

## Files Created/Modified

- `src/overlay/rule_editor.rs` — new presentation module: `render_rule_body` plus seven private per-condition renderers (`render_style`, `render_classes`, `render_rarity`, `render_defence`, `render_sockets`, `render_quality`, `render_corrupted`), 357 lines, zero direct `When`-field assignment (all mutation via `overlay::edit`)
- `src/overlay/app.rs` — accordion row loop reworked to collect `row_ids: Vec<RuleId>` up front and call `working.rule_mut(id)` per row (instead of iterating `working.rows()` directly), so the expanded row's body gets real write access; added a `config_ref`/`body_dirty` pair and wired `rule_editor::render_rule_body` into the `CollapsingHeader`'s content closure; removed the plan-14-02 placeholder `render_rule_body`
- `src/overlay/mod.rs` — registered `pub mod rule_editor;`

## Decisions Made

See `key-decisions` in the frontmatter — the up-front `row_ids` collection (to give the body a real `&mut Rule` without an aliasing conflict with the immutable `working.rows()` walk), the disjoint `config_ref` borrow held alongside `working`, the plain-English comparator labels, and keeping sockets/quality as two separate functions rather than one generic helper are the load-bearing ones.

## Deviations from Plan

**1. [Rule 1 — wording, not logic] Reworded three prose doc-comments so the plan's own anti-literal greps would not false-positive on documentation text**
- **Found during:** Tasks 2 and 3, running the plan's own `<verify>`/acceptance-criteria grep checks
- **Issue:** Three doc comments quoted the exact forbidden substrings the plan's greps search for, even though no actual code used them: (a) `render_rarity`'s doc comment originally wrote out `` `">=Magic"` ``, tripping the `'">=|"<=|"=='` comparator-string grep; (b) `render_defence`'s doc comment originally wrote `` `min: Some(1)` ``/`` `eq: Some(0)` ``, tripping the `'NumericCondition|min: *Some|eq: *Some'` grep; (c) `render_sockets`'s doc comment originally named `` `TextEdit` ``, tripping the `'TextEdit'` grep that asserts no hand-rolled text-entry widget exists.
- **Fix:** Reworded all three comments to describe the same behavior in prose without quoting the literal forbidden substrings — e.g. "a strictly-positive minimum" instead of `` `min: Some(1)` ``, "a hand-rolled text-entry-plus-parse-on-blur widget" instead of `` `TextEdit` ``.
- **Files modified:** `src/overlay/rule_editor.rs`
- **Committed in:** `764e53e` (Task 2, for (a) and (b)) and `afe8669` (Task 3, for (c)) — caught and fixed before each commit via the plan's own verification greps, never shipped tripping the check.

None of the above changed behavior or design — all three are wording-only fixes so the plan's own acceptance-criteria greps assert what they intend (no literal comparator strings / no literal `NumericCondition` construction / no hand-rolled text widget *in code*) without being confused by prose that merely *describes* those things for a reader.

---

**Total deviations:** 1 category, 3 occurrences, all Rule 1 (wording-only, self-caught pre-commit via the plan's own grep checks).
**Impact on plan:** No scope creep, no behavior change. All three fixes make the file's prose consistent with what its own acceptance criteria demand.

## Issues Encountered

None beyond the wording-only fixes documented above under Deviations.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 14-04 (Apply + the D-03 unapplied-changes indicator) can proceed directly: `self.config` already holds the full, live `Config` every condition edit in this plan mutates and autosave (from 14-02) already persists, ready for `dynamic::compile_store`/`write_filter` to consume unchanged.
- The `rule_editor.rs` module is intentionally presentation-only (D-18) — nothing here needs revisiting for Apply; `render_rule_body`'s `bool` return already feeds the existing `dirty` flag 14-02 built, and 14-04's own `applied: bool` indicator is a separate, additive field.
- No blockers or concerns. Manual look-at-it verification (accordion feel, glyph rendering, scroll-into-view with a fully populated condition body) is deferred to 14-04's checkpoint per the plan's own scope fence — this plan's acceptance criteria were structural (greps) plus the already-green library test suite, not a running-window check.

---
*Phase: 14-rule-editor-apply*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created/modified files (`src/overlay/rule_editor.rs`, `src/overlay/app.rs`,
`src/overlay/mod.rs`, this SUMMARY.md) confirmed present on disk. All three task commit hashes
(`f30e209`, `764e53e`, `afe8669`) confirmed present in `git log`.
