---
phase: 17-config-driven-styles
fixed_at: 2026-10-13T00:26:22Z
review_path: .planning/phases/17-config-driven-styles/17-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 17: Code Review Fix Report

**Fixed at:** 2026-10-13T00:26:22Z
**Source review:** .planning/phases/17-config-driven-styles/17-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 2
- Fixed: 2
- Skipped: 0

Out-of-scope extra: IN-02 (CHANGELOG range checks) was folded in as requested (commit 2cb655c).

Full `cargo test` passes after all fixes. `tests/fixtures/seed-universal.filter` is untouched.

## Fixed Issues

### WR-01: An unparseable inline style is treated as matching and silently dropped

**Files modified:** `src/loader.rs`
**Commit:** d4501f7
**Applied fix:** In `check_legacy_styles`, a `null` inline entry is now accepted only when
styles.json does not define that name. That is the pre-Phase-17 `"craftable": null` case. Any other
entry that fails to deserialize as a `Style` counts as differing and is named in the
`InlineStylesSection` error. The `load_config` doc comment now describes this rule. There are four
new unit tests:
- `{"Bogus": 7}` is rejected, naming `Bogus`.
- A misspelled field (`craftable.fnt`) is rejected, naming `craftable`.
- `{"craftable": null}` loads.
- `{"S": null}` is rejected, naming `S`.

Status: fixed: requires human verification (this is a logic change to the acceptance rule).

### WR-02: `load_styles_rejects_duplicate_name` fails on any fresh checkout

**Files modified:** `src/loader.rs`
**Commit:** 1e58e76
**Applied fix:** The splice now inserts the duplicate `S` entry after the first `{` character
instead of matching `"{\n"`. The test normalizes `include_str!` text to LF and then runs against
both the LF and the CRLF variants, so it passes whatever line endings the checkout has. It also
asserts `assert_ne!(dup, text, "splice must apply")`, so a splice that changes nothing fails clearly.

---

_Fixed: 2026-10-13T00:26:22Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_

## Author decision (2026-10-15)

Recorded at the v0.5 wrap-up.

Remaining Info findings in 17-REVIEW.md (IN-01, IN-03 to IN-08) are accepted as tech debt.
