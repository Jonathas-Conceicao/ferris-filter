# Requirements: FerrisFilter — v0.3 Economy Items & Exceptional Gear

**Defined:** 2026-06-23
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.

## v0.3 Requirements

Requirements for the v0.3 milestone. Each maps to a roadmap phase.

### Economy Categories (ECON)

- [x] **ECON-01
**: `sync` fetches Verisium (poe.ninja `type=Verisium`) and `generate` tiers it as regular stackable currency (`Class "Stackable Currency"`, `BaseType ==`), with no special background.
- [x] **ECON-02
**: `sync` fetches Lineage Support Gems (poe.ninja `type=LineageSupportGems`) — each gem's name as `BaseType` — and `generate` emits them with the correct filter `Class` (resolved by the blocking spike: `"Support Gems"` vs `"Skill Gems" "Support Gems"`).
- [x] **ECON-03
**: `generate` emits Uncut Gems that actually match in-game — the level is kept in the `BaseType` (`Uncut Skill/Spirit/Support Gem (Level N)`, one priced entry per level) and the `Class` is the per-subtype plural (`Uncut Skill Gems` / `Uncut Spirit Gems` / `Uncut Support Gems`). The `minLevel` floor is removed.

### Gem Styling (STYLE)

- [x] **STYLE-01
**: Priced gem categories carry a fixed per-category background override that replaces the Dracula background while preserving the value tier's text + border color (same mechanism as priced uniques' bronze background — not a new named style).
- [x] **STYLE-02
**: Uncut gems render with background `80 70 55`; Lineage Support Gems render with background `10 150 150`.

### Exceptional Gear (EXC)

- [ ] **EXC-01**: The rule `when` vocabulary supports a `corrupted` boolean condition, emitting `Corrupted True` / `Corrupted False`.
- [ ] **EXC-02**: `generate` highlights uncorrupted gear with extra sockets — `Sockets >= 3` for two-hand/body classes, `Sockets >= 2` for one-hand/smaller classes (separate rules; filter blocks are AND-only).
- [ ] **EXC-03**: `generate` highlights uncorrupted gear with `Quality >= 24`.
- [ ] **EXC-04**: Exceptional gear (EXC-02/03) uses a new dedicated `exceptional` named style, modeled on `Recycle` (tier-style text + border, Dracula background, colored beam, colored cross minimap icon), colored pink `255 121 198` for now. It is a *separate* style from S (same pink today) so the color can be retuned independently later.

## Future Requirements

Deferred — acknowledged, not in the v0.3 roadmap.

### Economy (next milestone candidates)

- **ECONX-01**: BaseType-matched, economy-tiered Tablets category (class-less `BaseType ==` blocks; corrected base names). *(v0.2 backlog — needs class-less category support.)*

### Generate / Sync QoL

- **GENX-01..04**: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning.
- **SYNX-01..02**: multiple build profiles; alternate price-source fallback.

### Tooling

- **STYLEX-01**: Config-driven style vocabulary (Phase 999.1 backlog) — define new named styles in config without a code change. *(Not pulled in: v0.3's `exceptional` style is added in code, consistent with the existing `unidentified`/`craftable` pattern.)*

## Out of Scope

Explicitly excluded for v0.3. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Tablets BaseType category | Deferred to backlog (ECONX-01); needs class-less category support — out of scope this milestone (author directive). |
| Config-driven style vocabulary (999.1) | The one new `exceptional` style is hardcoded like `unidentified`/`craftable`; the general refactor stays in backlog. |
| `GemLevel` `when` condition | Uncut gems use level-in-BaseType per the author's tested reference filters, so a `GemLevel` condition is not needed. |
| Single combined `sockets`+`quality` exceptional rule | Filter blocks are AND-only — that would require an item to have *both*. OR is expressed as separate rules (EXC-02/03). |
| `Corrupted == False` syntax | Boolean conditions take no operator; correct form is `Corrupted False`. |
| Blanket gem restyle / new price source | Keep the v0.1/v0.2 economy pipeline and design philosophy intact. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ECON-01 | Phase 8 | Complete |
| ECON-02 | Phase 8 | Complete |
| ECON-03 | Phase 8 | Complete |
| STYLE-01 | Phase 9 | Complete |
| STYLE-02 | Phase 9 | Complete |
| EXC-01 | Phase 10 | Pending |
| EXC-02 | Phase 10 | Pending |
| EXC-03 | Phase 10 | Pending |
| EXC-04 | Phase 10 | Pending |

**Coverage:**
- v0.3 requirements: 9 total
- Mapped to phases: 9 (100%) ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-23*
*Last updated: 2026-06-23 — traceability filled during roadmap creation*
