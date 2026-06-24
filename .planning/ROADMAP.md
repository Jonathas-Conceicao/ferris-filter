# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- 🔄 **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (started 2026-06-23)

## Phases

<details>
<summary>✅ v0.1 Initial Release (Phases 1-4) — SHIPPED 2026-05-28</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-05-24
- [x] Phase 2: Static Generate (3/3 plans) — completed 2026-05-24
- [x] Phase 3: Pricing Layer (4/4 plans) — completed 2026-05-25
- [x] Phase 4: Sync (4/4 plans) — completed 2026-05-28

Full detail: [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md)

</details>

<details>
<summary>✅ v0.2 Declarative Rule Model (Phases 5-7) — SHIPPED 2026-06-12</summary>

- [x] Phase 5: Rule Schema (2/2 plans) — completed 2026-06-05
- [x] Phase 6: Emitter Rewrite + Migration Parity (2/2 plans) — completed 2026-06-05
- [x] Phase 7: New Capabilities + Docs (3/3 plans) — completed 2026-06-12

Full detail: [milestones/v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md)

</details>

### v0.3 Economy Items & Exceptional Gear

- [x] **Phase 8: Sync Pipeline** — New economy categories + corrected Uncut Gem dispatch (completed 2026-06-24)
- [x] **Phase 9: Gem Background Styling** — Per-category background override on priced gem blocks (completed 2026-06-24)
- [ ] **Phase 10: Corrupted Condition + Exceptional Gear** — New `corrupted` when-field, exceptional-gear rules, new `exceptional` style, consolidated fixture re-baseline

## Phase Details

### Phase 8: Sync Pipeline
**Goal**: `sync` correctly fetches all three new economy categories (Verisium, Uncut Gems at per-level BaseType, Lineage Support Gems) and writes them to `prices.json` in a form `generate` can emit as valid in-game filter blocks.
**Depends on**: Nothing (first phase of v0.3)
**Requirements**: ECON-01, ECON-02, ECON-03
**Success Criteria** (what must be TRUE):
  1. `sync` produces `prices.json` entries for Verisium with `class = "Stackable Currency"` and no special background.
  2. `prices.json` uncut gem entries use per-level base types (e.g. `"Uncut Skill Gem (Level 20)"`) and per-subtype plural classes (`"Uncut Skill Gems"` / `"Uncut Spirit Gems"` / `"Uncut Support Gems"`); no `minLevel` floor silences lower-level entries that price above a tier cutoff.
  3. `prices.json` contains Lineage Support Gem entries with the filter `Class` confirmed by the poe2db spike (blocking: resolve `"Support Gems"` vs `"Skill Gems" "Support Gems"` before committing).
  4. `generate` emits a `Show # Verisium <tier>` block with `Class "Stackable Currency"` and `BaseType ==` containing Verisium base types.
  5. The `overview_types_use_correct_plural_spellings` guard test is extended to assert `Verisium` and `LineageSupportGems` are present and correctly spelled.
**Plans**: 2 plans
  - [x] 08-01-PLAN.md — mapping.json v2: gemFilters array (UncutGems + LineageSupportGems) + Verisium overviewType; reshape GemFilter struct (drop minLevel, add background); extend plural-spelling guard
  - [x] 08-02-PLAN.md — rewrite gem dispatch (per-level BaseType, prefix-derived plural Class, no strip/dedup/minLevel); multi-class emitter + pricer background carry; re-baseline seed fixtures + snapshot (suite green)

### Phase 9: Gem Background Styling
**Goal**: Priced gem blocks in the generated filter carry a per-category fixed background color while preserving the value tier's text and border color — identical in mechanism to the priced-unique bronze background.
**Depends on**: Phase 8
**Requirements**: STYLE-01, STYLE-02
**Success Criteria** (what must be TRUE):
  1. A generated uncut gem block (any tier) contains exactly one `SetBackgroundColor 80 70 55 255` line and no second `SetBackgroundColor` from the tier style.
  2. A generated Lineage Support Gem block (any tier) contains exactly one `SetBackgroundColor 10 150 150 255` line and no second `SetBackgroundColor` from the tier style.
  3. Verisium blocks and all non-gem currency blocks contain no `SetBackgroundColor` override (Dracula bg from the tier style is their sole background).
  4. A guard test asserts no gem block in the generated filter has duplicate `SetBackgroundColor` lines.
**Plans**: 1 plan
  - [x] 09-01-PLAN.md — emit per-category gem background override in emit_currency_sections (clone-null-append, mirroring emit_unique_sections) + no-duplicate-background guard test; re-baseline seed-universal.filter + insta snapshot
**UI hint**: no

### Phase 10: Corrupted Condition + Exceptional Gear
**Goal**: The rule `when` vocabulary gains a `corrupted` boolean condition, and `config.json` contains exceptional-gear rules (extra sockets or high quality, uncorrupted) that surface noteworthy drops before the gear-hide fallback; all fixture snapshots are re-baselined in one consolidated pass.
**Depends on**: Phase 8 (emitter changes are in the same file; sequenced last for a single fixture re-baseline pass)
**Requirements**: EXC-01, EXC-02, EXC-03, EXC-04
**Success Criteria** (what must be TRUE):
  1. `generate` emits `Corrupted False` for any rule with `"corrupted": false` in `when`, and `Corrupted True` for `"corrupted": true`; a rule with no `corrupted` field emits no `Corrupted` line.
  2. The generated filter contains a `Show # exceptional-sockets-2h` block (or equivalent) covering two-hand/body classes with `Sockets >= 3` and `Corrupted False`.
  3. The generated filter contains a `Show # exceptional-sockets-1h` block (or equivalent) covering one-hand/smaller classes with `Sockets >= 2` and `Corrupted False`.
  4. The generated filter contains a `Show # exceptional-quality` block with `Quality >= 24` (author-decided threshold) and `Corrupted False` covering gear classes.
  5. The `exceptional` style produces pink text `255 121 198`, pink border, Dracula background, a pink beam, and a pink cross minimap icon — verifiable in the emitted filter blocks.
  6. `seed-universal.filter` and `generate_block_counts` are re-baselined; `artifact_guard` + `generate_snapshot` pass green.
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v0.1 | 3/3 | Complete | 2026-05-24 |
| 2. Static Generate | v0.1 | 3/3 | Complete | 2026-05-24 |
| 3. Pricing Layer | v0.1 | 4/4 | Complete | 2026-05-25 |
| 4. Sync | v0.1 | 4/4 | Complete | 2026-05-28 |
| 5. Rule Schema | v0.2 | 2/2 | Complete | 2026-06-05 |
| 6. Emitter Rewrite + Migration Parity | v0.2 | 2/2 | Complete | 2026-06-05 |
| 7. New Capabilities + Docs | v0.2 | 3/3 | Complete | 2026-06-12 |
| 8. Sync Pipeline | v0.3 | 2/2 | Complete    | 2026-06-24 |
| 9. Gem Background Styling | v0.3 | 1/1 | Complete    | 2026-06-24 |
| 10. Corrupted Condition + Exceptional Gear | v0.3 | 0/? | Not started | - |

---
*v0.1 shipped 2026-05-28. v0.2 shipped 2026-06-12. v0.3 roadmap defined 2026-06-23.*

## Backlog

### Phase 999.1: Config-driven styles (no hardcoded vocabulary) (BACKLOG)

**Goal:** Any config should be able to define and use new named styles without a code change.
Today the style vocabulary is hardcoded in the `Styles` struct (`src/models/config.rs`) and
`emitter::resolve_style` (`src/emitter.rs`) — adding `craftable` (for `ssf_icewolf_config.json`)
required editing both. Replace the named-field `Styles` struct with a name→`Style` map (or keep
S–F/unchanged/hidden/unidentified as the known palette plus an open `extra` map), resolve styles
by lookup instead of a `match`, and validate at load-time that every `rule.style` resolves.
`FUTURE` comments already mark both code locations.
**Requirements:** TBD
**Plans:** 1/1 plans complete

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)
