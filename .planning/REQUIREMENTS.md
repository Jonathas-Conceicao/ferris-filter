# Requirements — v0.2 Declarative Rule Model

**Milestone goal:** Replace the generator's hardcoded gear/flask band logic with a general,
declarative rule schema in `config.json`, so any class + level-range + rarity (+ defence /
quality / sockets / unidentified-tier / base-type) policy is expressible in config — making
the fully-functional in-game filter reproducible from config alone.

**Acceptance bar (author-set):** Running `generate` on the migrated config produces a filter
**semantically equivalent** to the filter currently installed in the game folder. Block
**reordering is acceptable**; any other behavioral change is surfaced for author review.

**Scope:** config-flexibility only. CLI surface (`generate` / `sync` / `install`) is
unchanged. Price-driven currency/category + unique sections are untouched. Generate QoL and
sync extensions are deferred.

---

## v0.2 Requirements

### Rule Engine (RULE)

- [ ] **RULE-01**: Author can define filter policy as an ordered list of rules in `config.json`; the generator emits one `Show`/`Hide` block per rule, in list order, preserving first-match-wins semantics.
- [ ] **RULE-02**: Each rule selects its visuals and action by referencing a named style tier (`S`–`F`, `unchanged`, `hidden`, `Recycle`); whether the block is `Show` or `Hide` follows the referenced style's action.
- [ ] **RULE-03**: The hardcoded gear/armour/otherGear/flask special-casing is removed from the emitter; all gear, flask, charm, recycle, jewellery, and unidentified policy is produced from the rule list (no class-specific code paths remain in emission).

### Rule Conditions (COND)

- [ ] **COND-01**: A rule can match on a list of item classes (any-of).
- [ ] **COND-02**: A rule can constrain rarity with a comparator (`==` / `>=` / `<=` over Normal/Magic/Rare/Unique).
- [ ] **COND-03**: A rule can constrain area level with a minimum and/or maximum bound (either side optional, i.e. open-ended ranges allowed).
- [ ] **COND-04**: A rule can constrain numeric item attributes — base-defence (`BaseArmour` / `BaseEvasion` / `BaseEnergyShield`), `Quality`, and `Sockets` — each emitted as the corresponding filter condition.
- [ ] **COND-05**: A rule can match against a `BaseType` list.
- [ ] **COND-06**: A rule can constrain `UnidentifiedItemTier` with a threshold.

### New Capabilities — the in-game edits, now config (CAP)

- [ ] **CAP-01**: Build weapons (Staves/Wands/Foci) can be shown Magic+Rare up to a configurable area level, then fall through. *(edit #1)*
- [ ] **CAP-02**: Build-armour bands honor a configurable upper area-level cap. *(edit #2)*
- [ ] **CAP-03**: Flask/charm mid-band can show Magic+Rare, and the flask/charm `Hide` fallback can be bounded by rarity. *(edits #5, #7)*
- [ ] **CAP-04**: Charms can be targeted independently of flasks (e.g. show Normal charms at area level ≥ 78). *(edit #6)*
- [ ] **CAP-05**: Low-rarity jewellery (Rings/Amulets/Belts, ≤ Rare) can be hidden via config. *(edit #4)*
- [ ] **CAP-06**: High-tier unidentified items (`UnidentifiedItemTier >= N`) can be highlighted, for configurable class groups. *(edit #3)*

### Migration & Parity (MIG)

- [ ] **MIG-01**: `config.json` is migrated to the new rule schema; serde models validate it with field-path + expected-type errors, consistent with v0.1 input modeling.
- [ ] **MIG-02**: `generate` on the migrated config produces a filter semantically equivalent to the current in-game filter (game folder) — block reordering allowed; any difference beyond ordering is surfaced for author review.
- [ ] **MIG-03**: The current in-game filter is captured into the repo as the parity oracle, and the artifact-guard/snapshot fixture (`tests/fixtures/seed-universal.filter`) is re-baselined deliberately to the new generator output with the full test suite passing.
- [ ] **MIG-04**: Price-driven currency/category and unique sections continue to be emitted from sync data, unchanged by the redesign (including the `--early-game` fallback path).

### Docs (DOC)

- [ ] **DOC-01**: `CLAUDE.md` is realigned to the rule model (filter-grammar reference retained), and the "unverified" caveats for `UnidentifiedItemTier` + base-defence filters are updated to confirmed.

---

## Future Requirements (deferred)

- Generate QoL: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- Sync extensions: multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- Lenient-parse guard for Standard-league unique endpoints; optional no-flag `sync` → `--standard` hint when no challenge league is active

## Out of Scope

- New CLI subcommands or flags — v0.2 is an input-model + emitter redesign only.
- Changing the price-driven currency/category or unique tiering logic — carried over from v0.1 untouched.
- Filter rules for gems, jewels, quest items, waystones, trials — these still fall through to game default.
- Byte-for-byte reproduction of the old generated output — the baseline is re-locked to the new generator; equivalence is **semantic**, not byte-identical.

---

## Traceability

| REQ-ID | Phase |
|--------|-------|
| RULE-01 | Phase 5 |
| RULE-02 | Phase 5 |
| RULE-03 | Phase 6 |
| COND-01 | Phase 5 |
| COND-02 | Phase 5 |
| COND-03 | Phase 5 |
| COND-04 | Phase 5 |
| COND-05 | Phase 5 |
| COND-06 | Phase 5 |
| CAP-01 | Phase 7 |
| CAP-02 | Phase 7 |
| CAP-03 | Phase 7 |
| CAP-04 | Phase 7 |
| CAP-05 | Phase 7 |
| CAP-06 | Phase 7 |
| MIG-01 | Phase 5 |
| MIG-02 | Phase 6 |
| MIG-03 | Phase 6 |
| MIG-04 | Phase 6 |
| DOC-01 | Phase 7 |
