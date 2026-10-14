---
phase: 24-uat-feedback-fixes
reviewed: 2026-10-13T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/dynamic.rs
  - src/filter/keywords.rs
  - src/filter/tokenize.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/grammar.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/widgets.rs
findings:
  critical: 0
  warning: 3
  info: 6
  total: 9
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-10-13
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Scope: the phase-24 diff (`976144b..HEAD`) in the 11 listed files. The full suite passes
(`cargo test --all-targets`: 832 lib tests plus the integration targets, 0 failures).

What checks out:
- **PARSE-04 / idle frames.** Compact Yes/No booleans (`bool_effective` / `bool_edit`) emit nothing
  on idle frames, keep the existing operator, and rewrite only the value word. Ordering operators
  (`Corrupted > True`) fall back to the raw value control.
- **Minimal blocks.** These are displayed but never rewritten unless Show/Hide is clicked
  (`DISPOSITION_CHOICES`, `disposition_control`).
- **Preset to Custom.** Switching keeps the action lines and only drops the tag (`RemoveStyle` →
  `remove_meta`).
- **Removed PoE1 keywords.** These lines stay opaque and lossless and do not block Apply. The
  tests in `dynamic.rs` cover both the gate and the written bytes.
- **Slider defaults.** Every default is inside its slider range, and a table test enforces it.
- **egui IDs.** The ids are salted per block/keyword/ordinal. The vertical list's `ScrollArea`
  is salted from the widget id.
- **Collapsed row height.** The painted height of a collapsed row is right, but only because
  `show_list` sets `item_spacing.y = 0`.

No blockers found. The main defect is that clicking the collapsed-row summary text does not
expand the row. The style picker also cannot load a `Minimal` style, which the spec requires, and
the virtualization tests do not measure the painted heights.

## Warnings

### WR-01: Clicking the summary text does not expand the row (the selectable label takes the click)

**File:** `src/overlay/rule_list.rs:480-509`
**Issue:** The summary strip is allocated first with `Sense::click()` (`sresp`). The text is then
added as an `egui::Label` in a child UI on top of it. The app never turns off
`style.interaction.selectable_labels`, and egui defaults it to `true`. In egui 0.36,
`Label::layout_in_ui` therefore adds `Sense::click_and_drag()` for text selection
(egui-0.36.2 `widgets/label.rs:154-168`). Because the label is registered after `sresp`, it is the
topmost hit, so a click on the summary text starts a text selection instead of expanding the row.
Only the 40px indent and the blank area past the text are clickable. The header handles the same
problem by interacting a `row_click` rect after the label (lines 461-468). The summary does not
do this, and no test clicks on the summary text.
**Fix:** Make the label non-interactive, or register the click target after the label:
```rust
sui.add(
    egui::Label::new(RichText::new(&row.summary).size(theme::ROW_SUMMARY_SIZE).color(color))
        .truncate()
        .selectable(false),
);
// or: allocate srect with Sense::hover(), then after the label:
// let sresp = ui.interact(srect, ui.id().with(("ff-summary-click", row.id)), Sense::click());
```
Add a headless test that clicks at the centre of the summary text and asserts that
`list.expanded == Some(id)`.

### WR-02: A style JSON carrying `Minimal` still fails to load (UAT-04 requirement not met)

**File:** `src/models/config.rs:124-127` (consumed by `config_view.rs:612` and `styled.rs:82-85`)
**Issue:** The UAT-04 spec says: "If a style JSON somehow carries `Minimal`, keep loading it
(don't fail); the UI just never offers it." `StyleAction` has only `Show` and `Hide`, so
`"action": "Minimal"` fails serde in `loader::load_styles`. That rejects the whole `styles.json`,
so every styled rule turns `Unchecked` and the CONFIG Styles tab is empty. The phase changed the
style action UI (slide switch, Show/Hide-only picker) but did not add tolerance for this input,
and no test covers it.
**Fix:** Add a `Minimal` variant that is never offered. Map it in `rewrite_block`
(`StyleAction::Minimal => BlockKeyword::Minimal`). Have the slide switch treat any non-`Show`
action as "off" (it already does: `on = action == Show`). Add a loader test with a
`"Minimal"` style. If the decision is to keep refusing it, record that in CONTEXT/SUMMARY rather
than leave the spec silently unmet.

### WR-03: The row-height tests are tautological and never measure painted geometry

**File:** `src/overlay/rule_list.rs:896-928`, `src/overlay/rule_list.rs:479`
**Issue:**
- `collapsed_row_height_includes_summary_line` only asserts
  `row_height == header_height + SUMMARY_LINE_H`, which is the definition of `row_height`.
- `expanded_row_height_excludes_summary` asserts
  `build_offsets(...).last() == 2*row_h + expanded_h`, which is arithmetic inside `build_offsets`.
- The `debug_assert!` in `draw_row` repeats the same arithmetic.

None of these checks what `draw_row` actually allocates. The collapsed row paints
`header_h + SUMMARY_LINE_H` only because `show_list` sets `item_spacing.y = 0.0` (line 302) around
the two `allocate_exact_size` calls. If someone moves that line or calls `draw_row` from another
context, a 4px gap appears per row. The offsets then drift by `4 * n` over about 460 rows, and no
test fails. This is the UAT-13 virtualization invariant the phase was asked to guard.
**Fix:** In a headless frame, record `ui.cursor().min.y` before and after each `draw_row` (or read
`ui.min_rect()` of the per-row `push_id` scope). Assert that a collapsed row spans exactly
`row_height(ui)` and an expanded row spans exactly `list.expanded_h`.

## Info

### IN-01: Dead parameters and fields left by the style rework

**File:** `src/overlay/body.rs:1801, 1821`, `src/overlay/body.rs:1533-1538`, `src/overlay/config_state.rs:75`
**Issue:**
- `action_card(..., lock: Option<(&str, bool)>, ...)` is now always called with `None`. Its
  "locked" branch (around line 1867) cannot run.
- `style_owned` is now `#[cfg(test)]` only. `Style` was moved into a test-only import to make
  this compile.
- `StyleBadges::hide` is no longer read outside tests, since the CONFIG "Hide" badge was removed.
**Fix:** Drop the `lock` parameter and its branch, delete `style_owned` (or move it into the
test module), and remove `hide` from `StyleBadges`.

### IN-02: `Enum` and `BoolOrName` value shapes are now unreachable in production

**File:** `src/overlay/grammar.rs:964-977`, `src/overlay/body.rs:891-898`
**Issue:** After `HasInfluence`, `GemQualityType` and `TransfiguredGem` were removed, no keyword
uses `ValueShape::Enum` or `ValueShape::BoolOrName`. The codec arms, `enum_control` and
`bool_or_name_control` are kept alive only by test-only fake keywords (`TEST_ENUM`,
`TEST_BOOL_OR_NAME`).
**Fix:** Either remove the shapes and their UI, or add a comment on `ValueShape` saying they are
kept on purpose for future PoE2 keywords.

### IN-03: The `string_list` row cap ignores item spacing

**File:** `src/overlay/widgets.rs:736-746`
**Issue:** `max_height(max_rows as f32 * ROW_H)` assumes 26px rows. Each `ui.horizontal` row also
gets the inherited `item_spacing.y` (4px inside condition cards), so `max_rows = 7` shows about
6 rows. That is still inside the 6-8 range in the spec, but the parameter's name does not match
what it does.
**Fix:** Use `max_rows as f32 * (ROW_H + ui.spacing().item_spacing.y)`, or set
`item_spacing.y = 0` inside the scroll area.

### IN-04: Negated booleans hide their operator and flip into a double negative

**File:** `src/overlay/body.rs:726-741, 807-826`
**Issue:** `Corrupted != True` renders as "No" with no operator shown. Clicking "Yes" writes
`Corrupted != False`. This is valid and keeps the operator (which the spec allows), but the user
gets no visual hint that the line is negated, and the result is hard to read in the filter file.
**Fix:** Optionally add a hover tooltip on the Yes/No chips when `entry.op` is `Ne` or `Bang`
(for example "written as `!= True`").

### IN-05: Zebra striping restarts in each style group

**File:** `src/overlay/config_view.rs:566-568`
**Issue:** `style_row(..., i, ...)` uses the index within its group, so the Custom group starts
on the same stripe colour the Built-in group started on. This is only visual.
**Fix:** Pass a running index across both groups if continuous striping is wanted.

### IN-06: Presets on disabled rules only set the tag; Custom then keeps the old actions

**File:** `src/overlay/body.rs:1652-1660` (via `rules.rs:851-881`)
**Issue:** `ApplyStyle` rewrites the actions only when the block is enabled. On a disabled rule,
choosing a preset sets the tag and hides the action rows, but the old actions stay in the text.
Switching back to Custom then shows actions that never came from the preset. This is consistent
with the stated rule ("keeps the currently rendered actions"), but the actions in preset mode were
never visible, so the result may surprise the user.
**Fix:** Either document this, or disable the preset picker on disabled rules.

---

_Reviewed: 2026-10-13_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
