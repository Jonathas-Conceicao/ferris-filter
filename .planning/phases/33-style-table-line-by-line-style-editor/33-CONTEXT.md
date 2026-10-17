# Phase 33: Style Table & Line-by-Line Style Editor - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

The CONFIG tab's styles: an aligned table with a column header, a rule-editor-like inline style
editor, and a skinned colour-picker popup shared with the rule editor's colour actions. Keeps v0.5's
style model (stable ids, in-memory edits, one all-or-none Apply, Revert). TIERS moves to ECONOMY in
Phase 34 (leave it in CONFIG here).

</domain>

<decisions>
## Implementation Decisions

### Table
- One header row: Visibility / Name / Icon / Preview / Order above the style rows; Built-in and
  Custom groups keep their own headings (the Custom heading carries + Add style).
- Built-in rows show the Order arrows greyed out / disabled so the columns align; custom rows' arrows
  work.
- The textual Show/Hide caption before the icon is dropped (the visibility switch already says it).
- Icon column: the style's minimap icon (game art from `assets/poe2/minimap/icons.png`) at small
  size, blank when none.
- Built-in names are plain text; custom names are an inline text field (rename keeps id links).

### Style editor (line by line, like the rule editor)
- No boxed sub-panels / frames inside the editor. Rows: `Visibility  <Show><Hide>` on one line;
  Label: preview with Text / Border / Background swatches below it; Font size; Minimap: size pills
  (None / Large / Medium / Small), 11 fixed colours, 12 game shapes; Beam: none + colours, Temp;
  Sound: dropdown (None / alert 1-16 / custom file) + volume. The prototype pieces in
  `src/skin/editor.rs` and `src/skin/minimap.rs` are the starting point, minus the panel frames.
- Delete (custom only) sits at the bottom of the expanded editor and asks for confirmation in the
  skinned popup, showing how many rules use the style and warning when `config.json` references it.

### Colour picker (author: "for you to handle ... match the new art style")
- Clicking a swatch opens a popup in the game frame (9slicemenu art): saturation/brightness square +
  hue bar, hex field, R / G / B fields (no CMYK — filters are RGB), an alpha slider only when the
  target supports alpha (rule colour actions; styles are opaque), one row of the 9 Dracula colours as
  quick picks, and a "none" option where the slot can be empty. The same popup serves the RULES
  editor's colour actions (replacing the old picker).

### Claude's Discretion
- Popup placement, live-preview vs confirm semantics, keyboard handling.

</decisions>

<code_context>
## Existing Code Insights

- `src/overlay/config_view.rs`, `config_state.rs`: current CONFIG tab and style edit state (ids,
  rename, reorder, delete with usage counts, config.json reference warning).
- Prototype: `src/skin/editor.rs`, `src/skin/minimap.rs`; minimap atlas
  `assets/poe2/minimap/icons.png` (12 shapes x 11 colours, Large only; Medium / Small scaled).
- Rule colour actions are edited in `src/overlay/body.rs`.

</code_context>

<specifics>
## Specific Ideas

- Layout reference: Scalpel editor `C:\Users\jonat\Downloads\poe2_print010.png` (content), drawn in
  the PoE2 look; prototype screenshot `ff-proto-style-editor-kite-status-warning.png` in
  `.planning/quick/261016-1de-poe2-prototype-tab-layout-style-editor-a/`.

</specifics>

<deferred>
## Deferred Ideas

- Sound preview play icon — backlog 999.15.

</deferred>
