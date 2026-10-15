---
phase: 19
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/19-overlay-rule-list-over-parsed-blocks/19-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 3
skipped: 2
status: partial
---

# Phase 19: Code Review Fix Report

**Fixed at:** 2026-10-12
**Source review:** 19-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (critical + warning): 5
- Fixed: 3 (plus 3 Info items fixed opportunistically: IN-01, IN-02, IN-04)
- Skipped: 2 (WR-03, WR-05), plus IN-03

Full `cargo test` passes (498 unit tests plus integration tests), `cargo fmt --check` is clean,
`cargo build --bins` is warning-free. Fixtures are untouched.

## Fixed Issues

### WR-01: Clearing a loaded rule name collapses the NAME editor mid-edit

**Files modified:** `src/overlay/rule_list.rs`
**Commit:** 3aa18ae
**Applied fix:** the id is inserted into `name_open` whenever the field is drawn, so emptying the text no longer closes it. Toggling off still removes it. Logic change: requires human verification in the GUI.

### WR-02: Adding a rule the search hides did not actually expand or scroll to it

**Files modified:** `src/overlay/app.rs`
**Commit:** 35ff098
**Applied fix:** in the hidden case `apply_row_actions` now sets only the status message and no longer sets `expanded` or `pending_scroll`. The test was renamed (`..._does_not_expand_it`) and asserts that. Requires human verification.

### WR-04: Timing-based and count-pinned assertions

**Files modified:** `tests/overlay_rules.rs`
**Commit:** 91bacc2
**Applied fix:** load budget 30 s, search 2 s, average edit 1 s (still catches quadratic regressions, so the EDIT-07 guard stays). The 511/460/51 counts are replaced by derived invariants (enabled + disabled == rows, both nonzero, more than 100 rows). The revert-restores-bytes and search checks are kept.

### Info fixed alongside

- IN-02 (3aa18ae): removed the unreachable `cannot_enable` push in `flags_for`.
- IN-01 (35ff098): the indicator goes `Unapplied` only if `working.is_dirty()`, so no-op actions no longer make it stale.
- IN-04 (963ffb4, `src/overlay/rules.rs`): `SetContinue` clears the `moved` set so the "moved across Continue" flag cannot go stale.

## Skipped Issues

### WR-03: Every Apply with an economy section reprices it

**File:** `src/overlay/app.rs:139-149`
**Reason:** Per orchestrator guidance, refreshing the economy section on Apply is intended Phase 18 behaviour (how new prices reach the filter). The fallback case is already reported in the status message. No minimal, clearly correct improvement was identified, so no change was made.
**Original issue:** Apply silently falls back to fallback prices if `prices.json` is absent, and rule-only Apply can fail on economy errors.

### WR-05: `text_entry.rs` is an unused public module

**File:** `src/overlay/text_entry.rs`
**Reason:** Kept deliberately for Phase 20 (base-type list editing), per orchestrator guidance. It is a documented module, so no change.
**Original issue:** `parse_base_type_entry` is unreferenced.

### IN-03 (Info): search haystack includes index and Show/Hide keyword

**Reason:** The spec requires the label in the haystack; changing it is a behaviour decision, not trivially safe.

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

## Later resolution (v0.5 wrap-up, 2026-10-15)

- **WR-05 resolved by Phase 20, no deletion needed.** Re-checked at the v0.5 wrap-up:
  `parse_base_type_entry` is no longer dead. Phase 20 (commit 3681657, quoted-list add field)
  imports it in `src/overlay/widgets.rs` to split pasted quoted lists, so `text_entry.rs` is live
  code and was kept.
- **Author decision (2026-10-15), WR-03:** every Apply repricing the economy section is accepted as
  designed (it is how new prices reach the filter).
