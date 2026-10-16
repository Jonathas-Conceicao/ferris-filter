---
phase: 34
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/34-economy-about-tabs/34-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 34: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/phases/34-economy-about-tabs/34-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3
- Fixed: 3
- Skipped: 0

Work was done directly on the milestone branch (no separate worktree), per the orchestrator's instructions. Each commit passed `cargo test`, `cargo fmt --check`, and clippy (lib test 119 + cli_generate 2 + sync_prices 1 = 122, within the limit). `tests/fixtures` is untouched.

## Fixed Issues

### WR-02: A staged economy add/remove marks the RULES tab with ` *`, never ECONOMY

**Files modified:** `src/overlay/rules.rs`, `src/overlay/shell.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** c407b78
**Applied fix:** Added `WorkingFilter::rules_edited()`, which ignores the staged economy op. `tab_markers` takes an `economy_staged` flag and marks ECONOMY with `tiers_dirty || economy_staged`. RULES is marked only for real rule edits. Tests: `a_staged_economy_op_is_not_a_rules_edit`, `a_staged_economy_op_marks_economy_not_rules`. CLAUDE.md tab-marker text updated.

### WR-01: ECONOMY Revert drops the staged economy op before the tier-discard popup is confirmed

**Files modified:** `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 2a8114c
**Applied fix:** New `economy_revert_pressed`. With tier edits it only opens the Discard popup. Without them, a staged op is dropped at once. The Discard popup's confirm branch (Tiers scope) also unstages the economy op, and the popup count includes the staged op. Tests cover Keep editing, Discard, and the no-tier-edits path. CLAUDE.md Revert note updated.

### WR-03: ECONOMY "Price source" and "Last sync" misreport an empty or malformed prices.json

**Files modified:** `src/overlay/app.rs`, `src/overlay/shell.rs`, `CLAUDE.md`
**Commit:** 183f641
**Applied fix:** `EconomyInfo` now keeps a `problem` (load error, or "no priced items"). The file mtime is reported as `None` ("Never") when there is a problem. The Price source tooltip shows the reason instead of "No prices.json yet". A missing file keeps the old text. Tests extend `economy_info_reads_prices_json` and the render-every-state test. CLAUDE.md Economy note updated.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
