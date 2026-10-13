# Phase 21 — UI Review

**Audited:** 2026-10-13
**Baseline:** 21-UI-SPEC.md (design contract)
**Screenshots:** not captured (native egui overlay, GUI not launched). Code-only audit of `src/overlay/{config_view,app,widgets,body,theme}.rs`.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Contract strings are near-verbatim. `Enter a whole number` replaces `Enter a number` in the style editor. Tier errors are unchecked here. |
| 2. Visuals | 3/4 | Tab strip, preview chip and accordion match the contract. Tier column captions are misaligned. Style rows have no expand affordance. |
| 3. Color | 3/4 | Accent use stays within the reserved list. Style colours are RGB-only, where the spec asks for RGBA. |
| 4. Typography | 3/4 | Four sizes and two faces held. Literal `14.0` sizes are scattered. Footer buttons use the default size rather than a named token. |
| 5. Spacing | 3/4 | 4/8/16 scale respected. The tier header row is not inset like its rows, and the edge stroke is painted into the margin. |
| 6. Experience Design | 3/4 | Guards, drafts and disabled states are well covered. No focus management on drafts. Apply ignores open invalid drafts. Apply-blocked tooltip covers one cause only. |

**Overall: 18/24.** No BLOCKERs. All findings are WARNING.

---

## Top Priority Fixes

1. **Tier column captions misaligned with their rows** (config_view.rs:415-422). The caption `ui.horizontal` is not inset, but each row sits inside `zebra_row`'s 8px inner margin (widgets.rs:97). `TIER`, `MIN VALUE` and `UNIT` sit 8px left of their columns. CHEAP (about 2 lines): wrap the caption row in `Frame::new().inner_margin(Margin::symmetric(8, 0))`, or `ui.add_space(8.0)` as the first item.
2. **Name drafts do not take focus when opened** (config_view.rs:316-318, 499-501, 595-600). The spec says focus the first control. Pressing `+ ADD STYLE` or `New empty filter…` leaves the keyboard on the button. CHEAP (about 3 lines): when the draft is created, set a one-shot flag and call `edit.request_focus()`. Alternatively `ui.memory_mut(|m| m.request_focus(id))` when the draft is first `Some("")`.
3. **Apply is not blocked by an open, invalid pending name field** (config_state.rs:349 `can_apply` has no knowledge of `ConfigUi` drafts). The spec says Apply is disabled while any pending name field is invalid. Today an open `Add style` draft is silently ignored by Apply. Moderate (about 6 lines): pass a `drafts_invalid` bool into `footer_buttons` and `show_footer`.
4. **Apply-disabled tooltip covers only tier errors** (config_view.rs:1085). A dirty-free Apply, a load error, or `styles_error` gives a greyed button with no reason. CHEAP (about 4 lines): add `on_disabled_hover_text` variants, for example `No changes to apply.`
5. **Style colours are RGB-only** (widgets.rs:479 `colour_swatch_rgb`; config_view.rs:705). The contract asks for RGBA swatches, `rgba_caption` and an alpha picker. The implementation chose `Alpha::Opaque` because `Style` stores RGB and the emitter writes 255. That is defensible, but it is an undocumented deviation from the approved spec and creates a second swatch widget (the spec says no second implementation). Record it in the phase decisions. Do not change the code.
6. **Style row has no expand/collapse affordance** (config_view.rs:551-563). The row is click-anywhere with no chevron, no pointer cursor and no `widget_info`, so the accordion is not discoverable. The spec forbids arrows and toggles but does not forbid a hover cue. CHEAP (about 2 lines): `row.on_hover_cursor(CursorIcon::PointingHand)` plus `row.widget_info(...)` with an expanded state.
7. **Tier input is a bare `TextEdit`** (config_view.rs:434-438), not the `number_field` the spec names. This is fine functionally, since the buffer is separate and focus-safe. Be aware the spec's `Enter a number` appears only via `row.error`, and its wording depends on `ConfigState`.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Verbatim matches: `GAME FOLDER FILTER`, `New empty filter…`, helper lines (config_view.rs:351, 413, 481, 506), `Switch and discard` / `Stay`, `Create filter`, `Discard new filter`, `Add style`, `Discard new style`, `Delete style` / `Keep style`, `Built-in styles can't be deleted.`, `Names can't be changed…`, the tier helper and footnote, and the empty-styles line.
- Copy deviations (WARNING): `Enter a whole number` (config_view.rs:27) replaces the spec's `Enter a number`. It is defensible for integer sliders. The font hint is reused through `body::font_hint`. The footer's armed-Revert prompt adds a singular form (`1 unapplied config change`), a harmless extension.
- Extra copy not in the contract: the `new` and `edited` badge tooltips, and a `config_warning` line appended to the delete prompt (config_view.rs:167-170). It is useful and not contradictory.
- Footer prompt placement: the outside-edit and revert prompts are drawn as bare labels above the buttons (config_view.rs:1057-1072), not in the outcome area. This is acceptable, but the prompt is not in the capped scroll area, so a long message can grow the pinned footer past the 96px cap.

### Pillar 2: Visuals (3/4)
- Tab strip (app.rs:213-285): 28px segments, `HEADER_PLATE` fill, 2px `SECTION_CREAM` underline as a non-colour twin, `*` marker with tooltip, arrow-key navigation and `widget_info`. This matches the contract.
- Preview chip (widgets.rs:518-585): 120x24, checkerboard, 2px border, `Item Name` clamped 12-18, Hide drawn at half strength with a strike line, tooltip and accessible name. It matches the contract. Minor: the chip applies `fade` to the border and text only, not the background, so a Hide style's background stays full-strength. The spec says text and border at 50%, so this is correct.
- Misalignment (WARNING): tier column captions are not inset (fix 1).
- Affordance (WARNING): fix 6.
- The tier row's accent edge stroke is drawn after the row at `left-8 .. left-4` (config_view.rs:471-475). That is the outer margin gutter, outside the zebra frame, and may clip against the scroll area edge. Unverified without a render; flag for human check.
- Colour swatch row: `circular_toggle` has no text label and relies on the neighbouring caption. The spec says an `Off` toggle with `Not set (game default)` text. A bare circle plus caption is readable but the toggle polarity (on means set) is implicit.

### Pillar 3: Color (3/4)
- `WARM_ACCENT` fill appears only on `Switch and discard`, `Delete style`, `Discard`, `Overwrite` (config_view.rs:224-227, 1078-1083) and the tier edge stroke. That matches the reserved list.
- `WARM_ACCENT_TEXT` is used for tier errors, load errors and the outside-edit line. No raw-accent text.
- Hard-coded colours: no raw hex in config_view.rs. widgets.rs uses `from_gray(0x2A/0x1A)` for the checkerboard. This is content-adjacent and acceptable.
- Deviation: RGB-only swatches (top fix 5).
- Out-of-date line at 14px `VALUE_TEXT` has no accent, as specified.

### Pillar 4: Typography (3/4)
- Sizes in use are 11 (`.small()` captions), 13 (`ROW_SUMMARY_SIZE`), 14 (literal) and 18 (`section_header`), plus Cinzel on tabs. This matches the four sizes.
- Weights: no bold. Good.
- WARNING: literal `14.0` is repeated in config_view.rs (430, 531, 659, 1034) instead of a theme token. Footer buttons use `RichText::new(label)` at the default size (1079), while the other buttons use 13px. `Apply` and `Revert` are 14px against 13px for every other button in the tab. The spec does not name a footer size, so this is inconsistent rather than wrong.
- The Cinzel `FILTER` and `TIERS` headers sit directly under their parent with no extra gap. The `STYLES` header shares a line with the button. Headers are consistent in face and size.

### Pillar 5: Spacing (3/4)
- `add_space` values are 16 (between sections), 4 and 8 only. Margins are `Margin::same(8)`. Item spacing is 4 or 8. No off-scale values introduced in this file.
- WARNING: tier header caption misalignment (fix 1). The header uses fixed 24/112/64 widths that match the row widths but not their inset.
- Literal sizes `24.0`, `28.0`, `96.0`, `160.0` are within spec exceptions. `desired_width(160.0)` for the custom-sound file field is not in the spec.
- Minimap row widths: 88 + colour combo + 104 + 2x8 gaps. The colour combo width is set in `body::colour_combo`, and the total could exceed 424px. Needs a render check.
- Inherited off-scale items from widgets.rs (`Margin::symmetric(10, 6)`, badge `(4, 1)`) remain from Phase 20.

### Pillar 6: Experience Design (3/4)
- Strong: the switch guard is inline and never modal. The delete confirm disarms on row change. The Revert confirm cannot outlive its edits (config_view.rs:1052-1054). The outside-edit guard offers Reload, Overwrite and Keep editing. Drafts persist across a rejected name. The no-game-folder state disables the combo. A load-error view exists with a retry. There are headless tests for idle frames, footer on-screen and the missing folder.
- WARNING: no focus management (fix 2). The spec's "focus its first control" and "focus returns to the originating control" are unimplemented.
- WARNING: Apply does not account for open invalid drafts (fix 3).
- WARNING: Apply-disabled reason covers one cause (fix 4).
- WARNING: `Esc` discard relies on `lost_focus()`. It only works when the text field has focus, not when the user is on the draft's buttons.
- Style editor `Font` and `Minimap` toggles seed fixed values (32, White/Circle/size 2). These are reasonable but undocumented defaults.
- No loading state is needed (synchronous file reads).
- Registry audit: skipped, shadcn not initialised.

---

## Files Audited
- src/overlay/config_view.rs
- src/overlay/app.rs (tab_strip 213-285, config_body 1039-1109, tab wiring 1380-1460)
- src/overlay/widgets.rs (zebra_row, colour_swatch_rgb, style_preview_chip)
- src/overlay/config_state.rs (can_apply)
- .planning/phases/21-overlay-configuration-tab/21-UI-SPEC.md
- .planning/phases/20-full-grammar-block-editing/20-UI-REVIEW.md (consistency)

Not read in full: body.rs, theme.rs, the 21-0* SUMMARY/PLAN files, 21-CONTEXT.md. Findings that depend on them (the minimap row width, the `colour_combo` width) are marked as needing a render check.
