---
phase: 15-basetype-lists-and-rule-names
plan: 02
subsystem: ui
tags: [overlay, egui, rust, ephemeral-state, filter-syntax]

# Dependency graph
requires:
  - phase: 15-basetype-lists-and-rule-names
    plan: 01
    provides: "overlay::edit::parse_base_type_entry/add_base_type/remove_base_type/clear_base_type/set_name/clear_name/clear_class — the pure vocabulary this plan's controls call"
provides:
  - "render_name, render_classes' own off switch, and render_base_types in overlay::rule_editor — the interactive controls that make EDIT-10/EDIT-11 usable from the stripe"
  - "panel_open/set_panel_open — the id-keyed ephemeral panel-open primitive (RESEARCH Pattern 1), shared by all three new/changed toggles"
affects: [15-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ephemeral, egui-id-keyed toggle state (ui.data_mut/get_temp_mut_or) for a condition with no sensible non-empty default, decoupled from Option::is_some() (RESEARCH Pattern 1)"
    - "Deferred per-item removal from an owned per-frame clone, single Option<usize> applied after the loop (RESEARCH Pattern 2, mirroring app.rs's RowAction)"
    - "Commit-on-Enter via response.lost_focus() && Key::Enter, with request_focus() to keep typing without the mouse (RESEARCH Pattern 3, TextEdit's own documented usage)"

key-files:
  created: []
  modified:
    - src/overlay/rule_editor.rs

key-decisions:
  - "grep -c 'edit::clear_class' returns 3, not the plan's literal 1 — two of the three matches are intra-doc [`edit::clear_class`] links (the module-doc EDIT-08 inventory bullet and render_classes's own doc comment), not additional call sites. Verified the real call-site count directly (grep excluding doc-comment lines): exactly 1, at the toggle's off branch. This is the exact over-counting pattern CLAUDE.md's constraint 5 anticipated for this specific grep; the doc comments were kept as-is (removing them would reduce documentation quality for no correctness gain) and are documented here per that constraint's instruction rather than silently weakening the check."
  - "The initial doc comment for render_base_types's remove-button rationale spelled out the banned glyphs literally (✕/×/✖) as examples of what NOT to use, which tripped the plan's own negative grep for those exact codepoints (CLAUDE.md constraint 6's doc-comment false-positive case, restated for this phase). Reworded to describe the defect in words (\"a Unicode close/cross glyph\") instead of naming the codepoints, preserving the warning without tripping the grep."

requirements-completed: []

# Metrics
duration: 55min
completed: 2026-09-08
---

# Phase 15 Plan 02: BaseType Lists and Rule Names — Presentation Layer Summary

**Three new/changed controls in the rule editor's expanded body — a NAME field, CLASSES' own off switch, and a BASE TYPES list — all backed by egui's id-keyed ephemeral panel-open state instead of `Option::is_some()`, closing the one flicker-shut failure mode RESEARCH flagged as this phase's sole new architecture.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-09-08T23:30:00Z
- **Completed:** 2026-09-09T00:25:00Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- `render_name` renders a NAME field at the very top of the expanded body (D-16), gated by a
  `circular_toggle` whose checked-state is read from `panel_open` — a small id-keyed ephemeral
  helper written this plan — rather than `rule.name.is_some()`. Switching it on writes nothing
  to the model (D-11); switching it off calls `edit::clear_name` (D-12). The text field commits
  through `edit::set_name` on any loss of focus, and re-requests focus specifically when the
  commit came from Enter, so a run of edits never needs the mouse.
- `render_classes` gained its own `circular_toggle`, the same `panel_open`-backed shape as the
  name field: off hides all four `CollapsingHeader` groups entirely (not greyed out — D-07) and
  calls `edit::clear_class`; on pre-selects nothing (D-09). The existing config-sourced checkbox
  loop underneath is unchanged.
- `render_base_types` renders the list directly after classes (D-16): an owned per-frame clone of
  the entries, each with a plain `Remove` text button (not a Unicode glyph — this codebase has
  already shipped that tofu-box defect twice), removal deferred to a single `Option<usize>`
  applied after the loop (mirroring `app.rs`'s `RowAction`), plus an add field that commits on
  Enter or a `+ ADD` button through `edit::parse_base_type_entry`/`edit::add_base_type` and then
  clears and refocuses itself.
- Every one of the three new/changed toggles has its own three-frame click test
  (`the_name_field_survives_the_frame_after_its_toggle_was_clicked`,
  `the_class_groups_survive_the_frame_after_the_toggle_was_clicked`,
  `the_base_type_list_survives_the_frame_after_the_toggle_was_clicked`) that clicks the toggle
  through the real `render_*` function via `widgets.rs`'s own `click_at`/`headless_ctx` harness
  and asserts, via `egui::Context::interactive_rects_last_pass()`, that the panel is still open
  on the frame *after* the click — closing RESEARCH Pitfall 1 / threat T-15-05 for real, not just
  for the shared primitive in isolation.
- `a_new_rule_opens_with_every_condition_untouched` exercises the whole `render_rule_body` for two
  no-input frames and asserts every `When` field (plus `rule.name`) stays `None` throughout — the
  end-to-end proof that D-09 still holds with three more toggles in the file.

## Task Commits

Each task was committed atomically:

1. **Task 1: The ephemeral panel-open state, and the name field at the top of the body** - `2eb88fb` (feat)
2. **Task 2: The class off switch, and the module-doc inventory it invalidates** - `9206080` (feat)
3. **Task 3: The base-type list — add one at a time, remove any entry** - `2a7aefd` (feat)

_No plan-metadata commit yet — created after this SUMMARY._

## Files Created/Modified

- `src/overlay/rule_editor.rs` - `panel_open`/`set_panel_open` (the shared ephemeral primitive),
  `render_name`, `render_classes`'s new off switch, `render_base_types`, the rewritten EDIT-08
  module-doc inventory, and 8 new `#[cfg(test)]` tests (306 lib tests total, up from 298)

## Decisions Made

- Kept the two known over-counting greps from CLAUDE.md's project-specific constraints as loose
  sanity checks rather than literal gates, per that constraint's own instruction: `edit::clear_class`
  greps to 3 (1 real call site + 2 intra-doc links) and the base-type remove-glyph doc comment
  originally named the banned codepoints literally, tripping its own ban grep. Both are recorded
  in `key-decisions` above with the real, verified count/fix rather than silently weakening either
  check.
- `set_name`'s commit-on-any-loss-of-focus behavior (rather than Enter-only) was implemented
  exactly as the plan's Task 1 action specified, deliberately differing from the base-type add
  field's Enter/button-only commit — both are documented inline in each function's own doc comment
  so a future reader does not "unify" them into one shape.
- The EDIT-08 module-doc inventory's three new/changed bullets (`name`, `classes`, `base types`)
  are grouped with a single shared paragraph explaining why those three (and only those three)
  back their toggle with ephemeral state instead of the uniform `Option::is_some()` pattern the
  rest of the inventory uses — matching the plan's own instruction to record the reason so a
  future maintainer does not "simplify" them into a flicker regression.

## Deviations from Plan

None beyond the two documented grep false-positives above (both anticipated by CLAUDE.md's
project-specific constraints 5 and 6, verified against the real code, and fixed/left as
instructed rather than treated as scope changes).

## Issues Encountered

The research's own code sketches for `panel_open` (`*ui.data_mut(|d| d.get_temp_mut_or(...))`)
and the name-buffer read (`ui.data_mut(|d| d.get_temp_mut_or_insert_with(...)).clone()`) do not
compile as written against the installed egui 0.36.1: `Context::data_mut`'s closure signature is
effectively higher-ranked over the `&mut IdTypeMap` lifetime, so returning a `&mut T` tied to that
lifetime out of the closure (to dereference/clone outside it) fails to typecheck ("lifetime may
not live long enough"). Fixed by moving the dereference/`.clone()` *inside* the closure so the
closure returns an owned value (`bool`/`String`) instead of a reference — a mechanical fix with no
behavioral difference from the sketch's intent, applied to `panel_open` and both text-buffer reads
(`render_name`, `render_base_types`). No test needed to be reshaped by this; the fix is
purely a lifetime-satisfying rewrite of the same logic.

## User Setup Required

None - no external service configuration required.

## Requirements Note

`EDIT-10`/`EDIT-11` are **not** marked complete in `REQUIREMENTS.md` by this plan, per this
plan's explicit instruction: both requirements are end-user-facing and are not considered done
until plan 15-03's live in-game check passes. `.planning/STATE.md`, `.planning/ROADMAP.md` and
`.planning/REQUIREMENTS.md` were not touched by this executor at all, per the orchestrator's
explicit instruction that it owns those writes this session.

## Next Phase Readiness

All three controls (`render_name`, `render_classes`'s off switch, `render_base_types`) are wired
into `render_rule_body` in D-16's order (name, style, classes, base types, rarity, ...) and are
ready for 15-03's manual in-game verification: toggling name/class/base-type on and confirming the
panel stays open, typing several base names in a row using only Enter, expanding a rule with a
long base name to check the wrapped-label-plus-remove-button layout at 460px, and confirming the
`Remove` button reads cleanly. No blockers — `emitter.rs`/`models/config.rs`/`edit.rs` are
untouched, the byte-for-byte oracle held with zero fixture changes, and the full test suite is
green.

## Test Results

- `cargo test --lib` — **306 passed, 0 failed** (298 pre-plan + 8 new: 4 in Task 1, 1 in Task 2,
  3 in Task 3).
- `cargo test` (full suite: lib + integration + doctests) — **346 passed, 0 failed** (306 lib + 39
  integration + 1 doctest; the 39 non-lib tests are unchanged from the 338 baseline).
- `cargo clippy --lib` — 37 pre-existing warnings elsewhere in the crate, **zero** in
  `src/overlay/rule_editor.rs`.
- `git diff --stat src/emitter.rs src/models/config.rs` — empty across all three tasks.
- `git status --porcelain tests/fixtures tests/snapshots` — empty across all three tasks.
- `git diff --name-only src/` — `src/overlay/rule_editor.rs` alone, matching the plan's own
  "no other file changes" claim.
- `grep -nE 'rule\.name *=|when\.class *=|when\.base_type *='  src/overlay/rule_editor.rs` — empty
  (D-18: no direct field assignment from presentation code).
- `grep -nE 'trim\(\)|replace\(|split\(|contains\(.\"' src/overlay/rule_editor.rs` — empty (no
  string hygiene reimplemented outside `edit::`).
- `grep -nE 'u\{2715\}|u\{00d7\}|✕|×|✖' src/overlay/rule_editor.rs` — empty after the doc-comment
  reword (RESEARCH Pitfall 4 / CLAUDE.md constraint 6 closed for this file).

## Self-Check: PASSED

- FOUND: `src/overlay/rule_editor.rs`
- FOUND: `.planning/phases/15-basetype-lists-and-rule-names/15-02-SUMMARY.md`
- FOUND: `2eb88fb` (Task 1 commit)
- FOUND: `9206080` (Task 2 commit)
- FOUND: `2a7aefd` (Task 3 commit)

---
*Phase: 15-basetype-lists-and-rule-names*
*Completed: 2026-09-08*
