---
phase: 31-rules-tab-on-the-skin
fixed_at: 2026-10-16T13:49:14Z
review_path: .planning/phases/31-rules-tab-on-the-skin/31-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 7
skipped: 1
status: partial
---

# Phase 31: Code Review Fix Report

**Fixed at:** 2026-10-16T13:49:14Z
**Source review:** .planning/phases/31-rules-tab-on-the-skin/31-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (Critical/Warning; the 9 Info findings were out of scope)
- Fixed: 7
- Skipped: 1

Verification after all fixes: full `cargo test` passes (1184 lib tests + integration suites),
`cargo fmt --all --check` is clean, clippy stays at 126 warnings, and `tests/fixtures` is
unchanged.

## Fixed Issues

### WR-01: `insert_top_crlf` test no longer contains a single CR

**Files modified:** `src/filter/structure.rs`
**Commit:** 179c0ef
**Applied fix:** `insert_top_crlf` uses escaped `\r\n` literals again (`"Hide\r\n"` gives
`"Show\r\n\r\nHide\r\n"`, `"Show\r\nHide"` gives `"Show\r\n\r\nShow\r\nHide"`). A new mixed-ending
case (`"Hide\r\nShow\n"`) checks that the inserted rule takes the first line's ending.
`insert_top_above_disabled_block` uses `\t` / `\n` escapes again instead of a literal tab.

### WR-03: 44pt number field cannot show 4-digit values

**Files modified:** `src/overlay/widgets.rs`
**Commit:** 3337c98
**Applied fix:** Added `number_field_width(chars)`: 44pt for up to three characters, plus 9pt
for each extra character. `number_field` widens to fit its value or draft.
`themed_slider` reserves space for the widest of `min`, `max` and the current value, so the
slider and field still fit the row. Values of up to three digits lay out exactly as before. New
test: `number_field_grows_for_four_digits_and_the_slider_leaves_room`.

### WR-04: Log-scale sliders ignore the arrow keys

**Files modified:** `src/overlay/widgets.rs`
**Commit:** f2dcd05
**Applied fix:** When a log slider has focus, `themed_slider` now handles Left and Right itself
and moves the whole-number value by ±1, clamped to the range. Home and End are unchanged. New
test: `themed_slider_log_steps_by_one_on_arrow_keys`.

### WR-05: Skin dropdown has no scroll area; long option lists run off the window

**Files modified:** `src/skin/widgets.rs`
**Commit:** 7dfbc4c
**Applied fix:** The option list scrolls inside the game-art `ff_scroll_area` when it would need
more than 12 rows or more height than the room above or below the box. The minimum height is 3
rows. Lists shorter than that limit take the same path as before. Each option row is now drawn
by `dropdown_row`. New test: `dropdown_long_list_scrolls_inside_the_screen`. I confirmed it fails
when scrolling is turned off.

### WR-06: Collapsed-row summary is painted under the flags and reorder arrows

**Files modified:** `src/overlay/rule_list.rs`
**Commit:** 484961c
**Applied fix:** The summary on line 2 is now cut 8pt before the flags and arrows
(`used_left - 8.0`), the same way the name already was. New test:
`long_summary_stops_before_the_flags`. It fails without the fix: the summary ran to x≈505 past
the Continue flag at x≈388.

### WR-07: Quoted-list / int-list add field ignores its hint

**Files modified:** `src/overlay/widgets.rs`, `src/overlay/body.rs`
**Commit:** 1c3c915
**Applied fix:** `add_field` passes the caller's `hint` through again. The UI-SPEC asks for
"press Enter" wording, so the caller hints now read "Type a base type, press Enter", "Type a
mod, press Enter", "Type a name, press Enter" and "Type a whole number, press Enter". New test:
`add_field_shows_the_callers_hint`.

### WR-08: Reorder arrows lost their padded click target

**Files modified:** `src/skin/widgets.rs`, `src/overlay/widgets.rs` (test)
**Commit:** d84cb0e
**Applied fix:** `ff_arrow` still takes up a 16x16 area in the layout. When enabled, it now
senses clicks on a padded target (`ARROW_HIT_PAD` = 4pt horizontal, 3pt vertical, so 24x22),
like the pre-skin `arrow_button`. A disabled arrow still senses hover only. The vertical padding
is 3pt, not 4pt: with 4pt the target reached the centre of the collapsed row's summary line
(y=38), and the existing test `clicking_the_summary_text_expands_the_row` failed there. New test:
`enabled_arrow_button_has_a_padded_click_target`.

## Skipped Issues

### WR-02: + Add rule puts a match-everything `Show` first, shadowing the entire filter until conditions are added

**File:** `src/overlay/rules.rs:1097`, `src/filter/structure.rs:205-274`
**Reason:** Author decision; reported for the author. Inserting at the top of the file is an
explicit decision (RULES-03). The suggested mitigations go against the project's "UI not texty"
direction or the chosen behaviour: a standing grammar warning, inserting the rule disabled, or a
confirmation popup on Apply. I found no small fix that keeps the top placement.
**Original issue:** A new empty `Show` rule with no `Continue` becomes the first block. Until
conditions are added it matches every item, so if it is applied it overrides every Hide rule,
every styled rule and the economy section.

## Note for the orchestrator

The fix worktree was first checked out with CRLF line endings (system `core.autocrlf=true`).
In that checkout, two tests that scan source code with `include_str!` and look for `"\n}\n"`
fail: `defences_card_is_an_ff_card_with_three_lines` and
`picker_popup_uses_skin_search_and_sections`. They pass with an LF checkout, as in the main
repo. I re-checked out the worktree with LF before running the suite. The committed blobs are
LF (`git ls-files --eol` shows `i/lf`).

---

_Fixed: 2026-10-16T13:49:14Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
