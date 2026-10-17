---
phase: 33-style-table-line-by-line-style-editor
reviewed: 2026-10-16T19:30:00Z
depth: standard
iteration: 2
files_reviewed: 10
files_reviewed_list:
  - src/overlay/app.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/screenshot.rs
  - src/overlay/widgets.rs
  - src/skin/colour_picker.rs
  - src/skin/editor.rs
  - src/skin/minimap.rs
  - src/skin/mod.rs
  - src/skin/popup.rs
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 33: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I read `git diff 576d456..HEAD -- src CLAUDE.md` (fix commits e7b4c66, 2895bee, b1993e3, 5b6c83c,
b98445b, 0ce58b2) and traced each change against its iteration-1 finding and its callers
(`colour_popup` / `ff_colour_picker`, `colour_chip` / `colour_slot`, `rgba_edit` / `values_edit` /
`grammar::encode`, `ConfigUi::expand` / `set_expanded` / `reset_for_new_styles`, the
`remember` / `recall` epoch memory, `ConfigState::edit_style`).

I also ran two checks with `CARGO_TARGET_DIR` set to the temp folder:
- `cargo fmt --check` is clean.
- `cargo test --lib` passes: 1297 tests, 0 failures.

All five in-scope iteration-1 findings are fixed for the paths the review described, and I found no
regression in the fixed code. Two warnings remain:
- **WR-01:** CR-01 is only partly closed. On an unset slot, typing the seed's hex or R/G/B value
  still does nothing, which was the third no-op path that CR-01 named.
- **WR-02:** The switch-back memory that WR-02 and WR-03 build on lasts only while the row stays
  expanded. Collapsing the row, or expanding another one, bumps the epoch and erases it. CLAUDE.md
  and the `memo_id` comment both promise memory for the whole session.

The five iteration-1 Info items were out of the fixer's scope and are all still present. They are
listed again below.

## Verification of iteration-1 findings

| ID | Verdict | Evidence |
|----|---------|----------|
| CR-01 unset slot cannot take the seed colour | Fixed (partly, see WR-01) | `PickerResult.picked` is set by any quick-pick click, including the selected tile (`colour_picker.rs:519-539`). `colour_popup` emits `Set(working colour)` when `res.changed \|\| ((res.done \|\| res.picked) && current.is_none())` (`colour_picker.rs:656`). A set slot is unchanged: `current.is_none()` guards the new branch, and the `Some(now) != current` check still drops no-ops. Escape after a seed commit gives `opening == None != current`, so it returns `Clear`, which is correct. Tests: `use_colour_on_an_unset_slot_sets_the_seed` and `the_selected_quick_pick_on_an_unset_slot_sets_it`. The hex / R/G/B path is not covered (WR-01). |
| CR-02 Escape rewrites a 3-token line as 4 tokens | Fixed | `colour_chip` writes `had_alpha` under `popup_id.with("had-alpha")` only while the popup is closed (`widgets.rs:416-419`). On the opening click it is captured before `colour_popup` toggles the popup open, so the stored form is the opening form. `rgba_edit` now encodes with that form (`body.rs:2404-2418`, caller `body.rs:2887-2888`). `grammar::encode` writes 4 tokens only when `had_alpha \|\| a != 255` (`grammar.rs:559`), so alpha 128 followed by Escape gives back `1 2 3`, and `values_edit` compares text against the current `1 2 3 128` and emits the edit. Tests: `body::escape_after_an_alpha_edit_restores_a_three_token_line` (byte-equal `wf.text()`) and `widgets::escape_after_an_alpha_edit_reports_the_opening_form` (real chip, alpha click, Escape). `colour_swatch_rgb` passes `false`, which is right for a fixed alpha of 255. |
| WR-01 + Add style collapses the new row | Fixed | `style_draft_row` no longer sets `cfg.expanded` (`config_view.rs:1048-1054`). The handler calls the new non-toggle `ConfigUi::expand` (`app.rs:1945`), which drops the armed delete and bumps the epoch as `set_expanded` does. Test `app::adding_a_style_expands_it` covers both the pre-expanded and the not-expanded case. |
| WR-02 sound file lost through an Alert | Fixed (within one epoch, see WR-02) | `switch_sound` stores `memo.file` on any switch away from Custom and `memo.last` on a switch to None. `sound_for_choice(SOUND_CUSTOM, ..)` uses `base.custom`, then `last_file`, then `uwu.mp3` (`config_view.rs:1453-1497`). I traced Custom→Alert→Custom, Custom→None→Alert→Custom, and file-edit-then-switch: all keep the right file and the current volume. The `"sound-file"` memo id does not collide with the file-draft id (`"cfg-style-file-draft"`). Test `switching_back_to_custom_keeps_the_file`. |
| WR-03 Font off/on resets to 32 | Fixed (within one epoch, see WR-02) | Turning the check off remembers `style.font` (`u8`), and turning it on recalls `u8`, so the types match (`config_view.rs:1213-1224`, `models/config.rs:176`). The fallback is 32. Test `font_off_then_on_restores_the_last_size` (S 37→None→37). |
| IN-01 dead `colour_swatch_rgb` / `rgb_caption` | Open | Still only used by tests. The CR-02 fix even updated its signature (`widgets.rs:497-518`). |
| IN-02 table columns overflow on narrow widths | Open | Unchanged. |
| IN-03 editor memory keyed by name | Open | Unchanged. The new `"font"` and `"sound-file"` memos widen it. |
| IN-04 bad or empty sound file silently dropped on blur | Open | Unchanged (`config_view.rs:1553-1568`). |
| IN-05 unused `skin` in `beam_line` | Open | `let _ = skin;` is still at `config_view.rs:1390`. |

## Warnings

### WR-01: On an unset slot, typing the seed's hex or R/G/B value still commits nothing (CR-01 residue)

**File:** `src/skin/colour_picker.rs:446-485, 656`
**Issue:** Iteration-1 CR-01 named three ways to commit the seed colour of an unset slot that did
nothing: **Use colour**, the selected quick pick, and "typing the same hex or R/G/B value". The fix
covers the first two through `res.done` / `res.picked`. Typed values are still dropped:
- `value_field(..) && parse_hex(&t) && v != rgb` discards a hex equal to the seed.
- The R/G/B fields likewise check `v != rgb[k]`.

So `res.changed` stays false and neither `done` nor `picked` is set.

On an unset Text slot that opens on `#F8F8F2`, a user who types `#F8F8F2`, presses Enter and then
clicks outside ends with the slot still `(none)` and no message. CLAUDE.md now says only Use colour
and the quick picks set the colour shown on an empty slot, so this is documented, but it still
breaks the "an explicit choice commits" rule the fix set up. The fix report marks CR-01 fixed
without mentioning this path.
**Fix:** Report any committed typed value as an explicit choice, the same way as `picked`:
```rust
// ff_colour_picker: set a flag whenever a hex / channel field commits a parseable value,
// even when it equals the working colour
let mut entered = false;
if let Some(t) = value_field(..) && let Some(v) = parse_hex(&t) {
    entered = true;
    if v != rgb { typed = Some(v); }
}
// ... same for R/G/B ...
res.picked |= entered;   // or a separate `res.entered`
```
Then the existing `(res.done || res.picked) && current.is_none()` branch commits it. Add a test:
open with `current = None`, type the seed hex plus Enter, and expect `Set(seed)`.

### WR-02: Switch-back memory (minimap, beam, sound, sound file, font) is erased when any style row is collapsed or expanded, not kept "for the session"

**File:** `src/overlay/config_view.rs:106-122, 961-963, 1113-1136`
**Issue:** `memo_id` salts every remembered value with `cfg.epoch`. The epoch is bumped by:
- `set_expanded`, which runs on every row-header click (line 962);
- `expand`, used by + Add style;
- `reset_for_new_styles`, used by Revert, Reload, Apply and outside edits.

So the memory lives only while one row stays expanded. Example:
1. Turn Font size off on `S` (37 is remembered).
2. Click the `S` header to collapse it, then click it again to expand it.
3. Turn Font size back on. The result is **32**, and `S` stays dirty.

The same happens to a minimap, beam, sound or custom file after a collapse. Opening another row
and coming back has the same effect, because each header click bumps the epoch.

This contradicts:
- the `memo_id` doc comment: "kept for the session (until the next Revert / Reload, which bumps the
  epoch)";
- CLAUDE.md: "restores the last values of this session (a custom sound file is kept even through an
  Alert)".

The WR-02 and WR-03 fixes rely on this memory, and their tests stay inside one epoch, so they do
not catch it.
**Fix:** Separate the memo salt from the per-row widget epoch. Keep a `memo_epoch` in `ConfigUi`
that only `reset_for_new_styles` bumps (Revert, Reload, Apply), and use it in
`remember`/`recall`:
```rust
pub struct ConfigUi { /* ... */ pub memo_epoch: u64 }
pub fn reset_for_new_styles(&mut self) {
    /* ... */ self.epoch = self.epoch.wrapping_add(1);
    self.memo_epoch = self.memo_epoch.wrapping_add(1);
}
// style_editor: let memo = cfg.memo_epoch; pass `memo` (not `epoch`) to remember/recall
```
Alternatively, narrow the wording in CLAUDE.md and the comment to "while the row stays open". Add a
test: font off, `set_expanded("S")` twice, font on, and expect `Font(Some(37))`.

## Info

### IN-01: `colour_swatch_rgb` and `rgb_caption` are still dead in production code

**File:** `src/overlay/widgets.rs:496-518`
**Issue:** Only tests call them. The CR-02 fix kept them alive by threading a `false` form
through `colour_chip`. The trailing `.filter(|c| *c != rgb)` still repeats the check that
`colour_chip` already makes.
**Fix:** Remove both functions and their tests, or mark them `#[cfg(test)]`.

### IN-02: Styles-table columns and editor tiles overflow below about 394pt plus padding

**File:** `src/overlay/config_view.rs` (`table_columns`, `tile_size`)
**Issue:** This is unchanged from iteration 1. Nothing clamps the total column width, or the 12
beam tiles plus Temp, to the row width. It is latent at the fixed 660pt window.
**Fix:** Clamp the last column (and `tile_size`) to `width - pad`, and add a narrow-width test.

### IN-03: Editor memory, file drafts and row ids are keyed by style name, not id

**File:** `src/overlay/config_view.rs:1115-1116, 1540`
**Issue:** This is unchanged, and the new `"font"` and `"sound-file"` memos add to it. A rename
within one epoch loses the remembered values: `RenameStyle` does not bump the epoch, and the memo
key changes with the name. A style later renamed to the freed name inherits them.
**Fix:** Key the memos and drafts on `state.style_id(name).unwrap_or(name)`.

### IN-04: Invalid or emptied custom sound file text is dropped silently on focus loss

**File:** `src/overlay/config_view.rs:1553-1568`
**Issue:** This is unchanged. The draft is removed on blur and the field snaps back to the old file
with no message.
**Fix:** Keep the draft and its red outline until it becomes valid or the row collapses, or post a
short status hint.

### IN-05: Unused `skin` parameter in `beam_line`

**File:** `src/overlay/config_view.rs:1381-1390`
**Issue:** `let _ = skin;` still silences an unused parameter.
**Fix:** Drop the parameter from `beam_line` and from its call site at line 1253.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Iteration: 2_
