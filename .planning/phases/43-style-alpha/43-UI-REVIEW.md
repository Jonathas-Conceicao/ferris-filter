# Phase 43 - UI Review

**Audited:** 2026-10-23
**Baseline:** 43-UI-SPEC.md (approved)
**Screenshots:** captured by the debug `--screenshot` mode (`style-picker.png`, `style-table.png`). Two shots only: no RULES tab, no translucent-vs-opaque pair, no popup-clipping check. Code audit was light (picker and verification report), not exhaustive.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | `Alpha` label reused verbatim and nothing new added; the swatch tooltip and load-error rewording were not checked in the UI |
| 2. Visuals | 3/4 | Alpha row and checker strip match the RULES picker; the popup hides the editor and the Background card is the only cue |
| 3. Color | 2/4 | Background looks grey and dark in both shots, so the opaque case is unproven; the 255 no-regression claim is not visible |
| 4. Typography | 3/4 | Unchanged by design; no deviation seen |
| 5. Spacing | 3/4 | Alpha row follows the sibling rows; picker overlaps content, same as the RULES path |
| 6. Experience Design | 3/4 | Slider, Use colour, None tile and quick-pick row are present; the table Preview cannot be shown to follow edits |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **Prove the opaque case visually (WARNING)** - In both screenshots the S style's Background card is a muted grey, and the label preview and table Preview sit on a dark ground. `styles-default.json` S has a pale `248 248 242` background, which should read near-white at alpha 255. Either the shots were taken with a pending alpha edit (the picker shot shows Alpha 96 on the Background slot, so likely for `style-picker.png`) or the card and previews ignore the background. For `style-table.png` the cause is unclear. Fix: re-shoot with `--picker background` absent and a fresh home, and confirm the Background card and previews show the pale colour at 255. If they stay grey, the "Alpha 255 = identical pixels" contract is broken.
2. **Make a translucent edit visible in the previews (WARNING)** - Contract items 5-7 say the card swatch, label preview and table Preview must blend by alpha. The shots show no before/after pair, so this is unverified. Fix: capture S with Background alpha 255 and again at about 96 (`--tier-text` or a seeded `styles.json` with a 4-value colour), and compare the Background card, label preview and table Preview.
3. **Confirm the swatch tooltip (WARNING)** - The `R G B A` tooltip (shown only when alpha is not 255) cannot be seen in a static shot. VERIFICATION.md says the tooltip exists but names no test for it. Fix: add a unit test for the tooltip text at 255 (unchanged) and at 96 (`R G B A`).

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- The picker shows `Alpha`, as the contract requires. No new buttons, captions or messages were found, so the terse-UI rule holds.
- Not verified: the "3 or 4 values" load-error wording and the swatch tooltip.
- WARNING: no screenshot evidence for either string.

### Pillar 2: Visuals (3/4)
- `style-picker.png`: the Alpha row sits under the Hex/R/G/B row, with the checkerboard strip at left, the skinned slider and the exact number (96). This matches the RULES picker.
- The None tile (red slash) is kept at the end of the quick-pick row, as in the contract.
- The popup is anchored over the lower right and covers the Font size slider and the Minimap and Beam rows. The user cannot see the label preview change while editing the Background slot. It was acceptable on the RULES path, but the label preview (`ITEM NAME`) is visible above it, so live feedback is partly kept.
- The Background card is the only cue for which slot is open (a light outline). It is weak but pre-existing.
- The Text card is darker than the Border card, though both are pink 255. This may be the card shading or a composite artefact. Needs human review (`needs_human_review: true`).

### Pillar 3: Color (2/4)
- The Background card and the preview ground read grey and dark in both shots. In `style-table.png` there is no pending edit visible, yet the S style should have a pale background. This contradicts the contract unless the temp home's S was altered. Unresolved: hence the score.
- The picker strip uses the existing checkerboard, as allowed. The previews use no checkerboard, as decided by the author.
- A translucent-colour pair was not captured, so the unmultiplied blending claim has only a unit test (`label_look_paints_translucent_colours_unmultiplied`) behind it.

### Pillar 4: Typography (3/4)
- Fontin SmallCaps captions throughout. The Alpha slider uses the same number style as the siblings. No new sizes. No findings beyond the missing coverage of the tooltip.

### Pillar 5: Spacing (3/4)
- The Alpha row height and gaps look consistent with the R/G/B row. The popup grew by one row without clipping at 660pt width.
- Not checked: popup placement near the window's bottom edge when the Background slot of a lower style is open. The popup may be clipped or pushed up.

### Pillar 6: Experience Design (3/4)
- Present: live alpha slider, Use colour, None, quick picks, and (by VERIFICATION.md tests) idle and unchanged-open emit nothing.
- Equality, dirty state and Reset-at-defaults are covered by tests, not by the screenshots. The status bar reads `The filter does not exist yet. Apply...` with no ` *` or unapplied indicator in the shots, which is consistent with a clean state.
- WARNING: an alpha of 0 makes text or border vanish in the preview (by contract). There is no hint, which is acceptable for a terse UI but can look like a bug.
- WARNING: Escape restores RGBA, per contract. Not exercised in a screenshot.

---

## Files Audited
- `.planning/phases/43-style-alpha/43-UI-SPEC.md`, `43-VERIFICATION.md`
- `src/skin/colour_picker.rs` (alpha mode, `paint_checker_preview`, tests)
- `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png`, `style-table.png`
