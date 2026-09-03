# Stack Research — FerrisFilter v0.3 Economy Items

**Domain:** Rust CLI / poe.ninja API integration (subsequent milestone)
**Researched:** 2026-06-23
**Confidence:** HIGH — all three endpoints fetched live against "Runes of Aldur" league

---

## Verdict: No New Crates Required

The existing stack covers everything v0.3 needs. `serde_json`, `reqwest` (or whatever
`HttpClient` trait wraps), and `miette` are sufficient. No new Cargo dependencies.

---

## Live Endpoint Verification

All three URLs were fetched against league `Runes of Aldur` on 2026-06-23.

### 1. UncutGems (existing — shape verified)

**URL:** `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=UncutGems`

**`type=` spelling:** `UncutGems` (plural) — confirmed correct; the code already uses this.

**Top-level shape:**
```
{
  "core": { "items": [...3 rate-ref currencies...], "rates": {...}, "primary": "divine", "secondary": "chaos" },
  "lines": [ { "id": "...", "primaryValue": ..., "volumePrimaryValue": ..., "sparkline": {...} } ],
  "items": [ { "id": "...", "name": "...", "category": "UncutGems", "detailsId": "..." } ]
}
```

**`lines[]` fields:** `id` (slug), `primaryValue` (f64), `volumePrimaryValue` (f64),
`maxVolumeCurrency` (string), `maxVolumeRate` (f64), `sparkline` object.
Lines do NOT carry a `name` field — names are resolved by joining `lines[].id` to `items[].id`.

**`items[]` fields:** `id` (slug), `name` (display name with level), `category` ("UncutGems"), `detailsId`.

**Level representation — definitive answer:**
Each gem level is a **separate entry**. There are 50 items[] entries across three gem types
and multiple levels. The `name` field carries the level in parentheses:

```
id: "uncut-skill-gem-12"   → name: "Uncut Skill Gem (Level 12)"    category: "UncutGems"
id: "uncut-skill-gem-17"   → name: "Uncut Skill Gem (Level 17)"    category: "UncutGems"
id: "uncut-skill-gem-20"   → name: "Uncut Skill Gem (Level 20)"    category: "UncutGems"
id: "uncut-spirit-gem-18"  → name: "Uncut Spirit Gem (Level 18)"   category: "UncutGems"
id: "uncut-support-gem-20" → name: "Uncut Support Gem (Level 20)"  category: "UncutGems"
```

The id slug encodes the level as a trailing integer (e.g. `uncut-skill-gem-20`), and the
`name` string encodes it as `(Level N)`. Both patterns are already handled by the existing
`parse_gem_level` function in `src/sync/prices.rs`.

**Current bug:** The existing `build_prices` code calls `strip_gem_level` which removes the
`(Level N)` suffix before storing in `PriceItem.base_type` (e.g. stores `"Uncut Skill Gem"`).
The emitted filter block therefore uses `BaseType == "Uncut Skill Gem"` — but the in-game
filter requires `BaseType == "Uncut Skill Gem (Level 20)"` because each level is a distinct
base type. The level suffix must be KEPT in the emitted base type, not stripped.

**Fix:** For each gem, keep the full `"Uncut Skill Gem (Level 20)"` name as the `base_type`
in `PriceItem`. This means ONE `PriceItem` per level (not collapsed to base name), each with
its own price. The emitter then de-dupes by tier across the full `(Level N)` names, producing
a block like:

```
Show # Uncut Gems B
  BaseType == "Uncut Skill Gem (Level 20)" "Uncut Spirit Gem (Level 20)" "Uncut Support Gem (Level 20)"
  ...
```

The `minLevel: 18` filter in `mapping.json` / `gem_filter.min_level` can then be removed
(or repurposed) since tier will gate relevance. The current gem de-dup logic (keep
highest-valued variant per base type) must change to keep ALL levels above the min threshold,
each as a separate price item.

---

### 2. LineageSupportGems (new)

**URL:** `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=LineageSupportGems`

**`type=` spelling:** `LineageSupportGems` (camelCase, plural) — confirmed correct and live.
The endpoint returned 71 priced lines with non-empty `rates` on 2026-06-23.

**Top-level shape:** Identical schema to other `exchange/current/overview` endpoints:
```
{
  "core": { "items": [...3 rate-refs...], "rates": { "exalted": 358.7, "chaos": 8.41 }, "primary": "divine", "secondary": "chaos" },
  "lines": [ { "id": "ahns-citadel", "primaryValue": 0.03366, "volumePrimaryValue": 3.07, ... } ],
  "items": [ { "id": "ahns-citadel", "name": "Ahn's Citadel", "category": "LineageSupportGems", "detailsId": "..." } ]
}
```

**`lines[]` fields:** Same as UncutGems — `id` slug only, no embedded `name`. Name resolution
requires joining against `items[]`.

**`items[]` fields:** `id` (kebab slug), `name` (display name), `category` ("LineageSupportGems"),
`detailsId`. 76 items total in the items[] dictionary.

**Sample id → name mappings:**
```
"ahns-citadel"    → "Ahn's Citadel"
"ailiths-chimes"  → "Ailith's Chimes"
"amanamus-tithe"  → "Amanamu's Tithe"
"rakiatas-flow"   → "Rakiata's Flow"
```

**In-game item class:** `Support Gems` (verified via poe2db.tw — Ahn's Citadel has
`Item Class: Support Gem` and `Base Type: Ahn's Citadel`). Each Lineage Support Gem's
individual name IS its filter BaseType.

**Filter mechanism:** These are NOT stackable currency. They are individual named gems with
`Class "Support Gems"` (or a substring match). The correct filter block for price-tiered
Lineage Support Gems is:
```
Show # Lineage Support Gems S
  Class "Support Gems"
  BaseType == "Ahn's Citadel" "Amanamu's Tithe" ...
  SetTextColor 255 121 198 255
  SetBorderColor 255 121 198 255
  SetBackgroundColor 10 150 150 240
  ...
```

**Implication for mapping.json:** LineageSupportGems needs a new entry. It cannot use
`class: "Stackable Currency"` — it needs `class: "Support Gems"`. `baseTypeIsName` is true
(name == base type, like all the currency-style exchange entries). However, it uses its own
per-category background (`10 150 150` per PROJECT.md) rather than the standard currency style.
The category field on the emitted PriceItems can be `"Lineage Support Gems"` for grouping.

---

### 3. Verisium (new)

**URL:** `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=Verisium`

**`type=` spelling:** `Verisium` (singular, capitalized) — confirmed correct and live.

**Top-level shape:** Identical to all other `exchange/current/overview` endpoints:
```
{
  "core": { "items": [...3 rate-refs...], "rates": { "exalted": 358.7, "chaos": 8.41 }, "primary": "divine", "secondary": "chaos" },
  "lines": [ { "id": "adaptive-alloy", "primaryValue": 0.07411, ... } ],
  "items": [ { "id": "adaptive-alloy", "name": "Adaptive Alloy", "category": "Verisium", "detailsId": "..." } ]
}
```

**`lines[]` fields:** Same schema — `id` slug, no embedded name.

**`items[]` count:** 23 items. All have `category: "Verisium"`.

**Complete items[] name list (all 23):**
```
Adaptive Alloy          Celestial Alloy          Cyclonic Alloy
Exceptional Verisium    Expansive Alloy          Medved's Crest of the Circle
Mystic Alloy            Olroth's Crest of the Sun   Prismatic Alloy
Protective Alloy        Revered Starlit Ore      Runic Alloy
Sovereign Alloy         Swift Alloy              The Runefather's Alloy
The Runebinder's Alloy  Transcendent Alloy       Uhtred's Crest of the Chalice
Venerable Starlit Ore   Veridical Starlit Ore    Verisium
Vorana's Crest of the Scythe   Warding Starlit Ore
```

**In-game item class:** `Stackable Currency` (confirmed via poe2db.tw — Verisium has
`Item Class: Stackable Currency`, base type `Verisium`, stack size 1/1000).

**Filter mechanism:** These are stackable currency; standard `BaseType == "Verisium"
"Adaptive Alloy" ...` rules with `Class "Stackable Currency"` work. The existing currency
pipeline (price → tier → emit `BaseType ==` block) applies directly. No special treatment
needed beyond adding the type to `mapping.json`.

---

## mapping.json Changes Required

### Add to `overviewTypes[]` (Verisium):
```json
{
  "type": "Verisium",
  "category": "Verisium",
  "class": "Stackable Currency",
  "baseTypeIsName": true
}
```

### New top-level key `gemTypes[]` (LineageSupportGems) — or extend `gemFilter`:

The cleanest approach is a new `gemTypes` array in `mapping.json` for gem categories that
use per-category background overrides and a non-currency class:

```json
"gemTypes": [
  {
    "type": "LineageSupportGems",
    "category": "Lineage Support Gems",
    "class": "Support Gems",
    "baseTypeIsName": true,
    "background": [10, 150, 150]
  }
]
```

Alternatively, promote `gemFilter` into an array. The Mapping struct in `src/sync/mapping.rs`
needs a corresponding field.

### Modify `gemFilter` (UncutGems fix):
```json
"gemFilter": {
  "type": "UncutGems",
  "category": "Uncut Gems",
  "class": "Stackable Currency",
  "minLevel": 1
}
```
Change `minLevel` from 18 to 1 (or remove the floor entirely — the tier cutoffs will
suppress worthless low-level gems from being highlighted).

---

## Data Model Changes Required

### `ExchangeOverview` / `ExchangeItem` (src/sync/models.rs)
No changes needed. The `items[]` top-level array is already modeled as `Vec<ExchangeItem>`
with `#[serde(default)]`. The existing join logic in `build_prices` works for all three
new types.

### `PriceItem` (src/models/prices.rs)
No structural changes required. The `category` field already distinguishes between types
at generation time.

### Gem de-dup logic (src/sync/prices.rs — `build_prices`)
The `gem_best` HashMap currently collapses all levels to one entry per base name
(e.g. `"Uncut Skill Gem"` → highest-valued level). This must change to:
- Keep `(Level N)` suffix in the stored `base_type` and `name`
- Store one PriceItem per distinct `"Uncut Skill Gem (Level N)"` string above minLevel
- Remove the `strip_gem_level` call path for UncutGems (or keep strip only for the
  de-dup key when minLevel filtering — then keep the full `(Level N)` name in the stored item)

The `parse_gem_level` function is still needed for the `minLevel` filter step.

---

## Stack Summary

| Technology | Version | Status | Purpose |
|------------|---------|--------|---------|
| Rust | 1.x (stable) | Unchanged | CLI language |
| serde / serde_json | current | Unchanged | JSON deserialization of all three endpoints |
| reqwest (via HttpClient trait) | current | Unchanged | HTTP fetch — same endpoint base URL |
| miette | current | Unchanged | Error reporting |
| clap | current | Unchanged | CLI argument parsing |
| insta | current | Unchanged | Snapshot tests for emitted filter blocks |
| tempfile | current | Unchanged | Atomic writes for prices.json |

**No new crates.** The exchange endpoint schema is identical across UncutGems,
LineageSupportGems, and Verisium. The existing `ExchangeOverview` model parses all three.

---

## Sources

- Live fetch: `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=UncutGems` (2026-06-23)
- Live fetch: `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=LineageSupportGems` (2026-06-23)
- Live fetch: `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes+of+Aldur&type=Verisium` (2026-06-23)
- poe2db.tw item page for Ahn's Citadel — confirmed `Item Class: Support Gem`, `Base Type: Ahn's Citadel`
- poe2db.tw item page for Verisium — confirmed `Item Class: Stackable Currency`, stack size 1/1000
- [Verisium wiki (Fextralife)](https://pathofexile2.wiki.fextralife.com/Verisium) — confirmed Stackable Currency class
- Existing `src/sync/prices.rs`, `src/sync/models.rs`, `mapping.json` — current code/data reviewed

---
*Stack research for: FerrisFilter v0.3 — Economy Items & Exceptional Gear*
*Researched: 2026-06-23*
