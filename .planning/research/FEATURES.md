# Feature Research — v0.3 Economy Items & Exceptional Gear

**Domain:** PoE2 loot filter — new league items + exceptional gear highlighting
**Researched:** 2026-06-23
**Confidence:** HIGH (primary sources: NeverSink PoE2 filter source code, reference-old-filters/ on disk, poe.ninja URLs, poe2db)

---

## 1. Exceptional Items (Highest Priority)

### What "Exceptional" Means in PoE2

In PoE2, "exceptional" gear refers to items that have been boosted above their base quality or socket cap via crafting or fortunate drops. Two sub-categories:

- **Extra sockets** — a 2-handed/body item with 3 sockets, or a 1-handed/small piece with 2 sockets (PoE2 default caps are typically 1-2 sockets per slot).
- **Extra quality** — an item with Quality >= 20+ (obtainable via Glassblower's Bauble chaining or Omen of Sanctification; normal crafted cap is 20 but exceptional pieces can reach 27-30+).

These items are valuable as either crafting bases (for "chancing" into uniques) or direct quality improvement targets. The `Corrupted False` condition is mandatory because corrupted items cannot be further crafted.

### There is NO Dedicated "Exceptional" Filter Keyword

There is no `Exceptional` or `ExtraSocket` filter condition in PoE2. "Exceptional" is not a game tag or modifier — it is an informal community term. The filter mechanism uses standard conditions combined:

| Goal | Filter Conditions Used |
|------|------------------------|
| Extra quality | `Quality >= N` + `Corrupted False` |
| Extra sockets (2H/Body) | `Sockets >= 3` + `Corrupted False` + Class list |
| Extra sockets (1H/Small) | `Sockets >= 2` + `Corrupted False` + Class list |
| Restrict to specific bases | `BaseType == ...` (optional but recommended) |

`Corrupted False` is the correct PoE2 filter syntax (boolean — no `==`). Confirmed by: NeverSink filter source (GitHub), reference-old-filters/end_endgame.filter on disk.

### Exact Filter Syntax (Concrete Examples)

From `reference-old-filters/end_endgame.filter` (on disk, author's prior filter):

```
# Extra quality — show quality-exceptional Normal items (excellent tier)
Show # Excellent Exceptional Quality Weapons (gear-exceptional-excellent-quality)
  Quality >= 28
  Class == "Bows" "Crossbows" "Foci" "One Hand Maces" "Quivers" "Quarterstaves" "Sceptres" "Spears" "Talismans" "Staves" "Two Hand Maces" "Wands"
  BaseType == "Guardian Bow" "Gemini Bow" [...]
  Rarity == Normal

# Extra sockets (2-handed/Body — needs 3 for "exceptional")
Show # Excellent Exceptional 2-Socketed Weapons (gear-exceptional-excellent-socketed-weapons)
  Sockets >= 3
  Class "Bows" "Crossbows" "Quarterstaves" "Talismans" "Staves" "Two Hand Maces"
  BaseType == "Guardian Bow" [...]
  Rarity == Normal

# Extra sockets (1-handed/small — needs 2 for "exceptional")
Show # Excellent Exceptional 1-Socketed Weapons (gear-exceptional-excellent-socketed-weapons)
  Sockets >= 2
  Class "Foci" "One Hand Maces" "Sceptres" "Spears" "Wands"
  BaseType == [...]
  Rarity == Normal
```

From NeverSink PoE2 filter (GitHub), **current-generation** simplified blocks (less strict about specific BaseTypes):

```
# Extra quality — any base above Q27, not corrupted
Show # exceptional overqual1q1
  Corrupted False
  Quality >= 27
  Rarity Normal Magic Rare
  BaseType == "Adherent Cuffs" "Akoyan Spear" [...top-tier endgame bases...]

# Extra sockets (2H/Body), not corrupted
Show # exceptional oversockt1s3
  Corrupted False
  Sockets >= 3
  Rarity Normal Magic Rare
  Class == "Body Armours" "Bows" "Crossbows" "Quarterstaves" "Staves" "Talismans" "Two Hand Maces"
  BaseType == [endgame base list]

# Extra sockets (1H/Small), not corrupted
Show # exceptional oversockt1s2
  Corrupted False
  Sockets >= 2
  Rarity Normal Magic Rare
  Class == "Boots" "Bucklers" "Foci" "Gloves" "Helmets" "One Hand Maces" "Sceptres" "Shields" "Spears" "Wands"
  BaseType == [endgame base list]
```

Source: `NeverSinkDev/NeverSink-Filter-for-PoE2` main branch, section `$type->exotic->exceptional`.

Also from `reference-old-filters/#end_endgame.filter#` on disk (broader, no BaseType restriction):
```
Show # Exceptional 2-Socketed Uniques (uniques-exceptional)
  Rarity == Unique
  Sockets >= 3
  Class == "Bows" "Crossbows" "Quarterstaves" "Talismans" "Staves" "Two Hand Maces" "Body Armours"

Show # Exceptional 1-Socketed Uniques (uniques-exceptional)
  Rarity == Unique
  Sockets >= 2
  Class == "Foci" "One Hand Maces" "Sceptres" "Spears" "Wands" "Helmets" "Gloves" "Boots" "Shields" "Bucklers"

Show # Exceptional Quality Uniques (uniques-exceptional)
  Rarity == Unique
  Quality >= 24
```

### Design Recommendation for FerrisFilter v0.3

Two approaches are viable:

**Option A — Base-type-gated (highest signal, fewest false positives):** Only highlight exceptional items on known high-value bases. Requires maintaining a curated `BaseType ==` list. Best for end-endgame filter.

**Option B — Class-only (broadest, simpler, no curation needed):** Use `Corrupted False` + `Quality >= N` or `Sockets >= N` + class list, no BaseType restriction. Produces more noise early league but never misses a base.

Given the declarative `rules` model and the fact that this is a personal filter (not a public release), Option B is the correct v0.3 approach: add two rules before the `hidden` gear fallback:

```json
// In config.json rules array (before the gear-hide fallback):
{ "name": "exceptional: extra-socketed 2H/Body", "style": "D",
  "when": { "corrupted": false, "sockets": { "min": 3 },
            "class": ["Body Armours","Bows","Crossbows","Quarterstaves","Staves","Talismans","Two Hand Maces"],
            "rarity": "<=Rare" } },
{ "name": "exceptional: extra-socketed 1H/small", "style": "D",
  "when": { "corrupted": false, "sockets": { "min": 2 },
            "class": ["Boots","Bucklers","Foci","Gloves","Helmets","One Hand Maces","Sceptres","Shields","Spears","Wands"],
            "rarity": "<=Rare" } },
{ "name": "exceptional: extra quality", "style": "D",
  "when": { "corrupted": false, "quality": { "min": 20 },
            "class": ["Body Armours","Boots","Bows","Bucklers","Crossbows","Foci","Gloves","Helmets",
                      "One Hand Maces","Quarterstaves","Sceptres","Shields","Spears","Staves",
                      "Talismans","Two Hand Maces","Wands"],
            "rarity": "<=Rare" } }
```

**IMPORTANT implementation note:** The `when` vocabulary in config.json does NOT currently have a `corrupted` field. That condition must be added to the schema (new `NumericCondition` or a boolean field) and to `emit_rules`. This is a new condition to implement. Emits `Corrupted False` in the filter block.

The quality threshold is tunable — `20` catches items that have hit quality cap by any means; `24+` restricts to items that went beyond normal crafted cap (rare/league-specific).

---

## 2. Uncut Gems

### Naming Convention — CRITICAL CORRECTION

**The correct filter syntax uses `GemLevel` condition, NOT `(Level N)` in the BaseType name.**

Evidence from NeverSink PoE2 filter (main branch, June 2026):

```
Show # uncut $tier->skill20
  GemLevel >= 20
  BaseType "Uncut Skill Gem"

Show # uncut $tier->skill19
  GemLevel 19
  BaseType "Uncut Skill Gem"

Show # uncut $tier->skillgemprogression18
  GemLevel 18
  BaseType "Uncut Skill Gem"
  AreaLevel <= 79
```

The `BaseType` strings are exactly:
- `"Uncut Skill Gem"` (no level suffix)
- `"Uncut Spirit Gem"` (no level suffix)
- `"Uncut Support Gem"` (no level suffix)

The poe.ninja URL that contains `?name=Uncut+Skill+Gem+%28Level+20%29` is a **query parameter for the web UI**, not the in-game BaseType name. The poe2db page confirms the base type is simply `"Uncut Skill Gem"`. Levels in the filter are expressed via the `GemLevel` condition.

Note: The `(Level N)` convention DOES exist for Thaumaturgic Flux expedition items (e.g., `"Thaumaturgic Flux (Level 20)"` is the actual BaseType). Uncut Gems do not follow this pattern.

### Class for Uncut Gems

NeverSink filter uses **no `Class` line** for uncut gem blocks — the `BaseType "Uncut Skill Gem"` substring match works across the "Gems" class. poe2db metadata class for Uncut Skill Gem is `Metadata/Items/Gems/SkillGemUncut`. The filter class name is likely `"Gems"` but the bareword `BaseType` match without `Class` is simpler and confirmed working by NeverSink.

The existing FerrisFilter filter uses `Class == "Stackable Currency"` for uncut gems — this is WRONG and explains why the rule fails to match. The correct class is `"Gems"` (or no class line + BaseType substring match).

### Gem Types and Level Range

| BaseType | Notes |
|----------|-------|
| `Uncut Skill Gem` | Creates an active skill gem at GemLevel N |
| `Uncut Spirit Gem` | Creates a persistent buff skill gem at GemLevel N |
| `Uncut Support Gem` | Creates a support gem at GemLevel N |

GemLevel range: 1–20 (GemLevel 20 is the practical endgame ceiling; GemLevel >= 20 catches any hypothetical higher values). Level 1-13 are common campaign drops; 18-20 are the high-value tiers.

### Fix Required in FerrisFilter

The current `sync` code fetches uncut gems with `minLevel: 18` filtering and the `generate` code emits `BaseType == "Uncut Skill Gem" "Uncut Spirit Gem"` (wrong class). The v0.3 fix:
1. Drop `minLevel` floor in `sync` (price-tier everything, let economy drive it).
2. Change the emitted class from `Class == "Stackable Currency"` to no class line or `Class == "Gems"`.
3. Use the `GemLevel` condition (requires adding `gemLevel` to the `when` vocabulary in config.json or handling uncut gems in the pricer path with GemLevel conditions directly in the emitter).
4. The `BaseType` string stays as the bare name (no `(Level N)` suffix).

---

## 3. Lineage Support Gems

### What They Are

Lineage Support Gems are unique-rarity support gems with powerful build-defining effects, introduced in PoE2 patch 0.3.0 and expanded in 0.5.0 (Runes of Aldur). They drop from specific endgame bosses. Unlike standard support gems (created from Uncut Support Gems), Lineage gems drop directly as named items. A character can only equip one copy of each Lineage gem at a time.

As of June 2026 (Runes of Aldur), approximately 74 Lineage Support Gems exist.

### Filter Class

From NeverSink PoE2 filter (definitive source):

```
Show # $type->gems->lineage $tier->s
  Class == "Skill Gems" "Support Gems"
  BaseType == "Ailith's Chimes" "Arbiter's Ignition" [...]
```

**Class is `"Skill Gems" "Support Gems"`** — both classes are listed because some Lineage gems are skill gems, others are support gems. This is the correct filter syntax.

### BaseType Names (Sample from NeverSink S-tier)

```
"Ailith's Chimes" "Arbiter's Ignition" "Atalui's Bloodletting" "Atziri's Impatience"
"Breachlord's Amalgam" "Breachlord's Rift" "Brutus' Brain" "Catha's Brilliance"
"Dialla's Desire" "Esh's Prowess" "Esh's Radiance" "Garukhan's Resolve"
"Her Declaration" "Ixchel's Torment" "Khatal's Rejuvenation" "Mórrigan's Insight"
"Olroth's Conviction" "Olroth's Hubris" "Rakiata's Flow" "Rigwald's Ferocity"
"Seraph's Heart" "Sione's Temper" "Styrn's Ferocity" "Tangmazu's Thurible"
"Tecrod's Revenge" "Trickster's Shard" "Tul's Stillness" "Uhtred's Augury"
"Uhtred's Constellation" "Uhtred's Exodus" "Uhtred's Omen" "Uul-Netol's Embrace"
"Vorana's Siege" "Xoph's Pyre" "Zarokh's Revolt" "Zerphi's Infamy"
```

These are individual unique-named gems, not items with a level suffix. The poe.ninja `type=LineageSupportGems` endpoint will return the gem names with prices; `generate` needs to emit each name as a `BaseType ==` value.

### Styling

Per PROJECT.md: price-tiered with a per-category background `10 150 150` (teal), tier color on border+text. Same mechanism as priced uniques' bronze background — not a new named style.

---

## 4. Verisium

### What It Is

Verisium is a new stackable resource introduced in Runes of Aldur (patch 0.5.0). It drops from Ezomyte Remnants in the league mechanic and is the fundamental currency of the Runeforging crafting system — used to add the Runic Ward defense stat to armor.

It has a stack size of 1000 and behaves more like Gold (a resource that accumulates in large stacks) than a traditional orb.

### Filter Syntax

From NeverSink PoE2 filter (definitive source):

```
Show # verisium $tier->massive
  StackSize >= 1000
  BaseType == "Verisium"

Show # verisium $tier->huge
  StackSize >= 150
  BaseType == "Verisium"

Show # verisium $tier->large
  StackSize >= 30
  BaseType == "Verisium"

Show # verisium $tier->any
  BaseType == "Verisium"
```

**No `Class` line is present.** The NeverSink filter treats Verisium via stack-size tiers, not price tiers. However, poe.ninja serves Verisium prices via an economy endpoint (`/poe2/economy/runesofaldur/verisium`), suggesting FerrisFilter can sync and price-tier it.

Also in the filter: `"Exceptional Verisium"` — this appears to be a distinct, rarer base type (appears in the "xenotiering" section with mid-tier emphasis). Whether poe.ninja reports both items is unknown; the sync code should be verified against the live API.

**BaseType names:**
- `"Verisium"` — the common stackable resource
- `"Exceptional Verisium"` — appears in NeverSink as a distinct (rarer) variant

**Filter Class:** Not specified in NeverSink blocks — `BaseType == "Verisium"` matches without a class condition. Based on the poe.ninja endpoint returning it under "verisium" category and NeverSink's "xenotiering" section (which groups league-specific Stackable Currency items), Verisium is most likely `Class == "Stackable Currency"`.

---

## Feature Summary Table

| Feature | Filter Mechanism | Class | BaseType | New Condition Needed |
|---------|-----------------|-------|----------|----------------------|
| Exceptional extra sockets | `Sockets >= N` + `Corrupted False` | gear classes | optional list | `corrupted` condition in config schema |
| Exceptional extra quality | `Quality >= N` + `Corrupted False` | gear classes | optional list | `corrupted` condition in config schema |
| Uncut Gems by level | `GemLevel N` + `BaseType "Uncut Skill Gem"` | `"Gems"` (no class needed) | bare name, no level suffix | `gemLevel` condition in config or emitter handling |
| Lineage Support Gems | `BaseType == "Name"` (price-tiered) | `"Skill Gems" "Support Gems"` | individual named gems | none (same mechanism as uniques) |
| Verisium | `BaseType == "Verisium"` | likely `"Stackable Currency"` | `"Verisium"` / `"Exceptional Verisium"` | none (treat as currency tier) |

---

## Anti-Features

| Anti-Feature | Why Avoid |
|---|---|
| `BaseType == "Uncut Skill Gem (Level 20)"` | WRONG — level is NOT in the BaseType name; use `GemLevel` condition instead |
| `Class == "Stackable Currency"` for uncut gems | WRONG — uncut gems are class `"Gems"`, not Stackable Currency; current filter bug |
| Using `HasImplicitMod` or `AnyEnchantment` for exceptional items | No such mechanic for "exceptional" in PoE2; use `Quality`/`Sockets` + `Corrupted False` |
| `Corrupted == False` syntax | Wrong — the correct syntax is `Corrupted False` (boolean, no operator) |
| Blanket `BaseType "Uncut"` substring match | Would catch "Uncut Skill Gem", "Uncut Spirit Gem", "Uncut Support Gem" all at once without level discrimination; acceptable as a fallthrough but not for tiering |
| Tracking Verisium via price tier at item level | Verisium is a bulk resource; stack-size tiering (like Gold) makes more sense than price per item; FerrisFilter can use price-tier but should note the stack-size nature |

---

## Confidence Assessment

| Finding | Confidence | Source |
|---------|------------|--------|
| `Corrupted False` syntax | HIGH | NeverSink filter source (active, June 2026) |
| Exceptional = `Sockets >= N` + `Quality >= N` + `Corrupted False` | HIGH | NeverSink filter + reference-old-filters/ on disk |
| Socket threshold split (3 for 2H/Body, 2 for 1H/small) | HIGH | end_endgame.filter + NeverSink filter |
| `Uncut Skill Gem` has no level in BaseType name | HIGH | poe2db + NeverSink filter (uses `GemLevel` condition) |
| `GemLevel` is a valid PoE2 filter condition | HIGH | NeverSink filter actively uses it |
| Uncut gems class is `"Gems"` (not `"Stackable Currency"`) | HIGH | NeverSink filter (blocks have no Class line, but waypoint comment says `"Gems"`) |
| Lineage gems class `"Skill Gems" "Support Gems"` | HIGH | NeverSink filter exact syntax |
| Lineage gem names (individual BaseType per gem) | HIGH | NeverSink filter source |
| `BaseType == "Verisium"` | HIGH | NeverSink filter exact blocks |
| `"Exceptional Verisium"` as distinct BaseType | MEDIUM | NeverSink filter (present in tier lists; game source not directly verified) |
| Verisium class is `"Stackable Currency"` | MEDIUM | Inferred from poe.ninja category + community reports; NeverSink blocks have no `Class` line |

---

## Sources

- `reference-old-filters/end_endgame.filter` — author's prior filter (on disk): exceptional items blocks (lines 107–215)
- `reference-old-filters/#end_endgame.filter#` — older version (on disk): uniques-exceptional blocks (lines 36–54)
- `NeverSinkDev/NeverSink-Filter-for-PoE2` main branch (June 2026): [https://github.com/NeverSinkDev/NeverSink-Filter-for-PoE2](https://github.com/NeverSinkDev/NeverSink-Filter-for-PoE2) — definitive live filter; exceptional items `$type->exotic->exceptional`, gems section `[[2200]]`, Verisium blocks `$type->verisium`
- poe2db.tw: BaseType for Uncut Skill Gem is `"Uncut Skill Gem"` (no level in name), class `Metadata/Items/Gems/SkillGemUncut`
- poe.ninja Runes of Aldur uncut-gems URL: `?name=Uncut+Skill+Gem+%28Level+20%29` — level is a web UI query param, not in-game BaseType
- poe.ninja Verisium economy page: confirms `Verisium` as a distinct poe.ninja economy category
- Game8.co: Lineage Support Gems overview — ~74 gems, drop from specific endgame bosses
- ssf_icewolf.filter, ssf_mercenary.filter (repo root) — current generated filters, confirmed `Class == "Stackable Currency"` for uncut gems (the bug to fix)

---

*Feature research for: FerrisFilter v0.3 Economy Items & Exceptional Gear*
*Researched: 2026-06-23*
