# Phase 2: Static Generate - Research

**Researched:** 2026-05-23
**Domain:** Rust filter emitter, insta snapshot testing, serde struct refactor, git diff artifacts
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Re-baseline approach — define a single canonical action-emission order, regenerate
  `universal.filter`, commit as new golden master. Block order, conditions, BaseType lists, and all
  style values preserved; only intra-block action order normalized.
- **D-02:** Diff guardrail = behavioral equivalence (minor cleanup allowed). No block
  added/removed, no condition/BaseType/color/numeric value altered.
- **D-03:** Canonical action order is Claude's discretion — pick one consistent order, apply
  to every block. Recycle block's divergent order is normalized.
- **D-04:** Original hand-built `universal.filter` preserved via git history only.
- **D-05:** Regression lock = insta snapshot of the generated output string (committed `.snap`,
  updated deliberately via `cargo insta review`).
- **D-06:** Artifact-guard test: assert repo-root committed `universal.filter` byte-equals
  generator output. `.snap` and `universal.filter` are separate; running `generate` never clobbers
  the snapshot oracle.
- **D-07:** Determinism test: generating twice with identical inputs yields identical output.
- **D-08:** Exact string equality is acceptance — no separate SHA-256 assertion.
- **D-09:** `ferrisfilter generate` (no `--output`) writes to `universal.filter` at repo root.
  `--output <path>` overrides.
- **D-10:** Refactor `areaLevelBands` rarity rules from freeform strings into a typed, closed
  enum (Any | RareOnly | MagicPlusRare | MagicOnly | QualityOnly).
- **D-11:** Model `styles` as a typed struct of named fields (S, A, B, C, D, E, F, Recycle,
  unchanged, hidden) replacing `HashMap<String, Style>`.
- **D-12:** Phase 1 `config.rs` models and round-trip/inline tests evolve to match refactored
  schema; `config.json` is rewritten.
- **D-13:** Old → new equivalence validated by MANUAL in-game smoke test (human-UAT).
- **D-14:** At end of Phase 2 execution, produce human-reviewable diffs of config.json
  (old → refactored) and universal.filter (old hand-built → regenerated).

### Claude's Discretion

- The exact canonical action order; enum variant names and JSON spelling of refactored rules;
  emitter code structure/modules; how block and section order is derived (must reproduce the
  established sequence: recycle → gear bands → flask/charm fallbacks → currency/category tier
  blocks); reproduction vs cleanup of the header comment block.

### Deferred Ideas (OUT OF SCOPE)

- Price-driven tiering of currency and uniques (consuming `prices.json`) → Phase 3.
- `sync` subcommand / poe.ninja networking → Phase 4.
- Automated semantic-equivalence check (old vs new filter) → not building this phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GEN-01 | Deterministic compilation — identical inputs always produce byte-identical output | Emitter architecture (no HashMap iteration), insta snapshot, determinism test |
| GEN-02 | Reproduces established structure: style tiers, build-weapon/armour area-level bands, flask/charm ladder, recycle, currency tiers, gear-hide fallthrough | Complete emission inventory (85 blocks catalogued below) |
| GEN-03 | Each generated block carries a comment linking it to its config source (model path tag) | Block-tag format already established in universal.filter; emitter must reproduce per-block |
| GEN-04 | Uniques, gems, jewels, jewellery, quest items, waystones, trials, unidentified receive no rules — fall through | Confirmed: none of these appear in current universal.filter; GEN-04 = emit nothing for them |
</phase_requirements>

---

## Summary

Phase 2 implements `ferrisfilter generate` — a deterministic Rust emitter that reads
`config.json` and `currency.json` and writes `universal.filter`. The existing hand-built filter
is 848 lines, 85 blocks total: 2 Hide blocks and 83 Show blocks. All 85 blocks are cleanly
derivable from `config.json` + `currency.json` with two caveats (see Gaps below). The generator
must be re-baselined: it produces a behaviorally-equivalent but action-order-normalized output
that is committed as the new golden master.

The primary technical challenges are: (1) the `styles` HashMap nondeterminism must be eliminated
by a typed named-fields struct (D-11), (2) the canonical action-emission order must be defined
and applied uniformly, (3) the insta snapshot + artifact-guard tests must handle Windows CRLF
correctly, and (4) the `areaLevelBands` rarity strings must become a closed enum (D-10). None of
these are blocking problems — clear patterns exist for each.

**Primary recommendation:** Build a `src/emitter.rs` module that takes `(&Config, &CurrencyData)`
and returns `String`. Emit blocks by writing to a `String` via `write!` (use `std::fmt::Write`).
Keep all action emission in one `fn emit_style_actions(out: &mut String, style: &Style)` so the
canonical order is in exactly one place. Drive currency blocks by iterating
`currency.categories` (Vec → deterministic) and `category.entries` (Vec → deterministic).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Config parsing + validation | Library (loader.rs) | models/config.rs | Already established in Phase 1 |
| Currency data parsing | Library (loader.rs) | models/currency.rs | Already established in Phase 1 |
| Filter emission (text generation) | New emitter module | main.rs (wiring) | Pure function: (Config, CurrencyData) → String |
| Writing output file | main.rs cmd_generate | emitter (returns String) | Emitter stays pure; IO in cmd_generate |
| Snapshot regression lock | tests/ (insta) | — | insta assert_snapshot! on returned String |
| Artifact-guard test | tests/ (assert_cmd or inline) | — | Read universal.filter, compare to generated |
| Config schema refactor | models/config.rs | config.json | D-10/D-11: typed styles + rarity enum |
| Diff artifact production | Git (git diff HEAD) | .planning/phases/02-static-generate/ | At end of execution, not a runtime feature |

---

## Complete Emission Inventory

The existing `universal.filter` (848 lines, 85 blocks) is the target. Below is every block in
emission order with its tag format, conditions, and style applied. This inventory is the
authoritative scope for GEN-02 completeness.

[VERIFIED: universal.filter read in full]

### Section 1: File Header Comment (lines 1–13)

A single multi-line comment block. Not a filter block (no `Show`/`Hide`). The emitter must emit
this verbatim (or an equivalent) at the top of the output.

```
# universal.filter
# GENERATED FROM <source-indicator> — edit the model, not this file (then regenerate).
#
# Block tags link each rule back to the model:
#   [alwaysShow.K]          -> a level-independent highlight
#   [areaLevelBands "R".K]  -> band R, show-key K
#   [styles.X]              -> the style entry applied
#
# Philosophy: HIDE what we don't want, STYLE the few highlights, let everything
# else fall through to the game default ...
# Currency base lists are curated data kept outside the model (carried over here).
```

NOTE: The source indicator was "filter-model.json" in the old hand file. After D-12 config.json
rename, it should reference config.json. Minor cleanup allowed per D-02.

### Section 2: Recycle Block (line 18)

**1 block.** Source: `config.alwaysShow.recycle`.

| Block | Header | Tag | Conditions | Style |
|-------|--------|-----|------------|-------|
| recycle | `Show` | `[alwaysShow.recycle -> styles.Recycle]` | `Sockets >= 1` / `Quality >= 1` | `styles.Recycle` |

The recycle block is the ONLY block where the existing action order differs from the currency
blocks (see Canonical Action Order section for details).

### Section 3: Gear AreaLevel Bands (lines 36–86)

**8 blocks** — 7 Show + 1 Hide. All use `styles.unchanged` (no recolor) or `styles.hidden`.

The section heading comment:
```
# areaLevelBands — gear
# Shown gear = styles.unchanged ...
# ...
```

Blocks in order:

| # | Block | Tag | Key Conditions | Style |
|---|-------|-----|----------------|-------|
| 1 | band <=5: all armour any rarity | `[areaLevelBands "1-5".allArmour -> styles.unchanged]` | `AreaLevel <= 5` / `Class "Body Armours" "Helmets" "Gloves" "Boots" "Shields" "Bucklers"` | unchanged (Show) |
| 2 | band 6-25: build weapons any rarity | `[build.weapons -> styles.unchanged]` | `AreaLevel <= 25` / `Class "Staves" "Wands" "Foci"` | unchanged |
| 3 | bands 26+: build weapons Rare only | `[build.weapons -> styles.unchanged]` | `Class "Staves" "Wands" "Foci"` / `Rarity == Rare` | unchanged |
| 4 | band 6-25: build armour (non-evasion) | `[build.armour/anyNonEvasion -> styles.unchanged]` | `AreaLevel <= 25` / `Class <armourClasses>` / `BaseEvasion 0` / `Rarity >= Magic` | unchanged |
| 5 | band 26-45: build armour (ES/hybrid) | `[build.armour/esHybrid -> styles.unchanged]` | `AreaLevel >= 26` / `AreaLevel <= 45` / `Class <armourClasses>` / `BaseEnergyShield > 0` / `BaseEvasion 0` / `Rarity >= Magic` | unchanged |
| 6 | bands 46+: build armour (pure ES) | `[build.armour/pureES -> styles.unchanged]` | `AreaLevel >= 46` / `Class <armourClasses>` / `BaseEnergyShield > 0` / `BaseArmour 0` / `BaseEvasion 0` / `Rarity >= Magic` | unchanged |
| 7 | bands <=45: any other Rare gear | `[areaLevelBands.otherGear -> styles.unchanged]` | `AreaLevel <= 45` / `Class <weaponClasses + armourClasses>` / `Rarity == Rare` | unchanged |
| 8 | gear fallback | `[styles.hidden]` | `Class <weaponClasses + armourClasses>` / `Rarity <= Rare` | **Hide** + `SetFontSize 21` / `DisableDropSound` |

**Class lists from config.json:**
- `weaponClasses`: `"Bows" "Crossbows" "Foci" "One Hand Maces" "Quivers" "Quarterstaves" "Sceptres" "Spears" "Talismans" "Staves" "Two Hand Maces" "Wands"` (12 classes)
- `armourClasses`: `"Body Armours" "Helmets" "Gloves" "Boots" "Shields" "Bucklers"` (6 classes)
- `otherGear Class` = weaponClasses + armourClasses combined = 18 classes

The `armour` class list in blocks 4/5/6 is `build.armour` = `["Body Armours", "Helmets", "Gloves", "Boots"]` (4 classes, no Shields/Bucklers — verified in filter lines 52–72).

**NOTE:** Band "1-5" only has `allArmour` and does NOT emit a weapons block or otherGear block.
Weapons fall through in band 1-5 (game default). The band "6-25" weapons block uses
`AreaLevel <= 25` (covering both 6-25 and the "show all" zone). Similarly the weapons "Rare only"
block has no AreaLevel lower bound (applies 26+). The emitter must compute these cross-band
merged conditions — they cannot be naively emitted per-band.

**`unchanged` and `hidden` styles emit NO color/font/beam/sound actions** — only the block
header and conditions (plus font + DisableDropSound for `hidden`). This is correct per the design
philosophy: native rarity color shows through.

### Section 4: Flask/Charm AreaLevel Bands (lines 93–112)

**4 blocks** — 3 Show + 1 Hide. Section heading comment:
```
# areaLevelBands — flasks & charms  [flaskCharmClasses]
```

Blocks in order:

| # | Block | Tag | Key Conditions | Style |
|---|-------|-----|----------------|-------|
| 1 | quality always | `[areaLevelBands.flasksCharms -> styles.unchanged]` | `Quality >= 1` / `Class "Life Flasks" "Mana Flasks" "Charms"` | unchanged |
| 2 | bands <=25: all flasks/charms | `[styles.unchanged]` | `AreaLevel <= 25` / `Class "Life Flasks" "Mana Flasks" "Charms"` | unchanged |
| 3 | bands 26-65: magic flasks/charms | `[styles.unchanged]` | `AreaLevel <= 65` / `Class "Life Flasks" "Mana Flasks" "Charms"` / `Rarity == Magic` | unchanged |
| 4 | flasks/charms fallback | `[styles.hidden]` | `Class "Life Flasks" "Mana Flasks" "Charms"` | **Hide** + `SetFontSize 21` / `DisableDropSound` |

The "quality always" block has no AreaLevel bound — it supersedes the band rules for quality
items at any level. The "bands 26-65" block uses `AreaLevel <= 65` rather than a range, because
the 66+ band's "Quality only" rule is captured entirely by the quality-always block above.

**`flaskCharmClasses`** from config.json: `["Life Flasks", "Mana Flasks", "Charms"]`.

### Section 5: Currency/Category Tier Blocks (lines 122–848)

**72 blocks** — all Show. Source: `currency.categories` (15 categories, each with
`entries[]{tier, class, bases}`).

Section heading comment:
```
# alwaysShow.currency — tiered S-F  [styles.S .. styles.F]
# ...
# Gems are intentionally omitted (fall through to game default).
```

Each category gets a `# --- Category Name ---` comment before its entries.

**Complete category/block inventory** (15 categories, 72 total blocks):

| # | Category | Entries (tier: class) | Block count |
|---|----------|-----------------------|-------------|
| 1 | Currency | S/A/B/C/D/E/F : Stackable Currency | 7 |
| 2 | Abyssal Eyes | S/A/C : Augment | 3 |
| 3 | Ancient Bones | S/B/C/D/E : Stackable Currency | 5 |
| 4 | Breach | B/C/D : Stackable Currency | 3 |
| 5 | Delirium | S/A/B/C/D/E/F : Stackable Currency | 7 |
| 6 | Essences | S/A/B/C/D/E : Stackable Currency | 6 |
| 7 | Expedition | B(Expedition Logbooks)/C/D/E(Stackable Currency) | 4 |
| 8 | Idols | S/A/B/C/D : Augment | 5 |
| 9 | Incubators | D : Incubators | 1 |
| 10 | Map Fragments & Pinnacle Keys | S(Pinnacle Keys)/A(Map Frags)/A(Pinnacle Keys)/C(Map Frags)/E(Trial Coins)/E(Inscribed Ultimatum) | 6 |
| 11 | Omens | S/A/B/C/D/E(Omen)/E(Stackable Currency - Petition Splinter)/F : Omen + Stackable Currency | 8 |
| 12 | Runes | S/A/B/C/D/E/F : Augment | 7 |
| 13 | Soul Cores | S/A/B/C/D : Augment | 5 |
| 14 | Tablets | B/C : Tablet | 2 |
| 15 | Vault Keys | S/A/C : Vault Keys | 3 |

**Total: 72 currency blocks.** Grand total with gear+flask sections: **85 blocks**.

**Per-block structure for currency blocks:**
```
Show # {Category Name} {Tier Letter}
  Class == "{class}"
  BaseType == {space-separated quoted base names}
  {style actions per tier}
```

Note the `==` (exact-match) for Class and BaseType in currency blocks — this is different from
the gear blocks which use `Class "..."` without `==`.

**Style-to-action mapping for currency blocks (observations from filter):**

- **S**: PlayEffect + MinimapIcon + SetTextColor + SetBackgroundColor + SetBorderColor + SetFontSize 37 + CustomAlertSound
- **A**: PlayEffect + MinimapIcon + SetTextColor + SetBackgroundColor + SetBorderColor + SetFontSize 37 + CustomAlertSound
- **B**: PlayEffect + MinimapIcon + SetTextColor + SetBackgroundColor + SetBorderColor + SetFontSize 37 + PlayAlertSound
- **C**: PlayEffect + MinimapIcon + SetTextColor + SetBackgroundColor + SetBorderColor + (no font) + PlayAlertSound
- **D**: PlayEffect + MinimapIcon + SetTextColor + SetBackgroundColor + SetBorderColor + (no font, no sound)
- **E**: SetTextColor + SetBackgroundColor + SetBorderColor + SetFontSize 29 (no minimap, no beam, no sound)
- **F**: SetTextColor + SetBackgroundColor + SetBorderColor + SetFontSize 25 (no minimap, no beam, no sound)

These are driven by `config.styles.*` field presence (Option<T> == None → omit action).

---

## Gaps: Items in universal.filter NOT Derivable from config.json + currency.json

[VERIFIED: cross-checked filter against both input files]

**Gap 1: The alwaysShow section comment line 42 (recycle comment "# quality always shown...")**
The inline comment on line 92: `# quality always shown (this is the ">65 Quality only" tier)` is
free text editorial context. The emitter can reproduce this or omit it (minor cleanup per D-02).

**Gap 2: The CLAUDE.md `jewels-all` block does NOT appear in universal.filter.**
CLAUDE.md (§Tier & color system) describes: "Jewels (Class "Jewels") are always shown — every
rarity, every level — at native rarity color, font 32, with a cyan diamond minimap icon
(jewels-all)." However, searching universal.filter confirms there is no `jewels-all` block and
no `Class "Jewels"` rule anywhere in the current filter. Jewels fall through to game default
(consistent with GEN-04). The CLAUDE.md prose is outdated/aspirational. GEN-04 explicitly says
jewels get NO rules. The emitter must NOT add a jewels block. CLAUDE.md says "when it conflicts
with the model, the model wins." [VERIFIED]

**Gap 3: Section heading comments derive from known section names (not stored in the model).**
The `# ---  {Category Name}  ---` comments between currency category groups are derived from
`category.name`. The `#====` section break headers (alwaysShow.recycle, areaLevelBands — gear,
etc.) are emitter-hardcoded structural comments. They need to be faithfully reproduced.

**Gap 4: Per-band inline comments above gear blocks.**
Each gear block has a preceding `# [areaLevelBands ...]` inline comment line (in addition to the
block header comment). These are part of the output. The emitter must emit these per-block
prefatory comments.

**SUMMARY: No functional gaps.** Every block in universal.filter is fully derivable from
`config.json` + `currency.json`. No data is embedded in the filter that lives nowhere in the
inputs.

---

## Canonical Action Order

[VERIFIED: analyzed all 85 blocks in universal.filter]

### Observed Inconsistency in Current Filter

The current hand-built filter has two distinct action orderings:

**Currency blocks (S/A/B/C/D — most of the file):**
```
PlayEffect {Color}
MinimapIcon {Size} {Color} {Shape}
SetTextColor R G B 255
SetBackgroundColor R G B 255
SetBorderColor R G B 255
SetFontSize N           # if present
CustomAlertSound / PlayAlertSound  # if present
```

**Recycle block (lines 21–25) — divergent order:**
```
SetTextColor R G B 255
SetBorderColor R G B 255
SetBackgroundColor R G B 255
MinimapIcon 2 Green Cross
PlayEffect Green
# no sound, no font
```

**E/F tiers (no minimap/beam/sound):**
```
SetTextColor R G B 255
SetBackgroundColor R G B 255
SetBorderColor R G B 255
SetFontSize N
```

**Hidden blocks:**
```
SetFontSize 21
DisableDropSound
```

### Recommended Canonical Order (D-03: Claude's discretion)

The currency block order is used by ~60 of the 85 blocks and is the most established pattern.
Canonical order for ALL blocks:

```
PlayEffect {Color}              # if beam present
MinimapIcon {Size} {Color} {Shape}  # if minimap present
SetTextColor R G B 255          # if text color present
SetBackgroundColor R G B 255    # if background color present
SetBorderColor R G B 255        # if border color present
SetFontSize N                   # if font present
CustomAlertSound "file" Vol     # if sound.custom present
PlayAlertSound Id Vol           # if sound.id present
DisableDropSound                # if disable_drop_sound == true
```

**Rationale:** Beam + minimap first (world effects), then colors (visual), then font (size), then
sound, then sound-disable. This matches the dominant pattern in the existing file (currency
blocks S/A/B/C/D). The only block that changes from this re-baseline is the Recycle block, where
SetTextColor/SetBorderColor/SetBackgroundColor/MinimapIcon/PlayEffect currently appear in the
wrong order.

**Important:** `unchanged` and `hidden` are special cases:
- `unchanged`: emits NO color/font/beam/sound/minimap actions (all null). Header only + conditions.
- `hidden`: emits only `SetFontSize 21` and `DisableDropSound` (its two non-null fields).

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| insta | 1.47.2 | Snapshot testing | Industry standard for Rust snapshot tests; `cargo insta review` workflow |
| (existing) serde + serde_json | 1.0 | Config deserialization | Already in project |
| (existing) std::fmt::Write | stdlib | String building | Zero-dep, deterministic, no allocation overhead |

[VERIFIED: `cargo search insta` confirms 1.47.2 is current, 2026-05-23]

### No Additional Runtime Dependencies Needed

The emitter requires only `std::fmt::Write` (stdlib) and the existing serde types. No new
runtime crates.

**Installation (dev-dependency):**
```toml
[dev-dependencies]
insta = "1.47.2"
```

No features required for plain string snapshots. The `yaml` feature is not needed since we are
snapshotting a raw string (the generated filter text), not a structured value.

---

## Architecture Patterns

### System Architecture Diagram

```
config.json ──load_json──► Config (refactored)  ──┐
                                                   ├──► emitter::generate(&Config, &CurrencyData)
currency.json ─load_json──► CurrencyData          ──┘         │
                                                               ▼
                                                       String (filter text)
                                                               │
                                      ┌────────────────────────┼────────────────────────┐
                                      ▼                        ▼                        ▼
                              cmd_generate              test: snapshot          test: artifact-guard
                              fs::write(output)    assert_snapshot!(text)   read universal.filter
                                                                             compare == text
```

### Recommended Project Structure

```
src/
├── main.rs             # cmd_generate: load → emit → write (replace stub body)
├── cli.rs              # unchanged
├── error.rs            # unchanged
├── loader.rs           # unchanged
├── emitter.rs          # NEW: pub fn generate(config: &Config, currency: &CurrencyData) -> String
└── models/
    ├── mod.rs          # add pub mod emitter? No — emitter is top-level src/emitter.rs
    ├── config.rs       # REFACTORED: styles HashMap → Styles struct; BandShow rarity → RarityRule enum
    ├── currency.rs     # unchanged (Vec-based, already deterministic)
    ├── prices.rs       # unchanged
    └── tiers.rs        # unchanged
tests/
├── cli_generate.rs     # EXTENDED: add snapshot test + artifact-guard test + determinism test
└── cli_errors.rs       # unchanged
snapshots/              # created by insta automatically (next to tests/ directory)
    └── cli_generate__generate_snapshot.snap  # committed after first run + review
.planning/phases/02-static-generate/
    └── diffs/          # D-14: config.diff and universal_filter.diff written at end of execution
```

### Pattern 1: Pure Emitter Function

**What:** The emitter is a pure function — takes immutable refs, returns String.
No file I/O, no side effects. File writing stays in `cmd_generate`.

**Why:** Makes snapshot testing trivial (snapshot the return value directly in a unit test),
keeps the emitter independently testable, and keeps IO out of the core logic.

```rust
// src/emitter.rs
use std::fmt::Write;
use crate::models::config::Config;
use crate::models::currency::CurrencyData;

pub fn generate(config: &Config, currency: &CurrencyData) -> String {
    let mut out = String::with_capacity(32_000); // universal.filter is ~31 KB
    emit_header(&mut out, config);
    emit_recycle(&mut out, config);
    emit_gear_bands(&mut out, config);
    emit_flask_charm_bands(&mut out, config);
    emit_currency_sections(&mut out, config, currency);
    out
}
```

**Source:** [ASSUMED — standard Rust pattern for code generation]

### Pattern 2: Single Action Emitter

**What:** One function owns the canonical action order. All blocks call it.

```rust
fn emit_style_actions(out: &mut String, style: &Style) {
    if let Some(beam) = &style.beam {
        let temp = if beam.temp { " Temp" } else { "" };
        writeln!(out, "  PlayEffect {}{}", beam.color.as_filter_str(), temp).unwrap();
    }
    if let Some(mm) = &style.minimap {
        writeln!(out, "  MinimapIcon {} {} {}", mm.size, mm.color.as_filter_str(), mm.shape.as_filter_str()).unwrap();
    }
    if let Some(rgb) = style.text {
        writeln!(out, "  SetTextColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    if let Some(rgb) = style.background {
        writeln!(out, "  SetBackgroundColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    if let Some(rgb) = style.border {
        writeln!(out, "  SetBorderColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    if let Some(font) = style.font {
        writeln!(out, "  SetFontSize {}", font).unwrap();
    }
    if let Some(sound) = &style.sound {
        if let Some(custom) = &sound.custom {
            writeln!(out, "  CustomAlertSound \"{}\" {}", custom, sound.vol).unwrap();
        } else if let Some(id) = sound.id {
            writeln!(out, "  PlayAlertSound {} {}", id, sound.vol).unwrap();
        }
    }
    if style.disable_drop_sound {
        writeln!(out, "  DisableDropSound").unwrap();
    }
}
```

**Source:** [ASSUMED — standard Rust pattern; verified filter action values from universal.filter]

### Pattern 3: Tier Name Display for Display

`FilterColor`, `MinimapShape` need an `as_filter_str()` method (or `Display` impl) that renders
the exact string the filter expects (e.g. `FilterColor::Pink` → `"Pink"`). Since these are
already PascalCase matching the filter values, `format!("{:?}", color)` works in many cases, but
an explicit mapping is cleaner and avoids Debug format surprises.

```rust
impl FilterColor {
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            FilterColor::Red => "Red",
            FilterColor::Green => "Green",
            // ...
        }
    }
}
```

[ASSUMED — standard Rust pattern]

### Anti-Patterns to Avoid

- **HashMap iteration for styles:** Already a Phase 1 known issue (D-11). Eliminate by
  converting `HashMap<String, Style>` → `Styles` named-fields struct.
- **Match on string rarity rules:** Don't parse `"Magic + Rare"` strings at emit time. The
  `RarityRule` enum (D-10) means the emitter dispatches on a type-safe variant.
- **Inline `\r\n` in string literals:** On Windows with `core.autocrlf=true`, the working-tree
  filter is CRLF. The generator must emit LF only; the artifact-guard test must normalize both
  sides to LF before comparing (see CRLF Trap in Pitfalls).
- **`write!` returning ignored errors:** `String`'s `fmt::Write` impl never fails — it's safe to
  `.unwrap()` on `write!` calls into `String`. Do not use `?` propagation here.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot regression | Custom SHA-256 + expected-file comparison | `insta::assert_snapshot!` | insta handles file naming, review workflow, CI auto-reject, `.snap` format |
| String formatting | Manual `String::push_str` chains | `write!` / `writeln!` into String via `std::fmt::Write` | Cleaner, infallible for String, identical performance |
| Custom review workflow | Manual "save expected output" scripts | `cargo insta review` + `INSTA_UPDATE` | Already handles accept/reject/diff display |

**Key insight:** The entire emitter is essentially a code-generation problem; keeping it as
`fmt::Write` calls into a String means no parser, no intermediate IR, no allocation overhead
beyond the output String itself.

---

## config.json Schema Refactor (D-10/D-11)

### D-11: Styles HashMap → Named Struct

**Current `config.rs`:**
```rust
pub styles: HashMap<String, Style>,
```

**Target:**
```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Styles {
    #[serde(rename = "S")] pub s: Style,
    #[serde(rename = "A")] pub a: Style,
    #[serde(rename = "B")] pub b: Style,
    #[serde(rename = "C")] pub c: Style,
    #[serde(rename = "D")] pub d: Style,
    #[serde(rename = "E")] pub e: Style,
    #[serde(rename = "F")] pub f: Style,
    #[serde(rename = "Recycle")] pub recycle: Style,
    pub unchanged: Style,
    pub hidden: Style,
}

// In Config:
pub styles: Styles,
```

**config.json change:** No change to JSON structure — serde's `#[serde(rename)]` handles the
uppercase key names. The JSON stays `"S": {...}`, `"A": {...}`, etc.

**Emitter access:** `config.styles.s`, `config.styles.recycle`, etc. — fully deterministic,
no HashMap lookup.

[ASSUMED — standard serde pattern; verified against existing Style struct shape]

### D-10: Rarity Strings → RarityRule Enum

**Current values in config.json `areaLevelBands[].show` fields:**
- `"any rarity"` — appears in weapons (1-5), allArmour (1-5), flasksCharms (1-5 band)
- `"Magic + Rare"` — appears in armour rarity fields
- `"Rare only"` — appears in weapons (26+)
- `"Magic"` — appears in flasksCharms (26-65 band)
- `"Quality only"` — appears in flasksCharms (66+ band)

[VERIFIED: enumerated from config.json areaLevelBands section]

**Target enum:**
```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RarityRule {
    #[serde(rename = "any rarity")]    Any,
    #[serde(rename = "Rare only")]     RareOnly,
    #[serde(rename = "Magic + Rare")]  MagicPlusRare,
    #[serde(rename = "Magic")]         Magic,
    #[serde(rename = "Quality only")]  QualityOnly,
}
```

**config.json change:** The JSON strings stay unchanged — `#[serde(rename)]` maps the existing
string values to enum variants. No JSON rewrite needed for this field.

However, `BandShow.weapons`, `BandShow.all_armour`, `BandShow.other_gear`, and
`BandShow.flasks_charms` currently type as `Option<String>`. They change to
`Option<RarityRule>`. `ArmourShow.rarity` changes from `String` to `RarityRule`.

**Emitter dispatches:**
```rust
match rarity_rule {
    RarityRule::Any           => { /* no Rarity condition — show all */ }
    RarityRule::RareOnly      => writeln!(out, "  Rarity == Rare").unwrap(),
    RarityRule::MagicPlusRare => writeln!(out, "  Rarity >= Magic").unwrap(),
    RarityRule::Magic         => writeln!(out, "  Rarity == Magic").unwrap(),
    RarityRule::QualityOnly   => writeln!(out, "  Quality >= 1").unwrap(),
}
```

[ASSUMED — serde rename pattern; verified original string values from config.json]

### Updated Test Implications (D-12)

The Phase 1 tests in `config.rs` that reference `config.styles.len()` or use HashMap access
must be updated. Specific changes:

- `config_parses_real_file` — continues to work (no change to test logic)
- `config_round_trips` — continues to work (struct PartialEq is deterministic unlike HashMap)
- `config_rejects_unknown_key` — may need fixture JSON updated if fixture constructs styles inline
- `config_rejects_bad_enum_color` — unaffected
- `filter_color_variants_roundtrip` — unaffected

In `main.rs`, the stub `config.styles.len()` call must become a count of something else (e.g.
`10` hardcoded, or removed, since Phase 2 replaces the entire stub body).

---

## insta Snapshot Testing

[VERIFIED: docs.rs/insta/1.47.2, insta.rs/docs, cargo search]

### Setup

```toml
[dev-dependencies]
insta = "1.47.2"
```

No features needed for plain string snapshots.

### assert_snapshot! Macro

```rust
// Named snapshot (recommended for the filter — prevents name collisions):
insta::assert_snapshot!("generate_snapshot", generated_string);

// Or via use:
use insta::assert_snapshot;
assert_snapshot!("generate_snapshot", generated_string);
```

The snapshot name `"generate_snapshot"` combines with the test module name to produce the `.snap`
filename: `snapshots/cli_generate__generate_snapshot.snap` (when the test is in `tests/cli_generate.rs`).

For inline snapshots (inline review in source file) use `@"..."` syntax — but for a 848-line
filter, the file-based snapshot is the right choice. Use the named form.

### Snap File Location

Insta stores `.snap` files in a `snapshots/` folder **next to the test file**.

For `tests/cli_generate.rs` → snapshots live at `tests/snapshots/`.
For `#[cfg(test)]` in `src/emitter.rs` → snapshots live at `src/snapshots/`.

Recommendation: put the snapshot test in `tests/cli_generate.rs` (follows existing pattern) so
snapshots live in `tests/snapshots/`. This directory must be committed to git.

### Snap File Format (plain string)

```
---
source: tests/cli_generate.rs
expression: generated_filter_string
---
# universal.filter
# GENERATED FROM config.json ...
[... rest of generated filter ...]
```

The YAML front-matter is written by insta. The body is the raw string value. Insta normalizes
all newlines to LF in snapshot files regardless of platform — this is key for Windows
compatibility.

### Workflow

**First run (no `.snap` file yet):**
```
INSTA_UPDATE=new cargo test
```
This creates `tests/snapshots/cli_generate__generate_snapshot.snap.new`.

**Review and accept:**
```
cargo insta review
```
Press `a` to accept. The `.snap.new` is renamed to `.snap`.

Or force-accept without interactive review:
```
cargo insta test --review
```

**Commit the `.snap` file.** It is the regression oracle.

**CI behavior:** `INSTA_UPDATE` defaults to `auto`, which is `no` in CI. Tests fail if the
snapshot doesn't match — the `.snap` file must be committed. CI never auto-updates snapshots.

**Updating the snapshot after deliberate change:**
```
cargo insta test --review
```
Review the diff, accept the new value, commit the updated `.snap`.

### The Snapshot Test

```rust
// tests/cli_generate.rs
#[test]
fn generate_snapshot() {
    // Build and run the binary, capture stdout or use the emitter directly.
    // Recommended: call emitter directly if exposed, or run `ferrisfilter generate --output -`
    // But since emitter returns String (not a library fn), test via the binary writing to a
    // temp path, then read the output.
    let dir = /* temp dir with config.json, currency.json, tiers.json */;
    Command::cargo_bin("ferrisfilter").unwrap()
        .current_dir(&dir)
        .args(["generate", "--output", "out.filter"])
        .assert().success();
    let generated = std::fs::read_to_string(dir.join("out.filter")).unwrap();
    // Normalize CRLF → LF before snapshotting:
    let normalized = generated.replace("\r\n", "\n");
    insta::assert_snapshot!("generate_snapshot", normalized);
}
```

Alternatively: expose `emitter::generate` as `pub` and call it directly in a unit test within
`src/emitter.rs` — this avoids the binary round-trip and is faster. The emitter can be tested
as a pure Rust function.

```rust
// src/emitter.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{config::Config, currency::CurrencyData};

    #[test]
    fn snapshot_full_filter() {
        let config: Config = serde_json::from_str(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json"))
        ).unwrap();
        let currency: CurrencyData = serde_json::from_str(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/currency.json"))
        ).unwrap();
        let output = generate(&config, &currency);
        insta::assert_snapshot!("full_filter", output);
    }
}
```

This is the recommended approach: snapshot lives at `src/snapshots/emitter__tests__full_filter.snap`.

### Artifact-Guard Test (D-06)

The committed `universal.filter` must equal the generator output. This test:
1. Loads config.json + currency.json (from CARGO_MANIFEST_DIR)
2. Calls `emitter::generate()`
3. Reads the committed `universal.filter` from disk
4. Asserts both are equal (after normalizing CRLF → LF on both sides)

```rust
#[test]
fn artifact_guard() {
    let config: Config = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json"))
    ).unwrap();
    let currency: CurrencyData = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/currency.json"))
    ).unwrap();
    let generated = emitter::generate(&config, &currency);
    let committed = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/universal.filter")
    ).unwrap();
    let gen_lf = generated.replace("\r\n", "\n");
    let com_lf = committed.replace("\r\n", "\n");
    assert_eq!(gen_lf, com_lf,
        "committed universal.filter does not match generator output — run `ferrisfilter generate` and commit");
}
```

### Determinism Test (D-07)

```rust
#[test]
fn generate_is_deterministic() {
    let config: Config = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json"))
    ).unwrap();
    let currency: CurrencyData = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/currency.json"))
    ).unwrap();
    let first = emitter::generate(&config, &currency);
    let second = emitter::generate(&config, &currency);
    assert_eq!(first, second, "generate() is not deterministic");
}
```

---

## Common Pitfalls

### Pitfall 1: CRLF / LF Mismatch (HIGH risk on Windows)

**What goes wrong:** The emitter writes LF-only strings (using `writeln!` which emits `\n`).
On Windows with `core.autocrlf = true`, git checks out `*.filter` files as CRLF. The
artifact-guard test reads the committed `universal.filter` from disk and gets CRLF, while
`emitter::generate()` returns LF-only. The string comparison fails.

**Why it happens:** `.gitattributes` has `*.filter text` and `core.autocrlf = true`. Working-tree
checkout is CRLF on Windows. [VERIFIED: `cat -A universal.filter` confirms CRLF in working tree;
`git show HEAD:universal.filter` confirms LF in git blob]

**How to avoid:** Always normalize both sides to LF (`s.replace("\r\n", "\n")`) before comparing
in tests. The generator itself should emit LF-only (use `writeln!` which produces `\n` on all
platforms). When writing the output file with `fs::write`, the content should be LF-only — let
git's autocrlf handle the CRLF conversion on checkout if needed. The insta snapshot will contain
LF (insta normalizes to LF per its documentation). The artifact-guard normalizes before compare.

**Warning signs:** `assert_eq!` failure showing only `\r` difference. GitHub Actions CI (Linux)
passes, Windows local fails.

### Pitfall 2: HashMap styles iteration order

**What goes wrong:** If `styles` remains `HashMap<String, Style>`, any code that iterates the
map to, e.g., validate all tiers are present or emit a lookup table, will have nondeterministic
output between runs.

**Why it happens:** Rust's HashMap uses a randomized hasher by default.

**How to avoid:** D-11 eliminates the HashMap entirely. The typed `Styles` struct has named
fields — no iteration order issue.

**Warning signs:** GEN-01 determinism test passes most of the time but fails occasionally.

### Pitfall 3: Incorrect gear block Class list

**What goes wrong:** Emitting the wrong class list for gear blocks — e.g., putting
`armourClasses` instead of `build.armour` in the armour-type progressive Show blocks.

**Why it happens:** There are three distinct class lists:
- `armourClasses` = `["Body Armours", "Helmets", "Gloves", "Boots", "Shields", "Bucklers"]` (6 classes, includes shields/bucklers)
- `build.armour` = `["Body Armours", "Helmets", "Gloves", "Boots"]` (4 classes, no shields/bucklers)
- `otherGear` = `weaponClasses + armourClasses` (18 classes, for the any-Rare-<=45 block)

[VERIFIED: from universal.filter lines 53–78 and config.json build.armour field]

The armour-type progression blocks (anyNonEvasion / esHybrid / pureES) use `build.armour` (no
shields/bucklers). The "all armour <= 5" block and the gear fallback Hide use `armourClasses` (with
shields/bucklers). The `allArmour` band-1-5 block also uses `armourClasses`.

**How to avoid:** Cross-reference the block class list against universal.filter lines 36–86 during implementation.

### Pitfall 4: Band logic is cross-band, not per-band

**What goes wrong:** Naively iterating `areaLevelBands` and emitting one block per band per
show-key produces too many blocks with wrong conditions.

**Why it happens:** The current filter merges bands. Example: weapons "any rarity" appears in
band 1-5 and 6-25, but is emitted as ONE block with `AreaLevel <= 25`. The "Rare only" weapons
block covers bands 26-45, 46-65, AND 66+ as ONE block with no lower/upper AreaLevel bound.

**How to avoid:** The emitter should NOT iterate bands naively. The logic for each block type
must be hardcoded to the merged condition that reproduces the established structure (GEN-02).
The planner should treat each gear/flask block as a distinct emitter function with the specific
logic — not a loop over areaLevelBands.

The `areaLevelBands` data in `config.json` is used as *input policy* (what rarity/type applies
at each level) but the *specific block structure* (merged conditions) is derived knowledge.

### Pitfall 5: The `unchanged` style emitting spurious actions

**What goes wrong:** `emit_style_actions` on the `unchanged` style emits nothing (all fields
None / false), which is correct. But if the emitter accidentally calls the wrong style lookup,
gear show-blocks get no actions (correct) while currency blocks do get actions (correct). A
copy-paste error could emit `styles.unchanged` for a currency block or vice versa.

**How to avoid:** The emitter clearly separates the two paths: gear/flask blocks always use
`unchanged` style (emit no actions); currency blocks always resolve a tier style. No dynamic
dispatch needed.

### Pitfall 6: Missing trailing newline

**What goes wrong:** The generator's output doesn't end with a final `\n`, causing the insta
snapshot to differ from a file that does, and making the file look wrong in diffs.

**How to avoid:** After emitting the last block, ensure the output ends with exactly one `\n`.
`writeln!` after the last entry guarantees this. Verify by checking
`assert!(output.ends_with('\n'))` in the determinism test or as an assertion in `generate()`.

[VERIFIED: universal.filter ends at line 848 with `  PlayAlertSound 2 200` — the file DOES end
without a trailing blank line. The last block (Vault Keys C) has no blank line after it.]

---

## The Diff Artifact (D-14)

At the end of Phase 2 execution, the executor must produce two human-reviewable diffs:
1. `config.json` (old → refactored)
2. `universal.filter` (old hand-built → regenerated)

### Recommended Approach (Windows/Git environment)

**Before making any changes,** record the current HEAD SHA as the "before" reference. The
refactored `config.json` and regenerated `universal.filter` will be committed as part of the
phase. After committing, produce diffs:

```bash
# Save the before-SHA at the start of execution (before any changes):
BEFORE_SHA=$(git rev-parse HEAD)

# ... make changes, run generate, commit ...

# Produce diffs after the phase commit:
git diff ${BEFORE_SHA}..HEAD -- config.json > .planning/phases/02-static-generate/config.diff
git diff ${BEFORE_SHA}..HEAD -- universal.filter > .planning/phases/02-static-generate/universal_filter.diff
```

The `.diff` files are text (git diff format with `---`/`+++` unified diff). They are
human-readable.

**On Windows with `core.autocrlf=true`:** `git diff` outputs LF-normalized diffs regardless of
working tree encoding. No special handling needed.

**Alternative if committing happens in stages:** Use `git diff HEAD` (unstaged) or
`git diff --cached` (staged) at the appropriate point, or save before/after snapshots of each
file to temp locations and diff them:

```bash
# Save before state:
git show HEAD:config.json > /tmp/config_before.json
git show HEAD:universal.filter > /tmp/universal_before.filter

# ... make changes, run generate ...

# Diff against before state:
git diff --no-index /tmp/config_before.json config.json > .planning/phases/02-static-generate/config.diff || true
git diff --no-index /tmp/universal_before.filter universal.filter > .planning/phases/02-static-generate/universal_filter.diff || true
```

The `|| true` is needed because `git diff --no-index` exits non-zero when files differ.

On PowerShell, adapt:
```powershell
git show HEAD:config.json | Out-File -Encoding utf8 $env:TEMP\config_before.json
git show HEAD:universal.filter | Out-File -Encoding utf8 $env:TEMP\universal_before.filter
```

[VERIFIED: `git diff HEAD --` confirmed working in this repo. `git show HEAD:config.json`
confirmed working.]

---

## Determinism Traps Checklist

| Trap | Present? | Resolution |
|------|----------|-----------|
| `HashMap<String, Style>` styles | YES (Phase 1 config.rs) | D-11: convert to `Styles` named struct |
| `Build.armour_types: HashMap<String, Vec<String>>` | YES (Phase 1 config.rs) | Emitter hardcodes armour-type keys (anyNonEvasion/esHybrid/pureES by name) — does not iterate the map. Or convert to struct. |
| `currency.categories` Vec iteration | No issue | Vec → deterministic order |
| `category.entries` Vec iteration | No issue | Vec → deterministic order |
| `bases` Vec in each entry | No issue | Vec → deterministic order |
| Float formatting (tiers.json) | N/A (not emitted in Phase 2) | — |
| OS-dependent line endings | YES (Windows CRLF checkout) | Emit LF only; normalize before compare in tests |
| String-keyed enum lookup | Eliminated by D-10 | RarityRule enum replaces string matching |
| `#[derive(Hash)]` on custom types | N/A | No custom hash keys in emitter |

**The `Build.armour_types` HashMap** is a secondary determinism risk. The emitter needs to look
up `anyNonEvasion`, `esHybrid`, and `pureES` — three specific keys. Options:
1. Hardcode the three lookups by string key: `config.build.armour_types.get("anyNonEvasion")` — deterministic (lookup by key, not iteration)
2. Convert `armour_types` to a struct with named fields (analogous to D-11 for styles)

Option 1 is simpler for Phase 2. Option 2 is cleaner but more refactor work. The planner can
choose — both are deterministic as long as the emitter does NOT iterate `armour_types.keys()`.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `HashMap<String, Style>` in config | `Styles` named struct (D-11) | Phase 2 | Determinism; type safety |
| Freeform rarity strings | `RarityRule` closed enum (D-10) | Phase 2 | Parse-at-emit eliminated |
| Stub `cmd_generate` | Full emitter | Phase 2 | GEN-01 through GEN-04 satisfied |
| Hand-built `universal.filter` | Generator-produced, insta-locked | Phase 2 | Future edits go through config/code |

**The `filter-model.json` filename was renamed to `config.json` in Phase 1 (D-12 of Phase 1).
The header comment in universal.filter still references "filter-model.json" — the re-baseline
permits updating this to "config.json" (minor cleanup per D-02).**

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | Emitter build | ✓ | edition 2024, cargo check passes | — |
| insta crate | Snapshot tests | To be added | 1.47.2 | — |
| cargo-insta CLI | Snapshot review | Optional (dev workflow) | 1.47.2 | Use `INSTA_UPDATE=always` |
| git | Diff artifact (D-14) | ✓ | Available (git log confirmed) | — |
| config.json | Emitter input | ✓ | Present at repo root | — |
| currency.json | Emitter input | ✓ | Present at repo root, 15 categories | — |
| tiers.json | Already loaded by cmd_generate | ✓ | Present at repo root | — |

`cargo-insta` is not required for tests to pass — it is only needed for the interactive review
workflow. CI uses `INSTA_UPDATE=no` (default in CI environments via `auto` setting).

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 2 |
|-----------|-------------------|
| `universal.filter` is generated — do not hand-edit | Phase 2 makes this structurally enforced via generator + insta lock |
| Block tags `[alwaysShow.K]`, `[areaLevelBands "R".K]`, `[styles.X]` | Each emitted block must carry its tag in the block header comment |
| Design philosophy: uniques/jewels/gems/jewellery/quest/waystones/trials/unidentified get NO rules | GEN-04: emitter must not emit any blocks for these item types |
| `Rarity == Unique` blanket styling is explicitly out of scope | Confirmed: no such blocks in universal.filter, emitter must not add them |
| Native rarity color preserved on uncurated equipment | `unchanged` style: emit NO SetTextColor/SetBorderColor/SetBackgroundColor |
| `uwu.mp3` lives in `assets/` | `sound.custom` value is `"uwu.mp3"` — emitter emits this string verbatim from the config model |
| `==` (exact match) used for Class/BaseType in currency blocks | All currency blocks use `Class == "..."` and `BaseType == "..."` |
| Gear blocks use `Class "..."` without `==` | Gear/flask blocks use bare `Class "..."` (substring match) |
| Keep `Show # Description (slug)` naming convention | Block header comment format must follow this pattern |
| Ignore the trailing poe2filter.com config block | Not present in universal.filter; emitter must not add it |
| CLAUDE.md prose predates `filter-model.json`; when it conflicts with the model, the model wins | The `jewels-all` block described in CLAUDE.md does NOT exist in universal.filter and must NOT be added (GEN-04) |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `emit_style_actions` single-function pattern is idiomatic for this use case | Architecture Patterns | Low — alternative is inline per-block logic, which is more verbose but also deterministic |
| A2 | `String`'s `fmt::Write` impl never returns Err (safe to `.unwrap()`) | Architecture Patterns | Negligible — this is stdlib-guaranteed behavior |
| A3 | `RarityRule` enum with `#[serde(rename)]` correctly round-trips the existing string values | config.json Refactor | Medium — if exact string values differ from what's listed, serde deserialization fails. Values verified from config.json directly |
| A4 | `Build.armour_types` HashMap can be iterated by specific key lookups without converting to a struct | Determinism Traps | Low — key-lookup is deterministic; the only risk is if a future refactor adds iteration |
| A5 | insta normalizes snap file newlines to LF on all platforms | insta Snapshot Testing | Medium — if insta doesn't normalize, the snapshot comparison still works because the generated string is LF-only; only the snap file format matters |
| A6 | The emitter should emit exactly one trailing `\n` (no final blank line) | Common Pitfalls / Pitfall 6 | Low — universal.filter ends without a trailing blank line (verified); final block has content on last line |

---

## Open Questions

1. **Band 1-5 weapons: is the fall-through intentional?**
   - What we know: band 1-5 in config.json has `weapons: "any rarity"` as a key, but universal.filter does NOT emit a band-1-5 weapons block. Weapons in the 1-5 zone fall through to game default.
   - What's unclear: Was the band-1-5 weapons entry in config.json intended to produce a Show block that was accidentally omitted, or is it documented behavior that weapons fall through at 1-5?
   - Recommendation: Preserve the current behavior (no 1-5 weapons block) to maintain GEN-02 fidelity. The band-1-5 weapons entry in config.json may simply be unused (the `any rarity <= 25` weapons block already covers level 1-25 implicitly since the first block is `AreaLevel <= 25`). Note in the emitter: band-1-5's `weapons` field is NOT used to emit a block.

2. **`build.armour_types` as HashMap — convert to struct or leave as-is for Phase 2?**
   - What we know: The emitter needs three specific keys (`anyNonEvasion`, `esHybrid`, `pureES`). HashMap lookup by key is deterministic. Full struct conversion is cleaner but more refactor surface.
   - Recommendation: For Phase 2, use key lookups (`.get("anyNonEvasion").expect(...)`) — simpler, deterministic, and compatible with existing Phase 1 tests. A struct refactor can happen in a future phase.

3. **Should the refactored `config.json` preserve the current JSON formatting (spacing, key order)?**
   - What we know: D-14 says diffs will be presented to the user; a reformat-only diff adds noise. However, D-12 says "exact JSON spelling is Claude's discretion."
   - Recommendation: Preserve the existing compact style and key order as closely as possible; only change what's structurally required (e.g., no schema changes to JSON needed for D-11/D-10 — see Refactor section). The diff should show only semantic changes.

---

## Sources

### Primary (HIGH confidence)
- `universal.filter` — read in full (848 lines, all 85 blocks catalogued) [VERIFIED]
- `config.json` — read in full [VERIFIED]
- `currency.json` — read in full (15 categories, all entries catalogued) [VERIFIED]
- `src/models/config.rs` — current Phase 1 models [VERIFIED]
- `src/models/currency.rs`, `src/main.rs`, `src/loader.rs` — existing code [VERIFIED]
- `Cargo.toml` — dependency versions [VERIFIED]
- `.gitattributes` — CRLF handling rules [VERIFIED]
- `docs.rs/insta/1.47.2` — assert_snapshot! usage, INSTA_UPDATE modes, .snap file location
- `insta.rs/docs` — setup, workflow, CI behavior [VERIFIED via WebFetch]

### Secondary (MEDIUM confidence)
- `cargo search insta` — confirmed insta 1.47.2 is current version [VERIFIED]

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Complete emission inventory: HIGH — read universal.filter in full, cross-checked against currency.json
- Canonical action order: HIGH — analyzed all 85 blocks directly
- insta setup/workflow: HIGH — verified against docs.rs and insta.rs docs
- config.json refactor patterns: HIGH — serde rename pattern verified against existing model
- CRLF trap: HIGH — verified .gitattributes and working-tree encoding
- Diff artifact approach: HIGH — verified git commands work in this repo
- Emitter architecture: MEDIUM — standard Rust patterns (assumed, not verified against a specific authority)

**Research date:** 2026-05-23
**Valid until:** 2026-09-01 (stable domain — filter grammar, insta API, and Rust serde patterns are stable)
