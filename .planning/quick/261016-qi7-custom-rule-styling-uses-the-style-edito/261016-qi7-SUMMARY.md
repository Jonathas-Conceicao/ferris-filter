---
quick_id: 261016-qi7
phase: quick-261016-qi7
plan: 01
status: complete
subsystem: overlay
tags: [overlay, rules, config, style-editor]
requires: []
provides:
  - "src/overlay/style_lines.rs: shared style-editor lines (StyleView -> StyleIntent)"
  - "body.rs rule backend: rule_style_view / rule_intent_actions / style_leftovers"
  - "widgets::colour_slot_rgba (alpha colour card)"
affects: [src/overlay/config_view.rs, src/overlay/body.rs, src/overlay/widgets.rs, src/overlay/rule_list.rs, src/overlay/shell.rs, src/overlay/app.rs, src/overlay/screenshot.rs]
tech-stack:
  added: []
  patterns: ["one view component, two backends (styles.json Style / lossless rule lines)"]
key-files:
  created: [src/overlay/style_lines.rs]
  modified: [src/overlay/mod.rs, src/overlay/config_view.rs, src/overlay/body.rs, src/overlay/widgets.rs, src/overlay/rule_list.rs, src/overlay/rules.rs, src/overlay/shell.rs, src/overlay/app.rs, src/overlay/screenshot.rs, CLAUDE.md, CHANGELOG.md]
---

# Quick 261016-qi7: Custom rule styling uses the style editor lines; minimap-icon chip; every Revert confirms

## Task 1: Shared style-editor lines; CONFIG on it; RULES Custom STYLE on it — DONE

**Commit:** `5916af2` feat(overlay): custom rule styling uses the style editor lines

One-liner: the CONFIG style editor's lines are now one shared component (`style_lines::show`,
`StyleView` in / `StyleIntent` out); CONFIG maps intents to `StyleEdit`, and a Custom rule's STYLE
section draws the same lines over its own action lines through a lossless backend that binds the
first representable line per slot and keeps everything else as action cards.

### What changed
- `src/overlay/style_lines.rs` (new): `Vis`, `Rgba`, `StyleView` (+ `from_style`), `StyleIntent`,
  `show`, `label_look`; moved from `config_view.rs`: `ff_field_row_line`, memo helpers (now
  `memo.with(field)`), `tile_size`, font/minimap/beam/sound lines, `SoundMemo`, `switch_sound`,
  `sound_for_choice`, `safe_file_name`, `SOUND_*`, tip consts, the `probe` thread-local (all probe
  tags unchanged) and `drain_probes` (test-only).
- `config_view.rs`: `style_editor` = `StyleView::from_style` + `style_lines::show` + intent ->
  `StyleEdit`; styles_error and Delete style stay after the lines. `label_look` delegates. All
  CONFIG editor tests pass unchanged in what they assert (only imports / `drain_probes` plumbing).
- `body.rs`: `StyleBinding`, `rule_style_view`, `style_leftovers`, `rule_intent_actions`,
  `custom_style_lines` (binding rule documented in the section comment). `style_section` keeps the
  picker row, then (Custom only) the shared lines, then leftover action cards / read-only raw rows,
  then Continue last. Removed: `+ style` button, `disposition_control`/`disposition_word`/
  `DISPOSITION_CHOICES`, `action_rows`, `NO_ACTIONS`, `added_actions`, `pick_action`,
  `keyword_entries` (all became dead code; `picker_popup` kept for `+ condition`).
- `widgets.rs`: `SlotRgba` + `colour_slot_rgba` (alpha picker card, CR-02 form capture, `Set` only
  on a real change, `Cleared` only when set).

### Verification
- `cargo test`: 1337 lib tests + all integration suites green.
- `cargo fmt --check`: clean.
- clippy count: 122 (baseline 122 at dfd5204 re-verified).
- `tests/fixtures` untouched; `git grep '"+ style"' src/overlay/body.rs`: no match.

### Decisions (Task 1)
1. **Colour carries `(rgba, had_alpha)`** as `style_lines::Rgba { rgba, had_alpha }` in the view
   (plan left the choice open).
2. **Font representable only within 1..=45** (`grammar::FONT_SIZE_RANGE`): an out-of-range
   `SetFontSize` is a leftover card (keeps its "Readable in game" hint) so the 1-45 slider never
   clamps it; same for a sound volume outside 0..=300.
3. **Sound volume form:** a bound line without a volume shows the game default (50 alert / 100
   custom) and writes a volume only when the slider changes it; a kind switch keeps "no volume"
   when none was written (`PlayAlertSound 5` -> Custom -> `CustomAlertSound "uwu.mp3"`). A sound
   added to an absent slot always writes its volume (100 default).
4. **Absent-slot adds** use 4-token colours (`SetBorderColor r g b a`), `MinimapIcon <size>
   <Colour> <Shape>`, `PlayEffect <Colour>[ Temp]`, `SetFontSize n`, bare `DisableDropSound`,
   inserted by the existing `AddLine` (after the last action line).
5. **Stable id scope for the style lines** (`UiBuilder::id(("rule-style-lines", id))`): adding or
   removing a line bumps the list's card epoch; without this a colour picker opened on an absent
   slot would close on its first change. Memo id `("rule-style-last", id)` as planned.
6. **Unknown-keyword lines** (`FutureKw 3`) are `EntryKind::Opaque` and were already shown as raw
   rows in CONDITIONS; they stay there (not duplicated as STYLE leftovers) and are byte-identical
   through style edits (tested).
7. Visibility segment width is the CONFIG 128pt for both editors (RULES used 132pt before).

### Deviations
- **[Rule 3 - Blocking] rule_list test harness height.** `duplicate_and_delete_are_skin_buttons`
  and `section_above_button_emits_add_section_above` failed because an expanded empty Custom rule
  is now taller (the style lines) and its buttons fell below the 600pt test area. `run_frame` now
  sizes its area from the input's screen rect, and these two tests use a new 1200pt `tall_input`.
  No production change.
- Tests replaced (their subject was removed): `added_actions_and_picking` ->
  `absent_slots_add_one_line_in_the_default_form`; `disposition_offers_show_and_hide_only` ->
  `visibility_writes_show_or_hide_only_when_it_differs`; `empty_actions_state_exists` ->
  `rule_without_actions_renders_the_style_lines_idle`; `custom_add_button_reads_plus_style` ->
  `custom_rules_have_no_add_style_button`; `section_heads_use_skin_buttons` now asserts `+ style`
  is absent.

### New tests (body.rs / widgets.rs)
`rule_style_view_binds_the_first_representable_line_per_slot`, `colour_edits_keep_the_line_form`,
`minimap_none_removes_only_the_bound_line`, `unrepresentable_lines_survive_other_edits_as_leftovers`,
`sound_edits_keep_the_line_and_its_volume_form`, `font_and_drop_sound_add_and_remove_one_line`,
`untouched_custom_rule_renders_idle_and_byte_identical`,
`custom_rules_get_the_style_lines_presets_do_not`, `clicking_a_style_line_writes_its_line`,
`colour_slot_rgba_idle_frames_are_unchanged`.

## Task 2: Style chip draws the minimap icon — DONE

**Commit:** `f54235d` feat(overlay): style chip draws the minimap icon

One-liner: the collapsed-row style chip paints the style's minimap icon from the game atlas in
its colour (replacing the gold slider-thumb diamond) followed by the name; an untagged rule with a
representable `MinimapIcon` shows just that icon.

### What changed
- `rules.rs`: `RuleRow.chip_icon: Option<(usize, usize)>` (shape, colour indices into
  `skin::minimap::SHAPES`/`COLOURS`) and `chip_icon_desc: Option<String>` (`Medium Blue Star`),
  computed in `rebuild()` only (never per frame) via a new `chip_icon_of(&Minimap)`.
  `StyleState::Known` -> the resolved `TagMatch.style.minimap`; `StyleState::None` -> the minimap
  that `body::rule_style_view` binds (only when the block has a `MinimapIcon` line, so untagged
  rows without one pay nothing). `economy_row` fills `None`.
- `rule_list.rs`: `FlagKind::Styled { name: Option<String>, icon: Option<(usize, usize)> }`;
  `flags_for` emits Known -> name + icon (text `styled: <short>`, tip unchanged), None + icon ->
  icon only (text `minimap icon`, tip `Minimap icon: <desc>`). Width via `chip_width` =
  icon 18 + 4 gap (only when both) + name width. Painted with
  `skin::editor::paint_minimap_icon(.., FLAG_ICON_SIZE, WHITE)` at the left of the chip rect.
  `DIAMOND_W` removed. Flag order, `fit_flags`, `flag_budget`, 120px / +N / 140px unchanged.

### Verification
- `cargo test`: 1344 lib tests + all integration suites green.
- `cargo fmt --check`: clean. Clippy count: 122 (unchanged).
- `tests/fixtures` untouched.

### Decisions (Task 2)
1. **Reuse Task 1's binding rule** for the custom-rule icon: `rules.rs` calls
   `super::body::rule_style_view(&self.body_view(id), kw)` rather than a second decoder, so the chip
   and the Custom style lines can never disagree on which `MinimapIcon` line counts (first
   representable: size 0..=2, known colour/shape; `MinimapIcon -1` and out-of-range sizes skipped).
   No module cycle problem (sibling modules in `overlay`).
2. **Icon always drawn at the full 18pt flag size**, regardless of the minimap size (CONFIG's Icon
   column scales by size via `icon_points`); in a collapsed row a Small icon at ~10pt would be
   unreadable. The size is still named in the custom tooltip.
3. Icon-only chip only for `StyleState::None`; Unknown / Unchecked tags get no chip (Unknown keeps
   its ring flag), and a Known style without a minimap is name only.
4. Disabled untagged rules also get their icon chip (the chip describes the rule, not its state),
   consistent with the styled name chip on disabled rows.

### Deviations
- None functional. Tests were written alongside the implementation (no separate RED commit; quick
  task, one commit per task as the plan requires).

### New / updated tests (rule_list.rs)
`known_style_chip_draws_its_minimap_icon_then_the_name`, `known_style_without_minimap_chip_is_name_only`,
`untagged_rule_chip_is_its_minimap_icon_alone`, `unknown_or_unchecked_style_has_no_chip`,
`chip_icon_follows_the_loaded_styles` (rows rebuild on `set_styles`),
`chip_width_is_icon_gap_and_name`, `style_chip_paints_the_atlas_icon_left_of_the_name` (one 18pt
mesh on the minimap atlas texture left of the name for S; none for E); updated
`style_flags_known_truncated_unknown_and_none` and `flags_carry_kinds_tips_and_paint_order` to the
new `FlagKind::Styled { .. }`. `continue_word_is_8pt_from_styled_chip` passes unchanged.

### Docs for Task 3 (chip)
CLAUDE.md, the collapsed-row flag list in the rule-editor paragraph: replace "`styled` (gold diamond
plus the style name truncated to 10 characters)" with: `styled` = the style's minimap icon (game
art, in its colour) plus the name truncated to 10 characters; name only when the style has no
minimap icon; a Custom (untagged) rule with a `MinimapIcon` shows just that icon (tooltip
`Minimap icon: <Size Colour Shape>`). Also the `ff-proto`/skin rows need no change. CHANGELOG
`[Unreleased]` Changed: "The collapsed rule's style chip shows the style's minimap icon (game art)
instead of a gold diamond; Custom rules with a minimap icon show it too."

## Task 3: Every Revert confirms + docs — DONE

**Commit:** `9a0b71e` feat(overlay): every Revert asks first

One-liner: RULES Revert now opens a skinned **Discard rule changes?** popup (eighth popup), ECONOMY
Revert asks through the discard popup (titled **Discard economy changes?**) even when only a staged
economy add/remove is pending, and `--popup discard-rules` opens the new popup in screenshot mode.

### What changed
- `shell.rs`: `Popup::DiscardRules` (last in `active_popup`), `PopupFlags.discard_rules`,
  `PopupData::DiscardRules { filter }` (body `Unapplied rule changes to <file> will be discarded.`,
  buttons Discard (red) / Keep editing). `PopupData::DiscardConfig` gained `economy` and `staged`;
  with `economy` the title is `Discard economy changes?` and the body comes from the new
  `economy_revert_prompt(count, staged)` (`Discard N unapplied tier change(s)[ and the staged economy
  section change]?` / `Discard the staged economy section change?`).
- `app.rs`: `revert_rules_armed` field; the RULES bar Revert arms it; `popup_flags` uses
  `rules_revertible()` (`working.is_dirty()`, the bar's Revert predicate) and the new
  `config_discard_count()` (scope edits + 1 for a staged op on the Tiers scope); both armed flags
  auto-disarm when nothing is left. Dispatch: `DiscardRules` choice 0 -> `self.revert()` (unchanged
  status / apply_state). `economy_revert_pressed` arms the popup for tier edits OR a staged op (never
  drops silently); on Discard, `RevertScoped(Tiers)` runs only when tiers are dirty (no misleading
  "Reverted tier changes."), then the staged op is unstaged. `open_shot_popup("discard-rules")`
  toggles Continue on the first row in memory and arms the flag.
- `screenshot.rs`: `POPUPS: [&str; 8]` with `discard-rules`.
- `CLAUDE.md`: `ff` row `--popup` list; rule-editor STYLE paragraph (Custom = style editor lines,
  binding + lossless edits, leftover rows, no `+ style`; `+ condition` picker only); collapsed-row
  `styled` flag = minimap icon + name; Revert sentence in the rule list paragraph; Config-tab Revert
  sentence (every Revert asks first); Popups paragraph (eight, Discard rule changes?, economy title).
- `CHANGELOG.md` `[Unreleased]` / Changed: three entries for this quick task.

### Verification
- `cargo test`: 1349 lib tests + all integration suites green.
- `cargo fmt --check`: clean. Clippy count: 122 (unchanged).
- `git diff --stat -- tests/fixtures`: empty.

### Decisions (Task 3)
1. **ECONOMY reuses `Popup::DiscardConfig`** (no ninth popup), titled `Discard economy changes?` on
   the Tiers scope; CONFIG keeps `Discard config changes?` with `revert_prompt`.
2. **The staged op is no longer counted as a "config change"** in the popup body; it is named
   separately ("the staged economy section change").
3. **The rule popup body names the filter file** (from the loaded filter's path), falling back to a
   nameless sentence when no filter is loaded.
4. **`config_discard_count()` drives both the flag and the auto-disarm**, replacing the previous
   flag predicate `dirty_count() > 0` (any config edit) with the armed scope's own count, so a CONFIG
   Revert with only tier edits pending no longer opens an empty-scope popup.
5. **Screenshot edit** for `discard-rules` is a Continue toggle on the first row via the normal
   `run_row_actions` path (screenshot mode never applies).

### Deviations
- None functional. Test replaced (its behaviour was deliberately changed):
  `economy_revert_without_tier_edits_drops_the_staged_op_at_once` ->
  `economy_revert_without_tier_edits_asks_before_dropping_the_staged_op`.

### New tests
`shell.rs`: `popup_stacking_order` (DiscardRules last), `popup_specs_match_the_contract` (economy
and rule specs), `the_last_button_is_the_safe_one_and_never_danger` (new data). `app.rs`:
`economy_revert_without_tier_edits_asks_before_dropping_the_staged_op`,
`economy_revert_with_nothing_to_discard_does_not_arm`, `rules_revert_asks_first_and_discard_reverts`,
`discard_rules_flag_needs_something_to_revert`, `shot_popup_discard_rules_opens_the_popup`.
`screenshot.rs`: `every_popup_name_parses`.

## Self-Check: PASSED
- Commits present: 5916af2, f54235d, 9a0b71e.
- Files: src/overlay/style_lines.rs, shell.rs, app.rs, screenshot.rs, CLAUDE.md, CHANGELOG.md exist.

## Review fixes

One fix pass over `261016-qi7-REVIEW.md` (report: `261016-qi7-REVIEW-FIX.md`).

- `7ae01be` **CR-01, WR-02:** the style lines now bind the LAST line of each keyword family (the
  game's effective line; the four alert-sound keywords are one family, Disable/EnableDropSound
  another) and never add a second line of a family. An unrepresentable effective line shows its
  nearest state, is rewritten in place by an edit and also stays listed as a row. This supersedes
  Task 1's "first representable line" rule and decision 2 (an out-of-range font/volume is now
  bound, clamped, and listed). New `LinePatch::Content` keeps a trailing comment on a keyword
  change (sound kind switch, EnableDropSound rewrite). The chip follows the same binding.
- `c3a7808` **IN-01:** an empty-slot colour picker session keeps the 4-token form.
- `6585531` **WR-03, WR-04:** RULES Revert button and popup share `rules_revertible()` (rule edits
  only); RULES Revert keeps a staged economy add/remove (`WorkingFilter::revert_rules`).
- `3eceffe` **IN-03:** `--popup discard-rules` warns on stderr when it cannot stage an edit.
- `0fd431b`, `890673d` **WR-01, WR-05:** CLAUDE.md / CHANGELOG (`### Removed` entry) document the
  binding, the separate rows and the actions that can no longer be added; `action_groups()` /
  `ACTION_GROUPS` removed, `resolve()` test-only.
- Left as is (orchestrator): IN-02, IN-04.

Verification: `cargo test` green (1358 lib tests + integration suites), `cargo fmt --check` clean,
clippy count 122, `tests/fixtures` untouched.

## Screenshots (orchestrator, after review fixes)

Debug `ff.exe --screenshot` against a temp home (scratch test filters), viewed:
- `shots/rules-list-chips.png`: S = pink circle + `S`, kite = orange kite + `kite`, `unchanged` = name only, Custom rule with `MinimapIcon 1 Blue Star` = blue star only (beside Continue).
- `shots/rule-custom-expanded.png`: Custom STYLE shows Visibility / Label + swatches / Font size / Minimap / Beam / Sound / Volume / Drop sound, then leftover rows (the shadowed earlier `PlayAlertSound`, then the effective positional line).
- `shots/rule-preset-expanded.png`: preset S shows only the picker, then Continue.
- `shots/popup-discard-rules.png`: "Discard rule changes?" with Discard (red) / Keep editing.

Nit seen: the Background swatch caption is cut ("Background (") at the 660pt width when the slot is empty.
