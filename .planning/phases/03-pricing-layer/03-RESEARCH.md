# Phase 3: Pricing Layer - Research

**Researched:** 2026-05-24
**Domain:** Rust pricing math, tier assignment, emitter refactor, insta snapshot workflow
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `generate` no longer reads `currency.json`. Currency/category tiers are computed from
  `prices.json`. `currency.json` stays in the repo as a **reference/seed only** — never read at
  generate-time.
- **D-02:** No-`prices.json` behavior (GEN-05): if `prices.json` is absent, `generate` prints a
  warning to stderr and emits a valid filter containing only the config-driven blocks (gear
  AreaLevel bands, flask/charm ladder, recycle, Hide fallbacks) — **no currency/category blocks**.
  No crash.
- **D-03:** Each `prices.json` item: `{ name, baseType, class, category, exalt, chaos, div }`.
  `exalt`/`chaos`/`div` may be null. The first three items remain **Exalt, Chaos, Divine** (SYNC-02
  convention) and double as the cross-rate anchors.
- **D-04:** Group items by `category`; emit category blocks in **order of first appearance** in the
  items array. Within a category, emit tiers **S→F**, only populated tiers.
- **D-05:** **Cross-rate (TIER-01):** derive the div→chaos (and exalt→chaos) rates from the first
  three entries (Exalt/Chaos/Divine); normalize every item's value to **chaos** (the reference unit).
- **D-06:** **Tier assignment (TIER-02):** an item's tier = the highest `tiers.json` cutoff whose
  (chaos-normalized) lower bound it meets, using **inclusive ≥** comparison.
- **D-07:** **Abort on bad reference rate (TIER-04):** if Exalt, Chaos, or Divine has a null or zero
  price in `prices.json`, `generate` aborts with a clear error **naming the offending currency**.
  Reuse the miette/thiserror diagnostic style.
- **D-08:** Implement unique price-tiering now: a priced unique is tiered by its price; when multiple
  priced uniques **share a base type**, the emitted block uses the **highest tier** among them.
  Seed the committed `prices.json` with **NO uniques** → zero unique blocks → Phase 2 output
  preserved (aside from D-09). Prove unique logic + collision with a **separate fixture test**.
- **D-09:** Non-reference bases get their tier's lower-bound value. The three reference orbs get
  realistic values (Chaos=1c; Exalt≈4.5c; Divine≈220c). This **re-tiers**: Chaos C→E, Exalt C→D,
  Divine S→A. Accepted, deliberate small re-baseline.
- **D-10:** Phase 2 insta snapshot is updated ONCE to the new (re-baselined) output via `cargo insta
  review` and re-committed; thereafter it is the hard regression oracle. Produce a human-review
  diff. No changes beyond the anchor re-tiering and its direct block consequences are permitted
  without flagging.

### Claude's Discretion

- Cross-rate math details (handling items priced only in exalt/div vs chaos; which source wins).
- TIER-04 exact error wording.
- Unique block section placement/ordering.
- Emitter refactor structure (currency blocks now sourced from `prices.json` grouped by category +
  computed tier).
- How the seeded `prices.json` is generated from `currency.json` (one-time migration script vs
  hand-authored fixture).

### Deferred Ideas (OUT OF SCOPE)

- poe.ninja `sync` populating `prices.json` → Phase 4.
- Actually emitting unique highlight blocks in committed output → activates when `sync` provides
  priced uniques (capability + fixture test built in Phase 3, but seeded empty).
- Tier-pin / manual override for iconic items — pure price-driven chosen.
- How `sync` maps poe.ninja → class/category → Phase 4 discussion.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GEN-05 | `generate` runs without `prices.json` present — warns and skips price-tiered rules rather than failing | D-02 behavior; load path returns `Option<PricesFile>`, missing file = warn + skip section |
| TIER-01 | Derive currency cross-rates from first three `prices.json` entries (Exalt, Chaos, Divine) and convert every priced item to a chaos value | Cross-rate algorithm documented below; chaos is anchor |
| TIER-02 | Assign each priced item to a tier per `tiers.json` cutoffs (chaos as reference unit) | Cutoff normalization + inclusive ≥ assignment documented below |
| TIER-03 | Valuable uniques present in `prices.json` are highlighted by their price tier; uniques sharing a base type resolve to highest tier; untracked uniques fall through | Base-collision algorithm + unique block structure documented below |
| TIER-04 | `generate` aborts with clear error if Exalt/Chaos/Divine has null or zero price | Abort check after loading reference items 0..2; new `PriceRateError` diagnostic |

</phase_requirements>

---

## Summary

Phase 3 reworks the `generate` pipeline so the **currency/category section** is driven by
`prices.json` + `tiers.json` instead of `currency.json`. The core mathematical work is a
two-step process: (1) derive chaos-equivalent rates from the three anchor items, then (2) map
every priced item's chaos value to the highest cutoff it meets. The emitter's
`emit_currency_sections` function receives a new grouped, pre-tiered data structure instead of
`CurrencyData`, but all other sections (gear bands, flask/charm, recycle) are untouched.

The critical acceptance condition is that a seeded `prices.json` must reproduce the Phase 2
`universal.filter` output **except** for the three anchor orbs re-tiering (Chaos C→E, Exalt C→D,
Divine S→A). This has been **verified by computation** against the actual `currency.json` and
`tiers.json` — only those three Currency-category bases change; all 14 other categories are
unaffected because their non-anchor bases are seeded at tier lower-bound values that reproduce
their curated tiers exactly.

The unique-tiering capability (TIER-03) is built and fixture-tested this phase but produces zero
output blocks because the committed `prices.json` contains no uniques. The unique blocks activate
when Phase 4's `sync` populates them.

**Primary recommendation:** Add a `src/pricer.rs` module that implements `derive_rates`,
`normalize_to_chaos`, `assign_tier`, and `group_by_category`. The emitter takes the result of
`pricer::price_tiers(&PricesFile, &ValidatedTiers)` — a `Vec<CategoryBlock>` — instead of
`&CurrencyData`. `main.rs` loads `prices.json` optionally, calls the pricer, then passes results
to the emitter.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Load prices.json (optional) | `loader.rs` | `main.rs` | Reuse `load_json`; wrap in `Option`, not error, for absence |
| Reference-rate extraction + TIER-04 abort | New `pricer.rs` | `main.rs` (wiring) | Pure math; abort error surfaced before emitter |
| Item→chaos normalization (TIER-01) | `pricer.rs` | — | Deterministic, no I/O |
| Tier assignment (TIER-02) | `pricer.rs` | — | Consumes `ValidatedTiers`; inclusive ≥ per cutoff |
| Base-collision for uniques (TIER-03) | `pricer.rs` | — | Group by baseType, keep highest tier |
| Grouped/tiered data structure | `pricer.rs` (returns `Vec<CategoryBlock>`) | — | Feeds emitter without HashMap |
| Currency/unique block emission | `emitter.rs` (refactored) | — | New signature takes `&[CategoryBlock]` |
| Gear/flask/recycle emission | `emitter.rs` (unchanged) | — | Untouched by Phase 3 |
| prices.json schema extension | `models/prices.rs` | — | Add `class`, `category` fields to `PriceItem` |
| Seeded prices.json generation | One-time migration (see below) | — | Produces committed artifact from currency.json |
| Snapshot oracle update | `tests/cli_generate.rs` + insta | — | One-time `INSTA_UPDATE=always` run + commit |

---

## Standard Stack

### Core (no new dependencies needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde + serde_json | 1.0 (existing) | `prices.json` schema extension | Already in Cargo.toml |
| miette + thiserror | 7.6 / 2.0 (existing) | TIER-04 abort diagnostic | Matches established error pattern in `error.rs` |
| insta | 1.47.2 (existing) | Snapshot oracle update | Already in `[dev-dependencies]` |

No new crate dependencies are introduced. All Phase 3 work fits within the existing dependency set.

**Version verification:** All versions confirmed from `Cargo.toml` and `Cargo.lock` in the repo.
[VERIFIED: Cargo.lock grep, 2026-05-24]

---

## Architecture Patterns

### System Architecture Diagram

```
prices.json (optional)   tiers.json          config.json
      |                      |                    |
      v                      v                    v
 loader::load_json      loader::load_tiers   loader::load_json
 (Option<PricesFile>)   (ValidatedTiers)     (Config)
      |                      |                    |
      +----------+-----------+                    |
                 |                                |
                 v                                |
        pricer::price_tiers()                     |
        [TIER-04 abort if anchors null/zero]      |
        [derive div/exalt rates]                  |
        [normalize each item to chaos]            |
        [assign tier per cutoffs]                 |
        [group by category, tiers S->F]           |
        [unique base-collision -> highest]        |
                 |                                |
                 v                                v
        Vec<CategoryBlock>              (gear/flask/recycle data)
                 |                                |
                 +----------+-----------+---------+
                            |
                            v
                    emitter::generate()
                    [header]
                    [recycle]          <- config only
                    [gear bands]       <- config only
                    [flask/charm]      <- config only
                    [currency/uniques] <- CategoryBlock vec
                            |
                            v
                    universal.filter (String, LF-only)
```

### Recommended Project Structure (additions only)

```
src/
├── pricer.rs           # NEW: cross-rate math, tier assignment, grouping
├── emitter.rs          # MODIFIED: new currency emission path
├── models/
│   └── prices.rs       # MODIFIED: add class + category to PriceItem
├── loader.rs           # MODIFIED: add load_prices_optional()
└── main.rs             # MODIFIED: wire pricer into cmd_generate
```

### Pattern 1: Cross-Rate Derivation (TIER-01)

**What:** Extract chaos-equivalent rates from the first three `prices.json` items. Items[0]=Exalt,
Items[1]=Chaos, Items[2]=Divine (positional convention from D-03/SYNC-02).

**Algorithm (Claude's Discretion — recommended):**

The anchor items give us three fixed reference values:
- `chaos_rate`: `items[1].chaos` = 1.0 (Chaos Orb is the reference unit, value = 1c exactly).
- `exalt_rate`: `items[0].chaos` (how many chaos per 1 exalt).
- `div_rate`: `items[2].chaos` (how many chaos per 1 div). Also computable from `items[2].div = 1.0`,
  but `chaos` field is more direct.

TIER-04 abort check: any of these three yielding `None` or `Some(0.0)` is a hard error.

```rust
// Source: derived from tiers.json + prices.json schema (verified, 2026-05-24)
pub struct CrossRates {
    pub exalt_in_chaos: f64,   // chaos per 1 exalt
    pub div_in_chaos: f64,     // chaos per 1 div
}

pub fn derive_rates(items: &[PriceItem]) -> Result<CrossRates, PriceRateError> {
    // items[0]=Exalt, items[1]=Chaos, items[2]=Divine
    let exalt_chaos = items.get(0)
        .and_then(|i| i.chaos)
        .ok_or(PriceRateError::NullOrZero("Exalt"))?;
    let chaos_chaos = items.get(1)
        .and_then(|i| i.chaos)
        .ok_or(PriceRateError::NullOrZero("Chaos"))?;
    let div_chaos = items.get(2)
        .and_then(|i| i.chaos)
        .ok_or(PriceRateError::NullOrZero("Divine"))?;
    if exalt_chaos == 0.0 { return Err(PriceRateError::NullOrZero("Exalt")); }
    if chaos_chaos == 0.0 { return Err(PriceRateError::NullOrZero("Chaos")); }
    if div_chaos == 0.0   { return Err(PriceRateError::NullOrZero("Divine")); }
    Ok(CrossRates { exalt_in_chaos: exalt_chaos, div_in_chaos: div_chaos })
}
```

**Normalizing any item to chaos** (priority order for multi-field items):

```rust
// Preference: chaos > div > exalt (most direct first)
pub fn normalize_to_chaos(item: &PriceItem, rates: &CrossRates) -> Option<f64> {
    if let Some(c) = item.chaos { return Some(c); }
    if let Some(d) = item.div   { return Some(d * rates.div_in_chaos); }
    if let Some(e) = item.exalt { return Some(e * rates.exalt_in_chaos); }
    None // no usable price → skip this item
}
```

This preference order (chaos > div > exalt) is **Claude's Discretion** — justified because: (a)
chaos is the filter's reference unit with no conversion error, (b) div prices on poe.ninja are
direct market observations, (c) exalt is least reliable as many items are never priced in exalt.
Items with no field populated (all null) emit no tier block.

### Pattern 2: Tier Assignment (TIER-02)

**What:** Given an item's chaos value and the (already-loaded) `ValidatedTiers`, find the highest
cutoff (i.e., most valuable tier letter) that the item meets or exceeds.

**Cutoff normalization:** `tiers.json` mixes `"unit": "chaos"` and `"unit": "div"` entries.
Convert all cutoffs to chaos once, upfront, using `div_in_chaos`:

```rust
// Source: derived from tiers.json schema (verified, 2026-05-24)
fn normalize_cutoffs(tiers: &ValidatedTiers, rates: &CrossRates) -> Vec<(String, f64)> {
    // Returns (tier_letter, min_chaos) for all 7 tiers, sorted S->F (highest min first)
    let mut normalized: Vec<(String, f64)> = tiers.0.iter().map(|e| {
        let min_chaos = match e.unit {
            Unit::Chaos => e.min,
            Unit::Div   => e.min * rates.div_in_chaos,
        };
        (e.tier.clone(), min_chaos)
    }).collect();
    // Sort descending by min_chaos so we check highest first
    normalized.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    normalized
}

pub fn assign_tier(chaos_value: f64, cutoffs: &[(String, f64)]) -> Option<String> {
    // Inclusive >= : item gets the highest tier whose floor it meets
    for (tier, min_chaos) in cutoffs {
        if chaos_value >= *min_chaos {
            return Some(tier.clone());
        }
    }
    None // below F floor (0.1c) → no block emitted
}
```

**Computed cutoffs** for the seeded `prices.json` (div_rate = 220c):

| Tier | Raw cutoff | Chaos-normalized |
|------|-----------|-----------------|
| S | 5 div | 1100c |
| A | 1 div | 220c |
| B | 30c | 30c |
| C | 10c | 10c |
| D | 2c | 2c |
| E | 0.5c | 0.5c |
| F | 0.1c | 0.1c |

**Why inclusive ≥ is critical:** Seeded lower-bound values (e.g. D-tier items seeded at 2.0c)
must land exactly in their tier. `>` would miss the floor; `>=` is correct and required by D-06.

### Pattern 3: Grouping and Category Structure (D-04)

**What:** Transform a flat `Vec<PriceItem>` (with computed tiers) into a `Vec<CategoryBlock>`
that preserves category first-appearance order and tier S→F order within.

**Key constraint:** Use `Vec`-based tracking (no `HashMap`) to maintain deterministic first-
appearance order:

```rust
// Source: derived from D-03/D-04 decisions (verified, 2026-05-24)
pub struct TieredItem {
    pub base_type: String,
    pub class: String,
}

pub struct TierGroup {
    pub tier: String,  // "S", "A", ... "F"
    pub items: Vec<TieredItem>,
}

pub struct CategoryBlock {
    pub name: String,
    pub tiers: Vec<TierGroup>,  // populated S→F only
}

pub fn group_by_category(
    items: &[PriceItem],
    cutoffs: &[(String, f64)],
    rates: &CrossRates,
) -> Vec<CategoryBlock> {
    let mut category_order: Vec<String> = Vec::new();
    let mut category_map: std::collections::HashMap<String, HashMap<String, Vec<TieredItem>>> = ...;
    // HOWEVER: inner grouping iterates in S->F order from known tier order
    // To avoid non-deterministic HashMap iteration at emit time:
    // store as Vec<(tier, Vec<TieredItem>)> sorted by tier_order
}
```

**Recommended implementation:** Store `HashMap<category, HashMap<tier, Vec<TieredItem>>>` during
accumulation but convert to `Vec<CategoryBlock>` with `Vec<TierGroup>` at the end, iterating
`["S", "A", "B", "C", "D", "E", "F"]` in fixed order. The only HashMap in this pattern is
temporary; the emitter only ever sees `Vec<CategoryBlock>`. This eliminates the nondeterminism
footgun.

**Item ordering within a tier group:** Preserves the order of first appearance in the `items`
array. When iterating `items` linearly and appending to `tier_map[category][tier]`, the insertion
order is deterministic.

### Pattern 4: TIER-04 Abort (new error type)

**What:** A new `thiserror`/`miette` error type that names the offending reference currency.

```rust
// Source: existing error.rs pattern (verified, 2026-05-24)
#[derive(Debug, Error, Diagnostic)]
pub enum PriceRateError {
    #[error("prices.json: reference currency '{0}' has a null or zero price — \
             cannot derive cross-rates (run `ferrisfilter sync` to refresh prices.json)")]
    #[diagnostic(code(ferrisfilter::prices::bad_rate))]
    NullOrZero(&'static str),
}
```

This can live in `src/pricer.rs` or `src/error.rs` (either works; pricer.rs is cleaner since
it's the only caller).

**Where to call:** In `cmd_generate` in `main.rs`, immediately after loading `prices.json` and
before calling the emitter. The error is surfaced as a `miette::Result` abort; the filter is
never partially written.

### Pattern 5: Unique Block Structure (TIER-03)

**What:** Uniques in `prices.json` have `name` ≠ `baseType` (e.g. `name="Headhunter"`,
`baseType="Heavy Belt"`). Multiple uniques sharing a `baseType` get the highest-tier block.

**Block shape:** Emitted **after** currency/category blocks (unique items are not grouped by
`category` in the same sense as currency — they go in a dedicated uniques section, or appended
per the section order Claude decides). Recommended: emit uniques as a separate section at the
end of the currency region, after all category blocks.

```
# --- Uniques ---
Show # Unique <tier>
  Rarity == Unique
  BaseType == "<baseType1>" "<baseType2>"
  <style actions for tier>
```

**Base-collision algorithm:**

```rust
// Source: D-08 decision (verified, 2026-05-24)
// For each unique item in prices.json:
//   chaos_val = normalize_to_chaos(item, rates)
//   tier = assign_tier(chaos_val, cutoffs)
//   base_type = item.base_type.as_deref().unwrap_or(&item.name)
// Group by base_type -> keep the highest tier (max)
// Emit one block per tier with all base_types that landed in that tier
```

Tier ordering for "highest": S > A > B > C > D > E > F. Compare by index in `["S","A","B","C","D","E","F"]`, lower index = higher tier.

**Placement in the filter:** After all category (currency) blocks. Unique blocks should not
interleave with currency category blocks because unique drops don't belong to the currency
category hierarchy. A header comment `# --- Uniques (price-tiered) ---` precedes them.

**When seeded `prices.json` has zero uniques:** The uniques section emits nothing (no header,
no blocks). The emitter only emits the section if there are any priced uniques to show.

**Existing GEN-04 compatibility:** The current test `generate_no_unique_or_jewels_blocks`
asserts `!output.contains("Rarity == Unique")`. With no seeded uniques, this test continues to
pass. The fixture test for TIER-03 uses a **separate, non-committed prices.json fixture** in the
test body (inline JSON or a `tests/fixtures/` file), not the committed file.

### Anti-Patterns to Avoid

- **HashMap iteration for block emission:** Using `HashMap<String, ...>` and iterating it at
  emit time produces non-deterministic order. Use `Vec<CategoryBlock>` only.
- **Float formatting for comparison:** Never use `format!("{}", f64)` and compare the result.
  Use direct `>=` comparison on `f64` values. Seeded values (1.0, 4.5, 220.0, 0.1, 0.5, etc.)
  are all exactly representable in IEEE 754 binary64, so `>=` is stable.
- **Checking for `prices.json` absence by catching a parse error:** Use `std::path::Path::exists()`
  or `fs::metadata()` before attempting the load. Distinguish "file absent" (warn + skip) from
  "file present but malformed" (abort with error).
- **Mutating the committed `prices.json` in fixture tests:** The snapshot test and artifact-guard
  test run against the committed `prices.json`. Fixture tests for TIER-03 must use an inline or
  test-local fixture, not `--output` path tricks that affect the committed file.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Field-path-aware JSON parse errors | Custom error parser | `serde_path_to_error` (existing) | Already in use; gives dotted field path in errors |
| Diagnostic formatting | Custom error printer | `miette` + `thiserror` (existing) | Established pattern in `error.rs`; graphical renderer for free |
| Snapshot regression | String comparison in tests | `insta::assert_snapshot!` (existing) | Auto-managed `.snap` file; review workflow built in |
| Float-to-string for display in errors | `format!("{:.2}", val)` everywhere | Inline in the error message via `thiserror` `#[error]` | Less formatting code, consistent |

---

## Critical Finding: Exact Re-Baseline Enumeration

This is the most important verification output. **Computed directly from `currency.json` and `tiers.json`** using the D-09 seed values (Chaos=1c, Exalt=4.5c, Divine=220c, div_rate=220c).

### What Changes in universal.filter (5 block modifications, all in "Currency" category only)

**Block: "Currency S"**
- Phase 2 BaseType list: `"Perfect Chaos Orb" "Mirror of Kalandra" "Hinekora's Lock" "Perfect Exalted Orb" **"Divine Orb"** "Fracturing Orb" "Albino Rhoa Feather" "Vaal Cultivation Orb"` (8 bases)
- Phase 3 BaseType list: same without `"Divine Orb"` (7 bases)
- Change: Divine Orb removed (re-tiered to A; 220c = exactly A lower bound)

**Block: "Currency A"**
- Phase 2 BaseType list: `"Perfect Jeweller's Orb" "Core Destabiliser" "Vaal Infuser" "Architect's Orb" "Ancient Infuser" "Crystallised Corruption"` (6 bases)
- Phase 3 BaseType list: `**"Divine Orb"** "Perfect Jeweller's Orb" "Core Destabiliser" "Vaal Infuser" "Architect's Orb" "Ancient Infuser" "Crystallised Corruption"` (7 bases)
- Change: Divine Orb prepended (it appears at position index 4 in the seeded items array — after S-tier items and before A-tier items, so within the A tier group it is the first item)

**Block: "Currency C"**
- Phase 2 BaseType list: `**"Chaos Orb"** "Orb of Chance" **"Exalted Orb"** "Greater Exalted Orb" "Perfect Orb of Augmentation"` (5 bases)
- Phase 3 BaseType list: `"Orb of Chance" "Greater Exalted Orb" "Perfect Orb of Augmentation"` (3 bases)
- Change: Chaos Orb removed (re-tiered to E), Exalted Orb removed (re-tiered to D)

**Block: "Currency D"**
- Phase 2 BaseType list: `"Arcanist's Etcher" "Armourer's Scrap" "Orb of Alchemy" "Perfect Orb of Transmutation" "Greater Regal Orb" "Glassblower's Bauble" "Gemcutter's Prism" "Chance Shard" "Vaal Orb"` (9 bases)
- Phase 3 BaseType list: `**"Exalted Orb"** "Arcanist's Etcher" "Armourer's Scrap" "Orb of Alchemy" "Perfect Orb of Transmutation" "Greater Regal Orb" "Glassblower's Bauble" "Gemcutter's Prism" "Chance Shard" "Vaal Orb"` (10 bases)
- Change: Exalted Orb prepended (4.5c >= 2c cutoff; appears at its position in the items array, before the D-seeded items, so it is first in the D tier group)

**Block: "Currency E"**
- Phase 2 BaseType list: `"Blacksmith's Whetstone" "Lesser Jeweller's Orb" "Greater Jeweller's Orb" "Artificer's Orb"` (4 bases)
- Phase 3 BaseType list: `**"Chaos Orb"** "Blacksmith's Whetstone" "Lesser Jeweller's Orb" "Greater Jeweller's Orb" "Artificer's Orb"` (5 bases)
- Change: Chaos Orb prepended (1c >= 0.5c cutoff; appears at its position in the items array, before the E-seeded items, so it is first in the E tier group)

**Blocks B and F:** Unchanged.

### What Does NOT Change

- All other 14 categories (Abyssal Eyes, Ancient Bones, Breach, Delirium, Essences, Expedition,
  Idols, Incubators, Map Fragments & Pinnacle Keys, Omens, Runes, Soul Cores, Tablets, Vault Keys)
  are completely unchanged. [VERIFIED: computed from full currency.json, 2026-05-24]
- All gear bands, flask/charm bands, recycle block: completely unchanged (not touched by Phase 3).
- Block count: still 83 Show + 2 Hide = 85 blocks total (zero unique blocks with empty seed).
- The seeded prices.json has NO uniques, so GEN-04 (`no Rarity == Unique blocks`) still passes.

### Ordering of reference orbs within the modified tier groups

This matters for byte-identical output. In the seeded `prices.json`, items are in this order:
all S-curated items → all A-curated items → all B-curated items → all C-curated items → all
D-curated items → all E-curated items → all F-curated items.

- **Divine Orb** appears in the S-curated group (index 4 in Currency S, before Fracturing Orb).
  When re-tiered to A, it becomes the **first** item in the A-tier group (since it appears in the
  items array before all A-curated items).
- **Exalted Orb** appears in the C-curated group (index 2, after Chaos Orb). When re-tiered to D,
  it becomes the **first** item in the D-tier group (since it appears before all D-curated items).
- **Chaos Orb** appears in the C-curated group (index 0, first in C-curated). When re-tiered to E,
  it becomes the **first** item in the E-tier group.

[VERIFIED: computation above, 2026-05-24]

---

## Seeded prices.json Generation (D-09)

### Migration strategy: script in a separate plan task (recommended)

The seeded `prices.json` (~358 items) should be generated by a **one-time Rust script or test
helper** that reads `currency.json` and emits `prices.json`. This is preferable to hand-authoring
because:
1. Automation avoids transcription errors across 358 bases.
2. It documents the seeding logic (lower-bound values, reference orb overrides).
3. The script can be committed in `src/bin/seed_prices.rs` (excluded from main binary) or run
   inline in a migration test.

**Recommended approach:** A `src/bin/seed_prices.rs` binary (`[[bin]] name = "seed_prices"` in
Cargo.toml) that is run **once** at task time and whose output is committed. It does not become a
permanent part of the generate pipeline.

### Schema for the seeded items

For currency bases (where `name == baseType`):

```json
{
  "name": "Mirror of Kalandra",
  "baseType": "Mirror of Kalandra",
  "class": "Stackable Currency",
  "category": "Currency",
  "exalt": null,
  "chaos": 1100.0,
  "div": null
}
```

For non-Stackable-Currency entries (e.g. Augment class):

```json
{
  "name": "Amanamu's Gaze",
  "baseType": "Amanamu's Gaze",
  "class": "Augment",
  "category": "Abyssal Eyes",
  "exalt": null,
  "chaos": 1100.0,
  "div": null
}
```

**Reference orbs** use their realistic values:

```json
{ "name": "Chaos Orb",   "baseType": "Chaos Orb",   "class": "Stackable Currency", "category": "Currency", "exalt": null, "chaos": 1.0,   "div": null },
{ "name": "Exalted Orb", "baseType": "Exalted Orb", "class": "Stackable Currency", "category": "Currency", "exalt": null, "chaos": 4.5,   "div": null },
{ "name": "Divine Orb",  "baseType": "Divine Orb",  "class": "Stackable Currency", "category": "Currency", "exalt": null, "chaos": 220.0, "div": null }
```

The first three items in `prices.json` must be: `Exalted Orb` (chaos=4.5), `Chaos Orb` (chaos=1.0),
`Divine Orb` (chaos=220.0) — this is the SYNC-02/D-03 positional convention.

### prices.json model changes required

`PriceItem` in `src/models/prices.rs` currently has:
```rust
pub struct PriceItem {
    pub name: String,
    pub base_type: Option<String>,
    pub exalt: Option<f64>,
    pub chaos: Option<f64>,
    pub div: Option<f64>,
}
```

Phase 3 requires adding `class` and `category`, and making `base_type` **required** (not optional)
in the new schema. However, the existing 4-item `prices.json` fixture has no `class`/`category`
and the test `load_real_prices_json_parses` uses the real file — that file will be replaced by the
seeded one in this phase, so the model extension and file replacement happen together.

**Option:** Make `base_type`, `class`, `category` all `Option<String>` with `#[serde(default)]`
for backward compatibility with the existing 4-item fixture (avoids breaking the existing test
before it's updated). The pricer treats `None` class/category as "skip this item" or falls back
to `name`.

**Recommended:** Make `class` and `category` required (`String`, no Option) since all seeded
items will have them. The 4-item fixture is replaced atomically when the seeded file is committed.

---

## Common Pitfalls

### Pitfall 1: HashMap Iteration Order in Grouping

**What goes wrong:** Using `HashMap<String, HashMap<String, Vec<_>>>` for accumulation and then
iterating the outer HashMap to build `Vec<CategoryBlock>` produces non-deterministic output.
**Why it happens:** Rust's HashMap uses a randomized hasher; iteration order differs per run.
**How to avoid:** Accumulate with HashMap (for O(1) lookup), but reconstruct the output Vec by
iterating `category_order: Vec<String>` (tracked insertion order) and `TIER_ORDER: &[&str]`
(fixed `["S","A","B","C","D","E","F"]`). The HashMap is an implementation detail; the Vec drives emission.
**Warning signs:** Determinism test fails intermittently; block order in output differs between runs.

### Pitfall 2: IEEE 754 Float Comparison Edge Cases

**What goes wrong:** A base seeded at exactly `0.5` (E lower bound) is compared as `0.5 >= 0.5`
and should pass; a float computed as `4.999...` instead of `5.0` could miss S tier.
**Why it happens:** Multiplication of floats (e.g. `1.0 * 220.0`) can introduce tiny rounding.
**How to avoid:** Seeded values (`1.0`, `4.5`, `220.0`, `0.1`, `0.5`, `2.0`, `10.0`, `30.0`,
`220.0`) are all **exactly representable** in IEEE 754 binary64 and their products (e.g.
`5.0 * 220.0 = 1100.0`) are also exact. The only conversion needed is div→chaos multiplication,
and all the tier cutoffs result in exact floats. No epsilon comparison is needed for this seeded
data. For future live prices from poe.ninja (Phase 4), the same simplicity applies: poe.ninja
returns 1-2 decimal places, which are exact in f64.
**Warning signs:** Items at exact tier boundaries unexpectedly falling to the lower tier.

### Pitfall 3: CRLF in Generated Output (carried forward from Phase 2)

**What goes wrong:** On Windows, `std::fs::write` does not add CRLF, but `git checkout` will
convert LF→CRLF on Windows if `.gitattributes` has `*.filter text`. The test infrastructure
normalizes both sides before comparison (`normalize()` in `tests/cli_generate.rs`).
**Why it happens:** The Phase 2 emitter already emits LF-only via `writeln!`; Phase 3 must
preserve this invariant in any new emitter code.
**How to avoid:** All `writeln!` calls in emitter.rs already produce `\n`. No new I/O in the
emitter. The `generate_lf_only` test catches any regression.
**Warning signs:** `generate_lf_only` test fails on Windows.

### Pitfall 4: Snapshot Test vs. Artifact Guard Ordering

**What goes wrong:** Updating the insta snapshot file without also re-running `ferrisfilter generate`
to update `universal.filter` causes the artifact-guard test to fail.
**Why it happens:** Two separate files track the same truth: the `.snap` file and `universal.filter`.
**How to avoid:** The re-baseline update sequence is: (1) run `INSTA_UPDATE=always cargo test
--test cli_generate generate_snapshot`; (2) run `ferrisfilter generate` (or `cargo run --bin ff -- generate`)
to update `universal.filter`; (3) commit both. The artifact-guard test then passes.
**Warning signs:** `artifact_guard` test fails after updating the snapshot.

### Pitfall 5: prices.json Absence vs. Malformation

**What goes wrong:** Treating "file not found" as a `miette::Error` from `load_json` when it
should be a warn-and-skip.
**Why it happens:** `loader::load_json` returns `miette::Result<T>` — an I/O error (file not found)
and a parse error both return `Err(...)`. They should be handled differently: missing = GEN-05
warn-and-skip; malformed = abort with error.
**How to avoid:** Add `loader::load_prices_optional(path: &str) -> miette::Result<Option<PricesFile>>`
that catches `std::io::ErrorKind::NotFound` and returns `Ok(None)`, propagating all other errors.
Or check `std::path::Path::new(path).exists()` before calling `load_json`.
**Warning signs:** `generate` fails with "cannot read prices.json" when the file is intentionally absent.

### Pitfall 6: Unique Block Breaking GEN-04 Test

**What goes wrong:** Building the unique emission capability introduces `Rarity == Unique` blocks
in the committed output, failing `generate_no_unique_or_jewels_blocks`.
**Why it happens:** The capability is built but must remain dormant (seeded `prices.json` has no
uniques).
**How to avoid:** The emitter only emits the uniques section when `unique_blocks` is non-empty.
With zero seeded uniques, `Vec<UniqueBlock>` is empty, the section is skipped, and GEN-04 passes.
**Warning signs:** `generate_no_unique_or_jewels_blocks` fails after Phase 3 implementation.

---

## Code Examples

### Emitter Refactor: New Signature

```rust
// Source: verified from existing emitter.rs structure (2026-05-24)
// New public signature — replaces generate(&Config, &CurrencyData)
pub fn generate(config: &Config, priced: &PricedData) -> String {
    let mut out = String::with_capacity(32_000);
    emit_header(&mut out);
    emit_recycle(&mut out, config);
    emit_gear_bands(&mut out, config);
    emit_flask_charm_bands(&mut out, config);
    emit_currency_sections(&mut out, config, &priced.category_blocks);
    emit_unique_sections(&mut out, config, &priced.unique_blocks);
    debug_assert!(out.ends_with('\n'));
    out
}

// PricedData: the output of pricer::price_tiers()
pub struct PricedData {
    pub category_blocks: Vec<CategoryBlock>,
    pub unique_blocks: Vec<UniqueBlock>,
}
```

### Emitter: Currency Category Block Emission

```rust
// Source: derived from existing emit_currency_sections pattern (2026-05-24)
fn emit_currency_sections(out: &mut String, config: &Config, blocks: &[CategoryBlock]) {
    writeln!(out, "#============================================================").unwrap();
    writeln!(out, "# alwaysShow.currency — tiered S-F  [styles.S .. styles.F]").unwrap();
    // ... existing header lines ...
    writeln!(out, "#============================================================").unwrap();
    writeln!(out).unwrap();

    for category in blocks {
        writeln!(out, "# --- {} ---", category.name).unwrap();
        for tier_group in &category.tiers {
            let label = derive_block_label(&category.name, &tier_group.tier, &tier_group.class);
            let style = config.styles.by_tier(&tier_group.tier); // new helper on Styles
            let base_type_str = tier_group.items.iter()
                .map(|i| format!("\"{}\"", i.base_type))
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(out, "Show # {}", label).unwrap();
            writeln!(out, "  Class == \"{}\"", tier_group.class).unwrap();
            writeln!(out, "  BaseType == {}", base_type_str).unwrap();
            emit_style_actions(out, style);
            writeln!(out).unwrap();
        }
    }
    // Remove trailing blank line (same as existing pattern)
    if out.ends_with("\n\n") { out.pop(); }
}
```

Note: The `derive_block_label` function from Phase 2 carries forward unchanged — it maps
category/tier/class to the correct human-readable block label. Phase 3 must preserve it exactly.

### main.rs: Updated cmd_generate

```rust
// Source: derived from existing main.rs structure (2026-05-24)
fn cmd_generate(output: &str) -> miette::Result<()> {
    let config: Config = loader::load_json("config.json")?;
    let tiers = loader::load_tiers("tiers.json")?;

    // GEN-05: load prices.json if present, warn + skip if absent
    let prices_opt = loader::load_prices_optional("prices.json")?;

    let priced = match prices_opt {
        None => {
            eprintln!("warning: prices.json not found — currency/category blocks will be omitted");
            PricedData::empty()
        }
        Some(prices) => {
            // TIER-04: abort on bad reference rates
            let rates = pricer::derive_rates(&prices.items)?;
            pricer::price_tiers(&prices, &tiers, &rates)?
        }
    };

    let generated = emitter::generate(&config, &priced);
    std::fs::write(output, &generated)
        .map_err(|e| miette::miette!("Cannot write `{output}`: {e}"))?;
    println!("wrote {} bytes to {output}", generated.len());
    Ok(())
}
```

### Fixture Test for TIER-03 (Separate from Committed prices.json)

```rust
// Source: derived from existing test patterns in emitter.rs (2026-05-24)
#[test]
fn unique_base_collision_uses_highest_tier() {
    // Inline fixture: two uniques sharing "Heavy Belt", one B-tier, one D-tier
    let prices_json = r#"{
        "league": "Standard",
        "fetchedAt": "2026-01-01T00:00:00Z",
        "items": [
            {"name":"Exalted Orb","baseType":"Exalted Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":4.5,"div":null},
            {"name":"Chaos Orb","baseType":"Chaos Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":1.0,"div":null},
            {"name":"Divine Orb","baseType":"Divine Orb","class":"Stackable Currency","category":"Currency","exalt":null,"chaos":220.0,"div":null},
            {"name":"Headhunter","baseType":"Heavy Belt","class":"Belts","category":"Unique Belts","exalt":null,"chaos":50.0,"div":null},
            {"name":"Darkness Enthroned","baseType":"Heavy Belt","class":"Belts","category":"Unique Belts","exalt":null,"chaos":5.0,"div":null}
        ]
    }"#;
    let prices: PricesFile = serde_json::from_str(prices_json).unwrap();
    // Headhunter: 50c >= 30c = tier B; Darkness Enthroned: 5c >= 2c = tier D
    // Collision on "Heavy Belt": highest = B wins
    let tiers = load_real_tiers();
    let rates = pricer::derive_rates(&prices.items).unwrap();
    let priced = pricer::price_tiers(&prices, &tiers, &rates).unwrap();
    // Assert exactly one unique block at tier B with BaseType == "Heavy Belt"
    assert_eq!(priced.unique_blocks.len(), 1);
    assert_eq!(priced.unique_blocks[0].tier, "B");
    assert_eq!(priced.unique_blocks[0].base_types, vec!["Heavy Belt".to_string()]);
}
```

### Insta Snapshot Update Workflow

```bash
# Step 1: Accept the re-baselined snapshot
INSTA_UPDATE=always cargo test --test cli_generate generate_snapshot

# Step 2: Regenerate the committed universal.filter artifact
cargo run --bin ferrisfilter -- generate

# Step 3: Verify artifact guard now passes
cargo test --test cli_generate artifact_guard

# Step 4: Commit both updated files together
git add tests/snapshots/cli_generate__generate_snapshot.snap universal.filter
git commit -m "test(snapshot): re-baseline for Phase 3 anchor orb re-tiering"
```

The `cargo insta review` interactive command is also available for manual diff inspection before
accepting. `INSTA_UPDATE=always` is appropriate here because the change is deliberate and
pre-computed (the diff is known).

[CITED: insta.rs/docs/cli/ — review, accept, INSTA_UPDATE behavior]
[VERIFIED: Cargo.lock shows insta 1.47.2, 2026-05-24]

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `currency.json` as generate input | `prices.json` as value source | Phase 3 | `currency.json` = reference only; generate decoupled from curated data |
| Curated tier groupings (static) | Price-driven tier assignment (dynamic) | Phase 3 | Tiers reflect market prices; re-tier on `sync` |
| All currency tiers from hard-coded groupings | Tiers from `tiers.json` cutoffs + computed chaos value | Phase 3 | `tiers.json` becomes actively consumed |

**Deprecated/outdated after Phase 3:**
- `emitter::generate(&Config, &CurrencyData)` signature → replaced by `generate(&Config, &PricedData)`
- `loader::load_json::<CurrencyData>(...)` call in `cmd_generate` → removed
- `src/models/currency.rs` model → retained in repo, removed from generate path

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The priority order (chaos > div > exalt) for multi-field items is recommended as Claude's Discretion | Cross-rate pattern | If the user wants a different preference, the pricer logic needs updating |
| A2 | Unique blocks should be emitted after all category blocks (separate section) | Pattern 5 | If the user wants uniques interleaved or before currency, emitter order changes |
| A3 | The seeded prices.json uses `chaos` field only (not `div` field) for all items | Seeding section | The seeding script must use the `chaos` field consistently; if `div` is used instead, div_rate normalization must not double-count |
| A4 | `base_type` is required (not Optional) in the new `PriceItem` schema for Phase 3 | prices.json schema | If making it required breaks existing tests before the 4-item fixture is replaced, the schema change and fixture replacement must happen atomically |

**All mathematical claims (re-baseline computation, tier cutoffs, item ordering) are VERIFIED
against the actual `currency.json`, `tiers.json`, and `prices.json` files in the repo.**

---

## Open Questions (RESOLVED)

> **RESOLVED (plan-phase, 2026-05-24):** Both adopted in the plans.
> 1. Empty uniques section — **omit the header entirely** when `unique_blocks` is empty (Plan 03-03 T1).
> 2. `load_prices_optional` — use a **`std::path::Path::new(path).exists()` check first** (Plan 03-03 T1).

1. **Unique block section header when zero uniques**
   - What we know: seeded `prices.json` has no uniques; the uniques section should emit nothing
   - What's unclear: should the section header comment (`# --- Uniques ---`) appear even when empty?
   - Recommendation: omit the header entirely when `unique_blocks` is empty — keep the file clean

2. **load_prices_optional implementation**
   - What we know: `load_json` returns `miette::Result<T>` and wraps I/O errors as `miette::Error`
   - What's unclear: whether to check `path::exists()` before calling or to inspect the error kind
   - Recommendation: `std::path::Path::new(path).exists()` check first is simpler and avoids
     trying to downcast the miette error to `io::Error`

---

## Environment Availability

Step 2.6: This phase is pure Rust code changes + committed JSON files. All external dependencies
(Rust toolchain, Cargo, insta) are already confirmed present from Phase 2 execution.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | Build | ✓ | (Phase 2 complete) | — |
| insta | Snapshot update | ✓ | 1.47.2 | — |
| ferrisfilter generate | Artifact guard update | ✓ | (Phase 2 complete) | — |

---

## Sources

### Primary (HIGH confidence)
- `currency.json` — migration source, all 15 categories, 72 entries, 358 bases [VERIFIED: read directly]
- `tiers.json` — all 7 cutoffs with units [VERIFIED: read directly]
- `prices.json` — current 4-item fixture; schema to extend [VERIFIED: read directly]
- `src/emitter.rs` — full emitter including `emit_currency_sections`, `derive_block_label`, `emit_style_actions` [VERIFIED: read directly]
- `src/models/prices.rs` — current `PriceItem` schema [VERIFIED: read directly]
- `src/models/tiers.rs` — `ValidatedTiers`, `TierEntry`, `Unit` enum [VERIFIED: read directly]
- `src/loader.rs` — `load_json`, `load_tiers` patterns [VERIFIED: read directly]
- `src/error.rs` — `JsonLoadError` diagnostic pattern to replicate for `PriceRateError` [VERIFIED: read directly]
- `src/main.rs` — `cmd_generate` current wiring [VERIFIED: read directly]
- `tests/cli_generate.rs` — snapshot/artifact-guard/determinism tests and `normalize()` helper [VERIFIED: read directly]
- `Cargo.toml` / `Cargo.lock` — insta 1.47.2 [VERIFIED: file read + lock grep]
- Re-baseline computation — executed via Node.js script against real data [VERIFIED: computed, 2026-05-24]

### Secondary (MEDIUM confidence)
- insta snapshot update workflow — `INSTA_UPDATE=always`, `cargo insta review`, `cargo insta accept` [CITED: insta.rs/docs/cli/ + insta.rs/docs/settings/]

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all existing, no new crates
- Cross-rate algorithm: HIGH — simple arithmetic, verified against real data
- Re-baseline computation: HIGH — deterministically computed from real files
- Tier assignment: HIGH — verified against tiers.json + currency.json
- Emitter refactor structure: HIGH — directly based on existing Phase 2 emitter code
- Unique block structure: MEDIUM — D-08 defines behavior; exact placement is Claude's discretion
- Insta snapshot workflow: MEDIUM — cited from docs, consistent with Phase 2 test infrastructure

**Research date:** 2026-05-24
**Valid until:** 2026-07-01 (stable stack; insta 1.x API is stable)
