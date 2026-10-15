# Phase 16: Lossless Filter Parser - Research

**Researched:** 2026-10-12
**Domain:** Lossless, line-preserving parser/serializer for the PoE2 item-filter grammar (pure Rust, std only)
**Confidence:** HIGH (grammar re-fetched from the official page today; samples measured locally)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**Representation**
- **Line-preserving model.** Every line keeps its exact original text, including its own line ending (CRLF/LF/none) and indentation. A BOM and the presence or absence of a final newline are recorded. Parsed values are a lookup layer over those raw lines. Serializing an unedited file emits the raw lines verbatim, and editing a block re-renders only the lines that block owns. Never use `str::lines()` (it drops terminators), and never `trim()` a stored raw line.
- **Unedited bytes never change.** No pretty-printing or normalization on save (anti-feature).
- **Comments are just comments.** Comment lines directly above a block attach to that block, so they move with it when it is reordered later. Every other comment line stays where it is as standalone content.
- **No tool-specific comment parsing.** Do not interpret FilterBlade/NeverSink tags (`$type->…`, `$tier->…`, `%D4`, `!slug`) or poe2filter.com conventions (`# [[0100]]` banners, trailing `[[Config Start]]` JSON). They are ordinary comments. Only GGG's grammar is parsed.

**FerrisFilter's own metadata comments (cross-cutting; defined here, used by Phases 18-20)**
- FerrisFilter stores its own per-block configuration in a **header-style comment run directly above the block**, one `#ff:<key> <value>` line per setting. Examples: `#ff:name Exceptional 3 socks`, `#ff:style S`. The style follows the game's online-filter header (`#name:filterblade`).
- The parser recognizes this run and exposes it as structured metadata on the block (name, style, and later keys), while still preserving the bytes exactly.
- Section-level markers use the same family, e.g. `#ff:economy-begin` / `#ff:economy-end`. Phase 18 fixes the exact marker text.
- Foreign header comments (`Show # $type->currency …`) are **not** a name. Only `#ff:name` is. Exact key spelling is at the planner's discretion within this shape.

**Commented-out blocks (disabled rules)**
- A commented-out block is a **disabled rule**: a `#Show`/`#Hide`/`#Minimal` header line followed by `#`-prefixed condition/action lines. Disabling adds one `#` to each of the block's lines, and enabling removes exactly one.
- **A commented-out block may not be valid grammar.** The parser must try to parse its inner lines and classify it: **valid** (a disabled rule that can be enabled and edited) or **invalid** (kept byte-for-byte; can be shown, moved and deleted; the reason it can't be enabled is reported, e.g. "line 412: unknown operator"). Enabling is only possible when the inner lines parse cleanly. Comment text that doesn't look like a block at all stays an ordinary comment.
- **Enabled rules must always be grammatically correct.** Nothing the tool produces may yield an invalid enabled block. An enabled block that fails to parse in a loaded file is surfaced as an error tied to its line; the tool must not crash or silently rewrite it. It is flagged, and Apply is refused until fixed or disabled (later phases). The error must be visible, never silent.

**Grammar coverage**
- **One keyword table** is the single source of truth for the grammar: every block type, condition and action from GGG's page: name, kind (condition/action), allowed operators, value shape (number, range, quoted list, enum such as colour or shape, RGBA, boolean, socket string like `5GGG`, sound id/volume, file path, …). Covers `Continue`, `Import [Optional]` and `Minimal`. Re-fetch the official page during research and check the table against it. The FilterBlade sample is test material only, not a source of grammar.
- A keyword not in the table is kept as an opaque line with a warning, and its block remains otherwise usable. It must never stop a filter from loading.

**Test material**
- Commit a copy of the FilterBlade/NeverSink filter (`…/Path of Exile 2/filterblade.filter`, 258 KB, CRLF, tab-indented) as a test fixture. Add `*.filter -text` to `.gitattributes` so git never rewrites fixture line endings.
- Done when `serialize(parse(x)) == x` byte-for-byte for: that fixture, `reference-old-filters/*.filter` (git-ignored locally, tolerate absence), `tests/fixtures/seed-universal.filter`, a generated dynamic filter, and synthetic edge cases (LF, CRLF, mixed endings, BOM, no final newline, tabs, trailing whitespace, `#` inside quoted values, operators without spaces like `>=1`).
- A test edits one block and asserts that every other byte is unchanged.
- **No CLI `check` command.** No proptest.

### Claude's Discretion
- Module layout (suggested `src/filter/`), type names, error type, and how block identity is represented for later phases (stable IDs, not indices).
- How permissive operator/value parsing is (e.g. `Rarity Normal Magic Rare` bare multi-value, `Class "x"` substring vs `Class == "x"`); it must match the documented grammar and preserve operators exactly.
- Whether parsed values are computed eagerly or lazily.

### Deferred Ideas (OUT OF SCOPE)
- `ff-cli check <file>` diagnostic command.
- Raw-text block editing: backlog 999.10.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARSE-01 | Open any working PoE2 filter without error (FilterBlade/NeverSink, reference files, poe2filter.com exports, FerrisFilter output); unknown keyword kept opaque | Diagnostics-not-Err design; opaque-line node; "zero Error diagnostics on all real samples" gate |
| PARSE-02 | Every block type, condition, operator, action on the official page recognized, incl. Continue/Import/Minimal | Keyword table section (fetched 2026-10-12) |
| PARSE-03 | No-edit save is byte-identical (CRLF, LF, mixed, BOM, no final newline, real sample) | Line splitter spec + per-line EOL storage; test matrix |
| PARSE-04 | Editing one block changes only that block's lines | Block line-range ownership + edit API that replaces lines of one block only |
</phase_requirements>

## Summary

This is a pure-std Rust module with no new dependencies. The representation is a flat `Vec<Line>` where each `Line` stores `text` (no terminator) and `eol` (`Lf | CrLf | None`), plus document-level `bom: bool`. Serialization is `bom + for each line: text + eol`. That alone makes PARSE-03 true by construction as long as the splitter keeps terminators. Everything else (blocks, conditions, actions, disabled blocks, `#ff:` metadata) is an index layer over those lines: each block is a `Range<usize>` of line indices plus classified `LineKind`s and parsed fields. Editing a block replaces lines inside its range only (PARSE-04).

Measured sample facts: `filterblade.filter` (258,094 bytes, 5312 lines) is pure CRLF, tab-indented, no BOM, final `\r\n`, 447 Show / 13 Hide / 40 `Continue`, zero `Import`/`Minimal`, 468 lines matching `Show.*#`. The `reference-old-filters/*.filter` files are **LF-only with no final newline** (they end `##`), poe2filter.com banners `#####…`, 97 live blocks in `general.filter`. `tests/fixtures/seed-universal.filter` is stored LF in the git index but checked out CRLF here (`git ls-files --eol`: `i/lf w/crlf`) because of `* text=auto` + `*.filter text` + `core.autocrlf=true`. The existing `~/Path of Exile 2/dynamic.filter` has no BOM and ends `\n` with LF (single check: first bytes `#d`, last `0 \n`).

Parsing strategy for errors: the parser returns `Result<Document, LoadError>` where `Err` is ONLY for non-UTF-8 input. Every grammar problem is a `Diagnostic { line, severity, message }` stored on the document; the load never fails on grammar. Enabled-block errors are `Severity::Error`; unknown keywords are `Severity::Warning`.

**Primary recommendation:** Build `src/filter/{mod,lines,keywords,parse,block,disabled,meta,edit,diag}.rs` std-only; make the raw-line round trip the first commit and gate everything else on the byte-identity test over the FilterBlade fixture.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parse/serialize filter text | Library (`ferrisfilter::filter`) | — | CONTEXT: library + tests only, no overlay/CLI |
| Keyword table / grammar | Library | — | Single source of truth, consumed later by Phase 20 editor |
| File IO / game folder | Not this phase | Phases 18/19 | Parser takes `&[u8]`/`&str`, returns text; no IO policy here |
| Diagnostics display | Not this phase | Phases 18-20 | Phase 16 only produces `Vec<Diagnostic>` |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| std only | edition 2024 (Cargo.toml) | splitter, tokenizer, tables | The grammar is line-oriented; a regex crate would add nothing and invites normalization |
| thiserror | 2.0 (already a dep) | `LoadError` (invalid UTF-8) | Existing error convention (`src/error.rs`) [VERIFIED: Cargo.toml] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3 (existing) | tests writing temp files | Only if a test does a file round trip |
| insta | 1.47.2 (existing dev-dep) | optional | Do NOT snapshot the 258 KB file; use direct byte comparison |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| hand tokenizer | `logos`/`nom`/`chumsky`/`rowan` | `rowan` fits lossless CSTs but adds a dependency and tree complexity for a line-based format; rejected [ASSUMED: not evaluated further] |
| `regex` | — | Not a dep; hand-written scanning is ~100 lines |

**Installation:** none. **Package Legitimacy Audit:** no external packages are added this phase, so slopcheck was not required. (Packages recommended: none.)

## Official Grammar (re-fetched 2026-10-12) [CITED: https://www.pathofexile.com/item-filter/about]

The page does NOT specify comments, whitespace, line endings, escaping, or multi-value parsing. These are therefore conventions inferred from real files.

**Block types:** `Show`, `Hide` (normal filters), `Minimal` (Ruthless), `Continue` (modifier line inside a Show/Hide/Minimal block; matching does not stop), `Import "file" [Optional]` (top-level directive).
**Operators:** `=`, `==`, `!`, `!=`, `<`, `<=`, `>`, `>=`. The page gives no per-condition restrictions (only that `Rarity` takes comparisons). Bare value with no operator is legal and used in the wild.

**Conditions**
- Boolean (`True`/`False`): AlternateQuality, AlwaysShow, AnyEnchantment, BlightedMap, Corrupted, ElderItem, ElderMap, Exceptional, Foulborn, FracturedItem, HasCruciblePassiveTree, HasImplicitMod, HasVaalUniqueMod, Identified, Imbued, IsVaalUnique, MirageMap, Mirrored, Replica, Scourged, ShapedMap, ShaperItem, SynthesisedItem, TwiceCorrupted, UberBlightedMap, Vestigial, ZanaMemory; TransfiguredGem (bool OR quoted gem name).
- Numeric: AreaLevel, BaseArmour, BaseDefencePercentile, BaseEnergyShield, BaseEvasion, BaseWard, CorruptedMods, DropLevel, EnchantmentPassiveNum, GemLevel, Height, ItemLevel, LinkedSockets, MapTier, MemoryStrands, Quality, StackSize, UnidentifiedItemTier, WaystoneTier, Width, HasEaterOfWorldsImplicit (1-6), HasSearingExarchImplicit (1-6).
- Name/string list: ArchnemesisMod, BaseType, Class, EnchantmentPassiveNode, HasEnchantment (Num then name), HasExplicitMod (Num then one or more names).
- Enum: GemQualityType (Superior, Divergent, Anomalous, Phantasmal), HasInfluence (Shaper, Elder, Crusader, Hunter, Redeemer, Warlord, None), Rarity (Normal, Magic, Rare, Unique; multi-value allowed, ordered compare).
- Socket: `Sockets`, `SocketGroup`: number plus colour letters R G B A D W (`5GGG`, `6`, `RGB`).

**Actions**
- `PlayAlertSound Id [Volume]` / `PlayAlertSoundPositional Id [Volume]`: Id 1-16 or `None`; volume 0-300.
- `CustomAlertSound File [Volume]` / `CustomAlertSoundOptional File [Volume]`: file may be semicolon-separated list; `"None"` disables.
- `DisableDropSound`, `EnableDropSound`, `DisableDropSoundIfAlertSound`, `EnableDropSoundIfAlertSound`: no args.
- `MinimapIcon Size Colour Shape`: size 0-2 or `-1` (then colour/shape may be absent in disable form). Colours: Red Green Blue Brown White Yellow Cyan Grey Orange Pink Purple. Shapes: Circle Diamond Hexagon Square Star Triangle Cross Moon Raindrop Kite Pentagon UpsideDownHouse.
- `PlayEffect Colour [Temp]` or `None`.
- `SetBackgroundColor` / `SetBorderColor` / `SetTextColor R G B [A]` (0-255).
- `SetFontSize N` (1-45).

The page marks PoE2 additions (`AlwaysShow`, `IsVaalUnique`, `HasVaalUniqueMod`, `TwiceCorrupted`, `UnidentifiedItemTier`, `WaystoneTier`) but is one document for both games, so the table accepts the whole vocabulary [VERIFIED: fetched page + `.planning/research/FEATURES.md` A1]. Compare the table to the page at implementation time; add a test that every name in the table is unique and that the FilterBlade sample's keyword set is a subset of the table (that is a check of the table, not its source).

## Architecture Patterns

### Data flow
```
bytes/&str --> strip BOM --> split on '\n' (keep CR as part of eol) --> Vec<Line{text,eol}>
   --> classify each line (Blank | Comment | Header | Cond | Action | Continue | Import | Opaque | DisabledHeader | DisabledBody)
   --> group: leading comments + header + body lines => Block{id, range, enabled, kind, ...}
   --> Document{bom, lines, items:[Trivia|Block|Import|Opaque], diagnostics}
   --> serialize(): bom + concat(text+eol)   (edits replace lines inside one block range only)
```

### Recommended structure
```
src/filter/
├── mod.rs        # pub use; Document, parse(), serialize()
├── lines.rs      # Line{text,eol}, Eol, splitter, BOM
├── keywords.rs   # the ONE keyword table (static slice) + ValueShape/OpPolicy enums
├── tokenize.rs   # quote-aware scanning of one line: keyword, operator, values, trailing comment span
├── parse.rs      # classification + block grouping
├── disabled.rs   # commented-out block detection/validation, enable/disable
├── meta.rs       # `#ff:key value` run
├── edit.rs       # replace_block_lines, set_enabled, set_meta (minimal API proving PARSE-04)
└── diag.rs       # Diagnostic{line,severity,message}, LoadError
```
Add `pub mod filter;` to `src/lib.rs`. Nothing in generate/dynamic/overlay changes.

### Line splitting (PARSE-03 core) [ASSUMED design, standard technique]
- BOM: if input starts with `EF BB BF`, set `bom = true` and drop those 3 bytes from line 0.
- Split on `\n` only. If the segment before `\n` ends in `\r`, `eol = CrLf` and `text` excludes that `\r`; else `eol = Lf`. The last segment, if non-empty and not followed by `\n`, becomes a line with `eol = None`. If the input ends with `\n` there is no extra empty line. Empty input -> zero lines. A lone `\r` not before `\n` stays in `text` (verbatim).
- Use `str::split_inclusive('\n')`, not `lines()`. Take input as `&[u8]`, run `String::from_utf8` once; invalid UTF-8 returns `LoadError` (PoE filters are UTF-8; do not use lossy conversion because it would break byte identity).
- Never trim stored text. Classification works on borrowed trimmed views.
- Inserted/edited lines inherit the `eol` of the block's header line (or the nearest neighbour); if replacing the file's final line that had `eol = None`, keep `None` on the new last line.

### Line tokenizing (quote-aware)
- Leading whitespace (spaces/tabs), then a keyword = ASCII alphabetic run. Keyword match is **ASCII case-insensitive**; the stored raw text keeps original case.
- Operator may be glued to the keyword (`Rarity>=Magic`) or to the value (`HasExplicitMod >=1 "x"`, `>=1`). After the keyword, scan an operator from `== != <= >= = ! < >` (longest match first). A leading run of operator characters (`=<>!`) that is not a valid operator (e.g. `=>`) is an `unknown operator` error on that line.
- Values: `"..."` quoted (no escapes documented; a quote ends at the next `"`; unterminated = error), else a bare word up to whitespace or `#`.
- Comment starts at the first `#` **outside quotes**; keep the span (including preceding spaces) as trivia. Test `BaseType == "a # b" # real comment`.
- Do not validate ranges (volume 0-300, font 1-45) or names (BaseType/Class): the game warns on load; stale validation is a known anti-feature. Validate only arity/shape: e.g. `SetTextColor` needs 3-4 integers, `MinimapIcon` size then colour then shape (or `-1`), known colour/shape/enum words, numeric where Num. Over-strict shapes are the main false-positive risk; the real samples must produce zero Error diagnostics.

### Blocks and ownership
- Live header: first token (case-insens.) in `{Show, Hide, Minimal}` at line start after whitespace; remaining text must be empty or a `#` comment. If other tokens follow on the same line (the docs' compact `Hide ItemLevel < 85 Continue` form), keep the line verbatim, emit a Warning, and treat the extra text as opaque.
- Block body = following lines until the next header, `Import`, or EOF. Body lines are conditions, actions, `Continue` (a bare keyword line, kind Continue), opaque unknown keywords. The block's **owned range** ends at its last non-trivia body line; blank/comment lines after it are inter-block trivia, comments/blank lines between body lines are interior and belong to the block.
- Leading comments: the contiguous run of comment lines directly above the header (no blank line between) attach to the block. Detached comments and blank lines stay as standalone trivia items.
- Stable identity: `BlockId(u64)` from a document counter assigned at parse and never reused; moves/reorders (later phases) keep it. Do not use indices as identity.
- `Import "file" [Optional]` is a top-level item (Import node with path and optional flag); the file is never opened. `Continue` before any header or any condition/action outside a block: top-level opaque item with a Warning.
- Condition/action lines before the first header: ungrouped opaque items + Warning.

### Disabled (commented-out) blocks
- Candidate header: a comment line where, after removing the first `#` and leading whitespace, the first token is `Show|Hide|Minimal` (case-insens.) and the remainder is empty or starts with `#`. Real example: `#Show  # # %D9 $type->… !apex_stier` becomes `Show  # # %D9 …` after one `#` is removed, which is a valid header. Prose like `# Show items above 80` is NOT a header (a word follows the keyword) and stays a comment.
- Body: the contiguous following comment lines where removing one `#` leaves a line whose first token is a **known** keyword (condition/action/`Continue`). Real example: `#<tab>Continue`. The first line that does not fit (blank, live line, unknown first word, a new disabled header) ends the block. Using "known keyword" as the extension test keeps prose comments from being swallowed; mal-formed values inside known keywords make the block **invalid**, not non-blocks.
- Validity = re-run the same line parser on each uncommented inner line; collect diagnostics (`line N: unknown operator`). valid -> `enabled = false, can_enable = true`; invalid -> `can_enable = false, reasons: Vec<Diagnostic>`. Disabled-block diagnostics are Severity::Info/Warning, never Error (they are not live).
- Disable = prepend one `#` at column 0 of every owned line (before indent, matching the sample `#<tab>Continue`); enable = remove exactly the first `#` of every owned line. Round trip `enable(disable(b)) == b` must be tested. Leading metadata/comment lines are not touched.
- An enabled block with a malformed line: Error diagnostic tied to the line, block still present and byte-preserved.

### `#ff:` metadata
- A leading-comment line matching `^[ \t]*#ff:([A-Za-z0-9_-]+)(?:[ \t]+(.*))?$` (hand-scan, no regex) in the run directly above a block header is metadata. Expose `block.meta() -> Vec<(key, value)>`, `meta_get("name")`, `meta_get("style")`. Value is the text after one run of whitespace, with trailing whitespace trimmed in the view only. Lines remain in `lines` verbatim; setting a key rewrites only that one line, or inserts one new line immediately above the header (after existing `#ff:` lines) with the block's EOL.
- Not directly above a block (e.g. `#ff:economy-begin` separated by blank lines or followed by another section marker): remain ordinary comment lines, but expose a `Line::ff_marker()` view so Phase 18 can find them. Do not decide marker text now.
- Foreign tags are ordinary comments; do not interpret `$type->`, `%D4`, `!slug`, `# [[0100]]`.

### Minimal edit API (to prove PARSE-04; richer ops are Phase 18-20)
`Document::block(id)`, `set_enabled(id, bool) -> Result`, `replace_block_lines(id, Vec<String>)` (new lines get the header's EOL; leading comments and trailing trivia untouched), `set_meta(id, key, value)`, `serialize() -> String`/`to_bytes()`. After any edit, re-derive the affected block's classification only; other `BlockId`s remain valid.

### Anti-Patterns to Avoid
- Rebuilding text from parsed fields (loses indent, case, spacing, comments).
- `str::lines()`, `trim()` on stored text, `join("\n")`.
- Normalizing operator spacing or quoting when editing: change only the value token span; new lines copy the neighbour's indent.
- Failing the whole load on grammar errors; `Err` only for non-UTF-8.
- Treating a `Continue` inside a block as a block terminator or as a separate block.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Line termination handling | custom byte loop with edge cases | `str::split_inclusive('\n')` + strip trailing `\r` | Keeps terminator, handles final-line-without-newline |
| Atomic file writes | here | `tempfile` persist (Phase 18/19) | Out of scope this phase |
| Test fixture generation | mutating the real file by script | in-test `Vec<u8>` literals | git EOL attribute independence |

**Key insight:** the lossless property comes from storing raw lines, not from a clever parser. Keep the parser a read-only view.

## Runtime State Inventory
Omitted: greenfield module, no rename/migration.

## Common Pitfalls

### Pitfall 1: git rewrites fixture line endings (and the `-text` scope)
**What goes wrong:** `.gitattributes` has `* text=auto` and `*.filter text`; with `core.autocrlf=true` here, committed `.filter` fixtures become CRLF or LF depending on checkout, so a byte-identity test passes locally and fails elsewhere.
**How to avoid:** add `-text` for the new fixture. **Caution on the literal `*.filter -text` from CONTEXT:** it would also apply to `tests/fixtures/seed-universal.filter` and the parity fixtures, whose index content is LF but whose working copy is CRLF (measured: `i/lf w/crlf`), making git report them modified and changing what `include_str!`-based tests read. `artifact_guard` normalizes CRLF, so it would survive, but other `include_str!` unit tests may not. Recommended: scope it, e.g. `tests/fixtures/parser/*.filter -text`, put the FilterBlade copy there, and verify with `git check-attr text tests/fixtures/parser/filterblade.filter` and `git ls-files --eol`. If the planner prefers the global rule, it must also run `git add --renormalize` on the other fixtures and re-run the whole suite. Flagged in Open Questions.
**Warning signs:** `git status` shows existing fixtures modified after the change; `git ls-files --eol` shows `i/crlf` for the sample missing.
Also: copy the sample with `cp` (binary) from `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\filterblade.filter`, then confirm 258,094 bytes and 5312 CRLF.

### Pitfall 2: Seed fixture EOL depends on checkout
Tests on `seed-universal.filter` must assert `serialize(parse(x)) == x` on whatever bytes were read; never compare against a literal count of `\r`. Read with `fs::read` (bytes), not `read_to_string` + normalization.

### Pitfall 3: Reference files absent in CI
`reference-old-filters/` is git-ignored. Tests must `read_dir` it and skip with a printed note when missing (do not fail). They are LF, no final newline; a good natural test of that case.

### Pitfall 4: Prose comments mistaken for disabled blocks (and vice versa)
Use the header rule (nothing but a comment after the keyword) and the known-keyword body rule above. Add tests: `# Show items above this line`, `#Show`, `# Show`, `#Show  # # %D9 $type->x`, a disabled block with `#\tBaseType == "a # b"`, and an invalid disabled block with `#\tRarity =< Rare`.

### Pitfall 5: False-positive errors on real filters
Over-strict value-shape validation flags legitimate lines (`Class "x"` with no operator, `Rarity Normal Magic Rare`, `Sockets > 4`, `HasExplicitMod >=1 "…"`, `BaseType` bare). The gate: zero `Error` diagnostics across the FilterBlade fixture, seed fixture, and all reference files. Only Warnings (unknown keyword) allowed.

### Pitfall 6: Edited lines break EOL consistency
New lines must use the block's own EOL. For CRLF files an inserted `\n`-only line is a silent regression. Test: edit a block in a CRLF file and assert every `\n` in the output is preceded by `\r`.

### Pitfall 7: Final line without newline
Replacing or appending to the last block of an `eol=None` file: the previous last line needs an EOL added, and the new last line keeps `None`. Test both directions.

### Pitfall 8: Case-insensitive keywords
`show`, `SHOW`, `rarity>=magic` are valid. Raw text preserved; lookup lower-cases ASCII. [ASSUMED: case-insensitivity per `.planning/research/PITFALLS.md` #4, the official page does not state it; the parser should accept both anyway.]

## Code Examples

### Splitter (sketch; std only)
```rust
pub enum Eol { Lf, CrLf, None }
pub struct Line { pub text: String, pub eol: Eol }

pub fn split(input: &str) -> (bool, Vec<Line>) {
    let (bom, rest) = match input.strip_prefix('\u{feff}') { Some(r) => (true, r), None => (false, input) };
    let lines = rest.split_inclusive('\n').map(|seg| {
        if let Some(s) = seg.strip_suffix("\r\n") { Line { text: s.into(), eol: Eol::CrLf } }
        else if let Some(s) = seg.strip_suffix('\n') { Line { text: s.into(), eol: Eol::Lf } }
        else { Line { text: seg.into(), eol: Eol::None } }
    }).collect();
    (bom, lines)
}
```
`strip_prefix('\u{feff}')` is exactly EF BB BF in UTF-8. Serialization re-adds it.

### Byte-diff helper for test failures
Return the first differing offset plus a 40-byte window from each side; `assert_eq!` on 258 KB strings is unreadable.

### Keyword table shape
```rust
pub enum Kind { Block, Condition, Action, Directive }
pub enum Ops { None, Compare, EqOnly }       // table records the policy; parser only records the operator verbatim
pub enum Value { Bool, Int, IntRange, QuotedList, Enum(&'static [&'static str]), Rarity, Sockets,
                 CountThenNames, Rgba, SoundId, File, MinimapIcon, PlayEffect, None, BoolOrName }
pub struct Keyword { pub name: &'static str, pub kind: Kind, pub ops: Ops, pub value: Value }
pub static KEYWORDS: &[Keyword] = &[ /* ~75 entries */ ];
pub fn lookup(name: &str) -> Option<&'static Keyword> { KEYWORDS.iter().find(|k| k.name.eq_ignore_ascii_case(name)) }
```
Linear search over ~75 entries is fine for 5k lines.

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Parse to AST, re-emit | Raw lines + index layer | The only way to meet byte identity |
| PoE1 docs treated as the spec | One page covers PoE1 + PoE2 with tags | Accept the whole vocabulary, not a PoE2 subset |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Keywords and operators are case-insensitive / spacing-insensitive in the game parser | Pitfall 8 | Parser would be slightly more permissive than the game; harmless |
| A2 | Lone `\r` is not a line terminator for the game | Line splitting | Preserved verbatim anyway; only classification differs |
| A3 | `rowan`/parser crates add nothing over a hand line scanner | Alternatives | Low; stylistic |
| A4 | Compact single-line form `Hide ItemLevel < 85 Continue` from the docs is not used by real files | Blocks | Parser warns and preserves it; worst case stays opaque |
| A5 | Redistributing the NeverSink/FilterBlade sample as a test fixture is acceptable (licence) | Pitfall 1 | Repo licensing; user to confirm |
| A6 | `dynamic.filter` in the game folder (LF, no BOM) is representative of FerrisFilter output; the generator emits CRLF or LF depending on the `generate` path, so the test should generate fresh output | Test material | Low |

## Open Questions

1. **`.gitattributes` scope.** CONTEXT says `*.filter -text`; measured side effects on existing fixtures (see Pitfall 1). Recommendation: narrow path scope for the new fixture directory; if global, renormalize and re-run the suite. Needs a planner/user decision before the first commit.
2. **Metadata line with leading whitespace (`  #ff:name`).** Recommend accepting leading spaces/tabs. Phase 18 must fix the writer's exact form.
3. **Where block-owned trailing comments live** when a comment run sits between two blocks and is both "trailing the previous" and "leading the next": per CONTEXT, contiguous-above wins; everything else is trivia. Already specified above.
4. **Generated dynamic filter in tests:** build via `compile`/`emitter::generate_as` on `dynamic_base.json` + seed prices into a `String`, then round-trip it; exact call chosen by the planner (`src/compile.rs`, `src/emitter.rs`).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rustc (edition 2024) | build/test | ✓ (project builds, `target/` exists) | — | — |
| FilterBlade sample | fixture | ✓ `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\filterblade.filter`, 258,094 B | — | — |
| `reference-old-filters/*.filter` | corpus test | ✓ locally (11 files, 78-296 KB, LF, no final newline; also `#end_endgame.filter#` editor backup) | — | skip when absent |
| python | n/a | ✗ (Windows store stub) | — | use Rust tests / shell tools |

Corpus note: `reference-old-filters/#end_endgame.filter#` is an editor autosave with a `.filter#` extension; glob `*.filter` excludes it.

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; this section is included because the task explicitly requested a test strategy.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (unit tests in `src/filter/*`, integration in `tests/filter_roundtrip.rs`) |
| Config file | none |
| Quick run command | `cargo test --lib filter::` |
| Full suite command | `cargo test` |

Existing convention: integration tests in `tests/*.rs` read fixtures via `env!("CARGO_MANIFEST_DIR")`, plus `include_str!` unit tests; do not create fixtures by running the CLI from the repo root.

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARSE-01 | FilterBlade, reference, seed, generated parse with zero Error diagnostics; unknown keyword -> Warning + opaque | integration | `cargo test --test filter_roundtrip corpus` | Wave 0 |
| PARSE-02 | every table keyword parses; table names unique; sample keyword set subset of table; Continue/Import/Minimal | unit | `cargo test --lib filter::keywords` | Wave 0 |
| PARSE-03 | byte identity: FilterBlade fixture; seed; generated; reference (skip if absent); synthetic LF/CRLF/mixed/BOM/no-final-newline/tabs/trailing ws/`#` in quotes/`>=1`/empty file/only-BOM | integration + unit | `cargo test --test filter_roundtrip roundtrip` | Wave 0 |
| PARSE-04 | edit one block (set_enabled, replace lines, set_meta) -> diff limited to that range, other bytes equal; CRLF preserved; last-line-no-EOL case; enable(disable(b)) == b | integration | `cargo test --test filter_roundtrip edit` | Wave 0 |

Additional unit tests: disabled-block classification (valid/invalid/prose), `#ff:` extraction, leading-comment attachment, BlockId stability after an edit, exact sample counts (447 Show, 13 Hide, 40 Continue live; disabled blocks counted separately).

### Sampling Rate
- Per task commit: `cargo test --lib filter::` (fast, no 258 KB parse needed beyond the corpus test)
- Per wave merge: `cargo test --test filter_roundtrip`
- Phase gate: full `cargo test` green, plus `git status` shows no fixture drift (`git ls-files --eol` confirms the sample stays `i/crlf`)

### Wave 0 Gaps
- [ ] `tests/fixtures/parser/filterblade.filter` (binary copy, 258,094 bytes) + `.gitattributes` line
- [ ] `tests/filter_roundtrip.rs`: helpers `assert_bytes_eq` (first-diff offset), `corpus()` iterator that skips missing `reference-old-filters/`
- [ ] Synthetic cases as in-test `Vec<u8>`/`&[u8]` literals (never files, so git EOL handling cannot affect them)
- [ ] `src/filter/` skeleton and `pub mod filter;` in `src/lib.rs`

## Security Domain

Input is a local user file; no network, auth, or crypto. ASVS V5 (input validation) applies: the parser must not panic on arbitrary input (slicing on char boundaries, no `unwrap` on user text, unterminated quotes, empty lines, lone `\r`, only-BOM, invalid UTF-8 -> typed error). Mitigation: operate on `char_indices`/ASCII-checked byte offsets, and include a small "garbage input never panics" test over truncated prefixes of the sample (deterministic loop; proptest is excluded by CONTEXT). `Import` paths are stored, never opened (no path traversal surface). Other ASVS categories: not applicable.

## Project Constraints (from CLAUDE.md)
- The `generate` path, `config.json`, `seed-universal.filter`, `artifact_guard` must remain untouched/green.
- `Show # <name>` emission and `Continue`-aware ordering semantics are existing domain facts; the parser must preserve operators (`==` exact vs bare substring) exactly.
- Keep `CHANGELOG.md` `[Unreleased]` current as work lands (add a parser entry); update on milestone completion.
- Typed errors with context (miette on CLI side), library APIs return values instead of printing.
- Generate fixtures hermetically, not via `cargo run` from the repo root.

## Sources

### Primary (HIGH)
- https://www.pathofexile.com/item-filter/about (fetched 2026-10-12): block types, operators, conditions, actions, value enums
- Local measurement: `filterblade.filter` (CRLF counts, keyword histogram, disabled-block shapes such as `#Show  # # %D9 …` / `#<tab>Continue`), `reference-old-filters/*`, `tests/fixtures/*.filter` (`git ls-files --eol`), `.gitattributes`, `Cargo.toml`, `src/lib.rs`, `tests/cli_generate.rs`

### Secondary (MEDIUM)
- `.planning/research/PITFALLS.md` #1-5, `.planning/research/FEATURES.md` A1/A2 (prior milestone research, consistent with the fetch)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (std only, no new deps)
- Architecture: HIGH (raw lines + index is the direct consequence of the locked decisions)
- Pitfalls: HIGH for EOL/git, MEDIUM for disabled-block heuristics (rule chosen here; validate against the sample's ~13 + commented blocks)

**Research date:** 2026-10-12
**Valid until:** 30 days (grammar page changes per patch; re-check the keyword table at implementation)
