---
status: passed
phase: 03-pricing-layer
verified: 2026-05-25
method: human-authoritative (D-10 diff review) + guardrail test + full automated suite
requirements: [GEN-05, TIER-01, TIER-02, TIER-03, TIER-04]
---

# Phase 3: Pricing Layer — Verification

**Status: PASSED**

Per CONTEXT.md D-10, the one-time re-baseline required human sign-off. The user reviewed the
`universal.filter` diff (Phase 2 → Phase 3) and approved it ("the changes as presented are
approved"; in-game spot-check waived). The automated suite + guardrail corroborate.

## ROADMAP success criteria

1. **Price-derived tiers for currency + uniques.** ✓ — `generate` reads `prices.json`, derives
   cross-rates from the Exalt/Chaos/Divine anchors, normalizes to chaos, and assigns tiers via
   `tiers.json` cutoffs (`src/pricer.rs`). The committed re-baseline shows the three reference orbs
   re-tiered (Divine S→A, Chaos C→E, Exalt C→D) — verified by the `currency_retiering_present` test.
2. **Unique base-collision → highest tier.** ✓ — `unique_base_collision_uses_highest_tier` fixture
   test (Headhunter B + Darkness Enthroned D on Heavy Belt → one block at B). Capability is built but
   dormant (committed `prices.json` has no uniques → no unique blocks, GEN-04 intact).
3. **No `prices.json` → warn + valid filter, no crash (GEN-05).** ✓ — `load_prices_optional`
   (`Path::exists()` first) → stderr warning + `PricedData::empty()` → only config-driven blocks; no crash.
4. **Abort on null/zero Exalt/Chaos/Divine (TIER-04).** ✓ — `PriceRateError::NullOrZero` names the
   offending currency; 6 tests (null + zero for each anchor); wired into `cmd_generate` before tiering.

## Requirements coverage

- **GEN-05** — no-prices warn path (03-03). ✓
- **TIER-01** — cross-rate normalize-to-chaos, anchors from items[0..2] (03-02). ✓
- **TIER-02** — cutoff assignment, inclusive ≥, boundary-tested (03-02). ✓
- **TIER-03** — unique price-tiering + base-collision→highest, fixture-tested, dormant seed (03-02/03-01). ✓
- **TIER-04** — abort naming the offending reference currency (03-02/03-03). ✓

## Architecture outcome

- `generate` no longer reads `currency.json` (D-01) — `rg "currency.json" src/main.rs` returns nothing.
  `currency.json` remains in-repo as a reference/seed only.
- `prices.json` is now the single value source (358 seeded items; first 3 = Exalt/Chaos/Divine;
  class+category per item; zero uniques) — ready for Phase 4 `sync` to populate from poe.ninja.
- New `src/pricer.rs` (cross-rate, tier assignment, grouping, unique collision, TIER-04 abort) +
  `src/bin/seed_prices.rs` (one-time migration).

## Re-baseline guardrail (D-09/D-10)

`git diff 89547c8 HEAD -- universal.filter` is limited to exactly 5 BaseType-list edits in the
`Currency` section (Divine S→A, Chaos C→E, Exalt C→D). All 14 other categories + gear/flask/recycle
blocks are byte-identical; block count unchanged (83 Show + 2 Hide). Pinned by the
`currency_retiering_present` test. Human-reviewed via `03-04-REBASELINE-DIFF.md` and approved.

Test-isolation bug fixed: `generate_in_repo_root_succeeds` now writes to a temp `--output`, so test
runs no longer clobber the committed `universal.filter`; `artifact_guard` guards the committed file.

Full suite green: 53 unit + 4 cli_errors + 8 cli_generate.

## Notes for Phase 4 (Sync)

`sync` must write `prices.json` in this schema (incl. populating `class`/`category` from poe.ninja's
categorization or a static base→meta map) and may add valuable uniques, which will activate the
dormant unique-block emission. Spike the poe.ninja PoE2 API surface before writing HTTP code.
