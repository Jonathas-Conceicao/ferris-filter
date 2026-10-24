---
phase: 45-economy-tier-range-names
fixed_at: 2026-10-23T00:00:00Z
review_path: .planning/phases/45-economy-tier-range-names/45-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 2
skipped: 1
status: partial
---

# Phase 45: Code Review Fix Report

**Fixed at:** 2026-10-23
**Source review:** .planning/phases/45-economy-tier-range-names/45-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (WR-01, plus IN-04 and IN-05 added by the orchestrator)
- Fixed: 2
- Skipped: 1

`cargo test --locked` is green after the fixes (1710 lib tests plus the
integration suites). No fixture changed and no dependency was added.

## Fixed Issues

### IN-04: `useless_format` in a test

**Files modified:** `src/emitter.rs`
**Commit:** 40c5db3
**Applied fix:** In `gem_background_override_categories_carry_the_range`,
`let prefix = format!("# ff:name ");` became `let prefix = "# ff:name ";`, and
`l.starts_with(&prefix)` became `l.starts_with(prefix)`.

### IN-05: `fmt_amount` is `pub` with no outside caller

**Files modified:** `src/pricer.rs`
**Commit:** d86031c
**Applied fix:** `pub fn fmt_amount` is now a private `fn fmt_amount`. It is
called only from `tier_ranges` and the `fmt_amount_table` test, both in
`pricer.rs`.

## Skipped Issues

### WR-01: 2-decimal rounding makes labels wrong (or `< 0`) for sub-cent cutoffs

**File:** `src/pricer.rs:250-259`
**Reason:** skipped: conflicts with locked author decision; deferred to author.
CONTEXT.md fixes the rule "Numbers: at most 2 decimals, trailing zeros
trimmed". Changing the formatter, or making the tier editor accept at most 2
decimals, is the author's call.
**Original issue:** `format!("{v:.2}")` can print a bound on the wrong side of
the real cutoff (0.025 prints `0.03`, 1.999 prints `2`), and a cutoff below
0.005 prints `0`, which gives an impossible `< 0` label.

## Deferred (Info, out of scope, left unfixed)

- **IN-01:** a `-0` cutoff prints `-0` (`src/pricer.rs`).
- **IN-02:** in a mixed-unit file a tier's label uses the unit of the tier
  above it. Needs the author to confirm, then a pinning test.
- **IN-03:** a unique label describes the best unique on the base, not every
  unique highlighted on it. Documentation only.

---

_Fixed: 2026-10-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
