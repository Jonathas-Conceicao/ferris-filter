# Research Summary — FerrisFilter v0.3 Economy Items & Exceptional Gear

**Project:** FerrisFilter v0.3
**Domain:** Rust CLI / PoE2 loot filter compiler + poe.ninja economy sync
**Researched:** 2026-06-23
**Confidence:** HIGH (live API verification + codebase trace + reference-filter audit)

---

## Executive Summary

FerrisFilter v0.3 adds three economy categories (Verisium, Lineage Support Gems, and a
corrected Uncut Gems path) plus a new "exceptional gear" policy highlight. The work splits
cleanly into two concerns: the **sync/pricer pipeline** (economy data, new endpoints, gem
background overrides) and the **rule vocabulary** (a new `corrupted` condition enabling the
exceptional-gear rules). No new Rust crates are needed. The existing exchange-endpoint schema
handles all three new economy categories; only `mapping.json`, `src/sync/mapping.rs`,
`src/sync/prices.rs`, `src/pricer.rs`, `src/emitter.rs`, and `src/models/config.rs` change.

**Authoritative conflict resolution — Uncut Gems.** FEATURES.md (NeverSink-based) claimed the
in-game BaseType is bare `"Uncut Skill Gem"` matched with a `GemLevel` condition and Class
`"Gems"`. This is **superseded**. The author's own tested reference filters
(`reference-old-filters/chronomancer_farm.filter`, `bloodmage_farm.filter`,
`chronomancer_ssf.filter`) and STACK.md's live API verification confirm each gem level is a
**distinct in-game base type** and the correct block is:

```
Class == "Uncut Skill Gems"            # also "Uncut Spirit Gems" / "Uncut Support Gems"
BaseType == "Uncut Skill Gem (Level 20)"
```

The fix is therefore to **stop stripping** the `(Level N)` suffix (the current
`strip_gem_level` + `gem_best` dedup is the bug), keep one priced entry per level, and emit the
per-subtype plural `Class`. PITFALLS.md P1's framing ("stripping is correct") is reversed.
The current `mapping.json` gemFilter (`class: "Stackable Currency"`, `minLevel: 18`) is doubly
broken: wrong class **and** strips the level.

The highest-risk open question is **LineageSupportGems filter Class** — poe2db says
`"Support Gems"`; NeverSink uses `"Skill Gems" "Support Gems"`. One poe2db lookup resolves it;
treated as a blocking spike before the mapping is committed.

---

## Key Findings

### Recommended Stack

**No new dependencies.** Existing `serde_json`, `reqwest` (via the `HttpClient` trait),
`miette`, `clap`, `insta`, `tempfile` cover all v0.3 work. All three poe.ninja endpoints use
the identical `exchange/current/overview` schema already modeled by
`ExchangeOverview`/`ExchangeItem` in `src/sync/models.rs`; the `lines[].id` → `items[].id`
join pattern applies uniformly.

**Live-verified `type=` spellings (2026-06-23, league "Runes of Aldur"):**
- `type=UncutGems` — plural, correct (50 per-level items incl. Skill/Spirit/Support)
- `type=LineageSupportGems` — camelCase plural, correct (71 priced lines)
- `type=Verisium` — singular, correct (23 items, incl. "Exceptional Verisium")

`insta` + `artifact_guard` snapshots require explicit re-baselining at each phase that changes
emitter output.

### Feature Decisions (Resolved)

**Verisium** — `Class "Stackable Currency"`, `BaseType ==` from API, regular tiered currency
style, **no** special background. Treat identically to existing currency categories
(Essences/Runes). "Exceptional Verisium" is a distinct base type in the same endpoint; standard
price-tiering is expected to suffice.

**Uncut Gems fix (AUTHORITATIVE)** — Each gem level is a distinct PoE2 base type:
- `Class == "Uncut Skill Gems"` / `"Uncut Spirit Gems"` / `"Uncut Support Gems"` (three distinct
  plural classes, derived from the base-type prefix)
- `BaseType == "Uncut Skill Gem (Level N)"` — level in the base type, one entry per level
- Remove `strip_gem_level`, `gem_best` dedup, and the `minLevel` guard (tier cutoffs gate relevance)
- Fixed background `80 70 55`

**Lineage Support Gems** — Exchange endpoint confirmed live (71 lines). Individual named gems
(`baseTypeIsName: true`). `Class` is an **open spike** (`"Support Gems"` vs
`"Skill Gems" "Support Gems"`). Fixed background `10 150 150`.

**Exceptional gear highlight** — No native PoE2 filter tag. Approximated via standard
conditions; OR-logic needs **separate rules** (filter blocks are AND-only), placed before the
gear-hide fallback:
1. `Sockets >= 3` + `Corrupted False` — 2H/Body classes
2. `Sockets >= 2` + `Corrupted False` — 1H/smaller classes
3. `Quality >= N` + `Corrupted False` — all gear classes (threshold = author decision)

Requires adding `corrupted: Option<bool>` to the `When` struct (`src/models/config.rs`) + an
emit arm. Correct syntax is `Corrupted False` (boolean, no operator).

**Anti-features:** single rule combining `sockets`+`quality` (yields AND, wrong);
`Corrupted == False` (wrong syntax); `Class "Stackable Currency"` for uncut gems (current bug);
bare `BaseType == "Uncut Skill Gem"` without level (current bug).

### Architecture Approach

Surgical — no new pipeline stages or subcommands.

| File | Change | What changes |
|------|--------|--------------|
| `mapping.json` | Data | Add Verisium to `overviewTypes`; replace `gemFilter` with `gemFilters` array (+ per-category background, per-subtype class); bump `version` → 2 |
| `src/sync/mapping.rs` | Minor | `gem_filter: GemFilter` → `gem_filters: Vec<GemFilter>`; add `background_override: Option<[u8;3]>` (and possibly `class: Vec<String>`) |
| `src/sync/prices.rs` | Moderate | Generalize gem dispatch; remove `strip_gem_level` + `gem_best` dedup + `min_level` guard; one `PriceItem` per gem level |
| `src/pricer.rs` | Minor | Add `background_override: Option<[u8;3]>` to `CategoryBlock`; populate per gem category |
| `src/emitter.rs` | Minor | Clone-null-override pattern in `emit_currency_sections`; add `Corrupted` arm in `emit_rules` |
| `src/models/config.rs` | Minor | Add `corrupted: Option<bool>` to `When` |
| `config.json` | Data | Exceptional-gear rules using `"corrupted": false` |
| `tests/fixtures/seed-prices.json` + `seed-universal.filter` | Re-baseline | Per-level uncut gems, Verisium, Lineage gems, background overrides |

**Unchanged:** `src/models/prices.rs`, `src/sync/league.rs`, `src/sync/client.rs`,
`src/sync/models.rs`, `src/models/tiers.rs`, `tiers.json`, `tiers-fallback.json`.

**Background-override pattern:** mirror `emit_unique_sections` (emitter.rs:455–458) exactly —
clone the tier `Style`, set `background = None` on the clone, call `emit_style_actions(clone)`,
then `writeln!(out, "  SetBackgroundColor R G B 255")`. One `SetBackgroundColor` line per block.

### Critical Pitfalls

1. **Uncut Gem level stripping is the bug, not the fix** — remove `strip_gem_level` + `gem_best`
   dedup from the gem path; each `"Uncut … (Level N)"` is its own `PriceItem`.
2. **poe.ninja `type=` misspelling = silent empty fetch (HTTP 200)** — verify live; extend
   `overview_types_use_correct_plural_spellings` to cover `Verisium` + `LineageSupportGems`.
3. **OR-logic needs separate rules** — a single rule with `sockets`+`quality` is AND (wrong).
4. **`corrupted` absent from `When`** — adding `"corrupted": false` before adding the struct
   field triggers a `deny_unknown_fields` generate-time error. Add field + emit arm first.
5. **Snapshot/artifact-guard re-baseline** — `artifact_guard`, `generate_snapshot`,
   `generate_block_counts` lock to the seed fixture; do ONE consolidated re-baseline at the end.
6. **Background double-emit** — null the cloned style's background before appending the override.
7. **Gem `Class` correctness (Tablets lesson)** — a wrong/absent Class silently breaks the block;
   uncut = per-subtype plural, lineage = spike.

---

## Implications for Roadmap

**Suggested phases: 3** (continuing numbering → Phases 8–10)

### Phase 8 — Sync Pipeline: New Categories + Uncut Gem Fix
Foundation: everything downstream depends on correct `prices.json`.
- `mapping.json`: `gemFilters` array (UncutGems per-subtype class, no minLevel; LineageSupportGems
  + background); Verisium in `overviewTypes`; `version` → 2
- `src/sync/mapping.rs`: `gem_filters: Vec<GemFilter>` + `background_override` (+ class shape)
- `src/sync/prices.rs`: generalized gem dispatch; remove `strip_gem_level`/`gem_best`/`min_level`;
  per-level uncut gem `PriceItem`s
- Extend the plural-spelling guard test
- **BLOCKING SPIKE first:** confirm LineageSupportGems filter `Class` (poe2db lookup); confirm
  endpoint type (exchange vs item overview — exchange confirmed live, just verify dispatch fits)
- Avoids pitfalls 1, 2, 7

### Phase 9 — Pricer + Emitter: Gem Background Overrides
Depends on Phase 8's `CategoryBlock` data.
- `src/pricer.rs`: `background_override` on `CategoryBlock`; `Uncut*` → `[80,70,55]`,
  `Lineage Support Gems` → `[10,150,150]`
- `src/emitter.rs`: clone-null-override in `emit_currency_sections`
- Guard tests: no duplicate `SetBackgroundColor` in gem blocks; none on policy blocks
- Avoids pitfalls 5, 6

### Phase 10 — Rule Vocabulary: Corrupted Condition + Exceptional Gear
Independent of the economy pipeline; sequenced last for ONE consolidated fixture re-baseline.
- `src/models/config.rs`: `corrupted: Option<bool>` in `When`
- `src/emitter.rs`: `Corrupted True`/`Corrupted False` arm
- `config.json`: exceptional-gear rules (2–3 blocks) before gear-hide fallback
- Re-baseline `seed-*` fixtures + `generate_block_counts`
- **Author decisions:** quality threshold (`>= 20` vs higher); exceptional-gear style tier
- Avoids pitfalls 3, 4, 5

---

## Open Spikes (Resolve During Execution)

| Spike | Blocking Phase | How to Resolve |
|-------|---------------|----------------|
| LineageSupportGems filter `Class` name | Phase 8 | poe2db lookup on a Lineage gem (e.g. "Ahn's Citadel") → Item Class; if dual-class, `GemFilter.class: Vec<String>` |
| Confirm `Verisium`/`LineageSupportGems` spellings in guard test | Phase 8 | Live-verified by STACK.md; just update the test |
| "Exceptional Verisium" handling | Phase 8 | In the 23-item API list; standard tiering expected to suffice |
| `mapping.json` schema break (`gemFilter`→`gemFilters`) | Phase 8 | Bump `"version"` to 2 |
| Exceptional gear quality threshold + style tier | Phase 10 | Author decision (NeverSink ~`>=27`; reference filters ~`>=28`) |

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Endpoints live-verified 2026-06-23; no new crates |
| Verisium | HIGH | Stackable Currency confirmed (poe2db); 23 items |
| Uncut Gems | HIGH | AUTHORITATIVE: reference filters + live API confirm `(Level N)` in BaseType; FEATURES.md superseded |
| Lineage Gems class | MEDIUM | poe2db `"Support Gems"` vs NeverSink dual-class — spike required |
| Exceptional gear | HIGH | No native tag; `Sockets`/`Quality` + `Corrupted False`; two-rule OR confirmed |
| Architecture | HIGH | Full code trace; narrow change surface |
| Pitfalls | HIGH | Grounded in codebase + prior Tablets/Essences incidents |

**Overall: HIGH**, with one medium-confidence blocking spike (Lineage gem class).

---

## Sources

**Primary (HIGH):** Live poe.ninja API 2026-06-23 (`UncutGems`/`LineageSupportGems`/`Verisium`);
`reference-old-filters/{chronomancer_farm,bloodmage_farm,chronomancer_ssf}.filter` (author-tested);
direct trace of `src/sync/prices.rs`, `src/emitter.rs`, `src/models/config.rs`, `src/pricer.rs`;
existing test surface in `tests/`.
**Secondary (MEDIUM):** poe2db.tw (Verisium = Stackable Currency; Ahn's Citadel = Support Gem);
NeverSink-Filter-for-PoE2 (exceptional-gear + gem blocks).
**Tertiary (LOW — needs live confirmation):** NeverSink `"Skill Gems" "Support Gems"` for Lineage
(conflicts with poe2db; confirm before committing).

*Research completed: 2026-06-23 · Ready for roadmap: yes*
