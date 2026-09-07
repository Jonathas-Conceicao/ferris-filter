---
phase: 14-rule-editor-apply
plan: 01
subsystem: overlay
tags: [rust, serde, egui-free-library, tempfile, filter-editor]

# Dependency graph
requires:
  - phase: 12-dynamic-rule-store
    provides: dynamic::compile_store / write_filter / ensure_store as print-free, fully-parameterised callables
  - phase: 13-overlay-shell
    provides: overlay::store::load, overlay::view_model::rule_rows, the pure-module layering convention
provides:
  - "overlay::edit::WorkingRules — a RuleId-keyed working model with add/delete/move/toggle_enabled, immune to the ordinal-identity hazard under reordering"
  - "overlay::edit's per-condition set/clear functions covering EDIT-03..08 (style, class, rarity, defence, sockets, quality, corrupted)"
  - "overlay::store::save — atomic, crash-safe Config write via NamedTempFile::new_in + persist"
  - "tests/rule_editor.rs — the flask enable/disable end-to-end proof and the reorder-changes-precedence proof, both against compiled filter text"
affects: [14-02, 14-03, 14-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RuleId working model: session-only Vec<(RuleId, Rule)> identity layer, never serialised, so 'currently selected rule' state survives array permutation by construction"
    - "Uniform enable-toggle + value architecture for every When condition, including corrupted (no bespoke tri-state widget)"
    - "Atomic store write mirrors dynamic::write_filter's NamedTempFile::new_in + persist discipline"

key-files:
  created:
    - src/overlay/edit.rs
    - tests/rule_editor.rs
  modified:
    - src/overlay/store.rs
    - src/overlay/mod.rs

key-decisions:
  - "set_class collapses a fully-deselected class list to None, never Some(vec![]) — src/emitter.rs's Class line has no emptiness guard, so an empty-but-present vec would emit a malformed bare Class line"
  - "enable_defence_group writes all three defence attributes explicitly (eq: Some(0)); set_defence only ever writes its own attribute, so a partially-specified rule loaded from dynamic_base.json (evasion-only) is never rewritten just by being displayed"
  - "toggle_enabled collapses both None and Some(true) to None on re-enable, never writing a redundant \"enabled\": true key"
  - "store.rs's module header now documents that Phase 14 lifts the Phase 13 restriction on overlay-side compile/write mutation, rather than contradicting the code with a stale 'never writes' claim"

requirements-completed: [EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05, EDIT-06, EDIT-07, EDIT-08]

# Metrics
duration: ~55min
completed: 2026-09-07
---

# Phase 14 Plan 01: Pure Rule-Editing Library Layer Summary

**`overlay::edit`'s RuleId-keyed WorkingRules plus a full set of per-condition mutators (style/class/rarity/defence/sockets/quality/corrupted), all unit-tested with zero egui imports, backed by an atomic `store::save` and an end-to-end proof that disabling one rule removes the flask/charm block from a compiled filter.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 3 completed
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- `overlay::edit::WorkingRules` — `RuleId`-keyed `add`/`delete`/`move_up`/`move_down`/`toggle_enabled`, with a test that moves a *different* rule past a captured id and confirms the id still resolves correctly (D-19).
- Every EDIT-03..08 condition (style, class, rarity, the three-checkbox defence picker, sockets, quality, corrupted) is settable and clearable through a pure function, with `available_styles`/`class_groups` read live from the loaded `Config` — never a hardcoded literal (`PITFALLS.md` §10).
- `overlay::store::save` — atomic, crash-safe write via `tempfile::NamedTempFile::new_in` + `.persist()`, plus a `dynamic_base.json` round-trip guard asserting `Config` value equality (byte-identical text is explicitly not required — `HashMap` key-order reshuffling and explicit `"eq": null` keys are documented as harmless, not bugs).
- `tests/rule_editor.rs` proves roadmap success criteria 1 and 2 against **compiled filter text**: building a flask/charm rule through the edit API and disabling it removes the block from `dynamic::compile_store`'s output in both directions; moving one of two overlapping rules changes which style's block appears first.

## Task Commits

Each task was committed atomically:

1. **Task 1: The RuleId working model — add, delete, move, enable/disable** - `bac13c3` (feat)
2. **Task 2: Per-condition set/clear functions and the config-sourced option lists** - `5f42470` (feat)
3. **Task 3: Atomic store::save, the round-trip guard, and the flask end-to-end proof** - `3f44241` (feat)

**Plan metadata:** committed with this SUMMARY (see final commit below)

## Files Created/Modified

- `src/overlay/edit.rs` — new pure mutation module: `RuleId`, `WorkingRules`, `Defence`, `available_styles`, `class_groups`, `set_style`/`set_class`/`is_class_selected`/`set_rarity`/`clear_rarity`/`set_defence`/`defence_ticked`/`defence_group_enabled`/`enable_defence_group`/`clear_defence_group`/`set_sockets_min`/`clear_sockets`/`set_quality_min`/`clear_quality`/`set_corrupted`, 37 unit tests
- `src/overlay/store.rs` — added `save(&Config, &Path) -> Result<(), String>`; rewrote the module header to describe the read/write lifecycle instead of a now-stale "overlay never writes" claim; added 5 new tests plus the `dynamic_base_json_round_trips_through_config_losslessly` guard
- `src/overlay/mod.rs` — registered `pub mod edit;`
- `tests/rule_editor.rs` — new library-level integration test file: `disabling_one_flask_rule_removes_the_flask_block_from_the_compiled_filter` (roadmap criterion 2) and `move_up_changes_which_style_wins_for_overlapping_rules` (roadmap criterion 1)

## Decisions Made

- Split what the plan describes as two tasks' worth of code (`WorkingRules` and the per-condition functions) into two separate commits within the same file by staging a reduced intermediate version first, so each task lands as its own atomic, independently-compiling commit rather than one combined diff.
- `available_styles`/`class_groups` return borrowed/static data (`Vec<&'static str>`, `Vec<(&'static str, &[String])>`) rather than owned `String`/`Vec<String>` — cheaper for a per-frame call from the presentation layer in Plan 02, and the config's own class vectors already outlive the borrow within a single frame's render pass.
- `tests/rule_editor.rs` reuses `tests/dynamic_filter.rs`'s `workspace`/`policy_section` helper shapes (duplicated locally, since integration test binaries do not share code) rather than inventing a new harness style.

## Deviations from Plan

None — plan executed as written. Two test-authoring bugs were caught and fixed during self-verification before commit (not deviations from the plan's design, just fixing bugs in my own test code):

- Two boundary tests originally constructed a "stray" `RuleId` from an unrelated `WorkingRules::from_rules` call; since every `WorkingRules` starts its own id counter at 0, this produced an id that collided with a genuinely present id rather than testing the unknown-id path. Replaced with an `unknown_id` helper that adds-then-deletes a rule on the actual `wr` under test, guaranteeing a truly absent id.
- One D-19 test's expected post-move position was arithmetically wrong (asserted position 2 for a rule that ends up at position 1 after a different rule is moved twice past it). Corrected the assertion to match the actual, correct `Vec::swap`-based semantics.

Both were caught by `cargo test` failing before any commit was made — no incorrect assertion ever shipped.

## Issues Encountered

None beyond the test-authoring bugs above, both self-caught and fixed pre-commit.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plans 02+ (the presentation layer: accordion, per-condition widgets, autosave wiring, Apply) can call `overlay::edit`'s functions directly; no further library work is a prerequisite.
- `overlay::store::save` is ready for the per-frame autosave call Pattern 5 describes; nothing in `app.rs` calls it yet (that wiring is Plan 02/03's job per RESEARCH's recommended task shape).
- No blockers or concerns.

---
*Phase: 14-rule-editor-apply*
*Completed: 2026-09-07*
