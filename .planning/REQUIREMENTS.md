# Requirements: FerrisFilter

**Defined:** 2026-05-23
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.

## v1 Requirements

Requirements for initial release. Each maps to a roadmap phase.

### CLI

- [ ] **CLI-01**: User can run `ferrisfilter generate` to compile the filter
- [ ] **CLI-02**: User can run `ferrisfilter sync` to fetch prices
- [ ] **CLI-03**: `generate` accepts `--output <path>` (defaults to `universal.filter`)
- [ ] **CLI-04**: `sync` accepts combinable `--standard` and `--hc` flags; with neither, it targets the current trade league (softcore)
- [ ] **CLI-05**: Every subcommand exposes `--help`, and the binary exits non-zero with a clear message on failure

### Config

- [ ] **CFG-01**: Filter policy lives in `config.json` (renamed from `filter-model.json`): `styles`, build classes, `areaLevelBands`, `alwaysShow`
- [ ] **CFG-02**: `generate` validates `config.json` and reports actionable errors (field path + expected) instead of panicking
- [ ] **CFG-03**: Tier cutoffs live in `tiers.json` and are validated on load

### Generate

- [ ] **GEN-01**: `generate` deterministically compiles `config.json` (+ `tiers.json` + `prices.json`) into `universal.filter` — identical inputs always produce byte-identical output
- [ ] **GEN-02**: The generated filter reproduces the established structure: style tiers, build-weapon/armour area-level bands, flask/charm ladder, recycle, currency tiers, gear-hide fallthrough — matching the existing hand-built `universal.filter`
- [ ] **GEN-03**: Each generated block carries a comment linking it to its config source (model path tag)
- [ ] **GEN-04**: Uniques (untracked), gems, jewels, jewellery, quest items, waystones, trials, and unidentified items receive no rules and fall through to game default
- [ ] **GEN-05**: `generate` runs without `prices.json` present — it warns and skips price-tiered rules rather than failing

### Tiering

- [ ] **TIER-01**: `generate` derives currency cross-rates from the first three `prices.json` entries (Exalt, Chaos, Divine) and converts every priced item to a chaos value
- [ ] **TIER-02**: `generate` assigns each priced item to a tier per the `tiers.json` cutoffs (chaos as the reference unit)
- [ ] **TIER-03**: Valuable uniques present in `prices.json` are highlighted by their price tier; uniques sharing a base type resolve to the highest tier among them; untracked uniques fall through
- [ ] **TIER-04**: `generate` aborts with a clear error if a reference currency price (Exalt/Chaos/Divine) is missing or zero (no silent miscalculation)

### Sync

- [ ] **SYNC-01**: `sync` fetches PoE2 economy data from poe.ninja and writes `prices.json`
- [ ] **SYNC-02**: `prices.json` is an ordered list of `item → {exalt, chaos, div}` (any value may be null); the first three entries are always Exalt, Chaos, Divine
- [ ] **SYNC-03**: `sync` includes currency, uniques, and other poe.ninja-tracked drops; uncut gems are included only at level ≥ 18
- [ ] **SYNC-04**: `sync` resolves the correct league for the selected flags and handles missing/empty responses without corrupting an existing `prices.json`
- [ ] **SYNC-05**: `sync` records the fetched league and a timestamp in `prices.json`

## v2 Requirements

Deferred to a future release. Tracked but not in the current roadmap.

### Generate QoL

- **GENX-01**: `generate --dry-run` prints to stdout without writing
- **GENX-02**: `generate --diff` shows a line-level diff against the existing output
- **GENX-03**: `-v / --verbose` logs rule derivation to stderr
- **GENX-04**: `generate` warns when `prices.json` is stale beyond a configurable age

### Sync extensions

- **SYNX-01**: Multiple build profiles (`--config builds/<name>.json`)
- **SYNX-02**: Alternate price source (e.g. poe2scout) as a fallback

## Out of Scope

| Feature | Reason |
|---------|--------|
| GUI / web interface | CLI-only tool; FilterBlade already covers GUI filter editing |
| Auto-install into game folder | `install.bat` already copies the filter; OS-path coupling is fragile |
| Blanket `Rarity == Unique` styling | Violates the design philosophy; only price-tracked uniques get rules |
| NeverSink-style strictness levels | `areaLevelBands` already encode progression; a strictness dial would conflict |
| Rules for gems / jewels / jewellery / quest / waystones / trials / unidentified | Established philosophy: these fall through to game default |
| Historical price trends / sparklines | poe.ninja PoE2 API does not expose this; out of scope for a filter compiler |

## Traceability

Populated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| (to be mapped by roadmapper) | — | Pending |

**Coverage:**
- v1 requirements: 20 total
- Mapped to phases: 0 (pending roadmap)
- Unmapped: 20 ⚠️

---
*Requirements defined: 2026-05-23*
*Last updated: 2026-05-23 after initial definition*
