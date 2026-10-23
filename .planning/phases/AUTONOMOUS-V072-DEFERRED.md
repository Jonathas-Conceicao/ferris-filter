# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png` (debug screenshot, S with text alpha 160 and background alpha 96). Expect the CONFIG style colour picker to show an Alpha slider (reading 96) and the None tile, as on the RULES picker; check the same on Text and Border in the app.
2. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-table.png`. Expect the S label preview and table preview to look translucent (darker pink background and text over the dark ground), and hovering a card with alpha below 255 to show `R G B A`.
3. Phase 43: in game, give a style alpha below 255, press Apply and drop an item using it. Expect a translucent label as in the preview.

## Art substitutes for review (Phase 48 / 47)

## Open gaps and judgement calls

- Phase 43: the style colour type is `StyleColour([u8; 4])` (no Default or From<[u8;3]>; Eq/Hash derived for Phase 44). Phase 44 must treat a bare 3-token `SetBackgroundColor` as alpha 240, which is not equal to an alpha-255 style.
- Phase 43: the `R G B A` hover tooltip lives in the shared colour slot, so RULES colour cards show it too (only when alpha is below 255).
- Phase 43: quick picks in the colour picker keep the current alpha; the Hex field stays 6-digit RGB only.
- Phase 43: the opaque colour helpers (`colour_slot`, `colour_swatch_rgb`, `StyleView.alpha`) were removed and their tests ported to the alpha path.
- Phase 43: `styles.json` files with 3-value colours stay 3-value on disk until a CONFIG edit is applied (locked decision, no migration flag); `styles-default.json` is 4-value, `tests/fixtures/seed-styles.json` stays 3-value.
- Phase 43: cards and previews paint translucent colours over their dark ground; no checkerboard was added.
- Phase 43: `cargo clippy -D warnings` (127 errors) and `cargo fmt --check` (a few diffs) fail on the floating nightly in code Phase 43 did not introduce; not fixed.
- Phase 43: the Release Review changelog should note that a `styles.json` written by v0.7.2 (4-value colours, saved by any Apply that writes it, including the one that persists ids for an id-less file) cannot be read by older releases; the way back is to delete `styles.json` or use Reset to defaults (WR-01, no code fix by design).
