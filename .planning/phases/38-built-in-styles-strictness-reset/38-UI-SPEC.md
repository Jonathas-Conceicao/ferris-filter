---
phase: 38
slug: built-in-styles-strictness-reset
status: approved
reviewed_at: 2026-10-18
shadcn_initialized: false
preset: none
created: 2026-10-18
---

# Phase 38 - UI Design Contract

> Visual and interaction contract. FerrisFilter is a Rust egui overlay with the PoE2 game skin
> (`src/skin/`); there is no web design system. Every value below is a skin widget, palette
> constant or font from the existing code. Sources: 38-CONTEXT.md (locked), CLAUDE.md overlay
> notes, `src/overlay/config_view.rs`, `src/skin/{editor,widgets,popup,palette,fonts}.rs`,
> memory note "overlay UI not texty".

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui + hand-built skin, not shadcn) |
| Preset | not applicable |
| Component library | `src/skin/` `ff_*` widgets over egui |
| Icon library | game art atlas (`skin::minimap`), `skin::icons` painted flags |
| Font | Fontin Regular (`fonts::regular`) and Fontin SmallCaps (`fonts::small_caps`) |

Registry safety: not applicable (no shadcn, no third-party registries).

---

## Scope of UI change

| Surface | Change |
|---------|--------|
| Config tab, Styles section | New **Strictness row** between the "Styles" heading and the "Built-in" group heading. New **Reset to defaults** button at the right of the "Styles" heading. |
| Economy tab, Tiers section | New **Reset to defaults** button at the right of the "Tiers" heading (`shell.rs` `w::ff_section(ui, skin, "Tiers")`). |
| Popups | Two new `ff_modal` popups: **Reset styles to defaults?**, **Reset tiers to defaults?**. |
| Minimap icon tiles | FIX-01: one tooltip per tile (no layout change). |

No new tab, no new section, no new colours, no new fonts.

Composition with Phase 37 (parallel, edits the Filter section in `config_view.rs`): Phase 38 touches
only `show_styles` (insert the row right after its `section_title(ui, ctx, "Styles")` call, and
the heading button on that same line) and a new helper for the row. It must not edit `show_filter`,
`show_sections` or the Filter section's layout; the Filter section stays above, then the existing
`ui.add_space(16.0)`, then Styles. The Tiers button lives in `shell.rs`/`app.rs` Economy wiring,
not in the Filter section.

---

## Spacing Scale

The overlay uses egui points. Values reused from the existing code (all multiples of 4 except the
noted existing exceptions):

| Token | Value | Usage in this phase |
|-------|-------|---------------------|
| xs | 4pt | Gap between the Styles heading and the strictness row; item spacing inside the row's caption stack |
| sm | 8pt | Gap between the strictness row and the Built-in heading; gap between the heading button and the right edge (heading has 6pt text inset, existing) |
| md | 16pt | Existing gap between the Filter and Styles sections (unchanged) |

Exceptions: the segmented/strictness row height is 40pt (one step above `SEGMENT_HEIGHT` 30pt, to
fit two stacked caption lines); heading text inset 6pt and Fontin metrics are existing skin values,
not introduced here.

Row placement: `ui.add_space(4.0)` after the Styles heading, the row, `ui.add_space(8.0)`, then
the existing "Built-in" group heading. No frame around the row (the segmented art is its own
frame, as the Show | Hide switch).

---

## Typography

Reuses existing fonts and sizes only. No new size is introduced.

| Role | Font / Size | Usage here |
|------|-------------|------------|
| Section heading | `small_caps(21)` (`ff_section`) | "Styles" / "Tiers" (unchanged) |
| Strictness caption | `small_caps(14)`, shrink to 13 then 12 only if a caption would overflow its segment | Seven segment captions |
| Button caption | `ff_button` caption (existing) | Reset to defaults |
| Popup title/body | `ff_modal` (existing) | Reset popups |
| Tooltip | egui default hover text | All tooltips |

Weights: the skin has no weights; hierarchy is small caps vs regular only.

---

## Color

Existing palette constants only (`src/skin/palette.rs`).

| Role | Constant | Usage here |
|------|----------|------------|
| Dominant | `GROUND` / `PANEL` | window surface |
| Secondary | `ROW_EVEN` / `ROW_ODD` | table rows (unchanged) |
| Accent (gold) | `HEADING` `E6D3A6` for the **selected** strictness caption; `LABEL` for hover; `DISABLED` for idle | See below |
| Destructive | popup `danger: true` (red caption, existing) | Reset button inside the two popups only |

Strictness segment colours (same logic as `ff_segmented`):
- Selected: lit button art (state 1, white tint), caption `HEADING`.
- Idle: dim base art (state 0, grey 170 tint), caption `DISABLED`; hover caption `LABEL`.
- **Custom** segment: never lit-on-hover, never clickable, caption `DISABLED` always (dimmer than
  idle: draw at `DISABLED` with no hover change and no pointer cursor change). It is lit like any
  selected segment (art state 1, `HEADING` caption) only when it is the derived current level.
- Dividers: 1pt black alpha 160 line inset 7pt top and bottom (existing).

Accent reserved for: the selected strictness segment only. The Reset to defaults button is a plain
`ff_button` (not gold-highlighted, not red); red appears only on the confirm button inside the
popups.

---

## Strictness row (STRICT-01..04)

### Layout (hard requirement: one line in the 660pt window)

- One row, full available width of the Styles section, height 40pt, seven segments left to right:
  `Custom | Flat Broke | Scrounging | Getting By | Comfy | Loaded | Div Farmer`.
- Implementation: a new `skin::editor::ff_segmented_levels` (or a parameterised extension of
  `ff_segmented`) with the same art, dividers, hover and `WidgetInfo` radio semantics as
  `ff_segmented`. It must not change `ff_segmented`'s existing behaviour for Show | Hide.
- Segment widths: Custom is fixed 72pt; the six levels share the remaining width equally
  (about 88pt each at a ~600pt content width; the executor computes from `available_width()`).
  This makes Custom visually separate and lets the level names have the room.
- Captions: single-word names (`Scrounging`, `Comfy`, `Loaded`, `Custom`) on one line, vertically
  centred. Two-word names are **stacked on two lines** at the word break, centred, 14pt, 14pt
  line step: `Flat / Broke`, `Getting / By`, `Div / Farmer`. All captions are painted in small
  caps (case as typed in copy below; small caps renders the capitals).
- Overflow rule: if a caption's galley is wider than `segment_width - 8pt`, drop the font to 13pt,
  then 12pt; never truncate with an ellipsis and never wrap the row. A unit test asserts the widest
  caption (`Scrounging`) fits its segment at the 660pt window width.
- Left edge of the first segment and right edge of the last align with the table below (same
  content rect as the Built-in table header).

### Behaviour

| Element | Contract |
|---------|----------|
| Highlighted segment | Derived every frame from the pending style set: the visibility (Show/Hide) of `builtin:S`..`builtin:F` (matched by id). Hide-sets: Flat Broke = none hidden; Scrounging = F; Getting By = E, F; Comfy = D, E, F; Loaded = C, D, E, F; Div Farmer = B, C, D, E, F. Any other pattern (including S or A hidden, or a gap) highlights **Custom**. |
| Click on a level | Emits one `ConfigAction` that sets the visibility of the six tier styles at once (a single pending edit; Config tab gets ` *`, Revert undoes it, Apply writes). Clicking the already-highlighted level does nothing. |
| Click on Custom | Nothing: no action, no state change, no hover highlight, no focus ring. |
| Fresh install | All default styles shown reads Flat Broke. |
| Out of reach | Non-tier built-ins (`unchanged`, `hidden`) and all custom styles are never changed. |
| No styles / load error | Row is not drawn when the Styles section shows "No styles loaded". If a built-in tier id is missing from `styles.json`, the row is drawn with Custom highlighted and the level buttons still work on the tier styles that exist. |
| Per-row switches | The Built-in table's Visibility switches stay as they are; flipping one makes the row re-derive (usually to Custom). |

### Tooltips (the only explanatory text; no caption paragraph under the row)

Each level segment: line 1 what it hides, line 2 the shared note.

| Segment | Tooltip |
|---------|---------|
| Custom | `Your own mix of shown and hidden tiers. Pick a level to replace it.` |
| Flat Broke | `Hides nothing. Every tier shows.` + note |
| Scrounging | `Hides tier F.` + note |
| Getting By | `Hides tiers E and F.` + note |
| Comfy | `Hides tiers D to F.` + note |
| Loaded | `Hides tiers C to F.` + note |
| Div Farmer | `Hides tiers B to F.` + note |

Shared note (second paragraph of every level tooltip): `Affects every rule that uses these tier styles, yours included.`

Two-line captions are allowed by the author; the stacked captions above satisfy that. Beyond that,
the row has no inline text, badge, or label ("Strictness" is not drawn as a separate caption; the
tooltip is the explanation). If the author wants a label, it can be a small-caps `group_heading`
"Strictness" above the row, but this contract does not include it (keeps the section short).

---

## Reset to defaults (RESET-01..02)

### Buttons

| Property | Styles heading (Config tab) | Tiers heading (Economy tab) |
|----------|-----------------------------|-----------------------------|
| Widget | `ff_button_ex(ui, skin, "Reset to defaults", min_width 150, enabled, danger=false)` | same |
| Position | Right end of the heading line, vertically centred on the `ff_section` rect (draw into a child ui over `heading.rect` with `Layout::right_to_left(Align::Center)`, 8pt from the right edge; heading text stays at left) | same, on the Tiers heading rect in `shell.rs` |
| Enabled | When the pending style set differs from the embedded defaults | When the pending tier set (cutoffs text) differs from the default cutoffs |
| Disabled look | Dimmed art and `DISABLED` caption, no hover (existing `ff_button_ex` behaviour); tooltip `Already at the defaults.` via `on_disabled_hover_text` | same |
| Enabled tooltip | `Restore the built-in styles. Your custom styles are kept.` | `Restore the default cutoffs.` |
| Click | Opens the popup (does not change anything yet) | same |

"Differs from defaults" for styles: any built-in (by id) not equal to its default value, name or
position, or the built-ins not sitting first in default order. Custom styles never count as a
difference. Probes (`probe("reset-styles", rect)`, `probe("reset-tiers", rect)`) follow the existing
test-probe convention.

### Popups (skinned `ff_modal`, one at a time, same stacking rules as the other eight)

| | Reset styles | Reset tiers |
|---|---|---|
| Title | `Reset styles to defaults?` | `Reset tiers to defaults?` |
| Body | `The built-in styles go back to their default look, names and order. Your custom styles are kept. Press Apply to save, or Revert to undo.` | `The tier cutoffs go back to their defaults. Press Apply to save, or Revert to undo.` |
| Button 1 | `Reset` (`danger: true`, red caption) | `Reset` (`danger: true`) |
| Button 2 (safe, last) | `Keep` | `Keep` |
| Escape / outside click | answers `Keep` (last button, `ff_modal` contract) | same |

Popup stays open if the window is hidden and summoned again (existing behaviour). The popup is
opened from the Config tab / Economy tab respectively but resolves in the shared popup layer like
`Delete style?`.

### Result

- Styles reset: one pending edit. Built-ins return to default values, name (matched by id) and
  order, placed first; all custom styles follow in their current relative order. Strictness then
  reads Flat Broke. Any expanded style row collapses and the rename/delete drafts clear via the
  existing `reset_for_new_styles`.
- Tiers reset: one pending edit; the cutoff boxes show the default values; inline validation
  errors from earlier edits clear.
- Config tab shows ` *` after a styles reset; Economy tab shows ` *` after a tiers reset (existing
  rules). Status line carries no special reset message beyond the standing `Unapplied changes`.
  Revert (per tab, with its existing "Discard ..." popup) undoes either reset. Apply writes both
  through the single existing Apply path.

---

## FIX-01: duplicate minimap icon tooltip

Contract: exactly one tooltip per minimap icon tile, text unchanged (the shape name, e.g.
`Kite`; for rule chips `Minimap icon: <Size Colour Shape>`). Known suspect:
`skin::editor::ff_icon_button` ends with `resp.on_hover_text(name)` while a caller may also
attach `on_hover_text` to the returned response, and the colour-card path at `editor.rs:241`
chains `on_hover_text(&text).on_disabled_hover_text(text)`. The executor finds which pair
doubles up, removes the redundant one, and verifies every place a tile is drawn: the style editor
Minimap/Beam pickers, the rule-row style chip, the Config table Icon column, and the rule editor
Custom lines. Ownership rule: the widget that paints the tile owns its tooltip; callers must not
add a second one. A headless test hovers each tile and asserts one tooltip.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Strictness captions | `Custom`, `Flat Broke`, `Scrounging`, `Getting By`, `Comfy`, `Loaded`, `Div Farmer` |
| Reset button (both tabs) | `Reset to defaults` |
| Reset styles popup | title `Reset styles to defaults?`; buttons `Reset` / `Keep` |
| Reset tiers popup | title `Reset tiers to defaults?`; buttons `Reset` / `Keep` |
| Disabled reset tooltip | `Already at the defaults.` |
| Empty state | Unchanged: `No styles loaded` (Styles). The strictness row is hidden there. |
| Error state | None new. A missing built-in tier style: row shows Custom; no new message (existing load errors cover a broken `styles.json`). |
| Primary CTA | `Reset to defaults` (secondary action; the phase's main interaction is the strictness row) |
| Destructive confirmation | `Reset styles to defaults?` / `Reset tiers to defaults?` as above: red `Reset`, `Keep` is the safe default |

Voice: short, no walls of text (author preference, memory `overlay-ui-not-texty`). Explanations
live in tooltips; the only standing text added to the window is the seven captions and the button.

---

## Interaction states summary

| Control | Idle | Hover | Active/selected | Disabled |
|---------|------|-------|-----------------|----------|
| Level segment | dim art, `DISABLED` caption | caption `LABEL`, tooltip | lit art, `HEADING` caption | n/a |
| Custom segment | dim art, `DISABLED` caption | tooltip only | lit art, `HEADING` caption when derived | always unclickable |
| Reset to defaults | `ff_button` | `ff_button` hover | opens popup | dim art, tooltip `Already at the defaults.` |
| Popup Reset | red caption | hover art | applies reset, closes | n/a |

Accessibility: segments report `WidgetType::RadioButton` with selected state and the caption as
name (as `ff_segmented`); Custom reports as selected/unselected but not enabled; buttons keep their
labels.

---

## Choices made in the author's place

1. Strictness row implemented as a levels variant of the skin segmented switch (same art,
   dividers, colours) with a fixed-width Custom segment (72pt) and six equal level segments, rather
   than seven equal segments; reasoning: Custom is display-only and short.
2. Row height 40pt (vs 30pt) and two-word captions stacked on two lines; single-word captions one
   line. The author allowed stacked captions; I chose stacking by default over shrinking text,
   with 14 -> 13 -> 12pt shrink only as an overflow fallback.
3. No "Strictness" label above the row; the explanation is in tooltips only (not texty).
4. Tooltip wording: per-level "Hides tier(s) X" lines in the author's example form, plus one shared
   sentence about rules (user rules included) being affected; Custom tooltip wording invented.
   "Hides tiers D to F" style used for ranges.
5. Level click on the already-highlighted level is a no-op; clicking Custom is a no-op.
6. Reset button label `Reset to defaults`, a plain `ff_button` (not red) placed right-aligned on
   the heading line, min width 150pt; disabled state shows tooltip `Already at the defaults.`
7. Popup bodies' wording invented; both mention Apply/Revert. Button labels `Reset` (danger) and
   `Keep` per CONTEXT.
8. Missing built-in tier style: row still drawn, Custom highlighted, levels act on what exists.
   Row hidden only when the whole Styles section shows "No styles loaded".
9. Reset leaves expanded rows collapsed (reuse `reset_for_new_styles`).
10. Styles-reset "differs from defaults" ignores custom styles entirely, and includes built-in
    order and renamed names.
11. FIX-01 ownership rule: the tile-painting widget owns the tooltip, callers add none.

---

## Checker Sign-Off

- [x] Dimension 1 Copywriting: PASS
- [x] Dimension 2 Visuals: PASS
- [x] Dimension 3 Color: FLAG (non-blocking: gold on the selected segment only, red only on the popup Reset)
- [x] Dimension 4 Typography: FLAG (non-blocking: 13/12pt fallbacks close to 14pt)
- [x] Dimension 5 Spacing: FLAG (non-blocking: 150pt min width and 6/7pt insets are existing skin values)
- [x] Dimension 6 Registry Safety: PASS (not applicable)

**Approval:** approved 2026-10-18 (gsd-ui-checker)
