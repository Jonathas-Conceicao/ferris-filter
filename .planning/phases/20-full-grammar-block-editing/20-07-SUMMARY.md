---
phase: 20-full-grammar-block-editing
plan: 07
subsystem: overlay-body
tags: [rust, egui, overlay, conditions, editor]

requires:
  - phase: 20-02
    provides: grammar module and ClassCatalog
  - phase: 20-03
    provides: reusable body widgets
  - phase: 20-05
    provides: line-level RowActions and body_view
  - phase: 20-06
    provides: class catalogue loading, style state
provides:
  - overlay::body (BodyCtx, conditions_group, raw_row, picker_popup, pick_condition, pending_first_entry)
  - ListUi.pending / focus_line / last_expanded / line_error and PendingRow
  - show_list(.., classes: Option<&ClassCatalog>)
affects: [20-08, 20-09]

key-files:
  created: [src/overlay/body.rs]
  modified: [src/overlay/rule_list.rs, src/overlay/app.rs, src/overlay/mod.rs]

key-decisions:
  - "Pure edit helpers take the BlockId as first argument (int_edit, op_edit, rarity_edit, raw_edit, raw_line_edit): RowAction needs it and BodyEntry has none"
  - "list_edit, count_names_edit, bool_or_name_edit return Result<Option<RowAction>, EncodeError> so quote/control text raises the inline hint instead of being dropped"
  - "CONDITIONS shows Condition and Opaque entries; Action and Continue lines are left to 20-08 and the existing Continue checkbox"
  - "Opaque lines get a raw row (Whole edit); a refused edit sets ListUi.line_error, rendered 13px WARM_ACCENT_TEXT under that line's card and cleared on a successful edit of the block, on typing in the field, or when the expanded row changes"
  - "Sockets without a count (SocketGroup RGB) fall back to the raw value field; CountThenNames without a count shows `Any of these:` and keeps the count absent on edits"
  - "Rarity with no chip left removes the line and leaves a pending row (hint via the pending caption)"
  - "Enum single value is a combo, several values toggle chips (per plan); an emptied Enum removes the line"
  - "All value controls sit on line 2 of the card; line 1 is label (120px, truncated) + operator combo (104px) + remove X, so flags and arrows never squeeze the label"
  - "Raw drafts live in egui temp memory and Escape abandons them; a refused raw edit shows the current text plus the hint rather than the refused draft"
  - "BodyCtx.working is carried for 20-08 but not read here"

requirements-completed: []
requirements-progress: [EDIT-01]

completed: 2026-10-13
---

# Phase 20 Plan 07: Condition editor body Summary

**Every condition line is now an editable card (keyword, plain-English operator dropdown, value control by shape, remove X) with a searchable grouped `+ condition` picker and pending rows for list shapes; unknown and opaque lines are inline-editable raw rows.**

## Accomplishments
- `src/overlay/body.rs` (new) replaces the read-only CONDITIONS list; the "Editing conditions arrives in a later update." text is gone.
- Controls: whole number (slider + exact field, `Enter a whole number` hint), Yes/No toggle, enum combo or chips, rarity chips (multi switches to `Matches` and drops an ordering operator in the same `Both` commit), grouped class picker from the catalogue (natural-height CollapsingHeaders, unknown classes as removable chips, first 6 chips then `+N more`), chip lists with quoted-paste splitting, count + mod chips, socket count + colour slots, Any/Named gem, multi-integer chips, raw value field.
- Writes only on a real change; idle frames over every shape emit no RowAction (tests over scalar, list, catalogue and pending-row blocks).
- `+ condition`: `picker_popup` (Popup::from_toggle_button_response, CloseOnClickOutside, search autofocus, Esc closes), `picker_entries` filtering, `(PoE 1)` suffix. Defaults add a line at once; BaseType (`==`), Class, Rarity, mods become `PendingRow`s that write on the first entry. Pending rows drop on collapse/row switch.
- `apply_row_actions` records `focus_line` from `ActionOutcome.line` (card scrolls into view and focuses its primary control) and `line_error` for refused raw/value edits.

## Task Commits
1. Task 1 - body scaffold, card, operator dropdown, scalar and raw controls: 76eef65
2. Task 2 - list-shaped controls: 1cef6ce
3. Task 3 - picker, pending rows, focus wiring tests: f9e078e

## Deviations from Plan
- Pure helpers carry a leading `BlockId` parameter (see decisions); `ListEdit` was not introduced as a type, `list_edit` returns the Result described above.
- Short controls (toggle, enum combo, rarity chips) were not hoisted onto line 1 of the card; they sit on line 2. Keeps the label unsqueezed and the layout uniform.
- The `+ condition` click and Esc-close are covered by a coordinate-sweep headless test rather than a fixed-position click, because widget rects are not exposed.
- `ListUi` additionally gained `line_error` (specified in the interfaces block) and `PendingRow` lives in `rule_list.rs` per the contract.

## Known Stubs
None.

## Threat Flags
None. T-20-23/24/25 mitigated: every value goes through `grammar::encode`; raw text goes through the validated `patch_line`; idle-frame tests assert no actions.

## Verification
Full `cargo test` green (620 lib tests plus integration), `cargo fmt --check` clean, `cargo build --bins` warning-free, fixtures and snapshots unchanged.

## Self-Check: PASSED
