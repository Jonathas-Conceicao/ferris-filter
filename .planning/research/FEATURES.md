# Feature Landscape

**Domain:** Generic PoE2 loot-filter editor embedded in an existing overlay (FerrisFilter v0.5)
**Researched:** 2026-10-12
**Overall confidence:** MEDIUM-HIGH (grammar vocabulary HIGH: official page + real samples; editor-UX comparisons LOW-MEDIUM: FilterBlade/poe2filter.com sites were not fetchable, 403/no results)

## A. Evidence base: the PoE2 grammar as it really exists

### A1. Official vocabulary (pathofexile.com/item-filter/about, fetched 2026-10-12, HIGH)

The official page is one document for both games and **does tag PoE2 additions** (`AlwaysShow`, `IsVaalUnique`, `HasVaalUniqueMod`, `TwiceCorrupted`, `UnidentifiedItemTier`, `WaystoneTier`). No action is PoE2-tagged. `Import` (with `Optional`), `Continue`, `Minimal` are listed as general. The parser must therefore accept the **whole** vocabulary, not a PoE2 subset: a hand-written filter may use any of it and the game ignores/warns on what does not apply.

| Value kind | Keywords |
|---|---|
| Boolean (`True`/`False`, no operator) | AlternateQuality, **AlwaysShow**, AnyEnchantment, BlightedMap, Corrupted, ElderItem, ElderMap, Exceptional, Foulborn, FracturedItem, HasCruciblePassiveTree, HasImplicitMod, **HasVaalUniqueMod**, Identified, Imbued, **IsVaalUnique**, MirageMap, Mirrored, Replica, Scourged, ShapedMap, ShaperItem, SynthesisedItem, TransfiguredGem (also takes a gem name), **TwiceCorrupted**, UberBlightedMap, Vestigial, ZanaMemory |
| Numeric (operator + int) | ArchnemesisMod (name), AreaLevel, BaseArmour, BaseDefencePercentile, BaseEnergyShield, BaseEvasion, BaseWard, CorruptedMods, DropLevel, EnchantmentPassiveNum, GemLevel, Height, ItemLevel, LinkedSockets, MapTier, MemoryStrands, Quality, StackSize, **UnidentifiedItemTier**, **WaystoneTier**, Width |
| Name / list | BaseType, Class, EnchantmentPassiveNode, GemQualityType, HasEaterOfWorldsImplicit, HasEnchantment, HasExplicitMod, HasInfluence, HasSearingExarchImplicit, Rarity (ordered Normal<Magic<Rare<Unique), SocketGroup, Sockets (number + optional colour letters, e.g. `5GGG`) |
| Actions | PlayAlertSound, PlayAlertSoundPositional (id 1-16 or `None`, vol 0-300), CustomAlertSound, CustomAlertSoundOptional (semicolon-separated files = random pick), DisableDropSound, EnableDropSound, DisableDropSoundIfAlertSound, EnableDropSoundIfAlertSound, MinimapIcon (size 0-2 or -1, colour, shape), PlayEffect (colour [Temp] or `None`), SetBackgroundColor, SetBorderColor, SetTextColor (RGB[A]), SetFontSize (1-45) |
| Operators | `=`, `==`, `!`, `!=`, `<`, `<=`, `>`, `>=`; Rarity uses ordering; bare value with no operator is legal for every condition |

Caveat: `HasExplicitMod` takes `>=N "mod" "mod"` with **no space** between operator and count in real files (`HasExplicitMod >=1 "Hellion's"`), per the PoE1 patch note and the local sample. `Sockets`/`SocketGroup` values are not plain ints.

### A2. What the real samples actually use (grep of local files, HIGH)

**filterblade.filter** (NeverSink 0.10.4, SOFT/DEFAULT, 5312 lines, 258 KB, **CRLF line endings**, tab indent, 447 Show / 13 Hide / 40 `Continue`, 0 `Import`, 0 `Minimal`):

- Conditions seen: Class, Rarity, BaseType, AreaLevel, ItemLevel, Corrupted, UnidentifiedItemTier, WaystoneTier, Mirrored, Quality, Sockets, StackSize, GemLevel, Width, Height, BaseEvasion/BaseEnergyShield/BaseArmour, HasExplicitMod, AnyEnchantment, Identified, TwiceCorrupted, IsVaalUnique, HasVaalUniqueMod, **AlwaysShow**.
- Actions seen: SetFontSize, SetBorderColor, SetBackgroundColor, SetTextColor, PlayEffect, MinimapIcon, PlayAlertSound, DisableDropSound. (No CustomAlertSound in this variant; the CUSTOMSOUNDS style folder swaps it in.)
- Operator spread: `==` on Class/BaseType; `>=` / `<=` / `<` / `>` on numerics; **bare (no operator)** on Rarity (`Rarity Normal Magic Rare` = multi-value list), BaseType (55x, substring semantics), Corrupted, Mirrored, Identified, WaystoneTier (15), GemLevel (12), `Class` once; `Sockets >` (strict) and `Quality >` appear. Rarity with multiple values and no operator is common.
- `Continue` is used for decorator blocks (e.g. `Rarity Rare` + `AreaLevel >= 65` + `SetBorderColor` + `Continue`) -- ordering of Continue blocks relative to later blocks is semantically load-bearing.
- **Commented-out blocks are common** (`#Show ...` followed by `#\tCondition` lines) and must round-trip as comments, not be mis-parsed as blocks.
- Block headers carry machine tags: `Show # %D4 $type->currency->leveling $tier->jewellers !currency_c` -- `%D4`/`%H8` (strictness/visibility directive), `$type->a->b`, `$tier->x`, `!slug`. 92 distinct `!slug` values, 64 `# !! Waypoint` markers, and a table of contents using `# [[0100]] Name` section banners built from `#====` rulers. These are a **free structure index**: sections (banner), group (`$type`), tier (`$tier`).
- NeverSink repo README (fetched): 7 strictness files (`0-SOFT` .. `6-UBER-PLUS-STRICT`) and 7 style folders (AURA, COBALT, CUSTOMSOUNDS, DARKMODE, MYTHIC, VAAL, ZEN). No documentation of `$type`/`$tier` tags in the README; treat as convention observed in the file, not a spec.

**reference-old-filters/** (poe2filter.com exports + authored): Show/Hide/Continue(1), Class, BaseType, Rarity, HasExplicitMod (189), Identified, Corrupted, Sockets, Quality, ItemLevel, UnidentifiedItemTier, DropLevel, BaseEvasion/BaseEnergyShield, Height/Width, StackSize, IsVaalUnique, HasVaalUniqueMod, plus **CustomAlertSound** (27) and PlayAlertSound. Header-comment slug convention `Show # Description (slug)`. Trailing `### Filter Configuration` / `[[Config Start]]` commented JSON block (poe2filter.com) -- must be preserved as opaque trailing comments in generic mode.

Currently modelled by the overlay editor (v0.4): class, baseType, rarity, areaLevel (dynamic base has none), base defences, sockets, quality, corrupted, unidentifiedTier. **Gap versus real filters:** AlwaysShow, ItemLevel, DropLevel, WaystoneTier, Mirrored, Identified, TwiceCorrupted, GemLevel, StackSize, Width/Height, HasExplicitMod, AnyEnchantment, IsVaalUnique/HasVaalUniqueMod; multi-operator support (`!=`, `<`, `>`, `==` vs bare); all unmodelled actions (CustomAlertSound*, PlayAlertSoundPositional, Enable/DisableDropSound*, `None` forms, MinimapIcon size -1).

## B. How existing editors handle arbitrary filters (MEDIUM/LOW: only repo READMEs retrievable)

| Tool | Model | Takeaway |
|---|---|---|
| **Filtration** (PoE1 desktop, archived 2023; github.com/ben-wallis/Filtration) | Parses a script into a block list with sections; per-block condition/action editors with live colour preview; "Block Group Browser" to toggle Show/Hide for groups of blocks; copy/paste blocks between scripts; Theme Editor, Replace Colors. | The proven shape for generic editing: **ordered block list + per-block typed editors + group toggle + colour preview**. Archived -> no PoE2 grammar; confirms demand but offers no code to reuse. |
| **FilterBlade** (web, NeverSink-specific) | Not a generic parser: edits the *source* via NeverSink's own tag model (strictness slider, per-tier style/sound/size editing, sign-in sync to the account). Customisations survive updates because the site regenerates from tags. | Generic editors cannot copy this; FerrisFilter has no tag-model source. But the **`$type`/`$tier` tags give the same grouping** for read-only structure and for "restyle a whole tier". |
| **poe2filter.com** | Generator with advanced conditions (waystone tier, any-enchantment, since Jan 2025), exports commented config JSON at filter tail. | Output is plain grammar; load-and-save must survive its trailing config block. |

Confidence note: FilterBlade/poe2filter.com UX descriptions are from search snippets and the NeverSink README only (LOW-MEDIUM). Nothing found suggests any tool offers *lossless round-trip of arbitrary third-party filters*; FerrisFilter's byte-identical promise is a genuine differentiator, and also a strict engineering constraint.

## Table Stakes

Missing = the "edit any filter" claim fails.

| Feature | Why Expected | Complexity | Dependencies / Notes |
|---|---|---|---|
| Lossless parse of the full grammar (all conditions/actions in A1, all operators, bare values, multi-value lists, quoted strings with spaces/apostrophes, `Show`/`Hide`/`Minimal`/`Continue`/`Import [Optional]`) | The milestone goal; real filters use ~30 distinct keywords | High | New `filter` module (AST preserving trivia). Replaces use of `Config` serde model for the dynamic path only; `config.json` generate path untouched. |
| **Byte-identical round-trip** of an unedited filter (CRLF, tabs, trailing whitespace, comments, blank lines, commented-out blocks, header banners, trailing config block) | Stated acceptance criterion | High | Store raw text per line/span; only re-serialize *edited* blocks. Add golden test on `filterblade.filter` and each `reference-old-filters/*.filter`. Detect and preserve dominant EOL (CRLF) and indent. |
| Unknown keyword / unknown value preserved verbatim, shown read-only | Game adds conditions per patch; PoE2 gains new ones | Medium | "Raw line" node type attached to its block; UI shows it as a locked chip. Never drop, never reorder relative to siblings. |
| Block list with show/hide badge, derived title (header comment), index | Parity with current accordion rows | Medium | Current `view_model.rs`/`rule_editor.rs` keyed by rule index -> re-key on block identity (stable id per parse). Rule `name` becomes the header comment text, already how v0.4 works. |
| Section grouping from banner comments (`#=====` + `# [[0100]] Name`) -- collapsible groups of blocks | 447 blocks flat is unusable | Medium | Heuristic only; if no banners found, show a flat list. Do not make it required for correctness. |
| Edit existing conditions: class, baseType, rarity, areaLevel, itemLevel, quality, sockets, defences, corrupted, unidentified tier + the missing common ones (waystone tier, mirrored, identified, stack size, gem level, drop level, width/height, twice-corrupted, AlwaysShow) with correct operator selector | Real filters use them | Medium-High | Generalise the existing numeric/boolean/list controls into a data-driven condition descriptor table (keyword -> kind -> widget). Reuse v0.4 widgets. |
| Operator preservation: `==` vs bare vs `!=`/`<`/`>` re-emitted as read, changed only when the user touches it | Bare `BaseType` is substring match, `==` is exact; silently normalising changes behaviour | Medium | Store operator as parsed; UI exposes it. |
| Edit actions: text/border/background colour, font size, alert sound, custom sound, minimap icon, play effect, drop-sound toggles | Needed to restyle any block | Medium | Style = set of actions; existing `Style` emit logic provides serialisation. |
| Apply named style to a block (replace the block's action lines with a style's actions) | Existing v0.4 behaviour users rely on | Medium | Depends on config-driven styles (`styles.json`). Must define "block uses style X" for a parsed block: style is a **write operation** (stamp actions), not a stored reference (see Anti-features). |
| Enable/disable block | v0.4 parity | Low | Disabled = comment-out the block (`#Show ...`), consistent with how NeverSink/poe2filter authors do it. Parser must recognise the commented block as disabled-but-editable only if it matches the exact form we write; otherwise leave as plain comment. |
| Reorder / add / delete / duplicate block | v0.4 parity (order is precedence) | Medium | Moves a block's full text span incl. its leading comment. Duplicate is cheap and new. |
| Unsaved-edit model: in-memory, Apply writes, Revert reloads | v0.4 contract, config tab must follow it | Low | Apply = write the .filter into the game folder directly (filter *is* the store). Atomic write via temp+rename; keep a one-time `.bak` of the original on first save. |
| Open-file selection (pick which `.filter` to edit; remembered) | "any filter" implies choosing | Low-Med | New small config (last file). Overlay today only reads fixed repo-root paths; also hits the deferred CWD/anchoring issue -> settle path anchoring here. |
| Search/filter the block list (by text, BaseType, class) | 447 blocks; users reach for a specific item | Medium | Substring over block text; no new parser work. |
| Economy sections still refreshed from `prices.json`+`tiers.json` inside a hand-edited filter | Core product value survives | High | See section D. |
| Config tab: edit styles (`styles.json`) and tier cutoffs (`tiers.json`) with Apply/Revert | Milestone feature 4 | Medium | Style editor reuses colour pickers; tier editor = ordered numeric fields with ordering validation (existing loader already checks completeness/order). |
| Unknown style name caught at load time | Milestone feature 3 | Low | `styles.json` name->Style `HashMap`/`IndexMap`; validate all references on load, field-path error as elsewhere. |
| Single-instance `ff` (second launch shows existing stripe) | Milestone feature 5 | Low-Med | Named mutex + signal to the running instance reusing `visibility.rs` show path (Windows). |

## Differentiators

Not expected, valued; pick cheap ones.

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| Lossless guarantee + visible "unchanged blocks are untouched" diff before Apply | Nobody else promises it; builds trust for editing a 258 KB community filter | Medium | Block-level dirty flags give this nearly free; show "N blocks edited" in the stripe. |
| Tag-aware grouping: browse by `$type->` / `$tier->` and bulk-restyle a tier (e.g. all `$tier->s` blocks) | NeverSink's own tag model gives instant semantic grouping | Medium | Tags are optional metadata parsed from the header comment; absent = feature hidden. Uses same stamp-style operation applied to N blocks. |
| Economy refresh via marker-delimited managed sections (section D) | The unique selling point vs FilterBlade (no price-driven sections) | High | See D. |
| Block-level "what does this match?" plain-language summary | Makes unreadable `Class ==` mega-lists scannable | Low-Med | Derived from AST; truncate lists. |
| Show/Hide whole section toggle (Filtration's Block Group Browser) | Proven useful feature | Low-Med | Comment/uncomment spans; same disable mechanism. |
| Raw-text escape hatch per block (edit the block's text directly) | Covers anything the typed UI cannot model; also a safety net for parser gaps | Medium | Re-parse on commit; reject if it doesn't parse into a single block. Cheap insurance, recommend building. |
| Colour preview chip of the block as it renders (text on background, border, font scale) | Filtration/FilterBlade parity | Low-Med | v0.4 theme code already paints Dracula chips; extend to arbitrary RGBA. |
| Style "reverse lookup": mark blocks whose action set exactly equals a named style | Lets styles.json act as a palette over foreign filters | Medium | Pure comparison; optional. |

## Anti-Features

| Anti-Feature | Why Avoid | What to Do Instead |
|---|---|---|
| Full regeneration of the filter from a model (dropped on purpose) | Cannot be lossless; destroys hand edits | Patch only edited blocks / managed sections. |
| Storing "style: X" references inside the .filter (custom directive that the game would not understand) | Foreign syntax breaks the lossless/any-filter promise and may load-fail in-game | Stamp the style's concrete actions; optionally record the style name in the header comment as a plain comment hint. |
| Normalising/pretty-printing the user's filter on save | Violates byte-identical; creates giant diffs against upstream NeverSink updates | Touch only edited spans. |
| Validating BaseType/Class names against a bundled list | Goes stale each league; game warns on load (already a v0.4 decision) | Warn nothing; keep as-is. |
| Rewriting NeverSink's `Continue` semantics, collapsing blocks, or auto-merging duplicates | Order and `Continue` change item behaviour | Show `Continue` as a flag, preserve order, warn only when the user reorders across a Continue block. |
| Building a FilterBlade-style strictness slider / tier-regeneration engine | Requires NeverSink's source domain language; out of scope | Leave strictness to FilterBlade; FerrisFilter edits the exported result. |
| In-game load message / online-filter header | Online-only (spike 2026-10-12) | Deferred. |
| `Import` layering / locked rules, presets, economy change summary | Explicitly out of scope (999.4/999.7/999.9) | Parse `Import` and preserve it as a raw node only. |
| Auto-save on every keystroke | v0.4 accepted trade-off: explicit Apply/Revert | Keep model. |
| Auto-installing/overwriting an upstream filter file without backup | Users lose their only copy of a downloaded filter | `.bak` on first write; atomic rename. |
| Supporting multiple filters open at once / diff-merging upstream updates | Large scope, not needed | Single file at a time. Re-import upstream = reopen. |

## D. Economy-priced sections inside a hand-edited filter (the open design question)

How the ecosystem handles "generated section inside user-owned text":

1. **Tag-addressed (NeverSink/FilterBlade).** Every block carries `$type->x $tier->y !slug` in its header comment. A tool can *find* blocks by tag and *replace their conditions* (e.g. `BaseType ==` list) without a fenced region. Works for NeverSink filters only.
2. **Fenced managed region (generic pattern, e.g. `# >>> ferrisfilter:currency begin` / `end`).** Everything between the fences is owned by the tool and rewritten on Apply/Refresh; everything outside is untouched. Simple, robust to hand edits outside the fence, and visible to the user as a comment banner.
3. **Per-block ownership marker** (`# ff:managed currency S`) on each generated block, so reorder by the user is respected and individual blocks can be regenerated in place.

Recommendation (opinionated): **(3) with (2)'s fallback**. Mark each price-driven block with a comment marker `# ff:economy <category> <tier>` in its header, discover them on load, and on "refresh economy" rewrite only the `BaseType` list/styling of marked blocks in place (preserving the user's chosen position). First-time insertion of the economy set into a foreign filter appends a fenced region (default: at the top, since first-match-wins and the economy blocks must precede hide rules) that the user can then drag. The marker is a plain `#` comment so the game and every other tool ignore it. Complexity: High. Depends on pricer/emitter output still being a list of blocks (it is: `emit_rules` and economy sections already compile to `Show` blocks) -> reuse their text, parse it with the new parser into block nodes, splice. Open questions for phase research: where to insert in a NeverSink file (its own `$type->currency` blocks overlap and would shadow/duplicate), whether a user should be able to point the tool at NeverSink's tag blocks instead (differentiator, defer), and what Refresh does to a block the user hand-edited inside the managed region (recommend: overwrite, with the warning "managed block").

## Feature Dependencies

```
styles.json (name -> Style map, load-time validation)
   -> Config tab: style editor
   -> "apply style to block" in the rule editor
   -> economy block emission (tier styles)

Lossless filter parser/AST (trivia-preserving)
   -> byte-identical round-trip test (filterblade.filter + reference-old-filters/*)
   -> block list / sections / tags
   -> condition+action typed editors (data-driven descriptor table)
        -> raw-line preservation for unknown keywords
   -> filter-as-store (drop dynamic.json, dynamic_base.json, compile_store)
        -> Apply writes the .filter in place (+ .bak)
        -> economy managed blocks (marker discovery + refresh)
             <- pricer/emitter (unchanged) + tiers.json
   -> disable (comment-out) / reorder / duplicate / delete operate on spans

tiers.json editor (Config tab) -> independent of parser; shares Apply/Revert plumbing
Single-instance guard -> independent (win.rs / visibility.rs)
Open-file selection + path anchoring -> prerequisite for the overlay to find any filter
```

## MVP Recommendation

Build in this order (parser is the critical path):

1. Lossless parser + serializer with byte-identical golden tests (filterblade.filter, every reference filter). Everything else depends on it. No UI.
2. `styles.json` + config-driven style lookup (small, unblocks both the config tab and style application).
3. Filter-as-store: open file, block list with sections/search, existing v0.4 controls retargeted onto parsed blocks; Apply/Revert/.bak.
4. Generalised condition/action editors (descriptor table) + unknown-keyword chips + raw-block text editor.
5. Economy managed blocks with `# ff:economy` markers and refresh.
6. Config tab (styles, tier cutoffs) and single-instance guard (independent, can be parallel).

Defer: tag-based bulk restyle, style reverse lookup, section-level toggles (cheap once spans exist, but not required), plain-language summaries.

## Phase research flags

- Parser/round-trip: deeper research needed on edge cases (`Continue` position, quote escaping, comment-only blocks, Windows CRLF handling, `Sockets >= 5GGG`, bare multi-value lists); likely best driven by corpus tests rather than documentation.
- Economy marker design in a foreign filter: needs a design spike (insertion point vs NeverSink shadowing).
- Single-instance, tiers/styles editor: standard patterns.

## Sources

- Official filter language reference (PoE2-tagged conditions, HIGH): https://www.pathofexile.com/item-filter/about
- NeverSink PoE2 filter repo README (strictness/style structure, MEDIUM): https://github.com/NeverSinkDev/NeverSink-Filter-for-PoE2
- Filtration (archived PoE1 editor, features list, MEDIUM): https://github.com/ben-wallis/Filtration
- poe2filter.com thread/changelog (advanced waystone/enchantment conditions, LOW): https://de.pathofexile.com/forum/view-thread/3630063/page/36
- Local corpus (HIGH, primary): `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\filterblade.filter`; `reference-old-filters/*.filter`
- FilterBlade site (403 on fetch; editing model from search snippets only, LOW): https://filterblade.xyz/
