# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png` (debug screenshot, S with text alpha 160 and background alpha 96). Expect the CONFIG style colour picker to show an Alpha slider (reading 96) and the None tile, as on the RULES picker; check the same on Text and Border in the app.
2. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-table.png`. Expect the S label preview and table preview to look translucent (darker pink background and text over the dark ground), and hovering a card with alpha below 255 to show `R G B A`.
3. Phase 43: in game, give a style alpha below 255, press Apply and drop an item using it. Expect a translucent label as in the preview.
4. Phase 43: with default styles (no alpha edits), open CONFIG and look at S and A. Expect their pale background, label preview and table preview to look exactly as in v0.7.1 (alpha 255 paints opaque), and the Text card colour to match the Border card for an opaque pink.

## Art substitutes for review (Phase 48 / 47)

- Phase 47: the keybind button uses the game's own Options art (`options menu/optionsbuttonborder.dds` + `optionsbutton.dds` -> `assets/poe2/options/keybind_border.png` / `keybind_plate.png`, trimmed 60x60); the generic-button fallback was not needed. The border's white mask interior is keyed out by a new ff-art manifest option `key_white = [30, 30, 32]` (flood fill + colour-to-alpha against white), so the grey fringe becomes a soft dark vignette just inside the gold line instead of a pale halo. Check the Overlay row against `.planning/research/ui-refs/options-keybind-rows.png`: plate tint `GOLD` at rest, `#E6C882` lit, 20-texel nine-slice corners.

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
- Phase 47: a saved chord that fails to register for a reason other than "in use" falls back to Ctrl+Shift+Z with the warning `` `hotkey` <chord> could not be registered (<reason>), so Ctrl+Shift+Z is used `` (UI-SPEC only words the in-use case). The tray failure reason for an in-use chord reads `in use by another app`.
- Phase 47: an invalid saved `hotkey` alone gives the status warning `ff-settings.json could not be fully used; `hotkey` is not a usable key combination, so Ctrl+Shift+Z is used.` (the UI-SPEC sentence wrapped in the same form as the other key warnings).
- Phase 47: the keybind border's interior is cleared at extract time (new ff-art `key_white` option) rather than at draw time: egui only alpha-blends, and the irregular white interior reaches into the nine-slice edge pieces, so no draw-time trick removes it without covering the gold corners. Threshold 32 chosen over 96: it also keys the 32-96 grey fringe, which would otherwise stay opaque grey.
