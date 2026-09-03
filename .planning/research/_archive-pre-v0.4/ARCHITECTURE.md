# Architecture Research — FerrisFilter v0.3 Integration

**Domain:** CLI filter compiler + economy sync (Path of Exile 2)
**Researched:** 2026-06-23
**Confidence:** HIGH — all findings are from direct code trace of the committed source

---

## 1. Bronze-Background Code Path (Priced Uniques)

### Exact trace

`emit_unique_sections` in `src/emitter.rs` (lines 427–466):

```
emit_unique_sections(out, config, &priced.unique_blocks)
  for block in blocks:
    let style = style_for_tier(&config.styles, &block.tier)   // gets S/A/B/C/D/E/F Style
    let mut unique_style = style.clone()
    unique_style.background = None                             // suppress Dracula bg
    emit_style_actions(out, &unique_style)                     // beam, minimap, text, border, font, sound
    writeln!(out, "  SetBackgroundColor 175 96 37 240")        // bronze injected AFTER style actions
```

Key observations:
- The bronze background is NOT stored in any `Style` struct. It is **hardcoded in `emit_unique_sections`** as a literal `writeln!` after `emit_style_actions`.
- The mechanism is: clone the tier's `Style`, null out its `background` field, emit actions, then append the override color manually.
- `emit_style_actions` emits background at position 4 in a fixed canonical order (beam, minimap, text, bg, border, font, sound, disablesound). By nulling `background` on the clone and then writing `SetBackgroundColor` after the call, the output puts the background AFTER border and font — a minor deviation from canonical order that is tested and committed in the fixture.

### Cleanest replication for gems

Two valid approaches, ordered by preference:

**Option A (recommended): emitter-level per-category override (mirrors the unique pattern exactly)**

Add an `Option<[u8; 3]>` background field to `CategoryBlock` (in `src/pricer.rs`). `emit_currency_sections` checks it and, when present, clones the style, nulls its background, calls `emit_style_actions`, then writes `SetBackgroundColor R G B 255`. This is exactly what `emit_unique_sections` does — zero new mechanism, no new style names.

Change surface:
- `src/pricer.rs`: add `background_override: Option<[u8; 3]>` to `CategoryBlock`
- `src/pricer.rs` (`price_tiers`): populate it from category name (`"Uncut Gems"` → `[80, 70, 55]`, `"Lineage Support Gems"` → `[10, 150, 150]`, others → `None`)
- `src/emitter.rs` (`emit_currency_sections`): replicate the clone-null-override pattern when `category.background_override.is_some()`

**Option B: mapping.json `backgroundOverride` field**

Add `background_override: Option<[u8; 3]>` to `GemFilter` (and future gem-like `OverviewType` entries). The sync layer populates `CategoryBlock.background_override` from the mapping entry rather than from a category-name lookup. Slightly more flexible, requires touching `src/sync/mapping.rs` (model) and `src/sync/prices.rs` (propagation) in addition to the pricer/emitter.

Option A is preferred for v0.3 because the background values are fixed per-category and do not need to be in the mapping file. Option B is the right choice if gem category backgrounds are expected to change between leagues.

---

## 2. Uncut Gem Level Embedding

### Current behavior (v0.2 / committed)

In `src/sync/prices.rs`, `build_prices` processes the `UncutGems` poe.ninja type:

1. `parse_gem_level(&name)` extracts N from `"Uncut Skill Gem (Level N)"`.
2. `if level < mapping.gem_filter.min_level { continue; }` — drops level < 18.
3. `strip_gem_level(&name)` returns `"Uncut Skill Gem"` (no suffix).
4. The resulting `PriceItem` has `name = base_type = "Uncut Skill Gem"` — the level is gone.
5. The emitter emits `BaseType == "Uncut Skill Gem"` — a level-agnostic rule.

### What v0.3 requires

The in-game filter must use `BaseType == "Uncut Skill Gem (Level N)"` because PoE2 distinguishes gem levels for matching purposes. Currently the emitter strips the level and loses the per-level discrimination.

### Where the change goes

The stripping happens at line 204 of `src/sync/prices.rs`:
```rust
let base = strip_gem_level(&name);
```

For v0.3, **do not strip**. Keep `name` as the base_type (i.e., `"Uncut Skill Gem (Level 20)"`). Each level becomes a separate `PriceItem` and a separate tier group, emitting its own `BaseType == "Uncut Skill Gem (Level 20)"` block. The dedup-by-highest-value logic (`gem_best` HashMap) must also be removed, because levels are now distinct products, not duplicates.

Change surface:
- `src/sync/prices.rs` (`build_prices`, gem branch): remove `strip_gem_level` call; keep `name` as both `name` and `base_type`; remove the `gem_best` HashMap dedup; each level is a distinct entry.
- `src/sync/prices.rs`: drop the `min_level` check (set `min_level: 0` in `mapping.json` or skip the guard entirely).
- `mapping.json`: set `"minLevel": 0` (or remove the field if we make it optional).
- `src/sync/mapping.rs` (`GemFilter`): make `min_level` optional (`Option<u32>`) or leave at 0.
- `src/emitter.rs`: no change needed — `emit_currency_sections` already emits whatever is in `base_type`.
- `tests/fixtures/seed-prices.json` and `seed-universal.filter`: must be re-baselined after the change.

### Ripple: min_level removal

`gem_filter.min_level` is currently checked in one place: the `if level < mapping.gem_filter.min_level` guard in `build_prices`. Removing or zeroing it is a one-line change. The `parse_gem_level` and `strip_gem_level` helpers remain in place (they are tested independently) but stop being called in the main path.

---

## 3. New Categories: Verisium and LineageSupportGems

### Verisium

Verisium is a regular stackable currency item on the poe.ninja exchange endpoint. It requires:

1. **`mapping.json`**: add one new entry to `overviewTypes`:
   ```json
   { "type": "Verisium", "category": "Verisium", "class": "Stackable Currency", "baseTypeIsName": true }
   ```
2. **`src/sync/prices.rs`**: no code change. `run_sync_prices` iterates `mapping.overview_types` generically; the new entry is fetched automatically.
3. **`src/pricer.rs` / `src/emitter.rs`**: no code change. `price_tiers` groups items by `category`; `emit_currency_sections` emits a `# --- Verisium ---` section automatically.

No new struct, no new code path. Pure data change. Confirm poe.ninja serves `type=Verisium` on the exchange endpoint before committing.

### LineageSupportGems

LineageSupportGems are a gem-category item (level-bearing, per-level pricing) with a fixed background. The key question is whether they use the same exchange endpoint as UncutGems or a separate one.

**If poe.ninja serves LineageSupportGems on the exchange endpoint (same as UncutGems):**

Option A: add a second `GemFilter`-like entry. The current `Mapping` struct has a single `gem_filter: GemFilter` field. To support a second gem category, either:
- Promote `gem_filter` to `gem_filters: Vec<GemFilter>` (generalized, handles any number of level-bearing categories)
- Add a separate `lineage_gem_filter: Option<GemFilter>` field (minimal change)

The `Vec<GemFilter>` approach is cleaner for the long term. `build_prices` already dispatches on `type_value == &mapping.gem_filter.r#type`; generalize to `mapping.gem_filters.iter().any(|g| g.r#type == type_value)`.

**If poe.ninja serves LineageSupportGems on the item overview endpoint (like uniques):**

They would enter via `build_unique_items` with `name != base_type` behavior — which is wrong for gems (we want currency-style category blocks, not `Rarity == Unique` blocks). This would require a new dispatch path.

**Resolution required**: spike poe.ninja to confirm which endpoint and response shape LineageSupportGems use before implementing. The architecture recommendation below assumes the exchange endpoint (same as UncutGems).

Change surface (exchange endpoint assumption):
- `mapping.json`: replace `"gemFilter": {...}` with `"gemFilters": [{ ...uncutGems... }, { ...lineageSupportGems... }]`
- `src/sync/mapping.rs`: rename `gem_filter: GemFilter` to `gem_filters: Vec<GemFilter>`; update `load_mapping`, `Mapping::lookup` if applicable
- `src/sync/prices.rs` (`build_prices`): replace single `gem_filter` dispatch with iteration over `gem_filters`; each gem filter has its own `gem_best` accumulator
- `src/pricer.rs`: add `background_override: Option<[u8; 3]>` to `CategoryBlock` (see section 1); populate from the `GemFilter` entry's category
- `src/emitter.rs`: use the override when emitting

---

## 4. Exceptional Gear via Rules

### What `when` currently supports (from `src/models/config.rs`, `When` struct)

| Field | Emitted condition | Status |
|-------|-------------------|--------|
| `class` | `Class "..."` | Present |
| `rarity` | `Rarity == / >= / <=` | Present |
| `areaLevel` | `AreaLevel >= / <=` | Present |
| `baseArmour` | `BaseArmour >= N` | Present |
| `baseEvasion` | `BaseEvasion >= N` | Present |
| `baseEnergyShield` | `BaseEnergyShield >= N` | Present |
| `quality` | `Quality >= N` | Present |
| `sockets` | `Sockets >= N` | Present |
| `baseType` | `BaseType == "..."` | Present |
| `unidentifiedTier` | `UnidentifiedItemTier >= N` | Present |
| `corrupted` | `Corrupted True/False` | **ABSENT** |

### OR-of-conditions

PoE2 filter blocks have AND semantics within a block. There is no OR operator. To express `(Sockets >= 2 OR Quality >= 10) AND NOT Corrupted` you need two separate rules:

```json
{ "name": "exceptional gear: extra sockets", "style": "C",
  "when": { "sockets": {"min": 2}, "corrupted": false, "class": [...gear classes...] } },
{ "name": "exceptional gear: extra quality", "style": "C",
  "when": { "quality": {"min": 10}, "corrupted": false, "class": [...gear classes...] } }
```

Both rules must appear in `config.json` before the gear hide rules.

### Adding `corrupted` to `When`

The `Corrupted` filter condition takes a boolean (`True` or `False`). The schema addition:

In `src/models/config.rs`, `When` struct:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub corrupted: Option<bool>,
```

In `src/emitter.rs`, `emit_rules`, in the `when` condition emission block (after the `unidentifiedTier` arm, step 11):
```rust
// 11. Corrupted
if let Some(c) = when.corrupted {
    writeln!(out, "  Corrupted {}", if c { "True" } else { "False" }).unwrap();
}
```

That is the complete change. The `When` struct uses `deny_unknown_fields`, so the new field must also be added; serde's `#[serde(default)]` (or `Option<bool>` with `skip_serializing_if`) handles backward compatibility — existing rules without `"corrupted"` parse as `None` and emit nothing.

### Defining "exceptional gear" targets

The `class` field lists which gear slots to watch. For extra-sockets, equippable gear that can have sockets includes weapons and armour. For extra-quality, all equippable gear.

The exact `sockets` and `quality` thresholds are policy decisions (author-editable in `config.json`), not architecture decisions. A reasonable default for extra sockets is `{"min": 2}` (PoE2 base max is 1 socket for most items; 2+ means the item rolled an extra socket mod). For extra quality, `{"min": 15}` or `{"min": 20}` depending on league.

### Full schema additions required

- `src/models/config.rs` `When`: add `pub corrupted: Option<bool>`
- `src/emitter.rs` `emit_rules`: add arm 11 emitting `Corrupted True/False`
- `config.json` `rules`: insert two new rules (sockets-exceptional and quality-exceptional) before the gear hide fallback rules, each with `"corrupted": false`
- No changes to `src/pricer.rs`, `src/sync/`, or `mapping.json`

---

## 5. Component Map: New vs Modified

### Files modified

| File | Change type | What changes |
|------|-------------|--------------|
| `mapping.json` | Data | Add Verisium to `overviewTypes`; replace `gemFilter` with `gemFilters` array including LineageSupportGems |
| `src/sync/mapping.rs` | Minor modification | `gem_filter: GemFilter` → `gem_filters: Vec<GemFilter>`; add `background_override: Option<[u8;3]>` to `GemFilter` |
| `src/sync/prices.rs` | Moderate modification | Generalize gem dispatch to iterate `gem_filters`; remove `strip_gem_level`/`gem_best` dedup; remove `min_level` guard |
| `src/pricer.rs` | Minor modification | Add `background_override: Option<[u8;3]>` to `CategoryBlock`; populate it in `price_tiers` from gem category |
| `src/emitter.rs` | Minor modification | In `emit_currency_sections`: clone-null-emit-override pattern when `background_override.is_some()`; add `Corrupted` arm in `emit_rules` |
| `src/models/config.rs` | Minor modification | Add `corrupted: Option<bool>` to `When` |
| `config.json` | Data | Add two exceptional-gear rules; optionally add `"corrupted"` to existing rules that need it |
| `tests/fixtures/seed-prices.json` | Re-baseline | Add per-level uncut gem entries; add Verisium if present in seed |
| `tests/fixtures/seed-universal.filter` | Re-baseline | New blocks for per-level gems, background overrides, exceptional-gear rules |

### Files not modified

- `src/models/prices.rs` — `PriceItem` needs no new fields; category and class already carry all needed info
- `src/sync/league.rs`, `src/sync/client.rs`, `src/sync/models.rs` — no change
- `src/models/tiers.rs` — no change
- `tiers.json`, `tiers-fallback.json` — no change

---

## 6. Recommended Build Order

Dependencies drive this order: each step's output is consumed by the next.

**Step 1: mapping.json + sync/mapping.rs (data foundation)**
- Add Verisium to `overviewTypes`
- Replace `gem_filter` with `gem_filters: Vec<GemFilter>`; each entry gets `background_override: Option<[u8;3]>`
- Update `load_mapping`, fix all callsites
- Tests: mapping parse tests update; existing tests for gem_filter become gem_filters[0]

**Step 2: sync/prices.rs (sync pipeline)**
- Generalize gem dispatch to iterate `gem_filters`
- Remove `strip_gem_level` call, remove `gem_best` dedup, remove `min_level` guard
- Each gem level (e.g., `"Uncut Skill Gem (Level 20)"`) is now a distinct `PriceItem`
- Tests: update `gem_level_filter_keeps_only_min_level_and_above` → replace with level-preservation tests; re-baseline `seed-prices.json`

**Step 3: pricer.rs (background_override propagation)**
- Add `background_override: Option<[u8;3]>` to `CategoryBlock`
- In `price_tiers`, match category name to background: `"Uncut Gems"` → `[80, 70, 55]`, `"Lineage Support Gems"` → `[10, 150, 150]`
- Tests: unit test that `CategoryBlock` for gem categories carries the correct background

**Step 4: emitter.rs (output)**
- In `emit_currency_sections`: when `category.background_override.is_some()`, clone style, null its background, call `emit_style_actions`, then write `SetBackgroundColor`
- Re-baseline `seed-universal.filter`
- Tests: update `generate_block_counts`; add snapshot test for gem block with background

**Step 5: models/config.rs + emitter.rs (corrupted condition)**
- Add `corrupted: Option<bool>` to `When`
- Add arm 11 in `emit_rules`
- Tests: unit tests for `Corrupted True` and `Corrupted False` emission; `deny_unknown_fields` ensures no silent breakage

**Step 6: config.json (policy authoring)**
- Add two exceptional-gear rules using `"corrupted": false` and either `"sockets"` or `"quality"`
- Re-baseline `seed-universal.filter` if rules affect its block count
- Tests: `generate_block_counts` assertion will need updating (currently asserts 84 Show / 3 Hide)

---

## 7. Open Architecture Questions

1. **LineageSupportGems endpoint**: Confirm whether poe.ninja serves this on the exchange endpoint or the item overview endpoint. If it is the item overview endpoint, the dispatch logic in `run_sync_prices` needs a third fetch path distinct from both `overviews` (exchange) and `item_overviews` (uniques). Check poe.ninja API before implementing Step 1.

2. **Exceptional gear thresholds**: The specific `sockets` min and `quality` min values are policy (author decision), not architecture. Research what PoE2 base item socket caps are to pick a meaningful `sockets.min` value.

3. **`mapping.json` version bump**: If the schema changes from `gemFilter` (singular) to `gemFilters` (plural), the committed `mapping.json` `"version": 1` should increment to 2 to make the breaking schema change detectable.

4. **Fixture re-baseline scope**: Steps 2, 4, and 6 each independently invalidate `seed-universal.filter`. Consider doing a single re-baseline pass at the end rather than three incremental ones, unless intermediate snapshot tests are needed during development.

---

*Architecture research for: FerrisFilter v0.3 Economy Items & Exceptional Gear*
*Researched: 2026-06-23*
