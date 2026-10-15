---
phase: 20-full-grammar-block-editing
plan: 06
subsystem: overlay-styles
tags: [rust, overlay, styles, apply]

requires:
  - phase: 20-02
    provides: ClassCatalog
  - phase: 20-04
    provides: styled::{rewrite_styled, rewrite_block, style_name_ok}
  - phase: 20-05
    provides: line actions, coalesce_edits
provides:
  - rules::{StyleState, STYLE_LOCKED_STATUS_PREFIX}, RuleRow.style (replaces style_tag)
  - WorkingFilter::{set_styles, style_names, style, styles_error, text_with_styles, style_rewrite_pending, unknown_styles}
  - RowAction::{ApplyStyle, RemoveStyle}
  - apply::{styled_rules_message, unknown_styles_message, styles_unavailable_message, STYLE_REMOVED_STATUS}
  - OverlayApp classes/styles loading, mtime poll, style-aware Apply
affects: [20-07, 20-08, 20-09]

key-files:
  modified: [src/overlay/rules.rs, src/overlay/apply.rs, src/overlay/app.rs, src/overlay/rule_list.rs]

key-decisions:
  - "D-A1: disabled styled rules are not rewritten; ApplyStyle on a disabled rule writes the tag only"
  - "D-Q5: styles.json read at start, RELOAD, every Apply and when its mtime changes (polled every 2 s)"
  - "D-Q6: names failing style_name_ok, or absent from styles.json, are refused at ApplyStyle"
  - "D-STYLES-UNAVAILABLE: unreadable styles.json refuses Apply only when an enabled rule carries a #ff:style tag"
  - "config.json load failure shows 'Class groups unavailable (config.json): ...' in the list status and never blocks editing"
  - "The model lock applies only to StyleState::Known; Unknown and Unchecked rows stay fully editable"

requirements-completed: []
requirements-progress: [EDIT-06, STYLE-03]

completed: 2026-10-13
---

# Phase 20 Plan 06: Named styles in the working model and Apply Summary

**Rules carry a Known/Unknown/Unchecked style state; Apply rewrites styled rules from a freshly read styles.json on a document clone and reports `Updated N styled rules.` / unknown styles without ever mutating the user's edits on failure.**

## Accomplishments
- `ApplyStyle` is atomic (tag plus rewrite on a clone, swapped in only on success); `RemoveStyle` removes only the tag and reports `STYLE_REMOVED_STATUS`.
- Model-level lock: action AddLine/EditLine/RemoveLine and SetKeyword on a Known-styled rule are refused with `Set by style "S". Remove the style to edit it.`; conditions stay editable.
- `text_with_styles` / `style_rewrite_pending` (cached by generation and styles revision) / `unknown_styles`.
- `app.rs`: styles and class catalogue loaded at start and RELOAD, styles re-read at every Apply and on mtime change; changed styles.json marks the filter unapplied and enables Apply; clickable unknown-styles footer line; `coalesce_edits` applied per frame.
- `rule_list.rs` switched minimally from `style_tag` to `style` (lock and flag read `Known` only).

## Task Commits
1. Tasks 1 and 2 (StyleState, ApplyStyle/RemoveStyle, lock, text_with_styles, pending, unknown): 75d686d
2. Task 3 (app loading, Apply hook, poll, footer line): 5c9f65b

## Deviations from Plan
- Tasks 1 and 2 share one commit: the Task 2 tests need Task 1's state, and both live in `rules.rs`; they were written and passed together.
- `OverlayApp::classes()` carries `#[allow(dead_code)]` until 20-07 consumes it.
- Unknown-style jump uses the same `jump_to` mechanism as the grammar gate.

## Known Stubs
None.

## Self-Check: PASSED
Full `cargo test` green (596 lib tests plus integration), `cargo fmt --check` clean, bins warning-free, fixtures and snapshots unchanged.
