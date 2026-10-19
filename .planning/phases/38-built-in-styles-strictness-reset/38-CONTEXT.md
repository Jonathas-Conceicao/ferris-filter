# Phase 38: Built-in Styles, Strictness & Reset - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The shipped default styles become the author's reworked built-ins, a one-line Strictness row on the
Config tab sets the built-in tier styles' visibility, styles and tiers can be reset to their
defaults, and the duplicated minimap-icon tooltip is fixed. Covers BSTY-01..03, STRICT-01..04,
RESET-01..02, FIX-01. Depends on Phase 36's embedded defaults.

</domain>

<decisions>
## Implementation Decisions

### The new built-in styles (supersedes the BSTY-01 wording about shipped extras)
- `styles-default.json` becomes exactly the **built-ins** from the author's snapshot
  `.planning/research/author-styles-2026-10-17.json`: `S`, `A`, `B`, `C`, `D`, `E`, `F`,
  `unchanged`, `hidden`, with their values and ids as in the snapshot, in the snapshot's order.
- **No custom styles ship.** `Recycle`, `kite`, `exceptional`, `Build Items` and `White bases` are
  the author's own custom styles and are not in the defaults.
- None of the shipped styles uses a custom sound file (the snapshot's built-ins use built-in alerts
  only).
- CLAUDE.md's style and tier tables are updated to the new values and to "no custom styles ship".
- **CLI `generate` is not reworked** (author: no energy on `universal.filter`; it will be reworked
  later). To keep it and its tests untouched: today's `styles-default.json` is frozen as a test
  fixture (`tests/fixtures/seed-styles.json` or similar) that the generate tests and
  `artifact_guard` read, so the committed fixtures stay byte-identical and no re-baseline is needed.
  `config.json` is not changed. Known consequence, documented in CLAUDE.md: `generate` on a checkout
  with no live `styles.json` reports the unknown style (`Recycle` / `kite` / `exceptional`); the
  author's own `generate` keeps working through their live `styles.json`.
- A user's own `styles.json` is kept as it is; built-in ids stay `builtin:S`..`builtin:F`.

### Strictness row
- At the top of the Styles section, above the Built-in table.
- **One line** is the hard requirement. Seven buttons: `Custom | Flat Broke | Scrounging | Getting By
  | Comfy | Loaded | Div Farmer`; two-line (stacked) button captions are fine if needed to fit the
  660pt window. Make it look good with the skin (segmented look as Show | Hide); the author will
  give feedback later. Each level's tooltip names what it hides ("Hides tiers E and F"). Custom is
  leftmost and display-only (never clickable, dim caption).
- Levels hide nothing, F, E-F, D-F, C-F, B-F of `builtin:S`..`builtin:F` (matched by id, so a rename
  does not matter). A click changes the pending style set like switch clicks (Config tab gets
  ` *`, Revert undoes, Apply writes). The highlighted button is derived from the current S-F
  visibility; any other pattern shows Custom.
- Non-tier built-ins and custom styles are never touched. A tooltip says rules using tier styles
  (user rules included) are affected.
- A fresh install (all defaults shown) starts on Flat Broke.

### Reset to defaults
- A Reset to defaults button at the right of the Styles heading (Config tab) and of the Tiers
  heading (Economy tab); greyed when the pending set already equals the defaults.
- Popups "Reset styles to defaults?" / "Reset tiers to defaults?" with a red Reset and Keep (the
  skinned `ff_modal` pattern; Escape / outside click = Keep).
- Styles reset: the built-ins come back to their default values and order (matched by id; a renamed
  built-in gets its default name back). Every custom style is kept, listed after the built-ins.
  Strictness then reads Flat Broke.
- Tiers reset: the default cutoffs. Both are one pending edit (Revert undoes, Apply writes).

### Tooltip fix (FIX-01)
- Find why a minimap icon tile's tooltip is drawn twice (likely both the tile and its parent add
  hover text) and keep exactly one, everywhere the tiles are drawn (style editor, rule chips,
  pickers).

### Claude's Discretion
- Widget composition of the strictness row (caption wrapping, font size), where reset helpers live
  in `config_state.rs`, the frozen fixture's file name.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `skin::editor` segmented switch (Show | Hide), `skin::popup::ff_modal`, the Built-in / Custom style
  tables in `src/overlay/config_view.rs`, pending style / tier state in
  `src/overlay/config_state.rs`.
- Phase 36's embedded defaults (`include_str!` of `styles-default.json` / `tiers-default.json`) are
  the reset source.

### Established Patterns
- Config edits stay in memory until Apply; Revert per tab asks first; tab label ` *` for pending
  edits.
- `generate` tests copy `styles-default.json` into a temp dir (`tests/cli_generate.rs:36, 74, 228,
  295-324, 512, 647, 688, 731`) and `tests/cli_errors.rs:40-43` reads it: these are the call sites
  to point at the frozen fixture.

### Integration Points
- `config.json` rules use `Recycle` (2), `exceptional` (3), `kite` (2), `hidden`, `unchanged`.
- Minimap icon tiles: `skin::editor` minimap tiles, `skin::minimap`, rule-row style chips.

</code_context>

<specifics>
## Specific Ideas

- Snapshot values to ship (built-ins only): S/A white background `248 248 242`, font 37, Star
  minimap (S size 0 Pink, A size 1 Purple), alert 6 vol 300, `enableDropSound`; B font 37, Red Star,
  alert 2 vol 200, `enableDropSound`; C cyan text, Cyan Star, Yellow beam, alert 2; D green text,
  Green Star, White beam; E yellow text font 23; F white text font 20; `unchanged`; `hidden` (Hide,
  font 21, `disableDropSound`).

</specifics>

<deferred>
## Deferred Ideas

- Reworking CLI `generate` / `config.json` against the new defaults (author: later).

</deferred>
