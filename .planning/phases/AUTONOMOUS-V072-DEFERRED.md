# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-picker.png` (debug screenshot, S with text alpha 160 and background alpha 96). Expect the CONFIG style colour picker to show an Alpha slider (reading 96) and the None tile, as on the RULES picker; check the same on Text and Border in the app.
2. Phase 43: open `C:\Users\jonat\AppData\Local\Temp\ff43\style-table.png`. Expect the S label preview and table preview to look translucent (darker pink background and text over the dark ground), and hovering a card with alpha below 255 to show `R G B A`.
3. Phase 43: in game, give a style alpha below 255, press Apply and drop an item using it. Expect a translucent label as in the preview.
4. Phase 43: with default styles (no alpha edits), open CONFIG and look at S and A. Expect their pale background, label preview and table preview to look exactly as in v0.7.1 (alpha 255 paints opaque), and the Text card colour to match the Border card for an opaque pink.
5. Phase 44: in the app, open CONFIG and scroll to the Custom heading (the debug screenshot `C:/Users/jonat/AppData/Local/Temp/ff44/config-custom.png` cuts it off below the fold, only the buttons' top edge shows). Expect `Auto Detect` left of `+ Add style` on one line, not clipped, greyed with tooltip `No filter loaded.` when no filter is loaded.
6. Phase 44: on a real filter, click Auto Detect. Expect new `Custom Style N` rows, tagged rules with style chips, Config * and Rules * and Unapplied changes, status `Auto Detect: N styles, M rules tagged`; a second click says `Auto Detect: nothing new`. Revert on Rules drops the tags, Revert on Config drops the styles and leaves the tagged rules flagged `unknown style`.
7. Phase 44: press Apply after Auto Detect and reload in game. Expect the filter to look the same as before (rules now rewritten from their styles); `styles.json` gains the `Custom Style N` entries with ids.
### Phase 45: Economy tier-range names

1. Open the RULES tab on a filter with an economy section, press Apply once: every economy row name now ends with its range (`S tier Currency >= 30 chaos` live, `... exalt` under Fallback); the first Apply after upgrading rewrites all old names once (expected per CONTEXT).
2. Longer labels such as `D tier Map Fragments & Pinnacle Keys < 8 chaos` are cut cleanly with an ellipsis on collapsed rows and the style chip / flags still fit.
3. On the Economy tab change a cutoff (e.g. E to 1): the staged / previewed economy rows on RULES rename (`D tier ... < 1 chaos`) before Apply.
4. Search on RULES finds economy rows by the new text (e.g. typing `< 2 chaos`).

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
- Phase 44: a 3-value `SetBackgroundColor` is alpha 240 in game and Auto Detect reads it so, but the RULES Custom style editor still shows such a line as alpha 255 (Phase 43 decoder); the two views disagree until the line is rewritten. Review info items IN-01 (grammar-error rules skipped silently), IN-03 (styles left unused after a partial tag or a Reload) and IN-04 (a rule tagged S..F now follows Economy Visibility) are left as is (44-REVIEW.md).
- Phase 44: UI review (44-UI-REVIEW.md, advisory, 20/24): the author-set status `Auto Detect: 3 styles, 41 rules tagged` is cut with … in the bottom bar beside Revert / Reload / Apply (full text on hover; see `C:/Users/jonat/AppData/Local/Temp/ff44/config-status.png`); after a click the new collapsed `Custom Style N` rows sit below the fold with no scroll-to. Decide whether to shorten the copy or scroll to the Custom heading.
- Phase 45: a non-bottom cutoff below 0.005 renders as `0` under the locked 2-decimal rule (no shipped file hits it); mixed-unit tier files label each tier with its upper neighbour's own unit (and `div` if a user uses it); two tiers with the same chaos-equivalent floor (only possible across units) make the lower one read `< X` with X equal to its own floor.
- Phase 45 (code review WR-01, author call): the locked "at most 2 decimals" rule can misstate a range when a cutoff has more decimals: 0.025 exalt prints `< 0.03 exalt` (an item at 0.027 is really in the tier above), a top floor of 1.999 prints `>= 2`, and a cutoff below 0.005 prints `< 0`. Options: print more digits only when a cutoff is not exact at 2 decimals, or have the Tiers editor reject cutoffs with more than 2 decimals. Formatter: `fmt_amount` in `src/pricer.rs`.
- Phase 45 (code review IN-01/IN-03): a `-0` cutoff is accepted and prints `>= -0` / `< -0`; a unique name such as `A tier Uniques < 30 chaos` describes the most valuable unique on each base, while the block highlights every unique on those bases.
