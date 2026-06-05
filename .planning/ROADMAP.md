# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- 🔄 **v0.2 Declarative Rule Model** — Phases 5-7 (in progress)

## Phases

<details>
<summary>✅ v0.1 Initial Release (Phases 1-4) — SHIPPED 2026-05-28</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-05-24
- [x] Phase 2: Static Generate (3/3 plans) — completed 2026-05-24
- [x] Phase 3: Pricing Layer (4/4 plans) — completed 2026-05-25
- [x] Phase 4: Sync (4/4 plans) — completed 2026-05-28

Full detail: [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md)

</details>

### v0.2 Declarative Rule Model

- [x] **Phase 5: Rule Schema** — Define the declarative rule model in serde; every condition type is representable in config.json (completed 2026-06-05)
- [ ] **Phase 6: Emitter Rewrite + Migration Parity** — Replace hardcoded emitter with the rule compiler; migrate config.json; re-baseline fixture
- [ ] **Phase 7: New Capabilities + Docs** — Express the 7 in-game edits as config rules; realign CLAUDE.md to the new model

## Phase Details

### Phase 5: Rule Schema
**Goal**: The new rule schema exists in serde — every condition type (class, rarity, area level, base-defence, quality, sockets, UnidentifiedItemTier, BaseType) is modeled, validated, and parseable from config.json
**Depends on**: Phase 4 (v0.1 foundation; emitter untouched this phase)
**Requirements**: RULE-01, RULE-02, COND-01, COND-02, COND-03, COND-04, COND-05, COND-06, MIG-01
**Success Criteria** (what must be TRUE):
  1. `config.json` can be extended with an ordered `rules` array and `cargo test` still passes (serde models parse the new schema with field-path-aware errors on bad input)
  2. A rule referencing a named style tier (S–F, unchanged, hidden, Recycle) deserializes correctly; an unknown tier name is rejected with an actionable error
  3. Every condition field (class list, rarity comparator, area-level min/max, base-defence attributes, quality, sockets, UnidentifiedItemTier, BaseType list) deserializes from its JSON representation
  4. A round-trip serialize/deserialize of a rule list produces identical values (no data loss)
**Plans**: 2 plans
- [x] 05-01-PLAN.md - Add rule-schema types (Rule, When, RarityComparator, numeric/area-level/unidentified conditions) + tests
- [x] 05-02-PLAN.md - Wire rules + classGroups into Config, drop dead areaLevelBands types, round-trip/parity tests

### Phase 6: Emitter Rewrite + Migration Parity
**Goal**: The hardcoded `emit_gear_bands` / `emit_flask_charm_bands` special-casing is gone; `generate` compiles the rule list in order; `config.json` is migrated to the new schema and produces a filter semantically equivalent to the currently installed in-game filter; the artifact-guard fixture is re-baselined
**Depends on**: Phase 5
**Requirements**: RULE-03, MIG-02, MIG-03, MIG-04
**Success Criteria** (what must be TRUE):
  1. `src/emitter.rs` contains no class-specific code paths for gear, flasks, charms, recycle, or jewellery — all policy blocks are produced by iterating the rule list
  2. The frozen pre-edit oracle (`tests/fixtures/parity-baseline-earlygame.filter`) exists in the repo; a semantic comparison confirms the migrated generator's `--early-game` policy output matches it block-for-block (reordering permitted, no behavioral differences). [The full in-game filter incl. the 7 edits — `foo.filter` — is the Phase 7 oracle.]
  3. `cargo test` passes with the full suite; `generate_snapshot` and `artifact_guard` pass against the re-baselined `tests/fixtures/seed-universal.filter`
  4. `generate` with `--early-game` and with a live `prices.json` produces correct currency/unique sections unchanged from v0.1 behavior
**Plans**: 2 plans
- [ ] 06-01-PLAN.md — Rewrite emitter as a generic rule compiler (emit_rules + When→filter-line mapping + unknown-style hard error); migrate config.json to the rules schema (RULE-03)
- [ ] 06-02-PLAN.md — Semantic parity test vs the frozen --early-game oracle; re-baseline seed-universal.filter; confirm pricer sections unchanged (MIG-02/03/04)

### Phase 7: New Capabilities + Docs
**Goal**: All 6 capability requirements (the 7 in-game edits) are expressible and exercised in `config.json`; CLAUDE.md is realigned to the declarative model with confirmed-status filters
**Depends on**: Phase 6
**Requirements**: CAP-01, CAP-02, CAP-03, CAP-04, CAP-05, CAP-06, DOC-01
**Success Criteria** (what must be TRUE):
  1. `config.json` contains rules that express: build-weapon Magic+Rare up to a configurable area-level cap, build-armour upper cap, flask/charm mid-band Magic+Rare with rarity-bounded Hide fallback, charms independently targeted (Normal ≥ 78), jewellery hide (Rings/Amulets/Belts ≤ Rare), and UnidentifiedItemTier highlight — and `generate` emits the corresponding filter blocks
  2. Changing a cap value in a rule (e.g. weapon Magic+Rare max area level) produces a different filter block with the new bound — confirming the rule, not hardcoded logic, drives output
  3. `CLAUDE.md` no longer contains "unverified" or "verify before use" caveats for `UnidentifiedItemTier` or base-defence filters; the rule model schema is documented as the authoritative generator input
  4. `cargo test` passes with the full suite after the capability rules and doc changes are applied
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v0.1 | 3/3 | Complete | 2026-05-24 |
| 2. Static Generate | v0.1 | 3/3 | Complete | 2026-05-24 |
| 3. Pricing Layer | v0.1 | 4/4 | Complete | 2026-05-25 |
| 4. Sync | v0.1 | 4/4 | Complete | 2026-05-28 |
| 5. Rule Schema | v0.2 | 2/2 | Complete    | 2026-06-05 |
| 6. Emitter Rewrite + Migration Parity | v0.2 | 0/2 | Planned | - |
| 7. New Capabilities + Docs | v0.2 | 0/? | Not started | - |

---
*v0.1 shipped 2026-05-28. v0.2 roadmap created 2026-06-05.*
