# CLAUDE.md — FerrisFilter

This repo is **FerrisFilter**: a Rust CLI that compiles a human-editable config into a
Path of Exile 2 loot filter (`generate`) and fetches item prices from poe.ninja (`sync`).

- **Project plan & workflow:** managed with GSD in `.planning/` — see `.planning/PROJECT.md`
  (context), `.planning/ROADMAP.md` (4 phases), `.planning/REQUIREMENTS.md`, and
  `.planning/STATE.md`. Git uses a **milestone branching** workflow (see PROJECT.md → Git Workflow).
- **Changelog:** `CHANGELOG.md` (repo root) tracks notable changes. Keep `[Unreleased]` current
  as work lands, and **always update `CHANGELOG.md` when a milestone is completed** (e.g. via
  `/gsd-complete-milestone`): move the `[Unreleased]` entries into a new dated version section
  with a one-line milestone summary.
- **Domain spec (below):** the filter grammar, the Dracula style-tier system, and the design
  philosophy. `config.json` + `styles.json` are the executable spec the `generate` command compiles. The
  **byte-for-byte acceptance baseline** is now the committed fixture
  `tests/fixtures/seed-universal.filter` (generated from `config.json` + `styles.json` + the seed
  `tests/fixtures/seed-prices.json`); the `artifact_guard` test enforces it. The repo-root
  `universal.filter` / `prices.json` are **generated, git-ignored, and not committed**.
  Everything from "Repository layout" down is that reference — keep it accurate.

## Repository layout

| Path | Purpose |
|------|---------|
| `config.json` | **Source of truth.** Declarative policy model the filter is compiled from. Top-level sections: `build` (weapon/armour classes + defence-type taper), `weaponClasses`/`armourClasses`/`flaskCharmClasses` (complete class lists), `classGroups` (documentation aliases), the ordered `rules` array (the policy — first-match-wins), and `alwaysShow` (currency, recycle). Edit `rules` (and `styles.json` for how a style looks), then regenerate. A `styles` section here is rejected at load unless every entry in it equals the same-named `styles.json` entry (then it is dropped). |
| `styles.json` | **Committed single source of named styles** (repo root): a map of style name to style object (text/border/background colours, font, minimap, beam, sound, `action` Show/Hide, `disableDropSound`). File order is the overlay picker order. `S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; every other style is optional, and adding one needs no code change. Read by `generate`, `apply` and the overlay (beside the inputs / beside the store); `--styles <path>` on `generate` and `apply` overrides it. A config or store that still has an inline `styles` section loads only if every inline entry equals the same-named entry here (an untouched pre-v0.5 seed): the section is then dropped, and the overlay's next Apply saves the store without it. Otherwise it is rejected with a message naming the file and the differing styles; move those here and delete the section. |
| `universal.filter` (repo root) | **Generated** by `ferrisfilter generate` and **git-ignored** — regenerate on demand, do not commit or hand-edit. Each policy block is headed `Show # <rule name>` (from the `name` field in `config.json`). |
| `prices.json` (repo root) | **Generated** by `ferrisfilter sync` from poe.ninja and **git-ignored** — your live economy data, regenerated per sync, never committed. `sync --league <name\|slug>` targets a specific poe.ninja league (useful during a league transition, when the automatic pick resolves to the outgoing league). |
| `dynamic_base.json` (repo root) | **Committed template** for the dynamic filter (v0.4). A full `Config` like `config.json`, but with no `areaLevel` conditions and no rule `name`s — dynamic rules are identified by their **index in the `rules` array**. `ferrisfilter apply` seeds `dynamic.json` from it, and `apply --reset` restores from it to start a filter over without losing the baseline. It has no `styles` section either (styles come from `styles.json`). |
| `dynamic.json` (repo root) | **Generated + hand/overlay-edited working copy** of the dynamic rule set, seeded from `dynamic_base.json` on first `apply` (CLI) or on the overlay's first launch. **Git-ignored** — per-user state. Rules support `"enabled": false`, which keeps a rule in the file but emits nothing into the filter. The overlay editor (below) holds edits in memory; Apply is the single writer that persists this file and then compiles from it — nothing here changes until Apply is pressed, and a **Revert** button discards in-memory edits back to the last applied state. Two cosmetic round-trip quirks are expected and harmless, now surfacing on Apply rather than on every keystroke: an edited rule's `sockets`/`quality`/`areaLevel` objects can grow explicit `"eq": null`/`"min": null` keys (those types lack `skip_serializing_if`), and `classGroups`/`build.armourTypes` — both `HashMap`s — can reshuffle key order between saves. Both are value-equal round-trips on a git-ignored per-user file (asserted by `overlay::store`'s own round-trip test); neither is "fixed" because doing so would touch `config.json`'s own serde attributes and its fixtures. |
| `dynamic.filter` | **Generated** by `ferrisfilter apply` and written into the game folder. **Git-ignored** at the repo root. A complete, playable filter: policy blocks from `dynamic.json` plus the economy sections priced from `prices.json`. PoE2 does not hot-reload — after `apply`, reload via Options → Game → Item Filter → Refresh. |
| `tests/fixtures/seed-prices.json`, `tests/fixtures/seed-universal.filter` | **Committed test fixtures** — the frozen seed prices + its generated filter. These are the test oracle (`include_str!` unit tests + `generate_snapshot`/`artifact_guard`). Regenerate the `.filter` fixture only after a deliberate generator change. |
| `prices-fallback.json` + `tiers-fallback.json` (repo root) | **Committed early-league fallback pair**, always used *together* (their values are calibrated to each other). `generate` loads them instead of `prices.json`/`tiers.json` when `prices.json` is absent/empty (with a warning) or when `--early-game` is passed. Tier membership comes from a poe2filter.com export; cutoffs are **exalt-denominated** (`unit: "exalt"`) so Divine isn't pinned to the A floor. Every base — orbs included — is priced in **exalt at its tier floor** (Divine 20→S, Chaos 2.5→A, Exalt 1→B). No stored rates: `pricer::derive_rates` reads the three anchor orbs (items[0..2]) and expresses them relative to the Chaos Orb (`exalt_in_chaos = value(Exalt)/value(Chaos)` etc.), so quoting the orbs in exalt is enough. The same code on live chaos-quoted data (Chaos = 1c) reproduces the old numbers. The three anchors must share one unit. |
| other `*.filter` (repo root) | Active, version-controlled filters we author/maintain. |
| `reference-old-filters/` | Older/stale filters copied from the game's config folder. **Git-ignored**, kept only as reference. Do not edit these as if they were live; treat them as read-only history. |
| `ferrisfilter apply` | Compiles `dynamic.json` → `dynamic.filter` and writes it straight into the game folder (seeding the store from `dynamic_base.json` when absent). `--store` / `--base` / `--dest` override the paths; `--reset` restores the store from the template first; `--tiers <path>` or `--styles <path>` overrides tier cutoffs while keeping live prices (same flag `generate` accepts). The dynamic counterpart to `generate` + `install` in one step. |
| `ferrisfilter install` | Subcommand that copies `universal.filter` into the in-game config folder (replaces the old `install.bat`). Defaults to `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2`; pass `--dest <dir>` to override and `--input <file>` for a non-default filter. Copy-only — run `generate` first. |
| `ff` (binary, v0.4) | **The GUI overlay** — `cargo build`'s default binary (`default-run = "ff"`). Opens the always-on-top stripe window over the game; Windows GUI subsystem in release builds (no console flash), debug builds keep a console for `eprintln!`/panic visibility. Entry point is `ferrisfilter::overlay::launch()`. As of Phase 14 it is a full editor for `dynamic.json`, plus an Apply button that writes `dynamic.filter` — see the overlay note below. |
| `ff-cli` (binary, v0.4) | The short-name console CLI — identical behavior to `ferrisfilter`, just a shorter name to type. Holds the console shim body `ff` used to have before it became the GUI binary. |
| `ferrisfilter` (binary) | The long-name console CLI, unchanged: `generate` / `sync` / `install` / `apply`. |
| `CLAUDE.md` | This file — the filter-syntax reference so we rarely need to re-fetch the docs. |

> **Overlay (v0.4, `ff`):** Path of Exile 2 must be running in **windowed** or
> **windowed-fullscreen** mode — exclusive fullscreen prevents any external window from drawing
> above it, and `ff` does not detect this, so a blank/missing stripe means checking the game's
> display mode first (see `.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md`). Press
> **`Ctrl+Shift+Z`** (system-wide, hardcoded) to summon or hide the stripe without alt-tabbing —
> **only while Path of Exile 2 or the stripe itself is the foreground window**, so the binding stays
> free for other applications; the
> stripe's own `✕` only **hides** it, and the system tray icon's **Quit** is the only way to exit
> the process.
>
> **The rule editor (v0.4, Phase 14):** each rule in the stripe is a row that **expands in place**
> (an accordion, not a separate edit mode) — clicking it opens its own name/style/class/base-type/
> rarity/defence/sockets/quality/corrupted/unidentified-tier controls inline, with the rows above
> and below still visible, since ordinal position *is* precedence in this model. Up/down arrows
> reorder a rule (arrows, not drag-and-drop); a circular toggle enables/disables it without
> deleting it; **+ ADD RULE** appends a new no-op rule at the bottom; deleting asks for an inline
> confirm. Edits are held in memory as you make them — Apply is the single writer,
> persisting the edits to `dynamic.json` and then compiling `dynamic.filter` from it, in that
> order — and a **Revert** button discards unsaved in-memory edits back to the last applied state
> (the accepted trade-off: unsaved edits do not survive closing the overlay or the process, in
> exchange for a visible, deliberate discard instead of a silent autosave). `dynamic.filter`, the
> file the game actually reads, changes **only** when **Apply** is pressed (via the same
> `dynamic::compile_store`/`write_filter` the CLI's own `apply` subcommand uses), and the stripe
> always shows whether the written filter currently matches the in-memory rules — a stale
> "unapplied changes" state is expected the moment you edit anything, and clears once Apply
> succeeds. Because PoE2 never re-reads a filter on its own, the stripe also repeats the reload
> path after every successful Apply: **Options → Game → Item Filter → Refresh**.
>
> **Rule name, class off switch, and base-type list (v0.4, Phase 15):** a rule can carry an
> optional **name**, entered via its own circular toggle at the top of the expanded body; a named
> rule's row header reads `<index>  <style> - <name>` and the name becomes the emitted filter
> block's own comment (`Show # <name>`), while an unnamed rule keeps the generator's derived
> `rule <index>` label — there is no auto-fill, a name is empty until typed. **Classes** gained
> their own off switch alongside every other condition — and the picker's **JEWELLERY** group
> carries `Jewels` alongside Rings/Amulets/Belts (added 2026-09-16). A jewel is not jewellery, but
> it is the only class that would ever sit in a group of its own, and a one-item header would spend
> a whole collapsible section on one checkbox. The groups are picker convenience only: they come
> from `config.json`'s `jewelleryClasses` (PITFALLS §10 — never hardcoded in the GUI), the emitter
> never reads them, and listing a class there writes no rule anywhere. The off switch itself: switching it off hides the class picker
> entirely (not greyed out) and discards the selection, which is what makes a base-type-only rule
> — matching on `BaseType` with no `Class` line at all — expressible in the overlay. **Base types**
> are entered one at a time directly after the class control: type a name and press Enter or
> **+ ADD** to commit it to the list, and each listed entry has its own `Remove`; a quoted list
> pasted straight out of a hand-written filter (`"Heavy Belt" "Utility Belt"`) splits into separate
> entries. Base names are **not validated** against anything — PoE2 itself warns on an unknown
> base when the filter loads, which is a better and more current oracle than any list this repo
> could keep up to date across leagues.

> **Economy sync from the overlay (v0.4):** a **SYNC ECONOMY** button beside Apply refreshes
> `prices.json` in the store's directory, on a worker thread so the stripe never freezes on network
> I/O. It writes prices and nothing else — `dynamic.filter` still changes only on **Apply**, so the
> button is safe to press mid-map and the new economy lands on the next Apply you choose to make.
> The GUI exposes no `--standard`/`--hc`/`--league`/`--tiers` switches: it syncs with the tiers file
> already in place, and **the league is pinned** in `sync::OVERLAY_LEAGUE` — currently
> `"Forbidden Rites"`. **Update that constant at the start of each league.** It is pinned because
> poe.ninja's `indexed` flag does not reliably identify the current league: it was observed
> flipping between Forbidden Rites and Runes of Aldur within six hours on 2026-09-10/11, and a sync
> caught on the wrong side writes last league's economy into `prices.json` while the resulting
> filter looks completely normal. The success line names the league it priced against, so a stale
> pin is visible rather than silent, and a pin that no longer matches any league fails loudly with
> the list of available ones. The CLI keeps automatic resolution and its `--league` override.
>
> **Design philosophy (current):** Hide what we don't want, style the few highlights, and let
> everything else fall through to the game default. Uniques, jewels, gems, quest items,
> waystones, and trials have **no rules** — they show natively. Jewellery, unidentified
> high-tier items, and equipment are covered by explicit rules in the `config.json` `rules`
> array. `config.json` is the authoritative model — the reconciliation of the old
> hardcoded-band description with the actual code is complete as of v0.2.

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
- `UnidentifiedItemTier` — confirmed PoE2 (author-tested, v0.2): highlights high-tier
  unidentified items (e.g. `UnidentifiedItemTier >= 4`). Used by the unidentified rules, which
  wear the `kite` style. Note the condition and the style are independent: the condition is what
  matches, `kite` is only how a match is drawn.
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
- **Unwanted items are hidden, not deleted.** The `hidden` style uses `Hide` — off the screen
  by default but **revealable by holding Alt** in-game. Hidden blocks keep no color overrides
  (native rarity color), font 21, and `DisableDropSound`, so an Alt-reveal renders them
  de-emphasized, not in the game default. The `hidden` style currently applies to equipment
  (gear fallback rule), jewellery (rings/amulets/belts fallback rule), and the flask/charm
  fallback rule.

**Flasks & charms** have three show rules (native rarity color throughout): quality `≥1` always
show (highest priority); `AreaLevel ≤25` show all rarities; `AreaLevel ≤65` show `Magic` or
better (`>=Magic`). Deep endgame (AreaLevel ≥78) surfaces Normal-rarity charms. Everything else
falls to the `hidden` flask/charm fallback rule.

**Jewels** (`Class "Jewels"`) have **no rule in `config.json`** — they fall through to the game
default (GEN-04), which is what the emitter's `generate_no_unique_or_jewels_blocks` test holds.
The older "always shown, font 32, cyan diamond (`jewels-all`)" description came from the
hand-authored poe2filter.com filters in `reference-old-filters/` and has not described
`universal.filter` since v0.2. The class *is* offered in the overlay's class picker (below), so a
dynamic rule can target it — that is the editor, not the generated baseline.
- **Recycle** = items with **both `Sockets >= 1` and `Quality >= 1`**. Shown green regardless
  of zone level; the Recycle rule is first in the rules array so it wins before any hide rule.

## Declarative rule model

`universal.filter` is generated by compiling the `rules` array in `config.json`. This is the
authoritative description of how the filter is produced (v0.2+). There is no separate
leveling/endgame file — a single generated `universal.filter` handles all area levels because
the rules themselves carry `areaLevel` conditions.

### Rules array

`rules` is an **ordered array** of rule objects; the first matching rule wins
(first-match-wins, identical to how the filter engine evaluates blocks). Each rule compiles to
exactly one `Show` or `Hide` block — the disposition is determined by the `action` field of the
referenced style. A rule with no `when` is a catch-all that matches everything.

```json
{ "name": "band <=5: all armour any rarity", "style": "unchanged",
  "when": { "areaLevel": { "max": 5 }, "class": ["Body Armours", "Helmets", ...] } }
```

### `when` condition vocabulary

All fields are optional. All present conditions must match (AND semantics within a rule).

| Field | JSON type | Emitted filter condition |
|---|---|---|
| `class` | `["ClassName", ...]` | `Class "ClassName" "ClassName" ...` (bare, match-any) |
| `rarity` | compact string | `Rarity == Rare` / `Rarity >= Magic` / `Rarity <= Rare` (bare value = `==`) |
| `areaLevel` | `{ "min": N, "max": N }` | `AreaLevel >= N` and/or `AreaLevel <= N` |
| `baseArmour` | `{ "eq": N }` / `{ "min": N }` | `BaseArmour N` / `BaseArmour >= N` |
| `baseEvasion` | `{ "eq": N }` / `{ "min": N }` | `BaseEvasion N` / `BaseEvasion >= N` |
| `baseEnergyShield` | `{ "eq": N }` / `{ "min": N }` | `BaseEnergyShield N` / `BaseEnergyShield >= N` |
| `quality` | `{ "min": N }` | `Quality >= N` |
| `sockets` | `{ "min": N }` | `Sockets >= N` |
| `baseType` | `["Base Name", ...]` | `BaseType == "Base Name" "Base Name" ...` |
| `unidentifiedTier` | `{ "min": N }` | `UnidentifiedItemTier >= N` |

Note: `min: 1` on a defence attribute emits `> 0` (e.g. `BaseEnergyShield > 0`), matching the
oracle filter syntax. The `max` field on `NumericCondition` emits `<= max` (available for
future use).

> Confirmed working in-game (author-tested, v0.2): PoE2 honors
> `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` as base-defence filters and
> `UnidentifiedItemTier` as a threshold condition. (The recycle `Sockets>=1 & Quality>=1`
> threshold remains a tuning knob, not a correctness question.)

### Named styles

`styles.json` defines all named styles; the overlay's style picker lists them in file order.
`S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; any other style is
optional. An unknown style in an enabled rule, a missing required style, a duplicate name, a name
differing only by case from a required one, or a bad field (unknown colour, unsafe custom sound
name) is a load-time error naming the problem. A style's `action` decides Show/Hide for the rule
using it, and Hide styles may set any visual (shown on Alt-reveal). Current styles:

| Name | Action | What it does |
|---|---|---|
| `S` | Show | Pink text/border, Dracula bg, font 37, Pink beam, Pink circle minimap, custom sound uwu.mp3 |
| `A` | Show | Purple text/border, Dracula bg, font 37, Purple beam, Purple circle minimap, custom sound uwu.mp3 |
| `B` | Show | Red text/border, Dracula bg, font 37, Red beam, Red circle minimap, alert sound |
| `C` | Show | Yellow text/border, Dracula bg, no font override, Yellow beam, Yellow circle minimap, alert sound |
| `D` | Show | Cyan text/border, Dracula bg, no font override, White beam, White circle minimap |
| `E` | Show | White text/border, Dracula bg, font 29 |
| `F` | Show | White text/border, Dracula bg, font 25 (de-emphasized; used as fallback before a `hidden` rule) |
| `Recycle` | Show | Green text/border, Dracula bg, Green beam, Green cross minimap |
| `unchanged` | Show | No color/font/minimap/beam overrides — native rarity color and game defaults |
| `hidden` | Hide | No color overrides, font 21, DisableDropSound — Alt-reveals as de-emphasized |
| `kite` | Show | No color overrides (native rarity), Orange beam, Orange Kite minimap, alert sound. A **locator** style, not a value tier — it says "go look over there" without claiming worth, which is why its orange sits outside the Dracula ladder. Renamed from `unidentified` on 2026-09-07: the old name described the rules that happened to use it rather than what it does, so seeing it offered for an unrelated rule in the overlay picker read as nonsense. |
| `exceptional` | Show | Pink text/border, Dracula bg, no font override, Pink beam, Pink Cross minimap, no sound — the exceptional-gear highlight, kept separate from `S` so it can be retuned independently |

### `classGroups`

`classGroups` is a documentation-only alias map (e.g. `"buildWeapons": ["Staves","Wands","Foci"]`).
It is **not consumed by the compiler** — rules inline their full class lists in `when.class`.
Use `classGroups` as a reference when authoring new rules to keep class lists consistent.

### Fallthrough

Any item not matched by a rule is left to the game default (uniques, jewels, gems, quest
items, waystones, trials, etc.). See the `"fallthrough"` field in `config.json` for the note.

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
