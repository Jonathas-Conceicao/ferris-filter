# Phase 43: Style Alpha - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

Style colours (text, border, background) gain alpha, end to end: the `styles.json` model and loader,
the CONFIG style colour picker, the label preview and style table, and every place a style is
written to a filter (`generate`, the economy section, the styled-rule rewrite on Apply). Requirements
ALPHA-01..04. Auto Detect (Phase 44) builds on the final colour type.

</domain>

<decisions>
## Implementation Decisions

### Storage format (author, 2026-10-23)
- Reading: a colour array may have 3 or 4 values. A 3-value colour (older files) is read as alpha
  255, as today.
- Writing: from this version on, colours are **always saved with alpha present** (4 values), even
  when alpha is 255. (The author overrode the "write alpha only when not 255" proposal.)
- A `styles.json` with 3-value colours loads **without showing as an edit** (no ` *`, no unapplied
  indicator, no Quit popup), like the id migration: it is rewritten in the 4-value form by the next
  Apply that writes `styles.json` for another reason. `styles.json` is still written only when the
  CONFIG tab has edits.
- The committed `styles-default.json` moves to the 4-value form for consistency (it is the embedded
  default and the Reset to defaults source). `tests/fixtures/seed-styles.json` stays frozen as is
  (3-value) — it is the frozen pre-v0.7 spec and keeps proving the 3-value read path.
- Filter output is unchanged for alpha 255: the emitter already writes `R G B 255`, so `generate`
  output, the fixtures and `artifact_guard` stay byte-identical. The unique bronze background
  (`175 96 37 240`) stays a pricer/emitter constant, not a style.

### Editor
- The CONFIG style colour picker gets the same Alpha slider the RULES colour picker already has
  (`src/skin/colour_picker.rs` already has an `alpha` mode), for all three slots. The CLAUDE.md note
  "styles store RGB ... the style colour picker has no alpha" becomes false and must be updated.
- Reset to defaults, strictness and the "greyed at the defaults" check compare colours including
  alpha.

### Previews
- The label preview and the style table preview paint translucent colours over the dark panel
  ground (roughly the in-game look). No checkerboard.

### Claude's Discretion
- The Rust colour type (e.g. a `Colour { rgb, alpha }` newtype or `[u8; 4]`) and its serde.
- Whether Custom rule colour editing (RULES style lines) needs any change beyond sharing the type.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Style` in `src/models/config.rs` (`text` / `border` / `background: Option<[u8; 3]>`).
- `emitter::style_action_lines()` — the one style-action emitter used by `generate`, the economy
  section and the styled-rule rewrite (`src/styled.rs`).
- `src/skin/colour_picker.rs` — alpha-capable picker (RULES path) and the opaque style path.
- `src/skin/editor.rs` — colour cards and the item-label preview.

### Established Patterns
- `styles.json` loads leniently and migrates silently (ids assigned at load, persisted by the next
  Apply without showing as an edit) — follow the same pattern for 3 → 4 values.
- Style table and editor live in `src/overlay/config_view.rs` / `config_state.rs`.

### Integration Points
- `src/loader.rs` (styles loading / validation), `src/defaults.rs` (embedded defaults),
  `src/overlay/config_state.rs` (pending styles, equality with defaults), Apply in
  `src/overlay/apply_all.rs`.
- Tests: `tests/` integration suites, `artifact_guard`, styles round-trip tests.

</code_context>

<specifics>
## Specific Ideas

- Research: `.planning/research/SUMMARY.md` (pitfall 1 — byte changes; equality traps feed Phase 44).
- CLAUDE.md's `styles.json` row and the Config-tab colour picker note must be updated to the new
  format when this lands.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
