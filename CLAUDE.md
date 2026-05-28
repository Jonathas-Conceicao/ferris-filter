# CLAUDE.md — FerrisFilter

This repo is **FerrisFilter**: a Rust CLI that compiles a human-editable config into a
Path of Exile 2 loot filter (`generate`) and fetches item prices from poe.ninja (`sync`).

- **Project plan & workflow:** managed with GSD in `.planning/` — see `.planning/PROJECT.md`
  (context), `.planning/ROADMAP.md` (4 phases), `.planning/REQUIREMENTS.md`, and
  `.planning/STATE.md`. Git uses a **milestone branching** workflow (see PROJECT.md → Git Workflow).
- **Domain spec (below):** the filter grammar, the Dracula style-tier system, and the design
  philosophy. `config.json` is the executable spec the `generate` command compiles. The
  **byte-for-byte acceptance baseline** is now the committed fixture
  `tests/fixtures/seed-universal.filter` (generated from `config.json` + the seed
  `tests/fixtures/seed-prices.json`); the `artifact_guard` test enforces it. The repo-root
  `universal.filter` / `prices.json` are **generated, git-ignored, and not committed**.
  Everything from "Repository layout" down is that reference — keep it accurate.

## Repository layout

| Path | Purpose |
|------|---------|
| `filter-model.json` | **Source of truth.** Compact policy model the filter is generated from: `styles` (the Dracula tier palette + `unchanged`/`hidden`), `build` (weapon/armour classes + defence-type taper), `areaLevelBands` (per-level show logic), and `alwaysShow` (currency, recycle). Edit this, then regenerate. |
| `universal.filter` (repo root) | **Generated** by `ferrisfilter generate` and **git-ignored** — regenerate on demand, do not commit or hand-edit. Each block is tagged with its model path (e.g. `[areaLevelBands "6-25".weapons -> styles.unchanged]`). |
| `prices.json` (repo root) | **Generated** by `ferrisfilter sync` from poe.ninja and **git-ignored** — your live economy data, regenerated per sync, never committed. |
| `tests/fixtures/seed-prices.json`, `tests/fixtures/seed-universal.filter` | **Committed test fixtures** — the frozen seed prices + its generated filter. These are the test oracle (`include_str!` unit tests + `generate_snapshot`/`artifact_guard`). Regenerate the `.filter` fixture only after a deliberate generator change. |
| `prices-fallback.json` + `tiers-fallback.json` (repo root) | **Committed early-league fallback pair**, always used *together* (their values are calibrated to each other). `generate` loads them instead of `prices.json`/`tiers.json` when `prices.json` is absent/empty (with a warning) or when `--early-game` is passed. Tier membership comes from a poe2filter.com export; cutoffs are **exalt-denominated** (`unit: "exalt"`) so Divine isn't pinned to the A floor. Every base — orbs included — is priced in **exalt at its tier floor** (Divine 20→S, Chaos 2.5→A, Exalt 1→B). No stored rates: `pricer::derive_rates` reads the three anchor orbs (items[0..2]) and expresses them relative to the Chaos Orb (`exalt_in_chaos = value(Exalt)/value(Chaos)` etc.), so quoting the orbs in exalt is enough. The same code on live chaos-quoted data (Chaos = 1c) reproduces the old numbers. The three anchors must share one unit. |
| other `*.filter` (repo root) | Active, version-controlled filters we author/maintain. |
| `reference-old-filters/` | Older/stale filters copied from the game's config folder. **Git-ignored**, kept only as reference. Do not edit these as if they were live; treat them as read-only history. |
| `ferrisfilter install` | Subcommand that copies `universal.filter` into the in-game config folder (replaces the old `install.bat`). Defaults to `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2`; pass `--dest <dir>` to override and `--input <file>` for a non-default filter. Copy-only — run `generate` first. |
| `CLAUDE.md` | This file — the filter-syntax reference so we rarely need to re-fetch the docs. |

> **Design philosophy (current):** Hide what we don't want, style the few highlights, and let
> everything else fall through to the game default. Uniques, jewels, gems, jewellery, quest
> items, waystones, trials, and unidentified items have **no rules** — they show natively.
> The detailed tier/progression prose further down predates `filter-model.json`; when it
> conflicts with the model, **the model wins** (that section needs reconciling).

Source folder for filters in-game (where they must be copied to be usable):
`C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2`

External references (fetch only when something here is insufficient):
- Filter syntax: https://www.pathofexile.com/item-filter/about
- Exact item names / base values: https://poe2db.tw/us/

> **Caveat:** The official `item-filter/about` page documents the PoE1 filter language.
> PoE2 uses the *same grammar* but a different set of valid values (classes, base types,
> tiers) and omits some PoE1 concepts (see "PoE2 differences" below). When in doubt about
> an exact base name, class, or numeric range, verify against poe2db.tw or the live game.

---

## Filter grammar

A filter is an ordered list of **blocks**. Each item that drops is tested against blocks
**top to bottom**; the **first matching block wins** (unless `Continue` is used). A block is
a header (`Show` / `Hide` / etc.) followed by indented **conditions** and **actions**.

```
Show # Human description (optional-slug)
  Rarity == Unique
  Class == "Body Armours"
  SetTextColor 53 30 14 255
  MinimapIcon 1 Brown Kite
  PlayEffect Brown
```

### Block types
- `Show` — display matching items.
- `Hide` — conceal matching items.
- `Minimal` — minimal label (Ruthless filters only; rarely relevant for PoE2).
- `Continue` — placed inside a block so matching does **not** stop here; subsequent blocks
  may also apply (lets you layer styling rules).
- `Import "file.filter" [Optional]` — include another filter file.

### Comments
- `#` starts a comment to end of line. A `#` after a block header is a common convention for
  naming the block (e.g. `Show # Excellent Uniques (uniques-excellent)`).

### Operators
- `=` equal · `==` exact match · `!=` / `!` not equal
- `<` `<=` `>` `>=` for numeric/ordinal conditions
- For named/list conditions (e.g. `BaseType`, `Class`), listing multiple quoted values means
  **match any**. `==` enforces *exact* full-name match; without `==` it matches as a
  substring/prefix (e.g. `Class "Bows"` also matches `"Bow"`-containing class names). Prefer
  `==` with full names to avoid accidental matches.

---

## Conditions (common / PoE2-relevant)

**Numeric / ordinal**
- `ItemLevel` — item level.
- `DropLevel` — base item's drop level.
- `AreaLevel` — level of the area where it dropped.
- `Quality` — 0–20+ (corruption can exceed).
- `Rarity` — `Normal` < `Magic` < `Rare` < `Unique` (supports `<`/`>` ordering and `==`).
- `Sockets` — socket count (e.g. `Sockets >= 3`). PoE2 sockets hold runes/soul cores.
- `Height` / `Width` — item's inventory dimensions.
- `StackSize` — for stackable currency.
- `WaystoneTier` — PoE2 map (Waystone) tier.
- `BaseArmour` / `BaseEvasion` / `BaseEnergyShield` — base defence values.
- `GemLevel`, `MapTier` (PoE1), `MemoryStrands`, etc. — exist in the grammar but verify
  PoE2 applicability before relying on them.

**Boolean (value `True`/`False`)**
- `Corrupted`, `Mirrored`, `Identified`, `FracturedItem`, `HasImplicitMod`, `AnyEnchantment`
- `IsVaalUnique`, `HasVaalUniqueMod` (seen in the user's PoE2 uniques rules)

**Named / string**
- `Class "..."` — item class (PoE2 classes include: `Bows`, `Crossbows`, `Quarterstaves`,
  `Spears`, `Wands`, `Sceptres`, `Staves`, `Foci`, `One Hand Maces`, `Two Hand Maces`,
  `Quivers`, `Talismans`, `Body Armours`, `Helmets`, `Gloves`, `Boots`, `Shields`,
  `Bucklers`, `Rings`, `Amulets`, `Belts`, `Stackable Currency`, ...). Verify exact spelling.
- `BaseType "..."` — exact base item name (e.g. `"Sapphire Ring"`, `"Gold"`). The most
  precise lever; pull exact names from poe2db.tw.
- `Rarity` — see above.
- `HasExplicitMod`, `HasEnchantment`, `HasInfluence` — verify PoE2 support before use.

---

## Actions (styling & alerts)

**Colors** — `R G B [A]`, each `0–255`:
- `SetTextColor R G B A`
- `SetBorderColor R G B A`
- `SetBackgroundColor R G B A` (default alpha 240)

**Text size**
- `SetFontSize N` — `1–45` (in-game usable range ~18–45; the user's filters use up to 37).

**Sounds**
- `PlayAlertSound Id Volume` — `Id` 1–16, `Volume` 0–300 (default 50).
- `PlayAlertSoundPositional Id Volume` — positional variant.
- `CustomAlertSound "file" [Volume]` — custom sound file (e.g. the bundled `uwu.mp3`),
  volume 0–300 (default 100). `CustomAlertSoundOptional` to tolerate a missing file.
- `DisableDropSound` / `EnableDropSound` — toggle the default drop sound.

**Map / world**
- `MinimapIcon Size Colour Shape`
  - `Size`: `0` (largest) – `2` (smallest).
  - `Colour`: Red, Green, Blue, Brown, White, Yellow, Cyan, Grey, Orange, Pink, Purple.
  - `Shape`: Circle, Diamond, Hexagon, Square, Star, Triangle, Cross, Moon, Raindrop, Kite,
    Pentagon, UpsideDownHouse.
- `PlayEffect Colour [Temp]` — colored beam on the ground. Same colour set as above.
  `Temp` shows only briefly when the item drops.

---

## PoE2 differences to remember
- **No linked sockets / colors-as-links.** PoE2 has no `LinkedSockets`/`SocketGroup` link
  mechanic; `Sockets N` is just a count. Don't author link-based rules.
- **Waystones, not maps.** Use `WaystoneTier` for endgame map tiering.
- **Different base/class names.** Many PoE1 base types don't exist; always confirm via
  poe2db.tw before adding a `BaseType`.
- Existing filters in this repo were generated/edited with **poe2filter.com** and use a
  `Show # Description (slug)` naming convention — keep that style for consistency.

## Tier & color system (Dracula palette)

Color signals **value/desirability**. Text + border use the tier color; background is the
basic Dracula bg `40 42 54 255`. Beams/sounds only on the most valuable. These tier colors
are stable across leagues — only *which items* sit in each tier changes per season.

> **Native rarity color is preserved on uncurated equipment.** Tier *colors* apply only to
> **curated** drops — specific named bases (`BaseType ==` lists), currency, gems, and the
> Recycle/unique rules. Generic equipment/jewellery/flask/charm blocks that match by
> `Class` + `Rarity` alone must **not** repaint the item: omit `SetTextColor` /
> `SetBorderColor` / `SetBackgroundColor` so the game's native rarity color (Normal white,
> Magic blue, Rare yellow) shows through. For those blocks, **emphasis is conveyed by font
> size only** — bigger = more relevant, font 21 = de-emphasized.

| Tier | Meaning | Color | RGB (text+border) | Alert |
|---|---|---|---|---|
| **S** | top | Pink | `255 121 198 255` | `CustomAlertSound "uwu.mp3" 300` + beam + minimap |
| **A** | excellent | Purple | `189 147 249 255` | `CustomAlertSound "uwu.mp3" 300` + beam + minimap |
| **B** | high | Red | `255 85 85 255` | `PlayAlertSound` + beam + minimap |
| **C** | notable | Yellow | `241 250 140 255` | minimap icon |
| **D** | minor | Cyan | `139 233 253 255` | — |
| **E** | baseline | White | `248 248 242 255` | — |
| **F** | unwanted | *native rarity color* | (no color override) | **`Hide` (Alt-reveals it) — styled small font 21 + `DisableDropSound`** |
| **Recycle** ★ | salvage value | Green | `80 250 123 255` | green minimap icon + beam; overrides tier |

Full Dracula RGB reference: bg `40 42 54` · fg/white `248 248 242` · cyan `139 233 253` ·
green `80 250 123` · orange `255 184 108` · pink `255 121 198` · purple `189 147 249` ·
red `255 85 85` · yellow `241 250 140`. (Source: dracula/dracula-theme.)

**Rules:**
- **Uniques are highlighted by price tier, never blanket-restyled.** `sync` fetches per-unique
  prices from poe.ninja's `stash/current/item/overview` endpoint and records each with its
  filter **base type** (e.g. `Voidforge → Infernal Sword`). `generate` groups those bases into
  `Show # Unique <tier>` blocks (`Rarity == Unique` + `BaseType ==`, deduped per base to the
  highest tier). Price-tiered unique blocks get the tier's text/border/beam **plus a distinct
  bronze background `SetBackgroundColor 175 96 37 240`** so they read as "valuable unique"
  regardless of tier color. Uniques whose base is *not* priced get no rule — they fall through
  to native game style. There is still **no blanket restyle of all `Rarity == Unique`**; only
  curated/priced bases are touched. (Caveat: a `BaseType` match highlights every unique on that
  base, since PoE2 filters cannot target a unique by name.)
- **Unwanted items are hidden, not deleted.** They drop to **tier F**, which uses `Hide` —
  off the screen by default but **revealable by holding Alt** in-game. The `Hide` blocks keep
  their style actions (native rarity color, small font 21, `DisableDropSound`) so an Alt-reveal
  renders them in the de-emphasized look, not the game default. No color repaint, no black bg.
  Tier-F `Hide` currently applies to **equipment** (`gear-hide-remaining`) and the
  **flask/charm progression catch-all** (`flasks-charms-hide-remaining`). Jewellery
  (rings/amulets/belts) is kept `Show` (de-emphasized) so it stays visible — its
  `*-hidden-*` slugs are legacy names, not the current behavior.

**Flasks & charms** share one matched `AreaLevel` ladder (native rarity color throughout):
`≤25` show all rarities · `26–65` show **Magic** only · `>65` show **Quality ≥ 1** only.
Quality flasks/charms always show (they cover the >65 tier); everything else falls to the
`flasks-charms-hide-remaining` `Hide`. The curated `charms-high-ilevel` crafting bases are an
exception shown above the ladder.

**Jewels** (`Class "Jewels"`) are always shown — every rarity, every level — at native rarity
color, font 32, with a cyan diamond minimap icon (`jewels-all`).
- **Recycle** = items with **both `Sockets >= 1` and `Quality >= 1`**. Shown green regardless
  of zone level, placed before the F catch-all so it wins.

## Single-filter AreaLevel progression

`universal.filter` works at every level via an `AreaLevel` ladder (no separate leveling/endgame
files). Blocks ordered above the F catch-all:

All shown gear keeps its **native rarity color**; the column below is what gets *surfaced*
(larger font), not a recolor.

| AreaLevel | Shown (larger font, native color) | Falls to F (hidden, Alt-viewable) |
|---|---|---|
| **≤ 5** | All equipment, any rarity | — |
| **6–20** | My-type Magic + all Rares | off-type Normal/Magic |
| **20–45** | Build gear (my type) + all Rares | off-type Normal/Magic |
| **45+** | Build gear only | off-type Rares too (demoted to F-hidden) |

**"My armour type" tapers with level** (detected via base-defence conditions):
- `≤25`: ES **+** Armour **+** ES/Armour hybrid → `BaseEvasion 0`
- `26–45`: ES **+** ES/Armour hybrid → `BaseEnergyShield > 0` and `BaseEvasion 0`
- `45+`: full ES only → `BaseEnergyShield > 0`, `BaseArmour 0`, `BaseEvasion 0`

Build **weapons** (Staves, Wands, Foci) follow their own ladder: **all rarities ≤25**, then
**Rare only** above 25 (Magic/Normal fall to the gear F-hide). Other caster off-hands
(Sceptres, Talismans) are shown Magic + Rare at all levels. All native rarity color, surfaced
via font size (Rare font 32, Magic/mixed font 30).

> Open verification items (test in-game, tune freely): whether PoE2 honors
> `BaseEnergyShield`/`BaseArmour`/`BaseEvasion` as base-defence filters, and whether the
> recycle `Sockets>=1 & Quality>=1` threshold is too broad.

## Authoring conventions for this repo
- New filters derive from the curated item lists in `reference-old-filters/end_endgame.filter`
  (membership reshuffled per league), restyled into the tier system above.
- **Ignore the trailing config block** that poe2filter.com appended to the old filters
  (`### Filter Configuration` / `# [[Config Start]]` … commented JSON). Do not carry it into
  new filters — `universal.filter` already has it stripped.
- Order blocks **most specific → most general**; tier F is the catch-all.
- Keep block comments descriptive: `Show # What it matches (kebab-slug)`.
- The `uwu.mp3` custom sound lives in `assets/` (version-controlled). For the filter to play
  it in-game, the mp3 must also be in the game folder (it currently is).
- After editing a `.filter`, copy it into the game's config folder (path above) to take effect.
  We will not auto-copy unless asked.
