# Phase 16: Lossless Filter Parser - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

Library-level parsing and serialization of any PoE2 item filter written in GGG's filter grammar
(https://www.pathofexile.com/item-filter/about), held in a representation that writes back
byte-identically when nothing was edited, and changes only an edited block's lines when something
was. Requirements: PARSE-01..04. No overlay/UI work and no store changes in this phase. Those are
Phases 18-20, which build on this.

</domain>

<decisions>
## Implementation Decisions

### Representation
- **Line-preserving model.** Every line keeps its exact original text, including its own line
  ending (CRLF/LF/none) and indentation. A BOM and the presence or absence of a final newline are
  recorded. Parsed values are a lookup layer over those raw lines. Serializing an unedited file
  emits the raw lines verbatim, and editing a block re-renders only the lines that block owns.
  Never use `str::lines()` (it drops terminators), and never `trim()` a stored raw line.
- **Unedited bytes never change.** No pretty-printing or normalization on save (anti-feature).
- **Comments are just comments.** Comment lines directly above a block attach to that block, so
  they move with it when it is reordered later. Every other comment line stays where it is as
  standalone content.
- **No tool-specific comment parsing.** Do not interpret FilterBlade/NeverSink tags (`$type->…`,
  `$tier->…`, `%D4`, `!slug`) or poe2filter.com conventions (`# [[0100]]` banners, trailing
  `[[Config Start]]` JSON). They are ordinary comments. Only GGG's grammar is parsed.

### FerrisFilter's own metadata comments (cross-cutting; defined here, used by Phases 18-20)
- FerrisFilter stores its own per-block configuration in a **header-style comment run directly
  above the block**, one `#ff:<key> <value>` line per setting. Examples: `#ff:name Exceptional 3
  socks`, `#ff:style S`. The style follows the game's online-filter header (`#name:filterblade`).
- The parser recognizes this run and exposes it as structured metadata on the block (name, style,
  and later keys), while still preserving the bytes exactly.
- Section-level markers use the same family, e.g. `#ff:economy-begin` / `#ff:economy-end`. Phase
  18 fixes the exact marker text.
- Foreign header comments (`Show # $type->currency …`) are **not** a name. Only `#ff:name` is.
  Exact key spelling is at the planner's discretion within this shape.

### Commented-out blocks (disabled rules)
- A commented-out block is a **disabled rule**: a `#Show`/`#Hide`/`#Minimal` header line followed
  by `#`-prefixed condition/action lines. Disabling adds one `#` to each of the block's lines, and
  enabling removes exactly one.
- **A commented-out block may not be valid grammar** (real-world filters contain stale or
  hand-mangled commented blocks). The parser must try to parse its inner lines and classify it:
  - **valid**: a disabled rule that can be enabled and edited;
  - **invalid**: a disabled rule kept byte-for-byte. It can be shown, moved and deleted, and the
    reason it can't be enabled is reported (e.g. "line 412: unknown operator").
  Enabling is only possible when the inner lines parse cleanly. Comment text that doesn't look like
  a block at all stays an ordinary comment.
- **Enabled rules must always be grammatically correct.** Nothing the tool produces may yield an
  invalid enabled block. An enabled block that fails to parse in a loaded file is surfaced as an
  error tied to its line. A "working filter" shouldn't contain one, but the tool must not crash or
  silently rewrite it. It is flagged, and Apply is refused until it is fixed or disabled (later
  phases). The error must be visible, never silent.

### Grammar coverage
- **One keyword table** is the single source of truth for the grammar. It lists every block type,
  condition and action from GGG's documentation page: name, kind (condition/action), allowed
  operators, and value shape (number, range, quoted list, enum such as colour or shape, RGBA,
  boolean, socket string like `5GGG`, sound id/volume, file path, …). It covers `Continue`,
  `Import [Optional]` and `Minimal`. Re-fetch the official page during research and check the
  table against it. The FilterBlade sample is test material only, not a source of grammar.
- A keyword not in the table (e.g. one GGG adds after this ships) is kept as an opaque line with a
  warning, and its block remains otherwise usable. It must never stop a filter from loading.

### Test material
- Commit a copy of the FilterBlade/NeverSink filter
  (`…/Path of Exile 2/filterblade.filter`, 258 KB, CRLF, tab-indented) as a test fixture. Add
  `*.filter -text` to `.gitattributes` so git never rewrites fixture line endings.
- The phase is done when `serialize(parse(x)) == x` byte-for-byte holds for: that fixture,
  `reference-old-filters/*.filter` (git-ignored locally, so tolerate absence),
  `tests/fixtures/seed-universal.filter`, a generated dynamic filter, and synthetic edge cases (LF,
  CRLF, mixed endings, BOM, no final newline, tabs, trailing whitespace, `#` inside quoted values,
  operators without spaces like `>=1`).
- A test edits one block and asserts that every other byte is unchanged.
- **No CLI `check` command** (author chose library and tests only). No proptest.

### Claude's Discretion
- Module layout (suggested `src/filter/`), type names, error type, and how block identity is
  represented for later phases (stable IDs, not indices).
- How permissive operator/value parsing is (e.g. `Rarity Normal Magic Rare` bare multi-value,
  `Class "x"` substring vs `Class == "x"`); it must match the documented grammar and preserve
  operators exactly.
- Whether parsed values are computed eagerly or lazily.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/emitter.rs` writes the same grammar today (Show/Hide headers, `Class`, `BaseType ==`,
  `SetTextColor`, `MinimapIcon`, `PlayEffect`, `CustomAlertSound`). It is the reference for how
  FerrisFilter renders lines, which matters when Phase 20 renders edited lines.
- `src/models/config.rs` has `FilterColor`, `MinimapShape`, `Rarity`/`RarityOp` enums that the
  keyword table's value enums can mirror or reuse.

### Established Patterns
- Typed errors with field paths (`miette` diagnostics on the CLI side), and `compile::` as a
  library API returning values rather than printing (CORE-04).
- Fixtures: `include_str!` unit tests plus insta snapshots. Generate fixtures hermetically, never by
  `cargo run` from the repo root.

### Integration Points
- New module only. Nothing in `generate`, `dynamic.rs` or the overlay changes this phase.
- `seed-universal.filter` and `artifact_guard` must stay untouched.

</code_context>

<specifics>
## Specific Ideas

- Real sample facts (research, measured): 5312 lines, pure CRLF, final `\r\n`, tab-indented
  bodies, 447 Show / 13 Hide / 40 `Continue`, many commented-out blocks such as
  `#Show  # # %D9 $type->…`, and `HasExplicitMod >=1 "…"` with no space after the operator.
- `.planning/research/PITFALLS.md` §1-5 and `FEATURES.md` §A1 (vocabulary table) are the
  detailed inputs.

</specifics>

<deferred>
## Deferred Ideas

- `ff-cli check <file>` diagnostic command (author: not now).
- Raw-text block editing: backlog 999.10.

</deferred>
