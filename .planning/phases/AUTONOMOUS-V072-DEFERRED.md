# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png` (debug screenshot, S with text alpha 160 and background alpha 96). Expect the CONFIG style colour picker to show an Alpha slider (reading 96) and the None tile, as on the RULES picker; check the same on Text and Border in the app.
2. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-table.png`. Expect the S label preview and table preview to look translucent (darker pink background and text over the dark ground), and hovering a card with alpha below 255 to show `R G B A`.
3. Phase 43: in game, give a style alpha below 255, press Apply and drop an item using it. Expect a translucent label as in the preview.
4. Phase 43: with default styles (no alpha edits), open CONFIG and look at S and A. Expect their pale background, label preview and table preview to look exactly as in v0.7.1 (alpha 255 paints opaque), and the Text card colour to match the Border card for an opaque pink.

5. Phase 48: open `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\rules-headers.png`. Expect plates at the left (chevron up over a dot on the expanded sections, dot over chevron down on the collapsed economy header, which is below the visible part in this shot, so scroll to it in the app), the stone bar with its bronze frame, titles at the left, pencil/arrows/x (padlock on economy) at the right.
6. Phase 48: open `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\rules-search.png`. Expect the magnifier at the left and the round clear X at the right of the search field.
7. Phase 48: in the app, type in the RULES search and click the X. Expect the field empty and the caret still in it.
8. Phase 48: in the app, open the `+ condition` picker on an expanded rule, type, click the X (no screenshot flag opens this popup). Expect the text cleared, the popup still open and the caret in the field.
9. Phase 48: open `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\config-refresh.png` and `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\economy-refresh.png`. Expect each refresh button centred on its dropdown (it is a little taller than the dropdown, about 3 px each side at 1x; judge whether that is acceptable).
10. Phase 48: in the app, hover and press a section header plate. Expect brighter art on hover, dimmer while pressed.
11. Phase 48: at 150% and 200% Windows scaling, look at the section header bars. Expect no seams.

## Art substitutes for review (Phase 48 / 47)

Phase 48: none — all four pieces (toggle plates with hover/pressed, stone bar, clear X, magnifier) were found in the game files and extracted with ff-art; screenshots for a look: `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\rules-headers.png`, `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\rules-search.png`, `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\config-refresh.png`, `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\economy-refresh.png`.

## Open gaps and judgement calls

- Phase 43: the style colour type is `StyleColour([u8; 4])` (no Default or From<[u8;3]>; Eq/Hash derived for Phase 44). Phase 44 must treat a bare 3-token `SetBackgroundColor` as alpha 240, which is not equal to an alpha-255 style.
- Phase 43: the `R G B A` hover tooltip lives in the shared colour slot, so RULES colour cards show it too (only when alpha is below 255).
- Phase 43: quick picks in the colour picker keep the current alpha; the Hex field stays 6-digit RGB only.
- Phase 43: the opaque colour helpers (`colour_slot`, `colour_swatch_rgb`, `StyleView.alpha`) were removed and their tests ported to the alpha path.
- Phase 43: `styles.json` files with 3-value colours stay 3-value on disk until a CONFIG edit is applied (locked decision, no migration flag); `styles-default.json` is 4-value, `tests/fixtures/seed-styles.json` stays 3-value.
- Phase 43: cards and previews paint translucent colours over their dark ground; no checkerboard was added.
- Phase 43: `cargo clippy -D warnings` (134 diagnostics, same count before and after the phase) and `cargo fmt --check` (a few diffs) fail on the floating nightly in code Phase 43 did not introduce; not fixed.
- Phase 43: the Release Review changelog should note that a `styles.json` written by v0.7.2 (4-value colours, saved by any Apply that writes it, including the one that persists ids for an id-less file) cannot be read by older releases; the way back is to delete `styles.json` or use Reset to defaults (WR-01, no code fix by design).
- Phase 43: the UI review (43-UI-REVIEW.md, advisory, 17/24) also suggests a unit test that the `R G B A` swatch tooltip shows only below alpha 255, and the review info items IN-01..IN-05 (dead `ff_colour_card`, always-true `PickerMode::alpha`, "length 5" error text) are left for a later cleanup.
- Phase 48: the section plate is 27x32pt at the art's aspect, with a 4pt gap before the bar.
- Phase 48: the bar caps are painted at natural scale, the title sits at +60pt and the right-hand controls sit over the right cap's swirls; there is no 3pt gap between bars.
- Phase 48: the reload button grows to fill the row (about 9% larger than before).
- Phase 48: no italic placeholder in the search field; the clear X is not a tab stop and shows for whitespace-only text; its space is always reserved, so a long placeholder truncates about 22pt earlier.
- Phase 48: the magnifier pressed art was not extracted; only two search fields exist (RULES toolbar and the condition picker).
- Phase 48: legacy PNGs were not re-extracted (a UV-crop helper, `paint_visible` / `box_uv`, is used instead); the round check is now painted by its visible plate (0.8pt less art height); the clear X is cropped 36x36, not 37x37.
- Phase 48: the decorative rule line between the section title and the count was removed.
- Phase 48: plan 02 and 03 deviations: tests and code were committed together (no separate RED commit); `ff_search_field` takes `&Skin`; no harness test for the picker staying open on an X click (manual check 8).
- Phase 48: screenshot observation: the refresh plates are centred on their dropdowns but stand about 3px taller at 1x. In the screenshots, the section header arrows and x overlap the right cap swirls (as planned).
- Phase 48: UI audit (48-UI-REVIEW.md, 18/24, no blockers) suggests three follow-ups for the author to decide: (a) the dimmed section up/down arrows are hard to see over the bar's ornate right cap (move the cluster onto the flat middle or brighten them); (b) the RULES search placeholder now truncates (`Search 20 rules, classes, ...`) because the clear X space is always reserved (reserve it only with text, or shorten the hint); (c) the refresh plate is ~3px taller than its dropdown on each side (match heights either way). Code review (48-REVIEW.md) is clean after fixes; four Info items remain (dead `FlagIcon::ChevronRight/Down`, `ff_text_field` with no production caller, geometry magic numbers, title under controls only if the width ever varies).
