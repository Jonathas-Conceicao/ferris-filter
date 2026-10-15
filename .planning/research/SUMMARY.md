# Project Research Summary

**Project:** FerrisFilter v0.5 "Generic Filter Editor & Initial App Configuration"
**Researched:** 2026-10-12
**Confidence:** MEDIUM-HIGH

## Executive Summary

v0.5 changes FerrisFilter's job. It used to compile our own policy into a filter. Now it should
open any PoE2 filter, edit it, and keep our economy-priced sections up to date. The real-world
sample is a 258 KB CRLF NeverSink/FilterBlade export. All four research streams agree on the core
design:

- **The `.filter` file is the store.** A new parse tree in `src/filter/` keeps the original
  bytes of every line: indent, line ending and trailing comment. Parsed fields are only a cache,
  and only lines the user edits are written back from them, so content nobody touched can't change.
- **The static path doesn't change.** `Config`/`When`/`Rule` and `generate` stay as they are, which
  protects the `artifact_guard` byte-for-byte fixture.
- **The overlay's existing `When` editing functions are reused** as a typed view over the parsed
  blocks.

This needs no new crates:
- The parser is hand-written in std Rust, because the grammar is flat and line-oriented.
- The single-instance guard uses the `windows-sys` dependency we already have, plus two feature
  flags.
- The config tab uses egui's built-in colour pickers.
- Styles become an ordered name-to-`Style` map (`StyleSet`) that `config.json` and `styles.json`
  share.

Build order follows the dependencies:
1. The lossless parser, gated on a round-trip test over real filters. It is the highest risk and
   everything else depends on it.
2. The styles refactor, which can run at the same time as the parser. It is a pure refactor,
   guarded by `artifact_guard`.
3. Economy ownership and filter-as-store.
4. The editor UI.
5. The config tab.

The single-instance guard doesn't depend on anything else and can go anywhere.

The main risks:
- **Losing fidelity.** Rebuilding text from a syntax tree, normalizing CRLF or a BOM,
  mishandling `#` inside quotes, and git autocrlf rewriting CRLF fixtures can all change the
  file's bytes.
- **Refresh destroying edits.** Once the file is the user's only copy, an economy refresh that
  overwrites hand edits loses them for good.
- **Changing a filter's meaning.** `Continue`, operator differences (`Class "x"` matches a
  substring, `Class == "x"` an exact name) and unknown keywords can all be mishandled.
- **List size.** A filter has about 460 blocks, which egui redraws every frame.
- **The working-directory bug.** The known CWD-relative path defect gets worse with every new file
  the app reads.

## Key Findings

### Stack (STACK.md)
- **Parser:** hand-written with std only.
  - Each line keeps its own line ending; never use `str::lines()`, which drops them.
  - The BOM is preserved, and non-UTF-8 input fails loudly.
  - Rejected: winnow, nom, pest, chumsky, logos, rowan.
- **Single instance:** `windows-sys` 0.61 plus `Win32_System_Threading` and `Win32_Security`,
  using a named mutex and a named event.
  - Rejected: the `single-instance` crate (it can't signal the running copy), `interprocess`, the
    `windows` crate, `FindWindow`/`PostMessage`, lock files.
  - The feature names are unconfirmed until `cargo check`.
- **Config tab:** egui 0.36 built-ins (`color_edit_button_srgba`, `DragValue`, `ComboBox`). Colours
  are stored as `Color32` u8 values, so they never drift through floating point.
- **Persistence:** serde plus the existing `tempfile` atomic writes.

### Features (FEATURES.md)
- **Must have:**
  - Lossless parsing of the full grammar: about 60 keywords, every operator (including bare
    values), multi-value lists, quoted strings, and `Import`/`Continue`/`Minimal`.
  - Byte-identical round trip.
  - Unknown keywords kept and shown read-only.
  - A block list with stable IDs, grouping by section banners, and search.
  - Typed editors for the conditions real filters use (AlwaysShow, ItemLevel, WaystoneTier,
    StackSize, HasExplicitMod, …), keeping their operators.
  - Action editors, where applying a style writes that style's actions into the block.
  - Enable/disable, reorder, add, delete and duplicate blocks.
  - Apply/Revert, plus a `.bak` copy of the original.
  - Choosing which file to open.
  - Economy refresh inside a hand-edited filter.
  - The config tab and single-instance guard.
- **Should have (cheap):** an "N blocks edited, rest untouched" indicator, a per-block raw-text
  editor as an escape hatch, and colour preview chips.
- **Defer:**
  - Bulk restyle by NeverSink `$type`/`$tier` tags.
  - Finding which blocks use a style.
  - Section on/off toggles.
  - Working across several files or merging diffs.
  - A strictness engine.
- **Anti-features (don't build):**
  - Full regeneration from a model.
  - Pretty-printing on save.
  - A custom `style:` directive inside the filter.
  - Lists for validating BaseType or Class names.
  - Saving on every keystroke.
  - Rewriting a foreign file on open, or when Apply changes nothing.
  - Editing `Import`, which is parsed and kept as raw text only.
- **Sample-file facts:**
  - 5312 lines, CRLF line endings, tab indentation.
  - 447 Show, 13 Hide and 40 `Continue` blocks.
  - About 25 condition keywords the overlay can't edit today.
  - Values with no operator, like `Rarity Normal Magic Rare`, and `HasExplicitMod >=1` with no
    space after the operator.
  - Tags in block headers (`%D4 $type->… $tier->… !slug`) and `# [[0100]]` section banners.

### Architecture (ARCHITECTURE.md)
- **Each block is one of three kinds:**
  - **Editable** if reading its conditions into `When` and writing them back reproduces the exact
    original lines.
  - **Partial** otherwise. The conditions `When` understands get typed controls, the rest are shown
    as read-only chips, and `apply_when_diff` patches edits into the original lines.
  - **Locked** if it can't be edited at all. It shows as raw text, and can only be enabled, moved
    or deleted.
- **`StyleSet` is ordered and validated when loaded.** The tier styles S..F must exist. An unknown
  style on an enabled rule is a hard error. A disabled rule with an unknown style does nothing.
- **Compile changes:**
  - `load_economy` is split out of `load_inputs`.
  - `emit_economy(&StyleSet, &PricedData)` is exposed.
  - `dynamic.rs` shrinks to `write_filter`, `default_destination`, and
    `apply_text`/`refresh_economy`.
  - The seed/reset/base/store code is deleted.
- **One path resolver, `AppRoot`, replaces every use of `current_dir()`.** It tries
  `FERRISFILTER_HOME`, then the exe's folder, then folders above the exe, then the working
  directory, and otherwise shows an error.
- **Apply writes the rendered filter back to its own file atomically.**
  - Revert re-parses the last applied text, which is held in memory.
  - Apply refuses to write if the file changed on disk since it was loaded (mtime/hash check).
  - The config tab's Apply writes only `styles.json` and `tiers.json`, and marks the filter as
    out of date.

### Critical Pitfalls (PITFALLS.md)
1. **Rebuilding text from a syntax tree, or normalizing line endings and the BOM.**
   - Store each line's raw text, its own line ending, and flags for the BOM and the final newline.
   - The parser phase is done only when `serialize(parse(x)) == x` holds for:
     - the real FilterBlade sample, `reference-old-filters/*`, `seed-universal.filter`, and a
       generated `dynamic.filter`;
     - made-up test files with CRLF, LF, mixed endings, no final newline, and a BOM.
   - Add `*.filter -text` to `.gitattributes`.
2. **Splitting comments wrongly, or rejecting unknown keywords.**
   - The tokenizer has to know when it is inside quotes.
   - Commented-out blocks (`#Show  # # %D9 …`) must come back unchanged.
   - Every input must parse: an unknown keyword becomes an opaque line.
   - Never `trim()` a stored raw line.
3. **Economy refresh overwriting edits.**
   - Only rewrite bytes the tool owns.
   - Missing or duplicated markers stop the refresh.
   - Never add economy sections to a foreign filter without the user asking.
4. **Breaking the static byte guarantee.**
   - `StyleSet` is ordered, and the emitter writes style fields in a fixed order.
   - Before deleting the old `Styles` struct, assert that the default `styles.json` equals its
     hardcoded values.
   - Run `artifact_guard` after every styles commit.
   - Never regenerate a fixture in a refactor commit.
5. **Changing a filter's meaning when editing.**
   - Only change the bytes of the value being edited.
   - Never turn `Class "x"` into `Class == "x"` unless the user edits it.
   - Treat `Continue` as a flag on the block, and warn when a reorder moves a block across one.
   - Identify blocks by stable ID, not position.
6. **Style and tier edits.**
   - No renaming styles in v0.5.
   - The built-ins (`hidden`, `unchanged`, `kite`, and the tier letters) can't be deleted.
   - The tier editor keeps text buffers separate from the parsed values, and `ValidatedTiers` is
     the only validator.
   - The unit is shown read-only.
   - Applying both files validates both, writes both to temp files, then renames both.
7. **Single-instance traps.**
   - Use a `Local\` mutex held for the whole of `main`, and read `GetLastError` straight away.
   - The second launch has to signal the first.
   - A hidden window needs a repaint wake-up.
   - The signal must get past the hotkey's foreground-window check.
   - If the first copy is hung, show a message.
8. **List size and the working directory.**
   - Resolve all paths through one function early on.
   - Parse once, cache row summaries, only draw visible rows, and add search.

## Where the Researchers Disagree

**D1. How FerrisFilter owns the economy blocks inside a user-edited filter**

| | One fenced BEGIN/END region | A `# ff:economy <category> <tier>` marker on each block |
|---|---|---|
| Recommended by | ARCHITECTURE; PITFALLS #7 | FEATURES §D (fenced region only for the first insertion) |
| How it works | All economy blocks sit in one region, and refresh replaces that byte range | Each priced block is tagged, and refresh rewrites the BaseType list and styling of tagged blocks where they are |
| Pros | Simple. A refresh with nothing new produces identical bytes. Easy to test, easy to detect damage, shows as one locked row | The user can move individual economy blocks, e.g. put currency S above a specific Hide |
| Cons | Economy blocks can't be mixed in among the user's rules | Harder: finding the blocks, block counts changing as tiers gain or lose items, hand-edited tagged blocks. More ways for a refresh to be partial or inconsistent |

**What both sides agree on:** markers are plain `#` comments. Blocks are never identified by their
content or heading. Economy blocks are only added to a foreign filter through an explicit
"Insert economy section" action. The synthesizer's view: the fenced region alone is cheaper and
fits the lossless-first goal, and per-block markers could come in v0.6.

**D2. How a disabled block is written to disk**
- **ARCHITECTURE:** comment out the whole block with a distinctive `#~ ` prefix, and only treat
  blocks with that prefix as disabled. Unambiguous, but a new convention.
- **FEATURES:** comment it out with a plain `#`, as NeverSink and poe2filter.com authors do. That
  is idiomatic, but blocks commented out by someone else could be mistaken for ones we disabled.
- **PITFALLS:** blocks commented out by someone else stay as opaque comments. Turning them back on
  is optional; if offered, strip exactly one `#` from each line of the commented run.

**D3. Smaller disagreements**
- **Style order:**
  - STACK suggests `BTreeMap` plus an `order` list.
  - ARCHITECTURE says `BTreeMap` sorts S..F alphabetically and loses the intended order, and
    prefers `indexmap` or serde_json's `preserve_order`. `preserve_order` applies to every
    `serde_json::Value`, so its effect on the fixtures needs checking.
- **How a block remembers its style:**
  - ARCHITECTURE: a header tag like `Show # name [ff:S]`, with the actions regenerated from
    `styles.json` so style edits flow through to the filter.
  - FEATURES: write the style's actions into the block, at most with a plain comment as a hint.
  - PITFALLS: make the choice explicitly.
- **Waking the first instance:**
  - STACK: a watcher thread waiting on the event.
  - ARCHITECTURE: check the event every frame, next to the hotkey check.
  - Either way, a hidden window needs `request_repaint`.
  - PITFALLS mentions the `windows` crate; follow STACK and use `windows-sys`.
- **Backups:** FEATURES suggests a one-time `.bak`, ARCHITECTURE a `.ffbak` per launch, PITFALLS a
  timestamped copy before a reset. Pick one naming scheme.
- **proptest:** STACK says it isn't needed; ARCHITECTURE mentions proptest-style mutation tests.

## Implications for Roadmap

Seven suggested phases. The parser and styles phases are independent starting points.

1. **Foundation: `AppRoot` and fixture hygiene.**
   - The path resolver.
   - `.gitattributes` with `*.filter -text`.
   - Checking the existing fixtures.
   - This could be folded into phase 4.
2. **Lossless parser and round-trip test files (999.6 foundation).**
   - The `src/filter/` parse tree.
   - A keyword table, where anything unknown is opaque.
   - Writing the tree back out.
   - A rule for comment lines: those directly above a block belong to it.
   - Done when the round-trip test passes.
   - Needs research.
3. **Config-driven styles (999.1), alongside phase 2.**
   - `StyleSet`, loading and saving `styles.json`, validation, and protected built-ins.
   - Done when `artifact_guard` and `generate_snapshot` stay byte-identical.
   - Many tests build `Styles { s: .. }` directly, so add a `StyleSet::from_pairs` helper.
4. **Economy ownership and filter-as-store (999.5).**
   - Economy refresh where nothing changes if nothing is new.
   - The `load_economy` split.
   - A headless `apply_text` the CLI and overlay share.
   - Backup and the changed-on-disk check.
   - An atomic write in the same folder as the target, retrying when OneDrive has the file locked.
   - CLI `apply` loses `--reset`/`--base`/`--store`. This breaks existing usage, so it needs a
     CHANGELOG entry.
   - Optionally, a one-time `dynamic.json` import.
   - Needs research.
5. **Overlay editor over parsed blocks (999.6 UI), in two parts.**
   - **5a:**
     - Load, display, reorder, delete, enable and duplicate any block, but only edit Editable ones.
     - Search, grouping by section banners, and drawing only visible rows.
     - A locked row for the economy section.
     - A flag for `Continue`, with a warning when reordering across it.
     - Stable block IDs.
   - **5b:**
     - Partial blocks, and reading values with their operators kept.
     - Read-only chips for conditions the editor doesn't understand.
     - A bigger table of condition and action editors.
     - "Adopt style" and the raw-text escape hatch.
     - The edited-blocks indicator.
     - 5b needs research.
6. **Config tab (999.8, partial).**
   - RULES | CONFIG tabs.
   - A style editor: add, edit and delete, but no rename.
   - A tier editor that shows `ValidatedTiers` errors inline and the unit read-only.
   - Apply saves both files or neither.
   - Shows the resolved file paths.
   - Edits the live `tiers.json` only, never the committed fallback pair.
7. **Single-instance `ff` (999.2), independent of the rest.**
   - A `Local\` mutex held in `main`, and an event that reveals the stripe rather than toggling it.
   - Getting past the hotkey's foreground check, plus a repaint wake-up.
   - A timeout message if the first copy is hung.
   - A different mutex name for debug builds.

**Research flags:**
- **Phase 2:** the actual PoE2 keyword set; how `Import`/`Continue`/`Minimal` behave; FilterBlade
  and NeverSink quirks (BOM, mixed line endings, trailing config blocks, `#` in quotes,
  `Sockets >= 5GGG`, `HasExplicitMod >=1`). Driven by test files.
- **Phase 4:** the economy ownership design (D1); where to insert economy blocks in a foreign
  filter, since NeverSink has its own currency blocks; OneDrive rename and lock behavior.
- **Phase 5b:** edge cases in reading conditions with their operators, and egui performance with
  about 460 blocks.
- **Phases 1, 3, 6 and 7** follow standard patterns and can skip research.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | Crate versions checked with `cargo search`. The `windows-sys` feature names are unconfirmed |
| Features | MEDIUM-HIGH | The grammar is HIGH (official page plus local filters). How competitors' editors work is LOW-MEDIUM; FilterBlade and poe2filter.com couldn't be fetched |
| Architecture | MEDIUM-HIGH | Integration points come from reading the code. The tree, region and tag designs are untested recommendations |
| Pitfalls | MEDIUM-HIGH | Parser facts were measured on the sample. Win32 details are MEDIUM, and other NeverSink variants LOW |

**Gaps:**
- Only one third-party filter was measured. Phase 2 should add other strictness levels and files
  with `Import`, `Minimal` and mixed line endings.
- The PoE2 keyword table is built from real filters, so missing keywords are possible; treating
  unknown keywords as opaque keeps that from breaking anything.
- Where economy blocks go in a foreign filter needs a spike.
- The effect of `preserve_order` on the fixtures hasn't been checked.
- What a new filter starts with is undecided now that `dynamic_base.json` is gone.

## Open Questions for the Author

1. **Economy ownership (D1):** one fenced region, per-block markers, or both? Is an explicit
   "Insert economy section" action enough for a foreign filter, and should it insert at the top or
   the end?
2. **Disabled blocks (D2):** a `#~ ` prefix, a plain `#` before each line, or only allow
   re-enabling blocks the tool itself disabled?
3. **How a block remembers its style (D3):** a `[ff:S]` tag in the header so style edits flow
   through, or just write the actions into the block? Should foreign blocks get an
   "Adopt style" action?
4. **Should `styles.json` be committed or git-ignored?** If ignored, the defaults would be built
   into the binary and written out on first run.
5. **What does a new filter start with, and what happens to `dynamic.json`?** Does a new filter
   start empty with just an economy region, or from a built-in starter template? Should an
   existing `dynamic.json` be imported once or dropped?
6. **Where does the filter live?** Edit the game-folder `dynamic.filter` directly, or keep a
   working copy in the repo? Is "open any `.filter`, and remember its path" part of v0.5?
7. **How styles keep their order:** `indexmap`, `preserve_order`, or `BTreeMap` plus an order list?
8. **Renaming styles:** forbidden in v0.5, or supported?
9. **Backups:** one-time `.bak`, per-launch, or timestamped, and where do they go?
10. **CLI `apply` flags:** remove `--reset`/`--base`/`--store` outright, or deprecate them for a
    release first?
11. **Mutex name in debug builds:** the same as release, or a separate one?
12. **proptest:** add it for fuzzed round trips, or rely on test files alone?
13. **Config tab Apply:** confirm it writes only `styles.json` and `tiers.json` and marks the
    filter as out of date.

## Sources
- **Primary (HIGH):**
  - The local filters: `…/Path of Exile 2/filterblade.filter` and `reference-old-filters/*.filter`.
  - Reading the code, and `CLAUDE.md`.
  - pathofexile.com/item-filter/about.
  - `cargo search` and the local crate registry.
- **Secondary (MEDIUM):** the NeverSink PoE2 README, the Filtration repo, and general Win32 knowledge.
- **Tertiary (LOW):** the poe2filter.com changelog thread and FilterBlade (blocked when fetched).
