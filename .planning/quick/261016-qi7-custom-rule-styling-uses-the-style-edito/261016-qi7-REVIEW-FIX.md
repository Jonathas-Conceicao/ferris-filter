---
phase: quick-261016-qi7
fixed_at: 2026-10-16T23:33:32Z
review_path: .planning/quick/261016-qi7-custom-rule-styling-uses-the-style-edito/261016-qi7-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 8
skipped: 2
status: partial
---

# Quick 261016-qi7: Code Review Fix Report

**Fixed at:** 2026-10-16T23:33:32Z
**Source review:** .planning/quick/261016-qi7-custom-rule-styling-uses-the-style-edito/261016-qi7-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (all, per orchestrator)
- Fixed: 8 (CR-01, WR-01..05, IN-01, IN-03)
- Skipped: 2 (IN-02, IN-04: left as is by orchestrator decision)

| Finding | Status | Commit(s) |
|---|---|---|
| CR-01 | fixed: requires human verification (binding logic) | `7ae01be` |
| WR-01 | fixed | `0fd431b`, `890673d` |
| WR-02 | fixed | `7ae01be` |
| WR-03 | fixed | `6585531` |
| WR-04 | fixed | `6585531` |
| WR-05 | fixed | `0fd431b` |
| IN-01 | fixed | `c3a7808` |
| IN-02 | skipped (orchestrator: leave as is) | - |
| IN-03 | fixed | `3eceffe` |
| IN-04 | skipped (orchestrator: leave as is) | - |

## Fixed Issues

### CR-01: Duplicate same-keyword action lines

**Files modified:** `src/overlay/body.rs`, `src/overlay/rules.rs`, `src/overlay/rule_list.rs`, `src/filter/lineedit.rs`, `src/overlay/app.rs`
**Commit:** 7ae01be
**Applied fix:** each style line binds the LAST action line of its keyword family (the line the
game uses): one family per colour keyword, `SetFontSize`, `MinimapIcon`, `PlayEffect`, Sound
(`PlayAlertSound`, `PlayAlertSoundPositional`, `CustomAlertSound`, `CustomAlertSoundOptional` as
one slot, since an item plays one alert sound) and Drop sound (`DisableDropSound`,
`EnableDropSound`). `AddLine` runs only when the family has no line, so a family never gets a
second line. An unrepresentable effective line shows the nearest honest state (None for
`MinimapIcon -1` / `PlayEffect None` / a named or out-of-range sound id; font size and volume
clamped into the slider range; a positional / optional sound as its plain kind, keyword kept on
edit; `EnableDropSound` as the check unticked, `EnableDropSound False` as ticked). An edit rewrites
it in place (values patch, or the new `LinePatch::Content` for a keyword change such as
`EnableDropSound` -> `DisableDropSound`), and it is ALSO listed as a leftover row with its exact
text (both places edit the same line; the binding is recomputed each frame). A clamped volume is
kept as written until the slider moves. Earlier lines of a family stay leftover rows, untouched;
clearing the slot removes the effective line, so an earlier one shows through as in game. The
collapsed-row chip uses the same binding (a later `MinimapIcon -1` hides the chip).
Tests: updated `rule_style_view_binds_the_last_line_of_each_family` (renamed),
`minimap_none_removes_only_the_bound_line`, `unrepresentable_lines_survive_other_edits_as_leftovers`,
`untagged_rule_chip_is_its_minimap_icon_alone`, idle-render test gains a clamped/inexact source;
new `an_unrepresentable_last_line_shows_its_nearest_state_and_stays_listed`,
`an_edit_rewrites_the_last_line_and_never_adds_a_second_one` (only `MinimapIcon -1` + pick ->
rewritten, no duplicate; two `SetFontSize` -> the last is edited; two `SetTextColor`; `PlayEffect
None`; `SetFontSize 50`; named sound; optional custom sound), `drop_sound_follows_the_last_disable_or_enable_line`,
`positional_optional_and_out_of_range_sound_lines_keep_their_form`, and the UI-level
`picking_a_minimap_size_over_a_hidden_icon_rewrites_that_line`.
Status note: binding logic, so flagged for a human check in the overlay.

### WR-01: `+ style` removal undocumented; `action_groups()` dead

**Files modified:** `CHANGELOG.md`, `CLAUDE.md`, `src/overlay/grammar.rs`, `src/overlay/body.rs`
**Commits:** 0fd431b, 890673d
**Applied fix:** CHANGELOG `[Unreleased]` gains a `### Removed` entry listing the actions that can
no longer be added (`PlayAlertSoundPositional`, `CustomAlertSoundOptional`, `EnableDropSound`, the
two `...IfAlertSound` keywords, `MinimapIcon -1`, `PlayEffect None`); CLAUDE.md says the same and
"every condition and every action line in the file is editable" (new actions only through the
style lines). `ACTION_GROUPS` / `action_groups()` and `every_action_is_reachable_exactly_once`
removed; `style_owned_follows_style_output` walks the action keywords directly; `resolve()` became
test-only and is `#[cfg(test)]` (890673d fixes the dead-code warning 0fd431b introduced).

### WR-02: Sound kind switch dropped an inline comment

**Files modified:** `src/filter/lineedit.rs`, `src/overlay/body.rs`, `src/overlay/app.rs`
**Commit:** 7ae01be
**Applied fix:** new `LinePatch::Content(text)` replaces keyword + operator + values and keeps the
indent, a disabled block's `#` prefix and everything after the last value (spacing, `# comment`);
validated strictly like a value edit. The Alert <-> Custom switch uses it.
`app.rs` treats it like `Whole` for ordinal shifts and inline refusal messages. Tests:
`content_replaces_keyword_and_values_and_keeps_the_comment` (lineedit),
`sound_kind_switch_keeps_the_inline_comment` (body).

### WR-03: RULES Revert live but inert on a style-rewrite-only state

**Files modified:** `src/overlay/app.rs`
**Commit:** 6585531
**Applied fix:** one predicate: `rules_revertible()` = `working.rules_edited()`; the bar's RULES
Revert enablement is `rules_revert_live()` = that predicate through `footer_actions` (so it is still
off during a changed-on-disk refusal or a failed load). A pending style rewrite alone is not
revertable (Revert cannot undo a `styles.json` change), so the button is off then. Test
`rules_revert_button_and_popup_share_one_predicate`; `discard_rules_flag_needs_something_to_revert`
also checks the bar predicate.

### WR-04: RULES Revert dropped ECONOMY's staged add/remove

**Files modified:** `src/overlay/rules.rs`, `src/overlay/app.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 6585531 (docs in 0fd431b)
**Applied fix:** `WorkingFilter::revert_rules()` re-parses the applied text and keeps `pending`
(the economy section is locked to rule edits, so the op stays valid); `revert()` = clear pending +
`revert_rules()`. The app's RULES revert uses `revert_rules()` and keeps the apply state unapplied
while the staged op remains. With only a staged op, RULES Revert is off and the popup never opens.
Test `revert_rules_keeps_a_staged_economy_op` (rules.rs) and the app test above.

### WR-05: CLAUDE.md colour picker wording

**Files modified:** `CLAUDE.md`
**Commit:** 0fd431b
**Applied fix:** the style lines use the Text / Border / Background cards with Alpha and a None
tile (None removes the line); leftover colour rows keep the `R G B A` chip without None. The
binding paragraph now documents that an out-of-range font size or volume, `MinimapIcon -1`,
`PlayEffect None`, named / positional / optional sounds and `EnableDropSound` show as a separate
row (plus the nearest state on the style line). The chip and Revert sentences follow CR-01 / WR-04.

### IN-01: Empty-slot colour flips from 4 to 3 tokens

**Files modified:** `src/overlay/widgets.rs`, `src/overlay/body.rs`
**Commit:** c3a7808
**Applied fix:** `colour_slot_rgba` records the session form as `had_alpha || current.is_none()`
when the picker opens, so a line added by the first change (4 tokens) stays 4-valued for the whole
session. Test `a_colour_added_to_an_empty_slot_stays_four_valued` (intent level; no headless
harness drives this picker).

### IN-03: `--popup discard-rules` silent when nothing is staged

**Files modified:** `src/overlay/app.rs`
**Commit:** 3eceffe
**Applied fix:** after trying the Continue toggle, `eprintln!` a warning when
`rules_revertible()` is still false (no filter, no rules, first rule not editable).

## Skipped Issues

### IN-02: Rule memo keyed by BlockId outlives Revert/Reload

**File:** `src/overlay/body.rs` (memo id `("rule-style-last", id)`)
**Reason:** orchestrator decision: leave as is.
**Original issue:** after a re-parse, BlockIds can point at other blocks, so a stale remembered
font/minimap/beam/sound value could be restored on another rule.

### IN-04: Drop sound check reads inverted next to `EnableDropSound`

**File:** `src/overlay/style_lines.rs`
**Reason:** orchestrator decision: leave as is.
**Original issue:** the "Drop sound" check writes `DisableDropSound` when ticked (CONFIG parity);
the tooltip says "Silence the default drop sound".

## Verification

- `cargo test` (CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target): 1358 lib tests +
  every integration suite green.
- `cargo fmt --check`: clean.
- Clippy count (`cargo clippy --all-targets 2>&1 | grep '^warning' | grep -v generated | wc -l`): 122.
- `git diff --stat 9a0b71e -- tests/fixtures`: empty.

---

_Fixed: 2026-10-16T23:33:32Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
