---
phase: 49-drag-and-drop-reorder
verified: 2026-10-23T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (automated); real-pointer behaviour deferred to human
human_verification:
  - test: "Ten real-pointer checks listed in .planning/phases/AUTONOMOUS-V072-DEFERRED.md under 'Phase 49:' (drag and Apply, wheel and edge auto-scroll, Escape or release outside cancels, group and Continue flag, collapsed-header drop, economy barrier, dimmed handle while searching, custom styles in CONFIG with Revert, hide mid-drag cancels, art at 100% and 150-200% DPI)"
    expected: "Each behaves as in the roadmap success criteria"
    why_human: "Real pointer drags cannot be driven headlessly"
---

# Phase 49: Drag-and-Drop Reorder Verification Report

**Phase Goal:** Rules and custom styles are reordered by dragging.
**Status:** human_needed (no automated gaps)

## Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Handle with author's art; arrows gone on rules and custom styles, kept on section headers | VERIFIED | `ff_drag_handle` used at rule_list.rs:1346 and config_view.rs:1283; `ff_arrow` remains only at rule_list.rs:950-951 (section header). Screenshots rules-handles/config-styles show handles on rules and custom styles, arrows only on section headers, built-in Order cells blank. |
| 2 | Greyed source, translucent shadow, landing marker; wheel and edge auto-scroll | VERIFIED (visual) / human (scroll feel) | rules-drag.png shows greyed source, translucent copy, gold marker. Auto-scroll is covered by unit tests and the spike. A real pointer is needed for the feel. |
| 3 | Drop = one pending edit; outside or Escape cancels | VERIFIED (code and tests) / human (real pointer) | `Document::move_run_to`, `RowAction::MoveTo`, `move_custom_to` exist. Escape is consumed by the drag (rule_list.rs:581-584). Tests pass. |
| 4 | Arrow rules hold (group unit, Continue flag, banner/collapsed header, economy barrier, off while searching, customs never above built-ins) | VERIFIED (tests) | `drag_reorder.rs` pure slot logic with economy rules; `structure.rs` has 14 move/drag-related tests; the CONFIG side rotates custom slots only. |
| 5 | Nine art files committed under their own names | VERIFIED | `assets/custom/` holds icon_reoder_{normal,hover,pressed}.png and .ase, button-normal-palllet.ase, button-hover-palllet.ase, button-prassed-pallet.ase, all tracked (commit 16638d1). |

## Checks
- `cargo test --locked`: lib 1845 passed, 0 failed; all integration suites pass, including artifact_guard and generate_snapshot.
- `git diff b563e49..HEAD --stat -- tests/fixtures Cargo.toml Cargo.lock`: empty (no fixture or dependency change).
- No keyboard reordering added: the only arrow-key reference is in an existing key-noise test, and none are bound to a move action.
- Section headers keep arrows (rule_list.rs:950-951).

## Requirements Coverage

| ID | Status | Evidence |
|---|---|---|
| DND-01 | SATISFIED | SC1 |
| DND-02 | SATISFIED (visual), scroll human | SC2 |
| DND-03 | SATISFIED | SC3 |
| DND-04 | SATISFIED | SC4 |
| DND-05 | SATISFIED | SC5 |

No orphaned requirement IDs. REQUIREMENTS.md still marks them Pending and the ROADMAP plan boxes are unchecked; the orchestrator should update both.

## Anti-Patterns
None blocking. Release Review note: the unused arrow model API (`RowAction::Move`, `can_move_*`, `MoveStyle`, `move_custom`) is retained and already recorded for cleanup.

## Human Verification Required
The ten numbered real-pointer checks in AUTONOMOUS-V072-DEFERRED.md under "Phase 49:".

## Gaps Summary
No automated gaps. Only the human pointer checks remain.

_Verifier: Claude (gsd-verifier)_
