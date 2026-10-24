# Phase 49 — UI Review

**Audited:** 2026-10-23
**Baseline:** 49-UI-SPEC.md (design contract) plus 49-CONTEXT.md decisions
**Screenshots:** captured (debug `--screenshot` renders under `%TEMP%\claude\ffshot49\`: rules-handles, rules-economy, rules-drag, config-styles, config-drag). Pointer behaviours (hover art, cursor, wheel, auto-scroll, Escape, focus loss) cannot be seen in a one-shot render and were not verified visually (code review 49-REVIEW.md covers them).

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 4/4 | Only new string is the `Drag to reorder` tooltip; no captions or on-screen hints. Matches the "not texty" rule. |
| 2. Visuals | 3/4 | Handles, greyed source, shadow and gold marker all present and aligned. The handle glyph reads as a menu icon, and the CONFIG drag shot shows the marker and shadow colliding with the table header. |
| 3. Color | 3/4 | Gold is used only for the drop marker, as specified. The source-row wash differs between the two lists and one uses a magic colour. |
| 4. Typography | 4/4 | No new text. The shadow reuses the row's fonts. |
| 5. Spacing | 3/4 | 28pt handle is centred in its row, right edge lines up with the section-header X and the style Order column. Built-in style rows keep their cell. The shadow overlaps rows with no gap. |
| 6. Experience Design | 3/4 | Lifecycle is complete (one pending edit, cancel paths, search disabled, no economy handle). The `moved across Continue` flag can be cleared by a later drop (WR-04, deferred). The injected-drag shot does not match a real drag. |

**Overall: 20/24**

---

## Top 3 Priority Fixes

1. **Handle glyph looks like a hamburger / menu button (WARNING)**. The three-line plate (rules-handles.png, config-styles.png) is the same shape as a "menu" or "list" button. In a text-light UI the tooltip is the only cue it drags, and the tooltip needs a hover to appear. Fix: if the author agrees, redraw `icon_reoder_*.png` with a grip (two columns of dots) or up/down chevrons. Otherwise keep it and rely on the Grab cursor.
2. **Drag shadow hides the drop marker's context (WARNING)**. In config-drag.png the shadow sits on the column header row and the marker coincides with its top edge. The shadow's opaque PANEL ground also covers the rows it passes over (rules-drag.png: the "Show Split" name on row 7 is hidden). Fix: clamp the shadow to the drop rect for CONFIG, and consider a shadow ground below ~0.8 alpha so the row under the marker stays readable.
3. **CONFIG drag screenshot is not a real drag state (WARNING, IN-03)**. `--drag-style` plus `--expand-style` renders an open editor and a greyed expanded row, which a real drag cannot show (it collapses on start). It misleads anyone judging the visuals from the shot, including this review. Fix: clear `expanded` when injecting, and re-render config-drag.png.

Further, lower priority:

4. Source-row wash differs: CONFIG uses `Color32::from_rgba_unmultiplied(0x0A, 0x0A, 0x09, 140)`, RULES uses `pal::GROUND.gamma_multiply(0.55)` (IN-04). Use the palette expression in both.
5. WR-04: a later non-crossing drop clears `moved across Continue`, so the warning can disappear while the rule is still crossed. Author decision pending (sticky flag vs. compare with applied document).
6. IN-05: the reset paths drop a drag without `ctx.stop_dragging()`, so handle hover art stays suppressed until the button is released.
7. The arrow code paths (`RowAction::Move`, `ConfigAction::MoveStyle`) are dead in production (IN-01). This is not visual but leaves untested-in-UI surface.

---

## Detailed Findings

### Pillar 1: Copywriting (4/4)
- Spec: one new string, `Drag to reorder`; `Clear the search to reorder` reused; accessible name `Reorder`; no captions during the drag. None of the screenshots show any new on-screen text. Section headers keep their existing `{n} rules` copy.
- Residual: with the glyph ambiguity (fix 1) the lone tooltip carries all the meaning. Not a copy defect.

### Pillar 2: Visuals (3/4)
- rules-handles.png: handles sit at the right edge of every user rule row, vertically centred in the 52pt row, and share an x with the section header X. Section headers keep up/down arrows. The `CONTINUE` flag sits left of the handle with a 4pt-looking gap. Clear focal hierarchy is unchanged.
- rules-economy.png: economy rows have no handle and no dead gap; the `FF Economy Section` header shows a padlock only.
- rules-drag.png: source row 3 is greyed in place (text and check dimmed, handle dimmed); shadow is a translucent copy of the row at the pointer; a 2pt gold line is drawn over the shadow (WR-01 fixed). Hierarchy is readable. Weakness: shadow is nearly the same tone as the rows under it, so the eye finds the gold line first and the shadow second, and the shadow hides the row names it covers.
- config-styles.png: built-in rows show blank Order cells with the Preview column unaffected; `mine` and `other` custom rows have handles aligned under the `Order` header.
- config-drag.png: marker lands on the `Visibility / Name / Icon / Preview / Order` header (the "above the first custom row" slot), and the shadow with its row text layers over the header text. Reads as clutter. The expanded editor under a dragged row is the injection artefact (IN-03).
- Handle art: hamburger glyph (see fix 1). Rendering is crisp, no gutter offset visible, so ART-04 compensation works.

### Pillar 3: Color (3/4)
- Gold accent: only the marker line (new use), as contracted. Handle art carries its own palette. No other new colours.
- Source-row greying: RULES follows the spec (`GROUND` at 0.55). CONFIG hardcodes `0x0A,0x0A,0x09,140` (config_view.rs:1053 vs rule_list.rs:1536). Two near-equal washes, one a magic value.
- 60/30/10 holds: ground/panel dominant, rows secondary, gold only on the transient marker.

### Pillar 4: Typography (4/4)
- No new sizes or weights. Shadow reuses Fontin at the row's sizes at 60% opacity.

### Pillar 5: Spacing (3/4)
- Handle is 28pt visible / 32pt hit, scaled from 84 opaque texels, square, snapped. Right edges align across the rules list (rows, section header X) and the Order column.
- Built-in style cell reserved, so the table lines up (config-styles.png).
- Flag budget recomputed from handle width + 4; the label keeps its minimum (no truncation seen in the shots, though only one flag, `CONTINUE`, was rendered).
- Handle (28pt) is smaller than the 34pt control height by design.
- Shadow has no inset from the list's left edge and covers full content width, so on the CONFIG table it extends over the header cells.

### Pillar 6: Experience Design (3/4)
- Covered by the contract and verified by code review: drag only from the handle, collapse of an expanded row on start, one `MoveTo` = one pending edit (` *` on the tab, Revert undoes), cancel on Escape, outside release, hide and focus loss; disabled handle while searching with the existing tooltip text; no handle on economy or built-in rows; invalid slots show no marker; CONFIG drops accepted only over the custom table (WR-03 fixed); marker above shadow (WR-01 fixed); reorder-off mid-drag cancels (WR-02 fixed).
- Gaps: WR-04 (flag clearing), IN-05 (hover state after reset), IN-07 (legal drop may be refused in a blank-less hand-written file, unverified). No keyboard path, by decision.
- Not verifiable from screenshots: hover/pressed art, Grab/Grabbing cursor, auto-scroll speed, wheel during drag. These need the author's manual checks (deferred list items) at 100% and 200% DPI.
- The marker has no sub-case rendering for a collapsed-header drop in the shots; not captured.

---

## Files Audited
- `.planning/phases/49-drag-and-drop-reorder/49-UI-SPEC.md`, `49-CONTEXT.md`, `49-01..06-SUMMARY.md`, `49-REVIEW.md`
- Screenshots: `rules-handles.png`, `rules-economy.png`, `rules-drag.png`, `config-styles.png`, `config-drag.png`
- Code references taken from 49-REVIEW.md (`src/overlay/rule_list.rs`, `config_view.rs`, `drag_reorder.rs`, `src/skin/widgets.rs`); not re-read in this audit.
