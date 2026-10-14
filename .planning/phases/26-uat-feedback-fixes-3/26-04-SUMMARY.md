---
phase: 26-uat-feedback-fixes-3
plan: 04
subsystem: overlay
tags: [egui, rule-editor, regression, pending-rows]
requires: []
provides:
  - class-group open state that survives a pending row becoming a real line
  - operator dropdown on every pending condition
  - condition picker whose sections start collapsed
affects: [src/overlay/body.rs, src/overlay/rule_list.rs]
key-files:
  modified:
    - src/overlay/body.rs
    - src/overlay/rule_list.rs
requirements: [UAT3-04, UAT3-07]
completed: 2026-10-14
---

# Phase 26 Plan 04: Rule editor collapse fix, pending operators, collapsed picker

The first Class click no longer closes the open class group, every pending condition has its operator dropdown, and the + condition picker opens fully collapsed.

## Root cause (UAT3-04)

Proven by `first_class_click_keeps_rule_and_group_open`, which failed on the RED commit (059e86f) and passes after the fix (1d7c949). The test drives the real `show_list` with the headless harness, mirrors `app::apply_row_actions`, and clicks a pending Class group header and then a class checkbox.

- The rule row itself was NOT collapsed: `list.expanded` stayed `Some(id)` after the AddLine (the first assertion passed before the fix). What closed was the open class group (the "collapse" the author sees).
- Cause: the group's open/closed state lived in egui memory under an id that changed at the moment the pending row became a line. Two contributors, both in the id of the `CollapsingHeader` in `class_picker`:
  1. `rule_list.rs` `show_list` wraps each row in `push_id(("rule", row.id, list.epoch))`, and `app.rs` `apply_row_actions` (~L559-594) calls `list.forget_card_state()` (epoch bump) for every AddLine/RemoveLine. All ids under the row changed on the frame after the first click.
  2. The pending card used `push_id((block, kw, "pending"))` with salt `(block, kw, usize::MAX, "pending")`, while the real card uses `push_id((id, kw, ordinal))` with `card.salt("class")`. So even without the epoch bump the group id differed between the pending and real cards.
- A hypothesis the test ruled out: header click fall-through and `list.expanded` reset (the row stayed expanded); `BlockId` stability after `insert_body_line` (id unchanged).

## Fix

- `class_picker` keeps open state under an absolute id `("ff-class-group-open", row_epoch, salt, label)` and drives the header with `.open(Some(..))`, flipping the stored bool on `header_response.clicked()`.
- New `ListUi::row_epoch`, bumped only when the expanded row changes or ids reset (`clear_ids`), not on AddLine/RemoveLine. A group therefore stays open across line adds/removals and is closed again when the row is collapsed and reopened (same as before).
- The pending card now uses the real card's salt: `(block, kw, ordinal-of-the-line-to-be-created, "class")`, where the ordinal is the count of existing entries of that keyword (matches `BodyEntry.ordinal`).
- WR-07 untouched: `epoch` and `forget_card_state()` still orphan the per-card text drafts and bad-input flags; existing epoch tests pass. No edit to `app.rs` was needed (26-03's regions untouched).
- `first_rarity_chip_click_keeps_the_rule_expanded` covers the Rarity first-chip path. The BaseType first-entry path is covered at the function level (`pending_base_type_starts_exact_and_shows_its_operator`); typing into the add field is not simulated.

## Operator on pending rows (UAT3-04)

`pending_card` shows a ComboBox from `grammar::operator_choices(p.keyword, p.op, false)` (width `OP_W`), with "Add an entry to create this condition" as a hover tooltip on the card. A pick updates the matching `list.pending` entry (by block + keyword name) and is used by that frame's `pending_first_entry`; nothing is written until a value exists. Tests: `pending_class_operator_is_chosen_before_a_value_and_used_by_it` (UI-driven: pick Not, idle frame writes nothing, first class writes `Class != "..."`), `pending_base_type_starts_exact_and_shows_its_operator`, `pending_rarity_operator_applies_to_one_chip_but_not_to_two`, `idle_pending_rows_write_nothing_for_any_keyword`.

## Picker (UAT3-07)

`picker_popup` uses `PICKER_SECTION_DEFAULT_OPEN = false` and `picker_section_override(searching)` (search forces matching sections open, as before). Test `picker_sections_start_collapsed_and_a_search_opens_them` asserts the constant, the override and the absence of `default_open(gi == 0)`.

## Commits

- 059e86f test(26-04): failing regression (RED)
- 1d7c949 fix(26-04): keep the class group open when the first value adds the line
- 82c89f9 feat(26-04): operator dropdown on pending rows; picker sections start collapsed

## Deviations from Plan

- [Test approach] The picker "collapsed by default" and "search opens matches" behaviors are tested through a small pure helper/constant instead of a rendered popup. Headless egui did not grow the popup when a section opened (checked by probing click extents), so a UI-level pick test was unreliable. The search behavior itself is unchanged production code.
- Pending BaseType/Rarity quoted-list salt tag changed from "pending" to "class" along with the Class salt (only text-draft ids; harmless).

## Self-Check: PASSED

Full `cargo test` passes (874 lib tests plus integration), `cargo fmt --check` clean; generator fixtures untouched.
