# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png` (debug screenshot, S with text alpha 160 and background alpha 96). Expect the CONFIG style colour picker to show an Alpha slider (reading 96) and the None tile, as on the RULES picker; check the same on Text and Border in the app.
2. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-table.png`. Expect the S label preview and table preview to look translucent (darker pink background and text over the dark ground), and hovering a card with alpha below 255 to show `R G B A`.
3. Phase 43: in game, give a style alpha below 255, press Apply and drop an item using it. Expect a translucent label as in the preview.
4. Phase 43: with default styles (no alpha edits), open CONFIG and look at S and A. Expect their pale background, label preview and table preview to look exactly as in v0.7.1 (alpha 255 paints opaque), and the Text card colour to match the Border card for an opaque pink.
5. Phase 44: in the app, open CONFIG and scroll to the Custom heading (the debug screenshot `C:UsersjonatAppDataLocalTempf44nfig-custom.png` cuts it off below the fold, only the buttons' top edge shows). Expect `Auto Detect` left of `+ Add style` on one line, not clipped, greyed with tooltip `No filter loaded.` when no filter is loaded.
6. Phase 44: on a real filter, click Auto Detect. Expect new `Custom Style N` rows, tagged rules with style chips, Config * and Rules * and Unapplied changes, status `Auto Detect: N styles, M rules tagged`; a second click says `Auto Detect: nothing new`. Revert on Rules drops the tags, Revert on Config drops the styles and leaves the tagged rules flagged `unknown style`.
7. Phase 44: press Apply after Auto Detect and reload in game. Expect the filter to look the same as before (rules now rewritten from their styles); `styles.json` gains the `Custom Style N` entries with ids.

## Art substitutes for review (Phase 48 / 47)

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
- Phase 44: the Auto Detect button is 139.5pt wide, not 112pt: 112 is only the minimum of `ff_button_ex`, the small-caps label plus 44pt padding is wider; the gap to `+ Add style` is 8pt (the Ui's item spacing, no extra space added).
- Phase 44: rules with grammar errors, `can't enable` rules and compact headers (`Hide Continue`, which `rewrite_block` refuses at Apply and reports without blocking) are skipped or handled as researched; the status uses the `apply_message` slot.
- Phase 44: the bottom-bar Reload after Auto Detect (nothing changed on disk) re-reads the filter, so the tags are dropped with the other rule edits, while the new `Custom Style N` styles stay pending as CONFIG edits (usual per-tab behaviour; asserted by `auto_detect_bar_reload_drops_tags_keeps_new_styles`). The next Apply writes those styles to `styles.json` with no rule using them unless Auto Detect is clicked again or Config is reverted.
