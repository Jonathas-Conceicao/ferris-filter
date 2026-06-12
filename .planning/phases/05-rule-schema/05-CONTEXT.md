# Phase 5: Rule Schema - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Model the new declarative rule schema in serde (`src/models/config.rs`) so every condition
type — class, rarity, area-level, base-defence, quality, sockets, UnidentifiedItemTier,
BaseType — is parseable, validated, and round-trips from `config.json`. This phase is
DATA-MODEL ONLY: the emitter is NOT touched here (that is Phase 6). The schema must be
expressive enough to represent the entire current in-game filter's policy section as an
ordered list of rules, and to express the 6 new capabilities (CAP-01..06) in Phase 7.

Price-driven currency/category and unique sections stay OUT of the rule model — they remain
appended by the pricer (MIG-04), so the schema only covers the policy/gear/flask/recycle/
jewellery/unidentified blocks.

</domain>

<decisions>
## Implementation Decisions

### Rule object shape
- Rules live in a new top-level ordered array: `"rules": [ ... ]` in config.json.
- Each rule groups its match conditions under a nested `"when": { ... }` object with optional condition fields (extensible, clean separation from action).
- A rule chooses visuals + action via `"style": "<tier-name>"` only; whether the emitted block is `Show` or `Hide` is derived from the referenced style's `action` (RULE-02). No separate per-rule action field.
- A rule may carry an optional `"name"` string → emitted as the block comment header (`Show # <name>`). When absent, the emitter derives a label.

### Condition representation
- **Class:** `"class": ["Staves", ...]` → emits `Class == "..."` (exact full-name match, current convention).
- **Rarity (OVERRIDE — compact string):** `"rarity": "<op><Rarity>"` where `<op>` is `==`, `>=`, or `<=` and a bare value (no op) means `==`. Examples: `">=Magic"` → `Rarity >= Magic`; `"==Rare"` / `"Rare"` → `Rarity == Rare`; `"<=Rare"` → `Rarity <= Rare`. Rarity vocabulary: Normal/Magic/Rare/Unique.
- **Area level:** `"areaLevel": { "min": N, "max": M }` — both bounds optional (open-ended allowed). Emits `AreaLevel >= N` and/or `AreaLevel <= M`.
- **Numeric attributes (structured + validated):** base-defence/quality/sockets are structured operator objects, not raw passthrough strings. E.g. `"baseEvasion": {"eq": 0}`, `"baseEnergyShield": {"min": 1}` (i.e. `> 0`), `"baseArmour": {"eq": 0}`, `"quality": {"min": 1}`, `"sockets": {"min": 1}`. Operators map to filter conditions (`eq`→`X 0`/exact, `min`→`>=`, etc.). (Planner to finalize the exact operator set — must cover `BaseEvasion 0`, `BaseEnergyShield > 0`, `BaseArmour 0`, `Quality >= 1`, `Sockets >= 1` as used today.)

### Extended conditions & boundaries
- **UnidentifiedItemTier:** `"unidentifiedTier": {"min": N}` → `UnidentifiedItemTier >= N`.
- **BaseType:** `"baseType": ["...", ...]` → `BaseType == "..."` (exact).
- **Price-driven sections stay separate** — currency/category + unique blocks remain pricer-appended, NOT part of the rule list (MIG-04, untouched).
- **Class-group aliases:** add an optional `"classGroups"` map (e.g. `"buildWeapons": ["Staves","Wands","Foci"]`) that a rule's `when.class` can reference by name to avoid repetition. The dead `areaLevelBands` config block (and its now-unused `AreaLevelBand`/`BandShow`/`ArmourShow` types) is dropped as part of the migration.

### Validation & defaults
- A rule with an empty/absent `when` is ALLOWED — it matches everything (useful as a catch-all/fallback).
- An unknown `style` name is a HARD ERROR with a field-path-aware message (consistent with existing validation).
- Rule and `when` objects use `deny_unknown_fields` to catch typos (matches existing serde models).
- Rule ordering is preserved exactly as authored (first-match-wins); no auto-sorting.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/models/config.rs` — existing serde models: `Style`/`Styles` (named-tier fields incl. `unchanged`/`hidden`/`Recycle`), `FilterColor`/`MinimapShape`/`StyleAction` closed enums, `Build`, `Meta`. The `Styles` struct is the lookup target for a rule's `style` name.
- `RarityRule` closed enum (Any/RareOnly/MagicPlusRare/Magic/QualityOnly) + `AreaLevelBand`/`BandShow`/`ArmourShow` — currently PARSED BUT UNUSED (zero consumers outside config.rs). These are candidates for removal/replacement by the rule model.
- Validation pattern: `#[serde(deny_unknown_fields)]` + field-path/expected-type errors already established across all models.

### Established Patterns
- `#[serde(rename_all = "camelCase")]` + explicit `rename`/`alias` for JSON↔Rust naming.
- Closed-vocabulary enums (D-07) for any fixed value set; `deny_unknown_fields` rejects typos (e.g. tier "SS").
- Tests: serde round-trip + validation-error tests live alongside models; config.json is the live fixture.

### Integration Points
- `Config` top-level struct (src/models/config.rs ~line 247) gains `rules: Vec<Rule>` and optional `class_groups`. The emitter consumes `rules` in Phase 6 (not this phase).
- Loader (`src/loader.rs`) parses Config; new types must flow through unchanged.

</code_context>

<specifics>
## Specific Ideas

- Rarity is authored as a compact operator string (user preference), not an object.
- The schema must be able to represent the FULL current in-game filter policy section (the parity oracle) — keep that filter in view when finalizing condition coverage so Phase 6 migration is mechanical.

</specifics>

<deferred>
## Deferred Ideas

- Actually rewriting the emitter to consume `rules` and migrating `config.json` — Phase 6.
- Expressing the 6 new capabilities / 7 in-game edits as concrete rules — Phase 7.

</deferred>
