---
status: passed
phase: 02-static-generate
verified: 2026-05-24
method: human-authoritative (D-13 in-game smoke test) + automated suite
requirements: [GEN-01, GEN-02, GEN-03, GEN-04]
---

# Phase 2: Static Generate — Verification

**Status: PASSED**

Per CONTEXT.md D-13, the manual in-game smoke test was the *authoritative* equivalence gate for
the re-baseline. The user loaded the regenerated `universal.filter` in PoE2 and approved it
("the filter is working great"), and approved both D-14 review diffs (config.json + universal.filter).
The automated suite below corroborates.

## ROADMAP success criteria

1. **`generate` reproduces the established structure (snapshot passes).** ✓ — `cargo test` runs the
   insta snapshot test locking the full generator output; the artifact-guard test asserts the
   committed `universal.filter` byte-equals the generator output (CRLF-normalized). 75 tests pass.
2. **Each block carries a model-path comment tag (GEN-03).** ✓ — tags present in the established
   format (e.g. `[alwaysShow.recycle -> styles.Recycle]`, `[areaLevelBands "6-25".weapons -> styles.unchanged]`).
3. **No-rule items fall through (GEN-04).** ✓ — `grep -c "Rarity == Unique" universal.filter` = 0;
   `grep -c "Class.*Jewels"` = 0. No blocks for uniques/gems/jewels/jewellery/quest/waystones/trials/unidentified.
4. **Determinism (GEN-01).** ✓ — `generate_is_deterministic` test asserts two emitter runs produce
   identical output; emission uses Vec / named-struct traversal only (no HashMap iteration after the
   D-11 refactor).

## Requirements coverage

- **GEN-01** (deterministic compile) — typed models (Styles struct, RarityRule enum), determinism test. ✓
- **GEN-02** (reproduce established structure) — all 83 Show + 2 Hide blocks emitted; snapshot + artifact guard. ✓
- **GEN-03** (model-path comment per block) — present. ✓
- **GEN-04** (no-rule items fall through) — verified 0 unique/jewel blocks. ✓

## Re-baseline notes (D-01/D-02)

The regenerated `universal.filter` differs from the original hand-built file only by:
- header source label (`filter-model.json` → `config.json`), and
- the Recycle block's action order normalized to canonical (semantically irrelevant to the engine).

`config.json` changed once: tier-C `beam: null` → `Yellow`, reconciling the model to the filter's
existing `PlayEffect Yellow` on tier-C blocks (filter output unchanged for C). Both diffs reviewed
and approved by the user.

## Notes for Phase 3

The snapshot test established here is the regression oracle for Phase 3's price-driven rework:
when `generate` is reworked to derive tiers from `prices.json` and stops reading `currency.json`,
this same snapshot MUST still pass — mechanically enforcing "output does not change."
