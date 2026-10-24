---
phase: 43-style-alpha
reviewed: 2026-10-23T22:01:53Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/defaults.rs
  - src/emitter.rs
  - src/loader.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/body.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/style_lines.rs
  - src/overlay/widgets.rs
  - src/skin/colour_picker.rs
  - src/skin/editor.rs
  - src/styled.rs
  - styles-default.json
  - CLAUDE.md
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 43: Code Review Report

**Reviewed:** 2026-10-23T22:01:53Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the diff `b62741f..HEAD` for the listed files. I also ran `cargo test --locked --lib`
(separate target dir): 1704 passed, 0 failed.

The core of the phase holds up when traced:
- `StyleColour` serde reads 3 or 4 values and rejects 2 or 5+ values and out-of-range values.
  It always writes 4 values.
- Equality covers all four channels, so `styles_at_defaults`, the dirty count, strictness and
  the inline `config.json` styles equality all see alpha. A 3-value value equals the matching
  4-value one.
- `prepare_write` keeps the loaded bytes when nothing changed, so a 3-value `styles.json` does
  not count as an edit. A test covers this.
- At alpha 255 the emitter still writes `R G B 255`, so the frozen 3-value
  `seed-styles.json` / `artifact_guard` path is byte-identical.
- The card caption contrast uses `composite_over` (premultiplied over `INPUT_FILL`), and its
  maths is correct.

No blockers found. The findings are a format-compatibility risk, a gap in the docs about where
style background alpha is silently dropped, and some leftover code from removing the opaque
picker path.

## Warnings

### WR-01: 4-value `styles.json` cannot be read by any earlier release (one-way format change)

**File:** `src/models/config.rs:238-299`, `src/overlay/config_state.rs:700-712`
**Issue:** Earlier releases deserialise `text` / `border` / `background` as `Option<[u8; 3]>`.
serde rejects a 4-element array for that type ("invalid length 4, expected an array of length 3").
This build writes 4 values in two cases:
- any Apply with CONFIG edits;
- any Apply at all while `ids_unpersisted` is set (an id-less older file, `prepare_write_inner`
  line 702).

After that, downgrading (reinstalling an older setup.exe, or running an older `ff-cli apply` /
`generate` against the same FerrisFilter folder) fails to load `styles.json`. The older overlay
then shows the styles load error and refuses to work. The user did not choose this: the second
case triggers on a rules-only Apply. The author chose "always write 4", but the effect on a
downgrade is not written down anywhere (CONTEXT, CLAUDE.md, or a note for the Release Review).
**Fix:** Keep the format, but make the cost visible. Add a note to the milestone's Release Review
checklist (STATE/ROADMAP) so the changelog draft says that `styles.json` saved by this version
needs this version or newer, and that deleting `styles.json` (or Reset to defaults) recovers.
Optionally, use a tolerant error message: when a style colour fails with `invalid length 4`
in an older-format context, mention the newer format. That only helps future versions, so the
release note is the real fix.

### WR-02: CLAUDE.md says the style alpha reaches every emitted block, but some blocks drop the style background

**File:** `CLAUDE.md:231`; `src/emitter.rs:570-575`, `src/emitter.rs:634-640`
**Issue:** The new CLAUDE.md text says "the alpha reaches `generate`, the economy section and
styled rules". Not every block carries the style background:
- Economy blocks for categories with a `background_override` (Uncut Gems, Lineage Support Gems,
  `pricer.rs:59-65`) clear the style's background and write a hard-coded
  `SetBackgroundColor r g b 255` (emitter.rs:574).
- Price-tiered unique blocks write the bronze `175 96 37 240` instead (emitter.rs:640).

So a user who makes tier S's background translucent sees it on currency blocks, but not on gem or
unique blocks. The overlay gives no hint of this, and CLAUDE.md is the reference the project keeps
accurate. The unique bronze background is documented elsewhere. The gem-category override
(always alpha 255) is not.
**Fix:** Add a qualifier to the CLAUDE.md sentence, e.g. "... and styled rules (except
backgrounds that the economy replaces: the bronze on price-tiered uniques and the fixed
backgrounds of the Uncut Gems / Lineage Support Gems blocks)". Behaviour can stay as is, since
CONTEXT keeps these as emitter constants.

## Info

### IN-01: `PickerMode::alpha` is now always `true` in production

**File:** `src/skin/colour_picker.rs:31-36`, `src/overlay/widgets.rs:341`, `src/overlay/widgets.rs:390`
**Issue:** The opaque style path (`colour_slot`, `colour_swatch_rgb`) was removed, so every
production caller passes `alpha: true`. Only the test `alpha_and_none_follow_the_mode`
(colour_picker.rs:920) still uses `alpha: false`. The field and its doc comment now describe a
mode that nothing uses.
**Fix:** Remove the `alpha` field (always show the slider) and the `if mode.alpha` branch at
colour_picker.rs:498, or keep it and note that it exists only for tests or future callers.

### IN-02: `ff_colour_card` / `ff_colour_card_ex` have no production callers

**File:** `src/skin/editor.rs:332-346`
**Issue:** `colour_slot` was the last production caller of `ff_colour_card_ex`. Now only the
editor test at editor.rs:697 calls it, and `ff_colour_card` already had no callers before this
phase. These are two thin public wrappers around `ff_colour_card_tip` that nothing uses.
**Fix:** Fold them into `ff_colour_card_tip` (or rename that one to `ff_colour_card`) and update
the test.

### IN-03: The length error says 5 for any array longer than 4

**File:** `src/models/config.rs:288-290`
**Issue:** `de::Error::invalid_length(5, &self)` gives "invalid length 5" even for `[1,2,3,4,5,6,7]`.
This is harmless, but the error message is wrong.
**Fix:** Count the remaining elements (`while s.next_element::<de::IgnoredAny>()?.is_some() { n += 1 }`)
and report `n`, or word it as "more than 4 values" with `de::Error::custom`.

### IN-04: Quick picks keep the slot's alpha (undocumented, can look like nothing happened)

**File:** `src/overlay/widgets.rs:367-404` (exercised by `colour_slot_quick_pick_keeps_alpha_and_none_tile_clears`)
**Issue:** On a style slot with alpha 0 (or very low), a Dracula quick pick keeps that alpha. The
card and preview stay invisible, and the only sign of the change is the tooltip. CLAUDE.md
describes quick picks only for empty slots. This now applies to all style colours, not only RULES
lines.
**Fix:** Document "a quick pick keeps the current alpha" in the Colour picker note. Or reset alpha
to 255 when the current alpha is 0.

### IN-05: The R G B A tooltip now also shows on RULES Custom colour cards, which the RULES note does not mention

**File:** `src/overlay/widgets.rs:371`, `CLAUDE.md` (RULES editor note)
**Issue:** `colour_slot_rgba` is shared, so the RULES Custom Text / Border / Background cards also
gained the `R G B A` hover tooltip (alpha below 255) and the composited caption. The CLAUDE.md
change describes this only in the CONFIG style note.
**Fix:** Add a few words to the RULES Custom style-lines note ("a translucent card shows
`R G B A` on hover").

---

_Reviewed: 2026-10-23T22:01:53Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
