---
phase: 38-built-in-styles-strictness-reset
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 2
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
  warning: 0
  info: 5
  total: 5
status: clean
---

# Phase 38: Code Review Report (iteration 2)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 29 (scope as in iteration 1; this pass focused on `git diff e6d2f16 HEAD`)
**Status:** clean (no Critical or Warning findings remain; Info items carried over plus one new)

## Summary

This pass re-reviewed the four fix commits (efc9cb6, 85cc7ed, 78ddf24, 59f57ee) against
iteration 1's WR-01..WR-04. I traced each change into its callers: `ff_button_ex`, the
`ConfigAction::ResetTiers` dispatch, `tiers_reset_enabled`, `recompute_tier_errors`,
`ValidatedTiers::try_from`, `tiers_dirty_count`/`tiers_equal` and `revert_scope`. I also checked
whether the fixes cause regressions on this worktree's CRLF checkout:
`tests/fixtures/seed-styles.json` and `tiers-default.json` are `w/crlf`, and `styles-default.json`
is `w/lf`.

`cargo test --lib -- config_state defaults strictness` passes: 77 tests, run with
`FERRISFILTER_HOME`/`FERRISFILTER_GAME_DIR` pointing at an empty temp dir. Among them are the new
`reset_tiers_takes_default_order_when_file_order_would_not_validate` and the tightened
`old_twelve_style_file_loads_and_keeps_its_bytes`, which now also passes against the CRLF seed
fixture. That works because `styles_changed` is a semantic comparison, not a byte comparison.
`cargo fmt --check` reports no diff in phase-38 files. Its only diffs are in
`tests/isolation_guard.rs`, from phase 37, which is outside this scope.

### Resolution of iteration-1 findings

| ID | Status | Notes |
|----|--------|-------|
| WR-01 | Resolved | Both Reset buttons now attach the state-dependent tip with `on_hover_text(tip).on_disabled_hover_text(tip)`, so the tip shows whichever way the response's enabled flag is set: a greyed button in an enabled Ui, or any button in a disabled Ui. `ff_button_ex` (`src/skin/widgets.rs:1155-1171`) confirms the greyed button uses `Sense::hover()` in an enabled Ui. No tooltip test exists. This is UI-only and listed for manual hover check in the fix report. |
| WR-02 | Resolved | `reset_tiers` (`src/overlay/config_state.rs:329-345`) now calls `recompute_tier_errors()` after `resync_buffers()`. When the file order would not validate, it sorts into the default order. `tiers_at_defaults` also requires no edge error and no general error. Revert restores `tiers_applied` (the file order), and the per-frame `combined_dirty` sync (`app.rs:3223`) picks up the edit. Because the all-chaos defaults are strictly ascending, keeping the file order only happens when the file already lists the known tiers in F..S order, so the order fallback is consistent. One narrow residual case is noted as IN-05. |
| WR-03 | Resolved | The assertion is now unconditional (`config_state.rs:1740-1747`). |
| WR-04 | Resolved | The test normalises `\r\n` (`defaults.rs:186`). The other `STYLES_DEFAULT.text` uses (`defaults.rs:101,120,128,134`) parse JSON or search for a substring, so they do not depend on line endings. |

## Info

### IN-01: The Custom level's caption looks the same as every other unselected level (carried over)

**File:** `src/skin/editor.rs` (`ff_levels`, the `color` match)
**Issue:** An unselected clickable level and the display-only Custom level are both painted in
`pal::DISABLED`; only hover tells them apart. CONTEXT asks for Custom to be display-only with a dim
caption. Because every unselected level is equally dim, nothing marks Custom as not clickable. This
matches the `ff_segmented` convention, so it may be intentional.
**Fix:** Paint unselected clickable segments in `pal::LABEL`, and keep `pal::DISABLED` for
`!level.clickable`.

### IN-02: The Custom tooltip describes the user's own mix even when Custom is not the current state (carried over)

**File:** `src/overlay/strictness.rs:64-68`
**Issue:** The tooltip reads "Your own mix of shown and hidden tiers. Pick a level to replace it."
It also shows while a preset is selected, when the user has no own mix.
**Fix:** Word it so it holds in both states (for example "Shown when the tiers match no level."),
or pass the current level in and choose the text from it.

### IN-03: No upgrade notice for rules styled with the old shipped custom styles (carried over)

**File:** `styles-default.json` (removal of `Recycle`/`kite`/`exceptional`, ids `q8m3vz`/`t5hw2k`/`n7c4rj`)
**Issue:** A user with no live `styles.json` may have `dynamic.filter` rules tagged
`# ff:style q8m3vz Recycle` (and so on). After upgrading, those rules show `unknown style` and their
actions stay frozen, with no explanation. CLAUDE.md documents the `generate` consequence but not
this overlay-side one.
**Fix:** Add one sentence to CLAUDE.md or the CHANGELOG draft: rules tagged with the removed styles
show `unknown style` until the style is recreated (for example from `tests/fixtures/seed-styles.json`).

### IN-04: The Reset buttons are laid over the section heading without reserving space (carried over)

**File:** `src/overlay/config_view.rs:787-796`, `src/overlay/shell.rs:697-703`
**Issue:** The 34pt button sits in a child Ui sized to the `ff_section` heading rect, over the right
part of the gold underline. No space is allocated for it in the parent, so a long heading in a
narrow window could run under the button. This is a visual issue only.
**Fix:** Put the heading and the button in one horizontal row, or make the button shorter or offset
it so the underline stays clear. Check with `--screenshot --tab config`.

### IN-05: `reset_tiers` order fallback is not guaranteed to validate when `tiers.json` holds an unknown or duplicate tier name (new)

**File:** `src/overlay/config_state.rs:337-341`
**Issue:** `ValidatedTiers::try_from` requires that S..F are present but accepts extra entries, so a
`tiers.json` with an extra `{ "tier": "X", ... }` or a second `"S"` entry loads. Two cases then go
wrong:
- **Unknown tier.** `reset_tiers` leaves it unchanged and sorts it last (`unwrap_or(usize::MAX)`).
  If it shares the chaos unit with a lower cutoff, it now fails validation after S. The error goes
  in `errors.row["X"]`, but `tier_rows` shows only S..F (`DISPLAY_ORDER`), so the message is never
  displayed: S is highlighted as an edge, Apply is refused ("Fix the tier errors first"), and the
  Reset button stays enabled because `tiers_at_defaults` is now `false`.
- **Duplicate tier.** Both copies get the same default cutoff, which can never be strictly
  ascending. `set_tier_text` edits only the first copy, so the user can clear the error only with
  Revert.

Before the fix, both cases gave a silent "at defaults" state that failed at Apply. Now they show an
error, which is an improvement, but the docstring's claim that the default order validates holds
only for a file containing exactly the seven known tiers. These files are not ones the app
produces.
**Fix:** Either drop unknown or duplicate entries during a reset, since "defaults" means exactly
`tiers-default.json`:
```rust
pub fn reset_tiers(&mut self) {
    self.tiers = Self::default_tiers().to_vec();
    self.resync_buffers();
    self.recompute_tier_errors();
}
```
This would also drop the file-order branch, which with all-chaos defaults only applies when the
file is already in F..S order. Alternatively, keep the current behaviour and narrow the docstring to
"for a file holding exactly S..F".

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
