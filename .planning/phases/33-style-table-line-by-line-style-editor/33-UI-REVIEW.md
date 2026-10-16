# Phase 33 — UI Review

**Audited:** 2026-10-16
**Baseline:** 33-UI-SPEC.md (extends 32/31/30-UI-SPEC)
**Screenshots:** no dev server (Rust egui app). Committed shots viewed: `styles-table.png`, `editor-builtin.png`, `editor-custom.png`, `editor-minimap-none.png`, `picker-style-text.png`, `picker-style-background.png`, `picker-rule.png`, `delete-style.png`. No extra shots rendered. Code checked in `config_view.rs`, `skin/colour_picker.rs`, `overlay/widgets.rs`. Hover, drag, keyboard, Escape-revert, DPI and the custom editor's Delete line are needs_human_review (33-HUMAN-UAT.md is all `[pending]`).

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Spec labels, options and tooltips are present and icon-first. The status slot is cut in every shot, and the RULES line label reads `SET BACKGROUND C` (truncated, no ellipsis). |
| 2. Visuals | 3/4 | Aligned header and rows, frameless line editor, 12 shapes on one row and a skinned picker. Font-size off leaves an orphan circle, the preview does not visibly grow, and Delete is not evidenced. |
| 3. Color | 3/4 | No new UI palette. Gold is limited to the open-row outline, selected rings and the checked states. Unchanged/native styles preview in an invented yellow. One hardcoded `INVALID_RED` (spec-sanctioned). |
| 4. Typography | 3/4 | Only 14/15/17/21 and two faces. Built-in names render as small caps but custom ones as plain Regular text in fields, so the Name column mixes faces and case. |
| 5. Spacing | 3/4 | Columns line up in both tables at the real window. Hex/R/G/B field row is cramped (the `R` label touches the Hex box) and the preview well is wider than its label. |
| 6. Experience Design | 3/4 | Live edits with a no-op guard, Escape revert, None-tile, remembered values, popup keeps the window bounds. All of it is code/test evidence only; UAT is 0 of 9. |

**Overall: 18/24** (Phase 32: 19/24)

---

## Top Priority Fixes

1. **WARNING — Custom-style Delete line has no visual evidence.** `editor-custom.png` ends at `VOLUME`; the `Drop sound` line and the 128pt danger `Delete style` button sit below the window. This is the only destructive entry point in the editor, and the spec's 12pt gap plus red caption are unchecked. Fix: shoot the `Loot` editor scrolled to the bottom (or taller viewport) and add it to the shots folder; confirm in UAT item 2 and 7.
2. **WARNING — the RULES colour-action label is truncated.** `picker-rule.png` shows `SET BACKGROUND C` cut at the 128pt label column with no ellipsis, and the chip drops to a second line beneath it, leaving a tall, mostly empty row. Phase 33 reworked exactly this line (chip replaces the `R G B A` caption). Fix: truncate with `…` plus a tooltip, or put the chip on the label's line (the chip is 34x22 and the control column has 353pt free).
3. **WARNING — Font size "off" leaves an unlabeled lone circle.** `editor-custom.png` (`Loot`): the line shows only a dark round check, no slider, no value, so the line reads as empty. Spec says check then slider, and a dimmed slider with `—` or the game default would keep the line's shape. Fix: render the slider disabled (dimmed) when the override is off instead of removing it.
4. **WARNING — Name column mixes faces.** `styles-table.png`: built-ins are small caps (`UNCHANGED`, `HIDDEN`), customs are Regular in a boxed field (`kite`, `exceptional`). Per spec this is by design (field vs text), but the two tables sit one under the other and read as inconsistent. Fix: use the small caps face inside the custom `ff_text_field`, or accept and note.
5. **WARNING — status slot cut to uselessness.** Every shot shows `Filter out of date – press Apply, th…`. It is Phase 30 shell code, but the CONFIG tab now generates many more unapplied states, so the cut hides the most important part (what to do). Fix: shorten to `Unapplied changes` (the same remark as Phase 32 fix 2).
6. **WARNING — preview does not demonstrate the font-size line.** In `editor-builtin.png` the `S` label preview (font size 37) renders the same size as the table's preview (spec: "grows with the slider, clamped by the preview scale"). The slider and the preview feel unconnected. Fix: raise the clamp or show a small size scale so 21 vs 37 is visible; verify in UAT.
7. **WARNING — `unchanged` previews in yellow text on black.** `styles-table.png` `UNCHANGED` row shows `ITEM NAME` in a saturated yellow on pure black, which is not any palette entry and looks like a defect next to the Dracula rows. Fix: use the native Rare text colour on `ROW` ground with a tooltip, or state it as intentional in UAT.

Minor: picker `Hex` field and `R` label touch (`picker-style-text.png`); `HIDDEN` preview is dark-on-dark but intended; picker covers the controls it edits (by design, no backdrop).

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Present and matching: headers `Visibility / Name / Icon / Preview / Order` (both tables), `+ Add style`, editor labels `Visibility, Label, Font size, Minimap, Beam, Sound, Volume, Drop sound`, options `Show|Hide`, `None|Large|Medium|Small`, `Temp`, `Custom file`, `Use colour`, card captions `Text / Border / Background`, `No colour (game default)` (`colour_picker.rs:521`). `delete-style.png` shows the `Delete style?` popup with the config.json warning and Delete (red) / Keep.
- Not texty is honoured: no `Show/Hide` caption on rows, no `NAME` caption, no `R G B A` caption (tooltip only), no number fields. The remaining words are labels, option names and button text.
- Gaps: status string cut (fix 5). `SET BACKGROUND C` in the rule line (fix 2, `picker-rule.png`). In `picker-rule.png` the search hint is `Search 1 rules, ...` (grammar, Phase 31 code, noted only).
- `Delete style?` copy with `0 rules` case is evidenced; the count-prefixed variant is test-only (VERIFICATION notes this).

### Pillar 2: Visuals (3/4)
- Positive: `styles-table.png` header labels sit over their columns in Built-in and Custom, the icon dots (`config.json`-style colours) read quickly, built-in arrows are visibly greyed, the switch stands alone. `editor-builtin.png` shows no frames: one line per control, the label preview and the Text/Border/Background cards as the focal block. `editor-minimap-none.png` hides the colour and shape rows and dims Temp, as specified. 12 shape tiles on one row (332pt) in `editor-builtin.png`. The picker is a game-box frame with square, hue, fields, quick picks and a single `Use colour` button; the None tile fits on the quick-pick row in `picker-style-*`, better than the spec's fallback.
- Deficits: fixes 1, 3, 6, 7. In the picker the open swatch gold ring is visible on `TEXT` in `picker-style-text.png`; the card text on a bright fill (`TEXT` dark on pink) is readable.
- Icon-only controls: every tile has a tooltip (`on_hover_text(name)`, `colour_picker.rs:532`) and an accessible name (`widget_info`, `:300`, `:380`, `:434`). The Visibility `slide_switch` stays the legacy 32x16 (spec exception).
- The Preview well is wider than the label it holds (dark box with centred label), so the column looks half empty; acceptable but loose.

### Pillar 3: Color (3/4)
- Hardcoded colours in the phase files: `config_view.rs:33` `INVALID_RED` (spec says keep), `:988` converts style data to `Color32` (content, allowed), `colour_picker.rs:202,228` gradient/hue (content), `:250` checkerboard `from_gray(200/120)` (alpha strip, content). Everything else uses `pal::`.
- Gold is reserved for the open-row outline (`editor-*.png`), selected tile rings and checks, as specified; no gold fills on rows. `Delete` is the only red caption in the editor (popup shows red `DELETE STYLE`).
- Legacy theme remains outside the phase's scope: `theme::ROW_BASE`, `theme::WARM_ACCENT_TEXT`, `egui::ComboBox` for the filter picker, `zebra_row` for TIERS (`config_view.rs:319,335,469`). The CONFIG tab thus has two looks in one scroll area: Filter/Tiers legacy, Styles skinned. In `styles-table.png` the Filter dropdown (flat grey, upper-case) differs from the Sound dropdown (`editor-builtin.png`, dark with arrow). Out of scope per spec, still visible.
- Fix 7 (`unchanged` yellow preview).

### Pillar 4: Typography (3/4)
- Sizes: 14 (header, picker labels), 15 (headings), 17 (names, line labels), 21 (popup). No new sizes. No bold. Two faces only.
- Mix: small-caps built-in names vs Regular custom fields (fix 4). The tiny `R/G/B` labels (14 MUTED) in the picker are legible but crowded against the Hex field.
- The label preview sample text `Item Name` uses small caps at the table's 26pt row size; it does not scale in the editor (fix 6).

### Pillar 5: Spacing (3/4)
- Columns: Visibility, Name, Icon, Preview, Order all line up in both tables (`styles-table.png`). Row height 34, editor lines 34 with taller wrap on Beam/Temp (not wrapped in the shots: Temp fits at 353pt, tighter than the spec's expectation, good).
- Picker: 300pt wide, content 268, anchored below the card, flipping or clamping to the window for the Background card (`picker-style-background.png` is right-clamped and stays inside). The rule picker (`picker-rule.png`) also stays inside.
- Deficits: the Hex field leaves no gap before the `R` label; the chip row in `picker-rule.png` leaves a tall empty band under `SET TEXT COLOR`; empty band at the bottom of the RULES shot above `Add economy section` (Phase 32 carry-over).

### Pillar 6: Experience Design (3/4)
- Covered: live edits with a no-op guard (open/close leaves Apply unchanged), Escape restores, None tile only for style slots (absent in `picker-rule.png`), alpha only for rules, remembered values across a fold (WR-02 fix `297cbf9`), unset-slot seed (CR-01/WR-01), Minimap None hides tiles, built-in editable without rename/move/delete, rename keeps links by id, Delete opens the popup with the rule count and `config.json` warning, a styles load error still shows one warn line.
- Gaps: no evidence of the hover/keyboard states (arrows nudging the square, Tab order); `33-HUMAN-UAT.md` has every item `[pending]`; the in-editor Delete (fix 1) is unseen; 3-token rule colours and Escape byte-exact revert are test-only. A picker that overlays the very controls it edits (`picker-style-text.png` covers Font size and Minimap) is a conscious trade-off but hides the live preview line when the Text card is clicked? In the shot the preview stays visible above, so this is fine.
- Not-texty acceptance (6 items in 33-VERIFICATION) is a human decision and still open.
- No registry audit (no shadcn).

---

## Registry Safety
Not applicable (not a web project; `components.json` absent).

## Files Audited
- `src/overlay/config_view.rs`, `src/skin/colour_picker.rs`, `src/overlay/widgets.rs` (greps and targeted reads)
- `.planning/phases/33-style-table-line-by-line-style-editor/33-UI-SPEC.md`, `33-VERIFICATION.md`, `33-HUMAN-UAT.md`, `32-UI-REVIEW.md` (format)
- Shots: `styles-table.png`, `editor-builtin.png`, `editor-custom.png`, `editor-minimap-none.png`, `picker-style-text.png`, `picker-style-background.png`, `picker-rule.png`, `delete-style.png`
