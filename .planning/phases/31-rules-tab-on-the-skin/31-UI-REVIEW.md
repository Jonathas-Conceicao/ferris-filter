# Phase 31 — UI Review

**Audited:** 2026-10-16
**Baseline:** 31-UI-SPEC.md (approved). Accepted interim look: colour actions keep the legacy colour picker until Phase 33; dividers are Phase 32; CONFIG/ECONOMY/ABOUT are out of scope.
**Screenshots:** no dev server (Rust egui app). The 11 committed shots in `shots/` were viewed, together with the prototype `ff-proto-rules.png` and the Phase 30 baseline `rules.png`. Hover, keyboard and DPI behaviour are needs_human_review.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Spec strings match (`Search 102 rules…`, `ADD_RULE_TIP`, `RULE_ADDED_MESSAGE`, empty-state headings, Delete popup). The status line is still cut before the actionable part. |
| 2. Visuals | 3/4 | The skin carries through the list and editor. The expanded layout deviates from the spec in three places. |
| 3. Color | 3/4 | Gold is restrained, but the styled diamond still reads gold next to the gold `Continue`. |
| 4. Typography | 4/4 | Only Fontin 14/15/17/21. The row hierarchy is now clearly two levels: small-caps 17 name over regular 15 muted summary. |
| 5. Spacing | 3/4 | Search field and `+ Add rule` have a clear gap; rows are a fixed 52pt (`ROW_H`). Editor and card spacing deviate in places. |
| 6. Experience Design | 3/4 | Delete popup replaces the inline confirm, game scrollbar, arrows disabled while searching, add-rule re-keys state. Shots are viewport-clipped, so several states are unproven. |

**Overall: 19/24** (Phase 30: 17/24)

---

## Phase 30 RULES findings

- **Fixed:** gap between the search field and `+ Add rule` (`rules.png`, `rules-search.png`).
- **Fixed:** row hierarchy is two distinct levels.
- **Fixed:** grey egui widgets replaced by skin dropdowns, text fields, buttons and the segmented Yes/No and Show/Hide.
- **Partly fixed:** the styled chip sits left of `Continue` (`◆ A   Continue`), but the diamond still looks gold-tinted, so row 1 shows two gold cues.
- **Partly fixed:** the broken-link icon still reads as two joined rings at screenshot size; the padlock is still small.
- **Not fixed (shell, Phase 30 code):** the status line still reads `Apply refused: 2 enabled rules hav…` in every shot.

## Top Priority Fixes

1. **WARNING — numeric condition layout.** In `expanded-numeric.png` the slider and its value drop to a second line under the operator dropdown, instead of the prototype's one line (`Item Level  >=  slider  82`). Each numeric condition costs two lines and pushes Style and Continue below the fold. Fix: dropdown, slider and 44pt value field in one row, slider sized to the remainder. `Set Font Size` in `expanded-custom-actions.png` shows the same pattern.
2. **WARNING — Defences and Class cards do not follow the two-column grid.** In `expanded-defences.png` the Class group headers (`WEAPONS (0)`, `ARMOUR (1)`…) are not in a framed card and have no check lines; the Defences card is a large bordered box with the Armour toggle and operator stacked and its number field cut at the right edge. Fix: three compact sub-lines inside a card that fills the control column; frame the Class picker the same way.
3. **WARNING — gold on non-actions and small flag glyphs.** Tint the styled diamond `pal::LABEL`; enlarge or add a slash to the broken-link glyph; enlarge the padlock.
4. **WARNING — expanded row header.** The 1pt `INPUT_BORDER_HOT` outline is faint (weak focal point), and the header drops the summary, leaving a blank second line before NAME. Show the summary or compact the header to one line.
5. **WARNING — status line cut at `…hav…`.** The first bad rule and the click-to-jump are not discoverable. Shorten the standing string (e.g. `Fix rule 1: grammar error`) or let the status prefer its tail. (Shell code; may belong to a later phase.)
6. **WARNING — evidence gaps.** No shot shows colour actions in the legacy picker, the `+ condition` / `+ style` popup, the quoted-list `+ Add` field, Simple Defences chips wrapping, or the add-rule result. Expanded shots are viewport-clipped.

Minor: `Matches exactly` is tight in its dropdown in `expanded-quoted-list.png`; header dead space; Between and numeric lines use different vertical rhythms; `widgets.rs:491-500` hardcoded greys for the legacy colour swatch (accepted interim).

---

## Detailed Findings

### Copywriting (3/4)
All specified strings are present. `Matches exactly` is tight at the dropdown edge. Status line truncation as above.

### Visuals (3/4)
Positive: Simple Defences chips (gold on-state) match the contract; `Yes | No` and `Show | Hide` segmented controls are consistent; the collapsed row matches the prototype closely, and the plain `Show` word follows the contract. Deviations: numeric layout, Defences/Class cards, header dead space.

### Color (3/4)
Gold in rows: the ring tick, `Continue` and the diamond; the spec allows only the first two. Hardcoded colours: `rule_list.rs:493` (`Color32::WHITE` for text measurement, harmless) and the legacy swatch greys (accepted interim). Error/warn colours appear only on flags and the status.

### Typography (4/4)
Two faces, sizes from the allowed set. Index, summary and values regular 15; row header small-caps 17; monospace raw row as specified.

### Spacing (3/4)
Fixed 52pt rows, correct toolbar gap, label column consistent row to row (the spec's 128pt + 16 gap is not literally reproduced). Chip gaps about 8pt. Between `Min`/`Max` lines and numeric lines use different rhythms. Class card headers have no left padding relative to the labels.

### Experience Design (3/4)
In code: `RULE_ADDED_MESSAGE`, `ADD_RULE_TIP`, NAME focus after add, scroll-to-card focus state. In shots: arrows dim while searching; skinned Delete popup with red `Delete`. Not provable from stills: keyboard on rows, post-add scroll, scrollbar drag, 500-block smoothness, grouped-rule flows.

## needs_human_review
- Enter/Space row toggle and the focus ring.
- Add rule: scroll to top, new rule expanded with NAME focused, query cleared.
- Class picker and quoted-list scrolling, including paste-split.
- `+ condition` / `+ style` popups: collapsed sections, search, greyed items focusing existing cards.
- Colour action cards in the legacy picker.
- DPI 100-200%.
- 500-block performance.

## Registry Safety
Skipped: no shadcn, no third-party registries.

## Files Audited
- `shots/*.png` (11), `ff-proto-rules.png`, Phase 30 `rules.png` and `30-UI-REVIEW.md`
- `31-UI-SPEC.md`, `31-CONTEXT.md`
- Targeted greps: `src/overlay/rule_list.rs`, `body.rs`, `widgets.rs`, `apply.rs`
