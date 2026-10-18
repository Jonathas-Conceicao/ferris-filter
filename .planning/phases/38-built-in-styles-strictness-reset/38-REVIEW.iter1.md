---
phase: 38-built-in-styles-strictness-reset
reviewed: 2026-10-18T00:00:00Z
depth: standard
files_reviewed: 29
files_reviewed_list:
  - src/compile.rs
  - src/defaults.rs
  - src/dynamic.rs
  - src/emitter.rs
  - src/loader.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/body.rs
  - src/overlay/classes.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/overlay/strictness.rs
  - src/overlay/style_lines.rs
  - src/skin/editor.rs
  - src/styled.rs
  - styles-default.json
  - tests/cli_errors.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/fixtures/seed-styles.json
  - tests/sync_prices.rs
  - CLAUDE.md
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 38: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 29
**Status:** issues_found

## Summary

Reviewed the phase-38 diff (`87969dd..HEAD`): the new nine-built-in `styles-default.json`, the frozen
`tests/fixtures/seed-styles.json` (byte-identical to the old default, line endings aside), the
strictness model (`strictness.rs`, `ConfigState::set_strictness`/`strictness`), the style/tier reset
model, the `ff_levels` widget, the Reset buttons and the two new popups, the FIX-01 tooltip change
and the test re-pointing. I checked the shipped styles against
`.planning/research/author-styles-2026-10-17.json`. The author-locked decisions in 38-CONTEXT.md are
followed: only built-ins ship, generate tests use the frozen fixture, `generate` failing without a
live `styles.json` is documented, and strictness is one row.

The phase-38 unit tests pass (31 tests, run with `FERRISFILTER_HOME`/`FERRISFILTER_GAME_DIR` pointing
at an empty temp dir). The model logic is sound: strictness matches tier styles by built-in id, does
not touch non-tier or custom styles, and is derived rather than stored. Reset keeps custom ids and
order. Adding the unit to the tier dirty count is a real fix.

There are no blockers. The defects found:
- The disabled Reset buttons can never show their "Already at the defaults." tooltip. This is an egui
  enabled-flag mismatch.
- `reset_tiers` clears validation state without re-validating.
- One new test makes no assertion at all on one branch.
- One new byte-exact test fails on a CRLF checkout.

## Warnings

### WR-01: "Already at the defaults." tooltip on both Reset buttons never shows

**File:** `src/overlay/config_view.rs:805-809`, `src/overlay/shell.rs:705-711`
**Issue:** `ff_button_ex` shows a disabled button by giving it `Sense::hover()` inside a Ui that is
still enabled (`src/skin/widgets.rs:1168-1173`). In egui 0.36 the `Flags::ENABLED` bit comes from
the Ui's enabled state (`Context::get_response`, `res.flags.set(Flags::ENABLED, enabled)`), not from
the sense. So the greyed Reset button's `Response::enabled()` is still `true`.
`on_disabled_hover_text` opens its tooltip only when `!response.enabled()`
(`Tooltip::for_disabled`). The `else` branch therefore shows nothing, and hovering a greyed Reset
button gives no explanation. Neither the strictness tests nor the reset-button tests check tooltips,
so the tests miss this.
**Fix:** The response counts as enabled, so use `on_hover_text` in both branches (or chain both, as
`config_view.rs:383`/`:447` already do):
```rust
let tip = if enabled {
    "Restore the built-in styles. Your custom styles are kept."
} else {
    "Already at the defaults."
};
r = r.on_hover_text(tip).on_disabled_hover_text(tip);
```
Make the same change in `shell.rs` for the Tiers button.

### WR-02: `reset_tiers` clears every tier error without re-validating, and keeps a file order the defaults may not be valid in

**File:** `src/overlay/config_state.rs:331-339` (with `resync_buffers` at `:185-192`)
**Issue:** `reset_tiers` copies the default `min`/`unit` into each entry in the **current file
order**, then calls `resync_buffers`, which sets `self.errors = TierErrors::default()`.
`recompute_tier_errors` is never called. `ValidatedTiers::try_from` checks ascending cutoffs **in
file order per unit** (`models/tiers.rs:87-99`), so a `tiers.json` that loads fine in a non-`F..S`
order can turn invalid after a reset. Example: a mixed-unit file `F,E,D,C (chaos), S (exalt 1),
A (div 1), B (exalt 5)` loads. After the reset every row is chaos, and in file order the cutoffs read
`0.1,0.5,2,8,30,20,12`, which is not ascending.
- The editor then shows no edge error.
- `tiers_at_defaults()` returns `true` (it checks only `errors.row`), so the Reset button greys out.
- The problem first shows up as an Apply refusal.

Whatever the order, a reset should not leave the error state stale.
**Fix:** Re-validate after the reset. Preferably also put the entries in the default order, since
"defaults" includes an order that is known to validate:
```rust
pub fn reset_tiers(&mut self) {
    for entry in &mut self.tiers { /* copy min/unit as now */ }
    let order = |t: &str| Self::default_tiers().iter().position(|d| d.tier == t);
    self.tiers.sort_by_key(|e| order(&e.tier).unwrap_or(usize::MAX)); // optional
    self.resync_buffers();
    self.recompute_tier_errors();
}
```
If the file order must stay (the current test pins it), keep only the `recompute_tier_errors()` call,
and make `tiers_at_defaults` also require `self.errors.edges.is_empty() && self.errors.general.is_none()`.

### WR-03: `old_twelve_style_file_loads_and_keeps_its_bytes` asserts nothing on one branch

**File:** `src/overlay/config_state.rs` (test `old_twelve_style_file_loads_and_keeps_its_bytes`, `let w = s.prepare_write()...; if !w.styles_changed() { ... }`)
**Issue:** The test is named for the property "the old 12-style file keeps its bytes". It only checks
the bytes inside `if !w.styles_changed()`. If a regression made a clean load mark the styles as
changed (for example an id migration or a re-serialisation), the `if` would skip the check and the
test would still pass. The guarded property is exactly the one it claims to test.
**Fix:**
```rust
let w = s.prepare_write().unwrap();
assert!(!w.styles_changed(), "an untouched 12-style file must not be rewritten");
assert_eq!(w.styles_new, fs::read(d.path().join("styles.json")).unwrap());
```

### WR-04: `shipped_styles_round_trip_through_render_styles` fails on a CRLF checkout

**File:** `src/defaults.rs` (test `shipped_styles_round_trip_through_render_styles`)
**Issue:** The test compares `render_styles(...)` (always `\n`) byte for byte with
`STYLES_DEFAULT.text`, which is `include_str!` of the working-tree `styles-default.json`.
`.gitattributes` gives that file `* text=auto` ("check out native on each OS"). On a Windows clone
with `core.autocrlf=true` (the Git for Windows installer default), the file is checked out with CRLF
and this test fails, although nothing is wrong. Today's tree happens to be LF. The phase targets an
initial public release, so outside contributors on Windows will run into this.
**Fix:** Either pin the file's line endings in `.gitattributes` (`styles-default.json text eol=lf`,
and likewise `tiers-default.json`), or normalise in the test:
`assert_eq!(render_styles(..), STYLES_DEFAULT.text.replace("\r\n", "\n"))`.

## Info

### IN-01: The Custom level's caption looks the same as every other unselected level

**File:** `src/skin/editor.rs` (`ff_levels`, the `color` match)
**Issue:** An unselected clickable level and the display-only Custom level are both painted in
`pal::DISABLED`; only hover tells them apart. CONTEXT asks for Custom to be "display-only (… dim
caption)". In this row every unselected level reads as dim, so the six live levels look as disabled
as Custom. This matches the `ff_segmented` convention, so it may be intended, but the requirement to
mark Custom as non-clickable is not shown visually.
**Fix:** Paint unselected clickable segments in `pal::LABEL`, and keep `pal::DISABLED` for
`!level.clickable`.

### IN-02: The Custom tooltip describes the user's own mix even when Custom is not the current state

**File:** `src/overlay/strictness.rs:64-68`
**Issue:** The tooltip "Your own mix of shown and hidden tiers. Pick a level to replace it." also
shows while a preset is selected, when no own mix exists.
**Fix:** Word it so it holds in both states, for example "Shown when the tiers match no level.", or
pass the current level in to pick the text.

### IN-03: No upgrade notice for rules styled with the old shipped custom styles

**File:** `styles-default.json` (removal of `Recycle`/`kite`/`exceptional`, ids `q8m3vz`/`t5hw2k`/`n7c4rj`)
**Issue:** A user with no live `styles.json` whose `dynamic.filter` rules carry
`# ff:style q8m3vz Recycle` (and so on) will see those rules flagged `unknown style` after
upgrading. Their actions are then frozen as last written, and nothing says why. CONTEXT accepts that
these styles no longer ship and covers only the `generate` consequence. This overlay-side effect is
not documented in CLAUDE.md.
**Fix:** Add a sentence to CLAUDE.md or the CHANGELOG draft: rules tagged with the removed styles show
`unknown style` until the style is recreated (for example by copying it from
`tests/fixtures/seed-styles.json`).

### IN-04: The Reset buttons are laid over the section heading without reserving space

**File:** `src/overlay/config_view.rs:787-796`, `src/overlay/shell.rs:697-703`
**Issue:** The 34pt-high button is placed in a child Ui sized to the `ff_section` heading rect
(galley height + 12). That rect also holds the gold underline painted at `rect.max.y - 5`, so the
centred button sits over the right-hand part of the underline. Nothing is allocated in the parent,
so with a narrow window a long heading could also run under the button. This is a visual issue
only.
**Fix:** Lay the heading and the button out in one horizontal row, or reduce the button height or
offset so the underline stays clear. Check with `--screenshot --tab config --popup reset-styles`.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
