# Phase 3: Pricing Layer - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 reworks `ferrisfilter generate` so currency/category tiers (and, as a built-but-dormant
capability, unique tiers) are **derived from `prices.json`** via cross-rate normalization +
`tiers.json` cutoffs — **dropping `currency.json` as a `generate` input** (it stays in-repo only as
a reference/seed). `prices.json` becomes the single value source that Phase 4's `sync` will later
populate from poe.ninja.

**In scope:** migrate the curated currency data into a seeded `prices.json`; implement cross-rate
math (TIER-01), tier assignment from cutoffs (TIER-02), abort on bad reference rates (TIER-04),
unique price-tiering + base-collision resolution as a capability (TIER-03), and graceful no-`prices.json`
behavior (GEN-05). Reproduce the Phase 2 filter output except for a small, deliberate re-baseline
(the three reference orbs re-tier — see D-09).

**Out of scope:** poe.ninja networking / the `sync` command (Phase 4); actually emitting unique
highlight blocks in the committed output (capability built + fixture-tested, but seeded empty).

</domain>

<decisions>
## Implementation Decisions

### Architecture pivot: prices.json is the value source; currency.json is dropped
- **D-01:** `generate` no longer reads `currency.json`. Currency/category tiers are computed from
  `prices.json`. `currency.json` stays in the repo as a **reference/seed only** (the migration
  source), never read at generate-time.
- **D-02:** **No-`prices.json` behavior (GEN-05):** if `prices.json` is absent, `generate` prints a
  warning to stderr and emits a valid filter containing only the config-driven blocks
  (gear AreaLevel bands, flask/charm ladder, recycle, Hide fallbacks) — **no currency/category
  blocks**. No crash. (`prices.json` is a committed repo artifact, so absence is an edge case.)

### prices.json schema & block derivation
- **D-03:** Each `prices.json` item: `{ name, baseType, class, category, exalt, chaos, div }`.
  `exalt`/`chaos`/`div` may be null. `name` = identity (poe.ninja/display name, e.g. "Headhunter");
  `baseType` = the filter `BaseType` value (e.g. "Heavy Belt"; == name for currency); `class` = the
  `Class ==` value; `category` = block grouping ("Currency", "Breach", "Vault Keys", …). The first
  three items remain **Exalt, Chaos, Divine** (SYNC-02 convention) and double as the cross-rate anchors.
- **D-04:** **Block grouping/order:** group items by `category`; emit category blocks in **order of
  first appearance** in the items array (seed the array in the current `currency.json` category order).
  Within a category, emit tiers **S→F**, only populated tiers. This reproduces today's block sequence.

### Tiering math (TIER-01/02/04)
- **D-05:** **Cross-rate (TIER-01):** derive the div→chaos (and exalt→chaos) rates from the first
  three entries (Exalt/Chaos/Divine); normalize every item's value to **chaos** (the reference unit).
- **D-06:** **Tier assignment (TIER-02):** an item's tier = the highest `tiers.json` cutoff whose
  (chaos-normalized) lower bound it meets, using **inclusive ≥** comparison. Cutoffs (current
  `tiers.json`): F≥0.1c, E≥0.5c, D≥2c, C≥10c, B≥30c, A≥1div, S≥5div. Items below F's floor (or with
  no usable price) do not emit a tier block.
- **D-07:** **Abort on bad reference rate (TIER-04):** if Exalt, Chaos, or Divine has a null or zero
  price in `prices.json`, `generate` aborts with a clear error **naming the offending currency**
  (no silent miscalculation). Reuse the miette/thiserror diagnostic style.

### Uniques (TIER-03) — capability now, dormant in committed output
- **D-08:** Implement unique price-tiering now: a priced unique is tiered by its price; when multiple
  priced uniques **share a base type**, the emitted block uses the **highest tier** among them
  (collision → highest). Untracked uniques fall through (GEN-04). Emitted unique block shape:
  `Rarity == Unique` + `BaseType == "<base(s)>"`, styled by the unique's tier (exact section
  placement/ordering is Claude's discretion, researched against filter conventions).
  **Seed the committed `prices.json` with NO uniques** → zero unique blocks → Phase 2 output
  preserved (aside from D-09). Prove the unique logic + collision with a **separate fixture test**
  (a small prices.json containing uniques), not the committed file. Unique blocks activate when
  `sync` (Phase 4) adds priced uniques.

### Output preservation & the accepted re-baseline
- **D-09:** **Seeding:** migrate every base from `currency.json` into `prices.json` with a value that
  reproduces its curated tier — **non-reference bases get their tier's lower-bound value**
  (F→0.1c, E→0.5c, D→2c, C→10c, B→30c, A→1div, S→5div). The **three reference orbs get realistic
  values** (Chaos=1c; Exalt≈4.5c; Divine≈220c, reusing the existing fixture), which **re-tier them**:
  **Chaos C→E, Exalt C→D, Divine S→A** (Divine is structurally A: it is 1 div, and S=5 div). This is
  an **accepted, deliberate small re-baseline** — the feature working as intended (price-driven tiers
  reflect value, not the old hand-tuned desirability). Only the affected `Currency` blocks change;
  every other block stays byte-identical to Phase 2.
- **D-10:** **Acceptance gate:** the Phase 2 insta snapshot is updated ONCE to the new (re-baselined)
  output via `cargo insta review` and re-committed; thereafter it is the hard regression oracle.
  Produce a **human-review diff** (Phase-2 output → Phase-3 output) showing the limited currency
  changes, plus a **quick in-game spot-check** of the re-tiered orbs (lighter human-UAT than Phase 2).
  No changes beyond the anchor re-tiering and its direct block consequences are permitted without
  flagging.

### Claude's Discretion
- Cross-rate math details (handling items priced only in exalt/div vs chaos); TIER-04 exact error
  wording; unique block section placement/ordering; emitter refactor structure (currency blocks now
  sourced from `prices.json` grouped by category + computed tier); how the seeded `prices.json` is
  generated from `currency.json` (one-time migration script vs hand-authored fixture).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Inputs, target, and the regression oracle
- `currency.json` — the **migration source** (categories[].entries[]{tier, class, bases}); seed
  `prices.json` from it. After Phase 3 it is reference-only (not a generate input).
- `prices.json` — the new value source (schema per D-03); seeded this phase, populated by `sync` later.
- `tiers.json` — the cutoffs consumed by tier assignment (D-06). Now actively read by `generate`.
- `config.json` — styles + bands + build; still drives gear/flask/recycle blocks.
- `universal.filter` — the Phase 2 output; Phase 3 reproduces it except the D-09 anchor re-tiering.
- `tests/snapshots/cli_generate__generate_snapshot.snap` — the Phase 2 regression oracle; updated
  once for the D-09 re-baseline.

### Code
- `src/emitter.rs` — the emitter to rework (currency blocks now from `prices.json`).
- `src/models/prices.rs` — extend to the D-03 schema (add `class`, `category`; ensure `baseType`).
- `src/models/currency.rs` — model stays for reference; remove from the generate path.
- `src/models/tiers.rs` (ValidatedTiers), `src/loader.rs`, `src/main.rs` — cross-rate + tier
  assignment + abort logic wire in here.

### Requirements & criteria
- `.planning/REQUIREMENTS.md` §Generate/Tiering — GEN-05, TIER-01, TIER-02, TIER-03, TIER-04.
- `.planning/ROADMAP.md` → "Phase 3: Pricing Layer" — success criteria (note criterion 3's literal
  wording is satisfied by D-02).
- `.planning/phases/02-static-generate/02-CONTEXT.md` — D-13/D-14 (the snapshot-as-reference intent).
- `CLAUDE.md` — Dracula tiers, currency-as-curated-data note (now superseded by price-driven tiering),
  the "price-tracked uniques only" design philosophy (matches D-08).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `prices.rs` already models `{name, baseType?, exalt, chaos, div}` + a `{league, fetchedAt, items}`
  wrapper — extend with `class` + `category` and make `baseType` required-ish for emission.
- `ValidatedTiers` (tiers.rs) gives the ordered, unit-tagged cutoffs; tier assignment consumes it.
- `loader::load_json` + `JsonLoadError` (miette/thiserror) — reuse for prices.json load + the TIER-04
  abort error.
- `emitter::generate(&Config, &CurrencyData) -> String` currently takes `CurrencyData`; rework its
  signature to take the priced data instead (e.g. `generate(&Config, &PricedData, &ValidatedTiers)`),
  computing currency/category blocks from prices + cutoffs.
- insta snapshot + artifact-guard + determinism tests already exist (Phase 2) — they become the
  Phase 3 oracle after the one-time re-baseline.

### Constraints / Determinism
- `prices.json` items is a `Vec` (ordered) → deterministic iteration; group-by-category must preserve
  first-appearance order; no HashMap iteration.
- Tier comparison must be inclusive (≥) at the lower bound so seeded lower-bound values land in-tier.
- The three reference orbs are positional (items[0..2]) AND emit as normal currency blocks.

### Integration Points
- `install.bat` copies repo-root `universal.filter` (unchanged flow).
- Phase 4 `sync` will write `prices.json` in this exact schema — sync must populate `class`/`category`
  (from poe.ninja categorization or a static base→meta map); flagged for the Phase 4 discussion.

</code_context>

<specifics>
## Specific Ideas

- User's driving insight: all "how valuable is X" data belongs in `prices.json` (sync's target);
  `currency.json` was redundant as a value source. Phase 3 realizes the original PROJECT.md intent
  ("sync + generate replace curation with price-driven tiering").
- The Phase 2 snapshot is the mechanism that *proves* "output doesn't change" — and it precisely
  surfaces the D-09 anchor re-tiering as the only intended change.
- The user accepts that price-driven tiering legitimately re-tiers iconic orbs (Divine→A, etc.) —
  no tier-pin/override mechanism is being added.

</specifics>

<deferred>
## Deferred Ideas

- **poe.ninja `sync`** populating `prices.json` (currency + uniques + gems≥18) → Phase 4.
- **Actually emitting unique highlight blocks** in committed output → activates when `sync` provides
  priced uniques (capability + fixture test built in Phase 3, but seeded empty).
- **Tier-pin / manual override** for iconic items (considered, not adopted — pure price-driven chosen).
- **How `sync` maps poe.ninja → class/category** → Phase 4 discussion.

</deferred>

---

*Phase: 03-pricing-layer*
*Context gathered: 2026-05-24*
