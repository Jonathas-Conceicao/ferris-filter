# Phase 45: Economy Tier-Range Names - Research

**Researched:** 2026-10-23
**Domain:** Rust, economy-section emitter (`src/pricer.rs`, `src/emitter.rs`), overlay preview path
**Confidence:** HIGH (everything below was read from the codebase in this session; no external libraries involved)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Name format (author)
- `{T} tier {Group} < {next tier floor} {unit}` for every tier except the top one, e.g.
  `E tier Essences < 2 chaos`, `F tier Currency < 0.5 chaos`.
- The top tier states its floor with ASCII `>=`: `S tier Currency >= 30 chaos`.
- The range comes from the tier cutoffs (`tiers.json` / the fallback pair), never from item prices.
- Unit: the tiers' own unit word, `chaos` or `exalt`, never pluralised.
- Numbers: at most 2 decimals, trailing zeros trimmed (`0.5`, `2`, `12`, `0.15`).
- Price-tiered unique blocks and the gem categories with a background override get the same
  treatment (`A tier Uniques < 30 chaos`).

#### Behaviour
- Pending tier edits on the Economy tab rename the staged / previewed economy rows on the RULES tab,
  as they already re-price them.
- The first Apply after upgrading rewrites every economy name once — expected.

### Claude's Discretion
- Where the label is computed (research: a `tier_labels` map from `pricer`, used only on the tagged
  economy-section path) and the formatter's shape.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ENAME-01 | Each economy rule name states its tier's price range from the tier cutoffs (`E tier Essences < 2 chaos`; top tier `S tier Currency >= 30 chaos`) | Compute a per-tier range suffix in `pricer::price_tiers` from the same sorted cutoff list `assign_tier` uses; emit it at the two `# ff:name` sites in `emitter.rs` (551, 629) |
| ENAME-02 | Tiers' own unit (chaos live / exalt fallback), short number format; uniques get the same | `TierEntry.unit` + `Unit::as_str()` already exist; fallback pair is all-exalt, `tiers-default.json` all-chaos; unique site (emitter 629) and gem background-override categories go through the same two sites |
| ENAME-03 | Pending tier edits rename previewed economy rows; `generate` unchanged | Preview and Apply both reach `emitter::generate_economy` through `compile::load_economy_with` with the pending tiers, so names follow automatically. `generate` passes `tags = false`, so no `# ff:name` lines are ever written there |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- No new Rust dependencies (orchestrator instruction; nothing here needs one).
- Tests must pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` on every spawn (`tests/isolation_guard.rs` enforces it); lib tests never resolve the real folder.
- `tests/fixtures/seed-universal.filter` is the byte-for-byte `generate` baseline (`artifact_guard`), regenerated only after a deliberate generator change. This phase must not touch it.
- Agents add no CHANGELOG entries during a milestone (Release Review drafts them).
- CLAUDE.md is a reference that must stay accurate: its `dynamic.filter` row says "Every generated block carries `# ff:name <T> tier <Group>` (e.g. `S tier Currency`, `A tier Uniques`)" and must be updated to the new form. `tests/ui/README.md` line 50 repeats the same pattern.
- A bug introduced and fixed inside the milestone is never mentioned outside `.planning/`.
- Overlay UI should not be texty (memory): the longer names are only row labels, nothing new on screen.

## Summary

This is a small, contained change. Every economy `# ff:name` line is written in exactly two places, both in `src/emitter.rs` and both behind the `tags: bool` flag that only `generate_economy` sets to `true`: currency/category blocks at line 551 (`"# ff:name {} tier {}", tier_group.tier, category.name`) and price-tiered uniques at line 629 (`"# ff:name {} tier Uniques", block.tier`). The gem categories with a background override (Uncut Gems, Lineage Support Gems) go through the line-551 site, so they need no separate handling. `generate`/`generate_as` pass `tags = false`, so `universal.filter` and every fixture are untouched by construction.

All callers that render the economy section, the CLI `ff-cli apply` (`dynamic::economy_body`), the overlay Apply (`apply_all.rs:190`, `:662`) and the RULES-tab preview (`app.rs::staged_economy_body` called from `refresh_staged_economy`), funnel through `compile::load_priced_with` → `pricer::price_tiers(&prices, &tiers, &rates)` → `emitter::generate_economy(styles, &priced)`. The overlay preview passes `c.pending_tiers()`, and under Fallback the fallback tier file is substituted inside `load_priced_with`. So if `price_tiers` records the range label per tier in `PricedData`, ENAME-03 (preview rename) and the live/fallback unit switch come for free with no overlay code change.

`price_tiers` already has everything needed: the `ValidatedTiers` (each `TierEntry { tier, min, unit }`) and `normalize_cutoffs`, which sorts tiers by their chaos-equivalent floor descending, the exact order `assign_tier` uses. The range for the tier at sorted index 0 is `>= {its min} {its unit}`; for index i > 0 it is `< {sorted[i-1].min} {sorted[i-1].unit}`. Because sub-floor items are floored into the lowest tier (`assign_tier` returns `cutoffs.last()`), the lowest tier genuinely has no lower bound, so `F tier X < 0.5 chaos` is exactly right.

**Primary recommendation:** Add `tier_ranges: HashMap<String, String>` (tier letter → `">= 30 chaos"` / `"< 2 chaos"`) to `PricedData`, fill it in `price_tiers` from the normalized cutoff order, add a pure `fmt_amount(f64) -> String` helper, and append ` {range}` at the two tagged `# ff:name` sites (falling back to the plain name when the map has no entry). Update the four test pins and the two docs.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Range per tier (floor of next tier up, top tier floor) | Pricer (`src/pricer.rs`) | — | Only place holding both the raw `TierEntry` (min + unit) and the normalized order that defines "next tier up" |
| Number formatting | Pricer (pure helper) | — | Pure fn, unit-testable, no deps |
| Writing `# ff:name` | Emitter (`src/emitter.rs`, `tags == true` path only) | — | Already owns the line; `tags` gate keeps `generate` unchanged |
| Preview rename on pending tier edits | Overlay (`app.rs::refresh_staged_economy`) | Compile (`load_economy_with`) | Already re-renders from pending tiers; no change needed |
| Search on new names | Overlay rules list | — | Search matches `# ff:name` text; works unchanged |

## Standard Stack

No libraries. Pure `std` Rust: `format!("{:.2}", v)` plus string trimming, `std::collections::HashMap` (already used throughout `pricer.rs`).

## Package Legitimacy Audit

Not applicable: this phase installs no packages.

## Architecture Patterns

### Data flow

```
tiers.json / pending tiers (overlay) ─┐        tiers-fallback.json (Fallback / no prices.json)
                                      v                      v
             compile::load_priced_with(paths, policy, Some(pending))  -- picks ONE ValidatedTiers
                                      |
                                      v
     pricer::price_tiers(prices, tiers, rates)
        normalize_cutoffs -> [(tier, chaos_floor)] sorted desc    (NEW: also keep entry min+unit)
        assign_tier per item                                      (unchanged)
        NEW: tier_ranges { "S": ">= 30 chaos", "A": "< 30 chaos", ... }
                                      |
                                      v  PricedData { category_blocks, unique_blocks, tier_ranges }
        ┌─────────────────────────────┴───────────────────────────────┐
        v                                                             v
emitter::generate_as (tags=false)                       emitter::generate_economy (tags=true)
  universal.filter: NO ff:name (unchanged)                # ff:name {T} tier {Group} {range}
                                                                      |
                     ┌────────────────────────────────┬───────────────┴────────────┐
                     v                                v                            v
           ff-cli apply (economy_body)     overlay Apply (apply_all)    RULES preview (staged_economy_body
                                                                         via refresh_staged_economy)
```

### Pattern 1: Range computed from the normalized order (not the S..F label order)

`normalize_cutoffs` (pricer.rs:225) returns `Vec<(String, f64)>` sorted by chaos floor descending. Tiers can in principle carry different units per entry (`Unit::{Chaos, Div, Exalt}`; validation only checks ascending order *within* a unit group, models/tiers.rs). Using the same sorted order that `assign_tier` uses guarantees the label describes exactly what the tier matches, even for a hand-written mixed-unit file. For both shipped files (all-chaos, all-exalt) this order is S, A, B, C, D, E, F.

Recommended shape (illustrative, not verbatim):

```rust
// pricer.rs
fn tier_ranges(tiers: &ValidatedTiers, rates: &CrossRates) -> HashMap<String, String> {
    let mut entries: Vec<(&TierEntry, f64)> = tiers.0.iter()
        .map(|e| (e, chaos_floor(e, rates)))   // same conversion as normalize_cutoffs
        .collect();
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // stable, same as normalize_cutoffs
    entries.iter().enumerate().map(|(i, (e, _))| {
        let label = match i {
            0 => format!(">= {} {}", fmt_amount(e.min), e.unit.as_str()),
            _ => { let up = entries[i - 1].0;
                   format!("< {} {}", fmt_amount(up.min), up.unit.as_str()) }
        };
        (e.tier.clone(), label)
    }).collect()
}

/// At most 2 decimals, trailing zeros (and a bare '.') trimmed: 0.5, 2, 12, 0.15.
pub fn fmt_amount(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
```

Factor the unit→chaos conversion out of `normalize_cutoffs` into one helper so both use identical math (and identical stable sort, so ties resolve the same way).

### Pattern 2: Emit at the two tagged sites with a fallback

```rust
// emitter.rs, inside `if tags { ... }`
let name = economy_name(&tier_group.tier, &category.name, &priced_ranges);
// economy_name: "{T} tier {G} {range}" when the map has the tier, else "{T} tier {G}"
```

`emit_currency_sections` / `emit_unique_sections` currently take `&[CategoryBlock]` / `&[UniqueBlock]`; pass `&priced.tier_ranges` (or `&PricedData`) in as an extra argument. `generate_as` can pass an empty map or the real one — it never reads it because `tags` is false. Keep the fallback (no map entry → old plain name) so hand-built `PricedData` in tests and `PricedData::empty()` still work.

### Alternative considered (not recommended)
Storing a `range: String` on each `TierGroup` / `UniqueBlock`. Works, but duplicates the same string across every class group of a tier and touches every struct literal. The single map in `PricedData` matches CONTEXT's suggestion ("a `tier_labels` map from `pricer`").

### Anti-Patterns to Avoid
- **Deriving the range from item prices** (locked out by CONTEXT).
- **Using S..F label order to find "next tier up"** instead of the normalized order: diverges from `assign_tier` for mixed-unit files.
- **Adding the range inside `derive_block_label`** (the `Show # Currency S` header comment): that string is also used by `generate` and would break `seed-universal.filter`.
- **Normalising the label to chaos** for the fallback pair: the locked decision wants the tiers' own unit (`exalt`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rounding to 2 decimals | Manual multiply/round/divide | `format!("{:.2}")` then trim | std formatting rounds the exact decimal expansion correctly (e.g. 1.999 → `2.00` → `2`) |
| Unit word | New match | `Unit::as_str()` (models/tiers.rs) | Already the serde spelling; `chaos` / `div` / `exalt` |
| "Next tier" order | Hard-coded `TIER_ORDER` walk | The `normalize_cutoffs` sort | Same order `assign_tier` applies |

## Runtime State Inventory

This is a format change to generated text, not a rename of an identifier, but the old names do persist on disk:

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | Users' game-folder filters carry old `# ff:name S tier Currency` lines in their economy section | None in code: every Apply with an economy section regenerates it (`EconomyPlan::Refresh`), so the first Apply rewrites them (locked as expected). Until then, with no CONFIG edits, the RULES tab shows the file's old names |
| Live service config | None — verified, no external service stores names | none |
| OS-registered state | None | none |
| Secrets/env vars | None | none |
| Build artifacts | None (`tiers-*.json` are `include_str!`'d but unchanged) | none |

No code parses economy names back: `grep` for `" tier "`, `split(" tier`, `strip_suffix` across `src/` found only the test assertion at `app.rs:6812`. Economy blocks are identified by the section banner, not by name.

## Common Pitfalls

### Pitfall 1: Test reads the repo-root live files (environment-dependent names)
**What goes wrong:** `app.rs` test `economy_in_the_file_follows_pending_style_visibility` (line ~6860) uses `root = CARGO_MANIFEST_DIR` and looks up `r.name.as_deref() == Some("S tier Currency")`. In the author's main checkout the repo root holds a git-ignored `prices.json` and `tiers.json` (verified: both exist at `C:\Users\jonat\Repositories\poe2_myfilter`), so the body is Live-priced with the author's own cutoffs; in a clean worktree/CI there is no `prices.json`, so it is priced from the fallback pair (exalt). The new name is therefore `S tier Currency >= 30 chaos` in one place and `S tier Currency >= 20 exalt` in another.
**How to avoid:** Match with `name.starts_with("S tier Currency ")` (or `split(" >= ")`) there, not an exact string. Same for `app.rs:6812` (`.contains(" tier ")` keeps passing; optionally strengthen to also check `" >= "` or `" < "`).
**Warning signs:** Test passes in the worktree, fails in the main checkout (or vice versa).

### Pitfall 2: `generate` output accidentally changed
**What goes wrong:** Putting the range into `derive_block_label` or into an untagged line changes `universal.filter` and fails `artifact_guard` / `generate_snapshot`.
**How to avoid:** Only change lines inside `if tags { ... }`. Existing guards: `emitter.rs` test asserts `generate` has no `# ff:` lines; `tests/dynamic_filter.rs:163` compares the economy body minus `# ff:` lines with `generate`'s tail. Run `cargo test --locked` and confirm `artifact_guard` is green; never regenerate the fixture.

### Pitfall 3: Rounding to zero
**What goes wrong:** A cutoff below 0.005 formats as `0` (`format!("{:.2}", 0.004)` = `0.00` → trimmed `0`), giving `< 0 exalt`. The shipped fallback F floor is `0.001 exalt`, but F is the lowest tier so its own floor is never printed (its label uses E's `0.05`). Only a user editing a non-bottom cutoff below 0.005 would hit it.
**How to avoid:** Follow the locked format; treat as an open question (see below). Pin the shipped values in a test so a future change to the fallback file is noticed.

### Pitfall 4: Trim the dot, not the integer zeros
**What goes wrong:** a single `trim_end_matches(['0', '.'])` turns `10.00` into `1` and `20.00` into `2`.
**How to avoid:** Format with `{:.2}` (always has a `.`), trim `'0'` first, then a single trailing `'.'` — `"10.00"` → `"10."` → `"10"`. Correct as long as the string always contains a `.`, which `{:.2}` guarantees. Unit-test `10`, `100`, `0.5`, `0.15`, `2`, `12`, `20`, `1.999`, `0.05`.

### Pitfall 5: Hand-built `PricedData` literals
**What goes wrong:** Adding a field breaks `emitter.rs:1157` (struct literal in a test) and `PricedData::empty()`.
**How to avoid:** Add the field to `empty()` and the one literal (`tier_ranges: HashMap::new()` / `Default::default()`); consider `#[derive(Default)]` on `PricedData`. Emitter falls back to the plain name when a tier has no entry.

### Pitfall 6: Longer row labels
**What goes wrong:** Names like `D tier Map Fragments & Pinnacle Keys < 8 chaos` are longer; collapsed rows already cut the label with an ellipsis and keep at least 140px. No layout change is expected, but a screenshot check of the economy section is worth one manual item.

## Code Examples

See Pattern 1 and Pattern 2 above (illustrative). Existing reference points:
- `src/pricer.rs:225` `normalize_cutoffs` — conversion and sort to reuse.
- `src/pricer.rs:248` `assign_tier` — lowest tier catches sub-floor items (justifies `<` with no lower bound).
- `src/emitter.rs:550-555`, `:628-630` — the two `# ff:name` writes.
- `src/emitter.rs:125` `generate_economy` — the only `tags = true` caller; its doc comment (line 122) mentions `# ff:name <T> tier <Group>` and should be updated.

## Tests to update / add

| Location | Current pin | Change |
|----------|-------------|--------|
| `src/emitter.rs:1303` | `"# ff:name S tier Currency"` (seed prices + `tiers-default.json`, chaos) | `"# ff:name S tier Currency >= 30 chaos"` |
| `src/emitter.rs:1306` | `"# ff:name A tier Uniques"` (unique pushed onto a `price_tiers` result, so ranges exist) | `"# ff:name A tier Uniques < 30 chaos"` |
| `src/dynamic.rs:608` | `contains("# ff:name S tier Currency\n")` (seed dir: seed prices + tiers-default) | `contains("# ff:name S tier Currency >= 30 chaos\n")` |
| `src/overlay/app.rs:6876` | `== Some("S tier Currency")`, repo-root inputs | `starts_with("S tier Currency ")` (Pitfall 1) |
| `src/overlay/app.rs:6812` | `.contains(" tier ")` | keeps passing; optional strengthen |
| `src/overlay/rules.rs:2118-2140` | hand-written fixture text, parses names | no change needed (static parse test) |
| `tests/ui/test_ff.filter` | static UI fixture with old names | optional refresh to the new form so manual UI checks show it |
| `tests/dynamic_filter.rs:163` | strips `# ff:` lines, compares with `generate` | unchanged; this is the ENAME-03 "generate unchanged" guard |

New tests:
1. `pricer`: `fmt_amount` table (`0.5`, `2`, `12`, `0.15`, `10`, `100`, `20`, `1.999`→`2`, `0.05`).
2. `pricer`: `price_tiers` with `tiers-default.json` yields ranges S `>= 30 chaos`, A `< 30 chaos`, B `< 20 chaos`, C `< 12 chaos`, D `< 8 chaos`, E `< 2 chaos`, F `< 0.5 chaos`.
3. `pricer`: with `tiers-fallback.json` yields S `>= 20 exalt`, A `< 20 exalt`, B `< 2.5 exalt`, C `< 1 exalt`, D `< 0.2 exalt`, E `< 0.1 exalt`, F `< 0.05 exalt`.
4. `emitter`: a gem background-override category (e.g. Lineage Support Gems or Uncut Gems) carries the range too; uniques carry it; `generate` still has no `# ff:` line.
5. `dynamic` (`economy_body_with_uses_pending_tiers_and_styles`): with the S cutoff raised (`high`, S=1e6) the body contains `A tier ... < 1000000 chaos` / `S tier ... >= 1000000 chaos` where present — proves pending tiers rename (ENAME-03). Note S may have no items at 1e6; assert on a tier that still has items (e.g. the name line for A shows `< 1000000 chaos`).
6. Optionally `app.rs` `staged_economy_preview_follows_pending_tier_edits`: after `set_tier_text("S", "100000000")` + refresh, an economy row name ends with `< 100000000 chaos` (temp root with seed prices, already isolated).

Shipped cutoffs [VERIFIED: repo files]: `tiers-default.json` F 0.1, E 0.5, D 2, C 8, B 12, A 20, S 30 (chaos); `tiers-fallback.json` F 0.001, E 0.05, D 0.1, C 0.2, B 1, A 2.5, S 20 (exalt).

## State of the Art

| Old | New | Impact |
|-----|-----|--------|
| `# ff:name S tier Currency` | `# ff:name S tier Currency >= 30 chaos` | First Apply after upgrade rewrites the economy section names once |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | For a mixed-unit tiers file, the label uses the upper neighbour's own unit (e.g. `B tier X < 1 div`) | Pattern 1 | Cosmetic only; shipped files are single-unit |
| A2 | `div` is an acceptable unit word if a user's tiers use `"unit": "div"` (CONTEXT names only chaos / exalt) | Pattern 1 | Cosmetic; no shipped file uses div |

## Open Questions (RESOLVED)

1. **Cutoffs below 0.005 render as `0`.**
   - What we know: locked format says at most 2 decimals; no shipped non-bottom cutoff is that small.
   - What's unclear: whether a user-edited tiny cutoff should show more precision.
   - Recommendation: keep the locked rule; mention in the plan's verification list rather than special-casing.
   - RESOLVED: keep the locked rule; recorded as a judgement call in the deferred list.
2. **Ties in normalized floors** (two tiers with the same chaos-equivalent floor, only possible across units): the stable sort keeps file order, same as `assign_tier`. The lower of the tied pair would read `< X` where X equals its own floor. Accept; not reachable with shipped files. RESOLVED: accepted; recorded in the deferred list.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies; `cargo test --locked` with the existing nightly toolchain).

## Security Domain

No new input surface. The range string is built from validated `f64` cutoffs and a fixed unit enum and written into a `#` comment line; it cannot contain a newline or quote, so it cannot inject filter syntax. No ASVS category changes.

## Sources

### Primary (HIGH confidence, read this session)
- `src/pricer.rs` (PricedData, normalize_cutoffs, assign_tier, price_tiers)
- `src/emitter.rs` (generate_as, generate_economy, emit_currency_sections, emit_unique_sections, tests 1150-1345)
- `src/compile.rs` (load_economy, load_economy_with, load_priced_with)
- `src/dynamic.rs` (economy_body, economy_body_with, tests 529-635)
- `src/overlay/app.rs` (staged_economy_body, refresh_staged_economy, tests 6790-6890), `src/overlay/apply_all.rs:170-215`
- `src/models/tiers.rs` (Unit, TierEntry, ValidatedTiers validation)
- `tiers-default.json`, `tiers-fallback.json`, `tests/dynamic_filter.rs:150-170`, `tests/ui/README.md:50`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — std only.
- Architecture: HIGH — single render path verified by grep of every `generate_economy` / `economy_body*` caller.
- Pitfalls: HIGH — test pins located by grep; repo-root live-file dependency confirmed by listing the main checkout.

**Research date:** 2026-10-23
**Valid until:** 2026-11-22
