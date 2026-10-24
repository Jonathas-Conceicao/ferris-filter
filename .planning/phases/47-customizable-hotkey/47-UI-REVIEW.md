# Phase 47 — UI Review

**Audited:** 2026-10-23
**Baseline:** 47-UI-SPEC.md (approved 2026-10-23), reference `.planning/research/ui-refs/options-keybind-rows.png` / `options-input-tab.png`
**Screenshots:** used the debug screenshot mode captures already in `C:\Users\jonat\AppData\Local\Temp\claude\ff47-shots\` (`rest.png`, `custom.png`, `capture.png`, `invalid.png`, 660x1032 at 1pt = 1px). No dev server (egui desktop app). The screenshot mode has no flag for the rejected-caption state, so that state was checked in code only. Pixel values were measured with a small Node PNG decoder kept in the scratchpad.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | The contract copy matches word for word, but the extra rejected caption added by the review, `Used by Windows or FerrisFilter`, is wider than the button's 184pt caption box, so it shows cut off with `…` |
| 2. Visuals | 3/4 | The row has the reference structure (label left, keybind plate, gold corners, glow along the bottom), but the hover, focus and capture states look almost the same |
| 3. Color | 2/4 | The rest plate is about 1.8x brighter and warmer than the game's. The accent that should mark capture (lit tint plus HEADING caption) is barely visible. The tints were never tuned, although the spec asked for it |
| 4. Typography | 4/4 | Only 21pt and 17pt Fontin SmallCaps are used. Heading and label match the Filter section exactly |
| 5. Spacing | 3/4 | The 16pt section gaps, the 8pt frame inset and the 252pt control column are all correct, and the reset button lines up with the Filter refresh button. The reset art is still offset by its own margin (ART-04, deferred to Phase 48) |
| 6. Experience Design | 3/4 | The capture flow, cancel paths, pending-edit wiring and Apply refusal are thorough. Gaps: the accessible name never reports capture, Shift-only chords are accepted while the rejection caption suggests Shift, and the start-up fallback warning is cut off before the word "hotkey" |

**Overall: 18/24**

---

## Top 3 Priority Fixes

1. **WARNING — The capture state is hard to see.** User impact: after clicking, the only clear sign the app is waiting for keys is the caption text. The plate is about 15% brighter (mid-plate RGB 38/31/17 at rest, 44/38/25 while capturing). The caption colour moves from VALUE #ECEAE4 to HEADING #E6D3A6, a near-identical light tone. Hover uses the same lit tint, so a hovered button looks like a capturing one. Fix: make capture clearly different from hover. For example, use a stronger lit tint only while `lit` (capturing), and a smaller step for hover and focus. Use a clearly gold caption for capture (`pal::GOLD` #C8A25A, or a pale amber well away from VALUE). Then re-shoot `--capture-hotkey` and compare it with `rest.png`.
2. **WARNING — The rest plate is too bright and too brown compared with the reference.** User impact: the button reads as a flat brown slab, not the game's near-black plate with a gold line along the bottom. Measured mid-plate: ours 38/31/17, reference 21/18/13. The spec says "check against options-keybind-rows.png … tune by eye if the plate reads too bright"; 47-04-SUMMARY kept the starting values. Fix: darken the rest tint in `src/skin/widgets.rs` (`ff_keybind_button`, the `pal::GOLD` branch, line ~1247). For example, use `GOLD.gamma_multiply(~0.6)`, or a dedicated `KEYBIND_REST` near #7A6438, so the mid plate lands near 21/18/13 while the bottom glow stays visible. Record the final value in the deferred list.
3. **WARNING — `Used by Windows or FerrisFilter` does not fit the button.** User impact: pressing Ctrl+C, Ctrl+V, Alt+F4, Alt+Tab or another reserved chord shows a cut-off amber caption (about 31 characters at roughly 7.5pt each, about 230pt, against the 184pt box set in `ff_keybind_button`, `max_width: width - 24`). The explanation is lost, and no tooltip says which chords are reserved. This caption is not in the UI-SPEC copy table either. Fix: shorten it to fit in about 20 characters, for example `Reserved key combination` or `Already used by Windows`. Optionally add one clause to the capture tooltip (`Ctrl+C/V/X/A/Z, Alt+F4 and Alt+Tab are reserved.`), and add the caption to the UI-SPEC copy table. `Windows key not allowed` (23 characters, about 175pt) also sits right at the limit and should be checked once with a real capture.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

These match the contract exactly:
- `Overlay` heading, `Show / hide overlay` label (`config_view.rs:387`), the rest and capture tooltips (`config_view.rs:123-126`), `Reset to Ctrl+Shift+Z` with the accessible name `Reset hotkey` (`config_view.rs:127,428`).
- `Press a key…`, `Add Ctrl, Shift or Alt`, `Windows key not allowed`, `Key not supported` (`chord.rs:595,616-618`).
- The Apply outcome ` Show / hide is now <chord>.` (`app.rs:776`).
- The two in-use refusals (`hotkey.rs:180,183`), the settings-unusable refusal (`apply_all.rs:33`), and the tray `FerrisFilter — <chord> to toggle` / `… unavailable: <reason>` (`hotkey.rs:235,246`).

Findings:
- **WARNING:** `chord.rs:619` adds `Used by Windows or FerrisFilter` (RejectReason::Reserved). It is outside the contract and does not fit the 184pt caption width (see Top fix 3). The same reason covers egui's Copy, Cut and Paste events (`chord.rs:711`). That is right in function, but for Ctrl+A or Ctrl+Z "Used by Windows" reads oddly, and the caption gives no hint of what to do next.
- **WARNING:** The start-up fallback warning is composed as `ff-settings.json could not be fully used; `hotkey` is not a usable key combination, so Ctrl+Shift+Z is used.` (`settings.rs:418`). In `invalid.png` the one-line status shows `ff-settings.json could not be fully …`, so the word that matters ("hotkey") is never visible without hovering. This follows the spec's "same form as the other key warnings", so it is a house-pattern issue, but it is the only feedback for a broken saved chord. Consider leading with `hotkey` when it is the only bad key.
- **Minor:** The `could not be registered (<e>)` variants (`hotkey.rs:117,174,186,189`) are not in the copy table. They are reasonable extensions, but the copy table should list them.

### Pillar 2: Visuals (3/4)

- Against `options-keybind-rows.png`, the structure matches: SmallCaps label on the left, a wide plate with the ornate gold corner clusters on the right, the chord centred in pale text, and the glow along the bottom (rows 452-458 rise 43 -> 84 in red, like the reference's 32 -> 88). The extracted art (`keybind_border.png` / `keybind_plate.png`, 60x60, trimmed) renders cleanly, with no white interior and no fringe.
- **WARNING:** State differentiation (Top fix 1). `rest.png` and `capture.png` differ only in caption text at normal viewing size. Hover and focus share the capture tint (`widgets.rs:1244`), so the "you are capturing" signal is not unique.
- **WARNING:** The pale Ferris background line art runs straight behind the `Show / hide overlay` label and the keybind plate (`capture-crop` shows the crown spikes crossing the label). The Filter frame sits above the drawing, so this is the first CONFIG row where the line art competes with text. This is not a phase-47 defect as such (the frame uses the same `pal::ROW_EVEN` fill as Filter), but the new section moved Styles down and placed a row over the busiest part of the drawing.
- When greyed, the reset button is just a darker plate with a dim glyph. That is consistent with the Filter refresh button and passes.

### Pillar 3: Color (2/4)

- Palette roles in code match the spec: rest `pal::GOLD`, lit `#E6C882` (`widgets.rs:1215`), disabled GOLD at alpha 90, the border at white or white alpha 90, captions VALUE / HEADING / WARN (`config_view.rs:389-393`). The accent is used only on the keybind button. No hardcoded colours outside those two constants.
- **WARNING (spec process gap):** the spec said "The starting tints above must be checked against options-keybind-rows.png … and tuned by eye if the plate reads too bright or too flat". Measured, it is too bright: mid plate 38/31/17 against the reference's 21/18/13, about 1.8x, with a warmer cast. 47-04-SUMMARY reports "reads close" and kept the starting values. That judgement does not survive measurement.
- **WARNING:** The accent fails its one job. The spec reserves the accent for "the capture state only", but the measured lit-to-rest change is 6/7/8 levels on the plate, and HEADING against VALUE is a cream-to-white shift that is hard to see at 17pt. The 10% accent is effectively invisible, which is why the score is 2.
- WARN #E8B052 for rejected captions is clearly different from both. Code check only; it cannot be screenshotted.

### Pillar 4: Typography (4/4)

- The heading uses `section_title` (21pt SmallCaps, HEADING), identical to `Filter` and `Styles` in the screenshot, with its underline rule.
- The label uses `ff_row` (17pt SmallCaps, LABEL); `Show / hide overlay` fits without cutting, which confirms the choice of `ff_row` over `ff_field_row`.
- The chord caption is 17pt SmallCaps, centred, and moves down 1pt when pressed. No new sizes or faces were added.
- Finding (pass): `Ctrl+Shift+Z` is about 99pt and `Press a key…` about 92pt, well inside the 184pt caption box. The caption width problem is a copy-length problem (Pillar 1), not a type-scale one.

### Pillar 5: Spacing (3/4)

- Section gaps: the Filter frame ends at y≈347 and the Overlay heading starts at y≈392. The Overlay frame ends at y≈470 and the Styles heading starts at y≈514. The two gaps match each other and the earlier Filter-to-Styles gap (`ui.add_space(16.0)`).
- Frame: `inner_margin(8)` and `fill(pal::ROW_EVEN)`, the same as Filter (`config_view.rs:382-384`).
- Control column: 208pt button + `FIELD_GAP` 10pt + 34pt reset = 252pt (`config_view.rs:129,388`). The reset right edge lines up with the Filter refresh button at x≈560, as the spec requires.
- **WARNING (deferred, known):** the reset button's visible plate sits about 1-2pt high and left of the 30pt keybind plate's centre line, because `ff_reload_button`'s transparent right and bottom margin is not compensated (spec judgement call 5, ART-04, Phase 48). This is acceptable for this phase, but it is visible at 2x zoom, and Phase 48 must re-check this row together with the Filter refresh button.
- No arbitrary spacing values were added.

### Pillar 6: Experience Design (3/4)

Strong points, checked in code and backed by tests:
- Key events are drained at the top of each frame, so no other widget sees capture keys.
- Every cancel path in the spec works: outside press, second click, tab switch, hide, focus loss and popup.
- The live hotkey becomes the captured chord instead of toggling.
- Reset cancels capture first, and Reset is greyed at the default.
- A chord is a counted CONFIG edit (tab ` *`, Revert, Quit popup).
- A chord in use refuses the whole Apply without writing anything.
- The tray and header tooltips follow the live chord, and the second-launch message names no chord.

Findings:
- **WARNING:** The accessible name stays `Show / hide overlay: <chord>` while capturing (`config_view.rs:394` uses `view.chord_text`, not the caption). A screen-reader or keyboard user who starts capture with Space or Enter gets no announcement that the button is now waiting for keys or has rejected a press. Fix: use `Show / hide overlay: Press a key…`, or the rejected caption, while `view.capturing`.
- **WARNING:** Shift-only chords on printable keys (Shift+Z, Shift+1, Shift+Space) are accepted. While the game is in front, they toggle the overlay when the player types a capital letter in chat. The `Add Ctrl, Shift or Alt` caption steers users toward them. This is review WR-05, deferred by author decision to AUTONOMOUS-V072-DEFERRED.md. It is listed here because the copy itself is part of the problem: if the author picks "reject", the caption becomes `Add Ctrl or Alt`.
- **WARNING:** The rejected-reason caption for reserved chords is cut off (Top fix 3), so the "why" is lost at exactly the moment the user needs it.
- **Minor:** The button never shows a pending chord as different from the applied one. This is by design (house rule: no edited tags), and the tab ` *` plus the unapplied indicator carry it, so it is not scored down.
- **Minor:** Non-US punctuation keys register a different key from the one shown (WR-02, deferred, documented as a known limitation).

Registry audit: not applicable (no shadcn, no third-party registries).

---

## Files Audited

- `.planning/phases/47-customizable-hotkey/47-UI-SPEC.md`, `47-CONTEXT.md`, `47-01..04-PLAN.md`, `47-01..04-SUMMARY.md`, `47-REVIEW-FIX.iter2.md` / `.iter3.md`, `47-VERIFICATION.md`
- `src/overlay/config_view.rs` (`show_overlay`, the HotkeyRowView / HotkeyCaption types, tooltip constants)
- `src/skin/widgets.rs` (`ff_keybind_button`, `KEYBIND_LIT`, `KEYBIND_CORNER_TEXELS`)
- `src/overlay/chord.rs` (RESERVED, RejectReason captions, held_caption, capture_step)
- `src/overlay/hotkey.rs` (tooltip, failure and refusal copy)
- `src/settings.rs` (hotkey warnings)
- `src/overlay/apply_all.rs` (SETTINGS_UNUSABLE), `src/overlay/app.rs` (Apply outcome, header tooltip)
- Screenshots: `C:\Users\jonat\AppData\Local\Temp\claude\ff47-shots\rest.png`, `custom.png`, `capture.png`, `invalid.png`
- Reference: `.planning/research/ui-refs/options-keybind-rows.png`
