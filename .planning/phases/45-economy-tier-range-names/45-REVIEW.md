---
phase: 45-economy-tier-range-names
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/pricer.rs
  - src/emitter.rs
  - src/dynamic.rs
  - src/overlay/app.rs
  - tests/ui/test_ff.filter
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
status: issues_found
---

# Phase 45: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Reviewed the diff `93a0320..HEAD`: `pricer::tier_ranges` / `fmt_amount` / `chaos_floor`,
the new `PricedData::tier_ranges` field, `emitter::economy_name` at both tagged sites, the
updated tests in `emitter.rs`, `dynamic.rs` and `overlay/app.rs`, and the UI fixture.

The wiring is sound. Ranges come from the cutoffs and never from prices. They are sorted
by chaos floor exactly as `normalize_cutoffs` sorts them, so a label always matches the
tier `assign_tier` actually picks, mixed-unit files included (the sort is stable and both
sides use it). `generate` output is unchanged because `tags == false` never reads the map.
`ValidatedTiers` guarantees all seven tiers, so the plain-name fallback is reached only
with an empty map. The pending-tier preview path gets renamed rows because `price_tiers`
always rebuilds the map. `config_state::parse` already refuses non-finite and negative
cutoffs, so the existing `partial_cmp().unwrap()` cannot meet a NaN from the editor.

The one real defect: the 2-decimal formatting can make a label state a range that is
wrong for low cutoffs, which are realistic in exalt.

## Warnings

### WR-01: 2-decimal rounding makes labels wrong (or `< 0`) for sub-cent cutoffs

**File:** `src/pricer.rs:251-259` (`fmt_amount`), used at `src/pricer.rs:270-275`
**Issue:** `format!("{v:.2}")` rounds to the nearest hundredth, so the printed bound can
move to the wrong side of the real cutoff:
- Cutoff `0.025` exalt prints `0.03` (0.025 is stored as 0.02500000000000000139). With E
  at 0.025 exalt, the F label reads `< 0.03 exalt`, but an item at 0.027 exalt is in E,
  not F.
- Cutoff `0.005` prints `0.01`, and a top tier at 1.999 prints `>= 2`, so an item at
  1.9995 sits in that tier while its label says it needs 2.
- Cutoff `0.001` (or anything below 0.005) prints `0`, which gives `F tier Currency < 0
  exalt`, a range nothing can satisfy.

The fallback pair already goes down to 0.05 exalt, and the tier editor accepts any finite
value >= 0 (`config_state.rs:868-869`), so a user lowering E or F to 0.025 or 0.005 exalt
gets wrong names. The "at most 2 decimals" rule in CONTEXT.md is the author's, but it
assumed cutoffs that are already exact to 2 decimals. The `1.999 -> "2"` row in
`fmt_amount_table` pins this behaviour as intended.
**Fix:** keep 2 decimals when they are exact, and fall back to enough significant digits
otherwise, so the label never misstates the cutoff:
```rust
pub fn fmt_amount(v: f64) -> String {
    let trim = |s: String| if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else { s };
    let two = trim(format!("{v:.2}"));
    if (two.parse::<f64>().unwrap_or(f64::NAN) - v).abs() < 1e-9 {
        return two;
    }
    // Not representable in 2 decimals: shortest exact form, capped at 6 decimals.
    trim(format!("{v:.6}"))
}
```
Or validate in the tier editor that a cutoff has at most 2 decimals. Either way, replace
the `1.999 -> "2"` case in the test with an exactness case (`0.025 -> "0.025"`). Raise this
with the author, since it touches a decision in CONTEXT.md.

## Info

### IN-01: `-0` cutoff prints `-0`

**File:** `src/pricer.rs:251`
**Issue:** The editor accepts `-0` (`-0.0 >= 0.0` is true) and serde reads `-0` from
`tiers.json`. `format!("{:.2}", -0.0)` gives `-0.00`, which trims to `-0`, so the label
reads `>= -0 chaos` / `< -0 chaos`.
**Fix:** Normalise first: `let v = if v == 0.0 { 0.0 } else { v };`.

### IN-02: Mixed-unit files label a tier with its neighbour's unit

**File:** `src/pricer.rs:273-274`
**Issue:** `< {up.min} {up.unit}` uses the upper tier's unit. In a mixed file (S/A in
`div`, B-F in `chaos`, which `ValidatedTiers` allows) B reads `B tier ... < 1 div` even
though B is entered in chaos. This is correct and consistent, but CONTEXT.md says "the
tiers' own unit word", so it should be confirmed with the author. Also, `div` can now show
up even though the decision only names `chaos` / `exalt`.
**Fix:** Confirm the intent and add a mixed-unit case to the `tier_ranges` tests so the
behaviour is pinned.

### IN-03: Unique labels describe the best unique on the base, not every highlighted item

**File:** `src/emitter.rs:649-654`
**Issue:** `A tier Uniques < 30 chaos` is true for the base's most valuable unique (the
base's tier is the highest of its uniques), but the block highlights every unique on those
bases, cheap ones included. The new label makes the existing BaseType caveat look more
precise than it is.
**Fix:** No code change needed. Mention it next to the existing unique caveat if the
author wants.

### IN-04: `useless_format` in a test

**File:** `src/emitter.rs:1356`
**Issue:** `let prefix = format!("# ff:name ");` has no arguments. Clippy flags it
(`clippy::useless_format`), and the toolchain ships clippy.
**Fix:** `let prefix = "# ff:name ";`.

### IN-05: `fmt_amount` is `pub` with no outside caller

**File:** `src/pricer.rs:251`
**Issue:** It is only used inside `pricer.rs`, so `pub` widens the crate API for no reason.
**Fix:** Make it private (or `pub(crate)`).

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
