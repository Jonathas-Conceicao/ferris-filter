# Architecture Research

**Domain:** Rust CLI — config-to-PoE2-filter compiler with poe.ninja price sync
**Researched:** 2026-05-23
**Confidence:** HIGH (derived directly from the existing filter-model.json and universal.filter, which are the executable spec)

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI Entry Point (main.rs)                 │
│              clap: `generate` subcommand | `sync` subcommand     │
└────────────────────┬──────────────────────────┬─────────────────┘
                     │                          │
         ┌───────────▼──────────┐   ┌───────────▼───────────┐
         │   generate pipeline  │   │   sync pipeline        │
         │  (pure, no network)  │   │  (all network I/O)     │
         └───────────┬──────────┘   └───────────┬───────────┘
                     │                          │
    ┌────────────────┼──────────┐               │
    │                │          │               │
┌───▼────┐  ┌────────▼──┐  ┌───▼────┐    ┌─────▼──────┐
│ config │  │   tiers   │  │ prices │    │  poe_ninja  │
│ (serde)│  │  (serde)  │  │ (serde)│    │  (HTTP)     │
└───┬────┘  └────────┬──┘  └───┬────┘    └─────┬──────┘
    │                │          │               │
    └────────────────▼──────────┘               │
              ┌─────────────┐                   │
              │  pricing    │                   │
              │ (cross-rate │                   │
              │  + tiering) │                   │
              └─────┬───────┘                   │
                    │                           │
              ┌─────▼───────┐             ┌─────▼──────┐
              │    emit     │             │ prices.json │
              │ (block      │             │  (written)  │
              │  renderer)  │             └────────────┘
              └─────┬───────┘
                    │
              ┌─────▼──────────┐
              │ universal.filter│
              │   (written)     │
              └────────────────┘
```

---

## Module / Crate Layout

Single binary crate. All logic in `src/`. No workspace needed at this scale.

```
src/
├── main.rs              # CLI entry: clap App, routes to generate::run or sync::run
├── cli.rs               # clap Arg definitions, LeagueFlags struct
│
├── config/
│   └── mod.rs           # FilterConfig, Build, ArmourTypes, AreaLevelBand,
│                        #   AlwaysShow, Style, Minimap, Beam, Sound — serde models
│                        #   for config.json (renamed filter-model.json)
│
├── tiers/
│   └── mod.rs           # TierCutoffs struct: per-tier chaos thresholds (F/E/D/C/B/A/S),
│                        #   serde model for tiers.json; assign_tier(chaos_value, &cutoffs)
│
├── prices/
│   ├── mod.rs           # PriceList, ItemPrice { exalt, chaos, div } — serde model
│   │                    #   for prices.json; first 3 entries = Exalt/Chaos/Divine
│   └── poe_ninja.rs     # HTTP client: fetch_prices(league: &LeagueFlags) -> PriceList
│                        #   Hits poe.ninja PoE2 endpoints, normalizes response into PriceList
│
├── pricing/
│   └── mod.rs           # CrossRates derived from PriceList[0..3]:
│                        #   chaos_per_exalt, chaos_per_div
│                        # to_chaos(item: &ItemPrice, rates: &CrossRates) -> Option<f64>
│                        # assign_tier(value_chaos: f64, cutoffs: &TierCutoffs) -> Tier
│
├── emit/
│   ├── mod.rs           # emit_filter(plan: &FilterPlan, config: &FilterConfig) -> String
│   │                    #   Assembles ordered Vec<Block> and renders to filter text
│   ├── block.rs         # Block { action, comment, conditions, actions } and
│   │                    #   render_block(&Block) -> String
│   ├── style.rs         # style_to_actions(style: &Style) -> Vec<FilterAction>
│   │                    #   Handles null fields (omit), unchanged (no color actions),
│   │                    #   hidden (SetFontSize 21 + DisableDropSound only)
│   └── sections.rs      # One fn per logical section group (see emit order below):
│                        #   emit_recycle, emit_gear_bands, emit_gear_fallthrough,
│                        #   emit_flask_charm_bands, emit_currency_category,
│                        #   emit_price_tiered_items
│
├── generate.rs          # Orchestrates the pure pipeline:
│                        #   1. load config + tiers + prices
│                        #   2. derive CrossRates
│                        #   3. build PriceTieredItems (uniques + other price-tracked drops)
│                        #   4. call emit::emit_filter(plan, config)
│                        #   5. write universal.filter
│
└── sync.rs              # Orchestrates the network pipeline:
│                        #   1. parse LeagueFlags
│                        #   2. call prices::poe_ninja::fetch_prices
│                        #   3. write prices.json
```

---

## Core Data Types

### config.rs (mirrors filter-model.json exactly)

```rust
pub struct FilterConfig {
    pub meta: Meta,
    pub styles: IndexMap<String, Style>,   // ordered: S, A, B, C, D, E, F, Recycle, unchanged, hidden
    pub build: Build,
    pub weapon_classes: Vec<String>,
    pub armour_classes: Vec<String>,
    pub flask_charm_classes: Vec<String>,
    pub area_level_bands: Vec<AreaLevelBand>,
    pub always_show: AlwaysShow,
}

pub struct Style {
    pub text: Option<[u8; 3]>,
    pub border: Option<[u8; 3]>,
    pub background: Option<[u8; 3]>,
    pub font: Option<u8>,
    pub minimap: Option<Minimap>,
    pub beam: Option<Beam>,
    pub sound: Option<Sound>,
    pub action: BlockAction,           // Show | Hide
    pub disable_drop_sound: bool,
}

pub enum BlockAction { Show, Hide }
```

`IndexMap` (from the `indexmap` crate) preserves insertion order for styles — important because `S` before `A` before `B` etc. mirrors the model file and makes the rendered output deterministic.

### prices.rs

```rust
pub struct ItemPrice {
    pub exalt: Option<f64>,
    pub chaos: Option<f64>,
    pub div: Option<f64>,
}

// prices.json = Vec<(String, ItemPrice)> — ordered list
// prices[0] = ("Exalted Orb", ...), prices[1] = ("Chaos Orb", ...), prices[2] = ("Divine Orb", ...)
pub type PriceList = Vec<(String, ItemPrice)>;
```

The first-three-entries contract for cross-rate derivation is enforced in `pricing::CrossRates::from_price_list`, which panics (or returns `Err`) if the list has fewer than 3 entries or if `Chaos Orb` is missing a `chaos` value of `1.0`.

### pricing.rs

```rust
pub struct CrossRates {
    pub chaos_per_exalt: f64,
    pub chaos_per_div: f64,
}

impl CrossRates {
    pub fn from_price_list(list: &PriceList) -> Result<Self, PricingError> {
        // list[0] = Exalt, list[1] = Chaos (reference = 1.0c), list[2] = Divine
    }
    pub fn to_chaos(&self, price: &ItemPrice) -> Option<f64> {
        // Try chaos directly, else exalt * chaos_per_exalt, else div * chaos_per_div
        // Returns None if all fields are None
    }
}

pub fn assign_tier(value_chaos: f64, cutoffs: &TierCutoffs) -> Tier {
    // Iterates S down to F, returns first tier where value >= threshold
}
```

### emit/block.rs

```rust
pub struct Block {
    pub action: BlockAction,
    pub comment: String,            // "Currency S", "band <=5: all armour any rarity", etc.
    pub tag: String,                // "[alwaysShow.recycle -> styles.Recycle]"
    pub conditions: Vec<Condition>, // ordered
    pub filter_actions: Vec<FilterAction>, // ordered
}

pub enum Condition {
    AreaLevel(RangeOp, u16),
    Class(ExactMatch, Vec<String>),
    BaseType(ExactMatch, Vec<String>),
    Rarity(Op, Rarity),
    Quality(Op, u8),
    Sockets(Op, u8),
    BaseEvasion(Op, u32),
    BaseArmour(Op, u32),
    BaseEnergyShield(Op, u32),
}

pub enum FilterAction {
    SetTextColor([u8; 4]),
    SetBorderColor([u8; 4]),
    SetBackgroundColor([u8; 4]),
    SetFontSize(u8),
    PlayAlertSound(u8, u16),
    CustomAlertSound(String, u16),
    PlayEffect(String, bool),      // color, temp
    MinimapIcon(u8, String, String), // size, color, shape
    DisableDropSound,
}
```

`render_block` serializes to filter text. The ordering of filter_actions within a block follows the exact sequence in `universal.filter`: `PlayEffect`, `MinimapIcon`, `SetTextColor`, `SetBackgroundColor`, `SetBorderColor`, `SetFontSize`, `CustomAlertSound`/`PlayAlertSound`, `DisableDropSound`. Encoding this order in `style_to_actions` makes output byte-for-byte stable.

---

## Emit Order (the canonical block sequence)

`emit_filter` builds a `Vec<Block>` in this exact order, then renders linearly. This is the rule that makes first-match-wins work correctly:

```
Section 1: Recycle guard
  alwaysShow.recycle → styles.Recycle
  Show, Sockets >= 1, Quality >= 1

Section 2: Gear areaLevelBands (styles.unchanged = Show, no color actions)
  2a. band <=5: all armour any rarity        (armourClasses, AreaLevel <= 5)
  2b. band <=25: build weapons any rarity    (build.weapons, AreaLevel <= 25)
  2c. band 26+: build weapons Rare only      (build.weapons, Rarity == Rare)
  2d. band <=25: build armour anyNonEvasion  (build.armour, AreaLevel <= 25, BaseEvasion 0, Rarity >= Magic)
  2e. band 26-45: build armour esHybrid      (build.armour, AreaLevel 26-45, BaseEnergyShield > 0, BaseEvasion 0, Rarity >= Magic)
  2f. band 46+: build armour pureES          (build.armour, AreaLevel >= 46, BaseEnergyShield > 0, BaseArmour 0, BaseEvasion 0, Rarity >= Magic)
  2g. bands <=45: any other Rare gear        (all weaponClasses + armourClasses, AreaLevel <= 45, Rarity == Rare)

Section 3: Gear fallthrough (styles.hidden = Hide)
  Hide, all weaponClasses + armourClasses, Rarity <= Rare
  SetFontSize 21, DisableDropSound

Section 4: Flask/charm areaLevelBands (styles.unchanged)
  4a. quality always shown   (flaskCharmClasses, Quality >= 1)
  4b. bands <=25: all        (flaskCharmClasses, AreaLevel <= 25)
  4c. bands 26-65: magic     (flaskCharmClasses, AreaLevel <= 65, Rarity == Magic)

Section 5: Flask/charm fallthrough (styles.hidden)
  Hide, flaskCharmClasses, SetFontSize 21, DisableDropSound

Section 6: Currency / curated always-show blocks
  For each named category group (Currency, Abyssal Eyes, Ancient Bones, Breach,
  Delirium, Essences, Expedition, Idols, Incubators, Map Fragments/Pinnacle Keys,
  Omens, Runes, Soul Cores, Tablets, Vault Keys, ...):
    For each tier present in that category (S through F, descending):
      Show, Class == "<class>", BaseType == "<b1>" "<b2>" ..., <style actions>

Section 7: Price-tiered items (future — uniques + other poe.ninja drops)
  Emitted after curated currency, before any final fallthrough.
  For each tier S through F (descending):
    For each item in that tier:
      Show, BaseType == "<name>", Rarity == Unique (for uniques), <style actions>
  Items with no price or price below F threshold: no block → falls through to game default.
```

Sections 1–6 are fully deterministic from `config.json` + the curated `CurrencyData` struct embedded in `emit/sections.rs`. Section 7 is deterministic given a fixed `prices.json` + `tiers.json`.

---

## How Styles Map to Block Actions

`emit/style.rs::style_to_actions(style: &Style) -> Vec<FilterAction>`:

| Style field | When `null` | When present |
|---|---|---|
| `text` / `border` / `background` | Omit the action entirely | Emit `SetTextColor` / `SetBorderColor` / `SetBackgroundColor` with alpha 255 (text/border) or 255 (bg) |
| `font` | Omit `SetFontSize` | Emit `SetFontSize N` |
| `minimap` | Omit | Emit `MinimapIcon size color shape` |
| `beam` | Omit | Emit `PlayEffect color [Temp]` |
| `sound.custom` | Check `sound.id` | Emit `CustomAlertSound "file.mp3" vol` |
| `sound.id` | Omit sound | Emit `PlayAlertSound id vol` |
| `disable_drop_sound: true` | N/A | Emit `DisableDropSound` |

The `unchanged` style has all nullable fields set to `null` → emits a `Show` block with zero styling actions. The `hidden` style has `action: Hide`, `font: 21`, `disable_drop_sound: true`, all colors null → emits a `Hide` with `SetFontSize 21` + `DisableDropSound`.

---

## Curated Currency Data

Currency category lists (the `BaseType ==` lists in Section 6) are **curated data, not derived from the model or prices**. They belong to a `CurrencyData` struct embedded directly in `emit/sections.rs`. This matches the design decision in `filter-model.json`: `"alwaysShow": { "currency": { "style": "tiered S-F (base lists are curated data, kept outside this model)" } }`.

Structure:

```rust
// emit/sections.rs
pub struct CurrencyCategory {
    pub name: &'static str,     // "Currency", "Abyssal Eyes", etc.
    pub class: &'static str,    // "Stackable Currency", "Augment", etc.
    pub tiers: &'static [(Tier, &'static [&'static str])], // tier -> base names
}

// Populated from the exact base lists in universal.filter.
// When sync + price-tiering lands, these lists transition to being
// generated from prices.json rather than being static — but the
// emit interface (CurrencyCategory) stays identical.
static CURRENCY_DATA: &[CurrencyCategory] = &[ ... ];
```

Once `sync` + price-tiering is in place, `CURRENCY_DATA` either becomes the fallback for items poe.ninja doesn't price, or is replaced by the output of `pricing::assign_tier` applied to all `Stackable Currency`-class items.

---

## The Unique-by-Base Matching Problem

**Where it lives:** `generate.rs` + `pricing/mod.rs`, not in `emit/`.

**The problem:** poe.ninja tracks unique items by their display name (e.g., `"Shavronne's Wrappings"`). The PoE2 filter matches by `BaseType ==` (the base item name, e.g. `"Occultist's Vestment"`) or by the unique's display name — PoE2 filter grammar supports matching `BaseType == "Shavronne's Wrappings"` directly (uniques have their own name as base type in filter context). This means for price-tiered uniques:

- The block is: `Show`, `BaseType == "<unique display name>"`, `Rarity == Unique`, `<style actions>`
- **No ambiguity when matching by display name.** PoE2 filter `BaseType ==` matches the unique's actual name, not the base.

**Architectural implication:** `generate.rs` builds a `Vec<TieredItem>` where each entry is `(display_name: String, class: Option<String>, rarity_filter: Option<Rarity>, tier: Tier)`. For uniques, `rarity_filter = Some(Rarity::Unique)`. For currency items (once price-driven), `rarity_filter = None` (class match is sufficient). The emitter receives this flattened list; it does not need to resolve base types.

**Where the lookup is performed:** `generate.rs` calls `pricing::assign_tier` for every entry in `prices.json`, groups results by tier, and passes the grouped `Vec<TieredItem>` to `emit::emit_price_tiered_items`. All logic is in `generate.rs`; `emit/` is a pure renderer.

---

## Data Flow: `generate` Command

```
CLI args
    ↓
generate::run()
    ├── config::load("config.json")         → FilterConfig
    ├── tiers::load("tiers.json")           → TierCutoffs
    ├── prices::load("prices.json")         → PriceList
    │
    ├── pricing::CrossRates::from_price_list(&prices)  → CrossRates
    │
    ├── for each item in prices[3..]:
    │     to_chaos(item, &rates) → Option<f64>
    │     assign_tier(value, &cutoffs) → Tier
    │     → PriceTieredItems: BTreeMap<Tier, Vec<TieredItem>>
    │         (BTreeMap ensures deterministic S→F ordering)
    │
    └── emit::emit_filter(&config, &price_tiered_items)
            ├── emit_recycle(&config)
            ├── emit_gear_bands(&config)
            ├── emit_gear_fallthrough(&config)
            ├── emit_flask_charm_bands(&config)
            ├── emit_flask_charm_fallthrough(&config)
            ├── emit_currency_categories()        ← static CURRENCY_DATA
            └── emit_price_tiered_items(&price_tiered_items)
            → Vec<Block>
            → render_all_blocks(blocks) → String
            → write "universal.filter"
```

Everything from `CrossRates::from_price_list` onward is pure computation. No `mut` global state, no I/O after the initial reads.

---

## Data Flow: `sync` Command

```
CLI args (--hc, --standard flags)
    ↓
sync::run()
    ├── cli::parse_league_flags(&matches) → LeagueFlags
    └── prices::poe_ninja::fetch_prices(&league)
            ├── reqwest::Client::new()
            ├── GET poe.ninja/api/data/itemoverview?league=...&type=Currency
            ├── GET poe.ninja/api/data/itemoverview?league=...&type=UniqueWeapon
            ├── GET poe.ninja/api/data/itemoverview?league=...&type=UniqueArmour
            ├── ... (other categories)
            ├── normalize: build Vec<(name, ItemPrice)>
            │     prepend [("Exalted Orb", ...), ("Chaos Orb", 1.0c), ("Divine Orb", ...)]
            │     then all other items
            └── serialize to prices.json
```

`sync` is the **only** module that does network I/O. `generate` never touches the network. This isolation means `generate` is easily testable with fixture `prices.json` files and always produces deterministic output given the same inputs.

---

## Module Dependency Graph (Build Order)

Dependencies flow strictly downward. Build phases should follow this order:

```
Level 0 (no deps):
  config/     — serde models only
  tiers/      — serde models + assign_tier()
  prices/     — serde model (PriceList)

Level 1 (depends on Level 0):
  pricing/    — depends on prices, tiers

Level 2 (depends on Level 0–1):
  emit/       — depends on config (Style, FilterConfig)
               — depends on pricing (Tier enum, TieredItem)

Level 3 (depends on Level 0–2):
  generate    — depends on config, tiers, prices, pricing, emit
  sync        — depends on prices (only the model + poe_ninja client)

Level 4:
  main / cli  — depends on generate + sync
```

Recommended build phase structure:

1. **Phase 1 — emit foundation:** `config` models + `emit/block.rs` + `emit/style.rs`. Can render hardcoded blocks. Validates the filter grammar.
2. **Phase 2 — static generate:** `emit/sections.rs` with hardcoded `CURRENCY_DATA` and all gear/flask sections. `generate.rs` wired to read `config.json` and emit a deterministic `universal.filter` that matches the existing hand-built file. No prices yet.
3. **Phase 3 — tiers + pricing:** `tiers` + `pricing` modules. `generate.rs` extended to load `prices.json`, derive cross-rates, assign tiers, and emit `Section 7` blocks.
4. **Phase 4 — sync:** `prices/poe_ninja.rs` + `sync.rs`. End-to-end: `sync` → `prices.json` → `generate` → `universal.filter`.

---

## Architectural Patterns

### Pattern 1: Pure Generate, Impure Sync

**What:** `generate` reads files and writes one file. No network. No randomness. All intermediate values (CrossRates, PriceTieredItems) are plain data structs derived deterministically from inputs.

**When to use:** Always for the generate path. This is the core invariant.

**Trade-offs:** Requires the two-step workflow (`sync` first, then `generate`), but makes `generate` trivially testable and diff-friendly (same config + prices = same filter output).

### Pattern 2: Block List as Intermediate Representation

**What:** `emit::emit_filter` builds a `Vec<Block>` before rendering to text. The Block IR separates "what blocks exist and in what order" from "how to serialize a block to filter syntax."

**When to use:** Preferred. Makes section ordering explicit and testable. `emit_recycle` returns `Vec<Block>`, not a `String` — composable.

**Trade-offs:** Slight extra allocation, but filter output is small (< 5000 lines), so this is irrelevant.

### Pattern 3: Static Curated Data as Rust Literals

**What:** `CURRENCY_DATA` in `emit/sections.rs` is `&'static [CurrencyCategory]` — Rust `&'static str` arrays compiled into the binary. No runtime loading, no external file dependency.

**When to use:** For data that is hand-curated and changes only with league transitions (i.e., when the developer decides to update and recompile). When price-tiering replaces curation, this transitions to a computed structure — but the rest of `emit/` does not change.

**Trade-offs:** Requires recompile to update lists. Acceptable: the filter regeneration workflow already requires running the CLI.

---

## Anti-Patterns

### Anti-Pattern 1: Mixing Network I/O into `generate`

**What people do:** Call poe.ninja inside `generate` when `prices.json` is missing.

**Why it's wrong:** Breaks determinism, makes testing require mocks, makes `generate` fail on network issues even when cached prices are valid.

**Do this instead:** `generate` errors with a clear message if `prices.json` is absent. User runs `sync` first. The separation is intentional.

### Anti-Pattern 2: Building the Filter as String Concatenation

**What people do:** Each section function returns a `String` and the emitter `+`-concatenates them.

**Why it's wrong:** No ability to reorder blocks, inspect the output, or test individual blocks without parsing text.

**Do this instead:** Return `Vec<Block>` from each section function. Convert to string once at the end in `render_all_blocks`.

### Anti-Pattern 3: Blanket Unique Styling

**What people do:** Add a catch-all `Rarity == Unique` block to style all uniques with a tier color.

**Why it's wrong:** Violates the established design philosophy (CLAUDE.md: "Never apply a blanket restyle to `Rarity == Unique`"). Untracked uniques must fall through to the game default.

**Do this instead:** Only emit `BaseType == "<name>" Rarity == Unique` blocks for specific uniques present in `prices.json`. All other uniques fall through.

### Anti-Pattern 4: Sorting prices.json by Name

**What people do:** Sort the PriceList alphabetically or by value when writing `prices.json`.

**Why it's wrong:** The first three entries encode the cross-rate reference currencies. Sorting would destroy that contract.

**Do this instead:** `sync` always writes Exalt, Chaos, Divine as entries 0/1/2, then the rest in any consistent order (e.g., by descending chaos value for human readability).

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---|---|---|
| poe.ninja PoE2 API | `reqwest` HTTP GET in `prices::poe_ninja` | Rate limits unknown; research needed. Returns JSON arrays of `{name, chaosValue, divineValue, ...}`. Separate endpoints per item category. |

### Internal Boundaries

| Boundary | Communication | Notes |
|---|---|---|
| `generate` → `emit` | Passes `&FilterConfig` + `&BTreeMap<Tier, Vec<TieredItem>>` | emit never reads files directly |
| `generate` → `pricing` | Passes `&PriceList` + `&TierCutoffs`, receives `CrossRates` + tier assignments | Pure functions only |
| `sync` → `prices::poe_ninja` | Calls `fetch_prices(league)`, receives `PriceList` | All network in this fn |
| `config` / `tiers` / `prices` | Loaded by `generate` and `sync` respectively via `serde_json::from_reader` | Each is a standalone load function |

---

## Scaling Considerations

This is a single-user local CLI tool. Scaling is not relevant. The only performance concern worth noting:

| Concern | Approach |
|---|---|
| poe.ninja rate limits | Cache `prices.json` locally; only `sync` hits the API. Don't call `sync` in a loop. |
| Filter output size | The current hand-built filter is ~850 lines. Even with price-tiered uniques, output will be well under 10,000 lines. String allocation is trivially fast. |
| Determinism regression | The only risk is non-deterministic ordering in `BTreeMap` or `HashMap`. Use `BTreeMap<Tier, Vec<TieredItem>>` (ordered by Tier enum discriminant) and `IndexMap` for styles. Sort `Vec<TieredItem>` by name before emitting. |

---

## Sources

- `filter-model.json` — source of truth for all model types (HIGH confidence)
- `universal.filter` — executable spec for block emission order and rendering (HIGH confidence)
- `CLAUDE.md` — filter grammar, style rules, design philosophy (HIGH confidence)
- `.planning/PROJECT.md` — requirements and key decisions (HIGH confidence)

---
*Architecture research for: FerrisFilter (Rust CLI, config-to-PoE2-filter compiler)*
*Researched: 2026-05-23*
